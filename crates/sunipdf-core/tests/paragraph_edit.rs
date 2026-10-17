use std::path::Path;
use sunipdf_core::{Engine, EngineError, ObjectKind, TextStyle};

fn text_without_spaces(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn paragraph_native_metrics_preview_and_edit_preserve_fonts_rotation_and_atomic_refusals() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut plain_for_refusals = Vec::new();
    for rotation in [0, 90, 180, 270] {
        let mut pdf = lopdf::Document::load_mem(include_bytes!("fixtures/two-pages.pdf")).unwrap();
        let page = pdf.get_pages()[&1];
        let page = pdf.get_dictionary_mut(page).unwrap();
        page.set("Rotate", rotation);
        page.set(
            "CropBox",
            vec![100.into(), 50.into(), 500.into(), 650.into()],
        );
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("paragraph-source.pdf"))
            .unwrap();
        let old: Vec<_> = doc
            .page_objects(0)
            .unwrap()
            .iter()
            .map(|o| o.index)
            .collect();
        doc.delete_objects(0, &old).unwrap();
        doc.add_text(
            0,
            120.0,
            260.0,
            "Original text",
            &TextStyle {
                size_pt: 16.0,
                rgb: (20, 50, 90),
                opacity: 0.6,
                ..Default::default()
            },
        )
        .unwrap();
        let index = 0;
        doc.rotate_object(0, index, 25.0).unwrap();
        let before = doc.snapshot().unwrap();
        if rotation == 0 {
            plain_for_refusals = before.clone();
        }
        let original = doc.object_info(0, index).unwrap();
        let original_pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let text = "Wide WWW and narrow iii words with accents é à.
A second paragraph with preserved original font.";
        let preview = doc
            .preview_paragraph(0, &[index], text, 115.0, 1.3)
            .unwrap();
        assert_eq!(doc.object_info(0, index).unwrap(), original);
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original_pixels);
        let indices = doc.reflow_paragraph(0, &[index], text, 115.0, 1.3).unwrap();
        assert!(indices.len() > 2);
        let infos: Vec<_> = indices
            .iter()
            .map(|&i| doc.object_info(0, i).unwrap())
            .collect();
        assert!(infos.iter().all(|o| o.kind == ObjectKind::Text
            && o.font_name == original.font_name
            && o.fill_rgb == original.fill_rgb
            && o.opacity == original.opacity
            && (o.angle_deg - original.angle_deg).abs() < 0.01));
        let back = infos
            .iter()
            .filter_map(|o| o.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(text_without_spaces(&back), text_without_spaces(text));
        assert_eq!(
            doc.render_region(0, preview.scale, preview.region)
                .unwrap()
                .rgba,
            preview.image.rgba
        );
        let output = dir.path().join(format!("paragraph-{rotation}.pdf"));
        doc.save_as(&output).unwrap();
        let reopened = engine.open(&output, None).unwrap();
        assert_eq!(
            reopened.render_page(0, 1.0).unwrap().rgba,
            doc.render_page(0, 1.0).unwrap().rgba
        );
        let redo = doc.snapshot().unwrap();
        doc.restore(before.clone()).unwrap();
        assert_eq!(doc.object_info(0, index).unwrap(), original);
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original_pixels);
        for unsupported in ["Missing 🚀🚀", "Missing 中文"] {
            assert!(matches!(
                doc.preview_paragraph(0, &[index], unsupported, 115.0, 1.3),
                Err(EngineError::MissingGlyphs(_))
            ));
            assert!(matches!(
                doc.reflow_paragraph(0, &[index], unsupported, 115.0, 1.3),
                Err(EngineError::MissingGlyphs(_))
            ));
            assert_eq!(doc.object_info(0, index).unwrap(), original);
            assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original_pixels);
        }
        match doc.preview_paragraph(0, &[index], "Ligature ﬁ", 115.0, 1.3) {
            Ok(_) => {
                let ligature = doc
                    .reflow_paragraph(0, &[index], "Ligature ﬁ", 115.0, 1.3)
                    .unwrap();
                let back = ligature
                    .iter()
                    .map(|&i| doc.object_info(0, i).unwrap().text.unwrap())
                    .collect::<String>();
                assert_eq!(text_without_spaces(&back), "Ligatureﬁ");
            }
            Err(EngineError::MissingGlyphs(_)) => {}
            Err(other) => panic!("unexpected ligature result: {other}"),
        }
        doc.restore(before.clone()).unwrap();
        for width in [f32::NAN, 0.0, 1.0] {
            assert!(doc
                .reflow_paragraph(0, &[index], "Unbreakablelongword", width, 1.3)
                .is_err());
            assert_eq!(doc.object_info(0, index).unwrap(), original);
        }
        doc.restore(redo).unwrap();
        assert_eq!(
            doc.render_region(0, preview.scale, preview.region)
                .unwrap()
                .rgba,
            preview.image.rgba
        );
        let reedited = doc
            .reflow_paragraph(
                0,
                &indices,
                "One edited paragraph across existing lines.",
                130.0,
                1.2,
            )
            .unwrap();
        assert!(reedited.len() > 1);
    }
    for prefix in ["1 Tw\n", "/Span BMC\n", "q 110 60 350 550 re W n\n"] {
        let mut pdf = lopdf::Document::load_mem(&plain_for_refusals).unwrap();
        let page = pdf.get_pages()[&1];
        let mut content = prefix.as_bytes().to_vec();
        content.extend(pdf.get_page_content(page).unwrap());
        if prefix.contains("BMC") {
            content.extend(b"\nEMC");
        } else if prefix.starts_with('q') {
            content.extend(b"\nQ");
        }
        let stream = pdf.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), content));
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Contents", stream);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let source = dir.path().join("unsupported-paragraph.pdf");
        std::fs::write(&source, &bytes).unwrap();
        let mut doc = engine.open(&source, None).unwrap();
        let original = doc.page_objects(0).unwrap();
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        assert!(doc
            .preview_paragraph(0, &[0], "Safe refusal", 150.0, 1.2)
            .is_err());
        assert!(doc
            .reflow_paragraph(0, &[0], "Safe refusal", 150.0, 1.2)
            .is_err());
        assert_eq!(doc.page_objects(0).unwrap(), original);
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }
    let mut mixed = engine
        .open_bytes(plain_for_refusals, None, Path::new("mixed.pdf"))
        .unwrap();
    mixed
        .add_text(
            0,
            120.0,
            220.0,
            "Other style",
            &TextStyle {
                rgb: (200, 20, 10),
                ..Default::default()
            },
        )
        .unwrap();
    let original = mixed.page_objects(0).unwrap();
    assert!(mixed
        .reflow_paragraph(0, &[0, 1], "Mixed styles", 150.0, 1.2)
        .is_err());
    assert_eq!(mixed.page_objects(0).unwrap(), original);
}
