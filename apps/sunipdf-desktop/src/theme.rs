//! Design system SuniPDF : jetons de couleur, visuels et espacements egui.

use egui::{Color32, CornerRadius, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    Classic,
    LiquidGlass,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    System,
    #[default]
    Light,
    Dark,
}

/// Appearance state belongs to an egui context, not a process-wide global.
pub fn appearance(ctx: &egui::Context) -> (Style, bool) {
    ctx.data(|data| {
        data.get_temp::<(Style, bool)>(egui::Id::unique("sunipdf-appearance"))
            .unwrap_or((Style::Classic, false))
    })
}

pub fn palette_for(ctx: &egui::Context) -> Palette {
    palette_for_style(ctx.global_style().visuals.dark_mode, appearance(ctx).0)
}

pub const BLUE: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const ERROR: Color32 = Color32::from_rgb(0xDC, 0x26, 0x26);
pub const SUCCESS: Color32 = Color32::from_rgb(0x16, 0xA3, 0x4A);
/// Couleurs de catégories d'outils : fichier, pages, annotation, édition,
/// conversion, sécurité, navigation.
pub const CAT_FILE: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const CAT_PAGES: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
pub const CAT_ANNOTATE: Color32 = Color32::from_rgb(0x8B, 0x5C, 0xF6);
pub const CAT_EDIT: Color32 = Color32::from_rgb(0xE1, 0x1D, 0x48);
pub const CAT_CONVERT: Color32 = Color32::from_rgb(0x16, 0xA3, 0x4A);
pub const CAT_SECURITY: Color32 = Color32::from_rgb(0x0D, 0x94, 0x88);
pub const CAT_VIEW: Color32 = Color32::from_rgb(0x0E, 0xA5, 0xE9);

#[derive(Clone, Copy)]
pub struct Palette {
    /// Fond du chrome (barres, panneaux).
    pub chrome: Color32,
    /// Surfaces (cartes, champs, onglet actif).
    pub surface: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub canvas: Color32,
    pub accent: Color32,
    pub hover: Color32,
    /// Fond des boutons pleins (texte blanc lisible dans les deux thèmes).
    pub accent_fill: Color32,
    /// Fond d'une ligne ou d'un bouton sélectionné.
    pub accent_soft: Color32,
    /// Anneau de focus clavier.
    pub focus: Color32,
    /// Texte d'erreur lisible sur les surfaces du thème (≥ 4,5:1 ; `ERROR` sert aux fonds).
    pub error: Color32,
}

/// Rayons des coins (contrôles, lignes, cartes).
pub mod radius {
    pub const ROW: u8 = 8;
    pub const CARD: u8 = 12;
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            chrome: Color32::from_rgb(0x19, 0x1E, 0x2E),
            surface: Color32::from_rgb(0x22, 0x28, 0x3A),
            border: Color32::from_rgb(0x34, 0x3C, 0x52),
            text: Color32::from_rgb(0xF1, 0xF5, 0xF9),
            text_muted: Color32::from_rgb(0x94, 0xA3, 0xB8),
            canvas: Color32::from_rgb(0x0B, 0x10, 0x1A),
            accent: Color32::from_rgb(0x60, 0xA5, 0xFA),
            hover: Color32::from_rgb(0x2A, 0x36, 0x48),
            accent_fill: BLUE,
            accent_soft: Color32::from_rgb(0x2C, 0x3C, 0x59),
            focus: Color32::from_rgb(0x60, 0xA5, 0xFA),
            error: Color32::from_rgb(0xF8, 0x71, 0x71),
        }
    } else {
        Palette {
            chrome: Color32::from_rgb(0xF8, 0xFA, 0xFC),
            surface: Color32::WHITE,
            border: Color32::from_rgb(0xE2, 0xE8, 0xF0),
            text: Color32::from_rgb(0x0F, 0x17, 0x2A),
            text_muted: Color32::from_rgb(0x64, 0x74, 0x8B),
            canvas: Color32::from_rgb(0xE5, 0xE7, 0xEB),
            accent: BLUE,
            hover: Color32::from_rgb(0xEE, 0xF2, 0xF7),
            accent_fill: BLUE,
            accent_soft: Color32::from_rgb(0xE9, 0xEF, 0xFD),
            focus: BLUE,
            error: ERROR,
        }
    }
}

