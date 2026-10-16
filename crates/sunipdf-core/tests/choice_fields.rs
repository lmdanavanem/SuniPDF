use std::path::Path;
use sunipdf_core::{
    form_data, security, Engine, FieldKind, FieldOption, FieldValue, FillMode, PointRect,
    Protection,
};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

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
    let engine = Engine::new().unwrap();
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
            for (name, kind, rect, multi) in [
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
                ),
            ] {
                doc.add_form_field_with_options(0, name, kind, rect, &options(), multi)
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
            let values = vec![
                ("Language".into(), FieldValue::Choice(vec!["BE".into()])),
                ("Country".into(), FieldValue::Choice(vec!["FR".into()])),
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
    let engine = Engine::new().unwrap();
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
    for (kind, choices, multi) in [
        (FieldKind::Radio, Vec::new(), false),
        (
            FieldKind::Radio,
            vec![FieldOption {
                value: "Off".into(),
                label: "Off".into(),
            }],
            false,
        ),
        (
            FieldKind::Combo,
            vec![options()[0].clone(), options()[0].clone()],
            false,
        ),
        (FieldKind::Combo, options(), true),
        (FieldKind::Text, options(), false),
    ] {
        assert!(doc
            .add_form_field_with_options(0, "Invalid", kind, rect, &choices, multi)
            .is_err());
        assert_eq!(doc.form_info().unwrap(), before);
    }
}
