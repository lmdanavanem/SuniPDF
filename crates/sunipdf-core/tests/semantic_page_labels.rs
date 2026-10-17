use lopdf::{dictionary, Dictionary, Object, Stream};
use sunipdf_core::{security, Engine, Protection};

fn fixture(malformed: bool) -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let root = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let mut pages = Vec::new();
    for page in 0..6 {
        let content = pdf.add_object(Stream::new(
            Dictionary::new(),
            format!("BT /F1 12 Tf 40 80 Td (PAGE-{page}) Tj ET").into_bytes(),
        ));
        pages.push(
            pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>root, "Contents"=>content}),
        );
    }
    pdf.objects.insert(root, dictionary! {
        "Type"=>"Pages", "Kids"=>pages.iter().copied().map(Object::Reference).collect::<Vec<_>>(), "Count"=>6,
        "MediaBox"=>vec![0.into(),0.into(),300.into(),200.into()], "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},
    }.into());
    let custom =
        pdf.add_object(dictionary! {"Value"=>Object::string_literal("preserved extension")});
    let first = pdf.add_object(dictionary! {"Nums"=>vec![0.into(),dictionary!{"S"=>"r","P"=>lopdf::text_string("Pré-"),"St"=>3,"SuniCustom"=>custom}.into()], "Limits"=>vec![0.into(),0.into()]});
    let second = pdf.add_object(dictionary! {"Nums"=>vec![2.into(),dictionary!{"S"=>"D","P"=>lopdf::text_string("§-"),"St"=>8}.into(),4.into(),dictionary!{"P"=>lopdf::text_string("Annexe")}.into()], "Limits"=>vec![2.into(),4.into()]});
    let labels = pdf.add_object(
        dictionary! {"Kids"=>vec![first.into(),second.into()], "Limits"=>vec![0.into(),4.into()]},
    );
    if malformed {
        pdf.get_dictionary_mut(first)
            .unwrap()
            .set("Nums", vec![Object::Integer(0)]);
    }
    let catalog =
        pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>root,"PageLabels"=>labels});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn labels(doc: &sunipdf_core::Document<'_>) -> Vec<String> {
    (0..doc.page_count())
        .map(|page| doc.info().page_label(page).unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn authored_labels_follow_content_across_page_operations_and_undo_with_implicit_physical_defaults()
{
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let expected = ["Pré-iii", "Pré-iv", "§-8", "§-9", "Annexe", "Annexe"];
    for encryption in [None, Some(false), Some(true)] {
        let mut bytes = fixture(false);
        if let Some(aes256) = encryption {
            bytes = security::protect(
                &bytes,
                None,
                &Protection {
                    user_password: "labels-reader".into(),
                    owner_password: "labels-owner".into(),
                    allow_print: true,
                    allow_copy: true,
                    allow_modify: true,
                    allow_annotate: true,
                    aes256,
                },
            )
            .unwrap();
        }
        let source = temp.path().join(format!("source-{encryption:?}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        let password = encryption.map(|_| "labels-owner");
        let mut doc = engine.open(&source, password).unwrap();
        assert_eq!(labels(&doc), expected);
        assert_eq!(doc.info().resolve_page_input("Annexe"), None);
        assert_eq!(doc.info().resolve_page_input("#5"), Some(4));
        let before = doc.snapshot().unwrap();
        let pixels = doc.render_page(1, 1.0).unwrap().rgba;
        doc.move_page(1, 4).unwrap();
        assert_eq!(
            labels(&doc),
            ["Pré-iii", "§-8", "§-9", "Annexe", "Pré-iv", "Annexe"]
        );
        assert_eq!(doc.render_page(4, 1.0).unwrap().rgba, pixels);
        let moved = doc.snapshot().unwrap();
        doc.restore(before.clone()).unwrap();
        assert_eq!(labels(&doc), expected);
        doc.restore(moved).unwrap();
        doc.delete_pages(&[0, 2]).unwrap();
        assert_eq!(labels(&doc), ["§-8", "Annexe", "Pré-iv", "Annexe"]);
        doc.insert_blank_pages(1, 1, 300.0, 200.0).unwrap();
        assert_eq!(labels(&doc), ["§-8", "2", "Annexe", "Pré-iv", "Annexe"]);
        let output = temp.path().join(format!("edited-{encryption:?}.pdf"));
        doc.save_as(&output).unwrap();
        let raw = lopdf::Document::load_mem_with_options(
            &std::fs::read(&output).unwrap(),
            lopdf::LoadOptions::with_password(password.unwrap_or_default()),
        )
        .unwrap();
        let tree = raw
            .dereference(raw.catalog().unwrap().get(b"PageLabels").unwrap())
            .unwrap()
            .1
            .as_dict()
            .unwrap();
        let nums = tree.get(b"Nums").unwrap().as_array().unwrap();
        let custom_label = nums
            .as_chunks::<2>()
            .0
            .iter()
            .find(|pair| pair[0].as_i64().unwrap() == 3)
            .unwrap()[1]
            .as_dict()
            .unwrap();
        let extension = raw
            .dereference(custom_label.get(b"SuniCustom").unwrap())
            .unwrap()
            .1
            .as_dict()
            .unwrap();
        assert_eq!(
            extension.get(b"Value").unwrap().as_str().unwrap(),
            b"preserved extension"
        );
        for reopen_password in [password, encryption.map(|_| "labels-reader")] {
            let reopened = engine.open(&output, reopen_password).unwrap();
            assert_eq!(labels(&reopened), labels(&doc));
            assert_eq!(reopened.is_encrypted(), encryption.is_some());
        }
        let extract = temp.path().join(format!("extract-{encryption:?}.pdf"));
        doc.extract_to(&[3, 0, 3], &extract).unwrap();
        let extracted = engine.open(&extract, password).unwrap();
        assert_eq!(labels(&extracted), ["Pré-iv", "§-8", "Pré-iv"]);
        assert_eq!(extracted.info().resolve_page_input("Pré-iv"), None);
        assert_eq!(extracted.info().resolve_page_input("§-8"), Some(1));
        assert!(extracted.page_text(0).unwrap().contains("PAGE-1"));
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }
    let authored = temp.path().join("authored.pdf");
    let implicit = temp.path().join("implicit.pdf");
    std::fs::write(&authored, fixture(false)).unwrap();
    std::fs::write(&implicit, include_bytes!("fixtures/two-pages.pdf")).unwrap();
    let merged = temp.path().join("merged.pdf");
    let original = std::fs::read(&authored).unwrap();
    assert!(engine.merge(&[authored.clone()], &authored).is_err());
    assert_eq!(std::fs::read(&authored).unwrap(), original);
    engine
        .merge(&[authored.clone(), implicit.clone()], &merged)
        .unwrap();
    assert_eq!(
        labels(&engine.open(&merged, None).unwrap()),
        ["Pré-iii", "Pré-iv", "§-8", "§-9", "Annexe", "Annexe", "7", "8"]
    );
    let mut inserted = engine.open(&authored, None).unwrap();
    inserted
        .insert_pages_from(&implicit, None, &[1], 1)
        .unwrap();
    assert_eq!(
        labels(&inserted),
        ["Pré-iii", "2", "Pré-iv", "§-8", "§-9", "Annexe", "Annexe"]
    );
    let mut default = engine.open(&implicit, None).unwrap();
    default.move_page(0, 1).unwrap();
    default.insert_blank_pages(1, 1, 300.0, 200.0).unwrap();
    assert!(default.info().page_labels.iter().all(Option::is_none));
    let malformed = temp.path().join("malformed.pdf");
    std::fs::write(&malformed, fixture(true)).unwrap();
    let mut invalid = engine.open(&malformed, None).unwrap();
    let original_text = invalid.page_text(0).unwrap();
    assert!(invalid.delete_page(0).is_err());
    assert!(invalid.insert_blank_pages(0, 1, 300.0, 200.0).is_err());
    assert!(invalid.move_page(0, 1).is_err());
    assert_eq!(invalid.page_count(), 6);
    assert_eq!(invalid.page_text(0).unwrap(), original_text);
}
