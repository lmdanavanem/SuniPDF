use lopdf::{dictionary, Dictionary, Object};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    annot_rich::RichAnnotation,
    annotation_register::RegisterFields,
    measurement::{Measurement, MeasurementKind, MeasurementUnit},
    security, Engine, EngineError, PageCalibration, PageMargins, Protection, ResizeMode, Rotation,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn measurement() -> Measurement {
    Measurement {
        kind: MeasurementKind::Length,
        points: vec![(40.0, 80.0), (140.0, 80.0)],
        calibration: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}

fn parse(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or_default()),
    )
    .unwrap()
}

fn assert_unchanged(before: &[u8], after: &[u8], password: Option<&str>) {
    let normalized = |bytes: &[u8]| {
        let mut pdf = parse(bytes, password);
        pdf.trailer.remove(b"ID");
        for object in pdf.objects.values_mut() {
            if let Object::Stream(stream) = object {
                stream.start_position = None;
                stream.allows_compression = true;
            }
        }
        let permissions = pdf
            .encryption_state
            .as_ref()
            .map(|s| s.permissions().bits());
        (pdf.objects, pdf.trailer, permissions)
    };
    assert_eq!(normalized(before), normalized(after));
}

fn annotations(pdf: &lopdf::Document, page: u32) -> Vec<(Option<lopdf::ObjectId>, Dictionary)> {
    let dictionary = pdf.get_dictionary(pdf.get_pages()[&page]).unwrap();
    let Ok(raw) = dictionary.get(b"Annots") else {
        return Vec::new();
    };
    pdf.dereference(raw)
        .unwrap()
        .1
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let (id, object) = pdf.dereference(entry).unwrap();
            (id, object.as_dict().unwrap().clone())
        })
        .collect()
}

