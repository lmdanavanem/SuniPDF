//! Local vertex editing of an existing measurement, committed through the guarded preview.

use crate::{
    app::{SuniApp, SCREEN_PER_PDF_POINT},
    view::DragEvent,
};
use egui::{Color32, Key, Modifiers, Painter, Pos2, Rect, Stroke};
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementRecord};
use sunipdf_i18n::{tr, tr_args};

const HANDLE_HIT_RADIUS: f32 = 10.0;
const MAX_VERTICES: usize = 128;

pub(crate) fn geometry_count_valid(kind: MeasurementKind, count: usize) -> bool {
    match kind {
        MeasurementKind::Length => count == 2,
        MeasurementKind::Angle => count == 3,
        MeasurementKind::Polyline => (2..=MAX_VERTICES).contains(&count),
        MeasurementKind::Perimeter | MeasurementKind::Area => (3..=MAX_VERTICES).contains(&count),
    }
}

#[derive(Clone)]
pub(crate) struct GeometryDraft {
    pub page: usize,
    pub epoch: u64,
    pub original: MeasurementRecord,
    pub replacement: Measurement,
    dragging: Option<usize>,
    selected: Option<usize>,
    adding: bool,
    error: Option<&'static str>,
}

impl GeometryDraft {
    fn hit_vertex(&self, point: (f32, f32), screen_scale: f32) -> Option<usize> {
        if !screen_scale.is_finite() || screen_scale <= 0.0 {
            return None;
        }
        let mut hits: Vec<_> = self
            .replacement
            .points
            .iter()
            .enumerate()
            .map(|(index, &(x, y))| (index, (point.0 - x).hypot(point.1 - y) * screen_scale))
            .filter(|(_, distance)| *distance <= HANDLE_HIT_RADIUS)
            .collect();
        hits.sort_by(|a, b| a.1.total_cmp(&b.1));
        let &(index, distance) = hits.first()?;
        if hits.get(1).is_some_and(|(_, next)| next - distance <= 0.5) {
            return self.selected.filter(|selected| {
                hits.iter()
                    .any(|(index, d)| index == selected && d - distance <= 0.5)
            });
        }
        Some(index)
    }

    fn vertex_changes_allowed(&self) -> bool {
        matches!(
            self.replacement.kind,
            MeasurementKind::Polyline | MeasurementKind::Perimeter | MeasurementKind::Area
        )
    }

    fn can_add(&self) -> bool {
        self.vertex_changes_allowed() && self.replacement.points.len() < MAX_VERTICES
    }

    fn can_remove(&self) -> bool {
        self.vertex_changes_allowed()
            && self
                .selected
                .is_some_and(|index| index < self.replacement.points.len())
            && geometry_count_valid(
                self.replacement.kind,
                self.replacement.points.len().saturating_sub(1),
            )
    }

    fn remove_selected(&mut self) -> bool {
        if !self.can_remove() || self.dragging.is_some() {
            return false;
        }
        let index = self.selected.unwrap();
        let mut candidate = self.replacement.clone();
        candidate.points.remove(index);
        if candidate.value().is_err() {
            self.error = Some("measure.geometry.remove_refused");
            return false;
        }
        self.replacement = candidate;
        self.selected = Some(index.min(self.replacement.points.len() - 1));
        self.adding = false;
        self.error = None;
        true
    }

