//! Reviewed symbol matches. The worker owns the search and its source-bound count candidate.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::{EditOp, Request},
};
use std::collections::BTreeSet;
use sunipdf_core::{
    visual_search::{
        PreparedVisualSearchCounts, VisualSearchJob, VisualSearchOptions, VisualSearchProgress,
        VisualSearchReport, VisualSearchStatus, MAX_SCAN_PAGES,
    },
    Document, EngineError, PixelRect, PointRect, RenderedPage, Rotation,
};
use sunipdf_i18n::{tr, Language};

pub enum Job {
    Page {
        page: usize,
    },
    Start {
        page: usize,
        rect: PointRect,
        options: VisualSearchOptions,
    },
    Preview {
        id: u32,
    },
    Cancel,
    Discard,
}

pub enum Outcome {
    Page {
        page: usize,
        size: [f32; 2],
        image: RenderedPage,
        editable: bool,
    },
    Started {
        template: RenderedPage,
        progress: VisualSearchProgress,
    },
    Progress(VisualSearchProgress),
    Finished(VisualSearchReport),
    Preview {
        id: u32,
        image: RenderedPage,
    },
    Discarded,
}

#[derive(Default)]
pub struct WorkerCache {
    token: Option<(u64, u64)>,
    job: Option<VisualSearchJob>,
    running: bool,
    last_progress: Option<std::time::Instant>,
    last_counts: (usize, usize),
}

impl WorkerCache {
    pub fn clear(&mut self) {
        self.job = None;
        self.token = None;
        self.running = false;
        self.last_progress = None;
        self.last_counts = (0, 0);
    }
    pub fn discard(&mut self, epoch: u64, generation: u64) -> bool {
        if !self.accepts_replacement(epoch, generation) {
            return false;
        }
        self.clear();
        true
    }
    fn accepts_replacement(&self, epoch: u64, generation: u64) -> bool {
        self.token
            .is_none_or(|(current_epoch, current_generation)| {
                current_epoch == epoch && generation >= current_generation
            })
    }
    pub fn active(&self) -> bool {
        self.running && self.job.is_some()
    }
    pub fn token(&self) -> Option<(u64, u64)> {
        self.token
    }

    pub fn run(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        generation: u64,
        task: Job,
    ) -> Result<Outcome, EngineError> {
        match task {
            Job::Discard => {
                self.discard(epoch, generation);
                Ok(Outcome::Discarded)
            }
            Job::Page { page } => {
                if !self.accepts_replacement(epoch, generation) {
                    return Err(EngineError::Other("obsolete symbol search request".into()));
                }
                self.clear();
                let size = doc.page_size(page)?;
                let max_area_scale =
                    (8_388_608.0 / (size.width_pt * size.height_pt)).sqrt() * 0.999;
                let scale = 2.0_f32
                    .min(4096. / size.width_pt.max(size.height_pt))
                    .min(max_area_scale);
                let image = doc.render_visual_search_content(page, scale)?;
                let properties = doc.properties();
                Ok(Outcome::Page {
                    page,
                    size: [size.width_pt, size.height_pt],
                    image,
                    editable: properties.can_copy
                        && properties.can_annotate
                        && doc.signature_count() == 0,
                })
            }
            Job::Start {
                page,
                rect,
                options,
            } => {
                if !self.accepts_replacement(epoch, generation) {
                    return Err(EngineError::Other("obsolete symbol search request".into()));
                }
                self.clear();
                let job = doc.prepare_visual_search(page, rect, options)?;
                let result = Outcome::Started {
                    template: job.template_preview().clone(),
                    progress: job.progress(),
                };
                self.token = Some((epoch, generation));
                self.job = Some(job);
                self.running = true;
                self.last_progress = Some(std::time::Instant::now());
                Ok(result)
            }
            Job::Cancel => {
                self.check_token(epoch, generation)?;
                let job = self.job.as_mut().expect("checked search job");
                job.cancel();
                self.running = false;
                Ok(Outcome::Finished(job.report()))
            }
            Job::Preview { id } => {
                self.check_token(epoch, generation)?;
                let job = self.job.as_ref().expect("checked search job");
                doc.verify_visual_search(job)?;
                let candidate = job
                    .report()
                    .candidates
                    .into_iter()
                    .find(|c| c.id == id)
                    .ok_or_else(|| {
                        EngineError::Other("symbol search candidate no longer exists".into())
                    })?;
                let size = doc.page_size(candidate.page)?;
                let scale = (4096. / size.width_pt.max(size.height_pt)).min(2.);
                let pad = ((candidate.rect.right - candidate.rect.left)
                    .max(candidate.rect.top - candidate.rect.bottom)
                    * 0.2)
                    .max(4.);
                let left = (candidate.rect.left - pad).max(0.);
                let right = (candidate.rect.right + pad).min(size.width_pt);
                let top = (candidate.rect.top + pad).min(size.height_pt);
                let bottom = (candidate.rect.bottom - pad).max(0.);
                let x = (left * scale).floor() as u32;
                let y = ((size.height_pt - top) * scale).floor() as u32;
                let region = PixelRect::new(
                    x,
                    y,
                    ((right * scale).ceil() as u32).saturating_sub(x).max(1),
                    (((size.height_pt - bottom) * scale).ceil() as u32)
                        .saturating_sub(y)
                        .max(1),
                );
                let image = doc.render_region(candidate.page, scale, region)?;
                Ok(Outcome::Preview { id, image })
            }
        }
    }

