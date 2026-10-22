//! Annotations avancées dans l'interface : dessin à main levée, zone de texte libre, rectangle,
//! ellipse, ligne et flèche (de vraies annotations PDF), liens cliquables, sélection d'une
//! annotation existante (déplacer, redimensionner, recolorer, supprimer) et fils de réponses du
//! panneau Commentaires. Tout est écrit par le moteur (`sunipdf-core::annot_write`) ; ce module ne
//! garde que l'état d'interaction et la peinture des surcouches (`paint_slot` de `view.rs`).
//! Coordonnées : points d'affichage (celles du moteur), comme `editor.rs`.

use crate::annotate::{Drag, Tool};
use crate::app::SuniApp;
use crate::editor::{rgb_arr, rgb_tuple, Corner, HANDLE_OFFSET_PX, HANDLE_PX};
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Align, Button, CursorIcon, Frame, Key, Layout, RichText, Stroke};
use sunipdf_core::{
    is_safe_external_uri, thread_status, AnnotDetail, AnnotShape, AnnotStyle, Command, Destination,
    DestinationView, FreeTextStyle, LinkInfo, LinkTarget, PointRect, ReviewStatus,
};
use sunipdf_i18n::{tr, tr_args, Language};

/// Boîte d'une nouvelle zone de texte posée d'un clic (points).
const TEXT_BOX_W: f32 = 180.0;
const TEXT_BOX_H: f32 = 48.0;
/// Tolérance de prise d'une annotation, en points.
const HIT_PT: f32 = 2.0;
/// Zone de lien minimale (points) : plus petite, c'est un clic, pas un tracé.
const LINK_MIN_PT: f32 = 6.0;
const INK_MAX_STROKES: usize = 64;
const INK_MAX_POINTS: usize = 4000;

#[derive(Default)]
pub struct InkDraft {
    pub page: usize,
    pub strokes: Vec<Vec<(f32, f32)>>,
    pub pending: bool,
}

impl Tool {
    /// Outils des annotations avancées (dessin, zone de texte, formes, lien, sélection).
    pub fn is_adv(self) -> bool {
        matches!(
            self,
            Tool::Ink
                | Tool::FreeText
                | Tool::AnnotRect
                | Tool::AnnotEllipse
                | Tool::AnnotLine
                | Tool::AddLink
                | Tool::SelectAnnot
        )
    }
}

/// Geste en cours sur l'annotation sélectionnée.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AdvDrag {
    Move,
    Resize(Corner),
}

#[derive(Clone)]
pub struct FreeTextPrompt {
    pub page: usize,
    pub rect: PointRect,
    /// `Some(index)` : modification d'une zone de texte existante.
    pub index: Option<usize>,
    pub text: String,
    pub size: f32,
    pub rgb: [u8; 3],
    pub fill: bool,
    pub fill_rgb: [u8; 3],
    pub border: f32,
    pub focus: bool,
    pub epoch: u64,
    pub tool: Tool,
    pub workspace: Option<crate::workspace::Workspace>,
    pub pending: bool,
}

impl FreeTextPrompt {
    fn valid(&self) -> bool {
        !self.text.trim().is_empty()
            && (4.0..=200.0).contains(&self.size)
            && (0.0..=50.0).contains(&self.border)
            && [
                self.rect.left,
                self.rect.bottom,
                self.rect.right,
                self.rect.top,
            ]
            .into_iter()
            .all(f32::is_finite)
            && self.rect.right - self.rect.left >= 2.0 * self.size
            && self.rect.top - self.rect.bottom >= 2.0
    }

    fn screen_rect(&self, page: egui::Rect, k: f32, height: f32) -> egui::Rect {
        egui::Rect::from_min_max(
            page.min + egui::vec2(self.rect.left * k, (height - self.rect.top) * k),
            page.min + egui::vec2(self.rect.right * k, (height - self.rect.bottom) * k),
        )
    }
}

#[derive(Clone)]
pub struct LinkPrompt {
    pub page: usize,
    pub rect: PointRect,
    pub index: Option<usize>,
    pub to_page: bool,
    /// Numéro de page visé (base 1).
    pub page_number: usize,
    pub url: String,
    pub error: bool,
    pub precise: PreciseLinkPrompt,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum LinkViewport {
    #[default]
    Fit,
    Xyz,
    FitWidth,
}

#[derive(Clone, Default)]
pub struct PreciseLinkPrompt {
    pub named: bool,
    pub name: String,
    pub view: LinkViewport,
    pub x: String,
    pub y: String,
    pub zoom: String,
}

impl PreciseLinkPrompt {
    fn from_target(target: Option<&LinkTarget>) -> Self {
        let mut result = Self::default();
        match target {
            Some(LinkTarget::Named(name)) => {
                result.named = true;
                result.name = name.clone();
            }
            Some(LinkTarget::Destination(destination)) => {
                let (x, y, zoom) = match destination.view {
                    DestinationView::FitPage => (None, None, None),
                    DestinationView::Xyz { x, y, zoom } => {
                        result.view = LinkViewport::Xyz;
                        (x, y, zoom)
                    }
                    DestinationView::FitWidth { x, y } => {
                        result.view = LinkViewport::FitWidth;
                        (x, y, None)
                    }
                };
                result.x = x.map_or_else(String::new, |value| value.to_string());
                result.y = y.map_or_else(String::new, |value| value.to_string());
                result.zoom = zoom.map_or_else(String::new, |value| value.to_string());
            }
            _ => {}
        }
        result
    }

    fn target(&self, page_number: usize) -> Option<LinkTarget> {
        if self.named {
            return (!self.name.trim().is_empty()).then(|| LinkTarget::Named(self.name.clone()));
        }
        let page = page_number.checked_sub(1)?;
        let number = |text: &str| -> Option<Option<f32>> {
            if text.trim().is_empty() {
                Some(None)
            } else {
                text.trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .map(Some)
            }
        };
        let view = match self.view {
            LinkViewport::Fit => return Some(LinkTarget::Page(page)),
            LinkViewport::Xyz => {
                let zoom = number(&self.zoom)?;
                if zoom.is_some_and(|v| v <= 0.0) {
                    return None;
                }
                DestinationView::Xyz {
                    x: number(&self.x)?,
                    y: number(&self.y)?,
                    zoom,
                }
            }
            LinkViewport::FitWidth => DestinationView::FitWidth {
                x: number(&self.x)?,
                y: number(&self.y)?,
            },
        };
        Some(LinkTarget::Destination(Destination { page, view }))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThreadKind {
    Reply { parent: usize },
    Contents { index: usize },
}

/// Réponse à une annotation, ou modification de son commentaire.
#[derive(Clone)]
pub struct ThreadPrompt {
    pub page: usize,
    pub kind: ThreadKind,
    pub text: String,
    pub focus: bool,
}

/// État d'interaction des annotations avancées, par document (échangé avec l'onglet).
#[derive(Default)]
pub struct AnnotAdvState {
    /// Trait d'encre en cours.
    pub ink: Vec<(f32, f32)>,
    pub ink_draft: Option<InkDraft>,
    pub free_text: Option<FreeTextPrompt>,
    pub rich: Option<crate::annotation_rich::RichDraft>,
    pub rich_generation: u64,
    pub link: Option<LinkPrompt>,
    pub thread: Option<ThreadPrompt>,
    /// Adresse externe qui attend la confirmation de l'utilisateur.
    pub url_confirm: Option<String>,
    /// Lecture détaillée de la page `annotations_page` (vide si indisponible).
    pub details: Vec<AnnotDetail>,
    pub selected: Option<(usize, AnnotDetail)>,
    /// Après une modification, resélectionner cette annotation (page, indice ; `usize::MAX` = la
    /// dernière).
    pub reselect: Option<(usize, usize)>,
    pub links: Vec<LinkInfo>,
    pub links_page: Option<usize>,
    pub drag: Option<AdvDrag>,
}

impl AnnotAdvState {
    /// Les caches désignent des indices du document actif : repris à zéro quand il change de main.
    pub fn invalidate(&mut self) {
        self.free_text = None;
        self.rich = None;
        self.rich_generation = self.rich_generation.wrapping_add(1);
        self.link = None;
        self.details.clear();
        self.selected = None;
        self.reselect = None;
        self.links.clear();
        self.links_page = None;
        self.drag = None;
        self.ink.clear();
        self.ink_draft = None;
    }

    /// Une boîte de dialogue des annotations est ouverte : les raccourcis sont suspendus.
    pub fn dialog_open(&self) -> bool {
        self.free_text.is_some()
            || self.rich.is_some()
            || self.link.is_some()
            || self.thread.is_some()
            || self.url_confirm.is_some()
    }
}

// ----- Fonctions pures --------------------------------------------------------------------------

fn contains(r: &PointRect, pt: (f32, f32), tol: f32) -> bool {
    pt.0 >= r.left - tol && pt.0 <= r.right + tol && pt.1 >= r.bottom - tol && pt.1 <= r.top + tol
}

/// Annotation sous le point : la plus au-dessus (dernière du tableau) hors fenêtres contextuelles,
/// champs de formulaire, entrées illisibles, annotations cachées, réponses et états (sans forme sur
/// la page). Les liens, invisibles, ne passent qu'après les autres : un lien posé sur un texte
/// n'empêche pas de sélectionner ce qu'il recouvre.
pub fn hit_test(details: &[AnnotDetail], pt: (f32, f32)) -> Option<&AnnotDetail> {
    let eligible = |d: &&AnnotDetail| {
        !d.hidden
            && d.parent.is_none()
            && d.status.is_none()
            && !matches!(d.subtype.as_str(), "Popup" | "Widget" | "Unknown")
            && contains(&d.rect, pt, HIT_PT)
    };
    details
        .iter()
        .rev()
        .filter(eligible)
        .find(|d| d.subtype != "Link")
        .or_else(|| {
            details
                .iter()
                .rev()
                .filter(eligible)
                .find(|d| d.subtype == "Link")
        })
}

/// Rectangle après déplacement du coin `corner` de `(dx, dy)` ; inversé si on le passe de l'autre
/// côté, jamais plus petit que 4 points.
pub fn resized(r: PointRect, corner: Corner, dx: f32, dy: f32) -> PointRect {
    let mut b = r;
    match corner {
        Corner::BottomLeft => {
            b.left += dx;
            b.bottom += dy;
        }
        Corner::BottomRight => {
            b.right += dx;
            b.bottom += dy;
        }
        Corner::TopLeft => {
            b.left += dx;
            b.top += dy;
        }
        Corner::TopRight => {
            b.right += dx;
            b.top += dy;
        }
    }
    let (l, rt) = (b.left.min(b.right), b.left.max(b.right));
    let (bt, t) = (b.bottom.min(b.top), b.bottom.max(b.top));
    PointRect {
        left: l,
        right: rt.max(l + 4.0),
        bottom: bt,
        top: t.max(bt + 4.0),
    }
}

/// Boîte par défaut d'une zone de texte posée d'un clic (coin haut-gauche au clic), ramenée dans
/// la page.
pub fn default_text_rect(pt: (f32, f32), page_w: f32, page_h: f32) -> PointRect {
    let left = pt.0.min(page_w - TEXT_BOX_W).max(0.0);
    let bottom = (pt.1 - TEXT_BOX_H)
        .max(0.0)
        .min((page_h - TEXT_BOX_H).max(0.0));
    PointRect {
        left,
        bottom,
        right: left + TEXT_BOX_W,
        top: bottom + TEXT_BOX_H,
    }
}

/// `D:20261014123456Z` → « 14/10/2026 12:34 » (français) ou « 2026-10-14 12:34 ».
pub fn format_pdf_date(s: &str, lang: Language) -> Option<String> {
    let d = s.strip_prefix("D:")?;
    let digits: String = d.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 8 {
        return None;
    }
    let part = |a: usize, b: usize| digits.get(a..b).unwrap_or("00");
    let time = format!("{}:{}", part(8, 10), part(10, 12));
    Some(if lang == Language::French {
        format!("{}/{}/{} {time}", part(6, 8), part(4, 6), part(0, 4))
    } else {
        format!("{}-{}-{} {time}", part(0, 4), part(4, 6), part(6, 8))
    })
}

/// Annotations affichées comme cartes du panneau : celles qui ne sont ni une réponse ni un état,
/// hors fenêtres contextuelles et champs.
pub fn thread_roots(details: &[AnnotDetail]) -> Vec<&AnnotDetail> {
    details
        .iter()
        .filter(|d| {
            d.parent.is_none()
                && d.status.is_none()
                && !matches!(d.subtype.as_str(), "Popup" | "Widget" | "Unknown")
        })
        .collect()
}

/// Réponses (hors états) d'un fil, dans l'ordre du tableau, à tous les niveaux.
pub fn thread_replies(details: &[AnnotDetail], root: usize) -> Vec<&AnnotDetail> {
    let mut family = std::collections::HashSet::from([root]);
    let mut out = Vec::new();
    for d in details {
        if d.parent.is_some_and(|p| family.contains(&p)) {
            family.insert(d.index);
            if d.status.is_none() {
                out.push(d);
            }
        }
    }
    out
}

/// Ce que fait un clic sur un lien : rien ne s'ouvre sans confirmation, et seules les adresses de la
/// liste d'autorisation (`http`, `https`, `mailto`) arrivent jusqu'à la boîte de confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    GoTo(usize),
    Confirm(String),
    Blocked,
    Unsupported,
}

pub fn link_action(target: &LinkTarget) -> LinkAction {
    match target {
        LinkTarget::Page(n) => LinkAction::GoTo(*n),
        LinkTarget::Destination(destination) => LinkAction::GoTo(destination.page),
        LinkTarget::Named(_) => LinkAction::Unsupported,
        LinkTarget::Uri(u) if is_safe_external_uri(u) => LinkAction::Confirm(u.trim().to_string()),
        LinkTarget::Uri(_) => LinkAction::Blocked,
        LinkTarget::Unsupported => LinkAction::Unsupported,
    }
}

fn kind_id(subtype: &str) -> String {
    subtype.to_lowercase()
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn status_key(s: ReviewStatus) -> &'static str {
    match s {
        ReviewStatus::None => "comments.status.none",
        ReviewStatus::Accepted => "comments.status.accepted",
        ReviewStatus::Rejected => "comments.status.rejected",
        ReviewStatus::Cancelled => "comments.status.cancelled",
        ReviewStatus::Completed => "comments.status.completed",
    }
}

fn status_color(s: ReviewStatus) -> egui::Color32 {
    match s {
        ReviewStatus::None => theme::CAT_VIEW,
        ReviewStatus::Accepted => theme::SUCCESS,
        ReviewStatus::Rejected => theme::ERROR,
        ReviewStatus::Cancelled => egui::Color32::from_rgb(0x64, 0x74, 0x8B),
        ReviewStatus::Completed => theme::BLUE,
    }
}

// ----- Interactions -----------------------------------------------------------------------------

impl SuniApp {
    /// Nom de l'auteur écrit dans les nouvelles annotations (vide : omis).
    pub(crate) fn annot_author(&self) -> Option<String> {
        let a = self.settings.author.trim();
        (!a.is_empty()).then(|| a.to_string())
    }

