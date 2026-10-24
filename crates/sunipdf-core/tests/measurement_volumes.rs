use lopdf::{Dictionary, Object};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
};
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{
        Measurement, MeasurementDepth, MeasurementKind, MeasurementUnit, MeasurementValidity,
        RecalibrationRequest,
    },
    quantity_table::{
        DecimalValue, FormulaColumn, QuantityDimension, QuantityFields, QuantityOptions,
        QuantityUnit, UnitPrice,
    },
    security, Engine, PageCalibration, Protection, TextStyle,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().expect("PDFium is required")))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn rectangle(left: f32, bottom: f32, width: f32, height: f32) -> Vec<(f32, f32)> {
    vec![
        (left, bottom),
        (left + width, bottom),
        (left + width, bottom + height),
        (left, bottom + height),
    ]
}

fn area() -> Measurement {
    Measurement {
        kind: MeasurementKind::Area,
        points: rectangle(40.0, 80.0, 100.0, 100.0),
        holes: vec![rectangle(60.0, 100.0, 20.0, 50.0)],
        depth: None,
        calibration: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}

fn volume() -> Measurement {
    Measurement {
        kind: MeasurementKind::Volume,
        depth: Some(MeasurementDepth {
            value: 0.2,
            unit: MeasurementUnit::Metres,
        }),
        ..area()
    }
}

fn circle(x: f32, y: f32, radius: f32, n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / n as f64;
            (
                x + radius * angle.cos() as f32,
                y + radius * angle.sin() as f32,
            )
        })
        .collect()
}

