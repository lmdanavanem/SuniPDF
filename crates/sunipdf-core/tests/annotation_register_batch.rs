use lopdf::{dictionary, Document as Pdf, Object, ObjectId};
use std::{collections::BTreeMap, path::Path, sync::Mutex};

static PDFIUM_LOCK: Mutex<()> = Mutex::new(());
use sunipdf_core::annotation_register::{RegisterFields, RegisterRow};
use sunipdf_core::annotation_register_batch::{
    register_row_fingerprint, FieldPatch, RegisterFieldsPatch, MAX_REGISTER_BATCH_ROWS,
};
use sunipdf_core::quantity_table::{DecimalValue, QuantityFields, QuantityUnit, TypedValue};
use sunipdf_core::{security, Engine, EngineError, Protection, ReviewStatus};

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn patch() -> RegisterFieldsPatch {
    RegisterFieldsPatch {
        business_status: FieldPatch::Set("Ready for site".into()),
        discipline: FieldPatch::Clear,
        custom: BTreeMap::from([
            ("Room".into(), FieldPatch::Set("A-102".into())),
            ("Temporary".into(), FieldPatch::Clear),
            ("Preserved".into(), FieldPatch::Keep),
        ]),
    }
}

fn initial() -> RegisterFields {
    RegisterFields {
        business_status: Some("Open".into()),
        discipline: Some("Electrical".into()),
        custom: BTreeMap::from([
            ("Room".into(), "A-101".into()),
            ("Temporary".into(), "Remove me".into()),
            ("Preserved".into(), "Literal =SUM(A1)".into()),
            ("Unspecified".into(), "Keep me".into()),
        ]),
    }
}

