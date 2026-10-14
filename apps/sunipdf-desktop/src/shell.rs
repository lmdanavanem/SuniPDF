//! Coquille de l'application : barre d'onglets, barre d'outils, panneau « Tous les outils »,
//! page d'accueil, rail droit, palette de commandes, barre d'état.

use crate::app::{EngineState, SuniApp, Zoom, ZOOM_STEPS};
use crate::icons::{self, glyph};
use crate::theme::{self, Palette};
use crate::workspace::{bar_button, BarButton, Entry, Workspace, WsInfo};
use egui::{Align, Button, Color32, Frame, Key, Layout, Margin, RichText, Stroke};
use std::path::{Path, PathBuf};
use sunipdf_core::{Command, Menu};
use sunipdf_i18n::{tr, tr_args};

/// Couleur d'une commande : celle de son outil dans le hub (panneau, barre, palette Ctrl+K) ;
/// commandes système en bleu (fichier) ou bleu ciel (vue, navigation).
pub fn command_color(command: Command) -> Color32 {
    match Workspace::owner_of(command).or_else(|| Workspace::from_command(command)) {
        Some(ws) => ws.info().color,
        None if command.info().menu == Menu::File => theme::CAT_FILE,
        None => theme::CAT_VIEW,
    }
}

/// Pastille d'outil peinte dans `rect` : fond teinté, glyphe contrasté (`theme::glyph_color`).
pub fn paint_badge(
    painter: &egui::Painter,
    rect: egui::Rect,
    has_icons: bool,
    icon: (&str, &str),
    color: Color32,
    dark: bool,
) {
    let size = rect.height();
    painter.rect_filled(rect, (size * 0.27) as u8, theme::badge_fill(color, dark));
    let font = (size * 0.52).round();
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        if has_icons { icon.0 } else { icon.1 },
        if has_icons {
            egui::FontId::new(font, egui::FontFamily::Name(icons::ICONS.into()))
        } else {
            egui::FontId::proportional(font)
        },
        theme::glyph_color(color, dark),
    );
}

/// Pastille d'outil de `size` px, allouée dans le flux.
pub fn badge(
    ui: &mut egui::Ui,
    has_icons: bool,
    icon: (&str, &str),
    color: Color32,
    size: f32,
    dark: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_badge(ui.painter(), rect, has_icons, icon, color, dark);
    }
}

/// Ligne d'outil du panneau « Tous les outils » : pastille colorée, nom, chevron pour les outils
/// à barre ; outil actif = fond sélectionné, trait de sa couleur à gauche, nom en semi-gras.
pub fn tool_row(
    ui: &mut egui::Ui,
    has_icons: bool,
    info: &WsInfo,
    label: &str,
    desc: &str,
    active: bool,
) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let p = theme::palette(dark);
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label)
    });
    let resp = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(desc.to_string());
    if ui.is_rect_visible(rect) {
        let fade = theme::hover_fade(
            ui,
            resp.id.with("hover"),
            resp.hovered() && !active,
            p.hover,
        );
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            theme::radius::ROW,
            if active { p.accent_soft } else { fade },
        );
        if active {
            painter.rect_filled(
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(0.0, 9.0),
                    egui::vec2(3.0, rect.height() - 18.0),
                ),
                2.0,
                info.color,
            );
        }
        let badge =
            egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 6.0), egui::vec2(28.0, 28.0));
        paint_badge(painter, badge, has_icons, info.icon, info.color, dark);
        painter.text(
            egui::pos2(badge.right() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(
                14.0,
                if active {
                    egui::FontFamily::Name(icons::SEMIBOLD.into())
                } else {
                    egui::FontFamily::Proportional
                },
            ),
            p.text,
        );
        if matches!(info.entry, Some(Entry::Mode(_))) {
            let (g, f) = (glyph::CHEVRON_RIGHT, "›");
            painter.text(
                egui::pos2(rect.right() - 16.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                if has_icons { g } else { f },
                if has_icons {
                    egui::FontId::new(11.0, egui::FontFamily::Name(icons::ICONS.into()))
                } else {
                    egui::FontId::proportional(14.0)
                },
                p.text_muted,
            );
        }
        if resp.has_focus() {
            theme::focus_ring(painter, rect, &p, theme::radius::ROW);
        }
    }
    resp
}

