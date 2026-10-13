//! Édition du contenu des pages : sélection d'objets, déplacement, texte, images, formes.

use crate::annotate::{Drag, Tool};
use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Key, Stroke};
use sunipdf_core::{ObjectInfo, ObjectKind, PointRect, ShapeKind, ShapeStyle, TextStyle};
use sunipdf_i18n::{tr, tr_args};

pub struct TextPrompt {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    /// `Some(index)` : modification d'un objet texte existant.
    pub index: Option<usize>,
    pub text: String,
    pub size: f32,
    pub bold: bool,
}

pub struct EditState {
    pub selected: Option<(usize, ObjectInfo)>,
    /// Après une modification, resélectionner cet objet (page, index).
    pub reselect: Option<(usize, usize)>,
    pub text_prompt: Option<TextPrompt>,
    pub image_width_pt: f32,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            selected: None,
            reselect: None,
            text_prompt: None,
            image_width_pt: 200.0,
        }
    }
}

fn kind_key(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Text => "object.kind.text",
        ObjectKind::Image => "object.kind.image",
        ObjectKind::Path => "object.kind.path",
        ObjectKind::Other => "object.kind.other",
    }
}

fn contains(b: &PointRect, x: f32, y: f32) -> bool {
    x >= b.left - 2.0 && x <= b.right + 2.0 && y >= b.bottom - 2.0 && y <= b.top + 2.0
}

impl SuniApp {
    /// Clic simple avec l'outil de sélection d'objet.
    pub fn edit_click(&mut self, page: usize, pt: (f32, f32)) {
        if let Some((p, info)) = &self.edit.selected {
            if *p == page && contains(&info.bounds, pt.0, pt.1) {
                return; // déjà sélectionné
            }
        }
        self.worker.send(Request::ObjectAt {
            page,
            x: pt.0,
            y: pt.1,
        });
    }

    /// Double-clic : modifier un objet texte sous le pointeur.
    pub fn edit_double_click(&mut self, page: usize, pt: (f32, f32)) {
        match &self.edit.selected {
            Some((p, info)) if *p == page && contains(&info.bounds, pt.0, pt.1) => {
                self.open_text_prompt_for_selection()
            }
            _ => self.edit_click(page, pt),
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
        });
    }

    /// Fin d'un glissement avec un outil d'édition.
    pub fn edit_drag_finished(&mut self, drag: Drag) {
        let rect = drag.rect();
        let tiny = (rect.right - rect.left) < 2.0 && (rect.top - rect.bottom) < 2.0;
        match self.annotate.tool {
            Tool::SelectObject => {
                let moving = matches!(&self.edit.selected, Some((p, info)) if *p == drag.page && contains(&info.bounds, drag.start.0, drag.start.1));
                if moving && !tiny {
                    let (page, index) = self
                        .edit
                        .selected
                        .as_ref()
                        .map(|(p, i)| (*p, i.index))
                        .expect("sélection");
                    self.edit.reselect = Some((page, index));
                    self.worker.send(Request::Edit(EditOp::MoveObject {
                        page,
                        index,
                        dx: drag.current.0 - drag.start.0,
                        dy: drag.current.1 - drag.start.1,
                    }));
                } else {
                    self.edit_click(drag.page, drag.start);
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
                    self.worker.send(Request::Edit(EditOp::AddImage {
                        page: drag.page,
                        path,
                        x: drag.start.0,
                        y: drag.start.1 - width_pt * 0.75, // coin bas-gauche sous le clic
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
                    style: ShapeStyle::default(),
                }));
            }
            _ => {}
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
                self.edit.selected = Some((page, info));
            }
            None => {
                self.edit.selected = None;
            }
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

    /// Cadre de sélection de l'objet courant.
    pub fn paint_object_selection(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let Some((page, info)) = &self.edit.selected else {
            return;
        };
        if *page != page_index {
            return;
        }
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let b = &info.bounds;
        let rect = egui::Rect::from_min_max(
            page_rect.min + egui::vec2(b.left * k, (page_h_pt - b.top) * k),
            page_rect.min + egui::vec2(b.right * k, (page_h_pt - b.bottom) * k),
        )
        .expand(3.0);
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

    pub fn text_prompt_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(prompt) = &mut self.edit.text_prompt else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        let editing = prompt.index.is_some();
        egui::Modal::new(egui::Id::new("text-object")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr(
                lang,
                if editing {
                    "dialog.text.title_edit"
                } else {
                    "dialog.text.title_new"
                },
            ));
            let field = ui.add(
                egui::TextEdit::multiline(&mut prompt.text)
                    .hint_text(tr(lang, "dialog.text.placeholder"))
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            field.request_focus();
            ui.horizontal(|ui| {
                ui.label(tr(lang, "dialog.text.size"));
                ui.add(
                    egui::DragValue::new(&mut prompt.size)
                        .range(4.0..=200.0)
                        .speed(0.5),
                );
                if !editing {
                    ui.checkbox(&mut prompt.bold, tr(lang, "dialog.text.bold"));
                }
            });
            if editing {
                ui.label(
                    egui::RichText::new(tr(lang, "dialog.text.font_note"))
                        .size(12.0)
                        .color(theme::palette(self.settings.dark_theme).text_muted),
                );
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "dialog.text.ok")).clicked()
                    || ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if ui.button(tr(lang, "dialog.password.cancel")).clicked()
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            });
        });
        if submit {
            let prompt = self.edit.text_prompt.take().expect("prompt");
            let text = prompt.text.trim().to_string();
            if !text.is_empty() {
                match prompt.index {
                    Some(index) => {
                        self.edit.reselect = Some((prompt.page, index));
                        self.worker.send(Request::Edit(EditOp::SetText {
                            page: prompt.page,
                            index,
                            text,
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
                            rgb: (0, 0, 0),
                        },
                    })),
                }
            }
        } else if cancel {
            self.edit.text_prompt = None;
        }
    }
}
