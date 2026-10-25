use lopdf::Object;
use std::path::Path;
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{
        dimensioned_ellipse_points, DimensionedEllipse, Measurement, MeasurementKind,
        MeasurementUnit, MeasurementValidity, MeasurementViewport, RectangleScale,
        DIMENSIONED_ELLIPSE_SEGMENTS,
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

fn ellipse(scale: RectangleScale) -> DimensionedEllipse {
    DimensionedEllipse {
        center: (220.125, 260.375),
        diameter_x: 0.8,
        diameter_y: 1.1,
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

fn check_geometry(measurement: &Measurement, request: &DimensionedEllipse) {
    assert_eq!(measurement.kind, MeasurementKind::Area);
    assert_eq!(measurement.points.len(), 128);
    assert!(measurement.holes.is_empty());
    assert!(measurement.depth.is_none());
    let distance = |a: (f32, f32), b: (f32, f32)| {
        (f64::from(a.0) - f64::from(b.0)).hypot(f64::from(a.1) - f64::from(b.1))
    };
    let scale = measurement.known_distance
        / distance(measurement.calibration[0], measurement.calibration[1]);
    let points = &measurement.points;
    close(distance(points[0], points[64]) * scale, request.diameter_x);
    close(distance(points[32], points[96]) * scale, request.diameter_y);
    let angle = request.rotation_degrees.to_radians();
    let (s, c) = angle.sin_cos();
    for (index, &(x, y)) in points.iter().enumerate() {
        let t = std::f64::consts::TAU * index as f64 / 128.0;
        let expected_x = request.center.0 as f64
            + (request.diameter_x * t.cos() * c + request.diameter_y * t.sin() * s) / (2.0 * scale);
        let expected_y = request.center.1 as f64
            + (-request.diameter_x * t.cos() * s + request.diameter_y * t.sin() * c)
                / (2.0 * scale);
        assert!((f64::from(x) - expected_x).abs() <= 0.001);
        assert!((f64::from(y) - expected_y).abs() <= 0.001);
    }
    // Independent fan triangulation from the saved points verifies the exported quantity.
    let signed_area = (0..128)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % 128];
            (f64::from(a.0) - f64::from(request.center.0))
                * (f64::from(b.1) - f64::from(request.center.1))
                - (f64::from(a.1) - f64::from(request.center.1))
                    * (f64::from(b.0) - f64::from(request.center.0))
        })
        .sum::<f64>()
        * 0.5
        * scale
        * scale;
    assert!(signed_area > 0.0);
    close(measurement.value().unwrap(), signed_area);
    let analytic = std::f64::consts::PI * request.diameter_x * request.diameter_y / 4.0;
    let ratio = measurement.value().unwrap() / analytic;
    assert!(
        (0.99957..0.99963).contains(&ratio),
        "area must be the polygon, not pi*a*b: {ratio}"
    );
}

#[test]
fn sampled_outline_has_exact_cardinal_axes_and_refuses_invalid_geometry() {
    assert_eq!(DIMENSIONED_ELLIPSE_SEGMENTS, 128);
    let base = dimensioned_ellipse_points((100.0, 120.0), 80.0, 40.0, 0.0, (300.0, 300.0)).unwrap();
    assert_eq!(
        [base[0], base[32], base[64], base[96]],
        [
            (140.0, 120.0),
            (100.0, 140.0),
            (60.0, 120.0),
            (100.0, 100.0)
        ]
    );
    for (degrees, first) in [
        (90.0, (100.0, 80.0)),
        (180.0, (60.0, 120.0)),
        (270.0, (100.0, 160.0)),
        (360.0, base[0]),
    ] {
        let points =
            dimensioned_ellipse_points((100.0, 120.0), 80.0, 40.0, degrees, (300.0, 300.0))
                .unwrap();
        assert_eq!(points[0], first);
        assert_eq!(
            points,
            dimensioned_ellipse_points((100.0, 120.0), 80.0, 40.0, degrees - 720.0, (300.0, 300.0))
                .unwrap()
        );
    }
    for degrees in [33.5, -326.5, 393.5, 1e300, -1e300] {
        assert!(
            dimensioned_ellipse_points((100.0, 120.0), 80.0, 40.0, degrees, (300.0, 300.0)).is_ok()
        );
    }
    for (center, dx, dy, angle) in [
        ((100.0, 120.0), 0.0, 40.0, 0.0),
        ((100.0, 120.0), 0.1, 0.1, 0.0),
        ((100.0, 120.0), 80.0, 0.0001, 0.0),
        ((100.0, 120.0), f64::INFINITY, 40.0, 0.0),
        ((100.0, 120.0), 80.0, f64::NAN, 0.0),
        ((100.0, 120.0), 80.0, 40.0, f64::NAN),
        ((0.0, 120.0), 80.0, 40.0, 0.0),
        ((f32::NAN, 120.0), 80.0, 40.0, 0.0),
    ] {
        assert!(dimensioned_ellipse_points(center, dx, dy, angle, (300.0, 300.0)).is_err());
    }
}

