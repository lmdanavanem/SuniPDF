use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{
    measurement::{Measurement, MeasurementKind, MeasurementUnit},
    security, Engine, EngineError, PageCalibration, PageMargins, PointRect, Protection, RedactMark,
    RedactOptions, ResizeMode, Rotation,
};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");
const KEY: &[u8] = b"SuniPDFPageCalibration";

// PDFium bindings are initialized once per process. Keep the engine alive for this test binary
// and serialize whole cases so that every document is dropped before another case uses it.
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new().unwrap()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn scale(distance: f64) -> PageCalibration {
    PageCalibration {
        points: [(40.0, 40.0), (140.0, 40.0)],
        known_distance: distance,
        unit: MeasurementUnit::Metres,
    }
}

fn measurement(calibration: &PageCalibration) -> Measurement {
    Measurement {
        holes: Vec::new(),
        depth: None,
        kind: MeasurementKind::Length,
        points: vec![(40.0, 80.0), (90.0, 80.0)],
        calibration: calibration.points,
        known_distance: calibration.known_distance,
        unit: calibration.unit,
        precision: 2,
    }
}

fn credential() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn changed_pdf(input: &[u8], change: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(input).unwrap();
    change(&mut pdf);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn assert_same_document(before: &[u8], after: &[u8], password: Option<&str>) {
    let parse = |bytes: &[u8]| {
        let mut pdf = match password {
            Some(password) => lopdf::Document::load_mem_with_options(
                bytes,
                lopdf::LoadOptions::with_password(password),
            ),
            None => lopdf::Document::load_mem(bytes),
        }
        .unwrap();
        // PDFium may refresh the trailer ID when merely serializing a snapshot. File offsets
        // likewise describe serialization, not document state. Compare every actual object,
        // the remaining trailer and the decrypted permission state instead of ciphertext bytes.
        pdf.trailer.remove(b"ID");
        for object in pdf.objects.values_mut() {
            if let lopdf::Object::Stream(stream) = object {
                stream.start_position = None;
            }
        }
        let permissions = pdf
            .encryption_state
            .as_ref()
            .map(|state| state.permissions().bits());
        (pdf.objects, pdf.trailer, permissions)
    };
    assert_eq!(parse(before), parse(after));
}

#[test]
fn defaults_reopen_without_visual_changes_and_never_recalculate_saved_measurements() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    let before = doc.snapshot().unwrap();
    let pixels = doc.render_page(0, 1.0).unwrap().rgba;
    assert_eq!(doc.page_calibration(0).unwrap(), None);
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    doc.set_page_calibration(1, Some(&scale(20.0))).unwrap();
    assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, pixels);
    assert!(doc.measurement_records(0).unwrap().is_empty());
    let defaults = doc.snapshot().unwrap();
    doc.add_measurement_annotation(0, &measurement(&scale(10.0)), Some("Reviewer"))
        .unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(10.0)));
    let records = doc.measurements(0).unwrap();
    assert_eq!(records[0]["value"], 5.0);
    let typed = doc.measurement_records(0).unwrap();
    assert_eq!(typed[0].measurement, measurement(&scale(10.0)));
    assert_eq!(typed[0].result().unwrap(), records[0]);
    doc.set_page_calibration(0, Some(&scale(30.0))).unwrap();
    assert_eq!(doc.measurements(0).unwrap(), records);
    assert_eq!(doc.page_calibration(1).unwrap(), Some(scale(20.0)));
    let saved = doc.snapshot().unwrap();
    doc.set_page_calibration(0, None).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), None);
    assert_eq!(doc.measurements(0).unwrap(), records);
    doc.restore(saved).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(30.0)));
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("calibrated.pdf");
    doc.save_as(&output).unwrap();
    let reopened = engine.open(&output, None).unwrap();
    assert_eq!(reopened.page_calibration(0).unwrap(), Some(scale(30.0)));
    assert_eq!(reopened.page_calibration(1).unwrap(), Some(scale(20.0)));
    assert_eq!(reopened.measurement_records(0).unwrap(), typed);
    doc.restore(defaults).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(10.0)));
    assert!(doc.measurements(0).unwrap().is_empty());
    doc.restore(before).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), None);
    for invalid in [
        PageCalibration {
            points: [(1.0, 1.0); 2],
            ..scale(10.0)
        },
        PageCalibration {
            known_distance: f64::NAN,
            ..scale(10.0)
        },
        PageCalibration {
            known_distance: 0.0,
            ..scale(10.0)
        },
        PageCalibration {
            points: [(-1.0, 0.0), (10.0, 0.0)],
            ..scale(10.0)
        },
    ] {
        let snapshot = doc.snapshot().unwrap();
        assert!(doc.set_page_calibration(0, Some(&invalid)).is_err());
        assert_same_document(&snapshot, &doc.snapshot().unwrap(), None);
    }
}

