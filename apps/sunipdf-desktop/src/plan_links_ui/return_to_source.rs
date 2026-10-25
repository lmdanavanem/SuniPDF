//! One session-only hop back to the tab that opened a verified plan link.

use crate::{
    app::{EngineState, SuniApp, Zoom},
    reading::ViewRotation,
    tabs::TabState,
    view::{ScrollRequest, ViewMode},
};
use std::path::PathBuf;

#[derive(Clone, Copy)]
struct Location {
    page: usize,
    zoom: Zoom,
    effective_zoom: f32,
    scroll: egui::Vec2,
    origin: egui::Pos2,
    mode: ViewMode,
    rotation: ViewRotation,
}

#[derive(Clone)]
pub(crate) struct ReturnOrigin {
    pub(crate) tab_id: u64,
    departure_epoch: u64,
    path: PathBuf,
    page_sizes: Vec<sunipdf_core::PageSize>,
    location: Location,
    other: Option<Location>,
    split: bool,
    synchronized: bool,
    active_pane: usize,
}

impl ReturnOrigin {
    fn matches_ui(&self, ui: &TabState) -> bool {
        ui.document
            .as_ref()
            .is_some_and(|doc| doc.info.path == self.path && doc.info.page_sizes == self.page_sizes)
    }
}

struct ReturnHop {
    origin: ReturnOrigin,
    destination_id: u64,
    pending: bool,
}

#[derive(Default)]
pub(super) struct ReturnState {
    latest: Option<ReturnHop>,
}

impl SuniApp {
    pub(crate) fn capture_plan_return_origin(&self) -> Option<ReturnOrigin> {
        let doc = self.document.as_ref()?;
        let tab = self.tabs.tabs.get(self.tabs.active?)?;
        Some(ReturnOrigin {
            tab_id: tab.recovery_id,
            departure_epoch: self.view.epoch,
            path: doc.info.path.clone(),
            page_sizes: doc.info.page_sizes.clone(),
            location: Location {
                page: doc.page,
                zoom: self.zoom,
                effective_zoom: self.effective_zoom,
                scroll: self.view.offset,
                origin: self.reading_page_origin(),
                mode: self.view.mode,
                rotation: self.view.reading.rotation,
            },
            other: self.reader_views.other.as_ref().map(|pane| Location {
                page: pane.page,
                zoom: pane.zoom,
                effective_zoom: pane.effective_zoom,
                scroll: pane.view.offset,
                origin: pane.view.page_origin(&doc.info, pane.page, pane.zoom),
                mode: pane.view.mode,
                rotation: pane.view.reading.rotation,
            }),
            split: self.reader_views.enabled,
            synchronized: self.reader_views.synchronized,
            active_pane: self.reader_views.active,
        })
    }

    pub(crate) fn plan_return_departure_matches(&self, origin: &ReturnOrigin) -> bool {
        self.view.epoch == origin.departure_epoch
            && self
                .tabs
                .active
                .and_then(|index| self.tabs.tabs.get(index))
                .is_some_and(|tab| tab.recovery_id == origin.tab_id)
            && self.document.as_ref().is_some_and(|doc| {
                doc.info.path == origin.path && doc.info.page_sizes == origin.page_sizes
            })
    }

    pub(crate) fn complete_plan_return_departure(&mut self, origin: ReturnOrigin) {
        let Some(destination) = self.tabs.active.and_then(|index| self.tabs.tabs.get(index)) else {
            return;
        };
        if destination.recovery_id == origin.tab_id {
            return;
        }
        if !self.tabs.tabs.iter().any(|tab| {
            tab.recovery_id == origin.tab_id
                && tab.doc.is_some()
                && tab.ui.as_ref().is_some_and(|ui| origin.matches_ui(ui))
        }) {
            return;
        }
        self.plan_links.return_to_source.latest = Some(ReturnHop {
            origin,
            destination_id: destination.recovery_id,
            pending: false,
        });
    }

