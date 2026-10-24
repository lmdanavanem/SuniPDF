//! Reviewed raster boundaries become calibrated area measurements only after explicit acceptance.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    measurement_shapes_ui::RectangleScaleSelection,
    worker::{EditOp, Request},
};
use sunipdf_core::{
    boundary_fill::{
        BoundaryClosure, BoundaryFillJob, BoundaryFillMeasurementOptions, BoundaryFillOptions,
        BoundaryFillPhase, BoundaryFillProgress, BoundaryFillReport, BoundaryFillStatus,
        PreparedBoundaryFillMeasurement,
    },
    measurement::{Measurement, MeasurementUnit, PageCalibration},
    Document, EngineError, RenderedPage,
};
use sunipdf_i18n::{tr, Language};

pub enum Job {
    Page {
        page: usize,
    },
    Start {
        page: usize,
        seed: [f32; 2],
        options: BoundaryFillOptions,
    },
    Prepare {
        scale: RectangleScaleSelection,
        unit: MeasurementUnit,
        precision: usize,
    },
    Cancel,
    Discard,
}

pub enum Outcome {
    Page {
        size: [f32; 2],
        image: RenderedPage,
        editable: bool,
        regions: Vec<(String, String)>,
        scale_error: Option<String>,
    },
    Started(BoundaryFillProgress),
    Progress(BoundaryFillProgress),
    Finished(BoundaryFillReport),
    Prepared {
        measurement: Measurement,
        image: RenderedPage,
    },
    Discarded,
}

#[derive(Default)]
pub struct WorkerCache {
    token: Option<(u64, u64)>,
    job: Option<BoundaryFillJob>,
    prepared: Option<PreparedBoundaryFillMeasurement>,
    running: bool,
    last_progress: Option<std::time::Instant>,
}