    fn check_token(&self, epoch: u64, generation: u64) -> Result<(), EngineError> {
        if self.token != Some((epoch, generation)) || self.job.is_none() {
            return Err(EngineError::Other(
                "symbol search is stale; search again".into(),
            ));
        }
        Ok(())
    }

    pub fn step(&mut self, doc: &Document<'_>) -> Result<Option<Outcome>, EngineError> {
        let job = self
            .job
            .as_mut()
            .ok_or_else(|| EngineError::Other("no symbol search".into()))?;
        if job.pending_page().is_some() {
            doc.load_visual_search_page(job)?;
            return Ok(Some(Outcome::Progress(job.progress())));
        }
        let progress = job.step(16_384);
        self.running = progress.status == VisualSearchStatus::Running;
        if !self.running {
            return Ok(Some(Outcome::Finished(job.report())));
        }
        let counts = (progress.pages_scanned, progress.candidates);
        if self.last_counts != counts
            || self
                .last_progress
                .is_none_or(|at| at.elapsed().as_millis() >= 50)
        {
            self.last_progress = Some(std::time::Instant::now());
            self.last_counts = counts;
            Ok(Some(Outcome::Progress(progress)))
        } else {
            Ok(None)
        }
    }

    pub fn prepare_counts(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        generation: u64,
        ids: &[u32],
        group: &str,
        size: f32,
    ) -> Result<PreparedVisualSearchCounts, EngineError> {
        self.check_token(epoch, generation)?;
        if self.running {
            return Err(EngineError::Other(
                "finish or cancel the symbol search before counting".into(),
            ));
        }
        let prepared = doc.prepare_visual_search_counts(
            self.job.as_ref().expect("checked search job"),
            ids,
            group,
            size,
            None,
        )?;
        self.clear();
        Ok(prepared)
    }
}

pub struct VisualSearchState {
    pub open: bool,
    epoch: u64,
    generation: u64,
    pending: bool,
    searching: bool,
    editable: bool,
    page: usize,
    page_count: usize,
    size: [f32; 2],
    page_texture: Option<egui::TextureHandle>,
    canvas_zoom: f32,
    pan: bool,
    template_texture: Option<egui::TextureHandle>,
    preview_texture: Option<(u32, egui::TextureHandle)>,
    rect: Option<PointRect>,
    drag_start: Option<egui::Pos2>,
    target_pages: String,
    sensitivity: f32,
    scale: f32,
    quarter_turns: bool,
    progress: Option<VisualSearchProgress>,
    report: Option<VisualSearchReport>,
    selected: BTreeSet<u32>,
    reviewed: bool,
    group: String,
    marker_size: f32,
    error: Option<String>,
    offset: usize,
}

impl Default for VisualSearchState {
    fn default() -> Self {
        Self {
            open: false,
            epoch: 0,
            generation: 0,
            pending: false,
            searching: false,
            editable: false,
            page: 0,
            page_count: 0,
            size: [1., 1.],
            page_texture: None,
            canvas_zoom: 1.,
            pan: false,
            template_texture: None,
            preview_texture: None,
            rect: None,
            drag_start: None,
            target_pages: String::new(),
            sensitivity: 0.9,
            scale: 1.,
            quarter_turns: true,
            progress: None,
            report: None,
            selected: BTreeSet::new(),
            reviewed: false,
            group: String::new(),
            marker_size: 12.,
            error: None,
            offset: 0,
        }
    }
}

