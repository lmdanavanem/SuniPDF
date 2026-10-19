use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use sunipdf_core::{local_review::*, security, Engine, EngineError, Protection};

fn pdf(lines: &[&str], width: i64) -> Vec<u8> {
    let mut doc = Pdf::with_version("1.7");
    let pages = doc.new_object_id();
    let font =
        doc.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let mut kids = Vec::new();
    for text in lines {
        let content = doc.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 16 Tf 20 80 Td ({text}) Tj ET").into_bytes(),
        ));
        let page = doc.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
        "MediaBox"=>vec![0.into(),0.into(),width.into(),160.into()],
        "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}, "Contents"=>content});
        kids.push(Object::Reference(page));
    }
    doc.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>kids.len() as i64, "Kids"=>kids}.into(),
    );
    let root = doc.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    doc.trailer.set("Root", root);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn native_review_preserves_sources_reports_partial_failures_and_verifies_previews() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let left = temp.path().join("original-é.pdf");
    let right = temp.path().join("revised.pdf");
    let broken = temp.path().join("broken.pdf");
    let absent = temp.path().join("absent.pdf");
    let original = pdf(&["Needle original", "Needle second"], 200);
    let revised = pdf(&["Needle changed", "Needle second", "Added page"], 200);
    std::fs::write(&left, &original).unwrap();
    std::fs::write(&right, &revised).unwrap();
    std::fs::write(&broken, b"not a PDF").unwrap();
    let cancel = AtomicBool::new(false);
    let report = search_files(
        &engine,
        &[left.clone(), broken, absent, right.clone()],
        "needle",
        false,
        None,
        &cancel,
        |_, _, _, _| {},
    )
    .unwrap();
    assert!(!report.complete());
    assert_eq!(report.files[0].hits.len(), 2);
    assert_eq!(report.files[0].hits[0].page, 1);
    assert!(report.files[0].hits[0].snippet.contains("Needle original"));
    assert_eq!(
        report.files[1].error.as_ref().unwrap().code(),
        "invalid_format"
    );
    assert_eq!(report.files[2].error.as_ref().unwrap().code(), "io");
    assert_eq!(report.files[3].status, "complete");
    assert!(!report.json()["complete"].as_bool().unwrap());
    let none = search_files(
        &engine,
        std::slice::from_ref(&left),
        "absent",
        false,
        None,
        &cancel,
        |_, _, _, _| {},
    )
    .unwrap();
    assert!(none.complete());
    assert!(none.files[0].hits.is_empty());
    assert!(search_files(&engine, &[], "x", false, None, &cancel, |_, _, _, _| {}).is_err());
    assert!(search_files(
        &engine,
        std::slice::from_ref(&left),
        " ",
        false,
        None,
        &cancel,
        |_, _, _, _| {}
    )
    .is_err());
    assert!(search_files(
        &engine,
        std::slice::from_ref(&left),
        &"x".repeat(257),
        false,
        None,
        &cancel,
        |_, _, _, _| {}
    )
    .is_err());
    let interrupted = search_files(
        &engine,
        &[left.clone(), right.clone()],
        "Needle",
        true,
        None,
        &cancel,
        |file, _, page, _| {
            if file == 0 && page == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap();
    assert_eq!(interrupted.files[0].status, "cancelled");
    assert_eq!(interrupted.files[0].scanned_pages, 1);
    assert_eq!(interrupted.files[1].status, "cancelled");
    cancel.store(false, Ordering::Relaxed);
    for aes256 in [false, true] {
        let protected = temp.path().join(format!("protected-{aes256}.pdf"));
        let bytes = security::protect(
            &original,
            None,
            &Protection {
                user_password: "review-fixture-user".into(),
                owner_password: "review-fixture-owner".into(),
                allow_copy: false,
                aes256,
                ..Protection::default()
            },
        )
        .unwrap();
        std::fs::write(&protected, &bytes).unwrap();
        for (password, code) in [
            (None, "password_required"),
            (Some("review-fixture-user"), "security_restricted"),
        ] {
            let report = search_files(
                &engine,
                std::slice::from_ref(&protected),
                "Needle",
                false,
                password,
                &cancel,
                |_, _, _, _| {},
            )
            .unwrap();
            assert_eq!(report.files[0].error.as_ref().unwrap().code(), code);
            assert!(report.files[0].hits.is_empty());
            assert!(compare_files(
                &engine,
                &protected,
                &protected,
                CompareOptions::default(),
                password,
                &cancel,
                |_, _| {}
            )
            .is_err());
        }
        let owner = search_files(
            &engine,
            std::slice::from_ref(&protected),
            "Needle",
            true,
            Some("review-fixture-owner"),
            &cancel,
            |_, _, _, _| {},
        )
        .unwrap();
        assert_eq!(
            owner.files[0].error.as_ref().unwrap().code(),
            "security_restricted"
        );
        assert!(owner.files[0].hits.is_empty());
        assert_eq!(std::fs::read(protected).unwrap(), bytes);
    }
    let equal = compare_files(
        &engine,
        &left,
        &left,
        CompareOptions::default(),
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(equal.complete());
    assert!(equal.pages.iter().all(|p| !p.changed()));
    let changed = compare_files(
        &engine,
        &left,
        &right,
        CompareOptions::default(),
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(changed.complete());
    assert!(changed.pages[0].text_changed);
    assert!(changed.pages[0].changed_pixels > 0);
    assert!(changed.pages[0].bounds.is_some());
    assert!(!changed.pages[1].changed());
    assert_eq!(changed.pages[2].left_page, None);
    let preview = comparison_preview(&engine, &changed, 0, None).unwrap();
    assert!(preview.difference.width <= PREVIEW_SIDE + 1);
    assert!(preview
        .difference
        .rgba
        .as_chunks::<4>()
        .0
        .contains(&[220, 35, 65, 255]));
    let partial = compare_files(
        &engine,
        &left,
        &right,
        CompareOptions {
            max_pages: 1,
            ..CompareOptions::default()
        },
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(!partial.complete());
    assert_eq!(partial.pages.len(), 1);
    let interrupted = compare_files(
        &engine,
        &left,
        &right,
        CompareOptions::default(),
        None,
        &cancel,
        |_, _| {
            cancel.store(true, Ordering::Relaxed);
        },
    )
    .unwrap();
    assert!(interrupted.cancelled);
    assert_eq!(interrupted.pages.len(), 1);
    cancel.store(false, Ordering::Relaxed);
    let aligned = compare_files(
        &engine,
        &left,
        &right,
        CompareOptions {
            left_start: 2,
            right_start: 2,
            max_pages: 200,
        },
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(!aligned.pages[0].changed());
    assert!(compare_files(
        &engine,
        &left,
        &right,
        CompareOptions {
            left_start: 0,
            ..CompareOptions::default()
        },
        None,
        &cancel,
        |_, _| {}
    )
    .is_err());
    assert!(compare_files(
        &engine,
        &left,
        &right,
        CompareOptions {
            max_pages: 501,
            ..CompareOptions::default()
        },
        None,
        &cancel,
        |_, _| {}
    )
    .is_err());
    std::fs::write(
        &right,
        pdf(&["Needle changed", "Needle second", "Added page"], 240),
    )
    .unwrap();
    assert!(comparison_preview(&engine, &changed, 0, None).is_err());
    let resized = compare_files(
        &engine,
        &left,
        &right,
        CompareOptions::default(),
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(resized.pages[1].geometry_changed);
    let blank = temp.path().join("image-or-blank.pdf");
    std::fs::write(&blank, pdf(&[""], 200)).unwrap();
    let image_only = compare_files(
        &engine,
        &blank,
        &blank,
        CompareOptions::default(),
        None,
        &cancel,
        |_, _| {},
    )
    .unwrap();
    assert!(image_only.pages[0].no_text_on_either_page);
    let excessive_text = temp.path().join("excessive-text.pdf");
    std::fs::write(
        &excessive_text,
        pdf(&[&"X".repeat(MAX_PAGE_CHARS + 1)], 200),
    )
    .unwrap();
    let excessive = search_files(
        &engine,
        &[excessive_text],
        "X",
        true,
        None,
        &cancel,
        |_, _, _, _| {},
    )
    .unwrap();
    assert!(!excessive.complete());
    assert!(excessive.files[0]
        .error
        .as_ref()
        .unwrap()
        .to_string()
        .contains("250000"));
    let huge = temp.path().join("too-large.pdf");
    std::fs::File::create(&huge)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    let limited = search_files(
        &engine,
        &[huge, temp.path().into()],
        "Needle",
        true,
        None,
        &cancel,
        |_, _, _, _| {},
    )
    .unwrap();
    assert!(limited
        .files
        .iter()
        .all(|f| f.status == "error" && f.scanned_pages == 0));
    assert!(matches!(
        compare_files(
            &engine,
            Path::new("does-not-exist.pdf"),
            &left,
            CompareOptions::default(),
            None,
            &cancel,
            |_, _| {}
        ),
        Err(EngineError::Io(_))
    ));
    assert_eq!(std::fs::read(&left).unwrap(), original);
}
