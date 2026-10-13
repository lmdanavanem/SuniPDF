//! Design system SuniPDF : jetons de couleur, visuels et espacements egui.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BLUE: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const ERROR: Color32 = Color32::from_rgb(0xDC, 0x26, 0x26);
pub const SUCCESS: Color32 = Color32::from_rgb(0x16, 0xA3, 0x4A);

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
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            chrome: Color32::from_rgb(0x17, 0x1D, 0x2B),
            surface: Color32::from_rgb(0x1F, 0x29, 0x37),
            border: Color32::from_rgb(0x33, 0x41, 0x55),
            text: Color32::from_rgb(0xF1, 0xF5, 0xF9),
            text_muted: Color32::from_rgb(0x94, 0xA3, 0xB8),
            canvas: Color32::from_rgb(0x0B, 0x10, 0x1A),
            accent: Color32::from_rgb(0x60, 0xA5, 0xFA),
            hover: Color32::from_rgb(0x2A, 0x36, 0x48),
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
        }
    }
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let p = palette(dark);
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
    v.selection.bg_fill = p.accent.gamma_multiply(if dark { 0.5 } else { 0.25 });
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(10);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.active.bg_fill = p.accent;
    v.widgets.active.weak_bg_fill = p.accent.gamma_multiply(0.3);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.open.bg_fill = p.hover;
    v.widgets.open.weak_bg_fill = p.hover;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v.menu_corner_radius = CornerRadius::same(8);
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
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

/// Fond de la zone document (gris neutre, distinct de la page blanche).
pub fn canvas_bg(dark: bool) -> Color32 {
    palette(dark).canvas
}
