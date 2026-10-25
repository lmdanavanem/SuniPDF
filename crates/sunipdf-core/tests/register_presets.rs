use serde_json::{json, Value};
use std::collections::BTreeMap;
use sunipdf_core::annotation_register::{
    AnnotationRegister, RegisterFields, RegisterFilter, RegisterRow, RegisterSort,
};
use sunipdf_core::plan_zones::{ZoneAssignment, ZoneAssignmentStatus, ZoneFilter, ZonePathEntry};
use sunipdf_core::register_presets::{
    RegisterPreset, RegisterPresetLibrary, MAX_REGISTER_PRESETS, MAX_REGISTER_PRESET_JSON_BYTES,
};

fn preset(name: &str) -> RegisterPreset {
    RegisterPreset::capture(name, &RegisterFilter::default(), RegisterSort::Page, false).unwrap()
}

fn library_value() -> Value {
    let mut library = RegisterPresetLibrary::default();
    library.insert(preset("Electrical review")).unwrap();
    serde_json::from_slice(&library.to_json().unwrap()).unwrap()
}

fn row(page: usize, index: usize, author: &str, room: Option<&str>) -> RegisterRow {
    RegisterRow {
        page,
        index,
        annotation_id: Some(format!("local-{page}-{index}")),
        kind: "Stamp".into(),
        author: Some(author.into()),
        modified: Some("D:20261025093000Z".into()),
        contents: Some("Check distribution board".into()),
        subject: Some("Lighting review".into()),
        rect: Some([10.0, 20.0, 34.0, 44.0]),
        parent: None,
        is_reply: false,
        hidden: false,
        locked: false,
        review_status: Some("Accepted".into()),
        fields: RegisterFields {
            business_status: Some("Ready".into()),
            discipline: Some("Electrical".into()),
            custom: room
                .map(|room| BTreeMap::from([("Room".into(), room.into())]))
                .unwrap_or_default(),
        },
        count_group: Some("Lights".into()),
        zone: ZoneAssignment {
            status: ZoneAssignmentStatus::Assigned,
            zone_id: Some("room-a".into()),
            path: vec![ZonePathEntry {
                id: "room-a".into(),
                name: "Room A".into(),
            }],
            candidates: vec![],
        },
    }
}

#[test]
fn captured_filters_round_trip_and_filter_actual_register_rows() {
    let filter = RegisterFilter {
        query: "DISTRIBUTION".into(),
        author: "Alice".into(),
        modified: "D:202610".into(),
        review_status: "Accepted".into(),
        business_status: "Ready".into(),
        discipline: "Electrical".into(),
        kind: "Stamp".into(),
        count_group: "Lights".into(),
        include_replies: true,
        custom_sort: Some("Room".into()),
        ..Default::default()
    };
    let saved =
        RegisterPreset::capture("Site review", &filter, RegisterSort::Author, true).unwrap();
    let mut library = RegisterPresetLibrary::default();
    library.insert(saved).unwrap();
    let bytes = library.to_json().unwrap();
    let loaded = RegisterPresetLibrary::from_json(&bytes).unwrap();
    assert_eq!(library, loaded);
    let mut report = AnnotationRegister::default();
    report.page_count = 3;
    report.rows = vec![
        row(1, 0, "Alice", Some("A")),
        row(1, 1, "Alice", Some("C")),
        row(2, 0, "Bob", Some("Z")),
        row(3, 0, "Alice", None),
        row(3, 1, "Alice", Some("B")),
    ];
    report.rows[4].is_reply = true;
    let original = report.rows.clone();
    let mut active = RegisterFilter::default();
    let (mut sort, mut descending) = (RegisterSort::Page, false);
    loaded
        .get("SITE REVIEW")
        .unwrap()
        .apply_to(&mut active, &mut sort, &mut descending)
        .unwrap();
    assert_eq!(active, filter);
    assert_eq!((sort, descending), (RegisterSort::Author, true));
    assert_eq!(
        report.select_indices(&active, sort, descending),
        vec![1, 4, 0, 3]
    );
    active.include_replies = false;
    assert_eq!(
        report.select_indices(&active, sort, descending),
        vec![1, 0, 3]
    );
    assert_eq!(report.rows, original);
}