fn mutate(bytes: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = parse(bytes, None);
    change(&mut pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn utf16(text: &str) -> Object {
    let mut bytes = vec![0xfe, 0xff];
    bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    Object::String(bytes, lopdf::StringFormat::Hexadecimal)
}

#[test]
fn editing_preserves_identity_threads_register_defaults_and_preview_undo_across_page_rotations() {
    let engine = engine();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let mut doc = engine
            .open_bytes(SOURCE.to_vec(), None, Path::new("plan.pdf"))
            .unwrap();
        doc.set_page_rotation(0, rotation).unwrap();
        doc.crop_page(
            0,
            PageMargins {
                left: 10.0,
                right: 13.0,
                top: 12.0,
                bottom: 14.0,
            },
        )
        .unwrap();
        let initial = measurement();
        let scale = PageCalibration {
            points: initial.calibration,
            known_distance: 7.0,
            unit: initial.unit,
        };
        doc.set_page_calibration(0, Some(&scale)).unwrap();
        let index = doc
            .add_measurement_annotation(0, &initial, Some("Plan reviewer"))
            .unwrap();
        doc.add_measurement_annotation(1, &initial, Some("Other page"))
            .unwrap();
        doc.add_reply(0, index, "Keep this review comment", Some("Inspector"))
            .unwrap();
        let report = doc.annotation_register().unwrap();
        let row = report
            .rows
            .iter()
            .find(|r| r.page == 1 && r.index == index)
            .unwrap();
        let fields = RegisterFields {
            discipline: Some("Architecture".into()),
            business_status: Some("Checked".into()),
            custom: BTreeMap::from([("Room".into(), "A-101".into())]),
        };
        doc.set_annotation_register_fields(&report.target(row), &fields, None)
            .unwrap();
        // Preserve a real popup relationship in addition to the review reply and register fields.
        let with_popup = mutate(&doc.snapshot().unwrap(), |pdf| {
            let page = pdf.get_pages()[&1];
            let id = annotations(pdf, 1)[index].0.unwrap();
            let popup = pdf.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Popup", "Parent" => id, "P" => page,
                "Rect" => vec![20.into(), 20.into(), 120.into(), 120.into()],
            });
            pdf.get_dictionary_mut(id).unwrap().set("Popup", popup);
            let mut entries = pdf
                .dereference(pdf.get_dictionary(page).unwrap().get(b"Annots").unwrap())
                .unwrap()
                .1
                .as_array()
                .unwrap()
                .clone();
            entries.push(popup.into());
            pdf.get_dictionary_mut(page).unwrap().set("Annots", entries);
        });
        doc.restore(with_popup).unwrap();
        let before_render = doc.snapshot().unwrap();
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let before = doc.snapshot().unwrap();
        let before_pdf = parse(&before, None);
        let before_annots = annotations(&before_pdf, 1);
        let original = doc.measurement_records(0).unwrap().remove(0);
        let target = doc.measurement_target(0, &original).unwrap();
        let mut updated = original.measurement.clone();
        updated.points[1].0 = 160.0; // Geometry replacement is explicit, even if the UI only changes settings.
        updated.known_distance = 20.0;
        updated.precision = 3;
        let annotation = RichAnnotation::UpdateMeasurement {
            target: target.clone(),
            measurement: updated.clone(),
        };
        assert!(doc.preview_rich_annotation(1, &annotation, None).is_err());
        let preview = doc
            .preview_rich_annotation(0, &annotation, Some("Must not replace author"))
            .unwrap();
        assert_unchanged(&before, &doc.snapshot().unwrap(), None);
        assert_eq!(preview.index, index);
        assert!(preview.image.rgba.iter().any(|&channel| channel != 255));
        drop(preview); // Cancelling a real preview leaves every document object unchanged.
        assert_unchanged(&before, &doc.snapshot().unwrap(), None);
        let preview = doc.preview_rich_annotation(0, &annotation, None).unwrap();
        assert_eq!(
            doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                .unwrap(),
            index
        );
        assert_ne!(pixels, doc.render_page(0, 1.0).unwrap().rgba);
        let after = doc.snapshot().unwrap();
        let after_pdf = parse(&after, None);
        let after_annots = annotations(&after_pdf, 1);
        assert_eq!(before_annots.len(), after_annots.len());
        assert_eq!(
            before_annots[index].0, after_annots[index].0,
            "same annotation object"
        );
        for key in [
            b"NM".as_slice(),
            b"T",
            b"CreationDate",
            b"Popup",
            b"SuniPDFRegister",
            b"P",
        ] {
            assert_eq!(
                before_annots[index].1.get(key).ok(),
                after_annots[index].1.get(key).ok()
            );
        }
        for (i, previous) in before_annots
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
        {
            assert_eq!(previous, &after_annots[i], "reply and popup are unchanged");
        }
        assert_eq!(annotations(&before_pdf, 2), annotations(&after_pdf, 2));
        assert_eq!(doc.page_calibration(0).unwrap(), Some(scale));
        let saved = doc.measurement_records(0).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].annotation_id, original.annotation_id);
        assert_eq!(saved[0].measurement, updated);
        assert_eq!(saved[0].measurement.value().unwrap(), 24.0);
        assert_eq!(
            after_annots[index].1.get(b"Contents").unwrap(),
            &lopdf::text_string("24.000 m")
        );
        assert!(
            doc.update_measurement_annotation(&target, &initial)
                .is_err(),
            "old target cannot be reused"
        );
        assert_unchanged(&after, &doc.snapshot().unwrap(), None);
        let reopened = engine
            .open_bytes(after.clone(), None, Path::new("saved.pdf"))
            .unwrap();
        assert_eq!(reopened.measurement_records(0).unwrap(), saved);
        doc.restore(before.clone()).unwrap();
        assert_eq!(
            doc.measurement_records(0).unwrap()[0],
            original,
            "Undo restores value and identity"
        );
        // Reopening discards PDFium's unused render appearances, so compare to the complete
        // pre-render snapshot. Refused operations above still compare every post-render object.
        assert_unchanged(&before_render, &doc.snapshot().unwrap(), None);
        doc.restore(after.clone()).unwrap();
        assert_eq!(
            doc.measurement_records(0).unwrap(),
            saved,
            "Redo restores replacement"
        );
    }
}

