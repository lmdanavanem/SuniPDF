use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[path = "../../../crates/sunipdf-core/tests/support/certificate_identity.rs"]
mod test_identity;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

fn json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failed_signature_report(output: Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verification_failed"], true);
    assert_eq!(report["verification_passed"], false);
    assert_eq!(report["identity_trusted"], false);
    report
}

fn failed(output: Output, out: &Path) -> serde_json::Value {
    assert!(!output.status.success());
    assert!(!out.exists(), "a failed operation created output");
    serde_json::from_slice(&output.stderr).unwrap()
}

#[test]
fn new_form_fields_are_fillable_after_save_and_invalid_input_creates_no_output() {
    let temp = tempfile::tempdir().unwrap();
    let input = fixture("boxes.pdf");
    let original = std::fs::read(&input).unwrap();
    let text = temp.path().join("text.pdf");
    json(
        cli()
            .arg("add-form-field")
            .arg(&input)
            .args([
                "--kind",
                "text",
                "--name",
                "FullName",
                "--rect",
                "40,40,220,65",
                "--page",
                "3",
                "--out",
            ])
            .arg(&text)
            .output()
            .unwrap(),
    );
    let check = temp.path().join("check.pdf");
    json(
        cli()
            .arg("add-form-field")
            .arg(&text)
            .args([
                "--kind",
                "checkbox",
                "--name",
                "Consent",
                "--rect",
                "30,30,50,50",
                "--out",
            ])
            .arg(&check)
            .output()
            .unwrap(),
    );
    let fields = json(cli().arg("form-fields").arg(&check).output().unwrap());
    assert!(fields["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "FullName"));
    assert!(fields["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "Consent"));
    let filled = temp.path().join("filled.pdf");
    json(
        cli()
            .arg("fill-form")
            .arg(&check)
            .args(["--set", "FullName=Alice", "--set", "Consent=true", "--out"])
            .arg(&filled)
            .output()
            .unwrap(),
    );
    let values = json(
        cli()
            .arg("form-fields")
            .arg(&filled)
            .arg("--values-only")
            .output()
            .unwrap(),
    );
    assert_eq!(values["FullName"], "Alice");
    assert_eq!(values["Consent"], true);
    for (name, rect, page) in [
        ("Consent", "30,30,50,50", "1"),
        ("Invalid.Name", "30,30,50,50", "1"),
        ("Outside", "-2,30,50,50", "1"),
        ("Nonfinite", "NaN,30,50,50", "1"),
        ("BadPage", "30,30,50,50", "99"),
    ] {
        let out = temp.path().join("invalid.pdf");
        failed(
            cli()
                .arg("add-form-field")
                .arg(&check)
                .args([
                    "--kind", "text", "--name", name, "--rect", rect, "--page", page, "--out",
                ])
                .arg(&out)
                .output()
                .unwrap(),
            &out,
        );
    }
    assert_eq!(std::fs::read(input).unwrap(), original);
}

#[test]
fn local_ocr_produces_searchable_text_and_skips_existing_text_byte_for_byte() {
    let temp = tempfile::tempdir().unwrap();
    let input = fixture("two-pages.pdf");
    let original = std::fs::read(&input).unwrap();
    for language in ["eng", "fra", "eng+fra"] {
        let out = temp.path().join(format!("skip-{language}.pdf"));
        let report = json(
            cli()
                .arg("ocr")
                .arg(&input)
                .args(["--language", language, "--out"])
                .arg(&out)
                .output()
                .unwrap(),
        );
        assert_eq!(report["recognized_pages"], 0);
        assert_eq!(report["skipped_pages"], 2);
        assert_eq!(std::fs::read(out).unwrap(), original);
    }
    let blank = temp.path().join("blank.pdf");
    json(
        cli()
            .arg("insert-blank")
            .arg(&input)
            .args(["--at", "1", "--width", "500", "--height", "160", "--out"])
            .arg(&blank)
            .output()
            .unwrap(),
    );
    let vector = temp.path().join("vector.pdf");
    json(
        cli()
            .arg("add-text")
            .arg(&blank)
            .args([
                "--x",
                "24",
                "--y",
                "100",
                "--text",
                "Bonjour, l'été français arrive bientôt.",
                "--size",
                "22",
                "--out",
            ])
            .arg(&vector)
            .output()
            .unwrap(),
    );
    let image = temp.path().join("scan.png");
    json(
        cli()
            .arg("render")
            .arg(&vector)
            .args(["--page", "1", "--scale", "3", "--out"])
            .arg(&image)
            .output()
            .unwrap(),
    );
    let scan = temp.path().join("scan.pdf");
    json(
        cli()
            .arg("images2pdf")
            .arg(&image)
            .args(["--dpi", "216", "--out"])
            .arg(&scan)
            .output()
            .unwrap(),
    );
    let scan_bytes = std::fs::read(&scan).unwrap();
    let out = temp.path().join("recognized.pdf");
    let report = json(
        cli()
            .arg("ocr")
            .arg(&scan)
            .args(["--language", "eng+fra", "--pages", "1", "--out"])
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert_eq!(report["recognized_pages"], 1);
    assert!(report["words"].as_u64().unwrap() >= 5);
    let text = cli()
        .arg("text")
        .arg(&out)
        .args(["--page", "1"])
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    for word in ["Bonjour", "français", "bientôt"] {
        assert!(text.contains(word), "{text}");
    }
    assert_eq!(std::fs::read(&scan).unwrap(), scan_bytes);
    let invalid = temp.path().join("invalid.pdf");
    failed(
        cli()
            .arg("ocr")
            .arg(&scan)
            .args(["--pages", "0", "--out"])
            .arg(&invalid)
            .output()
            .unwrap(),
        &invalid,
    );
    assert_eq!(std::fs::read(input).unwrap(), original);
}

#[test]
fn digital_signatures_verify_saved_bytes_and_never_replace_the_source_or_disclose_passwords() {
    let temp = tempfile::tempdir().unwrap();
    let certificate = temp.path().join("identity.pfx");
    std::fs::write(&certificate, test_identity::identity()).unwrap();
    let input = temp.path().join("original.pdf");
    std::fs::copy(fixture("two-pages.pdf"), &input).unwrap();
    let original = std::fs::read(&input).unwrap();
    assert!(
        json(cli().arg("signatures").arg(&input).output().unwrap())["signatures"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let sign = |file: &Path, out: &Path, password: &str| {
        let mut command = cli();
        command
            .arg("sign")
            .arg(file)
            .arg("--certificate")
            .arg(&certificate)
            .args([
                "--certificate-password-env",
                "SUNIPDF_TEST_CERT_PASSWORD",
                "--reason",
                "Approved",
                "--location",
                "Geneva",
                "--out",
            ])
            .arg(out)
            .env("SUNIPDF_TEST_CERT_PASSWORD", password);
        command
    };
    let signed = temp.path().join("signed.pdf");
    let output = sign(&input, &signed, "test-only").output().unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains("test-only"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("test-only"));
    let report = json(output);
    assert_eq!(report["signatures"][0]["status"], "valid");
    assert_eq!(report["signatures"][0]["reason"], "Approved");
    assert_eq!(report["identity_trusted"], false);
    assert_eq!(
        report["signatures"][0]["certificate"]["identity_trusted"],
        false
    );
    assert!(report["signatures"][0]["certificate"]["subject"]
        .as_str()
        .unwrap()
        .contains("SuniPDF Test Identity"));
    let signed_bytes = std::fs::read(&signed).unwrap();
    assert!(signed_bytes.starts_with(&original));
    assert_eq!(std::fs::read(&input).unwrap(), original);
    let listed = json(cli().arg("signatures").arg(&signed).output().unwrap());
    assert_eq!(listed["signatures"][0]["status"], "valid");
    let tampered = temp.path().join("tampered.pdf");
    let mut bytes = signed_bytes.clone();
    let offset = bytes
        .windows(14)
        .position(|w| w == b"SuniPDF page 1")
        .unwrap();
    bytes[offset] = b'T';
    std::fs::write(&tampered, bytes).unwrap();
    assert_eq!(
        failed_signature_report(cli().arg("signatures").arg(&tampered).output().unwrap())
            ["signatures"][0]["status"],
        "invalid"
    );
    let appended = temp.path().join("appended.pdf");
    let mut bytes = signed_bytes.clone();
    bytes.extend_from_slice(b"\n% changed after signing\n");
    std::fs::write(&appended, bytes).unwrap();
    assert_eq!(
        failed_signature_report(cli().arg("signatures").arg(&appended).output().unwrap())
            ["signatures"][0]["status"],
        "modified_after_signing"
    );

    let twice = temp.path().join("twice.pdf");
    let mut child = cli()
        .arg("sign")
        .arg(&signed)
        .arg("--certificate")
        .arg(&certificate)
        .arg("--certificate-password-stdin")
        .arg("--out")
        .arg(&twice)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"test-only\r\n")
        .unwrap();
    let report = json(child.wait_with_output().unwrap());
    assert_eq!(report["signatures"][0]["status"], "modified_after_signing");
    assert_eq!(report["signatures"][1]["status"], "valid");
    assert!(std::fs::read(&twice).unwrap().starts_with(&signed_bytes));

    let wrong = temp.path().join("wrong.pdf");
    let output = sign(&input, &wrong, "wrong-password-keep-private")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("wrong-password-keep-private"));
    failed(output, &wrong);
    let output = sign(&input, &input, "test-only")
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read(&input).unwrap(), original);
    let encrypted = temp.path().join("encrypted-signed.pdf");
    let error = failed(
        sign(&fixture("encrypted-rc4.pdf"), &encrypted, "test-only")
            .output()
            .unwrap(),
        &encrypted,
    );
    assert!(error["message"].as_str().unwrap().contains("encrypted"));
}
