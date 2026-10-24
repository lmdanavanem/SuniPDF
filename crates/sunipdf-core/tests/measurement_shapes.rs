use lopdf::Object;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{
        DimensionedRectangle, Measurement, MeasurementKind, MeasurementUnit, MeasurementValidity,
        MeasurementViewport, RectangleScale,
    },
    quantity_table::{FormulaColumn, QuantityOptions, QuantityUnit},
    security, Engine, EngineError, PageCalibration, PageMargins, PointRect, Protection, Rotation,
    TextStyle,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn calibration() -> PageCalibration {
    PageCalibration {
        points: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 100.0,
        unit: MeasurementUnit::Centimetres,
    }
}

fn rectangle(scale: RectangleScale) -> DimensionedRectangle {
    DimensionedRectangle {
        anchor: (40.0, 80.0),
        width: 2.0,
        height: 3.0,
        unit: MeasurementUnit::Metres,
        precision: 4,
        scale,
    }
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-5,
        "{actual} != {expected}"
    );
}

fn dimensions(measurement: &Measurement, width: f64, height: f64) {
    assert_eq!(measurement.kind, MeasurementKind::Area);
    assert_eq!(measurement.points.len(), 4);
    let mut side = measurement.clone();
    side.kind = MeasurementKind::Length;
    side.points = vec![measurement.points[0], measurement.points[1]];
    close(side.value().unwrap(), width);
    side.points = vec![measurement.points[0], measurement.points[3]];
    close(side.value().unwrap(), height);
    close(measurement.value().unwrap(), width * height);
}

fn changed(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    change(&mut pdf);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

fn assert_same(before: &[u8], after: &[u8]) {
    let normalize = |bytes: &[u8]| {
        let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
        pdf.trailer.remove(b"ID");
        for object in pdf.objects.values_mut() {
            if let Object::Stream(stream) = object {
                stream.start_position = None;
                stream.allows_compression = true;
            }
        }
        (pdf.objects, pdf.trailer)
    };
    assert_eq!(normalize(before), normalize(after));
}

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn rectangle_dimensions_survive_units_crop_rotation_preview_undo_redo_and_save() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.restore(SOURCE.to_vec()).unwrap();
        doc.set_page_rotation(0, rotation).unwrap();
        doc.crop_page(
            0,
            PageMargins {
                left: 13.0,
                bottom: 17.0,
                right: 19.0,
                top: 11.0,
            },
        )
        .unwrap();
        let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
        let before = doc.snapshot().unwrap();
        for unit in MeasurementUnit::ALL {
            let mut request = rectangle(RectangleScale::Local(reference.clone()));
            request.unit = unit;
            request.width /= unit.metres();
            request.height /= unit.metres();
            let measurement = doc.dimensioned_rectangle(0, &request).unwrap();
            dimensions(&measurement, request.width, request.height);
            assert_eq!(
                measurement.points,
                vec![(40.0, 80.0), (240.0, 80.0), (240.0, 380.0), (40.0, 380.0)]
            );
        }
        assert_same(&before, &doc.snapshot().unwrap());
        let request = rectangle(RectangleScale::Local(reference));
        let preview = doc
            .preview_rich_annotation(
                0,
                &RichAnnotation::DimensionedRectangle(request),
                Some("Reviewer"),
            )
            .unwrap();
        assert!(preview.image.rgba.iter().any(|byte| *byte != 0));
        assert_same(&before, &doc.snapshot().unwrap());
        let index = doc
            .commit_rich_annotation_preview(preview.prepared.unwrap())
            .unwrap();
        let records = doc.measurement_records(0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].index, index);
        assert_eq!(records[0].validity, MeasurementValidity::Current);
        assert!(records[0].annotation_id.is_some());
        dimensions(&records[0].measurement, 2.0, 3.0);
        assert_eq!(
            doc.annotation_details(0)
                .unwrap()
                .into_iter()
                .find(|detail| detail.index == index)
                .unwrap()
                .author
                .as_deref(),
            Some("Reviewer")
        );
        let table = doc.quantity_table().unwrap();
        assert_eq!(table.rows().len(), 1);
        close(table.rows()[0].quantity.as_ref().unwrap().value, 6.0);
        let options = QuantityOptions {
            formulas: vec![FormulaColumn {
                name: "Double".into(),
                expression: "quantity * 2".into(),
                quantity_unit: Some(QuantityUnit::SquareMetres),
                rounding_decimals: 2,
            }],
            ..Default::default()
        };
        assert!(table.csv(&options).unwrap().contains("12.00"));
        let after = doc.snapshot().unwrap();
        doc.restore(before).unwrap();
        assert!(doc.measurement_records(0).unwrap().is_empty());
        doc.restore(after).unwrap();
        assert_eq!(doc.measurement_records(0).unwrap(), records);
        let output = temp.path().join(format!("rectangle-{rotation:?}.pdf"));
        doc.save_as(&output).unwrap();
        let reopened = engine.open(&output, None).unwrap();
        assert_eq!(reopened.measurement_records(0).unwrap(), records);
        assert!(reopened.measurement_records(1).unwrap().is_empty());
    }
}