impl VisualSearchState {
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = false;
        self.searching = false;
        self.progress = None;
        self.report = None;
        self.selected.clear();
        self.reviewed = false;
        self.preview_texture = None;
        self.template_texture = None;
        self.error = None;
        self.offset = 0;
    }
    fn can_apply(&self) -> bool {
        self.editable
            && !self.pending
            && !self.searching
            && self.reviewed
            && !self.selected.is_empty()
            && !self.group.trim().is_empty()
            && self
                .report
                .as_ref()
                .is_some_and(|r| r.progress.status.is_terminal())
    }
}

fn parse_pages(text: &str, total: usize) -> Result<Vec<usize>, ()> {
    if text.len() > 256 {
        return Err(());
    }
    let mut pages = BTreeSet::new();
    for part in text.split(',') {
        let part = part.trim();
        let (first, last) = if let Some((a, b)) = part.split_once('-') {
            (
                a.trim().parse::<usize>().map_err(|_| ())?,
                b.trim().parse::<usize>().map_err(|_| ())?,
            )
        } else {
            let n = part.parse::<usize>().map_err(|_| ())?;
            (n, n)
        };
        if first == 0 || first > last || last > total || last - first >= MAX_SCAN_PAGES {
            return Err(());
        }
        for page in first..=last {
            pages.insert(page - 1);
            if pages.len() > MAX_SCAN_PAGES {
                return Err(());
            }
        }
    }
    if pages.is_empty() {
        Err(())
    } else {
        Ok(pages.into_iter().collect())
    }
}

fn point_to_pdf(point: egui::Pos2, image: egui::Rect, size: [f32; 2]) -> [f32; 2] {
    [
        ((point.x - image.left()) / image.width()).clamp(0., 1.) * size[0],
        ((image.bottom() - point.y) / image.height()).clamp(0., 1.) * size[1],
    ]
}

fn selection_rect(bounds: PointRect, image: egui::Rect, size: [f32; 2]) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            image.left() + bounds.left / size[0] * image.width(),
            image.bottom() - bounds.top / size[1] * image.height(),
        ),
        egui::pos2(
            image.left() + bounds.right / size[0] * image.width(),
            image.bottom() - bounds.bottom / size[1] * image.height(),
        ),
    )
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

fn status_label(lang: Language, status: VisualSearchStatus) -> &'static str {
    tr(
        lang,
        match status {
            VisualSearchStatus::Running => "visual_search.running",
            VisualSearchStatus::Complete => "visual_search.complete",
            VisualSearchStatus::Cancelled => "visual_search.cancelled",
            VisualSearchStatus::ComparisonLimit => "visual_search.comparison_limit",
            VisualSearchStatus::CandidateLimit => "visual_search.candidate_limit",
        },
    )
}

impl SuniApp {
    pub(crate) fn open_visual_search(&mut self) {
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
        let page_count = doc.info.page_count();
        let generation = self.visual_search.generation.wrapping_add(1);
        self.visual_search = VisualSearchState {
            open: true,
            epoch: self.view.epoch,
            generation,
            pending: true,
            page,
            page_count,
            target_pages: (page + 1).to_string(),
            group: tr(self.lang(), "visual_search.default_group").into(),
            ..Default::default()
        };
        self.annotate.register.open = false;
        self.annotate.quantities.open = false;
        self.worker.send(Request::VisualSearch {
            epoch: self.view.epoch,
            generation,
            job: Job::Page { page },
        });
    }

    pub(crate) fn close_visual_search(&mut self) {
        self.visual_search.invalidate();
        self.visual_search.open = false;
        self.visual_search.page_texture = None;
        self.worker.send(Request::VisualSearch {
            epoch: self.view.epoch,
            generation: self.visual_search.generation,
            job: Job::Discard,
        });
    }

    pub(crate) fn on_visual_search(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        let state = &mut self.visual_search;
        if !state.open
            || state.epoch != epoch
            || epoch != self.view.epoch
            || state.generation != generation
        {
            return;
        }
        match result {
            Ok(Outcome::Page {
                page,
                size,
                image,
                editable,
            }) => {
                state.pending = false;
                state.page = page;
                state.size = size;
                state.editable = editable;
                state.page_texture = Some(texture(ctx, "visual-search-page", image));
            }
            Ok(Outcome::Started { template, progress }) => {
                state.pending = false;
                state.searching = true;
                state.progress = Some(progress);
                state.template_texture = Some(texture(ctx, "visual-search-template", template));
            }
            Ok(Outcome::Progress(progress)) => {
                state.progress = Some(progress);
            }
            Ok(Outcome::Finished(report)) => {
                state.pending = false;
                state.searching = false;
                state.progress = Some(report.progress.clone());
                state.report = Some(report);
                state.selected.clear();
                state.reviewed = false;
                state.offset = 0;
            }
            Ok(Outcome::Preview { id, image }) => {
                state.pending = false;
                state.preview_texture = Some((id, texture(ctx, "visual-search-match", image)));
            }
            Ok(Outcome::Discarded) => {}
            Err(error) => {
                state.pending = false;
                state.searching = false;
                state.progress = None;
                state.report = None;
                state.preview_texture = None;
                state.reviewed = false;
                state.selected.clear();
                state.error = Some(error.to_string());
            }
        }
    }

