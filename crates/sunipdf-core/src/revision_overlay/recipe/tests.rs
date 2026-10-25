use super::*;
use serde_json::{json, Value};

fn minimal() -> Value {
    json!({"version":1,"layers":[{"path":"base.pdf"},{"path":"revision.pdf"}]})
}

fn parsed(value: &Value) -> Result<Manifest, EngineError> {
    parse(&serde_json::to_vec(value).unwrap())
}

#[test]
fn cli_wire_defaults_optional_bindings_and_desktop_requirements_remain_distinct() {
    let mut value = minimal();
    let manifest = parsed(&value).unwrap();
    assert_eq!(manifest.long_side, DEFAULT_LONG_SIDE);
    let resolved = options(&manifest, Path::new("plans")).unwrap();
    assert_eq!(resolved.layers[0].path, Path::new("plans/base.pdf"));
    assert_eq!(resolved.layers[0].page, 1);
    assert_eq!(resolved.layers[0].color, [255, 0, 255]);
    assert_eq!(resolved.layers[1].color, [0, 255, 255]);
    assert_eq!(resolved.layers[0].opacity, 1.0);
    assert_eq!(resolved.layers[1].opacity, 0.65);
    assert!(require_desktop(&manifest).is_err());
    for layer in value["layers"].as_array_mut().unwrap() {
        layer["expected_sha256"] = json!("AB".repeat(32));
    }
    require_desktop(&parsed(&value).unwrap()).unwrap();
    value["layers"][1]["password_env"] = "LOCAL_OVERLAY_SECRET".into();
    let cli = parsed(&value).unwrap();
    assert_eq!(
        cli.layers[1].password_env.as_deref(),
        Some("LOCAL_OVERLAY_SECRET")
    );
    assert!(require_desktop(&cli)
        .unwrap_err()
        .to_string()
        .contains("environment"));
    value["layers"][0]["visible"] = false.into();
    value["layers"][1]["visible"] = false.into();
    assert!(options(&parsed(&value).unwrap(), Path::new(".")).is_err());
}

#[test]
fn parser_is_bounded_and_refuses_unknown_fields_remote_paths_and_secret_values() {
    for value in [
        json!({}),
        json!({"version":2,"layers":minimal()["layers"]}),
        json!({"version":1,"layers":[]}),
        json!({"version":1,"layers":vec![minimal()["layers"][0].clone();5]}),
    ] {
        assert!(parsed(&value).is_err());
    }
    for (field, value) in [
        ("password", json!("do-not-import")),
        ("expected_sha256", json!("bad")),
        ("password_env", json!("A=B")),
        ("password_env", json!("1INVALID")),
        ("path", json!("https://example.invalid/source.pdf")),
        ("path", json!("//server/source.pdf")),
        ("path", json!("\\\\server\\source.pdf")),
        ("path", json!("bad\nname.pdf")),
        ("path", json!("x".repeat(4097))),
        ("page", json!(-1)),
    ] {
        let mut changed = minimal();
        changed["layers"][1][field] = value;
        assert!(parsed(&changed).is_err(), "accepted {field}");
    }
    let mut unknown = minimal();
    unknown["sources"] = json!([]);
    assert!(parsed(&unknown).is_err());
    assert!(parse(b"").is_err());
    assert!(parse(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
    assert!(parse(br#"{"version":1,"version":1,"layers":[{"path":"a"},{"path":"b"}]}"#).is_err());
    for (field, value) in [("page", json!(0)), ("opacity", json!(1.1))] {
        let mut changed = minimal();
        changed["layers"][1][field] = value;
        assert!(options(&parsed(&changed).unwrap(), Path::new(".")).is_err());
    }
}

#[test]
fn reading_and_resolving_a_setup_does_not_open_or_rebind_missing_pdf_sources() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("setup.json");
    let mut value = minimal();
    for layer in value["layers"].as_array_mut().unwrap() {
        layer["expected_sha256"] = json!("a".repeat(64));
    }
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let loaded = read_regular(&path).unwrap();
    require_desktop(&loaded.manifest).unwrap();
    let options = options(&loaded.manifest, loaded.path.parent().unwrap()).unwrap();
    assert!(options.layers.iter().all(|layer| !layer.path.exists()));
    assert_eq!(loaded.path, path.canonicalize().unwrap());
    assert!(read_regular(directory.path()).is_err());
    let oversized = directory.path().join("oversized.json");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(MAX_MANIFEST_BYTES as u64 + 1)
        .unwrap();
    assert!(read_regular(&oversized).is_err());
    assert!(read_regular(&directory.path().join("missing.json")).is_err());
}

#[cfg(windows)]
#[test]
fn only_local_verbatim_disk_paths_are_converted_for_portable_setup_output() {
    assert_eq!(
        portable_local_path(Path::new(r"\\?\C:\plans\revision.pdf")).unwrap(),
        Path::new(r"C:\plans\revision.pdf")
    );
    assert!(portable_local_path(Path::new(r"\\?\UNC\server\plans\revision.pdf")).is_err());
    assert!(portable_local_path(Path::new(r"\\server\plans\revision.pdf")).is_err());
}
