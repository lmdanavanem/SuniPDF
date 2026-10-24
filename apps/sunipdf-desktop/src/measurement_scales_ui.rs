//! Persisted measurement regions, explicit recalculation previews, and vector snapping.

use crate::{
    app::{SuniApp, SCREEN_PER_PDF_POINT},
    dialog,
    measure_tools::measurement_panel_button,
    measurement_saved::MeasurementPageData,
    worker::{EditOp, Request},
};
use egui::{Color32, Painter, Pos2, Rect, Stroke};
use std::collections::{BTreeMap, BTreeSet};
use sunipdf_core::{
    measurement::{
        MeasurementKind, MeasurementViewport, PageCalibration, RecalibrationPlan,
        RecalibrationRequest, SnapKind, SnapPoint,
    },
    EngineError, PointRect,
};
use sunipdf_i18n::{tr, tr_args};

const MAX_BATCH_PAGES: usize = 512;
pub(crate) const PAGE_SCALE: &str = "*";
const SNAP_RADIUS_PX: f32 = 8.0;

pub(crate) struct RegionDraft {
    page: usize,
    points: Vec<(f32, f32)>,
    name: String,
    calibration: PageCalibration,
}

#[derive(Clone)]
struct BatchPage {
    records: Result<std::sync::Arc<[sunipdf_core::measurement::MeasurementRecord]>, String>,
    can_modify: bool,
    can_annotate: bool,
}

struct BatchDraft {
    epoch: u64,
    first: usize,
    last: usize,
    calibration: PageCalibration,
    explicit_reference: bool,
    recalculate: bool,
    pages: BTreeMap<usize, BatchPage>,
    pending: Option<u64>,
    plan: Option<RecalibrationPlan>,
}

pub(crate) struct ScaleToolsState {
    pub(crate) region: Option<RegionDraft>,
    pub(crate) selected: BTreeMap<usize, String>,
    known_region_pages: BTreeSet<usize>,
    pub(crate) snapped: Option<(usize, SnapPoint)>,
    pub(crate) snap_enabled: bool,
    batch: Option<BatchDraft>,
    applying: bool,
    remove_region: Option<(usize, String)>,
    reset_regions: Option<sunipdf_core::measurement::MeasurementViewportResetTarget>,
    error: Option<String>,
    generation: u64,
}

impl Default for ScaleToolsState {
    fn default() -> Self {
        Self {
            region: None,
            selected: BTreeMap::new(),
            known_region_pages: BTreeSet::new(),
            snapped: None,
            snap_enabled: true,
            batch: None,
            applying: false,
            remove_region: None,
            reset_regions: None,
            error: None,
            generation: 0,
        }
    }
}

impl ScaleToolsState {
    pub(crate) fn region_page(&self) -> Option<usize> {
        self.region.as_ref().map(|r| r.page)
    }
    pub(crate) fn needs_region_metadata(&self, page: usize) -> bool {
        self.known_region_pages.contains(&page)
            || self.selected.get(&page).is_some_and(|id| id != PAGE_SCALE)
    }
    pub(crate) fn dialog_open(&self) -> bool {
        self.batch.is_some()
            || self.applying
            || self.remove_region.is_some()
            || self.reset_regions.is_some()
    }
    pub(crate) fn cancel_region(&mut self) {
        self.region = None;
        self.snapped = None;
    }
    pub(crate) fn invalidate(&mut self) {
        self.region = None;
        self.batch = None;
        self.remove_region = None;
        self.reset_regions = None;
        self.snapped = None;
        self.generation = self.generation.wrapping_add(1);
        // Selection survives a content edit, but is revalidated against freshly read metadata.
    }
    pub(crate) fn accept_page(&mut self, page: usize, data: &MeasurementPageData) {
        if let Ok(viewports) = &data.viewports {
            if viewports.is_empty() {
                self.known_region_pages.remove(&page);
            } else {
                self.known_region_pages.insert(page);
            }
        }
        if let Some(batch) = self
            .batch
            .as_mut()
            .filter(|b| (b.first..=b.last).contains(&page))
        {
            batch.pages.insert(
                page,
                BatchPage {
                    records: data.records.clone(),
                    can_modify: data.can_modify,
                    can_annotate: data.can_annotate,
                },
            );
        }
        if let (Some(id), Ok(viewports)) = (self.selected.get(&page), &data.viewports) {
            if id != PAGE_SCALE && !viewports.iter().any(|v| &v.id == id) {
                self.selected.remove(&page);
            }
        }
    }
}

fn region_bounds(points: &[(f32, f32)]) -> Option<PointRect> {
    let [a, b] = points else { return None };
    if ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
        return None;
    }
    let bounds = PointRect {
        left: a.0.min(b.0),
        bottom: a.1.min(b.1),
        right: a.0.max(b.0),
        top: a.1.max(b.1),
    };
    (bounds.right - bounds.left >= 1.0 && bounds.top - bounds.bottom >= 1.0).then_some(bounds)
}

fn region_contains_calibration(bounds: PointRect, calibration: &PageCalibration) -> bool {
    calibration.points.iter().all(|p| {
        p.0 >= bounds.left && p.0 <= bounds.right && p.1 >= bounds.bottom && p.1 <= bounds.top
    })
}

