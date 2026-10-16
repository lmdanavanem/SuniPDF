use lopdf::{dictionary, Object, Stream};
use std::path::Path;
use sunipdf_core::{security, Engine, EngineError, FormKind, Protection};

#[path = "support/flatten_preservation.rs"]
mod fixture;

fn decoded(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or_default()),
    )
    .unwrap()
}

#[test]
fn flatten_preserves_missing_and_invalid_appearance_graphs_and_embedded_files() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("annotations.pdf");
    let original = fixture::document();
    std::fs::write(&source, &original).unwrap();

    let mut malformed = decoded(&original, None);
    let page = malformed.get_pages()[&1];
    let annots = malformed
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_reference()
        .unwrap();
    malformed
        .get_object_mut(annots)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .insert(1, Object::Integer(7));
    let mut malformed_bytes = Vec::new();
    malformed.save_to(&mut malformed_bytes).unwrap();
    let malformed_source = temp.path().join("malformed-annotation.pdf");
    std::fs::write(&malformed_source, &malformed_bytes).unwrap();
    let mut malformed = engine.open(&malformed_source, None).unwrap();
    let mut before = decoded(&malformed.snapshot().unwrap(), None);
    assert!(
        malformed.flatten(None).is_err(),
        "malformed annotation must not truncate iteration silently"
    );
    let mut after = decoded(&malformed.snapshot().unwrap(), None);
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert!(
        after.trailer == before.trailer,
        "malformed annotation changed trailer"
    );
    assert!(
        after.objects == before.objects,
        "malformed annotation leaves memory unchanged"
    );
    assert!(
        std::fs::read(&malformed_source).unwrap() == malformed_bytes,
        "malformed source changed"
    );

    let mut doc = engine.open(&source, None).unwrap();
    assert_eq!(
        doc.flatten(None).unwrap(),
        17,
        "preserved annotations are excluded from the count"
    );
    assert_eq!(doc.form_kind(), FormKind::None);
    fixture::assert_preserved(&decoded(&doc.snapshot().unwrap(), None));
    assert_eq!(doc.flatten(None).unwrap(), 0);
    fixture::assert_preserved(&decoded(&doc.snapshot().unwrap(), None));
    let output = temp.path().join("flattened.pdf");
    doc.save_as(&output).unwrap();
    let reopened = engine.open(&output, None).unwrap();
    fixture::assert_preserved(&decoded(&reopened.snapshot().unwrap(), None));
    assert!(
        std::fs::read(&source).unwrap() == original,
        "source bytes changed"
    );

    let mut partial = engine.open(&source, None).unwrap();
    let before = decoded(&partial.snapshot().unwrap(), None);
    let second = fixture::annotations(&before, 1);
    assert_eq!(partial.flatten(Some(&[0, 0])).unwrap(), 17);
    assert_eq!(partial.form_kind(), FormKind::AcroForm);
    let after = decoded(&partial.snapshot().unwrap(), None);
    fixture::assert_preserved(&after);
    assert_eq!(
        fixture::annotations(&after, 1),
        second,
        "unselected page remains unchanged"
    );
    assert_eq!(partial.flatten(Some(&[1])).unwrap(), 0);
    assert_eq!(partial.flatten(None).unwrap(), 0);
    fixture::assert_preserved(&decoded(&partial.snapshot().unwrap(), None));
    let mut before_invalid = decoded(&partial.snapshot().unwrap(), None);
    assert!(matches!(
        partial.flatten(Some(&[2])),
        Err(EngineError::PageOutOfRange(2))
    ));
    let mut after_invalid = decoded(&partial.snapshot().unwrap(), None);
    before_invalid.trailer.remove(b"ID");
    after_invalid.trailer.remove(b"ID");
    assert!(
        after_invalid.trailer == before_invalid.trailer,
        "invalid page changed trailer"
    );
    assert!(
        after_invalid.objects == before_invalid.objects,
        "invalid page changed document objects"
    );
    assert!(
        std::fs::read(&source).unwrap() == original,
        "invalid page changed source bytes"
    );

    let protection = Protection {
        user_password: "user".into(),
        owner_password: "owner".into(),
        aes256: true,
        ..Default::default()
    };
    let password = protection.user_password.as_str();
    let protected = security::protect(&original, None, &protection).unwrap();
    let mut encrypted = engine
        .open_bytes(protected, Some(password), Path::new("encrypted.pdf"))
        .unwrap();
    assert_eq!(encrypted.flatten(None).unwrap(), 17);
    assert!(encrypted.properties().encrypted);
    let output = temp.path().join("encrypted-flattened.pdf");
    encrypted.save_as(&output).unwrap();
    assert!(matches!(
        engine.open(&output, None),
        Err(EngineError::PasswordRequired)
    ));
    let reopened = engine.open(&output, Some(password)).unwrap();
    fixture::assert_preserved(&decoded(&reopened.snapshot().unwrap(), Some(password)));

    for (state, normal_states) in [
        (None, false),
        (Some(Object::Name(b"On".to_vec())), true),
        (Some(Object::string_literal("On")), true),
        (Some(Object::Name(Vec::new())), true),
        (None, true),
    ] {
        let mut pdf = decoded(&original, None);
        let page = pdf.get_pages()[&1];
        let appearance = pdf.add_object(Stream::new(dictionary! {
            "Type"=>"XObject", "Subtype"=>"Form", "BBox"=>vec![0.into(),0.into(),40.into(),20.into()],
            "Resources"=>dictionary! {},
        }, b"0 0 1 rg 0 0 40 20 re f".to_vec()));
        let mut annotation = dictionary! {
            "Type"=>"Annot", "Subtype"=>"Screen", "P"=>page, "F"=>0,
            "Rect"=>vec![40.into(),40.into(),80.into(),60.into()],
        };
        let ap = if normal_states {
            dictionary! {"N"=>dictionary! {"On"=>appearance}}
        } else {
            dictionary! {"N"=>appearance}
        };
        if let Some(state) = &state {
            annotation.set("AS", state.clone());
        }
        annotation.set("AP", ap);
        let annotation = pdf.add_object(annotation);
        let id = pdf
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_reference()
            .unwrap();
        pdf.get_object_mut(id)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(annotation.into());
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut drawable = engine
            .open_bytes(bytes, None, Path::new("drawable.pdf"))
            .unwrap();
        assert_eq!(
            drawable.flatten(None).unwrap(),
            18,
            "drawable AP state {state:?}"
        );
        fixture::assert_preserved(&decoded(&drawable.snapshot().unwrap(), None));
        let image = drawable.render_page(0, 1.0).unwrap();
        assert!(
            image
                .rgba
                .chunks(4)
                .filter(|p| p[2] > 180 && p[0] < 40 && p[1] < 40)
                .count()
                > 500
        );
        assert_eq!(drawable.flatten(None).unwrap(), 0, "drawn only once");
    }

    let mut deep = decoded(&original, None);
    let (_, attachment) = fixture::annotations(&deep, 0)
        .into_iter()
        .find(|(_, annotation)| {
            annotation.get(b"Subtype").unwrap().as_name().unwrap() == b"FileAttachment"
        })
        .unwrap();
    let file = attachment.get(b"FS").unwrap().as_reference().unwrap();
    let mut next = Object::string_literal("end");
    for _ in 0..80 {
        next = deep.add_object(dictionary! {"Next"=>next}).into();
    }
    deep.get_dictionary_mut(file).unwrap().set("Deep", next);
    let mut bytes = Vec::new();
    deep.save_to(&mut bytes).unwrap();
    let source = temp.path().join("deep-attachment.pdf");
    std::fs::write(&source, &bytes).unwrap();
    let mut deep = engine.open(&source, None).unwrap();
    let mut before = decoded(&deep.snapshot().unwrap(), None);
    assert!(
        deep.flatten(None).is_err(),
        "excessive preservation graph must fail rather than truncate"
    );
    let mut after = decoded(&deep.snapshot().unwrap(), None);
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert!(
        after.trailer == before.trailer,
        "deep graph changed trailer"
    );
    assert!(
        after.objects == before.objects,
        "failed flatten leaves memory unchanged"
    );
    assert!(
        std::fs::read(source).unwrap() == bytes,
        "deep graph source changed"
    );
}
