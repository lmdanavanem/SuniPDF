# Plan d'implémentation — tranche « print » (Imprimer, Ctrl+P)

État : plan rédigé le 2026-10-14, **aucune commande cargo exécutée** (consigne de lecture seule). Rien de ce qui suit n'a été compilé :
chaque appel de bibliothèque est cité avec son fichier et sa ligne dans les sources locales ; les points non vérifiés sont marqués ⚠.

Abréviations de chemins utilisées dans tout le document :

| Sigle | Chemin |
|---|---|
| `PR/` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src/` |
| `WS/` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-sys-0.61.2/src/Windows/Win32/` |
| `W/` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-0.62.2/src/Windows/Win32/` (alternative étudiée) |
| `EG/` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.2/src/` |
| (sans sigle) | chemins relatifs à `C:/dev_suniworks/SuniPDF/` |

---

## 0. Constats vérifiés sur ce poste (lecture seule)

| Fait | Source / commande | Conséquence |
|---|---|---|
| Imprimantes installées : `OneNote (Desktop)`, **`Microsoft Print to PDF`** (pilote « Microsoft Print To PDF », port `PORTPROMPT:`, **imprimante par défaut**, A4, 600×600 ppp, assemblage activé), `Adobe PDF` (1200×1200 ppp) | `Get-Printer`, `Get-CimInstance Win32_Printer`, `Get-PrintConfiguration -PrinterName 'Microsoft Print to PDF'` | Le test automatique visé existe sur ce poste. **Ne jamais** utiliser `OneNote (Desktop)` ni `Adobe PDF` dans un test (ils ouvrent une application). |
| Spouleur : `Running`, démarrage automatique. Windows 11 Pro, culture `en-GB` | `Get-Service Spooler`, `Win32_OperatingSystem` | Papier par défaut du PDF imprimé = A4 ici, mais Letter sur un poste `en-US` : les tests doivent accepter les deux. |
| `pdfium.dll` x64 (build 7881, `args.gn` : `pdf_enable_v8=false`) **exporte** `FPDF_RenderPage` et `FPDF_SetPrintMode` (460 exports lus dans la table PE) | analyse du PE de `third_party/pdfium/win-x64/bin/pdfium.dll` (script Python en lecture seule). La DLL ARM64 est présente, **table d'exports non lue** ⚠ | L'impression vectorielle existe côté DLL, mais voir §2 limite L1 : inaccessible via l'API publique de `pdfium-render`. |
| Association `.pdf` de cet utilisateur : `UserChoice = MSEdgePDF`, dont les verbes sont **`open` et `runas` seulement** ; `Acrobat.Document.DC` (qui a `Print`/`Printto`) n'est pas le choix utilisateur ; `PdfCraft.Document` est aussi enregistré | registre `HKCR\.pdf`, `HKCU\...\FileExts\.pdf\UserChoice` | Le repli `ShellExecute("print")` échoue ici (voir §2). |
| Docs Microsoft consultées **sur le web** (pas de source locale) : `PRINTDLGEXW`, `PrintDlgEx` (page archivée), `DOCINFOW`, `DEVNAMES`, `StretchDIBits`, `StartDocW` (learn.microsoft.com). Un échange Microsoft Q&A « WinUI3 : imprimer vers PDF sans interface » montre `CreateDC("WINSPOOL","Microsoft Print to PDF",null,0)` + `DOCINFO.lpszOutput = "E:\\temp\\testprint.pdf"` + `StretchDIBits` | `WebFetch` | `lpszOutput` pour `Microsoft Print to PDF` est attesté par un exemple publié mais **jamais exécuté ici** ⚠ : c'est la première chose à valider (étape 3). |
| `windows` 0.62.2 n'est utilisé que par `accesskit_windows` (features déjà compilées : `Win32_Graphics_Gdi`, `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_System_Com`…) ; `windows-sys` 0.61.2 est utilisé par 13 crates (`anstyle-query`, `anstyle-wincon`, `async-io`, `async-signal`, `eframe`, `errno`, `polling`, `rfd`, `rustix`, `tempfile`, `uds_windows`, `winapi-util`, `zbus`) | `Cargo.lock`, `target/debug/.fingerprint/windows-*/lib-windows.json` | Voir §9. |
| Autres plans en cours dans `docs/plans/` (`tool-hub`, `continuous-view`, `compress-lossy`, `sign-image`, `bookmarks-links`) touchent aussi `command.rs`, `shell.rs`, `app.rs` | lecture de `tool-hub.md` §0, §5.9, §10 | Risque de conflits de fusion : voir §5.1 (deux cas) et §10 (prérequis). |

---

## 1. Objectif et comportement visible

**Ce que voit l'utilisateur**

1. `Ctrl+P`, **Fichier › Imprimer…**, la palette `Ctrl+K`, et une icône d'imprimante dans la barre d'outils (à côté d'Enregistrer, ou dans l'accès rapide si le hub d'outils a atterri).
2. Boîte « **Imprimer** » (cadre `dialog::show`) : mise à l'échelle (Réduire les pages trop grandes — défaut / Adapter à la zone imprimable / Taille réelle), case « Pivoter les pages pour suivre l'orientation du papier » (cochée), qualité (Brouillon 150 ppp / **Standard 300 ppp** / Haute 600 ppp), bouton principal « Suivant… ». Les trois réglages sont mémorisés dans `settings.json`.
3. **Boîte Windows native** (`PrintDlgExW`, feuille de propriétés « Imprimer ») : choix de l'imprimante, *Toutes les pages / Page actuelle / Pages 1-3,5*, nombre de copies et assemblage (si le pilote les gère), bouton *Préférences* du pilote (papier, orientation, couleur, recto-verso).
4. Boîte modale « **Impression en cours** » : barre de progression « Page 3 sur 12 — Microsoft Print to PDF » et bouton « Annuler l'impression » (Échap équivaut au bouton). Fermer la fenêtre de l'application pendant l'impression annule d'abord le travail.
5. Barre d'état : « Impression terminée : 12 page(s) envoyée(s) à … » ou « Impression annulée » ; en cas d'échec, bandeau d'erreur (`error.printer_unavailable`, `error.print_failed`, `error.security`).

**Ce qui est imprimé** : l'état courant en mémoire (annotations, objets ajoutés, rotations, **modifications non enregistrées comprises**), rendu par PDFium avec `FPDF_ANNOT` + `FPDF_PRINTING`. L'impression **ne modifie pas le document** : pas d'`EditOp`, pas d'instantané d'annulation, pas de drapeau « modifié ».

**Correspondance PDFCraft** : les 14 outils de « All tools » de PDFCraft ne contiennent **pas** d'outil d'impression ; `docs/FEATURE_MATRIX.md` ligne 28 le classe « Impression Windows — Déclaré (fidélité limitée) — Planifié (P5) ». La parité visée est donc celle d'un lecteur PDF courant (Edge/Acrobat), pas celle d'un outil PDFCraft. L'outil PDFCraft le plus proche est « Export a PDF », que l'imprimante virtuelle *Microsoft Print to PDF* permet d'approcher (avec la limite L1 ci-dessous).

---

## 2. Faisabilité

**Verdict : oui**, pour le chemin « boîte Windows native + rendu raster par bandes + `StretchDIBits` », avec les limites suivantes à documenter telles quelles.

| # | Limite assumée | Raison |
|---|---|---|
| L1 | **Impression raster uniquement** (pas de texte vectoriel). Imprimer vers *Microsoft Print to PDF* produit un PDF-image (texte non sélectionnable). Le spool GDI pèse environ `largeur_px × hauteur_px × 3` octets par page : A4 ≈ 6,5 Mo à 150 ppp, **26 Mo à 300 ppp**, 105 Mo à 600 ppp. | La voie vectorielle `FPDF_RenderPage(HDC, FPDF_PAGE, …)` (liaison à `PR/bindings.rs:3374-3400`, derrière la feature `pdfium_use_win32` qui tire la crate `windows`) exige un `FPDF_PAGE` ; or `PdfPage::page_handle()` est `pub(crate)` (`PR/pdf/document/page.rs:229` ; les seuls `page_handle()` publics sont sur des types de propriété d'objets, `PR/pdf/document/page/object/ownership.rs:27, 46`, ce qui n'est pas un chemin sain). Pas atteignable sans patcher/forker `pdfium-render` ou écrire notre propre FFI : hors périmètre. Piste en §10. |
| L2 | Pas de mémorisation de l'imprimante ni des préférences du pilote entre deux impressions (la feuille s'ouvre sur l'imprimante par défaut). | Demanderait de reconstruire `hDevMode`/`hDevNames` : reporté. |
| L3 | Pas de « Sélection » (désactivée par `PD_NOSELECTION`), ni plusieurs pages par feuille, brochure, poster, ni choix du bac selon la taille de la page PDF. Recto-verso, couleur, bac : réglages du pilote. | Hors brief. |
| L4 | Le droit d'impression `/P` (bit 3) est respecté de façon **conservatrice** : refusé (`SecurityRestricted`) même si le document a été ouvert avec le mot de passe propriétaire. | `Document` ne mémorise pas quel mot de passe a ouvert le fichier. |
| L5 | Seules des imprimantes **virtuelles** sont testables ici ; aucun essai sur imprimante physique. Sur `windows-latest` (CI) le test saute si l'imprimante est absente. ARM64 : non compilé (voir `docs/KNOWN_ISSUES.md` n°12). | Matériel disponible. |
| L6 | L'annulation agit **entre deux bandes** (une bande d'une page à 600 ppp peut durer ~1 s). | Une bande = un appel PDFium non interruptible. |
| L7 | « Orientation par page » = **rotation du contenu de 90° anti-horaire** quand cela améliore l'ajustement (page paysage sur papier portrait et inversement), pas un changement d'orientation du pilote par `ResetDCW`. Avec *Microsoft Print to PDF*, toutes les pages du PDF produit ont donc la même orientation. | `ResetDCW` dépend du pilote ; la rotation est déterministe et testable. `ResetDCW` (`WS/Graphics/Gdi/mod.rs:310`) reste une évolution possible. |

