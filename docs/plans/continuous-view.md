# Plan d'implémentation — Vue continue, zoom ancré et rendu en tuiles (SUNI-012b)

> Statut : **plan, non exécuté**. Rédigé le 2026-10-14 en lecture seule : aucune commande `cargo` n'a été lancée,
> aucune API n'a été compilée ni testée. Chaque appel de bibliothèque ci-dessous a été relu dans les sources locales
> (`C:\Users\Emanuel\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\...`) et cité avec fichier:ligne.
> Aucun chiffre de performance n'est annoncé : la section 8.5 dit quoi mesurer, avec quelle commande.
> Estimation globale : **L**. Nouvelles dépendances : **aucune**. Nouvelle boîte de dialogue : **aucune**.

Sommaire : 1 Objectif · 2 Faisabilité · 3 API moteur · 4 Worker · 5 Interface · 6 CLI · 7 i18n · 8 Tests · 9 Dépendances · 10 Risques, étapes, estimation · 11 Documentation

---

## 1. Objectif et comportement visible

### 1.1 Ce que l'utilisateur voit

1. **Défilement continu** : toutes les pages sont empilées verticalement dans une seule zone de défilement
   (marge de 16 px, intervalle de 12 px entre pages, ombre portée comme aujourd'hui), pages centrées, largeur de
   contenu = page la plus large. Le mode **Page unique** actuel reste disponible (menu Affichage, bouton du rail
   droit, `F5`). Le mode choisi est mémorisé (`settings.json`, champ `continuous_view`, **continu par défaut**).
2. **Virtualisation** : seules les pages qui coupent la zone visible (plus une marge de pré-chargement) créent des
   widgets, des tuiles à rendre et des textures. Un document de 500 pages ou plus n'occupe qu'un nombre borné de
   textures GPU (budget en octets, éviction LRU) et un cache moteur borné (256 Mio, déjà en place).
3. **Page courante suivie au défilement** : le champ « page », la barre d'état (« Page x / n »), la miniature
   surlignée, le panneau des commentaires et les actions « Pivoter / Supprimer / Extraire la page » (qui visent
   `doc.page`) suivent la page qui occupe le plus de surface visible. « Aller à la page » (champ, miniature, signet,
   Origine / Fin, Page préc. / suiv., résultat de recherche) fait défiler jusqu'au haut de la page cible et
   *verrouille* la page courante tant que l'utilisateur ne défile pas lui-même.
4. **Modes de zoom** : *Ajuster à la largeur*, *Ajuster à la page*, pourcentage (liste existante étendue à 1600 %,
   Ctrl + molette jusqu'à 3200 %). **Le zoom est ancré** : Ctrl + molette (et pincement) zoome autour du pointeur ;
   les boutons, Ctrl +/- et la liste zooment autour du centre de la zone visible ; un redimensionnement de la
   fenêtre en mode « largeur » conserve la page et la fraction de page affichées en haut. Pendant un zoom,
   les textures de l'ancienne échelle restent affichées (étirées) jusqu'à l'arrivée des nouvelles : pas de page
   blanche, netteté rétablie 150 ms après la fin du geste.
5. **Clavier** (mode continu) : `Page suiv.` / `Page préc.` / `Espace` / `Maj+Espace` défilent d'un écran (90 %),
   `Flèche haut/bas` d'une ligne (48 px), `Flèche droite/gauche` = page suivante/précédente (comme aujourd'hui),
   `Origine` / `Fin` = première / dernière page. En page unique rien ne change, sauf `Flèche haut/bas` qui défilent.
6. **Fort zoom sans plafond de 16 384 px** : au-delà de 4096 px de côté ou 8 Mpx pour la page rendue, la page est
   découpée en tuiles de 1024 px rendues à la demande. **Cela corrige aussi un plantage latent** : le painter
   `egui_glow` fait `assert!(w <= max_texture_side && h <= max_texture_side)`
   (`egui_glow-0.36.2/src/painter.rs:553`) ; aujourd'hui une page A4 à 600 % sur un écran à 150 % (environ 7140 x 10100 px,
   rendue d'un seul tenant par `render_page`) **panique dès que le GPU / pilote limite `GL_MAX_TEXTURE_SIZE` à 8192 px**
   (limite lue en `painter.rs:167`, transmise à egui par `eframe-0.36.2/src/native/glow_integration.rs:274-278`).
   Non reproduit ici (pas de GPU à limite basse sous la main) : déduit de la lecture du code.
7. **Toutes les interactions restent par page** : sélection de texte, copie, surlignage / soulignement / barré,
   notes, sélection d'objet, déplacement, redimensionnement par poignées, dessin de formes, ajout de texte et
   d'image, surbrillance des résultats de recherche, curseur contextuel — sur n'importe quelle page visible.
   Le garant : le mode « page unique » devient un cas particulier de la même machinerie (fenêtre d'une seule page),
   donc **une seule fonction `paint_slot` + un seul `handle_page_event`** servent les deux modes.

### 1.2 Correspondance avec PDFCraft

La liste « All tools » de PDFCraft n'a pas d'entrée « vue continue » : c'est la **surface de lecture commune**
de *Edit a PDF*, *Add comments*, *Add a stamp*, *Organize pages* (miniatures, page courante), et le socle sur
lequel *Fill & Sign*, *Redact a PDF* et *Prepare a form* (tranches futures) travailleront page par page.
Le comportement (défilement continu, zoom sur le pointeur, page courante suivie) est celui attendu d'un lecteur
PDF de bureau ; il n'est pas copié de PDFCraft (D003, D006).

---

## 2. Faisabilité

**Verdict : oui**, avec le périmètre borné ci-dessous. Tout repose sur des API déjà présentes dans le dépôt ou lues
dans les sources locales. Une seule hypothèse dépend du comportement interne de PDFium (rendu d'une région dans un
petit bitmap à très grande échelle) : elle est couverte par des tests dédiés (T-REG-1/3/7, section 8.1) et a un plan B.

### 2.1 Volontairement hors périmètre (à écrire dans `KNOWN_ISSUES.md`)

| Élément | Raison | Où le noter |
|---|---|---|
| Sélection de texte **à cheval sur plusieurs pages** | `TextInRect` / `TextSelection` sont par page ; le glissement reste clampé à la page d'origine (`annotate::to_pdf` clampe y à `[0, page_h]`, `annotate.rs:97-102`). Un modèle de sélection multi-pages est une tranche à part. | KNOWN_ISSUES A11 |
| Défilement automatique quand on glisse près du bord | egui n'en fournit pas ; va avec la sélection multi-pages | A11 |
| Défilement animé / inertie | on utilise `ScrollArea::scroll_offset` (instantané) ; l'animation d'egui complique le verrou de page courante | A11 |
| Double page, mode livre, rotation de la vue | restent « planifié » (FEATURE_MATRIX l.19) | FEATURE_MATRIX |
| Outil « Main » dédié | glisser sur le fond gris fait déjà défiler (`ScrollSource` par défaut, `scroll_area.rs:407`) | — |
| Pincement tactile | le chemin `InputState::zoom_delta()` est identique à Ctrl+molette, mais **aucun écran tactile n'est disponible pour le tester** : non vérifié | A11 |
| Annulation d'un rendu PDFium **déjà commencé** | un appel `FPDF_RenderPageBitmap` ne s'interrompt pas ; l'annulation se fait par *remplacement de la liste de tuiles voulues* ; latence = durée d'une tuile | A11 |
| Rendu progressif multi-passes | remplacé par l'affichage étiré des textures d'une autre échelle (repli) | — |
| Crate `sunipdf-render` | tuiles et cache restent dans `sunipdf-core::{tiles, cache}` : le rendu est une méthode de `Document` (champ `inner` privé). À créer si un second consommateur apparaît ; création à valider par *architect* | DECISIONS D021 |
| « Ajuster à la page » en continu sur pages de tailles mixtes | le zoom est uniforme pour tout le document : on ajuste la **plus grande** page (largeur et hauteur max), sinon le zoom changerait en défilant | A12 |
| Précision au-delà de ~8 millions de px logiques de hauteur totale | positions en `f32` ; le zoom est plafonné pour que la hauteur totale reste <= 8 000 000 | A13 |
| Chiffres de performance | non mesurés ; protocole en 8.5 | PROJECT_STATUS (machine + commande) |

### 2.2 Hypothèses à vérifier à l'implémentation (et plan B)

| # | Hypothèse | Vérification | Plan B si fausse |
|---|---|---|---|
| H1 | PDFium rend correctement une région dans un petit bitmap avec `set_origin(-x, -y)` et une taille cible énorme (jusqu'à 131 072 px) | T-REG-1, T-REG-3, T-REG-7 | abaisser `MAX_TILED_SIDE_PX` ; si le *clear* (`FPDFBitmap_FillRect` à coordonnées négatives, `page.rs:642`) n'efface pas (pixels alpha = 0), passer `.clear_before_rendering(false)` et composer sur blanc côté Rust |
| H2 | Des tuiles alignées sur des pixels entiers se raccordent sans couture visible | T-REG-7 (moteur) + capture S4 (UI) | dessiner les tuiles avec un recouvrement de 0,5 px |
| H3 | `ScrollArea::scroll_offset` est appliqué avant la construction du `viewport` de la même frame | lecture `scroll_area.rs:742-743` puis `:828` ; capture S2 | utiliser `ScrollAreaOutput.state.offset` et attendre une frame |
| H4 | Cliquer sur une page ne laisse pas le focus clavier au widget de page (sinon `handle_input` n'applique plus que les raccourcis Ctrl, `app.rs:599-633`) | lecture : egui ne donne le focus que sur `request_focus` explicite (`response.rs:377`, `memory/mod.rs:918`) et le retire aux appuis (`context.rs:1553-1556`) ; capture S9 | utiliser `Sense::CLICK \| Sense::DRAG` (non focalisable, `sense.rs`) au lieu de `click_and_drag()` |

---

## 3. API moteur (`crates/sunipdf-core`)

Le rendu est en lecture seule (`&self`) : **aucune implication d'instantané, d'annulation ni de
`regenerate_content`**. Il rend l'état *en mémoire* (y compris les modifications non enregistrées, comme
`render_page`, `lib.rs:224-249`) ; les tuiles doivent donc être jetées après toute édition, annulation, rétablissement
ou restauration — c'est déjà le rôle de `State::invalidate()` (`worker.rs:346`) côté worker et de l'époque (`epoch`)
côté UI (section 4.3).

### 3.1 Nouveau module `src/tiles.rs` (pur, sans appel PDFium)

Déclaration : `pub mod tiles;` dans `lib.rs` (après `pub mod text;`, l.19) et ré-exports
`pub use tiles::{PixelRect, TileJob, TileLimits};` (après l.28). Utilisable par l'UI (qui n'importe jamais
`pdfium-render`), la CLI et les tests.

```rust
//! Géométrie des tuiles de rendu. Pur : aucun appel PDFium.
use crate::PageSize;

/// Côté d'une tuile (pixels) pour une page découpée.
pub const TILE_SIDE_PX: u32 = 1024;
/// Une page est rendue d'un seul tenant si ses deux côtés sont <= à ceci...
pub const WHOLE_PAGE_MAX_SIDE_PX: u32 = 4096;
/// ...et si son aire est <= à ceci (8 Mpx = 32 Mio en RGBA).
pub const WHOLE_PAGE_MAX_PIXELS: u32 = 8 * 1024 * 1024;
/// Plafond du côté de la page *entière* à l'échelle demandée (jamais allouée en entier).
pub const MAX_TILED_SIDE_PX: u32 = 131_072;
/// Plafond de l'aire d'une région rendue (100 Mpx = 400 Mo en RGBA).
pub const MAX_REGION_PIXELS: u64 = 100_000_000;

/// Rectangle en pixels dans la page rendue à une échelle donnée, origine en haut à gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self;
    pub fn right(&self) -> u32;            // x + width
    pub fn bottom(&self) -> u32;           // y + height
    pub fn is_empty(&self) -> bool;        // width == 0 || height == 0
    pub fn intersection(&self, other: &PixelRect) -> Option<PixelRect>;
}

/// Une tuile à rendre : clé stable (page, échelle en millièmes, région). `Eq + Hash` : clé de cache et de dédoublonnage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileJob { pub page: usize, pub scale_milli: u32, pub region: PixelRect }
impl TileJob { pub fn scale(&self) -> f32 { self.scale_milli as f32 / 1000.0 } }

/// Seuils de découpage, bornés par la taille de texture maximale du GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileLimits { pub tile_side: u32, pub whole_max_side: u32, pub whole_max_pixels: u32 }
impl TileLimits {
    /// `side = max_texture_side.clamp(256, 16384)` ; tile_side = min(1024, side) ; whole_max_side = min(4096, side).
    pub fn for_texture_side(max_texture_side: usize) -> Self;
}

/// Échelle arrondie au 1/200 (0,005), en millièmes : `((scale * 200).round().max(1)) * 5`.
/// Remplace `app.rs::quantize` (l.1920) ; `quantize_milli(1.3333) == 1335`.
pub fn quantize_milli(scale: f32) -> u32;

/// Taille en pixels de la page entière à `scale` ; MÊME arrondi que `render_page` (`lib.rs:229-230`).
/// `None` si l'échelle n'est pas finie/positive ou si un côté vaut 0 ou dépasse MAX_TILED_SIDE_PX.
pub fn page_pixels(size: PageSize, scale: f32) -> Option<(u32, u32)>;

/// Vrai si la page se rend d'un seul tenant (une seule « tuile » = la page entière).
pub fn is_whole(page_px: (u32, u32), limits: &TileLimits) -> bool;

/// Tuiles qui coupent `view` (pixels de la page). Page « entière » : `[page entière]` si `view` la coupe.
/// Sinon grille alignée sur des multiples de `tile_side` ancrés en (0, 0) — indépendante de `view`, donc
/// stable pendant le défilement (clés de cache identiques) ; tuiles de bord tronquées à la page.
pub fn plan_tiles(page_px: (u32, u32), view: PixelRect, limits: &TileLimits) -> Vec<PixelRect>;
```

Algorithme de `plan_tiles` : `view = view ∩ page` (sinon `vec![]`) ; si `is_whole` → `vec![page]` ; sinon
`tx0 = view.x / t`, `tx1 = (view.right() - 1) / t`, idem en y, et pour chaque `(tx, ty)` la tuile
`PixelRect::new(tx*t, ty*t, t.min(w - tx*t), t.min(h - ty*t))`.

### 3.2 `Document::render_region` (dans `lib.rs`, à côté de `render_page`)

```rust
/// Rend la zone `region` (pixels, origine en haut à gauche de la page rendue à `scale`) sans allouer la page entière.
pub fn render_region(&self, page_index: usize, scale: f32, region: PixelRect)
    -> Result<RenderedPage, EngineError>
{
    let size = self.page_size(page_index)?;                                   // PageOutOfRange
    let (page_w, page_h) = tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
    if region.is_empty() || region.right() > page_w || region.bottom() > page_h {
        return Err(EngineError::Other("render region is empty or outside the page".into()));
    }
    if region.width > MAX_RENDER_SIDE_PX || region.height > MAX_RENDER_SIDE_PX
        || u64::from(region.width) * u64::from(region.height) > tiles::MAX_REGION_PIXELS {
        return Err(EngineError::RenderTooLarge);
    }
    let index = PdfPageIndex::try_from(page_index).map_err(|_| EngineError::PageOutOfRange(page_index))?;
    let page = self.inner.pages().get(index)?;
    let config = PdfRenderConfig::new()
        .set_target_size(page_w as Pixels, page_h as Pixels)                  // taille de la PAGE ENTIÈRE
        .set_origin(-(region.x as Pixels), -(region.y as Pixels))             // la région tombe en (0, 0) du bitmap
        .render_form_data(true)
        .render_annotations(true);
    let mut bitmap = PdfBitmap::empty(region.width as Pixels, region.height as Pixels, PdfBitmapFormat::default())?;
    page.render_into_bitmap_with_config(&mut bitmap, &config)?;
    Ok(RenderedPage {
        page_index,
        width: bitmap.width() as u32,
        height: bitmap.height() as u32,
        rgba: bitmap.as_rgba_bytes(),
    })
}
```

`render_page` reste **inchangé** (CLI, export d'images `convert.rs:72`, tests existants) : pas de régression possible.
Erreurs : `PageOutOfRange` (page), `RenderTooLarge` (échelle invalide, page entière > 131 072 px, région trop grande),
`Other` (région vide ou hors page : erreur de programmation de l'appelant ; pas de nouvelle variante pour ne pas toucher
`error_key`/le catalogue), erreurs PDFium via `From<PdfiumError>` (`error.rs`).

**Appels de bibliothèque et sources** (pdfium-render 0.9.4) :

| Appel | Source |
|---|---|
| `PdfRenderConfig::new()` | `src/pdf/document/page/render_config.rs:76` |
| `set_target_size` → `set_target_width` / `set_target_height` (auto-échelle, la taille de sortie vaut `round(source * cible/source)`) | `render_config.rs:187`, `:197`, `:210` ; calcul `:719-805` |
| `set_origin(left, top)` : position du coin haut-gauche de la page dans le bitmap de destination ; la doc précise (traduite) qu'un bitmap de la taille d'une bande combiné à un `y` négatif sert à rendre une bande d'une grande page sans la rendre en entier | `render_config.rs:664-684` ; copié dans `PdfPageRenderSettings.start_x/start_y` `:900-901` |
| `render_form_data(true)` / `render_annotations(true)` (comme `render_page`, `lib.rs:239-240`) ; reste sur la branche `FPDF_RenderPageBitmap` (pas de matrice ni de `clip`, sinon les formulaires seraient désactivés) | `render_config.rs:393`, `:402` ; branches `page.rs:653-700` vs `:702-712` |
| `PdfBitmap::empty(w, h, format)` ; format par défaut BGRA | `src/pdf/bitmap.rs:132`, `:106-111` ; `type Pixels = i32` `:55` |
| `PdfPage::render_into_bitmap_with_config` → `render_into_bitmap_with_settings` : `FPDFBitmap_FillRect(start_x, start_y, width, height)` `:642`, `FPDF_RenderPageBitmap(start_x, start_y, width, height, …)` `:658`, `FPDF_FFLDraw` `:689` | `src/pdf/document/page.rs:620`, `:631` |
| `PdfBitmap::width/height/as_rgba_bytes` (inverse l'ordre R/B si le drapeau `REVERSE_BYTE_ORDER` est posé, ce qui est le défaut `render_config.rs:115`) | `bitmap.rs:264`, `:270`, `:294` |
| `Document::page_size` / tailles de pages | `lib.rs:215` ; `PdfPages::page_sizes` `pages.rs:180` (tailles *après* rotation) |

### 3.3 `src/cache.rs` : clé de tuile dans le même LRU

```rust
pub type CacheKey = (usize, u32, Option<PixelRect>);              // (page, échelle en millièmes, région)
pub fn cache_key(page: usize, scale: f32) -> CacheKey { (page, (scale * 1000.0).round() as u32, None) }
pub fn tile_key(job: &TileJob) -> CacheKey { (job.page, job.scale_milli, Some(job.region)) }

impl RenderCache {
    pub fn contains_tile(&self, job: &TileJob) -> bool;
    pub fn get_tile(&mut self, job: &TileJob) -> Option<Arc<RenderedPage>>;
    pub fn insert_tile(&mut self, job: TileJob, image: RenderedPage) -> Arc<RenderedPage>;
}
```
`insert` (pages entières / miniatures) et `insert_tile` partagent un `insert_keyed(key, page)` privé : même comptage
d'octets (`rgba.len()`), même éviction LRU, même règle « l'entrée insérée n'est jamais évincée ». Les tests existants
(`cache.rs:90-130`) sont adaptés à la nouvelle forme de clé (`(3, 1333, None)`). Le plafond reste `CACHE_BYTES`
(256 Mio, `worker.rs:16`), partagé miniatures + tuiles : une tuile 1024² = 4 Mio, une page « entière » <= 32 Mio.

### 3.4 Sources egui utilisées par la partie interface (vérifiées dans `egui-0.36.2`)

| API | Source |
|---|---|
| `ScrollArea::both().id_salt(..).auto_shrink(..)` | `containers/scroll_area.rs:381`, `:482`, `:605` |
| `ScrollArea::scroll_offset(Vec2)` : positionne le décalage de **cette frame** | `:495` ; appliqué dans `begin()` `:742-743` ; `viewport` construit ensuite `:828` |
| `ScrollArea::show_viewport(ui, \|ui, viewport\| ..)` : `viewport` = rectangle visible en coordonnées de contenu | `:1020` |
| `ScrollAreaOutput { state, inner_rect, content_size }`, `State.offset` publique | `:89-105`, `:28` |
| barres de défilement flottantes par défaut (aucune largeur réservée : le « fit largeur » n'a pas à retrancher la barre) | `style.rs:585-589`, `:656-662` |
| `InputState::zoom_delta()` (Ctrl+molette *et* pincement) ; avec Ctrl, `smooth_scroll_delta` est nul : la molette ne défile pas | `input_state/mod.rs:562`, `:116` (zoom_modifier = COMMAND), `:459-466` |
| `InputState::max_texture_side` | `input_state/mod.rs:268` |
| `Context::load_texture` (`debug_assert` sur la taille) ; `TextureHandle` libère la texture au `Drop` | `context.rs:2390-2410` ; `epaint-0.36.2/src/texture_handle.rs:25-29` |
| `Painter::image(texture_id, rect, uv, tint)` | `painter.rs:447` |
| `Ui::interact(rect, id, sense)`, `allocate_exact_size`, `put`, `clip_rect` | `ui.rs:907`, `:1151`, `:1566`, `:640` |
| `Context::request_repaint_after(Duration)` ; `InputState.time` | `context.rs:1872` ; `input_state/mod.rs:271` |
| `PointerState::hover_pos / latest_pos / press_origin / any_down` | `input_state/mod.rs:1316`, `:1310`, `:1291`, `:1414` |
| le zoom GUI clavier d'egui est consommé **en fin de frame** : pas de conflit avec Ctrl +/-/0 gérés par `handle_input` | `context.rs:2446-2447`, `gui_zoom.rs:33` |

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

Aucun nouvel `EditOp` ni `BatchOp` : la vue est en lecture seule, donc pas d'instantané d'annulation.
Le thread moteur reste le seul à toucher PDFium ; la conversion RGBA -> texture et l'envoi au GPU restent côté UI
(borné à une tuile par réponse).

**Pourquoi remplacer `Request::Render` plutôt que l'étendre.** Le protocole actuel suppose *une* page visible : un seul
rendu en vol (`in_flight`), un compteur `generation` qui ne sert qu'à ignorer une réponse tardive (le worker rend quand
même la page obsolète), un pré-chargement figé à ±1 page, et des pages entières uniquement (donc le plafond de
16 384 px et la limite de texture GPU). En vue continue il faut : plusieurs tuiles utiles simultanément, une priorité
(tuiles au centre de l'écran d'abord), l'abandon des tuiles devenues inutiles *avant* de les rendre, et des régions. Le
compteur `generation` devient l'`epoch` (invalidation par changement de document) ; l'abandon passe par le remplacement
de liste. `Request::Thumbnail` et le cache LRU (`RenderCache`) sont conservés.

### 4.1 Protocole

| Avant | Après |
|---|---|
| `Request::Render { page, scale, generation }` (l.183) | **supprimé** |
| `Response::Rendered { generation, image }` (l.246), `Response::RenderFailed` (l.250) | **supprimés** |
| `State::prefetch`, `schedule_prefetch` (l.318, 336-344) | **supprimés** (le pré-chargement est décidé par l'UI, 5.3.7) |
| — | `Request::SetWanted { epoch: u64, jobs: Vec<TileJob> }` : **remplace** la liste des tuiles voulues, par ordre de priorité décroissante |
| — | `Response::Tile { epoch: u64, job: TileJob, image: Arc<RenderedPage> }` |
| — | `Response::TileFailed { epoch: u64, job: TileJob, error: EngineError }` |
| `DocState { info, can_undo, can_redo }` (l.232) | + `pub pages_unchanged: bool` (faux par défaut dans `doc_state()`) |
| `Request::Thumbnail` / `Response::Thumbnail` | inchangés (cache `render_cached` page entière) |

### 4.2 File des tuiles voulues et boucle

```rust
/// Tuiles à rendre, remplacée en bloc à chaque `SetWanted` : c'est l'« annulation » des rendus de pages sorties de l'écran.
#[derive(Default)]
struct WantedQueue { epoch: u64, jobs: VecDeque<TileJob> }
impl WantedQueue {
    fn replace(&mut self, epoch: u64, jobs: Vec<TileJob>);
    fn pop(&mut self) -> Option<(u64, TileJob)>;
    fn clear(&mut self);
}
```
- `State` : `prefetch: VecDeque<(usize, f32)>` devient `wanted: WantedQueue` ; `invalidate()` (l.346) appelle aussi
  `wanted.clear()` ; il est déjà appelé par `Open`, `edit`, `undo_redo`, `Suspend`, `ResumeDoc`, `Close`.
- `State::render_tile(&mut self, job: &TileJob) -> Result<Arc<RenderedPage>, EngineError>` : `cache.get_tile(job)` ou
  `doc.render_region(job.page, job.scale(), job.region)` puis `cache.insert_tile(*job, image)` (même schéma que
  `render_cached`, l.324-334).
- Boucle `run` (l.618) : la source de travail devient une énumération pour garder l'envoi + vidage de file communs :

```rust
enum Work { Request(Request), Tile(u64, TileJob) }
let work = match queue.pop_front() {
    Some(r) => Work::Request(r),
    None => match rx.try_recv() {                         // 1. les requêtes (ouverture, édition, miniature, SetWanted...) d'abord
        Ok(r) => Work::Request(r),
        Err(TryRecvError::Disconnected) => return,
        Err(TryRecvError::Empty) => match state.wanted.pop() {   // 2. sinon UNE tuile voulue
            Some((epoch, job)) => Work::Tile(epoch, job),
            None => match rx.recv() { Ok(r) => Work::Request(r), Err(_) => return },  // 3. sinon bloquer
        },
    },
};
let response = match work {
    Work::Tile(epoch, job) => match state.render_tile(&job) {
        Ok(image) => Response::Tile { epoch, job, image },
        Err(error) => Response::TileFailed { epoch, job, error },
    },
    Work::Request(request) => match request {
        /* ... bras existants inchangés ... */
        Request::SetWanted { epoch, jobs } => { state.wanted.replace(epoch, jobs); continue; }
    },
};
```
Le reste (envoi, `ctx.request_repaint()`, vidage de la file locale, l.823-835) est inchangé. Conséquence : entre deux
tuiles le worker relit le canal ; un nouveau `SetWanted` ou une édition passe avant la tuile suivante. Un `SetWanted`
envoyé par l'UI à chaque changement de l'ensemble voulu (comparé à la dernière liste envoyée) suffit : pas de message
d'annulation séparé.

### 4.3 Époque (`epoch`) et cohérence avec les éditions

L'UI incrémente `view.epoch` à : ouverture, fermeture, changement d'onglet, `DocumentChanged`. Une `Response::Tile`
d'une autre époque est ignorée. L'ordre FIFO du canal garantit qu'une tuile rendue *avant* une édition arrive *avant*
`DocumentChanged` (donc est jetée par la purge), et qu'une tuile rendue *après* l'édition est rendue sur le contenu
neuf. Un `SetWanted` envoyé avant d'avoir vu `DocumentChanged` produit des tuiles à l'ancienne époque : ignorées, puis
redemandées (coût : quelques tuiles).

### 4.4 `State::edit` : classe d'édition (pour conserver les tuiles pendant une retouche)

```rust
impl EditOp {
    /// Vrai si l'opération ne change ni le nombre, ni l'ordre, ni la taille des pages (objets, annotations, tampon).
    fn keeps_pages(&self) -> bool {
        !matches!(self, EditOp::Rotate { .. } | EditOp::Delete { .. } | EditOp::Insert { .. }
                      | EditOp::Move { .. } | EditOp::Bytes(_))
    }
}
```
`State::edit` (l.372) calcule `let keeps = op.keeps_pages();` avant `apply`, puis, en cas de succès, fabrique
`DocState { pages_unchanged: keeps, ..state }`. Annuler / rétablir (`undo_redo`) laissent `false`.

---

## 5. Interface (`apps/sunipdf-desktop/src`)

### 5.1 Fichiers et fonctions à toucher

| Fichier | Changement |
|---|---|
| **`view.rs` (nouveau)** | `ViewMode`, `Layout`, `Anchor`, `ScrollRequest`, `NavLock`, `TexturePool<H>`, `TileTex<H>`, `ViewState`, `effective_zoom`, `clamp_zoom`, `wanted_jobs`, `tile_screen_rect` (fonctions **pures**, testées) ; `impl SuniApp { document_view, paint_slot, handle_page_event, resolve_scroll, apply_wheel_zoom, flush_wanted, set_view_mode, track_page, go_to_page_point, scroll_screens }` |
| `main.rs` | `mod view;` |
| `app.rs` | supprimer `PageTexture` (l.38), champs `texture`, `generation`, `in_flight`, `last_rendered` (l.90-93) et leur initialisation (l.171-176) ; ajouter `pub view: ViewState` (init : `ViewState::new(settings.continuous_view)`, à lire **avant** le déplacement de `settings`) ; `close_document` (l.243-246) -> `self.view.reset()` ; `go_to_page` (l.268) ; bras `Response::Rendered` (l.388), `DocumentChanged` (l.422-439), `RenderFailed` (l.529) -> `Tile`, `DocumentChanged`, `TileFailed` ; supprimer `request_render_if_needed` (l.558-572) ; `handle_input` (l.576) ; `execute` (l.793) ; `document_area` (l.1058) ne garde que l'écran vide / moteur absent puis appelle `self.document_view(ctx, ui)` ; `thumbnails_panel` (l.1235) ; `paint_matches` (l.1454) ; `enum DragEvent` (l.1888) -> `pub(crate)` ; supprimer `quantize` (l.1920) et adapter son test (l.1995) ; `ZOOM_STEPS` (l.16) passe à 15 entrées `[0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0]` (type `[f32; 15]`, importé par `shell.rs`) ; le motif de destructuration de `Response::DocumentChanged` (l.422-426) reçoit `pages_unchanged` ; le test `ui_keys_used_by_app_exist` (l.1964) doit aussi lire `view.rs` (`concat!(include_str!("app.rs"), include_str!("view.rs"), include_str!("main.rs"))`) |
| `worker.rs` | section 4 |
| `shell.rs` | `ToolTab::All` et `Read` (l.46-87) : + `ViewContinuous`, `ViewSinglePage` ; `command_icon` (l.246, `match` exhaustif) : 2 bras ; `right_rail` (l.874) : bouton de bascule ; combo de zoom (l.969-980) ; `status_bar` (l.1162) : `self.in_flight.is_some()` -> `self.view.is_rendering()` |
| `tabs.rs` | `TabState` + `scroll: egui::Vec2` ; `take_tab` (l.51) et `put_tab` (l.80) : retirer `texture/in_flight/last_rendered/generation`, appeler `self.view.reset_for_tab()` ; `put_tab` : `self.view.request(ScrollRequest::Offset(t.scroll))` |
| `settings.rs` | `pub continuous_view: bool` (défaut `true`) ; test d'aller-retour : l'ancien JSON sans le champ donne `true` |
| `annotate.rs`, `editor.rs`, `organize.rs`, `docinfo.rs`, `tools.rs`, `dialog.rs` | **inchangés** (leurs fonctions prennent déjà `page`, `page_rect`, `page_h_pt`) |
| `crates/sunipdf-core/src/command.rs` | 2 variantes + 2 entrées de registre (5.5) |

### 5.2 Constantes et types (`view.rs`)

```rust
pub const PAGE_MARGIN: f32 = 16.0;          // la marge actuelle de document_area
pub const PAGE_GAP: f32 = 12.0;             // entre deux pages en mode continu
pub const ZOOM_MIN: f32 = 0.1;              // plancher (molette) ; la liste reste à 0,25
pub const ZOOM_MAX: f32 = 32.0;             // 3200 %
pub const MAX_CONTENT_PX: f32 = 8_000_000.0;   // hauteur totale logique maximale (précision f32)
const POOL_BUDGET_BYTES: usize = 192 * 1024 * 1024;   // textures GPU des pages
const MAX_WANTED: usize = 24;               // longueur max d'un SetWanted
const PREFETCH_PAGES: usize = 1;            // pages voisines pré-chargées de chaque côté
const SETTLE_SECS: f64 = 0.15;              // silence avant de redemander des tuiles après un zoom
const LINE_SCROLL_PX: f32 = 48.0;
const SCREEN_SCROLL: f32 = 0.9;
const THUMB_KEEP: usize = 256;              // miniatures conservées au plus (5.3.9)

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ViewMode { Single, #[default] Continuous }

pub enum ScrollRequest {
    Page(usize),                                  // haut de la page, aligné en haut de la zone visible
    PagePoint { page: usize, y_from_top_pt: f32 },// point de la page placé au tiers supérieur (résultat de recherche)
    Offset(egui::Vec2),                           // décalage absolu (restauration d'onglet)
    By(f32),                                      // défilement relatif en px logiques (flèches)
    Screens(f32),                                 // défilement en hauteurs de zone visible (Page suiv./préc., Espace)
}

pub struct NavLock { pub page: usize, pub expected_y: Option<f32> }   // page courante verrouillée après un « aller à » ; None tant que le décalage résultant n'est pas connu

pub struct ViewState {
    pub mode: ViewMode,
    pub pool: TexturePool<egui::TextureHandle>,
    pub epoch: u64,
    pub pending: Option<ScrollRequest>,
    pub nav_lock: Option<NavLock>,
    pub offset: egui::Vec2,                 // dernier décalage connu (ancre de zoom, onglets)
    pub inner: egui::Rect,                  // zone de défilement de la dernière frame (écran)
    pub k_prev: Option<f32>,                // points logiques par point PDF à la dernière frame
    pub vw_prev: f32,                       // largeur utile à la dernière frame
    pub first_prev: usize,                  // première page de la fenêtre à la dernière frame
    pub wheel_anchor: Option<egui::Pos2>,   // pointeur lors d'un zoom molette/pincement
    pub settle_at: Option<f64>,             // instant (InputState.time) avant lequel on ne redemande pas de tuiles
    pub sent: Vec<TileJob>,                 // dernière liste envoyée au worker
    pub failed: std::collections::HashSet<TileJob>,
    pub missing_visible: usize,             // tuiles visibles sans texture exacte (barre d'état)
    pub follow_thumb: bool,                 // la miniature courante doit être recentrée
}
impl ViewState {
    pub fn new(continuous: bool) -> Self;       // mode initial = Settings.continuous_view
    pub fn bump_epoch(&mut self);               // epoch += 1 ; sent.clear() ; failed.clear() (le worker a purgé sa liste dans invalidate())
    pub fn reset(&mut self);                    // ouverture / fermeture : pool.clear(), bump_epoch(), pending = None, nav_lock = None, offset = ZERO, k_prev = None, settle_at = None ; le mode est conservé
    pub fn reset_for_tab(&mut self);            // = reset() (appelé par take_tab / put_tab)
    pub fn request(&mut self, r: ScrollRequest);// pending = Some(r)
    pub fn is_rendering(&self) -> bool;         // missing_visible > 0 (barre d'état)
}
```

### 5.3 Algorithmes

#### 5.3.1 Mise en page (`Layout`, pur)

```rust
pub struct Layout {
    pub first: usize,               // index absolu de la première page de la fenêtre
    pub pt: Vec<PageSize>,          // tailles PDF de la fenêtre
    pub tops: Vec<f32>,             // y du haut de chaque page (contenu), marge haute incluse
    pub sizes: Vec<egui::Vec2>,     // taille affichée = pt * k (points logiques)
    pub content: egui::Vec2,        // (max(largeur max + 2*marge, viewport_w), y final + marge)
}
impl Layout {
    pub fn new(window: &[PageSize], first: usize, k: f32, margin: f32, gap: f32, viewport_w: f32) -> Self;
    pub fn page_left(&self, page: usize) -> f32;          // (content.x - size(page).x) / 2
    pub fn page_top(&self, page: usize) -> f32;
    pub fn size(&self, page: usize) -> egui::Vec2;        // taille affichée de la page (index absolu)
    pub fn visible(&self, y0: f32, y1: f32) -> std::ops::Range<usize>;   // pages dont [top, top+h] coupe [y0, y1], recherche dichotomique
    pub fn dominant(&self, y0: f32, y1: f32) -> usize;    // plus grande surface visible, égalité -> plus petit indice
    pub fn anchor_at(&self, y: f32) -> (usize, f32);      // (page la plus proche, fraction verticale 0..1)
    pub fn y_of(&self, page: usize, frac: f32) -> f32;
}
```
`k = zoom * SCREEN_PER_PDF_POINT`. Fenêtre : **continu** `0..n` ; **simple** `p..p+1` avec `p = doc.page`. Coût par frame :
linéaire en nombre de pages de la fenêtre (à mesurer, 8.5) ; aucun widget ni texture pour les pages invisibles.

#### 5.3.2 Zoom effectif (pur)

```rust
pub fn effective_zoom(zoom: Zoom, mode: ViewMode, pages: &[PageSize], current: usize, avail: egui::Vec2, margin: f32) -> f32;
pub fn clamp_zoom(z: f32, pages: &[PageSize], ppp: f32, mode: ViewMode) -> (f32, bool /* limité */);
```
- Page de référence : **simple** = `pages[current]` (comportement actuel, `app.rs:1078-1090`) ; **continu** = largeur max et
  hauteur max de tout le document.
- `FitWidth = (avail.x - 2m) / (w_ref * SPP)` ; `FitPage = min(FitWidth, (avail.y - 2m) / (h_ref * SPP))` ; plancher 0,05.
- `clamp_zoom` : `z.clamp(ZOOM_MIN, min(ZOOM_MAX, MAX_TILED_SIDE_PX / (côté_max_pt * SPP * ppp), MAX_CONTENT_PX / (somme_hauteurs_pt * SPP)))` ;
  en cas de limitation, `status_msg = tr("status.zoom_limit")` (une fois par changement).
- `self.effective_zoom` est mis à jour à chaque frame **avant** les peintures : `paint_matches`, `paint_selection`,
  `paint_object_selection`, `corner_at`, `annotate::to_pdf` l'utilisent tels quels (zoom uniforme pour toutes les pages).

#### 5.3.3 Boucle d'une frame (`document_view`, remplace le corps de `document_area` l.1058-1233)

```rust
fn document_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let ppp = ctx.pixels_per_point();
    let avail = ui.available_size();
    self.view.pool.begin_frame();
    // 1. fenêtre, zoom
    let window = self.view_window();                       // 0..n (continu) ou p..p+1 (simple)
    self.apply_wheel_zoom(ctx);                            // Ctrl+molette : self.zoom = Zoom::Fixed(..), view.wheel_anchor
    let z = clamp(effective_zoom(...)); self.effective_zoom = z; let k = z * SCREEN_PER_PDF_POINT;
    let now = ctx.input(|i| i.time);
    if self.view.k_prev.map_or(true, |p| (p - k).abs() > 1e-4) { self.view.settle_at = Some(now + SETTLE_SECS); }
    // 2. mise en page (emprunt de self.document limité à ce bloc)
    let layout = Layout::new(&doc.info.page_sizes[window.clone()], window.start, k, PAGE_MARGIN, PAGE_GAP, avail.x);
    // 3. décalage imposé cette frame : requête en attente > ancre de zoom > rien (None : ne PAS appeler scroll_offset, cf. R3)
    let offset = self.resolve_scroll(&layout, avail);
    // 4. zone de défilement
    let mut area = egui::ScrollArea::both().id_salt("doc-scroll").auto_shrink([false, false]);
    if let Some(o) = offset { area = area.scroll_offset(o); }
    let mut event = None;
    let content_h = layout.content.y.max(avail.y);
    let v_off = if single { (content_h - layout.content.y) / 2.0 } else { 0.0 };   // centrage vertical actuel en page unique
    let out = area.show_viewport(ui, |ui, viewport| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(layout.content.x, content_h), egui::Sense::hover());   // taille TOTALE : la barre est exacte
        let y_view = viewport.translate(egui::vec2(0.0, -v_off));                // zone visible dans le repère de Layout
        for page in visible_pages(&layout, y_view, self.annotate.drag.map(|d| d.page)) {   // + la page d'un glissement en cours
            let page_rect = snapped(Rect::from_min_size(rect.min + vec2(layout.page_left(page), layout.page_top(page) + v_off), layout.size(page)), ppp);
            event = event.or(self.paint_slot(ui, &slot(page, page_rect, ...)));
        }
    });
    // 5. après la zone
    let viewport = egui::Rect::from_min_size(out.state.offset.to_pos2(), out.inner_rect.size());   // zone visible (contenu)
    self.view.offset = out.state.offset; self.view.inner = out.inner_rect; self.view.k_prev = Some(k); self.view.vw_prev = avail.x; ...
    if continu && self.view.nav_lock.is_none() { self.track_page(layout.dominant(viewport.min.y - v_off, viewport.max.y - v_off)); }
    self.flush_wanted(ctx, &layout, viewport.translate(egui::vec2(0.0, -v_off)), ...);   // 5.3.7
    self.view.pool.evict(POOL_BUDGET_BYTES);
    if let Some((page, ev)) = event { self.handle_page_event(page, ev); }
    self.tool_palette(ctx, ui, palette_origin); self.object_bar(ctx, palette_origin + egui::vec2(46.0, 0.0));
}
```
`snapped` aligne le coin haut-gauche sur la grille de pixels physiques (`(v * ppp).round() / ppp`) pour que
1 texel = 1 pixel physique ; la taille affichée reste la taille idéale (comme aujourd'hui), donc `to_pdf`, les
poignées et les surbrillances restent exacts à moins de 1 px.

#### 5.3.4 Page courante, « aller à », verrou

- `track_page(p)` (continu uniquement) : si `p != doc.page` -> `doc.page = p`, `page_input = (p+1).to_string()`,
  `view.follow_thumb = true`. N'efface **pas** `self.error`.
- `go_to_page(page)` (app.rs:268) : clamp ; si la page change -> `doc.page`, `error = None` ; **toujours**
  `page_input = ...` et `view.pending = Some(ScrollRequest::Page(clamped))` (même si la page est déjà la courante :
  l'utilisateur est peut-être au milieu de la page) ; `view.nav_lock = Some(NavLock { page: clamped, expected_y: None })`.
- `go_to_page_point(page, y_from_top_pt)` : idem avec `PagePoint` ; utilisée par « suivant / précédent » de la
  recherche (`find_bar`) et par l'arrivée des premiers résultats (`SearchResults`) avec `m.rects[0].top` converti en
  `page_h - top`. En page unique : équivalent à `go_to_page`.
- Résolution de `ScrollRequest` en décalage (`resolve_scroll`) : `Page(p)` -> `(offset.x, max(0, top(p) - PAGE_MARGIN))` ;
  `PagePoint` -> `top(p) + y_pt * k - inner_h / 3` ; `Offset(v)` -> `v` ; `By(dy)` -> `offset + (0, dy)` ;
  `Screens(s)` -> `offset + (0, s * inner_h)`. `By`/`Screens` effacent le verrou.
- Après `show_viewport` : si un décalage a été imposé cette frame (requête ou ancre) et que `nav_lock` existe,
  `expected_y = Some(out.state.offset.y)` (egui borne le décalage au contenu : la dernière page ne peut pas toujours être
  alignée en haut). Les frames suivantes : `if !lock_still_valid(expected_y, out.state.offset.y) { nav_lock = None }` avec
  `fn lock_still_valid(expected: Option<f32>, actual: f32) -> bool { expected.map_or(true, |e| (actual - e).abs() <= 1.0) }`.
  Le verrou évite que le suivi remplace « page 7 » par « page 8 » quand plusieurs pages tiennent à l'écran.
- Éditions structurelles (`pages_unchanged == false` **et** tailles de pages différentes) : `ScrollRequest::Page(doc.page)`
  (le sous-décalage dans la page est perdu : accepté). Sinon le décalage est conservé.

#### 5.3.5 Zoom ancré

- `apply_wheel_zoom` : `let zd = ctx.input(|i| i.zoom_delta())` ; si `zd != 1.0` **et** que le pointeur est dans
  `view.inner` (dernière frame) -> `self.zoom = Zoom::Fixed(clamp(self.effective_zoom * zd))`,
  `view.wheel_anchor = pointeur`. Les boutons / `ZoomIn` / `ZoomOut` / liste continuent d'écrire `self.zoom` (`zoom_step`, l.288).
- `resolve_scroll`, si aucune requête et `k != k_prev` (écart > 1e-4) et même fenêtre (`first == first_prev`) :
  `screen = wheel_anchor.map(|p| p - inner.min)` sinon centre de la zone (zoom fixe) ou `(largeur/2, 0)` (modes
  « ajuster » : on garde le haut de page) ; `Anchor::from(&ancienne_mise_en_page, offset, screen)` donne
  `(page, fy, fx)` ; le nouveau décalage est `y = layout.y_of(page, fy) - screen.y`,
  `x = layout.page_left(page) + fx * layout.size(page).x - screen.x` (bornés à >= 0 ; egui borne au contenu).
  L'ancre est exprimée en *(page, fraction)* et non en proportion du contenu, car marge et intervalle ne sont pas mis à l'échelle.
  L'ancienne mise en page se reconstruit par `Layout::new(.., k_prev, .., vw_prev)` (événement rare).
- Dès que `k` change : `view.settle_at = Some(time + SETTLE_SECS)` ; tant que `time < settle_at`, `flush_wanted`
  n'envoie rien et appelle `ctx.request_repaint_after(150 ms)`. Les textures d'une autre échelle servent de repli.

#### 5.3.6 `paint_slot` : une page (identique en simple et en continu)

```rust
struct Slot { page: usize, rect: egui::Rect /* page_rect écran */, size_pt: PageSize, scale_milli: u32,
              limits: TileLimits, total: usize, name: String /* pour a11y.page_image */ }
fn paint_slot(&mut self, ui: &mut egui::Ui, s: &Slot) -> Option<(usize, DragEvent)>
```
1. Ombre (+3, +3, alpha 60) et fond blanc, comme `app.rs:1111-1117`.
2. Tuiles : `view_px` = intersection de `ui.clip_rect()` avec `page_rect`, convertie en pixels de la page ;
   `plan_tiles(page_px, view_px, &limits)` ; pour chaque tuile `TileJob { page, scale_milli, region }` :
   texture exacte dans `view.pool` -> `painter.image(tex.id(), tile_screen_rect(..), UV_0_1, WHITE)` ; sinon
   `missing_visible += 1` et **repli** : toutes les entrées de la même page d'une autre échelle (ou périmées) qui
   coupent la tuile, de la plus éloignée à la plus proche, découpées à l'intersection (UV recalculées par
   normalisation). Aucune texture du tout -> `egui::Spinner` au centre de la partie visible (comme l.1139).
   Texte alternatif `a11y.page_image` porté par un `egui::Image` pour la tuile `(0, 0)` (ou la page entière), comme l.1119-1138.
3. Surcouches inchangées : `self.paint_matches(&painter, page_rect, page, h_pt)`, `self.paint_selection(..)`,
   `self.paint_object_selection(..)` (signatures actuelles).
4. `let interact = ui.interact(page_rect, egui::Id::new(("page-interact", page)), Sense::click_and_drag());`
   puis **exactement** la même extraction d'événement que `app.rs:1146-1186` (`Start` avec `press_origin`, `Move`,
   `Stop`, `DoubleClick`, `Click`) avec `crate::annotate::to_pdf(pos, page_rect, zoom, page_h_pt)` de cette page.
   Retourne `(page, DragEvent)`.
5. `interact.on_hover_cursor(self.page_cursor(page, hover))`.

`handle_page_event(page, ev)` = le `match drag_event` actuel (`app.rs:1187-1229`) avec `page_index` remplacé par `page`
(`edit_drag_start`, `annotate.drag`, `finish_drag`, `edit_click`, `edit_double_click`, `Note/AddText/AddImage`). Un seul
glissement est actif à la fois (egui garde le widget qui a démarré le glissement) ; sa page est forcée dans l'ensemble
visible tant qu'il dure, même si on fait défiler hors de l'écran (sinon le widget disparaîtrait et le glissement
resterait « fantôme »). `to_pdf` clampe aux bornes de la page d'origine : une sélection ne déborde jamais sur la page voisine.

#### 5.3.7 Tuiles voulues, pré-chargement, annulation (`wanted_jobs`, pur)

```rust
pub struct PlanInput<'a> {
    pub layout: &'a Layout, pub k: f32, pub scale_milli: u32,
    pub viewport: egui::Rect,            // zone visible en coordonnées de contenu
    pub drag_page: Option<usize>, pub limits: TileLimits,
}
pub fn wanted_jobs(input: &PlanInput, has_fresh_or_failed: &dyn Fn(&TileJob) -> bool) -> Vec<TileJob>;
```
1. Pages visibles (`layout.visible`) + page du glissement. Pour chacune : tuiles de `plan_tiles` sur la zone visible
   **élargie d'une demi-tuile** (`TILE_SIDE_PX / 2` px page) pour amortir le défilement ; priorité = distance au centre
   de la zone visible.
2. Pré-chargement : `PREFETCH_PAGES` page(s) avant la première et après la dernière page visible (dans la fenêtre) :
   tuiles de leur bord proche sur une hauteur de zone visible ; priorité = 1e9 + distance en pages.
3. On écarte les tuiles déjà fraîches dans le pool ou déjà en échec (`view.failed`), on trie, on dédoublonne, on
   tronque à `MAX_WANTED`.
`flush_wanted` envoie `Request::SetWanted { epoch, jobs }` **seulement si** la liste diffère de `view.sent` (y compris
la liste vide : elle annule le pré-chargement devenu inutile). Limite d'honnêteté : une tuile déjà en cours de rendu ne
s'interrompt pas.

`Response::Tile` (bras de `handle_responses`) : si `epoch == view.epoch` -> `ColorImage::from_rgba_unmultiplied`,
`ctx.load_texture("tile-<page>-<échelle>-<x>-<y>", .., TextureOptions::LINEAR)`, `pool.insert(page, TileTex { .. })`
avec `page_px = tiles::page_pixels(doc.info.page_sizes[job.page], job.scale())` et `bytes = image.rgba.len()`. Une tuile d'une
**autre échelle** mais de la bonne époque est acceptée (le contenu est valide ; elle servira de repli puis sera évincée).
`Response::TileFailed` : `view.failed.insert(job)`, `log::warn!`, et `self.error = Some(error_key(&e))` une seule fois.

#### 5.3.8 `TexturePool<H>` (pur, générique pour être testé sans GPU : `H = ()` dans les tests)

```rust
pub struct TileTex<H> { pub scale_milli: u32, pub region: PixelRect, pub page_px: (u32, u32), pub handle: H,
                        pub bytes: usize, pub used: u64, pub stale: bool }
pub struct TexturePool<H> { pages: HashMap<usize, Vec<TileTex<H>>>, bytes: usize, frame: u64 }
impl<H> TexturePool<H> {
    pub fn begin_frame(&mut self);                                         // frame += 1
    pub fn insert(&mut self, page: usize, tex: TileTex<H>);                // remplace (même échelle + région)
    pub fn exact(&mut self, job: &TileJob) -> Option<&H>;                  // non périmée ; marque « utilisée cette frame »
    pub fn has_fresh(&self, job: &TileJob) -> bool;
    pub fn fallbacks(&mut self, page: usize, scale_milli: u32, area: egui::Rect /* 0..1 */) -> Vec<(egui::Rect, &H)>;
    pub fn mark_stale(&mut self);                                          // éditions « contenu seul » : on garde l'image jusqu'au remplacement
    pub fn clear(&mut self);
    pub fn evict(&mut self, budget: usize);                                // LRU ; jamais une entrée utilisée cette frame
    pub fn bytes(&self) -> usize;  pub fn len(&self) -> usize;
}
```
Éviction : tant que `bytes > budget`, retirer l'entrée dont `used` est la plus petite parmi celles avec
`used < frame`. La borne est donc `budget + textures de la frame courante` (bornée par la taille de l'écran).
Un `TextureHandle` retiré est détruit -> texture GPU libérée (`texture_handle.rs:25-29`).

#### 5.3.9 Miniatures bornées et synchronisées

- `thumbnails: HashMap<usize, TextureHandle>` (app.rs) croît sans limite aujourd'hui (500 pages parcourues ≈ 500 textures).
  Dans `thumbnails_panel` : si `thumbnails.len() > THUMB_KEEP`, `retain(|p, _| p.abs_diff(current) <= 100)`.
- Si `view.follow_thumb` : `ScrollArea::vertical_scroll_offset((current as f32 * (row_h + item_spacing.y) - hauteur/2).max(0.0))`
  pour cette frame, puis `follow_thumb = false`. Un clic de miniature n'active pas le suivi (il appelle `go_to_page`).

### 5.4 Clavier (`handle_input`, app.rs:576)

Avant la fermeture `ctx.input_mut` des commandes (l.699) :

```rust
let continuous = self.view.mode == ViewMode::Continuous;
if continuous {
    let (down, up) = ctx.input_mut(|i| (
        i.consume_key(Modifiers::NONE, Key::PageDown) || i.consume_key(Modifiers::NONE, Key::Space),
        i.consume_key(Modifiers::NONE, Key::PageUp)   || i.consume_key(Modifiers::SHIFT, Key::Space)));
    if down { self.view.pending = Some(ScrollRequest::Screens(SCREEN_SCROLL)); self.view.nav_lock = None; }
    if up   { self.view.pending = Some(ScrollRequest::Screens(-SCREEN_SCROLL)); self.view.nav_lock = None; }
}
```
Les flèches haut/bas : après le bloc de décalage d'objet (l.672-698), `consume_key(NONE, ArrowDown/ArrowUp)` ->
`ScrollRequest::By(±LINE_SCROLL_PX)` (si un objet est sélectionné avec l'outil Sélection, ce bloc les a déjà consommées :
pas de conflit). Dans la fermeture des commandes : `key(Modifiers::NONE, Key::F5, if continuous { Command::ViewSinglePage } else { Command::ViewContinuous })`.
Les lignes `PageDown/PageUp -> NextPage/PreviousPage` (l.712-715) restent pour le mode page unique. Le garde-fou « un champ de
saisie a le clavier » (l.599-633) s'applique avant : ni F5 ni Espace ne s'activent pendant une saisie.

### 5.5 Commandes (registre `command.rs`, menu **Affichage**, `needs_document = true`)

| Variante | id | `label_key` | `shortcut_key` | Menu | Onglets d'outils | Icône |
|---|---|---|---|---|---|---|
| `ViewContinuous` | `view.continuous` | `menu.view.continuous` | `shortcut.view_toggle` | View | Tous les outils, Lire | `glyph::PAGES`, repli `▥` |
| `ViewSinglePage` | `view.single_page` | `menu.view.single_page` | `shortcut.view_toggle` | View | Tous les outils, Lire | `glyph::DOCUMENT`, repli `▯` |

Deux commandes idempotentes (plutôt qu'une bascule) pour que la palette, la CLI `commands` et les scénarios
`-Commands 'view.continuous,...'` soient déterministes ; **F5 bascule** entre les deux (choisi dans `handle_input`),
d'où le même `shortcut_key` affiché. Insertion dans `REGISTRY` après `FitPage` (avant `FirstPage`, donc avant le
séparateur du menu Affichage, `shell.rs:467`). `execute` : `Command::ViewContinuous => self.set_view_mode(ViewMode::Continuous)`,
`ViewSinglePage => ...Single` ; `set_view_mode` : sort si le mode est déjà actif, sinon `view.mode = m`,
`settings.continuous_view = (m == Continuous)`, `persist_settings()`, `view.pending = Some(Page(doc.page))`, `view.pool` conservé.
`command_color` : le joker `Menu::View` couvre déjà les deux. `ToolTab::commands` : ajouter les deux à `All` et `Read`.
Rail droit (`right_rail`, après le bouton « Organiser ») : un `Button` `.selected(continuous)` dont l'icône et l'info-bulle
sont ceux de la commande **opposée** (`menu.view.single_page` si on est en continu, et réciproquement).
Aucun bouton factice : les deux commandes sont connectées, testées (8.3) et capturées (8.6).

### 5.6 Onglets, annulation, mode Organiser

- **Onglets** : `take_tab` range `scroll = view.offset` ; `reset_for_tab()` vide le pool (libère le GPU des onglets
  inactifs), incrémente l'époque, efface `pending/nav_lock/k_prev/sent/failed` ; `put_tab` demande `Offset(scroll)`
  (l'état egui du `ScrollArea` est partagé par identifiant : sans cette requête l'onglet repris hériterait du décalage
  du précédent). Le mode (continu/simple) et le zoom (`Zoom`, déjà dans `TabState`) : le mode est global (préférence),
  le zoom par onglet.
- **Annulation / rétablissement** (`Ctrl+Z`) : `DocumentChanged { pages_unchanged: false }` -> `pool.clear()`, époque+1 ;
  décalage conservé si les tailles de pages sont identiques.
- **Éditions de contenu** (déplacer un objet, tampon, surlignage...) : `pages_unchanged == true` -> `pool.mark_stale()` :
  l'image précédente reste affichée (repli) jusqu'à l'arrivée de la tuile neuve -> **plus de page blanche + spinner à
  chaque retouche** (comportement actuel : `app.rs:436`). Les autres éditions vident le pool.
- **Mode Organiser** : `document_view` n'est pas appelé ; une requête de défilement en attente (`go_to_page` depuis la grille)
  est consommée à la prochaine frame où la vue document est dessinée.
- **Panneau des commentaires** : `refresh_annotations_if_needed` (annotate.rs) compare déjà `annotations_page` à `doc.page` :
  il suit la page courante pendant le défilement (une requête `Annotations` par changement de page).

### 5.7 Budgets mémoire (bornés, testés)

| Zone | Plafond | Où |
|---|---|---|
| Cache moteur (RGBA, pages entières + tuiles + miniatures) | 256 Mio (existant, `CACHE_BYTES`) | `worker.rs:16` |
| Pool de textures (GPU) | 192 Mio + textures de la frame courante | `POOL_BUDGET_BYTES` |
| Miniatures (GPU) | 256 entrées | `THUMB_KEEP` |
| Mise en page | O(pages de la fenêtre) | `Layout` |
| Liste de tuiles voulues | 24 | `MAX_WANTED` |
| Instantanés d'annulation | inchangé (50 entrées / 512 Mio) | `worker.rs:18-19` |
| Transitoire | 1 `ColorImage` par tuile reçue | `handle_responses` |

### 5.8 Conformité aux règles permanentes (`CLAUDE.md`)

Aucune chaîne UI littérale (clés 7) · l'UI n'importe jamais `pdfium-render` (elle utilise les types purs
`sunipdf_core::{PixelRect, TileJob, TileLimits}`) · un seul thread moteur, aucun rendu sur le thread UI (seule la conversion
RGBA -> texture d'une tuile s'y fait) · erreurs typées (`EngineError`, `TileFailed` -> `error_key`) · aucune écriture de
fichier ni envoi réseau · aucune boîte modale (donc pas de `dialog::show`) · aucun bouton factice (5.5) · pas de revendication
de sécurité · fonctions non livrées (sélection multi-pages, double page...) restent en feuille de route (2.1).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Pas de nouvelle sous-commande (ponytail) : `render` gagne une option, ce qui permet de tester les tuiles à la main.

```text
sunipdf-cli render <file> [--page N] [--scale S] [--region x,y,w,h] --out <png> [--password P]
```
- `--region x,y,w,h` : région en **pixels de la page rendue à `--scale`** (origine en haut à gauche) ; absente = page entière
  (comportement actuel, plafond 16 384 px inchangé). Analyse par `value_parser = parse_region` (`Result<PixelRect, String>`) ;
  l'appel devient `match region { Some(r) => doc.render_region(index, scale, r)?, None => doc.render_page(index, scale)? }`
  (l.587-607).
- Sortie JSON (succès) : `{"out":"tile.png","width":256,"height":256,"page":1,"region":[4000,7000,256,256],"page_pixels":[24480,31680]}` ;
  `region` vaut `null` sans l'option ; `page_pixels` vient de `tiles::page_pixels` (`null` si hors limites).
- Erreurs : JSON habituel avec `EngineError::code()` (`render_too_large`, `page_out_of_range`, `other` pour une région
  vide / hors page).
- Vérification manuelle (la page 1 de `two-pages.pdf` fait 612 x 792 pt ; son rectangle bleu occupe x 72..272 pt,
  y 500..650 pt depuis le bas) :
  `cargo run -p sunipdf-cli -j 2 -- render crates\sunipdf-core\tests\fixtures\two-pages.pdf --page 1 --scale 40 --region 4000,7000,256,256 --out tile.png`
  doit afficher `page_pixels` `[24480,31680]` et produire un PNG 256 x 256 uni bleu `(0,0,255)`.

---

## 7. i18n

Catalogues : `assets/i18n/fr-FR.json` **et** `en-US.json` (les tests `catalogs_have_identical_key_sets`,
`no_empty_values`, `placeholders_match_between_languages` et `scripts\check-i18n.ps1` doivent rester verts).

**Nouvelles clés (4)**

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.view.continuous` | Défilement continu | Continuous Scrolling |
| `menu.view.single_page` | Page unique | Single Page |
| `shortcut.view_toggle` | F5 | F5 |
| `status.zoom_limit` | Zoom maximal atteint pour ce document | Maximum zoom reached for this document |

**Clés existantes modifiées (3)** : le défilement d'un écran prend la place de « page suivante » sur Page suiv. en continu,
les flèches gauche/droite restent valides dans les deux modes -> le raccourci affiché devient neutre.

| Clé | fr-FR (avant -> après) | en-US (avant -> après) |
|---|---|---|
| `shortcut.next_page` | Page suiv. -> → | Page Down -> → |
| `shortcut.previous_page` | Page préc. -> ← | Page Up -> ← |
| `status.rendering` | Rendu de la page… -> Rendu des pages… | Rendering page… -> Rendering pages… |

Aucune chaîne littérale dans `view.rs` : tout passe par `tr` / `tr_args` (`a11y.page_image`, `status.zoom_limit`,
libellés de commandes). Le test `ui_keys_used_by_app_exist` lit désormais `view.rs` (5.1).

---

## 8. Tests

Conventions du dépôt : `engine()` sérialisé (`tests/engine.rs:12-48`), vérification par pixels rendus, aucune valeur
inventée. **Première tentative = égalité exacte** des pixels ; si PDFium diffère aux bords de tuile (anti-crénelage
dépendant du découpage), l'implémenteur relâche à « canal max <= 2 sur >= 99,9 % des pixels », le consigne dans le
commentaire du test et dans `KNOWN_ISSUES.md` — ne pas affirmer l'égalité exacte sans l'avoir observée.

### 8.1 Intégration moteur (`crates/sunipdf-core/tests/engine.rs`)

Aides à ajouter : `fn crop(img: &RenderedPage, r: PixelRect) -> Vec<u8>` et `fn px(img, x, y) -> [u8; 4]`.

| Test | Contenu vérifié |
|---|---|
| **T-REG-1** `render_region_equals_crop_of_full_render` | `two-pages.pdf` page 0, échelle 2,0 (1224 x 1584) : `render_page` puis `render_region` sur `(0,0,512,512)`, `(512,1024,712,560)` (bord non aligné), `(1000,1300,224,284)` (coin) ; dimensions = celles de la région ; octets = découpe du rendu complet |
| **T-REG-2** `render_region_on_rotated_page_matches_crop` | `set_page_rotation(0, Rotation::R90)` (pages.rs:158) puis rendu complet à 1,0 : taille `792 x 612` ; une région égale la découpe (les coordonnées de tuile sont dans l'espace *tourné*) |
| **T-REG-3** `render_region_deep_zoom_beyond_full_page_limit` | échelle 40 : `render_page(0, 40.0)` -> `RenderTooLarge` ; `render_region(0, 40.0, (4000,7000,256,256))` -> 65 536 pixels `(0,0,255,255)` (intérieur du rectangle bleu : x 2880..10880, y 5680..11680) ; région `(10800,7000,256,256)` à cheval sur le bord droit : colonne 40 bleue, colonne 120 blanche `(255,255,255,255)` (tolérance sur les colonnes 79..81) ; région `(2880,3000,1024,1024)` autour du texte bleu (lettres « Su » de 24 pt, trait d'environ 80 px à cette échelle) : une part significative de pixels colorés (seuil indicatif de 5 % à ajuster **après observation**, en le fixant nettement sous la valeur mesurée) ; **fond opaque** : aucun pixel alpha 0 (valide H1 : le *clear* à coordonnées négatives) |
| **T-REG-4** `render_region_rejects_invalid_requests` | région vide -> `Other` ; `x + width > page_w` -> `Other` ; `width > 16384` -> `RenderTooLarge` ; échelle `NaN` / `0` / négative -> `RenderTooLarge` ; page 9 -> `PageOutOfRange(9)` ; échelle 300 (page > 131 072 px) -> `RenderTooLarge` |
| **T-REG-5** `render_region_deep_zoom_on_image_page` | PNG 64 x 64 dégradé (crate `image`, dev-dep existante) écrit dans `std::env::temp_dir()`, `engine.images_to_pdf(&[png], &out, 72.0)`, ouverture, `render_region` à l'échelle 50 d'une zone 256² au centre : réussit, alpha 255 partout, pixels non uniformes (page raster à fort zoom : mémoire bornée par la région, pas par la page) |
| **T-REG-6** `many_pages_fixture_pages_have_expected_sizes_and_colors` | `many-pages.pdf` : 120 pages ; tailles lues par `page_size(i)` (portrait Lettre, paysage pour `i % 10 == 9`, A5 pour `i % 25 == 24` sauf collision) ; pour `i` dans `{0, 9, 24, 49, 119}`, un `render_region` 8 x 8 au centre du carré coloré donne `((i*37)%256, (i*91)%256, (i*53)%256)` à ±1 : prouve la correspondance indice de page -> contenu utilisée par worker et UI |
| **T-REG-7** `tiles_reassemble_into_full_render` | échelle 2,0 de `two-pages.pdf` : `plan_tiles` avec des limites forçant le découpage (`TileLimits { tile_side: 256, whole_max_side: 512, whole_max_pixels: 262_144 }`) sur la page entière ; rendre chaque tuile, les recoller dans un tampon, comparer au `render_page` : valide la grille, les tuiles de bord et l'absence de couture (H2) |
| **T-REG-8** `render_scroll_100_pages_timing` (`#[ignore]`) | voir 8.5 : mesure uniquement, aucune assertion de durée |

### 8.2 Unitaires moteur (sans PDFium)

- `tiles.rs` : `quantize_milli` (`1.3333 -> 1335`, `0.1 -> 100`, `0.0 -> 5`) ; `page_pixels` égale `render_page` (612x792 à 1,0 ; `0.5 -> (306,396)`) et renvoie `None` pour `NaN`, `0`, négatif, `> 131 072` ;
  `is_whole` aux seuils exacts (4096x2048 = 8 Mpx -> vrai, 4097 de large -> faux, 3000x3000 -> faux) ;
  `plan_tiles` : page entière -> une tuile quelle que soit la zone qui la coupe ; hors page -> vide ; grille 5000x7000 avec zone `(900,900,300,300)` -> 4 tuiles
  `(0,0)`, `(1024,0)`, `(0,1024)`, `(1024,1024)` ; la **réunion** des tuiles couvre la zone **sans trou ni recouvrement** (propriété testée sur plusieurs zones) ;
  tuiles de bord tronquées (`w - x`) ; `TileLimits::for_texture_side(2048)` -> tuile 1024 / entière 2048 ; `(512)` -> 512 ; `(16384)` -> 1024 / 4096 ; `(10)` -> 256.
- `cache.rs` : clés de tuiles distinctes par région et par échelle ; **plafond d'octets jamais dépassé** avec un mélange pages / tuiles / miniatures ; LRU (la tuile relue survit) ; entrée unique plus grosse que le plafond conservée seule ; `clear` remet `bytes()` à 0.

### 8.3 Unitaires interface (`view.rs`, `worker.rs`, `settings.rs`, `command.rs`) — purs, sans GPU ni PDFium

- `Layout` : 3 pages Lettre à `k = 1`, `viewport_w = 1000` -> `tops = [16, 820, 1624]`, `content = (1000, 2432)`, `page_left = 194` ; monotonie pour tailles mixtes ;
  **virtualisation** : 600 pages, zone visible de 800 px vers `y = 300 000` -> `visible()` renvoie au plus 3 indices, centrés sur l'indice attendu, jamais 600 ;
  `dominant` : plus grande surface, égalité -> plus petit indice, zone dans un intervalle entre pages -> page la plus proche ;
  `anchor_at` / `y_of` : aller-retour ; **ancre de zoom** : mise en page à `k1`, décalage `(0, 5000)`, ancre écran `(400, 300)`, puis `k2 = 1,7 * k1` -> la page et la fraction retrouvées au nouveau décalage diffèrent de moins de 1e-3 et la position écran de l'ancre de moins de 0,5 px.
- `effective_zoom` : pages `[612x792, 792x612]`, zone `(1000, 800)` : continu `FitWidth = 968/1056 ≈ 0,9167`, `FitPage = 768/1056 ≈ 0,7273` ; simple (page 0) `FitWidth = 968/816`.
- `clamp_zoom` : page 14 400 x 14 400 pt à `ppp = 2` plafonnée par `MAX_TILED_SIDE_PX` ; 600 pages A4 plafonnées par `MAX_CONTENT_PX` ; renvoie le drapeau « limité ».
- `lock_still_valid` : `(Some(100.0), 100.5) -> vrai`, `(Some(100.0), 102.0) -> faux`, `(None, 5000.0) -> vrai`.
- `TexturePool<()>` : 1000 tuiles de 1 Mio, budget 64 Mio, trois entrées marquées utilisées à la dernière frame -> `bytes() <= 64 Mio` après `evict`, les trois survivent, la plus ancienne a disparu (**mémoire du pool jamais au-dessus de la borne**, sauf textures de la frame) ;
  `mark_stale` : `exact` ne les voit plus, `fallbacks` les renvoie, un `insert` neuf remplace l'entrée périmée sans doubler `bytes()` ; `fallbacks` ordonnés du plus éloigné au plus proche.
- `wanted_jobs` : 500 pages, zone au milieu, pool vide -> `len() <= MAX_WANTED`, aucune page hors `[première visible - 1, dernière visible + 1]`, première tuile sur une page visible ; avec toutes les tuiles visibles fraîches -> uniquement du pré-chargement ; tuile en `failed` exclue ; page très zoomée (A4 à 20 px/pt) -> au plus `(L/1024 + 2) x (H/1024 + 2)` tuiles ; page du glissement forcée.
- `WantedQueue` : `replace` écrase l'ancienne liste (seule la dernière est rendue), `pop` dans l'ordre, `clear`.
- `EditOp::keeps_pages` : `true` pour `MoveObject`, `Markup`, `Stamp` ; `false` pour `Rotate`, `Delete`, `Insert`, `Move`, `Bytes`.
- `Settings` : valeur par défaut `continuous_view == true` ; un JSON ancien (`{"language":"xx-XX"}`) l'obtient aussi (`#[serde(default)]`).
- Commandes : `Command::from_id("view.continuous")` / `"view.single_page"` ; les tests existants `registry_keys_exist_in_both_catalogs` (app.rs) et `ids_are_unique_and_round_trip` couvrent clés et unicité.
- `quantize_milli` remplace `quantize` : adapter `zoom_quantization_and_pdf_detection` (`1.3333 -> 1335`).

### 8.4 Fixtures à générer (`scripts/make_fixtures.py`, sans dépendance)

Ajouter `many_pages(path, n)` et l'appeler dans `__main__` : `many_pages(os.path.join(out, "many-pages.pdf"), 120)` ;
option `--many N CHEMIN` pour produire à la demande un fichier de 500 pages **non versionné**
(`python -I scripts\make_fixtures.py --many 500 $env:TEMP\many-500.pdf`).

```python
def many_pages(path, n=120):
    # objets : 1 Catalog, 2 Pages, 3 Font, puis (page, contenu) = (4+2i, 5+2i)
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>", None, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
    kids = []
    for i in range(n):
        w, h = (792, 612) if i % 10 == 9 else ((420, 595) if i % 25 == 24 else (612, 792))
        r, g, b = (i * 37) % 256, (i * 91) % 256, (i * 53) % 256
        c = f"{r/255:.4f} {g/255:.4f} {b/255:.4f} rg 36 36 120 80 re f\n0 g\nBT /F1 24 Tf 36 {h-60} Td (Page {i+1}) Tj ET\n"
        kids.append(f"{4+2*i} 0 R")
        objs.append(f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents {5+2*i} 0 R "
                    f"/Resources << /Font << /F1 3 0 R >> >> >>".encode())
        objs.append(stream(c.encode()))
    objs[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {n} >>".encode()
    open(path, "wb").write(build(objs))
```
Le carré coloré occupe x 36..156 pt, y 36..116 pt depuis le bas (donc `y_haut = h - 116 .. h - 36` en pixels à 1,0) ;
le test échantillonne au centre `(96, h - 76)`. Fichier attendu : quelques dizaines de Kio. Régénérer ensuite
`python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures` et versionner `many-pages.pdf`.

### 8.5 Mesures (à faire à l'implémentation ; **ne rapporter que ce qui est mesuré, avec machine et commande**)

- Rendu : `cargo test -p sunipdf-core --release -j 2 -- --ignored --nocapture render_scroll_100_pages_timing` ;
  T-REG-8 affiche, pour les 100 premières pages de `many-pages.pdf` répétées, le temps total et le temps moyen par tuile
  1024² à 1,5 px/pt, plus le temps de la première page. Aucune assertion de durée.
- Mémoire : ouvrir `many-500.pdf`, `nav.last_page`, puis `{PGDN}` x 100 ; relever
  `Get-Process sunipdf | Select-Object WorkingSet64,PrivateMemorySize64` avant / après. Le journal
  `$env:RUST_LOG='sunipdf_desktop=debug'` affiche à chaque `flush_wanted` : octets du pool, nombre d'entrées, longueur de la
  liste envoyée (à ajouter en `log::debug!`) ; le worker journalise `cache.bytes()` en `trace`.
- Coût de `Layout::new` par frame : mesure avec 500 puis 5000 pages (`Instant` autour de l'appel, journal `trace`) pour
  confirmer que la reconstruction par frame suffit ; sinon la mettre en cache sur `(k, viewport_w, nombre de pages)`.
- À noter dans `PROJECT_STATUS.md` : modèle de CPU, RAM, GPU, mise à l'échelle Windows, commande exacte, valeurs.

### 8.6 Scénarios manuels (`scripts\dev-screenshot.ps1 -Pdf … -Commands '…' -Drag '…' -KeysAfter '…' -Out …`)

Prérequis : `. .\scripts\dev-env.ps1`, `cargo build -p sunipdf-desktop -j 2`, `.\scripts\fetch-pdfium.ps1`.
Les coordonnées de glissement dépendent de la fenêtre : prendre d'abord une capture **sans** `-Drag` pour repérer le texte,
puis rejouer (méthode déjà utilisée pour les captures précédentes). Fixtures : `F = crates\sunipdf-core\tests\fixtures`.

| # | Commande | Attendu |
|---|---|---|
| S1 | `-Pdf $F\many-pages.pdf -Out s1.png` | mode continu par défaut ; pages 1 et 2 empilées, intervalle gris, barre d'état « Page 1 / 120 », miniature 1 surlignée |
| S2 | `-Pdf $F\many-pages.pdf -Commands 'nav.last_page' -Out s2.png` | « Page 120 / 120 », dernière page visible, **aucune page blanche** (tuiles reçues), page 119 visible au-dessus ; la miniature 120 est recentrée dans le panneau |
| S3 | `-Pdf $F\many-pages.pdf -Commands 'view.single_page' -Out s3.png` | une page centrée comme avant ; comparer à `ref-single.png` (ancienne version, pris à l'étape 0 : `-Pdf $F\many-pages.pdf -Out ref-single.png`, puis `-Commands 'view.fit_page'` et `-Commands 'view.actual_size'`). Avant l'étape 3 (la commande n'existe pas encore), la comparaison se fait avec l'ouverture simple, qui est alors le seul mode |
| S4 | `-Pdf $F\two-pages.pdf -Commands 'view.fit_page,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in,view.zoom_in' -Out s4.png` | dix `view.zoom_in` après « ajuster à la page » (0,73 -> 0,75, 1, 1,25, 1,5, 2, 3, 4, 6, 8, 12) : zoom 1200 %, **pas de panique**, texte net, aucune couture visible entre tuiles (H2). Nécessite la correction de `zoom_step` de l'étape 4 : les commandes d'un scénario sont exécutées dans la même frame |
| S5 | `-Pdf $F\many-pages.pdf -Keys '^f' -KeysAfter 'Page 7{ENTER}' -Out s5.png` | surbrillance orange sur « Page 7 » **et défilement jusqu'à elle** ; compteur de résultats |
| S6 | `-Pdf $F\many-pages.pdf -Commands 'nav.next_page' -Drag '<sur le texte de la page 2>' -KeysAfter '^c' -Out s6.png` | sélection bleue sur la page 2 (pas la 1), message « Texte copié dans le presse-papiers » |
| S7 | `-Pdf $F\many-pages.pdf -Commands 'nav.next_page,tool.highlight' -Drag '<sur le texte de la page 2>' -Out s7.png` | surlignage jaune sur la page 2 après nouveau rendu de ses tuiles ; panneau commentaires (F7) : 1 annotation sur la page 2 |
| S8 | `-Pdf $F\many-pages.pdf -Commands 'nav.next_page,tool.rectangle' -Drag '500,300,700,420' -Out s8.png` | rectangle tracé sur la **page 2** ; puis `tool.select_object` + second glissement : poignées, déplacement, par page |
| S9 | `-Pdf $F\many-pages.pdf -Drag '600,400,600,400' -KeysAfter '{PGDN}{PGDN}{DOWN}' -Out s9.png` (glissement de longueur nulle = clic) | le défilement clavier fonctionne **après un clic sur une page** (H4) ; barre d'état : page courante incrémentée |
| S10 | `-Pdf $F\many-pages.pdf -Commands 'nav.last_page,view.fit_width,view.fit_page' -Out s10.png` | « ajuster » conserve la page courante en haut (ancre) |
| S11 | `-Pdf $F\many-pages.pdf -Commands 'nav.last_page' -KeysAfter '{HOME}' -Out s11.png` | retour en haut, « Page 1 / 120 » |

Non automatisable aujourd'hui : **Ctrl + molette** (le script n'injecte pas de molette). Étape 7 optionnelle : ajouter un paramètre
`-Wheel "dy,ctrl"` à `dev-screenshot.ps1` (`mouse_event` avec `MOUSEEVENTF_WHEEL = 0x0800` et `keybd_event` pour Ctrl) ;
à défaut, vérification manuelle par l'utilisateur : le point sous le pointeur reste fixe pendant le zoom.

### 8.7 Commandes de validation (règles du dépôt)

```powershell
. .\scripts\dev-env.ps1
cargo fmt --all
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
.\scripts\check-i18n.ps1
python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures
```

---

## 9. Nouvelles dépendances

**Aucune.** Tout repose sur `egui`/`eframe` 0.36, `pdfium-render` 0.9.4, `image` 0.25 (dev-dependency déjà présente dans
`sunipdf-core`) et la bibliothèque standard. `Cargo.lock` inchangé. Pas de nouvelle licence à consigner dans
`THIRD_PARTY_NOTICES.md`.

---

## 10. Risques, pièges connus, étapes, estimation

### 10.1 Risques et pièges

| # | Risque / piège | Parade |
|---|---|---|
| R1 | PDFium et tailles cibles énormes avec bitmap minuscule (H1) | T-REG-1/3/7 en premier (étape 1) ; plan B en 2.2 ; plafond `MAX_TILED_SIDE_PX` ajustable |
| R2 | **`egui_glow` panique si une texture dépasse `max_texture_side`** (`painter.rs:553`) ; `load_texture` n'a qu'un `debug_assert` (`context.rs:2399`) | `TileLimits::for_texture_side(ctx.input(\|i\| i.max_texture_side))` : tuiles <= 1024, pages entières <= 4096 et 8 Mpx, jamais plus que la limite du GPU |
| R3 | `ScrollArea::scroll_offset` est réappliqué **à chaque frame où il est passé** (`scroll_area.rs:742-743`) : le passer en permanence figerait le défilement | ne le passer que la frame où une requête ou une ancre est résolue ; sinon ne rien passer |
| R4 | `viewport` donné à la fermeture reflète le décalage *avant* la molette de cette frame (la molette est appliquée dans `end()`) | acceptable pour la sélection de pages (une frame de retard, marge de pré-chargement) ; le suivi de page utilise `out.state.offset` |
| R5 | Identifiants `ui.interact` : un `Id` fixe pour toutes les pages entrerait en collision | `Id::new(("page-interact", page))` |
| R6 | Précision `f32` des positions pour de très longs documents zoomés | plafond `MAX_CONTENT_PX` dans `clamp_zoom` (2.1, A13) |
| R7 | Régression dans un `app.rs` de 2000 lignes | **étape 2 d'abord** : même rendu, mode page unique sur la nouvelle machinerie, comparé par capture S3 avant d'introduire le mode continu |
| R8 | Granularité de l'annulation = une tuile ; une page très lourde peut figer 100 ms ou plus par tuile | tuiles de 1024 px ; mesure en 8.5 ; amélioration possible : `Document::render_regions(page, scale, &[PixelRect])` qui charge la page **une fois** pour 3 à 4 tuiles voisines (aujourd'hui chaque tuile recharge la page : `self.inner.pages().get(index)`) — **à n'ajouter que si la mesure montre que le chargement domine** |
| R9 | Mémoire GPU : pool 192 Mio + cache moteur 256 Mio + annulations 512 Mio au pire | tout borné ; machine de référence 14 Go ; journal de contrôle (8.5) |
| R10 | Changement d'écran (ppp différent) en cours de session | l'échelle est dans la clé : nouvelles tuiles, anciennes éjectées par LRU |
| R11 | Changement de comportement : mode continu par défaut ; Page suiv. défile d'un écran au lieu de changer de page | indiqué dans CHANGELOG et le guide ; `Flèche droite/gauche` et Origine/Fin conservent le saut de page ; bascule en un geste (F5) |
| R12 | `clippy -D warnings` sur une crate binaire : le code non utilisé est une erreur | chaque étape de 10.2 n'introduit que du code appelé ; constantes ajoutées au moment où elles servent |
| R13 | `Sense::click_and_drag()` rend les pages focalisables (Tab) : après Tab, `memory.focused()` serait `Some` et les touches seraient ignorées (même contrainte qu'aujourd'hui avec le widget unique) | si S9 le montre : `Sense::CLICK \| Sense::DRAG` (non focalisable) |
| R14 | Texture « entière » de 32 Mio : le téléversement GL bloque le thread UI quelques ms | borne 8 Mpx ; à mesurer ; au besoin abaisser `WHOLE_PAGE_MAX_PIXELS` à 4 Mpx |
| R15 | Les tests du worker ne peuvent pas créer plusieurs `Engine` dans un processus (A9) | les tests UI sont purs (`WantedQueue`, `view.rs`) ; aucun test du crate bureau ne charge PDFium |

### 10.2 Ordre d'implémentation (chaque étape compile, passe clippy et ses tests)

| Étape | Contenu | Livrable vérifiable | Taille |
|---|---|---|---|
| **0** | `make_fixtures.py` : `many_pages` + `--many` ; générer et versionner `many-pages.pdf` ; **captures de référence de l'ancienne version** (`ref-single.png` et variantes, voir S3) avant toute modification de l'interface | `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures` ; trois PNG de référence conservés hors dépôt | S |
| **1** | **Moteur** : `tiles.rs`, `render_region`, `cache.rs` (clé de tuile), tests T-REG-1..7, tests unitaires 8.2, CLI `--region` | `cargo test -p sunipdf-core -j 2` ; commande CLI de la section 6 ; l'application est inchangée | M |
| **2** | **Protocole + page unique sur tuiles** : `worker.rs` (`SetWanted`/`Tile`, `WantedQueue`, `pages_unchanged`), `view.rs` (`Layout` à une page, `TexturePool`, `wanted_jobs`, `ViewState`, `paint_slot`, `handle_page_event`), `app.rs` (suppression de l'ancien état, `document_area` -> `document_view`), `tabs.rs`, `shell.rs` (barre d'état), `Settings` inchangé ; *pas encore de mode continu* : `ViewState` n'a alors ni `mode`, ni `nav_lock`, ni `follow_thumb` (étape 3), ni `wheel_anchor`, `settle_at`, `k_prev`, `vw_prev`, `first_prev` (étape 4), pour ne laisser aucun code mort sous `-D warnings` ; le rendu est redemandé immédiatement après un changement d'échelle, comme aujourd'hui | capture S3 identique à la référence ; S4 sans panique à fort zoom ; `cargo test -p sunipdf-desktop -j 2` | L |
| **3** | **Mode continu** : `ViewMode`, fenêtre `0..n`, suivi de page + verrou, `go_to_page`/`ScrollRequest`, commandes + registre + icônes + i18n + `Settings.continuous_view` + bouton du rail + menu | S1, S2, S8, S9 ; tests `Layout`, `lock_still_valid`, registre | M |
| **4** | **Zoom** : ancre, Ctrl+molette / pincement, anti-rebond, `ZOOM_STEPS` étendu, `clamp_zoom`, `status.zoom_limit` ; `zoom_step` (app.rs:288) part de `Zoom::Fixed(z)` quand il est actif (sinon de `effective_zoom`) pour que des `view.zoom_in` enchaînés dans une même frame s'accumulent | S4, S10 ; test d'ancre ; vérification manuelle Ctrl+molette | M |
| **5** | **Clavier, recherche, miniatures** : Page suiv./préc./Espace/flèches, `go_to_page_point` pour la recherche, `paint_matches` par recherche dichotomique, miniatures bornées et synchronisées | S5, S9, S11 | S |
| **6** | **Robustesse** : onglets (`scroll`), tuiles périmées pour les éditions de contenu, glissement forcé visible, journaux de mesure | S6, S7 ; changement d'onglet conserve le défilement ; retouche d'objet sans page blanche | S |
| **7** | **Mesures et documentation** : 8.5, mise à jour des documents (section 11), option `-Wheel` du script (facultatif) | tableau de mesures dans `PROJECT_STATUS.md` | S |

**Estimation globale : L** (étape 2 = le gros morceau ; les étapes 1, 3 et 4 sont de taille moyenne ; 0, 5, 6 et 7 sont petites).
Aucune étape ne dépend d'une autre tranche du backlog. Les tranches qui modifient `app.rs::handle_input`, `EditOp` ou
`command.rs` en parallèle (édition avancée, QA) devront être rebasées sur l'étape 2 : prévoir de la fusionner tôt.

---

## 11. Documentation à mettre à jour

| Document | Modification |
|---|---|
| `docs/ARCHITECTURE.md` | schéma « ouvrir → afficher → naviguer » : remplacer `Request::Render{page,scale,gen}` / `Response::Rendered` par `SetWanted` / `Tile` et l'époque ; nouvelle section « Vue continue et tuiles » (fenêtre de pages, `Layout`, pool, annulation par remplacement) ; ligne `sunipdf-render` : « tuiles et cache dans `sunipdf-core::{tiles, cache}` » ; supprimer « un seul rendu en vol » et la phrase sur le plafond de 16 384 px |
| `docs/DECISIONS.md` | **D021** (2026-10-14) : vue continue = fenêtre de pages sur une seule machinerie de tuiles ; annulation par remplacement de liste ; tuiles de 1024 px alignées en (0,0), pages entières jusqu'à 4096 px / 8 Mpx ; ancre de zoom (page, fraction) ; mode continu par défaut ; crate `sunipdf-render` non créé. *Renuméroter si une autre tranche a pris D021.* |
| `docs/KNOWN_ISSUES.md` | **A2 résolu** (vue continue livrée) ; **A3 résolu** (plafond 16 384 px levé pour l'écran ; reste vrai pour `render_page` et la CLI sans `--region`) ; ajouter **A11** (sélection de texte limitée à une page, pas de défilement automatique, pincement non testé, rendu en cours non interruptible), **A12** (« Ajuster à la page » = plus grande page en continu), **A13** (précision `f32` / plafond de zoom), et la correction du plantage `max_texture_side` |
| `docs/NEXT_TASKS.md` | SUNI-012 : retirer « vue continue » (livrée), conserver « double page, rotation de vue, sélection multi-pages » en tâches séparées |
| `docs/PROJECT_STATUS.md` | ajouter `SUNI-012b Vue continue + tuiles` (cases cochées seulement après validation réelle), tableau de mesures (8.5) avec machine et commande |
| `docs/FEATURE_MATRIX.md` | l.19 « Affichage continu » -> **Fait** (double page, rotation de vue : planifié) ; l.20 « Rendu en tuiles » -> **Fait** (cache LRU 256 Mio, tuiles 1024 px) |
| `docs/ROADMAP.md` | l.23 et l.26 : retirer « affichage continu » et « tuiles » des « Reste » |
| `docs/USER_GUIDE.fr.md` / `.en.md` | section Affichage : défilement continu / page unique (F5), Ctrl + molette (zoom autour du pointeur), Espace / Page suiv. / flèches, zoom jusqu'à 3200 %, limites (sélection de texte limitée à une page) |
| `docs/I18N.md` | rien (clés sous `menu.view.*`, `shortcut.*`, `status.*` existants) |
| `CHANGELOG.md` | entrée « Vue continue, zoom ancré, tuiles ; correction d'un plantage à fort zoom sur GPU limité ; Page suiv. défile d'un écran en mode continu » |
| `CLAUDE.md` (liste « Modules existants ») | ajouter `view.rs` (vue continue, tuiles) à la description de `apps/sunipdf-desktop` |
