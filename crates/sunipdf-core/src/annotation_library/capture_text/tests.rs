use super::*;
use crate::annotation_library::{
    AnnotationLibrary, LibraryCaptureSelection, LibraryEntry, LibraryPlacement, LibraryProvenance,
};
use crate::{annot_rich::RichFreeTextStyle, Engine, FontFamily, TextStyle};
use std::path::Path;

const SOURCE: &[u8] = include_bytes!("../../../tests/fixtures/two-pages.pdf");

fn provenance() -> LibraryProvenance {
    LibraryProvenance {
        source: Some("Locally authored text".into()),
        ..Default::default()
    }
}

fn fixture(rotation: i32) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(SOURCE).unwrap();
    let page = pdf.get_pages()[&1];
    let page = pdf.get_dictionary_mut(page).unwrap();
    page.set("Rotate", rotation);
    page.set(
        "CropBox",
        crate::annot_write::nums(&[100.25, 50.5, 500.25, 650.5]),
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn rect() -> PointRect {
    PointRect {
        left: 20.0,
        bottom: 40.0,
        right: 280.0,
        top: 140.0,
    }
}

fn captured(document: &Document<'_>) -> LibraryEntry {
    let candidate = document.library_capture_candidates(0).unwrap().remove(0);
    let selection = LibraryCaptureSelection {
        index: 0,
        fingerprint: candidate
            .fingerprint
            .unwrap_or_else(|| panic!("native text refused: {:?}", candidate.refusal)),
    };
    document
        .capture_library_entry(
            0,
            &[selection],
            "saved-text".into(),
            "Saved text".into(),
            provenance(),
        )
        .unwrap()
}

fn annotation_id(pdf: &lopdf::Document, index: usize) -> lopdf::ObjectId {
    let page = pdf.get_pages()[&1];
    resolve(
        pdf,
        pdf.get_dictionary(page).unwrap().get(b"Annots").unwrap(),
    )
    .unwrap()
    .as_array()
    .unwrap()[index]
        .as_reference()
        .unwrap()
}

fn ap_id(pdf: &lopdf::Document, annotation: lopdf::ObjectId) -> lopdf::ObjectId {
    field(pdf, pdf.get_dictionary(annotation).unwrap(), b"AP")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"N")
        .unwrap()
        .as_reference()
        .unwrap()
}

