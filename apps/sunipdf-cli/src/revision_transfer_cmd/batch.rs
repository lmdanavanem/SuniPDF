use super::{invalid, read_recipe};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_transfer::{self, batch::PreparedTransferBatch, TransferPasswords};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroizing;

const MAX_MANIFEST_BYTES: usize = 512 * 1024;
const MAX_ITEMS: usize = 16;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    id: String,
    old: PathBuf,
    new: PathBuf,
    recipe: PathBuf,
    out: PathBuf,
    #[serde(default)]
    old_password_env: Option<String>,
    #[serde(default)]
    new_password_env: Option<String>,
}

fn validate_environment_name(value: &str) -> Result<(), EngineError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        })
    {
        return Err(invalid(
            "password environment names must use ASCII letters, digits and underscores",
        ));
    }
    Ok(())
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest, EngineError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid(
            "transfer batch manifest must contain at most 512 KiB",
        ));
    }
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| invalid(&format!("invalid transfer batch manifest: {error}")))?;
    if manifest.version != 1 || manifest.items.is_empty() || manifest.items.len() > MAX_ITEMS {
        return Err(invalid("transfer batch version 1 requires 1 to 16 entries"));
    }
    let mut ids = HashSet::new();
    for item in &manifest.items {
        if item.id.is_empty()
            || item.id.len() > 64
            || !item.id.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b'-'))
            })
            || !ids.insert(&item.id)
        {
            return Err(invalid("batch entry IDs must be unique, 1 to 64 ASCII letters/digits with optional dots, underscores and hyphens"));
        }
        for path in [&item.old, &item.new, &item.recipe, &item.out] {
            validate_path(path)?;
        }
        for name in [&item.old_password_env, &item.new_password_env]
            .into_iter()
            .flatten()
        {
            validate_environment_name(name)?;
        }
    }
    Ok(manifest)
}

fn validate_path(path: &Path) -> Result<(), EngineError> {
    let Some(text) = path.to_str() else {
        return Err(invalid("batch paths must be valid Unicode"));
    };
    if text.is_empty()
        || text.len() > 4096
        || text.chars().any(char::is_control)
        || text.starts_with("\\\\")
        || text.starts_with("//")
        || text.contains("://")
        || (!path.is_absolute()
            && path
                .components()
                .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir)))
    {
        return Err(invalid(
            "batch paths must be explicit local file paths, absolute or relative to the manifest",
        ));
    }
    Ok(())
}

fn resolve_path(parent: &Path, selected: &Path) -> Result<PathBuf, EngineError> {
    validate_path(selected)?;
    Ok(if selected.is_absolute() {
        selected.to_path_buf()
    } else {
        parent.join(selected)
    })
}

fn password(name: Option<&str>) -> Result<Option<Zeroizing<String>>, EngineError> {
    name.map(|name| {
        validate_environment_name(name)?;
        crate::password_input::read_secret(Some(name), false).map(Zeroizing::new)
    })
    .transpose()
}

