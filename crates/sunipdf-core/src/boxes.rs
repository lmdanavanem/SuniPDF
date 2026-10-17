//! Cadres de page : recadrer (CropBox), redimensionner (MediaBox + CropBox, contenu conservé ou
//! mis à l'échelle), insérer des pages vierges. Marges et tailles en points d'affichage.

use crate::geometry::geometry_of;
use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;

/// Côté minimal d'une page recadrée ou redimensionnée (points).
const MIN_SIDE_PT: f32 = 18.0;
/// Côté maximal d'une page (200 pouces, limite de la spécification PDF).
const MAX_SIDE_PT: f32 = 14_400.0;

/// Marges à retirer (points, côtés de la page telle qu'affichée).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PageMargins {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    /// Contenu inchangé, ancré en haut à gauche de la page affichée.
    KeepContent,
    /// Contenu mis à l'échelle pour tenir dans la nouvelle page, centré.
    ScaleContent,
    /// Scale content and explicitly include annotation/link geometry.
    ScaleContentWithAnnotations,
}

fn pdf_rect(r: PointRect) -> PdfRect {
    PdfRect::new(
        PdfPoints::new(r.bottom),
        PdfPoints::new(r.left),
        PdfPoints::new(r.top),
        PdfPoints::new(r.right),
    )
}

fn check_size(w: f32, h: f32) -> Result<(), EngineError> {
    let ok = |v: f32| v.is_finite() && (MIN_SIDE_PT..=MAX_SIDE_PT).contains(&v);
    if ok(w) && ok(h) {
        Ok(())
    } else {
        Err(EngineError::Other("invalid page size".into()))
    }
}

/// Matrice (espace utilisateur) qui envoie le contenu de l'ancienne page affichée, mis à
/// l'échelle et centré, dans la nouvelle : `new.to_user ∘ T ∘ old.to_display`.
pub(crate) fn scale_matrix(
    old: &crate::PageGeometry,
    new: &crate::PageGeometry,
    w: f32,
    h: f32,
) -> [f32; 6] {
    let (ow, oh) = old.display_size();
    let s = (w / ow).min(h / oh);
    let (ox, oy) = ((w - s * ow) / 2.0, (h - s * oh) / 2.0);
    let m = |x: f32, y: f32| {
        let (dx, dy) = old.to_display(x, y);
        new.to_user(s * dx + ox, s * dy + oy)
    };
    let (e, f) = m(0.0, 0.0);
    let (a, b) = m(1.0, 0.0);
    let (c, d) = m(0.0, 1.0);
    [a - e, b - f, c - e, d - f, e, f]
}