#[test]
fn saved_native_text_round_trips_all_standard_fonts_and_rotated_crops() {
    let engine = crate::test_engine();
    for rotation in [0, 90, 180, 270] {
        for family in [
            FontFamily::Helvetica,
            FontFamily::Times,
            FontFamily::Courier,
        ] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let source = fixture(rotation);
                let mut document = engine
                    .open_bytes(source.clone(), None, Path::new("source.pdf"))
                    .unwrap();
                let text = "Résumé café à vérifier\nSecond line with several words that wrap inside the text box.";
                let style = RichFreeTextStyle {
                    text: TextStyle {
                        family,
                        bold,
                        italic,
                        size_pt: 12.0,
                        rgb: (30, 70, 150),
                        opacity: 0.65,
                        ..Default::default()
                    },
                    fill_rgb: Some((240, 230, 210)),
                    border_pt: 1.25,
                };
                document
                    .add_rich_free_text_annotation(
                        0,
                        None,
                        rect(),
                        text,
                        &style,
                        Some("Original author"),
                    )
                    .unwrap();
                let saved = document.snapshot().unwrap();
                let expected_pixels = document.render_page(0, 1.0).unwrap().rgba;
                drop(document);
                let document = engine
                    .open_bytes(saved.clone(), None, Path::new("saved.pdf"))
                    .unwrap();
                let entry = captured(&document);
                assert!(crate::annot_rich::same_document(
                    &saved,
                    &document.snapshot().unwrap(),
                    None
                )
                .unwrap());
                assert_eq!(entry.primitives.len(), 1);
                match &entry.primitives[0] {
                    LibraryPrimitive::FreeText {
                        rect,
                        text: actual,
                        style: actual_style,
                    } => {
                        assert_eq!(*rect, [0.0, 0.0, 260.0, 100.0]);
                        assert_eq!(actual, text);
                        assert_eq!(actual_style.placed(1.0), style);
                    }
                    _ => panic!("native text primitive"),
                }
                let original = document.capture_original_annotation(0, 0).unwrap();
                assert!(
                    original.primitive.is_none() && original.refusal.is_some(),
                    "revision transfer scope stays unchanged"
                );
                let mut library = AnnotationLibrary::default();
                library.insert(entry.clone()).unwrap();
                let json = library.to_json().unwrap();
                let encoded = String::from_utf8(json.clone()).unwrap();
                assert!(
                    !encoded.contains("source.pdf")
                        && !encoded.contains("Original author")
                        && !encoded.contains("Resources")
                );
                let library = AnnotationLibrary::from_json(&json).unwrap();
                assert_eq!(library.entries()[0], entry);
                let mut destination = engine
                    .open_bytes(source, None, Path::new("destination.pdf"))
                    .unwrap();
                let before = destination.snapshot().unwrap();
                let placement = LibraryPlacement {
                    origin: (20.0, 40.0),
                    scale: 1.0,
                };
                if family == FontFamily::Helvetica && !bold && !italic {
                    let preview = destination
                        .preview_rich_annotation(
                            0,
                            &crate::annot_rich::RichAnnotation::LibraryPlacement {
                                entry: entry.clone(),
                                placement,
                            },
                            Some("Current author"),
                        )
                        .unwrap();
                    assert!(preview.region.width > 0 && preview.region.height > 0);
                    assert!(crate::annot_rich::same_document(
                        &before,
                        &destination.snapshot().unwrap(),
                        None
                    )
                    .unwrap());
                    destination
                        .commit_rich_annotation_preview(preview.prepared.unwrap())
                        .unwrap();
                } else {
                    destination
                        .place_library_entry(0, &entry, placement, Some("Current author"))
                        .unwrap();
                }
                assert_eq!(
                    destination.render_page(0, 1.0).unwrap().rgba,
                    expected_pixels,
                    "rendered text, wrapping, color and opacity survive placement"
                );
                let placed = destination.snapshot().unwrap();
                let old_name = document.annotation_details(0).unwrap()[0].name.clone();
                let new_name = destination.annotation_details(0).unwrap()[0].name.clone();
                assert_ne!(old_name, new_name);
                destination.restore(before).unwrap();
                assert!(destination.annotation_details(0).unwrap().is_empty());
                destination.restore(placed).unwrap();
                assert_eq!(captured(&destination).primitives, entry.primitives);
            }
        }
    }
    native_refusals(&engine);
}