#[test]
fn stale_invalid_locked_future_shared_and_foreign_measurements_refuse_without_mutation() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("plan.pdf"))
        .unwrap();
    let index = doc
        .add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    let baseline = doc.snapshot().unwrap();
    let record = doc.measurement_records(0).unwrap().remove(0);
    let target = doc.measurement_target(0, &record).unwrap();
    let mut invalid = measurement();
    invalid.points[0].0 = -1.0;
    assert!(doc
        .update_measurement_annotation(&target, &invalid)
        .is_err());
    assert_unchanged(&baseline, &doc.snapshot().unwrap(), None);
    invalid = measurement();
    invalid.calibration[1] = invalid.calibration[0];
    assert!(doc
        .update_measurement_annotation(&target, &invalid)
        .is_err());
    assert_unchanged(&baseline, &doc.snapshot().unwrap(), None);
    let mut wrong = record.clone();
    wrong.measurement.known_distance = 20.0;
    assert!(doc.measurement_target(0, &wrong).is_err());
    doc.add_reply(0, index, "A later edit", None).unwrap();
    let changed = doc.snapshot().unwrap();
    assert!(doc
        .update_measurement_annotation(&target, &measurement())
        .is_err());
    assert_unchanged(&changed, &doc.snapshot().unwrap(), None);
    doc.restore(baseline.clone()).unwrap();
    let preview = doc
        .preview_rich_annotation(
            0,
            &RichAnnotation::UpdateMeasurement {
                target,
                measurement: wrong.measurement,
            },
            None,
        )
        .unwrap();
    doc.add_reply(0, index, "After preview", None).unwrap();
    let changed = doc.snapshot().unwrap();
    assert!(doc
        .commit_rich_annotation_preview(preview.prepared.unwrap())
        .is_err());
    assert_unchanged(&changed, &doc.snapshot().unwrap(), None);
    for defect in [
        "readonly",
        "locked",
        "locked-contents",
        "flags",
        "future",
        "extra-field",
        "subtype",
        "missing-id",
        "duplicate",
        "duplicate-indirect",
        "shared",
        "alternate-appearance",
        "contents-pdfdoc",
        "contents-utf16",
        "contents-missing",
        "contents-malformed",
        "contents-oversized",
    ] {
        let changed = mutate(&baseline, |pdf| {
            let entries = annotations(pdf, 1);
            let id = entries[index].0.unwrap();
            if matches!(defect, "duplicate" | "duplicate-indirect" | "shared") {
                let new_entry = if defect == "shared" {
                    Object::Reference(id)
                } else {
                    let mut duplicate = entries[index].1.clone();
                    if defect == "duplicate-indirect" {
                        let identity =
                            lopdf::decode_text_string(duplicate.get(b"NM").unwrap()).unwrap();
                        let indirect = pdf.add_object(utf16(&identity));
                        duplicate.set("NM", indirect);
                    }
                    Object::Reference(pdf.add_object(duplicate))
                };
                let page = pdf.get_pages()[&2];
                pdf.get_dictionary_mut(page)
                    .unwrap()
                    .set("Annots", vec![new_entry]);
                return;
            }
            let d = pdf.get_dictionary_mut(id).unwrap();
            match defect {
                "readonly" => d.set("F", 64),
                "locked" => d.set("F", 128),
                "locked-contents" => d.set("F", 512),
                "flags" => d.set("F", Object::Null),
                "subtype" => d.set("Subtype", Object::Name(b"Square".to_vec())),
                "missing-id" => {
                    d.remove(b"NM");
                }
                "alternate-appearance" => {
                    d.set(
                        "AP",
                        dictionary! { "N" => Object::Null, "R" => Object::Null },
                    );
                }
                "contents-pdfdoc" => {
                    d.set("Contents", Object::string_literal("External review note"))
                }
                "contents-utf16" => {
                    d.set("Contents", utf16("External review note: \u{00e9}t\u{00e9}"))
                }
                "contents-missing" => {
                    d.remove(b"Contents");
                }
                "contents-malformed" => d.set("Contents", Object::Integer(7)),
                "contents-oversized" => d.set("Contents", Object::string_literal(vec![b'x'; 4097])),
                _ => {
                    let mut value: serde_json::Value = serde_json::from_slice(
                        d.get(b"SuniPDFMeasurement").unwrap().as_str().unwrap(),
                    )
                    .unwrap();
                    if defect == "future" {
                        value["version"] = 99.into();
                    } else {
                        value["future-field"] = true.into();
                    }
                    d.set(
                        "SuniPDFMeasurement",
                        Object::string_literal(value.to_string()),
                    );
                }
            }
        });
        let changed = engine
            .open_bytes(changed, None, Path::new("unsupported.pdf"))
            .unwrap();
        let before = changed.snapshot().unwrap();
        if let Ok(records) = changed.measurement_records(0) {
            assert!(
                changed.measurement_target(0, &records[0]).is_err(),
                "{defect}"
            );
        }
        assert_unchanged(&before, &changed.snapshot().unwrap(), None);
    }
}

