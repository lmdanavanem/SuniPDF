//! État et interface de l'application. Aucune logique PDF ici : tout passe par `worker`.

use crate::dialog;
use crate::icons::glyph;
use crate::settings::Settings;
use crate::theme;
use crate::updater::{Phase as UpdatePhase, Updater};
use crate::view::ViewMode;
use crate::worker::{BytesOp, DocState, EditOp, Request, Response, Worker};
use crate::workspace::Esc;
use egui::{Key, Modifiers};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use sunipdf_core::{Command, DocumentInfo, EngineError, Menu};
use sunipdf_i18n::{tr, tr_args, Language};

/// Points CSS par point PDF : à 100 %, une page de 612 pt occupe 816 px logiques (96 dpi).
pub const SCREEN_PER_PDF_POINT: f32 = 96.0 / 72.0;
pub const ZOOM_STEPS: [f32; 15] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0,
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zoom {
    FitWidth,
    FitPage,
    Fixed(f32),
}

pub enum EngineState {
    Starting,
    Ready,
    Failed,
}

pub struct OpenDocument {
    pub info: DocumentInfo,
    pub page: usize,
}

struct PasswordPrompt {
    path: PathBuf,
    input: String,
    wrong: bool,
}

#[derive(Clone)]
enum PendingAction {
    Close,
    Quit,
}

enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

impl FindState {
    /// Oublie une recherche en vol : elle sera relancée au prochain affichage de la barre.
    pub fn reset_in_flight(&mut self) {
        self.in_flight = false;
        self.searched = None;
    }
}

#[derive(Default)]
pub struct FindState {
    pub open: bool,
    pub(crate) query: String,
    match_case: bool,
    /// Requête dont les résultats sont affichés (évite de relancer à chaque frame).
    searched: Option<(String, bool)>,
    generation: u64,
    in_flight: bool,
    matches: Vec<sunipdf_core::TextMatch>,
    current: usize,
    focus_requested: bool,
}

pub struct PaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub focus_requested: bool,
}

#[cfg(test)]
use crate::shortcut_defaults::DEFAULTS as SHORTCUTS;

pub struct SuniApp {
    pub review: crate::local_review::ReviewState,
    pub overlay: crate::revision_overlay::OverlayState,
    pub processing: crate::processing::ProcessingState,
    pub office: crate::office::OfficeState,
    pub reading_full_screen: bool,
    pub recovery: crate::recovery::RecoveryState,
    pub printing: crate::printing::PrintState,
    pub ocr: crate::ocr::OcrState,
    pub form_builder: crate::form_builder::FormBuilderState,
    pub certificate: crate::certificate::CertificateState,
    has_signatures: bool,
    signed_save_confirm: Option<bool>,
    pub settings: Settings,
    pub shortcuts: crate::shortcuts::State,
    pub worker: Worker,
    pub engine: EngineState,
    pub document: Option<OpenDocument>,
    pub opening: Option<PathBuf>,
    pub zoom: Zoom,
    pub effective_zoom: f32,
    /// Vue du document : mode, défilement, tuiles et textures (`view.rs`).
    pub view: crate::view::ViewState,
    pub page_input: String,
    pub error: Option<&'static str>,
    password_prompt: Option<PasswordPrompt>,
    show_about: bool,
    /// Fichiers à ouvrir dès que le moteur est prêt (arguments de ligne de commande), un onglet chacun.
    pending_files: Vec<PathBuf>,
    pub thumbnails: HashMap<usize, egui::TextureHandle>,
    /// Requests remain registered across viewport and width changes until acknowledged.
    pub thumbs_pending: HashSet<usize>,
    pub thumbs_failed: HashSet<usize>,
    thumb_window: std::ops::Range<usize>,
    pub thumb_px: u32,
    pub find: FindState,
    pub dirty: bool,
    pub saving: bool,
    pending_save: Option<PathBuf>,
    saving_drafts: HashMap<String, String>,
    allow_close: bool,
    unsaved_prompt: Option<PendingAction>,
    after_save: Option<PendingAction>,
    deferred_close: Option<PendingAction>,
    install_on_quit: bool,
    updater: Updater,
    update_errors_visible: bool,
    install_confirm: bool,
    pub status_msg: Option<String>,
    pub has_icons: bool,
    /// Textures de marque (symbole, logos clair et sombre), chargées au premier usage.
    pub brand: Option<crate::brand::Brand>,
    /// Commande demandée sans document : exécutée dès qu'un fichier est ouvert.
    pub after_open: Option<Command>,
    pub show_tools: bool,
    /// Un widget avait le focus clavier à l'image précédente (egui le retire sur Échap avant que
    /// `handle_input` ne lise la touche) : Échap ne quitte alors pas l'outil actif.
    pub focus_last_frame: bool,
    pub palette: PaletteState,
    pub organize: crate::organize::OrganizeState,
    pub annotate: crate::annotate::AnnotateState,
    pub docinfo: crate::docinfo::DocInfoState,
    pub tools: crate::tools::ToolsState,
    pub tabs: crate::tabs::TabsState,
    pub edit: crate::editor::EditState,
    pub pagesetup: crate::pagesetup::PageSetupState,
    /// Signature visuelle : boîte, bibliothèque locale (globale, hors onglet) et placement.
    pub sign: crate::sign::SignState,
    pub library: crate::annotation_library::LibraryState,
    /// Formulaire AcroForm du document actif : champs lus, panneau, saisie en place.
    pub forms: crate::forms::FormsState,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Commandes (identifiants du registre) exécutées après la première ouverture : automatisation et captures.
    startup_commands: Vec<Command>,
}

