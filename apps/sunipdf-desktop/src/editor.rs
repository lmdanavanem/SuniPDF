//! Édition du contenu des pages : sélection (simple, multiple, rectangle élastique), déplacement,
//! redimensionnement, rotation libre, presse-papiers d'objets, texte, images, formes, couleurs,
//! opacité. Les coordonnées sont en points d'affichage (celles du moteur).

use crate::annotate::{Drag, Tool};
use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::theme;
use crate::worker::{EditOp, Pick, PickMode, Request};
use egui::{CursorIcon, Key, Stroke};
use std::path::PathBuf;
use std::sync::Arc;
use sunipdf_core::{
    Command, FontFamily, ObjectInfo, ObjectKind, ObjectStyle, PointRect, ShapeKind, ShapeStyle,
    SystemFont, TextStyle, ZOrder,
};
use sunipdf_i18n::{tr, tr_args};

/// Tolérance de prise des poignées, en pixels écran.
const HANDLE_PX: f32 = 10.0;
/// Les poignées sont dessinées à cette distance (pixels) hors des limites de l'objet.
const HANDLE_OFFSET_PX: f32 = 3.0;
/// Distance (pixels) entre le cadre et la poignée de rotation.
const ROTATE_GAP_PX: f32 = 24.0;

/// Police d'un nouveau texte : une des trois polices standard, ou une police du système.
#[derive(Clone, Debug, PartialEq)]
pub enum TextFont {
    Standard(FontFamily),
    System(SystemFont),
}

