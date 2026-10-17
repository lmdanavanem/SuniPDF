use super::*;
fn input() -> Vec<u8> {
    include_bytes!("../tests/fixtures/two-pages.pdf").to_vec()
}
fn loaded(bytes: &[u8]) -> lopdf::Document {
    crate::security::load(bytes, None).unwrap()
}
#[test]
fn bookmark_authoring_preserves_hierarchy_destinations_and_inert_actions() {
    let original = input();
    let bytes = edit(
        &original,
        None,
        &BookmarkEdit::Add {
            parent: None,
            title: "Parent é".into(),
            page: 0,
        },
    )
    .unwrap();
    let bytes = edit(
        &bytes,
        None,
        &BookmarkEdit::Add {
            parent: Some(0),
            title: "Child".into(),
            page: 1,
        },
    )
    .unwrap();
    let mut pdf = loaded(&bytes);
    let mut tree = Tree::read(&pdf).unwrap();
    assert_eq!(tree.nodes.len(), 2);
    let mut action = Dictionary::new();
    action.set("S", "Launch");
    action.set("F", Object::string_literal("never-executed.exe"));
    tree.nodes[1].dict.remove(b"Dest");
    tree.nodes[1].dict.set("A", action.clone());
    tree.write(&mut pdf).unwrap();
    let bytes = crate::security::save(pdf).unwrap();
    let bytes = edit(
        &bytes,
        None,
        &BookmarkEdit::Rename {
            index: 1,
            title: "Renamed".into(),
        },
    )
    .unwrap();
    let bytes = edit(
        &bytes,
        None,
        &BookmarkEdit::Move {
            index: 1,
            parent: None,
            position: 0,
        },
    )
    .unwrap();
    let pdf = loaded(&bytes);
    let tree = Tree::read(&pdf).unwrap();
    let first = tree.order()[0];
    assert_eq!(
        tree.nodes[first].dict.get(b"A").unwrap(),
        &Object::Dictionary(action)
    );
    assert_eq!(tree.roots.len(), 2);
    assert_eq!(
        lopdf::decode_text_string(tree.nodes[first].dict.get(b"Title").unwrap()).unwrap(),
        "Renamed"
    );
    let bytes = edit(
        &bytes,
        None,
        &BookmarkEdit::Destination { index: 0, page: 1 },
    )
    .unwrap();
    let pdf = loaded(&bytes);
    let tree = Tree::read(&pdf).unwrap();
    let first = tree.order()[0];
    assert!(!tree.nodes[first].dict.has(b"A"));
    let dest = tree.nodes[first]
        .dict
        .get(b"Dest")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(dest[0].as_reference().unwrap(), pdf.get_pages()[&2]);
    let bytes = edit(&bytes, None, &BookmarkEdit::Delete { index: 0 }).unwrap();
    assert_eq!(Tree::read(&loaded(&bytes)).unwrap().nodes.len(), 1);
    assert_eq!(original, input());
}
#[test]
fn cycles_bad_indices_oversized_titles_and_invalid_moves_are_refused_atomically() {
    let original = edit(
        &input(),
        None,
        &BookmarkEdit::Add {
            parent: None,
            title: "Root".into(),
            page: 0,
        },
    )
    .unwrap();
    let original = edit(
        &original,
        None,
        &BookmarkEdit::Add {
            parent: Some(0),
            title: "Child".into(),
            page: 1,
        },
    )
    .unwrap();
    for operation in [
        BookmarkEdit::Move {
            index: 0,
            parent: Some(1),
            position: 0,
        },
        BookmarkEdit::Rename {
            index: 0,
            title: "x".repeat(1025),
        },
        BookmarkEdit::Add {
            parent: None,
            title: "bad\0title".into(),
            page: 0,
        },
        BookmarkEdit::Destination { index: 1, page: 99 },
        BookmarkEdit::Delete { index: 99 },
    ] {
        assert!(edit(&original, None, &operation).is_err());
    }
    let mut pdf = loaded(&original);
    let root = pdf
        .catalog()
        .unwrap()
        .get(b"Outlines")
        .unwrap()
        .as_reference()
        .unwrap();
    let first = pdf
        .get_dictionary(root)
        .unwrap()
        .get(b"First")
        .unwrap()
        .as_reference()
        .unwrap();
    pdf.get_dictionary_mut(first).unwrap().set("Next", first);
    assert!(Tree::read(&pdf).is_err());
    let corrupt = crate::security::save(pdf).unwrap();
    assert!(edit(&corrupt, None, &BookmarkEdit::Delete { index: 0 }).is_err());
}
