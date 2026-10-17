use lopdf::{dictionary, Document as Pdf, Stream};
use std::path::Path;
use sunipdf_core::{Engine, ObjectInfo};

fn fixture(rotation: i32) -> Vec<u8> {
    let mut pdf = Pdf::load_mem(include_bytes!("fixtures/two-pages.pdf")).unwrap();
    let inner = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
            "BBox" => vec![0.into(), 0.into(), 140.into(), 80.into()],
            "Resources" => dictionary! {},
        },
        b"0.1 0.2 0.8 rg 0 0 m 140 0 l 10 80 l h f".to_vec(),
    ));
    let outer = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
            "BBox" => vec![0.into(), 0.into(), 160.into(), 100.into()],
            "Resources" => dictionary! {"XObject" => dictionary! {"Inner" => inner}},
        },
        b"q 1 0 0 1 10 10 cm /Inner Do Q".to_vec(),
    ));
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"0.8 0.1 0.1 rg 180 170 m 320 170 l 190 250 l h f
          0.1 0.7 0.1 rg 350 160 m 380 280 l 345 200 l h f
          q 1 0 0 1 200 340 cm /Outer Do Q"
            .to_vec(),
    ));
    let page = pdf.get_pages()[&1];
    let page = pdf.get_dictionary_mut(page).unwrap();
    page.set("Rotate", rotation);
    page.set(
        "CropBox",
        vec![100.into(), 50.into(), 500.into(), 650.into()],
    );
    page.set("Contents", content);
    page.set(
        "Resources",
        dictionary! {"XObject" => dictionary! {"Outer" => outer}},
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn assert_geometry(actual: &[ObjectInfo], expected: &[ObjectInfo]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.kind, expected.kind);
        for (a, b) in [
            (actual.bounds.left, expected.bounds.left),
            (actual.bounds.bottom, expected.bounds.bottom),
            (actual.bounds.right, expected.bounds.right),
            (actual.bounds.top, expected.bounds.top),
        ] {
            assert!(
                (a - b).abs() < 0.04,
                "geometry drift: {actual:?} vs {expected:?}"
            );
        }
    }
}

#[test]
fn repeated_path_group_and_nested_form_rotation_keep_one_pivot_after_reopen_and_restore() {
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let bytes = fixture(rotation);
        let source = dir.path().join(format!("rotation-source-{rotation}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        for indices in [vec![0], vec![0, 1], vec![2], vec![0, 2]] {
            let mut doc = engine.open(&source, None).unwrap();
            let original = doc.page_objects(0).unwrap();
            let original_pixels = doc.render_page(0, 1.0).unwrap().rgba;
            let initial = doc.snapshot().unwrap();
            let pivot = doc.rotation_center(0, &indices).unwrap();
            let mut total = 0.0;
            for (step, degrees) in [37.0, -17.0, 29.0, -11.0, -38.0].into_iter().enumerate() {
                let previous = doc.snapshot().unwrap();
                let previous_objects = doc.page_objects(0).unwrap();
                let previous_pixels = doc.render_page(0, 1.0).unwrap().rgba;
                doc.rotate_objects(0, &indices, degrees).unwrap();
                let current_pivot = doc.rotation_center(0, &indices).unwrap();
                assert!(
                    (pivot.0 - current_pivot.0).abs() < 0.02
                        && (pivot.1 - current_pivot.1).abs() < 0.02
                );
                total += degrees;
                let mut reference = engine
                    .open_bytes(bytes.clone(), None, Path::new("reference.pdf"))
                    .unwrap();
                reference.rotate_objects(0, &indices, total).unwrap();
                assert_geometry(
                    &doc.page_objects(0).unwrap(),
                    &reference.page_objects(0).unwrap(),
                );
                let actual_pixels = doc.render_page(0, 1.0).unwrap().rgba;
                let expected_pixels = reference.render_page(0, 1.0).unwrap().rgba;
                let difference: u64 = actual_pixels
                    .iter()
                    .zip(&expected_pixels)
                    .map(|(a, b)| a.abs_diff(*b) as u64)
                    .sum();
                assert!(difference < 4_000, "rotation render drift: {difference}");
                let redo = doc.snapshot().unwrap();
                doc.restore(previous).unwrap();
                assert_geometry(&doc.page_objects(0).unwrap(), &previous_objects);
                assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, previous_pixels);
                doc.restore(redo).unwrap();
                if step == 1 {
                    let output = dir
                        .path()
                        .join(format!("rotated-{rotation}-{}.pdf", indices.len()));
                    doc.save_as(&output).unwrap();
                    doc = engine.open(&output, None).unwrap();
                }
            }
            assert_eq!(total, 0.0);
            assert_geometry(&doc.page_objects(0).unwrap(), &original);
            doc.restore(initial).unwrap();
            assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original_pixels);
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
            let before_invalid = doc.page_objects(0).unwrap();
            assert!(doc.rotate_objects(0, &[0, usize::MAX], 20.0).is_err());
            assert!(doc.rotate_objects(0, &indices, f32::NAN).is_err());
            assert_eq!(doc.page_objects(0).unwrap(), before_invalid);
        }
    }
    let mut deep = Pdf::load_mem(&fixture(0)).unwrap();
    let mut form = deep.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {},
        },
        b"0 0 m 100 0 l 10 80 l h f".to_vec(),
    ));
    for _ in 0..34 {
        form = deep.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
                "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
                "Resources" => dictionary! {"XObject" => dictionary! {"Child" => form}},
            },
            b"/Child Do".to_vec(),
        ));
    }
    let content = deep.add_object(Stream::new(dictionary! {}, b"/Deep Do".to_vec()));
    let page = deep.get_pages()[&1];
    let page = deep.get_dictionary_mut(page).unwrap();
    page.set("Contents", content);
    page.set(
        "Resources",
        dictionary! {"XObject" => dictionary! {"Deep" => form}},
    );
    let mut bytes = Vec::new();
    deep.save_to(&mut bytes).unwrap();
    let mut doc = engine
        .open_bytes(bytes, None, Path::new("deep.pdf"))
        .unwrap();
    let before = doc.page_objects(0).unwrap();
    let pixels = doc.render_page(0, 1.0).unwrap().rgba;
    assert!(doc.rotation_center(0, &[0]).is_err());
    assert!(doc.rotate_object(0, 0, 30.0).is_err());
    assert_eq!(doc.page_objects(0).unwrap(), before);
    assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
}
