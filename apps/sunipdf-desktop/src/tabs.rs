//! Onglets multi-documents. Un seul document est « actif » dans le moteur ; les autres sont
//! suspendus (octets PDF + historique d'annulation) et leur état d'interface est conservé.

use crate::app::{EngineState, OpenDocument, SuniApp, Zoom};
use crate::worker::{Request, SuspendedDoc};
use std::collections::HashMap;
use std::path::PathBuf;

/// État d'interface propre à un document, échangé lors d'un changement d'onglet.
pub struct TabState {
    pub document: Option<OpenDocument>,
    pub zoom: Zoom,
    pub effective_zoom: f32,
    /// Décalage de la zone de défilement du document (l'état egui est partagé entre onglets).
    pub scroll: egui::Vec2,
    /// A queued geometry edit must return to a valid page, not an obsolete offset.
    scroll_to_page: Option<usize>,
    pub view_mode: crate::view::ViewMode,
    pub reading: crate::reading::ReadingState,
    pub reader_views: crate::reader_views::ReaderViews,
    pub page_input: String,
    pub thumbnails: HashMap<usize, egui::TextureHandle>,
    pub thumb_px: u32,
    pub find: crate::app::FindState,
    pub dirty: bool,
    pub status_msg: Option<String>,
    pub organize: crate::organize::OrganizeState,
    pub annotate: crate::annotate::AnnotateState,
    pub docinfo: crate::docinfo::DocInfoState,
    pub forms: crate::forms::FormsState,
    pub can_undo: bool,
    pub can_redo: bool,
}

pub struct Tab {
    pub path: PathBuf,
    pub dirty: bool,
    pub recovery_id: u64,
    /// État d'interface conservé pendant la suspension (`None` pour l'onglet actif).
    pub ui: Option<TabState>,
    /// Document suspendu (`None` tant que le moteur ne l'a pas renvoyé, ou pour l'onglet actif).
    pub doc: Option<SuspendedDoc>,
}

#[derive(Default)]
pub struct TabsState {
    pub tabs: Vec<Tab>,
    /// Index de l'onglet actif dans `tabs` (`None` : accueil).
    pub active: Option<usize>,
    /// Onglet dont le document est en cours de suspension (attend `Response::Suspended`).
    pub suspending: Option<usize>,
    /// Onglet en cours de reprise (attend `Response::Resumed`).
    pub resuming: Option<usize>,
    /// Après la reprise d'un onglet modifié : redemander la confirmation de fermeture (Quitter).
    pub quit_pending: bool,
}

impl SuniApp {
    /// Retire l'état d'interface du document actif (prêt à être stocké dans un onglet).
    pub fn take_tab(&mut self) -> TabState {
        self.form_builder.cancel();
        let bookmarks_open = self.docinfo.bookmarks_open;
        let comments_open = self.annotate.comments_open;
        let forms_open = self.forms.panel_open;
        let scroll = self.view.offset;
        let view_mode = self.view.mode;
        let reading = std::mem::take(&mut self.view.reading);
        self.reader_views.suspended();
        let reader_views = std::mem::take(&mut self.reader_views);
        // Textures libérées (GPU des onglets inactifs), époque suivante : tuiles en vol ignorées.
        self.view.reset();
        self.thumbs_pending.clear();
        self.thumbs_failed.clear();
        // La sélection d'objet désigne un index du document actif : elle ne doit jamais
        // survivre à un changement de document (Suppr agirait sur l'autre onglet).
        self.edit.reset_for_tab();
        self.pagesetup.close_labels();
        self.annotate.adv.link = None;
        self.annotate.adv.free_text = None;
        // Le placement d'une signature vise une page du document actif : il ne survit pas.
        self.cancel_placing();
        self.annotate.clear_reading();
        let mut state = TabState {
            document: self.document.take(),
            zoom: std::mem::replace(&mut self.zoom, Zoom::FitWidth),
            effective_zoom: std::mem::replace(&mut self.effective_zoom, 1.0),
            scroll,
            scroll_to_page: None,
            view_mode,
            reading,
            reader_views,
            page_input: std::mem::take(&mut self.page_input),
            thumbnails: std::mem::take(&mut self.thumbnails),
            thumb_px: std::mem::replace(&mut self.thumb_px, 0),
            find: std::mem::take(&mut self.find),
            dirty: std::mem::replace(&mut self.dirty, false),
            status_msg: self.status_msg.take(),
            organize: std::mem::take(&mut self.organize),
            annotate: std::mem::take(&mut self.annotate),
            docinfo: std::mem::take(&mut self.docinfo),
            forms: std::mem::take(&mut self.forms),
            can_undo: std::mem::replace(&mut self.can_undo, false),
            can_redo: std::mem::replace(&mut self.can_redo, false),
        };
        // Une recherche en vol sera relancée au retour sur l'onglet (sa réponse est ignorée).
        state.find.reset_in_flight();
        self.docinfo.bookmarks_open = bookmarks_open;
        self.annotate.comments_open = comments_open;
        self.forms.panel_open = forms_open;
        state
    }

