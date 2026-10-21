//! Outils d'annotation : sélection de texte, note, panneau des commentaires (les boutons d'outils
//! sont dans la barre contextuelle de `workspace.rs`).

use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::icons::glyph;
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Key, Stroke};
use sunipdf_core::annot::{ReadingPoint, ReadingSelection};
use sunipdf_core::{AnnotationInfo, Command, MarkupKind, PointRect, TextSelection};
use sunipdf_i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tool {
    #[default]
    Select,
    Highlight,
    Underline,
    StrikeOut,
    Note,
    Count,
    SelectObject,
    AddText,
    AddImage,
    Rectangle,
    Ellipse,
    Line,
    /// Remplir un formulaire : champs surlignés et saisie en place (`forms.rs`).
    FillForm,
    /// Annotations avancées (`annot_adv.rs`) : vraies annotations PDF.
    Ink,
    FreeText,
    AnnotRect,
    AnnotEllipse,
    AnnotLine,
    AddLink,
    SelectAnnot,
    /// Caviardage : tracer des zones (`redact.rs`).
    Redact,
}

impl Tool {
    pub const ALL: [Tool; 21] = [
        Tool::Select,
        Tool::Highlight,
        Tool::Underline,
        Tool::StrikeOut,
        Tool::Note,
        Tool::Count,
        Tool::SelectObject,
        Tool::AddText,
        Tool::AddImage,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Line,
        Tool::FillForm,
        Tool::Ink,
        Tool::FreeText,
        Tool::AnnotRect,
        Tool::AnnotEllipse,
        Tool::AnnotLine,
        Tool::AddLink,
        Tool::SelectAnnot,
        Tool::Redact,
    ];

