#[path = "../../../crates/sunipdf-core/tests/fixtures/radio_exports.rs"]
mod fixture;

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn values(path: &Path) -> Value {
    success(
        cli()
            .arg("form-fields")
            .arg(path)
            .arg("--values-only")
            .output()
            .unwrap(),
    )
}

#[test]
fn fdf_export_and_import_roundtrip_unicode_radio_options_without_changing_sources() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let fdf = directory.path().join("form values.fdf");
    let altered = directory.path().join("altered.pdf");
    let restored = directory.path().join("restored.pdf");
    let bytes = fixture::source(false);
    std::fs::write(&source, &bytes).unwrap();
    let original = values(&source);
    assert_eq!(original[fixture::FIELD_NAME], fixture::EXPORTS[1]);
    success(
        cli()
            .arg("form-export")
            .arg(&source)
            .arg("--out")
            .arg(&fdf)
            .output()
            .unwrap(),
    );
    let exchange = std::fs::read(&fdf).unwrap();
    assert!(exchange.starts_with(b"%FDF-1.2"));
    let raw = sunipdf_core::form_data::parse_values(&exchange, "fdf").unwrap();
    assert_eq!(
        raw[fixture::FIELD_NAME],
        "virement",
        "FDF carries the real button state"
    );
    success(
        cli()
            .arg("fill-form")
            .arg(&source)
            .args(["--set", "paiement=Cash"])
            .arg("--out")
            .arg(&altered)
            .output()
            .unwrap(),
    );
    success(
        cli()
            .arg("fill-form")
            .arg(&altered)
            .arg("--values")
            .arg(&fdf)
            .arg("--out")
            .arg(&restored)
            .output()
            .unwrap(),
    );
    assert_eq!(values(&restored), original);
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(std::fs::read(&fdf).unwrap(), exchange);
    let refusal = cli()
        .arg("form-export")
        .arg(&source)
        .arg("--out")
        .arg(&fdf)
        .output()
        .unwrap();
    assert_eq!(refusal.status.code(), Some(6));
    assert_eq!(std::fs::read(&fdf).unwrap(), exchange);
}

#[test]
fn invalid_fdf_button_state_refuses_import_without_writing_a_pdf() {
    use lopdf::{dictionary, Object};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let input = directory.path().join("invalid.fdf");
    let output = directory.path().join("out.pdf");
    let source_bytes = fixture::source(false);
    std::fs::write(&source, &source_bytes).unwrap();
    let mut data = lopdf::Document::with_version("1.2");
    data.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    let field = dictionary! {"T"=>lopdf::text_string(fixture::FIELD_NAME), "V"=>Object::Name(b"unknown".to_vec())};
    let root = data
        .add_object(dictionary! { "FDF"=>dictionary! {"Fields"=>vec![Object::Dictionary(field)]} });
    data.trailer.set("Root", root);
    let mut bytes = Vec::new();
    data.save_to(&mut bytes).unwrap();
    bytes[1..4].copy_from_slice(b"FDF");
    std::fs::write(&input, bytes).unwrap();
    let failure = cli()
        .arg("fill-form")
        .arg(&source)
        .arg("--values")
        .arg(&input)
        .arg("--out")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
}