    pub fn put_tab(&mut self, t: TabState) {
        self.form_builder.cancel();
        self.edit.reset_for_tab();
        let bookmarks_open = self.docinfo.bookmarks_open;
        let comments_open = self.annotate.comments_open;
        let forms_open = self.forms.panel_open;
        self.document = t.document;
        self.zoom = t.zoom;
        self.effective_zoom = t.effective_zoom;
        self.view.reset();
        self.view.reading = t.reading;
        if !self.view.reading.hidden_objects.is_empty() {
            self.worker.send(Request::SetHiddenObjects {
                epoch: self.view.epoch,
                hidden: self.view.reading.hidden_objects.clone(),
            });
        }
        self.reader_views = t.reader_views;
        if let Some(other) = &mut self.reader_views.other {
            other.view.reading.hidden_objects = self.view.reading.hidden_objects.clone();
        }
        if let Some(page) = t.scroll_to_page {
            self.view.request(crate::view::ScrollRequest::Page(page));
        } else if self.view.mode == t.view_mode {
            self.view
                .request(crate::view::ScrollRequest::Offset(t.scroll));
        } else {
            self.view.reading.history.invalidate_offsets();
            let page = self.document.as_ref().map_or(0, |doc| doc.page);
            self.view.request(crate::view::ScrollRequest::Page(page));
        }
        // Le panneau des miniatures est partagé entre onglets : ramener la page de cet onglet.
        self.view.follow_thumb = true;
        self.page_input = t.page_input;
        self.thumbnails = t.thumbnails;
        self.thumbs_pending.clear();
        self.thumbs_failed.clear();
        self.thumb_px = t.thumb_px;
        self.find = t.find;
        self.dirty = t.dirty;
        self.status_msg = t.status_msg;
        self.organize = t.organize;
        self.annotate = t.annotate;
        // Suspension cancels the engine inventory, and edits queued before it may
        // have changed a previously completed report. Keep filters, reload rows.
        self.annotate.register.invalidate();
        // The scales belong to this suspended document, not the shared viewport epoch.
        self.annotate.measure.advance_epoch(self.view.epoch);
        self.annotate.clear_reading();
        // Le moteur a rechargé le document : indices et zones de lien relus.
        self.annotate.adv.invalidate();
        self.annotate.annotations_page = None;
        self.docinfo = t.docinfo;
        self.docinfo.bookmarks_open = bookmarks_open;
        self.annotate.comments_open = comments_open;
        // Le document a été rechargé par le moteur : les champs sont relus, la réponse en vol
        // d'un autre onglet est ignorée.
        self.forms = t.forms;
        self.forms.panel_open = forms_open;
        self.forms.stale = true;
        self.forms.requested = false;
        self.can_undo = t.can_undo;
        self.can_redo = t.can_redo;
        let _ = std::mem::take(&mut self.thumbnails); // miniatures : recalculées (le cache moteur a été vidé)
    }

