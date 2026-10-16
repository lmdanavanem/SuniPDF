use lopdf::{dictionary, Dictionary, Object};
use std::path::{Path, PathBuf};
use sunipdf_core::{
    security, AnnotShape, AnnotStyle, Document, Engine, FreeTextStyle, PointRect, Protection,
    RenderedPage, TextStyle,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn rect(left: f32, bottom: f32, right: f32, top: f32) -> PointRect {
    PointRect {
        left,
        bottom,
        right,
        top,
    }
}

fn style() -> AnnotStyle {
    AnnotStyle {
        rgb: (0, 0, 255),
        width_pt: 2.0,
        dash_pattern: vec![12.0, 8.0, 0.0, 4.25],
        ..Default::default()
    }
}

fn decoded(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or("")),
    )
    .unwrap()
}

fn annotations(pdf: &lopdf::Document, page: usize) -> Vec<Dictionary> {
    let id = pdf.get_pages()[&(page as u32 + 1)];
    pdf.get_dictionary(id)
        .unwrap()
        .get_deref(b"Annots", pdf)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| pdf.dereference(value).unwrap().1.as_dict().unwrap().clone())
        .collect()
}

fn unchanged(before: &[u8], after: &[u8], password: Option<&str>) {
    let mut before = decoded(before, password);
    let mut after = decoded(after, password);
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(before.trailer, after.trailer);
    assert_eq!(before.objects, after.objects);
}

