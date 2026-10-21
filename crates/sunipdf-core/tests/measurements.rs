use std::path::Path;
use sunipdf_core::{
    annot_rich::RichAnnotation,
    measurement::{Measurement, MeasurementKind, MeasurementUnit},
    security, Engine, EngineError, PageMargins, Protection, Rotation,
};

fn measurement() -> Measurement {
    Measurement {
        kind: MeasurementKind::Area,
        points: vec![(40.0, 70.0), (140.0, 70.0), (140.0, 120.0), (40.0, 120.0)],
        calibration: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: 10.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn calibrated_measurements_survive_rotation_crop_preview_reopen_and_permissions_without_stale_moves(
) {
    let engine = Engine::new().unwrap();
    let bytes = include_bytes!("fixtures/two-pages.pdf");
    let mut doc = engine
        .open_bytes(bytes.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let original = doc.snapshot().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.restore(original.clone()).unwrap();
        doc.set_page_rotation(0, rotation).unwrap();
        doc.crop_page(
            0,
            PageMargins {
                left: 10.0,
                top: 12.0,
                right: 13.0,
                bottom: 14.0,
            },
        )
        .unwrap();
        let before = doc.snapshot().unwrap();
        let pixels = doc.render_page(0, 1.0).unwrap().rgba;
        let m = measurement();
        let preview = doc
            .preview_rich_annotation(0, &RichAnnotation::Measurement(m), Some("Plan reviewer"))
            .unwrap();
        assert!(
            doc.measurements(0).unwrap().is_empty(),
            "preview preserves original"
        );
        let index = doc
            .commit_rich_annotation_preview(preview.prepared.unwrap())
            .unwrap();
        assert_ne!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
        let records = doc.measurements(0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["value"], 50.0);
        assert_eq!(records[0]["unit"], "m2");
        assert!(doc.measurements(1).unwrap().is_empty());
        let detail = doc
            .annotation_details(0)
            .unwrap()
            .into_iter()
            .find(|d| d.index == index)
            .unwrap();
        assert!(
            !detail.can_move && !detail.can_resize && !detail.can_restyle && !detail.can_edit_text
        );
        assert!(doc.move_annotation(0, index, 10.0, 10.0).is_err());
        assert!(doc.resize_annotation(0, index, detail.rect).is_err());
        assert!(doc
            .set_annotation_contents(0, index, "wrong value")
            .is_err());
        let output = temp.path().join(format!("measurement-{rotation:?}.pdf"));
        doc.save_as(&output).unwrap();
        let mut reopened = engine.open(&output, None).unwrap();
        assert_eq!(reopened.measurements(0).unwrap(), records);
        reopened.delete_annotation_thread(0, index).unwrap();
        assert!(reopened.measurements(0).unwrap().is_empty());
        assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, pixels);
        doc.restore(before).unwrap();
        assert!(
            doc.measurements(0).unwrap().is_empty(),
            "Undo snapshot removes metadata too"
        );
        let mut invalid = measurement();
        invalid.calibration[1] = invalid.calibration[0];
        assert!(doc.add_measurement_annotation(0, &invalid, None).is_err());
        invalid = measurement();
        invalid.points[0].0 = -1.0;
        assert!(doc.add_measurement_annotation(0, &invalid, None).is_err());
        invalid = measurement();
        invalid.points[0].0 = f32::NAN;
        assert!(doc.add_measurement_annotation(0, &invalid, None).is_err());
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
        assert!(doc.measurements(0).unwrap().is_empty());
    }
    for aes256 in [false, true] {
        let user = credential();
        let owner = credential();
        for allow_annotate in [false, true] {
            let protected = security::protect(
                bytes,
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
            let mut doc = engine
                .open_bytes(protected, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            if allow_annotate {
                doc.add_measurement_annotation(0, &measurement(), None)
                    .unwrap();
                for password in [&user, &owner] {
                    let reopened = engine
                        .open_bytes(
                            doc.snapshot().unwrap(),
                            Some(password),
                            Path::new("saved.pdf"),
                        )
                        .unwrap();
                    assert!(reopened.properties().encrypted);
                    assert_eq!(reopened.measurements(0).unwrap()[0]["value"], 50.0);
                }
            } else {
                assert!(matches!(
                    doc.add_measurement_annotation(0, &measurement(), None),
                    Err(EngineError::SecurityRestricted)
                ));
            }
        }
    }
    doc.restore(original).unwrap();
    doc.add_measurement_annotation(0, &measurement(), None)
        .unwrap();
    let snapshot = doc.snapshot().unwrap();
    for change in ["version", "rect", "imported"] {
        let mut pdf = lopdf::Document::load_mem(&snapshot).unwrap();
        let dictionary = pdf
            .objects
            .values_mut()
            .filter_map(|o| o.as_dict_mut().ok())
            .find(|d| d.has(b"SuniPDFMeasurement"))
            .unwrap();
        if change == "version" {
            let raw = dictionary
                .get(b"SuniPDFMeasurement")
                .unwrap()
                .as_str()
                .unwrap();
            let mut value: serde_json::Value = serde_json::from_slice(raw).unwrap();
            value["version"] = 99.into();
            dictionary.set(
                "SuniPDFMeasurement",
                lopdf::Object::string_literal(value.to_string()),
            );
        } else if change == "imported" {
            dictionary.remove(b"SuniPDFMeasurement");
            dictionary.set("Measure", lopdf::Dictionary::new());
        } else {
            dictionary.set("Rect", vec![0.into(), 0.into(), 200.into(), 200.into()]);
        }
        let mut mutated = Vec::new();
        pdf.save_to(&mut mutated).unwrap();
        let changed = engine
            .open_bytes(mutated, None, Path::new("external.pdf"))
            .unwrap();
        assert!(
            changed.measurements(0).is_err(),
            "unsupported or moved records cannot export a plausible stale result"
        );
    }
}
