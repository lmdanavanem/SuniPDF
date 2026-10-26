use lopdf::Object;
use std::path::Path;
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{
        dimensioned_segment_points, DimensionedSegment, Measurement, MeasurementKind,
        MeasurementUnit, MeasurementValidity, MeasurementViewport, RectangleScale,
    },
    security, Engine, EngineError, PageCalibration, PointRect, Protection, TextStyle,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");

fn calibration() -> PageCalibration {
    PageCalibration {
        points: [(40.25, 40.5), (140.25, 40.5)],
        known_distance: 100.0,
        unit: MeasurementUnit::Centimetres,
    }
}

fn segment(scale: RectangleScale) -> DimensionedSegment {
    DimensionedSegment {
        start: (220.125, 260.375),
        length: 0.8,
        rotation_degrees: 33.5,
        unit: MeasurementUnit::Metres,
        precision: 4,
        scale,
    }
}

fn modified(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    change(&mut pdf);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

fn assert_unchanged(before: &[u8], after: &[u8]) {
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

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 2e-5,
        "{actual} != {expected}"
    );
}

fn check_geometry(measurement: &Measurement, request: &DimensionedSegment) {
    assert_eq!(measurement.kind, MeasurementKind::Length);
    assert_eq!(measurement.points.len(), 2);
    assert!(measurement.holes.is_empty());
    assert!(measurement.depth.is_none());
    let start = measurement.points[0];
    let end = measurement.points[1];
    let reference = (f64::from(measurement.calibration[1].0)
        - f64::from(measurement.calibration[0].0))
    .hypot(f64::from(measurement.calibration[1].1) - f64::from(measurement.calibration[0].1));
    let scale = measurement.known_distance / reference;
    let dx = f64::from(end.0) - f64::from(start.0);
    let dy = f64::from(end.1) - f64::from(start.1);
    close(dx.hypot(dy) * scale, request.length);
    close(measurement.value().unwrap(), request.length);
    let angle = request.rotation_degrees.to_radians();
    assert!((dx * scale - request.length * angle.cos()).abs() <= request.length * 2e-5);
    assert!((dy * scale + request.length * angle.sin()).abs() <= request.length * 2e-5);
    assert!((start.0 - request.start.0).abs() < 0.001);
    assert!((start.1 - request.start.1).abs() < 0.001);
}

#[test]
fn segment_endpoints_preserve_cardinals_and_reject_unrepresentable_geometry() {
    for (angle, end) in [
        (0.0, (160.0, 120.0)),
        (90.0, (100.0, 60.0)),
        (180.0, (40.0, 120.0)),
        (270.0, (100.0, 180.0)),
        (-90.0, (100.0, 180.0)),
        (450.0, (100.0, 60.0)),
    ] {
        assert_eq!(
            dimensioned_segment_points((100.0, 120.0), 60.0, angle, (300.0, 300.0)).unwrap(),
            [(100.0, 120.0), end]
        );
    }
    for angle in [33.5, -27.0, 393.5, 1e20] {
        let pts = dimensioned_segment_points((100.0, 120.0), 60.0, angle, (300.0, 300.0)).unwrap();
        close(
            (f64::from(pts[1].0) - 100.0).hypot(f64::from(pts[1].1) - 120.0),
            60.0,
        );
    }
    for length in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX, 1e-12] {
        assert!(dimensioned_segment_points((100.0, 120.0), length, 33.5, (300.0, 300.0)).is_err());
    }
    for angle in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(dimensioned_segment_points((100.0, 120.0), 60.0, angle, (300.0, 300.0)).is_err());
    }
    assert!(dimensioned_segment_points((299.0, 120.0), 60.0, 0.0, (300.0, 300.0)).is_err());
    assert!(dimensioned_segment_points(
        (1_000_000.0, 1_000_000.0),
        0.1,
        33.5,
        (2_000_000.0, 2_000_000.0)
    )
    .is_err());
    assert!(dimensioned_segment_points((f32::NAN, 120.0), 60.0, 0.0, (300.0, 300.0)).is_err());
}

// A single native engine owns all document cases in this integration process.
#[test]
fn segment_lifecycle_references_storage_and_encrypted_permissions() {
    let engine = Engine::new().unwrap();
    lifecycle(&engine);
    scale_and_stale_refusals(&engine);
    invalid_geometry(&engine);
    encrypted_sources(&engine);
}

