//! Édition du contenu des pages : objets texte, image, tracé et groupe (PDFium `FPDFPageObj_*`).
//! Chaque modification régénère le flux de contenu de la page (stratégie par défaut de pdfium-render).
//!
//! Toutes les coordonnées (entrées et `ObjectInfo`) sont en **espace d'affichage** (voir
//! `geometry.rs`) : les outils tombent au bon endroit sur les pages pivotées ou rognées.

use crate::geometry::{geometry_of, PageGeometry};
use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Text,
    Image,
    Path,
    /// XObject formulaire : groupe d'objets (copies, collages, duplications).
    Form,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub index: usize,
    pub kind: ObjectKind,
    /// Boîte englobante (espace d'affichage) des sommets de `quad`.
    pub bounds: PointRect,
    /// Sommets du cadre orienté (serré pour texte et image), espace d'affichage, ordre quelconque.
    pub quad: [(f32, f32); 4],
    /// Rotation à l'écran, degrés, sens horaire, dans (-180, 180].
    pub angle_deg: f32,
    /// 0 (transparent) à 1 (opaque) ; `None` : objet transparent dont PDFium ne donne pas la
    /// valeur (images).
    pub opacity: Option<f32>,
    /// Contenu d'un objet texte.
    pub text: Option<String>,
    pub font_size: f32,
    pub font_name: Option<String>,
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub stroke_rgb: Option<(u8, u8, u8)>,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFamily {
    #[default]
    Helvetica,
    Times,
    Courier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub family: FontFamily,
    pub rgb: (u8, u8, u8),
    /// 0 (invisible) à 1 (opaque).
    pub opacity: f32,
    /// Police TrueType du système à incorporer (`family`, `bold`, `italic` sont alors ignorés).
    pub font_file: Option<PathBuf>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size_pt: 12.0,
            bold: false,
            italic: false,
            family: FontFamily::Helvetica,
            rgb: (0, 0, 0),
            opacity: 1.0,
            font_file: None,
        }
    }
}

/// Couleurs, épaisseur et opacité à appliquer à des objets existants (`None` = inchangé).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectStyle {
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub stroke_rgb: Option<(u8, u8, u8)>,
    pub stroke_width: Option<f32>,
    /// 0 à 1 (bornée).
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    Line,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeStyle {
    pub stroke_rgb: (u8, u8, u8),
    pub stroke_width: f32,
    pub fill_rgb: Option<(u8, u8, u8)>,
    /// 0 à 1.
    pub opacity: f32,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            stroke_rgb: (0x25, 0x63, 0xEB),
            stroke_width: 2.0,
            fill_rgb: None,
            opacity: 1.0,
        }
    }
}

/// Ordre de dessin : premier plan, arrière-plan, un cran vers l'avant ou l'arrière.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrder {
    Front,
    Back,
    Forward,
    Backward,
}

/// Sélection par rectangle : objets entièrement contenus, ou simplement touchés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectMode {
    Contained,
    Touching,
}

