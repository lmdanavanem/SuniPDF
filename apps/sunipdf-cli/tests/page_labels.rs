//! Page-label commands round-trip through the CLI and native PDFium without changing page content.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use sunipdf_core::{security, Engine, Protection};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/page-labels.pdf")
}

fn cli(action: &str, source: &Path, out: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(action).arg(source).arg("--out").arg(out);
    command
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
fn labels_styles_defaults_reset_and_protected_copies_round_trip() {
    // PDFium is initialized once for all native cases in this integration-test process.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = fixture();
    let original = std::fs::read(&source).unwrap();
    let before = engine.open(&source, None).unwrap();
    let count = before.page_count();
    assert_eq!(count, 9);
    for (style, start, suffixes) in [
        ("decimal", "9", ["9", "10", "11"]),
        ("roman-upper", "9", ["IX", "X", "XI"]),
        ("roman-lower", "9", ["ix", "x", "xi"]),
        ("letters-upper", "26", ["Z", "AA", "BB"]),
        ("letters-lower", "26", ["z", "aa", "bb"]),
        ("prefix-only", "1", ["", "", ""]),
    ] {
        let out = temp.path().join(format!("{style}.pdf"));
        let report = success(
            cli("set-page-labels", &source, &out)
                .args([
                    "--from", "2", "--to", "4", "--style", style, "--start", start, "--prefix",
                    "É-",
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(report["from"], 2);
        assert_eq!(report["to"], 4);
        assert_eq!(report["count"], 3);
        let after = engine.open(&out, None).unwrap();
        assert_eq!(after.page_count(), count);
        for page in 0..count {
            if (1..=3).contains(&page) {
                assert_eq!(
                    after.info().page_label(page),
                    Some(format!("É-{}", suffixes[page - 1]).as_str())
                );
            } else {
                assert_eq!(
                    after.info().page_label(page),
                    before.info().page_label(page)
                );
            }
            assert_eq!(
                after.page_text(page).unwrap(),
                before.page_text(page).unwrap()
            );
            assert_eq!(
                after.render_page(page, 0.2).unwrap().rgba,
                before.render_page(page, 0.2).unwrap().rgba
            );
        }
        let mut info = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
        let info = success(info.arg("info").arg(&out).output().unwrap());
        let reported: Vec<_> = info["pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|page| page["label"].clone())
            .collect();
        assert_eq!(
            serde_json::Value::Array(reported),
            serde_json::json!(after.info().page_labels)
        );
    }
    let all = temp.path().join("default-all.pdf");
    let report = success(cli("set-page-labels", &source, &all).output().unwrap());
    assert_eq!(report["from"], 1);
    assert_eq!(report["to"], count);
    assert_eq!(report["count"], count);
    let all_doc = engine.open(&all, None).unwrap();
    for page in 0..count {
        assert_eq!(
            all_doc.info().page_label(page),
            Some((page + 1).to_string().as_str())
        );
    }
    let reset = temp.path().join("reset.pdf");
    assert_eq!(
        success(cli("reset-page-labels", &all, &reset).output().unwrap())["count"],
        count
    );
    assert!(engine
        .open(&reset, None)
        .unwrap()
        .info()
        .page_labels
        .iter()
        .all(Option::is_none));
    let unlabelled = source.with_file_name("two-pages.pdf");
    let subset = temp.path().join("previously-unlabelled-subset.pdf");
    assert_eq!(
        success(
            cli("set-page-labels", &unlabelled, &subset)
                .args(["--from", "2", "--to", "2", "--prefix", "Appendix-"])
                .output()
                .unwrap()
        )["count"],
        1
    );
    let subset = engine.open(&subset, None).unwrap();
    assert_eq!(subset.info().page_label(0), Some("1"));
    assert_eq!(subset.info().page_label(1), Some("Appendix-1"));
    for aes256 in [false, true] {
        let protected = security::protect(
            &original,
            None,
            &Protection {
                user_password: "label-user".into(),
                owner_password: "label-owner".into(),
                allow_print: false,
                allow_copy: false,
                allow_annotate: false,
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let source = temp.path().join(format!("protected-{aes256}.pdf"));
        std::fs::write(&source, &protected).unwrap();
        for opening_password in ["label-user", "label-owner"] {
            let out = temp
                .path()
                .join(format!("labelled-{aes256}-{opening_password}.pdf"));
            let report = success(
                cli("set-page-labels", &source, &out)
                    .args([
                        "--from",
                        "2",
                        "--to",
                        "4",
                        "--style",
                        "roman-lower",
                        "--start",
                        "9",
                        "--prefix",
                        "É-",
                        "--password",
                        opening_password,
                    ])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["count"], 3);
            assert!(engine.open(&out, None).is_err());
            for password in ["label-user", "label-owner"] {
                let doc = engine.open(&out, Some(password)).unwrap();
                assert_eq!(doc.info().page_label(1), Some("É-ix"));
                assert_eq!(doc.info().page_label(3), Some("É-xi"));
                let properties = doc.properties();
                assert!(properties.encrypted);
                assert!(!properties.can_copy && !properties.can_print && !properties.can_annotate);
                assert!(properties.can_modify);
                for page in 0..count {
                    if !(1..=3).contains(&page) {
                        assert_eq!(doc.info().page_label(page), before.info().page_label(page));
                    }
                    assert_eq!(
                        doc.page_text(page).unwrap(),
                        before.page_text(page).unwrap()
                    );
                    assert_eq!(
                        doc.render_page(page, 0.2).unwrap().rgba,
                        before.render_page(page, 0.2).unwrap().rgba
                    );
                }
            }
            let reset = temp
                .path()
                .join(format!("reset-{aes256}-{opening_password}.pdf"));
            assert_eq!(
                success(
                    cli("reset-page-labels", &out, &reset)
                        .args(["--password", opening_password])
                        .output()
                        .unwrap()
                )["count"],
                count
            );
            assert!(engine.open(&reset, None).is_err());
            for password in ["label-user", "label-owner"] {
                let doc = engine.open(&reset, Some(password)).unwrap();
                assert!(doc.info().page_labels.iter().all(Option::is_none));
                let properties = doc.properties();
                assert!(properties.encrypted);
                assert!(!properties.can_copy && !properties.can_print && !properties.can_annotate);
                for page in 0..count {
                    assert_eq!(
                        doc.page_text(page).unwrap(),
                        before.page_text(page).unwrap()
                    );
                }
            }
            assert_eq!(std::fs::read(&source).unwrap(), protected);
        }
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn invalid_labels_and_existing_output_never_write_partial_results() {
    let temp = tempfile::tempdir().unwrap();
    let source = fixture();
    let original = std::fs::read(&source).unwrap();
    let long_prefix = "x".repeat(257);
    for (index, args) in [
        vec!["--from", "0"],
        vec!["--from", "10"],
        vec!["--to", "0"],
        vec!["--to", "10"],
        vec!["--from", "4", "--to", "2"],
        vec!["--start", "0"],
        vec!["--start", "4294967295"],
        vec!["--style", "unknown"],
        vec!["--style", "prefix-only"],
        vec!["--prefix", "bad\nlabel"],
        vec!["--prefix", long_prefix.as_str()],
    ]
    .into_iter()
    .enumerate()
    {
        let out = temp.path().join(format!("invalid-{index}.pdf"));
        let result = cli("set-page-labels", &source, &out)
            .args(&args)
            .output()
            .unwrap();
        assert!(!result.status.success(), "invalid case {index}");
        assert!(!out.exists(), "invalid case {index} created an output");
        let sentinel = b"existing output must survive invalid input";
        std::fs::write(&out, sentinel).unwrap();
        let result = cli("set-page-labels", &source, &out)
            .args(&args)
            .arg("--overwrite")
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "overwrite accepted invalid case {index}"
        );
        assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    }
    for action in ["set-page-labels", "reset-page-labels"] {
        let out = temp.path().join(format!("existing-{action}.pdf"));
        let sentinel = b"existing output";
        std::fs::write(&out, sentinel).unwrap();
        let result = cli(action, &source, &out).output().unwrap();
        assert_eq!(result.status.code(), Some(6));
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"], "already_exists");
        assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}
