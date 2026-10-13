//! État et interface de l'application. Aucune logique PDF ici : tout passe par `worker`.

use crate::settings::Settings;
use crate::theme;
use crate::worker::{Request, Response, Worker};
use egui::{Key, Modifiers};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use sunipdf_core::{Command, DocumentInfo, EngineError, Menu};
use sunipdf_i18n::{tr, tr_args, Language};

/// Points CSS par point PDF : à 100 %, une page de 612 pt occupe 816 px logiques (96 dpi).
const SCREEN_PER_PDF_POINT: f32 = 96.0 / 72.0;
pub const ZOOM_STEPS: [f32; 12] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0,
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

struct PageTexture {
    handle: egui::TextureHandle,
    page: usize,
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
    Open(PathBuf),
}

enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

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

pub struct SuniApp {
    pub settings: Settings,
    worker: Worker,
    pub engine: EngineState,
    pub document: Option<OpenDocument>,
    pub opening: Option<PathBuf>,
    pub zoom: Zoom,
    pub effective_zoom: f32,
    texture: Option<PageTexture>,
    generation: u64,
    pub in_flight: Option<(usize, f32)>,
    last_rendered: Option<(usize, f32)>,
    pub page_input: String,
    error: Option<&'static str>,
    password_prompt: Option<PasswordPrompt>,
    show_about: bool,
    pending_file: Option<PathBuf>,
    thumbnails: HashMap<usize, egui::TextureHandle>,
    thumbs_pending: HashSet<usize>,
    thumb_px: u32,
    pub find: FindState,
    pub dirty: bool,
    pub saving: bool,
    allow_close: bool,
    unsaved_prompt: Option<PendingAction>,
    after_save: Option<PendingAction>,
    pub status_msg: Option<String>,
    pub has_icons: bool,
    pub show_tools: bool,
    pub tool_tab: crate::shell::ToolTab,
    pub palette: PaletteState,
}