pub fn palette_for_style(dark: bool, style: Style) -> Palette {
    if style == Style::Classic {
        return palette(dark);
    }
    if dark {
        Palette {
            chrome: Color32::from_rgb(0x20, 0x22, 0x26),
            surface: Color32::from_rgb(0x2A, 0x2C, 0x31),
            border: Color32::from_rgb(0x45, 0x49, 0x4F),
            text: Color32::from_rgb(0xF5, 0xF6, 0xF8),
            text_muted: Color32::from_rgb(0xB4, 0xB8, 0xC0),
            canvas: Color32::from_rgb(0x18, 0x1A, 0x1D),
            accent: Color32::from_rgb(0x78, 0xAE, 0xF7),
            hover: Color32::from_rgb(0x35, 0x38, 0x3E),
            accent_fill: Color32::from_rgb(0x0A, 0x64, 0xD8),
            accent_soft: Color32::from_rgb(0x2A, 0x3D, 0x54),
            focus: Color32::from_rgb(0x83, 0xB7, 0xFF),
            error: Color32::from_rgb(0xFF, 0xA7, 0xAE),
        }
    } else {
        Palette {
            chrome: Color32::from_rgb(0xF4, 0xF5, 0xF7),
            surface: Color32::WHITE,
            border: Color32::from_rgb(0xD4, 0xD8, 0xDF),
            text: Color32::from_rgb(0x1B, 0x1E, 0x23),
            text_muted: Color32::from_rgb(0x5B, 0x61, 0x6B),
            canvas: Color32::from_rgb(0xE9, 0xEB, 0xEE),
            accent: Color32::from_rgb(0x0A, 0x64, 0xD8),
            hover: Color32::from_rgb(0xEB, 0xED, 0xF0),
            accent_fill: Color32::from_rgb(0x0A, 0x64, 0xD8),
            accent_soft: Color32::from_rgb(0xE7, 0xEF, 0xFB),
            focus: Color32::from_rgb(0x0A, 0x64, 0xD8),
            error: Color32::from_rgb(0xB4, 0x23, 0x32),
        }
    }
}

#[cfg(test)]
pub fn apply(ctx: &egui::Context, dark: bool) {
    apply_appearance(ctx, dark, Style::Classic, false);
}

pub fn apply_appearance(ctx: &egui::Context, dark: bool, style: Style, reduce_transparency: bool) {
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::unique("sunipdf-appearance"),
            (style, reduce_transparency),
        );
    });
    // set_visuals edits the currently selected style slot.
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let p = palette_for_style(dark, style);
    let glass = style == Style::LiquidGlass;
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let r = CornerRadius::same(6);
    v.panel_fill = p.chrome;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.hover;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.error_fg_color = p.error;
    v.selection.bg_fill = if glass {
        p.accent_soft
    } else {
        p.accent.gamma_multiply(if dark { 0.5 } else { 0.25 })
    };
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(if glass { 12 } else { 10 });
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, if glass { p.border } else { p.accent });
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    // egui also uses the active foreground for strong labels, independently of override_text_color.
    // Generic active controls therefore use a soft background; filled actions set white explicitly.
    v.widgets.active.bg_fill = p.accent_soft;
    v.widgets.active.weak_bg_fill = p.accent_soft;
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.open.bg_fill = p.hover;
    v.widgets.open.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.open.weak_bg_fill = p.hover;
    if glass {
        v.widgets.active.bg_stroke = Stroke::new(1.0, p.focus);
        v.widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    }
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
        // Pas de « saut » de taille au survol.
        w.expansion = 0.0;
    }
    v.menu_corner_radius = CornerRadius::same(if glass { 12 } else { 8 });
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 150 } else { 40 }),
    };
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 160 } else { 60 }),
    };
    if glass {
        v.popup_shadow = egui::epaint::Shadow {
            offset: [0, 3],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 72 } else { 22 }),
        };
        v.window_shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 80 } else { 24 }),
        };
        if reduce_transparency {
            v.popup_shadow = Default::default();
            v.window_shadow = Default::default();
        }
    }
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.animation_time = 0.10;
        s.interaction.tooltip_delay = 0.35;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.menu_margin = egui::Margin::same(8);
        s.spacing.interact_size.y = 28.0;
        use egui::{FontId, TextStyle};
        s.text_styles
            .insert(TextStyle::Body, FontId::proportional(14.0));
        s.text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));
        s.text_styles
            .insert(TextStyle::Heading, FontId::proportional(22.0));
        s.text_styles
            .insert(TextStyle::Monospace, FontId::monospace(13.0));
    });
}

