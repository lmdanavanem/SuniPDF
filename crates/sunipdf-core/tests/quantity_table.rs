use std::{collections::BTreeMap, path::Path};
use sunipdf_core::annotation_register::RegisterFields;
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementUnit};
use sunipdf_core::quantity_table::*;
use sunipdf_core::{security, AnnotShape, AnnotStyle, Engine, EngineError, PointRect, Protection};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
fn fields() -> QuantityFields {
    QuantityFields {
        unit_price: Some(UnitPrice {
            amount: DecimalValue::parse("2.50").unwrap(),
            currency: "CHF".into(),
            per_unit: QuantityUnit::Metres,
            rounding_decimals: 2,
        }),
        custom: BTreeMap::from([
            (
                "Room".into(),
                TypedValue::Text {
                    value: "A-101".into(),
                },
            ),
            (
                "Checked".into(),
                TypedValue::Date {
                    value: "2026-10-22".into(),
                },
            ),
        ]),
    }
}
fn measurement() -> Measurement {
    Measurement {
        kind: MeasurementKind::Length,
        points: vec![(40.0, 80.0), (140.0, 80.0)],
        calibration: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}
fn equal_pdf(a: &[u8], b: &[u8], password: Option<&str>) {
    let normalize = |bytes: &[u8]| {
        let mut p = lopdf::Document::load_mem_with_options(
            bytes,
            lopdf::LoadOptions::with_password(password.unwrap_or_default()),
        )
        .unwrap();
        p.trailer.remove(b"ID");
        for o in p.objects.values_mut() {
            if let lopdf::Object::Stream(s) = o {
                s.start_position = None;
                s.allows_compression = true;
            }
        }
        (p.objects, p.trailer)
    };
    assert_eq!(normalize(a), normalize(b));
}

#[test]
fn quantity_table_roundtrip_targets_units_legacy_fields_permissions_and_atomicity() {
    let engine = Engine::new().unwrap();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("quantities.pdf"))
        .unwrap();
    let index = doc
        .add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    doc.add_count_marker(0, (200.0, 100.0), 18.0, "Equipment", None)
        .unwrap();
    doc.add_shape_annotation(
        0,
        AnnotShape::Square,
        PointRect {
            left: 200.0,
            bottom: 150.0,
            right: 250.0,
            top: 200.0,
        },
        &AnnotStyle::default(),
        None,
    )
    .unwrap();
    let register = doc.annotation_register().unwrap();
    let row = register
        .rows
        .iter()
        .find(|r| r.index == index && r.page == 1)
        .unwrap();
    let legacy = RegisterFields {
        business_status: Some("Open".into()),
        discipline: Some("Electrical".into()),
        custom: BTreeMap::from([("Legacy".into(), "untouched".into())]),
    };
    doc.set_annotation_register_fields(&register.target(row), &legacy, None)
        .unwrap();
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|r| r.register.index == index && r.register.page == 1)
        .unwrap();
    let target = table.target(row).unwrap();
    let before = doc.snapshot().unwrap();
    let expected = QuantityRow::from_json(&serde_json::to_vec(row).unwrap()).unwrap();
    assert!(doc.quantity_target(0, &expected).is_ok());
    doc.set_quantity_fields(&target, &fields()).unwrap();
    let priced = doc.quantity_table().unwrap();
    let row = priced
        .rows()
        .iter()
        .find(|r| r.register.index == index && r.register.page == 1)
        .unwrap();
    assert_eq!(row.line_total.as_ref().unwrap().amount, "25.00");
    assert_eq!(row.register.fields, legacy);
    assert!(priced.rows().iter().any(|r| r.quantity.is_none()));
    assert!(priced.rows().iter().any(|r| r
        .quantity
        .as_ref()
        .is_some_and(|q| q.unit == QuantityUnit::Each && q.value == 1.0)));
    let unchanged = doc.snapshot().unwrap();
    assert!(doc.set_quantity_fields(&target, &fields()).is_err());
    assert!(doc
        .export_quantity_table(&table, &QuantityOptions::default())
        .is_err());
    equal_pdf(&unchanged, &doc.snapshot().unwrap(), None);
    let reopened = engine
        .open_bytes(unchanged.clone(), None, Path::new("reopen.pdf"))
        .unwrap();
    assert_eq!(
        reopened
            .quantity_table()
            .unwrap()
            .rows()
            .iter()
            .find(|r| r.register.index == index && r.register.page == 1)
            .unwrap()
            .fields,
        fields()
    );
    drop(reopened);
    let record = doc.measurement_records(0).unwrap().remove(0);
    let target = doc.measurement_target(0, &record).unwrap();
    let mut converted = record.measurement.clone();
    converted.unit = MeasurementUnit::Centimetres;
    converted.known_distance *= 100.0;
    doc.update_measurement_annotation(&target, &converted)
        .unwrap();
    let converted = doc.quantity_table().unwrap();
    let row = converted
        .rows()
        .iter()
        .find(|r| r.register.index == index && r.register.page == 1)
        .unwrap();
    assert_eq!(row.quantity.as_ref().unwrap().value, 1000.0);
    assert_eq!(row.line_total.as_ref().unwrap().amount, "25.00");
    assert_eq!(
        row.fields.unit_price.as_ref().unwrap().per_unit,
        QuantityUnit::Metres
    );
    let mut invalid = fields();
    invalid.unit_price.as_mut().unwrap().per_unit = QuantityUnit::SquareMetres;
    let snapshot = doc.snapshot().unwrap();
    assert!(doc
        .set_quantity_fields(&converted.target(row).unwrap(), &invalid)
        .is_err());
    equal_pdf(&snapshot, &doc.snapshot().unwrap(), None);
    doc.clear_quantity_fields(&converted.target(row).unwrap())
        .unwrap();
    let cleared = doc.quantity_table().unwrap();
    let row = cleared
        .rows()
        .iter()
        .find(|r| r.register.index == index && r.register.page == 1)
        .unwrap();
    assert!(row.line_total.is_none() && row.fields.custom.is_empty());
    assert_eq!(row.register.fields, legacy);
    doc.restore(before).unwrap();
    assert!(doc
        .quantity_table()
        .unwrap()
        .rows()
        .iter()
        .all(|r| r.fields == QuantityFields::default()));

    for aes256 in [false, true] {
        for allow_copy in [false, true] {
            for allow_annotate in [false, true] {
                let mut random = [0u8; 16];
                getrandom::fill(&mut random).unwrap();
                let user = random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let owner = format!("owner-{user}");
                let encrypted = security::protect(
                    &unchanged,
                    None,
                    &Protection {
                        user_password: user.clone(),
                        owner_password: owner.clone(),
                        allow_copy,
                        allow_annotate,
                        allow_modify: false,
                        allow_print: false,
                        aes256,
                    },
                )
                .unwrap();
                for password in [&user, &owner] {
                    let mut protected = engine
                        .open_bytes(encrypted.clone(), Some(password), Path::new("locked.pdf"))
                        .unwrap();
                    let report = protected.quantity_table().unwrap();
                    let row = report
                        .rows()
                        .iter()
                        .find(|r| r.register.index == index && r.register.page == 1)
                        .unwrap();
                    if allow_copy {
                        assert!(protected
                            .export_quantity_table(&report, &QuantityOptions::default())
                            .is_ok());
                    } else {
                        assert!(matches!(
                            protected.export_quantity_table(&report, &QuantityOptions::default()),
                            Err(EngineError::SecurityRestricted)
                        ));
                        assert!(matches!(
                            report.csv(&QuantityOptions::default()),
                            Err(EngineError::SecurityRestricted)
                        ));
                    }
                    let original = protected.snapshot().unwrap();
                    if allow_annotate {
                        let target = report.target(row).unwrap();
                        let mut zero = fields();
                        zero.unit_price.as_mut().unwrap().amount =
                            DecimalValue::parse("0").unwrap();
                        protected.set_quantity_fields(&target, &zero).unwrap();
                        let bytes = protected.snapshot().unwrap();
                        assert!(engine
                            .open_bytes(bytes.clone(), None, Path::new("no-password.pdf"))
                            .is_err());
                        let verify = engine
                            .open_bytes(bytes, Some(password), Path::new("verify.pdf"))
                            .unwrap();
                        assert_eq!(verify.properties().can_copy, allow_copy);
                        assert!(!verify.properties().can_modify && !verify.properties().can_print);
                        assert_eq!(
                            verify
                                .quantity_table()
                                .unwrap()
                                .rows()
                                .iter()
                                .find(|r| r.register.index == index && r.register.page == 1)
                                .unwrap()
                                .line_total
                                .as_ref()
                                .unwrap()
                                .amount,
                            "0.00"
                        );
                    } else {
                        assert!(report.target(row).is_err());
                        equal_pdf(&original, &protected.snapshot().unwrap(), Some(password));
                    }
                }
            }
        }
    }
}
