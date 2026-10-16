use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use sunipdf_core::ocr::{OcrLanguage, OcrOptions};
use sunipdf_core::{Document, Engine, EngineError, PageMargins, Rotation, TextStyle};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// Every access is serialized; the single engine is kept alive for this test process.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}

fn engine() -> &'static Engine {
    let ptr = ENGINE.get_or_init(|| EnginePtr(Box::leak(Box::new(Engine::new().unwrap()))));
    unsafe { &*ptr.0 }
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/two-pages.pdf")
}

fn assert_same_pdf_contents(before: &[u8], after: &[u8]) {
    let mut before = lopdf::Document::load_mem(before).unwrap();
    let mut after = lopdf::Document::load_mem(after).unwrap();
    // PDFium may regenerate only the trailer ID when serializing an unchanged document.
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(after.objects, before.objects);
    assert_eq!(after.trailer, before.trailer);
}

fn scans(engine: &Engine) -> Document<'_> {
    let mut vector = engine.open(&fixture(), None).unwrap();
    vector.insert_blank_pages(0, 1, 500.0, 160.0).unwrap();
    vector.delete_page(2).unwrap();
    vector.delete_page(1).unwrap();
    let blank = vector.snapshot().unwrap();
    let mut doc = engine
        .open_bytes(blank.clone(), None, Path::new("scanned.pdf"))
        .unwrap();
    doc.insert_blank_pages(1, 1, 200.0, 540.0).unwrap();
    doc.crop_page(
        1,
        PageMargins {
            left: 20.0,
            top: 20.0,
            right: 20.0,
            bottom: 20.0,
        },
    )
    .unwrap();
    doc.set_page_rotation(1, Rotation::R90).unwrap();
    for (page, text) in [
        (0, "The quick brown fox jumps over the lazy dog."),
        (1, "Bonjour, l'été français arrive bientôt."),
    ] {
        vector.restore(blank.clone()).unwrap();
        vector
            .add_text(
                0,
                24.0,
                100.0,
                text,
                &TextStyle {
                    size_pt: 22.0,
                    ..Default::default()
                },
            )
            .unwrap();
        let image = vector.render_page(0, 3.0).unwrap();
        let image = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(image.width, image.height, image.rgba).unwrap(),
        );
        doc.add_image_data(page, &image, 0.0, 0.0, 500.0).unwrap();
        assert!(doc.page_text(page).unwrap().trim().is_empty());
    }
    doc
}

#[test]
fn local_english_french_ocr_keeps_pixels_and_page_geometry() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut doc = scans(engine);
    let before: Vec<_> = (0..2)
        .map(|page| doc.render_page(page, 1.0).unwrap().rgba)
        .collect();
    let geometry = doc.page_geometry(1).unwrap();
    let mut updates = Vec::new();
    let report = doc
        .ocr(
            &OcrOptions {
                pages: vec![0, 1],
                language: OcrLanguage::EnglishFrench,
            },
            &AtomicBool::new(false),
            |done, total| updates.push((done, total)),
        )
        .unwrap();
    assert_eq!(report.recognized_pages, 2);
    assert_eq!(report.skipped_pages, 0);
    assert!(report.words >= 10);
    assert_eq!(updates, [(1, 2), (2, 2)]);
    assert_eq!(doc.page_geometry(1).unwrap(), geometry);
    for (page, word) in [(0, "quick"), (1, "français")] {
        assert!(
            doc.page_text(page).unwrap().contains(word),
            "{}",
            doc.page_text(page).unwrap()
        );
        assert_eq!(
            doc.render_page(page, 1.0).unwrap().rgba,
            before[page],
            "OCR must be invisible"
        );
        let matches = doc.search_page(page, word, true).unwrap();
        assert_eq!(matches.len(), 1);
        for rect in &matches[0].rects {
            assert!(
                rect.left >= 20.0 && rect.right < 490.0 && rect.bottom > 95.0 && rect.top < 130.0,
                "unexpected OCR geometry: {rect:?}"
            );
        }
    }
    let bytes = doc.snapshot().unwrap();
    let reopened = engine
        .open_bytes(bytes, None, Path::new("saved-ocr.pdf"))
        .unwrap();
    assert!(reopened.page_text(1).unwrap().contains("français"));
}

#[test]
fn cancellation_after_a_recognized_page_does_not_mutate_document() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut doc = scans(engine);
    let before = doc.snapshot().unwrap();
    let cancel = AtomicBool::new(false);
    let result = doc.ocr(
        &OcrOptions {
            pages: vec![0, 1],
            language: OcrLanguage::EnglishFrench,
        },
        &cancel,
        |done, _| {
            if done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    );
    assert!(matches!(result, Err(EngineError::Cancelled)));
    assert_same_pdf_contents(&before, &doc.snapshot().unwrap());
    assert!(doc.page_text(0).unwrap().trim().is_empty());
}

#[test]
fn existing_text_is_skipped_without_rewriting_the_pdf() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut doc = engine.open(&fixture(), None).unwrap();
    let file_before = std::fs::read(fixture()).unwrap();
    let before = doc.snapshot().unwrap();
    let report = doc
        .ocr(
            &OcrOptions {
                pages: vec![0, 1, 1],
                language: OcrLanguage::English,
            },
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
    assert_eq!(report.skipped_pages, 2);
    assert_eq!(report.words, 0);
    assert_same_pdf_contents(&before, &doc.snapshot().unwrap());
    assert_eq!(std::fs::read(fixture()).unwrap(), file_before);
}

#[test]
fn invalid_pages_and_pre_cancelled_runs_leave_document_unchanged() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = engine();
    let mut doc = engine.open(&fixture(), None).unwrap();
    let file_before = std::fs::read(fixture()).unwrap();
    let before = doc.snapshot().unwrap();
    assert!(matches!(
        doc.ocr(
            &OcrOptions {
                pages: vec![0, 2],
                language: OcrLanguage::French
            },
            &AtomicBool::new(false),
            |_, _| {}
        ),
        Err(EngineError::PageOutOfRange(2))
    ));
    assert!(matches!(
        doc.ocr(
            &OcrOptions {
                pages: vec![0],
                language: OcrLanguage::French
            },
            &AtomicBool::new(true),
            |_, _| {}
        ),
        Err(EngineError::Cancelled)
    ));
    assert!(doc
        .ocr(
            &OcrOptions {
                pages: vec![],
                language: OcrLanguage::French
            },
            &AtomicBool::new(false),
            |_, _| {}
        )
        .is_err());
    assert_same_pdf_contents(&before, &doc.snapshot().unwrap());
    assert_eq!(std::fs::read(fixture()).unwrap(), file_before);
}
