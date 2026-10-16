use std::path::Path;
use sunipdf_core::Engine;

#[path = "support/conversion_images.rs"]
mod samples;

#[test]
fn jpeg_orientations_are_lossless_and_raster_batches_refuse_before_decoding_or_writing() {
    // All native conversion cases use this single PDFium engine.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut images = Vec::new();
    for exif in 1..=8 {
        let path = temp.path().join(format!("exif-{exif}.data"));
        samples::jpeg(&path, exif);
        images.push(path);
    }
    let invalid_exif = temp.path().join("invalid-exif.jpg");
    samples::jpeg(&invalid_exif, 0);
    images.push(invalid_exif);
    let png = temp.path().join("transparent.png");
    samples::transparent_png(&png);
    images.push(png);
    let originals: Vec<_> = images
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    let out = temp.path().join("images.pdf");
    assert_eq!(engine.images_to_pdf(&images, &out, 72.0).unwrap(), 10);
    let pdf = lopdf::Document::load(&out).unwrap();
    let document = engine.open(&out, None).unwrap();
    for (page, exif) in (1..=8).chain([0]).enumerate() {
        samples::assert_jpeg_bytes(&pdf, page, &originals[page]);
        let mut expected = image::ImageReader::open(&images[page])
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        expected.apply_orientation(
            image::metadata::Orientation::from_exif(exif)
                .unwrap_or(image::metadata::Orientation::NoTransforms),
        );
        let rendered = document.render_page(page, 1.0).unwrap();
        assert_eq!(
            (rendered.width, rendered.height),
            (expected.width(), expected.height())
        );
        let size = document.page_size(page).unwrap();
        assert_eq!(
            (size.width_pt, size.height_pt),
            (expected.width() as f32, expected.height() as f32)
        );
        let expected = expected.to_rgb8();
        for (x, y) in [
            (8, 8),
            (rendered.width - 9, 8),
            (8, rendered.height - 9),
            (rendered.width - 9, rendered.height - 9),
        ] {
            let offset = ((y * rendered.width + x) * 4) as usize;
            let actual = &rendered.rgba[offset..offset + 3];
            for (a, b) in actual.iter().zip(expected.get_pixel(x, y).0) {
                assert!(a.abs_diff(b) < 25, "EXIF {exif}, ({x},{y}): {actual:?}");
            }
        }
    }
    let transparent = document.render_page(9, 1.0).unwrap();
    let pixel = |x: usize| &transparent.rgba[(16 * 48 + x) * 4..(16 * 48 + x) * 4 + 3];
    assert!(pixel(8)[0] > 240 && pixel(8)[1] < 20);
    assert!(pixel(40).iter().all(|channel| *channel > 245));
    let tiny_page = temp.path().join("maximum-dpi.pdf");
    engine
        .images_to_pdf(&images[..1], &tiny_page, f32::MAX)
        .unwrap();
    let tiny = engine.open(&tiny_page, None).unwrap().page_size(0).unwrap();
    assert!(
        tiny.width_pt.is_finite()
            && tiny.width_pt > 0.0
            && tiny.height_pt.is_finite()
            && tiny.height_pt > 0.0
    );
    for (path, bytes) in images.iter().zip(&originals) {
        assert_eq!(std::fs::read(path).unwrap(), *bytes);
    }

    let header = temp.path().join("header-only.png");
    let bytes = samples::png_header(4096, 4096, 8, 6);
    std::fs::write(&header, &bytes).unwrap();
    let batch = vec![header.clone(); 9];
    let refuse = |inputs: &[std::path::PathBuf], output: &Path, dpi: f32, code: &str| {
        let sentinel = b"existing output must survive";
        std::fs::write(output, sentinel).unwrap();
        let error = engine.images_to_pdf(inputs, output, dpi).unwrap_err();
        assert_eq!(error.code(), code, "{error}");
        assert_eq!(std::fs::read(output).unwrap(), sentinel);
    };
    refuse(&batch, &out, 72.0, "image_batch_too_large");
    assert_eq!(std::fs::read(&header).unwrap(), bytes);
    let small_header = samples::png_header(8, 8, 8, 6);
    std::fs::write(&header, &small_header).unwrap();
    refuse(std::slice::from_ref(&header), &out, 72.0, "other");
    refuse(&images[..1], &out, f32::from_bits(1), "other");
    for (width, height, depth, color) in [(16384, 16384, 16, 6), (16384, 16384, 8, 0)] {
        let bytes = samples::png_header(width, height, depth, color);
        std::fs::write(&header, &bytes).unwrap();
        refuse(
            std::slice::from_ref(&header),
            &out,
            72.0,
            "image_batch_too_large",
        );
        assert_eq!(std::fs::read(&header).unwrap(), bytes);
    }
    for (path, bytes) in images.iter().zip(&originals) {
        assert_eq!(std::fs::read(path).unwrap(), *bytes);
    }
}