    fn insert_on_segment(&mut self, point: (f32, f32), scale: f32) -> bool {
        if !self.can_add()
            || self.dragging.is_some()
            || !scale.is_finite()
            || scale <= 0.0
            || !point.0.is_finite()
            || !point.1.is_finite()
        {
            return false;
        }
        let points = &self.replacement.points;
        if points
            .iter()
            .any(|&(x, y)| (point.0 - x).hypot(point.1 - y) * scale <= HANDLE_HIT_RADIUS)
        {
            return false;
        }
        let closed = matches!(
            self.replacement.kind,
            MeasurementKind::Area | MeasurementKind::Perimeter
        );
        let segments = points.len() - usize::from(!closed);
        let mut hits = Vec::new();
        for index in 0..segments {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            let dx = f64::from(b.0) - f64::from(a.0);
            let dy = f64::from(b.1) - f64::from(a.1);
            let length_squared = dx * dx + dy * dy;
            if length_squared < 0.0001 {
                continue;
            }
            let t = ((f64::from(point.0) - f64::from(a.0)) * dx
                + (f64::from(point.1) - f64::from(a.1)) * dy)
                / length_squared;
            if !(0.0..1.0).contains(&t) || t == 0.0 {
                continue;
            }
            let projected = (
                (f64::from(a.0) + t * dx) as f32,
                (f64::from(a.1) + t * dy) as f32,
            );
            let distance = (point.0 - projected.0).hypot(point.1 - projected.1) * scale;
            if distance > HANDLE_HIT_RADIUS {
                continue;
            }
            let interior = (projected.0 - a.0)
                .hypot(projected.1 - a.1)
                .min((projected.0 - b.0).hypot(projected.1 - b.1));
            if interior < 0.01 || interior * scale <= HANDLE_HIT_RADIUS {
                continue;
            }
            hits.push((index, projected, distance));
        }
        hits.sort_by(|a, b| a.2.total_cmp(&b.2));
        let Some(&(index, projected, distance)) = hits.first() else {
            return false;
        };
        if hits
            .get(1)
            .is_some_and(|(_, _, next)| next - distance <= 0.5)
        {
            return false;
        }
        let mut candidate = self.replacement.clone();
        candidate.points.insert(index + 1, projected);
        if candidate.value().is_err() {
            return false;
        }
        self.replacement = candidate;
        self.selected = Some(index + 1);
        self.adding = false;
        self.error = None;
        true
    }

    fn changed(&self) -> bool {
        self.replacement.points != self.original.measurement.points
    }
}

impl SuniApp {
    pub(crate) fn restore_measurement_geometry(
        &mut self,
        page: usize,
        original: MeasurementRecord,
        replacement: Measurement,
    ) -> bool {
        if !self.start_measurement_geometry(page, original) {
            return false;
        }
        self.annotate.measure.geometry.as_mut().unwrap().replacement = replacement;
        true
    }

    pub(crate) fn start_measurement_geometry(
        &mut self,
        page: usize,
        record: MeasurementRecord,
    ) -> bool {
        self.validate_measurement_context();
        if !self.measurement_ready()
            || self.annotate.measure.is_active_draft()
            || !self.annotate.measure.saved.can_edit(page, &record)
            || record.measurement.value().is_err()
            || !self
                .document
                .as_ref()
                .is_some_and(|doc| page < doc.info.page_count())
        {
            return false;
        }
        // Navigate before activating the draft; the draft then blocks page and tab changes.
        if let Some(&(x, y)) = record.measurement.points.first() {
            let height = self.document.as_ref().unwrap().info.page_sizes[page].height_pt;
            self.go_to_page_point(page, x, (height - y).max(0.0));
        }
        self.annotate.measure.geometry = Some(GeometryDraft {
            page,
            epoch: self.view.epoch,
            replacement: record.measurement.clone(),
            original: record,
            dragging: None,
            selected: None,
            adding: false,
            error: None,
        });
        true
    }

