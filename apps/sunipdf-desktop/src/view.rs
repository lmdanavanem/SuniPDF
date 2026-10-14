//! Vue du document : pages empilées (défilement continu) ou page unique, sur une seule machinerie.
//! Les pages sont rendues en tuiles par le thread moteur (`Request::SetWanted`) et virtualisées :
//! seules les pages visibles (plus une de chaque côté en pré-chargement) créent des widgets, des
//! demandes de rendu et des textures. La géométrie (`Layout`, zoom, tuiles voulues, pool de
//! textures) est pure et testée sans GPU ni PDFium.

use crate::app::{error_key, file_name, SuniApp, Zoom, SCREEN_PER_PDF_POINT};
use crate::worker::Request;
use egui::{pos2, vec2, Pos2, Rect, Vec2};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use sunipdf_core::{tiles, EngineError, PageSize, PixelRect, RenderedPage, TileJob, TileLimits};
use sunipdf_i18n::{tr, tr_args};

/// Marge autour des pages et intervalle entre deux pages (points logiques).
pub const PAGE_MARGIN: f32 = 16.0;
pub const PAGE_GAP: f32 = 12.0;
/// Zoom minimal à la molette ; la liste du rail commence à 25 %.
pub const ZOOM_MIN: f32 = 0.1;
/// 3200 %.
pub const ZOOM_MAX: f32 = 32.0;
/// Hauteur totale logique maximale (précision des positions en `f32`).
pub const MAX_CONTENT_PX: f32 = 8_000_000.0;
/// Défilement clavier : une ligne (flèches) et une fraction d'écran (Page suiv., Espace).
pub const LINE_SCROLL_PX: f32 = 48.0;
pub const SCREEN_SCROLL: f32 = 0.9;
/// Miniatures gardées au plus (au-delà, seules celles proches de la page courante restent).
pub const THUMB_KEEP: usize = 256;
/// Textures GPU des pages, hors textures dessinées dans l'image courante.
const POOL_BUDGET_BYTES: usize = 192 * 1024 * 1024;
/// Longueur maximale d'une liste de tuiles voulues.
const MAX_WANTED: usize = 24;
/// Pages voisines pré-chargées de chaque côté de la zone visible.
const PREFETCH_PAGES: usize = 1;
/// Silence après un changement d'échelle avant de redemander des tuiles (zoom en cours).
const SETTLE_SECS: f64 = 0.15;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Single,
    Continuous,
}

/// Défilement demandé, résolu à la prochaine image où la vue document est dessinée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollRequest {
    /// Haut de la page en haut de la zone visible.
    Page(usize),
    /// Point de la page au tiers supérieur, s'il n'est pas déjà bien visible (recherche).
    PagePoint {
        page: usize,
        x_pt: f32,
        y_from_top_pt: f32,
    },
    /// Décalage absolu (restauration d'un onglet).
    Offset(Vec2),
    /// Défilement relatif en points logiques (clavier).
    By(f32),
}

/// Page courante verrouillée après un « aller à » : le suivi au défilement reprend dès que le
/// décalage s'écarte de celui obtenu (l'utilisateur fait défiler).
#[derive(Clone, Copy, Debug)]
pub struct NavLock {
    expected_y: Option<f32>,
}

pub fn lock_still_valid(expected: Option<f32>, actual: f32) -> bool {
    expected.is_none_or(|e| (actual - e).abs() <= 1.0)
}

pub struct ViewState {
    pub mode: ViewMode,
    pub pool: TexturePool<egui::TextureHandle>,
    /// Incrémentée à chaque changement de document (ouverture, onglet, édition) : les tuiles d'une
    /// autre époque sont ignorées.
    pub epoch: u64,
    pending: Option<ScrollRequest>,
    nav_lock: Option<NavLock>,
    /// Dernier décalage connu de la zone de défilement (ancre de zoom, onglets).
    pub offset: Vec2,
    inner: Rect,
    k_prev: Option<f32>,
    avail_prev: Vec2,
    first_prev: usize,
    wheel_anchor: Option<Pos2>,
    settle_at: Option<f64>,
    sent: Vec<TileJob>,
    failed: HashSet<TileJob>,
    missing_visible: usize,
    hover_page: Option<usize>,
    /// La miniature de la page courante doit être ramenée dans le panneau.
    pub follow_thumb: bool,
    /// (décalage, hauteur) du panneau des miniatures à l'image précédente.
    pub thumb_view: Option<(f32, f32)>,
}

impl ViewState {
    pub fn new(continuous: bool) -> Self {
        Self {
            mode: if continuous {
                ViewMode::Continuous
            } else {
                ViewMode::Single
            },
            pool: TexturePool::default(),
            epoch: 0,
            pending: None,
            nav_lock: None,
            offset: Vec2::ZERO,
            inner: Rect::NOTHING,
            k_prev: None,
            avail_prev: Vec2::ZERO,
            first_prev: 0,
            wheel_anchor: None,
            settle_at: None,
            sent: Vec::new(),
            failed: HashSet::new(),
            missing_visible: 0,
            hover_page: None,
            follow_thumb: false,
            thumb_view: None,
        }
    }

    /// Le moteur a vidé sa liste (`invalidate`) : tout est à redemander.
    pub fn bump_epoch(&mut self) {
        self.epoch += 1;
        self.sent.clear();
        self.failed.clear();
    }

    /// Ouverture, fermeture, changement d'onglet. Le mode (préférence) est conservé ; les
    /// textures sont libérées (GPU des onglets inactifs).
    pub fn reset(&mut self) {
        self.pool.clear();
        self.bump_epoch();
        self.pending = None;
        self.nav_lock = None;
        self.offset = Vec2::ZERO;
        self.k_prev = None;
        self.settle_at = None;
        self.wheel_anchor = None;
        self.missing_visible = 0;
        self.thumb_view = None;
    }

    pub fn request(&mut self, r: ScrollRequest) {
        self.pending = Some(r);
    }

    /// Défilement relatif ; les appuis d'une même image s'additionnent.
    pub fn scroll_by(&mut self, dy: f32) {
        self.nav_lock = None;
        self.pending = Some(match self.pending {
            Some(ScrollRequest::By(d)) => ScrollRequest::By(d + dy),
            _ => ScrollRequest::By(dy),
        });
    }

    /// Hauteur de la zone visible à l'image précédente (défilement d'un écran).
    pub fn screen_height(&self) -> f32 {
        if self.inner.is_positive() {
            self.inner.height()
        } else {
            0.0
        }
    }

    /// Des tuiles visibles attendent leur rendu (barre d'état).
    pub fn is_rendering(&self) -> bool {
        self.missing_visible > 0
    }

    /// Après une édition : les textures restent affichées (périmées) si les pages n'ont pas
    /// changé, sinon elles sont jetées ; si les tailles changent, la page courante revient en haut.
    pub fn on_document_changed(&mut self, pages_unchanged: bool, same_sizes: bool, page: usize) {
        if pages_unchanged {
            self.pool.mark_stale();
        } else {
            self.pool.clear();
        }
        self.bump_epoch();
        if !same_sizes {
            self.pending = Some(ScrollRequest::Page(page));
        }
    }
}

// ----- Mise en page (pure) ---------------------------------------------------------------------