#[derive(Clone)]
pub struct TextPrompt {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    /// `Some(index)` : modification d'un objet texte existant.
    pub index: Option<usize>,
    pub text: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub font: TextFont,
    pub rgb: [u8; 3],
    /// Éditeur posé sur la page (sinon boîte de dialogue : texte pivoté).
    pub inline: bool,
    /// Le clic qui a ouvert l'éditeur ne doit pas le valider.
    pub armed: bool,
    pub focus: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Corner {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl Corner {
    const ALL: [Corner; 4] = [
        Corner::BottomLeft,
        Corner::BottomRight,
        Corner::TopLeft,
        Corner::TopRight,
    ];

    /// Position de la poignée (points), `off` points hors des limites.
    fn point(self, b: &PointRect, off: f32) -> (f32, f32) {
        match self {
            Corner::BottomLeft => (b.left - off, b.bottom - off),
            Corner::BottomRight => (b.right + off, b.bottom - off),
            Corner::TopLeft => (b.left - off, b.top + off),
            Corner::TopRight => (b.right + off, b.top + off),
        }
    }

    fn cursor(self) -> CursorIcon {
        match self {
            Corner::BottomLeft | Corner::TopRight => CursorIcon::ResizeNeSw,
            Corner::BottomRight | Corner::TopLeft => CursorIcon::ResizeNwSe,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragMode {
    Move,
    Resize(Corner),
    /// Rotation libre autour de `c` (centre des limites).
    Rotate {
        c: (f32, f32),
    },
    /// Rectangle élastique de sélection.
    Marquee,
}

/// Objets sélectionnés sur une page (triés par index).
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub page: usize,
    pub objects: Vec<ObjectInfo>,
}

impl Selection {
    pub fn single(&self) -> Option<&ObjectInfo> {
        match self.objects.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }

    pub fn indices(&self) -> Vec<usize> {
        self.objects.iter().map(|o| o.index).collect()
    }

    pub fn union_bounds(&self) -> PointRect {
        union(self.objects.iter().map(|o| o.bounds))
    }
}

fn union(mut it: impl Iterator<Item = PointRect>) -> PointRect {
    let first = it.next().unwrap_or(PointRect {
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
        top: 0.0,
    });
    it.fold(first, |u, b| PointRect {
        left: u.left.min(b.left),
        bottom: u.bottom.min(b.bottom),
        right: u.right.max(b.right),
        top: u.top.max(b.top),
    })
}

/// Sélection après une réponse `ObjectsSelected` : remplacer, ajouter ou basculer.
pub fn merge_selection(
    mut current: Vec<ObjectInfo>,
    infos: Vec<ObjectInfo>,
    mode: PickMode,
) -> Vec<ObjectInfo> {
    if mode == PickMode::Replace {
        current.clear();
    }
    for info in infos {
        match current.iter().position(|o| o.index == info.index) {
            Some(pos) if mode == PickMode::Toggle => {
                current.remove(pos);
            }
            Some(pos) => current[pos] = info,
            None => current.push(info),
        }
    }
    current.sort_by_key(|o| o.index);
    current
}

/// Objets copiés dans le presse-papiers du moteur.
pub struct ClipInfo {
    /// Texte placé dans le presse-papiers système : Ctrl+V ne colle les objets que s'il y est encore.
    pub marker: String,
    /// Document et page d'origine (décalage des collages successifs au même endroit).
    pub source: Option<(PathBuf, usize)>,
    pub pastes: u32,
}

/// Copie d'objets en vol : « couper » (le thread moteur supprime lui-même après la copie),
/// (chemin, page) d'origine.
pub type PendingCopy = (bool, Option<(PathBuf, usize)>);

pub struct EditState {
    pub selection: Option<Selection>,
    /// Après une modification, resélectionner cet objet (page, index ; `usize::MAX` = dernier).
    pub reselect: Option<(usize, usize)>,
    /// Après une modification, resélectionner ces objets.
    pub reselect_many: Option<(usize, Pick)>,
    pub text_prompt: Option<TextPrompt>,
    /// Dernier texte envoyé : rouvert si la police ne contient pas les caractères saisis.
    pub last_prompt: Option<TextPrompt>,
    pub image_width_pt: f32,
    pub drag_mode: Option<DragMode>,
    /// Modificateurs de l'image courante.
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub stroke_rgb: [u8; 3],
    pub fill_rgb: [u8; 3],
    pub fill: bool,
    pub stroke_width: f32,
    /// Opacité (%) : celle de la sélection, ou celle des prochaines formes et images.
    pub opacity_pct: f32,
    pub opacity_dirty: bool,
    /// Champ « Angle » de la barre d'objet.
    pub angle_field: f32,
    /// Presse-papiers d'objets : global (un onglet copie, un autre colle).
    pub clip: Option<ClipInfo>,
    /// Copie en vol : (objets à supprimer si c'est un « couper », page et chemin d'origine).
    pub pending_copy: Option<PendingCopy>,
    /// Message affiché après la prochaine modification réussie.
    pub pending_status: Option<String>,
    /// Ce message survit à la resélection qui suit l'opération.
    pub hold_status: bool,
    /// Panneau Objets ouvert.
    pub objects_open: bool,
    /// Objets de `objects_page` (panneau, prise directe au glisser).
    pub objects: Vec<ObjectInfo>,
    pub objects_page: Option<usize>,
    pub hover_index: Option<usize>,
    pub system_fonts: Option<Arc<Vec<SystemFont>>>,
    pub fonts_requested: bool,
    pub font_filter: String,
    /// Le prochain clic de l'outil Texte valide l'éditeur au lieu d'en ouvrir un autre.
    pub swallow_click: bool,
    /// Page affichée à l'image précédente : (index, rectangle écran, hauteur en points).
    pub page_view: Option<(usize, egui::Rect, f32)>,
}

impl Default for EditState {
    fn default() -> Self {
        let shape = ShapeStyle::default();
        Self {
            selection: None,
            reselect: None,
            reselect_many: None,
            text_prompt: None,
            last_prompt: None,
            image_width_pt: 200.0,
            drag_mode: None,
            shift: false,
            alt: false,
            ctrl: false,
            stroke_rgb: rgb_arr(shape.stroke_rgb),
            fill_rgb: [255, 255, 255],
            fill: false,
            stroke_width: shape.stroke_width,
            opacity_pct: 100.0,
            opacity_dirty: false,
            angle_field: 0.0,
            clip: None,
            pending_copy: None,
            pending_status: None,
            hold_status: false,
            objects_open: false,
            objects: Vec::new(),
            objects_page: None,
            hover_index: None,
            system_fonts: None,
            fonts_requested: false,
            font_filter: String::new(),
            swallow_click: false,
            page_view: None,
        }
    }
}

impl EditState {
    /// Changement d'onglet ou fermeture : tout ce qui désigne un objet du document actif part ;
    /// le presse-papiers, les polices, les réglages et l'état du panneau restent.
    pub fn reset_for_tab(&mut self) {
        self.selection = None;
        self.reselect = None;
        self.reselect_many = None;
        self.drag_mode = None;
        self.text_prompt = None;
        self.last_prompt = None;
        self.pending_copy = None;
        self.pending_status = None;
        self.hold_status = false;
        self.objects.clear();
        self.objects_page = None;
        self.hover_index = None;
        self.swallow_click = false;
    }
}

pub fn rgb_arr(c: (u8, u8, u8)) -> [u8; 3] {
    [c.0, c.1, c.2]
}

pub fn rgb_tuple(c: [u8; 3]) -> (u8, u8, u8) {
    (c[0], c[1], c[2])
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

pub fn kind_key(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Text => "object.kind.text",
        ObjectKind::Image => "object.kind.image",
        ObjectKind::Path => "object.kind.path",
        ObjectKind::Form => "object.kind.form",
        ObjectKind::Other => "object.kind.other",
    }
}

pub fn family_key(f: FontFamily) -> &'static str {
    match f {
        FontFamily::Helvetica => "font.helvetica",
        FontFamily::Times => "font.times",
        FontFamily::Courier => "font.courier",
    }
}

fn contains(b: &PointRect, x: f32, y: f32) -> bool {
    x >= b.left - 2.0 && x <= b.right + 2.0 && y >= b.bottom - 2.0 && y <= b.top + 2.0
}

fn normalized(mut b: PointRect) -> PointRect {
    if b.left > b.right {
        std::mem::swap(&mut b.left, &mut b.right);
    }
    if b.bottom > b.top {
        std::mem::swap(&mut b.bottom, &mut b.top);
    }
    b.right = b.right.max(b.left + 2.0);
    b.top = b.top.max(b.bottom + 2.0);
    b
}

fn center(b: &PointRect) -> (f32, f32) {
    ((b.left + b.right) / 2.0, (b.bottom + b.top) / 2.0)
}

/// Angle horaire (degrés) du vecteur centre → point, mesuré depuis le haut (Y vers le haut).
pub fn pointer_angle(c: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - c.0).atan2(p.1 - c.1).to_degrees()
}

pub fn norm180(deg: f32) -> f32 {
    let d = deg.rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

/// Rotation horaire de `p` autour de `c` (Y vers le haut).
pub fn rotate_point(c: (f32, f32), p: (f32, f32), deg: f32) -> (f32, f32) {
    let (s, co) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    (c.0 + dx * co + dy * s, c.1 - dx * s + dy * co)
}

/// Écart pour arriver au multiple de 15° le plus proche de `base + delta`.
pub fn snap_15(base: f32, delta: f32) -> f32 {
    ((base + delta) / 15.0).round() * 15.0 - base
}

/// Cadre orienté si l'objet est un texte ou une image (sommets triés autour du centre), sinon
/// les quatre coins de la boîte englobante.
fn frame(info: &ObjectInfo) -> [(f32, f32); 4] {
    if matches!(info.kind, ObjectKind::Text | ObjectKind::Image) {
        let c = center(&info.bounds);
        let mut q = info.quad;
        q.sort_by(|a, b| {
            (a.1 - c.1)
                .atan2(a.0 - c.0)
                .total_cmp(&(b.1 - c.1).atan2(b.0 - c.0))
        });
        q
    } else {
        let b = info.bounds;
        [
            (b.left, b.bottom),
            (b.right, b.bottom),
            (b.right, b.top),
            (b.left, b.top),
        ]
    }
}

/// Angle d'affichage du cadre (0 pour les objets dessinés par leur boîte englobante).
fn frame_angle(info: &ObjectInfo) -> f32 {
    if matches!(info.kind, ObjectKind::Text | ObjectKind::Image) {
        info.angle_deg
    } else {
        0.0
    }
}

/// Redimensionnable par les coins : non pivoté, ou pivoté d'un multiple de 90°.
pub fn resizable(info: &ObjectInfo) -> bool {
    let q = info.angle_deg / 90.0;
    (q - q.round()).abs() * 90.0 < 0.5
}

/// Poignée de rotation (points) : au-dessus du cadre, dans l'axe de l'objet.
pub fn rotate_handle(info: &ObjectInfo, k: f32) -> (f32, f32) {
    let c = center(&info.bounds);
    let a = frame_angle(info).to_radians();
    let up = (a.sin(), a.cos());
    let half = frame(info)
        .iter()
        .map(|p| (p.0 - c.0) * up.0 + (p.1 - c.1) * up.1)
        .fold(0.0, f32::max);
    let r = half + (HANDLE_OFFSET_PX + ROTATE_GAP_PX) / k;
    (c.0 + up.0 * r, c.1 + up.1 * r)
}

/// Objet le plus en avant sous le point, d'après la liste en cache (prise directe au glisser).
pub fn local_hit(objects: &[ObjectInfo], x: f32, y: f32) -> Option<&ObjectInfo> {
    objects.iter().rev().find(|o| {
        let b = o.bounds;
        let in_box =
            x >= b.left - 1.0 && x <= b.right + 1.0 && y >= b.bottom - 1.0 && y <= b.top + 1.0;
        in_box && (resizable(o) || in_polygon(&frame(o), x, y))
    })
}

fn in_polygon(q: &[(f32, f32); 4], x: f32, y: f32) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

impl Tool {
    pub fn is_edit(self) -> bool {
        matches!(
            self,
            Tool::SelectObject
                | Tool::AddText
                | Tool::AddImage
                | Tool::Rectangle
                | Tool::Ellipse
                | Tool::Line
        )
    }
}

impl SuniApp {
    /// L'objet sélectionné, s'il est seul et sur `page`.
    pub fn selected_on(&self, page: usize) -> Option<&ObjectInfo> {
        self.edit
            .selection
            .as_ref()
            .filter(|s| s.page == page)
            .and_then(Selection::single)
    }

    fn selection_on(&self, page: usize) -> Option<&Selection> {
        self.edit.selection.as_ref().filter(|s| s.page == page)
    }

    fn zoom_k(&self) -> f32 {
        self.effective_zoom * SCREEN_PER_PDF_POINT
    }

    /// Rectangle écran de la sélection (page affichée), poignées comprises.
    fn selection_screen_rect(&self) -> Option<egui::Rect> {
        let (page, page_rect, h) = self.edit.page_view?;
        let b = self.selection_on(page)?.union_bounds();
        let k = self.zoom_k();
        let pt = |x: f32, y: f32| page_rect.min + egui::vec2(x * k, (h - y) * k);
        Some(egui::Rect::from_two_pos(pt(b.left, b.top), pt(b.right, b.bottom)).expand(6.0))
    }

    fn corner_at(&self, page: usize, pt: (f32, f32)) -> Option<Corner> {
        let info = self.selected_on(page).filter(|i| resizable(i))?;
        let k = self.zoom_k();
        let (tol, off) = (HANDLE_PX / k, HANDLE_OFFSET_PX / k);
        Corner::ALL.into_iter().find(|c| {
            let (cx, cy) = c.point(&info.bounds, off);
            (cx - pt.0).abs() <= tol && (cy - pt.1).abs() <= tol
        })
    }

    fn on_rotate_handle(&self, page: usize, pt: (f32, f32)) -> bool {
        let k = self.zoom_k();
        self.selected_on(page).is_some_and(|info| {
            let h = rotate_handle(info, k);
            ((h.0 - pt.0).powi(2) + (h.1 - pt.1).powi(2)).sqrt() <= HANDLE_PX / k
        })
    }

    fn in_selection(&self, page: usize, pt: (f32, f32)) -> bool {
        self.selection_on(page)
            .is_some_and(|s| s.objects.iter().any(|o| contains(&o.bounds, pt.0, pt.1)))
    }

    /// Curseur à afficher au-dessus de la page selon l'outil et la position (points).
    pub fn page_cursor(&self, page: usize, pt: Option<(f32, f32)>) -> CursorIcon {
        if self.sign.placing.is_some() {
            return CursorIcon::Crosshair;
        }
        match self.annotate.tool {
            Tool::Note
            | Tool::AddText
            | Tool::AddImage
            | Tool::Rectangle
            | Tool::Ellipse
            | Tool::Line => CursorIcon::Crosshair,
            Tool::SelectObject => match (self.edit.drag_mode, pt) {
                (Some(DragMode::Resize(c)), _) => c.cursor(),
                (Some(DragMode::Move | DragMode::Rotate { .. }), _) => CursorIcon::Grabbing,
                (Some(DragMode::Marquee), _) => CursorIcon::Crosshair,
                (None, Some(pt)) => {
                    if self.on_rotate_handle(page, pt) {
                        CursorIcon::Grab
                    } else if let Some(c) = self.corner_at(page, pt) {
                        c.cursor()
                    } else if self.in_selection(page, pt) {
                        CursorIcon::Move
                    } else if self
                        .objects_here(page)
                        .is_some_and(|o| local_hit(o, pt.0, pt.1).is_some())
                    {
                        CursorIcon::PointingHand
                    } else {
                        CursorIcon::Default
                    }
                }
                _ => CursorIcon::Default,
            },
            // Les rectangles de champs posent eux-mêmes la main.
            Tool::FillForm => CursorIcon::Default,
            _ => CursorIcon::Text,
        }
    }

    fn objects_here(&self, page: usize) -> Option<&[ObjectInfo]> {
        (self.edit.objects_page == Some(page)).then_some(self.edit.objects.as_slice())
    }

    /// Début d'un glissement : poignée de rotation, coin, sélection (déplacer), objet non
    /// sélectionné (le prendre et le déplacer), sinon rectangle élastique.
    pub fn edit_drag_start(&mut self, page: usize, pt: (f32, f32)) {
        self.edit.drag_mode = if self.annotate.tool != Tool::SelectObject {
            None
        } else if self.on_rotate_handle(page, pt) {
            let c = self.selected_on(page).map(|i| center(&i.bounds));
            c.map(|c| DragMode::Rotate { c })
        } else if let Some(c) = self.corner_at(page, pt) {
            Some(DragMode::Resize(c))
        } else if self.in_selection(page, pt) {
            Some(DragMode::Move)
        } else if let Some(hit) = self
            .objects_here(page)
            .and_then(|o| local_hit(o, pt.0, pt.1))
            .cloned()
        {
            let current = match (&self.edit.selection, self.edit.shift) {
                (Some(s), true) if s.page == page => s.objects.clone(),
                _ => Vec::new(),
            };
            self.set_selection(page, merge_selection(current, vec![hit], PickMode::Add));
            Some(DragMode::Move)
        } else {
            Some(DragMode::Marquee)
        };
    }

    /// Pendant le glissement : angle affiché dans la barre d'état.
    pub fn edit_drag_moved(&mut self, drag: &Drag) {
        if let (Some(DragMode::Rotate { c }), Some(info)) =
            (self.edit.drag_mode, self.selected_on(drag.page))
        {
            let angle = norm180(info.angle_deg + self.rotate_delta(c, info.angle_deg, drag));
            self.status_msg = Some(tr_args(
                self.lang(),
                "status.rotating",
                &[("angle", &format!("{angle:.0}"))],
            ));
        }
    }

    fn rotate_delta(&self, c: (f32, f32), base: f32, drag: &Drag) -> f32 {
        let delta = norm180(pointer_angle(c, drag.current) - pointer_angle(c, drag.start));
        if self.edit.shift {
            snap_15(base, delta)
        } else {
            delta
        }
    }

    /// Limites de l'objet sélectionné pendant un redimensionnement (aperçu).
    fn preview_bounds(&self, drag: &Drag) -> Option<PointRect> {
        let info = self.selected_on(drag.page)?;
        let (dx, dy) = (drag.current.0 - drag.start.0, drag.current.1 - drag.start.1);
        let mut b = info.bounds;
        let DragMode::Resize(c) = self.edit.drag_mode? else {
            return None;
        };
        match c {
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
        // Images : proportions conservées par défaut (Maj pour les libérer) ; inverse sinon.
        let (w, h) = (
            info.bounds.right - info.bounds.left,
            info.bounds.top - info.bounds.bottom,
        );
        let lock = (info.kind == ObjectKind::Image) != self.edit.shift;
        if lock && w > 0.0 && h > 0.0 {
            let nh = (b.right - b.left).abs() * h / w;
            match c {
                Corner::BottomLeft | Corner::BottomRight => b.bottom = b.top - nh,
                Corner::TopLeft | Corner::TopRight => b.top = b.bottom + nh,
            }
        }
        Some(normalized(b))
    }

    /// Clic simple avec l'outil de sélection d'objet (Maj ou Ctrl : ajouter ou retirer).
    pub fn edit_click(&mut self, page: usize, pt: (f32, f32)) {
        if self.edit.shift || self.edit.ctrl {
            self.worker.send(Request::Pick {
                page,
                pick: Pick::At { x: pt.0, y: pt.1 },
                mode: PickMode::Toggle,
            });
            return;
        }
        if self
            .selected_on(page)
            .is_some_and(|i| contains(&i.bounds, pt.0, pt.1))
        {
            return; // déjà sélectionné
        }
        self.worker.send(Request::ObjectAt {
            page,
            x: pt.0,
            y: pt.1,
        });
    }

    /// Double-clic : modifier un objet texte sous le pointeur.
    pub fn edit_double_click(&mut self, page: usize, pt: (f32, f32)) {
        if self
            .selected_on(page)
            .is_some_and(|i| contains(&i.bounds, pt.0, pt.1))
        {
            self.open_text_prompt_for_selection();
        } else {
            self.edit_click(page, pt);
        }
    }

    /// Modifier le texte sélectionné : sur la page s'il est droit à l'écran, sinon en dialogue.
    pub fn open_text_prompt_for_selection(&mut self) {
        let Some(sel) = &self.edit.selection else {
            return;
        };
        let Some(info) = sel.single().filter(|i| i.kind == ObjectKind::Text) else {
            return;
        };
        self.edit.text_prompt = Some(TextPrompt {
            page: sel.page,
            // Texte existant : (x, y) = coin haut-gauche de ses limites (ancre de l'éditeur).
            x: info.bounds.left,
            y: info.bounds.top,
            index: Some(info.index),
            text: info.text.clone().unwrap_or_default(),
            size: if info.font_size > 0.0 {
                info.font_size
            } else {
                12.0
            },
            bold: false,
            italic: false,
            font: TextFont::Standard(FontFamily::Helvetica),
            rgb: info.fill_rgb.map(rgb_arr).unwrap_or([0, 0, 0]),
            inline: info.angle_deg.abs() < 0.5,
            armed: false,
            focus: true,
        });
    }

    /// Fin d'un glissement avec un outil d'édition.
    pub fn edit_drag_finished(&mut self, drag: Drag) {
        let rect = drag.rect();
        let tiny = (rect.right - rect.left) < 2.0 && (rect.top - rect.bottom) < 2.0;
        match self.annotate.tool {
            Tool::SelectObject => {
                let bounds = self.preview_bounds(&drag);
                let mode = self.edit.drag_mode.take();
                let selected = self
                    .edit
                    .selection
                    .as_ref()
                    .filter(|s| s.page == drag.page)
                    .map(|s| (s.page, s.indices()));
                match (mode, selected) {
                    (Some(DragMode::Move), Some((page, indices))) if !tiny => {
                        self.edit.reselect_many = Some((page, Pick::Indices(indices.clone())));
                        self.worker.send(Request::Edit(EditOp::MoveObjects {
                            page,
                            indices,
                            dx: drag.current.0 - drag.start.0,
                            dy: drag.current.1 - drag.start.1,
                        }));
                    }
                    (Some(DragMode::Resize(_)), Some((page, indices))) if !tiny => {
                        if let (Some(bounds), [index]) = (bounds, indices.as_slice()) {
                            self.edit.reselect = Some((page, *index));
                            self.worker.send(Request::Edit(EditOp::ResizeObject {
                                page,
                                index: *index,
                                bounds,
                            }));
                        }
                    }
                    (Some(DragMode::Rotate { c }), Some((page, indices))) => {
                        let base = self.selected_on(page).map_or(0.0, |i| i.angle_deg);
                        let delta = self.rotate_delta(c, base, &drag);
                        self.status_msg = None;
                        if let ([index], true) = (indices.as_slice(), delta.abs() > 0.25) {
                            self.edit.reselect = Some((page, *index));
                            self.worker.send(Request::Edit(EditOp::RotateObject {
                                page,
                                index: *index,
                                degrees: delta,
                            }));
                        }
                    }
                    (Some(DragMode::Marquee), _) if !tiny => {
                        self.worker.send(Request::Pick {
                            page: drag.page,
                            pick: Pick::InRect {
                                rect,
                                touching: self.edit.alt,
                            },
                            mode: if self.edit.shift || self.edit.ctrl {
                                PickMode::Add
                            } else {
                                PickMode::Replace
                            },
                        });
                    }
                    _ => self.edit_click(drag.page, drag.start),
                }
            }
            Tool::AddText => {
                if std::mem::take(&mut self.edit.swallow_click) {
                    return;
                }
                let font = match &self.edit.last_prompt {
                    Some(p) if p.index.is_none() => p.font.clone(),
                    _ => TextFont::Standard(FontFamily::Helvetica),
                };
                self.edit.text_prompt = Some(TextPrompt {
                    page: drag.page,
                    x: drag.start.0,
                    y: drag.start.1,
                    index: None,
                    text: String::new(),
                    size: 14.0,
                    bold: false,
                    italic: false,
                    font,
                    rgb: [0, 0, 0],
                    inline: true,
                    armed: false,
                    focus: true,
                });
            }
            Tool::AddImage => {
                let dialog = rfd::FileDialog::new()
                    .set_title(self.t("tools.images.pick_one"))
                    .add_filter(self.t("tools.images.filter"), IMAGE_EXTENSIONS);
                if let Some(path) = dialog.pick_file() {
                    let width_pt = self.edit.image_width_pt;
                    // Le point cliqué devient le coin haut-gauche de l'image.
                    let height_pt = match image::image_dimensions(&path) {
                        Ok((w, h)) if w > 0 => width_pt * h as f32 / w as f32,
                        _ => width_pt * 0.75,
                    };
                    self.edit.reselect_many = Some((drag.page, Pick::Last(1)));
                    self.worker.send(Request::Edit(EditOp::AddImage {
                        page: drag.page,
                        path,
                        x: drag.start.0,
                        y: drag.start.1 - height_pt,
                        width_pt,
                        opacity: self.edit.opacity_pct / 100.0,
                    }));
                }
            }
            Tool::Rectangle | Tool::Ellipse | Tool::Line => {
                if tiny {
                    return;
                }
                let kind = match self.annotate.tool {
                    Tool::Rectangle => ShapeKind::Rectangle,
                    Tool::Ellipse => ShapeKind::Ellipse,
                    _ => ShapeKind::Line,
                };
                let rect = if kind == ShapeKind::Line {
                    PointRect {
                        left: drag.start.0,
                        bottom: drag.start.1,
                        right: drag.current.0,
                        top: drag.current.1,
                    }
                } else {
                    rect
                };
                self.worker.send(Request::Edit(EditOp::AddShape {
                    page: drag.page,
                    kind,
                    rect,
                    style: self.shape_style(),
                }));
            }
            _ => {}
        }
    }

    fn shape_style(&self) -> ShapeStyle {
        ShapeStyle {
            stroke_rgb: rgb_tuple(self.edit.stroke_rgb),
            stroke_width: self.edit.stroke_width,
            fill_rgb: self.edit.fill.then_some(rgb_tuple(self.edit.fill_rgb)),
            opacity: self.edit.opacity_pct / 100.0,
        }
    }

    /// Réponse `ObjectSelected` (un objet ou aucun).
    pub fn on_object_selected(&mut self, page: usize, info: Option<ObjectInfo>) {
        self.set_selection(page, info.into_iter().collect());
    }

    pub fn on_objects_selected(&mut self, page: usize, infos: Vec<ObjectInfo>, mode: PickMode) {
        let current = match &self.edit.selection {
            Some(s) if s.page == page => s.objects.clone(),
            _ => Vec::new(),
        };
        self.set_selection(page, merge_selection(current, infos, mode));
    }

    fn set_selection(&mut self, page: usize, objects: Vec<ObjectInfo>) {
        let lang = self.lang();
        if objects.is_empty() {
            self.edit.selection = None;
            return;
        }
        let first = objects[0].clone();
        if !std::mem::take(&mut self.edit.hold_status) {
            self.status_msg = Some(if objects.len() == 1 {
                tr_args(
                    lang,
                    "status.object_selected",
                    &[("kind", tr(lang, kind_key(first.kind)))],
                )
            } else {
                tr_args(
                    lang,
                    "status.objects_selected",
                    &[("count", &objects.len().to_string())],
                )
            });
        }
        // La barre d'objet reflète le premier objet sélectionné.
        if let Some(c) = first.stroke_rgb {
            self.edit.stroke_rgb = rgb_arr(c);
        }
        if first.kind != ObjectKind::Text {
            if let Some(c) = first.fill_rgb {
                self.edit.fill_rgb = rgb_arr(c);
            }
            if first.stroke_width > 0.0 {
                self.edit.stroke_width = first.stroke_width;
            }
        } else if let Some(c) = first.fill_rgb {
            self.edit.stroke_rgb = rgb_arr(c);
        }
        if let Some(o) = first.opacity {
            self.edit.opacity_pct = (o * 100.0).round();
        }
        self.edit.opacity_dirty = false;
        self.edit.angle_field = first.angle_deg.round();
        self.edit.selection = Some(Selection { page, objects });
    }

    fn selection_ref(&self) -> Option<(usize, Vec<usize>)> {
        self.edit.selection.as_ref().map(|s| (s.page, s.indices()))
    }

    pub fn nudge_selected(&mut self, dx: f32, dy: f32) {
        if let Some((page, indices)) = self.selection_ref() {
            self.edit.reselect_many = Some((page, Pick::Indices(indices.clone())));
            self.worker.send(Request::Edit(EditOp::NudgeObjects {
                page,
                indices,
                dx,
                dy,
            }));
        }
    }

    pub fn rotate_selected(&mut self, degrees: f32) {
        if let Some((page, info)) = self
            .edit
            .selection
            .as_ref()
            .and_then(|s| s.single().map(|i| (s.page, i.index)))
        {
            self.edit.reselect = Some((page, info));
            self.worker.send(Request::Edit(EditOp::RotateObject {
                page,
                index: info,
                degrees,
            }));
        }
    }

    pub fn duplicate_selected(&mut self) {
        if let Some((page, indices)) = self.selection_ref() {
            self.edit.reselect_many = Some((page, Pick::Last(1)));
            self.worker
                .send(Request::Edit(EditOp::DuplicateObjects { page, indices }));
        }
    }

    pub fn reorder_selected(&mut self, op: ZOrder) {
        if let Some((page, indices)) = self.selection_ref() {
            let n = indices.len();
            let pick = match op {
                ZOrder::Front => Pick::Last(n),
                ZOrder::Back => Pick::Indices((0..n).collect()),
                ZOrder::Forward => Pick::Indices(indices.iter().map(|i| i + 1).collect()),
                ZOrder::Backward => {
                    Pick::Indices(indices.iter().map(|i| i.saturating_sub(1)).collect())
                }
            };
            self.edit.reselect_many = Some((page, pick));
            self.worker
                .send(Request::Edit(EditOp::ReorderObjects { page, indices, op }));
        }
    }

    pub fn delete_selected_object(&mut self) {
        if let Some((page, indices)) = self.selection_ref() {
            self.edit.selection = None;
            self.worker
                .send(Request::Edit(EditOp::DeleteObjects { page, indices }));
        }
    }

    /// Copier (ou couper) la sélection dans le presse-papiers d'objets.
    pub fn copy_objects(&mut self, cut: bool) {
        let Some((page, indices)) = self.selection_ref() else {
            return;
        };
        let source = self.document.as_ref().map(|d| (d.info.path.clone(), page));
        // Couper : le thread moteur supprime les objets juste après la copie (atomique : aucune
        // édition ne peut s'intercaler et décaler les index ; une copie échouée ne supprime
        // rien) ; Annuler ne défait que la suppression. La sélection part tout de suite pour que
        // Suppr ou les flèches ne visent pas les mêmes index entre-temps.
        if cut {
            self.edit.selection = None;
        }
        self.edit.pending_copy = Some((cut, source));
        self.worker
            .send(Request::CopyObjects { page, indices, cut });
    }

    /// Réponse `Copied` : repère texte dans le presse-papiers système (Ctrl+V le reconnaît) ;
    /// pour un « couper », le `DocumentChanged` de la suppression suit.
    pub fn on_copied(&mut self, ctx: &egui::Context, count: usize, text: String) {
        let lang = self.lang();
        let (cut, source) = self.edit.pending_copy.take().unwrap_or((false, None));
        let marker = if text.trim().is_empty() {
            tr_args(lang, "clipboard.marker", &[("count", &count.to_string())])
        } else {
            text
        };
        ctx.copy_text(marker.clone());
        self.edit.clip = Some(ClipInfo {
            marker,
            source,
            pastes: 0,
        });
        let msg = tr_args(
            lang,
            if cut {
                "status.objects_cut"
            } else {
                "status.objects_copied"
            },
            &[("count", &count.to_string())],
        );
        if cut {
            self.edit.pending_status = Some(msg);
        } else {
            self.status_msg = Some(msg);
        }
    }

    /// Coller sur la page courante : décalé de 12 pt par collage au même endroit, sinon à la
    /// position d'origine.
    pub fn paste_objects(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.document else { return };
        let (page, path) = (doc.page, doc.info.path.clone());
        let lang = self.lang();
        let Some(clip) = &mut self.edit.clip else {
            self.status_msg = Some(tr(lang, "status.paste_empty").to_string());
            return;
        };
        let (dx, dy) = if clip.source.as_ref() == Some(&(path, page)) {
            clip.pastes += 1;
            let d = 12.0 * clip.pastes as f32;
            (d, -d)
        } else {
            (0.0, 0.0)
        };
        if self.annotate.tool != Tool::SelectObject {
            self.execute(ctx, Command::ToolSelectObject);
        }
        self.edit.reselect_many = Some((page, Pick::Last(1)));
        self.edit.pending_status = Some(tr(lang, "status.objects_pasted").to_string());
        self.worker
            .send(Request::Edit(EditOp::PasteObjects { page, dx, dy }));
    }

    pub fn select_all_objects(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.document else { return };
        let page = doc.page;
        if self.annotate.tool != Tool::SelectObject {
            self.execute(ctx, Command::ToolSelectObject);
        }
        self.worker.send(Request::Pick {
            page,
            pick: Pick::All,
            mode: PickMode::Replace,
        });
    }

    pub fn replace_selected_image(&mut self) {
        let Some((page, index)) = self.edit.selection.as_ref().and_then(|s| {
            s.single()
                .filter(|i| i.kind == ObjectKind::Image)
                .map(|i| (s.page, i.index))
        }) else {
            return;
        };
        let dialog = rfd::FileDialog::new()
            .set_title(self.t("menu.edit.replace_image"))
            .add_filter(self.t("tools.images.filter"), IMAGE_EXTENSIONS);
        if let Some(path) = dialog.pick_file() {
            self.edit.reselect = Some((page, index));
            self.edit.pending_status = Some(self.t("status.image_replaced").to_string());
            self.worker
                .send(Request::Edit(EditOp::ReplaceImage { page, index, path }));
        }
    }

    /// Liste des objets de `page` (page courante, ou page survolée en défilement continu), tenue à
    /// jour pour le panneau et la prise directe.
    pub fn refresh_objects_if_needed(&mut self, page: usize) {
        if !(self.edit.objects_open || self.annotate.tool == Tool::SelectObject)
            || self.document.is_none()
        {
            return;
        }
        if self.edit.objects_page != Some(page) {
            self.edit.objects_page = Some(page);
            self.edit.objects.clear();
            self.edit.hover_index = None;
            self.worker.send(Request::Objects { page });
        }
    }

    /// Cadres de sélection (avec aperçus de déplacement, redimensionnement, rotation), poignées,
    /// rectangle élastique, aperçu des formes.
    pub fn paint_object_selection(
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
        let drag = self.annotate.drag.filter(|d| d.page == page_index);
        let accent = theme::BLUE;
        // Aperçu des formes en cours de tracé.
        if let Some(d) = drag {
            let stroke = Stroke::new(
                (self.edit.stroke_width * k).max(1.0),
                color32(self.edit.stroke_rgb).gamma_multiply(self.edit.opacity_pct / 100.0),
            );
            let fill = if self.edit.fill {
                color32(self.edit.fill_rgb).gamma_multiply(self.edit.opacity_pct / 100.0)
            } else {
                egui::Color32::TRANSPARENT
            };
            let r = to_screen(&d.rect());
            match self.annotate.tool {
                Tool::Rectangle => {
                    painter.rect(r, 0.0, fill, stroke, egui::StrokeKind::Middle);
                }
                Tool::Ellipse => {
                    painter.add(egui::epaint::EllipseShape {
                        center: r.center(),
                        radius: r.size() / 2.0,
                        angle: 0.0,
                        fill,
                        stroke,
                    });
                }
                Tool::Line => {
                    painter.line_segment([pt(d.start), pt(d.current)], stroke);
                }
                Tool::SelectObject if self.edit.drag_mode == Some(DragMode::Marquee) => {
                    painter.rect(
                        r,
                        2.0,
                        accent.gamma_multiply(0.08),
                        Stroke::new(1.0, accent),
                        egui::StrokeKind::Middle,
                    );
                }
                _ => {}
            }
        }
        // Survol d'une ligne du panneau Objets.
        if let (Some(i), Some(objects)) = (self.edit.hover_index, self.objects_here(page_index)) {
            if let Some(o) = objects.iter().find(|o| o.index == i) {
                let f = frame(o).map(pt);
                painter.add(egui::Shape::closed_line(
                    f.to_vec(),
                    Stroke::new(1.5, theme::CAT_PAGES),
                ));
            }
        }
        let Some(sel) = self.selection_on(page_index) else {
            return;
        };
        let mode = drag.and(self.edit.drag_mode);
        let (dx, dy) = match (mode, drag) {
            (Some(DragMode::Move), Some(d)) => (d.current.0 - d.start.0, d.current.1 - d.start.1),
            _ => (0.0, 0.0),
        };
        let single = sel.single();
        let outline = Stroke::new(1.5, accent);
        for o in &sel.objects {
            let mut f = frame(o).map(|p| (p.0 + dx, p.1 + dy));
            if let (Some(DragMode::Rotate { c }), Some(d)) = (mode, drag) {
                let delta = self.rotate_delta(c, o.angle_deg, &d);
                f = f.map(|p| rotate_point(c, p, delta));
            }
            if let (Some(DragMode::Resize(_)), Some(d)) = (mode, drag) {
                if let Some(b) = self.preview_bounds(&d) {
                    f = [
                        (b.left, b.bottom),
                        (b.right, b.bottom),
                        (b.right, b.top),
                        (b.left, b.top),
                    ];
                }
            }
            let screen: Vec<egui::Pos2> = f.iter().map(|p| pt(*p)).collect();
            if single.is_some() {
                // Cadre légèrement agrandi (3 px) pour ne pas masquer le bord de l'objet.
                let c = screen.iter().fold(egui::Vec2::ZERO, |a, p| a + p.to_vec2()) / 4.0;
                let grown: Vec<egui::Pos2> = screen
                    .iter()
                    .map(|p| {
                        let v = *p - c.to_pos2();
                        *p + v.normalized() * 3.0
                    })
                    .collect();
                painter.add(egui::Shape::closed_line(grown, outline));
            } else {
                painter.add(egui::Shape::closed_line(
                    screen,
                    Stroke::new(1.0, accent.gamma_multiply(0.8)),
                ));
            }
        }
        if single.is_none() {
            // Groupe : cadre englobant en pointillés.
            let mut u = sel.union_bounds();
            u.left += dx;
            u.right += dx;
            u.bottom += dy;
            u.top += dy;
            let r = to_screen(&u).expand(4.0);
            let corners = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            painter.extend(egui::Shape::dashed_line(&corners, outline, 6.0, 4.0));
            return;
        }
        let Some(info) = single else { return };
        if matches!(mode, Some(DragMode::Move | DragMode::Rotate { .. })) {
            return;
        }
        let handle = |p: egui::Pos2| {
            let r = egui::Rect::from_center_size(p, egui::vec2(8.0, 8.0));
            painter.rect(
                r,
                2.0,
                egui::Color32::WHITE,
                Stroke::new(1.5, accent),
                egui::StrokeKind::Middle,
            );
        };
        let bounds = drag
            .and_then(|d| self.preview_bounds(&d))
            .unwrap_or(info.bounds);
        if resizable(info) {
            let off = HANDLE_OFFSET_PX / k;
            for c in Corner::ALL {
                handle(pt(c.point(&bounds, off)));
            }
        }
        if mode.is_none() {
            // Poignée de rotation : tige depuis le haut du cadre, disque blanc cerclé.
            let h = pt(rotate_handle(info, k));
            let a = frame_angle(info).to_radians();
            let c = center(&info.bounds);
            let top = frame(info)
                .iter()
                .map(|p| (p.0 - c.0) * a.sin() + (p.1 - c.1) * a.cos())
                .fold(0.0, f32::max);
            let base = pt((c.0 + a.sin() * top, c.1 + a.cos() * top));
            painter.line_segment([base, h], Stroke::new(1.0, accent));
            painter.circle(h, 5.5, egui::Color32::WHITE, Stroke::new(1.5, accent));
        }
    }

    /// Barre flottante des propriétés d'objet (couleurs, épaisseur, opacité, angle, presse-papiers),
    /// posée sous la sélection (ou en haut à gauche de la zone document `bounds` pour un outil de
    /// création), sans déborder de `bounds` : en fenêtre étroite elle passe à la ligne.
    pub fn object_bar(&mut self, ctx: &egui::Context, bounds: egui::Rect) {
        // Sans sélection, la barre ne sert qu'aux outils de création (style des prochaines
        // formes et images) : elle ne masque pas la page pour rien.
        let creating = matches!(
            self.annotate.tool,
            Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::AddImage
        );
        if !self.annotate.tool.is_edit()
            || self.edit.text_prompt.as_ref().is_some_and(|p| p.inline)
            || (self.edit.selection.is_none() && !creating)
        {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        let mut apply = false;
        let mut angle_target = None;
        let mut action: Option<Command> = None;
        const GAP: f32 = 14.0;
        // Écarts de part et d'autre, marges intérieures (8 + 8) et trait du cadre (1 + 1).
        let max_w = (bounds.width() - 2.0 * GAP - 18.0).max(80.0);
        let id = egui::Id::new("object-bar");
        // Barre contextuelle : sous la sélection, ou au-dessus (place laissée à la poignée de
        // rotation) si elle ne tient pas dessous ; sans sélection, en haut à gauche.
        let bar_h = ctx.memory(|m| m.area_rect(id)).map_or(44.0, |r| r.height());
        let pos = match self.selection_screen_rect() {
            Some(r) if r.bottom() + 16.0 + bar_h <= bounds.bottom() - 8.0 => {
                egui::pos2(r.left(), r.bottom() + 16.0)
            }
            Some(r) => egui::pos2(r.left(), r.top() - bar_h - ROTATE_GAP_PX - 24.0),
            None => bounds.min + egui::vec2(GAP, GAP),
        };
        let pos = pos.clamp(
            bounds.min + egui::vec2(GAP, GAP),
            bounds.max - egui::vec2(GAP, GAP),
        );
        let sel = self.edit.selection.clone();
        let single = sel.as_ref().and_then(|s| s.single().cloned());
        let all_forms = sel.as_ref().is_some_and(|s| {
            s.objects
                .iter()
                .all(|o| matches!(o.kind, ObjectKind::Form | ObjectKind::Other))
        });
        let can_paste = self.edit.clip.is_some();
        egui::Area::new(id)
            .fixed_pos(pos)
            .constrain_to(bounds)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                egui::Frame::new()
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
                        // Fixe la largeur de coupure (l'Area ne connaît que sa taille précédente).
                        ui.set_max_width(max_w);
                        let icon = |ui: &mut egui::Ui, c: Command, enabled: bool| {
                            let (g, f) = crate::shell::command_icon(c);
                            let info = c.info();
                            let tip = match info.shortcut_key {
                                Some(key) => {
                                    format!("{}  ({})", tr(lang, info.label_key), tr(lang, key))
                                }
                                None => tr(lang, info.label_key).to_string(),
                            };
                            ui.add_enabled(
                                enabled,
                                egui::Button::new(crate::icons::icon(has_icons, g, f, 15.0))
                                    .frame(false),
                            )
                            .on_hover_text(&tip)
                            .on_disabled_hover_text(&tip)
                            .clicked()
                        };
                        // Rangée 1 : propriétés ; rangée 2 : actions. Chaque rangée passe à la
                        // ligne si la zone document est étroite.
                        ui.horizontal_wrapped(|ui| {
                            if let Some(s) = sel.as_ref().filter(|s| s.objects.len() > 1) {
                                ui.label(crate::icons::semibold(
                                    tr_args(
                                        lang,
                                        "bar.group",
                                        &[("count", &s.objects.len().to_string())],
                                    ),
                                    13.0,
                                ));
                                ui.separator();
                            }
                            ui.label(tr(lang, "bar.stroke"));
                            ui.color_edit_button_srgb(&mut self.edit.stroke_rgb);
                            ui.add(
                                egui::DragValue::new(&mut self.edit.stroke_width)
                                    .range(0.0..=50.0)
                                    .speed(0.1)
                                    .suffix(" pt"),
                            )
                            .on_hover_text(tr(lang, "bar.width"));
                            ui.separator();
                            ui.checkbox(&mut self.edit.fill, tr(lang, "bar.fill"));
                            ui.add_enabled_ui(self.edit.fill, |ui| {
                                ui.color_edit_button_srgb(&mut self.edit.fill_rgb);
                            });
                            ui.separator();
                            ui.label(tr(lang, "bar.opacity"));
                            let o = ui
                                .add_enabled(
                                    !all_forms,
                                    egui::DragValue::new(&mut self.edit.opacity_pct)
                                        .range(0.0..=100.0)
                                        .speed(1.0)
                                        .suffix(" %"),
                                )
                                .on_hover_text(tr(lang, "bar.opacity_hint"));
                            if o.changed() {
                                self.edit.opacity_dirty = true;
                            }
                            if let Some(info) = &single {
                                ui.separator();
                                ui.label(tr(lang, "bar.angle"));
                                let a = ui
                                    .add(
                                        egui::DragValue::new(&mut self.edit.angle_field)
                                            .range(-180.0..=180.0)
                                            .speed(1.0)
                                            .suffix("°"),
                                    )
                                    .on_hover_text(tr(lang, "bar.angle_hint"));
                                if (a.drag_stopped() || a.lost_focus())
                                    && (self.edit.angle_field - info.angle_deg).abs() > 0.05
                                {
                                    angle_target = Some(self.edit.angle_field);
                                }
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            if sel.is_some() {
                                if ui
                                    .button(tr(lang, "bar.apply"))
                                    .on_hover_text(tr_args(
                                        lang,
                                        "bar.apply_hint",
                                        &[(
                                            "kind",
                                            tr(
                                                lang,
                                                single.as_ref().map_or("object.kind.form", |i| {
                                                    kind_key(i.kind)
                                                }),
                                            ),
                                        )],
                                    ))
                                    .clicked()
                                {
                                    apply = true;
                                }
                                ui.separator();
                            }
                            for (c, enabled) in [
                                (Command::CopyObjects, sel.is_some()),
                                (Command::CutObjects, sel.is_some()),
                                (Command::PasteObjects, can_paste),
                            ] {
                                if icon(ui, c, enabled) {
                                    action = Some(c);
                                }
                            }
                            if single.as_ref().is_some_and(|i| i.kind == ObjectKind::Image)
                                && icon(ui, Command::ReplaceImage, true)
                            {
                                action = Some(Command::ReplaceImage);
                            }
                        });
                    });
            });
        if let (Some(target), Some(info)) = (angle_target, &single) {
            self.rotate_selected(norm180(target - info.angle_deg));
        }
        if apply {
            if let Some(sel) = &self.edit.selection {
                let only_text = sel.objects.iter().all(|o| o.kind == ObjectKind::Text);
                let opacity = self
                    .edit
                    .opacity_dirty
                    .then_some(self.edit.opacity_pct / 100.0);
                let style = if only_text {
                    ObjectStyle {
                        fill_rgb: Some(rgb_tuple(self.edit.stroke_rgb)),
                        opacity,
                        ..Default::default()
                    }
                } else {
                    ObjectStyle {
                        fill_rgb: self.edit.fill.then_some(rgb_tuple(self.edit.fill_rgb)),
                        stroke_rgb: Some(rgb_tuple(self.edit.stroke_rgb)),
                        stroke_width: Some(self.edit.stroke_width),
                        opacity,
                    }
                };
                let (page, indices) = (sel.page, sel.indices());
                self.edit.reselect_many = Some((page, Pick::Indices(indices.clone())));
                self.worker.send(Request::Edit(EditOp::SetObjectStyle {
                    page,
                    indices,
                    style,
                }));
            }
        }
        if let Some(c) = action {
            self.execute(ctx, c);
        }
    }

    /// Dialogue de texte (texte existant pivoté : l'édition sur la page n'est proposée que pour
    /// un texte droit à l'écran).
    pub fn text_prompt_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let muted = theme::palette(self.settings.dark_theme).text_muted;
        let Some(prompt) = self.edit.text_prompt.as_mut().filter(|p| !p.inline) else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let spec = crate::dialog::Spec::new(
            "text-object",
            tr(lang, "dialog.text.title_edit"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "dialog.text.subtitle_edit"))
        .icon(crate::icons::glyph::FONT, "T")
        .width(500.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                let field = ui.add(
                    egui::TextEdit::multiline(&mut prompt.text)
                        .hint_text(tr(lang, "dialog.text.placeholder"))
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                if prompt.focus {
                    field.request_focus();
                    prompt.focus = false;
                }
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "dialog.text.size"));
                    ui.add(
                        egui::DragValue::new(&mut prompt.size)
                            .range(4.0..=200.0)
                            .speed(0.5),
                    );
                    ui.label(tr(lang, "dialog.text.color"));
                    ui.color_edit_button_srgb(&mut prompt.rgb);
                });
                ui.label(
                    egui::RichText::new(tr(lang, "dialog.text.font_note"))
                        .size(12.0)
                        .color(muted),
                );
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "dialog.text.ok"))
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
            let prompt = self.edit.text_prompt.take().expect("prompt");
            self.commit_text_prompt(prompt);
        } else if cancel {
            self.edit.text_prompt = None;
        }
    }

    /// Envoie le texte saisi (ajout, ou modification de l'objet existant).
    pub fn commit_text_prompt(&mut self, prompt: TextPrompt) {
        let text = prompt.text.trim_end().to_string();
        if text.trim().is_empty() {
            return;
        }
        match prompt.index {
            Some(index) => {
                let current = self.selected_on(prompt.page).cloned();
                let same_text =
                    current.as_ref().and_then(|i| i.text.as_deref()) == Some(text.as_str());
                let same_size = current
                    .as_ref()
                    .is_some_and(|i| (i.font_size - prompt.size).abs() < 0.05);
                let same_rgb =
                    current.as_ref().and_then(|i| i.fill_rgb) == Some(rgb_tuple(prompt.rgb));
                if same_text && same_size && same_rgb {
                    return;
                }
                self.edit.reselect = Some((prompt.page, index));
                self.worker.send(Request::Edit(EditOp::SetText {
                    page: prompt.page,
                    index,
                    text: (!same_text).then_some(text),
                    size: (!same_size).then_some(prompt.size),
                    rgb: (!same_rgb).then_some(rgb_tuple(prompt.rgb)),
                }));
            }
            None => {
                let lines = text.lines().filter(|l| !l.trim().is_empty()).count();
                let (family, font_file) = match &prompt.font {
                    TextFont::Standard(f) => (*f, None),
                    TextFont::System(f) => (FontFamily::Helvetica, Some(f.path.clone())),
                };
                self.edit.reselect_many = Some((prompt.page, Pick::Last(lines.max(1))));
                self.worker.send(Request::Edit(EditOp::AddText {
                    page: prompt.page,
                    x: prompt.x,
                    y: prompt.y,
                    text,
                    style: TextStyle {
                        size_pt: prompt.size,
                        bold: prompt.bold,
                        italic: prompt.italic,
                        family,
                        rgb: rgb_tuple(prompt.rgb),
                        opacity: 1.0,
                        font_file,
                    },
                }));
                self.edit.last_prompt = Some(prompt);
            }
        }
    }
}

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"];

