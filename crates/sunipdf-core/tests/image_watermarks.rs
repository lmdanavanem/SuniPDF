use image::{Rgba, RgbaImage};
use sunipdf_core::{
    image_watermark::ImageWatermark, security, Engine, EngineError, ObjectKind, PageMargins,
    Protection, Rotation, StampPosition,
};

#[test]
fn image_watermarks_preserve_geometry_atomic_refusals_and_protection() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("logo.png");
    let mut logo = RgbaImage::from_pixel(80, 40, Rgba([255, 0, 0, 128]));
    for y in 0..40 {
        for x in 40..80 {
            logo.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    logo.save(&path).unwrap();
    let plain = include_bytes!("fixtures/two-pages.pdf");
    let input = dir.path().join("source.pdf");
    std::fs::write(&input, plain).unwrap();
    let mut doc = engine.open(&input, None).unwrap();
    let original = doc.snapshot().unwrap();
    let render = doc.render_page(0, 0.3).unwrap().rgba;
    assert!(doc
        .image_watermark(Some(&[0, 99]), &path, &ImageWatermark::default())
        .is_err());
    assert_eq!(doc.render_page(0, 0.3).unwrap().rgba, render);
    assert_eq!(
        doc.page_objects(0).unwrap().len(),
        engine
            .open(&input, None)
            .unwrap()
            .page_objects(0)
            .unwrap()
            .len()
    );
    for options in [
        ImageWatermark {
            margin_pt: 9999.0,
            ..Default::default()
        },
        ImageWatermark {
            opacity: f32::NAN,
            ..Default::default()
        },
    ] {
        assert!(doc.image_watermark(None, &path, &options).is_err());
    }
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.restore(original.clone()).unwrap();
        doc.set_page_rotation(0, rotation).unwrap();
        doc.crop_page(
            0,
            PageMargins {
                left: 10.0,
                top: 12.0,
                right: 13.0,
                bottom: 14.0,
            },
        )
        .unwrap();
        let before = doc.snapshot().unwrap();
        for position in StampPosition::ALL {
            doc.restore(before.clone()).unwrap();
            let count = doc.page_objects(0).unwrap().len();
            let untouched = doc.render_page(1, 0.2).unwrap().rgba;
            let options = ImageWatermark {
                position,
                width_percent: 90.0,
                angle: 45.0,
                opacity: 0.5,
                ..Default::default()
            };
            assert_eq!(
                doc.image_watermark(Some(&[0, 0]), &path, &options).unwrap(),
                1
            );
            assert_eq!(doc.page_objects(0).unwrap().len(), count + 1);
            let object = doc.object_info(0, count).unwrap();
            assert!(matches!(object.kind, ObjectKind::Image));
            let size = doc.page_size(0).unwrap();
            let b = object.bounds;
            assert!(
                b.left >= 27.8
                    && b.bottom >= 27.8
                    && b.right <= size.width_pt - 27.8
                    && b.top <= size.height_pt - 27.8,
                "{rotation:?} {position:?}: {b:?}"
            );
            assert_eq!(doc.render_page(1, 0.2).unwrap().rgba, untouched);
        }
    }
    let transparent = dir.path().join("transparent.png");
    RgbaImage::new(10, 10).save(&transparent).unwrap();
    let before = doc.render_page(0, 0.3).unwrap().rgba;
    assert!(doc
        .image_watermark(None, &transparent, &Default::default())
        .is_err());
    assert_eq!(doc.render_page(0, 0.3).unwrap().rgba, before);
    for aes256 in [false, true] {
        let protection = Protection {
            user_password: "fixture-open".into(),
            owner_password: "fixture-owner".into(),
            aes256,
            ..Default::default()
        };
        let encrypted = security::protect(plain, None, &protection).unwrap();
        let source = dir.path().join(format!("encrypted-{aes256}.pdf"));
        std::fs::write(&source, &encrypted).unwrap();
        let mut secured = engine.open(&source, Some("fixture-open")).unwrap();
        let properties = secured.properties();
        assert_eq!(
            secured
                .image_watermark(None, &path, &Default::default())
                .unwrap(),
            2
        );
        assert_eq!(secured.properties(), properties);
        let output = dir.path().join(format!("result-{aes256}.pdf"));
        secured.save_as(&output).unwrap();
        for password in ["fixture-open", "fixture-owner"] {
            let reopened = engine.open(&output, Some(password)).unwrap();
            assert!(reopened.properties().encrypted);
            assert!(reopened
                .page_objects(0)
                .unwrap()
                .iter()
                .any(|o| matches!(o.kind, ObjectKind::Image)));
        }
        let restricted = security::protect(
            plain,
            None,
            &Protection {
                allow_modify: false,
                ..protection
            },
        )
        .unwrap();
        std::fs::write(&source, &restricted).unwrap();
        let mut refused = engine.open(&source, Some("fixture-open")).unwrap();
        assert!(matches!(
            refused.image_watermark(None, &path, &Default::default()),
            Err(EngineError::SecurityRestricted)
        ));
        assert_eq!(std::fs::read(&source).unwrap(), restricted);
    }
    assert_eq!(std::fs::read(&input).unwrap(), plain);
}