// One engine owns every PDF case in this integration binary, including encrypted sources.
#[test]
fn dimensioned_ellipse_pdf_lifecycle_and_atomic_refusals() {
    let engine = Engine::new().unwrap();
    lifecycle(&engine);
    scale_and_stale_refusals(&engine);
    invalid_geometry_and_storage(&engine);
    encrypted_sources(&engine);
}

fn lifecycle(engine: &Engine) {
    let temporary = tempfile::tempdir().unwrap();
    for page_rotation in [0, 90, 180, 270] {
        let source = modified(SOURCE, |pdf| {
            let page = pdf.get_pages()[&1];
            let page = pdf.get_dictionary_mut(page).unwrap();
            page.set(
                "MediaBox",
                vec![0.into(), 0.into(), 900.into(), 1000.into()],
            );
            page.set(
                "CropBox",
                vec![100.25.into(), 50.5.into(), 800.25.into(), 950.5.into()],
            );
            page.set("Rotate", page_rotation);
        });
        let mut doc = engine
            .open_bytes(source, None, Path::new("cropped.pdf"))
            .unwrap();
        let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
        let before = doc.snapshot().unwrap();
        for circular in [false, true] {
            let mut request = ellipse(RectangleScale::Local(reference.clone()));
            if circular {
                request.diameter_y = request.diameter_x;
            }
            for unit in MeasurementUnit::ALL {
                let mut converted = request.clone();
                converted.unit = unit;
                converted.diameter_x /= unit.metres();
                converted.diameter_y /= unit.metres();
                let actual = doc.dimensioned_ellipse(0, &converted).unwrap();
                check_geometry(&actual, &converted);
            }
            assert_unchanged(&before, &doc.snapshot().unwrap());
            let preview = doc
                .preview_rich_annotation(
                    0,
                    &RichAnnotation::DimensionedEllipse(request.clone()),
                    Some("Plan reviewer"),
                )
                .unwrap();
            assert!(preview
                .image
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel[2] > pixel[0].saturating_add(40)));
            assert_unchanged(&before, &doc.snapshot().unwrap());
            let index = doc
                .commit_rich_annotation_preview(preview.prepared.unwrap())
                .unwrap();
            let records = doc.measurement_records(0).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].index, index);
            assert_eq!(records[0].validity, MeasurementValidity::Current);
            check_geometry(&records[0].measurement, &request);
            let table = doc.quantity_table().unwrap();
            close(
                table.rows()[0].quantity.as_ref().unwrap().value,
                records[0].measurement.value().unwrap(),
            );
            let after = doc.snapshot().unwrap();
            doc.restore(before.clone()).unwrap();
            assert!(doc.measurement_records(0).unwrap().is_empty());
            doc.restore(after).unwrap();
            assert_eq!(doc.measurement_records(0).unwrap(), records);
            let output = temporary
                .path()
                .join(format!("ellipse-{page_rotation}-{circular}.pdf"));
            doc.save_as(&output).unwrap();
            let reopened = engine.open(&output, None).unwrap();
            assert_eq!(reopened.measurement_records(0).unwrap(), records);
            close(
                reopened.quantity_table().unwrap().rows()[0]
                    .quantity
                    .as_ref()
                    .unwrap()
                    .value,
                records[0].measurement.value().unwrap(),
            );
            // The saved result is genuinely editable as a polygon, without a shape constraint.
            let target = doc.measurement_target(0, &records[0]).unwrap();
            let mut edited = records[0].measurement.clone();
            edited.points[0].0 += 0.25;
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
    let mut request = ellipse(RectangleScale::Region("detail".into()));
    let explicit = doc.dimensioned_ellipse(0, &request).unwrap();
    request.scale = RectangleScale::Automatic;
    assert_eq!(doc.dimensioned_ellipse(0, &request).unwrap(), explicit);
    for scale in [
        RectangleScale::Automatic,
        RectangleScale::Region("detail".into()),
    ] {
        request.scale = scale;
        request.diameter_x = 5.0;
        assert!(doc.add_dimensioned_ellipse(0, &request, None).is_err());
        assert_unchanged(&source, &doc.snapshot().unwrap());
    }
    request = ellipse(RectangleScale::Automatic);
    request.center = (160.0, 260.0);
    request.diameter_x = 3.0;
    request.diameter_y = 3.0;
    assert!(
        doc.dimensioned_ellipse(0, &request).is_err(),
        "center is outside but the ring encloses the region"
    );
    request.scale = RectangleScale::Page;
    assert!(
        doc.dimensioned_ellipse(0, &request).is_ok(),
        "explicit page override"
    );
    request.scale = RectangleScale::Region("missing".into());
    assert!(doc.dimensioned_ellipse(0, &request).is_err());
    let overlap = MeasurementViewport {
        id: "overlap".into(),
        name: "Overlap".into(),
        ..region.clone()
    };
    doc.set_measurement_viewports(0, std::slice::from_ref(&region), &[region.clone(), overlap])
        .unwrap();
    assert!(doc
        .dimensioned_ellipse(0, &ellipse(RectangleScale::Automatic))
        .is_err());
    assert!(doc
        .dimensioned_ellipse(0, &ellipse(RectangleScale::Region("detail".into())))
        .is_ok());
    doc.restore(source.clone()).unwrap();
    let local = ellipse(RectangleScale::Local(
        doc.local_calibration_reference(0, &calibration()).unwrap(),
    ));
    let preview = doc
        .preview_rich_annotation(0, &RichAnnotation::DimensionedEllipse(local.clone()), None)
        .unwrap();
    doc.add_dimensioned_ellipse(0, &local, None).unwrap();
    assert!(
        doc.dimensioned_ellipse(0, &local).is_ok(),
        "annotation-only changes retain local calibration"
    );
    assert!(
        doc.commit_rich_annotation_preview(preview.prepared.unwrap())
            .is_err(),
        "prepared source includes annotations"
    );
    assert!(
        doc.dimensioned_ellipse(1, &local).is_err(),
        "local reference belongs to page 0"
    );
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
            .add_dimensioned_ellipse(0, &ellipse(scale), None)
            .is_err());
        assert_unchanged(&changed, &doc.snapshot().unwrap());
    }
    let unsupported = modified(&source, |pdf| {
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page).unwrap().set("UserUnit", 2);
    });
    doc.restore(unsupported).unwrap();
    assert!(doc
        .dimensioned_ellipse(0, &ellipse(RectangleScale::Page))
        .is_err());
}

