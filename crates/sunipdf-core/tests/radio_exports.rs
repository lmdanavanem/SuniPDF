#[path = "fixtures/radio_exports.rs"]
mod fixture;

use lopdf::{dictionary, Object};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use sunipdf_core::{form_data, security, Engine, FieldValue, FillMode, Protection};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// Every access is serialized; PDFium stays initialized for this test process.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}

fn engine() -> &'static Engine {
    let ptr = ENGINE.get_or_init(|| EnginePtr(Box::leak(Box::new(Engine::new().unwrap()))));
    unsafe { &*ptr.0 }
}

fn choice(value: &str) -> FieldValue {
    FieldValue::Choice(vec![value.into()])
}

fn fdf(value: &str) -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.2");
    pdf.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    let field = pdf.add_object(dictionary! {
        "T" => lopdf::text_string(fixture::FIELD_NAME),
        "V" => lopdf::text_string(value),
    });
    let root = pdf.add_object(dictionary! {
        "FDF" => dictionary! {"Fields" => vec![Object::Reference(field)]},
    });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes[1..4].copy_from_slice(b"FDF");
    bytes
}

fn button_states(pdf: &lopdf::Document) -> (Object, Vec<Option<Object>>) {
    let dictionary = pdf.get_dictionary(fixture::field_id(pdf)).unwrap();
    let states = dictionary
        .get(b"Kids")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|kid| {
            pdf.get_dictionary(kid.as_reference().ok()?)
                .ok()?
                .get(b"AS")
                .ok()
                .cloned()
        })
        .collect();
    (dictionary.get(b"V").unwrap().clone(), states)
}

fn button_appearance_names(pdf: &lopdf::Document) -> Vec<Option<Vec<Vec<u8>>>> {
    pdf.get_dictionary(fixture::field_id(pdf))
        .unwrap()
        .get(b"Kids")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|kid| {
            let widget = pdf.get_dictionary(kid.as_reference().ok()?).ok()?;
            let ap = pdf
                .dereference(widget.get(b"AP").ok()?)
                .ok()?
                .1
                .as_dict()
                .ok()?;
            let normal = pdf.dereference(ap.get(b"N").ok()?).ok()?.1.as_dict().ok()?;
            let mut keys: Vec<_> = normal.iter().map(|(key, _)| key.clone()).collect();
            keys.sort();
            Some(keys)
        })
        .collect()
}

