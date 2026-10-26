use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::AtomicBool;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn batch(action: &str, file: &Path) -> Command {
    let mut cli = command();
    cli.args(["annotation-fields-batch", action]).arg(file);
    cli
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn refused(output: Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let _: Value = serde_json::from_slice(&output.stderr).unwrap();
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn seed(root: &Path) -> PathBuf {
    let mut source = fixture();
    for (index, page, group) in [(0, 1, "Lights"), (1, 2, "Sockets"), (2, 1, "Untouched")] {
        let output = root.join(format!("count-{index}.pdf"));
        success(
            command()
                .arg("count")
                .arg(&source)
                .args([
                    "--page",
                    &page.to_string(),
                    "--group",
                    group,
                    "--point",
                    "120,120",
                    "--out",
                ])
                .arg(&output)
                .output()
                .unwrap(),
        );
        source = output;
    }
    for (page, room, status) in [(1, "A", "Open"), (2, "B", "Pending")] {
        let output = root.join(format!("fields-{page}.pdf"));
        success(
            command()
                .arg("annotation-fields")
                .arg(&source)
                .args([
                    "--page",
                    &page.to_string(),
                    "--index",
                    "0",
                    "--business-status",
                    status,
                    "--discipline",
                    "Electrical",
                    "--field",
                    &format!("Room={room}"),
                    "--field",
                    &format!("Keep={room}"),
                    "--out",
                ])
                .arg(&output)
                .output()
                .unwrap(),
        );
        source = output;
    }
    source
}

fn inventory(source: &Path) -> Value {
    success(batch("inspect", source).output().unwrap())
}

fn plan(inventory: &Value) -> Value {
    let targets: Vec<_> = inventory["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["index"] == 0)
        .map(|row| {
            json!({"page":row["page"], "index":row["index"],
            "expected_row_sha256":row["expected_row_sha256"]})
        })
        .collect();
    json!({"version":1,"expected_source_sha256":inventory["source_file_sha256"],"targets":targets,
        "patch":{"business_status":{"operation":"set","value":"Reviewed"},
            "discipline":{"operation":"clear"},
            "custom":{"Room":{"operation":"clear"},"Note":{"operation":"set","value":"Checked"}}}})
}

#[test]
fn batch_cli_previews_and_saves_only_explicit_fields_with_atomic_refusals() {
    let directory = tempfile::tempdir().unwrap();
    let source = seed(directory.path());
    let original = std::fs::read(&source).unwrap();
    let before = inventory(&source);
    assert_eq!(before["selected_rows"], 0);
    assert_eq!(before["rows"].as_array().unwrap().len(), 3);
    let selection = plan(&before);
    let input = directory.path().join("plan.json");
    write_json(&input, &selection);
    let preview = success(
        batch("preview", &source)
            .arg("--plan")
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(preview["committed"], false);
    assert_eq!(preview["changes"]["selected_count"], 2);
    assert_eq!(preview["changes"]["changed_count"], 2);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let output = directory.path().join("reviewed.pdf");
    let applied = success(
        batch("apply", &source)
            .arg("--plan")
            .arg(&input)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert_eq!(applied["committed"], true);
    assert_eq!(applied["changes"]["changes"], preview["changes"]["changes"]);
    let after = inventory(&output);
    for original_row in before["rows"].as_array().unwrap() {
        let row = after["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["page"] == original_row["page"] && row["index"] == original_row["index"]
            })
            .unwrap();
        assert_eq!(row["annotation_id"], original_row["annotation_id"]);
        if row["index"] == 0 {
            assert_eq!(row["fields"]["business_status"], "Reviewed");
            assert!(row["fields"]["discipline"].is_null());
            assert!(row["fields"]["custom"].get("Room").is_none());
            assert_eq!(row["fields"]["custom"]["Note"], "Checked");
            assert_eq!(
                row["fields"]["custom"]["Keep"],
                original_row["fields"]["custom"]["Keep"]
            );
        } else {
            assert_eq!(row, original_row);
        }
    }
    let register = success(
        command()
            .arg("annotation-register")
            .arg(&output)
            .output()
            .unwrap(),
    );
    let rows = register["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (page, index, group) in [(1, 0, "Lights"), (2, 0, "Sockets"), (1, 1, "Untouched")] {
        let row = rows
            .iter()
            .find(|row| row["page"] == page && row["index"] == index)
            .unwrap();
        assert_eq!(row["count_group"], group);
    }

    let output_bytes = std::fs::read(&output).unwrap();
    for existing in [&output, &source] {
        refused(
            batch("apply", &source)
                .arg("--plan")
                .arg(&input)
                .arg("--out")
                .arg(existing)
                .arg("--overwrite")
                .output()
                .unwrap(),
        );
    }
    assert_eq!(std::fs::read(&output).unwrap(), output_bytes);
    let mut invalid = selection.clone();
    invalid["targets"][1]["expected_row_sha256"] = json!("0".repeat(64));
    write_json(&input, &invalid);
    let absent = directory.path().join("refused.pdf");
    refused(
        batch("apply", &source)
            .arg("--plan")
            .arg(&input)
            .arg("--out")
            .arg(&absent)
            .output()
            .unwrap(),
    );
    assert!(!absent.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    write_json(&input, &selection);
    let changed_source = directory.path().join("changed.pdf");
    std::fs::write(&changed_source, &original).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&changed_source)
        .unwrap()
        .write_all(b"\n% external revision\n")
        .unwrap();
    refused(
        batch("apply", &changed_source)
            .arg("--plan")
            .arg(&input)
            .arg("--out")
            .arg(&absent)
            .output()
            .unwrap(),
    );
    assert!(!absent.exists());

    let mut unchanged = selection.clone();
    unchanged["patch"] = json!({});
    write_json(&input, &unchanged);
    let preview = success(
        batch("preview", &source)
            .arg("--plan")
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(preview["changes"]["changed_count"], 0);
    refused(
        batch("apply", &source)
            .arg("--plan")
            .arg(&input)
            .arg("--out")
            .arg(&absent)
            .output()
            .unwrap(),
    );
    assert!(!absent.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

fn secret_free(output: &Output, secrets: &[&str]) {
    for secret in secrets {
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains(secret),
            "stdout must not disclose credentials"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains(secret),
            "stderr must not disclose credentials"
        );
    }
}

fn authenticated(mut cli: Command, credential: &str) -> Command {
    cli.args(["--password-env", "SUNIPDF_BATCH_TEST_OPENING"])
        .env("SUNIPDF_BATCH_TEST_OPENING", credential);
    assert!(
        !cli.get_args().any(|argument| argument == credential),
        "credentials must not enter argv"
    );
    cli
}

fn piped_password(mut cli: Command, credential: &str) -> Output {
    cli.arg("--password-stdin");
    assert!(
        !cli.get_args().any(|argument| argument == credential),
        "credentials must not enter argv"
    );
    let mut child = cli
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{credential}").unwrap();
    drop(input);
    child.wait_with_output().unwrap()
}

fn checked_success(output: Output, secrets: &[&str]) -> Value {
    secret_free(&output, secrets);
    success(output)
}

fn checked_failure(output: Output, secrets: &[&str], expected: &str) {
    secret_free(&output, secrets);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], expected);
}

#[test]
fn protected_batches_use_env_and_stdin_preserve_encryption_and_respect_both_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let plain = seed(directory.path());
    let plain_bytes = std::fs::read(&plain).unwrap();
    let plain_inventory = inventory(&plain);
    let nonce = format!(
        "{}-{}-{}",
        directory.path().file_name().unwrap().to_string_lossy(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let opening = format!("opening-{nonce}");
    let owner = format!("owner-{nonce}");
    let wrong = format!("incorrect-{nonce}");
    let secrets = [opening.as_str(), owner.as_str(), wrong.as_str()];
    for aes128 in [false, true] {
        for restriction in ["allowed", "no-copy", "no-annotate"] {
            let protected = directory
                .path()
                .join(format!("protected-{aes128}-{restriction}.pdf"));
            let mut protect = command();
            protect
                .arg("protect")
                .arg(&plain)
                .args([
                    "--user-password-env",
                    "SUNIPDF_BATCH_TEST_OPENING",
                    "--owner-password-env",
                    "SUNIPDF_BATCH_TEST_OWNER",
                    "--no-print",
                    "--no-modify",
                    "--out",
                ])
                .arg(&protected)
                .env("SUNIPDF_BATCH_TEST_OPENING", &opening)
                .env("SUNIPDF_BATCH_TEST_OWNER", &owner);
            if aes128 {
                protect.arg("--aes128");
            }
            if restriction != "allowed" {
                protect.arg(format!("--{restriction}"));
            }
            assert!(!protect
                .get_args()
                .any(|argument| secrets.iter().any(|secret| argument == *secret)));
            checked_success(protect.output().unwrap(), &secrets);
            let encrypted_original = std::fs::read(&protected).unwrap();
            let raw = lopdf::Document::load_mem(&encrypted_original).unwrap();
            assert!(raw.is_encrypted());
            let before_encryption = raw.get_encrypted().unwrap().clone();
            let (saved, _) = sunipdf_core::issue_report::SavedReportSource::capture(
                &protected,
                None,
                &AtomicBool::new(false),
            )
            .unwrap();
            let mut fallback_plan = plan(&plain_inventory);
            fallback_plan["expected_source_sha256"] = json!(saved.sha256());
            for (credential_name, credential) in
                [("opening", opening.as_str()), ("owner", owner.as_str())]
            {
                let inspected = authenticated(batch("inspect", &protected), credential)
                    .output()
                    .unwrap();
                let selection = if restriction == "no-copy" {
                    checked_failure(inspected, &secrets, "security_restricted");
                    // Explicit rows from the original plaintext still cannot bypass copy restrictions.
                    fallback_plan.clone()
                } else {
                    plan(&checked_success(inspected, &secrets))
                };
                let plan_path = directory.path().join(format!(
                    "plan-{aes128}-{restriction}-{credential_name}.json"
                ));
                write_json(&plan_path, &selection);
                let mut preview_command = batch("preview", &protected);
                preview_command.arg("--plan").arg(&plan_path);
                let preview = authenticated(preview_command, credential).output().unwrap();
                let output = directory.path().join(format!(
                    "edited-{aes128}-{restriction}-{credential_name}.pdf"
                ));
                let mut apply = batch("apply", &protected);
                apply
                    .arg("--plan")
                    .arg(&plan_path)
                    .arg("--out")
                    .arg(&output);
                let applied = piped_password(apply, credential);
                if restriction != "allowed" {
                    checked_failure(preview, &secrets, "security_restricted");
                    checked_failure(applied, &secrets, "security_restricted");
                    assert!(!output.exists(), "refused edits cannot leave a destination");
                    continue;
                }
                let preview = checked_success(preview, &secrets);
                assert_eq!(preview["committed"], false);
                assert_eq!(preview["changes"]["changed_count"], 2);
                let applied = checked_success(applied, &secrets);
                assert_eq!(applied["encrypted"], true);
                assert_eq!(applied["changes"]["changes"], preview["changes"]["changes"]);
                let actual = lopdf::Document::load_mem(&std::fs::read(&output).unwrap()).unwrap();
                assert!(actual.is_encrypted());
                let after_encryption = actual.get_encrypted().unwrap();
                for key in [b"V".as_slice(), b"R", b"P", b"Length"] {
                    assert_eq!(
                        after_encryption.get(key).unwrap(),
                        before_encryption.get(key).unwrap(),
                        "encryption version, strength and permission bits must survive editing"
                    );
                }
                checked_failure(
                    batch("inspect", &output).output().unwrap(),
                    &secrets,
                    "password_required",
                );
                checked_failure(
                    authenticated(batch("inspect", &output), &wrong)
                        .output()
                        .unwrap(),
                    &secrets,
                    "password_required",
                );
                let reopened = checked_success(
                    piped_password(batch("inspect", &output), credential),
                    &secrets,
                );
                let old_rows = plain_inventory["rows"].as_array().unwrap();
                for row in reopened["rows"].as_array().unwrap() {
                    let original = old_rows
                        .iter()
                        .find(|old| old["page"] == row["page"] && old["index"] == row["index"])
                        .unwrap();
                    assert_eq!(row["annotation_id"], original["annotation_id"]);
                    if row["index"] == 0 {
                        assert_eq!(row["fields"]["business_status"], "Reviewed");
                        assert_eq!(
                            row["fields"]["custom"]["Keep"],
                            original["fields"]["custom"]["Keep"]
                        );
                    } else {
                        assert_eq!(row, original);
                    }
                }
                let mut properties = command();
                properties.arg("properties").arg(&output);
                let properties = checked_success(
                    authenticated(properties, credential).output().unwrap(),
                    &secrets,
                );
                assert_eq!(properties["encrypted"], true);
                assert_eq!(
                    properties["permissions"],
                    json!({"print":false, "copy":true, "modify":false, "annotate":true})
                );
            }
            assert_eq!(std::fs::read(protected).unwrap(), encrypted_original);
        }
    }
    assert_eq!(std::fs::read(plain).unwrap(), plain_bytes);
}
