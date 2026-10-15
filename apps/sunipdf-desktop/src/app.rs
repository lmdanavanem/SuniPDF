//! État et interface de l'application. Aucune logique PDF ici : tout passe par `worker`.

use crate::dialog;
use crate::icons::glyph;
use crate::settings::Settings;
use crate::theme;
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
    query: String,
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

/// Raccourcis qui restent actifs quand un champ de saisie a le clavier (Ctrl seulement).
const FIELD_SHORTCUTS: &[(Modifiers, Key, Command)] = &[
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::S,
        Command::SaveAs,
    ),
    (Modifiers::COMMAND, Key::O, Command::OpenDocument),
    (Modifiers::COMMAND, Key::S, Command::Save),
    (Modifiers::COMMAND, Key::K, Command::CommandPalette),
    (Modifiers::COMMAND, Key::F, Command::Find),
];

/// Raccourcis globaux, dans l'ordre de lecture : une combinaison avec Maj précède toujours la
/// même touche sans Maj (`consume_key` ignore un Maj en trop ; invariant testé).
const SHORTCUTS: &[(Modifiers, Key, Command)] = &[
    (Modifiers::COMMAND, Key::O, Command::OpenDocument),
    (Modifiers::COMMAND, Key::W, Command::CloseDocument),
    (Modifiers::COMMAND, Key::Plus, Command::ZoomIn),
    (Modifiers::COMMAND, Key::Equals, Command::ZoomIn),
    (Modifiers::COMMAND, Key::Minus, Command::ZoomOut),
    (Modifiers::COMMAND, Key::Num0, Command::ActualSize),
    (Modifiers::NONE, Key::PageDown, Command::NextPage),
    (Modifiers::NONE, Key::ArrowRight, Command::NextPage),
    (Modifiers::NONE, Key::PageUp, Command::PreviousPage),
    (Modifiers::NONE, Key::ArrowLeft, Command::PreviousPage),
    (Modifiers::NONE, Key::Home, Command::FirstPage),
    (Modifiers::NONE, Key::End, Command::LastPage),
    (Modifiers::NONE, Key::F4, Command::ToggleThumbnails),
    (Modifiers::COMMAND, Key::F, Command::Find),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::S,
        Command::SaveAs,
    ),
    (Modifiers::COMMAND, Key::S, Command::Save),
    // Après Ctrl+S et Ctrl+Maj+S : `consume_key` exige la même touche Ctrl.
    (Modifiers::NONE, Key::S, Command::SignDocument),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::R,
        Command::RotateCounterClockwise,
    ),
    (Modifiers::COMMAND, Key::R, Command::RotateClockwise),
    (Modifiers::COMMAND, Key::Delete, Command::DeletePage),
    (Modifiers::NONE, Key::F9, Command::ToggleTools),
    (Modifiers::NONE, Key::F8, Command::OrganizePages),
    (Modifiers::NONE, Key::F7, Command::ToggleComments),
    (Modifiers::NONE, Key::F6, Command::ToggleBookmarks),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::D,
        Command::DuplicateObject,
    ),
    (Modifiers::COMMAND, Key::D, Command::Properties),
    (Modifiers::NONE, Key::F3, Command::ToggleObjects),
    (Modifiers::NONE, Key::V, Command::ToolSelect),
    (Modifiers::NONE, Key::H, Command::ToolHighlight),
    (Modifiers::NONE, Key::U, Command::ToolUnderline),
    (Modifiers::NONE, Key::X, Command::ToolStrikeOut),
    (Modifiers::NONE, Key::N, Command::ToolNote),
    (Modifiers::NONE, Key::E, Command::ToolSelectObject),
    (Modifiers::NONE, Key::T, Command::ToolAddText),
    (Modifiers::NONE, Key::I, Command::ToolAddImage),
    (Modifiers::NONE, Key::R, Command::ToolRectangle),
    (Modifiers::NONE, Key::O, Command::ToolEllipse),
    (Modifiers::NONE, Key::L, Command::ToolLine),
    (Modifiers::NONE, Key::Delete, Command::DeleteObject),
    (Modifiers::NONE, Key::F2, Command::EditObjectText),
    (Modifiers::COMMAND, Key::CloseBracket, Command::BringToFront),
    (Modifiers::COMMAND, Key::OpenBracket, Command::SendToBack),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::Z,
        Command::Redo,
    ),
    (Modifiers::COMMAND, Key::Z, Command::Undo),
    (Modifiers::COMMAND, Key::Y, Command::Redo),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::I,
        Command::InsertPages,
    ),
    (Modifiers::COMMAND, Key::K, Command::CommandPalette),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::Tab,
        Command::PrevTab,
    ),
    (Modifiers::COMMAND, Key::Tab, Command::NextTab),
];

