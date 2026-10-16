//! Independent structural checks of the repository's synthetic fixtures and CLI output.
//! Run explicitly with SUNIPDF_QPDF set by scripts/fetch-qpdf.ps1; no customer documents are used.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

#[path = "../../../crates/sunipdf-core/tests/support/certificate_identity.rs"]
mod test_identity;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

fn bounded(mut command: Command, input: Option<&[u8]>) -> Output {
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("QA subprocess must start");
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() >= Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("QA subprocess exceeded its 180-second limit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn cli(args: &[&str], paths: &[&Path]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.args(args);
    for path in paths {
        command.arg(path);
    }
    command
}

fn success(command: Command) -> serde_json::Value {
    let output = bounded(command, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn check(qpdf: &Path, file: &Path, password: Option<&str>) {
    let mut command = Command::new(qpdf);
    if let Some(password) = password {
        command.arg(format!("--password={password}"));
    }
    command.arg("--check").arg(file);
    let output = bounded(command, None);
    // Exit 3 means structural warnings; it must fail this gate as well as exit 2.
    assert!(
        output.status.success(),
        "qpdf rejected {} ({}):\n{}\n{}",
        file.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("qpdf OK: {}", file.file_name().unwrap().to_string_lossy());
}

#[test]
#[ignore = "requires the pinned official QPDF QA runtime; executed separately in Windows CI"]
fn synthetic_corpus_and_generated_outputs_pass_independent_structural_checks() {
    let qpdf =
        PathBuf::from(std::env::var_os("SUNIPDF_QPDF").expect("set SUNIPDF_QPDF to qpdf.exe"));
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "two-pages.pdf",
        "boxes.pdf",
        "annotated.pdf",
        "acroform.pdf",
        "acroform-noap.pdf",
        "acroform-links.pdf",
        "acroform-xfa.pdf",
        "redact.pdf",
        "redact-incremental.pdf",
        "many-pages.pdf",
    ] {
        check(&qpdf, &fixture(name), None);
    }
    check(&qpdf, &fixture("encrypted-rc4.pdf"), Some("secret"));
    let original = fixture("two-pages.pdf");
    let original_bytes = std::fs::read(&original).unwrap();
    let edited = temp.path().join("edited.pdf");
    let mut command = cli(&["rotate"], &[&original]);
    command.args(["--degrees", "90", "--out"]).arg(&edited);
    success(command);
    check(&qpdf, &edited, None);
    for (name, aes128) in [("aes256.pdf", false), ("aes128.pdf", true)] {
        let encrypted = temp.path().join(name);
        let mut command = cli(&["protect"], &[&edited]);
        command
            .args([
                "--user-password",
                "qa-user",
                "--owner-password",
                "qa-owner",
                "--out",
            ])
            .arg(&encrypted);
        if aes128 {
            command.arg("--aes128");
        }
        success(command);
        check(&qpdf, &encrypted, Some("qa-user"));
        let opened = success(cli(&["properties", "--password", "qa-user"], &[&encrypted]));
        assert!(opened.is_object());
    }
    let field = temp.path().join("field.pdf");
    let mut command = cli(&["add-form-field"], &[&original]);
    command
        .args([
            "--kind",
            "text",
            "--name",
            "FullName",
            "--rect",
            "40,40,220,65",
            "--out",
        ])
        .arg(&field);
    success(command);
    check(&qpdf, &field, None);
    let filled = temp.path().join("filled.pdf");
    let mut command = cli(&["fill-form"], &[&field]);
    command
        .args(["--set", "FullName=Alice", "--out"])
        .arg(&filled);
    success(command);
    check(&qpdf, &filled, None);
    let flat = temp.path().join("flattened.pdf");
    let mut command = cli(&["flatten"], &[&filled]);
    command.arg("--out").arg(&flat);
    success(command);
    check(&qpdf, &flat, None);
    let redacted = temp.path().join("redacted.pdf");
    let mut command = cli(&["redact"], &[&fixture("redact.pdf")]);
    command
        .args(["--text", "SECRET-ALPHA-7731", "--out"])
        .arg(&redacted);
    success(command);
    check(&qpdf, &redacted, None);
    let search = success(cli(
        &["search"],
        &[&redacted, Path::new("SECRET-ALPHA-7731")],
    ));
    assert_eq!(search["count"], 0);
    let revision = temp.path().join("redacted-revision.pdf");
    let mut command = cli(&["redact"], &[&fixture("redact-incremental.pdf")]);
    command
        .args(["--text", "Current revision", "--out"])
        .arg(&revision);
    success(command);
    check(&qpdf, &revision, None);
    assert!(!std::fs::read(&revision)
        .unwrap()
        .windows(16)
        .any(|bytes| bytes == b"OLDREV-SECRET-01"));

    let png = temp.path().join("scan.png");
    let mut command = cli(&["render"], &[&original]);
    command
        .args(["--page", "1", "--scale", "2", "--out"])
        .arg(&png);
    success(command);
    let scan = temp.path().join("scan.pdf");
    let mut command = cli(&["images2pdf"], &[&png]);
    command.args(["--dpi", "144", "--out"]).arg(&scan);
    success(command);
    check(&qpdf, &scan, None);
    let ocr = temp.path().join("ocr.pdf");
    let mut command = cli(&["ocr"], &[&scan]);
    command.args(["--language", "eng+fra", "--out"]).arg(&ocr);
    let report = success(command);
    assert_eq!(report["recognized_pages"], 1);
    assert!(report["words"].as_u64().unwrap() >= 3);
    check(&qpdf, &ocr, None);

    let pfx = temp.path().join("identity.pfx");
    std::fs::write(&pfx, test_identity::identity()).unwrap();
    let signed = temp.path().join("signed.pdf");
    let mut command = cli(&["sign"], &[&original]);
    command
        .arg("--certificate")
        .arg(&pfx)
        .arg("--certificate-password-stdin")
        .arg("--out")
        .arg(&signed);
    let output = bounded(command, Some(b"test-only\n"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check(&qpdf, &signed, None);
    let report = success(cli(&["signatures"], &[&signed]));
    assert_eq!(report["signatures"][0]["status"], "valid");

    for (name, bytes) in [
        ("empty.pdf", b"".as_slice()),
        ("not-pdf.pdf", b"This is not a PDF.".as_slice()),
        (
            "truncated.pdf",
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog".as_slice(),
        ),
    ] {
        let malformed = temp.path().join(name);
        std::fs::write(&malformed, bytes).unwrap();
        let mut command = Command::new(&qpdf);
        command.arg("--check").arg(&malformed);
        assert_eq!(
            bounded(command, None).status.code(),
            Some(2),
            "qpdf should reject {name}"
        );
        let out = temp.path().join(format!("{name}.output.pdf"));
        let mut command = cli(&["rotate"], &[&malformed]);
        command.args(["--degrees", "90", "--out"]).arg(&out);
        let output = bounded(command, None);
        assert!(!output.status.success(), "CLI accepted {name}");
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).expect("typed CLI error, not a process crash");
        assert!(error["error"].is_string());
        assert!(!out.exists(), "failed operation left output for {name}");
        assert_eq!(std::fs::read(&malformed).unwrap(), bytes);
    }
    assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
}
