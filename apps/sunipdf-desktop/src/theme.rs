//! Design system SuniPDF : jetons de couleur, visuels et espacements egui.

use egui::{Color32, CornerRadius, Stroke, Visuals};

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
    // Texte blanc sur fond plein : `accent` (clair en thème sombre) serait illisible (2,5:1).
    v.widgets.active.bg_fill = p.accent_fill;
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
        // Pas de « saut » de taille au survol.
        w.expansion = 0.0;
    }
    v.menu_corner_radius = CornerRadius::same(8);
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

/// Fond de la zone document (gris neutre, distinct de la page blanche).
pub fn canvas_bg(dark: bool) -> Color32 {
    palette(dark).canvas
}

/// Glyphe d'outil sur sa pastille : couleur brute assombrie (clair) ou éclaircie (sombre) de 35 %,
/// pour un contraste d'au moins 3,5:1 sur `badge_fill` (testé dans `workspace.rs`).
pub fn glyph_color(base: Color32, dark: bool) -> Color32 {
    base.lerp_to_gamma(if dark { Color32::WHITE } else { Color32::BLACK }, 0.35)
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
