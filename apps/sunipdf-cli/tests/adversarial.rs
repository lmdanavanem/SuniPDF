//! Deterministic bounded mutation smoke tests, not coverage-guided fuzzing.
#![cfg(windows)]

#[path = "../../../crates/sunipdf-core/tests/support/certificate_identity.rs"]
#[allow(dead_code)]
mod identity;
#[path = "support/bounded_process.rs"]
mod process;

use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn cli(operation: &str, path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(operation).arg(path);
    command
}

fn typed(output: &Output, signature: bool) {
    let code = output.status.code().expect("native process crashed");
    assert!(
        matches!(code, 0 | 1 | 3 | 4) || (signature && code == 2),
        "unexpected exit {code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if code == 0 || (signature && code == 2) {
        let _: Value = serde_json::from_slice(&output.stdout).expect("complete JSON report");
    } else {
        let report: Value =
            serde_json::from_slice(&output.stderr).expect("typed JSON error, never panic output");
        assert!(report["error"].is_string());
    }
}

#[test]
fn timeout_guard_terminates_and_reaps_a_stalled_child() {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Start-Sleep -Seconds 30",
    ]);
    let started = Instant::now();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process::bounded_for(
            command,
            Duration::from_millis(40)
        )))
        .is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn output_guard_rejects_an_excessive_child_report() {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[Console]::Out.Write([string]::new([char]88, 3145728))",
    ]);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process::bounded_for(
            command,
            Duration::from_secs(10)
        )))
        .is_err()
    );
}

#[test]
fn pdf_and_cms_mutations_keep_sources_and_emit_typed_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let seed = include_bytes!("../../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let temp = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let mut cases = Vec::new();
    for index in 0..32 {
        assert!(
            started.elapsed() < Duration::from_secs(180),
            "mutation campaign exceeded total budget"
        );
        let mut bytes = seed.to_vec();
        if index < 8 {
            bytes.truncate(index * seed.len() / 8);
        } else {
            let offset = (index * 97) % bytes.len();
            bytes[offset] ^= 1 << (index % 8);
        }
        let source = temp.path().join(format!("pdf-{index:02}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        let info = process::bounded_for(cli("info", &source), Duration::from_secs(10));
        typed(&info, false);
        let destination = temp.path().join(format!("pdf-{index:02}-optimized.pdf"));
        let mut command = cli("optimize", &source);
        command.arg("--out").arg(&destination);
        let edited = process::bounded_for(command, Duration::from_secs(10));
        typed(&edited, false);
        if edited.status.success() {
            let reopened = process::bounded_for(cli("info", &destination), Duration::from_secs(10));
            typed(&reopened, false);
            assert!(reopened.status.success(), "successful output must reopen");
        } else {
            assert!(!destination.exists(), "failed edit published an output");
        }
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        cases.push(json!({"kind":"pdf","index":index,"info_exit":info.status.code(),"optimize_exit":edited.status.code(),"source_unchanged":true}));
    }
    let password = format!(
        "fixture-{}",
        temp.path().file_name().unwrap().to_string_lossy()
    );
    let pfx = identity::identity_with_password(&password);
    let signed = sunipdf_core::certificate::sign_pdf(
        seed,
        &pfx,
        &password,
        &sunipdf_core::certificate::CertSignOptions::default(),
    )
    .unwrap();
    let valid = temp.path().join("original-signed.pdf");
    std::fs::write(&valid, &signed).unwrap();
    let baseline = process::bounded_for(cli("signatures", &valid), Duration::from_secs(10));
    typed(&baseline, true);
    assert!(
        baseline.status.success(),
        "positive CMS control must verify"
    );
    let contents = signed
        .windows(b"/Contents".len())
        .rposition(|bytes| bytes == b"/Contents")
        .unwrap();
    let start = contents
        + signed[contents..]
            .iter()
            .position(|&byte| byte == b'<')
            .unwrap()
        + 1;
    assert!(signed[start..start + 128].iter().all(u8::is_ascii_hexdigit));
    for index in 0..32 {
        assert!(
            started.elapsed() < Duration::from_secs(180),
            "mutation campaign exceeded total budget"
        );
        let mut bytes = signed.clone();
        let offset = start + index * 4;
        bytes[offset] = if bytes[offset] == b'0' { b'1' } else { b'0' };
        let source = temp.path().join(format!("cms-{index:02}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        let checked = process::bounded_for(cli("signatures", &source), Duration::from_secs(10));
        typed(&checked, true);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        cases.push(json!({"kind":"cms","index":index,"exit_code":checked.status.code(),"source_unchanged":true}));
    }
    assert_eq!(std::fs::read(valid).unwrap(), signed);
    let report = json!({"campaign":"deterministic mutation smoke","cases":cases,"elapsed_ms":started.elapsed().as_millis(),"commands":*process::COMMAND_METRICS.lock().unwrap(),"hardware":process::hardware(),"per_command_timeout_seconds":10,"campaign_budget_seconds":180,"memory_guard_bytes":process::MAX_MEMORY,"output_limit_bytes":process::MAX_OUTPUT,"production_identity_used":false,"coverage_guided_fuzzing":false});
    let report_directory = root.join("target");
    std::fs::create_dir_all(&report_directory).unwrap();
    std::fs::write(
        report_directory.join("adversarial-corpus-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
