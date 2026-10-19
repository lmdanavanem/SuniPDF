use lopdf::{dictionary, Dictionary, Object, Stream};
use sunipdf_core::{sanitize::sanitize_document, security, Engine, Protection};

fn fixture() -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let root = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let unused_program = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"UNUSED-FONT-SECRET".to_vec(),
    ));
    let unused = pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"TrueType","BaseFont"=>"ABCDEF+Unused",
        "FontDescriptor"=>dictionary! {"Type"=>"FontDescriptor","FontName"=>"ABCDEF+Unused","FontFile2"=>unused_program}});
    let content = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"BT /FUsed 12 Tf 24 100 Td (Visible content preserved) Tj ET".to_vec(),
    ));
    let attachment_bytes = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"EmbeddedFile"},
        b"ATTACHMENT-SECRET".to_vec(),
    ));
    let attachment = pdf.add_object(dictionary! {"Type"=>"Filespec","F"=>Object::string_literal("private.txt"),"EF"=>dictionary! {"F"=>attachment_bytes}});
    let annot = pdf.add_object(dictionary! {"Type"=>"Annot","Subtype"=>"FileAttachment","Rect"=>vec![600.into(),780.into(),610.into(),790.into()],"FS"=>attachment});
    let screen = pdf.add_object(dictionary! {"Type"=>"Annot","Subtype"=>"Screen","Rect"=>vec![600.into(),780.into(),610.into(),790.into()],
        "A"=>dictionary! {"S"=>"Rendition","R"=>dictionary! {"S"=>"MR","C"=>dictionary! {"S"=>"MCD","D"=>dictionary! {"FS"=>"URL","F"=>Object::string_literal("https://example.invalid/SCREEN-MEDIA-REMOVED")}}}}});
    let template = pdf.add_object(Stream::new(Dictionary::new(),
        b"<template><script contentType=\"application/x-javascript\">XFA-SCRIPT-REMOVED</script></template>".to_vec()));
    let datasets = pdf.add_object(Stream::new(
        Dictionary::new(),
        b"<datasets><value>XFA-DATA-REMOVED</value></datasets>".to_vec(),
    ));
    let form = pdf.add_object(dictionary! {"Fields"=>Vec::<Object>::new(),"XFA"=>vec![Object::string_literal("template"),Object::Reference(template),Object::string_literal("datasets"),Object::Reference(datasets)]});
    let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),500.into(),700.into()],
        "Resources"=>dictionary! {"Font"=>dictionary! {"FUsed"=>font,"Unused"=>unused}},"Contents"=>content,"Annots"=>vec![Object::Reference(annot),Object::Reference(screen)]});
    pdf.objects.insert(
        root,
        Object::Dictionary(
            dictionary! {"Type"=>"Pages","Kids"=>vec![Object::Reference(page)],"Count"=>1},
        ),
    );
    let script = pdf.add_object(
        dictionary! {"S"=>"JavaScript","JS"=>Object::string_literal("JAVASCRIPT-SECRET")},
    );
    let outlines = pdf.new_object_id();
    let bookmark = pdf.add_object(dictionary! {"Title"=>Object::string_literal("BOOKMARK-SECRET"),"Parent"=>outlines,"Dest"=>vec![Object::Reference(page),Object::Name(b"Fit".to_vec())]});
    pdf.objects.insert(
        outlines,
        Object::Dictionary(
            dictionary! {"Type"=>"Outlines","First"=>bookmark,"Last"=>bookmark,"Count"=>1},
        ),
    );
    let metadata = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"Metadata","Subtype"=>"XML"},
        b"METADATA-SECRET".to_vec(),
    ));
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root,"Outlines"=>outlines,"Metadata"=>metadata,"AcroForm"=>form,
        "OpenAction"=>script,"Names"=>dictionary! {
            "EmbeddedFiles"=>dictionary! {"Names"=>vec![Object::string_literal("ATTACHMENT-NAME-SECRET"),Object::Reference(attachment)]},
            "JavaScript"=>dictionary! {"Names"=>vec![Object::string_literal("script"),Object::Reference(script)]},
            "Dests"=>dictionary! {"Names"=>vec![Object::string_literal("DESTINATION-SECRET"),Object::Array(vec![Object::Reference(page),Object::Name(b"Fit".to_vec())])]}
        }});
    let info = pdf.add_object(dictionary! {"Title"=>Object::string_literal("INFO-SECRET")});
    pdf.trailer.set("Root", catalog);
    pdf.trailer.set("Info", info);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn contains(object: &Object, marker: &[u8]) -> bool {
    match object {
        Object::String(bytes, _) => bytes.windows(marker.len()).any(|value| value == marker),
        Object::Array(values) => values.iter().any(|value| contains(value, marker)),
        Object::Dictionary(dict) => dict.iter().any(|(_, value)| contains(value, marker)),
        Object::Stream(stream) => {
            stream
                .content
                .windows(marker.len())
                .any(|value| value == marker)
                || stream.dict.iter().any(|(_, value)| contains(value, marker))
        }
        _ => false,
    }
}

