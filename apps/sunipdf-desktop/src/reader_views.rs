//! Two locations in one document share its engine, revision and bounded texture cache.

use crate::app::{SuniApp, Zoom};
use crate::view::{ScrollRequest, ViewState};
use egui::{pos2, Pos2, Rect};
use sunipdf_core::{DocumentInfo, TileJob};
use sunipdf_i18n::{tr, tr_args};

pub struct ReaderPane {
    pub page: usize,
    pub zoom: Zoom,
    pub effective_zoom: f32,
    pub page_input: String,
    pub view: ViewState,
}

#[derive(Clone, Copy, PartialEq)]
struct Location {
    page: usize,
    zoom: f32,
    origin: Pos2,
    rotation: crate::reading::ViewRotation,
}

pub struct ReaderViews {
    pub enabled: bool,
    pub synchronized: bool,
    pub active: usize,
    pub other: Option<ReaderPane>,
    pub rects: [Rect; 2],
    pub(crate) collecting: bool,
    pub(crate) wanted: [Vec<TileJob>; 2],
    last_sync: Option<Location>,
}

impl Default for ReaderViews {
    fn default() -> Self {
        Self {
            enabled: false,
            synchronized: false,
            active: 0,
            other: None,
            rects: [Rect::NOTHING; 2],
            collecting: false,
            wanted: Default::default(),
            last_sync: None,
        }
    }
}

impl ReaderViews {
    pub(crate) fn document_changed(
        &mut self,
        info: &DocumentInfo,
        pages_unchanged: bool,
        same_sizes: bool,
    ) {
        self.last_sync = None;
        self.wanted = Default::default();
        if let Some(other) = &mut self.other {
            other.view.reading.hidden_objects.clear();
            other.page = other.page.min(info.page_count().saturating_sub(1));
            other.page_input = info.page_input(other.page);
            // This inactive viewport owns no textures or document identity.
            other
                .view
                .invalidate_navigation(other.page, !pages_unchanged || !same_sizes);
        }
    }

    pub(crate) fn suspended(&mut self) {
        self.last_sync = None;
        self.rects = [Rect::NOTHING; 2];
        self.wanted = Default::default();
        if let Some(other) = &mut self.other {
            other.view.request(ScrollRequest::Offset(other.view.offset));
        }
    }
}

impl SuniApp {
    fn reader_pane_change_ready(&mut self) -> bool {
        if self.document.is_none() || self.ink_blocks_action() || !self.reading_navigation_ready() {
            return false;
        }
        if self.dialog_open()
            || self.opening.is_some()
            || self.tabs.suspending.is_some()
            || self.tabs.resuming.is_some()
            || self.document_transition_pending()
            || self.annotate.drag.is_some()
            || self.annotate.reading_drag.is_some()
            || self.annotate.adv.drag.is_some()
            || self.form_builder.drag.is_some()
            || self.sign.placing.is_some()
            || self.library.placing.is_some()
            || self.worker.has_pending_selections()
            || self.annotate.pending_markup.is_some()
            || self.edit.pending_copy.is_some()
            || self.forms.pending_clear
            || self.forms.pending_import.is_some()
        {
            self.status_msg = Some(self.t("reading.finish_edit").into());
            return false;
        }
        true
    }

    pub(crate) fn toggle_reader_split(&mut self) {
        if !self.reader_pane_change_ready() {
            return;
        }
        if self.reader_views.enabled {
            self.reader_views = ReaderViews::default();
            // The surviving view retains the currently active location.
            self.view.request(ScrollRequest::Offset(self.view.offset));
            return;
        }
        let Some(doc) = &self.document else { return };
        let mut view = ViewState::new(self.view.mode == crate::view::ViewMode::Continuous);
        view.mode = self.view.mode;
        view.reading.rotation = self.view.reading.rotation;
        view.reading.hidden_objects = self.view.reading.hidden_objects.clone();
        let origin = self.reading_page_origin();
        view.request(ScrollRequest::Destination {
            page: doc.page,
            x_pt: origin.x,
            y_from_top_pt: origin.y,
        });
        self.reader_views = ReaderViews {
            enabled: true,
            other: Some(ReaderPane {
                page: doc.page,
                zoom: self.zoom,
                effective_zoom: self.effective_zoom,
                page_input: self.page_input.clone(),
                view,
            }),
            ..Default::default()
        };
        self.view.request(ScrollRequest::Destination {
            page: doc.page,
            x_pt: origin.x,
            y_from_top_pt: origin.y,
        });
    }

