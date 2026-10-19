use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(command: &str, source: &Path) -> Command {
    let mut command_line = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command_line.arg(command).arg(source);
    command_line
}

fn piped(mut command: Command, value: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(value).unwrap();
    child.wait_with_output().unwrap()
}

fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn passwords_use_environment_and_stdin_without_secret_argv_and_reprotect_requires_owner() {
    let directory = tempfile::tempdir().unwrap();
    let opening = format!("opening-été-{}", std::process::id());
    let owner = format!("owner-propriétaire-{}", std::process::id());
    for aes128 in [false, true] {
        let encrypted = directory.path().join(format!("protected-{aes128}.pdf"));
        let mut protect = cli("protect", &fixture());
        protect
            .args([
                "--user-password-env",
                "SUNIPDF_TEST_OPENING",
                "--owner-password-stdin",
                "--no-copy",
                "--no-modify",
                "--out",
            ])
            .arg(&encrypted)
            .env("SUNIPDF_TEST_OPENING", &opening);
        if aes128 {
            protect.arg("--aes128");
        }
        assert!(!protect
            .get_args()
            .any(|argument| argument == opening.as_str() || argument == owner.as_str()));
        success(piped(protect, format!("{owner}\r\n").as_bytes()));
        let original = std::fs::read(&encrypted).unwrap();

        let mut info = cli("info", &encrypted);
        info.args(["--password-env", "SUNIPDF_TEST_OPENING"])
            .env("SUNIPDF_TEST_OPENING", &opening);
        assert_eq!(success(info.output().unwrap())["page_count"], 2);
        let mut info = cli("info", &encrypted);
        info.arg("--password-stdin");
        assert_eq!(
            success(piped(info, format!("{opening}\n").as_bytes()))["page_count"],
            2
        );

        let denied = directory.path().join(format!("denied-{aes128}.pdf"));
        let mut reprotect = cli("protect", &encrypted);
        reprotect
            .args([
                "--password-env",
                "SUNIPDF_TEST_OPENING",
                "--owner-password-env",
                "SUNIPDF_TEST_OWNER",
                "--out",
            ])
            .arg(&denied)
            .env("SUNIPDF_TEST_OPENING", &opening)
            .env("SUNIPDF_TEST_OWNER", &owner);
        assert_eq!(reprotect.output().unwrap().status.code(), Some(3));
        assert!(!denied.exists());

        let unprotected = directory.path().join(format!("unprotected-{aes128}.pdf"));
        let mut unprotect = cli("unprotect", &encrypted);
        unprotect
            .args(["--password-stdin", "--out"])
            .arg(&unprotected);
        success(piped(unprotect, owner.as_bytes()));
        assert_eq!(
            success(cli("info", &unprotected).output().unwrap())["page_count"],
            2
        );
        assert_eq!(std::fs::read(&encrypted).unwrap(), original);
    }
}

#[test]
fn secret_input_errors_do_not_echo_values_and_help_marks_legacy_arguments_unsafe() {
    let secret = format!("private-input-{}", std::process::id());
    let mut conflicting = cli("info", &fixture());
    conflicting
        .args([
            "--password",
            &secret,
            "--password-env",
            "SUNIPDF_TEST_SECRET",
        ])
        .env("SUNIPDF_TEST_SECRET", &secret);
    let output = conflicting.output().unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(&secret));
    assert!(output.stdout.is_empty());

    let mut missing = cli("info", &fixture());
    missing
        .args(["--password-env", "SUNIPDF_TEST_MISSING"])
        .env_remove("SUNIPDF_TEST_MISSING");
    assert!(!missing.output().unwrap().status.success());

    let mut invalid_utf8 = cli("info", &fixture());
    invalid_utf8.arg("--password-stdin");
    assert!(!piped(invalid_utf8, &[0xff]).status.success());
    let mut excessive = cli("info", &fixture());
    excessive.arg("--password-stdin");
    assert!(!piped(excessive, &vec![b'x'; 65_537]).status.success());

    let directory = tempfile::tempdir().unwrap();
    let unused_output = directory.path().join("unused-secret-output.pdf");
    let mut two_reads = cli("protect", &fixture());
    two_reads
        .args(["--user-password-stdin", "--owner-password-stdin", "--out"])
        .arg(&unused_output);
    let output = piped(two_reads, b"");
    assert!(!output.status.success());
    assert!(!unused_output.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .args(["info", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.contains("--password-env")
            && help.contains("--password-stdin")
            && help.contains("Unsafe")
    );
}
