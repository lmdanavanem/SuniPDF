//! Explicit review of raster difference proposals; opaque candidates remain in the worker.

use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    revision_outlines::{
        self as core, OutlineClassification, OutlineOptions, OutlineReview, OutlineSelection,
    },
    revision_overlay::{OverlayAlignment, OverlayTransform},
    Engine, EngineError, RenderedPage,
};
use sunipdf_i18n::{tr, tr_args, Language};
use zeroize::Zeroizing;

type Passwords = [Zeroizing<String>; 2];
pub enum Job {
    Inspect {
        paths: [PathBuf; 2],
        options: OutlineOptions,
        passwords: Passwords,
        cancel: Arc<AtomicBool>,
    },
    Prepare {
        paths: [PathBuf; 2],
        review: OutlineReview,
        passwords: Passwords,
        cancel: Arc<AtomicBool>,
    },
    Save {
        token: u64,
        output: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Discard,
}
pub enum Outcome {
    Inspected(core::OutlineInspection),
    Preview {
        token: u64,
        image: RenderedPage,
        report: core::OutlineReport,
    },
    Saved(core::OutlineReport),
    Discarded,
}
#[derive(Default)]
pub struct WorkerCache {
    prepared: Option<(u64, core::PreparedRevisionOutlines)>,
    paths: Option<[PathBuf; 2]>,
}
fn passwords(values: &Passwords) -> core::OutlinePasswords<'_> {
    core::OutlinePasswords {
        old: (!values[0].is_empty()).then_some(values[0].as_str()),
        new: (!values[1].is_empty()).then_some(values[1].as_str()),
    }
}
fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
impl WorkerCache {
    pub fn touches_modified(&self, job: &Job, path: &Path) -> bool {
        let sources = match job {
            Job::Prepare { paths, .. } => Some(paths),
            Job::Save { .. } => self.paths.as_ref(),
            _ => None,
        };
        sources.is_some_and(|s| s.iter().any(|p| same_path(p, path)))
    }
    pub fn run(
        &mut self,
        engine: &Engine,
        generation: u64,
        job: Job,
        mut progress: impl FnMut(core::OutlineProgress),
    ) -> Result<Outcome, EngineError> {
        let result = (|| match job {
            Job::Inspect {
                paths,
                options,
                passwords: secrets,
                cancel,
            } => {
                self.prepared = None;
                self.paths = None;
                core::inspect_revision_outlines(
                    engine,
                    &paths[0],
                    &paths[1],
                    &options,
                    passwords(&secrets),
                    &cancel,
                    &mut progress,
                )
                .map(Outcome::Inspected)
            }
            Job::Prepare {
                paths,
                review,
                passwords: secrets,
                cancel,
            } => {
                self.prepared = None;
                self.paths = None;
                let prepared = core::prepare_revision_outlines(
                    engine,
                    &paths[0],
                    &paths[1],
                    &review,
                    passwords(&secrets),
                    &cancel,
                    &mut progress,
                )?;
                let image =
                    prepared.render_preview(engine, passwords(&secrets).new, 1024, &cancel)?;
                let report = prepared.report().clone();
                self.prepared = Some((generation, prepared));
                self.paths = Some(paths);
                Ok(Outcome::Preview {
                    token: generation,
                    image,
                    report,
                })
            }
            Job::Save {
                token,
                output,
                cancel,
            } => {
                let (id, prepared) = self.prepared.take().ok_or_else(|| {
                    EngineError::Other("Prepared revision outlines expired; preview again".into())
                })?;
                self.paths = None;
                if token != id {
                    return Err(EngineError::Other(
                        "Prepared revision outline token changed".into(),
                    ));
                }
                core::publish_revision_outlines(prepared, &output, &cancel).map(Outcome::Saved)
            }
            Job::Discard => {
                self.prepared = None;
                self.paths = None;
                Ok(Outcome::Discarded)
            }
        })();
        if result.is_err() {
            self.prepared = None;
            self.paths = None;
        }
        result
    }
}

