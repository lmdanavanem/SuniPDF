//! Édition du contenu des pages : sélection d'objets, déplacement, redimensionnement, texte,
//! images, formes, couleurs.

use crate::annotate::{Drag, Tool};
use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{CursorIcon, Key, Stroke};
use sunipdf_core::{
    Command, FontFamily, ObjectInfo, ObjectKind, ObjectStyle, PointRect, ShapeKind, ShapeStyle,
    TextStyle,
};
use sunipdf_i18n::{tr, tr_args};

/// Tolérance de prise des poignées, en pixels écran.
const HANDLE_PX: f32 = 10.0;
/// Les poignées sont dessinées à cette distance (pixels) hors des limites de l'objet.
const HANDLE_OFFSET_PX: f32 = 3.0;

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
    pub family: FontFamily,
    pub rgb: [u8; 3],
    focus: bool,
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

    /// Position de la poignée (points PDF), `off` points hors des limites.
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
}

pub struct EditState {
    pub selected: Option<(usize, ObjectInfo)>,
    /// Après une modification, resélectionner cet objet (page, index).
    pub reselect: Option<(usize, usize)>,
    pub text_prompt: Option<TextPrompt>,
    pub image_width_pt: f32,
    pub drag_mode: Option<DragMode>,
    /// Touche Maj enfoncée (proportions libres pour les images, verrouillées pour le reste).
    pub shift: bool,
    pub stroke_rgb: [u8; 3],
    pub fill_rgb: [u8; 3],
    pub fill: bool,
    pub stroke_width: f32,
}

impl Default for EditState {
    fn default() -> Self {
        let shape = ShapeStyle::default();
        Self {
            selected: None,
            reselect: None,
            text_prompt: None,
            image_width_pt: 200.0,
            drag_mode: None,
            shift: false,
            stroke_rgb: rgb_arr(shape.stroke_rgb),
            fill_rgb: [255, 255, 255],
            fill: false,
            stroke_width: shape.stroke_width,
        }
    }
}

fn rgb_arr(c: (u8, u8, u8)) -> [u8; 3] {
    [c.0, c.1, c.2]
}

fn rgb_tuple(c: [u8; 3]) -> (u8, u8, u8) {
    (c[0], c[1], c[2])
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn kind_key(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Text => "object.kind.text",
        ObjectKind::Image => "object.kind.image",
        ObjectKind::Path => "object.kind.path",
        ObjectKind::Other => "object.kind.other",
    }
}

