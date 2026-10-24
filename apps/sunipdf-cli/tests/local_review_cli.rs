use std::{
    path::Path,
    process::{Command, Output},
};
use sunipdf_core::{Engine, TextStyle};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .args(args)
        .output()
        .unwrap()
}
fn report(output: &Output, code: i32) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn local_review_cli_emits_full_partial_reports_and_uses_non_argv_passwords() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let left = temp.path().join("original.pdf");
    let right = temp.path().join("revised.pdf");
    let protected = temp.path().join("protected.pdf");
    let missing = temp.path().join("missing.pdf");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let mut doc = engine.open(&fixture, None).unwrap();
    doc.add_text(0, 30.0, 400.0, "UniqueNeedle", &TextStyle::default())
        .unwrap();
    doc.save_as(&left).unwrap();
    doc.add_text(1, 30.0, 350.0, "Added text", &TextStyle::default())
        .unwrap();
    doc.save_as(&right).unwrap();
    let a = left.to_str().unwrap();
    let b = right.to_str().unwrap();
    let original = std::fs::read(&left).unwrap();
    let revised = std::fs::read(&right).unwrap();
    let search = report(&run(&["search-files", a, b, "--query", "uniqueneedle"]), 0);
    assert_eq!(search["complete"], true);
    assert_eq!(search["files"][0]["hits"][0]["page"], 1);
    let partial = report(
        &run(&[
            "search-files",
            a,
            missing.to_str().unwrap(),
            "--query",
            "UniqueNeedle",
        ]),
        2,
    );
    assert_eq!(partial["complete"], false);
    assert_eq!(partial["files"][1]["error"], "io");
    assert_eq!(partial["files"][0]["hits"][0]["occurrences"], 1);
    let equal = report(&run(&["compare", a, a]), 0);
    assert!(equal["pages"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["changed"] == false));
    let changed = report(&run(&["compare", a, b]), 0);
    assert_eq!(changed["pages"][1]["text_changed"], true);
    assert!(changed["pages"][1]["changed_pixels"].as_u64().unwrap() > 0);
    let partial = report(&run(&["compare", a, b, "--max-pages", "1"]), 2);
    assert_eq!(partial["complete"], false);
    assert!(!run(&["compare", a, b, "--left-start", "0"])
        .status
        .success());
    let protection = sunipdf_core::Protection {
        user_password: "review-cli-test".into(),
        owner_password: "review-cli-owner".into(),
        ..Default::default()
    };
    let encrypted = sunipdf_core::security::protect(&original, None, &protection).unwrap();
    std::fs::write(&protected, &encrypted).unwrap();
    let p = protected.to_str().unwrap();
    let unopened = report(&run(&["search-files", p, "--query", "UniqueNeedle"]), 2);
    assert_eq!(unopened["files"][0]["error"], "password_required");
    for args in [
        vec![
            "search-files",
            p,
            "--query",
            "UniqueNeedle",
            "--password-env",
            "SUNIPDF_REVIEW_TEST_PASSWORD",
        ],
        vec![
            "compare",
            p,
            p,
            "--password-env",
            "SUNIPDF_REVIEW_TEST_PASSWORD",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
            .args(args)
            .env("SUNIPDF_REVIEW_TEST_PASSWORD", "review-cli-test")
            .output()
            .unwrap();
        assert_eq!(report(&output, 0)["complete"], true);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("review-cli-test"));
    }
    assert_eq!(std::fs::read(left).unwrap(), original);
    assert_eq!(std::fs::read(right).unwrap(), revised);
    assert_eq!(std::fs::read(protected).unwrap(), encrypted);
}
