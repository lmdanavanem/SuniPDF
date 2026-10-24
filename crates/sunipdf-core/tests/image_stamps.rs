use image::{Rgba, RgbaImage};
use std::path::Path;
use sunipdf_core::{
    annot_rich::RichAnnotation, security, Engine, EngineError, PageMargins, PointRect, Protection,
    Rotation,
};

fn rect() -> PointRect {
    PointRect {
        left: 40.0,
        bottom: 40.0,
        right: 240.0,
        top: 240.0,
    }
}

fn logo(path: &Path) {
    let mut image = RgbaImage::from_pixel(80, 40, Rgba([255, 0, 0, 128]));
    for y in 0..40 {
        for x in 40..80 {
            image.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    image.save(path).unwrap();
}

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn color(image: &sunipdf_core::RenderedPage, x: u32, y: u32) -> [u8; 3] {
    let at = ((image.height - 1 - y) * image.width + x) as usize * 4;
    image.rgba[at..at + 3].try_into().unwrap()
}

#[test]
fn image_stamp_preserves_alpha_aspect_crop_rotation_encryption_and_annotation_identity() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let image = temp.path().join("logo été 中文.png");
    logo(&image);
    let input = include_bytes!("fixtures/two-pages.pdf");
    let mut doc = engine
        .open_bytes(input.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let original = doc.snapshot().unwrap();
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
        let before = doc.render_page(0, 1.0).unwrap();
        let index = doc
            .add_image_stamp_annotation(0, rect(), &image, 0.5, Some("Review author"))
            .unwrap();
        let detail = doc
            .annotation_details(0)
            .unwrap()
            .into_iter()
            .find(|d| d.index == index)
            .unwrap();
        assert_eq!(detail.subtype, "Stamp");
        assert!((detail.rect.left - 40.0).abs() < 0.01 && (detail.rect.top - 240.0).abs() < 0.01);
        let output = temp.path().join(format!("rotation-{rotation:?}.pdf"));
        doc.save_as(&output).unwrap();
        let mut reopened = engine.open(&output, None).unwrap();
        let painted = reopened.render_page(0, 1.0).unwrap();
        let red = color(&painted, 80, 140);
        let blue = color(&painted, 180, 140);
        for (actual, background, ink, alpha) in [
            (red, color(&before, 80, 140), [255u8, 0, 0], 64u32),
            (blue, color(&before, 180, 140), [0u8, 0, 255], 128u32),
        ] {
            for channel in 0..3 {
                let expected = (u32::from(ink[channel]) * alpha
                    + u32::from(background[channel]) * (255 - alpha))
                    / 255;
                assert!(
                    actual[channel].abs_diff(expected as u8) <= 3,
                    "{rotation:?}: {actual:?}, background {background:?}"
                );
            }
        }
        assert_eq!(
            color(&painted, 80, 60),
            color(&before, 80, 60),
            "aspect fitting leaves the lower margin intact"
        );
        reopened.delete_annotation(0, index).unwrap();
        assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, before.rgba);
    }
    for aes256 in [false, true] {
        let user = credential();
        let owner = credential();
        let protected = security::protect(
            input,
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                aes256,
                allow_modify: false,
                ..Default::default()
            },
        )
        .unwrap();
        let mut doc = engine
            .open_bytes(protected, Some(&user), Path::new("protected.pdf"))
            .unwrap();
        let properties = doc.properties();
        assert!(!properties.can_modify && properties.can_annotate);
        doc.add_image_stamp_annotation(0, rect(), &image, 1.0, None)
            .unwrap();
        assert_eq!(doc.properties(), properties);
        let result = doc.snapshot().unwrap();
        for password in [&user, &owner] {
            let reopened = engine
                .open_bytes(result.clone(), Some(password), Path::new("result.pdf"))
                .unwrap();
            assert!(reopened.properties().encrypted);
            assert_eq!(
                reopened
                    .annotation_details(0)
                    .unwrap()
                    .last()
                    .unwrap()
                    .subtype,
                "Stamp"
            );
        }
    }
    image_stamp_refuses_unsafe_inputs_atomically_and_honors_annotation_permissions(&engine);
    image_stamp_commit_uses_the_preview_pixels_after_the_source_image_changes(&engine);
}

