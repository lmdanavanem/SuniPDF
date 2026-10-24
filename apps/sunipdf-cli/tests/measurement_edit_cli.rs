use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::Protection;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(command: &str, file: &Path) -> Command {
    let mut process = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    process.arg(command).arg(file);
    process
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

fn no_measurement_export(output: Output) -> Value {
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let result = success(output);
    assert!(result["measurement"].is_null());
    assert_eq!(result["export_unavailable"], true);
    for field in [
        "points",
        "holes",
        "depth",
        "area",
        "calibration",
        "known_distance",
        "value",
        "unit",
        "precision",
        "label",
    ] {
        assert!(
            !stdout.contains(&format!("\"{field}\"")),
            "stdout exposed {field}"
        );
    }
    result
}

fn auth(mut command: Command, password: &str) -> Command {
    command
        .args(["--password-env", "SUNIPDF_TEST_EDIT_PASSWORD"])
        .env("SUNIPDF_TEST_EDIT_PASSWORD", password);
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

fn records(file: &Path) -> Value {
    success(cli("measurements", file).output().unwrap())
}

fn seed(directory: &Path) -> (PathBuf, Value, PathBuf) {
    let file = directory.join("measured.pdf");
    let created = success(
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
                "--author",
                "Plan reviewer",
                "--out",
            ])
            .arg(&file)
            .output()
            .unwrap(),
    );
    assert_eq!(created["measurement"]["version"], 2);
    assert_eq!(created["measurement"]["validity"], "current");
    assert!(created["measurement"]["historical_value"].is_null());
    let record = records(&file)["pages"][0]["measurements"][0].clone();
    assert_eq!(record["validity"], "current");
    let expected = directory.join("expected.json");
    std::fs::write(&expected, serde_json::to_vec(&record).unwrap()).unwrap();
    (file, record, expected)
}

fn historical_copy(source: &Path, out: &Path, legacy: bool) {
    let mut pdf = lopdf::Document::load(source).unwrap();
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
        // Same page box, different drawing: page dimensions alone cannot certify a takeoff.
        let page = pdf.get_pages()[&1];
        let content = pdf.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"q 0 0 1 RG 30 30 120 60 re S Q".to_vec(),
        ));
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Contents", content);
    }
    pdf.save(out).unwrap();
}