/// Position des pages d'une fenêtre (continu : tout le document ; simple : une page) dans le
/// repère du contenu de la zone de défilement.
pub struct Layout {
    pub first: usize,
    pt: Vec<PageSize>,
    tops: Vec<f32>,
    sizes: Vec<Vec2>,
    pub content: Vec2,
}

impl Layout {
    /// `k` : points logiques par point PDF. Un document plus petit que la zone y est centré.
    pub fn new(window: &[PageSize], first: usize, k: f32, viewport: Vec2) -> Self {
        let sizes: Vec<Vec2> = window
            .iter()
            .map(|s| vec2(s.width_pt, s.height_pt) * k)
            .collect();
        let max_w = sizes.iter().map(|s| s.x).fold(0.0, f32::max);
        let mut tops = Vec::with_capacity(sizes.len());
        let mut y = PAGE_MARGIN;
        for s in &sizes {
            tops.push(y);
            y += s.y + PAGE_GAP;
        }
        let total = if sizes.is_empty() {
            2.0 * PAGE_MARGIN
        } else {
            y - PAGE_GAP + PAGE_MARGIN
        };
        let shift = ((viewport.y - total) / 2.0).max(0.0);
        for t in &mut tops {
            *t += shift;
        }
        Self {
            first,
            pt: window.to_vec(),
            tops,
            sizes,
            content: vec2(
                (max_w + 2.0 * PAGE_MARGIN).max(viewport.x),
                total.max(viewport.y),
            ),
        }
    }

    fn idx(&self, page: usize) -> usize {
        (page.saturating_sub(self.first)).min(self.tops.len().saturating_sub(1))
    }

    pub fn pages(&self) -> Range<usize> {
        self.first..self.first + self.tops.len()
    }

    pub fn contains(&self, page: usize) -> bool {
        self.pages().contains(&page)
    }

    pub fn page_size_pt(&self, page: usize) -> PageSize {
        self.pt[self.idx(page)]
    }

    pub fn size(&self, page: usize) -> Vec2 {
        self.sizes[self.idx(page)]
    }

    pub fn page_top(&self, page: usize) -> f32 {
        self.tops[self.idx(page)]
    }

    pub fn page_left(&self, page: usize) -> f32 {
        (self.content.x - self.size(page).x) / 2.0
    }

    /// Rectangle de la page dans le repère du contenu.
    pub fn page_rect(&self, page: usize) -> Rect {
        Rect::from_min_size(
            pos2(self.page_left(page), self.page_top(page)),
            self.size(page),
        )
    }

    /// Pages (indices absolus) qui coupent la bande `[y0, y1]` (recherche dichotomique).
    pub fn visible(&self, y0: f32, y1: f32) -> Range<usize> {
        // La dernière page qui commence avant y0 peut encore couper la bande ; on avance tant que
        // la page finit avant y0 (au plus une ou deux pages).
        let mut start = self.tops.partition_point(|&t| t < y0).saturating_sub(1);
        while start < self.tops.len() && self.tops[start] + self.sizes[start].y <= y0 {
            start += 1;
        }
        let end = self.tops.partition_point(|&t| t < y1).max(start);
        self.first + start..self.first + end
    }

    /// Page qui occupe la plus grande hauteur visible (égalité : la plus petite) ; dans un
    /// intervalle sans page, la plus proche.
    pub fn dominant(&self, y0: f32, y1: f32) -> usize {
        let mut best: Option<(usize, f32)> = None;
        for page in self.visible(y0, y1) {
            let (top, h) = (self.page_top(page), self.size(page).y);
            let seen = (top + h).min(y1) - top.max(y0);
            if best.is_none_or(|(_, b)| seen > b) {
                best = Some((page, seen));
            }
        }
        best.map_or_else(|| self.anchor_at((y0 + y1) / 2.0).0, |(p, _)| p)
    }

    /// (page, fraction verticale) du point `y` du contenu ; hors page, la page la plus proche
    /// (fraction < 0 ou > 1).
    pub fn anchor_at(&self, y: f32) -> (usize, f32) {
        if self.tops.is_empty() {
            return (self.first, 0.0);
        }
        let i = self.tops.partition_point(|&t| t <= y).saturating_sub(1);
        let i = if i + 1 < self.tops.len() {
            // Dans l'intervalle sous la page i : la plus proche des deux.
            let bottom = self.tops[i] + self.sizes[i].y;
            if y > bottom && (self.tops[i + 1] - y) < (y - bottom) {
                i + 1
            } else {
                i
            }
        } else {
            i
        };
        (
            self.first + i,
            (y - self.tops[i]) / self.sizes[i].y.max(f32::EPSILON),
        )
    }

    pub fn y_of(&self, page: usize, frac: f32) -> f32 {
        self.page_top(page) + frac * self.size(page).y
    }
}

/// Zoom affiché : page de référence = page courante (page unique) ou plus grande largeur et plus
/// grande hauteur du document (continu : le zoom ne change pas en défilant).
pub fn effective_zoom(
    zoom: Zoom,
    mode: ViewMode,
    pages: &[PageSize],
    current: usize,
    avail: Vec2,
) -> f32 {
    let (w, h) = match mode {
        ViewMode::Single => pages
            .get(current)
            .map_or((612.0, 792.0), |s| (s.width_pt, s.height_pt)),
        ViewMode::Continuous => pages.iter().fold((1.0f32, 1.0f32), |(w, h), s| {
            (w.max(s.width_pt), h.max(s.height_pt))
        }),
    };
    let fit_w = (avail.x - 2.0 * PAGE_MARGIN) / (w * SCREEN_PER_PDF_POINT);
    match zoom {
        Zoom::FitWidth => fit_w.max(0.05),
        Zoom::FitPage => fit_w
            .min((avail.y - 2.0 * PAGE_MARGIN) / (h * SCREEN_PER_PDF_POINT))
            .max(0.05),
        Zoom::Fixed(z) => z,
    }
}

/// Plafonne le zoom : 3200 %, côté de page rendue <= `MAX_TILED_SIDE_PX`, hauteur totale du
/// contenu <= `MAX_CONTENT_PX` (continu). Renvoie aussi « limité ».
pub fn clamp_zoom(z: f32, pages: &[PageSize], ppp: f32, mode: ViewMode) -> (f32, bool) {
    let side = pages
        .iter()
        .map(|s| s.width_pt.max(s.height_pt))
        .fold(1.0f32, f32::max);
    let mut hi =
        ZOOM_MAX.min(tiles::MAX_TILED_SIDE_PX as f32 / (side * SCREEN_PER_PDF_POINT * ppp));
    if mode == ViewMode::Continuous {
        let total: f32 = pages.iter().map(|s| s.height_pt).sum::<f32>().max(1.0);
        hi = hi.min(MAX_CONTENT_PX / (total * SCREEN_PER_PDF_POINT));
    }
    // ponytail: marges et intervalles ignorés dans la borne de hauteur (quelques px par page).
    let clamped = z.clamp(0.05f32.min(hi), hi);
    (clamped, clamped < z - 1e-4)
}

