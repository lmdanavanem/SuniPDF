//! Windows system typography with a bundled, MIT-licensed portable icon fallback.
//! The portable font maps the same application codepoints to Fluent System Icons.

use egui::{FontData, FontDefinitions, FontFamily, RichText};
use std::sync::Arc;

pub const ICONS: &str = "icons";
pub const SEMIBOLD: &str = "semibold";

/// Points de code Segoe Fluent Icons (identiques dans Segoe MDL2 Assets).
pub mod glyph {
    pub const MENU: &str = "\u{E700}";
    pub const HOME: &str = "\u{E80F}";
    pub const SAVE: &str = "\u{E74E}";
    pub const SAVE_AS: &str = "\u{E792}";
    pub const FIND: &str = "\u{E721}";
    pub const ZOOM_IN: &str = "\u{E8A3}";
    pub const ZOOM_OUT: &str = "\u{E71F}";
    pub const UNDO: &str = "\u{E7A7}";
    pub const REDO: &str = "\u{E7A6}";
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
    pub const COMBINE: &str = "\u{EA3C}";
    pub const SPLIT: &str = "\u{E8C6}";
    pub const EXTRACT: &str = "\u{E8D1}";
    pub const PICTURE: &str = "\u{E8B9}";
    pub const OPTIMIZE: &str = "\u{E92C}";
    pub const CURSOR: &str = "\u{E8B0}";
    pub const FONT: &str = "\u{E8D2}";
    pub const RENAME: &str = "\u{E8AC}";
    pub const RECT: &str = "\u{E739}";
    /// Carré plein (« CheckboxFill ») : zone à caviarder.
    pub const REDACT: &str = "\u{E73B}";
    pub const ELLIPSE: &str = "\u{EA3A}";
    pub const LINE: &str = "\u{E738}";
    pub const DUPLICATE: &str = "\u{E8C8}";
    /// RotateMapLeft / RotateMapRight : distincts d'Annuler / Rétablir (E7A7 / E7A6).
    pub const ROTATE_LEFT: &str = "\u{E80C}";
    pub const ROTATE_RIGHT: &str = "\u{E80D}";
    pub const FRONT: &str = "\u{E74A}";
    pub const BACK: &str = "\u{E74B}";
    pub const WARNING: &str = "\u{E7BA}";
    pub const SHIELD: &str = "\u{EA18}";
    pub const APP: &str = "\u{E8A5}";
    pub const STAMP: &str = "\u{E8EC}";
    pub const NUMBERS: &str = "\u{E8EF}";
    pub const EXPORT: &str = "\u{EDE1}";
    pub const EDIT_DOC: &str = "\u{E70F}";
    pub const ADD: &str = "\u{E710}";
    pub const CHEVRON_LEFT: &str = "\u{E76B}";
    pub const CHEVRON_RIGHT: &str = "\u{E76C}";
    pub const ARROW_LEFT: &str = "\u{E72B}";
    pub const IMPORT: &str = "\u{E8B5}";
    pub const CROP: &str = "\u{E7A8}";
    pub const CUT: &str = "\u{E8C6}";
    pub const PASTE: &str = "\u{E77F}";
    pub const SELECT_ALL: &str = "\u{E8B3}";
    pub const LAYERS: &str = "\u{E81E}";
    /// Photo : distincte de l'image à insérer (E8B9).
    pub const PHOTO: &str = "\u{E91B}";
    /// Défilement vertical (vue continue).
    pub const SCROLL: &str = "\u{ECE7}";
    /// « Handwriting » : signature visuelle.
    pub const SIGN: &str = "\u{E929}";
    pub const CALENDAR: &str = "\u{E787}";
    /// « CheckList » : remplir un formulaire.
    pub const FORM: &str = "\u{E9D5}";
    /// « MultiSelect » : panneau des champs du formulaire.
    pub const FORM_LIST: &str = "\u{E762}";
    /// Stylo qui trace : dessin à main levée.
    pub const INK: &str = "\u{EC87}";
    /// « Font » en petit : zone de texte libre.
    pub const FREE_TEXT: &str = "\u{E8E9}";
    pub const LINK: &str = "\u{E71B}";
    /// Pointeur et main : sélectionner une annotation.
    pub const SELECT_ANNOT: &str = "\u{E7C9}";
}

/// Install system typography when available and always provide the application's icon family.
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
    let icon_data = icons.map(FontData::from_owned).unwrap_or_else(|| {
        FontData::from_static(include_bytes!("../../../assets/fonts/SuniPDF-Icons.ttf"))
    });
    fonts
        .font_data
        .insert("SuniPDFIcons".into(), Arc::new(icon_data));
    fonts
        .families
        .insert(FontFamily::Name(ICONS.into()), vec!["SuniPDFIcons".into()]);
    ctx.set_fonts(fonts);
    true
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
