use std::path::Path;
use std::sync::{Mutex, OnceLock};
use sunipdf_core::{
    form_data, security, Engine, FieldKind, FieldOption, FieldValue, FillMode, PointRect,
    Protection,
};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

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

fn options() -> Vec<FieldOption> {
    vec![
        FieldOption {
            value: "FR".into(),
            label: "Fran\u{e7}ais".into(),
        },
        FieldOption {
            value: "BE".into(),
            label: "Belgique".into(),
        },
    ]
}

#[test]
fn choice_fields_fill_reopen_and_interchange_on_cropped_rotated_encrypted_pages() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    for rotation in [0, 90, 180, 270] {
        for encrypted in [false, true] {
            let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
            let page = pdf.get_pages()[&1];
            let dictionary = pdf.get_dictionary_mut(page).unwrap();
            dictionary.set("Rotate", rotation);
            dictionary.set(
                "CropBox",
                vec![20.into(), 30.into(), 400.into(), 600.into()],
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
            let password = encrypted.then_some("user");
            let mut doc = engine
                .open_bytes(bytes, password, Path::new("choices.pdf"))
                .unwrap();
            for (name, kind, rect, multi, editable) in [
                (
                    "Language",
                    FieldKind::Radio,
                    PointRect {
                        left: 40.0,
                        bottom: 50.0,
                        right: 140.0,
                        top: 110.0,
                    },
                    false,
                    false,
                ),
                (
                    "Country",
                    FieldKind::Combo,
                    PointRect {
                        left: 170.0,
                        bottom: 50.0,
                        right: 270.0,
                        top: 78.0,
                    },
                    false,
                    false,
                ),
                (
                    "OtherCountry",
                    FieldKind::Combo,
                    PointRect {
                        left: 170.0,
                        bottom: 90.0,
                        right: 270.0,
                        top: 118.0,
                    },
                    false,
                    true,
                ),
                (
                    "Countries",
                    FieldKind::List,
                    PointRect {
                        left: 40.0,
                        bottom: 150.0,
                        right: 180.0,
                        top: 210.0,
                    },
                    true,
                    false,
                ),
            ] {
                doc.add_form_field_with_flags(0, name, kind, rect, &options(), multi, editable)
                    .unwrap();
            }
            let info = doc.form_info().unwrap();
            assert_eq!(info.field("Language").unwrap().widgets.len(), 2);
            assert_eq!(info.field("Language").unwrap().options, options());
            let radio = &info.field("Language").unwrap().widgets[0];
            assert!(
                (radio.display.left - 40.0).abs() < 0.2 && (radio.display.top - 110.0).abs() < 0.2
            );
            assert!(info.field("Countries").unwrap().multi_select);
            assert!(!info.field("Country").unwrap().editable);
            assert!(info.field("OtherCountry").unwrap().editable);
            assert!(info
                .field("Country")
                .unwrap()
                .validate(&FieldValue::Choice(vec!["Suisse romande".into()]))
                .is_err());
            let values = vec![
                ("Language".into(), FieldValue::Choice(vec!["BE".into()])),
                ("Country".into(), FieldValue::Choice(vec!["FR".into()])),
                (
                    "OtherCountry".into(),
                    FieldValue::Choice(vec!["Suisse romande – été".into()]),
                ),
                (
                    "Countries".into(),
                    FieldValue::Choice(vec!["FR".into(), "BE".into()]),
                ),
            ];
            doc.fill_form(&values, FillMode::Strict).unwrap();
            let saved = doc.snapshot().unwrap();
            assert_eq!(
                lopdf::Document::load_mem(&saved).unwrap().is_encrypted(),
                encrypted
            );
            let mut doc = engine
                .open_bytes(saved, password, Path::new("reopened.pdf"))
                .unwrap();
            let info = doc.form_info().unwrap();
            for (name, value) in &values {
                assert_eq!(&info.field(name).unwrap().value, value);
            }
            assert!(info.field("OtherCountry").unwrap().editable);
            let invalid = vec![
                ("Country".into(), FieldValue::Choice(vec!["BE".into()])),
                (
                    "Language".into(),
                    FieldValue::Choice(vec!["missing".into()]),
                ),
            ];
            assert!(doc.fill_form(&invalid, FillMode::Strict).is_err());
            assert_eq!(
                doc.form_info().unwrap(),
                info,
                "the valid first value must not be applied"
            );
            let xfdf = form_data::export_values(&info, "xfdf").unwrap();
            let parsed = form_data::parse_values(&xfdf, "xfdf").unwrap();
            let plan = form_data::strict_plan_from_json(&info, &parsed).unwrap();
            doc.clear_form().unwrap();
            doc.fill_form(&plan.values, FillMode::Strict).unwrap();
            assert_eq!(
                form_data::values_to_json(&doc.form_info().unwrap()),
                form_data::values_to_json(&info)
            );
            assert!(doc.render_page(0, 1.0).is_ok());
        }
    }
}

#[test]
fn invalid_choice_creation_keeps_existing_fields() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut doc = engine
        .open_bytes(PDF.to_vec(), None, Path::new("choices.pdf"))
        .unwrap();
    let rect = PointRect {
        left: 40.0,
        bottom: 50.0,
        right: 180.0,
        top: 110.0,
    };
    doc.add_form_field(0, "Existing", FieldKind::Text, rect)
        .unwrap();
    let before = doc.form_info().unwrap();
    for (kind, choices, multi, editable) in [
        (FieldKind::Radio, Vec::new(), false, false),
        (
            FieldKind::Radio,
            vec![FieldOption {
                value: "Off".into(),
                label: "Off".into(),
            }],
            false,
            false,
        ),
        (
            FieldKind::Combo,
            vec![options()[0].clone(), options()[0].clone()],
            false,
            false,
        ),
        (FieldKind::Combo, options(), true, false),
        (FieldKind::Text, options(), false, false),
        (FieldKind::Text, Vec::new(), false, true),
        (FieldKind::Checkbox, Vec::new(), false, true),
        (FieldKind::Radio, options(), false, true),
        (FieldKind::List, options(), false, true),
    ] {
        assert!(doc
            .add_form_field_with_flags(0, "Invalid", kind, rect, &choices, multi, editable)
            .is_err());
        assert_eq!(doc.form_info().unwrap(), before);
    }
}