    /// Suspend le document actif dans un onglet (nouvel onglet ou onglet existant).
    pub fn suspend_active(&mut self) -> bool {
        if self.ink_blocks_action() {
            return false;
        }
        if self.navigation_blocked() {
            return false;
        }
        let Some(path) = self.document.as_ref().map(|doc| doc.info.path.clone()) else {
            return true;
        };
        if !self.commit_form_input() {
            return false;
        }
        let dirty = self.dirty;
        let state = self.take_tab();
        let index = match self.tabs.active {
            Some(i) if i < self.tabs.tabs.len() => {
                let tab = &mut self.tabs.tabs[i];
                tab.ui = Some(state);
                tab.dirty = dirty;
                tab.path = path;
                i
            }
            _ => {
                self.tabs.tabs.push(Tab {
                    path,
                    dirty,
                    recovery_id: crate::recovery::next_id(),
                    ui: Some(state),
                    doc: None,
                });
                self.tabs.tabs.len() - 1
            }
        };
        self.tabs.active = None;
        self.tabs.suspending = Some(index);
        self.worker.send(Request::Suspend);
        true
    }

    /// Active l'onglet `index` (le document actif, s'il existe, est suspendu).
    pub fn switch_to(&mut self, index: usize) {
        if self.ink_blocks_action() {
            return;
        }
        if index >= self.tabs.tabs.len()
            || self.tabs.active == Some(index)
            || !matches!(self.engine, EngineState::Ready)
            || self.navigation_blocked()
        {
            return;
        }
        if self.tabs.resuming.is_some() || self.opening.is_some() {
            return;
        }
        let had_active = self.document.is_some();
        if !self.suspend_active() {
            return;
        }
        self.tabs.resuming = Some(index);
        self.worker.send(Request::Resume(index));
        if !had_active && self.tabs.suspending.is_none() {
            // Depuis l'accueil, rien n'est en cours de suspension : reprendre tout de suite.
            match self.tabs.tabs[index].doc.take() {
                Some(doc) => self.worker.send(Request::ResumeDoc(doc)),
                None => self.tabs.resuming = None,
            }
        }
    }

    /// Fin de suspension : le moteur renvoie le document ; on le range dans son onglet.
    pub fn on_suspended(&mut self, suspended: SuspendedDoc) {
        if let Some(i) = self.tabs.suspending.take() {
            if let Some(tab) = self.tabs.tabs.get_mut(i) {
                // Le point d'enregistrement du thread moteur fait foi (D052) : il compte une
                // modification encore en vol et efface un « modifié » périmé côté interface.
                tab.dirty = suspended.modified;
                tab.doc = Some(suspended);
            }
        }
        // Reprise différée : le document à reprendre est maintenant disponible.
        if let Some(target) = self.tabs.resuming {
            if let Some(doc) = self.tabs.tabs.get_mut(target).and_then(|t| t.doc.take()) {
                self.worker.send(Request::ResumeDoc(doc));
            }
        }
    }

