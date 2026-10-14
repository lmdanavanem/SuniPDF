# Plan d'implémentation : tranche « qa-hardening » (SUNI-016, en partie)

Ce plan couvre quatre chantiers : les entrées hostiles et cas limites, un banc de mesure reproductible, les boîtes
« Paramètres » et « Raccourcis clavier », et les bases de l'accessibilité.

**État.** Plan rédigé le 2026-10-14 vers 10 h 50 par l'agent qa-performance. **Aucune commande cargo n'a été exécutée**
(consigne de lecture seule : une autre session compile). Rien de ce qui suit n'a donc été compilé. Chaque API citée a
été lue dans les sources locales, avec son fichier et sa ligne. Ce qui n'a pas pu être vérifié ici est marqué ⚠, et
chaque point ⚠ est tranché par un test nommé.

**Le dépôt bouge pendant la lecture.** Plusieurs fichiers ont changé pendant que je les lisais, d'où des numéros de
ligne décalés. Exemples : `lib.rs`, `error.rs`, `pages.rs`, `security.rs` et `convert.rs` modifiés à 10 h 41
(`EngineError::AlreadyExists`, `Document::set_path`, `ensure_absent`, `split_to`), et `app.rs`, `docinfo.rs` et
`tests/engine.rs` (1 262 lignes) modifiés eux aussi. Les numéros donnés ici ont été relevés entre 10 h 45 et 10 h 55.
**Repérez-vous d'abord au symbole cité**, la ligne ne sert que d'indication.

| Sigle | Chemin |
|---|---|
| `PR/` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src/` |
| `EG/` | `…/egui-0.36.2/src/` |
| `AK/` | `…/accesskit-0.24.1/src/` (version utilisée par egui 0.36.2, `EG/../Cargo.toml` l. 82-83) |
| `AKW/` | `…/accesskit_windows-0.32.1/src/` (version de `Cargo.lock`) |
| `WS/` | `…/windows-sys-0.61.2/src/Windows/Win32/` |
| (sans sigle) | chemin relatif à `C:/dev_suniworks/SuniPDF/` |

---

## 0. Constats vérifiés (lecture seule)

| Fait | Source | Conséquence |
|---|---|---|
| `Pdfium::load_pdf_from_file` fait un `File::open` puis `FPDF_LoadCustomDocument` et **garde le lecteur** pendant toute la vie du document. PDFium lit ensuite les objets à la demande. | `PR/pdfium.rs:371-377`, `410-432` | Un document ouvert dépend du fichier sur disque : si un autre programme le réécrit, les pages pas encore chargées, les instantanés et l'enregistrement relisent des octets étrangers (bug B7). |
| `Engine::wrap` lit les tailles par `pages().page_sizes()` (`FPDF_GetPageSizeByIndexF`). C'est la taille **affichée**, rotation comprise : le test existant `rotate_delete_and_save_atomically` obtient 792 × 612 après une rotation R90. | `crates/sunipdf-core/src/lib.rs:164-195`, `PR/pdf/document/pages.rs:151-187` | `PageSize` est dans l'espace affiché. |
| Le texte, les annotations et les objets renvoyés par PDFium sont en **espace utilisateur** PDF : non pivoté, et non recentré sur l'origine de la boîte visible. | Doc des liaisons `PR/bindings.rs` (« PDF user space », l. 4974-5449, 6104) | Mélanger taille affichée et coordonnées utilisateur est faux dès que `/Rotate` ≠ 0 ou que la boîte ne commence pas en (0, 0) (bugs B4, B5, B6). |
| L'interface convertit écran ↔ PDF avec `page_h_pt - y` sans tenir compte de la rotation ni de l'origine. | `apps/sunipdf-desktop/src/annotate.rs:99-104` (`to_pdf`), `:164-169` (`paint_selection`), `app.rs:1480` (`paint_matches`), `editor.rs:540-545`, `:573-575` | Toutes ces conversions passeront par un type unique (étape C). |
| `stamp_pages` calcule la position du tampon dans la taille **affichée** mais déplace l'objet en espace **utilisateur**. | `crates/sunipdf-core/src/stamp.rs:120-135` | Tampons hors page ou couchés (B5). |
| `PdfPageBoundaries::bounding()` appelle `FPDF_GetPageBoundingBox`, qui renvoie l'intersection MediaBox / CropBox. | `PR/pdf/document/page/boundaries.rs:248-263`, `PR/bindings.rs:2497-2507` | C'est la boîte qu'utilise PDFium pour l'affichage. L'héritage depuis les nœuds `/Pages` est géré côté PDFium ⚠ (vérifié par la page 4 de la fixture `rotated-offset.pdf`). |
| `PdfPage::rotation()` lit `/Rotate` ; `rotate_counter_clockwise_degrees` existe sur les objets texte et image. | `PR/pdf/document/page.rs:292-296`, `PR/pdf/transform.rs:197-219`, `PR/pdf/document/page/object/text.rs:517-523` | Permet de redresser le texte et les images ajoutés sur une page pivotée. |
| ⚠ `FPDF_LoadPage` analyse tout le contenu de la page. Ce point vient de la connaissance de l'implémentation PDFium, dont le source n'est pas sur ce poste. | Aucune source locale | La géométrie se calcule **au rendu**, page par page, et non pour toutes les pages à l'ouverture (coût sur 500 pages). |
| Le nom accessible d'un `egui::Button` est son texte. Pour un bouton à icône seule, c'est donc un glyphe à usage privé (U+E…). | `EG/widgets/button.rs:391-402` | Narrateur lit des caractères illisibles (B10). |
| `Context::accesskit_node_builder` (public), `Response::labelled_by`, `Response::widget_info` et `Context::text_edit_focused` existent. | `EG/context.rs:3684-3690`, `EG/response.rs:869-895`, `:985-991`, `EG/context.rs:2991-2997` | Outils de l'étape F. |
| Tab et Maj+Tab font circuler le focus ; les flèches seules le déplacent dans leur direction ; Échap le retire. Entrée ou Espace sur un widget focalisé équivaut à un clic. Une boîte modale retient le focus. | `EG/memory/mod.rs:591-601`, `EG/context.rs:1468-1474`, `EG/memory/mod.rs:956-977` | Les dialogues sont déjà piégés au clavier. Dans la fenêtre principale, les flèches entrent en conflit avec la navigation de page (étape D). |
| `consume_key` ignore une touche **Maj** en trop : `(CTRL \| SHIFT).matches_logically(CTRL)` est vrai. | `EG/input_state/mod.rs:691-711`, `EG/data/input/modifiers.rs:199-220` | Ctrl+Maj+D et Ctrl+Maj+Z sont capturés par Ctrl+D et Ctrl+Z, déclarés plus haut (B1, B2). |
| Le garde de `handle_input` bloque tout raccourci sans Ctrl dès qu'un widget, quel qu'il soit, a le focus. | `app.rs:623` et `:656` (`ctx.memory(\|m\| m.focused().is_some())`) | Après une seule pression sur Tab, F4, Page suiv., Suppr, Ctrl+Z, etc. ne répondent plus (B3). |
| `write_atomic` écrit `nom.PID.tmp` avec `fs::write`, sans `sync_all`, puis appelle `rename`. Aucune vérification de l'attribut lecture seule. | `crates/sunipdf-core/src/pages.rs:62-79` | B8. Le comportement de `std::fs::rename` sur une cible en lecture seule n'est pas vérifiable ici ⚠ (pas de `rust-src` dans les toolchains) : le refus sera donc explicite. |
| `K32GetProcessMemoryInfo` et `PROCESS_MEMORY_COUNTERS` (10 champs) sont exportés par `kernel32.dll`. | `WS/System/ProcessStatus/mod.rs:47`, `:103-114`, `WS/System/Threading/mod.rs:133` | Mesure mémoire du banc par une petite FFI, sans nouvelle dépendance. |
| accesskit_windows 0.32.1 émet `UIA_LiveRegionChangedEventId` pour un nœud nommé et « live » (Polite ou Assertive). Le rôle `Dialog` est reconnu. | `AKW/adapter.rs:225-231`, `:272-279`, `AKW/node.rs:128`, `:288` | Les erreurs peuvent être annoncées et les dialogues identifiés. |
| Machine de référence | `Get-CimInstance` | AMD Ryzen 7 6800HS Creator Edition, 8 cœurs / 16 threads, 13,7 Go, Windows 11 Pro 26300, SSD Samsung MZVL2512HCJQ. |
| Empreintes actuelles des fixtures | `sha256sum` | `two-pages.pdf` dd186d07…14da5c, `encrypted-rc4.pdf` 554e55c3…b389c1. La régénération ne doit **pas** les changer. |

---

## 1. Bugs trouvés à la lecture

Seuls B1 à B3 et B10 sont certains par lecture directe des sources. B4 à B9 sont très probables, et les tests de
l'étape indiquée les confirment ou les infirment.

| # | Défaut | Reproduction | Agent propriétaire | Corrigé à l'étape |
|---|---|---|---|---|
| B1 | Ctrl+Maj+D ouvre « Propriétés » au lieu de dupliquer l'objet sélectionné : `key(COMMAND, D, Properties)` (`app.rs:759`) précède `key(COMMAND\|SHIFT, D, DuplicateObject)` (`:774-778`). | Outil E, cliquer un objet, Ctrl+Maj+D | ui-ux | D |
| B2 | Ctrl+Maj+Z annule au lieu de rétablir : Ctrl+Z (`app.rs:781`) précède Ctrl+Maj+Z (`:783`). | Une modification, Ctrl+Z, Ctrl+Maj+Z | ui-ux | D |
| B3 | Une fois un bouton focalisé par Tab, seuls Ctrl+O, Ctrl+S, Ctrl+K et Ctrl+F fonctionnent (`app.rs:623`). | Tab une fois, puis F4 ou Page suiv. | ui-ux | D |
| B4 | Sur une page avec `/Rotate` ≠ 0 ou une boîte d'origine ≠ (0, 0), la sélection de texte, la surbrillance de recherche, les annotations, le choix et le déplacement d'objets et l'aperçu des formes tombent au mauvais endroit. | `two-pages.pdf`, Ctrl+R sur la page 1, puis glisser sur « SuniPDF page 1 » : rien n'est sélectionné | rendering + pdf-editor | B, C |
| B5 | Tampons et numéros de page hors de la page visible, ou verticaux, sur une page pivotée ou décalée (`stamp.rs:120-135`). | Ctrl+R, puis « Numéroter les pages » | pdf-editor | B |
| B6 | Texte et image ajoutés couchés à l'écran sur une page pivotée (`edit.rs:216`, `:484`). | Ctrl+R, outil T, clic | pdf-editor | B |
| B7 | Le document ouvert relit son fichier à la demande. S'il est réécrit par un autre programme, le texte et le rendu des pages pas encore chargées, ainsi que l'enregistrement, utilisent les nouveaux octets. | Test `open_document_ignores_later_changes_on_disk` | pdf-engine | A |
| B8 | `write_atomic` : (a) pas de `sync_all` avant `rename` (coupure de courant) ; (b) un nom de 246 caractères ou plus, suffixé de `.PID.tmp`, dépasse la limite NTFS de 255 et l'enregistrement échoue ; (c) une cible en lecture seule risque d'être remplacée sans avertissement ⚠. | Tests `unicode_and_long_paths_round_trip` et `save_never_overwrites_read_only_or_locked_targets` | pdf-engine | A |
| B9 | Un PDF sans page s'ouvre sans erreur : zone de document vide et « Page 1 sur 0 » (`app.rs` `document_area`, `page_sizes.get` → `return` ; `shell.rs:1146-1152`). | Test `hostile_inputs_fail_with_typed_errors` | pdf-engine | A |
| B10 | Une vingtaine de boutons à icône seule sont annoncés par leur glyphe. Les rangées d'outils (`shell.rs:189` `tool_row`), les onglets (`:393`), les lignes de la palette (`:1100`) et la pastille « Rechercher outils et commandes » (`:566`) n'ont ni rôle ni nom. | Narrateur (Ctrl+Win+Entrée), puis Tab | forms-accessibility | F |

---

## 2. Fixtures (`scripts/make_fixtures.py`, agent pdf-engine)

Sans dépendance externe, dans le style du script actuel. Le script lit `two-pages.pdf` après l'avoir écrit.
Le 0 octet et le « faux PDF » sont créés par les tests eux-mêmes, sans fichier versionné.

```python
def encrypted(path, user_pw=b"secret", owner_pw=None, P=-1):
    # R2 / RC4 40 bits. owner_pw None = propriétaire identique à l'utilisateur (fixture existante inchangée).
    doc_id = hashlib.md5(b"sunipdf-encrypted-fixture").digest()
    padded_user = (user_pw + PAD)[:32]
    padded_owner = ((owner_pw if owner_pw is not None else user_pw) + PAD)[:32]
    okey = hashlib.md5(padded_owner).digest()[:5]
    O = rc4(okey, padded_user)
    key = hashlib.md5(padded_user + O + struct.pack("<i", P) + doc_id).digest()[:5]
    U = rc4(key, PAD)
    # … suite identique (objkey, enc_for, content, hexs, objs, build) …

