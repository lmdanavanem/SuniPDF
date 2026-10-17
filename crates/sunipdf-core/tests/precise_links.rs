//! Native save/reopen and retained-viewport evidence for precise local link editing.
use lopdf::{dictionary, Object};
use std::path::{Path, PathBuf};
use sunipdf_core::{
    security, Destination, DestinationView, Engine, EngineError, LinkTarget, Protection, Rotation,
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/two-pages.pdf")
}

fn source(rotation: i32, locked: bool) -> Vec<u8> {
    let mut pdf = lopdf::Document::load(fixture()).unwrap();
    let pages = pdf.get_pages();
    let target = *pages.get(&2).unwrap();
    let page = *pages.get(&1).unwrap();
    pdf.get_dictionary_mut(target).unwrap().set(
        "CropBox",
        vec![100.into(), 50.into(), 700.into(), 550.into()],
    );
    pdf.get_dictionary_mut(target)
        .unwrap()
        .set("Rotate", rotation);
    let destination = Object::Array(vec![
        target.into(),
        Object::Name(b"XYZ".to_vec()),
        Object::Null,
        0.into(),
        Object::Null,
    ]);
    let key = Object::String(
        vec![0xfe, 0xff, 0, b'C', 0, b'a', 0, b'f', 0, 0xe9],
        lopdf::StringFormat::Hexadecimal,
    );
    let leaf = pdf.add_object(dictionary! { "Names" => vec![key, destination] });
    pdf.catalog_mut()
        .unwrap()
        .set("Names", dictionary! { "Dests" => leaf });
    let annotation = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![20.into(), 20.into(), 100.into(), 40.into()],
        "F" => if locked { 132 } else { 4 }, "Contents" => Object::string_literal("Keep this comment"),
        "T" => Object::string_literal("Reviewer"), "NM" => Object::string_literal("external-link"),
        "Dest" => vec![Object::Reference(page), Object::Name(b"Fit".to_vec())]
    });
    pdf.get_dictionary_mut(page)
        .unwrap()
        .set("Annots", vec![Object::Reference(annotation)]);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn precise_links_keep_viewport_semantics_comments_locks_undo_and_encryption() {
    let engine =
        Engine::new().expect("Run scripts/fetch-pdfium.ps1 before native regression tests");
    let directory = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        for encrypted in [false, true] {
            let original = source(rotation, false);
            let bytes = if encrypted {
                security::protect(
                    &original,
                    None,
                    &Protection {
                        user_password: "link-reader".into(),
                        owner_password: "link-owner".into(),
                        aes256: true,
                        ..Default::default()
                    },
                )
                .unwrap()
            } else {
                original
            };
            let password = encrypted.then_some("link-owner");
            let path = directory
                .path()
                .join(format!("links-{rotation}-{encrypted}.pdf"));
            let mut document = engine.open_bytes(bytes, password, &path).unwrap();
            let old = document.annotation_details(0).unwrap().remove(0);
            let before = document.snapshot().unwrap();
            let target = Destination {
                page: 1,
                view: DestinationView::Xyz {
                    x: None,
                    y: Some(120.0),
                    zoom: Some(1.5),
                },
            };
            document
                .set_link_target(0, 0, &LinkTarget::Destination(target))
                .unwrap();
            let after = document.snapshot().unwrap();
            let detail = document.annotation_details(0).unwrap().remove(0);
            assert_eq!(detail.link, Some(LinkTarget::Destination(target)));
            assert_eq!(
                (detail.rect, detail.contents, detail.author, detail.name),
                (old.rect, old.contents, old.author, old.name)
            );
            let link = document.page_links(0).unwrap().remove(0);
            assert_eq!(link.target, LinkTarget::Page(1));
            assert_eq!(link.destination, Some(target));
            document.restore(before).unwrap();
            assert_eq!(
                document.annotation_details(0).unwrap()[0].link,
                Some(LinkTarget::Page(0))
            );
            document.restore(after).unwrap();
            assert_eq!(
                document.annotation_details(0).unwrap()[0].link,
                Some(LinkTarget::Destination(target))
            );
            let unchanged = document.snapshot().unwrap();
            let invalid = LinkTarget::Destination(Destination {
                page: 1,
                view: DestinationView::Xyz {
                    x: None,
                    y: None,
                    zoom: Some(0.0),
                },
            });
            assert!(document.set_link_target(0, 0, &invalid).is_err());
            assert_eq!(document.snapshot().unwrap(), unchanged);
            document.save_as(&path).unwrap();
            let reopened = engine.open(&path, password).unwrap();
            assert_eq!(reopened.properties().encrypted, encrypted);
            assert_eq!(reopened.page_links(0).unwrap()[0].destination, Some(target));
            drop(reopened);

            assert_eq!(
                document.named_link_destinations().unwrap(),
                vec!["Caf\u{e9}"]
            );
            document
                .set_link_target(0, 0, &LinkTarget::Named("Caf\u{e9}".into()))
                .unwrap();
            let named = document.annotation_details(0).unwrap().remove(0);
            assert_eq!(named.link, Some(LinkTarget::Named("Caf\u{e9}".into())));
            let geo = document.page_geometry(1).unwrap();
            let named_view = match geo.rotation {
                Rotation::R0 => DestinationView::Xyz {
                    x: None,
                    y: Some(-50.0),
                    zoom: None,
                },
                Rotation::R90 => DestinationView::Xyz {
                    x: Some(-50.0),
                    y: None,
                    zoom: None,
                },
                Rotation::R180 => DestinationView::Xyz {
                    x: None,
                    y: Some(550.0),
                    zoom: None,
                },
                Rotation::R270 => DestinationView::Xyz {
                    x: Some(550.0),
                    y: None,
                    zoom: None,
                },
            };
            assert_eq!(
                document.page_links(0).unwrap()[0].destination,
                Some(Destination {
                    page: 1,
                    view: named_view
                })
            );
            let fit_view = match named_view {
                DestinationView::Xyz { x, y, .. } => DestinationView::FitWidth { x, y },
                _ => unreachable!(),
            };
            document
                .set_link_target(
                    0,
                    0,
                    &LinkTarget::Destination(Destination {
                        page: 1,
                        view: fit_view,
                    }),
                )
                .unwrap();
            document.save_as(&path).unwrap();
            assert_eq!(
                engine.open(&path, password).unwrap().page_links(0).unwrap()[0].destination,
                Some(Destination {
                    page: 1,
                    view: fit_view
                })
            );
            let retain = Destination {
                page: 1,
                view: DestinationView::FitWidth { x: None, y: None },
            };
            document
                .set_link_target(0, 0, &LinkTarget::Destination(retain))
                .unwrap();
            assert_eq!(document.page_links(0).unwrap()[0].destination, Some(retain));
            assert!(document
                .set_link_target(0, 0, &LinkTarget::Uri("javascript:unsafe".into()))
                .is_err());
        }
    }
    let mut locked = engine
        .open_bytes(source(90, true), None, &directory.path().join("locked.pdf"))
        .unwrap();
    let unchanged = locked.snapshot().unwrap();
    assert!(matches!(
        locked.set_link_target(0, 0, &LinkTarget::Page(1)),
        Err(EngineError::SecurityRestricted)
    ));
    assert_eq!(locked.snapshot().unwrap(), unchanged);
}
