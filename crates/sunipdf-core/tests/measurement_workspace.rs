use lopdf::{dictionary, Object, Stream};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    measurement::{
        select_viewport, Measurement, MeasurementKind, MeasurementUnit, MeasurementViewport,
        PageCalibration, RecalibrationRequest, SnapKind,
    },
    security, Engine, EngineError, PointRect, Protection, Rotation,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();
fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}
fn scale(distance: f64) -> PageCalibration {
    PageCalibration {
        points: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: distance,
        unit: MeasurementUnit::Metres,
    }
}
fn measurement() -> Measurement {
    Measurement {
        holes: Vec::new(),
        depth: None,
        kind: MeasurementKind::Length,
        points: vec![(40.0, 80.0), (140.0, 80.0)],
        calibration: scale(10.0).points,
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}
fn region(id: &str, distance: f64) -> MeasurementViewport {
    MeasurementViewport {
        id: id.into(),
        name: format!("Detail {id}"),
        bounds: PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 200.0,
            top: 200.0,
        },
        calibration: scale(distance),
    }
}
fn mutate(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    change(&mut pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn credential() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).unwrap();
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn unchanged(before: &[u8], after: &[u8]) {
    let normalize = |b: &[u8]| {
        let mut p = lopdf::Document::load_mem(b).unwrap();
        p.trailer.remove(b"ID");
        for o in p.objects.values_mut() {
            if let Object::Stream(s) = o {
                s.start_position = None;
                s.allows_compression = true;
            }
        }
        (p.objects, p.trailer)
    };
    assert_eq!(normalize(before), normalize(after));
}

#[test]
fn multi_page_preview_is_read_only_and_commit_is_one_reversible_revision() {
    let e = engine();
    let mut doc = e
        .open_bytes(SOURCE.to_vec(), None, Path::new("batch.pdf"))
        .unwrap();
    for page in 0..2 {
        doc.add_measurement_annotation(page, &measurement(), Some("Plan reviewer"))
            .unwrap();
    }
    doc.add_reply(0, 0, "Keep this review", Some("Inspector"))
        .unwrap();
    let records0 = doc.measurement_records(0).unwrap();
    let records1 = doc.measurement_records(1).unwrap();
    let before = doc.snapshot().unwrap();
    let requests = vec![
        RecalibrationRequest {
            page: 0,
            calibration: scale(20.0),
            update_page_default: true,
            measurements: records0.clone(),
        },
        RecalibrationRequest {
            page: 1,
            calibration: scale(30.0),
            update_page_default: false,
            measurements: records1.clone(),
        },
    ];
    let plan = doc.preview_recalibration(&requests).unwrap();
    assert_eq!(plan.page_count(), 2);
    assert_eq!(plan.changes().len(), 2);
    assert_eq!(
        (plan.changes()[0].before, plan.changes()[0].after),
        (10.0, 20.0)
    );
    unchanged(&before, &doc.snapshot().unwrap());
    doc.apply_recalibration(&plan).unwrap();
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
        30.0
    );
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(20.0)));
    assert_eq!(doc.page_calibration(1).unwrap(), None);
    assert_eq!(
        doc.measurement_records(0).unwrap()[0].annotation_id,
        records0[0].annotation_id
    );
    let pdf = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
    assert!(pdf.objects.values().any(|o| o
        .as_dict()
        .ok()
        .and_then(|d| d.get(b"Contents").ok())
        .and_then(|v| lopdf::decode_text_string(v).ok())
        .is_some_and(|text| text == "Keep this review")));
    let saved = doc.snapshot().unwrap();
    let reopened = e
        .open_bytes(saved.clone(), None, Path::new("saved.pdf"))
        .unwrap();
    assert_eq!(
        reopened.measurement_records(1).unwrap()[0]
            .measurement
            .value()
            .unwrap(),
        30.0
    );
    assert!(doc.apply_recalibration(&plan).is_err());
    unchanged(&saved, &doc.snapshot().unwrap());
    doc.restore(before).unwrap();
    assert_eq!(doc.measurement_records(0).unwrap(), records0);
    assert_eq!(doc.measurement_records(1).unwrap(), records1);
}

