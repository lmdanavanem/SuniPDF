//! Signature visuelle, moitié « signer » de l'outil Remplir et signer : boîte de dialogue (pavé
//! de dessin, image importée avec fond retiré, bibliothèque locale), paraphe, date du jour, puis
//! placement sur la page d'un objet image (ou texte) ordinaire, sélectionné aussitôt.
//!
//! **Aucune valeur cryptographique** : rien ici ne prouve l'identité du signataire ni l'intégrité
//! du document (le libellé de la commande, le sous-titre et l'encadré de la boîte le disent). La
//! signature numérique par certificat est une fonction distincte.

use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::dialog;
use crate::editor::TextFont;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{EditOp, Pick, Request};
use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sunipdf_core::signature_image as sig;
use sunipdf_core::{EngineError, FontFamily, TextStyle};
use sunipdf_i18n::{tr, tr_args, Language};

#[path = "sign_library.rs"]
pub(crate) mod library;

/// Côté maximal d'une image décodée (import ou bibliothèque), en pixels.
const MAX_SIDE: u32 = 1600;
/// Côté maximal d'une vignette (texture GPU, fantôme de placement).
const THUMB_SIDE: u32 = 320;
/// Largeur maximale de l'aperçu d'import (recalculé à chaque réglage du seuil).
const PREVIEW_W: u32 = 560;
/// Éléments gardés décodés en mémoire, par type.
const MAX_SAVED: usize = 12;
/// Pixels tramés par point logique du pavé.
const RASTER_SCALE: f32 = 2.0;
/// Marge de rognage autour de l'encre, en pixels tramés.
const TRIM_PX: u32 = 6;
const PAD_H: f32 = 170.0;
const CARD: egui::Vec2 = egui::Vec2::new(150.0, 60.0);
const INK_BLACK: [u8; 3] = [0x10, 0x10, 0x10];
const INK_BLUE: [u8; 3] = [0x1E, 0x3A, 0x8A];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SignKind {
    #[default]
    Signature,
    Initials,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Draw,
    Type,
    Import,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Scope {
    #[default]
    Current,
    All,
    Selected,
}

struct TypedPreview {
    source: image::RgbaImage,
    thumb: egui::TextureHandle,
}

impl SignKind {
    fn prefix(self) -> &'static str {
        match self {
            SignKind::Signature => "signature",
            SignKind::Initials => "initials",
        }
    }

    /// Numeric signature/initials names; legacy PNG and current DPAPI files only.
    fn from_file_name(name: &str) -> Option<SignKind> {
        let (prefix, rest) = name.split_once('-')?;
        let digits = rest
            .strip_suffix(".png")
            .or_else(|| rest.strip_suffix(".dpapi"))?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        match prefix {
            "signature" => Some(SignKind::Signature),
            "initials" => Some(SignKind::Initials),
            _ => None,
        }
    }
}

pub struct SavedSignature {
    pub path: PathBuf,
    pub kind: SignKind,
    pub image: Arc<DynamicImage>,
    pub thumb: egui::TextureHandle,
}

/// Ce que le prochain clic sur la page pose.
#[derive(Clone)]
pub enum Placing {
    Image {
        image: Arc<DynamicImage>,
        thumb: egui::TextureHandle,
        width_pt: f32,
        /// hauteur / largeur
        aspect: f32,
        pages: Option<Vec<usize>>,
    },
    Text(String),
}

struct Import {
    /// Image décodée par le thread moteur, au plus `MAX_SIDE` de côté.
    source: image::RgbaImage,
    /// Réduction pour l'aperçu.
    small: image::RgbaImage,
    remove_bg: bool,
    threshold: u8,
    /// Aperçu construit pour (`remove_bg`, `threshold`).
    preview: Option<(bool, u8, egui::TextureHandle)>,
}

pub struct SignState {
    /// Boîte ouverte, et sur quel type.
    pub open: Option<SignKind>,
    saved: Vec<SavedSignature>,
    mode: Mode,
    typed_text: String,
    typed_font: TextFont,
    font_filter: String,
    typed_preview: Option<TypedPreview>,
    preview_id: u64,
    preview_pending: Option<u64>,
    scope: Scope,
    selected_pages: Vec<usize>,
    /// Traits relatifs au coin haut-gauche du pavé, en points logiques.
    strokes: Vec<Vec<Pos2>>,
    drawing: bool,
    /// Largeur du pavé telle que dessinée (échelle du tramage).
    pad_w: f32,
    pen_blue: bool,
    pen_width: f32,
    import: Option<Import>,
    import_pending: Option<PathBuf>,
    /// « Enregistrer sur cet ordinateur ».
    save: bool,
    error: Option<&'static str>,
    confirm_delete: Option<PathBuf>,
    pub placing: Option<Placing>,
    pub busy: bool,
    pub done: usize,
    pub total: usize,
    pub cancel: Arc<AtomicBool>,
}

