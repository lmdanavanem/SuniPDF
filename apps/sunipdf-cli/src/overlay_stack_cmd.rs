use crate::password_input;
use clap::Args;
#[cfg(test)]
use recipe::{options, parse as parse_manifest, MAX_MANIFEST_BYTES};
use serde_json::{json, Value};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_overlay::recipe;
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroizing;

#[derive(Args)]
pub struct OverlayStackArgs {
    /// Version-1 JSON with 2 to 4 explicit local PDF layers; paths are relative to this file.
    #[arg(long)]
    pub manifest: PathBuf,
    /// Write a combined PNG, a JSON report, or a reusable source-bound setup.
    #[arg(long, default_value = "png", value_parser = ["png", "json", "setup"])]
    pub format: String,
    /// New output only; existing files are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(engine: &Engine, args: OverlayStackArgs) -> Result<Value, EngineError> {
    if args.out.symlink_metadata().is_ok() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let loaded = recipe::read_regular(&args.manifest)?;
    let parent = loaded
        .path
        .parent()
        .ok_or_else(|| EngineError::Other("overlay manifest directory is missing".into()))?;
    let manifest = loaded.manifest;
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
    let preview = recipe::prepare(engine, &manifest, parent, &borrowed, &cancel)?;
    let report = preview.export_json(&cancel)?;
    if args.format == "setup" {
        recipe::save_new(&preview, &args.out, &cancel)?;
        return Ok(
            json!({"out":args.out,"format":args.format,"originals_unchanged":true,"report":report}),
        );
    }
    let output = if args.format == "png" {
        preview.export_png(&cancel)?
    } else {
        serde_json::to_vec_pretty(&report).map_err(|error| EngineError::Other(error.to_string()))?
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
