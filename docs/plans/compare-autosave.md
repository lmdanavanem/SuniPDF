# Plan — tranche `compare-autosave` : comparer deux PDF, autosave et récupération après plantage

Rédigé le 2026-10-14 (version 0.3.0, branche `main`). Lecture seule du dépôt : aucune commande `cargo`
n'a été exécutée ; tout ce qui suit est à compiler et tester par l'implémenteur. Chaque appel de
bibliothèque cité l'est avec fichier:ligne des sources réelles.

Abréviations de chemins :
- `PR/` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src/`
- `EG/` = `…/egui-0.36.2/src/`, `EP/` = `…/epaint-0.36.2/src/`, `EF/` = `…/eframe-0.36.2/src/`, `RFD/` = `…/rfd-0.17.2/src/`
- chemins sans préfixe = dépôt `C:/dev_suniworks/SuniPDF/`

---

## 1. Objectif et comportement visible

### 1.1 Comparer deux PDF
Correspondance PDFCraft : crate `compare` (« Comparaison de deux versions », `docs/FEATURE_MATRIX.md:55`,
`docs/PDFCRAFT_AUDIT.md:28`). Ce n'est pas une entrée de la liste « All tools » de PDFCraft ; c'est
l'équivalent de « Compare files » d'Acrobat.

Ce que voit l'utilisateur :
1. Commande **Comparer deux PDF…** (menu Fichier, panneau d'outils « Tous les outils » et « Lire », palette Ctrl+K).
2. Boîte de dialogue : *Document A (version d'origine)* = le document ouvert (état en mémoire, modifications non
   enregistrées comprises) ou un fichier ; *Document B (nouvelle version)* = un fichier (pré-rempli avec l'autre
   onglet ouvert s'il y en a un) ; bouton « Inverser A et B » ; champ mot de passe qui n'apparaît que si le
   fichier est protégé.
3. Vue de comparaison plein écran (remplace la zone document, comme la vue Organiser) :
   - bandeau de synthèse : pages modifiées / total, mots ajoutés / supprimés, « Nombre de pages : 2 → 3 »,
     liste des métadonnées modifiées (titre, auteur, sujet, mots-clés, créateur, producteur, dates, version) ;
   - liste des paires de pages à gauche (A p. 1 ↔ B p. 1 « Modifiée », B p. 3 « Ajoutée »…) ;
   - mode **Côte à côte** : page A et page B rendues, mots supprimés surlignés en rouge sur A, mots ajoutés en
     vert sur B ; bascule **Différences visuelles** : masque magenta sur B là où les pixels diffèrent, avec le
     nombre de pixels différents ;
   - mode **Texte** (diff unifié, mot à mot) : texte de la page avec les mots supprimés barrés en rouge et les
     mots ajoutés sur fond vert ; option « Uniquement les lignes modifiées » ;
   - différence précédente / suivante, zoom, « Fermer la comparaison ».
4. CLI : `sunipdf-cli compare a.pdf b.pdf [--visual]` (JSON).

### 1.2 Autosave et récupération après plantage
Correspondance PDFCraft : « autosave et récupération » (`docs/PDFCRAFT_AUDIT.md:35,67`, `docs/FEATURE_MATRIX.md:29`).

Ce que voit l'utilisateur :
1. Rien pendant le travail : toutes les 60 s (et à chaque changement d'onglet), un instantané de chaque document
   **modifié** est écrit dans `%LOCALAPPDATA%\SuniPDF\recovery\<session>\`. L'original n'est jamais touché.
2. Enregistrer, fermer le document ou quitter normalement supprime l'instantané.
3. Après un plantage (ou arrêt forcé), au lancement suivant : boîte **Récupérer des documents** listant les
   documents retrouvés (nom, chemin d'origine, nombre de pages, âge de l'instantané), cases à cocher,
   boutons **Restaurer** / **Supprimer** (avec confirmation) / **Plus tard**.
4. Un document restauré s'ouvre dans un onglet au nom d'origine, marqué modifié. Son **premier enregistrement
   passe obligatoirement par « Enregistrer sous »** (nom d'origine pré-rempli) : l'original ne peut être remplacé
   qu'avec la confirmation d'écrasement du système.
5. Paramètres : cases « Enregistrement automatique de récupération » et « Rouvrir les documents de la dernière
   session » ; commande **Effacer les fichiers récents** (menu Fichier, palette, lien sur l'accueil).

---

## 2. Faisabilité

**Verdict : oui**, hors ligne, sans nouvelle dépendance, licences inchangées.

Limites assumées (à documenter dans `KNOWN_ISSUES.md` et les guides, ne pas présenter comme « fait ») :

Comparaison
- Diff **mot à mot** dans l'ordre du texte fourni par PDFium (ordre du flux de contenu, `PR/pdf/document/page/text.rs:240-246`).
  Un paragraphe déplacé apparaît comme supprimé + ajouté ; deux PDF de producteurs différents peuvent donner du bruit.
- Pas de comparaison de la mise en forme (police, taille, couleur), des annotations, formulaires, signets, pièces
  jointes, chiffrement/permissions. Les images ne sont comparées que par le diff de pixels.
- Appariement des pages : pages au texte identique alignées par LCS, le reste apparié par position. Les pages sans
  texte (scans) s'apparient dans l'ordre : seul le diff visuel est alors pertinent (l'interface le dit).
- Diff visuel calculé **à la demande pour la paire affichée** (pas d'analyse globale dans l'interface) ; la CLI
  `--visual` analyse toutes les paires.
- Plafond LCS : 4 000 000 cellules (≈ 2000 × 2000 mots par page) ; au-delà, la page est rendue comme
  « tout supprimé / tout ajouté ».
- Sources : le document **actif** (état mémoire) ou des fichiers sur disque. Un onglet d'arrière-plan modifié est
  comparé dans sa version enregistrée.
- Pas de rapport exportable (PDF/HTML), pas d'options « ignorer la casse / la ponctuation », pas de barre de
  progression ni d'annulation (le thread moteur est occupé pendant le calcul).
- Surlignages sur pages pivotées (`/Rotate` ≠ 0) non vérifiés (même hypothèse de repère que la recherche,
  `apps/sunipdf-desktop/src/app.rs:1474-1479`) : à contrôler, voir § 10.

Autosave / récupération
- Perte maximale = l'intervalle (60 s par défaut) pour le document actif.
- Restaure le **contenu** du document, pas l'historique d'annulation ni la page / le zoom.
- Document chiffré : l'instantané reste chiffré (à prouver par test, § 8) ; le mot de passe est redemandé, il
  n'est jamais écrit dans le manifeste.
- Intervalle réglable par `settings.json` (`autosave_secs`) ou variable d'environnement, pas d'interface de réglage.
- « Rouvrir la dernière session » rouvre les fichiers depuis le disque (pas la position de lecture).

Écartés volontairement : crate `similar` (voir § 9), comparaison d'onglets suspendus par clonage de leurs octets,
commande « Récupérer… » en cours de session (le dialogue revient au lancement suivant).

---

## 3. API moteur (`crates/sunipdf-core`)

### 3.1 `src/text.rs` — mots d'une page

```rust
/// Mot d'une page : texte, rectangle englobant (points PDF, origine bas-gauche, même repère que
/// `TextMatch`) et numéro de ligne dans la page (ordre PDFium).
#[derive(Debug, Clone, PartialEq)]
pub struct Word { pub text: String, pub rect: PointRect, pub line: u32 }

impl Document<'_> {
    /// Mots d'une page dans l'ordre PDFium. Séparateurs : espaces et caractères de contrôle.
    pub fn page_words(&self, page_index: usize) -> Result<Vec<Word>, EngineError>;
}
```

Implémentation (lecture seule, aucun instantané, aucune régénération de contenu) :
- `self.page_handle(page_index)?` (`src/pages.rs:282`, déjà `pub(crate)`, renvoie `PageOutOfRange`) ;
- `page.text()?` → `PdfPage::text` (`PR/pdf/document/page.rs:377`) ;
- `text.chars()` (`PR/pdf/document/page/text.rs:102`) puis `.iter()` (`PR/pdf/document/page/text/chars.rs:163`) ;
- par caractère : `unicode_char()` (`PR/pdf/document/page/text/char.rs:109`) ; si `is_whitespace() || is_control()`
  → clore le mot courant, et si c'est `'\n'` → `line += 1` (PDFium insère `\r\n` entre les lignes) ; sinon ajouter
  le caractère et unir son `loose_bounds()` (`PR/pdf/document/page/text/char.rs:545`) au rectangle du mot ;
- accumuler le rectangle en `Option<PointRect>` (un caractère sans boîte ne doit pas étirer le mot vers (0,0)) ;
  mot sans boîte → rectangle nul, ignoré par les surlignages (`rect.right > rect.left`).
- Conversion `PdfRect → PointRect` : rendre `annot::from_pdf_rect` (`src/annot.rs:39`) `pub(crate)` et la réutiliser.
- C'est le même parcours que `text_in_rect` (`src/annot.rs:81-108`), déjà en production.

Exporter dans `src/lib.rs` : `pub use text::{PointRect, TextMatch, Word};`.

### 3.2 `src/compare.rs` (nouveau module, `pub mod compare;` + ré-exports dans `lib.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOp { Equal(usize, usize), Delete(usize), Insert(usize) }

