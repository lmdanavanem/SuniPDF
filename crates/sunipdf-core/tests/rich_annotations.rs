use std::path::Path;
use sunipdf_core::annot_rich::{RichFreeTextStyle, StampKind, VertexAnnotKind};
use sunipdf_core::{AnnotStyle, Engine, PointRect, TextStyle};
#[path = "support/feature_evidence.rs"]
mod feature_evidence;

#[test]
fn rich_annotation_appearances_survive_rotation_save_and_undo_with_atomic_refusals() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut evidence = feature_evidence::Evidence::new("rich");
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
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("rich.pdf"))
            .unwrap();
        let before = doc.snapshot().unwrap();
        let count = doc.annotation_details(0).unwrap().len();
        let style = AnnotStyle {
            rgb: (200, 20, 50),
            fill_rgb: Some((255, 220, 230)),
            opacity: 0.6,
            ..Default::default()
        };
        for kind in [VertexAnnotKind::Polygon, VertexAnnotKind::Cloud] {
            doc.add_vertex_annotation(
                0,
                kind,
                &[(40.0, 70.0), (160.0, 70.0), (100.0, 150.0)],
                &style,
                Some("Test author"),
            )
            .unwrap();
        }
        doc.add_vertex_annotation(
            0,
            VertexAnnotKind::Polyline,
            &[(180.0, 70.0), (250.0, 130.0), (300.0, 70.0)],
            &AnnotStyle {
                fill_rgb: None,
                ..style.clone()
            },
            None,
        )
        .unwrap();
        doc.add_stamp_annotation(
            0,
            PointRect {
                left: 40.0,
                bottom: 180.0,
                right: 200.0,
                top: 230.0,
            },
            StampKind::Approved,
            None,
        )
        .unwrap();
        doc.add_file_attachment_annotation(
            0,
            PointRect {
                left: 270.0,
                bottom: 180.0,
                right: 294.0,
                top: 212.0,
            },
            "notes-é.txt",
            b"Local attachment contents",
            None,
        )
        .unwrap();
        let rich = doc
            .add_rich_free_text_annotation(
                0,
                None,
                PointRect {
                    left: 40.0,
                    bottom: 260.0,
                    right: 330.0,
                    top: 350.0,
                },
                "Bold italic native text\nAccents é à",
                &RichFreeTextStyle {
                    text: TextStyle {
                        bold: true,
                        italic: true,
                        size_pt: 16.0,
                        ..Default::default()
                    },
                    fill_rgb: Some((255, 250, 230)),
                    border_pt: 1.0,
                },
                None,
            )
            .unwrap();
        assert_eq!(doc.annotation_details(0).unwrap().len(), count + 6);
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let output = dir.path().join(format!("rich-{rotation}.pdf"));
        doc.save_as(&output).unwrap();
        let reopened = engine.open(&output, None).unwrap();
        assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, pixels);
        if matches!(rotation, 0 | 90) {
            evidence.save(&format!("rotation-{rotation}"), &output, &reopened, None);
        }
        let parsed = lopdf::Document::load(&output).unwrap();
        let entries = parsed
            .get_dictionary(parsed.get_pages()[&1])
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        for entry in &entries[count..] {
            let (_, annotation) = parsed.dereference(entry).unwrap();
            let annotation = annotation.as_dict().unwrap();
            let (_, appearance) = parsed
                .dereference(
                    annotation
                        .get(b"AP")
                        .unwrap()
                        .as_dict()
                        .unwrap()
                        .get(b"N")
                        .unwrap(),
                )
                .unwrap();
            assert!(!appearance.as_stream().unwrap().content.is_empty());
        }
        let undo = doc.snapshot().unwrap();
        assert!(
            doc.update_free_text(0, rich, "Plain fallback", &Default::default())
                .is_err(),
            "legacy edit must not discard embedded font resources"
        );
        assert!(doc
            .add_vertex_annotation(
                0,
                VertexAnnotKind::Polygon,
                &[(f32::NAN, 0.0); 3],
                &style,
                None
            )
            .is_err());
        assert!(doc
            .add_file_attachment_annotation(
                0,
                PointRect {
                    left: 270.0,
                    bottom: 180.0,
                    right: 294.0,
                    top: 212.0
                },
                "../escape.txt",
                b"data",
                None
            )
            .is_err());
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
        doc.restore(before).unwrap();
        assert_eq!(doc.annotation_details(0).unwrap().len(), count);
        doc.restore(undo).unwrap();
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
    }
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("cjk-annotation.pdf"),
        )
        .unwrap();
    let font =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/NotoSansJP-Regular.otf");
    let selected_font = dir.path().join("selected-cjk.otf");
    std::fs::copy(&font, &selected_font).unwrap();
    let style = RichFreeTextStyle {
        text: TextStyle {
            font_file: Some(selected_font.clone()),
            size_pt: 18.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let rect = PointRect {
        left: 40.0,
        bottom: 250.0,
        right: 300.0,
        top: 340.0,
    };
    let mut preview = doc
        .preview_rich_annotation(
            0,
            &sunipdf_core::annot_rich::RichAnnotation::FreeText {
                index: None,
                rect,
                text: "日本語中文".into(),
                style: style.clone(),
            },
            None,
        )
        .unwrap();
    std::fs::remove_file(&selected_font).unwrap();
    let index = doc
        .commit_rich_annotation_preview(preview.prepared.take().unwrap())
        .unwrap();
    std::fs::copy(&font, &selected_font).unwrap();
    assert_eq!(
        doc.render_region(0, preview.scale, preview.region)
            .unwrap()
            .rgba,
        preview.image.rgba
    );
    let output = dir.path().join("embedded-cjk.pdf");
    doc.save_as(&output).unwrap();
    let pdf = lopdf::Document::load(&output).unwrap();
    let annots = pdf
        .get_dictionary(pdf.get_pages()[&1])
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    let annotation = pdf
        .dereference(&annots[index])
        .unwrap()
        .1
        .as_dict()
        .unwrap();
    let ap = pdf
        .dereference(annotation.get(b"AP").unwrap())
        .unwrap()
        .1
        .as_dict()
        .unwrap();
    let ap = pdf
        .dereference(ap.get(b"N").unwrap())
        .unwrap()
        .1
        .as_stream()
        .unwrap();
    let resources = pdf
        .dereference(ap.dict.get(b"Resources").unwrap())
        .unwrap()
        .1
        .as_dict()
        .unwrap();
    let fonts = pdf
        .dereference(resources.get(b"Font").unwrap())
        .unwrap()
        .1
        .as_dict()
        .unwrap();
    assert!(
        fonts.iter().any(|(_, font)| {
            let font = pdf.dereference(font).unwrap().1.as_dict().unwrap();
            let unicode = font
                .get(b"ToUnicode")
                .ok()
                .and_then(|value| pdf.dereference(value).ok())
                .and_then(|(_, value)| value.as_stream().ok())
                .is_some_and(|stream| !stream.get_plain_content().unwrap().is_empty());
            let Some(descendants) = font
                .get(b"DescendantFonts")
                .ok()
                .and_then(|value| value.as_array().ok())
            else {
                return false;
            };
            unicode
                && descendants.iter().any(|font| {
                    let font = pdf.dereference(font).unwrap().1.as_dict().unwrap();
                    let descriptor = pdf
                        .dereference(font.get(b"FontDescriptor").unwrap())
                        .unwrap()
                        .1
                        .as_dict()
                        .unwrap();
                    // PDFium's CFF CID loader stores its actual SFNT program under FontFile.
                    // Follow the appearance's font graph rather than requiring one descriptor spelling.
                    [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
                        .iter()
                        .any(|key| {
                            descriptor
                                .get(key)
                                .ok()
                                .and_then(|value| pdf.dereference(value).ok())
                                .and_then(|(_, value)| value.as_stream().ok())
                                .is_some_and(|stream| {
                                    let program = stream.get_plain_content().unwrap();
                                    program.len() > 64 && program.starts_with(b"OTTO")
                                })
                        })
                })
        }),
        "CJK appearance must use its own embedded OTF program and a real ToUnicode stream"
    );
    let reopened = engine.open(&output, None).unwrap();
    evidence.save("embedded-cjk", &output, &reopened, None);
    assert_eq!(
        reopened
            .annotation_details(0)
            .unwrap()
            .iter()
            .find(|detail| detail.index == index)
            .unwrap()
            .contents
            .as_deref(),
        Some("日本語中文")
    );
    assert_eq!(
        reopened
            .render_region(0, preview.scale, preview.region)
            .unwrap()
            .rgba,
        preview.image.rgba
    );
    let before = doc.annotation_details(0).unwrap();
    assert!(doc
        .add_rich_free_text_annotation(0, None, rect, "Missing 🚀", &style, None)
        .is_err());
    assert_eq!(doc.annotation_details(0).unwrap(), before);
    assert!(
        doc.add_rich_free_text_annotation(0, None, rect, "e\u{301}", &style, None)
            .is_err(),
        "unshaped combining sequences must not be rendered silently"
    );
    assert_eq!(doc.annotation_details(0).unwrap(), before);
    let mut stale = doc
        .preview_rich_annotation(
            0,
            &sunipdf_core::annot_rich::RichAnnotation::Stamp {
                rect: PointRect {
                    left: 40.0,
                    bottom: 180.0,
                    right: 180.0,
                    top: 220.0,
                },
                kind: StampKind::Draft,
            },
            None,
        )
        .unwrap();
    let original = doc.snapshot().unwrap();
    doc.add_stamp_annotation(
        0,
        PointRect {
            left: 200.0,
            bottom: 180.0,
            right: 340.0,
            top: 220.0,
        },
        StampKind::Approved,
        None,
    )
    .unwrap();
    let changed = doc.annotation_details(0).unwrap();
    assert!(
        doc.commit_rich_annotation_preview(stale.prepared.take().unwrap())
            .is_err(),
        "stale candidate cannot replace a later document edit"
    );
    assert_eq!(doc.annotation_details(0).unwrap(), changed);
    doc.restore(original).unwrap();
    let ink = doc
        .add_ink_annotation(
            0,
            &[
                vec![(50.0, 80.0), (150.0, 80.0)],
                vec![(50.0, 120.0), (150.0, 120.0)],
            ],
            &AnnotStyle {
                opacity: 0.6,
                width_pt: 3.0,
                ..Default::default()
            },
            Some("Ink author"),
        )
        .unwrap();
    doc.set_annotation_contents(0, ink, "Keep this comment")
        .unwrap();
    doc.add_reply(0, ink, "Keep this reply", Some("Reviewer"))
        .unwrap();
    let before = doc.snapshot().unwrap();
    let remaining_region = sunipdf_core::PixelRect::new(40, 660, 120, 25);
    let before_pixels = doc.render_region(0, 1.0, remaining_region).unwrap().rgba;
    let mut preview = doc
        .preview_rich_annotation(
            0,
            &sunipdf_core::annot_rich::RichAnnotation::EraseInk {
                index: ink,
                path: vec![(100.0, 70.0), (100.0, 90.0)],
                radius: 4.0,
            },
            None,
        )
        .unwrap();
    doc.commit_rich_annotation_preview(preview.prepared.take().unwrap())
        .unwrap();
    let details = doc.annotation_details(0).unwrap();
    let detail = details.iter().find(|detail| detail.index == ink).unwrap();
    assert_eq!(detail.contents.as_deref(), Some("Keep this comment"));
    assert_eq!(detail.replies, 1);
    assert!(details
        .iter()
        .any(|detail| detail.contents.as_deref() == Some("Keep this reply")));
    assert_eq!(
        doc.render_region(0, 1.0, remaining_region).unwrap().rgba,
        before_pixels,
        "untouched stroke appearance is preserved"
    );
    let pdf = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
    let annotations = pdf
        .get_dictionary(pdf.get_pages()[&1])
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    let (_, annotation) = pdf.dereference(&annotations[ink]).unwrap();
    assert_eq!(
        annotation
            .as_dict()
            .unwrap()
            .get(b"InkList")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let unchanged = doc.annotation_details(0).unwrap();
    assert!(
        doc.erase_ink_annotation(0, ink, &[(100.0, 120.0)], 10.0)
            .is_err(),
        "all-hit requires explicit Delete"
    );
    assert_eq!(doc.annotation_details(0).unwrap(), unchanged);
    doc.restore(before).unwrap();
    assert_eq!(
        doc.annotation_details(0)
            .unwrap()
            .iter()
            .find(|detail| detail.index == ink)
            .unwrap()
            .replies,
        1
    );
    let protected = sunipdf_core::security::protect(
        &doc.snapshot().unwrap(),
        None,
        &sunipdf_core::Protection {
            user_password: "rich-user".into(),
            owner_password: "rich-owner".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let input = dir.path().join("rich-encrypted-input.pdf");
    std::fs::write(&input, &protected).unwrap();
    let mut encrypted = engine.open(&input, Some("rich-user")).unwrap();
    let mut preview = encrypted
        .preview_rich_annotation(
            0,
            &sunipdf_core::annot_rich::RichAnnotation::Stamp {
                rect: PointRect {
                    left: 40.0,
                    bottom: 180.0,
                    right: 200.0,
                    top: 230.0,
                },
                kind: StampKind::Confidential,
            },
            None,
        )
        .unwrap();
    encrypted
        .commit_rich_annotation_preview(preview.prepared.take().unwrap())
        .unwrap();
    assert!(encrypted.properties().encrypted);
    let output = dir.path().join("rich-encrypted-output.pdf");
    encrypted.save_as(&output).unwrap();
    assert!(engine.open(&output, None).is_err());
    let reopened = engine.open(&output, Some("rich-user")).unwrap();
    evidence.save("encrypted-rich", &output, &reopened, Some("rich-user"));
    assert_eq!(
        reopened
            .render_region(0, preview.scale, preview.region)
            .unwrap()
            .rgba,
        preview.image.rgba
    );
    assert_eq!(std::fs::read(input).unwrap(), protected);
    evidence.finish();
}