#[test]
fn bad_later_page_stale_record_and_unrelated_change_never_partially_recalibrate() {
    let e = engine();
    let mut doc = e
        .open_bytes(SOURCE.to_vec(), None, Path::new("atomic.pdf"))
        .unwrap();
    for page in 0..2 {
        doc.add_measurement_annotation(page, &measurement(), None)
            .unwrap();
    }
    let records = doc.measurement_records(0).unwrap();
    let first = RecalibrationRequest {
        page: 0,
        calibration: scale(20.0),
        update_page_default: true,
        measurements: records,
    };
    let before = doc.snapshot().unwrap();
    let mut bad = RecalibrationRequest {
        page: 1,
        calibration: scale(30.0),
        update_page_default: true,
        measurements: doc.measurement_records(1).unwrap(),
    };
    bad.calibration.points[1] = (1e6, 40.0);
    assert!(doc.preview_recalibration(&[first.clone(), bad]).is_err());
    unchanged(&before, &doc.snapshot().unwrap());
    let mut stale = first.clone();
    stale.measurements[0].measurement.known_distance = 99.0;
    assert!(doc.preview_recalibration(&[stale]).is_err());
    assert!(doc
        .preview_recalibration(&[first.clone(), first.clone()])
        .is_err());
    let plan = doc.preview_recalibration(&[first]).unwrap();
    doc.set_page_calibration(1, Some(&scale(90.0))).unwrap();
    let changed = doc.snapshot().unwrap();
    assert!(doc.apply_recalibration(&plan).is_err());
    unchanged(&changed, &doc.snapshot().unwrap());
    assert_eq!(
        doc.measurement_records(0).unwrap()[0]
            .measurement
            .value()
            .unwrap(),
        10.0
    );
}

#[test]
fn batch_preserves_annotation_units_and_squared_area_scale() {
    let e = engine();
    let mut doc = e
        .open_bytes(SOURCE.to_vec(), None, Path::new("area.pdf"))
        .unwrap();
    let mut m = measurement();
    m.kind = MeasurementKind::Area;
    m.points = vec![(40.0, 80.0), (140.0, 80.0), (140.0, 180.0), (40.0, 180.0)];
    m.unit = MeasurementUnit::Feet;
    m.known_distance = 10.0 / 0.3048;
    doc.add_measurement_annotation(0, &m, None).unwrap();
    let records = doc.measurement_records(0).unwrap();
    let plan = doc
        .preview_recalibration(&[RecalibrationRequest {
            page: 0,
            calibration: scale(20.0),
            update_page_default: false,
            measurements: records,
        }])
        .unwrap();
    assert!((plan.changes()[0].after / plan.changes()[0].before - 4.0).abs() < 1e-10);
    doc.apply_recalibration(&plan).unwrap();
    let saved = doc.measurement_records(0).unwrap();
    assert_eq!(saved[0].measurement.unit, MeasurementUnit::Feet);
    assert!((saved[0].measurement.known_distance - 20.0 / 0.3048).abs() < 1e-10);
}

