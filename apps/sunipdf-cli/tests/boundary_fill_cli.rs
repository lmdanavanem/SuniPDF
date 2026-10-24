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

fn fixture_pdf(open: bool) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    // Filled, integer-aligned walls make the interior geometry independently known:
    // 152 * 112 square points, minus a 30 * 30 island, equals 16124 square points.
    let bottom = if open {
        "20 20 70 4 re f 110 20 70 4 re f"
    } else {
        "20 20 160 4 re f"
    };
    let content = format!(
        "0 g {bottom} 20 136 160 4 re f 20 20 4 120 re f \
        176 20 4 120 re f 70 60 30 30 re f"
    );
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),240.into(),200.into()],
    "Resources"=>dictionary!{}, "Contents"=>stream});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>1,
        "Kids"=>vec![Object::Reference(page)]}
        .into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
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
    fn new(open: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("room.pdf");
        std::fs::write(&source, fixture_pdf(open)).unwrap();
        Self { directory, source }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn inspect(&self) -> Command {
        let mut command = cli();
        command
            .args(["region-fill", "inspect"])
            .arg(&self.source)
            .args(["--page", "1", "--seed", "40,40", "--scale", "1"]);
        command
    }

    fn action(&self, action: &str, review: &Path, out: &Path) -> Command {
        let mut command = cli();
        command
            .args(["region-fill", action])
            .arg(&self.source)
            .arg("--review")
            .arg(review)
            .arg("--out")
            .arg(out);
        command
    }
}

fn calibrated(mut review: Value) -> Value {
    review["reference"] = json!({"kind":"local", "points":[[0,0],[100,0]],
        "known_distance":1, "unit":"m"});
    review["unit"] = json!("m");
    review["precision"] = json!(4);
    review
}

fn write_review(path: &Path, review: &Value) {
    std::fs::write(path, serde_json::to_vec(review).unwrap()).unwrap();
}

