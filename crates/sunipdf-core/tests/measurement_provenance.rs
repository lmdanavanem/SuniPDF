use lopdf::{dictionary, Dictionary, Object, Stream};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{
        Measurement, MeasurementKind, MeasurementUnit, MeasurementValidity, MeasurementViewport,
        RecalibrationRequest,
    },
    quantity_table::{
        DecimalValue, FormulaColumn, QuantityFields, QuantityOptions, QuantityUnit, UnitPrice,
    },
    security, Engine, PageCalibration, PointRect, Protection, TextStyle,
};

static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();
const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn measure() -> Measurement {
    Measurement {
        kind: MeasurementKind::Length,
        points: vec![(40.0, 80.0), (140.0, 80.0)],
        calibration: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}

fn calibration() -> PageCalibration {
    PageCalibration {
        points: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 20.0,
        unit: MeasurementUnit::Metres,
    }
}

fn changed(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    change(&mut pdf);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

fn metadata(pdf: &mut lopdf::Document, change: impl FnOnce(&mut serde_json::Value)) {
    let annotation = pdf
        .objects
        .values_mut()
        .filter_map(|value| value.as_dict_mut().ok())
        .find(|dict| dict.has(b"SuniPDFMeasurement"))
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(
        annotation
            .get(b"SuniPDFMeasurement")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();
    change(&mut value);
    annotation.set(
        "SuniPDFMeasurement",
        Object::string_literal(value.to_string()),
    );
}

fn assert_same(before: &[u8], after: &[u8]) {
    let normalize = |bytes: &[u8]| {
        let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
        pdf.trailer.remove(b"ID");
        for value in pdf.objects.values_mut() {
            if let Object::Stream(stream) = value {
                stream.start_position = None;
                stream.allows_compression = true;
            }
        }
        (pdf.objects, pdf.trailer)
    };
    assert_eq!(normalize(before), normalize(after));
}

#[test]
fn priced_measurement_edits_and_batch_recalibration_refuse_incompatible_candidates_atomically() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("priced.pdf"))
        .unwrap();
    doc.add_measurement_annotation(0, &measure(), None).unwrap();
    let area = Measurement {
        kind: MeasurementKind::Area,
        points: vec![(40.0, 80.0), (140.0, 80.0), (140.0, 180.0), (40.0, 180.0)],
        known_distance: 1.0,
        ..measure()
    };
    doc.add_measurement_annotation(1, &area, None).unwrap();
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 2)
        .unwrap();
    let fields = QuantityFields {
        unit_price: Some(UnitPrice {
            amount: DecimalValue::parse("1").unwrap(),
            currency: "EUR".into(),
            per_unit: QuantityUnit::SquareMillimetres,
            rounding_decimals: 2,
        }),
        ..Default::default()
    };
    doc.set_quantity_fields(&table.target(row).unwrap(), &fields)
        .unwrap();
    let record = doc.measurement_records(1).unwrap().remove(0);
    let target = doc.measurement_target(1, &record).unwrap();
    let before = doc.snapshot().unwrap();
    let assert_price_unchanged = |doc: &sunipdf_core::Document<'_>| {
        assert_same(&before, &doc.snapshot().unwrap());
        let table = doc.quantity_table().unwrap();
        let row = table
            .rows()
            .iter()
            .find(|row| row.register.page == 2)
            .unwrap();
        assert_eq!(row.fields, fields);
        assert_eq!(row.line_total.as_ref().unwrap().amount, "1000000.00");
        assert_eq!(doc.measurement_records(1).unwrap(), vec![record.clone()]);
        assert!(table.target(row).is_ok());
    };
    for incompatible in [
        measure(),
        Measurement {
            kind: MeasurementKind::Angle,
            points: vec![(40.0, 80.0), (140.0, 80.0), (140.0, 180.0)],
            ..measure()
        },
    ] {
        let error = doc
            .update_measurement_annotation(&target, &incompatible)
            .unwrap_err();
        assert!(error.to_string().contains("quantity dimension"), "{error}");
        assert_price_unchanged(&doc);
    }
    let overflow = PageCalibration {
        known_distance: 10_000.0,
        ..calibration()
    };
    let oversized = Measurement {
        known_distance: overflow.known_distance,
        ..area.clone()
    };
    assert_eq!(oversized.value().unwrap(), 100_000_000.0);
    for error in [
        doc.update_measurement_annotation(&target, &oversized)
            .unwrap_err(),
        doc.recalibrate_measurement_annotation(&target, &area, &overflow)
            .unwrap_err(),
    ] {
        assert!(error.to_string().contains("pricing bounds"), "{error}");
        assert_price_unchanged(&doc);
    }
    // The first page would change its measurement and default scale. A later pricing failure
    // must refuse the entire preview without modifying either page or losing saved prices.
    let requests = [
        RecalibrationRequest {
            page: 0,
            calibration: calibration(),
            update_page_default: true,
            measurements: doc.measurement_records(0).unwrap(),
        },
        RecalibrationRequest {
            page: 1,
            calibration: overflow,
            update_page_default: true,
            measurements: vec![record.clone()],
        },
    ];
    let error = doc.preview_recalibration(&requests).unwrap_err();
    assert!(error.to_string().contains("pricing bounds"), "{error}");
    assert_price_unchanged(&doc);

    // A compatible change still commits through the same guarded batch path.
    let mut compatible = requests;
    compatible[1].calibration.known_distance = 2.0;
    let plan = doc.preview_recalibration(&compatible).unwrap();
    assert_price_unchanged(&doc);
    doc.apply_recalibration(&plan).unwrap();
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 2)
        .unwrap();
    assert_eq!(row.fields, fields);
    assert_eq!(row.line_total.as_ref().unwrap().amount, "4000000.00");
    assert_eq!(
        doc.measurement_records(0).unwrap()[0]
            .measurement
            .value()
            .unwrap(),
        20.0
    );
    assert_eq!(
        doc.measurement_records(1).unwrap()[0]
            .measurement
            .value()
            .unwrap(),
        4.0
    );
    doc.clear_quantity_fields(&table.target(row).unwrap())
        .unwrap();
    assert!(doc
        .quantity_table()
        .unwrap()
        .rows()
        .iter()
        .all(|row| row.line_total.is_none()));
}

