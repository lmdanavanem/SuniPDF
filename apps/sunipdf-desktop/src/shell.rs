//! Coquille de l'application : barre d'onglets, barre d'outils, panneau des outils,
//! page d'accueil, rail droit, palette de commandes, barre d'état.

use crate::app::{EngineState, SuniApp, Zoom, ZOOM_STEPS};
use crate::icons::{self, glyph};
use crate::theme::{self, Palette};
use egui::{Align, Button, Frame, Key, Layout, Margin, RichText, Stroke};
use std::path::{Path, PathBuf};
use sunipdf_core::{Command, Menu};
use sunipdf_i18n::{tr, tr_args};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolTab {
    All,
    Read,
    Annotate,
    EditTab,
    Pages,
    Convert,
    Security,
}

impl ToolTab {
    pub const ALL: [ToolTab; 7] = [
        ToolTab::All,
        ToolTab::Read,
        ToolTab::Annotate,
        ToolTab::EditTab,
        ToolTab::Pages,
        ToolTab::Convert,
        ToolTab::Security,
    ];

    fn label_key(self) -> &'static str {
        match self {
            Self::All => "toolbar.all_tools",
            Self::Read => "toolbar.read",
            Self::Annotate => "toolbar.annotate",
            Self::EditTab => "toolbar.edit",
            Self::Pages => "toolbar.pages",
            Self::Security => "toolbar.security",
            Self::Convert => "toolbar.convert",
        }
    }

    fn commands(self) -> &'static [Command] {
        match self {
            Self::All => &[
                Command::OpenDocument,
                Command::Find,
                Command::OrganizePages,
                Command::ToggleThumbnails,
                Command::RotateClockwise,
                Command::RotateCounterClockwise,
                Command::DeletePage,
                Command::InsertPages,
                Command::ToolHighlight,
                Command::ToolNote,
                Command::ToolSelectObject,
                Command::ToolAddText,
                Command::ToolAddImage,
                Command::ToggleComments,
                Command::ToggleBookmarks,
                Command::Properties,
                Command::Protect,
                Command::CombineFiles,
                Command::SplitDocument,
                Command::ExtractPages,
                Command::CreateFromImages,
                Command::ExportImages,
                Command::Optimize,
                Command::Undo,
                Command::Redo,
                Command::SaveAs,
                Command::CommandPalette,
            ],
            Self::Read => &[
                Command::Find,
                Command::ToggleBookmarks,
                Command::Properties,
                Command::ToolSelect,
                Command::CopyText,
                Command::ToggleThumbnails,
                Command::FitWidth,
                Command::FitPage,
                Command::ActualSize,
            ],
            Self::Annotate => &[
                Command::ToolSelect,
                Command::ToolHighlight,
                Command::ToolUnderline,
                Command::ToolStrikeOut,
                Command::ToolNote,
                Command::ToggleComments,
                Command::CopyText,
            ],
            Self::Convert => &[
                Command::CombineFiles,
                Command::CreateFromImages,
                Command::ExportImages,
                Command::Optimize,
            ],
            Self::Security => &[
                Command::Protect,
                Command::Unprotect,
                Command::StripMetadata,
                Command::Properties,
            ],
            Self::EditTab => &[
                Command::ToolSelectObject,
                Command::ToolAddText,
                Command::ToolAddImage,
                Command::ToolRectangle,
                Command::ToolEllipse,
                Command::ToolLine,
                Command::EditObjectText,
                Command::DuplicateObject,
                Command::RotateObjectLeft,
                Command::RotateObjectRight,
                Command::BringToFront,
                Command::SendToBack,
                Command::DeleteObject,
                Command::Undo,
                Command::Redo,
            ],
            Self::Pages => &[
                Command::OrganizePages,
                Command::SplitDocument,
                Command::ExtractPages,
                Command::RotateClockwise,
                Command::RotateCounterClockwise,
                Command::DeletePage,
                Command::InsertPages,
                Command::MovePageLeft,
                Command::MovePageRight,
                Command::Undo,
                Command::Redo,
                Command::SaveAs,
            ],
        }
    }
}

