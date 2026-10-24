use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OUTER: &str = "40,80 140,80 140,180 40,180";
const HOLE: &str = "60,100 80,100 80,150 60,150";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(action: &str, source: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(action).arg(source);
    command
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rejected(output: Output) {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stdout.is_empty());
}

fn near(value: &Value, expected: f64) {
    assert!(
        (value.as_f64().unwrap() - expected).abs() < 1e-5,
        "{value} != {expected}"
    );
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn measure(source: &Path, out: &Path, kind: &str, points: &str) -> Command {
    let mut command = cli("measure", source);
    command
        .args([
            "--kind",
            kind,
            "--points",
            points,
            "--calibration",
            "40,40 140,40",
            "--known-distance",
            "10",
            "--out",
        ])
        .arg(out);
    command
}

fn records(source: &Path) -> Value {
    success(cli("measurements", source).output().unwrap())["pages"][0]["measurements"].clone()
}

fn volume(directory: &Path) -> PathBuf {
    let path = directory.join("volume.pdf");
    success(
        measure(&fixture(), &path, "volume", OUTER)
            .args(["--hole", HOLE, "--depth", "20", "--depth-unit", "cm"])
            .output()
            .unwrap(),
    );
    path
}

fn edit(source: &Path, expected: &Path, index: &Value, out: &Path) -> Command {
    let mut command = cli("edit-measurement", source);
    command
        .args(["--page", "1", "--index"])
        .arg(index.as_u64().unwrap().to_string())
        .arg("--expected-record")
        .arg(expected)
        .arg("--out")
        .arg(out);
    command
}

#[test]
fn deducted_area_and_equivalent_physical_depths_persist_and_export() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture()).unwrap();
    for (name, kind, depth) in [
        ("area", "area", None),
        ("metres", "volume", Some(("0.2", "m"))),
        ("centimetres", "volume", Some(("20", "cm"))),
    ] {
        let out = directory.path().join(format!("{name}.pdf"));
        let mut command = measure(&fixture(), &out, kind, OUTER);
        command.args(["--hole", HOLE]);
        if let Some((value, unit)) = depth {
            command.args(["--depth", value, "--depth-unit", unit]);
        }
        let created = success(command.output().unwrap());
        assert_eq!(created["original_unchanged"], true);
        let all = records(&out);
        assert_eq!(all.as_array().unwrap().len(), 1);
        let record = &all[0];
        assert_eq!(record["version"], 3);
        assert_eq!(record["validity"], "current");
        assert_eq!(
            record["holes"],
            json!([[[60.0, 100.0], [80.0, 100.0], [80.0, 150.0], [60.0, 150.0]]])
        );
        near(&record["area"]["gross"], 100.0);
        near(&record["area"]["deductions"][0], 10.0);
        near(&record["area"]["net"], 90.0);
        assert_eq!(record["area_unit"], "m2");
        near(&record["value"], if depth.is_some() { 18.0 } else { 90.0 });
        assert_eq!(record["unit"], if depth.is_some() { "m3" } else { "m2" });
        assert_eq!(created["measurement"]["depth"], record["depth"]);
        assert_eq!(created["measurement"]["area"], record["area"]);
    }
    assert_eq!(std::fs::read(fixture()).unwrap(), original);
}

