use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::Protection;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(action: &str, file: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(action).arg(file);
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

fn failure(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn table(file: &Path) -> Value {
    success(cli("quantity-table", file).output().unwrap())
}

#[test]
fn historical_measurements_keep_prices_but_never_supply_totals_or_formula_quantities() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, fields_path) = seed(directory.path());
    let priced = directory.path().join("priced-current.pdf");
    success(
        set(&source, &expected, &fields_path, &priced)
            .output()
            .unwrap(),
    );
    let priced_bytes = std::fs::read(&priced).unwrap();
    let configuration = directory.path().join("formula.json");
    write_json(
        &configuration,
        &json!([
            {"name":"Measured length","expression":"quantity * 2","quantity_unit":"m","rounding_decimals":2}
        ]),
    );
    for (validity, legacy) in [("legacy_unverified", true), ("stale", false)] {
        let historical = directory.path().join(format!("{validity}.pdf"));
        let mut pdf = lopdf::Document::load(&priced).unwrap();
        if legacy {
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
        } else {
            let page = pdf.get_pages()[&1];
            let content = pdf.add_object(lopdf::Stream::new(
                lopdf::Dictionary::new(),
                b"q 0 0 1 RG 30 30 120 60 re S Q".to_vec(),
            ));
            pdf.get_dictionary_mut(page)
                .unwrap()
                .set("Contents", content);
        }
        pdf.save(&historical).unwrap();
        let original = std::fs::read(&historical).unwrap();
        let result = success(
            cli("quantity-table", &historical)
                .arg("--formulas")
                .arg(&configuration)
                .output()
                .unwrap(),
        );
        let historical_row = row(&result, 1);
        assert_eq!(historical_row["measurement_validity"], validity);
        assert!(historical_row["quantity"].is_null());
        assert!(historical_row["line_total"].is_null());
        assert_eq!(historical_row["historical_quantity"]["value"], 10.0);
        assert_eq!(historical_row["historical_quantity"]["unit"], "m");
        assert_eq!(historical_row["fields"]["unit_price"]["amount"], "2.5");
        assert_eq!(result["summary"]["unverified_measurement_rows"], 1);
        assert_eq!(
            result["summary"]["legacy_measurement_rows"],
            usize::from(legacy)
        );
        assert_eq!(
            result["summary"]["stale_measurement_rows"],
            usize::from(!legacy)
        );
        assert_eq!(result["summary"]["quantity_rows"], 1);
        assert_eq!(result["summary"]["unquantified_rows"], 1);
        let groups = result["summary"]["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["unit"], "each");
        assert_eq!(groups[0]["quantity"], 1.0);
        let formulas = result["summary"]["formula_groups"].as_array().unwrap();
        assert_eq!(formulas.len(), 1);
        assert_eq!(formulas[0]["contributing_rows"], 0);
        assert_eq!(formulas[0]["error_rows"], 2);
        assert_eq!(formulas[0]["complete"], false);
        assert_eq!(formulas[0]["totals"], json!([]));
        let csv_path = directory.path().join(format!("{validity}.csv"));
        success(
            cli("quantity-table", &historical)
                .arg("--formulas")
                .arg(&configuration)
                .args(["--format", "csv", "--out"])
                .arg(&csv_path)
                .output()
                .unwrap(),
        );
        let csv = std::fs::read_to_string(&csv_path).unwrap();
        assert!(csv.contains("\"measurement_validity\""));
        assert!(csv.contains("\"historical_quantity\""));
        assert!(csv.contains(&format!("\"{validity}\"")));

        let record = success(cli("measurements", &historical).output().unwrap())["pages"][0]
            ["measurements"][0]
            .clone();
        let record_path = directory.path().join(format!("{validity}-record.json"));
        write_json(&record_path, &record);
        let recalibrated = directory.path().join(format!("{validity}-current.pdf"));
        success(
            cli("edit-measurement", &historical)
                .args(["--page", "1", "--index", "1", "--expected-record"])
                .arg(&record_path)
                .args([
                    "--calibration",
                    "40,40 140,40",
                    "--known-distance",
                    "20",
                    "--out",
                ])
                .arg(&recalibrated)
                .output()
                .unwrap(),
        );
        let fresh = table(&recalibrated);
        assert_eq!(row(&fresh, 1)["measurement_validity"], "current");
        assert_eq!(row(&fresh, 1)["quantity"]["value"], 20.0);
        assert_eq!(row(&fresh, 1)["line_total"]["amount"], "50.00");
        assert!(row(&fresh, 1)["historical_quantity"].is_null());
        assert_eq!(fresh["summary"]["unverified_measurement_rows"], 0);
        assert_eq!(std::fs::read(&historical).unwrap(), original);
    }
    assert_eq!(std::fs::read(priced).unwrap(), priced_bytes);
}

fn row(table: &Value, index: usize) -> &Value {
    table["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["register"]["page"] == 1 && row["register"]["index"] == index)
        .unwrap()
}

fn target(action: &str, source: &Path, expected: &Path, out: &Path) -> Command {
    let mut command = cli(action, source);
    command
        .args(["--page", "1", "--expected-record"])
        .arg(expected)
        .arg("--out")
        .arg(out);
    command
}

fn fields() -> Value {
    json!({"unit_price":{"amount":"2.50","currency":"CHF","per_unit":"m","rounding_decimals":2},
    "custom":{
        "Note":{"type":"text","value":"=SUM(A1:A2)"},
        "Deadline":{"type":"date","value":"2028-02-29"},
        "Allowance":{"type":"number","value":"1.250000"},
        "Stage":{"type":"choice","value":"Reviewed","options":["Draft","Reviewed"]}
    }})
}

fn seed(directory: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let counted = directory.join("counted.pdf");
    success(
        cli("count", &fixture())
            .args([
                "--group",
                "Devices",
                "--point",
                "60,60",
                "--author",
                "Plan reviewer",
                "--out",
            ])
            .arg(&counted)
            .output()
            .unwrap(),
    );
    let measured = directory.join("measured.pdf");
    success(
        cli("measure", &counted)
            .args([
                "--kind",
                "length",
                "--points",
                "40,100 140,100",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "10",
                "--unit",
                "m",
                "--author",
                "Plan reviewer",
                "--out",
            ])
            .arg(&measured)
            .output()
            .unwrap(),
    );
    let expected = directory.join("expected.json");
    write_json(&expected, row(&table(&measured), 1));
    let fields_path = directory.join("fields.json");
    write_json(&fields_path, &fields());
    (measured, expected, fields_path)
}

fn set(source: &Path, expected: &Path, fields: &Path, out: &Path) -> Command {
    let mut command = target("set-quantity-fields", source, expected, out);
    command.arg("--fields").arg(fields);
    command
}

#[test]
fn number_units_survive_cli_roundtrip_and_reject_mixed_columns_without_overwriting() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, fields_path) = seed(directory.path());
    let source_bytes = std::fs::read(&source).unwrap();
    let mut declared = fields();
    declared["custom"]["Depth"] = json!({"type":"number","value":"0.250000","unit":"m"});
    write_json(&fields_path, &declared);
    let output = directory.path().join("unit-fields.pdf");
    success(
        set(&source, &expected, &fields_path, &output)
            .output()
            .unwrap(),
    );
    let current = table(&output);
    assert_eq!(current["version"], 2);
    assert_eq!(
        row(&current, 1)["fields"]["custom"]["Depth"],
        json!({"type":"number","value":"0.25","unit":"m"})
    );
    assert_eq!(
        row(&current, 1)["quantity"],
        row(&table(&source), 1)["quantity"]
    );
    assert_eq!(row(&current, 1)["line_total"]["amount"], "25.00");
    let found = success(
        cli("quantity-table", &output)
            .args(["--query", "0.25 m"])
            .output()
            .unwrap(),
    );
    assert_eq!(found["rows"].as_array().unwrap().len(), 1);
    let configuration = directory.path().join("dimensions.json");
    write_json(
        &configuration,
        &json!([
            {"name":"Section","expression":"quantity * [Depth]","quantity_unit":"m","rounding_decimals":2},
            {"name":"Depth area","expression":"[Depth] * [Depth]","quantity_unit":null,"rounding_decimals":2}
        ]),
    );
    let calculated = success(
        cli("quantity-table", &output)
            .arg("--formulas")
            .arg(&configuration)
            .output()
            .unwrap(),
    );
    let measured = calculated["formula_rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["index"] == 1)
        .unwrap();
    assert_eq!(measured["values"]["Section"]["value"], "2.50");
    assert_eq!(measured["values"]["Section"]["unit"], "m2");
    assert_eq!(measured["values"]["Depth area"]["value"], "0.06");
    assert_eq!(measured["values"]["Depth area"]["unit"], "m2");
    let csv = directory.path().join("units.csv");
    success(
        cli("quantity-table", &output)
            .args(["--format", "csv", "--out"])
            .arg(&csv)
            .output()
            .unwrap(),
    );
    let text = std::fs::read_to_string(&csv).unwrap();
    assert!(text.contains("\"custom:Depth:value\",\"custom:Depth:unit\""));
    assert!(text.contains("\"number\",\"0.25\",\"m\""));
    let bytes = std::fs::read(&output).unwrap();
    failure(
        set(&source, &expected, &fields_path, &output)
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&output).unwrap(), bytes);
    write_json(&expected, row(&current, 0));
    for unit in [Value::Null, json!("cm")] {
        write_json(
            &fields_path,
            &json!({"unit_price":null,"custom":{"Depth":{"type":"number","value":"1","unit":unit}}}),
        );
        let refused = directory.path().join("conflicting-unit.pdf");
        let error = failure(
            set(&output, &expected, &fields_path, &refused)
                .output()
                .unwrap(),
        );
        assert!(error
            .to_string()
            .contains("conflicting type, unit or choice definitions"));
        assert!(!refused.exists());
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
    }
    let mut malformed = lopdf::Document::load(&output).unwrap();
    for object in malformed.objects.values_mut() {
        let Ok(dict) = object.as_dict_mut() else {
            continue;
        };
        let Ok(raw) = dict.get(b"SuniPDFQuantity") else {
            continue;
        };
        let mut record: Value = serde_json::from_slice(raw.as_str().unwrap()).unwrap();
        record["version"] = 1.into();
        dict.set(
            "SuniPDFQuantity",
            lopdf::Object::string_literal(record.to_string()),
        );
    }
    let invalid = directory.path().join("mislabeled-v1.pdf");
    malformed.save(&invalid).unwrap();
    assert!(failure(cli("quantity-table", &invalid).output().unwrap())
        .to_string()
        .contains("metadata version"));
    write_json(&expected, row(&current, 1));
    declared["custom"]["Depth"]["unit"] = Value::Null;
    write_json(&fields_path, &declared);
    let plain = directory.path().join("plain-number.pdf");
    success(
        set(&output, &expected, &fields_path, &plain)
            .output()
            .unwrap(),
    );
    let after = table(&plain);
    assert_eq!(after["version"], 1);
    assert!(row(&after, 1)["fields"]["custom"]["Depth"]
        .get("unit")
        .is_none());
    assert_eq!(row(&after, 1)["fields"]["custom"]["Depth"]["value"], "0.25");
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

fn auth(mut command: Command, password: &str) -> Command {
    command
        .args(["--password-env", "SUNIPDF_TEST_QUANTITY_PASSWORD"])
        .env("SUNIPDF_TEST_QUANTITY_PASSWORD", password);
    command
}

fn piped(mut command: Command, password: &str) -> Output {
    command.arg("--password-stdin");
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(password.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn quantity_fields_round_trip_clear_and_preserve_price_basis_after_measurement_unit_conversion() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, input) = seed(directory.path());
    let original = std::fs::read(&source).unwrap();
    let before = table(&source);
    assert!(row(&before, 0)["line_total"].is_null());
    assert_eq!(row(&before, 0)["quantity"]["value"], 1.0);
    assert_eq!(row(&before, 0)["quantity"]["unit"], "each");
    let priced = directory.path().join("priced.pdf");
    let output = success(set(&source, &expected, &input, &priced).output().unwrap());
    assert_eq!(output["source_content_exported"], false);
    assert_eq!(output["original_unchanged"], true);
    let after = table(&priced);
    let selected = row(&after, 1);
    assert_eq!(
        selected["register"]["annotation_id"],
        row(&before, 1)["register"]["annotation_id"]
    );
    assert_eq!(selected["quantity"], row(&before, 1)["quantity"]);
    assert_eq!(selected["line_total"]["amount"], "25.00");
    assert_eq!(selected["line_total"]["currency"], "CHF");
    assert_eq!(
        selected["fields"]["custom"]["Deadline"]["value"],
        "2028-02-29"
    );
    assert_eq!(row(&after, 0), row(&before, 0));

    let measurement = success(cli("measurements", &priced).output().unwrap())["pages"][0]
        ["measurements"][0]
        .clone();
    let measure_expected = directory.path().join("measurement.json");
    write_json(&measure_expected, &measurement);
    let converted = directory.path().join("converted.pdf");
    success(
        cli("edit-measurement", &priced)
            .args([
                "--page",
                "1",
                "--index",
                "1",
                "--unit",
                "cm",
                "--expected-record",
            ])
            .arg(&measure_expected)
            .arg("--out")
            .arg(&converted)
            .output()
            .unwrap(),
    );
    let converted_table = table(&converted);
    let converted_row = row(&converted_table, 1);
    assert_eq!(converted_row["quantity"]["unit"], "cm");
    assert_eq!(converted_row["quantity"]["value"], 1000.0);
    assert_eq!(converted_row["fields"], selected["fields"]);
    assert_eq!(converted_row["line_total"], selected["line_total"]);

    write_json(&expected, converted_row);
    let cleared = directory.path().join("cleared.pdf");
    let result = success(
        target("clear-quantity-fields", &converted, &expected, &cleared)
            .output()
            .unwrap(),
    );
    assert_eq!(result["cleared"], true);
    let cleared_table = table(&cleared);
    let cleared_row = row(&cleared_table, 1);
    assert_eq!(cleared_row["quantity"], converted_row["quantity"]);
    assert!(cleared_row["fields"]["unit_price"].is_null());
    assert!(cleared_row["line_total"].is_null());
    assert_eq!(cleared_row["fields"]["custom"], json!({}));
    // An explicitly free item has a zero cost; an item without a price still has no cost.
    write_json(&expected, row(&before, 0));
    write_json(
        &input,
        &json!({"unit_price":{"amount":"0","currency":"USD","per_unit":"each","rounding_decimals":2},"custom":{}}),
    );
    let free = directory.path().join("free.pdf");
    success(set(&source, &expected, &input, &free).output().unwrap());
    let free_table = table(&free);
    assert_eq!(row(&free_table, 0)["line_total"]["amount"], "0.00");
    assert!(row(&free_table, 1)["line_total"].is_null());
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn custom_sort_and_formula_reports_reuse_configuration_without_modifying_the_pdf() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, input) = seed(directory.path());
    let priced = directory.path().join("formula-source.pdf");
    success(set(&source, &expected, &input, &priced).output().unwrap());
    let before = std::fs::read(&priced).unwrap();
    let configuration = directory.path().join("formulas.json");
    write_json(
        &configuration,
        &json!([
            {"name":"Allowance quantity","expression":"quantity * [Allowance]","quantity_unit":"m","rounding_decimals":2},
            {"name":"Missing input","expression":"[Absent] + 1","quantity_unit":null,"rounding_decimals":2}
        ]),
    );
    for descending in [false, true] {
        let mut command = cli("quantity-table", &priced);
        command
            .args(["--sort-custom", "Allowance", "--formulas"])
            .arg(&configuration);
        if descending {
            command.arg("--descending");
        }
        let result = success(command.output().unwrap());
        assert_eq!(result["filter"]["custom_sort"], "Allowance");
        assert_eq!(result["rows"][0]["register"]["index"], 1);
        assert_eq!(
            result["formula_rows"][0]["values"]["Allowance quantity"]["value"],
            "12.50"
        );
        assert_eq!(
            result["formula_rows"][0]["values"]["Allowance quantity"]["unit"],
            "m"
        );
        assert_eq!(
            result["formula_rows"][0]["values"]["Missing input"]["error"],
            "missing_field"
        );
        assert!(result["formula_rows"][0]["values"]["Missing input"]["value"].is_null());
        assert_eq!(
            result["formula_rows"][1]["values"]["Allowance quantity"]["error"],
            "unit_mismatch"
        );
        assert_eq!(
            result["formulas"][0]["expression"],
            "quantity * [Allowance]"
        );
        let groups = result["summary"]["formula_groups"].as_array().unwrap();
        let allowance = groups
            .iter()
            .find(|group| group["formula_name"] == "Allowance quantity")
            .unwrap();
        assert_eq!(allowance["matched_rows"], 2);
        assert_eq!(allowance["contributing_rows"], 1);
        assert_eq!(allowance["error_rows"], 1);
        assert_eq!(allowance["complete"], false);
        assert_eq!(allowance["totals"][0]["amount"], "12.50");
        assert_eq!(allowance["totals"][0]["unit"], "m");
        let missing = groups
            .iter()
            .find(|group| group["formula_name"] == "Missing input")
            .unwrap();
        assert!(missing["totals"].as_array().unwrap().is_empty());
        assert_eq!(missing["errors"]["missing_field"], 2);
    }
    let csv_path = directory.path().join("formula-report.csv");
    success(
        cli("quantity-table", &priced)
            .args(["--sort-custom", "Allowance", "--formulas"])
            .arg(&configuration)
            .args(["--format", "csv", "--out"])
            .arg(&csv_path)
            .output()
            .unwrap(),
    );
    let csv = std::fs::read_to_string(&csv_path).unwrap();
    assert!(csv.contains("formula:Allowance quantity:value"));
    assert!(csv.contains("12.50"));
    assert!(csv.contains("unit_mismatch"));
    assert!(csv.contains("missing_field"));
    assert!(csv.contains("\"formula_subtotal\""));
    assert!(csv.contains("\"formula_coverage\""));
    let invalid_out = directory.path().join("invalid-formula.csv");
    write_json(
        &configuration,
        &json!([{"name":"Rejected","expression":"exec(1)","quantity_unit":null,"rounding_decimals":2}]),
    );
    failure(
        cli("quantity-table", &priced)
            .arg("--formulas")
            .arg(&configuration)
            .args(["--format", "csv", "--out"])
            .arg(&invalid_out)
            .output()
            .unwrap(),
    );
    assert!(!invalid_out.exists());
    let register = success(
        cli("annotation-register", &priced)
            .args(["--sort-custom", "No such column", "--descending"])
            .output()
            .unwrap(),
    );
    assert_eq!(register["filter"]["custom_sort"], "No such column");
    assert_eq!(register["rows"][0]["index"], 0);
    assert_eq!(std::fs::read(&priced).unwrap(), before);
}