#[test]
fn equivalent_pdf_text_encodings_and_indirect_identities_remain_editable_without_losing_replies() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("plan.pdf"))
        .unwrap();
    let index = doc
        .add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    doc.add_reply(0, index, "A separate review comment", None)
        .unwrap();
    let baseline = doc.snapshot().unwrap();
    let original = doc.measurement_records(0).unwrap().remove(0);
    for unicode in [false, true] {
        for indirect in [false, true] {
            let bytes = mutate(&baseline, |pdf| {
                let id = annotations(pdf, 1)[index].0.unwrap();
                let encode = |text: &str| {
                    if unicode {
                        utf16(text)
                    } else {
                        Object::string_literal(text)
                    }
                };
                let mut contents = encode("10.00 m");
                let mut identity = encode(original.annotation_id.as_deref().unwrap());
                if indirect {
                    contents = pdf.add_object(contents).into();
                    identity = pdf.add_object(identity).into();
                }
                let dictionary = pdf.get_dictionary_mut(id).unwrap();
                dictionary.set("Contents", contents);
                dictionary.set("NM", identity);
            });
            doc.restore(bytes).unwrap();
            let before = doc.snapshot().unwrap();
            let before_annotations = annotations(&parse(&before, None), 1);
            let record = doc.measurement_records(0).unwrap().remove(0);
            assert_eq!(record, original, "the same PDF text has the same identity");
            let target = doc.measurement_target(0, &record).unwrap();
            assert_unchanged(&before, &doc.snapshot().unwrap(), None);
            let mut replacement = measurement();
            replacement.known_distance = 12.0;
            doc.update_measurement_annotation(&target, &replacement)
                .unwrap();
            let after_annotations = annotations(&parse(&doc.snapshot().unwrap(), None), 1);
            assert_eq!(
                before_annotations[1], after_annotations[1],
                "reply preserved"
            );
            assert_eq!(
                before_annotations[index].1.get(b"NM").unwrap(),
                after_annotations[index].1.get(b"NM").unwrap()
            );
            assert_eq!(
                doc.measurement_records(0).unwrap()[0].annotation_id,
                original.annotation_id
            );
            assert_eq!(
                doc.measurement_records(0).unwrap()[0].measurement,
                replacement
            );
        }
    }
}