impl WorkerCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn active(&self) -> bool {
        self.running && self.job.is_some()
    }
    pub fn token(&self) -> Option<(u64, u64)> {
        self.token
    }
    fn accepts(&self, epoch: u64, generation: u64) -> bool {
        self.token
            .is_none_or(|(e, g)| e == epoch && generation >= g)
    }
    pub fn discard(&mut self, epoch: u64, generation: u64) -> bool {
        if !self.accepts(epoch, generation) {
            return false;
        }
        self.clear();
        true
    }
    fn check(&self, epoch: u64, generation: u64) -> Result<(), EngineError> {
        if self.token != Some((epoch, generation)) || self.job.is_none() {
            return Err(EngineError::Other(
                "Region fill is stale; find the region again".into(),
            ));
        }
        Ok(())
    }
    pub fn page(&self) -> Option<usize> {
        self.job.as_ref().map(|job| job.report().page)
    }
    pub fn take(
        &mut self,
        epoch: u64,
        generation: u64,
    ) -> Result<PreparedBoundaryFillMeasurement, EngineError> {
        self.check(epoch, generation)?;
        let prepared = self.prepared.take().ok_or_else(|| {
            EngineError::Other("Preview the calibrated region before applying it".into())
        })?;
        self.clear();
        Ok(prepared)
    }
    pub fn run(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        generation: u64,
        task: Job,
    ) -> Result<Outcome, EngineError> {
        match task {
            Job::Page { page } => {
                if !self.accepts(epoch, generation) {
                    return Err(EngineError::Other("Obsolete region fill request".into()));
                }
                self.clear();
                let size = doc.page_size(page)?;
                let scale = 2.0_f32
                    .min(4096. / size.width_pt.max(size.height_pt))
                    .min((8_388_608.0 / (size.width_pt * size.height_pt)).sqrt() * 0.999);
                let image = doc.render_visual_search_content(page, scale)?;
                let (regions, scale_error) = match doc.measurement_viewports(page) {
                    Ok(regions) => (regions.into_iter().map(|r| (r.id, r.name)).collect(), None),
                    Err(error) => (vec![], Some(error.to_string())),
                };
                let properties = doc.properties();
                Ok(Outcome::Page {
                    size: [size.width_pt, size.height_pt],
                    image,
                    editable: properties.can_copy
                        && properties.can_annotate
                        && doc.signature_count() == 0,
                    regions,
                    scale_error,
                })
            }
            Job::Start {
                page,
                seed,
                options,
            } => {
                if !self.accepts(epoch, generation) {
                    return Err(EngineError::Other("Obsolete region fill request".into()));
                }
                self.clear();
                let job = doc.prepare_boundary_fill(page, seed, options)?;
                let progress = job.progress();
                self.token = Some((epoch, generation));
                self.running = !progress.status.is_terminal();
                self.last_progress = Some(std::time::Instant::now());
                let outcome = if self.running {
                    Outcome::Started(progress)
                } else {
                    Outcome::Finished(job.report())
                };
                self.job = Some(job);
                Ok(outcome)
            }
            Job::Cancel => {
                self.check(epoch, generation)?;
                let job = self.job.as_mut().expect("checked fill job");
                job.cancel();
                self.running = false;
                self.prepared = None;
                Ok(Outcome::Finished(job.report()))
            }
            Job::Discard => {
                self.discard(epoch, generation);
                Ok(Outcome::Discarded)
            }
            Job::Prepare { .. } => unreachable!("scale resolved by worker"),
        }
    }
    pub fn step(&mut self) -> Option<Outcome> {
        let job = self.job.as_mut()?;
        let progress = job.step(16_384);
        if progress.status.is_terminal() {
            self.running = false;
            return Some(Outcome::Finished(job.report()));
        }
        if self
            .last_progress
            .is_none_or(|last| last.elapsed() >= std::time::Duration::from_millis(50))
        {
            self.last_progress = Some(std::time::Instant::now());
            return Some(Outcome::Progress(progress));
        }
        None
    }
    pub fn prepare(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        generation: u64,
        options: BoundaryFillMeasurementOptions,
    ) -> Result<Outcome, EngineError> {
        self.check(epoch, generation)?;
        self.prepared = None;
        let job = self.job.as_ref().expect("checked fill job");
        let prepared = doc.prepare_boundary_fill_measurement(job, &options, None)?;
        let measurement = prepared.measurement().clone();
        let image = doc.render_boundary_fill_measurement_preview(&prepared, 1400)?;
        self.prepared = Some(prepared);
        Ok(Outcome::Prepared { measurement, image })
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
enum CanvasMode {
    #[default]
    Seed,
    Closure,
    Pan,
}

#[derive(Clone, PartialEq, Default)]
enum ScaleChoice {
    #[default]
    Automatic,
    Page,
    Local,
    Region(String),
}

pub struct BoundaryFillState {
    pub open: bool,
    epoch: u64,
    generation: u64,
    page: usize,
    size: [f32; 2],
    page_texture: Option<egui::TextureHandle>,
    preview_texture: Option<egui::TextureHandle>,
    zoom: f32,
    mode: CanvasMode,
    drag_start: Option<egui::Pos2>,
    seed: Option<[f32; 2]>,
    closures: Vec<BoundaryClosure>,
    scale: f32,
    threshold: u8,
    progress: Option<BoundaryFillProgress>,
    report: Option<BoundaryFillReport>,
    measurement: Option<Measurement>,
    calibration: ScaleChoice,
    regions: Vec<(String, String)>,
    local: Option<PageCalibration>,
    unit: MeasurementUnit,
    precision: usize,
    reviewed: bool,
    editable: bool,
    pending: bool,
    running: bool,
    error: Option<String>,
    scale_error: Option<String>,
}
impl Default for BoundaryFillState {
    fn default() -> Self {
        Self {
            open: false,
            epoch: 0,
            generation: 0,
            page: 0,
            size: [1., 1.],
            page_texture: None,
            preview_texture: None,
            zoom: 1.,
            mode: CanvasMode::Seed,
            drag_start: None,
            seed: None,
            closures: vec![],
            scale: 1.,
            threshold: 160,
            progress: None,
            report: None,
            measurement: None,
            calibration: ScaleChoice::Automatic,
            regions: vec![],
            local: None,
            unit: MeasurementUnit::Metres,
            precision: 2,
            reviewed: false,
            editable: false,
            pending: false,
            running: false,
            error: None,
            scale_error: None,
        }
    }
}
impl BoundaryFillState {
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = false;
        self.running = false;
        self.progress = None;
        self.report = None;
        self.measurement = None;
        self.preview_texture = None;
        self.reviewed = false;
        self.error = None;
    }
    fn complete(&self) -> bool {
        self.report.as_ref().is_some_and(|r| {
            r.progress.status == BoundaryFillStatus::Complete && r.region.is_some()
        })
    }
    fn can_apply(&self) -> bool {
        self.editable
            && !self.pending
            && !self.running
            && self.complete()
            && self.measurement.is_some()
            && self.preview_texture.is_some()
            && self.reviewed
    }
}

fn texture(ctx: &egui::Context, name: &str, image: RenderedPage) -> egui::TextureHandle {
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width as usize, image.height as usize],
            &image.rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}