#[test]
fn signed_numeric_csv_and_angular_formula_coverage_survive_saved_pdf_roundtrip() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, input) = seed(directory.path());
    let mut entered = fields();
    entered["custom"]["Allowance"]["value"] = "-2.5".into();
    entered["custom"]["Note"]["value"] = "-2+3".into();
    write_json(&input, &entered);
    let priced = directory.path().join("signed-fields.pdf");
    success(set(&source, &expected, &input, &priced).output().unwrap());
    let angle = directory.path().join("with-angle.pdf");
    success(
        cli("measure", &priced)
            .args([
                "--kind",
                "angle",
                "--points",
                "150,150 200,150 200,200",
                "--out",
            ])
            .arg(&angle)
            .output()
            .unwrap(),
    );
    let before = std::fs::read(&angle).unwrap();
    let configuration = directory.path().join("signed-formulas.json");
    write_json(
        &configuration,
        &json!([
            {"name":"Negative","expression":"-1.005","quantity_unit":null,"rounding_decimals":2},
            {"name":"Angle","expression":"quantity / 2","quantity_unit":"deg","rounding_decimals":2}
        ]),
    );
    let result = success(
        cli("quantity-table", &angle)
            .arg("--formulas")
            .arg(&configuration)
            .output()
            .unwrap(),
    );
    assert_eq!(result["summary"]["quantity_rows"], 3);
    assert_eq!(result["summary"]["additive_quantity_rows"], 2);
    assert_eq!(result["summary"]["angular_rows"], 1);
    assert_eq!(result["summary"]["unquantified_rows"], 0);
    assert_eq!(result["summary"]["excluded_quantity_rows"], 1);
    let summary = &result["summary"];
    assert_eq!(
        summary["quantity_rows"].as_u64().unwrap() + summary["unquantified_rows"].as_u64().unwrap(),
        summary["matched_rows"].as_u64().unwrap()
    );
    assert_eq!(
        summary["additive_quantity_rows"].as_u64().unwrap()
            + summary["excluded_quantity_rows"].as_u64().unwrap(),
        summary["matched_rows"].as_u64().unwrap()
    );
    assert_eq!(
        result["summary"]["formula_groups"][0]["totals"][0]["amount"],
        "-3.03"
    );
    assert_eq!(result["summary"]["formula_groups"][0]["complete"], true);
    assert_eq!(
        result["summary"]["formula_groups"][1]["totals"][0]["amount"],
        "45.00"
    );
    assert_eq!(
        result["summary"]["formula_groups"][1]["totals"][0]["unit"],
        "deg"
    );
    assert_eq!(result["summary"]["formula_groups"][1]["error_rows"], 2);
    let output = directory.path().join("signed.csv");
    success(
        cli("quantity-table", &angle)
            .arg("--formulas")
            .arg(&configuration)
            .args(["--format", "csv", "--out"])
            .arg(&output)
            .output()
            .unwrap(),
    );
    let csv = std::fs::read_to_string(&output).unwrap();
    assert!(csv.contains("\"number\",\"-2.5\""));
    assert!(csv.contains("\"text\",\"'-2+3\""));
    assert!(csv.contains("\"-1.01\""));
    assert!(csv.contains("\"-3.03\""));
    assert!(!csv.contains("\"'-1.01\""));
    assert!(!csv.contains("\"'-3.03\""));
    let metadata = csv
        .lines()
        .find(|line| line.starts_with("\"metadata\""))
        .unwrap();
    assert!(metadata.contains("\"\"unquantified_rows\"\":0"));
    assert!(metadata.contains("\"\"excluded_quantity_rows\"\":1"));
    assert_eq!(std::fs::read(&angle).unwrap(), before);
}