fn changed(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    change(&mut pdf);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

fn annotation(pdf: &mut lopdf::Document) -> &mut Dictionary {
    pdf.objects
        .values_mut()
        .filter_map(|value| value.as_dict_mut().ok())
        .find(|dict| dict.has(b"SuniPDFMeasurement"))
        .unwrap()
}

fn change_metadata(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    changed(bytes, |pdf| {
        let dict = annotation(pdf);
        let mut value: serde_json::Value =
            serde_json::from_slice(dict.get(b"SuniPDFMeasurement").unwrap().as_str().unwrap())
                .unwrap();
        change(&mut value);
        dict.set(
            "SuniPDFMeasurement",
            Object::string_literal(value.to_string()),
        );
    })
}

fn assert_same(before: &[u8], after: &[u8]) {
    let normalize = |bytes: &[u8]| {
        let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
        if pdf.is_encrypted() {
            pdf = lopdf::Document::load_mem_with_options(
                bytes,
                lopdf::LoadOptions::with_password("volume-fixture-user"),
            )
            .unwrap();
        }
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
fn net_area_and_constant_depth_use_physical_units_and_preserve_ring_order() {
    let mut surface = area();
    let breakdown = surface.area_breakdown().unwrap();
    assert_eq!(breakdown.gross, 100.0);
    assert_eq!(breakdown.deductions, vec![10.0]);
    assert_eq!(breakdown.net, 90.0);
    assert_eq!(surface.value().unwrap(), 90.0);
    surface.points.reverse();
    surface.holes[0].reverse();
    assert_eq!(surface.value().unwrap(), 90.0);
    surface.holes.push(rectangle(100.0, 100.0, 20.0, 20.0));
    assert_eq!(
        surface.area_breakdown().unwrap().deductions,
        vec![10.0, 4.0]
    );
    assert_eq!(surface.value().unwrap(), 86.0);
    let mut solid = volume();
    assert_eq!(solid.value().unwrap(), 18.0);
    solid.depth = Some(MeasurementDepth {
        value: 20.0,
        unit: MeasurementUnit::Centimetres,
    });
    assert_eq!(solid.value().unwrap(), 18.0);
    for unit in MeasurementUnit::ALL {
        solid.unit = unit;
        solid.known_distance = 10.0 / unit.metres();
        let represented = solid.value().unwrap() * unit.metres().powi(3);
        assert!((represented - 18.0).abs() < 1e-9, "{unit:?}: {represented}");
        assert_eq!(
            solid.depth.unwrap(),
            MeasurementDepth {
                value: 20.0,
                unit: MeasurementUnit::Centimetres
            }
        );
        let result = solid.result().unwrap();
        assert_eq!(result["unit"], format!("{}3", unit.id()));
        assert_eq!(result["depth"]["unit"], "cm");
        assert_eq!(result["area_unit"], format!("{}2", unit.id()));
    }
}

#[test]
fn topology_refuses_touching_overlapping_nested_crossing_or_unbounded_deductions() {
    let invalid_rings = vec![
        vec![rectangle(20.0, 100.0, 30.0, 20.0)],
        vec![rectangle(40.0, 100.0, 20.0, 20.0)],
        vec![
            rectangle(60.0, 100.0, 20.0, 50.0),
            rectangle(65.0, 110.0, 5.0, 5.0),
        ],
        vec![
            rectangle(60.0, 100.0, 20.0, 50.0),
            rectangle(70.0, 120.0, 30.0, 20.0),
        ],
        vec![
            rectangle(60.0, 100.0, 20.0, 50.0),
            rectangle(80.0, 110.0, 10.0, 10.0),
        ],
        vec![vec![
            (60.0, 100.0),
            (100.0, 140.0),
            (60.0, 140.0),
            (100.0, 100.0),
        ]],
        vec![vec![(60.0, 100.0), (80.0, 100.0), (70.0, 100.0)]],
        vec![vec![
            (60.0, 100.0),
            (80.0, 100.0),
            (80.0, 120.0),
            (60.0, 100.0),
        ]],
        vec![rectangle(60.0, 100.0, 0.005, 20.0)],
        vec![vec![(60.0, 100.0), (f32::NAN, 100.0), (80.0, 120.0)]],
        (0..17)
            .map(|i| rectangle(50.0 + i as f32 * 4.0, 100.0, 2.0, 2.0))
            .collect(),
        vec![circle(90.0, 130.0, 10.0, 129)],
        vec![
            circle(70.0, 110.0, 5.0, 128),
            circle(110.0, 110.0, 5.0, 128),
            circle(70.0, 150.0, 5.0, 128),
            circle(110.0, 150.0, 5.0, 128),
        ],
    ];
    for holes in invalid_rings {
        let invalid = Measurement { holes, ..area() };
        assert!(invalid.value().is_err(), "accepted {invalid:?}");
    }
    let maximum = Measurement {
        points: circle(200.0, 200.0, 150.0, 128),
        holes: vec![
            circle(150.0, 150.0, 10.0, 128),
            circle(250.0, 150.0, 10.0, 128),
            circle(200.0, 250.0, 10.0, 128),
        ],
        ..area()
    };
    assert!(maximum.value().is_ok());
    let mut invalid = area();
    invalid.points = vec![(40.0, 80.0), (140.0, 180.0), (40.0, 180.0), (140.0, 80.0)];
    assert!(invalid.value().is_err());
    let concave = Measurement {
        points: vec![
            (40.0, 80.0),
            (140.0, 80.0),
            (140.0, 180.0),
            (100.0, 180.0),
            (100.0, 120.0),
            (80.0, 120.0),
            (80.0, 180.0),
            (40.0, 180.0),
        ],
        // Every vertex lies inside, but the top edge crosses the exterior notch.
        holes: vec![rectangle(60.0, 100.0, 60.0, 40.0)],
        ..area()
    };
    assert!(concave.value().is_err());
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e10] {
        let invalid = Measurement {
            depth: Some(MeasurementDepth {
                value,
                unit: MeasurementUnit::Metres,
            }),
            ..volume()
        };
        assert!(invalid.value().is_err());
    }
    assert!(Measurement {
        depth: None,
        ..volume()
    }
    .value()
    .is_err());
    assert!(Measurement {
        kind: MeasurementKind::Area,
        ..volume()
    }
    .value()
    .is_err());
    assert!(Measurement {
        kind: MeasurementKind::Perimeter,
        ..area()
    }
    .value()
    .is_err());
}

#[test]
fn native_surface_volume_roundtrip_covers_rotations_crops_preview_restore_and_outline_appearance() {
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let source = changed(SOURCE, |pdf| {
            let page = pdf.get_pages()[&1];
            let dict = pdf.get_dictionary_mut(page).unwrap();
            dict.set("Rotate", rotation);
            dict.set(
                "MediaBox",
                vec![1000.into(), 2000.into(), 1612.into(), 2792.into()],
            );
            dict.set(
                "CropBox",
                vec![1020.into(), 2030.into(), 1600.into(), 2780.into()],
            );
        });
        let mut doc = engine
            .open_bytes(source, None, Path::new("surface.pdf"))
            .unwrap();
        let before = doc.snapshot().unwrap();
        doc.add_measurement_annotation(0, &area(), None).unwrap();
        let surface = doc.measurement_records(0).unwrap().remove(0);
        assert_eq!(surface.measurement, area());
        assert_eq!(surface.result().unwrap()["area"]["net"], 90.0);
        doc.restore(before.clone()).unwrap();
        let preview = doc
            .preview_rich_annotation(
                0,
                &RichAnnotation::Measurement(volume()),
                Some("Fixture author"),
            )
            .unwrap();
        assert_same(&before, &doc.snapshot().unwrap());
        doc.commit_rich_annotation_preview(preview.prepared.unwrap())
            .unwrap();
        let index = doc.measurement_records(0).unwrap()[0].index;
        let after = doc.snapshot().unwrap();
        let mut pdf = lopdf::Document::load_mem(&after).unwrap();
        let dict = annotation(&mut pdf).clone();
        let metadata: serde_json::Value =
            serde_json::from_slice(dict.get(b"SuniPDFMeasurement").unwrap().as_str().unwrap())
                .unwrap();
        assert_eq!(metadata["version"], 3);
        assert_eq!(metadata["holes"].as_array().unwrap().len(), 1);
        assert!(metadata.get("provenance").is_some());
        let ap = pdf
            .dereference(dict.get(b"AP").unwrap())
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
        let content = String::from_utf8(stream.get_plain_content().unwrap()).unwrap();
        assert!(content.contains("h S"));
        assert!(
            !content.contains(" re f"),
            "surface appearance must not conceal the plan"
        );
        doc.restore(before).unwrap();
        assert!(doc.measurement_records(0).unwrap().is_empty());
        doc.restore(after.clone()).unwrap();
        let record = doc.measurement_records(0).unwrap().remove(0);
        assert_eq!(record.index, index);
        assert_eq!(record.validity, MeasurementValidity::Current);
        assert_eq!(record.measurement, volume());
        assert_eq!(record.result().unwrap()["version"], 3);
        assert_eq!(record.result().unwrap()["label"], "18.00 m3");
        let file = temp.path().join(format!("surface-{rotation}.pdf"));
        doc.save_as(&file).unwrap();
        let reopened = engine.open(&file, None).unwrap();
        assert_eq!(
            reopened.measurement_records(0).unwrap()[0].measurement,
            volume()
        );
        assert_eq!(
            reopened.quantity_table().unwrap().rows()[0]
                .quantity
                .as_ref()
                .unwrap()
                .unit,
            QuantityUnit::CubicMetres
        );
    }
}

#[test]
fn pricing_conversion_recalibration_and_stale_exclusion_keep_depth_and_fields() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("priced.pdf"))
        .unwrap();
    doc.add_measurement_annotation(0, &volume(), None).unwrap();
    let fields = QuantityFields {
        unit_price: Some(UnitPrice {
            amount: DecimalValue::parse("2.5").unwrap(),
            currency: "CHF".into(),
            per_unit: QuantityUnit::CubicMetres,
            rounding_decimals: 2,
        }),
        ..Default::default()
    };
    let table = doc.quantity_table().unwrap();
    doc.set_quantity_fields(&table.target(&table.rows()[0]).unwrap(), &fields)
        .unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    let target = doc.measurement_target(0, &record).unwrap();
    let before = doc.snapshot().unwrap();
    assert!(doc.update_measurement_annotation(&target, &area()).is_err());
    assert_same(&before, &doc.snapshot().unwrap());
    let mut centimetres = volume();
    centimetres.unit = MeasurementUnit::Centimetres;
    centimetres.known_distance = 1000.0;
    doc.update_measurement_annotation(&target, &centimetres)
        .unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(record.measurement.depth, volume().depth);
    assert_eq!(record.measurement.value().unwrap(), 18_000_000.0);
    let table = doc.quantity_table().unwrap();
    assert_eq!(table.rows()[0].line_total.as_ref().unwrap().amount, "45.00");
    assert_eq!(table.rows()[0].fields, fields);
    assert_eq!(
        table.rows()[0].quantity.as_ref().unwrap().unit.dimension(),
        QuantityDimension::Volume
    );
    let plan = doc
        .preview_recalibration(&[RecalibrationRequest {
            page: 0,
            calibration: PageCalibration {
                points: volume().calibration,
                known_distance: 20.0,
                unit: MeasurementUnit::Metres,
            },
            update_page_default: false,
            measurements: vec![record],
        }])
        .unwrap();
    doc.apply_recalibration(&plan).unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(record.measurement.depth, volume().depth);
    assert_eq!(record.measurement.value().unwrap(), 72_000_000.0);
    assert_eq!(
        doc.quantity_table().unwrap().rows()[0]
            .line_total
            .as_ref()
            .unwrap()
            .amount,
        "180.00"
    );
    doc.add_text(0, 50.0, 250.0, "Revision", &TextStyle::default())
        .unwrap();
    let stale = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(stale.validity, MeasurementValidity::Stale);
    let result = stale.result().unwrap();
    assert!(result["value"].is_null() && result["area"].is_null());
    assert_eq!(result["historical_value"], 72_000_000.0);
    assert_eq!(result["historical_area"]["net"], 3_600_000.0);
    let table = doc.quantity_table().unwrap();
    let row = &table.rows()[0];
    assert_eq!(row.fields, fields);
    assert!(row.quantity.is_none() && row.line_total.is_none());
    let options = QuantityOptions {
        formulas: vec![FormulaColumn {
            name: "Double volume".into(),
            expression: "quantity * 2".into(),
            quantity_unit: Some(QuantityUnit::CubicCentimetres),
            rounding_decimals: 2,
        }],
        ..Default::default()
    };
    let summary = table.summary(&options).unwrap();
    assert_eq!(summary.formula_groups[0].errors["unverified_quantity"], 1);
    assert!(summary.formula_groups[0].totals.is_empty());
    let target = doc.measurement_target(0, &stale).unwrap();
    assert!(doc
        .update_measurement_annotation(&target, &stale.measurement)
        .is_err());
    doc.recalibrate_measurement_annotation(
        &target,
        &stale.measurement,
        &PageCalibration {
            points: volume().calibration,
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        },
    )
    .unwrap();
    let current = doc.measurement_records(0).unwrap().remove(0);
    assert_eq!(current.validity, MeasurementValidity::Current);
    assert_eq!(current.measurement.depth, volume().depth);
    assert_eq!(
        doc.quantity_table().unwrap().rows()[0]
            .line_total
            .as_ref()
            .unwrap()
            .amount,
        "45.00"
    );
}