fn source(engine: &Engine) -> Vec<u8> {
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("seed.pdf"),
        )
        .unwrap();
    doc.add_count_marker(0, (80.0, 90.0), 18.0, "Lighting", None)
        .unwrap();
    doc.add_count_marker(1, (90.0, 100.0), 18.0, "Switches", None)
        .unwrap();
    doc.add_count_marker(0, (150.0, 90.0), 18.0, "Unselected", None)
        .unwrap();
    for (page, index) in [(1, 0), (2, 0)] {
        let register = doc.annotation_register().unwrap();
        let row = register
            .rows
            .iter()
            .find(|row| row.page == page && row.index == index)
            .unwrap();
        doc.set_annotation_register_fields(&register.target(row), &initial(), None)
            .unwrap();
    }
    let table = doc.quantity_table().unwrap();
    let row = table
        .rows()
        .iter()
        .find(|row| row.register.page == 1 && row.register.index == 0)
        .unwrap();
    doc.set_quantity_fields(
        &table.target(row).unwrap(),
        &QuantityFields {
            custom: BTreeMap::from([(
                "Depth".into(),
                TypedValue::Number {
                    value: DecimalValue::parse("0.25").unwrap(),
                    unit: Some(QuantityUnit::Metres),
                },
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    doc.set_review_status(0, 0, ReviewStatus::Accepted, Some("Inspector"))
        .unwrap();
    doc.snapshot().unwrap()
}

fn selected(rows: &[RegisterRow]) -> Vec<RegisterRow> {
    rows.iter()
        .filter(|row| row.index == 0 && !row.is_reply)
        .cloned()
        .collect()
}

fn logical(bytes: &[u8]) -> Pdf {
    let mut pdf = Pdf::load_mem(bytes).unwrap();
    let ids = pdf.trailer.get_mut(b"ID").unwrap().as_array_mut().unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().all(|id| matches!(id, Object::String(_, _))));
    ids[1] = Object::Null;
    pdf
}

fn same_document(a: &[u8], b: &[u8]) {
    let a = logical(a);
    let b = logical(b);
    assert_eq!(a.version, b.version);
    assert_eq!(a.objects, b.objects);
    assert_eq!(a.trailer, b.trailer);
}

fn annot_id(pdf: &Pdf, page: u32, index: usize) -> ObjectId {
    let page = pdf.get_dictionary(pdf.get_pages()[&page]).unwrap();
    pdf.dereference(page.get(b"Annots").unwrap())
        .unwrap()
        .1
        .as_array()
        .unwrap()[index]
        .as_reference()
        .unwrap()
}

fn write_pdf(mut pdf: Pdf) -> Vec<u8> {
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn reviewed_batch_preserves_all_other_pdf_data_saved_sources_and_single_snapshot_history() {
    let _guard = PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let engine = Engine::new().expect("PDFium is required");
    let bytes = source(&engine);
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("original.pdf");
    std::fs::write(&input, &bytes).unwrap();
    let mut doc = engine.open(&input, None).unwrap();
    let before = doc.snapshot().unwrap();
    let register = doc.annotation_register().unwrap();
    let chosen = selected(&register.rows);
    let plan = doc
        .preview_annotation_register_batch(&register, &chosen, &patch())
        .unwrap();
    assert_eq!(
        (plan.report().selected_count, plan.report().changed_count),
        (2, 2)
    );
    let preview = serde_json::to_value(plan.report()).unwrap();
    assert_eq!(preview["changes"][0]["before"]["business_status"], "Open");
    assert_eq!(
        preview["changes"][0]["after"]["business_status"],
        "Ready for site"
    );
    assert!(preview["changes"][0]["after"]["discipline"].is_null());
    same_document(&before, &doc.snapshot().unwrap());
    let applied = doc.apply_annotation_register_batch(&plan).unwrap();
    assert_eq!(applied, *plan.report());
    let after = doc.snapshot().unwrap();
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    let updated = doc.annotation_register().unwrap();
    for old in &register.rows {
        let new = updated
            .rows
            .iter()
            .find(|row| row.page == old.page && row.index == old.index)
            .unwrap();
        if chosen.contains(old) {
            assert_eq!(
                new.fields.business_status.as_deref(),
                Some("Ready for site")
            );
            assert_eq!(new.fields.discipline, None);
            assert_eq!(new.fields.custom["Room"], "A-102");
            assert_eq!(new.fields.custom["Preserved"], "Literal =SUM(A1)");
            assert_eq!(new.fields.custom["Unspecified"], "Keep me");
            assert!(!new.fields.custom.contains_key("Temporary"));
            let mut expected = old.clone();
            expected.fields = new.fields.clone();
            expected.modified = new.modified.clone();
            assert_eq!(
                *new, expected,
                "identity, quantity group, geometry and review state stay exact"
            );
        } else {
            assert_eq!(old, new);
        }
    }
    let mut original_pdf = logical(&before);
    let mut changed_pdf = logical(&after);
    for row in &chosen {
        for pdf in [&mut original_pdf, &mut changed_pdf] {
            let id = annot_id(pdf, row.page as u32, row.index);
            let dict = pdf.get_dictionary_mut(id).unwrap();
            dict.remove(b"SuniPDFRegister");
            dict.remove(b"M");
        }
    }
    assert_eq!(
        original_pdf.objects, changed_pdf.objects,
        "only selected register metadata and dates may change"
    );
    assert_eq!(original_pdf.trailer, changed_pdf.trailer);
    assert!(
        doc.apply_annotation_register_batch(&plan).is_err(),
        "a committed plan cannot be replayed"
    );
    doc.restore(before.clone()).unwrap();
    assert_eq!(doc.annotation_register().unwrap().rows, register.rows);
    doc.restore(after.clone()).unwrap();
    assert_eq!(doc.annotation_register().unwrap().rows, updated.rows);
    let output = directory.path().join("edited.pdf");
    doc.save_as(&output).unwrap();
    let reopened = engine.open(&output, None).unwrap();
    assert_eq!(reopened.annotation_register().unwrap().rows, updated.rows);
    assert_eq!(std::fs::read(input).unwrap(), bytes);
}

#[test]
fn no_op_selection_forged_rows_and_stale_source_never_modify_any_record() {
    let _guard = PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let engine = Engine::new().unwrap();
    let bytes = source(&engine);
    let mut doc = engine
        .open_bytes(bytes, None, Path::new("guards.pdf"))
        .unwrap();
    let register = doc.annotation_register().unwrap();
    let chosen = selected(&register.rows);
    let before = doc.snapshot().unwrap();
    let no_op = doc
        .preview_annotation_register_batch(&register, &chosen, &RegisterFieldsPatch::default())
        .unwrap();
    assert_eq!(
        (no_op.report().selected_count, no_op.report().changed_count),
        (2, 0)
    );
    assert!(doc.apply_annotation_register_batch(&no_op).is_err());
    assert!(doc
        .preview_annotation_register_batch(&register, &[], &patch())
        .is_err());
    assert!(doc
        .preview_annotation_register_batch(
            &register,
            &[chosen[0].clone(), chosen[0].clone()],
            &patch()
        )
        .is_err());
    assert!(doc
        .preview_annotation_register_batch(
            &register,
            &vec![chosen[0].clone(); MAX_REGISTER_BATCH_ROWS + 1],
            &patch()
        )
        .is_err());
    let mut forged = register.clone();
    forged.rows[0].fields.business_status = Some("Forged".into());
    assert!(doc
        .preview_annotation_register_batch(&forged, &[forged.rows[0].clone()], &patch())
        .is_err());
    forged = register.clone();
    forged.source_sha256 = "0".repeat(64);
    assert!(doc
        .preview_annotation_register_batch(&forged, &chosen, &patch())
        .is_err());
    same_document(&before, &doc.snapshot().unwrap());
    let plan = doc
        .preview_annotation_register_batch(&register, &chosen, &patch())
        .unwrap();
    // A change outside the selected records still invalidates the complete reviewed document.
    doc.add_count_marker(1, (160.0, 90.0), 18.0, "New record", None)
        .unwrap();
    let changed = doc.snapshot().unwrap();
    assert!(doc.apply_annotation_register_batch(&plan).is_err());
    assert!(doc
        .preview_annotation_register_batch(&register, &chosen, &patch())
        .is_err());
    same_document(&changed, &doc.snapshot().unwrap());
    doc.restore(before).unwrap();
    let first_after = plan.report().changes[0].after.clone();
    doc.set_annotation_register_fields(&register.target(&chosen[0]), &first_after, None)
        .unwrap();
    let mixed_register = doc.annotation_register().unwrap();
    let mixed = doc
        .preview_annotation_register_batch(
            &mixed_register,
            &selected(&mixed_register.rows),
            &patch(),
        )
        .unwrap();
    assert_eq!(
        (mixed.report().selected_count, mixed.report().changed_count),
        (2, 1)
    );
    assert_eq!(
        mixed.report().changes[0].before,
        mixed.report().changes[0].after
    );
    let before_mixed = doc.snapshot().unwrap();
    doc.apply_annotation_register_batch(&mixed).unwrap();
    let after_mixed = doc.snapshot().unwrap();
    let old = logical(&before_mixed);
    let new = logical(&after_mixed);
    assert_eq!(
        old.get_dictionary(annot_id(&old, 1, 0)).unwrap(),
        new.get_dictionary(annot_id(&new, 1, 0)).unwrap(),
        "a selected unchanged row is guarded but not touched"
    );
}

#[test]
fn last_invalid_fields_and_locked_or_shared_targets_refuse_the_entire_batch() {
    let _guard = PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let engine = Engine::new().unwrap();
    let bytes = source(&engine);
    let mut doc = engine
        .open_bytes(bytes.clone(), None, Path::new("limit.pdf"))
        .unwrap();
    let register = doc.annotation_register().unwrap();
    let last = register
        .rows
        .iter()
        .find(|row| row.page == 2 && row.index == 0)
        .unwrap();
    doc.set_annotation_register_fields(
        &register.target(last),
        &RegisterFields {
            custom: (0..16)
                .map(|index| (format!("Field {index}"), "value".into()))
                .collect(),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let register = doc.annotation_register().unwrap();
    let before = doc.snapshot().unwrap();
    let add = RegisterFieldsPatch {
        custom: BTreeMap::from([("New field".into(), FieldPatch::Set("value".into()))]),
        ..Default::default()
    };
    assert!(doc
        .preview_annotation_register_batch(&register, &selected(&register.rows), &add)
        .is_err());
    same_document(&before, &doc.snapshot().unwrap());
    for mode in 0..5 {
        let mut pdf = Pdf::load_mem(&bytes).unwrap();
        let id = annot_id(&pdf, 2, 0);
        match mode {
            0 => {
                pdf.get_dictionary_mut(id).unwrap().set("F", 128);
            }
            1 => {
                pdf.get_dictionary_mut(id)
                    .unwrap()
                    .set("Subtype", Object::Name(b"Widget".to_vec()));
                pdf.get_dictionary_mut(id).unwrap().remove(b"SuniPDFCount");
            }
            2 => {
                let page = pdf.get_pages()[&2];
                let other = pdf.get_pages()[&1];
                pdf.get_dictionary_mut(id).unwrap().set("P", other);
                assert_ne!(page, other);
            }
            3 | 4 => {
                // Non-count records may lack NM; object/array alias guards must still protect them.
                for page in [1, 2] {
                    let id = annot_id(&pdf, page, 0);
                    let dict = pdf.get_dictionary_mut(id).unwrap();
                    dict.remove(b"SuniPDFCount");
                    dict.remove(b"NM");
                }
                let first = annot_id(&pdf, 1, 0);
                let page2 = pdf.get_pages()[&2];
                if mode == 3 {
                    pdf.get_dictionary_mut(page2)
                        .unwrap()
                        .set("Annots", vec![Object::Reference(first)]);
                } else {
                    let page1 = pdf.get_pages()[&1];
                    let array = pdf.add_object(vec![Object::Reference(first)]);
                    pdf.get_dictionary_mut(page1).unwrap().set("Annots", array);
                    pdf.get_dictionary_mut(page2).unwrap().set("Annots", array);
                }
            }
            _ => unreachable!(),
        }
        let mut doc = engine
            .open_bytes(write_pdf(pdf), None, Path::new("invalid.pdf"))
            .unwrap();
        let before = doc.snapshot().unwrap();
        let register = doc.annotation_register().unwrap();
        assert!(
            doc.preview_annotation_register_batch(&register, &selected(&register.rows), &patch())
                .is_err(),
            "mode {mode}"
        );
        same_document(&before, &doc.snapshot().unwrap());
        // Retain a mutable handle to ensure the failed preparation never alters its state.
        doc.restore(before).unwrap();
    }
}

#[test]
fn permission_and_exact_selection_cap_are_enforced() {
    let _guard = PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let engine = Engine::new().unwrap();
    let bytes = source(&engine);
    let mut pdf = Pdf::load_mem(&bytes).unwrap();
    let page = pdf.get_pages()[&1];
    let original = pdf.get_dictionary(annot_id(&pdf, 1, 0)).unwrap().clone();
    let entries = (0..MAX_REGISTER_BATCH_ROWS)
        .map(|index| {
            let mut dict = original.clone();
            dict.set(
                "NM",
                Object::string_literal(format!("SuniPDF-batch-{index}")),
            );
            Object::Reference(pdf.add_object(dict))
        })
        .collect::<Vec<_>>();
    pdf.get_dictionary_mut(page).unwrap().set("Annots", entries);
    let mut doc = engine
        .open_bytes(write_pdf(pdf), None, Path::new("bounded.pdf"))
        .unwrap();
    let register = doc.annotation_register().unwrap();
    let chosen = register
        .rows
        .iter()
        .filter(|row| row.page == 1)
        .cloned()
        .collect::<Vec<_>>();
    let plan = doc
        .preview_annotation_register_batch(&register, &chosen, &patch())
        .unwrap();
    assert_eq!(plan.report().changed_count, MAX_REGISTER_BATCH_ROWS);
    assert_eq!(
        doc.apply_annotation_register_batch(&plan)
            .unwrap()
            .changed_count,
        MAX_REGISTER_BATCH_ROWS
    );
    for (copy, annotate) in [(false, true), (true, false)] {
        let user = credential();
        let owner = credential();
        let protected = security::protect(
            &bytes,
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner,
                allow_copy: copy,
                allow_annotate: annotate,
                ..Default::default()
            },
        )
        .unwrap();
        let mut protected = engine
            .open_bytes(protected, Some(&user), Path::new("protected.pdf"))
            .unwrap();
        assert!(matches!(
            protected.preview_annotation_register_batch(&register, &chosen, &patch()),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            protected.apply_annotation_register_batch(&plan),
            Err(EngineError::SecurityRestricted)
        ));
    }
    let mut pdf = Pdf::load_mem(&bytes).unwrap();
    let signature = pdf.add_object(dictionary! {
        "FT" => "Sig", "T" => Object::string_literal("Synthetic signature"),
        "V" => dictionary! { "Type" => "Sig", "ByteRange" => vec![0.into(), 1.into(), 2.into(), 3.into()],
                              "Contents" => Object::string_literal("synthetic") }
    });
    let form = pdf.add_object(dictionary! { "Fields" => vec![Object::Reference(signature)] });
    let catalog = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
    pdf.get_dictionary_mut(catalog)
        .unwrap()
        .set("AcroForm", form);
    let mut signed = engine
        .open_bytes(write_pdf(pdf), None, Path::new("signed.pdf"))
        .unwrap();
    assert!(signed.signature_count() > 0);
    let signed_register = signed.annotation_register().unwrap();
    let signed_before = signed.snapshot().unwrap();
    assert!(signed
        .preview_annotation_register_batch(
            &signed_register,
            &selected(&signed_register.rows),
            &patch()
        )
        .is_err());
    assert!(signed.apply_annotation_register_batch(&plan).is_err());
    same_document(&signed_before, &signed.snapshot().unwrap());
}

#[test]
fn strict_patch_json_and_complete_row_fingerprints_preserve_explicit_intent() {
    let _guard = PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let parsed: RegisterFieldsPatch = serde_json::from_str(r#"{"business_status":{"operation":"set","value":"Ready"},"custom":{"Room":{"operation":"clear"}}}"#).unwrap();
    parsed.validate().unwrap();
    assert_eq!(parsed.discipline, FieldPatch::Keep);
    assert_eq!(
        serde_json::from_slice::<RegisterFieldsPatch>(&serde_json::to_vec(&parsed).unwrap())
            .unwrap(),
        parsed
    );
    for text in [
        r#"{"custom":{"Room":{"operation":"clear"},"Room":{"operation":"keep"}}}"#,
        r#"{"business_status":{"operation":"execute","value":"anything"}}"#,
        r#"{"custom":{"Room":{"operation":"set","value":1}}}"#,
        r#"{"business_status":{"operation":"keep","extra":true}}"#,
        r#"{"custom":{" Room":{"operation":"keep"}}}"#,
        r#"{"custom":{"a=b":{"operation":"clear"}}}"#,
        r#"{"unknown":true}"#,
    ] {
        assert!(
            serde_json::from_str::<RegisterFieldsPatch>(text).is_err(),
            "{text}"
        );
    }
    let too_many = RegisterFieldsPatch {
        custom: (0..17)
            .map(|index| (format!("Field {index}"), FieldPatch::Keep))
            .collect(),
        ..Default::default()
    };
    assert!(too_many.validate().is_err());
    assert!(
        serde_json::from_slice::<RegisterFieldsPatch>(&serde_json::to_vec(&too_many).unwrap())
            .is_err()
    );
    let empty_status = RegisterFieldsPatch {
        business_status: FieldPatch::Set(String::new()),
        ..Default::default()
    };
    assert!(empty_status.validate().is_err());
    let mut field = RegisterFieldsPatch {
        custom: BTreeMap::from([("Empty is literal".into(), FieldPatch::Set(String::new()))]),
        ..Default::default()
    };
    field.validate().unwrap();
    field
        .custom
        .insert("Too long".into(), FieldPatch::Set("x".repeat(1025)));
    assert!(field.validate().is_err());
    let engine = Engine::new().unwrap();
    let doc = engine
        .open_bytes(source(&engine), None, Path::new("fingerprints.pdf"))
        .unwrap();
    let row = doc.annotation_register().unwrap().rows.remove(0);
    let fingerprint = register_row_fingerprint(&row).unwrap();
    assert_eq!(
        fingerprint,
        register_row_fingerprint(
            &serde_json::from_slice(&serde_json::to_vec(&row).unwrap()).unwrap()
        )
        .unwrap()
    );
    for altered in [
        {
            let mut row = row.clone();
            row.annotation_id = Some("different".into());
            row
        },
        {
            let mut row = row.clone();
            row.fields.custom.insert("Other".into(), "changed".into());
            row
        },
        {
            let mut row = row.clone();
            row.review_status = Some("Rejected".into());
            row
        },
    ] {
        assert_ne!(fingerprint, register_row_fingerprint(&altered).unwrap());
    }
}
