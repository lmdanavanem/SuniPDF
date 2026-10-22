//! Local revision alignment. The engine worker owns rendering and checked exports.

use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    revision_overlay::{
        self as core, OverlayAlignment, OverlayOptions, OverlayPasswords, OverlayPreview,
        OverlayTransform,
    },
    EngineError,
};
use sunipdf_i18n::{tr, tr_args};
use zeroize::Zeroizing;

pub enum Job {
    Preview {
        base: PathBuf,
        revision: PathBuf,
        options: OverlayOptions,
        passwords: [Zeroizing<String>; 2],
        cancel: Arc<AtomicBool>,
    },
    Export {
        preview: Arc<OverlayPreview>,
        path: PathBuf,
        json: bool,
        cancel: Arc<AtomicBool>,
    },
}

pub enum Outcome {
    Preview(Arc<OverlayPreview>),
    Exported(PathBuf),
}

pub fn run_job(engine: &sunipdf_core::Engine, job: Job) -> Result<Outcome, EngineError> {
    match job {
        Job::Preview {
            base,
            revision,
            options,
            passwords,
            cancel,
        } => core::prepare_overlay(
            engine,
            &base,
            &revision,
            options,
            OverlayPasswords {
                base: (!passwords[0].is_empty()).then_some(passwords[0].as_str()),
                revision: (!passwords[1].is_empty()).then_some(passwords[1].as_str()),
            },
            &cancel,
        )
        .map(|preview| Outcome::Preview(Arc::new(preview))),
        Job::Export {
            preview,
            path,
            json,
            cancel,
        } => {
            let bytes = if json {
                serde_json::to_vec_pretty(&preview.export_json(&cancel)?)
                    .map_err(|error| EngineError::Other(error.to_string()))?
            } else {
                preview.export_png(&cancel)?
            };
            preview.revalidate_sources(&cancel)?;
            sunipdf_core::write_new_copy(&path, &bytes)?;
            Ok(Outcome::Exported(path))
        }
    }
}

struct SourceImage {
    texture: egui::TextureHandle,
    size_pt: [f64; 2],
    sha256: String,
}

struct Images {
    sources: [SourceImage; 2],
    combined: egui::TextureHandle,
}

pub struct OverlayState {
    pub open: bool,
    pub busy: bool,
    pub cancel: Arc<AtomicBool>,
    paths: [Option<PathBuf>; 2],
    pages: [usize; 2],
    passwords: [Zeroizing<String>; 2],
    two_points: bool,
    anchors: [Vec<[f64; 2]>; 2],
    transform: OverlayTransform,
    opacity: f32,
    long_side: u32,
    generation: u64,
    pending: Option<u64>,
    preview: Option<Arc<OverlayPreview>>,
    images: Option<Images>,
    dirty: bool,
    error: Option<String>,
    message: Option<String>,
}

impl Default for OverlayState {
    fn default() -> Self {
        let options = OverlayOptions::default();
        Self {
            open: false,
            busy: false,
            cancel: Arc::new(AtomicBool::new(false)),
            paths: [None, None],
            pages: [1, 1],
            passwords: Default::default(),
            two_points: false,
            anchors: Default::default(),
            transform: OverlayTransform::default(),
            opacity: options.opacity,
            long_side: options.long_side,
            generation: 0,
            pending: None,
            preview: None,
            images: None,
            dirty: true,
            error: None,
            message: None,
        }
    }
}

impl OverlayState {
    fn changed(&mut self) {
        self.dirty = true;
        self.error = None;
        self.message = None;
    }

    fn clear_sources(&mut self) {
        self.preview = None;
        self.images = None;
        self.reset_alignment();
    }

    fn reset_alignment(&mut self) {
        self.anchors = Default::default();
        self.transform = OverlayTransform::default();
        self.changed();
    }