#[test]
fn v3_metadata_is_strict_while_v1_v2_ordinary_measurements_remain_readable() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("schema.pdf"))
        .unwrap();
    doc.add_measurement_annotation(0, &volume(), None).unwrap();
    let saved = doc.snapshot().unwrap();
    for change in [
        |v: &mut serde_json::Value| {
            v["version"] = 4.into();
        },
        |v: &mut serde_json::Value| {
            v.as_object_mut().unwrap().remove("holes");
        },
        |v: &mut serde_json::Value| {
            v["depth"] = serde_json::Value::Null;
        },
        |v: &mut serde_json::Value| {
            v["depth"]["unit"] = "m3".into();
        },
        |v: &mut serde_json::Value| {
            v["depth"]["extra"] = 1.into();
        },
        |v: &mut serde_json::Value| {
            v["kind"] = "area".into();
        },
        |v: &mut serde_json::Value| {
            v["version"] = 2.into();
        },
    ] {
        let bytes = change_metadata(&saved, change);
        let invalid = engine
            .open_bytes(bytes, None, Path::new("invalid.pdf"))
            .unwrap();
        assert!(invalid.measurement_records(0).is_err());
    }
    let mut ordinary = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("ordinary.pdf"))
        .unwrap();
    ordinary
        .add_measurement_annotation(
            0,
            &Measurement {
                holes: Vec::new(),
                ..area()
            },
            None,
        )
        .unwrap();
    let v2 = ordinary.snapshot().unwrap();
    let current = ordinary.measurement_records(0).unwrap().remove(0);
    assert_eq!(current.result().unwrap()["version"], 2);
    assert_eq!(current.validity, MeasurementValidity::Current);
    assert!(current.measurement.holes.is_empty() && current.measurement.depth.is_none());
    let legacy = change_metadata(&v2, |v| {
        v["version"] = 1.into();
        v.as_object_mut().unwrap().remove("provenance");
    });
    let reopened = engine
        .open_bytes(legacy, None, Path::new("legacy.pdf"))
        .unwrap();
    let record = reopened.measurement_records(0).unwrap().remove(0);
    assert_eq!(record.validity, MeasurementValidity::LegacyUnverified);
    assert!(record.result().unwrap()["value"].is_null());
    assert_eq!(record.result().unwrap()["historical_area"]["net"], 100.0);
}

