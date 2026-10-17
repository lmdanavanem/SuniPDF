//! Guarded drafts for native-font FreeText and additional standard annotations.

use crate::app::SuniApp;
use crate::editor::{rgb_tuple, TextFont};
use crate::worker::{EditOp, Request};
use std::io::Read;
use sunipdf_core::annot_rich::{
    RichAnnotation, RichAnnotationPreview, RichFreeTextStyle, StampKind, VertexAnnotKind,
};
use sunipdf_core::{AnnotStyle, EngineError, FontFamily, PointRect, TextStyle};
use sunipdf_i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RichMode {
    FreeText,
    Polygon,
    Polyline,
    Cloud,
    Stamp,
    Attachment,
    EraseInk,
}

impl RichMode {
    fn key(self) -> &'static str {
        match self {
            Self::FreeText => "rich.free_text",
            Self::Polygon => "rich.polygon",
            Self::Polyline => "rich.polyline",
            Self::Cloud => "rich.cloud",
            Self::Stamp => "rich.stamp",
            Self::Attachment => "rich.attachment",
            Self::EraseInk => "rich.eraser",
        }
    }
}

pub struct RichDraft {
    pub page: usize,
    pub epoch: u64,
    pub generation: u64,
    pub mode: RichMode,
    pub rect: PointRect,
    pub text: String,
    pub points: String,
    pub font: TextFont,
    pub bold: bool,
    pub italic: bool,
    pub size: f32,
    pub rgb: [u8; 3],
    pub fill: bool,
    pub fill_rgb: [u8; 3],
    pub width: f32,
    pub opacity: f32,
    pub radius: f32,
    pub stamp: StampKind,
    pub filename: String,
    pub bytes: Vec<u8>,
    pub target: Option<usize>,
    pub replace: bool,
    pub font_chosen: bool,
    pub preview: Option<egui::TextureHandle>,
    pub prepared: bool,
    pub preview_index: usize,
    pub busy: bool,
    pub applying: bool,
    pub error: Option<String>,
    author: Option<String>,
}

fn points(text: &str, limit: usize) -> Option<Vec<(f32, f32)>> {
    let mut result = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if result.len() >= limit {
            return None;
        }
        let mut values = line
            .split([',', ';', ' ', '\t'])
            .filter(|value| !value.is_empty());
        let (x, y) = (
            values.next()?.parse::<f32>().ok()?,
            values.next()?.parse::<f32>().ok()?,
        );
        if values.next().is_some() || !x.is_finite() || !y.is_finite() {
            return None;
        }
        result.push((x, y));
    }
    (!result.is_empty()).then_some(result)
}

impl RichDraft {
    fn valid_inputs(&self) -> bool {
        match self.mode {
            RichMode::FreeText => {
                !self.text.trim().is_empty() && (!self.replace || self.font_chosen)
            }
            RichMode::Polygon | RichMode::Polyline | RichMode::Cloud => {
                points(&self.points, 512).is_some()
            }
            RichMode::EraseInk => self.target.is_some() && points(&self.points, 256).is_some(),
            RichMode::Attachment => !self.filename.is_empty(),
            RichMode::Stamp => true,
        }
    }
    fn operation(&self) -> Option<RichAnnotation> {
        let style = AnnotStyle {
            rgb: rgb_tuple(self.rgb),
            fill_rgb: self.fill.then_some(rgb_tuple(self.fill_rgb)),
            width_pt: self.width,
            opacity: self.opacity / 100.0,
            dash_pattern: Vec::new(),
        };
        Some(match self.mode {
            RichMode::FreeText => {
                if self.text.trim().is_empty() || (self.replace && !self.font_chosen) {
                    return None;
                }
                let (family, font_file, font_face_index) = match &self.font {
                    TextFont::Standard(family) => (*family, None, 0),
                    TextFont::System(font) => (
                        FontFamily::Helvetica,
                        Some(font.path.clone()),
                        font.face_index,
                    ),
                };
                RichAnnotation::FreeText {
                    index: if self.replace { self.target } else { None },
                    rect: self.rect,
                    text: self.text.clone(),
                    style: RichFreeTextStyle {
                        text: TextStyle {
                            family,
                            font_file,
                            font_face_index,
                            bold: self.bold,
                            italic: self.italic,
                            size_pt: self.size,
                            rgb: rgb_tuple(self.rgb),
                            opacity: self.opacity / 100.0,
                        },
                        fill_rgb: style.fill_rgb,
                        border_pt: self.width,
                    },
                }
            }
            RichMode::Polygon | RichMode::Polyline | RichMode::Cloud => RichAnnotation::Vertices {
                kind: match self.mode {
                    RichMode::Polygon => VertexAnnotKind::Polygon,
                    RichMode::Polyline => VertexAnnotKind::Polyline,
                    _ => VertexAnnotKind::Cloud,
                },
                points: points(&self.points, 512)?,
                style: if self.mode == RichMode::Polyline {
                    AnnotStyle {
                        fill_rgb: None,
                        ..style
                    }
                } else {
                    style
                },
            },
            RichMode::Stamp => RichAnnotation::Stamp {
                rect: self.rect,
                kind: self.stamp,
            },
            RichMode::Attachment => {
                if self.filename.is_empty() {
                    return None;
                }
                RichAnnotation::Attachment {
                    rect: self.rect,
                    filename: self.filename.clone(),
                    bytes: self.bytes.clone(),
                }
            }
            RichMode::EraseInk => RichAnnotation::EraseInk {
                index: self.target?,
                path: points(&self.points, 256)?,
                radius: self.radius,
            },
        })
    }