/// Pixels de la page (rendue en `page_px`) couverts par `local` (repère de la page, points
/// logiques, page de taille `page_size`).
fn to_pixels(local: Rect, page_size: Vec2, page_px: (u32, u32)) -> Option<PixelRect> {
    let sx = page_px.0 as f32 / page_size.x.max(f32::EPSILON);
    let sy = page_px.1 as f32 / page_size.y.max(f32::EPSILON);
    let x0 = (local.min.x * sx).floor().max(0.0) as u32;
    let y0 = (local.min.y * sy).floor().max(0.0) as u32;
    let x1 = ((local.max.x * sx).ceil().max(0.0) as u32).min(page_px.0);
    let y1 = ((local.max.y * sy).ceil().max(0.0) as u32).min(page_px.1);
    (x1 > x0 && y1 > y0).then(|| PixelRect::new(x0, y0, x1 - x0, y1 - y0))
}

/// Rectangle (même repère que `page_rect`) d'une région de la page rendue en `page_px`.
pub fn tile_screen_rect(page_rect: Rect, page_px: (u32, u32), r: PixelRect) -> Rect {
    let s = page_rect.size() / vec2(page_px.0 as f32, page_px.1 as f32);
    Rect::from_min_max(
        page_rect.min + vec2(r.x as f32, r.y as f32) * s,
        page_rect.min + vec2(r.right() as f32, r.bottom() as f32) * s,
    )
}

/// Entrées de la planification des tuiles d'une image.
pub struct PlanInput<'a> {
    pub layout: &'a Layout,
    pub scale_milli: u32,
    /// Zone visible, repère du contenu.
    pub viewport: Rect,
    /// Pages dessinées même hors de l'écran (glissement, éditeur de texte en cours).
    pub pinned: &'a [usize],
    pub limits: TileLimits,
}

/// Tuiles à demander, par priorité : pages visibles (zone élargie d'une demi-tuile, du centre de
/// l'écran vers les bords), puis le bord proche des pages voisines. Sans les tuiles déjà là ou
/// en échec (`skip`), au plus `MAX_WANTED`.
pub fn wanted_jobs(input: &PlanInput, skip: &dyn Fn(&TileJob) -> bool) -> Vec<TileJob> {
    let l = input.layout;
    let vp = input.viewport;
    let scale = input.scale_milli as f32 / 1000.0;
    let center = vp.center();
    let mut out: Vec<(f32, TileJob)> = Vec::new();
    let add = |page: usize, area: Rect, base: f32, out: &mut Vec<(f32, TileJob)>| {
        let rect = l.page_rect(page);
        let Some(page_px) = tiles::page_pixels(l.page_size_pt(page), scale) else {
            return;
        };
        let local = area.intersect(rect).translate(-rect.min.to_vec2());
        if !local.is_positive() {
            return;
        }
        let Some(view) = to_pixels(local, rect.size(), page_px) else {
            return;
        };
        for region in tiles::plan_tiles(page_px, view, &input.limits) {
            let job = TileJob {
                page,
                scale_milli: input.scale_milli,
                region,
            };
            if !skip(&job) {
                let c = tile_screen_rect(rect, page_px, region).center();
                out.push((base + c.distance(center), job));
            }
        }
    };
    let visible = l.visible(vp.min.y, vp.max.y);
    // Demi-tuile (pixels de la page) en points PDF, puis en points logiques (× k).
    let half_tile_pt = (input.limits.tile_side as f32 / 2.0) / scale.max(f32::EPSILON);
    for page in visible.clone() {
        let k = l.size(page).x / l.page_size_pt(page).width_pt.max(f32::EPSILON);
        add(page, vp.expand(half_tile_pt * k), 0.0, &mut out);
    }
    for &page in input.pinned {
        if l.contains(page) && !visible.contains(&page) {
            add(page, l.page_rect(page), 0.0, &mut out);
        }
    }
    let h = vp.height();
    for d in 1..=PREFETCH_PAGES {
        let base = 1e9 * d as f32;
        if let Some(before) = visible.start.checked_sub(d).filter(|p| l.contains(*p)) {
            let r = l.page_rect(before);
            add(
                before,
                Rect::from_min_max(pos2(r.min.x, r.max.y - h), r.max),
                base,
                &mut out,
            );
        }
        let after = visible.end + d - 1;
        if l.contains(after) && !visible.contains(&after) {
            let r = l.page_rect(after);
            add(
                after,
                Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + h)),
                base,
                &mut out,
            );
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut seen = HashSet::new();
    out.into_iter()
        .map(|(_, j)| j)
        .filter(|j| seen.insert(*j))
        .take(MAX_WANTED)
        .collect()
}

/// Faut-il envoyer `jobs` au moteur ? Oui si une tuile nouvelle apparaît, ou si une tuile envoyée
/// n'est plus voulue sans être arrivée (annulation). Une tuile arrivée ne justifie pas d'envoi.
pub fn should_send(sent: &[TileJob], jobs: &[TileJob], arrived: &dyn Fn(&TileJob) -> bool) -> bool {
    jobs.iter().any(|j| !sent.contains(j)) || sent.iter().any(|j| !jobs.contains(j) && !arrived(j))
}

// ----- Pool de textures (pur, générique pour être testé sans GPU) ------------------------------

pub struct TileTex<H> {
    pub scale_milli: u32,
    pub region: PixelRect,
    pub page_px: (u32, u32),
    pub handle: H,
    pub bytes: usize,
    pub used: u64,
    /// Contenu antérieur à une édition : affiché en repli jusqu'à son remplacement.
    pub stale: bool,
}

impl<H> TileTex<H> {
    /// Zone couverte en fraction de la page (0..1).
    fn norm(&self) -> Rect {
        let (w, h) = (self.page_px.0 as f32, self.page_px.1 as f32);
        Rect::from_min_max(
            pos2(self.region.x as f32 / w, self.region.y as f32 / h),
            pos2(
                self.region.right() as f32 / w,
                self.region.bottom() as f32 / h,
            ),
        )
    }
}

/// Textures des pages, bornées en octets ; éviction LRU sans jamais retirer une texture
/// dessinée dans l'image courante.
pub struct TexturePool<H> {
    pages: HashMap<usize, Vec<TileTex<H>>>,
    bytes: usize,
    frame: u64,
}

impl<H> Default for TexturePool<H> {
    fn default() -> Self {
        Self {
            pages: HashMap::new(),
            bytes: 0,
            frame: 0,
        }
    }
}

impl<H> TexturePool<H> {
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Ajoute ou remplace (même échelle et même région, périmée ou non).
    pub fn insert(&mut self, page: usize, mut tex: TileTex<H>) {
        tex.used = self.frame;
        self.bytes += tex.bytes;
        let list = self.pages.entry(page).or_default();
        match list
            .iter()
            .position(|t| t.scale_milli == tex.scale_milli && t.region == tex.region)
        {
            Some(i) => self.bytes -= std::mem::replace(&mut list[i], tex).bytes,
            None => list.push(tex),
        }
    }

    /// Texture à jour exactement pour `job` ; la marque utilisée dans cette image.
    pub fn exact(&mut self, job: &TileJob) -> Option<&H> {
        let frame = self.frame;
        let t = self
            .pages
            .get_mut(&job.page)?
            .iter_mut()
            .find(|t| !t.stale && t.scale_milli == job.scale_milli && t.region == job.region)?;
        t.used = frame;
        Some(&t.handle)
    }