    pub(crate) fn visual_search_dialog(&mut self, ctx: &egui::Context) {
        if !self.visual_search.open {
            return;
        }
        if self.visual_search.epoch != self.view.epoch {
            self.close_visual_search();
            return;
        }
        let lang = self.lang();
        let width = (ctx.content_rect().width() - 32.).clamp(300., 960.);
        let state = &mut self.visual_search;
        let busy = state.pending || state.searching;
        let can_start = !busy && state.rect.is_some() && state.page_texture.is_some();
        let can_apply = state.can_apply() && !self.worker.has_pending_mutations() && !self.saving;
        let searching = state.searching;
        let mut changed = false;
        let mut start = false;
        let mut cancel = false;
        let mut close = false;
        let mut apply = false;
        let mut load = false;
        let mut preview = None;
        dialog::show(
            ctx,
            Spec::new(
                "visual-search",
                tr(lang, "visual_search.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "visual_search.help"))
            .width(width),
            |ui| {
                ui.label(tr(lang, "visual_search.scope"));
                if let Some(error) = &state.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(tr(lang, "visual_search.template"));
                        ui.label(tr(lang, "register.page"));
                        let mut page = state.page + 1;
                        if ui
                            .add(egui::DragValue::new(&mut page).range(1..=state.page_count.max(1)))
                            .changed()
                        {
                            state.page = page - 1;
                            load = true;
                        }
                        ui.label(tr(lang, "visual_search.pages"));
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut state.target_pages)
                                    .desired_width(110.)
                                    .char_limit(256),
                            )
                            .changed();
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "visual_search.sensitivity"));
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut state.sensitivity, 0.5..=1.)
                                    .fixed_decimals(2),
                            )
                            .changed();
                        changed |= ui
                            .checkbox(
                                &mut state.quarter_turns,
                                tr(lang, "visual_search.rotations"),
                            )
                            .changed();
                        ui.label(tr(lang, "visual_search.resolution"));
                        egui::ComboBox::from_id_salt("symbol-search-resolution")
                            .selected_text(format!("{} dpi", (state.scale * 72.) as u32))
                            .show_ui(ui, |ui| {
                                for scale in [0.5, 1., 2.] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut state.scale,
                                            scale,
                                            format!("{} dpi", (scale * 72.) as u32),
                                        )
                                        .changed();
                                }
                            });
                    });
                    ui.small(tr(lang, "visual_search.select_help"));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr(lang, "visual_search.fit")).clicked() {
                            state.canvas_zoom = 1.;
                            state.drag_start = None;
                        }
                        ui.label(tr(lang, "visual_search.zoom"));
                        if ui
                            .add(
                                egui::Slider::new(&mut state.canvas_zoom, 1.0..=12.0)
                                    .fixed_decimals(1)
                                    .suffix("×"),
                            )
                            .changed()
                        {
                            state.drag_start = None;
                        }
                        if ui
                            .checkbox(&mut state.pan, tr(lang, "visual_search.pan"))
                            .changed()
                        {
                            state.drag_start = None;
                        }
                    });
                    if let Some(image) = &state.page_texture {
                        let fit = (ui.available_width() / state.size[0]).min(330. / state.size[1]);
                        let draw_size =
                            egui::vec2(state.size[0], state.size[1]) * fit * state.canvas_zoom;
                        let scroll_source = egui::scroll_area::ScrollSource {
                            drag: if state.pan {
                                egui::scroll_area::DragScroll::Always
                            } else {
                                egui::scroll_area::DragScroll::Never
                            },
                            ..Default::default()
                        };
                        egui::ScrollArea::both()
                            .id_salt("symbol-template-viewport")
                            .max_height(330.)
                            .auto_shrink([false, false])
                            .scroll_source(scroll_source)
                            .show(ui, |ui| {
                                let response = ui.add(
                                    egui::Image::new(image).fit_to_exact_size(draw_size).sense(
                                        if state.pan {
                                            egui::Sense::hover()
                                        } else {
                                            egui::Sense::drag()
                                        },
                                    ),
                                );
                                if let Some(rect) = state.rect {
                                    ui.painter_at(response.rect).rect_stroke(
                                        selection_rect(rect, response.rect, state.size),
                                        0.,
                                        egui::Stroke::new(2., egui::Color32::LIGHT_BLUE),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                if !state.pan && response.drag_started() {
                                    state.drag_start = response.interact_pointer_pos();
                                }
                                if !state.pan {
                                    if let (Some(a), Some(b)) =
                                        (state.drag_start, response.interact_pointer_pos())
                                    {
                                        ui.painter_at(response.rect).rect_stroke(
                                            egui::Rect::from_two_pos(a, b),
                                            0.,
                                            egui::Stroke::new(2., egui::Color32::LIGHT_BLUE),
                                            egui::StrokeKind::Inside,
                                        );
                                        if response.drag_stopped() {
                                            let a = point_to_pdf(a, response.rect, state.size);
                                            let b = point_to_pdf(b, response.rect, state.size);
                                            if (a[0] - b[0]).abs() >= 1.
                                                && (a[1] - b[1]).abs() >= 1.
                                            {
                                                state.rect = Some(PointRect {
                                                    left: a[0].min(b[0]),
                                                    bottom: a[1].min(b[1]),
                                                    right: a[0].max(b[0]),
                                                    top: a[1].max(b[1]),
                                                });
                                                changed = true;
                                            }
                                            state.drag_start = None;
                                        }
                                    }
                                }
                            });
                    }
                    if let Some(rect) = &mut state.rect {
                        ui.horizontal_wrapped(|ui| {
                            for (key, value) in [
                                ("zones.left", &mut rect.left),
                                ("zones.bottom", &mut rect.bottom),
                                ("zones.right", &mut rect.right),
                                ("zones.top", &mut rect.top),
                            ] {
                                ui.label(tr(lang, key));
                                changed |= ui
                                    .add(egui::DragValue::new(value).speed(0.5).max_decimals(2))
                                    .changed();
                            }
                        });
                    }
                });
                if let Some(progress) = &state.progress {
                    ui.separator();
                    ui.strong(status_label(lang, progress.status));
                    ui.label(format!(
                        "{} {}/{} · {} {}",
                        tr(lang, "visual_search.pages_scanned"),
                        progress.pages_scanned,
                        progress.total_pages,
                        progress.candidates,
                        tr(lang, "visual_search.matches")
                    ));
                    if state.searching {
                        ui.spinner();
                    }
                }
                if let Some(template) = &state.template_texture {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "visual_search.template"));
                        ui.add(egui::Image::new(template).max_size(egui::vec2(90., 90.)));
                    });
                }
                if let Some(report) = &state.report {
                    if report.progress.status != VisualSearchStatus::Complete {
                        ui.colored_label(crate::theme::ERROR, tr(lang, "visual_search.partial"));
                    }
                    ui.label(tr(lang, "visual_search.similarity_help"));
                    ui.label(tr(lang, "visual_search.existing_counts"));
                    ui.add_enabled_ui(!busy, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(tr(lang, "visual_search.select_all")).clicked() {
                                state.selected = report.candidates.iter().map(|c| c.id).collect();
                                state.reviewed = false;
                            }
                            if ui.button(tr(lang, "visual_search.select_none")).clicked() {
                                state.selected.clear();
                                state.reviewed = false;
                            }
                            ui.label(format!(
                                "{} / {}",
                                state.selected.len(),
                                report.candidates.len()
                            ));
                        });
                        for candidate in report.candidates.iter().skip(state.offset).take(25) {
                            ui.horizontal_wrapped(|ui| {
                                let mut selected = state.selected.contains(&candidate.id);
                                if ui
                                    .checkbox(
                                        &mut selected,
                                        format!(
                                            "#{} · {} {} · {:.1}% · {}°",
                                            candidate.id,
                                            tr(lang, "register.page"),
                                            candidate.page + 1,
                                            candidate.score * 100.,
                                            candidate.rotation.degrees()
                                        ),
                                    )
                                    .changed()
                                {
                                    if selected {
                                        state.selected.insert(candidate.id);
                                    } else {
                                        state.selected.remove(&candidate.id);
                                    }
                                    state.reviewed = false;
                                }
                                if ui.small_button(tr(lang, "visual_search.preview")).clicked() {
                                    preview = Some(candidate.id);
                                }
                            });
                        }
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_enabled(
                                    state.offset > 0,
                                    egui::Button::new(tr(lang, "visual_search.previous")),
                                )
                                .clicked()
                            {
                                state.offset = state.offset.saturating_sub(25);
                            }
                            if ui
                                .add_enabled(
                                    state.offset + 25 < report.candidates.len(),
                                    egui::Button::new(tr(lang, "visual_search.next")),
                                )
                                .clicked()
                            {
                                state.offset += 25;
                            }
                        });
                        if let Some((id, image)) = &state.preview_texture {
                            ui.label(format!("{} #{id}", tr(lang, "visual_search.preview")));
                            ui.add(
                                egui::Image::new(image)
                                    .max_size(egui::vec2(ui.available_width().min(360.), 260.)),
                            );
                        }
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr(lang, "visual_search.group"));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.group)
                                    .desired_width(200.)
                                    .char_limit(128),
                            );
                            ui.label(tr(lang, "visual_search.marker_size"));
                            ui.add(
                                egui::DragValue::new(&mut state.marker_size)
                                    .range(8.0..=72.0)
                                    .suffix(" pt"),
                            );
                        });
                        if !state.editable {
                            ui.label(tr(lang, "visual_search.read_only"));
                        }
                        ui.checkbox(&mut state.reviewed, tr(lang, "visual_search.reviewed"));
                    });
                }
            },
            |ui| {
                apply = ui
                    .add_enabled(
                        can_apply,
                        egui::Button::new(tr(lang, "visual_search.apply")),
                    )
                    .clicked();
                start = ui
                    .add_enabled(
                        can_start,
                        egui::Button::new(tr(lang, "visual_search.start")),
                    )
                    .clicked();
                if searching {
                    cancel = ui.button(tr(lang, "visual_search.cancel")).clicked();
                }
                close = ui.button(tr(lang, "visual_search.close")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if close {
            self.close_visual_search();
            return;
        }
        if changed || load {
            state.invalidate();
            self.worker.send(Request::VisualSearch {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Discard,
            });
        }
        if load {
            state.rect = None;
            state.canvas_zoom = 1.;
            state.pan = false;
            state.page_texture = None;
            state.pending = true;
            self.worker.send(Request::VisualSearch {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Page { page: state.page },
            });
        }
        if start && can_start && !load {
            let pages = match parse_pages(&state.target_pages, state.page_count) {
                Ok(pages) => pages,
                Err(()) => {
                    state.error = Some(tr(lang, "visual_search.bad_pages").into());
                    return;
                }
            };
            let Some(rect) = state.rect else {
                return;
            };
            state.invalidate();
            state.pending = true;
            let options = VisualSearchOptions {
                pages,
                scale: state.scale,
                sensitivity: state.sensitivity,
                rotations: if state.quarter_turns {
                    vec![Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270]
                } else {
                    vec![Rotation::R0]
                },
                ..Default::default()
            };
            self.worker.send(Request::VisualSearch {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Start {
                    page: state.page,
                    rect,
                    options,
                },
            });
        } else if cancel && state.searching {
            state.pending = true;
            self.worker.send(Request::VisualSearch {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Cancel,
            });
        } else if let Some(id) = preview {
            state.pending = true;
            self.worker.send(Request::VisualSearch {
                epoch: state.epoch,
                generation: state.generation,
                job: Job::Preview { id },
            });
        } else if apply && state.can_apply() {
            let request = EditOp::CommitVisualSearchCounts {
                epoch: state.epoch,
                generation: state.generation,
                ids: state.selected.iter().copied().collect(),
                group: state.group.trim().into(),
                size: state.marker_size,
            };
            state.open = false;
            state.page_texture = None;
            state.preview_texture = None;
            state.template_texture = None;
            self.worker.send(Request::Edit(request));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_page_ranges_are_explicit_bounded_and_deduplicated() {
        assert_eq!(parse_pages("3, 1-2,2", 8), Ok(vec![0, 1, 2]));
        for text in [
            "",
            "0",
            "1-1000000000",
            "2-1",
            "1,,2",
            "1-2-3",
            "17",
            "1-17",
        ] {
            assert!(parse_pages(text, 16).is_err(), "{text}");
        }
        assert_eq!(parse_pages("1-16", 100).unwrap().len(), MAX_SCAN_PAGES);
    }
    #[test]
    fn template_coordinates_use_displayed_bottom_left_and_clamp_to_page() {
        let image = egui::Rect::from_min_max(egui::pos2(20., 30.), egui::pos2(220., 430.));
        assert_eq!(
            point_to_pdf(egui::pos2(70., 330.), image, [100., 200.]),
            [25., 50.]
        );
        assert_eq!(
            point_to_pdf(egui::pos2(-20., 900.), image, [100., 200.]),
            [0., 0.]
        );
        let bounds = PointRect {
            left: 25.,
            bottom: 50.,
            right: 75.,
            top: 150.,
        };
        assert_eq!(
            selection_rect(bounds, image, [100., 200.]),
            egui::Rect::from_min_max(egui::pos2(70., 130.), egui::pos2(170., 330.))
        );
    }
    #[test]
    fn changing_search_settings_discards_review_and_old_generation() {
        let mut state = VisualSearchState {
            generation: 7,
            searching: true,
            pending: true,
            reviewed: true,
            ..Default::default()
        };
        state.selected.insert(4);
        state.invalidate();
        assert_eq!(state.generation, 8);
        assert!(
            !state.pending
                && !state.searching
                && !state.reviewed
                && state.selected.is_empty()
                && !state.can_apply()
        );
    }

    #[test]
    fn old_generation_or_document_discard_cannot_cancel_a_replacement_search() {
        let mut cache = WorkerCache {
            token: Some((7, 12)),
            running: true,
            ..Default::default()
        };
        assert!(!cache.discard(7, 11));
        assert_eq!(cache.token(), Some((7, 12)));
        assert!(cache.running);
        assert!(!cache.discard(6, 99));
        assert_eq!(cache.token(), Some((7, 12)));
        assert!(cache.discard(7, 13));
        assert!(cache.token().is_none() && !cache.running);
    }

    #[test]
    fn zoomed_and_scrolled_template_uses_the_same_pdf_coordinates() {
        let bounds = PointRect {
            left: 25.,
            bottom: 50.,
            right: 75.,
            top: 150.,
        };
        for zoom in [1., 4., 12.] {
            let image =
                egui::Rect::from_min_size(egui::pos2(-350., -900.), egui::vec2(200., 400.) * zoom);
            let screen = selection_rect(bounds, image, [100., 200.]);
            assert_eq!(
                point_to_pdf(screen.left_bottom(), image, [100., 200.]),
                [25., 50.]
            );
            assert_eq!(
                point_to_pdf(screen.right_top(), image, [100., 200.]),
                [75., 150.]
            );
        }
    }
    #[test]
    fn closed_and_old_document_results_cannot_reopen_the_tool() {
        let (mut app, requests, _responses) = crate::app::tests::test_app();
        app.open_visual_search();
        let (epoch, generation) = match requests.try_recv().unwrap() {
            Request::VisualSearch {
                epoch,
                generation,
                job: Job::Page { .. },
            } => (epoch, generation),
            _ => panic!("page preview request"),
        };
        app.close_visual_search();
        app.on_visual_search(
            &egui::Context::default(),
            epoch,
            generation,
            Err(EngineError::Other("late result".into())),
        );
        assert!(!app.visual_search.open && app.visual_search.error.is_none());
    }

    fn report(status: VisualSearchStatus) -> VisualSearchReport {
        VisualSearchReport {
            version: 1,
            source_sha256: "a".repeat(64),
            template_page: 0,
            template_rect: PointRect {
                left: 10.,
                bottom: 10.,
                right: 30.,
                top: 30.,
            },
            options: VisualSearchOptions::default(),
            progress: VisualSearchProgress {
                status,
                pages_scanned: usize::from(status == VisualSearchStatus::Complete),
                total_pages: 1,
                comparisons: 1000,
                candidates: 1,
            },
            candidates: vec![sunipdf_core::visual_search::VisualSearchCandidate {
                id: 1,
                page: 0,
                rect: PointRect {
                    left: 50.,
                    bottom: 50.,
                    right: 70.,
                    top: 70.,
                },
                rotation: Rotation::R0,
                score: 0.95,
            }],
        }
    }

    #[test]
    fn completed_or_partial_suggestions_require_explicit_selection_and_review() {
        for status in [
            VisualSearchStatus::Complete,
            VisualSearchStatus::Cancelled,
            VisualSearchStatus::ComparisonLimit,
            VisualSearchStatus::CandidateLimit,
        ] {
            let (mut app, _requests, _) = crate::app::tests::test_app();
            app.visual_search = VisualSearchState {
                open: true,
                epoch: app.view.epoch,
                generation: 9,
                editable: true,
                group: "Symbols".into(),
                searching: true,
                ..Default::default()
            };
            app.on_visual_search(
                &egui::Context::default(),
                app.view.epoch,
                9,
                Ok(Outcome::Finished(report(status))),
            );
            let state = &mut app.visual_search;
            assert!(
                !state.searching
                    && state.selected.is_empty()
                    && !state.reviewed
                    && !state.can_apply()
            );
            state.selected.insert(1);
            assert!(!state.can_apply());
            state.reviewed = true;
            assert!(state.can_apply());
            state.pending = true;
            assert!(!state.can_apply());
            state.pending = false;
            state.editable = false;
            assert!(!state.can_apply());
        }
    }

    #[test]
    fn new_document_epoch_rejects_completed_results() {
        let (mut app, _, _) = crate::app::tests::test_app();
        app.visual_search = VisualSearchState {
            open: true,
            epoch: app.view.epoch,
            generation: 3,
            ..Default::default()
        };
        let old_epoch = app.view.epoch;
        app.view.epoch = app.view.epoch.wrapping_add(1);
        app.on_visual_search(
            &egui::Context::default(),
            old_epoch,
            3,
            Ok(Outcome::Finished(report(VisualSearchStatus::Complete))),
        );
        assert!(app.visual_search.report.is_none());
        app.visual_search_dialog(&egui::Context::default());
        assert!(!app.visual_search.open);
    }

    #[test]
    fn failed_preview_clears_completed_results_and_acceptance() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        let completed = report(VisualSearchStatus::Complete);
        app.visual_search = VisualSearchState {
            open: true,
            epoch: app.view.epoch,
            generation: 4,
            editable: true,
            pending: true,
            searching: true,
            progress: Some(completed.progress.clone()),
            report: Some(completed),
            preview_texture: Some((
                1,
                texture(
                    &ctx,
                    "failed-preview",
                    RenderedPage {
                        page_index: 0,
                        width: 8,
                        height: 8,
                        rgba: vec![255; 8 * 8 * 4],
                    },
                ),
            )),
            selected: BTreeSet::from([1]),
            reviewed: true,
            group: "Symbols".into(),
            ..Default::default()
        };
        app.on_visual_search(
            &ctx,
            app.view.epoch,
            4,
            Err(EngineError::Other("source changed".into())),
        );
        let state = &app.visual_search;
        assert!(state.progress.is_none() && state.report.is_none());
        assert!(state.preview_texture.is_none());
        assert!(!state.pending && !state.searching && !state.reviewed && !state.can_apply());
        assert!(state.selected.is_empty());
        assert!(state.error.as_deref().unwrap().contains("source changed"));
    }

    #[test]
    fn bilingual_actions_fit_a_small_window_and_do_not_accept_suggestions() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.set_language(language);
            app.visual_search = VisualSearchState {
                open: true,
                epoch: app.view.epoch,
                editable: true,
                page_count: 1,
                size: [200., 200.],
                canvas_zoom: 8.,
                pan: true,
                target_pages: "1".into(),
                report: Some(report(VisualSearchStatus::ComparisonLimit)),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            app.visual_search.page_texture = Some(texture(
                &ctx,
                "symbol-qa-page",
                RenderedPage {
                    page_index: 0,
                    width: 200,
                    height: 200,
                    rgba: vec![255; 200 * 200 * 4],
                },
            ));
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
                    |_| app.visual_search_dialog(&ctx),
                );
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in [
                    "visual_search.apply",
                    "visual_search.start",
                    "visual_search.close",
                    "visual_search.fit",
                    "visual_search.pan",
                ] {
                    let button = tree
                        .nodes
                        .iter()
                        .map(|(_, n)| n)
                        .find(|n| n.label() == Some(tr(language, key)))
                        .unwrap();
                    let b = button.bounds().unwrap();
                    assert!(
                        b.x0 >= 0. && b.x1 <= 720. && b.y0 >= 0. && b.y1 <= 480.,
                        "{key}: {b:?}"
                    );
                    if key == "visual_search.apply" {
                        assert!(button.is_disabled());
                    }
                }
            }
            assert!(app.visual_search.selected.is_empty() && !app.visual_search.reviewed);
            assert!(
                requests.try_recv().is_err(),
                "rendering never selects or creates counts"
            );
        }
    }
}
