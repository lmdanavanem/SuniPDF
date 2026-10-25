//! Bounded version-1 overlay setups shared by CLI and desktop.
//! Setups contain local selections and source identities, never PDF data or credentials.

use super::stack::{
    prepare_overlay_stack, OverlayStackLayer, OverlayStackOptions, OverlayStackPreview,
};
use super::{check_cancel, OverlayAlignment, DEFAULT_LONG_SIDE};
use crate::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[cfg(test)]
mod tests;

pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    #[serde(default = "default_long_side")]
    pub long_side: u32,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub path: PathBuf,
    #[serde(default = "first_page")]
    pub page: usize,
    #[serde(default)]
    pub alignment: OverlayAlignment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
    /// CLI-only reference to an environment variable; its value is never part of this model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_env: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedManifest {
    pub manifest: Manifest,
    /// Canonical manifest path; relative layer paths resolve against its parent.
    pub path: PathBuf,
}

fn default_long_side() -> u32 {
    DEFAULT_LONG_SIDE
}
fn first_page() -> usize {
    1
}
fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::Other(message.into())
}

fn validate_path(path: &Path) -> Result<(), EngineError> {
    let value = path
        .to_str()
        .ok_or_else(|| invalid("overlay layer paths require valid Unicode"))?;
    if value.is_empty()
        || value.len() > 4096
        || value.chars().any(char::is_control)
        || value.starts_with("\\\\")
        || value.starts_with("//")
        || value.contains("://")
        || (!path.is_absolute()
            && path
                .components()
                .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir)))
    {
        return Err(invalid("overlay layer paths must be explicit local paths, absolute or relative to the manifest"));
    }
    Ok(())
}

fn validate_environment(name: &str) -> Result<(), EngineError> {
    if name.is_empty()
        || name.len() > 128
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        })
    {
        return Err(invalid(
            "layer password environment names require ASCII letters, digits and underscores",
        ));
    }
    Ok(())
}

fn validate_hash(hash: &str) -> Result<(), EngineError> {
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid(
            "expected layer SHA-256 requires 64 hexadecimal digits",
        ));
    }
    Ok(())
}

fn validate(manifest: &Manifest) -> Result<(), EngineError> {
    if manifest.version != 1 || !(2..=4).contains(&manifest.layers.len()) {
        return Err(invalid("overlay stack version 1 requires 2 to 4 layers"));
    }
    for layer in &manifest.layers {
        validate_path(&layer.path)?;
        if let Some(hash) = &layer.expected_sha256 {
            validate_hash(hash)?;
        }
        if let Some(environment) = &layer.password_env {
            validate_environment(environment)?;
        }
    }
    Ok(())
}

/// Parse the existing CLI version-1 format. CLI hashes and password environment references
/// remain optional; desktop callers additionally invoke `require_desktop`.
pub fn parse(bytes: &[u8]) -> Result<Manifest, EngineError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid(
            "overlay stack manifest must contain at most 256 KiB",
        ));
    }
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| invalid(format!("invalid overlay stack manifest: {error}")))?;
    validate(&manifest)?;
    Ok(manifest)
}

/// Read only the bounded setup file. No layer source is opened, rebound or validated here.
pub fn read_regular(path: &Path) -> Result<LoadedManifest, EngineError> {
    let selected = std::path::absolute(path)?;
    let path = selected.canonicalize()?;
    let file = crate::file_io::open_regular(&path)?;
    if file.metadata()?.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(invalid(
            "overlay manifest requires a regular file of at most 256 KiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if selected.canonicalize()? != path || path.canonicalize()? != path {
        return Err(invalid("overlay setup path changed; open the setup again"));
    }
    Ok(LoadedManifest {
        manifest: parse(&bytes)?,
        path,
    })
}

/// Desktop setup imports must bind every selected source and cannot request environment access.
pub fn require_desktop(manifest: &Manifest) -> Result<(), EngineError> {
    validate(manifest)?;
    for layer in &manifest.layers {
        if layer.password_env.is_some() {
            return Err(invalid(
                "desktop overlay setups cannot contain password environment references",
            ));
        }
        if layer.expected_sha256.is_none() {
            return Err(invalid(
                "desktop overlay setups require an expected SHA-256 for every layer",
            ));
        }
    }
    Ok(())
}

/// Resolve explicit paths and default styles without reading any PDF source.
pub fn options(manifest: &Manifest, parent: &Path) -> Result<OverlayStackOptions, EngineError> {
    validate(manifest)?;
    let layers = manifest
        .layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            let path = if layer.path.is_absolute() {
                layer.path.clone()
            } else {
                parent.join(&layer.path)
            };
            let mut result = OverlayStackLayer::new(path, layer.page, index);
            result.alignment = layer.alignment;
            if let Some(color) = layer.color {
                result.color = color;
            }
            if let Some(opacity) = layer.opacity {
                result.opacity = opacity;
            }
            if let Some(visible) = layer.visible {
                result.visible = visible;
            }
            result
        })
        .collect();
    let options = OverlayStackOptions {
        layers,
        long_side: manifest.long_side,
    };
    options.validate()?;
    Ok(options)
}