    /// Edits already queued before Suspend still belong to that tab's UI state.
    pub(crate) fn on_suspending_document_changed(&mut self, state: crate::worker::DocState) {
        if let Some(tab_id) = self
            .tabs
            .suspending
            .and_then(|index| self.tabs.tabs.get(index))
            .map(|tab| tab.recovery_id)
        {
            self.plan_return_source_changed(tab_id);
        }
        let Some(tab) = self
            .tabs
            .suspending
            .and_then(|index| self.tabs.tabs.get_mut(index))
        else {
            return;
        };
        self.has_signatures = state.has_signatures;
        tab.dirty = state.modified;
        let Some(ui) = tab.ui.as_mut() else {
            return;
        };
        ui.reading.hidden_objects.clear();
        let same_sizes = ui
            .document
            .as_ref()
            .is_some_and(|doc| doc.info.page_sizes == state.info.page_sizes);
        if state.measurement_scales_unchanged && same_sizes {
            ui.annotate.measure.advance_epoch(self.view.epoch);
        } else {
            ui.annotate.measure.invalidate(self.view.epoch);
        }
        ui.annotate.register.invalidate();
        ui.annotate.quantities.invalidate();
        ui.docinfo.properties = None;
        ui.docinfo.inspection = None;
        ui.docinfo.inspection_epoch = None;
        ui.find.document_changed();
        ui.organize
            .selected
            .retain(|page| *page < state.info.page_count());
        ui.organize.anchor = ui
            .organize
            .anchor
            .filter(|page| *page < state.info.page_count());
        ui.reader_views
            .document_changed(&state.info, state.pages_unchanged, same_sizes);
        if let Some(doc) = &mut ui.document {
            doc.page = doc.page.min(state.info.page_count().saturating_sub(1));
            ui.page_input = state.info.page_input(doc.page);
            if !same_sizes {
                ui.scroll_to_page = Some(doc.page);
            }
            doc.info = state.info;
        }
        if !state.pages_unchanged || !same_sizes {
            ui.reading.history.reset();
        }
        ui.dirty = state.modified;
        ui.can_undo = state.can_undo;
        ui.can_redo = state.can_redo;
    }

    /// Le moteur a repris un document : restaurer l'état d'interface de son onglet.
    pub fn on_resumed(&mut self, state: crate::worker::DocState, ctx: &egui::Context) {
        let Some(index) = self.tabs.resuming.take() else {
            return;
        };
        let Some(tab_id) = self.tabs.tabs.get(index).map(|tab| tab.recovery_id) else {
            return;
        };
        let return_location = self.plan_return_resume_location(tab_id, &state.info);
        let Some(tab) = self.tabs.tabs.get_mut(index) else {
            return;
        };
        let ui = tab.ui.take();
        self.tabs.active = Some(index);
        match ui {
            Some(mut ui) => {
                let same_sizes = ui
                    .document
                    .as_ref()
                    .is_some_and(|doc| doc.info.page_sizes == state.info.page_sizes);
                ui.reader_views
                    .document_changed(&state.info, state.pages_unchanged, same_sizes);
                if let Some(doc) = &mut ui.document {
                    if doc.info.page_sizes != state.info.page_sizes {
                        ui.annotate.measure.invalidate(self.view.epoch);
                        ui.reading.history.reset();
                        ui.scroll_to_page =
                            Some(doc.page.min(state.info.page_count().saturating_sub(1)));
                    }
                    doc.info = state.info;
                    doc.page = doc.page.min(doc.info.page_count().saturating_sub(1));
                    ui.page_input = doc.info.page_input(doc.page);
                    ui.organize
                        .selected
                        .retain(|page| *page < doc.info.page_count());
                    ui.organize.anchor = ui
                        .organize
                        .anchor
                        .filter(|page| *page < doc.info.page_count());
                }
                if let Some(origin) = &return_location {
                    self.restore_plan_source_mode(origin);
                }
                self.put_tab(ui);
            }
            None => {
                self.page_input = state.info.page_input(0);
                self.document = Some(OpenDocument {
                    info: state.info,
                    page: 0,
                });
                // La zone de défilement est partagée entre onglets : repartir du haut.
                self.view.request(crate::view::ScrollRequest::Page(0));
            }
        }
        self.can_undo = state.can_undo;
        self.can_redo = state.can_redo;
        self.dirty = state.modified;
        self.finish_plan_return_resume(tab_id, return_location);
        self.finish_plan_set_navigation();
        self.update_title(ctx);
        self.worker.send(Request::Bookmarks);
        if self.tabs.quit_pending {
            self.tabs.quit_pending = false;
            if self.has_unsaved_changes() {
                self.request_quit_prompt();
            } else {
                self.execute(ctx, sunipdf_core::Command::Quit);
            }
        }
    }