    /// Style des prochaines annotations : couleurs, épaisseur et opacité de la barre.
    fn annot_style(&self, with_fill: bool) -> AnnotStyle {
        AnnotStyle {
            rgb: rgb_tuple(self.edit.stroke_rgb),
            fill_rgb: (with_fill && self.edit.fill).then_some(rgb_tuple(self.edit.fill_rgb)),
            width_pt: self.edit.stroke_width.max(0.5),
            opacity: (self.edit.opacity_pct / 100.0).clamp(0.05, 1.0),
            dash_pattern: if matches!(
                self.annotate.tool,
                Tool::AnnotRect | Tool::AnnotEllipse | Tool::AnnotLine
            ) || (self.annotate.tool == Tool::SelectAnnot
                && self.annotate.adv.selected.as_ref().is_some_and(|(_, d)| {
                    matches!(d.subtype.as_str(), "Square" | "Circle" | "Line")
                })) {
                self.edit.annot_dash_pattern.clone()
            } else {
                Vec::new()
            },
        }
    }

    fn free_text_style(&self) -> FreeTextStyle {
        FreeTextStyle {
            size_pt: self.edit.free_text_size,
            rgb: rgb_tuple(self.edit.free_text_rgb),
            fill_rgb: self
                .edit
                .free_text_fill
                .then_some(rgb_tuple(self.edit.fill_rgb)),
            border_pt: self.edit.free_text_border,
        }
    }

    fn page_size_pt(&self, page: usize) -> (f32, f32) {
        self.document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
            .map_or((612.0, 792.0), |s| (s.width_pt, s.height_pt))
    }

    /// Détail de l'annotation sélectionnée, si elle est sur `page`.
    fn selected_annot_on(&self, page: usize) -> Option<&AnnotDetail> {
        self.annotate
            .adv
            .selected
            .as_ref()
            .filter(|(p, _)| *p == page)
            .map(|(_, d)| d)
    }

    fn corner_on_selected(&self, page: usize, pt: (f32, f32)) -> Option<Corner> {
        let d = self.selected_annot_on(page).filter(|d| d.can_resize)?;
        let k = self.zoom_k();
        let (tol, off) = (HANDLE_PX / k, HANDLE_OFFSET_PX / k);
        Corner::ALL.into_iter().find(|c| {
            let (cx, cy) = c.point(&d.rect, off);
            (cx - pt.0).abs() <= tol && (cy - pt.1).abs() <= tol
        })
    }

    /// Détails valables pour `page` (la lecture de la page survolée est arrivée).
    fn details_for(&self, page: usize) -> &[AnnotDetail] {
        if self.annotate.annotations_page == Some(page) {
            &self.annotate.adv.details
        } else {
            &[]
        }
    }

    fn select_annot(&mut self, page: usize, detail: AnnotDetail) {
        let lang = self.lang();
        // La barre reflète l'annotation : couleurs, épaisseur, fond, opacité, taille du texte.
        if let Some(c) = detail.rgb {
            if detail.subtype == "FreeText" {
                self.edit.free_text_rgb = rgb_arr(c);
            } else {
                self.edit.stroke_rgb = rgb_arr(c);
            }
        }
        match detail.fill_rgb {
            Some(c) => {
                self.edit.fill_rgb = rgb_arr(c);
                self.edit.fill = true;
                self.edit.free_text_fill = true;
            }
            None => {
                self.edit.fill = false;
                self.edit.free_text_fill = false;
            }
        }
        if let Some(w) = detail.width_pt {
            if detail.subtype == "FreeText" {
                self.edit.free_text_border = w;
            } else if w > 0.0 {
                self.edit.stroke_width = w;
            }
        }
        if let Some(s) = detail.font_size {
            self.edit.free_text_size = s;
        }
        self.edit.opacity_pct = (detail.opacity * 100.0).round();
        self.edit.annot_dash_pattern = detail.dash_pattern.clone();
        self.status_msg = Some(if detail.locked {
            tr(lang, "status.annot_locked").to_string()
        } else {
            tr_args(
                lang,
                "status.annot_selected",
                &[("kind", &kind_label(lang, &detail.subtype))],
            )
        });
        self.annotate.adv.selected = Some((page, detail));
    }

    fn deselect_annot(&mut self) {
        self.annotate.adv.selected = None;
        self.annotate.adv.drag = None;
    }

    pub(crate) fn ink_draft_active(&self) -> bool {
        self.annotate.adv.ink_draft.is_some()
    }

    pub(crate) fn ink_blocks_action(&mut self) -> bool {
        if self.annotate.quantities.open {
            self.status_msg = Some(self.t("quantity.finish_first").into());
            return true;
        }
        if self.library.blocks_document() {
            self.status_msg = Some(self.t("library.finish_first").into());
            return true;
        }
        if self.annotate.measure.saved.dialog_open() {
            self.status_msg = Some(self.t("measure.saved.finish_first").into());
            return true;
        }
        if self.annotate.measure.is_active_draft() {
            self.status_msg = Some(self.t("measure.finish_first").into());
            return true;
        }
        if self.annotate.adv.rich.is_some() {
            self.status_msg = Some(self.t("rich.finish_first").into());
            return true;
        }
        if self.annotate.adv.free_text.is_some() {
            self.status_msg = Some(self.t("free_text.finish_first").into());
            return true;
        }
        if !self.ink_draft_active() {
            return false;
        }
        self.status_msg = Some(self.t("ink.finish_first").into());
        true
    }

    pub(crate) fn ink_blocks_command(&self, command: Command) -> bool {
        if self.annotate.quantities.open {
            return true;
        }
        if self.library.blocks_document() {
            return true;
        }
        if self.annotate.measure.saved.dialog_open() {
            return true;
        }
        if self.annotate.measure.is_active_draft() {
            return !matches!(
                command,
                Command::ZoomIn
                    | Command::ZoomOut
                    | Command::ActualSize
                    | Command::FitWidth
                    | Command::FitPage
                    | Command::ToggleThumbnails
                    | Command::ToggleComments
                    | Command::ToggleTools
            );
        }
        if self.annotate.adv.rich.is_some() {
            return true;
        }
        if self.annotate.adv.free_text.is_some() {
            return !matches!(
                command,
                Command::ZoomIn
                    | Command::ZoomOut
                    | Command::ActualSize
                    | Command::FitWidth
                    | Command::FitPage
            );
        }
        self.ink_draft_active()
            && !matches!(
                command,
                Command::FirstPage
                    | Command::PreviousPage
                    | Command::NextPage
                    | Command::LastPage
                    | Command::ZoomIn
                    | Command::ZoomOut
                    | Command::ActualSize
                    | Command::FitWidth
                    | Command::FitPage
                    | Command::ViewContinuous
                    | Command::ViewSinglePage
                    | Command::ViewTwoPage
                    | Command::ViewBook
                    | Command::Find
                    | Command::HistoryBack
                    | Command::HistoryForward
                    | Command::ToggleThumbnails
                    | Command::ToggleComments
                    | Command::ToggleBookmarks
                    | Command::ToggleObjects
                    | Command::ToggleForms
                    | Command::ToggleTools
                    | Command::CommandPalette
                    | Command::CopyText
            )
    }

    fn start_ink_stroke(&mut self, page: usize, pt: (f32, f32)) {
        if !pt.0.is_finite() || !pt.1.is_finite() {
            return;
        }
        if !self.ink_draft_active() {
            if !self.ink_can_start() {
                self.status_msg = Some(self.t("ink.wait").into());
                return;
            }
            self.annotate.adv.ink_draft = Some(InkDraft {
                page,
                ..Default::default()
            });
            self.annotate.adv.selected = None;
        }
        let adv = &self.annotate.adv;
        let draft = adv.ink_draft.as_ref().unwrap();
        let problem = if draft.pending {
            Some("ink.wait")
        } else if draft.page != page {
            Some("ink.same_page")
        } else if draft.strokes.len() >= INK_MAX_STROKES {
            Some("ink.stroke_limit")
        } else if !adv.ink.is_empty() {
            Some("ink.wait")
        } else {
            None
        };
        if let Some(key) = problem {
            self.status_msg = Some(self.t(key).into());
            return;
        }
        self.annotate.adv.ink = vec![pt];
        self.status_msg = Some(self.t("ink.hint").into());
    }

    pub(crate) fn finish_ink_stroke(&mut self, page: usize) {
        let adv = &mut self.annotate.adv;
        if let Some(draft) = &mut adv.ink_draft {
            if draft.page == page && !draft.pending && !adv.ink.is_empty() {
                draft.strokes.push(std::mem::take(&mut adv.ink));
            }
        }
    }

    fn finish_ink_draft(&mut self) {
        let Some(draft) = self.annotate.adv.ink_draft.as_ref() else {
            return;
        };
        if draft.pending || !self.annotate.adv.ink.is_empty() || draft.strokes.is_empty() {
            return;
        }
        let (page, strokes) = (draft.page, draft.strokes.clone());
        let style = self.annot_style(false);
        let author = self.annot_author();
        self.annotate.adv.ink_draft.as_mut().unwrap().pending = true;
        self.worker.send(Request::Edit(EditOp::AddInk {
            page,
            strokes,
            style,
            author,
        }));
    }

    pub(crate) fn cancel_ink_draft(&mut self) {
        if self
            .annotate
            .adv
            .ink_draft
            .as_ref()
            .is_some_and(|draft| !draft.pending)
        {
            self.annotate.adv.ink_draft = None;
            self.annotate.adv.ink.clear();
            self.annotate.drag = None;
            self.status_msg = Some(self.t("status.cancelled").into());
        }
    }

    pub fn adv_drag_start(&mut self, page: usize, pt: (f32, f32)) {
        log::debug!(
            "annotation avancée : début {:?} page {page} {pt:?}",
            self.annotate.tool
        );
        match self.annotate.tool {
            Tool::Ink => self.start_ink_stroke(page, pt),
            Tool::SelectAnnot => {
                self.annotate.adv.drag = if let Some(c) = self.corner_on_selected(page, pt) {
                    Some(AdvDrag::Resize(c))
                } else if self
                    .selected_annot_on(page)
                    .is_some_and(|d| d.can_move && contains(&d.rect, pt, HIT_PT))
                {
                    Some(AdvDrag::Move)
                } else {
                    // Une annotation non sélectionnée se prend et se déplace d'un seul geste.
                    let hit = hit_test(self.details_for(page), pt).cloned();
                    match hit {
                        Some(d) => {
                            let movable = d.can_move;
                            self.select_annot(page, d);
                            movable.then_some(AdvDrag::Move)
                        }
                        None => {
                            self.deselect_annot();
                            None
                        }
                    }
                };
            }
            _ => {}
        }
    }

    pub fn adv_drag_moved(&mut self, drag: &Drag) {
        if self.annotate.tool == Tool::Ink {
            let adv = &mut self.annotate.adv;
            let Some(draft) = adv.ink_draft.as_ref() else {
                return;
            };
            if draft.page != drag.page || draft.pending || adv.ink.is_empty() {
                return;
            }
            let far = adv
                .ink
                .last()
                .is_none_or(|l| (l.0 - drag.current.0).hypot(l.1 - drag.current.1) > 0.5);
            if far {
                if adv.ink.len() < INK_MAX_POINTS {
                    if drag.current.0.is_finite() && drag.current.1.is_finite() {
                        adv.ink.push(drag.current);
                    }
                } else {
                    self.status_msg = Some(self.t("ink.point_limit").into());
                }
            }
        }
    }

    /// Rectangle de l'annotation sélectionnée pendant un geste (aperçu et envoi).
    fn dragged_rect(&self, drag: &Drag) -> Option<PointRect> {
        let d = self.selected_annot_on(drag.page)?;
        let (dx, dy) = (drag.current.0 - drag.start.0, drag.current.1 - drag.start.1);
        match self.annotate.adv.drag? {
            AdvDrag::Move => Some(PointRect {
                left: d.rect.left + dx,
                right: d.rect.right + dx,
                bottom: d.rect.bottom + dy,
                top: d.rect.top + dy,
            }),
            AdvDrag::Resize(c) => Some(resized(d.rect, c, dx, dy)),
        }
    }