#[test]
fn stored_geometry_precision_and_price_bounds_are_preflighted_before_every_recalibration() {
    let engine = engine();
    let source = changed(SOURCE, |pdf| {
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        let bounds: Vec<lopdf::Object> = [1_000_000, 1_000_000, 1_000_600, 1_000_800]
            .into_iter()
            .map(lopdf::Object::Integer)
            .collect();
        page.set("MediaBox", bounds.clone());
        page.set("CropBox", bounds);
    });
    let mut doc = engine
        .open_bytes(source, None, Path::new("offset.pdf"))
        .unwrap();
    let area = Measurement {
        kind: MeasurementKind::Area,
        points: vec![(40.0, 80.0), (140.0, 80.0), (140.0, 180.0), (40.0, 180.0)],
        known_distance: 1.0,
        precision: 0,
        ..measure()
    };
    doc.add_measurement_annotation(0, &area, None).unwrap();
    doc.add_measurement_annotation(1, &measure(), None).unwrap();
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 1)
        .unwrap();
    let fields = QuantityFields {
        unit_price: Some(UnitPrice {
            amount: DecimalValue::parse("1").unwrap(),
            currency: "EUR".into(),
            per_unit: QuantityUnit::SquareMillimetres,
            rounding_decimals: 2,
        }),
        ..Default::default()
    };
    doc.set_quantity_fields(&table.target(row).unwrap(), &fields)
        .unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    let target = doc.measurement_target(0, &record).unwrap();
    let before = doc.snapshot().unwrap();
    // A short reference loses 22% in user-space f32 at this crop origin. A longer reference
    // below loses less than 0.001%, but still crosses the pricing limit after serialization.
    for (reference_end, distance, message) in [
        (40.08, 0.8, "precision"),
        (140.000_02, 1000.0001, "pricing bounds"),
    ] {
        let calibration = PageCalibration {
            points: [(40.0, 40.0), (reference_end, 40.0)],
            known_distance: distance,
            unit: MeasurementUnit::Metres,
        };
        let candidate = Measurement {
            calibration: calibration.points,
            known_distance: distance,
            ..area.clone()
        };
        assert!(
            candidate.value().unwrap() * 1e6 < 1e12,
            "input alone passes the price bound"
        );
        let error = doc
            .update_measurement_annotation(&target, &candidate)
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        let error = doc
            .recalibrate_measurement_annotation(&target, &area, &calibration)
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        let requests = [
            RecalibrationRequest {
                page: 1,
                calibration: self::calibration(),
                update_page_default: true,
                measurements: doc.measurement_records(1).unwrap(),
            },
            RecalibrationRequest {
                page: 0,
                calibration,
                update_page_default: true,
                measurements: vec![record.clone()],
            },
        ];
        let error = doc.preview_recalibration(&requests).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert_same(&before, &doc.snapshot().unwrap());
        let table = doc.quantity_table().unwrap();
        let row = table
            .rows()
            .iter()
            .find(|row| row.register.page == 1)
            .unwrap();
        assert_eq!(row.fields, fields);
        assert_eq!(row.line_total.as_ref().unwrap().amount, "1000000.00");
        if message == "precision" {
            assert!(doc.add_measurement_annotation(0, &candidate, None).is_err());
            assert_same(&before, &doc.snapshot().unwrap());
        }
    }
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 1)
        .unwrap();
    doc.clear_quantity_fields(&table.target(row).unwrap())
        .unwrap();
    assert!(doc
        .quantity_table()
        .unwrap()
        .rows()
        .iter()
        .all(|row| row.line_total.is_none()));
}