#[test]
fn historical_measurements_require_real_recalibration_and_reject_forged_current_records() {
    let directory = tempfile::tempdir().unwrap();
    let (source, original_record, _) = seed(directory.path());
    let source_bytes = std::fs::read(&source).unwrap();
    for (name, legacy) in [("legacy_unverified", true), ("stale", false)] {
        let historical = directory.path().join(format!("{name}.pdf"));
        historical_copy(&source, &historical, legacy);
        let before = std::fs::read(&historical).unwrap();
        let record = records(&historical)["pages"][0]["measurements"][0].clone();
        assert_eq!(record["version"], 2);
        assert_eq!(record["validity"], name);
        assert!(record["value"].is_null());
        assert!(record["label"].is_null());
        assert_eq!(record["historical_value"], 10.0);
        assert_eq!(record["annotation_id"], original_record["annotation_id"]);
        let expected = directory.path().join(format!("{name}.json"));
        std::fs::write(&expected, serde_json::to_vec(&record).unwrap()).unwrap();
        for (change, arguments) in [
            ("unit-only", vec!["--unit", "cm"]),
            ("precision-only", vec!["--precision", "1"]),
            ("geometry-only", vec!["--points", "40,80 240,80"]),
        ] {
            let rejected = directory.path().join(format!("{name}-{change}.pdf"));
            failure(
                edit(&historical, &record, &expected, &rejected)
                    .args(arguments)
                    .output()
                    .unwrap(),
            );
            assert!(!rejected.exists());
        }
        let forged_path = directory.path().join(format!("{name}-forged.json"));
        let mut forged = record.clone();
        forged["validity"] = "current".into();
        forged["value"] = 10.0.into();
        std::fs::write(&forged_path, serde_json::to_vec(&forged).unwrap()).unwrap();
        let rejected = directory.path().join(format!("{name}-forged.pdf"));
        failure(
            edit(&historical, &record, &forged_path, &rejected)
                .args(["--calibration", "40,40 140,40", "--known-distance", "20"])
                .output()
                .unwrap(),
        );
        assert!(!rejected.exists());

        // An old version-1 CLI record remains readable, but is never upgraded by parsing alone.
        if legacy {
            let mut old_record = record.clone();
            old_record["version"] = 1.into();
            old_record.as_object_mut().unwrap().remove("validity");
            std::fs::write(&expected, serde_json::to_vec(&old_record).unwrap()).unwrap();
        }
        let repaired = directory.path().join(format!("{name}-recalibrated.pdf"));
        let updated = success(
            edit(&historical, &record, &expected, &repaired)
                .args([
                    "--calibration",
                    "40,40 140,40",
                    "--known-distance",
                    "20",
                    "--unit",
                    "m",
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(updated["measurement"]["validity"], "current");
        assert_eq!(updated["measurement"]["value"], 20.0);
        let reopened = records(&repaired)["pages"][0]["measurements"][0].clone();
        assert_eq!(reopened["validity"], "current");
        assert_eq!(reopened["value"], 20.0);
        assert!(reopened["historical_value"].is_null());
        assert_eq!(reopened["annotation_id"], record["annotation_id"]);
        assert_eq!(std::fs::read(&historical).unwrap(), before);
    }
    assert_eq!(std::fs::read(source).unwrap(), source_bytes);
}

fn edit(file: &Path, record: &Value, expected: &Path, out: &Path) -> Command {
    let mut command = cli("edit-measurement", file);
    command
        .args(["--page", "1", "--index"])
        .arg(record["index"].as_u64().unwrap().to_string())
        .arg("--expected-record")
        .arg(expected)
        .arg("--out")
        .arg(out);
    command
}

#[test]
fn editing_preserves_identity_other_records_and_original_with_explicit_physical_conversion() {
    let directory = tempfile::tempdir().unwrap();
    let (source, record, expected) = seed(directory.path());
    let original = std::fs::read(&source).unwrap();
    let output = directory.path().join("edited.pdf");
    let result = success(
        edit(&source, &record, &expected, &output)
            .args([
                "--points",
                "40,80 240,80",
                "--unit",
                "ft",
                "--precision",
                "3",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(result["original_unchanged"], true);
    let after = records(&output);
    let edited = &after["pages"][0]["measurements"][0];
    assert_eq!(edited["annotation_id"], record["annotation_id"]);
    assert_eq!(edited["index"], record["index"]);
    assert_eq!(edited["precision"], 3);
    assert_eq!(edited["unit"], "ft");
    assert!((edited["value"].as_f64().unwrap() - 20.0 / 0.3048).abs() < 1e-7);
    assert!(after["pages"][1]["measurements"]
        .as_array()
        .unwrap()
        .is_empty());
    let annotations = success(cli("annotation-register", &output).output().unwrap());
    assert!(annotations.to_string().contains("Plan reviewer"));
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let rejected = directory.path().join("stale-must-not-exist.pdf");
    failure(
        edit(&output, &record, &expected, &rejected)
            .args(["--precision", "1"])
            .output()
            .unwrap(),
    );
    assert!(!rejected.exists());
    let before_output = std::fs::read(&output).unwrap();
    for destination in [&source, &output] {
        assert_eq!(
            failure(
                edit(&source, &record, &expected, destination)
                    .args(["--precision", "1", "--overwrite"])
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&output).unwrap(), before_output);
}

#[test]
fn priced_measurement_dimension_changes_refuse_without_output_and_unit_conversion_keeps_price() {
    let directory = tempfile::tempdir().unwrap();
    let (source, _, _) = seed(directory.path());
    let source_bytes = std::fs::read(&source).unwrap();
    let initial = success(cli("quantity-table", &source).output().unwrap());
    let quantity_expected = directory.path().join("quantity-expected.json");
    std::fs::write(
        &quantity_expected,
        serde_json::to_vec(&initial["rows"][0]).unwrap(),
    )
    .unwrap();
    let fields = serde_json::json!({
        "unit_price": {
            "amount": "2.5", "currency": "CHF", "per_unit": "m", "rounding_decimals": 2
        },
        "custom": {"Material": {"type": "text", "value": "Cable"}}
    });
    let fields_path = directory.path().join("price-fields.json");
    std::fs::write(&fields_path, serde_json::to_vec(&fields).unwrap()).unwrap();
    let priced = directory.path().join("priced.pdf");
    success(
        cli("set-quantity-fields", &source)
            .args(["--page", "1", "--expected-record"])
            .arg(&quantity_expected)
            .arg("--fields")
            .arg(&fields_path)
            .arg("--out")
            .arg(&priced)
            .output()
            .unwrap(),
    );
    let priced_bytes = std::fs::read(&priced).unwrap();
    let record = records(&priced)["pages"][0]["measurements"][0].clone();
    let expected = directory.path().join("priced-measurement.json");
    std::fs::write(&expected, serde_json::to_vec(&record).unwrap()).unwrap();
    let before = success(cli("quantity-table", &priced).output().unwrap());
    assert_eq!(before["rows"][0]["fields"], fields);
    assert_eq!(before["rows"][0]["line_total"]["amount"], "25.00");

    for (kind, points) in [
        ("area", "40,80 140,80 140,180 40,180"),
        ("angle", "140,80 40,80 40,180"),
    ] {
        let rejected = directory.path().join(format!("priced-{kind}-refused.pdf"));
        let error = failure(
            edit(&priced, &record, &expected, &rejected)
                .args(["--kind", kind, "--points", points])
                .output()
                .unwrap(),
        );
        assert!(
            error.to_string().contains("quantity dimension"),
            "{kind} must fail because the retained price is incompatible: {error}"
        );
        assert!(!rejected.exists());
        assert_eq!(std::fs::read(&priced).unwrap(), priced_bytes);
        let unchanged = success(cli("quantity-table", &priced).output().unwrap());
        assert_eq!(unchanged["rows"][0]["fields"], before["rows"][0]["fields"]);
        assert_eq!(
            unchanged["rows"][0]["line_total"],
            before["rows"][0]["line_total"]
        );
    }

    let converted = directory.path().join("priced-centimetres.pdf");
    success(
        edit(&priced, &record, &expected, &converted)
            .args(["--unit", "cm"])
            .output()
            .unwrap(),
    );
    let reopened = records(&converted);
    let measured = &reopened["pages"][0]["measurements"][0];
    assert_eq!(measured["validity"], "current");
    assert_eq!(measured["annotation_id"], record["annotation_id"]);
    assert_eq!(measured["unit"], "cm");
    assert_eq!(measured["value"], 1000.0);
    let after = success(cli("quantity-table", &converted).output().unwrap());
    let row = &after["rows"][0];
    assert_eq!(row["fields"], before["rows"][0]["fields"]);
    assert_eq!(row["quantity"]["unit"], "cm");
    assert_eq!(row["quantity"]["value"], 1000.0);
    assert_eq!(row["line_total"], before["rows"][0]["line_total"]);
    assert_eq!(std::fs::read(&priced).unwrap(), priced_bytes);
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
}

#[test]
fn every_kind_can_be_edited_and_wrong_expected_identity_geometry_or_page_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (source, record, expected) = seed(directory.path());
    for (kind, points, value) in [
        ("length", "40,80 240,80", 20.0),
        ("polyline", "40,80 140,80 140,180", 20.0),
        ("perimeter", "40,80 140,80 140,180 40,180", 40.0),
        ("area", "40,80 140,80 140,180 40,180", 100.0),
        ("angle", "140,80 40,80 40,180", 90.0),
    ] {
        let out = directory.path().join(format!("{kind}.pdf"));
        success(
            edit(&source, &record, &expected, &out)
                .args(["--kind", kind, "--points", points])
                .output()
                .unwrap(),
        );
        let exported = records(&out);
        let edited = &exported["pages"][0]["measurements"][0];
        assert_eq!(edited["kind"], kind);
        assert!((edited["value"].as_f64().unwrap() - value).abs() < 1e-8);
        assert_eq!(edited["annotation_id"], record["annotation_id"]);
    }
    let out = directory.path().join("refused.pdf");
    for (field, wrong) in [
        ("annotation_id", Value::String("another-annotation".into())),
        ("kind", Value::String("polyline".into())),
        ("points", serde_json::json!([[40, 80], [150, 80]])),
        ("version", Value::from(4)),
        ("validity", Value::String("stale".into())),
        ("index", Value::from(123)),
        ("annotation_id", Value::Null),
    ] {
        let mut changed = record.clone();
        changed[field] = wrong;
        std::fs::write(&expected, serde_json::to_vec(&changed).unwrap()).unwrap();
        failure(
            edit(&source, &record, &expected, &out)
                .args(["--precision", "1"])
                .output()
                .unwrap(),
        );
        assert!(!out.exists(), "created output for changed {field}");
    }
    std::fs::write(&expected, serde_json::to_vec(&record).unwrap()).unwrap();
    let mut wrong_page = cli("edit-measurement", &source);
    wrong_page
        .args(["--page", "2", "--index", "0", "--expected-record"])
        .arg(&expected)
        .args(["--precision", "1", "--out"])
        .arg(&out);
    failure(wrong_page.output().unwrap());
    assert!(!out.exists());
    failure(
        edit(&source, &record, &expected, &out)
            .args(["--points", "40,80 99999,80"])
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
}

#[test]
fn angles_need_no_page_scale_and_explicit_recalibration_changes_only_the_selected_measurement() {
    let directory = tempfile::tempdir().unwrap();
    let angle = directory.path().join("angle.pdf");
    let created = success(
        cli("measure", &fixture())
            .args([
                "--kind",
                "angle",
                "--points",
                "140,80 40,80 40,180",
                "--out",
            ])
            .arg(&angle)
            .output()
            .unwrap(),
    );
    assert_eq!(created["measurement"]["value"], 90.0);
    let record = records(&angle)["pages"][0]["measurements"][0].clone();
    let expected = directory.path().join("angle.json");
    std::fs::write(&expected, serde_json::to_vec(&record).unwrap()).unwrap();
    let out = directory.path().join("needs-scale.pdf");
    failure(
        edit(&angle, &record, &expected, &out)
            .args(["--kind", "length", "--points", "40,80 140,80"])
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
    let changed = success(
        edit(&angle, &record, &expected, &out)
            .args([
                "--kind",
                "length",
                "--points",
                "40,80 140,80",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "5",
                "--unit",
                "m",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(changed["measurement"]["value"], 5.0);
    let scaled = directory.path().join("saved-page-scale.pdf");
    success(
        cli("set-page-calibration", &out)
            .args([
                "--page",
                "1",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "20",
                "--out",
            ])
            .arg(&scaled)
            .output()
            .unwrap(),
    );
    let expected_record = records(&scaled)["pages"][0]["measurements"][0].clone();
    std::fs::write(&expected, serde_json::to_vec(&expected_record).unwrap()).unwrap();
    let recalibrated = directory.path().join("recalibrated.pdf");
    let result = success(
        edit(&scaled, &expected_record, &expected, &recalibrated)
            .args(["--use-page-calibration", "--unit", "cm"])
            .output()
            .unwrap(),
    );
    assert_eq!(result["measurement"]["value"], 2000.0);
    assert_eq!(
        records(&scaled)["pages"][0]["measurements"][0]["value"],
        5.0
    );
}

#[test]
fn aes_copies_preserve_permissions_use_env_and_stdin_and_never_bypass_annotation_or_copy_restrictions(
) {
    let directory = tempfile::tempdir().unwrap();
    let (plain, record, expected) = seed(directory.path());
    let bytes = std::fs::read(plain).unwrap();
    let opening = format!("edit-opening-{}", std::process::id());
    let owner = format!("edit-owner-{}", std::process::id());
    for aes256 in [false, true] {
        for (name, allow_copy, allow_annotate) in [
            ("allowed", true, true),
            ("no-copy", false, true),
            ("no-annotations", true, false),
        ] {
            let protection = Protection {
                user_password: opening.clone(),
                owner_password: owner.clone(),
                allow_copy,
                allow_annotate,
                allow_print: false,
                allow_modify: false,
                aes256,
            };
            let encrypted = sunipdf_core::security::protect(&bytes, None, &protection).unwrap();
            let source = directory.path().join(format!("{aes256}-{name}.pdf"));
            std::fs::write(&source, &encrypted).unwrap();
            for (credential_name, credential) in [("opening", &opening), ("owner", &owner)] {
                let out = directory
                    .path()
                    .join(format!("{aes256}-{name}-{credential_name}-edit.pdf"));
                let mut command = edit(&source, &record, &expected, &out);
                command.args(["--precision", "1"]);
                let output = auth(command, credential).output().unwrap();
                if !allow_annotate {
                    assert_eq!(failure(output)["error"], "security_restricted");
                    assert!(!out.exists());
                    continue;
                }
                let result = if allow_copy {
                    success(output)
                } else {
                    no_measurement_export(output)
                };
                assert_eq!(result["encrypted"], true);
                let properties = success(piped(cli("properties", &out), credential));
                assert_eq!(properties["permissions"]["copy"], allow_copy);
                assert_eq!(properties["permissions"]["annotate"], true);
                assert_eq!(properties["permissions"]["modify"], false);
                assert_eq!(properties["permissions"]["print"], false);
                for read in ["measurements", "page-calibration"] {
                    let output = piped(cli(read, &out), credential);
                    if allow_copy {
                        success(output);
                    } else {
                        assert_eq!(failure(output)["error"], "security_restricted");
                    }
                }
            }
            assert_eq!(std::fs::read(source).unwrap(), encrypted);
        }
    }
}

#[test]
fn restricted_copy_outputs_hide_manual_and_saved_scales_while_preserving_real_annotations() {
    // This is the only PDFium engine in this integration-test binary; child CLIs are isolated.
    let engine = sunipdf_core::Engine::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (measured, record, expected) = seed(directory.path());
    let calibrated = directory.path().join("with-page-scale.pdf");
    success(
        cli("set-page-calibration", &measured)
            .args([
                "--page",
                "1",
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "19.25",
                "--unit",
                "m",
                "--out",
            ])
            .arg(&calibrated)
            .output()
            .unwrap(),
    );
    let plain = std::fs::read(&calibrated).unwrap();
    let opening = format!("hidden-scale-opening-{}", std::process::id());
    let owner = format!("hidden-scale-owner-{}", std::process::id());
    for aes256 in [false, true] {
        let protected = sunipdf_core::security::protect(
            &plain,
            None,
            &Protection {
                user_password: opening.clone(),
                owner_password: owner.clone(),
                allow_copy: false,
                allow_print: false,
                allow_modify: false,
                allow_annotate: true,
                aes256,
            },
        )
        .unwrap();
        let source = directory.path().join(format!("protected-{aes256}.pdf"));
        std::fs::write(&source, &protected).unwrap();
        for action in [
            "measure-manual",
            "measure-saved",
            "edit-manual",
            "edit-saved",
        ] {
            let out = directory.path().join(format!("{aes256}-{action}.pdf"));
            let mut command = if action.starts_with("edit") {
                edit(&source, &record, &expected, &out)
            } else {
                let mut command = cli("measure", &source);
                command
                    .args(["--kind", "length", "--points", "40,180 140,180", "--out"])
                    .arg(&out);
                command
            };
            if action.ends_with("saved") {
                command.args(["--use-page-calibration", "--unit", "cm"]);
            } else {
                command.args([
                    "--calibration",
                    "40,40 140,40",
                    "--known-distance",
                    "6.5",
                    "--unit",
                    "m",
                ]);
            }
            let result = no_measurement_export(auth(command, &opening).output().unwrap());
            assert!(out.exists());
            assert_eq!(result["encrypted"], true);
            for credential in [&opening, &owner] {
                let reopened = engine.open(&out, Some(credential)).unwrap();
                assert!(!reopened.properties().can_copy);
                assert!(!reopened.properties().can_modify);
                assert!(reopened.properties().can_annotate);
                let saved = reopened.measurement_records(0).unwrap();
                assert_eq!(saved.len(), if action.starts_with("edit") { 1 } else { 2 });
                let expected_value = if action.ends_with("saved") {
                    1925.0
                } else {
                    6.5
                };
                assert!(
                    (saved.last().unwrap().measurement.value().unwrap() - expected_value).abs()
                        < 1e-8
                );
                assert_eq!(
                    reopened
                        .page_calibration(0)
                        .unwrap()
                        .unwrap()
                        .known_distance,
                    19.25
                );
            }
            assert_eq!(std::fs::read(&source).unwrap(), protected);
        }
    }
}
