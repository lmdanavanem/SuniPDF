//! On-page calibrated measurements. Draft geometry never mutates the document.

use crate::{
    annotate::Tool,
    app::{EngineState, SuniApp, SCREEN_PER_PDF_POINT},
};
use egui::{Color32, Key, Modifiers, Painter, Pos2, Rect, Stroke};
use std::collections::BTreeMap;
use sunipdf_core::measurement::{
    Measurement, MeasurementDepth, MeasurementKind, MeasurementUnit, PageCalibration, MAX_HOLES,
    MAX_RING_VERTICES, MAX_TOTAL_VERTICES,
};
use sunipdf_i18n::{tr, tr_args};

type Point = (f32, f32);

#[derive(Clone, Copy)]
struct Scale {
    points: [Point; 2],
    distance: f64,
    unit: MeasurementUnit,
}

#[derive(Clone, Default)]
enum Gesture {
    #[default]
    Idle,
    Calibrate {
        page: Option<usize>,
        points: Vec<Point>,
    },
    Draw {
        page: usize,
        points: Vec<Point>,
    },
}

pub struct MeasurementState {
    pub(crate) scales_ui: crate::measurement_scales_ui::ScaleToolsState,
    pub(crate) saved: crate::measurement_saved::SavedMeasurements,
    pub(crate) geometry: Option<crate::measurement_geometry::GeometryDraft>,
    pub(crate) rectangle: crate::measurement_shapes_ui::RectangleToolsState,
    pub(crate) kind: MeasurementKind,
    pub(crate) unit: MeasurementUnit,
    pub(crate) precision: usize,
    pub(crate) input_distance: String,
    epoch: Option<u64>,
    scales: BTreeMap<usize, Scale>,
    gesture: Gesture,
    holes: Vec<Vec<Point>>,
    active_ring: Option<usize>,
    depth: MeasurementDepth,
    pub(crate) hover: Option<(usize, Point)>,
    error: Option<&'static str>,
}

impl Default for MeasurementState {
    fn default() -> Self {
        Self {
            scales_ui: Default::default(),
            saved: Default::default(),
            geometry: None,
            rectangle: Default::default(),
            kind: MeasurementKind::Length,
            unit: MeasurementUnit::Metres,
            precision: 2,
            input_distance: String::new(),
            epoch: None,
            scales: BTreeMap::new(),
            gesture: Gesture::Idle,
            holes: Vec::new(),
            active_ring: None,
            depth: MeasurementDepth {
                value: 1.0,
                unit: MeasurementUnit::Metres,
            },
            hover: None,
            error: None,
        }
    }
}

impl MeasurementState {
    pub fn is_active_draft(&self) -> bool {
        self.geometry.is_some()
            || self.rectangle.draft.is_some()
            || self.scales_ui.region.is_some()
            || !matches!(self.gesture, Gesture::Idle)
    }

    pub fn cancel(&mut self) -> bool {
        let active = self.is_active_draft();
        self.geometry = None;
        self.rectangle.draft = None;
        self.scales_ui.cancel_region();
        self.gesture = Gesture::Idle;
        self.holes.clear();
        self.active_ring = None;
        self.input_distance.clear();
        self.hover = None;
        self.error = None;
        active
    }

    /// Keep page scales only when the caller has proved page geometry is unchanged.
    pub(crate) fn advance_epoch(&mut self, epoch: u64) {
        self.cancel();
        self.saved.invalidate();
        self.scales_ui.invalidate();
        self.epoch = Some(epoch);
    }

    pub(crate) fn invalidate(&mut self, epoch: u64) {
        self.advance_epoch(epoch);
        self.scales.clear();
    }

    pub(crate) fn start_calibration(&mut self) {
        self.cancel();
        self.gesture = Gesture::Calibrate {
            page: None,
            points: Vec::new(),
        };
    }

    pub(crate) fn confirm_calibration(&mut self) -> bool {
        let Gesture::Calibrate {
            page: Some(page),
            points,
        } = &self.gesture
        else {
            return false;
        };
        let Ok(points) = <[Point; 2]>::try_from(points.as_slice()) else {
            return false;
        };
        let Ok(distance) =
            sunipdf_core::measurement::parse_measurement_distance(&self.input_distance, self.unit)
        else {
            self.error = Some("measure.scale_invalid");
            return false;
        };
        let scale = Scale {
            points,
            distance,
            unit: self.unit,
        };
        if measurement(
            MeasurementKind::Length,
            points.to_vec(),
            scale,
            self.unit,
            self.precision,
        )
        .value()
        .is_err()
        {
            self.error = Some("measure.scale_invalid");
            return false;
        }
        self.scales.insert(*page, scale);
        self.cancel();
        true
    }

    pub(crate) fn undo_point(&mut self) {
        if let Some(ring) = self.active_ring.and_then(|index| self.holes.get_mut(index)) {
            ring.pop();
            self.error = None;
            return;
        }
        match &mut self.gesture {
            Gesture::Calibrate { page, points } => {
                points.pop();
                if points.is_empty() {
                    *page = None;
                }
            }
            Gesture::Draw { points, .. } => {
                points.pop();
                if points.is_empty() {
                    self.gesture = Gesture::Idle;
                    self.holes.clear();
                    self.active_ring = None;
                }
            }
            Gesture::Idle => {}
        }
        self.error = None;
    }

    fn page(&self) -> Option<usize> {
        match self.gesture {
            Gesture::Calibrate { page, .. } => page,
            Gesture::Draw { page, .. } => Some(page),
            Gesture::Idle => None,
        }
        .or_else(|| self.scales_ui.region_page())
        .or_else(|| self.rectangle.draft.as_ref().map(|draft| draft.page))
    }

    pub(crate) fn points(&self) -> &[Point] {
        if let Some(ring) = self.active_ring.and_then(|index| self.holes.get(index)) {
            return ring;
        }
        match &self.gesture {
            Gesture::Calibrate { points, .. } | Gesture::Draw { points, .. } => points,
            Gesture::Idle => &[],
        }
    }

    fn result(&self, page: usize, points: Vec<Point>) -> Option<Measurement> {
        let mut holes = self.holes.clone();
        let outer = if let Some(index) = self.active_ring {
            *holes.get_mut(index)? = points;
            let Gesture::Draw { points, .. } = &self.gesture else {
                return None;
            };
            points.clone()
        } else {
            points
        };
        let points = outer;
        let scale = if self.kind == MeasurementKind::Angle {
            // Angle has no physical scale. Reuse real vertices to satisfy core geometry validation.
            Scale {
                points: [*points.first()?, *points.get(1)?],
                distance: 1.0,
                unit: self.unit,
            }
        } else {
            // Region selection accepts one contour. Valid deductions are strictly
            // inside this outer boundary and must never create connecting edges.
            self.scale_for_points(page, &points).ok()?
        };
        let mut result = measurement(self.kind, points, scale, self.unit, self.precision);
        result.holes = holes;
        result.depth = (self.kind == MeasurementKind::Volume).then_some(self.depth);
        Some(result)
    }

    fn ready_result(&self) -> Option<(usize, Measurement)> {
        let Gesture::Draw { page, .. } = &self.gesture else {
            return None;
        };
        let result = self.result(*page, self.points().to_vec())?;
        result.value().ok()?;
        Some((*page, result))
    }

    fn add_deduction(&mut self) -> bool {
        if !matches!(self.kind, MeasurementKind::Area | MeasurementKind::Volume)
            || self.holes.len() >= MAX_HOLES
            || self.ready_result().is_none()
        {
            return false;
        }
        self.active_ring = Some(self.holes.len());
        self.holes.push(Vec::new());
        self.hover = None;
        self.error = None;
        true
    }

    fn remove_deduction(&mut self) {
        if let Some(index) = self.active_ring.take() {
            self.holes.remove(index);
            self.hover = None;
            self.error = None;
        }
    }

