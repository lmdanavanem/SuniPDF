//! Outils d'annotation : palette flottante, sélection de texte, note, panneau des commentaires.

use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Align, Button, Frame, Key, Layout, Margin, RichText, Stroke};
use sunipdf_core::{AnnotationInfo, Command, MarkupKind, PointRect, TextSelection};
use sunipdf_i18n::{tr, tr_args};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tool {
    #[default]
    Select,
    Highlight,
    Underline,
    StrikeOut,
    Note,
}

impl Tool {
    pub const ALL: [Tool; 5] = [
        Tool::Select,
        Tool::Highlight,
        Tool::Underline,
        Tool::StrikeOut,
        Tool::Note,
    ];

    pub fn command(self) -> Command {
        match self {
            Tool::Select => Command::ToolSelect,
            Tool::Highlight => Command::ToolHighlight,
            Tool::Underline => Command::ToolUnderline,
            Tool::StrikeOut => Command::ToolStrikeOut,
            Tool::Note => Command::ToolNote,
        }
    }

    pub fn markup(self) -> Option<(MarkupKind, (u8, u8, u8))> {
        match self {
            Tool::Highlight => Some((MarkupKind::Highlight, (0xFF, 0xE6, 0x00))),
            Tool::Underline => Some((MarkupKind::Underline, (0x25, 0x63, 0xEB))),
            Tool::StrikeOut => Some((MarkupKind::StrikeOut, (0xDC, 0x26, 0x26))),
            _ => None,
        }
    }
}

/// Glissement en cours sur la page (coordonnées PDF).
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    pub page: usize,
    pub start: (f32, f32),
    pub current: (f32, f32),
}

impl Drag {
    pub fn rect(&self) -> PointRect {
        PointRect {
            left: self.start.0.min(self.current.0),
            right: self.start.0.max(self.current.0),
            bottom: self.start.1.min(self.current.1),
            top: self.start.1.max(self.current.1),
        }
    }
}

#[derive(Default)]
pub struct AnnotateState {
    pub tool: Tool,
    pub drag: Option<Drag>,
    pub selection: Option<(usize, TextSelection)>,
    pub note_prompt: Option<(usize, f32, f32, String)>,
    pub comments_open: bool,
    pub annotations: Vec<AnnotationInfo>,
    pub annotations_page: Option<usize>,
}

/// Point écran → point PDF.
pub fn to_pdf(pos: egui::Pos2, page_rect: egui::Rect, zoom: f32, page_h_pt: f32) -> (f32, f32) {
    let k = zoom * SCREEN_PER_PDF_POINT;
    let x = (pos.x - page_rect.min.x) / k;
    let y = page_h_pt - (pos.y - page_rect.min.y) / k;
    (x.clamp(0.0, f32::MAX), y.clamp(0.0, page_h_pt))
}