impl Default for SignState {
    fn default() -> Self {
        Self {
            open: None,
            saved: Vec::new(),
            mode: Mode::Draw,
            typed_text: String::new(),
            typed_font: TextFont::Standard(FontFamily::Times),
            font_filter: String::new(),
            typed_preview: None,
            preview_id: 0,
            preview_pending: None,
            scope: Scope::Current,
            selected_pages: Vec::new(),
            strokes: Vec::new(),
            drawing: false,
            pad_w: 556.0,
            pen_blue: false,
            pen_width: 3.0,
            import: None,
            import_pending: None,
            save: true,
            error: None,
            confirm_delete: None,
            placing: None,
            busy: false,
            done: 0,
            total: 0,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl SignState {
    fn invalidate_preview(&mut self) {
        self.preview_id = self.preview_id.wrapping_add(1);
        self.preview_pending = None;
        self.typed_preview = None;
    }

    fn ready(&self) -> bool {
        match self.mode {
            Mode::Draw => !self.strokes.is_empty(),
            Mode::Type => self.typed_preview.is_some() && self.preview_pending.is_none(),
            Mode::Import => self.import.is_some() && self.import_pending.is_none(),
        }
    }

    fn preview_request(&mut self) -> Option<(u64, String, TextStyle)> {
        if self.mode != Mode::Type || self.typed_text.trim().is_empty() {
            return None;
        }
        self.invalidate_preview();
        let id = self.preview_id;
        self.preview_pending = Some(id);
        let rgb = ink(self.pen_blue);
        let mut style = TextStyle {
            size_pt: 48.0,
            italic: true,
            rgb: (rgb[0], rgb[1], rgb[2]),
            ..Default::default()
        };
        match &self.typed_font {
            TextFont::Standard(family) => style.family = *family,
            TextFont::System(font) => style.font_file = Some(font.path.clone()),
        }
        Some((id, self.typed_text.clone(), style))
    }

    fn target_pages(&self, count: usize) -> Option<Vec<usize>> {
        match self.scope {
            Scope::Current => None,
            Scope::All => Some((0..count).collect()),
            Scope::Selected => Some(self.selected_pages.clone()),
        }
    }

    pub fn begin(&mut self, total: usize) {
        self.open = None;
        self.busy = true;
        self.done = 0;
        self.total = total;
        self.cancel.store(false, Ordering::Relaxed);
    }

    fn show_progress(&self, ctx: &egui::Context, lang: Language, dark: bool, has_icons: bool) {
        let mut cancel = false;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "signature_progress",
                tr(lang, "signature.title"),
                dark,
                has_icons,
            ),
            |ui| {
                ui.label(tr_args(
                    lang,
                    "signature.progress",
                    &[
                        ("done", &self.done.to_string()),
                        ("total", &self.total.to_string()),
                    ],
                ));
                ui.add(
                    egui::ProgressBar::new(self.done as f32 / self.total.max(1) as f32)
                        .show_percentage(),
                );
                if self.cancel.load(Ordering::Relaxed) {
                    ui.label(tr(lang, "ocr.cancelling"));
                }
            },
            |ui| {
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if cancel {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

enum Act {
    Place,
    Use(usize),
    Delete(PathBuf),
    PickImage,
    Preview,
    Cancel,
}

/// Dossier des signatures enregistrées : `SUNIPDF_SIGNATURES_DIR` (tests, captures), sinon
/// `%APPDATA%\SuniPDF\signatures`, protected for the current Windows user with DPAPI.
pub fn signatures_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SUNIPDF_SIGNATURES_DIR") {
        return Some(PathBuf::from(dir));
    }
    Some(
        crate::settings::Settings::path()?
            .parent()?
            .join("signatures"),
    )
}

/// Legacy PNG/current DPAPI signatures, newest first, at most twelve per kind.
fn list_saved(dir: &Path) -> Vec<(PathBuf, SignKind)> {
    // ponytail: plafond fixe de 12 par type, images gardées décodées en mémoire.
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, SignKind)> = entries
        .flatten()
        .filter_map(|e| {
            if !e.file_type().ok()?.is_file() {
                return None;
            }
            let path = e.path();
            let kind = SignKind::from_file_name(path.file_name()?.to_str()?)?;
            if path.extension().is_some_and(|e| e == "dpapi") && path.with_extension("png").exists()
            {
                return None;
            }
            Some((path, kind))
        })
        .collect();
    // Nom = préfixe + millisecondes Unix (13 chiffres) : tri par nom décroissant = par date.
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let mut per_kind = [0usize; 2];
    found.retain(|(_, k)| {
        per_kind[*k as usize] += 1;
        per_kind[*k as usize] <= MAX_SAVED
    });
    found
}

fn new_file_name(kind: SignKind) -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{}-{ms:013}.png", kind.prefix())
}

/// Largeur par défaut : une signature d'au plus 160 pt de large et 64 pt de haut, un paraphe
/// d'au plus 64 × 40 pt (`aspect` = hauteur / largeur).
fn default_width(kind: SignKind, aspect: f32) -> f32 {
    match kind {
        SignKind::Signature => 160.0_f32.min(64.0 / aspect),
        SignKind::Initials => 64.0_f32.min(40.0 / aspect),
    }
}

/// Date formatée selon le catalogue (`{d}`, `{m}`, `{y}`) : jamais de motif strftime, donc
/// jamais de panique sur un motif invalide.
fn date_text(lang: Language, year: i32, month: u32, day: u32) -> String {
    tr_args(
        lang,
        "signature.date_format",
        &[
            ("d", &format!("{day:02}")),
            ("m", &format!("{month:02}")),
            ("y", &year.to_string()),
        ],
    )
}

pub fn today_text(lang: Language) -> String {
    use chrono::Datelike;
    let now = chrono::Local::now();
    date_text(lang, now.year(), now.month(), now.day())
}

fn texture(ctx: &egui::Context, name: &str, img: &image::RgbaImage) -> egui::TextureHandle {
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [img.width() as usize, img.height() as usize],
            img.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

fn thumb_of(img: &DynamicImage) -> image::RgbaImage {
    if img.width().max(img.height()) > THUMB_SIDE {
        img.thumbnail(THUMB_SIDE, THUMB_SIDE).into_rgba8()
    } else {
        img.to_rgba8()
    }
}

/// Rectangle « contenir » de `size` dans `inner`, centré.
fn fit(inner: Rect, size: egui::Vec2) -> Rect {
    let k = (inner.width() / size.x).min(inner.height() / size.y);
    Rect::from_center_size(inner.center(), size * k)
}

const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

fn ink(blue: bool) -> [u8; 3] {
    if blue {
        INK_BLUE
    } else {
        INK_BLACK
    }
}

/// Point d'ancrage (bas-gauche) d'un texte posé, maintenu dans la page.
fn text_anchor(pt: (f32, f32), text: &str, size_pt: f32, page: (f32, f32)) -> (f32, f32) {
    // ponytail: largeur estimée à 0,556 em par caractère (chiffres Helvetica), sans mesure de
    // police ; une vraie mesure demanderait un aller-retour par le thread moteur.
    let w = text.chars().count() as f32 * 0.556 * size_pt;
    (
        pt.0.clamp(0.0, (page.0 - w).max(0.0)),
        pt.1.clamp(0.0, (page.1 - size_pt).max(0.0)),
    )
}

/// Journal sans chemin complet (donnée personnelle) : le nom du fichier seulement.
fn warn_unreadable(what: &str, path: &Path, e: &EngineError) {
    let full = format!("{}: ", path.display());
    let msg = e.to_string();
    let msg = msg.strip_prefix(&full).unwrap_or(&msg);
    log::warn!("{what} ({}) : {msg}", crate::app::file_name(path));
}

impl SuniApp {
    pub fn open_sign_dialog(&mut self, kind: SignKind) {
        self.cancel_placing();
        let selected_pages = if self.organize.active {
            self.organize.selected.iter().copied().collect()
        } else {
            Vec::new()
        };
        let st = &mut self.sign;
        st.invalidate_preview();
        st.scope = Scope::Current;
        st.selected_pages = selected_pages;
        st.open = Some(kind);
        st.error = None;
        st.confirm_delete = None;
        st.drawing = false;
        self.refresh_library();
    }

    /// The worker decrypts/migrates bounded library files and decodes their PNG in memory.
    fn refresh_library(&mut self) {
        let Some(dir) = signatures_dir() else { return };
        let listed = list_saved(&dir);
        self.sign
            .saved
            .retain(|s| listed.iter().any(|(p, _)| *p == s.path));
        for (path, _) in listed {
            if !self.sign.saved.iter().any(|s| s.path == path) {
                self.worker.send(Request::LoadImage {
                    path,
                    max_side: MAX_SIDE,
                });
            }
        }
    }

    pub fn sign_dialog(&mut self, ctx: &egui::Context) {
        if self.sign.busy {
            self.sign
                .show_progress(ctx, self.lang(), self.settings.dark_theme, self.has_icons);
            return;
        }
        let Some(kind_open) = self.sign.open else {
            return;
        };
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let p = theme::palette(dark);
        let dir_text = signatures_dir().map_or_else(String::new, |d| d.display().to_string());
        let mut kind = kind_open;
        let fonts = self.edit.system_fonts.clone();
        let mut want_fonts = false;
        let title = match kind {
            SignKind::Signature => "signature.title",
            SignKind::Initials => "signature.title_initials",
        };
        let spec = dialog::Spec::new("sign", tr(lang, title), dark, has_icons)
            .subtitle(tr(lang, "signature.subtitle"))
            .icon(glyph::SIGN, "✍")
            .width(600.0);
        let st = &mut self.sign;
        // Valeur de l'image précédente, comme « Apposer » du tampon.
        let ready = st.ready();
        let mut body_act = None;
        let mut foot_act = None;
        let mut new_confirm: Option<Option<PathBuf>> = None;
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut kind,
                        SignKind::Signature,
                        tr(lang, "signature.kind.signature"),
                    );
                    ui.selectable_value(
                        &mut kind,
                        SignKind::Initials,
                        tr(lang, "signature.kind.initials"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "signature.scope"));
                    ui.selectable_value(
                        &mut st.scope,
                        Scope::Current,
                        tr(lang, "stamp.scope.current"),
                    );
                    ui.selectable_value(&mut st.scope, Scope::All, tr(lang, "stamp.scope.all"));
                    ui.add_enabled_ui(!st.selected_pages.is_empty(), |ui| {
                        ui.selectable_value(
                            &mut st.scope,
                            Scope::Selected,
                            tr(lang, "signature.scope.selected"),
                        );
                    });
                });
                if st.scope != Scope::Current {
                    ui.label(
                        RichText::new(tr(lang, "signature.scope.hint"))
                            .small()
                            .color(p.text_muted),
                    );
                }
                // Encadré permanent : signature visuelle, sans valeur cryptographique.
                let (bg, fg) = if dark {
                    (
                        Color32::from_rgb(0x45, 0x30, 0x0E),
                        Color32::from_rgb(0xFC, 0xD3, 0x4D),
                    )
                } else {
                    (
                        Color32::from_rgb(0xFE, 0xF3, 0xC7),
                        Color32::from_rgb(0x92, 0x40, 0x0E),
                    )
                };
                egui::Frame::new()
                    .fill(bg)
                    .corner_radius(10)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            ui.label(icons::icon(has_icons, glyph::WARNING, "!", 16.0).color(fg));
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(tr(lang, "signature.disclaimer"))
                                        .size(12.5)
                                        .color(fg),
                                )
                                .wrap(),
                            );
                        });
                    });

                // Bibliothèque locale.
                dialog::field_label(ui, tr(lang, "signature.saved"));
                let mine: Vec<usize> = st
                    .saved
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.kind == kind)
                    .map(|(i, _)| i)
                    .collect();
                if mine.is_empty() {
                    ui.label(
                        RichText::new(tr(lang, "signature.saved.empty"))
                            .size(13.0)
                            .color(p.text_muted),
                    );
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for (n, i) in mine.iter().enumerate() {
                            let s = &st.saved[*i];
                            let (rect, resp) = ui.allocate_exact_size(CARD, Sense::click());
                            resp.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    tr_args(
                                        lang,
                                        match kind {
                                            SignKind::Signature => "a11y.signature_saved",
                                            SignKind::Initials => "a11y.initials_saved",
                                        },
                                        &[("n", &(n + 1).to_string())],
                                    ),
                                )
                            });
                            let resp = resp
                                .on_hover_text(tr(lang, "signature.saved.use"))
                                .on_hover_cursor(CursorIcon::PointingHand);
                            let trash = Rect::from_min_size(
                                rect.right_top() + egui::vec2(-24.0, 4.0),
                                egui::vec2(20.0, 20.0),
                            );
                            let del = ui
                                .interact(trash, resp.id.with("delete"), Sense::click())
                                .on_hover_text(tr(lang, "signature.saved.delete"))
                                .on_hover_cursor(CursorIcon::PointingHand);
                            let confirming = st.confirm_delete.as_ref() == Some(&s.path);
                            // Rien ne déborde de la carte (libellé de confirmation long).
                            let painter = ui.painter().with_clip_rect(rect);
                            let hot = resp.hovered() || resp.has_focus();
                            // Papier blanc dans les deux thèmes : de l'encre noire sur fond sombre
                            // serait illisible.
                            painter.rect(
                                rect,
                                8.0,
                                Color32::WHITE,
                                Stroke::new(
                                    if hot { 2.0 } else { 1.0 },
                                    if hot { p.accent } else { p.border },
                                ),
                                StrokeKind::Inside,
                            );
                            let inner = Rect::from_min_max(
                                rect.min + egui::vec2(8.0, 8.0),
                                rect.max - egui::vec2(28.0, 8.0),
                            );
                            painter.image(
                                s.thumb.id(),
                                fit(inner, s.thumb.size_vec2()),
                                UV,
                                Color32::WHITE,
                            );
                            let del_color = if confirming || del.hovered() {
                                theme::ERROR
                            } else {
                                Color32::from_gray(150)
                            };
                            painter.text(
                                trash.center(),
                                Align2::CENTER_CENTER,
                                if has_icons { glyph::DELETE } else { "x" },
                                if has_icons {
                                    FontId::new(13.0, egui::FontFamily::Name(icons::ICONS.into()))
                                } else {
                                    FontId::proportional(13.0)
                                },
                                del_color,
                            );
                            if confirming {
                                painter.text(
                                    rect.left_bottom() + egui::vec2(8.0, -5.0),
                                    Align2::LEFT_BOTTOM,
                                    tr(lang, "signature.saved.delete_confirm"),
                                    FontId::proportional(10.5),
                                    theme::ERROR,
                                );
                            }
                            if del.clicked() {
                                if confirming {
                                    body_act = Some(Act::Delete(s.path.clone()));
                                } else {
                                    new_confirm = Some(Some(s.path.clone()));
                                }
                            } else if resp.clicked() {
                                body_act = Some(Act::Use(*i));
                            }
                        }
                    });
                }

                // Nouvelle signature : dessiner, saisir ou importer.
                dialog::field_label(
                    ui,
                    tr(
                        lang,
                        match kind {
                            SignKind::Signature => "signature.new",
                            SignKind::Initials => "signature.new_initials",
                        },
                    ),
                );
                let mode_before = st.mode;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut st.mode, Mode::Draw, tr(lang, "signature.mode.draw"));
                    ui.selectable_value(&mut st.mode, Mode::Type, tr(lang, "signature.mode.type"));
                    ui.selectable_value(
                        &mut st.mode,
                        Mode::Import,
                        tr(lang, "signature.mode.import"),
                    );
                });
                if st.mode != mode_before || kind != kind_open {
                    st.invalidate_preview();
                    st.error = None;
                }
                match st.mode {
                    Mode::Draw => draw_pad(ui, st, lang, &p),
                    Mode::Type => {
                        let (act, wanted) =
                            typed_section(ui, st, lang, &p, fonts.as_deref().map(Vec::as_slice));
                        want_fonts |= wanted;
                        if act.is_some() {
                            body_act = act;
                        }
                    }
                    Mode::Import => {
                        if let Some(a) = import_section(ui, st, lang, &p) {
                            body_act = Some(a);
                        }
                    }
                }

                ui.add_space(2.0);
                ui.checkbox(&mut st.save, tr(lang, "signature.save"));
                ui.add(
                    egui::Label::new(
                        RichText::new(tr_args(lang, "signature.save_note", &[("path", &dir_text)]))
                            .size(12.0)
                            .color(p.text_muted),
                    )
                    .wrap(),
                );
                if let Some(key) = st.error {
                    ui.colored_label(p.error, tr(lang, key));
                }
            },
            |ui| {
                ui.add_enabled_ui(ready, |ui| {
                    if dialog::primary(ui, tr(lang, "signature.ok")) {
                        foot_act = Some(Act::Place);
                    }
                });
                if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    foot_act = Some(Act::Cancel);
                }
            },
        );
        if self.sign.open.is_some() {
            self.sign.open = Some(kind);
        }
        if want_fonts {
            self.worker.send(Request::SystemFonts);
        }
        if let Some(c) = new_confirm {
            self.sign.confirm_delete = c;
        }
        match body_act.or(foot_act) {
            Some(Act::Cancel) => {
                self.sign.invalidate_preview();
                self.sign.open = None;
                self.sign.drawing = false;
            }
            Some(Act::Delete(path)) => {
                let result = signatures_dir()
                    .ok_or_else(|| EngineError::Other("signature library is unavailable".into()))
                    .and_then(|dir| library::delete(&dir, &path));
                if let Err(e) = result {
                    log::warn!(
                        "suppression impossible ({}) : {e}",
                        crate::app::file_name(&path)
                    );
                    self.sign.error = Some("signature.library.error");
                    return;
                }
                self.sign.saved.retain(|s| s.path != path);
                self.sign.confirm_delete = None;
                // La 13e signature, au-delà du plafond, prend la place libérée.
                self.refresh_library();
            }
            Some(Act::PickImage) => {
                // `SUNIPDF_PICK_FILE` remplace le sélecteur (captures et automatisation, comme
                // `SUNIPDF_STARTUP_COMMANDS`) : le sélecteur natif ne se pilote pas.
                let picked = match std::env::var_os("SUNIPDF_PICK_FILE") {
                    Some(p) => Some(PathBuf::from(p)),
                    None => rfd::FileDialog::new()
                        .set_title(tr(lang, "signature.import.pick_title"))
                        .add_filter(
                            tr(lang, "tools.images.filter"),
                            crate::editor::IMAGE_EXTENSIONS,
                        )
                        .pick_file(),
                };
                if let Some(path) = picked {
                    self.sign.import = None;
                    self.sign.error = None;
                    self.sign.import_pending = Some(path.clone());
                    self.worker.send(Request::LoadImage {
                        path,
                        max_side: MAX_SIDE,
                    });
                }
            }
            Some(Act::Use(i)) => {
                let s = &self.sign.saved[i];
                let aspect = s.image.height() as f32 / s.image.width().max(1) as f32;
                let placing = Placing::Image {
                    image: s.image.clone(),
                    thumb: s.thumb.clone(),
                    width_pt: default_width(s.kind, aspect),
                    aspect,
                    pages: self
                        .sign
                        .target_pages(self.document.as_ref().map_or(0, |d| d.info.page_count())),
                };
                self.sign.invalidate_preview();
                self.sign.open = None;
                self.start_placing(placing, "signature.placing_hint");
            }
            Some(Act::Place) => self.place_new(ctx, kind),
            Some(Act::Preview) => {
                if let Some((id, text, style)) = self.sign.preview_request() {
                    self.sign.error = None;
                    self.worker
                        .send(Request::RenderSignature { id, text, style });
                }
            }
            None => {}
        }
    }

    /// Trame ou prépare l'image, l'enregistre si demandé, puis passe en mode de placement.
    fn place_new(&mut self, ctx: &egui::Context, kind: SignKind) {
        let count = self.document.as_ref().map_or(0, |d| d.info.page_count());
        let st = &mut self.sign;
        let img = if st.mode == Mode::Draw {
            let pts: Vec<Vec<(f32, f32)>> = st
                .strokes
                .iter()
                .map(|s| {
                    s.iter()
                        .map(|q| (q.x * RASTER_SCALE, q.y * RASTER_SCALE))
                        .collect()
                })
                .collect();
            let w = (st.pad_w * RASTER_SCALE).round() as u32;
            let h = (PAD_H * RASTER_SCALE).round() as u32;
            let rgb = ink(st.pen_blue);
            sig::trim(
                &sig::rasterize_strokes(
                    &pts,
                    w,
                    h,
                    st.pen_width * RASTER_SCALE,
                    (rgb[0], rgb[1], rgb[2]),
                ),
                TRIM_PX,
            )
        } else if st.mode == Mode::Type {
            st.typed_preview.as_ref().map(|p| p.source.clone())
        } else {
            let Some(imp) = &st.import else { return };
            let src = if imp.remove_bg {
                sig::remove_background(&imp.source, imp.threshold)
            } else {
                imp.source.clone()
            };
            sig::trim(&src, TRIM_PX)
        };
        let Some(img) = img else {
            st.error = Some("signature.error.empty");
            return;
        };
        let aspect = img.height() as f32 / img.width() as f32;
        let mut save_failed = false;
        let mut saved_path = None;
        if st.save {
            match signatures_dir() {
                Some(dir) => {
                    let name = new_file_name(kind);
                    match library::save(&dir, &name, &img) {
                        Ok(path) => saved_path = Some(path),
                        // Jamais de contenu dans le journal : seulement le nom du fichier.
                        Err(e) => {
                            log::warn!("signature non enregistrée ({}) : {e}", name);
                            save_failed = true;
                        }
                    }
                }
                None => save_failed = true,
            }
        }
        let image = Arc::new(DynamicImage::ImageRgba8(img));
        let thumb = texture(ctx, "sign-new", &thumb_of(&image));
        if let Some(path) = saved_path {
            st.saved.insert(
                0,
                SavedSignature {
                    path,
                    kind,
                    image: image.clone(),
                    thumb: thumb.clone(),
                },
            );
        }
        st.open = None;
        st.strokes.clear();
        st.drawing = false;
        st.import = None;
        st.invalidate_preview();
        let placing = Placing::Image {
            image,
            thumb,
            width_pt: default_width(kind, aspect),
            aspect,
            pages: st.target_pages(count),
        };
        self.start_placing(placing, "signature.placing_hint");
        if save_failed {
            // La pose continue ; l'échec est annoncé dans le bandeau.
            self.error = Some("signature.error.save");
        }
    }

    pub fn on_signature_rendered(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        result: Result<image::RgbaImage, EngineError>,
    ) {
        let st = &mut self.sign;
        if st.open.is_none() || st.mode != Mode::Type || st.preview_pending != Some(id) {
            return;
        }
        st.preview_pending = None;
        match result {
            Ok(source) if !sig::is_blank(&source) => {
                let image = DynamicImage::ImageRgba8(source.clone());
                st.typed_preview = Some(TypedPreview {
                    source,
                    thumb: texture(ctx, "sign-typed", &thumb_of(&image)),
                });
                st.error = None;
            }
            Ok(_) => st.error = Some("signature.error.empty"),
            Err(e) => st.error = Some(crate::app::error_key(&e)),
        }
        ctx.request_repaint();
    }

    /// Image décodée par le thread moteur : l'import en attente, sinon un fichier de la
    /// bibliothèque demandé par `refresh_library`.
    pub fn on_image_loaded(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
        result: Result<image::RgbaImage, EngineError>,
    ) {
        let st = &mut self.sign;
        if st.import_pending.as_ref() == Some(&path) {
            st.import_pending = None;
            match result {
                Ok(source) => {
                    let small = if source.width() > PREVIEW_W {
                        DynamicImage::ImageRgba8(source.clone())
                            .thumbnail(PREVIEW_W, PREVIEW_W)
                            .into_rgba8()
                    } else {
                        source.clone()
                    };
                    // Un PNG déjà transparent est pris tel quel ; une photo a besoin du seuil.
                    let has_alpha = source.pixels().any(|p| p[3] < 255);
                    st.import = Some(Import {
                        source,
                        small,
                        remove_bg: !has_alpha,
                        threshold: 200,
                        preview: None,
                    });
                    st.error = None;
                }
                Err(e) => {
                    warn_unreadable("image de signature illisible", &path, &e);
                    st.error = Some("signature.error.image");
                }
            }
            ctx.request_repaint();
            return;
        }
        let kind = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(SignKind::from_file_name);
        let Some(kind) = kind else { return };
        if st.saved.iter().any(|s| s.path == path) {
            return;
        }
        match result {
            Ok(img) => {
                if path.extension().is_some_and(|e| e == "png") {
                    st.error = Some("signature.library.migration_failed");
                }
                let image = DynamicImage::ImageRgba8(img);
                let thumb = texture(ctx, "sign-saved", &thumb_of(&image));
                st.saved.push(SavedSignature {
                    path,
                    kind,
                    image: Arc::new(image),
                    thumb,
                });
                st.saved.sort_by(|a, b| b.path.cmp(&a.path));
                ctx.request_repaint();
            }
            Err(e) => {
                warn_unreadable("signature enregistrée illisible", &path, &e);
                st.error = Some("signature.library.error");
            }
        }
    }

    fn start_placing(&mut self, p: Placing, hint_key: &str) {
        self.sign.placing = Some(p);
        self.organize.active = false;
        self.edit.selection = None;
        self.annotate.drag = None;
        self.status_msg = Some(tr(self.lang(), hint_key).to_string());
    }

    pub fn cancel_placing(&mut self) {
        self.sign.open = None;
        self.sign.invalidate_preview();
        self.sign.scope = Scope::Current;
        self.sign.selected_pages.clear();
        self.sign.import_pending = None;
        if self.sign.placing.take().is_some() {
            self.status_msg = None;
        }
    }

    pub fn start_placing_date(&mut self) {
        let text = today_text(self.lang());
        self.start_placing(Placing::Text(text), "signature.placing_hint_date");
    }

    /// Pose l'élément sous le clic (points PDF). `keep` (Maj) garde le mode actif pour poser
    /// plusieurs fois ; sinon l'objet posé est sélectionné dans l'outil de sélection d'objets.
    pub fn place_at(&mut self, page: usize, pt: (f32, f32), keep: bool) {
        let Some(placing) = self.sign.placing.clone() else {
            return;
        };
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page).copied())
        else {
            return;
        };
        let hint = if keep { self.status_msg.clone() } else { None };
        // Sans Maj, le dernier objet de la page est resélectionné à la réponse du moteur ; avec
        // Maj le mode de placement continue, sans poignées sur l'objet posé.
        self.edit.reselect_many = (!keep).then_some((page, Pick::Last(1)));
        // `DocumentChanged` remplace le message d'état par celui-ci (conservé avec Maj).
        self.edit.pending_status = hint;
        match placing {
            Placing::Image {
                image,
                width_pt,
                aspect,
                pages,
                ..
            } => {
                if let Some(pages) = pages {
                    if pages.is_empty() {
                        self.error = Some("signature.error.pages");
                        return;
                    }
                    self.edit.reselect_many = None;
                    self.sign.begin(pages.len());
                    self.worker.send(Request::PlaceSignature {
                        pages,
                        image,
                        center: pt,
                        width_pt,
                        cancel: self.sign.cancel.clone(),
                    });
                } else {
                    let r = match sig::placement_rect(
                        pt,
                        width_pt,
                        aspect,
                        (size.width_pt, size.height_pt),
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            self.error = Some(crate::app::error_key(&e));
                            return;
                        }
                    };
                    self.worker.send(Request::Edit(EditOp::AddImageData {
                        page,
                        image,
                        x: r.left,
                        y: r.bottom,
                        width_pt: r.right - r.left,
                    }));
                }
            }
            Placing::Text(text) => {
                let style = TextStyle::default();
                let (x, y) = text_anchor(pt, &text, style.size_pt, (size.width_pt, size.height_pt));
                self.worker.send(Request::Edit(EditOp::AddText {
                    page,
                    x,
                    y,
                    text,
                    style,
                }));
            }
        }
        if !keep {
            self.sign.placing = None;
            self.annotate.tool = crate::annotate::Tool::SelectObject;
            self.annotate.drag = None;
            self.edit.drag_mode = None;
        }
    }

    /// Fantôme de l'élément à poser, sous le pointeur (`pt` en points PDF de cette page).
    pub fn paint_placing(
        &self,
        painter: &egui::Painter,
        page_rect: Rect,
        page_index: usize,
        page_h_pt: f32,
        pt: (f32, f32),
    ) {
        let Some(placing) = &self.sign.placing else {
            return;
        };
        if self
            .document
            .as_ref()
            .is_none_or(|d| page_index >= d.info.page_count())
        {
            return;
        }
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let to_screen = |x: f32, y: f32| page_rect.min + egui::vec2(x * k, (page_h_pt - y) * k);
        match placing {
            Placing::Image {
                thumb,
                width_pt,
                aspect,
                ..
            } => {
                let Ok(r) =
                    sig::placement_rect(pt, *width_pt, *aspect, (page_rect.width() / k, page_h_pt))
                else {
                    return;
                };
                let screen =
                    Rect::from_min_max(to_screen(r.left, r.top), to_screen(r.right, r.bottom));
                painter.image(thumb.id(), screen, UV, Color32::from_white_alpha(170));
                painter.rect_stroke(
                    screen.expand(2.0),
                    3.0,
                    Stroke::new(1.0, theme::BLUE),
                    StrokeKind::Outside,
                );
            }
            Placing::Text(text) => {
                let size_pt = TextStyle::default().size_pt;
                let (x, y) = text_anchor(pt, text, size_pt, (page_rect.width() / k, page_h_pt));
                painter.text(
                    to_screen(x, y),
                    Align2::LEFT_BOTTOM,
                    text,
                    FontId::proportional(size_pt * k),
                    Color32::from_black_alpha(170),
                );
            }
        }
    }
}

