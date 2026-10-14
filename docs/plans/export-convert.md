# Plan d'implémentation — tranche `export-convert` (Exporter un PDF / Créer un PDF)

Rédigé le 2026-10-14 en lecture seule (aucune commande cargo exécutée). Tout appel de bibliothèque cité ci-dessous a été relu dans
les sources locales. Les chemins `pdfium-render`, `flate2`, `encoding_rs` sont relatifs à
`C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (convention de citation au paragraphe 3.0).
Ce qui n'a pas pu être vérifié sur ce poste est marqué **[non vérifié]** et repris dans la section 10.

## 0. Décisions structurantes (à lire en premier)

| # | Décision | Raison courte |
|---|---|---|
| 1 | Tout le moteur va dans `crates/sunipdf-core` (nouveaux modules `job`, `structure`, `export`, `compose`, `office`), pas dans un nouveau crate `sunipdf-convert`. | Le texte structuré a besoin de `Document.inner` (privé à `sunipdf-core`). Un crate de plus (Cargo.toml, CI, validation architect) n'apporte rien tant que l'OCR n'existe pas. Les modules purs (writers, parseur Markdown, mise en page) n'ont aucune dépendance PDFium et pourront être extraits plus tard. |
| 2 | Exports Markdown / HTML / DOCX = pipeline unique : `Document::structured_text` (PDFium, par caractère) → `StructuredText` → writers **purs** testables sans `pdfium.dll`. Texte brut = `page_text` PDFium, sans heuristique. | Une seule heuristique à mesurer ; les writers se testent en millisecondes. |
| 3 | DOCX : écrivain ZIP maison (~60 lignes) + `flate2` (déjà dans l'arbre via lopdf, MIT OR Apache-2.0) pour DEFLATE et CRC-32. Pas de crate `zip`. | `zip` n'est pas dans `Cargo.lock` ; `flate2 1.1.10` et `crc32fast 1.5.2` y sont déjà. |
| 4 | PDF → XLSX : **hors tranche**, reste en feuille de route (justification en 2.2). | Aucune détection de tableaux crédible et mesurable sans corpus. |
| 5 | Images dans les exports Markdown / HTML / DOCX : **hors v1**. Les PNG par page restent disponibles via le même dialogue. | Placer des images dans le flux de lecture demande une analyse de mise en page que la tranche ne fournit pas. |
| 6 | Création : page vierge, texte brut / Markdown simple (mise en page maison sur polices standard PDFium), images (existant), Office → PDF **via un convertisseur déjà installé** et détecté (Microsoft Office par COM/PowerShell, LibreOffice en `soffice --headless`). Rien n'est embarqué ni téléchargé. | Aucune bibliothèque Rust permissive ne rend DOCX/XLSX/PPTX fidèlement ; FEATURE_MATRIX.md (ligne « Office → PDF ») prévoyait déjà cette voie. |
| 7 | Conversion Office : hors thread moteur (elle n'utilise pas PDFium), thread dédié, annulable (`JobControl`), délai maximal, processus tué à l'annulation. Export de texte : sur le thread moteur (`BatchOp`), annulable et avec progression via le même `JobControl`. | PDFium reste confiné au thread `sunipdf-engine` ; le viewer n'est pas bloqué 3 minutes par un `soffice` lent. |
| 8 | Aucune de ces opérations ne modifie le document ouvert : pas d'`EditOp`, pas d'instantané, pas d'annulation, `dirty` inchangé. Les sorties sont de nouveaux fichiers ; les créations s'ouvrent dans un nouvel onglet comme `CreateFromImages`. | Cohérent avec `BatchOp` (worker.rs:30-38) : « produit de nouveaux fichiers, le document ouvert est intact ». |
| 9 | Fidélité annoncée honnêtement dans l'interface (une phrase par format, 5.2) ; aucun pourcentage affiché tant qu'il n'est pas mesuré (protocole en 8.5). | Exigence du projet : mesurer la fidélité au lieu de la supposer. |

## 1. Objectif et comportement visible

Correspondance PDFCraft : « Export a PDF » (vers Word, texte, images) et « Create a PDF » (blanc, texte, images, Office).

Ce que l'utilisateur voit :

1. Deux nouvelles lignes dans le panneau d'outils (onglets **Convertir** et **Tous les outils**), dans le menu **Convertir** et dans la palette Ctrl+K :
   - « Exporter vers Word, Markdown, HTML, texte… » (`convert.export`, exige un document : sans document, l'outil ouvre d'abord un fichier, comportement existant de `SuniApp::execute`, app.rs:793-799) ;
   - « Créer un PDF (page vierge, texte, Office)… » (`convert.create`, utilisable sans document).
   Les lignes existantes « Créer un PDF à partir d'images… » et « Exporter les pages en images… » restent (palette, CLI, habitudes).
2. **Dialogue Exporter** (un seul, via `dialog::show`) : format (Texte brut, Markdown, Page web HTML, Word DOCX, Images PNG), portée (tout le document / page(s) sélectionnée(s) ou courante), case « Ignorer les en-têtes et pieds de page répétés » (Markdown, HTML, DOCX), résolution pour PNG. Sous le format choisi, **une phrase de fidélité**, par exemple : « Fidélité : approchée. Titres déduits de la taille, paragraphes et listes simples ; ni images ni tableaux. » Bouton « Exporter » → sélecteur de fichier natif (confirmation d'écrasement native) → exécution sur le thread moteur avec spinner, « page 3 / 40 » et lien Annuler dans la barre d'état → message de fin dans la barre d'état. PDF numérisé (aucun texte) : erreur claire, **aucun fichier vide n'est écrit**.
3. **Dialogue Créer** : quatre sources (Page vierge / Fichier texte ou Markdown / Images / Document Office).
   - Page vierge : format (A3, A4, A5, Lettre, Légal), orientation, nombre de pages (1 à 500) → « Enregistrer sous » → ouverture dans un nouvel onglet.
   - Texte ou Markdown : choix du fichier, format de page, orientation, taille du texte → PDF paginé (titres, listes, blocs de code, paragraphes), ouvert dans un nouvel onglet. Note visible : images, tableaux, gras et italique en ligne non rendus ; caractères hors Windows-1252 remplacés par « ? » (leur nombre est annoncé).
   - Images : délègue à l'outil existant (`tool_create_from_images`, tools.rs:361-381).
   - Office : ligne d'état « Convertisseur détecté : Microsoft Office » ou « LibreOffice » ; s'il n'y en a aucun, message explicite et bouton désactivé (jamais de bouton factice). Note : « converti localement par un logiciel installé ; aucun fichier n'est envoyé sur Internet ; la mise en page dépend de ce logiciel ». Conversion annulable (lien Annuler dans la barre d'état), délai maximal 180 s.
4. CLI : `export`, `create-blank`, `text2pdf`, `office2pdf`, `converters` (section 6).
5. Rien n'est marqué « modifié », rien n'entre dans l'historique d'annulation.

## 2. Faisabilité

**Verdict : partiel.** Livrable : texte brut, Markdown, HTML, DOCX (structure approchée), page vierge, texte/Markdown → PDF, Office → PDF si un convertisseur est installé. Hors tranche : PDF → XLSX/PPTX, images dans les exports, tableaux, colonnes, HTML → PDF, mise en forme en ligne du Markdown.

### 2.1 Constats faits sur ce poste (commandes en lecture seule)

- Microsoft Word installé : `C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE`, version de fichier 16.0.20430.20146. Clé `App Paths\WINWORD.EXE` présente (vues 64 bits et WOW6432Node). `Word.Application`, `Excel.Application` et `PowerPoint.Application` ont tous une clé `CLSID` (COM utilisable). Aucun processus WINWORD en cours.
- LibreOffice **absent** : `C:\Program Files\LibreOffice\program\soffice.exe` et la variante x86 renvoient `False` ; pas de clé `App Paths\soffice.exe` ; ni `soffice` ni `libreoffice` dans le PATH. Tout ce qui concerne `soffice` est donc **[non vérifié]** ici : seule la documentation officielle a été consultée.
- Windows PowerShell 5.1.26100.9549 (`C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe`). Politique d'exécution : `Process = Bypass`, les autres scopes `Undefined` (donc `Restricted` par défaut pour un `.ps1` : le plan impose `-ExecutionPolicy Bypass` sur la ligne de commande).
- Imprimantes : « Microsoft Print to PDF » existe, mais l'automatiser passerait par l'impression d'un document ouvert dans son application (aucun gain sur COM) : écarté.
- `qpdf` absent du PATH (la vérification qpdf des PDF créés reste celle de la CI).
- `Cargo.lock` : pas de `zip` ; présents : `flate2 1.1.10` (dépendances : crc32fast, miniz_oxide 0.9.1, zlib-rs), `crc32fast 1.5.2`, `encoding_rs 0.8.42` (via lopdf), `quick-xml 0.41.0` (uniquement via wayland-scanner, inutilisable sous Windows).

### 2.2 Inclus / exclu et pourquoi

| Fonction | Statut | Pourquoi / limite à documenter |
|---|---|---|
| PDF → texte brut | Inclus | `page_text` PDFium ; ordre de lecture = ordre du flux de contenu ; sauts de page = U+000C. Déjà exposé par la CLI `text` (apps/sunipdf-cli/src/main.rs:281-300). |
| PDF → Markdown / HTML | Inclus, approché | Titres déduits **de la taille seule** (pas du gras seul), paragraphes par interligne, listes à puces ou numérotées simples, sauts de page. Pas d'images, tableaux, colonnes, notes, liens, couleurs. |
| PDF → DOCX | Inclus, approché | Mêmes blocs ; styles Titre 1 à 3, paragraphes, sauts de page, taille de page de la première page exportée, puces littérales (pas de numérotation Word native). Pas d'images, tableaux, colonnes, en-têtes/pieds Word. |
| PDF → XLSX | **Exclu** | Reconnaître des tableaux demande soit des lignes de grille (objets tracé regroupés), soit un alignement de colonnes ; sans corpus annoté, aucune précision ne peut être mesurée, et un XLSX faux est pire que pas de XLSX. Esquisse pour plus tard : détecter les grilles par rectangles/lignes (`ObjectKind::Path`, edit.rs:8-14), affecter les `TextLine` aux cellules, écrire sharedStrings + sheet1 avec le même écrivain ZIP. Prérequis : corpus et métrique (cellules exactes). Reste dans ROADMAP phase 4. |
| PDF → PPTX ; images dans le DOCX | Exclu | Même raison, sans valeur immédiate. |
| Images PNG par page | Déjà fait (`export_images`, convert.rs:57-89) | Intégré au dialogue Exporter ; réglage de la résolution ajouté. |
| Page vierge | Inclus | `PdfPages::create_page_at_end` (pdfium-render src/pdf/document/pages.rs:223). |
| Texte / Markdown → PDF | Inclus, limité | Latin (Windows-1252) seulement : polices standard Helvetica/Courier non incorporées ; pas de CJK, d'arabe, d'emoji. Mise en page maison simple. |
| Images → PDF | Déjà fait (convert.rs:17-54) | Seule la porte d'entrée change. |
| Office → PDF | Inclus **si** un convertisseur est installé | Microsoft Office (COM via PowerShell) ou LibreOffice. Aucun convertisseur embarqué (LibreOffice est MPL-2.0 mais pèse des centaines de Mo ; Office est propriétaire). Sans convertisseur : message et bouton désactivé. Fidélité = celle du logiciel tiers. |
| HTML → PDF | Exclu | Pas de moteur de rendu HTML dans l'arbre ; reste « P5 optionnel » du FEATURE_MATRIX. |

### 2.3 Licences

| Composant | Licence | Rôle | Nouveau dans `Cargo.lock` ? |
|---|---|---|---|
| `flate2` 1.1.10 | MIT OR Apache-2.0 (flate2-1.1.10/Cargo.toml:47) | DEFLATE et `flate2::Crc` pour le ZIP du DOCX | Non (déjà verrouillé ; devient dépendance directe de `sunipdf-core`) |
| `encoding_rs` 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause (encoding_rs-0.8.42/Cargo.toml:49) | Lire les .txt en Windows-1252 | Non (déjà verrouillé via lopdf) |
| Microsoft Office / LibreOffice | propriétaire / MPL-2.0 | Convertisseur externe détecté, jamais redistribué | Hors Cargo |
| DOCX (ISO/IEC 29500, ECMA-376) et ZIP (APPNOTE.TXT de PKWARE) | normes publiques | Structures écrites à la main | n/a |

## 3. API moteur (`crates/sunipdf-core`)

### 3.0 Conventions et inventaire des fichiers

Abréviations de citation : `PR:` = `pdfium-render-0.9.4/src/`, `F2:` = `flate2-1.1.10/src/`, `ER:` = `encoding_rs-0.8.42/src/`. Les chemins SuniPDF sont relatifs à `C:/dev_suniworks/SuniPDF`.

| Fichier | État | Contenu |
|---|---|---|
| `crates/sunipdf-core/Cargo.toml` | modifié | `flate2 = "1"` et `encoding_rs = "0.8"` (déjà verrouillés ; `cargo` réécrit seulement la liste de dépendances de `sunipdf-core` dans `Cargo.lock`) |
| `crates/sunipdf-core/src/error.rs` | modifié | 6 variantes typées (3.1) |
| `crates/sunipdf-core/src/job.rs` | nouveau | `JobControl` : annulation + progression partagées avec l'UI (3.2) |
| `crates/sunipdf-core/src/structure.rs` | nouveau | lignes de texte avec tailles/positions, blocs (titres, paragraphes, listes) (3.3) |
| `crates/sunipdf-core/src/export.rs` | nouveau | `ExportFormat`, writers Markdown/HTML/DOCX purs, écrivain ZIP, `Document::export_document` (3.4) |
| `crates/sunipdf-core/src/compose.rs` | nouveau | `PaperSize`, `Engine::create_blank`, `Engine::text_to_pdf`, parseur Markdown, mise en page (3.5) |
| `crates/sunipdf-core/src/office.rs` + `office_convert.ps1` | nouveaux | détection et invocation des convertisseurs externes (3.6) |
| `crates/sunipdf-core/src/lib.rs` | modifié | `pub mod job; structure; export; compose; office;` + ré-exports (3.7) |
| `crates/sunipdf-core/src/command.rs` | modifié | 2 commandes (3.7) |

Tous les types `pdfium-render` utilisés ci-dessous (`PdfPageTextChar`, `PdfFontWeight`, `PdfPageTextObject`, `PdfPagePaperSize`, `PdfPageObjectCommon`, `PdfPageObjectsCommon`, `PdfFontToken`, `PdfPageContentRegenerationStrategy`, `PdfColor`, `PdfPoints`) sont réexportés par `pdfium_render::prelude::*` (PR:lib.rs:102-200), déjà importé par `edit.rs` et `convert.rs`.

Convention du dépôt à respecter : les simplifications assumées portent un commentaire `// ponytail: <plafond> ; <voie d'évolution>` (voir edit.rs, annot.rs:77, pages.rs:230).

### 3.1 `error.rs` — nouvelles variantes

Ajouter à `EngineError` (error.rs:6-25) et à `code()` (error.rs:27-42) :