#[test]
fn radio_exports_keep_appearances_encryption_and_interchange_values() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for rotation in [0, 90, 180, 270] {
        for encrypted in [false, true] {
            let mut pdf = lopdf::Document::load_mem(&fixture::source(true)).unwrap();
            let page = pdf.get_pages()[&1];
            let dictionary = pdf.get_dictionary_mut(page).unwrap();
            dictionary.set("Rotate", rotation);
            dictionary.set(
                "CropBox",
                vec![20.into(), 30.into(), 580.into(), 820.into()],
            );
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            if encrypted {
                bytes = security::protect(
                    &bytes,
                    None,
                    &Protection {
                        user_password: "user".into(),
                        owner_password: "owner".into(),
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            let password = encrypted.then_some("owner");
            let mut doc = engine
                .open_bytes(bytes, password, Path::new("radio.pdf"))
                .unwrap();
            let info = doc.form_info().unwrap();
            let field = info.field(fixture::FIELD_NAME).unwrap();
            assert!(!field.read_only);
            assert_eq!(
                field
                    .options
                    .iter()
                    .map(|o| o.value.as_str())
                    .collect::<Vec<_>>(),
                fixture::EXPORTS
            );
            assert_eq!(field.value, choice(fixture::EXPORTS[1]));
            assert_eq!(
                field.widgets.len(),
                2,
                "invisible Kids still occupy their /Opt index"
            );
            assert_eq!(field.widgets[0].on_state.as_deref(), Some("virement"));
            assert_eq!(
                field.widgets[0].export_value.as_deref(),
                Some(fixture::EXPORTS[1])
            );
            for extension in ["json", "xfdf", "fdf"] {
                let exported = if extension == "fdf" {
                    fdf(fixture::EXPORTS[1])
                } else {
                    form_data::export_values(&info, extension).unwrap()
                };
                let parsed = form_data::parse_values(&exported, extension).unwrap();
                assert_eq!(parsed[fixture::FIELD_NAME], fixture::EXPORTS[1]);
                let plan = form_data::strict_plan_from_json(&info, &parsed).unwrap();
                doc.clear_form().unwrap();
                assert_eq!(
                    doc.form_info()
                        .unwrap()
                        .field(fixture::FIELD_NAME)
                        .unwrap()
                        .value,
                    FieldValue::Choice(Vec::new())
                );
                doc.fill_form(&plan.values, FillMode::Strict).unwrap();
                assert_eq!(
                    doc.form_info()
                        .unwrap()
                        .field(fixture::FIELD_NAME)
                        .unwrap()
                        .value,
                    choice(fixture::EXPORTS[1])
                );
            }
            doc.fill_form(
                &[(fixture::FIELD_NAME.into(), choice(fixture::EXPORTS[2]))],
                FillMode::Strict,
            )
            .unwrap();
            let saved = doc.snapshot().unwrap();
            assert_eq!(
                lopdf::Document::load_mem(&saved).unwrap().is_encrypted(),
                encrypted
            );
            let pdf = lopdf::Document::load_mem_with_options(
                &saved,
                lopdf::LoadOptions::with_password(password.unwrap_or("")),
            )
            .unwrap();
            let dictionary = pdf.get_dictionary(fixture::field_id(&pdf)).unwrap();
            assert_eq!(dictionary.get(b"V").unwrap().as_name().unwrap(), b"2");
            for (index, object) in dictionary
                .get(b"Kids")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let widget = pdf.get_dictionary(object.as_reference().unwrap()).unwrap();
                assert_eq!(
                    widget.get(b"AS").unwrap().as_name().unwrap(),
                    if index == 2 {
                        b"especes".as_slice()
                    } else {
                        b"Off".as_slice()
                    }
                );
                assert!(widget.has(b"AP"));
            }
            let reopened = engine
                .open_bytes(saved, password, Path::new("reopened.pdf"))
                .unwrap();
            assert_eq!(
                reopened
                    .form_info()
                    .unwrap()
                    .field(fixture::FIELD_NAME)
                    .unwrap()
                    .value,
                choice(fixture::EXPORTS[2])
            );
            assert!(reopened.render_page(0, 1.0).is_ok());
        }
    }
}

#[test]
fn malformed_or_ambiguous_radio_exports_do_not_block_other_fields_or_mutate() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for case in 0..11 {
        let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
        let field = fixture::field_id(&pdf);
        let kids = pdf
            .get_dictionary(field)
            .unwrap()
            .get(b"Kids")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        match case {
            0 => pdf
                .get_dictionary_mut(field)
                .unwrap()
                .set("Opt", lopdf::text_string("bad array")),
            1 => pdf
                .get_dictionary_mut(field)
                .unwrap()
                .set("Opt", vec![lopdf::text_string("short")]),
            2 => pdf.get_dictionary_mut(field).unwrap().set(
                "Opt",
                vec![
                    Object::Null,
                    lopdf::text_string(fixture::EXPORTS[1]),
                    lopdf::text_string("Cash"),
                ],
            ),
            3 => pdf.get_dictionary_mut(field).unwrap().set(
                "Opt",
                vec![
                    lopdf::text_string("duplicate"),
                    lopdf::text_string("duplicate"),
                    lopdf::text_string("Cash"),
                ],
            ),
            4 => {
                pdf.get_dictionary_mut(kids[0].as_reference().unwrap())
                    .unwrap()
                    .remove(b"AP");
            }
            5 => pdf
                .get_dictionary_mut(kids[0].as_reference().unwrap())
                .unwrap()
                .set("AS", Object::Name(b"carte".to_vec())),
            6 => pdf.get_dictionary_mut(field).unwrap().set(
                "Opt",
                vec![
                    lopdf::text_string("Off"),
                    lopdf::text_string(fixture::EXPORTS[1]),
                    lopdf::text_string("Cash"),
                ],
            ),
            7 => {
                pdf.objects
                    .insert(kids[0].as_reference().unwrap(), Object::Null);
            }
            8 => {
                let dictionary = pdf.get_dictionary_mut(field).unwrap();
                dictionary.set("Kids", vec![kids[0].clone(), Object::Null, kids[1].clone()]);
                dictionary.set(
                    "Opt",
                    vec![lopdf::text_string("A"), lopdf::text_string("B")],
                );
            }
            9 => {
                let dictionary = pdf.get_dictionary_mut(field).unwrap();
                dictionary.set(
                    "Kids",
                    vec![kids[0].clone(), kids[0].clone(), kids[1].clone()],
                );
                dictionary.set(
                    "Opt",
                    vec![
                        lopdf::text_string("A1"),
                        lopdf::text_string("A2"),
                        lopdf::text_string("B"),
                    ],
                );
                dictionary.set("V", Object::Name(b"2".to_vec()));
            }
            10 => {
                let widget = pdf
                    .get_dictionary_mut(kids[1].as_reference().unwrap())
                    .unwrap();
                let normal = widget
                    .get_mut(b"AP")
                    .unwrap()
                    .as_dict_mut()
                    .unwrap()
                    .get_mut(b"N")
                    .unwrap()
                    .as_dict_mut()
                    .unwrap();
                let appearance = normal.remove(b"virement").unwrap();
                normal.set(vec![0xe9], appearance);
                widget.set("AS", Object::Name(vec![0xe9]));
            }
            _ => unreachable!(),
        }
        let original_states = button_states(&pdf);
        let original_appearances = button_appearance_names(&pdf);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("ambiguous.pdf"))
            .unwrap();
        let before = doc.form_info().unwrap();
        let radio = before.field(fixture::FIELD_NAME).unwrap();
        assert!(radio.read_only, "case {case}");
        assert!(!radio.fillable());
        assert!(before.field("nom").unwrap().fillable());
        doc.fill_form(
            &[(fixture::FIELD_NAME.into(), radio.value.clone())],
            FillMode::Strict,
        )
        .unwrap();
        assert_eq!(
            doc.form_info().unwrap(),
            before,
            "unchanged round-trip is harmless"
        );
        let invalid = [
            ("nom".into(), FieldValue::Text("should not apply".into())),
            (fixture::FIELD_NAME.into(), choice("Cash")),
        ];
        assert!(doc.fill_form(&invalid, FillMode::Strict).is_err());
        assert_eq!(doc.form_info().unwrap(), before, "strict writes are atomic");
        doc.fill_form(
            &[("nom".into(), FieldValue::Text("still writable".into()))],
            FillMode::Strict,
        )
        .unwrap();
        let after = doc.form_info().unwrap();
        assert_eq!(
            after.field("nom").unwrap().value,
            FieldValue::Text("still writable".into())
        );
        let updated_radio = after.field(fixture::FIELD_NAME).unwrap();
        assert_eq!(updated_radio.value, radio.value);
        assert_eq!(updated_radio.options, radio.options);
        assert_eq!(
            button_states(&lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap()),
            original_states,
            "existing field value and widget appearance states must remain unchanged"
        );
        if case != 4 {
            assert_eq!(
                button_appearance_names(
                    &lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap()
                ),
                original_appearances
            );
        }
        if case == 4 {
            // PDFium may repair the missing appearance when rendering the changed text field.
            assert_eq!(
                updated_radio.widgets[0].export_value,
                radio.widgets[0].export_value
            );
            assert_eq!(&updated_radio.widgets[1..], &radio.widgets[1..]);
        } else {
            assert_eq!(updated_radio, radio);
        }
    }
}