impl Document<'_> {
    /// Recadre une page (masque les bords ; le contenu reste dans le fichier). Renvoie la nouvelle
    /// taille d'affichage.
    pub fn crop_page(
        &mut self,
        page_index: usize,
        m: PageMargins,
    ) -> Result<(f32, f32), EngineError> {
        let margins = [m.left, m.top, m.right, m.bottom];
        if margins.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(EngineError::Other("invalid margins".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let (w, h) = geo.display_size();
        let keep = PointRect {
            left: m.left,
            bottom: m.bottom,
            right: w - m.right,
            top: h - m.top,
        };
        if keep.right - keep.left < MIN_SIDE_PT || keep.top - keep.bottom < MIN_SIDE_PT {
            return Err(EngineError::Other("crop area too small".into()));
        }
        page.boundaries_mut()
            .set_crop(pdf_rect(geo.rect_to_user(keep)))?;
        drop(page);
        self.refresh_page_sizes()?;
        let size = self.page_size(page_index)?;
        Ok((size.width_pt, size.height_pt))
    }

    /// Rétablit le cadre d'origine (CropBox = MediaBox).
    pub fn reset_crop(&mut self, page_index: usize) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        let media = page.boundaries().media()?.bounds;
        page.boundaries_mut().set_crop(media)?;
        drop(page);
        self.refresh_page_sizes()
    }

    /// Donne à la page la taille d'affichage `width_pt` × `height_pt`.
    /// Annotation/link geometry follows only with `ScaleContentWithAnnotations`.
    pub fn resize_page(
        &mut self,
        page_index: usize,
        width_pt: f32,
        height_pt: f32,
        mode: ResizeMode,
    ) -> Result<(), EngineError> {
        check_size(width_pt, height_pt)?;
        if mode == ResizeMode::ScaleContentWithAnnotations {
            let old = self.page_geometry(page_index)?;
            let mut scratch = Document {
                inner: self
                    .pdfium
                    .load_pdf_from_byte_vec(self.snapshot()?, self.password())?,
                pdfium: self.pdfium,
                info: self.info.clone(),
                page_rotations: self.page_rotations.clone(),
                password: self.password.clone(),
                font_cache: Default::default(),
            };
            scratch.resize_page(page_index, width_pt, height_pt, ResizeMode::ScaleContent)?;
            let new = scratch.page_geometry(page_index)?;
            scratch.scale_annotation_geometry(
                page_index,
                scale_matrix(&old, &new, width_pt, height_pt),
            )?;
            self.inner = scratch.inner;
            self.info = scratch.info;
            self.page_rotations = scratch.page_rotations;
            self.font_cache.clear();
            return Ok(());
        }
        let mut page = self.page_handle(page_index)?;
        let old = geometry_of(&page)?;
        let (_, oh) = old.display_size();
        let new_box = match mode {
            ResizeMode::KeepContent => old.rect_to_user(PointRect {
                left: 0.0,
                bottom: oh - height_pt,
                right: width_pt,
                top: oh,
            }),
            ResizeMode::ScaleContent | ResizeMode::ScaleContentWithAnnotations => {
                let (uw, uh) =
                    if matches!(old.rotation, crate::Rotation::R90 | crate::Rotation::R270) {
                        (height_pt, width_pt)
                    } else {
                        (width_pt, height_pt)
                    };
                PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: uw,
                    top: uh,
                }
            }
        };
        // Les boîtes d'abord : PDFium rogne la transformation à la boîte courante.
        page.boundaries_mut().set_media(pdf_rect(new_box))?;
        page.boundaries_mut().set_crop(pdf_rect(new_box))?;
        if mode == ResizeMode::ScaleContent {
            let new = geometry_of(&page)?;
            let t = scale_matrix(&old, &new, width_pt, height_pt);
            page.transform_with_clip(t[0], t[1], t[2], t[3], t[4], t[5], pdf_rect(new_box))?;
        }
        drop(page);
        self.refresh_page_sizes()
    }

    /// Insère `count` pages blanches de `width_pt` × `height_pt` à l'index `at` (≤ nombre de pages).
    pub fn insert_blank_pages(
        &mut self,
        at: usize,
        count: usize,
        width_pt: f32,
        height_pt: f32,
    ) -> Result<(), EngineError> {
        if at > self.page_count() {
            return Err(EngineError::PageOutOfRange(at));
        }
        if count == 0 || count > 1000 {
            return Err(EngineError::Other("invalid page count".into()));
        }
        check_size(width_pt, height_pt)?;
        for k in 0..count {
            let index =
                PdfPageIndex::try_from(at + k).map_err(|_| EngineError::PageOutOfRange(at + k))?;
            self.inner.pages_mut().create_page_at_index(
                PdfPagePaperSize::new_custom(PdfPoints::new(width_pt), PdfPoints::new(height_pt)),
                index,
            )?;
        }
        self.refresh_page_sizes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageGeometry, Rotation};

    fn geo(w: f32, h: f32, rotation: Rotation) -> PageGeometry {
        PageGeometry {
            bbox: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: w,
                top: h,
            },
            rotation,
        }
    }

    #[test]
    fn scale_matrix_identity_half_and_rotated() {
        let a4 = geo(612.0, 792.0, Rotation::R0);
        assert_eq!(
            scale_matrix(&a4, &a4, 612.0, 792.0),
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        );
        let half = scale_matrix(&a4, &geo(306.0, 396.0, Rotation::R0), 306.0, 396.0);
        assert_eq!(half, [0.5, 0.0, 0.0, 0.5, 0.0, 0.0]);
        // Page pivotée : la matrice reste une simple échelle en espace utilisateur.
        let r = geo(612.0, 792.0, Rotation::R90);
        let t = scale_matrix(&r, &geo(306.0, 396.0, Rotation::R90), 396.0, 306.0);
        assert_eq!(t, [0.5, 0.0, 0.0, 0.5, 0.0, 0.0]);
    }
}