```rust
#[error("no extractable text in the selected pages")]            NoText,                  // "no_text"
#[error("no document converter (Microsoft Office or LibreOffice) found")] ConverterUnavailable, // "converter_unavailable"
#[error("converter failed: {0}")]                                 ConverterFailed(String), // "converter_failed"
#[error("converter timed out")]                                   ConverterTimeout,        // "converter_timeout"
#[error("unsupported file format: {0}")]                          UnsupportedFormat(String), // "unsupported_format"
#[error("operation cancelled")]                                   Cancelled,               // "cancelled"
```

Conséquences à traiter dans le même commit : `apps/sunipdf-desktop/src/app.rs::error_key` (app.rs:1924, `match` exhaustif) et le test `every_error_key_exists_in_both_catalogs` (liste `errors` à compléter) ; `apps/sunipdf-cli/src/main.rs` : correspondance code de sortie (main.rs:663-668) : `ConverterUnavailable → 6`, `NoText → 7`, le reste inchangé. La conversion `From<PdfiumError>` (error.rs:44-62) ne change pas.

### 3.2 `job.rs` — annulation et progression

```rust
#[derive(Debug, Default)]
pub struct JobControl { cancel: AtomicBool, done: AtomicUsize, total: AtomicUsize }
impl JobControl {
    pub fn cancel(&self);                      // store(true, Relaxed)
    pub fn is_cancelled(&self) -> bool;
    pub fn progress(&self) -> (usize, usize);  // (pages faites, pages totales) ; total 0 = indéterminé
    pub(crate) fn begin(&self, total: usize);
    pub(crate) fn tick(&self);                 // done += 1
    pub(crate) fn check(&self) -> Result<(), EngineError>; // Err(Cancelled) si annulé
}
```

Utilisé en `Arc<JobControl>` : l'UI garde un clone (bouton Annuler dans la barre d'état, lecture de la progression), le moteur reçoit `Option<&JobControl>`. `AtomicBool::store` ne demande que `&self` : la barre d'état (`pub fn status_bar(&self, ui)`, shell.rs:1122) peut annuler sans `&mut`.

### 3.3 `structure.rs` — texte avec tailles de police et positions

#### Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine { pub text: String, pub left: f32, pub right: f32, pub baseline: f32,
                      pub size: f32, pub bold: bool, pub italic: bool, pub soft_hyphen: bool }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind { Heading(u8 /* 1..=3 */), Paragraph, ListItem { ordered: bool } }
#[derive(Debug, Clone, PartialEq)] pub struct Block { pub kind: BlockKind, pub text: String }
#[derive(Debug, Clone)]          pub struct StructuredPage { pub index: usize, pub size: PageSize, pub blocks: Vec<Block> }
#[derive(Debug, Clone, Default)] pub struct ExtractStats { pub chars: usize, pub empty_pages: Vec<usize>,
                                                          pub dropped_repeated: usize, pub skipped_rotated: usize }
#[derive(Debug, Clone)]          pub struct StructuredText { pub pages: Vec<StructuredPage>, pub body_size: f32, pub stats: ExtractStats }
#[derive(Debug, Clone)]          pub struct ExtractOptions { pub pages: Option<Vec<usize>> /* None = toutes */, pub drop_repeated: bool }
impl Default for ExtractOptions { /* pages: None, drop_repeated: true */ }
```

#### Fonctions

```rust
impl Document<'_> {
    /// Lignes d'une page + nombre de caractères non horizontaux ignorés.
    pub fn text_lines(&self, page_index: usize) -> Result<(Vec<TextLine>, usize), EngineError>;
    pub fn structured_text(&self, opts: &ExtractOptions, control: Option<&JobControl>)
        -> Result<StructuredText, EngineError>;
}
// Pures (testables sans PDFium) :
pub fn body_font_size(lines: &[TextLine]) -> f32;
pub fn build_blocks(lines: &[TextLine], body_size: f32) -> Vec<Block>;
pub fn drop_repeated(pages: &mut [Vec<TextLine>], page_heights: &[f32]) -> usize; // nombre de lignes retirées
```

#### Appels PDFium utilisés (tous relus)

`text_lines(page_index)` :

1. `let page = self.page_handle(page_index)?;` — pages.rs:282 (contrôle de bornes + `PageOutOfRange`).
2. `let text = page.text()?;` — `PdfPage::text` : PR:pdf/document/page.rs:377. `PdfPageText` se ferme au `Drop` (PR:pdf/document/page/text.rs:415).
3. `let all = text.chars();` — PR:pdf/document/page/text.rs:102 ; `for ch in all.iter()` — PR:pdf/document/page/text/chars.rs:163 (même motif que `text_in_rect`, annot.rs:75-82).
4. Pour chaque caractère `ch: PdfPageTextChar` (PR:pdf/document/page/text/char.rs) :
   - `ch.unicode_char() -> Option<char>` (char.rs:109). `None`, U+FFFE, U+FFFF, caractères de contrôle autres que `\t` : ignorés. `\r` et `\n` (PDFium insère `\r\n` entre les lignes) : ferment la ligne courante (même logique que annot.rs:92-99). **U+0002** : marqueur de trait d'union de fin de ligne renvoyé par PDFium **[non vérifié ; comportement connu de PDFium, à confirmer par un PDF « exem-/ple »]** → supprimé du texte, `soft_hyphen = true` sur la ligne, la jointure de paragraphe recolle sans espace.
   - Rotation : `ch.get_rotation_counter_clockwise_degrees() -> f32` (PR:pdf/transform.rs:359, `atan2(b, a)` de la matrice du caractère, lue par `FPDFText_GetMatrix` : `get_matrix_impl` char.rs:565, getters générés par `create_transform_getters!` char.rs:587 ; renvoie 0 si la matrice est indisponible). Si `|a| > 10°` (donc aussi pour 180°, texte tête en bas), le caractère est ignoré et `skipped_rotated += 1` : filigrane diagonal posé par `stamp_pages`, texte vertical, qu'on ne sait pas remettre en ligne (c'est dit à l'utilisateur). On n'utilise **pas** `angle_degrees()` (char.rs:488) : elle renvoie `Err` pour tout angle négatif (char.rs:494-507) et confondrait un texte tourné dans le sens horaire avec un texte droit.
   - `ch.origin() -> Result<(PdfPoints, PdfPoints)>` (char.rs:594) : `y` = ligne de base, `x` = origine ; `Err` → caractère ignoré.
   - `ch.loose_bounds() -> Result<PdfRect>` (char.rs:545) : `left()/right()` pour les extrémités de ligne et l'écart avec le caractère précédent (`(bounds.left().value - prev.right) > 4 × taille` → un `\t` remplace l'espace généré : colonnes d'un tableau simple).
   - Taille : `ch.scaled_font_size().value.abs()` (char.rs:131-134 : taille de police × échelle verticale de la matrice) ; si non finie ou < 0,5 → `ch.unscaled_font_size()` (char.rs:142) ; si encore nulle → hauteur de `loose_bounds` / 1,2.
   - Gras : `ch.font_weight()` (char.rs:217 ; `PdfFontWeight` : PR:pdf/font.rs:50-81) ≥ `Weight600` ou `Custom(w ≥ 600)`, **ou** `ch.font_name()` (char.rs:209) contient (insensible à la casse) `bold`, `black`, `heavy`, `semibold` ou `demi`. La documentation de la bibliothèque prévient que le poids « may not reliably return the correct value for built-in fonts » (char.rs:213-216) : le nom de police sert donc de second avis.
   - Italique : `ch.font_is_italic()` (char.rs:309) ou nom contenant `italic` / `oblique`.
   - Ni `font_name()` ni `font_is_italic()` ne sont mis en cache au départ (2 appels FFI + une `String` par caractère). **Mesurer** (section 8.4) ; n'ajouter un cache par (taille, poids) que si une page de 3 000 caractères dépasse ~100 ms.

Regroupement en lignes : un caractère rejoint la ligne courante si `|y - ligne.baseline| <= 0,5 × max(taille, ligne.size)`, sinon la ligne est close. À la clôture : `text` rogné (espaces multiples réduits), `left/right` = min/max, `baseline` = celle du premier caractère, `size` = taille majoritaire (comptage par pas de 0,5 pt, hors espaces), `bold`/`italic` = vrai si ≥ 80 % des caractères non blancs le sont. L'ordre est celui du flux de contenu (aucun tri géométrique) : les documents multi-colonnes sont corrects si le producteur écrit les colonnes dans l'ordre de lecture, sinon l'ordre est celui de PDFium (même limite que `page_text`).

Pages tournées (`/Rotate`) : les coordonnées sont celles de l'espace utilisateur de la page ; seule la détection de rotation du caractère protège des textes réellement pivotés. Documenté comme limite.

`body_font_size` : taille dominante pondérée par le nombre de caractères sur **toutes** les lignes exportées (pas page par page).

`build_blocks` (pur) :

- Niveau de titre selon `ratio = line.size / body_size` : `>= 1,5 → H1`, `>= 1,25 → H2`, `>= 1,12 → H3`, sinon corps. Une ligne de plus de 200 caractères n'est jamais un titre. **Le gras seul ne fait pas un titre** (trop de faux positifs ; limite documentée).
- Puces : la ligne commence par `•`, `◦`, `▪`, `·`, `-`, `–`, `*` suivi d'une espace → `ListItem { ordered: false }` (marqueur retiré) ; `^\d{1,3}[.)] ` → `ListItem { ordered: true }` (le numéro d'origine est conservé dans le texte).
- Nouveau bloc si : première ligne, niveau différent du précédent, ligne de liste, bloc précédent titre, écart vertical `prev.baseline - line.baseline > 1,6 × max(tailles)` ou `< -0,5 × taille` (retour vers le haut : nouvelle colonne). Deux lignes consécutives de même niveau de titre et d'écart normal fusionnent (titre sur deux lignes).
- Jointure : `précédent + " " + ligne`, sans espace si le précédent porte `soft_hyphen` ; sinon le trait d'union de fin de ligne est conservé (pas de dé-césure heuristique : `États-` + `Unis` ne doit pas devenir `ÉtatsUnis`).

`drop_repeated` (pur) : zone de bord = `baseline > 0,92 × hauteur` ou `< 0,08 × hauteur`. Clé = texte en minuscules, chaînes de chiffres remplacées par `#`. Une clé est supprimée si elle apparaît dans la zone de bord d'au moins `max(2, ceil(0,6 × nombre de pages exportées))` pages (comptée une fois par page). Avec une seule page exportée, rien n'est supprimé. Le nombre de lignes supprimées est rapporté (`dropped_repeated`) pour être affiché.

`structured_text` : valide les indices (`PageOutOfRange`), appelle `control.check()?` puis `control.tick()` par page (`begin(total)` au départ), collecte les `TextLine`, calcule `body_font_size`, applique `drop_repeated` si demandé, construit les blocs par page. `stats.chars` = nombre de caractères non blancs avant suppression ; `empty_pages` = pages sans aucune ligne.

Implications : lecture seule du document (aucun `regenerate_content`, aucun instantané). Les modifications non enregistrées sont incluses puisque PDFium relit les objets courants, comme `export_images` (convert.rs:57-89).

### 3.4 `export.rs` — writers et export de fichier

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat { Text, Markdown, Html, Docx }
impl ExportFormat {
    pub const ALL: [ExportFormat; 4];
    pub fn id(self) -> &'static str;          // "txt" | "md" | "html" | "docx"
    pub fn extension(self) -> &'static str;   // identique à id()
    pub fn from_id(id: &str) -> Option<Self>;
}
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport { pub format: ExportFormat, pub pages: usize, pub chars: usize, pub headings: usize,
                          pub empty_pages: Vec<usize>, pub dropped_repeated: usize, pub skipped_rotated: usize, pub bytes: u64 }