fn native_refusals(engine: &Engine) {
    let mut document = engine
        .open_bytes(fixture(0), None, Path::new("source.pdf"))
        .unwrap();
    document
        .add_rich_free_text_annotation(
            0,
            None,
            rect(),
            "Native text",
            &RichFreeTextStyle::default(),
            None,
        )
        .unwrap();
    let snapshot = document.snapshot().unwrap();
    let (pdf, page, geometry) = document.capture_source(0).unwrap();
    let index = annotation_id(&pdf, 0);
    let base = pdf.get_dictionary(index).unwrap();
    assert!(capture(&document, &pdf, base, &geometry).is_ok());
    type Mutation = fn(&mut lopdf::Document, lopdf::ObjectId);
    let mutations: &[(&str, Mutation)] = &[
        ("foreign identity", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("NM", lopdf::text_string("foreign"));
        }),
        ("legacy text", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().remove(b"SuniPDFRich");
        }),
        ("different contents", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("Contents", lopdf::text_string("Modified text"));
        }),
        ("different size", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("DA", Object::string_literal("/FXF1 19 Tf 0 0 0 rg"));
        }),
        ("noncanonical DA", |pdf, id| {
            let dict = pdf.get_dictionary_mut(id).unwrap();
            let mut da = dict.get(b"DA").unwrap().as_str().unwrap().to_vec();
            da.extend_from_slice(b" ");
            dict.set("DA", Object::string_literal(da));
        }),
        ("rich contents", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("RC", Object::string_literal("<body>Rich</body>"));
        }),
        ("callout", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("CL", crate::annot_write::nums(&[1.0, 2.0, 3.0, 4.0]));
        }),
        ("changed opacity", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().set("CA", 0.5);
        }),
        ("changed border", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("BS", crate::annot_write::border_style(1.0, &[]));
        }),
        ("changed fill", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("C", crate::annot_write::rgb_obj((255, 200, 100)));
        }),
        ("review state", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("StateModel", lopdf::text_string("Review"));
        }),
        ("alignment", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().set("Q", 1);
        }),
        ("extra rotation", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().set("Rotate", 90);
        }),
        ("business fields", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("SuniPDFRegister", dictionary! {});
        }),
        ("measurement", |pdf, id| {
            pdf.get_dictionary_mut(id)
                .unwrap()
                .set("SuniPDFMeasurement", dictionary! {});
        }),
        ("action", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().set(
                "A",
                dictionary! {"S"=>"URI","URI"=>Object::string_literal("https://example.invalid/")},
            );
        }),
        ("locked", |pdf, id| {
            pdf.get_dictionary_mut(id).unwrap().set("F", 132);
        }),
        ("altered appearance", |pdf, id| {
            let ap = ap_id(pdf, id);
            pdf.get_object_mut(ap)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .set_content(b"q Q".to_vec());
        }),
        ("appearance metadata", |pdf, id| {
            let ap = ap_id(pdf, id);
            pdf.get_object_mut(ap)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .dict
                .set("Metadata", dictionary! {});
        }),
        ("appearance size", |pdf, id| {
            let ap = ap_id(pdf, id);
            pdf.get_object_mut(ap)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .dict
                .set("BBox", crate::annot_write::nums(&[0.0, 0.0, 260.01, 100.0]));
        }),
        ("oversized appearance", |pdf, id| {
            let ap = ap_id(pdf, id);
            pdf.get_object_mut(ap)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .set_content(vec![b' '; 2 * 1024 * 1024]);
        }),
        ("unused resource", |pdf, id| {
            let ap = ap_id(pdf, id);
            let resources = field(
                pdf,
                &pdf.get_object(ap).unwrap().as_stream().unwrap().dict,
                b"Resources",
            )
            .unwrap()
            .as_dict()
            .unwrap();
            let mut resources = resources.clone();
            resources.set("Pattern", dictionary! {});
            pdf.get_object_mut(ap)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .dict
                .set("Resources", resources);
        }),
        ("altered font", |pdf, id| {
            mutate_font(
                pdf,
                id,
                b"BaseFont",
                Object::Name(b"Helvetica-Bold".to_vec()),
            );
        }),
        ("embedded font", |pdf, id| {
            mutate_font(
                pdf,
                id,
                b"FontDescriptor",
                Object::Dictionary(
                    dictionary! {"FontFile2" => Object::Stream(Stream::new(dictionary!{}, b"unsupported font".to_vec()))},
                ),
            );
        }),
        ("CID font", |pdf, id| {
            mutate_font(pdf, id, b"Subtype", Object::Name(b"Type0".to_vec()));
        }),
        ("Type3 font", |pdf, id| {
            mutate_font(pdf, id, b"Subtype", Object::Name(b"Type3".to_vec()));
        }),
        ("changed encoding", |pdf, id| {
            mutate_font(
                pdf,
                id,
                b"Encoding",
                Object::Dictionary(
                    dictionary! {"Differences"=>vec![65.into(),Object::Name(b"B".to_vec())]},
                ),
            );
        }),
    ];
    for (label, mutate) in mutations {
        let mut changed = pdf.clone();
        mutate(&mut changed, index);
        assert!(
            capture(
                &document,
                &changed,
                changed.get_dictionary(index).unwrap(),
                &geometry
            )
            .is_err(),
            "{label}"
        );
    }
    let entry = captured(&document);
    let candidate = document.library_capture_candidates(0).unwrap().remove(0);
    let selection = LibraryCaptureSelection {
        index: 0,
        fingerprint: candidate.fingerprint.unwrap(),
    };
    document
        .add_rich_free_text_annotation(
            0,
            Some(0),
            rect(),
            "Different content",
            &RichFreeTextStyle::default(),
            None,
        )
        .unwrap();
    assert!(document
        .capture_library_entry(
            0,
            &[selection.clone()],
            "stale".into(),
            "Stale".into(),
            provenance()
        )
        .is_err());
    document.restore(snapshot.clone()).unwrap();
    document
        .add_reply(0, 0, "Follow up", Some("Reviewer"))
        .unwrap();
    assert!(document.library_capture_candidates(0).unwrap()[0]
        .fingerprint
        .is_none());
    document.restore(snapshot.clone()).unwrap();
    let legacy = document
        .add_free_text_annotation(
            0,
            PointRect {
                left: 20.0,
                bottom: 180.0,
                right: 280.0,
                top: 220.0,
            },
            "Legacy text",
            &Default::default(),
            None,
        )
        .unwrap();
    assert!(document.library_capture_candidates(0).unwrap()[legacy]
        .fingerprint
        .is_none());
    let before = document.snapshot().unwrap();
    assert!(document
        .capture_library_entry(
            0,
            &[
                selection,
                LibraryCaptureSelection {
                    index: legacy,
                    fingerprint: "0".repeat(64)
                }
            ],
            "mixed".into(),
            "Mixed".into(),
            provenance()
        )
        .is_err());
    assert!(
        crate::annot_rich::same_document(&before, &document.snapshot().unwrap(), None).unwrap()
    );
    let mut bad_group = entry;
    bad_group.primitives.push(LibraryPrimitive::FreeText {
        rect: [0.0, 0.0, 5.0, 5.0],
        text: "Cannot fit".into(),
        style: LibraryTextStyle::default(),
    });
    assert!(document
        .place_library_entry(
            0,
            &bad_group,
            LibraryPlacement {
                origin: (30.0, 40.0),
                scale: 1.0
            },
            None
        )
        .is_err());
    assert!(
        crate::annot_rich::same_document(&before, &document.snapshot().unwrap(), None).unwrap()
    );
    assert_eq!(pdf.get_pages()[&1], page);
}

