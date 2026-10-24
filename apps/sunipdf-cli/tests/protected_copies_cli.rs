use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(command: &str, source: &Path) -> Command {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    cli.arg(command).arg(source);
    cli
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
fn extract_and_split_report_and_retain_original_protection() {
    use sunipdf_core::{security, Protection};
    let temp = tempfile::tempdir().unwrap();
    let input = std::fs::read(fixture()).unwrap();
    for (name, aes256, user, owner) in [
        ("plain", None, "", ""),
        ("aes128", Some(false), "package-user", "package-owner"),
        ("aes128-latin1", Some(false), "été", "propriétaire"),
        ("aes256", Some(true), "package-user", "package-owner"),
    ] {
        let encrypted = aes256.is_some();
        let original = if let Some(aes256) = aes256 {
            security::protect(
                &input,
                None,
                &Protection {
                    user_password: user.into(),
                    owner_password: owner.into(),
                    allow_copy: false,
                    allow_print: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap()
        } else {
            input.clone()
        };
        let source = temp.path().join(format!("{name}.pdf"));
        std::fs::write(&source, &original).unwrap();
        let opening_passwords = if encrypted {
            vec![user, owner]
        } else {
            vec![""]
        };
        for (role, opening_password) in opening_passwords.into_iter().enumerate() {
            let out = temp.path().join(format!("{name}-{role}-extract.pdf"));
            let mut extract = cli("extract", &source);
            extract.args(["--pages", "2,1", "--out"]).arg(&out);
            if encrypted {
                extract.args(["--password", opening_password]);
            }
            let report = success(extract.output().unwrap());
            assert_eq!(report["page_count"], 2);
            assert_eq!(report["encryption_removed"], false);
            assert_eq!(report["encrypted"], encrypted);
            let mut split = cli("split", &source);
            split
                .args(["--every", "1", "--out-dir"])
                .arg(temp.path().join(format!("{name}-{role}-parts")));
            if encrypted {
                split.args(["--password", opening_password]);
            }
            let report = success(split.output().unwrap());
            assert_eq!(report["count"], 2);
            assert_eq!(report["encryption_removed"], false);
            assert_eq!(report["encrypted"], encrypted);
            let metadata = temp.path().join(format!("{name}-{role}-metadata.pdf"));
            let mut rewrite = cli("set-metadata", &source);
            rewrite
                .args(["--title", "Owner stream regression", "--out"])
                .arg(&metadata);
            if encrypted {
                rewrite.args(["--password", opening_password]);
            }
            success(rewrite.output().unwrap());
            let files: Vec<PathBuf> = report["files"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| PathBuf::from(value.as_str().unwrap()))
                .chain(std::iter::once(out))
                .chain(std::iter::once(metadata))
                .collect();
            for file in files {
                let mut properties = cli("properties", &file);
                if encrypted {
                    properties.args(["--password", user]);
                    let without_password = cli("info", &file).output().unwrap();
                    assert!(!without_password.status.success());
                    assert_eq!(
                        serde_json::from_slice::<serde_json::Value>(&without_password.stderr)
                            .unwrap()["error"],
                        "password_required"
                    );
                }
                let properties = success(properties.output().unwrap());
                assert_eq!(properties["encrypted"], encrypted);
                assert_eq!(properties["permissions"]["copy"], !encrypted);
                assert_eq!(properties["permissions"]["print"], !encrypted);
                let mut text = cli("text", &file);
                if encrypted {
                    text.args(["--password", user]);
                }
                let text = text.output().unwrap();
                assert!(text.status.success());
                assert!(
                String::from_utf8_lossy(&text.stdout).contains("SuniPDF page"),
                "{name} source opening role {role}: derived content stream must remain readable"
            );
            }
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}

#[test]
fn extract_cannot_overwrite_its_input_even_with_explicit_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.pdf");
    let original = std::fs::read(fixture()).unwrap();
    std::fs::write(&source, &original).unwrap();
    let output = cli("extract", &source)
        .args(["--overwrite", "--pages", "1", "--out"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"],
        "already_exists"
    );
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn snapshot_and_staged_outputs_preserve_protection_and_explicit_overwrite() {
    use sunipdf_core::{security, Engine, EngineError, Protection, Rotation};
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let input = std::fs::read(fixture()).unwrap();
    for aes256 in [false, true] {
        let protected = security::protect(
            &input,
            None,
            &Protection {
                user_password: "reader".into(),
                owner_password: "owner".into(),
                allow_copy: false,
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let source = temp.path().join(format!("source-{aes256}.pdf"));
        std::fs::write(&source, &protected).unwrap();
        for operation in ["rotate", "extract"] {
            let out = temp.path().join(format!("{operation}-{aes256}.pdf"));
            std::fs::write(&out, b"existing output").unwrap();
            let command = |overwrite: bool| {
                let mut command = cli(operation, &source);
                command.args(["--password", "reader", "--pages", "1", "--out"]);
                command.arg(&out);
                if operation == "rotate" {
                    command.args(["--degrees", "90"]);
                }
                if overwrite {
                    command.arg("--overwrite");
                }
                command
            };
            let refused = command(false).output().unwrap();
            assert_eq!(refused.status.code(), Some(6));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&refused.stderr).unwrap()["error"],
                "already_exists"
            );
            assert_eq!(std::fs::read(&out).unwrap(), b"existing output");
            let report = success(command(true).output().unwrap());
            assert_eq!(report["out"], out.to_string_lossy().as_ref());
            assert!(matches!(
                engine.open(&out, None),
                Err(EngineError::PasswordRequired)
            ));
            for password in ["reader", "owner"] {
                let document = engine.open(&out, Some(password)).unwrap();
                assert!(document.properties().encrypted);
                assert!(!document.properties().can_copy);
                assert!(document.page_text(0).unwrap().contains("SuniPDF page 1"));
                assert_eq!(
                    document.page_count(),
                    if operation == "rotate" { 2 } else { 1 }
                );
                if operation == "rotate" {
                    assert_eq!(document.page_rotation(0).unwrap(), Rotation::R90);
                }
            }
            assert_eq!(std::fs::read(&source).unwrap(), protected);
        }
    }
}
