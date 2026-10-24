use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}
fn ok(output: Output, code: i32) -> Value {
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
fn refused(output: Output) -> Value {
    refusal_with_code(output, 1)
}
fn collision(output: Output) {
    let error = refusal_with_code(output, 6);
    assert_eq!(error["error"], "already_exists");
}
fn refusal_with_code(output: Output, code: i32) -> Value {
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
fn fixture() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(dictionary! {}, b"0 g 5 5 5 5 re f".to_vec()));
    let mut annotations = Vec::new();
    for index in 0..3 {
        annotations.push(Object::Reference(pdf.add_object(dictionary!{
            "Type"=>"Annot","Subtype"=>"Stamp","Name"=>"SuniPDFCount","NM"=>Object::string_literal(format!("SuniPDF-cli-report-{index}")),
            "Contents"=>lopdf::text_string(if index==0{"Électricité Ω, first issue"}else{"=SUM(A1) / issue"}),"T"=>lopdf::text_string(if index==2{"Other"}else{"Reviewer É"}),
            "Rect"=>vec![30.into(),30.into(),40.into(),40.into()],"SuniPDFCount"=>Object::string_literal(br#"{"version":1,"group":"Equipment"}"#.to_vec()),
            "SuniPDFQuantity"=>Object::string_literal(br#"{"version":1,"fields":{"unit_price":{"amount":"12.5","currency":"CHF","per_unit":"each","rounding_decimals":2},"custom":{}}}"#.to_vec())
        })));
    }
    let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()],"Resources"=>dictionary!{},"Contents"=>content,"Annots"=>annotations});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn report(operation: &str, file: &Path) -> Command {
    let mut command = cli();
    command.args(["issue-report", operation]).arg(file);
    command
}
fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn selection_totals_formulas_and_actual_preview_match_exported_data() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("plan.pdf");
    let bytes = fixture();
    std::fs::write(&source, &bytes).unwrap();
    let definitions = dir.path().join("formula.json");
    std::fs::write(&definitions,br#"[{"name":"Double count","expression":"quantity * 2","quantity_unit":"each","rounding_decimals":2}]"#).unwrap();
    let inspected = ok(
        report("inspect", &source)
            .args(["--author", "Reviewer É", "--descending", "--formulas"])
            .arg(&definitions)
            .output()
            .unwrap(),
        0,
    );
    let rows = inspected["quantities"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["register"]["index"], 1);
    assert_eq!(rows[0]["line_total"]["amount"], "12.50");
    assert_eq!(
        inspected["quantities"]["formula_rows"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(inspected["page_count"].as_u64().unwrap() > 1);
    assert_eq!(
        inspected["provenance"]["saved_file_sha256"],
        sunipdf_core::plan_zones::source_sha256(&bytes)
    );
    for format in ["json", "csv", "pdf"] {
        let out = dir.path().join(format!("report.{format}"));
        let receipt = ok(
            report("export", &source)
                .args(["--author", "Reviewer É", "--descending", "--formulas"])
                .arg(&definitions)
                .args(["--format", format, "--out"])
                .arg(&out)
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(receipt["matched_annotations"], 2);
        assert_eq!(receipt["format"], format);
        assert!(out.metadata().unwrap().len() > 100);
        match format {
            "json" => {
                let model = read_json(&out);
                assert_eq!(model["quantities"]["rows"], inspected["quantities"]["rows"]);
                assert_eq!(
                    model["quantities"]["summary"],
                    inspected["quantities"]["summary"]
                );
                assert_eq!(
                    model["quantities"]["formula_rows"],
                    inspected["quantities"]["formula_rows"]
                );
            }
            "csv" => {
                let text = std::fs::read_to_string(&out).unwrap();
                assert!(text.contains("quantity_summary"));
                assert!(text.contains("12.50"));
                assert!(text.contains("'=SUM(A1)"));
                assert!(text.contains("SuniPDF-cli-report-0"));
                assert!(text.contains("SuniPDF-cli-report-1"));
                assert!(!text.contains("SuniPDF-cli-report-2"));
            }
            "pdf" => {
                let pdf = Document::load(&out).unwrap();
                assert_eq!(
                    pdf.get_pages().len(),
                    inspected["page_count"].as_u64().unwrap() as usize
                );
                assert!(!pdf.is_encrypted());
                let info = pdf.trailer.get(b"Info").unwrap().as_reference().unwrap();
                assert!(pdf.get_dictionary(info).unwrap().has(b"ReportFontLicence"));
            }
            _ => unreachable!(),
        }
    }
    let png = dir.path().join("preview.png");
    let preview = ok(
        report("preview", &source)
            .args([
                "--author",
                "Reviewer É",
                "--preview-page",
                "1",
                "--long-side",
                "256",
                "--out",
            ])
            .arg(&png)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(preview["report_published"], false);
    let image = image::open(&png).unwrap().to_rgba8();
    assert_eq!(image.width().max(image.height()), 256);
    assert!(image.pixels().any(|p| p.0 != [255, 255, 255, 255]));
    assert_eq!(std::fs::read(source).unwrap(), bytes);
}

#[test]
fn explicit_incomplete_formula_reports_keep_errors_and_exit_two() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    std::fs::write(&source, fixture()).unwrap();
    let formulas = dir.path().join("errors.json");
    std::fs::write(&formulas,br#"[{"name":"Undefined","expression":"quantity / 0","quantity_unit":"each","rounding_decimals":2}]"#).unwrap();
    let out = dir.path().join("incomplete.json");
    let receipt = ok(
        report("export", &source)
            .arg("--formulas")
            .arg(&formulas)
            .args(["--format", "json", "--out"])
            .arg(&out)
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(receipt["complete"], false);
    let model = read_json(&out);
    assert_eq!(model["quantities"]["formula_evaluation_complete"], false);
    assert_eq!(model["quantities"]["rows"].as_array().unwrap().len(), 3);
    assert_eq!(
        model["quantities"]["formula_rows"][0]["values"]["Undefined"]["error"],
        "division_by_zero"
    );
    assert!(!model["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn changed_hash_invalid_inputs_and_existing_outputs_never_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    let bytes = fixture();
    std::fs::write(&source, &bytes).unwrap();
    let out = dir.path().join("retained");
    std::fs::write(&out, b"retain").unwrap();
    for format in ["json", "csv", "pdf"] {
        collision(
            report("export", &source)
                .args(["--overwrite", "--format", format, "--out"])
                .arg(&out)
                .output()
                .unwrap(),
        );
        assert_eq!(std::fs::read(&out).unwrap(), b"retain");
    }
    collision(
        report("preview", &source)
            .args(["--overwrite", "--out"])
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&out).unwrap(), b"retain");
    let new = dir.path().join("absent.pdf");
    refused(
        report("export", &source)
            .arg("--expected-source-sha256")
            .arg("0".repeat(64))
            .arg("--out")
            .arg(&new)
            .output()
            .unwrap(),
    );
    assert!(!new.exists());
    refused(
        report("preview", &source)
            .args(["--preview-page", "999", "--out"])
            .arg(&new)
            .output()
            .unwrap(),
    );
    assert!(!new.exists());
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, b"[{\"name\":\"Wrong\",\"expression\":\"run()\"}]").unwrap();
    refused(
        report("export", &source)
            .arg("--formulas")
            .arg(&invalid)
            .arg("--out")
            .arg(&new)
            .output()
            .unwrap(),
    );
    assert!(!new.exists());
    refused(
        report("inspect", &source)
            .args(["--zone", "1:missing"])
            .output()
            .unwrap(),
    );
    let parser = report("preview", &source)
        .args(["--preview-page", "0", "--out"])
        .arg(&new)
        .output()
        .unwrap();
    assert_eq!(parser.status.code(), Some(2));
    assert!(!new.exists());
    assert_eq!(std::fs::read(source).unwrap(), bytes);
}

#[test]
fn empty_selection_and_excluded_legend_remain_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    std::fs::write(&source, fixture()).unwrap();
    let result = ok(
        report("inspect", &source)
            .args(["--query", "No such item", "--no-legend"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(result["quantities"]["rows"], json!([]));
    assert_eq!(result["legend_included"], false);
    assert_eq!(result["legend"], json!([]));
    assert_eq!(result["issue_groups"], json!([]));
    assert!(result["page_count"].as_u64().unwrap() > 0);
}

#[test]
fn protected_source_uses_ephemeral_password_input_and_copy_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().file_name().unwrap().to_string_lossy();
    let user = format!("{seed}-reader");
    let owner = format!("{seed}-owner");
    for allow_copy in [true, false] {
        let bytes = sunipdf_core::security::protect(
            &fixture(),
            None,
            &sunipdf_core::Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                allow_print: true,
                allow_copy,
                allow_modify: false,
                allow_annotate: false,
                aes256: true,
            },
        )
        .unwrap();
        let source = dir.path().join(format!("protected-{allow_copy}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        let mut child = report("inspect", &source)
            .arg("--password-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(child.stdin.take().unwrap(), "{user}").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(&user));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(&user));
        if allow_copy {
            let result = ok(output, 0);
            assert_eq!(result["provenance"]["source_encrypted"], true);
            assert_eq!(result["provenance"]["report_encrypted"], false);
        } else {
            refused(output);
        }
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }
}

#[test]
fn register_mode_keeps_legacy_text_sort_and_excludes_typed_only_matches() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    let mut pdf = Document::load_mem(&fixture()).unwrap();
    let page = pdf.get_pages()[&1];
    let annotations = pdf
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    for (index, annotation) in annotations.iter().enumerate() {
        let dict = pdf
            .get_dictionary_mut(annotation.as_reference().unwrap())
            .unwrap();
        dict.set(
            "Contents",
            lopdf::text_string(if index < 2 {
                "Visible issue"
            } else {
                "Other issue"
            }),
        );
        dict.set("SuniPDFRegister",Object::string_literal(serde_json::to_vec(&json!({"version":1,"fields":{"business_status":null,"discipline":null,"custom":{"Room":(["Z","A","M"][index])}}})).unwrap()));
        dict.set("SuniPDFQuantity",Object::string_literal(serde_json::to_vec(&json!({"version":1,"fields":{"unit_price":{"amount":"12.5","currency":"CHF","per_unit":"each","rounding_decimals":2},"custom":{"Tag":{"type":"text","value":if index==2{"visible private-tag"}else{"other"}}}}})).unwrap()));
    }
    pdf.save(&source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let register = ok(
        report("inspect", &source)
            .args([
                "--selection",
                "register",
                "--query",
                "visible",
                "--sort-custom",
                "Room",
            ])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(register["selection"], "register");
    let rows = register["quantities"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["register"]["index"], 1);
    assert_eq!(rows[1]["register"]["index"], 0);
    assert_eq!(
        register["quantities"]["summary"]["groups"][0]["costs"][0]["total"]["amount"],
        "25.00"
    );
    let typed = ok(
        report("inspect", &source)
            .args(["--selection", "quantities", "--query", "private-tag"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(typed["quantities"]["rows"].as_array().unwrap().len(), 1);
    assert_eq!(typed["quantities"]["rows"][0]["register"]["index"], 2);
    let absent = ok(
        report("inspect", &source)
            .args(["--selection", "register", "--query", "private-tag"])
            .output()
            .unwrap(),
        0,
    );
    assert!(absent["quantities"]["rows"].as_array().unwrap().is_empty());
    assert_eq!(std::fs::read(source).unwrap(), original);
}