impl SuniApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        let settings = Settings::load();
        let has_icons = crate::icons::install(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.dark_theme);
        Self {
            has_icons,
            show_tools: settings.show_tools,
            tool_tab: crate::shell::ToolTab::All,
            palette: PaletteState {
                open: false,
                query: String::new(),
                selected: 0,
                focus_requested: false,
            },
            worker: Worker::spawn(cc.egui_ctx.clone()),
            settings,
            engine: EngineState::Starting,
            document: None,
            opening: None,
            zoom: Zoom::FitWidth,
            effective_zoom: 1.0,
            texture: None,
            generation: 0,
            in_flight: None,
            last_rendered: None,
            page_input: String::new(),
            error: None,
            password_prompt: None,
            show_about: false,
            pending_file: initial_file,
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

    fn t(&self, key: &str) -> &'static str {
        tr(self.lang(), key)
    }

    // ----- Actions ---------------------------------------------------------

    fn open_file_dialog(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter(self.t("dialog.open.filter_pdf"), &["pdf"]);
        if let Some(path) = dialog.pick_file() {
            self.open_path(path, None);
        }
    }

    pub fn open_path(&mut self, path: PathBuf, password: Option<String>) {
        if !matches!(self.engine, EngineState::Ready) {
            self.pending_file = Some(path);
            return;
        }
        if self.dirty && self.unsaved_prompt.is_none() {
            self.unsaved_prompt = Some(PendingAction::Open(path));
            return;
        }
        self.error = None;
        self.close_document(false);
        self.opening = Some(path.clone());
        self.worker.send(Request::Open { path, password });
    }

    fn close_document(&mut self, announce: bool) {
        if self.document.take().is_some() {
            self.worker.send(Request::Close);
        }
        self.texture = None;
        self.in_flight = None;
        self.last_rendered = None;
        self.generation += 1;
        self.page_input.clear();
        self.thumbnails.clear();
        self.thumbs_pending.clear();
        self.close_find();
        self.dirty = false;
        self.saving = false;
        self.status_msg = None;
        if announce {
            self.error = None;
        }
    }

    pub fn go_to_page(&mut self, page: usize) {
        if let Some(doc) = &mut self.document {
            let clamped = page.min(doc.info.page_count().saturating_sub(1));
            if clamped != doc.page {
                doc.page = clamped;
                self.error = None;
            }
            self.page_input = (clamped + 1).to_string();
        }
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
        let current = self.effective_zoom;
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
                    if let Some(path) = self.pending_file.take() {
                        self.open_path(path, None);
                    }
                }
                Response::EngineFailed(e) => {
                    self.error = Some(error_key(&e));
                    log::error!("moteur indisponible : {e}");
                    self.engine = EngineState::Failed;
                }
                Response::Opened(info) => {
                    self.opening = None;
                    self.password_prompt = None;
                    self.page_input = "1".into();
                    self.settings.push_recent(&info.path);
                    self.persist_settings();
                    self.document = Some(OpenDocument { info, page: 0 });
                    self.dirty = false;
                    self.status_msg = None;
                    self.update_title(ctx);
                }
                Response::OpenFailed { path, error } => {
                    self.opening = None;
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
                Response::Rendered { generation, image } => {
                    if let Some(inflight) = self.in_flight.take() {
                        if generation == self.generation {
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [image.width as usize, image.height as usize],
                                &image.rgba,
                            );
                            self.texture = Some(PageTexture {
                                handle: ctx.load_texture(
                                    "sunipdf-page",
                                    color,
                                    egui::TextureOptions::LINEAR,
                                ),
                                page: image.page_index,
                            });
                            self.last_rendered = Some(inflight);
                        }
                    }
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
                Response::DocumentChanged(info) => {
                    if let Some(doc) = &mut self.document {
                        doc.info = info;
                        doc.page = doc.page.min(doc.info.page_count().saturating_sub(1));
                        self.page_input = (doc.page + 1).to_string();
                    }
                    self.texture = None;
                    self.last_rendered = None;
                    self.in_flight = None;
                    self.generation += 1;
                    self.thumbnails.clear();
                    self.thumbs_pending.clear();
                    self.find.searched = None;
                    self.find.matches.clear();
                    self.dirty = true;
                    self.status_msg = None;
                    self.update_title(ctx);
                }
                Response::OperationFailed(e) => {
                    log::warn!("opération impossible : {e}");
                    self.error = Some(error_key(&e));
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
                        if let Some(first) = self.find.matches.first() {
                            let page = first.page;
                            self.go_to_page(page);
                        }
                    }
                }
                Response::RenderFailed { generation, error } => {
                    self.in_flight = None;
                    if generation == self.generation {
                        log::warn!("rendu impossible : {error}");
                        self.error = Some(error_key(&error));
                        self.last_rendered = None;
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

    /// Demande un rendu si la page ou l'échelle souhaitée a changé et qu'aucun rendu n'est en cours.
    fn request_render_if_needed(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.document else { return };
        let scale = self.effective_zoom * SCREEN_PER_PDF_POINT * ctx.pixels_per_point();
        let wanted = (doc.page, quantize(scale));
        if self.in_flight.is_some() || self.last_rendered == Some(wanted) {
            return;
        }
        self.generation += 1;
        self.in_flight = Some(wanted);
        self.worker.send(Request::Render {
            page: wanted.0,
            scale: wanted.1,
            generation: self.generation,
        });
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
        if self.password_prompt.is_some() || self.show_about || self.unsaved_prompt.is_some() {
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
        let mut commands = Vec::new();
        ctx.input_mut(|i| {
            let mut key = |m: Modifiers, k: Key, c: Command| {
                if i.consume_key(m, k) {
                    commands.push(c);
                }
            };
            key(Modifiers::COMMAND, Key::O, Command::OpenDocument);
            key(Modifiers::COMMAND, Key::W, Command::CloseDocument);
            key(Modifiers::COMMAND, Key::Plus, Command::ZoomIn);
            key(Modifiers::COMMAND, Key::Equals, Command::ZoomIn);
            key(Modifiers::COMMAND, Key::Minus, Command::ZoomOut);
            key(Modifiers::COMMAND, Key::Num0, Command::ActualSize);
            key(Modifiers::NONE, Key::PageDown, Command::NextPage);
            key(Modifiers::NONE, Key::ArrowRight, Command::NextPage);
            key(Modifiers::NONE, Key::PageUp, Command::PreviousPage);
            key(Modifiers::NONE, Key::ArrowLeft, Command::PreviousPage);
            key(Modifiers::NONE, Key::Home, Command::FirstPage);
            key(Modifiers::NONE, Key::End, Command::LastPage);
            key(Modifiers::NONE, Key::F4, Command::ToggleThumbnails);
            key(Modifiers::COMMAND, Key::F, Command::Find);
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::S,
                Command::SaveAs,
            );
            key(Modifiers::COMMAND, Key::S, Command::Save);
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::R,
                Command::RotateCounterClockwise,
            );
            key(Modifiers::COMMAND, Key::R, Command::RotateClockwise);
            key(Modifiers::COMMAND, Key::Delete, Command::DeletePage);
            key(Modifiers::NONE, Key::F9, Command::ToggleTools);
            key(Modifiers::COMMAND, Key::K, Command::CommandPalette);
        });
        for command in commands {
            self.execute(ctx, command);
        }
    }

    /// Point d'entrée unique de toutes les commandes (menus, clavier, future palette / automatisation).
    pub fn execute(&mut self, ctx: &egui::Context, command: Command) {
        if command.info().needs_document && self.document.is_none() {
            return;
        }
        match command {
            Command::OpenDocument => self.open_file_dialog(),
            Command::CloseDocument => {
                if self.dirty {
                    self.unsaved_prompt = Some(PendingAction::Close);
                } else {
                    self.close_document(true);
                }
            }
            Command::Quit => {
                if self.dirty {
                    self.unsaved_prompt = Some(PendingAction::Quit);
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
                let page = self.document.as_ref().map(|d| d.page).unwrap_or(0);
                let delta_degrees = if command == Command::RotateClockwise {
                    90
                } else {
                    -90
                };
                self.worker.send(Request::Rotate {
                    page,
                    delta_degrees,
                });
            }
            Command::DeletePage => {
                let Some(doc) = &self.document else { return };
                if doc.info.page_count() <= 1 {
                    self.error = Some("error.last_page");
                } else {
                    self.worker.send(Request::DeletePage { page: doc.page });
                }
            }
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
                if menu_item(ui, tr(lang, info.label_key), shortcut, enabled) {
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
                    ui.colored_label(theme::ERROR, format!("⚠ {}", tr(lang, key)));
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
        let Some(doc) = &self.document else {
            if matches!(self.engine, EngineState::Failed) {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(theme::ERROR, tr(lang, "error.engine_missing"))
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
        };
        let Ok(size) = doc.info.page_sizes.get(doc.page).copied().ok_or(()) else {
            return;
        };
        let page_w = size.width_pt * SCREEN_PER_PDF_POINT;
        let page_h = size.height_pt * SCREEN_PER_PDF_POINT;
        let margin = 16.0;
        let avail = ui.available_size() - egui::vec2(2.0 * margin, 2.0 * margin);
        self.effective_zoom = match self.zoom {
            Zoom::FitWidth => (avail.x / page_w).max(0.05),
            Zoom::FitPage => (avail.x / page_w).min(avail.y / page_h).max(0.05),
            Zoom::Fixed(z) => z,
        };
        let display = egui::vec2(page_w, page_h) * self.effective_zoom;
        let page_index = doc.page;
        let total = doc.info.page_count();
        let name = file_name(&doc.info.path);
        self.request_render_if_needed(ctx);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let outer = egui::vec2(display.x + 2.0 * margin, display.y + 2.0 * margin)
                    .max(ui.available_size());
                let (rect, _) = ui.allocate_exact_size(outer, egui::Sense::hover());
                let page_rect = egui::Rect::from_center_size(
                    rect.center()
                        .max(rect.min + display / 2.0 + egui::vec2(margin, margin)),
                    display,
                );
                let painter = ui.painter();
                painter.rect_filled(
                    page_rect.translate(egui::vec2(3.0, 3.0)),
                    2.0,
                    egui::Color32::from_black_alpha(60),
                );
                painter.rect_filled(page_rect, 0.0, egui::Color32::WHITE);
                let painter = painter.clone();
                match &self.texture {
                    Some(tex) if tex.page == page_index => {
                        let alt = tr_args(
                            lang,
                            "a11y.page_image",
                            &[
                                ("current", &(page_index + 1).to_string()),
                                ("total", &total.to_string()),
                                ("name", &name),
                            ],
                        );
                        ui.put(
                            page_rect,
                            egui::Image::new(egui::load::SizedTexture::new(
                                tex.handle.id(),
                                display,
                            ))
                            .alt_text(alt),
                        );
                    }
                    _ => {
                        ui.put(page_rect, egui::Spinner::new().size(32.0));
                    }
                }
                self.paint_matches(&painter, page_rect, page_index, size.height_pt);
            });
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_h, count, |ui, rows| {
                for page in rows {
                    let ratio = sizes[page].height_pt / sizes[page].width_pt;
                    let size = egui::vec2(thumb_w, thumb_w * ratio);
                    if !self.thumbnails.contains_key(&page) && self.thumbs_pending.insert(page) {
                        self.worker.send(Request::Thumbnail { page, width_px: px });
                    }
                    ui.vertical_centered(|ui| {
                        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
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
        if let Some(page) = clicked {
            self.go_to_page(page);
        }
    }

    fn find_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
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
                    ui.colored_label(theme::ERROR, tr(lang, "find.none"));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("✕")
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
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.close_find();
            return;
        }
        self.request_search_if_needed();
        if step != 0 && !self.find.matches.is_empty() {
            let n = self.find.matches.len() as isize;
            self.find.current = ((self.find.current as isize + step).rem_euclid(n)) as usize;
            let page = self.find.matches[self.find.current].page;
            self.go_to_page(page);
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
    fn paint_matches(
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
        for (i, m) in self.find.matches.iter().enumerate() {
            if m.page != page_index {
                continue;
            }
            let color = if i == self.find.current {
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
        egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr(lang, "dialog.unsaved.title"));
            ui.label(tr_args(lang, "dialog.unsaved.text", &[("name", &name)]));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "dialog.unsaved.save")).clicked() {
                    choice = Some(UnsavedChoice::Save);
                }
                if ui.button(tr(lang, "dialog.unsaved.discard")).clicked() {
                    choice = Some(UnsavedChoice::Discard);
                }
                if ui.button(tr(lang, "dialog.unsaved.cancel")).clicked() {
                    choice = Some(UnsavedChoice::Cancel);
                }
            });
        });
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
            PendingAction::Close => self.close_document(true),
            PendingAction::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            PendingAction::Open(path) => self.open_path(path, None),
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
        egui::Modal::new(egui::Id::new("password")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(tr(lang, "dialog.password.title"));
            ui.label(file_name(&prompt.path));
            ui.label(tr(
                lang,
                if prompt.wrong {
                    "error.password_wrong"
                } else {
                    "error.password_required"
                },
            ));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(tr(lang, "dialog.password.label"));
                let field = ui.add(
                    egui::TextEdit::singleline(&mut prompt.input)
                        .password(true)
                        .desired_width(220.0),
                );
                field.request_focus();
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    submit = Some((prompt.path.clone(), prompt.input.clone()));
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "dialog.password.ok")).clicked() {
                    submit = Some((prompt.path.clone(), prompt.input.clone()));
                }
                if ui.button(tr(lang, "dialog.password.cancel")).clicked() {
                    cancel = true;
                }
            });
        });
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
        egui::Modal::new(egui::Id::new("about")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr(lang, "about.title"));
            ui.label(tr_args(
                lang,
                "about.version",
                &[("version", env!("CARGO_PKG_VERSION"))],
            ));
            ui.label(tr(lang, "about.description"));
            ui.label(tr(lang, "about.engine"));
            ui.add_space(8.0);
            if ui.button(tr(lang, "about.close")).clicked() {
                self.show_about = false;
            }
        });
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
        if self.document.is_some() {
            egui::Panel::right("rail")
                .resizable(false)
                .default_size(48.0)
                .frame(egui::Frame::new().fill(p.surface).inner_margin(4))
                .show(ui, |ui| self.right_rail(&ctx, ui));
            if self.settings.show_thumbnails {
                egui::Panel::left("thumbnails")
                    .resizable(true)
                    .default_size(170.0)
                    .frame(egui::Frame::new().fill(p.chrome).inner_margin(6))
                    .show(ui, |ui| self.thumbnails_panel(&ctx, ui));
            }
        }
        if self.document.is_some() {
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
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.unsaved_prompt = Some(PendingAction::Quit);
        }
        self.unsaved_dialog(&ctx);
        self.password_dialog(&ctx);
        self.about_dialog(&ctx);
        self.command_palette(&ctx);
    }
}

// ----- Utilitaires -----------------------------------------------------------

fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    let clicked = ui
        .add_enabled(enabled, egui::Button::new(label).shortcut_text(shortcut))
        .clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// Évite de relancer un rendu pour des variations d'échelle imperceptibles (redimensionnements).
fn quantize(scale: f32) -> f32 {
    (scale * 200.0).round() / 200.0
}

fn error_key(e: &EngineError) -> &'static str {
    match e {
        EngineError::LibraryNotFound(_) => "error.engine_missing",
        EngineError::LibraryLoad(_) => "error.engine_load",
        EngineError::Io(_) => "error.io",
        EngineError::PasswordRequired => "error.password_required",
        EngineError::InvalidFormat => "error.invalid_format",
        EngineError::SecurityRestricted => "error.security",
        EngineError::PageOutOfRange(_) => "error.page_range",
        EngineError::RenderTooLarge => "error.render_too_large",
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
        let src = concat!(include_str!("app.rs"), include_str!("main.rs"));
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

    #[test]
    fn zoom_quantization_and_pdf_detection() {
        assert_eq!(quantize(1.3333), 1.335);
        assert!(is_pdf(Path::new("a/b.PDF")));
        assert!(!is_pdf(Path::new("a/b.txt")));
    }
}
