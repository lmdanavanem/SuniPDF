//! Caviardage irréversible : ce qui se trouve sous une zone est **supprimé** du document, pas masqué.
//!
//! Méthode (une seule voie de code, deux modes) :
//! - **A. PDFium** : tout objet de page qui croise une zone est retiré en entier. Avant le retrait, la
//!   page est rendue en image et les zones y sont peintes en noir ; une vignette découpée dans ce rendu
//!   (zone ∪ empreinte des objets retirés) est ajoutée au premier plan. Les pixels de la zone sont noirs
//!   dans les données mêmes de l'image. Mode « page en image » : tous les objets, une vignette = la page.
//! - **B. lopdf** (`sanitize`) : annotations et champs sous les zones supprimés, dictionnaire de page
//!   réduit à une liste blanche, ressources filtrées aux noms encore dessinés, ressources que plus
//!   aucune page ne dessine détachées, informations cachées (option), ramasse-miettes, réécriture
//!   complète (aucune révision antérieure ne survit).
//! - **C. Vérification** : le résultat est rechargé ; s'il reste du texte extractible sous une zone,
//!   `EngineError::RedactionIncomplete` (l'appelant restaure son instantané).
//!
//! Les zones sont en espace d'affichage (comme toute l'API, voir `geometry.rs`). Aucun texte recherché
//! ni contenu n'est journalisé.

use crate::edit::{display_quad, quad_bounds};
use crate::{Document, EngineError, PointRect, RenderedPage, TextStyle, MAX_RENDER_SIDE_PX};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Object, ObjectId};
use pdfium_render::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Marge ajoutée autour d'une occurrence de texte (points).
const TEXT_PAD_PT: f32 = 1.0;
/// ponytail: plafond fixe (~120 Mo RVB) du rendu d'une page caviardée ; à rendre réglable si des plans
/// A0 à haute résolution l'exigent.
const RASTER_MAX_PIXELS: f32 = 40_000_000.0;
const JPEG_QUALITY: u8 = 90;
/// Les limites d'un objet texte partent de la ligne de base : on les agrandit (fraction de la hauteur)
/// pour attraper jambages et accents. Retirer trop est sans danger (la vignette restitue l'aspect).
const TEXT_BOUNDS_PAD: f32 = 0.3;
/// Zone « partout » : toutes les annotations d'une page convertie en image.
const EVERYWHERE: PointRect = PointRect {
    left: f32::MIN,
    bottom: f32::MIN,
    right: f32::MAX,
    top: f32::MAX,
};

/// Zone à caviarder : page et rectangle en espace d'affichage (points), comme `TextMatch::rects`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactMark {
    pub page: usize,
    pub rect: PointRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactOptions {
    /// Remplacer tout le contenu de chaque page marquée par une image (aucun texte ne subsiste).
    pub rasterize_pages: bool,
    /// Supprimer aussi /Info, XMP, /PieceInfo et l'arbre de structure (balisage d'accessibilité).
    pub clean_hidden_data: bool,
    /// Résolution des vignettes et des pages converties (bornée à 72..=600).
    pub raster_dpi: f32,
}