pub(super) fn run(engine: &Engine, path: &Path, apply: bool) -> Result<Value, EngineError> {
    let manifest_path = path.canonicalize()?;
    let parent = manifest_path
        .parent()
        .ok_or_else(|| invalid("manifest directory is missing"))?;
    let file = sunipdf_core::file_io::open_regular(&manifest_path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(invalid(
            "transfer batch manifest must be a regular file of at most 512 KiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let manifest = parse_manifest(&bytes)?;
    let cancel = AtomicBool::new(false);
    let mut batch = PreparedTransferBatch::new();
    for item in manifest.items {
        let old = resolve_path(parent, &item.old)?;
        let new = resolve_path(parent, &item.new)?;
        let recipe = read_recipe(&resolve_path(parent, &item.recipe)?)?;
        let output = resolve_path(parent, &item.out)?;
        let old_password = password(item.old_password_env.as_deref())?;
        let new_password = password(item.new_password_env.as_deref())?;
        let candidate = revision_transfer::prepare_revision_transfer(
            engine,
            &old,
            &new,
            TransferPasswords {
                old: old_password.as_ref().map(|value| value.as_str()),
                new: new_password.as_ref().map(|value| value.as_str()),
            },
            &recipe,
            &cancel,
        )?;
        batch
            .push(item.id, candidate, output)
            .map_err(|error| error.error)?;
    }
    if apply {
        let outcome = batch.publish(&cancel, |_| {});
        let mut value = outcome.json();
        value["operation"] = json!("batch_apply");
        value["complete"] = json!(outcome.complete());
        value["published"] = json!(!outcome.completed.is_empty());
        Ok(value)
    } else {
        batch.preflight(&cancel)?;
        Ok(json!({
            "operation": "batch_preview",
            "version": 1,
            "complete": true,
            "published": false,
            "candidate_bytes": batch.candidate_bytes(),
            "items": batch.summaries(),
            "publication": "Explicit batch-apply re-prepares and revalidates every input; each saved copy is atomic, the entire batch is not a filesystem transaction."
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> Value {
        json!({"id":id,"old":"old.pdf","new":"new.pdf","recipe":"review.json","out":"reviewed.pdf"})
    }

    #[test]
    fn strict_manifest_rejects_ambiguous_unbounded_or_embedded_passwords() {
        let valid = json!({"version":1,"items":[item("floor-1")]});
        assert_eq!(
            parse_manifest(&serde_json::to_vec(&valid).unwrap())
                .unwrap()
                .items
                .len(),
            1
        );
        for value in [
            json!({"version":2,"items":[item("a")]}),
            json!({"version":1,"items":[]}),
            json!({"version":1,"items":[item("a"),item("a")]}),
            json!({"version":1,"items":[item("_hidden")]}),
            json!({"version":1,"items":(0..17).map(|i|item(&format!("item{i}"))).collect::<Vec<_>>()}),
            json!({"version":1,"items":[item("a")],"password":"not-accepted"}),
        ] {
            assert!(parse_manifest(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut embedded = valid.clone();
        embedded["items"][0]["old_password"] = json!("not-accepted");
        assert!(parse_manifest(&serde_json::to_vec(&embedded).unwrap()).is_err());
        assert!(parse_manifest(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
        assert!(parse_manifest(br#"{"version":1,"version":1,"items":[]}"#).is_err());
    }

    #[test]
    fn manifest_paths_resolve_from_its_directory_and_reject_remote_or_control_paths() {
        let directory = std::env::current_dir().unwrap().join("manifest-test");
        assert_eq!(
            resolve_path(&directory, Path::new("plans/old.pdf")).unwrap(),
            directory.join("plans/old.pdf")
        );
        let absolute = directory.join("absolute.pdf");
        assert_eq!(resolve_path(&directory, &absolute).unwrap(), absolute);
        for value in [
            "",
            "https://example.invalid/a.pdf",
            "//server/share/a.pdf",
            "\\\\server\\share\\a.pdf",
            "old\n.pdf",
        ] {
            assert!(
                resolve_path(&directory, Path::new(value)).is_err(),
                "{value:?}"
            );
        }
        #[cfg(windows)]
        for value in ["C:old.pdf", "\\old.pdf"] {
            assert!(resolve_path(&directory, Path::new(value)).is_err());
        }
    }

    #[test]
    fn per_entry_password_references_are_only_environment_names() {
        for name in ["PDF_PASSWORD", "_PDF_2", "x"] {
            assert!(validate_environment_name(name).is_ok());
        }
        for name in ["", "2PDF", "PDF PASSWORD", "a=b", "a\0b", "é"] {
            assert!(validate_environment_name(name).is_err());
        }
        let mut value = json!({"version":1,"items":[item("a")]});
        value["items"][0]["old_password_env"] = json!("PDF_OLD");
        value["items"][0]["new_password_env"] = json!("PDF_NEW");
        assert!(parse_manifest(&serde_json::to_vec(&value).unwrap()).is_ok());
    }
}