fn lifecycle(engine: &Engine) {
    let temporary = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let source = modified(SOURCE, |pdf| {
            let id = pdf.get_pages()[&1];
            let page = pdf.get_dictionary_mut(id).unwrap();
            page.set(
                "MediaBox",
                vec![0.into(), 0.into(), 900.into(), 1000.into()],
            );
            page.set(
                "CropBox",
                vec![100.25.into(), 50.5.into(), 800.25.into(), 950.5.into()],
            );
            page.set("Rotate", rotation);
        });
        let mut doc = engine
            .open_bytes(source, None, Path::new("cropped.pdf"))
            .unwrap();
        let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
        let before = doc.snapshot().unwrap();
        for angle in [0.0, 33.5, 90.0, -27.0, 180.0, 270.0] {
            let mut request = segment(RectangleScale::Local(reference.clone()));
            request.rotation_degrees = angle;
            for unit in MeasurementUnit::ALL {
                let mut converted = request.clone();
                converted.unit = unit;
                converted.length /= unit.metres();
                converted.precision = 2;
                check_geometry(&doc.dimensioned_segment(0, &converted).unwrap(), &converted);
            }
            if rotation == 180 && angle == -27.0 {
                let mut too_precise = request.clone();
                too_precise.unit = MeasurementUnit::Millimetres;
                too_precise.length = 800.0;
                assert!(
                    doc.dimensioned_segment(0, &too_precise).is_err(),
                    "four-decimal label changes during user/display f32 storage"
                );
                assert_unchanged(&before, &doc.snapshot().unwrap());
            }
            let preview = doc
                .preview_rich_annotation(
                    0,
                    &RichAnnotation::DimensionedSegment(request.clone()),
                    Some("Plan reviewer"),
                )
                .unwrap();
            assert!(preview
                .image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|px| px[2] > px[0].saturating_add(40)));
            assert_unchanged(&before, &doc.snapshot().unwrap());
            doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                .unwrap();
            let records = doc.measurement_records(0).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].validity, MeasurementValidity::Current);
            check_geometry(&records[0].measurement, &request);
            close(
                doc.quantity_table().unwrap().rows()[0]
                    .quantity
                    .as_ref()
                    .unwrap()
                    .value,
                request.length,
            );
            let after = doc.snapshot().unwrap();
            doc.restore(before.clone()).unwrap();
            assert!(doc.measurement_records(0).unwrap().is_empty());
            doc.restore(after).unwrap();
            assert_eq!(doc.measurement_records(0).unwrap(), records);
            let out = temporary
                .path()
                .join(format!("segment-{rotation}-{angle}.pdf"));
            doc.save_as(&out).unwrap();
            let reopened = engine.open(&out, None).unwrap();
            assert_eq!(reopened.measurement_records(0).unwrap(), records);
            let target = doc.measurement_target(0, &records[0]).unwrap();
            let mut edited = records[0].measurement.clone();
            edited.points[1].0 += 0.25;
            doc.update_measurement_annotation(&target, &edited).unwrap();
            let updated = doc.measurement_records(0).unwrap();
            assert_eq!(updated[0].annotation_id, records[0].annotation_id);
            assert_ne!(
                updated[0].measurement.value().unwrap(),
                records[0].measurement.value().unwrap()
            );
            doc.restore(before.clone()).unwrap();
        }
    }
}

fn scale_and_stale_refusals(engine: &Engine) {
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("scales.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&calibration())).unwrap();
    let region = MeasurementViewport {
        id: "detail".into(),
        name: "Detail".into(),
        bounds: PointRect {
            left: 180.0,
            bottom: 220.0,
            right: 270.0,
            top: 310.0,
        },
        calibration: PageCalibration {
            points: [(190.0, 230.0), (240.0, 230.0)],
            known_distance: 1.0,
            unit: MeasurementUnit::Metres,
        },
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&region))
        .unwrap();
    let source = doc.snapshot().unwrap();
    let mut request = segment(RectangleScale::Region("detail".into()));
    let explicit = doc.dimensioned_segment(0, &request).unwrap();
    request.scale = RectangleScale::Automatic;
    assert_eq!(doc.dimensioned_segment(0, &request).unwrap(), explicit);
    request.start = (160.0, 260.0);
    request.rotation_degrees = 0.0;
    request.length = 2.0;
    assert!(
        doc.dimensioned_segment(0, &request).is_err(),
        "both endpoints outside but the segment crosses the region"
    );
    request.scale = RectangleScale::Page;
    assert!(doc.dimensioned_segment(0, &request).is_ok());
    request.scale = RectangleScale::Region("detail".into());
    assert!(doc.dimensioned_segment(0, &request).is_err());
    request.scale = RectangleScale::Region("missing".into());
    assert!(doc.dimensioned_segment(0, &request).is_err());
    assert_unchanged(&source, &doc.snapshot().unwrap());
    let overlap = MeasurementViewport {
        id: "overlap".into(),
        name: "Overlap".into(),
        ..region.clone()
    };
    doc.set_measurement_viewports(0, std::slice::from_ref(&region), &[region.clone(), overlap])
        .unwrap();
    assert!(doc
        .dimensioned_segment(0, &segment(RectangleScale::Automatic))
        .is_err());
    assert!(doc
        .dimensioned_segment(0, &segment(RectangleScale::Region("detail".into())))
        .is_ok());
    doc.restore(source.clone()).unwrap();
    let local = segment(RectangleScale::Local(
        doc.local_calibration_reference(0, &calibration()).unwrap(),
    ));
    let preview = doc
        .preview_rich_annotation(0, &RichAnnotation::DimensionedSegment(local.clone()), None)
        .unwrap();
    doc.add_dimensioned_segment(0, &local, None).unwrap();
    assert!(doc.dimensioned_segment(0, &local).is_ok());
    assert!(doc
        .commit_rich_annotation_preview(preview.prepared.unwrap())
        .is_err());
    assert!(doc.dimensioned_segment(1, &local).is_err());
    doc.add_text(0, 20.0, 500.0, "Revised drawing", &TextStyle::default())
        .unwrap();
    let changed = doc.snapshot().unwrap();
    for scale in [
        local.scale,
        RectangleScale::Page,
        RectangleScale::Automatic,
        RectangleScale::Region("detail".into()),
    ] {
        assert!(doc
            .add_dimensioned_segment(0, &segment(scale), None)
            .is_err());
        assert_unchanged(&changed, &doc.snapshot().unwrap());
    }
    let unsupported = modified(&source, |pdf| {
        let id = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(id).unwrap().set("UserUnit", 2);
    });
    doc.restore(unsupported).unwrap();
    assert!(doc
        .dimensioned_segment(0, &segment(RectangleScale::Page))
        .is_err());
}