fn pdf_point(point: egui::Pos2, rect: egui::Rect, size: [f32; 2]) -> [f32; 2] {
    [
        ((point.x - rect.left()) / rect.width()).clamp(0., 1.) * size[0],
        ((rect.bottom() - point.y) / rect.height()).clamp(0., 1.) * size[1],
    ]
}
fn screen_point(point: [f32; 2], rect: egui::Rect, size: [f32; 2]) -> egui::Pos2 {
    egui::pos2(
        rect.left() + point[0] / size[0] * rect.width(),
        rect.bottom() - point[1] / size[1] * rect.height(),
    )
}
fn outline(
    ui: &egui::Ui,
    points: &[[f32; 2]],
    image: egui::Rect,
    size: [f32; 2],
    color: egui::Color32,
) {
    let screen: Vec<_> = points
        .iter()
        .map(|p| screen_point(*p, image, size))
        .collect();
    if screen.len() >= 3 {
        ui.painter_at(image).add(egui::Shape::closed_line(
            screen,
            egui::Stroke::new(2., color),
        ));
    }
}

impl SuniApp {
    pub(crate) fn open_boundary_fill(&mut self) {
        if !matches!(self.engine, EngineState::Ready)
            || self.worker.has_pending_mutations()
            || self.saving
            || self.ink_blocks_action()
            || self.annotate.quantities.blocks_document()
            || self.annotate.measure.is_active_draft()
        {
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let page = doc.page;
        let generation = self.boundary_fill.generation.wrapping_add(1);
        self.boundary_fill = BoundaryFillState {
            open: true,
            epoch: self.view.epoch,
            generation,
            page,
            pending: true,
            local: self.annotate.measure.local_calibration(page),
            ..Default::default()
        };
        self.annotate.register.open = false;
        self.annotate.quantities.open = false;
        self.worker.send(Request::BoundaryFill {
            epoch: self.view.epoch,
            generation,
            job: Job::Page { page },
        });
    }
    pub(crate) fn close_boundary_fill(&mut self) {
        self.boundary_fill.invalidate();
        self.boundary_fill.open = false;
        self.boundary_fill.page_texture = None;
        self.worker.send(Request::BoundaryFill {
            epoch: self.view.epoch,
            generation: self.boundary_fill.generation,
            job: Job::Discard,
        });
    }
    pub(crate) fn on_boundary_fill(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        let state = &mut self.boundary_fill;
        if !state.open
            || state.epoch != epoch
            || epoch != self.view.epoch
            || state.generation != generation
        {
            return;
        }
        match result {
            Ok(Outcome::Page {
                size,
                image,
                editable,
                regions,
                scale_error,
            }) => {
                state.pending = false;
                state.size = size;
                state.editable = editable;
                state.regions = regions;
                state.scale_error = scale_error;
                state.page_texture = Some(texture(ctx, "boundary-page", image));
            }
            Ok(Outcome::Started(progress) | Outcome::Progress(progress)) => {
                state.pending = false;
                state.running = true;
                state.progress = Some(progress);
            }
            Ok(Outcome::Finished(report)) => {
                state.pending = false;
                state.running = false;
                state.progress = Some(report.progress.clone());
                state.report = Some(report);
                state.measurement = None;
                state.preview_texture = None;
                state.reviewed = false;
            }
            Ok(Outcome::Prepared { measurement, image }) => {
                state.pending = false;
                state.measurement = Some(measurement);
                state.preview_texture = Some(texture(ctx, "boundary-preview", image));
                state.reviewed = false;
            }
            Ok(Outcome::Discarded) => {}
            Err(error) => {
                state.pending = false;
                state.running = false;
                state.progress = None;
                state.report = None;
                state.measurement = None;
                state.preview_texture = None;
                state.reviewed = false;
                state.error = Some(error.to_string());
            }
        }
    }
    pub(crate) fn boundary_fill_dialog(&mut self, ctx: &egui::Context) {
        if !self.boundary_fill.open {
            return;
        }
        if self.boundary_fill.epoch != self.view.epoch {
            self.close_boundary_fill();
            return;
        }
        let lang = self.lang();
        let width = (ctx.content_rect().width() - 32.).clamp(300., 960.);
        let state = &mut self.boundary_fill;
        let busy = state.pending || state.running;
        let can_start = !busy && state.seed.is_some() && state.page_texture.is_some();
        let can_preview = !busy && state.editable && state.complete();
        let can_apply = state.can_apply() && !self.worker.has_pending_mutations() && !self.saving;
        let running = state.running;
        let (mut changed, mut start, mut cancel, mut prepare, mut apply, mut close) =
            (false, false, false, false, false, false);
        dialog::show(
            ctx,
            Spec::new(
                "boundary-fill",
                tr(lang, "boundary_fill.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "boundary_fill.help"))
            .width(width),
            |ui| {
                ui.label(tr(lang, "boundary_fill.scope"));
                if let Some(error) = &state.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(error) = &state.scale_error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{} {}", tr(lang, "register.page"), state.page + 1));
                        ui.label(tr(lang, "visual_search.resolution"));
                        egui::ComboBox::from_id_salt("boundary-resolution")
                            .selected_text(format!("{} dpi", (state.scale * 72.) as u32))
                            .show_ui(ui, |ui| {
                                for scale in [0.25, 0.5, 1., 2.] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut state.scale,
                                            scale,
                                            format!("{} dpi", (scale * 72.) as u32),
                                        )
                                        .changed();
                                }
                            });
                        ui.label(tr(lang, "boundary_fill.threshold"));
                        changed |= ui
                            .add(egui::Slider::new(&mut state.threshold, 1..=254))
                            .changed();
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(
                            &mut state.mode,
                            CanvasMode::Seed,
                            tr(lang, "boundary_fill.seed"),
                        );
                        ui.selectable_value(
                            &mut state.mode,
                            CanvasMode::Closure,
                            tr(lang, "boundary_fill.closure"),
                        );
                        ui.selectable_value(
                            &mut state.mode,
                            CanvasMode::Pan,
                            tr(lang, "visual_search.pan"),
                        );
                        if ui.button(tr(lang, "visual_search.fit")).clicked() {
                            state.zoom = 1.;
                            state.drag_start = None;
                        }
                        if ui
                            .add(
                                egui::Slider::new(&mut state.zoom, 1.0..=12.0)
                                    .fixed_decimals(1)
                                    .suffix("×"),
                            )
                            .changed()
                        {
                            state.drag_start = None;
                        }
                    });
                    ui.small(tr(lang, "boundary_fill.draw_help"));
                    if let Some(image) = &state.page_texture {
                        let fit = (ui.available_width() / state.size[0]).min(330. / state.size[1]);
                        let draw_size = egui::vec2(state.size[0], state.size[1]) * fit * state.zoom;
                        let source = egui::scroll_area::ScrollSource {
                            drag: if state.mode == CanvasMode::Pan {
                                egui::scroll_area::DragScroll::Always
                            } else {
                                egui::scroll_area::DragScroll::Never
                            },
                            ..Default::default()
                        };
                        egui::ScrollArea::both()
                            .id_salt("boundary-canvas")
                            .max_height(330.)
                            .auto_shrink([false, false])
                            .scroll_source(source)
                            .show(ui, |ui| {
                                let sense = match state.mode {
                                    CanvasMode::Seed => egui::Sense::click(),
                                    CanvasMode::Closure => egui::Sense::drag(),
                                    CanvasMode::Pan => egui::Sense::hover(),
                                };
                                let response = ui.add(
                                    egui::Image::new(image)
                                        .fit_to_exact_size(draw_size)
                                        .sense(sense),
                                );
                                let painter = ui.painter_at(response.rect);
                                if let Some(seed) = state.seed {
                                    painter.circle_stroke(
                                        screen_point(seed, response.rect, state.size),
                                        5.,
                                        egui::Stroke::new(2., egui::Color32::LIGHT_GREEN),
                                    );
                                }
                                for closure in &state.closures {
                                    painter.line_segment(
                                        [
                                            screen_point(closure.start, response.rect, state.size),
                                            screen_point(closure.end, response.rect, state.size),
                                        ],
                                        egui::Stroke::new(2., egui::Color32::YELLOW),
                                    );
                                }
                                if let Some(region) =
                                    state.report.as_ref().and_then(|r| r.region.as_ref())
                                {
                                    outline(
                                        ui,
                                        &region.outer,
                                        response.rect,
                                        state.size,
                                        egui::Color32::LIGHT_BLUE,
                                    );
                                    for hole in &region.holes {
                                        outline(
                                            ui,
                                            hole,
                                            response.rect,
                                            state.size,
                                            egui::Color32::LIGHT_RED,
                                        );
                                    }
                                }
                                if state.mode == CanvasMode::Seed && response.clicked() {
                                    if let Some(p) = response.interact_pointer_pos() {
                                        state.seed = Some(pdf_point(p, response.rect, state.size));
                                        changed = true;
                                    }
                                }
                                if state.mode == CanvasMode::Closure {
                                    if response.drag_started() {
                                        state.drag_start = response.interact_pointer_pos();
                                    }
                                    if let (Some(a), Some(b)) =
                                        (state.drag_start, response.interact_pointer_pos())
                                    {
                                        painter.line_segment(
                                            [a, b],
                                            egui::Stroke::new(2., egui::Color32::YELLOW),
                                        );
                                        if response.drag_stopped() {
                                            if a.distance(b) >= 2. && state.closures.len() < 32 {
                                                state.closures.push(BoundaryClosure {
                                                    start: pdf_point(a, response.rect, state.size),
                                                    end: pdf_point(b, response.rect, state.size),
                                                });
                                                changed = true;
                                            }
                                            state.drag_start = None;
                                        }
                                    }
                                } else {
                                    state.drag_start = None;
                                }
                            });
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{}: {}/32",
                            tr(lang, "boundary_fill.closures"),
                            state.closures.len()
                        ));
                        if ui
                            .add_enabled(
                                !state.closures.is_empty(),
                                egui::Button::new(tr(lang, "boundary_fill.undo_closure")),
                            )
                            .clicked()
                        {
                            state.closures.pop();
                            changed = true;
                        }
                        if ui
                            .add_enabled(
                                state.seed.is_some() || !state.closures.is_empty(),
                                egui::Button::new(tr(lang, "boundary_fill.reset")),
                            )
                            .clicked()
                        {
                            state.seed = None;
                            state.closures.clear();
                            state.drag_start = None;
                            changed = true;
                        }
                    });
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "measure.rectangle.scale"));
                        let selected = match &state.calibration {
                            ScaleChoice::Automatic => {
                                tr(lang, "measure.rectangle.automatic").to_string()
                            }
                            ScaleChoice::Page => tr(lang, "measure.rectangle.page").to_string(),
                            ScaleChoice::Local => tr(lang, "measure.rectangle.local").to_string(),
                            ScaleChoice::Region(id) => state
                                .regions
                                .iter()
                                .find(|(i, _)| i == id)
                                .map(|(_, name)| name.clone())
                                .unwrap_or_else(|| tr(lang, "measure.rectangle.region").into()),
                        };
                        egui::ComboBox::from_id_salt("boundary-scale")
                            .width(220.)
                            .truncate()
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut state.calibration,
                                        ScaleChoice::Automatic,
                                        tr(lang, "measure.rectangle.automatic"),
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut state.calibration,
                                        ScaleChoice::Page,
                                        tr(lang, "measure.rectangle.page"),
                                    )
                                    .changed();
                                ui.add_enabled_ui(state.local.is_some(), |ui| {
                                    changed |= ui
                                        .selectable_value(
                                            &mut state.calibration,
                                            ScaleChoice::Local,
                                            tr(lang, "measure.rectangle.local"),
                                        )
                                        .changed();
                                });
                                for (id, name) in &state.regions {
                                    changed |= ui
                                        .selectable_value(
                                            &mut state.calibration,
                                            ScaleChoice::Region(id.clone()),
                                            name,
                                        )
                                        .changed();
                                }
                            });
                        ui.label(tr(lang, "measure.unit"));
                        egui::ComboBox::from_id_salt("boundary-unit")
                            .selected_text(state.unit.id())
                            .show_ui(ui, |ui| {
                                for unit in MeasurementUnit::ALL {
                                    changed |= ui
                                        .selectable_value(&mut state.unit, unit, unit.id())
                                        .changed();
                                }
                            });
                        ui.label(tr(lang, "measure.precision"));
                        changed |= ui
                            .add(egui::DragValue::new(&mut state.precision).range(0..=4))
                            .changed();
                    });
                    ui.small(tr(lang, "boundary_fill.calibration_help"));
                });
                if let Some(progress) = &state.progress {
                    ui.separator();
                    ui.strong(status_label(lang, progress.status));
                    if running {
                        ui.label(phase_label(lang, progress.phase));
                    }
                    ui.label(format!(
                        "{}: {} · {}: {}",
                        tr(lang, "boundary_fill.pixels"),
                        progress.filled_pixels,
                        tr(lang, "boundary_fill.work"),
                        progress.work
                    ));
                    if running {
                        ui.spinner();
                    }
                }
                if let Some(report) = &state.report {
                    if report.progress.status != BoundaryFillStatus::Complete {
                        ui.colored_label(crate::theme::ERROR, tr(lang, "boundary_fill.incomplete"));
                    }
                    if let Some(region) = &report.region {
                        ui.label(format!(
                            "{}: {} · {}: {}",
                            tr(lang, "boundary_fill.vertices"),
                            region.outer.len(),
                            tr(lang, "boundary_fill.holes"),
                            region.holes.len()
                        ));
                        ui.small(tr(lang, "boundary_fill.outline_help"));
                    }
                }
                if let (Some(measurement), Some(image)) =
                    (&state.measurement, &state.preview_texture)
                {
                    if let Ok(value) = measurement.value() {
                        ui.strong(format!(
                            "{}: {:.*} {}²",
                            tr(lang, "boundary_fill.area"),
                            measurement.precision,
                            value,
                            measurement.unit.id()
                        ));
                    }
                    ui.add(
                        egui::Image::new(image).max_size(egui::vec2(ui.available_width(), 340.)),
                    );
                    ui.add_enabled_ui(!busy, |ui| {
                        ui.checkbox(&mut state.reviewed, tr(lang, "boundary_fill.reviewed"));
                    });
                }
                if !state.editable && !state.pending {
                    ui.label(tr(lang, "boundary_fill.read_only"));
                }
            },
            |ui| {
                apply = ui
                    .add_enabled(
                        can_apply,
                        egui::Button::new(tr(lang, "boundary_fill.apply")),
                    )
                    .clicked();
                prepare = ui
                    .add_enabled(
                        can_preview,
                        egui::Button::new(tr(lang, "boundary_fill.preview")),
                    )
                    .clicked();
                start = ui
                    .add_enabled(
                        can_start,
                        egui::Button::new(tr(lang, "boundary_fill.start")),
                    )
                    .clicked();
                if running {
                    cancel = ui.button(tr(lang, "visual_search.cancel")).clicked();
                }
                close = ui.button(tr(lang, "visual_search.close")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if close {
            self.close_boundary_fill();
            return;
        }
        if changed {
            state.invalidate();
            self.worker.send(Request::BoundaryFill {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Discard,
            });
        }
        if start && can_start {
            let Some(seed) = state.seed else {
                return;
            };
            state.invalidate();
            state.pending = true;
            self.worker.send(Request::BoundaryFill {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Start {
                    page: state.page,
                    seed,
                    options: BoundaryFillOptions {
                        scale: state.scale,
                        threshold: state.threshold,
                        closures: state.closures.clone(),
                        ..Default::default()
                    },
                },
            });
        } else if cancel && state.running {
            state.pending = true;
            self.worker.send(Request::BoundaryFill {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Cancel,
            });
        } else if prepare && state.complete() && !changed {
            let scale = match &state.calibration {
                ScaleChoice::Automatic => RectangleScaleSelection::Automatic,
                ScaleChoice::Page => RectangleScaleSelection::Page,
                ScaleChoice::Region(id) => RectangleScaleSelection::Region(id.clone()),
                ScaleChoice::Local => {
                    let Some(local) = &state.local else {
                        return;
                    };
                    RectangleScaleSelection::Local(local.clone())
                }
            };
            state.pending = true;
            state.reviewed = false;
            state.measurement = None;
            state.preview_texture = None;
            self.worker.send(Request::BoundaryFill {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Prepare {
                    scale,
                    unit: state.unit,
                    precision: state.precision,
                },
            });
        } else if apply && state.can_apply() {
            state.open = false;
            state.page_texture = None;
            state.preview_texture = None;
            self.worker.send(Request::Edit(EditOp::CommitBoundaryFill {
                epoch: state.epoch,
                generation: state.generation,
            }));
        }
    }
}