/// Pavé de signature : papier blanc, traits saisis à la souris, au pavé tactile ou au stylet.
fn draw_pad(ui: &mut egui::Ui, st: &mut SignState, lang: Language, p: &theme::Palette) {
    let width = ui.available_width();
    st.pad_w = width;
    let (resp, painter) = ui.allocate_painter(egui::vec2(width, PAD_H), Sense::drag());
    let rect = resp.rect;
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Other,
            true,
            tr(lang, "a11y.signature_pad"),
        )
    });
    painter.rect(
        rect,
        8.0,
        Color32::WHITE,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    // Ligne de base, comme un bordereau.
    let base_y = rect.bottom() - 40.0;
    painter.hline(
        (rect.left() + 24.0)..=(rect.right() - 24.0),
        base_y,
        Stroke::new(1.0, Color32::from_gray(205)),
    );
    let down = resp.is_pointer_button_down_on();
    // L'image du relâchement compte aussi : un trait appuyé et relâché dans la même image (stylet
    // rapide, automatisation) garderait sinon son seul point d'appui.
    if down || (st.drawing && resp.drag_stopped()) {
        let first = !st.drawing;
        if first {
            st.strokes.push(Vec::new());
            st.drawing = true;
        }
        // Tous les déplacements reçus pendant l'image (souris à 125–1000 Hz), sauf à la première
        // image du trait, où ils précèdent l'appui.
        let mut pts: Vec<Pos2> = if first {
            Vec::new()
        } else {
            ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::PointerMoved(p) => Some(*p),
                        _ => None,
                    })
                    .collect()
            })
        };
        pts.extend(resp.interact_pointer_pos());
        if let Some(stroke) = st.strokes.last_mut() {
            for q in pts {
                let q = (q.clamp(rect.min, rect.max) - rect.min).to_pos2();
                if stroke.last().is_none_or(|l| l.distance(q) > 0.5) {
                    stroke.push(q);
                }
            }
        }
        st.drawing = down;
    } else {
        st.drawing = false;
    }
    let rgb = ink(st.pen_blue);
    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    for s in &st.strokes {
        let pts: Vec<Pos2> = s.iter().map(|q| rect.min + q.to_vec2()).collect();
        match pts.as_slice() {
            [] => {}
            [one] => {
                painter.circle_filled(*one, st.pen_width / 2.0, color);
            }
            _ => {
                painter.add(egui::Shape::line(pts, Stroke::new(st.pen_width, color)));
            }
        }
    }
    if st.strokes.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            tr(lang, "signature.pad.hint"),
            FontId::proportional(14.0),
            Color32::from_gray(150),
        );
    }
    ui.horizontal(|ui| {
        ui.label(tr(lang, "signature.pad.ink"));
        for (blue, key) in [
            (false, "signature.pad.ink.black"),
            (true, "signature.pad.ink.blue"),
        ] {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
            let selected = st.pen_blue == blue;
            resp.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::RadioButton,
                    true,
                    selected,
                    tr(lang, key),
                )
            });
            let resp = resp
                .on_hover_text(tr(lang, key))
                .on_hover_cursor(CursorIcon::PointingHand);
            let c = ink(blue);
            // Encre sur papier blanc : lisible aussi en thème sombre.
            ui.painter()
                .circle(r.center(), 9.5, Color32::WHITE, Stroke::new(1.0, p.border));
            ui.painter()
                .circle_filled(r.center(), 6.5, Color32::from_rgb(c[0], c[1], c[2]));
            if selected || resp.has_focus() {
                ui.painter()
                    .circle_stroke(r.center(), 11.5, Stroke::new(2.0, p.accent));
            }
            if resp.clicked() {
                st.pen_blue = blue;
            }
        }
        ui.add_space(10.0);
        ui.label(tr(lang, "signature.pad.width"));
        ui.scope(|ui| {
            // Rail visible : le fond « inactif » du thème est la couleur de surface.
            ui.visuals_mut().widgets.inactive.bg_fill = p.border;
            ui.spacing_mut().slider_width = 110.0;
            ui.add(egui::Slider::new(&mut st.pen_width, 1.5..=6.0).show_value(false));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let has = !st.strokes.is_empty();
            if ui
                .add_enabled(has, egui::Button::new(tr(lang, "signature.pad.clear")))
                .clicked()
            {
                st.strokes.clear();
            }
            if ui
                .add_enabled(has, egui::Button::new(tr(lang, "signature.pad.undo")))
                .clicked()
            {
                st.strokes.pop();
            }
        });
    });
}