impl Default for RedactOptions {
    fn default() -> Self {
        Self {
            rasterize_pages: false,
            clean_hidden_data: true,
            raster_dpi: 200.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RedactReport {
    pub marks: usize,
    pub pages: usize,
    pub objects_removed: usize,
    pub annotations_removed: usize,
    pub fields_removed: usize,
    pub pages_rasterized: usize,
    pub hidden_data_cleaned: bool,
    /// Safely reconstructed selectable text runs outside the marked rectangles.
    pub native_text_runs: usize,
    /// Removed objects whose visible remainder is retained only in the raster patch.
    pub bitmap_objects: usize,
}

#[derive(Clone, Default)]
pub struct RedactAppearance {
    pub fill_rgb: (u8, u8, u8),
    pub overlay: Option<RedactOverlay>,
}

#[derive(Clone)]
pub struct RedactOverlay {
    pub text: String,
    pub style: TextStyle,
}

/// Opaque, fully verified candidate, bound to its protected source document.
pub struct PreparedRedaction {
    original: Vec<u8>,
    candidate: Vec<u8>,
    report: RedactReport,
    cancel: Arc<AtomicBool>,
}

fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

pub struct RedactionPreview {
    pub before: RenderedPage,
    pub after: RenderedPage,
    pub page: usize,
    pub scale: f32,
    pub report: RedactReport,
    pub prepared: Option<PreparedRedaction>,
}

fn intersects(a: &PointRect, b: &PointRect) -> bool {
    a.left < b.right && a.right > b.left && a.bottom < b.top && a.top > b.bottom
}

fn union(a: PointRect, b: &PointRect) -> PointRect {
    PointRect {
        left: a.left.min(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.max(b.right),
        top: a.top.max(b.top),
    }
}

fn intersection(a: &PointRect, b: &PointRect) -> PointRect {
    PointRect {
        left: a.left.max(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.min(b.right),
        top: a.top.min(b.top),
    }
}

fn area(r: &PointRect) -> f32 {
    (r.right - r.left).max(0.0) * (r.top - r.bottom).max(0.0)
}

fn normalized(r: PointRect) -> PointRect {
    PointRect {
        left: r.left.min(r.right),
        bottom: r.bottom.min(r.top),
        right: r.left.max(r.right),
        top: r.bottom.max(r.top),
    }
}

/// Rectangle de pixels `(x0, y0, x1, y1)` (origine en haut à gauche) qui contient `r` (points
/// d'affichage) : arrondi vers l'extérieur, borné à l'image.
fn pixel_rect(r: &PointRect, page_h: f32, sx: f32, sy: f32, pw: u32, ph: u32) -> [u32; 4] {
    let clamp = |v: f32, max: u32| v.clamp(0.0, max as f32) as u32;
    [
        clamp((r.left * sx).floor(), pw),
        clamp(((page_h - r.top) * sy).floor(), ph),
        clamp((r.right * sx).ceil(), pw),
        clamp(((page_h - r.bottom) * sy).ceil(), ph),
    ]
}

/// Valide les zones (avant toute modification) et les groupe par page.
fn group_marks(
    marks: &[RedactMark],
    page_count: usize,
) -> Result<BTreeMap<usize, Vec<PointRect>>, EngineError> {
    if marks.is_empty() || marks.len() > 4096 {
        return Err(EngineError::Other("nothing to redact".into()));
    }
    let mut by_page: BTreeMap<usize, Vec<PointRect>> = BTreeMap::new();
    for m in marks {
        if m.page >= page_count {
            return Err(EngineError::PageOutOfRange(m.page));
        }
        let r = normalized(m.rect);
        let finite = [r.left, r.bottom, r.right, r.top]
            .iter()
            .all(|v| v.is_finite());
        if !finite || r.right - r.left < 0.1 || r.top - r.bottom < 0.1 {
            return Err(EngineError::Other("invalid redaction rectangle".into()));
        }
        by_page.entry(m.page).or_default().push(r);
    }
    Ok(by_page)
}

impl<'a> Document<'a> {
    fn redaction_scratch(&self, bytes: Vec<u8>) -> Result<Document<'a>, EngineError> {
        Ok(Document {
            inner: self.pdfium.load_pdf_from_byte_vec(bytes, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
        })
    }

    pub fn preview_redaction(
        &self,
        marks: &[RedactMark],
        options: &RedactOptions,
        appearance: &RedactAppearance,
        page: usize,
    ) -> Result<RedactionPreview, EngineError> {
        self.preview_redaction_cancelable(
            marks,
            options,
            appearance,
            page,
            Arc::new(AtomicBool::new(false)),
        )
    }

    pub fn preview_redaction_cancelable(
        &self,
        marks: &[RedactMark],
        options: &RedactOptions,
        appearance: &RedactAppearance,
        page: usize,
        cancel: Arc<AtomicBool>,
    ) -> Result<RedactionPreview, EngineError> {
        cancelled(&cancel)?;
        let original = self.snapshot()?;
        if original.len() > 128 * 1024 * 1024 {
            return Err(EngineError::Other(
                "redaction preview is limited to 128 MiB documents".into(),
            ));
        }
        if !marks.iter().any(|mark| mark.page == page) {
            return Err(EngineError::Other(
                "preview page has no redaction marks".into(),
            ));
        }
        let size = self.page_size(page)?;
        let scale = (900.0 / size.width_pt.max(size.height_pt)).min(1.5);
        let mut scratch = self.redaction_scratch(original.clone())?;
        // Form rendering may materialize widget appearance resources. Keep even those native
        // cache updates on the candidate, so preparation cannot change its protected source.
        let before = scratch.render_page(page, scale)?;
        cancelled(&cancel)?;
        let report = scratch.redact_in_place(marks, options, appearance, &cancel)?;
        let after = scratch.render_page(page, scale)?;
        let candidate = scratch.snapshot()?;
        cancelled(&cancel)?;
        if candidate.len() > 160 * 1024 * 1024 {
            return Err(EngineError::Other(
                "redaction candidate exceeds 160 MiB".into(),
            ));
        }
        Ok(RedactionPreview {
            before,
            after,
            page,
            scale,
            report,
            prepared: Some(PreparedRedaction {
                original,
                candidate,
                report,
                cancel,
            }),
        })
    }

    pub fn commit_redaction_preview(
        &mut self,
        prepared: PreparedRedaction,
    ) -> Result<RedactReport, EngineError> {
        cancelled(&prepared.cancel)?;
        if !crate::annot_rich::same_document(
            &prepared.original,
            &self.snapshot()?,
            self.password(),
        )? {
            return Err(EngineError::Other(
                "document changed since redaction preview; preview again".into(),
            ));
        }
        cancelled(&prepared.cancel)?;
        self.restore(prepared.candidate)?;
        Ok(prepared.report)
    }

    /// Store standard, visibly unapplied /Redact annotations. No content is removed.
    pub fn persist_redaction_marks(
        &mut self,
        marks: &[RedactMark],
        appearance: &RedactAppearance,
        author: Option<&str>,
    ) -> Result<(), EngineError> {
        let groups = group_marks(marks, self.page_count())?;
        if author.is_some_and(|a| a.len() > 4096 || a.chars().any(char::is_control)) {
            return Err(EngineError::Other("invalid redaction author".into()));
        }
        if appearance.overlay.is_some() {
            return Err(EngineError::Other("persisted marks store rectangles and fill only; clear the overlay or preview and apply it".into()));
        }
        let existing = self.redaction_marks()?;
        let mut scratch = self.redaction_scratch(self.snapshot()?)?;
        for (page, rects) in groups {
            scratch.edit_pdf(page, |pdf, page_id, geo| {
                for rect in rects {
                    if existing.contains(&RedactMark { page, rect }) {
                        continue;
                    }
                    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
                    let (pw, ph) = geo.display_size();
                    if rect.left < 0.0 || rect.bottom < 0.0 || rect.right > pw || rect.top > ph {
                        return Err(EngineError::Other(
                            "redaction mark is outside the page".into(),
                        ));
                    }
                    let mut annotation = crate::annot_write::base_annotation("Redact", author)?;
                    annotation.set("Rect", crate::annot_write::rect_obj(geo.rect_to_user(rect)));
                    annotation.set("IC", crate::annot_write::rgb_obj(appearance.fill_rgb));
                    let style = crate::AnnotStyle {
                        rgb: (220, 35, 35),
                        fill_rgb: Some(appearance.fill_rgb),
                        opacity: 0.35,
                        width_pt: 1.0,
                        ..Default::default()
                    };
                    let ap = crate::appearance::square_ap(w, h, &style);
                    crate::annot_write::push_annotation(
                        pdf,
                        page_id,
                        annotation,
                        Some(crate::annot_write::ap_stream(&ap, geo)),
                    )?;
                }
                Ok(())
            })?;
        }
        self.restore(scratch.snapshot()?)
    }

    /// Recover standard /Redact rectangles in display coordinates; they are not applied.
    pub fn redaction_marks(&self) -> Result<Vec<RedactMark>, EngineError> {
        let pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let mut marks = Vec::new();
        for (number, page) in pdf.get_pages() {
            let index = number as usize - 1;
            let geo = self.page_geometry(index)?;
            for entry in page_annotations(&pdf, page).unwrap_or_default() {
                if entry.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Redact") {
                    continue;
                }
                if marks.len() == 4096 {
                    return Err(EngineError::Other(
                        "too many persisted redaction marks".into(),
                    ));
                }
                if let Some(rect) = rect_of(&pdf, &entry.dict) {
                    marks.push(RedactMark {
                        page: index,
                        rect: geo.rect_to_display(rect),
                    });
                }
            }
        }
        Ok(marks)
    }
    /// Une zone par segment de ligne de chaque occurrence de `query`, agrandie de 1 pt (affichage).
    pub fn redact_marks_for_text(
        &self,
        query: &str,
        match_case: bool,
    ) -> Result<Vec<RedactMark>, EngineError> {
        Ok(self
            .search(query, match_case)?
            .into_iter()
            .flat_map(|m| {
                m.rects.into_iter().map(move |r| RedactMark {
                    page: m.page,
                    rect: PointRect {
                        left: r.left - TEXT_PAD_PT,
                        bottom: r.bottom - TEXT_PAD_PT,
                        right: r.right + TEXT_PAD_PT,
                        top: r.top + TEXT_PAD_PT,
                    },
                })
            })
            .collect())
    }

    /// Apply on a scratch candidate, verify the full rewrite, then replace this document atomically.
    /// Errors leave the original document unchanged; no files are written.
    pub fn redact(
        &mut self,
        marks: &[RedactMark],
        options: &RedactOptions,
    ) -> Result<RedactReport, EngineError> {
        let mut scratch = self.redaction_scratch(self.snapshot()?)?;
        let report = scratch.redact_in_place(
            marks,
            options,
            &RedactAppearance::default(),
            &AtomicBool::new(false),
        )?;
        self.restore(scratch.snapshot()?)?;
        Ok(report)
    }

    fn redact_in_place(
        &mut self,
        marks: &[RedactMark],
        options: &RedactOptions,
        appearance: &RedactAppearance,
        cancel: &AtomicBool,
    ) -> Result<RedactReport, EngineError> {
        let by_page = group_marks(marks, self.page_count())?;
        let dpi = if options.raster_dpi.is_finite() {
            options.raster_dpi.clamp(72.0, 600.0)
        } else {
            200.0
        };
        let rasterize = options.rasterize_pages;
        // Zones en espace utilisateur pour les /Rect d'annotations (phase B).
        let mut user_zones = BTreeMap::new();
        for (&page, rects) in &by_page {
            cancelled(cancel)?;
            let geo = self.page_geometry(page)?;
            let zones = if rasterize {
                vec![EVERYWHERE]
            } else {
                rects.iter().map(|r| geo.rect_to_user(*r)).collect()
            };
            user_zones.insert(page, zones);
        }
        let mut report = RedactReport {
            marks: marks.len(),
            pages: by_page.len(),
            pages_rasterized: if rasterize { by_page.len() } else { 0 },
            hidden_data_cleaned: options.clean_hidden_data,
            ..Default::default()
        };
        for (&page, rects) in &by_page {
            cancelled(cancel)?;
            let (removed, native, bitmap) =
                self.redact_page_objects(page, rects, rasterize, dpi, appearance.fill_rgb, cancel)?;
            report.objects_removed += removed;
            report.native_text_runs += native;
            report.bitmap_objects += bitmap;
        }
        let mut clean = sanitize(
            &self.snapshot()?,
            self.password(),
            &user_zones,
            options.clean_hidden_data,
        )?;
        cancelled(cancel)?;
        report.annotations_removed = clean.annotations_removed;
        report.fields_removed = clean.fields_removed;
        if options.clean_hidden_data {
            let (bytes, hidden) =
                crate::sanitize::sanitize_document(&clean.bytes, self.password())?;
            clean.bytes = bytes;
            report.annotations_removed += hidden.removed_annotations;
        }
        cancelled(cancel)?;
        // Vérification sur l'état exact qui sera enregistré (rechargé depuis les octets assainis).
        self.restore(clean.bytes)?;
        let residual = by_page.iter().any(|(&page, rects)| {
            if rasterize {
                self.page_text(page).map_or(true, |t| !t.trim().is_empty())
            } else {
                rects.iter().any(|r| {
                    self.text_in_rect(page, *r)
                        .map_or(true, |s| !s.text.is_empty())
                })
            }
        });
        if residual {
            return Err(EngineError::RedactionIncomplete);
        }
        // Intentional replacement text is added only after checking the cleaned, fully rewritten PDF.
        if let Some(overlay) = &appearance.overlay {
            if overlay.text.is_empty()
                || overlay.text.len() > 256
                || !overlay.text.bytes().all(|b| (32..=126).contains(&b))
                || !overlay.style.size_pt.is_finite()
                || !(4.0..=200.0).contains(&overlay.style.size_pt)
            {
                return Err(EngineError::Other("redaction overlay requires up to 256 printable Latin characters and a 4..200 pt font".into()));
            }
            for (&page, rects) in &by_page {
                for rect in rects {
                    cancelled(cancel)?;
                    let index = self.page_objects(page)?.len();
                    self.add_text(
                        page,
                        rect.left + 2.0,
                        rect.bottom + 2.0 + overlay.style.size_pt * 0.25,
                        &overlay.text,
                        &overlay.style,
                    )?;
                    let bounds = self.object_info(page, index)?.bounds;
                    if bounds.left < rect.left
                        || bounds.bottom < rect.bottom
                        || bounds.right > rect.right
                        || bounds.top > rect.top
                    {
                        return Err(EngineError::Other(
                            "redaction overlay does not fit the marked rectangle".into(),
                        ));
                    }
                }
            }
        }
        log::info!(
            "caviardage : {} zone(s) sur {} page(s), {} objet(s) et {} annotation(s) retirés",
            report.marks,
            report.pages,
            report.objects_removed,
            report.annotations_removed
        );
        Ok(report)
    }

    /// Phase A pour une page : retire les objets touchés et pose les vignettes. Renvoie le nombre
    /// d'objets retirés.
    fn redact_page_objects(
        &self,
        page_index: usize,
        rects: &[PointRect],
        rasterize: bool,
        dpi: f32,
        fill_rgb: (u8, u8, u8),
        cancel: &AtomicBool,
    ) -> Result<(usize, usize, usize), EngineError> {
        let geo = self.page_geometry(page_index)?;
        let (w, h) = geo.display_size();
        if !(w > 0.0 && h > 0.0) {
            return Err(EngineError::RenderTooLarge);
        }
        let page_box = PointRect {
            left: 0.0,
            bottom: 0.0,
            right: w,
            top: h,
        };
        let mut page = self.page_handle(page_index)?;
        if page.objects().len() > 100_000 {
            return Err(EngineError::Other(
                "redaction page object limit exceeded".into(),
            ));
        }
        let native_allowed = !rasterize
            && native_redaction_page(&self.snapshot()?, self.password(), page_index)
                .unwrap_or(false);
        let mut retained = Vec::new();
        let mut safe_victims = 0;

        // 1. Victimes : (index, limites d'affichage, est une image). Limites illisibles : la page.
        let mut victims: Vec<(usize, PointRect, bool)> = Vec::new();
        for (i, object) in page.objects().iter().enumerate() {
            cancelled(cancel)?;
            let kind = object.object_type();
            let mut b = if object.bounds().is_ok() {
                quad_bounds(&display_quad(&object, &geo))
            } else {
                page_box
            };
            if kind == PdfPageObjectType::Text {
                let pad = TEXT_BOUNDS_PAD * (b.right - b.left).min(b.top - b.bottom).max(1.0);
                b = PointRect {
                    left: b.left - pad,
                    bottom: b.bottom - pad,
                    right: b.right + pad,
                    top: b.top + pad,
                };
            }
            if rasterize || rects.iter().any(|r| intersects(r, &b)) {
                victims.push((i, b, kind == PdfPageObjectType::Image));
                if native_allowed && kind == PdfPageObjectType::Text {
                    if let Some(runs) = retained_text_runs(&page, &object, &geo, rects)? {
                        if retained.len() + runs.len() <= 16_384 {
                            safe_victims += 1;
                            retained.extend(runs);
                        }
                    }
                }
            }
        }

        // 2. Empreintes (zone ∪ limites de ses victimes, bornée à la page) et choix JPEG : une
        //    image retirée couvre au moins la moitié de l'empreinte (photo, page numérisée).
        let photo = |fp: &PointRect, r: Option<&PointRect>| {
            victims.iter().any(|v| {
                v.2 && r.is_none_or(|r| intersects(r, &v.1))
                    && area(&intersection(&v.1, fp)) >= 0.5 * area(fp)
            })
        };
        let footprints: Vec<(PointRect, bool)> = if rasterize {
            vec![(page_box, photo(&page_box, None))]
        } else {
            rects
                .iter()
                .map(|r| {
                    let fp = victims
                        .iter()
                        .filter(|v| intersects(r, &v.1))
                        .fold(*r, |acc, v| union(acc, &v.1));
                    let fp = intersection(&fp, &page_box);
                    (fp, photo(&fp, Some(r)))
                })
                .filter(|(fp, _)| area(fp) > 0.0)
                .collect()
        };

        // 3. Rendu AVANT retrait (sans annotations ni champs en mode précis : elles restent des
        //    annotations ; en mode image elles sont toutes supprimées, donc incrustées).
        let scale = (dpi / 72.0)
            .min((RASTER_MAX_PIXELS / (w * h)).sqrt())
            .min(MAX_RENDER_SIDE_PX as f32 / w.max(h));
        let config = PdfRenderConfig::new()
            .set_target_size(
                ((w * scale).round() as Pixels).max(1),
                ((h * scale).round() as Pixels).max(1),
            )
            .render_form_data(rasterize)
            .render_annotations(rasterize);
        let mut img = page.render_with_config(&config)?.as_image()?.into_rgb8();
        cancelled(cancel)?;
        let (pw, ph) = (img.width(), img.height());
        let (sx, sy) = (pw as f32 / w, ph as f32 / h);

        // 4. Noir dans les pixels, arrondi vers l'extérieur.
        for r in rects {
            let [x0, y0, x1, y1] = pixel_rect(r, h, sx, sy, pw, ph);
            for y in y0..y1 {
                cancelled(cancel)?;
                for x in x0..x1 {
                    img.put_pixel(x, y, image::Rgb([fill_rgb.0, fill_rgb.1, fill_rgb.2]));
                }
            }
        }

        // 5. Retrait groupé, une seule régénération du flux de contenu.
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let native_count = retained.len();
        let result = (|| -> Result<(), EngineError> {
            for (i, ..) in victims.iter().rev() {
                cancelled(cancel)?;
                page.objects_mut()
                    .remove_object_at_index(*i as PdfPageObjectIndex)?;
            }
            // 6. Une vignette par empreinte, au premier plan, droite à l'écran.
            for (fp, jpeg) in &footprints {
                cancelled(cancel)?;
                let [x0, y0, x1, y1] = pixel_rect(fp, h, sx, sy, pw, ph);
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                let sub = image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image();
                let (left, right) = (x0 as f32 / sx, x1 as f32 / sx);
                let (top, bottom) = (h - y0 as f32 / sy, h - y1 as f32 / sy);
                let (ow, oh) = (right - left, top - bottom);
                let mut object = if *jpeg {
                    let mut buf = Vec::new();
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY)
                        .encode_image(&sub)
                        .map_err(|e| EngineError::Other(format!("jpeg: {e}")))?;
                    let mut o = PdfPageImageObject::new_from_jpeg_reader(
                        &self.inner,
                        std::io::Cursor::new(buf),
                    )?;
                    o.scale(ow, oh)?;
                    o
                } else {
                    PdfPageImageObject::new_with_size(
                        &self.inner,
                        &image::DynamicImage::ImageRgb8(sub),
                        PdfPoints::new(ow),
                        PdfPoints::new(oh),
                    )?
                };
                let m = geo.upright_matrix(left, bottom);
                object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
                page.objects_mut().add_image_object(object)?;
            }
            for run in retained.drain(..) {
                cancelled(cancel)?;
                let mut object = PdfPageTextObject::new(&self.inner, run.text, run.font, run.size)?;
                object.set_fill_color(run.color)?;
                let m = run.matrix;
                object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
                page.objects_mut().add_text_object(object)?;
            }
            crate::edit::regenerate_content(&mut page)?;
            Ok(())
        })();
        page.set_content_regeneration_strategy(
            PdfPageContentRegenerationStrategy::AutomaticOnEveryChange,
        );
        result?;
        Ok((victims.len(), native_count, victims.len() - safe_victims))
    }
}

// Native reconstruction is deliberately limited to uncomplicated Base14 glyphs. All other
// objects keep the established pixel-patch fallback; no guessed font/encoding is introduced.
fn native_redaction_page(
    bytes: &[u8],
    password: Option<&str>,
    page: usize,
) -> Result<bool, EngineError> {
    let pdf = crate::security::load(bytes, password)?;
    let catalog = pdf.catalog().map_err(lopdf_err)?;
    if catalog.has(b"OCProperties") || catalog.has(b"StructTreeRoot") {
        return Ok(false);
    }
    let page = *pdf
        .get_pages()
        .get(&(page as u32 + 1))
        .ok_or(EngineError::PageOutOfRange(page))?;
    let content = pdf
        .get_page_content_with_limit(page, 32 * 1024 * 1024)
        .map_err(lopdf_err)?;
    let ops = Content::decode_strict(&content)
        .map_err(lopdf_err)?
        .operations;
    if ops.len() > 100_000 {
        return Ok(false);
    }
    for op in ops {
        match op.operator.as_str() {
            "BMC" | "BDC" | "'" | "\"" | "k" | "K" | "cs" | "CS" | "sc" | "SC" | "scn" | "SCN" => {
                return Ok(false)
            }
            "TJ" if op
                .operands
                .first()
                .and_then(|v| v.as_array().ok())
                .is_none_or(|array| array.iter().any(|v| !matches!(v, Object::String(_, _)))) =>
            {
                return Ok(false)
            }
            "Tc" | "Tw" | "Ts" | "Tr"
                if op.operands.len() != 1 || op.operands[0].as_float().ok() != Some(0.0) =>
            {
                return Ok(false)
            }
            "Tz" if op.operands.len() != 1 || op.operands[0].as_float().ok() != Some(100.0) => {
                return Ok(false)
            }
            _ => {}
        }
    }
    let (direct, inherited) = pdf.get_page_resources(page).map_err(lopdf_err)?;
    for resources in direct.into_iter().chain(
        inherited
            .iter()
            .filter_map(|id| pdf.get_dictionary(*id).ok()),
    ) {
        let Some(gs) = resources.get(b"ExtGState").ok() else {
            continue;
        };
        let (_, gs) = pdf.dereference(gs).map_err(lopdf_err)?;
        for (_, gs) in gs.as_dict().map_err(lopdf_err)?.iter() {
            let (_, gs) = pdf.dereference(gs).map_err(lopdf_err)?;
            for (key, value) in gs.as_dict().map_err(lopdf_err)?.iter() {
                if key == b"Type"
                    || ((key == b"CA" || key == b"ca") && value.as_float().ok() == Some(1.0))
                    || (key == b"BM" && value.as_name().ok() == Some(b"Normal"))
                {
                    continue;
                }
                return Ok(false);
            }
        }
    }
    Ok(true)
}

struct RetainedGlyph {
    text: String,
    font: PdfFontToken,
    size: PdfPoints,
    color: PdfColor,
    matrix: [f32; 6],
}

fn retained_text_runs(
    page: &PdfPage<'_>,
    object: &PdfPageObject<'_>,
    geo: &crate::PageGeometry,
    zones: &[PointRect],
) -> Result<Option<Vec<RetainedGlyph>>, EngineError> {
    let Some(text) = object.as_text_object() else {
        return Ok(None);
    };
    if text.font().is_embedded()? {
        return Ok(None);
    }
    if text.render_mode() != PdfPageTextRenderMode::FilledUnstroked
        || object.get_clip_path().is_some_and(|clip| !clip.is_empty())
    {
        return Ok(None);
    }
    if !matches!(
        text.font().name().as_str(),
        "Helvetica"
            | "Helvetica-Bold"
            | "Helvetica-Oblique"
            | "Helvetica-BoldOblique"
            | "Courier"
            | "Courier-Bold"
            | "Courier-Oblique"
            | "Courier-BoldOblique"
            | "Times-Roman"
            | "Times-Bold"
            | "Times-Italic"
            | "Times-BoldItalic"
    ) {
        return Ok(None);
    }
    let value = text.text();
    if value.is_empty() || value.len() > 4096 || !value.bytes().all(|b| (32..=126).contains(&b)) {
        return Ok(None);
    }
    let matrix = text.matrix()?;
    let (a, b, c, d) = (matrix.a(), matrix.b(), matrix.c(), matrix.d());
    if ![a, b, c, d, matrix.e(), matrix.f()]
        .iter()
        .all(|v| v.is_finite())
        || (a * a + b * b - 1.0).abs() > 0.0001
        || (c * c + d * d - 1.0).abs() > 0.0001
        || (a * c + b * d).abs() > 0.0001
        || matrix.determinant() < 0.9999
    {
        return Ok(None);
    }
    let color = object.fill_color()?;
    if color.alpha() != 255 {
        return Ok(None);
    }
    let page_text = page.text()?;
    let chars = page_text.chars_for_object(text)?;
    if chars.len() != value.len() {
        return Ok(None);
    }
    let mut retained = Vec::new();
    for (index, character) in value.chars().enumerate() {
        let glyph = chars.get(index)?;
        if glyph.is_generated()? || glyph.unicode_char() != Some(character) {
            return Ok(None);
        }
        let bounds = glyph.loose_bounds()?;
        let rect = geo.rect_to_display(PointRect {
            left: bounds.left().value,
            bottom: bounds.bottom().value,
            right: bounds.right().value,
            top: bounds.top().value,
        });
        let (x, y) = glyph.origin()?;
        if ![
            rect.left,
            rect.bottom,
            rect.right,
            rect.top,
            x.value,
            y.value,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Ok(None);
        }
        // Expanded glyph boxes cover descenders, accents and antialiasing at the cut boundary.
        let rect = PointRect {
            left: rect.left - 0.5,
            bottom: rect.bottom - 0.5,
            right: rect.right + 0.5,
            top: rect.top + 0.5,
        };
        if zones.iter().any(|zone| intersects(zone, &rect)) {
            continue;
        }
        retained.push(RetainedGlyph {
            text: character.to_string(),
            font: text.font().token(),
            size: text.unscaled_font_size(),
            color,
            matrix: [a, b, c, d, x.value, y.value],
        });
    }
    Ok(Some(retained))
}

// ---------------------------------------------------------------------------------------------
// Phase B : assainissement lopdf (pur, testable sans PDFium).

pub(crate) struct Sanitized {
    pub bytes: Vec<u8>,
    pub annotations_removed: usize,
    pub fields_removed: usize,
}

/// Catégories de `/Resources` filtrées.
const RESOURCE_KINDS: [&[u8]; 7] = [
    b"XObject",
    b"Font",
    b"ExtGState",
    b"Shading",
    b"ColorSpace",
    b"Pattern",
    b"Properties",
];

/// Clés conservées dans le dictionnaire d'une page marquée.
const PAGE_KEYS: [&[u8]; 14] = [
    b"Type",
    b"Parent",
    b"MediaBox",
    b"CropBox",
    b"BleedBox",
    b"TrimBox",
    b"ArtBox",
    b"Rotate",
    b"UserUnit",
    b"Contents",
    b"Resources",
    b"Annots",
    b"Group",
    b"Tabs",
];

/// Noms de ressources dessinés par un flux de contenu, par catégorie.
#[derive(Default)]
struct Used {
    names: BTreeMap<&'static [u8], BTreeSet<Vec<u8>>>,
    /// Catégories à garder entières (usage indéterminable, ex. image en ligne illisible).
    all: BTreeSet<&'static [u8]>,
}

impl Used {
    fn uses(&self, cat: &[u8], name: &[u8]) -> bool {
        self.all.contains(cat) || self.names.get(cat).is_some_and(|s| s.contains(name))
    }

    fn add(&mut self, cat: &'static [u8], name: Option<&Object>) {
        if let Some(Ok(n)) = name.map(Object::as_name) {
            self.names.entry(cat).or_default().insert(n.to_vec());
        }
    }

    fn note(&mut self, op: &Operation) {
        let first = op.operands.first();
        match op.operator.as_str() {
            "Do" => self.add(b"XObject", first),
            "Tf" => self.add(b"Font", first),
            "gs" => self.add(b"ExtGState", first),
            "sh" => self.add(b"Shading", first),
            "cs" | "CS" => self.add(b"ColorSpace", first),
            "scn" | "SCN" => self.add(b"Pattern", op.operands.last()),
            "BDC" | "DP" => self.add(b"Properties", op.operands.get(1)),
            "BI" => match first {
                Some(Object::Stream(s)) => {
                    let cs = s.dict.get(b"CS").or_else(|_| s.dict.get(b"ColorSpace"));
                    self.add(b"ColorSpace", cs.ok());
                }
                _ => {
                    self.all.insert(b"ColorSpace");
                }
            },
            _ => {}
        }
    }
}

fn deref_dict<'a>(doc: &'a lopdf::Document, o: &'a Object) -> Option<&'a Dictionary> {
    doc.dereference(o).ok()?.1.as_dict().ok()
}

fn ref_of(dict: &Dictionary, key: &[u8]) -> Option<ObjectId> {
    dict.get(key).and_then(Object::as_reference).ok()
}

/// Ressources effectives d'une page : les siennes, sinon celles du plus proche ancêtre (l'héritage
/// PDF remplace, il ne fusionne pas). Clone.
fn effective_resources(doc: &lopdf::Document, page_id: ObjectId) -> Option<Dictionary> {
    let mut id = page_id;
    for _ in 0..64 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(r) = dict.get(b"Resources") {
            return deref_dict(doc, r).cloned();
        }
        id = ref_of(dict, b"Parent")?;
    }
    None
}

fn stream_data(s: &lopdf::Stream) -> Vec<u8> {
    s.decompressed_content()
        .unwrap_or_else(|_| s.content.clone())
}

/// Noms utilisés par `content`, y compris dans les formulaires sans `/Resources` propres (ils
/// héritent de `resources`). Flux non décodable strictement : `RedactionIncomplete`.
fn collect_used(
    doc: &lopdf::Document,
    content: &[u8],
    resources: &Dictionary,
    seen: &mut BTreeSet<ObjectId>,
    used: &mut Used,
) -> Result<(), EngineError> {
    let ops = Content::decode_strict(content).map_err(|_| EngineError::RedactionIncomplete)?;
    let xobjects = resources
        .get(b"XObject")
        .ok()
        .and_then(|o| deref_dict(doc, o));
    for op in &ops.operations {
        used.note(op);
        if op.operator != "Do" {
            continue;
        }
        let Some(Ok(name)) = op.operands.first().map(Object::as_name) else {
            continue;
        };
        let Some(Ok(id)) = xobjects.map(|x| x.get(name).and_then(Object::as_reference)) else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        if let Ok(s) = doc.get_object(id).and_then(Object::as_stream) {
            let form =
                s.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Form".as_slice());
            if form && s.dict.get(b"Resources").is_err() {
                collect_used(doc, &stream_data(s), resources, seen, used)?;
            }
        }
    }
    Ok(())
}

fn used_by_page(doc: &lopdf::Document, page_id: ObjectId) -> Result<Used, EngineError> {
    let resources = effective_resources(doc, page_id).unwrap_or_default();
    let mut used = Used::default();
    collect_used(
        doc,
        &doc.get_page_content(page_id),
        &resources,
        &mut BTreeSet::new(),
        &mut used,
    )?;
    Ok(used)
}

/// `/Rect` d'une annotation (espace utilisateur), `None` s'il est absent ou illisible.
fn rect_of(doc: &lopdf::Document, dict: &Dictionary) -> Option<PointRect> {
    let arr = doc
        .dereference(dict.get(b"Rect").ok()?)
        .ok()?
        .1
        .as_array()
        .ok()?;
    if arr.len() != 4 {
        return None;
    }
    let v: Vec<f32> = arr
        .iter()
        .map(|o| doc.dereference(o).ok().and_then(|(_, o)| o.as_float().ok()))
        .collect::<Option<_>>()?;
    if !v.iter().all(|x| x.is_finite()) {
        return None;
    }
    Some(normalized(PointRect {
        left: v[0],
        bottom: v[1],
        right: v[2],
        top: v[3],
    }))
}

struct AnnotEntry {
    id: Option<ObjectId>,
    raw: Object,
    dict: Dictionary,
}

fn page_annotations(doc: &lopdf::Document, page_id: ObjectId) -> Option<Vec<AnnotEntry>> {
    let annots = doc.get_dictionary(page_id).ok()?.get(b"Annots").ok()?;
    let arr = doc.dereference(annots).ok()?.1.as_array().ok()?;
    Some(
        arr.iter()
            .map(|raw| {
                let (id, obj) = match raw {
                    Object::Reference(id) => (Some(*id), doc.get_object(*id).ok()),
                    other => (None, Some(other)),
                };
                AnnotEntry {
                    id,
                    raw: raw.clone(),
                    dict: obj
                        .and_then(|o| o.as_dict().ok())
                        .cloned()
                        .unwrap_or_default(),
                }
            })
            .collect(),
    )
}

/// Annotations retirées d'une page : `/Rect` sous une zone (ou absent), puis, jusqu'au point fixe,
/// popups et réponses (`/Parent`, `/IRT`) des annotations retirées et `/Popup` de celles-ci.
fn doomed_annotations(
    doc: &lopdf::Document,
    entries: &[AnnotEntry],
    zones: &[PointRect],
) -> Vec<bool> {
    let mut removed: Vec<bool> = entries
        .iter()
        .map(|e| rect_of(doc, &e.dict).is_none_or(|r| zones.iter().any(|z| intersects(z, &r))))
        .collect();
    loop {
        let gone = |r: &[bool]| -> (BTreeSet<ObjectId>, BTreeSet<ObjectId>) {
            let mut ids = BTreeSet::new();
            let mut popups = BTreeSet::new();
            for (e, _) in entries.iter().zip(r).filter(|(_, r)| **r) {
                ids.extend(e.id);
                popups.extend(ref_of(&e.dict, b"Popup"));
            }
            (ids, popups)
        };
        let (ids, popups) = gone(&removed);
        let mut changed = false;
        for (e, r) in entries.iter().zip(removed.iter_mut()).filter(|(_, r)| !**r) {
            let linked = [b"Parent".as_slice(), b"IRT"]
                .iter()
                .any(|k| ref_of(&e.dict, k).is_some_and(|id| ids.contains(&id)))
                || e.id.is_some_and(|id| popups.contains(&id));
            if linked {
                *r = true;
                changed = true;
            }
        }
        if !changed {
            return removed;
        }
    }
}

/// Champ terminal d'un widget retiré (premier dictionnaire portant `/T`), remonté tant que le parent
/// porte une valeur héritée (`/V`, `/DV`).
fn terminal_field(doc: &lopdf::Document, widget: &AnnotEntry) -> Option<ObjectId> {
    let mut cur = widget.id;
    let mut dict = widget.dict.clone();
    for _ in 0..64 {
        if dict.has(b"T") {
            let mut t = cur?;
            for _ in 0..64 {
                let parent = doc
                    .get_dictionary(t)
                    .ok()
                    .and_then(|d| ref_of(d, b"Parent"));
                match parent {
                    Some(p)
                        if doc
                            .get_dictionary(p)
                            .is_ok_and(|d| d.has(b"V") || d.has(b"DV")) =>
                    {
                        t = p
                    }
                    _ => break,
                }
            }
            return Some(t);
        }
        let parent = ref_of(&dict, b"Parent")?;
        dict = doc.get_dictionary(parent).ok()?.clone();
        cur = Some(parent);
    }
    None
}

fn collect_kids(doc: &lopdf::Document, id: ObjectId, out: &mut BTreeSet<ObjectId>) {
    if !out.insert(id) {
        return;
    }
    let kids: Vec<ObjectId> = doc
        .get_dictionary(id)
        .ok()
        .and_then(|d| d.get(b"Kids").ok())
        .and_then(|k| doc.dereference(k).ok())
        .and_then(|(_, k)| k.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default();
    for k in kids {
        collect_kids(doc, k, out);
    }
}

fn remove_refs(d: &mut Dictionary, id: ObjectId) {
    let keys: Vec<Vec<u8>> = d
        .iter()
        .filter(|(_, v)| matches!(v, Object::Reference(r) if *r == id))
        .map(|(k, _)| k.clone())
        .collect();
    for k in keys {
        d.remove(&k);
    }
}

/// Retire `id` de la catégorie `cat` des ressources de chaque nœud de l'arbre des pages.
fn strip_resource_refs(doc: &mut lopdf::Document, nodes: &[ObjectId], cat: &[u8], id: ObjectId) {
    for &node in nodes {
        let res_id = match doc
            .get_dictionary(node)
            .ok()
            .and_then(|d| d.get(b"Resources").ok())
        {
            Some(Object::Reference(r)) => Some(*r),
            Some(Object::Dictionary(_)) => None,
            _ => continue,
        };
        let sub_ref = {
            let res = match res_id {
                Some(r) => doc.get_dictionary_mut(r),
                None => doc
                    .get_dictionary_mut(node)
                    .and_then(|d| d.get_mut(b"Resources"))
                    .and_then(Object::as_dict_mut),
            };
            let Ok(res) = res else { continue };
            match res.get_mut(cat) {
                Ok(Object::Dictionary(sub)) => {
                    remove_refs(sub, id);
                    None
                }
                Ok(Object::Reference(s)) => Some(*s),
                _ => None,
            }
        };
        if let Some(Ok(sub)) = sub_ref.map(|s| doc.get_dictionary_mut(s)) {
            remove_refs(sub, id);
        }
    }
}

fn lopdf_err(e: lopdf::Error) -> EngineError {
    crate::security::lopdf_err(e)
}

/// Phase B. `zones` : page → zones en espace utilisateur (`EVERYWHERE` pour une page en image).
pub(crate) fn sanitize(
    input: &[u8],
    password: Option<&str>,
    zones: &BTreeMap<usize, Vec<PointRect>>,
    clean_hidden: bool,
) -> Result<Sanitized, EngineError> {
    let mut doc = crate::security::load(input, password)?;
    let pages = doc.get_pages();
    let mut doomed_annots = BTreeSet::new();
    let mut doomed_fields = BTreeSet::new();
    let mut terminals = BTreeSet::new();
    let mut annotations_removed = 0;
    let mut candidates: BTreeSet<(&'static [u8], ObjectId)> = BTreeSet::new();
    let mut used_cache: BTreeMap<ObjectId, Used> = BTreeMap::new();

    for (&index, rects) in zones {
        let page_id = *pages
            .get(&(index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(index))?;

        // B1. Annotations et champs.
        if let Some(entries) = page_annotations(&doc, page_id) {
            let removed = doomed_annotations(&doc, &entries, rects);
            let mut kept = Vec::new();
            for (e, r) in entries.iter().zip(&removed) {
                if !*r {
                    kept.push(e.raw.clone());
                    continue;
                }
                annotations_removed += 1;
                doomed_annots.extend(e.id);
                let widget = e.dict.get(b"Subtype").and_then(Object::as_name).ok()
                    == Some(b"Widget".as_slice());
                if let Some(t) = widget.then(|| terminal_field(&doc, e)).flatten() {
                    if terminals.insert(t) {
                        // Sous-arbre collecté tout de suite : le widget peut être le champ lui-même.
                        collect_kids(&doc, t, &mut doomed_fields);
                    }
                }
            }
            let page = doc.get_dictionary_mut(page_id).map_err(lopdf_err)?;
            if kept.is_empty() {
                page.remove(b"Annots");
            } else {
                page.set("Annots", Object::Array(kept));
            }
        }

        // B2. Liste blanche du dictionnaire de page.
        let page = doc.get_dictionary_mut(page_id).map_err(lopdf_err)?;
        let extra: Vec<Vec<u8>> = page
            .iter()
            .map(|(k, _)| k.clone())
            .filter(|k| !PAGE_KEYS.contains(&k.as_slice()))
            .collect();
        for k in extra {
            page.remove(&k);
        }

        // B3. Ressources de la page réduites aux noms que son contenu dessine.
        let resources = effective_resources(&doc, page_id).unwrap_or_default();
        let used = used_by_page(&doc, page_id)?;
        let mut filtered = resources.clone();
        for cat in RESOURCE_KINDS {
            if used.all.contains(cat) {
                continue;
            }
            let Some(sub) = resources.get(cat).ok().and_then(|o| deref_dict(&doc, o)) else {
                continue;
            };
            let mut keep = Dictionary::new();
            for (name, value) in sub.iter() {
                if used.uses(cat, name) {
                    keep.set(name.clone(), value.clone());
                } else if let Object::Reference(id) = value {
                    candidates.insert((cat, *id));
                }
            }
            filtered.set(cat, Object::Dictionary(keep));
        }
        doc.get_dictionary_mut(page_id)
            .map_err(lopdf_err)?
            .set("Resources", Object::Dictionary(filtered));
        used_cache.insert(page_id, used);
    }

    for id in doomed_annots.iter().chain(&doomed_fields) {
        if doc.objects.contains_key(id) {
            doc.delete_object(*id);
        }
    }
    if !terminals.is_empty() {
        // /XFA garde une copie XML des valeurs.
        let acro = doc
            .catalog()
            .ok()
            .and_then(|c| c.get(b"AcroForm").ok())
            .cloned();
        match acro {
            Some(Object::Reference(id)) => {
                if let Ok(d) = doc.get_dictionary_mut(id) {
                    d.remove(b"XFA");
                }
            }
            Some(Object::Dictionary(_)) => {
                if let Ok(Object::Dictionary(d)) =
                    doc.catalog_mut().map_err(lopdf_err)?.get_mut(b"AcroForm")
                {
                    d.remove(b"XFA");
                }
            }
            _ => {}
        }
    }

    // B4. Ressources retirées que plus aucune page ne dessine : détachées de tout l'arbre des pages
    //     (un dictionnaire partagé les garderait sinon atteignables). ponytail: O(candidats × pages),
    //     ressources effectives clonées à chaque fois ; à mettre en cache si un gros document le montre.
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();
    let mut nodes: BTreeSet<ObjectId> = BTreeSet::new();
    for &p in &page_ids {
        let mut id = Some(p);
        while let Some(n) = id.filter(|n| nodes.insert(*n)) {
            id = doc
                .get_dictionary(n)
                .ok()
                .and_then(|d| ref_of(d, b"Parent"));
        }
    }
    let nodes: Vec<ObjectId> = nodes.into_iter().collect();
    for (cat, id) in candidates {
        let mut drawn = false;
        for &p in &page_ids {
            let Some(res) = effective_resources(&doc, p) else {
                continue;
            };
            let names: Vec<Vec<u8>> = match res.get(cat).ok().and_then(|o| deref_dict(&doc, o)) {
                Some(sub) => sub
                    .iter()
                    .filter(|(_, v)| matches!(v, Object::Reference(r) if *r == id))
                    .map(|(k, _)| k.clone())
                    .collect(),
                None => continue,
            };
            if names.is_empty() {
                continue;
            }
            let used = match used_cache.entry(p) {
                std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::btree_map::Entry::Vacant(e) => e.insert(used_by_page(&doc, p)?),
            };
            if names.iter().any(|n| used.uses(cat, n)) {
                drawn = true;
                break;
            }
        }
        if !drawn {
            strip_resource_refs(&mut doc, &nodes, cat, id);
        }
    }

    // B5. Informations cachées.
    if clean_hidden {
        crate::security::strip_metadata_in(&mut doc);
        let catalog = doc.catalog_mut().map_err(lopdf_err)?;
        for key in [b"StructTreeRoot".as_slice(), b"MarkInfo", b"PieceInfo"] {
            catalog.remove(key);
        }
    }

    // B6. Ramasse-miettes et réécriture complète (un seul xref, aucune révision antérieure).
    doc.prune_objects();
    Ok(Sanitized {
        bytes: crate::security::save(doc)?,
        annotations_removed,
        fields_removed: terminals.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    fn rect(l: f32, b: f32, r: f32, t: f32) -> PointRect {
        PointRect {
            left: l,
            bottom: b,
            right: r,
            top: t,
        }
    }

    #[test]
    fn mark_pixels_round_outwards() {
        let (w, h) = (612.0_f32, 792.0_f32);
        let (pw, ph) = (1700_u32, 2200_u32);
        let (sx, sy) = (pw as f32 / w, ph as f32 / h);
        for r in [
            rect(100.3, 200.7, 180.1, 260.2),
            rect(0.0, 0.0, 0.5, 0.5),
            rect(611.6, 791.4, 612.0, 792.0),
            rect(-10.0, -10.0, 700.0, 900.0),
        ] {
            let [x0, y0, x1, y1] = pixel_rect(&r, h, sx, sy, pw, ph);
            assert!(x0 as f32 <= (r.left * sx).max(0.0), "{r:?}");
            assert!(x1 as f32 >= (r.right * sx).min(pw as f32), "{r:?}");
            assert!(y0 as f32 <= ((h - r.top) * sy).max(0.0), "{r:?}");
            assert!(y1 as f32 >= ((h - r.bottom) * sy).min(ph as f32), "{r:?}");
            assert!(x1 <= pw && y1 <= ph && x0 <= x1 && y0 <= y1);
        }
        assert!(intersects(&rect(0.0, 0.0, 10.0, 10.0), &EVERYWHERE));
        assert!(!intersects(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(10.0, 0.0, 20.0, 10.0)
        ));
    }

    #[test]
    fn marks_are_validated_and_grouped() {
        let m = |page, r| RedactMark { page, rect: r };
        assert!(group_marks(&[], 1).is_err());
        assert!(matches!(
            group_marks(&[m(3, rect(0.0, 0.0, 5.0, 5.0))], 2),
            Err(EngineError::PageOutOfRange(3))
        ));
        assert!(group_marks(&[m(0, rect(0.0, 0.0, 0.05, 5.0))], 1).is_err());
        assert!(group_marks(&[m(0, rect(f32::NAN, 0.0, 5.0, 5.0))], 1).is_err());
        let g = group_marks(
            &[
                m(1, rect(5.0, 5.0, 0.0, 0.0)),
                m(0, rect(0.0, 0.0, 1.0, 1.0)),
                m(1, rect(2.0, 2.0, 3.0, 3.0)),
            ],
            2,
        )
        .unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[&1][0], rect(0.0, 0.0, 5.0, 5.0), "rectangle renormalisé");
    }

    /// Document lopdf de `contents.len()` pages ; `page_extra(i)` complète le dictionnaire de page i.
    fn build(
        contents: &[&[u8]],
        mut page_extra: impl FnMut(&mut lopdf::Document, usize) -> Dictionary,
        catalog_extra: Dictionary,
    ) -> lopdf::Document {
        let mut doc = lopdf::Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for (i, c) in contents.iter().enumerate() {
            let content = doc.add_object(Stream::new(Dictionary::new(), c.to_vec()));
            let mut page = dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Contents" => content,
            };
            for (k, v) in page_extra(&mut doc, i).iter() {
                page.set(k.clone(), v.clone());
            }
            kids.push(Object::Reference(doc.add_object(page)));
        }
        let count = kids.len() as i64;
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
        );
        let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
        for (k, v) in catalog_extra.iter() {
            catalog.set(k.clone(), v.clone());
        }
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);
        doc
    }

    fn bytes_of(doc: &mut lopdf::Document) -> Vec<u8> {
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    fn contains(hay: &[u8], needle: &str) -> bool {
        hay.windows(needle.len()).any(|w| w == needle.as_bytes())
    }

    fn zone_page0() -> BTreeMap<usize, Vec<PointRect>> {
        BTreeMap::from([(0, vec![rect(50.0, 50.0, 100.0, 100.0)])])
    }

    fn shared_images(page2: &'static [u8]) -> Vec<u8> {
        let mut res_id = None;
        let mut doc = build(
            &[b"q /Im2 Do Q", page2],
            |doc, _| {
                let id = *res_id.get_or_insert_with(|| {
                    let im = |doc: &mut lopdf::Document, mark: &str| {
                        doc.add_object(Stream::new(
                            dictionary! { "Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
                                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8 },
                            mark.as_bytes().to_vec(),
                        ))
                    };
                    let im1 = im(doc, "IM1-MARK");
                    let im2 = im(doc, "IM2-MARK");
                    doc.add_object(dictionary! { "XObject" => dictionary! { "Im1" => im1, "Im2" => im2 } })
                });
                dictionary! { "Resources" => id }
            },
            Dictionary::new(),
        );
        bytes_of(&mut doc)
    }

    #[test]
    fn sanitize_deletes_resources_no_page_draws() {
        let input = shared_images(b"q /Im2 Do Q");
        assert!(contains(&input, "IM1-MARK") && contains(&input, "IM2-MARK"));
        let out = sanitize(&input, None, &zone_page0(), false).unwrap().bytes;
        assert!(
            !contains(&out, "IM1-MARK"),
            "image dessinée par aucune page"
        );
        assert!(contains(&out, "IM2-MARK"));

        let out = sanitize(&shared_images(b"q /Im1 Do Q"), None, &zone_page0(), false)
            .unwrap()
            .bytes;
        assert!(contains(&out, "IM1-MARK"), "encore dessinée par la page 2");

        let err = sanitize(&shared_images(b"q [ (x"), None, &zone_page0(), false);
        assert!(matches!(err, Err(EngineError::RedactionIncomplete)));
    }

    #[test]
    fn sanitize_removes_popups_replies_and_terminal_fields() {
        let mut ids: BTreeMap<&str, ObjectId> = BTreeMap::new();
        let mut doc = build(
            &[b"", b""],
            |doc, i| {
                let r = |l: i64, b: i64| vec![l.into(), b.into(), (l + 20).into(), (b + 20).into()];
                if i == 0 {
                    let note = doc.new_object_id();
                    let popup = doc
                        .add_object(dictionary! { "Type" => "Annot", "Subtype" => "Popup",
                        "Rect" => r(400, 700), "Parent" => note, "Contents" => "POPUP-GONE" });
                    doc.objects.insert(note, Object::Dictionary(dictionary! { "Type" => "Annot",
                        "Subtype" => "Text", "Rect" => r(60, 60), "Popup" => popup, "Contents" => "NOTE-GONE" }));
                    let reply = doc
                        .add_object(dictionary! { "Type" => "Annot", "Subtype" => "Text",
                        "Rect" => r(300, 300), "IRT" => note, "Contents" => "REPLY-GONE" });
                    let keep = doc
                        .add_object(dictionary! { "Type" => "Annot", "Subtype" => "Square",
                        "Rect" => r(300, 500), "Contents" => "KEEP-ANNOT" });
                    let grp = doc.new_object_id();
                    let field = doc.new_object_id();
                    let w1 = doc
                        .add_object(dictionary! { "Type" => "Annot", "Subtype" => "Widget",
                        "Rect" => r(70, 70), "Parent" => field });
                    let w2 = doc.new_object_id();
                    doc.objects.insert(field, Object::Dictionary(dictionary! { "FT" => "Tx",
                        "T" => "FIELD-GONE", "V" => "VALUE-GONE", "Parent" => grp, "Kids" => vec![w1.into(), w2.into()] }));
                    doc.objects.insert(
                        grp,
                        Object::Dictionary(dictionary! { "T" => "GRP-KEEP",
                        "Kids" => vec![field.into()] }),
                    );
                    ids.insert("grp", grp);
                    ids.insert("w2", w2);
                    dictionary! { "Annots" => vec![note.into(), popup.into(), reply.into(), keep.into(), w1.into()],
                    "Thumb" => "THUMB-GONE", "StructParents" => 0 }
                } else {
                    let w2 = ids["w2"];
                    let field = doc
                        .get_dictionary(ids["grp"])
                        .unwrap()
                        .get(b"Kids")
                        .unwrap()
                        .as_array()
                        .unwrap()[0]
                        .as_reference()
                        .unwrap();
                    doc.objects.insert(
                        w2,
                        Object::Dictionary(dictionary! { "Type" => "Annot",
                        "Subtype" => "Widget", "Rect" => r(70, 70), "Parent" => field }),
                    );
                    dictionary! { "Annots" => vec![w2.into()] }
                }
            },
            Dictionary::new(),
        );
        let grp = ids["grp"];
        doc.catalog_mut().unwrap().set(
            "AcroForm",
            dictionary! { "Fields" => vec![grp.into()], "XFA" => "XFA-GONE" },
        );
        let input = bytes_of(&mut doc);
        let s = sanitize(&input, None, &zone_page0(), false).unwrap();
        assert_eq!(s.annotations_removed, 4, "note, popup, réponse, widget");
        assert_eq!(s.fields_removed, 1);
        for gone in [
            "NOTE-GONE",
            "POPUP-GONE",
            "REPLY-GONE",
            "FIELD-GONE",
            "VALUE-GONE",
            "XFA-GONE",
            "THUMB-GONE",
        ] {
            assert!(!contains(&s.bytes, gone), "{gone}");
        }
        assert!(contains(&s.bytes, "KEEP-ANNOT") && contains(&s.bytes, "GRP-KEEP"));
        let out = lopdf::Document::load_mem(&s.bytes).unwrap();
        let p2 = out.get_pages()[&2];
        let annots = out.get_dictionary(p2).unwrap().get(b"Annots");
        assert!(
            annots.map_or(true, |a| a.as_array().unwrap().is_empty()),
            "le second widget du champ disparaît aussi de la page 2"
        );
    }
}