impl SuniApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_files: Vec<PathBuf>) -> Self {
        let settings = Settings::load();
        let has_icons = crate::icons::install(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.dark_theme);
        // Ctrl+Plus / Moins / 0 zooment la page, jamais l'interface (egui les prendrait dès que
        // `handle_input` laisse passer les touches : champ focalisé, boîte de dialogue, palette).
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let recovery = crate::recovery::RecoveryState::load();
        let recovery_failed = recovery.failed;
        let mut app = Self {
            processing: Default::default(),
            office: Default::default(),
            review: Default::default(),
            overlay: Default::default(),
            reading_full_screen: false,
            recovery,
            printing: Default::default(),
            ocr: Default::default(),
            form_builder: Default::default(),
            certificate: Default::default(),
            has_signatures: false,
            signed_save_confirm: None,
            has_icons,
            brand: None,
            after_open: None,
            show_tools: settings.show_tools,
            focus_last_frame: false,
            palette: PaletteState {
                open: false,
                query: String::new(),
                selected: 0,
                focus_requested: false,
            },
            organize: Default::default(),
            annotate: Default::default(),
            docinfo: Default::default(),
            tools: crate::tools::ToolsState {
                split_every: 1,
                ..Default::default()
            },
            tabs: Default::default(),
            edit: Default::default(),
            pagesetup: Default::default(),
            sign: Default::default(),
            library: Default::default(),
            forms: Default::default(),
            can_undo: false,
            can_redo: false,
            startup_commands: std::env::var("SUNIPDF_STARTUP_COMMANDS")
                .ok()
                .map(|v| {
                    v.split(',')
                        .filter_map(|id| Command::from_id(id.trim()))
                        .collect()
                })
                .unwrap_or_default(),
            worker: Worker::spawn(cc.egui_ctx.clone()),
            view: crate::view::ViewState::new(settings.continuous_view),
            settings,
            engine: EngineState::Starting,
            document: None,
            opening: None,
            zoom: Zoom::FitWidth,
            effective_zoom: 1.0,
            page_input: String::new(),
            error: recovery_failed.then_some("error.recovery_failed"),
            password_prompt: None,
            show_about: false,
            shortcuts: Default::default(),
            pending_files: initial_files,
            thumbnails: HashMap::new(),
            thumbs_pending: HashSet::new(),
            thumbs_failed: HashSet::new(),
            thumb_window: 0..0,
            thumb_px: 0,
            find: FindState {
                open: false,
                query: String::new(),
                match_case: false,
                searched: None,
                generation: 0,
                in_flight: false,
                matches: Vec::new(),
                current: 0,
                focus_requested: false,
            },
            dirty: false,
            saving: false,
            pending_save: None,
            saving_drafts: HashMap::new(),
            allow_close: false,
            unsaved_prompt: None,
            after_save: None,
            deferred_close: None,
            install_on_quit: false,
            updater: Updater::default(),
            update_errors_visible: false,
            install_confirm: false,
            status_msg: None,
        };
        app.view.mode = match app.settings.view_mode.as_deref() {
            Some("single") => ViewMode::Single,
            Some("two_page") => ViewMode::TwoPage,
            Some("book") => ViewMode::Book,
            _ => app.view.mode,
        };
        app.start_update_check(&cc.egui_ctx, false);
        app
    }

    pub fn lang(&self) -> Language {
        self.settings.language()
    }

    pub fn t(&self, key: &str) -> &'static str {
        tr(self.lang(), key)
    }

    // ----- Actions ---------------------------------------------------------

    fn open_file_dialog(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter(self.t("dialog.open.filter_pdf"), &["pdf"]);
        if let Some(path) = dialog.pick_file() {
            self.open_path(path, None);
        } else {
            self.after_open = None;
        }
    }

    pub fn open_path(&mut self, path: PathBuf, password: Option<String>) {
        if self.ink_blocks_action() {
            return;
        }
        if !matches!(self.engine, EngineState::Ready) {
            self.pending_files.push(path);
            return;
        }
        if self.navigation_blocked() || self.tabs.resuming.is_some() || self.opening.is_some() {
            // Une ouverture ou un changement d'onglet est déjà en cours.
            return;
        }
        self.error = self.recovery.failed.then_some("error.recovery_failed");
        if self.document.is_some() && !self.suspend_active() {
            return;
        }
        self.opening = Some(path.clone());
        self.worker.send(Request::Open { path, password });
    }

    fn close_document(&mut self, announce: bool) {
        if let Some(slot) = self.recovery_slot() {
            if let Err(e) = slot.remove() {
                log::warn!("Recovery cleanup failed: {e}");
            }
        }
        if let Some(id) = self
            .tabs
            .active
            .and_then(|i| self.tabs.tabs.get(i))
            .map(|t| t.recovery_id)
        {
            if let Err(e) = self.recovery.finish(id) {
                log::warn!("Recovered file cleanup failed: {e}");
            }
        }
        self.has_signatures = false;
        self.certificate.visible = false;
        self.form_builder.cancel();
        if self.document.take().is_some() {
            self.worker.send(Request::Close);
        }
        self.view.reset();
        self.page_input.clear();
        self.thumbnails.clear();
        self.thumbs_pending.clear();
        self.thumbs_failed.clear();
        self.close_find();
        self.organize = Default::default();
        self.annotate = Default::default();
        // Le presse-papiers d'objets survit à la fermeture (coller dans un autre onglet).
        self.edit.reset_for_tab();
        self.pagesetup = Default::default();
        // Le cache des signatures enregistrées est global : seuls la boîte et le placement partent.
        self.cancel_placing();
        self.sign.open = None;
        let bookmarks_open = self.docinfo.bookmarks_open;
        self.docinfo = Default::default();
        self.docinfo.bookmarks_open = bookmarks_open;
        self.forms.reset_for_document();
        self.can_undo = false;
        self.can_redo = false;
        self.dirty = false;
        self.saving = false;
        self.pending_save = None;
        self.saving_drafts.clear();
        self.status_msg = None;
        if announce {
            self.error = None;
        }
        self.on_active_closed();
    }

    pub fn go_to_page(&mut self, page: usize) {
        if self.annotate.measure.is_active_draft() {
            self.status_msg = Some(self.t("measure.finish_first").into());
            return;
        }
        if self.edit.paragraph.is_some() {
            self.status_msg = Some(self.t("paragraph.finish_first").into());
            return;
        }
        if self.annotate.adv.free_text.is_some() {
            self.status_msg = Some(self.t("free_text.finish_first").into());
            return;
        }
        if let Some(doc) = &self.document {
            self.view.reading.history.jump(
                doc.page,
                (!self.view.navigation_pending()).then_some(self.view.offset),
                page.min(doc.info.page_count().saturating_sub(1)),
                self.effective_zoom,
            );
        }
        if let Some(doc) = &mut self.document {
            let clamped = page.min(doc.info.page_count().saturating_sub(1));
            if clamped != doc.page {
                doc.page = clamped;
                self.error = None;
                // Une sélection ou un texte en cours sur une page qui n'est plus affichée : Suppr ou
                // les flèches agiraient à l'aveugle, et l'éditeur invisible garderait le clavier.
                self.edit.selection = None;
                self.annotate.adv.selected = None;
                if self
                    .edit
                    .text_prompt
                    .as_ref()
                    .is_some_and(|p| p.page != clamped)
                {
                    self.edit.text_prompt = None;
                }
            }
            self.page_input = doc.info.page_input(clamped);
            // Toujours défiler, même vers la page courante (on peut être au milieu de la page).
            self.lock_on_page(clamped);
        }
    }

    pub fn submit_page_input(&mut self) {
        let Some(doc) = &self.document else { return };
        if let Some(page) = doc.info.resolve_page_input(&self.page_input) {
            self.status_msg = None;
            self.go_to_page(page);
        } else {
            self.page_input = doc.info.page_input(doc.page);
            self.status_msg = Some(self.t("reading.page_label_invalid").to_owned());
        }
    }

    /// Amène le résultat de recherche `i` à l'écran.
    fn go_to_match(&mut self, i: usize) {
        let Some(m) = self.find.matches.get(i) else {
            return;
        };
        let page = m.page;
        let top = m.rects.iter().map(|r| r.top).fold(f32::MIN, f32::max);
        let left = m.rects.iter().map(|r| r.left).fold(f32::MAX, f32::min);
        let h = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
            .map_or(0.0, |s| s.height_pt);
        self.go_to_page_point(page, left.max(0.0), (h - top).max(0.0));
    }

    fn step_page(&mut self, delta: isize) {
        if let Some(doc) = &self.document {
            let target = doc.page as isize + delta;
            if target >= 0 {
                self.go_to_page(target as usize);
            }
        }
    }

    fn zoom_step(&mut self, direction: isize) {
        // Zoom fixe : part de la valeur demandée (des pas enchaînés dans une même image s'ajoutent).
        let current = match self.zoom {
            Zoom::Fixed(z) => z,
            _ => self.effective_zoom,
        };
        let next = if direction > 0 {
            ZOOM_STEPS.iter().copied().find(|z| *z > current + 0.01)
        } else {
            ZOOM_STEPS
                .iter()
                .rev()
                .copied()
                .find(|z| *z < current - 0.01)
        };
        if let Some(z) = next {
            self.zoom = Zoom::Fixed(z);
        }
    }

    pub fn set_language(&mut self, language: Language) {
        if self.lang() != language {
            self.settings.set_language(language);
            self.persist_settings();
        }
    }

    pub fn set_theme(&mut self, ctx: &egui::Context, dark: bool) {
        if self.settings.dark_theme != dark {
            self.settings.dark_theme = dark;
            theme::apply(ctx, dark);
            self.persist_settings();
        }
    }

    pub fn persist_settings(&self) {
        #[cfg(not(test))]
        if let Err(e) = self.settings.save() {
            log::warn!("préférences non enregistrées : {e}");
        }
    }

    // ----- Messages du moteur ---------------------------------------------

    pub(crate) fn handle_responses(&mut self, ctx: &egui::Context) {
        let responses: Vec<Response> = self.worker.poll().collect();
        for response in responses {
            match response {
                Response::EngineReady => {
                    self.engine = EngineState::Ready;
                    self.open_next_pending();
                }
                Response::LocalReviewProgress {
                    done,
                    total,
                    page,
                    pages,
                } => {
                    self.review.progress = (done, total, page, pages);
                }
                Response::LocalReviewDone(result) => self.on_local_review(ctx, result),
                Response::RevisionOverlayDone { generation, result } => {
                    self.on_revision_overlay(ctx, generation, result);
                }
                Response::EngineFailed(e) => {
                    self.error = Some(error_key(&e));
                    log::error!("moteur indisponible : {e}");
                    self.engine = EngineState::Failed;
                }
                Response::ProcessingProgress {
                    done,
                    total,
                    item_done,
                    item_total,
                } => {
                    self.processing.done = done;
                    self.processing.total = total;
                    self.processing.item_done = item_done;
                    self.processing.item_total = item_total;
                }
                Response::ProcessingDone(results) => {
                    self.processing.busy = false;
                    self.processing.results = results;
                }
                Response::OfficeProgress {
                    done,
                    total,
                    item_done,
                    item_total,
                } => {
                    self.office.done = done;
                    self.office.total = total;
                    self.office.item_done = item_done;
                    self.office.item_total = item_total;
                }
                Response::OfficeDone(results) => {
                    self.office.busy = false;
                    self.office.results = results;
                }
                Response::TaskProgress { kind, done, total } => match kind {
                    crate::worker::TaskKind::Print => {
                        self.printing.done = done;
                        self.printing.total = total;
                    }
                    crate::worker::TaskKind::Ocr => {
                        self.ocr.done = done;
                        self.ocr.total = total;
                    }
                    crate::worker::TaskKind::Signature => {
                        self.sign.done = done;
                        self.sign.total = total;
                    }
                },
                Response::TaskDone { kind, result } => {
                    if kind == crate::worker::TaskKind::Signature && result.is_err() {
                        self.edit.pending_status = None;
                    }
                    match kind {
                        crate::worker::TaskKind::Print => self.printing.busy = false,
                        crate::worker::TaskKind::Ocr => self.ocr.busy = false,
                        crate::worker::TaskKind::Signature => self.sign.busy = false,
                    }
                    match result {
                        Ok(()) => {
                            self.status_msg = Some(
                                self.t(match kind {
                                    crate::worker::TaskKind::Print => "status.printed",
                                    crate::worker::TaskKind::Ocr => "status.ocr_done",
                                    crate::worker::TaskKind::Signature => "signature.done",
                                })
                                .into(),
                            );
                            if kind == crate::worker::TaskKind::Signature
                                && self.edit.pending_status.is_none()
                            {
                                self.edit.pending_status = self.status_msg.clone();
                            }
                        }
                        Err(EngineError::Cancelled) => {
                            self.status_msg = Some(self.t("status.cancelled").into())
                        }
                        Err(error) => {
                            log::error!("Operation failed: {error}");
                            self.error = Some(error_key(&error));
                        }
                    }
                }
                Response::ProtectionAuthorization { epoch, encrypted } => {
                    self.protection_authorized(epoch, encrypted);
                }
                Response::Signatures(result) => {
                    self.certificate.busy = false;
                    match result {
                        Ok(signatures) => self.certificate.results(signatures),
                        Err(e) => {
                            log::warn!("Signature verification failed: {e}");
                            self.certificate.error = Some(self.t(error_key(&e)).into());
                        }
                    }
                }
                Response::SignedCopy(result) => {
                    self.certificate.busy = false;
                    match result {
                        Ok(path) => {
                            self.certificate.visible = false;
                            self.status_msg = Some(self.t("status.signed_copy").into());
                            self.open_path(path, None);
                        }
                        Err(e) => {
                            log::warn!("Certificate signing failed: {e}");
                            self.certificate.error = Some(self.t(error_key(&e)).into());
                        }
                    }
                }
                Response::RecoverySaved(result) => {
                    self.recovery.pending = false;
                    if let Err(e) = result {
                        log::error!("Recovery snapshot failed: {e}");
                        self.error = Some("error.recovery_failed");
                    }
                }
                Response::SuspendFailed(error) => {
                    self.review.pending_page = None;
                    self.opening = None;
                    self.tabs.resuming = None;
                    if let Some(index) = self.tabs.suspending.take() {
                        if let Some(state) =
                            self.tabs.tabs.get_mut(index).and_then(|tab| tab.ui.take())
                        {
                            self.put_tab(state);
                            self.tabs.active = Some(index);
                        }
                    }
                    self.worker.send(Request::AcknowledgeSuspendFailure);
                    log::error!("Document suspension failed: {error}");
                    self.error = Some(error_key(&error));
                }
                Response::Opened(DocState {
                    mut info,
                    has_signatures,
                    ..
                }) => {
                    self.form_builder.cancel();
                    self.has_signatures = has_signatures;
                    let restored = self
                        .recovery
                        .pending_restore
                        .as_ref()
                        .filter(|entry| entry.snapshot == info.path)
                        .cloned();
                    if let Some(entry) = &restored {
                        info.path = entry.original.clone().unwrap_or_else(|| {
                            std::env::var_os("USERPROFILE")
                                .map(std::path::PathBuf::from)
                                .unwrap_or_default()
                                .join("Documents/Recovered.pdf")
                        });
                        self.recovery.pending_restore = None;
                    }
                    self.opening = None;
                    self.password_prompt = None;
                    self.page_input = info.page_input(0);
                    self.settings.push_recent(&info.path);
                    self.persist_settings();
                    self.can_undo = false;
                    self.can_redo = false;
                    let opened_path = info.path.clone();
                    // Rien du document précédent ne doit apparaître dans le nouveau.
                    self.thumbnails.clear();
                    self.thumbs_pending.clear();
                    self.thumbs_failed.clear();
                    self.edit.reset_for_tab();
                    self.pagesetup.close_labels();
                    self.annotate.adv = Default::default();
                    self.annotate.clear_reading();
                    self.annotate.annotations_page = None;
                    self.view.reset();
                    self.forms.reset_for_document();
                    // Première page en haut, centrée si le document mêle des tailles de page.
                    self.view.request(crate::view::ScrollRequest::Page(0));
                    self.document = Some(OpenDocument { info, page: 0 });
                    self.register_opened_tab(opened_path);
                    if let Some((path, page)) = self.review.pending_page.take() {
                        if self
                            .document
                            .as_ref()
                            .is_some_and(|doc| doc.info.path == path)
                        {
                            self.go_to_page(page.saturating_sub(1));
                        }
                    }
                    self.dirty = restored.is_some();
                    self.worker.send(Request::SetRecovery(self.recovery_slot()));
                    self.status_msg = None;
                    if let Some(entry) = restored {
                        if let Some(tab) = self.tabs.active.and_then(|i| self.tabs.tabs.get(i)) {
                            let id = tab.recovery_id;
                            self.worker.send(Request::MarkRecovered {
                                path: self.document.as_ref().unwrap().info.path.clone(),
                            });
                            self.recovery_restored(id, entry);
                        }
                    }
                    if let Some(cmd) = self.after_open.take() {
                        self.execute(ctx, cmd);
                    }
                    self.update_title(ctx);
                    self.worker.send(Request::Bookmarks);
                    self.worker.send(Request::GetRedactionMarks {
                        epoch: self.view.epoch,
                    });
                    // Les commandes scriptées attendent la première image du document (fin de `ui`).
                    self.open_next_pending();
                }
                Response::OpenFailed { path, error } => {
                    if !matches!(error, EngineError::PasswordRequired) {
                        self.review.pending_page = None;
                    }
                    self.opening = None;
                    if let Some(i) = self.tabs.resuming.take() {
                        if i < self.tabs.tabs.len() {
                            self.tabs.tabs.remove(i);
                        }
                    }
                    if matches!(error, EngineError::PasswordRequired) {
                        let wrong = self
                            .password_prompt
                            .as_ref()
                            .is_some_and(|p| p.path == path);
                        self.password_prompt = Some(PasswordPrompt {
                            path,
                            input: String::new(),
                            wrong,
                        });
                    } else {
                        log::warn!("ouverture impossible : {error}");
                        self.error = Some(error_key(&error));
                    }
                }
                Response::Tile { epoch, job, image } => self.on_tile(ctx, epoch, job, image),
                Response::TileFailed { epoch, job, error } => {
                    self.on_tile_failed(epoch, job, error)
                }
                Response::Thumbnail { page, image } => {
                    if self.thumbs_pending.remove(&page)
                        && image.width == self.thumb_px
                        && self.thumb_window.contains(&page)
                        && self.document.is_some()
                    {
                        let color = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.rgba,
                        );
                        let handle = ctx.load_texture(
                            format!("sunipdf-thumb-{page}"),
                            color,
                            egui::TextureOptions::LINEAR,
                        );
                        self.thumbnails.insert(page, handle);
                    }
                }
                Response::ThumbnailFailed { page, width_px } => {
                    if self.thumbs_pending.remove(&page)
                        && width_px == self.thumb_px
                        && self.thumb_window.contains(&page)
                        && self.document.is_some()
                    {
                        self.thumbs_failed.insert(page);
                    }
                }
                Response::DocumentChanged(DocState {
                    info,
                    can_undo,
                    can_redo,
                    pages_unchanged,
                    measurement_scales_unchanged,
                    modified,
                    has_signatures,
                }) => {
                    self.measurement_scale_changed();
                    self.annotate.register.invalidate();
                    self.annotate.quantities.invalidate();
                    self.docinfo.inspection_epoch = None;
                    self.docinfo.inspection = None;
                    if self.edit.paragraph.as_ref().is_some_and(|d| d.applying) {
                        self.edit.paragraph = None;
                    }
                    self.form_builder.cancel();
                    self.annotate.adv.free_text = None;
                    if self
                        .annotate
                        .adv
                        .ink_draft
                        .as_ref()
                        .is_some_and(|draft| draft.pending)
                    {
                        self.annotate.adv.ink_draft = None;
                    }
                    self.has_signatures = has_signatures;
                    self.annotate.clear_reading();
                    if !pages_unchanged {
                        self.pagesetup.close_labels();
                    }
                    if !pages_unchanged
                        && (self.sign.open.is_some()
                            || matches!(
                                self.sign.placing,
                                Some(crate::sign::Placing::Image { pages: Some(_), .. })
                            ))
                    {
                        self.cancel_placing();
                    }
                    if self.document.is_none() {
                        continue;
                    }
                    self.can_undo = can_undo;
                    self.can_redo = can_redo;
                    let count = info.page_count();
                    self.organize.selected.retain(|p| *p < count);
                    if let Some(doc) = &mut self.document {
                        let same_sizes = doc.info.page_sizes == info.page_sizes;
                        doc.info = info;
                        doc.page = doc.page.min(doc.info.page_count().saturating_sub(1));
                        self.page_input = doc.info.page_input(doc.page);
                        // Retouche : l'image reste affichée jusqu'aux nouvelles tuiles.
                        self.view
                            .on_document_changed(pages_unchanged, same_sizes, doc.page);
                        if measurement_scales_unchanged && same_sizes {
                            self.annotate.measure.advance_epoch(self.view.epoch);
                        } else {
                            self.annotate.measure.invalidate(self.view.epoch);
                        }
                    }
                    self.thumbnails.clear();
                    self.thumbs_pending.clear();
                    self.thumbs_failed.clear();
                    self.find.searched = None;
                    self.find.matches.clear();
                    self.annotate.selection = None;
                    self.annotate.annotations_page = None;
                    // Indices d'annotation et zones de lien : relus ; la sélection est reprise par
                    // `on_annotations` si une modification l'attend (`adv.reselect`).
                    self.annotate.adv.selected = None;
                    self.annotate.adv.drag = None;
                    self.annotate.adv.ink.clear();
                    self.annotate.adv.links_page = None;
                    self.annotate.adv.link = None;
                    // La liste d'objets en cache (panneau, prise directe) est périmée.
                    self.edit.objects_page = None;
                    // Les champs du formulaire aussi (valeur saisie, annulation, aplatissement).
                    self.forms.stale = true;
                    match (self.edit.reselect_many.take(), self.edit.reselect.take()) {
                        (Some((page, pick)), _) => self.worker.send(Request::Pick {
                            page,
                            pick,
                            mode: crate::worker::PickMode::Replace,
                        }),
                        (None, Some((page, index))) => {
                            self.worker.send(Request::ObjectInfo { page, index })
                        }
                        (None, None) => self.edit.selection = None,
                    }
                    self.worker.send(Request::Bookmarks);
                    if self.docinfo.properties_open {
                        self.worker.send(Request::Properties);
                    } else {
                        // Chiffrement, permissions et métadonnées ont pu changer (Protéger…) :
                        // le cache ne doit pas bloquer « Retirer la protection ».
                        self.docinfo.properties = None;
                    }
                    // Annuler jusqu'à l'état enregistré rend le document « intact » (point
                    // d'enregistrement tenu par le thread moteur).
                    self.dirty = modified;
                    self.status_msg = self.edit.pending_status.take();
                    // Le message de l'opération reste affiché après la resélection.
                    self.edit.hold_status = self.status_msg.is_some();
                    self.redact_on_document_changed();
                    self.worker.send(Request::GetRedactionMarks {
                        epoch: self.view.epoch,
                    });
                    self.update_title(ctx);
                }
                Response::Redacted(report) => self.on_redacted(report),
                Response::RedactionPreview {
                    epoch,
                    generation,
                    result,
                } => {
                    self.on_redaction_preview(ctx, epoch, generation, result);
                }
                Response::RedactionMarks { epoch, result } => {
                    self.on_persisted_redaction_marks(epoch, result);
                }
                Response::RedactMatches {
                    query,
                    verify,
                    marks,
                } => self.on_redact_matches(query, verify, marks),
                // L'intention (surligner, souligner, barrer ou simple sélection) est celle du
                // glissement, pas celle de l'outil au moment où la réponse arrive.
                Response::ReadingSelected {
                    epoch,
                    generation,
                    result,
                } => {
                    self.on_reading_selected(epoch, generation, result);
                }
                Response::TextSelected { page, selection } => {
                    match self.annotate.pending_markup.take() {
                        Some((kind, rgb)) if !selection.rects.is_empty() => {
                            self.worker.send(Request::Edit(EditOp::Markup {
                                page,
                                kind,
                                rects: selection.rects,
                                rgb,
                            }));
                            self.annotate.selection = None;
                        }
                        Some(_) => self.annotate.selection = None,
                        None if self.annotate.tool != crate::annotate::Tool::Select => {
                            self.annotate.selection = Some((page, selection))
                        }
                        None => {}
                    }
                }
                Response::QuantityTable {
                    epoch,
                    generation,
                    result,
                } => self.on_quantity_table(epoch, generation, result),
                Response::QuantityExported {
                    epoch,
                    generation,
                    result,
                } => self.on_quantity_exported(epoch, generation, result),
                Response::AnnotationRegister {
                    epoch,
                    generation,
                    result,
                } => self.on_annotation_register(epoch, generation, result),
                Response::MeasurementPage {
                    page,
                    epoch,
                    generation,
                    result,
                } => {
                    self.on_measurement_page(page, epoch, generation, result);
                }
                Response::Annotations {
                    page,
                    list,
                    details,
                } => self.on_annotations(page, list, details),
                Response::Links { page, list } => self.on_links(page, list),
                Response::FormInfo(result) => match result {
                    Ok(info) => self.on_form_info(info),
                    Err(e) => self.on_form_read_failed(e),
                },
                Response::BatchDone(result) => self.on_batch_done(result),
                Response::ObjectSelected { page, info } => self.on_object_selected(page, info),
                Response::ObjectsSelected { page, infos, mode } => {
                    self.on_objects_selected(page, infos, mode)
                }
                Response::RotationCenter {
                    page,
                    indices,
                    epoch,
                    generation,
                    center,
                } => {
                    self.on_rotation_center(page, &indices, epoch, generation, center);
                }
                Response::ParagraphPreview {
                    epoch,
                    generation,
                    result,
                } => {
                    self.on_paragraph_preview(ctx, epoch, generation, result);
                }
                Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result,
                } => {
                    self.on_rich_preview(ctx, epoch, generation, result);
                }
                Response::Objects { page, list } => {
                    if self.edit.objects_page == Some(page) {
                        self.edit.objects = list;
                    }
                }
                Response::Copied { count, text } => self.on_copied(ctx, count, text),
                Response::SystemFonts(list) => {
                    log::debug!("{} polices système incorporables", list.len());
                    self.edit.system_fonts = Some(list);
                }
                Response::ImageLoaded { path, result } => self.on_image_loaded(ctx, path, result),
                Response::SignatureRendered { id, result } => {
                    self.on_signature_rendered(ctx, id, result)
                }
                Response::Suspended(s) => self.on_suspended(s),
                Response::Resumed(state) => {
                    self.has_signatures = state.has_signatures;
                    self.on_resumed(state, ctx);
                    self.worker.send(Request::SetRecovery(self.recovery_slot()));
                    self.worker.send(Request::GetRedactionMarks {
                        epoch: self.view.epoch,
                    });
                }
                Response::ResumeFailed(error) => {
                    log::error!("reprise d'onglet impossible : {error}");
                    self.on_resume_failed();
                    self.error = Some(error_key(&error));
                }
                Response::Properties(props) => {
                    self.docinfo.metadata_edit = props.metadata.clone();
                    self.docinfo.properties = Some(props);
                }
                Response::Inspection { epoch, result } => self.inspection_received(epoch, result),
                Response::Bookmarks(list) => {
                    self.docinfo.bookmarks = list;
                }
                Response::OperationFailed(e) => {
                    self.measurement_scale_failed(&e);
                    self.register_edit_failed(&e);
                    self.quantity_edit_failed(&e);
                    self.rich_annotation_failed(&e);
                    if let Some(draft) = self.edit.paragraph.as_mut().filter(|d| d.applying) {
                        draft.applying = false;
                        draft.preview = None;
                        draft.error = Some(e.to_string());
                    }
                    if let Some(prompt) = &mut self.annotate.adv.free_text {
                        if prompt.pending {
                            prompt.pending = false;
                            self.status_msg = Some(self.t("free_text.retry").into());
                        }
                    }
                    if let Some(draft) = &mut self.annotate.adv.ink_draft {
                        if draft.pending {
                            draft.pending = false;
                            self.status_msg = Some(self.t("ink.retry").into());
                        }
                    }
                    self.pending_save = None;
                    self.saving = false;
                    self.cancel_after_save();
                    log::warn!("opération impossible : {e}");
                    self.error = Some(error_key(&e));
                    self.redact_failed();
                    self.edit.pending_status = None;
                    self.edit.reselect = None;
                    self.edit.reselect_many = None;
                    // Copie échouée : le « couper » qui l'attendait ne supprime rien.
                    self.edit.pending_copy = None;
                    // Valeur de champ refusée par le moteur : la valeur locale optimiste est
                    // fausse, le modèle est relu.
                    self.forms.info = None;
                    self.forms.stale = true;
                    if matches!(
                        e,
                        EngineError::MissingGlyphs(_) | EngineError::FontNotEmbeddable
                    ) {
                        // Retain the draft so unsupported text can be corrected without retyping.
                        if let Some(mut p) = self.edit.last_prompt.take() {
                            p.focus = true;
                            p.armed = false;
                            self.edit.text_prompt = Some(p);
                        }
                    }
                }
                Response::Saved { path } => {
                    self.saving = false;
                    self.dirty = false;
                    self.forms
                        .drafts
                        .retain(|name, text| self.saving_drafts.get(name) != Some(text));
                    self.saving_drafts.clear();
                    self.status_msg = Some(tr_args(
                        self.lang(),
                        "status.saved",
                        &[("name", &file_name(&path))],
                    ));
                    if let Some(doc) = &mut self.document {
                        doc.info.path = path.clone();
                    }
                    if let Some(tab) = self.tabs.active.and_then(|i| self.tabs.tabs.get_mut(i)) {
                        tab.path = path;
                        if let Err(e) = self.recovery.finish(tab.recovery_id) {
                            log::warn!("Recovered file cleanup failed: {e}");
                        }
                    }
                    self.worker.send(Request::SetRecovery(self.recovery_slot()));
                    self.update_title(ctx);
                    if let Some(action) = self.after_save.take() {
                        self.deferred_close = Some(action);
                    }
                }
                Response::SaveFailed(e) => {
                    self.saving = false;
                    self.saving_drafts.clear();
                    self.cancel_after_save();
                    log::error!("enregistrement impossible : {e}");
                    self.error = Some("error.save_failed");
                }
                Response::SearchResults {
                    generation,
                    matches,
                } => {
                    if generation == self.find.generation {
                        self.find.in_flight = false;
                        self.find.matches = matches;
                        self.find.current = 0;
                        self.go_to_match(0);
                    }
                }
                Response::MutationFinished => self.worker.finish_mutation(),
            }
        }
        self.flush_pending_save();
        if !self.close_must_wait() {
            if let Some(action) = self.deferred_close.take() {
                self.execute(
                    ctx,
                    match action {
                        PendingAction::Close => Command::CloseDocument,
                        PendingAction::Quit => Command::Quit,
                    },
                );
            }
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty
            || self.annotate.measure.is_active_draft()
            || self.annotate.adv.rich.is_some()
            || self.edit.paragraph.is_some()
            || self.forms.has_unsaved_input()
            || self.ink_draft_active()
            || self.annotate.adv.free_text.is_some()
    }

    pub(crate) fn document_transition_pending(&self) -> bool {
        !self.pending_files.is_empty()
            || self.pending_save.is_some()
            || self.after_save.is_some()
            || self.deferred_close.is_some()
    }

    pub(crate) fn form_builder_ready(&self) -> bool {
        self.document.is_some()
            && matches!(self.engine, EngineState::Ready)
            && !self.navigation_blocked()
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && self.pending_files.is_empty()
            && self.pending_save.is_none()
            && !self.ink_draft_active()
            && !self.forms.has_unsaved_input()
            && !self.forms.pending_clear
            && self.forms.pending_import.is_none()
            && self.annotate.pending_markup.is_none()
            && self.edit.pending_copy.is_none()
            && self.edit.text_prompt.is_none()
            && self.sign.placing.is_none()
    }

    pub(crate) fn ink_can_start(&self) -> bool {
        matches!(self.engine, EngineState::Ready)
            && self.document.is_some()
            && !self.dialog_open()
            && !self.palette.open
            && !self.navigation_blocked()
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && self.pending_files.is_empty()
            && self.pending_save.is_none()
            && !self.forms.has_unsaved_input()
            && !self.forms.pending_clear
            && self.forms.pending_import.is_none()
            && self.annotate.pending_markup.is_none()
            && self.edit.pending_copy.is_none()
    }

    fn start_update_check(&mut self, ctx: &egui::Context, manual: bool) {
        if manual {
            self.update_errors_visible = true;
        }
        if manual || self.settings.check_updates {
            self.updater.start_check(ctx);
        }
    }

    fn update_controls(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let (mut download, mut install, mut check) = (false, false, false);
        ui.horizontal_wrapped(|ui| match &self.updater.phase {
            UpdatePhase::Idle => {}
            UpdatePhase::Checking => {
                ui.spinner();
                ui.label(tr(lang, "updates.checking"));
            }
            UpdatePhase::UpToDate => {
                ui.label(tr(lang, "updates.uptodate"));
            }
            UpdatePhase::Available(release) => {
                ui.label(tr_args(
                    lang,
                    "updates.available",
                    &[("version", &release.version)],
                ));
                download = ui.button(tr(lang, "updates.download")).clicked();
                ui.hyperlink_to(tr(lang, "updates.notes"), &release.notes_url);
            }
            UpdatePhase::Downloading => {
                ui.spinner();
                ui.label(tr(lang, "updates.downloading"));
            }
            UpdatePhase::Ready(prepared) => {
                ui.label(tr_args(
                    lang,
                    "updates.ready",
                    &[("version", &prepared.release.version)],
                ));
                install = ui.button(tr(lang, "updates.install")).clicked();
            }
            UpdatePhase::Failed(error) if self.update_errors_visible => {
                ui.colored_label(
                    theme::palette(self.settings.dark_theme).error,
                    tr_args(lang, "updates.failed", &[("error", error)]),
                );
                check = !self.show_about && ui.button(tr(lang, "updates.check")).clicked();
            }
            UpdatePhase::Failed(_) => {}
        });
        if download {
            self.update_errors_visible = true;
            self.updater.download(ctx);
        }
        if install && !self.ink_blocks_action() {
            self.show_about = false;
            self.install_confirm = true;
        }
        if check {
            self.start_update_check(ctx, true);
        }
    }

    fn update_install_dialog(&mut self, ctx: &egui::Context) {
        if !self.install_confirm {
            return;
        }
        let lang = self.lang();
        let (mut install, mut cancel) = (false, false);
        dialog::show(
            ctx,
            dialog::Spec::new(
                "update-install",
                tr(lang, "updates.confirm.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(460.0),
            |ui| {
                ui.label(tr(lang, "updates.confirm.text"));
            },
            |ui| {
                install = dialog::primary(ui, tr(lang, "updates.confirm.install"));
                cancel = dialog::secondary(ui, tr(lang, "updates.confirm.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if install || cancel {
            self.install_confirm = false;
        }
        if install {
            self.install_on_quit = true;
            self.execute(ctx, Command::Quit);
        } else if cancel {
            self.cancel_after_save();
        }
    }

    pub(crate) fn navigation_blocked(&self) -> bool {
        self.overlay.open
            || self.library.blocks_document()
            || self.review.busy
            || self.overlay.busy
            || self.annotate.measure.saved.dialog_open()
            || self.annotate.measure.is_active_draft()
            || (self.annotate.register.dialog_open() || self.annotate.quantities.open)
            || self.edit.paragraph.is_some()
            || self.annotate.adv.rich.is_some()
            || self.processing.busy
            || self.office.busy
            || self.sign.busy
            || self.saving
            || self.after_save.is_some()
            || self.deferred_close.is_some()
            || self.printing.busy
            || self.ocr.busy
            || self.certificate.busy
    }

    fn close_must_wait(&self) -> bool {
        self.library.blocks_document()
            || self.review.busy
            || self.overlay.busy
            || self.annotate.measure.saved.dialog_open()
            || self.annotate.measure.is_active_draft()
            || (self.annotate.register.dialog_open() || self.annotate.quantities.open)
            || self.edit.paragraph.is_some()
            || self.annotate.adv.rich.is_some()
            || self.ink_draft_active()
            || self.processing.busy
            || self.office.busy
            || self.sign.busy
            || self.worker.has_pending_mutations()
            || self.printing.busy
            || self.ocr.busy
            || self.certificate.busy
            || self.saving
            || self.opening.is_some()
            || self.tabs.suspending.is_some()
            || self.tabs.resuming.is_some()
    }

    pub fn update_title(&self, ctx: &egui::Context) {
        let title = match &self.document {
            Some(doc) => tr_args(
                self.lang(),
                if self.has_unsaved_changes() {
                    "app.title_modified"
                } else {
                    "app.title_with_document"
                },
                &[("name", &file_name(&doc.info.path))],
            ),
            None => self.t("app.title").to_string(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    // ----- Entrées ---------------------------------------------------------

    pub(crate) fn handle_input(&mut self, ctx: &egui::Context) {
        self.validate_measurement_context();
        self.validate_quantity_context();
        self.validate_library_context(ctx);
        if self.annotate.measure.is_active_draft()
            && !self.dialog_open()
            && !self.palette.open
            && !egui::Popup::is_any_open(ctx)
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_measurement();
            return;
        }
        self.validate_free_text();
        self.validate_form_builder();
        if self.form_builder.drawing
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.form_builder.cancel();
            return;
        }
        if !self.annotate.adv.ink.is_empty() && !ctx.input(|i| i.pointer.primary_down()) {
            if let Some(page) = self
                .annotate
                .adv
                .ink_draft
                .as_ref()
                .filter(|draft| !draft.pending)
                .map(|draft| draft.page)
            {
                self.finish_ink_stroke(page);
                self.annotate.drag = None;
            }
        }
        if self.ink_draft_active()
            && !self.dialog_open()
            && !self.palette.open
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_ink_draft();
            return;
        }
        if self.document.is_some()
            && !self.dialog_open()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F11))
        {
            self.execute(ctx, Command::FullScreenReading);
            return;
        }
        if self.reading_full_screen
            && !self.dialog_open()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.set_reading_full_screen(ctx, false);
            return;
        }
        if self.dialog_open() {
            return;
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(path) = dropped.into_iter().next() {
            if is_pdf(&path) {
                self.open_path(path, None);
            } else {
                self.error = Some("error.not_pdf_dropped");
            }
        }
        if self.dialog_open() {
            // Les boîtes de dialogue lisent elles-mêmes Entrée et Échap.
            return;
        }
        if ctx.memory(|m| m.focused().is_some()) {
            // Un champ de saisie a le clavier : seuls les raccourcis avec Ctrl restent actifs.
            // La combinaison avec Maj est lue d'abord (`consume_key` ignore un Maj en trop).
            let mut cmds = Vec::new();
            ctx.input_mut(|i| {
                cmds.extend(
                    crate::shortcuts::consume(i, &self.settings.shortcuts, true)
                        .into_iter()
                        .filter(|c| !self.palette.open || *c == Command::CommandPalette),
                );
            });
            for c in cmds {
                self.execute(ctx, c);
            }
            return;
        }
        if self.palette.open {
            let mut cmds = Vec::new();
            ctx.input_mut(|i| {
                cmds.extend(
                    crate::shortcuts::consume(i, &self.settings.shortcuts, true)
                        .into_iter()
                        .filter(|c| *c == Command::CommandPalette),
                );
            });
            for c in cmds {
                self.execute(ctx, c);
            }
            return;
        }
        if self.find.open && ctx.memory(|m| m.focused().is_some()) {
            // Le champ de recherche a le clavier : seuls Ctrl+F et Échap restent globaux.
            let mut cmds = Vec::new();
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::COMMAND, Key::F) {
                    cmds.push(Command::Find);
                }
            });
            for c in cmds {
                self.execute(ctx, c);
            }
            return;
        }
        if log::log_enabled!(log::Level::Trace) {
            ctx.input(|i| {
                for e in &i.events {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = e
                    {
                        log::trace!("touche {key:?} {modifiers:?}");
                    }
                    if let egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        ..
                    } = e
                    {
                        log::trace!("souris {button:?} pressed={pressed} {pos:?}");
                    }
                }
            });
        }
        (self.edit.shift, self.edit.alt, self.edit.ctrl) =
            ctx.input(|i| (i.modifiers.shift, i.modifiers.alt, i.modifiers.command));
        // Placement d'une signature ou d'une date en cours : Échap l'annule, rien d'autre.
        if self.sign.placing.is_some()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_placing();
        }
        // Liste de choix ou éditeur de champ sans le clavier : Échap les ferme d'abord.
        if (self.forms.popup.is_some() || self.forms.editing.is_some())
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.forms.popup = None;
            self.forms.editing = None;
        }
        // Un Échap = un effet, le plus proche d'abord (`workspace::EscState::action`).
        let esc = crate::workspace::take_escape(ctx, self.esc_state(ctx));
        if esc == Some(Esc::Deselect) {
            self.edit.selection = None;
            self.annotate.adv.selected = None;
            self.annotate.adv.drag = None;
        }
        let object_tool = self.annotate.tool == crate::annotate::Tool::SelectObject;
        if object_tool && self.edit.selection.is_some() {
            // Objet sélectionné : flèches = décalage (Maj : ×10).
            let step = if self.edit.shift { 10.0 } else { 1.0 };
            let mut nudge = (0.0f32, 0.0f32);
            ctx.input_mut(|i| {
                for (k, d) in [
                    (Key::ArrowLeft, (-1.0, 0.0)),
                    (Key::ArrowRight, (1.0, 0.0)),
                    (Key::ArrowUp, (0.0, 1.0)),
                    (Key::ArrowDown, (0.0, -1.0)),
                ] {
                    if i.consume_key(Modifiers::NONE, k) || i.consume_key(Modifiers::SHIFT, k) {
                        nudge = d;
                    }
                }
            });
            if nudge != (0.0, 0.0) {
                self.nudge_selected(nudge.0 * step, nudge.1 * step);
            }
        }
        if self.annotate.tool == crate::annotate::Tool::SelectAnnot
            && self.annotate.adv.selected.is_some()
        {
            let step = if self.edit.shift { 10.0 } else { 1.0 };
            let mut nudge = (0.0f32, 0.0f32);
            ctx.input_mut(|i| {
                for (k, d) in [
                    (Key::ArrowLeft, (-1.0, 0.0)),
                    (Key::ArrowRight, (1.0, 0.0)),
                    (Key::ArrowUp, (0.0, 1.0)),
                    (Key::ArrowDown, (0.0, -1.0)),
                ] {
                    if i.consume_key(Modifiers::NONE, k) || i.consume_key(Modifiers::SHIFT, k) {
                        nudge = d;
                    }
                }
            });
            if nudge != (0.0, 0.0) {
                self.nudge_annot(nudge.0 * step, nudge.1 * step);
            }
        }
        let continuous = self.view.mode != ViewMode::Single;
        if self.document.is_some() && !self.organize.active {
            self.view.scroll_keys(ctx);
        }
        let mut commands = Vec::new();
        if esc == Some(Esc::Leave) {
            commands.push(Command::HubLeave);
        }
        // Ctrl+C / X / V : egui-winit les transforme en événements Copy / Cut / Paste (aucun
        // événement Key). Paste n'existe que si le presse-papiers système contient du texte : la
        // copie d'objets y place un repère, comparé ici.
        let (copy, cut, paste) = ctx.input(|i| {
            (
                i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                i.events.iter().find_map(|e| match e {
                    egui::Event::Paste(s) => Some(s.clone()),
                    _ => None,
                }),
            )
        });
        let objects = object_tool && self.edit.selection.is_some();
        if copy {
            commands.push(if objects {
                Command::CopyObjects
            } else {
                Command::CopyText
            });
        }
        if cut && objects {
            commands.push(Command::CutObjects);
        }
        if let (Some(s), Some(clip)) = (paste, &self.edit.clip) {
            if s.trim() == clip.marker.trim() {
                commands.push(Command::PasteObjects);
            }
        }
        ctx.input_mut(|i| {
            commands.extend(crate::shortcuts::consume(
                i,
                &self.settings.shortcuts,
                false,
            ));
            if i.consume_key(Modifiers::ALT, Key::ArrowLeft) {
                commands.push(Command::HistoryBack);
            }
            if i.consume_key(Modifiers::ALT, Key::ArrowRight) {
                commands.push(Command::HistoryForward);
            }
            if object_tool && i.consume_key(Modifiers::COMMAND, Key::A) {
                commands.push(Command::SelectAllObjects);
            }
            if i.consume_key(Modifiers::NONE, Key::F5) {
                commands.push(if continuous {
                    Command::ViewSinglePage
                } else {
                    Command::ViewContinuous
                });
            }
        });
        for command in commands {
            // Raccourci sans document : ignoré (seuls les clics d'outils ouvrent d'abord un fichier).
            if command.info().needs_document && self.document.is_none() {
                continue;
            }
            self.execute(ctx, command);
        }
    }

    /// Une boîte de dialogue ou un éditeur de texte est ouvert : les raccourcis sont suspendus et
    /// Échap leur appartient (ni la barre de recherche ni l'outil actif ne le voient).
    pub fn dialog_open(&self) -> bool {
        self.library.open
            || self.review.open
            || self.overlay.open
            || self.annotate.measure.saved.dialog_open()
            || (self.annotate.register.dialog_open() || self.annotate.quantities.open)
            || self.review.busy
            || self.overlay.busy
            || self.processing.open
            || self.office.open
            || self.office.busy
            || self.sign.busy
            || self.processing.busy
            || self.recovery.open
            || self.printing.busy
            || self.ocr.dialog_open()
            || self.form_builder.visible
            || self.certificate.visible
            || self.signed_save_confirm.is_some()
            || self.password_prompt.is_some()
            || self.show_about
            || self.shortcuts.draft.is_some()
            || self.unsaved_prompt.is_some()
            || self.tools.split_open
            || self.tools.sanitize_open
            || self.tools.stamp_open.is_some()
            || self.docinfo.unprotect_open
            || self.docinfo.properties_open
            || self.docinfo.protect_open
            || self.pagesetup.dialog.is_some()
            || self.annotate.note_prompt.is_some()
            || self.edit.text_prompt.is_some()
            || self.edit.paragraph.is_some()
            || self.sign.open.is_some()
            || self.forms.flatten_open
            || self.annotate.adv.dialog_open()
            || self.annotate.redact.dialog_open()
            || self.install_confirm
    }

    /// Ouvre le prochain fichier en attente (chaque fichier devient un onglet).
    fn open_next_pending(&mut self) {
        if !self.pending_files.is_empty() {
            let path = self.pending_files.remove(0);
            self.open_path(path, None);
        }
    }

    pub fn request_quit_prompt(&mut self) {
        self.unsaved_prompt = Some(PendingAction::Quit);
    }

    pub fn execute(&mut self, ctx: &egui::Context, command: Command) {
        if self.measurement_blocks_panel_command(command) {
            self.status_msg = Some(self.t("measure.finish_draft_first").to_owned());
            return;
        }
        if self.edit.paragraph.is_some() {
            self.status_msg = Some(self.t("paragraph.finish_first").into());
            return;
        }
        log::debug!("commande {}", command.info().id);
        if self.ink_blocks_command(command) {
            self.ink_blocks_action();
            return;
        }
        if (self.review.busy
            || self.overlay.open
            || self.overlay.busy
            || self.processing.busy
            || self.printing.busy
            || self.ocr.busy
            || self.certificate.busy
            || self.sign.busy)
            && !matches!(command, Command::Quit | Command::CloseDocument)
        {
            return;
        }
        if command == Command::CloseDocument && self.install_on_quit {
            self.cancel_after_save();
        }
        if matches!(command, Command::CloseDocument | Command::Quit) && self.close_must_wait() {
            self.overlay
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.review
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.processing
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.ocr
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.printing
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.sign
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.deferred_close = Some(if command == Command::CloseDocument {
                PendingAction::Close
            } else {
                PendingAction::Quit
            });
            return;
        }
        if matches!(command, Command::Quit | Command::CloseDocument) && self.overlay.open {
            self.overlay.close();
        }
        if command.info().needs_document && self.document.is_none() {
            // L'outil reste utilisable, il commence par ouvrir un fichier.
            self.after_open = Some(command);
            self.open_file_dialog();
            return;
        }
        if !matches!(
            command,
            Command::FirstPage
                | Command::PreviousPage
                | Command::NextPage
                | Command::LastPage
                | Command::ZoomIn
                | Command::ZoomOut
                | Command::ActualSize
                | Command::FitWidth
                | Command::FitPage
                | Command::ViewContinuous
                | Command::ViewSinglePage
                | Command::ViewTwoPage
                | Command::ViewBook
                | Command::HistoryBack
                | Command::HistoryForward
        ) {
            self.form_builder.cancel();
        }
        match command {
            Command::CompareFiles => self.open_local_review(true),
            Command::RevisionOverlay => self.open_revision_overlay(),
            Command::SearchFiles => self.open_local_review(false),
            Command::CompressImages => self.open_processing(false),
            Command::ProcessFiles => self.open_processing(true),
            Command::RichAnnotations => self.open_rich_annotations(ctx),
            Command::ToolMeasure => self.open_measurements(ctx),
            Command::AnnotationLibrary => self.open_annotation_library(),
            Command::AnnotationRegister => self.open_annotation_register(),
            Command::QuantityTable => self.open_quantity_table(),
            Command::EraseInkStrokes => self.open_ink_eraser(ctx),
            Command::OfficeToPdf => self.open_office(false),
            Command::ExportOffice => self.open_office(true),
            Command::RotateViewClockwise => self.rotate_reading_view(true),
            Command::RotateViewCounterClockwise => self.rotate_reading_view(false),
            Command::HistoryBack => self.navigate_reading_history(true),
            Command::HistoryForward => self.navigate_reading_history(false),
            Command::FullScreenReading => {
                if !self.reading_full_screen {
                    if self.forms.has_unsaved_input()
                        || self.edit.text_prompt.is_some()
                        || self.worker.has_pending_mutations()
                    {
                        self.status_msg = Some(self.t("reading.finish_edit").into());
                        return;
                    }
                    self.annotate.tool = crate::annotate::Tool::Select;
                    self.annotate.drag = None;
                    self.annotate.adv.drag = None;
                    self.annotate.adv.selected = None;
                    self.annotate.adv.ink.clear();
                    self.edit.selection = None;
                    self.forms.editing = None;
                    self.forms.popup = None;
                    self.cancel_placing();
                }
                self.organize.active = false;
                self.set_reading_full_screen(ctx, !self.reading_full_screen);
            }
            Command::OpenDocument => self.open_file_dialog(),
            Command::RecoverDocuments => {
                if let Some(session) = &self.recovery.session {
                    match session.scan() {
                        Ok(mut entries) => {
                            self.recovery.failed = false;
                            entries.retain(|entry| {
                                !self
                                    .recovery
                                    .restored
                                    .values()
                                    .any(|open| open.snapshot == entry.snapshot)
                            });
                            self.recovery.entries = entries;
                            self.recovery.open = true;
                        }
                        Err(e) => {
                            log::error!("Recovery scan failed: {e}");
                            self.error = Some("error.recovery_failed");
                        }
                    }
                }
            }
            Command::PrintDocument => self.start_print(),
            Command::OcrDocument => {
                self.ocr.open = true;
            }
            Command::ToolAddTextField
            | Command::ToolAddCheckbox
            | Command::ToolAddRadioField
            | Command::ToolAddComboField
            | Command::ToolAddListField => {
                if !self.form_builder_ready() {
                    self.status_msg = Some(self.t("reading.finish_edit").into());
                    return;
                }
                if let Some(doc) = &self.document {
                    let kind = match command {
                        Command::ToolAddRadioField => sunipdf_core::FieldKind::Radio,
                        Command::ToolAddComboField => sunipdf_core::FieldKind::Combo,
                        Command::ToolAddListField => sunipdf_core::FieldKind::List,
                        Command::ToolAddCheckbox => sunipdf_core::FieldKind::Checkbox,
                        _ => sunipdf_core::FieldKind::Text,
                    };
                    self.form_builder
                        .open(doc.page, kind, doc.info.page_sizes[doc.page]);
                    self.form_builder.bind(
                        self.view.epoch,
                        self.annotate.tool,
                        self.annotate.workspace,
                    );
                }
            }
            Command::VerifySignatures | Command::SignWithCertificate => {
                if self.has_unsaved_changes() || self.worker.has_pending_mutations() {
                    self.error = Some("error.certificate_save_first");
                } else {
                    self.certificate
                        .open(command == Command::SignWithCertificate);
                    if command == Command::VerifySignatures {
                        self.certificate.busy = true;
                        self.worker.send(Request::VerifySignatures);
                    }
                }
            }
            Command::CloseDocument => {
                if self.has_unsaved_changes() {
                    self.unsaved_prompt = Some(PendingAction::Close);
                } else {
                    self.close_document(true);
                    self.update_title(ctx);
                }
            }
            Command::Quit => {
                if self.has_unsaved_changes() {
                    self.unsaved_prompt = Some(PendingAction::Quit);
                } else if let Some(i) = self.any_background_dirty() {
                    self.tabs.quit_pending = true;
                    self.switch_to(i);
                } else {
                    if self.install_on_quit {
                        self.install_on_quit = false;
                        if let Err(error) = self.updater.launch_prepared() {
                            log::warn!("installation impossible : {error}");
                            self.error = Some("updates.install_failed");
                            return;
                        }
                    }
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Command::FirstPage => self.go_to_page(0),
            Command::PreviousPage => self.step_page(-1),
            Command::NextPage => self.step_page(1),
            Command::LastPage => self.go_to_page(usize::MAX),
            Command::ZoomIn => self.zoom_step(1),
            Command::ZoomOut => self.zoom_step(-1),
            Command::ActualSize => self.zoom = Zoom::Fixed(1.0),
            Command::FitWidth => self.zoom = Zoom::FitWidth,
            Command::FitPage => self.zoom = Zoom::FitPage,
            Command::ViewContinuous => self.set_view_mode(ViewMode::Continuous),
            Command::ViewSinglePage => self.set_view_mode(ViewMode::Single),
            Command::ViewTwoPage => self.set_view_mode(ViewMode::TwoPage),
            Command::ViewBook => self.set_view_mode(ViewMode::Book),
            Command::ToggleThumbnails => {
                self.toggle_navigation_panel(crate::reader_layout::NavigationPanel::Pages);
            }
            Command::Find => {
                self.find.open = true;
                self.find.focus_requested = true;
            }
            Command::Save => self.save_document(false),
            Command::SaveAs => self.save_document(true),
            Command::RotateClockwise | Command::RotateCounterClockwise => {
                let pages = self.target_pages();
                let delta_degrees = if command == Command::RotateClockwise {
                    90
                } else {
                    -90
                };
                self.worker.send(Request::Edit(EditOp::Rotate {
                    pages,
                    delta_degrees,
                }));
            }
            Command::DeletePage => {
                let Some(doc) = &self.document else { return };
                let pages = self.target_pages();
                if pages.len() >= doc.info.page_count() {
                    self.error = Some("error.last_page");
                } else {
                    self.worker.send(Request::Edit(EditOp::Delete { pages }));
                    // Les index de la grille désigneraient d'autres pages après la suppression.
                    self.organize.selected.clear();
                    self.organize.anchor = None;
                }
            }
            Command::Undo => {
                if self.can_undo {
                    self.worker.send(Request::Undo);
                }
            }
            Command::Redo => {
                if self.can_redo {
                    self.worker.send(Request::Redo);
                }
            }
            Command::InsertPages => {
                let Some(doc) = &self.document else { return };
                let at = self
                    .target_pages()
                    .last()
                    .map(|p| p + 1)
                    .unwrap_or(doc.page + 1);
                let dialog = rfd::FileDialog::new()
                    .set_title(self.t("dialog.insert.title"))
                    .add_filter(self.t("dialog.open.filter_pdf"), &["pdf"]);
                if let Some(source) = dialog.pick_file() {
                    self.worker.send(Request::Edit(EditOp::Insert {
                        source,
                        password: None,
                        pages: Vec::new(),
                        at,
                    }));
                }
            }
            Command::MovePageLeft | Command::MovePageRight => {
                let Some(doc) = &self.document else { return };
                let pages = self.target_pages();
                let (Some(&first), Some(&last)) = (pages.first(), pages.last()) else {
                    return;
                };
                if pages.len() != 1 {
                    return; // un seul déplacement à la fois ; blocs contigus plus tard.
                }
                let count = doc.info.page_count();
                let to = if command == Command::MovePageLeft {
                    first.checked_sub(1)
                } else if last + 1 < count {
                    Some(last + 1)
                } else {
                    None
                };
                if let Some(to) = to {
                    self.worker
                        .send(Request::Edit(EditOp::Move { from: first, to }));
                    self.organize.selected = std::iter::once(to).collect();
                    self.organize.anchor = Some(to);
                }
            }
            Command::CopyText => self.copy_selection(ctx),
            Command::DeleteObject => {
                if self.annotate.adv.selected.is_some() {
                    self.delete_selected_annot();
                } else {
                    self.delete_selected_object();
                }
            }
            Command::DuplicateObject => self.duplicate_selected(),
            Command::RotateObjectLeft => self.rotate_selected(-90.0),
            Command::RotateObjectRight => self.rotate_selected(90.0),
            Command::BringToFront => self.reorder_selected(sunipdf_core::ZOrder::Front),
            Command::SendToBack => self.reorder_selected(sunipdf_core::ZOrder::Back),
            Command::EditObjectText => self.open_text_prompt_for_selection(),
            Command::CopyObjects => self.copy_objects(false),
            Command::CutObjects => self.copy_objects(true),
            Command::PasteObjects => self.paste_objects(ctx),
            Command::PasteEditableObjects => self.paste_editable_objects(ctx),
            Command::SelectAllObjects => self.select_all_objects(ctx),
            Command::ReplaceImage => self.replace_selected_image(),
            Command::ToggleObjects => {
                self.toggle_context_panel(command);
            }
            Command::CropPages
            | Command::ResizePages
            | Command::InsertBlankPage
            | Command::PageLabels => self.open_page_dialog(command),
            Command::ToolSelect
            | Command::ToolHighlight
            | Command::ToolUnderline
            | Command::ToolStrikeOut
            | Command::ToolNote
            | Command::ToolCount
            | Command::ToolSelectObject
            | Command::ToolAddText
            | Command::ToolAddImage
            | Command::ToolRectangle
            | Command::ToolEllipse
            | Command::ToolLine
            | Command::ToolSelectAnnot
            | Command::ToolFreeText
            | Command::ToolInk
            | Command::ToolAnnotRect
            | Command::ToolAnnotEllipse
            | Command::ToolAnnotLine
            | Command::ToolLink
            | Command::ToolRedact => {
                self.cancel_placing();
                self.annotate.clear_reading();
                self.annotate.tool =
                    crate::annotate::Tool::from_command(command).unwrap_or_default();
                if command == Command::ToolCount {
                    self.annotate.register.count_open = true;
                }
                self.annotate.drag = None;
                if !matches!(self.annotate.tool, crate::annotate::Tool::SelectObject) {
                    self.edit.selection = None;
                }
                if self.annotate.tool != crate::annotate::Tool::SelectAnnot {
                    self.annotate.adv.selected = None;
                }
                self.annotate.adv.ink.clear();
                self.annotate.adv.drag = None;
                // Les annotations de la page survolée sont relues pour l'outil de sélection.
                self.annotate.annotations_page = None;
                // Raccourci, palette ou menu : la barre de l'outil propriétaire suit.
                crate::workspace::follow(
                    &mut self.annotate,
                    &mut self.organize,
                    &mut self.edit,
                    command,
                );
                if command == Command::ToolRedact {
                    self.redact_status();
                }
            }
            Command::RedactSearch | Command::RedactClear | Command::RedactApply => {
                self.redact_command(ctx, command)
            }
            Command::HubFillSign
            | Command::HubEdit
            | Command::HubOrganize
            | Command::HubComment
            | Command::HubMeasure
            | Command::HubProtect
            | Command::HubStamp
            | Command::HubPrepareForm
            | Command::HubOcr
            | Command::HubCompress
            | Command::HubCertificate
            | Command::HubRedact => {
                if let Some(ws) = crate::workspace::Workspace::from_command(command) {
                    self.enter_workspace(ctx, ws);
                }
            }
            Command::HubLeave => {
                self.cancel_placing();
                self.leave_workspace();
            }
            Command::SignDocument => self.open_sign_dialog(crate::sign::SignKind::Signature),
            Command::SignInitials => self.open_sign_dialog(crate::sign::SignKind::Initials),
            Command::InsertDate => self.start_placing_date(),
            Command::ToolFillForm => self.enter_fill_tool(),
            Command::ToggleForms => {
                self.toggle_context_panel(command);
            }
            Command::ClearForm => {
                self.forms.editing = None;
                self.forms.popup = None;
                // Exécuté sur le modèle lu (`refresh_forms_if_needed` / `on_form_info`).
                self.forms.pending_clear = true;
            }
            Command::FlattenForm => {
                self.forms.flatten_all_pages = true;
                self.forms.flatten_open = true;
            }
            Command::ExportFormData => self.tool_export_form(),
            Command::ImportFormData => self.tool_import_form(),
            Command::CombineFiles => self.tool_combine(),
            Command::SplitDocument => {
                self.tools.split_every = self.tools.split_every.max(1);
                self.tools.split_open = true;
            }
            Command::Watermark => self.tools.stamp_open = Some(crate::tools::StampKind::Watermark),
            Command::PageNumbers => {
                self.tools.stamp_open = Some(crate::tools::StampKind::PageNumbers)
            }
            Command::ExtractPages => self.tool_extract(),
            Command::CreateFromImages => self.tool_create_from_images(),
            Command::ExportImages => self.tool_export_images(),
            Command::Optimize => self.tool_optimize(),
            Command::ToggleBookmarks => {
                self.toggle_navigation_panel(crate::reader_layout::NavigationPanel::Bookmarks);
            }
            Command::Properties => {
                self.docinfo.properties = None;
                self.docinfo.inspection_epoch = None;
                self.docinfo.inspection = None;
                self.docinfo.properties_open = true;
                self.worker.send(Request::Properties);
            }
            Command::Protect => {
                self.docinfo.protection = sunipdf_core::Protection::default();
                self.docinfo.protect_error = None;
                self.docinfo.current_owner = None;
                self.docinfo.protect_open = false;
                self.docinfo.protection_epoch = Some(self.view.epoch);
                self.worker.send(Request::BeginProtection {
                    epoch: self.view.epoch,
                });
            }
            Command::Unprotect => {
                self.docinfo.protection_epoch = None;
                self.docinfo.current_owner = None;
                let encrypted = self.docinfo.properties.as_ref().map(|p| p.encrypted);
                if encrypted == Some(false) {
                    self.error = Some("error.not_encrypted");
                } else {
                    self.docinfo.unprotect_password.clear();
                    self.docinfo.unprotect_for_reprotect = false;
                    self.docinfo.unprotect_open = true;
                }
            }
            Command::StripMetadata => {
                self.worker
                    .send(Request::Edit(EditOp::Bytes(BytesOp::StripMetadata)));
            }
            Command::Sanitize => self.tools.sanitize_open = true,
            Command::ToggleComments => {
                self.toggle_context_panel(command);
            }
            Command::OrganizePages => {
                self.organize.active = !self.organize.active;
                if self.organize.active {
                    // La grille masque la page : pas de sélection d'objet invisible (Suppr, flèches).
                    self.edit.selection = None;
                    if let Some(doc) = &self.document {
                        self.organize.selected = std::iter::once(doc.page).collect();
                        self.organize.anchor = Some(doc.page);
                    }
                    self.close_find();
                }
                crate::workspace::follow(
                    &mut self.annotate,
                    &mut self.organize,
                    &mut self.edit,
                    command,
                );
            }
            Command::NextTab => self.next_tab(1),
            Command::PrevTab => self.next_tab(-1),
            Command::ToggleTools => {
                self.toggle_navigation_panel(crate::reader_layout::NavigationPanel::Tools);
            }
            Command::CommandPalette => {
                self.palette.open = !self.palette.open;
                self.palette.query.clear();
                self.palette.selected = 0;
                self.palette.focus_requested = self.palette.open;
            }
            Command::KeyboardShortcuts => {
                self.shortcuts.draft = Some(crate::shortcuts::Draft::new(&self.settings.shortcuts));
                self.palette.open = false;
            }
            Command::About => self.show_about = true,
        }
    }

    // ----- Interface -------------------------------------------------------

    /// Menu construit depuis le registre ; `separators_before` insère une ligne avant ces commandes.
    pub fn command_menu(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        title_key: &str,
        menu: Menu,
        separators_before: &[Command],
    ) {
        let lang = self.lang();
        let has_doc = self.document.is_some();
        ui.menu_button(tr(lang, title_key), |ui| {
            for info in Command::in_menu(menu) {
                if separators_before.contains(&info.command) {
                    ui.separator();
                }
                let shortcut = self.shortcut_text(info.command);
                let enabled = !info.needs_document || has_doc;
                // Mode de vue actif surligné (les deux entrées partagent F5).
                let active = has_doc
                    && match info.command {
                        Command::ViewContinuous => self.view.mode == ViewMode::Continuous,
                        Command::ViewSinglePage => self.view.mode == ViewMode::Single,
                        Command::ViewTwoPage => self.view.mode == ViewMode::TwoPage,
                        Command::ViewBook => self.view.mode == ViewMode::Book,
                        _ => false,
                    };
                if menu_item(ui, tr(lang, info.label_key), &shortcut, enabled, active) {
                    self.execute(ctx, info.command);
                }
            }
        });
    }

    fn error_banner(&mut self, ui: &mut egui::Ui) {
        let Some(key) = self.error else { return };
        let lang = self.lang();
        egui::Frame::new()
            .fill(theme::ERROR.gamma_multiply(0.15))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let error = theme::palette(ui.visuals().dark_mode).error;
                    ui.colored_label(error, format!("⚠ {}", tr(lang, key)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(lang, "error.dismiss")).clicked() {
                            self.error = None;
                        }
                    });
                });
            });
    }

    fn document_area(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        if self.document.is_none() {
            if matches!(self.engine, EngineState::Failed) {
                ui.centered_and_justified(|ui| {
                    let error = theme::palette(ui.visuals().dark_mode).error;
                    ui.colored_label(error, tr(lang, "error.engine_missing"))
                });
                return;
            }
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.3);
                ui.heading(tr(lang, "empty.title"));
                ui.label(tr(lang, "empty.hint"));
                ui.add_space(12.0);
                if ui.button(tr(lang, "empty.open_button")).clicked() {
                    self.open_file_dialog();
                }
            });
            return;
        }
        self.document_view(ctx, ui);
    }

    pub(crate) fn retain_thumbnail_window(&mut self, visible: std::ops::Range<usize>) {
        let margin = crate::view::THUMB_KEEP.saturating_sub(visible.len()) / 2;
        let start = visible.start.saturating_sub(margin);
        self.thumb_window = start..start.saturating_add(crate::view::THUMB_KEEP);
        self.thumbnails
            .retain(|page, _| self.thumb_window.contains(page));
        self.thumbs_failed
            .retain(|page| self.thumb_window.contains(page));
    }

    pub(crate) fn request_thumbnail(&mut self, page: usize, width_px: u32) {
        if self.thumbs_pending.len() < crate::view::THUMB_KEEP
            && self.thumb_window.contains(&page)
            && !self.thumbnails.contains_key(&page)
            && !self.thumbs_failed.contains(&page)
            && self.thumbs_pending.insert(page)
        {
            self.worker.send(Request::Thumbnail { page, width_px });
        }
    }

    /// Panneau des miniatures : seules les lignes visibles demandent un rendu (cache côté moteur).
    pub(crate) fn thumbnails_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        ui.add_space(4.0);
        ui.label(crate::icons::semibold(tr(lang, "panel.thumbnails"), 14.0));
        ui.separator();
        let Some(doc) = &self.document else { return };
        let count = doc.info.page_count();
        let current = doc.page;
        let thumb_w = (ui.available_width() - 12.0).max(48.0);
        // Largeur en pixels physiques, arrondie pour ne pas relancer les rendus à chaque pixel.
        let px = ((thumb_w * ctx.pixels_per_point() / 16.0).round() * 16.0).max(16.0) as u32;
        if px != self.thumb_px {
            self.thumb_px = px;
            self.thumbnails.clear();
            self.thumbs_failed.clear();
        }
        let max_ratio = doc
            .info
            .page_sizes
            .iter()
            .map(|s| s.height_pt / s.width_pt)
            .fold(0.0_f32, f32::max)
            .max(0.1);
        let row_h = thumb_w * max_ratio + 24.0;
        let sizes = doc.info.page_sizes.clone();
        let labels = doc.info.page_labels.clone();
        let mut clicked = None;
        let mut moved: Option<(usize, usize)> = None;
        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if std::mem::take(&mut self.view.follow_thumb) {
            // Page courante changée ailleurs (défilement, « aller à ») : sa miniature est ramenée
            // au centre du panneau si elle n'y est pas entièrement visible.
            let top = current as f32 * (row_h + ui.spacing().item_spacing.y);
            let seen = self
                .view
                .thumb_view
                .is_some_and(|(off, h)| top >= off && top + row_h <= off + h);
            if !seen {
                area = area
                    .vertical_scroll_offset((top - (ui.available_height() - row_h) / 2.0).max(0.0));
            }
        }
        let out = area.show_rows(ui, row_h, count, |ui, rows| {
            self.retain_thumbnail_window(rows.clone());
            for page in rows {
                let page_number = (page + 1).to_string();
                let label = labels.get(page).and_then(Option::as_deref);
                let caption = label.map_or_else(
                    || page_number.clone(),
                    |label| format!("{label} (#{page_number})"),
                );
                let ratio = sizes[page].height_pt / sizes[page].width_pt;
                let size = egui::vec2(thumb_w, thumb_w * ratio);
                self.request_thumbnail(page, px);
                ui.vertical_centered(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                    resp.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Image,
                            true,
                            tr_args(lang, "a11y.thumbnail", &[("page", &caption)]),
                        )
                    });
                    resp.dnd_set_drag_payload(page);
                    if let Some(payload) = resp.dnd_hover_payload::<usize>() {
                        if *payload != page {
                            let above = ui
                                .input(|i| i.pointer.hover_pos())
                                .is_some_and(|pos| pos.y < rect.center().y);
                            let y = if above {
                                rect.top() - 6.0
                            } else {
                                rect.bottom() + 6.0
                            };
                            ui.painter().hline(
                                rect.x_range(),
                                y,
                                egui::Stroke::new(3.0, theme::BLUE),
                            );
                        }
                    }
                    if let Some(payload) = resp.dnd_release_payload::<usize>() {
                        let from = *payload;
                        if from != page {
                            let above = ui
                                .input(|i| i.pointer.latest_pos())
                                .is_some_and(|pos| pos.y < rect.center().y);
                            let mut to = if above { page } else { page + 1 };
                            if from < to {
                                to -= 1;
                            }
                            if from != to {
                                moved = Some((from, to));
                            }
                        }
                    }
                    let stroke = if page == current {
                        egui::Stroke::new(2.0, theme::BLUE)
                    } else {
                        egui::Stroke::new(1.0, egui::Color32::from_gray(160))
                    };
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
                    if let Some(tex) = self.thumbnails.get(&page) {
                        ui.put(
                            rect,
                            egui::Image::new(egui::load::SizedTexture::new(tex.id(), size))
                                .alt_text(tr_args(lang, "a11y.thumbnail", &[("page", &caption)])),
                        );
                    }
                    ui.painter()
                        .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
                    if resp.clicked() {
                        clicked = Some(page);
                    }
                    ui.add(egui::Label::new(&caption).truncate())
                        .on_hover_text(caption);
                });
            }
        });
        self.view.thumb_view = Some((out.state.unclamped_offset().y, out.inner_rect.height()));
        if let Some(page) = clicked {
            self.go_to_page(page);
        }
        if let Some((from, to)) = moved {
            self.worker.send(Request::Edit(EditOp::Move { from, to }));
            self.go_to_page(to);
        }
    }

    fn find_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let has_icons = self.has_icons;
        let mut step: isize = 0;
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label("🔍");
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.find.query)
                    .hint_text(tr(lang, "find.placeholder"))
                    .desired_width(260.0),
            );
            if self.find.focus_requested {
                field.request_focus();
                self.find.focus_requested = false;
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                step = if ui.input(|i| i.modifiers.shift) {
                    -1
                } else {
                    1
                };
                self.find.focus_requested = true;
            }
            ui.checkbox(&mut self.find.match_case, tr(lang, "find.match_case"));
            let has_matches = !self.find.matches.is_empty();
            if ui
                .add_enabled(has_matches, egui::Button::new("▲"))
                .on_hover_text(tr(lang, "find.previous"))
                .clicked()
            {
                step = -1;
            }
            if ui
                .add_enabled(has_matches, egui::Button::new("▼"))
                .on_hover_text(tr(lang, "find.next"))
                .clicked()
            {
                step = 1;
            }
            if self.find.in_flight {
                ui.spinner();
                ui.label(tr(lang, "find.searching"));
            } else if self.find.searched.is_some() {
                if has_matches {
                    ui.label(tr_args(
                        lang,
                        "find.count",
                        &[
                            ("current", &(self.find.current + 1).to_string()),
                            ("total", &self.find.matches.len().to_string()),
                        ],
                    ));
                } else {
                    let error = theme::palette(ui.visuals().dark_mode).error;
                    ui.colored_label(error, tr(lang, "find.none"));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Glyphe de la police d'icônes : « ✕ » manque dans Segoe UI (carré vide).
                let close_icon = crate::icons::icon(has_icons, glyph::CLOSE, "x", 13.0);
                if ui
                    .add(egui::Button::new(close_icon).frame(false))
                    .on_hover_text(tr(lang, "find.close"))
                    .clicked()
                {
                    close = true;
                }
            });
        });
        if close {
            self.close_find();
            return;
        }
        // Échap d'une boîte de dialogue ou de la palette (dessinées après) ne ferme pas la recherche.
        if !self.dialog_open() && !self.palette.open && ui.input(|i| i.key_pressed(Key::Escape)) {
            self.close_find();
            return;
        }
        self.request_search_if_needed();
        if step != 0 && !self.find.matches.is_empty() {
            let n = self.find.matches.len() as isize;
            self.find.current = ((self.find.current as isize + step).rem_euclid(n)) as usize;
            self.go_to_match(self.find.current);
        }
    }

    pub fn close_find(&mut self) {
        self.find.open = false;
        self.find.matches.clear();
        self.find.searched = None;
        self.find.in_flight = false;
    }

    fn request_search_if_needed(&mut self) {
        let wanted = (self.find.query.clone(), self.find.match_case);
        if self.find.in_flight
            || self.find.searched.as_ref() == Some(&wanted)
            || self.document.is_none()
        {
            return;
        }
        if wanted.0.trim().is_empty() {
            self.find.matches.clear();
            self.find.searched = Some(wanted);
            return;
        }
        self.find.generation += 1;
        self.find.in_flight = true;
        self.find.searched = Some(wanted.clone());
        self.worker.send(Request::Search {
            query: wanted.0,
            match_case: wanted.1,
            generation: self.find.generation,
        });
    }

    /// Surbrillance des occurrences de la page affichée (coordonnées PDF → écran).
    pub fn paint_matches(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        if !self.find.open {
            return;
        }
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        // Résultats triés par page : seuls ceux de cette page sont parcourus.
        let start = self.find.matches.partition_point(|m| m.page < page_index);
        let on_page = self.find.matches[start..]
            .iter()
            .take_while(|m| m.page == page_index);
        for (i, m) in on_page.enumerate() {
            let color = if start + i == self.find.current {
                egui::Color32::from_rgba_unmultiplied(0xF5, 0x9E, 0x0B, 150)
            } else {
                egui::Color32::from_rgba_unmultiplied(0xFD, 0xE0, 0x47, 110)
            };
            for r in &m.rects {
                let rect = egui::Rect::from_min_max(
                    page_rect.min + egui::vec2(r.left * k, (page_h_pt - r.top) * k),
                    page_rect.min + egui::vec2(r.right * k, (page_h_pt - r.bottom) * k),
                );
                painter.rect_filled(rect, 2.0, color);
            }
        }
    }

    fn unsaved_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(action) = self.unsaved_prompt.clone() else {
            return;
        };
        let name = self
            .document
            .as_ref()
            .map(|d| file_name(&d.info.path))
            .unwrap_or_default();
        let mut choice = None;
        let spec = dialog::Spec::new(
            "unsaved",
            tr(lang, "dialog.unsaved.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "dialog.unsaved.subtitle"))
        .icon(glyph::WARNING, "!")
        .warning()
        .width(460.0);
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(tr_args(lang, "dialog.unsaved.text", &[("name", &name)]));
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "dialog.unsaved.save")) {
                    choice = Some(UnsavedChoice::Save);
                }
                if dialog::danger(ui, tr(lang, "dialog.unsaved.discard")) {
                    choice = Some(UnsavedChoice::Discard);
                }
                if dialog::secondary(ui, tr(lang, "dialog.unsaved.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    choice = Some(UnsavedChoice::Cancel);
                }
            },
        );
        match choice {
            Some(UnsavedChoice::Save) => {
                self.unsaved_prompt = None;
                self.after_save = Some(action);
                self.save_document(false);
            }
            Some(UnsavedChoice::Discard) => {
                self.unsaved_prompt = None;
                self.close_document(true);
                self.update_title(ctx);
                if matches!(action, PendingAction::Quit) {
                    self.execute(ctx, Command::Quit);
                }
            }
            Some(UnsavedChoice::Cancel) => {
                self.unsaved_prompt = None;
                self.cancel_after_save();
            }
            None => {}
        }
    }

    /// Enregistre vers le chemin courant, ou demande un chemin (`ask_path`). Une saisie de champ
    /// en cours est validée d'abord (le thread moteur l'applique avant d'enregistrer).
    /// Abandonne l'action prévue après l'enregistrement (fermer, quitter…).
    pub(crate) fn cancel_after_save(&mut self) {
        self.after_save = None;
        if self.install_on_quit {
            self.install_on_quit = false;
            self.deferred_close = None;
        }
    }

    pub(crate) fn save_document(&mut self, ask_path: bool) {
        if self.ink_blocks_action() {
            return;
        }
        self.form_builder.cancel();
        if self.saving {
            return;
        }
        let ask_path = self.requires_save_as(ask_path);
        if !ask_path && !self.has_unsaved_changes() && !self.worker.has_pending_mutations() {
            return;
        }
        if self.has_signatures {
            self.signed_save_confirm = Some(ask_path);
            return;
        }
        // Zones de caviardage non appliquées : le fichier contiendrait encore le contenu marqué.
        // Garde ici (et non dans `execute`) : la boîte « non enregistré » appelle cette fonction.
        if self.redact_blocks_save(ask_path) {
            return;
        }
        if !self.commit_form_input() {
            self.cancel_after_save();
            return;
        }
        let Some(doc) = &self.document else { return };
        let mut path = doc.info.path.clone();
        if ask_path {
            let dialog = rfd::FileDialog::new()
                .set_title(self.t("dialog.save.title"))
                .add_filter(self.t("dialog.open.filter_pdf"), &["pdf"])
                .set_file_name(file_name(&path));
            let dialog = match path.parent() {
                Some(dir) => dialog.set_directory(dir),
                None => dialog,
            };
            match dialog.save_file() {
                Some(chosen) => path = chosen,
                None => {
                    self.cancel_after_save();
                    return;
                }
            }
            if path.extension().is_none() {
                path.set_extension("pdf");
            }
        }
        self.saving = true;
        self.pending_save = Some(path);
        self.flush_pending_save();
    }

    fn flush_pending_save(&mut self) {
        if self.pending_save.is_none() || self.worker.has_pending_mutations() {
            return;
        }
        if !self.commit_form_input() {
            self.pending_save = None;
            self.saving = false;
            self.cancel_after_save();
            return;
        }
        if !self.worker.has_pending_mutations() {
            self.saving_drafts = self.forms.drafts.clone();
            if let Some(path) = self.pending_save.take() {
                self.worker.send(Request::Save { path });
            }
        }
    }

    pub(crate) fn requires_save_as(&self, ask_path: bool) -> bool {
        ask_path
            || self
                .tabs
                .active
                .and_then(|i| self.tabs.tabs.get(i))
                .is_some_and(|tab| self.recovery.restored.contains_key(&tab.recovery_id))
    }

    fn signed_save_dialog(&mut self, ctx: &egui::Context) {
        let Some(ask_path) = self.signed_save_confirm else {
            return;
        };
        let lang = self.lang();
        let (mut proceed, mut cancel) = (false, false);
        dialog::show(
            ctx,
            dialog::Spec::new(
                "signed-save",
                tr(lang, "certificate.save_warning_title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(480.0),
            |ui| {
                ui.label(tr(lang, "certificate.save_warning"));
            },
            |ui| {
                proceed = dialog::primary(ui, tr(lang, "certificate.save_changed"));
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if proceed || cancel {
            self.signed_save_confirm = None;
        }
        if proceed {
            self.has_signatures = false;
            self.save_document(ask_path);
            self.has_signatures = true;
        }
        if cancel {
            self.cancel_after_save();
        }
    }

    fn password_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(prompt) = &mut self.password_prompt else {
            return;
        };
        let mut submit = None;
        let mut cancel = false;
        let spec = dialog::Spec::new(
            "password",
            tr(lang, "dialog.password.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(file_name(&prompt.path))
        .icon(glyph::LOCK, "🔒")
        .width(420.0);
        let (path, wrong) = (prompt.path.clone(), prompt.wrong);
        let input = &mut prompt.input;
        let mut ok = false;
        let mut enter = false;
        dialog::show(
            ctx,
            spec,
            |ui| {
                dialog::field_label(ui, tr(lang, "dialog.password.label"));
                let field = ui.add(
                    egui::TextEdit::singleline(input)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    enter = true;
                }
                if wrong {
                    let error = theme::palette(ui.visuals().dark_mode).error;
                    ui.colored_label(error, tr(lang, "error.password_wrong"));
                } else {
                    ui.label(
                        egui::RichText::new(tr(lang, "dialog.password.subtitle"))
                            .size(12.5)
                            .color(theme::palette(ui.visuals().dark_mode).text_muted),
                    );
                }
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "dialog.password.ok")) {
                    ok = true;
                }
                if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if ok || enter {
            submit = Some((path, input.clone()));
        }
        if let Some((path, password)) = submit {
            self.opening = Some(path.clone());
            self.worker.send(Request::Open {
                path,
                password: Some(password),
            });
        } else if cancel {
            self.password_prompt = None;
            self.review.pending_page = None;
            self.cancel_after_save();
        }
    }

    fn about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let logo = self.brand_wordmark(ctx, 150.0);
        let mut close = false;
        let mut startup_check = self.settings.check_updates;
        let mut autosave = self.settings.autosave;
        let mut manual_check = false;
        let spec = dialog::Spec::new(
            "about",
            tr(lang, "about.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "about.subtitle"))
        .icon(glyph::APP, "S")
        .width(500.0);
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    if let Some(img) = logo.clone() {
                        ui.add(img);
                    } else {
                        ui.label(crate::icons::semibold("SuniPDF", 26.0).color(p.text));
                    }
                    ui.label(
                        egui::RichText::new(tr_args(
                            lang,
                            "about.version",
                            &[("version", env!("CARGO_PKG_VERSION"))],
                        ))
                        .size(13.0)
                        .color(p.text_muted),
                    );
                    ui.label(egui::RichText::new(tr(lang, "about.description")).size(13.5));
                });
                ui.add_space(10.0);
                egui::Frame::new()
                    .fill(p.chrome)
                    .stroke(egui::Stroke::new(1.0, p.border))
                    .corner_radius(10)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label(egui::RichText::new(tr(lang, "about.privacy")).size(13.0));
                        ui.label(
                            egui::RichText::new(tr(lang, "about.engine"))
                                .size(12.5)
                                .color(p.text_muted),
                        );
                        ui.label(
                            egui::RichText::new(tr(lang, "about.license"))
                                .size(12.5)
                                .color(p.text_muted),
                        );
                    });
                ui.add_space(10.0);
                self.update_controls(ctx, ui);
                ui.checkbox(&mut startup_check, tr(lang, "updates.startup"));
                ui.add_enabled(
                    !self.close_must_wait(),
                    egui::Checkbox::new(&mut autosave, tr(lang, "recovery.autosave")),
                );
                let busy = matches!(
                    self.updater.phase,
                    UpdatePhase::Checking | UpdatePhase::Downloading | UpdatePhase::Ready(_)
                );
                manual_check = ui
                    .add_enabled(!busy, egui::Button::new(tr(lang, "updates.check")))
                    .clicked();
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(crate::icons::semibold(tr(lang, "about.credit"), 13.5).color(p.text));
                    ui.label(egui::RichText::new("-").size(13.5));
                    ui.hyperlink_to(
                        egui::RichText::new(tr(lang, "about.website")).size(13.5),
                        "https://navanem.com/projets/sunipdf/",
                    );
                });
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "about.close"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    close = true;
                }
            },
        );
        if close {
            self.show_about = false;
        }
        if startup_check != self.settings.check_updates {
            self.settings.check_updates = startup_check;
            self.persist_settings();
        }
        if manual_check {
            self.start_update_check(ctx, true);
        }
        if autosave != self.settings.autosave {
            if !autosave {
                if let Some(session) = &self.recovery.session {
                    for tab in &self.tabs.tabs {
                        if let Err(e) = session.slot(tab.recovery_id, tab.path.clone(), 0).remove()
                        {
                            log::warn!("Recovery cleanup failed: {e}");
                            self.error = Some("error.recovery_failed");
                        }
                    }
                }
            }
            self.settings.autosave = autosave;
            self.worker.send(Request::SetRecovery(self.recovery_slot()));
            self.persist_settings();
        }
    }
}