/// Cases à cocher carrées (coins à 2 px) dans la portée de `ui` : le rayon du thème arrondit une
/// case de 14 px au point de la confondre avec un bouton radio.
pub fn square_boxes(ui: &mut egui::Ui) {
    let w = &mut ui.visuals_mut().widgets;
    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = CornerRadius::same(2);
    }
}

/// Chrome geometry remains explicit at the call site so theming cannot hide controls.
pub fn panel_frame(ctx: &egui::Context) -> egui::Frame {
    let p = palette_for(ctx);
    // Docked bars share their outer edges; rounding would expose the window clear color.
    egui::Frame::new().fill(p.chrome)
}

/// The opaque base keeps underlying document content out of text and controls.
pub fn material_frame(ctx: &egui::Context) -> egui::Frame {
    let p = palette_for(ctx);
    let glass = appearance(ctx).0 == Style::LiquidGlass;
    let mut frame = egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12);
    if glass && !appearance(ctx).1 {
        frame = frame.shadow(egui::epaint::Shadow {
            offset: [0, 3],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if ctx.global_style().visuals.dark_mode {
                56
            } else {
                16
            }),
        });
    }
    frame
}

/// Paint before a floating surface's contents. A fixed mesh provides a subtle reflection;
/// no framebuffer readback, live blur or continuously scheduled repaint is required.
pub fn paint_material(ui: &egui::Ui, rect: egui::Rect, radius: u8) {
    let shapes = material_shapes(ui, rect, radius);
    if shapes.is_empty() {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.add(egui::Shape::Vec(shapes));
}

/// Docked chrome remains opaque and flat. Its controls provide the interactive hierarchy.
pub fn material_scope<R>(
    ui: &mut egui::Ui,
    _radius: u8,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    contents(ui)
}

fn material_shapes(ui: &egui::Ui, rect: egui::Rect, radius: u8) -> Vec<egui::Shape> {
    let (style, reduce_transparency) = appearance(ui.ctx());
    if style != Style::LiquidGlass
        || reduce_transparency
        || !rect.is_positive()
        || !rect.is_finite()
    {
        return Vec::new();
    }
    let dark = ui.visuals().dark_mode;
    let mut shapes = Vec::with_capacity(1);
    let inset = f32::from(radius.min(12)).min(rect.width() * 0.25).max(1.0);
    let top = rect.top() + 1.0;
    let bottom = (top + 24.0).min(rect.bottom() - 1.0);
    if bottom > top && rect.width() > inset * 2.0 {
        let reflection = egui::Rect::from_min_max(
            egui::pos2(rect.left() + inset, top),
            egui::pos2(rect.right() - inset, bottom),
        );
        let mut mesh = egui::Mesh::default();
        let bright = Color32::from_white_alpha(if dark { 6 } else { 32 });
        for (pos, color) in [
            (reflection.left_top(), bright),
            (reflection.right_top(), bright),
            (reflection.left_bottom(), Color32::TRANSPARENT),
            (reflection.right_bottom(), Color32::TRANSPARENT),
        ] {
            mesh.colored_vertex(pos, color);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(2, 1, 3);
        shapes.push(egui::Shape::mesh(mesh));
    }
    shapes
}

/// Glyphe d'outil sur sa pastille : couleur brute assombrie (clair) ou éclaircie (sombre) de 35 %,
/// pour un contraste d'au moins 3,5:1 sur `badge_fill` (testé dans `workspace.rs`).
pub fn glyph_color(base: Color32, dark: bool) -> Color32 {
    base.lerp_to_gamma(if dark { Color32::WHITE } else { Color32::BLACK }, 0.35)
}

pub fn glyph_color_for(ctx: &egui::Context, base: Color32) -> Color32 {
    let dark = ctx.global_style().visuals.dark_mode;
    if dark && appearance(ctx).0 == Style::LiquidGlass {
        base.lerp_to_gamma(Color32::WHITE, 0.43)
    } else {
        glyph_color(base, dark)
    }
}

/// Pastille teintée d'un outil (13 % en clair, 32 % en sombre).
pub fn badge_fill(base: Color32, dark: bool) -> Color32 {
    base.gamma_multiply(if dark { 0.32 } else { 0.13 })
}

/// Fondu de survol (100 ms) : opacité de `color` selon l'état animé.
pub fn hover_fade(ui: &egui::Ui, id: egui::Id, hovered: bool, color: Color32) -> Color32 {
    color.gamma_multiply(ui.ctx().animate_bool_with_time(id, hovered, 0.10))
}

/// Anneau de focus clavier (2 px) à l'intérieur de `rect`.
pub fn focus_ring(painter: &egui::Painter, rect: egui::Rect, p: &Palette, radius: u8) {
    painter.rect_stroke(
        rect.shrink(1.0),
        radius,
        Stroke::new(2.0, p.focus),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, RichText, Shape};
    use sunipdf_i18n::{tr, Language};

    fn contrast(first: Color32, second: Color32) -> f64 {
        let luminance = |color: Color32| {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        };
        let first = luminance(first);
        let second = luminance(second);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    fn painted_colors(shape: &Shape, clip: Rect, label: &str, colors: &mut Vec<Color32>) {
        match shape {
            Shape::Text(text) if text.galley.text() == label => {
                let bounds = Rect::from_min_size(text.pos, text.galley.size());
                if !clip.contains_rect(bounds) {
                    return;
                }
                for section in &text.galley.job.sections {
                    colors.push(text.override_text_color.unwrap_or_else(|| {
                        if section.format.color == Color32::PLACEHOLDER {
                            text.fallback_color
                        } else {
                            section.format.color
                        }
                    }));
                }
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    painted_colors(shape, clip, label, colors);
                }
            }
            _ => {}
        }
    }

    fn assert_painted(
        output: &egui::FullOutput,
        label: &str,
        foreground: Color32,
        backgrounds: &[Color32],
    ) {
        let mut colors = Vec::new();
        for shape in &output.shapes {
            painted_colors(&shape.shape, shape.clip_rect, label, &mut colors);
        }
        assert!(!colors.is_empty(), "visible label missing: {label}");
        for color in colors {
            assert_eq!(color, foreground, "painted label color: {label}");
            for &background in backgrounds {
                assert!(
                    contrast(color, background) >= 4.5,
                    "insufficient contrast for {label}: {color:?} on {background:?}"
                );
            }
        }
    }

    #[test]
    fn strong_labels_and_active_controls_paint_readable_text_in_both_languages_and_styles() {
        for language in Language::ALL {
            for (dark, style, reduced) in [false, true].into_iter().flat_map(|dark| {
                [Style::Classic, Style::LiquidGlass]
                    .into_iter()
                    .flat_map(move |style| {
                        [false, true]
                            .into_iter()
                            .map(move |reduced| (dark, style, reduced))
                    })
            }) {
                let ctx = egui::Context::default();
                apply_appearance(&ctx, dark, style, reduced);
                let palette = palette_for(&ctx);
                let strong = tr(language, "measure.rectangle.title");
                let rich = tr(language, "measure.precision");
                let heading = tr(language, "measure.panel.saved");
                let button = tr(language, "measure.unit");
                let primary = tr(language, "register.apply");
                let danger = tr(language, "comments.delete");
                let mut button_rect = Rect::NOTHING;
                for pressed in [false, true] {
                    let events = if pressed {
                        vec![
                            egui::Event::PointerMoved(button_rect.center()),
                            egui::Event::PointerButton {
                                pos: button_rect.center(),
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    } else {
                        Vec::new()
                    };
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(900.0, 650.0),
                        )),
                        events,
                        ..Default::default()
                    };
                    let mut down = false;
                    let mut output = ctx.run_ui(input, |ui| {
                        ui.strong(strong);
                        ui.label(RichText::new(rich).strong());
                        ui.heading(heading);
                        let response = ui.button(button);
                        button_rect = response.rect;
                        down = response.is_pointer_button_down_on();
                        crate::dialog::primary(ui, primary);
                        crate::dialog::danger(ui, danger);
                    });
                    assert_eq!(
                        down,
                        pressed,
                        "active button was not exercised: {} dark={dark}",
                        language.code()
                    );
                    for label in [strong, rich, heading] {
                        assert_painted(
                            &output,
                            label,
                            palette.text,
                            &[palette.surface, palette.chrome],
                        );
                    }
                    assert_painted(
                        &output,
                        button,
                        palette.text,
                        &[if pressed {
                            palette.accent_soft
                        } else {
                            palette.surface
                        }],
                    );
                    assert_painted(&output, primary, Color32::WHITE, &[palette.accent_fill]);
                    assert_painted(&output, danger, Color32::WHITE, &[ERROR]);
                    if pressed {
                        let style = ctx.global_style();
                        let active = &style.visuals.widgets.active;
                        assert_eq!(active.bg_fill, palette.accent_soft);
                        assert_eq!(active.weak_bg_fill, palette.accent_soft);
                        assert!(contrast(active.fg_stroke.color, active.bg_fill) >= 4.5);
                    }
                    output.textures_delta.clear();
                }
            }
        }
    }
    #[test]
    fn standard_error_labels_are_legible_in_light_and_dark_styles() {
        for dark in [false, true] {
            for style in [Style::Classic, Style::LiquidGlass] {
                let ctx = egui::Context::default();
                apply_appearance(&ctx, dark, style, false);
                let palette = palette_for(&ctx);
                let label = "The destination changed; review it again";
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    ui.colored_label(ui.visuals().error_fg_color, label);
                });
                assert_painted(
                    &output,
                    label,
                    palette.error,
                    &[palette.chrome, palette.surface],
                );
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn glass_text_retains_contrast_on_material_states_and_reflections() {
        for dark in [false, true] {
            let p = palette_for_style(dark, Style::LiquidGlass);
            for base in [p.chrome, p.surface, p.hover, p.accent_soft] {
                // The reflection is brightest at its top; every other point is less intense.
                let amount = if dark { 6.0 / 255.0 } else { 32.0 / 255.0 };
                let highlighted = base.lerp_to_gamma(Color32::WHITE, amount);
                for background in [base, highlighted] {
                    for foreground in [p.text, p.text_muted, p.error] {
                        assert!(contrast(foreground, background) >= 4.5,
                            "unreadable material label: dark={dark}, {foreground:?} on {background:?}");
                    }
                }
            }
            assert!(contrast(Color32::WHITE, p.accent_fill) >= 4.5);
            assert!(contrast(p.focus, p.chrome) >= 3.0);
        }
    }

    #[test]
    fn appearance_switching_keeps_contexts_independent_and_uses_the_resolved_mode() {
        let first = egui::Context::default();
        let second = egui::Context::default();
        apply_appearance(&first, true, Style::LiquidGlass, false);
        apply_appearance(&second, false, Style::Classic, false);
        assert!(first.global_style().visuals.dark_mode);
        assert!(!second.global_style().visuals.dark_mode);
        assert_eq!(appearance(&first), (Style::LiquidGlass, false));
        assert_eq!(appearance(&second), (Style::Classic, false));
        let original = palette(false);
        assert_eq!(palette_for(&second).chrome, original.chrome);
        assert_eq!(palette_for(&second).surface, original.surface);
        apply_appearance(&first, false, Style::LiquidGlass, true);
        assert!(!first.global_style().visuals.dark_mode);
        assert_eq!(appearance(&first), (Style::LiquidGlass, true));
        assert_eq!(appearance(&second), (Style::Classic, false));
    }

    #[test]
    fn reduced_floating_material_removes_reflections_without_changing_layout_or_text() {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 650.0))),
            ..Default::default()
        };
        let mut heights = Vec::new();
        for reduced in [false, true] {
            let ctx = egui::Context::default();
            apply_appearance(&ctx, true, Style::LiquidGlass, reduced);
            let mut bounds = Rect::NOTHING;
            let mut output = ctx.run_ui(input.clone(), |ui| {
                let response = material_frame(&ctx).show(ui, |ui| {
                    let size = egui::vec2(220.0, 56.0);
                    ui.set_min_size(size);
                    paint_material(ui, Rect::from_min_size(ui.min_rect().min, size), 12);
                    ui.label("Theme preview");
                    ui.button("Example action").rect
                });
                bounds = response.response.rect;
            });
            let materials: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::Vec(shapes)
                        if shapes.iter().any(|shape| matches!(shape, Shape::Mesh(_))) =>
                    {
                        Some(shapes)
                    }
                    _ => None,
                })
                .collect();
            if reduced {
                assert!(materials.is_empty());
            } else {
                assert_eq!(materials.len(), 1);
                for shape in materials[0] {
                    assert!(bounds.contains_rect(shape.visual_bounding_rect()));
                }
            }
            assert!(
                bounds.height() < 100.0,
                "floating material must not reserve the whole viewport"
            );
            assert_painted(
                &output,
                "Theme preview",
                palette_for(&ctx).text,
                &[palette_for(&ctx).surface],
            );
            heights.push(bounds.height());
            output.textures_delta.clear();
        }
        assert_eq!(heights[0], heights[1]);
    }

    #[test]
    fn docked_chrome_covers_outer_corners_without_a_material_rim() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            apply_appearance(&ctx, dark, Style::LiquidGlass, false);
            let p = palette_for(&ctx);
            let mut bounds = Rect::NOTHING;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let response = panel_frame(&ctx).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(640.0, 40.0));
                    material_scope(ui, 12, |ui| ui.label("Document tools"));
                });
                bounds = response.response.rect;
            });
            let outer = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    Shape::Rect(rect)
                        if rect.fill == p.chrome && rect.rect.contains_rect(bounds) =>
                    {
                        Some(rect)
                    }
                    _ => None,
                })
                .expect("the dock must paint an opaque surface to all four edges");
            assert_eq!(outer.fill.a(), 255);
            assert_eq!(outer.corner_radius, CornerRadius::ZERO);
            assert!(
                !output.shapes.iter().any(|shape| matches!(&shape.shape,
                    Shape::Vec(shapes) if shapes.iter().any(|shape| matches!(shape, Shape::Mesh(_)))
                )),
                "docked chrome must not receive a second reflective outline"
            );
            assert_painted(&output, "Document tools", p.text, &[p.chrome]);
            output.textures_delta.clear();
        }
    }
}
