use lopdf::{dictionary, Dictionary, Object, Stream};
use sunipdf_core::{security, Engine, PageGeometry, Protection, ResizeMode};

fn fixture(rotation: i64, defect: Option<&str>) -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"0 0 1 rg 100 150 120 80 re f".to_vec(),
    ));
    let link = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![100.into(),150.into(),220.into(),230.into()],
        "QuadPoints" => vec![100.into(),230.into(),220.into(),230.into(),100.into(),150.into(),220.into(),150.into()],
        "NM" => Object::string_literal("unchanged-link"),
        "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://example.org/document") },
    });
    let ink = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Ink", "Rect" => vec![80.into(),130.into(),240.into(),250.into()],
        "InkList" => vec![Object::Array(vec![100.into(),150.into(),220.into(),230.into()])],
        "C" => vec![1.into(),0.into(),0.into()],
    });
    let line = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Line", "Rect" => vec![80.into(),130.into(),240.into(),250.into()],
        "L" => vec![100.into(),150.into(),220.into(),230.into()],
    });
    let free_text = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "FreeText", "Rect" => vec![80.into(),130.into(),240.into(),250.into()],
        "CL" => vec![100.into(),150.into(),120.into(),180.into(),220.into(),230.into()],
        "DA" => Object::string_literal("/Helvetica 10 Tf 0 g"),
        "Contents" => Object::string_literal("Callout"),
    });
    let polygon = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Polygon", "Rect" => vec![80.into(),130.into(),240.into(),250.into()],
        "Vertices" => vec![100.into(),150.into(),120.into(),210.into(),220.into(),230.into()],
    });
    if defect == Some("locked") {
        pdf.get_dictionary_mut(link).unwrap().set("F", 128);
    }
    if defect == Some("malformed") {
        pdf.get_dictionary_mut(link)
            .unwrap()
            .set("QuadPoints", vec![1.into(), 2.into(), 3.into()]);
    }
    if defect == Some("unlisted-popup") {
        let popup = pdf.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Popup",
            "Rect" => vec![100.into(),150.into(),220.into(),230.into()], "Parent" => link,
        });
        pdf.get_dictionary_mut(link).unwrap().set("Popup", popup);
    }
    let first = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![10.into(),20.into(),610.into(),820.into()],
        "CropBox" => vec![30.into(),40.into(),530.into(),740.into()], "Rotate" => rotation,
        "Resources" => Dictionary::new(), "Contents" => content,
        "Annots" => vec![link.into(),ink.into(),line.into(),free_text.into(),polygon.into(),link.into()],
    });
    let second = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(),0.into(),600.into(),800.into()],
        "Resources" => Dictionary::new(),
        "Annots" => if defect == Some("shared") { vec![Object::Reference(link)] } else { vec![] },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![first.into(),second.into()], "Count" => 2,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn decoded(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or_default()),
    )
    .unwrap()
}

fn annotations(pdf: &lopdf::Document) -> Vec<Dictionary> {
    pdf.dereference(
        pdf.get_dictionary(pdf.get_pages()[&1])
            .unwrap()
            .get(b"Annots")
            .unwrap(),
    )
    .unwrap()
    .1
    .as_array()
    .unwrap()
    .iter()
    .map(|item| pdf.dereference(item).unwrap().1.as_dict().unwrap().clone())
    .collect()
}

fn coordinates(pdf: &lopdf::Document, value: &Object) -> Vec<f32> {
    pdf.dereference(value)
        .unwrap()
        .1
        .as_array()
        .unwrap()
        .iter()
        .map(|v| pdf.dereference(v).unwrap().1.as_float().unwrap())
        .collect()
}

fn assert_scaled(
    old_pdf: &lopdf::Document,
    new_pdf: &lopdf::Document,
    old: PageGeometry,
    new: PageGeometry,
) {
    assert_eq!(annotations(old_pdf).len(), annotations(new_pdf).len());
    let expected = |x, y| {
        let (x, y) = old.to_display(x, y);
        new.to_user(x * 0.5, y * 0.5)
    };
    for (a, b) in annotations(old_pdf).iter().zip(annotations(new_pdf)) {
        for key in [b"Rect".as_slice(), b"QuadPoints", b"L", b"CL", b"Vertices"] {
            if let Ok(value) = a.get(key) {
                let old_values = coordinates(old_pdf, value);
                let values = coordinates(new_pdf, b.get(key).unwrap());
                for (from, to) in old_values
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .zip(values.as_chunks::<2>().0)
                {
                    let (x, y) = expected(from[0], from[1]);
                    assert!(
                        (x - to[0]).abs() < 0.02 && (y - to[1]).abs() < 0.02,
                        "{key:?}: {from:?} -> {to:?}"
                    );
                }
            }
        }
        if let Ok(value) = a.get(b"InkList") {
            let old_values = coordinates(old_pdf, &value.as_array().unwrap()[0]);
            let values = coordinates(new_pdf, &b.get(b"InkList").unwrap().as_array().unwrap()[0]);
            for (from, to) in old_values
                .as_chunks::<2>()
                .0
                .iter()
                .zip(values.as_chunks::<2>().0)
            {
                let (x, y) = expected(from[0], from[1]);
                assert!((x - to[0]).abs() < 0.02 && (y - to[1]).abs() < 0.02);
            }
        }
        for key in [b"NM".as_slice(), b"A", b"Subtype", b"DA", b"Contents"] {
            assert_eq!(a.get(key).ok(), b.get(key).ok());
        }
    }
}