/// Saisie et aperçu produit par le moteur, avec le même sélecteur de polices que l'édition.
fn typed_section(
    ui: &mut egui::Ui,
    st: &mut SignState,
    lang: Language,
    p: &theme::Palette,
    fonts: Option<&[sunipdf_core::SystemFont]>,
) -> (Option<Act>, bool) {
    let before = (st.typed_text.clone(), st.typed_font.clone(), st.pen_blue);
    dialog::field_label(ui, tr(lang, "signature.type.name"));
    ui.add(
        egui::TextEdit::singleline(&mut st.typed_text)
            .id(ui.make_persistent_id("signature-name"))
            .char_limit(256)
            .desired_width(f32::INFINITY)
            .hint_text(tr(lang, "signature.type.hint")),
    );
    let mut want_fonts = false;
    ui.horizontal(|ui| {
        ui.label(tr(lang, "dialog.text.font"));
        want_fonts = crate::inline_text::font_picker(
            ui,
            lang,
            &mut st.typed_font,
            &mut st.font_filter,
            fonts,
        )
        .0;
        for (blue, key) in [
            (false, "signature.pad.ink.black"),
            (true, "signature.pad.ink.blue"),
        ] {
            ui.selectable_value(&mut st.pen_blue, blue, tr(lang, key));
        }
    });
    if before != (st.typed_text.clone(), st.typed_font.clone(), st.pen_blue) {
        st.invalidate_preview();
        st.error = None;
    }
    let mut act = None;
    if ui
        .add_enabled(
            !st.typed_text.trim().is_empty(),
            egui::Button::new(tr(lang, "signature.type.preview")),
        )
        .clicked()
    {
        act = Some(Act::Preview);
    }
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), PAD_H), Sense::hover());
    ui.painter().rect(
        rect,
        8.0,
        Color32::WHITE,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    if st.preview_pending.is_some() {
        ui.put(
            Rect::from_center_size(rect.center(), egui::vec2(24.0, 24.0)),
            egui::Spinner::new(),
        );
    } else if let Some(preview) = &st.typed_preview {
        ui.painter().image(
            preview.thumb.id(),
            fit(rect.shrink(10.0), preview.thumb.size_vec2()),
            UV,
            Color32::WHITE,
        );
    } else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            tr(lang, "signature.type.preview_hint"),
            FontId::proportional(13.0),
            Color32::from_gray(120),
        );
    }
    (act, want_fonts)
}

