use image::{DynamicImage, Rgba, RgbaImage};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use sunipdf_core::signature_image::{self, placement_rect};
use sunipdf_core::{
    security, Document, Engine, EngineError, FontFamily, ObjectKind, PageMargins, PointRect,
    Protection, RenderedPage, Rotation, ShapeKind, ShapeStyle, TextStyle,
};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

fn pixel(image: &RenderedPage, x: f32, y: f32) -> [u8; 3] {
    let offset =
        (((image.height as f32 - y).floor() as u32 * image.width + x.floor() as u32) * 4) as usize;
    image.rgba[offset..offset + 3].try_into().unwrap()
}

fn pdf_contents(
    bytes: &[u8],
    password: Option<&str>,
) -> (lopdf::Object, Vec<(lopdf::ObjectId, lopdf::Object)>) {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    if pdf.is_encrypted() {
        pdf.decrypt(password.unwrap()).unwrap();
    }
    pdf.trailer.remove(b"ID");
    (pdf.trailer.into(), pdf.objects.into_iter().collect())
}

fn typed(engine: &Engine) {
    let blue = (30, 58, 138);
    let style = TextStyle {
        size_pt: 48.0,
        family: FontFamily::Times,
        italic: true,
        rgb: blue,
        ..Default::default()
    };
    let short = engine.typed_signature("Émilie Gyjp", &style).unwrap();
    assert!(short.width() > short.height());
    assert!(short.width().max(short.height()) <= 1600);
    assert!(!signature_image::is_blank(&short));
    assert!(short.pixels().any(|p| p[3] == 0));
    assert!(short.pixels().any(|p| p[3] > 0 && p[3] < 255));
    for p in short.pixels().filter(|p| p[3] > 0) {
        assert_eq!([p[0], p[1], p[2]], [blue.0, blue.1, blue.2]);
    }
    for x in 0..short.width() {
        assert_eq!(short.get_pixel(x, 0)[3], 0, "top cropped a glyph");
        assert_eq!(
            short.get_pixel(x, short.height() - 1)[3],
            0,
            "bottom cropped a descender"
        );
    }
    let long = engine
        .typed_signature(&"Gyjp Émilie ".repeat(21), &style)
        .unwrap();
    assert!(long.width().max(long.height()) <= 1600);
    assert!(
        long.height() < short.height(),
        "a long name must be fitted to the raster budget"
    );
    assert!(!signature_image::is_blank(&long));
    for text in [
        "",
        " ",
        "a\nb",
        "a\tb",
        "a\rb",
        "a\u{0}b",
        "a\u{2028}b",
        "a\u{2029}b",
        &"x".repeat(257),
    ] {
        assert!(engine.typed_signature(text, &style).is_err(), "{text:?}");
    }
    for size_pt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(engine
            .typed_signature(
                "Name",
                &TextStyle {
                    size_pt,
                    ..style.clone()
                }
            )
            .is_err());
    }
    for opacity in [0.0, -1.0, f32::NAN, f32::INFINITY, 2.0] {
        assert!(engine
            .typed_signature(
                "Name",
                &TextStyle {
                    opacity,
                    ..style.clone()
                }
            )
            .is_err());
    }
    assert!(matches!(
        engine.typed_signature("日本", &style),
        Err(EngineError::MissingGlyphs(_))
    ));
    let temp = tempfile::tempdir().unwrap();
    let invalid_font = temp.path().join("invalid.ttf");
    std::fs::write(&invalid_font, b"not a font").unwrap();
    assert!(matches!(
        engine.typed_signature(
            "Name",
            &TextStyle {
                font_file: Some(invalid_font.clone()),
                ..style.clone()
            }
        ),
        Err(EngineError::FontNotEmbeddable)
    ));
    // A restricted OS/2.fsType must be refused before PDFium sees the synthetic font.
    let mut restricted = vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    restricted.extend_from_slice(b"OS/2");
    restricted.extend_from_slice(&[0; 4]);
    restricted.extend_from_slice(&28u32.to_be_bytes());
    restricted.extend_from_slice(&10u32.to_be_bytes());
    restricted.extend_from_slice(&[0; 8]);
    restricted.extend_from_slice(&2u16.to_be_bytes());
    std::fs::write(&invalid_font, restricted).unwrap();
    assert!(matches!(
        engine.typed_signature(
            "Name",
            &TextStyle {
                font_file: Some(invalid_font),
                ..style.clone()
            }
        ),
        Err(EngineError::FontNotEmbeddable)
    ));
    let fonts = sunipdf_core::fonts::system_fonts();
    if let Some(font) = ["Segoe Script", "Gabriola", "Arial", "Segoe UI"]
        .iter()
        .find_map(|name| fonts.iter().find(|font| font.name == *name))
    {
        let native = engine
            .typed_signature(
                "Émilie Gyjp",
                &TextStyle {
                    font_file: Some(font.path.clone()),
                    ..style
                },
            )
            .unwrap();
        assert!(!signature_image::is_blank(&native));
        let saved = temp.path().join("signature.png");
        signature_image::save_png(&native, &saved).unwrap();
        assert_eq!(signature_image::load(&saved, 1600).unwrap(), native);
    } else {
        eprintln!(
            "no installed handwriting/Arial/Segoe UI TTF: optional system-font check skipped"
        );
    }
}

