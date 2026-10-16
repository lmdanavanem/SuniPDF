//! Vue du document : pages empilées (défilement continu) ou page unique, sur une seule machinerie.
//! Les pages sont rendues en tuiles par le thread moteur (`Request::SetWanted`) et virtualisées :
//! seules les pages visibles (plus une de chaque côté en pré-chargement) créent des widgets, des
//! demandes de rendu et des textures. La géométrie (`Layout`, zoom, tuiles voulues, pool de
//! textures) est pure et testée sans GPU ni PDFium.

use crate::app::{error_key, file_name, SuniApp, Zoom, SCREEN_PER_PDF_POINT};
use crate::reading::{ReadingState, ViewRotation};
use crate::worker::Request;
use egui::{pos2, vec2, Pos2, Rect, Vec2};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use sunipdf_core::annot::ReadingPoint;
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
const LINE_SCROLL_PX: f32 = 48.0;
const SCREEN_SCROLL: f32 = 0.9;
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
    TwoPage,
    Book,
}

impl ViewMode {
    pub fn key(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Continuous => "continuous",
            Self::TwoPage => "two_page",
            Self::Book => "book",
        }
    }
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
    Destination {
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
    pub reading: ReadingState,
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
    pub(crate) hover_page: Option<usize>,
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
            reading: ReadingState::default(),
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
        self.reading = ReadingState::default();
    }

    pub fn orientation_changed(&mut self, page: usize) {
        self.pending = Some(ScrollRequest::Page(page));
        self.nav_lock = Some(NavLock { expected_y: None });
        self.k_prev = None;
        self.sent.clear();
        self.wheel_anchor = None;
        self.reading.history.invalidate_offsets();
    }

    pub fn request(&mut self, r: ScrollRequest) {
        self.pending = Some(r);
    }

    pub fn navigation_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Défilement relatif ; les appuis d'une même image s'additionnent.
    pub fn scroll_by(&mut self, dy: f32) {
        self.nav_lock = None;
        self.pending = Some(match self.pending {
            Some(ScrollRequest::By(d)) => ScrollRequest::By(d + dy),
            _ => ScrollRequest::By(dy),
        });
    }

    /// Défilement clavier : flèches ↑ ↓ d'une ligne ; en continu, Page suiv./préc. et Espace
    /// d'un écran (en page unique, Page suiv./préc. changent de page dans `handle_input`).
    /// `consume_key` ignore une touche Maj en trop : Maj+Espace est lu avant Espace.
    pub fn scroll_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let screen = SCREEN_SCROLL * self.screen_height();
        let continuous = self.mode != ViewMode::Single;
        let mut dy = 0.0;
        ctx.input_mut(|i| {
            let mut each = |m: Modifiers, k: Key, d: f32| {
                dy += d * i.count_and_consume_key(m, k) as f32;
            };
            each(Modifiers::NONE, Key::ArrowDown, LINE_SCROLL_PX);
            each(Modifiers::NONE, Key::ArrowUp, -LINE_SCROLL_PX);
            if continuous {
                each(Modifiers::SHIFT, Key::Space, -screen);
                each(Modifiers::NONE, Key::Space, screen);
                each(Modifiers::NONE, Key::PageDown, screen);
                each(Modifiers::NONE, Key::PageUp, -screen);
            }
        });
        if dy != 0.0 {
            self.scroll_by(dy);
        }
    }

    /// Hauteur de la zone visible à l'image précédente (défilement d'un écran).
    fn screen_height(&self) -> f32 {
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
        if !pages_unchanged || !same_sizes {
            self.reading.history.reset();
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
    lefts: Vec<f32>,
    rows: Vec<(Range<usize>, f32, f32)>,
    pub content: Vec2,
    pub rotation: ViewRotation,
}

impl Layout {
    /// `k` : points logiques par point PDF. Un document plus petit que la zone y est centré.
    #[cfg(test)]
    pub fn new(window: &[PageSize], first: usize, k: f32, viewport: Vec2) -> Self {
        Self::for_mode(window, first, k, viewport, ViewMode::Continuous)
    }

    #[cfg(test)]
    pub fn for_mode(
        window: &[PageSize],
        first: usize,
        k: f32,
        viewport: Vec2,
        mode: ViewMode,
    ) -> Self {
        Self::for_rotation(window, first, k, viewport, mode, ViewRotation::Upright)
    }

    pub fn for_rotation(
        window: &[PageSize],
        first: usize,
        k: f32,
        viewport: Vec2,
        mode: ViewMode,
        rotation: ViewRotation,
    ) -> Self {
        let sizes: Vec<Vec2> = window
            .iter()
            .map(|s| {
                let s = rotation.size(*s);
                vec2(s.width_pt, s.height_pt) * k
            })
            .collect();
        let mut tops = vec![0.0; sizes.len()];
        let mut lefts = vec![0.0; sizes.len()];
        let mut rows = Vec::new();
        let mut max_w = 0.0f32;
        let mut y = PAGE_MARGIN;
        let mut i = 0;
        while i < sizes.len() {
            let pair = matches!(mode, ViewMode::TwoPage | ViewMode::Book)
                && !(mode == ViewMode::Book && first + i == 0);
            let end = (i + if pair { 2 } else { 1 }).min(sizes.len());
            let row_width =
                sizes[i..end].iter().map(|s| s.x).sum::<f32>() + PAGE_GAP * (end - i - 1) as f32;
            let row_height = sizes[i..end].iter().map(|s| s.y).fold(0.0, f32::max);
            max_w = max_w.max(row_width);
            rows.push((i..end, y, row_height));
            for top in &mut tops[i..end] {
                *top = y;
            }
            y += row_height + PAGE_GAP;
            i = end;
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
        let content_w = (max_w + 2.0 * PAGE_MARGIN).max(viewport.x);
        for (range, top, _) in &mut rows {
            *top += shift;
            let width = sizes[range.clone()].iter().map(|s| s.x).sum::<f32>()
                + PAGE_GAP * (range.len() - 1) as f32;
            let mut x = (content_w - width) / 2.0;
            for index in range.clone() {
                lefts[index] = x;
                x += sizes[index].x + PAGE_GAP;
            }
        }
        Self {
            first,
            pt: window.to_vec(),
            tops,
            sizes,
            lefts,
            rows,
            content: vec2(content_w, total.max(viewport.y)),
            rotation,
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
        self.lefts[self.idx(page)]
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
        let start = self.rows.partition_point(|(_, top, h)| top + h <= y0);
        let end = self
            .rows
            .partition_point(|(_, top, _)| *top < y1)
            .max(start);
        let first = self
            .rows
            .get(start)
            .map_or(self.sizes.len(), |(r, _, _)| r.start);
        let last = end
            .checked_sub(1)
            .and_then(|i| self.rows.get(i))
            .map_or(first, |(r, _, _)| r.end);
        self.first + first..self.first + last
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
        let i = self.idx(self.nearest_page(pos2(self.content.x / 2.0, y)));
        (
            self.first + i,
            (y - self.tops[i]) / self.sizes[i].y.max(f32::EPSILON),
        )
    }

    /// Nearest page to a content point, including page gaps and outside the viewport.
    pub fn nearest_page(&self, point: Pos2) -> usize {
        let mut row = self
            .rows
            .partition_point(|(_, top, h)| top + h < point.y)
            .min(self.rows.len().saturating_sub(1));
        if row > 0 && point.y < self.rows[row].1 {
            let before = &self.rows[row - 1];
            if point.y - (before.1 + before.2) < self.rows[row].1 - point.y {
                row -= 1;
            }
        }
        self.rows
            .get(row)
            .and_then(|(range, _, _)| {
                range.clone().min_by(|&a, &b| {
                    let distance = |index| {
                        let r = self.page_rect(self.first + index);
                        r.clamp(point).distance_sq(point)
                    };
                    distance(a).total_cmp(&distance(b))
                })
            })
            .map_or(self.first, |index| self.first + index)
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
        ViewMode::TwoPage | ViewMode::Book => {
            let mut i = 0;
            let mut w = 1.0f32;
            let mut h = 1.0f32;
            while i < pages.len() {
                let end = (i + if mode == ViewMode::Book && i == 0 {
                    1
                } else {
                    2
                })
                .min(pages.len());
                w = w.max(pages[i..end].iter().map(|s| s.width_pt).sum());
                h = h.max(
                    pages[i..end]
                        .iter()
                        .map(|s| s.height_pt)
                        .fold(0.0, f32::max),
                );
                i = end;
            }
            (w, h)
        }
    };
    let gap = if (mode == ViewMode::TwoPage && pages.len() > 1)
        || (mode == ViewMode::Book && pages.len() > 2)
    {
        PAGE_GAP
    } else {
        0.0
    };
    let fit_w = (avail.x - 2.0 * PAGE_MARGIN - gap) / (w * SCREEN_PER_PDF_POINT);
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
    if mode != ViewMode::Single {
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
        let source_size = l.rotation.source_size(rect.size());
        let local = l.rotation.inverse_rect(
            area.intersect(rect).translate(-rect.min.to_vec2()),
            source_size,
        );
        if !local.is_positive() {
            return;
        }
        let Some(view) = to_pixels(local, source_size, page_px) else {
            return;
        };
        for region in tiles::plan_tiles(page_px, view, &input.limits) {
            let job = TileJob {
                page,
                scale_milli: input.scale_milli,
                region,
            };
            if !skip(&job) {
                let source_rect = Rect::from_min_size(rect.min, source_size);
                let source = tile_screen_rect(source_rect, page_px, region).center() - rect.min;
                let c = rect.min + l.rotation.point(source.to_pos2(), source_size).to_vec2();
                out.push((base + c.distance(center), job));
            }
        }
    };
    let visible = l.visible(vp.min.y, vp.max.y);
    // Demi-tuile (pixels de la page) en points PDF, puis en points logiques (× k).
    let half_tile_pt = (input.limits.tile_side as f32 / 2.0) / scale.max(f32::EPSILON);
    for page in visible.clone() {
        let k = l.rotation.source_size(l.size(page)).x
            / l.page_size_pt(page).width_pt.max(f32::EPSILON);
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
    /// dessinée en dernier, donc au-dessus). Les périmées passent sous les autres : après une
    /// retouche, l'ancien contenu ne recouvre jamais un rendu à jour d'une autre échelle.
    pub fn fallbacks(&mut self, page: usize, scale_milli: u32, area: Rect) -> Vec<(Rect, &H)> {
        let frame = self.frame;
        let Some(list) = self.pages.get_mut(&page) else {
            return Vec::new();
        };
        let mut v: Vec<&mut TileTex<H>> = list
            .iter_mut()
            .filter(|t| (t.stale || t.scale_milli != scale_milli) && t.norm().intersects(area))
            .collect();
        v.sort_by_key(|t| {
            (
                !t.stale,
                std::cmp::Reverse(t.scale_milli.abs_diff(scale_milli)),
            )
        });
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
    rotation: ViewRotation,
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

fn paint_rotated_image(
    painter: &egui::Painter,
    id: egui::TextureId,
    rect: Rect,
    uv: Rect,
    rotation: ViewRotation,
    origin: Pos2,
    source_size: Vec2,
) {
    let mut mesh = egui::epaint::Mesh::with_texture(id);
    mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
    for vertex in &mut mesh.vertices {
        vertex.pos = origin
            + rotation
                .point((vertex.pos - origin).to_pos2(), source_size)
                .to_vec2();
    }
    painter.add(mesh);
}

/// Tessellate only document overlays; widgets and text editors remain ordinary egui widgets.
fn rotate_overlay_shapes(
    ctx: &egui::Context,
    painter: &egui::Painter,
    first: egui::layers::ShapeIdx,
    rotation: ViewRotation,
    origin: Pos2,
    source_size: Vec2,
) {
    if rotation == ViewRotation::Upright {
        return;
    }
    let shapes = ctx.graphics_mut(|g| {
        let list = g.entry(painter.layer_id());
        let shapes: Vec<_> = list.all_entries().skip(first.0).cloned().collect();
        let end = list.next_idx().0;
        for index in first.0..end {
            list.reset_shape(egui::layers::ShapeIdx(index));
        }
        shapes
    });
    for clipped in ctx.tessellate(shapes, ctx.pixels_per_point()) {
        if let egui::epaint::Primitive::Mesh(mut mesh) = clipped.primitive {
            for vertex in &mut mesh.vertices {
                vertex.pos = origin
                    + rotation
                        .point((vertex.pos - origin).to_pos2(), source_size)
                        .to_vec2();
            }
            let clip = rotation
                .rect(clipped.clip_rect.translate(-origin.to_vec2()), source_size)
                .translate(origin.to_vec2());
            painter.with_clip_rect(clip).add(mesh);
        }
    }
}

impl SuniApp {
    /// Pages dessinées même hors de l'écran : celle d'un glissement en cours (sinon le widget
    /// disparaîtrait au défilement) et celle de l'éditeur de texte posé sur la page.
    fn pinned_pages(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.annotate.drag.map(|d| d.page).into_iter().collect();
        if let Some(p) = self.edit.text_prompt.as_ref().filter(|p| p.inline) {
            v.push(p.page);
        }
        if let Some(e) = &self.forms.editing {
            v.push(e.page);
        }
        v
    }

    /// Zone document : fenêtre de pages, zoom, défilement, tuiles, interactions.
    pub fn document_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.view.reading.rotation != ViewRotation::Upright
            && (self.annotate.tool != crate::annotate::Tool::Select
                || self.sign.placing.is_some()
                || self.edit.text_prompt.is_some()
                || self.forms.editing.is_some()
                || self.forms.popup.is_some())
        {
            self.view.reading.rotation = ViewRotation::Upright;
            if let Some(doc) = &self.document {
                self.view.orientation_changed(doc.page);
            }
            self.status_msg = Some(tr(self.lang(), "reading.rotation_reset").to_string());
        }
        let Some(doc) = &self.document else { return };
        let sizes = doc.info.page_sizes.clone();
        let count = sizes.len();
        if count == 0 {
            return;
        }
        let current = doc.page.min(count - 1);
        let name = file_name(&doc.info.path);
        let mode = self.view.mode;
        let rotation = self.view.reading.rotation;
        let display_sizes: Vec<_> = sizes.iter().map(|s| rotation.size(*s)).collect();
        let window = match mode {
            ViewMode::Continuous | ViewMode::TwoPage | ViewMode::Book => 0..count,
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
        self.refresh_forms_if_needed();

        // 1. Zoom (molette ancrée sur le pointeur), plafonné.
        self.apply_wheel_zoom(ctx);
        let wanted = effective_zoom(self.zoom, mode, &display_sizes, current, avail);
        let (z, limited) = clamp_zoom(wanted, &display_sizes, ppp, mode);
        if limited && matches!(self.zoom, Zoom::Fixed(_)) {
            self.zoom = Zoom::Fixed(z);
            self.status_msg = Some(tr(self.lang(), "status.zoom_limit").to_string());
        }
        self.effective_zoom = z;
        let k = z * SCREEN_PER_PDF_POINT;
        if self.view.avail_prev != avail {
            self.view.reading.history.invalidate_offsets();
        }
        if self.view.k_prev.is_some_and(|p| (p - k).abs() > 1e-4) {
            self.view.settle_at = Some(now + SETTLE_SECS);
        }
        let scale_milli = tiles::quantize_milli(k * ppp);
        let scale = scale_milli as f32 / 1000.0;

        // 2. Mise en page, 3. décalage imposé (requête, ancre de zoom) seulement cette image.
        let layout = Layout::for_rotation(
            &sizes[window.clone()],
            window.start,
            k,
            avail,
            mode,
            rotation,
        );
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
                    rotation,
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
        if mode != ViewMode::Single && self.view.nav_lock.is_none() {
            self.track_page(layout.dominant(viewport.min.y, viewport.max.y));
        }
        if let Some(doc) = &self.document {
            self.view
                .reading
                .history
                .observe(doc.page, self.view.offset, self.effective_zoom);
        }
        // Outil Sélection d'objet sans panneau Objets : objets de la page survolée (prise directe
        // sur n'importe quelle page visible) ; sinon ceux de la page courante.
        let objects_page = match self.view.hover_page {
            Some(p) if !self.edit.objects_open => p,
            _ => self.document.as_ref().map_or(current, |d| d.page),
        };
        self.refresh_objects_if_needed(objects_page);
        // Après la zone : la page survolée de cette image est connue (sélection d'annotation, liens).
        self.refresh_annotations_if_needed();
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
            // Maj lue au moment du clic (`self.edit.shift` n'est pas mis à jour si `handle_input`
            // est sorti plus tôt).
            let shift = ctx.input(|i| i.modifiers.shift);
            self.handle_page_event(page, ev, shift);
        }
        self.reading_pointer(ctx, &layout, out.inner_rect);
        self.object_bar(ctx, doc_rect);
        self.annot_bar(ctx, doc_rect);
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
        let source_size = s.rotation.source_size(s.rect.size());
        let source_rect = Rect::from_min_size(s.rect.min, source_size);
        let source_clip = s
            .rotation
            .inverse_rect(clip.translate(-s.rect.min.to_vec2()), source_size)
            .translate(s.rect.min.to_vec2());
        let mut overlay_painter = painter.clone();
        overlay_painter.set_clip_rect(source_clip);
        let first_shape = ctx.graphics_mut(|g| g.entry(painter.layer_id()).next_idx());
        self.paint_matches(&overlay_painter, source_rect, s.page, h);
        self.paint_selection(&overlay_painter, source_rect, s.page, h);
        self.paint_object_selection(&overlay_painter, source_rect, s.page, h);
        self.paint_annot_adv(&overlay_painter, source_rect, s.page, h);
        self.paint_redact_marks(&overlay_painter, source_rect, s.page, h);
        rotate_overlay_shapes(
            ctx,
            &painter,
            first_shape,
            s.rotation,
            s.rect.min,
            source_size,
        );
        let on_selection = self
            .edit
            .selection
            .as_ref()
            .map(|sel| sel.page)
            .or_else(|| self.annotate.adv.selected.as_ref().map(|(page, _)| *page));
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
        let alt = match self
            .document
            .as_ref()
            .and_then(|d| d.info.page_label(s.page))
        {
            Some(label) => format!(
                "{alt}. {}",
                tr_args(self.lang(), "reading.page_label", &[("label", label)])
            ),
            None => alt,
        };
        interact.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, &alt));
        let (rect, zoom) = (s.rect, self.effective_zoom);
        let to_pdf = |pos: Pos2| {
            let source = s
                .rotation
                .inverse_point((pos - rect.min).to_pos2(), source_size);
            crate::annotate::to_pdf(rect.min + source.to_vec2(), source_rect, zoom, h)
        };
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
        if let Some(pt) = hover {
            // Fantôme de la signature ou de la date à placer, sous le pointeur.
            self.paint_placing(&painter, s.rect, s.page, h, pt);
        }
        interact.on_hover_cursor(self.page_cursor(s.page, hover));
        // Champs de formulaire : créés après le widget de la page pour avoir la priorité du clic ;
        // éditeur et liste de choix de cette page par-dessus.
        if let Some((field, widget)) = self.forms_overlay(ui, &painter, s.rect, s.page, h) {
            self.forms_activate(s.page, field, widget);
        }
        let view = ui.clip_rect();
        self.forms_editor(ctx, view, s.rect, s.page, h);
        self.forms_popup(ctx, view, s.rect, s.page, h);
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
        let source_size = s.rotation.source_size(s.rect.size());
        let source_rect = Rect::from_min_size(s.rect.min, source_size);
        let local = s
            .rotation
            .inverse_rect(clip.translate(-s.rect.min.to_vec2()), source_size);
        let Some(view_px) = to_pixels(local, source_size, page_px) else {
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
                Some(h) => exact.push((tile_screen_rect(source_rect, page_px, region), h.id())),
                None => missing += 1,
            }
        }
        if missing > 0 {
            self.view.missing_visible += missing;
            let size = source_size;
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
                paint_rotated_image(painter, id, r, uv, s.rotation, s.rect.min, source_size);
            }
        }
        for (r, id) in exact {
            paint_rotated_image(painter, id, r, uv, s.rotation, s.rect.min, source_size);
        }
    }

    /// Current viewport origin in the page's unrotated display coordinates.
    pub(crate) fn reading_page_origin(&self) -> Pos2 {
        let Some(doc) = &self.document else {
            return Pos2::ZERO;
        };
        let Some(size) = doc.info.page_sizes.get(doc.page) else {
            return Pos2::ZERO;
        };
        if let Some(ScrollRequest::Destination {
            x_pt,
            y_from_top_pt,
            ..
        }) = self.view.pending
        {
            return pos2(x_pt, y_from_top_pt);
        }
        if matches!(self.view.pending, Some(ScrollRequest::Page(_))) {
            return self
                .view
                .reading
                .rotation
                .inverse_point(Pos2::ZERO, vec2(size.width_pt, size.height_pt));
        }
        let (offset, k) = match (self.view.pending, self.zoom) {
            (Some(ScrollRequest::Offset(offset)), Zoom::Fixed(zoom)) => {
                (offset, zoom * SCREEN_PER_PDF_POINT)
            }
            _ => {
                let Some(k) = self.view.k_prev.filter(|k| *k > 0.0) else {
                    return Pos2::ZERO;
                };
                (self.view.offset, k)
            }
        };
        let window = if self.view.mode == ViewMode::Single {
            doc.page..doc.page + 1
        } else {
            0..doc.info.page_count()
        };
        let layout = Layout::for_rotation(
            &doc.info.page_sizes[window.clone()],
            window.start,
            k,
            self.view.avail_prev,
            self.view.mode,
            self.view.reading.rotation,
        );
        let point =
            ((offset - vec2(layout.page_left(doc.page), layout.page_top(doc.page))) / k).to_pos2();
        let origin = self
            .view
            .reading
            .rotation
            .inverse_point(point, vec2(size.width_pt, size.height_pt));
        pos2(
            origin.x.clamp(0.0, size.width_pt),
            origin.y.clamp(0.0, size.height_pt),
        )
    }

    /// Resolves explicit navigation before the usual anchored zoom adjustment.
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
                    let source_size = vec2(
                        layout.page_size_pt(p).width_pt,
                        layout.page_size_pt(p).height_pt,
                    );
                    let point = layout
                        .rotation
                        .point(pos2(x_pt, y_from_top_pt), source_size);
                    let x = layout.page_left(p) + point.x * k;
                    let y = layout.page_top(p) + point.y * k;
                    let seen_y = off.y + 0.1 * screen.y..=off.y + 0.85 * screen.y;
                    let seen_x = off.x..=off.x + 0.9 * screen.x;
                    (!(seen_y.contains(&y) && seen_x.contains(&x)))
                        .then(|| vec2(x_for(p, x - screen.x / 4.0), (y - screen.y / 3.0).max(0.0)))
                }
                ScrollRequest::Destination {
                    page,
                    x_pt,
                    y_from_top_pt,
                } => {
                    let p = page.clamp(layout.first, layout.pages().end - 1);
                    let size = layout.page_size_pt(p);
                    let point = layout.rotation.point(
                        pos2(x_pt, y_from_top_pt),
                        vec2(size.width_pt, size.height_pt),
                    );
                    Some(vec2(
                        x_for(p, layout.page_left(p) + point.x * k),
                        (layout.page_top(p) + point.y * k).max(0.0),
                    ))
                }
                ScrollRequest::Offset(o) => Some(o),
                ScrollRequest::By(dy) => Some(vec2(off.x, (off.y + dy).max(0.0))),
            };
        }
        let kp = v.k_prev?;
        if (kp - k).abs() <= 1e-4 || v.first_prev != layout.first || !v.inner.is_positive() {
            return None;
        }
        let old = Layout::for_rotation(
            window,
            layout.first,
            kp,
            v.avail_prev,
            v.mode,
            layout.rotation,
        );
        let screen = match (v.wheel_anchor, self.zoom) {
            (Some(p), _) => p - v.inner.min,
            (None, Zoom::Fixed(_)) => v.inner.size() / 2.0,
            (None, _) => vec2(v.inner.width() / 2.0, 0.0),
        };
        let page = old.nearest_page((off + screen).to_pos2());
        let fy = (off.y + screen.y - old.page_top(page)) / old.size(page).y.max(f32::EPSILON);
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
                self.page_input = doc.info.page_input(page);
                self.view.follow_thumb = true;
            }
        }
    }

    /// Un appui sur une page la rend courante (Pivoter, Supprimer, panneau Objets la visent).
    fn make_current(&mut self, page: usize) {
        if self.view.mode == ViewMode::Single
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
        self.view.reading.history.invalidate_offsets();
        self.settings.continuous_view = mode == ViewMode::Continuous;
        self.settings.view_mode = Some(mode.key().to_string());
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
    fn handle_page_event(&mut self, page: usize, ev: DragEvent, shift: bool) {
        if self.sign.placing.is_some() {
            // Mode de placement d'une signature : le premier appui pose ; les Move / Stop qui
            // suivent ne trouvent aucun glissement en cours et ne font rien.
            if let DragEvent::Start(pt) | DragEvent::Click(pt) = ev {
                self.make_current(page);
                self.place_at(page, pt, shift);
            }
            return;
        }
        if matches!(
            ev,
            DragEvent::Start(_) | DragEvent::Click(_) | DragEvent::DoubleClick(_)
        ) {
            self.make_current(page);
        }
        if self.annotate.tool == crate::annotate::Tool::Select {
            match ev {
                DragEvent::Start((x, y)) => self.start_reading(ReadingPoint { page, x, y }, false),
                DragEvent::DoubleClick((x, y)) => {
                    self.start_reading(ReadingPoint { page, x, y }, true)
                }
                DragEvent::Click(pt) if self.link_at(page, pt).is_some() => {
                    self.annotate.clear_reading();
                    self.adv_click(page, pt);
                }
                DragEvent::Click(_) => {
                    self.annotate.clear_reading();
                    self.annotate.selection = None;
                }
                DragEvent::Move(_) | DragEvent::Stop => {}
            }
            return;
        }
        match ev {
            DragEvent::Start(pt) => {
                self.edit_drag_start(page, pt);
                self.adv_drag_start(page, pt);
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
                    self.adv_drag_moved(&d);
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
                    crate::annotate::Tool::Redact => self.redact_click(page, pt),
                    _ => self.adv_click(page, pt),
                }
            }
            DragEvent::DoubleClick(pt) => {
                if matches!(self.annotate.tool, crate::annotate::Tool::SelectObject) {
                    self.edit_double_click(page, pt);
                } else {
                    self.adv_double_click(page, pt);
                }
            }
        }
    }

    fn reading_pointer(&mut self, ctx: &egui::Context, layout: &Layout, inner: Rect) {
        if self.annotate.reading_drag.is_none() {
            return;
        }
        let (position, down, dt) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
                i.stable_dt,
            )
        });
        if let Some(position) = position {
            let content = inner.clamp(position) - inner.min + self.view.offset;
            let page = layout.nearest_page(content.to_pos2());
            let rect = layout.page_rect(page);
            let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
            let source_size = layout.rotation.source_size(rect.size());
            let source = layout
                .rotation
                .inverse_point((content - rect.min.to_vec2()).to_pos2(), source_size);
            self.move_reading(ReadingPoint {
                page,
                x: (source.x / k).clamp(0.0, layout.page_size_pt(page).width_pt),
                y: (layout.page_size_pt(page).height_pt - source.y / k)
                    .clamp(0.0, layout.page_size_pt(page).height_pt),
            });
            let dy = reading_edge_scroll(position, inner, dt);
            if down && dy != 0.0 {
                self.view.scroll_by(dy);
                ctx.request_repaint();
            }
        }
        if !down {
            self.annotate.reading_drag = None;
        }
    }
}