fn alpha_u8(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color_a(rgb: (u8, u8, u8), alpha: u8) -> PdfColor {
    PdfColor::new(rgb.0, rgb.1, rgb.2, alpha)
}

fn not_found(index: usize) -> EngineError {
    EngineError::Other(format!("object {index} not found"))
}

/// Cadre orienté d'un objet, espace d'affichage.
pub(crate) fn display_quad(object: &PdfPageObject<'_>, geo: &PageGeometry) -> [(f32, f32); 4] {
    match object.bounds() {
        Ok(q) => [
            (q.x1().value, q.y1().value),
            (q.x2().value, q.y2().value),
            (q.x3().value, q.y3().value),
            (q.x4().value, q.y4().value),
        ]
        .map(|(x, y)| geo.to_display(x, y)),
        Err(_) => [(0.0, 0.0); 4],
    }
}

pub(crate) fn quad_bounds(q: &[(f32, f32); 4]) -> PointRect {
    q.iter().fold(
        PointRect {
            left: f32::MAX,
            bottom: f32::MAX,
            right: f32::MIN,
            top: f32::MIN,
        },
        |r, p| PointRect {
            left: r.left.min(p.0),
            bottom: r.bottom.min(p.1),
            right: r.right.max(p.0),
            top: r.top.max(p.1),
        },
    )
}

/// Point dans un quadrilatère convexe (sommets dans un ordre quelconque : triés par angle).
fn in_quad(q: &[(f32, f32); 4], x: f32, y: f32) -> bool {
    let (cx, cy) = (
        q.iter().map(|p| p.0).sum::<f32>() / 4.0,
        q.iter().map(|p| p.1).sum::<f32>() / 4.0,
    );
    let mut pts = *q;
    pts.sort_by(|a, b| {
        (a.1 - cy)
            .atan2(a.0 - cx)
            .total_cmp(&(b.1 - cy).atan2(b.0 - cx))
    });
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (pts[i], pts[(i + 1) % 4]);
        let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// Angle (degrés) qui n'est pas un multiple de 90° à 0,5° près.
fn tilted(deg: f32) -> bool {
    (deg / 90.0 - (deg / 90.0).round()).abs() * 90.0 > 0.5
}

/// Angle normalisé dans (-180, 180].
pub(crate) fn norm180(deg: f32) -> f32 {
    let d = deg.rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

/// Échelle verticale de la matrice d'un texte, rotation comprise : longueur du vecteur (c, d).
/// (`scaled_font_size` de pdfium-render ne lit que `d`, nul pour un texte à 90° : taille 0, puis
/// multipliée à la modification.)
fn text_scale(t: &PdfPageTextObject<'_>) -> f32 {
    t.matrix()
        .map(|m| (m.c() * m.c() + m.d() * m.d()).sqrt())
        .ok()
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0)
}

fn kind_of(object: &PdfPageObject<'_>) -> ObjectKind {
    match object.object_type() {
        PdfPageObjectType::Text => ObjectKind::Text,
        PdfPageObjectType::Image => ObjectKind::Image,
        PdfPageObjectType::Path => ObjectKind::Path,
        PdfPageObjectType::XObjectForm => ObjectKind::Form,
        _ => ObjectKind::Other,
    }
}

fn describe(
    index: usize,
    object: &PdfPageObject<'_>,
    geo: &PageGeometry,
    text_page: Option<&PdfPageText<'_>>,
) -> ObjectInfo {
    let kind = kind_of(object);
    let quad = display_quad(object, geo);
    let (text, font_size, font_name) = match object.as_text_object() {
        Some(t) => (
            // Une seule page texte pour toute la liste (`text()` en ouvrirait une par objet).
            // La page texte ajoute des espaces et sauts de ligne générés en fin d'objet.
            Some(match text_page {
                Some(tp) => tp
                    .for_object(t)
                    .trim_end_matches([' ', '\r', '\n'])
                    .to_string(),
                None => t.text(),
            }),
            t.unscaled_font_size().value * text_scale(t),
            Some(t.font().family()).filter(|n| !n.is_empty()),
        ),
        None => (None, 0.0, None),
    };
    let rgb = |c: PdfColor| (c.red(), c.green(), c.blue());
    let fill = object.fill_color().ok();
    let stroke = object.stroke_color().ok();
    let opacity = match fill {
        Some(c) => {
            let fill_alpha = c.alpha() as f32 / 255.0;
            Some(match (kind, stroke) {
                (ObjectKind::Path, Some(s)) => fill_alpha.min(s.alpha() as f32 / 255.0),
                _ => fill_alpha,
            })
        }
        // PDFium ne donne pas la couleur (donc l'alpha) d'une image : seulement si elle est
        // transparente.
        None if object.has_transparency() => None,
        None => Some(1.0),
    };
    ObjectInfo {
        index,
        kind,
        bounds: quad_bounds(&quad),
        quad,
        angle_deg: norm180(object.get_rotation_clockwise_degrees() + geo.rotation_degrees()),
        opacity,
        text,
        font_size,
        font_name,
        fill_rgb: fill.map(rgb),
        stroke_rgb: stroke.map(rgb),
        stroke_width: object.stroke_width().map(|w| w.value).unwrap_or(0.0),
    }
}

fn font_token(fonts: &mut PdfFonts, style: &TextStyle) -> PdfFontToken {
    use FontFamily::*;
    match (style.family, style.bold, style.italic) {
        (Helvetica, false, false) => fonts.helvetica(),
        (Helvetica, true, false) => fonts.helvetica_bold(),
        (Helvetica, false, true) => fonts.helvetica_oblique(),
        (Helvetica, true, true) => fonts.helvetica_bold_oblique(),
        (Times, false, false) => fonts.times_roman(),
        (Times, true, false) => fonts.times_bold(),
        (Times, false, true) => fonts.times_italic(),
        (Times, true, true) => fonts.times_bold_italic(),
        (Courier, false, false) => fonts.courier(),
        (Courier, true, false) => fonts.courier_bold(),
        (Courier, false, true) => fonts.courier_oblique(),
        (Courier, true, true) => fonts.courier_bold_oblique(),
    }
}

/// Index triés, dédoublonnés et validés (`count` objets sur la page).
pub(crate) fn checked_indices(indices: &[usize], count: usize) -> Result<Vec<usize>, EngineError> {
    if indices.is_empty() {
        return Err(EngineError::Other("no objects selected".into()));
    }
    let mut v = indices.to_vec();
    v.sort_unstable();
    v.dedup();
    match v.iter().find(|&&i| i >= count) {
        Some(&bad) => Err(not_found(bad)),
        None => Ok(v),
    }
}

/// Nouvel ordre de dessin (indices d'origine) après `op` sur `selected` (trié, sans doublon).
/// L'ordre relatif des objets sélectionnés est conservé ; un bloc contigu avance d'un seul cran.
pub(crate) fn reorder_target(count: usize, selected: &[usize], op: ZOrder) -> Vec<usize> {
    let sel = |i: &usize| selected.binary_search(i).is_ok();
    let mut order: Vec<usize> = (0..count).collect();
    match op {
        ZOrder::Front => {
            order.sort_by_key(|i| sel(i));
        }
        ZOrder::Back => {
            order.sort_by_key(|i| !sel(i));
        }
        ZOrder::Forward => {
            for i in (0..count.saturating_sub(1)).rev() {
                if sel(&order[i]) && !sel(&order[i + 1]) {
                    order.swap(i, i + 1);
                }
            }
        }
        ZOrder::Backward => {
            for i in 1..count {
                if sel(&order[i]) && !sel(&order[i - 1]) {
                    order.swap(i, i - 1);
                }
            }
        }
    }
    order
}

impl Document<'_> {
    /// Tous les objets d'une page, dans l'ordre de dessin (le dernier est devant).
    pub fn page_objects(&self, page_index: usize) -> Result<Vec<ObjectInfo>, EngineError> {
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let text = page.text().ok();
        Ok(page
            .objects()
            .iter()
            .enumerate()
            .map(|(i, o)| describe(i, &o, &geo, text.as_ref()))
            .collect())
    }

    pub fn object_info(&self, page_index: usize, index: usize) -> Result<ObjectInfo, EngineError> {
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| not_found(index))?;
        Ok(describe(index, &object, &geo, None))
    }

    /// Objet le plus en avant contenant le point (x, y) d'affichage.
    pub fn object_at(
        &self,
        page_index: usize,
        x: f32,
        y: f32,
    ) -> Result<Option<ObjectInfo>, EngineError> {
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let objects = page.objects();
        for i in (0..objects.len()).rev() {
            let Ok(object) = objects.get(i) else { continue };
            let quad = display_quad(&object, &geo);
            let b = quad_bounds(&quad);
            let in_box =
                x >= b.left - 1.0 && x <= b.right + 1.0 && y >= b.bottom - 1.0 && y <= b.top + 1.0;
            // Texte ou image pivotés : test sur le cadre orienté, pas sur la boîte englobante.
            let oriented = matches!(kind_of(&object), ObjectKind::Text | ObjectKind::Image)
                && tilted(norm180(object.get_rotation_clockwise_degrees()));
            if in_box && (!oriented || in_quad(&quad, x, y)) {
                let text = page.text().ok();
                return Ok(Some(describe(i, &object, &geo, text.as_ref())));
            }
        }
        Ok(None)
    }

    /// Objets dans un rectangle d'affichage (entièrement contenus, ou touchés).
    pub fn objects_in_rect(
        &self,
        page_index: usize,
        rect: PointRect,
        mode: RectMode,
    ) -> Result<Vec<ObjectInfo>, EngineError> {
        let (l, r) = (rect.left.min(rect.right), rect.left.max(rect.right));
        let (b, t) = (rect.bottom.min(rect.top), rect.bottom.max(rect.top));
        Ok(self
            .page_objects(page_index)?
            .into_iter()
            .filter(|o| match mode {
                RectMode::Contained => o
                    .quad
                    .iter()
                    .all(|p| p.0 >= l && p.0 <= r && p.1 >= b && p.1 <= t),
                RectMode::Touching => {
                    o.bounds.left <= r
                        && o.bounds.right >= l
                        && o.bounds.bottom <= t
                        && o.bounds.top >= b
                }
            })
            .collect())
    }

    /// Ajoute du texte dont le coin bas-gauche de la première ligne de base est en (x, y), droit à
    /// l'écran. Une ligne par `\n` (interligne 1,2 × taille) ; une tabulation vaut quatre espaces.
    /// Police standard (WinAnsi : latin seulement) ou police TrueType du système incorporée
    /// (`style.font_file`) : un caractère absent de la police donne
    /// [`EngineError::MissingGlyphs`] (qui liste ces caractères) et le document reste intact.
    /// Renvoie le nombre d'objets ajoutés.
    pub fn add_text(
        &mut self,
        page_index: usize,
        x: f32,
        y: f32,
        text: &str,
        style: &TextStyle,
    ) -> Result<usize, EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Other("invalid text coordinates".into()));
        }
        if text.trim().is_empty() {
            return Err(EngineError::Other("empty text".into()));
        }
        if !(style.size_pt.is_finite() && style.size_pt > 0.0) {
            return Err(EngineError::Other("invalid font size".into()));
        }
        let geo = self.page_geometry(page_index)?;
        let font = match &style.font_file {
            Some(path) => self.font_token_for_file(path)?,
            None => font_token(self.inner.fonts_mut(), style),
        };
        let alpha = alpha_u8(style.opacity);
        let pitch = style.size_pt * 1.2;
        let mut page = self.page_handle(page_index)?;
        let mut added = 0usize;
        // Aucune police PDF n'a de glyphe de tabulation (PDFium écrirait un caractère de
        // remplacement) : quatre espaces, comme un éditeur de texte.
        let text = text.replace('\t', "    ");
        for (i, line) in text
            .split('\n')
            .map(|l| l.trim_end_matches('\r'))
            .enumerate()
        {
            if line.trim().is_empty() {
                continue; // ligne vide = interligne
            }
            let mut object =
                PdfPageTextObject::new(&self.inner, line, font, PdfPoints::new(style.size_pt))?;
            object.set_fill_color(color_a(style.rgb, alpha))?;
            let m = geo.upright_matrix(x, y - i as f32 * pitch);
            object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
            let added_object = page.objects_mut().add_text_object(object)?;
            added += 1;
            // PDFium remplace sans erreur un caractère absent de la police (glyphe 0, ou rien pour
            // une police standard WinAnsi) : on relit le texte pour le détecter.
            let back = added_object
                .as_text_object()
                .map(|t| t.text())
                .unwrap_or_default();
            let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            if squash(&back) != squash(line) {
                for _ in 0..added {
                    let n = page.objects().len();
                    page.objects_mut().remove_object_at_index(n - 1)?;
                }
                page.regenerate_content()?;
                // Caractères perdus ou remplacés : ceux de la ligne absents de la relecture,
                // chacun une seule fois, dans l'ordre du texte.
                let mut seen = std::collections::HashSet::new();
                let missing: String = line
                    .chars()
                    .filter(|c| !c.is_whitespace() && !back.contains(*c) && seen.insert(*c))
                    .collect();
                return Err(EngineError::MissingGlyphs(missing));
            }
        }
        page.regenerate_content()?;
        Ok(added)
    }

    /// Police système incorporée une seule fois par document (cache vidé à chaque rechargement).
    fn font_token_for_file(&mut self, path: &Path) -> Result<PdfFontToken, EngineError> {
        if let Some(t) = self.font_cache.get(path) {
            return Ok(*t);
        }
        let bytes = std::fs::read(path)?;
        crate::fonts::check_embeddable(&bytes)?;
        let token = self
            .inner
            .fonts_mut()
            .load_true_type_from_bytes(&bytes, true)
            .map_err(|_| EngineError::FontNotEmbeddable)?;
        self.font_cache.insert(path.to_path_buf(), token);
        Ok(token)
    }

    /// Remplace le texte d'un objet texte existant (la police d'origine est conservée).
    pub fn set_object_text(
        &mut self,
        page_index: usize,
        index: usize,
        text: &str,
    ) -> Result<(), EngineError> {
        if text.is_empty() {
            return Err(EngineError::Other("empty text".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| not_found(index))?;
        let Some(text_object) = object.as_text_object_mut() else {
            return Err(EngineError::Other("not a text object".into()));
        };
        text_object.set_text(text)?;
        drop(object);
        // Les objets modifiés en place ne régénèrent pas le flux de contenu : le faire explicitement.
        page.regenerate_content()?;
        Ok(())
    }

    pub fn move_object(
        &mut self,
        page_index: usize,
        index: usize,
        dx: f32,
        dy: f32,
    ) -> Result<(), EngineError> {
        self.move_objects(page_index, &[index], dx, dy)
    }

    /// Déplace plusieurs objets du même vecteur d'affichage (une seule régénération).
    pub fn move_objects(
        &mut self,
        page_index: usize,
        indices: &[usize],
        dx: f32,
        dy: f32,
    ) -> Result<(), EngineError> {
        if !(dx.is_finite() && dy.is_finite()) {
            return Err(EngineError::Other("invalid offset".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let indices = checked_indices(indices, page.objects().len())?;
        let (ux, uy) = geo.vec_to_user(dx, dy);
        for i in indices {
            let mut object = page.objects().get(i as PdfPageObjectIndex)?;
            object.translate(PdfPoints::new(ux), PdfPoints::new(uy))?;
        }
        // Les objets modifiés en place ne régénèrent pas le flux de contenu : le faire explicitement.
        page.regenerate_content()?;
        Ok(())
    }

    /// Redimensionne un objet pour que ses limites d'affichage deviennent `bounds` (mise à
    /// l'échelle de sa matrice ; le coin bas-gauche d'origine est ramené sur le nouveau).
    /// Un objet pivoté d'un angle non multiple de 90° est mis à l'échelle uniformément (le plus
    /// petit des deux facteurs) : des facteurs différents selon les axes de la page le cisailleraient.
    pub fn resize_object(
        &mut self,
        page_index: usize,
        index: usize,
        bounds: PointRect,
    ) -> Result<(), EngineError> {
        let geo = self.page_geometry(page_index)?;
        let info = self.object_info(page_index, index)?;
        let current = geo.rect_to_user(info.bounds);
        let target = geo.rect_to_user(bounds);
        let (old_w, old_h) = (current.right - current.left, current.top - current.bottom);
        let (new_w, new_h) = (target.right - target.left, target.top - target.bottom);
        if !(old_w > 0.0 && old_h > 0.0 && new_w > 0.0 && new_h > 0.0) {
            return Err(EngineError::Other("invalid object size".into()));
        }
        let (mut sx, mut sy) = (new_w / old_w, new_h / old_h);
        if tilted(info.angle_deg) {
            sx = sx.min(sy);
            sy = sx;
        }
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| not_found(index))?;
        object.translate(
            PdfPoints::new(-current.left),
            PdfPoints::new(-current.bottom),
        )?;
        object.scale(sx, sy)?;
        object.translate(PdfPoints::new(target.left), PdfPoints::new(target.bottom))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Change couleurs, épaisseur et opacité d'un objet (texte : couleur de remplissage).
    pub fn set_object_style(
        &mut self,
        page_index: usize,
        index: usize,
        style: &ObjectStyle,
    ) -> Result<(), EngineError> {
        self.set_objects_style(page_index, &[index], style)
    }

    /// Style appliqué à plusieurs objets. L'opacité courante est conservée quand seule la
    /// couleur change (`PdfColor` porte l'alpha).
    pub fn set_objects_style(
        &mut self,
        page_index: usize,
        indices: &[usize],
        style: &ObjectStyle,
    ) -> Result<(), EngineError> {
        if style.opacity.is_some_and(|o| !o.is_finite()) {
            return Err(EngineError::Other("invalid opacity".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let indices = checked_indices(indices, page.objects().len())?;
        for i in indices {
            let mut object = page.objects().get(i as PdfPageObjectIndex)?;
            let is_path = object.object_type() == PdfPageObjectType::Path;
            let fill = object.fill_color().unwrap_or(PdfColor::new(0, 0, 0, 255));
            let alpha = style.opacity.map(alpha_u8);
            if style.fill_rgb.is_some() || alpha.is_some() {
                let rgb = style
                    .fill_rgb
                    .unwrap_or((fill.red(), fill.green(), fill.blue()));
                object.set_fill_color(color_a(rgb, alpha.unwrap_or(fill.alpha())))?;
            }
            if is_path {
                let stroke = object.stroke_color().unwrap_or(PdfColor::new(0, 0, 0, 255));
                if style.stroke_rgb.is_some() || alpha.is_some() {
                    let rgb =
                        style
                            .stroke_rgb
                            .unwrap_or((stroke.red(), stroke.green(), stroke.blue()));
                    object.set_stroke_color(color_a(rgb, alpha.unwrap_or(stroke.alpha())))?;
                }
                if let Some(w) = style.stroke_width {
                    object.set_stroke_width(PdfPoints::new(w.max(0.0)))?;
                }
            }
        }
        page.regenerate_content()?;
        Ok(())
    }

    /// Change la taille affichée (points) d'un objet texte existant.
    pub fn set_text_size(
        &mut self,
        page_index: usize,
        index: usize,
        size_pt: f32,
    ) -> Result<(), EngineError> {
        if !(size_pt.is_finite() && size_pt > 0.0) {
            return Err(EngineError::Other("invalid font size".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| not_found(index))?;
        let Some(text_object) = object.as_text_object() else {
            return Err(EngineError::Other("not a text object".into()));
        };
        let ratio = text_scale(text_object);
        text_object.set_unscaled_font_size(PdfPoints::new(size_pt / ratio))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Fait pivoter un objet autour du centre de ses limites (degrés, sens horaire à l'écran).
    // ponytail: pivot = centre de la boîte englobante ; un tracé asymétrique peut dériver d'un
    // petit écart à chaque rotation successive. Pivot mémorisé côté UI si cela gêne.
    pub fn rotate_object(
        &mut self,
        page_index: usize,
        index: usize,
        degrees: f32,
    ) -> Result<(), EngineError> {
        if !degrees.is_finite() {
            return Err(EngineError::Other("invalid angle".into()));
        }
        let geo = self.page_geometry(page_index)?;
        let b = self.object_info(page_index, index)?.bounds;
        // Une rotation horaire à l'écran est une rotation horaire en espace utilisateur.
        let (cx, cy) = geo.to_user((b.left + b.right) / 2.0, (b.bottom + b.top) / 2.0);
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| not_found(index))?;
        object.translate(PdfPoints::new(-cx), PdfPoints::new(-cy))?;
        object.rotate_clockwise_degrees(degrees)?;
        object.translate(PdfPoints::new(cx), PdfPoints::new(cy))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Duplique un objet (groupe décalé de 12 pt) ; renvoie l'index de la copie. L'original ne
    /// change pas de plan.
    pub fn duplicate_object(
        &mut self,
        page_index: usize,
        index: usize,
    ) -> Result<usize, EngineError> {
        self.duplicate_objects(page_index, &[index], 12.0, -12.0)
    }

    /// Duplique des objets en un groupe décalé de (dx, dy) ; renvoie l'index du groupe.
    pub fn duplicate_objects(
        &mut self,
        page_index: usize,
        indices: &[usize],
        dx: f32,
        dy: f32,
    ) -> Result<usize, EngineError> {
        let clip = self.copy_objects(page_index, indices)?;
        self.paste_objects(page_index, &clip, dx, dy)
    }

    /// Fusionne les flux de contenu d'une page en un seul (lopdf), si elle en a plusieurs.
    /// PDFium ordonne le dessin flux par flux : un seul flux rend l'ordre des objets fidèle
    /// à l'ordre de la liste.
    fn merge_content_streams(&mut self, page_index: usize) -> Result<bool, EngineError> {
        let bytes = self.snapshot()?;
        let mut doc = crate::security::load(&bytes, self.password())?;
        let page_id = *doc
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        if doc.get_page_contents(page_id).len() <= 1 {
            return Ok(false);
        }
        let content = doc.get_page_content(page_id);
        let stream_id = doc.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), content));
        doc.get_object_mut(page_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| EngineError::Other(e.to_string()))?
            .set("Contents", lopdf::Object::Reference(stream_id));
        let out = crate::security::save(doc)?;
        self.restore(out)?;
        Ok(true)
    }

    /// Place un objet au premier plan (`front = true`) ou à l'arrière-plan ; renvoie son nouvel index.
    pub fn reorder_object(
        &mut self,
        page_index: usize,
        index: usize,
        front: bool,
    ) -> Result<usize, EngineError> {
        let op = if front { ZOrder::Front } else { ZOrder::Back };
        Ok(self.reorder_objects(page_index, &[index], op)?[0])
    }

    /// Change l'ordre de dessin de plusieurs objets ; renvoie leurs nouveaux index (dans l'ordre
    /// croissant des index d'origine).
    pub fn reorder_objects(
        &mut self,
        page_index: usize,
        indices: &[usize],
        op: ZOrder,
    ) -> Result<Vec<usize>, EngineError> {
        let count = self.page_handle(page_index)?.objects().len();
        let selected = checked_indices(indices, count)?;
        let target = reorder_target(count, &selected, op);
        // Un objet retiré puis réinséré garde son flux d'origine et PDFium dessine flux par flux :
        // l'ordre n'est respecté qu'au sein d'un flux unique, premier plan compris (un objet
        // ajouté par PDFium vit dans son propre flux, ajouté en fin).
        self.merge_content_streams(page_index)?;
        let mut page = self.page_handle(page_index)?;
        if op == ZOrder::Front {
            for (k, &i) in selected.iter().enumerate() {
                let object = page
                    .objects_mut()
                    .remove_object_at_index((i - k) as PdfPageObjectIndex)?;
                page.objects_mut().add_object(object)?;
            }
        } else {
            let mut current: Vec<usize> = (0..count).collect();
            for (k, &id) in target.iter().enumerate() {
                if current[k] == id {
                    continue;
                }
                let pos = current.iter().position(|&x| x == id).expect("permutation");
                let object = page
                    .objects_mut()
                    .remove_object_at_index(pos as PdfPageObjectIndex)?;
                page.objects_mut()
                    .insert_object_at_index(k as PdfPageObjectIndex, object)?;
                current.remove(pos);
                current.insert(k, id);
            }
        }
        page.regenerate_content()?;
        Ok(selected
            .iter()
            .map(|s| target.iter().position(|t| t == s).expect("permutation"))
            .collect())
    }

    pub fn delete_object(&mut self, page_index: usize, index: usize) -> Result<(), EngineError> {
        self.delete_objects(page_index, &[index])
    }

    /// Supprime plusieurs objets (doublons ignorés) ; rien n'est supprimé si un index est invalide.
    pub fn delete_objects(
        &mut self,
        page_index: usize,
        indices: &[usize],
    ) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        let indices = checked_indices(indices, page.objects().len())?;
        for i in indices.into_iter().rev() {
            page.objects_mut()
                .remove_object_at_index(i as PdfPageObjectIndex)?;
        }
        page.regenerate_content()?;
        Ok(())
    }

    /// Insère une image (PNG, JPEG…) de `width_pt` points de large, coin bas-gauche en (x, y).
    pub fn add_image(
        &mut self,
        page_index: usize,
        image_path: &Path,
        x: f32,
        y: f32,
        width_pt: f32,
    ) -> Result<(), EngineError> {
        self.add_image_with(page_index, image_path, x, y, width_pt, 1.0)
    }

    /// Comme [`Document::add_image`], avec une opacité (0 à 1). L'image est droite à l'écran.
    pub fn add_image_with(
        &mut self,
        page_index: usize,
        image_path: &Path,
        x: f32,
        y: f32,
        width_pt: f32,
        opacity: f32,
    ) -> Result<(), EngineError> {
        let img = crate::convert::load_image(image_path)?;
        self.insert_image(page_index, &img, x, y, width_pt, opacity)
    }

    /// Insère une image déjà décodée de `width_pt` points de large, coin bas-gauche en (x, y).
    /// Le canal alpha est conservé (PDFium écrit un masque `/SMask`) : une signature visuelle
    /// laisse voir le contenu situé dessous. Aucune valeur cryptographique : c'est une image.
    pub fn add_image_data(
        &mut self,
        page_index: usize,
        image: &image::DynamicImage,
        x: f32,
        y: f32,
        width_pt: f32,
    ) -> Result<(), EngineError> {
        self.insert_image(page_index, image, x, y, width_pt, 1.0)
    }

    fn insert_image(
        &mut self,
        page_index: usize,
        img: &image::DynamicImage,
        x: f32,
        y: f32,
        width_pt: f32,
        opacity: f32,
    ) -> Result<(), EngineError> {
        if !(width_pt.is_finite() && width_pt > 0.0 && x.is_finite() && y.is_finite())
            || img.width() == 0
            || img.height() == 0
        {
            return Err(EngineError::Other("invalid image size".into()));
        }
        let height_pt = width_pt * img.height() as f32 / img.width() as f32;
        let geo = self.page_geometry(page_index)?;
        let mut page = self.page_handle(page_index)?;
        let mut object = PdfPageImageObject::new_with_size(
            &self.inner,
            img,
            PdfPoints::new(width_pt),
            PdfPoints::new(height_pt),
        )?;
        let m = geo.upright_matrix(x, y);
        object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
        if opacity < 1.0 {
            object.set_fill_color(PdfColor::new(255, 255, 255, alpha_u8(opacity)))?;
        }
        page.objects_mut().add_image_object(object)?;
        page.regenerate_content()?;
        Ok(())
    }

    /// Remplace le contenu d'un objet image par un autre fichier. Le cadre est conservé ; si l'image
    /// est droite à l'écran, la nouvelle est ajustée (« contenir ») et centrée dans ce cadre.
    pub fn replace_image(
        &mut self,
        page_index: usize,
        index: usize,
        image_path: &Path,
    ) -> Result<(), EngineError> {
        let img = crate::convert::load_image(image_path)?;
        if img.width() == 0 || img.height() == 0 {
            return Err(EngineError::Other("invalid image size".into()));
        }
        let info = self.object_info(page_index, index)?;
        if info.kind != ObjectKind::Image {
            return Err(EngineError::Other("not an image object".into()));
        }
        {
            let mut page = self.page_handle(page_index)?;
            let mut object = page.objects().get(index as PdfPageObjectIndex)?;
            let Some(image_object) = object.as_image_object_mut() else {
                return Err(EngineError::Other("not an image object".into()));
            };
            image_object.set_image(&img)?;
            drop(object);
            page.regenerate_content()?;
        }
        if info.angle_deg.abs() < 0.5 {
            let b = info.bounds;
            let (w, h) = (b.right - b.left, b.top - b.bottom);
            let ratio = img.width() as f32 / img.height() as f32;
            let (nw, nh) = if w / h > ratio {
                (h * ratio, h)
            } else {
                (w, w / ratio)
            };
            let (cx, cy) = ((b.left + b.right) / 2.0, (b.bottom + b.top) / 2.0);
            self.resize_object(
                page_index,
                index,
                PointRect {
                    left: cx - nw / 2.0,
                    bottom: cy - nh / 2.0,
                    right: cx + nw / 2.0,
                    top: cy + nh / 2.0,
                },
            )?;
        }
        Ok(())
    }

    /// Ajoute une forme vectorielle. Pour `Line`, `rect` donne les deux extrémités (left,bottom)→(right,top).
    pub fn add_shape(
        &mut self,
        page_index: usize,
        kind: ShapeKind,
        rect: PointRect,
        style: &ShapeStyle,
    ) -> Result<(), EngineError> {
        let geo = self.page_geometry(page_index)?;
        let mut page = self.page_handle(page_index)?;
        let alpha = alpha_u8(style.opacity);
        let stroke = Some(color_a(style.stroke_rgb, alpha));
        let width = Some(PdfPoints::new(style.stroke_width.max(0.1)));
        let fill = style.fill_rgb.map(|c| color_a(c, alpha));
        let u = geo.rect_to_user(rect);
        let pdf_rect = PdfRect::new(
            PdfPoints::new(u.bottom),
            PdfPoints::new(u.left),
            PdfPoints::new(u.top),
            PdfPoints::new(u.right),
        );
        let objects = page.objects_mut();
        match kind {
            ShapeKind::Rectangle => {
                objects.create_path_object_rect(pdf_rect, stroke, width, fill)?;
            }
            ShapeKind::Ellipse => {
                objects.create_path_object_ellipse(pdf_rect, stroke, width, fill)?;
            }
            ShapeKind::Line => {
                let (x1, y1) = geo.to_user(rect.left, rect.bottom);
                let (x2, y2) = geo.to_user(rect.right, rect.top);
                objects.create_path_object_line(
                    PdfPoints::new(x1),
                    PdfPoints::new(y1),
                    PdfPoints::new(x2),
                    PdfPoints::new(y2),
                    color_a(style.stroke_rgb, alpha),
                    PdfPoints::new(style.stroke_width.max(0.1)),
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simule `reorder_target` et vérifie l'ordre obtenu.
    #[test]
    fn reorder_keeps_relative_order_and_moves_blocks_one_step() {
        assert_eq!(
            reorder_target(5, &[1, 3], ZOrder::Front),
            vec![0, 2, 4, 1, 3]
        );
        assert_eq!(
            reorder_target(5, &[1, 3], ZOrder::Back),
            vec![1, 3, 0, 2, 4]
        );
        assert_eq!(
            reorder_target(5, &[1, 2], ZOrder::Forward),
            vec![0, 3, 1, 2, 4]
        );
        assert_eq!(
            reorder_target(5, &[2, 3], ZOrder::Backward),
            vec![0, 2, 3, 1, 4]
        );
        // Au bord : rien ne bouge.
        assert_eq!(reorder_target(3, &[2], ZOrder::Forward), vec![0, 1, 2]);
        assert_eq!(reorder_target(3, &[0], ZOrder::Backward), vec![0, 1, 2]);
        // Objet déjà devant : il reste ; l'autre avance d'un cran.
        assert_eq!(
            reorder_target(4, &[0, 3], ZOrder::Forward),
            vec![1, 0, 2, 3]
        );
    }

    #[test]
    fn indices_are_checked_and_deduplicated() {
        assert_eq!(checked_indices(&[2, 0, 0], 3).unwrap(), vec![0, 2]);
        assert!(checked_indices(&[], 3).is_err());
        assert!(checked_indices(&[3], 3).is_err());
    }

    #[test]
    fn quad_hit_test_and_angles() {
        // Losange (carré pivoté de 45°) centré en (0,0), demi-diagonale 10.
        let q = [(0.0, 10.0), (10.0, 0.0), (0.0, -10.0), (-10.0, 0.0)];
        assert!(in_quad(&q, 0.0, 0.0));
        assert!(in_quad(&q, 4.0, 4.0));
        assert!(!in_quad(&q, 8.0, 8.0), "coin de la boîte englobante");
        assert_eq!(norm180(270.0), -90.0);
        assert_eq!(norm180(-180.0), 180.0);
        assert_eq!(norm180(30.0), 30.0);
    }
}
