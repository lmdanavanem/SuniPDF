use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(action: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.args(["plan-set", action]);
    command
}

fn report(output: Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn create(source: &Path, out: &Path) -> Command {
    let mut command = cli("create");
    command
        .arg(source)
        .args(["--name", "Civic plans", "--out"])
        .arg(out);
    command
}

#[test]
fn plan_set_create_add_edit_remove_and_export_preserve_sources_and_rebase_paths() {
    let dir = tempfile::tempdir().unwrap();
    let plans = dir.path().join("plans");
    std::fs::create_dir(&plans).unwrap();
    let source = plans.join("Plans 'quoted' — étage.pdf");
    let additional = plans.join("Details.pdf");
    let first = plans.join("set.json");
    let second = plans.join("added.json");
    let edited = plans.join("edited.json");
    let removed = plans.join("removed.json");
    let exported = dir.path().join("rebased.json");
    let original_pdf = std::fs::read(fixture()).unwrap();
    std::fs::write(&source, &original_pdf).unwrap();
    let mut additional_pdf = original_pdf.clone();
    additional_pdf.extend_from_slice(b"\n% separate indexed document\n");
    std::fs::write(&additional, &additional_pdf).unwrap();

    let created = report(
        create(&source, &first)
            .args(["--discipline", "Architecture", "--revision", "A"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(created["sheets"], 2);
    assert_eq!(created["files"], 1);
    let original_manifest = std::fs::read(&first).unwrap();
    let initial = read(&first);
    let sheet_id = initial["sheets"][0]["id"].as_str().unwrap();
    assert_eq!(initial["sheets"][0]["discipline"], "Architecture");
    assert_eq!(initial["sheets"][0]["revision"], "A");
    assert_eq!(initial["sheets"][0]["page"], 1);

    let added = report(
        cli("add")
            .arg(&first)
            .arg("--pdf")
            .arg(&additional)
            .args(["--revision", "B", "--out"])
            .arg(&second)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(added["sheets"], 4);
    assert_eq!(added["files"], 2);
    report(
        cli("edit-sheet")
            .arg(&second)
            .args([
                "--id",
                sheet_id,
                "--number",
                "A-101",
                "--title",
                "Étage 'Nord' – level 1",
                "--discipline",
                "Structure",
                "--revision",
                "C",
                "--out",
            ])
            .arg(&edited)
            .output()
            .unwrap(),
        0,
    );
    let updated = read(&edited);
    assert_eq!(updated["sheets"][0]["id"], sheet_id);
    assert_eq!(updated["sheets"][0]["number"], "A-101");
    assert_eq!(updated["sheets"][0]["title"], "Étage 'Nord' – level 1");
    assert_eq!(updated["sheets"][0]["discipline"], "Structure");
    assert_eq!(updated["sheets"][0]["revision"], "C");
    let removed_result = report(
        cli("remove-sheet")
            .arg(&edited)
            .args(["--id", sheet_id, "--out"])
            .arg(&removed)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(removed_result["sheets"], 3);
    assert_eq!(removed_result["files"], 2);
    report(
        cli("export")
            .arg(&removed)
            .arg("--out")
            .arg(&exported)
            .output()
            .unwrap(),
        0,
    );
    let inspected = report(cli("inspect").arg(&exported).output().unwrap(), 0);
    assert_eq!(inspected["complete"], true);
    assert_eq!(inspected["manifest"]["sheets"].as_array().unwrap().len(), 3);
    assert_eq!(inspected["page_index_base"], 1);
    assert!(inspected["manifest"]["files"][0]["path"]
        .as_str()
        .unwrap()
        .starts_with("plans/"));
    assert!(inspected["files"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["status"] == "ready"));
    assert_eq!(std::fs::read(&first).unwrap(), original_manifest);
    assert_eq!(std::fs::read(source).unwrap(), original_pdf);
    assert_eq!(std::fs::read(additional).unwrap(), additional_pdf);
}

#[test]
fn plan_set_missing_changed_and_relocated_references_require_explicit_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir(&bundle).unwrap();
    let source = bundle.join("plan.pdf");
    let set = bundle.join("set.json");
    std::fs::copy(fixture(), &source).unwrap();
    report(create(&source, &set).output().unwrap(), 0);
    let original_manifest = std::fs::read(&set).unwrap();
    let initial = read(&set);
    let file_id = initial["files"][0]["id"].as_str().unwrap();
    let old_hash = initial["files"][0]["sha256"].clone();
    let relocated = dir.path().join("relocated");
    std::fs::rename(&bundle, &relocated).unwrap();
    let set = relocated.join("set.json");
    let source = relocated.join("plan.pdf");
    assert_eq!(
        report(cli("validate").arg(&set).output().unwrap(), 0)["complete"],
        true
    );

    let unavailable = relocated.join("moved-plan.pdf");
    std::fs::rename(&source, &unavailable).unwrap();
    let missing = report(cli("inspect").arg(&set).output().unwrap(), 2);
    assert_eq!(missing["complete"], false);
    assert_eq!(missing["files"][0]["status"], "missing");
    std::fs::rename(&unavailable, &source).unwrap();
    let mut changed_pdf = std::fs::read(&source).unwrap();
    changed_pdf.extend_from_slice(b"\n% explicit source revision\n");
    std::fs::write(&source, &changed_pdf).unwrap();
    let stale = report(cli("validate").arg(&set).output().unwrap(), 2);
    assert_eq!(stale["complete"], false);
    assert_eq!(stale["files"][0]["status"], "changed");

    let refused_out = relocated.join("unaccepted.json");
    assert!(!cli("refresh")
        .arg(&set)
        .args(["--file-id", file_id, "--out"])
        .arg(&refused_out)
        .output()
        .unwrap()
        .status
        .success());
    assert!(!refused_out.exists());
    let refreshed = relocated.join("refreshed.json");
    report(
        cli("refresh")
            .arg(&set)
            .args(["--file-id", file_id, "--accept-changed", "--out"])
            .arg(&refreshed)
            .output()
            .unwrap(),
        0,
    );
    let current = read(&refreshed);
    assert_ne!(current["files"][0]["sha256"], old_hash);
    assert_eq!(current["sheets"], initial["sheets"]);
    assert_eq!(
        report(cli("validate").arg(&refreshed).output().unwrap(), 0)["complete"],
        true
    );
    assert_eq!(std::fs::read(set).unwrap(), original_manifest);
    assert_eq!(std::fs::read(source).unwrap(), changed_pdf);
}

#[test]
fn plan_set_output_and_invalid_inputs_never_replace_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("plan.pdf");
    let set = dir.path().join("set.json");
    std::fs::copy(fixture(), &source).unwrap();
    report(create(&source, &set).output().unwrap(), 0);
    let before = std::fs::read(&set).unwrap();
    let pdf_before = std::fs::read(&source).unwrap();
    let existing = failure(
        cli("export")
            .arg(&set)
            .arg("--out")
            .arg(&set)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(existing["error"], "already_exists");
    assert_eq!(
        failure(
            create(&source, &source)
                .arg("--overwrite")
                .output()
                .unwrap()
        )["error"],
        "already_exists"
    );
    let unknown = dir.path().join("unknown.json");
    failure(
        cli("edit-sheet")
            .arg(&set)
            .args(["--id", "does-not-exist", "--title", "Ignored", "--out"])
            .arg(&unknown)
            .output()
            .unwrap(),
    );
    assert!(!unknown.exists());
    let missing = dir.path().join("not-found.pdf");
    let failed = dir.path().join("failed.json");
    failure(create(&source, &failed).arg(&missing).output().unwrap());
    assert!(
        !failed.exists(),
        "partial creation must not publish a manifest"
    );
    let invalid = dir.path().join("invalid.json");
    let mut value = read(&set);
    value["version"] = serde_json::json!(99);
    std::fs::write(&invalid, serde_json::to_vec(&value).unwrap()).unwrap();
    failure(cli("inspect").arg(&invalid).output().unwrap());
    let nested = dir.path().join("reports");
    std::fs::create_dir(&nested).unwrap();
    let outside_root = nested.join("outside-root.json");
    failure(
        cli("export")
            .arg(&set)
            .arg("--out")
            .arg(&outside_root)
            .output()
            .unwrap(),
    );
    assert!(
        !outside_root.exists(),
        "rebasing must not escape the new manifest root"
    );
    assert_eq!(std::fs::read(&set).unwrap(), before);
    assert_eq!(std::fs::read(&source).unwrap(), pdf_before);
}

#[test]
fn plan_set_password_sources_stay_out_of_manifests_and_incomplete_reports() {
    let dir = tempfile::tempdir().unwrap();
    let protected = dir.path().join("protected.pdf");
    let set = dir.path().join("secure-set.json");
    let password = "plan-set-fixture-passphrase";
    let protection = sunipdf_core::Protection {
        user_password: password.into(),
        owner_password: "plan-set-fixture-owner".into(),
        ..Default::default()
    };
    let encrypted =
        sunipdf_core::security::protect(&std::fs::read(fixture()).unwrap(), None, &protection)
            .unwrap();
    std::fs::write(&protected, &encrypted).unwrap();
    report(
        create(&protected, &set)
            .args(["--password-env", "SUNIPDF_PLAN_SET_TEST_PASSWORD"])
            .env("SUNIPDF_PLAN_SET_TEST_PASSWORD", password)
            .output()
            .unwrap(),
        0,
    );
    assert!(!std::fs::read_to_string(&set).unwrap().contains(password));
    let incomplete = report(cli("validate").arg(&set).output().unwrap(), 2);
    assert_eq!(incomplete["complete"], false);
    assert_eq!(incomplete["files"][0]["status"], "password_required");
    assert!(!incomplete.to_string().contains(password));
    let mut child = cli("inspect")
        .arg(&set)
        .arg("--password-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{password}\n").as_bytes())
        .unwrap();
    let checked = report(child.wait_with_output().unwrap(), 0);
    assert_eq!(checked["complete"], true);
    assert!(!checked.to_string().contains(password));
    assert_eq!(std::fs::read(protected).unwrap(), encrypted);
}
