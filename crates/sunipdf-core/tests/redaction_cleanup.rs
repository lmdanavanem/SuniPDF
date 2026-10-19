use lopdf::{dictionary, Dictionary, Object, Stream};
use sunipdf_core::{security, Engine, PointRect, Protection, RedactMark, RedactOptions};

fn fixture() -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content = pdf.add_object(Stream::new(Dictionary::new(),
        b"/Span << /ActualText (ACTUAL-PRIVATE-VALUE) /Alt (ALT-PRIVATE-VALUE) >> BDC BT /F1 18 Tf 40 160 Td (REMOVE) Tj ET 0.2 0.5 0.8 rg 200 20 50 50 re f EMC BT /F1 12 Tf 190 110 Td (PUBLIC KEEP) Tj ET".to_vec()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),300.into(),240.into()],"Contents"=>content,
        "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}});
    pdf.objects.insert(
        pages,
        Object::Dictionary(
            dictionary! {"Type"=>"Pages","Count"=>1,"Kids"=>vec![Object::Reference(page)]},
        ),
    );
    let template = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"<template><script>XFA-SCRIPT-PRIVATE</script></template>".to_vec(),
    ));
    let datasets = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"<datasets>XFA-DATA-PRIVATE</datasets>".to_vec(),
    ));
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages,"AcroForm"=>dictionary! {"Fields"=>Vec::<Object>::new(),
        "XFA"=>vec![Object::string_literal("template"),Object::Reference(template),Object::string_literal("datasets"),Object::Reference(datasets)]}});
    pdf.trailer.set("Root", catalog);
    pdf.trailer.set(
        "ID",
        vec![
            Object::string_literal(b"SuniPDFCleanupQA".to_vec()),
            Object::string_literal(b"SuniPDFCleanupQA".to_vec()),
        ],
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn has_private_dictionary(object: &Object) -> bool {
    match object {
        Object::Array(values) => values.iter().any(has_private_dictionary),
        Object::Dictionary(dictionary) => {
            dictionary.has(b"XFA")
                || dictionary.has(b"ActualText")
                || dictionary.has(b"Alt")
                || dictionary
                    .iter()
                    .any(|(_, value)| has_private_dictionary(value))
        }
        Object::Stream(stream) => has_private_dictionary(&Object::Dictionary(stream.dict.clone())),
        _ => false,
    }
}

#[test]
fn redaction_cleans_shared_replacement_text_and_xfa_without_changing_unmarked_content() {
    let engine = Engine::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    for encryption in [None, Some(false), Some(true)] {
        let mut input = fixture();
        if let Some(aes256) = encryption {
            input = security::protect(
                &input,
                None,
                &Protection {
                    user_password: "cleanup-reader".into(),
                    owner_password: "cleanup-owner".into(),
                    allow_copy: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let password = encryption.map(|_| "cleanup-reader");
        let source = directory.path().join(format!("source-{encryption:?}.pdf"));
        std::fs::write(&source, &input).unwrap();
        let mut document = engine.open(&source, password).unwrap();
        let properties = document.properties();
        let before = document.render_page(0, 1.0).unwrap();
        let marks = [RedactMark {
            page: 0,
            rect: PointRect {
                left: 30.0,
                bottom: 150.0,
                right: 150.0,
                top: 190.0,
            },
        }];
        let report = document.redact(&marks, &RedactOptions::default()).unwrap();
        assert!(report.hidden_data_cleaned && report.bitmap_objects > 0);
        assert_eq!(report.fields_removed, 0);
        assert_eq!(document.properties().encrypted, properties.encrypted);
        assert_eq!(document.properties().can_copy, properties.can_copy);
        let output = directory
            .path()
            .join(format!("redacted-{encryption:?}.pdf"));
        document.save_as(&output).unwrap();
        let reopened = engine.open(&output, password).unwrap();
        let after = reopened.render_page(0, 1.0).unwrap();
        assert_eq!((before.width, before.height), (after.width, after.height));
        for y in 0..before.height as usize {
            let start = (y * before.width as usize + 170) * 4;
            let end = (y + 1) * before.width as usize * 4;
            assert_eq!(
                &before.rgba[start..end],
                &after.rgba[start..end],
                "unmarked content changed"
            );
        }
        assert!(!reopened.page_text(0).unwrap().contains("REMOVE"));
        assert!(reopened.page_text(0).unwrap().contains("PUBLIC KEEP"));
        let bytes = std::fs::read(&output).unwrap();
        let parsed = lopdf::Document::load_mem_with_options(
            &bytes,
            lopdf::LoadOptions::with_password(password.unwrap_or_default()),
        )
        .unwrap();
        assert!(!parsed.objects.values().any(has_private_dictionary));
        for object in parsed.objects.values() {
            if let Object::Stream(stream) = object {
                let content = stream
                    .get_plain_content_with_limit(8 * 1024 * 1024)
                    .unwrap_or_else(|_| stream.content.clone());
                for marker in [
                    b"ACTUAL-PRIVATE-VALUE".as_slice(),
                    b"ALT-PRIVATE-VALUE",
                    b"XFA-SCRIPT-PRIVATE",
                    b"XFA-DATA-PRIVATE",
                ] {
                    assert!(!content.windows(marker.len()).any(|value| value == marker));
                }
            }
        }
        if encryption.is_some() {
            assert!(engine.open(&output, None).is_err());
            assert!(security::unprotect(&bytes, "cleanup-reader").is_err());
            assert!(security::unprotect(&bytes, "cleanup-owner").is_ok());
        }
        assert_eq!(std::fs::read(&source).unwrap(), input);
    }
}
