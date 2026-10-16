//! Reading-only orientation and bounded, document-local navigation history.

use crate::app::{SuniApp, Zoom};
use crate::view::ScrollRequest;
use egui::{pos2, vec2, Pos2, Rect, Vec2};
use sunipdf_core::{Destination, DestinationView, PageSize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewRotation {
    #[default]
    Upright,
    Clockwise,
    UpsideDown,
    Counterclockwise,
}

impl ViewRotation {
    pub fn turn(self, clockwise: bool) -> Self {
        let turns = match self {
            Self::Upright => 0,
            Self::Clockwise => 1,
            Self::UpsideDown => 2,
            Self::Counterclockwise => 3,
        };
        match (turns + if clockwise { 1 } else { 3 }) % 4 {
            0 => Self::Upright,
            1 => Self::Clockwise,
            2 => Self::UpsideDown,
            _ => Self::Counterclockwise,
        }
    }

    pub fn size(self, size: PageSize) -> PageSize {
        if matches!(self, Self::Clockwise | Self::Counterclockwise) {
            PageSize {
                width_pt: size.height_pt,
                height_pt: size.width_pt,
            }
        } else {
            size
        }
    }

    /// Top-left source coordinates to top-left display coordinates.
    pub fn point(self, p: Pos2, size: Vec2) -> Pos2 {
        match self {
            Self::Upright => p,
            Self::Clockwise => pos2(size.y - p.y, p.x),
            Self::UpsideDown => pos2(size.x - p.x, size.y - p.y),
            Self::Counterclockwise => pos2(p.y, size.x - p.x),
        }
    }

    pub fn inverse_point(self, p: Pos2, source_size: Vec2) -> Pos2 {
        match self {
            Self::Upright => p,
            Self::Clockwise => pos2(p.y, source_size.y - p.x),
            Self::UpsideDown => pos2(source_size.x - p.x, source_size.y - p.y),
            Self::Counterclockwise => pos2(source_size.x - p.y, p.x),
        }
    }

    pub fn rect(self, rect: Rect, source_size: Vec2) -> Rect {
        Rect::from_two_pos(
            self.point(rect.min, source_size),
            self.point(rect.max, source_size),
        )
    }

    pub fn inverse_rect(self, rect: Rect, source_size: Vec2) -> Rect {
        if !rect.is_positive() {
            return Rect::NOTHING;
        }
        Rect::from_two_pos(
            self.inverse_point(rect.min, source_size),
            self.inverse_point(rect.max, source_size),
        )
    }

    pub fn source_size(self, display_size: Vec2) -> Vec2 {
        if matches!(self, Self::Clockwise | Self::Counterclockwise) {
            vec2(display_size.y, display_size.x)
        } else {
            display_size
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Location {
    page: usize,
    offset: Option<Vec2>,
    zoom: Zoom,
    point: Option<Pos2>,
}

#[derive(Default)]
pub struct NavigationHistory {
    entries: Vec<Location>,
    current: usize,
    restoring: bool,
}

impl NavigationHistory {
    pub fn can_back(&self) -> bool {
        self.current > 0
    }
    pub fn can_forward(&self) -> bool {
        self.current + 1 < self.entries.len()
    }

    pub fn jump(&mut self, from: usize, offset: Option<Vec2>, to: usize, zoom: f32) {
        self.record_jump(from, offset, to, zoom, false);
    }

    fn record_jump(
        &mut self,
        from: usize,
        offset: Option<Vec2>,
        to: usize,
        zoom: f32,
        same_page: bool,
    ) {
        if self.restoring {
            return;
        }
        if let Some(offset) = offset
            .filter(|_| self.entries.is_empty() || self.entries[self.current].offset.is_some())
        {
            self.observe(from, offset, zoom);
        }
        if self.entries.is_empty() {
            self.entries.push(Location {
                page: from,
                offset: None,
                zoom: Zoom::Fixed(zoom),
                point: None,
            });
        }
        if from == to && !same_page {
            return;
        }
        self.entries.truncate(self.current + 1);
        self.entries.push(Location {
            page: to,
            offset: None,
            zoom: Zoom::Fixed(zoom),
            point: None,
        });
        if self.entries.len() > 256 {
            self.entries.remove(0);
        }
        self.current = self.entries.len() - 1;
    }

    pub fn observe(&mut self, page: usize, offset: Vec2, zoom: f32) {
        if !offset.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
            return;
        }
        let location = Location {
            page,
            offset: Some(offset.max(Vec2::ZERO)),
            zoom: Zoom::Fixed(zoom),
            point: None,
        };
        if self.entries.is_empty() {
            self.entries.push(location);
        } else {
            self.entries[self.current] = location;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn invalidate_offsets(&mut self) {
        for entry in &mut self.entries {
            entry.offset = None;
        }
    }

    fn step(&mut self, back: bool) -> Option<Location> {
        if if back {
            !self.can_back()
        } else {
            !self.can_forward()
        } {
            return None;
        }
        self.current = if back {
            self.current - 1
        } else {
            self.current + 1
        };
        Some(self.entries[self.current])
    }
}

#[derive(Default)]
pub struct ReadingState {
    pub rotation: ViewRotation,
    pub history: NavigationHistory,
}

impl SuniApp {
    pub fn go_to_destination(&mut self, destination: Destination) {
        if !self.reading_navigation_ready() {
            return;
        }
        let Some(doc) = &self.document else { return };
        let Some(size) = doc.info.page_sizes.get(destination.page).copied() else {
            return;
        };
        let from = doc.page;
        let origin = self.reading_page_origin();
        let source_height = doc
            .info
            .page_sizes
            .get(from)
            .map_or(size.height_pt, |s| s.height_pt);
        let (x, y, zoom) = match destination.view {
            DestinationView::Xyz { x, y, zoom } => (x, y, zoom.map(Zoom::Fixed)),
            DestinationView::FitPage => (Some(0.0), Some(size.height_pt), Some(Zoom::FitPage)),
            DestinationView::FitWidth { x, y } => (x, y, Some(Zoom::FitWidth)),
        };
        if x.is_some_and(|v| !v.is_finite())
            || y.is_some_and(|v| !v.is_finite())
            || matches!(zoom, Some(Zoom::Fixed(v)) if !v.is_finite() || v <= 0.0)
        {
            return;
        }
        let x_pt = x.unwrap_or(origin.x).clamp(0.0, size.width_pt);
        let y_from_top_pt =
            (size.height_pt - y.unwrap_or(source_height - origin.y)).clamp(0.0, size.height_pt);
        self.view.reading.history.record_jump(
            from,
            (!self.view.navigation_pending()).then_some(self.view.offset),
            destination.page,
            self.effective_zoom,
            true,
        );
        self.view.reading.history.restoring = true;
        self.go_to_page(destination.page);
        self.view.reading.history.restoring = false;
        self.zoom = match zoom {
            Some(zoom) => match zoom {
                Zoom::Fixed(z) => {
                    Zoom::Fixed(z.clamp(crate::view::ZOOM_MIN, crate::view::ZOOM_MAX))
                }
                fit => fit,
            },
            None => match self.zoom {
                Zoom::Fixed(z) => Zoom::Fixed(z),
                _ => Zoom::Fixed(self.effective_zoom),
            },
        };
        let point = (!matches!(destination.view, DestinationView::FitPage))
            .then_some(pos2(x_pt, y_from_top_pt));
        if let Some(entry) = self.view.reading.history.entries.last_mut() {
            entry.zoom = self.zoom;
            entry.point = point;
        }
        if let Some(point) = point {
            self.view.request(ScrollRequest::Destination {
                page: destination.page,
                x_pt: point.x,
                y_from_top_pt: point.y,
            });
        }
    }

    pub(crate) fn reading_navigation_ready(&mut self) -> bool {
        if self.navigation_blocked() {
            return false;
        }
        if self.edit.text_prompt.is_some()
            || self.forms.has_unsaved_input()
            || self.worker.has_pending_mutations()
        {
            self.status_msg =
                Some(sunipdf_i18n::tr(self.lang(), "reading.finish_edit").to_string());
            return false;
        }
        true
    }

    pub fn rotate_reading_view(&mut self, clockwise: bool) {
        let Some(page) = self.document.as_ref().map(|d| d.page) else {
            return;
        };
        if !self.reading_navigation_ready() {
            return;
        }
        // Editing controls use the document's upright display coordinates.
        self.annotate.tool = crate::annotate::Tool::Select;
        self.annotate.drag = None;
        self.annotate.adv.selected = None;
        self.annotate.adv.drag = None;
        self.annotate.adv.ink.clear();
        self.annotate.clear_reading();
        self.forms.editing = None;
        self.forms.popup = None;
        self.edit.reset_for_tab();
        self.cancel_placing();
        self.view.reading.rotation = self.view.reading.rotation.turn(clockwise);
        self.view.orientation_changed(page);
    }

    pub fn navigate_reading_history(&mut self, back: bool) {
        if !self.reading_navigation_ready() {
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        if !self.view.navigation_pending() {
            self.view
                .reading
                .history
                .observe(doc.page, self.view.offset, self.effective_zoom);
        }
        let Some(location) = self.view.reading.history.step(back) else {
            return;
        };
        self.forms.editing = None;
        self.forms.popup = None;
        self.view.reading.history.restoring = true;
        self.zoom = location.zoom;
        self.go_to_page(location.page);
        self.view.reading.history.restoring = false;
        if let Some(offset) = location.offset {
            self.view.request(ScrollRequest::Offset(offset));
        } else if let Some(point) = location.point {
            self.view.request(ScrollRequest::Destination {
                page: location.page,
                x_pt: point.x,
                y_from_top_pt: point.y,
            });
        }
    }

    pub fn set_reading_full_screen(&mut self, ctx: &egui::Context, enabled: bool) {
        self.reading_full_screen = enabled;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(enabled));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROTATIONS: [ViewRotation; 4] = [
        ViewRotation::Upright,
        ViewRotation::Clockwise,
        ViewRotation::UpsideDown,
        ViewRotation::Counterclockwise,
    ];

    #[test]
    fn same_page_destinations_keep_both_scroll_positions_and_zoom_in_history() {
        let mut history = NavigationHistory::default();
        history.record_jump(0, Some(vec2(0.0, 120.0)), 0, 1.0, true);
        history.observe(0, vec2(80.0, 900.0), 2.0);
        let previous = history.step(true).unwrap();
        assert_eq!(previous.page, 0);
        assert_eq!(previous.offset, Some(vec2(0.0, 120.0)));
        assert_eq!(previous.zoom, Zoom::Fixed(1.0));
        let next = history.step(false).unwrap();
        assert_eq!(next.offset, Some(vec2(80.0, 900.0)));
        assert_eq!(next.zoom, Zoom::Fixed(2.0));
        history.invalidate_offsets();
        assert!(history.step(true).unwrap().offset.is_none());
    }

    #[test]
    fn rapid_destination_jumps_restore_unpainted_points_and_requested_zoom() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let height = app.document.as_ref().unwrap().info.page_sizes[0].height_pt;
        app.view.offset = vec2(0.0, 40.0);
        for (x, top, zoom) in [(50.0, 100.0, 2.0), (80.0, 200.0, 3.0)] {
            app.go_to_destination(Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x: Some(x),
                    y: Some(height - top),
                    zoom: Some(zoom),
                },
            });
        }
        assert_eq!(app.zoom, Zoom::Fixed(3.0));
        app.navigate_reading_history(true);
        assert_eq!(app.zoom, Zoom::Fixed(2.0));
        assert_eq!(app.reading_page_origin(), pos2(50.0, 100.0));
        app.navigate_reading_history(true);
        assert_eq!(app.zoom, Zoom::Fixed(1.0));
        assert_eq!(
            app.view.reading.history.entries[0].offset,
            Some(vec2(0.0, 40.0))
        );
        assert!(!app.dirty);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn destination_immediately_after_back_uses_the_restored_location_not_old_frame() {
        let (mut app, _, _) = crate::app::tests::test_app();
        app.view
            .reading
            .history
            .record_jump(0, Some(vec2(0.0, 200.0)), 0, 1.0, true);
        app.view.reading.history.observe(0, vec2(80.0, 1000.0), 2.0);
        app.view.offset = vec2(80.0, 1000.0);
        app.effective_zoom = 2.0;
        app.zoom = Zoom::Fixed(2.0);
        app.navigate_reading_history(true);
        assert_eq!(app.zoom, Zoom::Fixed(1.0));
        let origin = app.reading_page_origin();
        assert!(origin.y > 100.0 && origin.y < 200.0);
        app.go_to_destination(Destination {
            page: 0,
            view: DestinationView::Xyz {
                x: None,
                y: None,
                zoom: None,
            },
        });
        assert_eq!(app.zoom, Zoom::Fixed(1.0));
        assert_eq!(app.reading_page_origin(), origin);
        app.navigate_reading_history(true);
        let previous = app.view.reading.history.entries[app.view.reading.history.current];
        assert_eq!(previous.offset, Some(vec2(0.0, 200.0)));
        assert_eq!(app.zoom, Zoom::Fixed(1.0));
    }

    #[test]
    fn invalid_destinations_and_uncommitted_input_do_not_navigate() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        for destination in [
            Destination {
                page: usize::MAX,
                view: DestinationView::FitPage,
            },
            Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x: Some(f32::NAN),
                    y: None,
                    zoom: None,
                },
            },
            Destination {
                page: 0,
                view: DestinationView::Xyz {
                    x: None,
                    y: None,
                    zoom: Some(f32::INFINITY),
                },
            },
        ] {
            app.go_to_destination(destination);
            assert!(!app.view.navigation_pending());
            assert!(!app.view.reading.history.can_back());
        }
        crate::app::tests::form_draft(&mut app, "New");
        app.go_to_destination(Destination {
            page: 0,
            view: DestinationView::FitPage,
        });
        assert_eq!(app.forms.drafts["Name"], "New");
        assert!(!app.view.navigation_pending());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn missing_destination_axes_and_zoom_retain_the_current_view() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let height = app.document.as_ref().unwrap().info.page_sizes[0].height_pt;
        app.zoom = Zoom::Fixed(2.0);
        app.go_to_destination(Destination {
            page: 0,
            view: DestinationView::Xyz {
                x: Some(50.0),
                y: Some(height - 100.0),
                zoom: None,
            },
        });
        app.go_to_destination(Destination {
            page: 0,
            view: DestinationView::Xyz {
                x: None,
                y: Some(height - 200.0),
                zoom: None,
            },
        });
        assert_eq!(app.zoom, Zoom::Fixed(2.0));
        assert_eq!(app.reading_page_origin(), pos2(50.0, 200.0));
        app.zoom = Zoom::FitWidth;
        app.effective_zoom = 1.25;
        app.go_to_destination(Destination {
            page: 0,
            view: DestinationView::Xyz {
                x: None,
                y: None,
                zoom: None,
            },
        });
        assert_eq!(app.zoom, Zoom::Fixed(1.25));
        assert_eq!(app.reading_page_origin(), pos2(50.0, 200.0));
    }

    #[test]
    fn quarter_turns_round_trip_pdf_display_points_and_rectangles() {
        let source = vec2(612.0, 792.0);
        for rotation in ROTATIONS {
            for p in [Pos2::ZERO, pos2(612.0, 792.0), pos2(44.0, 621.0)] {
                assert_eq!(rotation.inverse_point(rotation.point(p, source), source), p);
            }
            let r = Rect::from_min_max(pos2(20.0, 30.0), pos2(80.0, 110.0));
            assert_eq!(rotation.inverse_rect(rotation.rect(r, source), source), r);
            assert_eq!(rotation.turn(true).turn(false), rotation);
            assert_eq!(
                rotation.turn(true).turn(true).turn(true).turn(true),
                rotation
            );
        }
        assert_eq!(
            ViewRotation::Clockwise.point(Pos2::ZERO, source),
            pos2(792.0, 0.0)
        );
        assert_eq!(
            ViewRotation::Clockwise.point(pos2(612.0, 0.0), source),
            pos2(792.0, 612.0)
        );
    }

    #[test]
    fn history_keeps_offsets_and_truncates_forward_after_a_new_jump() {
        let mut h = NavigationHistory::default();
        h.jump(0, Some(vec2(0.0, 10.0)), 8, 1.0);
        h.observe(8, vec2(5.0, 800.0), 1.0);
        h.jump(8, Some(vec2(5.0, 800.0)), 3, 1.0);
        let previous = h.step(true).unwrap();
        assert_eq!(previous.page, 8);
        assert_eq!(previous.offset, Some(vec2(5.0, 800.0)));
        assert!(h.can_forward());
        h.jump(8, Some(vec2(5.0, 810.0)), 9, 1.0);
        assert!(!h.can_forward());
        assert_eq!(h.step(true).unwrap().offset, Some(vec2(5.0, 810.0)));
        assert_eq!(h.step(true).unwrap().page, 0);
        assert!(!h.can_back());
    }

    #[test]
    fn history_is_bounded_and_repeated_pages_do_not_add_entries() {
        let mut h = NavigationHistory::default();
        for page in 0..1000 {
            h.jump(page, Some(Vec2::ZERO), page + 1, 1.0);
        }
        assert_eq!(h.entries.len(), 256);
        h.jump(1000, Some(Vec2::ZERO), 1000, 1.0);
        assert_eq!(h.entries.len(), 256);
        h.reset();
        assert!(!h.can_back() && !h.can_forward());
    }

    #[test]
    fn rapid_navigation_does_not_assign_the_previous_page_offset_to_an_unshown_page() {
        let mut h = NavigationHistory::default();
        h.jump(0, Some(vec2(0.0, 40.0)), 8, 1.0);
        h.jump(8, Some(vec2(0.0, 40.0)), 3, 1.0);
        let location = h.step(true).unwrap();
        assert_eq!(location.page, 8);
        assert!(location.offset.is_none());
        h.invalidate_offsets();
        assert_eq!(h.step(true).unwrap().page, 0);
    }

    #[test]
    fn rotation_and_history_preserve_uncommitted_form_input() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.view.reading.history.jump(0, Some(Vec2::ZERO), 4, 1.0);
        crate::app::tests::form_draft(&mut app, "New");
        app.rotate_reading_view(true);
        app.navigate_reading_history(true);
        assert_eq!(app.view.reading.rotation, ViewRotation::Upright);
        assert_eq!(app.forms.drafts["Name"], "New");
        assert!(app.view.reading.history.can_back());
        assert_eq!(app.document.as_ref().unwrap().page, 0);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn reading_actions_change_only_view_state_and_emit_full_screen_commands() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.rotate_reading_view(true);
        assert_eq!(app.view.reading.rotation, ViewRotation::Clockwise);
        assert!(!app.dirty);
        assert!(requests.try_recv().is_err());
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.set_reading_full_screen(ui.ctx(), true)
        });
        out.textures_delta.clear();
        assert!(app.reading_full_screen);
        assert!(out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(true))));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.set_reading_full_screen(ui.ctx(), false)
        });
        out.textures_delta.clear();
        assert!(!app.reading_full_screen);
        assert!(out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(false))));
    }

    #[test]
    fn full_screen_entry_selects_text_and_exit_remains_available() {
        let (mut app, _, _) = crate::app::tests::test_app();
        app.annotate.tool = crate::annotate::Tool::AddText;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.execute(ui.ctx(), sunipdf_core::Command::FullScreenReading);
        });
        out.textures_delta.clear();
        assert!(app.reading_full_screen);
        assert_eq!(app.annotate.tool, crate::annotate::Tool::Select);
        assert!(out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(true))));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.execute(ui.ctx(), sunipdf_core::Command::FullScreenReading);
        });
        out.textures_delta.clear();
        assert!(!app.reading_full_screen);
        assert!(out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(false))));
        assert!(!app.dirty);
    }

    #[test]
    fn uncommitted_form_blocks_full_screen_entry_but_allows_exit() {
        let (mut app, _, _) = crate::app::tests::test_app();
        crate::app::tests::form_draft(&mut app, "New");
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.execute(ui.ctx(), sunipdf_core::Command::FullScreenReading);
        });
        out.textures_delta.clear();
        assert!(!app.reading_full_screen);
        assert_eq!(app.forms.drafts["Name"], "New");
        assert!(!out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(true))));
        app.reading_full_screen = true;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.execute(ui.ctx(), sunipdf_core::Command::FullScreenReading);
        });
        out.textures_delta.clear();
        assert!(!app.reading_full_screen);
        assert_eq!(app.forms.drafts["Name"], "New");
        assert!(out.viewport_output.values().any(|v| v
            .commands
            .contains(&egui::ViewportCommand::Fullscreen(false))));
    }
}
