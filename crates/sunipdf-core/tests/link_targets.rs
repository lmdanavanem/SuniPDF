use lopdf::{dictionary, Dictionary, Object};
use std::path::{Path, PathBuf};
use sunipdf_core::{security, Engine, EngineError, LinkTarget, Protection, TextStyle};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn annotations(pdf: &lopdf::Document) -> Vec<Dictionary> {
    let page = *pdf.get_pages().get(&1).unwrap();
    pdf.get_dictionary(page)
        .unwrap()
        .get_deref(b"Annots", pdf)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| match entry {
            Object::Reference(id) => pdf.get_dictionary(*id).unwrap().clone(),
            Object::Dictionary(dict) => dict.clone(),
            _ => panic!("invalid annotation"),
        })
        .collect()
}

fn source() -> (Vec<u8>, usize) {
    let mut pdf = lopdf::Document::load(fixture("acroform-links.pdf")).unwrap();
    let pages = pdf.get_pages();
    let page = pages[&1];
    let mut entries = pdf
        .get_dictionary(page)
        .unwrap()
        .get_deref(b"Annots", &pdf)
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    let start = entries.len();
    let link = dictionary! {
        "Type" => "Annot", "Subtype" => "Link",
        "Rect" => vec![20.into(), 20.into(), 110.into(), 40.into()],
        "Border" => vec![0.into(), 0.into(), 2.into()],
        "C" => vec![0.into(), 0.into(), 1.into()],
        "NM" => Object::string_literal("Foreign-link"),
        "Contents" => Object::string_literal("Keep the comment"),
        "CustomMetadata" => Object::string_literal("Keep this too"),
        "Dest" => vec![Object::Reference(pages[&2]), Object::Name(b"XYZ".to_vec()), 12.into(), 34.into(), 2.into()],
        "A" => dictionary! {"S" => "JavaScript", "JS" => Object::string_literal("old action"),
                              "Next" => dictionary! {"S" => "Launch", "F" => Object::string_literal("old.exe")}},
        "AA" => dictionary! {"D" => dictionary! {"S" => "JavaScript", "JS" => Object::string_literal("old trigger")}}
    };
    let indirect = pdf.add_object(link.clone());
    entries.push(Object::Reference(indirect));
    entries.push(Object::Dictionary(link.clone()));
    entries.push(dictionary! {"Type" => "Annot", "Subtype" => "Square", "Rect" => vec![10.into(), 10.into(), 20.into(), 20.into()]}.into());
    for flag in [
        Object::Integer(64),
        Object::Integer(128),
        Object::Integer(512),
        Object::Reference(pdf.add_object(Object::Integer(64))),
        Object::string_literal("invalid flag"),
        Object::Integer(-1024),
    ] {
        let mut locked = link.clone();
        locked.set("F", flag);
        entries.push(Object::Reference(pdf.add_object(locked)));
    }
    entries.push(Object::Reference(pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![20.into(), 20.into(), 30.into(), 30.into()],
        "IRT" => Object::Reference(indirect), "RT" => "R", "Contents" => Object::string_literal("Keep the reply")
    })));
    pdf.get_dictionary_mut(page)
        .unwrap()
        .set("Annots", Object::Array(entries));
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    (bytes, start)
}

fn decoded(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    // Encrypted object streams must be decrypted during parsing so their dictionaries are loaded.
    let pdf = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or("")),
    )
    .unwrap();
    assert!(!pdf.is_encrypted());
    pdf
}

fn unchanged(before: &[u8], after: &[u8], password: Option<&str>) {
    let mut before = decoded(before, password);
    let mut after = decoded(after, password);
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(before.trailer, after.trailer);
    assert_eq!(before.objects, after.objects);
}