#[test]
fn calibrated_regions_survive_all_rotations_and_preserve_existing_annotation_scales() {
    let e = engine();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let source = mutate(SOURCE, |pdf| {
            let page = pdf.get_pages()[&1];
            pdf.get_dictionary_mut(page).unwrap().set(
                "CropBox",
                vec![10.into(), 15.into(), 500.into(), 650.into()],
            );
        });
        let mut doc = e
            .open_bytes(source, None, Path::new("regions.pdf"))
            .unwrap();
        let original = region("a", 10.0);
        doc.set_measurement_viewports(0, &[], std::slice::from_ref(&original))
            .unwrap();
        doc.add_measurement_annotation(0, &measurement(), None)
            .unwrap();
        let initial_value = doc.measurement_records(0).unwrap()[0]
            .measurement
            .value()
            .unwrap();
        let initial_geo = doc.page_geometry(0).unwrap();
        let user_bounds = initial_geo.rect_to_user(original.bounds);
        doc.set_page_rotation(0, rotation).unwrap();
        let geo = doc.page_geometry(0).unwrap();
        let current = doc.measurement_viewports(0).unwrap();
        assert_eq!(current[0].bounds, geo.rect_to_display(user_bounds));
        assert_eq!(
            current[0].calibration.points,
            original.calibration.points.map(|(x, y)| {
                let (x, y) = initial_geo.to_user(x, y);
                geo.to_display(x, y)
            })
        );
        let mut changed = current.clone();
        changed[0].calibration.known_distance = 100.0;
        doc.set_measurement_viewports(0, &current, &changed)
            .unwrap();
        assert!(doc.set_measurement_viewports(0, &current, &[]).is_err());
        assert!(
            (doc.measurement_records(0).unwrap()[0]
                .measurement
                .value()
                .unwrap()
                - initial_value)
                .abs()
                < 1e-10
        );
        let reopened = e
            .open_bytes(
                doc.snapshot().unwrap(),
                None,
                Path::new("regions-saved.pdf"),
            )
            .unwrap();
        assert_eq!(reopened.measurement_viewports(0).unwrap(), changed);
    }
}

