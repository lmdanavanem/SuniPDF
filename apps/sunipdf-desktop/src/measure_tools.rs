//! On-page calibrated measurements. Draft geometry never mutates the document.

use crate::{
    annotate::Tool,
    app::{EngineState, SuniApp, SCREEN_PER_PDF_POINT},
};
use egui::{Color32, Key, Modifiers, Painter, Pos2, Rect, Stroke};
use std::collections::BTreeMap;
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementUnit};
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
    pub(crate) kind: MeasurementKind,
    pub(crate) unit: MeasurementUnit,
    pub(crate) precision: usize,
    pub(crate) input_distance: String,
    epoch: Option<u64>,
    scales: BTreeMap<usize, Scale>,
    gesture: Gesture,
    hover: Option<(usize, Point)>,
    error: Option<&'static str>,
}

impl Default for MeasurementState {
    fn default() -> Self {
        Self {
            kind: MeasurementKind::Length,
            unit: MeasurementUnit::Metres,
            precision: 2,
            input_distance: String::new(),
            epoch: None,
            scales: BTreeMap::new(),
            gesture: Gesture::Idle,
            hover: None,
            error: None,
        }
    }
}

impl MeasurementState {
    pub fn is_active_draft(&self) -> bool {
        !matches!(self.gesture, Gesture::Idle)
    }

    pub fn cancel(&mut self) -> bool {
        let active = self.is_active_draft();
        self.gesture = Gesture::Idle;
        self.input_distance.clear();
        self.hover = None;
        self.error = None;
        active
    }

