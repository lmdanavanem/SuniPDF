use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::{
    measurement::{Measurement, MeasurementKind, MeasurementUnit},
    Engine, Protection,
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(command: &str, source: &Path) -> Command {
    let mut process = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    process.arg(command).arg(source);
    process
}

fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> serde_json::Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn piped(mut command: Command, password: &str) -> Output {
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

fn set(source: &Path, out: &Path, distance: &str, unit: &str) -> Command {
    let mut command = cli("set-page-calibration", source);
    command
        .args([
            "--page",
            "1",
            "--calibration",
            "40,40 140,40",
            "--known-distance",
            distance,
            "--unit",
            unit,
            "--out",
        ])
        .arg(out);
    command
}

fn measure(source: &Path, out: &Path, unit: &str) -> Command {
    let mut command = cli("measure", source);
    command
        .args([
            "--page",
            "1",
            "--kind",
            "length",
            "--points",
            "40,80 140,80",
            "--use-page-calibration",
            "--unit",
            unit,
            "--out",
        ])
        .arg(out);
    command
}

#[test]
fn persisted_calibration_roundtrips_and_only_new_measurements_use_changed_scale() {
    let engine = Engine::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("original.pdf");
    let mut document = engine.open(&fixture(), None).unwrap();
    document
        .add_measurement_annotation(
            0,
            &Measurement {
                kind: MeasurementKind::Length,
                points: vec![(40.0, 80.0), (140.0, 80.0)],
                calibration: [(40.0, 40.0), (140.0, 40.0)],
                known_distance: 10.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            },
            None,
        )
        .unwrap();
    document.save_as(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let original_records = engine.open(&source, None).unwrap().measurements(0).unwrap();

    let scaled = directory.path().join("scaled.pdf");
    let saved = success(set(&source, &scaled, "20", "m").output().unwrap());
    assert_eq!(saved["existing_measurements_changed"], false);
    assert_eq!(saved["original_unchanged"], true);
    let scales = success(cli("page-calibration", &scaled).output().unwrap());
    assert_eq!(scales["pages"].as_array().unwrap().len(), 2);
    assert_eq!(scales["pages"][0]["page"], 1);
    assert_eq!(scales["pages"][0]["calibration"]["known_distance"], 20.0);
    assert_eq!(scales["pages"][0]["calibration"]["unit"], "m");
    assert!(scales["pages"][1]["calibration"].is_null());
    assert_eq!(
        engine.open(&scaled, None).unwrap().measurements(0).unwrap(),
        original_records
    );

    let feet = directory.path().join("measured-feet.pdf");
    let added = success(measure(&scaled, &feet, "ft").output().unwrap());
    assert!((added["measurement"]["value"].as_f64().unwrap() - 20.0 / 0.3048).abs() < 1e-8);
    assert_eq!(added["measurement"]["unit"], "ft");
    let old_records = engine.open(&feet, None).unwrap().measurements(0).unwrap();
    assert_eq!(old_records.len(), 2);
    assert_eq!(old_records[0], original_records[0]);

    let rescaled = directory.path().join("rescaled-feet.pdf");
    success(set(&feet, &rescaled, "6", "ft").output().unwrap());
    assert_eq!(
        engine
            .open(&rescaled, None)
            .unwrap()
            .measurements(0)
            .unwrap(),
        old_records
    );
    let centimetres = directory.path().join("measured-centimetres.pdf");
    let added = success(measure(&rescaled, &centimetres, "cm").output().unwrap());
    assert!((added["measurement"]["value"].as_f64().unwrap() - 182.88).abs() < 1e-8);
    assert_eq!(added["measurement"]["unit"], "cm");
    let final_records = engine
        .open(&centimetres, None)
        .unwrap()
        .measurements(0)
        .unwrap();
    assert_eq!(&final_records[..2], &old_records[..]);

    let removed = directory.path().join("removed.pdf");
    let result = success(
        cli("remove-page-calibration", &centimetres)
            .args(["--page", "1", "--out"])
            .arg(&removed)
            .output()
            .unwrap(),
    );
    assert_eq!(result["existing_measurements_changed"], false);
    let read = success(
        cli("page-calibration", &removed)
            .args(["--page", "1"])
            .output()
            .unwrap(),
    );
    assert!(read["pages"][0]["calibration"].is_null());
    assert_eq!(
        engine
            .open(&removed, None)
            .unwrap()
            .measurements(0)
            .unwrap(),
        final_records
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn calibration_writes_refuse_overwrite_and_invalid_or_missing_scale_without_creating_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("original.pdf");
    std::fs::copy(fixture(), &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let existing = directory.path().join("existing.pdf");
    std::fs::write(&existing, b"keep existing bytes").unwrap();
    for out in [&source, &existing] {
        assert_eq!(
            failure(
                set(&source, out, "10", "m")
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
        assert_eq!(
            failure(
                cli("remove-page-calibration", &source)
                    .args(["--page", "1", "--overwrite", "--out"])
                    .arg(out)
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
    }
    let out = directory.path().join("must-not-exist.pdf");
    let error = failure(measure(&source, &out, "m").output().unwrap());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("no persisted calibration"));
    assert!(!out.exists());
    for points in [
        "40,40 40,40",
        "40,40 140,40 240,40",
        "NaN,40 140,40",
        "40,40 99999,40",
    ] {
        let mut command = cli("set-page-calibration", &source);
        command
            .args([
                "--page",
                "1",
                "--calibration",
                points,
                "--known-distance",
                "10",
                "--out",
            ])
            .arg(&out);
        failure(command.output().unwrap());
        assert!(!out.exists());
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep existing bytes");
}

#[test]
fn calibration_copies_keep_aes_permissions_and_support_env_and_stdin_passwords() {
    let directory = tempfile::tempdir().unwrap();
    let plain = std::fs::read(fixture()).unwrap();
    let opening = format!("scale-opening-{}", std::process::id());
    let owner = format!("scale-owner-{}", std::process::id());
    for aes256 in [false, true] {
        let protection = Protection {
            user_password: opening.clone(),
            owner_password: owner.clone(),
            allow_copy: false,
            allow_print: false,
            allow_modify: true,
            aes256,
            ..Default::default()
        };
        let bytes = sunipdf_core::security::protect(&plain, None, &protection).unwrap();
        let restricted_bytes = sunipdf_core::security::protect(
            &plain,
            None,
            &Protection {
                allow_modify: false,
                ..protection
            },
        )
        .unwrap();
        let source = directory.path().join(format!("original-{aes256}.pdf"));
        let restricted = directory.path().join(format!("restricted-{aes256}.pdf"));
        let saved = directory.path().join(format!("saved-{aes256}.pdf"));
        let removed = directory.path().join(format!("removed-{aes256}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        std::fs::write(&restricted, &restricted_bytes).unwrap();
        for credential in [&opening, &owner] {
            let mut denied = set(&restricted, &saved, "10", "m");
            denied
                .args(["--password-env", "SUNIPDF_TEST_SCALE_PASSWORD"])
                .env("SUNIPDF_TEST_SCALE_PASSWORD", credential);
            let denied = denied.output().unwrap();
            assert_eq!(denied.status.code(), Some(1));
            assert_eq!(failure(denied)["error"], "security_restricted");
            assert!(!saved.exists());
        }
        let mut allowed = set(&source, &saved, "10", "m");
        allowed
            .args(["--password-env", "SUNIPDF_TEST_SCALE_PASSWORD"])
            .env("SUNIPDF_TEST_SCALE_PASSWORD", &opening);
        assert_eq!(success(allowed.output().unwrap())["encrypted"], true);
        for credential in [&opening, &owner] {
            for command in ["page-calibration", "measurements"] {
                let mut read = cli(command, &saved);
                read.args(["--page", "1", "--password-stdin"]);
                assert_eq!(
                    failure(piped(read, credential))["error"],
                    "security_restricted"
                );
            }
        }
        let mut remove = cli("remove-page-calibration", &saved);
        remove
            .args(["--page", "1", "--password-stdin", "--out"])
            .arg(&removed);
        assert_eq!(success(piped(remove, &owner))["encrypted"], true);
        for output in [&saved, &removed] {
            assert_eq!(
                failure(cli("properties", output).output().unwrap())["error"],
                "password_required"
            );
            let mut properties = cli("properties", output);
            properties
                .args(["--password-env", "SUNIPDF_TEST_SCALE_PASSWORD"])
                .env("SUNIPDF_TEST_SCALE_PASSWORD", &opening);
            let properties = success(properties.output().unwrap());
            assert_eq!(properties["encrypted"], true);
            for permission in ["copy", "print"] {
                assert_eq!(properties["permissions"][permission], false);
            }
            assert_eq!(properties["permissions"]["modify"], true);
        }
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        assert_eq!(std::fs::read(&restricted).unwrap(), restricted_bytes);
    }
}