    pub(crate) fn toggle_reader_sync(&mut self) {
        if !self.reader_views.enabled || !self.reader_pane_change_ready() {
            return;
        }
        self.reader_views.synchronized = !self.reader_views.synchronized;
        self.reader_views.last_sync = None;
    }

    pub(crate) fn activate_reader_pane(&mut self, pane: usize) -> bool {
        if !self.reader_views.enabled || pane > 1 || pane == self.reader_views.active {
            return false;
        }
        if !self.reader_pane_change_ready() {
            return false;
        }
        self.swap_reader_pane();
        self.reader_views.active = pane;
        self.reader_views.last_sync = None;
        self.annotate.clear_reading();
        self.annotate.adv.selected = None;
        self.annotate.adv.links_page = None;
        self.annotate.annotations_page = None;
        self.edit.reset_for_tab();
        self.edit.page_view = None;
        self.forms.editing = None;
        self.forms.popup = None;
        self.view.follow_thumb = true;
        true
    }

    fn swap_reader_pane(&mut self) {
        let (Some(doc), Some(other)) = (&mut self.document, &mut self.reader_views.other) else {
            return;
        };
        std::mem::swap(&mut doc.page, &mut other.page);
        std::mem::swap(&mut self.zoom, &mut other.zoom);
        std::mem::swap(&mut self.effective_zoom, &mut other.effective_zoom);
        std::mem::swap(&mut self.page_input, &mut other.page_input);
        self.view.swap_navigation(&mut other.view);
    }

    fn synchronize_reader_panes(&mut self) {
        if !self.reader_views.synchronized {
            return;
        }
        let Some(doc) = &self.document else { return };
        let location = Location {
            page: doc.page,
            zoom: self.effective_zoom,
            origin: self.reading_page_origin(),
            rotation: self.view.reading.rotation,
        };
        if self.reader_views.last_sync == Some(location) {
            return;
        }
        self.reader_views.last_sync = Some(location);
        if let Some(other) = &mut self.reader_views.other {
            other.page = location.page;
            other.page_input = doc.info.page_input(location.page);
            other.zoom = Zoom::Fixed(location.zoom);
            if other.view.reading.rotation != location.rotation {
                other.view.reading.history.invalidate_offsets();
            }
            other.view.reading.rotation = location.rotation;
            other.view.request(ScrollRequest::Destination {
                page: location.page,
                x_pt: location.origin.x,
                y_from_top_pt: location.origin.y,
            });
        }
    }

