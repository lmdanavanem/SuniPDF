use std::{
    path::Path,
    process::{Command, Output},
};
use sunipdf_core::{security, Engine, Protection, TextStyle};

fn call(args: &[String], credential: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .args(args)
        .env("SUNIPDF_ORDER_TEST_PASSWORD", credential)
        .output()
        .unwrap()
}
fn success(result: Output) -> serde_json::Value {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn reorder_cli_preserves_selection_sources_encryption_credentials_and_output_guards() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source with spaces.pdf");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let mut doc = engine.open(&fixture, None).unwrap();
    for (i, label) in ["DragA", "DragB", "DragC"].iter().enumerate() {
        doc.add_text(
            0,
            20.0,
            300.0 + i as f32 * 20.0,
            label,
            &TextStyle::default(),
        )
        .unwrap();
    }
    let count = doc.page_objects(0).unwrap().len();
    let first = count - 3;
    doc.save_as(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let nonce = temp.path().file_name().unwrap().to_string_lossy();
    let user = format!("opening-{nonce}");
    let owner = format!("owner-{nonce}");
    for encryption in [None, Some(false), Some(true)] {
        let input = temp.path().join(format!("in-{encryption:?}.pdf"));
        let bytes = encryption.map_or_else(
            || original.clone(),
            |aes256| {
                security::protect(
                    &original,
                    None,
                    &Protection {
                        user_password: user.clone(),
                        owner_password: owner.clone(),
                        aes256,
                        ..Default::default()
                    },
                )
                .unwrap()
            },
        );
        std::fs::write(&input, &bytes).unwrap();
        let out = temp.path().join(format!("out-{encryption:?}.pdf"));
        let args = vec![
            "--password-env".into(),
            "SUNIPDF_ORDER_TEST_PASSWORD".into(),
            "reorder-objects".into(),
            input.to_str().unwrap().into(),
            "--objects".into(),
            format!("{},{},{}", first + 2, first, first + 2),
            "--at".into(),
            "0".into(),
            "--out".into(),
            out.to_str().unwrap().into(),
        ];
        let report = success(call(&args, &user));
        assert_eq!(report["new_indices"], serde_json::json!([0, 1]));
        let saved = engine
            .open(&out, encryption.map(|_| user.as_str()))
            .unwrap();
        let objects = saved.page_objects(0).unwrap();
        assert_eq!(objects.len(), count);
        assert!(objects[0].text.as_ref().unwrap().contains("DragA"));
        assert!(objects[1].text.as_ref().unwrap().contains("DragC"));
        assert_eq!(saved.properties().encrypted, encryption.is_some());
        let written = std::fs::read(&out).unwrap();
        assert!(!call(&args, &user).status.success());
        assert_eq!(std::fs::read(&out).unwrap(), written);
        let mut invalid = args.clone();
        let bad = temp.path().join("invalid.pdf");
        *invalid.last_mut().unwrap() = bad.to_str().unwrap().into();
        invalid[7] = count.to_string();
        assert!(!call(&invalid, &user).status.success());
        assert!(!bad.exists());
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        if let Some(aes256) = encryption {
            let protected = temp.path().join("no-modifications.pdf");
            let bytes = security::protect(
                &original,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    allow_modify: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
            std::fs::write(&protected, &bytes).unwrap();
            let mut denied = invalid.clone();
            denied[3] = protected.to_str().unwrap().into();
            denied[7] = "0".into();
            assert!(!call(&denied, &user).status.success());
            assert!(!bad.exists());
            assert_eq!(std::fs::read(&protected).unwrap(), bytes);
        }
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let help = call(&["reorder-objects".into(), "--help".into()], &user);
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout).unwrap().contains("--at"));
}