    fn clear_preview(&mut self) {
        self.preview = None;
        self.prepared = false;
        self.error = None;
    }
}

impl SuniApp {
    pub(crate) fn open_rich_from_free_text_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.annotate.adv.free_text.take() else {
            return;
        };
        if prompt.pending {
            self.annotate.adv.free_text = Some(prompt);
            return;
        }
        self.open_rich_annotations(ctx);
        if let Some(draft) = self.annotate.adv.rich.as_mut() {
            draft.page = prompt.page;
            draft.rect = prompt.rect;
            draft.text = prompt.text;
            draft.size = prompt.size;
            draft.rgb = prompt.rgb;
            draft.fill = prompt.fill;
            draft.fill_rgb = prompt.fill_rgb;
            draft.width = prompt.border;
            draft.target = prompt.index;
            draft.replace = prompt.index.is_some();
        } else {
            self.annotate.adv.free_text = Some(prompt);
        }
    }
    pub fn open_rich_annotations(&mut self, _ctx: &egui::Context) {
        self.open_rich_draft(false);
    }

    pub fn open_ink_eraser(&mut self, _ctx: &egui::Context) {
        self.open_rich_draft(true);
    }

    fn open_rich_draft(&mut self, eraser: bool) {
        if !self.ink_can_start() || self.form_builder.active() || self.edit.text_prompt.is_some() {
            return;
        }
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let page = document.page;
        let selected = self
            .annotate
            .adv
            .selected
            .as_ref()
            .filter(|(p, detail)| *p == page && !detail.locked && detail.ours);
        if eraser && !selected.is_some_and(|(_, detail)| detail.subtype == "Ink") {
            self.status_msg = Some(tr(self.lang(), "rich.select_ink").into());
            return;
        }
        let selected = selected.map(|(_, detail)| detail.clone());
        let size = document.info.page_sizes[page];
        let target = selected
            .as_ref()
            .filter(|detail| detail.subtype == if eraser { "Ink" } else { "FreeText" })
            .map(|detail| detail.index);
        let rect = selected
            .as_ref()
            .filter(|detail| detail.subtype == "FreeText")
            .map(|detail| detail.rect)
            .unwrap_or(PointRect {
                left: 20.0,
                bottom: (size.height_pt - 160.0).max(20.0),
                right: 280.0f32.min(size.width_pt - 20.0),
                top: (size.height_pt - 20.0).max(40.0),
            });
        self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
        self.annotate.adv.rich = Some(RichDraft {
            page,
            epoch: self.view.epoch,
            generation: self.annotate.adv.rich_generation,
            mode: if eraser {
                RichMode::EraseInk
            } else {
                RichMode::FreeText
            },
            rect,
            text: selected
                .as_ref()
                .filter(|detail| detail.subtype == "FreeText")
                .and_then(|detail| detail.contents.clone())
                .unwrap_or_default(),
            points: "40,70\n160,70\n100,150".into(),
            font: TextFont::Standard(FontFamily::Helvetica),
            bold: false,
            italic: false,
            size: 14.0,
            rgb: [180, 20, 35],
            fill: false,
            fill_rgb: [255, 250, 230],
            width: 1.0,
            opacity: 100.0,
            radius: 8.0,
            stamp: StampKind::Approved,
            filename: String::new(),
            bytes: Vec::new(),
            target,
            replace: false,
            font_chosen: false,
            preview: None,
            prepared: false,
            preview_index: 0,
            busy: false,
            applying: false,
            error: None,
            author: self.annot_author(),
        });
        if eraser {
            if let Some(detail) = selected {
                let x = (detail.rect.left + detail.rect.right) / 2.0;
                let y = (detail.rect.bottom + detail.rect.top) / 2.0;
                self.annotate.adv.rich.as_mut().unwrap().points = format!("{x:.2},{y:.2}");
            }
        }
    }

    fn rich_ready(&self) -> bool {
        self.document.is_some()
            && matches!(self.engine, crate::app::EngineState::Ready)
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && !self.processing.busy
            && !self.office.busy
            && !self.sign.busy
            && !self.saving
            && !self.printing.busy
            && !self.ocr.busy
            && !self.certificate.busy
            && !self.document_transition_pending()
    }

    fn rich_context_current(&mut self) -> bool {
        let current = self.annotate.adv.rich.as_ref().is_some_and(|draft| {
            draft.epoch == self.view.epoch
                && self
                    .document
                    .as_ref()
                    .is_some_and(|document| draft.page < document.info.page_count())
                && self.opening.is_none()
                && self.tabs.resuming.is_none()
        });
        if !current {
            let error = tr(self.lang(), "rich.stale").to_string();
            if let Some(draft) = self.annotate.adv.rich.as_mut() {
                draft.busy = false;
                draft.preview = None;
                draft.prepared = false;
                draft.error = Some(error);
            }
        }
        current
    }

    pub(crate) fn request_rich_preview(&mut self) {
        if !self.rich_context_current() || !self.rich_ready() {
            return;
        }
        let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| !draft.busy && !draft.applying)
        else {
            return;
        };
        let Some(annotation) = draft.operation() else {
            return;
        };
        self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
        draft.generation = self.annotate.adv.rich_generation;
        draft.clear_preview();
        draft.busy = true;
        self.worker.send(Request::PreviewRichAnnotation {
            page: draft.page,
            annotation,
            author: draft.author.clone(),
            epoch: draft.epoch,
            generation: draft.generation,
        });
    }

    pub(crate) fn on_rich_preview(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<RichAnnotationPreview, EngineError>,
    ) {
        if !self.rich_context_current() || epoch != self.view.epoch {
            return;
        }
        let Some(draft) =
            self.annotate.adv.rich.as_mut().filter(|draft| {
                draft.epoch == epoch && draft.generation == generation && draft.busy
            })
        else {
            return;
        };
        draft.busy = false;
        match result {
            Ok(preview) => {
                draft.preview_index = preview.index;
                draft.prepared = true;
                draft.preview = Some(ctx.load_texture(
                    "rich-annotation-preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [preview.image.width as usize, preview.image.height as usize],
                        &preview.image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(error) => draft.error = Some(error.to_string()),
        }
    }

    pub(crate) fn apply_rich_draft(&mut self) {
        if !self.rich_context_current() || !self.rich_ready() {
            return;
        }
        let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| draft.prepared && !draft.busy && !draft.applying)
        else {
            return;
        };
        draft.applying = true;
        self.annotate.adv.reselect = Some((draft.page, draft.preview_index));
        self.worker
            .send(Request::Edit(EditOp::CommitRichAnnotationPreview {
                epoch: draft.epoch,
                generation: draft.generation,
            }));
    }

    pub(crate) fn rich_annotation_failed(&mut self, error: &EngineError) {
        if let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| draft.applying)
        {
            draft.applying = false;
            draft.busy = false;
            draft.prepared = false;
            draft.preview = None;
            draft.error = Some(error.to_string());
        }
    }

    pub(crate) fn rich_annotation_dialog(&mut self, ctx: &egui::Context) {
        let current = self.rich_context_current();
        let ready = self.rich_ready();
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let fonts = self.edit.system_fonts.clone();
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return;
        };
        let (mut preview, mut apply, mut cancel, mut choose_file, mut want_fonts) =
            (false, false, false, false, false);
        let can_preview =
            current && ready && !draft.busy && !draft.applying && draft.valid_inputs();
        let can_apply = current && ready && draft.prepared && !draft.busy && !draft.applying;
        let can_cancel = !draft.applying;
        crate::dialog::show(
            ctx,
            crate::dialog::Spec::new("rich-annotations", tr(lang, "rich.title"), dark, has_icons)
                .subtitle(tr(lang, "rich.note"))
                .width(680.0),
            |ui| {
                let mut changed = false;
                ui.add_enabled_ui(!draft.busy && !draft.applying, |ui| {
                    if draft.mode != RichMode::EraseInk {
                        let old = draft.mode;
                        egui::ComboBox::from_id_salt("rich-kind")
                            .selected_text(tr(lang, draft.mode.key()))
                            .show_ui(ui, |ui| {
                                for mode in [
                                    RichMode::FreeText,
                                    RichMode::Polygon,
                                    RichMode::Polyline,
                                    RichMode::Cloud,
                                    RichMode::Stamp,
                                    RichMode::Attachment,
                                ] {
                                    ui.selectable_value(
                                        &mut draft.mode,
                                        mode,
                                        tr(lang, mode.key()),
                                    );
                                }
                            });
                        if old != draft.mode {
                            changed = true;
                            if draft.mode == RichMode::Attachment {
                                draft.rect.right = draft.rect.left + 24.0;
                                draft.rect.top = draft.rect.bottom + 32.0;
                            }
                        }
                    }
                    if matches!(
                        draft.mode,
                        RichMode::FreeText | RichMode::Stamp | RichMode::Attachment
                    ) {
                        ui.horizontal_wrapped(|ui| {
                            for (label, value) in [
                                ("rich.left", &mut draft.rect.left),
                                ("rich.bottom", &mut draft.rect.bottom),
                                ("rich.right", &mut draft.rect.right),
                                ("rich.top", &mut draft.rect.top),
                            ] {
                                ui.label(tr(lang, label));
                                changed |= ui
                                    .add(egui::DragValue::new(value).speed(1.0).suffix(" pt"))
                                    .changed();
                            }
                        });
                    }
                    match draft.mode {
                        RichMode::FreeText => {
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut draft.text)
                                        .desired_rows(4)
                                        .desired_width(f32::INFINITY)
                                        .hint_text(tr(lang, "dialog.free_text.placeholder")),
                                )
                                .changed();
                            if draft.target.is_some() {
                                changed |= ui
                                    .checkbox(&mut draft.replace, tr(lang, "rich.replace_selected"))
                                    .changed();
                            }
                            let old = draft.font.clone();
                            let (wanted, _) = crate::inline_text::font_picker(
                                ui,
                                lang,
                                &mut draft.font,
                                &mut self.edit.font_filter,
                                fonts.as_deref().map(|fonts| fonts.as_slice()),
                            );
                            want_fonts |= wanted;
                            if old != draft.font {
                                draft.font_chosen = true;
                                changed = true;
                            }
                            if ui.button(tr(lang, "rich.confirm_font")).clicked() {
                                draft.font_chosen = true;
                                changed = true;
                            }
                            ui.horizontal_wrapped(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.size)
                                            .range(4.0..=200.0)
                                            .suffix(" pt"),
                                    )
                                    .changed();
                                ui.add_enabled_ui(
                                    matches!(draft.font, TextFont::Standard(_)),
                                    |ui| {
                                        changed |= ui
                                            .toggle_value(
                                                &mut draft.bold,
                                                egui::RichText::new("B").strong(),
                                            )
                                            .changed();
                                        changed |= ui
                                            .toggle_value(
                                                &mut draft.italic,
                                                egui::RichText::new("I").italics(),
                                            )
                                            .changed();
                                    },
                                );
                            });
                            ui.label(tr(lang, "rich.font_note"));
                        }
                        RichMode::Polygon
                        | RichMode::Polyline
                        | RichMode::Cloud
                        | RichMode::EraseInk => {
                            ui.label(tr(lang, "rich.points"));
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut draft.points)
                                        .desired_rows(4)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed();
                            if draft.mode == RichMode::EraseInk {
                                ui.label(tr(lang, "rich.eraser_note"));
                                ui.label(tr(lang, "rich.radius"));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.radius)
                                            .range(0.1..=100.0)
                                            .suffix(" pt"),
                                    )
                                    .changed();
                            }
                        }
                        RichMode::Stamp => {
                            let old = draft.stamp;
                            egui::ComboBox::from_id_salt("rich-stamp")
                                .selected_text(tr(lang, stamp_key(draft.stamp)))
                                .show_ui(ui, |ui| {
                                    for stamp in [
                                        StampKind::Approved,
                                        StampKind::Draft,
                                        StampKind::Confidential,
                                        StampKind::Rejected,
                                    ] {
                                        ui.selectable_value(
                                            &mut draft.stamp,
                                            stamp,
                                            tr(lang, stamp_key(stamp)),
                                        );
                                    }
                                });
                            changed |= old != draft.stamp;
                        }
                        RichMode::Attachment => {
                            if ui.button(tr(lang, "rich.choose_file")).clicked() {
                                choose_file = true;
                            }
                            ui.label(&draft.filename);
                            ui.label(tr(lang, "rich.attachment_note"));
                        }
                    }
                    if matches!(
                        draft.mode,
                        RichMode::FreeText
                            | RichMode::Polygon
                            | RichMode::Polyline
                            | RichMode::Cloud
                    ) {
                        ui.horizontal_wrapped(|ui| {
                            changed |= ui.color_edit_button_srgb(&mut draft.rgb).changed();
                            ui.label(tr(lang, "bar.width"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut draft.width)
                                        .range(if draft.mode == RichMode::FreeText {
                                            0.0..=50.0
                                        } else {
                                            0.1..=50.0
                                        })
                                        .suffix(" pt"),
                                )
                                .changed();
                            ui.label(tr(lang, "bar.opacity"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut draft.opacity)
                                        .range(0.0..=100.0)
                                        .suffix(" %"),
                                )
                                .changed();
                            if draft.mode != RichMode::Polyline {
                                changed |=
                                    ui.checkbox(&mut draft.fill, tr(lang, "bar.fill")).changed();
                                if draft.fill {
                                    changed |=
                                        ui.color_edit_button_srgb(&mut draft.fill_rgb).changed();
                                }
                            }
                        });
                    }
                });
                if changed {
                    draft.clear_preview();
                }
                if draft.busy || draft.applying {
                    ui.spinner();
                }
                if let Some(error) = &draft.error {
                    ui.colored_label(crate::theme::palette(dark).error, error);
                }
                if let Some(texture) = &draft.preview {
                    ui.add(
                        egui::Image::new(texture).fit_to_exact_size(
                            texture.size_vec2()
                                * (600.0 / texture.size_vec2().x)
                                    .min(240.0 / texture.size_vec2().y)
                                    .min(1.0),
                        ),
                    );
                }
            },
            |ui| {
                ui.add_enabled_ui(can_apply, |ui| {
                    apply = crate::dialog::primary(ui, tr(lang, "bar.apply"));
                });
                ui.add_enabled_ui(can_preview, |ui| {
                    preview = crate::dialog::secondary(ui, tr(lang, "paragraph.preview"));
                });
                ui.add_enabled_ui(can_cancel, |ui| {
                    cancel = crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                        });
                });
            },
        );
        if want_fonts && !self.edit.fonts_requested {
            self.edit.fonts_requested = true;
            self.worker.send(Request::SystemFonts);
        }
        if cancel {
            let draft = self.annotate.adv.rich.take().unwrap();
            self.worker.send(Request::DiscardRichAnnotationPreview {
                epoch: draft.epoch,
                generation: draft.generation,
            });
            self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
        } else if choose_file {
            self.choose_rich_attachment();
        } else if preview {
            self.request_rich_preview();
        } else if apply {
            self.apply_rich_draft();
        }
    }

    fn choose_rich_attachment(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr(self.lang(), "rich.choose_file"))
            .pick_file()
        else {
            return;
        };
        let result = (|| {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| EngineError::Other(tr(self.lang(), "rich.file_error").into()))?
                .to_string();
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err(EngineError::Other(
                    tr(self.lang(), "rich.file_error").into(),
                ));
            }
            Ok((filename, bytes))
        })();
        if let Some(draft) = self.annotate.adv.rich.as_mut() {
            draft.clear_preview();
            match result {
                Ok((filename, bytes)) => {
                    draft.filename = filename;
                    draft.bytes = bytes;
                }
                Err(error) => draft.error = Some(error.to_string()),
            }
        }
    }
}