fn qpdf(path: &Path, password: Option<&str>) {
    if let Some(executable) = std::env::var_os("SUNIPDF_QPDF") {
        let mut command = std::process::Command::new(executable);
        if let Some(password) = password {
            command.arg(format!("--password={password}"));
        }
        let result = command.arg("--check").arg(path).output().unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn pixel(image: &RenderedPage, x: usize, y: usize) -> [u8; 3] {
    let offset = ((image.height as usize - 1 - y) * image.width as usize + x) * 4;
    image.rgba[offset..offset + 3].try_into().unwrap()
}

fn check_style(doc: &Document<'_>, page: usize, index: usize, expected: &[f32], arrow: bool) {
    let details = doc.annotation_details(page).unwrap();
    assert_eq!(details[index].dash_pattern, expected);
    let pdf = decoded(&doc.snapshot().unwrap(), doc.password());
    let raw = annotations(&pdf, page);
    let annotation = &raw[index];
    let bs = annotation
        .get_deref(b"BS", &pdf)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(
        bs.get(b"S").unwrap().as_name().unwrap(),
        if expected.is_empty() { b"S" } else { b"D" }
    );
    if expected.is_empty() {
        assert!(bs.get(b"D").is_err());
    } else {
        let pattern: Vec<_> = bs
            .get(b"D")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect();
        assert_eq!(pattern, expected);
    }
    let ap = annotation
        .get_deref(b"AP", &pdf)
        .unwrap()
        .as_dict()
        .unwrap()
        .get_deref(b"N", &pdf)
        .unwrap()
        .as_stream()
        .unwrap();
    let bytes = if ap.dict.get(b"Filter").is_ok() {
        ap.decompressed_content().unwrap()
    } else {
        ap.content.clone()
    };
    let operations = lopdf::content::Content::decode(&bytes).unwrap().operations;
    let dashes: Vec<_> = operations
        .iter()
        .filter(|operation| operation.operator == "d")
        .collect();
    if expected.is_empty() {
        assert!(dashes.is_empty());
    } else {
        assert_eq!(dashes.len(), if arrow { 2 } else { 1 });
        let values: Vec<_> = dashes[0].operands[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect();
        assert_eq!(values, expected, "appearance and /BS disagree");
        assert_eq!(dashes[0].operands[1].as_i64().unwrap(), 0);
        if arrow {
            assert!(dashes[1].operands[0].as_array().unwrap().is_empty());
        }
    }
}

fn shapes_and_geometry(engine: &Engine) {
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let line = doc
        .add_shape_annotation(
            0,
            AnnotShape::Line,
            rect(80.0, 140.0, 280.0, 140.0),
            &style(),
            Some("Léa"),
        )
        .unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    let blue = pixel(&img, 86, 140);
    assert!(
        blue[2] > 200 && blue[0] < 60 && blue[1] < 60,
        "dash: {blue:?}"
    );
    assert_eq!(
        pixel(&img, 96, 140),
        [255, 255, 255],
        "gap must remain visible"
    );
    check_style(&doc, 0, line, &style().dash_pattern, false);
    let mut filled = style();
    filled.fill_rgb = Some((255, 255, 0));
    let square = doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(80.0, 200.0, 280.0, 300.0),
            &filled,
            None,
        )
        .unwrap();
    let circle = doc
        .add_shape_annotation(
            0,
            AnnotShape::Circle,
            rect(310.0, 200.0, 410.0, 300.0),
            &style(),
            None,
        )
        .unwrap();
    let arrow = doc
        .add_shape_annotation(
            0,
            AnnotShape::Arrow,
            rect(80.0, 400.0, 280.0, 400.0),
            &style(),
            None,
        )
        .unwrap();
    assert_eq!(
        pixel(&doc.render_page(0, 1.0).unwrap(), 180, 250),
        [255, 255, 0]
    );
    for (index, arrow) in [(square, false), (circle, false), (arrow, true)] {
        check_style(&doc, 0, index, &style().dash_pattern, arrow);
    }
    doc.move_annotation(0, line, 10.0, 20.0).unwrap();
    check_style(&doc, 0, line, &style().dash_pattern, false);
    doc.resize_annotation(0, line, rect(100.0, 150.0, 350.0, 180.0))
        .unwrap();
    let mut changed = style();
    changed.rgb = (255, 0, 0);
    changed.width_pt = 3.0;
    doc.set_annotation_style(0, line, &changed).unwrap();
    check_style(&doc, 0, line, &changed.dash_pattern, false);
    changed.dash_pattern.clear();
    doc.set_annotation_style(0, line, &changed).unwrap();
    check_style(&doc, 0, line, &[], false);

    for legacy in [false, true] {
        let mut pdf = decoded(&doc.snapshot().unwrap(), None);
        let page = pdf.get_pages()[&1];
        let id = pdf
            .get_dictionary(page)
            .unwrap()
            .get_deref(b"Annots", &pdf)
            .unwrap()
            .as_array()
            .unwrap()[line]
            .as_reference()
            .unwrap();
        let annotation = pdf.get_dictionary_mut(id).unwrap();
        let expected = if legacy {
            vec![2.0, 1.0, 4.25]
        } else {
            vec![3.0]
        };
        if legacy {
            annotation.remove(b"BS");
            annotation.set(
                "Border",
                Object::Array(vec![
                    0.into(),
                    0.into(),
                    2.into(),
                    Object::Array(expected.iter().map(|v| Object::Real(*v)).collect()),
                ]),
            );
        } else {
            annotation.set("BS", dictionary! {"S" => "D", "W" => 2});
        }
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut imported = engine
            .open_bytes(bytes, None, Path::new("default-or-legacy-border.pdf"))
            .unwrap();
        assert_eq!(
            imported.annotation_details(0).unwrap()[line].dash_pattern,
            expected
        );
        imported
            .resize_annotation(0, line, rect(100.0, 150.0, 350.0, 180.0))
            .unwrap();
        check_style(&imported, 0, line, &expected, false);
        let mut recolored = style();
        recolored.dash_pattern = expected.clone();
        imported.set_annotation_style(0, line, &recolored).unwrap();
        check_style(&imported, 0, line, &expected, false);
    }

    let mut boxes = engine.open(&fixture("boxes.pdf"), None).unwrap();
    for page in 0..boxes.page_count() {
        let target = rect(40.0, 50.0, 160.0, 110.0);
        let index = boxes
            .add_shape_annotation(page, AnnotShape::Square, target, &style(), None)
            .unwrap();
        check_style(&boxes, page, index, &style().dash_pattern, false);
        let actual = boxes.annotation_details(page).unwrap()[index].rect;
        assert!([
            actual.left - target.left,
            actual.bottom - target.bottom,
            actual.right - target.right,
            actual.top - target.top
        ]
        .iter()
        .all(|v| v.abs() < 0.01));
        let image = boxes.render_page(page, 1.0).unwrap();
        let blue = pixel(&image, 47, 51);
        assert!(blue[2] > 180 && blue[0] < 100, "page {page}: {blue:?}");
        boxes.move_annotation(page, index, 15.0, 10.0).unwrap();
        boxes
            .resize_annotation(page, index, rect(60.0, 70.0, 200.0, 150.0))
            .unwrap();
        check_style(&boxes, page, index, &style().dash_pattern, false);
    }
}

fn protected_round_trips(engine: &Engine) {
    let temp = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture("acroform-links.pdf")).unwrap();
    for (case, (aes, password)) in [
        (None, None),
        (Some(false), Some("user")),
        (Some(false), Some("owner")),
        (Some(true), Some("user")),
        (Some(true), Some("owner")),
    ]
    .into_iter()
    .enumerate()
    {
        let input = match aes {
            Some(aes256) => security::protect(
                &original,
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
            None => original.clone(),
        };
        let source = temp.path().join(format!("source-{case}.pdf"));
        std::fs::write(&source, &input).unwrap();
        let mut doc = engine.open(&source, password).unwrap();
        doc.add_text(1, 40.0, 40.0, "Unsaved content", &TextStyle::default())
            .unwrap();
        let form = doc.form_info().unwrap();
        let bookmarks = doc.bookmarks();
        let properties = doc.properties();
        let sizes = doc.info().page_sizes.clone();
        let text: Vec<_> = (0..doc.page_count())
            .map(|page| doc.page_text(page).unwrap())
            .collect();
        let untouched_render = doc.render_page(1, 0.5).unwrap().rgba;
        let before = annotations(&decoded(&doc.snapshot().unwrap(), password), 0);
        let index = doc
            .add_shape_annotation(
                0,
                AnnotShape::Square,
                rect(30.0, 30.0, 170.0, 90.0),
                &style(),
                Some("Léa"),
            )
            .unwrap();
        assert_eq!(index, before.len());
        doc.move_annotation(0, index, 10.0, 5.0).unwrap();
        doc.resize_annotation(0, index, rect(45.0, 45.0, 205.0, 115.0))
            .unwrap();
        let mut recolored = style();
        recolored.rgb = (200, 20, 20);
        doc.set_annotation_style(0, index, &recolored).unwrap();
        check_style(&doc, 0, index, &recolored.dash_pattern, false);
        let after = annotations(&decoded(&doc.snapshot().unwrap(), password), 0);
        assert_eq!(&after[..index], &before);
        assert_eq!(doc.properties(), properties);
        assert_eq!(doc.info().page_sizes, sizes);
        assert_eq!(doc.form_info().unwrap(), form);
        assert_eq!(doc.bookmarks(), bookmarks);
        assert_eq!(doc.render_page(1, 0.5).unwrap().rgba, untouched_render);
        let rendered = doc.render_page(0, 0.5).unwrap().rgba;
        let output = temp.path().join(format!("dashed-{case}.pdf"));
        doc.save_as(&output).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), input);
        for reopen_password in if aes.is_some() {
            vec![Some("user"), Some("owner")]
        } else {
            vec![None]
        } {
            qpdf(&output, reopen_password);
            let reopened = engine.open(&output, reopen_password).unwrap();
            check_style(&reopened, 0, index, &recolored.dash_pattern, false);
            assert_eq!(reopened.properties().encrypted, aes.is_some());
            assert_eq!(reopened.properties().can_copy, properties.can_copy);
            assert_eq!(reopened.properties().can_print, properties.can_print);
            assert_eq!(reopened.form_info().unwrap(), form);
            assert_eq!(reopened.bookmarks(), bookmarks);
            assert_eq!(reopened.render_page(0, 0.5).unwrap().rgba, rendered);
            assert_eq!(reopened.render_page(1, 0.5).unwrap().rgba, untouched_render);
            for (page, expected) in text.iter().enumerate() {
                assert_eq!(&reopened.page_text(page).unwrap(), expected);
            }
        }
        if aes.is_some() {
            assert!(engine.open(&output, None).is_err());
        }
    }
}

