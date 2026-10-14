//! Signature visuelle, moitié « signer » de l'outil Remplir et signer : boîte de dialogue (pavé
//! de dessin, image importée avec fond retiré, bibliothèque locale), paraphe, date du jour, puis
//! placement sur la page d'un objet image (ou texte) ordinaire, sélectionné aussitôt.
//!
//! **Aucune valeur cryptographique** : rien ici ne prouve l'identité du signataire ni l'intégrité
//! du document (le libellé de la commande, le sous-titre et l'encadré de la boîte le disent). La
//! signature numérique par certificat est une autre fonction, non livrée.

use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::dialog;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{EditOp, Pick, Request};
use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sunipdf_core::signature_image as sig;
use sunipdf_core::{EngineError, PointRect, TextStyle};
use sunipdf_i18n::{tr, tr_args, Language};

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

impl SignKind {
    fn prefix(self) -> &'static str {
        match self {
            SignKind::Signature => "signature",
            SignKind::Initials => "initials",
        }
    }

    /// `signature-<ms>.png` / `initials-<ms>.png` ; tout autre fichier est ignoré.
    fn from_file_name(name: &str) -> Option<SignKind> {
        let (prefix, rest) = name.split_once('-')?;
        let digits = rest.strip_suffix(".png")?;
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
    draw_mode: bool,
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
}

impl Default for SignState {
    fn default() -> Self {
        Self {
            open: None,
            saved: Vec::new(),
            draw_mode: true,
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
        }
    }
}

enum Act {
    Place,
    Use(usize),
    Delete(PathBuf),
    PickImage,
    Cancel,
}

/// Dossier des signatures enregistrées : `SUNIPDF_SIGNATURES_DIR` (tests, captures), sinon
/// `%APPDATA%\SuniPDF\signatures`. PNG en clair : aucun chiffrement, jamais envoyé sur Internet.
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

/// Fichiers `signature-*.png` et `initials-*.png`, plus récents d'abord, 12 au plus par type.
fn list_saved(dir: &Path) -> Vec<(PathBuf, SignKind)> {
    // ponytail: plafond fixe de 12 par type, images gardées décodées en mémoire.
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, SignKind)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let kind = SignKind::from_file_name(path.file_name()?.to_str()?)?;
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