#[test]
fn selected_reserved_radio_exports_round_trip_unchanged_and_refuse_changes() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for reserved in ["Off", ""] {
        let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
        pdf.get_dictionary_mut(fixture::field_id(&pdf))
            .unwrap()
            .set(
                "Opt",
                vec![
                    lopdf::text_string("Card"),
                    lopdf::text_string(reserved),
                    lopdf::text_string("Cash"),
                ],
            );
        let original_states = button_states(&pdf);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("reserved.pdf"))
            .unwrap();
        let info = doc.form_info().unwrap();
        let radio = info.field(fixture::FIELD_NAME).unwrap();
        assert!(radio.read_only);
        assert_eq!(radio.value, choice(reserved));
        for extension in ["json", "xfdf"] {
            let exported = form_data::export_values(&info, extension).unwrap();
            let parsed = form_data::parse_values(&exported, extension).unwrap();
            let plan = form_data::strict_plan_from_json(&info, &parsed).unwrap();
            assert!(!plan
                .values
                .iter()
                .any(|(name, _)| name == fixture::FIELD_NAME));
            doc.fill_form(&plan.values, FillMode::Strict).unwrap();
            assert_eq!(doc.form_info().unwrap(), info);
            assert_eq!(
                button_states(&lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap()),
                original_states
            );
        }
        assert!(doc
            .fill_form(
                &[(fixture::FIELD_NAME.into(), choice("Cash"))],
                FillMode::Strict
            )
            .is_err());
        assert_eq!(doc.form_info().unwrap(), info);
    }
}

#[test]
fn radio_exports_can_read_index_or_unique_appearance_without_widget_state() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for value in [b"1".as_slice(), b"virement".as_slice()] {
        let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
        let field = fixture::field_id(&pdf);
        let kids = pdf
            .get_dictionary(field)
            .unwrap()
            .get(b"Kids")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        pdf.get_dictionary_mut(field)
            .unwrap()
            .set("V", Object::Name(value.to_vec()));
        for kid in kids {
            pdf.get_dictionary_mut(kid.as_reference().unwrap())
                .unwrap()
                .remove(b"AS");
        }
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let doc = engine
            .open_bytes(bytes, None, Path::new("fallback.pdf"))
            .unwrap();
        assert_eq!(
            doc.form_info()
                .unwrap()
                .field(fixture::FIELD_NAME)
                .unwrap()
                .value,
            choice(fixture::EXPORTS[1])
        );
    }
}
