#[path = "fixtures/radio_exports.rs"]
mod fixture;

use lopdf::{dictionary, Object};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use sunipdf_core::{form_data, security, Engine, FieldValue, FillMode, Protection};
#[cfg(windows)]
#[path = "support/certificate_identity.rs"]
mod test_identity;

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

fn assert_snapshot_unchanged(before: &[u8], after: &[u8], password: Option<&str>) {
    let load = |bytes: &[u8]| {
        lopdf::Document::load_mem_with_options(
            bytes,
            lopdf::LoadOptions::with_password(password.unwrap_or("")),
        )
        .unwrap()
    };
    let before = load(before);
    let after = load(after);
    // AES output can use fresh IVs on each serialization. Compare every decoded object,
    // including signature contents/ranges, field values, permissions and appearances.
    assert_eq!(after.objects, before.objects);
    assert_eq!(
        after.trailer.get(b"Root").unwrap(),
        before.trailer.get(b"Root").unwrap()
    );
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

fn set_need_appearances(pdf: &mut lopdf::Document) {
    let acro = pdf.catalog().unwrap().get(b"AcroForm").unwrap().clone();
    let mut acro = pdf.dereference(&acro).unwrap().1.as_dict().unwrap().clone();
    acro.set("NeedAppearances", true);
    let id = pdf.add_object(acro);
    pdf.catalog_mut().unwrap().set("AcroForm", id);
}

fn typed_fdf(fields: Vec<lopdf::Dictionary>) -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.2");
    pdf.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    let fields: Vec<_> = fields
        .into_iter()
        .map(|field| Object::Reference(pdf.add_object(field)))
        .collect();
    let root = pdf.add_object(dictionary! {
        "FDF" => dictionary! {"Fields" => fields},
    });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes[1..4].copy_from_slice(b"FDF");
    bytes
}

