use super::*;
use crate::PointRect;
use lopdf::dictionary;

fn fixture() -> (lopdf::Document, ObjectId) {
    let mut pdf = lopdf::Document::with_version("1.7");
    let page = pdf.add_object(dictionary! { "Type" => "Page", "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()] });
    let pages = pdf
        .add_object(dictionary! { "Type" => "Pages", "Count" => 1, "Kids" => vec![page.into()] });
    pdf.get_dictionary_mut(page).unwrap().set("Parent", pages);
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    (pdf, page)
}

fn geometry(rotation: Rotation) -> PageGeometry {
    PageGeometry {
        bbox: PointRect {
            left: 100.0,
            bottom: 50.0,
            right: 500.0,
            top: 650.0,
        },
        rotation,
    }
}

fn destination(page: ObjectId, mode: &str, rest: Vec<Object>) -> Object {
    let mut array = vec![
        Object::Reference(page),
        Object::Name(mode.as_bytes().to_vec()),
    ];
    array.extend(rest);
    Object::Array(array)
}

#[test]
fn xyz_retains_null_axes_and_zoom_through_crop_and_all_rotations() {
    let (pdf, page) = fixture();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let geo = geometry(rotation);
        for (x, y) in [
            (None, Some(0.0)),
            (Some(10.0), None),
            (None, None),
            (Some(4.0), Some(9.0)),
        ] {
            let desired = Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x,
                    y,
                    zoom: Some(1.25),
                },
            };
            let raw = authored(
                &pdf,
                &LinkTarget::Destination(desired),
                &Dictionary::new(),
                &|_| Ok(geo),
            )
            .unwrap();
            assert_eq!(
                Resolver::new(&pdf).destination(&raw, &|_| Ok(geo)).unwrap(),
                desired
            );
        }
        let raw = destination(page, "XYZ", vec![Object::Null, 0.into(), 0.into()]);
        let actual = Resolver::new(&pdf).destination(&raw, &|_| Ok(geo)).unwrap();
        let axes = display_axes(geo, None, Some(0.0));
        assert_eq!(
            actual.view,
            DestinationView::Xyz {
                x: axes.0,
                y: axes.1,
                zoom: None
            }
        );
        for zoom in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let invalid = LinkTarget::Destination(Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x: None,
                    y: None,
                    zoom: Some(zoom),
                },
            });
            assert!(authored(&pdf, &invalid, &Dictionary::new(), &|_| Ok(geo)).is_err());
        }
    }
}

#[test]
fn fith_null_and_zero_are_distinct_and_unrepresentable_axes_are_refused() {
    let (pdf, page) = fixture();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let geo = geometry(rotation);
        for y in [Object::Null, Object::Real(0.0), Object::Real(400.0)] {
            let raw = destination(page, "FitH", vec![y]);
            let decoded = Resolver::new(&pdf).destination(&raw, &|_| Ok(geo)).unwrap();
            assert_eq!(
                authored(
                    &pdf,
                    &LinkTarget::Destination(decoded),
                    &Dictionary::new(),
                    &|_| Ok(geo)
                )
                .unwrap(),
                raw
            );
        }
        let bad = if matches!(rotation, Rotation::R90 | Rotation::R270) {
            DestinationView::FitWidth {
                x: None,
                y: Some(10.0),
            }
        } else {
            DestinationView::FitWidth {
                x: Some(10.0),
                y: None,
            }
        };
        assert!(authored(
            &pdf,
            &LinkTarget::Destination(Destination { page: 0, view: bad }),
            &Dictionary::new(),
            &|_| Ok(geo)
        )
        .is_err());
    }
}

#[test]
fn named_tree_and_legacy_keys_preserve_original_bytes_and_string_form() {
    let (mut pdf, page) = fixture();
    let key = vec![0xfe, 0xff, 0, b'C', 0, b'a', 0, b'f', 0, 0xe9];
    let raw = destination(page, "XYZ", vec![Object::Null, 200.into(), Object::Null]);
    let leaf =
        pdf.add_object(dictionary! { "Names" => vec![Object::string_literal(key.clone()), raw] });
    pdf.catalog_mut()
        .unwrap()
        .set("Names", dictionary! { "Dests" => leaf });
    let resolver = Resolver::new(&pdf);
    assert_eq!(resolver.names().unwrap(), vec!["Caf\u{e9}"]);
    let original = Object::String(key.clone(), lopdf::StringFormat::Hexadecimal);
    let annotation = dictionary! { "Dest" => original.clone() };
    let (target, precise) = resolver
        .link(&annotation, &|_| Ok(geometry(Rotation::R90)))
        .unwrap();
    assert_eq!(target, LinkTarget::Named("Caf\u{e9}".into()));
    assert!(matches!(
        precise.unwrap().view,
        DestinationView::Xyz {
            x: Some(_),
            y: None,
            zoom: None
        }
    ));
    assert_eq!(
        authored(&pdf, &target, &annotation, &|_| Ok(geometry(Rotation::R90))).unwrap(),
        original
    );
    assert_eq!(
        authored(&pdf, &target, &Dictionary::new(), &|_| Ok(geometry(
            Rotation::R0
        )))
        .unwrap()
        .as_str()
        .unwrap(),
        key
    );
    pdf.catalog_mut().unwrap().set(
        "Dests",
        dictionary! { "legacy" => destination(page, "Fit", vec![]) },
    );
    assert_eq!(
        authored(
            &pdf,
            &LinkTarget::Named("legacy".into()),
            &Dictionary::new(),
            &|_| Ok(geometry(Rotation::R0))
        )
        .unwrap(),
        Object::Name(b"legacy".to_vec())
    );
}

#[test]
fn duplicate_decoded_names_cycles_and_remote_actions_are_refused() {
    let (mut pdf, page) = fixture();
    pdf.catalog_mut().unwrap().set(
        "Dests",
        dictionary! {
            "Cafe" => destination(page, "Fit", vec![]),
            vec![0xfe, 0xff, 0, b'C', 0, b'a', 0, b'f', 0, b'e'] => destination(page, "Fit", vec![])
        },
    );
    assert!(Resolver::new(&pdf).names().is_err());
    assert!(authored(
        &pdf,
        &LinkTarget::Named("Cafe".into()),
        &Dictionary::new(),
        &|_| Ok(geometry(Rotation::R0))
    )
    .is_err());
    pdf.catalog_mut().unwrap().set("Dests", dictionary! { "one" => Object::Name(b"two".to_vec()), "two" => Object::Name(b"one".to_vec()) });
    assert!(authored(
        &pdf,
        &LinkTarget::Named("one".into()),
        &Dictionary::new(),
        &|_| Ok(geometry(Rotation::R0))
    )
    .is_err());
    let cyclic = pdf.new_object_id();
    pdf.objects.insert(cyclic, Object::Reference(cyclic));
    assert!(Resolver::new(&pdf)
        .link(&dictionary! { "Dest" => cyclic }, &|_| Ok(geometry(
            Rotation::R0
        )))
        .is_err());
    let remote = dictionary! { "A" => dictionary! { "S" => "GoToR", "D" => destination(page, "XYZ", vec![0.into(), 0.into(), 1.into()]) } };
    assert!(Resolver::new(&pdf)
        .link(&remote, &|_| Ok(geometry(Rotation::R0)))
        .is_err());
    assert!(Resolver::new(&pdf).fallback_page(&remote).is_none());
}