/// Check source bindings against the exact retained preview, including hidden layers.
/// Optional bindings support existing CLI manifests and explicit source-picker replacements.
pub fn verify_expected_hashes(
    preview: &OverlayStackPreview,
    expected: &[Option<String>],
    cancel: &AtomicBool,
) -> Result<(), EngineError> {
    check_cancel(cancel)?;
    if expected.len() != preview.options().layers.len() {
        return Err(invalid(
            "provide one optional expected SHA-256 for every overlay layer",
        ));
    }
    preview.revalidate_sources(cancel)?;
    for (index, expected) in expected.iter().enumerate() {
        check_cancel(cancel)?;
        if let Some(expected) = expected {
            validate_hash(expected)?;
            let actual = preview
                .source_sha256(index)
                .ok_or_else(|| invalid("overlay layer hash is unavailable"))?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(invalid(format!("overlay layer {} differs from its expected SHA-256; explicitly choose its source again", index + 1)));
            }
        }
    }
    check_cancel(cancel)
}

/// Prepare through the ordinary permission-checked stack pipeline and verify setup bindings
/// before returning a usable preview. Passwords are borrowed for this operation only.
pub fn prepare(
    engine: &Engine,
    manifest: &Manifest,
    parent: &Path,
    passwords: &[Option<&str>],
    cancel: &AtomicBool,
) -> Result<OverlayStackPreview, EngineError> {
    check_cancel(cancel)?;
    let preview = prepare_overlay_stack(engine, options(manifest, parent)?, passwords, cancel)?;
    let expected: Vec<_> = manifest
        .layers
        .iter()
        .map(|layer| layer.expected_sha256.clone())
        .collect();
    verify_expected_hashes(&preview, &expected, cancel)?;
    Ok(preview)
}

/// Windows canonicalization adds a verbatim disk prefix. Emit an ordinary explicit local
/// drive path so exported absolute selections obey the same path grammar as CLI input.
fn portable_local_path(path: &Path) -> Result<PathBuf, EngineError> {
    #[cfg(windows)]
    if let Some(Component::Prefix(prefix)) = path.components().next() {
        if let std::path::Prefix::VerbatimDisk(drive) = prefix.kind() {
            let mut result = PathBuf::from(format!("{}:\\", char::from(drive)));
            for part in path.components().skip(2) {
                result.push(part.as_os_str());
            }
            validate_path(&result)?;
            return Ok(result);
        }
    }
    validate_path(path)?;
    Ok(path.to_path_buf())
}

fn output_path(output: &Path) -> Result<(PathBuf, PathBuf), EngineError> {
    let selected = std::path::absolute(output)?;
    let parent = selected
        .parent()
        .ok_or_else(|| invalid("choose an overlay setup directory"))?
        .canonicalize()?;
    let name = selected
        .file_name()
        .ok_or_else(|| invalid("choose an overlay setup filename"))?;
    let destination = parent.join(name);
    match destination.symlink_metadata() {
        Ok(_) => return Err(EngineError::AlreadyExists(output.into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok((destination, parent))
}

/// Export only from a current source-validated preview, always to one new file.
/// Every layer receives a hash, including hidden layers; no credential/environment field is emitted.
pub fn save_new(
    preview: &OverlayStackPreview,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<Manifest, EngineError> {
    check_cancel(cancel)?;
    preview.revalidate_sources(cancel)?;
    let (destination, parent) = output_path(output)?;
    let portable_parent = portable_local_path(&parent)?;
    let mut layers = Vec::with_capacity(preview.options().layers.len());
    for (index, layer) in preview.options().layers.iter().enumerate() {
        check_cancel(cancel)?;
        if preview.canonical_source_path(index) == Some(destination.as_path()) {
            return Err(invalid(
                "overlay setup destination is a referenced source file",
            ));
        }
        let selected = portable_local_path(&layer.path)?;
        let path = selected
            .strip_prefix(&portable_parent)
            .ok()
            .filter(|relative| {
                !relative.as_os_str().is_empty()
                    && relative
                        .components()
                        .all(|part| matches!(part, Component::Normal(_)))
            })
            .map(Path::to_path_buf)
            .unwrap_or(selected);
        layers.push(Layer {
            path,
            page: layer.page,
            alignment: layer.alignment,
            color: Some(layer.color),
            opacity: Some(layer.opacity),
            visible: Some(layer.visible),
            expected_sha256: Some(
                preview
                    .source_sha256(index)
                    .ok_or_else(|| invalid("overlay layer hash is unavailable"))?
                    .to_owned(),
            ),
            password_env: None,
        });
    }
    let manifest = Manifest {
        version: 1,
        long_side: preview.options().long_side,
        layers,
    };
    require_desktop(&manifest)?;
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| invalid(error.to_string()))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid(
            "overlay stack manifest must contain at most 256 KiB",
        ));
    }
    preview.revalidate_sources(cancel)?;
    // Pin the selected output directory too, so a retargeted directory is not silently used.
    if std::path::absolute(output)?
        .parent()
        .ok_or_else(|| invalid("choose an overlay setup directory"))?
        .canonicalize()?
        != parent
    {
        return Err(invalid(
            "overlay setup output directory changed; choose its destination again",
        ));
    }
    check_cancel(cancel)?;
    crate::write_new_copy(&destination, &bytes)?;
    Ok(manifest)
}