    fn plan_return_target(&self) -> Option<usize> {
        let hop = self.plan_links.return_to_source.latest.as_ref()?;
        if hop.pending {
            return None;
        }
        if let Some(active) = self.tabs.active.and_then(|index| self.tabs.tabs.get(index)) {
            if active.recovery_id != hop.destination_id {
                return None;
            }
        } else if !self
            .tabs
            .tabs
            .iter()
            .any(|tab| tab.recovery_id == hop.destination_id)
        {
            return None;
        }
        self.tabs.tabs.iter().position(|tab| {
            tab.recovery_id == hop.origin.tab_id
                && tab.doc.is_some()
                && tab.ui.as_ref().is_some_and(|ui| hop.origin.matches_ui(ui))
        })
    }

    pub(crate) fn plan_return_visible(&self) -> bool {
        self.plan_links.return_to_source.latest.is_some()
    }

    pub(crate) fn plan_return_ready(&self) -> bool {
        matches!(self.engine, EngineState::Ready)
            && !self.navigation_blocked()
            && !self.dialog_open()
            && !self.document_transition_pending()
            && !self.forms.has_unsaved_input()
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && self.plan_return_target().is_some()
    }

    pub(crate) fn return_to_plan_source(&mut self) {
        if !self.plan_return_ready() {
            self.status_msg = Some(self.t("plan_links.return_unavailable").into());
            return;
        }
        let Some(index) = self.plan_return_target() else {
            return;
        };
        self.switch_to(index);
        if self.tabs.resuming == Some(index) {
            if let Some(hop) = &mut self.plan_links.return_to_source.latest {
                hop.pending = true;
            }
        }
    }

    pub(crate) fn plan_return_source_changed(&mut self, tab_id: u64) {
        self.plan_sets.invalidate_return_origin(tab_id);
        if self
            .plan_links
            .return_to_source
            .latest
            .as_ref()
            .is_some_and(|hop| hop.origin.tab_id == tab_id)
        {
            self.plan_links.return_to_source.latest = None;
        }
    }

    pub(crate) fn plan_return_transition_failed(&mut self) {
        if let Some(hop) = &mut self.plan_links.return_to_source.latest {
            hop.pending = false;
        }
    }

    pub(crate) fn plan_return_resume_location(
        &self,
        tab_id: u64,
        info: &sunipdf_core::DocumentInfo,
    ) -> Option<ReturnOrigin> {
        self.plan_links
            .return_to_source
            .latest
            .as_ref()
            .filter(|hop| {
                hop.pending
                    && hop.origin.tab_id == tab_id
                    && info.path == hop.origin.path
                    && info.page_sizes == hop.origin.page_sizes
            })
            .map(|hop| hop.origin.clone())
    }

    pub(crate) fn restore_plan_source_mode(&mut self, origin: &ReturnOrigin) {
        // put_tab otherwise keeps the destination's global mode and discards the source offset.
        self.view.mode = origin.location.mode;
    }