fn invalid_geometry(engine: &Engine) {
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
    let before = doc.snapshot().unwrap();
    for (length, angle, start, precision) in [
        (0.0, 0.0, (220.0, 260.0), 2),
        (-1.0, 0.0, (220.0, 260.0), 2),
        (1e-12, 0.0, (220.0, 260.0), 2),
        (f64::MAX, 0.0, (220.0, 260.0), 2),
        (f64::NAN, 0.0, (220.0, 260.0), 2),
        (0.8, f64::INFINITY, (220.0, 260.0), 2),
        (0.8, 90.0, (0.0, 0.0), 2),
        (0.8, 0.0, (f32::NAN, 260.0), 2),
        (0.8, 0.0, (220.0, 260.0), 5),
    ] {
        let mut request = segment(RectangleScale::Local(reference.clone()));
        request.length = length;
        request.rotation_degrees = angle;
        request.start = start;
        request.precision = precision;
        assert!(doc.add_dimensioned_segment(0, &request, None).is_err());
        assert_unchanged(&before, &doc.snapshot().unwrap());
    }
    let shifted = modified(SOURCE, |pdf| {
        let id = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(id).unwrap();
        let bounds: Vec<Object> = [1_000_000, 1_000_000, 1_000_600, 1_000_800]
            .into_iter()
            .map(Object::Integer)
            .collect();
        page.set("MediaBox", bounds.clone());
        page.set("CropBox", bounds);
    });
    doc.restore(shifted).unwrap();
    let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
    assert!(doc
        .dimensioned_segment(0, &segment(RectangleScale::Local(reference)))
        .is_err());
}

fn encrypted_sources(engine: &Engine) {
    for aes256 in [false, true] {
        for allow_annotate in [false, true] {
            let user = format!("segment-user-{}", std::process::id());
            let owner = format!("segment-owner-{}", std::process::id());
            let protection = Protection {
                user_password: user.clone(),
                owner_password: owner,
                aes256,
                allow_copy: false,
                allow_modify: false,
                allow_annotate,
                ..Default::default()
            };
            let bytes = security::protect(SOURCE, None, &protection).unwrap();
            let mut doc = engine
                .open_bytes(bytes, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            if !allow_annotate {
                assert!(matches!(
                    doc.dimensioned_segment(0, &segment(RectangleScale::Page)),
                    Err(EngineError::SecurityRestricted)
                ));
                assert!(doc.measurement_records(0).unwrap().is_empty());
                continue;
            }
            let request = segment(RectangleScale::Local(
                doc.local_calibration_reference(0, &calibration()).unwrap(),
            ));
            let preview = doc
                .preview_rich_annotation(
                    0,
                    &RichAnnotation::DimensionedSegment(request.clone()),
                    None,
                )
                .unwrap();
            doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                .unwrap();
            let reopened = engine
                .open_bytes(
                    doc.snapshot().unwrap(),
                    Some(&user),
                    Path::new("reopened.pdf"),
                )
                .unwrap();
            assert!(reopened.properties().encrypted);
            assert!(!reopened.properties().can_copy);
            assert!(!reopened.properties().can_modify);
            check_geometry(
                &reopened.measurement_records(0).unwrap()[0].measurement,
                &request,
            );
        }
    }
}