    fn push_point(&mut self, page: usize, point: Point, shift: bool) -> bool {
        if self.page().is_some_and(|p| p != page) {
            self.error = Some("measure.same_page");
            return false;
        }
        let point = constrain(self.points().last().copied(), point, shift);
        if let Some(index) = self.active_ring {
            let total = self.holes.iter().map(Vec::len).sum::<usize>()
                + match &self.gesture {
                    Gesture::Draw { points, .. } => points.len(),
                    _ => 0,
                };
            let ring = &mut self.holes[index];
            if ring.len() >= MAX_RING_VERTICES || total >= MAX_TOTAL_VERTICES {
                self.error = Some("measure.deductions.limit");
                return false;
            }
            if ring
                .last()
                .is_some_and(|last| distance(*last, point) < 0.01)
            {
                return false;
            }
            ring.push(point);
            self.error = None;
            return true;
        }
        if !matches!(self.gesture, Gesture::Calibrate { .. }) && self.kind != MeasurementKind::Angle
        {
            let mut proposed = self.points().to_vec();
            proposed.push(point);
            if let Err(key) = self.scale_for_points(page, &proposed) {
                self.error = Some(key);
                return false;
            }
        }
        if matches!(self.gesture, Gesture::Idle) {
            self.gesture = Gesture::Draw {
                page,
                points: Vec::new(),
            };
        }
        let calibrating = matches!(self.gesture, Gesture::Calibrate { .. });
        if !calibrating
            && self.points().len() + self.holes.iter().map(Vec::len).sum::<usize>()
                >= MAX_TOTAL_VERTICES
        {
            self.error = Some("measure.deductions.limit");
            return false;
        }
        let max = if calibrating { 2 } else { 128 };
        match &mut self.gesture {
            Gesture::Calibrate { points, .. } | Gesture::Draw { points, .. } => {
                if points
                    .last()
                    .is_some_and(|last| distance(*last, point) < 0.01)
                {
                    return false;
                }
                if points.len() == max {
                    if !calibrating {
                        self.error = Some("measure.point_limit");
                    }
                    return false;
                }
                points.push(point);
                // The first calibration click pins the page independently of scrolling.
                if let Gesture::Calibrate { page: captured, .. } = &mut self.gesture {
                    *captured = Some(page);
                }
            }
            Gesture::Idle => unreachable!(),
        }
        self.error = None;
        true
    }

    fn scale(&self, page: usize) -> Option<Scale> {
        self.scales.get(&page).copied().or_else(|| {
            let calibration = self.saved.calibration(page)?;
            Some(Scale {
                points: calibration.points,
                distance: calibration.known_distance,
                unit: calibration.unit,
            })
        })
    }

    fn scale_for_points(&self, page: usize, points: &[Point]) -> Result<Scale, &'static str> {
        if self
            .scales_ui
            .selected
            .get(&page)
            .is_some_and(|id| id == crate::measurement_scales_ui::PAGE_SCALE)
        {
            return self.scale(page).ok_or("measure.no_scale");
        }
        if self.saved.page_data(page).is_none() && self.scales_ui.needs_region_metadata(page) {
            return Err("measure.viewport.unavailable");
        }
        if let Some(data) = self.saved.page_data(page) {
            let viewports = data
                .viewports
                .as_ref()
                .map_err(|_| "measure.viewport.unavailable")?;
            let selected = self.scales_ui.selected.get(&page).map(String::as_str);
            let region = sunipdf_core::measurement::select_viewport(viewports, points, selected)
                .map_err(|_| "measure.viewport.conflict")?;
            if let Some(region) = region {
                return Ok(Scale {
                    points: region.calibration.points,
                    distance: region.calibration.known_distance,
                    unit: region.calibration.unit,
                });
            }
        }
        self.scale(page).ok_or("measure.no_scale")
    }

    pub(crate) fn effective_calibration(&self, page: usize) -> Option<PageCalibration> {
        if let Gesture::Draw {
            page: drawing_page,
            points,
        } = &self.gesture
        {
            if *drawing_page == page {
                let scale = self.scale_for_points(page, points).ok()?;
                return Some(PageCalibration {
                    points: scale.points,
                    known_distance: scale.distance,
                    unit: scale.unit,
                });
            }
        }

        if let Some(id) = self
            .scales_ui
            .selected
            .get(&page)
            .filter(|id| id.as_str() != crate::measurement_scales_ui::PAGE_SCALE)
        {
            let data = self.saved.page_data(page)?;
            let viewport = data.viewports.as_ref().ok()?.iter().find(|v| &v.id == id)?;
            return Some(viewport.calibration.clone());
        }
        self.scale(page).map(|s| PageCalibration {
            points: s.points,
            known_distance: s.distance,
            unit: s.unit,
        })
    }

    pub(crate) fn local_calibration(&self, page: usize) -> Option<PageCalibration> {
        self.scales.get(&page).map(|scale| PageCalibration {
            points: scale.points,
            known_distance: scale.distance,
            unit: scale.unit,
        })
    }

    pub(crate) fn use_calibration(&mut self, page: usize, calibration: &PageCalibration) -> bool {
        let scale = Scale {
            points: calibration.points,
            distance: calibration.known_distance,
            unit: calibration.unit,
        };
        if measurement(
            MeasurementKind::Length,
            scale.points.to_vec(),
            scale,
            scale.unit,
            2,
        )
        .value()
        .is_err()
        {
            return false;
        }
        self.scales.insert(page, scale);
        self.unit = calibration.unit;
        self.error = None;
        true
    }

    pub(crate) fn saved_scale_applied(&mut self, page: usize) {
        self.scales.remove(&page);
    }

    pub(crate) fn clear_temporary_scale(&mut self, page: usize) -> bool {
        if self.scales.remove(&page).is_none() {
            return false;
        }
        if let Some(calibration) = self.saved.calibration(page) {
            self.unit = calibration.unit;
        }
        self.error = None;
        true
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (f64::from(a.0) - f64::from(b.0)).hypot(f64::from(a.1) - f64::from(b.1))
}

pub(crate) fn constrain(anchor: Option<Point>, point: Point, shift: bool) -> Point {
    match (anchor, shift) {
        (Some(a), true) if (point.0 - a.0).abs() >= (point.1 - a.1).abs() => (point.0, a.1),
        (Some(a), true) => (a.0, point.1),
        _ => point,
    }
}

fn measurement(
    kind: MeasurementKind,
    points: Vec<Point>,
    scale: Scale,
    unit: MeasurementUnit,
    precision: usize,
) -> Measurement {
    Measurement {
        holes: Vec::new(),
        depth: None,
        kind,
        points,
        calibration: scale.points,
        known_distance: scale.distance * scale.unit.metres() / unit.metres(),
        unit,
        precision,
    }
}

fn kind_key(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::Length => "measure.length",
        MeasurementKind::Polyline => "measure.polyline",
        MeasurementKind::Perimeter => "measure.perimeter",
        MeasurementKind::Area => "measure.area",
        MeasurementKind::Volume => "measure.volume",
        MeasurementKind::Angle => "measure.angle",
    }
}

impl SuniApp {
    pub(crate) fn open_measurements(&mut self, _ctx: &egui::Context) {
        if !self.ink_can_start() || self.form_builder.active() || self.edit.text_prompt.is_some() {
            return;
        }
        self.validate_measurement_context();
        self.cancel_placing();
        self.annotate.clear_reading();
        self.annotate.selection = None;
        self.annotate.drag = None;
        self.edit.selection = None;
        self.annotate.adv.selected = None;
        self.annotate.adv.drag = None;
        self.annotate.adv.ink.clear();
        self.annotate.annotations_page = None;
        self.annotate.tool = Tool::Measure;
        crate::workspace::follow(
            &mut self.annotate,
            &mut self.organize,
            &mut self.edit,
            sunipdf_core::Command::ToolMeasure,
        );
    }

    pub(crate) fn cancel_measurement(&mut self) -> bool {
        self.annotate.measure.cancel()
    }

