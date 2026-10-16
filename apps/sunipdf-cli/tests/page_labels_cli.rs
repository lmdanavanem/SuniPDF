use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

#[test]
fn info_exposes_page_labels_without_changing_physical_indices_or_source() {
    for name in ["page-labels.pdf", "two-pages.pdf"] {
        let source = fixture(name);
        let original = std::fs::read(&source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
            .arg("info")
            .arg(&source)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let info: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let pages = info["pages"].as_array().unwrap();
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page["index"], i + 1);
        }
        if name == "page-labels.pdf" {
            assert_eq!(pages.len(), 9);
            for (i, label) in [
                Some("i"),
                Some("ii"),
                Some("1"),
                Some("2"),
                Some("AA"),
                Some("Annexe-É7"),
                Some("Repeat"),
                Some("Repeat"),
                None,
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(pages[i]["label"].as_str(), label);
            }
        } else {
            assert!(pages.iter().all(|page| page["label"].is_null()));
        }
        assert_eq!(std::fs::read(source).unwrap(), original);
    }
}
