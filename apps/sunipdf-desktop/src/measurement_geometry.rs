//! Local vertex editing of an existing measurement, committed through the guarded preview.

use crate::{
    app::{SuniApp, SCREEN_PER_PDF_POINT},
    view::DragEvent,
};
use egui::{Color32, Key, Modifiers, Painter, Pos2, Rect, Stroke};
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementRecord};
use sunipdf_i18n::{tr, tr_args};

const HANDLE_HIT_RADIUS: f32 = 10.0;

#[derive(Clone)]
pub(crate) struct GeometryDraft {
    pub page: usize,
    pub epoch: u64,
    pub original: MeasurementRecord,
    pub replacement: Measurement,
    dragging: Option<usize>,
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
            return None;
        }
        Some(index)
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
                    draft.dragging = draft.hit_vertex(point, scale);
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
            DragEvent::Click(_) | DragEvent::DoubleClick(_) => {}
        }
    }

    /// Also called on pointer release, because the shared Stop event carries no coordinates.
    pub(crate) fn measurement_geometry_move(&mut self, page: usize, point: (f32, f32)) {
        self.validate_measurement_context();
        if !self.measurement_ready() || !point.0.is_finite() || !point.1.is_finite() {
            return;
        }
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
        if !draft.changed() || draft.dragging.is_some() || draft.replacement.value().is_err() {
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
        let ready = self.measurement_ready();
        let draft = self.annotate.measure.geometry.as_ref().unwrap();
        let valid = draft.replacement.value().is_ok();
        let can_finish = ready && draft.changed() && valid && draft.dragging.is_none();
        ui.strong(tr(lang, "measure.geometry.open"));
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
        if !valid {
            ui.colored_label(
                crate::theme::palette(self.settings.dark_theme).error,
                tr(lang, "measure.invalid"),
            );
        }
        let mut finish = false;
        let mut cancel = false;
        ui.horizontal_wrapped(|ui| {
            finish = ui
                .add_enabled(
                    can_finish,
                    egui::Button::new(tr(lang, "measure.geometry.preview")),
                )
                .clicked();
            cancel = ui.button(tr(lang, "measure.cancel_draft")).clicked();
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
        for (index, point) in points.iter().enumerate() {
            let selected = draft.dragging == Some(index);
            painter.circle_filled(
                screen(*point),
                6.0,
                if selected { color } else { Color32::WHITE },
            );
            painter.circle_stroke(screen(*point), 6.0, Stroke::new(2.0, color));
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
                index: 0,
                annotation_id: Some("measure-0".into()),
                measurement: measurement.clone(),
            },
            replacement: measurement,
            dragging: None,
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
    }
}