#[test]
fn rectangle_rejects_unrepresentable_dimensions_and_invalid_references_without_mutation() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
    let before = doc.snapshot().unwrap();
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX, 1e-20] {
        for width in [true, false] {
            let mut request = rectangle(RectangleScale::Local(reference.clone()));
            if width {
                request.width = value;
            } else {
                request.height = value;
            }
            assert!(doc.add_dimensioned_rectangle(0, &request, None).is_err());
            assert_same(&before, &doc.snapshot().unwrap());
        }
    }
    for anchor in [
        (f32::NAN, 10.0),
        (10.0, f32::INFINITY),
        (-1.0, 10.0),
        (590.0, 800.0),
    ] {
        let mut request = rectangle(RectangleScale::Local(reference.clone()));
        request.anchor = anchor;
        assert!(doc.add_dimensioned_rectangle(0, &request, None).is_err());
        assert_same(&before, &doc.snapshot().unwrap());
    }
    let mut request = rectangle(RectangleScale::Local(reference.clone()));
    request.precision = 5;
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
    let mut invalid = calibration();
    invalid.points[1] = invalid.points[0];
    assert!(doc.local_calibration_reference(0, &invalid).is_err());
    invalid = calibration();
    invalid.points[1].0 = 1000.0;
    assert!(doc.local_calibration_reference(0, &invalid).is_err());
    let mut fractional = rectangle(RectangleScale::Local(reference.clone()));
    fractional.width = 1.2345;
    fractional.height = 2.3456;
    dimensions(
        &doc.dimensioned_rectangle(0, &fractional).unwrap(),
        1.2345,
        2.3456,
    );
    // A width above the 0.01 pt geometry floor can still lose too much relative precision at
    // this anchor. Reject it rather than advertising dimensions the stored vertices cannot keep.
    fractional.width = 0.00010001;
    fractional.anchor.0 = 400.0;
    assert!(doc.dimensioned_rectangle(0, &fractional).is_err());
    assert_same(&before, &doc.snapshot().unwrap());
    assert!(doc
        .dimensioned_rectangle(1, &rectangle(RectangleScale::Local(reference)))
        .is_err());

    // The shape itself has representable sides here, but the short calibration would collapse
    // when translated into user-space f32 coordinates at a large crop origin.
    let shifted = changed(SOURCE, |pdf| {
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        let bounds: Vec<Object> = [1_000_000, 1_000_000, 1_000_600, 1_000_800]
            .into_iter()
            .map(Object::Integer)
            .collect();
        page.set("MediaBox", bounds.clone());
        page.set("CropBox", bounds);
    });
    let shifted = engine
        .open_bytes(shifted, None, Path::new("offset.pdf"))
        .unwrap();
    let narrow = PageCalibration {
        points: [(40.0, 40.0), (40.02, 40.0)],
        known_distance: 0.02,
        unit: MeasurementUnit::Metres,
    };
    let reference = shifted.local_calibration_reference(0, &narrow).unwrap();
    assert!(shifted
        .dimensioned_rectangle(0, &rectangle(RectangleScale::Local(reference)))
        .is_err());
}

