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

    let parts = engine
        .split(&merged, None, 3, &dir.join("parts"), false)
        .unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(engine.open(&parts[0], None).unwrap().page_count(), 3);
    assert_eq!(engine.open(&parts[1], None).unwrap().page_count(), 1);
    assert!(parts[1].ends_with("merged-2.pdf"));
    // Aucun écrasement silencieux : les morceaux existent déjà.
    assert!(matches!(
        engine.split(&merged, None, 3, &dir.join("parts"), false),
        Err(EngineError::AlreadyExists(_))
    ));
    assert_eq!(
        engine
            .split(&merged, None, 3, &dir.join("parts"), true)
            .unwrap()
            .len(),
        2
    );
    assert!(engine.split(&merged, None, 0, &dir, false).is_err());
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
        .export_images(&dir.join("png"), 72.0, false)
        .unwrap();
    assert!(matches!(
        engine
            .open(&fixture("two-pages.pdf"), None)
            .unwrap()
            .export_images(&dir.join("png"), 72.0, false),
        Err(EngineError::AlreadyExists(_))
    ));
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
            ..Default::default()
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
            ..Default::default()
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
            opacity: None,
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
            opacity: 1.0,
            font_file: None,
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
        ..Default::default()
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

#[test]
fn stamp_watermark_and_page_numbers() {
    use sunipdf_core::{Stamp, StampPosition, TextStyle};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-stamp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let before: Vec<usize> = (0..2).map(|p| doc.page_objects(p).unwrap().len()).collect();
    let stamp = Stamp {
        text: "CONFIDENTIEL {n}".into(),
        ..Default::default()
    };
    assert_eq!(doc.stamp_pages(None, &stamp).unwrap(), 2);
    for (p, &count_before) in before.iter().enumerate() {
        let objs = doc.page_objects(p).unwrap();
        assert_eq!(objs.len(), count_before + 1);
        let last = objs.last().unwrap();
        assert_eq!(
            last.text.as_deref(),
            Some(format!("CONFIDENTIEL {}", p + 1).as_str())
        );
        let (cx, cy) = (
            (last.bounds.left + last.bounds.right) / 2.0,
            (last.bounds.bottom + last.bounds.top) / 2.0,
        );
        let size = doc.page_size(p).unwrap();
        assert!(
            (cx - size.width_pt / 2.0).abs() < 3.0 && (cy - size.height_pt / 2.0).abs() < 3.0,
            "centre page {p} : {cx} {cy}"
        );
        assert!(
            last.bounds.top - last.bounds.bottom > 100.0,
            "pivoté à 45° : {:?}",
            last.bounds
        );
    }
    // Opacité 0,3 : gris atténué au rendu, jamais le gris plein (128).
    let img = doc.render_page(0, 1.0).unwrap();
    let faint = (0..img.rgba.len() / 4)
        .filter(|i| {
            let p = &img.rgba[i * 4..i * 4 + 3];
            p[0] == p[1] && p[1] == p[2] && p[0] > 150 && p[0] < 240
        })
        .count();
    assert!(faint > 500, "pixels gris atténués : {faint}");
    assert!(
        doc.stamp_pages(None, &Stamp::default()).is_err(),
        "texte vide"
    );
    assert!(
        doc.stamp_pages(Some(&[5]), &stamp).is_err(),
        "page hors limites"
    );

    // Numéros de page en bas à droite, page 2 seulement.
    let numbers = Stamp::page_numbers("Page {n} / {total}", StampPosition::BottomRight);
    assert_eq!(doc.stamp_pages(Some(&[1]), &numbers).unwrap(), 1);
    let objs = doc.page_objects(1).unwrap();
    let num = objs.last().unwrap();
    assert_eq!(num.text.as_deref(), Some("Page 2 / 2"));
    let size = doc.page_size(1).unwrap();
    assert!(
        (num.bounds.right - (size.width_pt - 28.0)).abs() < 1.5
            && (num.bounds.bottom - 28.0).abs() < 1.5,
        "{:?}",
        num.bounds
    );
    assert!(!doc.page_text(0).unwrap().contains("Page 1 / 2"));
    // Persistance.
    let out = dir.join("stamped.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert!(again.page_text(0).unwrap().contains("CONFIDENTIEL 1"));
    assert!(again.page_text(1).unwrap().contains("Page 2 / 2"));
    let style = TextStyle {
        opacity: 2.0,
        ..Default::default()
    };
    let mut doc2 = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert!(
        doc2.add_text(0, 10.0, 10.0, "x", &style).is_ok(),
        "opacité bornée"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lopdf_round_trips_keep_encryption_and_unprotect_needs_owner_password() {
    use sunipdf_core::{convert, security, Metadata, Protection};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-keepenc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();

    // Protection par mot de passe propriétaire seul : le document s'ouvre sans mot de passe,
    // mais lever la protection exige le mot de passe propriétaire.
    let owner_only = security::protect(
        &input,
        None,
        &Protection {
            owner_password: "own".into(),
            allow_copy: false,
            ..Default::default()
        },
    )
    .unwrap();
    let owner_only_path = dir.join("owner-only.pdf");
    std::fs::write(&owner_only_path, &owner_only).unwrap();
    let props = engine.open(&owner_only_path, None).unwrap().properties();
    assert!(
        props.encrypted && !props.can_copy && props.can_print,
        "protection propriétaire seule visible : {props:?}"
    );
    for wrong in ["", "nimportequoi", "OWN"] {
        assert!(
            matches!(
                security::unprotect(&owner_only, wrong),
                Err(EngineError::PasswordRequired)
            ),
            "mot de passe {wrong:?} refusé"
        );
    }
    let clear = security::unprotect(&owner_only, "own").unwrap();
    let clear_path = dir.join("clear.pdf");
    std::fs::write(&clear_path, &clear).unwrap();
    assert!(
        !engine
            .open(&clear_path, None)
            .unwrap()
            .properties()
            .encrypted
    );
    assert!(
        security::unprotect(&clear, "own").is_err(),
        "plus rien à retirer"
    );

    // Des restrictions sans mot de passe propriétaire distinct seraient trompeuses : refusées.
    for owner in ["", "u"] {
        let p = Protection {
            user_password: "u".into(),
            owner_password: owner.into(),
            allow_print: false,
            ..Default::default()
        };
        assert!(
            security::protect(&input, None, &p).is_err(),
            "owner={owner:?}"
        );
    }

    // Métadonnées, nettoyage et optimisation conservent le chiffrement, les mots de passe et les
    // permissions d'origine.
    for aes256 in [false, true] {
        let protected = security::protect(
            &input,
            None,
            &Protection {
                user_password: "user".into(),
                owner_password: "owner".into(),
                allow_copy: false,
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let meta = Metadata {
            title: Some("Titre protégé".into()),
            ..Default::default()
        };
        let outputs = [
            (
                "meta",
                security::set_metadata(&protected, Some("user"), &meta).unwrap(),
            ),
            (
                "strip",
                security::strip_metadata(&protected, Some("user")).unwrap(),
            ),
            (
                "optimize",
                convert::optimize(&protected, Some("user")).unwrap(),
            ),
        ];
        for (name, bytes) in outputs {
            let path = dir.join(format!("{name}-{aes256}.pdf"));
            std::fs::write(&path, &bytes).unwrap();
            assert!(
                matches!(engine.open(&path, None), Err(EngineError::PasswordRequired)),
                "{name} aes256={aes256} : toujours protégé"
            );
            let doc = engine.open(&path, Some("user")).unwrap();
            let props = doc.properties();
            assert!(props.encrypted && !props.can_copy, "{name} aes256={aes256}");
            assert!(doc.page_text(0).unwrap().contains("SuniPDF page 1"));
            if name == "meta" {
                assert_eq!(props.metadata.title.as_deref(), Some("Titre protégé"));
            }
            assert!(engine.open(&path, Some("owner")).is_ok());
        }
    }

    // Chiffrement RC4 (fixture tierce) conservé lui aussi.
    let rc4 = std::fs::read(fixture("encrypted-rc4.pdf")).unwrap();
    let rc4_meta = security::set_metadata(
        &rc4,
        Some("secret"),
        &Metadata {
            title: Some("RC4".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let rc4_path = dir.join("rc4-meta.pdf");
    std::fs::write(&rc4_path, &rc4_meta).unwrap();
    assert!(matches!(
        engine.open(&rc4_path, None),
        Err(EngineError::PasswordRequired)
    ));
    let doc = engine.open(&rc4_path, Some("secret")).unwrap();
    assert_eq!(doc.properties().metadata.title.as_deref(), Some("RC4"));

    // Entrée qui n'est pas un PDF : erreur de format typée, pas « other ».
    assert!(matches!(
        convert::optimize(b"not a pdf at all", None),
        Err(EngineError::InvalidFormat)
    ));
    assert!(matches!(
        security::protect(
            b"not a pdf",
            None,
            &Protection {
                user_password: "x".into(),
                ..Default::default()
            }
        ),
        Err(EngineError::InvalidFormat)
    ));
    // Optimisation sans gain : jamais de fichier plus gros que l'entrée.
    let src = dir.join("src.pdf");
    std::fs::write(&src, &input).unwrap();
    let opt = convert::optimize_file(&src, &dir.join("opt.pdf"), None).unwrap();
    assert!(opt.after <= opt.before);
    let _ = std::fs::remove_dir_all(dir);
}

// ----- Édition avancée (SUNI-031c) : pages pivotées, opacité, rotation libre, presse-papiers… -----

use sunipdf_core::{PointRect, RenderedPage};

fn rect(left: f32, bottom: f32, right: f32, top: f32) -> PointRect {
    PointRect {
        left,
        bottom,
        right,
        top,
    }
}

fn near(a: PointRect, b: PointRect, tol: f32) -> bool {
    (a.left - b.left).abs() <= tol
        && (a.bottom - b.bottom).abs() <= tol
        && (a.right - b.right).abs() <= tol
        && (a.top - b.top).abs() <= tol
}

/// Pixel au point d'affichage (x, y) d'un rendu à l'échelle 1 (y compté depuis le bas).
fn px(img: &RenderedPage, x: f32, y: f32) -> [u8; 3] {
    let col = (x.floor() as u32).min(img.width - 1);
    let row = ((img.height as f32 - y).floor() as u32).min(img.height - 1);
    let i = ((row * img.width + col) * 4) as usize;
    [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
}

/// Boîte (espace d'affichage, rendu à l'échelle 1) des pixels qui vérifient `pred`.
fn pixel_box(img: &RenderedPage, pred: impl Fn([u8; 3]) -> bool) -> Option<PointRect> {
    pixel_box_below(img, f32::MAX, pred)
}

/// Comme [`pixel_box`], limité aux pixels sous l'ordonnée d'affichage `max_y`.
fn pixel_box_below(
    img: &RenderedPage,
    max_y: f32,
    pred: impl Fn([u8; 3]) -> bool,
) -> Option<PointRect> {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut out: Option<PointRect> = None;
    for y in 0..h {
        if (h - y) as f32 > max_y {
            continue;
        }
        for x in 0..w {
            let i = (y * w + x) * 4;
            if pred([img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]) {
                let r = rect(x as f32, (h - y - 1) as f32, x as f32 + 1.0, (h - y) as f32);
                out = Some(match out {
                    None => r,
                    Some(u) => rect(
                        u.left.min(r.left),
                        u.bottom.min(r.bottom),
                        u.right.max(r.right),
                        u.top.max(r.top),
                    ),
                });
            }
        }
    }
    out
}

fn is_red(p: [u8; 3]) -> bool {
    p[0] > 200 && p[1] < 60 && p[2] < 60
}

fn is_blue(p: [u8; 3]) -> bool {
    p[2] > 200 && p[0] < 60 && p[1] < 60
}

fn is_dark(p: [u8; 3]) -> bool {
    p[0] < 90 && p[1] < 90 && p[2] < 90
}

fn close3(p: [u8; 3], q: [u8; 3], tol: i32) -> bool {
    (0..3).all(|i| (p[i] as i32 - q[i] as i32).abs() <= tol)
}

fn center(r: PointRect) -> (f32, f32) {
    ((r.left + r.right) / 2.0, (r.bottom + r.top) / 2.0)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sunipdf-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(dir: &Path, name: &str, w: u32, h: u32, rgb: [u8; 3]) -> PathBuf {
    let path = dir.join(name);
    let data: Vec<u8> = (0..w * h).flat_map(|_| rgb).collect();
    image::save_buffer(&path, &data, w, h, image::ColorType::Rgb8).unwrap();
    path
}

/// (rectangle rouge, rectangle bleu) de chaque page de `boxes.pdf`, espace utilisateur.
const BOXES: [((f32, f32), (f32, f32)); 4] = [
    ((50.0, 50.0), (460.0, 640.0)),
    ((50.0, 50.0), (460.0, 640.0)),
    ((160.0, 110.0), (540.0, 590.0)),
    ((110.0, 110.0), (390.0, 490.0)),
];

#[test]
fn geometry_matches_rendered_pixels_on_rotated_and_cropped_pages() {
    use sunipdf_core::ObjectKind;
    let engine = engine();
    let doc = engine.open(&fixture("boxes.pdf"), None).unwrap();
    let sizes = [
        (792.0, 612.0),
        (612.0, 792.0),
        (600.0, 500.0),
        (400.0, 500.0),
    ];
    for (p, (&(w, h), &(red, blue))) in sizes.iter().zip(BOXES.iter()).enumerate() {
        let size = doc.page_size(p).unwrap();
        assert_eq!((size.width_pt, size.height_pt), (w, h), "page {p}");
        let geo = doc.page_geometry(p).unwrap();
        assert_eq!(geo.display_size(), (w, h));
        let img = doc.render_page(p, 1.0).unwrap();
        for (user, pred, rgb) in [
            (red, is_red as fn([u8; 3]) -> bool, (255, 0, 0)),
            (blue, is_blue as fn([u8; 3]) -> bool, (0, 0, 255)),
        ] {
            let expected =
                geo.rect_to_display(rect(user.0, user.1, user.0 + 100.0, user.1 + 100.0));
            let seen = pixel_box(&img, pred).expect("pixels du carré");
            assert!(
                near(seen, expected, 1.5),
                "page {p} : rendu {seen:?} ≠ {expected:?}"
            );
            let obj = doc
                .page_objects(p)
                .unwrap()
                .into_iter()
                .find(|o| o.kind == ObjectKind::Path && o.fill_rgb == Some(rgb))
                .unwrap();
            assert!(
                near(obj.bounds, expected, 1.0),
                "page {p} : {:?}",
                obj.bounds
            );
            let (cx, cy) = center(expected);
            assert_eq!(
                doc.object_at(p, cx, cy).unwrap().map(|o| o.index),
                Some(obj.index)
            );
        }
        assert!(doc.object_at(p, 5.0, 5.0).unwrap().is_none(), "page {p}");
    }
    // Valeurs attendues écrites en dur (pas seulement via les formules) : page A, R90.
    let a = doc.page_geometry(0).unwrap();
    assert_eq!(
        a.rect_to_display(rect(50.0, 50.0, 150.0, 150.0)),
        rect(50.0, 462.0, 150.0, 562.0)
    );
    // Page C : boîte rognée [150 100 650 700], R270.
    let c = doc.page_geometry(2).unwrap();
    assert_eq!(
        c.rect_to_display(rect(160.0, 110.0, 260.0, 210.0)),
        rect(490.0, 10.0, 590.0, 110.0)
    );
}

#[test]
fn text_and_markup_rects_follow_display_space_on_rotated_pages() {
    use sunipdf_core::MarkupKind;
    let engine = engine();
    let mut doc = engine.open(&fixture("boxes.pdf"), None).unwrap();
    for (p, name) in [(0usize, "BOXES A"), (2, "BOXES C")] {
        let img = doc.render_page(p, 1.0).unwrap();
        let dark = pixel_box(&img, is_dark).expect("texte noir");
        // Texte horizontal en espace utilisateur, vertical à l'écran sur ces pages.
        assert!(
            dark.top - dark.bottom > dark.right - dark.left,
            "page {p} : {dark:?}"
        );
        let found = doc.search_page(p, "BOXES", false).unwrap();
        let r = found[0].rects[0];
        let (c1, c2) = (center(r), center(dark));
        assert!(
            (c1.0 - c2.0).abs() < 6.0 && r.top - r.bottom > r.right - r.left,
            "page {p} : recherche {r:?} / pixels {dark:?}"
        );
        let grown = rect(
            dark.left - 4.0,
            dark.bottom - 4.0,
            dark.right + 4.0,
            dark.top + 4.0,
        );
        let sel = doc.text_in_rect(p, grown).unwrap();
        assert_eq!(sel.text, name, "page {p}");
        assert!(!sel.rects.is_empty());
        doc.add_markup(p, MarkupKind::Highlight, &sel.rects, (255, 230, 0))
            .unwrap();
        let ann = doc.annotations(p).unwrap();
        let hl = ann.last().unwrap();
        assert!(
            hl.rect.left <= dark.left + 2.0 && hl.rect.right >= dark.right - 2.0,
            "page {p} : surlignage {:?} / texte {dark:?}",
            hl.rect
        );
        let img = doc.render_page(p, 1.0).unwrap();
        let (hx, hy) = center(hl.rect);
        let yellow = (-6..=6)
            .flat_map(|dx| (-20..=20).map(move |dy| (dx, dy)))
            .filter(|(dx, dy)| {
                let q = px(&img, hx + *dx as f32, hy + *dy as f32);
                q[0] > 200 && q[1] > 170 && q[2] < 140
            })
            .count();
        assert!(yellow > 20, "page {p} : pixels jaunes {yellow}");
        doc.add_note(p, 300.0, 300.0, "n").unwrap();
        let note = doc.annotations(p).unwrap().last().unwrap().rect;
        assert!(
            (note.left - 300.0).abs() < 1.0 && (note.top - 300.0).abs() < 1.0,
            "page {p} : {note:?}"
        );
    }
}

#[test]
fn edits_land_where_displayed_on_rotated_pages() {
    use sunipdf_core::{ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let dir = temp_dir("rotated-edits");
    let green = png(&dir, "green.png", 40, 20, [0, 160, 0]);
    let mut doc = engine.open(&fixture("boxes.pdf"), None).unwrap();
    for p in 0..3 {
        let style = ShapeStyle {
            stroke_rgb: (255, 0, 255),
            stroke_width: 1.0,
            fill_rgb: Some((255, 0, 255)),
            ..Default::default()
        };
        doc.add_shape(
            p,
            ShapeKind::Rectangle,
            rect(300.0, 200.0, 340.0, 240.0),
            &style,
        )
        .unwrap();
        let shape = doc.page_objects(p).unwrap().len() - 1;
        assert!(near(
            doc.object_info(p, shape).unwrap().bounds,
            rect(300.0, 200.0, 340.0, 240.0),
            1.0
        ));
        let img = doc.render_page(p, 1.0).unwrap();
        assert_eq!(px(&img, 320.0, 220.0), [255, 0, 255], "page {p}");

        assert_eq!(
            doc.add_text(
                p,
                60.0,
                300.0,
                "Droit",
                &TextStyle {
                    size_pt: 20.0,
                    ..Default::default()
                }
            )
            .unwrap(),
            1
        );
        let text = doc.page_objects(p).unwrap().len() - 1;
        let t = doc.object_info(p, text).unwrap();
        assert!(
            t.angle_deg.abs() < 0.5,
            "page {p} : texte droit, angle {}",
            t.angle_deg
        );
        assert!(
            (t.bounds.left - 60.0).abs() < 3.0 && t.bounds.bottom <= 301.0,
            "{:?}",
            t.bounds
        );
        assert!(t.bounds.right - t.bounds.left > t.bounds.top - t.bounds.bottom);
        let (cx, cy) = center(t.bounds);
        assert_eq!(
            doc.object_at(p, cx, cy).unwrap().map(|o| o.index),
            Some(text)
        );

        doc.add_image(p, &green, 150.0, 300.0, 120.0).unwrap();
        let image = doc.page_objects(p).unwrap().len() - 1;
        let ib = doc.object_info(p, image).unwrap();
        assert!(
            near(ib.bounds, rect(150.0, 300.0, 270.0, 360.0), 1.0),
            "page {p} : {:?}",
            ib.bounds
        );
        assert!(ib.angle_deg.abs() < 0.5);
        let img = doc.render_page(p, 1.0).unwrap();
        assert_eq!(px(&img, 210.0, 330.0), [0, 160, 0], "page {p}");
        let dark = (0..40)
            .flat_map(|dx| (0..20).map(move |dy| (dx, dy)))
            .filter(|(dx, dy)| is_dark(px(&img, 62.0 + *dx as f32, 300.0 + *dy as f32)))
            .count();
        assert!(dark > 20, "page {p} : texte rendu ({dark})");

        doc.move_object(p, text, 40.0, 0.0).unwrap();
        let moved = doc.object_info(p, text).unwrap().bounds;
        assert!(
            (moved.left - t.bounds.left - 40.0).abs() < 0.5
                && (moved.bottom - t.bounds.bottom).abs() < 0.5
        );
        doc.resize_object(p, shape, rect(300.0, 200.0, 360.0, 260.0))
            .unwrap();
        assert!(near(
            doc.object_info(p, shape).unwrap().bounds,
            rect(300.0, 200.0, 360.0, 260.0),
            1.5
        ));
        doc.rotate_object(p, image, 90.0).unwrap();
        let rb = doc.object_info(p, image).unwrap().bounds;
        assert!(
            (rb.right - rb.left - 60.0).abs() < 1.5 && (rb.top - rb.bottom - 120.0).abs() < 1.5,
            "{rb:?}"
        );
    }
    let out = dir.join("rotated.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    for p in 0..3 {
        let img = again.render_page(p, 1.0).unwrap();
        assert_eq!(px(&img, 330.0, 230.0), [255, 0, 255], "page {p} rouverte");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn free_rotation_reports_angle_and_oriented_quad() {
    use sunipdf_core::{ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let red = ShapeStyle {
        stroke_rgb: (255, 0, 0),
        stroke_width: 0.5,
        fill_rgb: Some((255, 0, 0)),
        ..Default::default()
    };
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(300.0, 300.0, 400.0, 350.0),
        &red,
    )
    .unwrap();
    let i = doc.page_objects(0).unwrap().len() - 1;
    doc.rotate_object(0, i, 30.0).unwrap();
    let o = doc.object_info(0, i).unwrap();
    assert!((o.angle_deg - 30.0).abs() < 0.5, "{}", o.angle_deg);
    let (w, h) = (
        o.bounds.right - o.bounds.left,
        o.bounds.top - o.bounds.bottom,
    );
    assert!(
        (w - 111.6).abs() < 2.0 && (h - 93.3).abs() < 2.0,
        "{w} × {h}"
    );
    let c = center(o.bounds);
    assert!(
        (c.0 - 350.0).abs() < 1.0 && (c.1 - 325.0).abs() < 1.0,
        "{c:?}"
    );
    doc.rotate_object(0, i, 15.0).unwrap();
    assert!((doc.object_info(0, i).unwrap().angle_deg - 45.0).abs() < 0.5);
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(doc.rotate_object(0, i, bad).is_err());
    }
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(is_red(px(&img, 350.0, 325.0)), "centre du rectangle pivoté");
    assert_eq!(
        px(&img, 398.0, 373.0),
        [255, 255, 255],
        "coin de la boîte, hors du rectangle"
    );

    doc.add_text(
        0,
        300.0,
        120.0,
        "Rotated text",
        &TextStyle {
            size_pt: 20.0,
            ..Default::default()
        },
    )
    .unwrap();
    let t = doc.page_objects(0).unwrap().len() - 1;
    doc.rotate_object(0, t, 20.0).unwrap();
    let info = doc.object_info(0, t).unwrap();
    assert!((info.angle_deg - 20.0).abs() < 1.0, "{}", info.angle_deg);
    // Arête la plus longue du cadre orienté : pente de -20° (sens horaire).
    let mut q = info.quad;
    let c = center(info.bounds);
    q.sort_by(|a, b| {
        (a.1 - c.1)
            .atan2(a.0 - c.0)
            .total_cmp(&(b.1 - c.1).atan2(b.0 - c.0))
    });
    let mut best = (0.0f32, 0.0f32);
    for k in 0..4 {
        let (a, b) = (q[k], q[(k + 1) % 4]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len > best.0 {
            let mut ang = dy.atan2(dx).to_degrees();
            if ang > 90.0 {
                ang -= 180.0;
            } else if ang <= -90.0 {
                ang += 180.0;
            }
            best = (len, ang);
        }
    }
    assert!((best.1 + 20.0).abs() < 1.0, "pente {}", best.1);
    let b = info.bounds;
    assert!(
        doc.object_at(0, b.left + 1.0, b.top - 1.0)
            .unwrap()
            .is_none(),
        "coin hors du cadre orienté"
    );
    let (cx, cy) = center(b);
    assert_eq!(doc.object_at(0, cx, cy).unwrap().map(|o| o.index), Some(t));
}

#[test]
fn opacity_text_shapes_images_and_style_preservation() {
    use sunipdf_core::{ObjectStyle, ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let dir = temp_dir("opacity");
    let green = png(&dir, "green.png", 40, 20, [0, 160, 0]);
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let style = ShapeStyle {
        stroke_rgb: (255, 0, 255),
        stroke_width: 1.0,
        fill_rgb: Some((255, 0, 255)),
        opacity: 0.5,
    };
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(300.0, 100.0, 400.0, 200.0),
        &style,
    )
    .unwrap();
    let r = doc.page_objects(0).unwrap().len() - 1;
    assert!((doc.object_info(0, r).unwrap().opacity.unwrap() - 0.5).abs() < 0.01);
    let at =
        |doc: &sunipdf_core::Document, x: f32, y: f32| px(&doc.render_page(0, 1.0).unwrap(), x, y);
    assert!(
        close3(at(&doc, 350.0, 150.0), [255, 128, 255], 4),
        "{:?}",
        at(&doc, 350.0, 150.0)
    );
    // Nouvelle couleur sans opacité : l'alpha courant est conservé (défaut corrigé).
    doc.set_object_style(
        0,
        r,
        &ObjectStyle {
            fill_rgb: Some((0, 160, 0)),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        close3(at(&doc, 350.0, 150.0), [128, 207, 128], 4),
        "{:?}",
        at(&doc, 350.0, 150.0)
    );
    doc.set_object_style(
        0,
        r,
        &ObjectStyle {
            opacity: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(at(&doc, 350.0, 150.0), [0, 160, 0]);

    doc.add_image_with(0, &green, 300.0, 250.0, 100.0, 0.5)
        .unwrap();
    let im = doc.page_objects(0).unwrap().len() - 1;
    // PDFium ne donne pas l'alpha d'une image : « transparente, valeur inconnue ».
    assert_eq!(doc.object_info(0, im).unwrap().opacity, None);
    assert!(
        close3(at(&doc, 350.0, 275.0), [128, 207, 128], 4),
        "image : {:?}",
        at(&doc, 350.0, 275.0)
    );
    doc.set_objects_style(
        0,
        &[im],
        &ObjectStyle {
            opacity: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(at(&doc, 350.0, 275.0), [0, 160, 0]);
    assert_eq!(doc.object_info(0, im).unwrap().opacity, Some(1.0));

    let line = ShapeStyle {
        stroke_rgb: (0, 0, 0),
        stroke_width: 6.0,
        fill_rgb: None,
        opacity: 0.5,
    };
    doc.add_shape(0, ShapeKind::Line, rect(450.0, 100.0, 450.0, 300.0), &line)
        .unwrap();
    assert!(
        close3(at(&doc, 450.0, 200.0), [128, 128, 128], 12),
        "{:?}",
        at(&doc, 450.0, 200.0)
    );

    doc.add_text(
        0,
        100.0,
        420.0,
        "Half",
        &TextStyle {
            opacity: 0.5,
            ..Default::default()
        },
    )
    .unwrap();
    let t = doc.page_objects(0).unwrap().len() - 1;
    assert!((doc.object_info(0, t).unwrap().opacity.unwrap() - 0.5).abs() < 0.01);
    // Bornes : 2 → 1, -1 → 0 ; NaN refusé.
    doc.set_object_style(
        0,
        t,
        &ObjectStyle {
            opacity: Some(2.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(doc.object_info(0, t).unwrap().opacity, Some(1.0));
    doc.set_object_style(
        0,
        t,
        &ObjectStyle {
            opacity: Some(-1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(doc.object_info(0, t).unwrap().opacity, Some(0.0));
    assert!(doc
        .set_object_style(
            0,
            t,
            &ObjectStyle {
                opacity: Some(f32::NAN),
                ..Default::default()
            }
        )
        .is_err());
    doc.set_object_style(
        0,
        t,
        &ObjectStyle {
            opacity: Some(0.5),
            ..Default::default()
        },
    )
    .unwrap();
    let out = dir.join("opacity.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    let half = again
        .page_objects(0)
        .unwrap()
        .into_iter()
        .find(|o| o.text.as_deref() == Some("Half"))
        .unwrap();
    assert!(
        (half.opacity.unwrap() - 0.5).abs() < 0.01,
        "opacité relue : {:?}",
        half.opacity
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn system_font_embeds_unicode_and_rejects_missing_glyphs() {
    use sunipdf_core::TextStyle;
    {
        // Police standard (WinAnsi) : le cyrillique est refusé au lieu d'être perdu en silence.
        let engine = engine();
        let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
        let count = doc.page_objects(0).unwrap().len();
        assert!(matches!(
            doc.add_text(0, 60.0, 300.0, "Привет", &TextStyle::default()),
            Err(EngineError::MissingGlyphs)
        ));
        assert_eq!(doc.page_objects(0).unwrap().len(), count);
        assert_eq!(
            doc.add_text(0, 60.0, 300.0, "Été « 5 € »", &TextStyle::default())
                .unwrap(),
            1
        );
    }
    let fonts = sunipdf_core::fonts::system_fonts();
    let Some(font) = ["Arial", "Segoe UI"]
        .iter()
        .find_map(|n| fonts.iter().find(|f| f.name == *n))
        .or(fonts.first())
    else {
        eprintln!("aucune police TrueType système : test ignoré");
        return;
    };
    let engine = engine();
    let dir = temp_dir("fonts");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let base = doc.snapshot().unwrap().len();
    let style = TextStyle {
        size_pt: 18.0,
        font_file: Some(font.path.clone()),
        ..Default::default()
    };
    assert_eq!(
        doc.add_text(0, 60.0, 300.0, "Привет Ωmega ł é", &style)
            .unwrap(),
        1
    );
    assert!(
        doc.page_text(0).unwrap().contains("Привет"),
        "{}",
        doc.page_text(0).unwrap()
    );
    let first = doc.snapshot().unwrap().len();
    assert!(
        (first - base) as u64 >= font.size_bytes / 3,
        "police incorporée : +{} octets pour {} ({} octets)",
        first - base,
        font.name,
        font.size_bytes
    );
    doc.add_text(0, 60.0, 260.0, "Deuxième ligne", &style)
        .unwrap();
    let second = doc.snapshot().unwrap().len();
    assert!(
        second - first < 20_000,
        "même police réutilisée : +{}",
        second - first
    );
    let count = doc.page_objects(0).unwrap().len();
    assert!(matches!(
        doc.add_text(0, 60.0, 220.0, "日本", &style),
        Err(EngineError::MissingGlyphs)
    ));
    assert_eq!(doc.page_objects(0).unwrap().len(), count, "document intact");
    let not_a_font = png(&dir, "x.png", 2, 2, [0, 0, 0]);
    let bad = TextStyle {
        font_file: Some(not_a_font),
        ..Default::default()
    };
    assert!(matches!(
        doc.add_text(0, 60.0, 220.0, "x", &bad),
        Err(EngineError::FontNotEmbeddable)
    ));
    let out = dir.join("fonts.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert!(again.page_text(0).unwrap().contains("Привет"));
    let img = again.render_page(0, 1.0).unwrap();
    let dark = (60..200)
        .flat_map(|x| (300..318).map(move |y| (x, y)))
        .filter(|(x, y)| is_dark(px(&img, *x as f32, *y as f32)))
        .count();
    assert!(dark > 50, "texte Unicode rendu ({dark} px)");

    // Plusieurs lignes : un objet par ligne, interligne 1,2 × taille.
    let plain = TextStyle {
        size_pt: 20.0,
        ..Default::default()
    };
    assert_eq!(doc.add_text(0, 400.0, 400.0, "a\nb", &plain).unwrap(), 2);
    let objs = doc.page_objects(0).unwrap();
    let (a, b) = (&objs[objs.len() - 2], &objs[objs.len() - 1]);
    assert_eq!(
        (a.text.as_deref(), b.text.as_deref()),
        (Some("a"), Some("b"))
    );
    assert!(
        (a.bounds.bottom - b.bounds.bottom - 24.0).abs() < 1.5,
        "{:?} {:?}",
        a.bounds,
        b.bounds
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn multi_object_ops_keep_order_and_are_atomic() {
    use sunipdf_core::{RectMode, ShapeKind, ShapeStyle, ZOrder};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let filled = |rgb| ShapeStyle {
        stroke_rgb: rgb,
        stroke_width: 0.5,
        fill_rgb: Some(rgb),
        ..Default::default()
    };
    let n = doc.page_objects(0).unwrap().len();
    let (red, green, yellow) = ((220, 0, 0), (0, 160, 0), (200, 200, 0));
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(100.0, 100.0, 200.0, 200.0),
        &filled(red),
    )
    .unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(150.0, 150.0, 250.0, 250.0),
        &filled(green),
    )
    .unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(400.0, 100.0, 450.0, 150.0),
        &filled(yellow),
    )
    .unwrap();
    let ids = |v: Vec<sunipdf_core::ObjectInfo>| v.into_iter().map(|o| o.index).collect::<Vec<_>>();
    assert_eq!(
        ids(doc
            .objects_in_rect(0, rect(90.0, 90.0, 260.0, 260.0), RectMode::Contained)
            .unwrap()),
        vec![n, n + 1]
    );
    assert_eq!(
        ids(doc
            .objects_in_rect(0, rect(120.0, 120.0, 180.0, 180.0), RectMode::Contained)
            .unwrap()),
        Vec::<usize>::new()
    );
    assert_eq!(
        ids(doc
            .objects_in_rect(0, rect(190.0, 190.0, 195.0, 195.0), RectMode::Touching)
            .unwrap()),
        vec![n, n + 1]
    );
    let before: Vec<_> = doc
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|o| o.bounds)
        .collect();
    doc.move_objects(0, &[n, n + 2], 30.0, -20.0).unwrap();
    let after: Vec<_> = doc
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|o| o.bounds)
        .collect();
    for i in [n, n + 2] {
        assert!(
            (after[i].left - before[i].left - 30.0).abs() < 0.5
                && (after[i].bottom - before[i].bottom + 20.0).abs() < 0.5
        );
    }
    assert_eq!(after[n + 1], before[n + 1]);
    // Recouvrement rouge/vert en (190, 165) : l'objet dessiné le plus tard est visible.
    let top = |doc: &sunipdf_core::Document| px(&doc.render_page(0, 1.0).unwrap(), 190.0, 165.0);
    let rgb3 = |c: (u8, u8, u8)| [c.0, c.1, c.2];
    assert_eq!(top(&doc), rgb3(green));
    assert_eq!(
        doc.reorder_objects(0, &[n + 1], ZOrder::Back).unwrap(),
        vec![0]
    );
    assert_eq!(top(&doc), rgb3(red));
    assert_eq!(
        doc.reorder_objects(0, &[0], ZOrder::Front).unwrap(),
        vec![n + 2]
    );
    assert_eq!(top(&doc), rgb3(green));
    // Ordre : …, rouge (n), jaune (n+1), vert (n+2). Reculer d'un cran : vert passe sous le jaune.
    assert_eq!(
        doc.reorder_objects(0, &[n + 2], ZOrder::Backward).unwrap(),
        vec![n + 1]
    );
    assert_eq!(top(&doc), rgb3(green));
    assert_eq!(
        doc.reorder_objects(0, &[n + 1], ZOrder::Backward).unwrap(),
        vec![n]
    );
    assert_eq!(top(&doc), rgb3(red));
    assert_eq!(
        doc.reorder_objects(0, &[n], ZOrder::Forward).unwrap(),
        vec![n + 1]
    );
    assert_eq!(top(&doc), rgb3(green));
    // Groupe : rouge et vert ensemble au premier plan, ordre relatif conservé.
    let objs = doc.page_objects(0).unwrap();
    let find = |c| objs.iter().find(|o| o.fill_rgb == Some(c)).unwrap().index;
    let (r, g) = (find(red), find(green));
    let new = doc.reorder_objects(0, &[g, r], ZOrder::Front).unwrap();
    assert_eq!(new.len(), 2);
    let objs = doc.page_objects(0).unwrap();
    assert_eq!(objs.last().unwrap().fill_rgb, Some(green));
    // Suppression groupée (désordre + doublon), atomique en cas d'index invalide.
    let count = objs.len();
    assert!(doc.delete_objects(0, &[0, 999]).is_err());
    assert_eq!(doc.page_objects(0).unwrap().len(), count);
    let y = doc
        .page_objects(0)
        .unwrap()
        .iter()
        .find(|o| o.fill_rgb == Some(yellow))
        .unwrap()
        .index;
    let r = doc
        .page_objects(0)
        .unwrap()
        .iter()
        .find(|o| o.fill_rgb == Some(red))
        .unwrap()
        .index;
    doc.delete_objects(0, &[y, r, r]).unwrap();
    let left = doc.page_objects(0).unwrap();
    assert_eq!(left.len(), count - 2);
    assert!(left.iter().any(|o| o.fill_rgb == Some(green)));
    assert!(!left
        .iter()
        .any(|o| o.fill_rgb == Some(red) || o.fill_rgb == Some(yellow)));
    assert!(doc.move_objects(0, &[], 1.0, 1.0).is_err());
}

#[test]
fn object_clipboard_copy_cut_paste_across_pages_documents_and_rotation() {
    use sunipdf_core::{ObjectKind, ShapeKind, ShapeStyle, TextStyle};
    let engine = engine();
    let dir = temp_dir("clipboard");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let magenta = ShapeStyle {
        stroke_rgb: (255, 0, 255),
        stroke_width: 1.0,
        fill_rgb: Some((255, 0, 255)),
        ..Default::default()
    };
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(300.0, 100.0, 400.0, 160.0),
        &magenta,
    )
    .unwrap();
    doc.add_text(
        0,
        300.0,
        200.0,
        "Clip me",
        &TextStyle {
            size_pt: 16.0,
            ..Default::default()
        },
    )
    .unwrap();
    let objs = doc.page_objects(0).unwrap();
    let (a, b) = (objs.len() - 2, objs.len() - 1);
    let snapshot = |doc: &sunipdf_core::Document| {
        doc.page_objects(0)
            .unwrap()
            .into_iter()
            .map(|o| (o.kind, o.bounds, o.fill_rgb))
            .collect::<Vec<_>>()
    };
    let before = snapshot(&doc);
    let clip = doc.copy_objects(0, &[b, a]).unwrap();
    assert_eq!(clip.count, 2);
    assert_eq!(clip.text, "Clip me");
    assert_eq!(
        snapshot(&doc),
        before,
        "copier ne modifie rien, ordre compris"
    );

    let pasted = doc.paste_objects(1, &clip, 0.0, 0.0).unwrap();
    let info = doc.object_info(1, pasted).unwrap();
    assert_eq!(info.kind, ObjectKind::Form);
    assert!(
        near(info.bounds, clip.bounds, 1.5),
        "{:?} / {:?}",
        info.bounds,
        clip.bounds
    );
    let img = doc.render_page(1, 1.0).unwrap();
    assert_eq!(px(&img, 350.0, 130.0), [255, 0, 255]);
    let offset = doc.paste_objects(1, &clip, 50.0, -30.0).unwrap();
    let ob = doc.object_info(1, offset).unwrap().bounds;
    assert!(
        (ob.left - clip.bounds.left - 50.0).abs() < 1.5
            && (ob.bottom - clip.bounds.bottom + 30.0).abs() < 1.5
    );

    // Dans un autre document, sur une page pivotée (R90) : même aspect, même place.
    let mut other = engine.open(&fixture("boxes.pdf"), None).unwrap();
    let p = other.paste_objects(0, &clip, 0.0, 0.0).unwrap();
    let pi = other.object_info(0, p).unwrap();
    assert!(pi.angle_deg.abs() < 0.5, "{}", pi.angle_deg);
    assert!(near(pi.bounds, clip.bounds, 1.5), "{:?}", pi.bounds);
    assert_eq!(
        px(&other.render_page(0, 1.0).unwrap(), 350.0, 130.0),
        [255, 0, 255]
    );

    // Depuis une page pivotée : le texte vertical à l'écran le reste après collage.
    let a_text = other
        .page_objects(0)
        .unwrap()
        .into_iter()
        .find(|o| o.text.as_deref() == Some("BOXES A"))
        .unwrap();
    let tall = other.copy_objects(0, &[a_text.index]).unwrap();
    assert!(tall.bounds.top - tall.bounds.bottom > tall.bounds.right - tall.bounds.left);
    let q = doc.paste_objects(0, &tall, 0.0, 0.0).unwrap();
    let qb = doc.object_info(0, q).unwrap().bounds;
    assert!(near(qb, tall.bounds, 2.0), "{qb:?} / {:?}", tall.bounds);

    // Couper = copier puis supprimer.
    let count = doc.page_objects(0).unwrap().len();
    let cut = doc.copy_objects(0, &[a]).unwrap();
    doc.delete_objects(0, &[a]).unwrap();
    assert_eq!(doc.page_objects(0).unwrap().len(), count - 1);
    assert_eq!(cut.count, 1);
    assert!(doc.copy_objects(0, &[]).is_err());
    assert!(doc.copy_objects(0, &[999]).is_err());

    // Dupliquer : l'original garde son plan (le collé passe devant).
    let objs = doc.page_objects(0).unwrap();
    let first_path = objs
        .iter()
        .find(|o| o.kind == ObjectKind::Path)
        .unwrap()
        .clone();
    let dup = doc.duplicate_object(0, first_path.index).unwrap();
    let objs = doc.page_objects(0).unwrap();
    assert_eq!(
        objs[first_path.index].bounds, first_path.bounds,
        "original inchangé, même index"
    );
    assert_eq!(dup, objs.len() - 1);

    let out = dir.join("pasted.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert!(again
        .page_objects(1)
        .unwrap()
        .iter()
        .any(|o| o.kind == ObjectKind::Form));
    assert_eq!(
        px(&again.render_page(1, 1.0).unwrap(), 350.0, 130.0),
        [255, 0, 255]
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn replace_image_keeps_frame_and_updates_pixels() {
    use sunipdf_core::ObjectKind;
    let engine = engine();
    let dir = temp_dir("replace-image");
    let green = png(&dir, "green.png", 40, 20, [0, 160, 0]);
    let red = png(&dir, "red.png", 20, 40, [230, 0, 0]);
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.add_image(0, &green, 300.0, 100.0, 200.0).unwrap();
    let i = doc.page_objects(0).unwrap().len() - 1;
    doc.replace_image(0, i, &red).unwrap();
    let info = doc.object_info(0, i).unwrap();
    assert_eq!(info.kind, ObjectKind::Image);
    assert!(
        near(info.bounds, rect(375.0, 100.0, 425.0, 200.0), 1.0),
        "{:?}",
        info.bounds
    );
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!(px(&img, 400.0, 150.0), [230, 0, 0]);
    assert_eq!(
        px(&img, 310.0, 110.0),
        [255, 255, 255],
        "ancien cadre libéré"
    );
    assert!(
        pixel_box(&img, |p| p == [0, 160, 0]).is_none(),
        "plus de vert"
    );

    // Image pivotée de 90° : cadre conservé (étirée).
    doc.add_image(0, &green, 100.0, 300.0, 100.0).unwrap();
    let j = doc.page_objects(0).unwrap().len() - 1;
    doc.rotate_object(0, j, 90.0).unwrap();
    let frame = doc.object_info(0, j).unwrap().bounds;
    doc.replace_image(0, j, &red).unwrap();
    assert!(near(doc.object_info(0, j).unwrap().bounds, frame, 1.0));

    let count = doc.page_objects(0).unwrap().len();
    let text = doc
        .page_objects(0)
        .unwrap()
        .into_iter()
        .find(|o| o.kind == ObjectKind::Text)
        .unwrap();
    assert!(
        doc.replace_image(0, text.index, &red).is_err(),
        "pas une image"
    );
    assert!(doc.replace_image(0, i, &dir.join("missing.png")).is_err());
    assert_eq!(doc.page_objects(0).unwrap().len(), count);
    assert_eq!(
        doc.object_info(0, i).unwrap().bounds,
        info.bounds,
        "document intact"
    );
    let out = dir.join("replaced.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert_eq!(
        px(&again.render_page(0, 1.0).unwrap(), 400.0, 150.0),
        [230, 0, 0]
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn crop_resize_and_blank_pages() {
    use sunipdf_core::{PageMargins, ResizeMode, Rotation};
    let engine = engine();
    let dir = temp_dir("boxes");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let count = doc.page_objects(0).unwrap().len();
    let all = |v: f32| PageMargins {
        left: v,
        top: v,
        right: v,
        bottom: v,
    };
    assert_eq!(doc.crop_page(0, all(36.0)).unwrap(), (540.0, 720.0));
    let size = doc.page_size(0).unwrap();
    assert_eq!((size.width_pt, size.height_pt), (540.0, 720.0));
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!((img.width, img.height), (540, 720));
    // Le titre de la page est bleu lui aussi : on ne regarde que sous lui.
    let blue = pixel_box_below(&img, 640.0, is_blue).unwrap();
    assert!(near(blue, rect(36.0, 464.0, 236.0, 614.0), 1.5), "{blue:?}");
    assert_eq!(
        doc.page_objects(0).unwrap().len(),
        count,
        "recadrer ne supprime rien"
    );
    doc.reset_crop(0).unwrap();
    assert_eq!(doc.page_size(0).unwrap().width_pt, 612.0);
    assert!(doc
        .crop_page(
            0,
            PageMargins {
                left: -1.0,
                ..all(0.0)
            }
        )
        .is_err());
    assert!(doc.crop_page(0, all(300.0)).is_err(), "zone trop petite");
    doc.crop_page(0, all(36.0)).unwrap();
    let out = dir.join("cropped.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert_eq!(again.page_size(0).unwrap().width_pt, 540.0);

    // Page pivotée : la marge gauche est celle de l'écran.
    let mut boxes = engine.open(&fixture("boxes.pdf"), None).unwrap();
    assert_eq!(
        boxes
            .crop_page(
                0,
                PageMargins {
                    left: 10.0,
                    ..all(0.0)
                }
            )
            .unwrap(),
        (782.0, 612.0)
    );
    let red = pixel_box(&boxes.render_page(0, 1.0).unwrap(), is_red).unwrap();
    assert!(near(red, rect(40.0, 462.0, 140.0, 562.0), 1.5), "{red:?}");

    // Redimensionner en conservant le contenu : ancrage en haut à gauche.
    let mut keep = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    keep.resize_page(0, 400.0, 500.0, ResizeMode::KeepContent)
        .unwrap();
    let s = keep.page_size(0).unwrap();
    assert_eq!((s.width_pt, s.height_pt), (400.0, 500.0));
    let blue = pixel_box_below(&keep.render_page(0, 1.0).unwrap(), 380.0, is_blue).unwrap();
    assert!(near(blue, rect(72.0, 208.0, 272.0, 358.0), 1.5), "{blue:?}");
    // Mise à l'échelle : contenu réduit de moitié.
    let mut scale = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    scale
        .resize_page(0, 306.0, 396.0, ResizeMode::ScaleContent)
        .unwrap();
    let blue = pixel_box_below(&scale.render_page(0, 1.0).unwrap(), 340.0, is_blue).unwrap();
    assert!(near(blue, rect(36.0, 250.0, 136.0, 325.0), 2.0), "{blue:?}");
    assert!(scale
        .resize_page(0, 0.0, 100.0, ResizeMode::KeepContent)
        .is_err());
    assert!(scale
        .resize_page(0, 20_000.0, 100.0, ResizeMode::KeepContent)
        .is_err());

    // Pages vierges.
    scale.insert_blank_pages(1, 2, 300.0, 400.0).unwrap();
    assert_eq!(scale.page_count(), 4);
    for p in [1, 2] {
        let s = scale.page_size(p).unwrap();
        assert_eq!((s.width_pt, s.height_pt), (300.0, 400.0));
        assert_eq!(scale.page_rotation(p).unwrap(), Rotation::R0);
        let img = scale.render_page(p, 1.0).unwrap();
        assert!(
            img.rgba.chunks(4).all(|q| q[..3] == [255, 255, 255]),
            "page vierge"
        );
    }
    assert!(scale.insert_blank_pages(9, 1, 300.0, 400.0).is_err());
    assert!(scale.insert_blank_pages(0, 1, 0.0, 400.0).is_err());
    assert!(scale.insert_blank_pages(0, 1, 300.0, 20_000.0).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn page_objects_listing_is_linear() {
    use sunipdf_core::TextStyle;
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    for k in 0..300 {
        let (x, y) = (20.0 + (k % 10) as f32 * 55.0, 40.0 + (k / 10) as f32 * 14.0);
        doc.add_text(
            0,
            x,
            y,
            "item",
            &TextStyle {
                size_pt: 8.0,
                ..Default::default()
            },
        )
        .unwrap();
    }
    let start = std::time::Instant::now();
    let objs = doc.page_objects(0).unwrap();
    assert!(objs.len() > 300);
    assert!(
        objs.iter()
            .filter(|o| o.text.as_deref() == Some("item"))
            .count()
            == 300
    );
    for k in 0..20 {
        doc.object_at(0, 25.0 + k as f32, 42.0).unwrap();
    }
    assert!(start.elapsed().as_secs_f32() < 5.0, "{:?}", start.elapsed());
}