#[test]
fn sanitization_removes_inline_and_indirect_xfa_packets() {
    for inline_form in [false, true] {
        for packet_array in [false, true] {
            let mut pdf = lopdf::Document::load_mem(&fixture()).unwrap();
            let form = pdf.catalog().unwrap().get(b"AcroForm").unwrap().clone();
            let mut form = pdf.dereference(&form).unwrap().1.as_dict().unwrap().clone();
            if !packet_array {
                let packet = pdf.add_object(Stream::new(Dictionary::new(),
                    b"<xfa><script>XFA-SCRIPT-REMOVED</script><value>XFA-DATA-REMOVED</value></xfa>".to_vec()));
                form.set("XFA", packet);
            }
            let form = if inline_form {
                Object::Dictionary(form)
            } else {
                Object::Reference(pdf.add_object(form))
            };
            pdf.catalog_mut().unwrap().set("AcroForm", form);
            let mut input = Vec::new();
            pdf.save_to(&mut input).unwrap();
            let (output, report) = sanitize_document(&input, None).unwrap();
            assert!(report.verified);
            let cleaned = lopdf::Document::load_mem(&output).unwrap();
            let form = cleaned.catalog().unwrap().get(b"AcroForm").unwrap();
            assert!(!cleaned
                .dereference(form)
                .unwrap()
                .1
                .as_dict()
                .unwrap()
                .has(b"XFA"));
            for marker in [
                b"XFA-SCRIPT-REMOVED".as_slice(),
                b"XFA-DATA-REMOVED",
                b"SCREEN-MEDIA-REMOVED",
            ] {
                assert!(!cleaned
                    .objects
                    .values()
                    .any(|object| contains(object, marker)));
            }
        }
    }
}

#[test]
fn sanitization_rewrites_verified_new_copies_preserving_visible_content_and_protection() {
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for encryption in [None, Some(false), Some(true)] {
        let mut input = fixture();
        if let Some(aes256) = encryption {
            input = security::protect(
                &input,
                None,
                &Protection {
                    user_password: "sanitize-reader".into(),
                    owner_password: "sanitize-owner".into(),
                    allow_print: true,
                    allow_copy: true,
                    allow_modify: true,
                    allow_annotate: true,
                    aes256,
                },
            )
            .unwrap();
        }
        let password = encryption.map(|_| "sanitize-owner");
        let path = temp.path().join(format!("original-{encryption:?}.pdf"));
        std::fs::write(&path, &input).unwrap();
        let original = engine.open(&path, password).unwrap();
        let rendered = original.render_page(0, 1.0).unwrap();
        let text = original.page_text(0).unwrap();
        let (output, report) = sanitize_document(&input, password).unwrap();
        assert!(report.verified && report.removed_entries >= 6 && report.removed_annotations == 2);
        assert!(report.unused_font_entries >= 1 && report.unreachable_objects >= 5);
        let copy = temp.path().join(format!("sanitized-{encryption:?}.pdf"));
        sunipdf_core::write_new_copy(&copy, &output).unwrap();
        assert!(sunipdf_core::write_new_copy(&path, &output).is_err());
        for password in [password, encryption.map(|_| "sanitize-reader")] {
            let reopened = engine.open(&copy, password).unwrap();
            assert_eq!(reopened.page_text(0).unwrap(), text);
            assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, rendered.rgba);
            assert_eq!(reopened.is_encrypted(), encryption.is_some());
            let decoded = lopdf::Document::load_mem_with_options(
                &output,
                lopdf::LoadOptions::with_password(password.unwrap_or_default()),
            )
            .unwrap();
            assert!(!decoded.trailer.has(b"Prev") && !decoded.trailer.has(b"Info"));
            for marker in [
                b"UNUSED-FONT-SECRET".as_slice(),
                b"ATTACHMENT-SECRET",
                b"JAVASCRIPT-SECRET",
                b"BOOKMARK-SECRET",
                b"METADATA-SECRET",
                b"DESTINATION-SECRET",
                b"INFO-SECRET",
                b"XFA-SCRIPT-REMOVED",
                b"XFA-DATA-REMOVED",
                b"SCREEN-MEDIA-REMOVED",
            ] {
                assert!(
                    !decoded
                        .objects
                        .values()
                        .any(|object| contains(object, marker)),
                    "hidden structure survived rewrite"
                );
            }
        }
        assert_eq!(std::fs::read(&path).unwrap(), input);
        assert_eq!(original.page_text(0).unwrap(), text);
        let (twice, report) = sanitize_document(&output, password).unwrap();
        assert!(report.verified);
        assert_eq!(report.removed_annotations, 0);
        assert!(!twice.is_empty());
    }
    let mut malformed = lopdf::Document::load_mem(&fixture()).unwrap();
    let page = malformed.get_pages()[&1];
    malformed
        .get_dictionary_mut(page)
        .unwrap()
        .set("Contents", Object::Reference((u32::MAX, 0)));
    let mut bytes = Vec::new();
    malformed.save_to(&mut bytes).unwrap();
    assert!(sanitize_document(&bytes, None).is_err());
}
