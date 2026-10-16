use lopdf::{dictionary, Object};
use std::path::{Path, PathBuf};
use sunipdf_core::{
    security, Engine, EngineError, PageLabelRange, PageLabelStyle, Protection, Rotation, TextStyle,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn labels<'a>(doc: &'a sunipdf_core::Document<'_>) -> Vec<Option<&'a str>> {
    doc.info()
        .page_labels
        .iter()
        .map(Option::as_deref)
        .collect()
}

fn decoded(bytes: &[u8], password: Option<&str>) -> lopdf::Document {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    if pdf.is_encrypted() {
        pdf.decrypt(password.unwrap()).unwrap();
    }
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
        let result = command.arg("--check").arg(path).output().unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn range(
    first: usize,
    last: usize,
    style: PageLabelStyle,
    start: u32,
    prefix: &str,
) -> PageLabelRange {
    PageLabelRange {
        first,
        last,
        style,
        prefix: prefix.into(),
        start,
    }
}

fn all_styles_and_default_boundaries(engine: &Engine) {
    let original = std::fs::read(fixture("page-labels.pdf")).unwrap();
    for (style, start, prefix, expected) in [
        (PageLabelStyle::Decimal, 9, "É-", ["É-9", "É-10", "É-11"]),
        (PageLabelStyle::RomanUpper, 9, "", ["IX", "X", "XI"]),
        (PageLabelStyle::RomanLower, 9, "", ["ix", "x", "xi"]),
        (PageLabelStyle::LettersUpper, 26, "", ["Z", "AA", "BB"]),
        (PageLabelStyle::LettersLower, 26, "", ["z", "aa", "bb"]),
        (
            PageLabelStyle::PrefixOnly,
            1,
            "Annexe-É",
            ["Annexe-É", "Annexe-É", "Annexe-É"],
        ),
    ] {
        let mut doc = engine
            .open_bytes(original.clone(), None, &fixture("page-labels.pdf"))
            .unwrap();
        let before = doc.info().page_labels.clone();
        doc.set_page_labels(&range(1, 3, style, start, prefix))
            .unwrap();
        for page in 0..9 {
            assert_eq!(
                doc.info().page_label(page),
                if (1..=3).contains(&page) {
                    Some(expected[page - 1])
                } else {
                    before[page].as_deref()
                }
            );
        }
        assert_eq!(doc.info().resolve_page_input("#2"), Some(1));
        if style == PageLabelStyle::PrefixOnly {
            assert_eq!(doc.info().resolve_page_input(prefix), None);
        }
    }
    let mut plain = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert_eq!(labels(&plain), [None, None]);
    plain
        .set_page_labels(&range(0, 0, PageLabelStyle::RomanLower, 1, ""))
        .unwrap();
    assert_eq!(labels(&plain), [Some("i"), Some("2")]);
    plain.reset_page_labels().unwrap();
    assert_eq!(labels(&plain), [None, None]);
    plain
        .set_page_labels(&range(1, 1, PageLabelStyle::RomanUpper, 8, ""))
        .unwrap();
    assert_eq!(labels(&plain), [Some("1"), Some("VIII")]);
    // Replacing a middle part of an existing numbered range must resume its original ordinal afterward.
    let mut numbered = engine.open(&fixture("page-labels.pdf"), None).unwrap();
    numbered
        .set_page_labels(&range(0, 8, PageLabelStyle::Decimal, 40, "Page-"))
        .unwrap();
    numbered
        .set_page_labels(&range(2, 3, PageLabelStyle::RomanUpper, 9, ""))
        .unwrap();
    assert_eq!(
        labels(&numbered),
        [
            Some("Page-40"),
            Some("Page-41"),
            Some("IX"),
            Some("X"),
            Some("Page-44"),
            Some("Page-45"),
            Some("Page-46"),
            Some("Page-47"),
            Some("Page-48")
        ]
    );
}

fn protected_round_trips(engine: &Engine) {
    let temp = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture("page-labels.pdf")).unwrap();
    for (index, (aes, password)) in [
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
        let source = temp.path().join(format!("source-{index}.pdf"));
        std::fs::write(&source, &input).unwrap();
        let mut doc = engine.open(&source, password).unwrap();
        doc.set_page_rotation(0, Rotation::R90).unwrap();
        doc.add_text(4, 40.0, 40.0, "Unsaved content", &TextStyle::default())
            .unwrap();
        let before = doc.snapshot().unwrap();
        let before_labels = doc.info().page_labels.clone();
        let properties = doc.properties();
        let sizes = doc.info().page_sizes.clone();
        let text: Vec<_> = (0..9).map(|p| doc.page_text(p).unwrap()).collect();
        let rendered: Vec<_> = (0..9)
            .map(|p| doc.render_page(p, 0.25).unwrap().rgba)
            .collect();
        doc.set_page_labels(&range(1, 3, PageLabelStyle::RomanUpper, 9, "É-"))
            .unwrap();
        assert_eq!(doc.password(), password);
        assert_eq!(doc.info().path, source);
        assert_eq!(doc.info().page_sizes, sizes);
        assert_eq!(doc.properties(), properties);
        for page in 0..9 {
            assert_eq!(doc.page_text(page).unwrap(), text[page]);
            assert_eq!(doc.render_page(page, 0.25).unwrap().rgba, rendered[page]);
            if !(1..=3).contains(&page) {
                assert_eq!(doc.info().page_labels[page], before_labels[page]);
            }
        }
        let after = doc.snapshot().unwrap();
        doc.restore(before.clone()).unwrap();
        assert_eq!(doc.info().page_labels, before_labels);
        doc.restore(after).unwrap();
        let output = temp.path().join(format!("labels-{index}.pdf"));
        doc.save_as(&output).unwrap();
        qpdf(&output, password);
        assert_eq!(std::fs::read(&source).unwrap(), input);
        for reopen_password in if aes.is_some() {
            vec![Some("user"), Some("owner")]
        } else {
            vec![None]
        } {
            let reopened = engine.open(&output, reopen_password).unwrap();
            assert_eq!(labels(&reopened), labels(&doc));
            assert_eq!(reopened.page_rotation(0).unwrap(), Rotation::R90);
            assert_eq!(reopened.properties().encrypted, aes.is_some());
            for (page, expected_text) in text.iter().enumerate() {
                assert_eq!(&reopened.page_text(page).unwrap(), expected_text);
            }
        }
        if aes.is_some() {
            assert!(engine.open(&output, None).is_err());
        }
        doc.reset_page_labels().unwrap();
        assert_eq!(labels(&doc), vec![None; 9]);
        assert_eq!(doc.properties(), properties);
        let reset = temp.path().join(format!("reset-{index}.pdf"));
        doc.save_as(&reset).unwrap();
        qpdf(&reset, password);
        for reopen_password in if aes.is_some() {
            vec![Some("user"), Some("owner")]
        } else {
            vec![None]
        } {
            let reopened = engine.open(&reset, reopen_password).unwrap();
            assert_eq!(labels(&reopened), vec![None; 9]);
            assert_eq!(reopened.properties().encrypted, aes.is_some());
            for (page, expected_text) in text.iter().enumerate() {
                assert_eq!(&reopened.page_text(page).unwrap(), expected_text);
            }
        }
        assert_eq!(std::fs::read(&source).unwrap(), input);
    }
}

fn malformed_trees_and_invalid_ranges_are_atomic(engine: &Engine) {
    let mut doc = engine.open(&fixture("page-labels.pdf"), None).unwrap();
    let before = doc.snapshot().unwrap();
    for range in [
        range(8, 1, PageLabelStyle::Decimal, 1, ""),
        range(0, 99, PageLabelStyle::Decimal, 1, ""),
        range(0, 1, PageLabelStyle::Decimal, 0, ""),
        range(0, 1, PageLabelStyle::LettersUpper, i32::MAX as u32, ""),
        range(0, 1, PageLabelStyle::PrefixOnly, 1, ""),
        range(0, 1, PageLabelStyle::Decimal, 1, "bad\nlabel"),
    ] {
        assert!(doc.set_page_labels(&range).is_err());
        unchanged(&before, &doc.snapshot().unwrap(), None);
    }
    for (index, nums) in [
        vec![0.into(), dictionary! {"S"=>"Unknown"}.into()],
        vec![
            0.into(),
            dictionary! {"S"=>"D"}.into(),
            0.into(),
            dictionary! {"S"=>"D"}.into(),
        ],
        vec![0.into(), dictionary! {"St"=>0}.into()],
    ]
    .into_iter()
    .enumerate()
    {
        let mut pdf = lopdf::Document::load(fixture("two-pages.pdf")).unwrap();
        pdf.catalog_mut()
            .unwrap()
            .set("PageLabels", dictionary! {"Nums"=>nums});
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let mut malformed = engine
            .open_bytes(bytes, None, Path::new("malformed.pdf"))
            .unwrap();
        let before = malformed.snapshot().unwrap();
        let result = malformed.set_page_labels(&range(0, 0, PageLabelStyle::Decimal, 1, ""));
        assert!(
            matches!(result, Err(EngineError::Other(_))),
            "malformed case {index}: {result:?}"
        );
        unchanged(&before, &malformed.snapshot().unwrap(), None);
        malformed.reset_page_labels().unwrap();
        assert_eq!(labels(&malformed), [None, None]);
        assert!(malformed.page_text(0).unwrap().contains("SuniPDF page 1"));
    }
}

fn preserves_unknown_keys_forms_and_links(engine: &Engine) {
    let mut pdf = lopdf::Document::load(fixture("acroform-links.pdf")).unwrap();
    let dict =
        dictionary! {"S"=>"D", "St"=>7, "CustomLabelMetadata"=>Object::string_literal("keep me")};
    pdf.catalog_mut().unwrap().set("PageLabels", dictionary! {"Nums"=>vec![0.into(), dict.clone().into()], "CustomTreeMetadata"=>Object::Integer(12)});
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let mut doc = engine
        .open_bytes(bytes, None, Path::new("forms-links.pdf"))
        .unwrap();
    let form_before = doc.form_info().unwrap();
    let bookmarks = doc.bookmarks();
    let links: Vec<_> = (0..doc.page_count())
        .map(|p| doc.page_links(p).unwrap())
        .collect();
    doc.set_page_labels(&range(0, 0, PageLabelStyle::RomanLower, 1, ""))
        .unwrap();
    assert_eq!(doc.form_info().unwrap(), form_before);
    assert_eq!(doc.bookmarks(), bookmarks);
    for (page, before) in links.iter().enumerate() {
        assert_eq!(&doc.page_links(page).unwrap(), before);
    }
    let output = decoded(&doc.snapshot().unwrap(), None);
    let tree = output
        .catalog()
        .unwrap()
        .get_deref(b"PageLabels", &output)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(
        tree.get(b"CustomTreeMetadata").unwrap().as_i64().unwrap(),
        12
    );
    if doc.page_count() > 1 {
        let nums = tree.get(b"Nums").unwrap().as_array().unwrap();
        let restored = nums[3].as_dict().unwrap();
        assert_eq!(
            restored.get(b"CustomLabelMetadata").unwrap(),
            dict.get(b"CustomLabelMetadata").unwrap()
        );
        assert_eq!(restored.get(b"St").unwrap().as_i64().unwrap(), 8);
    }
}

#[test]
fn page_label_authoring_preserves_outside_ranges_content_and_protection() {
    // PDFium has one process-global instance; all native scenarios share this engine.
    let engine = Engine::new().unwrap();
    all_styles_and_default_boundaries(&engine);
    protected_round_trips(&engine);
    malformed_trees_and_invalid_ranges_are_atomic(&engine);
    preserves_unknown_keys_forms_and_links(&engine);
}