fn status_label(lang: Language, status: BoundaryFillStatus) -> &'static str {
    tr(
        lang,
        match status {
            BoundaryFillStatus::Running => "boundary_fill.running",
            BoundaryFillStatus::Complete => "boundary_fill.complete",
            BoundaryFillStatus::Cancelled => "boundary_fill.cancelled",
            BoundaryFillStatus::Leaked => "boundary_fill.leaked",
            BoundaryFillStatus::SeedBlocked => "boundary_fill.seed_blocked",
            BoundaryFillStatus::PixelLimit
            | BoundaryFillStatus::FrontierLimit
            | BoundaryFillStatus::EdgeLimit
            | BoundaryFillStatus::WorkLimit => "boundary_fill.limit",
            BoundaryFillStatus::TopologyRejected => "boundary_fill.topology",
            BoundaryFillStatus::ComplexityLimit => "boundary_fill.complexity",
        },
    )
}

fn phase_label(lang: Language, phase: BoundaryFillPhase) -> &'static str {
    tr(
        lang,
        match phase {
            BoundaryFillPhase::Threshold => "boundary_fill.phase.threshold",
            BoundaryFillPhase::Closures => "boundary_fill.phase.closures",
            BoundaryFillPhase::Flood => "boundary_fill.phase.flood",
            BoundaryFillPhase::Edges | BoundaryFillPhase::Contours => {
                "boundary_fill.phase.contours"
            }
            BoundaryFillPhase::Validation | BoundaryFillPhase::Finished => {
                "boundary_fill.phase.validation"
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::boundary_fill::{BoundaryFillGeometry, BoundaryFillRegion};
    use sunipdf_core::measurement::MeasurementKind;

    fn report(status: BoundaryFillStatus) -> BoundaryFillReport {
        BoundaryFillReport {
            version: 1,
            source_sha256: "a".repeat(64),
            page: 0,
            seed: [25., 25.],
            geometry: BoundaryFillGeometry {
                bounds: [0., 0., 100., 100.],
                rotation: 0,
                display_size: [100., 100.],
            },
            raster_width: 100,
            raster_height: 100,
            closure_width_pixels: 1,
            closure_rasterization: "supercover".into(),
            closure_pixels: 0,
            options: BoundaryFillOptions::default(),
            progress: BoundaryFillProgress {
                status,
                phase: BoundaryFillPhase::Finished,
                work: 2000,
                filled_pixels: 2500,
                frontier: 0,
                edges: 200,
                contours: 1,
            },
            region: Some(BoundaryFillRegion {
                outer: vec![[10., 10.], [60., 10.], [60., 60.], [10., 60.]],
                holes: vec![],
                pixel_count: 2500,
                area_points2: 2500.,
            }),
        }
    }
    fn measurement() -> Measurement {
        Measurement {
            kind: MeasurementKind::Area,
            points: vec![(10., 10.), (60., 10.), (60., 60.), (10., 60.)],
            holes: vec![],
            depth: None,
            calibration: [(0., 0.), (10., 0.)],
            known_distance: 1.,
            unit: MeasurementUnit::Metres,
            precision: 2,
        }
    }
    fn white_image() -> RenderedPage {
        RenderedPage {
            page_index: 0,
            width: 100,
            height: 100,
            rgba: vec![255; 100 * 100 * 4],
        }
    }

    #[test]
    fn canvas_coordinates_remain_exact_when_zoomed_and_scrolled() {
        for zoom in [1., 4., 12.] {
            let canvas =
                egui::Rect::from_min_size(egui::pos2(-150., -300.), egui::vec2(200., 300.) * zoom);
            let point = [25., 175.];
            let result = pdf_point(
                screen_point(point, canvas, [100., 200.]),
                canvas,
                [100., 200.],
            );
            assert!((result[0] - point[0]).abs() < 0.001 && (result[1] - point[1]).abs() < 0.001);
        }
    }

    #[test]
    fn older_discard_cannot_remove_a_replacement_region_job() {
        let mut cache = WorkerCache {
            token: Some((20, 8)),
            running: true,
            ..Default::default()
        };
        assert!(!cache.discard(20, 7));
        assert!(!cache.discard(19, 99));
        assert_eq!(cache.token(), Some((20, 8)));
        assert!(cache.running);
        assert!(cache.discard(20, 9));
        assert!(cache.token().is_none() && !cache.running);
    }

    #[test]
    fn changed_settings_clear_all_acceptance_state_but_keep_the_draft() {
        let mut state = BoundaryFillState {
            seed: Some([10., 20.]),
            generation: 3,
            running: true,
            pending: true,
            report: Some(report(BoundaryFillStatus::Complete)),
            measurement: Some(measurement()),
            reviewed: true,
            closures: vec![BoundaryClosure {
                start: [1., 2.],
                end: [3., 4.],
            }],
            ..Default::default()
        };
        state.invalidate();
        assert_eq!(state.generation, 4);
        assert!(state.report.is_none() && state.measurement.is_none() && state.progress.is_none());
        assert!(!state.running && !state.pending && !state.reviewed && !state.can_apply());
        assert_eq!(state.seed, Some([10., 20.]));
        assert_eq!(state.closures.len(), 1);
    }

    #[test]
    fn only_complete_prepared_reviewed_regions_can_be_applied() {
        let ctx = egui::Context::default();
        let mut state = BoundaryFillState {
            editable: true,
            report: Some(report(BoundaryFillStatus::Complete)),
            ..Default::default()
        };
        assert!(!state.can_apply());
        state.measurement = Some(measurement());
        state.preview_texture = Some(texture(&ctx, "review-area", white_image()));
        assert!(!state.can_apply());
        state.reviewed = true;
        assert!(state.can_apply());
        for status in [
            BoundaryFillStatus::Running,
            BoundaryFillStatus::Cancelled,
            BoundaryFillStatus::Leaked,
            BoundaryFillStatus::SeedBlocked,
            BoundaryFillStatus::PixelLimit,
            BoundaryFillStatus::FrontierLimit,
            BoundaryFillStatus::EdgeLimit,
            BoundaryFillStatus::WorkLimit,
            BoundaryFillStatus::TopologyRejected,
            BoundaryFillStatus::ComplexityLimit,
        ] {
            state.report = Some(report(status));
            assert!(!state.can_apply(), "{status:?} must not create an area");
        }
        state.report = Some(report(BoundaryFillStatus::Complete));
        state.pending = true;
        assert!(!state.can_apply());
        state.pending = false;
        state.editable = false;
        assert!(!state.can_apply());
    }

    #[test]
    fn failed_preview_removes_stale_measurement_and_review() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.boundary_fill = BoundaryFillState {
            open: true,
            epoch: app.view.epoch,
            generation: 4,
            pending: true,
            report: Some(report(BoundaryFillStatus::Complete)),
            measurement: Some(measurement()),
            progress: Some(report(BoundaryFillStatus::Complete).progress),
            reviewed: true,
            preview_texture: Some(texture(&ctx, "failed-area", white_image())),
            ..Default::default()
        };
        app.on_boundary_fill(
            &ctx,
            app.view.epoch,
            4,
            Err(EngineError::Other("scale changed".into())),
        );
        let state = &app.boundary_fill;
        assert!(
            state.report.is_none()
                && state.progress.is_none()
                && state.measurement.is_none()
                && state.preview_texture.is_none()
        );
        assert!(!state.pending && !state.running && !state.reviewed && !state.can_apply());
        assert!(state.error.as_deref().unwrap().contains("scale changed"));
    }

    #[test]
    fn closed_or_changed_document_ignores_late_results_and_reopens_fresh() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_boundary_fill();
        let (epoch, generation) = match requests.try_recv().unwrap() {
            Request::BoundaryFill {
                epoch,
                generation,
                job: Job::Page { .. },
            } => (epoch, generation),
            _ => panic!("page load"),
        };
        app.boundary_fill.seed = Some([12., 12.]);
        app.close_boundary_fill();
        app.on_boundary_fill(
            &egui::Context::default(),
            epoch,
            generation,
            Ok(Outcome::Finished(report(BoundaryFillStatus::Complete))),
        );
        assert!(!app.boundary_fill.open && app.boundary_fill.report.is_none());
        app.open_boundary_fill();
        assert!(app.boundary_fill.seed.is_none() && app.boundary_fill.closures.is_empty());
        assert!(app.boundary_fill.generation > generation);
        app.view.epoch = app.view.epoch.wrapping_add(1);
        app.boundary_fill_dialog(&egui::Context::default());
        assert!(!app.boundary_fill.open);
    }

    #[test]
    fn bilingual_footer_actions_fit_small_windows_without_implicit_acceptance() {
        for language in Language::ALL {
            for running in [false, true] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.settings.set_language(language);
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                app.boundary_fill = BoundaryFillState {
                    open: true,
                    epoch: app.view.epoch,
                    editable: true,
                    size: [100., 100.],
                    zoom: 8.,
                    mode: CanvasMode::Pan,
                    running,
                    report: Some(report(if running {
                        BoundaryFillStatus::Running
                    } else {
                        BoundaryFillStatus::Complete
                    })),
                    page_texture: Some(texture(&ctx, "qa-boundary", white_image())),
                    ..Default::default()
                };
                for frame in 0..3 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(720., 480.),
                            )),
                            time: Some(frame as f64 * 0.025),
                            ..Default::default()
                        },
                        |_| app.boundary_fill_dialog(&ctx),
                    );
                    output.textures_delta.clear();
                    if frame < 2 {
                        continue;
                    }
                    let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                    for key in [
                        "boundary_fill.apply",
                        "boundary_fill.preview",
                        "boundary_fill.start",
                        "visual_search.close",
                    ] {
                        let node = tree
                            .nodes
                            .iter()
                            .map(|(_, n)| n)
                            .find(|n| n.label() == Some(tr(language, key)))
                            .unwrap();
                        let bounds = node.bounds().unwrap();
                        assert!(
                            bounds.x0 >= 0.
                                && bounds.x1 <= 720.
                                && bounds.y0 >= 0.
                                && bounds.y1 <= 480.,
                            "{key}: {bounds:?}"
                        );
                        if key == "boundary_fill.apply" {
                            assert!(node.is_disabled());
                        }
                    }
                }
                assert!(!app.boundary_fill.reviewed && app.boundary_fill.measurement.is_none());
                assert!(
                    requests.try_recv().is_err(),
                    "rendering never commits or selects a region"
                );
            }
        }
    }
}
