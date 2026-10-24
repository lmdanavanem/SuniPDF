use lopdf::{dictionary, Document as Pdf, Object, Stream};
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::issue_report::*;
use sunipdf_core::quantity_table::{DecimalValue, QuantityFields, QuantityUnit, UnitPrice};
use sunipdf_core::{security, Engine, EngineError, Protection};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// A single retained PDFium instance is used only while holding SERIAL.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}
struct SharedEngine {
    _guard: MutexGuard<'static, ()>,
    engine: &'static Engine,
}
impl std::ops::Deref for SharedEngine {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        self.engine
    }
}
fn engine() -> SharedEngine {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = ENGINE.get_or_init(|| {
        EnginePtr(Box::leak(Box::new(
            Engine::new().expect("PDFium is required"),
        )))
    });
    SharedEngine {
        _guard: guard,
        engine: unsafe { &*engine.0 },
    }
}
fn fixture(count: usize) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(dictionary! {}, b"0 g 5 5 5 5 re f".to_vec()));
    let mut annotations = Vec::new();
    for i in 0..count {
        let item = dictionary! {"Type"=>"Annot","Subtype"=>"Stamp","Name"=>"SuniPDFCount","NM"=>Object::string_literal(format!("SuniPDF-count-{i}")),"Contents"=>lopdf::text_string(if i==0{"Électricité Ω / first issue"}else{"=SUM(A1) / second issue"}),"T"=>lopdf::text_string("Reviewer É"),"Rect"=>vec![30.into(),30.into(),40.into(),40.into()],"SuniPDFCount"=>Object::string_literal(br#"{"version":1,"group":"Equipment"}"#.to_vec())};
        annotations.push(Object::Reference(pdf.add_object(item)));
    }
    annotations.push(Object::Reference(pdf.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Square","NM"=>Object::string_literal("SuniPDF-shape-real"),"Rect"=>vec![100.into(),100.into(),150.into(),150.into()],"C"=>vec![Object::Real(0.2),Object::Real(0.4),Object::Real(0.6)],"IC"=>vec![Object::Real(0.8),Object::Real(0.7),Object::Real(0.6)],"BS"=>dictionary!{"W"=>2,"S"=>"D","D"=>vec![4.into(),2.into()]},"CA"=>Object::Real(0.5)})));
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
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut cell)),
            '\r' if !quoted && chars.peek() == Some(&'\n') => {
                chars.next();
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            c => cell.push(c),
        }
    }
    assert!(!quoted);
    assert!(cell.is_empty() && row.is_empty());
    rows
}

