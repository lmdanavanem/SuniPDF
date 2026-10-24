//! Unsupported OCR fails before changing an opened document.
#![cfg(target_os = "macos")]

use std::path::Path;
use std::sync::atomic::AtomicBool;
use sunipdf_core::ocr::{OcrLanguage, OcrOptions};
use sunipdf_core::{Engine, EngineError};

#[test]
fn unsupported_ocr_leaves_the_document_and_original_file_unchanged() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/two-pages.pdf");
    let original = std::fs::read(&path).unwrap();
    let engine = Engine::new().unwrap();
    let mut document = engine.open(&path, None).unwrap();
    let before = document.snapshot().unwrap();
    let mut progress_called = false;
    let result = document.ocr(
        &OcrOptions {
            pages: vec![0],
            language: OcrLanguage::EnglishFrench,
        },
        &AtomicBool::new(false),
        |_, _| progress_called = true,
    );
    assert!(matches!(result, Err(EngineError::Other(message))
        if message == "OCR is unavailable on this platform"));
    assert!(!progress_called);
    let mut before = lopdf::Document::load_mem(&before).unwrap();
    let mut after = lopdf::Document::load_mem(&document.snapshot().unwrap()).unwrap();
    // PDFium may regenerate the trailer ID when serializing an unchanged document.
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(before.objects, after.objects);
    assert_eq!(before.trailer, after.trailer);
    assert_eq!(std::fs::read(path).unwrap(), original);
}
