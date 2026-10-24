use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn response(output: Output, expected: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if expected == 0 || expected == 2 {
        assert!(output.stderr.is_empty());
        serde_json::from_slice(&output.stdout).unwrap()
    } else {
        assert!(output.stdout.is_empty());
        serde_json::from_slice(&output.stderr).unwrap()
    }
}

fn fixture_pdf() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut ink = String::new();
    for x in [20, 120, 220] {
        ink.push_str(&format!(
            "q 1 0 0 1 {x} 20 cm 0 g 0 0 5 25 re f 0 0 25 5 re f 12 12 7 7 re f Q\n"
        ));
    }
    // A solid square is a distractor, not a fourth matching symbol.
    ink.push_str("0 g 120 110 25 25 re f\n");
    let content = pdf.add_object(Stream::new(dictionary! {}, ink.into_bytes()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),300.into(),180.into()],
    "Resources"=>dictionary!{},"Contents"=>content});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Count"=>1,"Kids"=>vec![Object::Reference(page)]}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

struct Fixture {
    directory: tempfile::TempDir,
    source: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("symbols.pdf");
        std::fs::write(&source, fixture_pdf()).unwrap();
        Self { directory, source }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn inspect(&self) -> Command {
        let mut command = cli();
        command
            .args(["visual-search", "inspect"])
            .arg(&self.source)
            .args([
                "--template-page",
                "1",
                "--template-rect",
                "16,16,49,49",
                "--pages",
                "1",
                "--similarity",
                "95",
            ]);
        command
    }

    fn apply(&self, review: &Path, out: &Path) -> Command {
        let mut command = cli();
        command
            .args(["visual-search", "apply"])
            .arg(&self.source)
            .arg("--review")
            .arg(review)
            .arg("--out")
            .arg(out);
        command
    }
}

#[test]
fn inspect_is_read_only_and_only_explicit_reviewed_matches_become_counts() {
    let fixture = Fixture::new();
    let original = std::fs::read(&fixture.source).unwrap();
    let review_path = fixture.path("review.json");
    let inspected = response(
        fixture
            .inspect()
            .arg("--review-out")
            .arg(&review_path)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(inspected["complete"], true);
    assert_eq!(inspected["accepted_count"], 0);
    assert_eq!(inspected["published"], false);
    assert_eq!(inspected["similarity_is_probability"], false);
    let candidates = inspected["inspection"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 3, "{inspected}");
    let mut review: Value = serde_json::from_slice(&std::fs::read(&review_path).unwrap()).unwrap();
    let out = fixture.path("counted.pdf");
    response(fixture.apply(&review_path, &out).output().unwrap(), 1);
    assert!(!out.exists());
    review["selected_ids"] = json!([candidates[0]["id"], candidates[2]["id"]]);
    review["group"] = json!("Reviewed equipment");
    std::fs::write(&review_path, serde_json::to_vec(&review).unwrap()).unwrap();
    let applied = response(fixture.apply(&review_path, &out).output().unwrap(), 0);
    assert_eq!(applied["accepted_count"], 2);
    assert_eq!(applied["source_unchanged"], true);
    assert_eq!(std::fs::read(&fixture.source).unwrap(), original);
    let register = response(
        cli().arg("annotation-register").arg(&out).output().unwrap(),
        0,
    );
    let serialized = serde_json::to_string(&register).unwrap();
    assert!(serialized.contains("Reviewed equipment"));
    let saved = std::fs::read(&out).unwrap();
    response(
        fixture
            .apply(&review_path, &out)
            .arg("--overwrite")
            .output()
            .unwrap(),
        6,
    );
    assert_eq!(std::fs::read(&out).unwrap(), saved);
}

#[test]
fn changed_sources_forged_results_and_duplicate_selections_preserve_outputs() {
    let fixture = Fixture::new();
    let inspected = response(fixture.inspect().output().unwrap(), 0);
    let original_review = inspected["review"].clone();
    let id = inspected["inspection"]["candidates"][0]["id"].clone();
    let review_path = fixture.path("review.json");
    let out = fixture.path("refused.pdf");
    for (field, value) in [
        ("selected_ids", json!([id.clone(), id.clone()])),
        ("selected_ids", json!([u32::MAX])),
        ("raw_file_sha256", json!("0".repeat(64))),
        ("inspection_sha256", json!("0".repeat(64))),
        ("version", json!(2)),
        ("unrecognized", json!(true)),
    ] {
        let mut review = original_review.clone();
        review["selected_ids"] = json!([id]);
        review[field] = value;
        std::fs::write(&review_path, serde_json::to_vec(&review).unwrap()).unwrap();
        response(fixture.apply(&review_path, &out).output().unwrap(), 1);
        assert!(!out.exists(), "{field}");
    }
    let mut review = original_review;
    review["selected_ids"] = json!([id]);
    std::fs::write(&review_path, serde_json::to_vec(&review).unwrap()).unwrap();
    let mut changed = std::fs::read(&fixture.source).unwrap();
    changed.extend_from_slice(b"\n% changed source\n");
    std::fs::write(&fixture.source, &changed).unwrap();
    response(fixture.apply(&review_path, &out).output().unwrap(), 1);
    assert!(!out.exists());
    assert_eq!(std::fs::read(&fixture.source).unwrap(), changed);
}

#[test]
fn resource_limits_and_invalid_inputs_are_explicit() {
    let fixture = Fixture::new();
    let partial = response(
        fixture
            .inspect()
            .args(["--max-comparisons", "1"])
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(partial["complete"], false);
    assert_ne!(partial["inspection"]["progress"]["status"], "complete");
    assert_eq!(partial["accepted_count"], 0);
    let blank = response(
        cli()
            .args(["visual-search", "inspect"])
            .arg(&fixture.source)
            .args(["--template-rect", "10,120,40,150", "--pages", "1"])
            .output()
            .unwrap(),
        1,
    );
    assert!(blank["message"].is_string());
    let collision = fixture.path("existing.json");
    std::fs::write(&collision, b"keep").unwrap();
    response(
        fixture
            .inspect()
            .arg("--review-out")
            .arg(&collision)
            .arg("--overwrite")
            .output()
            .unwrap(),
        6,
    );
    assert_eq!(std::fs::read(collision).unwrap(), b"keep");
}