    pub(crate) fn finish_plan_return_resume(&mut self, tab_id: u64, origin: Option<ReturnOrigin>) {
        if let Some(origin) = origin {
            let location = origin.location;
            if let Some(doc) = &mut self.document {
                doc.page = location.page;
                self.page_input = doc.info.page_input(location.page);
            }
            self.zoom = location.zoom;
            self.effective_zoom = location.effective_zoom;
            self.view.mode = location.mode;
            self.view.reading.rotation = location.rotation;
            self.view.offset = location.scroll;
            self.view.request(ScrollRequest::Destination {
                page: location.page,
                x_pt: location.origin.x,
                y_from_top_pt: location.origin.y,
            });
            self.reader_views.enabled = origin.split;
            self.reader_views.synchronized = origin.synchronized;
            self.reader_views.active = origin.active_pane;
            if let (Some(pane), Some(location)) = (&mut self.reader_views.other, origin.other) {
                pane.page = location.page;
                pane.zoom = location.zoom;
                pane.effective_zoom = location.effective_zoom;
                if let Some(doc) = &self.document {
                    pane.page_input = doc.info.page_input(location.page);
                }
                pane.view.mode = location.mode;
                pane.view.reading.rotation = location.rotation;
                pane.view.offset = location.scroll;
                pane.view.request(ScrollRequest::Destination {
                    page: location.page,
                    x_pt: location.origin.x,
                    y_from_top_pt: location.origin.y,
                });
            }
            self.status_msg = Some(self.t("plan_links.returned").into());
        }
        if self
            .plan_links
            .return_to_source
            .latest
            .as_ref()
            .is_some_and(|hop| hop.origin.tab_id == tab_id)
        {
            self.plan_links.return_to_source.latest = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{tests::test_app, OpenDocument},
        reader_views::ReaderPane,
        worker::{DocState, Request, Response, SuspendedDoc},
    };
    use std::sync::mpsc::Receiver;
    use sunipdf_core::{Command, DocumentInfo, EngineError};

    fn suspended(info: &DocumentInfo, name: &[u8]) -> SuspendedDoc {
        SuspendedDoc {
            bytes: name.to_vec(),
            password: None,
            path: info.path.clone(),
            undo: vec![],
            redo: vec![],
            save_point: None,
            modified: true,
            signature_source: None,
        }
    }
    fn state(info: DocumentInfo) -> DocState {
        DocState {
            info,
            can_undo: true,
            can_redo: true,
            pages_unchanged: true,
            measurement_scales_unchanged: true,
            modified: true,
            has_signatures: false,
        }
    }
    fn source(app: &mut SuniApp) {
        let doc = app.document.as_mut().unwrap();
        doc.info.page_sizes = vec![doc.info.page_sizes[0]; 4];
        doc.info.form = sunipdf_core::FormKind::None;
        doc.page = 2;
        app.dirty = true;
        app.zoom = Zoom::FitWidth;
        app.view.mode = ViewMode::Continuous;
        app.view.request(ScrollRequest::Destination {
            page: 2,
            x_pt: 0.0,
            y_from_top_pt: 120.0,
        });
        app.reader_views.enabled = true;
        app.reader_views.active = 1;
        let mut view = crate::view::ViewState::new(true);
        view.request(ScrollRequest::Destination {
            page: 1,
            x_pt: 0.0,
            y_from_top_pt: 90.0,
        });
        app.reader_views.other = Some(ReaderPane {
            page: 1,
            zoom: Zoom::FitWidth,
            effective_zoom: 1.0,
            page_input: "2".into(),
            view,
        });
    }
    fn depart(app: &mut SuniApp) -> DocumentInfo {
        let origin = app.capture_plan_return_origin().unwrap();
        assert!(app.plan_return_departure_matches(&origin));
        let info = app.document.as_ref().unwrap().info.clone();
        let mut target = info.clone();
        target.path = "test/target.pdf".into();
        app.navigate_plan_sheet(crate::plan_sets::Navigation {
            sheet: sunipdf_core::plan_set::VerifiedSheet {
                path: target.path.clone(),
                page: 2,
                bytes: b"verified destination fixture".to_vec(),
                sha256: "a".repeat(64),
            },
            password: None,
            page_count: target.page_count(),
            active_source: None,
            return_source: Some(Box::new(origin)),
        });
        assert!(app.tabs.suspending.is_some());
        app.on_suspended(suspended(&info, b"source unsaved bytes"));
        app.opening = None;
        app.document = Some(OpenDocument {
            info: target.clone(),
            page: 1,
        });
        app.register_opened_tab(target.path.clone());
        app.dirty = true;
        app.view.mode = ViewMode::Single;
        app.zoom = Zoom::Fixed(3.0);
        app.finish_plan_set_navigation();
        info
    }
    fn begin_return(app: &mut SuniApp, requests: &Receiver<Request>) -> SuspendedDoc {
        let destination = app.document.as_ref().unwrap().info.clone();
        requests.try_iter().for_each(drop);
        app.return_to_plan_source();
        assert!(app.tabs.resuming.is_some());
        app.on_suspended(suspended(&destination, b"destination unsaved bytes"));
        requests
            .try_iter()
            .find_map(|request| match request {
                Request::ResumeDoc(doc) => Some(doc),
                Request::Open { .. } | Request::OpenPlanSheet(_) | Request::Save { .. } => {
                    panic!("return must only resume memory")
                }
                _ => None,
            })
            .unwrap()
    }
    fn paint(app: &mut SuniApp, ctx: &egui::Context, width: f32, time: f64) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 720.0),
                )),
                time: Some(time),
                ..Default::default()
            },
            |ui| app.reader_workspace(ctx, ui),
        );
        output.textures_delta.clear();
    }

    #[test]
    fn return_preserves_dirty_tabs_and_both_pdf_locations_after_resize() {
        let (mut app, requests, _) = test_app();
        source(&mut app);
        let ctx = egui::Context::default();
        for frame in 0..3 {
            paint(&mut app, &ctx, 1200.0, frame as f64 * 0.1);
        }
        let main_origin = app.reading_page_origin();
        let other = app.reader_views.other.as_ref().unwrap();
        let other_origin =
            other
                .view
                .page_origin(&app.document.as_ref().unwrap().info, other.page, other.zoom);
        let info = depart(&mut app);
        assert!(app.plan_return_ready());
        let resumed = begin_return(&mut app, &requests);
        assert_eq!(resumed.bytes, b"source unsaved bytes");
        assert!(resumed.modified);
        assert!(
            app.plan_return_visible(),
            "pending return must not be consumed early"
        );
        app.on_resumed(state(info), &ctx);
        assert!(app.dirty && app.can_undo && app.can_redo);
        assert_eq!(app.view.mode, ViewMode::Continuous);
        assert_eq!(app.zoom, Zoom::FitWidth);
        assert!(app.reader_views.enabled);
        assert_eq!(app.reader_views.active, 1);
        for frame in 0..3 {
            paint(&mut app, &ctx, 900.0, 1.0 + frame as f64 * 0.1);
        }
        assert_eq!(app.document.as_ref().unwrap().page, 2);
        assert!((app.reading_page_origin().y - main_origin.y).abs() < 2.0);
        let other = app.reader_views.other.as_ref().unwrap();
        assert_eq!(other.page, 1);
        assert_eq!(other.zoom, Zoom::FitWidth);
        let point =
            other
                .view
                .page_origin(&app.document.as_ref().unwrap().info, other.page, other.zoom);
        assert!((point.y - other_origin.y).abs() < 2.0);
        let target = app
            .tabs
            .tabs
            .iter()
            .find(|tab| tab.path.ends_with("target.pdf"))
            .unwrap();
        assert!(target.dirty && target.doc.as_ref().unwrap().modified);
        assert_eq!(
            target.doc.as_ref().unwrap().bytes,
            b"destination unsaved bytes"
        );
        assert!(!app.plan_return_visible());
    }

    #[test]
    fn return_uses_stable_identity_after_reorder_and_refuses_reopened_path() {
        for reopen in [false, true] {
            let (mut app, requests, _) = test_app();
            source(&mut app);
            depart(&mut app);
            app.tabs.tabs.swap(0, 1);
            app.tabs.active = Some(0);
            if reopen {
                app.tabs.tabs[1].recovery_id = crate::recovery::next_id();
            }
            requests.try_iter().for_each(drop);
            app.return_to_plan_source();
            if reopen {
                assert!(app.tabs.resuming.is_none());
                assert!(requests.try_iter().next().is_none());
            } else {
                assert_eq!(app.tabs.resuming, Some(1));
            }
        }
    }

    #[test]
    fn failed_resume_and_suspension_keep_return_available_for_retry() {
        for resume_failure in [false, true] {
            let (mut app, requests, responses) = test_app();
            source(&mut app);
            let info = depart(&mut app);
            let ctx = egui::Context::default();
            if resume_failure {
                let doc = begin_return(&mut app, &requests);
                app.on_resume_failed(doc);
                assert!(app.document.is_none());
            } else {
                requests.try_iter().for_each(drop);
                app.return_to_plan_source();
                responses
                    .send(Response::SuspendFailed(EngineError::InvalidFormat))
                    .unwrap();
                app.handle_responses(&ctx);
                assert!(app
                    .document
                    .as_ref()
                    .unwrap()
                    .info
                    .path
                    .ends_with("target.pdf"));
            }
            assert!(app.plan_return_ready());
            assert!(!Command::ReturnToPlanSource.info().needs_document);
            requests.try_iter().for_each(drop);
            if resume_failure {
                app.return_to_plan_source();
                assert!(requests
                    .try_iter()
                    .any(|r| matches!(r, Request::ResumeDoc(doc) if doc.modified)));
            } else {
                begin_return(&mut app, &requests);
            }
            app.on_resumed(state(info), &ctx);
            assert!(!app.plan_return_visible());
            assert!(app.dirty);
        }
    }

    #[test]
    fn manual_source_activation_consumes_return_without_forcing_the_saved_mode() {
        let (mut app, requests, _) = test_app();
        source(&mut app);
        let info = depart(&mut app);
        let destination = app.document.as_ref().unwrap().info.clone();
        app.switch_to(0);
        app.on_suspended(suspended(&destination, b"destination unsaved bytes"));
        requests.try_iter().for_each(drop);
        app.on_resumed(state(info), &egui::Context::default());
        assert_eq!(app.view.mode, ViewMode::Single);
        assert!(!app.plan_return_visible());
    }

    #[test]
    fn changed_source_context_and_pending_edits_refuse_return_without_discarding_work() {
        for blocked in ["source", "geometry", "pending_file", "note", "form_dialog"] {
            let (mut app, requests, _) = test_app();
            source(&mut app);
            depart(&mut app);
            match blocked {
                "source" => app.plan_return_source_changed(app.tabs.tabs[0].recovery_id),
                "geometry" => {
                    app.tabs.tabs[0]
                        .ui
                        .as_mut()
                        .unwrap()
                        .document
                        .as_mut()
                        .unwrap()
                        .info
                        .page_sizes
                        .pop();
                }
                "pending_file" => app.pending_files.push("queued.pdf".into()),
                "note" => app.annotate.note_prompt = Some((0, 1.0, 2.0, "draft note".into())),
                "form_dialog" => app.form_builder.visible = true,
                _ => unreachable!(),
            }
            requests.try_iter().for_each(drop);
            app.execute(&egui::Context::default(), Command::ReturnToPlanSource);
            assert!(requests.try_iter().next().is_none(), "{blocked}");
            assert!(app
                .document
                .as_ref()
                .unwrap()
                .info
                .path
                .ends_with("target.pdf"));
            if blocked == "note" {
                assert!(app.annotate.note_prompt.is_some());
            }
            if blocked == "form_dialog" {
                assert!(app.form_builder.visible);
            }
            if blocked == "pending_file" {
                assert_eq!(app.pending_files.len(), 1);
            }
        }
    }

    #[test]
    fn unsuccessful_departure_and_preview_cleanup_preserve_the_last_successful_return() {
        let (mut app, _, _) = test_app();
        source(&mut app);
        depart(&mut app);
        let previous = app
            .plan_links
            .return_to_source
            .latest
            .as_ref()
            .unwrap()
            .origin
            .tab_id;
        app.close_plan_links();
        let unsuccessful = app.capture_plan_return_origin().unwrap();
        app.complete_plan_return_departure(unsuccessful); // Same active tab cannot create a hop.
        app.plan_sets.navigation_failed();
        assert_eq!(
            app.plan_links
                .return_to_source
                .latest
                .as_ref()
                .unwrap()
                .origin
                .tab_id,
            previous
        );
        assert!(app.plan_return_ready());
        assert_eq!(
            Command::from_id("view.return_plan_source"),
            Some(Command::ReturnToPlanSource)
        );
        for language in sunipdf_i18n::Language::ALL {
            for key in [
                "plan_links.return_source",
                "plan_links.return_hint",
                "plan_links.returned",
                "plan_links.return_unavailable",
            ] {
                assert_ne!(sunipdf_i18n::tr(language, key), key);
            }
        }
    }
}
