use lopdf::{dictionary, Document as Pdf, Stream};
use sunipdf_core::{Engine, ObjectKind, ObjectStyle};

#[test]
fn existing_image_alpha_is_reported_as_unknown_without_changing_rendering() {
    let mut pdf = Pdf::load_mem(include_bytes!("fixtures/two-pages.pdf")).unwrap();
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        vec![0, 160, 0],
    ));
    let gs = pdf.add_object(dictionary! {"Type" => "ExtGState", "ca" => 0.25, "CA" => 0.25});
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"q /Alpha gs 100 0 0 50 200 200 cm /Image Do Q".to_vec(),
    ));
    let page = pdf.get_pages()[&1];
    let page = pdf.get_dictionary_mut(page).unwrap();
    page.set("Contents", content);
    page.set(
        "Resources",
        dictionary! {
            "XObject" => dictionary! {"Image" => image},
            "ExtGState" => dictionary! {"Alpha" => gs},
        },
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("image-alpha.pdf");
    std::fs::write(&source, &bytes).unwrap();
    let engine = Engine::new().unwrap();
    let mut doc = engine.open(&source, None).unwrap();
    let info = doc.object_info(0, 0).unwrap();
    assert_eq!(info.kind, ObjectKind::Image);
    assert_eq!(info.opacity, None);
    let pixels = doc.render_page(0, 1.0).unwrap();
    let at = (((pixels.height - 225) * pixels.width + 250) * 4) as usize;
    let pixel = &pixels.rgba[at..at + 3];
    for (actual, expected) in pixel.iter().zip([191, 231, 191]) {
        assert!(actual.abs_diff(expected) <= 3, "unexpected blend {pixel:?}");
    }
    let before = doc.snapshot().unwrap();
    doc.set_object_style(
        0,
        0,
        &ObjectStyle {
            fill_rgb: Some((240, 10, 20)),
            stroke_rgb: Some((20, 10, 240)),
            stroke_width: Some(4.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(doc.object_info(0, 0).unwrap().opacity, None);
    assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels.rgba);
    doc.set_object_style(
        0,
        0,
        &ObjectStyle {
            opacity: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(doc.object_info(0, 0).unwrap().opacity, Some(1.0));
    let opaque = doc.render_page(0, 1.0).unwrap();
    assert_eq!(&opaque.rgba[at..at + 3], &[0, 160, 0]);
    doc.restore(before).unwrap();
    assert_eq!(doc.object_info(0, 0).unwrap().opacity, None);
    assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels.rgba);
    let output = dir.path().join("saved.pdf");
    doc.save_as(&output).unwrap();
    let reopened = engine.open(&output, None).unwrap();
    assert_eq!(reopened.object_info(0, 0).unwrap().opacity, None);
    assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, pixels.rgba);
    assert_eq!(std::fs::read(source).unwrap(), bytes);
}