### Repli bon marché : `ShellExecute("print")` — **non retenu**

Idée : écrire `Document::snapshot()` (`crates/sunipdf-core/src/pages.rs:246`) dans un fichier temporaire et appeler `ShellExecuteW(hwnd, "print", chemin, …)` (10 lignes). Inacceptable comme chemin principal :

1. **Il échoue sur ce poste** : le gestionnaire choisi par l'utilisateur (`MSEdgePDF`) n'a pas de verbe `print` (verbes : `open`, `runas`). Retour attendu : erreur d'association (`ERROR_NO_ASSOCIATION` = 1155, `WS/Foundation/mod.rs:3385`). Acrobat (verbes `Print`/`Printto`) n'est pas le choix de l'utilisateur.
2. Il imprime un **fichier**, pas l'état en mémoire : il faut un fichier temporaire que l'on ne peut pas supprimer de façon sûre (l'application tierce le lit en asynchrone).
3. Aucun contrôle de l'étendue de pages, de l'échelle, de la progression, de l'annulation ni du résultat ; comportement différent d'un lecteur à l'autre.
4. Non testable automatiquement (dépend d'un lecteur tiers installé).

Il ne serait acceptable que comme entrée de menu explicite « Imprimer avec l'application par défaut », ajoutée plus tard et seulement si le verbe existe. Si le chemin natif prend du retard, **ne rien livrer** plutôt qu'un bouton fragile (règle « pas de bouton factice »).

---

## 3. API moteur (`sunipdf-core`)

### 3.1 Organisation

- `crates/sunipdf-core/src/print/mod.rs` : types, géométrie, découpage en bandes, rendu d'un bandeau (indépendant de la plateforme, testable sans imprimante), plus les ré-exports Windows.
- `crates/sunipdf-core/src/print/win.rs` (`#[cfg(windows)]`, **seul fichier contenant du `unsafe`** ; chaque bloc `unsafe` porte un commentaire `// SAFETY:`) : DC d'imprimante, `Document::print`, boîte native, imprimante par défaut, attente du fichier de sortie.
- `lib.rs` : ajouter `pub mod print;` (après `pub mod pages;`) et `pub use print::{PrintOptions, PrintOutcome, PrintScale};`.
- Pourquoi pas la crate `sunipdf-platform` annoncée dans `docs/ARCHITECTURE.md` : le moteur (rendu PDFium) et le test d'intégration `engine.rs` (moteur unique sérialisé) sont dans `sunipdf-core` ; une crate de plus doublerait les dépendances croisées. Le `unsafe` reste confiné à `print/win.rs`, extractible plus tard. À consigner comme décision (§11).

### 3.2 Erreurs (`error.rs`)

Ajouter à `EngineError` (avec `#[error(...)]` en anglais comme les autres) :

```rust
#[error("printer unavailable: {0}")] PrinterUnavailable(String), // code() = "printer_unavailable"
#[error("print job failed: {0}")]    PrintFailed(String),        // code() = "print_failed"
```

Réutilisés tels quels : `PageOutOfRange(usize)` (page demandée hors document), `SecurityRestricted` (droit `/P` absent), `RenderTooLarge` (bandeau/échelle invalide), `Other("no pages selected")` (liste vide).
Annulation utilisateur ≠ erreur : `PrintOutcome::Cancelled`.

### 3.3 Types et fonctions indépendants de la plateforme (`print/mod.rs`)

```rust
use std::ops::ControlFlow;
use std::path::PathBuf;
use crate::{Document, EngineError, PageSize, MAX_RENDER_SIDE_PX}; // lib.rs:36

pub const DEFAULT_MAX_DPI: u32 = 300;           // qualité « Standard »
pub const DPI_CAP_MIN: u32 = 72;
pub const DPI_CAP_MAX: u32 = 1200;
/// Taille maximale d'un bandeau BGR 24 bits ; `as_raw_bytes()` en fait une copie : pic ≈ 2 × cette valeur.
pub const BAND_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PrintScale { #[default] Shrink, Fit, ActualSize }   // id() : "shrink" | "fit" | "actual" ; from_id()

/// Géométrie du DC d'imprimante en pixels périphérique (origine = coin haut-gauche de la zone imprimable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaperGeometry {
    pub dpi_x: u32, pub dpi_y: u32,           // LOGPIXELSX / LOGPIXELSY
    pub paper_w: u32, pub paper_h: u32,       // PHYSICALWIDTH / PHYSICALHEIGHT
    pub printable_w: u32, pub printable_h: u32, // HORZRES / VERTRES
    pub offset_x: i32, pub offset_y: i32,     // PHYSICALOFFSETX / PHYSICALOFFSETY
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageLayout {
    pub rotate_ccw90: bool,
    pub dest_x: i32, pub dest_y: i32, pub dest_w: u32, pub dest_h: u32, // rectangle de destination (pixels périphérique)
    pub div_x: u32, pub div_y: u32,           // diviseurs entiers de résolution (≥ 1)
    pub src_w: u32, pub src_h: u32,           // taille en pixels de la page entière rendue (après rotation)
}

pub fn plan_page(page: PageSize, geom: &PaperGeometry, scale: PrintScale,
                 auto_rotate: bool, max_dpi: u32) -> Result<PageLayout, EngineError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band { pub top: u32, pub rows: u32, pub dest_top: u32, pub dest_rows: u32 }
pub fn bands(layout: &PageLayout, band_bytes: usize) -> Vec<Band>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripRotation { None, Ccw90 }
pub struct StripSpec { pub full_width: u32, pub full_height: u32, pub rotation: StripRotation }
/// Bandeau BGR 24 bits, lignes de haut en bas, `stride` multiple de 4.
pub struct Strip { pub width: u32, pub rows: u32, pub stride: usize, pub bgr: Vec<u8> }

impl Document<'_> {
    pub fn render_strip(&self, page_index: usize, spec: &StripSpec, top: u32, rows: u32)
        -> Result<Strip, EngineError>;
}

pub enum RangeChoice { All, Current, Ranges(Vec<(u32, u32)>) }  // plages 1-based inclusives
pub fn expand_ranges(choice: &RangeChoice, page_count: usize, current: usize) -> Vec<usize>; // 0-based, ordre conservé, hors-limites ignorés

#[derive(Clone, Debug)]
pub struct PrinterTarget { pub device: String, pub devmode: Option<Vec<u8>> } // DEVMODEW + zone pilote, octets bruts

#[derive(Clone, Debug)]
pub struct PrintOptions {
    pub target: PrinterTarget,
    pub pages: Vec<usize>,          // 0-based, dans l'ordre d'impression
    pub scale: PrintScale,
    pub auto_rotate: bool,
    pub max_dpi: u32,               // borné à 72..=1200
    pub job_name: String,           // nom affiché dans la file d'impression
    pub output_file: Option<PathBuf>, // imprimantes « vers fichier » (Print to PDF) : tests, CLI, automatisation
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrintProgress { pub pages_done: usize, pub pages_total: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintOutcome { Completed { pages: usize }, Cancelled { pages_done: usize } }
```

**Algorithme de `plan_page`** (arithmétique en `f64`, résultat en entiers ; `dpi_x/dpi_y/printable_*` nuls ou page non finie/≤ 0 → `PrintFailed("invalid printer geometry")`) :

1. `nat(w,h) = (w·dpi_x/72, h·dpi_y/72)` (taille naturelle en pixels périphérique) ; `fit(w,h) = min(printable_w/nat_w, printable_h/nat_h)`.
2. `rotate = auto_rotate && fit(h,w) > fit(w,h) · 1.01` (pivote seulement si cela améliore l'ajustement : une page carrée ou presque ne pivote jamais).
3. `factor = match scale { Fit => s_fit, Shrink => s_fit.min(1.0), ActualSize => 1.0 }` où `s_fit` est l'ajustement de l'orientation retenue.
4. `dest_w = max(1, round(nat_w·factor))`, idem `dest_h`.
5. Position (division entière avec troncature) : `centré = (paper − dest)/2 − offset`; si `dest ≤ printable`, `pos = centré.clamp(0, printable − dest)`, sinon `pos = centré` (la page dépasse : le DC rogne symétriquement ; c'est le comportement « taille réelle »).
6. Résolution de rendu bornée **par diviseurs entiers** (évite toute couture entre bandes) : `div_x = ceil(dpi_x/max_dpi)`, `div_y = ceil(dpi_y/max_dpi)` (≥ 1) ; `src_w = ceil(dest_w/div_x)`, `src_h = ceil(dest_h/div_y)` ; tant que `src_w` ou `src_h > MAX_RENDER_SIDE_PX`, incrémenter les deux diviseurs.

Exemple chiffré à reprendre en test unitaire : `geom = {dpi 600×600, paper 4961×7016, printable 4800×6850, offset (80,83)}`, `max_dpi = 300`, page 612×792 pt :
`Shrink` → `rotate=false`, `dest = 4800×6212`, `dest_x = 0`, `dest_y = 319`, `div = (2,2)`, `src = 2400×3106` ; `Fit` identique ; `ActualSize` → `dest = 5100×6600`, `dest_x = −149`, `dest_y = (7016−6600)/2 − 83 = 125` ; page 792×612 avec `auto_rotate` → `rotate=true`, même rectangle que le premier cas ; sans `auto_rotate` → `rotate=false`, `Shrink` ajuste la largeur (`factor = 4800/6600`) ; page A5 420×595 → `Shrink` : `factor = 1.0` (non agrandie, centrée), `Fit` : agrandie à ≈ 4800×6799 ; imprimante 1200×1200 ppp, `max_dpi 300` → `div = (4,4)` ; 2400×600 ppp → `div = (8,2)`.

**`bands`** : `stride = ((src_w·3 + 3)/4)·4` ; `rows_max = max(1, band_bytes/stride)` ; bandes successives `{top, rows = min(rows_max, src_h−top)}` ; `dest_top = top·div_y` ; `dest_rows = rows·div_y`, **sauf la dernière** qui prend `dest_h − dest_top`. Les rectangles destination sont donc contigus et leur somme vaut exactement `dest_h`.

**`render_strip`** (appels `pdfium-render`, tous vérifiés dans `PR/`) :

1. `let size = self.page_size(page_index)?` (`lib.rs:215`, `PageOutOfRange` si hors limites). Contrôles : `full_width`, `full_height` dans `1..=MAX_RENDER_SIDE_PX`, `rows ≥ 1`, `top + rows ≤ full_height`, sinon `RenderTooLarge` (même convention que `render_page`, `lib.rs:224-237`).
2. `let index = PdfPageIndex::try_from(page_index)…` puis `let page = self.inner.pages().get(index)?` (idiome identique à `lib.rs:234-243`).
3. `let mut bitmap = PdfBitmap::empty(full_width as Pixels, rows as Pixels, PdfBitmapFormat::BGR)?` (`PR/pdf/bitmap.rs:132` ; format `BGR` = 3 octets/pixel, `bitmap.rs:61` et `:96`). Échec d'allocation (`PdfiumLibraryInternalError(Unknown)`) → mapper en `RenderTooLarge`.
4. Configuration :
   ```rust
   let mut config = PdfRenderConfig::new()
       .set_fixed_size(full_width as Pixels, full_height as Pixels) // render_config.rs:158 : taille de la PAGE ENTIÈRE
       .set_origin(0, -(top as Pixels))                             // render_config.rs:681 : décalage négatif = bandeau
       .set_format(PdfBitmapFormat::BGR)                            // :360
       .set_reverse_byte_order(false)                               // :510 — défaut = true (RVB) ; StretchDIBits veut BGR
       .use_print_quality(true)                                     // :466 (FPDF_PRINTING)
       .render_annotations(true)                                    // :402 (FPDF_ANNOT)
       .render_form_data(true);                                     // :393 (par défaut) — obligatoire pour que start_x/start_y soient honorés
   if spec.rotation == StripRotation::Ccw90 {
       config = config.rotate(PdfPageRenderRotation::Degrees270, false); // :304 ; 270° horaire = 90° anti-horaire (as_pdfium() = 3)
   }
   ```
   Ne **pas** utiliser `clip()` (`:655`) ni les transformations : elles désactivent `render_form_data` et imposent le chemin `FPDF_RenderPageBitmapWithMatrix` (`PR/pdf/document/page.rs:706`).
5. `page.render_into_bitmap_with_config(&mut bitmap, &config)?` (`PR/pdf/document/page.rs:620` → `render_into_bitmap_with_settings` `:631` : `FPDFBitmap_FillRect` `:642`, `FPDF_RenderPageBitmap` `:658`, `FPDF_FFLDraw` `:689`). La documentation de `set_origin` (`render_config.rs:667-680`) décrit précisément ce cas d'usage (« bitmap de la taille d'une bande + décalage `y` négatif »). ⚠ Le `FillRect` reçoit la taille de la page entière (`settings.width/height`) alors que le bitmap est plus petit : PDFium doit rogner ; **le test `render_strip_matches_full_render` tranche**.
6. `let bgr = bitmap.as_raw_bytes()` (`bitmap.rs:288`, copie) ; `stride = bgr.len() / rows` (idiome de `bitmap.rs:301`) ; vérifier `stride ≥ width·3` et `stride % 4 == 0` (règle PDFium, `bitmap.rs:421-428`).

**Effets** : aucun `regenerate_content`, aucun instantané, aucune mutation. Le contenu édité est déjà régénéré par chaque opération d'édition (`edit.rs:242, 261, 291…`). Une page est rechargée par bandeau (coût négligeable devant le rendu) ; les images décodées sont réutilisées par le cache de page de PDFium entre bandes.

**`expand_ranges`** : `All` → `0..page_count` ; `Current` → `[current]` si valide ; `Ranges` → pour chaque `(a,b)` avec `1 ≤ a ≤ b`, indices `a−1..=min(b,page_count)−1` ; plages invalides ignorées.

### 3.4 Partie Windows (`print/win.rs`)

Toutes les signatures ci-dessous sont celles de **windows-sys 0.61.2** (dépendance recommandée, §9) ; les équivalents `windows` 0.62.2 sont donnés en dernière colonne si l'équipe préfère les types sûrs. Dans `windows-sys` les constantes `GET_DEVICE_CAPS_INDEX` sont des `u32` alors que `GetDeviceCaps` prend un `i32` : écrire `HORZRES as i32`.

| Appel | windows-sys 0.61.2 | windows 0.62.2 |
|---|---|---|
| `CreateDCW(pdriver, pdevice, pport, pdm) -> HDC` | `WS/Graphics/Gdi/mod.rs:38` | `W/Graphics/Gdi/mod.rs:222` |
| `DeleteDC` | `Gdi:74` | `Gdi:448` |
| `GetDeviceCaps(hdc, index: i32) -> i32` ; constantes `HORZRES` `:2931`, `VERTRES` `:4127`, `LOGPIXELSX/Y` `:3069-3070`, `PHYSICALWIDTH` `:3646`, `PHYSICALHEIGHT` `:3643`, `PHYSICALOFFSETX/Y` `:3644-3645` | `Gdi:168` | `Gdi:954` ; constantes `:5468`, `:7024`, `:5693-5694`, `:6321`, `:6318`, `:6319-6320` |
| `SetStretchBltMode` + `STRETCH_HALFTONE` | `Gdi:351`, `:3898` | `Gdi:1911`, `:6651` |
| `StretchDIBits(hdc, xd, yd, wd, hd, xs, ys, ws, hs, bits, bmi, usage, rop) -> i32` ; `BITMAPINFO`/`BITMAPINFOHEADER` ; `BI_RGB` ; `DIB_RGB_COLORS` ; `SRCCOPY` | `Gdi:365` ; `:559`, `:570` ; `:640` ; `:1083` ; `:3888` | `Gdi:1981` ; `:2319`, `:2330` ; `:2402` ; `:3007` ; `:6639` |
| `StartDocW`, `StartPage`, `EndPage`, `EndDoc`, `AbortDoc` ; `DOCINFOW { cbSize, lpszDocName, lpszOutput, lpszDatatype, fwType }` | `WS/Storage/Xps/mod.rs:22, 24, 10, 8, 2` ; struct `:73` | `W/Storage/Xps/mod.rs:73, 79, 37, 31, 5` ; struct `:126` |
| `PrintDlgExW(*mut PRINTDLGEXW) -> HRESULT` ; `PRINTDLGEXW` (variante x64/ARM64, `#[repr(C)]`) ; `PRINTPAGERANGE` ; `DEVNAMES` ; `START_PAGE_GENERAL` | `WS/UI/Controls/Dialogs/mod.rs:23` ; `:1229` ; `:1335` ; `:351` ; `:1368` | `W/UI/Controls/Dialogs/mod.rs:98` ; `:1601` ; `:1728` ; `:548` ; `:1761` |
| Drapeaux `PD_ALLPAGES` `:1028`, `PD_COLLATE` `:1029`, `PD_CURRENTPAGE` `:1030`, `PD_HIDEPRINTTOFILE` `:1039`, `PD_NOSELECTION` `:1043`, `PD_PAGENUMS` `:1045`, `PD_RETURNDC` `:1051`, `PD_USEDEVMODECOPIESANDCOLLATE` `:1057`, `PD_RESULT_APPLY/CANCEL/PRINT` `:1048-1050` | `WS/UI/Controls/Dialogs/mod.rs` | `W/UI/Controls/Dialogs/mod.rs:1416-1445` |
| `GlobalLock`, `GlobalSize`, `GlobalUnlock` ; `GlobalFree`, `GetLastError` | `WS/System/Memory/mod.rs:33, 35, 36` ; `WS/Foundation/mod.rs:7, 6` | `W/System/Memory/mod.rs:158, 169, 174` ; `W/Foundation/mod.rs:32, 27` |
| `GetDefaultPrinterW(pszbuffer, *mut u32) -> BOOL` | `WS/Graphics/Printing/mod.rs:137` | `W/Graphics/Printing/mod.rs:910` |
| `GetActiveWindow() -> HWND` | `WS/UI/Input/KeyboardAndMouse/mod.rs:5` | `W/UI/Input/KeyboardAndMouse/mod.rs:23` |
| Codes d'erreur : `ERROR_PRINT_CANCELLED` (63), `ERROR_CANCELLED` (1223), `ERROR_PRINTER_NOT_FOUND` (3012), `ERROR_INVALID_PRINTER_NAME` (1801) | `WS/Foundation/mod.rs:3607, 1172, 3604, 2814` | idem |

Structures utiles : `DEVMODEW` (`W/Graphics/Gdi/mod.rs:2789`, idem dans `WS`) : `dmSize` est à l'offset 68 et `dmDriverExtra` à l'offset 70 (à figer dans un test `core::mem::offset_of!`) ; le DEVMODE complet fait `dmSize + dmDriverExtra` octets. Les offsets de `DEVNAMES` sont **en caractères** (UTF-16) depuis le début de la structure (doc Microsoft `DEVNAMES`, web).

**`pub fn default_printer() -> Option<String>`** : `GetDefaultPrinterW(null, &mut len)` (premier appel : renvoie FALSE et `len`), allouer `Vec<u16>` de `len`, second appel, tronquer au `\0`.

**`pub fn wait_for_output(path: &Path, timeout: Duration) -> bool`** : interroge toutes les 250 ms ; renvoie `true` quand le fichier existe, est non vide et que sa taille est identique sur deux lectures consécutives. Nécessaire car *Microsoft Print to PDF* écrit probablement le fichier **après** le retour d'`EndDoc` (traitement du spouleur) ⚠ à confirmer à l'étape 3 ; servira à la CLI et aux tests.

**`pub fn show_print_dialog(req: &DialogRequest) -> Result<Option<DialogChoice>, EngineError>`** avec
`pub struct DialogRequest { pub page_count: usize, pub current_page: usize }` et
`pub struct DialogChoice { pub target: PrinterTarget, pub pages: Vec<usize> }` (`None` = annulé) :

1. `owner = GetActiveWindow()` ; nul → `PrintFailed("no active window")`. (Le thread appelant est le thread UI : la fenêtre eframe est active quand l'utilisateur appuie sur Ctrl+P. Évite d'importer `raw-window-handle`.)
2. `ranges = vec![PRINTPAGERANGE::default(); 64]` ; `ranges[0] = {1, page_count}`.
3. `let mut pd: Box<PRINTDLGEXW> = Box::default();` — la doc Microsoft exige une structure **allouée dynamiquement** ; `Default` est un `zeroed()` (`WS/UI/Controls/Dialogs/mod.rs`, juste après `:1229`). Renseigner : `lStructSize = size_of::<PRINTDLGEXW>()`, `hwndOwner = owner` (**ne peut pas être nul**, doc `PRINTDLGEXW`), `Flags = PD_ALLPAGES | PD_RETURNDC | PD_USEDEVMODECOPIESANDCOLLATE | PD_HIDEPRINTTOFILE | PD_NOSELECTION`, `nPageRanges = 1`, `nMaxPageRanges = 64` (doit être > 0 sans `PD_NOPAGENUMS`), `lpPageRanges = ranges.as_mut_ptr()`, `nMinPage = 1`, `nMaxPage = page_count`, `nCopies = 1`, `nStartPage = START_PAGE_GENERAL`.
   - `PD_USEDEVMODECOPIESANDCOLLATE` : le pilote gère copies et assemblage via le DEVMODE (la case est grisée s'il ne sait pas) ; **avec `PD_RETURNDC`** car la doc impose les deux pour obtenir `dmCopies`/`dmCollate` corrects. Le DC renvoyé n'est pas utilisé : `DeleteDC(pd.hDC)` après lecture.
4. `hr = PrintDlgExW(&mut *pd)` ; `hr < 0` → `PrintFailed(format!("PrintDlgEx: 0x{hr:08X}"))` (E_HANDLE, E_INVALIDARG, E_OUTOFMEMORY…).
5. Seul `dwResultAction == PD_RESULT_PRINT` produit un choix ; `PD_RESULT_CANCEL` et `PD_RESULT_APPLY` → `Ok(None)` (mais libérer les handles quand même).
6. Extraire : nom de l'imprimante = chaîne UTF-16 à `ptr_devnames + wDeviceOffset` (le nom de `dmDeviceName` est tronqué à 32 caractères, la doc recommande `wDeviceOffset`) ; DEVMODE = copie de `dmSize + dmDriverExtra` octets, bornée par `GlobalSize(hDevMode)` ; pages = `expand_ranges(Current si Flags & PD_CURRENTPAGE, Ranges(...) si Flags & PD_PAGENUMS, sinon All)`.
7. Libérer : `GlobalUnlock` puis `GlobalFree(hDevMode)`, `GlobalFree(hDevNames)` (la doc `PrintDlgEx` impose de libérer ces blocs), `DeleteDC(hDC)`.

La boîte est **modale et bloque le thread UI** pendant son affichage, comme les `rfd::FileDialog::pick_file()` déjà utilisés dans `app.rs` ; aucun travail PDF n'est fait à ce moment.

**`impl Document<'_> { pub fn print(&self, options: &PrintOptions, progress: &mut dyn FnMut(PrintProgress) -> ControlFlow<()>) -> Result<PrintOutcome, EngineError> }`**

Ordre des contrôles (important : les deux premiers n'exigent aucune imprimante, donc testables partout) :

1. `pages` vide → `Other("no pages selected")` ; `p ≥ page_count` → `PageOutOfRange(p)` ; `max_dpi` borné à `72..=1200`.
2. `self.properties().can_print` faux → `SecurityRestricted` (`security.rs:249-300` ; coûte une sérialisation du document, une fois par impression).
3. `open_dc` : `CreateDCW("WINSPOOL", device, null, devmode_alignée)` (`Gdi:38`) ; retour `NULL` → `PrinterUnavailable(device)`. Le DEVMODE (octets, alignement 1) est copié dans un `Vec<u32>` avant l'appel (alignement 4 requis). RAII : `Drop` appelle `DeleteDC`.
4. `geometry` : 8 appels `GetDeviceCaps`. `StartDocW(hdc, &DOCINFOW { cbSize = size_of, lpszDocName = job_name, lpszOutput = output_file ou NULL, lpszDatatype = NULL, fwType = 0 })` ; retour ≤ 0 : si `GetLastError()` vaut 63 ou 1223 (l'utilisateur a annulé la boîte « Enregistrer la sortie » de *Print to PDF* ⚠) → `Ok(Cancelled{pages_done:0})`, sinon `PrintFailed("StartDoc: {code}")`. Un garde RAII `JobGuard` appelle `AbortDoc` à la destruction si le travail n'est pas terminé : toute erreur ou `?` annule le travail entier (tout ou rien).
5. Pour chaque page `n` (progression : appel du rappel **avant la page** et **avant chaque bande** avec `pages_done = n` ; `Break` → `AbortDoc`, `Ok(Cancelled{pages_done:n})`) :
   `plan_page` → `StartPage` (≤ 0 → erreur) → `SetStretchBltMode(hdc, STRETCH_HALFTONE)` (à refaire après chaque `StartPage`) → pour chaque `Band` : `render_strip(...)` puis
   `StretchDIBits(hdc, dest_x, dest_y + dest_top, dest_w, dest_rows, 0, 0, strip.width, strip.rows, strip.bgr.as_ptr(), &bmi, DIB_RGB_COLORS, SRCCOPY)` avec `bmi.bmiHeader = { biSize: 40, biWidth: width, biHeight: −rows (haut en bas), biPlanes: 1, biBitCount: 24, biCompression: BI_RGB, biSizeImage: stride·rows }`. Retour `0` = échec ou aucune ligne copiée (doc `StretchDIBits`, web) → `PrintFailed`. Puis `EndPage` (≤ 0 → erreur).
6. Appel final du rappel avec `pages_done = total` (un `Break` à ce stade est ignoré), `EndDoc` (≤ 0 → erreur), `Ok(Completed{pages: total})`.

Notes : les paramètres de page du pilote (papier, orientation, recto-verso, couleur, copies) viennent du DEVMODE passé à `CreateDCW` et se retrouvent dans `GetDeviceCaps` ; l'application ne les recalcule jamais. `StartDocW` est bloquant (doc web) : c'est une raison de plus pour l'exécuter sur le thread moteur.

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

L'impression est **séquentielle sur le thread moteur** (PDFium n'est pas thread-safe, `worker.rs:1-3`). Elle bloque les autres requêtes du moteur pendant sa durée ; c'est acceptable car la boîte de progression est modale (l'utilisateur ne peut ni changer d'onglet ni éditer). Ce n'est **ni un `EditOp`** (aucun instantané, cf. `State::edit`, `worker.rs:372`) **ni un `BatchOp`** (qui ne gère ni progression ni annulation) : une requête dédiée.

```rust
// types
pub struct PrintJob {
    pub options: sunipdf_core::print::PrintOptions,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>, // posé par l'UI, lu entre deux bandes
}
// Request (worker.rs:178)   : + Print(PrintJob)
// Response (worker.rs:238)  : + PrintProgress { done: usize, total: usize }
//                             + PrintFinished(Result<sunipdf_core::print::PrintOutcome, EngineError>)
```

Dans `run()` (`worker.rs:618`), nouveau bras du `match request` (à côté de `Request::Batch`, `:728`) :

```rust
Request::Print(job) => {
    // Ne PAS faire `continue` ici : la boîte modale de l'UI attendrait indéfiniment.
    let Some(doc) = &state.document else {
        let _ = tx.send(Response::PrintFinished(Err(EngineError::Other("no document".into()))));
        continue;
    };
    let mut last = usize::MAX;
    let result = doc.print(&job.options, &mut |p| {
        if p.pages_done != last {
            last = p.pages_done;
            let _ = tx.send(Response::PrintProgress { done: p.pages_done, total: p.pages_total });
            ctx.request_repaint();
        }
        if job.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    });
    Response::PrintFinished(result)
}
```

`tx` et `ctx` sont les variables de `run` ; l'emprunt de `tx` par la fermeture se termine avant l'envoi de la réponse en fin de boucle (`worker.rs:823`). Le document n'est pas modifié : `undo`/`redo`, caches et pré-rendu restent intacts. **Onglets** : `Print` s'applique au document actif ; comme la boîte est modale, aucun `Suspend`/`Resume` ne peut s'intercaler.

---

## 5. Interface

### 5.1 Commande, raccourci, menus (`crates/sunipdf-core/src/command.rs`, `apps/sunipdf-desktop/src/*`)

| Élément | Changement |
|---|---|
| `Command` (enum, `command.rs:4`) | + `Print` (après `SaveAs`, ligne 21) |
| `REGISTRY` | `cmd!(Print, "file.print", "menu.file.print", Some("shortcut.print"), true, File)` inséré après l'entrée `SaveAs` (`command.rs:~135-141`) ; `needs_document = true` : sans document, `execute` ouvre d'abord un fichier puis rejoue la commande (`app.rs:793-799`, comportement D020) |
| Test `command.rs:648` | `assert_eq!(Command::in_menu(Menu::File).count(), 7)` → **8** |
| `icons.rs` (module `glyph`, après `NUMBERS` `:69`) | `pub const PRINT: &str = "\u{E749}";` (Segoe MDL2 « Print » ⚠ à vérifier visuellement) |
| `shell.rs::command_icon` (`:252` voisinage) | `Command::Print => (glyph::PRINT, "🖨"),` |
| `shell.rs::command_color` | rien : `Menu::File` → `theme::CAT_FILE` |
| Menu Fichier (`shell.rs:464`) | séparateurs : `&[Command::Save, Command::Print, Command::Quit]` |
| `app.rs::handle_input` | `key(Modifiers::COMMAND, Key::P, Command::Print);` près de `Key::S` (`:725`) ; ajouter `(Key::P, Command::Print)` à la liste des raccourcis actifs pendant la saisie de texte (`:608`) ; ajouter `\|\| self.print.options_open \|\| self.print.job.is_some()` aux conditions de retour anticipé (`:595`) |
| `app.rs::execute` (`:840` voisinage) | `Command::Print => self.on_print_command(ctx),` |

**Deux cas selon l'ordre d'atterrissage de la tranche « tool-hub »** (`docs/plans/tool-hub.md`, qui supprime `ToolTab` et impose le test `every_registry_command_has_a_home`) :

- **Cas A, hub pas encore atterri** : ajouter `Command::Print` aux listes `ToolTab::All` (avant `SaveAs`, `shell.rs:74`) et `ToolTab::Read` (après `Properties`, `shell.rs:80`) ; ajouter un bouton icône `glyph::PRINT` avant l'icône Enregistrer dans la barre d'outils (bloc `if self.document.is_some()`, `shell.rs:529`), info-bulle `menu.file.print`.
- **Cas B, hub atterri (ordre recommandé)** : ne rien ajouter à `ToolTab` (disparu) ; `Print` est une commande **système** (aucun outil PDFCraft ne la possède) : l'ajouter à la liste `SYSTEM_COMMANDS` du test `every_registry_command_has_a_home` et à l'accès rapide de la barre d'outils (`quick_button(ui, ctx, Command::Print)`, après Enregistrer). Couleur : `CAT_FILE` (règle « commandes système » du hub).

### 5.2 État et dialogues (nouveau fichier `apps/sunipdf-desktop/src/printing.rs`, déclaré `mod printing;` dans `main.rs`)

```rust
#[derive(Default)]
pub struct PrintState {
    pub options_open: bool,          // boîte « Imprimer » (réglages)
    pub job: Option<ActiveJob>,      // boîte « Impression en cours »
}
pub struct ActiveJob {
    pub cancel: Arc<AtomicBool>,
    pub printer: String,
    pub done: usize,
    pub total: usize,
    pub cancelling: bool,
}
```

- `SuniApp` gagne `pub print: crate::printing::PrintState` (`app.rs:~120`, initialisé par `Default` dans `new` `:149`). L'état est global et transitoire (modal) : il n'entre pas dans `TabState` (`tabs.rs`).
- `impl SuniApp` dans `printing.rs`, même style que `tools.rs` :
  - `pub fn on_print_command(&mut self, ctx)` : si `SUNIPDF_PRINT_DEVICE` est défini (hook d'automatisation, §8.4) → `start_print(Some(auto))` directement ; sinon `self.print.options_open = true`.
  - `pub fn print_options_dialog(&mut self, ctx)` : `dialog::show(ctx, Spec::new("print", tr("print.title"), dark, has_icons).subtitle(tr("print.subtitle")).icon(glyph::PRINT, "🖨").width(480.0), body, footer)` ; corps : `field_label(print.scale)` + trois `ui.radio_value(&mut scale, …)` (même idiome que `tools.rs` `ui.radio_value(&mut form.all_pages, …)`), `ui.checkbox(&mut auto_rotate, tr("print.auto_rotate"))`, `egui::ComboBox::from_id_salt("print-quality")` à trois entrées, note grisée `print.note` ; pied : `dialog::primary(ui, tr("print.next"))` ou Entrée → valider ; `dialog::secondary(ui, tr("dialog.password.cancel"))` ou Échap → fermer. Les trois valeurs sont lues/écrites dans `self.settings` (champs ajoutés ci-dessous) puis `persist_settings()` à la validation.
  - `fn start_print(&mut self, auto: Option<AutoPrint>)` : récupère `page_count`/`current` depuis `self.document` ; **sans** automatisation → `sunipdf_core::print::show_print_dialog(&DialogRequest{..})` : `Ok(None)` → ne rien faire ; `Err(e)` → `self.error = Some(error_key(&e))` ; `Ok(Some(choice))` → construire `PrintOptions` (nom de job = nom du fichier, `max_dpi` selon la qualité), créer `Arc<AtomicBool>`, remplir `self.print.job`, envoyer `Request::Print(PrintJob{..})`.
  - `pub fn print_progress_dialog(&mut self, ctx)` : `dialog::show` avec `Spec::new("print-progress", tr("print.progress.title"), …).icon(glyph::PRINT, "🖨").width(440.0)` ; corps : `egui::ProgressBar::new(done as f32 / total as f32).text(tr_args("print.progress.text", current/total/printer))` (API : `EG/widgets/progress_bar.rs:27, :62`) et libellé d'accessibilité `a11y.print_progress` sur la barre ; pied : `dialog::secondary(ui, tr(if cancelling {"print.progress.cancelling"} else {"print.progress.cancel"}))` ou Échap → `cancel.store(true, Relaxed)`, `cancelling = true` (bouton ensuite sans effet).
  - `pub fn on_print_progress(&mut self, done, total)` et `pub fn on_print_finished(&mut self, result)` : `self.print.job = None` ; `Ok(Completed{pages})` → `status_msg = tr_args("print.done", pages, printer)` ; `Ok(Cancelled{..})` → `status_msg = tr("print.cancelled")` ; `Err(e)` → `log::warn!` + `self.error = Some(error_key(&e))`.
- `app.rs::handle_responses` (`:478` voisinage) : `Response::PrintProgress{done,total} => self.on_print_progress(done,total)`, `Response::PrintFinished(r) => self.on_print_finished(r)`.
- `app.rs::ui` (`:1877`, avec les autres `*_dialog`) : `self.print_options_dialog(&ctx); self.print_progress_dialog(&ctx);`.
- Fermeture de la fenêtre pendant un travail (`app.rs:1868`, bloc `close_requested`) : si `self.print.job.is_some()` → `ctx.send_viewport_cmd(ViewportCommand::CancelClose)` et poser le drapeau d'annulation (l'utilisateur referme une fois le travail annulé).
- `error_key` (`app.rs:1924`) : `PrinterUnavailable(_) => "error.printer_unavailable"`, `PrintFailed(_) => "error.print_failed"` ; compléter la liste du test `every_error_key_exists_in_both_catalogs` (`app.rs:~1944`).
- `settings.rs` (`#[serde(default)]`, compatible avec les anciens fichiers) : `print_scale: String` (`"shrink"`), `print_auto_rotate: bool` (`true`), `print_dpi: u32` (`300`) ; étendre le test `round_trip_and_defaults`.

### 5.3 Interaction avec annulation (undo), onglets, enregistrement

- Aucune entrée d'annulation, aucun `dirty`. Imprimer un document modifié ne l'enregistre pas ; la boîte « Imprimer » rappelle que les modifications non enregistrées sont imprimées (`print.note`).
- Onglets : un seul document actif dans le moteur ; la boîte de progression est modale (`egui::Modal` via `dialog::show`) donc `switch_to`, `Ctrl+Tab`, la fermeture d'onglet et les raccourcis sont inertes (ajout de la condition dans `handle_input`).
- Droit d'impression refusé → `Response::PrintFinished(Err(SecurityRestricted))` → bandeau `error.security` (clé existante).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Sous-commande ajoutée à l'enum `Command` (messages d'aide en anglais, D009) et au `match` de `run` :

```text
sunipdf-cli print <FILE> [--printer <NAME>] [--pages all|1,3-5] [--scale shrink|fit|actual]
                         [--no-rotate] [--dpi 300] [--output-file <PATH>] [--wait 30] [--password <PW>]
```

- `--printer` absent → `print::default_printer()` ; `None` → `PrinterUnavailable("no default printer")`.
- `--pages` réutilise `parse_pages` (`main.rs:~630`), 1-based ; `--scale` via `PrintScale::from_id` (id inconnu → `Other("unknown scale: …")`, comme `unknown position`).
- `--dpi` → `max_dpi` (bornes 72–1200). `--output-file` : pour les imprimantes « vers fichier » ; avec `--wait N` (défaut 30 s, 0 = ne pas attendre) la commande appelle `wait_for_output` avant de rendre la main, afin que les scripts trouvent le fichier.
- Pas de boîte de dialogue, pas de copies (réglage du pilote). Pas de sous-commande `printers` : `Get-Printer` suffit (YAGNI) — à signaler dans l'aide de `print`.
- Sortie JSON (stdout) :
  ```json
  { "printer": "Microsoft Print to PDF", "pages": [1, 2], "page_count": 2, "scale": "shrink",
    "max_dpi": 300, "outcome": "completed", "output_file": "C:\\Temp\\out.pdf", "output_ready": true }
  ```
  `output_file`/`output_ready` valent `null` sans `--output-file`. Erreurs sur stderr au format existant `{ "error": code, "message": … }` avec les codes `printer_unavailable`, `print_failed`, `security_restricted`, `page_out_of_range` ; code de sortie 1 (le `match` de `main`, `:664-667`, n'a pas à changer).
- Test unitaire CLI : analyse de `--scale` et de `--pages`. Essai réel noté dans le rapport d'étape : `sunipdf-cli print crates\sunipdf-core\tests\fixtures\two-pages.pdf --printer "Microsoft Print to PDF" --output-file $env:TEMP\sunipdf-cli-print.pdf` puis `sunipdf-cli info $env:TEMP\sunipdf-cli-print.pdf` → `page_count: 2`.

---

## 7. i18n — 24 nouvelles clés (à ajouter dans `assets/i18n/fr-FR.json` **et** `en-US.json`, en fin de fichier ; penser à la virgule après la dernière clé actuelle `status.stamped`)

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.print` | Imprimer… | Print… |
| `shortcut.print` | Ctrl+P | Ctrl+P |
| `print.title` | Imprimer | Print |
| `print.subtitle` | Mise en page, puis choix de l'imprimante | Page layout, then choose the printer |
| `print.scale` | Mise à l'échelle | Page scaling |
| `print.scale.shrink` | Réduire les pages trop grandes (sans agrandir) | Shrink oversized pages (never enlarge) |
| `print.scale.fit` | Adapter à la zone imprimable | Fit to printable area |
| `print.scale.actual` | Taille réelle (100 %) | Actual size (100%) |
| `print.auto_rotate` | Pivoter les pages pour suivre l'orientation du papier | Rotate pages to match the paper orientation |
| `print.quality` | Qualité d'impression | Print quality |
| `print.quality.draft` | Brouillon (150 ppp) | Draft (150 dpi) |
| `print.quality.standard` | Standard (300 ppp) | Standard (300 dpi) |
| `print.quality.high` | Haute (600 ppp) | High (600 dpi) |
| `print.note` | L'imprimante, les pages et le nombre de copies se choisissent à l'étape suivante, dans la boîte de dialogue Windows. Le document est imprimé tel qu'il est affiché, modifications non enregistrées comprises. | The printer, pages and number of copies are chosen in the next step, in the Windows dialog. The document is printed as displayed, including unsaved changes. |
| `print.next` | Suivant… | Next… |
| `print.progress.title` | Impression en cours | Printing |
| `print.progress.text` | Page {current} sur {total} — {printer} | Page {current} of {total} — {printer} |
| `print.progress.cancel` | Annuler l'impression | Cancel printing |
| `print.progress.cancelling` | Annulation en cours… | Cancelling… |
| `print.done` | Impression terminée : {pages} page(s) envoyée(s) à {printer} | Printing finished: {pages} page(s) sent to {printer} |
| `print.cancelled` | Impression annulée | Printing cancelled |
| `error.printer_unavailable` | L'imprimante est introuvable ou indisponible. Vérifiez qu'elle est installée et en ligne. | The printer was not found or is unavailable. Check that it is installed and online. |
| `error.print_failed` | L'impression a échoué. Le document n'a pas été modifié. | Printing failed. The document was not modified. |
| `a11y.print_progress` | Progression de l'impression | Printing progress |

Clés existantes réutilisées : `dialog.password.cancel` (« Annuler »), `error.security`. Les paramètres `{current}`, `{total}`, `{printer}`, `{pages}` sont identiques dans les deux langues (test `placeholders_match_between_languages`). Le test `ui_keys_used_by_app_exist` (`app.rs:~1964`) ne lit que `app.rs` et `main.rs` : ajouter dans `printing.rs` un test équivalent sur `include_str!("printing.rs")`. Ne pas traduire le nom de l'imprimante ni le contenu du PDF.

---

## 8. Tests

### 8.1 Tests unitaires (dans `crates/sunipdf-core/src/print/mod.rs`, sans PDFium ni imprimante)

- `plan_page` : les cas chiffrés de §3.3 (A4/Letter, paysage avec et sans rotation, carré sans rotation, A5 `Shrink` vs `Fit`, `ActualSize` rogné et centré, diviseurs 600→300 = 2, 1200→300 = 4, 2400×600 → (8,2), 300 natifs avec `max_dpi` 600 → 1, dépassement de `MAX_RENDER_SIDE_PX` qui incrémente les diviseurs, géométrie nulle → `PrintFailed`).
- `bands` : couverture exacte (`Σ rows = src_h`, `Σ dest_rows = dest_h`, contiguïté des `dest_top`), `stride·rows ≤ band_bytes`, une seule bande quand la page tient (A4 300 ppp = 26 Mo < 32 Mo), cas `band_bytes` minuscule (au moins 1 ligne par bande).
- `expand_ranges` : `All`, `Current`, plages `[(1,3),(5,5)]` → `[0,1,2,4]`, plage hors limites ignorée/tronquée, `b < a` ignoré.
- (`#[cfg(windows)]`) `offset_of!(DEVMODEW, dmSize) == 68` et `dmDriverExtra == 70` ; lecture d'un `DEVNAMES` synthétique (tampon `Vec<u16>` : en-tête de 4 mots puis chaînes, offsets en caractères).

### 8.2 Tests d'intégration (`crates/sunipdf-core/tests/engine.rs`, moteur partagé `engine()`, fixture `two-pages.pdf` : page 1 portrait 612×792 avec rectangle bleu `72,500,200×150` et texte bleu « SuniPDF page 1 » ; page 2 paysage 792×612 avec rectangle rouge `72,72,300×100` et texte rouge « SuniPDF page 2 » — `scripts/make_fixtures.py:37-46`)

Helper de test commun `bbox_dense(img, pred)` : compte les pixels vérifiant `pred` par ligne et par colonne, ne garde que les lignes/colonnes dont le compte atteint **50 % du maximum**, renvoie la boîte englobante. Le texte (traits fins) est ainsi exclu et seul le rectangle plein est mesuré.

1. **`render_strip_matches_full_render`** — `full = doc.render_page(0, 1.5)` (918×1188, `lib.rs:224`) ; trois bandeaux `(top,rows)` = `(0,400)`, `(400,400)`, `(800,388)` avec `StripSpec{918,1188,None}` ; assertions : `width = 918`, `stride ≥ 2754` et multiple de 4, `bgr.len() = stride·rows` ; pour chaque pixel `|Δcanal| ≤ 4` par rapport à `full` (conversion BGR↔RGBA), le pixel `(250,300)` vaut exactement B=255,G=0,R=0 (rectangle bleu) ; erreurs : `top+rows > full_height`, `rows = 0`, `full_width > MAX_RENDER_SIDE_PX` → `RenderTooLarge` ; page 9 → `PageOutOfRange(9)`. **C'est le test qui valide l'hypothèse du `FillRect` rogné (⚠ §3.3 point 5).**
2. **`render_strip_rotation_ccw_places_content`** — page 2, `StripSpec{612,792,Ccw90}`, un bandeau de 792 lignes : `bbox_dense` des pixels rouges (R>200, G<60, B<60) donne un rectangle d'environ **100 px de large × 300 px de haut** (±4) dont le centre est à peu près en **(490, 570)** (±6, zone droite-basse : la rotation anti-horaire envoie le coin bas-gauche de la page en bas-droite) ; sans rotation (`792×612`), centre ≈ (222, 490), taille 300×100. Si l'observation est inversée, corriger la correspondance `Degrees270`/`Degrees90` (le test fige le sens voulu, il ne se règle pas sur l'observation).
3. **`print_rejects_invalid_requests`** (Windows, aucune imprimante requise) — pages `[]` → `Other` ; pages `[5]` → `PageOutOfRange(5)` ; imprimante `"SuniPDF-no-such-printer"` → `PrinterUnavailable` ; document protégé par `security::protect(&bytes, None, &Protection{ user_password:"user", owner_password:"owner", allow_print:false, ..Default::default() })` (même montage que `tests/engine.rs:462-500`) ouvert avec `"user"` → `SecurityRestricted`, **même avec une imprimante inexistante** (le contrôle de permission précède l'ouverture du DC).
4. **`print_to_pdf_two_pages`** (Windows) — voir ci-dessous.
5. **`print_cancel_aborts_job`** (Windows) — rappel qui renvoie `Break` dès `pages_done ≥ 1` → `Ok(Cancelled{pages_done:1})` ; puis, jusqu'à 3 s, le fichier de sortie n'apparaît pas (⚠ si *Print to PDF* laisse un fichier partiel, supprimer cette dernière assertion et ne garder que le résultat `Cancelled`).

**Détail de `print_to_pdf_two_pages`**

1. Cible : `PrinterTarget{ device: "Microsoft Print to PDF".into(), devmode: None }` (valeurs par défaut du pilote : A4 ou Letter, portrait). Sortie : `std::env::temp_dir().join(format!("sunipdf-print-{}.pdf", std::process::id()))`, supprimée au préalable et à la fin.
2. **Saut** (`eprintln!("SKIP …")` + `return`) si `print` renvoie `PrinterUnavailable` (poste sans cette imprimante, ex. runner CI) ou si la variable `SUNIPDF_SKIP_PRINT_TESTS` est définie. Aucune détection préalable séparée n'est nécessaire.
3. `PrintOptions{ pages:[0,1], scale: Shrink, auto_rotate: true, max_dpi: 150, job_name:"SuniPDF test", output_file: Some(out) }` (150 ppp : spool minimal et test rapide).
4. Rappel de progression : enregistre la suite des `pages_done` ; attendu monotone, premier = 0, dernier = 2. Résultat attendu `Ok(Completed{pages:2})`.
5. `wait_for_output(&out, 30 s)` doit renvoyer `true` ; l'en-tête commence par `%PDF-` et la fin contient `%%EOF`.
6. Réouverture avec le moteur : `page_count() == 2`, les deux pages ont la **même** taille, égale à A4 (≈595×842) ou Letter (612×792) à ±3 pt.
7. Pixels (`render_page(i, 1.0)` sur la sortie, `bbox_dense`) :
   - page 1 : pixels « bleus » (B>180, R<80, G<80) : largeur de la boîte dense de **170 à 210 px** (200 pt × facteur d'ajustement ≈ 0,97) et centre dans la **moitié gauche** de la feuille ;
   - page 2 : pixels « rouges » (R>180, G<80, B<80) : boîte dense d'environ 85–110 px de large et 270–310 px de haut, centre dans la **moitié droite et la moitié basse** de la feuille (la page paysage a été pivotée de 90° anti-horaire) — c'est la preuve de bout en bout de l'orientation par page ;
   - aucun pixel coloré collé au bord de la feuille (zone imprimable respectée).
8. Le test tourne sous le verrou `SERIAL` du moteur ; les noms de fichiers incluent le PID pour supporter deux `cargo test` simultanés.

### 8.3 Test du worker (`apps/sunipdf-desktop/src/worker.rs`, module `#[cfg(test)]`)

`Worker::spawn(egui::Context::default())` ; attendre `Response::EngineReady` ; `Request::Open` de la fixture ; `Request::Print(PrintJob{ options: Print to PDF → fichier temporaire, cancel })` ; vérifier la séquence `PrintProgress`… puis `PrintFinished(Ok(Completed{pages:2}))` ; cas sans document → `PrintFinished(Err(_))` (et non un blocage) ; cas d'annulation (drapeau posé avant l'envoi) → `Cancelled`. Même règle de saut que ci-dessus. Aucun autre moteur n'est créé dans ce binaire de test.

### 8.4 Scénarios manuels avec `scripts/dev-screenshot.ps1` (paramètres `-Pdf`, `-Commands`, `-Delay`, `-KeysAfter`, `-Keys` ; `-Drag` sans objet ici)

Hook d'automatisation (à implémenter dans `printing.rs`, documenté dans `docs/PROJECT_STATUS.md` comme `SUNIPDF_STARTUP_COMMANDS`) : si `SUNIPDF_PRINT_DEVICE` est défini, `Command::Print` saute les deux boîtes et imprime toutes les pages vers cette imprimante, avec `SUNIPDF_PRINT_OUTPUT` comme fichier de sortie facultatif et les réglages enregistrés. Ce n'est pas une fonction utilisateur.

```powershell
. .\scripts\dev-env.ps1
cargo build -p sunipdf-desktop -p sunipdf-cli -j 2
$f = "crates\sunipdf-core\tests\fixtures\two-pages.pdf"

# 1. boîte « Imprimer » (réglages)
.\scripts\dev-screenshot.ps1 -Out print-options.png -Pdf $f -Commands "file.print" -Delay 5

# 2. feuille Windows native (Entrée valide « Suivant… » ; la feuille est détenue par la fenêtre, donc dans le cadre capturé)
.\scripts\dev-screenshot.ps1 -Out print-native.png -Pdf $f -Commands "file.print" -KeysAfter "{ENTER}" -Delay 5

# 3. progression : document de 80 pages, impression automatique vers Print to PDF
$env:SUNIPDF_PRINT_DEVICE = "Microsoft Print to PDF"; $env:SUNIPDF_PRINT_OUTPUT = "$env:TEMP\sunipdf-shot.pdf"
.\target\debug\sunipdf-cli.exe merge (1..40 | ForEach-Object { $f }) --out "$env:TEMP\big.pdf"
.\scripts\dev-screenshot.ps1 -Out print-progress.png -Pdf "$env:TEMP\big.pdf" -Commands "file.print" -Delay 6

# 4. annulation : Échap dans la boîte de progression
.\scripts\dev-screenshot.ps1 -Out print-cancelled.png -Pdf "$env:TEMP\big.pdf" -Commands "file.print" -Delay 6 -KeysAfter "{ESC}"

# 5. fin normale : barre d'état « Impression terminée »
.\scripts\dev-screenshot.ps1 -Out print-done.png -Pdf $f -Commands "file.print" -Delay 15
.\target\debug\sunipdf-cli.exe info "$env:TEMP\sunipdf-shot.pdf"   # page_count attendu : 2
Remove-Item Env:SUNIPDF_PRINT_DEVICE, Env:SUNIPDF_PRINT_OUTPUT
```

Vérification visuelle à consigner : thème clair et sombre, FR et EN, icône (glyphe `E749`), barre de progression, message d'état. Les boîtes natives ne sont pas thémées par l'application (normal).

---

## 9. Nouvelles dépendances

**Recommandé : `windows-sys` 0.61.2 (licence MIT OR Apache-2.0), dépendance `cfg(windows)` de `sunipdf-core`, déjà présente dans `Cargo.lock`** (aucun téléchargement : seule une arête est ajoutée).

```toml
# Cargo.toml (racine, [workspace.dependencies])
windows-sys = "0.61"

# crates/sunipdf-core/Cargo.toml
[target.'cfg(windows)'.dependencies]
windows-sys = { workspace = true, features = [
    "Win32_Foundation",                # HWND, HGLOBAL, GlobalFree, GetLastError
    "Win32_Graphics_Gdi",              # CreateDCW, GetDeviceCaps, StretchDIBits, DEVMODEW, BITMAPINFO…
    "Win32_Graphics_Printing",         # GetDefaultPrinterW
    "Win32_Storage_Xps",               # StartDocW, StartPage, EndPage, EndDoc, AbortDoc, DOCINFOW
    "Win32_System_Memory",             # GlobalLock, GlobalUnlock, GlobalSize
    "Win32_UI_Controls_Dialogs",       # PrintDlgExW, PRINTDLGEXW, DEVNAMES
    "Win32_UI_Input_KeyboardAndMouse", # GetActiveWindow
] }
```

Dépendances de features vérifiées : `Win32_Storage_Xps = ["Win32_Storage"]`, `Win32_UI_Controls_Dialogs = ["Win32_UI_Controls"]`, `Win32_Graphics_Gdi = ["Win32_Graphics"]` (`windows-sys-0.61.2/Cargo.toml:90, 94, 157, 177, 222, 261-262, 266`). Les liaisons utilisent `windows_link::link!` (raw-dylib, aucune bibliothèque d'import à installer) ; avec le repli GNU de `scripts/dev-env.ps1` il faut `dlltool` (WinLibs complet), comme pour les autres crates raw-dylib (`docs/KNOWN_ISSUES.md` n°3).

**Pourquoi pas la crate `windows` 0.62.2 (alternative étudiée, signatures vérifiées dans le tableau §3.4)** : elle fonctionnerait, mais le coût de compilation est très différent. `Win32_Storage_Xps` ajoute `W/Storage/Xps/mod.rs` = **12 983 lignes, 68 interfaces COM** (`define_interface!`) et `Win32_Graphics_Printing` ajoute `W/Graphics/Printing/mod.rs` = **14 010 lignes, 100 interfaces COM**, alors que `Win32_System_Com` est déjà actif (via `accesskit_windows`), donc ces interfaces seraient compilées en entier ; `Win32_UI_Controls` ajoute encore 11 749 lignes. Les mêmes modules dans `windows-sys` : `Storage/Xps` 440 lignes, `Graphics/Printing` 5 024, `UI/Controls` 8 163 (déclarations `extern` et structures simples, sans vtables). Le coût d'une modification de features est un recompilage unique de la crate concernée et de ses dépendants : `windows` → `accesskit_windows` → `accesskit_winit` → `egui-winit`/`eframe` ; `windows-sys` 0.61.2 → les 13 crates listées au §0. **Aucune mesure n'a été faite** (cargo interdit) : à l'étape 1, mesurer `cargo build -p sunipdf-core --timings -j 2` avant/après et consigner le résultat dans `docs/KNOWN_ISSUES.md`. Si la mesure surprend, repli = `windows` 0.62.2 (`default-features = false`, mêmes noms de features : `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_Graphics_Printing`, `Win32_Storage_Xps`, `Win32_System_Memory`, `Win32_UI_Controls_Dialogs`, `Win32_UI_Input_KeyboardAndMouse`) ; la différence de code est mécanique (`Result<()>` au lieu de `HRESULT`, `Param<PCWSTR>`, newtypes `HDC(…)`).

Option écartée : déclarations FFI manuelles (zéro dépendance) — risque d'erreur de disposition de `PRINTDLGEXW`/`DEVMODEW` non compensé par un gain réel.

Aucune autre dépendance : `std::ops::ControlFlow`, `core::mem::offset_of!` (Rust ≥ 1.77, le workspace exige 1.85), `egui::ProgressBar` (déjà dans egui 0.36). Pas de nouvelle ressource embarquée ; `comdlg32.dll`, `gdi32.dll`, `winspool.drv` sont des DLL système (rien à ajouter au MSI ni au ZIP portable).

---

## 10. Risques, pièges, ordre d'implémentation, estimation

### Prérequis et conflits de fusion

Aucun prérequis dur. Recommandé : faire atterrir **`tool-hub` en premier** (il remplace `ToolTab` et ajoute le test `every_registry_command_has_a_home`) ; sinon appliquer le cas A de §5.1 puis migrer vers le cas B au rebase. Les tranches `continuous-view` (modifie `document_area`), `compress-lossy`, `sign-image`, `bookmarks-links` ajoutent aussi des commandes : faire un commit court par étape et rebaser fréquemment (`command.rs`, `app.rs::execute`, `handle_input`, `handle_responses`, `ui()`).

### Risques et pièges connus

1. **`lpszOutput` avec *Microsoft Print to PDF*** (⚠ non exécuté ici) : si la boîte « Enregistrer la sortie » apparaît quand même, le test se bloque. Première validation à l'étape 3, **à la main** (`cargo test -p sunipdf-core --test engine print_to_pdf -- --nocapture` sous surveillance). Le fichier peut apparaître **après** `EndDoc` (traitement par le spouleur) : d'où `wait_for_output`.
2. **`FillRect`/`RenderPageBitmap` avec un bitmap plus petit que la page** (⚠) : documenté par `render_config.rs:667-680` mais jamais exécuté ici ; test n°1 de §8.2. Si PDFium ne rogne pas : rendre la page entière à `div` plus grand (dégradation de qualité) ou fournir un bitmap complet puis découper (mémoire).
3. **Couture entre bandes** : évitée par construction (diviseurs entiers, §3.3 point 6). Vérifier visuellement sur une imprimante 600 ppp à 300 ppp (diviseur 2).
4. **Taille du spool** (L1) : 26 Mo/page à 300 ppp, 105 Mo à 600 ppp ; un document de 100 pages en « Haute » peut produire ~10 Go de spool temporaire. Défaut volontairement à 300 ppp ; message à ajouter dans le guide utilisateur. Piste : `StretchDIBits` accepte `BI_JPEG`/`BI_PNG` si le pilote répond au code d'échappement `CHECKJPEGFORMAT`/`CHECKPNGFORMAT` (doc `StretchDIBits`, web) — non implémenté.
5. **Mémoire** : pic ≈ 2 × `BAND_BYTES` (bitmap PDFium + copie `as_raw_bytes`) soit ~64 Mo, plus les images décodées de la page. Contrainte du poste (14 Go, `-j 2`) respectée.
6. **`PrintDlgExW` sur le thread UI** : fenêtre propriétaire obligatoire (`GetActiveWindow` ; en automatisation sans fenêtre active, l'appel échoue en `E_HANDLE` → `error.print_failed`, d'où le hook d'environnement). La boucle de messages modale réentre dans le gestionnaire d'événements de winit : même situation que `rfd` aujourd'hui. Après retour, un état de touche (Ctrl) peut rester enfoncé côté egui, comme après les boîtes de fichiers existantes.
7. **Structure `PRINTDLGEXW` dynamique**, blocs `hDevMode`/`hDevNames` à libérer, `hDC` à supprimer : fuite de handles sinon.
8. **Sens de rotation** : `Degrees270` = 90° anti-horaire (`as_pdfium()` = 3 ; doc de `FPDF_RenderPage` dans `PR/bindings.rs` : « 3 = 90° anti-horaire ») ; convention choisie, figée par le test n°2 ; à confirmer visuellement sur le PDF produit.
9. **Retour 0 de `StretchDIBits`** : traité comme échec (doc : « échec ou aucune ligne copiée »). Si un pilote renvoie 0 à tort, la validation sur *Print to PDF* le révélera ; ne pas relâcher le contrôle sans preuve.
10. **Pilotes qui ignorent `STRETCH_HALFTONE`** (PostScript) : sans conséquence ; si le halftone ralentit sur un pilote, utiliser `COLORONCOLOR` quand `div == 1`.
11. **Rotation de page `/Rotate`** : `page_size` renvoie les dimensions après rotation de page (comportement PDFium de `page_sizes()`, déjà utilisé par la vue) ; ajouter un cas de test avec `set_page_rotation` si une régression est suspectée.
12. **CI `windows-latest`** : l'imprimante peut manquer ; test conçu pour sauter. Si la CI se bloque, définir `SUNIPDF_SKIP_PRINT_TESTS=1` dans `ci.yml`.
13. **Premier `unsafe` du crate** (`print/win.rs`) : `cargo clippy --workspace --all-targets -- -D warnings` doit rester vert ; garder chaque bloc `unsafe` minimal, avec `// SAFETY:`.

### Ordre d'implémentation (chaque étape compile, passe les tests et ne livre aucune commande tant qu'elle ne fonctionne pas)

1. **Dépendance et erreurs** : `windows-sys` (workspace + `sunipdf-core`), mesure `--timings`, `EngineError::{PrinterUnavailable, PrintFailed}` + `code()`, `app.rs::error_key` + son test, clés `error.printer_unavailable`/`error.print_failed` dans les deux catalogues, `check-i18n.ps1`.
2. **`print/mod.rs` indépendant de la plateforme** : types, `plan_page`, `bands`, `expand_ranges`, `render_strip` + tests unitaires et tests d'intégration n°1 et n°2.
3. **`print/win.rs` : `open_dc`, géométrie, `Document::print`, `wait_for_output`** + tests n°3, n°4, n°5 (premier essai surveillé, voir risque 1). Rapporter la sortie réelle (PDF produit, nombre de pages, durée).
4. **CLI `print`** + essai réel sur la fixture (voir §6).
5. **`show_print_dialog`, `default_printer`** + tests unitaires `DEVNAMES`/offsets ; essai manuel (capture `print-native.png`).
6. **Worker + interface** : `Request::Print`/`Response::*`, `printing.rs` (état, boîte de progression, hook d'environnement), `Command::Print` (registre, test `== 8`), raccourci, menus, icône, clés i18n restantes, test du worker (§8.3). Boîte « Imprimer » réduite à un `start_print` direct avec réglages par défaut tant que l'étape 7 n'est pas faite.
7. **Boîte de réglages** (échelle, rotation, qualité) + champs `Settings` + test `printing.rs` des clés.
8. **Documentation et finitions** (§11), puis `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace -j 2`, `.\scripts\check-i18n.ps1`, scénarios `dev-screenshot` (§8.4) ; rapporter les sorties réelles.

### Estimation : **L** (≈ 1,5 à 2 jours) — dont ~40 % pour la partie Win32 (`print/win.rs`) et sa validation sur *Print to PDF*, ~25 % pour l'interface et le worker, ~35 % pour les tests et la documentation. Le risque dominant est l'inconnue des étapes 2-3 (rognage du bitmap, comportement de `lpszOutput`).

Pistes ultérieures (hors tranche) : mémoriser imprimante et DEVMODE ; compression `BI_JPEG` pour réduire le spool ; impression vectorielle via `FPDF_RenderPage` (nécessite l'accès à `FPDF_PAGE`, donc un correctif amont de `pdfium-render` ou une FFI maison) ; `ResetDCW` pour l'orientation par page côté pilote ; « Sélection » depuis la vue Organiser ; plusieurs pages par feuille ; ARM64 à compiler.

---

## 11. Documentation à mettre à jour

- `docs/PROJECT_STATUS.md` : case « SUNI-0xx Impression » avec **résultats réels** (commandes exécutées, nombre de tests, PDF produit par *Print to PDF*, captures) ; retirer « impression » de « Non réalisé » ; décrire le hook `SUNIPDF_PRINT_DEVICE`/`SUNIPDF_PRINT_OUTPUT` ; mettre à jour le nombre de commandes et de tests ; journal daté.
- `docs/NEXT_TASKS.md` : ajouter les pistes de la fin du §10.
- `docs/FEATURE_MATRIX.md` ligne 28 « Impression Windows » : « **Fait** (boîte Windows native, rendu raster par bandes, échelle, orientation par page, annulation ; imprimantes virtuelles testées, pas de vectoriel) ».
- `docs/ROADMAP.md` : impression passée de P5 à livrée, avec les pistes ci-dessus.
- `docs/DECISIONS.md` : nouvelle ligne (numéro libre suivant D020 au moment de l'implémentation) — « Impression Windows par GDI : rendu raster par bandes PDFium (`set_origin` négatif) puis `StretchDIBits`, boîte `PrintDlgExW`, `windows-sys` plutôt que `windows`, module `print` dans `sunipdf-core` (pas de crate `sunipdf-platform` pour l'instant) ; `ShellExecute print` rejeté ; vectoriel reporté (handle de page non public) ».
- `docs/KNOWN_ISSUES.md` : limites L1–L7 (spool volumineux, PDF-image, pas d'imprimante mémorisée, droit `/P` conservateur, pas de test sur matériel, ARM64 non compilé) + résultat de la mesure de compilation de l'étape 1.
- `docs/ARCHITECTURE.md` : module `print` dans « Workspace actuel », flux « imprimer → boîte native → worker → GDI », et note sur la crate `sunipdf-platform` non créée.
- `docs/THIRD_PARTY_NOTICES.md` : `windows-sys` 0.61.2 (MIT OR Apache-2.0), dépendance directe de `sunipdf-core`.
- `docs/USER_GUIDE.fr.md` (ligne ~125 « pas d'impression directe ») et `docs/USER_GUIDE.en.md` (ligne ~119) : décrire Ctrl+P, les réglages, la remarque sur le spool et le PDF-image avec *Microsoft Print to PDF*.
- `CHANGELOG.md` (section « Non publié › Ajouté ») et `CLAUDE.md` (liste des commandes : mention de `sunipdf-cli print`, du module `print`, et du hook de capture).
- `.github/workflows/ci.yml` : rien d'obligatoire ; ajouter `SUNIPDF_SKIP_PRINT_TESTS: 1` seulement si les tests d'impression posent problème sur le runner.
