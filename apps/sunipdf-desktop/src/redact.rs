//! Redaction drafts, persisted unapplied PDF marks and an exact verified before/after candidate.
//! Applying commits the preview bytes through the worker in one Undo step; preparation is read-only.

use crate::annotate::{Drag, Tool};
use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::dialog;
use crate::icons::glyph;
use crate::theme;
use crate::worker::Request;
use egui::{Key, Modifiers, Stroke};
use std::collections::BTreeSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use sunipdf_core::redact::{RedactAppearance, RedactOverlay, RedactionPreview};
use sunipdf_core::{Command, PointRect, RedactMark, RedactOptions, RedactReport};
use sunipdf_i18n::{tr, tr_args};

/// Côté minimal d'une zone tracée (points) : en dessous, c'est un clic.
const MIN_SIDE_PT: f32 = 2.0;

pub struct RedactState {
    /// Zones non appliquées (espace d'affichage, comme l'API du moteur).
    pub marks: Vec<RedactMark>,
    pub search_open: bool,
    pub search_query: String,
    pub search_match_case: bool,
    /// Dernier texte marqué : recherché à nouveau après application (contrôle).
    pub last_query: Option<(String, bool)>,
    pub confirm_open: bool,
    pub rasterize: bool,
    pub clean_hidden: bool,
    pub applying: bool,
    /// Enregistrement demandé avec des zones non appliquées : `Some(ask_path)`.
    pub save_prompt: Option<bool>,
    /// « Enregistrer sans caviarder » choisi : le prochain enregistrement passe la garde.
    pub save_confirmed: bool,
    pub fill_rgb: [u8; 3],
    pub overlay: String,
    pub overlay_rgb: [u8; 3],
    pub overlay_size: f32,
    generation: u64,
    preview_epoch: u64,
    preview_page: Option<usize>,
    preview: Option<(egui::TextureHandle, egui::TextureHandle)>,
    preview_report: Option<RedactReport>,
    preview_busy: bool,
    preview_cancel: Option<Arc<AtomicBool>>,
    preview_error: Option<String>,
    persisting: bool,
}

impl Default for RedactState {
    fn default() -> Self {
        Self {
            marks: Vec::new(),
            search_open: false,
            search_query: String::new(),
            search_match_case: false,
            last_query: None,
            confirm_open: false,
            rasterize: false,
            clean_hidden: true,
            applying: false,
            save_prompt: None,
            save_confirmed: false,
            fill_rgb: [0, 0, 0],
            overlay: String::new(),
            overlay_rgb: [255, 255, 255],
            overlay_size: 10.0,
            generation: 0,
            preview_epoch: 0,
            preview_page: None,
            preview: None,
            preview_report: None,
            preview_busy: false,
            preview_cancel: None,
            preview_error: None,
            persisting: false,
        }
    }
}

