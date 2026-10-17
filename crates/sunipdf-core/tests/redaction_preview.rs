use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use sunipdf_core::redact::{RedactAppearance, RedactOverlay};
use sunipdf_core::{Engine, RedactOptions, TextStyle};

#[test]
fn prepared_redaction_preserves_safe_native_runs_and_is_atomic_and_persistent() {
    let engine = Engine::new().unwrap();
    let mut doc = engine
        .open_bytes(
            include_bytes!("fixtures/two-pages.pdf").to_vec(),
            None,
            Path::new("redact.pdf"),
        )
        .unwrap();
    doc.add_text(0, 60.0, 400.0, "LEFT SECRET RIGHT", &TextStyle::default())
        .unwrap();
    let marks = doc.redact_marks_for_text("SECRET", true).unwrap();
    let original = doc.snapshot().unwrap();
    let appearance = RedactAppearance {
        fill_rgb: (245, 220, 190),
        overlay: None,
    };
    let mut preview = doc
        .preview_redaction(&marks, &RedactOptions::default(), &appearance, 0)
        .unwrap();
    assert!(doc.page_text(0).unwrap().contains("SECRET"));
    assert_ne!(preview.before.rgba, preview.after.rgba);
    assert!(preview.report.native_text_runs > 0);
    doc.commit_redaction_preview(preview.prepared.take().unwrap())
        .unwrap();
    let text = doc.page_text(0).unwrap();
    assert!(
        text.contains("LEFT") && text.contains("RIGHT") && !text.contains("SECRET"),
        "{text}"
    );
    let clean = doc.snapshot().unwrap();
    let parsed = lopdf::Document::load_mem(&clean).unwrap();
    for (_, object) in parsed.objects {
        if let lopdf::Object::Stream(stream) = object {
            let bytes = stream.decompressed_content().unwrap_or(stream.content);
            assert!(!bytes.windows(6).any(|window| window == b"SECRET"));
        }
    }
    doc.restore(original.clone()).unwrap();
    doc.persist_redaction_marks(&marks, &appearance, Some("Test author"))
        .unwrap();
    assert_eq!(doc.redaction_marks().unwrap(), marks);
    assert!(
        doc.page_text(0).unwrap().contains("SECRET"),
        "marks do not apply redaction"
    );
    let marked = doc.snapshot().unwrap();
    let reopened = engine
        .open_bytes(marked, None, Path::new("marked.pdf"))
        .unwrap();
    assert_eq!(reopened.redaction_marks().unwrap(), marks);
    doc.restore(original).unwrap();
    let mut preview = doc
        .preview_redaction(&marks, &RedactOptions::default(), &appearance, 0)
        .unwrap();
    doc.add_text(0, 60.0, 300.0, "Later edit", &TextStyle::default())
        .unwrap();
    let before_failure = doc.page_text(0).unwrap();
    assert!(doc
        .commit_redaction_preview(preview.prepared.take().unwrap())
        .is_err());
    assert_eq!(doc.page_text(0).unwrap(), before_failure);
    let invalid = RedactAppearance {
        overlay: Some(RedactOverlay {
            text: "this intentional overlay cannot fit".into(),
            style: TextStyle {
                size_pt: 500.0,
                ..Default::default()
            },
        }),
        ..appearance
    };
    assert!(doc
        .preview_redaction(&marks, &RedactOptions::default(), &invalid, 0)
        .is_err());
    assert_eq!(doc.page_text(0).unwrap(), before_failure);
    let cancel = Arc::new(AtomicBool::new(true));
    assert!(matches!(
        doc.preview_redaction_cancelable(
            &marks,
            &RedactOptions::default(),
            &RedactAppearance::default(),
            0,
            cancel.clone()
        ),
        Err(sunipdf_core::EngineError::Cancelled)
    ));
    cancel.store(false, Ordering::Relaxed);
    let mut preview = doc
        .preview_redaction_cancelable(
            &marks,
            &RedactOptions::default(),
            &RedactAppearance::default(),
            0,
            cancel.clone(),
        )
        .unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        doc.commit_redaction_preview(preview.prepared.take().unwrap()),
        Err(sunipdf_core::EngineError::Cancelled)
    ));
    assert_eq!(doc.page_text(0).unwrap(), before_failure);

    let protected = sunipdf_core::security::protect(
        include_bytes!("fixtures/redact.pdf"),
        None,
        &sunipdf_core::Protection {
            user_password: "preview-user".into(),
            owner_password: "preview-owner".into(),
            allow_copy: false,
            ..Default::default()
        },
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.pdf");
    std::fs::write(&source, &protected).unwrap();
    let mut doc = engine.open(&source, Some("preview-user")).unwrap();
    let properties = doc.properties();
    let marks = doc.redact_marks_for_text("HIDDEN-OCR-5512", true).unwrap();
    let mut preview = doc
        .preview_redaction(
            &marks,
            &RedactOptions::default(),
            &RedactAppearance::default(),
            0,
        )
        .unwrap();
    assert!(preview.report.bitmap_objects > 0);
    let expected = preview.after.rgba.clone();
    let scale = preview.scale;
    doc.commit_redaction_preview(preview.prepared.take().unwrap())
        .unwrap();
    assert_eq!(doc.render_page(0, scale).unwrap().rgba, expected);
    assert!(!doc.page_text(0).unwrap().contains("HIDDEN-OCR-5512"));
    assert!(doc.is_encrypted());
    assert_eq!(doc.properties().can_copy, properties.can_copy);
    assert_eq!(std::fs::read(&source).unwrap(), protected);
    let output = temp.path().join("redacted.pdf");
    doc.save_as(&output).unwrap();
    assert!(engine.open(&output, None).is_err());
    let reopened = engine.open(&output, Some("preview-user")).unwrap();
    assert_eq!(reopened.render_page(0, scale).unwrap().rgba, expected);
    let cleaned = lopdf::Document::load_mem_with_options(
        &std::fs::read(output).unwrap(),
        lopdf::LoadOptions::with_password("preview-user"),
    )
    .unwrap();
    assert!(cleaned.trailer.get(b"Info").is_err());
    assert!(cleaned.catalog().unwrap().get(b"StructTreeRoot").is_err());
}