#[test]
fn loading_preserves_current_document_scope_and_does_not_export_it() {
    let captured = RegisterFilter {
        page: Some(99),
        zone: Some(ZoneFilter {
            page: 99,
            id: "other-document-zone".into(),
            descendants: true,
        }),
        zone_status: Some(ZoneAssignmentStatus::Stale),
        author: "Alice".into(),
        ..Default::default()
    };
    let saved =
        RegisterPreset::capture("Current author", &captured, RegisterSort::Page, false).unwrap();
    let mut library = RegisterPresetLibrary::default();
    library.insert(saved).unwrap();
    let value: Value = serde_json::from_slice(&library.to_json().unwrap()).unwrap();
    let root = value.as_object().unwrap();
    assert_eq!(root.len(), 3);
    assert!(["format", "presets", "version"]
        .iter()
        .all(|key| root.contains_key(*key)));
    let criteria = value["presets"][0]["criteria"].as_object().unwrap();
    assert_eq!(criteria.len(), 10);
    for forbidden in [
        "page",
        "zone",
        "zone_status",
        "path",
        "password",
        "source_sha256",
        "annotation_id",
        "selection",
    ] {
        assert!(!criteria.contains_key(forbidden));
    }
    assert!(!String::from_utf8(library.to_json().unwrap())
        .unwrap()
        .contains("other-document-zone"));
    let mut active = RegisterFilter {
        page: Some(2),
        zone: Some(ZoneFilter {
            page: 2,
            id: "room-a".into(),
            descendants: false,
        }),
        zone_status: Some(ZoneAssignmentStatus::Assigned),
        ..Default::default()
    };
    let scope = (active.page, active.zone.clone(), active.zone_status);
    let (mut sort, mut descending) = (RegisterSort::Kind, true);
    library
        .get("Current author")
        .unwrap()
        .apply_to(&mut active, &mut sort, &mut descending)
        .unwrap();
    assert_eq!(
        (active.page, active.zone.clone(), active.zone_status),
        scope
    );
    let mut report = AnnotationRegister::default();
    report.rows = vec![
        row(1, 0, "Alice", None),
        row(2, 0, "Alice", None),
        row(2, 1, "Bob", None),
    ];
    assert_eq!(report.select_indices(&active, sort, descending), vec![1]);
}

#[test]
fn built_in_sort_direction_and_custom_sort_are_retained_exactly() {
    let mut report = AnnotationRegister::default();
    report.rows = vec![
        row(1, 0, "Zoe", Some("A")),
        row(2, 0, "Alice", Some("Z")),
        row(1, 1, "Zoe", None),
    ];
    for descending in [false, true] {
        for sort in RegisterSort::ALL {
            let filter = RegisterFilter::default();
            let saved = RegisterPreset::capture("Order", &filter, sort, descending).unwrap();
            let mut active = RegisterFilter {
                custom_sort: Some("Room".into()),
                ..Default::default()
            };
            let (mut actual_sort, mut actual_direction) = (RegisterSort::Page, !descending);
            saved
                .apply_to(&mut active, &mut actual_sort, &mut actual_direction)
                .unwrap();
            assert_eq!(active.custom_sort, None);
            assert_eq!(
                report.select_indices(&active, actual_sort, actual_direction),
                report.select_indices(&filter, sort, descending)
            );
        }
    }
    assert_eq!(
        report.select_indices(&RegisterFilter::default(), RegisterSort::Author, false),
        vec![1, 0, 2]
    );
    assert_eq!(
        report.select_indices(&RegisterFilter::default(), RegisterSort::Author, true),
        vec![2, 0, 1]
    );
}

#[test]
fn imports_are_all_or_nothing_for_late_collisions_and_invalid_entries() {
    let mut library = RegisterPresetLibrary::default();
    library.insert(preset("Existing")).unwrap();
    let before = library.clone();
    let mut incoming = library_value();
    incoming["presets"] = json!([preset("New"), preset("EXISTING")]);
    assert!(library
        .import_json(&serde_json::to_vec(&incoming).unwrap())
        .is_err());
    assert_eq!(library, before);
    incoming["presets"][1] = serde_json::to_value(preset("Other")).unwrap();
    incoming["presets"][1]["criteria"]["query"] = json!("line\nbreak");
    assert!(library
        .import_json(&serde_json::to_vec(&incoming).unwrap())
        .is_err());
    assert_eq!(library, before);
    incoming["presets"][1]["criteria"]["query"] = json!("safe");
    assert_eq!(
        library
            .import_json(&serde_json::to_vec(&incoming).unwrap())
            .unwrap(),
        2
    );
    assert_eq!(library.entries().len(), 3);
    assert!(library.remove("new"));
    assert!(!library.remove("missing"));
    assert!(library.get("EXISTING").is_some());
}

