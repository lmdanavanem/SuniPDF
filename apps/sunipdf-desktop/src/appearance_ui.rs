//! Appearance preferences, live theme application and an accessible preview picker.

use crate::app::SuniApp;
use crate::dialog::{self, Spec};
use crate::settings::Settings;
use crate::theme::{self, Mode, Style};
use egui::{Color32, FontId, Rect, RichText, Stroke};
use sunipdf_i18n::{tr, Language};

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub error: bool,
    applied: Option<(bool, Style, bool)>,
}

pub(crate) fn resolved_dark(mode: Mode, system: Option<egui::Theme>) -> bool {
    match mode {
        Mode::Light => false,
        Mode::Dark => true,
        Mode::System => system == Some(egui::Theme::Dark),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Choice {
    style: Style,
    mode: Mode,
    reduce_transparency: bool,
}

impl Choice {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            style: settings.theme_style,
            mode: settings.effective_theme_mode(),
            reduce_transparency: settings.reduce_transparency,
        }
    }
}

impl SuniApp {
    pub(crate) fn sync_appearance(&mut self, ctx: &egui::Context) {
        let dark = resolved_dark(self.settings.effective_theme_mode(), ctx.system_theme());
        let resolved = (
            dark,
            self.settings.theme_style,
            self.settings.reduce_transparency,
        );
        if self.appearance.applied != Some(resolved) {
            theme::apply_appearance(ctx, dark, resolved.1, resolved.2);
            self.appearance.applied = Some(resolved);
        }
        // Keep the compatibility field resolved without writing on every system-theme change.
        self.settings.dark_theme = dark;
    }

    fn commit_appearance_choice(
        &mut self,
        ctx: &egui::Context,
        choice: Choice,
        writer: impl FnOnce(&Settings) -> std::io::Result<()>,
    ) {
        let dark = resolved_dark(choice.mode, ctx.system_theme());
        if self
            .settings
            .commit_appearance(
                choice.style,
                choice.mode,
                choice.reduce_transparency,
                dark,
                writer,
            )
            .is_err()
        {
            self.appearance.error = true;
            return;
        }
        self.appearance.error = false;
        self.sync_appearance(ctx);
        ctx.request_repaint();
    }

    pub(crate) fn appearance_dialog(&mut self, ctx: &egui::Context) {
        if !self.appearance.open {
            return;
        }
        let lang = self.lang();
        let before = Choice::from_settings(&self.settings);
        let mut choice = before;
        let dark = self.settings.dark_theme;
        let palette = theme::palette_for(ctx);
        let mut close = false;
        let width = (ctx.content_rect().width() - 40.0).clamp(280.0, 640.0);
        let error = self.appearance.error;
        dialog::show(
            ctx,
            Spec::new(
                "appearance-settings",
                tr(lang, "appearance.title"),
                dark,
                self.has_icons,
            )
            .subtitle(tr(lang, "appearance.subtitle"))
            .icon(crate::icons::glyph::SUN, "\u{2600}")
            .width(width),
            |ui| {
                ui.label(
                    RichText::new(tr(lang, "appearance.theme"))
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(tr(lang, "appearance.style")).strong());
                ui.add_space(4.0);
                if ui.available_width() >= 440.0 {
                    ui.columns(2, |columns| {
                        for (ui, style) in
                            columns.iter_mut().zip([Style::Classic, Style::LiquidGlass])
                        {
                            if style_card(ui, lang, style, choice.style == style, dark).clicked() {
                                choice.style = style;
                            }
                        }
                    });
                } else {
                    for style in [Style::Classic, Style::LiquidGlass] {
                        if style_card(ui, lang, style, choice.style == style, dark).clicked() {
                            choice.style = style;
                        }
                        ui.add_space(4.0);
                    }
                }
                ui.add_space(14.0);
                ui.label(RichText::new(tr(lang, "appearance.mode")).strong());
                ui.horizontal_wrapped(|ui| {
                    for (mode, key) in [
                        (Mode::System, "appearance.system"),
                        (Mode::Light, "appearance.light"),
                        (Mode::Dark, "appearance.dark"),
                    ] {
                        if ui
                            .selectable_label(choice.mode == mode, tr(lang, key))
                            .clicked()
                        {
                            choice.mode = mode;
                        }
                    }
                });
                ui.label(
                    RichText::new(tr(lang, "appearance.system.description"))
                        .small()
                        .color(palette.text_muted),
                );
                if choice.style == Style::LiquidGlass {
                    ui.add_space(12.0);
                    ui.checkbox(
                        &mut choice.reduce_transparency,
                        tr(lang, "appearance.reduce_transparency"),
                    );
                    ui.label(
                        RichText::new(tr(lang, "appearance.reduce_transparency.description"))
                            .small()
                            .color(palette.text_muted),
                    );
                }
                ui.add_space(10.0);
                if error {
                    ui.colored_label(palette.error, tr(lang, "appearance.save_failed"));
                } else {
                    ui.label(
                        RichText::new(tr(lang, "appearance.saved"))
                            .small()
                            .color(palette.text_muted),
                    );
                }
            },
            |ui| {
                close = dialog::secondary(ui, tr(lang, "dialog.close"))
                    || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            },
        );
        if choice != before {
            self.commit_appearance_choice(ctx, choice, |settings| {
                #[cfg(not(test))]
                {
                    settings.save_appearance()
                }
                #[cfg(test)]
                {
                    let _ = settings;
                    Ok(())
                }
            });
        }
        if close {
            self.appearance.open = false;
        }
    }
}