/// Rectangle (points PDF) centré sur le clic, maintenu dans la page.
pub fn placement_rect(
    center: (f32, f32),
    width_pt: f32,
    aspect: f32,
    page: (f32, f32),
) -> PointRect {
    let w = width_pt.max(1.0);
    let h = w * aspect;
    let left = (center.0 - w / 2.0).clamp(0.0, (page.0 - w).max(0.0));
    let bottom = (center.1 - h / 2.0).clamp(0.0, (page.1 - h).max(0.0));
    PointRect {
        left,
        bottom,
        right: left + w,
        top: bottom + h,
    }
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

impl SuniApp {
    pub fn open_sign_dialog(&mut self, ctx: &egui::Context, kind: SignKind) {
        self.cancel_placing();
        let st = &mut self.sign;
        st.open = Some(kind);
        st.error = None;
        st.confirm_delete = None;
        st.drawing = false;
        let Some(dir) = signatures_dir() else { return };
        let listed = list_saved(&dir);
        st.saved
            .retain(|s| listed.iter().any(|(p, _)| *p == s.path));
        for (path, kind) in listed {
            if st.saved.iter().any(|s| s.path == path) {
                continue;
            }
            // Fichiers écrits par SuniPDF, bornés en taille : décodage sur place, à l'ouverture.
            match sig::load(&path, MAX_SIDE) {
                Ok(img) => {
                    let image = DynamicImage::ImageRgba8(img);
                    let thumb = texture(ctx, "sign-saved", &thumb_of(&image));
                    st.saved.push(SavedSignature {
                        path,
                        kind,
                        image: Arc::new(image),
                        thumb,
                    });
                }
                Err(e) => log::warn!(
                    "signature enregistrée illisible ({}) : {e}",
                    crate::app::file_name(&path)
                ),
            }
        }
        st.saved.sort_by(|a, b| b.path.cmp(&a.path));
    }

    pub fn sign_dialog(&mut self, ctx: &egui::Context) {
        let Some(kind_open) = self.sign.open else {
            return;
        };
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let p = theme::palette(dark);
        let dir_text = signatures_dir().map_or_else(String::new, |d| d.display().to_string());
        let mut kind = kind_open;
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
        let ready = if st.draw_mode {
            !st.strokes.is_empty()
        } else {
            st.import.is_some()
        };
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
                                        "a11y.signature_saved",
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
                            let painter = ui.painter();
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

                // Nouvelle signature : dessiner ou importer.
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
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut st.draw_mode, true, tr(lang, "signature.mode.draw"));
                    ui.selectable_value(
                        &mut st.draw_mode,
                        false,
                        tr(lang, "signature.mode.import"),
                    );
                });
                if st.draw_mode {
                    draw_pad(ui, st, lang, &p);
                } else if let Some(a) = import_section(ui, st, lang, &p) {
                    body_act = Some(a);
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
        if let Some(c) = new_confirm {
            self.sign.confirm_delete = c;
        }
        match body_act.or(foot_act) {
            Some(Act::Cancel) => {
                self.sign.open = None;
                self.sign.drawing = false;
            }
            Some(Act::Delete(path)) => {
                if let Err(e) = std::fs::remove_file(&path) {
                    log::warn!(
                        "suppression impossible ({}) : {e}",
                        crate::app::file_name(&path)
                    );
                }
                self.sign.saved.retain(|s| s.path != path);
                self.sign.confirm_delete = None;
            }
            Some(Act::PickImage) => {
                let dialog = rfd::FileDialog::new()
                    .set_title(tr(lang, "signature.import.pick_title"))
                    .add_filter(
                        tr(lang, "tools.images.filter"),
                        crate::editor::IMAGE_EXTENSIONS,
                    );
                if let Some(path) = dialog.pick_file() {
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
                };
                self.sign.open = None;
                self.start_placing(placing, "signature.placing_hint");
            }
            Some(Act::Place) => self.place_new(ctx, kind),
            None => {}
        }
    }

    /// Trame ou prépare l'image, l'enregistre si demandé, puis passe en mode de placement.
    fn place_new(&mut self, ctx: &egui::Context, kind: SignKind) {
        let st = &mut self.sign;
        let img = if st.draw_mode {
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
                    let path = dir.join(new_file_name(kind));
                    match sig::save_png(&img, &path) {
                        Ok(()) => saved_path = Some(path),
                        // Jamais de contenu dans le journal : seulement le nom du fichier.
                        Err(e) => {
                            log::warn!(
                                "signature non enregistrée ({}) : {e}",
                                crate::app::file_name(&path)
                            );
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
        let placing = Placing::Image {
            image,
            thumb,
            width_pt: default_width(kind, aspect),
            aspect,
        };
        self.start_placing(placing, "signature.placing_hint");
        if save_failed {
            // La pose continue ; l'échec est annoncé dans le bandeau.
            self.error = Some("signature.error.save");
        }
    }

    pub fn on_image_loaded(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
        result: Result<image::RgbaImage, EngineError>,
    ) {
        let st = &mut self.sign;
        if st.import_pending.as_ref() != Some(&path) {
            return;
        }
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
                log::warn!("image de signature illisible : {e}");
                st.error = Some("signature.error.image");
            }
        }
        ctx.request_repaint();
    }

    fn start_placing(&mut self, p: Placing, hint_key: &str) {
        self.sign.placing = Some(p);
        self.organize.active = false;
        self.edit.selection = None;
        self.annotate.drag = None;
        self.status_msg = Some(tr(self.lang(), hint_key).to_string());
    }

    pub fn cancel_placing(&mut self) {
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
        // Le dernier objet de la page est resélectionné à la réponse du moteur.
        self.edit.reselect_many = Some((page, Pick::Last(1)));
        // `DocumentChanged` remplace le message d'état par celui-ci (conservé avec Maj).
        self.edit.pending_status = hint;
        match placing {
            Placing::Image {
                image,
                width_pt,
                aspect,
                ..
            } => {
                let r = placement_rect(pt, width_pt, aspect, (size.width_pt, size.height_pt));
                self.worker.send(Request::Edit(EditOp::AddImageData {
                    page,
                    image,
                    x: r.left,
                    y: r.bottom,
                    width_pt,
                }));
            }
            Placing::Text(text) => {
                self.worker.send(Request::Edit(EditOp::AddText {
                    page,
                    x: pt.0,
                    y: pt.1,
                    text,
                    style: TextStyle::default(),
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
                let r = placement_rect(pt, *width_pt, *aspect, (page_rect.width() / k, page_h_pt));
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
                let size = TextStyle::default().size_pt * k;
                painter.text(
                    to_screen(pt.0, pt.1),
                    Align2::LEFT_BOTTOM,
                    text,
                    FontId::proportional(size),
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
    if resp.is_pointer_button_down_on() {
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
            ui.painter()
                .circle_filled(r.center(), 8.0, Color32::from_rgb(c[0], c[1], c[2]));
            if selected || resp.has_focus() {
                ui.painter()
                    .circle_stroke(r.center(), 11.0, Stroke::new(2.0, p.accent));
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
        let r = placement_rect((300.0, 400.0), 160.0, 0.25, page);
        assert_eq!(
            (r.left, r.bottom, r.right, r.top),
            (220.0, 380.0, 380.0, 420.0)
        );
        // Bords : gauche / bas, puis droite / haut.
        let r = placement_rect((10.0, 5.0), 160.0, 0.25, page);
        assert_eq!((r.left, r.bottom), (0.0, 0.0));
        let r = placement_rect((610.0, 790.0), 160.0, 0.25, page);
        assert_eq!((r.right, r.top), (612.0, 792.0));
        // Plus large que la page : collée à gauche, largeur conservée.
        let r = placement_rect((300.0, 400.0), 700.0, 0.1, page);
        assert_eq!((r.left, r.right), (0.0, 700.0));
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
    fn date_text_follows_language() {
        assert_eq!(date_text(Language::French, 2026, 10, 14), "14/10/2026");
        assert_eq!(date_text(Language::English, 2026, 10, 14), "10/14/2026");
        assert_eq!(date_text(Language::French, 2026, 1, 5), "05/01/2026");
        assert!(!today_text(Language::French).is_empty());
    }

    #[test]
    fn default_state_saves_and_draws() {
        let st = SignState::default();
        assert!(st.save && st.draw_mode && st.placing.is_none());
        assert!(st.pen_width >= 1.5 && st.pen_width <= 6.0);
    }
}