#[test]
fn measurements_bind_page_content_without_binding_annotations_and_suspend_unverified_prices() {
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source.pdf");
    std::fs::write(&source_path, SOURCE).unwrap();
    let mut doc = engine.open(&source_path, None).unwrap();
    doc.add_measurement_annotation(0, &measure(), Some("Reviewer"))
        .unwrap();
    let original = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(original.validity, MeasurementValidity::Current);
    assert_eq!(original.result().unwrap()["version"], 2);
    doc.add_reply(0, original.index, "Review note", Some("Reviewer"))
        .unwrap();
    assert_eq!(doc.measurement_records(0).unwrap(), vec![original.clone()]);
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.index == original.index && row.register.page == 1)
        .unwrap();
    let target = table.target(row).unwrap();
    let fields = QuantityFields {
        unit_price: Some(UnitPrice {
            amount: DecimalValue::parse("2.5").unwrap(),
            currency: "EUR".into(),
            per_unit: QuantityUnit::Metres,
            rounding_decimals: 2,
        }),
        ..Default::default()
    };
    doc.set_quantity_fields(&target, &fields).unwrap();
    let current = doc.snapshot().unwrap();
    let options = QuantityOptions {
        formulas: vec![FormulaColumn {
            name: "Double".into(),
            expression: "quantity * 2".into(),
            quantity_unit: Some(QuantityUnit::Metres),
            rounding_decimals: 2,
        }],
        ..Default::default()
    };
    for bytes in [
        current.clone(),
        sunipdf_core::convert::optimize(&current, None).unwrap(),
    ] {
        let reopened = engine
            .open_bytes(bytes, None, Path::new("saved.pdf"))
            .unwrap();
        assert_eq!(
            reopened.measurement_records(0).unwrap()[0].validity,
            MeasurementValidity::Current
        );
        let rows = reopened.quantity_table().unwrap();
        let row = rows
            .rows()
            .iter()
            .find(|row| row.register.index == original.index)
            .unwrap();
        assert_eq!(row.line_total.as_ref().unwrap().amount, "25.00");
    }

    doc.add_text(0, 40.0, 180.0, "Revised drawing", &TextStyle::default())
        .unwrap();
    let stale = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(stale.validity, MeasurementValidity::Stale);
    assert_eq!(stale.measurement.value().unwrap(), 10.0);
    let exported = stale.result().unwrap();
    assert!(exported["value"].is_null());
    assert!(exported["label"].is_null());
    assert_eq!(exported["historical_value"], 10.0);
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.index == original.index)
        .unwrap();
    assert!(row.quantity.is_none());
    assert!(row.line_total.is_none());
    assert_eq!(row.historical_quantity.as_ref().unwrap().value, 10.0);
    assert_eq!(row.fields, fields);
    let summary = table.summary(&options).unwrap();
    assert_eq!(summary.stale_measurement_rows, 1);
    assert_eq!(summary.unverified_measurement_rows, 1);
    assert!(summary.groups.is_empty());
    assert_eq!(summary.formula_groups[0].errors["unverified_quantity"], 1);
    assert!(summary.formula_groups[0].totals.is_empty());
    let csv = table.csv(&options).unwrap();
    assert!(csv.contains("\"measurement_validity\""));
    assert!(csv.contains("\"historical_quantity\""));
    assert!(csv.contains("\"stale\""));
    assert!(!csv.contains("\"25.00\""));
    let target = doc.measurement_target(0, &stale).unwrap();
    let before = doc.snapshot().unwrap();
    let mut cosmetic = stale.measurement.clone();
    cosmetic.precision = 3;
    cosmetic.points[1].0 += 20.0;
    assert!(doc
        .update_measurement_annotation(&target, &cosmetic)
        .is_err());
    assert_same(&before, &doc.snapshot().unwrap());

    let preview = doc
        .preview_rich_annotation(
            0,
            &RichAnnotation::RecalibrateMeasurement {
                target: target.clone(),
                measurement: stale.measurement.clone(),
                calibration: calibration(),
            },
            None,
        )
        .unwrap();
    assert_same(&before, &doc.snapshot().unwrap());
    assert!(preview.prepared.is_some());
    doc.recalibrate_measurement_annotation(&target, &stale.measurement, &calibration())
        .unwrap();
    let recalibrated = doc.snapshot().unwrap();
    let now = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(now.validity, MeasurementValidity::Current);
    assert_eq!(now.measurement.value().unwrap(), 20.0);
    assert_eq!(now.annotation_id, stale.annotation_id);
    assert_eq!(
        doc.quantity_table().unwrap().rows()[0]
            .line_total
            .as_ref()
            .unwrap()
            .amount,
        "50.00"
    );
    doc.restore(before.clone()).unwrap();
    assert_eq!(
        doc.measurement_records(0).unwrap()[0].validity,
        MeasurementValidity::Stale
    );
    doc.restore(recalibrated).unwrap();
    assert_eq!(
        doc.measurement_records(0).unwrap()[0].validity,
        MeasurementValidity::Current
    );
    let output = temp.path().join("recalibrated.pdf");
    doc.save_as(&output).unwrap();
    assert_eq!(
        engine
            .open(&output, None)
            .unwrap()
            .measurement_records(0)
            .unwrap()[0]
            .validity,
        MeasurementValidity::Current
    );
    assert_eq!(std::fs::read(&source_path).unwrap(), SOURCE);
}