    pub(crate) fn validate_measurement_context(&mut self) {
        let state = &mut self.annotate.measure;
        if state.epoch != Some(self.view.epoch) {
            let had_scale = !state.scales.is_empty();
            state.invalidate(self.view.epoch);
            if had_scale {
                state.error = Some("measure.stale");
            }
        }
        if state.geometry.as_ref().is_some_and(|draft| {
            draft.epoch != self.view.epoch
                || !self
                    .document
                    .as_ref()
                    .is_some_and(|doc| doc.page == draft.page && draft.page < doc.info.page_count())
                || !state.saved.can_edit(draft.page, &draft.original)
        }) {
            state.cancel();
            state.error = Some("measure.stale");
        }
        if state.rectangle.draft.as_ref().is_some_and(|draft| {
            draft.epoch != self.view.epoch
                || !self
                    .document
                    .as_ref()
                    .is_some_and(|doc| doc.page == draft.page)
        }) {
            state.cancel();
            state.error = Some("measure.stale");
        }
    }

    pub(crate) fn measurement_ready(&self) -> bool {
        self.annotate.tool == Tool::Measure
            && self.document.is_some()
            && matches!(self.engine, EngineState::Ready)
            && !self.dialog_open()
            && !self.palette.open
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && !self.document_transition_pending()
            && !self.review.busy
            && !self.processing.busy
            && !self.office.busy
            && !self.sign.busy
            && !self.saving
            && !self.printing.busy
            && !self.ocr.busy
            && !self.certificate.busy
            && !self.form_builder.active()
            && self.edit.text_prompt.is_none()
            && !self.forms.has_unsaved_input()
            && !self.forms.pending_clear
            && self.forms.pending_import.is_none()
            && self.annotate.pending_markup.is_none()
            && self.edit.pending_copy.is_none()
    }

    pub(crate) fn measurement_click(
        &mut self,
        ctx: &egui::Context,
        page: usize,
        point: Point,
        shift: bool,
        double_click: bool,
    ) {
        self.validate_measurement_context();
        if !self.measurement_ready() || self.annotate.measure.geometry.is_some() {
            return;
        }
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
        else {
            return;
        };
        if !point.0.is_finite()
            || !point.1.is_finite()
            || point.0 < 0.0
            || point.1 < 0.0
            || point.0 > size.width_pt
            || point.1 > size.height_pt
        {
            return;
        }
        if self
            .annotate
            .measure
            .page()
            .is_some_and(|captured| captured != page)
        {
            self.annotate.measure.error = Some("measure.same_page");
            return;
        }
        if self.annotate.measure.scales_ui.region.is_some() {
            self.measurement_region_click(page, point);
            self.make_current(page);
            return;
        }
        let point = self.snap_measurement_point(page, point, shift);
        if self.annotate.measure.rectangle.draft.is_some() {
            self.make_current(page);
            self.place_dimensioned_rectangle(page, point);
            return;
        }
        if double_click && matches!(self.annotate.measure.gesture, Gesture::Draw { .. }) {
            // The first release already added the final vertex. Mouse jitter on the
            // second release must not create a tiny extra segment or closing edge.
            self.make_current(page);
            self.finish_measurement(ctx);
            return;
        }
        if self.annotate.measure.push_point(page, point, shift) {
            self.make_current(page);
        }
        let state = &self.annotate.measure;
        if matches!(&state.gesture, Gesture::Draw { points, .. }
            if (state.kind == MeasurementKind::Length && points.len() == 2)
            || (state.kind == MeasurementKind::Angle && points.len() == 3))
        {
            self.finish_measurement(ctx);
        }
    }

    pub(crate) fn measurement_hover(&mut self, page: usize, point: Option<Point>, shift: bool) {
        if self.annotate.tool != Tool::Measure || self.annotate.measure.geometry.is_some() {
            return;
        }
        if point.is_some() {
            self.request_measurement_page(page);
        }
        let point = point.map(|point| self.snap_measurement_point(page, point, shift));
        if point.is_none() {
            self.annotate.measure.scales_ui.snapped = None;
        }
        let state = &mut self.annotate.measure;
        if state.page().is_some_and(|p| p != page) {
            return;
        }
        if let Some(point) = point {
            state.hover = Some((
                page,
                constrain(state.points().last().copied(), point, shift),
            ));
        } else if state.hover.is_some_and(|(p, _)| p == page) {
            state.hover = None;
        }
    }

    pub(crate) fn finish_measurement(&mut self, _ctx: &egui::Context) {
        self.validate_measurement_context();
        if !self.measurement_ready() {
            return;
        }
        let Some((page, measurement)) = self.annotate.measure.ready_result() else {
            self.annotate.measure.error = Some("measure.invalid");
            return;
        };
        let gesture = self.annotate.measure.gesture.clone();
        let holes = self.annotate.measure.holes.clone();
        let active_ring = self.annotate.measure.active_ring;
        self.annotate.measure.cancel();
        if !self.open_measurement_preview(page, measurement) {
            self.annotate.measure.gesture = gesture;
            self.annotate.measure.holes = holes;
            self.annotate.measure.active_ring = active_ring;
        }
    }