    /// Keep page scales only when the caller has proved page geometry is unchanged.
    pub(crate) fn advance_epoch(&mut self, epoch: u64) {
        self.cancel();
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
        let distance = self
            .input_distance
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok();
        let Some(distance) = distance else {
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
    }

    fn points(&self) -> &[Point] {
        match &self.gesture {
            Gesture::Calibrate { points, .. } | Gesture::Draw { points, .. } => points,
            Gesture::Idle => &[],
        }
    }

    fn result(&self, page: usize, points: Vec<Point>) -> Option<Measurement> {
        let scale = if self.kind == MeasurementKind::Angle {
            // Angle has no physical scale. Reuse real vertices to satisfy core geometry validation.
            Scale {
                points: [*points.first()?, *points.get(1)?],
                distance: 1.0,
                unit: self.unit,
            }
        } else {
            *self.scales.get(&page)?
        };
        Some(measurement(
            self.kind,
            points,
            scale,
            self.unit,
            self.precision,
        ))
    }

    fn ready_result(&self) -> Option<(usize, Measurement)> {
        let Gesture::Draw { page, points } = &self.gesture else {
            return None;
        };
        let result = self.result(*page, points.clone())?;
        result.value().ok()?;
        Some((*page, result))
    }

    fn push_point(&mut self, page: usize, point: Point, shift: bool) -> bool {
        if self.page().is_some_and(|p| p != page) {
            self.error = Some("measure.same_page");
            return false;
        }
        if matches!(self.gesture, Gesture::Idle) {
            if self.kind != MeasurementKind::Angle && !self.scales.contains_key(&page) {
                self.error = Some("measure.no_scale");
                return false;
            }
            self.gesture = Gesture::Draw {
                page,
                points: Vec::new(),
            };
        }
        let point = constrain(self.points().last().copied(), point, shift);
        let calibrating = matches!(self.gesture, Gesture::Calibrate { .. });
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
}

fn distance(a: Point, b: Point) -> f64 {
    (f64::from(a.0) - f64::from(b.0)).hypot(f64::from(a.1) - f64::from(b.1))
}

fn constrain(anchor: Option<Point>, point: Point, shift: bool) -> Point {
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
    }

    fn measurement_ready(&self) -> bool {
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
        if !self.measurement_ready() {
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
        if self.annotate.tool != Tool::Measure {
            return;
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
        self.annotate.measure.cancel();
        if !self.open_measurement_preview(page, measurement) {
            self.annotate.measure.gesture = gesture;
        }
    }

    pub(crate) fn measurement_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.annotate.tool != Tool::Measure {
            return;
        }
        self.validate_measurement_context();
        let lang = self.lang();
        let ready = self.measurement_ready();
        let popup_was_open = egui::Popup::is_any_open(ctx);
        let page = self
            .annotate
            .measure
            .page()
            .or_else(|| self.document.as_ref().map(|d| d.page));
        let mut finish = false;
        let state = &mut self.annotate.measure;
        ui.add_enabled_ui(ready, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr(lang, "measure.tool"));
                ui.add_enabled_ui(!state.is_active_draft(), |ui| {
                    egui::ComboBox::from_id_salt("measure-on-page-kind")
                        .selected_text(tr(lang, kind_key(state.kind)))
                        .show_ui(ui, |ui| {
                            for kind in MeasurementKind::ALL {
                                ui.selectable_value(
                                    &mut state.kind,
                                    kind,
                                    tr(lang, kind_key(kind)),
                                );
                            }
                        });
                });
                ui.label(tr(lang, "measure.unit"));
                egui::ComboBox::from_id_salt("measure-on-page-unit")
                    .selected_text(state.unit.id())
                    .show_ui(ui, |ui| {
                        for unit in MeasurementUnit::ALL {
                            ui.selectable_value(&mut state.unit, unit, unit.id());
                        }
                    });
                ui.label(tr(lang, "measure.precision"));
                ui.add(egui::DragValue::new(&mut state.precision).range(0..=4));
                if ui
                    .add_enabled(
                        !state.is_active_draft(),
                        egui::Button::new(tr(lang, "measure.calibrate")),
                    )
                    .clicked()
                {
                    state.start_calibration();
                }
                if ui
                    .add_enabled(
                        !state.points().is_empty(),
                        egui::Button::new(tr(lang, "measure.undo_point")),
                    )
                    .clicked()
                {
                    state.undo_point();
                }
                if ui
                    .add_enabled(
                        state.ready_result().is_some(),
                        egui::Button::new(tr(lang, "measure.finish")),
                    )
                    .clicked()
                {
                    finish = true;
                }
            });
            let confirm =
                matches!(&state.gesture, Gesture::Calibrate { points, .. } if points.len() == 2);
            if confirm {
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr(lang, "measure.known_distance"));
                    let distance = ui.add(
                        egui::TextEdit::singleline(&mut state.input_distance)
                            .desired_width(120.0)
                            .char_limit(32),
                    );
                    ui.label(state.unit.id());
                    if ui.button(tr(lang, "measure.confirm_scale")).clicked()
                        || (distance.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        state.confirm_calibration();
                    }
                });
            }
            let hint = match &state.gesture {
                Gesture::Calibrate { points, .. } if points.len() == 2 => "measure.distance_hint",
                Gesture::Calibrate { .. } => "measure.calibration_hint",
                Gesture::Draw { .. } if state.kind == MeasurementKind::Angle => {
                    "measure.angle_hint"
                }
                Gesture::Draw { .. } => "measure.draw_hint",
                Gesture::Idle
                    if state.kind != MeasurementKind::Angle
                        && !page.is_some_and(|p| state.scales.contains_key(&p)) =>
                {
                    "measure.no_scale"
                }
                Gesture::Idle if state.kind == MeasurementKind::Angle => "measure.angle_hint",
                Gesture::Idle => "measure.ready_hint",
            };
            ui.label(tr(lang, hint));
            if let Some((page, scale)) = page.and_then(|p| state.scales.get(&p).map(|s| (p, s))) {
                ui.small(tr_args(
                    lang,
                    "measure.scale_summary",
                    &[
                        ("page", &(page + 1).to_string()),
                        (
                            "span",
                            &format!("{:.2}", distance(scale.points[0], scale.points[1])),
                        ),
                        ("distance", &scale.distance.to_string()),
                        ("unit", scale.unit.id()),
                    ],
                ))
                .on_hover_text(tr(lang, "measure.page_scope"));
            }
            if let Some(error) = state.error {
                ui.colored_label(
                    crate::theme::palette(self.settings.dark_theme).error,
                    tr(lang, error),
                );
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
        });
        // Cancellation only discards local points and must remain available while work is busy.
        if ui
            .add_enabled(
                state.is_active_draft(),
                egui::Button::new(tr(lang, "measure.cancel_draft")),
            )
            .clicked()
        {
            state.cancel();
            finish = false;
        }
        if finish {
            self.finish_measurement(ctx);
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
        for pair in points.windows(2) {
            painter.line_segment([screen(pair[0]), screen(pair[1])], stroke);
        }
        if !calibrating
            && matches!(
                state.kind,
                MeasurementKind::Area | MeasurementKind::Perimeter
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
        assert!(state.scales.contains_key(&0));
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
                    egui::Panel::top("measurement-two-page-controls")
                        .show(ui, |ui| app.measurement_bar(ctx, ui));
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
        app.zoom = crate::app::Zoom::Fixed(0.6);
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
            assert!(requests.try_recv().is_err());
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
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 400.0))),
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
}
