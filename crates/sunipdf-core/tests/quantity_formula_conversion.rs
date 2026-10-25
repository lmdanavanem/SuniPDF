use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicBool};
use sunipdf_core::quantity_table::{
    DecimalValue, FormulaColumn, QuantityFields, QuantityOptions, QuantityUnit, TypedValue,
};
use sunipdf_core::quantity_workbook::{
    QuantityWorkbookManifest, QuantityWorkbookOptions, WorkbookChangeStatus, WorkbookFormat,
};
use sunipdf_core::Engine;

fn snapshot_without_volatile_second_id(bytes: &[u8]) -> lopdf::Document {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    let ids = pdf.trailer.get_mut(b"ID").unwrap().as_array_mut().unwrap();
    assert_eq!(ids.len(), 2, "snapshot file ID must contain two strings");
    assert!(
        ids.iter()
            .all(|id| matches!(id, lopdf::Object::String(_, _))),
        "snapshot file ID must contain two strings"
    );
    // PDFium may refresh the save identifier; the permanent ID and every object stay exact.
    ids[1] = lopdf::Object::Null;
    pdf
}

#[test]
fn saved_quantity_conversion_exports_and_refreshes_without_changing_pdf_or_schema() {
    let engine = Engine::new().expect("PDFium is required");
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("conversion.pdf"),
        )
        .unwrap();
    let index = doc
        .add_count_marker(0, (120.0, 120.0), 18.0, "Equipment", None)
        .unwrap();
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 1 && row.register.index == index)
        .unwrap();
    doc.set_quantity_fields(
        &table.target(row).unwrap(),
        &QuantityFields {
            custom: BTreeMap::from([(
                "Length".into(),
                TypedValue::Number {
                    value: DecimalValue::parse("2").unwrap(),
                    unit: Some(QuantityUnit::Feet),
                },
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    let saved = doc.snapshot().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("reopened.pdf");
    std::fs::write(&source, &saved).unwrap();
    let reopened = engine.open(&source, None).unwrap();
    let before = reopened.snapshot().unwrap();
    let formulas = vec![FormulaColumn {
        name: "Length in metres".into(),
        expression: "convert([Length], m)".into(),
        quantity_unit: None,
        rounding_decimals: 6,
    }];
    assert_eq!(
        QuantityOptions::formulas_from_json(&serde_json::to_vec(&formulas).unwrap()).unwrap(),
        formulas
    );
    let options = QuantityWorkbookOptions {
        quantities: QuantityOptions {
            formulas,
            ..Default::default()
        },
        ..Default::default()
    };
    let table = reopened.quantity_table().unwrap();
    let exported = reopened
        .export_quantity_table(&table, &options.quantities)
        .unwrap();
    assert_eq!(
        exported.json["unit_conversion"],
        "price_basis_and_explicit_formulas"
    );
    let result = exported.json["formula_rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["page"] == 1 && row["index"] == index)
        .unwrap();
    assert_eq!(
        result["values"]["Length in metres"],
        serde_json::json!({"value":"0.609600","unit":"m","error":null})
    );
    assert!(exported
        .csv
        .contains("\"2\",\"ft\",\"0.609600\",\"m\",\"\""));
    let cancel = AtomicBool::new(false);
    let prepared = reopened
        .prepare_quantity_workbook(&table, &options, None, false, None, &cancel, |_| {})
        .unwrap();
    assert_eq!(prepared.model().version, 2);
    let manifest = QuantityWorkbookManifest::from_json(
        &serde_json::to_vec(&prepared.manifest_json()).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.version, 2);
    assert_eq!(manifest.options, options);
    let baseline = manifest
        .rows
        .iter()
        .find(|row| row.row.register.page == 1 && row.row.register.index == index)
        .unwrap();
    assert_eq!(
        baseline.formulas["Length in metres"].value.as_deref(),
        Some("0.609600")
    );
    assert_eq!(
        baseline.formulas["Length in metres"].unit,
        Some(QuantityUnit::Metres)
    );
    let refreshed = reopened
        .prepare_quantity_workbook(
            &table,
            &options,
            Some(&manifest),
            false,
            None,
            &cancel,
            |_| {},
        )
        .unwrap();
    assert!(refreshed
        .model()
        .changes
        .iter()
        .all(|change| change.status == WorkbookChangeStatus::Unchanged));
    let output = directory.path().join("converted.xlsx");
    let receipt = reopened
        .publish_quantity_workbook(&refreshed, WorkbookFormat::Xlsx, &output, &cancel)
        .unwrap();
    assert!(receipt.source_unchanged);
    assert!(std::fs::metadata(output).unwrap().len() > 0);
    assert_eq!(std::fs::read(&source).unwrap(), saved);
    let before = snapshot_without_volatile_second_id(&before);
    let after = snapshot_without_volatile_second_id(&reopened.snapshot().unwrap());
    assert_eq!(after.version, before.version);
    assert_eq!(after.objects, before.objects);
    assert_eq!(after.trailer, before.trailer);
}
