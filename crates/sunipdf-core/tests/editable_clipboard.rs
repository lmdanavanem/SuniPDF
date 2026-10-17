use std::path::Path;
use sunipdf_core::{Engine, ObjectKind, ObjectStyle, PointRect, ShapeKind, ShapeStyle, TextStyle};

#[test]
fn native_paste_preserves_individual_fonts_styles_geometry_and_atomic_refusals() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut source = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("source.pdf"),
        )
        .unwrap();
    let old: Vec<_> = source
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|o| o.index)
        .collect();
    source.delete_objects(0, &old).unwrap();
    source
        .add_shape(
            0,
            ShapeKind::Rectangle,
            PointRect {
                left: 180.0,
                bottom: 150.0,
                right: 300.0,
                top: 220.0,
            },
            &ShapeStyle {
                fill_rgb: Some((220, 30, 80)),
                opacity: 0.4,
                ..Default::default()
            },
        )
        .unwrap();
    source
        .add_text(
            0,
            180.0,
            250.0,
            "Café editable",
            &TextStyle {
                size_pt: 16.0,
                rgb: (20, 50, 90),
                opacity: 0.6,
                ..Default::default()
            },
        )
        .unwrap();
    source.rotate_object(0, 1, 23.0).unwrap();
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        4,
        4,
        image::Rgba([30, 80, 220, 128]),
    ));
    source
        .add_image_data(0, &image, 310.0, 170.0, 40.0)
        .unwrap();
    let original = source.page_objects(0).unwrap();
    let original_pixels = source.render_page(0, 1.0).unwrap().rgba;
    let clip = source.copy_objects(0, &[2, 0, 1]).unwrap();
    assert_eq!(clip.count, 3);
    for rotation in [0, 90, 180, 270] {
        let mut pdf = lopdf::Document::load_mem(include_bytes!("fixtures/two-pages.pdf")).unwrap();
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Rotate", rotation);
        pdf.get_dictionary_mut(page).unwrap().set(
            "CropBox",
            vec![100.into(), 50.into(), 500.into(), 650.into()],
        );
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut target = engine
            .open_bytes(bytes, None, Path::new("target.pdf"))
            .unwrap();
        target
            .add_text(
                0,
                10.0,
                30.0,
                "Destination Courier",
                &TextStyle {
                    family: sunipdf_core::FontFamily::Courier,
                    ..Default::default()
                },
            )
            .unwrap();
        let before = target.snapshot().unwrap();
        let count = target.page_objects(0).unwrap().len();
        let pasted = target.paste_editable_objects(0, &clip, 0.0, 0.0).unwrap();
        let mut reference = engine
            .open_bytes(before.clone(), None, Path::new("group-reference.pdf"))
            .unwrap();
        reference.paste_objects(0, &clip, 0.0, 0.0).unwrap();
        let native_pixels = target.render_page(0, 1.0).unwrap().rgba;
        let group_pixels = reference.render_page(0, 1.0).unwrap().rgba;
        let difference: usize = native_pixels
            .iter()
            .zip(&group_pixels)
            .map(|(&a, &b)| a.abs_diff(b) as usize)
            .sum();
        assert!(
            difference as f64 / (native_pixels.len() as f64) < 0.25,
            "native paste differs from the source appearance"
        );
        assert_eq!(pasted, (count..count + 3).collect::<Vec<_>>());
        for (&index, old) in pasted.iter().zip(&original) {
            let new = target.object_info(0, index).unwrap();
            assert_eq!(new.kind, old.kind);
            assert_ne!(new.kind, ObjectKind::Form);
            assert_eq!(new.text, old.text);
            assert_eq!(new.font_name, old.font_name);
            assert_eq!(new.fill_rgb, old.fill_rgb);
            assert_eq!(new.opacity, old.opacity);
            assert!((new.angle_deg - old.angle_deg).abs() < 0.02);
            for (a, b) in [
                (new.bounds.left, old.bounds.left),
                (new.bounds.bottom, old.bounds.bottom),
                (new.bounds.right, old.bounds.right),
                (new.bounds.top, old.bounds.top),
            ] {
                assert!((a - b).abs() < 0.1, "{new:?} / {old:?}");
            }
        }
        let redo = target.snapshot().unwrap();
        let output = dir.path().join(format!("native-{rotation}.pdf"));
        target.save_as(&output).unwrap();
        let reopened = engine.open(&output, None).unwrap();
        assert_eq!(
            reopened.page_objects(0).unwrap(),
            target.page_objects(0).unwrap()
        );
        assert_eq!(
            reopened.render_page(0, 1.0).unwrap().rgba,
            target.render_page(0, 1.0).unwrap().rgba
        );
        target
            .set_object_text(0, pasted[1], "Still editable")
            .unwrap();
        assert_eq!(
            target.object_info(0, pasted[1]).unwrap().text.as_deref(),
            Some("Still editable")
        );
        target
            .set_objects_style(
                0,
                &pasted,
                &ObjectStyle {
                    fill_rgb: Some((15, 120, 40)),
                    opacity: Some(0.3),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(pasted[..2]
            .iter()
            .all(|&i| target.object_info(0, i).unwrap().fill_rgb == Some((15, 120, 40))));
        target.restore(before).unwrap();
        assert_eq!(target.page_objects(0).unwrap().len(), count);
        target.restore(redo).unwrap();
        assert_eq!(target.page_objects(0).unwrap().len(), count + 3);
        let before_refusal = target.page_objects(0).unwrap();
        assert!(target
            .paste_editable_objects(0, &clip, f32::NAN, 0.0)
            .is_err());
        assert_eq!(target.page_objects(0).unwrap(), before_refusal);
    }
    let mut grouped = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("grouped.pdf"),
        )
        .unwrap();
    let group = grouped.paste_objects(0, &clip, 0.0, 0.0).unwrap();
    let unsupported = grouped.copy_objects(0, &[group]).unwrap();
    let before = grouped.page_objects(0).unwrap();
    let pixels = grouped.render_page(0, 1.0).unwrap().rgba;
    assert!(grouped
        .paste_editable_objects(0, &unsupported, 0.0, 0.0)
        .is_err());
    assert_eq!(grouped.page_objects(0).unwrap(), before);
    assert_eq!(grouped.render_page(0, 1.0).unwrap().rgba, pixels);
    let mut marked_pdf = lopdf::Document::load_mem(&source.snapshot().unwrap()).unwrap();
    let marked_page = marked_pdf.get_pages()[&1];
    let mut marked_content = b"/Span BMC\n".to_vec();
    marked_content.extend(marked_pdf.get_page_content(marked_page).unwrap());
    marked_content.extend(b"\nEMC");
    let stream =
        marked_pdf.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), marked_content));
    marked_pdf
        .get_dictionary_mut(marked_page)
        .unwrap()
        .set("Contents", stream);
    let mut bytes = Vec::new();
    marked_pdf.save_to(&mut bytes).unwrap();
    let marked_source = engine
        .open_bytes(bytes, None, Path::new("marked-source.pdf"))
        .unwrap();
    let marked_clip = marked_source.copy_objects(0, &[0]).unwrap();
    assert!(
        grouped
            .paste_editable_objects(0, &marked_clip, 0.0, 0.0)
            .is_err(),
        "original marked content must be checked before regeneration"
    );
    assert_eq!(grouped.page_objects(0).unwrap(), before);
    assert_eq!(source.page_objects(0).unwrap(), original);
    assert_eq!(source.render_page(0, 1.0).unwrap().rgba, original_pixels);

    let mut clipped_pdf = lopdf::Document::load_mem(&source.snapshot().unwrap()).unwrap();
    let clipped_page = clipped_pdf.get_pages()[&1];
    let mut clipped_content = b"q 180 150 70 160 re W n\n".to_vec();
    clipped_content.extend(clipped_pdf.get_page_content(clipped_page).unwrap());
    clipped_content.extend(b"\nQ");
    let clipped_stream = clipped_pdf.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        clipped_content,
    ));
    let page = clipped_pdf.get_dictionary_mut(clipped_page).unwrap();
    page.set("Contents", clipped_stream);
    page.set("Rotate", 90);
    page.set(
        "CropBox",
        vec![100.into(), 50.into(), 500.into(), 650.into()],
    );
    let mut bytes = Vec::new();
    clipped_pdf.save_to(&mut bytes).unwrap();
    let clipped_source = engine
        .open_bytes(bytes, None, Path::new("clipped-source.pdf"))
        .unwrap();
    let clipped_clip = clipped_source.copy_objects(0, &[0, 1, 2]).unwrap();
    let target_bytes = include_bytes!("fixtures/two-pages.pdf").to_vec();
    let mut clipped_target = engine
        .open_bytes(target_bytes.clone(), None, Path::new("clipped-target.pdf"))
        .unwrap();
    let mut clipped_reference = engine
        .open_bytes(target_bytes, None, Path::new("clipped-reference.pdf"))
        .unwrap();
    let old: Vec<_> = clipped_target
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|object| object.index)
        .collect();
    clipped_target.delete_objects(0, &old).unwrap();
    clipped_reference.delete_objects(0, &old).unwrap();
    let clipped_indices = clipped_target
        .paste_editable_objects(0, &clipped_clip, 20.0, -15.0)
        .unwrap();
    clipped_reference
        .paste_objects(0, &clipped_clip, 20.0, -15.0)
        .unwrap();
    let actual = clipped_target.render_page(0, 1.0).unwrap().rgba;
    let expected = clipped_reference.render_page(0, 1.0).unwrap().rgba;
    let difference: usize = actual
        .iter()
        .zip(&expected)
        .map(|(&a, &b)| a.abs_diff(b) as usize)
        .sum();
    assert!(
        difference as f64 / (actual.len() as f64) < 0.25,
        "rotated source clipping must survive"
    );
    let original_region = clipped_source
        .render_region(0, 1.0, sunipdf_core::PixelRect::new(80, 40, 250, 250))
        .unwrap()
        .rgba;
    let pasted_region = clipped_target
        .render_region(0, 1.0, sunipdf_core::PixelRect::new(100, 447, 250, 250))
        .unwrap()
        .rgba;
    let difference: usize = original_region
        .iter()
        .zip(&pasted_region)
        .map(|(&a, &b)| a.abs_diff(b) as usize)
        .sum();
    assert!(
        difference as f64 / (original_region.len() as f64) < 0.25,
        "clipping and appearance match the original source, not just the intermediate clip"
    );
    let clipped_output = dir.path().join("clipped-native.pdf");
    clipped_target.save_as(&clipped_output).unwrap();
    let mut clipped_reopened = engine.open(&clipped_output, None).unwrap();
    assert_eq!(clipped_reopened.render_page(0, 1.0).unwrap().rgba, actual);
    clipped_reopened
        .set_object_text(0, clipped_indices[1], "Edit preserves clip")
        .unwrap();
    let edited_output = dir.path().join("clipped-edited.pdf");
    clipped_reopened.save_as(&edited_output).unwrap();
    let edited = engine.open(&edited_output, None).unwrap();
    assert_eq!(
        edited.render_page(0, 1.0).unwrap().rgba,
        clipped_reopened.render_page(0, 1.0).unwrap().rgba
    );

    let protected = sunipdf_core::security::protect(
        &source.snapshot().unwrap(),
        None,
        &sunipdf_core::Protection {
            user_password: "native-test".into(),
            owner_password: "native-owner".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let encrypted_source = engine
        .open_bytes(
            protected.clone(),
            Some("native-test"),
            Path::new("encrypted-source.pdf"),
        )
        .unwrap();
    let encrypted_clip = encrypted_source.copy_objects(0, &[0, 1, 2]).unwrap();
    let input = dir.path().join("encrypted-target.pdf");
    std::fs::write(&input, &protected).unwrap();
    let mut encrypted = engine.open(&input, Some("native-test")).unwrap();
    encrypted
        .paste_editable_objects(1, &encrypted_clip, 20.0, -10.0)
        .unwrap();
    assert!(encrypted.properties().encrypted);
    let output = dir.path().join("native-encrypted.pdf");
    encrypted.save_as(&output).unwrap();
    assert!(engine.open(&output, None).is_err());
    let reopened = engine.open(&output, Some("native-test")).unwrap();
    assert_eq!(
        reopened.page_objects(1).unwrap(),
        encrypted.page_objects(1).unwrap()
    );
    assert_eq!(
        reopened.render_page(1, 1.0).unwrap().rgba,
        encrypted.render_page(1, 1.0).unwrap().rgba
    );
    assert_eq!(std::fs::read(input).unwrap(), protected);
}