impl eframe::App for SuniApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

impl SuniApp {
    pub(crate) fn render_ui(&mut self, ui: &mut egui::Ui) {
        if self.settings.autosave && self.has_unsaved_changes() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs(5));
        }
        let ctx = ui.ctx().clone();
        if self.updater.poll() {
            ctx.request_repaint();
        }
        self.handle_responses(&ctx);
        self.handle_input(&ctx);
        let p = theme::palette(self.settings.dark_theme);
        let chrome = |fill: egui::Color32| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(6, 4))
        };

        if !self.reading_full_screen {
            egui::Panel::top("tabstrip")
                .frame(chrome(p.chrome))
                .show(ui, |ui| self.tab_strip(&ctx, ui));
            egui::Panel::top("toolbar")
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(egui::Stroke::new(1.0, p.border))
                        .inner_margin(egui::Margin::symmetric(6, 6)),
                )
                .show(ui, |ui| self.toolbar(&ctx, ui));
            if matches!(
                self.updater.phase,
                UpdatePhase::Available(_) | UpdatePhase::Downloading | UpdatePhase::Ready(_)
            ) || (self.update_errors_visible
                && matches!(
                    self.updater.phase,
                    UpdatePhase::Checking | UpdatePhase::Failed(_)
                ))
            {
                egui::Panel::top("update-banner")
                    .frame(chrome(p.surface))
                    .show(ui, |ui| self.update_controls(&ctx, ui));
            }
            if self.document.is_some()
                && self
                    .annotate
                    .workspace
                    .is_some_and(|w| w.is_mode() && w != crate::workspace::Workspace::Measure)
            {
                // Hauteur fixe : pas de saut quand les libellés apparaissent ou disparaissent.
                egui::Panel::top("workspace")
                    .exact_size(48.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.surface)
                            .stroke(egui::Stroke::new(1.0, p.border)),
                    )
                    .show(ui, |ui| self.workspace_bar(&ctx, ui));
            }
            if self.library.placing.is_some() {
                egui::Panel::top("library-placement")
                    .frame(chrome(p.surface))
                    .show(ui, |ui| self.library_placement_bar(ui));
            }
            if self.find.open && self.document.is_some() {
                egui::Panel::top("find")
                    .frame(chrome(p.chrome))
                    .show(ui, |ui| self.find_bar(ui));
            }
            egui::Panel::bottom("status")
                .frame(chrome(p.chrome))
                .show(ui, |ui| self.status_bar(ui));
            if self.document.is_some() {
                egui::Panel::bottom("reader-controls")
                    .frame(chrome(p.surface))
                    .show(ui, |ui| self.reader_bar(&ctx, ui));
                egui::Panel::right("rail")
                    .resizable(false)
                    .exact_size(48.0)
                    .frame(egui::Frame::new().fill(p.surface).inner_margin(4))
                    .show(ui, |ui| self.right_rail(&ctx, ui));
            }
            let context_panel = self.context_panel();
            let reserved_width = if self.navigation_panel().is_some() {
                380.0
            } else {
                220.0
            };
            let context_max_width = (ui.available_width() - reserved_width).clamp(240.0, 380.0);
            self.measurement_dock(&ctx, ui);
            if context_panel == Some(crate::reader_layout::ContextPanel::Comments) {
                egui::Panel::right("comments")
                    .resizable(true)
                    .default_size(280.0_f32.min(context_max_width))
                    .size_range(240.0..=context_max_width)
                    .frame(
                        egui::Frame::new()
                            .fill(p.chrome)
                            .inner_margin(egui::Margin::symmetric(8, 4)),
                    )
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.ink_draft_active()
                                && !self.annotate.measure.is_active_draft()
                                && self.annotate.adv.free_text.is_none(),
                            |ui| self.comments_panel(&ctx, ui),
                        );
                    });
            }
            if context_panel == Some(crate::reader_layout::ContextPanel::Objects) {
                egui::Panel::right("objects")
                    .resizable(true)
                    .default_size(260.0_f32.min(context_max_width))
                    .size_range(240.0..=context_max_width)
                    .frame(
                        egui::Frame::new()
                            .fill(p.chrome)
                            .inner_margin(egui::Margin::symmetric(8, 4)),
                    )
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.ink_draft_active()
                                && !self.annotate.measure.is_active_draft()
                                && self.annotate.adv.free_text.is_none(),
                            |ui| self.objects_panel(&ctx, ui),
                        );
                    });
            }
            if context_panel == Some(crate::reader_layout::ContextPanel::Forms) {
                egui::Panel::right("forms")
                    .resizable(true)
                    .default_size(300.0_f32.min(context_max_width))
                    .size_range(240.0..=context_max_width)
                    .frame(
                        egui::Frame::new()
                            .fill(p.chrome)
                            .inner_margin(egui::Margin::symmetric(8, 4)),
                    )
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.ink_draft_active()
                                && !self.annotate.measure.is_active_draft()
                                && self.annotate.adv.free_text.is_none(),
                            |ui| self.forms_panel(&ctx, ui),
                        );
                    });
            }
            self.navigation_dock(&ctx, ui);
        }
        if self.reading_full_screen {
            egui::Area::new(egui::Id::unique("reading_exit"))
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    if ui.button(self.t("reading.exit")).clicked() {
                        self.set_reading_full_screen(&ctx, false);
                    }
                });
        }
        if self.document.is_some() && self.organize.active {
            let canvas = egui::Frame::new().fill(theme::canvas_bg(self.settings.dark_theme));
            egui::CentralPanel::default().frame(canvas).show(ui, |ui| {
                self.error_banner(ui);
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(egui::Stroke::new(1.0, p.border))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.organize_toolbar(ui)
                    });
                self.organize_grid(&ctx, ui);
            });
        } else if self.document.is_some() {
            let canvas = egui::Frame::new().fill(theme::canvas_bg(self.settings.dark_theme));
            egui::CentralPanel::default().frame(canvas).show(ui, |ui| {
                self.error_banner(ui);
                if self.has_signatures && self.has_unsaved_changes() {
                    ui.colored_label(p.text_muted, self.t("certificate.dirty_warning"));
                }
                if !self.reading_full_screen {
                    ui.add_enabled_ui(
                        !self.ink_draft_active()
                            && !self.annotate.measure.is_active_draft()
                            && self.annotate.adv.free_text.is_none(),
                        |ui| self.forms_bar(&ctx, ui),
                    );
                }
                self.document_area(&ctx, ui);
            });
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.chrome))
                .show(ui, |ui| {
                    self.error_banner(ui);
                    self.home(&ctx, ui);
                });
        }
        // Commandes scriptées : après la première image du dernier fichier ouvert, pour partir du
        // zoom ajusté et de l'ancre de la vue, comme un clic.
        if !self.startup_commands.is_empty()
            && self.document.is_some()
            && self.opening.is_none()
            && self.pending_files.is_empty()
        {
            for command in std::mem::take(&mut self.startup_commands) {
                self.execute(&ctx, command);
            }
            ctx.request_repaint();
        }
        if ctx.input(|i| i.viewport().close_requested())
            && (self.has_unsaved_changes()
                || self.any_background_dirty().is_some()
                || self.close_must_wait())
            && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            // Même logique que Quitter : invite pour le document actif s'il est modifié, sinon
            // reprise de l'onglet modifié en arrière-plan (jamais d'écriture d'un document intact).
            if self.unsaved_prompt.is_none() {
                self.execute(&ctx, Command::Quit);
            }
        }
        self.unsaved_dialog(&ctx);
        self.signed_save_dialog(&ctx);
        self.feature_dialogs(&ctx);
        self.revision_overlay_dialog(&ctx);
        self.autosave_tick();
        self.split_dialog(&ctx);
        self.flatten_dialog(&ctx);
        self.stamp_dialog(&ctx);
        self.sign_dialog(&ctx);
        self.properties_dialog(&ctx);
        self.protect_dialog(&ctx);
        self.unprotect_dialog(&ctx);
        self.redact_dialogs(&ctx);
        self.text_prompt_dialog(&ctx);
        self.paragraph_dialog(&ctx);
        self.rich_annotation_dialog(&ctx);
        self.annotation_library_dialog(&ctx);
        self.saved_measurements_dialog(&ctx);
        self.annotation_register_dialog(&ctx);
        self.quantity_table_dialog(&ctx);
        self.count_marker_dialog(&ctx);
        self.page_setup_dialog(&ctx);
        self.note_dialog(&ctx);
        self.adv_dialogs(&ctx);
        self.password_dialog(&ctx);
        self.about_dialog(&ctx);
        self.shortcut_dialog(&ctx);
        self.update_install_dialog(&ctx);
        self.command_palette(&ctx);
        self.focus_last_frame = ctx.memory(|m| m.focused().is_some());
    }
}