    fn options(&self, allow_source_preview: bool) -> Result<OverlayOptions, EngineError> {
        let alignment = if self.two_points {
            if self.anchors.iter().all(|points| points.len() == 2) {
                OverlayAlignment::TwoPoints {
                    base: [self.anchors[0][0], self.anchors[0][1]],
                    revision: [self.anchors[1][0], self.anchors[1][1]],
                }
            } else if allow_source_preview {
                OverlayAlignment::default()
            } else {
                return Err(EngineError::Other(
                    "Choose two corresponding points on each page".into(),
                ));
            }
        } else {
            OverlayAlignment::Manual {
                transform: self.transform,
            }
        };
        let options = OverlayOptions {
            base_page: self.pages[0],
            revision_page: self.pages[1],
            alignment,
            opacity: self.opacity,
            long_side: self.long_side,
        };
        options.validate()?;
        Ok(options)
    }

    fn add_anchor(&mut self, source: usize, point: [f64; 2]) {
        if self.busy || !self.two_points || source >= 2 {
            return;
        }
        let Some(images) = &self.images else { return };
        let size = images.sources[source].size_pt;
        if point
            .iter()
            .enumerate()
            .any(|(i, value)| !value.is_finite() || *value < 0.0 || *value > size[i])
        {
            return;
        }
        let points = &mut self.anchors[source];
        if points.len() == 2 {
            return;
        }
        if points
            .last()
            .is_some_and(|last| (last[0] - point[0]).hypot(last[1] - point[1]) < 0.001)
        {
            return;
        }
        points.push(point);
        self.changed();
    }

    fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(self.generation);
        self.busy = true;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.error = None;
        self.message = None;
        self.generation
    }

    fn cancel_job(&mut self) {
        if self.busy {
            self.cancel.store(true, Ordering::Relaxed);
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub(crate) fn close(&mut self) {
        if self.busy {
            self.cancel_job();
            return;
        }
        self.open = false;
        self.clear_sources();
        self.passwords = Default::default();
    }

    fn export_ready(&self) -> bool {
        self.open && !self.busy && !self.dirty && self.preview.is_some()
    }
}

impl SuniApp {
    pub(crate) fn open_revision_overlay(&mut self) {
        if self.overlay.busy || self.review.busy || self.ink_blocks_action() {
            return;
        }
        self.review.open = false;
        self.overlay.open = true;
        self.overlay.error = None;
    }

    pub(crate) fn overlay_from_comparison(
        &mut self,
        base: Option<PathBuf>,
        revision: Option<PathBuf>,
        pages: [usize; 2],
    ) {
        self.open_revision_overlay();
        if self.overlay.open && !self.overlay.busy {
            self.overlay.paths = [base, revision];
            self.overlay.pages = pages;
            self.overlay.passwords = Default::default();
            self.overlay.clear_sources();
        }
    }

    fn start_overlay_preview(&mut self) {
        if self.overlay.busy || !self.overlay.open || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        let [Some(base), Some(revision)] = self.overlay.paths.clone() else {
            return;
        };
        let options = match self.overlay.options(true) {
            Ok(options) => options,
            Err(error) => {
                self.overlay.error = Some(error.to_string());
                return;
            }
        };
        let generation = self.overlay.begin();
        self.overlay.dirty = true;
        self.worker.send(Request::RevisionOverlay {
            generation,
            job: Job::Preview {
                base,
                revision,
                options,
                passwords: self.overlay.passwords.clone(),
                cancel: self.overlay.cancel.clone(),
            },
        });
    }

    fn export_overlay_to(&mut self, path: PathBuf, json: bool) {
        if !self.overlay.export_ready() {
            return;
        }
        let preview = self.overlay.preview.as_ref().unwrap().clone();
        let generation = self.overlay.begin();
        self.worker.send(Request::RevisionOverlay {
            generation,
            job: Job::Export {
                preview,
                path,
                json,
                cancel: self.overlay.cancel.clone(),
            },
        });
    }

    pub(crate) fn on_revision_overlay(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if self.overlay.pending != Some(generation) {
            return;
        }
        self.overlay.pending = None;
        self.overlay.busy = false;
        if !self.overlay.open {
            return;
        }
        // An export already committed before cancellation must still acknowledge its output.
        if let Ok(Outcome::Exported(path)) = &result {
            self.overlay.message = Some(tr_args(
                self.lang(),
                "overlay.exported",
                &[("name", &crate::app::file_name(path))],
            ));
            return;
        }
        if self.overlay.generation != generation || self.overlay.cancel.load(Ordering::Relaxed) {
            self.overlay.message = Some(tr(self.lang(), "review.cancelled").into());
            return;
        }
        match result {
            Ok(Outcome::Preview(preview)) => {
                let report = preview.report();
                let load = |name: &str, page: &sunipdf_core::RenderedPage| {
                    ctx.load_texture(
                        format!("revision-overlay-{name}-{generation}"),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [page.width as usize, page.height as usize],
                            &page.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    )
                };
                let source = |name: &str,
                              image: &sunipdf_core::RenderedPage|
                 -> Result<SourceImage, EngineError> {
                    let size_pt = [
                        report[name]["display_width_pt"].as_f64(),
                        report[name]["display_height_pt"].as_f64(),
                    ];
                    let [Some(width), Some(height)] = size_pt else {
                        return Err(EngineError::Other("Missing overlay page geometry".into()));
                    };
                    let Some(sha256) = report[name]["source_sha256"].as_str() else {
                        return Err(EngineError::Other(
                            "Missing overlay source fingerprint".into(),
                        ));
                    };
                    Ok(SourceImage {
                        texture: load(name, image),
                        size_pt: [width, height],
                        sha256: sha256.into(),
                    })
                };
                let images = (|| {
                    Ok::<_, EngineError>(Images {
                        sources: [
                            source("base", preview.base_image())?,
                            source("revision", preview.revision_image())?,
                        ],
                        combined: load("combined", preview.image()),
                    })
                })();
                match images {
                    Ok(images) => {
                        let source_changed = self.overlay.images.as_ref().is_some_and(|previous| {
                            previous
                                .sources
                                .iter()
                                .zip(&images.sources)
                                .any(|(old, new)| old.sha256 != new.sha256)
                        });
                        if source_changed {
                            self.overlay.clear_sources();
                            self.overlay.message =
                                Some(tr(self.lang(), "overlay.sources_changed").into());
                            return;
                        }
                        self.overlay.dirty = match self.overlay.options(false) {
                            Ok(options) => &options != preview.options(),
                            Err(_) => true,
                        };
                        self.overlay.transform = preview.transform();
                        self.overlay.images = Some(images);
                        self.overlay.preview = Some(preview);
                    }
                    Err(error) => self.overlay.error = Some(error.to_string()),
                }
            }
            Ok(Outcome::Exported(_)) => unreachable!(),
            Err(error) => {
                self.overlay.error = Some(format!(
                    "{} ({error})",
                    tr(self.lang(), crate::app::error_key(&error))
                ));
            }
        }
    }

    pub(crate) fn revision_overlay_dialog(&mut self, ctx: &egui::Context) {
        if !self.overlay.open {
            return;
        }
        let lang = self.lang();
        let busy = self.overlay.busy;
        let can_preview = !busy
            && matches!(self.engine, EngineState::Ready)
            && self.overlay.paths.iter().all(Option::is_some);
        let can_export = self.overlay.export_ready();
        let (mut pick, mut preview, mut export, mut close, mut cancel) =
            (None, false, None, false, false);
        let mut clicked_anchor = None;
        let mut changed = false;
        let mut sources_changed = false;
        dialog::show(
            ctx,
            Spec::new(
                "revision_overlay",
                tr(lang, "overlay.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(960.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "overlay.saved_files"));
                ui.small(tr(lang, "overlay.limits"));
                ui.add_enabled_ui(!busy, |ui| {
                    ui.columns(2, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            column.horizontal(|ui| {
                                if ui
                                    .button(tr(
                                        lang,
                                        if index == 0 {
                                            "overlay.base"
                                        } else {
                                            "overlay.revision"
                                        },
                                    ))
                                    .clicked()
                                {
                                    pick = Some(index);
                                }
                                if let Some(path) = &self.overlay.paths[index] {
                                    ui.add(
                                        egui::Label::new(crate::app::file_name(path)).truncate(),
                                    )
                                    .on_hover_text(path.display().to_string());
                                }
                            });
                            column.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "overlay.page"));
                                sources_changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut self.overlay.pages[index])
                                            .range(1..=65535),
                                    )
                                    .changed();
                                ui.label(tr(lang, "overlay.password"));
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(
                                            &mut *self.overlay.passwords[index],
                                        )
                                        .password(true)
                                        .char_limit(256)
                                        .desired_width(140.0),
                                    )
                                    .changed();
                            });
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        changed |= ui
                            .radio_value(
                                &mut self.overlay.two_points,
                                false,
                                tr(lang, "overlay.manual"),
                            )
                            .changed();
                        changed |= ui
                            .radio_value(
                                &mut self.overlay.two_points,
                                true,
                                tr(lang, "overlay.two_points"),
                            )
                            .changed();
                        if ui.button(tr(lang, "overlay.reset")).clicked() {
                            self.overlay.reset_alignment();
                        }
                        ui.label(tr(lang, "overlay.opacity"));
                        changed |= ui
                            .add(egui::Slider::new(&mut self.overlay.opacity, 0.0..=1.0))
                            .changed();
                    });
                    if self.overlay.two_points {
                        ui.label(tr(lang, "overlay.points_hint"));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for (index, key) in ["overlay.offset_x", "overlay.offset_y"]
                                .into_iter()
                                .enumerate()
                            {
                                ui.label(tr(lang, key));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut self.overlay.transform.translation[index],
                                        )
                                        .range(-1_000_000.0..=1_000_000.0)
                                        .speed(0.25)
                                        .suffix(" pt"),
                                    )
                                    .changed();
                            }
                            ui.label(tr(lang, "overlay.rotation"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(
                                        &mut self.overlay.transform.rotation_degrees,
                                    )
                                    .range(-360.0..=360.0)
                                    .speed(0.1)
                                    .suffix("°"),
                                )
                                .changed();
                            ui.label(tr(lang, "overlay.scale"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.overlay.transform.scale)
                                        .range(0.01..=100.0)
                                        .speed(0.001)
                                        .max_decimals(6),
                                )
                                .changed();
                        });
                        ui.small(tr(lang, "overlay.coordinates"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "overlay.resolution"));
                        egui::ComboBox::from_id_salt("overlay_resolution")
                            .selected_text(format!("{} px", self.overlay.long_side))
                            .show_ui(ui, |ui| {
                                for size in [512, 1024, 1536, 2048] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.overlay.long_side,
                                            size,
                                            format!("{size} px"),
                                        )
                                        .changed();
                                }
                            });
                        ui.colored_label(
                            egui::Color32::from_rgb(210, 0, 180),
                            tr(lang, "overlay.base_color"),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 150, 170),
                            tr(lang, "overlay.revision_color"),
                        );
                    });
                });
                // Changes take effect before drawing, so an old source image cannot accept a new-page anchor.
                if sources_changed {
                    self.overlay.clear_sources();
                }
                if changed {
                    self.overlay.changed();
                }
                if busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr(
                            lang,
                            if self.overlay.cancel.load(Ordering::Relaxed) {
                                "ocr.cancelling"
                            } else {
                                "overlay.working"
                            },
                        ));
                    });
                }
                if let Some(images) = &self.overlay.images {
                    ui.separator();
                    ui.columns(2, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            let source = &images.sources[index];
                            column.strong(tr(
                                lang,
                                if index == 0 {
                                    "review.original"
                                } else {
                                    "review.revised"
                                },
                            ));
                            let response = image(
                                column,
                                &source.texture,
                                240.0,
                                !busy && self.overlay.two_points,
                            );
                            if !busy && self.overlay.two_points && response.clicked() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    clicked_anchor = Some((
                                        index,
                                        point_from_image(response.rect, pos, source.size_pt),
                                    ));
                                }
                            }
                            paint_anchors(
                                column.painter(),
                                response.rect,
                                source.size_pt,
                                &self.overlay.anchors[index],
                            );
                            if self.overlay.two_points {
                                column.small(tr_args(
                                    lang,
                                    "overlay.points_selected",
                                    &[("count", &self.overlay.anchors[index].len().to_string())],
                                ));
                            }
                        }
                    });
                    ui.separator();
                    ui.strong(tr(lang, "overlay.result"));
                    if self.overlay.dirty {
                        ui.label(tr(lang, "overlay.refresh_hint"));
                    }
                    image(ui, &images.combined, 380.0, false);
                    ui.small(tr_args(
                        lang,
                        "overlay.canvas",
                        &[
                            ("width", &images.combined.size()[0].to_string()),
                            ("height", &images.combined.size()[1].to_string()),
                        ],
                    ));
                    ui.collapsing(tr(lang, "overlay.fingerprints"), |ui| {
                        for (index, source) in images.sources.iter().enumerate() {
                            ui.label(tr(
                                lang,
                                if index == 0 {
                                    "review.original"
                                } else {
                                    "review.revised"
                                },
                            ));
                            ui.monospace(&source.sha256);
                        }
                        ui.small(tr(lang, "overlay.fingerprint_hint"));
                    });
                } else {
                    ui.label(tr(lang, "overlay.load_hint"));
                }
                if let Some(error) = &self.overlay.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = &self.overlay.message {
                    ui.label(message);
                }
            },
            |ui| {
                if busy {
                    cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|input| input.key_pressed(egui::Key::Escape));
                } else {
                    preview = ui
                        .add_enabled(can_preview, egui::Button::new(tr(lang, "overlay.preview")))
                        .clicked();
                    if ui
                        .add_enabled(
                            can_export,
                            egui::Button::new(tr(lang, "overlay.export_png")),
                        )
                        .clicked()
                    {
                        export = Some(false);
                    }
                    if ui
                        .add_enabled(
                            can_export,
                            egui::Button::new(tr(lang, "overlay.export_json")),
                        )
                        .clicked()
                    {
                        export = Some(true);
                    }
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|input| input.key_pressed(egui::Key::Escape));
                }
            },
        );
        if let Some(index) = pick {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_file()
            {
                self.overlay.paths[index] = Some(path);
                self.overlay.passwords[index] = Default::default();
                self.overlay.clear_sources();
            }
        }
        if let Some((index, point)) = clicked_anchor {
            self.overlay.add_anchor(index, point);
        }
        if preview {
            self.start_overlay_preview();
        }
        if let Some(json) = export {
            let extension = if json { "json" } else { "png" };
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(extension, &[extension])
                .set_file_name(format!("revision-overlay.{extension}"))
                .save_file()
            {
                self.export_overlay_to(path, json);
            }
        }
        if cancel {
            self.overlay.cancel_job();
        }
        if close {
            self.overlay.close();
        }
    }
}