impl Drop for RedactState {
    fn drop(&mut self) {
        if let Some(cancel) = &self.preview_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl RedactState {
    fn appearance(&self) -> RedactAppearance {
        RedactAppearance {
            fill_rgb: (self.fill_rgb[0], self.fill_rgb[1], self.fill_rgb[2]),
            overlay: (!self.overlay.trim().is_empty()).then(|| RedactOverlay {
                text: self.overlay.clone(),
                style: sunipdf_core::TextStyle {
                    size_pt: self.overlay_size,
                    rgb: (
                        self.overlay_rgb[0],
                        self.overlay_rgb[1],
                        self.overlay_rgb[2],
                    ),
                    ..Default::default()
                },
            }),
        }
    }

    fn invalidate_preview(&mut self) {
        if let Some(cancel) = self.preview_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.generation = self.generation.wrapping_add(1);
        self.preview = None;
        self.preview_report = None;
        self.preview_busy = false;
    }
    pub fn dialog_open(&self) -> bool {
        self.search_open || self.confirm_open || self.save_prompt.is_some()
    }

    pub fn page_count(&self) -> usize {
        self.marks
            .iter()
            .map(|m| m.page)
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Ajoute des zones sans doublon exact ; renvoie le nombre ajouté.
    pub fn add(&mut self, marks: impl IntoIterator<Item = RedactMark>) -> usize {
        let before = self.marks.len();
        for m in marks {
            if !self.marks.contains(&m) {
                self.marks.push(m);
            }
        }
        if self.marks.len() != before {
            self.invalidate_preview();
        }
        self.marks.len() - before
    }

    /// Retire la zone la plus récente qui contient le point ; `true` si une zone a été retirée.
    pub fn remove_at(&mut self, page: usize, pt: (f32, f32)) -> bool {
        let hit = self.marks.iter().rposition(|m| {
            m.page == page
                && (m.rect.left..=m.rect.right).contains(&pt.0)
                && (m.rect.bottom..=m.rect.top).contains(&pt.1)
        });
        if let Some(i) = hit {
            self.marks.remove(i);
            self.invalidate_preview();
            true
        } else {
            false
        }
    }
}

/// Rectangle tracé (affichage), borné à la page ; `None` s'il est trop petit.
pub fn drawn_rect(rect: PointRect, page_w: f32, page_h: f32) -> Option<PointRect> {
    let r = PointRect {
        left: rect.left.clamp(0.0, page_w),
        right: rect.right.clamp(0.0, page_w),
        bottom: rect.bottom.clamp(0.0, page_h),
        top: rect.top.clamp(0.0, page_h),
    };
    (r.right - r.left >= MIN_SIDE_PT && r.top - r.bottom >= MIN_SIDE_PT).then_some(r)
}

impl SuniApp {
    fn discard_redaction_preview(&mut self) {
        let r = &mut self.annotate.redact;
        self.worker.send(Request::DiscardRedactionPreview {
            epoch: r.preview_epoch,
            generation: r.generation,
        });
        r.invalidate_preview();
    }

    pub(crate) fn on_persisted_redaction_marks(
        &mut self,
        epoch: u64,
        result: Result<Vec<RedactMark>, sunipdf_core::EngineError>,
    ) {
        if epoch != self.view.epoch {
            return;
        }
        match result {
            Ok(marks) => {
                self.annotate.redact.add(marks);
            }
            Err(error) => self.status_msg = Some(error.to_string()),
        }
    }

    pub(crate) fn on_redaction_preview(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<RedactionPreview, sunipdf_core::EngineError>,
    ) {
        let r = &mut self.annotate.redact;
        if epoch != self.view.epoch
            || epoch != r.preview_epoch
            || generation != r.generation
            || !r.preview_busy
            || !r.confirm_open
        {
            return;
        }
        r.preview_busy = false;
        match result {
            Ok(preview) => {
                let texture = |name, image: &sunipdf_core::RenderedPage| {
                    ctx.load_texture(
                        name,
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    )
                };
                r.preview = Some((
                    texture("redaction-before", &preview.before),
                    texture("redaction-after", &preview.after),
                ));
                r.preview_report = Some(preview.report);
                r.preview_error = None;
            }
            Err(error) => r.preview_error = Some(error.to_string()),
        }
    }

    pub(crate) fn persist_redact_marks(&mut self, ctx: &egui::Context) {
        if !self.redaction_idle() || self.annotate.redact.marks.is_empty() {
            return;
        }
        self.discard_redaction_preview();
        let r = &mut self.annotate.redact;
        r.confirm_open = false;
        r.persisting = true;
        self.worker.send(Request::PersistRedactionMarks {
            marks: r.marks.clone(),
            appearance: r.appearance(),
            author: None,
        });
        ctx.request_repaint();
    }

    fn redaction_idle(&self) -> bool {
        self.document.is_some()
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.resuming.is_none()
            && self.tabs.suspending.is_none()
            && !self.processing.busy
            && !self.office.busy
            && !self.saving
            && !self.printing.busy
            && !self.ocr.busy
            && !self.sign.busy
            && !self.certificate.busy
    }

    fn request_redaction_preview(&mut self) {
        if !self.redaction_idle() {
            return;
        }
        self.discard_redaction_preview();
        let r = &mut self.annotate.redact;
        let Some(page) = r
            .preview_page
            .filter(|page| r.marks.iter().any(|m| m.page == *page))
            .or_else(|| r.marks.first().map(|m| m.page))
        else {
            return;
        };
        r.preview_page = Some(page);
        r.preview_epoch = self.view.epoch;
        r.preview_busy = true;
        let cancel = Arc::new(AtomicBool::new(false));
        r.preview_cancel = Some(cancel.clone());
        r.preview_error = None;
        self.worker.send(Request::PreviewRedaction {
            marks: r.marks.clone(),
            options: RedactOptions {
                rasterize_pages: r.rasterize,
                clean_hidden_data: r.clean_hidden,
                ..Default::default()
            },
            appearance: r.appearance(),
            page,
            epoch: r.preview_epoch,
            generation: r.generation,
            cancel,
        });
    }
    pub(crate) fn redact_status(&mut self) {
        let r = &self.annotate.redact;
        self.status_msg = Some(if r.marks.is_empty() {
            tr(self.lang(), "security.redact.bar.empty").to_string()
        } else {
            tr_args(
                self.lang(),
                "security.redact.bar.count",
                &[
                    ("count", &r.marks.len().to_string()),
                    ("pages", &r.page_count().to_string()),
                ],
            )
        });
    }

    /// Zones de la page : noir translucide à liseré rouge, opaque au survol (aperçu du résultat).
    pub fn paint_redact_marks(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let to_screen = |r: &PointRect| {
            egui::Rect::from_min_max(
                page_rect.min + egui::vec2(r.left * k, (page_h_pt - r.top) * k),
                page_rect.min + egui::vec2(r.right * k, (page_h_pt - r.bottom) * k),
            )
        };
        let hover = painter.ctx().pointer_hover_pos();
        let paint = |r: &PointRect| {
            let rect = to_screen(r);
            let alpha = if hover.is_some_and(|p| rect.contains(p)) {
                255
            } else {
                110
            };
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(alpha));
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.5, theme::ERROR),
                egui::StrokeKind::Outside,
            );
        };
        for m in self.annotate.redact.marks.iter().filter(|m| m.page == page) {
            paint(&m.rect);
        }
        if self.annotate.tool == Tool::Redact {
            if let Some(d) = self.annotate.drag.filter(|d| d.page == page) {
                paint(&d.rect());
            }
        }
    }