#[test]
fn calibration_uses_user_space_and_rejects_changed_bounds_or_page_content() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.crop_page(
        0,
        PageMargins {
            left: 10.0,
            right: 12.0,
            top: 14.0,
            bottom: 16.0,
        },
    )
    .unwrap();
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    let geo = doc.page_geometry(0).unwrap();
    let user_points = scale(10.0).points.map(|(x, y)| geo.to_user(x, y));
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        doc.set_page_rotation(0, rotation).unwrap();
        let geo = doc.page_geometry(0).unwrap();
        let expected = PageCalibration {
            points: user_points.map(|(x, y)| geo.to_display(x, y)),
            ..scale(10.0)
        };
        assert_eq!(doc.page_calibration(0).unwrap(), Some(expected.clone()));
        let reopened = engine
            .open_bytes(doc.snapshot().unwrap(), None, Path::new("rotated.pdf"))
            .unwrap();
        assert_eq!(reopened.page_calibration(0).unwrap(), Some(expected));
    }
    doc.set_page_rotation(0, Rotation::R0).unwrap();
    let saved = doc.snapshot().unwrap();
    doc.crop_page(
        0,
        PageMargins {
            left: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(doc.page_calibration(0).is_err());
    doc.set_page_calibration(0, Some(&scale(20.0))).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(20.0)));
    doc.restore(saved.clone()).unwrap();
    doc.reset_crop(0).unwrap();
    assert!(doc.page_calibration(0).is_err());
    doc.set_page_calibration(0, None).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), None);
    for mode in [ResizeMode::KeepContent, ResizeMode::ScaleContent] {
        doc.restore(saved.clone()).unwrap();
        doc.resize_page(0, 350.0, 450.0, mode).unwrap();
        assert!(doc.page_calibration(0).is_err());
        doc.set_page_calibration(0, Some(&scale(30.0))).unwrap();
        assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(30.0)));
    }
    let content_changed = changed_pdf(&saved, |pdf| {
        let page = pdf.get_pages()[&1];
        let content = pdf.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"q 0 0 20 20 re f Q".to_vec(),
        ));
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Contents", content);
    });
    let mut changed = engine
        .open_bytes(content_changed, None, Path::new("external.pdf"))
        .unwrap();
    assert_eq!(changed.page_geometry(0).unwrap(), geo);
    assert!(changed.page_calibration(0).is_err());
    changed.set_page_calibration(0, Some(&scale(40.0))).unwrap();
    assert_eq!(changed.page_calibration(0).unwrap(), Some(scale(40.0)));
}

#[test]
fn defaults_follow_page_objects_through_move_extract_insert_duplicate_and_delete() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    doc.set_page_calibration(1, Some(&scale(20.0))).unwrap();
    doc.move_page(0, 1).unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(20.0)));
    assert_eq!(doc.page_calibration(1).unwrap(), Some(scale(10.0)));
    let temp = tempfile::tempdir().unwrap();
    let extracted = temp.path().join("pages.pdf");
    doc.extract_to(&[1, 0, 1], &extracted).unwrap();
    let reopened = engine.open(&extracted, None).unwrap();
    for (page, distance) in [10.0, 20.0, 10.0].into_iter().enumerate() {
        assert_eq!(
            reopened.page_calibration(page).unwrap(),
            Some(scale(distance))
        );
    }
    let mut target = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("target.pdf"))
        .unwrap();
    target.insert_pages_from(&extracted, None, &[], 1).unwrap();
    target.insert_blank_pages(2, 1, 612.0, 792.0).unwrap();
    for (page, distance) in [None, Some(10.0), None, Some(20.0), Some(10.0), None]
        .into_iter()
        .enumerate()
    {
        assert_eq!(target.page_calibration(page).unwrap(), distance.map(scale));
    }
    target.delete_pages(&[0, 2, 5]).unwrap();
    for (page, distance) in [10.0, 20.0, 10.0].into_iter().enumerate() {
        assert_eq!(
            target.page_calibration(page).unwrap(),
            Some(scale(distance))
        );
    }
    target.set_page_calibration(0, Some(&scale(99.0))).unwrap();
    assert_eq!(target.page_calibration(2).unwrap(), Some(scale(10.0)));
}