#[test]
fn replacing_measurements_preserves_aes_and_requires_annotation_permission_without_copy_or_modify()
{
    let engine = engine();
    let mut source = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("plan.pdf"))
        .unwrap();
    source
        .add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    let source = source.snapshot().unwrap();
    for aes256 in [false, true] {
        for allow_annotate in [false, true] {
            let user = credential();
            let owner = credential();
            let protected = security::protect(
                &source,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_copy: false,
                    allow_modify: false,
                    allow_annotate,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut doc = engine
                .open_bytes(protected, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            assert!(!doc.properties().can_copy && !doc.properties().can_modify);
            let record = doc.measurement_records(0).unwrap().remove(0);
            let before = doc.snapshot().unwrap();
            if allow_annotate {
                let target = doc.measurement_target(0, &record).unwrap();
                let mut replacement = measurement();
                replacement.known_distance = 15.0;
                let preview = doc
                    .preview_rich_annotation(
                        0,
                        &RichAnnotation::UpdateMeasurement {
                            target,
                            measurement: replacement.clone(),
                        },
                        None,
                    )
                    .unwrap();
                assert_unchanged(&before, &doc.snapshot().unwrap(), Some(&user));
                doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                    .unwrap();
                let bytes = doc.snapshot().unwrap();
                assert!(engine
                    .open_bytes(bytes.clone(), None, Path::new("saved.pdf"))
                    .is_err());
                for password in [&user, &owner] {
                    let reopened = engine
                        .open_bytes(bytes.clone(), Some(password), Path::new("saved.pdf"))
                        .unwrap();
                    assert!(reopened.properties().encrypted);
                    assert_eq!(
                        reopened.measurement_records(0).unwrap()[0].measurement,
                        replacement
                    );
                    assert_eq!(
                        reopened.measurement_records(0).unwrap()[0].annotation_id,
                        record.annotation_id
                    );
                }
            } else {
                assert!(matches!(
                    doc.measurement_target(0, &record),
                    Err(EngineError::SecurityRestricted)
                ));
                assert_unchanged(&before, &doc.snapshot().unwrap(), Some(&user));
            }
        }
    }
}

#[test]
fn resize_refuses_measurements_atomically_and_edit_detaches_shared_appearances() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("plan.pdf"))
        .unwrap();
    doc.add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    doc.add_measurement_annotation(1, &measurement(), None)
        .unwrap();
    let baseline = doc.snapshot().unwrap();
    for imported in [false, true] {
        let bytes = if imported {
            mutate(&baseline, |pdf| {
                let id = annotations(pdf, 1)[0].0.unwrap();
                let d = pdf.get_dictionary_mut(id).unwrap();
                d.remove(b"SuniPDFMeasurement");
                d.set("Measure", Dictionary::new());
            })
        } else {
            baseline.clone()
        };
        doc.restore(bytes).unwrap();
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let before = doc.snapshot().unwrap();
        assert!(doc
            .resize_page(0, 350.0, 450.0, ResizeMode::ScaleContentWithAnnotations)
            .is_err());
        assert_unchanged(&before, &doc.snapshot().unwrap(), None);
        assert_eq!(pixels, doc.render_page(0, 1.0).unwrap().rgba);
    }
    let bytes = mutate(&baseline, |pdf| {
        let first = annotations(pdf, 1)[0].1.get(b"AP").unwrap().clone();
        let second = annotations(pdf, 2)[0].0.unwrap();
        pdf.get_dictionary_mut(second).unwrap().set("AP", first);
    });
    doc.restore(bytes).unwrap();
    let second_pixels = doc.render_page(1, 1.0).unwrap().rgba;
    let before = parse(&doc.snapshot().unwrap(), None);
    let second = annotations(&before, 2);
    let record = doc.measurement_records(0).unwrap().remove(0);
    let target = doc.measurement_target(0, &record).unwrap();
    let mut replacement = measurement();
    replacement.known_distance = 25.0;
    doc.update_measurement_annotation(&target, &replacement)
        .unwrap();
    assert_eq!(doc.render_page(1, 1.0).unwrap().rgba, second_pixels);
    assert_eq!(
        annotations(&parse(&doc.snapshot().unwrap(), None), 2),
        second
    );
    assert_eq!(
        doc.measurement_records(1).unwrap()[0].measurement,
        measurement()
    );
}