    pub fn command(self) -> Command {
        match self {
            Tool::Select => Command::ToolSelect,
            Tool::Highlight => Command::ToolHighlight,
            Tool::Underline => Command::ToolUnderline,
            Tool::StrikeOut => Command::ToolStrikeOut,
            Tool::Note => Command::ToolNote,
            Tool::Count => Command::ToolCount,
            Tool::SelectObject => Command::ToolSelectObject,
            Tool::AddText => Command::ToolAddText,
            Tool::AddImage => Command::ToolAddImage,
            Tool::Rectangle => Command::ToolRectangle,
            Tool::Ellipse => Command::ToolEllipse,
            Tool::Line => Command::ToolLine,
            Tool::FillForm => Command::ToolFillForm,
            Tool::Ink => Command::ToolInk,
            Tool::FreeText => Command::ToolFreeText,
            Tool::AnnotRect => Command::ToolAnnotRect,
            Tool::AnnotEllipse => Command::ToolAnnotEllipse,
            Tool::AnnotLine => Command::ToolAnnotLine,
            Tool::AddLink => Command::ToolLink,
            Tool::SelectAnnot => Command::ToolSelectAnnot,
            Tool::Redact => Command::ToolRedact,
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
    pub register: crate::annotation_register::RegisterState,
    pub tool: Tool,
    pub drag: Option<Drag>,
    pub selection: Option<(usize, TextSelection)>,
    pub reading: Option<ReadingSelection>,
    pub(crate) reading_drag: Option<(ReadingPoint, ReadingPoint)>,
    reading_generation: u64,
    reading_pending: Option<(u64, u64)>,
    reading_desired: Option<(ReadingPoint, ReadingPoint, bool)>,
    /// Annotation voulue par le glissement en cours de résolution (`TextInRect` envoyé) :
    /// changer d'outil avant la réponse ne change pas l'effet.
    pub pending_markup: Option<(MarkupKind, (u8, u8, u8))>,
    pub note_prompt: Option<(usize, f32, f32, String)>,
    pub comments_open: bool,
    pub annotations: Vec<AnnotationInfo>,
    pub annotations_page: Option<usize>,
    /// Outil actif du hub. Ici pour suivre l'onglet (`take_tab` / `put_tab` échangent `annotate`)
    /// et se vider à la fermeture du document.
    pub workspace: Option<crate::workspace::Workspace>,
    /// Annotations avancées : tracé, sélection, boîtes de dialogue, liens (`annot_adv.rs`).
    pub adv: crate::annot_adv::AnnotAdvState,
    /// Zones de caviardage non appliquées et boîtes de l'outil (suivent l'onglet comme `workspace`).
    pub redact: crate::redact::RedactState,
}

impl AnnotateState {
    pub fn clear_reading(&mut self) {
        self.reading_generation = self.reading_generation.wrapping_add(1);
        self.reading = None;
        self.reading_drag = None;
        self.reading_desired = None;
    }
}

/// Point écran → point PDF.
pub fn to_pdf(pos: egui::Pos2, page_rect: egui::Rect, zoom: f32, page_h_pt: f32) -> (f32, f32) {
    let k = zoom * SCREEN_PER_PDF_POINT;
    let x = (pos.x - page_rect.min.x) / k;
    let y = page_h_pt - (pos.y - page_rect.min.y) / k;
    (x.clamp(0.0, f32::MAX), y.clamp(0.0, page_h_pt))
}

impl SuniApp {
    pub(crate) fn start_reading(&mut self, point: ReadingPoint, word: bool) {
        self.annotate.clear_reading();
        self.annotate.selection = None;
        self.annotate.pending_markup = None;
        self.annotate.reading_drag = (!word).then_some((point, point));
        self.annotate.reading_desired = Some((point, point, word));
        self.send_reading();
    }

    pub(crate) fn move_reading(&mut self, point: ReadingPoint) {
        let Some((anchor, focus)) = self.annotate.reading_drag else {
            return;
        };
        if point == focus {
            return;
        }
        self.annotate.reading_drag = Some((anchor, point));
        self.annotate.reading_generation = self.annotate.reading_generation.wrapping_add(1);
        self.annotate.reading_desired = Some((anchor, point, false));
        self.send_reading();
    }

    fn send_reading(&mut self) {
        if self
            .annotate
            .reading_pending
            .is_some_and(|(epoch, _)| epoch != self.view.epoch)
        {
            self.annotate.reading_pending = None;
        }
        if self.annotate.reading_pending.is_some() {
            return;
        }
        let Some((anchor, focus, word)) = self.annotate.reading_desired.take() else {
            return;
        };
        let epoch = self.view.epoch;
        let generation = self.annotate.reading_generation;
        self.annotate.reading_pending = Some((epoch, generation));
        self.worker.send(Request::ReadingSelection {
            epoch,
            generation,
            anchor,
            focus,
            word,
        });
    }

    pub fn on_reading_selected(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<ReadingSelection, sunipdf_core::EngineError>,
    ) {
        if self.annotate.reading_pending != Some((epoch, generation)) {
            return;
        }
        self.annotate.reading_pending = None;
        if epoch == self.view.epoch && generation == self.annotate.reading_generation {
            match result {
                Ok(selection) => self.annotate.reading = Some(selection),
                Err(error) => self.error = Some(crate::app::error_key(&error)),
            }
        }
        self.send_reading();
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
        if let Some(reading) = &self.annotate.reading {
            if let Some((_, selection)) = reading.pages.iter().find(|(page, _)| *page == page_index)
            {
                for r in &selection.rects {
                    painter.rect_filled(to_screen(r), 2.0, theme::BLUE.gamma_multiply(0.3));
                }
            }
        }
        if let Some(drag) = &self.annotate.drag {
            if drag.page == page_index
                && !self.annotate.tool.is_edit()
                && !self.annotate.tool.is_adv()
                && self.annotate.tool != Tool::FillForm
                && self.annotate.tool != Tool::Redact
            {
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
            Tool::Count => {
                if tiny {
                    self.place_count_marker(drag.page, drag.start);
                }
            }
            Tool::Note => {
                self.annotate.note_prompt =
                    Some((drag.page, drag.start.0, drag.start.1, String::new()));
            }
            Tool::Select => {
                if tiny {
                    self.annotate.selection = None;
                } else {
                    self.annotate.pending_markup = None;
                    self.worker.send(Request::TextInRect {
                        page: drag.page,
                        rect,
                    });
                }
            }
            Tool::SelectObject
            | Tool::AddText
            | Tool::AddImage
            | Tool::Rectangle
            | Tool::Ellipse
            | Tool::Line => self.edit_drag_finished(drag),
            // Un glissement ne sélectionne rien : les champs se cliquent.
            Tool::FillForm => {}
            Tool::Ink
            | Tool::FreeText
            | Tool::AnnotRect
            | Tool::AnnotEllipse
            | Tool::AnnotLine
            | Tool::AddLink
            | Tool::SelectAnnot => self.adv_drag_finished(drag),
            Tool::Redact => self.redact_drag_finished(drag),
            _ => {
                // Outils de texte : la sélection revient par TextSelected, puis l'annotation est créée.
                if !tiny {
                    self.annotate.pending_markup = self.annotate.tool.markup();
                    self.worker.send(Request::TextInRect {
                        page: drag.page,
                        rect,
                    });
                }
            }
        }
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        if let Some(reading) = &self.annotate.reading {
            let text = reading
                .pages
                .iter()
                .map(|(_, selection)| selection.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                ctx.copy_text(text);
                self.status_msg = Some(tr(self.lang(), "status.copied").to_string());
            }
            return;
        }
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
        let spec = crate::dialog::Spec::new(
            "note",
            tr(lang, "dialog.note.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "dialog.note.subtitle"))
        .icon(glyph::NOTE, "✎")
        .width(440.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                let field = ui.add(
                    egui::TextEdit::multiline(text)
                        .hint_text(tr(lang, "dialog.note.placeholder"))
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "dialog.note.ok"))
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
}

/// Libellé traduit d'un sous-type d'annotation (identifiants stables du moteur, jamais un nom
/// de variante de bibliothèque ; sous-type inconnu → « Autre annotation »).
pub(crate) fn annotation_label(lang: sunipdf_i18n::Language, kind: &str) -> String {
    let id = if sunipdf_core::ANNOTATION_KINDS.contains(&kind) {
        kind
    } else {
        "other"
    };
    tr(lang, &format!("comments.kind.{id}")).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_i18n::Language;

    #[test]
    fn every_annotation_kind_has_a_translated_label() {
        for kind in sunipdf_core::ANNOTATION_KINDS {
            for lang in Language::ALL {
                let label = annotation_label(lang, kind);
                assert!(
                    !label.contains("comments.kind"),
                    "{kind} en {}",
                    lang.code()
                );
            }
        }
        assert_eq!(annotation_label(Language::French, "strikeout"), "Barré");
        assert_eq!(
            annotation_label(Language::English, "strikeout"),
            "Strikethrough"
        );
        assert_eq!(
            annotation_label(Language::French, "Strikeout"),
            annotation_label(Language::French, "other"),
            "nom de variante inconnu : libellé générique"
        );
    }
}