/// Icône associée à une commande (glyphe Fluent + repli texte).
/// Couleur de catégorie d'une commande (icônes du panneau d'outils, cartes d'accueil).
pub fn command_color(command: Command) -> egui::Color32 {
    use sunipdf_core::Menu;
    match command {
        Command::ToolSelect
        | Command::ToolHighlight
        | Command::ToolUnderline
        | Command::ToolStrikeOut
        | Command::ToolNote
        | Command::ToggleComments
        | Command::CopyText => theme::CAT_ANNOTATE,
        Command::ToolSelectObject
        | Command::ToolAddText
        | Command::ToolAddImage
        | Command::ToolRectangle
        | Command::ToolEllipse
        | Command::ToolLine
        | Command::EditObjectText
        | Command::DeleteObject
        | Command::DuplicateObject
        | Command::RotateObjectLeft
        | Command::RotateObjectRight
        | Command::BringToFront
        | Command::SendToBack => theme::CAT_EDIT,
        Command::Find | Command::ToggleBookmarks => theme::CAT_VIEW,
        _ => match command.info().menu {
            Menu::File => theme::CAT_FILE,
            Menu::Pages => theme::CAT_PAGES,
            Menu::Security => theme::CAT_SECURITY,
            Menu::Convert => theme::CAT_CONVERT,
            Menu::View => theme::CAT_VIEW,
            Menu::Edit | Menu::Help => theme::CAT_VIEW,
        },
    }
}