#[test]
fn local_reference_reuses_annotation_only_changes_but_rejects_rendering_scale_and_prepared_changes()
{
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
    let request = rectangle(RectangleScale::Local(reference));
    let before = doc.snapshot().unwrap();
    doc.add_dimensioned_rectangle(0, &request, None).unwrap();
    let mut second = request.clone();
    second.anchor = (50.0, 90.0);
    doc.add_dimensioned_rectangle(0, &second, None).unwrap();
    assert_eq!(doc.measurement_records(0).unwrap().len(), 2);
    let preview = doc
        .preview_rich_annotation(
            0,
            &RichAnnotation::DimensionedRectangle(request.clone()),
            None,
        )
        .unwrap();
    doc.delete_annotation_thread(0, 1).unwrap();
    assert!(doc
        .commit_rich_annotation_preview(preview.prepared.unwrap())
        .is_err());
    assert!(doc.dimensioned_rectangle(0, &request).is_ok());
    for change in ["content", "rotation", "crop", "page-scale", "layers"] {
        doc.restore(before.clone()).unwrap();
        match change {
            "content" => {
                doc.add_text(0, 20.0, 400.0, "Revised drawing", &TextStyle::default())
                    .unwrap();
            }
            "rotation" => doc.set_page_rotation(0, Rotation::R90).unwrap(),
            "crop" => {
                doc.crop_page(
                    0,
                    PageMargins {
                        left: 1.0,
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                    },
                )
                .unwrap();
            }
            "page-scale" => doc.set_page_calibration(0, Some(&calibration())).unwrap(),
            "layers" => {
                let bytes = changed(&before, |pdf| {
                    pdf.catalog_mut().unwrap().set("OCProperties", lopdf::dictionary! { "D" => lopdf::dictionary! { "BaseState" => "OFF" }, "OCGs" => Vec::<Object>::new() });
                });
                doc.restore(bytes).unwrap();
            }
            _ => unreachable!(),
        }
        let changed = doc.snapshot().unwrap();
        assert!(
            doc.add_dimensioned_rectangle(0, &request, None).is_err(),
            "{change}"
        );
        assert_same(&changed, &doc.snapshot().unwrap());
        let fresh = doc.local_calibration_reference(0, &calibration()).unwrap();
        assert!(doc
            .dimensioned_rectangle(0, &rectangle(RectangleScale::Local(fresh)))
            .is_ok());
    }
}