#[test]
fn storage_refuses_uniform_position_loss_even_when_volume_and_edge_lengths_are_unchanged() {
    let engine = engine();
    let source = changed(SOURCE, |pdf| {
        let page = pdf.get_pages()[&1];
        let dict = pdf.get_dictionary_mut(page).unwrap();
        let bounds: Vec<Object> = [16_777_216, 16_777_216, 16_777_816, 16_778_016]
            .into_iter()
            .map(Object::Integer)
            .collect();
        dict.set("MediaBox", bounds.clone());
        dict.set("CropBox", bounds);
        dict.set("Rotate", 0);
    });
    let mut doc = engine
        .open_bytes(source, None, Path::new("uniform-position-loss.pdf"))
        .unwrap();
    let before = doc.snapshot().unwrap();
    let geo = doc.page_geometry(0).unwrap();
    let roundtrip = |(x, y)| {
        let (x, y) = geo.to_user(x, y);
        geo.to_display(x, y)
    };
    for displaced in ["hole", "outer", "calibration"] {
        let mut measurement = volume();
        measurement.calibration = [(40.0, 80.0), (140.0, 80.0)];
        let points = match displaced {
            "hole" => measurement.holes[0].as_mut_slice(),
            "outer" => measurement.points.as_mut_slice(),
            _ => measurement.calibration.as_mut_slice(),
        };
        for point in points {
            point.0 += 0.75;
            point.1 += 0.75;
        }
        let mut stored = measurement.clone();
        stored.points = stored.points.into_iter().map(roundtrip).collect();
        stored.holes = stored
            .holes
            .into_iter()
            .map(|ring| ring.into_iter().map(roundtrip).collect())
            .collect();
        stored.calibration = stored.calibration.map(roundtrip);
        assert_ne!(stored, measurement, "{displaced} must move in storage");
        assert_eq!(measurement.value().unwrap(), 18.0);
        assert_eq!(stored.value().unwrap(), 18.0);
        assert_eq!(
            stored.area_breakdown().unwrap(),
            measurement.area_breakdown().unwrap()
        );
        assert!(
            doc.add_measurement_annotation(0, &measurement, None)
                .is_err(),
            "{displaced}"
        );
        assert_same(&before, &doc.snapshot().unwrap());
        assert!(doc.measurement_records(0).unwrap().is_empty());
    }
}