def zero_pages(path):
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [] /Count 0 >>"]
    open(path, "wb").write(build(objs))

def truncated(path, src):
    data = open(src, "rb").read()
    # Coupe au milieu du flux de contenu de la page 2 (octet 564 avec la fixture actuelle).
    open(path, "wb").write(data[: len(data) * 37 // 100])

def broken_xref(path, src):
    data = open(src, "rb").read()
    i = data.rindex(b"startxref\n") + len(b"startxref\n")
    j = data.index(b"\n", i)
    # startxref pointe 40 octets trop loin : PDFium doit reconstruire la table.
    open(path, "wb").write(data[:i] + str(int(data[i:j]) + 40).encode() + data[j:])

def rotated_offset(path):
    def content(x, y, tx, ty, label):
        return stream(f"0 0 1 rg {x} {y} 80 40 re f\nBT /F1 24 Tf {tx} {ty} Td ({label}) Tj ET\n".encode())
    res = b" /Resources << /Font << /F1 9 0 R >> >> >>"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 5 0 R 7 0 R 10 0 R] /Count 4 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 /Contents 4 0 R" + res,
        content(100, 600, 300, 300, "MARK1"),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [100 200 712 992] /Contents 6 0 R" + res,
        content(200, 800, 400, 500, "MARK2"),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [-50 -50 562 742] /CropBox [0 0 500 700] /Rotate -90 /Contents 8 0 R" + res,
        content(100, 600, 200, 300, "MARK3"),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        # Page 4 : MediaBox et Rotate hérités d'un nœud /Pages intermédiaire.
        b"<< /Type /Pages /Parent 2 0 R /Kids [11 0 R] /Count 1 /MediaBox [0 0 300 400] /Rotate 180 >>",
        b"<< /Type /Page /Parent 10 0 R /Contents 12 0 R" + res,
        content(50, 300, 50, 100, "MARK4"),
    ]
    open(path, "wb").write(build(objs))

if __name__ == "__main__":
    out = sys.argv[1]
    two = os.path.join(out, "two-pages.pdf")
    two_pages(two)
    encrypted(os.path.join(out, "encrypted-rc4.pdf"))
    encrypted(os.path.join(out, "owner-only-rc4.pdf"), user_pw=b"", owner_pw=b"owner", P=-5)  # impression interdite
    zero_pages(os.path.join(out, "zero-pages.pdf"))
    truncated(os.path.join(out, "truncated.pdf"), two)
    broken_xref(os.path.join(out, "broken-xref.pdf"), two)
    rotated_offset(os.path.join(out, "rotated-offset.pdf"))
    print("ok")
```

Valeurs attendues pour `rotated-offset.pdf`. Tous les rectangles bleus font 80 × 40 pt.

| Page | Boîte visible (utilisateur) | Rotation | Taille affichée | Centre du rectangle bleu (utilisateur → affiché) | Pixel à l'échelle 1 (x, ligne depuis le haut) |
|---|---|---|---|---|---|
| 0 | 0 0 612 792 | R90 | 792 × 612 | (140, 620) → (620, 472) | (620, 140) |
| 1 | 100 200 712 992 | R0 | 612 × 792 | (240, 820) → (140, 620) | (140, 172) |
| 2 | 0 0 500 700 (CropBox ∩ MediaBox) | R270 (`/Rotate -90`) | 700 × 500 | (140, 620) → (80, 140) | (80, 360) |
| 3 | 0 0 300 400 (héritée) | R180 (héritée) | 300 × 400 | (90, 320) → (210, 80) | (210, 320) |

Après `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`, vérifiez que `sha256sum` donne les
**mêmes empreintes** que ci-dessus pour `two-pages.pdf` et `encrypted-rc4.pdf`.

---

## 3. Étape A : moteur, entrées hostiles et enregistrement sûr (agent pdf-engine)

### A1. Ouverture en mémoire, refus des fichiers vides et sans page (B7, B9) : `crates/sunipdf-core/src/lib.rs`

```rust
    /// Ouvre un document. Le fichier est lu entièrement : il n'est pas gardé ouvert, et une réécriture
    /// par un autre programme ne peut ni corrompre l'affichage ni l'enregistrement (coût : sa taille en mémoire).
    pub fn open(&self, path: &Path, password: Option<&str>) -> Result<Document<'_>, EngineError> {
        self.open_bytes(std::fs::read(path)?, password, path)
    }

    pub fn open_bytes(&self, bytes: Vec<u8>, password: Option<&str>, path: &Path) -> Result<Document<'_>, EngineError> {
        if bytes.is_empty() {
            return Err(EngineError::InvalidFormat);
        }
        let inner = self.pdfium.load_pdf_from_byte_vec(bytes, password)?;
        self.wrap(inner, path, password)
    }
