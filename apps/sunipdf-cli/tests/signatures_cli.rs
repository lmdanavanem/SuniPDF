use lopdf::{dictionary, Object};
use std::path::Path;
use std::process::Command;

#[test]
fn signature_verification_prints_json_and_returns_failure_for_an_invalid_signature() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let unsigned = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .arg("signatures")
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(unsigned.status.success());
    let report: serde_json::Value = serde_json::from_slice(&unsigned.stdout).unwrap();
    assert_eq!(report["verification_failed"], false);
    assert_eq!(report["verification_passed"], false);

    let directory = tempfile::tempdir().unwrap();
    let invalid = directory.path().join("invalid-signature.pdf");
    let mut pdf = lopdf::Document::load(&fixture).unwrap();
    let value = pdf.add_object(dictionary! { "Type"=>"Sig", "SubFilter"=>"adbe.pkcs7.detached" });
    let field = pdf
        .add_object(dictionary! { "FT"=>"Sig", "T"=>lopdf::text_string("Signature"), "V"=>value });
    let form = pdf.add_object(dictionary! { "Fields"=>vec![Object::Reference(field)] });
    pdf.catalog_mut().unwrap().set("AcroForm", form);
    pdf.save(&invalid).unwrap();
    let original = std::fs::read(&invalid).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .arg("signatures")
        .arg(&invalid)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verification_failed"], true);
    assert_eq!(report["verification_passed"], false);
    assert_eq!(report["identity_trusted"], false);
    assert!(!report["signatures"][0]["failure_cause"].is_null());
    assert_eq!(std::fs::read(&invalid).unwrap(), original);
}