    pub(crate) fn measurement_geometry_event(&mut self, page: usize, event: DragEvent) {
        self.validate_measurement_context();
        if !self.measurement_ready() {
            // A modal or a mutation appearing during a drag must not keep a captured handle.
            if let Some(draft) = self.annotate.measure.geometry.as_mut() {
                draft.dragging = None;
            }
            return;
        }
        match event {
            DragEvent::Start(point) => {
                let scale = self.effective_zoom * SCREEN_PER_PDF_POINT;
                if let Some(draft) = self
                    .annotate
                    .measure
                    .geometry
                    .as_mut()
                    .filter(|d| d.page == page)
                {
                    if !draft.adding {
                        draft.dragging = draft.hit_vertex(point, scale);
                        if draft.dragging.is_some() {
                            draft.selected = draft.dragging;
                            draft.error = None;
                        }
                    }
                }
            }
            DragEvent::Move(point) => self.measurement_geometry_move(page, point),
            DragEvent::Stop => {
                if let Some(draft) = self
                    .annotate
                    .measure
                    .geometry
                    .as_mut()
                    .filter(|d| d.page == page)
                {
                    draft.dragging = None;
                }
            }
            DragEvent::Click(point) => {
                let scale = self.effective_zoom * SCREEN_PER_PDF_POINT;
                if let Some(draft) = self
                    .annotate
                    .measure
                    .geometry
                    .as_mut()
                    .filter(|d| d.page == page)
                {
                    if draft.adding {
                        if !draft.insert_on_segment(point, scale) {
                            draft.error = Some("measure.geometry.insert_refused");
                        }
                    } else {
                        draft.selected = draft.hit_vertex(point, scale);
                        draft.error = None;
                    }
                }
            }
            DragEvent::DoubleClick(_) => {}
        }
    }

    /// Also called on pointer release, because the shared Stop event carries no coordinates.
    pub(crate) fn measurement_geometry_move(&mut self, page: usize, point: (f32, f32)) {
        self.validate_measurement_context();
        if !self.measurement_ready() || !point.0.is_finite() || !point.1.is_finite() {
            return;
        }
        let point = self.snap_measurement_point(page, point, false);
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
        else {
            return;
        };
        if let Some(draft) = self
            .annotate
            .measure
            .geometry
            .as_mut()
            .filter(|d| d.page == page)
        {
            if let Some(index) = draft.dragging {
                draft.replacement.points[index] = (
                    point.0.clamp(0.0, size.width_pt),
                    point.1.clamp(0.0, size.height_pt),
                );
            }
        }
    }

    pub(crate) fn finish_measurement_geometry(&mut self) -> bool {
        self.validate_measurement_context();
        if !self.measurement_ready() {
            return false;
        }
        let Some(draft) = self.annotate.measure.geometry.as_ref() else {
            return false;
        };
        if !draft.changed()
            || draft.dragging.is_some()
            || draft.adding
            || draft.replacement.value().is_err()
        {
            return false;
        }
        let draft = self.annotate.measure.geometry.take().unwrap();
        if self.open_measurement_geometry_preview(
            draft.page,
            draft.original.clone(),
            draft.replacement.clone(),
        ) {
            true
        } else {
            self.annotate.measure.geometry = Some(draft);
            false
        }
    }