#[test]
fn legacy_records_require_explicit_recalibration_and_future_records_remain_untouched() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.add_measurement_annotation(0, &measure(), None).unwrap();
    let current = doc.snapshot().unwrap();
    let legacy = changed(&current, |pdf| {
        metadata(pdf, |record| {
            record["version"] = 1.into();
            record.as_object_mut().unwrap().remove("provenance");
        })
    });
    doc.restore(legacy).unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(record.validity, MeasurementValidity::LegacyUnverified);
    let snapshot = doc.snapshot().unwrap();
    assert_eq!(
        doc.quantity_table()
            .unwrap()
            .summary(&QuantityOptions::default())
            .unwrap()
            .legacy_measurement_rows,
        1
    );
    let target = doc.measurement_target(0, &record).unwrap();
    assert!(doc
        .update_measurement_annotation(&target, &record.measurement)
        .is_err());
    assert_same(&snapshot, &doc.snapshot().unwrap());
    let plan = doc
        .preview_recalibration(&[RecalibrationRequest {
            page: 0,
            calibration: calibration(),
            update_page_default: false,
            measurements: vec![record],
        }])
        .unwrap();
    assert_eq!(
        plan.changes()[0].before_validity,
        MeasurementValidity::LegacyUnverified
    );
    assert_same(&snapshot, &doc.snapshot().unwrap());
    doc.apply_recalibration(&plan).unwrap();
    assert_eq!(
        doc.measurement_records(0).unwrap()[0].validity,
        MeasurementValidity::Current
    );
    assert!(doc.apply_recalibration(&plan).is_err());

    for fault in ["version", "hash", "bounds", "future_field"] {
        let damaged = changed(&current, |pdf| {
            metadata(pdf, |record| match fault {
                "version" => record["version"] = 99.into(),
                "hash" => record["provenance"]["content_sha256"] = "invalid".into(),
                "bounds" => record["provenance"]["page_bounds"] = serde_json::json!([0, 0, 0, 0]),
                _ => record["future_field"] = true.into(),
            })
        });
        doc.restore(damaged).unwrap();
        let before = doc.snapshot().unwrap();
        assert!(doc.measurement_records(0).is_err(), "{fault}");
        assert!(doc.quantity_table().is_err(), "{fault}");
        assert_same(&before, &doc.snapshot().unwrap());
    }
}

