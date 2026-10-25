use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicBool};
use sunipdf_core::annotation_register::{
    RegisterFieldFilter, RegisterFields, RegisterFilter, RegisterSort,
};
use sunipdf_core::issue_report::{IssueReportFormat, IssueReportOptions, IssueReportSelection};
use sunipdf_core::quantity_table::{QuantityFields, QuantityOptions, TypedValue};
use sunipdf_core::quantity_workbook::{
    QuantityWorkbookManifest, QuantityWorkbookOptions, WorkbookFormat,
};
use sunipdf_core::Engine;

fn filter(value: &str) -> RegisterFilter {
    RegisterFilter {
        custom_field: Some(RegisterFieldFilter {
            key: "Room".into(),
            value: value.into(),
        }),
        ..Default::default()
    }
}

fn graph(bytes: &[u8]) -> (BTreeMap<lopdf::ObjectId, lopdf::Object>, lopdf::Dictionary) {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    pdf.trailer.remove(b"ID");
    for object in pdf.objects.values_mut() {
        if let lopdf::Object::Stream(stream) = object {
            stream.start_position = None;
            stream.allows_compression = true;
        }
    }
    (pdf.objects, pdf.trailer)
}

#[test]
fn exact_register_text_filters_exports_and_manifest_refresh_without_editing_the_source() {
    // One native engine for this integration-test process.
    let engine = Engine::new().unwrap();
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("source.pdf"),
        )
        .unwrap();
    let rooms = [
        Some("A-101"),
        Some("a-101"),
        Some(""),
        None,
        Some("A-101"),
        Some(" A-101 "),
    ];
    for (number, room) in rooms.into_iter().enumerate() {
        let page = number / 3;
        let index = doc
            .add_count_marker(
                page,
                (60.0 + (number % 3) as f32 * 50.0, 80.0),
                18.0,
                "Lights",
                Some("Reviewer"),
            )
            .unwrap();
        let report = doc.annotation_register().unwrap();
        let row = report
            .rows
            .iter()
            .find(|row| row.page == page + 1 && row.index == index)
            .unwrap();
        let fields = RegisterFields {
            business_status: Some("Open".into()),
            discipline: Some("Electrical".into()),
            custom: room
                .map(|room| BTreeMap::from([("Room".into(), room.into())]))
                .unwrap_or_default(),
        };
        doc.set_annotation_register_fields(&report.target(row), &fields, None)
            .unwrap();
    }
    // Conflicting typed fields must never substitute for the register text field.
    for (page, index, value) in [
        (1, 1, "A-101"),
        (2, 0, "A-101"),
        (2, 1, "Different typed room"),
    ] {
        let table = doc.quantity_table().unwrap();
        let row = table
            .rows()
            .iter()
            .find(|row| row.register.page == page && row.register.index == index)
            .unwrap();
        doc.set_quantity_fields(
            &table.target(row).unwrap(),
            &QuantityFields {
                unit_price: None,
                custom: BTreeMap::from([(
                    "Room".into(),
                    TypedValue::Text {
                        value: value.into(),
                    },
                )]),
            },
        )
        .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    doc.save_as(&source).unwrap();
    drop(doc);
    let bytes = std::fs::read(&source).unwrap();
    let doc = engine.open(&source, None).unwrap();
    let original_graph = graph(&doc.snapshot().unwrap());
    let register = doc.annotation_register().unwrap();
    let table = doc.quantity_table().unwrap();
    let before_rows = serde_json::to_value(table.rows()).unwrap();
    let exact = filter("A-101");
    let json = register.json(&exact, RegisterSort::Page, false);
    assert_eq!(json["matched_annotations"], 2);
    assert_eq!(
        json["filter"]["custom_field"],
        serde_json::json!({"key":"Room","value":"A-101"})
    );
    assert_eq!(json["rows"][0]["page"], 1);
    assert_eq!(json["rows"][0]["index"], 0);
    assert_eq!(json["rows"][1]["page"], 2);
    assert_eq!(json["rows"][1]["index"], 1);
    assert_eq!(
        register
            .csv(&exact, RegisterSort::Page, false)
            .unwrap()
            .lines()
            .count(),
        3
    );
    let options = QuantityOptions {
        filter: exact.clone(),
        ..Default::default()
    };
    let quantities = table.json(&options).unwrap();
    assert_eq!(quantities["summary"]["matched_rows"], 2);
    assert_eq!(
        quantities["filter"]["custom_field"],
        json["filter"]["custom_field"]
    );
    assert_eq!(quantities["rows"][1]["register"]["index"], 1);
    let empty = QuantityOptions {
        filter: filter(""),
        ..Default::default()
    };
    assert_eq!(table.json(&empty).unwrap()["summary"]["matched_rows"], 1);
    assert_eq!(
        table.json(&empty).unwrap()["rows"][0]["register"]["index"],
        2
    );
    let mut scoped = options.clone();
    scoped.filter.page = Some(1);
    scoped.filter.author = "Reviewer".into();
    assert_eq!(table.json(&scoped).unwrap()["summary"]["matched_rows"], 1);
    scoped.filter.author = "Other reviewer".into();
    assert_eq!(table.json(&scoped).unwrap()["summary"]["matched_rows"], 0);
    scoped.filter.custom_field.as_mut().unwrap().key.clear();
    assert!(table.json(&scoped).is_err());
    assert!(table.csv(&scoped).is_err());
    let cancel = AtomicBool::new(false);
    for selection in [
        IssueReportSelection::Register,
        IssueReportSelection::Quantities,
    ] {
        let report_options = IssueReportOptions {
            quantities: options.clone(),
            selection,
            ..Default::default()
        };
        let prepared = doc
            .prepare_issue_report(&table, &report_options, None, &cancel, |_| {})
            .unwrap();
        assert_eq!(prepared.model().quantities["summary"]["matched_rows"], 2);
        assert_eq!(
            prepared.model().quantities["filter"]["custom_field"],
            json["filter"]["custom_field"]
        );
        let output = dir.path().join(format!("report-{}.json", selection.id()));
        doc.publish_issue_report(&prepared, IssueReportFormat::Json, &output, &cancel)
            .unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        assert_eq!(
            exported["quantities"]["filter"]["custom_field"],
            json["filter"]["custom_field"]
        );
    }
    let workbook_options = QuantityWorkbookOptions {
        quantities: options.clone(),
        ..Default::default()
    };
    let workbook = doc
        .prepare_quantity_workbook(
            &table,
            &workbook_options,
            None,
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(workbook.manifest().rows.len(), 2);
    assert_eq!(
        workbook.manifest().options.quantities.filter.custom_field,
        exact.custom_field
    );
    let manifest_path = dir.path().join("filtered-workbook.json");
    doc.publish_quantity_workbook(&workbook, WorkbookFormat::Manifest, &manifest_path, &cancel)
        .unwrap();
    let manifest =
        QuantityWorkbookManifest::from_json(&std::fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.options, workbook_options);
    let refreshed = doc
        .prepare_quantity_workbook(
            &table,
            &workbook_options,
            Some(&manifest),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert_eq!(refreshed.manifest().rows.len(), 2);
    for (before, after) in manifest.rows.iter().zip(&refreshed.manifest().rows) {
        assert_eq!(before.comparison_sha256, after.comparison_sha256);
    }
    let xlsx = dir.path().join("filtered-workbook.xlsx");
    let receipt = doc
        .publish_quantity_workbook(&workbook, WorkbookFormat::Xlsx, &xlsx, &cancel)
        .unwrap();
    assert_eq!(receipt.matched_rows, 2);
    assert!(receipt.source_unchanged);
    assert_eq!(&std::fs::read(&xlsx).unwrap()[..4], b"PK\x03\x04");
    assert!(doc
        .publish_quantity_workbook(&workbook, WorkbookFormat::Xlsx, &xlsx, &cancel)
        .is_err());
    let baseline = QuantityWorkbookOptions::default();
    let baseline_json = serde_json::to_value(&baseline).unwrap();
    assert!(baseline_json["quantities"]["filter"]
        .get("custom_field")
        .is_none());
    assert_eq!(
        serde_json::from_value::<QuantityWorkbookOptions>(baseline_json).unwrap(),
        baseline
    );
    assert_eq!(serde_json::to_value(table.rows()).unwrap(), before_rows);
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(graph(&doc.snapshot().unwrap()), original_graph);
}
