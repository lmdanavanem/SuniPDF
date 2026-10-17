use super::*;
use std::collections::BTreeMap;

fn stored_parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut parts = BTreeMap::new();
    let mut offset = 0;
    while bytes.get(offset..offset + 4) == Some(b"PK\x03\x04") {
        let short = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as usize;
        let long = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        assert_eq!(short(offset + 8), 0);
        let size = long(offset + 18);
        assert_eq!(size, long(offset + 22));
        let name_size = short(offset + 26);
        let extra = short(offset + 28);
        let start = offset + 30 + name_size + extra;
        let name = String::from_utf8(bytes[offset + 30..offset + 30 + name_size].to_vec()).unwrap();
        assert!(parts
            .insert(name, bytes[start..start + size].to_vec())
            .is_none());
        offset = start + size;
    }
    assert_eq!(&bytes[offset..offset + 4], b"PK\x01\x02");
    for (name, content) in &parts {
        if name.ends_with(".xml") || name.ends_with(".rels") {
            for event in xml::reader::EventReader::new(content.as_slice()) {
                event.unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
    }
    parts
}

#[test]
fn xml_unicode_and_formula_like_cells_stay_literal() {
    assert_eq!(
        xml_text("Café <&> '😀'").unwrap(),
        "Café &lt;&amp;&gt; &apos;😀&apos;"
    );
    assert!(xml_text("bad\0control").is_err());
    assert!(Format::from_id("html").is_none());
}

#[test]
fn office_exports_validate_packages_preserve_sources_and_refuse_atomic_failures() {
    let engine = crate::Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let fixture = include_bytes!("../tests/fixtures/two-pages.pdf");
    let source_path = dir.path().join("source.pdf");
    std::fs::write(&source_path, fixture).unwrap();
    let mut doc = engine.open(&source_path, None).unwrap();
    for page in 0..doc.page_count() {
        let indices: Vec<_> = doc
            .page_objects(page)
            .unwrap()
            .iter()
            .map(|o| o.index)
            .collect();
        doc.delete_objects(page, &indices).unwrap();
        doc.add_text(
            page,
            72.0,
            doc.page_size(page).unwrap().height_pt - 72.0,
            "=1+1 & <literal>",
            &crate::TextStyle::default(),
        )
        .unwrap();
    }
    let before = doc.snapshot().unwrap();
    for format in [Format::Word, Format::Excel, Format::PowerPoint] {
        let output = dir.path().join(format!("export.{}", format.extension()));
        let mut progress = 0;
        let report = doc
            .export_office(&output, format, &cancel, |done, _| progress = done)
            .unwrap();
        assert_eq!(progress, 2);
        assert_eq!(report.pages, 2);
        let bytes = std::fs::read(&output).unwrap();
        let parts = stored_parts(&bytes);
        #[cfg(windows)]
        crate::office::validate_package(&output, format.extension(), &cancel, dir.path()).unwrap();
        match format {
            Format::Word => {
                assert!(String::from_utf8_lossy(&parts["word/document.xml"])
                    .contains("=1+1 &amp; &lt;literal&gt;"));
            }
            Format::Excel => {
                let xml = String::from_utf8_lossy(&parts["xl/worksheets/sheet1.xml"]);
                assert!(xml.contains("t=\"inlineStr\""));
                assert!(xml.contains("=1+1 &amp; &lt;literal&gt;"));
                assert!(!xml.contains("<f>"));
            }
            Format::PowerPoint => {
                for page in 0..doc.page_count() {
                    let png =
                        image::load_from_memory(&parts[&format!("ppt/media/page{}.png", page + 1)])
                            .unwrap()
                            .into_rgba8();
                    let rendered = doc.render_page(page, 150.0 / 72.0).unwrap();
                    assert_eq!(png.as_raw(), &rendered.rgba);
                    assert_eq!(png.width(), rendered.width);
                }
            }
        }
        assert!(matches!(
            doc.export_office(&output, format, &cancel, |_, _| {}),
            Err(EngineError::AlreadyExists(_))
        ));
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
    }
    assert_eq!(doc.snapshot().unwrap(), before);
    assert_eq!(std::fs::read(&source_path).unwrap(), fixture);
    cancel.store(true, Ordering::Relaxed);
    let cancelled = dir.path().join("cancel.docx");
    assert!(matches!(
        doc.export_office(&cancelled, Format::Word, &cancel, |_, _| {}),
        Err(EngineError::Cancelled)
    ));
    assert!(!cancelled.exists());
    cancel.store(false, Ordering::Relaxed);
    let interrupted = dir.path().join("interrupted.xlsx");
    assert!(matches!(
        doc.export_office(&interrupted, Format::Excel, &cancel, |done, _| {
            if done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        }),
        Err(EngineError::Cancelled)
    ));
    assert!(!interrupted.exists());
    assert_eq!(doc.snapshot().unwrap(), before);
    cancel.store(false, Ordering::Relaxed);
    doc.add_shape(
        0,
        crate::ShapeKind::Rectangle,
        crate::PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 30.0,
            top: 30.0,
        },
        &crate::ShapeStyle::default(),
    )
    .unwrap();
    let unsupported = dir.path().join("unsupported.docx");
    assert!(doc
        .export_office(&unsupported, Format::Word, &cancel, |_, _| {})
        .is_err());
    assert!(!unsupported.exists());
    let indices: Vec<_> = doc
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|object| object.index)
        .collect();
    doc.delete_objects(0, &indices).unwrap();
    doc.add_text(
        0,
        72.0,
        700.0,
        "Hidden white text",
        &crate::TextStyle {
            rgb: (255, 255, 255),
            ..Default::default()
        },
    )
    .unwrap();
    let white_before = doc.snapshot().unwrap();
    for format in [Format::Word, Format::Excel] {
        let output = dir.path().join(format!("hidden.{}", format.extension()));
        assert!(doc
            .export_office(&output, format, &cancel, |_, _| {})
            .is_err());
        assert!(!output.exists());
    }
    assert_eq!(doc.snapshot().unwrap(), white_before);
    assert_eq!(std::fs::read(&source_path).unwrap(), fixture);
}