fn stamp_key(kind: StampKind) -> &'static str {
    match kind {
        StampKind::Approved => "rich.stamp_approved",
        StampKind::Draft => "rich.stamp_draft",
        StampKind::Confidential => "rich.stamp_confidential",
        StampKind::Rejected => "rich.stamp_rejected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview() -> RichAnnotationPreview {
        RichAnnotationPreview {
            image: sunipdf_core::RenderedPage {
                page_index: 0,
                width: 4,
                height: 2,
                rgba: vec![255; 32],
            },
            scale: 1.0,
            region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
            index: 7,
            prepared: None,
        }
    }

    #[test]
    fn rich_draft_requires_preview_preserves_errors_and_rejects_late_tokens() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_rich_annotations(&ctx);
        app.annotate.adv.rich.as_mut().unwrap().text = "Keep this draft".into();
        assert!(app.has_unsaved_changes() && app.dialog_open());
        for command in [
            sunipdf_core::Command::Save,
            sunipdf_core::Command::OpenDocument,
            sunipdf_core::Command::Quit,
        ] {
            app.execute(&ctx, command);
        }
        app.apply_rich_draft();
        assert!(requests.try_recv().is_err());
        app.request_rich_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("rich preview"),
        };
        app.on_rich_preview(&ctx, epoch, generation.wrapping_sub(1), Ok(preview()));
        assert!(app.annotate.adv.rich.as_ref().unwrap().busy);
        app.on_rich_preview(
            &ctx,
            epoch,
            generation,
            Err(EngineError::MissingGlyphs("🚀".into())),
        );
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(!draft.busy && !draft.prepared && draft.error.is_some());
        assert_eq!(draft.text, "Keep this draft");
        app.request_rich_preview();
        let generation = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation { generation, .. } => generation,
            _ => panic!("retry"),
        };
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        app.apply_rich_draft();
        assert!(
            matches!(requests.recv().unwrap(),Request::Edit(EditOp::CommitRichAnnotationPreview {epoch:sent_epoch,generation:sent_generation}) if sent_epoch==epoch && sent_generation==generation)
        );
        assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
        assert_eq!(app.annotate.adv.reselect, Some((0, 7)));
        app.rich_annotation_failed(&EngineError::Other("stale candidate".into()));
        app.worker.finish_mutation();
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(!draft.applying && !draft.prepared && draft.error.is_some());
        assert_eq!(draft.text, "Keep this draft");
        app.view.bump_epoch();
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        app.request_rich_preview();
        assert!(requests.try_recv().is_err());
        assert!(app.annotate.adv.rich.as_ref().unwrap().error.is_some());
        app.annotate.adv.invalidate();
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        assert!(app.annotate.adv.rich.is_none());
    }

    #[test]
    fn rich_dialog_english_french_escape_discards_candidate_without_applying() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            app.open_rich_annotations(&ctx);
            app.annotate.adv.rich.as_mut().unwrap().text = "Draft".into();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 900.0),
                )),
                ..Default::default()
            };
            let mut found = false;
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |_| app.rich_annotation_dialog(&ctx));
                if let Some(tree) = output.platform_output.accesskit_update.as_ref() {
                    found |= tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(tr(app.lang(), "paragraph.preview")));
                }
                output.textures_delta.clear();
            }
            assert!(found);
            let mut input = input;
            input.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let mut output = ctx.run_ui(input, |_| app.rich_annotation_dialog(&ctx));
            output.textures_delta.clear();
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests.try_iter().all(|request| matches!(
                request,
                Request::DiscardRichAnnotationPreview { .. } | Request::SystemFonts
            )));
        }
    }

    #[test]
    fn free_text_bridge_preserves_input_and_existing_font_replacement_is_explicit() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.tool = crate::annotate::Tool::FreeText;
        app.annotate.adv.free_text = Some(crate::annot_adv::FreeTextPrompt {
            page: 0,
            rect: PointRect {
                left: 40.0,
                bottom: 100.0,
                right: 280.0,
                top: 220.0,
            },
            index: Some(2),
            text: "Existing text 中文".into(),
            size: 18.0,
            rgb: [20, 30, 40],
            fill: true,
            fill_rgb: [240, 240, 220],
            border: 1.0,
            focus: false,
            epoch: app.view.epoch,
            tool: app.annotate.tool,
            workspace: app.annotate.workspace,
            pending: false,
        });
        app.open_rich_from_free_text_prompt(&ctx);
        assert!(app.annotate.adv.free_text.is_none());
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert_eq!(draft.text, "Existing text 中文");
        assert_eq!(draft.target, Some(2));
        assert!(draft.replace && !draft.font_chosen);
        assert!(draft.operation().is_none());
        app.request_rich_preview();
        assert!(
            requests.try_recv().is_err(),
            "existing annotation needs explicit font choice"
        );
        app.annotate.adv.rich.as_mut().unwrap().font_chosen = true;
        assert!(matches!(
            app.annotate.adv.rich.as_ref().unwrap().operation(),
            Some(RichAnnotation::FreeText { index: Some(2), .. })
        ));
        assert!(points("1,2\n3,4", 512).is_some());
        assert!(points("NaN,2", 512).is_none());
        assert!(points("1,2,3", 512).is_none());
    }
}