/// Mode Importer : choix du fichier, aperçu sur papier blanc, suppression du fond par seuil.
/// Alternative accessible au dessin : tout y est atteignable au clavier.
fn import_section(
    ui: &mut egui::Ui,
    st: &mut SignState,
    lang: Language,
    p: &theme::Palette,
) -> Option<Act> {
    let mut act = None;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, PAD_H), Sense::hover());
    ui.painter().rect(
        rect,
        8.0,
        Color32::WHITE,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    if st.import_pending.is_some() {
        ui.put(
            Rect::from_center_size(rect.center(), egui::vec2(220.0, 24.0)),
            egui::Spinner::new(),
        );
        ui.painter().text(
            rect.center() + egui::vec2(0.0, 28.0),
            Align2::CENTER_CENTER,
            tr(lang, "signature.import.loading"),
            FontId::proportional(13.0),
            Color32::from_gray(120),
        );
    } else if let Some(imp) = &mut st.import {
        let stale = imp
            .preview
            .as_ref()
            .is_none_or(|(bg, t, _)| *bg != imp.remove_bg || *t != imp.threshold);
        if stale {
            let shown = if imp.remove_bg {
                sig::remove_background(&imp.small, imp.threshold)
            } else {
                imp.small.clone()
            };
            imp.preview = Some((
                imp.remove_bg,
                imp.threshold,
                texture(ui.ctx(), "sign-import", &shown),
            ));
        }
        if let Some((_, _, tex)) = &imp.preview {
            ui.painter().image(
                tex.id(),
                fit(rect.shrink(10.0), tex.size_vec2()),
                UV,
                Color32::WHITE,
            );
        }
    } else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            tr(lang, "signature.import.empty"),
            FontId::proportional(13.5),
            Color32::from_gray(150),
        );
    }
    ui.horizontal(|ui| {
        if ui.button(tr(lang, "signature.import.pick")).clicked() {
            act = Some(Act::PickImage);
        }
        if let Some(imp) = &mut st.import {
            ui.add_space(8.0);
            ui.checkbox(&mut imp.remove_bg, tr(lang, "signature.import.remove_bg"));
            ui.add_enabled_ui(imp.remove_bg, |ui| {
                ui.label(tr(lang, "signature.import.threshold"));
                ui.scope(|ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = p.border;
                    ui.spacing_mut().slider_width = 110.0;
                    ui.add(egui::Slider::new(&mut imp.threshold, 60..=250).show_value(true));
                });
            });
        }
    });
    if st.import.is_some() {
        ui.label(
            RichText::new(tr(lang, "signature.import.threshold_hint"))
                .size(12.0)
                .color(p.text_muted),
        );
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_is_centered_and_clamped() {
        let page = (612.0, 792.0);
        let r = sig::placement_rect((300.0, 400.0), 160.0, 0.25, page).unwrap();
        assert_eq!(
            (r.left, r.bottom, r.right, r.top),
            (220.0, 380.0, 380.0, 420.0)
        );
        // Bords : gauche / bas, puis droite / haut.
        let r = sig::placement_rect((10.0, 5.0), 160.0, 0.25, page).unwrap();
        assert_eq!((r.left, r.bottom), (0.0, 0.0));
        let r = sig::placement_rect((610.0, 790.0), 160.0, 0.25, page).unwrap();
        assert_eq!((r.right, r.top), (612.0, 792.0));
        // Plus large que la page : réduite en conservant les proportions.
        let r = sig::placement_rect((300.0, 400.0), 700.0, 0.1, page).unwrap();
        assert_eq!((r.left, r.right), (0.0, 612.0));
        assert!(default_width(SignKind::Signature, 2.0) < 40.0);
        assert_eq!(default_width(SignKind::Initials, 0.5), 64.0);
    }

    #[test]
    fn saved_listing_filters_and_sorts() {
        let dir = std::env::temp_dir().join(format!("sunipdf-sign-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "signature-1700000000001.png",
            "signature-1700000000009.png",
            "initials-1700000000005.png",
            "Photo1.png",
            "signature-1700000000003.txt",
            "signature-abc.png",
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let listed = list_saved(&dir);
        let names: Vec<(String, SignKind)> = listed
            .iter()
            .map(|(p, k)| (p.file_name().unwrap().to_string_lossy().into_owned(), *k))
            .collect();
        assert_eq!(
            names,
            vec![
                (
                    "signature-1700000000009.png".to_string(),
                    SignKind::Signature
                ),
                (
                    "signature-1700000000001.png".to_string(),
                    SignKind::Signature
                ),
                ("initials-1700000000005.png".to_string(), SignKind::Initials),
            ]
        );
        assert!(list_saved(&dir.join("absent")).is_empty());
        // Le nom généré a 13 chiffres : le tri par nom reste un tri par date.
        let n = new_file_name(SignKind::Initials);
        assert!(
            n.starts_with("initials-") && n.ends_with(".png") && n.len() == 26,
            "{n}"
        );
        assert_eq!(SignKind::from_file_name(&n), Some(SignKind::Initials));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_anchor_keeps_date_in_page() {
        let page = (612.0, 792.0);
        let date = "15/10/2026";
        assert_eq!(
            text_anchor((100.0, 500.0), date, 12.0, page),
            (100.0, 500.0)
        );
        // Bord droit : reculé de la largeur estimée ; bord haut : descendu d'un corps.
        let (x, y) = text_anchor((605.0, 790.0), date, 12.0, page);
        assert!(x + 10.0 * 0.556 * 12.0 <= 612.0 && x < 605.0, "x {x}");
        assert_eq!(y, 780.0);
        assert_eq!(text_anchor((-5.0, -5.0), date, 12.0, page), (0.0, 0.0));
    }

    #[test]
    fn date_text_follows_language() {
        assert_eq!(date_text(Language::French, 2026, 10, 14), "14/10/2026");
        assert_eq!(date_text(Language::English, 2026, 10, 14), "10/14/2026");
        assert_eq!(date_text(Language::French, 2026, 1, 5), "05/01/2026");
        assert!(!today_text(Language::French).is_empty());
    }

    #[test]
    fn default_state_saves_and_draws() {
        let st = SignState::default();
        assert!(st.save && st.mode == Mode::Draw && st.placing.is_none());
        assert!(st.pen_width >= 1.5 && st.pen_width <= 6.0);
    }

    fn visible_mark() -> image::RgbaImage {
        image::RgbaImage::from_pixel(60, 20, image::Rgba([10, 10, 10, 255]))
    }

    #[test]
    fn typed_preview_rejects_old_requests_mode_changes_and_tab_cancellation() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.sign.open = Some(SignKind::Signature);
        app.sign.mode = Mode::Type;
        app.sign.typed_text = "Émilie".into();
        let (old_id, text, style) = app.sign.preview_request().unwrap();
        assert_eq!(text, "Émilie");
        assert_eq!(style.family, FontFamily::Times);
        assert!(style.italic && style.font_file.is_none());
        assert!(!app.sign.ready());

        app.sign.typed_text = "EM".into();
        app.sign.pen_blue = true;
        let (id, _, style) = app.sign.preview_request().unwrap();
        assert_ne!(id, old_id);
        assert_eq!(style.rgb, (INK_BLUE[0], INK_BLUE[1], INK_BLUE[2]));
        app.on_signature_rendered(&ctx, old_id, Err(EngineError::Other("stale".into())));
        assert_eq!(app.sign.preview_pending, Some(id));
        assert!(app.sign.error.is_none());
        app.on_signature_rendered(&ctx, id, Ok(visible_mark()));
        assert!(app.sign.ready());

        app.sign.mode = Mode::Import;
        app.sign.invalidate_preview();
        app.on_signature_rendered(&ctx, id, Ok(visible_mark()));
        assert!(app.sign.typed_preview.is_none());
        app.sign.mode = Mode::Type;
        let (id, _, _) = app.sign.preview_request().unwrap();
        app.sign.scope = Scope::Selected;
        app.sign.selected_pages = vec![0];
        app.cancel_placing();
        app.on_signature_rendered(&ctx, id, Ok(visible_mark()));
        assert!(app.sign.open.is_none() && app.sign.typed_preview.is_none());
        assert_eq!(app.sign.scope, Scope::Current);
        assert!(app.sign.selected_pages.is_empty());
    }

    #[test]
    fn typed_preview_keeps_system_font_and_refuses_blank_or_failed_output() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.sign.open = Some(SignKind::Initials);
        app.sign.mode = Mode::Type;
        assert!(app.sign.preview_request().is_none());
        app.sign.typed_text = "EM".into();
        let path = PathBuf::from("test-font.ttf");
        app.sign.typed_font = TextFont::System(sunipdf_core::SystemFont {
            name: "Test font".into(),
            path: path.clone(),
            size_bytes: 123,
        });
        let (id, _, style) = app.sign.preview_request().unwrap();
        assert_eq!(style.font_file, Some(path));
        app.on_signature_rendered(&ctx, id, Ok(image::RgbaImage::new(60, 20)));
        assert!(!app.sign.ready());
        assert_eq!(app.sign.error, Some("signature.error.empty"));
        let (id, _, _) = app.sign.preview_request().unwrap();
        app.on_signature_rendered(&ctx, id, Err(EngineError::Other("font rejected".into())));
        assert!(!app.sign.ready());
        assert!(app.sign.error.is_some() && app.sign.preview_pending.is_none());
    }

    #[test]
    fn typed_signature_uses_existing_image_flow_and_captured_selected_pages_once() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.document.as_mut().unwrap().info.page_sizes.extend([
            sunipdf_core::PageSize {
                width_pt: 300.0,
                height_pt: 500.0,
            },
            sunipdf_core::PageSize {
                width_pt: 100.0,
                height_pt: 50.0,
            },
        ]);
        app.organize.active = true;
        app.organize.selected = [0, 2].into_iter().collect();
        app.open_sign_dialog(SignKind::Initials);
        app.sign.saved.clear();
        while requests.try_recv().is_ok() {}
        assert_eq!(app.sign.scope, Scope::Current);
        assert_eq!(app.sign.selected_pages, [0, 2]);
        app.sign.scope = Scope::Selected;
        app.sign.mode = Mode::Type;
        app.sign.typed_text = "EM".into();
        app.sign.save = false;
        let (id, _, _) = app.sign.preview_request().unwrap();
        app.on_signature_rendered(&ctx, id, Ok(visible_mark()));
        app.place_new(&ctx, SignKind::Initials);
        assert!(!app.organize.active);
        assert!(
            matches!(&app.sign.placing, Some(Placing::Image { pages: Some(pages), .. }) if pages == &[0, 2])
        );
        app.place_at(1, (250.0, 480.0), false);
        assert!(
            matches!(requests.try_recv().unwrap(), Request::PlaceSignature { pages, center: (250.0, 480.0), width_pt: 64.0, .. } if pages == [0, 2])
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        assert!(requests.try_recv().is_err());
        assert!(app.sign.busy && app.sign.total == 2 && app.sign.placing.is_none());
        assert!(app.edit.reselect_many.is_none());
        app.sign.cancel.store(true, Ordering::Relaxed);
        app.sign.begin(2);
        assert!(!app.sign.cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn current_signature_shrinks_on_small_pages_and_shift_keeps_placement() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        let image = Arc::new(DynamicImage::ImageRgba8(visible_mark()));
        app.document.as_mut().unwrap().info.page_sizes[0] = sunipdf_core::PageSize {
            width_pt: 30.0,
            height_pt: 20.0,
        };
        app.sign.placing = Some(Placing::Image {
            image: image.clone(),
            thumb: texture(&ctx, "test-placement", &thumb_of(&image)),
            width_pt: 160.0,
            aspect: 1.0 / 3.0,
            pages: None,
        });
        app.place_at(0, (29.0, 19.0), true);
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::Edit(EditOp::AddImageData {
                page: 0,
                x: 0.0,
                y: 10.0,
                width_pt: 30.0,
                ..
            })
        ));
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        assert!(app.sign.placing.is_some() && app.edit.reselect_many.is_none());
        assert_eq!(app.sign.target_pages(3), None);
        app.sign.scope = Scope::All;
        assert_eq!(app.sign.target_pages(3), Some(vec![0, 1, 2]));
    }

    #[test]
    fn keyboard_edit_invalidates_preview_and_escape_cancels_batch_progress() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        app.sign.open = Some(SignKind::Signature);
        app.sign.mode = Mode::Type;
        app.sign.typed_text = "EM".into();
        let (id, _, _) = app.sign.preview_request().unwrap();
        app.on_signature_rendered(&ctx, id, Ok(visible_mark()));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.memory_mut(|m| m.request_focus(ui.make_persistent_id("signature-name")));
            typed_section(
                ui,
                &mut app.sign,
                Language::English,
                &theme::palette(false),
                Some(&[]),
            );
        });
        out.textures_delta.clear();
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Text("x".into())],
                ..Default::default()
            },
            |ui| {
                typed_section(
                    ui,
                    &mut app.sign,
                    Language::English,
                    &theme::palette(false),
                    Some(&[]),
                );
            },
        );
        out.textures_delta.clear();
        assert!(app.sign.typed_text.contains('x'));
        assert!(!app.sign.ready() && app.sign.typed_preview.is_none());

        app.sign.begin(3);
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| {
                app.sign_dialog(&ctx);
            },
        );
        out.textures_delta.clear();
        assert!(app.sign.busy && app.sign.cancel.load(Ordering::Relaxed));
    }
}