    /// Fin d'un glissement avec un outil d'annotation avancée.
    pub fn adv_drag_finished(&mut self, drag: Drag) {
        log::debug!(
            "annotation avancée : fin {:?} {drag:?} ({} points)",
            self.annotate.tool,
            self.annotate.adv.ink.len()
        );
        let rect = drag.rect();
        let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
        let tiny = w < 2.0 && h < 2.0;
        let author = self.annot_author();
        match self.annotate.tool {
            Tool::Ink => {
                self.finish_ink_stroke(drag.page);
            }
            Tool::AnnotRect | Tool::AnnotEllipse | Tool::AnnotLine => {
                if w < 2.0 && h < 2.0 {
                    return;
                }
                let (shape, rect, fill) = match self.annotate.tool {
                    Tool::AnnotRect => (AnnotShape::Square, rect, true),
                    Tool::AnnotEllipse => (AnnotShape::Circle, rect, true),
                    _ => (
                        if self.edit.arrow {
                            AnnotShape::Arrow
                        } else {
                            AnnotShape::Line
                        },
                        PointRect {
                            left: drag.start.0,
                            bottom: drag.start.1,
                            right: drag.current.0,
                            top: drag.current.1,
                        },
                        false,
                    ),
                };
                if matches!(shape, AnnotShape::Square | AnnotShape::Circle) && (w < 2.0 || h < 2.0)
                {
                    return;
                }
                self.worker.send(Request::Edit(EditOp::AddAnnotShape {
                    page: drag.page,
                    shape,
                    rect,
                    style: self.annot_style(fill),
                    author,
                }));
            }
            Tool::FreeText => {
                if !self.free_text_can_start() {
                    return;
                }
                let (pw, ph) = self.page_size_pt(drag.page);
                let rect = if tiny || w < 20.0 || h < 12.0 {
                    default_text_rect(drag.start, pw, ph)
                } else {
                    rect
                };
                let st = self.free_text_style();
                self.annotate.adv.free_text = Some(FreeTextPrompt {
                    page: drag.page,
                    rect,
                    index: None,
                    text: String::new(),
                    size: st.size_pt,
                    rgb: self.edit.free_text_rgb,
                    fill: self.edit.free_text_fill,
                    fill_rgb: self.edit.fill_rgb,
                    border: st.border_pt,
                    focus: true,
                    epoch: self.view.epoch,
                    tool: self.annotate.tool,
                    workspace: self.annotate.workspace,
                    pending: false,
                });
            }
            Tool::AddLink => {
                if w >= LINK_MIN_PT && h >= LINK_MIN_PT {
                    let count = self.document.as_ref().map_or(1, |d| d.info.page_count());
                    self.annotate.adv.link = Some(LinkPrompt {
                        page: drag.page,
                        rect,
                        index: None,
                        to_page: true,
                        page_number: (drag.page + 2).min(count),
                        url: String::new(),
                        error: false,
                        precise: PreciseLinkPrompt::default(),
                    });
                } else {
                    self.status_msg = Some(tr(self.lang(), "status.link_too_small").to_string());
                }
            }
            Tool::SelectAnnot => {
                let mode = self.annotate.adv.drag.take();
                let new_rect = self.dragged_rect_with(&drag, mode);
                let Some((page, d)) = self.annotate.adv.selected.clone() else {
                    return;
                };
                if page != drag.page {
                    return;
                }
                let (dx, dy) = (drag.current.0 - drag.start.0, drag.current.1 - drag.start.1);
                match (mode, new_rect) {
                    (Some(AdvDrag::Move), _) if dx.hypot(dy) >= 1.0 => {
                        self.annotate.adv.reselect = Some((page, d.index));
                        self.worker.send(Request::Edit(EditOp::MoveAnnotation {
                            page,
                            index: d.index,
                            dx,
                            dy,
                        }));
                    }
                    (Some(AdvDrag::Resize(_)), Some(rect)) if dx.hypot(dy) >= 1.0 => {
                        self.annotate.adv.reselect = Some((page, d.index));
                        self.worker.send(Request::Edit(EditOp::ResizeAnnotation {
                            page,
                            index: d.index,
                            rect,
                        }));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn dragged_rect_with(&self, drag: &Drag, mode: Option<AdvDrag>) -> Option<PointRect> {
        let d = self.selected_annot_on(drag.page)?;
        let (dx, dy) = (drag.current.0 - drag.start.0, drag.current.1 - drag.start.1);
        match mode? {
            AdvDrag::Resize(c) => Some(resized(d.rect, c, dx, dy)),
            AdvDrag::Move => None,
        }
    }

    /// Clic sans glissement : point d'encre, boîte de texte par défaut, sélection d'une annotation,
    /// ou lien suivi (outil de lecture).
    pub fn adv_click(&mut self, page: usize, pt: (f32, f32)) {
        log::debug!(
            "annotation avancée : clic {:?} page {page} {pt:?} ({} liens en cache pour la page {:?})",
            self.annotate.tool,
            self.annotate.adv.links.len(),
            self.annotate.adv.links_page
        );
        match self.annotate.tool {
            Tool::Ink => {
                self.start_ink_stroke(page, pt);
                self.finish_ink_stroke(page);
            }
            Tool::FreeText => self.adv_drag_finished(Drag {
                page,
                start: pt,
                current: pt,
            }),
            Tool::SelectAnnot => {
                let hit = hit_test(self.details_for(page), pt).cloned();
                match hit {
                    Some(d) => self.select_annot(page, d),
                    None => self.deselect_annot(),
                }
            }
            Tool::Select => self.follow_link_at(page, pt),
            _ => {}
        }
    }

    /// Double-clic avec l'outil de sélection : modifier le texte ou la destination du lien.
    pub fn adv_double_click(&mut self, page: usize, pt: (f32, f32)) {
        if self.annotate.tool != Tool::SelectAnnot {
            return;
        }
        let hit = hit_test(self.details_for(page), pt).cloned();
        if let Some(d) = hit {
            let link = d.subtype == "Link";
            self.select_annot(page, d);
            if link {
                self.edit_selected_annot_link();
            } else {
                self.edit_selected_annot_text();
            }
        }
    }

    pub(crate) fn link_at(&self, page: usize, pt: (f32, f32)) -> Option<&LinkInfo> {
        if self.annotate.adv.links_page != Some(page) {
            return None;
        }
        self.annotate
            .adv
            .links
            .iter()
            .rev()
            .find(|l| contains(&l.rect, pt, 0.0))
    }

    /// Suit le lien sous le pointeur : saut de page, ou adresse après confirmation.
    fn follow_link_at(&mut self, page: usize, pt: (f32, f32)) {
        let Some(link) = self.link_at(page, pt).cloned() else {
            return;
        };
        let lang = self.lang();
        match link_action(&link.target) {
            LinkAction::GoTo(n) => match link.destination {
                Some(destination) => self.go_to_destination(destination),
                None if self.reading_navigation_ready() => self.go_to_page(n),
                None => {}
            },
            LinkAction::Confirm(u) => self.annotate.adv.url_confirm = Some(u),
            LinkAction::Blocked => {
                self.status_msg = Some(tr(lang, "status.link_blocked").to_string());
            }
            LinkAction::Unsupported => {
                self.status_msg = Some(tr(lang, "status.link_unsupported").to_string());
            }
        }
    }

    /// Curseur au-dessus d'une page pour les outils d'annotation avancée et les liens.
    pub fn adv_cursor(&self, page: usize, pt: Option<(f32, f32)>) -> Option<CursorIcon> {
        match self.annotate.tool {
            Tool::Ink
            | Tool::FreeText
            | Tool::AnnotRect
            | Tool::AnnotEllipse
            | Tool::AnnotLine
            | Tool::AddLink => Some(CursorIcon::Crosshair),
            Tool::SelectAnnot => Some(match (self.annotate.adv.drag, pt) {
                (Some(AdvDrag::Resize(c)), _) => c.cursor(),
                (Some(AdvDrag::Move), _) => CursorIcon::Grabbing,
                (None, Some(pt)) => {
                    if let Some(c) = self.corner_on_selected(page, pt) {
                        c.cursor()
                    } else if self
                        .selected_annot_on(page)
                        .is_some_and(|d| d.can_move && contains(&d.rect, pt, HIT_PT))
                    {
                        CursorIcon::Move
                    } else if hit_test(self.details_for(page), pt).is_some() {
                        CursorIcon::PointingHand
                    } else {
                        CursorIcon::Default
                    }
                }
                _ => CursorIcon::Default,
            }),
            Tool::Select => pt
                .is_some_and(|pt| self.link_at(page, pt).is_some())
                .then_some(CursorIcon::PointingHand),
            _ => None,
        }
    }

    /// Flèches : décale l'annotation sélectionnée (Maj : ×10).
    pub fn nudge_annot(&mut self, dx: f32, dy: f32) {
        let Some((page, d)) = self.annotate.adv.selected.clone() else {
            return;
        };
        if !d.can_move {
            return;
        }
        self.annotate.adv.reselect = Some((page, d.index));
        self.worker.send(Request::Edit(EditOp::MoveAnnotation {
            page,
            index: d.index,
            dx,
            dy,
        }));
    }

    /// Supprime l'annotation sélectionnée avec ses réponses.
    pub fn delete_selected_annot(&mut self) {
        if let Some((page, d)) = self.annotate.adv.selected.take() {
            self.annotate.adv.drag = None;
            self.worker
                .send(Request::Edit(EditOp::DeleteAnnotationThread {
                    page,
                    index: d.index,
                }));
        }
    }

    /// Modifier le texte de l'annotation sélectionnée : zone de texte libre sur la page ou
    /// commentaire d'une autre annotation.
    pub fn edit_selected_annot_text(&mut self) {
        if !self.free_text_can_start() {
            return;
        }
        let Some((page, d)) = self.annotate.adv.selected.clone() else {
            return;
        };
        if !d.can_edit_text {
            return;
        }
        if d.subtype == "FreeText" {
            self.annotate.adv.free_text = Some(FreeTextPrompt {
                page,
                rect: d.rect,
                index: Some(d.index),
                text: d.contents.clone().unwrap_or_default(),
                size: d.font_size.unwrap_or(12.0),
                rgb: rgb_arr(d.rgb.unwrap_or((0, 0, 0))),
                fill: d.fill_rgb.is_some(),
                fill_rgb: rgb_arr(d.fill_rgb.unwrap_or((255, 255, 255))),
                border: d.width_pt.unwrap_or(0.0),
                focus: true,
                epoch: self.view.epoch,
                tool: self.annotate.tool,
                workspace: self.annotate.workspace,
                pending: false,
            });
        } else {
            self.annotate.adv.thread = Some(ThreadPrompt {
                page,
                kind: ThreadKind::Contents { index: d.index },
                text: d.contents.clone().unwrap_or_default(),
                focus: true,
            });
        }
    }

    fn edit_selected_annot_link(&mut self) {
        let Some((page, detail)) = self.annotate.adv.selected.as_ref() else {
            return;
        };
        if !detail.can_edit_link {
            return;
        }
        let cached = self
            .annotate
            .adv
            .links
            .iter()
            .find(|link| self.annotate.adv.links_page == Some(*page) && link.index == detail.index)
            .map(|link| &link.target);
        let target = match detail.link.as_ref() {
            Some(target @ (LinkTarget::Named(_) | LinkTarget::Destination(_))) => Some(target),
            _ => cached.or(detail.link.as_ref()),
        };
        let (to_page, page_number, url) = match target {
            Some(LinkTarget::Page(page)) => (true, page + 1, String::new()),
            Some(LinkTarget::Destination(destination)) => {
                (true, destination.page + 1, String::new())
            }
            Some(LinkTarget::Named(_)) => (false, 1, String::new()),
            Some(LinkTarget::Uri(url)) => (false, 1, url.clone()),
            _ => (false, 1, String::new()),
        };
        self.annotate.adv.link = Some(LinkPrompt {
            page: *page,
            rect: detail.rect,
            index: Some(detail.index),
            to_page,
            page_number,
            url,
            error: false,
            precise: PreciseLinkPrompt::from_target(target),
        });
    }

    fn start_reply(&mut self, page: usize, parent: usize) {
        self.annotate.adv.thread = Some(ThreadPrompt {
            page,
            kind: ThreadKind::Reply { parent },
            text: String::new(),
            focus: true,
        });
    }

    /// Réponse `Annotations` : liste rapide, lecture détaillée, resélection après modification.
    pub fn on_annotations(
        &mut self,
        page: usize,
        mut list: Vec<sunipdf_core::AnnotationInfo>,
        details: Option<Vec<AnnotDetail>>,
    ) {
        if self.annotate.annotations_page != Some(page) {
            return;
        }
        // Les champs de formulaire ont leur panneau ; « supprimer » ici détruirait un champ.
        list.retain(|a| a.kind != "widget");
        self.annotate.annotations = list;
        self.annotate.adv.details = details.unwrap_or_default();
        if let Some((rp, ri)) = self.annotate.adv.reselect {
            if rp == page {
                self.annotate.adv.reselect = None;
                let found = if ri == usize::MAX {
                    self.annotate.adv.details.last()
                } else {
                    self.annotate.adv.details.iter().find(|d| d.index == ri)
                }
                .cloned();
                if let Some(d) = found {
                    let keep = self.status_msg.take();
                    self.select_annot(page, d);
                    if keep.is_some() {
                        self.status_msg = keep;
                    }
                }
            }
        }
    }

    pub fn on_links(&mut self, page: usize, list: Vec<LinkInfo>) {
        if self.annotate.adv.links_page == Some(page) {
            self.annotate.adv.links = list;
        }
    }

    /// Page dont les annotations sont voulues : celle qui attend une resélection, la page survolée
    /// (outil de sélection d'annotation), ou la page courante (panneau Commentaires).
    fn annotations_wanted_page(&self) -> Option<usize> {
        let doc = self.document.as_ref()?;
        if let Some((p, _)) = self.annotate.adv.reselect {
            return Some(p);
        }
        if self.annotate.tool == Tool::SelectAnnot {
            return Some(self.view.hover_page.unwrap_or(doc.page));
        }
        self.annotate.comments_open.then_some(doc.page)
    }

    /// Demande la lecture des annotations (et des zones de lien) des pages qui en ont besoin.
    pub fn refresh_annotations_if_needed(&mut self) {
        if let Some(page) = self.annotations_wanted_page() {
            if self.annotate.annotations_page != Some(page) {
                self.annotate.annotations_page = Some(page);
                self.annotate.annotations.clear();
                self.annotate.adv.details.clear();
                self.worker.send(Request::Annotations {
                    page,
                    detailed: true,
                });
            }
        }
        let wants_links = matches!(
            self.annotate.tool,
            Tool::Select | Tool::AddLink | Tool::SelectAnnot
        );
        if let (true, Some(page)) = (wants_links, self.view.hover_page) {
            if self.annotate.adv.links_page != Some(page) {
                self.annotate.adv.links_page = Some(page);
                self.annotate.adv.links.clear();
                self.worker.send(Request::Links { page });
            }
        }
    }
}

/// Libellé traduit d'un sous-type d'annotation (nom PDF) ; inconnu : « Autre annotation ».
pub fn kind_label(lang: Language, subtype: &str) -> String {
    crate::annotate::annotation_label(lang, &kind_id(subtype))
}

// ----- Peinture ---------------------------------------------------------------------------------

fn dash_shapes(
    points: &[egui::Pos2],
    pattern: &[f32],
    scale: f32,
    stroke: Stroke,
) -> Option<Vec<egui::Shape>> {
    let cycle: f64 = pattern
        .iter()
        .map(|v| f64::from(*v) * f64::from(scale))
        .sum();
    let length: f64 = points
        .windows(2)
        .map(|p| f64::from(p[0].distance(p[1])))
        .sum();
    if pattern.is_empty()
        || pattern.len() > 16
        || pattern
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0 || (*v > 0.0 && *v < 0.1) || *v > 1_000.0)
        || !scale.is_finite()
        || scale <= 0.0
        || !cycle.is_finite()
        || cycle <= 0.0
        || !length.is_finite()
        || length / cycle * pattern.len() as f64 > 100_000.0
    {
        // Invalid or excessive patterns omit the preview stroke rather than misrepresenting it as solid.
        return None;
    }
    let count = pattern.len()
        * if pattern.len().is_multiple_of(2) {
            1
        } else {
            2
        };
    let expanded: Vec<_> = pattern
        .iter()
        .cycle()
        .take(count)
        .map(|v| v * scale)
        .collect();
    let dashes: Vec<_> = expanded.iter().step_by(2).copied().collect();
    let gaps: Vec<_> = expanded.iter().skip(1).step_by(2).copied().collect();
    Some(egui::Shape::dashed_line_with_offset(
        points, stroke, &dashes, &gaps, 0.0,
    ))
}

fn paint_annot_stroke(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    pattern: &[f32],
    scale: f32,
    stroke: Stroke,
) {
    if pattern.is_empty() {
        painter.add(egui::epaint::PathShape::line(points.to_vec(), stroke));
    } else if let Some(shapes) = dash_shapes(points, pattern, scale, stroke) {
        painter.extend(shapes);
    }
}

impl SuniApp {
    /// Aperçus de tracé (encre, formes, zones), liens du mode Lien, cadre et poignées de
    /// l'annotation sélectionnée.
    pub fn paint_annot_adv(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let k = self.zoom_k();
        let pt = |p: (f32, f32)| page_rect.min + egui::vec2(p.0 * k, (page_h_pt - p.1) * k);
        let to_screen =
            |b: &PointRect| egui::Rect::from_min_max(pt((b.left, b.top)), pt((b.right, b.bottom)));
        let accent = theme::BLUE;
        let tool = self.annotate.tool;
        let drag = self.annotate.drag.filter(|d| d.page == page_index);
        let alpha = (self.edit.opacity_pct / 100.0).clamp(0.05, 1.0);
        let stroke = Stroke::new(
            (self.edit.stroke_width.max(0.5) * k).max(1.0),
            color32(self.edit.stroke_rgb).gamma_multiply(alpha),
        );
        if tool == Tool::Ink {
            let adv = &self.annotate.adv;
            if let Some(draft) = adv
                .ink_draft
                .as_ref()
                .filter(|draft| draft.page == page_index)
            {
                for path in draft.strokes.iter().chain(std::iter::once(&adv.ink)) {
                    if path.len() == 1 {
                        painter.circle_filled(pt(path[0]), stroke.width / 2.0, stroke.color);
                    } else if path.len() > 1 {
                        let pts = sunipdf_core::smooth_polyline(path, 4)
                            .iter()
                            .map(|p| pt(*p))
                            .collect();
                        painter.add(egui::epaint::PathShape::line(pts, stroke));
                    }
                }
            }
        }
        if let Some(d) = drag {
            let r = to_screen(&d.rect());
            let fill = if self.edit.fill {
                color32(self.edit.fill_rgb).gamma_multiply(alpha)
            } else {
                egui::Color32::TRANSPARENT
            };
            match tool {
                Tool::AnnotRect => {
                    painter.rect_filled(r, 0.0, fill);
                    paint_annot_stroke(
                        painter,
                        &[
                            r.left_bottom(),
                            r.right_bottom(),
                            r.right_top(),
                            r.left_top(),
                            r.left_bottom(),
                        ],
                        &self.edit.annot_dash_pattern,
                        k,
                        stroke,
                    );
                }
                Tool::AnnotEllipse => {
                    painter.add(egui::epaint::EllipseShape {
                        center: r.center(),
                        radius: r.size() / 2.0,
                        angle: 0.0,
                        fill,
                        stroke: if self.edit.annot_dash_pattern.is_empty() {
                            stroke
                        } else {
                            Stroke::NONE
                        },
                    });
                    if !self.edit.annot_dash_pattern.is_empty() {
                        let points: Vec<_> = (0..=128)
                            .map(|i| {
                                let angle = std::f32::consts::TAU * i as f32 / 128.0;
                                r.center()
                                    + egui::vec2(
                                        angle.cos() * r.width() / 2.0,
                                        -angle.sin() * r.height() / 2.0,
                                    )
                            })
                            .collect();
                        paint_annot_stroke(
                            painter,
                            &points,
                            &self.edit.annot_dash_pattern,
                            k,
                            stroke,
                        );
                    }
                }
                Tool::AnnotLine => {
                    let (a, b) = (pt(d.start), pt(d.current));
                    paint_annot_stroke(painter, &[a, b], &self.edit.annot_dash_pattern, k, stroke);
                    if self.edit.arrow && (b - a).length() > 1.0 {
                        let u = (b - a).normalized();
                        let n = egui::vec2(-u.y, u.x);
                        let hl = (4.0 * self.edit.stroke_width.max(0.5)).max(8.0) * k;
                        let base = b - u * hl;
                        painter.line_segment([b, base + n * hl * 0.466], stroke);
                        painter.line_segment([b, base - n * hl * 0.466], stroke);
                    }
                }
                Tool::FreeText | Tool::AddLink => {
                    let c = [
                        r.left_top(),
                        r.right_top(),
                        r.right_bottom(),
                        r.left_bottom(),
                        r.left_top(),
                    ];
                    painter.rect_filled(r, 2.0, accent.gamma_multiply(0.08));
                    painter.extend(egui::Shape::dashed_line(
                        &c,
                        Stroke::new(1.5, accent),
                        6.0,
                        4.0,
                    ));
                }
                _ => {}
            }
        }
        // Zones de lien (outil Lien) : pointillés bleus.
        if tool == Tool::AddLink && self.annotate.adv.links_page == Some(page_index) {
            for l in &self.annotate.adv.links {
                let r = to_screen(&l.rect);
                let c = [
                    r.left_top(),
                    r.right_top(),
                    r.right_bottom(),
                    r.left_bottom(),
                    r.left_top(),
                ];
                painter.rect_filled(r, 0.0, accent.gamma_multiply(0.06));
                painter.extend(egui::Shape::dashed_line(
                    &c,
                    Stroke::new(1.0, accent),
                    4.0,
                    3.0,
                ));
            }
        }
        // Annotation sélectionnée : cadre (déplacé ou redimensionné pendant le geste), poignées.
        let Some(sel) = self.selected_annot_on(page_index) else {
            return;
        };
        let moving = drag
            .and_then(|d| self.dragged_rect(&d))
            .filter(|_| self.annotate.adv.drag.is_some());
        let bounds = moving.unwrap_or(sel.rect);
        let frame = to_screen(&bounds).expand(3.0);
        painter.rect_stroke(
            frame,
            2.0,
            Stroke::new(1.5, accent),
            egui::StrokeKind::Middle,
        );
        if sel.can_resize && !matches!(self.annotate.adv.drag, Some(AdvDrag::Move)) {
            let off = HANDLE_OFFSET_PX / k;
            for c in Corner::ALL {
                let r =
                    egui::Rect::from_center_size(pt(c.point(&bounds, off)), egui::vec2(8.0, 8.0));
                painter.rect(
                    r,
                    2.0,
                    egui::Color32::WHITE,
                    Stroke::new(1.5, accent),
                    egui::StrokeKind::Middle,
                );
            }
        }
    }
}

// ----- Barre flottante --------------------------------------------------------------------------

/// Ce que la barre propose selon l'outil ou l'annotation sélectionnée.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
struct BarKind {
    stroke: bool,
    width: bool,
    fill: bool,
    opacity: bool,
    arrow: bool,
    dashed: bool,
    text: bool,
}

fn bar_kind(tool: Tool, selected: Option<&AnnotDetail>) -> Option<BarKind> {
    let shape = |fill, arrow| BarKind {
        stroke: true,
        width: true,
        fill,
        opacity: true,
        arrow,
        dashed: true,
        text: false,
    };
    match tool {
        Tool::Ink => Some(BarKind {
            dashed: false,
            ..shape(false, false)
        }),
        Tool::AnnotRect | Tool::AnnotEllipse => Some(shape(true, false)),
        Tool::AnnotLine => Some(shape(false, true)),
        Tool::FreeText => Some(BarKind {
            stroke: true,
            width: true,
            fill: true,
            opacity: false,
            arrow: false,
            dashed: false,
            text: true,
        }),
        Tool::SelectAnnot => {
            let d = selected?;
            Some(match d.subtype.as_str() {
                "Square" | "Circle" => shape(true, false),
                "Line" => shape(false, false),
                "Ink" => BarKind {
                    dashed: false,
                    ..shape(false, false)
                },
                "FreeText" => BarKind {
                    stroke: true,
                    width: true,
                    fill: true,
                    opacity: false,
                    arrow: false,
                    dashed: false,
                    text: true,
                },
                _ => BarKind {
                    stroke: true,
                    opacity: true,
                    ..Default::default()
                },
            })
        }
        _ => None,
    }
}

impl SuniApp {
    /// Rectangle écran de l'annotation sélectionnée (page affichée), poignées comprises.
    fn annot_screen_rect(&self) -> Option<egui::Rect> {
        let (page, page_rect, h) = self.edit.page_view?;
        let b = self.selected_annot_on(page)?.rect;
        let k = self.zoom_k();
        let pt = |x: f32, y: f32| page_rect.min + egui::vec2(x * k, (h - y) * k);
        Some(egui::Rect::from_two_pos(pt(b.left, b.top), pt(b.right, b.bottom)).expand(6.0))
    }

    /// Barre flottante des annotations avancées : couleurs, épaisseur, fond, opacité, flèche, taille
    /// du texte ; pour l'annotation sélectionnée, aussi Appliquer, Répondre, Texte et Supprimer.
    pub fn annot_bar(&mut self, ctx: &egui::Context, bounds: egui::Rect) {
        let selected = self.annotate.adv.selected.clone();
        let Some(kind) = bar_kind(self.annotate.tool, selected.as_ref().map(|(_, d)| d)) else {
            return;
        };
        if self.annotate.adv.dialog_open() {
            return;
        }
        // Une annotation qu'on ne peut pas restyler (verrouillée, d'un autre producteur) n'offre que
        // les actions : pas de réglages qui ne feraient rien.
        let kind = if selected.as_ref().is_none_or(|(_, d)| d.can_restyle) {
            kind
        } else {
            BarKind::default()
        };
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        const GAP: f32 = 14.0;
        let max_w = (bounds.width() - 2.0 * GAP - 18.0).max(80.0);
        let id = egui::Id::unique("annot-bar");
        let bar_h = ctx.memory(|m| m.area_rect(id)).map_or(44.0, |r| r.height());
        let pos = match (selected.as_ref(), self.annot_screen_rect()) {
            (Some(_), Some(r)) if r.bottom() + 16.0 + bar_h <= bounds.bottom() - 8.0 => {
                egui::pos2(r.left(), r.bottom() + 16.0)
            }
            (Some(_), Some(r)) => egui::pos2(r.left(), r.top() - bar_h - 12.0),
            _ => bounds.min + egui::vec2(GAP, GAP),
        };
        let pos = pos.clamp(
            bounds.min + egui::vec2(GAP, GAP),
            bounds.max - egui::vec2(GAP, GAP),
        );
        let mut apply = false;
        let mut reply = false;
        let mut edit_text = false;
        let mut edit_link = false;
        let mut delete = false;
        let mut finish_ink = false;
        let mut cancel_ink = false;
        let sel_detail = selected.as_ref().map(|(_, d)| d.clone());
        let size_range = 6.0..=96.0;
        egui::Area::new(id)
            .fixed_pos(pos)
            .constrain_to(bounds)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 2],
                        blur: 8,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(28),
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(max_w);
                        if kind != BarKind::default()
                            && !self
                                .annotate
                                .adv
                                .ink_draft
                                .as_ref()
                                .is_some_and(|draft| draft.pending)
                        {
                            ui.horizontal_wrapped(|ui| {
                                if kind.stroke {
                                    ui.label(tr(
                                        lang,
                                        if kind.text {
                                            "bar.text_color"
                                        } else {
                                            "bar.stroke"
                                        },
                                    ));
                                    let color = if kind.text {
                                        &mut self.edit.free_text_rgb
                                    } else {
                                        &mut self.edit.stroke_rgb
                                    };
                                    ui.color_edit_button_srgb(color);
                                }
                                if kind.text {
                                    ui.separator();
                                    ui.label(tr(lang, "bar.font_size"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.free_text_size)
                                            .range(size_range.clone())
                                            .speed(0.5)
                                            .suffix(" pt"),
                                    );
                                }
                                if kind.width {
                                    ui.separator();
                                    let (value, hint) = if kind.text {
                                        (&mut self.edit.free_text_border, "bar.border")
                                    } else {
                                        (&mut self.edit.stroke_width, "bar.width")
                                    };
                                    ui.add(
                                        egui::DragValue::new(value)
                                            .range(if kind.text { 0.0..=10.0 } else { 0.5..=50.0 })
                                            .speed(0.1)
                                            .suffix(" pt"),
                                    )
                                    .on_hover_text(tr(lang, hint));
                                }
                                if kind.fill {
                                    ui.separator();
                                    let flag = if kind.text {
                                        &mut self.edit.free_text_fill
                                    } else {
                                        &mut self.edit.fill
                                    };
                                    ui.checkbox(flag, tr(lang, "bar.fill"));
                                    let on = *flag;
                                    ui.add_enabled_ui(on, |ui| {
                                        ui.color_edit_button_srgb(&mut self.edit.fill_rgb);
                                    });
                                }
                                if kind.opacity {
                                    ui.separator();
                                    ui.label(tr(lang, "bar.opacity"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.opacity_pct)
                                            .range(10.0..=100.0)
                                            .speed(1.0)
                                            .suffix(" %"),
                                    );
                                }
                                if kind.arrow {
                                    ui.separator();
                                    ui.checkbox(&mut self.edit.arrow, tr(lang, "bar.arrow"));
                                }
                                if kind.dashed {
                                    ui.separator();
                                    let mut dashed = !self.edit.annot_dash_pattern.is_empty();
                                    if ui.checkbox(&mut dashed, tr(lang, "bar.dashed")).changed() {
                                        self.edit.annot_dash_pattern =
                                            if dashed { vec![6.0, 3.0] } else { Vec::new() };
                                    }
                                }
                            });
                        }
                        if let Some(d) = &sel_detail {
                            ui.horizontal_wrapped(|ui| {
                                let tip = tr_args(
                                    lang,
                                    "bar.annot_apply_hint",
                                    &[("kind", &kind_label(lang, &d.subtype))],
                                );
                                if d.can_restyle {
                                    if ui
                                        .add(Button::new(tr(lang, "bar.annot_apply")))
                                        .on_hover_text(&tip)
                                        .clicked()
                                    {
                                        apply = true;
                                    }
                                    ui.separator();
                                }
                                let icon_button =
                                    |ui: &mut egui::Ui,
                                     glyph: &str,
                                     fallback: &str,
                                     key: &str,
                                     enabled: bool| {
                                        ui.add_enabled(
                                            enabled,
                                            Button::new(icons::icon(
                                                has_icons, glyph, fallback, 15.0,
                                            ))
                                            .frame(false),
                                        )
                                        .on_hover_text(tr(lang, key))
                                        .on_disabled_hover_text(tr(lang, key))
                                        .clicked()
                                    };
                                reply = icon_button(ui, glyph::COMMENT, "💬", "bar.reply", true);
                                edit_text = icon_button(
                                    ui,
                                    glyph::RENAME,
                                    "T",
                                    "bar.annot_edit_text",
                                    d.can_edit_text,
                                );
                                if d.subtype == "Link" {
                                    edit_link = ui
                                        .add_enabled(
                                            d.can_edit_link,
                                            Button::new(tr(lang, "bar.annot_edit_link")),
                                        )
                                        .clicked();
                                }
                                delete = icon_button(
                                    ui,
                                    glyph::DELETE,
                                    "🗑",
                                    "bar.annot_delete",
                                    !d.locked,
                                );
                            });
                        }
                        if self.annotate.tool == Tool::Ink {
                            ui.horizontal_wrapped(|ui| {
                                let draft = self.annotate.adv.ink_draft.as_ref();
                                let pending = draft.is_some_and(|draft| draft.pending);
                                let ready = draft.is_some_and(|draft| !draft.strokes.is_empty())
                                    && self.annotate.adv.ink.is_empty()
                                    && !pending;
                                finish_ink = ui
                                    .add_enabled(ready, Button::new(tr(lang, "ink.finish")))
                                    .on_hover_text(tr(lang, "ink.finish_hint"))
                                    .clicked();
                                cancel_ink = ui
                                    .add_enabled(
                                        draft.is_some() && !pending,
                                        Button::new(tr(lang, "ink.cancel")),
                                    )
                                    .on_hover_text(tr(lang, "ink.cancel_hint"))
                                    .clicked();
                                ui.label(tr(lang, if pending { "ink.wait" } else { "ink.hint" }));
                            });
                        }
                    });
            });
        if finish_ink {
            self.finish_ink_draft();
        }
        if cancel_ink {
            self.cancel_ink_draft();
        }
        if let Some((page, d)) = selected {
            if apply {
                self.annotate.adv.reselect = Some((page, d.index));
                let op = if d.subtype == "FreeText" {
                    EditOp::UpdateFreeText {
                        page,
                        index: d.index,
                        text: d.contents.clone().unwrap_or_default(),
                        style: self.free_text_style(),
                    }
                } else {
                    EditOp::StyleAnnotation {
                        page,
                        index: d.index,
                        style: self.annot_style(matches!(d.subtype.as_str(), "Square" | "Circle")),
                    }
                };
                self.worker.send(Request::Edit(op));
            }
            if reply {
                self.start_reply(page, d.index);
            }
            if edit_text {
                self.edit_selected_annot_text();
            }
            if edit_link {
                self.edit_selected_annot_link();
            }
            if delete {
                self.delete_selected_annot();
            }
        }
    }
}

// ----- Boîtes de dialogue -----------------------------------------------------------------------

impl SuniApp {
    pub fn adv_dialogs(&mut self, ctx: &egui::Context) {
        self.link_dialog(ctx);
        self.thread_dialog(ctx);
        self.url_dialog(ctx);
    }