fn family_key(f: FontFamily) -> &'static str {
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
    fn selected_on(&self, page: usize) -> Option<&ObjectInfo> {
        match &self.edit.selected {
            Some((p, info)) if *p == page => Some(info),
            _ => None,
        }
    }

    fn corner_at(&self, page: usize, pt: (f32, f32)) -> Option<Corner> {
        let info = self.selected_on(page)?;
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let (tol, off) = (HANDLE_PX / k, HANDLE_OFFSET_PX / k);
        Corner::ALL.into_iter().find(|c| {
            let (cx, cy) = c.point(&info.bounds, off);
            (cx - pt.0).abs() <= tol && (cy - pt.1).abs() <= tol
        })
    }

    /// Curseur à afficher au-dessus de la page selon l'outil et la position (points PDF).
    pub fn page_cursor(&self, page: usize, pt: Option<(f32, f32)>) -> CursorIcon {
        match self.annotate.tool {
            Tool::Note
            | Tool::AddText
            | Tool::AddImage
            | Tool::Rectangle
            | Tool::Ellipse
            | Tool::Line => CursorIcon::Crosshair,
            Tool::SelectObject => match (self.edit.drag_mode, pt) {
                (Some(DragMode::Resize(c)), _) => c.cursor(),
                (Some(DragMode::Move), _) => CursorIcon::Grabbing,
                (None, Some(pt)) => {
                    if let Some(c) = self.corner_at(page, pt) {
                        c.cursor()
                    } else if self
                        .selected_on(page)
                        .is_some_and(|i| contains(&i.bounds, pt.0, pt.1))
                    {
                        CursorIcon::Grab
                    } else {
                        CursorIcon::Default
                    }
                }
                _ => CursorIcon::Default,
            },
            _ => CursorIcon::Text,
        }
    }

    /// Début d'un glissement : décide s'il déplace ou redimensionne l'objet sélectionné.
    pub fn edit_drag_start(&mut self, page: usize, pt: (f32, f32)) {
        self.edit.drag_mode = if self.annotate.tool != Tool::SelectObject {
            None
        } else if let Some(c) = self.corner_at(page, pt) {
            Some(DragMode::Resize(c))
        } else if self
            .selected_on(page)
            .is_some_and(|i| contains(&i.bounds, pt.0, pt.1))
        {
            Some(DragMode::Move)
        } else {
            None
        };
    }

    /// Limites de l'objet sélectionné pendant un glissement (aperçu).
    fn preview_bounds(&self, drag: &Drag) -> Option<PointRect> {
        let info = self.selected_on(drag.page)?;
        let (dx, dy) = (drag.current.0 - drag.start.0, drag.current.1 - drag.start.1);
        let mut b = info.bounds;
        match self.edit.drag_mode? {
            DragMode::Move => {
                b.left += dx;
                b.right += dx;
                b.bottom += dy;
                b.top += dy;
                Some(b)
            }
            DragMode::Resize(c) => {
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
        }
    }

    /// Clic simple avec l'outil de sélection d'objet.
    pub fn edit_click(&mut self, page: usize, pt: (f32, f32)) {
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

    pub fn open_text_prompt_for_selection(&mut self) {
        let Some((page, info)) = &self.edit.selected else {
            return;
        };
        if info.kind != ObjectKind::Text {
            return;
        }
        self.edit.text_prompt = Some(TextPrompt {
            page: *page,
            x: info.bounds.left,
            y: info.bounds.bottom,
            index: Some(info.index),
            text: info.text.clone().unwrap_or_default(),
            size: if info.font_size > 0.0 {
                info.font_size
            } else {
                12.0
            },
            bold: false,
            italic: false,
            family: FontFamily::Helvetica,
            rgb: info.fill_rgb.map(rgb_arr).unwrap_or([0, 0, 0]),
            focus: true,
        });
    }

    /// Fin d'un glissement avec un outil d'édition.
    pub fn edit_drag_finished(&mut self, drag: Drag) {
        log::debug!(
            "fin de glissement {:?} mode {:?} sélection {:?}",
            drag,
            self.edit.drag_mode,
            self.edit.selected.as_ref().map(|(p, i)| (*p, i.index))
        );
        let rect = drag.rect();
        let tiny = (rect.right - rect.left) < 2.0 && (rect.top - rect.bottom) < 2.0;
        match self.annotate.tool {
            Tool::SelectObject => {
                let bounds = self.preview_bounds(&drag);
                let mode = self.edit.drag_mode.take();
                let Some((page, index)) = self.edit.selected.as_ref().map(|(p, i)| (*p, i.index))
                else {
                    self.edit_click(drag.page, drag.start);
                    return;
                };
                match (mode, bounds) {
                    (Some(DragMode::Move), _) if !tiny => {
                        self.edit.reselect = Some((page, index));
                        self.worker.send(Request::Edit(EditOp::MoveObject {
                            page,
                            index,
                            dx: drag.current.0 - drag.start.0,
                            dy: drag.current.1 - drag.start.1,
                        }));
                    }
                    (Some(DragMode::Resize(_)), Some(bounds)) if !tiny => {
                        self.edit.reselect = Some((page, index));
                        self.worker.send(Request::Edit(EditOp::ResizeObject {
                            page,
                            index,
                            bounds,
                        }));
                    }
                    _ => self.edit_click(drag.page, drag.start),
                }
            }
            Tool::AddText => {
                self.edit.text_prompt = Some(TextPrompt {
                    page: drag.page,
                    x: drag.start.0,
                    y: drag.start.1,
                    index: None,
                    text: String::new(),
                    size: 12.0,
                    bold: false,
                    italic: false,
                    family: FontFamily::Helvetica,
                    rgb: [0, 0, 0],
                    focus: true,
                });
            }
            Tool::AddImage => {
                let dialog = rfd::FileDialog::new()
                    .set_title(self.t("tools.images.pick_one"))
                    .add_filter(
                        self.t("tools.images.filter"),
                        &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"],
                    );
                if let Some(path) = dialog.pick_file() {
                    let width_pt = self.edit.image_width_pt;
                    // Le point cliqué devient le coin haut-gauche de l'image.
                    let height_pt = match image::image_dimensions(&path) {
                        Ok((w, h)) if w > 0 => width_pt * h as f32 / w as f32,
                        _ => width_pt * 0.75,
                    };
                    self.worker.send(Request::Edit(EditOp::AddImage {
                        page: drag.page,
                        path,
                        x: drag.start.0,
                        y: drag.start.1 - height_pt,
                        width_pt,
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
        }
    }

    pub fn on_object_selected(&mut self, page: usize, info: Option<ObjectInfo>) {
        let lang = self.lang();
        match info {
            Some(info) => {
                self.status_msg = Some(tr_args(
                    lang,
                    "status.object_selected",
                    &[("kind", tr(lang, kind_key(info.kind)))],
                ));
                // La barre d'objet reflète l'objet sélectionné.
                if let Some(c) = info.stroke_rgb {
                    self.edit.stroke_rgb = rgb_arr(c);
                }
                if info.kind != ObjectKind::Text {
                    if let Some(c) = info.fill_rgb {
                        self.edit.fill_rgb = rgb_arr(c);
                    }
                    if info.stroke_width > 0.0 {
                        self.edit.stroke_width = info.stroke_width;
                    }
                }
                self.edit.selected = Some((page, info));
            }
            None => {
                self.edit.selected = None;
            }
        }
    }

    fn selected_ref(&self) -> Option<(usize, usize)> {
        self.edit.selected.as_ref().map(|(p, i)| (*p, i.index))
    }

    pub fn nudge_selected(&mut self, dx: f32, dy: f32) {
        if let Some((page, index)) = self.selected_ref() {
            self.edit.reselect = Some((page, index));
            self.worker.send(Request::Edit(EditOp::MoveObject {
                page,
                index,
                dx,
                dy,
            }));
        }
    }

    pub fn rotate_selected(&mut self, degrees: f32) {
        if let Some((page, index)) = self.selected_ref() {
            self.edit.reselect = Some((page, index));
            self.worker.send(Request::Edit(EditOp::RotateObject {
                page,
                index,
                degrees,
            }));
        }
    }

    pub fn duplicate_selected(&mut self) {
        if let Some((page, index)) = self.selected_ref() {
            self.edit.reselect = Some((page, usize::MAX));
            self.worker
                .send(Request::Edit(EditOp::DuplicateObject { page, index }));
        }
    }

    pub fn reorder_selected(&mut self, front: bool) {
        if let Some((page, index)) = self.selected_ref() {
            self.edit.reselect = Some((page, if front { usize::MAX } else { 0 }));
            self.worker
                .send(Request::Edit(EditOp::ReorderObject { page, index, front }));
        }
    }

    pub fn delete_selected_object(&mut self) {
        if let Some((page, info)) = self.edit.selected.take() {
            self.worker.send(Request::Edit(EditOp::DeleteObject {
                page,
                index: info.index,
            }));
        }
    }

    /// Cadre de sélection (avec aperçu du déplacement / redimensionnement) et aperçu des formes.
    pub fn paint_object_selection(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let to_screen = |b: &PointRect| {
            egui::Rect::from_min_max(
                page_rect.min + egui::vec2(b.left * k, (page_h_pt - b.top) * k),
                page_rect.min + egui::vec2(b.right * k, (page_h_pt - b.bottom) * k),
            )
        };
        let drag = self.annotate.drag.filter(|d| d.page == page_index);
        // Aperçu des formes en cours de tracé.
        if let Some(d) = drag {
            let stroke = Stroke::new(
                (self.edit.stroke_width * k).max(1.0),
                color32(self.edit.stroke_rgb),
            );
            let fill = if self.edit.fill {
                color32(self.edit.fill_rgb)
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
                    let p = |pt: (f32, f32)| {
                        page_rect.min + egui::vec2(pt.0 * k, (page_h_pt - pt.1) * k)
                    };
                    painter.line_segment([p(d.start), p(d.current)], stroke);
                }
                _ => {}
            }
        }
        let Some(info) = self.selected_on(page_index) else {
            return;
        };
        let bounds = drag
            .and_then(|d| self.preview_bounds(&d))
            .unwrap_or(info.bounds);
        let rect = to_screen(&bounds).expand(3.0);
        painter.rect_stroke(
            rect,
            2.0,
            Stroke::new(1.5, theme::BLUE),
            egui::StrokeKind::Outside,
        );
        for corner in [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ] {
            painter.rect_filled(
                egui::Rect::from_center_size(corner, egui::vec2(7.0, 7.0)),
                1.0,
                theme::BLUE,
            );
        }
    }

    /// Barre flottante des propriétés d'objet (couleurs, épaisseur), à droite de la palette.
    pub fn object_bar(&mut self, ctx: &egui::Context, origin: egui::Pos2) {
        if !self.annotate.tool.is_edit() {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let mut apply = false;
        let mut action: Option<Command> = None;
        let has_icons = self.has_icons;
        egui::Area::new(egui::Id::new("object-bar"))
            .fixed_pos(origin)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
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
                            if let Some((_, info)) = &self.edit.selected {
                                ui.separator();
                                for cmd in [
                                    Command::RotateObjectLeft,
                                    Command::RotateObjectRight,
                                    Command::DuplicateObject,
                                    Command::BringToFront,
                                    Command::SendToBack,
                                    Command::DeleteObject,
                                ] {
                                    let (g, f) = crate::shell::command_icon(cmd);
                                    if ui
                                        .add(
                                            egui::Button::new(crate::icons::icon(
                                                has_icons, g, f, 15.0,
                                            ))
                                            .frame(false)
                                            .min_size(egui::vec2(26.0, 26.0)),
                                        )
                                        .on_hover_text(tr(lang, cmd.info().label_key))
                                        .clicked()
                                    {
                                        action = Some(cmd);
                                    }
                                }
                                ui.separator();
                                if ui
                                    .button(tr(lang, "bar.apply"))
                                    .on_hover_text(tr_args(
                                        lang,
                                        "bar.apply_hint",
                                        &[("kind", tr(lang, kind_key(info.kind)))],
                                    ))
                                    .clicked()
                                {
                                    apply = true;
                                }
                            }
                        });
                    });
            });
        if let Some(cmd) = action {
            self.execute(ctx, cmd);
        }
        if apply {
            if let Some((page, info)) = &self.edit.selected {
                let style = if info.kind == ObjectKind::Text {
                    ObjectStyle {
                        fill_rgb: Some(rgb_tuple(self.edit.stroke_rgb)),
                        ..Default::default()
                    }
                } else {
                    ObjectStyle {
                        fill_rgb: self.edit.fill.then_some(rgb_tuple(self.edit.fill_rgb)),
                        stroke_rgb: Some(rgb_tuple(self.edit.stroke_rgb)),
                        stroke_width: Some(self.edit.stroke_width),
                    }
                };
                self.edit.reselect = Some((*page, info.index));
                self.worker.send(Request::Edit(EditOp::SetObjectStyle {
                    page: *page,
                    index: info.index,
                    style,
                }));
            }
        }
    }

    pub fn text_prompt_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let muted = theme::palette(self.settings.dark_theme).text_muted;
        let Some(prompt) = &mut self.edit.text_prompt else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let editing = prompt.index.is_some();
        let spec = crate::dialog::Spec::new(
            "text-object",
            tr(
                lang,
                if editing {
                    "dialog.text.title_edit"
                } else {
                    "dialog.text.title_new"
                },
            ),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(
            lang,
            if editing {
                "dialog.text.subtitle_edit"
            } else {
                "dialog.text.subtitle_new"
            },
        ))
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
                    if !editing {
                        ui.label(tr(lang, "dialog.text.font"));
                        egui::ComboBox::from_id_salt("text-font")
                            .selected_text(tr(lang, family_key(prompt.family)))
                            .show_ui(ui, |ui| {
                                for f in [
                                    FontFamily::Helvetica,
                                    FontFamily::Times,
                                    FontFamily::Courier,
                                ] {
                                    ui.selectable_value(
                                        &mut prompt.family,
                                        f,
                                        tr(lang, family_key(f)),
                                    );
                                }
                            });
                    }
                });
                if !editing {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut prompt.bold, tr(lang, "dialog.text.bold"));
                        ui.checkbox(&mut prompt.italic, tr(lang, "dialog.text.italic"));
                    });
                } else {
                    ui.label(
                        egui::RichText::new(tr(lang, "dialog.text.font_note"))
                            .size(12.0)
                            .color(muted),
                    );
                }
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
            let text = prompt.text.trim().to_string();
            if text.is_empty() {
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
                None => self.worker.send(Request::Edit(EditOp::AddText {
                    page: prompt.page,
                    x: prompt.x,
                    y: prompt.y,
                    text,
                    style: TextStyle {
                        size_pt: prompt.size,
                        bold: prompt.bold,
                        italic: prompt.italic,
                        family: prompt.family,
                        rgb: rgb_tuple(prompt.rgb),
                        opacity: 1.0,
                    },
                })),
            }
        } else if cancel {
            self.edit.text_prompt = None;
        }
    }
}