#[test]
fn malformed_or_future_metadata_is_never_silently_used_replaced_or_removed() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    let saved = doc.snapshot().unwrap();
    for change in [
        "version",
        "unit",
        "zero",
        "points",
        "nan",
        "hash",
        "unknown",
        "size",
        "null",
        "reference",
    ] {
        let bytes = changed_pdf(&saved, |pdf| {
            let page = pdf.get_pages()[&1];
            let dict = pdf.get_dictionary_mut(page).unwrap();
            let raw = dict.get(KEY).unwrap().as_str().unwrap();
            let mut value: serde_json::Value = serde_json::from_slice(raw).unwrap();
            match change {
                "version" => {
                    let future_version = value["version"].as_u64().unwrap() + 1;
                    value["version"] = future_version.into();
                }
                "unit" => value["unit"] = "unknown".into(),
                "zero" => value["known_distance"] = 0.into(),
                "points" => value["points"] = serde_json::json!([[40, 40], [40, 40]]),
                "nan" => value["known_distance"] = serde_json::Value::Null,
                "hash" => value["content_sha256"] = "invalid".into(),
                "unknown" => value["unknown"] = true.into(),
                _ => (),
            }
            let raw = match change {
                "size" => lopdf::Object::string_literal(" ".repeat(4097)),
                "null" => lopdf::Object::Null,
                "reference" => lopdf::Object::Reference(page),
                _ => lopdf::Object::string_literal(value.to_string()),
            };
            dict.set(KEY, raw);
        });
        let mut changed = engine
            .open_bytes(bytes, None, Path::new("malformed.pdf"))
            .unwrap();
        let before = changed.snapshot().unwrap();
        assert!(changed.page_calibration(0).is_err(), "{change}");
        assert!(
            changed.set_page_calibration(0, Some(&scale(20.0))).is_err(),
            "{change}"
        );
        assert!(changed.set_page_calibration(0, None).is_err(), "{change}");
        assert_same_document(&before, &changed.snapshot().unwrap(), None);
    }
    let scaled_unit = changed_pdf(&saved, |pdf| {
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page).unwrap().set("UserUnit", 2);
    });
    let mut changed = engine
        .open_bytes(scaled_unit, None, Path::new("user-unit.pdf"))
        .unwrap();
    assert!(changed.page_calibration(0).is_err());
    assert!(changed.set_page_calibration(0, Some(&scale(20.0))).is_err());
    changed.set_page_calibration(0, None).unwrap();
    assert_eq!(changed.page_calibration(0).unwrap(), None);
    assert!(matches!(
        doc.page_calibration(2),
        Err(EngineError::PageOutOfRange(2))
    ));
}

