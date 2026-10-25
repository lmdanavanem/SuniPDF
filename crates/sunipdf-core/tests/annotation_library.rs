use lopdf::Object;
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;
use sunipdf_core::annot_rich::RichAnnotation;
use sunipdf_core::annotation_library::*;
use sunipdf_core::{security, AnnotShape, AnnotStyle, Engine, EngineError, PointRect, Protection};

const SOURCE: &[u8] = include_bytes!("fixtures/two-pages.pdf");

#[path = "annotation_library/saved_stamps.rs"]
mod saved_stamps;

fn entry() -> LibraryEntry {
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(3, 2, image::Rgba([240, 30, 30, 180]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    LibraryEntry {
        id: "review-group".into(),
        name: "Review group".into(),
        provenance: LibraryProvenance::default(),
        primitives: vec![
            LibraryPrimitive::Shape {
                shape: LibraryShape::Square,
                rect: [0.0, 0.0, 90.0, 45.0],
                style: LibraryStyle::default(),
            },
            LibraryPrimitive::FreeText {
                rect: [0.0, 55.0, 160.0, 100.0],
                text: "Local template".into(),
                style: LibraryTextStyle::default(),
            },
            LibraryPrimitive::Stamp {
                rect: [180.0, 0.0, 280.0, 30.0],
                stamp: LibraryStamp::Approved,
            },
            LibraryPrimitive::Vertices {
                vertex_kind: LibraryVertexKind::Polyline,
                points: vec![(0.0, 110.0), (50.0, 140.0), (120.0, 110.0)],
                style: LibraryStyle::default(),
            },
            LibraryPrimitive::Vertices {
                vertex_kind: LibraryVertexKind::Polygon,
                points: vec![(0.0, 160.0), (60.0, 220.0), (120.0, 160.0)],
                style: LibraryStyle::default(),
            },
            LibraryPrimitive::Vertices {
                vertex_kind: LibraryVertexKind::Cloud,
                points: vec![(160.0, 160.0), (210.0, 220.0), (260.0, 160.0)],
                style: LibraryStyle::default(),
            },
            LibraryPrimitive::Image {
                rect: [180.0, 60.0, 240.0, 120.0],
                png: EmbeddedPng::from_png_bytes(png.get_ref()).unwrap(),
                opacity: 0.7,
            },
        ],
    }
}

fn normalized(
    bytes: &[u8],
    password: Option<&str>,
) -> (
    std::collections::BTreeMap<lopdf::ObjectId, Object>,
    lopdf::Dictionary,
) {
    let mut pdf = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or_default()),
    )
    .unwrap();
    pdf.trailer.remove(b"ID");
    for object in pdf.objects.values_mut() {
        if let Object::Stream(stream) = object {
            stream.start_position = None;
            stream.allows_compression = true;
        }
    }
    (pdf.objects, pdf.trailer)
}

