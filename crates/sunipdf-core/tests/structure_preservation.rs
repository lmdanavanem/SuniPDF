use lopdf::{dictionary, Dictionary, Object, ObjectId, Stream};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    bookmarks::{self, BookmarkEdit},
    security, Engine, Protection,
};
#[path = "support/feature_evidence.rs"]
mod feature_evidence;

static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, PartialEq)]
struct LogicalSnapshot {
    encrypted: bool,
    encryption: Option<Dictionary>,
    objects: BTreeMap<ObjectId, Object>,
    trailer: Dictionary,
}

fn normalize_xref_storage(pdf: &mut lopdf::Document) {
    fn references(value: &Object, found: &mut HashSet<ObjectId>) {
        match value {
            Object::Reference(id) => {
                found.insert(*id);
            }
            Object::Array(values) => {
                for value in values {
                    references(value, found);
                }
            }
            Object::Dictionary(dictionary) => {
                for (_, value) in dictionary {
                    references(value, found);
                }
            }
            Object::Stream(stream) => {
                for (_, value) in &stream.dict {
                    references(value, found);
                }
            }
            _ => {}
        }
    }
    let mut referenced = HashSet::new();
    references(&Object::Dictionary(pdf.trailer.clone()), &mut referenced);
    for object in pdf.objects.values() {
        references(object, &mut referenced);
    }
    let mut storage = Vec::new();
    for (id, object) in &pdf.objects {
        let Ok(stream) = object.as_stream() else {
            continue;
        };
        if referenced.contains(id)
            || !stream.dict.has_type(b"XRef")
            || stream.dict.iter().any(|(key, _)| {
                !matches!(
                    key.as_slice(),
                    b"Type"
                        | b"Filter"
                        | b"Length"
                        | b"Index"
                        | b"Root"
                        | b"Size"
                        | b"W"
                        | b"ID"
                        | b"Info"
                        | b"Encrypt"
                        | b"DecodeParms"
                )
            })
        {
            continue;
        }
        for key in [b"Root".as_slice(), b"Size", b"Info", b"Encrypt"] {
            assert_eq!(stream.dict.get(key).ok(), pdf.trailer.get(key).ok());
        }
        let stream_ids = stream.dict.get(b"ID").unwrap().as_array().unwrap();
        let trailer_ids = pdf.trailer.get(b"ID").unwrap().as_array().unwrap();
        assert_eq!(stream_ids.len(), 2);
        assert_eq!(
            stream_ids[0].as_str().unwrap(),
            trailer_ids[0].as_str().unwrap()
        );
        let widths = stream.dict.get(b"W").unwrap().as_array().unwrap();
        assert_eq!(widths.len(), 3);
        let row_width = widths
            .iter()
            .map(|value| {
                let width = usize::try_from(value.as_i64().unwrap()).unwrap();
                assert!(width <= 8);
                width
            })
            .sum::<usize>();
        assert!(row_width > 0);
        let index = stream.dict.get(b"Index").unwrap().as_array().unwrap();
        assert!(!index.is_empty() && index.len() % 2 == 0);
        let size = usize::try_from(stream.dict.get(b"Size").unwrap().as_i64().unwrap()).unwrap();
        let mut end = 0;
        let mut rows = 0usize;
        for pair in index.as_chunks::<2>().0 {
            let start = usize::try_from(pair[0].as_i64().unwrap()).unwrap();
            let count = usize::try_from(pair[1].as_i64().unwrap()).unwrap();
            assert!(start >= end && count > 0);
            end = start.checked_add(count).unwrap();
            assert!(end <= size);
            rows = rows.checked_add(count).unwrap();
        }
        assert_eq!(
            stream
                .get_plain_content_with_limit(1024 * 1024)
                .unwrap()
                .len(),
            rows.checked_mul(row_width).unwrap()
        );
        storage.push(*id);
    }
    if !storage.is_empty() {
        let original_highest = pdf.objects.keys().map(|id| id.0).max().unwrap();
        assert_eq!(
            pdf.trailer.get(b"Size").unwrap().as_i64().unwrap(),
            i64::from(original_highest) + 1
        );
        for id in storage {
            pdf.objects.remove(&id);
        }
        let highest = pdf.objects.keys().map(|id| id.0).max().unwrap();
        pdf.trailer.set("Size", i64::from(highest) + 1);
    }
}

