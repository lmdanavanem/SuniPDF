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
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub size_pt: f32,
    pub bold: bool,
    pub rgb: (u8, u8, u8),
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size_pt: 12.0,
            bold: false,
            rgb: (0, 0, 0),
        }
    }
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
    let (text, font_size) = match object.as_text_object() {
        Some(t) => (Some(t.text()), t.scaled_font_size().value),
        None => (None, 0.0),
    };
    ObjectInfo {
        index,
        kind,
        bounds,
        text,
        font_size,
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

    /// Ajoute un objet texte (Helvetica, WinAnsi : accents latins pris en charge) dont le coin
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
        let font = if style.bold {
            self.inner.fonts_mut().helvetica_bold()
        } else {
            self.inner.fonts_mut().helvetica()
        };
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