impl SuniApp {
    pub(crate) fn snap_measurement_point(
        &mut self,
        page: usize,
        point: (f32, f32),
        shift: bool,
    ) -> (f32, f32) {
        let anchor = self.annotate.measure.points().last().copied();
        let point = crate::measure_tools::constrain(anchor, point, shift);
        let scale = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let state = &mut self.annotate.measure;
        state.scales_ui.snapped = None;
        if !state.scales_ui.snap_enabled || !scale.is_finite() || scale <= 0.0 {
            return point;
        }
        let Some(geometry) = state
            .saved
            .page_data(page)
            .and_then(|d| d.snap_geometry.as_ref().ok())
        else {
            return point;
        };
        let Some(hit) = geometry.nearest(point, SNAP_RADIUS_PX / scale) else {
            return point;
        };
        // Shift remains an exact horizontal/vertical constraint, even near another vector.
        if shift && crate::measure_tools::constrain(anchor, hit.point, true) != hit.point {
            return point;
        }
        state.scales_ui.snapped = Some((page, hit));
        hit.point
    }

    pub(crate) fn measurement_region_click(&mut self, page: usize, point: (f32, f32)) {
        let point = self.snap_measurement_point(page, point, false);
        let Some(region) = self.annotate.measure.scales_ui.region.as_mut() else {
            return;
        };
        if region.page == page && region.points.len() < 2 {
            region.points.push(point);
        }
    }