#[test]
fn edits_preserve_inputs_convert_result_units_and_require_explicit_clears() {
    let directory = tempfile::tempdir().unwrap();
    let source = volume(directory.path());
    let original = std::fs::read(&source).unwrap();
    let record = records(&source)[0].clone();
    let expected = directory.path().join("record.json");
    write_json(&expected, &record);
    let converted = directory.path().join("feet.pdf");
    success(
        edit(&source, &expected, &record["index"], &converted)
            .args(["--unit", "ft"])
            .output()
            .unwrap(),
    );
    let after = records(&converted)[0].clone();
    assert_eq!(after["depth"], json!({"value":20.0,"unit":"cm"}));
    assert_eq!(after["holes"], record["holes"]);
    assert_eq!(after["annotation_id"], record["annotation_id"]);
    near(&after["value"], 18.0 / 0.3048_f64.powi(3));
    near(&after["area"]["net"], 90.0 / 0.3048_f64.powi(2));
    let changed = directory.path().join("hole-only.pdf");
    success(
        edit(&source, &expected, &record["index"], &changed)
            .args(["--hole", "60,100 110,100 110,150 60,150"])
            .output()
            .unwrap(),
    );
    let changed_record = records(&changed)[0].clone();
    near(&changed_record["value"], 15.0);
    assert_eq!(changed_record["depth"], record["depth"]);
    assert_eq!(changed_record["points"], record["points"]);
    for (name, flags, value, kind) in [
        ("cleared-holes", vec!["--clear-holes"], 20.0, "volume"),
        (
            "surface",
            vec!["--kind", "area", "--clear-depth"],
            90.0,
            "area",
        ),
    ] {
        let out = directory.path().join(format!("{name}.pdf"));
        success(
            edit(&source, &expected, &record["index"], &out)
                .args(flags)
                .output()
                .unwrap(),
        );
        let row = records(&out)[0].clone();
        near(&row["value"], value);
        assert_eq!(row["kind"], kind);
        if kind == "area" {
            assert!(row["depth"].is_null());
        } else {
            assert_eq!(row["holes"], json!([]));
        }
    }
    for (number, flags) in [
        vec!["--clear-depth"],
        vec!["--kind", "area"],
        vec!["--hole", HOLE, "--clear-holes"],
        vec!["--depth", "1"],
        vec!["--depth-unit", "m"],
        vec!["--clear-depth", "--depth", "1", "--depth-unit", "m"],
    ]
    .into_iter()
    .enumerate()
    {
        let out = directory.path().join(format!("ambiguous-{number}.pdf"));
        rejected(
            edit(&source, &expected, &record["index"], &out)
                .args(flags)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    let stale_target = directory.path().join("stale-target.pdf");
    rejected(
        edit(&changed, &expected, &record["index"], &stale_target)
            .args(["--precision", "1"])
            .output()
            .unwrap(),
    );
    assert!(!stale_target.exists());
    rejected(
        edit(&source, &expected, &record["index"], &source)
            .args(["--precision", "1", "--overwrite"])
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn invalid_topology_and_depths_never_write_or_overwrite_sources() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture()).unwrap();
    for (number, flags) in [
        vec!["--hole", "30,100 50,100 50,120 30,120"],
        vec!["--hole", "40,100 60,100 60,120 40,120"],
        vec!["--hole", "60,100 80,150 60,150 80,100"],
        vec!["--hole", HOLE, "--hole", HOLE],
        vec!["--hole", HOLE, "--hole", "65,110 75,110 75,120 65,120"],
        vec!["--hole", HOLE, "--hole", "70,110 100,110 100,140 70,140"],
        vec!["--hole", "NaN,100 80,100 80,150"],
        vec!["--depth", "1", "--depth-unit", "m"],
    ]
    .into_iter()
    .enumerate()
    {
        let out = directory.path().join(format!("bad-area-{number}.pdf"));
        rejected(
            measure(&fixture(), &out, "area", OUTER)
                .args(flags)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    for (number, flags) in [
        vec![],
        vec!["--depth", "0", "--depth-unit", "m"],
        vec!["--depth", "NaN", "--depth-unit", "m"],
        vec!["--depth", "1"],
        vec!["--depth-unit", "m"],
    ]
    .into_iter()
    .enumerate()
    {
        let out = directory.path().join(format!("bad-volume-{number}.pdf"));
        rejected(
            measure(&fixture(), &out, "volume", OUTER)
                .args(flags)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    let too_many = directory.path().join("too-many.pdf");
    let mut command = measure(&fixture(), &too_many, "area", OUTER);
    for _ in 0..17 {
        command.args(["--hole", HOLE]);
    }
    rejected(command.output().unwrap());
    assert!(!too_many.exists());
    let existing = directory.path().join("existing.pdf");
    std::fs::write(&existing, b"keep existing output").unwrap();
    rejected(
        measure(&fixture(), &existing, "volume", OUTER)
            .args(["--depth", "20", "--depth-unit", "cm", "--overwrite"])
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep existing output");
    assert_eq!(std::fs::read(fixture()).unwrap(), original);
}

#[test]
fn cubic_prices_and_formulas_use_current_net_volume_and_reject_dimension_changes() {
    let directory = tempfile::tempdir().unwrap();
    let source = volume(directory.path());
    let original = std::fs::read(&source).unwrap();
    let initial = success(cli("quantity-table", &source).output().unwrap());
    near(&initial["rows"][0]["quantity"]["value"], 18.0);
    assert_eq!(initial["rows"][0]["quantity"]["unit"], "m3");
    let expected = directory.path().join("quantity-row.json");
    write_json(&expected, &initial["rows"][0]);
    let fields = directory.path().join("fields.json");
    write_json(
        &fields,
        &json!({"unit_price":{"amount":"2.50","currency":"CHF","per_unit":"m3","rounding_decimals":2},"custom":{}}),
    );
    let priced = directory.path().join("priced.pdf");
    success(
        cli("set-quantity-fields", &source)
            .args(["--page", "1", "--expected-record"])
            .arg(&expected)
            .arg("--fields")
            .arg(&fields)
            .arg("--out")
            .arg(&priced)
            .output()
            .unwrap(),
    );
    let before = std::fs::read(&priced).unwrap();
    let formulas = directory.path().join("formulas.json");
    write_json(
        &formulas,
        &json!([{"name":"Double volume","expression":"quantity * 2","quantity_unit":"m3","rounding_decimals":2}]),
    );
    let table = success(
        cli("quantity-table", &priced)
            .arg("--formulas")
            .arg(&formulas)
            .output()
            .unwrap(),
    );
    assert_eq!(table["rows"][0]["line_total"]["amount"], "45.00");
    assert_eq!(table["summary"]["formula_groups"][0]["complete"], true);
    assert_eq!(
        table["summary"]["formula_groups"][0]["totals"][0]["amount"],
        "36.00"
    );
    let record = records(&priced)[0].clone();
    let expected_measurement = directory.path().join("measurement.json");
    write_json(&expected_measurement, &record);
    let incompatible = directory.path().join("incompatible.pdf");
    rejected(
        edit(
            &priced,
            &expected_measurement,
            &record["index"],
            &incompatible,
        )
        .args(["--kind", "area", "--clear-depth"])
        .output()
        .unwrap(),
    );
    assert!(!incompatible.exists());
    let converted = directory.path().join("priced-feet.pdf");
    success(
        edit(&priced, &expected_measurement, &record["index"], &converted)
            .args(["--unit", "ft"])
            .output()
            .unwrap(),
    );
    let converted_table = success(cli("quantity-table", &converted).output().unwrap());
    assert_eq!(converted_table["rows"][0]["line_total"]["amount"], "45.00");
    assert_eq!(converted_table["rows"][0]["quantity"]["unit"], "ft3");
    let csv = directory.path().join("volume.csv");
    success(
        cli("quantity-table", &priced)
            .args(["--format", "csv", "--out"])
            .arg(&csv)
            .output()
            .unwrap(),
    );
    let csv_text = std::fs::read_to_string(csv).unwrap();
    assert!(csv_text.contains("m3") && csv_text.contains("45.00"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&priced).unwrap(), before);
}

#[test]
fn stale_volumes_retain_historical_inputs_but_require_explicit_recalibration() {
    let directory = tempfile::tempdir().unwrap();
    let source = volume(directory.path());
    let source_bytes = std::fs::read(&source).unwrap();
    let stale = directory.path().join("stale.pdf");
    let mut pdf = lopdf::Document::load(&source).unwrap();
    let page = pdf.get_pages()[&1];
    let content = pdf.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"q 1 0 0 RG 30 30 120 60 re S Q".to_vec(),
    ));
    pdf.get_dictionary_mut(page)
        .unwrap()
        .set("Contents", content);
    pdf.save(&stale).unwrap();
    let stale_bytes = std::fs::read(&stale).unwrap();
    let record = records(&stale)[0].clone();
    assert_eq!(record["validity"], "stale");
    assert!(record["value"].is_null() && record["area"].is_null());
    near(&record["historical_value"], 18.0);
    near(&record["historical_area"]["net"], 90.0);
    assert_eq!(record["depth"], json!({"value":20.0,"unit":"cm"}));
    let table = success(cli("quantity-table", &stale).output().unwrap());
    assert!(table["rows"][0]["quantity"].is_null());
    near(&table["rows"][0]["historical_quantity"]["value"], 18.0);
    let expected = directory.path().join("stale.json");
    write_json(&expected, &record);
    let refused = directory.path().join("refused.pdf");
    rejected(
        edit(&stale, &expected, &record["index"], &refused)
            .args(["--depth", "30", "--depth-unit", "cm"])
            .output()
            .unwrap(),
    );
    assert!(!refused.exists());
    let recalibrated = directory.path().join("recalibrated.pdf");
    success(
        edit(&stale, &expected, &record["index"], &recalibrated)
            .args(["--calibration", "40,40 140,40", "--known-distance", "20"])
            .output()
            .unwrap(),
    );
    let current = records(&recalibrated)[0].clone();
    assert_eq!(current["validity"], "current");
    assert_eq!(current["holes"], record["holes"]);
    assert_eq!(current["depth"], record["depth"]);
    near(&current["value"], 72.0);
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    assert_eq!(std::fs::read(&stale).unwrap(), stale_bytes);
}