    /// Fin d'un tracé : nouvelle zone (aucune requête au moteur).
    pub fn redact_drag_finished(&mut self, drag: Drag) {
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(drag.page).copied())
        else {
            return;
        };
        if let Some(rect) = drawn_rect(drag.rect(), size.width_pt, size.height_pt) {
            self.annotate.redact.add([RedactMark {
                page: drag.page,
                rect,
            }]);
            self.redact_status();
        }
    }

    pub fn redact_click(&mut self, page: usize, pt: (f32, f32)) {
        if self.annotate.redact.remove_at(page, pt) {
            self.redact_status();
            if !self.annotate.redact.marks.is_empty() {
                self.status_msg = Some(self.t("security.redact.status.removed").to_string());
            }
        }
    }

    pub fn redact_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::RedactSearch => {
                if self.annotate.tool != Tool::Redact {
                    self.execute(ctx, Command::ToolRedact);
                }
                let r = &mut self.annotate.redact;
                if r.search_query.is_empty() {
                    r.search_query = self.find.query.clone();
                }
                r.search_open = true;
            }
            Command::RedactClear => {
                self.discard_redaction_preview();
                self.annotate.redact.marks.clear();
                self.status_msg = Some(self.t("security.redact.status.all_removed").to_string());
            }
            Command::RedactApply => {
                if self.annotate.redact.marks.is_empty() {
                    self.error = Some("error.redact_no_marks");
                } else if !self.annotate.redact.applying {
                    self.discard_redaction_preview();
                    self.annotate.redact.preview_epoch = self.view.epoch;
                    self.annotate.redact.confirm_open = true;
                }
            }
            _ => {}
        }
    }

    /// Le document a changé (rotation, page supprimée, annulation…) : les zones ne désignent plus
    /// forcément le même contenu. Règle simple et sûre : elles sont effacées.
    pub fn redact_on_document_changed(&mut self) {
        self.discard_redaction_preview();
        self.annotate.redact.persisting = false;
        if !self.annotate.redact.marks.is_empty() {
            self.annotate.redact.marks.clear();
            self.status_msg = Some(self.t("security.redact.status.cleared").to_string());
        }
    }

    pub fn on_redact_matches(&mut self, query: String, verify: bool, marks: Vec<RedactMark>) {
        let lang = self.lang();
        let pages = marks.iter().map(|m| m.page).collect::<BTreeSet<_>>();
        if verify {
            if !marks.is_empty() {
                self.status_msg = Some(tr_args(
                    lang,
                    "security.redact.status.remaining",
                    &[("count", &marks.len().to_string()), ("query", &query)],
                ));
                self.annotate.redact.add(marks);
            }
            return;
        }
        let first = marks.first().map(|m| m.page);
        let added = self.annotate.redact.add(marks);
        self.status_msg = Some(if added == 0 {
            tr(lang, "security.redact.status.none").to_string()
        } else {
            tr_args(
                lang,
                "security.redact.status.marked",
                &[
                    ("count", &added.to_string()),
                    ("pages", &pages.len().to_string()),
                ],
            )
        });
        if let Some(page) = first {
            self.go_to_page(page);
        }
    }

    /// Bilan d'un caviardage réussi ; `DocumentChanged` suit et affiche le message.
    pub fn on_redacted(&mut self, report: RedactReport) {
        let r = &mut self.annotate.redact;
        r.applying = false;
        r.marks.clear();
        self.tools.running = None;
        self.edit.pending_status = Some(tr_args(
            self.lang(),
            "security.redact.status.done",
            &[
                ("marks", &report.marks.to_string()),
                ("objects", &report.objects_removed.to_string()),
                (
                    "annotations",
                    &(report.annotations_removed + report.fields_removed).to_string(),
                ),
            ],
        ));
        if let Some((query, match_case)) = self.annotate.redact.last_query.take() {
            self.worker.send(Request::RedactSearch {
                query,
                match_case,
                verify: true,
            });
        }
    }

    /// Échec du moteur pendant l'application : les zones restent pour réessayer (page en image).
    pub fn redact_failed(&mut self) {
        self.annotate.redact.persisting = false;
        if self.annotate.redact.applying {
            self.annotate.redact.applying = false;
            self.tools.running = None;
            self.annotate.redact.confirm_open = true;
            self.annotate.redact.preview_error =
                Some(tr(self.lang(), "security.redact.preview.failed").to_string());
            self.discard_redaction_preview();
        }
    }

    /// Garde d'enregistrement : `true` si l'enregistrement doit attendre la réponse de l'utilisateur.
    pub fn redact_blocks_save(&mut self, ask_path: bool) -> bool {
        let r = &mut self.annotate.redact;
        if r.marks.is_empty() || std::mem::take(&mut r.save_confirmed) {
            return false;
        }
        r.save_prompt = Some(ask_path);
        true
    }

    pub fn redact_dialogs(&mut self, ctx: &egui::Context) {
        self.redact_search_dialog(ctx);
        self.redact_confirm_dialog(ctx);
        self.redact_unapplied_dialog(ctx);
    }

    fn redact_search_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.redact.search_open {
            return;
        }
        let lang = self.lang();
        let spec = dialog::Spec::new(
            "redact-search",
            tr(lang, "security.redact.search.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "security.redact.search.subtitle"))
        .icon(glyph::FIND, "🔍")
        .width(460.0);
        let (mut submit, mut cancel) = (false, false);
        let r = &mut self.annotate.redact;
        dialog::show(
            ctx,
            spec,
            |ui| {
                dialog::field_label(ui, tr(lang, "security.redact.search.label"));
                let field = ui.add(
                    egui::TextEdit::singleline(&mut r.search_query).desired_width(f32::INFINITY),
                );
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                ui.checkbox(&mut r.search_match_case, tr(lang, "find.match_case"));
                ui.add_space(4.0);
                ui.add(egui::Label::new(tr(lang, "security.redact.search.note")).wrap());
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "security.redact.search.ok"))
                    || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        let r = &mut self.annotate.redact;
        if submit && !r.search_query.trim().is_empty() {
            r.search_open = false;
            let query = r.search_query.trim().to_string();
            r.last_query = Some((query.clone(), r.search_match_case));
            self.worker.send(Request::RedactSearch {
                query,
                match_case: r.search_match_case,
                verify: false,
            });
        } else if cancel {
            r.search_open = false;
        }
    }

    fn redact_confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.redact.confirm_open {
            return;
        }
        if self.annotate.redact.preview_epoch != self.view.epoch
            && (self.annotate.redact.preview.is_some() || self.annotate.redact.preview_busy)
        {
            self.discard_redaction_preview();
            self.annotate.redact.preview_error =
                Some(tr(self.lang(), "security.redact.preview.stale").to_string());
        }
        let idle = self.redaction_idle();
        let lang = self.lang();
        let r = &self.annotate.redact;
        let spec = dialog::Spec::new(
            "redact-confirm",
            tr(lang, "security.redact.confirm.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr_args(
            lang,
            "security.redact.confirm.subtitle",
            &[
                ("count", &r.marks.len().to_string()),
                ("pages", &r.page_count().to_string()),
            ],
        ))
        .icon(glyph::REDACT, "█")
        .warning()
        .width(720.0);
        let (mut submit, mut cancel, mut preview, mut persist, mut changed) =
            (false, false, false, false, false);
        let r = &mut self.annotate.redact;
        let ready =
            r.preview.is_some() && !r.preview_busy && r.preview_epoch == self.view.epoch && idle;
        let can_preview = idle && !r.preview_busy;
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.add(egui::Label::new(tr(lang, "security.redact.confirm.text")).wrap());
                ui.add_enabled_ui(!r.preview_busy, |ui| {
                    changed |= ui
                        .checkbox(
                            &mut r.rasterize,
                            tr(lang, "security.redact.confirm.rasterize"),
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut r.clean_hidden,
                            tr(lang, "security.redact.confirm.clean"),
                        )
                        .changed();
                    if !r.clean_hidden {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(tr(
                                    lang,
                                    "security.redact.confirm.keep_warning",
                                ))
                                .color(egui::Color32::from_rgb(220, 70, 70)),
                            )
                            .wrap(),
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "security.redact.fill"));
                        changed |= ui.color_edit_button_srgb(&mut r.fill_rgb).changed();
                        ui.label(tr(lang, "security.redact.overlay.color"));
                        changed |= ui.color_edit_button_srgb(&mut r.overlay_rgb).changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut r.overlay_size)
                                    .range(4.0..=200.0)
                                    .suffix(" pt"),
                            )
                            .changed();
                    });
                    ui.label(tr(lang, "security.redact.overlay"));
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut r.overlay)
                                .char_limit(256)
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr(lang, "security.redact.overlay.note")).small(),
                        )
                        .wrap(),
                    );
                    let pages = r.marks.iter().map(|m| m.page).collect::<BTreeSet<_>>();
                    let mut page = r
                        .preview_page
                        .filter(|p| pages.contains(p))
                        .or_else(|| pages.first().copied())
                        .unwrap_or(0);
                    let before = page;
                    egui::ComboBox::from_id_salt("redaction-preview-page")
                        .selected_text(format!(
                            "{} {}",
                            tr(lang, "security.redact.preview.page"),
                            page + 1
                        ))
                        .show_ui(ui, |ui| {
                            for p in pages {
                                ui.selectable_value(&mut page, p, (p + 1).to_string());
                            }
                        });
                    r.preview_page = Some(page);
                    changed |= page != before;
                });
                if r.preview_busy {
                    ui.spinner();
                    ui.label(tr(lang, "security.redact.preview.running"));
                }
                if let Some((before, after)) = &r.preview {
                    ui.columns(2, |columns| {
                        for (column, (label, texture)) in columns.iter_mut().zip([
                            ("security.redact.preview.before", before),
                            ("security.redact.preview.after", after),
                        ]) {
                            column.label(tr(lang, label));
                            let scale = (column.available_width() / texture.size_vec2().x)
                                .min(260.0 / texture.size_vec2().y);
                            column.add(egui::Image::new((
                                texture.id(),
                                texture.size_vec2() * scale,
                            )));
                        }
                    });
                }
                if let Some(report) = r.preview_report {
                    ui.add(
                        egui::Label::new(tr_args(
                            lang,
                            "security.redact.preview.report",
                            &[
                                ("native", &report.native_text_runs.to_string()),
                                ("bitmap", &report.bitmap_objects.to_string()),
                            ],
                        ))
                        .wrap(),
                    );
                    if report.bitmap_objects > 0 {
                        ui.add(
                            egui::Label::new(tr(lang, "security.redact.preview.fallback")).wrap(),
                        );
                    }
                }
                if let Some(error) = &r.preview_error {
                    ui.colored_label(egui::Color32::from_rgb(220, 70, 70), error);
                }
                ui.add(egui::Label::new(tr(lang, "security.redact.confirm.note")).wrap());
            },
            |ui| {
                preview = ui
                    .add_enabled_ui(can_preview, |ui| {
                        dialog::primary(ui, tr(lang, "security.redact.preview.action"))
                    })
                    .inner;
                persist = ui
                    .add_enabled_ui(can_preview, |ui| {
                        dialog::secondary(ui, tr(lang, "security.redact.persist"))
                    })
                    .inner;
                submit = ui
                    .add_enabled_ui(ready, |ui| {
                        dialog::danger(ui, tr(lang, "security.redact.confirm.ok"))
                    })
                    .inner
                    || (ready && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)));
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if changed {
            self.discard_redaction_preview();
        }
        if cancel {
            self.discard_redaction_preview();
            self.annotate.redact.confirm_open = false;
        } else if persist {
            self.persist_redact_marks(ctx);
        } else if preview {
            self.request_redaction_preview();
        } else if submit && !changed {
            let r = &mut self.annotate.redact;
            r.confirm_open = false;
            r.applying = true;
            self.tools.running = Some("security.redact.running");
            self.worker.send(Request::CommitRedactionPreview {
                epoch: r.preview_epoch,
                generation: r.generation,
            });
        }
    }
    fn redact_unapplied_dialog(&mut self, ctx: &egui::Context) {
        let Some(ask_path) = self.annotate.redact.save_prompt else {
            return;
        };
        let lang = self.lang();
        let spec = dialog::Spec::new(
            "redact-unapplied",
            tr(lang, "security.redact.unapplied.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .icon(glyph::WARNING, "⚠")
        .warning()
        .width(480.0);
        let count = self.annotate.redact.marks.len().to_string();
        let (mut apply, mut save, mut cancel) = (false, false, false);
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.add(
                    egui::Label::new(tr_args(
                        lang,
                        "security.redact.unapplied.text",
                        &[("count", &count)],
                    ))
                    .wrap(),
                );
            },
            |ui| {
                apply = dialog::primary(ui, tr(lang, "security.redact.unapplied.apply"));
                save = dialog::secondary(ui, tr(lang, "security.redact.unapplied.save"));
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if apply || save || cancel {
            self.annotate.redact.save_prompt = None;
        }
        if apply {
            self.cancel_after_save();
            self.annotate.redact.confirm_open = true;
        } else if save {
            self.annotate.redact.save_confirmed = true;
            self.save_document(ask_path);
        } else if cancel {
            self.cancel_after_save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(l: f32, b: f32, r: f32, t: f32) -> PointRect {
        PointRect {
            left: l,
            bottom: b,
            right: r,
            top: t,
        }
    }

    #[test]
    fn confirmation_displays_hidden_data_warning_only_when_cleanup_is_disabled() {
        for lang in sunipdf_i18n::Language::ALL {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.settings.set_language(lang);
            let ctx = egui::Context::default();
            app.has_icons = crate::icons::install(&ctx);
            app.annotate.redact.confirm_open = true;
            app.annotate.redact.marks.push(RedactMark {
                page: 0,
                rect: rect(10.0, 10.0, 50.0, 50.0),
            });
            for clean in [true, false] {
                app.annotate.redact.clean_hidden = clean;
                let warning = tr(lang, "security.redact.confirm.keep_warning");
                assert_ne!(warning, "security.redact.confirm.keep_warning");
                let mut output =
                    ctx.run_ui(Default::default(), |_| app.redact_confirm_dialog(&ctx));
                output.textures_delta.clear();
                output = ctx.run_ui(Default::default(), |_| app.redact_confirm_dialog(&ctx));
                let shown = output.shapes.iter().any(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => text.galley.job.text == warning,
                    _ => false,
                });
                output.textures_delta.clear();
                assert_eq!(
                    shown,
                    !clean,
                    "cleanup warning visibility in {}",
                    lang.code()
                );
            }
        }
    }

    #[test]
    fn preview_cancel_and_stale_responses_preserve_the_draft() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.redact.marks.push(RedactMark {
            page: 0,
            rect: rect(10.0, 10.0, 100.0, 40.0),
        });
        app.annotate.redact.confirm_open = true;
        app.annotate.redact.preview_epoch = app.view.epoch;
        app.annotate.redact.preview_busy = true;
        app.annotate.redact.overlay = "KEEP DRAFT".into();
        let cancel = Arc::new(AtomicBool::new(false));
        app.annotate.redact.preview_cancel = Some(cancel.clone());
        let generation = app.annotate.redact.generation;
        app.on_redaction_preview(
            &ctx,
            app.view.epoch,
            generation.wrapping_add(1),
            Err(sunipdf_core::EngineError::Cancelled),
        );
        assert!(app.annotate.redact.preview_busy);
        app.discard_redaction_preview();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(matches!(
            requests.recv().unwrap(),
            Request::DiscardRedactionPreview { .. }
        ));
        assert_eq!(app.annotate.redact.overlay, "KEEP DRAFT");
        assert_eq!(app.annotate.redact.marks.len(), 1);
        app.on_redaction_preview(
            &ctx,
            app.view.epoch,
            generation,
            Err(sunipdf_core::EngineError::RedactionIncomplete),
        );
        assert!(!app.annotate.redact.preview_busy && app.annotate.redact.preview.is_none());
    }

    #[test]
    fn marks_are_added_once_and_removed_by_click() {
        let mut s = RedactState::default();
        assert!(s.clean_hidden, "informations cachées supprimées par défaut");
        let a = RedactMark {
            page: 0,
            rect: rect(10.0, 10.0, 50.0, 50.0),
        };
        let b = RedactMark {
            page: 2,
            rect: rect(30.0, 30.0, 80.0, 80.0),
        };
        let c = RedactMark {
            page: 0,
            rect: rect(20.0, 20.0, 60.0, 60.0),
        };
        assert_eq!(s.add([a, b, a]), 2);
        assert_eq!(s.add([c]), 1);
        assert_eq!(s.page_count(), 2);
        // Le clic retire la zone la plus récente qui contient le point, sur sa page seulement.
        assert!(s.remove_at(0, (40.0, 40.0)));
        assert_eq!(s.marks, vec![a, b]);
        assert!(!s.remove_at(1, (40.0, 40.0)));
        assert!(!s.remove_at(0, (5.0, 5.0)));
        assert!(!s.dialog_open());
    }

    #[test]
    fn drawn_rect_is_clamped_and_tiny_ones_ignored() {
        assert_eq!(
            drawn_rect(rect(-5.0, 10.0, 700.0, 20.0), 612.0, 792.0),
            Some(rect(0.0, 10.0, 612.0, 20.0))
        );
        assert!(drawn_rect(rect(10.0, 10.0, 11.0, 40.0), 612.0, 792.0).is_none());
        assert!(drawn_rect(rect(10.0, 10.0, 40.0, 11.5), 612.0, 792.0).is_none());
        assert!(drawn_rect(rect(700.0, 10.0, 720.0, 40.0), 612.0, 792.0).is_none());
    }
}
