//! Existing link annotations keep their appearance, neighbours and protected-document state.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use sunipdf_core::{form_data, security, Document, Engine, LinkTarget, Protection};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

fn cli(source: &Path, out: &Path, index: usize) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command
        .arg("set-link-target")
        .arg(source)
        .arg("--index")
        .arg(index.to_string())
        .arg("--out")
        .arg(out);
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

fn unchanged_except_target(before: &Document<'_>, after: &Document<'_>, index: usize) {
    assert_eq!(after.page_count(), before.page_count());
    for page in 0..before.page_count() {
        assert_eq!(
            after.page_size(page).unwrap(),
            before.page_size(page).unwrap()
        );
        assert_eq!(
            after.page_text(page).unwrap(),
            before.page_text(page).unwrap()
        );
        assert_eq!(
            after.render_page(page, 0.2).unwrap().rgba,
            before.render_page(page, 0.2).unwrap().rgba
        );
        let mut expected = before.annotation_details(page).unwrap();
        let actual = after.annotation_details(page).unwrap();
        if page == 0 {
            expected[index].link = actual[index].link.clone();
            expected[index].modified = actual[index].modified.clone();
            assert!(actual[index].modified.is_some());
        }
        assert_eq!(actual, expected);
    }
    assert_eq!(
        form_data::info_to_json(&after.form_info().unwrap()),
        form_data::info_to_json(&before.form_info().unwrap())
    );
}

#[test]
fn retargets_foreign_links_and_preserves_content_forms_and_both_encrypted_roles() {
    // All native checks share this one engine instance in this test process.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = fixture("annotated.pdf");
    let original = std::fs::read(&source).unwrap();
    let before = engine.open(&source, None).unwrap();
    for (index, args, target, json) in [
        (
            7,
            ["--goto-page", "2"],
            LinkTarget::Page(1),
            serde_json::json!({"page":2}),
        ),
        (
            9,
            ["--url", "  https://example.org/retargeted  "],
            LinkTarget::Uri("https://example.org/retargeted".into()),
            serde_json::json!({"uri":"https://example.org/retargeted"}),
        ),
        (
            10,
            ["--url", "mailto:reader@example.org"],
            LinkTarget::Uri("mailto:reader@example.org".into()),
            serde_json::json!({"uri":"mailto:reader@example.org"}),
        ),
    ] {
        let out = temp.path().join(format!("retargeted-{index}.pdf"));
        let report = success(cli(&source, &out, index).args(args).output().unwrap());
        assert_eq!(report["page"], 1);
        assert_eq!(report["index"], index);
        assert_eq!(report["link"], json);
        assert_eq!(report["out"], out.to_string_lossy().as_ref());
        let after = engine.open(&out, None).unwrap();
        assert_eq!(
            after.annotation_details(0).unwrap()[index].link,
            Some(target)
        );
        unchanged_except_target(&before, &after, index);
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let source = fixture("acroform-links.pdf");
    let original = std::fs::read(&source).unwrap();
    let before = engine.open(&source, None).unwrap();
    let index = before.page_links(0).unwrap()[0].index;
    for aes256 in [false, true] {
        let protected = security::protect(
            &original,
            None,
            &Protection {
                user_password: "link-user".into(),
                owner_password: "link-owner".into(),
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
        for opening_password in ["link-user", "link-owner"] {
            let out = temp
                .path()
                .join(format!("retargeted-{aes256}-{opening_password}.pdf"));
            let report = success(
                cli(&source, &out, index)
                    .args(["--goto-page", "2", "--password", opening_password])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["link"], serde_json::json!({"page":2}));
            assert!(engine.open(&out, None).is_err());
            for password in ["link-user", "link-owner"] {
                let after = engine.open(&out, Some(password)).unwrap();
                assert_eq!(
                    after.annotation_details(0).unwrap()[index].link,
                    Some(LinkTarget::Page(1))
                );
                let properties = after.properties();
                assert!(properties.encrypted);
                assert!(!properties.can_copy && !properties.can_print && !properties.can_annotate);
                assert!(properties.can_modify);
                unchanged_except_target(&before, &after, index);
            }
            assert_eq!(std::fs::read(&source).unwrap(), protected);
        }
        let denied = temp.path().join(format!("bad-password-{aes256}.pdf"));
        assert!(!cli(&source, &denied, index)
            .args(["--goto-page", "2", "--password", "wrong"])
            .output()
            .unwrap()
            .status
            .success());
        assert!(!denied.exists());
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn invalid_targets_and_existing_outputs_never_write_partial_results() {
    let temp = tempfile::tempdir().unwrap();
    let source = fixture("annotated.pdf");
    let original = std::fs::read(&source).unwrap();
    for (case, args) in [
        vec!["--page", "0", "--goto-page", "2"],
        vec!["--page", "999", "--goto-page", "2"],
        vec!["--goto-page", "0"],
        vec!["--goto-page", "999"],
        vec!["--url", "javascript:alert(1)"],
        vec!["--url", "file:///C:/private.pdf"],
        vec!["--url", "https://a b"],
        vec![],
        vec!["--goto-page", "2", "--url", "https://example.org"],
    ]
    .into_iter()
    .enumerate()
    {
        let out = temp.path().join(format!("invalid-{case}.pdf"));
        assert!(!cli(&source, &out, 7)
            .args(&args)
            .output()
            .unwrap()
            .status
            .success());
        assert!(!out.exists());
        let sentinel = b"existing output must survive invalid input";
        std::fs::write(&out, sentinel).unwrap();
        assert!(!cli(&source, &out, 7)
            .args(&args)
            .arg("--overwrite")
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    }
    for index in [0, 999] {
        let out = temp.path().join(format!("invalid-index-{index}.pdf"));
        assert!(!cli(&source, &out, index)
            .args(["--goto-page", "2"])
            .output()
            .unwrap()
            .status
            .success());
        assert!(!out.exists());
    }
    let out = temp.path().join("existing.pdf");
    let sentinel = b"existing output";
    std::fs::write(&out, sentinel).unwrap();
    let result = cli(&source, &out, 7)
        .args(["--goto-page", "2"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(6));
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"], "already_exists");
    assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    assert_eq!(
        success(
            cli(&source, &out, 7)
                .args(["--goto-page", "2", "--overwrite"])
                .output()
                .unwrap()
        )["link"],
        serde_json::json!({"page":2})
    );
    assert_ne!(std::fs::read(&out).unwrap(), sentinel);
    assert_eq!(std::fs::read(&source).unwrap(), original);
}