    pub fn has_fresh(&self, job: &TileJob) -> bool {
        self.pages.get(&job.page).is_some_and(|l| {
            l.iter()
                .any(|t| !t.stale && t.scale_milli == job.scale_milli && t.region == job.region)
        })
    }

    /// Replis de la page qui coupent `area` (fraction de page) : autres échelles et textures
    /// périmées, de la plus éloignée de `scale_milli` à la plus proche (la plus proche est
    /// dessinée en dernier, donc au-dessus).
    pub fn fallbacks(&mut self, page: usize, scale_milli: u32, area: Rect) -> Vec<(Rect, &H)> {
        let frame = self.frame;
        let Some(list) = self.pages.get_mut(&page) else {
            return Vec::new();
        };
        let mut v: Vec<&mut TileTex<H>> = list
            .iter_mut()
            .filter(|t| (t.stale || t.scale_milli != scale_milli) && t.norm().intersects(area))
            .collect();
        v.sort_by_key(|t| std::cmp::Reverse(t.scale_milli.abs_diff(scale_milli)));
        v.into_iter()
            .map(|t| {
                t.used = frame;
                (t.norm(), &t.handle)
            })
            .collect()
    }

    /// Une texture couvrant toute la page (aperçu du recadrage), la plus nette d'abord.
    pub fn whole(&self, page: usize) -> Option<&H> {
        self.pages
            .get(&page)?
            .iter()
            .filter(|t| t.region == PixelRect::new(0, 0, t.page_px.0, t.page_px.1))
            .max_by_key(|t| (!t.stale, t.scale_milli))
            .map(|t| &t.handle)
    }

    pub fn mark_stale(&mut self) {
        for t in self.pages.values_mut().flatten() {
            t.stale = true;
        }
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.bytes = 0;
    }

    /// Retire les entrées les moins récemment dessinées jusqu'à `budget` octets, jamais une
    /// entrée dessinée dans l'image courante : la borne est `budget` + textures de l'écran.
    pub fn evict(&mut self, budget: usize) {
        while self.bytes > budget {
            let victim = self
                .pages
                .iter()
                .flat_map(|(p, l)| {
                    l.iter()
                        .enumerate()
                        .filter(|(_, t)| t.used < self.frame)
                        .map(move |(i, t)| (t.used, *p, i))
                })
                .min();
            let Some((_, page, i)) = victim else { break };
            let list = self.pages.get_mut(&page).expect("page du pool");
            self.bytes -= list.swap_remove(i).bytes;
            if list.is_empty() {
                self.pages.remove(&page);
            }
        }
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.pages.values().map(Vec::len).sum()
    }
}

// ----- Interface --------------------------------------------------------------------------------

pub(crate) enum DragEvent {
    Start((f32, f32)),
    Move((f32, f32)),
    Stop,
    Click((f32, f32)),
    DoubleClick((f32, f32)),
}

/// Une page à dessiner dans l'image courante.
struct Slot<'a> {
    page: usize,
    /// Rectangle écran de la page.
    rect: Rect,
    size_pt: PageSize,
    page_px: Option<(u32, u32)>,
    scale_milli: u32,
    limits: TileLimits,
    current: bool,
    total: usize,
    name: &'a str,
}

/// Aligne le coin haut-gauche sur la grille des pixels physiques (1 texel = 1 pixel).
fn snapped(r: Rect, ppp: f32) -> Rect {
    let snap = |v: f32| (v * ppp).round() / ppp;
    Rect::from_min_size(pos2(snap(r.min.x), snap(r.min.y)), r.size())
}

impl SuniApp {
    /// Pages dessinées même hors de l'écran : celle d'un glissement en cours (sinon le widget
    /// disparaîtrait au défilement) et celle de l'éditeur de texte posé sur la page.
    fn pinned_pages(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.annotate.drag.map(|d| d.page).into_iter().collect();
        if let Some(p) = self.edit.text_prompt.as_ref().filter(|p| p.inline) {
            v.push(p.page);
        }
        v
    }

    /// Zone document : fenêtre de pages, zoom, défilement, tuiles, interactions.
    pub fn document_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(doc) = &self.document else { return };
        let sizes = doc.info.page_sizes.clone();
        let count = sizes.len();
        if count == 0 {
            return;
        }
        let current = doc.page.min(count - 1);
        let name = file_name(&doc.info.path);
        let mode = self.view.mode;
        let window = match mode {
            ViewMode::Continuous => 0..count,
            ViewMode::Single => current..current + 1,
        };
        let ppp = ctx.pixels_per_point();
        let avail = ui.available_size();
        let doc_rect = ui.max_rect();
        let now = ctx.input(|i| i.time);
        let limits = TileLimits::for_texture_side(ctx.input(|i| i.max_texture_side));
        self.view.pool.begin_frame();
        self.view.missing_visible = 0;
        self.edit.page_view = None;
        self.refresh_annotations_if_needed();

        // 1. Zoom (molette ancrée sur le pointeur), plafonné.
        self.apply_wheel_zoom(ctx);
        let wanted = effective_zoom(self.zoom, mode, &sizes, current, avail);
        let (z, limited) = clamp_zoom(wanted, &sizes, ppp, mode);
        if limited && matches!(self.zoom, Zoom::Fixed(_)) {
            self.zoom = Zoom::Fixed(z);
            self.status_msg = Some(tr(self.lang(), "status.zoom_limit").to_string());
        }
        self.effective_zoom = z;
        let k = z * SCREEN_PER_PDF_POINT;
        if self.view.k_prev.is_some_and(|p| (p - k).abs() > 1e-4) {
            self.view.settle_at = Some(now + SETTLE_SECS);
        }
        let scale_milli = tiles::quantize_milli(k * ppp);
        let scale = scale_milli as f32 / 1000.0;

        // 2. Mise en page, 3. décalage imposé (requête, ancre de zoom) seulement cette image.
        let layout = Layout::new(&sizes[window.clone()], window.start, k, avail);
        let forced = self.resolve_scroll(&layout, &sizes[window.clone()], avail, k);
        let pinned = self.pinned_pages();

        // 4. Zone de défilement : seules les pages visibles sont dessinées.
        let mut area = egui::ScrollArea::both()
            .id_salt("doc-scroll")
            .auto_shrink([false, false]);
        if let Some(o) = forced {
            area = area.scroll_offset(o);
        }
        let mut event = None;
        self.view.hover_page = None;
        let out = area.show_viewport(ui, |ui, viewport| {
            let (rect, _) = ui.allocate_exact_size(layout.content, egui::Sense::hover());
            let mut pages: Vec<usize> = layout.visible(viewport.min.y, viewport.max.y).collect();
            for &p in &pinned {
                if layout.contains(p) && !pages.contains(&p) {
                    pages.push(p);
                }
            }
            for page in pages {
                let size_pt = layout.page_size_pt(page);
                let slot = Slot {
                    page,
                    rect: snapped(layout.page_rect(page).translate(rect.min.to_vec2()), ppp),
                    size_pt,
                    page_px: tiles::page_pixels(size_pt, scale),
                    scale_milli,
                    limits,
                    current: page == current,
                    total: count,
                    name: &name,
                };
                if let Some(e) = self.paint_slot(ctx, ui, &slot) {
                    event.get_or_insert(e);
                }
            }
        });

