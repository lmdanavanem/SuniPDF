//! Tests d'intégration du moteur : nécessitent `pdfium.dll` (voir scripts/fetch-pdfium.ps1).

use std::path::{Path, PathBuf};
use sunipdf_core::{Engine, EngineError, MAX_RENDER_SIDE_PX};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn engine() -> Engine {
    Engine::new().expect("pdfium.dll introuvable : exécuter scripts/fetch-pdfium.ps1")
}

#[test]
fn library_not_found_lists_searched_dirs() {
    let dirs = vec![std::env::temp_dir().join("sunipdf-no-such-dir")];
    match Engine::with_library_dirs(&dirs) {
        Err(EngineError::LibraryNotFound(searched)) => assert_eq!(searched, dirs),
        other => panic!(
            "attendu LibraryNotFound, obtenu {:?}",
            other.err().map(|e| e.to_string())
        ),
    }
}

#[test]
fn opens_two_page_fixture_with_sizes_and_title() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert_eq!(doc.page_count(), 2);
    let p0 = doc.page_size(0).unwrap();
    let p1 = doc.page_size(1).unwrap();
    assert_eq!((p0.width_pt, p0.height_pt), (612.0, 792.0));
    assert_eq!((p1.width_pt, p1.height_pt), (792.0, 612.0));
    assert_eq!(doc.info().title.as_deref(), Some("SuniPDF fixture"));
    assert!(matches!(
        doc.page_size(2),
        Err(EngineError::PageOutOfRange(2))
    ));
}

#[test]
fn renders_page_with_expected_dimensions_and_content() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!((img.width, img.height), (612, 792));
    assert_eq!(img.rgba.len(), 612 * 792 * 4);
    // Le rectangle bleu (0 0 1 rg) couvre x 72..272, y 500..650 (origine PDF en bas).
    let px = |x: u32, y_from_top: u32| {
        let i = ((y_from_top * img.width + x) * 4) as usize;
        (
            img.rgba[i],
            img.rgba[i + 1],
            img.rgba[i + 2],
            img.rgba[i + 3],
        )
    };
    assert_eq!(
        px(150, 792 - 575),
        (0, 0, 255, 255),
        "intérieur du rectangle bleu"
    );
    assert_eq!(px(400, 100), (255, 255, 255, 255), "fond blanc");
    // Le texte (bleu lui aussi, 24 pt à y=700) est le seul contenu au-dessus du rectangle :
    // des pixels colorés dans les 140 premières lignes prouvent que la police a été rendue.
    let text_rows = &img.rgba[..(140 * img.width * 4) as usize];
    let text_pixels = text_rows
        .chunks(4)
        .filter(|p| p[0] < 200 || p[1] < 200)
        .count();
    assert!(
        text_pixels > 100,
        "le texte doit produire des pixels colorés ({text_pixels})"
    );

    let half = doc.render_page(1, 0.5).unwrap();
    assert_eq!((half.width, half.height), (396, 306));
}

#[test]
fn rejects_out_of_range_page_and_oversized_render() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert!(matches!(
        doc.render_page(5, 1.0),
        Err(EngineError::PageOutOfRange(5))
    ));
    let too_big = (MAX_RENDER_SIDE_PX as f32 + 1.0) / 612.0;
    assert!(matches!(
        doc.render_page(0, too_big),
        Err(EngineError::RenderTooLarge)
    ));
    assert!(matches!(
        doc.render_page(0, 0.0),
        Err(EngineError::RenderTooLarge)
    ));
    assert!(matches!(
        doc.render_page(0, f32::NAN),
        Err(EngineError::RenderTooLarge)
    ));
}

#[test]
fn garbage_file_is_invalid_format() {
    let engine = engine();
    let path = std::env::temp_dir().join("sunipdf-not-a-pdf.txt");
    std::fs::write(&path, b"ceci n'est pas un PDF").unwrap();
    assert!(matches!(
        engine.open(&path, None),
        Err(EngineError::InvalidFormat)
    ));
}

#[test]
fn missing_file_is_io_error() {
    let engine = engine();
    let err = engine
        .open(&fixture("does-not-exist.pdf"), None)
        .unwrap_err();
    assert!(matches!(err, EngineError::Io(_)), "obtenu {err:?}");
    assert_eq!(err.code(), "io");
}

#[test]
fn encrypted_document_requires_correct_password() {
    let engine = engine();
    let path = fixture("encrypted-rc4.pdf");
    assert!(matches!(
        engine.open(&path, None),
        Err(EngineError::PasswordRequired)
    ));
    assert!(matches!(
        engine.open(&path, Some("wrong")),
        Err(EngineError::PasswordRequired)
    ));
    let doc = engine.open(&path, Some("secret")).unwrap();
    assert_eq!(doc.page_count(), 1);
    let img = doc.render_page(0, 0.5).unwrap();
    let green = img
        .rgba
        .chunks(4)
        .filter(|p| p[0] == 0 && p[1] > 100 && p[2] == 0)
        .count();
    assert!(
        green > 1000,
        "le rectangle vert déchiffré doit être visible ({green})"
    );
}

#[test]
fn extracts_text_and_finds_matches_with_bounds() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert!(doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    assert!(doc.page_text(1).unwrap().contains("SuniPDF page 2"));
    assert!(matches!(
        doc.page_text(7),
        Err(EngineError::PageOutOfRange(7))
    ));

    let all = doc.search("page", false).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!((all[0].page, all[1].page), (0, 1));
    let rect = all[0].rects[0];
    // Texte à 24 pt, placé en (72, 700) : l'occurrence est dans la zone du texte.
    assert!(rect.left > 72.0 && rect.right < 400.0, "{rect:?}");
    assert!(rect.bottom > 690.0 && rect.top < 730.0, "{rect:?}");
    assert_eq!(doc.search("sunipdf", true).unwrap().len(), 0);
    assert_eq!(doc.search("sunipdf", false).unwrap().len(), 2);
    assert_eq!(doc.search("   ", false).unwrap().len(), 0);
    assert_eq!(doc.search_page(1, "page 2", false).unwrap().len(), 1);
}