#[test]
fn resource_only_geometry_and_user_unit_edits_invalidate_records_and_angles_cannot_be_retrusted() {
    let engine = engine();
    let source = changed(SOURCE, |pdf| {
        let form = pdf.add_object(Stream::new(dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(),0.into(),300.into(),300.into()], "Resources" => Dictionary::new()}, b"40 80 m 140 80 l S".to_vec()));
        let contents = pdf.add_object(Stream::new(Dictionary::new(), b"/Fm0 Do".to_vec()));
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Contents", contents);
        page.set(
            "Resources",
            dictionary! {"XObject" => dictionary! {"Fm0" => form}},
        );
    });
    let mut doc = engine
        .open_bytes(source, None, Path::new("drawing.pdf"))
        .unwrap();
    let mut angle = measure();
    angle.kind = MeasurementKind::Angle;
    angle.points = vec![(40.0, 80.0), (140.0, 80.0), (140.0, 180.0)];
    doc.add_measurement_annotation(0, &angle, None).unwrap();
    let current = doc.snapshot().unwrap();
    for fault in ["resource", "bounds", "user_unit"] {
        let modified = changed(&current, |pdf| {
            let page = pdf.get_pages()[&1];
            match fault {
                "resource" => {
                    let page = pdf.get_dictionary(page).unwrap();
                    let resources = pdf
                        .dereference(page.get(b"Resources").unwrap())
                        .unwrap()
                        .1
                        .as_dict()
                        .unwrap();
                    let objects = pdf
                        .dereference(resources.get(b"XObject").unwrap())
                        .unwrap()
                        .1
                        .as_dict()
                        .unwrap();
                    let form_id = objects.get(b"Fm0").unwrap().as_reference().unwrap();
                    let form = pdf
                        .get_object_mut(form_id)
                        .unwrap()
                        .as_stream_mut()
                        .unwrap();
                    form.dict.set(
                        "Matrix",
                        vec![2.into(), 0.into(), 0.into(), 1.into(), 0.into(), 0.into()],
                    );
                }
                "bounds" => pdf
                    .get_dictionary_mut(page)
                    .unwrap()
                    .set("CropBox", vec![0.into(), 0.into(), 600.into(), 700.into()]),
                _ => pdf.get_dictionary_mut(page).unwrap().set("UserUnit", 2),
            }
        });
        doc.restore(modified).unwrap();
        let record = doc.measurement_records(0).unwrap().remove(0);
        assert_eq!(record.validity, MeasurementValidity::Stale, "{fault}");
        let target = doc.measurement_target(0, &record).unwrap();
        let before = doc.snapshot().unwrap();
        assert!(doc
            .update_measurement_annotation(&target, &record.measurement)
            .is_err());
        assert!(doc
            .recalibrate_measurement_annotation(&target, &record.measurement, &calibration())
            .is_err());
        assert_same(&before, &doc.snapshot().unwrap());
    }
    for aes256 in [false, true] {
        let protected = security::protect(
            &current,
            None,
            &Protection {
                user_password: "reader".into(),
                owner_password: "owner".into(),
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let encrypted = engine
            .open_bytes(protected, Some("reader"), Path::new("protected.pdf"))
            .unwrap();
        assert_eq!(
            encrypted.measurement_records(0).unwrap()[0].validity,
            MeasurementValidity::Current
        );
    }
}

#[test]
fn layer_visibility_and_page_group_changes_invalidate_measurements_scales_and_regions() {
    let engine = engine();
    let source = changed(SOURCE, |pdf| {
        let layer = pdf.add_object(dictionary! {"Type" => "OCG", "Name" => "Drawing alternative"});
        pdf.catalog_mut().unwrap().set("OCProperties", dictionary! {"OCGs" => vec![Object::Reference(layer)], "D" => dictionary! {"BaseState" => "ON"}});
        let contents = pdf.add_object(Stream::new(
            Dictionary::new(),
            b"/OC /Layer1 BDC 40 80 m 140 80 l S EMC".to_vec(),
        ));
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Contents", contents);
        page.set(
            "Resources",
            dictionary! {"Properties" => dictionary! {"Layer1" => layer}},
        );
        page.set("Group", dictionary! {"Type" => "Group", "S" => "Transparency", "CS" => "DeviceRGB", "I" => false});
    });
    let mut doc = engine
        .open_bytes(source, None, Path::new("layers.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&calibration())).unwrap();
    let region = MeasurementViewport {
        id: "detail".into(),
        name: "Detail".into(),
        bounds: PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 200.0,
            top: 200.0,
        },
        calibration: calibration(),
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&region))
        .unwrap();
    doc.add_measurement_annotation(0, &measure(), None).unwrap();
    let current = doc.snapshot().unwrap();
    assert_eq!(
        doc.measurement_records(0).unwrap()[0].validity,
        MeasurementValidity::Current
    );
    assert_eq!(doc.page_calibration(0).unwrap(), Some(calibration()));
    assert_eq!(doc.measurement_viewports(0).unwrap(), vec![region]);
    for change in ["visibility", "group", "remove_visibility", "remove_group"] {
        let bytes = changed(&current, |pdf| {
            let page = pdf.get_pages()[&1];
            match change {
                "visibility" => {
                    let mut properties = pdf.catalog().unwrap().get_deref(b"OCProperties", pdf).unwrap().as_dict().unwrap().clone();
                    properties.set("D", dictionary! {"BaseState" => "OFF"});
                    pdf.catalog_mut().unwrap().set("OCProperties", properties);
                }
                "group" => pdf.get_dictionary_mut(page).unwrap().set("Group", dictionary! {"Type" => "Group", "S" => "Transparency", "CS" => "DeviceRGB", "I" => true}),
                "remove_visibility" => { pdf.catalog_mut().unwrap().remove(b"OCProperties"); }
                _ => { pdf.get_dictionary_mut(page).unwrap().remove(b"Group"); }
            }
        });
        doc.restore(bytes).unwrap();
        assert_eq!(
            doc.measurement_records(0).unwrap()[0].validity,
            MeasurementValidity::Stale,
            "{change}"
        );
        assert!(doc.page_calibration(0).is_err(), "{change}");
        assert!(doc.measurement_viewports(0).is_err(), "{change}");
        assert_eq!(
            doc.quantity_table()
                .unwrap()
                .summary(&QuantityOptions::default())
                .unwrap()
                .stale_measurement_rows,
            1
        );
    }

    // An old digest cannot prove that rendering state was not removed since capture.
    let legacy = changed(&current, |pdf| {
        pdf.catalog_mut().unwrap().remove(b"OCProperties");
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.remove(b"Group");
        for (key, version) in [
            (b"SuniPDFPageCalibration".as_slice(), 2),
            (b"SuniPDFMeasurementViewports".as_slice(), 1),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_slice(page.get(key).unwrap().as_str().unwrap()).unwrap();
            value["version"] = version.into();
            page.set(key, Object::string_literal(value.to_string()));
        }
    });
    doc.restore(legacy).unwrap();
    let before = doc.snapshot().unwrap();
    assert!(doc
        .page_calibration(0)
        .unwrap_err()
        .to_string()
        .contains("legacy saved scale"));
    assert!(doc
        .measurement_viewports(0)
        .unwrap_err()
        .to_string()
        .contains("legacy saved regions"));
    assert_same(&before, &doc.snapshot().unwrap());
    doc.set_page_calibration(0, Some(&calibration())).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(calibration()));
    let reset = doc.measurement_viewport_reset_target(0).unwrap();
    doc.clear_measurement_viewports(&reset).unwrap();
    assert!(doc.measurement_viewports(0).unwrap().is_empty());
}