    pub(crate) fn measurement_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.annotate.tool != Tool::Measure {
            return;
        }
        self.validate_measurement_context();
        self.request_measurement_page_if_needed();
        if self.annotate.measure.geometry.is_some() {
            self.measurement_geometry_bar(ctx, ui);
            return;
        }
        let lang = self.lang();
        let ready = self.measurement_ready();
        let popup_was_open = egui::Popup::is_any_open(ctx);
        let page = self
            .annotate
            .measure
            .page()
            .or_else(|| self.document.as_ref().map(|d| d.page));
        let palette = crate::theme::palette_for(ctx);
        let mut finish = false;
        let mut capture_calibration = None;
        ui.scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.spacing_mut().item_spacing.y = 6.0;
            let state = &mut self.annotate.measure;
            ui.label(egui::RichText::new(tr(lang, "measure.panel.create")).strong().color(palette.text));
            ui.add_enabled_ui(ready, |ui| {
                let kind_label = ui.label(tr(lang, "measure.kind"));
                ui.add_enabled_ui(!state.is_active_draft(), |ui| {
                    let width = ui.available_width();
                    egui::ComboBox::from_id_salt("measure-on-page-kind")
                        .width(width)
                        .truncate()
                        .selected_text(tr(lang, kind_key(state.kind)))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.set_max_width(width);
                            for kind in MeasurementKind::ALL {
                                ui.selectable_value(
                                    &mut state.kind,
                                    kind,
                                    tr(lang, kind_key(kind)),
                                );
                            }
                        })
                        .response
                        .labelled_by(kind_label.id)
                        .on_hover_text(tr(lang, kind_key(state.kind)));
                });
                ui.columns(2, |columns| {
                    let unit_label = columns[0].label(tr(lang, "measure.unit"));
                    egui::ComboBox::from_id_salt("measure-on-page-unit")
                        .width(columns[0].available_width())
                        .selected_text(state.unit.id())
                        .show_ui(&mut columns[0], |ui| {
                            for unit in MeasurementUnit::ALL {
                                ui.selectable_value(&mut state.unit, unit, unit.id());
                            }
                        })
                        .response
                        .labelled_by(unit_label.id);
                    let precision_label = columns[1].label(tr(lang, "measure.precision"));
                    columns[1]
                        .add(egui::DragValue::new(&mut state.precision).range(0..=4))
                        .labelled_by(precision_label.id);
                });
                if state.kind == MeasurementKind::Volume {
                    crate::measurement_area_ui::depth_controls(ui, lang, &mut state.depth);
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.label(egui::RichText::new(tr(lang, "measure.panel.scale")).strong().color(palette.text));
            ui.add_enabled_ui(ready, |ui| {
                if measurement_panel_button(
                    ui,
                    !state.is_active_draft(),
                    tr(lang, "measure.calibrate"),
                )
                .clicked()
                {
                    state.start_calibration();
                }
                let confirm =
                    matches!(&state.gesture, Gesture::Calibrate { points, .. } if points.len() == 2);
                if confirm {
                    let distance_label = ui.label(tr(lang, "measure.known_distance"));
                    let distance = ui.horizontal(|ui| {
                        let input_width = (ui.available_width() - 42.0).max(40.0);
                        let distance = ui.add(
                            egui::TextEdit::singleline(&mut state.input_distance)
                                .desired_width(input_width)
                                .char_limit(64)
                                .hint_text(tr(lang, "measure.distance_format")),
                        );
                        ui.label(state.unit.id());
                        distance.labelled_by(distance_label.id)
                    }).inner;
                    if measurement_panel_button(ui, true, tr(lang, "measure.confirm_scale"))
                        .clicked()
                        || (distance.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        let calibrated_page = state.page();
                        if state.confirm_calibration() {
                            capture_calibration = calibrated_page.and_then(|page| {
                                state.local_calibration(page).map(|calibration| (page, calibration))
                            });
                        }
                    }
                }
                if let Some((page, scale)) = page.and_then(|p| state.effective_calibration(p).map(|s| (p, s))) {
                    ui.small(tr_args(
                        lang,
                        "measure.scale_summary",
                        &[
                            ("page", &(page + 1).to_string()),
                            (
                                "span",
                                &format!("{:.2}", distance(scale.points[0], scale.points[1])),
                            ),
                            ("distance", &scale.known_distance.to_string()),
                            ("unit", scale.unit.id()),
                        ],
                    ))
                    .on_hover_text(tr(lang, "measure.page_scope"));
                }
            });

            self.measurement_saved_scale_controls(ui);
            self.measurement_advanced_scale_controls(ctx, ui);
            if self.annotate.measure.kind != MeasurementKind::Volume
                || self.annotate.measure.rectangle.draft.is_some()
            {
                self.measurement_rectangle_controls(ui);
            }
            ui.add_space(4.0);
            ui.separator();
            ui.label(egui::RichText::new(tr(lang, "measure.panel.drawing")).strong().color(palette.text));
            let state = &mut self.annotate.measure;
            if matches!(state.kind, MeasurementKind::Area | MeasurementKind::Volume)
                && matches!(state.gesture, Gesture::Draw { .. }) {
                let previous = state.active_ring;
                crate::measurement_area_ui::ring_selector(ui, lang, "measure-drawing-ring", &mut state.active_ring, state.holes.len());
                if previous != state.active_ring { state.hover = None; state.error = None; }
                if measurement_panel_button(ui, ready && state.holes.len() < MAX_HOLES && state.ready_result().is_some(), tr(lang, "measure.deductions.add")).clicked() {
                    state.add_deduction();
                }
                if measurement_panel_button(ui, ready && state.active_ring.is_some(), tr(lang, "measure.deductions.remove")).clicked() {
                    state.remove_deduction();
                }
                ui.small(tr(lang, "measure.deductions.hint"));
                if let Some(result) = page.and_then(|page| state.result(page, state.points().to_vec())) {
                    crate::measurement_area_ui::area_summary(ui, lang, &result);
                }
            }
            let hint = match &state.gesture {
                _ if state.rectangle.draft.is_some() => "measure.rectangle.place_hint",
                _ if state.scales_ui.region.is_some() => "measure.viewport.draw",
                Gesture::Calibrate { points, .. } if points.len() == 2 => "measure.distance_hint",
                Gesture::Calibrate { .. } => "measure.calibration_hint",
                Gesture::Draw { .. } if state.kind == MeasurementKind::Angle => {
                    "measure.angle_hint"
                }
                Gesture::Draw { .. } => "measure.draw_hint",
                Gesture::Idle
                    if state.kind != MeasurementKind::Angle
                        && !page.is_some_and(|p| state.scale(p).is_some()) =>
                {
                    "measure.no_scale"
                }
                Gesture::Idle if state.kind == MeasurementKind::Angle => "measure.angle_hint",
                Gesture::Idle => "measure.ready_hint",
            };
            ui.add(egui::Label::new(egui::RichText::new(tr(lang, hint)).color(palette.text_muted)).wrap());
            if let Some(error) = state
                .error
                .filter(|error| *error != hint && *error != "measure.no_scale")
            {
                ui.colored_label(palette.error, tr(lang, error));
            }
            if measurement_panel_button(
                ui,
                ready && !state.points().is_empty(),
                tr(lang, "measure.undo_point"),
            )
            .clicked()
            {
                state.undo_point();
            }
            if measurement_panel_button(
                ui,
                ready && state.ready_result().is_some(),
                tr(lang, "measure.finish"),
            )
            .clicked()
            {
                finish = true;
            }
            // Cancellation only discards local points and remains available while work is busy.
            if measurement_panel_button(
                ui,
                state.is_active_draft(),
                tr(lang, "measure.cancel_draft"),
            )
            .clicked()
            {
                state.cancel();
                finish = false;
            }
            if ready
                && !popup_was_open
                && !egui::Popup::is_any_open(ctx)
                && !ctx.egui_wants_keyboard_input()
            {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace)) {
                    state.undo_point();
                }
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                    finish = state.ready_result().is_some();
                }
            }
            if finish {
                self.finish_measurement(ctx);
            }
            ui.add_space(4.0);
            ui.separator();
            ui.label(egui::RichText::new(tr(lang, "measure.panel.saved")).strong().color(palette.text));
            self.measurement_saved_bar(ui);
            if measurement_panel_button(
                ui,
                ready && !self.annotate.measure.is_active_draft(),
                tr(lang, "quantity.title"),
            )
            .clicked()
            {
                self.open_quantity_table();
            }
        });
        if let Some((page, calibration)) = capture_calibration {
            self.worker
                .send(crate::worker::Request::CaptureRectangleCalibration { page, calibration });
        }
    }

    pub(crate) fn paint_measurement(
        &self,
        painter: &Painter,
        rect: Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        if self.annotate.tool != Tool::Measure {
            return;
        }
        self.paint_measurement_scales(painter, rect, page, page_h_pt);
        if self.annotate.measure.rectangle.draft.is_some() {
            self.paint_dimensioned_rectangle(painter, rect, page, page_h_pt);
            return;
        }
        if self.annotate.measure.geometry.is_some() {
            self.paint_measurement_geometry(painter, rect, page, page_h_pt);
            return;
        }
        let state = &self.annotate.measure;
        if state.epoch != Some(self.view.epoch) || state.page() != Some(page) {
            return;
        }
        let calibrating = matches!(state.gesture, Gesture::Calibrate { .. });
        let mut points = state.points().to_vec();
        let fixed = points.len();
        if let Some((_, hover)) = state.hover.filter(|(p, _)| *p == page) {
            if !points.is_empty()
                && points
                    .last()
                    .is_some_and(|last| distance(*last, hover) >= 0.01)
                && (!calibrating || fixed < 2)
                && fixed < 128
            {
                points.push(hover);
            }
        }
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let screen =
            |p: Point| Pos2::new(rect.left() + p.0 * k, rect.top() + (page_h_pt - p.1) * k);
        let color = if calibrating {
            Color32::from_rgb(20, 115, 225)
        } else {
            Color32::from_rgb(190, 30, 55)
        };
        let stroke = Stroke::new(2.0, color);
        if let Gesture::Draw { points: outer, .. } = &state.gesture {
            for (index, ring) in std::iter::once(outer).chain(state.holes.iter()).enumerate() {
                if index == state.active_ring.map_or(0, |i| i + 1) {
                    continue;
                }
                let muted = Stroke::new(1.5, color.gamma_multiply(0.65));
                for pair in ring.windows(2) {
                    painter.line_segment([screen(pair[0]), screen(pair[1])], muted);
                }
                if ring.len() >= 3 {
                    painter.line_segment([screen(*ring.last().unwrap()), screen(ring[0])], muted);
                }
            }
        }
        for pair in points.windows(2) {
            painter.line_segment([screen(pair[0]), screen(pair[1])], stroke);
        }
        if !calibrating
            && matches!(
                state.kind,
                MeasurementKind::Volume | MeasurementKind::Area | MeasurementKind::Perimeter
            )
            && points.len() >= 3
        {
            painter.line_segment([screen(*points.last().unwrap()), screen(points[0])], stroke);
        }
        for point in points.iter().take(fixed) {
            painter.circle_filled(screen(*point), 4.0, color);
        }
        let label = if calibrating {
            Some(
                tr(
                    self.lang(),
                    if fixed == 2 {
                        "measure.distance_hint"
                    } else {
                        "measure.calibration_hint"
                    },
                )
                .to_string(),
            )
        } else {
            state
                .result(page, points.clone())
                .and_then(|m| m.result().ok())
                .and_then(|v| v["label"].as_str().map(str::to_string))
        };
        if let (Some(label), Some(point)) = (label, points.last()) {
            let visible = rect.intersect(painter.clip_rect()).shrink(4.0);
            if !visible.is_positive() {
                return;
            }
            let galley = painter.layout(
                label,
                egui::FontId::proportional(14.0),
                color,
                visible.width().min(360.0),
            );
            let desired = screen(*point) + egui::vec2(10.0, -12.0);
            let position = Pos2::new(
                desired.x.clamp(
                    visible.left(),
                    (visible.right() - galley.size().x).max(visible.left()),
                ),
                desired.y.clamp(
                    visible.top(),
                    (visible.bottom() - galley.size().y).max(visible.top()),
                ),
            );
            let label_rect = Rect::from_min_size(position, galley.size()).expand(4.0);
            painter.rect_filled(label_rect, 3.0, Color32::from_white_alpha(235));
            painter.galley(position, galley, color);
        }
    }
}