#[test]
fn table_exports_real_filters_source_identity_csv_formula_guards_and_never_overwrites() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, input) = seed(directory.path());
    let priced = directory.path().join("priced.pdf");
    success(set(&source, &expected, &input, &priced).output().unwrap());
    let all = table(&priced);
    assert_eq!(all["rows"].as_array().unwrap().len(), 2);
    let hash = all["source_sha256"].as_str().unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let filtered = success(
        cli("quantity-table", &priced)
            .args([
                "--count-group",
                "Devices",
                "--author",
                "Plan reviewer",
                "--group-by",
                "count_group",
                "--sort",
                "author",
                "--descending",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(filtered["rows"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["rows"][0]["register"]["count_group"], "Devices");
    assert_eq!(filtered["filter"]["count_group"], "Devices");
    assert_eq!(filtered["filter"]["author"], "Plan reviewer");
    assert_eq!(filtered["sort"], "author");
    assert_eq!(filtered["descending"], true);
    assert_eq!(filtered["group_by"], "count_group");
    assert_eq!(filtered["summary"]["groups"][0]["group"], "Devices");
    assert_eq!(filtered["summary"]["groups"][0]["missing_prices"], 1);
    let missing = success(
        cli("quantity-table", &priced)
            .args(["--query", "definitely absent"])
            .output()
            .unwrap(),
    );
    assert_eq!(missing["rows"], json!([]));
    let second_page = success(
        cli("quantity-table", &priced)
            .args(["--page", "2"])
            .output()
            .unwrap(),
    );
    assert_eq!(second_page["rows"], json!([]));
    let with_note = directory.path().join("with-note.pdf");
    success(
        cli("annotate", &priced)
            .args([
                "--kind",
                "freetext",
                "--rect",
                "40,180,240,220",
                "--text",
                "Review note",
                "--out",
            ])
            .arg(&with_note)
            .output()
            .unwrap(),
    );
    let unquantified = table(&with_note);
    assert!(row(&unquantified, 2)["quantity"].is_null());
    assert!(row(&unquantified, 2)["line_total"].is_null());
    assert_eq!(unquantified["summary"]["unquantified_rows"], 1);
    let json_path = directory.path().join("table.json");
    success(
        cli("quantity-table", &priced)
            .arg("--out")
            .arg(&json_path)
            .output()
            .unwrap(),
    );
    let exported: Value = serde_json::from_slice(&std::fs::read(&json_path).unwrap()).unwrap();
    assert_eq!(exported["rows"], all["rows"]);
    let csv_path = directory.path().join("table.csv");
    success(
        cli("quantity-table", &priced)
            .args(["--format", "csv", "--out"])
            .arg(&csv_path)
            .output()
            .unwrap(),
    );
    let csv = std::fs::read_to_string(&csv_path).unwrap();
    assert!(
        csv.contains("'=SUM(A1:A2)"),
        "CSV did not neutralize a formula: {csv}"
    );
    assert!(csv.contains("25.00"));
    failure(
        cli("quantity-table", &priced)
            .args(["--format", "csv"])
            .output()
            .unwrap(),
    );
    failure(
        cli("quantity-table", &priced)
            .args(["--page", "9999"])
            .output()
            .unwrap(),
    );
    for out in [&source, &priced, &json_path, &csv_path] {
        let before = std::fs::read(out).unwrap();
        assert_eq!(
            failure(
                cli("quantity-table", &priced)
                    .arg("--out")
                    .arg(out)
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
        assert_eq!(std::fs::read(out).unwrap(), before);
    }
}

fn change_annotation(
    source: &Path,
    output: &Path,
    index: usize,
    change: impl FnOnce(&mut lopdf::Dictionary),
) {
    let mut pdf = lopdf::Document::load(source).unwrap();
    let page = pdf.get_dictionary(pdf.get_pages()[&1]).unwrap();
    let annotations = pdf
        .dereference(page.get(b"Annots").unwrap())
        .unwrap()
        .1
        .as_array()
        .unwrap();
    let id = annotations[index].as_reference().unwrap();
    change(pdf.get_dictionary_mut(id).unwrap());
    pdf.save(output).unwrap();
}

#[test]
fn stale_ambiguous_locked_and_invalid_inputs_refuse_atomically_without_overwriting() {
    let directory = tempfile::tempdir().unwrap();
    let (source, expected, input) = seed(directory.path());
    let original = std::fs::read(&source).unwrap();
    let priced = directory.path().join("priced.pdf");
    success(set(&source, &expected, &input, &priced).output().unwrap());
    let out = directory.path().join("refused.pdf");
    failure(set(&priced, &expected, &input, &out).output().unwrap());
    failure(
        target("clear-quantity-fields", &priced, &expected, &out)
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
    for destination in [&source, &priced] {
        let before = std::fs::read(destination).unwrap();
        assert_eq!(
            failure(
                set(&source, &expected, &input, destination)
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
        assert_eq!(std::fs::read(destination).unwrap(), before);
    }
    let locked = directory.path().join("locked.pdf");
    change_annotation(&source, &locked, 1, |annotation| {
        annotation.set("F", 128);
    });
    let locked_table = table(&locked);
    assert_eq!(row(&locked_table, 1)["editable"], false);
    write_json(&expected, row(&locked_table, 1));
    failure(set(&locked, &expected, &input, &out).output().unwrap());
    assert!(!out.exists());

    let normal_table = table(&source);
    write_json(&expected, row(&normal_table, 1));
    let duplicate = directory.path().join("ambiguous.pdf");
    let identity = row(&normal_table, 1)["register"]["annotation_id"]
        .as_str()
        .unwrap();
    change_annotation(&source, &duplicate, 0, |annotation| {
        annotation.set("NM", lopdf::text_string(identity));
    });
    failure(set(&duplicate, &expected, &input, &out).output().unwrap());
    assert!(!out.exists());

    let mut invalid = Vec::new();
    let mut value = fields();
    value["unit_price"]["amount"] = json!(2.5);
    invalid.push(value);
    let mut value = fields();
    value["unit_price"]["amount"] = json!("-1");
    invalid.push(value);
    let mut value = fields();
    value["unit_price"]["currency"] = json!("chf");
    invalid.push(value);
    let mut value = fields();
    value["custom"]["Deadline"]["value"] = json!("2027-02-29");
    invalid.push(value);
    let mut value = fields();
    value["custom"]["Stage"]["value"] = json!("Unknown");
    invalid.push(value);
    let mut value = fields();
    value["unexpected"] = json!(true);
    invalid.push(value);
    let mut value = fields();
    value["unit_price"]["per_unit"] = json!("m2");
    invalid.push(value);
    for value in invalid {
        write_json(&input, &value);
        failure(set(&source, &expected, &input, &out).output().unwrap());
        assert!(!out.exists());
    }
    std::fs::write(
        &input,
        br#"{"unit_price":null,"unit_price":null,"custom":{}}"#,
    )
    .unwrap();
    failure(set(&source, &expected, &input, &out).output().unwrap());
    std::fs::write(&input, br#"{"unit_price":null,"custom":{"A":{"type":"text","value":"one"},"A":{"type":"text","value":"two"}}}"#).unwrap();
    failure(set(&source, &expected, &input, &out).output().unwrap());
    std::fs::write(&input, vec![b' '; 65537]).unwrap();
    failure(set(&source, &expected, &input, &out).output().unwrap());
    write_json(&input, &fields());
    std::fs::write(&expected, vec![b' '; 65537]).unwrap();
    failure(set(&source, &expected, &input, &out).output().unwrap());
    assert!(!out.exists());
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn aes_user_owner_permissions_export_guards_and_password_sources_are_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let (plain, expected, input) = seed(directory.path());
    let bytes = std::fs::read(&plain).unwrap();
    let opening = format!("quantity-opening-{}", std::process::id());
    let owner = format!("quantity-owner-{}", std::process::id());
    for aes256 in [false, true] {
        for (name, allow_copy, allow_annotate) in [
            ("allowed", true, true),
            ("no-copy", false, true),
            ("no-annotate", true, false),
        ] {
            let protected = sunipdf_core::security::protect(
                &bytes,
                None,
                &Protection {
                    user_password: opening.clone(),
                    owner_password: owner.clone(),
                    allow_copy,
                    allow_annotate,
                    allow_print: false,
                    allow_modify: false,
                    aes256,
                },
            )
            .unwrap();
            let source = directory.path().join(format!("{aes256}-{name}.pdf"));
            std::fs::write(&source, &protected).unwrap();
            for (credential_name, credential) in [("user", &opening), ("owner", &owner)] {
                let export = piped(cli("quantity-table", &source), credential);
                if allow_copy {
                    success(export);
                } else {
                    assert_eq!(failure(export)["error"], "security_restricted");
                    let csv = directory
                        .path()
                        .join(format!("{aes256}-{name}-{credential_name}.csv"));
                    let mut command = cli("quantity-table", &source);
                    command.args(["--format", "csv", "--out"]).arg(&csv);
                    assert_eq!(
                        failure(piped(command, credential))["error"],
                        "security_restricted"
                    );
                    assert!(!csv.exists());
                }
                let out = directory
                    .path()
                    .join(format!("{aes256}-{name}-{credential_name}.pdf"));
                let output = auth(set(&source, &expected, &input, &out), credential)
                    .output()
                    .unwrap();
                if !allow_annotate {
                    failure(output);
                    assert!(!out.exists());
                    continue;
                }
                let result = success(output);
                assert_eq!(result["source_content_exported"], false);
                assert_eq!(result["encrypted"], true);
                for key in [
                    "rows",
                    "fields",
                    "unit_price",
                    "line_total",
                    "contents",
                    "quantity",
                ] {
                    assert!(result.get(key).is_none());
                }
                let properties = success(piped(cli("properties", &out), credential));
                assert_eq!(properties["permissions"]["copy"], allow_copy);
                assert_eq!(properties["permissions"]["annotate"], allow_annotate);
                assert_eq!(properties["permissions"]["modify"], false);
                assert_eq!(properties["permissions"]["print"], false);
                if allow_copy {
                    let exported = success(piped(cli("quantity-table", &out), credential));
                    assert_eq!(row(&exported, 1)["line_total"]["amount"], "25.00");
                    let current = directory.path().join("current.json");
                    write_json(&current, row(&exported, 1));
                    let cleared = directory
                        .path()
                        .join(format!("{aes256}-{name}-{credential_name}-clear.pdf"));
                    success(piped(
                        target("clear-quantity-fields", &out, &current, &cleared),
                        credential,
                    ));
                    let reread = success(piped(cli("quantity-table", &cleared), credential));
                    assert!(row(&reread, 1)["line_total"].is_null());
                } else {
                    assert_eq!(
                        failure(piped(cli("quantity-table", &out), credential))["error"],
                        "security_restricted"
                    );
                }
            }
            assert_eq!(std::fs::read(source).unwrap(), protected);
        }
    }
}
