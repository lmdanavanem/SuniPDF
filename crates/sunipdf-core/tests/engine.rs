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
fn page_labels_with_control_characters_or_excessive_prefixes_use_physical_numbers() {
    use lopdf::{dictionary, Object};
    let engine = engine();
    let mut pdf = lopdf::Document::load(fixture("two-pages.pdf")).unwrap();
    let catalog = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
    for length in [4097, 4096] {
        pdf.get_dictionary_mut(catalog).unwrap().set("PageLabels", dictionary! {
            "Nums" => vec![0.into(), dictionary! {"P" => Object::string_literal("x".repeat(length))}.into(),
                           1.into(), dictionary! {"P" => Object::string_literal("bad\nlabel")}.into()]
        });
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let doc = engine
            .open_bytes(bytes, None, &fixture("two-pages.pdf"))
            .unwrap();
        assert_eq!(
            doc.info().page_label(0).map(str::len),
            (length == 4096).then_some(4096)
        );
        assert_eq!(doc.info().page_label(1), None);
        assert_eq!(doc.info().resolve_page_input("#2"), Some(1));
        assert_eq!(doc.page_count(), 2);
    }
}

#[test]
fn page_labels_read_nested_ranges_without_changing_source_bytes() {
    let engine = engine();
    let path = fixture("page-labels.pdf");
    let original = std::fs::read(&path).unwrap();
    let doc = engine.open(&path, None).unwrap();
    let labels: Vec<_> = doc
        .info()
        .page_labels
        .iter()
        .map(Option::as_deref)
        .collect();
    assert_eq!(
        labels,
        vec![
            Some("i"),
            Some("ii"),
            Some("1"),
            Some("2"),
            Some("AA"),
            Some("Annexe-É7"),
            Some("Repeat"),
            Some("Repeat"),
            None
        ]
    );
    assert!(doc.page_text(2).unwrap().contains("physical page 3"));
    assert_eq!(doc.info().resolve_page_input("1"), Some(2));
    assert_eq!(doc.info().resolve_page_input("#1"), Some(0));
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn page_labels_survive_crop_rotation_encryption_save_and_restore() {
    use sunipdf_core::{security, PageMargins, Protection, Rotation};
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixture("page-labels.pdf")).unwrap();
    for encrypted in [false, true] {
        let bytes = if encrypted {
            security::protect(
                &original,
                None,
                &Protection {
                    user_password: "user".into(),
                    owner_password: "owner".into(),
                    aes256: true,
                    ..Default::default()
                },
            )
            .unwrap()
        } else {
            original.clone()
        };
        let password = encrypted.then_some("owner");
        let mut doc = engine
            .open_bytes(bytes, password, &fixture("page-labels.pdf"))
            .unwrap();
        let labels = doc.info().page_labels.clone();
        let before = doc.snapshot().unwrap();
        for rotation in [Rotation::R90, Rotation::R180, Rotation::R270, Rotation::R0] {
            doc.set_page_rotation(0, rotation).unwrap();
            assert_eq!(doc.page_rotation(0).unwrap(), rotation);
            assert_eq!(doc.info().page_labels, labels);
        }
        doc.crop_page(
            1,
            PageMargins {
                left: 10.0,
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
            },
        )
        .unwrap();
        assert_eq!(doc.info().page_labels, labels);
        assert_eq!(doc.page_rotation(1).unwrap(), Rotation::R0);
        let out = temp.path().join(format!("labels-{encrypted}.pdf"));
        doc.save_as(&out).unwrap();
        let reopened = engine.open(&out, password).unwrap();
        assert_eq!(reopened.info().page_labels, labels);
        assert_eq!(reopened.page_rotation(0).unwrap(), Rotation::R0);
        assert_eq!(reopened.properties().encrypted, encrypted);
        doc.restore(before).unwrap();
        assert_eq!(doc.page_rotation(0).unwrap(), Rotation::R0);
        assert!(matches!(
            doc.page_rotation(9),
            Err(EngineError::PageOutOfRange(9))
        ));
        assert_eq!(doc.info().page_labels, labels);
    }
    assert_eq!(std::fs::read(fixture("page-labels.pdf")).unwrap(), original);
}

#[test]
fn grouped_page_deletion_validates_before_mutation_and_refreshes_metadata_once() {
    let engine = engine();
    let mut doc = engine.open(&fixture("page-labels.pdf"), None).unwrap();
    let before = doc.info().page_labels.clone();
    assert!(doc.delete_pages(&[0, 99]).is_err());
    assert_eq!(doc.page_count(), 9);
    assert_eq!(doc.info().page_labels, before);
    doc.delete_pages(&[1, 3, 1]).unwrap();
    assert_eq!(doc.page_count(), 7);
    assert!(doc.page_text(1).unwrap().contains("physical page 3"));
    let reopened = engine
        .open_bytes(doc.snapshot().unwrap(), None, &fixture("page-labels.pdf"))
        .unwrap();
    assert_eq!(doc.info().page_labels, reopened.info().page_labels);
    for page in 0..doc.page_count() {
        assert_eq!(
            doc.page_rotation(page).unwrap(),
            reopened.page_rotation(page).unwrap()
        );
    }
    assert!(doc.delete_pages(&(0..7).collect::<Vec<_>>()).is_err());
    assert_eq!(doc.page_count(), 7);
}

#[test]
fn page_labels_refresh_after_structural_changes_matches_saved_document() {
    use sunipdf_core::Rotation;
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let mut doc = engine.open(&fixture("page-labels.pdf"), None).unwrap();
    doc.set_page_rotation(0, Rotation::R90).unwrap();
    doc.set_page_rotation(3, Rotation::R180).unwrap();
    let before = doc.snapshot().unwrap();
    doc.move_page(0, 2).unwrap();
    assert_eq!(doc.page_rotation(2).unwrap(), Rotation::R90);
    assert!(doc.page_text(2).unwrap().contains("physical page 1"));
    assert_eq!(
        doc.info().page_label(0),
        Some("i"),
        "existing number-tree ranges remain attached to physical indices"
    );
    doc.delete_page(1).unwrap();
    doc.insert_pages_from(&fixture("two-pages.pdf"), None, &[0], 0)
        .unwrap();
    assert_eq!(doc.page_rotation(2).unwrap(), Rotation::R90);
    assert_eq!(doc.page_rotation(3).unwrap(), Rotation::R180);
    assert_eq!(doc.info().page_labels.len(), doc.page_count());
    let out = temp.path().join("reordered.pdf");
    doc.save_as(&out).unwrap();
    assert_eq!(
        engine.open(&out, None).unwrap().info().page_labels,
        doc.info().page_labels
    );
    let reopened = engine.open(&out, None).unwrap();
    for page in 0..doc.page_count() {
        assert_eq!(
            reopened.page_rotation(page).unwrap(),
            doc.page_rotation(page).unwrap()
        );
    }
    eprintln!(
        "Page-label ranges after move/delete/insert: {:?}",
        doc.info().page_labels
    );
    let extracted = temp.path().join("extracted.pdf");
    doc.extract_to(&[0, 2], &extracted).unwrap();
    let extracted = engine.open(&extracted, None).unwrap();
    assert_eq!(extracted.info().page_labels.len(), 2);
    assert_eq!(extracted.page_rotation(1).unwrap(), Rotation::R90);
    eprintln!(
        "Page labels after extract: {:?}",
        extracted.info().page_labels
    );
    doc.restore(before).unwrap();
    assert_eq!(doc.page_rotation(0).unwrap(), Rotation::R90);
    assert_eq!(doc.page_rotation(3).unwrap(), Rotation::R180);
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
fn move_page_preserves_bookmark_and_link_destinations() {
    let engine = engine();
    let dir = temp_dir("move-destinations");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    for (page, target) in [(0, 1), (1, 0)] {
        doc.add_link_annotation(
            page,
            rect(20.0, 20.0, 100.0, 40.0),
            &LinkTarget::Page(target),
        )
        .unwrap();
    }
    for (from, to, first, second) in [(0, 1, 1, 0), (1, 0, 0, 1)] {
        doc.move_page(from, to).unwrap();
        let check = |doc: &sunipdf_core::Document<'_>| {
            let bookmarks = doc.bookmarks();
            assert_eq!(bookmarks.len(), 3);
            assert_eq!(bookmarks[0].page, Some(first), "Premiere page");
            assert_eq!(bookmarks[1].page, Some(first), "Rectangle bleu");
            assert_eq!(bookmarks[2].page, Some(second), "Deuxieme page");
            assert_eq!(
                doc.page_links(first).unwrap()[0].target,
                LinkTarget::Page(second)
            );
            assert_eq!(
                doc.page_links(second).unwrap()[0].target,
                LinkTarget::Page(first)
            );
            assert!(doc.page_text(first).unwrap().contains("page 1"));
            assert!(doc.page_text(second).unwrap().contains("page 2"));
        };
        check(&doc);
        let out = dir.join("moved.pdf");
        doc.save_as(&out).unwrap();
        check(&engine.open(&out, None).unwrap());
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn move_page_preserves_inherited_geometry_and_resources() {
    use lopdf::{dictionary, Object};
    let engine = engine();
    let mut pdf = lopdf::Document::load(fixture("boxes.pdf")).unwrap();
    let root = pdf
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let pages: Vec<_> = pdf.get_pages().into_values().collect();
    let mut parents = Vec::new();
    for id in pages {
        let mut parent = dictionary! {
            "Type" => "Pages", "Parent" => root,
            "Kids" => vec![Object::Reference(id)], "Count" => 1,
        };
        let page = pdf.get_dictionary_mut(id).unwrap();
        for key in [b"MediaBox".as_slice(), b"CropBox", b"Resources", b"Rotate"] {
            if let Some(value) = page.remove(key) {
                parent.set(key, value);
            }
        }
        let parent_id = pdf.add_object(parent);
        pdf.get_dictionary_mut(id).unwrap().set("Parent", parent_id);
        parents.push(Object::Reference(parent_id));
    }
    pdf.get_dictionary_mut(root).unwrap().set("Kids", parents);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let mut doc = engine
        .open_bytes(bytes, None, &fixture("boxes.pdf"))
        .unwrap();
    let before: Vec<_> = (0..doc.page_count())
        .map(|p| {
            (
                doc.page_geometry(p).unwrap(),
                doc.render_page(p, 1.0).unwrap(),
            )
        })
        .collect();
    for (from, to, order) in [(0, 3, [1, 2, 3, 0]), (3, 1, [1, 0, 2, 3])] {
        doc.move_page(from, to).unwrap();
        for (page, original) in order.into_iter().enumerate() {
            assert_eq!(doc.page_geometry(page).unwrap(), before[original].0);
            assert_eq!(
                doc.render_page(page, 1.0).unwrap().rgba,
                before[original].1.rgba
            );
        }
    }
}

#[test]
fn move_page_keeps_form_widgets_editable_and_encrypted() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = temp_dir("move-form");
    for encrypted in [false, true] {
        let input = std::fs::read(fixture("acroform.pdf")).unwrap();
        let (bytes, password) = if encrypted {
            let bytes = security::protect(
                &input,
                None,
                &Protection {
                    user_password: "secret".into(),
                    owner_password: "owner".into(),
                    ..Default::default()
                },
            )
            .unwrap();
            (bytes, Some("secret"))
        } else {
            (input, None)
        };
        let path = dir.join(format!("form-{encrypted}.pdf"));
        let mut doc = engine.open_bytes(bytes, password, &path).unwrap();
        doc.insert_blank_pages(1, 1, 595.0, 842.0).unwrap();
        doc.fill_form(&[("nom".into(), text("Jean Dupont"))], FillMode::Strict)
            .unwrap();
        for (from, to, name) in [(0, 1, "DEPLACEMENT"), (1, 0, "RETOUR")] {
            let before = doc.render_page(from, 1.0).unwrap();
            let mut expected = doc.form_info().unwrap();
            for field in &mut expected.fields {
                for widget in &mut field.widgets {
                    widget.page = to;
                }
            }
            doc.move_page(from, to).unwrap();
            assert_eq!(doc.form_info().unwrap(), expected, "encrypted={encrypted}");
            assert_eq!(doc.render_page(to, 1.0).unwrap().rgba, before.rgba);
            doc.fill_form(&[("nom".into(), text(name))], FillMode::Strict)
                .unwrap();
            expected
                .fields
                .iter_mut()
                .find(|f| f.name == "nom")
                .unwrap()
                .value = text(name);
            assert_eq!(doc.form_info().unwrap(), expected);
            let filled = doc.render_page(to, 1.0).unwrap();
            assert_ne!(filled.rgba, before.rgba, "the visible widget must change");
            assert!(dark_in(&filled, NOM) > 300);
            assert_eq!(doc.properties().encrypted, encrypted);
            doc.save_as(&path).unwrap();
            if encrypted {
                assert!(matches!(
                    engine.open(&path, None),
                    Err(EngineError::PasswordRequired)
                ));
            }
            let reopened = engine.open(&path, password).unwrap();
            assert_eq!(reopened.form_info().unwrap(), expected);
            assert_eq!(reopened.render_page(to, 1.0).unwrap().rgba, filled.rgba);
            assert_eq!(reopened.properties().encrypted, encrypted);
        }
    }
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
    doc.add_markup(0, MarkupKind::StrikeOut, &sel.rects, (220, 38, 38))
        .unwrap();
    let list = doc.annotations(0).unwrap();
    assert_eq!(list.len(), 4);
    // Identifiants stables (pas le nom de variante de pdfium-render, « Strikeout »).
    assert_eq!(list[0].kind, "highlight");
    assert_eq!(list[2].kind, "text");
    assert_eq!(list[3].kind, "strikeout");
    assert_eq!(list[2].contents.as_deref(), Some("Relire ce paragraphe"));
    doc.delete_annotation(0, 3).unwrap();

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
        let parsed = lopdf::Document::load_mem(&bytes).unwrap();
        assert_eq!(
            parsed
                .get_encrypted()
                .unwrap()
                .get(b"Length")
                .unwrap()
                .as_i64()
                .unwrap(),
            if aes256 { 256 } else { 128 },
        );
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

fn assert_same_pdf_contents(before: &[u8], after: &[u8]) {
    let mut before = lopdf::Document::load_mem(before).unwrap();
    let mut after = lopdf::Document::load_mem(after).unwrap();
    // PDFium may regenerate the trailer ID when serializing an unchanged document.
    before.trailer.remove(b"ID");
    after.trailer.remove(b"ID");
    assert_eq!(after.objects, before.objects);
    assert_eq!(after.trailer, before.trailer);
}

#[test]
fn add_text_rejects_non_finite_coordinates_without_mutation() {
    use sunipdf_core::TextStyle;
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let before = doc.snapshot().unwrap();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for (x, y) in [(bad, 40.0), (40.0, bad)] {
            assert!(
                matches!(
                    doc.add_text(0, x, y, "Invalid coordinates", &TextStyle::default()),
                    Err(EngineError::Other(message)) if message == "invalid text coordinates"
                ),
                "non-finite coordinates must be rejected: ({x}, {y})"
            );
            assert_same_pdf_contents(&before, &doc.snapshot().unwrap());
        }
    }
}

#[test]
fn stamps_reject_line_breaks_without_mutation() {
    use sunipdf_core::Stamp;
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let before = doc.snapshot().unwrap();
    for text in ["First\nSecond", "First\r\nSecond", "First\rSecond", "\n"] {
        let stamp = Stamp {
            text: text.into(),
            ..Default::default()
        };
        for pages in [None, Some([].as_slice())] {
            assert!(
                matches!(
                    doc.stamp_pages(pages, &stamp),
                    Err(EngineError::Other(message)) if message == "stamp text must be a single line"
                ),
                "line breaks must be rejected even without selected pages: {text:?}"
            );
            assert_same_pdf_contents(&before, &doc.snapshot().unwrap());
        }
    }
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
            let parsed = lopdf::Document::load_mem(&bytes).unwrap();
            assert_eq!(
                parsed
                    .get_encrypted()
                    .unwrap()
                    .get(b"Length")
                    .unwrap()
                    .as_i64()
                    .unwrap(),
                if aes256 { 256 } else { 128 },
                "{name} aes256={aes256}: explicit encryption key length",
            );
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
    // Tampon sur des pages pivotées : centré et à 45° à l'écran, comme sur une page droite.
    let stamp = sunipdf_core::Stamp {
        text: "TAMPON".into(),
        ..Default::default()
    };
    doc.stamp_pages(Some(&[0, 2]), &stamp).unwrap();
    for p in [0, 2] {
        let s = doc.page_objects(p).unwrap().last().unwrap().clone();
        let size = doc.page_size(p).unwrap();
        let (cx, cy) = center(s.bounds);
        assert!(
            (cx - size.width_pt / 2.0).abs() < 3.0 && (cy - size.height_pt / 2.0).abs() < 3.0,
            "page {p} : {:?}",
            s.bounds
        );
        assert!(
            (s.angle_deg + 45.0).abs() < 1.0,
            "page {p} : {}",
            s.angle_deg
        );
        assert!(
            (s.font_size - 48.0).abs() < 0.5,
            "page {p} : {}",
            s.font_size
        );
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
    // Taille d'un texte pivoté : la rotation ne la change pas, et la modifier donne la valeur voulue.
    assert!((info.font_size - 20.0).abs() < 0.1, "{}", info.font_size);
    doc.rotate_object(0, t, 70.0).unwrap();
    assert!(
        (doc.object_info(0, t).unwrap().font_size - 20.0).abs() < 0.1,
        "à 90°"
    );
    doc.set_text_size(0, t, 30.0).unwrap();
    assert!((doc.object_info(0, t).unwrap().font_size - 30.0).abs() < 0.1);
    doc.rotate_object(0, t, -70.0).unwrap();
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
        // L'erreur liste les caractères absents (dédoublonnés, même non consécutifs) ;
        // tabulation = quatre espaces.
        match doc.add_text(0, 60.0, 300.0, "Привет мир", &TextStyle::default()) {
            Err(EngineError::MissingGlyphs(chars)) => {
                assert_eq!(chars, "Приветм", "{chars:?}")
            }
            other => panic!("attendu MissingGlyphs, obtenu {other:?}"),
        }
        assert_eq!(doc.page_objects(0).unwrap().len(), count, "document intact");
        assert_eq!(
            doc.add_text(0, 60.0, 280.0, "a\tb", &TextStyle::default())
                .unwrap(),
            1
        );
        let last = doc.page_objects(0).unwrap().pop().unwrap();
        let t = last.text.clone().unwrap_or_default();
        assert!(
            t.starts_with('a') && t.ends_with('b') && t.contains(' ') && !t.contains('\t'),
            "{t:?}"
        );
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
        Err(EngineError::MissingGlyphs(_))
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

// ----- Vue continue (SUNI-012b) : rendu de régions et tuiles -----

use sunipdf_core::tiles::{self, PixelRect, TileLimits};

/// Octets RGBA de la zone `r` d'un rendu.
fn crop(img: &RenderedPage, r: PixelRect) -> Vec<u8> {
    let mut out = Vec::with_capacity((r.width * r.height * 4) as usize);
    for y in r.y..r.bottom() {
        let start = ((y * img.width + r.x) * 4) as usize;
        out.extend_from_slice(&img.rgba[start..start + (r.width * 4) as usize]);
    }
    out
}

/// Pixel RGBA (x, y depuis le haut) d'un rendu.
fn pixel_at(img: &RenderedPage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    [
        img.rgba[i],
        img.rgba[i + 1],
        img.rgba[i + 2],
        img.rgba[i + 3],
    ]
}

#[test]
fn render_region_equals_crop_of_full_render() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let full = doc.render_page(0, 2.0).unwrap();
    assert_eq!((full.width, full.height), (1224, 1584));
    for r in [
        PixelRect::new(0, 0, 512, 512),
        PixelRect::new(512, 1024, 712, 560),
        PixelRect::new(1000, 1300, 224, 284),
    ] {
        let part = doc.render_region(0, 2.0, r).unwrap();
        assert_eq!((part.width, part.height), (r.width, r.height));
        assert!(
            part.rgba == crop(&full, r),
            "région {r:?} différente du rendu complet"
        );
    }
}

#[test]
fn render_region_on_rotated_page_matches_crop() {
    use sunipdf_core::Rotation;
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.set_page_rotation(0, Rotation::R90).unwrap();
    let full = doc.render_page(0, 1.0).unwrap();
    assert_eq!((full.width, full.height), (792, 612));
    let r = PixelRect::new(100, 50, 400, 300);
    let part = doc.render_region(0, 1.0, r).unwrap();
    assert!(part.rgba == crop(&full, r));
}

#[test]
fn render_region_deep_zoom_beyond_full_page_limit() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    // Page entière à 40 px/pt : 24 480 x 31 680 px, refusée d'un seul tenant.
    assert!(matches!(
        doc.render_page(0, 40.0),
        Err(EngineError::RenderTooLarge)
    ));
    assert_eq!(
        tiles::page_pixels(doc.page_size(0).unwrap(), 40.0),
        Some((24_480, 31_680))
    );
    // Intérieur du rectangle bleu (x 2880..10880, y 5680..11680 depuis le haut).
    let inside = doc
        .render_region(0, 40.0, PixelRect::new(4000, 7000, 256, 256))
        .unwrap();
    assert_eq!(inside.rgba.len(), 256 * 256 * 4);
    assert!(inside.rgba.chunks(4).all(|p| p == [0, 0, 255, 255]));
    // À cheval sur le bord droit du rectangle (x = 10880 → colonne 80 de la région).
    let edge = doc
        .render_region(0, 40.0, PixelRect::new(10_800, 7000, 256, 256))
        .unwrap();
    assert_eq!(pixel_at(&edge, 40, 128), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&edge, 120, 128), [255, 255, 255, 255]);
    // Autour du texte bleu « SuniPDF » (24 pt à y = 700) : des lettres, et un fond opaque.
    let text = doc
        .render_region(0, 40.0, PixelRect::new(2880, 3000, 1024, 1024))
        .unwrap();
    let colored = text.rgba.chunks(4).filter(|p| p[0] < 128).count();
    let share = colored as f64 / (1024.0 * 1024.0);
    // Mesuré : 0,20 ; seuil fixé nettement en dessous.
    assert!(share > 0.05, "part de pixels du texte : {share}");
    assert!(text.rgba.chunks(4).all(|p| p[3] == 255), "fond transparent");
}