    pub(crate) fn measurement_geometry_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let text_color = crate::theme::palette(self.settings.dark_theme).text;
        let ready = self.measurement_ready();
        let draft = self.annotate.measure.geometry.as_mut().unwrap();
        let valid = draft.replacement.value().is_ok();
        let can_finish =
            ready && draft.changed() && valid && draft.dragging.is_none() && !draft.adding;
        ui.label(
            egui::RichText::new(tr(lang, "measure.geometry.open"))
                .strong()
                .color(text_color),
        );
        ui.label(tr(lang, "measure.geometry.hint"));
        ui.small(tr_args(
            lang,
            "measure.geometry.details",
            &[
                ("points", &draft.replacement.points.len().to_string()),
                ("unit", draft.replacement.unit.id()),
                ("precision", &draft.replacement.precision.to_string()),
            ],
        ));
        ui.add_enabled_ui(ready && draft.dragging.is_none(), |ui| {
            ui.vertical(|ui| {
                ui.label(tr(lang, "measure.geometry.vertex"));
                egui::ComboBox::from_id_salt("measurement-geometry-vertex")
                    .width(ui.available_width())
                    .truncate()
                    .selected_text(draft.selected.map_or_else(
                        || tr(lang, "measure.geometry.select_vertex").to_owned(),
                        |index| (index + 1).to_string(),
                    ))
                    .show_ui(ui, |ui| {
                        for index in 0..draft.replacement.points.len() {
                            if ui
                                .selectable_value(
                                    &mut draft.selected,
                                    Some(index),
                                    (index + 1).to_string(),
                                )
                                .changed()
                            {
                                draft.adding = false;
                                draft.error = None;
                            }
                        }
                    });
                if crate::measure_tools::measurement_panel_button(
                    ui,
                    draft.can_add() || draft.adding,
                    tr(
                        lang,
                        if draft.adding {
                            "measure.geometry.stop_adding"
                        } else {
                            "measure.geometry.add_vertex"
                        },
                    ),
                )
                .clicked()
                {
                    draft.adding = !draft.adding;
                    draft.error = None;
                }
                if crate::measure_tools::measurement_panel_button(
                    ui,
                    draft.can_remove(),
                    tr(lang, "measure.geometry.remove_vertex"),
                )
                .clicked()
                {
                    draft.remove_selected();
                }
            });
        });
        if draft.adding {
            ui.label(tr(lang, "measure.geometry.add_hint"));
        }
        if !draft.vertex_changes_allowed() {
            ui.small(tr(lang, "measure.geometry.fixed_count"));
        }
        if let Some(error) = draft.error {
            ui.colored_label(
                crate::theme::palette(self.settings.dark_theme).error,
                tr(lang, error),
            );
        }
        if !valid {
            ui.colored_label(
                crate::theme::palette(self.settings.dark_theme).error,
                tr(lang, "measure.invalid"),
            );
        }
        let mut finish = false;
        let mut cancel = false;
        ui.vertical(|ui| {
            finish = crate::measure_tools::measurement_panel_button(
                ui,
                can_finish,
                tr(lang, "measure.geometry.preview"),
            )
            .clicked();
            cancel = crate::measure_tools::measurement_panel_button(
                ui,
                true,
                tr(lang, "measure.cancel_draft"),
            )
            .clicked();
        });
        if can_finish && !ctx.egui_wants_keyboard_input() && !egui::Popup::is_any_open(ctx) {
            finish |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        }
        if cancel {
            self.cancel_measurement();
        } else if finish {
            self.finish_measurement_geometry();
        }
    }

    pub(crate) fn paint_measurement_geometry(
        &self,
        painter: &Painter,
        rect: Rect,
        page: usize,
        height: f32,
    ) {
        let Some(draft) = self
            .annotate
            .measure
            .geometry
            .as_ref()
            .filter(|d| d.page == page && d.epoch == self.view.epoch)
        else {
            return;
        };
        let scale = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let screen = |point: (f32, f32)| {
            Pos2::new(
                rect.left() + point.0 * scale,
                rect.top() + (height - point.1) * scale,
            )
        };
        let color = Color32::from_rgb(20, 115, 225);
        let points = &draft.replacement.points;
        for pair in points.windows(2) {
            painter.line_segment([screen(pair[0]), screen(pair[1])], Stroke::new(2.0, color));
        }
        if matches!(
            draft.replacement.kind,
            MeasurementKind::Area | MeasurementKind::Perimeter
        ) && points.len() >= 3
        {
            painter.line_segment(
                [screen(*points.last().unwrap()), screen(points[0])],
                Stroke::new(2.0, color),
            );
        }
        for point in points {
            painter.circle_filled(screen(*point), 6.0, Color32::WHITE);
            painter.circle_stroke(screen(*point), 6.0, Stroke::new(2.0, color));
        }
        if let Some((index, point)) = draft
            .selected
            .and_then(|index| points.get(index).map(|point| (index, point)))
        {
            // Paint last so another vertex at the same position cannot hide the selected handle.
            painter.circle_filled(screen(*point), 6.0, color);
            painter.circle_stroke(screen(*point), 6.0, Stroke::new(2.0, color));
            painter.text(
                screen(*point) + egui::vec2(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                (index + 1).to_string(),
                egui::FontId::proportional(13.0),
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::measurement::MeasurementUnit;

    #[test]
    fn geometry_hit_radius_stays_constant_on_screen_and_nearest_vertex_wins() {
        let measurement = Measurement {
            kind: MeasurementKind::Length,
            points: vec![(50.0, 100.0), (150.0, 100.0)],
            calibration: [(10.0, 20.0), (110.0, 20.0)],
            known_distance: 12.0,
            unit: MeasurementUnit::Feet,
            precision: 3,
        };
        let draft = GeometryDraft {
            page: 0,
            epoch: 7,
            original: MeasurementRecord {
                validity: sunipdf_core::measurement::MeasurementValidity::Current,
                index: 0,
                annotation_id: Some("measure-0".into()),
                measurement: measurement.clone(),
            },
            replacement: measurement,
            dragging: None,
            selected: None,
            adding: false,
            error: None,
        };
        for scale in [0.1, 1.0, 3.0, 10.0] {
            assert_eq!(
                draft.hit_vertex((50.0, 100.0 + 9.0 / scale), scale),
                Some(0)
            );
            assert_eq!(draft.hit_vertex((50.0, 100.0 + 11.0 / scale), scale), None);
        }
        assert_eq!(draft.hit_vertex((149.0, 100.0), 0.1), Some(1));
        assert_eq!(draft.hit_vertex((f32::NAN, 100.0), 1.0), None);
        assert_eq!(draft.hit_vertex((50.0, 100.0), 0.0), None);
        assert_eq!(
            draft.hit_vertex((100.0, 100.0), 0.1),
            None,
            "equidistant handles are ambiguous"
        );
        let mut coincident = draft.clone();
        coincident.replacement.kind = MeasurementKind::Polyline;
        coincident.replacement.points =
            vec![(50.0, 100.0), (150.0, 100.0), (50.0, 100.0), (50.0, 200.0)];
        assert!(coincident.replacement.value().is_ok());
        assert_eq!(
            coincident.hit_vertex((50.0, 100.0), 1.0),
            None,
            "coincident nonadjacent vertices must not pick the wrong vertex"
        );
        assert_eq!(coincident.hit_vertex((150.0, 100.0), 1.0), Some(1));
        coincident.selected = Some(2);
        assert_eq!(
            coincident.hit_vertex((50.0, 100.0), 1.0),
            Some(2),
            "explicit selector resolves a coincident vertex"
        );
        coincident.selected = Some(0);
        assert_eq!(coincident.hit_vertex((50.0, 100.0), 1.0), Some(0));
    }

    fn editable(kind: MeasurementKind, points: Vec<(f32, f32)>) -> GeometryDraft {
        let measurement = Measurement {
            kind,
            points,
            calibration: [(10.0, 10.0), (110.0, 10.0)],
            known_distance: 12.0,
            unit: MeasurementUnit::Feet,
            precision: 3,
        };
        GeometryDraft {
            page: 0,
            epoch: 7,
            original: MeasurementRecord {
                validity: sunipdf_core::measurement::MeasurementValidity::Current,
                index: 3,
                annotation_id: Some("editable-vertices".into()),
                measurement: measurement.clone(),
            },
            replacement: measurement,
            dragging: None,
            selected: None,
            adding: false,
            error: None,
        }
    }

    #[test]
    fn insert_projects_on_open_and_closing_segments_preserving_scale_and_remove_restores_geometry()
    {
        for kind in [
            MeasurementKind::Polyline,
            MeasurementKind::Perimeter,
            MeasurementKind::Area,
        ] {
            let mut draft = editable(
                kind,
                vec![(20.0, 20.0), (220.0, 20.0), (220.0, 220.0), (20.0, 220.0)],
            );
            let original = draft.replacement.clone();
            draft.adding = true;
            assert!(draft.insert_on_segment((120.0, 23.0), 1.0));
            assert_eq!(draft.selected, Some(1));
            assert_eq!(draft.replacement.points[1], (120.0, 20.0));
            assert!(!draft.adding, "successful insertion is one shot");
            assert!(draft.remove_selected());
            assert_eq!(draft.replacement, original);
            draft.adding = true;
            if kind == MeasurementKind::Polyline {
                assert!(
                    !draft.insert_on_segment((22.0, 120.0), 1.0),
                    "open polyline has no closing edge"
                );
            } else {
                assert!(draft.insert_on_segment((22.0, 120.0), 1.0));
                assert_eq!(draft.selected, Some(4));
                assert_eq!(draft.replacement.points[4], (20.0, 120.0));
                assert_eq!(
                    draft.replacement.value().unwrap(),
                    original.value().unwrap()
                );
                assert!(draft.remove_selected());
                assert_eq!(draft.replacement, original);
            }
        }
    }

    #[test]
    fn vertex_insertion_refuses_endpoints_crossings_and_screen_ambiguity_at_any_zoom() {
        for scale in [0.25, 1.0, 4.0] {
            let mut draft = editable(
                MeasurementKind::Polyline,
                vec![(0.0, 0.0), (200.0, 200.0), (0.0, 200.0), (200.0, 0.0)],
            );
            let original = draft.replacement.clone();
            assert!(!draft.insert_on_segment((100.0, 100.0), scale));
            assert!(!draft.insert_on_segment((1.0, 1.0), scale));
            assert!(!draft.insert_on_segment((500.0, 500.0), scale));
            assert!(!draft.insert_on_segment((f32::NAN, 100.0), scale));
            assert_eq!(draft.replacement, original);
            assert!(
                draft.insert_on_segment((100.0, 200.0), scale),
                "clear closest segment remains usable"
            );
        }
    }

    #[test]
    fn vertex_cardinality_limits_and_fixed_measurement_kinds_cannot_be_bypassed() {
        for kind in MeasurementKind::ALL {
            let minimum = if matches!(
                kind,
                MeasurementKind::Area | MeasurementKind::Perimeter | MeasurementKind::Angle
            ) {
                3
            } else {
                2
            };
            assert!(geometry_count_valid(kind, minimum));
            assert!(!geometry_count_valid(kind, minimum - 1));
            assert!(!geometry_count_valid(kind, 129));
            let points = if minimum == 3 {
                vec![(20.0, 20.0), (220.0, 20.0), (220.0, 220.0)]
            } else {
                vec![(20.0, 20.0), (220.0, 20.0)]
            };
            let mut draft = editable(kind, points);
            draft.selected = Some(0);
            assert!(!draft.remove_selected());
            if matches!(kind, MeasurementKind::Length | MeasurementKind::Angle) {
                assert!(!draft.can_add());
                assert!(!draft.insert_on_segment((120.0, 20.0), 1.0));
            } else {
                assert!(draft.insert_on_segment((120.0, 20.0), 1.0));
                assert!(draft.remove_selected());
                assert!(!draft.remove_selected());
            }
        }
        let mut maximum = editable(
            MeasurementKind::Polyline,
            (0..128).map(|i| (i as f32 * 20.0, 20.0)).collect(),
        );
        assert!(!maximum.can_add());
        assert!(!maximum.insert_on_segment((50.0, 20.0), 1.0));
        maximum.selected = Some(127);
        assert!(maximum.remove_selected());
        assert_eq!(maximum.selected, Some(126));
        assert!(maximum.can_add());
    }

    #[test]
    fn removing_a_vertex_refuses_self_intersections_and_zero_length_segments_atomically() {
        for (kind, points, selected) in [
            (
                MeasurementKind::Area,
                vec![
                    (0.0, 0.0),
                    (100.0, 0.0),
                    (100.0, 100.0),
                    (50.0, 20.0),
                    (0.0, 100.0),
                ],
                0,
            ),
            (
                MeasurementKind::Polyline,
                vec![(0.0, 0.0), (100.0, 0.0), (0.0, 0.0)],
                1,
            ),
        ] {
            let mut draft = editable(kind, points);
            assert!(draft.replacement.value().is_ok());
            let original = draft.replacement.clone();
            draft.selected = Some(selected);
            assert!(draft.can_remove());
            assert!(!draft.remove_selected());
            assert_eq!(draft.replacement, original);
            assert_eq!(draft.selected, Some(selected));
            assert_eq!(draft.error, Some("measure.geometry.remove_refused"));
        }
    }
}