#[test]
fn rectangle_scale_selection_checks_all_corners_enclosed_regions_stale_metadata_and_user_unit() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&calibration())).unwrap();
    let region = MeasurementViewport {
        id: "detail".into(),
        name: "Detail".into(),
        bounds: PointRect {
            left: 100.0,
            bottom: 100.0,
            right: 300.0,
            top: 400.0,
        },
        calibration: PageCalibration {
            points: [(120.0, 120.0), (220.0, 120.0)],
            known_distance: 2.0,
            unit: MeasurementUnit::Metres,
        },
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&region))
        .unwrap();
    let source = doc.snapshot().unwrap();
    let mut request = rectangle(RectangleScale::Region("detail".into()));
    request.anchor = (120.0, 160.0);
    let explicit = doc.dimensioned_rectangle(0, &request).unwrap();
    dimensions(&explicit, 2.0, 3.0);
    assert_eq!(explicit.points[2], (220.0, 310.0));
    request.scale = RectangleScale::Automatic;
    assert_eq!(doc.dimensioned_rectangle(0, &request).unwrap(), explicit);
    request.width = 5.0;
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
    request.scale = RectangleScale::Region("detail".into());
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
    request = rectangle(RectangleScale::Automatic);
    request.width = 4.0;
    request.height = 4.0;
    request.anchor = (40.0, 40.0);
    assert!(
        doc.dimensioned_rectangle(0, &request).is_err(),
        "enclosed region must not be missed"
    );
    request.scale = RectangleScale::Page;
    dimensions(&doc.dimensioned_rectangle(0, &request).unwrap(), 4.0, 4.0);
    request.scale = RectangleScale::Region("missing".into());
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
    let overlap = MeasurementViewport {
        id: "overlap".into(),
        name: "Overlap".into(),
        ..region.clone()
    };
    doc.set_measurement_viewports(0, std::slice::from_ref(&region), &[region.clone(), overlap])
        .unwrap();
    request = rectangle(RectangleScale::Automatic);
    request.anchor = (120.0, 160.0);
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
    request.scale = RectangleScale::Region("detail".into());
    assert!(doc.dimensioned_rectangle(0, &request).is_ok());
    doc.restore(source.clone()).unwrap();
    doc.add_text(0, 20.0, 500.0, "Revised drawing", &TextStyle::default())
        .unwrap();
    let stale = doc.snapshot().unwrap();
    for scale in [
        RectangleScale::Page,
        RectangleScale::Region("detail".into()),
        RectangleScale::Automatic,
    ] {
        request.scale = scale;
        assert!(doc.add_dimensioned_rectangle(0, &request, None).is_err());
        assert_same(&stale, &doc.snapshot().unwrap());
    }
    let scaled = changed(&source, |pdf| {
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page).unwrap().set("UserUnit", 2);
    });
    doc.restore(scaled).unwrap();
    assert!(doc.local_calibration_reference(0, &calibration()).is_err());
    request.scale = RectangleScale::Page;
    assert!(doc.dimensioned_rectangle(0, &request).is_err());
}

#[test]
fn rectangle_preserves_encryption_respects_annotation_permission_and_binds_protection() {
    let engine = engine();
    for aes256 in [false, true] {
        for empty_user in [false, true] {
            let user = if empty_user {
                String::new()
            } else {
                credential()
            };
            let owner = credential();
            for allow_annotate in [false, true] {
                let protection = Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_copy: false,
                    allow_modify: false,
                    allow_annotate,
                    ..Default::default()
                };
                let encrypted = security::protect(SOURCE, None, &protection).unwrap();
                let mut doc = engine
                    .open_bytes(encrypted.clone(), Some(&user), Path::new("protected.pdf"))
                    .unwrap();
                if !allow_annotate {
                    assert!(matches!(
                        doc.local_calibration_reference(0, &calibration()),
                        Err(EngineError::SecurityRestricted)
                    ));
                    assert!(matches!(
                        doc.dimensioned_rectangle(0, &rectangle(RectangleScale::Page)),
                        Err(EngineError::SecurityRestricted)
                    ));
                    assert!(doc.measurement_records(0).unwrap().is_empty());
                    continue;
                }
                let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
                let request = rectangle(RectangleScale::Local(reference));
                let preview = doc
                    .preview_rich_annotation(
                        0,
                        &RichAnnotation::DimensionedRectangle(request.clone()),
                        None,
                    )
                    .unwrap();
                assert!(doc.measurement_records(0).unwrap().is_empty());
                doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                    .unwrap();
                assert!(
                    doc.dimensioned_rectangle(0, &request).is_ok(),
                    "annotation-only continuation"
                );
                assert!(!doc.properties().can_copy);
                assert!(!doc.properties().can_modify);
                assert!(doc.properties().encrypted);
                for password in [&user, &owner] {
                    let reopened = engine
                        .open_bytes(
                            doc.snapshot().unwrap(),
                            Some(password),
                            Path::new("reopened.pdf"),
                        )
                        .unwrap();
                    dimensions(
                        &reopened.measurement_records(0).unwrap()[0].measurement,
                        2.0,
                        3.0,
                    );
                    assert!(reopened.properties().encrypted);
                }
                let altered = security::protect(
                    SOURCE,
                    None,
                    &Protection {
                        owner_password: credential(),
                        ..protection
                    },
                )
                .unwrap();
                doc.restore(altered).unwrap();
                assert!(
                    doc.dimensioned_rectangle(0, &request).is_err(),
                    "changed encryption identity cannot reuse a local reference"
                );
            }
        }
    }
}
