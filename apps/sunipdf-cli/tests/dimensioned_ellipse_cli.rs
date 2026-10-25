use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::{security, Protection};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(action: &str, file: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    cmd.arg(action).arg(file);
    cmd
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

fn shape(file: &Path, out: &Path, circle: bool, dx: &str, dy: &str) -> Command {
    let mut cmd = cli(
        if circle {
            "dimensioned-circle"
        } else {
            "dimensioned-ellipse"
        },
        file,
    );
    cmd.args(["--center", "220.125,260.375", "--out"]).arg(out);
    if circle {
        cmd.args(["--diameter", dx]);
    } else {
        cmd.args(["--diameter-x", dx, "--diameter-y", dy]);
    }
    cmd
}

fn local(file: &Path, out: &Path, circle: bool, dx: &str, dy: &str) -> Command {
    let mut cmd = shape(file, out, circle, dx, dy);
    cmd.args([
        "--calibration",
        "40.25,40.5 140.25,40.5",
        "--known-distance",
        "100",
        "--calibration-unit",
        "cm",
    ]);
    cmd
}

fn expected_area(dx: f64, dy: f64) -> f64 {
    16.0 * dx * dy * (std::f64::consts::TAU / 128.0).sin()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 2e-5,
        "{actual} != {expected}"
    );
}