#[test]
fn annotation_library_placement_preview_atomicity_identity_rotation_and_encryption() {
    let engine = Engine::new().unwrap();
    saved_groups_capture_round_trip_and_refuse_stale_or_mixed_sources(&engine);
    saved_stamps::round_trip_with_alpha_and_revision_isolation(&engine);
    let entry = entry();
    let placement = LibraryPlacement {
        origin: (40.0, 60.0),
        scale: 1.0,
    };
    for rotation in [0, 90, 180, 270] {
        let mut pdf = lopdf::Document::load_mem(SOURCE).unwrap();
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Rotate", rotation);
        pdf.get_dictionary_mut(page).unwrap().set(
            "CropBox",
            vec![100.into(), 50.into(), 500.into(), 650.into()],
        );
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("library.pdf"))
            .unwrap();
        let before = doc.snapshot().unwrap();
        let count = doc.annotation_details(0).unwrap().len();
        let annotation = RichAnnotation::LibraryPlacement {
            entry: entry.clone(),
            placement,
        };
        let preview = doc
            .preview_rich_annotation(0, &annotation, Some("Current author"))
            .unwrap();
        assert_eq!(
            normalized(&before, None),
            normalized(&doc.snapshot().unwrap(), None)
        );
        assert!(
            preview.region.width > 500 && preview.region.height > 400,
            "preview covers the entire group"
        );
        doc.commit_rich_annotation_preview(preview.prepared.unwrap())
            .unwrap();
        let after = doc.snapshot().unwrap();
        let details = doc.annotation_details(0).unwrap();
        assert_eq!(details.len(), count + entry.primitives.len());
        let names: HashSet<_> = details[count..]
            .iter()
            .map(|d| d.name.clone().unwrap())
            .collect();
        assert_eq!(names.len(), entry.primitives.len());
        assert!(details[count..]
            .iter()
            .all(|d| d.author.as_deref() == Some("Current author")));
        let reopened = engine
            .open_bytes(after, None, Path::new("reopen.pdf"))
            .unwrap();
        assert_eq!(reopened.annotation_details(0).unwrap().len(), details.len());
        drop(reopened);
        let more = doc.place_library_entry(0, &entry, placement, None).unwrap();
        assert_eq!(more.len(), entry.primitives.len());
        assert!(
            doc.annotation_details(0).unwrap()[count + entry.primitives.len()..]
                .iter()
                .all(|d| !names.contains(d.name.as_ref().unwrap()))
        );
        doc.restore(before.clone()).unwrap();
        assert_eq!(doc.annotation_details(0).unwrap().len(), count);

        let preview = doc.preview_rich_annotation(0, &annotation, None).unwrap();
        doc.add_shape_annotation(
            0,
            AnnotShape::Square,
            PointRect {
                left: 300.0,
                bottom: 40.0,
                right: 350.0,
                top: 90.0,
            },
            &AnnotStyle::default(),
            None,
        )
        .unwrap();
        let changed = doc.snapshot().unwrap();
        assert!(doc
            .commit_rich_annotation_preview(preview.prepared.unwrap())
            .is_err());
        assert_eq!(
            normalized(&changed, None),
            normalized(&doc.snapshot().unwrap(), None)
        );
        doc.restore(before).unwrap();
        let mut invalid = entry.clone();
        invalid.primitives.push(LibraryPrimitive::Stamp {
            rect: [9000.0, 0.0, 9100.0, 30.0],
            stamp: LibraryStamp::Draft,
        });
        let before = doc.snapshot().unwrap();
        assert!(doc
            .place_library_entry(0, &invalid, placement, None)
            .is_err());
        assert!(doc
            .preview_rich_annotation(
                0,
                &RichAnnotation::LibraryPlacement {
                    entry: invalid,
                    placement
                },
                None
            )
            .is_err());
        assert_eq!(
            normalized(&before, None),
            normalized(&doc.snapshot().unwrap(), None)
        );
    }

    for aes256 in [false, true] {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).unwrap();
        let user = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let owner = format!("owner-{user}");
        for allowed in [false, true] {
            let encrypted = security::protect(
                SOURCE,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    allow_copy: false,
                    allow_modify: false,
                    allow_print: false,
                    allow_annotate: allowed,
                    aes256,
                },
            )
            .unwrap();
            for credential in [&user, &owner] {
                let mut doc = engine
                    .open_bytes(
                        encrypted.clone(),
                        Some(credential),
                        Path::new("protected.pdf"),
                    )
                    .unwrap();
                let before = doc.snapshot().unwrap();
                assert!(matches!(
                    doc.library_capture_candidates(0),
                    Err(EngineError::SecurityRestricted)
                ));
                if allowed {
                    let preview = doc
                        .preview_rich_annotation(
                            0,
                            &RichAnnotation::LibraryPlacement {
                                entry: entry.clone(),
                                placement,
                            },
                            None,
                        )
                        .unwrap();
                    doc.commit_rich_annotation_preview(preview.prepared.unwrap())
                        .unwrap();
                    let bytes = doc.snapshot().unwrap();
                    assert!(engine
                        .open_bytes(bytes.clone(), None, Path::new("locked.pdf"))
                        .is_err());
                    let reopened = engine
                        .open_bytes(bytes, Some(credential), Path::new("protected-result.pdf"))
                        .unwrap();
                    assert!(
                        !reopened.properties().can_copy
                            && !reopened.properties().can_modify
                            && !reopened.properties().can_print
                    );
                    assert!(reopened.properties().can_annotate);
                    assert_eq!(
                        reopened.annotation_details(0).unwrap().len(),
                        entry.primitives.len()
                    );
                } else {
                    assert!(matches!(
                        doc.place_library_entry(0, &entry, placement, None),
                        Err(EngineError::SecurityRestricted)
                    ));
                    assert!(matches!(
                        doc.preview_rich_annotation(
                            0,
                            &RichAnnotation::LibraryPlacement {
                                entry: entry.clone(),
                                placement
                            },
                            None
                        ),
                        Err(EngineError::SecurityRestricted)
                    ));
                    assert_eq!(
                        normalized(&before, Some(credential)),
                        normalized(&doc.snapshot().unwrap(), Some(credential))
                    );
                }
            }
        }
    }
}