#[test]
fn overlapping_regions_are_explicit_and_malformed_or_stale_metadata_is_preserved() {
    let e = engine();
    let mut doc = e
        .open_bytes(SOURCE.to_vec(), None, Path::new("overlap.pdf"))
        .unwrap();
    let regions = vec![region("a", 10.0), region("b", 50.0)];
    doc.set_measurement_viewports(0, &[], &regions).unwrap();
    assert!(select_viewport(&regions, &measurement().points, None).is_err());
    assert_eq!(
        select_viewport(&regions, &measurement().points, Some("b"))
            .unwrap()
            .unwrap()
            .calibration
            .known_distance,
        50.0
    );
    assert!(select_viewport(&regions, &[(0.0, 80.0), (250.0, 80.0)], None).is_err());
    let saved = doc.snapshot().unwrap();
    for change in ["version", "unknown", "crop", "content", "nonfinite"] {
        let bytes = mutate(&saved, |pdf| {
            let page = pdf.get_pages()[&1];
            if change == "content" {
                let content =
                    pdf.add_object(Stream::new(dictionary! {}, b"0 0 m 10 10 l S".to_vec()));
                pdf.get_dictionary_mut(page)
                    .unwrap()
                    .set("Contents", content);
                return;
            }
            let dict = pdf.get_dictionary_mut(page).unwrap();
            if change == "crop" {
                dict.set("CropBox", vec![0.into(), 0.into(), 300.into(), 300.into()]);
                return;
            }
            let mut v: serde_json::Value = serde_json::from_slice(
                dict.get(b"SuniPDFMeasurementViewports")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            match change {
                "version" => v["version"] = 99.into(),
                "unknown" => v["unknown"] = true.into(),
                "nonfinite" => v["regions"][0]["bounds"][0] = serde_json::Value::Null,
                _ => unreachable!(),
            };
            dict.set(
                "SuniPDFMeasurementViewports",
                Object::string_literal(v.to_string()),
            );
        });
        let mut broken = e.open_bytes(bytes, None, Path::new("broken.pdf")).unwrap();
        let before = broken.snapshot().unwrap();
        assert!(broken.measurement_viewports(0).is_err(), "{change}");
        assert!(
            broken.set_measurement_viewports(0, &regions, &[]).is_err(),
            "{change}"
        );
        unchanged(&before, &broken.snapshot().unwrap());
        if matches!(change, "crop" | "content") {
            let target = broken.measurement_viewport_reset_target(0).unwrap();
            broken.clear_measurement_viewports(&target).unwrap();
            assert!(broken.measurement_viewports(0).unwrap().is_empty());
            assert!(broken.clear_measurement_viewports(&target).is_err());
            broken.restore(before).unwrap();
            assert!(broken.measurement_viewports(0).is_err());
        } else {
            assert!(
                broken.measurement_viewport_reset_target(0).is_err(),
                "{change}"
            );
        }
    }
    for bytes in [
        security::strip_metadata(&saved, None).unwrap(),
        doc.sanitized_copy().unwrap().0,
    ] {
        let cleaned = e.open_bytes(bytes, None, Path::new("clean.pdf")).unwrap();
        assert!(cleaned.measurement_viewports(0).unwrap().is_empty());
    }
}

#[test]
fn new_measurement_writes_preserve_aes_and_honor_independent_permissions() {
    let e = engine();
    let mut source = e
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    source
        .add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    for aes256 in [false, true] {
        for (allow_modify, allow_annotate) in [(true, true), (false, true), (true, false)] {
            let user = credential();
            let owner = credential();
            let bytes = security::protect(
                &source.snapshot().unwrap(),
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_modify,
                    allow_annotate,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut doc = e
                .open_bytes(bytes, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            let request = RecalibrationRequest {
                page: 0,
                calibration: scale(20.0),
                update_page_default: false,
                measurements: doc.measurement_records(0).unwrap(),
            };
            if allow_annotate {
                let plan = doc
                    .preview_recalibration(std::slice::from_ref(&request))
                    .unwrap();
                doc.apply_recalibration(&plan).unwrap();
            } else {
                assert!(matches!(
                    doc.preview_recalibration(std::slice::from_ref(&request)),
                    Err(EngineError::SecurityRestricted)
                ));
            }
            if allow_modify {
                doc.set_measurement_viewports(0, &[], &[region("a", 10.0)])
                    .unwrap();
            } else {
                assert!(matches!(
                    doc.set_measurement_viewports(0, &[], &[region("a", 10.0)]),
                    Err(EngineError::SecurityRestricted)
                ));
            }
            let reread = e
                .open_bytes(
                    doc.snapshot().unwrap(),
                    Some(&user),
                    Path::new("encrypted-copy.pdf"),
                )
                .unwrap();
            assert!(reread.properties().encrypted);
            assert_eq!(reread.properties().can_modify, allow_modify);
            assert_eq!(reread.properties().can_annotate, allow_annotate);
        }
    }
}

#[test]
fn signed_documents_refuse_region_and_batch_mutation_even_for_invalid_signatures() {
    let e = engine();
    let bytes = mutate(SOURCE, |pdf| {
        let signature=pdf.add_object(dictionary!{"Type"=>"Sig","Filter"=>"Adobe.PPKLite","SubFilter"=>"adbe.pkcs7.detached","ByteRange"=>vec![0.into(),1.into(),3.into(),1.into()],"Contents"=>Object::String(vec![0x30,0],lopdf::StringFormat::Hexadecimal)});
        let field = pdf.add_object(
            dictionary! {"FT"=>"Sig","T"=>lopdf::text_string("Unverified"),"V"=>signature},
        );
        let form = pdf.add_object(dictionary! {"Fields"=>vec![Object::Reference(field)]});
        pdf.catalog_mut().unwrap().set("AcroForm", form);
    });
    let mut doc = e.open_bytes(bytes, None, Path::new("signed.pdf")).unwrap();
    assert_eq!(doc.signature_count(), 1);
    let before = doc.snapshot().unwrap();
    assert!(doc
        .set_measurement_viewports(0, &[], &[region("a", 10.0)])
        .is_err());
    assert!(doc
        .preview_recalibration(&[RecalibrationRequest {
            page: 0,
            calibration: scale(10.0),
            update_page_default: true,
            measurements: vec![]
        }])
        .is_err());
    unchanged(&before, &doc.snapshot().unwrap());
}

#[test]
fn native_vector_snapping_uses_paths_not_bounds_with_crop_rotation_and_matrix() {
    let e = engine();
    for rotation in [0, 90, 180, 270] {
        let bytes = mutate(SOURCE, |pdf| {
            let content=pdf.add_object(Stream::new(dictionary!{},b"q 2 0 0 2 100 100 cm 0 0 m 100 100 l S 0 100 m 100 0 l S Q 350 350 m 400 350 400 400 350 400 c S".to_vec()));
            let page = pdf.get_pages()[&1];
            let d = pdf.get_dictionary_mut(page).unwrap();
            d.set("Rotate", rotation);
            d.set(
                "CropBox",
                vec![50.into(), 50.into(), 500.into(), 650.into()],
            );
            d.set("Contents", content);
        });
        let doc = e.open_bytes(bytes, None, Path::new("vectors.pdf")).unwrap();
        let geo = doc.page_geometry(0).unwrap();
        let geometry = doc.vector_snap_geometry(0).unwrap();
        let endpoint = geo.to_display(100.0, 100.0);
        assert_eq!(geometry.nearest(endpoint, 0.1).unwrap().point, endpoint);
        assert_eq!(
            geometry.nearest(endpoint, 0.1).unwrap().kind,
            SnapKind::Endpoint
        );
        let crossing = geo.to_display(200.0, 200.0);
        assert_eq!(geometry.nearest(crossing, 0.1).unwrap().point, crossing);
        assert_eq!(
            geometry.nearest(crossing, 0.1).unwrap().kind,
            SnapKind::Intersection
        );
        assert!(
            geometry
                .nearest(geo.to_display(400.0, 350.0), 0.1)
                .is_none(),
            "Bezier handles are not real endpoints"
        );
        assert_eq!(
            geometry
                .nearest(geo.to_display(350.0, 400.0), 0.1)
                .unwrap()
                .kind,
            SnapKind::Endpoint
        );
    }
}

#[test]
fn native_nested_form_paths_use_composed_transforms_and_clipped_content_is_not_guessed() {
    let e = engine();
    let bytes = mutate(SOURCE, |pdf| {
        let inner=pdf.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),100.into(),100.into()],"Resources"=>dictionary!{}},b"10 10 m 90 90 l S 10 90 m 90 10 l S".to_vec()));
        let outer=pdf.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),300.into(),300.into()],"Resources"=>dictionary!{"XObject"=>dictionary!{"Inner"=>inner}}},b"q 2 0 0 2 10 20 cm /Inner Do Q".to_vec()));
        let contents = pdf.add_object(Stream::new(
            dictionary! {},
            b"q 1 0 0 1 100 200 cm /Outer Do Q q 0 0 20 20 re W n 300 300 m 400 400 l S Q".to_vec(),
        ));
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Contents", contents);
        page.set(
            "Resources",
            dictionary! {"XObject"=>dictionary!{"Outer"=>outer}},
        );
    });
    let doc = e.open_bytes(bytes, None, Path::new("nested.pdf")).unwrap();
    let geometry = doc.vector_snap_geometry(0).unwrap();
    assert_eq!(
        geometry.nearest((130.0, 240.0), 0.1).unwrap().kind,
        SnapKind::Endpoint
    );
    assert_eq!(
        geometry.nearest((210.0, 320.0), 0.1).unwrap().kind,
        SnapKind::Intersection
    );
    assert!(geometry.nearest((400.0, 400.0), 0.1).is_none());
}

#[test]
fn transparent_vector_paint_never_offers_an_invisible_snap() {
    let e = engine();
    let bytes = mutate(SOURCE, |pdf| {
        let content = pdf.add_object(Stream::new(
            dictionary! {},
            b"q /Invisible gs 10 10 m 200 200 l S 10 200 m 200 10 l S Q 300 300 m 400 400 l S"
                .to_vec(),
        ));
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Contents", content);
        page.set("Resources",dictionary!{"ExtGState"=>dictionary!{"Invisible"=>dictionary!{"Type"=>"ExtGState","CA"=>0,"ca"=>0}}});
    });
    let doc = e
        .open_bytes(bytes, None, Path::new("transparent.pdf"))
        .unwrap();
    let geometry = doc.vector_snap_geometry(0).unwrap();
    assert!(geometry.nearest((10.0, 10.0), 0.1).is_none());
    assert!(geometry.nearest((105.0, 105.0), 0.1).is_none());
    assert_eq!(
        geometry.nearest((300.0, 300.0), 0.1).unwrap().kind,
        SnapKind::Endpoint
    );
}