fn image_stamp_refuses_unsafe_inputs_atomically_and_honors_annotation_permissions(engine: &Engine) {
    let temp = tempfile::tempdir().unwrap();
    let valid = temp.path().join("valid.png");
    logo(&valid);
    let input = include_bytes!("fixtures/two-pages.pdf");
    let mut doc = engine
        .open_bytes(input.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let before = doc.render_page(0, 0.5).unwrap().rgba;
    let count = doc.annotation_details(0).unwrap().len();
    for opacity in [f32::NAN, f32::INFINITY, -1.0, 0.0, 1.1] {
        assert!(doc
            .add_image_stamp_annotation(0, rect(), &valid, opacity, None)
            .is_err());
    }
    for bad in [
        PointRect {
            left: f32::NAN,
            ..rect()
        },
        PointRect {
            right: 41.0,
            ..rect()
        },
        PointRect {
            bottom: -1.0,
            ..rect()
        },
    ] {
        assert!(doc
            .add_image_stamp_annotation(0, bad, &valid, 1.0, None)
            .is_err());
    }
    assert!(doc
        .add_image_stamp_annotation(9, rect(), &valid, 1.0, None)
        .is_err());
    assert!(doc
        .add_image_stamp_annotation(0, rect(), &valid, 1.0, Some("bad\nauthor"))
        .is_err());
    let transparent = temp.path().join("transparent.png");
    RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 0]))
        .save(&transparent)
        .unwrap();
    let wide = temp.path().join("wide.png");
    RgbaImage::from_pixel(4097, 1, Rgba([255; 4]))
        .save(&wide)
        .unwrap();
    let budget = temp.path().join("budget.png");
    RgbaImage::from_pixel(2001, 2000, Rgba([255; 4]))
        .save(&budget)
        .unwrap();
    let large = temp.path().join("large.bin");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let broken = temp.path().join("broken.png");
    std::fs::write(&broken, b"not an image").unwrap();
    for path in [&transparent, &wide, &budget, &large, &broken, temp.path()] {
        assert!(doc
            .add_image_stamp_annotation(0, rect(), path, 1.0, None)
            .is_err());
        assert_eq!(doc.annotation_details(0).unwrap().len(), count);
        assert_eq!(doc.render_page(0, 0.5).unwrap().rgba, before);
    }
    let user = credential();
    let protected = security::protect(
        input,
        None,
        &Protection {
            user_password: user.clone(),
            owner_password: credential(),
            allow_annotate: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut restricted = engine
        .open_bytes(protected, Some(&user), Path::new("restricted.pdf"))
        .unwrap();
    assert!(matches!(
        restricted.add_image_stamp_annotation(
            0,
            rect(),
            &temp.path().join("missing.png"),
            1.0,
            None
        ),
        Err(EngineError::SecurityRestricted)
    ));
}

fn image_stamp_commit_uses_the_preview_pixels_after_the_source_image_changes(engine: &Engine) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("chosen.png");
    logo(&path);
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("source.pdf"),
        )
        .unwrap();
    let before = doc.render_page(0, 1.0).unwrap().rgba;
    let mut preview = doc
        .preview_rich_annotation(
            0,
            &RichAnnotation::ImageStamp {
                rect: rect(),
                image: path.clone(),
                opacity: 1.0,
            },
            None,
        )
        .unwrap();
    assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, before);
    RgbaImage::from_pixel(80, 40, Rgba([0, 255, 0, 255]))
        .save(&path)
        .unwrap();
    doc.commit_rich_annotation_preview(preview.prepared.take().unwrap())
        .unwrap();
    let blue = color(&doc.render_page(0, 1.0).unwrap(), 180, 140);
    assert!(
        blue[2] > 250 && blue[0] < 4 && blue[1] < 4,
        "the committed annotation must match the preview, not a reread image: {blue:?}"
    );
}