fn style_card(
    ui: &mut egui::Ui,
    lang: Language,
    style: Style,
    selected: bool,
    dark: bool,
) -> egui::Response {
    let palette = theme::palette_for(ui.ctx());
    let (label_key, description_key) = match style {
        Style::Classic => ("appearance.classic", "appearance.classic.description"),
        Style::LiquidGlass => ("appearance.glass", "appearance.glass.description"),
    };
    let label = tr(lang, label_key);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 164.0), egui::Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, label));
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            12,
            if selected {
                palette.accent_soft
            } else if response.hovered() {
                palette.hover
            } else {
                palette.surface
            },
            Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    palette.accent
                } else {
                    palette.border
                },
            ),
            egui::StrokeKind::Inside,
        );
        let preview = Rect::from_min_size(
            rect.min + egui::vec2(12.0, 12.0),
            egui::vec2(width - 24.0, 78.0),
        );
        paint_preview(ui.painter(), preview, style, dark);
        ui.painter().text(
            rect.min + egui::vec2(14.0, 100.0),
            egui::Align2::LEFT_TOP,
            label,
            FontId::proportional(15.0),
            palette.text,
        );
        if selected {
            let center = egui::pos2(rect.right() - 22.0, rect.top() + 109.0);
            ui.painter().circle_filled(center, 7.0, palette.accent_fill);
            ui.painter().line_segment(
                [
                    center + egui::vec2(-3.5, 0.0),
                    center + egui::vec2(-1.0, 2.5),
                ],
                Stroke::new(1.5, Color32::WHITE),
            );
            ui.painter().line_segment(
                [
                    center + egui::vec2(-1.0, 2.5),
                    center + egui::vec2(3.8, -3.0),
                ],
                Stroke::new(1.5, Color32::WHITE),
            );
        }
        let description = ui.painter().layout(
            tr(lang, description_key).to_string(),
            FontId::proportional(12.0),
            palette.text_muted,
            width - 28.0,
        );
        ui.painter().galley(
            rect.min + egui::vec2(14.0, 124.0),
            description,
            palette.text_muted,
        );
        if response.has_focus() {
            theme::focus_ring(ui.painter(), rect, &palette, 12);
        }
    }
    response.on_hover_text(tr(lang, description_key))
}

