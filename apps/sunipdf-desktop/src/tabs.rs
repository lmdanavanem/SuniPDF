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
        let bookmarks_open = self.docinfo.bookmarks_open;
        let comments_open = self.annotate.comments_open;
        let forms_open = self.forms.panel_open;
        let scroll = self.view.offset;
        // Textures libérées (GPU des onglets inactifs), époque suivante : tuiles en vol ignorées.
        self.view.reset();
        self.thumbs_pending.clear();
        self.thumbs_failed.clear();
        // La sélection d'objet désigne un index du document actif : elle ne doit jamais
        // survivre à un changement de document (Suppr agirait sur l'autre onglet).
        self.edit.reset_for_tab();
        // Le placement d'une signature vise une page du document actif : il ne survit pas.
        self.cancel_placing();
        let mut state = TabState {
            document: self.document.take(),
            zoom: std::mem::replace(&mut self.zoom, Zoom::FitWidth),
            effective_zoom: std::mem::replace(&mut self.effective_zoom, 1.0),
            scroll,
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
        self.edit.reset_for_tab();
        let bookmarks_open = self.docinfo.bookmarks_open;
        let comments_open = self.annotate.comments_open;
        let forms_open = self.forms.panel_open;
        self.document = t.document;
        self.zoom = t.zoom;
        self.effective_zoom = t.effective_zoom;
        self.view.reset();
        self.view
            .request(crate::view::ScrollRequest::Offset(t.scroll));
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
    pub fn suspend_active(&mut self) {
        if self.navigation_blocked() {
            return;
        }
        let Some(doc) = &self.document else { return };
        let path = doc.info.path.clone();
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
                    ui: Some(state),
                    doc: None,
                });
                self.tabs.tabs.len() - 1
            }
        };
        self.tabs.active = None;
        self.tabs.suspending = Some(index);
        self.worker.send(Request::Suspend);
    }

    /// Active l'onglet `index` (le document actif, s'il existe, est suspendu).
    pub fn switch_to(&mut self, index: usize) {
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
        self.suspend_active();
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

    /// Le moteur a repris un document : restaurer l'état d'interface de son onglet.
    pub fn on_resumed(&mut self, state: crate::worker::DocState, ctx: &egui::Context) {
        let Some(index) = self.tabs.resuming.take() else {
            return;
        };
        let Some(tab) = self.tabs.tabs.get_mut(index) else {
            return;
        };
        let ui = tab.ui.take();
        self.tabs.active = Some(index);
        match ui {
            Some(mut ui) => {
                if let Some(doc) = &mut ui.document {
                    doc.info = state.info;
                }
                self.put_tab(ui);
            }
            None => {
                self.document = Some(OpenDocument {
                    info: state.info,
                    page: 0,
                });
                self.page_input = "1".into();
                // La zone de défilement est partagée entre onglets : repartir du haut.
                self.view.request(crate::view::ScrollRequest::Page(0));
            }
        }
        self.can_undo = state.can_undo;
        self.can_redo = state.can_redo;
        self.dirty = state.modified;
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

    /// La reprise a échoué : l'onglet visé est retiré (son document n'est plus disponible).
    pub fn on_resume_failed(&mut self) {
        if let Some(i) = self.tabs.resuming.take() {
            if i < self.tabs.tabs.len() {
                self.tabs.tabs.remove(i);
            }
        }
    }

    /// Le document actif vient d'être ouvert : il prend place dans un nouvel onglet actif.
    pub fn register_opened_tab(&mut self, path: PathBuf) {
        self.tabs.tabs.push(Tab {
            path,
            dirty: false,
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
