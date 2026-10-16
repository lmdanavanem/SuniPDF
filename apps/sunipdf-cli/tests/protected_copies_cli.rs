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
    for (name, aes256) in [
        ("plain", None),
        ("aes128", Some(false)),
        ("aes256", Some(true)),
    ] {
        let encrypted = aes256.is_some();
        let original = if let Some(aes256) = aes256 {
            security::protect(
                &input,
                None,
                &Protection {
                    user_password: "secret".into(),
                    owner_password: "owner".into(),
                    allow_copy: false,
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
        let out = temp.path().join(format!("{name}-extract.pdf"));
        let mut extract = cli("extract", &source);
        extract.args(["--pages", "2,1", "--out"]).arg(&out);
        if encrypted {
            extract.args(["--password", "secret"]);
        }
        let report = success(extract.output().unwrap());
        assert_eq!(report["page_count"], 2);
        assert_eq!(report["encryption_removed"], false);
        assert_eq!(report["encrypted"], encrypted);
        let mut split = cli("split", &source);
        split
            .args(["--every", "1", "--out-dir"])
            .arg(temp.path().join(format!("{name}-parts")));
        if encrypted {
            split.args(["--password", "secret"]);
        }
        let report = success(split.output().unwrap());
        assert_eq!(report["count"], 2);
        assert_eq!(report["encryption_removed"], false);
        assert_eq!(report["encrypted"], encrypted);
        let files: Vec<PathBuf> = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| PathBuf::from(value.as_str().unwrap()))
            .chain(std::iter::once(out))
            .collect();
        for file in files {
            let mut properties = cli("properties", &file);
            if encrypted {
                properties.args(["--password", "secret"]);
                let without_password = cli("info", &file).output().unwrap();
                assert!(!without_password.status.success());
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&without_password.stderr).unwrap()
                        ["error"],
                    "password_required"
                );
            }
            let properties = success(properties.output().unwrap());
            assert_eq!(properties["encrypted"], encrypted);
            assert_eq!(properties["permissions"]["copy"], !encrypted);
        }
        assert_eq!(std::fs::read(&source).unwrap(), original);
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
