//! Polices : Segoe UI (texte) et Segoe Fluent Icons / MDL2 (icônes), chargées depuis
//! `C:\Windows\Fonts` à l'exécution. Absentes → polices egui par défaut, icônes remplacées
//! par leur libellé textuel. Aucune police n'est embarquée dans le binaire.

use egui::{FontData, FontDefinitions, FontFamily, RichText};
use std::sync::Arc;

pub const ICONS: &str = "icons";
pub const SEMIBOLD: &str = "semibold";

/// Points de code Segoe Fluent Icons (identiques dans Segoe MDL2 Assets).
pub mod glyph {
    pub const MENU: &str = "\u{E700}";
    pub const HOME: &str = "\u{E80F}";
    pub const OPEN: &str = "\u{E8E5}";
    pub const SAVE: &str = "\u{E74E}";
    pub const SAVE_AS: &str = "\u{E792}";
    pub const FIND: &str = "\u{E721}";
    pub const ZOOM_IN: &str = "\u{E8A3}";
    pub const ZOOM_OUT: &str = "\u{E71F}";
    pub const ROTATE: &str = "\u{E7AD}";
    pub const UNDO: &str = "\u{E7A7}";
    pub const DELETE: &str = "\u{E74D}";
    pub const CLOSE: &str = "\u{E711}";
    pub const CHEVRON_UP: &str = "\u{E70E}";
    pub const CHEVRON_DOWN: &str = "\u{E70D}";
    pub const HELP: &str = "\u{E897}";
    pub const INFO: &str = "\u{E946}";
    pub const DOCUMENT: &str = "\u{E8A5}";
    pub const SUN: &str = "\u{E706}";
    pub const MOON: &str = "\u{E708}";
    pub const FIT_PAGE: &str = "\u{E9A6}";
    pub const FIT_WIDTH: &str = "\u{E740}";
    pub const GRID: &str = "\u{E8A9}";
    pub const ALL_APPS: &str = "\u{E71D}";
    pub const FOLDER_OPEN: &str = "\u{E838}";
    pub const GLOBE: &str = "\u{E774}";
    pub const READ: &str = "\u{E736}";
    pub const PAGES: &str = "\u{E7C3}";
    pub const SEARCH_BOX: &str = "\u{E721}";
    pub const SELECT: &str = "\u{E8E6}";
    pub const HIGHLIGHT: &str = "\u{E7E6}";
    pub const UNDERLINE: &str = "\u{E8DC}";
    pub const STRIKE: &str = "\u{EDE0}";
    pub const NOTE: &str = "\u{E70B}";
    pub const COMMENT: &str = "\u{E90A}";
    pub const BOOKMARK: &str = "\u{E8A4}";
    pub const LOCK: &str = "\u{E72E}";
    pub const UNLOCK: &str = "\u{E785}";
    pub const ERASE: &str = "\u{E75C}";
    pub const COMBINE: &str = "\u{E8F9}";
    pub const SPLIT: &str = "\u{E8E4}";
    pub const EXTRACT: &str = "\u{E8AB}";
    pub const PICTURE: &str = "\u{E8B9}";
    pub const OPTIMIZE: &str = "\u{E8BA}";
    pub const CURSOR: &str = "\u{E8B0}";
    pub const FONT: &str = "\u{E8D2}";
    pub const RECT: &str = "\u{E739}";
    pub const ELLIPSE: &str = "\u{EA3A}";
    pub const LINE: &str = "\u{E738}";
    pub const DUPLICATE: &str = "\u{E8C8}";
    pub const ROTATE_LEFT: &str = "\u{E7A7}";
    pub const FRONT: &str = "\u{E74A}";
    pub const BACK: &str = "\u{E74B}";
    pub const WARNING: &str = "\u{E7BA}";
    pub const SHIELD: &str = "\u{EA18}";
    pub const APP: &str = "\u{E8A5}";
}

/// Installe les polices système si elles existent. Renvoie `true` si la police d'icônes est disponible.
pub fn install(ctx: &egui::Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let windir = std::env::var_os("WINDIR").map(std::path::PathBuf::from);
    let load = |name: &str| -> Option<Vec<u8>> {
        let dir = windir.as_ref()?;
        std::fs::read(dir.join("Fonts").join(name)).ok()
    };
    if let Some(bytes) = load("segoeui.ttf") {
        fonts
            .font_data
            .insert("SegoeUI".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "SegoeUI".into());
    }
    let mut semibold = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(bytes) = load("seguisb.ttf") {
        fonts
            .font_data
            .insert("SegoeSB".into(), Arc::new(FontData::from_owned(bytes)));
        semibold.insert(0, "SegoeSB".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    let icons = load("SegoeIcons.ttf").or_else(|| load("segmdl2.ttf"));
    let has_icons = icons.is_some();
    if let Some(bytes) = icons {
        fonts
            .font_data
            .insert("SegoeIcons".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .insert(FontFamily::Name(ICONS.into()), vec!["SegoeIcons".into()]);
    }
    ctx.set_fonts(fonts);
    has_icons
}

/// Icône de taille `size`, ou `fallback` (texte) si la police d'icônes manque.
pub fn icon(has_icons: bool, glyph: &str, fallback: &str, size: f32) -> RichText {
    if has_icons {
        RichText::new(glyph)
            .family(FontFamily::Name(ICONS.into()))
            .size(size)
    } else {
        RichText::new(fallback).size(size)
    }
}

pub fn semibold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into())
        .family(FontFamily::Name(SEMIBOLD.into()))
        .size(size)
}