fn mutate_font(pdf: &mut lopdf::Document, annotation: lopdf::ObjectId, key: &[u8], value: Object) {
    let ap = ap_id(pdf, annotation);
    let resources = field(
        pdf,
        &pdf.get_object(ap).unwrap().as_stream().unwrap().dict,
        b"Resources",
    )
    .unwrap()
    .as_dict()
    .unwrap();
    let mut resources = resources.clone();
    let mut fonts = field(pdf, &resources, b"Font")
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let (name, font) = fonts.iter().next().unwrap();
    let name = name.clone();
    let mut font = resolve(pdf, font).unwrap().as_dict().unwrap().clone();
    font.set(key, value);
    fonts.set(name, font);
    resources.set("Font", fonts);
    pdf.get_object_mut(ap)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .dict
        .set("Resources", resources);
}

#[test]
fn portable_text_translation_refuses_precision_loss_without_guessing_a_layout() {
    assert!(preserve_dimensions([0.0, 0.0, 260.0, 100.0], [20.0, 40.0, 280.0, 140.0], 1.0).is_ok());
    assert!(preserve_dimensions([0.0, 0.0, 260.0, 100.0], [20.0, 40.0, 150.0, 90.0], 0.5).is_ok());
    assert!(
        preserve_dimensions([0.0, 0.0, 10.1, 100.0], [4000.0, 40.0, 4010.1, 140.0], 1.0).is_err()
    );
    assert!(preserve_dimensions(
        [0.0, 0.0, 260.0, 100.0],
        [20.0, 40.0, f32::INFINITY, 140.0],
        1.0
    )
    .is_err());
    let primitive = LibraryPrimitive::FreeText {
        rect: [0.0, 0.0, 10.1, 40.0],
        text: "Text".into(),
        style: LibraryTextStyle::default(),
    };
    let original_primitive = primitive.clone();
    assert!(primitive.translated(4000.0, 0.0).is_err());
    assert_eq!(primitive, original_primitive);
    assert!(primitive.translated(0.0, 40.0).is_ok());
    let mut entry = LibraryEntry {
        id: "precision".into(),
        name: "Precision".into(),
        provenance: provenance(),
        primitives: vec![LibraryPrimitive::FreeText {
            rect: [0.001, 0.0, 50.17, 40.0],
            text: "Text".into(),
            style: LibraryTextStyle::default(),
        }],
    };
    entry.primitives.push(LibraryPrimitive::Stamp {
        rect: [0.0003, 50.0, 100.0, 80.0],
        stamp: super::super::LibraryStamp::Draft,
    });
    let original = entry.clone();
    // The generic guard also applies to authored drafts and mixed groups.
    assert!(entry.normalized().is_err());
    assert_eq!(entry, original);
}

#[test]
fn native_color_conversion_requires_exact_byte_colors() {
    for byte in 0..=255u8 {
        assert_eq!(color(f32::from(byte) / 255.0).unwrap(), byte);
    }
    for invalid in [f32::NAN, f32::INFINITY, -0.01, 1.01, 0.5] {
        assert!(color(invalid).is_err());
    }
}