    /// Keep the tab, its UI and its complete journal available for an explicit retry.
    pub fn on_resume_failed(&mut self, suspended: SuspendedDoc) {
        self.plan_return_transition_failed();
        self.plan_sets.navigation_failed();
        if let Some(i) = self.tabs.resuming.take() {
            if let Some(tab) = self.tabs.tabs.get_mut(i) {
                tab.dirty = suspended.modified;
                tab.doc = Some(suspended);
            }
        }
        self.tabs.quit_pending = false;
    }

    /// Le document actif vient d'être ouvert : il prend place dans un nouvel onglet actif.
    pub fn register_opened_tab(&mut self, path: PathBuf) {
        self.tabs.tabs.push(Tab {
            path,
            dirty: false,
            recovery_id: crate::recovery::next_id(),
            ui: None,
            doc: None,
        });
        self.tabs.active = Some(self.tabs.tabs.len() - 1);
    }

    /// Le document actif a été fermé : retirer son onglet et activer le voisin.
    pub fn on_active_closed(&mut self) {
        if let Some(i) = self.tabs.active.take() {
            if i < self.tabs.tabs.len() {
                self.tabs.tabs.remove(i);
            }
            let next = if self.tabs.tabs.is_empty() {
                None
            } else {
                Some(i.min(self.tabs.tabs.len() - 1))
            };
            if let Some(n) = next {
                self.tabs.resuming = Some(n);
                self.worker.send(Request::Resume(n));
                if let Some(doc) = self.tabs.tabs[n].doc.take() {
                    self.worker.send(Request::ResumeDoc(doc));
                }
            }
        }
    }

    pub fn any_background_dirty(&self) -> Option<usize> {
        self.tabs
            .tabs
            .iter()
            .enumerate()
            .find(|(i, t)| {
                Some(*i) != self.tabs.active
                    && (t.dirty || t.ui.as_ref().is_some_and(|ui| ui.forms.has_unsaved_input()))
            })
            .map(|(i, _)| i)
    }