#[test]
fn need_appearances_fdf_round_trips_keep_radio_states_and_empty_choices() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for encrypted in [false, true] {
        let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
        set_need_appearances(&mut pdf);
        let country = pdf
            .objects
            .iter()
            .find_map(|(&id, object)| {
                let field = object.as_dict().ok()?;
                (lopdf::decode_text_string(field.get(b"T").ok()?).ok()? == "pays").then_some(id)
            })
            .unwrap();
        let field = pdf.get_dictionary_mut(country).unwrap();
        field.set("Opt", vec![lopdf::text_string(""), lopdf::text_string("A")]);
        field.set("V", lopdf::text_string("A"));
        let appearances = button_appearance_names(&pdf);
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
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join("refresh.pdf");
        std::fs::write(&source_path, &bytes).unwrap();
        let mut doc = if encrypted {
            engine
                .open_bytes(bytes.clone(), password, &source_path)
                .unwrap()
        } else {
            engine.open(&source_path, password).unwrap()
        };
        assert!(
            !doc.form_info()
                .unwrap()
                .field(fixture::FIELD_NAME)
                .unwrap()
                .read_only,
            "the original radio group is unambiguous"
        );
        let before_render = doc.snapshot().unwrap();
        let started = std::time::Instant::now();
        for _ in 0..3 {
            doc.render_page(0, 0.25).unwrap();
            doc.render_region(
                0,
                0.25,
                sunipdf_core::PixelRect {
                    x: 0,
                    y: 0,
                    width: 64,
                    height: 64,
                },
            )
            .unwrap();
        }
        eprintln!(
            "form render copy: bytes={}, encrypted={encrypted}, six renders={:?}",
            bytes.len(),
            started.elapsed()
        );
        assert_snapshot_unchanged(&before_render, &doc.snapshot().unwrap(), password);
        let info = doc.form_info().unwrap();
        assert!(
            !info.field(fixture::FIELD_NAME).unwrap().read_only,
            "rendering before the first import must preserve editable radio states"
        );
        let readonly = info.field("ref").unwrap().clone();
        let mut wanted = info.clone();
        wanted.fields.retain(|field| {
            matches!(field.name.as_str(), "nom" | "pays") || field.name == fixture::FIELD_NAME
        });
        for field in &mut wanted.fields {
            field.value = match field.name.as_str() {
                "nom" => FieldValue::Text("Élodie 雪 — première valeur".into()),
                "pays" => choice(""),
                _ => choice(fixture::EXPORTS[1]),
            };
        }
        let first = form_data::export_values(&wanted, "fdf").unwrap();
        let clear = typed_fdf(vec![
            dictionary! {"T" => lopdf::text_string(fixture::FIELD_NAME), "V" => Object::Name(b"Off".to_vec())},
            dictionary! {"T" => lopdf::text_string("pays"), "V" => Object::Null},
        ]);
        // Repeat the original import after clearing, reopening and rendering between every
        // operation. Names in FDF must keep mapping to /Opt exports after serialization.
        for (step, values) in [&first, &clear, &first].into_iter().enumerate() {
            let parsed = form_data::parse_import_values(values, "fdf").unwrap();
            let plan =
                form_data::strict_plan_from_json(&doc.form_info().unwrap(), &parsed).unwrap();
            doc.fill_form(&plan.values, FillMode::Strict).unwrap();
            let info = doc.form_info().unwrap();
            assert_eq!(info.field("ref").unwrap(), &readonly);
            assert_eq!(
                info.field("nom").unwrap().value,
                FieldValue::Text("Élodie 雪 — première valeur".into())
            );
            let radio = info.field(fixture::FIELD_NAME).unwrap();
            assert!(!radio.read_only, "step {step}, encrypted={encrypted}");
            assert_eq!(
                radio.value,
                if step == 1 {
                    FieldValue::Choice(Vec::new())
                } else {
                    choice(fixture::EXPORTS[1])
                }
            );
            assert_eq!(
                info.field("pays").unwrap().value,
                if step == 1 {
                    FieldValue::Choice(Vec::new())
                } else {
                    choice("")
                }
            );
            let exported = form_data::export_values(&info, "fdf").unwrap();
            let exported = form_data::parse_import_values(&exported, "fdf").unwrap();
            assert!(form_data::strict_plan_from_json(&info, &exported).is_ok());
            if step == 0 {
                let started = std::time::Instant::now();
                for _ in 0..3 {
                    doc.render_page(0, 0.25).unwrap();
                    doc.render_region(
                        0,
                        0.25,
                        sunipdf_core::PixelRect {
                            x: 0,
                            y: 0,
                            width: 64,
                            height: 64,
                        },
                    )
                    .unwrap();
                }
                eprintln!(
                    "prepared form: encrypted={encrypted}, six renders={:?}",
                    started.elapsed()
                );
            }
            let saved = doc.snapshot().unwrap();
            assert_eq!(
                lopdf::Document::load_mem(&saved).unwrap().is_encrypted(),
                encrypted
            );
            let saved_pdf = lopdf::Document::load_mem_with_options(
                &saved,
                lopdf::LoadOptions::with_password(password.unwrap_or("")),
            )
            .unwrap();
            assert_eq!(button_appearance_names(&saved_pdf), appearances);
            let acro = saved_pdf.catalog().unwrap().get(b"AcroForm").unwrap();
            let acro = saved_pdf.dereference(acro).unwrap().1.as_dict().unwrap();
            assert!(!acro.get(b"NeedAppearances").unwrap().as_bool().unwrap());
            doc = engine
                .open_bytes(saved, password, Path::new("reopened.pdf"))
                .unwrap();
            doc.render_page(0, 0.25).unwrap();
            assert!(
                !doc.form_info()
                    .unwrap()
                    .field(fixture::FIELD_NAME)
                    .unwrap()
                    .read_only
            );
        }
        // A restore may replace a cached ordinary document with one requesting a global
        // refresh. Its diagnostic must be recomputed even if PDFium reuses a handle.
        for _ in 0..3 {
            doc.restore(bytes.clone()).unwrap();
            let original = doc.snapshot().unwrap();
            doc.render_page(0, 0.25).unwrap();
            assert_snapshot_unchanged(&original, &doc.snapshot().unwrap(), password);
            assert!(
                !doc.form_info()
                    .unwrap()
                    .field(fixture::FIELD_NAME)
                    .unwrap()
                    .read_only
            );
            doc.fill_form(
                &[(fixture::FIELD_NAME.into(), choice(fixture::EXPORTS[2]))],
                FillMode::Strict,
            )
            .unwrap();
        }
        assert_eq!(std::fs::read(&source_path).unwrap(), bytes);
    }
}