    fn free_text_can_start(&self) -> bool {
        self.ink_can_start() && self.form_builder_ready() && !self.form_builder.active()
    }

    pub(crate) fn validate_free_text(&mut self) {
        let valid = self.annotate.adv.free_text.as_ref().is_none_or(|p| {
            self.document
                .as_ref()
                .is_some_and(|d| p.page < d.info.page_count())
                && p.epoch == self.view.epoch
                && p.tool == self.annotate.tool
                && p.workspace == self.annotate.workspace
        });
        if !valid {
            self.annotate.adv.free_text = None;
        }
    }

    pub(crate) fn free_text_editor(
        &mut self,
        ctx: &egui::Context,
        page_rect: egui::Rect,
        page: usize,
        height: f32,
    ) {
        self.validate_free_text();
        let lang = self.lang();
        let palette = theme::palette(self.settings.dark_theme);
        let ready = self.form_builder_ready() && !self.form_builder.active();
        let k = self.effective_zoom * crate::app::SCREEN_PER_PDF_POINT;
        let Some(prompt) = self
            .annotate
            .adv
            .free_text
            .as_mut()
            .filter(|p| p.page == page)
        else {
            return;
        };
        let rect = prompt.screen_rect(page_rect, k, height);
        let (mut submit, mut cancel) = (false, false);
        let mut rich_font = false;
        egui::Area::new(egui::Id::unique("inline-free-text"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(true)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.5, theme::BLUE))
                    .corner_radius(6)
                    .inner_margin(6)
                    .show(ui, |ui| {
                        ui.set_width(rect.width().clamp(220.0, 600.0));
                        ui.add_enabled_ui(!prompt.pending, |ui| {
                            let color = egui::Color32::from_rgb(
                                prompt.rgb[0],
                                prompt.rgb[1],
                                prompt.rgb[2],
                            );
                            let fill = if prompt.fill {
                                egui::Color32::from_rgb(
                                    prompt.fill_rgb[0],
                                    prompt.fill_rgb[1],
                                    prompt.fill_rgb[2],
                                )
                            } else {
                                egui::Color32::WHITE
                            };
                            let field = egui::ScrollArea::vertical()
                                .max_height(
                                    (ctx.content_rect().height() - 220.0).clamp(80.0, 320.0),
                                )
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut prompt.text)
                                            .font(egui::FontId::proportional(
                                                (prompt.size * k).max(6.0),
                                            ))
                                            .text_color(color)
                                            .background_color(fill)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(3)
                                            .hint_text(tr(lang, "dialog.free_text.placeholder")),
                                    )
                                })
                                .inner;
                            if prompt.focus {
                                field.request_focus();
                                prompt.focus = false;
                            }
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "dialog.text.size"));
                                ui.add(
                                    egui::DragValue::new(&mut prompt.size)
                                        .range(6.0..=96.0)
                                        .speed(0.5),
                                );
                                ui.color_edit_button_srgb(&mut prompt.rgb)
                                    .on_hover_text(tr(lang, "dialog.text.color"));
                                ui.checkbox(
                                    &mut prompt.fill,
                                    tr(lang, "dialog.free_text.background"),
                                );
                                ui.add_enabled_ui(prompt.fill, |ui| {
                                    ui.color_edit_button_srgb(&mut prompt.fill_rgb);
                                });
                                ui.label(tr(lang, "dialog.free_text.border"));
                                ui.add(
                                    egui::DragValue::new(&mut prompt.border)
                                        .range(0.0..=10.0)
                                        .speed(0.1),
                                );
                            });
                            ui.label(
                                RichText::new(tr(lang, "dialog.free_text.latin_note"))
                                    .small()
                                    .color(palette.text_muted),
                            );
                            rich_font = ui.button(tr(lang, "rich.open_font")).clicked();
                            if !prompt.text.trim().is_empty() && !prompt.valid() {
                                ui.colored_label(palette.error, tr(lang, "free_text.invalid"));
                            }
                            ui.horizontal(|ui| {
                                submit = ui
                                    .add_enabled(
                                        ready && prompt.valid(),
                                        Button::new(tr(
                                            lang,
                                            if prompt.index.is_some() {
                                                "dialog.free_text.ok_edit"
                                            } else {
                                                "dialog.free_text.ok"
                                            },
                                        )),
                                    )
                                    .clicked();
                                cancel = ui.button(tr(lang, "dialog.password.cancel")).clicked();
                            });
                        });
                        if prompt.pending {
                            ui.spinner();
                        }
                    });
            });
        if !prompt.pending
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
        {
            cancel = true;
        }
        if rich_font {
            self.open_rich_from_free_text_prompt(ctx);
        } else if cancel {
            self.annotate.adv.free_text = None;
        } else if submit {
            self.submit_free_text();
        }
    }

    fn submit_free_text(&mut self) {
        self.validate_free_text();
        if !self.form_builder_ready() || self.form_builder.active() {
            return;
        }
        let author = self.annot_author();
        let Some(p) = self
            .annotate
            .adv
            .free_text
            .as_mut()
            .filter(|p| !p.pending && p.valid())
        else {
            return;
        };
        p.pending = true;
        let style = FreeTextStyle {
            size_pt: p.size,
            rgb: rgb_tuple(p.rgb),
            fill_rgb: p.fill.then_some(rgb_tuple(p.fill_rgb)),
            border_pt: p.border,
        };
        self.edit.free_text_size = p.size;
        self.edit.free_text_border = p.border;
        self.edit.free_text_rgb = p.rgb;
        self.edit.free_text_fill = p.fill;
        self.edit.fill_rgb = p.fill_rgb;
        let op = match p.index {
            Some(index) => {
                self.annotate.adv.reselect = Some((p.page, index));
                EditOp::UpdateFreeText {
                    page: p.page,
                    index,
                    text: p.text.clone(),
                    style,
                }
            }
            None => EditOp::AddFreeText {
                page: p.page,
                rect: p.rect,
                text: p.text.clone(),
                style,
                author,
            },
        };
        self.worker.send(Request::Edit(op));
    }

    fn link_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let count = self.document.as_ref().map_or(1, |d| d.info.page_count());
        let error = theme::palette(self.settings.dark_theme).error;
        let Some(prompt) = self.annotate.adv.link.as_mut() else {
            return;
        };
        let editing = prompt.index.is_some();
        let mut submit = false;
        let mut cancel = false;
        let spec = crate::dialog::Spec::new(
            "link",
            tr(
                lang,
                if editing {
                    "dialog.link.title_edit"
                } else {
                    "dialog.link.title"
                },
            ),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(
            lang,
            if editing {
                "dialog.link.subtitle_edit"
            } else {
                "dialog.link.subtitle"
            },
        ))
        .icon(glyph::GLOBE, "🔗")
        .width(460.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                if ui
                    .radio(
                        prompt.to_page && !prompt.precise.named,
                        tr(lang, "dialog.link.kind_page"),
                    )
                    .clicked()
                {
                    prompt.to_page = true;
                    prompt.precise.named = false;
                }
                if ui
                    .radio(
                        !prompt.to_page && !prompt.precise.named,
                        tr(lang, "dialog.link.kind_url"),
                    )
                    .clicked()
                {
                    prompt.to_page = false;
                    prompt.precise.named = false;
                }
                if ui
                    .radio(prompt.precise.named, tr(lang, "dialog.link.kind_named"))
                    .clicked()
                {
                    prompt.precise.named = true;
                    prompt.to_page = false;
                }
                ui.add_space(4.0);
                if prompt.precise.named {
                    ui.label(tr(lang, "dialog.link.named_label"));
                    ui.text_edit_singleline(&mut prompt.precise.name);
                    ui.small(tr(lang, "dialog.link.named_note"));
                } else if prompt.to_page {
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "dialog.link.page"));
                        ui.add(
                            egui::DragValue::new(&mut prompt.page_number)
                                .range(1..=count)
                                .speed(0.2),
                        );
                        ui.label(format!("/ {count}"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "dialog.link.view"));
                        ui.selectable_value(&mut prompt.precise.view, LinkViewport::Fit, "Fit");
                        ui.selectable_value(&mut prompt.precise.view, LinkViewport::Xyz, "XYZ");
                        ui.selectable_value(
                            &mut prompt.precise.view,
                            LinkViewport::FitWidth,
                            "FitH",
                        );
                    });
                    if prompt.precise.view != LinkViewport::Fit {
                        ui.small(tr(lang, "dialog.link.axes_note"));
                        ui.horizontal(|ui| {
                            ui.label("X");
                            ui.add(
                                egui::TextEdit::singleline(&mut prompt.precise.x)
                                    .desired_width(75.0),
                            );
                            ui.label("Y");
                            ui.add(
                                egui::TextEdit::singleline(&mut prompt.precise.y)
                                    .desired_width(75.0),
                            );
                            if prompt.precise.view == LinkViewport::Xyz {
                                ui.label(tr(lang, "dialog.link.zoom"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut prompt.precise.zoom)
                                        .desired_width(75.0),
                                );
                            }
                        });
                    }
                } else {
                    crate::dialog::field_label(ui, tr(lang, "dialog.link.url"));
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut prompt.url)
                            .hint_text(tr(lang, "dialog.link.url_hint"))
                            .desired_width(f32::INFINITY),
                    );
                    if ui.memory(|m| m.focused().is_none()) {
                        field.request_focus();
                    }
                    if field.changed() {
                        prompt.error = false;
                    }
                }
                if prompt.error {
                    ui.label(
                        RichText::new(tr(lang, "dialog.link.invalid_target"))
                            .size(12.5)
                            .color(error),
                    );
                }
            },
            |ui| {
                if crate::dialog::primary(
                    ui,
                    tr(
                        lang,
                        if editing {
                            "dialog.link.ok_edit"
                        } else {
                            "dialog.link.ok"
                        },
                    ),
                ) || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if submit {
            let Some(p) = self.annotate.adv.link.clone() else {
                return;
            };
            let target = if p.precise.named || p.to_page {
                if !p.precise.named && !(1..=count).contains(&p.page_number) {
                    return;
                }
                let Some(target) = p.precise.target(p.page_number) else {
                    if let Some(prompt) = self.annotate.adv.link.as_mut() {
                        prompt.error = true;
                    }
                    return;
                };
                target
            } else if is_safe_external_uri(&p.url) {
                LinkTarget::Uri(p.url.trim().to_string())
            } else {
                if let Some(l) = self.annotate.adv.link.as_mut() {
                    l.error = true;
                }
                return;
            };
            self.annotate.adv.link = None;
            // Les zones de lien en cache sont périmées : relues après la modification.
            self.annotate.adv.links_page = None;
            let op = match p.index {
                Some(index) => {
                    self.annotate.adv.reselect = Some((p.page, index));
                    EditOp::SetLinkTarget {
                        page: p.page,
                        index,
                        target,
                    }
                }
                None => EditOp::AddLink {
                    page: p.page,
                    rect: p.rect,
                    target,
                },
            };
            self.worker.send(Request::Edit(op));
        } else if cancel {
            self.annotate.adv.link = None;
        }
    }

    fn thread_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(prompt) = self.annotate.adv.thread.as_mut() else {
            return;
        };
        let reply = matches!(prompt.kind, ThreadKind::Reply { .. });
        let mut submit = false;
        let mut cancel = false;
        let (title, sub, hint, ok) = if reply {
            (
                "dialog.reply.title",
                "dialog.reply.subtitle",
                "dialog.reply.placeholder",
                "dialog.reply.ok",
            )
        } else {
            (
                "dialog.comment.title",
                "dialog.comment.subtitle",
                "dialog.comment.placeholder",
                "dialog.free_text.ok_edit",
            )
        };
        let spec = crate::dialog::Spec::new(
            "annot-thread",
            tr(lang, title),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, sub))
        .icon(glyph::COMMENT, "💬")
        .width(440.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                let field = ui.add(
                    egui::TextEdit::multiline(&mut prompt.text)
                        .hint_text(tr(lang, hint))
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                if prompt.focus {
                    field.request_focus();
                    prompt.focus = false;
                }
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, ok))
                    || ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if submit {
            let Some(p) = self.annotate.adv.thread.take() else {
                return;
            };
            let text = p.text.trim().to_string();
            match p.kind {
                ThreadKind::Reply { parent } if !text.is_empty() => {
                    let author = self.annot_author();
                    self.worker.send(Request::Edit(EditOp::Reply {
                        page: p.page,
                        parent,
                        text,
                        author,
                    }));
                }
                ThreadKind::Contents { index } => {
                    self.annotate.adv.reselect = Some((p.page, index));
                    self.worker.send(Request::Edit(EditOp::SetAnnotationText {
                        page: p.page,
                        index,
                        text,
                    }));
                }
                ThreadKind::Reply { .. } => {}
            }
        } else if cancel {
            self.annotate.adv.thread = None;
        }
    }

    /// Confirmation avant d'ouvrir une adresse externe : rien ne s'ouvre sans accord explicite.
    fn url_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(url) = self.annotate.adv.url_confirm.clone() else {
            return;
        };
        let p = theme::palette(self.settings.dark_theme);
        let (mut open, mut copy, mut cancel) = (false, false, false);
        let spec = crate::dialog::Spec::new(
            "url-confirm",
            tr(lang, "dialog.url.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "dialog.url.subtitle"))
        .icon(glyph::GLOBE, "🔗")
        .warning()
        .width(520.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(RichText::new(tr(lang, "dialog.url.body")).color(p.text_muted));
                ui.add_space(4.0);
                Frame::new()
                    .fill(p.chrome)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        // Adresse complète, sélectionnable : l'utilisateur voit ce qui sera ouvert.
                        ui.add(
                            egui::Label::new(RichText::new(&url).monospace().size(13.0))
                                .selectable(true)
                                .wrap(),
                        );
                    });
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "dialog.url.open")) {
                    open = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.url.copy")) {
                    copy = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if open {
            self.annotate.adv.url_confirm = None;
            ctx.open_url(egui::OpenUrl::new_tab(url));
        } else if copy {
            ctx.copy_text(url);
            self.status_msg = Some(tr(lang, "status.copied").to_string());
            self.annotate.adv.url_confirm = None;
        } else if cancel {
            self.annotate.adv.url_confirm = None;
        }
    }
}