#[test]
fn resource_only_form_edits_invalidate_page_and_region_scales_and_legacy_scales_are_recoverable() {
    use lopdf::{dictionary, Dictionary, Object, Stream};
    use sunipdf_core::measurement::MeasurementViewport;
    let engine = engine();
    let input = changed_pdf(SOURCE, |pdf| {
        let form = pdf.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form",
                "BBox" => vec![0.into(),0.into(),300.into(),300.into()],
                "Matrix" => vec![1.into(),0.into(),0.into(),1.into(),0.into(),0.into()],
                "Resources" => Dictionary::new(),
            },
            b"0 0 0 RG 2 w 40 80 m 140 80 l S".to_vec(),
        ));
        let contents = pdf.add_object(Stream::new(Dictionary::new(), b"/Fm0 Do".to_vec()));
        let page = pdf.get_pages()[&1];
        let dict = pdf.get_dictionary_mut(page).unwrap();
        dict.set(
            "Resources",
            dictionary! {"XObject" => dictionary! {"Fm0" => form}},
        );
        dict.set("Contents", contents);
    });
    let mut doc = engine
        .open_bytes(input, None, Path::new("form-resources.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    let region = MeasurementViewport {
        id: "detail".into(),
        name: "Detail".into(),
        bounds: PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 200.0,
            top: 200.0,
        },
        calibration: scale(20.0),
    };
    doc.set_measurement_viewports(0, &[], std::slice::from_ref(&region))
        .unwrap();
    let saved = doc.snapshot().unwrap();
    let reopened = engine
        .open_bytes(saved.clone(), None, Path::new("saved.pdf"))
        .unwrap();
    assert_eq!(reopened.page_calibration(0).unwrap(), Some(scale(10.0)));
    assert_eq!(reopened.measurement_viewports(0).unwrap(), vec![region]);
    for change in ["stream", "matrix"] {
        let bytes = changed_pdf(&saved, |pdf| {
            let page = pdf.get_pages()[&1];
            let page = pdf.get_dictionary(page).unwrap();
            let resources = pdf
                .dereference(page.get(b"Resources").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            let xobjects = pdf
                .dereference(resources.get(b"XObject").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            let form = xobjects.get(b"Fm0").unwrap().as_reference().unwrap();
            let stream = pdf.get_object_mut(form).unwrap().as_stream_mut().unwrap();
            if change == "stream" {
                stream.dict.remove(b"Filter");
                stream.dict.remove(b"DecodeParms");
                stream.content = b"0 0 0 RG 2 w 40 80 m 240 80 l S".to_vec();
            } else {
                stream.dict.set(
                    "Matrix",
                    vec![2.into(), 0.into(), 0.into(), 2.into(), 0.into(), 0.into()],
                );
            }
        });
        let mut changed = engine
            .open_bytes(bytes, None, Path::new("changed-form.pdf"))
            .unwrap();
        assert!(changed.page_calibration(0).is_err(), "{change}");
        assert!(changed.measurement_viewports(0).is_err(), "{change}");
        let reset = changed.measurement_viewport_reset_target(0).unwrap();
        changed.clear_measurement_viewports(&reset).unwrap();
        assert!(changed.measurement_viewports(0).unwrap().is_empty());
        assert!(changed.page_calibration(0).is_err());
    }
    let legacy = changed_pdf(&saved, |pdf| {
        let page = pdf.get_pages()[&1];
        let dict = pdf.get_dictionary_mut(page).unwrap();
        let mut record: serde_json::Value =
            serde_json::from_slice(dict.get(KEY).unwrap().as_str().unwrap()).unwrap();
        assert_eq!(record["version"], 3);
        record["version"] = 1.into();
        dict.set(KEY, Object::string_literal(record.to_string()));
    });
    for replace in [false, true] {
        let mut legacy = engine
            .open_bytes(legacy.clone(), None, Path::new("legacy-scale.pdf"))
            .unwrap();
        assert!(legacy
            .page_calibration(0)
            .unwrap_err()
            .to_string()
            .contains("recalibrate or remove"));
        if replace {
            legacy.set_page_calibration(0, Some(&scale(30.0))).unwrap();
            assert_eq!(legacy.page_calibration(0).unwrap(), Some(scale(30.0)));
        } else {
            legacy.set_page_calibration(0, None).unwrap();
            assert_eq!(legacy.page_calibration(0).unwrap(), None);
        }
    }
}

#[test]
fn page_defaults_respect_modification_permissions_and_preserve_aes_protection() {
    let engine = engine();
    let mut source = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    source.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    let original = source.snapshot().unwrap();
    for aes256 in [false, true] {
        let user = credential();
        let owner = credential();
        for allow_modify in [false, true] {
            let protected = security::protect(
                &original,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    aes256,
                    allow_modify,
                    allow_annotate: true,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut doc = engine
                .open_bytes(protected, Some(&user), Path::new("protected.pdf"))
                .unwrap();
            assert_eq!(doc.page_calibration(0).unwrap(), Some(scale(10.0)));
            if allow_modify {
                doc.set_page_calibration(0, Some(&scale(20.0))).unwrap();
            } else {
                let before = doc.snapshot().unwrap();
                assert!(matches!(
                    doc.set_page_calibration(0, Some(&scale(20.0))),
                    Err(EngineError::SecurityRestricted)
                ));
                assert!(matches!(
                    doc.set_page_calibration(0, None),
                    Err(EngineError::SecurityRestricted)
                ));
                assert_same_document(&before, &doc.snapshot().unwrap(), Some(&user));
                doc.add_measurement_annotation(0, &measurement(&scale(10.0)), None)
                    .unwrap();
            }
            for password in [&user, &owner] {
                let reopened = engine
                    .open_bytes(
                        doc.snapshot().unwrap(),
                        Some(password),
                        Path::new("saved.pdf"),
                    )
                    .unwrap();
                assert!(reopened.properties().encrypted);
                assert_eq!(reopened.properties().can_modify, allow_modify);
                assert_eq!(
                    reopened.page_calibration(0).unwrap(),
                    Some(scale(if allow_modify { 20.0 } else { 10.0 }))
                );
            }
        }
    }
}

#[test]
fn metadata_cleanup_and_redaction_remove_private_page_scales() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(SOURCE.to_vec(), None, Path::new("source.pdf"))
        .unwrap();
    doc.set_page_calibration(0, Some(&scale(10.0))).unwrap();
    doc.set_page_calibration(1, Some(&scale(20.0))).unwrap();
    let bytes = doc.snapshot().unwrap();
    for cleaned in [
        security::strip_metadata(&bytes, None).unwrap(),
        doc.sanitized_copy().unwrap().0,
    ] {
        let reopened = engine
            .open_bytes(cleaned, None, Path::new("cleaned.pdf"))
            .unwrap();
        assert_eq!(reopened.page_calibration(0).unwrap(), None);
        assert_eq!(reopened.page_calibration(1).unwrap(), None);
    }
    doc.redact(
        &[RedactMark {
            page: 0,
            rect: PointRect {
                left: 10.0,
                bottom: 10.0,
                right: 20.0,
                top: 20.0,
            },
        }],
        &RedactOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.page_calibration(0).unwrap(), None);
    assert_eq!(doc.page_calibration(1).unwrap(), None);
}
