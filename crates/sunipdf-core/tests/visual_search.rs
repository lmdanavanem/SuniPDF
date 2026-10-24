use lopdf::{dictionary, Document as Pdf, Stream};
use std::{collections::HashSet, path::Path};
use sunipdf_core::{
    security, visual_search::*, Document, Engine, EngineError, PageGeometry, PointRect, Protection,
    Rotation,
};

fn fixture(rotation: Rotation, blank: bool) -> Vec<u8> {
    let geometry = PageGeometry {
        bbox: PointRect {
            left: 100.,
            bottom: 50.,
            right: 260.,
            top: 210.,
        },
        rotation,
    };
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut content = String::new();
    if !blank {
        for (x, y, quarter) in [(16., 16., 0), (80., 16., 1), (16., 80., 2), (80., 80., 3)] {
            let matrix = geometry.upright_matrix(x, y);
            content.push_str(&format!(
                "q {} {} {} {} {} {} cm ",
                matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5]
            ));
            content.push_str(match quarter {
                0 => "1 0 0 1 0 0 cm ",
                1 => "0 -1 1 0 0 24 cm ",
                2 => "-1 0 0 -1 24 24 cm ",
                _ => "0 1 -1 0 24 0 cm ",
            });
            content.push_str("0 g 4 4 4 16 re f 4 4 16 4 re f 14 14 4 4 re f Q\n");
        }
        let matrix = geometry.upright_matrix(128., 128.);
        content.push_str(&format!(
            "q {} {} {} {} {} {} cm 0 g 0 0 8 8 re f Q\n",
            matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5]
        ));
    }
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
        "MediaBox"=>vec![0.into(),0.into(),400.into(),400.into()],
        "CropBox"=>vec![100.into(),50.into(),260.into(),210.into()],"Rotate"=>i64::from(rotation.degrees()),
        "Resources"=>dictionary!{}, "Contents"=>stream});
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
fn template() -> PointRect {
    PointRect {
        left: 16.,
        bottom: 16.,
        right: 40.,
        top: 40.,
    }
}
fn options() -> VisualSearchOptions {
    VisualSearchOptions {
        rotations: vec![Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270],
        sensitivity: 0.98,
        ..Default::default()
    }
}
fn scan(doc: &Document<'_>, options: VisualSearchOptions) -> VisualSearchJob {
    let mut job = doc.prepare_visual_search(0, template(), options).unwrap();
    for _ in 0..100_000 {
        if job.progress().is_terminal() {
            return job;
        }
        if job.pending_page().is_some() {
            doc.load_visual_search_page(&mut job).unwrap();
        }
        job.step(8192);
    }
    panic!("bounded visual search did not terminate");
}
fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn visual_search_content_geometry_review_atomic_counts_and_source_guards() {
    // One PDFium engine and one test keep all native handles on this worker thread.
    let engine = Engine::new().expect("PDFium is required");
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let input = fixture(rotation, false);
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.pdf");
        std::fs::write(&path, &input).unwrap();
        let mut doc = engine.open(&path, None).unwrap();
        let original = doc.snapshot().unwrap();
        let plain = doc.render_visual_search_content(0, 1.).unwrap();
        assert_eq!(
            plain.rgba,
            doc.render_page(0, 1.).unwrap().rgba,
            "content-only raster must match the normal reader on annotation-free CropBox/Rotate pages: {rotation:?}"
        );
        for scale in [0.5, 1.0, 1.25, 2.0] {
            let full = doc.render_visual_search_content(0, scale).unwrap();
            let region = sunipdf_core::PixelRect::new(7, 11, 29, 23);
            let cropped = doc
                .render_visual_search_content_region(0, scale, region)
                .unwrap();
            let mut expected = Vec::new();
            for row in region.y..region.bottom() {
                let first = (row as usize * full.width as usize + region.x as usize) * 4;
                expected.extend_from_slice(&full.rgba[first..first + region.width as usize * 4]);
            }
            assert_eq!(
                (cropped.width, cropped.height),
                (region.width, region.height)
            );
            assert_eq!(
                cropped.rgba, expected,
                "content region must be an exact crop at {scale}, {rotation:?}"
            );
        }
        doc.add_count_marker(0, (28., 28.), 18., "Existing", None)
            .unwrap();
        assert_ne!(doc.render_page(0, 1.).unwrap().rgba, plain.rgba);
        assert_eq!(
            doc.render_visual_search_content(0, 1.).unwrap().rgba,
            plain.rgba
        );
        let before = doc.snapshot().unwrap();
        let job = scan(&doc, options());
        assert_eq!(job.progress().status, VisualSearchStatus::Complete);
        let report = job.report();
        assert_eq!(report.candidates.len(), 4, "{rotation:?}: {report:?}");
        for (x, y) in [(28., 28.), (92., 28.), (28., 92.), (92., 92.)] {
            assert!(report
                .candidates
                .iter()
                .any(|c| ((c.rect.left + c.rect.right) / 2. - x).abs() < 0.1
                    && ((c.rect.bottom + c.rect.top) / 2. - y).abs() < 0.1));
        }
        assert!(report
            .candidates
            .iter()
            .all(|c| c.score >= 0.98 && c.score <= 1.));
        assert_eq!(
            doc.annotation_register().unwrap().rows.len(),
            1,
            "inspection cannot count automatically"
        );
        assert_eq!(std::fs::read(&path).unwrap(), input);
        let ids: Vec<_> = report.candidates.iter().take(2).map(|c| c.id).collect();
        for bad in [vec![], vec![ids[0], ids[0]], vec![999_999]] {
            assert!(doc
                .prepare_visual_search_counts(&job, &bad, "Reviewed", 12., Some("Inspector"))
                .is_err());
        }
        assert!(doc
            .prepare_visual_search_counts(&job, &ids, " ", 12., None)
            .is_err());
        assert!(doc
            .prepare_visual_search_counts(&job, &ids, "Reviewed", f32::NAN, None)
            .is_err());
        // The second candidate cannot fit this large marker near the edge: reject
        // the entire batch and leave the first candidate uncommitted too.
        assert!(doc
            .prepare_visual_search_counts(&job, &ids, "Reviewed", 72., None)
            .is_err());
        assert_eq!(doc.annotation_register().unwrap().rows.len(), 1);
        let prepared = doc
            .prepare_visual_search_counts(&job, &ids, "Reviewed", 12., Some("Inspector"))
            .unwrap();
        assert_eq!(prepared.count(), 2);
        assert_eq!(doc.annotation_register().unwrap().rows.len(), 1);
        let preview = doc
            .render_visual_search_counts_preview(&prepared, 0, 320)
            .unwrap();
        assert_eq!((preview.width, preview.height), (320, 320));
        assert_eq!(doc.apply_visual_search_counts(&prepared).unwrap(), 2);
        let register = doc.annotation_register().unwrap();
        assert_eq!(register.rows.len(), 3);
        assert_eq!(
            register
                .rows
                .iter()
                .filter(|r| r.count_group.as_deref() == Some("Reviewed"))
                .count(),
            2
        );
        let unique: HashSet<_> = register
            .rows
            .iter()
            .map(|r| r.annotation_id.clone().unwrap())
            .collect();
        assert_eq!(unique.len(), 3);
        assert!(
            doc.apply_visual_search_counts(&prepared).is_err(),
            "source-bound batch cannot apply twice"
        );
        let after = doc.snapshot().unwrap();
        doc.restore(before.clone()).unwrap();
        assert_eq!(
            doc.annotation_register().unwrap().rows.len(),
            1,
            "one Undo snapshot removes the entire batch"
        );
        doc.restore(after).unwrap();
        assert_eq!(
            doc.annotation_register().unwrap().rows.len(),
            3,
            "one Redo snapshot restores the entire batch"
        );
        let copy = temp.path().join("reviewed.pdf");
        sunipdf_core::write_new_copy(&copy, &doc.snapshot().unwrap()).unwrap();
        let reopened = engine.open(&copy, None).unwrap();
        assert_eq!(reopened.annotation_register().unwrap().rows, register.rows);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            input,
            "source file remains byte-identical"
        );
        // A rendered content match survives annotations visually, but its review
        // permission is intentionally stale when the source annotations changed.
        assert!(doc.verify_visual_search(&job).is_err());
        doc.restore(before).unwrap();
        let mut stale = doc.prepare_visual_search(0, template(), options()).unwrap();
        doc.set_page_rotation(0, rotation.turned(90)).unwrap();
        assert!(doc.load_visual_search_page(&mut stale).is_err());
        assert!(doc
            .prepare_visual_search_counts(&job, &ids, "Reviewed", 12., None)
            .is_err());
        doc.restore(original).unwrap();
        let mut running = doc.prepare_visual_search(0, template(), options()).unwrap();
        assert!(doc
            .prepare_visual_search_counts(&running, &[1], "Reviewed", 12., None)
            .is_err());
        running.cancel();
        assert!(doc
            .prepare_visual_search_counts(&running, &[1], "Reviewed", 12., None)
            .is_err());
        let partial = scan(
            &doc,
            VisualSearchOptions {
                max_candidates: 1,
                ..options()
            },
        );
        assert_eq!(
            partial.progress().status,
            VisualSearchStatus::CandidateLimit
        );
        let selected = partial.report().candidates[0].id;
        let partial_prepared = doc
            .prepare_visual_search_counts(&partial, &[selected], "Reviewed partial", 8., None)
            .unwrap();
        assert_eq!(
            partial_prepared.count(),
            1,
            "a stopped partial search only counts explicitly reviewed IDs"
        );
    }

    let input = fixture(Rotation::R0, false);
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
        let mut doc = engine
            .open_bytes(protected, Some(&user), Path::new("restricted.pdf"))
            .unwrap();
        if !allow_copy {
            assert!(matches!(
                doc.prepare_visual_search(0, template(), options()),
                Err(EngineError::SecurityRestricted)
            ));
            continue;
        }
        let job = scan(&doc, options());
        let id = job.report().candidates[0].id;
        let result = doc.prepare_visual_search_counts(&job, &[id], "Reviewed", 8., None);
        if !allow_annotate {
            assert!(matches!(result, Err(EngineError::SecurityRestricted)));
        } else {
            doc.apply_visual_search_counts(&result.unwrap()).unwrap();
            let bytes = doc.snapshot().unwrap();
            for password in [&user, &owner] {
                let reopened = engine
                    .open_bytes(bytes.clone(), Some(password), Path::new("reopened.pdf"))
                    .unwrap();
                assert!(reopened.properties().encrypted);
                assert_eq!(reopened.annotation_register().unwrap().rows.len(), 1);
            }
        }
    }
    let blank = engine
        .open_bytes(fixture(Rotation::R0, true), None, Path::new("blank.pdf"))
        .unwrap();
    assert!(blank
        .prepare_visual_search(0, template(), options())
        .is_err());
    let doc = engine
        .open_bytes(input, None, Path::new("bounds.pdf"))
        .unwrap();
    for invalid in [
        PointRect {
            left: f32::NAN,
            ..template()
        },
        PointRect {
            left: -1.,
            ..template()
        },
        PointRect {
            right: 18.,
            ..template()
        },
        PointRect {
            right: 160.,
            ..template()
        },
    ] {
        assert!(doc.prepare_visual_search(0, invalid, options()).is_err());
    }
    assert!(doc
        .prepare_visual_search(
            0,
            template(),
            VisualSearchOptions {
                pages: vec![1],
                ..options()
            }
        )
        .is_err());
}
