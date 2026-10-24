use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}
fn cli(name: &str, file: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    c.arg(name).arg(file);
    c
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn failure(o: Output) {
    assert!(!o.status.success());
    assert!(o.stdout.is_empty());
    assert!(serde_json::from_slice::<Value>(&o.stderr).is_ok());
}

#[test]
fn region_and_batch_cli_require_explicit_expected_state_and_preserve_source() {
    let dir = tempfile::tempdir().unwrap();
    let measured = dir.path().join("measured.pdf");
    success(
        cli("measure", &fixture())
            .args([
                "--kind",
                "length",
                "--points",
                "40,80 140,80",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "10",
                "--out",
            ])
            .arg(&measured)
            .output()
            .unwrap(),
    );
    let original = std::fs::read(&measured).unwrap();
    let records = success(cli("measurements", &measured).output().unwrap());
    let record = records["pages"][0]["measurements"][0].clone();
    let request = dir.path().join("request.json");
    let requests = json!([{"page":1,"calibration":{"points":[[40,40],[140,40]],"known_distance":20,"unit":"m"},"update_page_default":true,"measurements":[record]}]);
    std::fs::write(&request, serde_json::to_vec(&requests).unwrap()).unwrap();
    let preview = success(
        cli("recalibrate-measurements", &measured)
            .arg("--requests")
            .arg(&request)
            .output()
            .unwrap(),
    );
    assert_eq!(preview["applied"], false);
    assert_eq!(preview["changes"][0]["before"], 10.0);
    assert_eq!(preview["changes"][0]["before_validity"], "current");
    assert!(preview["changes"][0]["historical_before"].is_null());
    assert_eq!(preview["changes"][0]["after"], 20.0);
    assert_eq!(std::fs::read(&measured).unwrap(), original);
    let output = dir.path().join("recalibrated.pdf");
    success(
        cli("recalibrate-measurements", &measured)
            .arg("--requests")
            .arg(&request)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
    );
    let reread = success(cli("measurements", &output).output().unwrap());
    assert_eq!(reread["pages"][0]["measurements"][0]["value"], 20.0);
    let stale = dir.path().join("stale.pdf");
    failure(
        cli("recalibrate-measurements", &output)
            .arg("--requests")
            .arg(&request)
            .arg("--out")
            .arg(&stale)
            .output()
            .unwrap(),
    );
    assert!(!stale.exists());
    failure(
        cli("recalibrate-measurements", &measured)
            .arg("--requests")
            .arg(&request)
            .arg("--out")
            .arg(&output)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    let empty = dir.path().join("empty.json");
    std::fs::write(&empty, "[]").unwrap();
    let regions = dir.path().join("regions.json");
    let region = json!([{"id":"detail-a","name":"Detail A","bounds":[20,20,200,200],"calibration":{"points":[[40,40],[140,40]],"known_distance":"6' 6\"","unit":"ft"}}]);
    std::fs::write(&regions, serde_json::to_vec(&region).unwrap()).unwrap();
    let with_regions = dir.path().join("regions.pdf");
    success(
        cli("set-measurement-viewports", &output)
            .arg("--expected")
            .arg(&empty)
            .arg("--regions")
            .arg(&regions)
            .arg("--out")
            .arg(&with_regions)
            .output()
            .unwrap(),
    );
    let exported = success(
        cli("measurement-viewports", &with_regions)
            .output()
            .unwrap(),
    );
    assert_eq!(exported["regions"][0]["calibration"]["known_distance"], 6.5);
    assert_eq!(exported["regions"][0]["id"], "detail-a");
    let wrong = dir.path().join("wrong.pdf");
    failure(
        cli("set-measurement-viewports", &with_regions)
            .arg("--expected")
            .arg(&empty)
            .arg("--regions")
            .arg(&empty)
            .arg("--out")
            .arg(&wrong)
            .output()
            .unwrap(),
    );
    assert!(!wrong.exists());
    let original_regions = std::fs::read(&with_regions).unwrap();
    let cleared = dir.path().join("cleared.pdf");
    let cleared_result = success(
        cli("clear-measurement-viewports", &with_regions)
            .args(["--page", "1", "--out"])
            .arg(&cleared)
            .output()
            .unwrap(),
    );
    assert_eq!(cleared_result["existing_measurements_changed"], false);
    let cleared_regions = success(cli("measurement-viewports", &cleared).output().unwrap());
    assert_eq!(cleared_regions["regions"], json!([]));
    let cleared_measurements = success(cli("measurements", &cleared).output().unwrap());
    assert_eq!(cleared_measurements["pages"], reread["pages"]);
    failure(
        cli("clear-measurement-viewports", &with_regions)
            .args(["--page", "1", "--out"])
            .arg(&cleared)
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&with_regions).unwrap(), original_regions);
    assert_eq!(std::fs::read(&measured).unwrap(), original);
}

#[test]
fn batch_recalibration_previews_legacy_values_as_historical_and_binds_the_saved_copy() {
    let dir = tempfile::tempdir().unwrap();
    let measured = dir.path().join("measured.pdf");
    success(
        cli("measure", &fixture())
            .args([
                "--kind",
                "length",
                "--points",
                "40,80 140,80",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "10",
                "--out",
            ])
            .arg(&measured)
            .output()
            .unwrap(),
    );
    let legacy = dir.path().join("legacy.pdf");
    let mut pdf = lopdf::Document::load(&measured).unwrap();
    let annotation = pdf
        .objects
        .values_mut()
        .filter_map(|object| object.as_dict_mut().ok())
        .find(|dict| dict.has(b"SuniPDFMeasurement"))
        .unwrap();
    let mut metadata: Value = serde_json::from_slice(
        annotation
            .get(b"SuniPDFMeasurement")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();
    metadata["version"] = 1.into();
    metadata.as_object_mut().unwrap().remove("provenance");
    annotation.set(
        "SuniPDFMeasurement",
        lopdf::Object::string_literal(metadata.to_string()),
    );
    pdf.save(&legacy).unwrap();
    let original = std::fs::read(&legacy).unwrap();
    let record = success(cli("measurements", &legacy).output().unwrap())["pages"][0]
        ["measurements"][0]
        .clone();
    assert_eq!(record["validity"], "legacy_unverified");
    let requests = json!([{
        "page": 1,
        "calibration": {"points": [[40,40],[140,40]], "known_distance": 20, "unit": "m"},
        "update_page_default": false,
        "measurements": [record.clone()]
    }]);
    let request = dir.path().join("request.json");
    std::fs::write(&request, serde_json::to_vec(&requests).unwrap()).unwrap();
    let preview = success(
        cli("recalibrate-measurements", &legacy)
            .arg("--requests")
            .arg(&request)
            .output()
            .unwrap(),
    );
    assert_eq!(preview["applied"], false);
    assert!(preview["changes"][0]["before"].is_null());
    assert_eq!(preview["changes"][0]["historical_before"], 10.0);
    assert_eq!(
        preview["changes"][0]["before_validity"],
        "legacy_unverified"
    );
    assert_eq!(preview["changes"][0]["after"], 20.0);
    assert_eq!(std::fs::read(&legacy).unwrap(), original);
    let out = dir.path().join("recalibrated.pdf");
    success(
        cli("recalibrate-measurements", &legacy)
            .arg("--requests")
            .arg(&request)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
    );
    let reopened = success(cli("measurements", &out).output().unwrap());
    let after = &reopened["pages"][0]["measurements"][0];
    assert_eq!(after["validity"], "current");
    assert_eq!(after["value"], 20.0);
    assert!(after["historical_value"].is_null());
    assert_eq!(after["annotation_id"], record["annotation_id"]);
    assert_eq!(std::fs::read(&legacy).unwrap(), original);
}

#[test]
fn region_cli_rejects_missing_identity_zero_scale_and_oversized_requests() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.json");
    std::fs::write(&empty, "[]").unwrap();
    let bad = dir.path().join("bad.json");
    let out = dir.path().join("bad.pdf");
    for value in [
        json!([{"name":"missing identity"}]),
        json!([{"id":"a","name":"A","bounds":[20,20,200,200],"calibration":{"points":[[40,40],[140,40]],"known_distance":0,"unit":"m"}}]),
    ] {
        std::fs::write(&bad, serde_json::to_vec(&value).unwrap()).unwrap();
        failure(
            cli("set-measurement-viewports", &fixture())
                .arg("--expected")
                .arg(&empty)
                .arg("--regions")
                .arg(&bad)
                .arg("--out")
                .arg(&out)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    std::fs::write(&bad, vec![b' '; 128 * 1024 + 1]).unwrap();
    failure(
        cli("set-measurement-viewports", &fixture())
            .arg("--expected")
            .arg(&empty)
            .arg("--regions")
            .arg(&bad)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
}