fn qpdf(path: &Path, password: Option<&str>) {
    if let Some(executable) = std::env::var_os("SUNIPDF_QPDF") {
        let mut command = std::process::Command::new(executable);
        if let Some(password) = password {
            command.arg(format!("--password={password}"));
        }
        let output = command.arg("--check").arg(path).output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn retargeting_links_preserves_annotations_content_and_protection() {
    // All native scenarios share one process-global PDFium engine.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (original, start) = source();
    for (case, (aes, password)) in [
        (None, None),
        (Some(false), Some("user")),
        (Some(false), Some("owner")),
        (Some(true), Some("user")),
        (Some(true), Some("owner")),
    ]
    .into_iter()
    .enumerate()
    {
        let input = match aes {
            Some(aes256) => security::protect(
                &original,
                None,
                &Protection {
                    user_password: "user".into(),
                    owner_password: "owner".into(),
                    allow_print: false,
                    allow_copy: false,
                    allow_modify: false,
                    allow_annotate: false,
                    aes256,
                },
            )
            .unwrap(),
            None => original.clone(),
        };
        let source = temp.path().join(format!("source-{case}.pdf"));
        std::fs::write(&source, &input).unwrap();
        let mut doc = engine.open(&source, password).unwrap();
        doc.add_text(1, 40.0, 40.0, "Unsaved content", &TextStyle::default())
            .unwrap();
        let details = doc.annotation_details(0).unwrap();
        assert!(details[start].can_edit_link && details[start + 1].can_edit_link);
        assert!(!details[start + 2].can_edit_link);
        for detail in &details[start + 3..start + 9] {
            assert!(!detail.can_edit_link);
        }
        assert_eq!(details[start].replies, 1);
        let properties = doc.properties();
        let sizes = doc.info().page_sizes.clone();
        let form = doc.form_info().unwrap();
        let bookmarks = doc.bookmarks();
        let text: Vec<_> = (0..doc.page_count())
            .map(|page| doc.page_text(page).unwrap())
            .collect();
        let rendered: Vec<_> = (0..doc.page_count())
            .map(|page| doc.render_page(page, 0.25).unwrap().rgba)
            .collect();
        // PDFium generates missing form appearances on first read/render, before editing starts.
        let before = doc.snapshot().unwrap();
        let before_annotations = annotations(&decoded(&before, password));
        for (page, index, target) in [
            (2, start, LinkTarget::Page(0)),
            (0, usize::MAX, LinkTarget::Page(0)),
            (0, start + 2, LinkTarget::Page(0)),
            (0, start + 3, LinkTarget::Page(0)),
            (0, start + 4, LinkTarget::Page(0)),
            (0, start + 5, LinkTarget::Page(0)),
            (0, start + 6, LinkTarget::Page(0)),
            (0, start + 7, LinkTarget::Page(0)),
            (0, start + 8, LinkTarget::Page(0)),
            (0, start, LinkTarget::Page(2)),
            (0, start, LinkTarget::Page(usize::MAX)),
            (0, start, LinkTarget::Unsupported),
            (0, start, LinkTarget::Uri("javascript:alert(1)".into())),
            (0, start, LinkTarget::Uri("file:///C:/private.pdf".into())),
            (0, start, LinkTarget::Uri("https://a/\0bad".into())),
            (
                0,
                start,
                LinkTarget::Uri(format!("https://a/{}", "x".repeat(2048))),
            ),
        ] {
            let result = doc.set_link_target(page, index, &target);
            assert!(
                result.is_err(),
                "accepted invalid target {target:?} at {page}/{index}"
            );
            if page == 0 && (start + 3..start + 9).contains(&index) {
                assert!(matches!(result, Err(EngineError::SecurityRestricted)));
            }
            unchanged(&before, &doc.snapshot().unwrap(), password);
        }
        doc.set_link_target(
            0,
            start,
            &LinkTarget::Uri("  https://example.org/new?x=1#section  ".into()),
        )
        .unwrap();
        doc.set_link_target(0, start + 1, &LinkTarget::Page(0))
            .unwrap();
        let uri = LinkTarget::Uri("https://example.org/new?x=1#section".into());
        let links = doc.page_links(0).unwrap();
        assert_eq!(links.iter().find(|l| l.index == start).unwrap().target, uri);
        assert_eq!(
            links.iter().find(|l| l.index == start + 1).unwrap().target,
            LinkTarget::Page(0)
        );
        let after_annotations = annotations(&decoded(&doc.snapshot().unwrap(), password));
        assert_eq!(after_annotations.len(), before_annotations.len());
        for (index, (before, after)) in before_annotations
            .iter()
            .zip(&after_annotations)
            .enumerate()
        {
            let mut before = before.clone();
            let mut after = after.clone();
            if index == start || index == start + 1 {
                assert!(after.get(b"AA").is_err());
                assert!(after
                    .get(if index == start {
                        b"Dest".as_slice()
                    } else {
                        b"A"
                    })
                    .is_err());
                for key in [b"A".as_slice(), b"Dest", b"AA", b"M"] {
                    before.remove(key);
                    after.remove(key);
                }
            }
            assert_eq!(after, before, "annotation {index} changed unexpectedly");
        }
        assert_eq!(doc.properties(), properties);
        assert_eq!(doc.info().page_sizes, sizes);
        assert_eq!(doc.form_info().unwrap(), form);
        assert_eq!(doc.bookmarks(), bookmarks);
        assert_eq!(doc.annotation_details(0).unwrap()[start].replies, 1);
        for (page, expected) in rendered.iter().enumerate() {
            assert_eq!(&doc.render_page(page, 0.25).unwrap().rgba, expected);
            assert_eq!(doc.page_text(page).unwrap(), text[page]);
        }
        let output = temp.path().join(format!("retargeted-{case}.pdf"));
        doc.save_as(&output).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), input);
        for reopen_password in if aes.is_some() {
            vec![Some("user"), Some("owner")]
        } else {
            vec![None]
        } {
            qpdf(&output, reopen_password);
            let reopened = engine.open(&output, reopen_password).unwrap();
            assert_eq!(reopened.properties().encrypted, aes.is_some());
            let details = reopened.annotation_details(0).unwrap();
            assert_eq!(details[start].link, Some(uri.clone()));
            assert_eq!(details[start + 1].link, Some(LinkTarget::Page(0)));
            assert_eq!(details[start].replies, 1);
            assert_eq!(reopened.form_info().unwrap(), form);
            for (page, expected) in rendered.iter().enumerate() {
                assert_eq!(&reopened.render_page(page, 0.25).unwrap().rgba, expected);
                assert_eq!(reopened.page_text(page).unwrap(), text[page]);
            }
        }
        if aes.is_some() {
            assert!(engine.open(&output, None).is_err());
        }
        // The same writer also validates new links; unsafe additions stay atomic.
        let before_add = doc.snapshot().unwrap();
        let rect = doc.annotation_details(0).unwrap()[start].rect;
        assert!(doc
            .add_link_annotation(0, rect, &LinkTarget::Uri("data:text/plain,bad".into()))
            .is_err());
        unchanged(&before_add, &doc.snapshot().unwrap(), password);
        let added = doc
            .add_link_annotation(0, rect, &LinkTarget::Uri("mailto:test@example.org".into()))
            .unwrap();
        assert_eq!(
            doc.annotation_details(0).unwrap()[added].link,
            Some(LinkTarget::Uri("mailto:test@example.org".into()))
        );
        doc.set_link_target(0, added, &LinkTarget::Page(1)).unwrap();
        assert_eq!(
            doc.annotation_details(0).unwrap()[added].link,
            Some(LinkTarget::Page(1))
        );
    }
}