    pub(crate) fn reader_workspace(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if !self.reader_views.enabled || self.reader_views.other.is_none() {
            self.document_view(ctx, ui);
            return;
        }
        let lang = self.lang();
        let mut close = false;
        let mut sync = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(tr(lang, "reader.split_hint"));
            sync = ui
                .selectable_label(self.reader_views.synchronized, tr(lang, "menu.view.sync"))
                .clicked();
            close = ui.button(tr(lang, "reader.close_split")).clicked();
        });
        if close {
            self.toggle_reader_split();
        }
        if sync {
            self.toggle_reader_sync();
        }
        if !self.reader_views.enabled {
            self.document_view(ctx, ui);
            return;
        }
        let available = ui.available_rect_before_wrap();
        let gap = 6.0;
        let stacked = available.width() < 640.0;
        let (first, second) = if stacked {
            let middle = available.center().y;
            (
                Rect::from_min_max(available.min, pos2(available.right(), middle - gap / 2.0)),
                Rect::from_min_max(pos2(available.left(), middle + gap / 2.0), available.max),
            )
        } else {
            let middle = available.center().x;
            (
                Rect::from_min_max(available.min, pos2(middle - gap / 2.0, available.bottom())),
                Rect::from_min_max(pos2(middle + gap / 2.0, available.top()), available.max),
            )
        };
        self.reader_views.rects = [first, second];
        let mut switched = false;
        let activate_at = ctx.input(|input| {
            (input.pointer.any_pressed()
                || input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::MouseWheel { .. }))
                || (input.zoom_delta() - 1.0).abs() > 1e-4)
                .then(|| input.pointer.hover_pos())
                .flatten()
        });
        if let Some(pos) = activate_at
            .filter(|_| !self.dialog_open() && !self.palette.open && !egui::Popup::is_any_open(ctx))
        {
            if let Some(pane) = self
                .reader_views
                .rects
                .iter()
                .position(|rect| rect.contains(pos))
            {
                switched = self.activate_reader_pane(pane);
            }
        }
        self.begin_reader_frame();
        self.reader_views.collecting = true;
        self.reader_views.wanted = Default::default();
        let mut activate_after = None;
        let suppress_edit_click = switched && ctx.input(|input| input.pointer.any_pressed());
        // Draw the active location first: synchronization always flows from user input.
        let active = self.reader_views.active;
        for pane in [active, 1 - active] {
            let passive = pane != active;
            if passive {
                self.synchronize_reader_panes();
                self.swap_reader_pane();
            }
            let rect = self.reader_views.rects[pane];
            ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("reader-pane", pane))
                    .max_rect(rect),
                |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(rect));
                    let page = self.document.as_ref().map_or(0, |doc| doc.page);
                    let label =
                        tr_args(lang, "reader.pane", &[("number", &(pane + 1).to_string())]);
                    ui.horizontal(|ui| {
                        let response = ui.selectable_label(!passive, label);
                        if response.clicked() && passive {
                            activate_after = Some(pane);
                        }
                        response.on_hover_text(tr(lang, "reader.activate_hint"));
                        ui.label(tr_args(
                            lang,
                            "reader.location",
                            &[
                                ("page", &(page + 1).to_string()),
                                ("zoom", &format!("{:.0}", self.effective_zoom * 100.0)),
                            ],
                        ));
                    });
                    let border = if passive {
                        ui.visuals().widgets.noninteractive.bg_stroke
                    } else {
                        ui.visuals().selection.stroke
                    };
                    ui.painter().rect_stroke(
                        rect.shrink(0.5),
                        2.0,
                        border,
                        egui::StrokeKind::Inside,
                    );
                    self.document_view_pane(ctx, ui, pane, !passive && !suppress_edit_click);
                },
            );
            if passive {
                self.swap_reader_pane();
            }
        }
        self.reader_views.collecting = false;
        self.finish_reader_frame();
        if let Some(pane) = activate_after {
            self.activate_reader_pane(pane);
        }
        ui.allocate_rect(available, egui::Sense::hover());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::test_app;
    use crate::reading::ViewRotation;
    use crate::view::ViewMode;
    use crate::worker::{Request, Response};

    fn split_app() -> SuniApp {
        let (mut app, _, _) = test_app();
        let info = &mut app.document.as_mut().unwrap().info;
        info.page_sizes = vec![info.page_sizes[0]; 3];
        info.form = sunipdf_core::FormKind::None;
        app.toggle_reader_split();
        app
    }

    #[test]
    fn switching_keeps_independent_page_zoom_rotation_history_and_document_identity() {
        let mut app = split_app();
        let epoch = app.view.epoch;
        app.view.mode = ViewMode::Single;
        app.go_to_page(1);
        app.zoom = Zoom::Fixed(1.5);
        app.view.reading.rotation = ViewRotation::Clockwise;
        assert!(app.activate_reader_pane(1));
        assert_eq!(app.document.as_ref().unwrap().page, 0);
        assert_eq!(app.zoom, Zoom::FitWidth);
        assert_eq!(app.view.reading.rotation, ViewRotation::Upright);
        assert!(!app.view.reading.history.can_back());
        app.go_to_page(2);
        app.zoom = Zoom::Fixed(0.5);
        assert!(app.activate_reader_pane(0));
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert_eq!(app.zoom, Zoom::Fixed(1.5));
        assert_eq!(app.view.mode, ViewMode::Single);
        assert_eq!(app.view.reading.rotation, ViewRotation::Clockwise);
        assert!(app.view.reading.history.can_back());
        assert_eq!(app.reader_views.other.as_ref().unwrap().page, 2);
        assert_eq!(app.view.epoch, epoch);
    }

    #[test]
    fn synchronization_copies_document_location_once_without_feedback() {
        let mut app = split_app();
        app.go_to_page(2);
        app.effective_zoom = 0.75;
        app.view.reading.rotation = ViewRotation::Clockwise;
        app.view.request(ScrollRequest::Destination {
            page: 2,
            x_pt: 72.0,
            y_from_top_pt: 120.0,
        });
        app.toggle_reader_sync();
        app.synchronize_reader_panes();
        let other = app.reader_views.other.as_ref().unwrap();
        assert_eq!(other.page, 2);
        assert_eq!(other.zoom, Zoom::Fixed(0.75));
        assert_eq!(other.view.reading.rotation, ViewRotation::Clockwise);
        // A passive viewport does not drive the active view or reissue a synchronization.
        app.reader_views.other.as_mut().unwrap().page = 1;
        app.synchronize_reader_panes();
        assert_eq!(app.reader_views.other.as_ref().unwrap().page, 1);
        assert_eq!(app.document.as_ref().unwrap().page, 2);
        app.toggle_reader_sync();
        app.go_to_page(0);
        app.synchronize_reader_panes();
        assert_eq!(app.reader_views.other.as_ref().unwrap().page, 1);
    }

    #[test]
    fn pane_switch_waits_for_selection_reply_and_its_fifo_barrier() {
        let (mut app, requests, responses) = test_app();
        app.toggle_reader_split();
        app.worker.send(Request::ObjectAt {
            page: 0,
            x: 20.0,
            y: 20.0,
        });
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::ObjectAt { .. }
        ));
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishSelection
        ));
        assert!(!app.activate_reader_pane(1));
        responses
            .send(Response::ObjectSelected {
                page: 0,
                info: None,
            })
            .unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(!app.activate_reader_pane(1));
        responses.send(Response::SelectionFinished).unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(app.activate_reader_pane(1));
        assert!(app.edit.selection.is_none());
    }

    #[test]
    fn pane_switch_preserves_pending_forms_and_active_drag() {
        let mut app = split_app();
        app.annotate.note_prompt = Some((0, 1.0, 2.0, "Unfinished note".into()));
        assert!(!app.activate_reader_pane(1));
        assert!(app.annotate.note_prompt.is_some());
        app.annotate.note_prompt = None;
        app.forms.pending_clear = true;
        assert!(!app.activate_reader_pane(1));
        assert!(app.forms.pending_clear);
        app.forms.pending_clear = false;
        app.annotate.drag = Some(crate::annotate::Drag {
            page: 0,
            start: (1.0, 2.0),
            current: (3.0, 4.0),
        });
        assert!(!app.activate_reader_pane(1));
        assert!(app.annotate.drag.is_some());
        app.annotate.drag = None;
        app.tabs.suspending = Some(0);
        assert!(!app.activate_reader_pane(1));
        assert_eq!(app.reader_views.active, 0);
    }

    #[test]
    fn closing_split_preserves_active_location_unsaved_document_and_epoch() {
        let mut app = split_app();
        assert!(app.activate_reader_pane(1));
        app.go_to_page(2);
        app.dirty = true;
        let epoch = app.view.epoch;
        app.toggle_reader_split();
        assert!(!app.reader_views.enabled);
        assert!(app.reader_views.other.is_none());
        assert_eq!(app.document.as_ref().unwrap().page, 2);
        assert!(app.dirty);
        assert_eq!(app.view.epoch, epoch);
    }

    #[test]
    fn tab_suspend_restores_both_locations_and_clamps_after_page_deletion() {
        let mut app = split_app();
        app.reader_views.other.as_mut().unwrap().page = 2;
        app.reader_views.other.as_mut().unwrap().zoom = Zoom::Fixed(2.0);
        app.reader_views.synchronized = true;
        let mut saved = app.take_tab();
        assert!(!app.reader_views.enabled);
        let info = &mut saved.document.as_mut().unwrap().info;
        info.page_sizes.truncate(1);
        saved.reader_views.document_changed(info, false, false);
        app.put_tab(saved);
        assert!(app.reader_views.enabled && app.reader_views.synchronized);
        let other = app.reader_views.other.as_ref().unwrap();
        assert_eq!(other.page, 0);
        assert_eq!(other.page_input, "1");
        assert_eq!(other.zoom, Zoom::Fixed(2.0));
        assert!(other.view.navigation_pending());
    }

    #[test]
    fn both_panes_submit_one_bounded_shared_epoch_queue_and_survive_narrow_layout() {
        for width in [520.0, 1280.0] {
            let (mut app, requests, _) = test_app();
            let info = &mut app.document.as_mut().unwrap().info;
            info.page_sizes = vec![info.page_sizes[0]; 2];
            info.form = sunipdf_core::FormKind::None;
            app.view.mode = ViewMode::Single;
            app.zoom = Zoom::Fixed(0.4);
            app.toggle_reader_split();
            let other = app.reader_views.other.as_mut().unwrap();
            other.page = 1;
            other.view.request(ScrollRequest::Page(1));
            let ctx = egui::Context::default();
            let epoch = app.view.epoch;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 800.0))),
                    time: Some(1.0),
                    ..Default::default()
                },
                |ui| app.reader_workspace(&ctx, ui),
            );
            output.textures_delta.clear();
            let queues: Vec<_> = requests
                .try_iter()
                .filter_map(|request| match request {
                    Request::SetWanted { epoch, jobs } => Some((epoch, jobs)),
                    Request::Edit(_) => panic!("A reader frame must not edit the PDF"),
                    _ => None,
                })
                .collect();
            assert_eq!(queues.len(), 1);
            assert_eq!(queues[0].0, epoch);
            assert!(queues[0].1.len() <= 24);
            assert!(queues[0].1.iter().any(|job| job.page == 0));
            assert!(queues[0].1.iter().any(|job| job.page == 1));
            let [first, second] = app.reader_views.rects;
            assert!(first.is_positive() && second.is_positive());
            assert!(!first.intersects(second));
            assert_eq!(width < 640.0, second.top() > first.bottom());
        }
    }

    #[test]
    fn activation_click_does_not_place_a_note_but_the_next_click_does() {
        let (mut app, requests, _) = test_app();
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.zoom = Zoom::Fixed(0.6);
        app.toggle_reader_split();
        app.annotate.tool = crate::annotate::Tool::Note;
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut run = |app: &mut SuniApp, events| {
            time += 0.5;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ui| app.reader_workspace(&ctx, ui),
            );
            output.textures_delta.clear();
        };
        for _ in 0..3 {
            run(&mut app, vec![]);
        }
        let point = app.reader_views.rects[1].center();
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(
            &mut app,
            vec![egui::Event::PointerMoved(point), button(true)],
        );
        run(&mut app, vec![button(false)]);
        assert_eq!(app.reader_views.active, 1);
        assert!(app.annotate.note_prompt.is_none());
        assert!(app.annotate.drag.is_none());
        assert!(!requests
            .try_iter()
            .any(|request| matches!(request, Request::Edit(_))));
        run(&mut app, vec![button(true)]);
        run(&mut app, vec![button(false)]);
        assert!(
            app.annotate.note_prompt.is_some(),
            "The active pane must remain editable after focusing it"
        );
    }

    #[test]
    fn palette_blocks_underlying_pointer_focus_changes() {
        let mut app = split_app();
        let ctx = egui::Context::default();
        let frame = |app: &mut SuniApp, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
                    events,
                    ..Default::default()
                },
                |ui| app.reader_workspace(&ctx, ui),
            );
            output.textures_delta.clear();
        };
        frame(&mut app, vec![]);
        let point = app.reader_views.rects[1].center();
        app.palette.open = true;
        frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(app.reader_views.active, 0);
    }
}