#[test]
fn strict_json_rejects_unknown_duplicate_keys_versions_and_document_fields() {
    let original = library_value();
    for (key, value) in [
        ("version", json!(2)),
        ("version", json!(1.0)),
        ("format", json!("other")),
        ("source_sha256", json!("a".repeat(64))),
        ("password", json!("injected")),
    ] {
        let mut invalid = original.clone();
        invalid[key] = value;
        assert!(RegisterPresetLibrary::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    for field in [
        "page",
        "zone",
        "zone_status",
        "path",
        "selection",
        "annotation_id",
    ] {
        let mut invalid = original.clone();
        invalid["presets"][0]["criteria"][field] = json!("injected");
        assert!(RegisterPresetLibrary::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    let text = serde_json::to_string(&original).unwrap();
    for invalid in [
        text.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        text.replacen("\"name\":", "\"name\":\"Earlier\",\"name\":", 1),
        text.replacen("\"query\":", "\"query\":\"Earlier\",\"query\":", 1),
        format!("{text} {text}"),
    ] {
        assert!(
            RegisterPresetLibrary::from_json(invalid.as_bytes()).is_err(),
            "{invalid}"
        );
    }
    let mut duplicate = original.clone();
    duplicate["presets"] = json!([preset("Review"), preset("review")]);
    assert!(RegisterPresetLibrary::from_json(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    let bom = [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat();
    assert!(RegisterPresetLibrary::from_json(&bom).is_ok());
}

#[test]
fn capacity_utf8_bounds_and_failed_apply_leave_state_intact() {
    assert!(RegisterPreset::capture(
        &"é".repeat(32),
        &RegisterFilter::default(),
        RegisterSort::Page,
        false
    )
    .is_ok());
    for name in [
        String::new(),
        " ".into(),
        " trailing ".into(),
        "x\ty".into(),
        "é".repeat(33),
    ] {
        assert!(RegisterPreset::capture(
            &name,
            &RegisterFilter::default(),
            RegisterSort::Page,
            false
        )
        .is_err());
    }
    let mut library = RegisterPresetLibrary::default();
    for index in 0..MAX_REGISTER_PRESETS {
        library.insert(preset(&format!("Preset {index}"))).unwrap();
    }
    let before = library.clone();
    assert!(library.insert(preset("Excess")).is_err());
    assert_eq!(library, before);
    assert!(
        RegisterPresetLibrary::from_json(&vec![b' '; MAX_REGISTER_PRESET_JSON_BYTES + 1]).is_err()
    );
    let mut invalid = preset("Invalid");
    invalid.criteria.query = "é".repeat(257);
    let mut active = RegisterFilter {
        author: "Original".into(),
        ..Default::default()
    };
    let original = active.clone();
    let (mut sort, mut descending) = (RegisterSort::Author, true);
    assert!(invalid
        .apply_to(&mut active, &mut sort, &mut descending)
        .is_err());
    assert_eq!(
        (active, sort, descending),
        (original, RegisterSort::Author, true)
    );
    invalid.criteria.query = String::new();
    invalid.criteria.custom_sort = Some("x".repeat(65));
    assert!(invalid.validate().is_err());
}

#[test]
fn encoded_byte_limit_is_checked_before_an_insert_or_import_changes_the_library() {
    let filter = RegisterFilter {
        query: "\\".repeat(512),
        author: "\\".repeat(512),
        modified: "\\".repeat(512),
        review_status: "\\".repeat(512),
        business_status: "\\".repeat(512),
        discipline: "\\".repeat(512),
        kind: "\\".repeat(512),
        count_group: "\\".repeat(512),
        ..Default::default()
    };
    let mut library = RegisterPresetLibrary::default();
    let mut refusal = false;
    for index in 0..MAX_REGISTER_PRESETS {
        let before = library.clone();
        if library
            .insert(
                RegisterPreset::capture(
                    &format!("Large {index}"),
                    &filter,
                    RegisterSort::Page,
                    false,
                )
                .unwrap(),
            )
            .is_err()
        {
            assert_eq!(library, before);
            refusal = true;
            break;
        }
        assert!(library.to_json().unwrap().len() <= MAX_REGISTER_PRESET_JSON_BYTES);
    }
    assert!(refusal);
    let imported = serde_json::to_vec(&library_value()).unwrap();
    let mut full = RegisterPresetLibrary::default();
    for index in 0..MAX_REGISTER_PRESETS {
        full.insert(preset(&format!("Full {index}"))).unwrap();
    }
    let before = full.clone();
    assert!(full.import_json(&imported).is_err());
    assert_eq!(full, before);
}

#[test]
fn caller_new_copy_export_does_not_replace_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("filters.json");
    let mut library = RegisterPresetLibrary::default();
    library.insert(preset("Review")).unwrap();
    let bytes = library.to_json().unwrap();
    sunipdf_core::write_new_copy(&path, &bytes).unwrap();
    assert!(sunipdf_core::write_new_copy(&path, b"replacement").is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
