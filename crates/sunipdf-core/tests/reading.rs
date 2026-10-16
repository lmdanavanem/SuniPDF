use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::Path;
use sunipdf_core::annot::ReadingPoint;
use sunipdf_core::{Document, Engine, EngineError};

fn reading_pdf() -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier", "Encoding" => "WinAnsiEncoding"
    });
    let mut kids = Vec::new();
    for text in [
        b"Alpha l'amour \xe9t\xe9 l\x92amour omega".as_slice(),
        b"Beta second".as_slice(),
    ] {
        let mut content = b"BT /F1 12 Tf 1 0 0 1 100 600 Tm (".to_vec();
        content.extend_from_slice(text);
        content.extend_from_slice(b") Tj ET");
        let contents = pdf.add_object(Stream::new(dictionary! {}, content));
        let page = pdf.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(),0.into(),612.into(),792.into()],
            "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}}, "Contents" => contents
        });
        kids.push(Object::Reference(page));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type" => "Pages", "Kids" => kids, "Count" => 2}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn point(doc: &Document<'_>, page: usize, needle: &str, last: bool) -> ReadingPoint {
    let r = doc.search_page(page, needle, false).unwrap()[0].rects[0];
    ReadingPoint {
        page,
        x: if last { r.right - 0.1 } else { r.left + 0.1 },
        y: (r.bottom + r.top) / 2.0,
    }
}

#[test]
fn reading_ranges_words_rotations_and_crop() {
    let engine = Engine::new().unwrap();
    let doc = engine
        .open_bytes(reading_pdf(), None, Path::new("reading.pdf"))
        .unwrap();
    let start = point(&doc, 0, "Alpha", false);
    let end = point(&doc, 0, "omega", true);
    let forward = doc.reading_selection(start, end, false).unwrap();
    assert_eq!(forward.pages[0].1.text, "Alpha l'amour été l’amour omega");
    let reverse = doc.reading_selection(end, start, false).unwrap();
    assert_eq!(reverse.pages, forward.pages);
    assert!(reverse.anchor > reverse.focus);
    for word in ["l'amour", "été", "l’amour"] {
        let r = doc.search_page(0, word, false).unwrap()[0].rects[0];
        let p = ReadingPoint {
            page: 0,
            x: (r.left + r.right) / 2.0,
            y: (r.bottom + r.top) / 2.0,
        };
        assert_eq!(
            doc.reading_selection(p, p, true).unwrap().pages[0].1.text,
            word
        );
    }
    let end = point(&doc, 1, "second", true);
    let selected = doc.reading_selection(start, end, false).unwrap();
    assert_eq!(
        selected
            .pages
            .iter()
            .map(|(_, s)| s.text.as_str())
            .collect::<Vec<_>>(),
        ["Alpha l'amour été l’amour omega", "Beta second"]
    );
    assert_eq!(
        doc.reading_selection(end, start, false).unwrap().pages,
        selected.pages
    );
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(doc
            .reading_selection(
                ReadingPoint {
                    x: invalid,
                    ..start
                },
                end,
                false
            )
            .is_err());
        assert!(doc
            .reading_selection(start, ReadingPoint { y: invalid, ..end }, false)
            .is_err());
    }
    assert!(matches!(
        doc.reading_selection(start, ReadingPoint { page: 3, ..end }, false),
        Err(EngineError::PageOutOfRange(3))
    ));

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/boxes.pdf");
    let doc = engine.open(&path, None).unwrap();
    for page in [0, 2] {
        // These fixtures have R90/R270 and non-zero CropBox origins.
        let r = doc.search_page(page, "BOXES", false).unwrap()[0].rects[0];
        let p = ReadingPoint {
            page,
            x: (r.left + r.right) / 2.0,
            y: (r.bottom + r.top) / 2.0,
        };
        let selection = doc.reading_selection(p, p, true).unwrap();
        assert_eq!(selection.pages[0].1.text, "BOXES");
        assert!(selection.pages[0]
            .1
            .rects
            .iter()
            .all(|s| s.top - s.bottom > s.right - s.left));
        let s = selection.pages[0].1.rects[0];
        // Selection uses loose character boxes; search reports tighter glyph bounds.
        assert!(
            s.left <= r.left && s.right >= r.right && s.bottom <= r.bottom && s.top >= r.top,
            "{s:?} / {r:?}"
        );
        assert!(
            s.right - s.left <= r.right - r.left + 24.0
                && s.top - s.bottom <= r.top - r.bottom + 24.0,
            "selection must stay within one font em of the word: {s:?} / {r:?}"
        );
    }
}