fn reading_edge_scroll(pointer: Pos2, inner: Rect, dt: f32) -> f32 {
    let edge = 32.0;
    let direction = if pointer.y < inner.top() + edge {
        -((inner.top() + edge - pointer.y) / edge).clamp(0.0, 1.0)
    } else if pointer.y > inner.bottom() - edge {
        ((pointer.y - inner.bottom() + edge) / edge).clamp(0.0, 1.0)
    } else {
        0.0
    };
    direction * 720.0 * dt.clamp(0.0, 0.05)
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
    fn destination_positions_use_reading_rotation_and_align_even_visible_points() {
        for rotation in [
            ViewRotation::Upright,
            ViewRotation::Clockwise,
            ViewRotation::UpsideDown,
            ViewRotation::Counterclockwise,
        ] {
            for mode in [
                ViewMode::Single,
                ViewMode::Continuous,
                ViewMode::TwoPage,
                ViewMode::Book,
            ] {
                let (mut app, _, _) = crate::app::tests::test_app();
                let pages = [letter(), landscape(), letter()];
                app.document.as_mut().unwrap().info.page_sizes = pages.to_vec();
                app.view.reading.rotation = rotation;
                app.view.mode = mode;
                let avail = vec2(200.0, 180.0);
                app.view.inner = Rect::from_min_size(Pos2::ZERO, avail);
                let layout = Layout::for_rotation(&pages, 0, 1.0, avail, mode, rotation);
                app.go_to_destination(sunipdf_core::Destination {
                    page: 0,
                    view: sunipdf_core::DestinationView::Xyz {
                        x: Some(20.0),
                        y: Some(762.0),
                        zoom: None,
                    },
                });
                let offset = app.resolve_scroll(&layout, &pages, avail, 1.0).unwrap();
                let point = match rotation {
                    ViewRotation::Upright => pos2(20.0, 30.0),
                    ViewRotation::Clockwise => pos2(762.0, 20.0),
                    ViewRotation::UpsideDown => pos2(592.0, 762.0),
                    ViewRotation::Counterclockwise => pos2(30.0, 592.0),
                };
                assert!((offset.y - (layout.page_top(0) + point.y)).abs() < 0.001);
                let max_x = layout.page_left(0) + layout.size(0).x + PAGE_MARGIN - avail.x;
                let expected_x = (layout.page_left(0) + point.x).clamp(0.0, max_x);
                assert!(
                    (offset.x - expected_x).abs() < 0.001,
                    "{rotation:?} {mode:?}"
                );
                app.go_to_destination(sunipdf_core::Destination {
                    page: 0,
                    view: sunipdf_core::DestinationView::FitPage,
                });
                assert_eq!(app.zoom, Zoom::FitPage);
                let offset = app.resolve_scroll(&layout, &pages, avail, 1.0).unwrap();
                assert_eq!(
                    offset.y, 0.0,
                    "FitPage must show the whole rotated page from its top"
                );
            }
        }
    }

    #[test]
    fn destination_zoom_frames_restore_same_page_offsets_in_both_directions() {
        let (mut app, _, _) = crate::app::tests::test_app();
        app.zoom = Zoom::Fixed(1.0);
        app.view.mode = ViewMode::Single;
        let ctx = egui::Context::default();
        let frame = |app: &mut SuniApp| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0))),
                    ..Default::default()
                },
                |ui| app.document_view(&ctx, ui),
            );
            output.textures_delta.clear();
        };
        frame(&mut app);
        app.view.scroll_by(200.0);
        frame(&mut app);
        let original = app.view.offset;
        assert!(original.y > 100.0);
        app.go_to_destination(sunipdf_core::Destination {
            page: 0,
            view: sunipdf_core::DestinationView::Xyz {
                x: Some(0.0),
                y: Some(100.0),
                zoom: Some(2.0),
            },
        });
        frame(&mut app);
        let destination = app.view.offset;
        assert_eq!(app.effective_zoom, 2.0);
        assert!(destination.y > original.y);
        app.navigate_reading_history(true);
        frame(&mut app);
        assert_eq!(app.effective_zoom, 1.0);
        assert!((app.view.offset - original).length() < 1.0);
        app.navigate_reading_history(false);
        frame(&mut app);
        assert_eq!(app.effective_zoom, 2.0);
        assert!((app.view.offset - destination).length() < 1.0);
        assert!(!app.dirty);
    }

    #[test]
    fn text_selection_tool_follows_internal_links_and_only_confirms_safe_external_links() {
        use sunipdf_core::{
            annot_write::LinkTarget, Destination, DestinationView, LinkInfo, PointRect,
        };
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.tool = crate::annotate::Tool::Select;
        app.annotate.adv.links_page = Some(0);
        app.annotate.adv.links = vec![LinkInfo {
            index: 0,
            rect: PointRect {
                left: 10.0,
                bottom: 10.0,
                right: 40.0,
                top: 40.0,
            },
            target: LinkTarget::Page(0),
            destination: Some(Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x: Some(100.0),
                    y: Some(600.0),
                    zoom: Some(2.0),
                },
            }),
        }];
        app.handle_page_event(0, DragEvent::Click((20.0, 20.0)), false);
        assert_eq!(app.zoom, Zoom::Fixed(2.0));
        assert_eq!(app.reading_page_origin(), pos2(100.0, 192.0));
        assert!(app.view.reading.history.can_back());
        app.annotate.adv.links[0].target = LinkTarget::Uri("https://example.org".into());
        app.handle_page_event(0, DragEvent::Click((20.0, 20.0)), false);
        assert_eq!(
            app.annotate.adv.url_confirm.as_deref(),
            Some("https://example.org")
        );
        app.annotate.adv.url_confirm = None;
        app.annotate.adv.links[0].target = LinkTarget::Uri("file:///C:/private.pdf".into());
        app.handle_page_event(0, DragEvent::Click((20.0, 20.0)), false);
        assert!(app.annotate.adv.url_confirm.is_none());
        assert!(!app.dirty);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn spreads_cover_first_odd_last_and_mixed_sizes() {
        let pages = [letter(), landscape(), letter(), letter(), landscape()];
        let two = Layout::for_mode(&pages, 0, 1.0, vec2(1000.0, 800.0), ViewMode::TwoPage);
        assert_eq!(two.page_top(0), two.page_top(1));
        assert_eq!(two.page_top(2), two.page_top(3));
        assert!(two.page_top(4) > two.page_top(3));
        assert_eq!(two.page_left(1), two.page_rect(0).right() + PAGE_GAP);
        assert_eq!(two.content.x, 612.0 + 792.0 + PAGE_GAP + 2.0 * PAGE_MARGIN);
        assert_eq!(
            two.visible(700.0, 750.0),
            0..2,
            "the tall page keeps the whole spread visible"
        );
        assert_eq!(two.dominant(700.0, 750.0), 0);
        assert_eq!(two.nearest_page(two.page_rect(1).center()), 1);
        let book = Layout::for_mode(&pages[..4], 0, 1.0, vec2(1000.0, 800.0), ViewMode::Book);
        assert!(book.page_top(1) > book.page_top(0));
        assert_eq!(book.page_top(1), book.page_top(2));
        assert!(book.page_top(3) > book.page_top(2));
        assert_eq!(book.visible(book.page_top(0), book.page_top(1)), 0..1);
        assert_eq!(book.page_rect(0).center().x, book.content.x / 2.0);
        assert_eq!(book.page_rect(3).center().x, book.content.x / 2.0);
    }

    #[test]
    fn spread_fit_zoom_and_tile_cache_stay_bounded() {
        let pages: Vec<_> = (0..600)
            .map(|p| if p % 3 == 0 { landscape() } else { letter() })
            .collect();
        let avail = vec2(1300.0, 900.0);
        for mode in [ViewMode::TwoPage, ViewMode::Book] {
            let zoom = effective_zoom(Zoom::FitWidth, mode, &pages, 0, avail);
            let fit = Layout::for_mode(&pages, 0, zoom * SCREEN_PER_PDF_POINT, avail, mode);
            assert!((fit.content.x - avail.x).abs() < 0.01);
            let zoom = effective_zoom(Zoom::FitPage, mode, &pages, 0, avail);
            let fit = Layout::for_mode(&pages, 0, zoom * SCREEN_PER_PDF_POINT, avail, mode);
            assert!(fit
                .sizes
                .iter()
                .all(|s| s.y + 2.0 * PAGE_MARGIN <= avail.y + 0.01));
            let layout = Layout::for_mode(&pages, 0, 8.0, avail, mode);
            let mut pool = TexturePool::<()>::default();
            for page in (0..600).step_by(17) {
                pool.begin_frame();
                let viewport = Rect::from_min_size(layout.page_rect(page).min, avail);
                let jobs = wanted_jobs(
                    &PlanInput {
                        layout: &layout,
                        scale_milli: 8000,
                        viewport,
                        pinned: &[],
                        limits: TileLimits::for_texture_side(4096),
                    },
                    &|_| false,
                );
                assert!(jobs.len() <= MAX_WANTED && !jobs.is_empty());
                for job in jobs {
                    let bytes = job.region.width as usize * job.region.height as usize * 4;
                    pool.insert(
                        job.page,
                        TileTex {
                            scale_milli: job.scale_milli,
                            region: job.region,
                            page_px: tiles::page_pixels(pages[job.page], 8.0).unwrap(),
                            handle: (),
                            bytes,
                            used: 0,
                            stale: false,
                        },
                    );
                }
                pool.evict(POOL_BUDGET_BYTES);
                assert!(pool.bytes() <= POOL_BUDGET_BYTES);
            }
        }
    }

    #[test]
    fn reading_edge_scroll_is_bounded_and_reverses() {
        let inner = Rect::from_min_size(pos2(100.0, 100.0), vec2(800.0, 600.0));
        assert_eq!(reading_edge_scroll(inner.center(), inner, 0.016), 0.0);
        assert!(reading_edge_scroll(pos2(200.0, 110.0), inner, 0.016) < 0.0);
        assert!(reading_edge_scroll(pos2(200.0, 690.0), inner, 0.016) > 0.0);
        assert_eq!(reading_edge_scroll(pos2(200.0, 10000.0), inner, 10.0), 36.0);
    }

    #[test]
    fn rotated_layout_and_tiling_keep_source_coordinates_and_bounded_jobs() {
        let size = PageSize {
            width_pt: 600.0,
            height_pt: 800.0,
        };
        for mode in [
            ViewMode::Single,
            ViewMode::Continuous,
            ViewMode::TwoPage,
            ViewMode::Book,
        ] {
            for rotation in [
                ViewRotation::Clockwise,
                ViewRotation::UpsideDown,
                ViewRotation::Counterclockwise,
            ] {
                let sizes = [size, size, size];
                let layout =
                    Layout::for_rotation(&sizes, 0, 8.0, vec2(900.0, 600.0), mode, rotation);
                assert_eq!(layout.page_size_pt(0), size);
                let display_size = rotation.size(size);
                assert_eq!(
                    layout.size(0),
                    vec2(display_size.width_pt, display_size.height_pt) * 8.0
                );
                let source_size = vec2(4800.0, 6400.0);
                let source_clip = Rect::from_min_size(pos2(3500.0, 200.0), vec2(300.0, 300.0));
                let viewport = rotation
                    .rect(source_clip, source_size)
                    .translate(layout.page_rect(0).min.to_vec2());
                let jobs = wanted_jobs(
                    &PlanInput {
                        layout: &layout,
                        scale_milli: 8000,
                        viewport,
                        pinned: &[],
                        limits: TileLimits::default(),
                    },
                    &|_| false,
                );
                assert!(!jobs.is_empty() && jobs.len() <= MAX_WANTED);
                assert!(jobs.iter().any(|j| j.page == 0
                    && j.region.x <= 3500
                    && j.region.right() > 3500
                    && j.region.y <= 200
                    && j.region.bottom() > 200));
                assert!(jobs
                    .iter()
                    .all(|j| j.region.right() <= 4800 && j.region.bottom() <= 6400));
            }
        }
    }

    #[test]
    fn rotated_tile_mesh_keeps_uvs_and_overlay_clip_in_display_space() {
        let ctx = egui::Context::default();
        let source = vec2(600.0, 800.0);
        let origin = pos2(20.0, 30.0);
        let rect = Rect::from_min_size(origin + vec2(100.0, 200.0), vec2(60.0, 100.0));
        let clip = Rect::from_min_size(origin, source);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut painter = ui.painter().clone();
            painter.set_clip_rect(clip);
            paint_rotated_image(
                &painter,
                egui::TextureId::User(9),
                rect,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                ViewRotation::Clockwise,
                origin,
                source,
            );
            let first = ctx.graphics_mut(|g| g.entry(painter.layer_id()).next_idx());
            painter.rect_filled(rect, 0.0, egui::Color32::RED);
            rotate_overlay_shapes(
                &ctx,
                &painter,
                first,
                ViewRotation::Clockwise,
                origin,
                source,
            );
        });
        out.textures_delta.clear();
        let tile = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == egui::TextureId::User(9) => {
                    Some(mesh)
                }
                _ => None,
            })
            .unwrap();
        let vertex = tile.vertices.iter().find(|v| v.uv == Pos2::ZERO).unwrap();
        assert_eq!(vertex.pos, origin + vec2(600.0, 100.0));
        let overlay = out.shapes.iter().find(|s| matches!(&s.shape, egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::User(9))).unwrap();
        assert_eq!(
            overlay.clip_rect,
            clip.intersect(Rect::from_min_size(origin, vec2(800.0, 600.0)))
        );
        let egui::Shape::Mesh(mesh) = &overlay.shape else {
            unreachable!()
        };
        assert!(mesh
            .vertices
            .iter()
            .all(|v| v.pos.x > origin.x + 499.0 && v.pos.x < origin.x + 601.0));
    }

    #[test]
    fn rotated_reading_pointer_maps_back_to_source_pdf_points() {
        let k = SCREEN_PER_PDF_POINT;
        let inner = Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 1400.0));
        for rotation in [
            ViewRotation::Clockwise,
            ViewRotation::UpsideDown,
            ViewRotation::Counterclockwise,
        ] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.effective_zoom = 1.0;
            let ctx = egui::Context::default();
            let layout =
                Layout::for_rotation(&[letter()], 0, k, inner.size(), ViewMode::Single, rotation);
            let source = pos2(120.0 * k, (792.0 - 600.0) * k);
            let position = layout.page_rect(0).min
                + rotation.point(source, vec2(612.0 * k, 792.0 * k)).to_vec2();
            let anchor = ReadingPoint {
                page: 0,
                x: 50.0,
                y: 600.0,
            };
            app.annotate.reading_drag = Some((anchor, anchor));
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(inner),
                    events: vec![egui::Event::PointerMoved(position)],
                    ..Default::default()
                },
                |ui| app.reading_pointer(ui.ctx(), &layout, inner),
            );
            out.textures_delta.clear();
            let Request::ReadingSelection { focus, .. } = requests.recv().unwrap() else {
                panic!("reading focus expected");
            };
            assert!((focus.x - 120.0).abs() < 0.001);
            assert!((focus.y - 600.0).abs() < 0.001);
        }
    }

    #[test]
    fn headless_reading_drag_crosses_pages_and_discards_stale_results() {
        use sunipdf_core::annot::{ReadingSelection, TextPosition};
        use sunipdf_core::TextSelection;
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.document.as_mut().unwrap().info.page_sizes = vec![letter(); 3];
        app.effective_zoom = 1.0;
        let ctx = egui::Context::default();
        let inner = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 600.0));
        let layout = Layout::for_mode(
            &[letter(); 3],
            0,
            SCREEN_PER_PDF_POINT,
            inner.size(),
            ViewMode::Continuous,
        );
        app.handle_page_event(0, DragEvent::Start((100.0, 700.0)), false);
        let Request::ReadingSelection {
            epoch,
            generation,
            anchor,
            ..
        } = requests.recv().unwrap()
        else {
            panic!("reading request expected")
        };
        let pointer_frame = |app: &mut SuniApp, position: Pos2, pressed: bool| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(inner),
                    events: vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| app.reading_pointer(ui.ctx(), &layout, inner),
            );
            out.textures_delta.clear();
        };
        pointer_frame(&mut app, pos2(250.0, 590.0), true);
        assert!(matches!(app.view.pending, Some(ScrollRequest::By(dy)) if dy>0.0));
        app.view.offset.y = layout.page_top(1) - 100.0;
        pointer_frame(&mut app, pos2(250.0, 300.0), true);
        assert_eq!(app.annotate.reading_drag.unwrap().1.page, 1);
        assert!(
            requests.try_recv().is_err(),
            "one request stays in flight while the latest focus is queued"
        );
        let result = |text: &str| ReadingSelection {
            anchor: TextPosition { page: 0, index: 0 },
            focus: TextPosition { page: 1, index: 3 },
            pages: vec![
                (
                    0,
                    TextSelection {
                        text: text.into(),
                        rects: vec![],
                    },
                ),
                (
                    1,
                    TextSelection {
                        text: "Beta".into(),
                        rects: vec![],
                    },
                ),
            ],
        };
        app.on_reading_selected(epoch, generation, Ok(result("stale")));
        assert!(app.annotate.reading.is_none());
        let Request::ReadingSelection {
            epoch,
            generation,
            anchor: next_anchor,
            focus,
            word,
        } = requests.recv().unwrap()
        else {
            panic!("latest queued reading request expected")
        };
        assert_eq!(next_anchor, anchor);
        assert_eq!(focus.page, 1);
        assert!(!word);
        app.on_reading_selected(epoch, generation, Ok(result("Alpha")));
        pointer_frame(&mut app, pos2(250.0, 300.0), false);
        assert!(app.annotate.reading_drag.is_none());
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| app.copy_selection(ui.ctx()));
        out.textures_delta.clear();
        assert!(out
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c,egui::OutputCommand::CopyText(text) if text=="Alpha\nBeta")));
        app.annotate.clear_reading();
        app.on_reading_selected(epoch, generation, Ok(result("late")));
        assert!(app.annotate.reading.is_none());
    }

    #[test]
    fn reading_word_and_document_epochs_cannot_resurrect_cancelled_selection() {
        use sunipdf_core::annot::{ReadingSelection, TextPosition};
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.handle_page_event(0, DragEvent::DoubleClick((80.0, 70.0)), false);
        let Request::ReadingSelection {
            epoch,
            generation,
            word,
            ..
        } = requests.recv().unwrap()
        else {
            panic!("word request expected")
        };
        assert!(word);
        assert!(app.annotate.reading_drag.is_none());
        app.view.bump_epoch();
        app.on_reading_selected(
            epoch,
            generation,
            Ok(ReadingSelection {
                anchor: TextPosition { page: 0, index: 0 },
                focus: TextPosition { page: 0, index: 2 },
                pages: vec![],
            }),
        );
        assert!(app.annotate.reading.is_none());
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
        let tile = |scale_milli: u32, handle: u32, stale: bool| TileTex {
            scale_milli,
            region: PixelRect::new(0, 0, 10, 10),
            page_px: (10, 10),
            handle,
            bytes: 1,
            used: 0,
            stale,
        };
        for (s, h) in [(1500u32, 1u32), (400, 2), (1000, 3)] {
            pool.insert(4, tile(s, h, false));
        }
        let order = |pool: &mut TexturePool<u32>| -> Vec<u32> {
            pool.fallbacks(4, 1100, all)
                .into_iter()
                .map(|(_, h)| *h)
                .collect()
        };
        assert_eq!(order(&mut pool), [2, 1, 3]);
        assert!(pool
            .fallbacks(4, 1100, Rect::from_min_size(pos2(2.0, 2.0), vec2(1.0, 1.0)))
            .is_empty());
        assert_eq!(pool.whole(4), Some(&1));
        // Après une retouche, la texture périmée de l'échelle voulue passe sous les replis à jour.
        pool.insert(4, tile(1100, 4, true));
        assert_eq!(order(&mut pool), [4, 2, 1, 3]);
    }

    /// Maj+Espace remonte d'un écran, Espace descend (Maj n'est pas ignorée, egui 0.36).
    #[test]
    fn space_and_shift_space_scroll_one_screen() {
        let ctx = egui::Context::default();
        let mut v = ViewState::new(true);
        v.inner = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        let press = |v: &mut ViewState, modifiers: egui::Modifiers| {
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| v.scroll_keys(ui.ctx()));
            out.textures_delta.clear();
            v.pending.take()
        };
        let screen = SCREEN_SCROLL * 600.0;
        assert_eq!(
            press(&mut v, egui::Modifiers::SHIFT),
            Some(ScrollRequest::By(-screen))
        );
        assert_eq!(
            press(&mut v, egui::Modifiers::NONE),
            Some(ScrollRequest::By(screen))
        );
        // Page unique : Espace ne défile pas (Page suiv./préc. changent de page).
        v.mode = ViewMode::Single;
        assert_eq!(press(&mut v, egui::Modifiers::SHIFT), None);
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

    /// Mesure seulement : `cargo test -p sunipdf-desktop -j 2 -- --ignored --nocapture layout_cost`
    #[test]
    #[ignore]
    fn layout_cost_for_long_documents() {
        for n in [500, 5000] {
            let pages = vec![letter(); n];
            let start = std::time::Instant::now();
            for _ in 0..100 {
                let l = Layout::new(&pages, 0, 1.0, vec2(1000.0, 800.0));
                std::hint::black_box(l.visible(400_000.0, 400_800.0));
            }
            println!(
                "Layout::new + visible, {n} pages : {:?} par image",
                start.elapsed() / 100
            );
        }
    }
}
