use lopdf::{dictionary, Document as Pdf, Object, Stream};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::issue_report::SavedReportSource;
use sunipdf_core::quantity_table::{
    DecimalValue, FormulaColumn, QuantityFields, QuantityUnit, UnitPrice,
};
use sunipdf_core::quantity_workbook::*;
use sunipdf_core::{security, Engine, EngineError, Protection};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// The retained PDFium instance is accessed only while holding SERIAL.
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
    let guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
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
fn fixture(rows: &[(&str, &str, bool, usize)]) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut children = Vec::new();
    for physical_page in 1..=2 {
        let content = pdf.add_object(Stream::new(dictionary! {}, b"0 g 5 5 5 5 re f".to_vec()));
        let mut annotations = Vec::new();
        for (id, contents, count, page) in rows {
            if *page != physical_page {
                continue;
            }
            let mut item = dictionary! {"Type"=>"Annot","Subtype"=>if *count {"Stamp"} else {"Square"},"NM"=>Object::string_literal(*id),"Contents"=>lopdf::text_string(contents),"Rect"=>vec![20.into(),20.into(),32.into(),32.into()]};
            if *count {
                item.set(
                    "SuniPDFCount",
                    Object::string_literal(br#"{"version":1,"group":"Equipment"}"#.to_vec()),
                );
            }
            annotations.push(Object::Reference(pdf.add_object(item)));
        }
        children.push(Object::Reference(pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()],"Resources"=>dictionary!{},"Contents"=>content,"Annots"=>annotations})));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>children,"Count"=>2}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn parts(bytes: &[u8]) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut pos = 0;
    while bytes.get(pos..pos + 4) == Some(b"PK\x03\x04") {
        let u16at =
            |index| u16::from_le_bytes(bytes[index..index + 2].try_into().unwrap()) as usize;
        let u32at =
            |index| u32::from_le_bytes(bytes[index..index + 4].try_into().unwrap()) as usize;
        assert_eq!(u16at(pos + 8), 0, "STORE package");
        let length = u32at(pos + 18);
        let name_end = pos + 30 + u16at(pos + 26);
        let name = std::str::from_utf8(&bytes[pos + 30..name_end])
            .unwrap()
            .to_owned();
        let start = name_end + u16at(pos + 28);
        let xml = std::str::from_utf8(&bytes[start..start + length])
            .unwrap()
            .to_owned();
        for event in xml::reader::EventReader::from_str(&xml) {
            event.unwrap();
        }
        assert!(found.insert(name, xml).is_none());
        pos = start + length;
    }
    assert_eq!(&bytes[pos..pos + 4], b"PK\x01\x02");
    found
}
fn formulas() -> Vec<FormulaColumn> {
    vec![FormulaColumn {
        name: "Double".into(),
        expression: "quantity * 2".into(),
        quantity_unit: Some(QuantityUnit::Each),
        rounding_decimals: 2,
    }]
}
fn snapshot_without_volatile_second_id(bytes: &[u8]) -> Pdf {
    let mut pdf = Pdf::load_mem(bytes).unwrap();
    let ids = pdf.trailer.get_mut(b"ID").unwrap().as_array_mut().unwrap();
    assert_eq!(ids.len(), 2, "snapshot file ID must contain two strings");
    assert!(
        ids.iter().all(|id| matches!(id, Object::String(_, _))),
        "snapshot file ID must contain two strings"
    );
    // PDFium may regenerate the second ID on save; preserve the permanent ID and all content.
    ids[1] = Object::Null;
    pdf
}
fn status(model: &QuantityWorkbookModel, id: &str) -> WorkbookChangeStatus {
    model
        .changes
        .iter()
        .find(|row| row.annotation_id.as_deref() == Some(id))
        .unwrap()
        .status
}