```

Dans `wrap` (l. 164), en tête : `if inner.pages().is_empty() { return Err(EngineError::InvalidFormat); }`.
Un document sans page n'offre rien à afficher ni à éditer, et l'interface suppose au moins une page.
`PdfPages::is_empty` est public (`PR/pdf/document/pages.rs:103`). `merge`, `split` et `insert_pages_from`
gardent `load_pdf_from_file` : leurs sources sont fermées en fin d'appel.

### A2. `write_atomic` (B8) : `crates/sunipdf-core/src/pages.rs:62-79`

```rust
/// Écrit `bytes` dans `path` sans jamais laisser de fichier partiel : fichier temporaire voisin, vidé sur disque,
/// puis `rename`. Refuse une cible en lecture seule (aucun écrasement silencieux).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    if std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly()) {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "target is read-only"));
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    // Nom court et unique : un nom de 255 caractères suffixé dépasserait la limite NTFS.
    let tmp = path.with_file_name(format!(
        ".sunipdf-{}-{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let result = std::fs::File::create(&tmp)
        .and_then(|mut f| {
            f.write_all(bytes)?;
            f.sync_all()
        })
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
```

Le test unitaire existant `write_atomic_replaces_and_leaves_no_temp_file` (`pages.rs:354`) reste valable.
L'interface affiche déjà `error.save_failed` (« … Le fichier original n'a pas été modifié. »), et ce message reste exact.

### A3. Tests d'intégration, ajoutés **à la fin** de `crates/sunipdf-core/tests/engine.rs` (agent qa-performance)

Ils réutilisent le harnais `engine()` et `fixture()` (l. 6, 34). Les imports nécessaires sont `std::path::Path`,
`sunipdf_core::{Engine, EngineError, Rotation}`. Le fichier grossit en ce moment : si une autre session y a déjà
ajouté un utilitaire équivalent à `temp` ou `no_temp_file`, réutiliser l'existant plutôt que d'en créer un second.

```rust
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sunipdf-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn no_temp_file(dir: &Path) -> bool {
    std::fs::read_dir(dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp"))
}

/// Ouvre un fichier hostile : jamais de panique ; une erreur typée ou un document parcourable.
fn open_hostile(engine: &Engine, path: &Path) -> Result<usize, EngineError> {
    let doc = engine.open(path, None)?;
    for page in 0..doc.page_count() {
        let _ = doc.render_page(page, 0.5); // erreur typée acceptée, panique non
        let _ = doc.page_text(page);
    }
    Ok(doc.page_count())
}

#[test]
fn hostile_inputs_fail_with_typed_errors() {
    let engine = engine();
    let dir = temp("hostile");
    let empty = dir.join("vide.pdf");
    std::fs::write(&empty, b"").unwrap();
    assert!(matches!(engine.open(&empty, None), Err(EngineError::InvalidFormat)));
    assert!(matches!(engine.open_bytes(Vec::new(), None, &empty), Err(EngineError::InvalidFormat)));
    let html = dir.join("page.pdf");
    std::fs::write(&html, b"<!doctype html><title>x</title>").unwrap();
    assert!(matches!(engine.open(&html, None), Err(EngineError::InvalidFormat)));
    let png = dir.join("image.pdf");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon/sunipdf-64.png"), &png).unwrap();
    assert!(matches!(engine.open(&png, None), Err(EngineError::InvalidFormat)));
    assert!(matches!(engine.open(&fixture("zero-pages.pdf"), None), Err(EngineError::InvalidFormat)));
    match open_hostile(&engine, &fixture("truncated.pdf")) {
        Ok(pages) => assert!(pages >= 1),
        Err(e) => assert!(matches!(e, EngineError::InvalidFormat), "{e:?}"),
    }
    // ⚠ hypothèse : PDFium reconstruit une table xref décalée.
    let doc = engine.open(&fixture("broken-xref.pdf"), None).unwrap();
    assert_eq!(doc.page_count(), 2);
    assert!(doc.page_text(1).unwrap().contains("SuniPDF page 2"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unicode_and_long_paths_round_trip() {
    let engine = engine();
    let base = temp("chemins");
    let unicode = base.join("Données 文書 📄").join("été ✓.pdf");
    // Chemin > 600 caractères ; nom de 250 caractères (limite NTFS : 255).
    let long = base.join("a".repeat(120)).join("b".repeat(120)).join(format!("{}.pdf", "c".repeat(246)));
    for path in [unicode, long] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(fixture("two-pages.pdf"), &path).unwrap();
        let mut doc = engine.open(&path, None).unwrap();
        doc.set_page_rotation(0, Rotation::R90).unwrap();
        doc.save_as(&path).unwrap();
        drop(doc);
        assert_eq!(engine.open(&path, None).unwrap().page_rotation(0).unwrap(), Rotation::R90);
        assert!(no_temp_file(path.parent().unwrap()));
    }
    let _ = std::fs::remove_dir_all(base);
}

#[test]
#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)] // Windows : retire l'attribut lecture seule.
fn save_never_overwrites_read_only_or_locked_targets() {
    use std::os::windows::fs::OpenOptionsExt;
    let engine = engine();
    let dir = temp("cibles");
    let original = std::fs::read(fixture("two-pages.pdf")).unwrap();
    let mut doc = engine.open(&fixture("two-pages.pdf"), None).unwrap();
    doc.set_page_rotation(0, Rotation::R90).unwrap();

    let ro = dir.join("lecture-seule.pdf");
    std::fs::write(&ro, &original).unwrap();
    let mut perm = std::fs::metadata(&ro).unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(&ro, perm.clone()).unwrap();
    match doc.save_as(&ro) {
        Err(EngineError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
        other => panic!("attendu un refus, obtenu {other:?}"),
    }
    assert_eq!(std::fs::read(&ro).unwrap(), original);
    perm.set_readonly(false);
    std::fs::set_permissions(&ro, perm).unwrap();

    // Ouvert sans partage par un autre programme : le fichier temporaire est écrit, le renommage échoue,
    // l'original reste intact. C'est aussi le scénario « enregistrement interrompu ».
    let locked = dir.join("verrouille.pdf");
    std::fs::write(&locked, &original).unwrap();
    let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();
    assert!(matches!(doc.save_as(&locked), Err(EngineError::Io(_))));
    assert!(matches!(engine.open(&locked, None), Err(EngineError::Io(_))));
    drop(lock);
    assert_eq!(std::fs::read(&locked).unwrap(), original);
    assert!(no_temp_file(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn open_document_ignores_later_changes_on_disk() {
    let engine = engine();
    let dir = temp("externe");
    let path = dir.join("doc.pdf");
    std::fs::copy(fixture("two-pages.pdf"), &path).unwrap();
    let doc = engine.open(&path, None).unwrap();
    std::fs::write(&path, b"%PDF-1.4 reecrit par un autre programme").unwrap();
    assert!(doc.page_text(1).unwrap().contains("SuniPDF page 2"));
    assert!(doc.render_page(1, 0.5).is_ok());
    let copy = dir.join("copie.pdf");
    doc.save_as(&copy).unwrap();
    assert_eq!(engine.open(&copy, None).unwrap().page_count(), 2);
    drop(doc);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn empty_user_password_opens_without_prompt() {
    let engine = engine();
    for password in [None, Some(""), Some("owner")] {
        let doc = engine.open(&fixture("owner-only-rc4.pdf"), password).unwrap();
        assert!(doc.page_text(0).unwrap().contains("Encrypted page"), "{password:?}");
    }
    let props = engine.open(&fixture("owner-only-rc4.pdf"), None).unwrap().properties();
    assert!(props.encrypted);
    assert!(!props.can_print, "P = -5 : impression interdite");
    assert!(matches!(engine.open(&fixture("encrypted-rc4.pdf"), Some("")), Err(EngineError::PasswordRequired)));
}
```

Résultats attendus **avant** A1 et A2. Ce sont des hypothèses, à noter telles qu'observées :

- `open_document_ignores_later_changes_on_disk` échoue (B7).
- `unicode_and_long_paths_round_trip` échoue sur le nom de 250 caractères (B8 b).
- `hostile_inputs_fail_with_typed_errors` échoue sur `zero-pages.pdf`, et peut-être sur le fichier vide si PDFium
  renvoie `FileError` (→ `Io`).
- `save_never_overwrites…` échoue si `rename` remplace une cible en lecture seule.

Si `empty_user_password…` échoue sur `encrypted` ou `can_print`, le défaut est dans `Document::properties` (lecture
lopdf, `security.rs:304`) et non dans le test. Il faut alors le rapporter à security-signatures. Une autre session
modifie `security.rs` en ce moment.

---

## 4. Étape B : géométrie des pages (agents rendering et pdf-editor)

### B1. Type `PageGeometry` dans `crates/sunipdf-core/src/pages.rs`, à côté de `Rotation`

Rendre `Rotation::from_pdfium` (l. 52) `pub(crate)`. Ajouter `PointRect` aux imports :
`use crate::{Document, Engine, EngineError, PageSize, PointRect};`.

```rust
/// Passage entre l'espace utilisateur PDF (texte, annotations et objets renvoyés par PDFium) et l'espace
/// **affiché** : origine en bas à gauche de la page telle qu'on la voit, boîte visible (CropBox ∩ MediaBox)
/// ramenée à l'origine, puis `/Rotate` appliqué dans le sens horaire. `PageSize` est la taille de cet espace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry {
    /// Boîte visible, en espace utilisateur.
    pub bbox: PointRect,
    pub rotation: Rotation,
}

impl Default for PageGeometry {
    /// Identité : page non pivotée, boîte à l'origine.
    fn default() -> Self {
        Self {
            bbox: PointRect { left: 0.0, bottom: 0.0, right: 0.0, top: 0.0 },
            rotation: Rotation::R0,
        }
    }
}

impl PageGeometry {
    pub(crate) fn of(page: &PdfPage<'_>) -> Result<Self, EngineError> {
        // FPDF_GetPageBoundingBox : intersection MediaBox / CropBox telle que PDFium l'affiche.
        let b = page.boundaries().bounding()?.bounds;
        Ok(Self {
            bbox: PointRect {
                left: b.left().value,
                bottom: b.bottom().value,
                right: b.right().value,
                top: b.top().value,
            },
            rotation: Rotation::from_pdfium(page.rotation()?),
        })
    }

    /// Point utilisateur → point affiché.
    pub fn to_display(&self, x: f32, y: f32) -> (f32, f32) {
        let b = &self.bbox;
        match self.rotation {
            Rotation::R0 => (x - b.left, y - b.bottom),
            Rotation::R90 => (y - b.bottom, b.right - x),
            Rotation::R180 => (b.right - x, b.top - y),
            Rotation::R270 => (b.top - y, x - b.left),
        }
    }

    /// Point affiché → point utilisateur (inverse exact de `to_display`).
    pub fn to_user(&self, x: f32, y: f32) -> (f32, f32) {
        let b = &self.bbox;
        match self.rotation {
            Rotation::R0 => (x + b.left, y + b.bottom),
            Rotation::R90 => (b.right - y, x + b.bottom),
            Rotation::R180 => (b.right - x, b.top - y),
            Rotation::R270 => (y + b.left, b.top - x),
        }
    }

    /// Rectangle utilisateur → rectangle affiché (un quart de tour garde les rectangles droits).
    pub fn rect_to_display(&self, r: PointRect) -> PointRect {
        let (x0, y0) = self.to_display(r.left, r.bottom);
        let (x1, y1) = self.to_display(r.right, r.top);
        PointRect { left: x0.min(x1), bottom: y0.min(y1), right: x0.max(x1), top: y0.max(y1) }
    }
}

impl Document<'_> {
    /// Géométrie d'affichage d'une page (boîte visible et rotation).
    pub fn page_geometry(&self, page_index: usize) -> Result<PageGeometry, EngineError> {
        PageGeometry::of(&self.page_handle(page_index)?)
    }
}
```

La convention R90 est le sens horaire, ce qui correspond à la valeur 1 de `FPDFPage_GetRotation`
(`PR/pdf/document/page.rs:292`). Elle est vérifiée par les pixels du test B4.

### B2. La géométrie accompagne chaque rendu : `lib.rs`

- Ligne 27 : `pub use pages::{write_atomic, PageGeometry, Rotation};`. Conserver les autres réexportations ajoutées
  entre-temps (par exemple `ensure_absent`).
- `RenderedPage` (l. 47) reçoit `/// Géométrie de la page rendue (conversions écran ↔ espace utilisateur). pub geometry: PageGeometry,`.
- `render_page` (l. 234) : juste après `let page = self.inner.pages().get(index)?;`, ajouter
  `let geometry = PageGeometry::of(&page)?;`. Le champ `geometry` va ensuite dans `Ok(RenderedPage { … })` (l. 252).
- `cache.rs:94-100` (test) : ajouter `geometry: crate::PageGeometry::default(),`.

Ce choix ajoute deux appels FFI par rendu, sans aucune analyse de page en plus : la page est déjà chargée pour le rendu.

### B3. Texte et image ajoutés droits à l'écran (B6) : `crates/sunipdf-core/src/edit.rs`

Importer `Rotation` (`use crate::{Document, EngineError, PointRect, Rotation};`).

Dans `add_text` (l. 196), juste après `let mut page = self.page_handle(page_index)?;`, puis juste avant
`object.translate(…)` (l. 216) :

```rust
        let turn = Rotation::from_pdfium(page.rotation()?).degrees() as f32;
        // …
        if turn != 0.0 {
            // Page pivotée (/Rotate) : contre-rotation pour que le texte soit horizontal à l'écran.
            object.rotate_counter_clockwise_degrees(turn)?;
        }
```

Faire de même dans `add_image` (l. 463), avant `object.translate` (l. 484). L'objet est créé à l'origine : la
rotation autour de l'origine laisse son coin bas-gauche local au point d'ancrage `(x, y)`. Les fixtures existantes
sont toutes en R0, donc les tests actuels ne changent pas.

### B4. Tampons (B5) : `crates/sunipdf-core/src/stamp.rs:116-138`

```rust
            let size = self.page_size(page)?; // taille affichée
            let geo = self.page_geometry(page)?;
            self.add_text(page, 0.0, 0.0, &text, &stamp.style)?; // déjà redressé (B3)
            let index = self.page_objects(page)?.len() - 1;
            let b = geo.rect_to_display(self.object_info(page, index)?.bounds);
            let (w, h) = (b.right - b.left, b.top - b.bottom);
            let m = stamp.margin_pt;
            let (x, y) = match stamp.position { /* inchangé */ };
            // Translation calculée à l'écran puis ramenée en espace utilisateur.
            let (from, to) = (geo.to_user(b.left, b.bottom), geo.to_user(x, y));
            self.move_object(page, index, to.0 - from.0, to.1 - from.1)?;
            if stamp.angle != 0.0 {
                self.rotate_object(page, index, -stamp.angle)?; // une rotation de page conserve le sens
            }
```

### B5. Tests (agent qa-performance)

**Test unitaire**, dans `pages.rs`, module `tests` (l. 341) :

```rust
    #[test]
    fn page_geometry_round_trips_and_maps_corners() {
        let bbox = PointRect { left: 100.0, bottom: 200.0, right: 712.0, top: 992.0 };
        for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
            let g = PageGeometry { bbox, rotation };
            for (x, y) in [(100.0, 200.0), (712.0, 992.0), (300.5, 450.25)] {
                let (dx, dy) = g.to_display(x, y);
                assert_eq!(g.to_user(dx, dy), (x, y), "{rotation:?}");
            }
        }
        // Quart de tour horaire : le coin haut-gauche de la page passe en haut à droite.
        let g = PageGeometry { bbox: PointRect { left: 0.0, bottom: 0.0, right: 612.0, top: 792.0 }, rotation: Rotation::R90 };
        assert_eq!(g.to_display(0.0, 792.0), (792.0, 612.0));
        assert_eq!(PageGeometry::default().to_display(3.0, 4.0), (3.0, 4.0));
    }
```

**Tests d'intégration**, à la fin de `engine.rs`. Imports supplémentaires :
`sunipdf_core::{MarkupKind, PointRect, Stamp, StampPosition, TextStyle}`. Ne pas importer `PageGeometry` : seules ses
méthodes servent, et un import inutilisé ferait échouer clippy `-D warnings`. Si `engine.rs` importe déjà certains
de ces noms dans une fonction (`use` local, comme à la l. 379), garder un seul import de niveau module.

```rust
#[test]
fn geometry_matches_rendering_text_and_annotations_on_rotated_pages() {
    let engine = engine();
    let mut doc = engine.open(&fixture("rotated-offset.pdf"), None).unwrap();
    let cases = [
        (Rotation::R90, (792.0, 612.0), (140.0, 620.0)),
        (Rotation::R0, (612.0, 792.0), (240.0, 820.0)),
        (Rotation::R270, (700.0, 500.0), (140.0, 620.0)),
        (Rotation::R180, (300.0, 400.0), (90.0, 320.0)),
    ];
    for (i, (rotation, (w, h), (ux, uy))) in cases.into_iter().enumerate() {
        let size = doc.page_size(i).unwrap();
        assert_eq!((size.width_pt, size.height_pt), (w, h), "page {i}");
        let img = doc.render_page(i, 1.0).unwrap();
        let geo = img.geometry;
        assert_eq!(geo.rotation, rotation, "page {i}");
        assert_eq!(doc.page_geometry(i).unwrap(), geo);
        let px = |x: f32, y_up: f32| {
            let k = (((h - y_up) as u32 * img.width + x as u32) * 4) as usize;
            (img.rgba[k], img.rgba[k + 1], img.rgba[k + 2])
        };
        let (x, y) = geo.to_display(ux, uy);
        assert_eq!(px(x, y), (0, 0, 255), "page {i} : rectangle bleu attendu en ({x}, {y})");
        let back = geo.to_user(x, y);
        assert!((back.0 - ux).abs() < 0.01 && (back.1 - uy).abs() < 0.01, "page {i}");
        // Recherche et sélection : rectangles PDFium dans la page affichée, texte relu.
        let label = format!("MARK{}", i + 1);
        let found = doc.search_page(i, &label, true).unwrap();
        assert_eq!(found.len(), 1, "page {i}");
        let r = geo.rect_to_display(found[0].rects[0]);
        assert!(r.left >= 0.0 && r.bottom >= 0.0 && r.right <= w && r.top <= h, "page {i} : {r:?}");
        assert!(doc.text_in_rect(i, found[0].rects[0]).unwrap().text.contains(&label));
        // Surlignage : des pixels jaunes là où le texte est affiché.
        doc.add_markup(i, MarkupKind::Highlight, &found[0].rects, (255, 230, 0)).unwrap();
        let img = doc.render_page(i, 1.0).unwrap();
        let yellow = (r.left as u32..r.right as u32)
            .flat_map(|x| ((h - r.top) as u32..(h - r.bottom) as u32).map(move |row| (x, row)))
            .filter(|&(x, row)| {
                let k = ((row * img.width + x) * 4) as usize;
                img.rgba[k] > 200 && img.rgba[k + 1] > 180 && img.rgba[k + 2] < 120
            })
            .count();
        assert!(yellow > 50, "page {i} : surlignage hors du texte ({yellow})");
    }
}

#[test]
fn added_text_images_and_stamps_are_upright_on_rotated_pages() {
    let engine = engine();
    let dir = temp("redressement");
    let mut doc = engine.open(&fixture("rotated-offset.pdf"), None).unwrap();
    /// Limites affichées du dernier objet de la page.
    fn last_bounds(doc: &sunipdf_core::Document<'_>, page: usize) -> PointRect {
        let n = doc.page_objects(page).unwrap().len() - 1;
        doc.page_geometry(page).unwrap().rect_to_display(doc.object_info(page, n).unwrap().bounds)
    }
    for page in 0..doc.page_count() {
        let (x, y) = doc.page_geometry(page).unwrap().to_user(50.0, 50.0);
        doc.add_text(page, x, y, "Horizontal", &TextStyle::default()).unwrap();
        let d = last_bounds(&doc, page);
        assert!(d.right - d.left > 2.0 * (d.top - d.bottom), "page {page} : {d:?}");
        assert!((d.left - 50.0).abs() < 2.0, "page {page} : ancre {d:?}");
    }
    let png = dir.join("bande.png");
    image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255])).save(&png).unwrap();
    let (x, y) = doc.page_geometry(0).unwrap().to_user(100.0, 100.0);
    doc.add_image(0, &png, x, y, 80.0).unwrap();
    let d = last_bounds(&doc, 0);
    assert!((d.right - d.left - 80.0).abs() < 1.0 && (d.top - d.bottom - 40.0).abs() < 1.0, "{d:?}");
    assert!((d.left - 100.0).abs() < 1.0 && (d.bottom - 100.0).abs() < 1.0, "{d:?}");
    doc.stamp_pages(None, &Stamp::page_numbers("Page {n}", StampPosition::BottomCenter)).unwrap();
    for page in 0..doc.page_count() {
        let size = doc.page_size(page).unwrap();
        let d = last_bounds(&doc, page);
        assert!(d.left >= 0.0 && d.right <= size.width_pt && d.bottom >= 0.0 && d.top <= size.height_pt, "page {page} : {d:?}");
        assert!(((d.left + d.right) / 2.0 - size.width_pt / 2.0).abs() < 1.0, "page {page} : centrage {d:?}");
        assert!((d.bottom - 28.0).abs() < 1.0, "page {page} : marge {d:?}");
        assert!(d.right - d.left > d.top - d.bottom, "page {page} : horizontal {d:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}
```

Résultats attendus **avant** B3 et B4 : le second test échoue (texte vertical et tampons hors de la page). Le premier
test doit passer dès B1 et B2, car le moteur ne changeait pas de repère. Il fixe le contrat que l'interface utilise
ensuite.

---

## 5. Étape C : interface, conversions écran ↔ PDF (agent pdf-editor)

### C1. Type `PageView` : `apps/sunipdf-desktop/src/annotate.rs`

Il remplace `to_pdf` (l. 99-104). Retirer `SCREEN_PER_PDF_POINT` de l'import (l. 3), sinon clippy le signale comme
inutilisé. Ajouter `PageGeometry` à l'import de `sunipdf_core`.

```rust
/// Page affichée : rectangle écran, échelle et géométrie (rotation, origine de la boîte visible).
/// Seul passage entre l'écran et l'espace utilisateur PDF, celui de toutes les API du moteur.
#[derive(Clone, Copy, Debug)]
pub struct PageView {
    pub rect: egui::Rect,
    /// Points écran (logiques) par point PDF.
    pub k: f32,
    /// Hauteur affichée de la page (points), rotation comprise.
    pub height_pt: f32,
    pub geo: PageGeometry,
}

impl PageView {
    /// Point écran → point utilisateur PDF, borné à la page.
    pub fn to_pdf(&self, pos: egui::Pos2) -> (f32, f32) {
        let x = ((pos.x - self.rect.min.x) / self.k).clamp(0.0, self.rect.width() / self.k);
        let y = (self.height_pt - (pos.y - self.rect.min.y) / self.k).clamp(0.0, self.height_pt);
        self.geo.to_user(x, y)
    }

    /// Point utilisateur PDF → point écran.
    pub fn point(&self, (x, y): (f32, f32)) -> egui::Pos2 {
        let (x, y) = self.geo.to_display(x, y);
        self.rect.min + egui::vec2(x * self.k, (self.height_pt - y) * self.k)
    }

    /// Rectangle utilisateur PDF → rectangle écran.
    pub fn rect(&self, r: &PointRect) -> egui::Rect {
        egui::Rect::from_two_pos(self.point((r.left, r.bottom)), self.point((r.right, r.top)))
    }
}
```

### C2. Branchements

- `app.rs:38` `PageTexture` : ajouter `pub geometry: sunipdf_core::PageGeometry`. Dans le bras
  `Response::Rendered` (l. 412-417), ajouter `geometry: image.geometry,`.
- `app.rs` `document_area` (l. 1084), dans la fermeture du `ScrollArea` :
  - Le `match &self.texture` (l. 1145-1166) renvoie
    `Some(crate::annotate::PageView { rect: page_rect, k: self.effective_zoom * SCREEN_PER_PDF_POINT, height_pt: size.height_pt, geo: tex.geometry })`
    dans la branche image, et `None` après le `Spinner`.
  - Ajouter ensuite `let Some(view) = view else { return };`. Sans rendu de cette page, il n'y a pas de géométrie :
    ni surbrillance ni interaction, la page n'est pas encore visible.
  - Les 3 appels `paint_*` (l. 1168-1170) deviennent `(&painter, &view, page_index)`.
  - Les 5 appels `crate::annotate::to_pdf(pos, page_rect, zoom, page_h)` deviennent `view.to_pdf(pos)`. Supprimer
    les variables locales `zoom` et `page_h`.
- Signatures `(&self, painter: &egui::Painter, view: &crate::annotate::PageView, page_index: usize)` :
  - `annotate.rs:156` `paint_selection` : `to_screen(r)` → `view.rect(r)`.
  - `app.rs:1480` `paint_matches` : idem.
  - `editor.rs:532` `paint_object_selection` : `k` → `view.k`, `to_screen` → `view.rect`, la fermeture `p` (l. 573)
    → `view.point`.
- `editor.rs:389-408` (outil image : le point cliqué devient le coin haut-gauche **à l'écran**) :

  ```rust
  let geo = self.texture.as_ref().map(|t| t.geometry).unwrap_or_default();
  let (sx, sy) = geo.to_display(drag.start.0, drag.start.1);
  let (x, y) = geo.to_user(sx, sy - height_pt);
  // EditOp::AddImage { page: drag.page, path, x, y, width_pt }
  ```

- `editor.rs:483` `nudge_selected` : les flèches donnent un décalage **à l'écran**, converti en espace utilisateur :

  ```rust
  let geo = self.texture.as_ref().map(|t| t.geometry).unwrap_or_default();
  let (x0, y0) = geo.to_user(0.0, 0.0);
  let (x1, y1) = geo.to_user(dx, dy);
  // EditOp::NudgeObject { page, index, dx: x1 - x0, dy: y1 - y0 }
  ```

Le reste de `editor.rs` (`corner_at`, `preview_bounds`, `edit_drag_finished`) travaille déjà en espace utilisateur
et reste juste : une rotation d'un multiple de 90° garde les distances et les rectangles droits. Seul défaut
cosmétique accepté : sur une page à 90° ou 270°, les curseurs de redimensionnement en diagonale sont inversés.

**Conflit possible** : le plan `continuous-view.md` remplace `PageTexture` par des tuiles. Le premier des deux qui
est intégré impose sa structure à l'autre. La géométrie voyage dans `RenderedPage`, elle survit donc à cette refonte.

### C3. Test unitaire : `annotate.rs`, nouveau module `tests`

```rust
#[cfg(test)]
mod tests {
    use super::PageView;
    use sunipdf_core::{PageGeometry, PointRect, Rotation};

    #[test]
    fn page_view_follows_rotation() {
        let geo = PageGeometry { bbox: PointRect { left: 0.0, bottom: 0.0, right: 612.0, top: 792.0 }, rotation: Rotation::R90 };
        let view = PageView { rect: egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(792.0, 612.0)), k: 1.0, height_pt: 612.0, geo };
        // Rectangle bleu de la page 1 de rotated-offset.pdf (100..180 × 600..640).
        let r = view.rect(&PointRect { left: 100.0, bottom: 600.0, right: 180.0, top: 640.0 });
        assert_eq!(r, egui::Rect::from_min_max(egui::pos2(610.0, 120.0), egui::pos2(650.0, 200.0)));
        let (x, y) = view.to_pdf(r.center());
        assert!((x - 140.0).abs() < 1e-3 && (y - 620.0).abs() < 1e-3);
    }
}
```

---

## 6. Étape D : clavier (agent ui-ux)

### D1. Table de raccourcis unique : `apps/sunipdf-desktop/src/app.rs`

La table remplace la fermeture `key(…)` (l. 721-791). Elle corrige B1 et B2, et `Modifiers::plus` est `const fn`
(`EG/data/input/modifiers.rs:139`).

```rust
const SHIFT_CMD: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// Raccourcis globaux (hors champ de saisie), lus par `handle_input` et vérifiés contre le registre.
/// `consume_key` ignore une touche Maj en trop : chaque variante avec Maj précède sa variante simple.
pub const BINDINGS: &[(Modifiers, Key, Command)] = &[
    (Modifiers::COMMAND, Key::O, Command::OpenDocument),
    (Modifiers::COMMAND, Key::W, Command::CloseDocument),
    (Modifiers::COMMAND, Key::Plus, Command::ZoomIn),
    (Modifiers::COMMAND, Key::Equals, Command::ZoomIn),
    (Modifiers::COMMAND, Key::Minus, Command::ZoomOut),
    (Modifiers::COMMAND, Key::Num0, Command::ActualSize),
    (Modifiers::NONE, Key::PageDown, Command::NextPage),
    (Modifiers::NONE, Key::ArrowRight, Command::NextPage),
    (Modifiers::NONE, Key::PageUp, Command::PreviousPage),
    (Modifiers::NONE, Key::ArrowLeft, Command::PreviousPage),
    (Modifiers::NONE, Key::Home, Command::FirstPage),
    (Modifiers::NONE, Key::End, Command::LastPage),
    (Modifiers::NONE, Key::F4, Command::ToggleThumbnails),
    (Modifiers::COMMAND, Key::F, Command::Find),
    (SHIFT_CMD, Key::S, Command::SaveAs),
    (Modifiers::COMMAND, Key::S, Command::Save),
    (SHIFT_CMD, Key::R, Command::RotateCounterClockwise),
    (Modifiers::COMMAND, Key::R, Command::RotateClockwise),
    (Modifiers::COMMAND, Key::Delete, Command::DeletePage),
    (Modifiers::NONE, Key::F9, Command::ToggleTools),
    (Modifiers::NONE, Key::F8, Command::OrganizePages),
    (Modifiers::NONE, Key::F7, Command::ToggleComments),
    (Modifiers::NONE, Key::F6, Command::ToggleBookmarks),
    (SHIFT_CMD, Key::D, Command::DuplicateObject), // avant Ctrl+D (B1)
    (Modifiers::COMMAND, Key::D, Command::Properties),
    (Modifiers::COMMAND, Key::C, Command::CopyText),
    (Modifiers::NONE, Key::V, Command::ToolSelect),
    (Modifiers::NONE, Key::H, Command::ToolHighlight),
    (Modifiers::NONE, Key::U, Command::ToolUnderline),
    (Modifiers::NONE, Key::X, Command::ToolStrikeOut),
    (Modifiers::NONE, Key::N, Command::ToolNote),
    (Modifiers::NONE, Key::E, Command::ToolSelectObject),
    (Modifiers::NONE, Key::T, Command::ToolAddText),
    (Modifiers::NONE, Key::I, Command::ToolAddImage),
    (Modifiers::NONE, Key::R, Command::ToolRectangle),
    (Modifiers::NONE, Key::O, Command::ToolEllipse),
    (Modifiers::NONE, Key::L, Command::ToolLine),
    (Modifiers::NONE, Key::Delete, Command::DeleteObject),
    (Modifiers::NONE, Key::F2, Command::EditObjectText),
    (Modifiers::COMMAND, Key::CloseBracket, Command::BringToFront),
    (Modifiers::COMMAND, Key::OpenBracket, Command::SendToBack),
    (SHIFT_CMD, Key::Z, Command::Redo), // avant Ctrl+Z (B2)
    (Modifiers::COMMAND, Key::Z, Command::Undo),
    (Modifiers::COMMAND, Key::Y, Command::Redo),
    (SHIFT_CMD, Key::I, Command::InsertPages),
    (Modifiers::COMMAND, Key::K, Command::CommandPalette),
    (SHIFT_CMD, Key::Tab, Command::PrevTab),
    (Modifiers::COMMAND, Key::Tab, Command::NextTab),
    (Modifiers::COMMAND, Key::Comma, Command::Settings),
    (Modifiers::NONE, Key::F1, Command::KeyboardShortcuts),
];
```

L'ordre et les entrées reprennent exactement la fermeture actuelle. Seuls changent la place de Ctrl+Maj+D et de
Ctrl+Maj+Z, et l'ajout de Ctrl+, et F1 (étape E).

### D2. `handle_input` (l. 597)

- l. 612-616 : ajouter `|| self.show_settings || self.show_shortcuts` au garde des dialogues.
- l. 623 et l. 656 : remplacer `ctx.memory(|m| m.focused().is_some())` par `ctx.text_edit_focused()` (B3). Seul un
  **champ de saisie** suspend les raccourcis sans Ctrl.
- Avant le bloc de décalage d'objet (l. 690 environ), ajouter
  `let widget_focus = ctx.memory(|m| m.focused().is_some());` (bouton atteint par Tab). Ajouter aussi
  `&& !widget_focus` à la condition du décalage par flèches.
- Remplacer la fermeture `key` par :

  ```rust
  let mut commands = Vec::new();
  ctx.input_mut(|i| {
      for &(m, k, c) in BINDINGS {
          // Un contrôle a le focus : les flèches seules servent à la navigation clavier d'egui.
          let arrow = matches!(k, Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown);
          if widget_focus && arrow && !m.any() {
              continue;
          }
          if i.consume_key(m, k) {
              commands.push(c);
          }
      }
  });
  ```

### D3. Test dans `app.rs` (module `tests`, l. 1975 et suivantes)

```rust
    #[test]
    fn bindings_match_registry_and_shift_variants_come_first() {
        use std::collections::HashSet;
        let bound: HashSet<Command> = BINDINGS.iter().map(|b| b.2).collect();
        for info in sunipdf_core::REGISTRY {
            // Alt+F4 est géré par Windows.
            if info.shortcut_key.is_some() && info.command != Command::Quit {
                assert!(bound.contains(&info.command), "{} : raccourci affiché mais non lié", info.id);
            }
        }
        for c in &bound {
            assert!(c.info().shortcut_key.is_some(), "{} : raccourci lié mais absent du registre", c.info().id);
        }
        for (i, (m, k, _)) in BINDINGS.iter().enumerate() {
            if !m.shift {
                assert!(
                    !BINDINGS[i + 1..].iter().any(|(m2, k2, _)| k2 == k && m2.shift && m2.command == m.command && m2.alt == m.alt),
                    "{k:?} : la variante avec Maj doit précéder la variante simple"
                );
            }
        }
    }
```

Avant D1 (ordre actuel), ce test échoue sur `D` et `Z` : c'est la reproduction automatique de B1 et B2.

---

## 7. Étape E : boîtes « Paramètres » et « Raccourcis clavier » (agent ui-ux)

### E1. Registre : `crates/sunipdf-core/src/command.rs`

- Variantes `Settings` et `KeyboardShortcuts` dans `enum Command` (l. 5-71).
- Entrées dans `REGISTRY` :
  - `cmd!(Settings, "app.settings", "menu.file.settings", Some("shortcut.settings"), false, File)`, juste **avant**
    l'entrée `Quit` (l. 159).
  - `cmd!(KeyboardShortcuts, "help.shortcuts", "menu.help.shortcuts", Some("shortcut.shortcuts"), false, Help)`,
    juste **avant** `About` (l. 606).
- Test `every_command_variant_is_registered` (l. 648) : `assert_eq!(Command::in_menu(Menu::File).count(), 8);`.
- **Conflit possible** avec `tool-hub.md` et `print.md`, qui ajoutent aussi des commandes `File` : ajuster le
  compte au moment de la fusion.

### E2. Icônes et coquille

- `icons.rs` (module `glyph`) : `pub const SETTINGS: &str = "\u{E713}";` et `pub const KEYBOARD: &str = "\u{E765}";`.
  Ce sont les points de code Segoe MDL2 « Setting » et « KeyboardClassic » ⚠, à vérifier sur capture.
- `shell.rs:246` `command_icon`, `match` exhaustif : `Command::Settings => (glyph::SETTINGS, "⚙")`,
  `Command::KeyboardShortcuts => (glyph::KEYBOARD, "⌨")`. `command_color` les classe déjà par menu.
- `shell.rs:471-500` : **supprimer** le sous-menu « Paramètres » (langue, thème) du menu ☰. « Paramètres… » y
  apparaît désormais par le menu Fichier, construit depuis le registre. Le globe et le bouton de thème de la barre
  d'onglets restent comme raccourcis.
- `shell.rs:427` : le bouton « ? » (`toolbar.help`) exécute `Command::KeyboardShortcuts`. « À propos » reste
  accessible par le bouton « i » (l. 538) et par le menu Aide.

### E3. Préférences : `apps/sunipdf-desktop/src/settings.rs`

```rust
    /// Zoom appliqué à chaque document ouvert : `fit_width`, `fit_page` ou `actual_size`.
    pub default_zoom: String,          // champ de Settings ; Default : "fit_width".into()

    pub fn zoom(&self) -> crate::app::Zoom {
        use crate::app::Zoom;
        match self.default_zoom.as_str() {
            "fit_page" => Zoom::FitPage,
            "actual_size" => Zoom::Fixed(1.0),
            _ => Zoom::FitWidth, // valeur inconnue : défaut, sans perdre les autres préférences
        }
    }
```

Le champ reste une `String`, comme `language`. Un `enum` serde mal lu ferait échouer toute la désérialisation, et
`Settings::load` reviendrait alors aux valeurs par défaut pour **toutes** les préférences.

Application : `app.rs` bras `Response::Opened` (l. 349), ajouter `self.zoom = self.settings.zoom();`. C'est
l'unique point par lequel passe tout nouveau document, onglet compris.

Test, dans le module `tests` de `settings.rs` :

```rust
    #[test]
    fn unicode_recent_paths_and_default_zoom_round_trip() {
        use crate::app::Zoom;
        let mut s = Settings::default();
        assert_eq!(s.zoom(), Zoom::FitWidth);
        s.push_recent(std::path::Path::new(r"C:\Données\文書 📄\été.pdf"));
        s.default_zoom = "fit_page".into();
        let back: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.zoom(), Zoom::FitPage);
        let odd: Settings = serde_json::from_str(r#"{"default_zoom":"zoom-42","dark_theme":true}"#).unwrap();
        assert_eq!(odd.zoom(), Zoom::FitWidth);
        assert!(odd.dark_theme, "une valeur inconnue ne réinitialise pas le reste");
    }
```

### E4. Nouveau module `apps/sunipdf-desktop/src/prefs.rs`

Le déclarer par `mod prefs;` dans `main.rs` (l. 3-15).

```rust
//! Boîtes « Paramètres » et « Raccourcis clavier ». Corps sans état d'application : testables avec un
//! `egui::Context` nu (voir les tests).

use crate::app::SuniApp;
use crate::dialog;
use crate::icons::glyph;
use crate::settings::Settings;
use egui::{Key, RichText};
use sunipdf_core::{Menu, REGISTRY};
use sunipdf_i18n::{tr, tr_args, Language};

/// Zooms proposés à l'ouverture : (valeur enregistrée, clé du libellé).
const ZOOMS: [(&str, &str); 3] = [
    ("fit_width", "menu.view.fit_width"),
    ("fit_page", "menu.view.fit_page"),
    ("actual_size", "menu.view.actual_size"),
];

/// Menus du registre, dans l'ordre d'affichage, avec la clé de leur titre.
const MENUS: [(Menu, &str); 7] = [
    (Menu::File, "menu.file"),
    (Menu::Edit, "menu.edit"),
    (Menu::View, "menu.view"),
    (Menu::Pages, "menu.pages"),
    (Menu::Security, "menu.security"),
    (Menu::Convert, "menu.convert"),
    (Menu::Help, "menu.help"),
];

/// Corps de « Paramètres » : modifie `s` ; l'appelant applique et enregistre la différence.
pub fn settings_form(ui: &mut egui::Ui, s: &mut Settings) {
    let lang = s.language();
    dialog::field_label(ui, tr(lang, "menu.settings.language"));
    ui.horizontal(|ui| {
        for l in Language::ALL {
            if ui.radio(lang == l, l.native_name()).clicked() {
                s.set_language(l);
            }
        }
    });
    dialog::field_label(ui, tr(lang, "menu.settings.theme"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut s.dark_theme, false, tr(lang, "menu.settings.theme.light"));
        ui.radio_value(&mut s.dark_theme, true, tr(lang, "menu.settings.theme.dark"));
    });
    dialog::field_label(ui, tr(lang, "settings.default_zoom"));
    ui.horizontal_wrapped(|ui| {
        for (id, key) in ZOOMS {
            if ui.radio(s.default_zoom == id, tr(lang, key)).clicked() {
                s.default_zoom = id.to_string();
            }
        }
    });
    dialog::field_label(ui, tr(lang, "settings.panels"));
    ui.checkbox(&mut s.show_thumbnails, tr(lang, "menu.view.thumbnails"));
    ui.checkbox(&mut s.show_tools, tr(lang, "menu.view.tools"));
    dialog::field_label(ui, tr(lang, "home.recent"));
    ui.horizontal(|ui| {
        let count = s.recent.len().to_string();
        ui.label(tr_args(lang, "settings.recent_count", &[("count", &count)]));
        let clear = egui::Button::new(tr(lang, "settings.recent_clear"));
        if ui.add_enabled(!s.recent.is_empty(), clear).clicked() {
            s.recent.clear();
        }
    });
}

/// Liste des raccourcis, construite depuis le registre et groupée par menu.
pub fn shortcuts_list(ui: &mut egui::Ui, lang: Language) {
    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
        for (menu, title) in MENUS {
            let rows: Vec<(&str, &str)> = REGISTRY
                .iter()
                .filter(|c| c.menu == menu)
                .filter_map(|c| c.shortcut_key.map(|k| (c.label_key, k)))
                .collect();
            if rows.is_empty() {
                continue;
            }
            ui.label(crate::icons::semibold(tr(lang, title), 14.0));
            egui::Grid::new(title).num_columns(2).striped(true).show(ui, |ui| {
                for (label, shortcut) in rows {
                    ui.label(tr(lang, label));
                    ui.label(RichText::new(tr(lang, shortcut)).monospace());
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
        }
        ui.label(RichText::new(tr(lang, "shortcuts.note")).size(12.5));
    });
}

impl SuniApp {
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let lang = self.lang();
        let before = self.settings.clone();
        let mut close = false;
        let spec = dialog::Spec::new("settings", tr(lang, "menu.settings"), self.settings.dark_theme, self.has_icons)
            .subtitle(tr(lang, "settings.subtitle"))
            .icon(glyph::SETTINGS, "⚙")
            .width(460.0);
        let settings = &mut self.settings;
        dialog::show(ctx, spec, |ui| settings_form(ui, settings), |ui| {
            if dialog::primary(ui, tr(lang, "about.close")) || ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
        });
        if self.settings != before {
            if self.settings.dark_theme != before.dark_theme {
                crate::theme::apply(ctx, self.settings.dark_theme);
            }
            if self.settings.language != before.language {
                self.update_title(ctx);
            }
            if self.settings.recent.is_empty() && !before.recent.is_empty() {
                self.status_msg = Some(tr(self.lang(), "status.recent_cleared").to_string());
            }
            self.show_tools = self.settings.show_tools;
            self.persist_settings();
        }
        if close {
            self.show_settings = false;
        }
    }

    pub fn shortcuts_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            return;
        }
        let lang = self.lang();
        let mut close = false;
        let spec = dialog::Spec::new("shortcuts", tr(lang, "menu.help.shortcuts"), self.settings.dark_theme, self.has_icons)
            .subtitle(tr(lang, "shortcuts.subtitle"))
            .icon(glyph::KEYBOARD, "⌨")
            .width(520.0);
        dialog::show(ctx, spec, |ui| shortcuts_list(ui, lang), |ui| {
            if dialog::primary(ui, tr(lang, "about.close")) || ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
        });
        if close {
            self.show_shortcuts = false;
        }
    }
}
```

Branchements dans `app.rs` :

- Champs `pub show_settings: bool` et `pub show_shortcuts: bool` (près de `show_about`, l. 105), initialisés à
  `false` dans `new`.
- Bras de `execute` (l. 1035) : `Command::Settings => self.show_settings = true,` et
  `Command::KeyboardShortcuts => self.show_shortcuts = true,`.
- Dans `fn ui` (l. 1914) : `self.settings_dialog(&ctx); self.shortcuts_dialog(&ctx);` après `self.about_dialog(&ctx);`.
- Test `ui_keys_used_by_app_exist` (l. 1997-1998) :
  `concat!(include_str!("app.rs"), include_str!("main.rs"), include_str!("prefs.rs"))`. `prefs.rs` ne contient
  aucune chaîne « a.b » minuscule qui ne soit pas une clé : `fit_width` et les autres valeurs n'ont pas de point.

Les modifications s'appliquent immédiatement, comme les menus actuels de langue et de thème : pas de bouton
« Appliquer » ni de copie de travail.

### E5. Tests, dans `prefs.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Node, NodeId, Role, TreeUpdate};

    fn key(k: egui::Key) -> egui::Event {
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }
    }

    fn frame(ctx: &egui::Context, s: &mut Settings, events: Vec<egui::Event>) -> TreeUpdate {
        let input = egui::RawInput { events, ..Default::default() };
        ctx.run_ui(input, |ui| settings_form(ui, s)).platform_output.accesskit_update.expect("AccessKit actif")
    }

    fn label(update: &TreeUpdate, id: NodeId) -> String {
        update.nodes.iter().find(|(n, _)| *n == id).and_then(|(_, node)| node.label()).unwrap_or_default().to_string()
    }

    #[test]
    fn settings_form_is_named_tab_ordered_and_keyboard_operable() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut s = Settings::default();
        s.push_recent(std::path::Path::new(r"C:\Données\文書 📄.pdf"));
        let first = frame(&ctx, &mut s, vec![]);
        let controls: Vec<&Node> = first.nodes.iter().map(|(_, n)| n)
            .filter(|n| matches!(n.role(), Role::Button | Role::RadioButton | Role::CheckBox)).collect();
        assert_eq!(controls.len(), 10);
        for n in &controls {
            let l = n.label().unwrap_or("");
            assert!(!l.trim().is_empty() && !l.chars().any(|c| ('\u{E000}'..='\u{F8FF}').contains(&c)), "nom illisible : {l:?}");
        }
        let fr = Language::French;
        let mut expected: Vec<String> = Language::ALL.iter().map(|l| l.native_name().to_string()).collect();
        expected.extend(
            ["menu.settings.theme.light", "menu.settings.theme.dark", "menu.view.fit_width", "menu.view.fit_page",
             "menu.view.actual_size", "menu.view.thumbnails", "menu.view.tools", "settings.recent_clear"]
                .map(|k| tr(fr, k).to_string()),
        );
        let order: Vec<String> = (0..expected.len())
            .map(|_| { let u = frame(&ctx, &mut s, vec![key(egui::Key::Tab)]); label(&u, u.focus) })
            .collect();
        assert_eq!(order, expected, "ordre de tabulation = ordre visuel");
        // Entrée sur « Effacer la liste » (dernier contrôle focalisé) vide les fichiers récents.
        let _ = frame(&ctx, &mut s, vec![key(egui::Key::Enter)]);
        assert!(s.recent.is_empty());
    }

    #[test]
    fn shortcuts_dialog_lists_every_registry_shortcut() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        for lang in Language::ALL {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| shortcuts_list(ui, lang));
            let update = out.platform_output.accesskit_update.expect("AccessKit actif");
            let texts: std::collections::HashSet<&str> =
                update.nodes.iter().filter_map(|(_, n)| n.value().or(n.label())).collect();
            for info in REGISTRY {
                if let Some(k) = info.shortcut_key {
                    assert!(texts.contains(tr(lang, k)), "{} : {k}", lang.code());
                    assert!(texts.contains(tr(lang, info.label_key)), "{} : {}", lang.code(), info.label_key);
                }
            }
        }
    }
}
```

APIs vérifiées pour ces tests :

| API | Source |
|---|---|
| `Context::run_ui` | `EG/context.rs:794` |
| `enable_accesskit` | `EG/context.rs:3701` |
| `PlatformOutput::accesskit_update` | `EG/data/output.rs:176` |
| `TreeUpdate { nodes: Vec<(NodeId, Node)>, focus }` | `AK/lib.rs:2850`, `:2878` |
| `Node::label` et `Node::value` | `AK/lib.rs:1916-1918` |
| `Event::Key { key, physical_key, pressed, repeat, modifiers }` | `EG/data/input/event.rs:37-69` |
| Un `Label` porte son texte en `value`, les autres widgets en `label` | `EG/response.rs:965-971` |
| `Language::native_name` | `crates/sunipdf-i18n/src/lib.rs:33` |

⚠ Les lignes du `ScrollArea` hors de la zone visible créent bien leur widget, puisque le défilement n'est pas
virtualisé. Si le second test ne trouvait pas une ligne basse, augmenter `max_height` dans le test n'est **pas** la
réponse : il faut utiliser `ui.set_max_height` côté appelant.

---

## 8. Étape F : accessibilité de base (agent forms-accessibility)

### F1. Nom accessible des boutons à icône seule (B10) : `apps/sunipdf-desktop/src/icons.rs`

```rust
/// Infobulle et nom accessible d'un bouton à icône seule. Sans nom, AccessKit (Narrateur, NVDA) annonce le
/// glyphe, un caractère à usage privé illisible (le nom par défaut d'un `Button` est son texte).
pub fn named(resp: egui::Response, label: &str) -> egui::Response {
    resp.ctx.accesskit_node_builder(resp.id, |node| node.set_label(label));
    resp.on_hover_text(label)
}
```

Les infobulles existantes (déjà traduites) servent de noms. La forme générale est `.on_hover_text(x)` →
`icons::named(…, x)`.

| Fichier | Emplacements |
|---|---|
| `shell.rs` | `fn icon_button` (l. 322-326 : `named(ui.add(…), tooltip).clicked()`, ce qui couvre 11 appels) ; bouton Accueil (l. 342, `tabs.home`) ; globe (`ui.menu_button(…).response`, l. 430, `menu.settings.language`) ; Enregistrer (l. 530, `menu.file.save`) ; rail : miniatures, organiser, signets, commentaires, recherche, ajuster largeur, ajuster page (l. 886-962) |
| `annotate.rs` | palette d'outils (l. 136), fermer les commentaires (l. 320), supprimer une annotation (l. 361-366) |
| `docinfo.rs:46` | fermer |
| `organize.rs:69` | barre d'outils |
| `editor.rs:656` | barre d'objet |
| `app.rs` `find_bar` | « ▲ » (l. 1392, `find.previous`), « ▼ » (l. 1399, `find.next`), « ✕ » (l. 1424, `find.close`) |

Champs sans libellé visible : `named(field, …)` avec leur indication.

- Page (`shell.rs:999-1005`, `toolbar.page_field`).
- Recherche (`app.rs` `find_bar`, `find.placeholder`).
- Palette (`shell.rs:1039`, `palette.placeholder`).
- Note (`annotate.rs:255`, `dialog.note.placeholder`).

### F2. Rôle et nom des widgets personnalisés : `Response::widget_info` (`EG/response.rs:869`)

| Emplacement | Ajout |
|---|---|
| `shell.rs:189` `tool_row`, avant le `return` anticipé (l. 207) | `resp.widget_info(\|\| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));` |
| `shell.rs:393` onglet | `resp.widget_info(\|\| egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_active, &title));` |
| `shell.rs:566` pastille de recherche | `labeled(Button, true, tr(lang, "toolbar.search_commands"))` |
| `shell.rs:1100` ligne de palette | `selected(Button, true, selected, tr(lang, info.label_key))` |
| `app.rs:1299` miniature, `organize.rs:152` cellule | `labeled(Button, true, tr_args(lang, "a11y.thumbnail", &[("page", &(page + 1).to_string())]))` |
| `app.rs:1171` zone de page (`page-interact`) | `labeled(WidgetType::Other, true, tr(lang, "a11y.document_canvas"))`. La clé existe déjà mais n'était utilisée nulle part. |

### F3. Libellés associés aux champs

- `dialog.rs:210` : `pub fn field_label(ui, text) -> egui::Response`. Elle renvoie le `ui.label(…)`. `Response`
  n'est pas `#[must_use]` (`EG/response.rs:23-24`), donc les appels qui ignorent le retour compilent sans
  avertissement.
- Champ précédé d'un `field_label` : `ui.add(TextEdit…).labelled_by(label.id)` (`EG/response.rs:985`). Cela concerne
  `app.rs` `password_dialog` (l. 1637), `docinfo.rs:343`, `tools.rs:187` et `tools.rs:220`.
- Grilles à deux colonnes, même chose avec le `ui.label` de la colonne 1 : `docinfo.rs:260-272` (mots de passe) et
  `docinfo.rs:128-140` (propriétés).

### F4. Focus visible, ordre, annonces

- **Focus visible.** Les boutons egui ont déjà un rendu « actif » au focus, et `tool_row` peint `has_focus`
  (`shell.rs:211`). Ajouter `if resp.has_focus() { ui.painter().rect_stroke(resp.rect, 6.0, Stroke::new(2.0, p.accent), egui::StrokeKind::Outside); }`
  après l'interaction pour : onglets (`shell.rs:393`), lignes de palette (`:1100`), pastille de recherche (`:566`),
  miniatures (`app.rs:1299`).
- **Ordre de tabulation.** egui suit l'ordre de création des widgets. Dans `fn ui` (`app.rs:1788`), déplacer le
  bloc du panneau `thumbnails` (l. 1858-1864) **avant** le panneau droit `comments`. Il faut pour cela le sortir du
  `if self.document.is_some()` du rail et le garder sous la même condition.
  - Ordre obtenu : onglets, barre d'outils, recherche, panneaux gauches (outils, signets, miniatures), panneaux
    droits (commentaires, rail), zone document.
  - L'ordre relatif des panneaux gauches entre eux, et des panneaux droits entre eux, ne change pas : aucune
    modification de mise en page attendue, à vérifier par capture.
  - La zone document reste la dernière : c'est une contrainte d'egui, le panneau central se déclare en dernier.
- **Erreurs annoncées.** Dans `app.rs:1074` `error_banner` :
  `let banner = ui.colored_label(…); banner.ctx.accesskit_node_builder(banner.id, |n| n.set_live(egui::accesskit::Live::Assertive));`.
  Même chose avec `Live::Polite` pour le message d'état (`shell.rs:1178`). `set_live` est à `AK/lib.rs:2139`, et
  l'émission UIA à `AKW/adapter.rs:225-231`.
- **Dialogues identifiés.** Dans `dialog.rs:80`, garder le `ModalResponse` (`EG/containers/modal.rs:124-138`), puis :

  ```rust
  modal.response.ctx.accesskit_node_builder(modal.response.id, |n| {
      n.set_role(egui::accesskit::Role::Dialog);
      n.set_label(spec.title.as_str());
  });
  ```

  ⚠ À valider au Narrateur : l'annonce « boîte de dialogue, *titre* » à l'ouverture. Sans effet audible, retirer
  ces lignes.

### F5. Garde-fou automatique, dans le module `tests` de `icons.rs`

```rust
#[cfg(test)]
mod tests {
    /// Chaque bouton à icône seule doit recevoir un nom accessible par `named`.
    // ponytail: comptage textuel, pas d'analyse syntaxique ; suffisant tant que le style de code reste celui-ci.
    #[test]
    fn icon_only_buttons_are_named() {
        let files = [
            ("annotate.rs", include_str!("annotate.rs")),
            ("app.rs", include_str!("app.rs")),
            ("docinfo.rs", include_str!("docinfo.rs")),
            ("editor.rs", include_str!("editor.rs")),
            ("organize.rs", include_str!("organize.rs")),
            ("prefs.rs", include_str!("prefs.rs")),
            ("shell.rs", include_str!("shell.rs")),
            ("tools.rs", include_str!("tools.rs")),
        ];
        let icon_first = ["icons::icon(", "crate::icons::icon(", "self.icon("];
        for (name, src) in files {
            let flat: String = src.chars().filter(|c| !c.is_whitespace()).collect();
            let buttons = flat
                .match_indices("Button::new(")
                .filter(|(i, m)| icon_first.iter().any(|p| flat[i + m.len()..].starts_with(p)))
                .count()
                + flat.matches("menu_button(self.icon(").count();
            let named = flat.matches("named(").count();
            assert!(named >= buttons, "{name} : {buttons} bouton(s) à icône seule, {named} nom(s) accessible(s)");
        }
    }
}
```

Avant F1, ce test échoue sur `annotate.rs`, `docinfo.rs`, `editor.rs`, `organize.rs` et `shell.rs`.

### F6. Vérification manuelle (à consigner, pas d'automatisation possible ici)

1. Lancer le Narrateur (Ctrl+Win+Entrée). Ouvrir `two-pages.pdf`. Parcourir avec Tab et Maj+Tab.
2. Chaque bouton doit annoncer son action (« Fermer l'onglet, bouton », « Miniatures, bouton », …) et jamais un
   caractère inconnu.
3. Ouvrir F1 et Ctrl+, : le titre du dialogue est annoncé, Échap ferme, le focus reste dans le dialogue.
4. Provoquer une erreur (déposer un `.txt`) : le message est annoncé.
5. Clavier seul, sans souris, pour chaque scénario : ouvrir un fichier (Ctrl+O), changer de page, zoomer, ouvrir le
   panneau d'outils (F9) puis lancer un outil (Tab, Entrée), passer d'onglet (Ctrl+Tab), enregistrer (Ctrl+S).
6. Consigner le résultat (« OK » ou défaut) dans `docs/PROJECT_STATUS.md`. Toute limite restante va dans
   `docs/KNOWN_ISSUES.md`, par exemple : pas de sélection de texte au clavier, zone document dernière dans l'ordre
   de tabulation.

---

## 9. Étape G : banc de mesure reproductible (agent qa-performance)

Nouveau fichier `crates/sunipdf-core/tests/bench.rs`. C'est un binaire de test dédié, ignoré par défaut : aucune
modification de `Cargo.toml` n'est nécessaire, et la mesure mémoire n'est pas polluée par d'autres tests.

```rust
//! Banc de mesure reproductible, ignoré par `cargo test` :
//!
//! ```text
//! cargo test -p sunipdf-core --release -j 2 --test bench -- --ignored --nocapture
//! ```
//!
//! Document : 500 pages synthétiques (250 fusions de `two-pages.pdf`) ou le fichier désigné par
//! `SUNIPDF_BENCH_PDF`. Un seul test dans ce binaire, donc un seul `Engine::new()` par processus
//! (PDFium ne se réinitialise pas) : ne pas ajouter d'autre test ici.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sunipdf_core::Engine;

const ITERATIONS: usize = 10;

#[test]
#[ignore = "banc de mesure : à lancer en --release, voir l'en-tête du fichier"]
fn bench_open_render_search_memory() {
    let engine = Engine::new().expect("pdfium.dll introuvable : scripts/fetch-pdfium.ps1");
    let dir = std::env::temp_dir().join(format!("sunipdf-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let custom = std::env::var_os("SUNIPDF_BENCH_PDF");
    let synthetic = custom.is_none();
    let path: PathBuf = match custom {
        Some(p) => p.into(),
        None => {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/two-pages.pdf");
            let out = dir.join("bench-500.pdf");
            assert_eq!(engine.merge(&vec![src; 250], &out).unwrap(), 500);
            out
        }
    };
    let (ws_start, _) = memory();
    let open = measure(|| drop(engine.open(&path, None).unwrap()));
    let first_render = measure(|| {
        let doc = engine.open(&path, None).unwrap();
        doc.render_page(0, 2.0).unwrap(); // 100 % sur un écran à 150 %
    });
    let doc = engine.open(&path, None).unwrap();
    let (ws_open, _) = memory();
    let mut hits = 0;
    let search = measure(|| hits = doc.search("page", false).unwrap().len());
    let started = Instant::now();
    for page in 0..doc.page_count() {
        doc.render_page(page, 96.0 / 72.0).unwrap(); // défilement complet à 100 %
    }
    let render_all = started.elapsed();
    let (ws_end, peak) = memory();
    if synthetic {
        assert_eq!(hits, 500, "une occurrence de « page » par page");
    }
    println!(
        "bench | machine | {} | {} threads | profil {}",
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default(),
        std::thread::available_parallelism().map_or(0, |n| n.get()),
        if cfg!(debug_assertions) { "debug" } else { "release" }
    );
    println!("bench | document | {} | {} pages | {} octets", path.display(), doc.page_count(), std::fs::metadata(&path).unwrap().len());
    report("ouverture", open);
    report("ouverture + rendu p. 1 (x2)", first_render);
    report(&format!("recherche ({hits} occurrences)"), search);
    println!("bench | rendu de toutes les pages (100 %) | {:.1} ms", ms(render_all));
    println!(
        "bench | ensemble de travail | départ {} Mo | document ouvert {} Mo | fin {} Mo | pic {} Mo",
        ws_start >> 20, ws_open >> 20, ws_end >> 20, peak >> 20
    );
    drop(doc);
    let _ = std::fs::remove_dir_all(dir);
}

fn measure(mut f: impl FnMut()) -> [Duration; 3] {
    let mut runs: Vec<Duration> = (0..ITERATIONS)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .collect();
    runs.sort();
    [runs[0], runs[ITERATIONS / 2], runs[ITERATIONS - 1]]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn report(name: &str, [min, med, max]: [Duration; 3]) {
    println!("bench | {name} | min {:.1} ms | médiane {:.1} ms | max {:.1} ms | n={ITERATIONS}", ms(min), ms(med), ms(max));
}

/// PROCESS_MEMORY_COUNTERS (WS/System/ProcessStatus/mod.rs:103-114).
#[repr(C)]
#[derive(Default)]
struct MemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_pools: [usize; 4],
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn K32GetProcessMemoryInfo(process: *mut std::ffi::c_void, counters: *mut MemoryCounters, cb: u32) -> i32;
}

/// (ensemble de travail, pic) du processus, en octets.
fn memory() -> (usize, usize) {
    let mut c = MemoryCounters { cb: std::mem::size_of::<MemoryCounters>() as u32, ..Default::default() };
    // SAFETY : pseudo-handle du processus courant ; structure de la taille annoncée dans `cb`.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    assert_ne!(ok, 0, "K32GetProcessMemoryInfo");
    (c.working_set, c.peak_working_set)
}
```

Procédure. Branché sur secteur, autres applications fermées, lancer **deux fois** la commande d'en-tête et
conserver la seconde sortie. Lancer aussi une fois avec un vrai PDF volumineux, désigné par
`SUNIPDF_BENCH_PDF=chemin`, sans le versionner et sans données personnelles.

Créer ensuite `docs/QA_PERFORMANCE.md` avec :

- la machine (sortie de
  `Get-CimInstance Win32_Processor, Win32_OperatingSystem | Select Name, NumberOfCores, NumberOfLogicalProcessors, Caption, BuildNumber, TotalVisibleMemorySize`) ;
- la version de PDFium (7881) ;
- la commande exacte et la date ;
- les lignes `bench |` **copiées telles quelles**.

Aucun seuil n'est affirmé avant la première mesure. Le chiffre « mémoire avec 500 pages » est le pic de l'ensemble
de travail après le rendu de toutes les pages. Il n'inclut ni le cache de 256 Mo ni les textures de l'application,
ce que le document doit indiquer.

---

## 10. Clés i18n (`assets/i18n/fr-FR.json` **et** `en-US.json`, propriétaire ui-ux, D013)

Format : JSON indenté de 2 espaces, UTF-8 sans BOM, fins de ligne CRLF (comme les fichiers actuels).

**Nouvelles clés (12)**

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.settings` | Paramètres… | Settings… |
| `shortcut.settings` | Ctrl+, | Ctrl+, |
| `menu.help.shortcuts` | Raccourcis clavier | Keyboard Shortcuts |
| `shortcut.shortcuts` | F1 | F1 |
| `settings.subtitle` | Enregistrés sur ce poste et appliqués immédiatement | Saved on this computer and applied immediately |
| `settings.default_zoom` | Zoom à l'ouverture d'un document | Zoom when a document opens |
| `settings.panels` | Panneaux affichés | Visible panels |
| `settings.recent_count` | {count} fichier(s) dans la liste | {count} file(s) in the list |
| `settings.recent_clear` | Effacer la liste | Clear list |
| `status.recent_cleared` | Liste des fichiers récents effacée | Recent files list cleared |
| `shortcuts.subtitle` | Raccourcis disponibles, par menu | Available shortcuts, by menu |
| `shortcuts.note` | Objet sélectionné : flèches pour le déplacer (Maj : 10 pt), Échap pour le désélectionner. Tab passe d'un contrôle à l'autre, Entrée ou Espace l'active. | Selected object: arrow keys move it (Shift: 10 pt), Esc deselects it. Tab moves between controls, Enter or Space activates them. |

**Clés réutilisées (déjà présentes dans les deux catalogues)**

- Titres et libellés des dialogues : `menu.settings`, `menu.settings.language`, `menu.settings.theme`,
  `menu.settings.theme.light`, `menu.settings.theme.dark`.
- Zoom, panneaux et fichiers récents : `menu.view.fit_width`, `menu.view.fit_page`, `menu.view.actual_size`,
  `menu.view.thumbnails`, `menu.view.tools`, `home.recent`, `about.close`.
- Titres de menus (dialogue des raccourcis) : `menu.file`, `menu.edit`, `menu.view`, `menu.pages`,
  `menu.security`, `menu.convert`, `menu.help`.
- Noms accessibles : les infobulles existantes, `toolbar.page_field`, `find.previous`, `find.next`, `find.close`,
  `find.placeholder`, `palette.placeholder`, `dialog.note.placeholder`, `toolbar.search_commands`,
  `a11y.document_canvas` et `a11y.thumbnail`.

Total attendu : 380 + 12 = 392 clés par catalogue, à confirmer par `scripts/check-i18n.ps1`.

---

## 11. Vérifications (Git Bash, une commande cargo à la fois)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
python -I scripts/make_fixtures.py crates/sunipdf-core/tests/fixtures
sha256sum crates/sunipdf-core/tests/fixtures/two-pages.pdf crates/sunipdf-core/tests/fixtures/encrypted-rc4.pdf  # dd186d07…, 554e55c3…
cargo test -p sunipdf-core -j 2            # étapes A et B : engine (nouveaux tests), pages, cache
cargo test -p sunipdf-desktop -j 2         # étapes C à F : annotate, app, settings, prefs, icons
cargo test -p sunipdf-i18n -j 2
cargo test --workspace -j 2
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo fmt --all -- --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-i18n.ps1
cargo test -p sunipdf-core --release -j 2 --test bench -- --ignored --nocapture   # étape G, deux fois
```

Ordre de travail recommandé : écrire d'abord chaque test de l'étape, l'exécuter et **noter son échec réel** (c'est la
reproduction du bug), puis corriger. Un test qui reste rouge n'est pas mis en `#[ignore]` : il est signalé.

Captures (`scripts/dev-screenshot.ps1`, une à la fois, après `cargo build -p sunipdf-desktop -j 2`) :

| Fichier | Commande | Ce qu'il faut voir |
|---|---|---|
| `settings.png` | `-Out settings.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands "app.settings"` | Dialogue « Paramètres » |
| `shortcuts.png` | `-Out shortcuts.png -Pdf … -Keys "{F1}"` | Liste groupée par menu ; Ctrl+Maj+D figure dans « Édition » |
| `focus.png` | `-Out focus.png -Pdf … -Keys "{TAB}{TAB}{TAB}{TAB}"` | Contour de focus visible |
| `rotated.png` | `-Out rotated.png -Pdf crates\sunipdf-core\tests\fixtures\rotated-offset.pdf -Commands "edit.find"` | Une fois le mot « MARK » saisi : surbrillance sur le texte (contrôle visuel de B4) |
| `numbers.png` | `-Pdf rotated-offset.pdf -Commands "pages.numbers"`, puis valider | Numéros en bas, horizontaux |

Ensuite, restaurer `%APPDATA%\SuniPDF\settings.json` s'il a été modifié (`docs/ENGINEERING_NOTES.md` §5).

---

## 12. Documentation à mettre à jour

- `docs/PROJECT_STATUS.md` : ligne SUNI-016 (partiel) et tableau « Dernière exécution », avec les résultats réels.
- `docs/NEXT_TASKS.md` : SUNI-016 ; il reste le corpus externe, le fuzzing et la simulation du disque plein.
- `docs/KNOWN_ISSUES.md` :
  - fichier lu entièrement en mémoire à l'ouverture ;
  - l'enregistrement par renommage ne conserve ni les attributs, ni les flux alternatifs (Mark of the Web), ni les
    ACL explicites, ni les liens physiques de l'original ;
  - curseurs diagonaux inversés à 90° et 270° ;
  - zone document en dernier dans l'ordre de tabulation ;
  - pas de sélection de texte au clavier ;
  - coupure de courant et disque plein non simulés.
- `docs/DECISIONS.md`, au prochain numéro libre, une décision par ligne :
  - « Ouverture : fichier lu en mémoire (`Engine::open` → `open_bytes`) » ;
  - « Coordonnées : moteur en espace utilisateur PDF, conversion écran unique `PageView` + `PageGeometry` livrée
    avec chaque rendu » ;
  - « Raccourcis : table `BINDINGS` vérifiée contre le registre ».
- `CHANGELOG.md`, section « Non publié » : Paramètres (Ctrl+,), Raccourcis (F1), accessibilité, corrections B1 à B10.
- `docs/USER_GUIDE.fr.md` et `docs/USER_GUIDE.en.md` : Paramètres, F1, navigation au clavier.
- `docs/FEATURE_MATRIX.md` : Paramètres, aide clavier.
- `docs/QA_PERFORMANCE.md` (étape G).
- `docs/QA_CORPUS.md` : les 7 fixtures, leur provenance (`scripts/make_fixtures.py`, licence du projet), leur
  empreinte SHA-256 et l'issue attendue de chacune.

---

## 13. Limites, points non vérifiés et hors périmètre

- **Non compilé, non exécuté.** Toutes les issues de tests ci-dessus sont des attentes. Les hypothèses ⚠ :
  - reconstruction xref par PDFium ;
  - erreur renvoyée sur un fichier vide (le garde de A1 la rend déterministe) ;
  - comportement de `rename` sur une cible en lecture seule (le refus de A2 le rend déterministe) ;
  - héritage de `/Rotate` et `/MediaBox` ;
  - points de code des glyphes ;
  - annonce du rôle `Dialog` par le Narrateur ;
  - présence, dans l'arbre AccessKit, des lignes hors champ du `ScrollArea`.
- **Coupure de courant et disque plein.** Pas d'injection de faute possible sans outillage administrateur (VHD).
  « Enregistrement interrompu » se vérifie au stade du renommage (cible verrouillée) et par construction : l'original
  n'est touché que par `rename`.
- **Hors périmètre de ce plan :**
  - corpus externe (pdf.js, à constituer avec vérification des licences) ;
  - `cargo fuzz` (nightly) ;
  - sélection de texte au clavier ;
  - thème « système » ;
  - raccourcis configurables ;
  - CLI : les défauts connus sont traités par une autre session, et la CLI hérite de A1 et A2 sans changement de code.
- **Notes (annotation `Text`) sur une page pivotée.** L'icône est placée en espace utilisateur (`annot.rs:242-259`).
  Sa position relative au clic n'est pas vérifiée : à regarder sur capture, et à signaler si elle est décalée.

---

## 14. Ordre, dépendances, effort

| Ordre | Étape | Dépend de | Propriétaire | Effort |
|---|---|---|---|---|
| 0 | Laisser aboutir les corrections en cours (`lib.rs`, `pages.rs`, `security.rs`, `app.rs`) et relocaliser les symboles | — | intégrateur | S |
| 1 | Fixtures (§2) | 0 | pdf-engine | S |
| 2 | A : ouverture en mémoire, refus 0 octet et 0 page, `write_atomic`, tests | 1 | pdf-engine, puis qa-performance pour les tests | M |
| 3 | B : `PageGeometry`, rendu, redressement, tampons, tests | 1 | rendering, pdf-editor | M |
| 4 | C : `PageView` et branchements, test | 3 | pdf-editor | M |
| 5 | D : `BINDINGS`, garde de focus, test | — (en parallèle de 2 et 3) | ui-ux | S |
| 6 | E : commandes, `prefs.rs`, `settings.rs`, i18n, tests | 5 | ui-ux | M |
| 7 | F : noms, rôles, libellés, focus, ordre, annonces, garde-fou | 6 (pour `prefs.rs` dans le garde-fou) | forms-accessibility | M |
| 8 | G : banc, mesures, `QA_PERFORMANCE.md` | 2 (l'ouverture en mémoire change les mesures) | qa-performance | S |
| 9 | Garde-fous, captures, Narrateur, documentation | tout | intégrateur | S |

Effort global : **L**. Les étapes D, E et F (fichiers `app.rs`, `shell.rs`, `prefs.rs`) d'une part, et A et B
(moteur) d'autre part, peuvent avancer en parallèle dans deux worktrees. C attend B. Une seule compilation cargo
à la fois sur ce poste.
