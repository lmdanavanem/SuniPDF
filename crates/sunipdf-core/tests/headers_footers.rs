use sunipdf_core::{header_footer::HeaderFooter, Engine, PageMargins, Rotation};

#[test]
fn six_zone_headers_fit_rotated_pages_expand_numbers_and_refuse_atomically() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.pdf");
    let source = include_bytes!("fixtures/two-pages.pdf");
    std::fs::write(&input, source).unwrap();
    let mut doc = engine.open(&input, None).unwrap();
    let original = doc.snapshot().unwrap();
    let options = HeaderFooter {
        zones: [
            "Left".into(),
            "Center".into(),
            "Right".into(),
            "Author".into(),
            "{n} / {total}".into(),
            "Copy".into(),
        ],
        ..Default::default()
    };
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.restore(original.clone()).unwrap();
        doc.set_page_rotation(0, rotation).unwrap();
        doc.crop_page(
            0,
            PageMargins {
                left: 10.0,
                right: 12.0,
                top: 13.0,
                bottom: 14.0,
            },
        )
        .unwrap();
        let count = doc.page_objects(0).unwrap().len();
        let untouched = doc.render_page(1, 0.2).unwrap().rgba;
        assert_eq!(doc.headers_footers(Some(&[0, 0]), &options).unwrap(), 1);
        assert_eq!(doc.page_objects(0).unwrap().len(), count + 6);
        let size = doc.page_size(0).unwrap();
        let objects = doc.page_objects(0).unwrap();
        for (zone, object) in objects[count..].iter().enumerate() {
            let b = object.bounds;
            assert!(
                b.left >= 27.8
                    && b.bottom >= 27.8
                    && b.right <= size.width_pt - 27.8
                    && b.top <= size.height_pt - 27.8,
                "{rotation:?}: {b:?}"
            );
            if zone % 3 != 2 {
                assert!(b.right < objects[count + zone + 1].bounds.left);
            }
        }
        assert!(doc.page_text(0).unwrap().contains("1 / 2"));
        assert_eq!(doc.render_page(1, 0.2).unwrap().rgba, untouched);
        let before = doc.render_page(0, 0.3).unwrap().rgba;
        let count = objects.len();
        let mut overflow = options.clone();
        overflow.zones[5] = "W".repeat(200);
        assert!(doc.headers_footers(None, &overflow).is_err());
        assert!(doc.headers_footers(Some(&[0, 99]), &options).is_err());
        assert_eq!(doc.page_objects(0).unwrap().len(), count);
        assert_eq!(doc.render_page(0, 0.3).unwrap().rgba, before);
    }
    assert_eq!(std::fs::read(&input).unwrap(), source);
}
