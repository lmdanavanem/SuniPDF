use lopdf::dictionary;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::{
    measurement::{MeasurementUnit, MeasurementViewport},
    Engine, PageCalibration, PointRect, Protection,
};

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

fn rectangle(file: &Path, out: &Path, anchor: &str, width: &str, height: &str) -> Command {
    let mut command = cli("dimensioned-rectangle", file);
    command
        .args([
            "--anchor", anchor, "--width", width, "--height", height, "--out",
        ])
        .arg(out);
    command
}

fn local_rectangle(file: &Path, out: &Path, width: &str, height: &str) -> Command {
    let mut command = rectangle(file, out, "40,80", width, height);
    command.args(["--calibration", "40,40 140,40", "--known-distance", "1"]);
    command
}

fn scale(distance: f64) -> PageCalibration {
    PageCalibration {
        points: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: distance,
        unit: MeasurementUnit::Metres,
    }
}

fn assert_area(result: &Value, expected: f64, unit: &str) {
    let record = &result["measurement"];
    assert_eq!(record["kind"], "area");
    assert_eq!(record["version"], 2);
    assert_eq!(record["validity"], "current");
    assert_eq!(record["unit"], unit);
    assert!((record["value"].as_f64().unwrap() - expected).abs() <= expected * 1e-5);
}

