//! Édition du contenu des pages : objets texte, image et tracé (PDFium `FPDFPageObj_*`).
//! Chaque modification régénère le flux de contenu de la page (stratégie par défaut de pdfium-render).

use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Text,
    Image,
    Path,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub index: usize,
    pub kind: ObjectKind,
    pub bounds: PointRect,
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
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size_pt: 12.0,
            bold: false,
            italic: false,
            family: FontFamily::Helvetica,
            rgb: (0, 0, 0),
        }
    }
}

/// Couleurs et épaisseur à appliquer à un objet existant (`None` = inchangé).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectStyle {
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub stroke_rgb: Option<(u8, u8, u8)>,
    pub stroke_width: Option<f32>,
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
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            stroke_rgb: (0x25, 0x63, 0xEB),
            stroke_width: 2.0,
            fill_rgb: None,
        }
    }
}

fn color(rgb: (u8, u8, u8)) -> PdfColor {
    PdfColor::new(rgb.0, rgb.1, rgb.2, 255)
}

fn quad_to_rect(q: PdfQuadPoints) -> PointRect {
    PointRect {
        left: q.left().value,
        bottom: q.bottom().value,
        right: q.right().value,
        top: q.top().value,
    }
}

fn describe(index: usize, object: &PdfPageObject<'_>) -> ObjectInfo {
    let kind = match object.object_type() {
        PdfPageObjectType::Text => ObjectKind::Text,
        PdfPageObjectType::Image => ObjectKind::Image,
        PdfPageObjectType::Path => ObjectKind::Path,
        _ => ObjectKind::Other,
    };
    let bounds = object.bounds().map(quad_to_rect).unwrap_or(PointRect {
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
        top: 0.0,
    });
    let (text, font_size, font_name) = match object.as_text_object() {
        Some(t) => (
            Some(t.text()),
            t.scaled_font_size().value,
            Some(t.font().family()).filter(|n| !n.is_empty()),
        ),
        None => (None, 0.0, None),
    };
    let rgb = |c: PdfColor| (c.red(), c.green(), c.blue());
    ObjectInfo {
        index,
        kind,
        bounds,
        text,
        font_size,
        font_name,
        fill_rgb: object.fill_color().ok().map(rgb),
        stroke_rgb: object.stroke_color().ok().map(rgb),
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

impl Document<'_> {
    /// Tous les objets d'une page, dans l'ordre de dessin.
    pub fn page_objects(&self, page_index: usize) -> Result<Vec<ObjectInfo>, EngineError> {
        let page = self.page_handle(page_index)?;
        Ok(page
            .objects()
            .iter()
            .enumerate()
            .map(|(i, o)| describe(i, &o))
            .collect())
    }

    pub fn object_info(&self, page_index: usize, index: usize) -> Result<ObjectInfo, EngineError> {
        let page = self.page_handle(page_index)?;
        let object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        Ok(describe(index, &object))
    }

    /// Objet le plus en avant contenant le point (x, y) en points PDF.
    pub fn object_at(
        &self,
        page_index: usize,
        x: f32,
        y: f32,
    ) -> Result<Option<ObjectInfo>, EngineError> {
        Ok(self.page_objects(page_index)?.into_iter().rev().find(|o| {
            let b = o.bounds;
            x >= b.left - 1.0 && x <= b.right + 1.0 && y >= b.bottom - 1.0 && y <= b.top + 1.0
        }))
    }

    /// Ajoute un objet texte (police standard, WinAnsi : accents latins pris en charge) dont le coin
    /// bas-gauche de la ligne de base est en (x, y).
    pub fn add_text(
        &mut self,
        page_index: usize,
        x: f32,
        y: f32,
        text: &str,
        style: &TextStyle,
    ) -> Result<(), EngineError> {
        if text.trim().is_empty() {
            return Err(EngineError::Other("empty text".into()));
        }
        if !(style.size_pt.is_finite() && style.size_pt > 0.0) {
            return Err(EngineError::Other("invalid font size".into()));
        }
        let font = font_token(self.inner.fonts_mut(), style);
        let mut page = self.page_handle(page_index)?;
        let mut object =
            PdfPageTextObject::new(&self.inner, text, font, PdfPoints::new(style.size_pt))?;
        object.set_fill_color(color(style.rgb))?;
        object.translate(PdfPoints::new(x), PdfPoints::new(y))?;
        page.objects_mut().add_text_object(object)?;
        Ok(())
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
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
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
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        object.translate(PdfPoints::new(dx), PdfPoints::new(dy))?;
        drop(object);
        // Les objets modifiés en place ne régénèrent pas le flux de contenu : le faire explicitement.
        page.regenerate_content()?;
        Ok(())
    }

    /// Redimensionne un objet pour que ses limites deviennent `bounds` (mise à l'échelle de sa
    /// matrice ; le coin bas-gauche d'origine est ramené sur le nouveau coin bas-gauche).
    pub fn resize_object(
        &mut self,
        page_index: usize,
        index: usize,
        bounds: PointRect,
    ) -> Result<(), EngineError> {
        let current = self.object_info(page_index, index)?.bounds;
        let (old_w, old_h) = (current.right - current.left, current.top - current.bottom);
        let (new_w, new_h) = (bounds.right - bounds.left, bounds.top - bounds.bottom);
        if !(old_w > 0.0 && old_h > 0.0 && new_w > 0.0 && new_h > 0.0) {
            return Err(EngineError::Other("invalid object size".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        object.translate(
            PdfPoints::new(-current.left),
            PdfPoints::new(-current.bottom),
        )?;
        object.scale(new_w / old_w, new_h / old_h)?;
        object.translate(PdfPoints::new(bounds.left), PdfPoints::new(bounds.bottom))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Change les couleurs et l'épaisseur de trait d'un objet (texte : couleur de remplissage).
    pub fn set_object_style(
        &mut self,
        page_index: usize,
        index: usize,
        style: &ObjectStyle,
    ) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        if let Some(rgb) = style.fill_rgb {
            object.set_fill_color(color(rgb))?;
        }
        if let Some(rgb) = style.stroke_rgb {
            object.set_stroke_color(color(rgb))?;
        }
        if let Some(w) = style.stroke_width {
            object.set_stroke_width(PdfPoints::new(w.max(0.0)))?;
        }
        drop(object);
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
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        let Some(text_object) = object.as_text_object() else {
            return Err(EngineError::Other("not a text object".into()));
        };
        let (scaled, unscaled) = (
            text_object.scaled_font_size().value,
            text_object.unscaled_font_size().value,
        );
        let ratio = if unscaled > 0.0 && scaled > 0.0 {
            scaled / unscaled
        } else {
            1.0
        };
        text_object.set_unscaled_font_size(PdfPoints::new(size_pt / ratio))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Fait pivoter un objet autour du centre de ses limites (degrés, sens horaire).
    pub fn rotate_object(
        &mut self,
        page_index: usize,
        index: usize,
        degrees: f32,
    ) -> Result<(), EngineError> {
        if !degrees.is_finite() {
            return Err(EngineError::Other("invalid angle".into()));
        }
        let b = self.object_info(page_index, index)?.bounds;
        let (cx, cy) = ((b.left + b.right) / 2.0, (b.bottom + b.top) / 2.0);
        let mut page = self.page_handle(page_index)?;
        let mut object = page
            .objects()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        object.translate(PdfPoints::new(-cx), PdfPoints::new(-cy))?;
        object.rotate_clockwise_degrees(degrees)?;
        object.translate(PdfPoints::new(cx), PdfPoints::new(cy))?;
        drop(object);
        page.regenerate_content()?;
        Ok(())
    }

    /// Duplique un objet (copie sous forme de XObject, décalée de 12 pt) ; renvoie l'index de la copie.
    pub fn duplicate_object(
        &mut self,
        page_index: usize,
        index: usize,
    ) -> Result<usize, EngineError> {
        let mut page = self.page_handle(page_index)?;
        // `objects_mut()` rend un objet de durée de vie du document, sans emprunter la page.
        let mut object = page
            .objects_mut()
            .get(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
        let mut copy = object.copy_to_page(&mut page)?;
        copy.translate(PdfPoints::new(12.0), PdfPoints::new(-12.0))?;
        drop(copy);
        drop(object);
        page.regenerate_content()?;
        Ok(page.objects().len() - 1)
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
        if !front {
            // Un objet retiré puis réinséré garde son flux d'origine : l'ordre n'est respecté
            // qu'au sein d'un flux unique.
            self.merge_content_streams(page_index)?;
        }
        let mut page = self.page_handle(page_index)?;
        let count = page.objects().len();
        if index >= count {
            return Err(EngineError::Other(format!("object {index} not found")));
        }
        let object = page
            .objects_mut()
            .remove_object_at_index(index as PdfPageObjectIndex)?;
        let new_index = if front {
            page.objects_mut().add_object(object)?;
            count - 1
        } else {
            page.objects_mut().insert_object_at_index(0, object)?;
            0
        };
        page.regenerate_content()?;
        Ok(new_index)
    }

    pub fn delete_object(&mut self, page_index: usize, index: usize) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        page.objects_mut()
            .remove_object_at_index(index as PdfPageObjectIndex)
            .map_err(|_| EngineError::Other(format!("object {index} not found")))?;
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
        let img = image::open(image_path)
            .map_err(|e| EngineError::Other(format!("{}: {e}", image_path.display())))?;
        if !(width_pt.is_finite() && width_pt > 0.0) || img.width() == 0 {
            return Err(EngineError::Other("invalid image size".into()));
        }
        let height_pt = width_pt * img.height() as f32 / img.width() as f32;
        let mut page = self.page_handle(page_index)?;
        let mut object = PdfPageImageObject::new_with_size(
            &self.inner,
            &img,
            PdfPoints::new(width_pt),
            PdfPoints::new(height_pt),
        )?;
        object.translate(PdfPoints::new(x), PdfPoints::new(y))?;
        page.objects_mut().add_image_object(object)?;
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
        let mut page = self.page_handle(page_index)?;
        let stroke = Some(color(style.stroke_rgb));
        let width = Some(PdfPoints::new(style.stroke_width.max(0.1)));
        let fill = style.fill_rgb.map(color);
        let pdf_rect = PdfRect::new(
            PdfPoints::new(rect.bottom.min(rect.top)),
            PdfPoints::new(rect.left.min(rect.right)),
            PdfPoints::new(rect.bottom.max(rect.top)),
            PdfPoints::new(rect.left.max(rect.right)),
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
                objects.create_path_object_line(
                    PdfPoints::new(rect.left),
                    PdfPoints::new(rect.bottom),
                    PdfPoints::new(rect.right),
                    PdfPoints::new(rect.top),
                    color(style.stroke_rgb),
                    PdfPoints::new(style.stroke_width.max(0.1)),
                )?;
            }
        }
        Ok(())
    }
}
