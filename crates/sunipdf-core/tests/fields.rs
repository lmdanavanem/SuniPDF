use sunipdf_core::{security, Engine, FieldKind, FieldValue, FillMode, PointRect, Protection};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

fn assert_same_pdf_contents(before: &[u8], after: &[u8]) {
    let mut before = lopdf::Document::load_mem(before).unwrap();
    let mut after = lopdf::Document::load_mem(after).unwrap();
    // PDFium may regenerate the trailer ID when serializing an unchanged document.
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(after.objects, before.objects);
    assert_eq!(after.trailer, before.trailer);
}

#[test]
fn create_fill_reopen_and_validate_fields_on_rotated_encrypted_and_existing_forms() {
    let engine = Engine::new().unwrap();
    let rect = PointRect {
        left: 40.0,
        bottom: 50.0,
        right: 200.0,
        top: 76.0,
    };
    for rotation in [0, 90, 180, 270] {
        for encrypted in [false, true] {
            let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
            let page = pdf.get_pages()[&1];
            pdf.get_dictionary_mut(page)
                .unwrap()
                .set("Rotate", rotation);
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
                .open_bytes(bytes, password, std::path::Path::new("fields.pdf"))
                .unwrap();
            doc.add_form_field(0, "Prénom 日本", FieldKind::Text, rect)
                .unwrap();
            let checkbox = PointRect {
                left: 210.0,
                right: 230.0,
                ..rect
            };
            doc.add_form_field(0, "Approved", FieldKind::Checkbox, checkbox)
                .unwrap();
            assert_eq!(doc.form_info().unwrap().fields.len(), 2);
            doc.fill_form(
                &[
                    ("Prénom 日本".into(), FieldValue::Text("Émilie".into())),
                    ("Approved".into(), FieldValue::Bool(true)),
                ],
                FillMode::Strict,
            )
            .unwrap();
            let bytes = doc.snapshot().unwrap();
            assert_eq!(
                lopdf::Document::load_mem(&bytes).unwrap().is_encrypted(),
                encrypted
            );
            let reopened = engine
                .open_bytes(bytes, password, std::path::Path::new("reopened.pdf"))
                .unwrap();
            let info = reopened.form_info().unwrap();
            assert_eq!(
                info.field("Prénom 日本").unwrap().value,
                FieldValue::Text("Émilie".into())
            );
            assert_eq!(
                info.field("Approved").unwrap().value,
                FieldValue::Bool(true)
            );
            let actual = info.field("Prénom 日本").unwrap().widgets[0].display;
            assert!(
                (actual.left - rect.left).abs() < 0.2 && (actual.top - rect.top).abs() < 0.2,
                "rotation {rotation}: {actual:?}"
            );
            assert!(reopened.render_page(0, 1.0).is_ok());
        }
    }
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/acroform.pdf").to_vec(),
            None,
            std::path::Path::new("existing.pdf"),
        )
        .unwrap();
    let before = doc.form_info().unwrap();
    let links = doc.page_links(0).unwrap();
    doc.add_form_field(0, "Added", FieldKind::Text, rect)
        .unwrap();
    let after = doc.form_info().unwrap();
    assert_eq!(doc.page_links(0).unwrap(), links);
    assert_eq!(after.fields.len(), before.fields.len() + 1);
    for field in before.fields {
        assert_eq!(after.field(&field.name).unwrap().value, field.value);
    }
    let snapshot = doc.snapshot().unwrap();
    for name in ["", " ", "a.b", "a\nb", "Added"] {
        assert!(doc.add_form_field(0, name, FieldKind::Text, rect).is_err());
        assert_same_pdf_contents(&snapshot, &doc.snapshot().unwrap());
    }
    for invalid in [
        PointRect {
            left: f32::NAN,
            ..rect
        },
        PointRect {
            right: f32::INFINITY,
            ..rect
        },
        PointRect {
            right: 42.0,
            ..rect
        },
        PointRect { left: -1.0, ..rect },
    ] {
        assert!(doc
            .add_form_field(0, "Invalid", FieldKind::Text, invalid)
            .is_err());
        assert_same_pdf_contents(&snapshot, &doc.snapshot().unwrap());
    }
    assert!(doc
        .add_form_field(0, "Radio", FieldKind::Radio, rect)
        .is_err());
    assert_same_pdf_contents(&snapshot, &doc.snapshot().unwrap());
}