/// Icône associée à une commande (glyphe Fluent + repli texte).
pub fn command_icon(command: Command) -> (&'static str, &'static str) {
    match command {
        Command::OpenDocument => (glyph::FOLDER_OPEN, "📂"),
        Command::CloseDocument => (glyph::CLOSE, "✕"),
        Command::Quit => (glyph::CLOSE, "✕"),
        Command::Save => (glyph::SAVE, "💾"),
        Command::SaveAs => (glyph::SAVE_AS, "💾"),
        Command::Find => (glyph::FIND, "🔍"),
        Command::ToggleThumbnails => (glyph::GRID, "▦"),
        Command::ToggleTools => (glyph::ALL_APPS, "▤"),
        Command::CommandPalette => (glyph::SEARCH_BOX, "⌘"),
        Command::RotateClockwise => (glyph::ROTATE_RIGHT, "↻"),
        Command::RotateCounterClockwise => (glyph::ROTATE_LEFT, "↺"),
        Command::DeletePage => (glyph::DELETE, "🗑"),
        Command::InsertPages => (glyph::IMPORT, "⤵"),
        Command::MovePageLeft => (glyph::CHEVRON_LEFT, "◀"),
        Command::MovePageRight => (glyph::CHEVRON_RIGHT, "▶"),
        Command::Undo => (glyph::UNDO, "↶"),
        Command::Redo => (glyph::REDO, "↷"),
        Command::OrganizePages => (glyph::GRID, "▦"),
        Command::FirstPage | Command::PreviousPage => (glyph::CHEVRON_UP, "▲"),
        Command::NextPage | Command::LastPage => (glyph::CHEVRON_DOWN, "▼"),
        Command::ZoomIn => (glyph::ZOOM_IN, "+"),
        Command::ZoomOut => (glyph::ZOOM_OUT, "−"),
        Command::ActualSize => (glyph::DOCUMENT, "1:1"),
        Command::FitWidth => (glyph::FIT_WIDTH, "↔"),
        Command::FitPage => (glyph::FIT_PAGE, "▭"),
        Command::ViewContinuous => (glyph::SCROLL, "⇕"),
        Command::ViewSinglePage => (glyph::DOCUMENT, "▯"),
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
        Command::EditObjectText => (glyph::RENAME, "T"),
        Command::DuplicateObject => (glyph::DUPLICATE, "⧉"),
        Command::RotateObjectLeft => (glyph::ROTATE_LEFT, "↺"),
        Command::RotateObjectRight => (glyph::ROTATE_RIGHT, "↻"),
        Command::BringToFront => (glyph::FRONT, "⤒"),
        Command::SendToBack => (glyph::BACK, "⤓"),
        Command::Watermark => (glyph::STAMP, "◈"),
        Command::PageNumbers => (glyph::NUMBERS, "#"),
        Command::HubEdit
        | Command::HubOrganize
        | Command::HubComment
        | Command::HubProtect
        | Command::HubStamp => {
            Workspace::from_command(command).map_or((glyph::ALL_APPS, "▤"), |w| w.info().icon)
        }
        Command::HubLeave => (glyph::CLOSE, "✕"),
        Command::About => (glyph::INFO, "i"),
        Command::ToggleObjects => (glyph::LAYERS, "☰"),
        Command::CopyObjects => (glyph::DUPLICATE, "⎘"),
        Command::CutObjects => (glyph::CUT, "✂"),
        Command::PasteObjects => (glyph::PASTE, "📋"),
        Command::SelectAllObjects => (glyph::SELECT_ALL, "▣"),
        Command::ReplaceImage => (glyph::PHOTO, "🖼"),
        Command::CropPages => (glyph::CROP, "⌗"),
        Command::ResizePages => (glyph::FIT_PAGE, "▭"),
        Command::InsertBlankPage => (glyph::ADD, "+"),
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

    /// Bouton d'accès rapide de la barre d'outils (icône seule, info-bulle « libellé (raccourci) »).
    fn quick_button(&self, ui: &mut egui::Ui, command: Command, enabled: bool) -> egui::Response {
        let lang = self.lang();
        let info = command.info();
        let label = tr(lang, info.label_key);
        let tooltip = match info.shortcut_key {
            Some(k) => format!("{label}  ({})", tr(lang, k)),
            None => label.to_string(),
        };
        let r = bar_button(
            ui,
            self.has_icons,
            BarButton {
                icon: command_icon(command),
                tint: None,
                label,
                show_label: false,
                tooltip: &tooltip,
                selected: false,
                enabled,
            },
        );
        if r.clicked() {
            // Comme les boutons du hub : les raccourcis à une lettre restent actifs (A14).
            r.surrender_focus();
        }
        r
    }

    /// Menu « Outils » : un élément par outil disponible (coché si actif), puis « Quitter l'outil ».
    fn tools_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let mut run = None;
        ui.menu_button(tr(lang, "menu.tools"), |ui| {
            for ws in Workspace::available() {
                let on = self.annotate.workspace == Some(ws);
                let item = Button::new(tr(lang, ws.info().name_key)).selected(on);
                if ui.add(item).clicked() {
                    run = ws.activate();
                    ui.close();
                }
            }
            ui.separator();
            let leave = Command::HubLeave.info();
            if crate::app::menu_item(
                ui,
                tr(lang, leave.label_key),
                tr(lang, "shortcut.hub_leave"),
                self.annotate.workspace.is_some(),
                false,
            ) {
                run = Some(Command::HubLeave);
            }
        });
        if let Some(c) = run {
            self.execute(ctx, c);
        }
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
                if is_active {
                    // Trait d'accent en haut de l'onglet actif.
                    let r = resp.rect.shrink2(egui::vec2(6.0, 0.0));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(r.min, egui::vec2(r.width(), 2.0)),
                        1.0,
                        p.accent,
                    );
                }
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
                    self.tools_menu(ctx, ui);
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
            // Accès rapide.
            let has_doc = self.document.is_some();
            let mut run = None;
            for (command, enabled) in [
                (Command::OpenDocument, true),
                (Command::Save, has_doc && self.dirty && !self.saving),
                (Command::Undo, has_doc && self.can_undo),
                (Command::Redo, has_doc && self.can_redo),
            ] {
                if self.quick_button(ui, command, enabled).clicked() {
                    run = Some(command);
                }
            }
            ui.separator();
            let tools_tip = format!(
                "{}  ({})",
                tr(lang, "menu.view.tools"),
                tr(lang, "shortcut.tools")
            );
            let all_tools = bar_button(
                ui,
                self.has_icons,
                BarButton {
                    icon: (glyph::ALL_APPS, "▤"),
                    tint: None,
                    label: tr(lang, "toolbar.all_tools"),
                    // Fenêtre étroite : icône seule, la recherche (≈ 300 px) garde sa place.
                    show_label: ui.available_width() > 460.0,
                    tooltip: &tools_tip,
                    selected: self.show_tools,
                    enabled: true,
                },
            );
            if all_tools.clicked() {
                all_tools.surrender_focus();
                run = Some(Command::ToggleTools);
            }
            if let Some(c) = run {
                self.execute(ctx, c);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
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
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(icons::semibold(tr(lang, "toolbar.all_tools"), 16.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.icon_button(ui, glyph::CLOSE, "✕", tr(lang, "tools.close")) {
                    self.show_tools = false;
                    self.settings.show_tools = false;
                    self.persist_settings();
                }
            });
        });
        ui.add_space(6.0);
        let active = self.annotate.workspace;
        let mut run = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for ws in Workspace::available() {
                    let info = ws.info();
                    let resp = tool_row(
                        ui,
                        self.has_icons,
                        info,
                        tr(lang, info.name_key),
                        tr(lang, info.desc_key),
                        active == Some(ws),
                    );
                    if resp.clicked() {
                        // Sinon le focus clavier reste sur la ligne et bloque les raccourcis à une touche.
                        resp.surrender_focus();
                        run = if active == Some(ws) {
                            Some(Command::HubLeave)
                        } else {
                            ws.activate()
                        };
                    }
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr(lang, "tools.hint"))
                                .size(12.0)
                                .color(p.text_muted),
                        )
                        .wrap(),
                    );
                });
            });
        if let Some(c) = run {
            self.execute(ctx, c);
        }
    }

    // ----- Accueil ---------------------------------------------------------

    pub fn home(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = self.pal();
        let mut run: Option<Command> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(40, 28))
                    .show(ui, |ui| {
                        // `set_max_width` peut élargir : borner à la largeur réellement disponible.
                        ui.set_max_width(ui.available_width().min(1100.0));
                        ui.label(icons::semibold(tr(lang, "home.welcome"), 28.0));
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(tr(lang, "home.tagline"))
                                .color(p.text_muted)
                                .size(15.0),
                        );
                        ui.add_space(20.0);
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
                        if self.open_card(ui).clicked() {
                            run = Some(Command::OpenDocument);
                        }
                        ui.add_space(28.0);
                        ui.label(icons::semibold(tr(lang, "home.recommended"), 17.0));
                        ui.add_space(10.0);
                        if let Some(c) = self.tool_cards(ui) {
                            run = Some(c);
                        }

                        ui.add_space(28.0);
                        ui.label(icons::semibold(tr(lang, "home.recent"), 17.0));
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
        if let Some(c) = run {
            // Outil à barre ou outil direct : même chemin (ouverture d'un fichier d'abord si besoin).
            self.execute(ctx, c);
        }
    }

    /// Grande carte « Ouvrir un fichier » (fond accent, entièrement cliquable).
    fn open_card(&self, ui: &mut egui::Ui) -> egui::Response {
        let lang = self.lang();
        let p = self.pal();
        let w = ui.available_width().min(1100.0);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 84.0), egui::Sense::click());
        let label = tr(lang, "home.open_file");
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let t = ui.ctx().animate_bool_with_time(
            resp.id.with("hover"),
            resp.hovered() || resp.has_focus(),
            0.12,
        );
        let painter = ui.painter();
        if !self.settings.dark_theme {
            let shadow = egui::epaint::Shadow {
                offset: [0, 3],
                blur: 14,
                spread: 0,
                color: Color32::from_rgba_unmultiplied(0x25, 0x63, 0xEB, 40 + (t * 30.0) as u8),
            };
            painter.add(shadow.as_shape(rect, theme::radius::CARD));
        }
        let fill = p.accent_fill.lerp_to_gamma(Color32::BLACK, 0.12 * t);
        painter.rect_filled(rect, theme::radius::CARD, fill);
        let circle = egui::pos2(rect.left() + 46.0, rect.center().y);
        painter.circle_filled(circle, 24.0, Color32::from_white_alpha(38));
        let (g, f) = (glyph::FOLDER_OPEN, "📂");
        painter.text(
            circle,
            egui::Align2::CENTER_CENTER,
            if self.has_icons { g } else { f },
            if self.has_icons {
                egui::FontId::new(20.0, egui::FontFamily::Name(icons::ICONS.into()))
            } else {
                egui::FontId::proportional(20.0)
            },
            Color32::WHITE,
        );
        let x = rect.left() + 86.0;
        painter.text(
            egui::pos2(x, rect.center().y - 2.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::new(18.0, egui::FontFamily::Name(icons::SEMIBOLD.into())),
            Color32::WHITE,
        );
        painter.text(
            egui::pos2(x, rect.center().y + 3.0),
            egui::Align2::LEFT_TOP,
            tr(lang, "home.open_file.desc"),
            egui::FontId::proportional(13.5),
            Color32::from_white_alpha(225),
        );
        painter.text(
            egui::pos2(rect.right() - 24.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            tr(lang, "shortcut.open"),
            egui::FontId::proportional(13.0),
            Color32::from_white_alpha(200),
        );
        if resp.has_focus() {
            painter.rect_stroke(
                rect.shrink(2.0),
                theme::radius::CARD,
                Stroke::new(2.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }
        resp
    }

    /// Grille des outils disponibles (3 colonnes si la largeur le permet), cartes entièrement
    /// cliquables : bordure de la couleur de l'outil au survol. Renvoie la commande choisie.
    fn tool_cards(&self, ui: &mut egui::Ui) -> Option<Command> {
        const GAP: f32 = 14.0;
        const CARD_H: f32 = 96.0;
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let p = self.pal();
        let tools: Vec<Workspace> = Workspace::available().collect();
        let w = ui.available_width().min(1100.0);
        let cols = if w >= 3.0 * 240.0 + 2.0 * GAP {
            3
        } else if w >= 2.0 * 240.0 + GAP {
            2
        } else {
            1
        };
        let card_w = (w - GAP * (cols as f32 - 1.0)) / cols as f32;
        let rows = tools.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(
            egui::vec2(w, rows as f32 * (CARD_H + GAP) - GAP),
            egui::Sense::hover(),
        );
        let mut run = None;
        for (i, ws) in tools.into_iter().enumerate() {
            let info = ws.info();
            let rect = egui::Rect::from_min_size(
                area.min
                    + egui::vec2(
                        (i % cols) as f32 * (card_w + GAP),
                        (i / cols) as f32 * (CARD_H + GAP),
                    ),
                egui::vec2(card_w, CARD_H),
            );
            let name = tr(lang, info.name_key);
            let resp = ui.interact(rect, egui::Id::new(("home-tool", i)), egui::Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
            let resp = resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(tr(lang, info.desc_key));
            if resp.clicked() {
                resp.surrender_focus();
                run = ws.activate();
            }
            if !ui.is_rect_visible(rect) {
                continue;
            }
            let t = ui.ctx().animate_bool_with_time(
                resp.id.with("hover"),
                resp.hovered() || resp.has_focus(),
                0.12,
            );
            let painter = ui.painter();
            if !dark {
                // En thème sombre, l'ombre noire est invisible : la bordure colorée porte l'effet.
                let shadow = egui::epaint::Shadow {
                    offset: [0, 1 + (t * 3.0) as i8],
                    blur: 8 + (t * 10.0) as u8,
                    spread: 0,
                    color: Color32::from_black_alpha(12 + (t * 16.0) as u8),
                };
                painter.add(shadow.as_shape(rect, theme::radius::CARD));
            }
            painter.rect(
                rect,
                theme::radius::CARD,
                p.surface,
                Stroke::new(1.0, p.border.lerp_to_gamma(info.color, t)),
                egui::StrokeKind::Inside,
            );
            let badge_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 18.0, rect.center().y - 22.0),
                egui::vec2(44.0, 44.0),
            );
            paint_badge(
                painter,
                badge_rect,
                self.has_icons,
                info.icon,
                info.color,
                dark,
            );
            let x = badge_rect.right() + 16.0;
            let wrap = (rect.right() - 16.0 - x).max(40.0);
            let title = painter.layout(
                name.to_string(),
                egui::FontId::new(15.5, egui::FontFamily::Name(icons::SEMIBOLD.into())),
                p.text,
                wrap,
            );
            let desc = painter.layout(
                tr(lang, info.desc_key).to_string(),
                egui::FontId::proportional(13.0),
                p.text_muted,
                wrap,
            );
            let block = title.size().y + 4.0 + desc.size().y;
            let y = rect.center().y - block / 2.0;
            let title_h = title.size().y;
            painter.galley(egui::pos2(x, y), title, p.text);
            painter.galley(egui::pos2(x, y + title_h + 4.0), desc, p.text_muted);
            if resp.has_focus() {
                theme::focus_ring(painter, rect, &p, theme::radius::CARD);
            }
        }
        run
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
                .add(Button::selectable(
                    self.settings.show_thumbnails,
                    self.icon(glyph::GRID, "▦", 16.0),
                ))
                .on_hover_text(tr(lang, "menu.view.thumbnails"));
            if thumbs.clicked() {
                self.execute(ctx, Command::ToggleThumbnails);
            }
            let organize = ui
                .add(Button::selectable(
                    self.organize.active,
                    self.icon(glyph::PAGES, "▦", 16.0),
                ))
                .on_hover_text(tr(lang, "menu.view.organize"));
            if organize.clicked() {
                self.execute(ctx, Command::OrganizePages);
            }
            // Bascule de la vue : allumé = défilement continu (F5).
            let continuous = self.view.mode == crate::view::ViewMode::Continuous;
            let view_toggle = ui
                .add(Button::selectable(
                    continuous,
                    self.icon(glyph::SCROLL, "⇕", 16.0),
                ))
                .on_hover_text(format!(
                    "{}  ({})",
                    tr(lang, "menu.view.continuous"),
                    tr(lang, "shortcut.view_toggle")
                ));
            if view_toggle.clicked() {
                self.execute(
                    ctx,
                    if continuous {
                        Command::ViewSinglePage
                    } else {
                        Command::ViewContinuous
                    },
                );
            }
            let bookmarks = ui
                .add(Button::selectable(
                    self.docinfo.bookmarks_open,
                    self.icon(glyph::BOOKMARK, "🔖", 16.0),
                ))
                .on_hover_text(tr(lang, "menu.view.bookmarks"));
            if bookmarks.clicked() {
                self.execute(ctx, Command::ToggleBookmarks);
            }
            let comments = ui
                .add(Button::selectable(
                    self.annotate.comments_open,
                    self.icon(glyph::COMMENT, "💬", 16.0),
                ))
                .on_hover_text(tr(lang, "menu.view.comments"));
            if comments.clicked() {
                self.execute(ctx, Command::ToggleComments);
            }
            let objects = ui
                .add(Button::selectable(
                    self.edit.objects_open,
                    self.icon(glyph::LAYERS, "☰", 16.0),
                ))
                .on_hover_text(format!(
                    "{}  ({})",
                    tr(lang, "menu.view.objects"),
                    tr(lang, "shortcut.objects")
                ));
            if objects.clicked() {
                self.execute(ctx, Command::ToggleObjects);
            }
            let find = ui
                .add(Button::selectable(
                    self.find.open,
                    self.icon(glyph::FIND, "🔍", 16.0),
                ))
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
                if self.icon_button(
                    ui,
                    glyph::ROTATE_RIGHT,
                    "↻",
                    tr(lang, "menu.pages.rotate_cw"),
                ) {
                    self.execute(ctx, Command::RotateClockwise);
                }
                let fit_w = ui
                    .add(Button::selectable(
                        self.zoom == Zoom::FitWidth,
                        self.icon(glyph::FIT_WIDTH, "↔", 16.0),
                    ))
                    .on_hover_text(tr(lang, "menu.view.fit_width"));
                if fit_w.clicked() {
                    self.execute(ctx, Command::FitWidth);
                }
                let fit_p = ui
                    .add(Button::selectable(
                        self.zoom == Zoom::FitPage,
                        self.icon(glyph::FIT_PAGE, "▭", 16.0),
                    ))
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
            // « Quitter l'outil » sans outil actif serait une entrée sans effet.
            .filter(|c| c.command != Command::HubLeave || self.annotate.workspace.is_some())
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
                                    // Couleur de l'outil propriétaire (même code couleur que le hub).
                                    let tint = theme::glyph_color(
                                        command_color(*command),
                                        self.settings.dark_theme,
                                    );
                                    ui.label(self.icon(g, f, 15.0).color(tint));
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
                    if self.view.is_rendering() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Toggled;
    use sunipdf_i18n::Language;

    fn private_use(s: &str) -> bool {
        s.chars().any(|c| ('\u{E000}'..='\u{F8FF}').contains(&c))
    }

    /// Anneau de focus (`theme::focus_ring`) présent parmi les formes peintes.
    fn focus_ring_painted(shapes: &[egui::epaint::ClippedShape], color: Color32) -> bool {
        shapes.iter().any(|c| {
            matches!(&c.shape, egui::Shape::Rect(r) if r.stroke.color == color && r.stroke.width == 2.0)
        })
    }

    /// Une passe sans fenêtre : panneau « Tous les outils » (Commenter actif) et barre de Commenter.
    fn draw(ui: &mut egui::Ui, has_icons: bool, rows: &mut Vec<egui::Rect>) {
        let lang = Language::French;
        let comment = Workspace::Comment.info();
        rows.clear();
        ui.vertical(|ui| {
            ui.set_width(250.0);
            for ws in Workspace::available() {
                let info = ws.info();
                let r = tool_row(
                    ui,
                    has_icons,
                    info,
                    tr(lang, info.name_key),
                    tr(lang, info.desc_key),
                    ws == Workspace::Comment,
                );
                rows.push(r.rect);
            }
        });
        ui.horizontal(|ui| {
            for c in comment.commands() {
                bar_button(
                    ui,
                    has_icons,
                    BarButton {
                        icon: command_icon(c),
                        tint: Some(comment.color),
                        label: tr(lang, c.info().label_key),
                        show_label: true,
                        tooltip: "",
                        selected: c == Command::ToolSelect,
                        enabled: c != Command::CopyText,
                    },
                );
            }
        });
    }

    /// Mise en page et arbre AccessKit sans fenêtre : lignes du hub de 40 px sur toute la largeur,
    /// libellés lisibles (jamais un glyphe d'icône), état sélectionné et désactivé exposés, anneau de
    /// focus peint après Tab, dans les deux thèmes.
    #[test]
    fn hub_rows_and_bar_buttons_expose_readable_labels() {
        let lang = Language::French;
        let ctx = egui::Context::default();
        let has_icons = icons::install(&ctx);
        ctx.enable_accesskit();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 780.0),
            )),
            ..Default::default()
        };
        let comment = Workspace::Comment.info();
        let run = |input: egui::RawInput, rows: &mut Vec<egui::Rect>| {
            let mut out = ctx.run_ui(input, |ui| draw(ui, has_icons, rows));
            // Pas de rendu GPU ici : les textures produites sont abandonnées volontairement.
            out.textures_delta.clear();
            out
        };
        for dark in [false, true] {
            theme::apply(&ctx, dark);
            let mut rows = Vec::new();
            // Deux passes : les polices installées ne servent qu'à partir de la passe suivante.
            let _ = run(input.clone(), &mut rows);
            let output = run(input.clone(), &mut rows);
            assert_eq!(rows.len(), Workspace::available().count());
            for r in &rows {
                assert_eq!(r.height(), 40.0);
                assert_eq!(r.width(), 250.0);
            }
            assert!(!output.shapes.is_empty());
            let tree = output
                .platform_output
                .accesskit_update
                .expect("arbre AccessKit");
            let node = |label: &str| {
                tree.nodes
                    .iter()
                    .map(|(_, n)| n)
                    .find(|n| n.label() == Some(label))
                    .unwrap_or_else(|| panic!("nœud « {label} » absent"))
            };
            assert_eq!(
                node(tr(lang, comment.name_key)).toggled(),
                Some(Toggled::True)
            );
            assert_eq!(
                node(tr(lang, Workspace::Edit.info().name_key)).toggled(),
                Some(Toggled::False)
            );
            for c in comment.commands() {
                let n = node(tr(lang, c.info().label_key));
                assert_eq!(n.is_disabled(), c == Command::CopyText, "{c:?}");
            }
            for (_, n) in &tree.nodes {
                assert!(!n.label().is_some_and(private_use), "{:?}", n.label());
            }

            // Clavier : Tab donne le focus à la première ligne, qui peint son anneau.
            let focus = theme::palette(dark).focus;
            assert!(!focus_ring_painted(&output.shapes, focus));
            let mut tab = input.clone();
            tab.events.push(egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let output = run(tab, &mut rows);
            assert!(ctx.memory(|m| m.focused().is_some()));
            assert!(focus_ring_painted(&output.shapes, focus));
            ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap()));
        }
    }
}