#[derive(Clone, Debug)]
struct Decision {
    selected: bool,
    classification: OutlineClassification,
    confirmed: bool,
}
impl Default for Decision {
    fn default() -> Self {
        Self {
            selected: false,
            classification: OutlineClassification::Uncertain,
            confirmed: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Inspect,
    Preview,
    Save,
}

pub struct OutlineState {
    pub open: bool,
    generation: u64,
    pending: Option<(u64, Pending)>,
    cancel: Arc<AtomicBool>,
    paths: [Option<PathBuf>; 2],
    passwords: Passwords,
    options: OutlineOptions,
    anchors: bool,
    points: [[[f64; 2]; 2]; 2],
    transform: OverlayTransform,
    saved_files_confirmed: bool,
    inspection: Option<core::OutlineInspection>,
    decisions: Vec<Decision>,
    selected_region: usize,
    textures: [Option<egui::TextureHandle>; 4],
    image_tab: usize,
    token: Option<u64>,
    reviewed: bool,
    report: Option<core::OutlineReport>,
    progress: Option<core::OutlineProgress>,
    message: Option<String>,
    error: Option<String>,
}
impl Default for OutlineState {
    fn default() -> Self {
        Self {
            open: false,
            generation: 0,
            pending: None,
            cancel: Arc::new(AtomicBool::new(false)),
            paths: [None, None],
            passwords: Default::default(),
            options: Default::default(),
            anchors: false,
            points: [[[0.0, 0.0], [100.0, 0.0]]; 2],
            transform: Default::default(),
            saved_files_confirmed: false,
            inspection: None,
            decisions: Vec::new(),
            selected_region: 0,
            textures: Default::default(),
            image_tab: 2,
            token: None,
            reviewed: false,
            report: None,
            progress: None,
            message: None,
            error: None,
        }
    }
}
impl OutlineState {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn begin(&mut self, kind: Pending) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some((self.generation, kind));
        self.cancel = Arc::new(AtomicBool::new(false));
        self.progress = None;
        self.error = None;
        self.message = None;
        self.generation
    }
    fn invalidate(&mut self, sources: bool) {
        self.token = None;
        self.reviewed = false;
        self.report = None;
        self.textures[3] = None;
        self.message = None;
        self.error = None;
        if sources {
            self.inspection = None;
            self.decisions.clear();
            self.textures = Default::default();
            self.selected_region = 0;
        }
    }
    fn cancel_pending(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.passwords = Default::default();
        // Publication can already have committed. Keep the generation until its result arrives.
        if self.pending.is_some_and(|(_, p)| p == Pending::Save) {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.invalidate(false);
        self.progress = None;
    }
    fn request(&self) -> Option<OutlineReview> {
        let inspection = self.inspection.as_ref()?;
        if inspection.regions.len() != self.decisions.len() {
            return None;
        }
        let selections = inspection
            .regions
            .iter()
            .zip(&self.decisions)
            .filter(|(_, d)| d.selected)
            .map(|(r, d)| OutlineSelection {
                id: r.id.clone(),
                fingerprint: r.fingerprint.clone(),
                classification: d.classification,
                confirmed: d.confirmed,
            })
            .collect();
        Some(OutlineReview {
            version: 1,
            old_sha256: inspection.old_sha256.clone(),
            new_sha256: inspection.new_sha256.clone(),
            options: inspection.options.clone(),
            selections,
        })
    }
    fn can_prepare(&self) -> bool {
        !self.busy()
            && self.saved_files_confirmed
            && self
                .request()
                .is_some_and(|r| core::encode_review(&r).is_ok())
    }
    fn can_save(&self) -> bool {
        !self.busy()
            && self.token.is_some()
            && self.reviewed
            && self
                .report
                .as_ref()
                .is_some_and(core::OutlineReport::can_publish)
    }
}

fn texture(ctx: &egui::Context, name: &str, image: &RenderedPage) -> egui::TextureHandle {
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width as usize, image.height as usize],
            &image.rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}
fn class_key(class: OutlineClassification) -> &'static str {
    match class {
        OutlineClassification::ContentChange => "outlines.content",
        OutlineClassification::RenderingNoise => "outlines.noise",
        OutlineClassification::Uncertain => "outlines.uncertain",
    }
}