#[test]
fn one_model_keeps_selected_rows_prices_formulas_and_all_three_formats_identical() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("plan.pdf");
    std::fs::write(&source_path, fixture(2)).unwrap();
    let (binding, bytes) = SavedReportSource::capture(&source_path, None, &cancel).unwrap();
    let original = bytes.clone();
    let mut doc = engine.open_bytes(bytes, None, &source_path).unwrap();
    let table = doc.quantity_table().unwrap();
    let target = table.target(&table.rows()[0]).unwrap();
    doc.set_quantity_fields(
        &target,
        &QuantityFields {
            unit_price: Some(UnitPrice {
                amount: DecimalValue::parse("12.5").unwrap(),
                currency: "CHF".into(),
                per_unit: QuantityUnit::Each,
                rounding_decimals: 2,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    // The draft has changed, so binding it to the old saved file must fail.
    let table = doc.quantity_table().unwrap();
    assert!(doc
        .prepare_issue_report(
            &table,
            &IssueReportOptions::default(),
            Some(&binding),
            &cancel,
            |_| {}
        )
        .is_err());
    let mut options = IssueReportOptions {
        title: "Électricité Ω".into(),
        ..Default::default()
    };
    options.quantities.filter.kind = "Stamp".into();
    options.quantities.formulas=sunipdf_core::quantity_table::QuantityOptions::formulas_from_json(br#"[{"name":"Double count","expression":"quantity * 2","quantity_unit":"each","rounding_decimals":2}]"#).unwrap();
    let expected = doc
        .export_quantity_table(&table, &options.quantities)
        .unwrap()
        .json;
    let prepared = doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .unwrap();
    assert_eq!(prepared.model().quantities, expected);
    assert_eq!(
        prepared.model().quantities["rows"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        prepared.model().quantities["rows"][0]["line_total"]["amount"],
        "12.50"
    );
    assert!(prepared.model().provenance.saved_file_sha256.is_none());
    assert_eq!(
        prepared
            .model()
            .legend
            .iter()
            .map(|l| l.count)
            .sum::<usize>(),
        2
    );
    let json_path = dir.path().join("report.json");
    let csv_path = dir.path().join("report.csv");
    let pdf_path = dir.path().join("report.pdf");
    for (format, path) in [
        (IssueReportFormat::Json, &json_path),
        (IssueReportFormat::Csv, &csv_path),
        (IssueReportFormat::Pdf, &pdf_path),
    ] {
        let receipt = doc
            .publish_issue_report(&prepared, format, path, &cancel)
            .unwrap();
        assert_eq!(receipt.matched_annotations, 2);
        assert!(!receipt.provenance.report_encrypted);
    }
    let saved: Value = serde_json::from_slice(&std::fs::read(json_path).unwrap()).unwrap();
    assert_eq!(saved, prepared.model().json());
    let csv = parse_csv(&std::fs::read_to_string(csv_path).unwrap());
    let rows = csv
        .iter()
        .filter(|r| r[0] == "annotation")
        .map(|r| serde_json::from_str::<Value>(&r[24]).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(Value::Array(rows), expected["rows"]);
    let summary = csv.iter().find(|r| r[0] == "quantity_summary").unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&summary[24]).unwrap(),
        expected["summary"]
    );
    assert!(csv
        .iter()
        .any(|r| r.get(10).is_some_and(|s| s.starts_with("'=SUM"))));
    let report = engine.open(&pdf_path, None).unwrap();
    let raw_report = Pdf::load(&pdf_path).unwrap();
    let font_programs = raw_report
        .objects
        .values()
        .filter_map(|object| object.as_stream().ok())
        .map(|stream| {
            stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone())
        })
        .collect::<Vec<_>>();
    assert!(font_programs
        .iter()
        .any(|bytes| bytes.as_slice() == include_bytes!("fixtures/fonts/Ubuntu-Light.ttf")));
    let info = raw_report
        .trailer
        .get(b"Info")
        .unwrap()
        .as_reference()
        .unwrap();
    assert!(raw_report
        .get_dictionary(info)
        .unwrap()
        .has(b"ReportFontLicence"));
    assert_eq!(report.page_count(), prepared.page_count());
    let pages = (0..report.page_count())
        .map(|p| report.page_text(p).unwrap())
        .collect::<Vec<_>>();
    assert!(pages[0].contains("Report summary"));
    assert!(pages[0].contains("12.50 CHF"));
    assert!(pages[0].contains("Cost coverage: 1 of 2"));
    assert!(pages[0].contains("Missing prices are not zero"));
    assert!(pages[0].contains("Unassigned: 2"));
    assert!(!pages[0].contains("snapshot_hash_kind"));
    assert!(!pages[0].contains("formula_arithmetic"));
    let appendix = pages
        .iter()
        .position(|page| page.contains("Technical appendix"))
        .unwrap();
    assert!(
        appendix > 0,
        "Technical provenance must start after the report content"
    );
    assert!(pages[appendix]
        .trim_start()
        .starts_with("Technical appendix"));
    let text = pages.concat();
    assert!(text.find("Annotation details").unwrap() < text.find("Technical appendix").unwrap());
    assert!(text.contains("Électricité Ω"));
    assert!(text.contains("12.50"));
    assert!(text.contains("partial subtotal"));
    assert!(text.contains("Line cost: not available"));
    assert!(text
        .split_whitespace()
        .collect::<String>()
        .contains(&prepared.model().provenance.snapshot_sha256));
    assert!(text.contains("SuniPDF-count-0"));
    assert!(text.contains("SuniPDF-count-1"));
    assert!(!text.contains("SuniPDF-shape-real"));
    let image = doc
        .render_issue_report_preview(&prepared, 0, 320, &cancel)
        .unwrap();
    assert_eq!(image.width.max(image.height), 320);
    assert!(image.rgba.windows(4).any(|p| p != [255, 255, 255, 255]));
    assert_eq!(std::fs::read(source_path).unwrap(), original);
}

#[test]
fn legends_only_contain_real_declared_styles_and_unknowns_keep_members() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let doc = engine
        .open_bytes(fixture(2), None, Path::new("plan.pdf"))
        .unwrap();
    let prepared = doc
        .prepare_issue_report(
            &doc.quantity_table().unwrap(),
            &IssueReportOptions::default(),
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    let known = prepared
        .model()
        .legend
        .iter()
        .find(|l| l.kind == "Square")
        .unwrap();
    let style = known.style.as_ref().unwrap();
    assert_eq!(style.stroke_rgb, [51, 102, 153]);
    assert_eq!(style.width_pt, 2.0);
    assert_eq!(style.opacity, 0.5);
    assert_eq!(style.dash_pattern, vec![4.0, 2.0]);
    assert_eq!(known.count, 1);
    let unknown = prepared
        .model()
        .legend
        .iter()
        .find(|l| l.kind == "Stamp")
        .unwrap();
    assert!(unknown.style.is_none());
    assert_eq!(unknown.style_status, "unknown");
    assert_eq!(unknown.count, 2);
    assert_eq!(unknown.members.len(), 2);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("legend.pdf");
    doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &out, &cancel)
        .unwrap();
    let pdf = Pdf::load(out).unwrap();
    assert!(pdf.get_pages().values().any(|id| pdf
        .get_page_content(*id)
        .windows(8)
        .any(|w| w == b"ReportSt")));
}

#[test]
fn stale_document_register_and_file_are_rejected_without_outputs() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.pdf");
    let raw = fixture(1);
    std::fs::write(&path, &raw).unwrap();
    let cancel = AtomicBool::new(false);
    let (source, bytes) = SavedReportSource::capture(&path, None, &cancel).unwrap();
    let mut doc = engine.open_bytes(bytes, None, &path).unwrap();
    let table = doc.quantity_table().unwrap();
    let register = doc.annotation_register().unwrap();
    let prepared = doc
        .prepare_issue_report(
            &table,
            &IssueReportOptions::default(),
            Some(&source),
            &cancel,
            |_| {},
        )
        .unwrap();
    std::fs::write(
        &path,
        [raw.as_slice(), b"\n% changed saved revision"].concat(),
    )
    .unwrap();
    let out = dir.path().join("stale.pdf");
    assert!(doc
        .publish_issue_report(&prepared, IssueReportFormat::Pdf, &out, &cancel)
        .is_err());
    assert!(!out.exists());
    std::fs::write(&path, &raw).unwrap();
    doc.add_count_marker(0, (60.0, 60.0), 18.0, "New", None)
        .unwrap();
    assert!(doc.validate_report_register(&register).is_err());
    assert!(doc
        .prepare_issue_report(
            &table,
            &IssueReportOptions::default(),
            None,
            &cancel,
            |_| {}
        )
        .is_err());
    assert!(doc
        .render_issue_report_preview(&prepared, 0, 320, &cancel)
        .is_err());
    assert!(doc
        .publish_issue_report(&prepared, IssueReportFormat::Json, &out, &cancel)
        .is_err());
    assert!(!out.exists());
}

#[test]
fn every_export_refuses_existing_files_and_cancellation_is_observed() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let doc = engine
        .open_bytes(fixture(1), None, Path::new("source.pdf"))
        .unwrap();
    let table = doc.quantity_table().unwrap();
    let options = IssueReportOptions::default();
    let prepared = doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("kept");
    std::fs::write(&existing, b"retain me").unwrap();
    for format in [
        IssueReportFormat::Pdf,
        IssueReportFormat::Csv,
        IssueReportFormat::Json,
    ] {
        assert!(matches!(
            doc.publish_issue_report(&prepared, format, &existing, &cancel),
            Err(EngineError::AlreadyExists(_))
        ));
        assert_eq!(std::fs::read(&existing).unwrap(), b"retain me");
    }
    let result = doc.prepare_issue_report(&table, &options, None, &cancel, |p| {
        if matches!(p.phase, IssueReportPhase::Layout) {
            cancel.store(true, Ordering::Relaxed);
        }
    });
    assert!(matches!(result, Err(EngineError::Cancelled)));
    let out = dir.path().join("cancelled.pdf");
    assert!(matches!(
        doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &out, &cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(!out.exists());
}

#[test]
fn copy_permission_is_required_but_protected_and_signed_sources_remain_read_only() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().file_name().unwrap().to_string_lossy();
    let user = format!("{seed}-reader");
    let owner = format!("{seed}-owner");
    for allow_copy in [false, true] {
        let bytes = security::protect(
            &fixture(1),
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                allow_copy,
                allow_print: true,
                allow_modify: false,
                allow_annotate: false,
                aes256: true,
            },
        )
        .unwrap();
        let path = dir.path().join(format!("protected-{allow_copy}.pdf"));
        std::fs::write(&path, &bytes).unwrap();
        let (source, raw) = SavedReportSource::capture(&path, None, &cancel).unwrap();
        let doc = engine.open_bytes(raw, Some(&user), &path).unwrap();
        assert!(!doc.properties().can_modify);
        if allow_copy {
            let prepared = doc
                .prepare_issue_report(
                    &doc.quantity_table().unwrap(),
                    &IssueReportOptions::default(),
                    Some(&source),
                    &cancel,
                    |_| {},
                )
                .unwrap();
            assert!(prepared.model().provenance.source_encrypted);
            assert!(!prepared.model().provenance.report_encrypted);
            let output = dir.path().join("derived.pdf");
            doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &output, &cancel)
                .unwrap();
            assert!(!Pdf::load(output).unwrap().is_encrypted());
            assert!(!prepared.model().json().to_string().contains(&user));
        } else {
            let table = doc.quantity_table().unwrap();
            assert!(!doc.properties().can_copy);
            assert!(!table.can_export());
            assert!(matches!(
                doc.prepare_issue_report(
                    &table,
                    &IssueReportOptions::default(),
                    Some(&source),
                    &cancel,
                    |_| {},
                ),
                Err(EngineError::SecurityRestricted)
            ));
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    let mut pdf = Pdf::load_mem(&fixture(1)).unwrap();
    let sig=pdf.add_object(dictionary!{"FT"=>"Sig","T"=>Object::string_literal("Synthetic signed field"),"V"=>dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),1.into(),2.into(),3.into()],"Contents"=>Object::string_literal("synthetic")}});
    pdf.catalog_mut().unwrap().set(
        "AcroForm",
        dictionary! {"Fields"=>vec![Object::Reference(sig)]},
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let path = dir.path().join("signed.pdf");
    std::fs::write(&path, &bytes).unwrap();
    let (source, raw) = SavedReportSource::capture(&path, None, &cancel).unwrap();
    let doc = engine.open_bytes(raw, None, &path).unwrap();
    assert!(doc.signature_count() > 0);
    let prepared = doc
        .prepare_issue_report(
            &doc.quantity_table().unwrap(),
            &IssueReportOptions::default(),
            Some(&source),
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        prepared.model().provenance.source_signature_count,
        doc.signature_count()
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn ambiguous_named_zone_refuses_report_and_empty_filter_is_explicit() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let mut doc = engine
        .open_bytes(fixture(1), None, Path::new("source.pdf"))
        .unwrap();
    let zones = [sunipdf_core::plan_zones::PlanZone {
        id: "room".into(),
        name: "Room".into(),
        parent_id: None,
        bounds: [35.0, 20.0, 80.0, 80.0],
    }];
    let p = doc.prepare_plan_zones(0, &zones, false).unwrap();
    doc.apply_plan_zones(p).unwrap();
    let table = doc.quantity_table().unwrap();
    let mut options = IssueReportOptions::default();
    options.quantities.filter.zone = Some(sunipdf_core::plan_zones::ZoneFilter {
        page: 1,
        id: "room".into(),
        descendants: false,
    });
    assert!(doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .is_err());
    options.quantities.filter.zone = None;
    options.quantities.filter.query = "NoSuchAnnotation".into();
    let prepared = doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .unwrap();
    assert!(prepared.model().quantities["rows"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(prepared.model().legend.is_empty());
    assert!(prepared.model().issue_groups.is_empty());
}

#[test]
fn original_unicode_fonts_are_embedded_and_unsupported_shaping_is_explicit() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let doc = engine
        .open_bytes(fixture(0), None, Path::new("source.pdf"))
        .unwrap();
    let table = doc.quantity_table().unwrap();
    let options = IssueReportOptions {
        title: "検査報告".into(),
        ..Default::default()
    };
    let prepared = doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .unwrap();
    assert!(prepared.model().pdf_font.starts_with("Noto"));
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("unicode.pdf");
    doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &out, &cancel)
        .unwrap();
    let report = engine.open(&out, None).unwrap();
    let text = report.page_text(0).unwrap();
    assert!(
        text.contains("検査報告"),
        "Saved report lost its exact Unicode title: {text:?}"
    );
    assert!(
        text.contains("Report summary"),
        "The summary must remain selectable on the first page"
    );
    assert!(!text.contains("snapshot_hash_kind"));
    let pages = (0..report.page_count())
        .map(|page| report.page_text(page).unwrap())
        .collect::<Vec<_>>();
    assert!(pages
        .iter()
        .skip(1)
        .any(|page| page.contains("Technical appendix")));
    assert!(
        pages.iter().any(
            |page| page.contains("Declared style sample (1:5)") && page.contains("Appearance:")
        ),
        "The real style sample must retain its text and appearance description"
    );
    // A content array can itself be indirect, but its members must be streams.
    // Legend insertion must not wrap an existing array inside another array.
    let saved = Pdf::load(&out).unwrap();
    for page_id in saved.get_pages().values() {
        let contents = saved
            .get_dictionary(*page_id)
            .unwrap()
            .get(b"Contents")
            .unwrap();
        let resolved = match contents {
            Object::Reference(id) => saved.get_object(*id).unwrap(),
            value => value,
        };
        match resolved {
            Object::Array(items) => {
                for item in items {
                    assert!(
                        saved
                            .get_object(item.as_reference().unwrap())
                            .unwrap()
                            .as_stream()
                            .is_ok(),
                        "Every content-array member must resolve to an actual stream"
                    );
                }
            }
            Object::Stream(_) => {}
            _ => panic!("Invalid saved report contents"),
        }
    }
    let options = IssueReportOptions {
        title: "e\u{301}".into(),
        ..Default::default()
    };
    assert!(doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .is_err());
}

#[test]
fn readable_totals_preserve_currency_precision_errors_and_selected_coverage() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let mut doc = engine
        .open_bytes(fixture(4), None, Path::new("pricing.pdf"))
        .unwrap();
    for (index, amount, currency, decimals) in [
        (0, "0", "CHF", 2),
        (1, "3.25", "EUR", 2),
        (2, "2", "CHF", 3),
    ] {
        let table = doc.quantity_table().unwrap();
        let target = table.target(&table.rows()[index]).unwrap();
        doc.set_quantity_fields(
            &target,
            &QuantityFields {
                unit_price: Some(UnitPrice {
                    amount: DecimalValue::parse(amount).unwrap(),
                    currency: currency.into(),
                    per_unit: QuantityUnit::Each,
                    rounding_decimals: decimals,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let table = doc.quantity_table().unwrap();
    let mut options = IssueReportOptions::default();
    options.quantities.filter.kind = "Stamp".into();
    options.quantities.formulas = sunipdf_core::quantity_table::QuantityOptions::formulas_from_json(
        br#"[{"name":"Zero allowance","expression":"0","quantity_unit":null,"rounding_decimals":2},{"name":"Unavailable allowance","expression":"[Missing]","quantity_unit":null,"rounding_decimals":2}]"#
    ).unwrap();
    let prepared = doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .unwrap();
    assert_eq!(
        prepared.model().quantities["zone_coverage"]["counts"]["unassigned"],
        5
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("totals.pdf");
    doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &path, &cancel)
        .unwrap();
    let report = engine.open(&path, None).unwrap();
    let pages = (0..report.page_count())
        .map(|page| report.page_text(page).unwrap())
        .collect::<Vec<_>>();
    let text = pages
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(pages[0].contains("Cost coverage: 3 of 4"));
    assert!(pages[0].contains("Unassigned: 4"));
    assert!(!pages[0].contains("Unassigned: 5"));
    assert!(text.contains("0.00 CHF (2 decimal places)"));
    assert!(text.contains("3.25 EUR (2 decimal places)"));
    assert!(text.contains("EUR per each (2 decimal places)"));
    assert!(text.contains("2.000 CHF (3 decimal places)"));
    assert!(text.contains("Total: 0.00 dimensionless"));
    assert!(text.contains("Total: not available; no successful calculation"));
    assert!(text.contains("Missing field: 4 annotations"));
    assert!(text.contains("Unavailable allowance: not available (Missing field)"));
    assert!(text.contains("source page/reply scope before the remaining annotation filters"));
    for index in 0..4 {
        assert!(text.contains(&format!("SuniPDF-count-{index}")));
    }
    assert!(!text.contains("SuniPDF-shape-real"));
}

#[test]
fn long_cjk_comments_continue_without_losing_selected_text_or_heading_context() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let mut source = Pdf::load_mem(&fixture(1)).unwrap();
    let page = source.get_pages()[&1];
    let annotation = source
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .as_reference()
        .unwrap();
    let comment = (0..90)
        .map(|index| format!("SEG{index:03} 検査対象配管と設備の確認を行います。 "))
        .collect::<String>();
    source
        .get_dictionary_mut(annotation)
        .unwrap()
        .set("Contents", lopdf::text_string(&comment));
    let mut bytes = Vec::new();
    source.save_to(&mut bytes).unwrap();
    let doc = engine
        .open_bytes(bytes, None, Path::new("long.pdf"))
        .unwrap();
    let mut options = IssueReportOptions {
        title: "検査報告".into(),
        ..Default::default()
    };
    options.quantities.filter.kind = "Stamp".into();
    let prepared = doc
        .prepare_issue_report(
            &doc.quantity_table().unwrap(),
            &options,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.pdf");
    doc.publish_issue_report(&prepared, IssueReportFormat::Pdf, &path, &cancel)
        .unwrap();
    let report = engine.open(&path, None).unwrap();
    let pages = (0..report.page_count())
        .map(|page| {
            report
                .page_text(page)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    let compact = pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            page.split_whitespace()
                .collect::<String>()
                .replace(&format!("SuniPDF|Page{}/{}", index + 1, pages.len()), "")
        })
        .collect::<String>();
    assert!(
        compact.contains(&comment.split_whitespace().collect::<String>()),
        "All comment characters must survive wrapping and page boundaries in their original order"
    );
    let first_comment_page = pages
        .iter()
        .position(|page| page.contains("SEG000"))
        .unwrap();
    let last_comment_page = pages
        .iter()
        .position(|page| page.contains("SEG089"))
        .unwrap();
    assert!(
        first_comment_page < last_comment_page,
        "Long comments must flow onto subsequent pages"
    );
    let annotation_page = pages
        .iter()
        .find(|page| page.contains("Page 1 | Annotation 0 | Stamp"))
        .unwrap();
    assert!(
        annotation_page.contains("Subject: Not set"),
        "The annotation heading must stay with its first detail"
    );
    assert!(pages[0].contains("Report summary"));
    assert!(
        pages
            .iter()
            .position(|page| page.contains("Technical appendix"))
            .unwrap()
            > last_comment_page
    );
}

#[test]
fn saved_hash_and_regular_file_guards_precede_report_preparation() {
    let cancel = AtomicBool::new(false);
    let dir = tempfile::tempdir().unwrap();
    assert!(SavedReportSource::capture(dir.path(), None, &cancel).is_err());
    let path = dir.path().join("source.pdf");
    std::fs::write(&path, fixture(0)).unwrap();
    assert!(SavedReportSource::capture(&path, Some(&"0".repeat(64)), &cancel).is_err());
    let (source, bytes) = SavedReportSource::capture(&path, None, &cancel).unwrap();
    assert_eq!(
        source.sha256(),
        sunipdf_core::plan_zones::source_sha256(&bytes)
    );
}

#[test]
fn bounded_groups_and_filters_refuse_without_truncating_rows() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let mut pdf = Pdf::load_mem(&fixture(257)).unwrap();
    let page = pdf.get_pages()[&1];
    let annotations = pdf
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    for (index, annotation) in annotations.iter().take(257).enumerate() {
        pdf.get_dictionary_mut(annotation.as_reference().unwrap())
            .unwrap()
            .set(
                "SuniPDFCount",
                Object::string_literal(
                    serde_json::to_vec(
                        &serde_json::json!({"version":1,"group":format!("Group {index}")}),
                    )
                    .unwrap(),
                ),
            );
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = engine
        .open_bytes(bytes, None, Path::new("bounded.pdf"))
        .unwrap();
    let table = doc.quantity_table().unwrap();
    assert_eq!(table.rows().len(), 258);
    assert!(doc
        .prepare_issue_report(
            &table,
            &IssueReportOptions::default(),
            None,
            &cancel,
            |_| {}
        )
        .is_err());
    let mut options = IssueReportOptions::default();
    options.quantities.filter.query = "x".repeat(4097);
    assert!(doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .is_err());
    options.quantities.filter.query.clear();
    options.quantities.filter.page = Some(0);
    assert!(doc
        .prepare_issue_report(&table, &options, None, &cancel, |_| {})
        .is_err());
}

#[test]
fn register_selection_keeps_its_text_fields_order_and_formula_membership() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    for typed_room in [false, true] {
        let mut pdf = Pdf::load_mem(&fixture(3)).unwrap();
        let page = pdf.get_pages()[&1];
        let annotations = pdf
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        for (index, annotation) in annotations.iter().take(3).enumerate() {
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
            dict.set("SuniPDFRegister",Object::string_literal(serde_json::to_vec(&serde_json::json!({"version":1,"fields":{"business_status":null,"discipline":null,"custom":{"Room":(["Z","A","M"][index])}}})).unwrap()));
            let mut custom = serde_json::Map::new();
            custom.insert("Tag".into(),serde_json::json!({"type":"text","value":if index==2{"visible private-tag"}else{"other"}}));
            if typed_room {
                custom.insert(
                    "Room".into(),
                    serde_json::json!({"type":"text","value":(["A","Z","M"][index])}),
                );
            }
            dict.set("SuniPDFQuantity",Object::string_literal(serde_json::to_vec(&serde_json::json!({"version":1,"fields":{"unit_price":{"amount":"12.5","currency":"CHF","per_unit":"each","rounding_decimals":2},"custom":custom}})).unwrap()));
        }
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.pdf");
        std::fs::write(&source, &bytes).unwrap();
        let doc = engine.open_bytes(bytes.clone(), None, &source).unwrap();
        let table = doc.quantity_table().unwrap();
        let mut register = doc.annotation_register().unwrap();
        let mut options = IssueReportOptions {
            selection: IssueReportSelection::Register,
            ..Default::default()
        };
        options.quantities.filter.kind = "Stamp".into();
        options.quantities.filter.query = "visible".into();
        options.quantities.filter.custom_sort = Some("Room".into());
        options.quantities.formulas=sunipdf_core::quantity_table::QuantityOptions::formulas_from_json(br#"[{"name":"Double","expression":"quantity * 2","quantity_unit":"each","rounding_decimals":2}]"#).unwrap();
        let expected = register
            .select(&options.quantities.filter, options.quantities.sort, false)
            .into_iter()
            .map(|row| row.index)
            .collect::<Vec<_>>();
        assert_eq!(expected, vec![1, 0]);
        // Public display rows cannot supply report identities or override the private source snapshot.
        register.rows.clear();
        assert!(doc.validate_report_register(&register).is_ok());
        let prepared = doc
            .prepare_issue_report(&table, &options, None, &cancel, |_| {})
            .unwrap();
        let model = prepared.model();
        assert_eq!(model.selection, IssueReportSelection::Register);
        let rows = model.quantities["rows"].as_array().unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row["register"]["index"].as_u64().unwrap() as usize)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            model.quantities["summary"]["groups"][0]["costs"][0]["total"]["amount"],
            "25.00"
        );
        assert_eq!(
            model.quantities["formula_rows"].as_array().unwrap().len(),
            2
        );
        assert_eq!(
            model.quantities["summary"]["formula_groups"][0]["totals"][0]["amount"],
            "4.00"
        );
        assert_eq!(
            model.legend.iter().map(|entry| entry.count).sum::<usize>(),
            2
        );
        let csv = dir.path().join("report.csv");
        doc.publish_issue_report(&prepared, IssueReportFormat::Csv, &csv, &cancel)
            .unwrap();
        let csv_rows = parse_csv(&std::fs::read_to_string(csv).unwrap());
        let exported = csv_rows
            .iter()
            .filter(|row| row[0] == "annotation")
            .map(|row| serde_json::from_str::<Value>(&row[24]).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(&exported, rows);
        options.selection = IssueReportSelection::Quantities;
        let quantity = doc
            .prepare_issue_report(&table, &options, None, &cancel, |_| {})
            .unwrap();
        assert_eq!(
            quantity.model().quantities["rows"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        options.selection = IssueReportSelection::Register;
        options.quantities.filter.query = "private-tag".into();
        let empty = doc
            .prepare_issue_report(&table, &options, None, &cancel, |_| {})
            .unwrap();
        assert!(empty.model().quantities["rows"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }
}

#[test]
fn saved_source_normalization_preserves_full_document_identity_and_raw_file_binding() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("external.pdf");
    // Includes uncompressed streams and fractional appearance values which PDFium serializes.
    let bytes = fixture(1);
    std::fs::write(&path, &bytes).unwrap();
    let (source, raw) = SavedReportSource::capture(&path, None, &cancel).unwrap();
    let doc = engine.open_bytes(raw, None, &path).unwrap();
    let table = doc.quantity_table().unwrap();
    let options = IssueReportOptions::default();
    let prepared = doc
        .prepare_issue_report(&table, &options, Some(&source), &cancel, |_| {})
        .unwrap();
    assert_eq!(
        prepared.model().provenance.saved_file_sha256.as_deref(),
        Some(source.sha256())
    );
    assert_eq!(
        prepared.model().provenance.snapshot_sha256,
        table.source_sha256()
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    for change in ["annotation", "resource", "metadata"] {
        let mut wrong = Pdf::load_mem(&bytes).unwrap();
        let page_id = *wrong.get_pages().values().next().unwrap();
        match change {
            "annotation" => {
                let annot = wrong
                    .get_dictionary(page_id)
                    .unwrap()
                    .get(b"Annots")
                    .unwrap()
                    .as_array()
                    .unwrap()[0]
                    .as_reference()
                    .unwrap();
                wrong.get_dictionary_mut(annot).unwrap().set(
                    "Contents",
                    lopdf::text_string("Different annotation, unchanged page content"),
                );
            }
            "resource" => {
                wrong.get_dictionary_mut(page_id).unwrap().set(
                    "Resources",
                    dictionary! {"ProcSet"=>vec![Object::Name(b"PDF".to_vec())]},
                );
            }
            _ => {
                let info = wrong.add_object(
                    dictionary! {"Title"=>lopdf::text_string("Different source metadata")},
                );
                wrong.trailer.set("Info", info);
            }
        }
        let mut wrong_bytes = Vec::new();
        wrong.save_to(&mut wrong_bytes).unwrap();
        let wrong_path = dir.path().join(format!("wrong-{change}.pdf"));
        std::fs::write(&wrong_path, &wrong_bytes).unwrap();
        let (binding, _) = SavedReportSource::capture(&wrong_path, None, &cancel).unwrap();
        assert!(
            doc.prepare_issue_report(&table, &options, Some(&binding), &cancel, |_| {})
                .is_err(),
            "wrong {change} must not pass a page-count or text-only equivalence"
        );
    }
    // Even an object-equivalent file edit invalidates the original exact byte identity.
    std::fs::write(&path, [bytes.as_slice(), b"\n% changed bytes"].concat()).unwrap();
    let out = dir.path().join("must-not-exist.json");
    assert!(doc
        .publish_issue_report(&prepared, IssueReportFormat::Json, &out, &cancel)
        .is_err());
    assert!(!out.exists());
}
