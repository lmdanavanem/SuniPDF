use std::{collections::BTreeMap, path::Path};
use sunipdf_core::{
    annotation_register::{RegisterFields, RegisterFilter, RegisterSort},
    security, Engine, EngineError, PageMargins, Protection, ReviewStatus, Rotation,
};

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn count_markers_and_register_keep_identity_totals_rotation_permissions_and_stale_edit_guards() {
    let engine = Engine::new().unwrap();
    let input = include_bytes!("fixtures/two-pages.pdf");
    let mut doc = engine
        .open_bytes(input.to_vec(), None, Path::new("plan.pdf"))
        .unwrap();
    let original = doc.snapshot().unwrap();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.restore(original.clone()).unwrap();
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
        let before = doc.snapshot().unwrap();
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let index = doc
            .add_count_marker(
                0,
                (80.0, 90.0),
                18.0,
                "Lights été 中文",
                Some("Site reviewer"),
            )
            .unwrap();
        assert_ne!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
        doc.add_count_marker(1, (60.0, 70.0), 24.0, "Lights été 中文", None)
            .unwrap();
        let report = doc.annotation_register().unwrap();
        let filter = RegisterFilter {
            count_group: "Lights été 中文".into(),
            ..Default::default()
        };
        let json = report.json(&filter, RegisterSort::Page, false);
        assert_eq!(json["counts"][0]["quantity"], 2);
        assert_eq!(json["counts"][0]["markers"].as_array().unwrap().len(), 2);
        let row = report
            .rows
            .iter()
            .find(|r| r.page == 1 && r.index == index)
            .unwrap();
        assert_eq!(row.rect, Some([71.0, 81.0, 89.0, 99.0]));
        assert_eq!(row.author.as_deref(), Some("Site reviewer"));
        let id = row.annotation_id.clone();
        let target = report.target(row);
        let fields = RegisterFields {
            business_status: Some("Ready for site".into()),
            discipline: Some("Electrical".into()),
            custom: BTreeMap::from([
                ("Room".into(), "A-101".into()),
                ("Formula-like text".into(), "=2+3".into()),
            ]),
        };
        doc.set_annotation_register_fields(&target, &fields, Some("Sockets"))
            .unwrap();
        assert!(
            doc.set_annotation_register_fields(&target, &fields, None)
                .is_err(),
            "stale index cannot be reused"
        );
        let report = doc.annotation_register().unwrap();
        let row = report
            .rows
            .iter()
            .find(|r| r.page == 1 && r.index == index)
            .unwrap();
        assert_eq!(row.annotation_id, id);
        assert_eq!(row.fields, fields);
        assert_eq!(row.count_group.as_deref(), Some("Sockets"));
        assert!(
            row.review_status.is_none(),
            "business statuses do not become PDF review states"
        );
        let found = report.select(
            &RegisterFilter {
                query: "a-101".into(),
                ..Default::default()
            },
            RegisterSort::Page,
            false,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].annotation_id, id);
        doc.set_review_status(0, index, ReviewStatus::Accepted, Some("Inspector"))
            .unwrap();
        let report = doc.annotation_register().unwrap();
        let row = report
            .rows
            .iter()
            .find(|r| r.page == 1 && r.index == index)
            .unwrap();
        assert_eq!(row.review_status.as_deref(), Some("Accepted"));
        assert_eq!(
            row.fields.business_status.as_deref(),
            Some("Ready for site")
        );
        assert_eq!(
            report
                .select(&RegisterFilter::default(), RegisterSort::Page, false)
                .len(),
            2
        );
        assert_eq!(
            report
                .select(
                    &RegisterFilter {
                        include_replies: true,
                        ..Default::default()
                    },
                    RegisterSort::Page,
                    false
                )
                .len(),
            3
        );
        assert!(doc
            .set_annotation_contents(0, index, "invented count")
            .is_err());
        doc.move_annotation(0, index, 12.0, 14.0).unwrap();
        let moved = doc.annotation_register().unwrap();
        let moved = moved
            .rows
            .iter()
            .find(|r| r.page == 1 && r.index == index)
            .unwrap();
        assert_eq!(moved.annotation_id, id);
        assert_eq!(moved.rect, Some([83.0, 95.0, 101.0, 113.0]));
        let snapshot = doc.snapshot().unwrap();
        doc.restore(snapshot.clone()).unwrap();
        assert_eq!(
            doc.annotation_register()
                .unwrap()
                .rows
                .iter()
                .find(|r| r.page == 1 && r.index == index)
                .unwrap()
                .count_group
                .as_deref(),
            Some("Sockets")
        );
        assert_eq!(doc.delete_annotation_thread(0, index).unwrap(), 2);
        assert_eq!(
            doc.annotation_register().unwrap().json(
                &RegisterFilter::default(),
                RegisterSort::Page,
                false
            )["counts"][0]["quantity"],
            1
        );
        doc.restore(snapshot).unwrap();
        assert_eq!(
            doc.annotation_register().unwrap().json(
                &RegisterFilter::default(),
                RegisterSort::Page,
                false
            )["counts"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        doc.restore(before).unwrap();
        assert!(
            doc.annotation_register().unwrap().rows.is_empty(),
            "Undo snapshot includes count metadata"
        );
        for (point, size, group) in [
            ((f32::NAN, 80.0), 18.0, "Lights"),
            ((1.0, 1.0), 18.0, "Lights"),
            ((80.0, 80.0), f32::INFINITY, "Lights"),
            ((80.0, 80.0), 7.0, "Lights"),
            ((80.0, 80.0), 18.0, " "),
            ((80.0, 80.0), 18.0, "A\nB"),
        ] {
            assert!(doc.add_count_marker(0, point, size, group, None).is_err());
        }
        assert!(doc.annotation_register().unwrap().rows.is_empty());
    }
    for aes256 in [false, true] {
        let user = credential();
        let owner = credential();
        for allow_annotate in [false, true] {
            let bytes = security::protect(
                input,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_modify: false,
                    allow_annotate,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut encrypted = engine
                .open_bytes(bytes, None, Path::new("protected.pdf"))
                .err();
            assert!(matches!(
                encrypted.take(),
                Some(EngineError::PasswordRequired)
            ));
            let bytes = security::protect(
                input,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_modify: false,
                    allow_annotate,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut encrypted = engine
                .open_bytes(bytes, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            let result = encrypted.add_count_marker(0, (80.0, 90.0), 18.0, "Lights", None);
            if !allow_annotate {
                assert!(matches!(result, Err(EngineError::SecurityRestricted)));
                continue;
            }
            let index = result.unwrap();
            let report = encrypted.annotation_register().unwrap();
            let row = report
                .rows
                .iter()
                .find(|r| r.page == 1 && r.index == index)
                .unwrap();
            encrypted
                .set_annotation_register_fields(
                    &report.target(row),
                    &RegisterFields::default(),
                    Some("Sockets"),
                )
                .unwrap();
            encrypted.move_annotation(0, index, 3.0, 4.0).unwrap();
            encrypted
                .set_review_status(0, index, ReviewStatus::Accepted, None)
                .unwrap();
            let bytes = encrypted.snapshot().unwrap();
            for password in [&user, &owner] {
                let reopened = engine
                    .open_bytes(bytes.clone(), Some(password), Path::new("saved.pdf"))
                    .unwrap();
                assert!(reopened.properties().encrypted);
                assert_eq!(
                    reopened.annotation_register().unwrap().rows[0]
                        .count_group
                        .as_deref(),
                    Some("Sockets")
                );
            }
        }
        let bytes = security::protect(
            input,
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner.clone(),
                aes256,
                allow_copy: false,
                ..Default::default()
            },
        )
        .unwrap();
        let encrypted = engine
            .open_bytes(bytes, Some(&user), Path::new("no-copy.pdf"))
            .unwrap();
        assert!(matches!(
            encrypted.annotation_register(),
            Err(EngineError::SecurityRestricted)
        ));
    }
    doc.restore(original).unwrap();
    doc.add_count_marker(0, (80.0, 90.0), 18.0, "Lights", None)
        .unwrap();
    let snapshot = doc.snapshot().unwrap();
    for aes256 in [false, true] {
        let user = credential();
        let owner = credential();
        let bytes = security::protect(
            &snapshot,
            None,
            &Protection {
                user_password: user.clone(),
                owner_password: owner,
                aes256,
                allow_modify: false,
                allow_annotate: false,
                ..Default::default()
            },
        )
        .unwrap();
        let mut restricted = engine
            .open_bytes(
                bytes,
                Some(&user),
                Path::new("existing-count-no-annotation.pdf"),
            )
            .unwrap();
        let before = restricted.annotation_register().unwrap();
        let index = before.rows[0].index;
        assert!(
            matches!(
                restricted.move_annotation(0, index, 10.0, 10.0),
                Err(EngineError::SecurityRestricted)
            ),
            "moving an existing count requires annotation permission"
        );
        assert!(
            matches!(
                restricted.delete_annotation_thread(0, index),
                Err(EngineError::SecurityRestricted)
            ),
            "deleting an existing count requires annotation permission"
        );
        assert!(matches!(
            restricted.add_reply(0, index, "Reply", None),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            restricted.set_review_status(0, index, ReviewStatus::Accepted, None),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            restricted.set_annotation_register_fields(
                &before.target(&before.rows[0]),
                &RegisterFields::default(),
                Some("Changed")
            ),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            restricted.delete_annotation(0, index),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            restricted.add_note(0, 80.0, 90.0, "Note"),
            Err(EngineError::SecurityRestricted)
        ));
        let after = restricted.annotation_register().unwrap();
        assert_eq!(after.rows.len(), before.rows.len());
        assert_eq!(after.rows[0].rect, before.rows[0].rect);
        assert_eq!(after.rows[0].count_group, before.rows[0].count_group);
    }
    for fault in 0..7 {
        let mut pdf = lopdf::Document::load_mem(&snapshot).unwrap();
        let page = pdf.get_pages()[&1];
        let entries = pdf
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let id = entries[0].as_reference().unwrap();
        match fault {
            0 => pdf
                .get_dictionary_mut(page)
                .unwrap()
                .set("Annots", lopdf::Object::Null),
            1 => pdf.get_dictionary_mut(id).unwrap().set(
                "SuniPDFCount",
                lopdf::Object::string_literal(br#"{"version":2,"group":"Lights"}"#.to_vec()),
            ),
            2 => pdf.get_dictionary_mut(id).unwrap().set(
                "SuniPDFRegister",
                lopdf::Object::string_literal(br#"{"version":9,"fields":{}}"#.to_vec()),
            ),
            3 => pdf.get_dictionary_mut(id).unwrap().set("IRT", id),
            4 => pdf
                .get_dictionary_mut(id)
                .unwrap()
                .set("Contents", lopdf::Object::string_literal(vec![b'A'; 16385])),
            5 => pdf.get_dictionary_mut(page).unwrap().set(
                "Annots",
                lopdf::Object::Array(vec![entries[0].clone(), entries[0].clone()]),
            ),
            _ => pdf.get_dictionary_mut(page).unwrap().set(
                "Annots",
                lopdf::Object::Array(vec![lopdf::Object::Integer(7)]),
            ),
        }
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let changed = engine
            .open_bytes(bytes, None, Path::new("invalid-register.pdf"))
            .unwrap();
        assert!(
            changed.annotation_register().is_err(),
            "malformed case {fault} must not become a plausible zero or partial total"
        );
    }
}