#[cfg(test)]
mod tests {
    use super::*;

    fn info(index: usize) -> ObjectInfo {
        ObjectInfo {
            index,
            kind: ObjectKind::Path,
            bounds: PointRect {
                left: index as f32,
                bottom: 0.0,
                right: index as f32 + 10.0,
                top: 10.0,
            },
            quad: [(0.0, 0.0); 4],
            angle_deg: 0.0,
            opacity: Some(1.0),
            text: None,
            font_size: 0.0,
            font_name: None,
            fill_rgb: None,
            stroke_rgb: None,
            stroke_width: 1.0,
        }
    }

    #[test]
    fn selection_merge_modes() {
        let ids = |v: Vec<ObjectInfo>| v.iter().map(|o| o.index).collect::<Vec<_>>();
        let cur = vec![info(1), info(3)];
        assert_eq!(
            ids(merge_selection(
                cur.clone(),
                vec![info(2)],
                PickMode::Replace
            )),
            vec![2]
        );
        assert_eq!(
            ids(merge_selection(cur.clone(), vec![info(2)], PickMode::Add)),
            vec![1, 2, 3]
        );
        assert_eq!(
            ids(merge_selection(cur.clone(), vec![info(3)], PickMode::Add)),
            vec![1, 3]
        );
        assert_eq!(
            ids(merge_selection(
                cur.clone(),
                vec![info(3), info(0)],
                PickMode::Toggle
            )),
            vec![0, 1]
        );
        let s = Selection {
            page: 0,
            objects: vec![info(1), info(3)],
        };
        assert!(s.single().is_none());
        assert_eq!(s.indices(), vec![1, 3]);
        let u = s.union_bounds();
        assert_eq!((u.left, u.right), (1.0, 13.0));
    }