#[test]
fn workbook_cells_share_the_quantity_model_and_preserve_exact_values_without_formulas() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let mut doc = engine
        .open_bytes(
            fixture(&[("SuniPDF-A", "=SUM(A1) Étage 日本語", true, 1)]),
            None,
            Path::new("source.pdf"),
        )
        .unwrap();
    let table = doc.quantity_table().unwrap();
    doc.set_quantity_fields(
        &table.target(&table.rows()[0]).unwrap(),
        &QuantityFields {
            unit_price: Some(UnitPrice {
                amount: DecimalValue::parse("123456789012.123456").unwrap(),
                currency: "CHF".into(),
                per_unit: QuantityUnit::Each,
                rounding_decimals: 2,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let mut options = QuantityWorkbookOptions::default();
    options.quantities.formulas = formulas();
    let table = doc.quantity_table().unwrap();
    let expected = doc
        .export_quantity_table(&table, &options.quantities)
        .unwrap()
        .json;
    let before = doc.snapshot().unwrap();
    let prepared = doc
        .prepare_quantity_workbook(&table, &options, None, false, None, &cancel, |_| {})
        .unwrap();
    assert_eq!(prepared.model().quantities, expected);
    assert!(prepared.model().complete);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("quantities.xlsx");
    doc.publish_quantity_workbook(&prepared, WorkbookFormat::Xlsx, &out, &cancel)
        .unwrap();
    let archive = parts(&std::fs::read(out).unwrap());
    assert_eq!(archive.len(), 9);
    let rows = &archive["xl/worksheets/sheet1.xml"];
    assert!(rows.contains("=SUM(A1) Étage 日本語"));
    assert!(rows.contains("123456789012.123456</t>"));
    assert!(rows.contains("t=\"inlineStr\""));
    assert!(archive
        .values()
        .all(|xml| !xml.contains("<f>") && !xml.contains("TargetMode=\"External\"")));
    let manifest = QuantityWorkbookManifest::from_json(
        &serde_json::to_vec(&prepared.manifest_json()).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.rows.len(), 1);
    assert!(manifest.pending_rows.is_empty());
    let before = snapshot_without_volatile_second_id(&before);
    let after = snapshot_without_volatile_second_id(&doc.snapshot().unwrap());
    assert_eq!(after.version, before.version);
    assert_eq!(after.objects, before.objects);
    assert_eq!(after.trailer, before.trailer);
}

#[test]
fn refresh_tracks_identity_across_page_and_filter_changes_and_preserves_missing_rows() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let original = engine
        .open_bytes(
            fixture(&[
                ("SuniPDF-A", "original", true, 1),
                ("SuniPDF-B", "original", true, 1),
            ]),
            None,
            Path::new("old.pdf"),
        )
        .unwrap();
    let mut options = QuantityWorkbookOptions::default();
    options.quantities.filter.query = "original".into();
    options.quantities.formulas = formulas();
    let prepared = original
        .prepare_quantity_workbook(
            &original.quantity_table().unwrap(),
            &options,
            None,
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    let current = engine
        .open_bytes(
            fixture(&[
                ("SuniPDF-A", "changed description", true, 2),
                ("SuniPDF-C", "original", true, 1),
            ]),
            None,
            Path::new("current.pdf"),
        )
        .unwrap();
    let table = current.quantity_table().unwrap();
    let refreshed = current
        .prepare_quantity_workbook(
            &table,
            &options,
            Some(prepared.manifest()),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        refreshed.model().quantities["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        refreshed.model().quantities["rows"][0]["register"]["page"],
        2
    );
    assert_eq!(
        status(refreshed.model(), "SuniPDF-A"),
        WorkbookChangeStatus::Changed
    );
    assert_eq!(
        status(refreshed.model(), "SuniPDF-B"),
        WorkbookChangeStatus::Missing
    );
    assert_eq!(
        status(refreshed.model(), "SuniPDF-C"),
        WorkbookChangeStatus::ExcludedNew
    );
    assert!(!refreshed.model().complete);
    assert_eq!(refreshed.manifest().pending_rows.len(), 1);
    let again = current
        .prepare_quantity_workbook(
            &table,
            &options,
            Some(refreshed.manifest()),
            true,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        status(again.model(), "SuniPDF-A"),
        WorkbookChangeStatus::Unchanged
    );
    assert_eq!(
        status(again.model(), "SuniPDF-B"),
        WorkbookChangeStatus::Missing
    );
    assert_eq!(
        status(again.model(), "SuniPDF-C"),
        WorkbookChangeStatus::Added
    );
    assert_eq!(
        again.model().quantities["rows"].as_array().unwrap().len(),
        2
    );
    let moved = engine
        .open_bytes(
            fixture(&[
                ("SuniPDF-A", "original", true, 2),
                ("SuniPDF-B", "original", true, 2),
            ]),
            None,
            Path::new("moved.pdf"),
        )
        .unwrap();
    let moved_report = moved
        .prepare_quantity_workbook(
            &moved.quantity_table().unwrap(),
            &options,
            Some(prepared.manifest()),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert!(moved_report
        .model()
        .changes
        .iter()
        .all(|row| row.status == WorkbookChangeStatus::Unchanged));
}

#[test]
fn unknown_or_ambiguous_quantities_remain_explicit_and_invalid_manifests_are_rejected() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let original = engine
        .open_bytes(
            fixture(&[("SuniPDF-A", "Count", true, 1)]),
            None,
            Path::new("old.pdf"),
        )
        .unwrap();
    let options = QuantityWorkbookOptions::default();
    let before = original
        .prepare_quantity_workbook(
            &original.quantity_table().unwrap(),
            &options,
            None,
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    let current = engine
        .open_bytes(
            fixture(&[("SuniPDF-A", "Now a shape", false, 1)]),
            None,
            Path::new("new.pdf"),
        )
        .unwrap();
    let unquantified = current
        .prepare_quantity_workbook(
            &current.quantity_table().unwrap(),
            &options,
            Some(before.manifest()),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        status(unquantified.model(), "SuniPDF-A"),
        WorkbookChangeStatus::Unquantified
    );
    assert!(!unquantified.model().complete);
    assert_eq!(
        unquantified.model().quantities["summary"]["unquantified_rows"],
        1
    );
    let ambiguous = engine
        .open_bytes(
            fixture(&[
                ("SuniPDF-A", "One", false, 1),
                ("SuniPDF-A", "Two", false, 2),
            ]),
            None,
            Path::new("ambiguous.pdf"),
        )
        .unwrap();
    let result = ambiguous
        .prepare_quantity_workbook(
            &ambiguous.quantity_table().unwrap(),
            &options,
            Some(before.manifest()),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        status(result.model(), "SuniPDF-A"),
        WorkbookChangeStatus::Ambiguous
    );
    assert!(result.model().quantities["rows"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(result.manifest().pending_rows.len(), 1);
    for mutate in ["unknown", "tamper", "version"] {
        let mut value = before.manifest_json();
        match mutate {
            "unknown" => value["execute"] = json!("anything"),
            "tamper" => value["rows"][0]["row"]["quantity"]["value"] = json!(8),
            _ => value["version"] = json!(2),
        }
        assert!(QuantityWorkbookManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut wrong_options = options.clone();
    wrong_options.quantities.filter.query = "different".into();
    assert!(current
        .prepare_quantity_workbook(
            &current.quantity_table().unwrap(),
            &wrong_options,
            Some(before.manifest()),
            false,
            None,
            &cancel,
            |_| {}
        )
        .is_err());
}

#[test]
fn source_bindings_edits_cancellation_and_destination_collisions_prevent_publication() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.pdf");
    let bytes = fixture(&[("SuniPDF-A", "Count", true, 1)]);
    std::fs::write(&path, &bytes).unwrap();
    let cancel = AtomicBool::new(false);
    let (source, raw) = SavedReportSource::capture(&path, None, &cancel).unwrap();
    let mut doc = engine.open_bytes(raw, None, &path).unwrap();
    let table = doc.quantity_table().unwrap();
    let options = QuantityWorkbookOptions::default();
    let prepared = doc
        .prepare_quantity_workbook(
            &table,
            &options,
            None,
            false,
            Some(&source),
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        prepared.model().provenance.saved_file_sha256.as_deref(),
        Some(source.sha256())
    );
    let existing = dir.path().join("kept");
    std::fs::write(&existing, b"retain").unwrap();
    for format in [WorkbookFormat::Xlsx, WorkbookFormat::Manifest] {
        assert!(matches!(
            doc.publish_quantity_workbook(&prepared, format, &existing, &cancel),
            Err(EngineError::AlreadyExists(_))
        ));
    }
    assert_eq!(std::fs::read(&existing).unwrap(), b"retain");
    let out = dir.path().join("never.xlsx");
    std::fs::write(&path, [bytes.as_slice(), b"\n% changed"].concat()).unwrap();
    assert!(doc
        .publish_quantity_workbook(&prepared, WorkbookFormat::Xlsx, &out, &cancel)
        .is_err());
    assert!(!out.exists());
    std::fs::write(&path, &bytes).unwrap();
    let result =
        doc.prepare_quantity_workbook(&table, &options, None, false, None, &cancel, |progress| {
            if matches!(progress.phase, WorkbookPhase::Packaging) {
                cancel.store(true, Ordering::Relaxed);
            }
        });
    assert!(matches!(result, Err(EngineError::Cancelled)));
    cancel.store(false, Ordering::Relaxed);
    doc.add_count_marker(0, (80.0, 80.0), 18.0, "New", None)
        .unwrap();
    assert!(doc
        .publish_quantity_workbook(&prepared, WorkbookFormat::Xlsx, &out, &cancel)
        .is_err());
    assert!(doc
        .prepare_quantity_workbook(&table, &options, None, false, None, &cancel, |_| {})
        .is_err());
    assert!(!out.exists());
}

#[test]
fn protected_sources_keep_copy_permissions_and_formula_errors_are_not_zero_totals() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().file_name().unwrap().to_string_lossy();
    let reader = format!("{seed}-reader");
    let owner = format!("{seed}-owner");
    for allow_copy in [false, true] {
        let raw = security::protect(
            &fixture(&[("SuniPDF-A", "Count", true, 1)]),
            None,
            &Protection {
                user_password: reader.clone(),
                owner_password: owner.clone(),
                allow_print: true,
                allow_modify: false,
                allow_copy,
                allow_annotate: false,
                aes256: true,
            },
        )
        .unwrap();
        for password in [&reader, &owner] {
            let doc = engine
                .open_bytes(raw.clone(), Some(password), Path::new("protected.pdf"))
                .unwrap();
            let table = doc.quantity_table().unwrap();
            let result = doc.prepare_quantity_workbook(
                &table,
                &QuantityWorkbookOptions::default(),
                None,
                false,
                None,
                &cancel,
                |_| {},
            );
            assert_eq!(result.is_ok(), allow_copy);
            if let Ok(prepared) = result {
                assert!(prepared.model().provenance.source_encrypted);
            }
        }
    }
    let doc = engine
        .open_bytes(
            fixture(&[("SuniPDF-A", "Count", true, 1)]),
            None,
            Path::new("plain.pdf"),
        )
        .unwrap();
    let mut options = QuantityWorkbookOptions::default();
    options.quantities.formulas = formulas();
    options.quantities.formulas[0].expression = "quantity / 0".into();
    let prepared = doc
        .prepare_quantity_workbook(
            &doc.quantity_table().unwrap(),
            &options,
            None,
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert!(!prepared.model().complete);
    assert_eq!(
        prepared.model().quantities["formula_rows"][0]["values"]["Double"]["error"],
        "division_by_zero"
    );
    assert_eq!(
        prepared.model().quantities["summary"]["formula_groups"][0]["totals"],
        json!([])
    );
    assert!(QuantityWorkbookManifest::from_json(
        &serde_json::to_vec(&prepared.manifest_json()).unwrap()
    )
    .is_ok());
}

#[test]
fn settings_are_strict_and_reject_out_of_range_filters() {
    let mut value = serde_json::to_value(QuantityWorkbookOptions::default()).unwrap();
    value["quantities"]["filter"]["surprise"] = Value::Bool(true);
    assert!(QuantityWorkbookOptions::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut options = QuantityWorkbookOptions::default();
    options.quantities.filter.page = Some(0);
    assert!(options.validate().is_err());
    options.quantities.filter.page = None;
    options.quantities.filter.query = "x".repeat(4097);
    assert!(options.validate().is_err());
}