fn saved_groups_capture_round_trip_and_refuse_stale_or_mixed_sources(engine: &Engine) {
    let provenance = LibraryProvenance {
        author: Some("Plan reviewer".into()),
        source: Some("Locally authored test group".into()),
        license: Some("CC0-1.0".into()),
    };
    let mut original = entry();
    original.primitives.retain(|primitive| {
        matches!(
            primitive,
            LibraryPrimitive::Shape { .. } | LibraryPrimitive::Vertices { .. }
        )
    });
    original.primitives.extend([
        LibraryPrimitive::Shape {
            shape: LibraryShape::Arrow,
            rect: [110.0, 40.0, 180.0, 90.0],
            style: LibraryStyle {
                rgb: [30, 90, 180],
                opacity: 0.75,
                width_pt: 1.5,
                dash_pattern: vec![4.0, 2.0],
                ..Default::default()
            },
        },
        LibraryPrimitive::Shape {
            shape: LibraryShape::Circle,
            rect: [170.0, 0.0, 230.0, 40.0],
            style: LibraryStyle {
                fill_rgb: Some([220, 240, 180]),
                ..Default::default()
            },
        },
    ]);
    for rotation in [0, 90, 180, 270] {
        let mut pdf = lopdf::Document::load_mem(SOURCE).unwrap();
        let page = pdf.get_pages()[&1];
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Rotate", rotation);
        pdf.get_dictionary_mut(page).unwrap().set(
            "CropBox",
            vec![100.into(), 50.into(), 500.into(), 650.into()],
        );
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut doc = engine
            .open_bytes(bytes, None, Path::new("capture.pdf"))
            .unwrap();
        let placed = doc
            .place_library_entry(
                0,
                &original,
                LibraryPlacement {
                    origin: (40.0, 60.0),
                    scale: 1.0,
                },
                Some("Source author"),
            )
            .unwrap();
        let before = doc.snapshot().unwrap();
        let candidates = doc.library_capture_candidates(0).unwrap();
        let selections: Vec<_> = candidates
            .iter()
            .map(|candidate| LibraryCaptureSelection {
                index: candidate.index,
                fingerprint: candidate.fingerprint.clone().unwrap_or_else(|| {
                    panic!(
                        "capture refusal on rotation {rotation}: {:?}",
                        candidate.refusal
                    )
                }),
            })
            .collect();
        assert_eq!(selections.len(), placed.len());
        let captured = doc
            .capture_library_entry(
                0,
                &selections,
                "captured".into(),
                "Saved group".into(),
                provenance.clone(),
            )
            .unwrap();
        assert_eq!(captured.primitives, original.primitives);
        assert_eq!(captured.provenance, provenance);
        assert_eq!(
            normalized(&before, None),
            normalized(&doc.snapshot().unwrap(), None)
        );
        let mut library = AnnotationLibrary::default();
        library.insert(captured.clone()).unwrap();
        assert_eq!(
            AnnotationLibrary::from_json(&library.to_json().unwrap()).unwrap(),
            library
        );
        let source_names: HashSet<_> = doc
            .annotation_details(0)
            .unwrap()
            .iter()
            .filter_map(|annotation| annotation.name.clone())
            .collect();
        doc.place_library_entry(
            1,
            &captured,
            LibraryPlacement {
                origin: (40.0, 60.0),
                scale: 1.0,
            },
            Some("Destination author"),
        )
        .unwrap();
        let destination = doc.annotation_details(1).unwrap();
        assert_eq!(destination.len(), captured.primitives.len());
        assert!(destination.iter().all(|annotation| annotation
            .name
            .as_ref()
            .is_some_and(|name| !source_names.contains(name))));
        assert_eq!(
            doc.capture_library_entry(
                1,
                &doc.library_capture_candidates(1)
                    .unwrap()
                    .iter()
                    .map(|row| LibraryCaptureSelection {
                        index: row.index,
                        fingerprint: row.fingerprint.clone().unwrap()
                    })
                    .collect::<Vec<_>>(),
                "again".into(),
                "Again".into(),
                provenance.clone()
            )
            .unwrap()
            .primitives,
            captured.primitives
        );
        doc.restore(before.clone()).unwrap();
        assert!(doc.annotation_details(1).unwrap().is_empty());
        assert!(doc
            .capture_library_entry(0, &[], "bad".into(), "Bad".into(), provenance.clone())
            .is_err());
        assert!(doc
            .capture_library_entry(
                0,
                &vec![selections[0].clone(); 17],
                "bad".into(),
                "Bad".into(),
                provenance.clone()
            )
            .is_err());
        assert!(doc
            .capture_library_entry(
                0,
                &[selections[0].clone(), selections[0].clone()],
                "bad".into(),
                "Bad".into(),
                provenance.clone()
            )
            .is_err());
        assert!(doc
            .capture_library_entry(
                0,
                &selections,
                "bad".into(),
                "Bad".into(),
                LibraryProvenance::default()
            )
            .is_err());
        let unsupported = doc
            .add_rich_free_text_annotation(
                0,
                None,
                PointRect {
                    left: 40.0,
                    bottom: 310.0,
                    right: 150.0,
                    top: 340.0,
                },
                "Saved text needs font proof",
                &Default::default(),
                None,
            )
            .unwrap();
        let mut mixed = selections.clone();
        mixed.push(LibraryCaptureSelection {
            index: unsupported,
            fingerprint: "0".repeat(64),
        });
        let mixed_before = doc.snapshot().unwrap();
        assert!(doc
            .capture_library_entry(0, &mixed, "bad".into(), "Bad".into(), provenance.clone())
            .is_err());
        let inventory = doc.library_capture_candidates(0).unwrap();
        assert!(
            inventory[unsupported].fingerprint.is_none()
                && inventory[unsupported].refusal.is_some()
        );
        assert!(inventory[unsupported]
            .refusal
            .as_deref()
            .unwrap()
            .contains("font resources"));
        assert_eq!(
            normalized(&mixed_before, None),
            normalized(&doc.snapshot().unwrap(), None)
        );
        let mut changed = lopdf::Document::load_mem(&before).unwrap();
        let page = changed.get_pages()[&1];
        let annotation = changed
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .as_reference()
            .unwrap();
        changed
            .get_dictionary_mut(annotation)
            .unwrap()
            .set("NM", lopdf::text_string("SuniPDF-changed-identity"));
        let mut changed_bytes = Vec::new();
        changed.save_to(&mut changed_bytes).unwrap();
        doc.restore(changed_bytes).unwrap();
        assert!(doc
            .capture_library_entry(
                0,
                &selections,
                "stale".into(),
                "Stale".into(),
                provenance.clone()
            )
            .unwrap_err()
            .to_string()
            .contains("changed"));
        changed.get_dictionary_mut(annotation).unwrap().set("A", lopdf::dictionary! {"S" => "URI", "URI" => Object::string_literal("https://example.invalid/")});
        let mut unsafe_bytes = Vec::new();
        changed.save_to(&mut unsafe_bytes).unwrap();
        doc.restore(unsafe_bytes).unwrap();
        assert!(doc.library_capture_candidates(0).unwrap()[0]
            .fingerprint
            .is_none());
        if rotation == 0 {
            let mut crowded = lopdf::Document::load_mem(&before).unwrap();
            let page = crowded.get_pages()[&1];
            let first = crowded
                .get_dictionary(page)
                .unwrap()
                .get(b"Annots")
                .unwrap()
                .as_array()
                .unwrap()[0]
                .as_reference()
                .unwrap();
            let template = crowded.get_dictionary(first).unwrap().clone();
            let mut annotations = Vec::new();
            for index in 0..513 {
                let mut annotation = template.clone();
                annotation.set(
                    "NM",
                    lopdf::text_string(&format!("SuniPDF-crowded-{index}")),
                );
                annotations.push(Object::Reference(crowded.add_object(annotation)));
            }
            crowded
                .get_dictionary_mut(page)
                .unwrap()
                .set("Annots", Object::Array(annotations));
            let mut bytes = Vec::new();
            crowded.save_to(&mut bytes).unwrap();
            doc.restore(bytes).unwrap();
            assert!(doc
                .library_capture_candidates(0)
                .unwrap_err()
                .to_string()
                .contains("512"));
            assert!(
                doc.capture_library_entry(
                    0,
                    &[selections[0].clone()],
                    "crowded".into(),
                    "Crowded".into(),
                    provenance.clone()
                )
                .unwrap_err()
                .to_string()
                .contains("512"),
                "direct capture must enforce the same page limit as the inventory"
            );
        }
    }
}
