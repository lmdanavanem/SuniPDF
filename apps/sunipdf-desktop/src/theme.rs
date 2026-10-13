//! Design system SuniPDF : palette et application des visuels egui.

use egui::{Color32, Visuals};

pub const NAVY: Color32 = Color32::from_rgb(0x11, 0x18, 0x27);
pub const BLUE: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const CYAN: Color32 = Color32::from_rgb(0x06, 0xB6, 0xD4);
pub const LIGHT_BG: Color32 = Color32::from_rgb(0xF8, 0xFA, 0xFC);
pub const SURFACE: Color32 = Color32::WHITE;
pub const TEXT: Color32 = Color32::from_rgb(0x0F, 0x17, 0x2A);
pub const ERROR: Color32 = Color32::from_rgb(0xDC, 0x26, 0x26);

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.selection.bg_fill = BLUE;
    v.selection.stroke.color = Color32::WHITE;
    v.hyperlink_color = if dark { CYAN } else { BLUE };
    v.widgets.hovered.bg_stroke.color = BLUE;
    v.widgets.active.bg_fill = BLUE;
    v.widgets.active.fg_stroke.color = Color32::WHITE;
    if dark {
        v.panel_fill = NAVY;
        v.window_fill = Color32::from_rgb(0x1F, 0x29, 0x37);
        v.extreme_bg_color = Color32::from_rgb(0x0B, 0x10, 0x1A);
    } else {
        v.panel_fill = LIGHT_BG;
        v.window_fill = SURFACE;
        v.extreme_bg_color = Color32::from_rgb(0xE2, 0xE8, 0xF0);
        v.override_text_color = Some(TEXT);
    }
    ctx.set_visuals(v);
}

/// Fond de la zone document (gris neutre, distinct de la page blanche).
pub fn canvas_bg(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x0B, 0x10, 0x1A)
    } else {
        Color32::from_rgb(0xCB, 0xD5, 0xE1)
    }
}
