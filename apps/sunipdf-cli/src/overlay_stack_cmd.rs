use crate::password_input;
use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_overlay::{
    stack::{prepare_overlay_stack, OverlayStackLayer, OverlayStackOptions},
    OverlayAlignment,
};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroizing;

const MAX_MANIFEST_BYTES: usize = 256 * 1024;

#[derive(Args)]
pub struct OverlayStackArgs {
    /// Version-1 JSON with 2 to 4 explicit local PDF layers; paths are relative to this file.
    #[arg(long)]
    pub manifest: PathBuf,
    /// Write one combined PNG or one source-bound JSON report per call.
    #[arg(long, default_value = "png", value_parser = ["png", "json"])]
    pub format: String,
    /// New output only; existing files are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    #[serde(default = "default_long_side")]
    long_side: u32,
    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    path: PathBuf,
    #[serde(default = "first_page")]
    page: usize,
    #[serde(default)]
    alignment: OverlayAlignment,
    color: Option<[u8; 3]>,
    opacity: Option<f32>,
    visible: Option<bool>,
    expected_sha256: Option<String>,
    password_env: Option<String>,
}

fn default_long_side() -> u32 {
    sunipdf_core::revision_overlay::DEFAULT_LONG_SIDE
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
        return Err(invalid(
            "overlay layer paths must be explicit local paths, absolute or relative to the manifest",
        ));
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

fn parse_manifest(bytes: &[u8]) -> Result<Manifest, EngineError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid(
            "overlay stack manifest must contain at most 256 KiB",
        ));
    }
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| invalid(format!("invalid overlay stack manifest: {error}")))?;
    if manifest.version != 1 || !(2..=4).contains(&manifest.layers.len()) {
        return Err(invalid("overlay stack version 1 requires 2 to 4 layers"));
    }
    for layer in &manifest.layers {
        validate_path(&layer.path)?;
        if let Some(expected) = &layer.expected_sha256 {
            if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(invalid(
                    "expected layer SHA-256 requires 64 hexadecimal digits",
                ));
            }
        }
        if let Some(environment) = &layer.password_env {
            validate_environment(environment)?;
        }
    }
    Ok(manifest)
}

fn options(manifest: &Manifest, parent: &Path) -> Result<OverlayStackOptions, EngineError> {
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

pub fn run(engine: &Engine, args: OverlayStackArgs) -> Result<Value, EngineError> {
    if args.out.symlink_metadata().is_ok() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let path = args.manifest.canonicalize()?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("overlay manifest directory is missing"))?;
    let file = sunipdf_core::file_io::open_regular(&path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(invalid(
            "overlay manifest requires a regular file of at most 256 KiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let manifest = parse_manifest(&bytes)?;
    let options = options(&manifest, parent)?;
    let passwords = manifest
        .layers
        .iter()
        .map(|layer| {
            layer
                .password_env
                .as_deref()
                .map(|name| password_input::read_secret(Some(name), false).map(Zeroizing::new))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let borrowed = passwords
        .iter()
        .map(|password| password.as_ref().map(|value| value.as_str()))
        .collect::<Vec<_>>();
    let cancel = AtomicBool::new(false);
    let preview = prepare_overlay_stack(engine, options, &borrowed, &cancel)?;
    let report = preview.export_json(&cancel)?;
    for (index, layer) in manifest.layers.iter().enumerate() {
        if let Some(expected) = &layer.expected_sha256 {
            let actual = report["layers"][index]["source"]["source_sha256"]
                .as_str()
                .ok_or_else(|| invalid("overlay layer hash is unavailable"))?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(invalid(format!(
                    "overlay layer {} differs from its expected SHA-256",
                    index + 1
                )));
            }
        }
    }
    let output = if args.format == "png" {
        preview.export_png(&cancel)?
    } else {
        serde_json::to_vec_pretty(&report).map_err(|error| invalid(error.to_string()))?
    };
    preview.revalidate_sources(&cancel)?;
    sunipdf_core::write_new_copy(&args.out, &output)?;
    Ok(json!({"out":args.out,"format":args.format,"originals_unchanged":true,"report":report}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_strict_manifest_and_explicit_layer_paths() {
        let valid = json!({"version":1,"long_side":256,"layers":[{"path":"base.pdf"},{"path":"revision.pdf"}]});
        let parse = |value: &Value| parse_manifest(&serde_json::to_vec(value).unwrap());
        let manifest = parse(&valid).unwrap();
        let resolved = options(&manifest, Path::new("plans")).unwrap();
        assert_eq!(resolved.layers[0].path, Path::new("plans/base.pdf"));
        assert_eq!(resolved.layers[0].color, [255, 0, 255]);
        assert_eq!(resolved.layers[1].color, [0, 255, 255]);
        assert_eq!(resolved.layers[0].opacity, 1.0);
        assert_eq!(resolved.layers[1].opacity, 0.65);
        for value in [
            json!({}),
            json!({"version":2,"layers":valid["layers"]}),
            json!({"version":1,"layers":[]}),
            json!({"version":1,"layers":vec![valid["layers"][0].clone(); 5]}),
        ] {
            assert!(parse(&value).is_err());
        }
        for (field, value) in [
            ("password", json!("not-supported")),
            ("expected_sha256", json!("bad")),
            ("password_env", json!("A=B")),
            ("path", json!("https://example.invalid/file.pdf")),
            ("path", json!("//server/file.pdf")),
            ("path", json!("bad\nname.pdf")),
        ] {
            let mut changed = valid.clone();
            changed["layers"][1][field] = value;
            assert!(parse(&changed).is_err(), "{field}");
        }
        assert!(parse_manifest(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
        let mut hidden = valid.clone();
        hidden["layers"][0]["visible"] = false.into();
        hidden["layers"][1]["visible"] = false.into();
        assert!(options(&parse(&hidden).unwrap(), Path::new(".")).is_err());
    }
}