#[test]
fn closed_region_retains_hole_and_only_reviewed_calibrated_measurement_is_saved() {
    let fixture = Fixture::new(false);
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
    assert_eq!(inspected["measurement_created"], false);
    assert_eq!(inspected["published"], false);
    assert_eq!(inspected["review"]["accepted"], false);
    let region = &inspected["inspection"]["region"];
    assert_eq!(region["holes"].as_array().unwrap().len(), 1);
    assert!((region["area_points2"].as_f64().unwrap() - 16124.0).abs() < 0.001);
    let mut review = calibrated(inspected["review"].clone());
    write_review(&review_path, &review);
    let output = fixture.path("measured.pdf");
    response(
        fixture
            .action("apply", &review_path, &output)
            .output()
            .unwrap(),
        1,
    );
    assert!(!output.exists());

    let preview_path = fixture.path("preview.png");
    let preview = response(
        fixture
            .action("preview", &review_path, &preview_path)
            .args(["--long-side", "600"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(preview["measurement_created"], false);
    assert_eq!(preview["published"], false);
    assert_eq!(preview["measurement"]["unit"], "m2");
    assert!((preview["measurement"]["value"].as_f64().unwrap() - 1.6124).abs() < 0.000001);
    let image = image::open(&preview_path).unwrap();
    assert_eq!(image.width().max(image.height()), 600);
    assert_eq!(std::fs::read(&fixture.source).unwrap(), original);

    review["accepted"] = json!(true);
    write_review(&review_path, &review);
    let applied = response(
        fixture
            .action("apply", &review_path, &output)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(applied["measurement_created"], true);
    assert_eq!(applied["source_unchanged"], true);
    assert_eq!(applied["measurement"]["holes"].as_array().unwrap().len(), 1);
    let saved = response(cli().arg("measurements").arg(&output).output().unwrap(), 0);
    let records = saved["pages"][0]["measurements"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["validity"], "current");
    assert_eq!(records[0]["unit"], "m2");
    assert!((records[0]["value"].as_f64().unwrap() - 1.6124).abs() < 0.000001);

    let pdf = Document::load(&output).unwrap();
    let page = pdf.get_pages()[&1];
    let annotation = pdf
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .as_reference()
        .unwrap();
    let raw = pdf
        .get_dictionary(annotation)
        .unwrap()
        .get(b"SuniPDFBoundaryFill")
        .unwrap()
        .as_str()
        .unwrap();
    let provenance: Value = serde_json::from_slice(raw).unwrap();
    assert_eq!(provenance["version"], 1);
    assert_eq!(
        provenance["inspection"]["region"]["holes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let before = std::fs::read(&output).unwrap();
    response(
        fixture
            .action("apply", &review_path, &output)
            .arg("--overwrite")
            .output()
            .unwrap(),
        6,
    );
    assert_eq!(std::fs::read(&output).unwrap(), before);
    assert_eq!(std::fs::read(&fixture.source).unwrap(), original);
}

#[test]
fn leaks_and_limited_regions_are_never_applied_and_manual_closures_are_explicit() {
    let open = Fixture::new(true);
    let inspected = response(open.inspect().output().unwrap(), 2);
    assert_eq!(inspected["inspection"]["progress"]["status"], "leaked");
    assert!(inspected["inspection"]["region"].is_null());
    let review_path = open.path("leaked.json");
    let mut review = calibrated(inspected["review"].clone());
    review["accepted"] = json!(true);
    write_review(&review_path, &review);
    let output = open.path("unsafe.pdf");
    response(
        open.action("apply", &review_path, &output)
            .output()
            .unwrap(),
        1,
    );
    assert!(!output.exists());
    let closed = response(
        open.inspect()
            .args(["--closure", "88,22,112,22"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(closed["complete"], true);
    assert_eq!(closed["inspection"]["closure_width_pixels"], 1);
    assert_eq!(
        closed["inspection"]["options"]["closures"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        closed["inspection"]["region"]["holes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let fixture = Fixture::new(false);
    let limited = response(
        fixture
            .inspect()
            .args(["--max-work", "1"])
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(limited["inspection"]["progress"]["status"], "work_limit");
    let review_path = fixture.path("partial.json");
    let mut review = calibrated(limited["review"].clone());
    review["accepted"] = json!(true);
    write_review(&review_path, &review);
    let output = fixture.path("partial.pdf");
    response(
        fixture
            .action("apply", &review_path, &output)
            .output()
            .unwrap(),
        1,
    );
    assert!(!output.exists());
}

#[test]
fn changed_sources_unknown_review_fields_and_output_collisions_are_refused() {
    let fixture = Fixture::new(false);
    let inspected = response(fixture.inspect().output().unwrap(), 0);
    let mut original = calibrated(inspected["review"].clone());
    original["accepted"] = json!(true);
    let review_path = fixture.path("review.json");
    let output = fixture.path("refused.pdf");
    for (field, value) in [
        ("version", json!(2)),
        ("inspection_sha256", json!("0".repeat(64))),
        ("raw_file_sha256", json!("0".repeat(64))),
        ("unknown", json!(true)),
        ("seed", json!([42, 42])),
        ("reference", json!({"kind":"automatic", "extra":true})),
        ("unit", json!("unsupported")),
        ("precision", json!(5)),
    ] {
        let mut review = original.clone();
        review[field] = value;
        write_review(&review_path, &review);
        response(
            fixture
                .action("apply", &review_path, &output)
                .output()
                .unwrap(),
            1,
        );
        assert!(!output.exists(), "{field}");
    }
    let mut review = original.clone();
    review["options"]["unknown"] = json!(true);
    write_review(&review_path, &review);
    response(
        fixture
            .action("apply", &review_path, &output)
            .output()
            .unwrap(),
        1,
    );
    assert!(!output.exists());
    write_review(&review_path, &original);
    let mut changed = std::fs::read(&fixture.source).unwrap();
    changed.extend_from_slice(b"\n% source revision\n");
    std::fs::write(&fixture.source, &changed).unwrap();
    response(
        fixture
            .action("apply", &review_path, &output)
            .output()
            .unwrap(),
        1,
    );
    assert!(!output.exists());
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
    assert_eq!(std::fs::read(&fixture.source).unwrap(), changed);
}
