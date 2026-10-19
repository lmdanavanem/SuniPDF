//! Licensed PDF Association examples; this is interoperability QA, not conformance certification.
#![cfg(windows)]

#[path = "support/bounded_process.rs"]
mod process;

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn cli(operation: &str, file: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(operation).arg(file);
    command
}

fn success(command: Command) -> Value {
    let output = process::bounded(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn page_text(file: &Path) -> Vec<u8> {
    let mut command = cli("text", file);
    command.args(["--page", "1"]);
    let output = process::bounded(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn render(file: &Path, output: &Path) -> image::RgbaImage {
    let mut command = cli("render", file);
    command
        .args(["--page", "1", "--scale", "0.4", "--out"])
        .arg(output);
    success(command);
    let image = image::open(output).unwrap().into_rgba8();
    assert!(image.width() > 100 && image.height() > 100);
    image
}

fn qpdf(qpdf: &Path, file: &Path) {
    let mut command = Command::new(qpdf);
    command.arg("--check").arg(file);
    let output = process::bounded(command);
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires the pinned interoperability corpus and qpdf; explicitly required in Windows CI"]
fn pdf20_examples_preserve_text_pixels_annotations_and_source_bytes() {
    let started = Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: Value =
        serde_json::from_str(include_str!("../../../scripts/interop-corpus.json")).unwrap();
    let corpus = PathBuf::from(
        std::env::var_os("SUNIPDF_INTEROP_CORPUS")
            .expect("run fetch-regression-corpus.ps1 -Interop"),
    );
    let checker = PathBuf::from(std::env::var_os("SUNIPDF_QPDF").expect("run fetch-qpdf.ps1"));
    let mut version = Command::new(&checker);
    version.arg("--version");
    let version = process::bounded(version);
    assert!(
        version.status.success()
            && String::from_utf8_lossy(&version.stdout).starts_with("qpdf version 12.4.2")
    );
    let checker_sha256 = process::sha256(&checker);
    let temp = tempfile::tempdir().unwrap();
    let mut results = Vec::new();
    for entry in manifest["documents"].as_array().unwrap() {
        let case = temp.path().join(entry["id"].as_str().unwrap());
        std::fs::create_dir(&case).unwrap();
        let source = std::path::absolute(corpus.join(entry["filename"].as_str().unwrap())).unwrap();
        let original = std::fs::read(&source).unwrap();
        assert_eq!(original.len() as u64, entry["bytes"].as_u64().unwrap());
        assert_eq!(process::sha256(&source), entry["sha256"].as_str().unwrap());
        let info = success(cli("info", &source));
        assert_eq!(info["page_count"], entry["page_count"]);
        qpdf(&checker, &source);
        let text = page_text(&source);
        let marker = entry["text_marker"].as_str().unwrap();
        if marker.is_empty() {
            assert!(String::from_utf8_lossy(&text).trim().is_empty());
        } else {
            assert!(String::from_utf8_lossy(&text).contains(marker));
        }
        let annotations = success(cli("annotations", &source));
        let pixels = render(&source, &case.join("original.png"));
        let optimized = case.join("optimized.pdf");
        let mut command = cli("optimize", &source);
        command.arg("--out").arg(&optimized);
        let optimized_report = success(command);
        qpdf(&checker, &optimized);
        assert_eq!(success(cli("info", &optimized))["pages"], info["pages"]);
        assert_eq!(page_text(&optimized), text);
        assert_eq!(success(cli("annotations", &optimized)), annotations);
        assert_eq!(render(&optimized, &case.join("optimized.png")), pixels);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let copy = case.join("extracted.pdf");
        let mut command = cli("extract", &source);
        command.args(["--pages", "1", "--out"]).arg(&copy);
        success(command);
        qpdf(&checker, &copy);
        assert_eq!(page_text(&copy), text);
        assert_eq!(render(&copy, &case.join("extracted.png")), pixels);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        results.push(json!({"id":entry["id"],"page_count":entry["page_count"],"source_sha256":entry["sha256"],"source_unchanged":true,"text_and_pixels_preserved":true,"optimization_annotations_preserved":true,"optimize":optimized_report}));
        std::fs::remove_file(optimized).unwrap();
        std::fs::remove_file(copy).unwrap();
    }
    let report = json!({"documents":results,"elapsed_ms":started.elapsed().as_millis(),"commands":*process::COMMAND_METRICS.lock().unwrap(),"hardware":process::hardware(),"memory_guard_bytes":process::MAX_MEMORY,"timeout_seconds":180,"output_limit_bytes":process::MAX_OUTPUT,"license":manifest["license_policy_url"],"upstream_revision":manifest["upstream_revision"],"qpdf_version":"12.4.2","qpdf_sha256":checker_sha256,"corpus_distributed":false,"conformance_certification":false});
    std::fs::write(
        root.join("target/interop-corpus-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