// ----- Utilitaires -----------------------------------------------------------

pub(crate) fn menu_item(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    selected: bool,
) -> bool {
    let clicked = ui
        .add_enabled(
            enabled,
            egui::Button::new(label)
                .shortcut_text(shortcut)
                .selected(selected),
        )
        .clicked();
    if clicked {
        ui.close();
    }
    clicked
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

pub fn error_key(e: &EngineError) -> &'static str {
    match e {
        EngineError::LibraryNotFound(_) => "error.engine_missing",
        EngineError::LibraryLoad(_) => "error.engine_load",
        EngineError::Io(_) => "error.io",
        EngineError::PasswordRequired => "error.password_required",
        EngineError::InvalidFormat => "error.invalid_format",
        EngineError::SecurityRestricted => "error.security",
        EngineError::PageOutOfRange(_) => "error.page_range",
        EngineError::RenderTooLarge => "error.render_too_large",
        EngineError::ImageBatchTooLarge => "error.image_batch_too_large",
        EngineError::AlreadyExists(_) => "error.already_exists",
        EngineError::MissingGlyphs(_) => "error.missing_glyphs",
        EngineError::FontNotEmbeddable => "error.font_not_embeddable",
        EngineError::PasswordNotLatin1 => "error.password_not_latin1",
        EngineError::FormFieldNotFound(_) => "error.form_field_not_found",
        EngineError::FormValueInvalid(_) => "error.form_value_invalid",
        EngineError::FormXfa => "error.form_xfa",
        EngineError::RedactionIncomplete => "error.redact_incomplete",
        EngineError::Cancelled => "error.cancelled",
        EngineError::Other(message) if message == crate::worker::CERTIFICATE_SOURCE_CHANGED => {
            "error.certificate_source_changed"
        }
        EngineError::Other(message) if message == crate::worker::CERTIFICATE_SAVE_FIRST => {
            "error.certificate_save_first"
        }
        EngineError::Other(_) => "error.unknown",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn test_app() -> (
        SuniApp,
        std::sync::mpsc::Receiver<Request>,
        std::sync::mpsc::Sender<Response>,
    ) {
        let (worker, requests, responses) = Worker::test_channels();
        let mut app = SuniApp {
            processing: Default::default(),
            office: Default::default(),
            review: Default::default(),
            overlay: Default::default(),
            reading_full_screen: false,
            recovery: Default::default(),
            printing: Default::default(),
            ocr: Default::default(),
            form_builder: Default::default(),
            certificate: Default::default(),
            has_signatures: false,
            signed_save_confirm: None,
            settings: Settings::default(),
            worker,
            engine: EngineState::Ready,
            document: None,
            opening: None,
            zoom: Zoom::FitWidth,
            effective_zoom: 1.0,
            view: crate::view::ViewState::new(true),
            page_input: "1".into(),
            error: None,
            password_prompt: None,
            show_about: false,
            shortcuts: Default::default(),
            pending_files: vec![],
            thumbnails: HashMap::new(),
            thumbs_pending: HashSet::new(),
            thumbs_failed: HashSet::new(),
            thumb_window: 0..0,
            thumb_px: 0,
            find: FindState::default(),
            dirty: false,
            saving: false,
            allow_close: false,
            pending_save: None,
            saving_drafts: HashMap::new(),
            deferred_close: None,
            install_on_quit: false,
            updater: Updater::default(),
            update_errors_visible: false,
            install_confirm: false,
            unsaved_prompt: None,
            after_save: None,
            status_msg: None,
            has_icons: false,
            brand: None,
            after_open: None,
            show_tools: false,
            focus_last_frame: false,
            palette: PaletteState {
                open: false,
                query: String::new(),
                selected: 0,
                focus_requested: false,
            },
            organize: Default::default(),
            annotate: Default::default(),
            docinfo: Default::default(),
            tools: Default::default(),
            tabs: Default::default(),
            edit: Default::default(),
            pagesetup: Default::default(),
            sign: Default::default(),
            library: Default::default(),
            forms: Default::default(),
            can_undo: false,
            can_redo: false,
            startup_commands: vec![],
        };
        app.document = Some(OpenDocument {
            info: test_info("test/first.pdf"),
            page: 0,
        });
        app.register_opened_tab(PathBuf::from("test/first.pdf"));
        (app, requests, responses)
    }

    fn test_info(path: &str) -> DocumentInfo {
        DocumentInfo {
            path: PathBuf::from(path),
            title: None,
            page_labels: vec![],
            page_sizes: vec![sunipdf_core::PageSize {
                width_pt: 612.0,
                height_pt: 792.0,
            }],
            form: sunipdf_core::FormKind::AcroForm,
        }
    }

    #[test]
    fn form_builder_guards_open_draw_and_create_against_busy_or_unsaved_state() {
        let ctx = egui::Context::default();
        for blocked in 0..15 {
            let (mut app, requests, _) = test_app();
            app.execute(&ctx, Command::ToolAddTextField);
            assert!(app.form_builder.visible);
            app.form_builder.start_drawing();
            match blocked {
                0 => app.saving = true,
                1 => app.opening = Some(PathBuf::from("pending.pdf")),
                2 => app.tabs.suspending = Some(0),
                3 => app.tabs.resuming = Some(0),
                4 => app.worker.send(Request::Edit(EditOp::Rotate {
                    pages: vec![0],
                    delta_degrees: 90,
                })),
                5 => {
                    app.forms
                        .drafts
                        .insert("Existing".into(), "unsaved input".into());
                }
                6 => {
                    app.annotate.tool = crate::annotate::Tool::Ink;
                    app.adv_click(0, (20.0, 30.0));
                }
                7 => app.processing.busy = true,
                8 => app.sign.busy = true,
                9 => app.printing.busy = true,
                10 => app.ocr.busy = true,
                11 => app.forms.pending_clear = true,
                12 => app.forms.pending_import = Some(serde_json::json!({"Existing": "queued"})),
                13 => app.pending_files.push(PathBuf::from("queued.pdf")),
                14 => app.pending_save = Some(PathBuf::from("queued-save.pdf")),
                _ => unreachable!(),
            }
            assert!(!app.form_builder_ready(), "case {blocked}");
            app.validate_form_builder();
            assert!(!app.form_builder.active());
            for command in [
                Command::ToolAddTextField,
                Command::ToolAddCheckbox,
                Command::ToolAddRadioField,
                Command::ToolAddComboField,
                Command::ToolAddListField,
            ] {
                app.execute(&ctx, command);
                assert!(
                    !app.form_builder.active(),
                    "case {blocked}, command {command:?}"
                );
            }
            let edits = requests
                .try_iter()
                .filter(|r| matches!(r, Request::Edit(_)))
                .count();
            assert_eq!(edits, usize::from(blocked == 4));
        }
    }

    #[test]
    fn form_builder_cannot_survive_revision_tabs_close_tool_workspace_or_escape() {
        let ctx = egui::Context::default();
        for change in 0..8 {
            let (mut app, requests, _) = test_app();
            app.execute(&ctx, Command::ToolAddComboField);
            app.form_builder.start_drawing();
            let size = app.document.as_ref().unwrap().info.page_sizes[0];
            app.form_builder.start_drag(0, (30.0, 40.0), size);
            match change {
                0 => app.view.bump_epoch(),
                1 => app.annotate.tool = crate::annotate::Tool::AddText,
                2 => app.annotate.workspace = Some(crate::workspace::Workspace::Edit),
                3 => {
                    app.take_tab();
                }
                4 => app.close_document(false),
                5 => app.save_document(false),
                6 => app.leave_workspace(),
                7 => {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events: vec![egui::Event::Key {
                                key: Key::Escape,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: Modifiers::NONE,
                            }],
                            ..Default::default()
                        },
                        |_| app.handle_input(&ctx),
                    );
                    output.textures_delta.clear();
                }
                _ => unreachable!(),
            }
            app.validate_form_builder();
            assert!(
                !app.form_builder.active() && app.form_builder.drag.is_none(),
                "change {change}"
            );
            assert!(
                !requests.try_iter().any(|r| matches!(r, Request::Edit(_))),
                "cancel never edits"
            );
        }
        let (mut app, requests, _) = test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes.push(size);
        app.execute(&ctx, Command::ToolAddListField);
        app.form_builder.start_drawing();
        app.execute(&ctx, Command::ZoomIn);
        app.execute(&ctx, Command::NextPage);
        app.execute(&ctx, Command::ViewSinglePage);
        app.validate_form_builder();
        assert!(
            app.form_builder.drawing,
            "zoom and page navigation keep the draft"
        );
        assert!(!requests.try_iter().any(|r| matches!(r, Request::Edit(_))));
    }

    #[test]
    fn unfinished_ink_guards_commands_tabs_files_workspace_and_native_close_then_escape_cancels() {
        for language in Language::ALL {
            let (mut app, requests, _) = test_app();
            app.settings.set_language(language);
            let size = app.document.as_ref().unwrap().info.page_sizes[0];
            app.document.as_mut().unwrap().info.page_sizes = vec![size; 2];
            let ctx = egui::Context::default();
            app.annotate.tool = crate::annotate::Tool::Ink;
            app.adv_click(0, (20.0, 30.0));
            assert!(app.has_unsaved_changes() && app.close_must_wait());
            for entry in sunipdf_core::command::REGISTRY {
                if app.ink_blocks_command(entry.command) {
                    assert!(!app.command_enabled(entry.command));
                    app.execute(&ctx, entry.command);
                    assert!(app.ink_draft_active() && !app.allow_close);
                }
            }
            app.save_document(true);
            app.open_path(PathBuf::from("test/replacement.pdf"), None);
            app.switch_to(1);
            assert!(!app.suspend_active());
            app.enter_workspace(&ctx, crate::workspace::Workspace::Edit);
            app.leave_workspace();
            app.rotate_reading_view(true);
            app.set_reading_full_screen(&ctx, true);
            assert!(requests.try_recv().is_err());
            assert!(
                app.opening.is_none()
                    && app.unsaved_prompt.is_none()
                    && app.deferred_close.is_none()
            );
            assert_eq!(app.annotate.tool, crate::annotate::Tool::Ink);
            assert_eq!(app.status_msg.as_deref(), Some(app.t("ink.finish_first")));
            app.execute(&ctx, Command::NextPage);
            app.execute(&ctx, Command::ZoomIn);
            assert_eq!(app.document.as_ref().unwrap().page, 1);
            assert_eq!(app.annotate.adv.ink_draft.as_ref().unwrap().page, 0);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |_| app.handle_input(&ctx),
            );
            output.textures_delta.clear();
            assert!(!app.ink_draft_active() && !app.dirty && !app.has_unsaved_changes());
            assert!(!app.close_must_wait());
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn ink_release_after_single_page_navigation_keeps_the_captured_stroke() {
        let (mut app, requests, _) = test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 2];
        app.view.mode = ViewMode::Single;
        app.annotate.tool = crate::annotate::Tool::Ink;
        let drag = crate::annotate::Drag {
            page: 0,
            start: (20.0, 30.0),
            current: (40.0, 50.0),
        };
        app.adv_drag_start(drag.page, drag.start);
        app.adv_drag_moved(&drag);
        app.annotate.drag = Some(drag);
        let ctx = egui::Context::default();
        for pressed in [true, false] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::PointerButton {
                        pos: egui::pos2(10.0, 10.0),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |_| {
                    app.go_to_page(1);
                    app.handle_input(&ctx);
                    assert_eq!(app.annotate.adv.ink.is_empty(), !pressed);
                },
            );
            output.textures_delta.clear();
        }
        let draft = app.annotate.adv.ink_draft.as_ref().unwrap();
        assert_eq!(draft.page, 0);
        assert_eq!(draft.strokes, vec![vec![(20.0, 30.0), (40.0, 50.0)]]);
        assert!(app.annotate.drag.is_none());
        app.adv_click(0, (60.0, 70.0));
        assert_eq!(
            app.annotate.adv.ink_draft.as_ref().unwrap().strokes.len(),
            2
        );
        assert!(app.annotate.adv.ink.is_empty() && requests.try_recv().is_err());
    }

    #[test]
    fn ink_draft_starts_only_when_document_and_worker_are_idle() {
        for blocked in 0..11 {
            let (mut app, requests, _) = test_app();
            app.annotate.tool = crate::annotate::Tool::Ink;
            match blocked {
                0 => app.worker.send(Request::Edit(EditOp::Rotate {
                    pages: vec![0],
                    delta_degrees: 90,
                })),
                1 => app.opening = Some(PathBuf::from("test/other.pdf")),
                2 => app.saving = true,
                3 => app.show_about = true,
                4 => app.tabs.suspending = Some(0),
                5 => app.tabs.resuming = Some(0),
                6 => app.pending_save = Some(PathBuf::from("test/save.pdf")),
                7 => {
                    app.forms
                        .drafts
                        .insert("field".into(), "uncommitted".into());
                }
                8 => app.forms.pending_clear = true,
                9 => app.forms.pending_import = Some(serde_json::json!({"field":"value"})),
                _ => app.worker.send(Request::MarkRecovered {
                    path: PathBuf::from("test/recovered.pdf"),
                }),
            }
            requests.try_iter().for_each(drop);
            app.adv_click(0, (20.0, 30.0));
            assert!(!app.ink_draft_active(), "busy case {blocked}");
            assert!(requests.try_recv().is_err());
            assert_eq!(app.status_msg.as_deref(), Some(app.t("ink.wait")));
        }
    }

    #[test]
    fn labelled_navigation_rejects_ambiguity_and_preserves_each_tab_input() {
        let (mut app, requests, _) = test_app();
        let doc = app.document.as_mut().unwrap();
        doc.info.page_sizes = vec![doc.info.page_sizes[0]; 5];
        doc.info.page_labels = ["i", "ii", "1", "Repeat", "Repeat"]
            .into_iter()
            .map(|label| Some(label.to_owned()))
            .collect();
        for (input, page, field) in [("ii", 1, "ii"), ("1", 2, "1"), ("#1", 0, "i")] {
            app.page_input = input.to_owned();
            app.submit_page_input();
            assert_eq!(app.document.as_ref().unwrap().page, page);
            assert_eq!(app.page_input, field);
        }
        for input in ["Repeat", "unknown", "#6", "0"] {
            app.page_input = input.to_owned();
            app.submit_page_input();
            assert_eq!(app.document.as_ref().unwrap().page, 0);
            assert_eq!(app.page_input, "i");
            assert_eq!(
                app.status_msg.as_deref(),
                Some(app.t("reading.page_label_invalid"))
            );
        }
        app.page_input = "ii".to_owned();
        app.submit_page_input();
        let first = app.take_tab();
        app.document = Some(OpenDocument {
            info: test_info("test/second.pdf"),
            page: 0,
        });
        app.page_input = "1".to_owned();
        let second = app.take_tab();
        app.put_tab(first);
        assert_eq!(app.page_input, "ii");
        assert_eq!(
            app.document.as_ref().unwrap().info.page_label(1),
            Some("ii")
        );
        app.put_tab(second);
        assert_eq!(app.page_input, "1");
        assert_eq!(app.document.as_ref().unwrap().info.page_label(0), None);
        assert!(!app.dirty);
        assert!(
            requests.try_iter().next().is_none(),
            "reading labels must not mutate the worker document"
        );
    }

    fn organize_frame(app: &mut SuniApp, ctx: &egui::Context, offset: f32) {
        for scroll in [true, false] {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(egui::pos2(450.0, 300.0))],
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 600.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let id = ui.make_persistent_id(egui::IdSalt::new("scroll_area"));
                    let current = egui::scroll_area::State::load(ctx, id)
                        .unwrap_or_default()
                        .clamped_offset()
                        .y;
                    ui.input_mut(|input| {
                        input.smooth_scroll_delta.y = if scroll { current - offset } else { 0.0 }
                    });
                    app.organize_grid(ctx, ui);
                },
            );
            out.textures_delta.clear();
        }
    }

    fn sidebar_frame(app: &mut SuniApp, ctx: &egui::Context) {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| app.thumbnails_panel(ctx, ui),
        );
        out.textures_delta.clear();
    }

    fn thumbnail_jobs(requests: &std::sync::mpsc::Receiver<Request>) -> Vec<(usize, u32)> {
        requests
            .try_iter()
            .filter_map(|request| match request {
                Request::Thumbnail { page, width_px } => Some((page, width_px)),
                _ => None,
            })
            .collect()
    }

    fn deliver_thumbnails(responses: &std::sync::mpsc::Sender<Response>, jobs: &[(usize, u32)]) {
        for &(page, width) in jobs {
            responses
                .send(Response::Thumbnail {
                    page,
                    image: std::sync::Arc::new(sunipdf_core::RenderedPage {
                        page_index: page,
                        width,
                        height: 1,
                        rgba: vec![255; width as usize * 4],
                    }),
                })
                .unwrap();
        }
    }

    #[test]
    fn organize_browsing_large_document_keeps_thumbnail_textures_bounded() {
        let (mut app, requests, responses) = test_app();
        let sizes = &mut app.document.as_mut().unwrap().info.page_sizes;
        sizes.resize(5000, sizes[0]);
        app.organize.active = true;
        let ctx = egui::Context::default();
        let mut furthest_page = 0;
        for step in 0..80 {
            organize_frame(&mut app, &ctx, step as f32 * 1500.0);
            let jobs = thumbnail_jobs(&requests);
            assert!(
                jobs.len() < 100,
                "only virtualized rows should request thumbnails"
            );
            furthest_page = furthest_page.max(jobs.iter().map(|job| job.0).max().unwrap_or(0));
            deliver_thumbnails(&responses, &jobs);
            app.handle_responses(&ctx);
            assert!(
                app.thumbnails.len() <= crate::view::THUMB_KEEP,
                "scrolling must not retain every previously visited page thumbnail"
            );
            assert!(jobs.iter().all(|job| app.thumbnails.contains_key(&job.0)));
        }
        assert!(
            furthest_page > 1000,
            "the test must browse beyond the cache window"
        );
        assert_eq!(app.document.as_ref().unwrap().page, 0);
    }

    #[test]
    fn organize_fast_scroll_keeps_pending_thumbnail_requests_bounded() {
        let (mut app, requests, responses) = test_app();
        let sizes = &mut app.document.as_mut().unwrap().info.page_sizes;
        sizes.resize(5000, sizes[0]);
        app.organize.active = true;
        let ctx = egui::Context::default();
        let mut jobs = Vec::new();
        for step in 0..80 {
            organize_frame(&mut app, &ctx, step as f32 * 1500.0);
            jobs.extend(thumbnail_jobs(&requests));
        }
        assert!(
            jobs.len() <= crate::view::THUMB_KEEP,
            "fast scrolling must not enqueue the whole document before the worker responds"
        );
        assert!(app.thumbs_pending.len() <= crate::view::THUMB_KEEP);
        deliver_thumbnails(&responses, &jobs);
        app.handle_responses(&ctx);
        assert!(app.thumbs_pending.is_empty());
        organize_frame(&mut app, &ctx, 118_500.0);
        let visible_jobs = thumbnail_jobs(&requests);
        assert!(!visible_jobs.is_empty());
        assert!(visible_jobs.iter().all(|job| job.0 > 1000));
    }

    #[test]
    fn organize_discards_thumbnail_responses_from_a_previous_scroll_window() {
        let (mut app, requests, responses) = test_app();
        let sizes = &mut app.document.as_mut().unwrap().info.page_sizes;
        sizes.resize(5000, sizes[0]);
        app.organize.active = true;
        let ctx = egui::Context::default();
        organize_frame(&mut app, &ctx, 0.0);
        let old_jobs = thumbnail_jobs(&requests);
        assert!(!old_jobs.is_empty());
        organize_frame(&mut app, &ctx, 100_000.0);
        let visible_jobs = thumbnail_jobs(&requests);
        assert!(!visible_jobs.is_empty());
        assert!(visible_jobs.iter().all(|job| job.0 > 1000));
        deliver_thumbnails(&responses, &old_jobs);
        app.handle_responses(&ctx);
        assert!(
            app.thumbnails.is_empty(),
            "late responses outside the current cache window must not allocate textures"
        );
        deliver_thumbnails(&responses, &visible_jobs);
        app.handle_responses(&ctx);
        assert!(!visible_jobs.is_empty());
        assert!(visible_jobs
            .iter()
            .all(|job| app.thumbnails.contains_key(&job.0)));
    }

    #[test]
    fn organize_failed_thumbnails_release_quota_without_retries_each_frame() {
        let (mut app, requests, responses) = test_app();
        let sizes = &mut app.document.as_mut().unwrap().info.page_sizes;
        sizes.resize(5000, sizes[0]);
        app.organize.active = true;
        let ctx = egui::Context::default();
        let mut jobs = Vec::new();
        for step in 0..80 {
            organize_frame(&mut app, &ctx, step as f32 * 1500.0);
            jobs.extend(thumbnail_jobs(&requests));
        }
        assert_eq!(jobs.len(), crate::view::THUMB_KEEP);
        for (page, width_px) in jobs {
            responses
                .send(Response::ThumbnailFailed { page, width_px })
                .unwrap();
        }
        app.handle_responses(&ctx);
        assert!(app.thumbs_pending.is_empty());
        organize_frame(&mut app, &ctx, 118_500.0);
        let visible_jobs = thumbnail_jobs(&requests);
        assert!(!visible_jobs.is_empty());
        assert!(visible_jobs.iter().all(|job| job.0 > 1000));
        for (page, width_px) in visible_jobs {
            responses
                .send(Response::ThumbnailFailed { page, width_px })
                .unwrap();
        }
        app.handle_responses(&ctx);
        assert!(app.thumbs_pending.is_empty());
        assert!(!app.thumbs_failed.is_empty());
        for _ in 0..5 {
            organize_frame(&mut app, &ctx, 118_500.0);
            assert!(
                thumbnail_jobs(&requests).is_empty(),
                "failed pages must not spin on every frame"
            );
        }
        app.go_to_page(4999);
        assert_eq!(app.document.as_ref().unwrap().page, 4999);
        organize_frame(&mut app, &ctx, 0.0);
        assert!(app.thumbs_failed.is_empty());
        organize_frame(&mut app, &ctx, 118_500.0);
        assert!(
            !thumbnail_jobs(&requests).is_empty(),
            "leaving the failed window should allow an explicit revisit"
        );
    }

    #[test]
    fn organize_sidebar_width_changes_bound_requests_and_tab_reset_allows_retry() {
        let (mut app, requests, responses) = test_app();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        organize_frame(&mut app, &ctx, 0.0);
        let old_jobs = thumbnail_jobs(&requests);
        assert_eq!(old_jobs.len(), 1);
        let old_width = old_jobs[0].1;
        for _ in 0..20 {
            sidebar_frame(&mut app, &ctx);
            organize_frame(&mut app, &ctx, 0.0);
        }
        sidebar_frame(&mut app, &ctx);
        assert_ne!(app.thumb_px, old_width);
        assert!(
            thumbnail_jobs(&requests).is_empty(),
            "the same page must wait for its old-width result"
        );
        deliver_thumbnails(&responses, &old_jobs);
        app.handle_responses(&ctx);
        assert!(app.thumbnails.is_empty());
        assert!(app.thumbs_pending.is_empty());
        sidebar_frame(&mut app, &ctx);
        let jobs = thumbnail_jobs(&requests);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].1, app.thumb_px);
        responses
            .send(Response::ThumbnailFailed {
                page: jobs[0].0,
                width_px: jobs[0].1,
            })
            .unwrap();
        app.handle_responses(&ctx);
        sidebar_frame(&mut app, &ctx);
        assert!(thumbnail_jobs(&requests).is_empty());
        let tab = app.take_tab();
        app.put_tab(tab);
        sidebar_frame(&mut app, &ctx);
        let jobs = thumbnail_jobs(&requests);
        assert_eq!(
            jobs.len(),
            1,
            "a resumed document must retry its previous thumbnail failure"
        );
        deliver_thumbnails(&responses, &jobs);
        app.handle_responses(&ctx);
        assert!(app.thumbnails.contains_key(&0));
        app.execute(&ctx, Command::CloseDocument);
        assert!(app.thumbnails.is_empty());
        assert!(app.thumbs_pending.is_empty());
        assert!(app.thumbs_failed.is_empty());
    }

    fn cancel_unsaved_prompt(app: &mut SuniApp, ctx: &egui::Context) {
        crate::icons::install(ctx);
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| app.unsaved_dialog(ctx),
        );
        out.textures_delta.clear();
    }

    #[test]
    fn cancelling_quit_disarms_update_installation_and_keeps_the_document() {
        let (mut app, _requests, _responses) = test_app();
        app.dirty = true;
        app.install_on_quit = true;
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::Quit);
        cancel_unsaved_prompt(&mut app, &ctx);
        assert!(
            !app.install_on_quit,
            "Cancel must disarm installation on a later Quit"
        );
        assert!(app.document.is_some());
        assert!(!app.allow_close);
        app.dirty = false;
        let mut out = ctx.run_ui(Default::default(), |_| app.execute(&ctx, Command::Quit));
        out.textures_delta.clear();
        assert!(
            out.viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::Close)),
            "a later normal Quit must not try to install the cancelled update"
        );
    }

    #[test]
    fn save_failure_disarms_update_installation() {
        let (mut app, requests, responses) = test_app();
        app.dirty = true;
        app.install_on_quit = true;
        app.after_save = Some(PendingAction::Quit);
        app.save_document(false);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        responses
            .send(Response::SaveFailed(EngineError::InvalidFormat))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(
            !app.install_on_quit,
            "failed Save must cancel the installation intent"
        );
        assert!(app.document.is_some());
        assert!(!app.allow_close);
    }

    #[test]
    fn invalid_form_draft_disarms_update_installation() {
        let (mut app, requests, _responses) = test_app();
        form_draft(&mut app, "Too long to fit");
        app.install_on_quit = true;
        app.after_save = Some(PendingAction::Quit);
        app.save_document(false);
        assert!(
            !app.install_on_quit,
            "invalid draft must cancel the installation intent"
        );
        assert!(!requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        assert!(app.document.is_some());
    }

    #[test]
    fn update_installation_waits_for_pending_edit_and_unsaved_confirmation() {
        let (mut app, _requests, responses) = test_app();
        app.install_on_quit = true;
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::RotateClockwise);
        app.execute(&ctx, Command::Quit);
        assert!(!app.allow_close);
        assert!(app.unsaved_prompt.is_none());
        responses.send(changed("test/first.pdf", true)).unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(matches!(app.unsaved_prompt, Some(PendingAction::Quit)));
        assert!(app.install_on_quit);
        cancel_unsaved_prompt(&mut app, &ctx);
        assert!(!app.install_on_quit);
        assert!(app.document.is_some());
    }

    #[test]
    fn startup_update_check_obeys_preference_and_manual_check_can_override_it() {
        let (mut app, _requests, _responses) = test_app();
        let (updater, checks) = Updater::test_channels();
        app.updater = updater;
        let ctx = egui::Context::default();
        app.settings.check_updates = false;
        app.start_update_check(&ctx, false);
        assert!(
            checks.try_recv().is_err(),
            "disabled startup check must not send a request"
        );
        app.settings.check_updates = true;
        app.start_update_check(&ctx, false);
        assert!(checks.try_recv().is_ok());
        assert!(!app.update_errors_visible, "startup failures stay quiet");
        app.updater.phase = UpdatePhase::Idle;
        app.settings.check_updates = false;
        app.start_update_check(&ctx, true);
        assert!(checks.try_recv().is_ok());
        assert!(app.update_errors_visible);
    }

    #[test]
    fn installation_launch_failure_keeps_the_application_open_and_disarms_retry() {
        let (mut app, _requests, _responses) = test_app();
        app.install_on_quit = true;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |_| app.execute(&ctx, Command::Quit));
        out.textures_delta.clear();
        assert!(!out
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::Close)));
        assert!(!app.allow_close);
        assert!(!app.install_on_quit);
        assert!(app.document.is_some());
        assert_eq!(app.error, Some("updates.install_failed"));
    }

    #[test]
    fn closing_a_tab_instead_of_pending_update_quit_disarms_installation() {
        let (mut app, _requests, _responses) = test_app();
        app.install_on_quit = true;
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::RotateClockwise);
        app.execute(&ctx, Command::Quit);
        app.execute(&ctx, Command::CloseDocument);
        assert!(
            !app.install_on_quit,
            "replacing Quit with a tab close must not install on a later Quit"
        );
        assert!(matches!(app.deferred_close, Some(PendingAction::Close)));
        assert!(app.document.is_some());
    }

    #[test]
    fn clicking_active_tab_cross_closes_the_document() {
        let (mut app, requests, _responses) = test_app();
        let ctx = egui::Context::default();
        click_tab_control(&mut app, &ctx, "✕", 0);
        assert!(
            requests.try_iter().any(|r| matches!(r, Request::Close)),
            "the tab cross must deliver the close action"
        );
        assert!(app.document.is_none());
        assert!(app.tabs.tabs.is_empty());
    }

    fn click_tab_control(app: &mut SuniApp, ctx: &egui::Context, label: &str, index: usize) {
        crate::icons::install(ctx);
        ctx.enable_accesskit();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 780.0),
            )),
            ..Default::default()
        };
        let mut target = None;
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |ui| app.tab_strip(ctx, ui));
            if let Some(tree) = &out.platform_output.accesskit_update {
                let mut bounds: Vec<_> = tree
                    .nodes
                    .iter()
                    .map(|(_, node)| node)
                    .filter(|node| node.label() == Some(label))
                    .filter_map(|node| node.bounds())
                    .collect();
                bounds.sort_by(|a, b| a.x0.total_cmp(&b.x0));
                if let Some(bounds) = bounds.get(index) {
                    target = Some(egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.0) as f32,
                        ((bounds.y0 + bounds.y1) / 2.0) as f32,
                    ));
                }
            }
            out.textures_delta.clear();
        }
        let pos = target.expect("tab close button bounds");
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
            let mut out = ctx.run_ui(input, |ui| app.tab_strip(ctx, ui));
            out.textures_delta.clear();
        }
    }

    #[test]
    fn clicking_inactive_tab_cross_closes_only_that_tab_or_prompts_for_its_changes() {
        for dirty in [false, true] {
            let (mut app, requests, responses) = test_app();
            add_background_tab(&mut app, dirty);
            let ctx = egui::Context::default();
            click_tab_control(&mut app, &ctx, "✕", 1);
            responses
                .send(Response::Suspended(crate::worker::SuspendedDoc {
                    signature_source: None,
                    bytes: vec![],
                    password: None,
                    path: PathBuf::from("test/first.pdf"),
                    undo: vec![],
                    redo: vec![],
                    save_point: Some(0),
                    modified: false,
                }))
                .unwrap();
            app.handle_responses(&ctx);
            responses
                .send(Response::Resumed(DocState {
                    info: test_info("test/second.pdf"),
                    can_undo: dirty,
                    can_redo: false,
                    modified: dirty,
                    pages_unchanged: false,
                    measurement_scales_unchanged: false,
                    has_signatures: false,
                }))
                .unwrap();
            app.handle_responses(&ctx);
            if dirty {
                assert!(
                    matches!(app.unsaved_prompt, Some(PendingAction::Close)),
                    "the inactive tab cross must confirm its own unsaved changes"
                );
                assert!(!requests.try_iter().any(|r| matches!(r, Request::Close)));
            } else {
                assert!(
                    requests.try_iter().any(|r| matches!(r, Request::Close)),
                    "the inactive tab cross must close after resuming the target tab"
                );
                assert_eq!(app.tabs.tabs.len(), 1);
            }
            assert_eq!(app.tabs.tabs[0].path, PathBuf::from("test/first.pdf"));
        }
    }

    fn add_background_tab(app: &mut SuniApp, dirty: bool) {
        let mut second_ui = app.take_tab();
        second_ui.document.as_mut().unwrap().info = test_info("test/second.pdf");
        second_ui.dirty = dirty;
        app.document = Some(OpenDocument {
            info: test_info("test/first.pdf"),
            page: 0,
        });
        app.tabs.tabs.push(crate::tabs::Tab {
            recovery_id: crate::recovery::next_id(),
            path: PathBuf::from("test/second.pdf"),
            dirty,
            ui: Some(second_ui),
            doc: Some(crate::worker::SuspendedDoc {
                signature_source: None,
                bytes: vec![],
                password: None,
                path: PathBuf::from("test/second.pdf"),
                undo: vec![],
                redo: vec![],
                save_point: Some(0),
                modified: dirty,
            }),
        });
    }

    #[test]
    fn clicking_tab_name_still_switches_to_that_document() {
        let (mut app, requests, _responses) = test_app();
        add_background_tab(&mut app, false);
        let ctx = egui::Context::default();
        let label = file_name(Path::new("test/second.pdf"));
        click_tab_control(&mut app, &ctx, &label, 0);
        assert_eq!(app.tabs.resuming, Some(1));
        assert!(requests.try_iter().any(|r| matches!(r, Request::Suspend)));
    }

    #[test]
    fn clicking_inactive_tab_cross_during_save_does_not_close_active_document() {
        let (mut app, requests, responses) = test_app();
        add_background_tab(&mut app, false);
        app.dirty = true;
        let ctx = egui::Context::default();
        app.save_document(false);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        click_tab_control(&mut app, &ctx, "✕", 1);
        responses
            .send(Response::Saved {
                path: PathBuf::from("test/first.pdf"),
            })
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, Request::Close | Request::Suspend | Request::Resume(_))),
            "a refused tab switch must never close the active document instead"
        );
        assert_eq!(app.tabs.active, Some(0));
        assert_eq!(
            app.document.as_ref().unwrap().info.path,
            PathBuf::from("test/first.pdf")
        );
    }

    pub(crate) fn form_draft(app: &mut SuniApp, text: &str) {
        app.forms.info = Some(sunipdf_core::FormInfo {
            kind: sunipdf_core::FormKind::AcroForm,
            has_scripts: false,
            fields: vec![sunipdf_core::FormField {
                name: "Name".into(),
                kind: sunipdf_core::FieldKind::Text,
                value: sunipdf_core::FieldValue::Text("Old".into()),
                tooltip: None,
                read_only: false,
                required: false,
                multiline: false,
                password: false,
                comb: false,
                max_len: Some(10),
                multi_select: false,
                editable: false,
                no_toggle_off: false,
                options: vec![],
                widgets: vec![],
            }],
        });
        app.forms.drafts.insert("Name".into(), text.into());
    }

    #[test]
    fn panel_draft_ctrl_s_sends_new_value_before_save() {
        let (mut app, requests, responses) = test_app();
        form_draft(&mut app, "New");
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
        out.textures_delta.clear();
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::S,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::COMMAND,
                }],
                ..Default::default()
            },
            |_| app.handle_input(&ctx),
        );
        out.textures_delta.clear();
        let first = requests.try_recv().expect("form value before save");
        assert!(matches!(first, Request::Edit(EditOp::FillForm { values })
            if values == vec![("Name".into(), sunipdf_core::FieldValue::Text("New".into()))]));
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, Request::Save { .. })),
            "save waits for the form edit"
        );
        responses.send(changed("test/first.pdf", true)).unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
    }

    #[test]
    fn invalid_panel_draft_blocks_save_without_losing_text() {
        let (mut app, requests, _) = test_app();
        form_draft(&mut app, "Too long to fit");
        app.save_document(false);
        assert!(
            requests.try_recv().is_err(),
            "invalid draft must not save old value"
        );
        assert_eq!(
            app.forms.drafts.get("Name").map(String::as_str),
            Some("Too long to fit")
        );
        assert_eq!(app.error, Some("error.form_value_invalid"));
    }

    #[test]
    fn closing_panel_draft_keeps_document_for_unsaved_prompt() {
        let (mut app, requests, _) = test_app();
        form_draft(&mut app, "New");
        app.execute(&egui::Context::default(), Command::CloseDocument);
        assert!(app.document.is_some(), "typed value must survive closing");
        assert!(matches!(app.unsaved_prompt, Some(PendingAction::Close)));
        assert!(!requests.try_iter().any(|r| matches!(r, Request::Close)));
    }

    #[test]
    fn closing_waits_for_requested_rotation() {
        let (mut app, requests, responses) = test_app();
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::RotateClockwise);
        app.execute(&ctx, Command::CloseDocument);
        assert!(matches!(
            requests.try_recv(),
            Ok(Request::Edit(EditOp::Rotate { .. }))
        ));
        assert!(
            !requests.try_iter().any(|r| matches!(r, Request::Close)),
            "rotation has not finished"
        );
        assert!(app.document.is_some());
        responses.send(changed("test/first.pdf", true)).unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(matches!(app.unsaved_prompt, Some(PendingAction::Close)));
        assert!(app.document.is_some());
    }

    #[test]
    fn quitting_waits_for_requested_rotation() {
        let (mut app, _requests, _responses) = test_app();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |_| {
            app.execute(&ctx, Command::RotateClockwise);
            app.execute(&ctx, Command::Quit);
        });
        out.textures_delta.clear();
        assert!(
            !out.viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::Close)),
            "quitting must wait for the rotation result"
        );
    }

    #[test]
    fn old_selection_during_tab_switch_cannot_delete_from_resumed_document() {
        for multiple in [false, true] {
            let (mut app, requests, responses) = test_app();
            let mut second_ui = app.take_tab();
            second_ui.document.as_mut().unwrap().info = test_info("test/second.pdf");
            app.document = Some(OpenDocument {
                info: test_info("test/first.pdf"),
                page: 0,
            });
            app.tabs.tabs.push(crate::tabs::Tab {
                recovery_id: crate::recovery::next_id(),
                path: PathBuf::from("test/second.pdf"),
                dirty: false,
                ui: Some(second_ui),
                doc: Some(crate::worker::SuspendedDoc {
                    signature_source: None,
                    bytes: vec![],
                    password: None,
                    path: PathBuf::from("test/second.pdf"),
                    undo: vec![],
                    redo: vec![],
                    save_point: Some(0),
                    modified: false,
                }),
            });
            let ctx = egui::Context::default();
            app.switch_to(1);
            let object = sunipdf_core::ObjectInfo {
                index: 0,
                kind: sunipdf_core::ObjectKind::Text,
                bounds: sunipdf_core::PointRect {
                    left: 10.0,
                    bottom: 10.0,
                    right: 100.0,
                    top: 30.0,
                },
                quad: [(10.0, 10.0), (100.0, 10.0), (100.0, 30.0), (10.0, 30.0)],
                angle_deg: 0.0,
                opacity: Some(1.0),
                text: Some("First document".into()),
                font_size: 12.0,
                font_name: None,
                fill_rgb: Some((0, 0, 0)),
                stroke_rgb: None,
                stroke_width: 0.0,
            };
            responses
                .send(if multiple {
                    Response::ObjectsSelected {
                        page: 0,
                        infos: vec![object],
                        mode: crate::worker::PickMode::Replace,
                    }
                } else {
                    Response::ObjectSelected {
                        page: 0,
                        info: Some(object),
                    }
                })
                .unwrap();
            app.handle_responses(&ctx);
            responses
                .send(Response::Resumed(DocState {
                    info: test_info("test/second.pdf"),
                    can_undo: false,
                    can_redo: false,
                    modified: false,
                    pages_unchanged: false,
                    measurement_scales_unchanged: false,
                    has_signatures: false,
                }))
                .unwrap();
            app.handle_responses(&ctx);
            app.execute(&ctx, Command::DeleteObject);
            assert!(
                !requests
                    .try_iter()
                    .any(|r| matches!(r, Request::Edit(EditOp::DeleteObjects { .. }))),
                "old object indices must not edit second.pdf"
            );
        }
    }

    fn changed(path: &str, modified: bool) -> Response {
        Response::DocumentChanged(DocState {
            info: test_info(path),
            can_undo: true,
            can_redo: false,
            modified,
            pages_unchanged: false,
            measurement_scales_unchanged: false,
            has_signatures: false,
        })
    }

    #[test]
    fn failed_form_commit_cancels_waiting_save_and_preserves_draft() {
        let (mut app, requests, responses) = test_app();
        form_draft(&mut app, "New");
        app.save_document(false);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Edit(EditOp::FillForm { .. }))));
        responses
            .send(Response::OperationFailed(EngineError::FormValueInvalid(
                "Name".into(),
            )))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(!requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        assert_eq!(
            app.forms.drafts.get("Name").map(String::as_str),
            Some("New")
        );
        assert!(!app.saving);
        app.save_document(false);
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, Request::Save { .. })),
            "retry must wait for the real form values after a failed optimistic update"
        );
    }

    #[test]
    fn failed_edit_releases_waiting_close_without_hanging() {
        let (mut app, requests, responses) = test_app();
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::RotateClockwise);
        app.execute(&ctx, Command::CloseDocument);
        responses
            .send(Response::OperationFailed(EngineError::InvalidFormat))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(requests.try_iter().any(|r| matches!(r, Request::Close)));
        assert!(app.document.is_none());
    }

    #[test]
    fn save_failure_keeps_document_and_cancels_close_after_save() {
        let (mut app, requests, responses) = test_app();
        app.dirty = true;
        app.after_save = Some(PendingAction::Close);
        app.save_document(false);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        responses
            .send(Response::SaveFailed(EngineError::InvalidFormat))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(app.document.is_some());
        assert!(!requests.try_iter().any(|r| matches!(r, Request::Close)));
        assert!(app.after_save.is_none());
    }

    #[test]
    fn window_close_with_panel_draft_is_cancelled() {
        let (mut app, _requests, _responses) = test_app();
        form_draft(&mut app, "New");
        let ctx = egui::Context::default();
        app.has_icons = crate::icons::install(&ctx);
        theme::apply(&ctx, app.settings.dark_theme);
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 780.0),
            )),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
        let mut frame = eframe::Frame::_new_kittest();
        let mut out = ctx.run_ui(input, |ui| eframe::App::ui(&mut app, ui, &mut frame));
        out.textures_delta.clear();
        assert!(out
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
        assert!(app.document.is_some());
        assert_eq!(
            app.forms.drafts.get("Name").map(String::as_str),
            Some("New")
        );
    }

    #[test]
    fn quitting_discard_closes_only_the_confirmed_tab() {
        let (mut app, requests, _responses) = test_app();
        let mut second_ui = app.take_tab();
        second_ui.document.as_mut().unwrap().info = test_info("test/second.pdf");
        second_ui.dirty = true;
        app.document = Some(OpenDocument {
            info: test_info("test/first.pdf"),
            page: 0,
        });
        app.dirty = true;
        app.tabs.tabs.push(crate::tabs::Tab {
            recovery_id: crate::recovery::next_id(),
            path: PathBuf::from("test/second.pdf"),
            dirty: true,
            ui: Some(second_ui),
            doc: Some(crate::worker::SuspendedDoc {
                signature_source: None,
                bytes: vec![],
                password: None,
                path: PathBuf::from("test/second.pdf"),
                undo: vec![],
                redo: vec![],
                save_point: None,
                modified: true,
            }),
        });
        let ctx = egui::Context::default();
        app.has_icons = crate::icons::install(&ctx);
        ctx.enable_accesskit();
        app.execute(&ctx, Command::Quit);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 780.0),
            )),
            ..Default::default()
        };
        let mut target = None;
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |_| app.unsaved_dialog(&ctx));
            if let Some(tree) = &out.platform_output.accesskit_update {
                if let Some(bounds) = tree
                    .nodes
                    .iter()
                    .map(|(_, node)| node)
                    .find(|node| node.label() == Some(app.t("dialog.unsaved.discard")))
                    .and_then(|node| node.bounds())
                {
                    target = Some(egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.0) as f32,
                        ((bounds.y0 + bounds.y1) / 2.0) as f32,
                    ));
                }
            }
            out.textures_delta.clear();
        }
        let pos = target.expect("discard button bounds");
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
            let mut out = ctx.run_ui(input, |_| app.unsaved_dialog(&ctx));
            out.textures_delta.clear();
        }
        assert!(
            requests.try_iter().any(|r| matches!(r, Request::Close)),
            "discard must drop the confirmed document, not suspend its modified snapshot"
        );
        assert_eq!(app.tabs.tabs.len(), 1);
        assert_eq!(app.tabs.tabs[0].path, PathBuf::from("test/second.pdf"));
    }

    #[test]
    fn repeated_save_before_completion_cannot_save_resumed_tab_over_first_file() {
        let (mut app, requests, responses) = test_app();
        let mut second_ui = app.take_tab();
        second_ui.document.as_mut().unwrap().info = test_info("test/second.pdf");
        app.document = Some(OpenDocument {
            info: test_info("test/first.pdf"),
            page: 0,
        });
        app.tabs.tabs.push(crate::tabs::Tab {
            recovery_id: crate::recovery::next_id(),
            path: PathBuf::from("test/second.pdf"),
            dirty: false,
            ui: Some(second_ui),
            doc: Some(crate::worker::SuspendedDoc {
                signature_source: None,
                bytes: vec![],
                password: None,
                path: PathBuf::from("test/second.pdf"),
                undo: vec![],
                redo: vec![],
                save_point: Some(0),
                modified: false,
            }),
        });
        let ctx = egui::Context::default();
        form_draft(&mut app, "Old");
        app.save_document(false);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Save { .. })));
        app.forms.drafts.insert("Name".into(), "Next".into());
        app.save_document(false);
        let duplicate_commit = requests
            .try_iter()
            .any(|r| matches!(r, Request::Edit(EditOp::FillForm { .. })));
        responses
            .send(Response::Saved {
                path: PathBuf::from("test/first.pdf"),
            })
            .unwrap();
        app.handle_responses(&ctx);
        app.switch_to(1);
        responses.send(Response::MutationFinished).unwrap();
        if duplicate_commit {
            responses.send(changed("test/first.pdf", true)).unwrap();
            responses.send(Response::MutationFinished).unwrap();
        }
        responses
            .send(Response::Suspended(crate::worker::SuspendedDoc {
                signature_source: None,
                bytes: vec![],
                password: None,
                path: PathBuf::from("test/first.pdf"),
                undo: vec![],
                redo: vec![],
                save_point: Some(0),
                modified: duplicate_commit,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        responses
            .send(Response::Resumed(DocState {
                info: test_info("test/second.pdf"),
                can_undo: false,
                can_redo: false,
                modified: false,
                pages_unchanged: false,
                measurement_scales_unchanged: false,
                has_signatures: false,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, Request::Save { .. })),
            "a repeated Save must not write second.pdf into first.pdf"
        );
        assert_eq!(
            app.tabs.tabs[0]
                .ui
                .as_ref()
                .unwrap()
                .forms
                .drafts
                .get("Name")
                .map(String::as_str),
            Some("Next")
        );
    }

    #[test]
    fn failed_operation_keeps_inline_form_text_during_model_reload() {
        let (mut app, _requests, responses) = test_app();
        form_draft(&mut app, "Old");
        app.forms.drafts.clear();
        let rect = sunipdf_core::PointRect {
            left: 10.0,
            bottom: 700.0,
            right: 200.0,
            top: 730.0,
        };
        app.forms.info.as_mut().unwrap().fields[0]
            .widgets
            .push(sunipdf_core::FieldWidget {
                page: 0,
                rect,
                display: rect,
                on_state: None,
                export_value: None,
            });
        app.forms.editing = Some(crate::forms::InPlace {
            page: 0,
            field: "Name".into(),
            widget: 0,
            text: "Unsaved".into(),
            multiline: false,
            max_len: Some(10),
            focus: true,
            armed: true,
        });
        let ctx = egui::Context::default();
        responses
            .send(Response::OperationFailed(EngineError::InvalidFormat))
            .unwrap();
        app.handle_responses(&ctx);
        let view = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 780.0));
        let mut out = ctx.run_ui(Default::default(), |_| {
            app.forms_editor(&ctx, view, view, 0, 792.0)
        });
        out.textures_delta.clear();
        assert_eq!(
            app.forms.editing.as_ref().map(|e| e.text.as_str()),
            Some("Unsaved"),
            "missing model during reload must not discard the active editor"
        );
    }

    #[test]
    fn cancelling_close_prompt_allows_opening_another_document() {
        let (mut app, requests, _responses) = test_app();
        form_draft(&mut app, "New");
        let ctx = egui::Context::default();
        app.has_icons = crate::icons::install(&ctx);
        app.execute(&ctx, Command::CloseDocument);
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| app.unsaved_dialog(&ctx),
        );
        out.textures_delta.clear();
        app.open_path(PathBuf::from("test/second.pdf"), None);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::Open { path, .. }
            if path == Path::new("test/second.pdf"))));
        assert_eq!(
            app.tabs.tabs[0]
                .ui
                .as_ref()
                .unwrap()
                .forms
                .drafts
                .get("Name")
                .map(String::as_str),
            Some("New")
        );
    }

    #[test]
    fn quitting_waits_for_edited_background_tab_to_finish_suspending() {
        let (mut app, _requests, responses) = test_app();
        let ctx = egui::Context::default();
        app.execute(&ctx, Command::RotateClockwise);
        app.suspend_active();
        app.execute(&ctx, Command::Quit);
        responses.send(changed("test/first.pdf", true)).unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        assert!(
            !app.allow_close,
            "the edited tab has not reached its suspended state"
        );
        responses
            .send(Response::Suspended(crate::worker::SuspendedDoc {
                signature_source: None,
                bytes: vec![],
                password: None,
                path: PathBuf::from("test/first.pdf"),
                undo: vec![],
                redo: vec![],
                save_point: None,
                modified: true,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        responses
            .send(Response::Resumed(DocState {
                info: test_info("test/first.pdf"),
                can_undo: true,
                can_redo: false,
                modified: true,
                pages_unchanged: false,
                measurement_scales_unchanged: false,
                has_signatures: false,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        assert!(matches!(app.unsaved_prompt, Some(PendingAction::Quit)));
        assert!(app.document.is_some());
    }

    #[test]
    fn every_error_key_exists_in_both_catalogs() {
        let errors = [
            EngineError::LibraryNotFound(vec![]),
            EngineError::LibraryLoad(String::new()),
            EngineError::Io(std::io::Error::other("x")),
            EngineError::PasswordRequired,
            EngineError::InvalidFormat,
            EngineError::SecurityRestricted,
            EngineError::PageOutOfRange(0),
            EngineError::RenderTooLarge,
            EngineError::ImageBatchTooLarge,
            EngineError::AlreadyExists(PathBuf::new()),
            EngineError::MissingGlyphs(String::new()),
            EngineError::FontNotEmbeddable,
            EngineError::PasswordNotLatin1,
            EngineError::FormFieldNotFound(String::new()),
            EngineError::FormValueInvalid(String::new()),
            EngineError::FormXfa,
            EngineError::RedactionIncomplete,
            EngineError::Other(String::new()),
        ];
        for e in errors {
            let key = error_key(&e);
            for lang in Language::ALL {
                assert_ne!(tr(lang, key), key, "clé {key} absente en {}", lang.code());
            }
        }
    }

    #[test]
    fn oversized_image_batch_reports_a_localized_error_without_changing_the_document() {
        assert_eq!(
            EngineError::ImageBatchTooLarge.code(),
            "image_batch_too_large"
        );
        for lang in Language::ALL {
            let (mut app, requests, responses) = test_app();
            app.settings.language = lang.code().to_owned();
            app.dirty = true;
            app.tools.running = Some("tools.running.images");
            let path = app.document.as_ref().unwrap().info.path.clone();
            responses
                .send(Response::BatchDone(crate::worker::BatchResult::Failed(
                    EngineError::ImageBatchTooLarge,
                )))
                .unwrap();
            app.handle_responses(&egui::Context::default());
            assert_eq!(app.error, Some("error.image_batch_too_large"));
            assert_ne!(
                app.t("error.image_batch_too_large"),
                "error.image_batch_too_large"
            );
            assert!(app.t("error.image_batch_too_large").contains("512"));
            assert_eq!(app.document.as_ref().unwrap().info.path, path);
            assert!(app.dirty);
            assert!(app.tools.running.is_none());
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn ui_keys_used_by_app_exist() {
        let src = concat!(
            include_str!("app.rs"),
            include_str!("view.rs"),
            include_str!("main.rs"),
            include_str!("shell.rs"),
            include_str!("workspace.rs"),
            include_str!("editor.rs"),
            include_str!("objects.rs"),
            include_str!("inline_text.rs"),
            include_str!("pagesetup.rs"),
            include_str!("sign.rs"),
            include_str!("forms.rs"),
            include_str!("annot_adv.rs"),
            include_str!("redact.rs"),
            include_str!("features.rs"),
            include_str!("form_builder.rs"),
            include_str!("certificate.rs"),
            include_str!("ocr.rs"),
            include_str!("printing.rs")
        );
        let mut checked = 0;
        for key in src.split('"').filter(|s| {
            s.contains('.')
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
        }) {
            if !key.contains("..")
                && !key.ends_with(".rs")
                && !key.ends_with(".pdf")
                && key.split('.').all(|p| !p.is_empty())
            {
                checked += 1;
                for lang in Language::ALL {
                    assert_ne!(tr(lang, key), key, "clé {key} absente en {}", lang.code());
                }
            }
        }
        assert!(checked > 40, "{checked} clés vérifiées");
    }

    #[test]
    fn registry_keys_exist_in_both_catalogs() {
        for info in sunipdf_core::REGISTRY {
            for key in std::iter::once(info.label_key).chain(info.shortcut_key) {
                for lang in Language::ALL {
                    assert_ne!(tr(lang, key), key, "clé {key} absente en {}", lang.code());
                }
            }
        }
    }

    /// `consume_key(COMMAND, k)` accepte aussi Ctrl+Maj+k : la variante avec Maj doit être lue en
    /// premier, sinon Ctrl+Maj+S enregistre sur l'original, Ctrl+Maj+D ouvre Propriétés et
    /// Ctrl+Maj+Z annule au lieu de rétablir (constats des revues).
    #[test]
    fn shift_shortcuts_precede_their_plain_variant() {
        for (name, table) in [("global", SHORTCUTS)] {
            for (i, (m, k, c)) in table.iter().enumerate() {
                if !m.shift {
                    continue;
                }
                let plain = Modifiers { shift: false, ..*m };
                let before = table[..i]
                    .iter()
                    .find(|(m2, k2, _)| k2 == k && *m2 == plain);
                assert!(
                    before.is_none(),
                    "{name} : {:?} lu avant {m:?}+{k:?} ({c:?})",
                    before.unwrap()
                );
            }
        }
        let pos = |c: Command| SHORTCUTS.iter().position(|(_, _, x)| *x == c).unwrap();
        assert!(pos(Command::SaveAs) < pos(Command::Save));
        assert!(pos(Command::DuplicateObject) < pos(Command::Properties));
        assert!(pos(Command::Redo) < pos(Command::Undo));
        assert!(pos(Command::RotateCounterClockwise) < pos(Command::RotateClockwise));
    }

    #[test]
    fn zoom_quantization_and_pdf_detection() {
        assert_eq!(sunipdf_core::tiles::quantize_milli(1.3333), 1335);
        assert_eq!(ZOOM_STEPS.last(), Some(&16.0));
        assert!(ZOOM_STEPS.windows(2).all(|w| w[0] < w[1]));
        assert!(is_pdf(Path::new("a/b.PDF")));
        assert!(!is_pdf(Path::new("a/b.txt")));
    }
}

#[cfg(test)]
#[path = "feature_tests.rs"]
mod feature_tests;
