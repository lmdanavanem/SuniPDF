//! Tests d'intégration du moteur : nécessitent `pdfium.dll` (voir scripts/fetch-pdfium.ps1).

use std::path::{Path, PathBuf};
use sunipdf_core::{Engine, EngineError, MAX_RENDER_SIDE_PX};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// PDFium n'est pas thread-safe et ne se réinitialise pas dans un même processus après
/// `FPDF_DestroyLibrary` : un seul moteur, jamais détruit, utilisé par un test à la fois.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
static ENGINE: std::sync::OnceLock<EnginePtr> = std::sync::OnceLock::new();

struct EnginePtr(*const Engine);
// Sûreté : tout accès au moteur est sérialisé par `SERIAL`.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}

struct TestEngine {
    inner: &'static Engine,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl std::ops::Deref for TestEngine {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        self.inner
    }
}

fn engine() -> TestEngine {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ptr = ENGINE.get_or_init(|| {
        EnginePtr(Box::leak(Box::new(Engine::new().expect(
            "pdfium.dll introuvable : exécuter scripts/fetch-pdfium.ps1",
        ))))
    });
    // Sûreté : pointeur vers une valeur fuitée, donc valide pour toute la durée du processus.
    let inner = unsafe { &*ptr.0 };
    TestEngine {
        inner,
        _guard: guard,
    }
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

#[test]
fn merge_extract_split_round_trip() {
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-pages-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = fixture("two-pages.pdf");

    let merged = dir.join("merged.pdf");
    assert_eq!(
        engine.merge(&[src.clone(), src.clone()], &merged).unwrap(),
        4
    );
    let doc = engine.open(&merged, None).unwrap();
    assert_eq!(doc.page_count(), 4);
    assert_eq!(doc.page_size(3).unwrap().width_pt, 792.0);
    assert!(doc.page_text(2).unwrap().contains("SuniPDF page 1"));

    let extracted = dir.join("sub").join("extracted.pdf");
    assert_eq!(doc.extract_to(&[3, 0], &extracted).unwrap(), 2);
    let sub = engine.open(&extracted, None).unwrap();
    assert_eq!(sub.page_count(), 2);
    assert_eq!(
        sub.page_size(0).unwrap().width_pt,
        792.0,
        "ordre demandé conservé"
    );
    assert!(matches!(
        doc.extract_to(&[9], &extracted),
        Err(EngineError::PageOutOfRange(9))
    ));
    assert!(doc.extract_to(&[], &extracted).is_err());

    let parts = engine.split(&merged, None, 3, &dir.join("parts")).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(engine.open(&parts[0], None).unwrap().page_count(), 3);
    assert_eq!(engine.open(&parts[1], None).unwrap().page_count(), 1);
    assert!(parts[1].ends_with("merged-2.pdf"));
    assert!(engine.split(&merged, None, 0, &dir).is_err());
    assert!(engine.merge(&[], &dir.join("x.pdf")).is_err());
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rotate_delete_and_save_atomically() {
    use sunipdf_core::Rotation;
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-rotate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let work = dir.join("work.pdf");
    std::fs::copy(fixture("two-pages.pdf"), &work).unwrap();

    let mut doc = engine.open(&work, None).unwrap();
    assert_eq!(doc.page_rotation(0).unwrap(), Rotation::R0);
    doc.set_page_rotation(0, Rotation::R90).unwrap();
    assert_eq!(doc.page_rotation(0).unwrap(), Rotation::R90);
    assert_eq!(
        doc.page_size(0).unwrap().width_pt,
        792.0,
        "taille affichée après rotation"
    );
    assert!(matches!(
        doc.set_page_rotation(5, Rotation::R90),
        Err(EngineError::PageOutOfRange(5))
    ));
    doc.save_as(&work).unwrap();
    drop(doc);

    let mut again = engine.open(&work, None).unwrap();
    assert_eq!(again.page_rotation(0).unwrap(), Rotation::R90);
    assert_eq!(again.page_count(), 2);
    again.delete_page(0).unwrap();
    assert_eq!(again.page_count(), 1);
    assert!(again.page_text(0).unwrap().contains("page 2"));
    assert!(
        again.delete_page(0).is_err(),
        "la dernière page ne peut pas être supprimée"
    );
    let out = dir.join("one.pdf");
    again.save_as(&out).unwrap();
    assert_eq!(engine.open(&out, None).unwrap().page_count(), 1);
    // Un enregistrement vers un dossier impossible échoue proprement, sans fichier temporaire.
    let bad = Path::new("Z:\\sunipdf-no-such-drive\\x.pdf");
    assert!(matches!(again.save_as(bad), Err(EngineError::Io(_))));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn insert_move_and_snapshot_restore() {
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-insert-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let work = dir.join("work.pdf");
    std::fs::copy(fixture("two-pages.pdf"), &work).unwrap();
    let mut doc = engine.open(&work, None).unwrap();

    // Insertion de la page 2 (paysage) de la fixture au début.
    assert_eq!(
        doc.insert_pages_from(&fixture("two-pages.pdf"), None, &[1], 0)
            .unwrap(),
        1
    );
    assert_eq!(doc.page_count(), 3);
    assert_eq!(doc.page_size(0).unwrap().width_pt, 792.0);
    assert!(doc.page_text(0).unwrap().contains("page 2"));
    assert!(matches!(
        doc.insert_pages_from(&fixture("two-pages.pdf"), None, &[], 9),
        Err(EngineError::PageOutOfRange(9))
    ));
    assert!(matches!(
        doc.insert_pages_from(&fixture("two-pages.pdf"), None, &[5], 0),
        Err(EngineError::PageOutOfRange(5))
    ));

    // Déplacement : [L, P1, P2] → move(0, 2) → [P1, P2, L].
    let before = doc.snapshot().unwrap();
    doc.move_page(0, 2).unwrap();
    assert!(doc.page_text(0).unwrap().contains("page 1"));
    assert!(doc.page_text(2).unwrap().contains("page 2"));
    assert_eq!(doc.page_size(2).unwrap().width_pt, 792.0);
    doc.move_page(2, 0).unwrap();
    assert_eq!(doc.page_size(0).unwrap().width_pt, 792.0);
    assert!(matches!(
        doc.move_page(7, 0),
        Err(EngineError::PageOutOfRange(7))
    ));

    // Restauration de l'instantané : retour à [L, P1, P2] puis annulation complète vers 2 pages.
    doc.delete_page(1).unwrap();
    assert_eq!(doc.page_count(), 2);
    doc.restore(before).unwrap();
    assert_eq!(doc.page_count(), 3);
    assert!(doc.page_text(1).unwrap().contains("page 1"));
    doc.save_as(&work).unwrap();
    assert_eq!(engine.open(&work, None).unwrap().page_count(), 3);

    // Document chiffré : l'instantané se recharge avec le mot de passe mémorisé.
    let mut enc = engine
        .open(&fixture("encrypted-rc4.pdf"), Some("secret"))
        .unwrap();
    let snap = enc.snapshot().unwrap();
    enc.restore(snap).unwrap();
    assert_eq!(enc.page_count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn text_selection_and_annotations_round_trip() {
    use sunipdf_core::{MarkupKind, PointRect};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-annot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    // Le texte « SuniPDF page 1 » (24 pt) est placé en (72, 700) : rectangle large autour.
    let sel = doc
        .text_in_rect(
            0,
            PointRect {
                left: 60.0,
                bottom: 690.0,
                right: 400.0,
                top: 730.0,
            },
        )
        .unwrap();
    assert_eq!(sel.text, "SuniPDF page 1");
    assert_eq!(sel.rects.len(), 1, "une seule ligne");
    assert!(sel.rects[0].left >= 60.0 && sel.rects[0].right <= 400.0);
    let empty = doc
        .text_in_rect(
            0,
            PointRect {
                left: 400.0,
                bottom: 100.0,
                right: 500.0,
                top: 150.0,
            },
        )
        .unwrap();
    assert!(empty.text.is_empty());

    doc.add_markup(0, MarkupKind::Highlight, &sel.rects, (255, 230, 0))
        .unwrap();
    doc.add_markup(0, MarkupKind::Underline, &sel.rects, (0, 0, 255))
        .unwrap();
    doc.add_note(0, 300.0, 600.0, "Relire ce paragraphe")
        .unwrap();
    let list = doc.annotations(0).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].kind, "Highlight");
    assert_eq!(list[2].kind, "Text");
    assert_eq!(list[2].contents.as_deref(), Some("Relire ce paragraphe"));

    // Le surlignage est visible au rendu : pixels jaunes dans la zone du texte.
    let img = doc.render_page(0, 1.0).unwrap();
    let yellow = img
        .rgba
        .chunks(4)
        .filter(|p| p[0] > 200 && p[1] > 180 && p[2] < 120)
        .count();
    assert!(yellow > 500, "surlignage rendu ({yellow} pixels)");

    let out = dir.join("annotated.pdf");
    doc.save_as(&out).unwrap();
    let mut again = engine.open(&out, None).unwrap();
    assert_eq!(again.annotations(0).unwrap().len(), 3);
    again.delete_annotation(0, 1).unwrap();
    assert_eq!(again.annotations(0).unwrap().len(), 2);
    assert!(again.delete_annotation(0, 9).is_err());
    assert!(doc
        .add_markup(0, MarkupKind::StrikeOut, &[], (0, 0, 0))
        .is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn bookmarks_and_properties() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let marks = doc.bookmarks();
    assert_eq!(marks.len(), 3, "{marks:?}");
    assert_eq!(
        (marks[0].title.as_str(), marks[0].page, marks[0].depth),
        ("Premiere page", Some(0), 0)
    );
    assert_eq!(
        (marks[1].title.as_str(), marks[1].page, marks[1].depth),
        ("Rectangle bleu", Some(0), 1)
    );
    assert_eq!(
        (marks[2].title.as_str(), marks[2].page, marks[2].depth),
        ("Deuxieme page", Some(1), 0)
    );
    let props = doc.properties();
    assert_eq!(props.metadata.title.as_deref(), Some("SuniPDF fixture"));
    assert_eq!(props.metadata.author.as_deref(), Some("SuniPDF tests"));
    assert_eq!(props.page_count, 2);
    assert!(!props.encrypted);
    assert!(props.can_print && props.can_copy);
    assert_eq!(props.version, "1.4");
    assert!(props.file_size.unwrap() > 1000);
}

#[test]
fn protect_unprotect_and_metadata() {
    use sunipdf_core::{security, Metadata, Protection};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-sec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();

    for aes256 in [false, true] {
        let p = Protection {
            user_password: "user".into(),
            owner_password: "owner".into(),
            allow_print: false,
            allow_copy: false,
            aes256,
            ..Default::default()
        };
        let bytes = security::protect(&input, None, &p).unwrap();
        let path = dir.join(format!("protected-{aes256}.pdf"));
        std::fs::write(&path, &bytes).unwrap();
        assert!(
            matches!(engine.open(&path, None), Err(EngineError::PasswordRequired)),
            "aes256={aes256}"
        );
        assert!(matches!(
            engine.open(&path, Some("nope")),
            Err(EngineError::PasswordRequired)
        ));
        let doc = engine.open(&path, Some("user")).unwrap();
        assert_eq!(doc.page_count(), 2);
        assert!(
            doc.page_text(0).unwrap().contains("SuniPDF page 1"),
            "contenu lisible après déchiffrement"
        );
        let props = doc.properties();
        assert!(props.encrypted);
        assert!(
            !props.can_print && !props.can_copy,
            "permissions appliquées (aes256={aes256})"
        );
        assert!(engine.open(&path, Some("owner")).is_ok());
        // Rendu intact.
        let img = doc.render_page(0, 0.5).unwrap();
        assert!(img.rgba.chunks(4).any(|px| px[2] == 255 && px[0] == 0));

        let clear = security::unprotect(&bytes, "owner").unwrap();
        let clear_path = dir.join(format!("clear-{aes256}.pdf"));
        std::fs::write(&clear_path, &clear).unwrap();
        let reopened = engine.open(&clear_path, None).unwrap();
        assert!(!reopened.properties().encrypted);
        assert_eq!(reopened.page_count(), 2);
        assert!(matches!(
            security::unprotect(&bytes, "wrong"),
            Err(EngineError::PasswordRequired)
        ));
    }
    assert!(
        security::protect(&input, None, &Protection::default()).is_err(),
        "mot de passe obligatoire"
    );

    let meta = Metadata {
        title: Some("Titre modifié".into()),
        author: None,
        subject: Some("Sujet".into()),
        keywords: Some("a, b".into()),
    };
    let edited = security::set_metadata(&input, None, &meta).unwrap();
    let edited_path = dir.join("meta.pdf");
    std::fs::write(&edited_path, &edited).unwrap();
    let props = engine.open(&edited_path, None).unwrap().properties();
    assert_eq!(props.metadata.title.as_deref(), Some("Titre modifié"));
    assert_eq!(props.metadata.author, None);
    assert_eq!(props.metadata.subject.as_deref(), Some("Sujet"));

    let stripped = security::strip_metadata(&input, None).unwrap();
    let stripped_path = dir.join("stripped.pdf");
    std::fs::write(&stripped_path, &stripped).unwrap();
    let props = engine.open(&stripped_path, None).unwrap().properties();
    assert_eq!(props.metadata, Metadata::default());
    assert!(
        !String::from_utf8_lossy(&stripped).contains("SuniPDF tests"),
        "aucune trace de l'auteur"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn images_to_pdf_export_images_and_optimize() {
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-conv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Image rouge 300×150 px en PNG et une copie JPEG.
    let png = dir.join("red.png");
    let rgb: Vec<u8> = (0..300 * 150).flat_map(|_| [220u8, 20, 20]).collect();
    image::save_buffer(&png, &rgb, 300, 150, image::ColorType::Rgb8).unwrap();
    let jpg = dir.join("red.jpg");
    image::save_buffer(&jpg, &rgb, 300, 150, image::ColorType::Rgb8).unwrap();

    let out = dir.join("images.pdf");
    assert_eq!(
        engine
            .images_to_pdf(&[png.clone(), jpg.clone()], &out, 150.0)
            .unwrap(),
        2
    );
    let doc = engine.open(&out, None).unwrap();
    assert_eq!(doc.page_count(), 2);
    let size = doc.page_size(0).unwrap();
    assert!(
        (size.width_pt - 144.0).abs() < 0.5 && (size.height_pt - 72.0).abs() < 0.5,
        "{size:?}"
    );
    let img = doc.render_page(0, 1.0).unwrap();
    let red = img
        .rgba
        .chunks(4)
        .filter(|p| p[0] > 180 && p[1] < 80 && p[2] < 80)
        .count();
    assert!(
        red > (img.width * img.height / 2) as usize,
        "image rendue ({red} px rouges)"
    );
    assert!(engine.images_to_pdf(&[], &out, 150.0).is_err());
    assert!(engine
        .images_to_pdf(&[dir.join("missing.png")], &out, 150.0)
        .is_err());

    // Export de toutes les pages en PNG.
    let exported = engine
        .open(&fixture("two-pages.pdf"), None)
        .unwrap()
        .export_images(&dir.join("png"), 72.0)
        .unwrap();
    assert_eq!(exported.len(), 2);
    let back = image::open(&exported[1]).unwrap();
    assert_eq!((back.width(), back.height()), (792, 612));

    // Optimisation : fichier valide, même contenu, pas plus gros qu'un simple aller-retour.
    let res =
        sunipdf_core::convert::optimize_file(&fixture("two-pages.pdf"), &dir.join("opt.pdf"), None)
            .unwrap();
    assert!(res.before > 0 && res.after > 0);
    let opt = engine.open(&dir.join("opt.pdf"), None).unwrap();
    assert_eq!(opt.page_count(), 2);
    assert!(opt.page_text(0).unwrap().contains("SuniPDF page 1"));
    assert_eq!(
        opt.bookmarks().len(),
        3,
        "les signets survivent à l'optimisation"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn edit_page_content_text_image_shapes() {
    use sunipdf_core::{ObjectKind, ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-edit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let initial = doc.page_objects(0).unwrap();
    assert!(
        initial
            .iter()
            .any(|o| o.kind == ObjectKind::Text && o.text.as_deref() == Some("SuniPDF page 1")),
        "{initial:?}"
    );
    assert!(
        initial.iter().any(|o| o.kind == ObjectKind::Path),
        "le rectangle bleu est un tracé"
    );

    // Texte ajouté avec accents, puis modifié, déplacé, retrouvé par position, supprimé.
    doc.add_text(
        0,
        100.0,
        300.0,
        "Bonjour éditeur",
        &TextStyle {
            size_pt: 18.0,
            bold: true,
            rgb: (200, 0, 0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(doc.page_text(0).unwrap().contains("Bonjour éditeur"));
    let objs = doc.page_objects(0).unwrap();
    let added = objs
        .iter()
        .find(|o| o.text.as_deref() == Some("Bonjour éditeur"))
        .expect("objet texte ajouté");
    assert!((added.font_size - 18.0).abs() < 0.5, "{added:?}");
    assert!(
        added.bounds.left >= 99.0 && added.bounds.bottom <= 301.0 && added.bounds.top > 300.0,
        "{:?}",
        added.bounds
    );
    let hit = doc
        .object_at(0, added.bounds.left + 5.0, added.bounds.bottom + 3.0)
        .unwrap()
        .expect("objet sous le point");
    assert_eq!(hit.index, added.index);
    assert!(doc.object_at(0, 600.0, 10.0).unwrap().is_none());
    doc.set_object_text(0, added.index, "Texte modifié")
        .unwrap();
    assert!(doc.page_text(0).unwrap().contains("Texte modifié"));
    assert!(!doc.page_text(0).unwrap().contains("Bonjour"));
    let before_move = doc.object_info(0, added.index).unwrap();
    doc.move_object(0, added.index, 50.0, -20.0).unwrap();
    let moved = doc.object_info(0, added.index).unwrap();
    assert!(
        (moved.bounds.left - before_move.bounds.left - 50.0).abs() < 1.0,
        "{:?}",
        moved.bounds
    );
    assert!((moved.bounds.bottom - before_move.bounds.bottom + 20.0).abs() < 1.0);

    // Modification du texte d'origine du document.
    let title = initial
        .iter()
        .find(|o| o.text.as_deref() == Some("SuniPDF page 1"))
        .unwrap();
    // Police d'origine (Type1 sans /Encoding) : l'ASCII est sûr, les accents dépendent de l'encodage.
    doc.set_object_text(0, title.index, "SuniPDF modifie")
        .unwrap();
    let t = doc.page_text(0).unwrap();
    assert!(t.contains("SuniPDF modifie"), "{t}");
    assert!(doc.set_object_text(0, 99, "x").is_err());
    assert!(doc
        .add_text(0, 0.0, 0.0, "   ", &TextStyle::default())
        .is_err());

    // Image et formes : pixels visibles au rendu.
    let png = dir.join("green.png");
    let rgb: Vec<u8> = (0..40 * 20).flat_map(|_| [0u8, 160, 0]).collect();
    image::save_buffer(&png, &rgb, 40, 20, image::ColorType::Rgb8).unwrap();
    doc.add_image(0, &png, 300.0, 100.0, 200.0).unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        sunipdf_core::PointRect {
            left: 400.0,
            bottom: 300.0,
            right: 500.0,
            top: 400.0,
        },
        &ShapeStyle {
            stroke_rgb: (0, 0, 0),
            stroke_width: 1.0,
            fill_rgb: Some((255, 0, 255)),
        },
    )
    .unwrap();
    doc.add_shape(
        0,
        ShapeKind::Line,
        sunipdf_core::PointRect {
            left: 50.0,
            bottom: 55.0,
            right: 550.0,
            top: 55.0,
        },
        &ShapeStyle::default(),
    )
    .unwrap();
    doc.add_shape(
        0,
        ShapeKind::Ellipse,
        sunipdf_core::PointRect {
            left: 100.0,
            bottom: 350.0,
            right: 200.0,
            top: 450.0,
        },
        &ShapeStyle::default(),
    )
    .unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    let px = |x: usize, y_top: usize| {
        let i = (y_top * 612 + x) * 4;
        (img.rgba[i], img.rgba[i + 1], img.rgba[i + 2])
    };
    assert_eq!(
        px(400, 792 - 150),
        (0, 160, 0),
        "image verte à (300..500, 100..200)"
    );
    assert_eq!(
        px(450, 792 - 350),
        (255, 0, 255),
        "rectangle magenta rempli"
    );
    let blue_line = (50..550)
        .filter(|&x| {
            let p = px(x, 792 - 55);
            p.2 > 150 && p.0 < 100
        })
        .count();
    assert!(blue_line > 300, "ligne bleue ({blue_line} px)");

    // Suppression, puis persistance après enregistrement.
    let count_before = doc.page_objects(0).unwrap().len();
    doc.delete_object(0, added.index).unwrap();
    assert_eq!(doc.page_objects(0).unwrap().len(), count_before - 1);
    assert!(!doc.page_text(0).unwrap().contains("Texte modifié"));
    assert!(doc.delete_object(0, 999).is_err());
    let out = dir.join("edited.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert!(again.page_text(0).unwrap().contains("SuniPDF modifie"));
    let again_img = again.render_page(0, 1.0).unwrap();
    let i = ((792 - 150) * 612 + 400) * 4;
    assert_eq!((again_img.rgba[i], again_img.rgba[i + 1]), (0, 160));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn edit_resize_style_text_size_and_fonts() {
    use sunipdf_core::{FontFamily, ObjectStyle, PointRect, ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    // Rectangle magenta plein, redimensionné puis recoloré en vert : pixels vérifiés.
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        PointRect {
            left: 100.0,
            bottom: 100.0,
            right: 200.0,
            top: 150.0,
        },
        &ShapeStyle {
            stroke_rgb: (0, 0, 0),
            stroke_width: 1.0,
            fill_rgb: Some((255, 0, 255)),
        },
    )
    .unwrap();
    let index = doc.page_objects(0).unwrap().len() - 1;
    let info = doc.object_info(0, index).unwrap();
    assert_eq!(info.fill_rgb, Some((255, 0, 255)));
    assert_eq!(info.stroke_rgb, Some((0, 0, 0)));
    doc.resize_object(
        0,
        index,
        PointRect {
            left: 300.0,
            bottom: 400.0,
            right: 500.0,
            top: 600.0,
        },
    )
    .unwrap();
    let b = doc.object_info(0, index).unwrap().bounds;
    assert!(
        (b.left - 300.0).abs() < 1.5
            && (b.right - 500.0).abs() < 1.5
            && (b.bottom - 400.0).abs() < 1.5
            && (b.top - 600.0).abs() < 1.5,
        "{b:?}"
    );
    doc.set_object_style(
        0,
        index,
        &ObjectStyle {
            fill_rgb: Some((0, 160, 0)),
            stroke_rgb: None,
            stroke_width: Some(0.5),
        },
    )
    .unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    let px = |x: usize, y_top: usize| {
        let i = (y_top * 612 + x) * 4;
        (img.rgba[i], img.rgba[i + 1], img.rgba[i + 2])
    };
    assert_eq!(
        px(400, 792 - 500),
        (0, 160, 0),
        "rectangle redimensionné et recoloré"
    );
    assert_eq!(
        px(150, 792 - 125),
        (255, 255, 255),
        "ancienne position vide"
    );
    assert!(doc
        .resize_object(
            0,
            index,
            PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 10.0
            }
        )
        .is_err());

    // Texte Times italique rouge, puis taille changée : hauteur des limites doublée.
    doc.add_text(
        0,
        100.0,
        700.0,
        "Times italique",
        &TextStyle {
            size_pt: 10.0,
            bold: false,
            italic: true,
            family: FontFamily::Times,
            rgb: (200, 0, 0),
        },
    )
    .unwrap();
    let t = doc
        .page_objects(0)
        .unwrap()
        .into_iter()
        .find(|o| o.text.as_deref() == Some("Times italique"))
        .unwrap();
    assert!(
        t.font_name
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("times"),
        "{:?}",
        t.font_name
    );
    assert_eq!(t.fill_rgb, Some((200, 0, 0)));
    let h1 = t.bounds.top - t.bounds.bottom;
    doc.set_text_size(0, t.index, 20.0).unwrap();
    let t2 = doc.object_info(0, t.index).unwrap();
    assert!((t2.font_size - 20.0).abs() < 0.5, "{t2:?}");
    let h2 = t2.bounds.top - t2.bounds.bottom;
    assert!((h2 / h1 - 2.0).abs() < 0.2, "{h1} → {h2}");
    assert!(doc.set_text_size(0, index, 20.0).is_err(), "pas un texte");
    assert!(doc.page_text(0).unwrap().contains("Times italique"));
}

#[test]
fn edit_rotate_duplicate_reorder() {
    use sunipdf_core::{PointRect, ShapeKind, ShapeStyle};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let filled = |rgb| ShapeStyle {
        stroke_rgb: rgb,
        stroke_width: 0.5,
        fill_rgb: Some(rgb),
    };
    // Rectangle 100 × 50 pivoté de 90° : limites 50 × 100 autour du même centre.
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        PointRect {
            left: 300.0,
            bottom: 300.0,
            right: 400.0,
            top: 350.0,
        },
        &filled((255, 0, 0)),
    )
    .unwrap();
    let red = doc.page_objects(0).unwrap().len() - 1;
    doc.rotate_object(0, red, 90.0).unwrap();
    let b = doc.object_info(0, red).unwrap().bounds;
    assert!(
        (b.right - b.left - 50.0).abs() < 1.5 && (b.top - b.bottom - 100.0).abs() < 1.5,
        "{b:?}"
    );
    assert!(
        ((b.left + b.right) / 2.0 - 350.0).abs() < 1.0
            && ((b.bottom + b.top) / 2.0 - 325.0).abs() < 1.0,
        "{b:?}"
    );
    assert!(doc.rotate_object(0, red, f32::NAN).is_err());

    // Duplication : un objet de plus, décalé de (+12, −12).
    let before = doc.page_objects(0).unwrap().len();
    let copy = doc.duplicate_object(0, red).unwrap();
    assert_eq!(doc.page_objects(0).unwrap().len(), before + 1);
    let cb = doc.object_info(0, copy).unwrap().bounds;
    assert!(
        (cb.left - b.left - 12.0).abs() < 1.0 && (cb.bottom - b.bottom + 12.0).abs() < 1.0,
        "{cb:?}"
    );

    // Ordre Z : vert dessiné après rouge au même endroit → vert visible ; envoyé à l'arrière → rouge.
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        PointRect {
            left: 100.0,
            bottom: 100.0,
            right: 200.0,
            top: 200.0,
        },
        &filled((200, 0, 0)),
    )
    .unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        PointRect {
            left: 100.0,
            bottom: 100.0,
            right: 200.0,
            top: 200.0,
        },
        &filled((0, 160, 0)),
    )
    .unwrap();
    let green = doc.page_objects(0).unwrap().len() - 1;
    let px = |doc: &sunipdf_core::Document| {
        let img = doc.render_page(0, 1.0).unwrap();
        let i = ((792 - 150) * 612 + 150) * 4;
        (img.rgba[i], img.rgba[i + 1], img.rgba[i + 2])
    };
    assert_eq!(px(&doc), (0, 160, 0));
    let back = doc.reorder_object(0, green, false).unwrap();
    assert_eq!(back, 0);
    assert_eq!(px(&doc), (200, 0, 0), "vert passé derrière le rouge");
    let front = doc.reorder_object(0, back, true).unwrap();
    assert_eq!(front, doc.page_objects(0).unwrap().len() - 1);
    assert_eq!(px(&doc), (0, 160, 0), "vert revenu devant");
    assert!(doc.reorder_object(0, 999, true).is_err());
}