#[test]
fn rectangle_real_dimensions_survive_all_rotations_crop_and_quantity_reopen() {
    let directory = tempfile::tempdir().unwrap();
    for cropped in [false, true] {
        for rotation in [0_i64, 90, 180, 270] {
            let source = directory
                .path()
                .join(format!("source-{cropped}-{rotation}.pdf"));
            let out = directory
                .path()
                .join(format!("rectangle-{cropped}-{rotation}.pdf"));
            let mut pdf = lopdf::Document::load(fixture()).unwrap();
            let page = pdf.get_pages()[&1];
            let page = pdf.get_dictionary_mut(page).unwrap();
            page.set("Rotate", rotation);
            if cropped {
                page.set(
                    "CropBox",
                    vec![20.into(), 30.into(), 592.into(), 762.into()],
                );
            }
            pdf.save(&source).unwrap();
            let original = std::fs::read(&source).unwrap();
            let result = success(
                local_rectangle(&source, &out, "2", "3")
                    .args(["--author", "Plan reviewer"])
                    .output()
                    .unwrap(),
            );
            assert_area(&result, 6.0, "m2");
            assert_eq!(
                result["measurement"]["points"],
                json!([[40.0, 80.0], [240.0, 80.0], [240.0, 380.0], [40.0, 380.0]])
            );
            let inventory = success(
                cli("measurements", &out)
                    .args(["--page", "1"])
                    .output()
                    .unwrap(),
            );
            let records = inventory["pages"][0]["measurements"].as_array().unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0]["value"], 6.0);
            assert_eq!(records[0]["validity"], "current");
            assert!(records[0]["annotation_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty()));
            assert_eq!(records[0]["points"], result["measurement"]["points"]);
            let table = success(cli("quantity-table", &out).output().unwrap());
            assert_eq!(table["summary"]["quantity_rows"], 1);
            assert_eq!(table["summary"]["unverified_measurement_rows"], 0);
            assert_eq!(table["rows"][0]["quantity"]["value"], 6.0);
            assert_eq!(table["rows"][0]["quantity"]["unit"], "m2");
            assert_eq!(table["summary"]["groups"][0]["quantity"], 6.0);
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}

#[test]
fn rectangle_converts_reference_units_and_accepts_fractional_feet_inches() {
    let directory = tempfile::tempdir().unwrap();
    for (unit, width, height, expected) in [
        ("mm", "2000", "3000", 6_000_000.0),
        ("cm", "200", "300", 60_000.0),
        ("m", "2", "3", 6.0),
        ("ft", "6' 6\"", "3'", 19.5),
        ("in", "6 1/2 in", "12 in", 78.0),
    ] {
        let out = directory.path().join(format!("rectangle-{unit}.pdf"));
        let result = success(
            rectangle(&fixture(), &out, "40,80", width, height)
                .args([
                    "--calibration",
                    "40,40 140,40",
                    "--known-distance",
                    "100",
                    "--calibration-unit",
                    "cm",
                    "--unit",
                    unit,
                    "--precision",
                    "4",
                ])
                .output()
                .unwrap(),
        );
        assert_area(&result, expected, &format!("{unit}2"));
        assert_eq!(result["measurement"]["precision"], 4);
        assert_eq!(result["measurement"]["calibration"]["unit"], unit);
    }
}

#[test]
fn rotated_rectangle_reports_normalized_angle_and_preserves_dimensions_after_reopening() {
    let directory = tempfile::tempdir().unwrap();
    for rotation in [0_i64, 90, 180, 270] {
        let source = directory
            .path()
            .join(format!("rotated-source-{rotation}.pdf"));
        let mut pdf = lopdf::Document::load(fixture()).unwrap();
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Rotate", rotation);
        page.set(
            "CropBox",
            vec![20.into(), 30.into(), 592.into(), 762.into()],
        );
        pdf.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut previous_points = None;
        for (unit, width, height, angle, expected_area) in [
            ("m", "0.8", "1.1", "37.5", 0.88),
            ("cm", "80", "110", "-322.5", 8800.0),
        ] {
            let output = directory
                .path()
                .join(format!("rotated-output-{rotation}-{unit}.pdf"));
            let result = success(
                rectangle(&source, &output, "220,260", width, height)
                    .args([
                        "--calibration",
                        "40,40 140,40",
                        "--calibration-unit",
                        "m",
                        "--known-distance",
                        "1",
                        "--unit",
                        unit,
                        "--rotation-degrees",
                        angle,
                    ])
                    .output()
                    .unwrap(),
            );
            assert_eq!(result["rotation_degrees"], 37.5);
            assert_area(&result, expected_area, &format!("{unit}2"));
            assert_eq!(result["measurement"]["points"][0], json!([220.0, 260.0]));
            assert!(result["measurement"]["points"][1][1].as_f64().unwrap() < 260.0);
            if let Some(expected_points) = previous_points {
                assert_eq!(result["measurement"]["points"], expected_points);
            }
            previous_points = Some(result["measurement"]["points"].clone());
            let inventory = success(
                cli("measurements", &output)
                    .args(["--page", "1"])
                    .output()
                    .unwrap(),
            );
            let record = &inventory["pages"][0]["measurements"][0];
            assert_eq!(record["validity"], "current");
            assert!(
                (record["value"].as_f64().unwrap() - expected_area).abs() <= expected_area * 1e-5
            );
            assert_eq!(record["points"], result["measurement"]["points"]);
            let table = success(cli("quantity-table", &output).output().unwrap());
            assert_eq!(table["summary"]["quantity_rows"], 1);
            assert_eq!(table["summary"]["unverified_measurement_rows"], 0);
            assert_eq!(table["rows"][0]["quantity"]["unit"], format!("{unit}2"));
            assert!(
                (table["rows"][0]["quantity"]["value"].as_f64().unwrap() - expected_area).abs()
                    <= expected_area * 1e-5
            );
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}

#[test]
fn rectangle_rotation_refuses_nonfinite_angles_or_out_of_page_geometry_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("original.pdf");
    std::fs::copy(fixture(), &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let output = directory.path().join("must-not-exist.pdf");
    for value in ["NaN", "inf", "-inf", "1e999"] {
        let result = local_rectangle(&source, &output, "2", "3")
            .args(["--rotation-degrees", value])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!output.exists());
    }
    failure(
        rectangle(&source, &output, "0,0", "2", "3")
            .args([
                "--calibration",
                "40,40 140,40",
                "--known-distance",
                "1",
                "--rotation-degrees",
                "45",
            ])
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn rectangle_refuses_invalid_geometry_and_destinations_without_mutating_sources() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("original.pdf");
    std::fs::copy(fixture(), &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let existing = directory.path().join("existing.pdf");
    std::fs::write(&existing, b"keep this output").unwrap();
    let alias = directory.path().join("source-alias.pdf");
    std::fs::hard_link(&source, &alias).unwrap();
    for out in [&source, &existing, &alias] {
        assert_eq!(
            failure(
                local_rectangle(&source, out, "2", "3")
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
    }
    let out = directory.path().join("must-not-exist.pdf");
    for (width, height) in [
        ("0", "3"),
        ("-2", "3"),
        ("NaN", "3"),
        ("2", "inf"),
        ("999999999", "3"),
        ("0.0000000001", "3"),
        ("2", "0"),
    ] {
        failure(
            local_rectangle(&source, &out, width, height)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    for (anchor, calibration) in [
        ("600,780", "40,40 140,40"),
        ("NaN,80", "40,40 140,40"),
        ("40,80 50,90", "40,40 140,40"),
        ("40,80", "40,40 40,40"),
        ("40,80", "40,40 900,40"),
    ] {
        failure(
            rectangle(&source, &out, anchor, "2", "3")
                .args(["--calibration", calibration, "--known-distance", "1"])
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    for page in ["0", "3"] {
        failure(
            local_rectangle(&source, &out, "2", "3")
                .args(["--page", page])
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&alias).unwrap(), original);
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep this output");
}

#[test]
fn rectangle_scale_selection_refuses_overlap_crossing_enclosure_missing_and_stale_scales() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::new().unwrap();
    let source = directory.path().join("regions.pdf");
    let mut doc = engine.open(&fixture(), None).unwrap();
    doc.set_page_calibration(0, Some(&scale(1.0))).unwrap();
    let region = MeasurementViewport {
        id: "detail-a".into(),
        name: "Detail A".into(),
        bounds: PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 300.0,
            top: 400.0,
        },
        calibration: scale(2.0),
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&region))
        .unwrap();
    doc.save_as(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    for (name, selection, expected_x) in [
        ("page", vec!["--use-page-calibration"], 240.0),
        ("region", vec!["--region", "detail-a"], 140.0),
        ("automatic", vec!["--automatic-scale"], 140.0),
    ] {
        let out = directory.path().join(format!("selected-{name}.pdf"));
        let result = success(
            rectangle(&source, &out, "40,80", "2", "3")
                .args(selection)
                .output()
                .unwrap(),
        );
        assert_area(&result, 6.0, "m2");
        assert_eq!(result["measurement"]["points"][1][0], expected_x);
    }
    let outside = directory.path().join("automatic-page-fallback.pdf");
    let fallback = success(
        rectangle(&source, &outside, "350,450", "1", "1")
            .arg("--automatic-scale")
            .output()
            .unwrap(),
    );
    assert_area(&fallback, 1.0, "m2");
    assert_eq!(fallback["measurement"]["points"][1][0], 450.0);
    let out = directory.path().join("must-not-exist.pdf");
    for (anchor, width, height, selection) in [
        ("40,80", "2", "3", vec!["--region", "missing"]),
        ("280,80", "2", "3", vec!["--region", "detail-a"]),
        ("280,80", "2", "3", vec!["--automatic-scale"]),
        ("10,10", "4", "5", vec!["--automatic-scale"]),
    ] {
        failure(
            rectangle(&source, &out, anchor, width, height)
                .args(selection)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    let crossed = directory.path().join("explicit-page-crossing.pdf");
    assert_area(
        &success(
            rectangle(&source, &crossed, "280,80", "2", "3")
                .arg("--use-page-calibration")
                .output()
                .unwrap(),
        ),
        6.0,
        "m2",
    );

    let overlapping = directory.path().join("overlap.pdf");
    let second = MeasurementViewport {
        id: "detail-b".into(),
        name: "Detail B".into(),
        ..region.clone()
    };
    doc.set_measurement_viewports(0, std::slice::from_ref(&region), &[region.clone(), second])
        .unwrap();
    doc.save_as(&overlapping).unwrap();
    failure(
        rectangle(&overlapping, &out, "40,80", "2", "3")
            .arg("--automatic-scale")
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
    let explicit_overlap = directory.path().join("selected-overlap.pdf");
    assert_area(
        &success(
            rectangle(&overlapping, &explicit_overlap, "40,80", "2", "3")
                .args(["--region", "detail-a"])
                .output()
                .unwrap(),
        ),
        6.0,
        "m2",
    );
    for selection in [
        vec!["--use-page-calibration"],
        vec!["--automatic-scale"],
        vec!["--region", "detail-a"],
    ] {
        failure(
            rectangle(&fixture(), &out, "40,80", "2", "3")
                .args(selection)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }

    let stale = directory.path().join("stale.pdf");
    let mut pdf = lopdf::Document::load(&source).unwrap();
    let page = pdf.get_pages()[&1];
    let stream = pdf.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"q 1 0 0 RG 20 20 10 10 re S Q".to_vec(),
    ));
    pdf.get_dictionary_mut(page)
        .unwrap()
        .set("Contents", stream);
    pdf.save(&stale).unwrap();
    for selection in [
        vec!["--use-page-calibration"],
        vec!["--automatic-scale"],
        vec!["--region", "detail-a"],
    ] {
        failure(
            rectangle(&stale, &out, "40,80", "2", "3")
                .args(selection)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

fn with_password(mut command: Command, password: &str) -> Command {
    command
        .args(["--password-env", "SUNIPDF_TEST_RECTANGLE_PASSWORD"])
        .env("SUNIPDF_TEST_RECTANGLE_PASSWORD", password);
    command
}

#[test]
fn rectangle_creation_preserves_foreign_measurements_without_exporting_their_inventory() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("foreign.pdf");
    let out = directory.path().join("rectangle.pdf");
    let mut pdf = lopdf::Document::load(fixture()).unwrap();
    let page = pdf.get_pages()[&1];
    let annotation = pdf.add_object(lopdf::dictionary! {
        "Type" => "Annot", "Subtype" => "Line",
        "Rect" => vec![50.into(), 500.into(), 150.into(), 510.into()],
        "L" => vec![50.into(), 500.into(), 150.into(), 500.into()],
        "F" => 4, "Measure" => lopdf::dictionary! {"Type" => "Measure", "Subtype" => "RL"},
    });
    pdf.get_dictionary_mut(page)
        .unwrap()
        .set("Annots", vec![annotation.into()]);
    pdf.save(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let result = success(local_rectangle(&source, &out, "2", "3").output().unwrap());
    assert_area(&result, 6.0, "m2");
    assert_eq!(result["index"], 1);
    let error = failure(cli("measurements", &out).output().unwrap());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("imported measurement metadata is unsupported"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn rectangle_refuses_non_default_page_user_unit_without_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("scaled-user-space.pdf");
    let out = directory.path().join("must-not-exist.pdf");
    let mut pdf = lopdf::Document::load(fixture()).unwrap();
    let page = pdf.get_pages()[&1];
    pdf.get_dictionary_mut(page).unwrap().set("UserUnit", 2);
    pdf.save(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    failure(local_rectangle(&source, &out, "2", "3").output().unwrap());
    assert!(!out.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

fn piped(mut command: Command, password: &str) -> Output {
    let mut child = command
        .arg("--password-stdin")
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
fn rectangle_preserves_aes_permissions_and_suppresses_copy_restricted_exports() {
    let directory = tempfile::tempdir().unwrap();
    let plain = std::fs::read(fixture()).unwrap();
    let owner = format!("rectangle-owner-{}", std::process::id());
    for aes256 in [false, true] {
        for empty_user in [false, true] {
            let opening = if empty_user {
                String::new()
            } else {
                format!("rectangle-user-{}", std::process::id())
            };
            let protection = Protection {
                user_password: opening.clone(),
                owner_password: owner.clone(),
                allow_copy: false,
                allow_print: false,
                allow_modify: false,
                allow_annotate: true,
                aes256,
            };
            let source = directory
                .path()
                .join(format!("protected-{aes256}-{empty_user}.pdf"));
            let out = directory
                .path()
                .join(format!("rectangle-{aes256}-{empty_user}.pdf"));
            let bytes = sunipdf_core::security::protect(&plain, None, &protection).unwrap();
            std::fs::write(&source, &bytes).unwrap();
            let output = piped(local_rectangle(&source, &out, "2", "3"), &opening);
            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
            let result = success(output);
            assert_eq!(result["encrypted"], true);
            assert_eq!(result["export_unavailable"], true);
            assert!(result["measurement"].is_null());
            for field in [
                "points",
                "calibration",
                "known_distance",
                "value",
                "unit",
                "label",
            ] {
                assert!(!stdout.contains(&format!("\"{field}\"")));
            }
            for credential in [&opening, &owner] {
                let props = success(
                    with_password(cli("properties", &out), credential)
                        .output()
                        .unwrap(),
                );
                assert_eq!(props["encrypted"], true);
                for permission in ["copy", "print", "modify"] {
                    assert_eq!(props["permissions"][permission], false);
                }
                assert_eq!(props["permissions"]["annotate"], true);
                for action in ["measurements", "quantity-table"] {
                    assert_eq!(
                        failure(
                            with_password(cli(action, &out), credential)
                                .output()
                                .unwrap()
                        )["error"],
                        "security_restricted"
                    );
                }
            }
            let blocked = directory
                .path()
                .join(format!("blocked-{aes256}-{empty_user}.pdf"));
            let blocked_out = directory
                .path()
                .join(format!("blocked-out-{aes256}-{empty_user}.pdf"));
            let denied_bytes = sunipdf_core::security::protect(
                &plain,
                None,
                &Protection {
                    allow_annotate: false,
                    ..protection
                },
            )
            .unwrap();
            std::fs::write(&blocked, &denied_bytes).unwrap();
            for credential in [&opening, &owner] {
                assert_eq!(
                    failure(
                        with_password(
                            local_rectangle(&blocked, &blocked_out, "2", "3"),
                            credential
                        )
                        .output()
                        .unwrap()
                    )["error"],
                    "security_restricted"
                );
                assert!(!blocked_out.exists());
            }
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
            assert_eq!(std::fs::read(&blocked).unwrap(), denied_bytes);
        }
    }
}