fn invalid_and_foreign_are_atomic(engine: &Engine) {
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let index = doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(40.0, 40.0, 140.0, 100.0),
            &style(),
            None,
        )
        .unwrap();
    let before = doc.snapshot().unwrap();
    for pattern in [
        vec![0.0],
        vec![-1.0],
        vec![f32::NAN],
        vec![f32::INFINITY],
        vec![0.01],
        vec![1001.0],
        vec![1.0; 17],
    ] {
        let invalid = AnnotStyle {
            dash_pattern: pattern,
            ..style()
        };
        assert!(doc.set_annotation_style(0, index, &invalid).is_err());
        assert!(doc
            .add_shape_annotation(
                0,
                AnnotShape::Square,
                rect(40.0, 40.0, 140.0, 100.0),
                &invalid,
                None
            )
            .is_err());
        unchanged(&before, &doc.snapshot().unwrap(), None);
    }
    assert!(doc
        .add_ink_annotation(0, &[vec![(20.0, 20.0), (40.0, 40.0)]], &style(), None)
        .is_err());
    let free = doc
        .add_free_text_annotation(
            0,
            rect(20.0, 20.0, 150.0, 60.0),
            "Keep me",
            &FreeTextStyle::default(),
            None,
        )
        .unwrap();
    let before = doc.snapshot().unwrap();
    assert!(doc.set_annotation_style(0, free, &style()).is_err());
    assert!(doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(0.0, 0.0, 1.0e9, 1.0e9),
            &AnnotStyle {
                dash_pattern: vec![0.1, 0.1],
                ..style()
            },
            None
        )
        .is_err());
    unchanged(&before, &doc.snapshot().unwrap(), None);

    let mut foreign = engine.open(&fixture("annotated.pdf"), None).unwrap();
    let before = foreign.snapshot().unwrap();
    assert!(foreign.set_annotation_style(0, 0, &style()).is_err());
    assert!(foreign
        .resize_annotation(0, 0, rect(20.0, 20.0, 80.0, 80.0))
        .is_err());
    unchanged(&before, &foreign.snapshot().unwrap(), None);
    assert!(
        foreign.set_annotation_style(0, 3, &style()).is_err(),
        "markup cannot silently ignore dashes"
    );
    unchanged(&before, &foreign.snapshot().unwrap(), None);

    for border in [
        dictionary! {"S" => "B"}.into(),
        dictionary! {"S" => "D", "D" => Object::Null}.into(),
        dictionary! {"S" => "D", "D" => vec![Object::Real(0.01)]}.into(),
        Object::Null,
    ] {
        let mut pdf = decoded(&doc.snapshot().unwrap(), None);
        let page = pdf.get_pages()[&1];
        let id = pdf
            .get_dictionary(page)
            .unwrap()
            .get_deref(b"Annots", &pdf)
            .unwrap()
            .as_array()
            .unwrap()[index]
            .as_reference()
            .unwrap();
        pdf.get_dictionary_mut(id).unwrap().set("BS", border);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut malformed = engine
            .open_bytes(bytes, None, Path::new("malformed-border.pdf"))
            .unwrap();
        let details = malformed.annotation_details(0).unwrap();
        assert!(!details[index].can_move);
        assert!(!details[index].can_resize);
        assert!(!details[index].can_restyle);
        assert!(
            details[index].can_edit_text,
            "shape comments do not regenerate the border"
        );
        let before = malformed.snapshot().unwrap();
        assert!(malformed.move_annotation(0, index, 10.0, 10.0).is_err());
        assert!(malformed
            .resize_annotation(0, index, rect(50.0, 50.0, 180.0, 120.0))
            .is_err());
        assert!(malformed.set_annotation_style(0, index, &style()).is_err());
        unchanged(&before, &malformed.snapshot().unwrap(), None);
    }
}

#[test]
fn dashed_shapes_preserve_geometry_appearances_and_protection() {
    // PDFium has one process-global instance; every native scenario shares this engine.
    let engine = Engine::new().unwrap();
    shapes_and_geometry(&engine);
    protected_round_trips(&engine);
    invalid_and_foreign_are_atomic(&engine);
}
