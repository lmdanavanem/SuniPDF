//! État et interface de l'application. Aucune logique PDF ici : tout passe par `worker`.

use crate::settings::Settings;
use crate::theme;
use crate::worker::{Request, Response, Worker};
use egui::{Key, Modifiers};
use std::path::{Path, PathBuf};
use sunipdf_core::{DocumentInfo, EngineError};
use sunipdf_i18n::{tr, tr_args, Language};

/// Points CSS par point PDF : à 100 %, une page de 612 pt occupe 816 px logiques (96 dpi).
const SCREEN_PER_PDF_POINT: f32 = 96.0 / 72.0;
const ZOOM_STEPS: [f32; 12] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0,
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Zoom {
    FitWidth,
    FitPage,
    Fixed(f32),
}

enum EngineState {
    Starting,
    Ready,
    Failed,
}

struct OpenDocument {
    info: DocumentInfo,
    page: usize,
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

pub struct SuniApp {
    settings: Settings,
    worker: Worker,
    engine: EngineState,
    document: Option<OpenDocument>,
    opening: Option<PathBuf>,
    zoom: Zoom,
    effective_zoom: f32,
    texture: Option<PageTexture>,
    generation: u64,
    in_flight: Option<(usize, f32)>,
    last_rendered: Option<(usize, f32)>,
    page_input: String,
    error: Option<&'static str>,
    password_prompt: Option<PasswordPrompt>,
    show_about: bool,
    pending_file: Option<PathBuf>,
}

impl SuniApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        let settings = Settings::load();
        theme::apply(&cc.egui_ctx, settings.dark_theme);
        Self {
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
        }
    }

    fn lang(&self) -> Language {
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

    fn open_path(&mut self, path: PathBuf, password: Option<String>) {
        if !matches!(self.engine, EngineState::Ready) {
            self.pending_file = Some(path);
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
        if announce {
            self.error = None;
        }
    }

    fn go_to_page(&mut self, page: usize) {
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

    fn set_language(&mut self, language: Language) {
        if self.lang() != language {
            self.settings.set_language(language);
            self.persist_settings();
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context, dark: bool) {
        if self.settings.dark_theme != dark {
            self.settings.dark_theme = dark;
            theme::apply(ctx, dark);
            self.persist_settings();
        }
    }

    fn persist_settings(&self) {
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
                    self.document = Some(OpenDocument { info, page: 0 });
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

    fn update_title(&self, ctx: &egui::Context) {
        let title = match &self.document {
            Some(doc) => tr_args(
                self.lang(),
                "app.title_with_document",
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
        if self.password_prompt.is_some() || self.show_about {
            return;
        }
        let mut actions: Vec<fn(&mut SuniApp)> = Vec::new();
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::O) {
                actions.push(|a| a.open_file_dialog());
            }
            if i.consume_key(Modifiers::COMMAND, Key::W) {
                actions.push(|a| a.close_document(true));
            }
            if i.consume_key(Modifiers::COMMAND, Key::Plus)
                || i.consume_key(Modifiers::COMMAND, Key::Equals)
            {
                actions.push(|a| a.zoom_step(1));
            }
            if i.consume_key(Modifiers::COMMAND, Key::Minus) {
                actions.push(|a| a.zoom_step(-1));
            }
            if i.consume_key(Modifiers::COMMAND, Key::Num0) {
                actions.push(|a| a.zoom = Zoom::Fixed(1.0));
            }
            if i.consume_key(Modifiers::NONE, Key::PageDown)
                || i.consume_key(Modifiers::NONE, Key::ArrowRight)
            {
                actions.push(|a| a.step_page(1));
            }
            if i.consume_key(Modifiers::NONE, Key::PageUp)
                || i.consume_key(Modifiers::NONE, Key::ArrowLeft)
            {
                actions.push(|a| a.step_page(-1));
            }
            if i.consume_key(Modifiers::NONE, Key::Home) {
                actions.push(|a| a.go_to_page(0));
            }
            if i.consume_key(Modifiers::NONE, Key::End) {
                actions.push(|a| a.go_to_page(usize::MAX));
            }
        });
        for action in actions {
            action(self);
        }
    }

    // ----- Interface -------------------------------------------------------

    fn menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let has_doc = self.document.is_some();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(tr(lang, "menu.file"), |ui| {
                if menu_item(
                    ui,
                    tr(lang, "menu.file.open"),
                    tr(lang, "shortcut.open"),
                    true,
                ) {
                    self.open_file_dialog();
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.file.close"),
                    tr(lang, "shortcut.close"),
                    has_doc,
                ) {
                    self.close_document(true);
                }
                ui.separator();
                if menu_item(
                    ui,
                    tr(lang, "menu.file.quit"),
                    tr(lang, "shortcut.quit"),
                    true,
                ) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(tr(lang, "menu.view"), |ui| {
                if menu_item(
                    ui,
                    tr(lang, "menu.view.zoom_in"),
                    tr(lang, "shortcut.zoom_in"),
                    has_doc,
                ) {
                    self.zoom_step(1);
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.view.zoom_out"),
                    tr(lang, "shortcut.zoom_out"),
                    has_doc,
                ) {
                    self.zoom_step(-1);
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.view.actual_size"),
                    tr(lang, "shortcut.actual_size"),
                    has_doc,
                ) {
                    self.zoom = Zoom::Fixed(1.0);
                }
                if menu_item(ui, tr(lang, "menu.view.fit_width"), "", has_doc) {
                    self.zoom = Zoom::FitWidth;
                }
                if menu_item(ui, tr(lang, "menu.view.fit_page"), "", has_doc) {
                    self.zoom = Zoom::FitPage;
                }
                ui.separator();
                if menu_item(
                    ui,
                    tr(lang, "menu.view.first_page"),
                    tr(lang, "shortcut.first_page"),
                    has_doc,
                ) {
                    self.go_to_page(0);
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.view.previous_page"),
                    tr(lang, "shortcut.previous_page"),
                    has_doc,
                ) {
                    self.step_page(-1);
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.view.next_page"),
                    tr(lang, "shortcut.next_page"),
                    has_doc,
                ) {
                    self.step_page(1);
                }
                if menu_item(
                    ui,
                    tr(lang, "menu.view.last_page"),
                    tr(lang, "shortcut.last_page"),
                    has_doc,
                ) {
                    self.go_to_page(usize::MAX);
                }
            });
            ui.menu_button(tr(lang, "menu.settings"), |ui| {
                ui.menu_button(tr(lang, "menu.settings.language"), |ui| {
                    for l in Language::ALL {
                        if ui.radio(lang == l, l.native_name()).clicked() {
                            self.set_language(l);
                            self.update_title(ctx);
                            ui.close();
                        }
                    }
                });
                ui.menu_button(tr(lang, "menu.settings.theme"), |ui| {
                    let dark = self.settings.dark_theme;
                    if ui
                        .radio(!dark, tr(lang, "menu.settings.theme.light"))
                        .clicked()
                    {
                        self.set_theme(ctx, false);
                        ui.close();
                    }
                    if ui
                        .radio(dark, tr(lang, "menu.settings.theme.dark"))
                        .clicked()
                    {
                        self.set_theme(ctx, true);
                        ui.close();
                    }
                });
            });
            ui.menu_button(tr(lang, "menu.help"), |ui| {
                if ui.button(tr(lang, "menu.help.about")).clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let (has_doc, page, count) = match &self.document {
            Some(d) => (true, d.page, d.info.page_count()),
            None => (false, 0, 0),
        };
        ui.horizontal(|ui| {
            if ui
                .button("📂")
                .on_hover_text(tr(lang, "toolbar.open"))
                .clicked()
            {
                self.open_file_dialog();
            }
            ui.separator();
            if ui
                .add_enabled(has_doc && page > 0, egui::Button::new("◀"))
                .on_hover_text(tr(lang, "toolbar.previous_page"))
                .clicked()
            {
                self.step_page(-1);
            }
            let field = ui.add_enabled(
                has_doc,
                egui::TextEdit::singleline(&mut self.page_input)
                    .desired_width(44.0)
                    .horizontal_align(egui::Align::Center),
            );
            field.clone().on_hover_text(tr(lang, "toolbar.page_field"));
            if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                match self.page_input.trim().parse::<usize>() {
                    Ok(n) if n >= 1 => self.go_to_page(n - 1),
                    _ => self.page_input = (page + 1).to_string(),
                }
            }
            ui.label(format!("/ {count}"));
            if ui
                .add_enabled(has_doc && page + 1 < count, egui::Button::new("▶"))
                .on_hover_text(tr(lang, "toolbar.next_page"))
                .clicked()
            {
                self.step_page(1);
            }
            ui.separator();
            ui.add_enabled_ui(has_doc, |ui| {
                if ui
                    .button("−")
                    .on_hover_text(tr(lang, "menu.view.zoom_out"))
                    .clicked()
                {
                    self.zoom_step(-1);
                }
                let selected = match self.zoom {
                    Zoom::FitWidth => tr(lang, "toolbar.fit_width").to_string(),
                    Zoom::FitPage => tr(lang, "toolbar.fit_page").to_string(),
                    Zoom::Fixed(_) => format!("{} %", (self.effective_zoom * 100.0).round()),
                };
                egui::ComboBox::from_id_salt("zoom")
                    .selected_text(selected)
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.zoom,
                            Zoom::FitWidth,
                            tr(lang, "menu.view.fit_width"),
                        );
                        ui.selectable_value(
                            &mut self.zoom,
                            Zoom::FitPage,
                            tr(lang, "menu.view.fit_page"),
                        );
                        for z in ZOOM_STEPS {
                            ui.selectable_value(
                                &mut self.zoom,
                                Zoom::Fixed(z),
                                format!("{} %", (z * 100.0).round()),
                            );
                        }
                    });
                if ui
                    .button("+")
                    .on_hover_text(tr(lang, "menu.view.zoom_in"))
                    .clicked()
                {
                    self.zoom_step(1);
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        let lang = self.lang();
        ui.horizontal(|ui| match (&self.engine, &self.document, &self.opening) {
            (EngineState::Starting, _, _) => {
                ui.spinner();
                ui.label(tr(lang, "status.engine_starting"));
            }
            (EngineState::Failed, _, _) => {
                ui.colored_label(theme::ERROR, tr(lang, "error.engine_load"));
            }
            (_, _, Some(path)) => {
                ui.spinner();
                ui.label(tr_args(
                    lang,
                    "status.opening",
                    &[("name", &file_name(path))],
                ));
            }
            (_, Some(doc), None) => {
                ui.label(tr_args(
                    lang,
                    "status.page_of",
                    &[
                        ("current", &(doc.page + 1).to_string()),
                        ("total", &doc.info.page_count().to_string()),
                    ],
                ));
                ui.separator();
                ui.label(tr_args(
                    lang,
                    "status.zoom",
                    &[(
                        "percent",
                        &format!("{}", (self.effective_zoom * 100.0).round()),
                    )],
                ));
                if self.in_flight.is_some() {
                    ui.separator();
                    ui.spinner();
                    ui.label(tr(lang, "status.rendering"));
                }
            }
            (_, None, None) => {
                ui.label(tr(lang, "status.ready"));
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
            });
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

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(&ctx, ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        let canvas = egui::Frame::new().fill(theme::canvas_bg(self.settings.dark_theme));
        egui::CentralPanel::default().frame(canvas).show(ui, |ui| {
            self.error_banner(ui);
            self.document_area(&ctx, ui);
        });
        self.password_dialog(&ctx);
        self.about_dialog(&ctx);
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
    fn zoom_quantization_and_pdf_detection() {
        assert_eq!(quantize(1.3333), 1.335);
        assert!(is_pdf(Path::new("a/b.PDF")));
        assert!(!is_pdf(Path::new("a/b.txt")));
    }
}