fn invalid_geometry_and_storage(engine: &Engine) {
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let reference = doc.local_calibration_reference(0, &calibration()).unwrap();
    let before = doc.snapshot().unwrap();
    for (dx, dy, angle, center, precision) in [
        (0.0, 1.1, 0.0, (220.0, 260.0), 2),
        (-1.0, 1.1, 0.0, (220.0, 260.0), 2),
        (0.001, 0.001, 0.0, (220.0, 260.0), 2),
        (0.8, 1e-7, 33.5, (220.0, 260.0), 2),
        (f64::MAX, 1.1, 0.0, (220.0, 260.0), 2),
        (0.8, f64::NAN, 0.0, (220.0, 260.0), 2),
        (0.8, 1.1, f64::INFINITY, (220.0, 260.0), 2),
        (0.8, 1.1, 0.0, (0.0, 0.0), 2),
        (0.8, 1.1, 0.0, (f32::NAN, 260.0), 2),
        (0.8, 1.1, 0.0, (220.0, 260.0), 5),
    ] {
        let mut request = ellipse(RectangleScale::Local(reference.clone()));
        request.diameter_x = dx;
        request.diameter_y = dy;
        request.rotation_degrees = angle;
        request.center = center;
        request.precision = precision;
        assert!(doc.add_dimensioned_ellipse(0, &request, None).is_err());
        assert_unchanged(&before, &doc.snapshot().unwrap());
    }
    let shifted = modified(SOURCE, |pdf| {
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
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
        .dimensioned_ellipse(0, &ellipse(RectangleScale::Local(reference)))
        .is_err());
}

fn encrypted_sources(engine: &Engine) {
    for aes256 in [false, true] {
        for allow_annotate in [false, true] {
            let user = format!("ellipse-user-{}", std::process::id());
            let owner = format!("ellipse-owner-{}", std::process::id());
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
                    doc.dimensioned_ellipse(0, &ellipse(RectangleScale::Page)),
                    Err(EngineError::SecurityRestricted)
                ));
                assert!(doc.measurement_records(0).unwrap().is_empty());
                continue;
            }
            let request = ellipse(RectangleScale::Local(
                doc.local_calibration_reference(0, &calibration()).unwrap(),
            ));
            let preview = doc
                .preview_rich_annotation(
                    0,
                    &RichAnnotation::DimensionedEllipse(request.clone()),
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