fn paint_preview(painter: &egui::Painter, rect: Rect, style: Style, dark: bool) {
    let p = theme::palette_for_style(dark, style);
    let glass = style == Style::LiquidGlass;
    painter.rect_filled(rect, 7, p.canvas);
    let top = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 16.0));
    painter.rect_filled(top, 6, p.chrome);
    if glass {
        painter.hline(
            (rect.left() + 7.0)..=(rect.right() - 7.0),
            rect.top() + 2.0,
            Stroke::new(1.0, Color32::from_white_alpha(if dark { 70 } else { 220 })),
        );
    }
    for x in [8.0, 16.0, 24.0] {
        painter.circle_filled(rect.min + egui::vec2(x, 8.0), 2.0, p.text_muted);
    }
    let inset = if glass { 7.0 } else { 0.0 };
    let sidebar = Rect::from_min_max(
        egui::pos2(rect.left() + inset, rect.top() + 19.0),
        egui::pos2(rect.left() + rect.width() * 0.27, rect.bottom() - inset),
    );
    painter.rect_filled(sidebar, if glass { 6 } else { 0 }, p.surface);
    for y in [8.0, 19.0, 30.0] {
        painter.rect_filled(
            Rect::from_min_size(
                sidebar.min + egui::vec2(6.0, y),
                egui::vec2((sidebar.width() - 12.0).max(1.0), 3.0),
            ),
            1,
            p.border,
        );
    }
    let paper = Rect::from_min_size(
        egui::pos2(rect.left() + rect.width() * 0.43, rect.top() + 23.0),
        egui::vec2(rect.width() * 0.34, 48.0),
    );
    painter.rect_filled(paper, 1, Color32::WHITE);
    for (y, width) in [(9.0, 0.58), (17.0, 0.74), (24.0, 0.68), (31.0, 0.48)] {
        painter.rect_filled(
            Rect::from_min_size(
                paper.min + egui::vec2(5.0, y),
                egui::vec2(paper.width() * width, 2.0),
            ),
            0,
            Color32::from_gray(185),
        );
    }
    if glass {
        let control = Rect::from_center_size(
            egui::pos2(rect.right() - 20.0, rect.bottom() - 13.0),
            egui::vec2(23.0, 12.0),
        );
        painter.rect(
            control,
            6,
            p.surface,
            Stroke::new(1.0, p.border),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers, Pos2, Shape};
    use sunipdf_core::Command;

    fn frame(
        app: &mut SuniApp,
        ctx: &egui::Context,
        events: Vec<Event>,
        size: egui::Vec2,
    ) -> egui::FullOutput {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ui| {
                app.sync_appearance(ui.ctx());
                app.handle_input(ui.ctx());
                app.appearance_dialog(ui.ctx());
            },
        )
    }

    fn find_label(shape: &Shape, label: &str) -> Option<Rect> {
        match shape {
            Shape::Text(text) if text.galley.text() == label => {
                Some(Rect::from_min_size(text.pos, text.galley.size()))
            }
            Shape::Vec(shapes) => shapes.iter().find_map(|shape| find_label(shape, label)),
            _ => None,
        }
    }

    fn visible_label(output: &egui::FullOutput, label: &str) -> Rect {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                find_label(&shape.shape, label).filter(|rect| shape.clip_rect.contains_rect(*rect))
            })
            .unwrap_or_else(|| panic!("missing visible label: {label}"))
    }

    fn click(app: &mut SuniApp, ctx: &egui::Context, point: Pos2, size: egui::Vec2) {
        for pressed in [true, false] {
            let mut output = frame(
                app,
                ctx,
                vec![
                    Event::PointerMoved(point),
                    Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
                size,
            );
            output.textures_delta.clear();
        }
    }

    #[test]
    fn appearance_controls_switch_live_without_touching_the_document() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.has_icons = crate::icons::install(&ctx);
        let size = egui::vec2(1100.0, 900.0);
        let original_path = app.document.as_ref().unwrap().info.path.clone();
        app.execute(&ctx, Command::ThemeSettings);
        assert!(app.dialog_open());
        assert!(!app.palette.open);
        for _ in 0..3 {
            let mut output = frame(&mut app, &ctx, vec![], size);
            output.textures_delta.clear();
        }
        for (label, expected_style, expected_dark) in [
            ("Liquid Glass", Style::LiquidGlass, false),
            ("Dark", Style::LiquidGlass, true),
            ("Classic", Style::Classic, true),
            ("Light", Style::Classic, false),
        ] {
            let mut output = frame(&mut app, &ctx, vec![], size);
            let target = visible_label(&output, label).center();
            output.textures_delta.clear();
            click(&mut app, &ctx, target, size);
            assert_eq!(app.settings.theme_style, expected_style, "{label}");
            assert_eq!(app.settings.dark_theme, expected_dark, "{label}");
            assert_eq!(
                ctx.global_style().visuals.dark_mode,
                expected_dark,
                "{label}"
            );
            assert_eq!(theme::appearance(&ctx).0, expected_style, "{label}");
        }
        assert_eq!(app.document.as_ref().unwrap().info.path, original_path);
        assert!(!app.dirty);
        assert!(requests.try_iter().next().is_none());
    }

    #[test]
    fn system_mode_tracks_os_changes_without_changing_the_selected_preferences() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.theme_mode = Some(Mode::System);
        app.settings.theme_style = Style::LiquidGlass;
        app.settings.reduce_transparency = true;
        let ctx = egui::Context::default();
        for (system, expected) in [
            (Some(egui::Theme::Dark), true),
            (Some(egui::Theme::Light), false),
            (None, false),
        ] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    system_theme: system,
                    ..Default::default()
                },
                |ui| app.sync_appearance(ui.ctx()),
            );
            output.textures_delta.clear();
            assert_eq!(app.settings.dark_theme, expected);
            assert_eq!(ctx.global_style().visuals.dark_mode, expected);
            assert_eq!(app.settings.theme_mode, Some(Mode::System));
            assert_eq!(app.settings.theme_style, Style::LiquidGlass);
            assert!(app.settings.reduce_transparency);
        }
        assert!(requests.try_iter().next().is_none());
    }

    #[test]
    fn failed_save_keeps_active_theme_and_shows_feedback() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.has_icons = crate::icons::install(&ctx);
        app.sync_appearance(&ctx);
        let before = app.settings.clone();
        app.commit_appearance_choice(
            &ctx,
            Choice {
                style: Style::LiquidGlass,
                mode: Mode::Dark,
                reduce_transparency: true,
            },
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "fixture refusal",
                ))
            },
        );
        assert_eq!(app.settings, before);
        assert_eq!(theme::appearance(&ctx).0, Style::Classic);
        assert!(!ctx.global_style().visuals.dark_mode);
        assert!(app.appearance.error);
        app.appearance.open = true;
        let mut output = frame(&mut app, &ctx, vec![], egui::vec2(1100.0, 900.0));
        output.textures_delta.clear();
        let mut output = frame(&mut app, &ctx, vec![], egui::vec2(1100.0, 900.0));
        visible_label(&output, tr(app.lang(), "appearance.save_failed"));
        output.textures_delta.clear();
    }

    #[test]
    fn appearance_modal_blocks_editing_and_escape_keeps_changes_at_small_sizes() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.set_language(language);
            app.settings.theme_style = Style::LiquidGlass;
            app.settings.theme_mode = Some(Mode::Dark);
            app.can_undo = true;
            let ctx = egui::Context::default();
            app.has_icons = crate::icons::install(&ctx);
            app.execute(&ctx, Command::ThemeSettings);
            let size = egui::vec2(720.0, 480.0);
            for _ in 0..3 {
                let mut output = frame(&mut app, &ctx, vec![], size);
                output.textures_delta.clear();
            }
            let key = |key, modifiers| Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            let mut output = frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)], size);
            let close = visible_label(&output, tr(language, "dialog.close"));
            assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(close));
            output.textures_delta.clear();
            assert!(app.appearance.open);
            assert!(requests.try_iter().next().is_none());
            let mut output = frame(
                &mut app,
                &ctx,
                vec![key(Key::Escape, Modifiers::NONE)],
                size,
            );
            output.textures_delta.clear();
            assert!(!app.appearance.open);
            assert_eq!(app.settings.theme_style, Style::LiquidGlass);
            assert_eq!(app.settings.effective_theme_mode(), Mode::Dark);
            assert!(!app.dirty);
        }
    }
    #[test]
    fn style_cards_accept_keyboard_space_and_enter() {
        for style in [Style::Classic, Style::LiquidGlass] {
            for key in [Key::Space, Key::Enter] {
                let ctx = egui::Context::default();
                theme::apply_appearance(&ctx, false, Style::Classic, false);
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let response = style_card(ui, Language::English, style, false, false);
                    response.request_focus();
                });
                output.textures_delta.clear();
                let mut activated = false;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events: vec![Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        }],
                        ..Default::default()
                    },
                    |ui| {
                        activated =
                            style_card(ui, Language::English, style, false, false).clicked();
                    },
                );
                output.textures_delta.clear();
                assert!(activated, "keyboard activation: {style:?} {key:?}");
            }
        }
    }
}
