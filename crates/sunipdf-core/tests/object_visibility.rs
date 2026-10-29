use lopdf::{dictionary, Document as Pdf, Object, Stream};
use sunipdf_core::{Engine, PixelRect};

fn fixture(rotation: i64) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Image", "Width"=>1, "Height"=>1,
            "ColorSpace"=>"DeviceRGB", "BitsPerComponent"=>8,
        },
        vec![0, 180, 0],
    ));
    let form = pdf.add_object(Stream::new(dictionary! {
        "Type"=>"XObject", "Subtype"=>"Form", "BBox"=>vec![0.into(),0.into(),40.into(),40.into()],
        "Resources"=>dictionary!{},
    }, b"0 0 1 rg 0 0 40 40 re f".to_vec()));
    let content = pdf.add_object(Stream::new(dictionary!{},
        b"q 1 0 0 rg 20 20 40 40 re f Q BT /F1 12 Tf 20 150 Td (Visible text) Tj ET q 40 0 0 40 110 20 cm /Im0 Do Q q 1 0 0 1 110 100 cm /Fm0 Do Q".to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),200.into(),200.into()],
        "CropBox"=>vec![10.into(),10.into(),190.into(),190.into()], "Rotate"=>rotation,
        "Contents"=>content, "Resources"=>dictionary! {
            "Font"=>dictionary!{"F1"=>font}, "XObject"=>dictionary!{"Im0"=>image,"Fm0"=>form},
        },
    });
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>1, "Kids"=>vec![Object::Reference(page)]}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn reader_visibility_hides_and_restores_every_object_kind_without_changing_saved_content() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let source = temp.path().join(format!("source-{rotation}.pdf"));
        let bytes = fixture(rotation);
        std::fs::write(&source, &bytes).unwrap();
        let doc = engine.open(&source, None).unwrap();
        let objects = doc.page_objects(0).unwrap();
        assert_eq!(objects.len(), 4);
        let original = doc.render_page(0, 1.0).unwrap();
        let snapshot = doc.snapshot().unwrap();
        for object in &objects {
            let hidden = doc.render_page_hiding(0, 1.0, &[object.index]).unwrap();
            assert_ne!(
                hidden.rgba, original.rgba,
                "object {} at {rotation} degrees",
                object.index
            );
            assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original.rgba);
            let b = object.bounds;
            assert!(doc
                .object_at_visible(
                    0,
                    (b.left + b.right) / 2.0,
                    (b.bottom + b.top) / 2.0,
                    &[object.index]
                )
                .unwrap()
                .is_none());
        }
        let all: Vec<_> = objects.iter().map(|object| object.index).collect();
        let blank = doc.render_page_hiding(0, 1.0, &all).unwrap();
        assert!(blank
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [255, 255, 255, 255]));
        let region = PixelRect::new(0, 0, original.width, original.height);
        assert_eq!(
            doc.render_region_hiding(0, 1.0, region, &all).unwrap().rgba,
            blank.rgba
        );
        let invalid = [vec![4], vec![usize::MAX], vec![0, 4]];
        for indices in invalid {
            assert!(doc.render_page_hiding(0, 1.0, &indices).is_err());
            assert!(doc.render_region_hiding(0, 1.0, region, &indices).is_err());
            assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, original.rgba);
        }
        assert_eq!(doc.page_objects(0).unwrap(), objects);
        let before = Pdf::load_mem(&snapshot).unwrap();
        let after = Pdf::load_mem(&doc.snapshot().unwrap()).unwrap();
        assert_eq!(
            before.get_page_content(before.get_pages()[&1]),
            after.get_page_content(after.get_pages()[&1])
        );
        let saved = temp.path().join(format!("saved-{rotation}.pdf"));
        doc.save_as(&saved).unwrap();
        let saved = engine.open(&saved, None).unwrap();
        assert_eq!(saved.page_objects(0).unwrap().len(), objects.len());
        assert_eq!(saved.render_page(0, 1.0).unwrap().rgba, original.rgba);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }
}
