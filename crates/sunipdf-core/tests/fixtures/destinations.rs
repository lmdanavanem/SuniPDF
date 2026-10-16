use lopdf::{dictionary, Object};

pub const PER_PAGE: usize = 12;

pub fn document() -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(include_bytes!("boxes.pdf")).unwrap();
    let pages: Vec<_> = pdf.get_pages().values().copied().collect();
    assert_eq!(pages.len(), 4);
    let mut annotations: Vec<Object> = Vec::new();
    let mut outlines = Vec::new();
    let outline_root = pdf.new_object_id();
    let mut old_names = lopdf::Dictionary::new();
    let mut tree_names = Vec::new();

    for (page, id) in pages.iter().copied().enumerate() {
        let target = pdf.get_dictionary_mut(id).unwrap();
        target.set("Rotate", (page * 90) as i64);
        target.set(
            "CropBox",
            vec![100.into(), 50.into(), 500.into(), 650.into()],
        );
        let xyz = |x: Object, y: Object, zoom: Object| {
            Object::Array(vec![id.into(), "XYZ".into(), x, y, zoom])
        };
        let old_name = format!("Old{page}");
        let tree_name = format!("Tree{page}");
        old_names.set(old_name.as_bytes(), xyz(150.into(), 450.into(), 2.into()));
        tree_names.push(Object::string_literal(tree_name.as_bytes()));
        tree_names.push(Object::Dictionary(dictionary! {
            "D" => xyz(300.into(), Object::Null, Object::Null),
        }));
        let rows: Vec<(Object, bool)> = vec![
            (xyz(200.into(), 400.into(), 1.5.into()), false),
            (xyz(Object::Null, 400.into(), 0.into()), true),
            (Object::Name(old_name.into_bytes()), false),
            (Object::string_literal(tree_name.as_bytes()), false),
            (Object::Name(tree_name.into_bytes()), true),
            (Object::Array(vec![id.into(), "Fit".into()]), false),
            (
                Object::Array(vec![id.into(), "FitH".into(), 400.into()]),
                true,
            ),
            (
                Object::Array(vec![id.into(), "FitH".into(), Object::Null]),
                false,
            ),
            (
                Object::Array(vec![id.into(), "FitH".into(), 0.into()]),
                false,
            ),
            (
                Object::Array(vec![id.into(), "FitV".into(), 100.into()]),
                false,
            ),
            (
                Object::Array(vec![id.into(), "XYZ".into(), 100.into()]),
                false,
            ),
            (xyz(200.into(), 400.into(), (-2).into()), true),
        ];
        assert_eq!(rows.len(), PER_PAGE);
        for (row, (destination, action)) in rows.into_iter().enumerate() {
            let mut link = dictionary! {
                "Type" => "Annot", "Subtype" => "Link",
                "Rect" => vec![110.into(), 100.into(), 160.into(), 120.into()],
            };
            let mut bookmark = dictionary! {
                "Title" => Object::string_literal(format!("Page {page} destination {row}")),
                "Parent" => outline_root,
            };
            for dictionary in [&mut link, &mut bookmark] {
                if action {
                    dictionary.set("A", dictionary! {"S"=>"GoTo", "D"=>destination.clone()});
                } else {
                    dictionary.set("Dest", destination.clone());
                }
            }
            annotations.push(pdf.add_object(link).into());
            outlines.push(pdf.add_object(bookmark));
        }
    }

    for action in [
        dictionary! {"S"=>"GoTo", "D"=>vec![999.into(), "Fit".into()]},
        dictionary! {"S"=>"GoTo", "D"=>Object::string_literal("missing")},
        dictionary! {"S"=>"GoToR", "F"=>Object::string_literal("external.pdf"), "D"=>vec![0.into(), "Fit".into()]},
        dictionary! {"S"=>"Launch", "F"=>Object::string_literal("calc.exe")},
        dictionary! {"S"=>"JavaScript", "JS"=>Object::string_literal("app.alert(1)")},
        dictionary! {"S"=>"URI", "URI"=>Object::string_literal("https://example.org")},
        dictionary! {"S"=>"URI", "URI"=>Object::string_literal("javascript:alert(1)")},
    ] {
        let link = dictionary! {
            "Type"=>"Annot", "Subtype"=>"Link",
            "Rect"=>vec![110.into(), 140.into(), 160.into(), 160.into()], "A"=>action.clone(),
        };
        annotations.push(pdf.add_object(link).into());
        outlines.push(pdf.add_object(dictionary! {
            "Title"=>Object::string_literal("Unsafe or unresolved"), "Parent"=>outline_root, "A"=>action,
        }));
    }
    for (index, id) in outlines.iter().copied().enumerate() {
        let bookmark = pdf.get_dictionary_mut(id).unwrap();
        if index > 0 {
            bookmark.set("Prev", outlines[index - 1]);
        }
        if index + 1 < outlines.len() {
            bookmark.set("Next", outlines[index + 1]);
        }
    }
    pdf.objects.insert(
        outline_root,
        dictionary! {
            "Type"=>"Outlines", "First"=>outlines[0], "Last"=>*outlines.last().unwrap(),
            "Count"=>outlines.len() as i64,
        }
        .into(),
    );
    pdf.get_dictionary_mut(pages[0])
        .unwrap()
        .set("Annots", annotations);
    let catalog = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let catalog = pdf.get_dictionary_mut(catalog).unwrap();
    catalog.set("Outlines", outline_root);
    catalog.set("Dests", old_names);
    catalog.set(
        "Names",
        dictionary! {"Dests"=>dictionary! {"Names"=>tree_names}},
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
