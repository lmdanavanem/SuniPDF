use super::*;
use std::sync::atomic::AtomicBool;
use sunipdf_core::annot_write::ReviewStatus;
use sunipdf_core::revision_transfer::*;

fn selections(doc: &sunipdf_core::Document<'_>) -> Vec<LibraryCaptureSelection> {
    doc.library_capture_candidates(0)
        .unwrap()
        .into_iter()
        .map(|row| LibraryCaptureSelection {
            index: row.index,
            fingerprint: row
                .fingerprint
                .unwrap_or_else(|| panic!("native stamp refused: {:?}", row.refusal)),
        })
        .collect()
}

fn provenance() -> LibraryProvenance {
    LibraryProvenance {
        author: None,
        source: Some("Locally authored review marks".into()),
        license: None,
    }
}

fn capture(
    doc: &sunipdf_core::Document<'_>,
    selections: &[LibraryCaptureSelection],
) -> LibraryEntry {
    doc.capture_library_entry(
        0,
        selections,
        "saved-review".into(),
        "Saved review marks".into(),
        provenance(),
    )
    .unwrap()
}

pub(super) fn round_trip_with_alpha_and_revision_isolation(engine: &Engine) {
    fractional_native_capture_round_trips_on_cropped_rotated_pages(engine);
    let mut original = entry();
    original
        .primitives
        .retain(|p| !matches!(p, LibraryPrimitive::FreeText { .. }));
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_fn(4, 2, |x, _| {
        image::Rgba([220, 40, 70, [255, 180, 90, 0][x as usize]])
    })
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
    for primitive in &mut original.primitives {
        if let LibraryPrimitive::Image {
            png: embedded,
            opacity,
            ..
        } = primitive
        {
            *embedded = EmbeddedPng::from_png_bytes(png.get_ref()).unwrap();
            *opacity = 0.4;
        }
    }
    for (index, stamp) in [
        LibraryStamp::Draft,
        LibraryStamp::Confidential,
        LibraryStamp::Rejected,
    ]
    .into_iter()
    .enumerate()
    {
        original.primitives.push(LibraryPrimitive::Stamp {
            rect: [
                0.0,
                230.0 + index as f32 * 30.0,
                150.0,
                255.0 + index as f32 * 30.0,
            ],
            stamp,
        });
    }
    let mut opaque = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2, 3, image::Rgba([40, 80, 200, 255]))
        .write_to(&mut opaque, image::ImageFormat::Png)
        .unwrap();
    original.primitives.push(LibraryPrimitive::Image {
        rect: [290.0, 60.0, 350.0, 130.0],
        png: EmbeddedPng::from_png_bytes(opaque.get_ref()).unwrap(),
        opacity: 1.0,
    });
    let placement = LibraryPlacement {
        origin: (20.0, 20.0),
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
        let mut source = Vec::new();
        pdf.save_to(&mut source).unwrap();
        let mut doc = engine
            .open_bytes(source.clone(), None, Path::new("private-source.pdf"))
            .unwrap();
        doc.place_library_entry(0, &original, placement, Some("Original author"))
            .unwrap();
        let saved = doc.snapshot().unwrap();
        drop(doc);
        let mut doc = engine
            .open_bytes(saved.clone(), None, Path::new("saved-source.pdf"))
            .unwrap();
        let selected = selections(&doc);
        let captured = capture(&doc, &selected);
        assert_eq!(
            captured.provenance,
            provenance(),
            "source author and license must not be inferred"
        );
        assert_eq!(captured.primitives.len(), original.primitives.len());
        let images: Vec<_> = captured
            .primitives
            .iter()
            .filter_map(|primitive| {
                if let LibraryPrimitive::Image { png, opacity, .. } = primitive {
                    assert_eq!(*opacity, 1.0, "saved opacity is already baked into alpha");
                    Some(
                        image::load_from_memory(&png.png_bytes().unwrap())
                            .unwrap()
                            .into_rgba8(),
                    )
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(images[0].dimensions(), (4, 2));
        assert_eq!(images[0].get_pixel(0, 0)[3], 102);
        assert_eq!(images[0].get_pixel(1, 0)[3], 72);
        assert_eq!(images[0].get_pixel(2, 0)[3], 36);
        assert_eq!(images[0].get_pixel(3, 0).0, [0, 0, 0, 0]);
        assert!(images[1].pixels().all(|p| p[3] == 255));
        assert_eq!(
            normalized(&saved, None),
            normalized(&doc.snapshot().unwrap(), None)
        );

        let mut library = AnnotationLibrary::default();
        library.insert(captured.clone()).unwrap();
        let json = library.to_json().unwrap();
        let encoded = String::from_utf8(json.clone()).unwrap();
        assert!(!encoded.contains("private-source.pdf") && !encoded.contains("Original author"));
        let imported = AnnotationLibrary::from_json(&json).unwrap();
        let imported = &imported.entries()[0];
        assert_eq!(imported, &captured);
        let names: HashSet<_> = doc
            .annotation_details(0)
            .unwrap()
            .iter()
            .filter_map(|d| d.name.clone())
            .collect();
        let mut destination = engine
            .open_bytes(source, None, Path::new("destination.pdf"))
            .unwrap();
        let undo = destination.snapshot().unwrap();
        let preview = destination
            .preview_rich_annotation(
                0,
                &RichAnnotation::LibraryPlacement {
                    entry: imported.clone(),
                    placement,
                },
                Some("New author"),
            )
            .unwrap();
        assert_eq!(
            normalized(&undo, None),
            normalized(&destination.snapshot().unwrap(), None)
        );
        assert!(preview.region.width > 0 && preview.region.height > 0);
        destination
            .commit_rich_annotation_preview(preview.prepared.unwrap())
            .unwrap();
        assert!(destination
            .annotation_details(0)
            .unwrap()
            .iter()
            .all(|d| !names.contains(d.name.as_ref().unwrap())
                && d.author.as_deref() == Some("New author")));
        let placed = destination.snapshot().unwrap();
        let reopened = engine
            .open_bytes(placed, None, Path::new("reopened.pdf"))
            .unwrap();
        assert_eq!(
            capture(&reopened, &selections(&reopened)).primitives,
            captured.primitives
        );
        assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, reopened.render_page(0, 1.0).unwrap().rgba,
            "capture must preserve alpha, letterboxing and native appearances at rotation {rotation}");
        destination.restore(undo).unwrap();
        assert!(destination.annotation_details(0).unwrap().is_empty());

        if rotation == 0 {
            revision_transfer_still_refuses_stamps(engine, &saved, &selected);
            stamp_history_and_copy_restrictions(engine, &saved, &selected);
            let stamp = selected
                .iter()
                .find(|row| {
                    matches!(
                        captured.primitives[row.index],
                        LibraryPrimitive::Stamp { .. }
                    )
                })
                .unwrap();
            let mut changed = lopdf::Document::load_mem(&saved).unwrap();
            let annotation = annotation_id(&changed, stamp.index);
            changed
                .get_dictionary_mut(annotation)
                .unwrap()
                .set("NM", lopdf::text_string("SuniPDF-new-identity"));
            let mut bytes = Vec::new();
            changed.save_to(&mut bytes).unwrap();
            doc.restore(bytes).unwrap();
            assert!(doc
                .capture_library_entry(0, &selected, "stale".into(), "Stale".into(), provenance())
                .unwrap_err()
                .to_string()
                .contains("changed"));
        }
    }
}

fn fractional_native_capture_round_trips_on_cropped_rotated_pages(engine: &Engine) {
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(3, 2, image::Rgba([220, 40, 80, 180]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
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
        let mut source = Vec::new();
        pdf.save_to(&mut source).unwrap();
        let mut doc = engine
            .open_bytes(source.clone(), None, Path::new("fractional-source.pdf"))
            .unwrap();
        doc.add_stamp_annotation(
            0,
            PointRect {
                left: 20.1,
                bottom: 30.2,
                right: 148.2,
                top: 91.3,
            },
            sunipdf_core::annot_rich::StampKind::Approved,
            None,
        )
        .unwrap();
        doc.add_image_stamp_annotation_bytes(
            0,
            PointRect {
                left: 180.1,
                bottom: 130.2,
                right: 280.2,
                top: 211.3,
            },
            png.get_ref(),
            0.5,
            None,
        )
        .unwrap();
        let saved = doc.snapshot().unwrap();
        drop(doc);
        let doc = engine
            .open_bytes(saved, None, Path::new("fractional-saved.pdf"))
            .unwrap();
        let captured = capture(&doc, &selections(&doc));
        assert_eq!(captured.primitives.len(), 2);
        let mut library = AnnotationLibrary::default();
        library.insert(captured).unwrap();
        let imported = AnnotationLibrary::from_json(&library.to_json().unwrap()).unwrap();
        let mut destination = engine
            .open_bytes(source, None, Path::new("fractional-reuse.pdf"))
            .unwrap();
        destination
            .place_library_entry(
                0,
                &imported.entries()[0],
                LibraryPlacement {
                    origin: (20.1, 30.2),
                    scale: 1.0,
                },
                None,
            )
            .unwrap();
        let reused = engine
            .open_bytes(
                destination.snapshot().unwrap(),
                None,
                Path::new("fractional-reopened.pdf"),
            )
            .unwrap();
        let recaptured = capture(&reused, &selections(&reused));
        assert!(matches!(
            recaptured.primitives[0],
            LibraryPrimitive::Stamp {
                stamp: LibraryStamp::Approved,
                ..
            }
        ));
        let LibraryPrimitive::Image {
            png: pixels,
            opacity,
            ..
        } = &recaptured.primitives[1]
        else {
            panic!("missing captured image")
        };
        assert_eq!(*opacity, 1.0);
        assert!(image::load_from_memory(&pixels.png_bytes().unwrap())
            .unwrap()
            .into_rgba8()
            .pixels()
            .all(|pixel| pixel.0 == [220, 40, 80, 90]));
        assert_eq!(
            doc.render_page(0, 1.0).unwrap().rgba,
            reused.render_page(0, 1.0).unwrap().rgba,
            "fractional native capture changes rendered pixels at rotation {rotation}"
        );
    }
}

fn annotation_id(pdf: &lopdf::Document, index: usize) -> lopdf::ObjectId {
    pdf.get_dictionary(pdf.get_pages()[&1])
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()[index]
        .as_reference()
        .unwrap()
}

fn revision_transfer_still_refuses_stamps(
    engine: &Engine,
    saved: &[u8],
    library: &[LibraryCaptureSelection],
) {
    let directory = tempfile::tempdir().unwrap();
    let old = directory.path().join("old.pdf");
    let new = directory.path().join("new.pdf");
    std::fs::write(&old, saved).unwrap();
    std::fs::write(&new, SOURCE).unwrap();
    let cancel = AtomicBool::new(false);
    let inspection = inspect_transfer_candidates(
        engine,
        &old,
        &new,
        TransferPasswords::default(),
        &[(1, 1)],
        &cancel,
    )
    .unwrap();
    let candidates = &inspection.pages[0].candidates;
    assert!(candidates
        .iter()
        .any(|row| row.subtype == "Square" && row.fingerprint.is_some()));
    let stamps: Vec<_> = candidates
        .iter()
        .filter(|row| row.subtype == "Stamp")
        .collect();
    assert_eq!(stamps.len(), 6);
    assert!(stamps
        .iter()
        .all(|row| row.fingerprint.is_none() && row.refusal.is_some()));
    for stamp in stamps {
        let request = RevisionTransferRequest {
            version: 1,
            old_sha256: inspection.old_sha256.clone(),
            new_sha256: inspection.new_sha256.clone(),
            pairs: vec![TransferPagePair {
                old_page: 1,
                new_page: 1,
                old_sheet_id: None,
                new_sheet_id: None,
                confirmed: true,
                alignment: TransferAlignment::Identity,
                selections: vec![TransferSelection {
                    index: stamp.index,
                    fingerprint: library[stamp.index].fingerprint.clone(),
                }],
            }],
            acknowledge_repeated_transfer: false,
        };
        assert!(
            prepare_revision_transfer(
                engine,
                &old,
                &new,
                TransferPasswords::default(),
                &request,
                &cancel
            )
            .is_err(),
            "library fingerprints must not authorize revision transfer of stamps or images"
        );
    }
}

fn stamp_history_and_copy_restrictions(
    engine: &Engine,
    saved: &[u8],
    selected: &[LibraryCaptureSelection],
) {
    let mut doc = engine
        .open_bytes(saved.to_vec(), None, Path::new("threads.pdf"))
        .unwrap();
    let stamp_indices: Vec<_> = doc
        .library_capture_candidates(0)
        .unwrap()
        .into_iter()
        .filter(|row| row.subtype == "Stamp")
        .map(|row| row.index)
        .collect();
    for index in stamp_indices {
        for history in [false, true] {
            doc.restore(saved.to_vec()).unwrap();
            if history {
                doc.set_review_status(0, index, ReviewStatus::Accepted, None)
                    .unwrap();
            } else {
                doc.add_reply(0, index, "Keep this reply attached", None)
                    .unwrap();
            }
            let inventory = doc.library_capture_candidates(0).unwrap();
            assert!(inventory[index].fingerprint.is_none());
            assert!(inventory[index]
                .refusal
                .as_deref()
                .unwrap()
                .contains("replies or review history"));
            let before = doc.snapshot().unwrap();
            assert!(doc
                .capture_library_entry(
                    0,
                    selected,
                    "blocked".into(),
                    "Blocked".into(),
                    provenance()
                )
                .is_err());
            assert_eq!(
                normalized(&before, None),
                normalized(&doc.snapshot().unwrap(), None)
            );
        }
    }
    for aes256 in [false, true] {
        let protected = security::protect(
            saved,
            None,
            &Protection {
                user_password: "capture-reader".into(),
                owner_password: "capture-owner".into(),
                allow_copy: false,
                allow_modify: true,
                allow_print: true,
                allow_annotate: true,
                aes256,
            },
        )
        .unwrap();
        let doc = engine
            .open_bytes(
                protected,
                Some("capture-reader"),
                Path::new("restricted.pdf"),
            )
            .unwrap();
        assert!(matches!(
            doc.library_capture_candidates(0),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            doc.capture_library_entry(
                0,
                selected,
                "blocked".into(),
                "Blocked".into(),
                provenance()
            ),
            Err(EngineError::SecurityRestricted)
        ));
    }
}