/// Opérations transformant `a` en `b` (plus longue sous-séquence commune). Pur Rust, sans PDFium.
pub fn diff<T: PartialEq>(a: &[T], b: &[T]) -> Vec<DiffOp>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageStatus { Identical, Modified, OnlyA, OnlyB }
impl PageStatus { pub fn id(self) -> &'static str; }   // "identical" | "modified" | "only_a" | "only_b"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind { Same, Removed, Added }
#[derive(Debug, Clone, PartialEq)]
pub struct Span { pub kind: SpanKind, pub text: String }
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiffLine { pub spans: Vec<Span> }
impl DiffLine { pub fn changed(&self) -> bool; }       // au moins un span != Same

#[derive(Debug, Clone, PartialEq)]
pub struct PagePair {
    pub a: Option<usize>,            // index de page dans A (None : page seulement dans B)
    pub b: Option<usize>,
    pub status: PageStatus,          // d'après le TEXTE uniquement
    pub size_a: Option<PageSize>,
    pub size_b: Option<PageSize>,
    pub words_a: usize,
    pub words_b: usize,
    pub removed: Vec<PointRect>,     // mots supprimés, sur la page A (vide pour OnlyA)
    pub added: Vec<PointRect>,       // mots ajoutés, sur la page B (vide pour OnlyB)
    pub lines: Vec<DiffLine>,        // diff unifié ; vide si Identical
}
impl PagePair { pub fn size_changed(&self) -> bool; }

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiff { pub field: &'static str, pub a: Option<String>, pub b: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub struct CompareReport {
    pub page_count_a: usize,
    pub page_count_b: usize,
    pub metadata: Vec<FieldDiff>,    // uniquement les champs qui diffèrent
    pub pages: Vec<PagePair>,
    pub words_added: usize,
    pub words_removed: usize,
}
impl CompareReport {
    /// Pages toutes `Identical`, mêmes formats, même nombre de pages, aucune métadonnée différente.
    pub fn identical(&self) -> bool;
}

/// Compare le texte, la pagination et les métadonnées de deux documents ouverts.
pub fn compare(a: &Document<'_>, b: &Document<'_>) -> Result<CompareReport, EngineError>;

#[derive(Debug, Clone, PartialEq)]
pub struct PixelDiff {
    pub width: u32,
    pub height: u32,
    pub changed: u64,                // pixels différents au-delà de la tolérance
    pub bbox: Option<[u32; 4]>,      // x0, y0, x1, y1 (origine haut-gauche, bornes incluses) ; None si aucun
    pub mask: Vec<u8>,               // RGBA non prémultiplié : transparent si identique, magenta (255,0,255,150) sinon
}
/// Compare deux rendus. `tolerance` : écart maximal toléré par canal (0-255).
pub fn pixel_diff(a: &RenderedPage, b: &RenderedPage, tolerance: u8) -> PixelDiff;
```

Algorithmes (à implémenter tels quels) :

**`diff`** — LCS classique, ≈ 45 lignes :
1. retirer le préfixe commun puis le suffixe commun (sans chevauchement) ;
2. milieu `a'` (n) et `b'` (m) : si `n * m > 4_000_000` → `Delete` de tout `a'` puis `Insert` de tout `b'`
   (commentaire `// ponytail: plafond mémoire ; Myers O(ND) ou crate similar si des pages très denses le demandent`) ;
3. table `(n+1) × (m+1)` en `u16` (un seul `Vec<u16>`), remplie de la fin vers le début
   (`t[i][j] = t[i+1][j+1] + 1` si égal, sinon `max(t[i+1][j], t[i][j+1])`). `u16` suffit : la LCS vaut au plus
   `min(n, m) ≤ 2000` sous le plafond ;
4. parcours avant : égal → `Equal` ; sinon `Delete` si `t[i+1][j] >= t[i][j+1]`, sinon `Insert` ; puis les restes.
   Les indices renvoyés sont ceux des tranches d'origine (décalés du préfixe).

**`compare`**
1. `wa[i] = a.page_words(i)?` pour chaque page de A, `wb[j]` pour B (propager les erreurs).
2. Signature d'une page = textes des mots joints par un espace. `diff(&sig_a, &sig_b)`.
3. Parcourir les opérations : `Equal(i, j)` → vider le « trou » en cours puis paire `Identical` ; `Delete(i)` /
   `Insert(j)` → accumuler dans deux listes. Vider un trou = apparier `dels[k]` avec `ins[k]` (`Modified`), puis le
   surplus en `OnlyA` / `OnlyB`. Vider aussi à la fin.
4. Paire `Modified` : `diff(&mots_a, &mots_b)` sur le texte des mots ; `Delete(i)` → `removed.push(wa[i].rect)`,
   `Insert(j)` → `added.push(wb[j].rect)` ; compteurs globaux.
5. Diff unifié (`lines`), une passe sur les opérations : `Equal`/`Insert` prennent le mot de B, `Delete` celui de A.
   Nouvelle ligne quand le numéro de ligne B change entre deux mots de B, ou quand le numéro de ligne A change
   entre deux mots supprimés consécutifs ; après un `Equal(i, j)` mémoriser `wa[i].line` pour qu'une suppression
   sur la même ligne reste en ligne. Fusionner les spans voisins de même nature (séparés par un espace).
6. `OnlyA` : `lines` = tout le texte en `Removed`, `words_removed += n` ; `OnlyB` symétrique. Pas de rectangles
   (la page entière est concernée, le badge suffit).
7. Tailles : `a.page_size(i)` / `b.page_size(j)` (`src/lib.rs:215`).
8. Métadonnées : pour `("title", Title)`, `author`, `subject`, `keywords`, `creator`, `producer`, `created`
   (CreationDate), `modified` (ModificationDate) lire `doc.inner.metadata().get(tag)` (`PR/pdf/document.rs:296`,
   `PR/pdf/document/metadata.rs:116`, valeur `metadata.rs:48`, enum `metadata.rs:16-25`), vide → `None` ; plus
   `"version"` (`PdfDocument::version`, `PR/pdf/document.rs:227`). **Ne pas appeler `Document::properties()`** :
   elle sérialise tout le document et le relit avec lopdf pour connaître le chiffrement
   (`src/security.rs:260-279`) — trop coûteux ici. Extraire de `src/security.rs:292-294` un
   `pub(crate) fn version_label(&self) -> String` réutilisé par les deux.
   (`inner` est un champ privé de `Document` défini à la racine du crate : accessible depuis `compare.rs` comme
   il l'est depuis `text.rs`.)

**`pixel_diff`** — infaillible : canevas = `max` des largeurs et hauteurs ; un pixel hors de l'une des images
compte comme différent ; sinon différent si un canal R, G, B ou A s'écarte de plus de `tolerance`. Mettre à jour
`changed`, `bbox`, écrire le masque. Les deux pages sont rendues à la même échelle par l'appelant avec
`Document::render_page` (`src/lib.rs:224` ; `PdfPage::render_with_config` `PR/pdf/document/page.rs:569`,
`PdfBitmap::as_rgba_bytes` `PR/pdf/bitmap.rs:294`). `RenderTooLarge` protège déjà la mémoire.

Erreurs : aucune nouvelle variante d'`EngineError`. `PageOutOfRange`, `PasswordRequired`, `InvalidFormat`, `Io`
remontent tels quels (ouverture par l'appelant).

### 3.3 Autosave / récupération : **aucune nouvelle API moteur**
- Écrire l'instantané : `Document::save_as(&self, path)` (`src/pages.rs:276-280`) = `save_to_bytes`
  (`PR/pdf/document.rs:388`) + `write_atomic` (`src/pages.rs:63-79`, crée le dossier, `.tmp` voisin puis `rename`).
  Méthode `&self` : le document en mémoire n'est pas modifié, donc **pas d'entrée d'annulation, pas
  d'invalidation du cache de rendu, pas de `DocumentChanged`**. Même chemin de sérialisation que « Enregistrer »
  et que l'annulation (`Document::snapshot`, `src/pages.rs:246`) : les objets édités ont déjà été régénérés par
  `edit.rs` (`regenerate_content`, `src/edit.rs:242,261,291,317,350,375,395,450`).
- Restaurer : `Engine::open_bytes(bytes, password, original_path)` (`src/lib.rs:143-151`,
  `Pdfium::load_pdf_from_byte_vec` `PR/pdfium.rs:330`) : le document vit en mémoire, garde le chemin d'origine,
  et le fichier de récupération n'est pas tenu ouvert (il peut être déplacé ou supprimé). À l'inverse
  `Engine::open` garde le fichier ouvert (`PR/pdfium.rs:371-377`, `File::open`) : ne pas l'utiliser pour restaurer.
- Chiffrement : `save_to_writer` passe `flags = 0` à `FPDF_SaveAsCopy` (`PR/pdf/document.rs:328-357`), donc sans
  `FPDF_REMOVE_SECURITY`. Le test existant `protect_unprotect_and_metadata` (`crates/sunipdf-core/tests/engine.rs:462`)
  vérifie déjà `properties().encrypted` sur un instantané : le dictionnaire `/Encrypt` est conservé. Le test
  § 8.2-6 doit le prouver pour l'instantané de récupération ; **s'il échoue, ne pas livrer l'autosave des
  documents protégés** (sauter l'autosave quand `doc.password().is_some()` et le documenter).

Le manifeste, les dossiers de session et le verrou sont de la logique d'application (JSON + `serde`, absent de
`sunipdf-core`) : ils vivent dans `apps/sunipdf-desktop/src/recovery.rs` (§ 5.5).

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

Le moteur sait ouvrir plusieurs documents à la fois (`Engine::open` renvoie un `Document<'_>` indépendant ;
`insert_pages_from` et `merge` le font déjà, `crates/sunipdf-core/src/pages.rs:96-99,195`). **Il n'y a donc pas
besoin de suspendre l'onglet actif** : la comparaison ouvre ses deux documents à côté de `state.document`.
`tabs.rs` (Suspend/Resume) n'est pas modifié par la comparaison.

Ajouts :

```rust
pub enum CompareSource {
    /// État en mémoire du document actif (instantané rechargé : indépendant des modifications ultérieures).
    Active,
    File { path: PathBuf, password: Option<String> },
}

// Request (après `Batch`, worker.rs:217)
Compare { a: CompareSource, b: CompareSource },
ComparePage { a: Option<usize>, b: Option<usize>, scale: f32, visual: bool, generation: u64 },
CompareClose,
Autosave { tab: u64, path: PathBuf },
// Request::Open gagne un champ :
Open { path: PathBuf, password: Option<String>, restore_as: Option<PathBuf> },

// Response
Compared(Box<sunipdf_core::CompareReport>),
CompareFailed { second: bool, error: EngineError },        // second = l'erreur concerne B
ComparePage { generation: u64, a: Option<RenderedPage>, b: Option<RenderedPage>, diff: Option<PixelDiff> },
Autosaved { tab: u64, path: PathBuf, result: Result<(), EngineError> },
```

`State` (worker.rs:314-321) gagne `compare: Option<(Document<'a>, Document<'a>)>`.

Traitement dans `run` (worker.rs:661-822) :
- `Compare` : `open_source(&engine, state.document.as_ref(), src)` pour A puis B —
  `File` → `engine.open(&path, password.as_deref())` ; `Active` →
  `engine.open_bytes(doc.snapshot()?, doc.password(), &doc.info().path)` (erreur `Other("no document")` sans
  document). Échec → `CompareFailed { second, error }`. Succès → `sunipdf_core::compare::compare(&a, &b)`,
  stocker `state.compare = Some((a, b))`, répondre `Compared(Box::new(report))`.
- `ComparePage` : sans session → `continue`. Rendre `a.render_page(pa, scale)` / `b.render_page(pb, scale)` pour
  les côtés présents (**hors `state.cache`**, dont la clé est (page, échelle) du document actif) ; si `visual` et
  les deux présents → `pixel_diff(&ra, &rb, 16)`. Erreur de rendu → `log::warn!` et côté `None`.
- `CompareClose` : `state.compare = None; continue`.
- `Autosave` : toujours répondre (sinon l'UI attend indéfiniment) :
  `result = match &state.document { Some(doc) => doc.save_as(&path), None => Err(EngineError::Other("no document".into())) }`.
  Ce n'est **pas** un `EditOp` : pas d'instantané d'annulation, pas d'`invalidate()`.
- `Open { path, password, restore_as }` (worker.rs:662-671) :
  ```rust
  let opened = match &restore_as {
      Some(original) => std::fs::read(&path).map_err(EngineError::Io)
          .and_then(|bytes| engine.open_bytes(bytes, password.as_deref(), original)),
      None => engine.open(&path, password.as_deref()),
  };
  // Err(error) => Response::OpenFailed { path: restore_as.unwrap_or(path), error }
  ```
  (le chemin renvoyé en cas d'échec est le chemin d'origine : c'est lui qu'affiche la demande de mot de passe).

Pas de nouvel `EditOp`, pas de nouveau `BatchOp` : comparer et sauvegarder ne modifient pas le document.
Les requêtes sont traitées dans l'ordre d'envoi : une `Autosave` envoyée avant `Suspend` porte bien sur le
document qui était actif.

---

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Commandes (`crates/sunipdf-core/src/command.rs`)

| Variante | id | clé libellé | raccourci | `needs_document` | menu | ToolTab |
|---|---|---|---|---|---|---|
| `CompareFiles` | `file.compare` | `menu.file.compare` | aucun | `false` | `File` (après `CombineFiles`, l. 142-149) | `All` (après `CombineFiles`), `Read` (après `Properties`) |
| `ClearRecent` | `file.clear_recent` | `menu.file.clear_recent` | aucun | `false` | `File` (avant `Quit`, l. 158) | — |

À mettre à jour dans le même commit (correspondances exhaustives et tests) :
- `command.rs:648` : `Command::in_menu(Menu::File).count()` passe de 7 à **9** ;
- `shell.rs:246-309` `command_icon` : `CompareFiles => (glyph::DUPLICATE, "⇄")`, `ClearRecent => (glyph::ERASE, "⌫")`
  (glyphes existants de `icons.rs` ; aucun nouveau point de code à deviner) ;
- `shell.rs:46-145` `ToolTab::commands` ; `command_color` retombe sur `CAT_FILE` par le menu (rien à faire) ;
- `app.rs:793-1011` `execute` : `CompareFiles => self.open_compare_dialog()`,
  `ClearRecent => { self.settings.recent.clear(); self.persist_settings(); }` ;
- `shell.rs:459-465` : séparateurs du menu Fichier `&[Command::Save, Command::ClearRecent]`.

### 5.2 Nouveau fichier `src/compare.rs` (déclarer `mod compare;` dans `main.rs:3-15`)

```rust
#[derive(Default)]
pub struct CompareState {
    pub form: Option<CompareForm>,          // boîte de configuration ouverte
    pub active: bool,                       // vue de comparaison affichée
    pub report: Option<Box<CompareReport>>,
    pub names: (String, String),            // noms de fichiers A et B
    pub pair: usize,
    pub text_mode: bool,                    // false = côte à côte
    pub show_text: bool,                    // surlignages de mots (vrai à l'ouverture)
    pub show_visual: bool,
    pub only_changes: bool,
    pub zoom: f32,                          // 1.0 = ajusté à la largeur
    pub generation: u64,
    pub in_flight: Option<(usize, f32, bool)>,
    pub rendered: Option<(usize, f32, bool)>,
    pub tex: [Option<egui::TextureHandle>; 3],   // A, B, masque
    pub visual: HashMap<usize, (u64, u64)>,      // paire → (pixels différents, pixels totaux)
}
pub struct CompareForm {
    pub a_active: bool,
    pub a: Option<PathBuf>, pub b: Option<PathBuf>,
    pub pw: [String; 2], pub need_pw: [bool; 2], pub wrong: [bool; 2],
}
```
Champ `pub compare: crate::compare::CompareState` dans `SuniApp` (`app.rs:82-127`), **hors `TabState`** (état global).

Fonctions (`impl SuniApp`) :
- `open_compare_dialog()` : `a_active = self.document.is_some()` ; `b` = chemin du premier onglet autre que
  l'actif s'il existe.
- `compare_dialog(ctx)` : `dialog::show` (`dialog.rs:72`) avec `Spec::new("compare", tr("compare.title"), …)`
  `.subtitle("compare.subtitle").icon(glyph::DUPLICATE, "⇄").width(560.0)`. Corps : deux blocs
  (`dialog::field_label`, radios « Document ouvert : {name} » / « Autre fichier », chemin choisi, bouton
  « Parcourir… », champ mot de passe `TextEdit::singleline(..).password(true)` si `need_pw`, message
  `error.password_wrong` si `wrong`), bouton « Inverser A et B ». Pied : `dialog::primary("compare.ok")` (actif si
  les deux côtés sont définis) ou Entrée ; `dialog::secondary("dialog.password.cancel")` ou Échap.
  Les `rfd::FileDialog::pick_file()` (`RFD/file_dialog.rs:122`) se lancent **après** le retour de `dialog::show`
  (drapeau posé dans la fermeture), comme `tool_split_submit` (`tools.rs:462-463`).
  Validation : `self.tools.running = Some("compare.running")`, `Request::CompareClose` puis
  `Request::Compare { a, b }`, fermer la boîte en conservant le formulaire pour une éventuelle erreur.
- `on_compared(report)` : `tools.running = None`, stocker, `active = true`, `pair` = première paire non
  `Identical` (sinon 0), `show_text = true`, `zoom = 1.0`, vider textures et `rendered`.
- `on_compare_failed(second, error)` : `tools.running = None` ; `PasswordRequired` → rouvrir le formulaire avec
  `need_pw[côté] = true` (et `wrong` si un mot de passe avait été saisi) ; sinon `self.error = Some(error_key(&error))`.
- `compare_view(ctx, ui)` : barre du haut (titre `compare.view.title`, synthèse, précédent / suivant, bascules
  `ui.toggle_value` (`EG/ui.rs:1875`) Côte à côte / Texte, Différences de texte, Différences visuelles, zoom avec
  les glyphes et libellés existants `menu.view.zoom_in/out`, « Fermer la comparaison ») ; à gauche liste des paires
  (`ScrollArea::vertical().show_rows`, `EG/containers/scroll_area.rs:375,983`, lignes `ui.selectable_label`
  `EG/ui.rs:1929`) précédée du nombre de pages et des métadonnées modifiées (libellés existants
  `properties.doc_title`, `.author`, `.subject`, `.keywords`, `.creator`, `.producer`, `.created`, `.modified`,
  `.version`) ; à droite le contenu :
  - **Côte à côte** : `ScrollArea::both()` (`scroll_area.rs:381`) contenant les deux pages (défilement commun
    = défilement synchronisé gratuit). Échelle logique `k = zoom * largeur_colonne / max(largeur_a, largeur_b)`
    (px logiques par point) ; échelle de rendu `quantize(k * ctx.pixels_per_point())` — rendre `quantize`
    (`app.rs:1920`) `pub(crate)`. Demander `Request::ComparePage` quand `(pair, échelle, show_visual) != rendered`
    et qu'aucune requête n'est en vol (même schéma que `request_render_if_needed`, `app.rs:558-572`).
    Fond blanc + `egui::Image::new(SizedTexture)` avec `alt_text(tr_args("a11y.compare_page", …))` (comme
    `app.rs:1129-1136`). Surlignages si `show_text` : même calcul que `paint_matches` (`app.rs:1474-1479`) avec
    `painter.rect_filled` (`EG/painter.rs:397`), rouge `theme::ERROR` alpha ≈ 90 sur A, vert `theme::SUCCESS`
    alpha ≈ 90 sur B. Masque si `show_visual` : `painter.image(tex.id(), rect, uv 0..1, Color32::WHITE)`
    (`EG/painter.rs:447`) posé sur la page B, taille logique = taille du masque en pixels / `pixels_per_point`.
    Côté absent → cadre avec `compare.missing_page`. Sous les pages : `compare.visual_count` / `compare.visual_none`,
    `compare.size_changed`, `compare.no_text` si `words_a == 0 && words_b == 0`.
  - **Texte** : `ScrollArea::vertical()`, une étiquette par `DiffLine` construite avec
    `egui::text::LayoutJob::append` (`EP/text/text_layout_types.rs:200`) et `TextFormat { color, background,
    strikethrough, ..Default::default() }` (`text_layout_types.rs:478-541`), passée à `ui.label`
    (`EG/ui.rs:1696` ; `impl From<LayoutJob> for WidgetText`, `EG/widget_text.rs:866`). Supprimé = rouge barré
    sur fond rouge pâle ; ajouté = fond vert pâle. Filtre `only_changes` = `DiffLine::changed()`. Légende
    `compare.legend.removed` / `.added`.
- `on_compare_page(ctx, generation, a, b, diff)` : ignorer si `generation` périmée ; charger les textures
  `ctx.load_texture(…, ColorImage::from_rgba_unmultiplied(..), TextureOptions::LINEAR)`
  (`EG/context.rs:2390`, `EP/image.rs:113`, comme `app.rs:391-400`) ; mémoriser `visual[pair]`.
- `close_compare()` : `active = false`, vider rapport et textures, `Request::CompareClose`.

Câblage dans `app.rs` :
- `handle_responses` (l. 327-539) : quatre nouveaux bras (`Compared`, `CompareFailed`, `ComparePage`, `Autosaved`).
- `handle_input` : ajouter `self.compare.form.is_some() || self.recovery.prompt.is_some()` à la garde des
  boîtes modales (l. 591-598). Juste après, si `self.compare.active` : consommer Échap (fermer), PageSuiv / →
  (paire suivante), PagePréc / ← (précédente), `T` (mode texte), `V` (visuel), `Ctrl+K`, puis `return` (sinon ces
  touches partent vers le document masqué : `V` = outil Sélection l. 739, `T` = outil Texte l. 745).
- `ui()` (l. 1762-1885) : `let comparing = self.compare.active;` ; ajouter `&& !comparing` aux panneaux
  recherche (l. 1784), signets (l. 1803), commentaires (l. 1814), rail et miniatures (l. 1825-1838) ; panneau
  central : `if comparing { error_banner + compare_view } else if …organize… ` ; ajouter
  `self.compare_dialog(&ctx); self.recovery_dialog(&ctx);` à la liste des boîtes (l. 1875-1884).
- Navigation : appeler `close_compare()` au début de `open_path` (l. 223), `switch_to` (`tabs.rs:137`) et dans
  le clic « Accueil » (`shell.rs:347-351`) — toute navigation ferme la comparaison.
- `status_bar` (`shell.rs:1181-1188`, branche sans document) : afficher aussi `self.tools.running` (la
  comparaison peut tourner sans document ouvert).

Onglets et annulation : la comparaison ne lit que des copies ; elle ne touche ni `state.document`, ni
l'historique, ni l'état `dirty`. Elle survit aux modifications du document actif (instantané figé au lancement) ;
relancer la commande pour rafraîchir.

### 5.3 Paramètres (`src/settings.rs`)
Ajouter à `Settings` (l. 9-16, `#[serde(default)]` déjà présent : les anciens `settings.json` restent lisibles) :
```rust
/// Intervalle d'enregistrement automatique de récupération, en secondes (0 = désactivé).
pub autosave_secs: u32,            // défaut 60
pub reopen_last_session: bool,     // défaut false
/// Chemins des onglets ouverts lors de la dernière utilisation.
pub session: Vec<String>,
```
Sous-menu Paramètres (`shell.rs:471-498`) : deux `ui.checkbox` (`EG/ui.rs:1866`) —
`menu.settings.autosave` (bascule `autosave_secs` entre 0 et 60) et `menu.settings.reopen_session` — suivis de
`persist_settings()`. Accueil (`shell.rs:832`) : à côté du titre « Récents », bouton sans cadre
`home.recent.clear` (visible si la liste n'est pas vide) → `execute(Command::ClearRecent)`.

Session : `fn sync_session(&mut self)` recalcule `settings.session` depuis `self.tabs.tabs[*].path` et persiste
si la liste a changé ; **ne rien faire tant que `pending_files` n'est pas vide** (réouverture en cours).
Appels : fin du bras `Opened` (après `register_opened_tab`, `app.rs:350`), fin de `close_document` (l. 265), bras
`Saved`. Au démarrage (`SuniApp::new`, l. 180) : si `initial_files` est vide et `reopen_last_session`, alors
`pending_files = settings.session` filtré par `Path::is_file()`.
Correction incluse (1 ligne, nécessaire à la session et au manifeste) : dans `Response::Saved` (l. 493-508),
mettre aussi à jour `self.tabs.tabs[actif].path` (aujourd'hui le titre d'onglet reste l'ancien nom après
« Enregistrer sous » jusqu'au prochain changement d'onglet, `shell.rs:357`).

### 5.4 Onglets (`src/tabs.rs`)
`Tab` (l. 27-34) gagne `pub id: u64` (stable, sert de nom au fichier de récupération), `pub autosave_stale: bool`
(modifié depuis le dernier instantané) et `pub recovered: bool`. `TabsState` gagne `next_id: u64`
(incrémenté dans `register_opened_tab`, l. 207, et dans la création de repli de `suspend_active`, l. 122 ;
premier identifiant = 1).

### 5.5 Nouveau fichier `src/recovery.rs` (`mod recovery;`)

Disposition sur disque (un dossier **par session d'application**, donc aucun manifeste partagé entre instances) :
```
%LOCALAPPDATA%\SuniPDF\recovery\
  <pid>-<millisecondes unix>\
    lock            fichier tenu ouvert sans partage pendant toute la vie du processus
    manifest.json   { "version": 1, "entries": [ { "tab": 1, "original": "C:\\docs\\a.pdf", "saved_unix": 1760438400, "pages": 12 } ] }
    1.pdf           instantané de l'onglet 1 (écrit par write_atomic)
```
Racine : `SUNIPDF_RECOVERY_DIR` si définie (tests, captures), sinon `LOCALAPPDATA` + `SuniPDF\recovery`
(même style que `Settings::path`, `settings.rs:48-50`). Intervalle : `SUNIPDF_AUTOSAVE_SECS` si définie, sinon
`settings.autosave_secs`. Ces deux variables rejoignent `SUNIPDF_STARTUP_COMMANDS` (`app.rs:157`) et
`SUNIPDF_PDFIUM_DIR` (`crates/sunipdf-core/src/lib.rs:85`) comme réglages de développement / QA.

Détection « session morte » sans dépendance : le verrou.
`OpenOptions::new().write(true).create(true).share_mode(0).open(dir.join("lock"))` avec
`std::os::windows::fs::OpenOptionsExt::share_mode` (stable depuis 1.10 ; documentation locale
`~/.rustup/toolchains/stable-x86_64-pc-windows-msvc/share/doc/rust/html/std/os/windows/fs/trait.OpenOptionsExt.html`,
Rust 1.99 : « Removing any of the flags will prevent other processes from performing the corresponding operation
until the file handle is closed » ; le composant `rust-src` n'est pas installé, pas de ligne source à citer).
- La session courante garde ce `File` : tant que le processus vit, personne d'autre ne peut ouvrir `lock`.
  Windows ferme le descripteur à la mort du processus, plantage compris.
- Au démarrage, pour chaque autre sous-dossier : la même ouverture **échoue** si l'instance propriétaire tourne
  (on l'ignore) et **réussit** si elle est morte ; le `File` obtenu est conservé = revendication, ce qui empêche
  deux nouvelles instances de proposer la même récupération.
- Indispensable : rien n'empêche plusieurs instances de SuniPDF (`main.rs` n'a pas de garde mono-instance) ; sans
  verrou, une deuxième instance prendrait les instantanés de la première pour un plantage.

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry { pub tab: u64, pub original: String, pub saved_unix: u64, pub pages: usize }
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Manifest { version: u32, entries: Vec<Entry> }

pub struct Orphan { pub dir: PathBuf, pub entry: Entry }          // fichier = dir/<tab>.pdf
pub struct Prompt { pub items: Vec<(Orphan, bool)>, pub confirm_discard: bool }

pub struct RecoveryState {
    pub prompt: Option<Prompt>,
    pub to_restore: Vec<Orphan>,
    pub in_flight: bool,
    pub last_tick: std::time::Instant,
    pub warned: bool,
    /* privé : racine, session courante (dossier, verrou, manifeste), revendications (dossier, File) */
}
impl RecoveryState {
    pub fn start() -> Self;                                  // racine + scan → `prompt`
    pub fn interval(&self, settings: &Settings) -> u32;      // variable d'environnement sinon réglage
    pub fn slot(&mut self, tab: u64) -> Option<PathBuf>;     // crée dossier + verrou au premier besoin ; None si indisponible
    pub fn record(&mut self, entry: Entry);                  // remplace l'entrée de l'onglet, écrit le manifeste (write_atomic)
    pub fn forget(&mut self, tab: u64);                      // supprime <tab>.pdf et l'entrée
    pub fn adopt(&mut self, orphan: &Orphan, tab: u64, pages: usize) -> std::io::Result<()>; // rename vers la session courante + record
    pub fn discard(&mut self, orphan: &Orphan);              // supprime l'instantané orphelin
    pub fn release_claims(&mut self);                        // libère les verrous ; supprime les dossiers orphelins sans .pdf
    pub fn clear_session(&mut self);                         // libère le verrou puis remove_dir_all
}
pub fn scan(root: &Path) -> (Vec<Orphan>, Vec<(PathBuf, std::fs::File)>);   // testable sans UI
```
Règles : `scan` ignore les entrées dont le `.pdf` manque et supprime les dossiers revendiqués sans entrée valide ;
le manifeste est écrit **après** le PDF (un plantage entre les deux laisse un PDF non référencé, nettoyé avec le
dossier) ; toutes les écritures passent par `sunipdf_core::write_atomic`. Fonctions petites, erreurs d'E/S
journalisées (`log::warn!`) : l'autosave ne doit jamais bloquer le travail.

Câblage (`app.rs`, `tabs.rs`) — champs `pub recovery: RecoveryState` et `restoring: Option<Orphan>` dans `SuniApp` :

| Événement | Action |
|---|---|
| `Response::DocumentChanged` (`app.rs:422`) | onglet actif : `autosave_stale = true` |
| chaque frame, après `handle_responses` | `autosave_tick(ctx)` (ci-dessous) |
| `suspend_active` (`tabs.rs:108`), avant `take_tab` | `request_autosave(true)` : un onglet modifié part en arrière-plan avec un instantané à jour |
| `Response::Autosaved` | `in_flight = false` ; succès et onglet encore présent **et modifié** → `autosave_stale = false`, `record(Entry)` ; onglet fermé ou enregistré entre-temps → `forget(tab)` ; échec → `log::warn!` et, une seule fois par session, `self.error = Some("error.autosave_failed")` |
| `Response::Saved` (`app.rs:493`) | `forget(id)`, `recovered = false`, `autosave_stale = false`, `tab.path = path` |
| `close_document` (`app.rs:239`), avant `on_active_closed` | `forget(id de l'onglet actif)` |
| sortie propre | `impl eframe::App` : `fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) { self.recovery.clear_session(); }` (signature avec la feature `glow` activée par le workspace : `EF/epi.rs:222` ; ré-export `EF/lib.rs:159` ; appelée à l'arrêt normal `EF/native/glow_integration.rs:452`, jamais lors d'un plantage) |

```rust
fn autosave_tick(&mut self, ctx: &egui::Context) {
    let secs = self.recovery.interval(&self.settings);
    if secs == 0 || !self.dirty { return; }
    let every = std::time::Duration::from_secs(secs as u64);
    ctx.request_repaint_after(every);                       // egui est réactif : sans cela, pas de frame sans événement
    if self.recovery.last_tick.elapsed() < every || ctx.input(|i| i.pointer.any_down()) { return; }
    self.recovery.last_tick = std::time::Instant::now();
    self.request_autosave(false);
}
/// `force` : ignorer `in_flight` (changement d'onglet). Conditions : intervalle > 0, onglet actif, `dirty`,
/// `autosave_stale`, pas d'enregistrement en cours (`saving`), `slot(id)` disponible.
fn request_autosave(&mut self, force: bool);
```
(`Context::request_repaint_after` `EG/context.rs:1872` ; `PointerState::any_down` `EG/input_state/mod.rs:1414` :
pas d'instantané au milieu d'un glissement.) Coût d'un instantané = celui pris avant **chaque** modification par
`State::edit` (`worker.rs:376`) : l'utilisateur le paie déjà à chaque édition.

Ordre des messages : les requêtes sont traitées et répondues dans l'ordre. Un `DocumentChanged` reçu **après**
`Autosaved` remet `autosave_stale` à vrai ; reçu **avant**, la modification est contenue dans l'instantané.

### 5.6 Dialogue de récupération et restauration
`recovery_dialog(ctx)` (dans `recovery.rs`) via `dialog::show` : `Spec::new("recovery", tr("recovery.title"), …)`
`.subtitle(tr("recovery.subtitle")).icon(glyph::WARNING, "!").width(560.0)`. Corps : `recovery.text`, puis
`ScrollArea::vertical().max_height(240.0)` (`EG/containers/scroll_area.rs:432`) avec, par document : case à
cocher, nom de fichier (semi-gras), chemin d'origine et `recovery.item` (pages + âge : `recovery.age_minutes` /
`_hours` / `_days`, calcul entier sur `saved_unix`, pas de bibliothèque de dates), `recovery.original_missing` si
l'original n'existe plus ; si `confirm_discard` : `recovery.discard_warning` en rouge. Pied (le premier bouton
ajouté est à droite, `dialog.rs:70-71`) : `dialog::primary("recovery.restore")` ou Entrée ; `dialog::danger` —
premier clic `recovery.discard` (arme `confirm_discard`), second clic `recovery.discard_confirm` (supprime la
sélection) ; `dialog::secondary("recovery.later")` ou Échap.
- **Restaurer** : entrées cochées → `to_restore` ; les autres restent sur disque (reproposées au prochain
  lancement) ; fermer la boîte ; `open_next_pending()`.
- **Supprimer** : `discard` des entrées cochées ; la boîte se ferme s'il n'en reste pas.
- **Plus tard** : `release_claims()`, rien n'est supprimé.

File d'ouverture : `open_next_pending` (`app.rs:782-787`) devient : ne rien faire tant que `recovery.prompt` est
ouvert ; sinon dépiler `to_restore` (→ `open_recovered`) avant `pending_files`. Condition de `Opened`
(l. 358-364) : lancer les commandes de démarrage seulement quand `pending_files` **et** `to_restore` sont vides.
En restaurant une entrée, retirer de `pending_files` le même chemin d'origine (comparaison insensible à la casse
et aux séparateurs, comme `push_recent`, `settings.rs:39-46`) pour ne pas ouvrir l'original et sa version récupérée.

`open_recovered(orphan)` : mêmes gardes que `open_path` (l. 224-234), `self.opening = Some(original)`,
`self.restoring = Some(orphan)`, `Request::Open { path: fichier de récupération, password: None, restore_as: Some(original) }`.
- `Response::Opened` (l. 340-365) : `let restored = self.restoring.take();` ; après `register_opened_tab`,
  `self.dirty = restored.is_some()` ; si restauré : `tab.dirty = true`, `tab.recovered = true`,
  `recovery.adopt(&orphan, tab.id, pages)` (le fichier **déménage** dans la session courante : pas de fenêtre
  sans instantané), `status_msg = tr("status.recovered")` ; quand `to_restore` est vide → `release_claims()`.
- `Response::OpenFailed` (l. 366-387) : mot de passe requis → la boîte existante s'ouvre avec le nom d'origine ;
  autre erreur → `restoring = None` (l'instantané reste sur disque), bannière d'erreur, puis
  `open_next_pending()` (aujourd'hui la file s'arrête sur un échec).
- `password_dialog` (l. 1647-1655) : la nouvelle tentative reconstruit
  `Request::Open { path, password, restore_as }` à partir de `self.restoring` ; Annuler → `restoring = None` puis
  `open_next_pending()`. Tous les autres envois de `Request::Open` passent `restore_as: None`.

**Jamais d'écrasement silencieux** : dans `save_document` (l. 1560),
`let ask_path = ask_path || onglet_actif.recovered;`. Le premier enregistrement d'un document restauré ouvre
donc « Enregistrer sous » (`rfd::FileDialog::save_file`, `RFD/file_dialog.rs:172`) avec le nom d'origine
pré-rempli ; remplacer l'original exige la confirmation d'écrasement de la boîte système. rfd ne modifie pas les
options de la boîte d'enregistrement Windows (`RFD/backend/win_cid/file_dialog/dialog_ffi.rs:300-309`) ;
l'invite d'écrasement est le comportement par défaut de `IFileSaveDialog` selon la documentation Microsoft —
**non vérifiable dans les sources locales, à confirmer au scénario manuel § 8.5**.

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Une sous-commande (21 au total). Rien pour l'autosave : c'est une fonction de l'application.

```rust
/// Compare two PDFs: page pairing, word-level text changes, metadata; --visual adds a pixel comparison.
Compare {
    a: PathBuf,
    b: PathBuf,
    #[arg(long)] password_a: Option<String>,
    #[arg(long)] password_b: Option<String>,
    /// Also render every page pair and count differing pixels.
    #[arg(long)] visual: bool,
    /// Pixels per PDF point used by --visual.
    #[arg(long, default_value_t = 1.0)] scale: f32,
    /// Per-channel tolerance (0-255) used by --visual.
    #[arg(long, default_value_t = 16)] tolerance: u8,
},
```
Exécution : `engine.open` × 2, `sunipdf_core::compare::compare`, et avec `--visual`, pour chaque paire complète,
`render_page` × 2 + `pixel_diff`. Code de sortie 0 que les documents diffèrent ou non (le résultat est dans le
JSON) ; erreurs avec les codes existants (`main.rs:658-669`). Pages en base 1 comme les autres commandes.

```json
{
  "identical": false,
  "page_count": { "a": 2, "b": 3 },
  "metadata": [ { "field": "title", "a": "SuniPDF fixture", "b": "SuniPDF fixture v2" } ],
  "words": { "added": 3, "removed": 0 },
  "pages": [
    { "a": 1, "b": 1, "status": "modified", "added": 1, "removed": 0, "size_changed": false,
      "changes": [ { "kind": "added", "text": "revised" } ],
      "visual": { "changed_pixels": 1840, "ratio": 0.0038, "bbox": [255, 74, 330, 97] } },
    { "a": 2, "b": 2, "status": "identical", "added": 0, "removed": 0, "size_changed": false, "changes": [],
      "visual": { "changed_pixels": 30000, "ratio": 0.0619, "bbox": [72, 440, 371, 539] } },
    { "a": null, "b": 3, "status": "only_b", "added": 2, "removed": 0, "size_changed": false,
      "changes": [ { "kind": "added", "text": "Annexe ajoutee" } ] }
  ]
}
```
`changes` = spans `Removed` / `Added` de `PagePair::lines`, dans l'ordre. `visual` présent seulement avec
`--visual` et quand les deux pages existent. Avec `--visual`, `identical` exige aussi zéro pixel différent.
(Les nombres de la page 1 sont illustratifs.)

---

## 7. i18n — nouvelles clés (à ajouter à `assets/i18n/fr-FR.json` **et** `en-US.json`)

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.compare` | Comparer deux PDF… | Compare two PDFs… |
| `menu.file.clear_recent` | Effacer les fichiers récents | Clear recent files |
| `menu.settings.autosave` | Enregistrement automatique de récupération | Autosave for crash recovery |
| `menu.settings.reopen_session` | Rouvrir les documents de la dernière session | Reopen documents from the last session |
| `home.recent.clear` | Effacer la liste | Clear list |
| `compare.title` | Comparer deux PDF | Compare two PDFs |
| `compare.subtitle` | Texte, pages, métadonnées et apparence. | Text, pages, metadata and appearance. |
| `compare.doc_a` | Document A (version d'origine) | Document A (original version) |
| `compare.doc_b` | Document B (nouvelle version) | Document B (new version) |
| `compare.use_active` | Document ouvert : {name} | Open document: {name} |
| `compare.use_file` | Autre fichier | Another file |
| `compare.browse` | Parcourir… | Browse… |
| `compare.no_file` | Aucun fichier choisi | No file chosen |
| `compare.pick_a` | Choisir le document A | Choose document A |
| `compare.pick_b` | Choisir le document B | Choose document B |
| `compare.password` | Mot de passe | Password |
| `compare.swap` | Inverser A et B | Swap A and B |
| `compare.ok` | Comparer | Compare |
| `compare.running` | Comparaison en cours… | Comparing… |
| `compare.view.title` | Comparaison : {a} ↔ {b} | Comparison: {a} ↔ {b} |
| `compare.summary` | {changed} page(s) modifiée(s) sur {total} · {added} mot(s) ajouté(s) · {removed} mot(s) supprimé(s) | {changed} of {total} page(s) changed · {added} word(s) added · {removed} word(s) removed |
| `compare.identical` | Aucune différence de texte, de pagination ni de métadonnées. | No text, page or metadata differences. |
| `compare.page_count` | Nombre de pages : {a} → {b} | Page count: {a} → {b} |
| `compare.metadata` | Métadonnées modifiées | Changed metadata |
| `compare.metadata.empty` | (vide) | (empty) |
| `compare.pair` | A p. {a} ↔ B p. {b} | A p. {a} ↔ B p. {b} |
| `compare.pair.only_a` | A p. {a} | A p. {a} |
| `compare.pair.only_b` | B p. {b} | B p. {b} |
| `compare.status.identical` | Texte identique | Same text |
| `compare.status.modified` | Modifiée | Changed |
| `compare.status.only_a` | Supprimée | Removed |
| `compare.status.only_b` | Ajoutée | Added |
| `compare.size_changed` | Format de page modifié | Page size changed |
| `compare.no_text` | Aucun texte sur cette page : utilisez les différences visuelles. | No text on this page: use the visual differences. |
| `compare.missing_page` | Page absente de ce document | Page missing from this document |
| `compare.mode.side` | Côte à côte | Side by side |
| `compare.mode.text` | Texte | Text |
| `compare.show_text` | Différences de texte | Text differences |
| `compare.show_visual` | Différences visuelles | Visual differences |
| `compare.visual_count` | {count} pixel(s) différent(s) ({percent} %) | {count} differing pixel(s) ({percent}%) |
| `compare.visual_none` | Aucune différence visuelle | No visual difference |
| `compare.only_changes` | Uniquement les lignes modifiées | Changed lines only |
| `compare.prev` | Différence précédente | Previous difference |
| `compare.next` | Différence suivante | Next difference |
| `compare.close` | Fermer la comparaison | Close comparison |
| `compare.legend.removed` | Supprimé (A) | Removed (A) |
| `compare.legend.added` | Ajouté (B) | Added (B) |
| `a11y.compare_page` | Document {side}, page {page} | Document {side}, page {page} |
| `recovery.title` | Récupérer des documents | Recover documents |
| `recovery.subtitle` | SuniPDF ne s'est pas fermé correctement. | SuniPDF did not shut down properly. |
| `recovery.text` | Des modifications non enregistrées ont été retrouvées. Les fichiers d'origine n'ont pas été modifiés. | Unsaved changes were found. The original files have not been modified. |
| `recovery.item` | {pages} page(s) · {age} | {pages} page(s) · {age} |
| `recovery.age_minutes` | instantané d'il y a {n} min | snapshot from {n} min ago |
| `recovery.age_hours` | instantané d'il y a {n} h | snapshot from {n} h ago |
| `recovery.age_days` | instantané d'il y a {n} j | snapshot from {n} d ago |
| `recovery.original_missing` | Fichier d'origine introuvable | Original file not found |
| `recovery.restore` | Restaurer | Restore |
| `recovery.discard` | Supprimer | Delete |
| `recovery.discard_confirm` | Supprimer définitivement | Delete permanently |
| `recovery.discard_warning` | Les instantanés sélectionnés seront supprimés définitivement. | The selected snapshots will be permanently deleted. |
| `recovery.later` | Plus tard | Later |
| `status.recovered` | Document récupéré : enregistrez-le pour conserver les modifications. | Recovered document: save it to keep the changes. |
| `error.autosave_failed` | L'enregistrement automatique de récupération a échoué (espace disque ou droits). | Recovery autosave failed (disk space or permissions). |

Clés existantes réutilisées : `dialog.password.cancel`, `error.password_wrong`, `error.password_required`,
`properties.*` (libellés de métadonnées), `menu.view.zoom_in`, `menu.view.zoom_out`.
63 nouvelles clés ; mêmes paramètres `{…}` dans les deux langues (test `placeholders_match_between_languages`).
Préfixes `compare.*` et `recovery.*` à déclarer dans `docs/I18N.md`.

---

## 8. Tests

### 8.1 Fixture à générer — `scripts/make_fixtures.py`
Ajouter `two_pages_v2(path)` (copie de `two_pages`, l. 37-56) et l'appeler dans le `__main__` (l. 84-88) pour
produire `crates/sunipdf-core/tests/fixtures/two-pages-v2.pdf` :
- page 1 (612 × 792) : même rectangle bleu, texte `(SuniPDF page 1 revised)` ;
- page 2 (792 × 612) : **même texte** `(SuniPDF page 2)`, rectangle `0 0.5 0 rg 72 72 300 100 re f` (vert au lieu
  de rouge, même emplacement) → texte identique, apparence différente ;
- page 3 (612 × 792), nouvelle : `(Annexe ajoutee)` ;
- `/Info` : `/Title (SuniPDF fixture v2)`, même auteur ; sans plan de signets.
Régénérer : `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures` (les deux fixtures
existantes sont réécrites à l'identique : le script est déterministe).

### 8.2 Tests d'intégration — `crates/sunipdf-core/tests/engine.rs` (assistants `engine()` l. 34, `fixture()` l. 6)
1. `page_words_have_text_lines_and_bounds` : `page_words(0)` de `two-pages.pdf` = `["SuniPDF", "page", "1"]`,
   même `line`, rectangles croissants en `left`, `bottom > 690` et `top < 730` (mêmes bornes que le test de
   recherche existant, l. 202-203), `right > left` ; `page_words(9)` → `Err(PageOutOfRange(9))`.
2. `compare_reports_text_pages_and_metadata` (A = `two-pages.pdf`, B = `two-pages-v2.pdf`) : nombres de pages
   2 et 3 ; `metadata` contient `title` (`"SuniPDF fixture"` → `"SuniPDF fixture v2"`) ; trois paires :
   `(Some(0), Some(0))` `Modified` avec `added.len() == 1`, `removed` vide, rectangle ajouté `left > 200` et
   `bottom > 690`, une ligne dont le dernier span est `Added "revised"` ; `(Some(1), Some(1))` `Identical`
   sans ligne ; `(None, Some(2))` `OnlyB` ; `words_added == 3`, `words_removed == 0` ; `!report.identical()`.
3. `compare_identical_documents_is_clean` : même fichier ouvert deux fois → toutes les paires `Identical`,
   `metadata` vide, `identical()`.
4. `pixel_diff_locates_visual_change` (**pixels rendus**) : page 2 de A et de B à l'échelle 1.0 →
   `changed > 20_000` (rectangle 300 × 100 pt, rouge contre vert), `bbox` à ± 2 px de `[72, 440, 371, 539]`
   (page de 612 pt de haut : y depuis le haut = 612 − 172 … 612 − 72), masque opaque en (150, 490), transparent en
   (600, 100). Page 1 de A et de B : `changed > 0`, `bbox` entièrement au-dessus de y = 140 (ligne de texte, comme
   l'assertion des 140 premières lignes l. 100-110) et `x0 > 200`. Page 1 de A contre elle-même : `changed == 0`,
   `bbox == None`, tous les alphas du masque à 0.
5. `recovery_snapshot_round_trip_keeps_original_untouched` (**relecture + pixels**) : copier la fixture dans un
   dossier temporaire (`orig.pdf`), mémoriser ses octets ; ouvrir, `set_page_rotation(0, R90)` et
   `add_text(0, 72.0, 300.0, "Brouillon", &TextStyle::default())` ; `doc.save_as(&rec_dir.join("1.pdf"))`.
   Assertions : octets de `orig.pdf` inchangés ; le dossier de récupération contient exactement 1 fichier (aucun
   `.tmp`) ; `engine.open_bytes(std::fs::read(rec)?, None, &orig)` → `info().path == orig`, 2 pages,
   `page_size(0).width_pt == 792.0` (comme l. 272-276), `page_text(0)` contient `"Brouillon"`, et le rendu de la
   page 0 contient le bleu du rectangle (`(0, 0, 255)`, plus de 1000 pixels).
6. `recovery_snapshot_of_encrypted_document_stays_encrypted` (**affirmation de sécurité, donc testée**) : ouvrir
   `encrypted-rc4.pdf` avec `"secret"`, `save_as(rec)` ; `engine.open(&rec, None)` → `Err(PasswordRequired)` ;
   `open_bytes(octets, Some("secret"), …)` → `page_text(0)` contient `"Encrypted page"`. En cas d'échec :
   appliquer le repli du § 3.3 et le documenter.

### 8.3 Tests unitaires
- `crates/sunipdf-core/src/compare.rs` (sans PDFium) : `diff` sur vide / égal / insertion / suppression /
  remplacement / préfixe et suffixe communs, et la propriété « rejouer les opérations sur `a` reconstruit `b` » ;
  repli au-delà du plafond ; `pixel_diff` sur images synthétiques (identiques ; un pixel à tolérance ± 1 ;
  tailles différentes).
- `apps/sunipdf-desktop/src/recovery.rs` (dossiers temporaires, sans PDFium) : aller-retour du manifeste +
  `forget` ; `scan` ignore une session vivante (verrou tenu), la propose une fois le verrou relâché, et un
  second `scan` ne la propose plus tant que la première revendication est tenue ; dossier sans `.pdf` valide
  supprimé ; `adopt` déplace le fichier et l'inscrit dans la session courante ; `clear_session` supprime tout.
- `apps/sunipdf-desktop/src/settings.rs` : valeurs par défaut (`autosave_secs == 60`, session vide), et un ancien
  JSON sans ces champs se charge toujours.
- `crates/sunipdf-core/src/command.rs:648` : compte du menu Fichier = 9.
- `apps/sunipdf-desktop/src/app.rs:1964-1981` : étendre `ui_keys_used_by_app_exist` à
  `include_str!("compare.rs")` et `include_str!("recovery.rs")`, et exclure les littéraux finissant par `.json`.

### 8.4 Commandes de validation (à exécuter par l'implémenteur, `-j 2`)
`cargo test --workspace -j 2`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`,
`.\scripts\check-i18n.ps1`, `cargo tree -p sunipdf-core` (aucune dépendance UI/CLI ajoutée),
`cargo run -p sunipdf-cli -- compare <A> <B> --visual`.

### 8.5 Scénarios manuels — `scripts/dev-screenshot.ps1`
`$fx = "crates\sunipdf-core\tests\fixtures"`. Le dernier fichier de `-Pdf` (séparateur `;`, script l. 34) est
l'onglet actif = document A ; l'autre onglet pré-remplit B.

Comparaison :
```powershell
$two = "$fx\two-pages-v2.pdf;$fx\two-pages.pdf"
.\scripts\dev-screenshot.ps1 -Out cmp-1-dialogue.png -Pdf $two -Commands 'file.compare'
.\scripts\dev-screenshot.ps1 -Out cmp-2-cote-a-cote.png -Pdf $two -Commands 'file.compare' -Keys '{ENTER}'
.\scripts\dev-screenshot.ps1 -Out cmp-3-texte.png -Pdf $two -Commands 'file.compare' -Keys '{ENTER}' -KeysAfter 't'
.\scripts\dev-screenshot.ps1 -Out cmp-4-visuel.png -Pdf $two -Commands 'file.compare' -Keys '{ENTER}' -KeysAfter '{PGDN}v'
```
Attendu : (2) « revised » surligné en vert sur B, synthèse « 2 page(s) modifiée(s) sur 3 », ligne « Nombre de
pages : 2 → 3 », titre dans les métadonnées modifiées ; (3) le mot ajouté sur fond vert ; (4) paire 2 : masque
magenta sur le rectangle. `-Keys` puis `-KeysAfter` (et non une seule chaîne) : le rapport doit être arrivé avant
`t`/`v`, sinon ces touches activent des outils du document. Refaire (2) en thème sombre et en anglais.

Récupération (dossier isolé, intervalle d'une seconde ; le script termine l'application par `Stop-Process -Force`,
l. 85, ce qui simule un plantage) :
```powershell
$env:SUNIPDF_RECOVERY_DIR = "$env:TEMP\sunipdf-recovery-test"; $env:SUNIPDF_AUTOSAVE_SECS = "1"
$before = (Get-FileHash "$fx\two-pages.pdf").Hash
.\scripts\dev-screenshot.ps1 -Out rec-1-modifie.png -Pdf "$fx\two-pages.pdf" -Commands 'pages.rotate_cw' -Delay 6
Get-ChildItem $env:SUNIPDF_RECOVERY_DIR -Recurse      # attendu : <session>\lock, manifest.json, 1.pdf
.\scripts\dev-screenshot.ps1 -Out rec-2-dialogue.png
.\scripts\dev-screenshot.ps1 -Out rec-3-restaure.png -Keys '{ENTER}'
(Get-FileHash "$fx\two-pages.pdf").Hash -eq $before   # attendu : True
Remove-Item $env:SUNIPDF_RECOVERY_DIR -Recurse -Force; Remove-Item Env:SUNIPDF_RECOVERY_DIR, Env:SUNIPDF_AUTOSAVE_SECS
```
Attendu : (2) boîte « Récupérer des documents » avec `two-pages.pdf`, 2 pages ; (3) onglet `two-pages.pdf •`,
page 1 en paysage, message d'état « Document récupéré… ». À la main ensuite : Ctrl+S ouvre « Enregistrer sous »
et Windows demande confirmation avant de remplacer l'original ; « Supprimer » demande un second clic ; lancer deux
instances et vérifier que la seconde ne propose pas les instantanés de la première ; quitter normalement vide le
dossier de session.

---

## 9. Nouvelles dépendances

**Aucune.**
- Diff : LCS maison (≈ 45 lignes, testée). Le crate `similar` (3.2.0, Apache-2.0, sans dépendance obligatoire —
  vérifié sur docs.rs par requête web le 2026-10-14, **absent** de `Cargo.lock` et du registre local) conviendrait
  côté licence, mais ajouter un crate pour une LCS bornée par page ne se justifie pas. Voie de sortie notée dans
  le code (`ponytail:`) si des pages très denses dépassent le plafond.
- Verrou de session : bibliothèque standard (`share_mode`), pas de crate `windows` en dépendance directe.
- Manifeste : `serde` / `serde_json`, déjà dépendances de `sunipdf-desktop`.
- Dates : calcul entier sur l'heure Unix (`chrono` n'est qu'une dépendance transitive, on ne s'y lie pas).

---

## 10. Risques, pièges, ordre d'implémentation, estimation

### Risques
1. **Bruit du diff de texte** entre PDF de producteurs différents (ordre du flux). Limite annoncée ; le diff
   visuel sert de recours.
2. **Thread moteur occupé** pendant une grosse comparaison (extraction de texte de toutes les pages des deux
   documents) : le rendu du document actif attend ; ni progression ni annulation. Si c'est gênant : découper
   par page (une requête par paire) — hors tranche.
3. **Mémoire** : deux documents supplémentaires ouverts tant que la vue de comparaison est affichée, plus un
   instantané quand A est le document actif.
4. **Pages pivotées** : rectangles de mots possiblement décalés (même hypothèse que la recherche). Vérifier avec
   une page `/Rotate 90` ; si décalé, masquer les surlignages sur ces pages (le mode Texte reste exact) et l'inscrire
   dans `KNOWN_ISSUES.md`.
5. **Instantané d'un document protégé** : dépend du comportement de PDFium à l'enregistrement ; couvert par le
   test § 8.2-6, repli décrit § 3.3.
6. **Hoquet d'autosave** sur très gros documents (une sérialisation complète sur le thread moteur). Atténué par :
   uniquement si modifié depuis le dernier instantané, jamais bouton de souris enfoncé, intervalle réglable.
7. **Plusieurs instances** : traité par un dossier et un verrou par session ; à vérifier manuellement (§ 8.5).
8. **Vie privée** : des copies de documents résident dans `%LOCALAPPDATA%\SuniPDF\recovery` jusqu'à
   l'enregistrement, la fermeture ou la sortie normale. Local uniquement ; à écrire dans les guides.

### Pièges connus
- `ui_keys_used_by_app_exist` (`app.rs:1964-1981`) traite tout littéral en minuscules contenant un point comme
  une clé i18n : **aucun nom de fichier (`manifest.json`) dans `app.rs` / `main.rs`** ; le garder dans
  `recovery.rs` et ajouter l'exclusion `.json` avant d'inclure ce fichier dans le test.
- Correspondances exhaustives sur `Command` : `execute` (`app.rs:801`) et `command_icon` (`shell.rs:247`) ne
  compilent plus tant que les nouvelles variantes n'y sont pas ; le compte du menu Fichier (`command.rs:648`) et
  `registry_keys_exist_in_both_catalogs` (`app.rs:1984`) échouent tant que les clés manquent.
- `Request::Open` est construit à deux endroits (`app.rs:236` et `app.rs:1649`) : ajouter `restore_as` aux deux.
- `Tab { … }` est construit à deux endroits (`tabs.rs:122` et `tabs.rs:208`).
- egui ne redessine pas sans événement : sans `request_repaint_after`, l'autosave ne se déclenche jamais sur une
  fenêtre au repos.
- `on_exit` n'existe qu'avec la signature `Option<&glow::Context>` quand la feature `glow` est active (c'est le
  cas, `Cargo.toml` racine) ; si le backend wgpu est réactivé (D010), la signature change (`EF/epi.rs:222-228`).
- Un verrou ouvert avec `share_mode(0)` empêche aussi `remove_dir_all` : relâcher le `File` **avant** de
  supprimer un dossier de session.
- `dev-screenshot.ps1` tue l'application : toute capture avec document modifié et instantané écrit laisse
  désormais une session à récupérer, et la boîte de récupération apparaîtra dans les captures suivantes.
  **Contrat proposé** au propriétaire du script : quand `SUNIPDF_RECOVERY_DIR` n'est pas défini par l'appelant,
  le script le pointe vers un dossier temporaire neuf à chaque exécution.
- Les rendus de comparaison ne doivent pas passer par `state.cache` (clé = page du document actif).
- `Document::properties()` est coûteuse (sérialisation + lopdf) : ne pas l'appeler dans `compare`.
- Fichiers partagés avec d'autres tranches (`app.rs`, `worker.rs`, `command.rs`, `shell.rs`, catalogues i18n) :
  conflits de fusion probables ; intégrer cette tranche dans un worktree et la rebaser en dernier sur ces fichiers.

### Ordre d'implémentation (chaque étape compile et passe ses tests)
1. `compare.rs` côté moteur, partie pure : `diff`, `pixel_diff`, types + tests unitaires.
2. `text.rs::page_words`, `compare::compare`, fixture v2, tests d'intégration 1 à 4.
3. CLI `compare`.
4. Worker : `CompareSource`, `Compare` / `ComparePage` / `CompareClose`, `State.compare`.
5. Commande `CompareFiles` + clés `compare.*` + `apps/sunipdf-desktop/src/compare.rs` (boîte, vue) + câblage
   `app.rs` / `shell.rs` ; captures § 8.5 (comparaison).
6. `Settings` (trois champs) + `recovery.rs` (logique pure) + tests unitaires.
7. Worker `Autosave` + `Open.restore_as` ; `Tab.id` / `autosave_stale` / `recovered` ; minuteur, instantané au
   changement d'onglet, nettoyage (`Saved`, fermeture, `on_exit`) ; tests d'intégration 5 et 6.
8. Boîte de récupération, file de restauration, « Enregistrer sous » imposé ; captures § 8.5 (récupération).
9. Session (`sync_session`, réouverture), commande `ClearRecent`, cases du menu Paramètres, lien d'accueil.
10. Documentation (§ 11), ADR, `cargo test --workspace -j 2`, clippy, fmt.

Les étapes 1-5 (comparaison) et 6-9 (récupération) sont indépendantes et peuvent être livrées séparément.

### Estimation : **L**
Moteur ≈ 300 lignes (dont tests unitaires), worker ≈ 110, interface ≈ 650 (vue de comparaison ≈ 400, récupération
≈ 250), CLI ≈ 70, tests d'intégration ≈ 220, 63 clés × 2 catalogues, documentation.

---

## 11. Documentation à mettre à jour

- `docs/DECISIONS.md` — deux ADR (prochains numéros libres, D021 et D022 si personne ne les a pris) :
  *Comparaison par LCS maison mot à mot, appariement des pages par texte, pas de crate `similar`* ;
  *Autosave : un dossier par session sous `%LOCALAPPDATA%\SuniPDF\recovery`, verrou par mode de partage,
  manifeste JSON côté application, restauration par `open_bytes`, premier enregistrement en « Enregistrer sous »*.
- `docs/ARCHITECTURE.md` — section « Comparaison » (session de deux documents dans le worker, indépendante de
  l'onglet actif) et « Autosave et récupération » (flux, disposition des fichiers, variables d'environnement) ;
  ajouter `compare.rs` et `recovery.rs` au schéma du workspace.
- `docs/FEATURE_MATRIX.md` — lignes « Récupération après plantage, autosave » (l. 29), « Historique de
  navigation, onglets rouverts » (l. 26, réouverture de session) et « Comparaison de deux versions » (l. 55) :
  passer à **Fait** uniquement avec les résultats de tests réels.
- `docs/ROADMAP.md` (l. 34, 38, 52, 56, 70) — retirer autosave / récupération et comparaison des « reste » ;
  y laisser : rapport de comparaison exportable, analyse visuelle globale, comparaison de mise en forme et
  d'annotations, restauration de la position de lecture, réglage de l'intervalle dans l'interface.
- `docs/KNOWN_ISSUES.md` — limites du § 2 et piège `dev-screenshot`.
- `docs/PROJECT_STATUS.md`, `docs/NEXT_TASKS.md` — état, commandes exécutées et résultats réels.
- `docs/USER_GUIDE.fr.md` / `docs/USER_GUIDE.en.md` — sections « Comparer deux PDF » et « Récupération après
  plantage » (emplacement des instantanés, confidentialité), « Paramètres », ligne de commande `compare`,
  « Limites actuelles ».
- `docs/I18N.md` — préfixes `compare.*`, `recovery.*` ; test de clés étendu à `compare.rs` et `recovery.rs`.
- `CHANGELOG.md` — entrées de version.
- `CLAUDE.md` — « Modules existants » : `sunipdf-core` (+ `compare`), `apps/sunipdf-desktop` (+ `compare.rs`,
  `recovery.rs`), CLI (21 commandes) ; variables `SUNIPDF_RECOVERY_DIR` et `SUNIPDF_AUTOSAVE_SECS`.
  (Modification de `CLAUDE.md` à valider par l'utilisateur, pas par un agent.)
- `scripts/dev-screenshot.ps1` — contrat proposé au § 10 (dossier de récupération isolé par défaut).