pub struct SuniApp {
    pub settings: Settings,
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
    pub thumbs_pending: HashSet<usize>,
    pub thumb_px: u32,
    pub find: FindState,
    pub dirty: bool,
    pub saving: bool,
    allow_close: bool,
    unsaved_prompt: Option<PendingAction>,
    after_save: Option<PendingAction>,
    pub status_msg: Option<String>,
    pub has_icons: bool,
    pub logo: Option<egui::TextureHandle>,
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
        Self {
            has_icons,
            logo: None,
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
            error: None,
            password_prompt: None,
            show_about: false,
            pending_files: initial_files,
            thumbnails: HashMap::new(),
            thumbs_pending: HashSet::new(),
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
            allow_close: false,
            unsaved_prompt: None,
            after_save: None,
            status_msg: None,
        }
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
        if !matches!(self.engine, EngineState::Ready) {
            self.pending_files.push(path);
            return;
        }
        if self.tabs.resuming.is_some() || self.opening.is_some() {
            // Une ouverture ou un changement d'onglet est déjà en cours.
            return;
        }
        self.error = None;
        if self.document.is_some() {
            self.suspend_active();
        }
        self.opening = Some(path.clone());
        self.worker.send(Request::Open { path, password });
    }

    fn close_document(&mut self, announce: bool) {
        if self.document.take().is_some() {
            self.worker.send(Request::Close);
        }
        self.view.reset();
        self.page_input.clear();
        self.thumbnails.clear();
        self.thumbs_pending.clear();
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
        self.can_undo = false;
        self.can_redo = false;
        self.dirty = false;
        self.saving = false;
        self.status_msg = None;
        if announce {
            self.error = None;
        }
        self.on_active_closed();
    }

    pub fn go_to_page(&mut self, page: usize) {
        if let Some(doc) = &mut self.document {
            let clamped = page.min(doc.info.page_count().saturating_sub(1));
            if clamped != doc.page {
                doc.page = clamped;
                self.error = None;
                // Une sélection ou un texte en cours sur une page qui n'est plus affichée : Suppr ou
                // les flèches agiraient à l'aveugle, et l'éditeur invisible garderait le clavier.
                self.edit.selection = None;
                if self
                    .edit
                    .text_prompt
                    .as_ref()
                    .is_some_and(|p| p.page != clamped)
                {
                    self.edit.text_prompt = None;
                }
            }
            self.page_input = (clamped + 1).to_string();
            // Toujours défiler, même vers la page courante (on peut être au milieu de la page).
            self.lock_on_page(clamped);
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
        if let Err(e) = self.settings.save() {
            log::warn!("préférences non enregistrées : {e}");
        }
    }

    // ----- Messages du moteur ---------------------------------------------

    fn handle_responses(&mut self, ctx: &egui::Context) {
        let responses: Vec<Response> = self.worker.poll().collect();
        for response in responses {
            match response {
                Response::EngineReady => {
                    self.engine = EngineState::Ready;
                    self.open_next_pending();
                }
                Response::EngineFailed(e) => {
                    self.error = Some(error_key(&e));
                    log::error!("moteur indisponible : {e}");
                    self.engine = EngineState::Failed;
                }
                Response::Opened(DocState { info, .. }) => {
                    self.opening = None;
                    self.password_prompt = None;
                    self.page_input = "1".into();
                    self.settings.push_recent(&info.path);
                    self.persist_settings();
                    self.can_undo = false;
                    self.can_redo = false;
                    let opened_path = info.path.clone();
                    // Rien du document précédent ne doit apparaître dans le nouveau.
                    self.thumbnails.clear();
                    self.thumbs_pending.clear();
                    self.edit.reset_for_tab();
                    self.view.reset();
                    // Première page en haut, centrée si le document mêle des tailles de page.
                    self.view.request(crate::view::ScrollRequest::Page(0));
                    self.document = Some(OpenDocument { info, page: 0 });
                    self.register_opened_tab(opened_path);
                    self.dirty = false;
                    self.status_msg = None;
                    if let Some(cmd) = self.after_open.take() {
                        self.execute(ctx, cmd);
                    }
                    self.update_title(ctx);
                    self.worker.send(Request::Bookmarks);
                    // Les commandes scriptées attendent la première image du document (fin de `ui`).
                    self.open_next_pending();
                }
                Response::OpenFailed { path, error } => {
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
                    self.thumbs_pending.remove(&page);
                    if image.width == self.thumb_px {
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
                Response::DocumentChanged(DocState {
                    info,
                    can_undo,
                    can_redo,
                    pages_unchanged,
                    modified,
                }) => {
                    self.can_undo = can_undo;
                    self.can_redo = can_redo;
                    let count = info.page_count();
                    self.organize.selected.retain(|p| *p < count);
                    if let Some(doc) = &mut self.document {
                        let same_sizes = doc.info.page_sizes == info.page_sizes;
                        doc.info = info;
                        doc.page = doc.page.min(doc.info.page_count().saturating_sub(1));
                        self.page_input = (doc.page + 1).to_string();
                        // Retouche : l'image reste affichée jusqu'aux nouvelles tuiles.
                        self.view
                            .on_document_changed(pages_unchanged, same_sizes, doc.page);
                    }
                    self.thumbnails.clear();
                    self.thumbs_pending.clear();
                    self.find.searched = None;
                    self.find.matches.clear();
                    self.annotate.selection = None;
                    self.annotate.annotations_page = None;
                    // La liste d'objets en cache (panneau, prise directe) est périmée.
                    self.edit.objects_page = None;
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
                    self.update_title(ctx);
                }
                // L'intention (surligner, souligner, barrer ou simple sélection) est celle du
                // glissement, pas celle de l'outil au moment où la réponse arrive.
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
                        None => self.annotate.selection = Some((page, selection)),
                    }
                }
                Response::Annotations { page, list } => {
                    if self.annotate.annotations_page == Some(page) {
                        self.annotate.annotations = list;
                    }
                }
                Response::BatchDone(result) => self.on_batch_done(result),
                Response::ObjectSelected { page, info } => self.on_object_selected(page, info),
                Response::ObjectsSelected { page, infos, mode } => {
                    self.on_objects_selected(page, infos, mode)
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
                Response::Suspended(s) => self.on_suspended(s),
                Response::Resumed(state) => self.on_resumed(state, ctx),
                Response::ResumeFailed(error) => {
                    log::error!("reprise d'onglet impossible : {error}");
                    self.on_resume_failed();
                    self.error = Some(error_key(&error));
                }
                Response::Properties(props) => {
                    self.docinfo.metadata_edit = props.metadata.clone();
                    self.docinfo.properties = Some(props);
                }
                Response::Bookmarks(list) => {
                    self.docinfo.bookmarks = list;
                }
                Response::OperationFailed(e) => {
                    log::warn!("opération impossible : {e}");
                    self.error = Some(error_key(&e));
                    self.edit.pending_status = None;
                    self.edit.reselect = None;
                    self.edit.reselect_many = None;
                    // Copie échouée : le « couper » qui l'attendait ne supprime rien.
                    self.edit.pending_copy = None;
                    if matches!(
                        e,
                        EngineError::MissingGlyphs(_) | EngineError::FontNotEmbeddable
                    ) {
                        // Le texte saisi n'est pas perdu : l'éditeur rouvre pour changer de police.
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
                    self.status_msg = Some(tr_args(
                        self.lang(),
                        "status.saved",
                        &[("name", &file_name(&path))],
                    ));
                    if let Some(doc) = &mut self.document {
                        doc.info.path = path;
                    }
                    self.update_title(ctx);
                    if let Some(action) = self.after_save.take() {
                        self.run_pending(ctx, action);
                    }
                }
                Response::SaveFailed(e) => {
                    self.saving = false;
                    self.after_save = None;
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
            }
        }
    }

    pub fn update_title(&self, ctx: &egui::Context) {
        let title = match &self.document {
            Some(doc) => tr_args(
                self.lang(),
                if self.dirty {
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

    fn handle_input(&mut self, ctx: &egui::Context) {
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
                for &(m, k, c) in FIELD_SHORTCUTS {
                    if i.consume_key(m, k) {
                        cmds.push(c);
                    }
                }
            });
            for c in cmds {
                self.execute(ctx, c);
            }
            return;
        }
        if self.palette.open {
            let mut cmds = Vec::new();
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::COMMAND, Key::K) {
                    cmds.push(Command::CommandPalette);
                }
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
        // Un Échap = un effet, le plus proche d'abord (`workspace::EscState::action`).
        let esc = crate::workspace::take_escape(ctx, self.esc_state(ctx));
        if esc == Some(Esc::Deselect) {
            self.edit.selection = None;
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
        let continuous = self.view.mode == ViewMode::Continuous;
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
            for &(m, k, c) in SHORTCUTS {
                if i.consume_key(m, k) {
                    commands.push(c);
                }
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
        self.password_prompt.is_some()
            || self.show_about
            || self.unsaved_prompt.is_some()
            || self.tools.split_open
            || self.tools.stamp_open.is_some()
            || self.docinfo.unprotect_open
            || self.docinfo.properties_open
            || self.docinfo.protect_open
            || self.pagesetup.dialog.is_some()
            || self.annotate.note_prompt.is_some()
            || self.edit.text_prompt.is_some()
            || self.sign.open.is_some()
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
        log::debug!("commande {}", command.info().id);
        if command.info().needs_document && self.document.is_none() {
            // Façon PDFCraft : l'outil reste utilisable, il commence par ouvrir un fichier.
            self.after_open = Some(command);
            self.open_file_dialog();
            return;
        }
        match command {
            Command::OpenDocument => self.open_file_dialog(),
            Command::CloseDocument => {
                if self.dirty {
                    self.unsaved_prompt = Some(PendingAction::Close);
                } else {
                    self.close_document(true);
                    self.update_title(ctx);
                }
            }
            Command::Quit => {
                if self.dirty {
                    self.unsaved_prompt = Some(PendingAction::Quit);
                } else if let Some(i) = self.any_background_dirty() {
                    self.tabs.quit_pending = true;
                    self.switch_to(i);
                } else {
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
            Command::ToggleThumbnails => {
                self.settings.show_thumbnails = !self.settings.show_thumbnails;
                self.persist_settings();
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
                    return; // ponytail: un seul déplacement à la fois ; blocs contigus plus tard.
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
            Command::DeleteObject => self.delete_selected_object(),
            Command::DuplicateObject => self.duplicate_selected(),
            Command::RotateObjectLeft => self.rotate_selected(-90.0),
            Command::RotateObjectRight => self.rotate_selected(90.0),
            Command::BringToFront => self.reorder_selected(sunipdf_core::ZOrder::Front),
            Command::SendToBack => self.reorder_selected(sunipdf_core::ZOrder::Back),
            Command::EditObjectText => self.open_text_prompt_for_selection(),
            Command::CopyObjects => self.copy_objects(false),
            Command::CutObjects => self.copy_objects(true),
            Command::PasteObjects => self.paste_objects(ctx),
            Command::SelectAllObjects => self.select_all_objects(ctx),
            Command::ReplaceImage => self.replace_selected_image(),
            Command::ToggleObjects => {
                self.edit.objects_open = !self.edit.objects_open;
                self.edit.objects_page = None;
            }
            Command::CropPages | Command::ResizePages | Command::InsertBlankPage => {
                self.open_page_dialog(command)
            }
            Command::ToolSelect
            | Command::ToolHighlight
            | Command::ToolUnderline
            | Command::ToolStrikeOut
            | Command::ToolNote
            | Command::ToolSelectObject
            | Command::ToolAddText
            | Command::ToolAddImage
            | Command::ToolRectangle
            | Command::ToolEllipse
            | Command::ToolLine => {
                self.cancel_placing();
                self.annotate.tool =
                    crate::annotate::Tool::from_command(command).unwrap_or_default();
                self.annotate.drag = None;
                if !matches!(self.annotate.tool, crate::annotate::Tool::SelectObject) {
                    self.edit.selection = None;
                }
                // Raccourci, palette ou menu : la barre de l'outil propriétaire suit.
                crate::workspace::follow(
                    &mut self.annotate,
                    &mut self.organize,
                    &mut self.edit,
                    command,
                );
            }
            Command::HubFillSign
            | Command::HubEdit
            | Command::HubOrganize
            | Command::HubComment
            | Command::HubProtect
            | Command::HubStamp => {
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
                self.docinfo.bookmarks_open = !self.docinfo.bookmarks_open;
            }
            Command::Properties => {
                self.docinfo.properties = None;
                self.docinfo.properties_open = true;
                self.worker.send(Request::Properties);
            }
            Command::Protect => {
                self.docinfo.protection = sunipdf_core::Protection::default();
                self.docinfo.protect_error = None;
                self.docinfo.protect_open = true;
            }
            Command::Unprotect => {
                let encrypted = self.docinfo.properties.as_ref().map(|p| p.encrypted);
                if encrypted == Some(false) {
                    self.error = Some("error.not_encrypted");
                } else {
                    self.docinfo.unprotect_password.clear();
                    self.docinfo.unprotect_open = true;
                }
            }
            Command::StripMetadata => {
                self.worker
                    .send(Request::Edit(EditOp::Bytes(BytesOp::StripMetadata)));
            }
            Command::ToggleComments => {
                self.annotate.comments_open = !self.annotate.comments_open;
                self.annotate.annotations_page = None;
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
                self.show_tools = !self.show_tools;
                self.settings.show_tools = self.show_tools;
                self.persist_settings();
            }
            Command::CommandPalette => {
                self.palette.open = !self.palette.open;
                self.palette.query.clear();
                self.palette.selected = 0;
                self.palette.focus_requested = self.palette.open;
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
                let shortcut = info.shortcut_key.map(|k| tr(lang, k)).unwrap_or("");
                let enabled = !info.needs_document || has_doc;
                // Mode de vue actif surligné (les deux entrées partagent F5).
                let active = has_doc
                    && match info.command {
                        Command::ViewContinuous => self.view.mode == ViewMode::Continuous,
                        Command::ViewSinglePage => self.view.mode == ViewMode::Single,
                        _ => false,
                    };
                if menu_item(ui, tr(lang, info.label_key), shortcut, enabled, active) {
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

    /// Panneau des miniatures : seules les lignes visibles demandent un rendu (cache côté moteur).
    fn thumbnails_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
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
            self.thumbs_pending.clear();
        }
        if self.thumbnails.len() > crate::view::THUMB_KEEP {
            // Long document parcouru : seules les miniatures proches de la page courante restent.
            self.thumbnails.retain(|p, _| p.abs_diff(current) <= 100);
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
            for page in rows {
                let ratio = sizes[page].height_pt / sizes[page].width_pt;
                let size = egui::vec2(thumb_w, thumb_w * ratio);
                if !self.thumbnails.contains_key(&page) && self.thumbs_pending.insert(page) {
                    self.worker.send(Request::Thumbnail { page, width_px: px });
                }
                ui.vertical_centered(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
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
                                .alt_text(tr_args(
                                    lang,
                                    "a11y.thumbnail",
                                    &[("page", &(page + 1).to_string())],
                                )),
                        );
                    }
                    ui.painter()
                        .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
                    if resp.clicked() {
                        clicked = Some(page);
                    }
                    ui.label((page + 1).to_string());
                });
            }
        });
        self.view.thumb_view = Some((out.state.offset.y, out.inner_rect.height()));
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
                self.dirty = false;
                self.run_pending(ctx, action);
            }
            Some(UnsavedChoice::Cancel) => self.unsaved_prompt = None,
            None => {}
        }
    }

    fn run_pending(&mut self, ctx: &egui::Context, action: PendingAction) {
        match action {
            PendingAction::Close => {
                self.close_document(true);
                self.update_title(ctx);
            }
            PendingAction::Quit => {
                if let Some(i) = self.any_background_dirty() {
                    self.tabs.quit_pending = true;
                    self.switch_to(i);
                } else {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    /// Enregistre vers le chemin courant, ou demande un chemin (`ask_path`).
    fn save_document(&mut self, ask_path: bool) {
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
                    self.after_save = None;
                    return;
                }
            }
            if path.extension().is_none() {
                path.set_extension("pdf");
            }
        }
        self.saving = true;
        self.worker.send(Request::Save { path });
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
        }
    }

    fn about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        if self.logo.is_none() {
            let png = include_bytes!("../../../assets/icon/sunipdf-128.png");
            if let Ok(img) = image::load_from_memory(png) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                self.logo = Some(ctx.load_texture(
                    "logo",
                    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        let logo = self.logo.clone();
        let mut close = false;
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
                ui.horizontal(|ui| {
                    if let Some(tex) = &logo {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(egui::vec2(72.0, 72.0))
                                .corner_radius(14),
                        );
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.label(crate::icons::semibold("SuniPDF", 26.0).color(p.text));
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
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(crate::icons::semibold(tr(lang, "about.credit"), 13.5).color(p.text));
                    ui.label(egui::RichText::new("-").size(13.5));
                    ui.hyperlink_to(
                        egui::RichText::new(tr(lang, "about.website")).size(13.5),
                        "https://www.navanem.com",
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
    }
}

impl eframe::App for SuniApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_responses(&ctx);
        self.handle_input(&ctx);
        let p = theme::palette(self.settings.dark_theme);
        let chrome = |fill: egui::Color32| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(6, 4))
        };

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
        if self.document.is_some() && self.annotate.workspace.is_some_and(|w| w.is_mode()) {
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
        if self.find.open && self.document.is_some() {
            egui::Panel::top("find")
                .frame(chrome(p.chrome))
                .show(ui, |ui| self.find_bar(ui));
        }
        egui::Panel::bottom("status")
            .frame(chrome(p.chrome))
            .show(ui, |ui| self.status_bar(ui));
        if self.show_tools {
            egui::Panel::left("tools")
                .resizable(true)
                .default_size(250.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .inner_margin(egui::Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| self.tools_panel(&ctx, ui));
        }
        if self.document.is_some() && self.docinfo.bookmarks_open && !self.organize.active {
            egui::Panel::left("bookmarks")
                .resizable(true)
                .default_size(220.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.chrome)
                        .inner_margin(egui::Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| self.bookmarks_panel(&ctx, ui));
        }
        if self.document.is_some() && self.annotate.comments_open && !self.organize.active {
            egui::Panel::right("comments")
                .resizable(true)
                .default_size(280.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.chrome)
                        .inner_margin(egui::Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| self.comments_panel(&ctx, ui));
        }
        if self.document.is_some() && self.edit.objects_open && !self.organize.active {
            egui::Panel::right("objects")
                .resizable(true)
                .default_size(260.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.chrome)
                        .inner_margin(egui::Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| self.objects_panel(&ctx, ui));
        }
        if self.document.is_some() {
            egui::Panel::right("rail")
                .resizable(false)
                .default_size(48.0)
                .frame(egui::Frame::new().fill(p.surface).inner_margin(4))
                .show(ui, |ui| self.right_rail(&ctx, ui));
            if self.settings.show_thumbnails && !self.organize.active {
                egui::Panel::left("thumbnails")
                    .resizable(true)
                    .default_size(170.0)
                    .frame(egui::Frame::new().fill(p.chrome).inner_margin(6))
                    .show(ui, |ui| self.thumbnails_panel(&ctx, ui));
            }
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
            && (self.dirty || self.any_background_dirty().is_some())
            && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            // Même logique que Quitter : invite pour le document actif s'il est modifié, sinon
            // reprise de l'onglet modifié en arrière-plan (jamais d'écriture d'un document intact).
            if self.unsaved_prompt.is_none() && self.tabs.resuming.is_none() {
                self.execute(&ctx, Command::Quit);
            }
        }
        self.unsaved_dialog(&ctx);
        self.split_dialog(&ctx);
        self.stamp_dialog(&ctx);
        self.sign_dialog(&ctx);
        self.properties_dialog(&ctx);
        self.protect_dialog(&ctx);
        self.unprotect_dialog(&ctx);
        self.text_prompt_dialog(&ctx);
        self.page_setup_dialog(&ctx);
        self.note_dialog(&ctx);
        self.password_dialog(&ctx);
        self.about_dialog(&ctx);
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
        EngineError::AlreadyExists(_) => "error.already_exists",
        EngineError::MissingGlyphs(_) => "error.missing_glyphs",
        EngineError::FontNotEmbeddable => "error.font_not_embeddable",
        EngineError::PasswordNotLatin1 => "error.password_not_latin1",
        EngineError::Other(_) => "error.unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            EngineError::AlreadyExists(PathBuf::new()),
            EngineError::MissingGlyphs(String::new()),
            EngineError::FontNotEmbeddable,
            EngineError::PasswordNotLatin1,
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
            include_str!("sign.rs")
        );
        let mut checked = 0;
        for key in src.split('"').filter(|s| {
            s.contains('.')
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
        }) {
            if !key.contains("..") && !key.ends_with(".rs") && key.split('.').all(|p| !p.is_empty())
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
        for (name, table) in [("field", FIELD_SHORTCUTS), ("global", SHORTCUTS)] {
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
        assert_eq!(FIELD_SHORTCUTS[0].2, Command::SaveAs);
        // Le scénario réel : Ctrl+Maj+S avec un champ focalisé donne « Enregistrer sous ».
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::S,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
            }],
            ..Default::default()
        };
        let mut seen = Vec::new();
        let mut out = ctx.run_ui(input, |ui| {
            ui.ctx().input_mut(|i| {
                for &(m, k, c) in FIELD_SHORTCUTS {
                    if i.consume_key(m, k) {
                        seen.push(c);
                    }
                }
            });
        });
        out.textures_delta.clear();
        assert_eq!(seen, vec![Command::SaveAs]);
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