    #[test]
    fn rotation_geometry() {
        // Angle horaire depuis le haut : à droite = 90°, en bas = 180°.
        assert_eq!(pointer_angle((0.0, 0.0), (0.0, 5.0)), 0.0);
        assert!((pointer_angle((0.0, 0.0), (5.0, 0.0)) - 90.0).abs() < 1e-4);
        assert!((pointer_angle((0.0, 0.0), (-5.0, 0.0)) + 90.0).abs() < 1e-4);
        let p = rotate_point((0.0, 0.0), (0.0, 10.0), 90.0);
        assert!((p.0 - 10.0).abs() < 1e-4 && p.1.abs() < 1e-4, "{p:?}");
        // 10° + 12° = 22° → 15° (écart 5°) ; 10° + 14° = 24° → 30° (écart 20°).
        assert_eq!(snap_15(10.0, 12.0), 5.0);
        assert_eq!(snap_15(10.0, 14.0), 20.0);
        assert_eq!(snap_15(0.0, 7.0), 0.0);
        assert_eq!(norm180(270.0), -90.0);
        let mut i = info(0);
        i.angle_deg = 90.0;
        assert!(resizable(&i));
        i.angle_deg = 30.0;
        assert!(!resizable(&i));
        // Poignée au-dessus d'un objet droit, au centre horizontal.
        let mut j = info(0);
        j.kind = ObjectKind::Text;
        j.quad = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let h = rotate_handle(&j, 1.0);
        assert!(
            (h.0 - 5.0).abs() < 1e-4 && h.1 > 10.0 + ROTATE_GAP_PX - 1.0,
            "{h:?}"
        );
    }

    #[test]
    fn local_hit_uses_oriented_frame() {
        let mut t = info(0);
        t.kind = ObjectKind::Text;
        t.angle_deg = 45.0;
        // Losange inscrit dans la boîte 0..10.
        t.quad = [(5.0, 0.0), (10.0, 5.0), (5.0, 10.0), (0.0, 5.0)];
        t.bounds = PointRect {
            left: 0.0,
            bottom: 0.0,
            right: 10.0,
            top: 10.0,
        };
        let objs = vec![t];
        assert!(local_hit(&objs, 5.0, 5.0).is_some());
        assert!(local_hit(&objs, 0.5, 9.5).is_none(), "coin hors du losange");
        assert!(local_hit(&objs, 20.0, 5.0).is_none());
    }
}
