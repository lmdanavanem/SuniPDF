use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}
fn workbook(action: &str, path: &Path) -> Command {
    let mut command = cli();
    command.args(["quantity-workbook", action]).arg(path);
    command
}
fn success(output: Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn refusal(output: Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}
fn fixture(rows: &[(&str, &str, usize)]) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut children = Vec::new();
    for physical_page in 1..=2 {
        let content = pdf.add_object(Stream::new(
            dictionary! {},
            b"0 g 10 10 10 10 re f".to_vec(),
        ));
        let mut annotations = Vec::new();
        for (id, contents, page) in rows {
            if physical_page != *page {
                continue;
            }
            let item = dictionary! {"Type"=>"Annot","Subtype"=>"Stamp","NM"=>Object::string_literal(*id),"Contents"=>lopdf::text_string(contents),"Rect"=>vec![30.into(),30.into(),42.into(),42.into()],"SuniPDFCount"=>Object::string_literal(br#"{"version":1,"group":"Equipment"}"#.to_vec())};
            annotations.push(Object::Reference(pdf.add_object(item)));
        }
        children.push(Object::Reference(pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()],"Resources"=>dictionary!{},"Contents"=>content,"Annots"=>annotations})));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Count"=>2,"Kids"=>children}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn json_file(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn defaults_settings_preview_and_both_exports_share_current_quantities() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.pdf");
    let original = fixture(&[
        ("SuniPDF-A", "=SUM(A1) Étage 日本語", 1),
        ("SuniPDF-B", "Second", 2),
    ]);
    std::fs::write(&input, &original).unwrap();
    let mut options = success(
        cli()
            .args(["quantity-workbook", "defaults"])
            .output()
            .unwrap(),
        0,
    );
    options["quantities"]["formulas"] = json!([{"name":"Double","expression":"quantity * 2","quantity_unit":"each","rounding_decimals":2}]);
    let settings = dir.path().join("settings.json");
    std::fs::write(&settings, serde_json::to_vec(&options).unwrap()).unwrap();
    let model = success(
        workbook("inspect", &input)
            .arg("--settings")
            .arg(&settings)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(model["quantities"]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(
        model["quantities"]["summary"]["formula_groups"][0]["totals"][0]["amount"],
        "4.00"
    );
    assert_eq!(
        model["provenance"]["snapshot_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        model["provenance"]["saved_file_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    let mut manifest = None;
    for (format, extension) in [("xlsx", "xlsx"), ("manifest", "json")] {
        let out = dir.path().join(format!("report.{extension}"));
        let receipt = success(
            workbook("export", &input)
                .arg("--settings")
                .arg(&settings)
                .args(["--format", format, "--out"])
                .arg(&out)
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(receipt["matched_rows"], 2);
        assert_eq!(receipt["source_unchanged"], true);
        assert_eq!(receipt["output_encrypted"], false);
        if format == "xlsx" {
            assert!(std::fs::read(&out).unwrap().starts_with(b"PK\x03\x04"));
        } else {
            manifest = Some(json_file(&out));
        }
    }
    let manifest = manifest.unwrap();
    assert_eq!(manifest["rows"][0]["row"], model["quantities"]["rows"][0]);
    assert_eq!(manifest["options"], options);
    assert_eq!(std::fs::read(input).unwrap(), original);
}

#[test]
fn refresh_is_explicit_and_unresolved_ids_survive_a_second_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.pdf");
    std::fs::write(
        &old,
        fixture(&[
            ("SuniPDF-A", "Original", 1),
            ("SuniPDF-B", "Missing later", 1),
        ]),
    )
    .unwrap();
    let manifest = dir.path().join("baseline.json");
    success(
        workbook("export", &old)
            .args(["--format", "manifest", "--out"])
            .arg(&manifest)
            .output()
            .unwrap(),
        0,
    );
    let current = dir.path().join("current.pdf");
    std::fs::write(
        &current,
        fixture(&[("SuniPDF-A", "Original", 2), ("SuniPDF-C", "New", 1)]),
    )
    .unwrap();
    let preview = success(
        workbook("inspect", &current)
            .arg("--previous")
            .arg(&manifest)
            .output()
            .unwrap(),
        2,
    );
    let status = |value: &Value, id: &str| {
        value["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["annotation_id"] == id)
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(status(&preview, "SuniPDF-A"), "unchanged");
    assert_eq!(status(&preview, "SuniPDF-B"), "missing");
    assert_eq!(status(&preview, "SuniPDF-C"), "excluded_new");
    assert_eq!(preview["quantities"]["rows"].as_array().unwrap().len(), 1);
    let next = dir.path().join("next.json");
    success(
        workbook("export", &current)
            .arg("--previous")
            .arg(&manifest)
            .args(["--format", "manifest", "--out"])
            .arg(&next)
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(
        json_file(&next)["pending_rows"].as_array().unwrap().len(),
        1
    );
    let included = success(
        workbook("inspect", &current)
            .arg("--previous")
            .arg(&next)
            .arg("--include-new")
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(status(&included, "SuniPDF-B"), "missing");
    assert_eq!(status(&included, "SuniPDF-C"), "added");
    assert_eq!(included["quantities"]["rows"].as_array().unwrap().len(), 2);
}

#[test]
fn malformed_configuration_changed_sources_and_collisions_do_not_write_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    let original = fixture(&[("SuniPDF-A", "Count", 1)]);
    std::fs::write(&source, &original).unwrap();
    let out = dir.path().join("kept.xlsx");
    std::fs::write(&out, b"retain").unwrap();
    let error = refusal(
        workbook("export", &source)
            .args(["--overwrite", "--out"])
            .arg(&out)
            .output()
            .unwrap(),
        6,
    );
    assert_eq!(error["error"], "already_exists");
    assert_eq!(std::fs::read(&out).unwrap(), b"retain");
    let absent = dir.path().join("absent.xlsx");
    refusal(
        workbook("export", &source)
            .arg("--expected-source-sha256")
            .arg("0".repeat(64))
            .arg("--out")
            .arg(&absent)
            .output()
            .unwrap(),
        1,
    );
    assert!(!absent.exists());
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, b"{\"unknown\":1}").unwrap();
    for flag in ["--settings", "--previous"] {
        refusal(
            workbook("export", &source)
                .arg(flag)
                .arg(&invalid)
                .arg("--out")
                .arg(&absent)
                .output()
                .unwrap(),
            1,
        );
        assert!(!absent.exists());
    }
    let requires = workbook("inspect", &source)
        .arg("--include-new")
        .output()
        .unwrap();
    assert_eq!(requires.status.code(), Some(2));
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn copy_permission_and_formula_failure_are_reported_without_exposing_passwords() {
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().file_name().unwrap().to_string_lossy();
    let user = format!("{seed}-reader");
    let owner = format!("{seed}-owner");
    for allow_copy in [true, false] {
        let source = dir.path().join(format!("protected-{allow_copy}.pdf"));
        let bytes = sunipdf_core::security::protect(
            &fixture(&[("SuniPDF-A", "Count", 1)]),
            None,
            &sunipdf_core::Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                allow_print: true,
                allow_modify: false,
                allow_copy,
                allow_annotate: false,
                aes256: true,
            },
        )
        .unwrap();
        std::fs::write(&source, &bytes).unwrap();
        let output = workbook("inspect", &source)
            .args(["--password-env", "SUNIPDF_TEST_WORKBOOK_PASSWORD"])
            .env("SUNIPDF_TEST_WORKBOOK_PASSWORD", &user)
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(&user));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(&user));
        if allow_copy {
            assert_eq!(success(output, 0)["provenance"]["source_encrypted"], true);
        } else {
            refusal(output, 1);
        }
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }
    let plain = dir.path().join("plain.pdf");
    std::fs::write(&plain, fixture(&[("SuniPDF-A", "Count", 1)])).unwrap();
    let mut options = success(
        cli()
            .args(["quantity-workbook", "defaults"])
            .output()
            .unwrap(),
        0,
    );
    options["quantities"]["formulas"] = json!([{"name":"Undefined","expression":"quantity / 0","quantity_unit":"each","rounding_decimals":2}]);
    let settings = dir.path().join("settings.json");
    std::fs::write(&settings, serde_json::to_vec(&options).unwrap()).unwrap();
    let incomplete = success(
        workbook("inspect", &plain)
            .arg("--settings")
            .arg(&settings)
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(
        incomplete["quantities"]["formula_rows"][0]["values"]["Undefined"]["error"],
        "division_by_zero"
    );
    assert_eq!(
        incomplete["quantities"]["summary"]["formula_groups"][0]["totals"],
        json!([])
    );
}