#[test]
fn need_appearances_rendering_preserves_restricted_document_and_password() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
    set_need_appearances(&mut pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let bytes = security::protect(
        &bytes,
        None,
        &Protection {
            user_password: "viewer".into(),
            owner_password: "owner".into(),
            allow_copy: false,
            allow_modify: false,
            allow_annotate: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut doc = engine
        .open_bytes(bytes.clone(), Some("viewer"), Path::new("restricted.pdf"))
        .unwrap();
    let before = doc.snapshot().unwrap();
    let info = doc.form_info().unwrap();
    doc.render_page(0, 0.25).unwrap();
    doc.render_region(
        0,
        0.25,
        sunipdf_core::PixelRect {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        },
    )
    .unwrap();
    assert_snapshot_unchanged(&before, &doc.snapshot().unwrap(), Some("viewer"));
    assert_eq!(doc.form_info().unwrap(), info);
    assert_eq!(doc.password(), Some("viewer"));
    assert!(doc
        .restore_with_password(bytes, Some("incorrect".into()))
        .is_err());
    assert_snapshot_unchanged(&before, &doc.snapshot().unwrap(), Some("viewer"));
    assert_eq!(doc.password(), Some("viewer"));
}

#[cfg(windows)]
#[test]
fn need_appearances_rendering_preserves_signed_source_and_signature_dictionary() {
    use sunipdf_core::certificate::{
        sign_pdf, verify_signatures, CertSignOptions, SignatureStatus,
    };
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
    set_need_appearances(&mut pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let original_signature_fields = engine
        .open_bytes(bytes.clone(), None, Path::new("unsigned.pdf"))
        .unwrap()
        .signature_count();
    let signed = sign_pdf(
        &bytes,
        &test_identity::identity(),
        "test-only",
        &CertSignOptions::default(),
    )
    .unwrap();
    assert_eq!(
        verify_signatures(&signed, None).unwrap()[0].status,
        SignatureStatus::Valid
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("signed.pdf");
    std::fs::write(&path, &signed).unwrap();
    let doc = engine.open(&path, None).unwrap();
    assert_eq!(doc.signature_count(), original_signature_fields + 1);
    let before = doc.snapshot().unwrap();
    let info = doc.form_info().unwrap();
    doc.render_page(0, 0.25).unwrap();
    doc.render_region(
        0,
        0.25,
        sunipdf_core::PixelRect {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
        },
    )
    .unwrap();
    assert_snapshot_unchanged(&before, &doc.snapshot().unwrap(), None);
    assert_eq!(doc.form_info().unwrap(), info);
    assert_eq!(std::fs::read(&path).unwrap(), signed);
    assert_eq!(
        verify_signatures(&std::fs::read(&path).unwrap(), None).unwrap()[0].status,
        SignatureStatus::Valid
    );
}

#[test]
fn appearance_refresh_after_annotation_style_or_field_creation_keeps_radios_editable() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for create_field in [false, true] {
        let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
        set_need_appearances(&mut pdf);
        let appearances = button_appearance_names(&pdf);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("edit-before-import.pdf"))
            .unwrap();
        doc.render_page(0, 0.25).unwrap();
        let rect = sunipdf_core::PointRect {
            left: 30.0,
            bottom: 30.0,
            right: 140.0,
            top: 50.0,
        };
        if create_field {
            doc.add_form_field(0, "additional", sunipdf_core::FieldKind::Text, rect)
                .unwrap();
        } else {
            doc.add_markup(
                0,
                sunipdf_core::MarkupKind::Highlight,
                &[rect],
                (255, 255, 0),
            )
            .unwrap();
            let index = doc.annotations(0).unwrap().last().unwrap().index;
            doc.set_annotation_style(
                0,
                index,
                &sunipdf_core::AnnotStyle {
                    rgb: (255, 0, 0),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let info = doc.form_info().unwrap();
        assert!(!info.field(fixture::FIELD_NAME).unwrap().read_only);
        assert_eq!(
            button_appearance_names(&lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap()),
            appearances
        );
        let mut wanted = info.clone();
        wanted
            .fields
            .retain(|field| field.name == fixture::FIELD_NAME);
        wanted.fields[0].value = choice(fixture::EXPORTS[2]);
        let exported = form_data::export_values(&wanted, "fdf").unwrap();
        let parsed = form_data::parse_import_values(&exported, "fdf").unwrap();
        let plan = form_data::strict_plan_from_json(&info, &parsed).unwrap();
        doc.fill_form(&plan.values, FillMode::Strict).unwrap();
        assert_eq!(
            doc.form_info()
                .unwrap()
                .field(fixture::FIELD_NAME)
                .unwrap()
                .value,
            choice(fixture::EXPORTS[2])
        );
    }
}

#[test]
fn need_appearances_refreshes_unedited_text_on_another_page() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
    set_need_appearances(&mut pdf);
    let text_field = pdf
        .objects
        .iter()
        .find_map(|(&id, object)| {
            let field = object.as_dict().ok()?;
            (lopdf::decode_text_string(field.get(b"T").ok()?).ok()? == "nom").then_some(id)
        })
        .unwrap();
    let first_page = pdf.get_pages()[&1];
    let mut second = pdf.get_dictionary(first_page).unwrap().clone();
    let parent = second.get(b"Parent").unwrap().as_reference().unwrap();
    second.set("Annots", vec![Object::Reference(text_field)]);
    second.remove(b"Contents");
    let second_page = pdf.add_object(second);
    let tree = pdf.get_dictionary_mut(parent).unwrap();
    tree.get_mut(b"Kids")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(Object::Reference(second_page));
    tree.set("Count", 2);
    pdf.get_dictionary_mut(first_page)
        .unwrap()
        .get_mut(b"Annots")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .retain(|object| object.as_reference().ok() != Some(text_field));
    let field = pdf.get_dictionary_mut(text_field).unwrap();
    field.set("P", second_page);
    field.set("Ff", 1);
    field.set("V", lopdf::text_string("REFRESHED VALUE"));
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let mut doc = engine
        .open_bytes(bytes, None, Path::new("refresh-other-page.pdf"))
        .unwrap();
    let untouched = doc.form_info().unwrap().field("nom").unwrap().clone();
    assert!(untouched.read_only);
    assert_eq!(untouched.widgets[0].page, 1);
    doc.fill_form(
        &[(fixture::FIELD_NAME.into(), choice(fixture::EXPORTS[0]))],
        FillMode::Strict,
    )
    .unwrap();
    assert_eq!(doc.form_info().unwrap().field("nom").unwrap(), &untouched);
    let pdf = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
    let (_, field) = pdf
        .objects
        .iter()
        .find(|(_, object)| {
            object
                .as_dict()
                .ok()
                .and_then(|field| field.get(b"T").ok())
                .and_then(|name| lopdf::decode_text_string(name).ok())
                .as_deref()
                == Some("nom")
        })
        .unwrap();
    let ap = field.as_dict().unwrap().get(b"AP").unwrap();
    let ap = pdf.dereference(ap).unwrap().1.as_dict().unwrap();
    let normal = pdf
        .dereference(ap.get(b"N").unwrap())
        .unwrap()
        .1
        .as_stream()
        .unwrap();
    assert!(normal.dict.has(b"BBox"));
    assert!(!normal
        .decompressed_content()
        .unwrap_or_else(|_| normal.content.clone())
        .is_empty());
}

#[test]
fn need_appearances_does_not_make_ambiguous_buttons_editable() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut pdf = lopdf::Document::load_mem(&fixture::source(false)).unwrap();
    set_need_appearances(&mut pdf);
    let widget = pdf
        .get_dictionary(fixture::field_id(&pdf))
        .unwrap()
        .get(b"Kids")
        .unwrap()
        .as_array()
        .unwrap()[1]
        .as_reference()
        .unwrap();
    let widget = pdf.get_dictionary_mut(widget).unwrap();
    let normal = widget
        .get_mut(b"AP")
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .get_mut(b"N")
        .unwrap()
        .as_dict_mut()
        .unwrap();
    normal.set("1", normal.get(b"virement").unwrap().clone());
    let appearances = button_appearance_names(&pdf);
    let states = button_states(&pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let mut doc = engine
        .open_bytes(bytes, None, Path::new("ambiguous-refresh.pdf"))
        .unwrap();
    doc.render_page(0, 0.25).unwrap();
    assert!(
        doc.form_info()
            .unwrap()
            .field(fixture::FIELD_NAME)
            .unwrap()
            .read_only
    );
    doc.fill_form(
        &[("nom".into(), FieldValue::Text("updated text".into()))],
        FillMode::Strict,
    )
    .unwrap();
    let info = doc.form_info().unwrap();
    assert!(info.field(fixture::FIELD_NAME).unwrap().read_only);
    assert!(doc
        .fill_form(
            &[(fixture::FIELD_NAME.into(), choice("Cash"))],
            FillMode::Strict
        )
        .is_err());
    assert_eq!(doc.form_info().unwrap(), info);
    let pdf = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
    assert_eq!(button_appearance_names(&pdf), appearances);
    assert_eq!(button_states(&pdf), states);
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