impl SuniApp {
    pub(crate) fn cancel_revision_outlines_for_exit(&mut self) {
        self.revision_outlines.cancel.store(true, Ordering::Relaxed);
    }
    pub(crate) fn open_revision_outlines(&mut self) {
        if self.revision_outlines.busy()
            || self.ink_blocks_action()
            || self.worker.has_pending_mutations()
            || self.saving
        {
            return;
        }
        self.revision_outlines.invalidate(true);
        self.revision_outlines.open = true;
        self.revision_outlines.saved_files_confirmed = false;
        self.discard_revision_outlines();
    }
    pub(crate) fn close_revision_outlines(&mut self) {
        if self.revision_outlines.busy() {
            self.cancel_revision_outlines_for_exit();
            return;
        }
        self.revision_outlines.cancel_pending();
        self.revision_outlines.open = false;
        self.revision_outlines.invalidate(true);
        self.discard_revision_outlines();
    }
    fn discard_revision_outlines(&mut self) {
        self.worker.send(Request::RevisionOutlines {
            generation: self.revision_outlines.generation,
            job: Job::Discard,
        });
    }
    fn outlines_have_dirty_source(&self) -> bool {
        self.revision_outlines.paths.iter().flatten().any(|p| {
            (self.dirty
                && self
                    .document
                    .as_ref()
                    .is_some_and(|d| same_path(p, &d.info.path)))
                || self.tabs.tabs.iter().any(|t| {
                    (t.dirty || t.ui.as_ref().is_some_and(|u| u.dirty)) && same_path(p, &t.path)
                })
        })
    }
    fn inspect_revision_outlines(&mut self) {
        if self.revision_outlines.busy()
            || !self.revision_outlines.saved_files_confirmed
            || !matches!(self.engine, EngineState::Ready)
        {
            return;
        }
        let [Some(old), Some(new)] = self.revision_outlines.paths.clone() else {
            return;
        };
        self.revision_outlines.invalidate(true);
        let generation = self.revision_outlines.begin(Pending::Inspect);
        self.worker.send(Request::RevisionOutlines {
            generation,
            job: Job::Inspect {
                paths: [old, new],
                options: self.revision_outlines.options.clone(),
                passwords: std::mem::take(&mut self.revision_outlines.passwords),
                cancel: self.revision_outlines.cancel.clone(),
            },
        });
    }
    fn preview_revision_outlines(&mut self) {
        if !self.revision_outlines.can_prepare() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        if self.outlines_have_dirty_source() {
            self.revision_outlines.error = Some(tr(self.lang(), "transfer.dirty_source").into());
            return;
        }
        let Some(review) = self.revision_outlines.request() else {
            return;
        };
        let [Some(old), Some(new)] = self.revision_outlines.paths.clone() else {
            return;
        };
        self.revision_outlines.invalidate(false);
        let generation = self.revision_outlines.begin(Pending::Preview);
        self.worker.send(Request::RevisionOutlines {
            generation,
            job: Job::Prepare {
                paths: [old, new],
                review,
                passwords: std::mem::take(&mut self.revision_outlines.passwords),
                cancel: self.revision_outlines.cancel.clone(),
            },
        });
    }
    fn save_revision_outlines(&mut self, output: PathBuf) {
        if !self.revision_outlines.can_save() {
            return;
        }
        if self.outlines_have_dirty_source() {
            self.revision_outlines.error = Some(tr(self.lang(), "transfer.dirty_source").into());
            return;
        }
        let token = self.revision_outlines.token.expect("checked candidate");
        let generation = self.revision_outlines.begin(Pending::Save);
        self.worker.send(Request::RevisionOutlines {
            generation,
            job: Job::Save {
                token,
                output,
                cancel: self.revision_outlines.cancel.clone(),
            },
        });
    }
    pub(crate) fn on_revision_outlines_progress(
        &mut self,
        generation: u64,
        progress: core::OutlineProgress,
    ) {
        if self.revision_outlines.pending.map(|p| p.0) == Some(generation) {
            self.revision_outlines.progress = Some(progress);
        }
    }
    pub(crate) fn on_revision_outlines(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if self.revision_outlines.pending.map(|p| p.0) != Some(generation) {
            return;
        }
        self.revision_outlines.pending = None;
        self.revision_outlines.progress = None;
        match result {
            Ok(Outcome::Inspected(mut inspection)) => {
                if let Some(images) = inspection.images.take() {
                    self.revision_outlines.textures[0] =
                        Some(texture(ctx, "outline_old", &images.old));
                    self.revision_outlines.textures[1] =
                        Some(texture(ctx, "outline_new", &images.new));
                    self.revision_outlines.textures[2] =
                        Some(texture(ctx, "outline_difference", &images.difference));
                }
                self.revision_outlines.decisions =
                    vec![Decision::default(); inspection.regions.len()];
                self.revision_outlines.inspection = Some(inspection);
                self.revision_outlines.selected_region = 0;
                self.revision_outlines.image_tab = 2;
            }
            Ok(Outcome::Preview {
                token,
                image,
                report,
            }) => {
                self.revision_outlines.token = Some(token);
                self.revision_outlines.textures[3] =
                    Some(texture(ctx, "outline_candidate", &image));
                self.revision_outlines.report = Some(report);
                self.revision_outlines.reviewed = false;
                self.revision_outlines.image_tab = 3;
            }
            Ok(Outcome::Saved(report)) => {
                self.revision_outlines.token = None;
                self.revision_outlines.reviewed = false;
                self.revision_outlines.passwords = Default::default();
                self.revision_outlines.message = Some(tr_args(
                    self.lang(),
                    "transfer.saved",
                    &[(
                        "path",
                        &report
                            .output_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                    )],
                ));
                self.revision_outlines.report = Some(report);
            }
            Ok(Outcome::Discarded) => {}
            Err(error) => {
                self.revision_outlines.invalidate(false);
                self.revision_outlines.error = Some(format!(
                    "{} ({error})",
                    tr(self.lang(), crate::app::error_key(&error))
                ));
                self.discard_revision_outlines();
            }
        }
    }