fn logical_snapshot(document: &sunipdf_core::Document<'_>) -> LogicalSnapshot {
    let bytes = document.snapshot().unwrap();
    let raw = lopdf::Document::load_mem(&bytes).unwrap();
    let encrypted = raw.is_encrypted();
    let encryption = raw.get_encrypted().ok().cloned();
    assert_eq!(encrypted, document.properties().encrypted);
    let mut pdf = lopdf::Document::load_mem_with_options(
        &bytes,
        lopdf::LoadOptions::with_password(document.password().unwrap_or("")),
    )
    .unwrap();
    normalize_xref_storage(&mut pdf);
    let mut ids = pdf.trailer.get(b"ID").unwrap().as_array().unwrap().clone();
    assert_eq!(ids.len(), 2);
    let Object::String(_, format) = &ids[1] else {
        panic!("second trailer ID is not a string");
    };
    ids[1] = Object::String(Vec::new(), *format);
    pdf.trailer.set("ID", ids);
    LogicalSnapshot {
        encrypted,
        encryption,
        objects: pdf.objects,
        trailer: pdf.trailer,
    }
}

#[test]
fn xref_storage_normalization_retains_referenced_streams_content_and_first_id() {
    let mut original = lopdf::Document::with_version("1.7");
    let root = original.add_object(dictionary! {"Type"=>"Catalog"});
    let content = original.add_object(Stream::new(Dictionary::new(), b"kept".to_vec()));
    let ids = vec![
        Object::string_literal(b"SuniPDFStructure".to_vec()),
        Object::string_literal(b"SuniPDFStructure".to_vec()),
    ];
    original.trailer.set("Root", root);
    original.trailer.set("ID", ids.clone());
    original.trailer.set("Size", 4);
    let mut rows = vec![0; 21];
    for row in rows.as_chunks_mut::<7>().0 {
        row[0] = 1;
    }
    let storage = original.add_object(Stream::new(
        dictionary! {"Type"=>"XRef","Root"=>root,"Size"=>4,"ID"=>ids,"W"=>vec![1.into(),4.into(),2.into()],"Index"=>vec![1.into(),3.into()]},
        rows,
    ));
    let mut baseline = original.clone();
    normalize_xref_storage(&mut baseline);
    assert!(!baseline.objects.contains_key(&storage));
    assert_eq!(baseline.trailer.get(b"Size").unwrap().as_i64().unwrap(), 3);

    let mut changed = original.clone();
    changed
        .get_object_mut(content)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .content = b"changed".to_vec();
    normalize_xref_storage(&mut changed);
    assert_ne!(changed.objects, baseline.objects);

    let mut changed = original.clone();
    let changed_ids = vec![
        Object::string_literal(b"ChangedStructure".to_vec()),
        Object::string_literal(b"SuniPDFStructure".to_vec()),
    ];
    changed.trailer.set("ID", changed_ids.clone());
    changed
        .get_object_mut(storage)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .dict
        .set("ID", changed_ids);
    normalize_xref_storage(&mut changed);
    assert_ne!(changed.trailer, baseline.trailer);

    let mut referenced = original;
    referenced
        .get_dictionary_mut(root)
        .unwrap()
        .set("ReferencedStream", storage);
    normalize_xref_storage(&mut referenced);
    assert!(referenced.objects.contains_key(&storage));
    assert_eq!(
        referenced.trailer.get(b"Size").unwrap().as_i64().unwrap(),
        4
    );
}

