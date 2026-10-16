#[path = "../../../crates/sunipdf-core/tests/fixtures/destinations.rs"]
mod fixture;

use std::process::Command;

fn read(command: &str, path: &std::path::Path) -> serde_json::Value {
    let result = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .arg(command)
        .arg(path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn bookmarks_and_annotation_links_report_one_based_precise_destinations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("destinations.pdf");
    std::fs::write(&path, fixture::document()).unwrap();
    let bookmarks = read("bookmarks", &path);
    let annotations = read("annotations", &path);
    let links = annotations["pages"][0]["annotations"].as_array().unwrap();
    assert_eq!(
        bookmarks.as_array().unwrap().len(),
        4 * fixture::PER_PAGE + 7
    );
    for page in 0..4 {
        let index = page * fixture::PER_PAGE;
        assert_eq!(bookmarks[index]["page"], page + 1);
        assert_eq!(bookmarks[index]["depth"], 0);
        assert_eq!(
            bookmarks[index]["title"],
            format!("Page {page} destination 0")
        );
        let destination = &bookmarks[index]["destination"];
        assert_eq!(destination["page"], page + 1);
        assert_eq!(destination["view"], "XYZ");
        assert_eq!(destination["zoom"], 1.5);
        assert_eq!(links[index]["link"]["page"], page + 1);
        assert_eq!(&links[index]["link"]["destination"], destination);
        assert_eq!(bookmarks[index + 5]["destination"]["view"], "Fit");
        assert_eq!(bookmarks[index + 6]["destination"]["view"], "FitH");
        assert!(bookmarks[index + 7]["destination"].is_null());
        assert!(bookmarks[index + 8]["destination"].is_null());
        assert!(bookmarks[index + 9]["destination"].is_null());
    }
    assert!(bookmarks[4 * fixture::PER_PAGE]["page"].is_null());
    assert!(links[4 * fixture::PER_PAGE]["link"]["unsupported"]
        .as_bool()
        .unwrap());
    assert_eq!(
        links[4 * fixture::PER_PAGE + 5]["link"]["uri"],
        "https://example.org"
    );
}