fn mixed_pages(engine: &Engine, rotation: Rotation) -> Vec<u8> {
    let mut doc = engine
        .open_bytes(PDF.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.crop_page(
        0,
        PageMargins {
            left: 100.0,
            bottom: 50.0,
            right: 112.0,
            top: 142.0,
        },
    )
    .unwrap();
    doc.set_page_rotation(0, rotation).unwrap();
    doc.insert_blank_pages(2, 1, 80.0, 40.0).unwrap();
    for page in [0, 2] {
        let size = doc.page_size(page).unwrap();
        doc.add_shape(
            page,
            ShapeKind::Rectangle,
            PointRect {
                left: 0.0,
                bottom: 0.0,
                right: size.width_pt,
                top: size.height_pt,
            },
            &ShapeStyle {
                stroke_rgb: (0, 180, 0),
                fill_rgb: Some((0, 180, 0)),
                stroke_width: 0.1,
                opacity: 1.0,
            },
        )
        .unwrap();
    }
    doc.snapshot().unwrap()
}

fn check_placement(doc: &Document<'_>, image: &DynamicImage, original_counts: &[usize]) {
    for page in [0, 2] {
        let size = doc.page_size(page).unwrap();
        let expected = placement_rect(
            (300.0, 200.0),
            160.0,
            image.height() as f32 / image.width() as f32,
            (size.width_pt, size.height_pt),
        )
        .unwrap();
        let objects = doc.page_objects(page).unwrap();
        assert_eq!(objects.len(), original_counts[page] + 1);
        let placed = objects.last().unwrap();
        assert_eq!(placed.kind, ObjectKind::Image);
        assert!(placed.angle_deg.abs() < 0.01);
        for (actual, expected) in [
            (placed.bounds.left, expected.left),
            (placed.bounds.bottom, expected.bottom),
            (placed.bounds.right, expected.right),
            (placed.bounds.top, expected.top),
        ] {
            assert!((actual - expected).abs() < 0.1, "{actual} != {expected}");
        }
        let rendered = doc.render_page(page, 1.0).unwrap();
        let width = expected.right - expected.left;
        let y = (expected.bottom + expected.top) / 2.0;
        assert_eq!(
            pixel(&rendered, expected.left + width * 0.1, y),
            [0, 180, 0],
            "transparent pixels must retain underlying page content"
        );
        assert_eq!(
            pixel(&rendered, expected.left + width * 0.5, y),
            [30, 58, 138]
        );
    }
}

#[test]
fn typed_signatures_and_atomic_batch_placement_round_trip_with_pdfium() {
    // One engine in this process: PDFium cannot safely be destroyed and reinitialized between tests.
    let engine = Engine::new().unwrap();
    typed(&engine);
    let signature = DynamicImage::ImageRgba8(RgbaImage::from_fn(100, 40, |x, y| {
        Rgba([
            30,
            58,
            138,
            if (40..60).contains(&x) && (10..30).contains(&y) {
                255
            } else {
                0
            },
        ])
    }));
    let temp = tempfile::tempdir().unwrap();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let plain = mixed_pages(&engine, rotation);
        for (encryption, password) in [
            (None, None),
            (Some(false), Some("user")),
            (Some(false), Some("owner")),
            (Some(true), Some("user")),
            (Some(true), Some("owner")),
        ] {
            let bytes = match encryption {
                Some(aes256) => security::protect(
                    &plain,
                    None,
                    &Protection {
                        user_password: "user".into(),
                        owner_password: "owner".into(),
                        allow_print: false,
                        allow_copy: false,
                        allow_modify: false,
                        allow_annotate: false,
                        aes256,
                    },
                )
                .unwrap(),
                None => plain.clone(),
            };
            let source = temp.path().join("source.pdf");
            std::fs::write(&source, &bytes).unwrap();
            let mut doc = engine.open(&source, password).unwrap();
            let before = doc.snapshot().unwrap();
            let original_counts: Vec<_> = (0..3)
                .map(|page| doc.page_objects(page).unwrap().len())
                .collect();
            let untouched = doc.render_page(1, 0.5).unwrap().rgba;
            let before_properties = doc.properties();
            let cancel = AtomicBool::new(false);
            for pages in [&[][..], &[0, usize::MAX][..]] {
                assert!(doc
                    .place_signature_image(
                        pages,
                        &signature,
                        (300.0, 200.0),
                        160.0,
                        &cancel,
                        |_, _| panic!("validation must precede mutation")
                    )
                    .is_err());
            }
            assert_eq!(
                pdf_contents(&doc.snapshot().unwrap(), password),
                pdf_contents(&before, password)
            );
            assert!(matches!(
                doc.place_signature_image(
                    &[2, 0],
                    &signature,
                    (300.0, 200.0),
                    160.0,
                    &cancel,
                    |done, total| {
                        assert_eq!(total, 2);
                        if done == 1 {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                ),
                Err(EngineError::Cancelled)
            ));
            assert_eq!(
                pdf_contents(&doc.snapshot().unwrap(), password),
                pdf_contents(&before, password),
                "cancellation after the first page must discard the whole batch"
            );
            cancel.store(false, Ordering::Relaxed);
            assert!(
                matches!(
                    doc.place_signature_image(
                        &[0],
                        &signature,
                        (300.0, 200.0),
                        160.0,
                        &cancel,
                        |_, _| cancel.store(true, Ordering::Relaxed)
                    ),
                    Err(EngineError::Cancelled)
                ),
                "cancellation after the final page must precede commit"
            );
            assert_eq!(
                pdf_contents(&doc.snapshot().unwrap(), password),
                pdf_contents(&before, password)
            );
            cancel.store(false, Ordering::Relaxed);
            let mut progress = Vec::new();
            assert_eq!(
                doc.place_signature_image(
                    &[2, 0, 2],
                    &signature,
                    (300.0, 200.0),
                    160.0,
                    &cancel,
                    |done, total| progress.push((done, total))
                )
                .unwrap(),
                2
            );
            assert_eq!(progress, [(1, 2), (2, 2)]);
            assert_eq!(doc.info().path, source);
            assert_eq!(doc.password(), password);
            assert_eq!(doc.properties(), before_properties);
            assert_eq!(doc.render_page(1, 0.5).unwrap().rgba, untouched);
            check_placement(&doc, &signature, &original_counts);
            let after = doc.snapshot().unwrap();
            doc.restore(before.clone()).unwrap();
            for (page, count) in original_counts.iter().enumerate() {
                assert_eq!(doc.page_objects(page).unwrap().len(), *count);
            }
            doc.restore(after).unwrap();
            let output = temp.path().join("signed.pdf");
            doc.save_as(&output).unwrap();
            assert_eq!(
                std::fs::read(&source).unwrap(),
                bytes,
                "batch placement never writes the source file"
            );
            let stored = lopdf::Document::load(&output).unwrap();
            assert_eq!(stored.is_encrypted(), encryption.is_some());
            for reopened_password in if encryption.is_some() {
                vec![Some("user"), Some("owner")]
            } else {
                vec![None]
            } {
                let reopened = engine.open(&output, reopened_password).unwrap();
                check_placement(&reopened, &signature, &original_counts);
                assert_eq!(reopened.render_page(1, 0.5).unwrap().rgba, untouched);
                assert_eq!(reopened.info().page_sizes, doc.info().page_sizes);
                if encryption.is_some() {
                    let props = reopened.properties();
                    assert!(props.encrypted);
                    assert!(
                        !props.can_copy
                            && !props.can_print
                            && !props.can_modify
                            && !props.can_annotate
                    );
                }
            }
            if let Some(qpdf) = std::env::var_os("SUNIPDF_QPDF") {
                let mut command = std::process::Command::new(qpdf);
                if encryption.is_some() {
                    command.arg("--password=user");
                }
                let result = command.arg("--check").arg(&output).output().unwrap();
                assert!(
                    result.status.success(),
                    "{}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
    }
}