impl Document<'_> {
    pub fn export_document(&self, format: ExportFormat, opts: &ExtractOptions, output: &Path,
                           control: Option<&JobControl>) -> Result<ExportReport, EngineError>;
}
// Pures :
pub fn render_markdown(text: &StructuredText) -> String;
pub fn render_html(text: &StructuredText, title: &str) -> String;
pub fn render_docx(text: &StructuredText) -> Result<Vec<u8>, EngineError>;
pub(crate) fn zip_archive(files: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, EngineError>;
```

`export_document` :

- `Text` : pour chaque page sélectionnée, `self.page_text(i)` (text.rs:24, `page.text()?.all()` donc PR:pdf/document/page/text.rs:246), `"\r\n"` → `"\n"`, pages séparées par U+000C (comme la CLI `text`, apps/sunipdf-cli/src/main.rs:292-298), UTF-8 sans BOM. `control.check()` / `tick()` par page. Si tout est vide après `trim` → `Err(NoText)`.
- Autres formats : `structured_text(...)`, `Err(NoText)` si `stats.chars == 0` (rien n'est écrit), rendu, puis `write_atomic(output, &bytes)` (pages.rs:63 : fichier voisin `.tmp` + `rename`, jamais de fichier partiel). Le titre HTML = `info().title` sinon nom de fichier sans extension (lib.rs:207).
- Les erreurs d'E/S restent des `EngineError::Io` (conversion `From<io::Error>`, error.rs:12).

`render_markdown` : `Heading(n)` → `#`×n + espace ; `ListItem` non ordonné → `- ` ; ordonné → texte tel quel (le numéro d'origine est dans le texte) ; paragraphes séparés par une ligne vide ; chaque saut de page → ligne `---` entourée de lignes vides. Échappement (`md_escape`) par barre oblique inverse des caractères : barre oblique inverse, accent grave, `*`, `_`, `[`, `]`, `<`, `>`, `|` partout ; en début de bloc, `#`, `+`, `-`, `>` et `^\d+[.)]` (pour qu'un paragraphe ne devienne pas une liste). Les tabulations deviennent 4 espaces.

`render_html` : `<!doctype html><html><head><meta charset="utf-8"><title>…</title><style>…</style></head><body>` ; une `<section class="page" id="page-N">` par page ; `<h1..h3>`, `<p>`, `<ul>`/`<ol>` regroupant les `ListItem` consécutifs de même type ; échappement `& < > "` ; pas d'attribut `lang` (la langue du contenu est inconnue et on ne traduit jamais le contenu).

`render_docx` : paquet OPC de 5 parties, écrit dans cet ordre (le premier fichier est `[Content_Types].xml`) :

| Partie | Contenu |
|---|---|
| `[Content_Types].xml` | `Default rels` (`application/vnd.openxmlformats-package.relationships+xml`), `Default xml` (`application/xml`), `Override /word/document.xml` (`application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml`), `Override /word/styles.xml` (`…wordprocessingml.styles+xml`) |
| `_rels/.rels` | `rId1` de type `http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument` → `word/document.xml` |
| `word/_rels/document.xml.rels` | `rId1` de type `…/relationships/styles` → `styles.xml` |
| `word/styles.xml` | `docDefaults` (Calibri 11 pt), style `Normal`, `Heading1` à `Heading3` (gras, 20/16/13 pt, `keepNext`, `outlineLvl` 0/1/2) |
| `word/document.xml` | `w:body` : titre = `<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>…`; paragraphe = `<w:p><w:r><w:t xml:space="preserve">…`; puce = paragraphe à retrait suspendu avec « • » littéral + `<w:tab/>` ; premier bloc de chaque page à partir de la 2ᵉ : `<w:pageBreakBefore/>` dans `w:pPr` (pas de paragraphe vide parasite) ; `w:sectPr` final avec `w:pgSz` = taille de la première page exportée en vingtièmes de point (`pt × 20`) et marges de 1 pouce (1440) |

Les noms de types de contenu et de relations proviennent de la norme (ISO/IEC 29500) et de la page Microsoft « Structure of a WordprocessingML document » (consultée : document minimal = `document` / `body` / `p` / `r` / `t`) **[types de contenu et URI de relations non re-vérifiés en ligne ; validation = ouverture dans Word installé sur ce poste, test manuel 8.3]**.

Échappement XML (`xml_escape`) : `& < > "` et **suppression** des caractères interdits en XML 1.0 (contrôles < 0x20 sauf `\t`, `\n`, `\r`, U+FFFE, U+FFFF) : sans cela Word déclare le fichier corrompu. Les `\t` du texte deviennent `<w:tab/>`.

Écrivain ZIP (`zip_archive`, ~60 lignes, format PKWARE APPNOTE : en-tête local `0x04034b50`, répertoire central `0x02014b50`, fin de répertoire `0x06054b50`, petit-boutiste ; consultés dans APPNOTE.TXT) :

- Par entrée : `let mut crc = flate2::Crc::new(); crc.update(data); crc.sum()` — F2:lib.rs:144 (réexport), F2:crc.rs:8-45 (`new`, `update`, `sum`).
- DEFLATE brut (méthode 8, sans en-tête zlib) : `flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default())`, `write_all`, `finish()` — F2:lib.rs:186 (réexport `write::DeflateEncoder`), F2:deflate/write.rs:30 (struct), :40 (`new(w, level)`), :125 (`finish`). Si le résultat n'est pas plus petit que l'entrée → méthode 0 (stockée).
- Version nécessaire 20, drapeaux `0x0800` (noms UTF-8), date/heure DOS fixes 1980-01-01 00:00 (`date = 0x0021`, `time = 0`) → archive **déterministe** (testable octet pour octet).
- Pas de ZIP64 : toute taille ou tout décalage > `u32::MAX` → `EngineError::Other("export too large")`.

Implications : fonctions pures hors `export_document` ; aucune écriture dans le document, aucun instantané.

### 3.5 `compose.rs` — page vierge, texte et Markdown → PDF

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize { A3, A4, A5, Letter, Legal }
impl PaperSize {
    pub const ALL: [PaperSize; 5];
    pub fn id(self) -> &'static str;                    // "a3" "a4" "a5" "letter" "legal"
    pub fn from_id(id: &str) -> Option<Self>;
    pub fn points(self, landscape: bool) -> (f32, f32); // (largeur, hauteur) en points
}
// Portrait : A3 841.89 x 1190.55 ; A4 595.28 x 841.89 ; A5 419.53 x 595.28 ; Letter 612 x 792 ; Legal 612 x 1008
// (ISO 216 : mm x 72 / 25,4). Paysage = permutation.

pub const MAX_BLANK_PAGES: usize = 500;
pub const MAX_TEXT_INPUT_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_TEXT_PAGES: usize = 2000;

#[derive(Debug, Clone)]
pub struct TextPdfOptions { pub paper: PaperSize, pub landscape: bool, pub markdown: bool,
                            pub font_size_pt: f32 /* 8..=18, défaut 11 */, pub margin_pt: f32 /* défaut 56.7 = 2 cm */ }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPdfReport { pub pages: usize, pub replaced_chars: usize }

impl Engine {
    pub fn create_blank(&self, output: &Path, paper: PaperSize, landscape: bool, pages: usize)
        -> Result<usize, EngineError>;
    pub fn text_to_pdf(&self, input: &Path, output: &Path, opts: &TextPdfOptions)
        -> Result<TextPdfReport, EngineError>;
}
```

`create_blank` (validation : `pages` dans `1..=MAX_BLANK_PAGES` sinon `Other("page count must be between 1 and 500")`) :

1. `let mut doc = self.pdfium.create_new_pdf()?;` — même départ que `images_to_pdf` (convert.rs:29).
2. `doc.pages_mut().create_page_at_end(PdfPagePaperSize::new_custom(PdfPoints::new(w), PdfPoints::new(h)))?` — `create_page_at_end` : PR:pdf/document/pages.rs:223 (délègue à `FPDFPage_New`, pages.rs:232-253) ; `new_custom` : PR:pdf/document/page/size.rs:425 ; motif identique à convert.rs:37-42. La `PdfPage` retournée est lâchée aussitôt : aucune régénération de contenu à faire (page sans objet).
3. `write_atomic(output, &doc.save_to_bytes()?)` (pages.rs:63). Retourne le nombre de pages.

`text_to_pdf` :

1. **Lecture** : `metadata.len() > MAX_TEXT_INPUT_BYTES` → `Other("input too large")`. BOM UTF-16 (`FF FE` / `FE FF`) → `UnsupportedFormat("utf-16")` (limite assumée). Décodage : `std::str::from_utf8(&bytes)` (BOM UTF-8 retiré) sinon `encoding_rs::WINDOWS_1252.decode_without_bom_handling(&bytes).0` — ER:lib.rs:1905 (`WINDOWS_1252`), ER:lib.rs:3134 (`decode_without_bom_handling`). C'est le cas des `.txt` ANSI créés par le Bloc-notes historique, fréquents en France.
2. **Assainissement WinAnsi** : le texte est écrit avec les polices standard PDFium (`FPDFText_LoadStandardFont`, encodage WinAnsi, cf. commentaire edit.rs:194). Caractères conservés : U+0020-U+007E, U+00A0-U+00FF et les 27 caractères de Windows-1252 hors Latin-1 (`€ ‚ ƒ „ … † ‡ ˆ ‰ Š ‹ Œ Ž ‘ ’ “ ” • – — ˜ ™ š › œ ž Ÿ`) ; tabulation → 4 espaces ; `\r` supprimé ; tout autre caractère → `?` et `replaced_chars += 1`. **[non vérifié]** que PDFium encode bien les 27 caractères Windows-1252 : le test 8.2 relit le texte extrait et fixe la liste réellement garantie (repli : restreindre l'ensemble conservé au Latin-1).
3. **Analyse** : `parse_markdown(&str) -> Vec<MdBlock>` (Markdown) ou `parse_plain(&str) -> Vec<MdBlock>` (texte : une ligne source = une ligne, police Courier, retours à la ligne aux espaces, lignes blanches conservées). Sous-ensemble Markdown : titres `#` à `######`, paragraphes, puces `-`/`*`/`+` (3 niveaux de retrait), listes numérotées, blocs de code ``` ou ~~~, citations `>` (retrait), filets `---` (simple espace vertical). Formatage en ligne (fonction `strip_inline`) : `**x**` → `x`, `` `x` `` → `x`, `*x*` → `x` (jamais `_x_` : noms en snake_case), `[t](u)` → `t (u)`, `![a](u)` → `a`. **Non rendus** : tableaux (affichés en texte), HTML, images, notes, titres soulignés (setext). Ces limites sont affichées dans le dialogue (note `convert.create.note_text`).
4. **Polices** : `let regular = doc.fonts_mut().helvetica(); let bold = doc.fonts_mut().helvetica_bold(); let mono = doc.fonts_mut().courier();` — PR:pdf/document/fonts.rs:194, 200, 218 ; `fonts_mut` : PR:pdf/document.rs:290. `PdfFontToken` est `Copy` (PR:pdf/document/fonts.rs:85-86) : les jetons sont pris **avant** de créer des pages, ce qui évite le conflit d'emprunt que edit.rs:210 contourne de la même façon.
5. **Mesure de largeur** (clé de la justesse des retours à la ligne) : largeur d'un caractère = largeur d'un objet texte à 1000 pt divisée par 1000, mise en cache par `(Face, char)` ; largeur d'une chaîne = somme (les polices de base PDF n'ont pas de crénage appliqué par PDFium). Appels : `PdfPageTextObject::new(&doc, c.to_string(), token, PdfPoints::new(1000.0))` — PR:pdf/document/page/object/text.rs:197 ; `object.width()` — PR:pdf/document/page/object.rs:683 (`bounds()` → `bounds_impl`, PR:pdf/document/page/object/private.rs:321-338, qui appelle `FPDFPageObj_GetRotatedBounds` pour un objet texte). Marge de sécurité : largeur utile × 0,99 (PDFium substitue des polices Foxit « compatibles » aux métriques Helvetica/Courier ; **[non vérifié]**). **Risque R1** : la lecture des limites d'un objet texte **non rattaché à une page** n'est pas garantie par la documentation. Contrôle obligatoire à l'étape 4 (test : `largeur("iiii") < largeur("MMMM")`, `largeur("MM") ≈ 2 × largeur("M")`, `largeur(" ") > 0`). Repli prévu si `width()` échoue ou renvoie 0 : document jetable `self.pdfium.create_new_pdf()`, une page, `page.objects_mut().create_text_object(PdfPoints::ZERO, PdfPoints::ZERO, "W", token, PdfPoints::new(1000.0))` (PR:pdf/document/page/objects/common.rs:141 et 401-432) puis `.bounds()?.width()` ; le document jetable est lâché ensuite (aucun nettoyage de page nécessaire).
6. **Mise en page pure** : `layout(blocks, &Geometry, &mut dyn FnMut(Face, f32, &str) -> f32) -> Result<Vec<Vec<Placed>>, EngineError>` avec `Face { Regular, Bold, Mono }`, `Placed { x, y, face, size, text }`. Règles : largeur utile = largeur de page − 2 × marge ; interligne 1,25 × taille ; titres H1 à H6 = taille × `[2.0, 1.6, 1.3, 1.15, 1.05, 1.0]` en gras, espace avant 0,8 × taille (pas en haut de page) et après 0,35 × taille ; paragraphes séparés de 0,6 × taille de base ; code en Courier 0,9 × taille, coupé **au caractère** ; puces « • » en retrait 18 pt par niveau avec retrait suspendu 14 pt ; un titre ne reste jamais seul en bas de page (saut avant s'il ne reste pas de place pour 2 lignes) ; mot plus long que la ligne coupé au caractère ; saut de page quand la ligne suivante dépasserait la marge basse ; `> MAX_TEXT_PAGES` pages → `Other("output too long")` ; entrée sans caractère imprimable → `Other("empty input")`. Testable sans PDFium avec une fausse mesure (`|_, s, t| t.chars().count() as f32 * s * 0.5`).
7. **Dessin** : pour chaque page du plan : `let mut page = doc.pages_mut().create_page_at_end(PdfPagePaperSize::new_custom(..))?;` puis `page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::AutomaticOnDrop)` (PR:pdf/document/page.rs:909 ; enum : page.rs:150 — évite une régénération du flux à chaque objet, coût quadratique sur une page de 60 lignes) ; pour chaque `Placed` non vide : `let mut o = page.objects_mut().create_text_object(PdfPoints::new(x), PdfPoints::new(y), &text, token, PdfPoints::new(size))?; o.set_fill_color(PdfColor::new(0, 0, 0, 255))?;` (création + `translate` + ajout : PR:objects/common.rs:401-432 ; couleur explicite comme edit.rs:215 et edit.rs:308). **Jamais d'objet texte vide** : PR:object/text.rs:191-195 documente qu'`FPDFText_LoadPage` plante si une page en contient un. Avant de lâcher la page : `page.regenerate_content()?` (PR:page.rs:942) pour ne rien perdre.
8. **Sortie** : `let bytes = doc.save_to_bytes()?;` puis titre = nom du fichier sans extension via `security::set_metadata(&bytes, None, &Metadata { title: Some(stem), ..Default::default() })` (security.rs:215, motif de worker.rs:547) puis `write_atomic`.

Implications : document neuf, jamais le document ouvert ; pas d'instantané ni d'annulation. Le fichier produit est réouvert par l'UI (`open_path`) comme dans `on_batch_done` (tools.rs:496-503).

### 3.6 `office.rs` — Office et OpenDocument → PDF via un convertisseur installé

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ConverterKind { LibreOffice, MicrosoftOffice }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum OfficeApp { Word, Excel, PowerPoint }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converter { pub kind: ConverterKind, pub path: PathBuf, pub apps: Vec<OfficeApp> /* vide pour LibreOffice */ }
#[derive(Debug, Clone)]
pub struct OfficeOptions { pub timeout: Duration, pub prefer: Option<ConverterKind> }
impl Default for OfficeOptions { /* timeout: 180 s, prefer: None */ }

pub const OFFICE_EXTENSIONS: &[&str] =
    &["doc", "docx", "rtf", "odt", "txt", "htm", "html", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp"];
pub fn office_app(path: &Path) -> Option<OfficeApp>;        // doc docx rtf odt txt htm html → Word ; xls xlsx ods csv → Excel ; ppt pptx odp → PowerPoint
pub fn detect_converters() -> Vec<Converter>;               // lecture de fichiers uniquement, < 1 ms : appelable depuis le thread UI
pub fn choose_converter<'a>(c: &'a [Converter], app: OfficeApp, prefer: Option<ConverterKind>) -> Option<&'a Converter>;
pub fn convert_to_pdf(input: &Path, output: &Path, opts: &OfficeOptions, control: Option<&JobControl>)
    -> Result<ConverterKind, EngineError>;                  // n'utilise PAS PDFium : appelable hors thread moteur
```

**Détection** (`detect_converters`, aucune API Windows ni sous-processus) :

- Variable de développement `SUNIPDF_OFFICE_DISABLE=1` → liste vide (permet de tester le message « aucun convertisseur » sur ce poste, où Word est installé).
- LibreOffice : `SUNIPDF_SOFFICE` (chemin explicite, installations portables) → sinon recherche de `soffice.exe` dans `PATH` (`std::env::split_paths`) → sinon `%ProgramFiles%\LibreOffice\program\soffice.exe` et `%ProgramFiles(x86)%\…`. **[non vérifié : absent de ce poste]**.
- Microsoft Office : pour `racine` dans `%ProgramFiles%`, `%ProgramFiles(x86)%` × disposition dans `Microsoft Office\root\Office{v}` (Click-to-Run), `Microsoft Office\Office{v}` (MSI) × `v` dans `16, 15, 14` : `WINWORD.EXE`, `EXCEL.EXE`, `POWERPNT.EXE` ; `is_file()` remplit `apps`. **Vérifié ici** : `C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE` existe (recherche récursive). La détection par fichiers ne prouve pas que COM est utilisable ; si l'activation échoue, l'erreur est `ConverterFailed` avec le message de PowerShell.

**Choix** (`choose_converter`) : `prefer` si disponible ; sinon formats natifs Microsoft (`doc docx xls xlsx ppt pptx`) → Microsoft Office si l'application correspondante est présente, sinon LibreOffice ; autres formats (`odt ods odp rtf txt htm html csv`) → LibreOffice d'abord, sinon Microsoft Office si l'application est présente. Aucun candidat → `ConverterUnavailable`. Extension hors `OFFICE_EXTENSIONS` → `UnsupportedFormat(ext)`.

**Invocation** (`convert_to_pdf`) :

1. Dossier de travail `%TEMP%\sunipdf-office-<pid>-<nanosecondes>` (sous-dossiers `out`, `profile`), supprimé par un garde `Drop` (au mieux) même en cas d'erreur ou d'annulation. Chemins en absolu par `std::path::absolute` (stable depuis Rust 1.79 ; `rust-version = 1.85` du workspace), **pas** `canonicalize` (préfixe `\\?\` mal accepté par Office).
2. **LibreOffice** : `soffice.exe --headless --norestore --nolockcheck -env:UserInstallation=<URL file:/// du dossier profile> --convert-to pdf --outdir <tmp\out> <entrée>`. Options relevées dans l'aide officielle « Command Line Parameters » (https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html, consultée) : `--headless`, `--convert-to OutputFileExtension[:OutputFilterName[:OutputFilterParams]]`, `--outdir`, `-env:VAR[=VALUE]` (exemple `-env:UserInstallation=file:///tmp/test`), `--norestore`, `--nolockcheck`. La page ne montre pas d'exemple combinant `--headless` et `--convert-to` : combinaison **[non vérifiée ici]** mais usage courant. Le profil jetable évite que la commande soit confiée à une instance LibreOffice déjà ouverte avec le même profil **[comportement non vérifié]**. URL : fonction `file_url(&Path)` (antislashs → `/`, tout octet hors `[A-Za-z0-9-._~/:]` encodé en `%XX` d'après son UTF-8) ; test `C:\a b\é` → `file:///C:/a%20b/%C3%A9`. Sortie attendue : `<tmp>\out\<nom sans extension>.pdf`.
3. **Microsoft Office** : le script `office_convert.ps1` (embarqué par `include_str!`, écrit dans le dossier de travail) est lancé par `C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <tmp>\convert.ps1 -In <entrée> -Out <tmp>\out\result.pdf -App word|excel|powerpoint`. Les chemins sont des arguments distincts de `-File` (jamais interpolés dans une commande : pas d'injection) ; `-ExecutionPolicy Bypass` est nécessaire : sur ce poste les scopes machine et utilisateur sont `Undefined` (donc `Restricted` par défaut sur un Windows client pour un `.ps1`) ; le `Bypass` observé au niveau `Process` vient de la session d'outillage et ne peut pas être supposé chez l'utilisateur.

```powershell
param([string]$In, [string]$Out, [string]$App)
$ErrorActionPreference = 'Stop'
$code = 0; $office = $null; $doc = $null
try {
  switch ($App) {
    'word' {
      $office = New-Object -ComObject Word.Application
      $office.Visible = $false; $office.DisplayAlerts = 0          # wdAlertsNone
      $office.AutomationSecurity = 3                                # msoAutomationSecurityForceDisable
      # Documents.Open(FileName, ConfirmConversions, ReadOnly, AddToRecentFiles, PasswordDocument)
      $doc = $office.Documents.Open($In, $false, $true, $false, 'sunipdf-no-password')
      $doc.ExportAsFixedFormat($Out, 17)                            # wdExportFormatPDF
    }
    'excel' {
      $office = New-Object -ComObject Excel.Application
      $office.Visible = $false; $office.DisplayAlerts = $false
      $office.AutomationSecurity = 3
      # Workbooks.Open(FileName, UpdateLinks, ReadOnly, Format, Password)
      $doc = $office.Workbooks.Open($In, 0, $true, [Type]::Missing, 'sunipdf-no-password')
      $doc.ExportAsFixedFormat(0, $Out)                             # xlTypePDF
    }
    'powerpoint' {
      $office = New-Object -ComObject PowerPoint.Application
      $office.AutomationSecurity = 3
      # Presentations.Open(FileName, ReadOnly, Untitled, WithWindow)
      $doc = $office.Presentations.Open($In, -1, 0, 0)
      $doc.SaveAs($Out, 32)                                         # ppSaveAsPDF
    }
    default { throw "unknown app $App" }
  }
} catch { [Console]::Error.WriteLine($_.Exception.Message); $code = 2 }
finally {
  if ($doc)    { try { $doc.Close() } catch {} }
  if ($office) { try { $office.Quit() } catch {} }
}
exit $code
```

Constantes et signatures relues sur Microsoft Learn : `Document.ExportAsFixedFormat(OutputFileName, ExportFormat, …)` et `wdExportFormatPDF = 17` ; `Documents.Open(FileName, ConfirmConversions, ReadOnly, AddToRecentFiles, PasswordDocument, …)` ; `Workbook.ExportAsFixedFormat(Type, FileName, …)` et `xlTypePDF = 0` ; `Workbooks.Open(FileName, UpdateLinks, ReadOnly, Format, Password, …)` (`UpdateLinks = 0` : liens externes non mis à jour) ; `Presentations.Open(FileName, ReadOnly, Untitled, WithWindow)` (MsoTriState) ; `ppSaveAsPDF = 32` ; `MsoAutomationSecurity` : `Low = 1` (défaut, **macros activées à l'ouverture programmatique**), `ByUI = 2`, `ForceDisable = 3`. Le réglage `AutomationSecurity = 3` est donc obligatoire ; l'effet réel (macros bloquées) n'est **pas testé automatiquement** et ne doit pas être présenté comme une garantie de sécurité avant test manuel avec un document contenant une macro d'ouverture.

4. **Supervision** : `Command` avec `stdin(Stdio::null())`, sortie standard et erreur standard redirigées vers **un fichier journal** du dossier de travail (`File::try_clone`) — pas de tube, donc pas d'interblocage si la sortie dépasse 64 Ko ; sous Windows `CommandExt::creation_flags(0x0800_0000)` (`CREATE_NO_WINDOW`) pour qu'aucune console ne clignote. Boucle : `child.try_wait()` toutes les 50 ms ; `control.is_cancelled()` → `child.kill()` + `wait()` → `Err(Cancelled)` ; échéance dépassée → `kill()` → `Err(ConverterTimeout)` ; code de sortie ≠ 0 → `Err(ConverterFailed(<400 derniers caractères du journal>))`.
5. **Résultat** : le PDF attendu doit exister, commencer par `%PDF-` et ne pas être vide ; il est alors recopié par `write_atomic(output, &bytes)` (jamais d'écriture directe de la sortie par le convertisseur : pas de PDF partiel chez l'utilisateur, pas d'écrasement silencieux).

Limites assumées à écrire dans KNOWN_ISSUES : (a) tuer PowerShell (annulation ou délai) ne tue pas `WINWORD.EXE`/`EXCEL.EXE`/`POWERPNT.EXE` (lancés par DCOM, pas enfants de PowerShell) : l'instance d'automatisation se termine normalement quand son client disparaît, mais une boîte de dialogue modale invisible peut la retenir **[non vérifié]** ; (b) document protégé par mot de passe : le mot de passe factice vise à provoquer une erreur plutôt qu'une invite modale invisible **[non vérifié : test manuel 8.3 avec Word installé ici]** ; (c) si SuniPDF est lancé « en tant qu'administrateur » et Office non, l'activation COM peut échouer ; (d) politiques d'entreprise (AppLocker, mode de langage contraint de PowerShell) peuvent bloquer le script : erreur `ConverterFailed` ; (e) classeurs et présentations sont paginés comme à l'impression.

### 3.7 `lib.rs` et `command.rs`

- `lib.rs` (déclarations lib.rs:10-19) : `pub mod compose; pub mod export; pub mod job; pub mod office; pub mod structure;` ; ré-exports (lib.rs:21-30) : `pub use compose::{PaperSize, TextPdfOptions, TextPdfReport}; pub use export::{ExportFormat, ExportReport}; pub use job::JobControl; pub use structure::{Block, BlockKind, ExtractOptions, ExtractStats, StructuredText, TextLine};`. `office` reste adressé par chemin (`sunipdf_core::office::…`) comme `security`.
- `edit.rs` : **aucun changement** (`compose.rs` prend directement les jetons `helvetica()`, `helvetica_bold()`, `courier()` ; `font_token`, edit.rs:142, reste privée).
- `command.rs` : variantes `ExportDocument` et `CreateDocument` dans `enum Command` (command.rs:5-69) et dans `REGISTRY` après `ExportImages` (command.rs:438-445) :

```rust
cmd!(CreateDocument, "convert.create", "menu.file.create", None, false, Convert),
cmd!(ExportDocument, "convert.export", "menu.file.export", None, true,  Convert),
```

Le test `Command::in_menu(Menu::File).count() == 7` (command.rs:648) n'est pas touché (menu `Convert`). `command_color` donne automatiquement `CAT_CONVERT` (shell.rs:181). Aucun raccourci clavier (évite les conflits avec les autres tranches).

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

**Aucun nouveau `EditOp`, `Request` ni `Response`** : ces opérations ne modifient pas le document (décision 8). Seuls `BatchOp` et `BatchResult` s'étendent (worker.rs:31-65). La conversion Office n'y passe pas (elle n'utilise pas PDFium et ne doit pas bloquer le rendu : section 5.3).

```rust
pub enum BatchOp {
    /* … Combine, Split, Extract, CreateFromImages, ExportImages, Optimize (inchangés) … */
    ExportText { format: ExportFormat, pages: Option<Vec<usize>>, drop_repeated: bool,
                 out: PathBuf, control: Arc<JobControl> },
    CreateBlank { out: PathBuf, paper: PaperSize, landscape: bool, pages: usize },
    CreateFromText { input: PathBuf, out: PathBuf, options: TextPdfOptions },
}
pub enum BatchResult {
    /* … existants … */
    ExportedText { out: PathBuf, report: ExportReport },
    CreatedFromText { out: PathBuf, report: TextPdfReport },
    // CreateBlank réutilise Created { out, pages } (tools.rs:496-503 : message + ouverture de l'onglet)
}
```

Branchement dans `run_batch` (worker.rs:559-616), même style que les arms existants :

```rust
BatchOp::ExportText { format, pages, drop_repeated, out, control } => {
    let doc = doc.ok_or(EngineError::Other("no document".into()))?;
    let report = doc.export_document(format, &ExtractOptions { pages, drop_repeated }, &out, Some(&control))?;
    Ok(BatchResult::ExportedText { out, report })
}
BatchOp::CreateBlank { out, paper, landscape, pages } => {
    let n = engine.create_blank(&out, paper, landscape, pages)?;
    Ok(BatchResult::Created { out, pages: n })
}
BatchOp::CreateFromText { input, out, options } => {
    let report = engine.text_to_pdf(&input, &out, &options)?;
    Ok(BatchResult::CreatedFromText { out, report })
}
```

Imports à compléter (worker.rs:9-13) : `ExportFormat, ExportReport, ExtractOptions, JobControl, PaperSize, TextPdfOptions, TextPdfReport`. `BatchOp` doit rester `Send` (`Arc<JobControl>` composé d'atomiques : oui).

Points d'attention :

- `run_batch` reçoit `state.document.as_ref()` (worker.rs:729) : `CreateBlank` et `CreateFromText` fonctionnent **sans document ouvert** (ils n'utilisent que `engine`) ; `ExportText` exige un document (`no document` sinon).
- L'export lit l'état courant en mémoire (modifications non enregistrées comprises), comme `Split`, `Extract` et `ExportImages`.
- Le thread moteur est occupé pendant l'export : le rendu de la page affichée attend la fin (même comportement que `Optimize`). Le bouton Annuler (barre d'état) positionne `JobControl::cancel`, lue à chaque page ; l'export répond `BatchResult::Failed(EngineError::Cancelled)`.
- Les onglets : un `Suspend` envoyé pendant l'export est mis en file derrière lui (file du worker, worker.rs:638-659). Le document utilisé par l'export est donc bien celui de l'onglet actif au lancement.
- Historique d'annulation (undo/redo) : `State::edit` (worker.rs:372-393) n'est pas utilisé ; `State.undo/redo` ne bougent pas.

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Fichiers à toucher

| Fichier | Changement |
|---|---|
| `src/export.rs` (**nouveau**) | `ExportState`, `ExportTarget`, `impl SuniApp { export_dialog, export_submit, on_export_done }` |
| `src/create.rs` (**nouveau**) | `CreateState`, `CreateMode`, `OfficeJob`, `impl SuniApp { create_dialog, create_submit, start_office_job, poll_office_job, on_text_created }` |
| `src/main.rs` | `mod create; mod export;` (liste main.rs:3-14, ordre alphabétique) |
| `src/app.rs` | champs `pub export: crate::export::ExportState`, `pub create: crate::create::CreateState` dans `SuniApp` (app.rs:82-127) et dans le constructeur (valeurs `Default::default()`, motif de `tools:` app.rs:148-152) ; arms `execute` (app.rs:943-955) ; garde de `handle_input` (app.rs:591-596 : ajouter `|| self.export.open || self.create.open`) ; appels dans `update` à côté de `split_dialog`/`stamp_dialog` (app.rs:1876-1877) ; `error_key` (app.rs:1924) : 6 arms (`Cancelled` → `"convert.cancelled"`, jamais affiché comme erreur) ; test `every_error_key_exists_in_both_catalogs` (app.rs:1943) : ajouter les 6 variantes à la liste ; test `ui_keys_used_by_app_exist` (app.rs:1964) : ajouter `include_str!("export.rs")` et `include_str!("create.rs")` à la concaténation des sources contrôlées |
| `src/tools.rs` | `ToolsState.job: Option<Arc<JobControl>>` (tools.rs:13-20) ; nouvel assistant `save_dialog_ext` ; arms `ExportedText` / `CreatedFromText` / `Failed(Cancelled)` dans `on_batch_done` (tools.rs:470-533) |
| `src/shell.rs` | lignes d'outils (`ToolTab::Convert` shell.rs:97-102 et `ToolTab::All` shell.rs:48-76) ; `command_icon` (shell.rs:246-308, match **exhaustif** : 2 arms) ; `status_bar` (shell.rs:1122-1192) : progression + Annuler dans **les deux** branches (`(_, Some(doc), None)` shell.rs:1144 et `(_, None, None)` shell.rs:1181) |
| `src/worker.rs` | section 4 |
| `src/dialog.rs`, `src/theme.rs`, `src/icons.rs` | inchangés (glyphes existants : `SAVE_AS` pour Exporter, `DOCUMENT` pour Créer ; aucun code point de police nouveau, donc rien d'inventé) |

### 5.2 Boîte de dialogue « Exporter » (`export.rs`)

État :

```rust
#[derive(Clone, Copy, PartialEq, Eq)] pub enum ExportTarget { Docx, Markdown, Html, Text, Png }
pub struct ExportState { pub open: bool, pub target: ExportTarget, pub current_only: bool,
                         pub drop_repeated: bool, pub dpi: f32 }
// Default : open=false, target=Docx, current_only=false, drop_repeated=true, dpi=150.0 (impl Default écrite à la main)
```

Ouverture : `Command::ExportDocument => self.export.open = true` (`needs_document = true`, donc `execute` a déjà ouvert un fichier au besoin).

Construction : `dialog::Spec::new("export", tr(lang, "convert.export.title"), self.settings.dark_theme, self.has_icons).subtitle(tr(lang, "convert.export.subtitle")).icon(icons::glyph::SAVE_AS, "⇪").width(560.0)` puis `dialog::show(ctx, spec, body, footer)` — jamais `egui::Modal` direct (D020). Même structure que `stamp_dialog` (tools.rs:141-329) : drapeaux `submit` / `cancel` locaux, Échap = annuler, Entrée = valider (aucun champ de saisie dans ce dialogue).

Corps, de haut en bas :

1. `dialog::field_label(ui, tr(lang, "convert.export.format"))` puis cinq `ui.radio_value(&mut st.target, ExportTarget::X, tr(lang, "convert.export.format.x"))`.
2. Une ligne grisée (`RichText::size(12.0).color(palette.text_muted)`, comme `stamp.placeholders`, tools.rs:203-207) avec la **phrase de fidélité** du format sélectionné (`convert.export.fidelity.<x>`). C'est l'endroit où la limite est annoncée avant l'action.
3. `dialog::field_label(.., "convert.export.scope")` + deux `radio_value(&mut st.current_only, false/true, …)` ; désactivés (`ui.add_enabled_ui(false, …)`) pour PNG, dont l'outil existant traite toujours tout le document.
4. Si le format est Markdown, Html ou Docx : `ui.checkbox(&mut st.drop_repeated, tr(lang, "convert.export.drop_repeated"))`. Si PNG : `DragValue` résolution `72.0..=600.0`.
5. Note grisée `convert.export.note` (PDF numérisé sans texte ; modifications non enregistrées incluses).

Pied : `dialog::primary(ui, tr(lang, "convert.export.ok"))` puis `dialog::secondary(ui, tr(lang, "dialog.password.cancel"))` (clé existante, tools.rs:283).

Validation (`export_submit`), motif de `tool_split_submit` (tools.rs:331-347) : **le dialogue ne se ferme qu'une fois la destination choisie** ; si le sélecteur natif est annulé, le dialogue reste ouvert.

- PNG : `rfd::FileDialog::new().set_title(self.t("convert.export.pick_dir")).pick_folder()` → `self.run_batch(BatchOp::ExportImages { out_dir, dpi }, "convert.running.export")`.
- Autres formats : `save_dialog_ext("convert.export.save.<x>", &suggestion, "convert.export.filter.<x>", ext)` (copie généralisée de `save_dialog`, tools.rs:105-118 : filtre `ext`, répertoire et nom suggérés `<nom du PDF>.<ext>`, extension ajoutée si absente). Puis :

```rust
let control = Arc::new(JobControl::default());
self.tools.job = Some(control.clone());
let pages = if st.current_only { Some(self.target_pages()) } else { None };   // organize.rs:25-34
self.run_batch(BatchOp::ExportText { format, pages, drop_repeated: st.drop_repeated, out, control },
               "convert.running.export");
```

Fin (`on_batch_done`, tools.rs:470) :

- `ExportedText { out, report }` : `tools.job = None` ; message `convert.done.export` (`{name}`, `{pages}`), ou `convert.done.export_partial` si `report.empty_pages` n'est pas vide (`{empty}` = nombre) ; suffixes ` · ` + `convert.done.dropped` si `dropped_repeated > 0` et ` · ` + `convert.done.rotated` si `skipped_rotated > 0`. Le fichier n'est pas ouvert dans SuniPDF (ce n'est pas un PDF).
- `Failed(EngineError::Cancelled)` : `status_msg = convert.cancelled`, pas de bandeau d'erreur.
- `Failed(NoText)` etc. : `self.error = Some(error_key(&e))` (code existant, tools.rs:528-531).

Limite : l'export PNG reste l'outil existant (`BatchOp::ExportImages`, worker.rs:597-602) : ni progression ni annulation, toutes les pages. Les deux seront ajoutables plus tard en lui passant le même `JobControl`.

Phrases de fidélité à afficher (textes exacts en section 7) : DOCX/Markdown/HTML « approchée », TXT « texte exact, sans mise en forme », PNG « rendu exact, texte non sélectionnable ». Aucun pourcentage.

### 5.3 Boîte de dialogue « Créer un PDF » (`create.rs`)

État :

```rust
#[derive(Clone, Copy, PartialEq, Eq)] pub enum CreateMode { Blank, Text, Images, Office }
pub struct CreateState {
    pub open: bool, pub mode: CreateMode,
    pub paper: PaperSize, pub landscape: bool, pub pages: usize, pub font_size: f32,
    pub source: Option<PathBuf>,                       // fichier texte/Markdown ou Office choisi
    pub converters: Vec<sunipdf_core::office::Converter>,   // redétectés à chaque ouverture du dialogue
    pub job: Option<OfficeJob>,
}
// Default : mode=Blank, paper=A4, landscape=false, pages=1, font_size=11.0
```

Ouverture : `Command::CreateDocument => { self.create.open = true; self.create.converters = sunipdf_core::office::detect_converters(); }` (la détection se fait **à chaque ouverture du dialogue**, sans cache : quelques `is_file()`, coût négligeable, et une installation faite pendant la session est prise en compte).

Construction : `Spec::new("create", tr(lang, "convert.create.title"), …).subtitle(tr(lang, "convert.create.subtitle")).icon(glyph::DOCUMENT, "+").width(560.0)`.

Corps :

1. Rangée de 4 `ui.selectable_value(&mut st.mode, CreateMode::X, tr(lang, "convert.create.mode.x"))` (boutons segmentés).
2. Selon le mode :
   - **Blank** : `ComboBox::from_id_salt("create-paper")` sur `PaperSize::ALL` (`convert.create.paper.<id>`, motif de `position_combo`, tools.rs:61-69) ; deux `radio_value` portrait/paysage ; `DragValue` pages `1..=500` ; ligne grisée de dimensions en points non traduite (« 595 × 842 pt » : nombres, exception documentée par I18N.md).
   - **Text** : `dialog::field_label(ui, tr(lang, "convert.create.source"))`, bouton `convert.create.browse` (« Choisir… ») → `rfd::FileDialog::new().set_title(self.t("convert.create.pick_text")).add_filter(self.t("convert.create.filter_text"), &["txt", "md", "markdown"]).pick_file()` ; nom du fichier ou `convert.create.no_file` ; format de page, orientation, `DragValue` taille `8.0..=18.0` ; note `convert.create.note_text`.
   - **Images** : note `convert.create.note_images` ; le bouton principal affiche `convert.create.ok_images` et appelle `self.create.open = false; self.tool_create_from_images()` (tools.rs:361-381) — aucun doublon de logique.
   - **Office** : première ligne = `convert.create.converter_found` avec `{name}` = `tr(lang, "convert.converter.office" | "convert.converter.libreoffice")` pour le convertisseur **qui sera utilisé pour le fichier choisi** (`choose_converter`) ou, avant le choix d'un fichier, la liste des convertisseurs détectés séparés par « , » ; si la liste est vide : texte rouge `convert.create.converter_none` et bouton principal désactivé. Puis `convert.create.source`, bouton `convert.create.browse` → `rfd::FileDialog::new().set_title(self.t("convert.create.pick_office")).add_filter(self.t("convert.create.filter_office"), OFFICE_EXTENSIONS).pick_file()`, nom du fichier (ou `convert.create.no_file`), note `convert.create.note_office`.
3. Aucun bouton factice : le bouton principal est `ui.add_enabled(…)` dans les cas incomplets (pas de fichier choisi ; Office sans convertisseur ; travail déjà en cours `self.tools.running.is_some()`). `dialog::primary` ne gère pas l'état désactivé : pour ces cas, dessiner `egui::Button` grisé via `ui.add_enabled(false, …)` avec le même habillage ou utiliser `ui.add_enabled_ui(enabled, |ui| dialog::primary(ui, …))` (compatible avec le widget actuel : `ui.add` sous `add_enabled_ui` est automatiquement inactif).

Validation (`create_submit`) :

- **Blank** : `save_dialog("convert.create.save", &PathBuf::from("document.pdf"))` (tools.rs:105-118 inchangé) → `run_batch(BatchOp::CreateBlank { out, paper, landscape, pages }, "tools.running.images")` (clé existante « Création du PDF… »). Fin : `BatchResult::Created` → message `tools.done.images` + `open_path` (code existant, tools.rs:496-503).
- **Text** : `save_dialog` avec suggestion `<source>.pdf` → `run_batch(BatchOp::CreateFromText { input, out, options: TextPdfOptions { markdown: ext ∈ {md, markdown}, … } }, "tools.running.images")`. Fin : `CreatedFromText { out, report }` → `tools.done.images` ou, si `report.replaced_chars > 0`, `convert.done.text_replaced` ; puis `open_path`.
- **Office** : `save_dialog` → `start_office_job(ctx, input, out)` :

```rust
pub struct OfficeJob { rx: mpsc::Receiver<Result<ConverterKind, EngineError>>, pub control: Arc<JobControl>, out: PathBuf }
impl Drop for OfficeJob { fn drop(&mut self) { self.control.cancel(); } }  // fermeture de l'application => le processus enfant est tué
fn start_office_job(&mut self, ctx: &egui::Context, input: PathBuf, out: PathBuf) {
    let control = Arc::new(JobControl::default());
    let (tx, rx) = mpsc::channel();
    let (c2, out2, ctx2) = (control.clone(), out.clone(), ctx.clone());
    std::thread::Builder::new().name("sunipdf-office".into()).spawn(move || {
        let r = sunipdf_core::office::convert_to_pdf(&input, &out2, &OfficeOptions::default(), Some(&c2));
        let _ = tx.send(r);
        ctx2.request_repaint();
    }).expect("thread office");
    self.tools.running = Some("convert.running.office");
    self.tools.job = Some(control.clone());
    self.create.job = Some(OfficeJob { rx, control, out });
}
```

`poll_office_job(ctx)` (appelée à chaque `update`, avant les dialogues) : `try_recv()` ; `Ok(Ok(kind))` → `tools.running = None`, `tools.job = None`, `status_msg = convert.done.office` (`{converter}`, `{name}`), `open_path(out, None)` ; `Ok(Err(Cancelled))` → `status_msg = convert.cancelled` ; `Ok(Err(e))` → `log::warn!`, `self.error = Some(error_key(&e))` (le détail, par exemple la fin du journal du convertisseur, reste dans le journal et la CLI) ; `Err(Disconnected)` → `error.unknown`. Tant que la tâche vit : `ctx.request_repaint_after(Duration::from_millis(250))` pour rafraîchir le spinner.

Le message de fin est effacé quand le résultat s'ouvre (`Response::Opened` remet `status_msg` à `None`, app.rs:352) : comportement déjà présent pour `CreateFromImages`, l'onglet qui s'ouvre sert de confirmation ; les messages d'export, eux, persistent puisque aucun PDF ne s'ouvre.

Cette tâche ne touche pas PDFium : elle ne viole ni « un seul thread moteur » ni « l'UI n'importe jamais pdfium-render » ; le PDF produit est ouvert ensuite par `Request::Open` comme tout fichier.

### 5.4 Barre d'état (`shell.rs`, `status_bar`)

Aujourd'hui `tools.running` ne s'affiche que quand un document est ouvert (shell.rs:1167-1171). À faire : extraire `fn running_strip(&self, ui: &mut egui::Ui, muted: &dyn Fn(String) -> RichText)` appelé dans les deux branches. Contenu : spinner + libellé `tr(lang, key)` ; si `tools.job` existe et que `progress().1 > 0` : `convert.running.export_progress` (`{done}`, `{total}`) ; lien « Annuler » (`convert.cancel`) dont le clic fait `job.cancel()` (`AtomicBool::store` : compatible avec `&self`). Sans cela, une création sans document ouvert (accueil) n'afficherait aucun témoin de travail.

### 5.5 Registre, panneau d'outils, menus, palette

- `command.rs` : 2 commandes (3.7). `command_menu` (app.rs:1016-1038) les liste dans le menu Convertir ; `needs_document` grise « Exporter… » dans le menu quand rien n'est ouvert mais le panneau d'outils reste actif et ouvre un fichier d'abord (app.rs:793-799).
- `ToolTab::Convert` (shell.rs:97-102) devient `[CombineFiles, CreateDocument, CreateFromImages, ExportDocument, ExportImages, Optimize]` ; `ToolTab::All` (shell.rs:48-76) : insérer `CreateDocument` avant `CreateFromImages` et `ExportDocument` avant `ExportImages`.
- `command_icon` : `Command::CreateDocument => (glyph::DOCUMENT, "+")`, `Command::ExportDocument => (glyph::SAVE_AS, "⇪")`. `command_color` : aucune modification (`Menu::Convert` → `CAT_CONVERT`).
- Carte d'accueil « Convertir » : seule la description change (clé existante `home.card.convert.desc`, section 7).

### 5.6 Onglets, annulation et état « modifié »

- Aucun `EditOp` : `dirty`, `can_undo`, `can_redo` inchangés ; rien dans `TabState` (tabs.rs:9-24). `ExportState` et `CreateState` sont globaux à l'application comme `ToolsState` (non échangés par `take_tab`/`put_tab`) car les dialogues sont modaux : aucun changement d'onglet possible pendant leur affichage.
- Une création terminée pendant que l'utilisateur est sur un autre onglet s'ouvre dans un nouvel onglet (`open_path` suspend l'onglet actif) : comportement existant de `CreateFromImages`.
- Fermeture de l'application avec une conversion en cours : `Drop` de `OfficeJob` → `cancel()` → le thread tue le processus enfant et nettoie le dossier temporaire.
- Accessibilité : widgets egui standard (radio, combo, boutons) exposés à AccessKit via leurs libellés traduits ; pas de clé `a11y.*` supplémentaire.

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Sorties JSON, messages en anglais (D009), codes d'erreur stables. Ajouts à `enum Command` (main.rs:21-265) et à `run` (main.rs:267) ; motif identique à `Images2pdf` (main.rs:533-536) et `ExportImages` (main.rs:537-546). Les valeurs textuelles (`--format`, `--paper`, `--converter`) sont des `String` résolues par `from_id` du moteur, comme `--position` de `stamp` (`StampPosition::from_id`) ; identifiant inconnu → `EngineError::Other("unknown format: …")`.

```text
sunipdf-cli export <file> --format txt|md|html|docx --out <path> [--pages all|1,3-5] [--keep-headers] [--password <pw>]
sunipdf-cli create-blank --out <path> [--paper a3|a4|a5|letter|legal] [--landscape] [--pages N]
sunipdf-cli text2pdf <input.txt|input.md> --out <path> [--format auto|text|markdown] [--paper …] [--landscape] [--font-size 11]
sunipdf-cli office2pdf <input> --out <path> [--converter auto|office|libreoffice] [--timeout 180]
sunipdf-cli converters
```

- `--pages` utilise `parse_pages` (main.rs:625-649) ; `all` → pas de sélection. `--keep-headers` désactive `drop_repeated`.
- `export` : `engine.open(&file, password)` puis `doc.export_document(format, &ExtractOptions { … }, &out, None)`.
- `create-blank` / `text2pdf` : `engine.create_blank(..)` / `engine.text_to_pdf(..)` ; `--format auto` choisit Markdown pour `.md` / `.markdown`.
- `office2pdf` : n'a pas besoin de PDFium pour convertir, mais `run` crée déjà un `Engine` (main.rs:278) et relit le PDF produit (`engine.open(&out, None)?.page_count()`) pour fournir `page_count` ; l'annulation n'existe pas en ligne de commande (Ctrl+C tue le processus ; le dossier temporaire peut subsister : limite documentée).
- `converters` : traité **avant** `Engine::new()` comme `Commands` (main.rs:268-277), car il ne demande pas `pdfium.dll`.

Sorties (clés stables ; numéros de page 1-based comme `search`) :

| Commande | JSON |
|---|---|
| `export` | `{ "out": "…", "format": "docx", "pages": 3, "chars": 1234, "headings": 2, "empty_pages": [2], "dropped_repeated": 4, "skipped_rotated": 0, "bytes": 5678 }` |
| `create-blank` | `{ "out": "…", "page_count": 3, "paper": "a4", "landscape": false }` |
| `text2pdf` | `{ "out": "…", "page_count": 7, "replaced_chars": 0 }` |
| `office2pdf` | `{ "out": "…", "converter": "microsoft-office" \| "libreoffice", "page_count": 12 }` |
| `converters` | `{ "converters": [ { "kind": "microsoft-office", "path": "C:\\…\\WINWORD.EXE", "apps": ["word","excel","powerpoint"] } ], "supported_extensions": ["doc","docx",…] }` (liste vide si aucun) |

Erreurs : JSON `{ "error": "<code>", "message": "…" }` sur stderr (main.rs:658-662). Codes de sortie ajoutés dans la correspondance de `main` (main.rs:663-668) : `converter_unavailable → 6`, `no_text → 7` ; `converter_failed`, `converter_timeout`, `unsupported_format`, `cancelled` → 1 (défaut). `sunipdf-cli commands` (main.rs:268-277) liste automatiquement `convert.export` et `convert.create`.

Test CLI : `parse_paper(&str)` / `parse_format(&str)` en tests unitaires (motif de `parses_page_specifications`, main.rs:675-690).

## 7. i18n (fr-FR et en-US)

Espace réservé `convert.*` (docs/I18N.md) + `menu.file.*` + `error.*`. Ajouter **les mêmes clés aux deux fichiers** `assets/i18n/fr-FR.json` et `assets/i18n/en-US.json` (le dernier élément actuel est `"status.stamped"` sans virgule finale : ajouter la virgule, insérer, refermer l'accolade). Les paramètres `{…}` doivent être identiques dans les deux langues (test `placeholders_match_between_languages`, crates/sunipdf-i18n/src/lib.rs). Aucun guillemet droit `"` à l'intérieur d'une valeur (utiliser « » en français et “ ” en anglais). Pour limiter les conflits avec les autres tranches qui éditent les mêmes fichiers, l'orchestrateur applique le bloc i18n au moment de l'intégration.

### Menus et commandes

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.export` | Exporter vers Word, Markdown, HTML, texte… | Export to Word, Markdown, HTML, text… |
| `menu.file.create` | Créer un PDF (vierge, texte, Office)… | Create a PDF (blank, text, Office)… |

### Dialogue Exporter

| Clé | fr-FR | en-US |
|---|---|---|
| `convert.export.title` | Exporter le document | Export document |
| `convert.export.subtitle` | Reconstitue le texte et une structure simple ; la mise en page d'origine n'est pas conservée. | Rebuilds the text and a simple structure; the original layout is not preserved. |
| `convert.export.format` | Format | Format |
| `convert.export.format.docx` | Word (.docx) | Word (.docx) |
| `convert.export.format.md` | Markdown (.md) | Markdown (.md) |
| `convert.export.format.html` | Page web (.html) | Web page (.html) |
| `convert.export.format.txt` | Texte brut (.txt) | Plain text (.txt) |
| `convert.export.format.png` | Images PNG (une par page) | PNG images (one per page) |
| `convert.export.fidelity.docx` | Fidélité approchée : titres déduits de la taille du texte, paragraphes, listes simples et sauts de page. Ni images, ni tableaux, ni colonnes. | Approximate fidelity: headings guessed from text size, paragraphs, simple lists and page breaks. No images, tables or columns. |
| `convert.export.fidelity.md` | Fidélité approchée : titres déduits de la taille du texte, paragraphes et listes simples. Ni images, ni tableaux. | Approximate fidelity: headings guessed from text size, paragraphs and simple lists. No images or tables. |
| `convert.export.fidelity.html` | Fidélité approchée : titres déduits de la taille du texte, paragraphes et listes simples, une section par page. Ni images, ni tableaux. | Approximate fidelity: headings guessed from text size, paragraphs and simple lists, one section per page. No images or tables. |
| `convert.export.fidelity.txt` | Texte exact tel que lu par le moteur PDF, sans mise en forme. Les pages sont séparées par un saut de page. | Exact text as read by the PDF engine, without formatting. Pages are separated by a form feed. |
| `convert.export.fidelity.png` | Rendu visuel exact ; le texte n'est plus sélectionnable. | Exact visual rendering; the text is no longer selectable. |
| `convert.export.scope` | Pages | Pages |
| `convert.export.scope.all` | Tout le document | Whole document |
| `convert.export.scope.current` | Pages sélectionnées ou page courante | Selected pages or current page |
| `convert.export.dpi` | Résolution (ppp) | Resolution (dpi) |
| `convert.export.drop_repeated` | Ignorer les en-têtes et pieds de page répétés | Skip repeated headers and footers |
| `convert.export.note` | Un PDF numérisé ne contient pas de texte à exporter. Les modifications non enregistrées sont incluses. | A scanned PDF has no text to export. Unsaved changes are included. |
| `convert.export.ok` | Exporter | Export |
| `convert.export.pick_dir` | Dossier de destination des images PNG | Destination folder for PNG images |
| `convert.export.save.docx` | Enregistrer le document Word | Save the Word document |
| `convert.export.save.md` | Enregistrer le fichier Markdown | Save the Markdown file |
| `convert.export.save.html` | Enregistrer la page web | Save the web page |
| `convert.export.save.txt` | Enregistrer le texte brut | Save the plain text |
| `convert.export.filter.docx` | Documents Word | Word documents |
| `convert.export.filter.md` | Fichiers Markdown | Markdown files |
| `convert.export.filter.html` | Pages web | Web pages |
| `convert.export.filter.txt` | Fichiers texte | Text files |

### Dialogue Créer

| Clé | fr-FR | en-US |
|---|---|---|
| `convert.create.title` | Créer un PDF | Create a PDF |
| `convert.create.subtitle` | Page vierge, fichier texte ou Markdown, images ou document Office. | Blank page, text or Markdown file, images or Office document. |
| `convert.create.mode.blank` | Page vierge | Blank page |
| `convert.create.mode.text` | Texte ou Markdown | Text or Markdown |
| `convert.create.mode.images` | Images | Images |
| `convert.create.mode.office` | Document Office | Office document |
| `convert.create.paper` | Format de page | Page size |
| `convert.create.paper.a3` | A3 | A3 |
| `convert.create.paper.a4` | A4 | A4 |
| `convert.create.paper.a5` | A5 | A5 |
| `convert.create.paper.letter` | Lettre (US) | Letter (US) |
| `convert.create.paper.legal` | Légal (US) | Legal (US) |
| `convert.create.orientation.portrait` | Portrait | Portrait |
| `convert.create.orientation.landscape` | Paysage | Landscape |
| `convert.create.pages` | Nombre de pages | Number of pages |
| `convert.create.font_size` | Taille du texte (pt) | Text size (pt) |
| `convert.create.source` | Fichier source | Source file |
| `convert.create.browse` | Choisir… | Browse… |
| `convert.create.no_file` | Aucun fichier choisi | No file chosen |
| `convert.create.note_text` | Titres, listes à puces, blocs de code et paragraphes sont mis en page. Images, tableaux et mise en forme en ligne (gras, italique) ne sont pas rendus. Les caractères hors Windows-1252 sont remplacés par « ? ». | Headings, bullet lists, code blocks and paragraphs are laid out. Images, tables and inline formatting (bold, italic) are not rendered. Characters outside Windows-1252 are replaced by “?”. |
| `convert.create.note_images` | Une page par image, à la taille de l'image (150 ppp). | One page per image, sized to the image (150 dpi). |
| `convert.create.note_office` | Le document est converti localement par un logiciel installé sur cet ordinateur ; aucun fichier n'est envoyé sur Internet. La mise en page dépend de ce logiciel. | The document is converted locally by software installed on this computer; no file is sent over the Internet. The layout depends on that software. |
| `convert.create.converter_found` | Convertisseur détecté : {name} | Converter found: {name} |
| `convert.create.converter_none` | Aucun convertisseur détecté. Installez LibreOffice (gratuit) ou Microsoft Office. | No converter found. Install LibreOffice (free) or Microsoft Office. |
| `convert.converter.office` | Microsoft Office | Microsoft Office |
| `convert.converter.libreoffice` | LibreOffice | LibreOffice |
| `convert.create.ok` | Créer | Create |
| `convert.create.ok_images` | Choisir les images… | Choose images… |
| `convert.create.save` | Enregistrer le PDF créé | Save the created PDF |
| `convert.create.pick_text` | Choisir le fichier texte ou Markdown | Choose the text or Markdown file |
| `convert.create.filter_text` | Texte et Markdown | Text and Markdown |
| `convert.create.pick_office` | Choisir le document à convertir | Choose the document to convert |
| `convert.create.filter_office` | Documents Office et OpenDocument | Office and OpenDocument documents |

### Progression, fins de tâche, erreurs

| Clé | fr-FR | en-US |
|---|---|---|
| `convert.running.export` | Export en cours… | Exporting… |
| `convert.running.export_progress` | Export : page {done} / {total}… | Exporting: page {done} / {total}… |
| `convert.running.office` | Conversion du document… (peut durer une minute) | Converting the document… (may take a minute) |
| `convert.cancel` | Annuler | Cancel |
| `convert.cancelled` | Opération annulée | Operation cancelled |
| `convert.done.export` | Exporté : {name} ({pages} page(s)) | Exported: {name} ({pages} page(s)) |
| `convert.done.export_partial` | Exporté : {name} ({pages} page(s), dont {empty} sans texte) | Exported: {name} ({pages} page(s), {empty} without text) |
| `convert.done.dropped` | {dropped} ligne(s) d'en-tête ou de pied de page ignorée(s) | {dropped} header or footer line(s) skipped |
| `convert.done.rotated` | {rotated} caractère(s) de texte pivoté ignoré(s) | {rotated} rotated text character(s) skipped |
| `convert.done.text_replaced` | PDF créé : {name} ({pages} page(s)) ; {replaced} caractère(s) remplacé(s) par « ? » | PDF created: {name} ({pages} page(s)); {replaced} character(s) replaced by “?” |
| `convert.done.office` | PDF créé avec {converter} : {name} | PDF created with {converter}: {name} |
| `error.no_text` | Ce document ne contient aucun texte à exporter (document numérisé ou composé d'images). | This document has no text to export (scanned or image-only document). |
| `error.converter_missing` | Aucun convertisseur n'a été trouvé. Installez LibreOffice (gratuit) ou Microsoft Office pour convertir ce type de document. | No converter was found. Install LibreOffice (free) or Microsoft Office to convert this kind of document. |
| `error.converter_failed` | La conversion a échoué. Le document est peut-être protégé par un mot de passe, endommagé ou d'un format non pris en charge. Détails dans le journal. | The conversion failed. The document may be password-protected, damaged or in an unsupported format. See the log for details. |
| `error.converter_timeout` | La conversion a dépassé le délai autorisé et a été interrompue. | The conversion exceeded the allowed time and was stopped. |
| `error.unsupported_format` | Ce type de fichier n'est pas pris en charge. | This file type is not supported. |

### Clé existante dont le texte change

| Clé | fr-FR (nouveau) | en-US (nouveau) |
|---|---|---|
| `home.card.convert.desc` | Créer · Exporter (Word, Markdown, HTML) · Optimiser | Create · Export (Word, Markdown, HTML) · Optimize |

Total : 77 clés nouvelles + 1 texte modifié (à recompter avec `scripts\check-i18n.ps1` après l'ajout). Réutilisées sans changement : `dialog.password.cancel` (Annuler), `tools.running.images` (Création du PDF…), `tools.done.images` (PDF créé : …).

## 8. Tests

Règle projet : rendu vérifié par pixels ou relecture du texte ; commandes exécutées avec `-j 2` (poste à 14 Go) ; aucun résultat annoncé avant exécution réelle.

### 8.1 Tests unitaires (sans `pdfium.dll`, dans les modules)

| Module | Assertions |
|---|---|
| `job.rs` | `cancel()` puis `check()` → `Err(Cancelled)` ; `begin(5)` + 3 `tick()` → `progress() == (3, 5)`. |
| `structure.rs` | `build_blocks` sur lignes synthétiques : ratios 2,18 / 1,45 / 1,15 / 1,0 → `Heading(1)` / `Heading(2)` / `Heading(3)` / `Paragraph` ; deux lignes de titre consécutives fusionnées ; trois lignes à interligne 14 pt pour un corps de 11 pt → un seul paragraphe ; écart de 32 pt → deux paragraphes ; marqueurs `•`, `-`, `1.` → `ListItem` (ordonné ou non) ; `soft_hyphen` recolle sans espace, un `-` ordinaire est conservé ; ligne de plus de 200 caractères jamais titre ; gras seul jamais titre. `drop_repeated` : trois pages avec « Confidentiel » en haut et « Page 1/2/3 » en bas → 6 lignes retirées ; une seule page → 0 ; une ligne répétée mais au milieu de la page → conservée. `body_font_size` pondéré par les caractères (un titre de 24 pt ne devient pas le corps). |
| `export.rs` | `flate2::Crc` sur `b"123456789"` = `0xCBF43926` (valeur de contrôle CRC-32 standard) ; `zip_archive` : signatures `PK\x03\x04` en tête, fin de répertoire avec le bon nombre d'entrées et des décalages cohérents, deux appels → octets identiques (déterminisme), entrée incompressible → méthode 0 ; `xml_escape("a&b<c>\u{1}\u{FFFE}")` = `a&amp;b&lt;c&gt;` ; `md_escape("a * b_c [d]")` ; `render_markdown`, `render_html`, `render_docx` sur un `StructuredText` construit à la main. |
| `compose.rs` | `PaperSize::points` (A4 portrait 595.28 × 841.89, paysage permuté, Lettre 612 × 792) ; `parse_markdown` (titres, puces à 3 niveaux, bloc de code non fermé jusqu'à la fin, `---`) ; `strip_inline` (`**a**` → `a`, `` `x` `` → `x`, `snake_case` intact, `[t](u)` → `t (u)`) ; assainissement WinAnsi (`☺` → `?` compté ; `œ`, `€`, `«` conservés) ; décodage `[0x93, b'A', 0x94, 0xE9]` Windows-1252 → `“A”é` ; `layout` avec fausse mesure `|_, s, t| t.chars().count() as f32 * s * 0.5` : aucune ligne plus large que la largeur utile, nombre de pages attendu, titre jamais seul en bas de page, mot géant coupé, entrée vide → `Other("empty input")`. |
| `office.rs` | `file_url("C:\\a b\\é")` = `file:///C:/a%20b/%C3%A9` ; `office_app` pour chaque extension ; `choose_converter` (table de vérité : `docx` avec Office seul, avec LibreOffice seul, avec les deux ; `odt` avec les deux → LibreOffice ; `prefer`) ; arguments LibreOffice construits = vecteur attendu (aucun lancement) ; détection **par injection** (`struct DetectEnv { disable, soffice, path, program_files }` construite depuis l'environnement réel par `detect_converters()`, fournie à la main par les tests : arbre temporaire contenant des `WINWORD.EXE` vides) → pas de modification de variables d'environnement dans les tests (elles sont globales au processus). Supervision, `#[cfg(windows)]` : `cmd /c exit 3` → `ConverterFailed` ; `cmd /c ping -n 30 127.0.0.1 > nul` avec délai 300 ms → `ConverterTimeout` en moins de 3 s ; drapeau d'annulation levé après 200 ms par un autre thread → `Cancelled` en moins de 3 s ; le dossier temporaire a disparu. |

### 8.2 Tests d'intégration (`crates/sunipdf-core/tests/engine.rs`, helper `engine()` sérialisé, `fixture()`)

Nouvelle fixture `structured.pdf` (8.6). Dossiers temporaires `std::env::temp_dir().join(format!("sunipdf-export-{}", std::process::id()))` comme engine.rs:620-623.

1. `text_lines_report_sizes_positions_and_styles` — `doc.text_lines(0)` : la ligne « Rapport annuel » a `size` à ±0,5 de 24, `bold`, `baseline` à ±1,5 de 720, `left` à ±1 de 72 ; « Résumé exécutif » ≈ 16 pt gras ; première ligne du paragraphe ≈ 11 pt, non grasse, baseline ≈ 650 ; « Confidentiel » ≈ 9 pt vers 770. Puis filigrane : `stamp_pages(Some(&[0]), &Stamp { text: "CONFIDENTIEL".into(), ..Default::default() })` (stamp.rs:96) → `text_lines(0)` ne contient plus « CONFIDENTIEL », le compteur de caractères pivotés est ≥ 10, tandis que `page_text(0)` le contient encore (preuve que l'exclusion est volontaire).
2. `structured_text_builds_headings_paragraphs_lists_and_drops_headers` — page 1 : `[Heading(1) "Rapport annuel", Heading(2) "Résumé exécutif", Paragraph (3 lignes jointes en une, accents intacts), Paragraph "Second paragraphe distinct.", ListItem{ordered:false} "premier point", ListItem "second point"]` ; `stats.dropped_repeated == 6` ; avec `drop_repeated: false`, « Confidentiel » est présent ; `pages: Some(vec![1])` → une seule page, `Heading(2) "Conclusion"`.
3. `export_text_markdown_html_keep_accents` — `.txt` : contient `Les élèves ont étudié l'été à Genève`, 2 caractères U+000C ; `.md` : commence par `# Rapport annuel`, contient `## Résumé exécutif`, `- premier point`, une ligne `---` entre pages, ne contient ni « Confidentiel » ni « Page 2 » ; `.html` : `<!doctype html>`, `<meta charset="utf-8">`, `<h1>Rapport annuel</h1>`, `<h2>Résumé exécutif</h2>`, `R&amp;D`, `<ul>`, trois `<section class="page"`. Aucun `*.tmp` résiduel dans le dossier (écriture atomique).
4. `export_docx_is_a_valid_package` — lecture du ZIP avec un petit lecteur local au fichier de test (≈ 30 lignes : recherche de `0x06054b50`, parcours du répertoire central `0x02014b50`, décompression par `flate2::read::DeflateDecoder` — F2:lib.rs:171 —, contrôle du CRC avec `flate2::Crc`) : cinq parties dans l'ordre attendu (`[Content_Types].xml` en premier), CRC et tailles exacts, chaque partie XML équilibrée (petit vérificateur de balises), `document.xml` contient `w:pStyle w:val="Heading1"`, « Rapport annuel », `R&amp;D`, exactement 2 `<w:pageBreakBefore/>`, `<w:pgSz w:w="12240" w:h="15840"/>` (612 × 792 pt × 20), `styles.xml` définit `Heading1` à `Heading3`.
5. `export_errors_and_cancellation` — PDF d'images (`images_to_pdf`, comme le test engine.rs:550) : `export_document(Markdown/Text/Docx)` → `Err(NoText)`, **fichier de sortie absent**, aucun `.tmp` ; page 9 hors limites → `PageOutOfRange(9)` ; `JobControl` annulé d'avance → `Err(Cancelled)` sans fichier ; succès → `progress() == (3, 3)` ; sortie dans un dossier inexistant créé par `write_atomic` (pages.rs:63-66).
6. `create_blank_pdf_sizes_orientation_and_pixels` — pour les 5 formats × 2 orientations : réouverture, nombre de pages, `page_size` à ±0,05 de `PaperSize::points`, rendu de la page 0 à l'échelle 0,25 dont **tous** les pixels valent `(255,255,255,255)` ; erreurs : 0 page et 501 pages ; pas de `.tmp`.
7. `text_to_pdf_wraps_paginates_keeps_accents_and_margins` — fichier UTF-8 de 200 lignes `Ligne {n} : élève, été, œuvre € « test »` plus une ligne de 600 `x` sans espace ; A4, marge 56,7 pt, 11 pt : plusieurs pages ; `page_text(0)` contient `Ligne 1 : élève, été, œuvre` ; la dernière page contient `Ligne 200` ; pour **chaque** objet texte de chaque page (`page_objects`, edit.rs:162) : `bounds.left >= marge - 1`, `bounds.right <= largeur - marge + 1`, `bounds.bottom >= marge - 4`, `bounds.top <= hauteur - marge + 1` ; rendu de la page 0 à l'échelle 1 : **aucun pixel non blanc** dans la bande extérieure aux marges et plus de 100 pixels sombres à l'intérieur. Ce test est la **porte de validation du risque R1** (mesure de largeur) : une mesure fausse fait déborder la ligne de 600 caractères. Variantes : octets Windows-1252 (`0xE9`, `0x80`) → `é` et `€` relus ; `☺` → `replaced_chars == 1` et `?` relu ; titre du document = nom du fichier ; UTF-16 → `UnsupportedFormat`.
8. `markdown_to_pdf_styles_headings_lists_and_code` — source avec `# Titre`, `## Sous-titre`, paragraphe avec `**gras**`, `` `code` `` et un lien, deux puces, bloc ``` : le titre est un objet texte de `font_size` ≈ 22 (±0,6) situé au-dessus du paragraphe, le corps ≈ 11, l'objet de code a une police de famille Courier (`ObjectInfo.font_name`, edit.rs:24) ; le texte relu contient `• a` et `gras` sans `**`, `code` sans accents graves, `texte (https://exemple.fr)`.
9. `#[ignore] office_word_roundtrip` (manuel, Word requis) — export DOCX de `structured.pdf` → `office::convert_to_pdf` → réouverture : `page_text` contient « Rapport annuel » et « Les élèves ». Lancé à la demande : `cargo test -p sunipdf-core --test engine -j 2 -- --ignored office_word_roundtrip`.

### 8.3 Vérifications manuelles indépendantes (Word est installé sur ce poste)

1. Structure ZIP lue par .NET : `Add-Type -AssemblyName System.IO.Compression.FileSystem; [IO.Compression.ZipFile]::OpenRead('C:\…\structured.docx').Entries | ForEach-Object FullName` → 5 entrées.
2. Ouverture dans Word en COM, lecture seule : `$w = New-Object -ComObject Word.Application; $w.Visible=$false; $d = $w.Documents.Open('C:\…\structured.docx', $false, $true); $d.Paragraphs.Count; $d.Paragraphs(1).Range.Text; $d.Paragraphs(1).Style.NameLocal; $d.ComputeStatistics(2); $d.Close(0); $w.Quit()` → premier paragraphe « Rapport annuel » au style « Titre 1 » (ou « Heading 1 »), 3 pages. C'est la validation **indépendante** des types de contenu et des relations OPC (voir 3.4).
3. Office → PDF : un `.docx`, un `.xlsx`, un `.pptx` convertis par le dialogue ; vérifier qu'aucun `powershell.exe` ni `WINWORD.EXE` ne subsiste ensuite.
4. Document protégé par mot de passe : l'erreur doit être rapide (pas d'attente du délai de 180 s) ; sinon retirer le mot de passe factice de `office_convert.ps1` et documenter la limite.
5. Macros : `.doc` contenant une macro `AutoOpen` qui écrit un fichier marqueur dans `%TEMP%` : après conversion, **le marqueur ne doit pas exister** (effet d'`AutomationSecurity = 3`). Tant que ce test manuel n'est pas fait, ne pas écrire « macros désactivées » dans le guide.
6. Annulation : conversion d'un gros `.docx`, clic sur Annuler dans la barre d'état → retour en moins de 2 s, processus PowerShell absent, état de Word relevé (instance résiduelle ou non) et consigné dans KNOWN_ISSUES.
7. Message « aucun convertisseur » : lancer avec `SUNIPDF_OFFICE_DISABLE=1` (variable de développement) : le mode Office affiche `convert.create.converter_none`, bouton grisé ; `sunipdf-cli office2pdf x.docx --out y.pdf` → JSON `converter_unavailable`, code de sortie 6.
8. LibreOffice (à faire sur un poste où il est installé, **pas sur celui-ci**) : `office2pdf` d'un `.odt` ; conversion lancée pendant qu'une fenêtre LibreOffice est ouverte (vérifie l'intérêt du profil jetable) ; durée du premier lancement.

### 8.4 Mesures de performance à relever (sans seuil promis)

Temps de `text_lines` sur une page d'environ 3 000 caractères (appels FFI par caractère : poids, nom de police, italique, angle, origine, bornes) ; `structured_text` sur un document de 100 pages ; export DOCX de 100 pages (taille obtenue) ; création d'un PDF de 200 pages de texte ; matériel noté (Windows 11 Pro 26300, 14 Go). Si une page dépasse ~100 ms, introduire le cache par (taille, poids) décrit en 3.3. Aucun chiffre n'est reporté dans la documentation avant d'avoir été mesuré.

### 8.5 Protocole de mesure de fidélité (pour annoncer un chiffre un jour)

Corpus : 5 à 10 PDF à licence libre, textuels (par exemple extraits du corpus pdf.js, SUNI-016 de NEXT_TASKS.md), plus la fixture. Pour DOCX : export SuniPDF → réexport PDF par Word (COM, `ExportAsFixedFormat`) → `page_text` normalisé (espaces réduits, sauts de ligne supprimés) comparé au `page_text` de l'original : taux d'erreur caractère (distance d'édition / longueur), nombre de titres retrouvés (comptage manuel sur 3 documents). Pour Markdown/HTML : même comparaison après suppression du balisage. Résultats consignés dans un tableau (fichier, commande, valeur) ; tant qu'il n'existe pas, l'interface n'affiche que les phrases qualitatives de la section 7.

### 8.6 Fixture `structured.pdf` (à produire par `scripts/make_fixtures.py`)

Ajouter une fonction `structured(path)` au script (motifs de `two_pages`, make_fixtures.py:37, et de `build`, make_fixtures.py:17) et son appel dans le bloc `__main__` ; régénération : `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures` (les deux fixtures existantes sont reproduites à l'identique : le script est déterministe). Contenu : 3 pages 612 × 792 ; polices `F1` = Helvetica et `F2` = Helvetica-Bold, toutes deux `/Encoding /WinAnsiEncoding` ; chaînes encodées en cp1252 avec échappement de `\`, `(` et `)` (utile : `\351` = é, `\350` = è, `\340` = à, `\352` = ê, `\225` = puce).

| Page | Objets texte (police, taille, position) |
|---|---|
| 1 | `Confidentiel` (F1, 9, 72 770) ; `Rapport annuel` (F2, 24, 72 720) ; `Résumé exécutif` (F2, 16, 72 680) ; paragraphe de 3 lignes (F1, 11, x 72, y 650 / 636 / 622) : `Les élèves ont étudié l'été à Genève avec soin et` / `méthode. Le budget R&D a augmenté de 12 %, soit 3 millions` / `d'euros pour l'année.` ; `Second paragraphe distinct.` (F1, 11, 72 590) ; `• premier point` (F1, 11, 72 560) ; `• second point` (F1, 11, 72 546) ; `Page 1` (F1, 9, 300 30) |
| 2 | `Confidentiel` ; `Conclusion` (F2, 16, 72 720) ; `La conclusion tient en deux lignes simples` / `et reste dans le même paragraphe.` (F1, 11, 72 690 / 676) ; `Page 2` |
| 3 | `Confidentiel` ; `Dernier paragraphe.` (F1, 11, 72 720) ; `Page 3` |

Autres entrées de test créées à la volée dans le dossier temporaire (pas de binaire versionné) : PNG de 40 × 20 pixels pour le PDF « numérisé » (comme engine.rs:550), fichiers `.txt` / `.md` UTF-8 et Windows-1252.

### 8.7 Commandes de validation (à lancer à l'implémentation, résultats à rapporter tels quels)

```powershell
. .\scripts\dev-env.ps1
python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test -p sunipdf-core -j 2
cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2
cargo test -p sunipdf-cli -j 2
cargo test --workspace -j 2
.\scripts\check-i18n.ps1
```

Si `qpdf` est disponible (absent du PATH de ce poste ; la vérification qpdf reste celle de la CI), lancer `qpdf --check` sur les PDF créés par les tests et la CLI (page vierge, texte, Markdown). Rapporter les sorties réelles, y compris les échecs.

### 8.8 Scénario de capture manuelle (`scripts\dev-screenshot.ps1`)

Paramètres disponibles (scripts/dev-screenshot.ps1:3-14) : `-Out`, `-Pdf` (plusieurs chemins séparés par `;`), `-Delay` (s), `-Keys` (avant les glissements), `-Commands 'id1,id2'` (exécutées après la première ouverture, via `SUNIPDF_STARTUP_COMMANDS`, app.rs:157-162 et 359-361), `-Drag 'x1,y1,x2,y2;…'` (relatif au coin haut-gauche de la fenêtre, placée en (0,0) ; **un glissement dont le départ et l'arrivée sont identiques sert de clic**), `-KeysAfter`.

1. Dialogue Exporter, état par défaut : `.\scripts\dev-screenshot.ps1 -Out export-dialog.png -Pdf crates\sunipdf-core\tests\fixtures\structured.pdf -Commands 'convert.export'`. À vérifier : titre, cinq choix de format avec « Word (.docx) » sélectionné, phrase de fidélité sous les choix, portée, case « Ignorer les en-têtes… », boutons « Exporter » (bleu) / « Annuler », cadre homogène avec les autres dialogues (ombre, pied grisé).
2. Variante PNG : relever sur la capture 1 les coordonnées de la ligne « Images PNG », puis `-Drag 'X,Y,X,Y'` : portée grisée, champ de résolution visible, case des en-têtes masquée.
3. Dialogue Créer : `-Out create-blank.png -Pdf …\two-pages.pdf -Commands 'convert.create'` (mode Page vierge : format, orientation, nombre de pages). Puis clic sur « Document Office » (coordonnées relevées) : ligne « Convertisseur détecté : Microsoft Office » (Word est installé sur ce poste) ; refaire avec `$env:SUNIPDF_OFFICE_DISABLE='1'` positionnée dans la session PowerShell avant l'appel (le script lance l'application par `Start-Process`, qui hérite de l'environnement) : texte d'absence de convertisseur et bouton « Créer » grisé.
4. Panneau d'outils : `-Out tools.png -Pdf …\two-pages.pdf` (panneau visible par défaut, `show_tools = true`, settings.rs:24) : lignes « Exporter vers Word, Markdown, HTML, texte… » et « Créer un PDF (vierge, texte, Office)… » avec pastille verte de la catégorie Convertir.
5. Anglais et thème sombre : changer la langue dans Paramètres (ou `settings.json`) puis refaire 1 et 3 ; vérifier qu'aucune clé brute (`convert.…`) n'apparaît et que les textes longs passent à la ligne sans déborder du cadre de 560 pt.
6. Barre d'état en cours de tâche : non capturable de façon déterministe (tâche trop brève) ; vérifier à la main avec une conversion Office longue : spinner, texte `convert.running.office`, lien « Annuler » ; refaire **sans document ouvert** (accueil) pour couvrir la branche `(_, None, None)` de `status_bar`.

## 9. Nouvelles dépendances

| Crate | Version | Licence | Raison | Remarque |
|---|---|---|---|---|
| `flate2` | `1` (verrouillé 1.1.10) | MIT OR Apache-2.0 | DEFLATE brut + `flate2::Crc` pour l'écrivain ZIP du DOCX | Déjà dans `Cargo.lock` via lopdf, png, tiff ; devient dépendance directe de `sunipdf-core` (aucun téléchargement nouveau) |
| `encoding_rs` | `0.8` (verrouillé 0.8.42) | (Apache-2.0 OR MIT) AND BSD-3-Clause | Décoder les `.txt` en Windows-1252 | Déjà dans `Cargo.lock` via lopdf |

Aucun crate `zip`, `docx-rs`, `rust_xlsxwriter`, `pulldown-cmark`, `comrak` : écartés (non présents, ou fonction réalisable en quelques dizaines de lignes). Aucun appel réseau introduit (revue : `flate2` et `encoding_rs` sont de purs calculs ; le seul sous-processus est le convertisseur local choisi par l'utilisateur). À ajouter à THIRD_PARTY_NOTICES.md : mention que Microsoft Office et LibreOffice sont des logiciels **tiers optionnels, non redistribués**, appelés localement.

## 10. Risques, pièges connus, étapes d'implémentation, estimation

### 10.1 Risques et pièges (avec parade)

| # | Risque / piège | Parade ou vérification |
|---|---|---|
| R1 | La largeur d'un objet texte **non rattaché à une page** (`PdfPageTextObject::new` puis `width()`) n'est pas garantie par la documentation de pdfium-render ; toute la justesse des retours à la ligne en dépend. | Test 7 (ligne de 600 caractères, bornes de tous les objets, pixels des marges) à passer **avant** de poursuivre ; repli prévu en 3.5 (document jetable + `create_text_object` + `bounds()`). |
| R2 | PDFium peut renvoyer un poids de police peu fiable pour les polices intégrées (char.rs:213-216) ; `scaled_font_size` peut valoir 0 si la matrice est absente (`unwrap_or(0.0)`, PR:pdf/transform.rs get_vertical_scale). | Second avis par le nom de police ; repli sur `unscaled_font_size` puis sur la hauteur des bornes ; test 1 sur la fixture. |
| R3 | La qualité des titres/paragraphes est **heuristique** et n'a été mesurée sur aucun corpus. | Phrases de fidélité qualitatives dans l'UI ; protocole 8.5 ; seuils (1,5 / 1,25 / 1,12 et 1,6 d'interligne) regroupés en constantes nommées, faciles à régler. |
| R4 | `soffice` : syntaxe documentée, mais LibreOffice **n'est pas installé ici** ; le comportement avec instance ouverte, premier lancement lent, codes de sortie sont **non vérifiés**. | Étape 6 limitée à des tests de supervision avec `cmd.exe` ; test manuel 8.3.8 sur un poste équipé avant d'annoncer la prise en charge de LibreOffice. |
| R5 | Automatisation d'Office : instance restante après arrêt du client, invite de mot de passe invisible, mode « Protégé » ou licence expirée, Office lancé à un niveau d'élévation différent, politiques PowerShell d'entreprise (mode de langage contraint, AppLocker). | Délai maximal + annulation, mot de passe factice, `DisplayAlerts` coupé, messages `error.converter_failed` + détail au journal ; tests manuels 8.3.3-8.3.6 ; limites inscrites dans KNOWN_ISSUES. |
| R6 | Sécurité : Word ouvre par défaut avec macros **activées** en mode programmatique (`msoAutomationSecurityLow`). | `AutomationSecurity = 3` dans le script ; ne rien affirmer avant le test 8.3.5 ; le document est ouvert en lecture seule, sans mise à jour des liens (Excel). |
| R7 | Un objet texte vide fait planter `FPDFText_LoadPage` (PR:object/text.rs:191-195). | Jamais d'objet texte vide en composition ; ligne vide = simple avance verticale. |
| R8 | Caractères Windows-1252 hors Latin-1 (`€ œ « » …`) : encodage par PDFium des polices standard non vérifié. | Test 7 relit le texte ; si un caractère échoue, retirer ce caractère de l'ensemble conservé (il devient `?` et est compté). |
| R9 | XML invalide → Word déclare le DOCX corrompu (caractères de contrôle, U+FFFE que PDFium peut renvoyer). | `xml_escape` filtre ; test unitaire dédié ; validation par Word (8.3.2). |
| R10 | Concurrence entre tranches : `error.rs`, `command.rs`, `lib.rs`, `worker.rs`, `app.rs`, `shell.rs`, `tools.rs`, les deux JSON i18n, `tests/engine.rs`, CLI `main.rs`, `make_fixtures.py` sont des fichiers partagés. Les variantes d'`EngineError` en particulier peuvent être ajoutées par d'autres tranches (OCR, formulaires). | Étape 0 intégrée en premier et seule ; modifications additives uniquement ; l'orchestrateur applique le bloc i18n et résout les conflits ; fusion séquentielle des tranches. |
| R11 | Le thread moteur est occupé pendant un export long : rendu et miniatures attendent. | Comportement identique à `Optimize` ; annulation et progression disponibles ; hors périmètre : export par morceaux. |
| R12 | Mémoire : `StructuredText` d'un document de 1 000 pages ; archive DOCX construite en mémoire. | Quelques Mo de texte ; garde-fous ZIP32 (4 Gio) et `MAX_TEXT_PAGES` ; documenté. |
| R13 | Texte pivoté ou vertical, pages avec `/Rotate`, colonnes entrelacées, droite-à-gauche, CJK : ordre ou regroupement faux. | Détection de rotation du caractère (caractères ignorés et comptés) ; limites écrites dans le guide ; aucun correctif heuristique en v1. |
| R14 | `std::path::absolute` et chemins très longs (> 260 caractères) avec Office. | Message d'échec générique ; signalé dans KNOWN_ISSUES si observé. |
| R15 | Nom de la variable de développement `SUNIPDF_OFFICE_DISABLE` / `SUNIPDF_SOFFICE` : surface de configuration non persistée. | Documentées comme variables de développement/portables dans le guide ; pas de réglage UI en v1. |

### 10.2 Ordre d'implémentation (chaque étape compile et se teste seule)

| Étape | Contenu | Fichiers | Validation | Taille |
|---|---|---|---|---|
| 0 | Socle : dépendances Cargo, 6 variantes d'erreur (+ `error_key`, test des clés, codes de sortie CLI), `job.rs`, exports `lib.rs` | Cargo.toml, error.rs, job.rs, lib.rs, app.rs (arms), CLI main.rs (codes) | `cargo build --workspace -j 2`, `cargo test -p sunipdf-core -p sunipdf-desktop -j 2` (les clés d'erreur ne passent qu'avec le catalogue : ajouter à ce stade les 5 clés `error.*` et `convert.cancelled` aux deux JSON) | S |
| 1 | Texte structuré | structure.rs, make_fixtures.py + `structured.pdf`, tests 1-2 et unitaires | tests ; mesures 8.4 ; **décision cache FFI** | M |
| 2 | Writers texte/Markdown/HTML + `export_document` + CLI `export` | export.rs, CLI | tests 3 et 5 ; commande CLI exécutée sur la fixture, sortie relue | M |
| 3 | DOCX (ZIP + OPC) | export.rs | test 4 ; **8.3.1-8.3.2 avec Word** | M |
| 4 | Page vierge + texte brut → PDF + CLI `create-blank`, `text2pdf` | compose.rs | tests 6-7 (**porte R1**) ; rendu relu | M |
| 5 | Markdown (parseur, `strip_inline`, titres/puces/code) | compose.rs | test 8 | S |
| 6 | Office → PDF (détection par injection, script, supervision) + CLI `office2pdf`, `converters` | office.rs, office_convert.ps1, CLI | unitaires de supervision ; test 9 ignoré ; **8.3.3-8.3.7** | M |
| 7 | Commandes, `BatchOp`, dialogue Exporter, lignes d'outils, icônes, barre d'état (progression + Annuler) | command.rs, worker.rs, export.rs (UI), tools.rs, shell.rs, app.rs, main.rs, i18n (clés export) | `cargo test -p sunipdf-i18n -p sunipdf-desktop`, captures 1, 2, 4 | M |
| 8 | Dialogue Créer + tâche Office annulable | create.rs, app.rs, shell.rs (`status_bar`), i18n (clés création) | captures 3, 5, 6 ; essais manuels 8.3.3-8.3.7 depuis l'UI | M |
| 9 | Documentation, ADR, état du projet | section 11 | `cargo test --workspace -j 2`, clippy, fmt, `check-i18n.ps1` | S |

Parallélisation possible après l'étape 0 : {1→2→3}, {4→5} et {6} touchent des fichiers moteur distincts ; les étapes 7-8 viennent ensuite car elles dépendent des trois.

### 10.3 Estimation

**XL** au total (≈ 1 400 lignes de moteur, ≈ 700 lignes d'interface, ≈ 700 lignes de tests, 77 clés i18n). Découpage conseillé en deux livraisons : **A = étapes 0 à 5 et 7** (export + création locale, effort L) ; **B = étapes 6 et 8** (Office → PDF, effort M, dont une partie du risque est liée à du logiciel tiers).

### 10.4 Questions ouvertes (pour l'utilisateur / l'orchestrateur)

1. Ordre de préférence des convertisseurs pour les formats Microsoft : Office d'abord (fidélité supérieure) ou LibreOffice d'abord (aucune session COM, pas de risque d'instance Office résiduelle) ? Le plan retient Office d'abord pour `doc docx xls xlsx ppt pptx`, LibreOffice d'abord pour les autres.
2. Faut-il une action « Ouvrir le fichier exporté » / « Afficher dans le dossier » après un export ? Absente du plan (aucune dépendance supplémentaire ; `rfd` ne le fournit pas).
3. Glisser-déposer d'un `.docx` / `.txt` / `.md` sur la fenêtre pour ouvrir directement le dialogue Créer ? Aujourd'hui tout fichier non PDF déposé affiche `error.not_pdf_dropped` (app.rs:588) ; non inclus.
4. Persister les derniers choix (format d'export, papier, taille) dans `settings.json` ? Non inclus (les formulaires de tampon ne le font pas non plus).

## 11. Documentation à mettre à jour

| Fichier | Mise à jour |
|---|---|
| `docs/PROJECT_STATUS.md` | Entrée « SUNI-0xx Exporter / Créer » **uniquement avec les commandes réellement exécutées et leurs résultats** ; ne pas cocher Office → PDF sans l'essai 8.3.3 ; ne rien annoncer pour LibreOffice tant que 8.3.8 n'est pas fait. |
| `docs/NEXT_TASKS.md` | Suites : PDF → XLSX (grilles + corpus), images dans les exports, dépôt de fichiers non PDF, export PNG de pages choisies, dé-césure, mémorisation des choix, mesure de fidélité (8.5), conversion par lots. |
| `docs/FEATURE_MATRIX.md` (section D) | « PDF → texte » : Fait (UI + CLI) ; « PDF → Word » : Partiel livré (fidélité approchée, texte et structure simple) ; ajouter Markdown et HTML ; « PDF → Excel / PowerPoint » : non livré ; « Office → PDF » : Fait si convertisseur installé (Office/LibreOffice) ; texte/Markdown → PDF et page vierge : Fait. |
| `docs/ROADMAP.md` (phase 4) | Déplacer « PDF → texte/Word, Office → PDF » vers « Fait » avec les réserves ; laisser « OCR, PDF → Excel ». |
| `docs/KNOWN_ISSUES.md` | Limites de 2.2, risques R4-R6 et R13, instance Office résiduelle, annulation CLI sans nettoyage, texte → PDF latin seulement. |
| `docs/DECISIONS.md` | D021 : export/composition/Office dans `sunipdf-core` (pas de crate `sunipdf-convert` pour l'instant) ; D022 : DOCX par écrivain ZIP maison + `flate2` (pas de crate `zip`) ; D023 : Office → PDF par convertisseur local détecté (COM/PowerShell, `soffice`), jamais embarqué ni en ligne, macros désactivées par `AutomationSecurity` (sous réserve du test 8.3.5) ; D024 : PDF → XLSX différé (pas de détection de tableaux mesurable). Numéros provisoires : à recaler selon les autres tranches. |
| `docs/ARCHITECTURE.md` | Liste des modules de `sunipdf-core` (job, structure, export, compose, office) ; ligne `sunipdf-convert` du tableau des crates planifiés : « fusionné dans core pour l'instant » ; flux de la tâche Office (thread dédié, hors thread moteur). |
| `docs/USER_GUIDE.fr.md` / `.en.md` | Sections Exporter et Créer : fidélité par format, limites (images, tableaux, colonnes, latin), prérequis Office/LibreOffice, aucune donnée envoyée sur Internet ; commandes CLI et codes de sortie 6 et 7 ; variables `SUNIPDF_SOFFICE` et `SUNIPDF_OFFICE_DISABLE`. |
| `docs/I18N.md` | Préfixe `convert.*` désormais utilisé (retirer des « réservés »). |
| `docs/THIRD_PARTY_NOTICES.md` | `flate2` et `encoding_rs` (déjà couverts par « Autres crates Rust » via `cargo about`, à lister explicitement) ; Microsoft Office et LibreOffice : logiciels tiers facultatifs, non redistribués. |
| `CHANGELOG.md` | Entrée de version avec les nouvelles commandes UI et CLI. |
| `CLAUDE.md` (« Modules existants ») | Ajouter `job`, `structure`, `export`, `compose`, `office` dans la liste des modules de `sunipdf-core` (proposition à valider par l'utilisateur : ce fichier est une instruction de projet). |