#[test]
fn render_region_rejects_invalid_requests() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let other = |r: Result<RenderedPage, EngineError>| matches!(r, Err(EngineError::Other(_)));
    let too_large =
        |r: Result<RenderedPage, EngineError>| matches!(r, Err(EngineError::RenderTooLarge));
    assert!(other(doc.render_region(
        0,
        1.0,
        PixelRect::new(0, 0, 0, 10)
    )));
    assert!(other(doc.render_region(
        0,
        1.0,
        PixelRect::new(600, 0, 13, 10)
    )));
    assert!(other(doc.render_region(
        0,
        1.0,
        PixelRect::new(0, 790, 10, 3)
    )));
    assert!(too_large(doc.render_region(
        0,
        40.0,
        PixelRect::new(0, 0, 16_385, 10)
    )));
    for scale in [f32::NAN, 0.0, -1.0, 300.0] {
        assert!(too_large(doc.render_region(
            0,
            scale,
            PixelRect::new(0, 0, 1, 1)
        )));
    }
    assert!(matches!(
        doc.render_region(9, 1.0, PixelRect::new(0, 0, 1, 1)),
        Err(EngineError::PageOutOfRange(9))
    ));
}

#[test]
fn render_region_deep_zoom_on_image_page() {
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-tiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("gradient.png");
    let rgb: Vec<u8> = (0..64u32 * 64)
        .flat_map(|i| [(i % 64 * 4) as u8, (i / 64 * 4) as u8, 128])
        .collect();
    image::save_buffer(&png, &rgb, 64, 64, image::ColorType::Rgb8).unwrap();
    let out = dir.join("image.pdf");
    engine.images_to_pdf(&[png], &out, 72.0).unwrap();
    let doc = engine.open(&out, None).unwrap();
    let (w, h) = tiles::page_pixels(doc.page_size(0).unwrap(), 50.0).unwrap();
    assert!(w > 3000 && h > 3000, "{w} x {h}");
    let tile = doc
        .render_region(0, 50.0, PixelRect::new(w / 2 - 128, h / 2 - 128, 256, 256))
        .unwrap();
    assert!(tile.rgba.chunks(4).all(|p| p[3] == 255));
    let first = &tile.rgba[..4];
    assert!(tile.rgba.chunks(4).any(|p| p != first), "tuile uniforme");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn many_pages_fixture_pages_have_expected_sizes_and_colors() {
    let engine = engine();
    let doc = engine.open(&fixture("many-pages.pdf"), None).unwrap();
    assert_eq!(doc.page_count(), 120);
    for i in [0usize, 9, 24, 49, 119] {
        let size = doc.page_size(i).unwrap();
        let expected = if i % 10 == 9 {
            (792.0, 612.0)
        } else if i % 25 == 24 {
            (420.0, 595.0)
        } else {
            (612.0, 792.0)
        };
        assert_eq!((size.width_pt, size.height_pt), expected, "page {i}");
        // Centre du carré coloré (x 96, y 76 depuis le bas) : région 8 x 8 à l'échelle 1.
        let top = size.height_pt as u32 - 76 - 4;
        let tile = doc
            .render_region(i, 1.0, PixelRect::new(92, top, 8, 8))
            .unwrap();
        let want = [(i * 37) % 256, (i * 91) % 256, (i * 53) % 256];
        for p in tile.rgba.chunks(4) {
            for c in 0..3 {
                assert!(
                    (p[c] as i32 - want[c] as i32).abs() <= 1,
                    "page {i} : {p:?} au lieu de {want:?}"
                );
            }
        }
    }
}

#[test]
fn tiles_reassemble_into_full_render() {
    let engine = engine();
    let doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let full = doc.render_page(0, 2.0).unwrap();
    let limits = TileLimits {
        tile_side: 256,
        whole_max_side: 512,
        whole_max_pixels: 262_144,
    };
    let page_px = (full.width, full.height);
    let plan = tiles::plan_tiles(page_px, PixelRect::new(0, 0, page_px.0, page_px.1), &limits);
    assert_eq!(plan.len(), 5 * 7);
    let mut canvas = vec![0u8; full.rgba.len()];
    for r in plan {
        let tile = doc.render_region(0, 2.0, r).unwrap();
        for row in 0..r.height {
            let src = (row * r.width * 4) as usize;
            let dst = (((r.y + row) * full.width + r.x) * 4) as usize;
            canvas[dst..dst + (r.width * 4) as usize]
                .copy_from_slice(&tile.rgba[src..src + (r.width * 4) as usize]);
        }
    }
    // Égalité exacte observée pour les régions isolées (test ci-dessus) mais pas pour le recollage
    // complet : PDFium place différemment quelques pixels d'anticrénelage des glyphes qui
    // traversent un bord de tuile (mesuré : 121 pixels sur 1 938 816, écart max 134, tous sur le
    // texte, aucun sur les aplats). Tolérance : 99,9 % des pixels à <= 2 près (KNOWN_ISSUES).
    let off = canvas
        .chunks(4)
        .zip(full.rgba.chunks(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 2))
        .count();
    let total = (full.width * full.height) as usize;
    assert!(
        off * 1000 <= total,
        "{off} pixels sur {total} diffèrent du rendu complet"
    );
    // Les aplats (rectangle bleu, fond) sont identiques au pixel près.
    let blue = PixelRect::new(144, 284, 400, 300);
    assert!(crop_buf(&canvas, full.width, blue) == crop(&full, blue));
}

fn crop_buf(rgba: &[u8], width: u32, r: PixelRect) -> Vec<u8> {
    crop(
        &RenderedPage {
            page_index: 0,
            width,
            height: (rgba.len() / 4) as u32 / width,
            rgba: rgba.to_vec(),
        },
        r,
    )
}

/// Mesure seulement (aucune assertion de durée), en release : PDFium est cherché dans
/// `SUNIPDF_PDFIUM_DIR` (third_party/pdfium/win-x64/bin) :
/// `cargo test -p sunipdf-core --release -j 2 --test engine -- --ignored --nocapture render_scroll_100_pages_timing`
#[test]
#[ignore]
fn render_scroll_100_pages_timing() {
    let engine = engine();
    let doc = engine.open(&fixture("many-pages.pdf"), None).unwrap();
    let limits = TileLimits::default();
    // 1,5 px/pt : pages entières (100 % sur un écran à 150 %) ; 6 px/pt : tuiles de 1024 px.
    for (scale, pages) in [(1.5f32, 100usize), (6.0, 10)] {
        let start = std::time::Instant::now();
        let mut first = None;
        let mut count = 0u32;
        for i in 0..pages {
            let page = i % doc.page_count();
            let page_px = tiles::page_pixels(doc.page_size(page).unwrap(), scale).unwrap();
            let all = PixelRect::new(0, 0, page_px.0, page_px.1);
            for r in tiles::plan_tiles(page_px, all, &limits) {
                doc.render_region(page, scale, r).unwrap();
                count += 1;
            }
            first.get_or_insert(start.elapsed());
        }
        let total = start.elapsed();
        println!(
            "{pages} pages à {scale} px/pt : {count} rendus, total {total:?}, moyenne {:?}, première page {:?}",
            total / count,
            first.unwrap()
        );
    }
}

// ----- Correctifs des revues adverses (tranche review-fixes) -----

/// JPEG `w` × `h` : moitié gauche rouge, moitié droite bleue, orientation EXIF `exif` (6 = 90° horaire).
fn exif_jpeg(dir: &Path, name: &str, w: u32, h: u32, exif: u8) -> PathBuf {
    use image::ImageEncoder;
    let path = dir.join(name);
    let img = image::RgbImage::from_fn(w, h, |x, _| {
        image::Rgb(if x < w / 2 { [255, 0, 0] } else { [0, 0, 255] })
    });
    // Bloc TIFF minimal (petit-boutiste) : une entrée IFD0, tag 0x0112 Orientation, SHORT.
    let chunk = [
        b"II*\0".to_vec(),
        8u32.to_le_bytes().to_vec(),
        1u16.to_le_bytes().to_vec(),
        0x0112u16.to_le_bytes().to_vec(),
        3u16.to_le_bytes().to_vec(),
        1u32.to_le_bytes().to_vec(),
        (exif as u16).to_le_bytes().to_vec(),
        0u16.to_le_bytes().to_vec(),
        0u32.to_le_bytes().to_vec(),
    ]
    .concat();
    let mut file = std::fs::File::create(&path).unwrap();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
    enc.set_exif_metadata(chunk).unwrap();
    enc.encode_image(&img).unwrap();
    path
}

#[test]
fn review_aes128_refuses_non_latin1_passwords_and_keeps_accessibility_bit() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = temp_dir("review-sec");
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    let cyrillic = Protection {
        user_password: "пароль".into(),
        owner_password: "Пароль2026".into(),
        allow_copy: false,
        aes256: false,
        ..Default::default()
    };
    assert!(matches!(
        security::protect(&input, None, &cyrillic),
        Err(EngineError::PasswordNotLatin1)
    ));
    // AES-256 (UTF-8 + SASLprep) accepte le même mot de passe, et il protège vraiment.
    let bytes = security::protect(
        &input,
        None,
        &Protection {
            aes256: true,
            ..cyrillic.clone()
        },
    )
    .unwrap();
    let path = dir.join("cyr.pdf");
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        engine.open(&path, None),
        Err(EngineError::PasswordRequired)
    ));
    assert!(matches!(
        engine.open(&path, Some("")),
        Err(EngineError::PasswordRequired)
    ));
    let doc = engine.open(&path, Some("пароль")).unwrap();
    assert!(doc.is_encrypted() && !doc.properties().can_copy);
    assert!(engine.open(&path, Some("Пароль2026")).is_ok());
    // Latin-1 accepté en AES-128 ; la copie est refusée mais le bit 10 (accessibilité) reste à 1.
    let latin = security::protect(
        &input,
        None,
        &Protection {
            user_password: "Été".into(),
            owner_password: "Noël¿".into(),
            allow_copy: false,
            aes256: false,
            ..Default::default()
        },
    )
    .unwrap();
    let doc =
        lopdf::Document::load_mem_with_options(&latin, lopdf::LoadOptions::with_password("Été"))
            .unwrap();
    let bits = doc.encryption_state.as_ref().unwrap().permissions().bits();
    assert_eq!(bits & (1 << 9), 1 << 9, "bit 10 accessibilité : {bits:#x}");
    assert_eq!(bits & (1 << 4), 0, "bit 5 copie refusé : {bits:#x}");
    let _ = std::fs::remove_dir_all(dir);
}