    pub(crate) fn revision_outlines_dialog(&mut self, ctx: &egui::Context) {
        if !self.revision_outlines.open {
            return;
        }
        let lang = self.lang();
        let busy = self.revision_outlines.busy();
        let can_inspect = !busy
            && self.revision_outlines.saved_files_confirmed
            && self.revision_outlines.paths.iter().all(Option::is_some)
            && self.revision_outlines.options.validate().is_ok()
            && matches!(self.engine, EngineState::Ready);
        let can_preview = self.revision_outlines.can_prepare();
        let can_save = self.revision_outlines.can_save();
        let (mut pick, mut inspect, mut preview, mut save, mut close, mut cancel) =
            (None, false, false, false, false, false);
        let (mut changed, mut source_changed) = (false, false);
        dialog::show(
            ctx,
            Spec::new(
                "revision_outlines",
                tr(lang, "outlines.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(920.0_f32.min((ctx.content_rect().width() - 60.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "outlines.limits"));
                ui.add_enabled_ui(!busy, |ui| {
                    ui.columns(2, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            column.strong(tr(
                                lang,
                                if index == 0 {
                                    "transfer.old"
                                } else {
                                    "transfer.new"
                                },
                            ));
                            if column.button(tr(lang, "transfer.choose")).clicked() {
                                pick = Some(index);
                            }
                            if let Some(path) = &self.revision_outlines.paths[index] {
                                column
                                    .add(egui::Label::new(crate::app::file_name(path)).truncate())
                                    .on_hover_text(path.display().to_string());
                            }
                            column.label(tr(lang, "overlay.password"));
                            column.add(
                                egui::TextEdit::singleline(
                                    &mut *self.revision_outlines.passwords[index],
                                )
                                .password(true)
                                .char_limit(256)
                                .desired_width(column.available_width()),
                            );
                        }
                    });
                    ui.small(tr(lang, "transfer.password_hint"));
                    source_changed |= ui
                        .checkbox(
                            &mut self.revision_outlines.saved_files_confirmed,
                            tr(lang, "outlines.saved_ack"),
                        )
                        .changed();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "transfer.old_page"));
                        source_changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.revision_outlines.options.old_page)
                                    .range(1..=65535),
                            )
                            .changed();
                        ui.label(tr(lang, "transfer.new_page"));
                        source_changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.revision_outlines.options.new_page)
                                    .range(1..=65535),
                            )
                            .changed();
                    });
                    ui.collapsing(tr(lang, "outlines.settings"), |ui| {
                        source_changed |= option_controls(ui, lang, &mut self.revision_outlines);
                    });
                    if let Some(inspection) = &self.revision_outlines.inspection {
                        ui.separator();
                        ui.label(tr_args(
                            lang,
                            "outlines.detected",
                            &[
                                ("count", &inspection.regions.len().to_string()),
                                ("pixels", &inspection.changed_pixels.to_string()),
                            ],
                        ));
                        if !inspection.complete() {
                            ui.colored_label(
                                crate::theme::palette_for(ctx).error,
                                tr(lang, "outlines.partial"),
                            );
                        }
                        ui.small(tr_args(
                            lang,
                            "outlines.coverage",
                            &[
                                ("new", &inspection.unmapped_pixels.to_string()),
                                ("old", &inspection.unmapped_old_pixels.to_string()),
                                ("filtered", &inspection.filtered_pixels.to_string()),
                                ("omitted", &inspection.omitted_regions.to_string()),
                            ],
                        ));
                        if inspection.regions.is_empty() {
                            ui.label(tr(lang, "outlines.none"));
                        }
                        ui.collapsing(tr(lang, "overlay.fingerprints"), |ui| {
                            ui.monospace(&inspection.old_sha256);
                            ui.monospace(&inspection.new_sha256);
                        });
                        let selected_count = self
                            .revision_outlines
                            .decisions
                            .iter()
                            .filter(|d| d.selected)
                            .count();
                        egui::ScrollArea::vertical()
                            .id_salt("outline_regions")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for (index, (region, decision)) in inspection
                                    .regions
                                    .iter()
                                    .zip(&mut self.revision_outlines.decisions)
                                    .enumerate()
                                {
                                    ui.push_id(index, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            if ui
                                                .selectable_label(
                                                    self.revision_outlines.selected_region == index,
                                                    format!("#{}", index + 1),
                                                )
                                                .clicked()
                                            {
                                                self.revision_outlines.selected_region = index;
                                            }
                                            if ui
                                                .add_enabled(
                                                    decision.selected
                                                        || selected_count < core::MAX_SELECTIONS,
                                                    egui::Checkbox::new(
                                                        &mut decision.selected,
                                                        tr(lang, "outlines.select"),
                                                    ),
                                                )
                                                .changed()
                                            {
                                                decision.confirmed = false;
                                                changed = true;
                                            }
                                            ui.label(format!("{} px", region.changed_pixels));
                                            egui::ComboBox::from_id_salt("classification")
                                                .selected_text(tr(
                                                    lang,
                                                    class_key(decision.classification),
                                                ))
                                                .show_ui(ui, |ui| {
                                                    for class in [
                                                        OutlineClassification::ContentChange,
                                                        OutlineClassification::RenderingNoise,
                                                        OutlineClassification::Uncertain,
                                                    ] {
                                                        if ui
                                                            .selectable_value(
                                                                &mut decision.classification,
                                                                class,
                                                                tr(lang, class_key(class)),
                                                            )
                                                            .changed()
                                                        {
                                                            decision.confirmed = false;
                                                            changed = true;
                                                        }
                                                    }
                                                });
                                            changed |= ui
                                                .add_enabled(
                                                    decision.selected,
                                                    egui::Checkbox::new(
                                                        &mut decision.confirmed,
                                                        tr(lang, "outlines.confirm"),
                                                    ),
                                                )
                                                .changed();
                                            if region.status != "review_required" {
                                                ui.colored_label(
                                                    crate::theme::palette_for(ctx).error,
                                                    tr(lang, "outlines.edge"),
                                                );
                                            }
                                        })
                                    });
                                }
                            });
                    }
                });
                if source_changed {
                    self.revision_outlines.invalidate(true);
                } else if changed {
                    self.revision_outlines.invalidate(false);
                }
                show_images(ui, lang, &mut self.revision_outlines, busy);
                if let Some(report) = &self.revision_outlines.report {
                    let noise = report
                        .decisions
                        .iter()
                        .filter(|d| d.classification == OutlineClassification::RenderingNoise)
                        .count();
                    ui.label(tr_args(
                        lang,
                        "outlines.summary",
                        &[
                            ("created", &report.created.len().to_string()),
                            ("noise", &noise.to_string()),
                            ("excluded", &report.excluded_regions.len().to_string()),
                        ],
                    ));
                    if !report.complete() {
                        ui.colored_label(
                            crate::theme::palette_for(ctx).error,
                            tr(lang, "outlines.partial"),
                        );
                    }
                    if !report.can_publish() {
                        ui.label(tr(lang, "outlines.no_output"));
                    }
                }
                if busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr(lang, "outlines.working"));
                    });
                    if let Some(p) = self.revision_outlines.progress {
                        let label = tr(
                            lang,
                            match p.phase {
                                "read" => "outlines.phase_read",
                                "render" => "outlines.phase_render",
                                "coverage" => "outlines.phase_coverage",
                                "compare" => "outlines.phase_compare",
                                "components" => "outlines.phase_components",
                                _ => "outlines.phase_create",
                            },
                        );
                        ui.add(
                            egui::ProgressBar::new(if p.total == 0 {
                                0.0
                            } else {
                                p.done as f32 / p.total as f32
                            })
                            .text(format!("{label} {}/{}", p.done, p.total)),
                        );
                    }
                }
                if let Some(error) = &self.revision_outlines.error {
                    ui.colored_label(crate::theme::palette_for(ctx).error, error);
                }
                if let Some(message) = &self.revision_outlines.message {
                    ui.label(message);
                }
            },
            |ui| {
                if busy {
                    cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                } else {
                    save = ui
                        .add_enabled(can_save, egui::Button::new(tr(lang, "outlines.save")))
                        .clicked();
                    preview = ui
                        .add_enabled(can_preview, egui::Button::new(tr(lang, "outlines.preview")))
                        .clicked();
                    inspect = ui
                        .add_enabled(can_inspect, egui::Button::new(tr(lang, "outlines.inspect")))
                        .clicked();
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                }
            },
        );
        if changed || source_changed {
            self.discard_revision_outlines();
        }
        if let Some(index) = pick {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_file()
            {
                self.revision_outlines.paths[index] = Some(path);
                self.revision_outlines.passwords[index] = Default::default();
                self.revision_outlines.saved_files_confirmed = false;
                self.revision_outlines.invalidate(true);
                self.discard_revision_outlines();
            }
        }
        if inspect {
            self.inspect_revision_outlines();
        }
        if preview {
            self.preview_revision_outlines();
        }
        if save {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .set_file_name("revision-outlines.pdf")
                .save_file()
            {
                self.save_revision_outlines(path);
            }
        }
        if cancel {
            self.revision_outlines.cancel_pending();
            if !self.revision_outlines.busy() {
                self.discard_revision_outlines();
            }
        }
        if close {
            self.close_revision_outlines();
        }
    }
}