    pub fn next_tab(&mut self, delta: isize) {
        let n = self.tabs.tabs.len();
        if n == 0 {
            return;
        }
        let current = self.tabs.active.map(|a| a as isize).unwrap_or(-1);
        let target = (current + delta).rem_euclid(n as isize) as usize;
        self.switch_to(target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_page_deletion_updates_suspended_ui_after_resume_or_suspend_failure() {
        for suspend_succeeds in [false, true] {
            let (mut app, _, responses) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            let doc = app.document.as_mut().unwrap();
            doc.info.page_sizes = vec![doc.info.page_sizes[0]; 3];
            doc.page = 2;
            app.page_input = "3".into();
            app.organize.selected.extend([0, 2]);
            app.organize.anchor = Some(2);
            app.view.offset = egui::vec2(0.0, 1500.0);
            app.view
                .reading
                .history
                .jump(0, Some(egui::Vec2::ZERO), 2, 1.0);
            let mut info = app.document.as_ref().unwrap().info.clone();
            info.page_sizes.truncate(1);
            info.page_labels = vec![Some("Cover".into())];
            let state = crate::worker::DocState {
                info: info.clone(),
                can_undo: true,
                can_redo: false,
                pages_unchanged: false,
                measurement_scales_unchanged: false,
                modified: true,
                has_signatures: false,
            };
            assert!(app.suspend_active());
            responses
                .send(crate::worker::Response::DocumentChanged(state.clone()))
                .unwrap();
            app.handle_responses(&ctx);
            let ui = app.tabs.tabs[0].ui.as_ref().unwrap();
            assert_eq!(ui.document.as_ref().unwrap().info.page_count(), 1);
            assert_eq!(ui.document.as_ref().unwrap().page, 0);
            assert_eq!(ui.page_input, "Cover");
            assert_eq!(ui.scroll_to_page, Some(0));
            assert!(!ui.reading.history.can_back());
            assert_eq!(
                ui.organize.selected.iter().copied().collect::<Vec<_>>(),
                vec![0]
            );
            assert_eq!(ui.organize.anchor, None);
            if suspend_succeeds {
                app.on_suspended(SuspendedDoc {
                    bytes: vec![],
                    password: None,
                    path: info.path.clone(),
                    undo: vec![],
                    redo: vec![],
                    save_point: None,
                    modified: true,
                    signature_source: None,
                });
                app.switch_to(0);
                responses
                    .send(crate::worker::Response::Resumed(state))
                    .unwrap();
            } else {
                responses
                    .send(crate::worker::Response::SuspendFailed(
                        sunipdf_core::EngineError::InvalidFormat,
                    ))
                    .unwrap();
            }
            app.handle_responses(&ctx);
            assert_eq!(app.tabs.active, Some(0));
            assert_eq!(app.document.as_ref().unwrap().page, 0);
            assert_eq!(app.document.as_ref().unwrap().info.page_count(), 1);
            assert_eq!(app.page_input, "Cover");
            assert!(app.dirty && app.can_undo);
            assert!(!app.view.reading.history.can_back());
        }
    }

    #[test]
    fn failed_resume_keeps_unsaved_tab_and_ui_for_retry_and_cancels_pending_close() {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        let info = app.document.as_ref().unwrap().info.clone();
        app.dirty = true;
        app.zoom = Zoom::Fixed(1.75);
        app.find.query = "retained query".into();
        let recovery_id = app.tabs.tabs[0].recovery_id;
        assert!(app.suspend_active());
        app.on_suspended(SuspendedDoc {
            bytes: b"unsaved tab bytes".to_vec(),
            password: Some("retained credential".into()),
            path: info.path.clone(),
            undo: vec![],
            redo: vec![],
            save_point: None,
            modified: true,
            signature_source: None,
        });
        app.switch_to(0);
        let suspended = requests
            .try_iter()
            .find_map(|request| match request {
                Request::ResumeDoc(suspended) => Some(suspended),
                _ => None,
            })
            .expect("resume request must own the suspended document");
        app.tabs.quit_pending = true;
        app.execute(&ctx, sunipdf_core::Command::CloseDocument);
        responses
            .send(crate::worker::Response::ResumeFailed {
                error: sunipdf_core::EngineError::InvalidFormat,
                suspended,
            })
            .unwrap();
        app.handle_responses(&ctx);
        assert_eq!(app.tabs.tabs.len(), 1);
        assert_eq!(app.tabs.tabs[0].recovery_id, recovery_id);
        assert_eq!(app.any_background_dirty(), Some(0));
        assert!(app.tabs.resuming.is_none());
        assert!(!app.tabs.quit_pending);
        assert!(
            requests.try_recv().is_err(),
            "failure must cancel the deferred close"
        );
        let retained = app.tabs.tabs[0].doc.as_ref().unwrap();
        assert_eq!(retained.bytes, b"unsaved tab bytes");
        assert_eq!(retained.password.as_deref(), Some("retained credential"));
        assert_eq!(retained.save_point, None);
        app.switch_to(0);
        assert!(requests
            .try_iter()
            .any(|request| matches!(request, Request::ResumeDoc(doc) if doc.bytes == b"unsaved tab bytes")));
        responses
            .send(crate::worker::Response::Resumed(crate::worker::DocState {
                info,
                can_undo: true,
                can_redo: true,
                pages_unchanged: false,
                measurement_scales_unchanged: false,
                modified: true,
                has_signatures: false,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        assert_eq!(app.tabs.active, Some(0));
        assert!(app.dirty && app.can_undo && app.can_redo);
        assert!(matches!(app.zoom, Zoom::Fixed(value) if value == 1.75));
        assert_eq!(app.find.query, "retained query");
        assert!(app.document.is_some());
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, Request::Close | Request::Save { .. })));
    }

    #[test]
    fn view_rotation_and_history_survive_tab_suspension_without_leaking() {
        let (mut app, _, _) = crate::app::tests::test_app();
        app.view.reading.rotation = crate::reading::ViewRotation::Counterclockwise;
        app.view
            .reading
            .history
            .jump(0, Some(egui::vec2(0.0, 42.0)), 4, 1.0);
        let first = app.take_tab();
        assert_eq!(
            app.view.reading.rotation,
            crate::reading::ViewRotation::Upright
        );
        assert!(!app.view.reading.history.can_back());
        app.view.reading.rotation = crate::reading::ViewRotation::Clockwise;
        app.put_tab(first);
        assert_eq!(
            app.view.reading.rotation,
            crate::reading::ViewRotation::Counterclockwise
        );
        assert!(app.view.reading.history.can_back());
    }

    #[test]
    fn shared_view_mode_changes_do_not_restore_old_continuous_offsets() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 3];
        app.document.as_mut().unwrap().page = 1;
        app.zoom = Zoom::Fixed(1.0);
        app.view.offset = egui::vec2(0.0, 900.0);
        app.view
            .reading
            .history
            .jump(0, Some(egui::Vec2::ZERO), 1, 1.0);
        app.view.reading.history.observe(1, app.view.offset, 1.0);
        let previous = app.take_tab();
        app.view.mode = crate::view::ViewMode::Single;
        app.put_tab(previous);
        assert_eq!(app.view.mode, crate::view::ViewMode::Single);
        assert!(app.view.reading.history.can_back());
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx().clone();
                app.document_view(&ctx, ui);
            },
        );
        out.textures_delta.clear();
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(
            app.view.offset.y < 32.0,
            "old continuous offset must not scroll to the bottom of the single page"
        );
    }

    #[test]
    fn recovery_ids_survive_tab_reordering_and_removal() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let first = app.tabs.tabs[0].recovery_id;
        app.register_opened_tab(PathBuf::from("second.pdf"));
        let second = app.tabs.tabs[1].recovery_id;
        app.register_opened_tab(PathBuf::from("third.pdf"));
        let third = app.tabs.tabs[2].recovery_id;
        assert!(first != second && second != third && first != third);
        app.tabs.tabs.swap(0, 2);
        app.tabs.tabs.remove(1);
        assert_eq!(app.tabs.tabs[0].recovery_id, third);
        assert_eq!(app.tabs.tabs[1].recovery_id, first);
    }

    #[test]
    fn invalid_form_draft_prevents_tab_switch_without_losing_input() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.register_opened_tab(PathBuf::from("second.pdf"));
        app.tabs.active = Some(0);
        crate::app::tests::form_draft(&mut app, "too-long-value");
        app.switch_to(1);
        assert_eq!(app.tabs.active, Some(0));
        assert!(app.tabs.resuming.is_none() && app.tabs.suspending.is_none());
        assert!(app.document.is_some());
        assert_eq!(app.forms.drafts["Name"], "too-long-value");
        assert_eq!(app.error, Some("error.form_value_invalid"));
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn valid_form_draft_reaches_engine_before_suspension() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let id = app.tabs.tabs[0].recovery_id;
        crate::app::tests::form_draft(&mut app, "New");
        assert!(app.suspend_active());
        let requests: Vec<_> = requests.try_iter().collect();
        let edit = requests.iter().position(|r| matches!(r, Request::Edit(crate::worker::EditOp::FillForm { values }) if values[0].1 == sunipdf_core::FieldValue::Text("New".into()))).unwrap();
        let suspend = requests
            .iter()
            .position(|r| matches!(r, Request::Suspend))
            .unwrap();
        assert!(edit < suspend);
        assert_eq!(app.tabs.tabs[0].recovery_id, id);
        assert!(app.document.is_none());
        assert_eq!(app.tabs.suspending, Some(0));
    }
}