fn fixture() -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let root = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let group = pdf.add_object(dictionary! {"Type"=>"OCG","Name"=>Object::string_literal("Layer")});
    let mut pages = Vec::new();
    for page in 0..2 {
        let stream = pdf.add_object(Stream::new(
            Dictionary::new(),
            format!("BT /F1 12 Tf 40 150 Td (STRUCTURE) Tj ET /OC /L1 BDC BT /F1 12 Tf 40 100 Td (PAGE{page}) Tj ET EMC").into_bytes(),
        ));
        pages.push(pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),300.into(),200.into()],"Contents"=>stream,"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font},"Properties"=>dictionary!{"L1"=>group}}}));
    }
    pdf.objects.insert(root,dictionary!{"Type"=>"Pages","Count"=>2,"Kids"=>pages.iter().copied().map(Object::Reference).collect::<Vec<_>>()}.into());
    let link0=pdf.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Link","Rect"=>vec![20.into(),20.into(),80.into(),40.into()],"P"=>pages[0],"A"=>dictionary!{"S"=>"GoTo","D"=>Object::string_literal("target")}});
    let link1=pdf.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Link","Rect"=>vec![20.into(),20.into(),80.into(),40.into()],"P"=>pages[1],"Dest"=>vec![Object::Reference(pages[0]),Object::Name(b"Fit".to_vec())]});
    let field_appearance=pdf.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),150.into(),20.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"Helv"=>font}}},b"BT /Helv 12 Tf 0 g 2 4 Td (kept) Tj ET".to_vec()));
    let field=pdf.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Widget","FT"=>"Tx","T"=>Object::string_literal("value"),"V"=>Object::string_literal("kept"),"DA"=>Object::string_literal("/Helv 12 Tf 0 g"),"Rect"=>vec![30.into(),50.into(),180.into(),70.into()],"P"=>pages[0],"AP"=>dictionary!{"N"=>field_appearance}});
    pdf.get_dictionary_mut(pages[0]).unwrap().set(
        "Annots",
        vec![Object::Reference(link0), Object::Reference(field)],
    );
    pdf.get_dictionary_mut(pages[1])
        .unwrap()
        .set("Annots", vec![Object::Reference(link1)]);
    let attachment = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"EmbeddedFile"},
        b"ATTACHMENT-BYTES".to_vec(),
    ));
    let spec=pdf.add_object(dictionary!{"Type"=>"Filespec","F"=>Object::string_literal("note.txt"),"UF"=>lopdf::text_string("note.txt"),"EF"=>dictionary!{"F"=>attachment},"AFRelationship"=>"Data"});
    let catalog=pdf.add_object(dictionary!{"Type"=>"Catalog","Pages"=>root,"AcroForm"=>dictionary!{"Fields"=>vec![Object::Reference(field)],"DR"=>dictionary!{"Font"=>dictionary!{"Helv"=>font}},"DA"=>Object::string_literal("/Helv 12 Tf 0 g")},"Names"=>dictionary!{"Dests"=>dictionary!{"Names"=>vec![Object::string_literal("target"),Object::Array(vec![Object::Reference(pages[1]),Object::Name(b"XYZ".to_vec()),Object::Null,120.into(),Object::Null])]},"EmbeddedFiles"=>dictionary!{"Names"=>vec![Object::string_literal("note.txt"),Object::Reference(spec)]}},"AF"=>vec![Object::Reference(spec)],"OCProperties"=>dictionary!{"OCGs"=>vec![Object::Reference(group)],"D"=>dictionary!{"BaseState"=>"ON","Order"=>vec![Object::Reference(group)],"OFF"=>vec![Object::Reference(group)]}}});
    pdf.trailer.set("Root", catalog);
    pdf.trailer.set(
        "ID",
        vec![
            Object::string_literal(b"SuniPDFStructure".to_vec()),
            Object::string_literal(b"SuniPDFStructure".to_vec()),
        ],
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let bytes = bookmarks::edit(
        &bytes,
        None,
        &BookmarkEdit::Add {
            parent: None,
            title: "Outside selection".into(),
            page: 1,
        },
    )
    .unwrap();
    bookmarks::edit(
        &bytes,
        None,
        &BookmarkEdit::Add {
            parent: None,
            title: "Selected".into(),
            page: 0,
        },
    )
    .unwrap()
}
fn dictionary<'a>(pdf: &'a lopdf::Document, value: &'a Object) -> &'a Dictionary {
    pdf.dereference(value).unwrap().1.as_dict().unwrap()
}
fn load(path: &std::path::Path, password: Option<&str>) -> lopdf::Document {
    // Supply the password while parsing: the encrypted reader defers body objects
    // until authentication, so decrypting a passwordless load cannot recover them.
    let pdf = lopdf::Document::load_with_options(
        path,
        lopdf::LoadOptions::with_password(password.unwrap_or("")),
    )
    .unwrap();
    assert!(
        !pdf.is_encrypted(),
        "structure oracle was not authenticated"
    );
    if let Err(error) = pdf.catalog() {
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/feature-030-evidence/structure");
        std::fs::create_dir_all(&output).unwrap();
        let diagnostic = output.join(format!(
            "diagnostic-{}",
            path.file_name().unwrap().to_string_lossy()
        ));
        std::fs::copy(path, &diagnostic).unwrap();
        panic!(
            "saved synthetic structure catalog failed: {error:?}; root={:?}; objects={:?}; evidence={}",
            pdf.trailer.get(b"Root"),
            pdf.objects.keys().collect::<Vec<_>>(),
            diagnostic.display()
        );
    }
    pdf
}
fn outline_dicts(pdf: &lopdf::Document) -> Vec<&Dictionary> {
    let root = dictionary(pdf, pdf.catalog().unwrap().get(b"Outlines").unwrap());
    let mut next = root.get(b"First").ok();
    let mut result = Vec::new();
    while let Some(value) = next {
        let item = dictionary(pdf, value);
        result.push(item);
        next = item.get(b"Next").ok();
        assert!(result.len() < 10);
    }
    result
}
fn annotations(pdf: &lopdf::Document, page: ObjectId) -> Vec<ObjectId> {
    pdf.get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_reference().unwrap())
        .collect()
}
fn check_attachments_layers_fields(pdf: &lopdf::Document, count: usize) {
    let catalog = pdf.catalog().unwrap();
    let names = dictionary(pdf, catalog.get(b"Names").unwrap());
    let files = dictionary(pdf, names.get(b"EmbeddedFiles").unwrap())
        .get(b"Names")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(files.len(), count * 2);
    for pair in files.as_chunks::<2>().0 {
        let spec = dictionary(pdf, &pair[1]);
        let ef = dictionary(pdf, spec.get(b"EF").unwrap());
        let stream = pdf
            .dereference(ef.get(b"F").unwrap())
            .unwrap()
            .1
            .as_stream()
            .unwrap();
        assert_eq!(stream.get_plain_content().unwrap(), b"ATTACHMENT-BYTES");
    }
    let groups = dictionary(pdf, catalog.get(b"OCProperties").unwrap())
        .get(b"OCGs")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), count);
    for page in pdf.get_pages().values() {
        let resources = dictionary(
            pdf,
            pdf.get_dictionary(*page)
                .unwrap()
                .get(b"Resources")
                .unwrap(),
        );
        let properties = dictionary(pdf, resources.get(b"Properties").unwrap());
        assert!(groups.contains(properties.get(b"L1").unwrap()));
    }
    let form = dictionary(pdf, catalog.get(b"AcroForm").unwrap());
    let fields = form.get(b"Fields").unwrap().as_array().unwrap();
    assert_eq!(fields.len(), count);
    let mut names = std::collections::HashSet::new();
    for field in fields {
        let field = dictionary(pdf, field);
        assert!(names.insert(lopdf::decode_text_string(field.get(b"T").unwrap()).unwrap()));
        assert_eq!(field.get(b"V").unwrap().as_str().unwrap(), b"kept");
        let page = field.get(b"P").unwrap().as_reference().unwrap();
        assert!(pdf.get_pages().values().any(|id| *id == page));
    }
}
#[test]
fn merge_extract_insert_preserve_independent_structure_encryption_and_originals() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let mut evidence = feature_evidence::Evidence::new("structure");
    let original = fixture();
    let first = dir.path().join("first.pdf");
    let second = dir.path().join("second.pdf");
    std::fs::write(&first, &original).unwrap();
    std::fs::write(&second, &original).unwrap();
    let merged = dir.path().join("merged.pdf");
    assert_eq!(
        engine
            .merge(&[first.clone(), second.clone()], &merged)
            .unwrap(),
        4
    );
    let pdf = load(&merged, None);
    check_attachments_layers_fields(&pdf, 2);
    evidence.save(
        "merged",
        &merged,
        &engine.open(&merged, None).unwrap(),
        None,
    );
    let pages = pdf.get_pages();
    let outlines = outline_dicts(&pdf);
    assert_eq!(outlines.len(), 4);
    assert_eq!(
        outlines[2].get(b"Dest").unwrap().as_array().unwrap()[0]
            .as_reference()
            .unwrap(),
        pages[&4]
    );
    for (index, page) in [pages[&1], pages[&3]].iter().enumerate() {
        let annots = annotations(&pdf, *page);
        let action = dictionary(
            &pdf,
            pdf.get_dictionary(annots[0]).unwrap().get(b"A").unwrap(),
        );
        let destination = action.get(b"D").unwrap().as_array().unwrap();
        assert_eq!(
            destination[0].as_reference().unwrap(),
            pages[&((index * 2 + 2) as u32)]
        );
        assert_eq!(
            &destination[2..],
            &[Object::Null, Object::Integer(120), Object::Null]
        );
    }
    let plain = dir.path().join("selected-plain.pdf");
    engine
        .open(&first, None)
        .unwrap()
        .extract_to(&[0], &plain)
        .unwrap();
    check_attachments_layers_fields(&load(&plain, None), 1);
    evidence.save(
        "extracted-plain",
        &plain,
        &engine.open(&plain, None).unwrap(),
        None,
    );
    for aes in [false, true] {
        let bytes = security::protect(
            &original,
            None,
            &Protection {
                user_password: "reader".into(),
                owner_password: "owner".into(),
                allow_copy: true,
                allow_modify: true,
                allow_print: true,
                allow_annotate: true,
                aes256: aes,
            },
        )
        .unwrap();
        let source = dir.path().join(format!("encrypted-{aes}.pdf"));
        std::fs::write(&source, &bytes).unwrap();
        let doc = engine.open(&source, Some("reader")).unwrap();
        let output = dir.path().join(format!("selected-{aes}.pdf"));
        doc.extract_to(&[0], &output).unwrap();
        let reopened = engine.open(&output, Some("reader")).unwrap();
        assert!(reopened.properties().encrypted);
        assert_eq!(reopened.page_count(), 1);
        evidence.save(
            if aes {
                "extracted-aes256"
            } else {
                "extracted-aes128"
            },
            &output,
            &reopened,
            Some("reader"),
        );
        let pdf = load(&output, Some("reader"));
        check_attachments_layers_fields(&pdf, 1);
        let page = pdf.get_pages()[&1];
        let links = annotations(&pdf, page);
        assert!(!pdf.get_dictionary(links[0]).unwrap().has(b"A"));
        let outlines = outline_dicts(&pdf);
        assert!(!outlines[0].has(b"Dest"));
        assert_eq!(
            outlines[1].get(b"Dest").unwrap().as_array().unwrap()[0]
                .as_reference()
                .unwrap(),
            page
        );
        assert!(!pdf
            .objects
            .values()
            .filter_map(|value| value.as_stream().ok())
            .any(|stream| stream
                .get_plain_content()
                .is_ok_and(|content| content.windows(5).any(|bytes| bytes == b"PAGE1"))));
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }
    let mut target = engine.open(&first, None).unwrap();
    let before = target.snapshot().unwrap();
    let logical_before = logical_snapshot(&target);
    target.insert_pages_from(&second, None, &[], 1).unwrap();
    assert_eq!(target.page_count(), 4);
    let inserted = dir.path().join("inserted.pdf");
    target.save_as(&inserted).unwrap();
    check_attachments_layers_fields(&load(&inserted, None), 2);
    evidence.save(
        "inserted",
        &inserted,
        &engine.open(&inserted, None).unwrap(),
        None,
    );
    target.restore(before.clone()).unwrap();
    assert_eq!(logical_snapshot(&target), logical_before);
    assert_eq!(std::fs::read(&first).unwrap(), original);
    assert_eq!(std::fs::read(&second).unwrap(), original);
    evidence.finish();
}
#[test]
fn malformed_graphs_copy_restrictions_and_duplicate_widgets_fail_without_outputs_or_changes() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let original = fixture();
    let path = dir.path().join("source.pdf");
    std::fs::write(&path, &original).unwrap();
    let doc = engine.open(&path, None).unwrap();
    let output = dir.path().join("duplicate.pdf");
    assert!(doc.extract_to(&[0, 0], &output).is_err());
    assert!(!output.exists());
    let restricted = security::protect(
        &original,
        None,
        &Protection {
            user_password: "reader".into(),
            owner_password: "owner".into(),
            allow_copy: false,
            allow_modify: true,
            allow_print: true,
            allow_annotate: true,
            aes256: true,
        },
    )
    .unwrap();
    assert!(engine.open_bytes(restricted.clone(), None, &path).is_err());
    let restricted_path = dir.path().join("restricted-source.pdf");
    std::fs::write(&restricted_path, &restricted).unwrap();
    let doc = engine.open(&restricted_path, Some("reader")).unwrap();
    let original_permissions = doc.properties();
    let output = dir.path().join("restricted-derivative.pdf");
    assert_eq!(doc.extract_to(&[0], &output).unwrap(), 1);
    assert!(engine.open(&output, None).is_err());
    for password in ["reader", "owner"] {
        let derivative = engine.open(&output, Some(password)).unwrap();
        let properties = derivative.properties();
        assert!(properties.encrypted && !properties.can_copy);
        assert_eq!(properties.can_modify, original_permissions.can_modify);
        assert_eq!(properties.can_print, original_permissions.can_print);
        assert_eq!(properties.can_annotate, original_permissions.can_annotate);
        check_attachments_layers_fields(&load(&output, Some(password)), 1);
    }
    let mut imported_target = engine.open(&path, None).unwrap();
    let before_import = logical_snapshot(&imported_target);
    for password in ["reader", "owner"] {
        assert!(matches!(
            imported_target.insert_pages_from(&restricted_path, Some(password), &[0], 1),
            Err(sunipdf_core::EngineError::SecurityRestricted)
        ));
        assert_eq!(logical_snapshot(&imported_target), before_import);
    }
    let mut owner_target = engine.open(&restricted_path, Some("owner")).unwrap();
    owner_target
        .insert_pages_from(&path, None, &[0], 1)
        .unwrap();
    assert_eq!(owner_target.page_count(), 3);
    let inserted_output = dir.path().join("restricted-target-inserted.pdf");
    owner_target.save_as(&inserted_output).unwrap();
    assert!(engine.open(&inserted_output, None).is_err());
    for password in ["reader", "owner"] {
        let reopened = engine.open(&inserted_output, Some(password)).unwrap();
        assert_eq!(reopened.page_count(), 3);
        assert!(reopened.properties().encrypted && !reopened.properties().can_copy);
    }
    assert_eq!(std::fs::read(&restricted_path).unwrap(), restricted);
    let mut pdf = lopdf::Document::load_mem(&original).unwrap();
    let root = pdf
        .catalog()
        .unwrap()
        .get(b"Outlines")
        .unwrap()
        .as_reference()
        .unwrap();
    let node = pdf
        .get_dictionary(root)
        .unwrap()
        .get(b"First")
        .unwrap()
        .as_reference()
        .unwrap();
    pdf.get_dictionary_mut(node).unwrap().set("Next", node);
    let mut malformed = Vec::new();
    pdf.save_to(&mut malformed).unwrap();
    let bad = dir.path().join("bad.pdf");
    std::fs::write(&bad, &malformed).unwrap();
    let mut target = engine.open(&path, None).unwrap();
    let before = logical_snapshot(&target);
    assert!(target.insert_pages_from(&bad, None, &[0], 0).is_err());
    assert_eq!(logical_snapshot(&target), before);
    let output = dir.path().join("bad-merge.pdf");
    assert!(engine.merge(&[path.clone(), bad], &output).is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