#[test]
fn storage_precision_and_pdf_permissions_refuse_changes_without_partial_annotation() {
    let engine = engine();
    let source = changed(SOURCE, |pdf| {
        let page = pdf.get_pages()[&1];
        let dict = pdf.get_dictionary_mut(page).unwrap();
        let bounds: Vec<Object> = [1_000_000, 1_000_000, 1_000_600, 1_000_800]
            .into_iter()
            .map(Object::Integer)
            .collect();
        dict.set("MediaBox", bounds.clone());
        dict.set("CropBox", bounds);
    });
    let mut doc = engine
        .open_bytes(source, None, Path::new("precision.pdf"))
        .unwrap();
    let before = doc.snapshot().unwrap();
    let narrow = Measurement {
        holes: vec![rectangle(60.01, 100.0, 0.02, 50.0)],
        ..area()
    };
    assert!(narrow.value().is_ok());
    assert!(doc.add_measurement_annotation(0, &narrow, None).is_err());
    assert_same(&before, &doc.snapshot().unwrap());
    assert!(doc.measurement_records(0).unwrap().is_empty());
    for aes256 in [false, true] {
        for allow_annotate in [false, true] {
            let protected = security::protect(
                SOURCE,
                None,
                &Protection {
                    user_password: "volume-fixture-user".into(),
                    owner_password: "volume-fixture-owner".into(),
                    allow_annotate,
                    allow_copy: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut doc = engine
                .open_bytes(
                    protected,
                    Some("volume-fixture-user"),
                    Path::new("protected.pdf"),
                )
                .unwrap();
            let before = doc.snapshot().unwrap();
            let result = doc.add_measurement_annotation(0, &volume(), None);
            assert_eq!(result.is_ok(), allow_annotate);
            if allow_annotate {
                let bytes = doc.snapshot().unwrap();
                assert!(engine
                    .open_bytes(bytes.clone(), None, Path::new("still-protected.pdf"))
                    .is_err());
                let reopened = engine
                    .open_bytes(
                        bytes,
                        Some("volume-fixture-user"),
                        Path::new("protected.pdf"),
                    )
                    .unwrap();
                assert_eq!(
                    reopened.measurement_records(0).unwrap()[0]
                        .measurement
                        .value()
                        .unwrap(),
                    18.0
                );
                assert!(!reopened.properties().can_copy);
            } else {
                assert_same(&before, &doc.snapshot().unwrap());
            }
        }
    }
}