#[test]
fn page_scaling_explicitly_preserves_annotation_geometry_targets_encryption_and_undo() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        for encryption in [None, Some(false), Some(true)] {
            let mut bytes = fixture(rotation, None);
            if let Some(aes256) = encryption {
                bytes = security::protect(
                    &bytes,
                    None,
                    &Protection {
                        user_password: "resize-reader".into(),
                        owner_password: "resize-owner".into(),
                        allow_print: true,
                        allow_copy: true,
                        allow_modify: true,
                        allow_annotate: true,
                        aes256,
                    },
                )
                .unwrap();
            }
            let password = encryption.map(|_| "resize-owner");
            let path = temp
                .path()
                .join(format!("source-{rotation}-{encryption:?}.pdf"));
            std::fs::write(&path, &bytes).unwrap();
            let mut doc = engine.open(&path, password).unwrap();
            let old = doc.page_geometry(0).unwrap();
            let before = doc.snapshot().unwrap();
            let old_pdf = decoded(&before, password);
            let (w, h) = old.display_size();
            doc.resize_page(0, w * 0.5, h * 0.5, ResizeMode::ScaleContent)
                .unwrap();
            assert_eq!(
                annotations(&old_pdf),
                annotations(&decoded(&doc.snapshot().unwrap(), password)),
                "existing mode changed annotations"
            );
            doc.restore(before.clone()).unwrap();
            doc.resize_page(0, w * 0.5, h * 0.5, ResizeMode::ScaleContentWithAnnotations)
                .unwrap();
            let new = doc.page_geometry(0).unwrap();
            let after = doc.snapshot().unwrap();
            assert_scaled(&old_pdf, &decoded(&after, password), old, new);
            let output = temp
                .path()
                .join(format!("resized-{rotation}-{encryption:?}.pdf"));
            doc.save_as(&output).unwrap();
            for reopen_password in [password, encryption.map(|_| "resize-reader")] {
                let reopened = engine.open(&output, reopen_password).unwrap();
                assert_scaled(
                    &old_pdf,
                    &decoded(&reopened.snapshot().unwrap(), reopen_password),
                    old,
                    reopened.page_geometry(0).unwrap(),
                );
                assert!(reopened
                    .render_page(0, 0.5)
                    .unwrap()
                    .rgba
                    .iter()
                    .any(|v| *v != 255));
                assert_eq!(
                    decoded(&std::fs::read(&output).unwrap(), reopen_password).is_encrypted(),
                    encryption.is_some()
                );
            }
            doc.restore(before).unwrap();
            assert_eq!(doc.page_geometry(0).unwrap(), old);
            assert_eq!(
                annotations(&decoded(&doc.snapshot().unwrap(), password)),
                annotations(&old_pdf)
            );
            doc.restore(after).unwrap();
            assert_scaled(
                &old_pdf,
                &decoded(&doc.snapshot().unwrap(), password),
                old,
                new,
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
    }
    for defect in ["locked", "malformed", "shared", "unlisted-popup"] {
        let path = temp.path().join(format!("{defect}.pdf"));
        std::fs::write(&path, fixture(90, Some(defect))).unwrap();
        let mut doc = engine.open(&path, None).unwrap();
        let old = doc.page_geometry(0).unwrap();
        let before = decoded(&doc.snapshot().unwrap(), None);
        assert!(doc
            .resize_page(0, 350.0, 250.0, ResizeMode::ScaleContentWithAnnotations)
            .is_err());
        assert_eq!(doc.page_geometry(0).unwrap(), old);
        let after = decoded(&doc.snapshot().unwrap(), None);
        assert_eq!(
            before.objects, after.objects,
            "{defect} mutated original objects"
        );
    }
}
