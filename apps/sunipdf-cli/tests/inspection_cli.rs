#![cfg(windows)]

#[path = "support/bounded_process.rs"]
mod process;
use lopdf::{dictionary, Object};
use std::process::Command;

fn run(path: &std::path::Path, password: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg("inspect").arg(path);
    if let Some(password) = password {
        command
            .args(["--password-env", "SUNIPDF_INSPECTION_TEST_CREDENTIAL"])
            .env("SUNIPDF_INSPECTION_TEST_CREDENTIAL", password);
    }
    process::bounded(command)
}

#[test]
fn inspection_reports_findings_partial_results_and_non_argv_encrypted_inputs_without_writing() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("Inspection été 中文.pdf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let mut pdf = lopdf::Document::load(&fixture).unwrap();
    pdf.add_object(dictionary! {"S"=>"JavaScript", "JS"=>Object::string_literal("never execute")});
    pdf.save(&path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let output = run(&path, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complete"], true);
    assert_eq!(report["findings"][0]["dictionaries"], 1);
    assert_eq!(report["network_accessed"], false);
    assert_eq!(report["file_modified"], false);
    assert_eq!(std::fs::read(&path).unwrap(), original);

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for aes256 in [false, true] {
        let protection = sunipdf_core::Protection {
            user_password: format!("{nonce}-{}-opening", std::process::id()),
            owner_password: format!("{nonce}-{}-owner", std::process::id()),
            aes256,
            ..Default::default()
        };
        let encrypted = sunipdf_core::security::protect(&original, None, &protection).unwrap();
        let encrypted_path = temp.path().join(format!("encrypted-{aes256}.pdf"));
        std::fs::write(&encrypted_path, &encrypted).unwrap();
        let denied = run(&encrypted_path, None);
        assert_eq!(denied.status.code(), Some(3));
        for credential in [&protection.user_password, &protection.owner_password] {
            let output = run(&encrypted_path, Some(credential));
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["complete"], true);
            assert_eq!(report["findings"][0]["dictionaries"], 1);
            assert!(!String::from_utf8_lossy(&output.stdout).contains(credential));
            assert!(!String::from_utf8_lossy(&output.stderr).contains(credential));
        }
        assert_eq!(std::fs::read(&encrypted_path).unwrap(), encrypted);
    }
    pdf.add_object(dictionary! {"JS"=>Object::Reference((9999,0))});
    pdf.save(&path).unwrap();
    let partial_input = std::fs::read(&path).unwrap();
    let partial = run(&path, None);
    assert_eq!(partial.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert_eq!(report["complete"], false);
    assert_eq!(report["findings"][0]["dictionaries"], 2);
    assert_eq!(std::fs::read(&path).unwrap(), partial_input);

    let malformed = temp.path().join("malformed.pdf");
    std::fs::write(&malformed, b"not a PDF").unwrap();
    let failed = run(&malformed, None);
    assert_eq!(failed.status.code(), Some(4));
    assert!(failed.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failed.stderr).unwrap()["error"],
        "invalid_format"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 4);
}