/// AES-128 : tout mot de passe accepté par `Protection::validate` doit rouvrir le fichier dans
/// PDFium (qui reçoit le mot de passe en UTF-8 et ne réessaie qu'en Latin-1) ; les symboles de
/// PDFDocEncoding hors Latin-1 (€, œ, tirets, guillemets typographiques) sont refusés d'emblée.
#[test]
fn review_aes128_latin1_passwords_round_trip_through_pdfium() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = temp_dir("review-aes128");
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    // Bornes de la plage acceptée : 0x20, 0x7E, 0xA1, 0xAC, 0xAE, 0xFF, et des accents courants.
    for (i, user) in [" ~", "¡¬", "®ÿ", "Été¿", "Noël 2026"].iter().enumerate() {
        let owner = format!("{user}¡");
        let bytes = security::protect(
            &input,
            None,
            &Protection {
                user_password: user.to_string(),
                owner_password: owner.clone(),
                allow_print: false,
                aes256: false,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{user:?} : {e}"));
        let path = dir.join(format!("latin1-{i}.pdf"));
        std::fs::write(&path, &bytes).unwrap();
        assert!(
            matches!(engine.open(&path, None), Err(EngineError::PasswordRequired)),
            "{user:?} : ouverture libre"
        );
        let doc = engine
            .open(&path, Some(user))
            .unwrap_or_else(|e| panic!("{user:?} : {e}"));
        assert!(
            doc.is_encrypted() && !doc.properties().can_print,
            "{user:?}"
        );
        assert!(engine.open(&path, Some(&owner)).is_ok(), "{owner:?}");
    }
    for pw in ["prix5€", "œuvre", "a—b", "“x”"] {
        for (user, owner) in [(pw, "own3r"), ("us3r", pw)] {
            assert!(
                matches!(
                    security::protect(
                        &input,
                        None,
                        &Protection {
                            user_password: user.into(),
                            owner_password: owner.into(),
                            aes256: false,
                            ..Default::default()
                        },
                    ),
                    Err(EngineError::PasswordNotLatin1)
                ),
                "{user:?} / {owner:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn metadata_edit_preserves_pdfa_and_custom_xmp_in_compressed_and_encrypted_pdfs() {
    use sunipdf_core::{security, Metadata, Protection};
    let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/"
 xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:custom="urn:preserved:custom"
 pdfaid:part="2" custom:record="retained">
 <pdfaid:conformance>B</pdfaid:conformance>
 <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Old title</rdf:li></rdf:Alt></dc:title>
 <dc:creator><rdf:Seq><rdf:li>Old author</rdf:li></rdf:Seq></dc:creator>
 <custom:data><rdf:Seq><rdf:li>keep this value</rdf:li></rdf:Seq></custom:data>
</rdf:Description></rdf:RDF></x:xmpmeta>"#;
    let mut doc = lopdf::Document::load(fixture("two-pages.pdf")).unwrap();
    let mut stream = lopdf::Stream::new(
        lopdf::dictionary! { "Type" => "Metadata", "Subtype" => "XML", "CustomMarker" => 42 },
        xmp.to_vec(),
    );
    stream.compress().unwrap();
    assert!(!stream.filters().unwrap().is_empty());
    let id = doc.add_object(stream);
    doc.catalog_mut()
        .unwrap()
        .set("Metadata", lopdf::Object::Reference(id));
    let mut original = Vec::new();
    doc.save_to(&mut original).unwrap();
    for encrypted in [false, true] {
        let input = if encrypted {
            security::protect(
                &original,
                None,
                &Protection {
                    user_password: "user".into(),
                    owner_password: "owner".into(),
                    ..Default::default()
                },
            )
            .unwrap()
        } else {
            original.clone()
        };
        let edited = security::set_metadata(
            &input,
            encrypted.then_some("user"),
            &Metadata {
                title: Some("New title".into()),
                author: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            lopdf::Document::load_mem(&edited).unwrap().is_encrypted(),
            encrypted
        );
        let parsed = if encrypted {
            // Decrypt during parsing: a password-less load can skip encrypted objects permanently.
            lopdf::Document::load_mem_with_options(
                &edited,
                lopdf::LoadOptions::with_password("user"),
            )
            .unwrap()
        } else {
            lopdf::Document::load_mem(&edited).unwrap()
        };
        let id = parsed
            .catalog()
            .unwrap()
            .get(b"Metadata")
            .unwrap()
            .as_reference()
            .unwrap();
        let stream = parsed.get_object(id).unwrap().as_stream().unwrap();
        assert_eq!(
            stream.dict.get(b"CustomMarker").unwrap().as_i64().unwrap(),
            42
        );
        let text = String::from_utf8(stream.get_plain_content().unwrap()).unwrap();
        for kept in [
            "pdfaid:part=\"2\"",
            "<pdfaid:conformance>B</pdfaid:conformance>",
            "custom:record=\"retained\"",
            "keep this value",
            "New title",
        ] {
            assert!(text.contains(kept), "missing {kept}: {text}");
        }
        assert!(!text.contains("Old title") && !text.contains("Old author"));
    }

    let mut malformed = lopdf::Document::load_mem(&original).unwrap();
    let id = malformed
        .catalog()
        .unwrap()
        .get(b"Metadata")
        .unwrap()
        .as_reference()
        .unwrap();
    malformed
        .get_object_mut(id)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(b"<x:xmpmeta><rdf:RDF/></x:xmpmeta>".to_vec());
    let mut input = Vec::new();
    malformed.save_to(&mut input).unwrap();
    let path = temp_dir("malformed-xmp").join("original.pdf");
    std::fs::write(&path, &input).unwrap();
    let error = security::set_metadata(&input, None, &Metadata::default()).unwrap_err();
    assert!(error
        .to_string()
        .contains("XMP metadata could not be updated"));
    assert_eq!(std::fs::read(&path).unwrap(), input);
}

#[test]
fn review_metadata_edits_reach_xmp_and_strip_cleans_pages_and_images() {
    use sunipdf_core::{security, Metadata};
    let engine = engine();
    let dir = temp_dir("review-xmp");
    // Fixture avec XMP de catalogue, XMP de page et /PieceInfo (exports Word / InDesign).
    let mut doc = lopdf::Document::load(fixture("two-pages.pdf")).unwrap();
    let xmp = |who: &str| {
        lopdf::Stream::new(
            lopdf::dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
            format!("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:creator><rdf:Seq><rdf:li>{who}</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>")
                .into_bytes(),
        )
    };
    let cat_xmp = doc.add_object(xmp("Jean Dupont"));
    doc.catalog_mut()
        .unwrap()
        .set("Metadata", lopdf::Object::Reference(cat_xmp));
    let page_xmp = doc.add_object(xmp("jdoe PageAuthor"));
    let page_id = doc.get_pages()[&1];
    let page = doc.get_object_mut(page_id).unwrap().as_dict_mut().unwrap();
    page.set("Metadata", lopdf::Object::Reference(page_xmp));
    page.set(
        "PieceInfo",
        lopdf::dictionary! { "Illustrator" => lopdf::dictionary! { "Private" => "C:/Users/jdoe/secret.ai" } },
    );
    let mut input = Vec::new();
    doc.save_to(&mut input).unwrap();
    assert!(String::from_utf8_lossy(&input).contains("Jean Dupont"));

    let edited = security::set_metadata(
        &input,
        None,
        &Metadata {
            title: Some("Nouveau titre".into()),
            author: None,
            ..Default::default()
        },
    )
    .unwrap();
    let text = String::from_utf8_lossy(&edited);
    assert!(!text.contains("Jean Dupont"), "ancien auteur retiré du XMP");
    assert!(
        text.contains("<dc:title>") && text.contains("Nouveau titre"),
        "XMP régénéré"
    );
    let path = dir.join("meta.pdf");
    std::fs::write(&path, &edited).unwrap();
    let props = engine.open(&path, None).unwrap().properties();
    assert_eq!(props.metadata.title.as_deref(), Some("Nouveau titre"));
    assert_eq!(props.metadata.author, None);

    let stripped = security::strip_metadata(&input, None).unwrap();
    let text = String::from_utf8_lossy(&stripped);
    for leak in [
        "Jean Dupont",
        "jdoe",
        "PieceInfo",
        "/Metadata",
        "SuniPDF tests",
    ] {
        assert!(!text.contains(leak), "{leak} encore présent");
    }
    let path = dir.join("stripped.pdf");
    std::fs::write(&path, &stripped).unwrap();
    let doc = engine.open(&path, None).unwrap();
    assert_eq!(doc.page_count(), 2);
    assert!(doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn extract_and_split_preserve_protection_passwords_permissions_and_current_pages() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    let mut cases = vec![("plain", input.clone(), None, None)];
    let legacy = lopdf::Document::load_mem_with_options(
        &std::fs::read(fixture("encrypted-rc4.pdf")).unwrap(),
        lopdf::LoadOptions::with_password("secret"),
    )
    .unwrap();
    let mut rc4 = lopdf::Document::load_mem(&input).unwrap();
    rc4.trailer
        .set("ID", legacy.trailer.get(b"ID").unwrap().clone());
    rc4.encrypt(legacy.encryption_state.as_ref().unwrap())
        .unwrap();
    let mut rc4_bytes = Vec::new();
    rc4.save_to(&mut rc4_bytes).unwrap();
    cases.push(("rc4", rc4_bytes, Some("secret"), None));
    for (name, aes256, user) in [
        ("aes128", false, "secret"),
        ("aes256", true, "secret"),
        ("owner-only", true, ""),
    ] {
        cases.push((
            name,
            security::protect(
                &input,
                None,
                &Protection {
                    user_password: user.into(),
                    owner_password: "owner".into(),
                    allow_print: false,
                    allow_copy: false,
                    allow_modify: false,
                    allow_annotate: false,
                    aes256,
                },
            )
            .unwrap(),
            (!user.is_empty()).then_some(user),
            Some("owner"),
        ));
    }
    for (name, original, password, owner) in cases {
        let path = dir.path().join(format!("{name}.pdf"));
        std::fs::write(&path, &original).unwrap();
        let mut doc = engine.open(&path, password).unwrap();
        let encrypted = doc.is_encrypted();
        let props = doc.properties();
        doc.set_page_rotation(1, sunipdf_core::Rotation::R90)
            .unwrap();
        let out = dir.path().join(format!("{name}-extract.pdf"));
        assert_eq!(doc.extract_to(&[1, 0, 1], &out).unwrap(), 3);
        let parts = doc
            .split_to(1, dir.path(), &format!("{name}-part"), false)
            .unwrap();
        assert_eq!(parts.len(), 2);
        for output in std::iter::once(&out).chain(&parts) {
            let copy = engine.open(output, password).unwrap();
            let copy_props = copy.properties();
            assert_eq!(copy_props.encrypted, encrypted, "{name}");
            assert_eq!(copy_props.can_print, props.can_print, "{name}");
            assert_eq!(copy_props.can_copy, props.can_copy, "{name}");
            assert_eq!(copy_props.can_modify, props.can_modify, "{name}");
            assert_eq!(copy_props.can_annotate, props.can_annotate, "{name}");
            if encrypted && password.is_some() {
                assert!(matches!(
                    engine.open(output, None),
                    Err(EngineError::PasswordRequired)
                ));
                assert!(matches!(
                    engine.open(output, Some("wrong")),
                    Err(EngineError::PasswordRequired)
                ));
            }
            if let Some(owner) = owner {
                assert!(engine.open(output, Some(owner)).is_ok(), "{name}");
            }
            if encrypted {
                let read = |bytes: &[u8]| {
                    lopdf::Document::load_mem_with_options(
                        bytes,
                        lopdf::LoadOptions::with_password(password.unwrap_or("")),
                    )
                    .unwrap()
                };
                let source = read(&original);
                let copied = read(&std::fs::read(output).unwrap());
                let source_state = source.encryption_state.as_ref().unwrap();
                let copy_state = copied.encryption_state.as_ref().unwrap();
                assert_eq!(
                    copy_state.file_encryption_key(),
                    source_state.file_encryption_key(),
                    "{name}"
                );
                let source_security = source_state.encode().unwrap();
                let copy_security = copy_state.encode().unwrap();
                for key in ["V", "R", "P", "O", "U", "OE", "UE", "Perms", "CF"] {
                    assert_eq!(
                        copy_security.get(key.as_bytes()).ok(),
                        source_security.get(key.as_bytes()).ok(),
                        "{name} /{key}"
                    );
                }
                assert_eq!(
                    copied
                        .trailer
                        .get(b"ID")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .first(),
                    source
                        .trailer
                        .get(b"ID")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .first(),
                    "{name} permanent file ID used by legacy encryption"
                );
            }
        }
        let extracted = engine.open(&out, password).unwrap();
        assert!(extracted.page_text(0).unwrap().contains("SuniPDF page 2"));
        assert!(extracted.page_text(1).unwrap().contains("SuniPDF page 1"));
        assert!(extracted.page_text(2).unwrap().contains("SuniPDF page 2"));
        assert_eq!(
            extracted.page_rotation(0).unwrap(),
            sunipdf_core::Rotation::R90
        );
        assert_eq!(
            engine
                .open(&parts[1], password)
                .unwrap()
                .page_rotation(0)
                .unwrap(),
            sunipdf_core::Rotation::R90
        );
        assert_eq!(std::fs::read(&path).unwrap(), original, "{name} source");
    }
}

#[test]
fn extract_and_split_refuse_to_replace_the_source_and_keep_existing_parts() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("part-1.pdf");
    let original = std::fs::read(fixture("two-pages.pdf")).unwrap();
    std::fs::write(&source, &original).unwrap();
    let doc = engine.open(&source, None).unwrap();
    assert!(matches!(
        doc.extract_to(&[1], &dir.path().join(".").join("part-1.pdf")),
        Err(EngineError::AlreadyExists(_))
    ));
    assert!(matches!(
        doc.split_to(1, dir.path(), "part", true),
        Err(EngineError::AlreadyExists(_))
    ));
    assert!(!dir.path().join("part-2.pdf").exists());
    let existing = dir.path().join("other-2.pdf");
    std::fs::write(&existing, b"existing copy").unwrap();
    assert!(matches!(
        doc.split_to(1, dir.path(), "other", false),
        Err(EngineError::AlreadyExists(_))
    ));
    assert!(!dir.path().join("other-1.pdf").exists());
    assert_eq!(std::fs::read(existing).unwrap(), b"existing copy");
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn review_images_follow_exif_orientation_and_jpegs_are_embedded_as_is() {
    let engine = engine();
    let dir = temp_dir("review-exif");
    // 40 × 20 : gauche rouge, droite bleue ; EXIF 6 = 90° horaire → affichée 20 × 40, rouge en haut.
    let portrait = exif_jpeg(&dir, "portrait.jpg", 40, 20, 6);
    let plain = exif_jpeg(&dir, "plain.jpg", 40, 20, 1);
    let out = dir.join("photos.pdf");
    assert_eq!(
        engine
            .images_to_pdf(&[portrait.clone(), plain.clone()], &out, 72.0)
            .unwrap(),
        2
    );
    let bytes = std::fs::read(&out).unwrap();
    assert!(
        String::from_utf8_lossy(&bytes).contains("DCTDecode"),
        "JPEG incorporé tel quel (pas de pixels bruts recompressés)"
    );
    let doc = engine.open(&out, None).unwrap();
    let size = doc.page_size(0).unwrap();
    assert_eq!((size.width_pt, size.height_pt), (20.0, 40.0));
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(
        is_red(px(&img, 10.0, 35.0)),
        "haut rouge : {:?}",
        px(&img, 10.0, 35.0)
    );
    assert!(
        is_blue(px(&img, 10.0, 5.0)),
        "bas bleu : {:?}",
        px(&img, 10.0, 5.0)
    );
    let size = doc.page_size(1).unwrap();
    assert_eq!((size.width_pt, size.height_pt), (40.0, 20.0));
    let img = doc.render_page(1, 1.0).unwrap();
    assert!(is_red(px(&img, 5.0, 10.0)) && is_blue(px(&img, 35.0, 10.0)));

    // Insertion et remplacement d'image : même chargeur, même orientation.
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.add_image(0, &portrait, 100.0, 100.0, 50.0).unwrap();
    let index = doc.page_objects(0).unwrap().len() - 1;
    let b = doc.object_info(0, index).unwrap().bounds;
    assert!(
        (b.right - b.left - 50.0).abs() < 0.5 && (b.top - b.bottom - 100.0).abs() < 0.5,
        "portrait : {b:?}"
    );
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(is_red(px(&img, 125.0, 190.0)) && is_blue(px(&img, 125.0, 110.0)));
    doc.replace_image(0, index, &plain).unwrap();
    let b = doc.object_info(0, index).unwrap().bounds;
    assert!(
        (b.right - b.left - 50.0).abs() < 0.5 && (b.top - b.bottom - 25.0).abs() < 0.5,
        "paysage contenu dans le cadre : {b:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn review_bring_to_front_survives_reload_and_stamps_stay_inside_corners() {
    use sunipdf_core::{ShapeKind, ShapeStyle, Stamp, StampPosition};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let filled = |rgb| ShapeStyle {
        stroke_rgb: rgb,
        stroke_width: 0.5,
        fill_rgb: Some(rgb),
        ..Default::default()
    };
    let square = rect(100.0, 100.0, 200.0, 200.0);
    // Chaque ajout PDFium vit dans son propre flux de contenu : bleu (flux 1) puis rouge (flux 2).
    doc.add_shape(0, ShapeKind::Rectangle, square, &filled((0, 0, 255)))
        .unwrap();
    let blue = doc.page_objects(0).unwrap().len() - 1;
    doc.add_shape(0, ShapeKind::Rectangle, square, &filled((255, 0, 0)))
        .unwrap();
    let probe = |doc: &sunipdf_core::Document| px(&doc.render_page(0, 1.0).unwrap(), 150.0, 150.0);
    assert!(is_red(probe(&doc)));
    let front = doc.reorder_object(0, blue, true).unwrap();
    assert_eq!(front, doc.page_objects(0).unwrap().len() - 1);
    assert!(is_blue(probe(&doc)), "bleu devant");
    // Rechargement (ce que font chaque rendu, l'annulation et l'enregistrement) : toujours devant.
    let snapshot = doc.snapshot().unwrap();
    doc.restore(snapshot).unwrap();
    assert!(
        is_blue(probe(&doc)),
        "bleu toujours devant après rechargement"
    );

    // Tampon à 45° dans un coin : la boîte pivotée reste dans la page, à la marge près.
    let size = doc.page_size(0).unwrap();
    for position in [
        StampPosition::TopRight,
        StampPosition::BottomLeft,
        StampPosition::TopLeft,
        StampPosition::BottomRight,
    ] {
        let stamp = Stamp {
            text: "CONFIDENTIEL".into(),
            position,
            ..Default::default()
        };
        doc.stamp_pages(Some(&[0]), &stamp).unwrap();
        let last = doc.page_objects(0).unwrap().pop().unwrap();
        let b = last.bounds;
        assert!(
            b.left >= 27.0
                && b.bottom >= 27.0
                && b.right <= size.width_pt - 27.0
                && b.top <= size.height_pt - 27.0,
            "{position:?} : {b:?}"
        );
        assert!(
            b.top - b.bottom > 100.0 && (last.angle_deg + 45.0).abs() < 1.0,
            "pivoté : {:?} {}",
            b,
            last.angle_deg
        );
    }
}

#[test]
fn review_resizing_a_tilted_object_scales_uniformly_and_rotated_pages_keep_their_text() {
    use sunipdf_core::{Rotation, ShapeKind, ShapeStyle};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(300.0, 300.0, 400.0, 350.0),
        &ShapeStyle::default(),
    )
    .unwrap();
    let index = doc.page_objects(0).unwrap().len() - 1;
    doc.rotate_object(0, index, 45.0).unwrap();
    let before = doc.object_info(0, index).unwrap();
    let w = before.bounds.right - before.bounds.left;
    assert!((w - (before.bounds.top - before.bounds.bottom)).abs() < 1.0);
    // Cible 2 × 1 : des facteurs différents cisailleraient le rectangle à 45°.
    doc.resize_object(0, index, rect(100.0, 100.0, 300.0, 200.0))
        .unwrap();
    let after = doc.object_info(0, index).unwrap();
    let (aw, ah) = (
        after.bounds.right - after.bounds.left,
        after.bounds.top - after.bounds.bottom,
    );
    assert!((aw - ah).abs() < 1.0, "boîte restée carrée : {aw} × {ah}");
    assert!((aw - 100.0).abs() < 1.5, "plus petit facteur : {aw}");
    assert!((after.bounds.left - 100.0).abs() < 1.0 && (after.bounds.bottom - 100.0).abs() < 1.0);
    assert!((after.angle_deg - before.angle_deg).abs() < 0.5);

    // Texte complet d'une page pivotée : « SuniPDF page 1 » est en y = 700 > 612 (largeur affichée).
    doc.set_page_rotation(0, Rotation::R90).unwrap();
    assert!(doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    doc.set_page_rotation(0, Rotation::R180).unwrap();
    assert!(doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    let boxes = engine.open(&fixture("boxes.pdf"), None).unwrap();
    for (p, name) in ["A", "B", "C", "D"].iter().enumerate() {
        assert!(boxes
            .page_text(p)
            .unwrap()
            .contains(&format!("BOXES {name}")));
    }
}

// ----- Signature visuelle (tranche sign-image) ------------------------------------------------

/// Image d'essai 60 × 20 : tiers rouge opaque, tiers rouge à demi transparent, tiers noir
/// totalement transparent (noir pour qu'une perte de l'alpha se voie au rendu).
fn alpha_test_image() -> image::RgbaImage {
    image::RgbaImage::from_fn(60, 20, |x, _| match x / 20 {
        0 => image::Rgba([220, 20, 20, 255]),
        1 => image::Rgba([220, 20, 20, 128]),
        _ => image::Rgba([0, 0, 0, 0]),
    })
}

/// L'image couvre x 100..250, y 540..590, entièrement sur le rectangle bleu de la page 1.
fn assert_alpha_pixels(doc: &sunipdf_core::Document<'_>) {
    let r = doc.render_page(0, 1.0).unwrap();
    let y = 565.0;
    assert!(is_red(px(&r, 125.0, y)), "{:?}", px(&r, 125.0, y));
    let m = px(&r, 175.0, y);
    assert!(
        80 < m[0] && m[0] < 140 && 100 < m[2] && m[2] < 170,
        "mélange à 50 % attendu (masque 8 bits), obtenu {m:?}"
    );
    assert_eq!(
        px(&r, 225.0, y),
        [0, 0, 255],
        "zone transparente : bleu du dessous"
    );
    assert_eq!(px(&r, 90.0, y), [0, 0, 255], "hors image");
}

#[test]
fn image_alpha_is_preserved_as_soft_mask() {
    use sunipdf_core::ObjectKind;
    let engine = engine();
    let dir = temp_dir("sig-alpha");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let initial = doc.page_objects(0).unwrap().len();
    let dyn_img = image::DynamicImage::ImageRgba8(alpha_test_image());
    let snap = doc.snapshot().unwrap();
    doc.add_image_data(0, &dyn_img, 100.0, 540.0, 150.0)
        .unwrap();
    assert_alpha_pixels(&doc);
    let objects = doc.page_objects(0).unwrap();
    assert_eq!(objects.len(), initial + 1);
    let last = objects.last().unwrap();
    assert_eq!(last.kind, ObjectKind::Image);
    assert!(
        near(last.bounds, rect(100.0, 540.0, 250.0, 590.0), 1.0),
        "{:?}",
        last.bounds
    );

    // Annulation par instantané.
    doc.restore(snap).unwrap();
    assert_eq!(doc.page_objects(0).unwrap().len(), initial);
    assert!(is_blue(px(&doc.render_page(0, 1.0).unwrap(), 125.0, 565.0)));

    // Persistance : rechargement des octets, puis fichier enregistré et rouvert.
    doc.add_image_data(0, &dyn_img, 100.0, 540.0, 150.0)
        .unwrap();
    doc.restore(doc.snapshot().unwrap()).unwrap();
    assert_alpha_pixels(&doc);
    let out = dir.join("signed.pdf");
    doc.save_as(&out).unwrap();
    let reopened = engine.open(&out, None).unwrap();
    assert_alpha_pixels(&reopened);

    // Édition après pose : le masque suit l'objet.
    let i = doc.page_objects(0).unwrap().len() - 1;
    doc.move_object(0, i, 0.0, -30.0).unwrap();
    let r = doc.render_page(0, 1.0).unwrap();
    assert_eq!(
        px(&r, 225.0, 535.0),
        [0, 0, 255],
        "transparent après déplacement"
    );
    assert!(is_red(px(&r, 125.0, 535.0)));
    doc.resize_object(0, i, rect(100.0, 500.0, 400.0, 600.0))
        .unwrap();
    let r = doc.render_page(0, 1.0).unwrap();
    assert_eq!(
        px(&r, 350.0, 550.0),
        [255, 255, 255],
        "tiers transparent étiré sur le fond blanc"
    );

    // Même image par fichier PNG (chemin « importer un PNG transparent »).
    let png = dir.join("alpha.png");
    image::save_buffer(
        &png,
        alpha_test_image().as_raw(),
        60,
        20,
        image::ColorType::Rgba8,
    )
    .unwrap();
    let mut by_file = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    by_file.add_image(0, &png, 100.0, 540.0, 150.0).unwrap();
    assert_alpha_pixels(&by_file);

    // Erreurs : document inchangé.
    let count = doc.page_objects(0).unwrap().len();
    assert!(doc.add_image_data(0, &dyn_img, 100.0, 540.0, 0.0).is_err());
    assert!(doc
        .add_image_data(0, &dyn_img, 100.0, 540.0, f32::NAN)
        .is_err());
    assert!(doc
        .add_image_data(0, &dyn_img, f32::INFINITY, 540.0, 150.0)
        .is_err());
    assert!(matches!(
        doc.add_image_data(9, &dyn_img, 100.0, 540.0, 150.0),
        Err(EngineError::PageOutOfRange(9))
    ));
    let empty = image::DynamicImage::ImageRgba8(image::RgbaImage::new(0, 0));
    assert!(doc.add_image_data(0, &empty, 100.0, 540.0, 150.0).is_err());
    assert_eq!(doc.page_objects(0).unwrap().len(), count);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signature_strokes_to_page_pipeline() {
    use sunipdf_core::signature_image::{rasterize_strokes, trim};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let before = doc.snapshot().unwrap().len();
    let strokes = vec![
        vec![(100.0, 300.0), (600.0, 60.0), (1000.0, 280.0)],
        vec![(1050.0, 40.0)],
    ];
    let img = trim(
        &rasterize_strokes(&strokes, 1112, 340, 10.0, (0x1E, 0x3A, 0x8A)),
        6,
    )
    .unwrap();
    let (w, h) = img.dimensions();
    assert!(w < 1112 && h < 340, "rognée : {w} × {h}");
    let h_pt = 140.0 * h as f32 / w as f32;
    doc.add_image_data(
        0,
        &image::DynamicImage::ImageRgba8(img),
        300.0,
        100.0,
        140.0,
    )
    .unwrap();
    // Garde-fou de taille : PDFium compresse les flux d'image à l'enregistrement.
    let after = doc.snapshot().unwrap().len();
    assert!(after - before < 300_000, "+{} octets", after - before);
    let r = doc.render_page(0, 1.0).unwrap();
    let mut dark = 0;
    for y in 100..(100.0 + h_pt) as u32 {
        for x in 300..440u32 {
            let p = px(&r, x as f32 + 0.5, y as f32 + 0.5);
            // Trait d'environ 1,5 pt anti-crénelé : encre bleue nettement plus bleue que rouge.
            if p[0] < 170 && p[2] > p[0] + 40 {
                dark += 1;
            }
        }
    }
    assert!(dark > 150, "{dark} pixels d'encre bleue");
    // Coin haut-gauche de l'image, loin des traits : le fond de la page reste blanc.
    let k = 140.0 / w as f32;
    let p = px(&r, 300.0 + 16.0 * k, 100.0 + h_pt - 16.0 * k);
    assert_eq!(p, [255, 255, 255]);
}

#[test]
fn aes256_passwords_normalized_by_lopdf_still_open_as_typed() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = std::env::temp_dir().join(format!("sunipdf-saslprep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    // SASLprep ramène l'espace insécable à un espace et la ligature « ﬁ » à « fi » : lopdf chiffre
    // cette forme, PDFium reçoit la saisie brute. Les deux formes doivent ouvrir le fichier.
    for (typed, simple) in [("pass\u{00A0}word", "pass word"), ("\u{FB01}nal", "final")] {
        assert_eq!(security::normalized_password(typed), simple);
        let bytes = security::protect(
            &input,
            None,
            &Protection {
                user_password: typed.into(),
                owner_password: "own".into(),
                aes256: true,
                ..Default::default()
            },
        )
        .unwrap();
        let path = dir.join(format!("{simple}.pdf"));
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            engine.open(&path, None),
            Err(EngineError::PasswordRequired)
        ));
        assert!(matches!(
            engine.open(&path, Some("password")),
            Err(EngineError::PasswordRequired)
        ));
        let doc = engine.open(&path, Some(typed)).unwrap();
        assert!(
            doc.page_text(0).unwrap().contains("SuniPDF page 1"),
            "{typed:?}"
        );
        assert_eq!(
            doc.password(),
            Some(simple),
            "mot de passe mémorisé sous sa forme normalisée"
        );
        let snapshot = doc.snapshot().unwrap();
        let mut doc = doc;
        doc.restore(snapshot).unwrap();
        assert!(engine.open(&path, Some(simple)).is_ok());
        let again = engine
            .open_bytes(bytes.clone(), Some(typed), &path)
            .unwrap();
        assert_eq!(again.page_count(), 2);
    }
    // Un mot de passe simple n'est pas réessayé ni modifié.
    assert_eq!(security::normalized_password("Été 2026"), "Été 2026");
    let _ = std::fs::remove_dir_all(dir);
}

// ----- Formulaires AcroForm (tranche forms) ------------------------------------------------------

use sunipdf_core::{form_data, FieldKind, FieldValue, FillMode, FormKind};

/// Pixels sombres (R, V et B < 100) dans un rectangle d'affichage, rendu à l'échelle 1.
fn dark_in(img: &RenderedPage, r: PointRect) -> usize {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut n = 0;
    for y in 0..h {
        let y_pt = (h - y) as f32 - 0.5;
        if y_pt < r.bottom || y_pt > r.top {
            continue;
        }
        for x in 0..w {
            let x_pt = x as f32 + 0.5;
            if x_pt < r.left || x_pt > r.right {
                continue;
            }
            let i = (y * w + x) * 4;
            if img.rgba[i] < 100 && img.rgba[i + 1] < 100 && img.rgba[i + 2] < 100 {
                n += 1;
            }
        }
    }
    n
}

fn yellow_count(img: &RenderedPage) -> usize {
    img.rgba
        .chunks(4)
        .filter(|p| p[0] > 200 && p[1] > 180 && p[2] < 120)
        .count()
}

/// Contenu des flux d'apparence `/AP /N` des widgets du champ `field` (nom propre ou nom du
/// parent), relu avec lopdf dans les octets enregistrés : preuve que l'apparence est dans le fichier.
fn ap_bytes(bytes: &[u8], field: &str) -> Vec<u8> {
    let d = lopdf::Document::load_mem(bytes).unwrap();
    let name_of = |dict: &lopdf::Dictionary| -> Option<String> {
        dict.get(b"T")
            .ok()
            .and_then(|t| lopdf::decode_text_string(t).ok())
    };
    let mut out = Vec::new();
    for obj in d.objects.values() {
        let Ok(dict) = obj.as_dict() else { continue };
        if !dict
            .get(b"Subtype")
            .ok()
            .and_then(|s| s.as_name().ok())
            .is_some_and(|s| s == b"Widget")
        {
            continue;
        }
        let parent_name = dict
            .get(b"Parent")
            .ok()
            .and_then(|p| d.dereference(p).ok())
            .and_then(|(_, p)| p.as_dict().ok())
            .and_then(name_of);
        if name_of(dict).as_deref() != Some(field) && parent_name.as_deref() != Some(field) {
            continue;
        }
        let Some(stream) = dict
            .get(b"AP")
            .ok()
            .and_then(|ap| d.dereference(ap).ok())
            .and_then(|(_, ap)| ap.as_dict().ok())
            .and_then(|ap| ap.get(b"N").ok())
            .and_then(|n| d.dereference(n).ok())
            .and_then(|(_, n)| n.as_stream().ok())
        else {
            continue;
        };
        out.extend(stream.decompressed_content().unwrap_or_default());
        out.push(b'\n');
    }
    out
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.into())
}

fn choice(v: &[&str]) -> FieldValue {
    FieldValue::Choice(v.iter().map(|s| s.to_string()).collect())
}

const NOM: PointRect = PointRect {
    left: 150.0,
    bottom: 696.0,
    right: 410.0,
    top: 716.0,
};
const VILLE: PointRect = PointRect {
    left: 150.0,
    bottom: 666.0,
    right: 410.0,
    top: 686.0,
};
const CASE: PointRect = PointRect {
    left: 200.0,
    bottom: 526.0,
    right: 214.0,
    top: 540.0,
};
const CARTE: PointRect = PointRect {
    left: 200.0,
    bottom: 496.0,
    right: 214.0,
    top: 510.0,
};
const VIREMENT: PointRect = PointRect {
    left: 200.0,
    bottom: 476.0,
    right: 214.0,
    top: 490.0,
};

#[test]
fn form_lists_fields_types_values_and_widgets() {
    let engine = engine();
    let doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    assert_eq!(doc.form_kind(), FormKind::AcroForm);
    assert_eq!(doc.info().form, FormKind::AcroForm);
    let info = doc.form_info().unwrap();
    assert_eq!(info.kind, FormKind::AcroForm);
    assert!(!info.has_scripts);
    let names: Vec<&str> = info.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "nom",
            "ville",
            "commentaire",
            "ref",
            "accepte",
            "paiement",
            "pays",
            "langues",
            "adresse.rue",
            "adresse.code",
            "telephone",
            "signature",
            "envoyer"
        ]
    );
    let f = |n: &str| info.field(n).unwrap();
    assert_eq!(f("nom").kind, FieldKind::Text);
    assert_eq!(f("nom").tooltip.as_deref(), Some("Nom et prenom"));
    assert_eq!(f("nom").value, text(""));
    assert_eq!(f("nom").widgets.len(), 1);
    assert_eq!(f("nom").widgets[0].rect, NOM);
    assert_eq!(f("nom").widgets[0].display, NOM, "page non pivotée");
    assert_eq!(f("nom").widgets[0].page, 0);
    assert_eq!(f("ville").value, text("Élodie Müller"), "PDFDocEncoding");
    assert!(f("ref").read_only);
    assert_eq!(f("ref").value, text("REF-42"));
    assert!(f("commentaire").multiline);
    assert_eq!(f("adresse.code").max_len, Some(5));
    assert_eq!(f("adresse.rue").kind, FieldKind::Text);
    let radio = f("paiement");
    assert_eq!(radio.kind, FieldKind::Radio);
    assert_eq!(radio.widgets.len(), 3);
    assert!(radio.no_toggle_off);
    assert_eq!(
        radio
            .options
            .iter()
            .map(|o| o.value.as_str())
            .collect::<Vec<_>>(),
        ["carte", "virement", "especes"]
    );
    assert_eq!(radio.value, choice(&["virement"]));
    assert_eq!(radio.widgets[0].on_state.as_deref(), Some("carte"));
    assert_eq!(f("telephone").widgets.len(), 2);
    let pays = f("pays");
    assert_eq!(pays.kind, FieldKind::Combo);
    assert_eq!(pays.value, choice(&["BE"]));
    assert_eq!(pays.options[0].value, "FR");
    assert_eq!(pays.options[0].label, "France");
    assert_eq!(pays.label_of("BE"), "Belgique");
    let langues = f("langues");
    assert_eq!(langues.kind, FieldKind::List);
    assert!(langues.multi_select);
    assert_eq!(langues.value, choice(&["Anglais"]));
    assert_eq!(langues.options[0].label, "Français");
    assert_eq!(f("accepte").kind, FieldKind::Checkbox);
    assert_eq!(f("accepte").value, FieldValue::Bool(false));
    assert_eq!(f("accepte").widgets[0].on_state.as_deref(), Some("Yes"));
    assert_eq!(f("signature").kind, FieldKind::Signature);
    assert_eq!(f("signature").value, FieldValue::None);
    assert_eq!(f("envoyer").kind, FieldKind::PushButton);
    // Sans AcroForm : rien à analyser.
    let plain = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    assert_eq!(plain.form_kind(), FormKind::None);
    assert_eq!(plain.info().form, FormKind::None);
    assert_eq!(
        plain.form_info().unwrap(),
        sunipdf_core::FormInfo::default()
    );
}

#[test]
fn form_fill_roundtrip_values_and_pixels() {
    let engine = engine();
    let dir = temp_dir("forms-fill");
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    let before = doc.render_page(0, 1.0).unwrap();
    assert_eq!(dark_in(&before, NOM), 0, "apparence vide d'origine");
    assert_eq!(dark_in(&before, CASE), 0);
    assert_eq!(dark_in(&before, CARTE), 0);
    assert!(dark_in(&before, VIREMENT) >= 30, "radio choisi dessiné");
    let values = vec![
        ("nom".to_string(), text("Jean Dupont")),
        ("ville".to_string(), text("Zoë")),
        ("commentaire".to_string(), text("ligne 1\nligne 2")),
        ("accepte".to_string(), FieldValue::Bool(true)),
        ("paiement".to_string(), choice(&["carte"])),
        ("pays".to_string(), choice(&["Canada"])),
        ("langues".to_string(), choice(&["Français", "Allemand"])),
        ("telephone".to_string(), text("0102030405")),
        ("adresse.rue".to_string(), text("1 rue X")),
    ];
    let report = doc.fill_form(&values, FillMode::Strict).unwrap();
    assert_eq!(report.applied.len(), 9);
    assert!(report.skipped_unknown.is_empty() && report.skipped_invalid.is_empty());
    let check = |doc: &sunipdf_core::Document<'_>| {
        let info = doc.form_info().unwrap();
        let f = |n: &str| info.field(n).unwrap();
        assert_eq!(f("nom").value, text("Jean Dupont"));
        assert_eq!(f("ville").value, text("Zoë"));
        assert_eq!(f("commentaire").value, text("ligne 1\nligne 2"));
        assert_eq!(f("accepte").value, FieldValue::Bool(true));
        assert_eq!(f("paiement").value, choice(&["carte"]));
        assert_eq!(f("pays").value, choice(&["CA"]), "libellé → export");
        assert_eq!(f("langues").value, choice(&["Français", "Allemand"]));
        assert_eq!(f("telephone").value, text("0102030405"));
        assert_eq!(f("telephone").widgets.len(), 2);
        assert_eq!(f("adresse.rue").value, text("1 rue X"));
        assert_eq!(f("ref").value, text("REF-42"), "intact");
        let img = doc.render_page(0, 1.0).unwrap();
        assert!(dark_in(&img, NOM) > 300, "texte et bordure régénérés");
        assert!(dark_in(&img, CASE) > 10, "coche");
        assert!(dark_in(&img, CARTE) > 30, "radio carte");
        assert_eq!(dark_in(&img, VIREMENT), 0, "radio virement décoché");
        // Les deux widgets du téléphone affichent la valeur.
        assert!(dark_in(&img, rect(150.0, 236.0, 300.0, 256.0)) > 100);
        assert!(dark_in(&img, rect(320.0, 236.0, 470.0, 256.0)) > 100);
        dark_in(&img, NOM)
    };
    let n1 = check(&doc);
    // Aller-retour par instantané, puis par fichier.
    let snapshot = doc.snapshot().unwrap();
    let again = engine
        .open_bytes(snapshot, None, &fixture("acroform.pdf"))
        .unwrap();
    assert_eq!(check(&again), n1);
    let out = dir.join("filled.pdf");
    doc.save_as(&out).unwrap();
    let reopened = engine.open(&out, None).unwrap();
    assert_eq!(check(&reopened), n1);
    assert_eq!(reopened.info().form, FormKind::AcroForm);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn form_fill_errors_are_typed_and_atomic() {
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    let strict = |doc: &mut sunipdf_core::Document<'_>, name: &str, v: FieldValue| {
        doc.fill_form(&[(name.to_string(), v)], FillMode::Strict)
            .map(|_| ())
    };
    assert!(matches!(
        strict(&mut doc, "zzz", text("x")),
        Err(EngineError::FormFieldNotFound(n)) if n == "zzz"
    ));
    let invalid = |r: Result<(), EngineError>, what: &str| match r {
        Err(EngineError::FormValueInvalid(m)) => {
            assert!(m.starts_with(what), "{m}");
            assert_eq!(
                EngineError::FormValueInvalid(m).code(),
                "form_value_invalid"
            );
        }
        other => panic!("{what} : {other:?}"),
    };
    invalid(strict(&mut doc, "ref", text("x")), "ref");
    invalid(strict(&mut doc, "nom", FieldValue::Bool(true)), "nom");
    invalid(strict(&mut doc, "nom", text("a\nb")), "nom");
    invalid(strict(&mut doc, "pays", choice(&["XX"])), "pays");
    invalid(strict(&mut doc, "pays", choice(&["FR", "BE"])), "pays");
    invalid(
        strict(&mut doc, "adresse.code", text("123456")),
        "adresse.code",
    );
    invalid(strict(&mut doc, "signature", text("x")), "signature");
    invalid(strict(&mut doc, "envoyer", FieldValue::None), "envoyer");
    invalid(strict(&mut doc, "paiement", choice(&[])), "paiement");
    invalid(
        strict(&mut doc, "langues", choice(&["Anglais", "Zoulou"])),
        "langues",
    );
    strict(&mut doc, "commentaire", text("a\nb")).unwrap();
    strict(&mut doc, "adresse.code", text("75001")).unwrap();
    // Atomicité : une entrée inconnue en mode strict n'applique rien.
    let both = vec![
        ("nom".to_string(), text("A")),
        ("zzz".to_string(), text("B")),
    ];
    assert!(doc.fill_form(&both, FillMode::Strict).is_err());
    assert_eq!(
        doc.form_info().unwrap().field("nom").unwrap().value,
        text("")
    );
    let report = doc.fill_form(&both, FillMode::Lenient).unwrap();
    assert_eq!(report.applied, vec!["nom"]);
    assert_eq!(report.skipped_unknown, vec!["zzz"]);
    assert_eq!(
        doc.form_info().unwrap().field("nom").unwrap().value,
        text("A")
    );
    let report = doc
        .fill_form(&[("ref".to_string(), text("x"))], FillMode::Lenient)
        .unwrap();
    assert!(report.applied.is_empty());
    assert_eq!(report.skipped_invalid[0].0, "ref");
}

#[test]
fn form_appearance_is_materialized_in_saved_bytes() {
    let engine = engine();
    // Sans aucun flux d'apparence : PDFium les génère et l'instantané les contient.
    let mut doc = engine.open(&fixture("acroform-noap.pdf"), None).unwrap();
    let before = doc.render_page(0, 1.0).unwrap();
    assert!(
        dark_in(&before, VILLE) > 300,
        "valeur d'origine dessinée sans /AP"
    );
    doc.fill_form(
        &[("nom".to_string(), text("Jean Dupont"))],
        FillMode::Strict,
    )
    .unwrap();
    let ap = ap_bytes(&doc.snapshot().unwrap(), "nom");
    assert!(
        ap.windows(16).any(|w| w == b"(Jean Dupont) Tj"),
        "{}",
        String::from_utf8_lossy(&ap)
    );
    // Apparence existante remplacée (accents en WinAnsi), liste déroulante par libellé.
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    doc.fill_form(
        &[
            ("ville".to_string(), text("Élodie Müller")),
            ("pays".to_string(), choice(&["CA"])),
        ],
        FillMode::Strict,
    )
    .unwrap();
    let bytes = doc.snapshot().unwrap();
    let ville = ap_bytes(&bytes, "ville");
    assert!(
        ville.windows(7).any(|w| w == b"lodie M") && ville.windows(2).any(|w| w == b"Tj"),
        "{}",
        String::from_utf8_lossy(&ville)
    );
    let pays = ap_bytes(&bytes, "pays");
    assert!(
        pays.windows(8).any(|w| w == b"(Canada)"),
        "{}",
        String::from_utf8_lossy(&pays)
    );
}

#[test]
fn flatten_keeps_content_and_removes_widgets() {
    use sunipdf_core::MarkupKind;
    let engine = engine();
    let dir = temp_dir("forms-flatten");
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    doc.fill_form(
        &[("nom".to_string(), text("Jean Dupont"))],
        FillMode::Strict,
    )
    .unwrap();
    // Surlignage créé par PDFium (indicateur /F 0) sur le libellé « Reference (lecture seule) ».
    let sel = doc
        .text_in_rect(0, rect(38.0, 560.0, 199.0, 578.0))
        .unwrap();
    assert!(sel.text.starts_with("Reference"), "{sel:?}");
    doc.add_markup(0, MarkupKind::Highlight, &sel.rects, (255, 230, 0))
        .unwrap();
    let before = doc.render_page(0, 1.0).unwrap();
    let ville_before = dark_in(&before, VILLE);
    assert!(ville_before > 100, "{ville_before}");
    assert!(yellow_count(&before) > 300);
    let n = doc.flatten(None).unwrap();
    assert!(n >= 17, "{n} annotations traitées");
    assert!(doc.annotations(0).unwrap().is_empty());
    assert_eq!(doc.form_kind(), FormKind::None);
    assert_eq!(doc.info().form, FormKind::None);
    assert!(doc.form_info().unwrap().fields.is_empty());
    let after = doc.render_page(0, 1.0).unwrap();
    // Widget sans indicateur Print (ville), champ rempli, surlignage : tous dessinés.
    assert!(dark_in(&after, VILLE) > 100, "{}", dark_in(&after, VILLE));
    assert!(dark_in(&after, NOM) > 300);
    assert!(yellow_count(&after) > 300);
    assert!(dark_in(&after, VIREMENT) >= 30, "radio choisi conservé");
    let out = dir.join("flat.pdf");
    doc.save_as(&out).unwrap();
    let reopened = engine.open(&out, None).unwrap();
    assert_eq!(reopened.form_kind(), FormKind::None);
    assert!(reopened.annotations(0).unwrap().is_empty());
    let img = reopened.render_page(0, 1.0).unwrap();
    assert!(dark_in(&img, NOM) > 300 && yellow_count(&img) > 300);
    assert_eq!(doc.flatten(None).unwrap(), 0, "plus rien à aplatir");
    // Sans flux d'apparence et sans rendu préalable : rien ne disparaît.
    let mut noap = engine.open(&fixture("acroform-noap.pdf"), None).unwrap();
    noap.flatten(None).unwrap();
    let img = noap.render_page(0, 1.0).unwrap();
    assert!(dark_in(&img, VILLE) > 300);
    // Aplatissement partiel (document de trois pages) : la page aplatie perd ses widgets, le
    // formulaire reste ; toutes les pages d'un coup retirent /AcroForm.
    let mut partial = engine.open(&fixture("acroform.pdf"), None).unwrap();
    partial
        .insert_pages_from(&fixture("two-pages.pdf"), None, &[], 1)
        .unwrap();
    assert_eq!(partial.page_count(), 3);
    partial.flatten(Some(&[0])).unwrap();
    assert_eq!(partial.form_kind(), FormKind::AcroForm);
    let info = partial.form_info().unwrap();
    assert_eq!(info.fields.len(), 13);
    assert!(info.fields.iter().all(|f| f.widgets.is_empty()));
    assert!(matches!(
        partial.flatten(Some(&[7])),
        Err(EngineError::PageOutOfRange(7))
    ));
    partial.flatten(Some(&[1, 2])).unwrap();
    assert_eq!(
        partial.form_kind(),
        FormKind::AcroForm,
        "pages 2 et 3 seulement"
    );
    partial.flatten(None).unwrap();
    assert_eq!(partial.form_kind(), FormKind::None);
    let _ = std::fs::remove_dir_all(dir);
}

/// Liens `/Link` de la page 1 relus par lopdf : (URI, page de destination d'un GoTo), triés.
fn links_in(bytes: &[u8]) -> Vec<(Option<String>, Option<u32>)> {
    let d = lopdf::Document::load_mem(bytes).unwrap();
    let pages = d.get_pages();
    let page = d.get_dictionary(pages[&1]).unwrap();
    let annots = d
        .get_object(
            page.get(b"Annots")
                .unwrap()
                .as_reference()
                .unwrap_or((0, 0)),
        )
        .ok()
        .or_else(|| page.get(b"Annots").ok())
        .and_then(|a| a.as_array().ok())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for a in annots {
        let (_, a) = d.dereference(&a).unwrap();
        let a = a.as_dict().unwrap();
        if a.get(b"Subtype").unwrap().as_name().unwrap() != b"Link" {
            continue;
        }
        let (_, action) = d.dereference(a.get(b"A").unwrap()).unwrap();
        let action = action.as_dict().unwrap();
        let uri = action
            .get(b"URI")
            .ok()
            .map(|u| String::from_utf8_lossy(u.as_str().unwrap()).into_owned());
        let dest = action
            .get(b"D")
            .ok()
            .and_then(|dst| dst.as_array().ok())
            .and_then(|dst| dst.first()?.as_reference().ok())
            .and_then(|id| pages.iter().find(|(_, pid)| **pid == id).map(|(n, _)| *n));
        out.push((uri, dest));
    }
    out.sort();
    out
}

#[test]
fn flatten_keeps_link_annotations() {
    let engine = engine();
    let dir = temp_dir("forms-flatten-links");
    let expected = vec![
        (None, Some(2)),
        (Some("https://example.org/suni".to_string()), None),
    ];
    let mut doc = engine.open(&fixture("acroform-links.pdf"), None).unwrap();
    assert_eq!(doc.page_count(), 2);
    assert_eq!(links_in(&doc.snapshot().unwrap()), expected);
    assert_eq!(doc.form_info().unwrap().fields.len(), 14);
    doc.fill_form(&[("nom".to_string(), text("Jean"))], FillMode::Strict)
        .unwrap();
    // 17 widgets aplatis ; les deux liens ne comptent pas.
    assert_eq!(doc.flatten(None).unwrap(), 17);
    assert_eq!(doc.form_kind(), FormKind::None);
    let after = doc.annotations(0).unwrap();
    assert_eq!(after.len(), 2, "{after:?}");
    assert!(after.iter().all(|a| a.kind == "link"));
    assert_eq!(after[0].rect, rect(40.0, 740.0, 200.0, 760.0));
    assert_eq!(after[1].rect, rect(300.0, 740.0, 500.0, 760.0));
    assert_eq!(
        links_in(&doc.snapshot().unwrap()),
        expected,
        "cibles intactes"
    );
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(dark_in(&img, NOM) > 300, "valeur aplatie dessinée");
    let out = dir.join("links-flat.pdf");
    doc.save_as(&out).unwrap();
    let mut reopened = engine.open(&out, None).unwrap();
    assert_eq!(reopened.form_kind(), FormKind::None);
    assert_eq!(links_in(&std::fs::read(&out).unwrap()), expected);
    assert_eq!(reopened.annotations(0).unwrap().len(), 2);
    // Page où il ne reste que des liens : PDFium n'a rien à faire et garde /Annots ; les liens
    // ne sont pas dupliqués.
    assert_eq!(reopened.flatten(None).unwrap(), 0, "plus rien à aplatir");
    assert_eq!(reopened.annotations(0).unwrap().len(), 2);
    assert_eq!(links_in(&reopened.snapshot().unwrap()), expected);
    // Aplatissement partiel (page 1 seulement, formulaire conservé) : mêmes liens ; pages en
    // double dans la liste = une seule fois.
    let mut partial = engine.open(&fixture("acroform-links.pdf"), None).unwrap();
    assert_eq!(partial.flatten(Some(&[0, 0])).unwrap(), 17);
    assert_eq!(partial.form_kind(), FormKind::AcroForm);
    assert_eq!(links_in(&partial.snapshot().unwrap()), expected);
    assert_eq!(partial.annotations(0).unwrap().len(), 2);
    // Page sans lien ni champ : rien à faire, rien ne change.
    assert_eq!(partial.flatten(Some(&[1])).unwrap(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn form_password_field_is_listed_but_never_filled_nor_exported() {
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform-links.pdf"), None).unwrap();
    let info = doc.form_info().unwrap();
    let secret = info.field("secret").unwrap();
    assert!(secret.password && !secret.fillable() && secret.cleared().is_none());
    assert!(form_data::values_to_json(&info).get("secret").is_none());
    let err = doc
        .fill_form(&[("secret".to_string(), text("x"))], FillMode::Strict)
        .unwrap_err();
    assert_eq!(err.code(), "form_value_invalid");
    // Export → import strict : rien d'ignoré, la lecture seule n'est pas réécrite.
    let plan = form_data::plan_from_json(&info, &form_data::values_to_json(&info)).unwrap();
    assert!(plan.skipped_unknown.is_empty() && plan.skipped_invalid.is_empty());
    assert!(plan.values.iter().all(|(n, _)| n != "ref" && n != "secret"));
    let report = doc.fill_form(&plan.values, FillMode::Strict).unwrap();
    assert_eq!(report.applied.len(), plan.values.len());
    // La valeur courante d'un champ en lecture seule est acceptée sans être écrite.
    let report = doc
        .fill_form(&[("ref".to_string(), text("REF-42"))], FillMode::Strict)
        .unwrap();
    assert!(report.applied.is_empty() && report.skipped_invalid.is_empty());
    assert_eq!(
        doc.form_info().unwrap().field("ref").unwrap().value,
        text("REF-42")
    );
}

#[test]
fn form_xfa_is_reported_and_blocks_write() {
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform-xfa.pdf"), None).unwrap();
    assert_eq!(doc.form_kind(), FormKind::Xfa);
    assert_eq!(doc.info().form, FormKind::Xfa);
    let info = doc.form_info().unwrap();
    assert_eq!(info.kind, FormKind::Xfa);
    assert_eq!(info.fields.len(), 13, "champs AcroForm lisibles");
    let err = doc
        .fill_form(&[("nom".to_string(), text("x"))], FillMode::Strict)
        .unwrap_err();
    assert!(matches!(err, EngineError::FormXfa));
    assert_eq!(err.code(), "form_xfa");
    assert!(matches!(doc.flatten(None), Err(EngineError::FormXfa)));
    assert!(matches!(doc.clear_form(), Err(EngineError::FormXfa)));
}

#[test]
fn form_fill_keeps_encryption() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = temp_dir("forms-encrypted");
    let input = std::fs::read(fixture("acroform.pdf")).unwrap();
    let bytes = security::protect(
        &input,
        None,
        &Protection {
            user_password: "secret".into(),
            owner_password: "owner".into(),
            aes256: true,
            ..Default::default()
        },
    )
    .unwrap();
    let path = dir.join("form-aes.pdf");
    std::fs::write(&path, bytes).unwrap();
    let mut doc = engine.open(&path, Some("secret")).unwrap();
    assert!(doc.properties().encrypted);
    assert_eq!(doc.form_info().unwrap().fields.len(), 13);
    doc.fill_form(
        &[
            ("nom".to_string(), text("Chiffré")),
            ("accepte".to_string(), FieldValue::Bool(true)),
        ],
        FillMode::Strict,
    )
    .unwrap();
    assert!(doc.properties().encrypted, "le document reste chiffré");
    let out = dir.join("filled-aes.pdf");
    doc.save_as(&out).unwrap();
    assert!(matches!(
        engine.open(&out, None),
        Err(EngineError::PasswordRequired)
    ));
    let reopened = engine.open(&out, Some("secret")).unwrap();
    let info = reopened.form_info().unwrap();
    assert_eq!(info.field("nom").unwrap().value, text("Chiffré"));
    assert_eq!(info.field("accepte").unwrap().value, FieldValue::Bool(true));
    let img = reopened.render_page(0, 1.0).unwrap();
    assert!(dark_in(&img, NOM) > 300 && dark_in(&img, CASE) > 10);
    // Aplatir un document chiffré : toujours chiffré, sans champ.
    let mut flat = engine.open(&out, Some("secret")).unwrap();
    flat.flatten(None).unwrap();
    assert_eq!(flat.form_kind(), FormKind::None);
    assert!(flat.properties().encrypted);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn form_display_rect_follows_page_rotation() {
    use sunipdf_core::Rotation;
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    let expect = [
        (Rotation::R90, rect(696.0, 202.0, 716.0, 462.0)),
        (Rotation::R180, rect(202.0, 76.0, 462.0, 96.0)),
        (Rotation::R270, rect(76.0, 150.0, 96.0, 410.0)),
        (Rotation::R0, NOM),
    ];
    for (rotation, display) in expect {
        doc.set_page_rotation(0, rotation).unwrap();
        let info = doc.form_info().unwrap();
        let w = info.field("nom").unwrap().widgets[0].clone();
        assert_eq!(w.rect, NOM, "{rotation:?} : /Rect inchangé");
        assert!(
            near(w.display, display, 0.01),
            "{rotation:?} : {:?}",
            w.display
        );
        let size = doc.page_size(0).unwrap();
        assert!(w.display.right <= size.width_pt && w.display.top <= size.height_pt);
        // La valeur saisie se dessine bien dans le rectangle affiché.
        doc.fill_form(&[("nom".to_string(), text("Pivot"))], FillMode::Strict)
            .unwrap();
        let img = doc.render_page(0, 1.0).unwrap();
        assert!(dark_in(&img, w.display) > 100, "{rotation:?}");
    }
}

#[test]
fn form_json_export_import_roundtrip() {
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    let info = doc.form_info().unwrap();
    let v = form_data::values_to_json(&info);
    assert_eq!(v["ville"], "Élodie Müller");
    assert_eq!(v["accepte"], false);
    assert_eq!(v["paiement"], "virement");
    assert_eq!(v["pays"], "BE");
    assert_eq!(v["langues"], serde_json::json!(["Anglais"]));
    assert_eq!(v["nom"], "");
    assert!(v.get("signature").is_none() && v.get("envoyer").is_none());
    let plan = form_data::plan_from_json(
        &info,
        &serde_json::json!({
            "nom": "Zoé", "accepte": "oui", "paiement": "carte", "pays": "Canada",
            "langues": ["Allemand"], "inconnu": 1, "ref": "x", "adresse.code": "123456"
        }),
    )
    .unwrap();
    assert_eq!(plan.values.len(), 5);
    assert_eq!(plan.skipped_unknown, vec!["inconnu"]);
    let mut invalid: Vec<&str> = plan
        .skipped_invalid
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    invalid.sort_unstable();
    assert_eq!(invalid, vec!["adresse.code", "ref"]);
    let report = doc.fill_form(&plan.values, FillMode::Strict).unwrap();
    assert_eq!(report.applied.len(), 5);
    let v = form_data::values_to_json(&doc.form_info().unwrap());
    assert_eq!(v["nom"], "Zoé");
    assert_eq!(v["accepte"], true);
    assert_eq!(v["paiement"], "carte");
    assert_eq!(v["pays"], "CA");
    assert_eq!(v["langues"], serde_json::json!(["Allemand"]));
    let full = form_data::info_to_json(&info);
    assert_eq!(full["kind"], "acroform");
    assert_eq!(full["count"], 13);
    assert_eq!(full["fields"][0]["widgets"][0]["page"], 1);
    assert_eq!(
        full["fields"][0]["widgets"][0]["rect"],
        serde_json::json!([150.0, 696.0, 410.0, 716.0])
    );
}

#[test]
fn form_clear_resets_editable_fields() {
    let engine = engine();
    let mut doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
    doc.fill_form(
        &[
            ("nom".to_string(), text("Jean")),
            ("accepte".to_string(), FieldValue::Bool(true)),
        ],
        FillMode::Strict,
    )
    .unwrap();
    // nom, ville, accepte, pays, langues changent ; commentaire, adresse.* et telephone sont déjà
    // vides ; ref est en lecture seule ; paiement garde toujours un choix.
    assert_eq!(doc.clear_form().unwrap(), 5);
    let info = doc.form_info().unwrap();
    let f = |n: &str| &info.field(n).unwrap().value;
    assert_eq!(*f("nom"), text(""));
    assert_eq!(*f("ville"), text(""));
    assert_eq!(*f("accepte"), FieldValue::Bool(false));
    assert_eq!(*f("pays"), choice(&[]));
    assert_eq!(*f("langues"), choice(&[]));
    assert_eq!(*f("ref"), text("REF-42"));
    assert_eq!(*f("paiement"), choice(&["virement"]));
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!(dark_in(&img, CASE), 0);
    assert!(dark_in(&img, VIREMENT) >= 30);
    assert_eq!(doc.clear_form().unwrap(), 0, "déjà vide");
}

// ----- Annotations avancées (encre, texte libre, formes, liens, relecture) -----

use sunipdf_core::{
    thread_status, AnnotShape, AnnotStyle, Document as CoreDocument, FreeTextStyle, LinkTarget,
    ReviewStatus,
};

fn astyle(rgb: (u8, u8, u8), width: f32) -> AnnotStyle {
    AnnotStyle {
        rgb,
        fill_rgb: None,
        width_pt: width,
        opacity: 1.0,
    }
}

fn count(img: &RenderedPage, pred: impl Fn([u8; 3]) -> bool) -> usize {
    img.rgba
        .chunks(4)
        .filter(|p| pred([p[0], p[1], p[2]]))
        .count()
}

/// Pixels qui vérifient `pred` dans le rectangle d'affichage `r` (rendu à l'échelle 1).
fn count_in(img: &RenderedPage, r: PointRect, pred: impl Fn([u8; 3]) -> bool) -> usize {
    let mut n = 0;
    for y in r.bottom.floor() as i32..r.top.ceil() as i32 {
        for x in r.left.floor() as i32..r.right.ceil() as i32 {
            if pred(px(img, x as f32 + 0.5, y as f32 + 0.5)) {
                n += 1;
            }
        }
    }
    n
}

fn is_green(p: [u8; 3]) -> bool {
    p[1] > 150 && p[0] < 100 && p[2] < 100
}

fn is_yellowish(p: [u8; 3]) -> bool {
    p[0] > 200 && p[1] > 180 && p[2] < 120
}

fn is_magenta(p: [u8; 3]) -> bool {
    p[0] > 200 && p[1] < 60 && p[2] > 200
}

/// Dictionnaires bruts de `/Annots` de la page 1, lus par lopdf dans un instantané.
fn raw_annots(doc: &CoreDocument<'_>) -> Vec<lopdf::Dictionary> {
    let pdf = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
    let page = *pdf.get_pages().get(&1).unwrap();
    let annots = pdf.get_dictionary(page).unwrap().get(b"Annots").unwrap();
    let list = pdf
        .dereference(annots)
        .unwrap()
        .1
        .as_array()
        .unwrap()
        .clone();
    list.iter()
        .map(|e| {
            pdf.dereference(e)
                .unwrap()
                .1
                .as_dict()
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn ink_annotation_round_trip() {
    let engine = engine();
    let dir = temp_dir("ink");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let strokes = vec![
        vec![(100.0, 100.0), (150.0, 160.0), (200.0, 100.0)],
        vec![(100.0, 200.0), (200.0, 200.0)],
    ];
    let i = doc
        .add_ink_annotation(0, &strokes, &astyle((0, 0, 255), 4.0), Some("Alice"))
        .unwrap();
    assert_eq!(i, 0);
    let list = doc.annotations(0).unwrap();
    assert_eq!((list.len(), list[0].kind), (1, "ink"));
    // Boîte des points ± (demi-épaisseur + 0,5).
    assert!(
        near(list[0].rect, rect(97.5, 97.5, 202.5, 202.5), 0.6),
        "{:?}",
        list[0].rect
    );
    let d = doc.annotation_details(0).unwrap();
    assert!(d[0].ours && d[0].can_move && d[0].can_resize && d[0].can_restyle);
    assert_eq!(d[0].author.as_deref(), Some("Alice"));
    assert_eq!(d[0].subtype, "Ink");
    assert!(d[0].modified.as_deref().unwrap().starts_with("D:20"));

    let img = doc.render_page(0, 1.0).unwrap();
    let blue_page = count(&img, is_blue);
    // Le rectangle bleu de la page (72..272 × 500..650) compte aussi : on compte hors de lui.
    let in_box = count_in(&img, rect(94.0, 94.0, 206.0, 206.0), is_blue);
    assert!(in_box > 300, "trait rendu : {in_box} pixels");
    let seen = pixel_box_below(&img, 300.0, is_blue).unwrap();
    assert!(
        near(seen, rect(98.0, 98.0, 202.0, 202.0), 3.0),
        "pixels bleus {seen:?}"
    );
    assert!(blue_page > in_box);

    // lopdf relit /InkList : deux tracés ; le premier garde ses trois sommets.
    let raw = raw_annots(&doc);
    let ink = raw[0].get(b"InkList").unwrap().as_array().unwrap();
    assert_eq!(ink.len(), 2);
    assert_eq!(ink[0].as_array().unwrap().len(), 6);

    let out = dir.join("ink.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    let img2 = again.render_page(0, 1.0).unwrap();
    let in_box2 = count_in(&img2, rect(94.0, 94.0, 206.0, 206.0), is_blue);
    assert!(
        (in_box2 as f32 - in_box as f32).abs() <= in_box as f32 * 0.02,
        "{in_box} → {in_box2}"
    );
    assert!(again.annotation_details(0).unwrap()[0].ours);

    // Erreurs : rien à tracer, trait démesuré, épaisseur hors bornes. Un point isolé est un point.
    let s = astyle((0, 0, 255), 4.0);
    assert!(doc.add_ink_annotation(0, &[], &s, None).is_err());
    assert!(doc.add_ink_annotation(0, &[vec![]], &s, None).is_err());
    assert!(doc
        .add_ink_annotation(0, &[vec![(f32::NAN, 1.0), (2.0, f32::NAN)]], &s, None)
        .is_err());
    assert!(doc
        .add_ink_annotation(0, &strokes, &astyle((0, 0, 255), 80.0), None)
        .is_err());
    let many: Vec<Vec<(f32, f32)>> = (0..65).map(|_| strokes[0].clone()).collect();
    assert!(doc.add_ink_annotation(0, &many, &s, None).is_err());
    assert!(matches!(
        doc.add_ink_annotation(7, &strokes, &s, None),
        Err(EngineError::PageOutOfRange(7))
    ));
    assert_eq!(doc.annotations(0).unwrap().len(), 1, "échecs sans effet");
    doc.add_ink_annotation(0, &[vec![(300.0, 300.0)]], &s, None)
        .unwrap();
    let dot = doc.render_page(0, 1.0).unwrap();
    assert!(count_in(&dot, rect(296.0, 296.0, 304.0, 304.0), is_blue) > 6);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn shape_annotations_render() {
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    // Rectangle jaune à moitié opaque, trait bleu.
    let mut filled = astyle((0, 0, 255), 2.0);
    filled.fill_rgb = Some((255, 255, 0));
    filled.opacity = 0.5;
    doc.add_shape_annotation(
        0,
        AnnotShape::Square,
        rect(100.0, 100.0, 200.0, 200.0),
        &filled,
        None,
    )
    .unwrap();
    // Ellipse sans fond : centre blanc, anneau rouge.
    doc.add_shape_annotation(
        0,
        AnnotShape::Circle,
        rect(250.0, 100.0, 350.0, 200.0),
        &astyle((255, 0, 0), 4.0),
        None,
    )
    .unwrap();
    // Ligne puis flèche de même longueur.
    let blue = astyle((0, 0, 255), 3.0);
    doc.add_shape_annotation(
        0,
        AnnotShape::Line,
        rect(100.0, 300.0, 300.0, 300.0),
        &blue,
        None,
    )
    .unwrap();
    doc.add_shape_annotation(
        0,
        AnnotShape::Arrow,
        rect(100.0, 400.0, 300.0, 400.0),
        &blue,
        None,
    )
    .unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    let c = px(&img, 150.0, 150.0);
    assert!(
        c[0] > 240 && c[1] > 240 && (90..170).contains(&c[2]),
        "fond jaune à 50 % sur blanc : {c:?}"
    );
    assert_eq!(
        px(&img, 300.0, 150.0),
        [255, 255, 255],
        "centre de l'ellipse"
    );
    assert!(
        is_red(px(&img, 347.0, 150.0)),
        "anneau {:?}",
        px(&img, 347.0, 150.0)
    );
    let tip = |y: f32| rect(280.0, y - 10.0, 300.0, y + 10.0);
    let (line_tip, arrow_tip) = (
        count_in(&img, tip(300.0), is_blue),
        count_in(&img, tip(400.0), is_blue),
    );
    assert!(
        arrow_tip as f32 > line_tip as f32 * 1.4,
        "pointe : {arrow_tip} contre {line_tip}"
    );
    let tail = count_in(&img, rect(100.0, 390.0, 120.0, 410.0), is_blue);
    assert!(arrow_tip > tail, "la pointe est du côté de l'extrémité");
    let details = doc.annotation_details(0).unwrap();
    assert_eq!(
        details
            .iter()
            .map(|d| d.subtype.as_str())
            .collect::<Vec<_>>(),
        ["Square", "Circle", "Line", "Line"]
    );
    assert!(!details[2].arrow && details[3].arrow);
    assert_eq!(details[0].fill_rgb, Some((255, 255, 0)));
    assert!((details[0].opacity - 0.5).abs() < 1e-3);
    // Erreurs.
    assert!(doc
        .add_shape_annotation(
            0,
            AnnotShape::Line,
            rect(50.0, 50.0, 50.2, 50.2),
            &blue,
            None
        )
        .is_err());
    assert!(doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(50.0, 50.0, 51.0, 80.0),
            &blue,
            None
        )
        .is_err());
    assert_eq!(doc.annotations(0).unwrap().len(), 4);
}

#[test]
fn free_text_wraps_grows_and_keeps_the_exact_text() {
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore";
    let style = FreeTextStyle::default();
    let r = rect(300.0, 300.0, 450.0, 320.0);
    doc.add_free_text_annotation(0, r, text, &style, Some("Bob"))
        .unwrap();
    let d = &doc.annotation_details(0).unwrap()[0];
    assert_eq!(d.subtype, "FreeText");
    assert!(d.rect.top - d.rect.bottom > 20.0, "grandit : {:?}", d.rect);
    assert!(
        (d.rect.top - 320.0).abs() < 0.01,
        "vers le bas : {:?}",
        d.rect
    );
    assert_eq!((d.rect.left, d.rect.right), (300.0, 450.0));
    assert_eq!(d.contents.as_deref(), Some(text));
    assert_eq!((d.font_size, d.rgb), (Some(12.0), Some((0, 0, 0))));
    let img = doc.render_page(0, 1.0).unwrap();
    // Au moins trois bandes de lignes de pixels sombres.
    let mut bands = 0;
    let mut inside = false;
    for y in 0..792 {
        let dark = (300..450).any(|x| is_dark(px(&img, x as f32 + 0.5, y as f32 + 0.5)));
        if dark && !inside {
            bands += 1;
        }
        inside = dark;
    }
    assert!(bands >= 3, "{bands} lignes de texte");
    let seen = pixel_box_below(&img, 400.0, is_dark).unwrap();
    assert!(
        seen.left >= 299.0
            && seen.right <= 451.0
            && seen.top <= 321.5
            && seen.bottom >= d.rect.bottom - 1.5,
        "texte dans la boîte : {seen:?} / {:?}",
        d.rect
    );

    // Accents et caractères hors WinAnsi : le contenu garde le texte exact.
    for t in ["Élève à l'école — ça va ?", "Coché ✓ et 日本"] {
        doc.add_free_text_annotation(0, rect(100.0, 100.0, 300.0, 130.0), t, &style, None)
            .unwrap();
        let all = doc.annotation_details(0).unwrap();
        assert_eq!(all.last().unwrap().contents.as_deref(), Some(t));
    }
    // Erreurs.
    for (rc, t) in [
        (r, "   "),
        (r, ""),
        (rect(0.0, 0.0, 10.0, 30.0), "trop étroit"),
    ] {
        assert!(doc
            .add_free_text_annotation(0, rc, t, &style, None)
            .is_err());
    }
    let mut bad = style.clone();
    bad.size_pt = 1.0;
    assert!(doc.add_free_text_annotation(0, r, "x", &bad, None).is_err());
}

#[test]
fn annotations_land_where_displayed_on_rotated_and_cropped_pages() {
    let engine = engine();
    let mut doc = engine.open(&fixture("boxes.pdf"), None).unwrap();
    // Texte de référence sur une page droite (région vierge de two-pages.pdf).
    let tr = rect(100.0, 300.0, 260.0, 330.0);
    let tstyle = FreeTextStyle {
        fill_rgb: Some((255, 255, 200)),
        ..Default::default()
    };
    let mut flat = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    flat.add_free_text_annotation(0, tr, "Upright 123 abc", &tstyle, None)
        .unwrap();
    let reference = flat.render_page(0, 1.0).unwrap();
    let mut magenta_fill = astyle((255, 0, 255), 0.0);
    magenta_fill.fill_rgb = Some((255, 0, 255));
    for p in 0..4 {
        let target = rect(200.0, 150.0, 260.0, 200.0);
        doc.add_shape_annotation(p, AnnotShape::Square, target, &magenta_fill, None)
            .unwrap();
        doc.add_shape_annotation(
            p,
            AnnotShape::Arrow,
            rect(300.0, 250.0, 400.0, 250.0),
            &astyle((0, 0, 255), 3.0),
            None,
        )
        .unwrap();
        doc.add_free_text_annotation(p, tr, "Upright 123 abc", &tstyle, None)
            .unwrap();
        let img = doc.render_page(p, 1.0).unwrap();
        let seen = pixel_box(&img, is_magenta).expect("carré magenta");
        assert!(near(seen, target, 1.5), "page {p} : {seen:?} ≠ {target:?}");
        let details = doc.annotation_details(p).unwrap();
        assert!(near(details[0].rect, target, 0.01), "page {p}");
        // Pointe de la flèche à droite à l'écran, quelle que soit la rotation de la page.
        let head = count_in(&img, rect(380.0, 238.0, 402.0, 262.0), is_blue);
        let tail = count_in(&img, rect(298.0, 238.0, 320.0, 262.0), is_blue);
        assert!(
            head as f32 > tail as f32 * 1.4,
            "page {p} : pointe {head}, queue {tail}"
        );
        // Le texte libre est droit à l'écran : même rendu que sur une page non tournée.
        let mut diff = 0usize;
        let mut total = 0usize;
        for y in 300..330 {
            for x in 100..260 {
                let (a, b) = (
                    px(&img, x as f32 + 0.5, y as f32 + 0.5),
                    px(&reference, x as f32 + 0.5, y as f32 + 0.5),
                );
                total += 1;
                if !close3(a, b, 60) {
                    diff += 1;
                }
            }
        }
        assert!(
            diff * 100 < total * 3,
            "page {p} : texte différent du rendu droit ({diff}/{total})"
        );
    }
}

#[test]
fn link_annotations_and_safety() {
    let engine = engine();
    let dir = temp_dir("links");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let blue = astyle((0, 0, 255), 2.0);
    let sq = |doc: &mut CoreDocument<'_>, x: f32| {
        doc.add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(x, 20.0, x + 30.0, 50.0),
            &blue,
            None,
        )
        .unwrap();
    };
    sq(&mut doc, 20.0);
    let page_link = rect(72.0, 600.0, 200.0, 630.0);
    let url_link = rect(300.0, 600.0, 400.0, 630.0);
    assert_eq!(
        doc.add_link_annotation(0, page_link, &LinkTarget::Page(1))
            .unwrap(),
        1
    );
    sq(&mut doc, 80.0);
    doc.add_link_annotation(0, url_link, &LinkTarget::Uri("https://example.org".into()))
        .unwrap();
    let links = doc.page_links(0).unwrap();
    assert_eq!(links.len(), 2, "pas de doublons : {links:?}");
    assert_eq!((links[0].index, links[1].index), (1, 3));
    assert_eq!(links[0].target, LinkTarget::Page(1));
    assert_eq!(
        links[1].target,
        LinkTarget::Uri("https://example.org".into())
    );
    assert!(near(links[0].rect, page_link, 0.01) && near(links[1].rect, url_link, 0.01));
    // Les mêmes liens vus par lopdf.
    let d = doc.annotation_details(0).unwrap();
    assert_eq!(d[1].link, Some(LinkTarget::Page(1)));
    assert_eq!(d[3].link, links.last().map(|l| l.target.clone()));
    // Refus : page absente, adresses dangereuses, cible non prise en charge, zone trop petite.
    assert!(matches!(
        doc.add_link_annotation(0, page_link, &LinkTarget::Page(9)),
        Err(EngineError::PageOutOfRange(9))
    ));
    for bad in [
        "file:///C:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        "C:\\Windows\\calc.exe",
        "ms-msdt:id",
    ] {
        assert!(
            doc.add_link_annotation(0, page_link, &LinkTarget::Uri(bad.into()))
                .is_err(),
            "{bad}"
        );
    }
    assert!(doc
        .add_link_annotation(0, page_link, &LinkTarget::Unsupported)
        .is_err());
    assert!(doc
        .add_link_annotation(0, rect(1.0, 1.0, 2.0, 2.0), &LinkTarget::Page(0))
        .is_err());
    assert_eq!(doc.annotations(0).unwrap().len(), 4);
    // Après enregistrement et ouverture.
    let out = dir.join("links.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    assert_eq!(again.page_links(0).unwrap(), links);
    // Page tournée et rognée : la zone revient où on l'a posée.
    let mut boxes = engine.open(&fixture("boxes.pdf"), None).unwrap();
    for p in [0, 2, 3] {
        let r = rect(10.0, 20.0, 70.0, 50.0);
        boxes
            .add_link_annotation(p, r, &LinkTarget::Page(0))
            .unwrap();
        let l = boxes.page_links(p).unwrap();
        assert!(near(l[0].rect, r, 0.01), "page {p} : {:?}", l[0].rect);
        assert_eq!(l[0].target, LinkTarget::Page(0));
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn annotated_fixture_is_read_faithfully() {
    let engine = engine();
    let doc = engine.open(&fixture("annotated.pdf"), None).unwrap();
    let d = doc.annotation_details(0).unwrap();
    let subtypes: Vec<&str> = d.iter().map(|d| d.subtype.as_str()).collect();
    assert_eq!(
        subtypes,
        [
            "Square",
            "Ink",
            "FreeText",
            "Highlight",
            "Text",
            "Text",
            "Text",
            "Link",
            "Square",
            "Link",
            "Link",
            "Popup"
        ]
    );
    assert!(d.iter().all(|d| !d.ours && !d.locked && !d.hidden));
    assert_eq!(
        (d[5].parent, d[6].parent, d[4].parent),
        (Some(4), Some(4), None)
    );
    assert_eq!(d[6].status, Some(ReviewStatus::Accepted));
    assert_eq!(d[5].status, None);
    assert_eq!(thread_status(&d, 4), ReviewStatus::Accepted);
    assert_eq!(d[4].replies, 2);
    assert_eq!(d[5].contents.as_deref(), Some("Réponse de Bob"));
    assert_eq!(d[5].author.as_deref(), Some("Bob"));
    assert_eq!(d[0].author.as_deref(), Some("Alice"));
    assert_eq!(d[0].contents.as_deref(), Some("Cadre rouge"));
    assert_eq!(d[0].rgb, Some((255, 0, 0)));
    assert!(near(d[0].rect, rect(100.0, 500.0, 200.0, 560.0), 0.01));
    assert_eq!(d[2].font_size, Some(12.0));
    assert_eq!(
        d[7].link,
        Some(LinkTarget::Uri("https://example.com".into()))
    );
    assert_eq!(d[9].link, Some(LinkTarget::Page(1)));
    assert_eq!(d[10].link, Some(LinkTarget::Unsupported));
    // Droits : déplacer oui, redimensionner et restyler non (autre producteur) ; surlignage
    // recolorable mais pas déplaçable.
    assert!(d[0].can_move && !d[0].can_resize && !d[0].can_restyle && d[0].can_edit_text);
    assert!(!d[2].can_edit_text && d[3].can_restyle && !d[3].can_move);
    assert!(!d[11].can_move && !d[11].can_edit_text);
    // La lecture PDFium (liste rapide et liens) voit la même chose, sans doublon.
    assert_eq!(doc.annotations(0).unwrap().len(), 12);
    let links = doc.page_links(0).unwrap();
    assert_eq!(
        links.iter().map(|l| l.index).collect::<Vec<_>>(),
        [7, 9, 10]
    );
    assert_eq!(
        links[0].target,
        LinkTarget::Uri("https://example.com".into())
    );
    assert_eq!(links[1].target, LinkTarget::Page(1));
    assert_eq!(links[2].target, LinkTarget::Unsupported);
    assert!(doc.annotation_details(1).unwrap().is_empty());
}

#[test]
fn existing_annotations_move_restyle_and_delete_without_touching_the_others() {
    let engine = engine();
    let mut doc = engine.open(&fixture("annotated.pdf"), None).unwrap();
    let before = doc.annotation_details(0).unwrap();
    let img0 = doc.render_page(0, 1.0).unwrap();
    let red0 = pixel_box(&img0, is_red).unwrap();

    // (a) Déplacement du carré d'Acrobat (+30, −20) : /Rect et rendu suivent, le reste est intact.
    doc.move_annotation(0, 0, 30.0, -20.0).unwrap();
    let after = doc.annotation_details(0).unwrap();
    assert!(near(after[0].rect, rect(130.0, 480.0, 230.0, 540.0), 0.01));
    let red1 = pixel_box(&doc.render_page(0, 1.0).unwrap(), is_red).unwrap();
    assert!(
        near(
            red1,
            rect(
                red0.left + 30.0,
                red0.bottom - 20.0,
                red0.right + 30.0,
                red0.top - 20.0
            ),
            1.5
        ),
        "{red0:?} → {red1:?}"
    );
    for i in 1..before.len() {
        let (mut a, mut b) = (before[i].clone(), after[i].clone());
        (a.modified, b.modified) = (None, None);
        assert_eq!(a, b, "annotation {i} intacte");
    }
    // L'encre : tous ses points suivent.
    doc.move_annotation(0, 1, 30.0, 0.0).unwrap();
    let ink = &raw_annots(&doc)[1];
    let first = ink.get(b"InkList").unwrap().as_array().unwrap()[0]
        .as_array()
        .unwrap()[0]
        .as_float()
        .unwrap();
    assert!((first - 140.0).abs() < 0.01, "{first}");
    // Non déplaçables : surlignage (ancré au texte), fenêtre contextuelle, indice absent.
    for i in [3usize, 11, 99] {
        assert!(doc.move_annotation(0, i, 5.0, 5.0).is_err(), "{i}");
    }

    // (b) Redimensionner ou restyler une annotation étrangère est refusé (apparence intacte).
    let r = rect(100.0, 100.0, 200.0, 200.0);
    assert!(doc.resize_annotation(0, 0, r).is_err());
    assert!(doc
        .set_annotation_style(0, 2, &astyle((0, 0, 255), 1.0))
        .is_err());
    assert!(doc
        .update_free_text(0, 2, "x", &FreeTextStyle::default())
        .is_err());
    assert!(doc.set_annotation_contents(0, 2, "x").is_err());

    // (c) Recolorer le surlignage jaune en vert : l'apparence est régénérée et mémorisée.
    let title = rect(72.0, 694.0, 250.0, 724.0);
    let yellow_before = count_in(&doc.render_page(0, 1.0).unwrap(), title, is_yellowish);
    assert!(yellow_before > 500, "{yellow_before}");
    doc.set_annotation_style(0, 3, &astyle((0, 200, 0), 1.0))
        .unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!(count_in(&img, title, is_yellowish), 0);
    assert!(count_in(&img, title, is_green) > 500);
    assert!(
        raw_annots(&doc)[3].get(b"AP").is_ok(),
        "apparence mémorisée"
    );
    assert_eq!(doc.annotations(0).unwrap().len(), 12);

    // (d) Commentaire d'une annotation.
    doc.set_annotation_contents(0, 0, "Vérifié ✓ à 100 %")
        .unwrap();
    assert_eq!(
        doc.annotation_details(0).unwrap()[0].contents.as_deref(),
        Some("Vérifié ✓ à 100 %")
    );
    doc.set_annotation_contents(0, 0, "").unwrap();
    assert_eq!(doc.annotation_details(0).unwrap()[0].contents, None);

    // (e) Supprimer le fil de la note : note, réponse, état et fenêtre contextuelle partent.
    assert_eq!(doc.delete_annotation_thread(0, 4).unwrap(), 4);
    let left = doc.annotation_details(0).unwrap();
    assert_eq!(left.len(), 8);
    assert_eq!(
        left.iter().map(|d| d.subtype.as_str()).collect::<Vec<_>>(),
        [
            "Square",
            "Ink",
            "FreeText",
            "Highlight",
            "Link",
            "Square",
            "Link",
            "Link"
        ]
    );
    assert!(doc.delete_annotation_thread(0, 40).is_err());

    // (f) Annotations de SuniPDF : redimensionner régénère l'apparence, le style la recolore.
    let mut red = astyle((255, 0, 0), 3.0);
    red.fill_rgb = Some((255, 0, 0));
    let i = doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            rect(300.0, 100.0, 380.0, 160.0),
            &red,
            None,
        )
        .unwrap();
    let nr = rect(400.0, 300.0, 520.0, 340.0);
    doc.resize_annotation(0, i, nr).unwrap();
    assert!(near(doc.annotation_details(0).unwrap()[i].rect, nr, 0.01));
    let img = doc.render_page(0, 1.0).unwrap();
    let seen = pixel_box_below(&img, 350.0, is_red).unwrap();
    assert!(
        seen.right >= nr.right - 1.5 && seen.top <= nr.top + 1.5 && seen.left >= 300.0,
        "{seen:?}"
    );
    assert_eq!(count_in(&img, rect(300.0, 100.0, 380.0, 160.0), is_red), 0);
    let mut blue = red.clone();
    blue.rgb = (0, 0, 255);
    blue.fill_rgb = Some((0, 0, 255));
    doc.set_annotation_style(0, i, &blue).unwrap();
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(count_in(&img, nr, is_blue) > 4000);
    assert!(too_small(&mut doc, i));

    // Texte libre : modifier le texte et la couleur.
    let t = doc
        .add_free_text_annotation(
            0,
            rect(100.0, 100.0, 300.0, 130.0),
            "Avant",
            &FreeTextStyle::default(),
            None,
        )
        .unwrap();
    let style = FreeTextStyle {
        size_pt: 18.0,
        rgb: (0, 0, 255),
        fill_rgb: Some((255, 255, 0)),
        border_pt: 1.0,
    };
    doc.update_free_text(0, t, "Après", &style).unwrap();
    let d = &doc.annotation_details(0).unwrap()[t];
    assert_eq!(d.contents.as_deref(), Some("Après"));
    assert_eq!(
        (d.font_size, d.rgb, d.fill_rgb, d.width_pt),
        (
            Some(18.0),
            Some((0, 0, 255)),
            Some((255, 255, 0)),
            Some(1.0)
        )
    );
    doc.set_annotation_contents(0, t, "Encore").unwrap();
    assert_eq!(
        doc.annotation_details(0).unwrap()[t].contents.as_deref(),
        Some("Encore")
    );
    // L'encre de SuniPDF se redimensionne : la boîte suit.
    let ink = doc
        .add_ink_annotation(
            0,
            &[vec![(500.0, 100.0), (540.0, 160.0), (580.0, 100.0)]],
            &astyle((255, 0, 0), 2.0),
            None,
        )
        .unwrap();
    let target = rect(450.0, 80.0, 600.0, 200.0);
    doc.resize_annotation(0, ink, target).unwrap();
    assert!(near(
        doc.annotation_details(0).unwrap()[ink].rect,
        target,
        0.01
    ));
}

/// Un redimensionnement sous 4 pt est refusé.
fn too_small(doc: &mut CoreDocument<'_>, i: usize) -> bool {
    doc.resize_annotation(0, i, rect(10.0, 10.0, 12.0, 12.0))
        .is_err()
}

#[test]
fn replies_and_status_are_invisible_and_threaded() {
    let engine = engine();
    let dir = temp_dir("replies");
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.add_note(0, 300.0, 600.0, "Relire").unwrap();
    let alone = doc.render_page(0, 1.0).unwrap();
    let r1 = doc.add_reply(0, 0, "D'accord", Some("Bob")).unwrap();
    let r2 = doc.add_reply(0, r1, "Merci é", None).unwrap();
    doc.set_review_status(0, 0, ReviewStatus::Accepted, Some("Bob"))
        .unwrap();
    doc.set_review_status(0, 0, ReviewStatus::Rejected, Some("Bob"))
        .unwrap();
    let with = doc.render_page(0, 1.0).unwrap();
    assert!(
        with.rgba == alone.rgba,
        "réponses et états sans trace sur la page"
    );
    let d = doc.annotation_details(0).unwrap();
    assert_eq!(d.len(), 5);
    assert_eq!((d[r1].parent, d[r2].parent), (Some(0), Some(r1)));
    assert_eq!(d[r1].author.as_deref(), Some("Bob"));
    assert_eq!(d[r2].contents.as_deref(), Some("Merci é"));
    assert_eq!(d[3].status, Some(ReviewStatus::Accepted));
    assert_eq!(thread_status(&d, 0), ReviewStatus::Rejected);
    assert_eq!(d[0].replies, 4);
    // Le fil survit à l'enregistrement.
    let out = dir.join("thread.pdf");
    doc.save_as(&out).unwrap();
    let again = engine.open(&out, None).unwrap();
    let d2 = again.annotation_details(0).unwrap();
    assert_eq!(thread_status(&d2, 0), ReviewStatus::Rejected);
    assert!(again.render_page(0, 1.0).unwrap().rgba == alone.rgba);
    // Erreurs : texte vide, parent inexistant ou fenêtre contextuelle.
    assert!(doc.add_reply(0, 0, "  ", None).is_err());
    assert!(doc.add_reply(0, 42, "x", None).is_err());
    assert!(doc
        .set_review_status(0, 42, ReviewStatus::None, None)
        .is_err());
    // Supprimer le fil emporte tout.
    assert_eq!(doc.delete_annotation_thread(0, 0).unwrap(), 5);
    assert!(doc.annotations(0).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn annotation_edits_keep_the_protection_of_encrypted_documents() {
    use sunipdf_core::{security, Protection};
    let engine = engine();
    let dir = temp_dir("enc-annot");
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    let protected = security::protect(
        &input,
        None,
        &Protection {
            user_password: "u".into(),
            owner_password: "o".into(),
            allow_copy: false,
            aes256: true,
            ..Default::default()
        },
    )
    .unwrap();
    let path = dir.join("protected.pdf");
    std::fs::write(&path, &protected).unwrap();
    let mut doc = engine.open(&path, Some("u")).unwrap();
    let s = astyle((255, 0, 0), 2.0);
    doc.add_ink_annotation(0, &[vec![(100.0, 100.0), (200.0, 150.0)]], &s, Some("Zoé"))
        .unwrap();
    doc.add_shape_annotation(
        0,
        AnnotShape::Circle,
        rect(50.0, 50.0, 90.0, 90.0),
        &s,
        None,
    )
    .unwrap();
    doc.add_free_text_annotation(
        0,
        rect(300.0, 300.0, 450.0, 330.0),
        "Texte protégé",
        &FreeTextStyle::default(),
        None,
    )
    .unwrap();
    doc.add_link_annotation(0, rect(10.0, 10.0, 40.0, 30.0), &LinkTarget::Page(1))
        .unwrap();
    doc.move_annotation(0, 0, 5.0, 5.0).unwrap();
    doc.add_reply(0, 2, "Réponse", None).unwrap();
    let snap = doc.snapshot().unwrap();
    assert!(
        snap.windows(8).any(|w| w == b"/Encrypt"),
        "le document reste chiffré en mémoire"
    );
    let out = dir.join("out.pdf");
    doc.save_as(&out).unwrap();
    assert!(matches!(
        engine.open(&out, None),
        Err(EngineError::PasswordRequired)
    ));
    let again = engine.open(&out, Some("u")).unwrap();
    let props = again.properties();
    assert!(props.encrypted && !props.can_copy, "{props:?}");
    assert_eq!(again.annotations(0).unwrap().len(), 5);
    assert_eq!(
        again.annotation_details(0).unwrap()[2].contents.as_deref(),
        Some("Texte protégé")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn annotation_edits_work_on_the_rc4_fixture() {
    let engine = engine();
    let mut doc = engine
        .open(&fixture("encrypted-rc4.pdf"), Some("secret"))
        .unwrap();
    let before = doc.annotations(0).unwrap().len();
    doc.add_shape_annotation(
        0,
        AnnotShape::Square,
        rect(100.0, 100.0, 150.0, 150.0),
        &astyle((255, 0, 0), 2.0),
        None,
    )
    .unwrap();
    assert_eq!(doc.annotations(0).unwrap().len(), before + 1);
    assert!(doc.snapshot().unwrap().windows(8).any(|w| w == b"/Encrypt"));
    // PDFium sait toujours ajouter une note sur ce document.
    doc.add_note(0, 200.0, 200.0, "ok").unwrap();
}

#[test]
fn annotation_snapshot_undo() {
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let snap = doc.snapshot().unwrap();
    doc.add_ink_annotation(
        0,
        &[vec![(100.0, 100.0), (200.0, 200.0)]],
        &astyle((255, 0, 0), 2.0),
        None,
    )
    .unwrap();
    doc.add_reply(0, 0, "x", None).unwrap();
    assert_eq!(doc.annotations(0).unwrap().len(), 2);
    doc.restore(snap).unwrap();
    assert!(doc.annotations(0).unwrap().is_empty());
}

#[test]
fn entries_of_annots_keep_their_indices_when_one_is_unreadable() {
    let engine = engine();
    let dir = temp_dir("null-annot");
    let mut pdf =
        lopdf::Document::load_mem(&std::fs::read(fixture("annotated.pdf")).unwrap()).unwrap();
    let page = *pdf.get_pages().get(&1).unwrap();
    let dict = pdf.get_dictionary_mut(page).unwrap();
    let annots = dict.get_mut(b"Annots").unwrap().as_array_mut().unwrap();
    annots.insert(1, lopdf::Object::Null);
    let path = dir.join("null.pdf");
    pdf.save(&path).unwrap();
    let mut doc = engine.open(&path, None).unwrap();
    let list = doc.annotations(0).unwrap();
    assert_eq!(list.len(), 13);
    assert_eq!((list[1].kind, list[2].kind), ("other", "ink"));
    let d = doc.annotation_details(0).unwrap();
    assert_eq!(d.len(), 13);
    assert_eq!(
        (d[1].subtype.as_str(), d[2].subtype.as_str()),
        ("Unknown", "Ink")
    );
    assert!(!d[1].can_move && !d[1].can_edit_text);
    assert!(doc.move_annotation(0, 1, 1.0, 1.0).is_err());
    doc.move_annotation(0, 2, 5.0, 5.0).unwrap();
    assert_eq!(doc.annotation_details(0).unwrap().len(), 13);
    // Les liens gardent leur indice d'annotation.
    let links = doc.page_links(0).unwrap();
    assert_eq!(
        links.iter().map(|l| l.index).collect::<Vec<_>>(),
        [8, 10, 11]
    );
    let _ = std::fs::remove_dir_all(dir);
}

// --- Caviardage ------------------------------------------------------------------------------

fn find_bytes(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

fn count_occurrences(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

/// Cherche `needle` dans les octets bruts, puis dans chaque objet relu par lopdf : chaînes (y compris
/// hexadécimales, décodées par l'analyseur), noms, flux décompressés et chaînes affichées par les
/// opérateurs de texte (`Tj`, `TJ`, `'`, `"`, concaténées dans l'ordre du flux). ASCII et UTF-16BE.
/// `password` : document chiffré (lopdf le déchiffre au chargement).
fn pdf_contains_pw(bytes: &[u8], needle: &str, password: Option<&str>) -> bool {
    use lopdf::Object;
    let ascii = needle.as_bytes().to_vec();
    let utf16: Vec<u8> = needle
        .encode_utf16()
        .flat_map(|u| u.to_be_bytes())
        .collect();
    let has = |hay: &[u8]| find_bytes(hay, &ascii) || find_bytes(hay, &utf16);
    if has(bytes) {
        return true;
    }
    let doc = match password {
        Some(pw) => {
            lopdf::Document::load_mem_with_options(bytes, lopdf::LoadOptions::with_password(pw))
        }
        None => lopdf::Document::load_mem(bytes),
    }
    .expect("lopdf relit le fichier");
    fn visit(o: &Object, has: &dyn Fn(&[u8]) -> bool) -> bool {
        match o {
            Object::String(s, _) => has(s),
            Object::Name(n) => has(n),
            Object::Array(a) => a.iter().any(|x| visit(x, has)),
            Object::Dictionary(d) => d.iter().any(|(k, v)| has(k) || visit(v, has)),
            Object::Stream(s) => {
                if s.dict.iter().any(|(k, v)| has(k) || visit(v, has)) {
                    return true;
                }
                let data = s
                    .decompressed_content()
                    .unwrap_or_else(|_| s.content.clone());
                if has(&data) {
                    return true;
                }
                let Ok(content) = lopdf::content::Content::decode(&data) else {
                    return false;
                };
                let mut shown = Vec::new();
                for op in &content.operations {
                    if matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                        for operand in &op.operands {
                            match operand {
                                Object::String(s, _) => shown.extend_from_slice(s),
                                Object::Array(a) => {
                                    for x in a {
                                        if let Object::String(s, _) = x {
                                            shown.extend_from_slice(s);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                has(&shown)
            }
            _ => false,
        }
    }
    doc.objects.values().any(|o| visit(o, &has)) || visit(&Object::Dictionary(doc.trailer), &has)
}

fn pdf_contains(bytes: &[u8], needle: &str) -> bool {
    pdf_contains_pw(bytes, needle, None)
}

const REDACT_MARKERS: [&str; 10] = [
    "SECRET-ALPHA-7731",
    "HIDDEN-OCR-5512",
    "IMGSECRET-9F3A",
    "NOTE-SECRET-88",
    "FIELD-SECRET-314",
    "THUMB-SECRET-01",
    "PIECE-SECRET-02",
    "ORPHAN-SECRET-77",
    "META-SECRET-55",
    "ALT-SECRET-66",
];

fn mark(page: usize, l: f32, b: f32, r: f32, t: f32) -> sunipdf_core::RedactMark {
    sunipdf_core::RedactMark {
        page,
        rect: rect(l, b, r, t),
    }
}

#[test]
fn redaction_removes_text_from_extraction_bytes_and_pixels() {
    use sunipdf_core::{RedactOptions, ShapeKind, ShapeStyle};
    let engine = engine();
    let dir = temp_dir("redact-t1");
    let input = std::fs::read(fixture("redact.pdf")).unwrap();
    // 1. Contrôles positifs : chaque marqueur est bien trouvé dans la fixture.
    for m in REDACT_MARKERS {
        assert!(pdf_contains(&input, m), "fixture sans {m}");
    }
    // 2. Contrôle du détecteur : un flux régénéré par PDFium contient encore le secret, et le
    //    détecteur le voit.
    let mut doc = engine.open(&fixture("redact.pdf"), None).unwrap();
    doc.add_shape(
        0,
        ShapeKind::Rectangle,
        rect(500.0, 50.0, 550.0, 80.0),
        &ShapeStyle::default(),
    )
    .unwrap();
    assert!(pdf_contains(&doc.snapshot().unwrap(), "SECRET-ALPHA-7731"));
    drop(doc);

    // 3. Caviardage du secret.
    let mut doc = engine.open(&fixture("redact.pdf"), None).unwrap();
    let marks = doc
        .redact_marks_for_text("SECRET-ALPHA-7731", true)
        .unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].page, 0);
    let zone = marks[0].rect;
    let report = doc.redact(&marks, &RedactOptions::default()).unwrap();
    assert_eq!(report.pages, 1);
    assert!(report.objects_removed >= 1, "{report:?}");
    assert_eq!(report.annotations_removed, 2, "note et son popup");
    assert_eq!(report.pages_rasterized, 0);
    assert!(report.hidden_data_cleaned);

    // 4. Extraction.
    let check_text = |doc: &sunipdf_core::Document<'_>| {
        let text = doc.page_text(0).unwrap();
        assert!(!text.contains("SECRET-ALPHA"), "{text}");
        assert!(text.contains("Public header line") && text.contains("KEEPME-42"));
        assert!(doc.search("SECRET-ALPHA", false).unwrap().is_empty());
    };
    check_text(&doc);

    // 5. Pixels : zone noire, « Name: » conservé en image, le reste blanc.
    let img = doc.render_page(0, 1.0).unwrap();
    let mut y = zone.bottom + 2.0;
    while y < zone.top - 2.0 {
        let mut x = zone.left + 2.0;
        while x < zone.right - 2.0 {
            assert!(close3(px(&img, x, y), [0, 0, 0], 10), "({x}, {y})");
            x += 1.0;
        }
        y += 1.0;
    }
    let name_dark = (72..105)
        .flat_map(|x| (680..690).map(move |y| (x, y)))
        .filter(|&(x, y)| is_dark(px(&img, x as f32, y as f32)))
        .count();
    assert!(
        name_dark > 20,
        "« Name: » visible ({name_dark} pixels sombres)"
    );
    assert!(close3(px(&img, 400.0, 100.0), [255, 255, 255], 2));

    // 6. Octets, dans l'instantané et dans le fichier enregistré.
    let out = dir.join("redacted.pdf");
    doc.save_as(&out).unwrap();
    let saved = std::fs::read(&out).unwrap();
    for bytes in [doc.snapshot().unwrap(), saved.clone()] {
        for gone in [
            "SECRET-ALPHA-7731",
            "NOTE-SECRET-88",
            "THUMB-SECRET-01",
            "PIECE-SECRET-02",
            "ORPHAN-SECRET-77",
            "META-SECRET-55",
            "ALT-SECRET-66",
        ] {
            assert!(!pdf_contains(&bytes, gone), "{gone} encore présent");
        }
        for kept in [
            "KEEP-LINK",
            "KEEP-FIELD-9",
            "FIELD-SECRET-314",
            "IMGSECRET-9F3A",
        ] {
            assert!(
                pdf_contains(&bytes, kept),
                "{kept} non marqué : doit rester"
            );
        }
        assert_eq!(count_occurrences(&bytes, b"%%EOF"), 1);
    }
    // 7. Réouverture du fichier.
    drop(doc);
    let doc = engine.open(&out, None).unwrap();
    check_text(&doc);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn redaction_blacks_out_image_path_and_hidden_text() {
    use sunipdf_core::{ObjectKind, RedactOptions};
    let engine = engine();
    let mut doc = engine.open(&fixture("redact.pdf"), None).unwrap();
    let marks = [
        mark(0, 350.0, 544.0, 478.0, 608.0),
        mark(0, 72.0, 500.0, 172.0, 560.0),
    ];
    let report = doc.redact(&marks, &RedactOptions::default()).unwrap();
    assert!(report.objects_removed >= 3, "{report:?}");
    assert!(!doc.page_text(0).unwrap().contains("HIDDEN-OCR-5512"));
    assert!(
        !doc.page_objects(0)
            .unwrap()
            .iter()
            .any(|o| o.kind == ObjectKind::Path
                && near(o.bounds, rect(72.0, 500.0, 272.0, 560.0), 3.0)),
        "le rectangle rouge n'est plus un tracé"
    );
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(
        close3(px(&img, 414.0, 576.0), [0, 0, 0], 24),
        "moitié haute de l'image noire"
    );
    let g = px(&img, 414.0, 512.0);
    assert!(
        g[1] > 120 && g[0] < 80 && g[2] < 80,
        "moitié basse verte : {g:?}"
    );
    assert!(close3(px(&img, 120.0, 530.0), [0, 0, 0], 4));
    assert!(is_red(px(&img, 220.0, 530.0)));
    let bytes = doc.snapshot().unwrap();
    assert!(!pdf_contains(&bytes, "IMGSECRET-9F3A"));
    assert!(!pdf_contains(&bytes, "HIDDEN-OCR-5512"));
    assert!(pdf_contains(&bytes, "KEEPME-42"));
}

#[test]
fn redaction_removes_form_field_under_mark() {
    use sunipdf_core::RedactOptions;
    let engine = engine();
    let mut doc = engine.open(&fixture("redact.pdf"), None).unwrap();
    let options = RedactOptions {
        clean_hidden_data: false,
        ..Default::default()
    };
    let report = doc
        .redact(&[mark(0, 72.0, 440.0, 272.0, 460.0)], &options)
        .unwrap();
    assert_eq!(report.fields_removed, 1);
    assert!(!report.hidden_data_cleaned);
    let bytes = doc.snapshot().unwrap();
    assert!(!pdf_contains(&bytes, "FIELD-SECRET-314"));
    assert!(pdf_contains(&bytes, "KEEP-FIELD-9"));
    assert!(
        pdf_contains(&bytes, "META-SECRET-55"),
        "métadonnées conservées sans l'option"
    );
    assert!(!doc
        .annotations(0)
        .unwrap()
        .iter()
        .any(|a| a.kind == "widget" && a.rect.bottom < 465.0 && a.rect.left < 280.0));
}

#[test]
fn redaction_rasterize_mode_leaves_only_pixels() {
    use sunipdf_core::{ObjectKind, RedactOptions};
    let engine = engine();
    let mut doc = engine.open(&fixture("redact.pdf"), None).unwrap();
    let options = RedactOptions {
        rasterize_pages: true,
        ..Default::default()
    };
    let report = doc
        .redact(&[mark(0, 500.0, 100.0, 520.0, 120.0)], &options)
        .unwrap();
    assert_eq!(report.pages_rasterized, 1);
    assert!(doc.page_text(0).unwrap().trim().is_empty());
    let objects = doc.page_objects(0).unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].kind, ObjectKind::Image);
    assert!(doc.annotations(0).unwrap().is_empty());
    let bytes = doc.snapshot().unwrap();
    for m in REDACT_MARKERS.iter().chain(&["KEEP-LINK", "KEEP-FIELD-9"]) {
        assert!(!pdf_contains(&bytes, m), "{m}");
    }
    let img = doc.render_page(0, 1.0).unwrap();
    assert!(close3(px(&img, 510.0, 110.0), [0, 0, 0], 4));
    assert!(is_red(px(&img, 172.0, 530.0)));
}

#[test]
fn redaction_respects_page_rotation() {
    use sunipdf_core::{RedactOptions, Rotation};
    let engine = engine();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.set_page_rotation(0, Rotation::R90).unwrap();
    let marks = doc.redact_marks_for_text("SuniPDF page 1", true).unwrap();
    assert_eq!(marks.len(), 1);
    let d = marks[0].rect;
    doc.redact(&marks, &RedactOptions::default()).unwrap();
    assert_eq!(doc.page_rotation(0).unwrap(), Rotation::R90);
    assert!(!doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    let img = doc.render_page(0, 1.0).unwrap();
    assert_eq!((img.width, img.height), (792, 612));
    let (cx, cy) = center(d);
    assert!(
        close3(px(&img, cx, cy), [0, 0, 0], 4),
        "centre de la zone noir"
    );
    // Rectangle bleu (utilisateur 72..272 × 500..650) : affichage x 500..650, y 340..540.
    assert!(is_blue(px(&img, 575.0, 440.0)));
    assert!(!pdf_contains(&doc.snapshot().unwrap(), "SuniPDF page 1"));
    assert!(pdf_contains(&doc.snapshot().unwrap(), "SuniPDF page 2"));
}

#[test]
fn redaction_drops_previous_revisions() {
    use sunipdf_core::RedactOptions;
    let engine = engine();
    let dir = temp_dir("redact-t6");
    let input = std::fs::read(fixture("redact-incremental.pdf")).unwrap();
    assert!(find_bytes(&input, b"OLDREV-SECRET-01"));
    assert_eq!(count_occurrences(&input, b"%%EOF"), 2);
    let mut doc = engine
        .open(&fixture("redact-incremental.pdf"), None)
        .unwrap();
    assert_eq!(doc.page_text(0).unwrap().trim(), "Current revision");
    doc.redact(
        &[mark(0, 400.0, 100.0, 420.0, 120.0)],
        &RedactOptions::default(),
    )
    .unwrap();
    let out = dir.join("out.pdf");
    doc.save_as(&out).unwrap();
    let saved = std::fs::read(&out).unwrap();
    assert!(!pdf_contains(&saved, "OLDREV-SECRET-01"));
    assert_eq!(count_occurrences(&saved, b"%%EOF"), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn redaction_of_encrypted_document_and_errors() {
    use sunipdf_core::{security, Protection, RedactOptions};
    let engine = engine();
    let dir = temp_dir("redact-t7");
    let input = std::fs::read(fixture("two-pages.pdf")).unwrap();
    let p = Protection {
        user_password: "user".into(),
        owner_password: "owner".into(),
        ..Default::default()
    };
    let protected = dir.join("protected.pdf");
    std::fs::write(&protected, security::protect(&input, None, &p).unwrap()).unwrap();
    let mut doc = engine.open(&protected, Some("user")).unwrap();
    let marks = doc.redact_marks_for_text("SuniPDF page 1", true).unwrap();
    doc.redact(&marks, &RedactOptions::default()).unwrap();
    assert!(doc.is_encrypted(), "la protection est conservée");
    let out = dir.join("out.pdf");
    doc.save_as(&out).unwrap();
    drop(doc);
    assert!(matches!(
        engine.open(&out, None),
        Err(EngineError::PasswordRequired)
    ));
    let doc = engine.open(&out, Some("user")).unwrap();
    assert!(!doc.page_text(0).unwrap().contains("SuniPDF page 1"));
    let saved = std::fs::read(&out).unwrap();
    assert!(!pdf_contains_pw(&saved, "SuniPDF page 1", Some("user")));
    assert!(pdf_contains_pw(&saved, "SuniPDF page 2", Some("user")));
    drop(doc);

    // Erreurs : rien n'est modifié.
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    let before = doc.page_text(0).unwrap();
    let o = RedactOptions::default();
    assert!(doc.redact(&[], &o).is_err());
    assert!(matches!(
        doc.redact(&[mark(9, 0.0, 0.0, 10.0, 10.0)], &o),
        Err(EngineError::PageOutOfRange(9))
    ));
    assert!(doc.redact(&[mark(0, 10.0, 10.0, 10.0, 50.0)], &o).is_err());
    assert!(doc
        .redact(&[mark(0, f32::NAN, 10.0, 20.0, 50.0)], &o)
        .is_err());
    assert_eq!(doc.page_text(0).unwrap(), before);
    assert_eq!(
        EngineError::RedactionIncomplete.code(),
        "redaction_incomplete"
    );
    let _ = std::fs::remove_dir_all(dir);
}