/// Rangée d'outil : pastille colorée + libellé, surlignage au survol.
pub fn tool_row(
    ui: &mut egui::Ui,
    has_icons: bool,
    glyph_fallback: (&str, &str),
    color: egui::Color32,
    label: &str,
    enabled: bool,
) -> egui::Response {
    let p = theme::palette(ui.visuals().dark_mode);
    let desired = egui::vec2(ui.available_width(), 36.0);
    let (rect, resp) = ui.allocate_exact_size(
        desired,
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    if enabled && (resp.hovered() || resp.has_focus()) {
        painter.rect_filled(rect, 8.0, p.hover);
    }
    let alpha = if enabled { 1.0 } else { 0.45 };
    let badge = egui::Rect::from_min_size(
        rect.min + egui::vec2(8.0, (rect.height() - 26.0) / 2.0),
        egui::vec2(26.0, 26.0),
    );
    painter.rect_filled(
        badge,
        7.0,
        color.gamma_multiply(if ui.visuals().dark_mode { 0.32 } else { 0.13 } * alpha),
    );
    let (g, f) = glyph_fallback;
    painter.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        if has_icons { g } else { f },
        if has_icons {
            egui::FontId::new(14.0, egui::FontFamily::Name(icons::ICONS.into()))
        } else {
            egui::FontId::proportional(14.0)
        },
        color.gamma_multiply(alpha),
    );
    painter.text(
        egui::pos2(badge.right() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        p.text.gamma_multiply(alpha),
    );
    resp
}

pub fn command_icon(command: Command) -> (&'static str, &'static str) {
    match command {
        Command::OpenDocument => (glyph::OPEN, "📂"),
        Command::CloseDocument => (glyph::CLOSE, "✕"),
        Command::Quit => (glyph::CLOSE, "✕"),
        Command::Save => (glyph::SAVE, "💾"),
        Command::SaveAs => (glyph::SAVE_AS, "💾"),
        Command::Find => (glyph::FIND, "🔍"),
        Command::ToggleThumbnails => (glyph::GRID, "▦"),
        Command::ToggleTools => (glyph::ALL_APPS, "▤"),
        Command::CommandPalette => (glyph::SEARCH_BOX, "⌘"),
        Command::RotateClockwise => (glyph::ROTATE, "↻"),
        Command::RotateCounterClockwise => (glyph::UNDO, "↺"),
        Command::DeletePage => (glyph::DELETE, "🗑"),
        Command::InsertPages => (glyph::OPEN, "⤵"),
        Command::MovePageLeft => (glyph::CHEVRON_UP, "◀"),
        Command::MovePageRight => (glyph::CHEVRON_DOWN, "▶"),
        Command::Undo => (glyph::UNDO, "↶"),
        Command::Redo => (glyph::ROTATE, "↷"),
        Command::OrganizePages => (glyph::GRID, "▦"),
        Command::FirstPage | Command::PreviousPage => (glyph::CHEVRON_UP, "▲"),
        Command::NextPage | Command::LastPage => (glyph::CHEVRON_DOWN, "▼"),
        Command::ZoomIn => (glyph::ZOOM_IN, "+"),
        Command::ZoomOut => (glyph::ZOOM_OUT, "−"),
        Command::ActualSize => (glyph::DOCUMENT, "1:1"),
        Command::FitWidth => (glyph::FIT_WIDTH, "↔"),
        Command::FitPage => (glyph::FIT_PAGE, "▭"),
        Command::CopyText => (glyph::DOCUMENT, "⎘"),
        Command::ToolSelect => (glyph::SELECT, "I"),
        Command::ToolHighlight => (glyph::HIGHLIGHT, "▬"),
        Command::ToolUnderline => (glyph::UNDERLINE, "_"),
        Command::ToolStrikeOut => (glyph::STRIKE, "S"),
        Command::ToolNote => (glyph::NOTE, "✎"),
        Command::ToggleComments => (glyph::COMMENT, "💬"),
        Command::ToggleBookmarks => (glyph::BOOKMARK, "🔖"),
        Command::Properties => (glyph::INFO, "i"),
        Command::Protect => (glyph::LOCK, "🔒"),
        Command::Unprotect => (glyph::UNLOCK, "🔓"),
        Command::StripMetadata => (glyph::ERASE, "⌫"),
        Command::CombineFiles => (glyph::COMBINE, "⧉"),
        Command::SplitDocument => (glyph::SPLIT, "⫶"),
        Command::ExtractPages => (glyph::EXTRACT, "⤴"),
        Command::CreateFromImages => (glyph::PICTURE, "🖼"),
        Command::ExportImages => (glyph::PICTURE, "🖼"),
        Command::Optimize => (glyph::OPTIMIZE, "⤓"),
        Command::NextTab | Command::PrevTab => (glyph::DOCUMENT, "⇥"),
        Command::ToolSelectObject => (glyph::CURSOR, "↖"),
        Command::ToolAddText => (glyph::FONT, "T"),
        Command::ToolAddImage => (glyph::PICTURE, "🖼"),
        Command::ToolRectangle => (glyph::RECT, "▭"),
        Command::ToolEllipse => (glyph::ELLIPSE, "◯"),
        Command::ToolLine => (glyph::LINE, "╱"),
        Command::DeleteObject => (glyph::DELETE, "🗑"),
        Command::EditObjectText => (glyph::FONT, "T"),
        Command::DuplicateObject => (glyph::DUPLICATE, "⧉"),
        Command::RotateObjectLeft => (glyph::ROTATE_LEFT, "↺"),
        Command::RotateObjectRight => (glyph::ROTATE, "↻"),
        Command::BringToFront => (glyph::FRONT, "⤒"),
        Command::SendToBack => (glyph::BACK, "⤓"),
        Command::About => (glyph::INFO, "i"),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl SuniApp {
    fn icon(&self, glyph: &str, fallback: &str, size: f32) -> RichText {
        icons::icon(self.has_icons, glyph, fallback, size)
    }

    fn icon_button(&self, ui: &mut egui::Ui, glyph: &str, fallback: &str, tooltip: &str) -> bool {
        ui.add(Button::new(self.icon(glyph, fallback, 16.0)).frame(false))
            .on_hover_text(tooltip)
            .clicked()
    }

    fn pal(&self) -> Palette {
        theme::palette(self.settings.dark_theme)
    }

    // ----- Rangée 1 : onglets de documents ----------------------------------

    pub fn tab_strip(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let home_selected = self.document.is_none();
            let home = ui
                .add(
                    Button::new(self.icon(glyph::HOME, "⌂", 15.0))
                        .selected(home_selected)
                        .min_size(egui::vec2(32.0, 28.0)),
                )
                .on_hover_text(tr(lang, "tabs.home"));
            if home.clicked() && !home_selected {
                // Accueil sans fermer : le document actif est suspendu dans son onglet.
                self.suspend_active();
                self.update_title(ctx);
            }
            let mut switch = None;
            let mut close = None;
            let active = self.tabs.active;
            for (i, tab) in self.tabs.tabs.iter().enumerate() {
                let is_active = active == Some(i);
                let name = file_name(&tab.path);
                let dirty = if is_active { self.dirty } else { tab.dirty };
                let title = if dirty { format!("{name} •") } else { name };
                let fill = if is_active {
                    p.surface
                } else {
                    egui::Color32::TRANSPARENT
                };
                let stroke = if is_active {
                    Stroke::new(1.0, p.border)
                } else {
                    Stroke::NONE
                };
                let resp = Frame::new()
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(6)
                    .inner_margin(Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(self.icon(glyph::DOCUMENT, "▤", 14.0).color(if is_active {
                                p.accent
                            } else {
                                p.text_muted
                            }));
                            ui.label(RichText::new(title).size(14.0).color(if is_active {
                                p.text
                            } else {
                                p.text_muted
                            }));
                            if self.icon_button(ui, glyph::CLOSE, "✕", tr(lang, "tabs.close")) {
                                close = Some(i);
                            }
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() && !is_active {
                    switch = Some(i);
                }
                if resp.hovered() && !is_active {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
            if let Some(i) = close {
                if active == Some(i) {
                    self.execute(ctx, Command::CloseDocument);
                } else {
                    switch = Some(i);
                }
            }
            if let Some(i) = switch {
                self.switch_to(i);
            }
            let open =
                ui.add(Button::new(format!("{}  {}", "+", tr(lang, "tabs.open"))).frame(false));
            if open.on_hover_text(tr(lang, "toolbar.open")).clicked() {
                self.execute(ctx, Command::OpenDocument);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
                let (g, f) = if self.settings.dark_theme {
                    (glyph::SUN, "☀")
                } else {
                    (glyph::MOON, "☾")
                };
                if self.icon_button(ui, g, f, tr(lang, "toolbar.theme")) {
                    let dark = !self.settings.dark_theme;
                    self.set_theme(ctx, dark);
                }
                if self.icon_button(ui, glyph::HELP, "?", tr(lang, "toolbar.help")) {
                    self.execute(ctx, Command::About);
                }
                ui.menu_button(self.icon(glyph::GLOBE, "🌐", 16.0), |ui| {
                    for l in sunipdf_i18n::Language::ALL {
                        if ui.radio(lang == l, l.native_name()).clicked() {
                            self.set_language(l);
                            self.update_title(ctx);
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text(tr(lang, "menu.settings.language"));
            });
        });
    }

    // ----- Rangée 2 : barre d'outils ---------------------------------------

    pub fn toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let menu = ui.menu_button(
                (
                    self.icon(glyph::MENU, "☰", 15.0),
                    RichText::new(format!("  {}", tr(lang, "toolbar.menu"))).size(14.0),
                ),
                |ui| {
                    ui.set_min_width(220.0);
                    self.command_menu(
                        ctx,
                        ui,
                        "menu.file",
                        Menu::File,
                        &[Command::Save, Command::Quit],
                    );
                    self.command_menu(ctx, ui, "menu.edit", Menu::Edit, &[]);
                    self.command_menu(ctx, ui, "menu.view", Menu::View, &[Command::FirstPage]);
                    self.command_menu(ctx, ui, "menu.pages", Menu::Pages, &[]);
                    self.command_menu(ctx, ui, "menu.convert", Menu::Convert, &[]);
                    self.command_menu(ctx, ui, "menu.security", Menu::Security, &[]);
                    ui.menu_button(tr(lang, "menu.settings"), |ui| {
                        ui.menu_button(tr(lang, "menu.settings.language"), |ui| {
                            for l in sunipdf_i18n::Language::ALL {
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
                    self.command_menu(ctx, ui, "menu.help", Menu::Help, &[]);
                },
            );
            menu.response.on_hover_text(tr(lang, "toolbar.menu_hint"));
            ui.add_space(6.0);
            ui.separator();
            for tab in ToolTab::ALL {
                let active = self.show_tools && self.tool_tab == tab;
                let text = if active {
                    icons::semibold(tr(lang, tab.label_key()), 14.0).color(p.text)
                } else {
                    RichText::new(tr(lang, tab.label_key()))
                        .size(14.0)
                        .color(p.text_muted)
                };
                let resp = ui.add(Button::new(text).frame(false));
                if active {
                    let r = resp.rect;
                    ui.painter()
                        .hline(r.x_range(), r.bottom() + 4.0, Stroke::new(2.0, p.accent));
                }
                if resp.clicked() {
                    self.tool_tab = tab;
                    self.show_tools = true;
                    self.settings.show_tools = true;
                    self.persist_settings();
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
                if self.document.is_some() {
                    let save = ui.add_enabled(
                        self.dirty && !self.saving,
                        Button::new(self.icon(glyph::SAVE, "💾", 16.0)).frame(false),
                    );
                    if save.on_hover_text(tr(lang, "menu.file.save")).clicked() {
                        self.execute(ctx, Command::Save);
                    }
                }
                if self.icon_button(ui, glyph::INFO, "i", tr(lang, "menu.help.about")) {
                    self.execute(ctx, Command::About);
                }
                let search = Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(16)
                    .inner_margin(Margin::symmetric(12, 5))
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label(self.icon(glyph::FIND, "🔍", 14.0).color(p.text_muted));
                            ui.label(
                                RichText::new(tr(lang, "toolbar.search_commands"))
                                    .color(p.text_muted)
                                    .size(13.0),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(tr(lang, "shortcut.palette"))
                                        .color(p.text_muted)
                                        .size(11.0),
                                );
                            });
                        });
                    })
                    .response;
                if search
                    .interact(egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.execute(ctx, Command::CommandPalette);
                }
            });
        });
    }

    // ----- Panneau gauche : outils ----------------------------------------

    pub fn tools_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, self.tool_tab.label_key()), 15.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.icon_button(ui, glyph::CLOSE, "✕", tr(lang, "tools.close")) {
                    self.show_tools = false;
                    self.settings.show_tools = false;
                    self.persist_settings();
                }
            });
        });
        ui.separator();
        ui.add_space(4.0);
        let has_doc = self.document.is_some();
        let mut run = None;
        for &command in self.tool_tab.commands() {
            let info = command.info();
            // Sans document, les outils restent actifs : ils ouvrent d'abord un fichier.
            let _ = has_doc;
            let enabled = match command {
                Command::Undo => self.can_undo,
                Command::Redo => self.can_redo,
                _ => true,
            };
            let resp = tool_row(
                ui,
                self.has_icons,
                command_icon(command),
                command_color(command),
                tr(lang, info.label_key),
                enabled,
            );
            let resp = match info.shortcut_key {
                Some(k) => resp.on_hover_text(tr(lang, k)),
                None => resp,
            };
            if resp.clicked() {
                run = Some(command);
            }
        }
        if let Some(command) = run {
            self.execute(ctx, command);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr(lang, "tools.hint"))
                .size(12.0)
                .color(p.text_muted),
        );
    }

    // ----- Accueil ---------------------------------------------------------

    pub fn home(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(40, 28))
                    .show(ui, |ui| {
                        ui.set_max_width(1100.0);
                        ui.label(icons::semibold(tr(lang, "home.welcome"), 28.0));
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(tr(lang, "home.tagline"))
                                .color(p.text_muted)
                                .size(15.0),
                        );
                        ui.add_space(18.0);
                        if matches!(self.engine, EngineState::Failed) {
                            Frame::new()
                                .fill(theme::ERROR.gamma_multiply(0.12))
                                .corner_radius(8)
                                .inner_margin(14)
                                .show(ui, |ui| {
                                    ui.colored_label(
                                        theme::ERROR,
                                        tr(lang, "error.engine_missing"),
                                    );
                                });
                            ui.add_space(12.0);
                        }

                        ui.label(icons::semibold(tr(lang, "home.recommended"), 16.0));
                        ui.add_space(8.0);
                        let cards: [(&str, &str, &str, &str, egui::Color32); 7] = [
                            (
                                glyph::READ,
                                "📖",
                                "home.card.read.title",
                                "home.card.read.desc",
                                theme::CAT_FILE,
                            ),
                            (
                                glyph::FIND,
                                "🔍",
                                "home.card.find.title",
                                "home.card.find.desc",
                                theme::CAT_VIEW,
                            ),
                            (
                                glyph::PAGES,
                                "▦",
                                "home.card.pages.title",
                                "home.card.pages.desc",
                                theme::CAT_PAGES,
                            ),
                            (
                                glyph::HIGHLIGHT,
                                "▬",
                                "home.card.annot.title",
                                "home.card.annot.desc",
                                theme::CAT_ANNOTATE,
                            ),
                            (
                                glyph::LOCK,
                                "🔒",
                                "home.card.security.title",
                                "home.card.security.desc",
                                theme::CAT_SECURITY,
                            ),
                            (
                                glyph::PICTURE,
                                "🖼",
                                "home.card.convert.title",
                                "home.card.convert.desc",
                                theme::CAT_CONVERT,
                            ),
                            (
                                glyph::FONT,
                                "T",
                                "home.card.edit.title",
                                "home.card.edit.desc",
                                theme::CAT_EDIT,
                            ),
                        ];
                        let mut open_clicked = false;
                        ui.horizontal_wrapped(|ui| {
                            for (g, f, title, desc, color) in cards {
                                Frame::new()
                                    .fill(p.surface)
                                    .stroke(Stroke::new(1.0, p.border))
                                    .corner_radius(12)
                                    .inner_margin(16)
                                    .shadow(egui::epaint::Shadow {
                                        offset: [0, 2],
                                        blur: 10,
                                        spread: 0,
                                        color: egui::Color32::from_black_alpha(
                                            if self.settings.dark_theme { 60 } else { 14 },
                                        ),
                                    })
                                    .show(ui, |ui| {
                                        ui.set_width(220.0);
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                let (badge, _) = ui.allocate_exact_size(
                                                    egui::vec2(36.0, 36.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_filled(
                                                    badge,
                                                    9.0,
                                                    color.gamma_multiply(
                                                        if self.settings.dark_theme {
                                                            0.32
                                                        } else {
                                                            0.13
                                                        },
                                                    ),
                                                );
                                                ui.painter().text(
                                                    badge.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    if self.has_icons { g } else { f },
                                                    if self.has_icons {
                                                        egui::FontId::new(
                                                            17.0,
                                                            egui::FontFamily::Name(
                                                                icons::ICONS.into(),
                                                            ),
                                                        )
                                                    } else {
                                                        egui::FontId::proportional(17.0)
                                                    },
                                                    color,
                                                );
                                                ui.add_space(2.0);
                                                ui.label(icons::semibold(tr(lang, title), 15.0));
                                            });
                                            ui.add_space(6.0);
                                            ui.label(
                                                RichText::new(tr(lang, desc))
                                                    .color(p.text_muted)
                                                    .size(13.0),
                                            );
                                            ui.add_space(8.0);
                                            if ui
                                                .add(
                                                    Button::new(
                                                        RichText::new(tr(lang, "home.card.use"))
                                                            .color(p.accent),
                                                    )
                                                    .frame(false),
                                                )
                                                .clicked()
                                            {
                                                open_clicked = true;
                                            }
                                        });
                                    });
                            }
                            Frame::new()
                                .fill(p.hover)
                                .corner_radius(10)
                                .inner_margin(16)
                                .show(ui, |ui| {
                                    ui.set_width(150.0);
                                    ui.vertical(|ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(6.0);
                                            ui.label(
                                                self.icon(glyph::FOLDER_OPEN, "📂", 26.0)
                                                    .color(p.accent),
                                            );
                                            ui.add_space(6.0);
                                            if ui
                                                .add(
                                                    Button::new(icons::semibold(
                                                        tr(lang, "home.open_file"),
                                                        14.0,
                                                    ))
                                                    .frame(false),
                                                )
                                                .clicked()
                                            {
                                                open_clicked = true;
                                            }
                                            ui.add_space(6.0);
                                        });
                                    });
                                });
                        });
                        if open_clicked {
                            self.execute(ctx, Command::OpenDocument);
                        }

                        ui.add_space(26.0);
                        ui.label(icons::semibold(tr(lang, "home.recent"), 18.0));
                        ui.add_space(6.0);
                        let recent: Vec<PathBuf> =
                            self.settings.recent.iter().map(PathBuf::from).collect();
                        if recent.is_empty() {
                            ui.label(
                                RichText::new(tr(lang, "home.recent.empty")).color(p.text_muted),
                            );
                        } else {
                            let mut to_open = None;
                            for path in recent {
                                let resp = ui.add(
                                    Button::new((
                                        self.icon(glyph::DOCUMENT, "▤", 15.0).color(p.accent),
                                        RichText::new(format!("  {}", file_name(&path))).size(14.0),
                                    ))
                                    .frame(false)
                                    .min_size(egui::vec2(ui.available_width().min(700.0), 30.0)),
                                );
                                if resp.on_hover_text(path.display().to_string()).clicked() {
                                    to_open = Some(path);
                                }
                            }
                            if let Some(path) = to_open {
                                self.open_path(path, None);
                            }
                        }

                        ui.add_space(26.0);
                        ui.label(
                            RichText::new(tr(lang, "home.privacy"))
                                .size(11.0)
                                .color(p.text_muted),
                        );
                        ui.label(tr(lang, "home.privacy.text"));
                        ui.add_space(20.0);
                    });
            });
    }

    // ----- Rail droit : panneaux, navigation, zoom --------------------------

    pub fn right_rail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        let (page, count) = self
            .document
            .as_ref()
            .map(|d| (d.page, d.info.page_count()))
            .unwrap_or((0, 0));
        ui.vertical_centered(|ui| {
            ui.add_space(6.0);
            let thumbs = ui
                .add(
                    Button::new(self.icon(glyph::GRID, "▦", 16.0))
                        .frame(false)
                        .selected(self.settings.show_thumbnails),
                )
                .on_hover_text(tr(lang, "menu.view.thumbnails"));
            if thumbs.clicked() {
                self.execute(ctx, Command::ToggleThumbnails);
            }
            let organize = ui
                .add(
                    Button::new(self.icon(glyph::PAGES, "▦", 16.0))
                        .frame(false)
                        .selected(self.organize.active),
                )
                .on_hover_text(tr(lang, "menu.view.organize"));
            if organize.clicked() {
                self.execute(ctx, Command::OrganizePages);
            }
            let bookmarks = ui
                .add(
                    Button::new(self.icon(glyph::BOOKMARK, "🔖", 16.0))
                        .frame(false)
                        .selected(self.docinfo.bookmarks_open),
                )
                .on_hover_text(tr(lang, "menu.view.bookmarks"));
            if bookmarks.clicked() {
                self.execute(ctx, Command::ToggleBookmarks);
            }
            let comments = ui
                .add(
                    Button::new(self.icon(glyph::COMMENT, "💬", 16.0))
                        .frame(false)
                        .selected(self.annotate.comments_open),
                )
                .on_hover_text(tr(lang, "menu.view.comments"));
            if comments.clicked() {
                self.execute(ctx, Command::ToggleComments);
            }
            let find = ui
                .add(
                    Button::new(self.icon(glyph::FIND, "🔍", 16.0))
                        .frame(false)
                        .selected(self.find.open),
                )
                .on_hover_text(tr(lang, "menu.edit.find"));
            if find.clicked() {
                if self.find.open {
                    self.close_find();
                } else {
                    self.execute(ctx, Command::Find);
                }
            }
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.add_space(8.0);
                if self.icon_button(ui, glyph::ZOOM_OUT, "−", tr(lang, "menu.view.zoom_out")) {
                    self.execute(ctx, Command::ZoomOut);
                }
                if self.icon_button(ui, glyph::ZOOM_IN, "+", tr(lang, "menu.view.zoom_in")) {
                    self.execute(ctx, Command::ZoomIn);
                }
                if self.icon_button(ui, glyph::ROTATE, "↻", tr(lang, "menu.pages.rotate_cw")) {
                    self.execute(ctx, Command::RotateClockwise);
                }
                let fit_w = ui
                    .add(
                        Button::new(self.icon(glyph::FIT_WIDTH, "↔", 16.0))
                            .frame(false)
                            .selected(self.zoom == Zoom::FitWidth),
                    )
                    .on_hover_text(tr(lang, "menu.view.fit_width"));
                if fit_w.clicked() {
                    self.execute(ctx, Command::FitWidth);
                }
                let fit_p = ui
                    .add(
                        Button::new(self.icon(glyph::FIT_PAGE, "▭", 16.0))
                            .frame(false)
                            .selected(self.zoom == Zoom::FitPage),
                    )
                    .on_hover_text(tr(lang, "menu.view.fit_page"));
                if fit_p.clicked() {
                    self.execute(ctx, Command::FitPage);
                }
                let zoom_label = format!("{}%", (self.effective_zoom * 100.0).round());
                egui::ComboBox::from_id_salt("rail-zoom")
                    .selected_text(RichText::new(zoom_label).size(12.0))
                    .width(64.0)
                    .show_ui(ui, |ui| {
                        for z in ZOOM_STEPS {
                            ui.selectable_value(
                                &mut self.zoom,
                                Zoom::Fixed(z),
                                format!("{} %", (z * 100.0).round()),
                            );
                        }
                    });
                ui.add_space(6.0);
                if self.icon_button(ui, glyph::CHEVRON_DOWN, "▼", tr(lang, "toolbar.next_page")) {
                    self.execute(ctx, Command::NextPage);
                }
                if self.icon_button(
                    ui,
                    glyph::CHEVRON_UP,
                    "▲",
                    tr(lang, "toolbar.previous_page"),
                ) {
                    self.execute(ctx, Command::PreviousPage);
                }
                ui.label(
                    RichText::new(count.to_string())
                        .size(12.0)
                        .color(p.text_muted),
                );
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.page_input)
                        .desired_width(34.0)
                        .horizontal_align(Align::Center)
                        .font(egui::TextStyle::Small),
                );
                field.clone().on_hover_text(tr(lang, "toolbar.page_field"));
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    match self.page_input.trim().parse::<usize>() {
                        Ok(n) if n >= 1 => self.go_to_page(n - 1),
                        _ => self.page_input = (page + 1).to_string(),
                    }
                }
            });
        });
    }

    // ----- Palette de commandes (Ctrl+K) ------------------------------------

    pub fn command_palette(&mut self, ctx: &egui::Context) {
        if !self.palette.open {
            return;
        }
        let lang = self.lang();
        let p = self.pal();
        let has_doc = self.document.is_some();
        let query = self.palette.query.to_lowercase();
        let matches: Vec<Command> = sunipdf_core::REGISTRY
            .iter()
            .filter(|c| !c.needs_document || has_doc)
            .filter(|c| query.is_empty() || tr(lang, c.label_key).to_lowercase().contains(&query))
            .map(|c| c.command)
            .collect();
        if self.palette.selected >= matches.len() {
            self.palette.selected = 0;
        }
        let mut chosen = None;
        let mut close = false;
        egui::Modal::new(egui::Id::new("palette")).show(ctx, |ui| {
            ui.set_width(520.0);
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.palette.query)
                    .hint_text(tr(lang, "palette.placeholder"))
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Heading),
            );
            if self.palette.focus_requested {
                field.request_focus();
                self.palette.focus_requested = false;
            }
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) && !matches.is_empty() {
                    self.palette.selected = (self.palette.selected + 1) % matches.len();
                }
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) && !matches.is_empty() {
                    self.palette.selected =
                        (self.palette.selected + matches.len() - 1) % matches.len();
                }
                if i.consume_key(egui::Modifiers::NONE, Key::Enter) {
                    chosen = matches.get(self.palette.selected).copied();
                }
                if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
                    close = true;
                }
            });
            ui.separator();
            if matches.is_empty() {
                ui.label(RichText::new(tr(lang, "palette.empty")).color(p.text_muted));
            }
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for (i, command) in matches.iter().enumerate() {
                        let info = command.info();
                        let (g, f) = command_icon(*command);
                        let selected = i == self.palette.selected;
                        let row = Frame::new()
                            .fill(if selected {
                                p.hover
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .corner_radius(6)
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(self.icon(g, f, 15.0).color(p.accent));
                                    ui.label(tr(lang, info.label_key));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if let Some(k) = info.shortcut_key {
                                            ui.label(
                                                RichText::new(tr(lang, k))
                                                    .size(12.0)
                                                    .color(p.text_muted),
                                            );
                                        }
                                    });
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                        if row.hovered() {
                            self.palette.selected = i;
                        }
                        if row.clicked() {
                            chosen = Some(*command);
                        }
                    }
                });
        });
        if let Some(command) = chosen {
            self.palette.open = false;
            self.palette.query.clear();
            self.execute(ctx, command);
        } else if close {
            self.palette.open = false;
            self.palette.query.clear();
        }
    }

    // ----- Barre d'état ------------------------------------------------------

    pub fn status_bar(&self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let muted = |t: String| RichText::new(t).size(12.0).color(p.text_muted);
            match (&self.engine, &self.document, &self.opening) {
                (EngineState::Starting, _, _) => {
                    ui.spinner();
                    ui.label(muted(tr(lang, "status.engine_starting").into()));
                }
                (EngineState::Failed, _, _) => {
                    ui.colored_label(theme::ERROR, tr(lang, "error.engine_load"));
                }
                (_, _, Some(path)) => {
                    ui.spinner();
                    ui.label(muted(tr_args(
                        lang,
                        "status.opening",
                        &[("name", &file_name(path))],
                    )));
                }
                (_, Some(doc), None) => {
                    ui.label(muted(tr_args(
                        lang,
                        "status.page_of",
                        &[
                            ("current", &(doc.page + 1).to_string()),
                            ("total", &doc.info.page_count().to_string()),
                        ],
                    )));
                    ui.separator();
                    ui.label(muted(tr_args(
                        lang,
                        "status.zoom",
                        &[(
                            "percent",
                            &format!("{}", (self.effective_zoom * 100.0).round()),
                        )],
                    )));
                    if self.in_flight.is_some() {
                        ui.separator();
                        ui.spinner();
                        ui.label(muted(tr(lang, "status.rendering").into()));
                    }
                    if let Some(key) = self.tools.running {
                        ui.separator();
                        ui.spinner();
                        ui.label(muted(tr(lang, key).into()));
                    }
                    if self.saving {
                        ui.separator();
                        ui.spinner();
                        ui.label(muted(tr(lang, "status.saving").into()));
                    } else if let Some(msg) = &self.status_msg {
                        ui.separator();
                        ui.label(RichText::new(msg).size(12.0).color(theme::SUCCESS));
                    }
                }
                (_, None, None) => {
                    if self.tabs.resuming.is_some() {
                        ui.spinner();
                        ui.label(muted(tr(lang, "tabs.switching").into()));
                    } else {
                        ui.label(muted(tr(lang, "status.ready").into()));
                    }
                }
            }
        });
    }
}
