use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::Path;
use sunipdf_core::{
    boundary_fill::*,
    measurement::{MeasurementUnit, MeasurementValidity, MeasurementViewport, RectangleScale},
    security, Document, Engine, EngineError, PageCalibration, PageGeometry, PointRect, Protection,
    Rotation,
};

fn fixture(rotation: Rotation, doorway: bool) -> Vec<u8> {
    let geometry = PageGeometry {
        bbox: PointRect {
            left: 100.,
            bottom: 50.,
            right: 260.,
            top: 210.,
        },
        rotation,
    };
    let m = geometry.upright_matrix(0., 0.);
    let mut content = format!(
        "q {} {} {} {} {} {} cm 0 g 20 20 100 100 re f 1 g 22 22 96 96 re f 0 g 60 60 10 10 re f ",
        m[0], m[1], m[2], m[3], m[4], m[5],
    );
    if doorway {
        content.push_str("1 g 20 45 2 10 re f ");
    }
    content.push('Q');
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),400.into(),400.into()],
    "CropBox"=>vec![100.into(),50.into(),260.into(),210.into()],
    "Rotate"=>i64::from(rotation.degrees()),"Resources"=>dictionary!{},"Contents"=>stream});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn calibration() -> PageCalibration {
    PageCalibration {
        points: [(30., 30.), (40., 30.)],
        known_distance: 10.,
        unit: MeasurementUnit::Centimetres,
    }
}
fn options(scale: RectangleScale) -> BoundaryFillMeasurementOptions {
    BoundaryFillMeasurementOptions {
        scale,
        unit: MeasurementUnit::Metres,
        precision: 4,
    }
}
fn scan(doc: &Document<'_>, options: BoundaryFillOptions) -> BoundaryFillJob {
    let mut job = doc.prepare_boundary_fill(0, [40., 40.], options).unwrap();
    while !job.progress().is_terminal() {
        job.step(509);
    }
    job
}
fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn boundary_fill_geometry_review_scales_atomic_roundtrip_and_permissions() {
    // All native handles live on this one test thread.
    let engine = Engine::new().expect("PDFium is required");
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let bytes = fixture(rotation, false);
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.pdf");
        std::fs::write(&path, &bytes).unwrap();
        let mut doc = engine.open(&path, None).unwrap();
        doc.set_page_calibration(0, Some(&calibration())).unwrap();
        let original = doc.snapshot().unwrap();
        let job = scan(&doc, Default::default());
        let report = job.report();
        assert_eq!(
            report.progress.status,
            BoundaryFillStatus::Complete,
            "{rotation:?}: {report:?}"
        );
        assert_eq!(report.geometry.rotation, rotation.degrees());
        assert_eq!(report.geometry.bounds, [100., 50., 260., 210.]);
        let region = report.region.as_ref().unwrap();
        assert_eq!(region.outer.len(), 4);
        assert_eq!(region.holes.len(), 1);
        assert_eq!(region.holes[0].len(), 4);
        close(region.area_points2, 9116.);
        assert_eq!(region.pixel_count, 9116);
        assert!(region
            .outer
            .iter()
            .all(|p| [22., 118.].contains(&p[0]) && [22., 118.].contains(&p[1])));
        assert!(doc.measurement_records(0).unwrap().is_empty());
        let prepared = doc
            .prepare_boundary_fill_measurement(
                &job,
                &options(RectangleScale::Automatic),
                Some("Inspector"),
            )
            .unwrap();
        close(prepared.measurement().value().unwrap(), 0.9116);
        assert!(doc.measurement_records(0).unwrap().is_empty());
        let image = doc
            .render_boundary_fill_measurement_preview(&prepared, 480)
            .unwrap();
        assert_eq!((image.width, image.height), (480, 480));
        assert!(!image.rgba.is_empty());
        doc.verify_boundary_fill(&job).unwrap();
        let index = doc.apply_boundary_fill_measurement(&prepared).unwrap();
        let records = doc.measurement_records(0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].index, index);
        assert_eq!(records[0].validity, MeasurementValidity::Current);
        assert!(records[0].annotation_id.is_some());
        close(records[0].measurement.value().unwrap(), 0.9116);
        let provenance = doc.boundary_fill_provenance(0, index).unwrap().unwrap();
        assert_eq!(provenance.inspection, report);
        assert_eq!(provenance.reference, "automatic");
        assert_eq!(provenance.measurement["unit"], "m2");
        assert_eq!(provenance.measurement["holes"].as_array().unwrap().len(), 1);
        let edited = doc.snapshot().unwrap();
        assert!(
            doc.apply_boundary_fill_measurement(&prepared).is_err(),
            "review cannot be applied twice"
        );
        doc.restore(original.clone()).unwrap();
        assert!(doc.measurement_records(0).unwrap().is_empty());
        doc.restore(edited.clone()).unwrap();
        assert_eq!(doc.measurement_records(0).unwrap(), records);
        let output = temp.path().join("reviewed.pdf");
        std::fs::write(&output, edited).unwrap();
        let reopened = engine.open(&output, None).unwrap();
        assert_eq!(reopened.measurement_records(0).unwrap(), records);
        assert_eq!(
            reopened
                .boundary_fill_provenance(0, index)
                .unwrap()
                .unwrap()
                .inspection,
            report
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "source file is never overwritten"
        );

        doc.restore(original.clone()).unwrap();
        let plain = doc.render_visual_search_content(0, 1.).unwrap();
        doc.add_count_marker(0, (40., 40.), 10., "Existing", None)
            .unwrap();
        assert_eq!(
            doc.render_visual_search_content(0, 1.).unwrap().rgba,
            plain.rgba
        );
        assert_eq!(
            scan(&doc, Default::default()).report().region,
            report.region,
            "annotations do not become region boundaries"
        );
        assert!(doc.verify_boundary_fill(&job).is_err());
        assert!(doc.apply_boundary_fill_measurement(&prepared).is_err());
        doc.restore(original.clone()).unwrap();
        doc.set_page_rotation(0, rotation.turned(90)).unwrap();
        assert!(doc.apply_boundary_fill_measurement(&prepared).is_err());
        doc.restore(original).unwrap();
        let running = doc
            .prepare_boundary_fill(0, [40., 40.], Default::default())
            .unwrap();
        assert!(doc
            .prepare_boundary_fill_measurement(&running, &options(RectangleScale::Page), None)
            .is_err());
        let mut cancelled = running;
        cancelled.cancel();
        assert!(doc
            .prepare_boundary_fill_measurement(&cancelled, &options(RectangleScale::Page), None)
            .is_err());
        let limited = scan(
            &doc,
            BoundaryFillOptions {
                max_work: 1,
                ..Default::default()
            },
        );
        assert_eq!(limited.progress().status, BoundaryFillStatus::WorkLimit);
        assert!(doc
            .prepare_boundary_fill_measurement(&limited, &options(RectangleScale::Page), None)
            .is_err());
    }

    let mut doc = engine
        .open_bytes(
            fixture(Rotation::R0, false),
            None,
            Path::new("references.pdf"),
        )
        .unwrap();
    doc.set_page_calibration(0, Some(&calibration())).unwrap();
    let local = doc.local_calibration_reference(0, &calibration()).unwrap();
    let inner = MeasurementViewport {
        id: "inner".into(),
        name: "Inset".into(),
        bounds: PointRect {
            left: 80.,
            bottom: 80.,
            right: 90.,
            top: 90.,
        },
        calibration: PageCalibration {
            points: [(81., 81.), (89., 81.)],
            known_distance: 8.,
            unit: MeasurementUnit::Metres,
        },
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&inner))
        .unwrap();
    let job = scan(&doc, Default::default());
    assert!(
        doc.prepare_boundary_fill_measurement(&job, &options(RectangleScale::Automatic), None)
            .is_err(),
        "enclosed viewport requires explicit reference even with no outer vertex inside"
    );
    assert!(
        doc.prepare_boundary_fill_measurement(
            &job,
            &options(RectangleScale::Region(inner.id.clone())),
            None
        )
        .is_err(),
        "region must contain all resulting geometry"
    );
    close(
        doc.prepare_boundary_fill_measurement(&job, &options(RectangleScale::Page), None)
            .unwrap()
            .measurement()
            .value()
            .unwrap(),
        0.9116,
    );
    assert!(
        doc.prepare_boundary_fill_measurement(&job, &options(RectangleScale::Local(local)), None)
            .is_err(),
        "changing saved references invalidates captured local reference"
    );
    let outer = MeasurementViewport {
        id: "whole".into(),
        name: "Whole room".into(),
        bounds: PointRect {
            left: 20.,
            bottom: 20.,
            right: 120.,
            top: 120.,
        },
        calibration: calibration(),
    };
    doc.set_measurement_viewports(
        0,
        std::slice::from_ref(&inner),
        &[inner.clone(), outer.clone()],
    )
    .unwrap();
    let job = scan(&doc, Default::default());
    assert!(doc
        .prepare_boundary_fill_measurement(&job, &options(RectangleScale::Automatic), None)
        .is_err());
    let explicit = doc
        .prepare_boundary_fill_measurement(
            &job,
            &options(RectangleScale::Region(outer.id.clone())),
            None,
        )
        .unwrap();
    close(explicit.measurement().value().unwrap(), 0.9116);
    let fresh = doc.local_calibration_reference(0, &calibration()).unwrap();
    close(
        doc.prepare_boundary_fill_measurement(&job, &options(RectangleScale::Local(fresh)), None)
            .unwrap()
            .measurement()
            .value()
            .unwrap(),
        0.9116,
    );

    let doorway = engine
        .open_bytes(
            fixture(Rotation::R0, true),
            None,
            Path::new("open-room.pdf"),
        )
        .unwrap();
    let open = scan(&doorway, Default::default());
    assert_eq!(open.progress().status, BoundaryFillStatus::Leaked);
    let manual = scan(
        &doorway,
        BoundaryFillOptions {
            closures: vec![BoundaryClosure {
                start: [21., 44.],
                end: [21., 56.],
            }],
            ..Default::default()
        },
    );
    assert_eq!(manual.progress().status, BoundaryFillStatus::Complete);
    assert!(manual.report().closure_pixels > 0);
    assert!(doorway
        .prepare_boundary_fill_measurement(&open, &options(RectangleScale::Page), None)
        .is_err());
    for seed in [[-1., 2.], [f32::NAN, 2.], [160., 30.], [30., 0.]] {
        assert!(doorway
            .prepare_boundary_fill(0, seed, Default::default())
            .is_err());
    }
    assert!(doorway
        .prepare_boundary_fill(1, [40., 40.], Default::default())
        .is_err());
    assert!(doorway
        .prepare_boundary_fill(
            0,
            [40., 40.],
            BoundaryFillOptions {
                closures: vec![BoundaryClosure {
                    start: [1., 1.],
                    end: [1., 1.]
                }],
                ..Default::default()
            }
        )
        .is_err());

    // User permissions and encryption survive a permitted annotation-only edit.
    let mut base = engine
        .open_bytes(fixture(Rotation::R0, false), None, Path::new("base.pdf"))
        .unwrap();
    base.set_page_calibration(0, Some(&calibration())).unwrap();
    let input = base.snapshot().unwrap();
    let user = credential();
    let owner = credential();
    for (allow_copy, allow_annotate) in [(false, true), (true, false), (true, true)] {
        let protected = security::protect(
            &input,
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                allow_copy,
                allow_annotate,
                allow_modify: false,
                ..Default::default()
            },
        )
        .unwrap();
        let mut restricted = engine
            .open_bytes(protected, Some(&user), Path::new("restricted.pdf"))
            .unwrap();
        if !allow_copy {
            assert!(matches!(
                restricted.prepare_boundary_fill(0, [40., 40.], Default::default()),
                Err(EngineError::SecurityRestricted)
            ));
            continue;
        }
        let job = scan(&restricted, Default::default());
        let prepared = restricted.prepare_boundary_fill_measurement(
            &job,
            &options(RectangleScale::Page),
            None,
        );
        if !allow_annotate {
            assert!(matches!(prepared, Err(EngineError::SecurityRestricted)));
            continue;
        }
        let index = restricted
            .apply_boundary_fill_measurement(&prepared.unwrap())
            .unwrap();
        let bytes = restricted.snapshot().unwrap();
        for password in [&user, &owner] {
            let reopened = engine
                .open_bytes(
                    bytes.clone(),
                    Some(password),
                    Path::new("protected-result.pdf"),
                )
                .unwrap();
            assert!(reopened.properties().encrypted);
            assert_eq!(reopened.measurement_records(0).unwrap().len(), 1);
            assert!(reopened
                .boundary_fill_provenance(0, index)
                .unwrap()
                .is_some());
        }
    }
    let mut pdf = Pdf::load_mem(&input).unwrap();
    let sig=pdf.add_object(dictionary!{"FT"=>"Sig","T"=>Object::string_literal("Synthetic signature"),"V"=>dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),1.into(),2.into(),3.into()],"Contents"=>Object::string_literal("test")}});
    let root = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
    pdf.get_dictionary_mut(root).unwrap().set(
        "AcroForm",
        dictionary! {"Fields"=>vec![Object::Reference(sig)]},
    );
    let mut signed = Vec::new();
    pdf.save_to(&mut signed).unwrap();
    let signed = engine
        .open_bytes(signed, None, Path::new("signed.pdf"))
        .unwrap();
    assert!(signed.signature_count() > 0);
    let job = scan(&signed, Default::default());
    assert!(signed
        .prepare_boundary_fill_measurement(&job, &options(RectangleScale::Page), None)
        .is_err());
}