fn option_controls(ui: &mut egui::Ui, lang: Language, state: &mut OutlineState) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(lang, "outlines.resolution"));
        changed |= ui
            .add(egui::DragValue::new(&mut state.options.long_side).range(64..=1536))
            .changed();
        ui.label(tr(lang, "outlines.tolerance"));
        changed |= ui
            .add(egui::DragValue::new(&mut state.options.channel_tolerance).range(1..=254))
            .changed();
        ui.label(tr(lang, "outlines.minimum"));
        changed |= ui
            .add(egui::DragValue::new(&mut state.options.min_component_pixels).range(1..=10000))
            .changed();
        ui.label(tr(lang, "outlines.gap"));
        changed |= ui
            .add(egui::DragValue::new(&mut state.options.merge_gap_pixels).range(0..=16))
            .changed();
    });
    ui.small(tr(lang, "outlines.coordinates"));
    changed |= ui
        .checkbox(&mut state.anchors, tr(lang, "outlines.anchors"))
        .changed();
    if state.anchors {
        ui.columns(2, |columns| {
            for (source, column) in columns.iter_mut().enumerate() {
                column.label(tr(
                    lang,
                    if source == 0 {
                        "transfer.old"
                    } else {
                        "transfer.new"
                    },
                ));
                for anchor in 0..2 {
                    column.push_id(anchor, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{}", anchor + 1));
                            for axis in 0..2 {
                                ui.label(if axis == 0 { "x" } else { "y" });
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut state.points[source][anchor][axis],
                                        )
                                        .range(0.0..=1_000_000.0)
                                        .speed(0.5),
                                    )
                                    .changed();
                            }
                        })
                    });
                }
            }
        });
        state.options.alignment = OverlayAlignment::TwoPoints {
            base: state.points[1],
            revision: state.points[0],
        };
    } else {
        ui.horizontal_wrapped(|ui| {
            for axis in 0..2 {
                ui.label(if axis == 0 { "x" } else { "y" });
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut state.transform.translation[axis])
                            .range(-1_000_000.0..=1_000_000.0)
                            .speed(0.5),
                    )
                    .changed();
            }
            ui.label(tr(lang, "outlines.rotation"));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut state.transform.rotation_degrees)
                        .range(-360.0..=360.0)
                        .speed(0.1),
                )
                .changed();
            ui.label(tr(lang, "outlines.scale"));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut state.transform.scale)
                        .range(0.01..=100.0)
                        .speed(0.01),
                )
                .changed();
        });
        state.options.alignment = OverlayAlignment::Manual {
            transform: state.transform,
        };
    }
    changed
}
fn show_images(ui: &mut egui::Ui, lang: Language, state: &mut OutlineState, busy: bool) {
    if state.textures.iter().all(Option::is_none) {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for (index, key) in [
            "transfer.old",
            "transfer.new",
            "outlines.difference",
            "outlines.candidate",
        ]
        .into_iter()
        .enumerate()
        {
            if ui
                .add_enabled(
                    state.textures[index].is_some(),
                    egui::Button::new(tr(lang, key)).selected(state.image_tab == index),
                )
                .clicked()
            {
                state.image_tab = index;
            }
        }
    });
    if let Some(image) = &state.textures[state.image_tab] {
        let size = image.size_vec2();
        let scale = (ui.available_width() / size.x).min(380.0 / size.y).min(1.0);
        let response = ui.add(egui::Image::new((image.id(), size * scale)));
        if matches!(state.image_tab, 1 | 2) {
            if let Some(inspection) = &state.inspection {
                if let Some(region) = inspection.regions.get(state.selected_region) {
                    let [x0, y0, x1, y1] = region.pixel_bounds;
                    let [w, h] = inspection.raster_size;
                    let bounds = egui::Rect::from_min_max(
                        response.rect.min
                            + egui::vec2(
                                x0 as f32 / w as f32 * response.rect.width(),
                                y0 as f32 / h as f32 * response.rect.height(),
                            ),
                        response.rect.min
                            + egui::vec2(
                                x1 as f32 / w as f32 * response.rect.width(),
                                y1 as f32 / h as f32 * response.rect.height(),
                            ),
                    );
                    ui.painter().rect_stroke(
                        bounds,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 130, 240)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        }
        if state.image_tab == 3 && state.token.is_some() {
            ui.add_enabled(
                !busy,
                egui::Checkbox::new(&mut state.reviewed, tr(lang, "outlines.reviewed")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::test_app;

    fn inspection() -> core::OutlineInspection {
        let geometry = core::OutlineGeometry {
            page_count: 1,
            page: 1,
            display_size_pt: [600.0, 800.0],
            crop_box: [0.0, 0.0, 600.0, 800.0],
            rotation_degrees: 0,
        };
        core::OutlineInspection {
            version: 1,
            old_sha256: "a".repeat(64),
            new_sha256: "b".repeat(64),
            options: Default::default(),
            transform: Default::default(),
            old_geometry: geometry.clone(),
            new_geometry: geometry,
            raster_size: [600, 800],
            changed_pixels: 400,
            compared_pixels: 480000,
            unmapped_pixels: 0,
            unmapped_old_pixels: 0,
            filtered_pixels: 0,
            omitted_regions: 0,
            truncated: false,
            regions: vec![core::OutlineRegion {
                id: "region-0001".into(),
                fingerprint: "c".repeat(64),
                pixel_bounds: [10, 10, 30, 30],
                rect: [10.0, 770.0, 30.0, 790.0],
                changed_pixels: 400,
                status: "review_required".into(),
            }],
            warnings: vec![],
            images: None,
        }
    }
    fn state() -> OutlineState {
        OutlineState {
            open: true,
            paths: [Some("old.pdf".into()), Some("new.pdf".into())],
            saved_files_confirmed: true,
            inspection: Some(inspection()),
            decisions: vec![Decision::default()],
            ..Default::default()
        }
    }
    fn report(created: bool) -> core::OutlineReport {
        core::OutlineReport {
            version: 1,
            inspection: inspection(),
            decisions: vec![],
            created: if created {
                vec![core::CreatedOutline {
                    region_id: "region-0001".into(),
                    page: 1,
                    index: 0,
                    name: "SuniPDF-outline".into(),
                    classification: OutlineClassification::Uncertain,
                    rect: [10.0, 770.0, 30.0, 790.0],
                }]
            } else {
                vec![]
            },
            excluded_regions: vec![],
            output_path: None,
            output_sha256: None,
        }
    }
    #[test]
    fn explicit_decisions_and_actual_candidate_review_are_required() {
        let mut state = state();
        assert!(!state.can_prepare());
        state.decisions[0].selected = true;
        assert!(!state.can_prepare());
        state.decisions[0].confirmed = true;
        assert!(state.can_prepare());
        assert_eq!(
            state.request().unwrap().selections[0].fingerprint,
            "c".repeat(64)
        );
        state.token = Some(7);
        state.report = Some(report(true));
        assert!(!state.can_save());
        state.reviewed = true;
        assert!(state.can_save());
        state.report = Some(report(false));
        assert!(
            !state.can_save(),
            "noise-only reviews never create an empty PDF copy"
        );
        state.invalidate(false);
        assert!(state.inspection.is_some());
        assert!(!state.can_save());
        state.invalidate(true);
        assert!(state.inspection.is_none() && state.decisions.is_empty());
    }
    #[test]
    fn cancellation_ignores_late_previews_but_keeps_committed_publication_results() {
        let (mut app, _, _) = test_app();
        app.revision_outlines = state();
        let old = app.revision_outlines.begin(Pending::Preview);
        let cancel = app.revision_outlines.cancel.clone();
        app.revision_outlines.cancel_pending();
        assert!(cancel.load(Ordering::Relaxed));
        let current = app.revision_outlines.begin(Pending::Inspect);
        app.on_revision_outlines(
            &egui::Context::default(),
            old,
            Ok(Outcome::Preview {
                token: old,
                image: RenderedPage {
                    page_index: 0,
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                },
                report: report(true),
            }),
        );
        assert_eq!(
            app.revision_outlines.pending,
            Some((current, Pending::Inspect))
        );
        assert!(app.revision_outlines.token.is_none());
        let generation = app.revision_outlines.begin(Pending::Save);
        app.revision_outlines.cancel_pending();
        assert_eq!(
            app.revision_outlines.pending,
            Some((generation, Pending::Save))
        );
        let mut report = report(true);
        report.output_path = Some("reviewed-copy.pdf".into());
        report.output_sha256 = Some("d".repeat(64));
        app.on_revision_outlines(
            &egui::Context::default(),
            generation,
            Ok(Outcome::Saved(report)),
        );
        assert!(!app.revision_outlines.busy());
        assert!(app
            .revision_outlines
            .message
            .as_ref()
            .unwrap()
            .contains("reviewed-copy.pdf"));
    }
    #[test]
    fn modified_tabs_and_credentials_are_not_silently_substituted_for_saved_revisions() {
        let (mut app, requests, _) = test_app();
        app.revision_outlines = state();
        app.revision_outlines.decisions[0].selected = true;
        app.revision_outlines.decisions[0].confirmed = true;
        app.revision_outlines.paths[0] = Some(app.document.as_ref().unwrap().info.path.clone());
        app.dirty = true;
        app.can_undo = true;
        app.preview_revision_outlines();
        assert!(app.revision_outlines.error.is_some());
        assert!(requests.try_iter().next().is_none());
        assert!(app.dirty && app.can_undo);
        app.revision_outlines.passwords[0] = Zeroizing::new("temporary-input".into());
        app.inspect_revision_outlines();
        assert!(app.revision_outlines.passwords.iter().all(|s| s.is_empty()));
        assert!(matches!(
            requests.recv().unwrap(),
            Request::RevisionOutlines {
                job: Job::Inspect { .. },
                ..
            }
        ));
        app.revision_outlines.passwords[1] = Zeroizing::new("temporary-input".into());
        app.revision_outlines.cancel_pending();
        assert!(app.revision_outlines.passwords.iter().all(|s| s.is_empty()));
    }
    #[test]
    fn bilingual_modal_keeps_all_actions_reachable_and_save_disabled_until_review() {
        for language in Language::ALL {
            let (mut app, _, _) = test_app();
            app.settings.set_language(language);
            app.revision_outlines = state();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720.0, 480.0),
                        )),
                        time: Some(frame as f64 * 0.025),
                        ..Default::default()
                    },
                    |_| app.revision_outlines_dialog(&ctx),
                );
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in [
                    "outlines.save",
                    "outlines.preview",
                    "outlines.inspect",
                    "dialog.close",
                ] {
                    let button = tree
                        .nodes
                        .iter()
                        .map(|(_, n)| n)
                        .find(|n| n.label() == Some(tr(language, key)))
                        .unwrap_or_else(|| panic!("{key} accessible in {}", language.code()));
                    let b = button.bounds().unwrap();
                    assert!(
                        b.x0 >= 0.0 && b.x1 <= 720.0 && b.y0 >= 0.0 && b.y1 <= 480.0,
                        "{key}: {b:?}"
                    );
                    if key == "outlines.save" {
                        assert!(button.is_disabled());
                    }
                }
            }
        }
    }
}