impl SuniApp {
    /// Palette d'outils flottante, en haut à gauche de la zone document.
    pub fn tool_palette(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, origin: egui::Pos2) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let mut chosen = None;
        egui::Area::new(egui::Id::new("tool-palette"))
            .fixed_pos(origin)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(Margin::same(4))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 2],
                        blur: 8,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(40),
                    })
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for tool in Tool::ALL {
                                let (g, f) = crate::shell::command_icon(tool.command());
                                let resp = ui
                                    .add(
                                        Button::new(icons::icon(self.has_icons, g, f, 16.0))
                                            .frame(false)
                                            .selected(self.annotate.tool == tool)
                                            .min_size(egui::vec2(30.0, 30.0)),
                                    )
                                    .on_hover_text(tr(lang, tool.command().info().label_key));
                                if resp.clicked() {
                                    chosen = Some(tool);
                                }
                            }
                        });
                    });
            });
        let _ = ui;
        if let Some(tool) = chosen {
            self.execute(ctx, tool.command());
        }
    }

    /// Dessine la sélection de texte et le rectangle de glissement en cours.
    pub fn paint_selection(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let to_screen = |r: &PointRect| {
            egui::Rect::from_min_max(
                page_rect.min + egui::vec2(r.left * k, (page_h_pt - r.top) * k),
                page_rect.min + egui::vec2(r.right * k, (page_h_pt - r.bottom) * k),
            )
        };
        if let Some((page, sel)) = &self.annotate.selection {
            if *page == page_index {
                for r in &sel.rects {
                    painter.rect_filled(to_screen(r), 2.0, theme::BLUE.gamma_multiply(0.3));
                }
            }
        }
        if let Some(drag) = &self.annotate.drag {
            if drag.page == page_index {
                painter.rect_stroke(
                    to_screen(&drag.rect()),
                    0.0,
                    Stroke::new(1.0, theme::BLUE),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

    /// Fin d'un glissement : sélection de texte, annotation de texte ou note selon l'outil.
    pub fn finish_drag(&mut self, drag: Drag) {
        let rect = drag.rect();
        let tiny = (rect.right - rect.left) < 2.0 && (rect.top - rect.bottom) < 2.0;
        match self.annotate.tool {
            Tool::Note => {
                self.annotate.note_prompt =
                    Some((drag.page, drag.start.0, drag.start.1, String::new()));
            }
            Tool::Select => {
                if tiny {
                    self.annotate.selection = None;
                } else {
                    self.worker.send(Request::TextInRect {
                        page: drag.page,
                        rect,
                    });
                }
            }
            _ => {
                // Outils de texte : la sélection revient par TextSelected, puis l'annotation est créée.
                if !tiny {
                    self.worker.send(Request::TextInRect {
                        page: drag.page,
                        rect,
                    });
                }
            }
        }
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        if let Some((_, sel)) = &self.annotate.selection {
            if !sel.text.is_empty() {
                ctx.copy_text(sel.text.clone());
                self.status_msg = Some(tr(self.lang(), "status.copied").to_string());
            }
        }
    }

    pub fn note_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some((page, x, y, text)) = &mut self.annotate.note_prompt else {
            return;
        };
        let mut submit = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("note")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(tr(lang, "dialog.note.title"));
            let field = ui.add(
                egui::TextEdit::multiline(text)
                    .hint_text(tr(lang, "dialog.note.placeholder"))
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            field.request_focus();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "dialog.note.ok")).clicked()
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
            let (page, x, y, text) = (*page, *x, *y, text.trim().to_string());
            self.annotate.note_prompt = None;
            if !text.is_empty() {
                self.worker
                    .send(Request::Edit(EditOp::Note { page, x, y, text }));
            }
        } else if cancel {
            self.annotate.note_prompt = None;
        }
    }

    /// Demande la liste des annotations de la page courante si elle n'est pas à jour.
    pub fn refresh_annotations_if_needed(&mut self) {
        if !self.annotate.comments_open {
            return;
        }
        let Some(doc) = &self.document else { return };
        if self.annotate.annotations_page != Some(doc.page) {
            self.annotate.annotations_page = Some(doc.page);
            self.annotate.annotations.clear();
            self.worker.send(Request::Annotations { page: doc.page });
        }
    }

    pub fn comments_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let Some(doc) = &self.document else { return };
        let page = doc.page;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, "comments.title"), 15.0));
            ui.label(
                RichText::new(self.annotate.annotations.len().to_string())
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
        if self.annotate.annotations.is_empty() {
            ui.label(RichText::new(tr(lang, "comments.empty")).color(p.text_muted));
        }
        let mut delete = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for a in &self.annotate.annotations {
                    Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(8)
                        .inner_margin(10)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(icons::semibold(annotation_label(lang, &a.kind), 13.0));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add(
                                            Button::new(icons::icon(
                                                self.has_icons,
                                                glyph::DELETE,
                                                "🗑",
                                                14.0,
                                            ))
                                            .frame(false),
                                        )
                                        .on_hover_text(tr(lang, "comments.delete"))
                                        .clicked()
                                    {
                                        delete = Some(a.index);
                                    }
                                });
                            });
                            if let Some(c) = &a.contents {
                                ui.label(RichText::new(c).size(13.0));
                            }
                        });
                    ui.add_space(6.0);
                }
            });
        if let Some(index) = delete {
            self.worker
                .send(Request::Edit(EditOp::DeleteAnnotation { page, index }));
        }
    }
}

fn annotation_label(lang: sunipdf_i18n::Language, kind: &str) -> String {
    let key = match kind {
        "Highlight" => "comments.kind.highlight",
        "Underline" => "comments.kind.underline",
        "StrikeOut" => "comments.kind.strikeout",
        "Text" => "comments.kind.note",
        "Link" => "comments.kind.link",
        "Widget" => "comments.kind.widget",
        _ => "",
    };
    if key.is_empty() {
        kind.to_string()
    } else {
        tr(lang, key).to_string()
    }
}