fn image(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    max_height: f32,
    clickable: bool,
) -> egui::Response {
    let size = texture.size_vec2();
    let ratio = (ui.available_width() / size.x)
        .min(max_height / size.y)
        .min(1.0);
    ui.add(
        egui::Image::new(texture)
            .fit_to_exact_size(size * ratio)
            .sense(if clickable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            }),
    )
}

fn point_from_image(rect: egui::Rect, pos: egui::Pos2, size: [f64; 2]) -> [f64; 2] {
    [
        f64::from(((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0)) * size[0],
        f64::from(((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0)) * size[1],
    ]
}

fn paint_anchors(painter: &egui::Painter, rect: egui::Rect, size: [f64; 2], points: &[[f64; 2]]) {
    for (index, point) in points.iter().enumerate() {
        let pos = rect.min
            + egui::vec2(
                (point[0] / size[0]) as f32 * rect.width(),
                (point[1] / size[1]) as f32 * rect.height(),
            );
        painter.circle_filled(pos, 5.0, egui::Color32::from_rgb(220, 120, 0));
        painter.circle_stroke(pos, 5.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
        painter.text(
            pos + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            (index + 1).to_string(),
            egui::FontId::proportional(14.0),
            egui::Color32::BLACK,
        );
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app::tests::test_app;
    use egui::{Event, FullOutput, Modifiers, PointerButton, Pos2, Rect};
    use sunipdf_core::Command;
    use sunipdf_i18n::Language;

    struct Controls {
        ctx: egui::Context,
        time: f64,
        size: egui::Vec2,
    }
    impl Controls {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
            Self {
                ctx,
                time: 1.0,
                size: egui::vec2(1500.0, 1500.0),
            }
        }
        fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.local_review_dialog(ui.ctx());
                    app.revision_overlay_dialog(ui.ctx());
                },
            );
            output.textures_delta.clear();
            output
        }
        fn click_at(&mut self, app: &mut SuniApp, pos: Pos2) {
            self.time += 0.6;
            for pressed in [true, false] {
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            self.frame(app, vec![]);
        }
        fn click(&mut self, app: &mut SuniApp, key: &str) {
            fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(value) if value.galley.text() == text => {
                        Some(value.pos + value.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(values) => {
                        values.iter().rev().find_map(|value| find(value, text))
                    }
                    _ => None,
                }
            }
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let text = tr(app.lang(), key);
            let pos = output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| find(&shape.shape, text))
                .unwrap_or_else(|| panic!("visible overlay control: {text}"));
            self.click_at(app, pos);
        }
        fn source_point(&mut self, app: &mut SuniApp, index: usize, fraction: [f32; 2]) {
            fn find(shape: &egui::Shape, texture: egui::TextureId) -> Option<Rect> {
                match shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                        let first = mesh.vertices.first()?.pos;
                        let (mut min, mut max) = (first, first);
                        for vertex in &mesh.vertices {
                            min.x = min.x.min(vertex.pos.x);
                            min.y = min.y.min(vertex.pos.y);
                            max.x = max.x.max(vertex.pos.x);
                            max.y = max.y.max(vertex.pos.y);
                        }
                        Some(Rect::from_min_max(min, max))
                    }
                    egui::Shape::Vec(values) => {
                        values.iter().find_map(|value| find(value, texture))
                    }
                    _ => None,
                }
            }
            let texture = app.overlay.images.as_ref().unwrap().sources[index]
                .texture
                .id();
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let rect = output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, texture))
                .expect("visible source image");
            self.click_at(
                app,
                rect.min + rect.size() * egui::vec2(fraction[0], fraction[1]),
            );
        }
        fn escape(&mut self, app: &mut SuniApp) {
            self.frame(
                app,
                vec![Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }

    #[test]
    fn overlay_entry_is_visible_without_document_and_compare_opens_it() {
        use crate::workspace::{Entry, Workspace};
        let entry = &crate::workspace::TABLE[Workspace::RevisionOverlay as usize];
        assert_eq!(entry.entry, Some(Entry::Direct(Command::RevisionOverlay)));
        assert!(!Command::RevisionOverlay.info().needs_document);
        assert_eq!(
            Command::RevisionOverlay.info().menu,
            sunipdf_core::Menu::File
        );
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            app.settings.set_language(lang);
            app.engine = EngineState::Ready;
            app.document = None;
            let mut controls = Controls::new();
            app.execute(&controls.ctx, Command::CompareFiles);
            controls.click(&mut app, "overlay.title");
            assert!(app.overlay.open && !app.review.open && app.dialog_open());
            assert!(app.navigation_blocked());
            controls.click(&mut app, "overlay.preview");
            assert!(requests.try_recv().is_err(), "Preview requires two files");
            controls.click(&mut app, "overlay.two_points");
            assert!(app.overlay.two_points);
            controls.click(&mut app, "overlay.manual");
            assert!(!app.overlay.two_points);
            app.overlay.transform.translation = [12.0, 7.0];
            app.overlay.transform.rotation_degrees = 20.0;
            app.overlay.transform.scale = 2.0;
            controls.click(&mut app, "overlay.reset");
            assert_eq!(app.overlay.transform, OverlayTransform::default());
            app.overlay.passwords[0] = Zeroizing::new("temporary".into());
            controls.escape(&mut app);
            assert!(!app.overlay.open && app.overlay.passwords[0].is_empty());
        }
    }

    #[test]
    fn footer_stays_visible_at_minimum_size_and_quit_releases_the_modal() {
        fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(value) if value.galley.text() == text => {
                    Some(value.pos + value.galley.size() / 2.0)
                }
                egui::Shape::Vec(values) => values.iter().find_map(|value| find(value, text)),
                _ => None,
            }
        }
        for lang in Language::ALL {
            let (mut app, _, _) = test_app();
            let mut controls = Controls::new();
            controls.size = egui::vec2(720.0, 480.0);
            app.settings.set_language(lang);
            app.open_revision_overlay();
            controls.frame(&mut app, vec![]);
            let output = controls.frame(&mut app, vec![]);
            for key in [
                "overlay.preview",
                "overlay.export_png",
                "overlay.export_json",
                "dialog.close",
            ] {
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|shape| find(&shape.shape, tr(lang, key)))
                    .expect("visible footer label");
                assert!(
                    Rect::from_min_size(Pos2::ZERO, controls.size).contains(pos),
                    "footer inside the window: {key}"
                );
            }
            app.dirty = true;
            app.execute(&controls.ctx, Command::Quit);
            assert!(
                !app.overlay.open,
                "an idle overlay cannot hide the unsaved-document prompt"
            );
            assert!(app.dialog_open() && app.has_unsaved_changes());
            app.overlay.open = true;
            app.overlay.busy = true;
            app.execute(&controls.ctx, Command::Quit);
            assert!(app.overlay.cancel.load(Ordering::Relaxed));
            assert!(
                app.overlay.busy,
                "quit waits for the worker acknowledgement"
            );
        }
    }

    #[test]
    fn coordinates_use_displayed_top_left_and_invalid_alignment_is_refused() {
        let rect = Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(220.0, 130.0));
        assert_eq!(point_from_image(rect, rect.min, [600.0, 300.0]), [0.0, 0.0]);
        assert_eq!(
            point_from_image(rect, rect.max, [600.0, 300.0]),
            [600.0, 300.0]
        );
        let mut state = OverlayState {
            two_points: true,
            ..Default::default()
        };
        assert!(state.options(false).is_err());
        assert!(matches!(
            state.options(true).unwrap().alignment,
            OverlayAlignment::Manual { .. }
        ));
        state.anchors = [vec![[1.0, 1.0], [1.0, 1.0]], vec![[2.0, 2.0], [3.0, 3.0]]];
        assert!(state.options(false).is_err());
        state.two_points = false;
        state.transform.scale = f64::NAN;
        assert!(state.options(true).is_err());
    }

    /// Uses a real preview from the single shared engine integration test.
    pub(crate) fn verify_prepared_controls(preview: &Arc<OverlayPreview>) {
        verify_replaced_sources_invalidate_alignment(preview);
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            let mut controls = Controls::new();
            app.settings.set_language(lang);
            app.engine = EngineState::Ready;
            app.execute(&controls.ctx, Command::RevisionOverlay);
            let report = preview.report();
            app.overlay.paths = ["base", "revision"]
                .map(|key| Some(PathBuf::from(report[key]["path"].as_str().unwrap())));
            app.overlay.long_side = preview.options().long_side;
            controls.click(&mut app, "overlay.preview");
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job: Job::Preview { options, .. },
                } => {
                    assert_eq!(&options, preview.options());
                    generation
                }
                _ => panic!("overlay preview request"),
            };
            app.on_revision_overlay(
                &controls.ctx,
                generation + 1,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.busy && app.overlay.images.is_none());
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.export_ready());
            let texture = app.overlay.images.as_ref().unwrap().combined.id();
            controls.frame(&mut app, vec![]);
            controls.frame(&mut app, vec![]);
            assert_eq!(app.overlay.images.as_ref().unwrap().combined.id(), texture);
            assert!(
                requests.try_recv().is_err(),
                "idle frames do not render or recompose"
            );
            controls.click(&mut app, "overlay.two_points");
            for index in 0..2 {
                controls.source_point(&mut app, index, [0.2, 0.25]);
                controls.source_point(&mut app, index, [0.7, 0.65]);
            }
            assert_eq!(app.overlay.anchors[0].len(), 2);
            assert_eq!(app.overlay.anchors[1].len(), 2);
            assert!(matches!(
                app.overlay.options(false).unwrap().alignment,
                OverlayAlignment::TwoPoints { .. }
            ));
            assert!(!app.overlay.export_ready());
            controls.click(&mut app, "overlay.preview");
            let (generation, cancel) = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job:
                        Job::Preview {
                            options, cancel, ..
                        },
                } => {
                    assert!(matches!(
                        options.alignment,
                        OverlayAlignment::TwoPoints { .. }
                    ));
                    (generation, cancel)
                }
                _ => panic!("aligned preview request"),
            };
            controls.escape(&mut app);
            assert!(cancel.load(Ordering::Relaxed) && app.overlay.busy);
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(!app.overlay.busy && app.overlay.dirty);
            assert_eq!(
                app.overlay.images.as_ref().unwrap().combined.id(),
                texture,
                "cancelled results cannot replace textures"
            );
            controls.click(&mut app, "overlay.reset");
            assert!(app.overlay.anchors.iter().all(Vec::is_empty));
            controls.click(&mut app, "overlay.manual");
            controls.click(&mut app, "overlay.preview");
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay { generation, .. } => generation,
                _ => panic!("fresh manual preview"),
            };
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.export_ready());
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("overlay.png");
            app.export_overlay_to(path.clone(), false);
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job:
                        Job::Export {
                            path: sent,
                            json: false,
                            ..
                        },
                } => {
                    assert_eq!(sent, path);
                    generation
                }
                _ => panic!("checked export request"),
            };
            assert!(!app.overlay.export_ready());
            controls.escape(&mut app);
            app.on_revision_overlay(&controls.ctx, generation, Ok(Outcome::Exported(path)));
            assert!(
                app.overlay
                    .message
                    .as_ref()
                    .unwrap()
                    .contains("overlay.png"),
                "an export committed before cancellation is acknowledged"
            );
            controls.click(&mut app, "dialog.close");
            assert!(
                !app.overlay.open && app.overlay.images.is_none() && app.overlay.preview.is_none()
            );
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.images.is_none());
        }
    }

    fn verify_replaced_sources_invalidate_alignment(preview: &Arc<OverlayPreview>) {
        for changed_source in 0..2 {
            for two_points in [false, true] {
                let (mut app, requests, _) = test_app();
                let ctx = egui::Context::default();
                app.overlay.open = true;
                app.overlay.long_side = preview.options().long_side;
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.export_ready());
                app.overlay.two_points = two_points;
                app.overlay.anchors = [
                    vec![[1.0, 2.0], [20.0, 30.0]],
                    vec![[3.0, 4.0], [40.0, 50.0]],
                ];
                app.overlay.transform.translation = [15.0, 25.0];
                app.overlay.images.as_mut().unwrap().sources[changed_source].sha256 =
                    "previous-source-bytes".into();
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.images.is_none() && app.overlay.preview.is_none());
                assert!(app.overlay.anchors.iter().all(Vec::is_empty));
                assert_eq!(app.overlay.transform, OverlayTransform::default());
                assert!(app.overlay.dirty && !app.overlay.export_ready());
                app.export_overlay_to(PathBuf::from("must-not-export.png"), false);
                assert!(requests.try_recv().is_err());
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.images.is_some());
                assert_eq!(app.overlay.export_ready(), !two_points);
                assert!(app.overlay.anchors.iter().all(Vec::is_empty));
            }
        }
    }
}