// ----- Panneau Commentaires ---------------------------------------------------------------------

enum PanelAction {
    Select(usize),
    Reply(usize),
    Status(usize, ReviewStatus),
    DeleteThread(usize),
    DeleteQuick(usize),
}

impl SuniApp {
    pub fn comments_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let Some(page) = self.annotate.annotations_page else {
            // La lecture de la page est en route : en-tête seul.
            self.comments_header(ctx, ui, 0, 0);
            return;
        };
        let details = self.annotate.adv.details.clone();
        let quick = self.annotate.annotations.clone();
        let roots: Vec<AnnotDetail> = thread_roots(&details).into_iter().cloned().collect();
        let count = if details.is_empty() {
            quick.len()
        } else {
            roots.len()
        };
        self.comments_header(ctx, ui, page, count);
        let mut actions: Vec<PanelAction> = Vec::new();
        let selected = self.annotate.adv.selected.as_ref().map(|(_, d)| d.index);
        if count == 0 {
            ui.label(RichText::new(tr(lang, "comments.empty")).color(p.text_muted));
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if details.is_empty() {
                    // Lecture détaillée indisponible : liste à plat de PDFium.
                    for a in &quick {
                        Frame::new()
                            .fill(p.surface)
                            .stroke(Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .inner_margin(10)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(icons::semibold(
                                        crate::annotate::annotation_label(lang, a.kind),
                                        13.0,
                                    ));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if self
                                            .trash_button(ui)
                                            .on_hover_text(tr(lang, "comments.delete"))
                                            .clicked()
                                        {
                                            actions.push(PanelAction::DeleteQuick(a.index));
                                        }
                                    });
                                });
                                if let Some(c) = &a.contents {
                                    ui.label(RichText::new(c).size(13.0));
                                }
                            });
                        ui.add_space(6.0);
                    }
                    return;
                }
                for root in &roots {
                    let status = thread_status(&details, root.index);
                    let replies = thread_replies(&details, root.index);
                    let is_selected = selected == Some(root.index);
                    Frame::new()
                        .fill(if is_selected {
                            p.accent_soft
                        } else {
                            p.surface
                        })
                        .stroke(Stroke::new(
                            1.0,
                            if is_selected { p.accent } else { p.border },
                        ))
                        .corner_radius(8)
                        .inner_margin(10)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let label = ui.add(
                                    egui::Label::new(icons::semibold(
                                        kind_label(lang, &root.subtype),
                                        13.0,
                                    ))
                                    .sense(egui::Sense::click()),
                                );
                                if label.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                    actions.push(PanelAction::Select(root.index));
                                }
                                if status != ReviewStatus::None {
                                    ui.label(
                                        RichText::new(tr(lang, status_key(status)))
                                            .size(11.5)
                                            .strong()
                                            .color(status_color(status)),
                                    );
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if self
                                        .trash_button(ui)
                                        .on_hover_text(tr(
                                            lang,
                                            if replies.is_empty() {
                                                "comments.delete"
                                            } else {
                                                "comments.delete_thread"
                                            },
                                        ))
                                        .clicked()
                                    {
                                        actions.push(PanelAction::DeleteThread(root.index));
                                    }
                                });
                            });
                            self.meta_line(ui, root, lang);
                            if let Some(c) = &root.contents {
                                ui.label(RichText::new(c).size(13.0));
                            }
                            for r in &replies {
                                ui.add_space(4.0);
                                // Réponse en retrait : marge extérieure (pas de rangée imbriquée,
                                // dont la largeur grandirait à chaque image).
                                Frame::new()
                                    .fill(p.chrome)
                                    .corner_radius(6)
                                    .inner_margin(egui::Margin::symmetric(8, 5))
                                    .outer_margin(egui::Margin {
                                        left: 10,
                                        ..Default::default()
                                    })
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            self.meta_line(ui, r, lang);
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if self
                                                        .trash_button(ui)
                                                        .on_hover_text(tr(lang, "comments.delete"))
                                                        .clicked()
                                                    {
                                                        actions.push(PanelAction::DeleteThread(
                                                            r.index,
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                        if let Some(c) = &r.contents {
                                            ui.label(RichText::new(c).size(12.5));
                                        }
                                    });
                            }
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if ui.small_button(tr(lang, "comments.reply")).clicked() {
                                    actions.push(PanelAction::Reply(root.index));
                                }
                                let mut chosen = None;
                                egui::ComboBox::from_id_salt(("review-status", root.index))
                                    .selected_text(
                                        RichText::new(tr(lang, status_key(status))).size(12.5),
                                    )
                                    .width(110.0)
                                    .show_ui(ui, |ui| {
                                        for s in ReviewStatus::ALL {
                                            if ui
                                                .selectable_label(
                                                    s == status,
                                                    tr(lang, status_key(s)),
                                                )
                                                .clicked()
                                                && s != status
                                            {
                                                chosen = Some(s);
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text(tr(lang, "comments.status"));
                                if let Some(s) = chosen {
                                    actions.push(PanelAction::Status(root.index, s));
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        for a in actions {
            match a {
                PanelAction::Select(index) => {
                    if let Some(d) = details.iter().find(|d| d.index == index).cloned() {
                        self.execute(ctx, Command::ToolSelectAnnot);
                        self.select_annot(page, d);
                    }
                }
                PanelAction::Reply(parent) => self.start_reply(page, parent),
                PanelAction::Status(parent, status) => {
                    let author = self.annot_author();
                    self.worker.send(Request::Edit(EditOp::SetReviewStatus {
                        page,
                        parent,
                        status,
                        author,
                    }));
                }
                PanelAction::DeleteThread(index) => {
                    if self
                        .annotate
                        .adv
                        .selected
                        .as_ref()
                        .is_some_and(|(_, d)| d.index == index)
                    {
                        self.annotate.adv.selected = None;
                    }
                    self.worker
                        .send(Request::Edit(EditOp::DeleteAnnotationThread {
                            page,
                            index,
                        }));
                }
                PanelAction::DeleteQuick(index) => self
                    .worker
                    .send(Request::Edit(EditOp::DeleteAnnotation { page, index })),
            }
        }
    }

    fn trash_button(&self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(Button::new(icons::icon(self.has_icons, glyph::DELETE, "🗑", 14.0)).frame(false))
    }

    /// « Auteur · date » d'une annotation, en petit.
    fn meta_line(&self, ui: &mut egui::Ui, d: &AnnotDetail, lang: Language) {
        let muted = theme::palette(self.settings.dark_theme).text_muted;
        let date = d.modified.as_deref().and_then(|m| format_pdf_date(m, lang));
        let text = match (&d.author, date) {
            (Some(a), Some(t)) => format!("{a} · {t}"),
            (Some(a), None) => a.clone(),
            (None, Some(t)) => t,
            (None, None) => return,
        };
        ui.label(RichText::new(text).size(11.5).color(muted));
    }

    fn comments_header(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        page: usize,
        count: usize,
    ) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, "comments.title"), 15.0));
            ui.label(
                RichText::new(count.to_string())
                    .color(p.text_muted)
                    .size(13.0),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        Button::new(icons::icon(self.has_icons, glyph::CLOSE, "✕", 14.0))
                            .frame(false),
                    )
                    .on_hover_text(tr(lang, "tools.close"))
                    .clicked()
                {
                    self.execute(ctx, Command::ToggleComments);
                }
            });
        });
        ui.separator();
        // Auteur des annotations, réponses et statuts créés ensuite (mémorisé, vide : omis).
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(lang, "comments.author"))
                    .size(12.0)
                    .color(p.text_muted),
            );
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.settings.author)
                    .hint_text(tr(lang, "comments.author_hint"))
                    .desired_width(ui.available_width().max(60.0)),
            );
            if field.lost_focus() {
                self.settings.author = self.settings.author.trim().to_string();
                self.persist_settings();
            }
        });
        ui.label(
            RichText::new(tr_args(
                lang,
                "comments.page",
                &[("page", &(page + 1).to_string())],
            ))
            .size(12.0)
            .color(p.text_muted),
        );
        ui.add_space(4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precise_link_controls_preserve_null_axes_zoom_and_named_targets_in_both_languages() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::SelectAnnot;
            let size = app.document.as_ref().unwrap().info.page_sizes[0];
            app.document.as_mut().unwrap().info.page_sizes = vec![size; 4];
            for target in [
                LinkTarget::Destination(Destination {
                    page: 2,
                    view: DestinationView::Xyz {
                        x: None,
                        y: Some(35.0),
                        zoom: Some(1.75),
                    },
                }),
                LinkTarget::Destination(Destination {
                    page: 1,
                    view: DestinationView::FitWidth {
                        x: Some(20.0),
                        y: None,
                    },
                }),
                LinkTarget::Named("Café".into()),
            ] {
                let mut detail = editable_link();
                detail.link = Some(target.clone());
                app.select_annot(0, detail);
                app.edit_selected_annot_link();
                let prompt = app.annotate.adv.link.as_ref().unwrap();
                assert_eq!(
                    prompt.precise.target(prompt.page_number),
                    Some(target.clone())
                );
                click_link_control(&mut app, &ctx, "dialog.link.ok_edit");
                assert!(
                    matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::SetLinkTarget { target: actual, .. }) if actual == target)
                );
                assert!(matches!(
                    requests.try_recv().unwrap(),
                    Request::FinishMutation
                ));
                app.worker.finish_mutation();
                assert!(requests.try_recv().is_err());
            }
            let mut invalid = PreciseLinkPrompt {
                view: LinkViewport::Xyz,
                zoom: "0".into(),
                ..Default::default()
            };
            assert!(invalid.target(1).is_none());
            invalid.zoom.clear();
            invalid.x = "NaN".into();
            assert!(invalid.target(1).is_none());
        }
    }

    fn ink_gesture(app: &mut SuniApp, page: usize, start: (f32, f32), end: (f32, f32)) {
        let drag = Drag {
            page,
            start,
            current: end,
        };
        app.adv_drag_start(page, start);
        app.adv_drag_moved(&drag);
        app.adv_drag_finished(drag);
    }

    #[test]
    fn grouped_ink_buttons_commit_once_retry_without_loss_and_cancel_in_both_languages() {
        use crate::worker::{DocState, Response};
        for language in Language::ALL {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::Ink;
            ink_gesture(&mut app, 0, (20.0, 30.0), (80.0, 90.0));
            app.adv_click(0, (100.0, 110.0));
            let expected = app.annotate.adv.ink_draft.as_ref().unwrap().strokes.clone();
            assert_eq!(expected.len(), 2);
            assert_eq!(expected[1], vec![(100.0, 110.0)]);
            assert!(requests.try_recv().is_err() && !app.dirty && app.has_unsaved_changes());
            assert!(!app.dialog_open());
            app.edit.stroke_rgb = [12, 34, 56];
            click_link_control(&mut app, &ctx, "ink.finish");
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::AddInk {
                page: 0, strokes, style, ..
            }) if strokes == expected && style.rgb == (12, 34, 56))
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.finish_ink_draft();
            app.cancel_ink_draft();
            assert!(app.annotate.adv.ink_draft.as_ref().unwrap().pending);
            assert!(requests.try_recv().is_err());
            responses
                .send(Response::OperationFailed(sunipdf_core::EngineError::Other(
                    "test refusal".into(),
                )))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&ctx);
            assert_eq!(app.status_msg.as_deref(), Some(tr(language, "ink.retry")));
            assert!(!app.annotate.adv.ink_draft.as_ref().unwrap().pending);
            assert_eq!(
                app.annotate.adv.ink_draft.as_ref().unwrap().strokes,
                expected
            );
            click_link_control(&mut app, &ctx, "ink.finish");
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::AddInk { strokes, .. }) if strokes == expected)
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            let info = app.document.as_ref().unwrap().info.clone();
            responses
                .send(Response::DocumentChanged(DocState {
                    info,
                    can_undo: true,
                    can_redo: false,
                    pages_unchanged: true,
                    measurement_scales_unchanged: false,
                    modified: true,
                    has_signatures: false,
                }))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&ctx);
            assert!(!app.ink_draft_active() && app.dirty && app.can_undo);
            requests.try_iter().for_each(drop);
            app.adv_click(0, (40.0, 50.0));
            click_link_control(&mut app, &ctx, "ink.cancel");
            assert!(!app.ink_draft_active() && app.dirty);
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn grouped_ink_keeps_page_and_visible_stroke_and_point_limits() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.tool = Tool::Ink;
        for i in 0..INK_MAX_STROKES {
            app.adv_click(0, (i as f32, 20.0));
        }
        app.adv_click(1, (10.0, 10.0));
        assert_eq!(app.status_msg.as_deref(), Some(app.t("ink.same_page")));
        app.adv_click(0, (200.0, 20.0));
        assert_eq!(app.status_msg.as_deref(), Some(app.t("ink.stroke_limit")));
        assert_eq!(
            app.annotate.adv.ink_draft.as_ref().unwrap().strokes.len(),
            64
        );
        app.cancel_ink_draft();
        app.adv_drag_start(0, (0.0, 10.0));
        for i in 1..=INK_MAX_POINTS {
            app.adv_drag_moved(&Drag {
                page: 0,
                start: (0.0, 10.0),
                current: (i as f32, 10.0),
            });
        }
        assert_eq!(app.annotate.adv.ink.len(), 4000);
        assert_eq!(app.status_msg.as_deref(), Some(app.t("ink.point_limit")));
        app.adv_drag_finished(Drag {
            page: 0,
            start: (0.0, 10.0),
            current: (5000.0, 10.0),
        });
        app.adv_click(0, (40.0, 10.0));
        let draft = app.annotate.adv.ink_draft.as_ref().unwrap();
        assert_eq!(draft.strokes.len(), 2);
        assert_eq!(draft.strokes[0].last(), Some(&(3999.0, 10.0)));
        assert_eq!(draft.strokes[1], vec![(40.0, 10.0)]);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn grouped_ink_preview_retains_completed_paths_and_dots_on_original_page() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.tool = Tool::Ink;
        ink_gesture(&mut app, 0, (10.0, 20.0), (50.0, 60.0));
        app.adv_click(0, (80.0, 90.0));
        app.adv_drag_start(0, (100.0, 110.0));
        let ctx = egui::Context::default();
        let mut own_shapes = None;
        let mut other_shapes = None;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let painter = ui.painter();
            let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(612.0, 792.0));
            let before = painter.ctx().graphics(|graphics| {
                graphics
                    .get(painter.layer_id())
                    .map_or(0, |list| list.next_idx().0)
            });
            app.paint_annot_adv(painter, bounds, 0, 792.0);
            let after = painter.ctx().graphics(|graphics| {
                graphics
                    .get(painter.layer_id())
                    .map_or(0, |list| list.next_idx().0)
            });
            app.paint_annot_adv(painter, bounds, 1, 792.0);
            let other = painter.ctx().graphics(|graphics| {
                graphics
                    .get(painter.layer_id())
                    .map_or(0, |list| list.next_idx().0)
            });
            own_shapes = Some(after - before);
            other_shapes = Some(other - after);
        });
        output.textures_delta.clear();
        assert_eq!(
            own_shapes,
            Some(3),
            "completed line, completed dot, active dot"
        );
        assert_eq!(other_shapes, Some(0));
        assert!(requests.try_recv().is_err());
    }

    fn detail(index: usize, subtype: &str, rect: PointRect) -> AnnotDetail {
        AnnotDetail {
            index,
            subtype: subtype.into(),
            rect,
            contents: None,
            author: None,
            modified: None,
            name: None,
            rgb: None,
            fill_rgb: None,
            width_pt: None,
            dash_pattern: Vec::new(),
            opacity: 1.0,
            font_size: None,
            parent: None,
            status: None,
            link: None,
            arrow: false,
            hidden: false,
            locked: false,
            ours: true,
            can_move: true,
            can_resize: true,
            can_restyle: true,
            can_edit_text: true,
            can_edit_link: false,
            replies: 0,
        }
    }

    fn r(l: f32, b: f32, rt: f32, t: f32) -> PointRect {
        PointRect {
            left: l,
            bottom: b,
            right: rt,
            top: t,
        }
    }

    fn editable_link() -> AnnotDetail {
        let mut link = detail(2, "Link", r(10.0, 10.0, 100.0, 30.0));
        link.link = Some(LinkTarget::Uri("https://example.org/old".into()));
        link.can_edit_text = false;
        link.can_edit_link = true;
        link.can_restyle = false;
        link
    }

    fn link_frame(
        app: &mut SuniApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 780.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(bounds),
                events,
                ..Default::default()
            },
            |_| {
                if app.annotate.adv.free_text.is_some() {
                    let height = app.page_size_pt(0).1;
                    app.free_text_editor(ctx, bounds.shrink(40.0), 0, height);
                } else {
                    app.annot_bar(ctx, bounds);
                    app.link_dialog(ctx);
                }
            },
        );
        output.textures_delta.clear();
        output
    }

    fn click_link_control(app: &mut SuniApp, ctx: &egui::Context, key: &str) {
        let mut target = None;
        for _ in 0..2 {
            let output = link_frame(app, ctx, vec![]);
            if let Some(tree) = output.platform_output.accesskit_update {
                if let Some(bounds) = tree
                    .nodes
                    .iter()
                    .map(|(_, node)| node)
                    .find(|node| node.label() == Some(app.t(key)))
                    .and_then(|node| node.bounds())
                {
                    assert!(bounds.x0 >= 0.0 && bounds.x1 <= 1280.0 && bounds.y1 <= 780.0);
                    target = Some(egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.0) as f32,
                        ((bounds.y0 + bounds.y1) / 2.0) as f32,
                    ));
                }
            }
        }
        let pos = target.expect("visible accessible link control");
        for pressed in [true, false] {
            link_frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn free_text_inline_add_save_cancel_and_retry_preserve_multiline_input_in_both_languages() {
        use crate::worker::Response;
        for language in Language::ALL {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::FreeText;
            app.adv_click(0, (40.0, 700.0));
            assert!(app.annotate.adv.free_text.is_some() && requests.try_recv().is_err());
            link_frame(&mut app, &ctx, vec![]);
            link_frame(
                &mut app,
                &ctx,
                vec![egui::Event::Text("Hello\nBonjour".into())],
            );
            let prompt = app.annotate.adv.free_text.as_mut().unwrap();
            assert_eq!(prompt.text, "Hello\nBonjour");
            prompt.size = 18.0;
            prompt.rgb = [20, 40, 60];
            prompt.fill = true;
            prompt.fill_rgb = [240, 230, 220];
            prompt.border = 1.5;
            let rect = prompt.rect;
            assert!(requests.try_recv().is_err() && !app.dirty && app.has_unsaved_changes());
            link_frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(
                requests.try_recv().is_err(),
                "Enter must only insert a line break"
            );
            let typed = app.annotate.adv.free_text.as_ref().unwrap().text.clone();
            click_link_control(&mut app, &ctx, "dialog.free_text.ok");
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::AddFreeText {
                page: 0, rect: actual, text, style, ..
            }) if actual == rect && text == typed && style.size_pt == 18.0
                && style.rgb == (20,40,60) && style.fill_rgb == Some((240,230,220))
                && style.border_pt == 1.5)
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.submit_free_text();
            assert!(
                requests.try_recv().is_err(),
                "pending submission must not be duplicated"
            );
            responses
                .send(Response::OperationFailed(sunipdf_core::EngineError::Other(
                    "refused".into(),
                )))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&ctx);
            assert_eq!(app.annotate.adv.free_text.as_ref().unwrap().text, typed);
            assert!(!app.annotate.adv.free_text.as_ref().unwrap().pending);
            assert_eq!(
                app.status_msg.as_deref(),
                Some(tr(language, "free_text.retry"))
            );
            click_link_control(&mut app, &ctx, "dialog.password.cancel");
            assert!(app.annotate.adv.free_text.is_none() && requests.try_recv().is_err());

            app.annotate.tool = Tool::SelectAnnot;
            let mut selected = detail(3, "FreeText", rect);
            selected.contents = Some("Existing text".into());
            selected.font_size = Some(12.0);
            selected.rgb = Some((1, 2, 3));
            selected.fill_rgb = Some((230, 240, 250));
            selected.width_pt = Some(2.0);
            app.select_annot(0, selected);
            app.edit_selected_annot_text();
            let prompt = app.annotate.adv.free_text.as_mut().unwrap();
            assert_eq!(prompt.text, "Existing text");
            assert_eq!(
                (prompt.size, prompt.rgb, prompt.fill_rgb, prompt.border),
                (12.0, [1, 2, 3], [230, 240, 250], 2.0)
            );
            prompt.text = "Edited\nModifie".into();
            click_link_control(&mut app, &ctx, "dialog.free_text.ok_edit");
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::UpdateFreeText {
                page: 0, index: 3, text, ..
            }) if text == "Edited\nModifie")
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            assert_eq!(app.annotate.adv.reselect, Some((0, 3)));
        }
    }

    #[test]
    fn free_text_inline_validation_busy_and_outside_click_never_discard_or_commit_input() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        ctx.enable_accesskit();
        app.annotate.tool = Tool::FreeText;
        app.adv_click(0, (30.0, 700.0));
        let original = app.annotate.adv.free_text.as_ref().unwrap().clone();
        for bad in 0..6 {
            let p = app.annotate.adv.free_text.as_mut().unwrap();
            *p = original.clone();
            p.text = "Keep my text".into();
            match bad {
                0 => p.text = " \n ".into(),
                1 => p.size = f32::NAN,
                2 => p.border = 51.0,
                3 => p.rect.right = p.rect.left + 2.0 * p.size - 0.1,
                4 => p.rect.top = p.rect.bottom + 1.9,
                _ => p.rect.left = f32::INFINITY,
            }
            assert!(!p.valid());
            app.submit_free_text();
            assert!(app.annotate.adv.free_text.is_some() && requests.try_recv().is_err());
        }
        app.annotate.adv.free_text = Some(original);
        app.annotate.adv.free_text.as_mut().unwrap().text = "Keep my text".into();
        for busy in 0..4 {
            app.saving = busy == 0;
            app.processing.busy = busy == 1;
            if busy == 2 {
                app.worker.send(Request::Undo);
            }
            app.forms.pending_clear = busy == 3;
            app.submit_free_text();
            assert!(!app.annotate.adv.free_text.as_ref().unwrap().pending);
            app.worker.finish_mutation();
            while requests.try_recv().is_ok() {}
        }
        app.saving = false;
        app.processing.busy = false;
        app.forms.pending_clear = false;
        link_frame(&mut app, &ctx, vec![]);
        for pressed in [true, false] {
            link_frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerButton {
                    pos: egui::pos2(1200.0, 750.0),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        assert_eq!(
            app.annotate.adv.free_text.as_ref().unwrap().text,
            "Keep my text"
        );
        assert!(requests.try_recv().is_err());
        click_link_control(&mut app, &ctx, "dialog.password.cancel");
        assert!(app.annotate.adv.free_text.is_none());
        app.adv_click(0, (30.0, 700.0));
        link_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.annotate.adv.free_text.is_none() && requests.try_recv().is_err());
    }

    #[test]
    fn free_text_anchor_zoom_page_and_stale_context_are_guarded() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.tool = Tool::FreeText;
        app.adv_drag_finished(Drag {
            page: 0,
            start: (240.0, 600.0),
            current: (40.0, 700.0),
        });
        let original = app.annotate.adv.free_text.as_ref().unwrap().clone();
        assert_eq!(original.rect, r(40.0, 600.0, 240.0, 700.0));
        let page = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(600.0, 800.0));
        assert_eq!(
            original.screen_rect(page, 2.0, 800.0),
            egui::Rect::from_min_max(egui::pos2(90.0, 220.0), egui::pos2(490.0, 420.0))
        );
        let zoom = app.zoom;
        app.execute(&ctx, Command::ZoomIn);
        assert_ne!(app.zoom, zoom);
        assert!(app.annotate.adv.free_text.is_some());
        app.go_to_page(1);
        assert_eq!(app.document.as_ref().unwrap().page, 0);
        for command in [
            Command::Save,
            Command::ToolInk,
            Command::NextPage,
            Command::ViewSinglePage,
            Command::CloseDocument,
        ] {
            app.execute(&ctx, command);
            assert!(app.annotate.adv.free_text.is_some() && requests.try_recv().is_err());
        }
        for stale in 0..5 {
            app.annotate.adv.free_text = Some(original.clone());
            app.annotate.tool = original.tool;
            app.annotate.workspace = original.workspace;
            app.view.epoch = original.epoch;
            match stale {
                0 => app.view.epoch += 1,
                1 => app.annotate.tool = Tool::Ink,
                2 => app.annotate.workspace = Some(crate::workspace::Workspace::Organize),
                3 => {
                    let _ = app.take_tab();
                }
                _ => app.annotate.adv.invalidate(),
            }
            app.validate_free_text();
            assert!(app.annotate.adv.free_text.is_none(), "stale case {stale}");
        }
    }

    #[test]
    fn free_text_start_guards_busy_drafts_and_locked_existing_annotations() {
        for busy in 0..8 {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.annotate.tool = Tool::FreeText;
            match busy {
                0 => app.saving = true,
                1 => app.opening = Some("pending.pdf".into()),
                2 => app.worker.send(Request::Undo),
                3 => {
                    app.forms.drafts.insert("Name".into(), "Unsaved".into());
                }
                4 => app.annotate.adv.ink_draft = Some(InkDraft::default()),
                5 => app.form_builder.visible = true,
                6 => app.tabs.resuming = Some(1),
                _ => app.sign.placing = Some(crate::sign::Placing::Text("Date".into())),
            }
            app.adv_click(0, (40.0, 700.0));
            assert!(app.annotate.adv.free_text.is_none(), "busy case {busy}");
            assert!(!requests.try_iter().any(|r| matches!(r, Request::Edit(_))));
        }
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.tool = Tool::SelectAnnot;
        let mut locked = detail(0, "FreeText", r(40.0, 600.0, 220.0, 650.0));
        locked.can_edit_text = false;
        app.select_annot(0, locked);
        app.edit_selected_annot_text();
        assert!(app.annotate.adv.free_text.is_none() && requests.try_recv().is_err());
    }

    #[test]
    fn free_text_revision_and_tab_reloads_remove_stale_indices_after_worker_success() {
        use crate::worker::{DocState, Response};
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.tool = Tool::FreeText;
        app.adv_click(0, (40.0, 700.0));
        app.annotate.adv.free_text.as_mut().unwrap().text = "Saved once".into();
        app.submit_free_text();
        while requests.try_recv().is_ok() {}
        responses
            .send(Response::DocumentChanged(DocState {
                info: app.document.as_ref().unwrap().info.clone(),
                can_undo: true,
                can_redo: false,
                modified: true,
                pages_unchanged: true,
                measurement_scales_unchanged: false,
                has_signatures: false,
            }))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(app.annotate.adv.free_text.is_none() && app.dirty);
        app.adv_click(0, (40.0, 700.0));
        assert!(app.annotate.adv.free_text.is_some());
        let state = app.take_tab();
        assert!(state.annotate.adv.free_text.is_none());
        app.put_tab(state);
        assert!(app.annotate.adv.free_text.is_none());
    }

    #[test]
    fn link_target_buttons_prefill_and_replace_once_in_both_languages() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::SelectAnnot;
            app.select_annot(0, editable_link());
            click_link_control(&mut app, &ctx, "bar.annot_edit_link");
            assert!(app.dialog_open() && requests.try_recv().is_err());
            let prompt = app.annotate.adv.link.as_mut().unwrap();
            assert_eq!(prompt.index, Some(2));
            assert_eq!(prompt.url, "https://example.org/old");
            assert!(!prompt.to_page);
            prompt.url = "https://example.org/new?source=pdf".into();
            click_link_control(&mut app, &ctx, "dialog.link.ok_edit");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::SetLinkTarget { page: 0, index: 2, target: LinkTarget::Uri(url) })
                    if url == "https://example.org/new?source=pdf"
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            assert!(requests.try_recv().is_err());
            assert!(app.annotate.adv.link.is_none() && app.annotate.adv.links_page.is_none());
            assert_eq!(app.annotate.adv.reselect, Some((0, 2)));
            assert!(app.annotate.adv.url_confirm.is_none());
            assert!(
                !app.dirty && !app.can_undo,
                "only the worker commits the edit"
            );
            let mut updated = editable_link();
            updated.link = Some(LinkTarget::Uri("https://example.org/new?source=pdf".into()));
            app.annotate.annotations_page = Some(0);
            app.on_annotations(0, vec![], Some(vec![updated.clone()]));
            assert_eq!(app.annotate.adv.selected.unwrap().1.link, updated.link);
            assert!(app.annotate.adv.reselect.is_none());
        }
    }

    #[test]
    fn link_target_dialog_rejects_unsafe_or_unknown_destinations_and_cancel_keeps_original() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::SelectAnnot;
            app.select_annot(0, editable_link());
            click_link_control(&mut app, &ctx, "bar.annot_edit_link");
            for url in ["javascript:alert(1)", "file:///C:/secret.pdf", ""] {
                app.annotate.adv.link.as_mut().unwrap().url = url.into();
                click_link_control(&mut app, &ctx, "dialog.link.ok_edit");
                assert!(app.annotate.adv.link.as_ref().unwrap().error);
                assert!(requests.try_recv().is_err() && app.annotate.adv.url_confirm.is_none());
            }
            click_link_control(&mut app, &ctx, "dialog.password.cancel");
            assert!(app.annotate.adv.link.is_none() && requests.try_recv().is_err());
            assert_eq!(
                app.annotate.adv.selected.as_ref().unwrap().1.link,
                editable_link().link
            );
            let mut unsupported = editable_link();
            unsupported.link = Some(LinkTarget::Unsupported);
            app.select_annot(0, unsupported);
            click_link_control(&mut app, &ctx, "bar.annot_edit_link");
            let prompt = app.annotate.adv.link.as_ref().unwrap();
            assert!(!prompt.to_page && prompt.url.is_empty());
            click_link_control(&mut app, &ctx, "dialog.link.ok_edit");
            assert!(requests.try_recv().is_err());
            click_link_control(&mut app, &ctx, "dialog.link.kind_page");
            click_link_control(&mut app, &ctx, "dialog.link.ok_edit");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::SetLinkTarget {
                    page: 0,
                    index: 2,
                    target: LinkTarget::Page(0)
                })
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
        }
    }

    #[test]
    fn link_target_locked_controls_and_double_click_cannot_open_editor() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            app.annotate.tool = Tool::SelectAnnot;
            let mut locked = editable_link();
            locked.can_edit_link = false;
            app.select_annot(0, locked.clone());
            click_link_control(&mut app, &ctx, "bar.annot_edit_link");
            assert!(app.annotate.adv.link.is_none() && requests.try_recv().is_err());
            app.annotate.annotations_page = Some(0);
            app.annotate.adv.details = vec![locked];
            app.adv_double_click(0, (50.0, 20.0));
            assert!(app.annotate.adv.link.is_none() && requests.try_recv().is_err());
        }
    }

    #[test]
    fn link_target_uses_resolved_destination_and_discards_stale_dialogs() {
        use crate::worker::{DocState, Response};
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.tool = Tool::SelectAnnot;
        let doc = app.document.as_mut().unwrap();
        doc.info.page_sizes = vec![doc.info.page_sizes[0]; 5];
        let mut named = editable_link();
        named.link = Some(LinkTarget::Unsupported);
        app.annotate.annotations_page = Some(0);
        app.annotate.adv.details = vec![named];
        app.annotate.adv.links_page = Some(0);
        app.annotate.adv.links = vec![LinkInfo {
            index: 2,
            rect: r(10.0, 10.0, 100.0, 30.0),
            target: LinkTarget::Page(3),
            destination: Some(sunipdf_core::Destination {
                page: 3,
                view: sunipdf_core::DestinationView::Xyz {
                    x: Some(12.0),
                    y: Some(100.0),
                    zoom: Some(2.0),
                },
            }),
        }];
        app.adv_double_click(0, (50.0, 20.0));
        let prompt = app.annotate.adv.link.as_ref().unwrap();
        assert!(prompt.to_page && prompt.page_number == 4 && prompt.index == Some(2));
        let tab = app.take_tab();
        assert!(app.annotate.adv.link.is_none() && tab.annotate.adv.link.is_none());
        app.put_tab(tab);
        assert!(app.annotate.adv.link.is_none());
        for pages_unchanged in [true, false] {
            app.select_annot(0, editable_link());
            app.edit_selected_annot_link();
            assert!(app.annotate.adv.link.is_some());
            responses
                .send(Response::DocumentChanged(DocState {
                    info: app.document.as_ref().unwrap().info.clone(),
                    can_undo: true,
                    can_redo: false,
                    pages_unchanged,
                    measurement_scales_unchanged: false,
                    modified: true,
                    has_signatures: false,
                }))
                .unwrap();
            app.handle_responses(&ctx);
            assert!(app.annotate.adv.link.is_none());
            assert!(!requests
                .try_iter()
                .any(|request| matches!(request, Request::Edit(_))));
        }
    }

    #[test]
    fn dashed_shape_controls_create_each_shape_in_both_languages() {
        for language in Language::ALL {
            for (tool, shape) in [
                (Tool::AnnotRect, AnnotShape::Square),
                (Tool::AnnotEllipse, AnnotShape::Circle),
                (Tool::AnnotLine, AnnotShape::Line),
                (Tool::AnnotLine, AnnotShape::Arrow),
            ] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                let ctx = egui::Context::default();
                crate::icons::install(&ctx);
                ctx.enable_accesskit();
                app.settings.set_language(language);
                app.annotate.tool = tool;
                app.edit.arrow = shape == AnnotShape::Arrow;
                click_link_control(&mut app, &ctx, "bar.dashed");
                assert_eq!(app.edit.annot_dash_pattern, [6.0, 3.0]);
                assert!(requests.try_recv().is_err());
                app.adv_drag_finished(Drag {
                    page: 0,
                    start: (20.0, 30.0),
                    current: (120.0, 80.0),
                });
                assert!(matches!(
                    requests.try_recv().unwrap(),
                    Request::Edit(EditOp::AddAnnotShape { page: 0, shape: actual, style, .. })
                        if actual == shape && style.dash_pattern == [6.0, 3.0]
                ));
                assert!(matches!(
                    requests.try_recv().unwrap(),
                    Request::FinishMutation
                ));
                app.worker.finish_mutation();
                assert!(requests.try_recv().is_err());
                assert!(!app.dirty && !app.can_undo);
            }
        }
    }

    #[test]
    fn selected_shape_preserves_custom_dashes_until_the_checkbox_changes() {
        for language in Language::ALL {
            for subtype in ["Square", "Circle", "Line"] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                let ctx = egui::Context::default();
                crate::icons::install(&ctx);
                ctx.enable_accesskit();
                app.settings.set_language(language);
                app.annotate.tool = Tool::SelectAnnot;
                let mut annotation = detail(2, subtype, r(20.0, 30.0, 120.0, 80.0));
                annotation.dash_pattern = vec![4.0, 2.0, 1.0];
                app.select_annot(0, annotation.clone());
                assert_eq!(app.edit.annot_dash_pattern, annotation.dash_pattern);
                link_frame(&mut app, &ctx, vec![]);
                app.edit.stroke_rgb = [20, 40, 60];
                click_link_control(&mut app, &ctx, "bar.annot_apply");
                assert!(matches!(
                    requests.try_recv().unwrap(),
                    Request::Edit(EditOp::StyleAnnotation { page: 0, index: 2, style })
                        if style.dash_pattern == annotation.dash_pattern && style.rgb == (20, 40, 60)
                ));
                assert!(matches!(
                    requests.try_recv().unwrap(),
                    Request::FinishMutation
                ));
                app.worker.finish_mutation();
                assert!(requests.try_recv().is_err());
                app.annotate.annotations_page = Some(0);
                app.on_annotations(0, vec![], Some(vec![annotation]));
                assert_eq!(
                    app.annotate.adv.selected.as_ref().unwrap().1.dash_pattern,
                    [4.0, 2.0, 1.0]
                );
                click_link_control(&mut app, &ctx, "bar.dashed");
                assert!(app.edit.annot_dash_pattern.is_empty());
                click_link_control(&mut app, &ctx, "bar.dashed");
                assert_eq!(app.edit.annot_dash_pattern, [6.0, 3.0]);
                assert!(requests.try_recv().is_err());
            }
        }
    }

    #[test]
    fn unsupported_and_locked_annotations_do_not_offer_dashed_borders() {
        for language in Language::ALL {
            for subtype in [
                "Ink",
                "FreeText",
                "Highlight",
                "Link",
                "Square",
                "Circle",
                "Line",
            ] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                app.settings.set_language(language);
                app.annotate.tool = Tool::SelectAnnot;
                let mut annotation = detail(2, subtype, r(20.0, 30.0, 120.0, 80.0));
                if matches!(subtype, "Square" | "Circle" | "Line") {
                    annotation.locked = true;
                    annotation.can_restyle = false;
                }
                app.select_annot(0, annotation);
                for _ in 0..2 {
                    let output = link_frame(&mut app, &ctx, vec![]);
                    assert!(!output
                        .platform_output
                        .accesskit_update
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(app.t("bar.dashed"))));
                }
                assert!(requests.try_recv().is_err());
            }
            for tool in [Tool::Ink, Tool::FreeText] {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.annotate.tool = tool;
                app.edit.annot_dash_pattern = vec![6.0, 3.0];
                assert!(app.annot_style(false).dash_pattern.is_empty());
                assert!(!bar_kind(tool, None).unwrap().dashed);
            }
        }
    }

    #[test]
    fn dash_preview_keeps_odd_patterns_zero_entries_and_phase_across_edges() {
        let stroke = Stroke::new(1.0, egui::Color32::BLACK);
        let points = [egui::pos2(0.0, 0.0), egui::pos2(18.0, 0.0)];
        let ranges = |shapes: Vec<egui::Shape>| -> Vec<(f32, f32)> {
            shapes
                .into_iter()
                .filter_map(|shape| match shape {
                    egui::Shape::LineSegment { points, .. } if points[0] != points[1] => {
                        Some((points[0].x, points[1].x))
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            ranges(dash_shapes(&points, &[3.0, 2.0, 1.0], 1.0, stroke).unwrap()),
            [
                (0.0, 3.0),
                (5.0, 6.0),
                (9.0, 11.0),
                (12.0, 15.0),
                (17.0, 18.0)
            ]
        );
        assert_eq!(
            ranges(dash_shapes(&points, &[0.0, 0.0, 3.0], 1.0, stroke).unwrap()),
            [(0.0, 3.0), (6.0, 9.0), (12.0, 15.0)]
        );
        let bent = [
            egui::pos2(0.0, 0.0),
            egui::pos2(4.0, 0.0),
            egui::pos2(4.0, 8.0),
        ];
        let shapes = dash_shapes(&bent, &[6.0, 3.0], 1.0, stroke).unwrap();
        assert!(matches!(
            &shapes[1], egui::Shape::LineSegment { points, .. }
                if *points == [egui::pos2(4.0, 0.0), egui::pos2(4.0, 2.0)]
        ));
        for pattern in [
            vec![],
            vec![0.0, 0.0],
            vec![f32::NAN],
            vec![0.001, 3.0],
            vec![3.0; 17],
        ] {
            assert!(dash_shapes(&points, &pattern, 1.0, stroke).is_none());
        }
        assert!(dash_shapes(&points, &[0.1, 0.1], 0.1, stroke).is_some());
        assert!(dash_shapes(&points, &[6.0, 3.0], 0.0, stroke).is_none());
        assert!(dash_shapes(
            &[egui::Pos2::ZERO, egui::pos2(1_000_000.0, 0.0)],
            &[0.0, 0.5],
            1.0,
            stroke
        )
        .is_none());
    }

    #[test]
    fn hit_test_prefers_the_top_most_visible_annotation_and_lets_links_yield() {
        let mut list = vec![
            detail(0, "Square", r(0.0, 0.0, 100.0, 100.0)),
            detail(1, "Link", r(0.0, 0.0, 100.0, 100.0)),
            detail(2, "Ink", r(40.0, 40.0, 60.0, 60.0)),
            detail(3, "Popup", r(0.0, 0.0, 100.0, 100.0)),
            detail(4, "Widget", r(0.0, 0.0, 100.0, 100.0)),
        ];
        assert_eq!(hit_test(&list, (50.0, 50.0)).map(|d| d.index), Some(2));
        // Hors de l'encre : le carré passe avant le lien, malgré l'ordre du tableau.
        assert_eq!(hit_test(&list, (10.0, 10.0)).map(|d| d.index), Some(0));
        // Tolérance de 2 points.
        assert_eq!(hit_test(&list, (101.5, 50.0)).map(|d| d.index), Some(0));
        assert!(hit_test(&list, (110.0, 50.0)).is_none());
        // Seul un lien : il est sélectionnable.
        list.remove(2);
        list.remove(0);
        assert_eq!(hit_test(&list, (10.0, 10.0)).map(|d| d.index), Some(1));
        // Cachées, réponses et états n'ont pas de forme sur la page.
        let mut hidden = detail(0, "Square", r(0.0, 0.0, 10.0, 10.0));
        hidden.hidden = true;
        let mut reply = detail(1, "Text", r(0.0, 0.0, 10.0, 10.0));
        reply.parent = Some(0);
        let mut state = detail(2, "Text", r(0.0, 0.0, 10.0, 10.0));
        state.status = Some(ReviewStatus::Accepted);
        assert!(hit_test(&[hidden, reply, state], (5.0, 5.0)).is_none());
    }

    #[test]
    fn resizing_follows_the_corner_and_never_collapses() {
        let base = r(100.0, 100.0, 200.0, 150.0);
        let g = resized(base, Corner::TopRight, 20.0, 10.0);
        assert_eq!(
            (g.right, g.top, g.left, g.bottom),
            (220.0, 160.0, 100.0, 100.0)
        );
        let g = resized(base, Corner::BottomLeft, -10.0, -10.0);
        assert_eq!((g.left, g.bottom), (90.0, 90.0));
        // Passé de l'autre côté : rectangle retourné.
        let g = resized(base, Corner::TopRight, -150.0, 0.0);
        assert!(g.left < g.right && (g.left, g.right) == (50.0, 100.0));
        // Jamais plus petit que 4 points.
        let g = resized(base, Corner::TopRight, -100.0, -50.0);
        assert!(g.right - g.left >= 4.0 && g.top - g.bottom >= 4.0);
    }

    #[test]
    fn default_text_box_stays_inside_the_page() {
        let b = default_text_rect((100.0, 500.0), 612.0, 792.0);
        assert_eq!(
            (b.left, b.top, b.right, b.bottom),
            (100.0, 500.0, 280.0, 452.0)
        );
        let edge = default_text_rect((600.0, 20.0), 612.0, 792.0);
        assert!(edge.right <= 612.0 && edge.bottom >= 0.0 && edge.left >= 0.0);
        assert_eq!(edge.top - edge.bottom, TEXT_BOX_H);
        let top = default_text_rect((10.0, 800.0), 612.0, 792.0);
        assert!(top.top <= 792.0);
    }

    #[test]
    fn pdf_dates_are_formatted_for_the_language() {
        let d = "D:20261014123456Z";
        assert_eq!(
            format_pdf_date(d, Language::French).as_deref(),
            Some("14/10/2026 12:34")
        );
        assert_eq!(
            format_pdf_date(d, Language::English).as_deref(),
            Some("2026-10-14 12:34")
        );
        assert_eq!(
            format_pdf_date("D:20261014", Language::English).as_deref(),
            Some("2026-10-14 00:00")
        );
        assert!(format_pdf_date("hier", Language::French).is_none());
        assert!(format_pdf_date("D:2026", Language::French).is_none());
    }

    #[test]
    fn threads_group_replies_and_hide_states() {
        let mut note = detail(0, "Text", r(0.0, 0.0, 20.0, 20.0));
        note.ours = false;
        let mut reply = detail(1, "Text", r(0.0, 0.0, 20.0, 20.0));
        reply.parent = Some(0);
        let mut nested = detail(2, "Text", r(0.0, 0.0, 20.0, 20.0));
        nested.parent = Some(1);
        let mut state = detail(3, "Text", r(0.0, 0.0, 20.0, 20.0));
        state.parent = Some(0);
        state.status = Some(ReviewStatus::Completed);
        let other = detail(4, "Square", r(0.0, 0.0, 20.0, 20.0));
        let popup = detail(5, "Popup", r(0.0, 0.0, 20.0, 20.0));
        let all = vec![note, reply, nested, state, other, popup];
        let roots: Vec<usize> = thread_roots(&all).iter().map(|d| d.index).collect();
        assert_eq!(roots, [0, 4]);
        let replies: Vec<usize> = thread_replies(&all, 0).iter().map(|d| d.index).collect();
        assert_eq!(replies, [1, 2]);
        assert!(thread_replies(&all, 4).is_empty());
        assert_eq!(thread_status(&all, 0), ReviewStatus::Completed);
    }

    #[test]
    fn a_link_click_never_opens_anything_by_itself() {
        assert_eq!(link_action(&LinkTarget::Page(3)), LinkAction::GoTo(3));
        assert_eq!(
            link_action(&LinkTarget::Uri("  https://example.org/a?b=1 ".into())),
            LinkAction::Confirm("https://example.org/a?b=1".into())
        );
        assert_eq!(
            link_action(&LinkTarget::Uri("mailto:a@b.c".into())),
            LinkAction::Confirm("mailto:a@b.c".into())
        );
        for bad in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-msdt:id",
            r"\\srv\share",
            r"C:\Windows\calc.exe",
            "",
        ] {
            assert_eq!(
                link_action(&LinkTarget::Uri(bad.into())),
                LinkAction::Blocked,
                "{bad}"
            );
        }
        assert_eq!(
            link_action(&LinkTarget::Unsupported),
            LinkAction::Unsupported
        );
    }

    #[test]
    fn the_bar_offers_what_each_annotation_can_change() {
        assert!(bar_kind(Tool::Select, None).is_none());
        assert!(bar_kind(Tool::SelectAnnot, None).is_none());
        let ink = bar_kind(Tool::Ink, None).unwrap();
        assert!(ink.stroke && ink.width && !ink.fill && ink.opacity && !ink.arrow && !ink.dashed);
        let line = bar_kind(Tool::AnnotLine, None).unwrap();
        assert!(line.arrow && !line.fill && line.dashed);
        assert!(bar_kind(Tool::AnnotRect, None).unwrap().fill);
        let text = bar_kind(Tool::FreeText, None).unwrap();
        assert!(text.text && text.fill && !text.opacity && !text.dashed);
        let hl = detail(0, "Highlight", r(0.0, 0.0, 10.0, 10.0));
        let k = bar_kind(Tool::SelectAnnot, Some(&hl)).unwrap();
        assert!(k.stroke && k.opacity && !k.width && !k.fill && !k.dashed);
        let sq = detail(0, "Square", r(0.0, 0.0, 10.0, 10.0));
        let square = bar_kind(Tool::SelectAnnot, Some(&sq)).unwrap();
        assert!(square.fill && square.dashed);
    }
}