    fn start_measurement_region(&mut self, page: usize) {
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return;
        }
        let Some(calibration) = self
            .annotate
            .measure
            .local_calibration(page)
            .or_else(|| self.annotate.measure.effective_calibration(page))
        else {
            return;
        };
        let Some(data) = self.annotate.measure.saved.page_data(page) else {
            return;
        };
        if !data.can_modify || data.viewports.is_err() {
            return;
        }
        self.annotate.measure.scales_ui.region = Some(RegionDraft {
            page,
            points: Vec::new(),
            name: String::new(),
            calibration,
        });
        self.annotate.measure.scales_ui.error = None;
    }

    fn save_measurement_region(&mut self) {
        if !self.measurement_ready() {
            return;
        }
        let state = &self.annotate.measure;
        let Some(region) = state.scales_ui.region.as_ref() else {
            return;
        };
        let Some(bounds) = region_bounds(&region.points) else {
            return;
        };
        if !region_contains_calibration(bounds, &region.calibration) {
            return;
        }
        let name = region.name.trim();
        if name.is_empty() {
            return;
        }
        let Some(data) = state.saved.page_data(region.page) else {
            return;
        };
        if !data.can_modify {
            return;
        }
        let Ok(expected) = &data.viewports else {
            return;
        };
        let mut replacement = expected.to_vec();
        let id = (1u64..)
            .map(|n| format!("region-{n}"))
            .find(|id| replacement.iter().all(|v| &v.id != id))
            .unwrap();
        replacement.push(MeasurementViewport {
            id: id.clone(),
            name: name.into(),
            bounds,
            calibration: region.calibration.clone(),
        });
        let page = region.page;
        let expected = expected.to_vec();
        self.annotate.measure.scales_ui.selected.insert(page, id);
        self.annotate.measure.scales_ui.region = None;
        self.annotate.measure.scales_ui.applying = true;
        self.annotate.measure.scales_ui.error = None;
        self.worker
            .send(Request::Edit(EditOp::SetMeasurementViewports {
                page,
                expected,
                replacement,
            }));
    }

    fn replace_selected_region_scale(&mut self, page: usize) {
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return;
        }
        let state = &self.annotate.measure;
        let Some(calibration) = state.local_calibration(page) else {
            return;
        };
        let Some(id) = state.scales_ui.selected.get(&page) else {
            return;
        };
        let Some(data) = state.saved.page_data(page).filter(|d| d.can_modify) else {
            return;
        };
        let Ok(expected) = &data.viewports else {
            return;
        };
        let mut replacement = expected.to_vec();
        let Some(region) = replacement.iter_mut().find(|v| &v.id == id) else {
            return;
        };
        if !region_contains_calibration(region.bounds, &calibration) {
            self.annotate.measure.scales_ui.error =
                Some(tr(self.lang(), "measure.viewport.reference_inside").into());
            return;
        }
        region.calibration = calibration;
        let expected = expected.to_vec();
        self.annotate.measure.scales_ui.applying = true;
        self.annotate.measure.scales_ui.error = None;
        self.worker
            .send(Request::Edit(EditOp::SetMeasurementViewports {
                page,
                expected,
                replacement,
            }));
    }

    pub(crate) fn measurement_advanced_scale_changed(&mut self) {
        if self.annotate.measure.scales_ui.applying {
            self.annotate.measure.scales_ui.applying = false;
            self.annotate.measure.scales_ui.error = None;
            self.edit.pending_status = Some(tr(self.lang(), "measure.batch.applied").into());
        }
    }

    pub(crate) fn measurement_advanced_scale_failed(&mut self, error: &EngineError) {
        if self.annotate.measure.scales_ui.applying {
            self.annotate.measure.scales_ui.applying = false;
            self.annotate.measure.scales_ui.error = Some(error.to_string());
        }
    }

    pub(crate) fn measurement_advanced_scale_controls(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        let Some(page) = self.document.as_ref().map(|d| d.page) else {
            return;
        };
        let lang = self.lang();
        let ready = self.measurement_ready();
        let idle = ready && !self.annotate.measure.is_active_draft();
        ui.separator();
        ui.strong(tr(lang, "measure.snap.title"));
        ui.add_enabled_ui(ready, |ui| {
            ui.checkbox(
                &mut self.annotate.measure.scales_ui.snap_enabled,
                tr(lang, "measure.snap.enable"),
            );
        });
        let snap_key = if !self.annotate.measure.scales_ui.snap_enabled {
            "measure.snap.disabled"
        } else if let Some((_, hit)) = self.annotate.measure.scales_ui.snapped {
            match hit.kind {
                SnapKind::Endpoint => "measure.snap.endpoint",
                SnapKind::Intersection => "measure.snap.intersection",
            }
        } else {
            "measure.snap.help"
        };
        ui.small(tr(lang, snap_key));
        if let Some(Ok(geometry)) = self
            .annotate
            .measure
            .saved
            .page_data(page)
            .map(|d| &d.snap_geometry)
        {
            if geometry.is_empty() {
                ui.small(tr(lang, "measure.snap.empty"));
            }
            if geometry.skipped_clipped_objects > 0 {
                ui.small(tr(lang, "measure.snap.clipped"));
            }
        }
        if let Some(Err(error)) = self
            .annotate
            .measure
            .saved
            .page_data(page)
            .map(|d| &d.snap_geometry)
        {
            ui.small(tr(lang, "measure.snap.unavailable"))
                .on_hover_text(error);
        }
        ui.separator();
        ui.strong(tr(lang, "measure.viewport.title"));
        let data = self.annotate.measure.saved.page_data(page).cloned();
        let regions = data
            .as_ref()
            .and_then(|d| d.viewports.as_ref().ok())
            .cloned();
        let mut selected = self.annotate.measure.scales_ui.selected.get(&page).cloned();
        let label = if selected.as_deref() == Some(PAGE_SCALE) {
            tr(lang, "measure.viewport.page_scale").to_owned()
        } else {
            selected
                .as_ref()
                .and_then(|id| regions.as_ref()?.iter().find(|r| &r.id == id))
                .map_or_else(
                    || tr(lang, "measure.viewport.automatic").to_owned(),
                    |r| r.name.clone(),
                )
        };
        ui.add_enabled_ui(idle, |ui| {
            egui::ComboBox::from_id_salt("measurement-region-selection")
                .width(ui.available_width())
                .truncate()
                .selected_text(label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected,
                        None,
                        tr(lang, "measure.viewport.automatic"),
                    );
                    ui.selectable_value(
                        &mut selected,
                        Some(PAGE_SCALE.into()),
                        tr(lang, "measure.viewport.page_scale"),
                    );
                    if let Some(regions) = &regions {
                        for region in regions.iter() {
                            ui.selectable_value(
                                &mut selected,
                                Some(region.id.clone()),
                                &region.name,
                            );
                        }
                    }
                });
        });
        match selected {
            Some(id) => {
                self.annotate.measure.scales_ui.selected.insert(page, id);
            }
            None => {
                self.annotate.measure.scales_ui.selected.remove(&page);
            }
        }
        ui.small(tr(lang, "measure.viewport.help"));
        if let Some(calibration) = self.annotate.measure.effective_calibration(page) {
            ui.small(format!(
                "{}: {} {} / {:.2} pt",
                tr(lang, "measure.viewport.active_scale"),
                calibration.known_distance,
                calibration.unit.id(),
                (calibration.points[1].0 - calibration.points[0].0)
                    .hypot(calibration.points[1].1 - calibration.points[0].1)
            ));
        }
        if let Some(Err(error)) = data.as_ref().map(|d| &d.viewports) {
            ui.colored_label(Color32::RED, error);
        }
        if let Some(data) = data.as_ref().filter(|data| data.viewports.is_err()) {
            if let Some(target) = &data.viewport_reset_target {
                if measurement_panel_button(
                    ui,
                    idle && data.can_modify,
                    tr(lang, "measure.viewport.reset"),
                )
                .clicked()
                {
                    self.annotate.measure.scales_ui.reset_regions = Some(target.clone());
                }
            }
        }
        let can_modify = data
            .as_ref()
            .is_some_and(|d| d.can_modify && d.viewports.is_ok());
        let has_scale = self.annotate.measure.effective_calibration(page).is_some();
        if measurement_panel_button(
            ui,
            idle && can_modify && has_scale,
            tr(lang, "measure.viewport.create"),
        )
        .clicked()
        {
            self.start_measurement_region(page);
        }
        if let Some(region) = self.annotate.measure.scales_ui.region.as_mut() {
            ui.label(tr(lang, "measure.viewport.draw"));
            let label = ui.label(tr(lang, "measure.viewport.name"));
            ui.add(
                egui::TextEdit::singleline(&mut region.name)
                    .char_limit(80)
                    .desired_width(ui.available_width()),
            )
            .labelled_by(label.id);
            ui.small(tr(lang, "measure.viewport.reference_inside"));
            let save = ready
                && region_bounds(&region.points)
                    .is_some_and(|bounds| region_contains_calibration(bounds, &region.calibration))
                && !region.name.trim().is_empty();
            if measurement_panel_button(ui, save, tr(lang, "measure.viewport.save")).clicked() {
                self.save_measurement_region();
            }
        } else {
            let selected = self
                .annotate
                .measure
                .scales_ui
                .selected
                .get(&page)
                .filter(|id| id.as_str() != PAGE_SCALE)
                .cloned();
            if selected.is_some() {
                if measurement_panel_button(
                    ui,
                    idle && can_modify && self.annotate.measure.local_calibration(page).is_some(),
                    tr(lang, "measure.viewport.update_scale"),
                )
                .clicked()
                {
                    self.replace_selected_region_scale(page);
                }
                ui.small(tr(lang, "measure.viewport.existing_unchanged"));
                if measurement_panel_button(
                    ui,
                    idle && can_modify,
                    tr(lang, "measure.viewport.remove"),
                )
                .clicked()
                {
                    self.annotate.measure.scales_ui.remove_region = selected.map(|id| (page, id));
                }
            }
        }
        if measurement_panel_button(
            ui,
            idle && has_scale && can_modify,
            tr(lang, "measure.batch.open"),
        )
        .clicked()
        {
            self.open_measurement_batch(page);
        }
        if measurement_panel_button(ui, idle && self.can_undo, tr(lang, "measure.batch.undo"))
            .clicked()
        {
            self.execute(ctx, sunipdf_core::Command::Undo);
        }
        if let Some(error) = &self.annotate.measure.scales_ui.error {
            ui.colored_label(Color32::RED, error);
        }
    }

    fn open_measurement_batch(&mut self, page: usize) {
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return;
        }
        let Some(calibration) = self.annotate.measure.effective_calibration(page) else {
            return;
        };
        self.annotate.measure.scales_ui.error = None;
        self.annotate.measure.scales_ui.batch = Some(BatchDraft {
            epoch: self.view.epoch,
            first: page,
            last: page,
            calibration,
            explicit_reference: self.annotate.measure.local_calibration(page).is_some(),
            recalculate: true,
            pages: BTreeMap::new(),
            pending: None,
            plan: None,
        });
    }

    fn read_measurement_batch_pages(&mut self) {
        let Some(batch) = self.annotate.measure.scales_ui.batch.as_ref() else {
            return;
        };
        if batch.pending.is_some() || batch.plan.is_some() {
            return;
        }
        let pages: Vec<_> = (batch.first..=batch.last)
            .filter(|p| !batch.pages.contains_key(p))
            .take(MAX_BATCH_PAGES)
            .collect();
        for page in pages {
            if let Some(data) = self.annotate.measure.saved.page_data(page).cloned() {
                self.annotate.measure.scales_ui.accept_page(page, &data);
            } else {
                self.request_measurement_page(page);
            }
        }
    }

    fn scale_dialog_mutation_ready(&self) -> bool {
        self.annotate.tool == crate::annotate::Tool::Measure
            && self.document.is_some()
            && matches!(self.engine, crate::app::EngineState::Ready)
            && !self.worker.has_pending_mutations()
            && !self.saving
            && !self.processing.busy
            && !self.office.busy
            && !self.sign.busy
            && !self.printing.busy
            && !self.ocr.busy
            && !self.certificate.busy
            && !self.review.busy
            && self.opening.is_none()
            && self.tabs.resuming.is_none()
            && self.tabs.suspending.is_none()
            && !self.document_transition_pending()
            && !self.forms.has_unsaved_input()
            && !self.forms.pending_clear
            && self.forms.pending_import.is_none()
    }

    fn preview_measurement_batch(&mut self) {
        if !self.scale_dialog_mutation_ready() {
            return;
        }
        let Some(batch) = self.annotate.measure.scales_ui.batch.as_ref() else {
            return;
        };
        if batch.epoch != self.view.epoch
            || batch.pending.is_some()
            || batch.first > batch.last
            || batch.last - batch.first >= MAX_BATCH_PAGES
        {
            return;
        }
        let mut requests = Vec::new();
        for page in batch.first..=batch.last {
            let Some(data) = batch.pages.get(&page) else {
                return;
            };
            if !data.can_modify || (batch.recalculate && !data.can_annotate) {
                return;
            }
            let Ok(records) = &data.records else {
                return;
            };
            if batch.recalculate
                && !batch.explicit_reference
                && records.iter().any(|record| {
                    !record.validity.is_current()
                        && record.measurement.kind != MeasurementKind::Angle
                })
            {
                return;
            }
            requests.push(RecalibrationRequest {
                page,
                calibration: batch.calibration.clone(),
                update_page_default: true,
                measurements: if batch.recalculate {
                    records
                        .iter()
                        .filter(|r| r.measurement.kind != MeasurementKind::Angle)
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                },
            });
        }
        self.annotate.measure.scales_ui.generation =
            self.annotate.measure.scales_ui.generation.wrapping_add(1);
        let generation = self.annotate.measure.scales_ui.generation;
        self.annotate
            .measure
            .scales_ui
            .batch
            .as_mut()
            .unwrap()
            .pending = Some(generation);
        self.annotate.measure.scales_ui.error = None;
        self.worker.send(Request::PreviewRecalibration {
            requests,
            epoch: self.view.epoch,
            generation,
        });
    }

    pub(crate) fn on_recalibration_preview(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<RecalibrationPlan, EngineError>,
    ) {
        let Some(batch) = self.annotate.measure.scales_ui.batch.as_mut() else {
            return;
        };
        if epoch != self.view.epoch || epoch != batch.epoch || batch.pending != Some(generation) {
            return;
        }
        batch.pending = None;
        match result {
            Ok(plan) => batch.plan = Some(plan),
            Err(error) => self.annotate.measure.scales_ui.error = Some(error.to_string()),
        }
    }

    pub(crate) fn measurement_scale_dialogs(&mut self, ctx: &egui::Context) {
        self.measurement_reset_regions_dialog(ctx);
        self.measurement_remove_region_dialog(ctx);
        if self
            .annotate
            .measure
            .scales_ui
            .batch
            .as_ref()
            .is_some_and(|b| b.epoch != self.view.epoch)
        {
            self.annotate.measure.scales_ui.batch = None;
        }
        self.read_measurement_batch_pages();
        let Some(batch) = self.annotate.measure.scales_ui.batch.as_ref() else {
            return;
        };
        let lang = self.lang();
        let count = self.document.as_ref().map_or(0, |d| d.info.page_count());
        let mut first = batch.first + 1;
        let mut last = batch.last + 1;
        let mut recalculate = batch.recalculate;
        let locked = batch.pending.is_some() || batch.plan.is_some();
        let can_preview = !locked
            && (batch.first..=batch.last).all(|p| {
                batch.pages.get(&p).is_some_and(|d| {
                    d.can_modify
                        && (!recalculate || d.can_annotate)
                        && d.records.as_ref().is_ok_and(|records| {
                            !recalculate
                                || batch.explicit_reference
                                || records.iter().all(|record| {
                                    record.validity.is_current()
                                        || record.measurement.kind == MeasurementKind::Angle
                                })
                        })
                })
            });
        let mut close = false;
        let mut preview = false;
        let mut apply = false;
        let mut revise = false;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "measurement-batch",
                tr(lang, "measure.batch.open"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(760.0_f32.min((ctx.content_rect().width() - 40.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "measure.batch.help"));
                let has_unverified = batch.pages.values().any(|data| {
                    data.records.as_ref().is_ok_and(|records| {
                        records.iter().any(|record| !record.validity.is_current())
                    })
                });
                if recalculate && has_unverified {
                    ui.colored_label(Color32::YELLOW, tr(lang, "measure.validity.batch_warning"));
                    if !batch.explicit_reference {
                        ui.small(tr(lang, "measure.validity.reference_required"));
                    }
                }
                ui.small(format!(
                    "{} {} / {:.2} pt",
                    batch.calibration.known_distance,
                    batch.calibration.unit.id(),
                    (batch.calibration.points[1].0 - batch.calibration.points[0].0)
                        .hypot(batch.calibration.points[1].1 - batch.calibration.points[0].1)
                ));
                ui.add_enabled_ui(!locked, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "measure.batch.from"));
                        ui.add(egui::DragValue::new(&mut first).range(1..=count.max(1)));
                        ui.label(tr(lang, "measure.batch.to"));
                        ui.add(egui::DragValue::new(&mut last).range(first..=count.max(first)));
                    });
                    ui.checkbox(&mut recalculate, tr(lang, "measure.batch.recalculate"));
                });
                ui.small(tr(lang, "measure.batch.regions_unchanged"));
                if batch.last.saturating_sub(batch.first) >= MAX_BATCH_PAGES {
                    ui.colored_label(Color32::RED, tr(lang, "measure.batch.limit"));
                }
                if batch.pending.is_some() {
                    ui.spinner();
                    ui.label(tr(lang, "measure.batch.preparing"));
                } else if let Some(plan) = &batch.plan {
                    ui.strong(tr_args(
                        lang,
                        "measure.batch.summary",
                        &[
                            ("pages", &plan.page_count().to_string()),
                            ("measurements", &plan.changes().len().to_string()),
                        ],
                    ));
                    ui.label(tr(lang, "measure.batch.confirm_help"));
                    egui::ScrollArea::vertical().max_height(280.0).show_rows(
                        ui,
                        24.0,
                        plan.changes().len(),
                        |ui, range| {
                            for change in &plan.changes()[range] {
                                ui.label(format!(
                                    "p. {} · #{} · {:.4}{} → {:.4} {}{}",
                                    change.page + 1,
                                    change.index,
                                    change.before,
                                    if change.before_validity.is_current() {
                                        String::new()
                                    } else {
                                        format!(
                                            " ({})",
                                            tr(lang, "measure.validity.historical_value")
                                        )
                                    },
                                    change.after,
                                    change.unit.id(),
                                    if change.kind == MeasurementKind::Area {
                                        "²"
                                    } else {
                                        ""
                                    }
                                ));
                            }
                        },
                    );
                } else {
                    ui.label(tr_args(
                        lang,
                        "measure.batch.loaded",
                        &[
                            ("loaded", &batch.pages.len().to_string()),
                            ("total", &(batch.last - batch.first + 1).to_string()),
                        ],
                    ));
                    for (page, data) in &batch.pages {
                        if !data.can_modify || (recalculate && !data.can_annotate) {
                            ui.colored_label(
                                Color32::RED,
                                format!(
                                    "p. {}: {}",
                                    page + 1,
                                    tr(lang, "measure.saved.restricted")
                                ),
                            );
                        }
                        if let Err(error) = &data.records {
                            ui.colored_label(Color32::RED, format!("p. {}: {error}", page + 1));
                        }
                    }
                }
                if let Some(error) = &self.annotate.measure.scales_ui.error {
                    ui.colored_label(Color32::RED, error);
                }
            },
            |ui| {
                close = ui.button(tr(lang, "register.close")).clicked();
                preview = ui
                    .add_enabled(
                        can_preview,
                        egui::Button::new(tr(lang, "measure.batch.preview")),
                    )
                    .clicked();
                revise = ui
                    .add_enabled(
                        batch.plan.is_some(),
                        egui::Button::new(tr(lang, "measure.batch.revise")),
                    )
                    .clicked();
                apply = ui
                    .add_enabled(
                        batch.plan.is_some() && self.scale_dialog_mutation_ready(),
                        egui::Button::new(tr(lang, "measure.batch.apply")),
                    )
                    .clicked();
            },
        );
        if close {
            self.annotate.measure.scales_ui.batch = None;
            return;
        }
        if let Some(batch) = self.annotate.measure.scales_ui.batch.as_mut() {
            let first = first.saturating_sub(1);
            let last = last.saturating_sub(1).max(first);
            if !locked
                && (first != batch.first || last != batch.last || recalculate != batch.recalculate)
            {
                batch.first = first;
                batch.last = last;
                batch.recalculate = recalculate;
                batch.pages.clear();
                self.annotate.measure.scales_ui.error = None;
                return;
            }
            if revise {
                batch.plan = None;
                return;
            }
        }
        if preview {
            self.preview_measurement_batch();
        }
        if apply && self.scale_dialog_mutation_ready() {
            let Some(batch) = self.annotate.measure.scales_ui.batch.take() else {
                return;
            };
            if batch.epoch != self.view.epoch {
                return;
            }
            if let Some(plan) = batch.plan {
                self.annotate.measure.scales_ui.applying = true;
                self.worker
                    .send(Request::Edit(EditOp::ApplyRecalibration(plan)));
            }
        }
    }

    fn measurement_reset_regions_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.annotate.measure.scales_ui.reset_regions.clone() else {
            return;
        };
        let lang = self.lang();
        let mut close = false;
        let mut reset = false;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "measurement-reset-regions",
                tr(lang, "measure.viewport.reset"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(480.0),
            |ui| {
                ui.label(tr(lang, "measure.viewport.reset_help"));
            },
            |ui| {
                close = ui.button(tr(lang, "register.close")).clicked();
                reset = ui
                    .add_enabled(
                        self.scale_dialog_mutation_ready(),
                        egui::Button::new(tr(lang, "measure.viewport.reset_confirm")),
                    )
                    .clicked();
            },
        );
        if close {
            self.annotate.measure.scales_ui.reset_regions = None;
        }
        if reset && self.scale_dialog_mutation_ready() {
            self.annotate.measure.scales_ui.reset_regions = None;
            self.annotate.measure.scales_ui.applying = true;
            self.annotate.measure.scales_ui.error = None;
            self.worker
                .send(Request::Edit(EditOp::ClearMeasurementViewports(target)));
        }
    }

    fn measurement_remove_region_dialog(&mut self, ctx: &egui::Context) {
        let Some((page, id)) = self.annotate.measure.scales_ui.remove_region.clone() else {
            return;
        };
        let lang = self.lang();
        let mut close = false;
        let mut remove = false;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "measurement-remove-region",
                tr(lang, "measure.viewport.remove"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(480.0),
            |ui| {
                ui.label(tr(lang, "measure.viewport.remove_help"));
            },
            |ui| {
                close = ui.button(tr(lang, "register.close")).clicked();
                remove = ui
                    .add_enabled(
                        self.scale_dialog_mutation_ready(),
                        egui::Button::new(tr(lang, "measure.viewport.remove")),
                    )
                    .clicked();
            },
        );
        if close {
            self.annotate.measure.scales_ui.remove_region = None;
        }
        if remove && self.scale_dialog_mutation_ready() {
            self.annotate.measure.scales_ui.remove_region = None;
            let Some(data) = self
                .annotate
                .measure
                .saved
                .page_data(page)
                .filter(|d| d.can_modify)
            else {
                return;
            };
            let Ok(expected) = &data.viewports else {
                return;
            };
            let replacement: Vec<_> = expected.iter().filter(|v| v.id != id).cloned().collect();
            if replacement.len() == expected.len() {
                return;
            }
            let expected = expected.to_vec();
            self.annotate.measure.scales_ui.applying = true;
            self.worker
                .send(Request::Edit(EditOp::SetMeasurementViewports {
                    page,
                    expected,
                    replacement,
                }));
        }
    }

    pub(crate) fn paint_measurement_scales(
        &self,
        painter: &Painter,
        rect: Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let screen = |point: (f32, f32)| {
            Pos2::new(
                rect.left() + point.0 * k,
                rect.top() + (page_h_pt - point.1) * k,
            )
        };
        let state = &self.annotate.measure;
        if let Some(regions) = state
            .saved
            .page_data(page)
            .and_then(|d| d.viewports.as_ref().ok())
        {
            for region in regions.iter() {
                let selected = state.scales_ui.selected.get(&page) == Some(&region.id);
                let color = if selected {
                    Color32::from_rgb(45, 150, 230)
                } else {
                    Color32::from_rgba_unmultiplied(70, 140, 190, 100)
                };
                let bounds = region.bounds;
                let area = Rect::from_two_pos(
                    screen((bounds.left, bounds.bottom)),
                    screen((bounds.right, bounds.top)),
                );
                painter.rect_stroke(
                    area,
                    0.0,
                    Stroke::new(if selected { 2.0 } else { 1.0 }, color),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    area.left_top() + egui::vec2(5.0, 5.0),
                    egui::Align2::LEFT_TOP,
                    &region.name,
                    egui::FontId::proportional(12.0),
                    color,
                );
            }
        }
        if let Some(region) = state.scales_ui.region.as_ref().filter(|r| r.page == page) {
            let mut points = region.points.clone();
            if points.len() == 1 {
                if let Some((p, point)) = state.hover {
                    if p == page {
                        points.push(point);
                    }
                }
            }
            if let Some(bounds) = region_bounds(&points) {
                painter.rect_stroke(
                    Rect::from_two_pos(
                        screen((bounds.left, bounds.bottom)),
                        screen((bounds.right, bounds.top)),
                    ),
                    0.0,
                    Stroke::new(2.0, Color32::LIGHT_BLUE),
                    egui::StrokeKind::Inside,
                );
            }
            for point in &region.points {
                painter.circle_filled(screen(*point), 4.0, Color32::LIGHT_BLUE);
            }
        }
        if let Some((_, hit)) = state.scales_ui.snapped.filter(|(p, _)| *p == page) {
            let at = screen(hit.point);
            let color = Color32::from_rgb(35, 190, 130);
            painter.rect_stroke(
                Rect::from_center_size(at, egui::vec2(12.0, 12.0)),
                1.0,
                Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
            painter.text(
                at + egui::vec2(11.0, -11.0),
                egui::Align2::LEFT_BOTTOM,
                tr(
                    self.lang(),
                    match hit.kind {
                        SnapKind::Endpoint => "measure.snap.endpoint",
                        SnapKind::Intersection => "measure.snap.intersection",
                    },
                ),
                egui::FontId::proportional(12.0),
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_requires_two_finite_separated_corners_and_normalizes_direction() {
        assert!(region_bounds(&[]).is_none());
        assert!(region_bounds(&[(0.0, 0.0), (0.5, 2.0)]).is_none());
        assert!(region_bounds(&[(f32::NAN, 0.0), (20.0, 20.0)]).is_none());
        assert_eq!(
            region_bounds(&[(40.0, 90.0), (10.0, 20.0)]),
            Some(PointRect {
                left: 10.0,
                bottom: 20.0,
                right: 40.0,
                top: 90.0
            })
        );
    }
    fn calibration(distance: f64) -> PageCalibration {
        PageCalibration {
            points: [(20.0, 20.0), (100.0, 20.0)],
            known_distance: distance,
            unit: sunipdf_core::measurement::MeasurementUnit::Metres,
        }
    }

    fn ready() -> (SuniApp, std::sync::mpsc::Receiver<Request>) {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.open_measurements(&egui::Context::default());
        app.validate_measurement_context();
        assert!(app.annotate.measure.use_calibration(0, &calibration(8.0)));
        (app, requests)
    }

    fn load(
        app: &mut SuniApp,
        requests: &std::sync::mpsc::Receiver<Request>,
        page: usize,
        data: MeasurementPageData,
    ) {
        app.request_measurement_page(page);
        let Request::MeasurementPage {
            page: actual,
            epoch,
            generation,
        } = requests.recv().unwrap()
        else {
            panic!("page metadata")
        };
        assert_eq!(actual, page);
        app.on_measurement_page(page, epoch, generation, Ok(data));
    }

    #[test]
    fn region_creation_waits_for_in_region_reference_and_sends_one_exact_snapshot_edit() {
        let (mut app, requests) = ready();
        load(
            &mut app,
            &requests,
            0,
            MeasurementPageData::new(Ok(None), Ok(vec![]), true),
        );
        app.start_measurement_region(0);
        assert!(app.annotate.measure.is_active_draft());
        app.measurement_region_click(0, (0.0, 0.0));
        app.measurement_region_click(0, (80.0, 80.0));
        app.annotate.measure.scales_ui.region.as_mut().unwrap().name = "Detail A".into();
        app.save_measurement_region();
        assert!(
            requests.try_recv().is_err(),
            "both calibration points must be inside the region"
        );
        app.annotate
            .measure
            .scales_ui
            .region
            .as_mut()
            .unwrap()
            .points[1] = (120.0, 80.0);
        app.save_measurement_region();
        let Request::Edit(EditOp::SetMeasurementViewports {
            page,
            expected,
            replacement,
        }) = requests.recv().unwrap()
        else {
            panic!("region edit")
        };
        assert_eq!(page, 0);
        assert!(expected.is_empty());
        assert_eq!(replacement.len(), 1);
        assert_eq!(replacement[0].name, "Detail A");
        assert_eq!(replacement[0].calibration, calibration(8.0));
        assert!(!app.annotate.measure.is_active_draft());
        assert!(app.annotate.measure.scales_ui.dialog_open());
    }

    #[test]
    fn protected_page_cannot_start_region_or_recalculation_mutation() {
        let (mut app, requests) = ready();
        load(
            &mut app,
            &requests,
            0,
            MeasurementPageData::new(Ok(None), Ok(vec![]), false),
        );
        app.start_measurement_region(0);
        assert!(app.annotate.measure.scales_ui.region.is_none());
        app.open_measurement_batch(0);
        app.read_measurement_batch_pages();
        app.preview_measurement_batch();
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn historical_batch_measurements_require_a_current_explicit_reference() {
        use sunipdf_core::measurement::{
            Measurement, MeasurementRecord, MeasurementUnit, MeasurementValidity,
        };
        for validity in [
            MeasurementValidity::LegacyUnverified,
            MeasurementValidity::Stale,
        ] {
            let (mut app, requests) = ready();
            let record = MeasurementRecord {
                index: 3,
                annotation_id: Some("historical-length".into()),
                validity,
                measurement: Measurement {
                    kind: MeasurementKind::Length,
                    points: vec![(20.0, 40.0), (60.0, 40.0)],
                    calibration: calibration(4.0).points,
                    known_distance: 4.0,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                },
            };
            load(
                &mut app,
                &requests,
                0,
                MeasurementPageData::new(
                    Ok(Some(calibration(4.0))),
                    Ok(vec![record.clone()]),
                    true,
                ),
            );
            app.annotate.measure.clear_temporary_scale(0);
            app.open_measurement_batch(0);
            app.read_measurement_batch_pages();
            app.preview_measurement_batch();
            assert!(requests.try_recv().is_err());
            app.annotate.measure.scales_ui.batch = None;
            assert!(app.annotate.measure.use_calibration(0, &calibration(8.0)));
            app.open_measurement_batch(0);
            app.read_measurement_batch_pages();
            app.preview_measurement_batch();
            assert!(
                matches!(requests.recv().unwrap(), Request::PreviewRecalibration { requests, .. }
                if requests[0].measurements == vec![record]
                    && requests[0].calibration == calibration(8.0))
            );
        }
    }

    #[test]
    fn batch_preview_freezes_explicit_records_and_discards_stale_replies() {
        use sunipdf_core::measurement::{Measurement, MeasurementRecord, MeasurementUnit};
        let (mut app, requests) = ready();
        let record = MeasurementRecord {
            validity: sunipdf_core::measurement::MeasurementValidity::Current,
            index: 3,
            annotation_id: Some("length-a".into()),
            measurement: Measurement {
                kind: MeasurementKind::Length,
                points: vec![(20.0, 40.0), (60.0, 40.0)],
                calibration: calibration(4.0).points,
                known_distance: 4.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            },
        };
        load(
            &mut app,
            &requests,
            0,
            MeasurementPageData::new(Ok(None), Ok(vec![record.clone()]), true),
        );
        app.open_measurement_batch(0);
        app.read_measurement_batch_pages();
        app.preview_measurement_batch();
        let Request::PreviewRecalibration {
            requests: batch,
            epoch,
            generation,
        } = requests.recv().unwrap()
        else {
            panic!("preview")
        };
        assert_eq!(batch.len(), 1);
        assert!(batch[0].update_page_default);
        assert_eq!(batch[0].calibration, calibration(8.0));
        assert_eq!(batch[0].measurements, vec![record]);
        app.on_recalibration_preview(
            epoch + 1,
            generation,
            Err(EngineError::Other("stale epoch".into())),
        );
        app.on_recalibration_preview(
            epoch,
            generation + 1,
            Err(EngineError::Other("stale token".into())),
        );
        assert!(app.annotate.measure.scales_ui.error.is_none());
        assert_eq!(
            app.annotate
                .measure
                .scales_ui
                .batch
                .as_ref()
                .unwrap()
                .pending,
            Some(generation)
        );
        app.on_recalibration_preview(
            epoch,
            generation,
            Err(EngineError::Other("current error".into())),
        );
        assert_eq!(
            app.annotate.measure.scales_ui.error.as_deref(),
            Some("engine error: current error")
        );
        app.annotate.measure.scales_ui.batch = None;
        app.on_recalibration_preview(
            epoch,
            generation,
            Err(EngineError::Other("late error".into())),
        );
        assert_eq!(
            app.annotate.measure.scales_ui.error.as_deref(),
            Some("engine error: current error")
        );
    }

    #[test]
    fn batch_reader_retains_pages_beyond_the_normal_eight_page_cache_and_bounds_pending_reads() {
        let (mut app, requests) = ready();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 12];
        load(
            &mut app,
            &requests,
            0,
            MeasurementPageData::new(Ok(None), Ok(vec![]), true),
        );
        app.open_measurement_batch(0);
        app.annotate.measure.scales_ui.batch.as_mut().unwrap().last = 11;
        for _ in 0..4 {
            app.read_measurement_batch_pages();
            let pending: Vec<_> = requests.try_iter().collect();
            assert!(pending.len() <= 4);
            for request in pending {
                let Request::MeasurementPage {
                    page,
                    epoch,
                    generation,
                } = request
                else {
                    panic!("read only")
                };
                app.on_measurement_page(
                    page,
                    epoch,
                    generation,
                    Ok(MeasurementPageData::new(Ok(None), Ok(vec![]), true)),
                );
            }
        }
        assert_eq!(
            app.annotate
                .measure
                .scales_ui
                .batch
                .as_ref()
                .unwrap()
                .pages
                .len(),
            12
        );
        app.annotate
            .measure
            .scales_ui
            .batch
            .as_mut()
            .unwrap()
            .recalculate = false;
        app.preview_measurement_batch();
        let Request::PreviewRecalibration { requests, .. } = requests.recv().unwrap() else {
            panic!("preview")
        };
        assert_eq!(requests.len(), 12);
        assert!(requests.iter().all(|r| r.measurements.is_empty()));
    }
}