        // 5. Après la zone : état, page courante, tuiles voulues, mémoire.
        let viewport = Rect::from_min_size(out.state.offset.to_pos2(), out.inner_rect.size());
        let v = &mut self.view;
        v.offset = out.state.offset;
        v.inner = out.inner_rect;
        v.k_prev = Some(k);
        v.avail_prev = avail;
        v.first_prev = layout.first;
        v.wheel_anchor = None;
        if let Some(lock) = &mut v.nav_lock {
            if forced.is_some() || lock.expected_y.is_none() {
                lock.expected_y = Some(out.state.offset.y);
            } else if !lock_still_valid(lock.expected_y, out.state.offset.y) {
                v.nav_lock = None;
            }
        }
        if mode == ViewMode::Continuous && self.view.nav_lock.is_none() {
            self.track_page(layout.dominant(viewport.min.y, viewport.max.y));
        }
        // Outil Sélection d'objet sans panneau Objets : objets de la page survolée (prise directe
        // sur n'importe quelle page visible) ; sinon ceux de la page courante.
        let objects_page = match self.view.hover_page {
            Some(p) if !self.edit.objects_open => p,
            _ => self.document.as_ref().map_or(current, |d| d.page),
        };
        self.refresh_objects_if_needed(objects_page);
        self.flush_wanted(
            ctx,
            &PlanInput {
                layout: &layout,
                scale_milli,
                viewport,
                pinned: &pinned,
                limits,
            },
            now,
        );
        self.view.pool.evict(POOL_BUDGET_BYTES);
        if let Some((page, ev)) = event {
            self.handle_page_event(page, ev);
        }
        self.object_bar(ctx, doc_rect);
    }

    /// Ombre, tuiles (ou replis), surcouches et interactions d'une page ; identique en continu et
    /// en page unique.
    fn paint_slot(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        s: &Slot,
    ) -> Option<(usize, DragEvent)> {
        let painter = ui.painter().clone();
        let dark = self.settings.dark_theme;
        painter.add(
            egui::Shadow {
                offset: [0, 2],
                blur: 14,
                spread: 0,
                color: egui::Color32::from_black_alpha(if dark { 120 } else { 46 }),
            }
            .as_shape(s.rect, 2),
        );
        painter.rect_filled(s.rect, 0.0, egui::Color32::WHITE);
        let clip = ui.clip_rect().intersect(s.rect);
        if let (true, Some(page_px)) = (clip.is_positive(), s.page_px) {
            self.paint_tiles(ui, &painter, s, page_px, clip);
        }
        let h = s.size_pt.height_pt;
        self.paint_matches(&painter, s.rect, s.page, h);
        self.paint_selection(&painter, s.rect, s.page, h);
        self.paint_object_selection(&painter, s.rect, s.page, h);
        let on_selection = self.edit.selection.as_ref().map(|sel| sel.page);
        if on_selection.map_or(s.current, |p| p == s.page) {
            self.edit.page_view = Some((s.page, s.rect, h));
        }
        self.inline_text_editor(ctx, s.rect, s.page, h);

        let interact = ui.interact(
            s.rect,
            egui::Id::new(("page-interact", s.page)),
            egui::Sense::click_and_drag(),
        );
        let alt = tr_args(
            self.lang(),
            "a11y.page_image",
            &[
                ("current", &(s.page + 1).to_string()),
                ("total", &s.total.to_string()),
                ("name", s.name),
            ],
        );
        interact.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, &alt));
        let (rect, zoom) = (s.rect, self.effective_zoom);
        let to_pdf = |pos| crate::annotate::to_pdf(pos, rect, zoom, h);
        let mut event = None;
        if interact.drag_started() {
            // Origine réelle de la pression : le glissement n'est détecté qu'après le seuil.
            let origin = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| interact.interact_pointer_pos());
            event = origin.map(|p| DragEvent::Start(to_pdf(p)));
        } else if interact.dragged() {
            event = interact
                .interact_pointer_pos()
                .map(|p| DragEvent::Move(to_pdf(p)));
        } else if interact.drag_stopped() {
            event = Some(DragEvent::Stop);
        } else if interact.double_clicked() {
            event = interact
                .interact_pointer_pos()
                .map(|p| DragEvent::DoubleClick(to_pdf(p)));
        } else if interact.clicked() {
            event = interact
                .interact_pointer_pos()
                .map(|p| DragEvent::Click(to_pdf(p)));
        }
        if interact.hovered() {
            self.view.hover_page = Some(s.page);
        }
        let hover = interact.hover_pos().map(to_pdf);
        interact.on_hover_cursor(self.page_cursor(s.page, hover));
        event.map(|e| (s.page, e))
    }

    /// Tuiles exactes ; en attendant, textures d'une autre échelle ou périmées étirées (pas de page
    /// blanche pendant un zoom ou après une retouche) ; rien du tout : indicateur d'attente.
    fn paint_tiles(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        s: &Slot,
        page_px: (u32, u32),
        clip: Rect,
    ) {
        let local = clip.translate(-s.rect.min.to_vec2());
        let Some(view_px) = to_pixels(local, s.rect.size(), page_px) else {
            return;
        };
        let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        let mut exact = Vec::new();
        let mut missing = 0;
        for region in tiles::plan_tiles(page_px, view_px, &s.limits) {
            let job = TileJob {
                page: s.page,
                scale_milli: s.scale_milli,
                region,
            };
            match self.view.pool.exact(&job) {
                Some(h) => exact.push((tile_screen_rect(s.rect, page_px, region), h.id())),
                None => missing += 1,
            }
        }
        if missing > 0 {
            self.view.missing_visible += missing;
            let size = s.rect.size();
            let area = Rect::from_min_max(
                (local.min.to_vec2() / size).to_pos2(),
                (local.max.to_vec2() / size).to_pos2(),
            );
            let fallbacks: Vec<(Rect, egui::TextureId)> = self
                .view
                .pool
                .fallbacks(s.page, s.scale_milli, area)
                .into_iter()
                .map(|(n, h)| (n, h.id()))
                .collect();
            if fallbacks.is_empty() && exact.is_empty() {
                egui::Spinner::new()
                    .size(26.0)
                    .color(egui::Color32::from_gray(150))
                    .paint_at(ui, Rect::from_center_size(clip.center(), Vec2::splat(26.0)));
            }
            for (n, id) in fallbacks {
                let r = Rect::from_min_max(
                    s.rect.min + n.min.to_vec2() * size,
                    s.rect.min + n.max.to_vec2() * size,
                );
                painter.image(id, r, uv, egui::Color32::WHITE);
            }
        }
        for (r, id) in exact {
            painter.image(id, r, uv, egui::Color32::WHITE);
        }
    }

    /// Résout la requête de défilement en attente, sinon garde le point ancré si l'échelle a
    /// changé (molette : sous le pointeur ; boutons : centre ; ajustements : haut de la zone).
    fn resolve_scroll(
        &mut self,
        layout: &Layout,
        window: &[PageSize],
        avail: Vec2,
        k: f32,
    ) -> Option<Vec2> {
        let v = &mut self.view;
        let off = v.offset;
        let screen = if v.inner.is_positive() {
            v.inner.size()
        } else {
            avail
        };
        // Horizontalement : page centrée si elle tient, sinon décalage gardé dans la page (une
        // page étroite d'un document mixte ne laisse pas l'écran sur la marge grise).
        let x_for = |p: usize, want: f32| {
            let (left, w) = (layout.page_left(p), layout.size(p).x);
            let x = if w <= screen.x {
                left + (w - screen.x) / 2.0
            } else {
                want.clamp(left - PAGE_MARGIN, left + w + PAGE_MARGIN - screen.x)
            };
            x.max(0.0)
        };
        if let Some(req) = v.pending.take() {
            return match req {
                ScrollRequest::Page(p) => {
                    let p = p.clamp(layout.first, layout.pages().end - 1);
                    Some(vec2(
                        x_for(p, off.x),
                        (layout.page_top(p) - PAGE_MARGIN).max(0.0),
                    ))
                }
                ScrollRequest::PagePoint {
                    page,
                    x_pt,
                    y_from_top_pt,
                } => {
                    let p = page.clamp(layout.first, layout.pages().end - 1);
                    let x = layout.page_left(p) + x_pt * k;
                    let y = layout.page_top(p) + y_from_top_pt * k;
                    let seen_y = off.y + 0.1 * screen.y..=off.y + 0.85 * screen.y;
                    let seen_x = off.x..=off.x + 0.9 * screen.x;
                    (!(seen_y.contains(&y) && seen_x.contains(&x)))
                        .then(|| vec2(x_for(p, x - screen.x / 4.0), (y - screen.y / 3.0).max(0.0)))
                }
                ScrollRequest::Offset(o) => Some(o),
                ScrollRequest::By(dy) => Some(vec2(off.x, (off.y + dy).max(0.0))),
            };
        }
        let kp = v.k_prev?;
        if (kp - k).abs() <= 1e-4 || v.first_prev != layout.first || !v.inner.is_positive() {
            return None;
        }
        let old = Layout::new(window, layout.first, kp, v.avail_prev);
        let screen = match (v.wheel_anchor, self.zoom) {
            (Some(p), _) => p - v.inner.min,
            (None, Zoom::Fixed(_)) => v.inner.size() / 2.0,
            (None, _) => vec2(v.inner.width() / 2.0, 0.0),
        };
        let (page, fy) = old.anchor_at(off.y + screen.y);
        let fx = (off.x + screen.x - old.page_left(page)) / old.size(page).x.max(f32::EPSILON);
        Some(
            vec2(
                layout.page_left(page) + fx * layout.size(page).x - screen.x,
                layout.y_of(page, fy) - screen.y,
            )
            .max(Vec2::ZERO),
        )
    }

    /// Ctrl + molette et pincement : zoom autour du pointeur (la molette ne défile pas alors).
    fn apply_wheel_zoom(&mut self, ctx: &egui::Context) {
        let (zd, pos) = ctx.input(|i| (i.zoom_delta(), i.pointer.hover_pos()));
        if (zd - 1.0).abs() < 1e-4 {
            return;
        }
        let Some(pos) = pos.filter(|p| self.view.inner.contains(*p)) else {
            return;
        };
        self.zoom = Zoom::Fixed((self.effective_zoom * zd).clamp(ZOOM_MIN, ZOOM_MAX));
        self.view.wheel_anchor = Some(pos);
    }

    /// Envoie la liste des tuiles voulues si elle a changé ; rien pendant un zoom en cours.
    fn flush_wanted(&mut self, ctx: &egui::Context, input: &PlanInput, now: f64) {
        if let Some(t) = self.view.settle_at {
            if now < t {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(t - now));
                return;
            }
            self.view.settle_at = None;
        }
        let v = &self.view;
        let jobs = wanted_jobs(input, &|j| v.pool.has_fresh(j) || v.failed.contains(j));
        if should_send(&v.sent, &jobs, &|j| v.pool.has_fresh(j)) {
            log::debug!(
                "tuiles voulues : {} ; pool {} textures, {} octets",
                jobs.len(),
                v.pool.len(),
                v.pool.bytes()
            );
            self.worker.send(Request::SetWanted {
                epoch: self.view.epoch,
                jobs: jobs.clone(),
            });
            self.view.sent = jobs;
        }
    }

    /// Page qui occupe l'écran (défilement continu) : champ, barre d'état, miniature, commentaires.
    fn track_page(&mut self, page: usize) {
        if let Some(doc) = &mut self.document {
            if doc.page != page {
                doc.page = page;
                self.page_input = (page + 1).to_string();
                self.view.follow_thumb = true;
            }
        }
    }

    /// Un appui sur une page la rend courante (Pivoter, Supprimer, panneau Objets la visent).
    fn make_current(&mut self, page: usize) {
        if self.view.mode != ViewMode::Continuous
            || self.document.as_ref().is_none_or(|d| d.page == page)
        {
            return;
        }
        self.track_page(page);
        self.view.nav_lock = Some(NavLock {
            expected_y: Some(self.view.offset.y),
        });
    }

    /// Verrouille la page courante après un « aller à » (défilement à la prochaine image).
    pub fn lock_on_page(&mut self, page: usize) {
        self.view.request(ScrollRequest::Page(page));
        self.view.nav_lock = Some(NavLock { expected_y: None });
        self.view.follow_thumb = true;
    }

    /// Aller au point (x, y depuis le haut, en points) d'une page (résultat de recherche) : s'il
    /// n'est pas déjà visible, il est amené au tiers supérieur.
    pub fn go_to_page_point(&mut self, page: usize, x_pt: f32, y_from_top_pt: f32) {
        self.go_to_page(page);
        if let Some(doc) = &self.document {
            let page = doc.page;
            self.view.request(ScrollRequest::PagePoint {
                page,
                x_pt,
                y_from_top_pt,
            });
        }
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view.mode == mode {
            return;
        }
        self.view.mode = mode;
        self.settings.continuous_view = mode == ViewMode::Continuous;
        self.persist_settings();
        if let Some(page) = self.document.as_ref().map(|d| d.page) {
            self.lock_on_page(page);
        }
    }

    pub fn on_tile(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        job: TileJob,
        image: Arc<RenderedPage>,
    ) {
        if epoch != self.view.epoch || self.view.pool.has_fresh(&job) {
            return;
        }
        let Some(page_px) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(job.page).copied())
            .and_then(|size| tiles::page_pixels(size, job.scale()))
        else {
            return;
        };
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [image.width as usize, image.height as usize],
            &image.rgba,
        );
        let r = job.region;
        let handle = ctx.load_texture(
            format!("tile-{}-{}-{}-{}", job.page, job.scale_milli, r.x, r.y),
            color,
            egui::TextureOptions::LINEAR,
        );
        self.view.pool.insert(
            job.page,
            TileTex {
                scale_milli: job.scale_milli,
                region: job.region,
                page_px,
                handle,
                bytes: image.rgba.len(),
                used: 0,
                stale: false,
            },
        );
    }

    pub fn on_tile_failed(&mut self, epoch: u64, job: TileJob, error: EngineError) {
        if epoch != self.view.epoch {
            return;
        }
        log::warn!("tuile {job:?} impossible : {error}");
        if self.view.failed.is_empty() {
            self.error = Some(error_key(&error));
        }
        self.view.failed.insert(job);
    }

    /// Un événement de souris sur une page ; un seul glissement à la fois.
    fn handle_page_event(&mut self, page: usize, ev: DragEvent) {
        if matches!(
            ev,
            DragEvent::Start(_) | DragEvent::Click(_) | DragEvent::DoubleClick(_)
        ) {
            self.make_current(page);
        }
        match ev {
            DragEvent::Start(pt) => {
                self.edit_drag_start(page, pt);
                self.annotate.drag = Some(crate::annotate::Drag {
                    page,
                    start: pt,
                    current: pt,
                });
                self.annotate.selection = None;
            }
            DragEvent::Move(pt) => {
                if let Some(d) = &mut self.annotate.drag {
                    d.current = pt;
                    let d = *d;
                    self.edit_drag_moved(&d);
                }
            }
            DragEvent::Stop => {
                if let Some(d) = self.annotate.drag.take() {
                    self.finish_drag(d);
                }
            }
            DragEvent::Click(pt) => {
                self.annotate.selection = None;
                match self.annotate.tool {
                    crate::annotate::Tool::Note
                    | crate::annotate::Tool::AddText
                    | crate::annotate::Tool::AddImage => {
                        self.finish_drag(crate::annotate::Drag {
                            page,
                            start: pt,
                            current: pt,
                        });
                    }
                    crate::annotate::Tool::SelectObject => self.edit_click(page, pt),
                    _ => {}
                }
            }
            DragEvent::DoubleClick(pt) => {
                if matches!(self.annotate.tool, crate::annotate::Tool::SelectObject) {
                    self.edit_double_click(page, pt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter() -> PageSize {
        PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        }
    }

    fn landscape() -> PageSize {
        PageSize {
            width_pt: 792.0,
            height_pt: 612.0,
        }
    }

    #[test]
    fn layout_stacks_and_centers_pages() {
        let l = Layout::new(&[letter(); 3], 0, 1.0, vec2(1000.0, 0.0));
        assert_eq!(l.tops, [16.0, 820.0, 1624.0]);
        assert_eq!(l.content, vec2(1000.0, 2432.0));
        assert_eq!(l.page_left(1), 194.0);
        // Tailles mixtes : hauts strictement croissants, pages centrées sur la plus large.
        let mixed = Layout::new(&[letter(), landscape(), letter()], 0, 0.5, vec2(100.0, 0.0));
        assert!(mixed.tops.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(mixed.content.x, 792.0 * 0.5 + 32.0);
        assert_eq!(mixed.page_left(1), 16.0);
        // Page unique plus petite que la zone : centrée verticalement.
        let single = Layout::new(&[letter()], 7, 0.5, vec2(1000.0, 1000.0));
        assert_eq!(single.page_top(7), (1000.0 - 396.0) / 2.0);
        assert_eq!(single.pages(), 7..8);
    }

    #[test]
    fn only_visible_pages_are_listed() {
        let l = Layout::new(&[letter(); 600], 0, 1.0, vec2(1000.0, 800.0));
        let y = 300_000.0;
        let v = l.visible(y, y + 800.0);
        assert!(!v.is_empty() && v.len() <= 3, "{v:?}");
        let expected = ((y - 16.0) / 804.0) as usize;
        assert!(v.contains(&expected), "{v:?} / {expected}");
        assert_eq!(l.visible(0.0, 10.0), 0..0, "marge haute seulement");
        assert_eq!(l.visible(0.0, 17.0), 0..1);
        assert_eq!(l.visible(1e9, 1e9 + 1.0).len(), 0);
    }

    #[test]
    fn dominant_page_and_anchors() {
        let l = Layout::new(&[letter(); 3], 0, 1.0, vec2(1000.0, 0.0));
        assert_eq!(l.dominant(700.0, 1500.0), 1);
        assert_eq!(l.dominant(16.0, 1624.0), 0, "égalité : plus petit indice");
        assert_eq!(
            l.dominant(811.0, 819.0),
            1,
            "intervalle : la page la plus proche"
        );
        assert_eq!(l.dominant(808.0, 811.0), 0);
        for y in [16.0, 500.0, 900.0, 2000.0] {
            let (p, f) = l.anchor_at(y);
            assert!((l.y_of(p, f) - y).abs() < 1e-3, "{y}");
        }
    }

    #[test]
    fn zoom_anchor_keeps_the_point_under_the_pointer() {
        let pages: Vec<PageSize> = (0..40)
            .map(|i| if i % 3 == 0 { landscape() } else { letter() })
            .collect();
        let view = vec2(1200.0, 800.0);
        let k1 = 1.1;
        let old = Layout::new(&pages, 0, k1, view);
        let (offset, screen) = (vec2(0.0, 5000.0), vec2(400.0, 300.0));
        let (page, fy) = old.anchor_at(offset.y + screen.y);
        let fx = (offset.x + screen.x - old.page_left(page)) / old.size(page).x;
        let k2 = 1.7 * k1;
        let new = Layout::new(&pages, 0, k2, view);
        let off2 = vec2(
            new.page_left(page) + fx * new.size(page).x - screen.x,
            new.y_of(page, fy) - screen.y,
        );
        let (page2, fy2) = new.anchor_at(off2.y + screen.y);
        assert_eq!(page2, page);
        assert!((fy2 - fy).abs() < 1e-3);
        let fx2 = (off2.x + screen.x - new.page_left(page)) / new.size(page).x;
        assert!((fx2 - fx).abs() * new.size(page).x < 0.5);
    }

    #[test]
    fn fit_zoom_uses_the_reference_page() {
        let pages = [letter(), landscape()];
        let avail = vec2(1000.0, 800.0);
        let cw = effective_zoom(Zoom::FitWidth, ViewMode::Continuous, &pages, 0, avail);
        assert!((cw - 968.0 / 1056.0).abs() < 1e-4, "{cw}");
        let cp = effective_zoom(Zoom::FitPage, ViewMode::Continuous, &pages, 0, avail);
        assert!((cp - 768.0 / 1056.0).abs() < 1e-4, "{cp}");
        let sw = effective_zoom(Zoom::FitWidth, ViewMode::Single, &pages, 0, avail);
        assert!((sw - 968.0 / 816.0).abs() < 1e-4, "{sw}");
        assert_eq!(
            effective_zoom(Zoom::Fixed(2.5), ViewMode::Single, &pages, 1, avail),
            2.5
        );
    }

    #[test]
    fn zoom_is_capped_by_tiles_and_content_height() {
        let huge = [PageSize {
            width_pt: 14_400.0,
            height_pt: 14_400.0,
        }];
        let (z, limited) = clamp_zoom(10.0, &huge, 2.0, ViewMode::Single);
        assert!(limited && z < 3.5, "{z}");
        let a4 = vec![
            PageSize {
                width_pt: 595.0,
                height_pt: 842.0,
            };
            600
        ];
        let (z, limited) = clamp_zoom(20.0, &a4, 1.0, ViewMode::Continuous);
        assert!(limited && z < 12.0, "{z}");
        assert!(z * SCREEN_PER_PDF_POINT * 842.0 * 600.0 <= MAX_CONTENT_PX + 1.0);
        assert_eq!(
            clamp_zoom(1.5, &a4, 1.0, ViewMode::Continuous),
            (1.5, false)
        );
        assert!(clamp_zoom(50.0, &[letter()], 1.0, ViewMode::Single).0 <= ZOOM_MAX);
    }

    #[test]
    fn nav_lock_validity() {
        assert!(lock_still_valid(Some(100.0), 100.5));
        assert!(!lock_still_valid(Some(100.0), 102.0));
        assert!(lock_still_valid(None, 5000.0));
    }

    fn tex(scale_milli: u32, x: u32, bytes: usize) -> TileTex<()> {
        TileTex {
            scale_milli,
            region: PixelRect::new(x, 0, 100, 100),
            page_px: (1000, 1000),
            handle: (),
            bytes,
            used: 0,
            stale: false,
        }
    }

    fn job(page: usize, scale_milli: u32, x: u32) -> TileJob {
        TileJob {
            page,
            scale_milli,
            region: PixelRect::new(x, 0, 100, 100),
        }
    }

    #[test]
    fn pool_memory_stays_within_budget() {
        const MIB: usize = 1024 * 1024;
        let mut pool = TexturePool::<()>::default();
        for i in 0..1000 {
            pool.begin_frame();
            pool.insert(i, tex(1000, 0, MIB));
            if pool.bytes() > 64 * MIB {
                // Les trois dernières pages sont « à l'écran » cette image.
                for p in i.saturating_sub(2)..=i {
                    assert!(pool.exact(&job(p, 1000, 0)).is_some());
                }
                pool.evict(64 * MIB);
            }
            assert!(pool.bytes() <= 64 * MIB, "{}", pool.bytes());
        }
        assert_eq!(pool.len(), 64);
        assert!(pool.has_fresh(&job(999, 1000, 0)));
        assert!(pool.has_fresh(&job(997, 1000, 0)));
        assert!(
            !pool.has_fresh(&job(0, 1000, 0)),
            "la plus ancienne a disparu"
        );
        // Rien d'évincé si tout est dessiné dans l'image courante (borne = budget + écran).
        let mut pool = TexturePool::<()>::default();
        pool.begin_frame();
        pool.insert(0, tex(1000, 0, 10));
        pool.insert(0, tex(1000, 100, 10));
        pool.evict(0);
        assert_eq!(pool.len(), 2);
        pool.begin_frame();
        pool.evict(0);
        assert_eq!((pool.len(), pool.bytes()), (0, 0));
    }

    #[test]
    fn stale_textures_are_fallbacks_until_replaced() {
        let mut pool = TexturePool::<()>::default();
        pool.begin_frame();
        pool.insert(0, tex(1000, 0, 40));
        pool.insert(0, tex(2000, 0, 160));
        pool.insert(0, tex(500, 0, 10));
        pool.mark_stale();
        assert!(pool.exact(&job(0, 1000, 0)).is_none());
        let all = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        assert_eq!(pool.fallbacks(0, 1000, all).len(), 3);
        pool.insert(0, tex(1000, 0, 40));
        assert_eq!(pool.bytes(), 210, "remplacée, pas doublée");
        assert!(pool.exact(&job(0, 1000, 0)).is_some());
        assert!(pool.whole(0).is_none());
        // Ordre : du plus éloigné de l'échelle voulue au plus proche (dessiné en dernier).
        let mut pool = TexturePool::<u32>::default();
        for (s, h) in [(1500u32, 1u32), (400, 2), (1000, 3)] {
            let t = TileTex {
                scale_milli: s,
                region: PixelRect::new(0, 0, 10, 10),
                page_px: (10, 10),
                handle: h,
                bytes: 1,
                used: 0,
                stale: false,
            };
            pool.insert(4, t);
        }
        let order: Vec<u32> = pool
            .fallbacks(4, 1100, all)
            .into_iter()
            .map(|(_, h)| *h)
            .collect();
        assert_eq!(order, [2, 1, 3]);
        assert!(pool
            .fallbacks(4, 1100, Rect::from_min_size(pos2(2.0, 2.0), vec2(1.0, 1.0)))
            .is_empty());
        assert_eq!(pool.whole(4), Some(&1));
    }

    fn plan(
        pages: &[PageSize],
        k: f32,
        y: f32,
        scale: f32,
        pinned: &[usize],
    ) -> (Layout, Rect, Vec<usize>) {
        let layout = Layout::new(pages, 0, k, vec2(1200.0, 800.0));
        let viewport = Rect::from_min_size(pos2(0.0, y), vec2(1200.0, 800.0));
        let input = PlanInput {
            layout: &layout,
            scale_milli: tiles::quantize_milli(scale),
            viewport,
            pinned,
            limits: TileLimits::default(),
        };
        let pages = wanted_jobs(&input, &|_| false)
            .iter()
            .map(|j| j.page)
            .collect();
        (layout, viewport, pages)
    }

    #[test]
    fn wanted_tiles_are_bounded_and_prioritized() {
        let pages = vec![letter(); 500];
        let (layout, vp, wanted) = plan(&pages, 1.0, 200_000.0, 1.5, &[]);
        let visible = layout.visible(vp.min.y, vp.max.y);
        assert!(wanted.len() <= MAX_WANTED && !wanted.is_empty());
        assert!(visible.contains(&wanted[0]), "{wanted:?} / {visible:?}");
        assert!(wanted
            .iter()
            .all(|p| *p + 1 >= visible.start && *p <= visible.end));
        // Toutes les tuiles visibles déjà là : seulement le pré-chargement des voisines.
        let input = PlanInput {
            layout: &layout,
            scale_milli: 1500,
            viewport: vp,
            pinned: &[],
            limits: TileLimits::default(),
        };
        let rest = wanted_jobs(&input, &|j| visible.contains(&j.page));
        assert!(!rest.is_empty() && rest.iter().all(|j| !visible.contains(&j.page)));
        // Une tuile en échec n'est jamais redemandée.
        let first = wanted_jobs(&input, &|_| false)[0];
        assert!(!wanted_jobs(&input, &|j| *j == first).contains(&first));
        // Page très zoomée (A4 à 20 px/pt) : grille bornée par la zone visible.
        let a4 = [PageSize {
            width_pt: 595.0,
            height_pt: 842.0,
        }];
        let (_, _, deep) = plan(&a4, 10.0, 3000.0, 20.0, &[]);
        // Zone de 2400 x 1600 px élargie d'une demi-tuile de chaque côté : 3424 x 2624 px, soit
        // au plus 5 x 4 tuiles de 1024 px.
        assert!(!deep.is_empty() && deep.len() <= 5 * 4, "{}", deep.len());
        // Page d'un glissement en cours : demandée même hors de l'écran.
        let (_, _, pinned) = plan(&pages, 1.0, 0.0, 1.0, &[300]);
        assert!(pinned.contains(&300));
    }

    #[test]
    fn sending_is_skipped_when_only_arrivals_changed() {
        let (a, b, c) = (job(0, 1000, 0), job(0, 1000, 100), job(1, 1000, 0));
        assert!(should_send(&[], &[a], &|_| false));
        assert!(!should_send(&[a, b], &[a, b], &|_| false));
        assert!(!should_send(&[a, b], &[b], &|j| *j == a), "a est arrivée");
        assert!(should_send(&[a, b], &[b], &|_| false), "a annulée");
        assert!(should_send(&[a, b], &[b, c], &|_| true));
        assert!(
            should_send(&[a], &[], &|_| false),
            "liste vide : annule le reste"
        );
    }
}