pub(crate) fn measurement_panel_button(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized(
            [ui.available_width(), 30.0],
            egui::Button::new(label).wrap(),
        )
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibrated() -> MeasurementState {
        let mut state = MeasurementState::default();
        state.start_calibration();
        assert!(state.push_point(0, (10.0, 10.0), false));
        assert!(state.push_point(0, (110.0, 10.0), false));
        state.input_distance = "10".into();
        assert!(state.confirm_calibration());
        state
    }

    #[test]
    fn rectangle_controls_remain_discoverable_and_leave_volume_deductions_accessible() {
        for language in ["en-US", "fr-FR"] {
            let mut deduction_positions = Vec::new();
            for kind in MeasurementKind::ALL {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.settings.language = language.into();
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                app.open_measurements(&ctx);
                app.annotate.measure = calibrated();
                app.annotate.measure.kind = kind;
                app.annotate.measure.advance_epoch(app.view.epoch);
                if matches!(kind, MeasurementKind::Area | MeasurementKind::Volume) {
                    for point in [(20.0, 30.0), (120.0, 30.0), (120.0, 130.0), (20.0, 130.0)] {
                        assert!(app.annotate.measure.push_point(0, point, false));
                    }
                }
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 2400.0))),
                    ..Default::default()
                };
                for frame in 0..2 {
                    let mut output = ctx.run_ui(input.clone(), |ui| app.measurement_bar(&ctx, ui));
                    let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                    let has_rectangle = tree.nodes.iter().any(|(_, node)| {
                        node.label() == Some(tr(app.lang(), "measure.rectangle.place"))
                    });
                    assert_eq!(has_rectangle, kind != MeasurementKind::Volume, "{kind:?}");
                    if matches!(kind, MeasurementKind::Area | MeasurementKind::Volume) {
                        let add = tree
                            .nodes
                            .iter()
                            .map(|(_, node)| node)
                            .find(|node| {
                                node.label() == Some(tr(app.lang(), "measure.deductions.add"))
                            })
                            .unwrap();
                        assert!(!add.is_disabled());
                        if frame == 1 {
                            deduction_positions.push(add.bounds().unwrap().y0);
                        }
                    }
                    output.textures_delta.clear();
                }
            }
            assert!(deduction_positions[1] < deduction_positions[0]);

            let (mut app, _, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            app.open_measurements(&ctx);
            app.annotate.measure.kind = MeasurementKind::Volume;
            app.annotate.measure.rectangle.draft =
                Some(crate::measurement_shapes_ui::RectangleDraft {
                    page: 0,
                    epoch: app.view.epoch,
                    placement: crate::measurement_shapes_ui::RectanglePlacement {
                        anchor: (20.0, 30.0),
                        width: 2.0,
                        height: 3.0,
                        rotation_degrees: 0.0,
                        unit: MeasurementUnit::Metres,
                        precision: 2,
                        scale: crate::measurement_shapes_ui::RectangleScaleSelection::Automatic,
                    },
                });
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 2400.0))),
                    ..Default::default()
                },
                |ui| app.measurement_bar(&ctx, ui),
            );
            let tree = output.platform_output.accesskit_update.as_ref().unwrap();
            assert!(tree.nodes.iter().any(|(_, node)| {
                node.label() == Some(tr(app.lang(), "measure.rectangle.place"))
            }));
            let cancel = tree
                .nodes
                .iter()
                .map(|(_, node)| node)
                .find(|node| node.label() == Some(tr(app.lang(), "measure.cancel_draft")))
                .unwrap();
            assert!(!cancel.is_disabled());
            output.textures_delta.clear();
        }
    }

    #[test]
    fn calibration_requires_explicit_positive_distance_and_stays_on_one_page() {
        let mut state = MeasurementState::default();
        assert!(!state.push_point(0, (10.0, 20.0), false));
        assert!(!state.is_active_draft());
        state.start_calibration();
        assert!(state.push_point(0, (10.0, 20.0), false));
        assert!(!state.push_point(1, (110.0, 20.0), false));
        assert!(!state.push_point(0, (10.0, 20.0), false));
        assert!(state.push_point(0, (110.0, 20.0), false));
        for value in ["", "0", "-10", "NaN", "inf", "1e100"] {
            state.input_distance = value.into();
            assert!(!state.confirm_calibration());
            assert!(state.scales.is_empty());
        }
        state.input_distance = "2,5".into();
        assert!(state.confirm_calibration());
        assert!(!state.is_active_draft());
        assert_eq!(state.scales[&0].distance, 2.5);
        assert!(!state.push_point(1, (10.0, 20.0), false));
    }

    #[test]
    fn units_convert_confirmed_scale_without_reinterpreting_the_known_distance() {
        let mut state = calibrated();
        state.unit = MeasurementUnit::Centimetres;
        state.push_point(0, (10.0, 30.0), false);
        state.push_point(0, (60.0, 30.0), false);
        assert!((state.ready_result().unwrap().1.value().unwrap() - 500.0).abs() < 1e-8);
        state.cancel();
        assert!(state.scales.contains_key(&0));
        state.advance_epoch(2);
        assert!(state.scales.contains_key(&0));
        state.invalidate(3);
        assert!(state.scales.is_empty());
    }

    #[test]
    fn angle_needs_no_physical_scale_and_shift_undo_never_change_the_document() {
        let mut state = MeasurementState {
            kind: MeasurementKind::Angle,
            ..Default::default()
        };
        state.push_point(0, (10.0, 20.0), false);
        state.push_point(0, (110.0, 28.0), true);
        assert_eq!(state.points()[1], (110.0, 20.0));
        state.push_point(0, (113.0, 120.0), true);
        assert!((state.ready_result().unwrap().1.value().unwrap() - 90.0).abs() < 1e-8);
        assert!(!state.push_point(0, (110.0, 120.0), false));
        state.undo_point();
        assert!(state.ready_result().is_none());
        state.undo_point();
        state.undo_point();
        assert!(!state.is_active_draft());
    }

    #[test]
    fn closed_geometry_uses_core_validation_and_deduplicates_double_click_endpoints() {
        let mut state = calibrated();
        state.kind = MeasurementKind::Area;
        for point in [(20.0, 20.0), (120.0, 120.0), (20.0, 120.0), (120.0, 20.0)] {
            state.push_point(0, point, false);
        }
        assert!(state.ready_result().is_none());
        state.cancel();
        for point in [(20.0, 20.0), (120.0, 20.0), (120.0, 120.0)] {
            state.push_point(0, point, false);
        }
        assert!(!state.push_point(0, (120.0, 120.0), false));
        assert_eq!(state.points().len(), 3);
        assert!((state.ready_result().unwrap().1.value().unwrap() - 50.0).abs() < 1e-8);
    }

    #[test]
    fn deducted_area_drawing_selects_rings_and_preserves_real_depth_on_unit_changes() {
        let mut state = calibrated();
        state.kind = MeasurementKind::Volume;
        state.depth = MeasurementDepth {
            value: 20.0,
            unit: MeasurementUnit::Centimetres,
        };
        let outer = [(20.0, 20.0), (220.0, 20.0), (220.0, 220.0), (20.0, 220.0)];
        for point in outer {
            assert!(state.push_point(0, point, false));
        }
        assert!(state.add_deduction());
        let hole = [(70.0, 70.0), (120.0, 70.0), (120.0, 120.0), (70.0, 120.0)];
        for point in hole {
            assert!(state.push_point(0, point, false));
        }
        let measurement = state.ready_result().unwrap().1;
        assert_eq!(measurement.points, outer);
        assert_eq!(measurement.holes, vec![hole.to_vec()]);
        assert!((measurement.area_breakdown().unwrap().net - 375.0).abs() < 1e-8);
        assert!((measurement.value().unwrap() - 75.0).abs() < 1e-8);
        state.undo_point();
        assert_eq!(state.points().len(), 3);
        assert!(state.push_point(0, hole[3], false));
        state.unit = MeasurementUnit::Centimetres;
        let converted = state.ready_result().unwrap().1;
        assert_eq!(converted.depth, measurement.depth);
        assert!((converted.value().unwrap() - 75_000_000.0).abs() < 1e-4);
        state.remove_deduction();
        assert_eq!(state.points(), outer);
        assert!(state.ready_result().unwrap().1.holes.is_empty());
        state.cancel();
        assert!(state.holes.is_empty() && state.active_ring.is_none());
    }

    #[test]
    fn unfinished_touching_or_outside_deductions_cannot_be_previewed() {
        let mut state = calibrated();
        state.kind = MeasurementKind::Area;
        for point in [(20.0, 20.0), (220.0, 20.0), (220.0, 220.0), (20.0, 220.0)] {
            state.push_point(0, point, false);
        }
        assert!(state.add_deduction());
        assert!(state.ready_result().is_none());
        assert!(!state.add_deduction());
        for point in [(20.0, 20.0), (80.0, 30.0), (60.0, 80.0)] {
            state.push_point(0, point, false);
        }
        assert!(
            state.ready_result().is_none(),
            "touching outer contour is rejected"
        );
        state.remove_deduction();
        assert!(state.add_deduction());
        for point in [(250.0, 250.0), (280.0, 250.0), (280.0, 280.0)] {
            state.push_point(0, point, false);
        }
        assert!(state.ready_result().is_none(), "outside ring is rejected");
        state.remove_deduction();
        assert!(
            state.ready_result().is_some(),
            "removing invalid local ring restores previewability"
        );
    }

    #[test]
    fn double_click_jitter_finishes_the_captured_polygon_without_appending_a_vertex() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes.push(size);
        let ctx = egui::Context::default();
        app.open_measurements(&ctx);
        app.annotate.measure = calibrated();
        app.annotate.measure.advance_epoch(app.view.epoch);
        app.annotate.measure.kind = MeasurementKind::Area;
        let vertices = [(20.0, 20.0), (120.0, 20.0), (120.0, 120.0)];
        for point in vertices {
            app.measurement_click(&ctx, 0, point, false, false);
        }
        app.measurement_click(&ctx, 1, (120.5, 119.7), false, true);
        assert_eq!(app.annotate.measure.points(), vertices);
        assert!(
            requests.try_recv().is_err(),
            "another page cannot finish the draft"
        );
        app.measurement_click(&ctx, 0, (120.5, 119.7), false, true);
        let request = requests.recv().unwrap();
        let crate::worker::Request::PreviewRichAnnotation {
            page,
            annotation: sunipdf_core::annot_rich::RichAnnotation::Measurement(measurement),
            ..
        } = request
        else {
            panic!("expected one guarded measurement preview");
        };
        assert_eq!(page, 0);
        assert_eq!(measurement.points, vertices);
        assert!((measurement.value().unwrap() - 50.0).abs() < 1e-8);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn clicking_the_right_page_keeps_its_confirmed_scale_current_in_two_page_view() {
        fn frame(app: &mut SuniApp, ctx: &egui::Context, time: &mut f64, events: Vec<egui::Event>) {
            *time += 0.1;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1000.0))),
                    time: Some(*time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Panel::right("measurement-two-page-controls")
                        .exact_size(280.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| app.measurement_bar(ctx, ui));
                        });
                    app.document_view(ctx, ui);
                },
            );
            output.textures_delta.clear();
        }
        fn click_right(app: &mut SuniApp, ctx: &egui::Context, time: &mut f64, point: Point) {
            frame(app, ctx, time, Vec::new());
            let (visible_page, rect, height) = app.edit.page_view.unwrap();
            let k = app.effective_zoom * SCREEN_PER_PDF_POINT;
            let layout = crate::view::Layout::for_mode(
                &app.document.as_ref().unwrap().info.page_sizes,
                0,
                k,
                egui::vec2(1600.0, 1000.0),
                crate::view::ViewMode::TwoPage,
            );
            let offset = layout.page_rect(1).min - layout.page_rect(visible_page).min;
            let pos = rect.min + offset + egui::vec2(point.0 * k, (height - point.1) * k);
            *time += 0.6;
            for pressed in [true, false] {
                frame(
                    app,
                    ctx,
                    time,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        let (mut app, requests, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes.push(size);
        // One screen pixel per PDF point keeps synthetic clicks exact after
        // each page origin is independently snapped to the pixel grid.
        app.zoom = crate::app::Zoom::Fixed(0.75);
        app.view.mode = crate::view::ViewMode::TwoPage;
        let ctx = egui::Context::default();
        app.open_measurements(&ctx);
        let mut time = 1.0;
        for _ in 0..3 {
            frame(&mut app, &ctx, &mut time, Vec::new());
        }
        click_right(&mut app, &ctx, &mut time, (60.0, 650.0));
        assert_eq!(
            app.document.as_ref().unwrap().page,
            0,
            "uncalibrated clicks do not retarget the page"
        );
        app.annotate.measure.start_calibration();
        click_right(&mut app, &ctx, &mut time, (60.0, 650.0));
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        click_right(&mut app, &ctx, &mut time, (160.0, 650.0));
        app.annotate.measure.input_distance = "25".into();
        assert!(app.annotate.measure.confirm_calibration());
        frame(&mut app, &ctx, &mut time, Vec::new());
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(!app.annotate.measure.scales.contains_key(&0));
        assert_eq!(app.annotate.measure.scales.get(&1).unwrap().distance, 25.0);
        click_right(&mut app, &ctx, &mut time, (60.0, 550.0));
        click_right(&mut app, &ctx, &mut time, (160.0, 550.0));
        let mut previews = 0;
        for request in requests.try_iter() {
            match request {
                crate::worker::Request::PreviewRichAnnotation {
                    page,
                    annotation: sunipdf_core::annot_rich::RichAnnotation::Measurement(measurement),
                    ..
                } => {
                    previews += 1;
                    assert_eq!(page, 1);
                    assert!((measurement.value().unwrap() - 25.0).abs() < 0.01);
                }
                crate::worker::Request::Edit(_) => panic!("drawing must not mutate the PDF"),
                _ => {}
            }
        }
        assert_eq!(previews, 1);
    }

    #[test]
    fn keyboard_shortcuts_leave_open_popups_and_busy_measurements_alone() {
        for (popup, busy, key) in [
            (true, false, Key::Enter),
            (true, false, Key::Backspace),
            (false, true, Key::Enter),
            (false, true, Key::Backspace),
        ] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            app.open_measurements(&ctx);
            app.annotate.measure = calibrated();
            app.annotate.measure.advance_epoch(app.view.epoch);
            app.annotate.measure.kind = MeasurementKind::Polyline;
            app.annotate.measure.push_point(0, (10.0, 30.0), false);
            app.annotate.measure.push_point(0, (110.0, 30.0), false);
            app.saving = busy;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 400.0))),
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                if popup {
                    egui::Popup::open_id(&ctx, egui::Id::unique("measurement-test-popup"));
                }
                app.measurement_bar(&ctx, ui);
            });
            output.textures_delta.clear();
            assert_eq!(app.annotate.measure.points().len(), 2);
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests
                .try_iter()
                .all(|request| matches!(request, crate::worker::Request::MeasurementPage { .. })));
        }
    }

    #[test]
    fn busy_measurement_controls_keep_a_clickable_cancel_action() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        app.open_measurements(&ctx);
        app.annotate.measure.start_calibration();
        app.saving = true;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(280.0, 1000.0))),
            ..Default::default()
        };
        let mut target = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.measurement_bar(&ctx, ui));
            let tree = output.platform_output.accesskit_update.as_ref().unwrap();
            let button = tree
                .nodes
                .iter()
                .map(|(_, node)| node)
                .find(|node| node.label() == Some(tr(app.lang(), "measure.cancel_draft")))
                .unwrap();
            assert!(!button.is_disabled());
            let bounds = button.bounds().unwrap();
            target = Some(Pos2::new(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            ));
            output.textures_delta.clear();
        }
        let pos = target.unwrap();
        for pressed in [true, false] {
            let mut input = input.clone();
            input.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ];
            let mut output = ctx.run_ui(input, |ui| app.measurement_bar(&ctx, ui));
            output.textures_delta.clear();
        }
        assert!(!app.annotate.measure.is_active_draft());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn calibration_prerequisites_are_neutral_and_failed_input_keeps_its_error() {
        fn luminance(color: egui::Color32) -> f64 {
            let linear = |value: u8| {
                let value = f64::from(value) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        }

        fn text_colors(
            shape: &egui::Shape,
            label: &str,
            clip: Rect,
            colors: &mut Vec<egui::Color32>,
        ) {
            match shape {
                egui::Shape::Text(text)
                    if text.galley.text() == label
                        && clip.intersects(Rect::from_min_size(text.pos, text.galley.size())) =>
                {
                    let color = text.galley.job.sections[0].format.color;
                    let painted = if let Some(override_color) = text.override_text_color {
                        override_color
                    } else if color == egui::Color32::PLACEHOLDER {
                        text.fallback_color
                    } else {
                        color
                    };
                    colors.push(painted);
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_colors(shape, label, clip, colors);
                    }
                }
                _ => {}
            }
        }
        for language in ["en-US", "fr-FR"] {
            for dark in [false, true] {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.settings.language = language.into();
                app.settings.dark_theme = dark;
                let ctx = egui::Context::default();
                crate::theme::apply(&ctx, dark);
                app.open_measurements(&ctx);
                app.measurement_click(&ctx, 0, (60.0, 650.0), false, false);
                assert_eq!(app.annotate.measure.error, Some("measure.no_scale"));
                for invalid in [false, true] {
                    if invalid {
                        let state = &mut app.annotate.measure;
                        state.start_calibration();
                        assert!(state.push_point(0, (60.0, 650.0), false));
                        assert!(state.push_point(0, (160.0, 650.0), false));
                        state.input_distance = "invalid".into();
                        assert!(!state.confirm_calibration());
                    }
                    let headings = [
                        "measure.tool",
                        "measure.panel.create",
                        "measure.panel.scale",
                        "measure.panel.drawing",
                        "measure.panel.saved",
                        "measure.kind",
                        "measure.unit",
                        "measure.precision",
                    ];
                    let mut seen = std::collections::BTreeSet::new();
                    let mut saw_message = false;
                    let palette = crate::theme::palette(dark);
                    for frame in 0..48 {
                        let events = if frame == 0 || (frame >= 8 && frame % 8 == 0) {
                            vec![
                                egui::Event::PointerMoved(Pos2::new(140.0, 360.0)),
                                egui::Event::MouseWheel {
                                    unit: egui::MouseWheelUnit::Point,
                                    delta: egui::vec2(
                                        0.0,
                                        if frame == 0 { 2000.0 } else { -250.0 },
                                    ),
                                    phase: egui::TouchPhase::Move,
                                    source: egui::MouseWheelSource::Wheel,
                                    modifiers: Modifiers::NONE,
                                },
                            ]
                        } else {
                            Vec::new()
                        };
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(Rect::from_min_size(
                                    Pos2::ZERO,
                                    egui::vec2(280.0, 720.0),
                                )),
                                time: Some(
                                    (if invalid { 48.0 } else { 0.0 } + frame as f64) * 0.05,
                                ),
                                events,
                                ..Default::default()
                            },
                            |ui| app.measurement_dock(&ctx, ui),
                        );
                        output.textures_delta.clear();
                        for key in headings {
                            let mut colors = Vec::new();
                            for shape in &output.shapes {
                                text_colors(
                                    &shape.shape,
                                    tr(app.lang(), key),
                                    shape.clip_rect,
                                    &mut colors,
                                );
                            }
                            if colors.is_empty() {
                                continue;
                            }
                            seen.insert(key);
                            assert!(
                                colors.iter().all(|color| *color == palette.text),
                                "all visible labels must retain contrast: {language}, dark={dark}, {key}, {colors:?}"
                            );
                            let foreground = luminance(colors[0]);
                            let background = luminance(palette.surface);
                            assert!(
                                (foreground.max(background) + 0.05)
                                    / (foreground.min(background) + 0.05)
                                    >= 4.5,
                                "panel text must be readable: {language}, dark={dark}, {key}"
                            );
                        }
                        let mut colors = Vec::new();
                        let message = if invalid {
                            "measure.scale_invalid"
                        } else {
                            "measure.no_scale"
                        };
                        for shape in &output.shapes {
                            text_colors(
                                &shape.shape,
                                tr(app.lang(), message),
                                shape.clip_rect,
                                &mut colors,
                            );
                        }
                        if !colors.is_empty() {
                            saw_message = true;
                            assert_eq!(
                                colors,
                                vec![if invalid { palette.error } else { palette.text_muted }],
                                "show one neutral prerequisite or one actual failure: {language}, dark={dark}"
                            );
                        }
                    }
                    assert_eq!(seen, headings.into_iter().collect(), "all panel sections must be reachable by scrolling: {language}, dark={dark}");
                    assert!(
                        saw_message,
                        "calibration guidance must remain reachable: {language}, dark={dark}"
                    );
                }
            }
        }
    }

    #[test]
    fn confirmed_scales_survive_tab_switches_without_leaking_between_documents() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_measurements(&ctx);
        app.annotate.measure = calibrated();
        app.annotate.measure.advance_epoch(app.view.epoch);
        let mut other_info = app.document.as_ref().unwrap().info.clone();
        other_info.path = "second-plan.pdf".into();
        let first = app.take_tab();
        assert!(app.annotate.measure.scales.is_empty());
        app.document = Some(crate::app::OpenDocument {
            info: other_info,
            page: 0,
        });
        app.open_measurements(&ctx);
        assert!(app.annotate.measure.scales.is_empty());
        app.annotate.measure = calibrated();
        app.annotate.measure.scales.get_mut(&0).unwrap().distance = 25.0;
        app.annotate.measure.advance_epoch(app.view.epoch);
        let second = app.take_tab();
        for (tab, expected) in [(first, 10.0), (second, 25.0)] {
            app.put_tab(tab);
            app.validate_measurement_context();
            let measurement = app
                .annotate
                .measure
                .result(0, vec![(10.0, 30.0), (110.0, 30.0)])
                .unwrap();
            assert!((measurement.value().unwrap() - expected).abs() < 1e-8);
            assert!(!app.annotate.measure.is_active_draft());
        }
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn resumed_document_with_different_page_geometry_drops_saved_session_scales() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_measurements(&ctx);
        app.annotate.measure = calibrated();
        app.annotate.measure.advance_epoch(app.view.epoch);
        let mut info = app.document.as_ref().unwrap().info.clone();
        let tab = app.take_tab();
        app.tabs.tabs.push(crate::tabs::Tab {
            path: info.path.clone(),
            dirty: false,
            recovery_id: 0,
            ui: Some(tab),
            doc: None,
        });
        app.tabs.resuming = Some(app.tabs.tabs.len() - 1);
        info.page_sizes[0].width_pt += 40.0;
        app.on_resumed(
            crate::worker::DocState {
                info,
                can_undo: false,
                can_redo: false,
                modified: false,
                pages_unchanged: false,
                measurement_scales_unchanged: false,
                has_signatures: false,
            },
            &ctx,
        );
        app.validate_measurement_context();
        assert!(app.annotate.measure.scales.is_empty());
        assert!(!app.annotate.measure.is_active_draft());
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, crate::worker::Request::Edit(_))));
    }
    #[test]
    fn feet_inches_calibration_is_parsed_in_the_selected_unit() {
        let mut state = MeasurementState {
            unit: MeasurementUnit::Feet,
            ..Default::default()
        };
        state.start_calibration();
        state.push_point(0, (20.0, 20.0), false);
        state.push_point(0, (140.0, 20.0), false);
        state.input_distance = "5' 6 1/2\"".into();
        assert!(state.confirm_calibration());
        state.push_point(0, (20.0, 40.0), false);
        state.push_point(0, (140.0, 40.0), false);
        assert!(
            (state.ready_result().unwrap().1.value().unwrap() - (5.0 + 6.5 / 12.0)).abs() < 1e-10
        );
    }

    fn area_with_viewports(
        viewports: Vec<sunipdf_core::measurement::MeasurementViewport>,
    ) -> MeasurementState {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.open_measurements(&egui::Context::default());
        app.request_measurement_page(0);
        let crate::worker::Request::MeasurementPage {
            page,
            epoch,
            generation,
        } = requests.recv().unwrap()
        else {
            panic!("measurement metadata")
        };
        let calibration = PageCalibration {
            points: [(0.0, 0.0), (100.0, 0.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        };
        let mut data = crate::measurement_saved::MeasurementPageData::new(
            Ok(Some(calibration)),
            Ok(vec![]),
            true,
        );
        data.viewports = Ok(std::sync::Arc::from(viewports));
        app.on_measurement_page(page, epoch, generation, Ok(data));
        let mut state = std::mem::take(&mut app.annotate.measure);
        state.kind = MeasurementKind::Area;
        state.gesture = Gesture::Draw {
            page: 0,
            points: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
        };
        state
    }

    #[test]
    fn scale_selection_accepts_more_than_128_total_vertices_across_separate_rings() {
        let mut state = area_with_viewports(vec![]);
        for (x, y) in [(30.0, 30.0), (70.0, 30.0), (50.0, 70.0)] {
            state.holes.push(
                (0..64)
                    .map(|index| {
                        let angle = index as f32 * std::f32::consts::TAU / 64.0;
                        (x + 10.0 * angle.cos(), y + 10.0 * angle.sin())
                    })
                    .collect(),
            );
        }
        let (_, measurement) = state
            .ready_result()
            .expect("196 vertices in four valid rings");
        assert_eq!(
            measurement.points.len() + measurement.holes.iter().map(Vec::len).sum::<usize>(),
            196
        );
        assert_eq!(measurement.holes.len(), 3);
        assert!((90.0..91.0).contains(&measurement.value().unwrap()));
        state.active_ring = Some(1);
        assert_eq!(
            state.ready_result().unwrap().1,
            measurement,
            "selected deduction does not change the outer scale contour"
        );
    }

    #[test]
    fn enclosed_scale_region_cannot_be_hidden_by_connecting_outer_and_deduction_vertices() {
        use sunipdf_core::{measurement::MeasurementViewport, PointRect};
        let mut state = area_with_viewports(vec![MeasurementViewport {
            id: "internal-region".into(),
            name: "Internal detail".into(),
            bounds: PointRect {
                left: 2.5,
                bottom: 45.0,
                right: 5.0,
                top: 50.0,
            },
            calibration: PageCalibration {
                points: [(3.0, 46.0), (4.0, 46.0)],
                known_distance: 1.0,
                unit: MeasurementUnit::Metres,
            },
        }]);
        state
            .holes
            .push(vec![(20.0, 20.0), (40.0, 20.0), (40.0, 40.0), (20.0, 40.0)]);
        assert!(
            state.ready_result().is_none(),
            "automatic choice must reject a region enclosed by the outer boundary"
        );
        state.active_ring = Some(0);
        assert!(
            state.ready_result().is_none(),
            "selecting the hole cannot hide that region"
        );
        state
            .scales_ui
            .selected
            .insert(0, crate::measurement_scales_ui::PAGE_SCALE.into());
        assert!(
            (state.ready_result().unwrap().1.value().unwrap() - 96.0).abs() < 1e-8,
            "explicit page scale resolves the ambiguity"
        );
    }

    #[test]
    fn viewport_overlap_refuses_drawing_until_user_chooses_a_region_or_page_scale() {
        use crate::measurement_saved::MeasurementPageData;
        use crate::worker::Request;
        use sunipdf_core::{measurement::MeasurementViewport, PointRect};
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.open_measurements(&egui::Context::default());
        app.validate_measurement_context();
        app.annotate.measure.use_calibration(
            0,
            &PageCalibration {
                points: [(20.0, 20.0), (100.0, 20.0)],
                known_distance: 8.0,
                unit: MeasurementUnit::Metres,
            },
        );
        app.request_measurement_page(0);
        let Request::MeasurementPage {
            page,
            epoch,
            generation,
        } = requests.recv().unwrap()
        else {
            panic!("metadata")
        };
        let mut data = MeasurementPageData::new(Ok(None), Ok(vec![]), true);
        data.viewports = Ok(std::sync::Arc::from(vec![
            MeasurementViewport {
                id: "a".into(),
                name: "A".into(),
                bounds: PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 180.0,
                    top: 180.0,
                },
                calibration: PageCalibration {
                    points: [(20.0, 20.0), (100.0, 20.0)],
                    known_distance: 16.0,
                    unit: MeasurementUnit::Metres,
                },
            },
            MeasurementViewport {
                id: "b".into(),
                name: "B".into(),
                bounds: PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 180.0,
                    top: 180.0,
                },
                calibration: PageCalibration {
                    points: [(20.0, 20.0), (100.0, 20.0)],
                    known_distance: 24.0,
                    unit: MeasurementUnit::Metres,
                },
            },
        ]));
        app.on_measurement_page(page, epoch, generation, Ok(data));
        let state = &mut app.annotate.measure;
        assert!(!state.push_point(0, (20.0, 60.0), false));
        assert_eq!(state.error, Some("measure.viewport.conflict"));
        state.scales_ui.selected.insert(0, "b".into());
        assert!(state.push_point(0, (20.0, 60.0), false));
        assert!(
            !state.push_point(0, (200.0, 60.0), false),
            "selected region boundary is enforced"
        );
        assert!(state.push_point(0, (100.0, 60.0), false));
        assert!((state.ready_result().unwrap().1.value().unwrap() - 24.0).abs() < 1e-9);
        assert_eq!(state.effective_calibration(0).unwrap().known_distance, 24.0);
        state.cancel();
        state
            .scales_ui
            .selected
            .insert(0, crate::measurement_scales_ui::PAGE_SCALE.into());
        assert!(state.push_point(0, (20.0, 60.0), false));
        assert!(state.push_point(0, (200.0, 60.0), false));
        assert!((state.ready_result().unwrap().1.value().unwrap() - 18.0).abs() < 1e-9);
    }
}
