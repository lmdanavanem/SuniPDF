use lopdf::{dictionary, Document as Pdf, Object, Stream};
use sunipdf_core::{security, Engine, EngineError, Protection, ZOrder};

fn fixture() -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let streams: Vec<Object> = [
        "1 0 0 rg 20 20 100 100 re f",
        "0 1 0 rg 20 20 100 100 re f",
        "0 0 1 rg 20 20 100 100 re f",
        "BT /F1 12 Tf 10 160 Td (Retained text) Tj ET",
    ]
    .iter()
    .map(|text| {
        Object::Reference(pdf.add_object(Stream::new(dictionary! {}, text.as_bytes().to_vec())))
    })
    .collect();
    let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),200.into(),200.into()],
        "Contents"=>streams, "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}});
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
fn object_block_moves_preserve_streams_rendering_protection_and_reject_invalid_destinations() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("multiple-streams.pdf");
    let bytes = fixture();
    std::fs::write(&source, &bytes).unwrap();
    let nonce = temp.path().file_name().unwrap().to_string_lossy();
    let user = format!("opening-{nonce}");
    let owner = format!("owner-{nonce}");
    for encryption in [None, Some(false), Some(true)] {
        let input = temp.path().join(format!("input-{encryption:?}.pdf"));
        let original = encryption.map_or_else(
            || bytes.clone(),
            |aes256| {
                security::protect(
                    &bytes,
                    None,
                    &Protection {
                        user_password: user.clone(),
                        owner_password: owner.clone(),
                        aes256,
                        ..Default::default()
                    },
                )
                .unwrap()
            },
        );
        std::fs::write(&input, &original).unwrap();
        for (credential_index, credential) in
            [user.as_str(), owner.as_str()].into_iter().enumerate()
        {
            let mut doc = engine.open(&input, encryption.map(|_| credential)).unwrap();
            let objects = doc.page_objects(0).unwrap();
            assert_eq!(objects.len(), 4);
            let before = doc.page_objects(0).unwrap();
            for (indices, at) in [
                (vec![], 0),
                (vec![99], 0),
                (vec![0, 2], 3),
                (vec![0], usize::MAX),
            ] {
                assert!(doc.reorder_objects(0, &indices, ZOrder::At(at)).is_err());
                assert_eq!(doc.page_objects(0).unwrap(), before);
            }
            assert_eq!(
                doc.reorder_objects(0, &[0], ZOrder::At(0)).unwrap(),
                vec![0]
            );
            assert_eq!(doc.page_objects(0).unwrap(), before);
            assert_eq!(
                doc.reorder_objects(0, &[0], ZOrder::At(2)).unwrap(),
                vec![2]
            );
            let rendered = doc.render_page(0, 1.0).unwrap();
            let pixel = ((150 * rendered.width + 50) * 4) as usize;
            assert_eq!(&rendered.rgba[pixel..pixel + 3], &[255, 0, 0]);
            let output = temp
                .path()
                .join(format!("out-{encryption:?}-{credential_index}.pdf"));
            doc.save_as(&output).unwrap();
            let mut saved = engine
                .open(&output, encryption.map(|_| credential))
                .unwrap();
            assert!(saved.page_text(0).unwrap().contains("Retained text"));
            assert_eq!(saved.properties().encrypted, encryption.is_some());
            let before = saved.page_objects(0).unwrap();
            assert_eq!(
                saved.reorder_objects(0, &[2, 0, 2], ZOrder::At(1)).unwrap(),
                vec![1, 2]
            );
            let after = saved.page_objects(0).unwrap();
            for (old, new) in [(1, 0), (0, 1), (2, 2), (3, 3)] {
                assert_eq!(before[old].kind, after[new].kind);
                assert_eq!(before[old].bounds, after[new].bounds);
                assert_eq!(before[old].fill_rgb, after[new].fill_rgb);
                assert_eq!(before[old].text, after[new].text);
            }
        }
        if let Some(aes256) = encryption {
            let restricted = security::protect(
                &bytes,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    allow_modify: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
            let path = temp.path().join("restricted.pdf");
            std::fs::write(&path, &restricted).unwrap();
            for credential in [user.as_str(), owner.as_str()] {
                let mut doc = engine.open(&path, Some(credential)).unwrap();
                if !doc.properties().can_modify {
                    let before = doc.page_objects(0).unwrap();
                    assert!(matches!(
                        doc.reorder_objects(0, &[0], ZOrder::At(2)),
                        Err(EngineError::SecurityRestricted)
                    ));
                    assert_eq!(doc.page_objects(0).unwrap(), before);
                }
            }
        }
        assert_eq!(std::fs::read(&input).unwrap(), original);
    }
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
}