#[test]
fn saved_circle_and_oriented_ellipse_keep_crop_rotation_geometry_and_quantity() {
    let directory = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let source = directory.path().join(format!("source-{rotation}.pdf"));
        let mut pdf = lopdf::Document::load(fixture()).unwrap();
        let page = pdf.get_pages()[&1];
        let dict = pdf.get_dictionary_mut(page).unwrap();
        dict.set(
            "MediaBox",
            vec![0.into(), 0.into(), 900.into(), 1000.into()],
        );
        dict.set(
            "CropBox",
            vec![100.25.into(), 50.5.into(), 800.25.into(), 950.5.into()],
        );
        dict.set("Rotate", rotation);
        pdf.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        for circle in [false, true] {
            let out = directory
                .path()
                .join(format!("shape-{rotation}-{circle}.pdf"));
            let result = success(
                local(&source, &out, circle, "0.8", "1.1")
                    .args(["--rotation-degrees", "393.5", "--precision", "4"])
                    .output()
                    .unwrap(),
            );
            assert_eq!(result["segments"], 128);
            assert_eq!(result["approximation"], "polygonal_area");
            assert_eq!(result["persistent_shape_constraint"], false);
            assert_eq!(result["rotation_degrees"], 33.5);
            assert_eq!(result["shape"], if circle { "circle" } else { "ellipse" });
            let measurement = &result["measurement"];
            assert_eq!(measurement["kind"], "area");
            assert_eq!(measurement["unit"], "m2");
            assert_eq!(measurement["validity"], "current");
            assert_eq!(measurement["points"].as_array().unwrap().len(), 128);
            close(
                measurement["value"].as_f64().unwrap(),
                expected_area(0.8, if circle { 0.8 } else { 1.1 }),
            );
            let records = success(
                cli("measurements", &out)
                    .args(["--page", "1"])
                    .output()
                    .unwrap(),
            );
            let saved = &records["pages"][0]["measurements"][0];
            assert_eq!(saved["points"], measurement["points"]);
            assert_eq!(saved["value"], measurement["value"]);
            let table = success(cli("quantity-table", &out).output().unwrap());
            assert_eq!(table["summary"]["quantity_rows"], 1);
            assert_eq!(table["rows"][0]["quantity"]["value"], saved["value"]);
            // Inspect the actual PDF metadata and drawing commands, independently of JSON output.
            let pdf = lopdf::Document::load(&out).unwrap();
            let page = pdf.get_dictionary(pdf.get_pages()[&1]).unwrap();
            let annots = pdf
                .dereference(page.get(b"Annots").unwrap())
                .unwrap()
                .1
                .as_array()
                .unwrap();
            assert_eq!(annots.len(), 1);
            let annotation = pdf.dereference(&annots[0]).unwrap().1.as_dict().unwrap();
            assert_eq!(
                annotation.get(b"Subtype").unwrap().as_name().unwrap(),
                b"Stamp"
            );
            let metadata: Value = serde_json::from_slice(
                annotation
                    .get(b"SuniPDFMeasurement")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(metadata["points"].as_array().unwrap().len(), 128);
            let ap = pdf
                .dereference(annotation.get(b"AP").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            let stream = pdf
                .dereference(ap.get(b"N").unwrap())
                .unwrap()
                .1
                .as_stream()
                .unwrap();
            let content = stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone());
            let drawing = String::from_utf8(content).unwrap();
            assert_eq!(
                drawing.lines().filter(|line| line.ends_with(" l")).count(),
                127
            );
            assert!(drawing.contains("h\nS\n"));
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}

#[test]
fn ellipse_accepts_mixed_reference_units_and_literal_feet_inches() {
    let directory = tempfile::tempdir().unwrap();
    for (unit, dx, dy, x, y) in [
        ("mm", "800", "1100", 800.0, 1100.0),
        ("cm", "80", "110", 80.0, 110.0),
        ("m", "0.8", "1.1", 0.8, 1.1),
        ("ft", "6' 6\"", "3'", 6.5, 3.0),
        ("in", "6 1/2 in", "12 in", 6.5, 12.0),
    ] {
        let out = directory.path().join(format!("ellipse-{unit}.pdf"));
        let result = success(
            local(&fixture(), &out, false, dx, dy)
                .args(["--unit", unit])
                .output()
                .unwrap(),
        );
        close(
            result["measurement"]["value"].as_f64().unwrap(),
            expected_area(x, y),
        );
        assert_eq!(result["measurement"]["unit"], format!("{unit}2"));
        assert_eq!(result["measurement"]["calibration"]["unit"], unit);
    }
}

#[test]
fn ellipse_refuses_invalid_geometry_missing_scales_and_existing_destinations() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let original = std::fs::read(fixture()).unwrap();
    std::fs::write(&source, &original).unwrap();
    for (index, dx, dy) in [
        (0, "0", "1"),
        (1, "-1", "1"),
        (2, "NaN", "1"),
        (3, "1", "inf"),
        (4, "0.001", "0.001"),
        (5, "1", "0.0000001"),
        (6, "10000", "10000"),
    ] {
        let out = directory.path().join(format!("rejected-{index}.pdf"));
        failure(local(&source, &out, false, dx, dy).output().unwrap());
        assert!(!out.exists());
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
    for scale in [
        vec!["--use-page-calibration"],
        vec!["--automatic-scale"],
        vec!["--region", "missing"],
    ] {
        let out = directory.path().join("missing-scale.pdf");
        failure(
            shape(&source, &out, true, "1", "1")
                .args(scale)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    let sentinel = directory.path().join("existing.pdf");
    std::fs::write(&sentinel, b"preserve existing output").unwrap();
    assert_eq!(
        failure(
            local(&source, &sentinel, true, "1", "1")
                .arg("--overwrite")
                .output()
                .unwrap()
        )["error"],
        "already_exists"
    );
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"preserve existing output"
    );
    failure(
        local(&source, &source, false, "1", "2")
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    let alias = directory.path().join("alias.pdf");
    std::fs::hard_link(&source, &alias).unwrap();
    failure(
        local(&source, &alias, true, "1", "1")
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&alias).unwrap(), original);
}

#[test]
fn ellipse_selects_saved_scales_explicitly_and_refuses_changed_page_content() {
    let directory = tempfile::tempdir().unwrap();
    let page_source = directory.path().join("page-scale.pdf");
    success(
        cli("set-page-calibration", &fixture())
            .args([
                "--page",
                "1",
                "--calibration",
                "40.25,40.5 140.25,40.5",
                "--known-distance",
                "100",
                "--unit",
                "cm",
                "--out",
            ])
            .arg(&page_source)
            .output()
            .unwrap(),
    );
    let expected = directory.path().join("expected.json");
    let regions = directory.path().join("regions.json");
    std::fs::write(&expected, b"[]").unwrap();
    std::fs::write(
        &regions,
        serde_json::to_vec(&serde_json::json!([{
            "id":"detail", "name":"Detail", "bounds":[180,220,270,310],
            "calibration":{"points":[[190,230],[240,230]],"known_distance":1,"unit":"m"}
        }]))
        .unwrap(),
    )
    .unwrap();
    let source = directory.path().join("regions.pdf");
    success(
        cli("set-measurement-viewports", &page_source)
            .arg("--expected")
            .arg(&expected)
            .arg("--regions")
            .arg(&regions)
            .arg("--out")
            .arg(&source)
            .output()
            .unwrap(),
    );
    let original = std::fs::read(&source).unwrap();
    for (name, flags, first_x) in [
        ("page", vec!["--use-page-calibration"], 260.125),
        ("region", vec!["--region", "detail"], 240.125),
        ("automatic", vec!["--automatic-scale"], 240.125),
    ] {
        let out = directory.path().join(format!("{name}.pdf"));
        let result = success(
            shape(&source, &out, true, "0.8", "0.8")
                .args(flags)
                .output()
                .unwrap(),
        );
        assert_eq!(result["measurement"]["points"][0][0], first_x);
        close(
            result["measurement"]["value"].as_f64().unwrap(),
            expected_area(0.8, 0.8),
        );
    }
    for flags in [vec!["--region", "detail"], vec!["--automatic-scale"]] {
        let out = directory.path().join("crossing.pdf");
        failure(
            shape(&source, &out, false, "5", "1.1")
                .args(flags)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let mut pdf = lopdf::Document::load_mem(&original).unwrap();
    let page = pdf.get_pages()[&1];
    let extra = pdf.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"q 1 0 0 RG 20 20 m 60 60 l S Q\n".to_vec(),
    ));
    let page = pdf.get_dictionary_mut(page).unwrap();
    let mut contents = match page.get(b"Contents").unwrap().clone() {
        lopdf::Object::Array(items) => items,
        item => vec![item],
    };
    contents.push(lopdf::Object::Reference(extra));
    page.set("Contents", contents);
    let changed = directory.path().join("changed.pdf");
    pdf.save(&changed).unwrap();
    let changed_bytes = std::fs::read(&changed).unwrap();
    for flags in [
        vec!["--use-page-calibration"],
        vec!["--region", "detail"],
        vec!["--automatic-scale"],
    ] {
        let out = directory.path().join("stale.pdf");
        failure(
            shape(&changed, &out, true, "0.8", "0.8")
                .args(flags)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
        assert_eq!(std::fs::read(&changed).unwrap(), changed_bytes);
    }
}

fn piped(mut cmd: Command, password: &str) -> Output {
    let mut child = cmd
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
fn ellipse_preserves_protection_and_suppresses_copy_restricted_measurements() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture()).unwrap();
    for aes256 in [false, true] {
        let user = format!("ellipse-user-{}", std::process::id());
        let owner = format!("ellipse-owner-{}", std::process::id());
        for allow_annotate in [false, true] {
            let source = directory
                .path()
                .join(format!("source-{aes256}-{allow_annotate}.pdf"));
            let out = directory
                .path()
                .join(format!("ellipse-{aes256}-{allow_annotate}.pdf"));
            let protection = Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                aes256,
                allow_copy: false,
                allow_modify: false,
                allow_annotate,
                ..Default::default()
            };
            let protected = security::protect(&original, None, &protection).unwrap();
            std::fs::write(&source, &protected).unwrap();
            let output = piped(local(&source, &out, false, "0.8", "1.1"), &user);
            if allow_annotate {
                let text = String::from_utf8(output.stdout.clone()).unwrap();
                let result = success(output);
                assert_eq!(result["encrypted"], true);
                assert_eq!(result["export_unavailable"], true);
                assert!(result["measurement"].is_null());
                assert!(!text.contains(&user));
                assert!(!text.contains(&owner));
                for field in [
                    "points",
                    "calibration",
                    "known_distance",
                    "value",
                    "unit",
                    "label",
                ] {
                    assert!(!text.contains(&format!("\"{field}\"")));
                }
                let props = success(piped(cli("properties", &out), &user));
                assert_eq!(props["encrypted"], true);
                assert_eq!(props["permissions"]["annotate"], true);
                assert_eq!(props["permissions"]["copy"], false);
                assert_eq!(props["permissions"]["modify"], false);
                assert_eq!(
                    failure(piped(cli("measurements", &out), &user))["error"],
                    "security_restricted"
                );
            } else {
                assert_eq!(failure(output)["error"], "security_restricted");
                assert!(!out.exists());
            }
            assert_eq!(std::fs::read(&source).unwrap(), protected);
        }
    }
}
