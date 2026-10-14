//! Géométrie de page : passage entre l'espace **utilisateur** PDF (celui des objets, du texte et des
//! annotations) et l'espace **d'affichage** (la page telle qu'on la voit : après `/Rotate`, origine
//! au coin bas-gauche de la boîte effective MediaBox ∩ CropBox, Y vers le haut, en points).
//!
//! Toute coordonnée publique du moteur (`ObjectInfo`, `TextSelection`, `TextMatch`, entrées des
//! fonctions d'édition) est en espace d'affichage : c'est l'espace de `Document::page_size` et du
//! rendu. Les conversions vivent ici seulement. Sur une page sans rotation dont la boîte part de
//! (0, 0), elles sont l'identité.
//!
//! Matrices reprises de PDFium (`CPDF_Page::UpdateDimensions`) : R0 `(1,0,0,1,-l,-b)`,
//! R90 `(0,-1,1,0,-b,r)`, R180 `(-1,0,0,-1,r,t)`, R270 `(0,1,-1,0,t,-l)`.

use crate::{Document, EngineError, PointRect, Rotation};
use pdfium_render::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry {
    /// Boîte effective (MediaBox ∩ CropBox), espace utilisateur.
    pub bbox: PointRect,
    /// Rotation d'affichage `/Rotate` (sens horaire).
    pub rotation: Rotation,
}

impl PageGeometry {
    fn swapped(&self) -> bool {
        matches!(self.rotation, Rotation::R90 | Rotation::R270)
    }

    /// Taille de la page affichée (points).
    pub fn display_size(&self) -> (f32, f32) {
        let (w, h) = (
            self.bbox.right - self.bbox.left,
            self.bbox.top - self.bbox.bottom,
        );
        if self.swapped() {
            (h, w)
        } else {
            (w, h)
        }
    }

    pub fn to_display(&self, x: f32, y: f32) -> (f32, f32) {
        let b = &self.bbox;
        match self.rotation {
            Rotation::R0 => (x - b.left, y - b.bottom),
            Rotation::R90 => (y - b.bottom, b.right - x),
            Rotation::R180 => (b.right - x, b.top - y),
            Rotation::R270 => (b.top - y, x - b.left),
        }
    }

    pub fn to_user(&self, x: f32, y: f32) -> (f32, f32) {
        let b = &self.bbox;
        match self.rotation {
            Rotation::R0 => (x + b.left, y + b.bottom),
            Rotation::R90 => (b.right - y, x + b.bottom),
            Rotation::R180 => (b.right - x, b.top - y),
            Rotation::R270 => (y + b.left, b.top - x),
        }
    }

    /// Partie linéaire de [`Self::to_user`] (déplacements).
    pub fn vec_to_user(&self, dx: f32, dy: f32) -> (f32, f32) {
        match self.rotation {
            Rotation::R0 => (dx, dy),
            Rotation::R90 => (-dy, dx),
            Rotation::R180 => (-dx, -dy),
            Rotation::R270 => (dy, -dx),
        }
    }

    /// Partie linéaire de [`Self::to_display`].
    pub fn vec_to_display(&self, dx: f32, dy: f32) -> (f32, f32) {
        match self.rotation {
            Rotation::R0 => (dx, dy),
            Rotation::R90 => (dy, -dx),
            Rotation::R180 => (-dx, -dy),
            Rotation::R270 => (-dy, dx),
        }
    }

    pub fn rect_to_display(&self, r: PointRect) -> PointRect {
        let a = self.to_display(r.left, r.bottom);
        let b = self.to_display(r.right, r.top);
        normalize(a, b)
    }

    pub fn rect_to_user(&self, r: PointRect) -> PointRect {
        let a = self.to_user(r.left, r.bottom);
        let b = self.to_user(r.right, r.top);
        normalize(a, b)
    }

    /// Matrice PDF `[a, b, c, d, e, f]` qui place l'origine locale d'un objet au point d'affichage
    /// (x, y), axes alignés sur l'écran : un texte ainsi posé est droit à l'écran sur toute page.
    pub fn upright_matrix(&self, x: f32, y: f32) -> [f32; 6] {
        let (a, b) = self.vec_to_user(1.0, 0.0);
        let (c, d) = self.vec_to_user(0.0, 1.0);
        let (e, f) = self.to_user(x, y);
        [a, b, c, d, e, f]
    }

    pub fn rotation_degrees(&self) -> f32 {
        self.rotation.degrees() as f32
    }
}

fn normalize(a: (f32, f32), b: (f32, f32)) -> PointRect {
    PointRect {
        left: a.0.min(b.0),
        bottom: a.1.min(b.1),
        right: a.0.max(b.0),
        top: a.1.max(b.1),
    }
}

/// Géométrie d'une page ouverte. `bounding()` appelle `FPDF_GetPageBoundingBox`, qui renvoie la
/// boîte effective MediaBox ∩ CropBox (et non l'enveloppe du contenu, contrairement à la doc
/// de pdfium-render).
pub(crate) fn geometry_of(page: &PdfPage<'_>) -> Result<PageGeometry, EngineError> {
    let b = page.boundaries().bounding()?.bounds;
    Ok(PageGeometry {
        bbox: PointRect {
            left: b.left().value,
            bottom: b.bottom().value,
            right: b.right().value,
            top: b.top().value,
        },
        rotation: Rotation::from_pdfium(page.rotation()?),
    })
}

impl Document<'_> {
    pub fn page_geometry(&self, page_index: usize) -> Result<PageGeometry, EngineError> {
        geometry_of(&self.page_handle(page_index)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geo(rotation: Rotation) -> PageGeometry {
        PageGeometry {
            bbox: PointRect {
                left: 100.0,
                bottom: 50.0,
                right: 712.0,
                top: 842.0,
            },
            rotation,
        }
    }

    const ALL: [Rotation; 4] = [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270];

    #[test]
    fn round_trips_and_corners() {
        for r in ALL {
            let g = geo(r);
            for (x, y) in [(100.0, 50.0), (712.0, 842.0), (300.0, 400.0)] {
                let (dx, dy) = g.to_display(x, y);
                assert_eq!(g.to_user(dx, dy), (x, y), "{r:?}");
                let v = g.vec_to_display(3.0, -7.0);
                assert_eq!(g.vec_to_user(v.0, v.1), (3.0, -7.0), "{r:?}");
            }
            // Les quatre coins de la boîte couvrent exactement la taille d'affichage.
            let (w, h) = g.display_size();
            let d = g.rect_to_display(g.bbox);
            assert_eq!(
                (d.left, d.bottom, d.right, d.top),
                (0.0, 0.0, w, h),
                "{r:?}"
            );
        }
        let (w, h) = (612.0, 792.0);
        // R90 : le coin haut-gauche utilisateur passe en haut à droite de l'écran.
        assert_eq!(geo(Rotation::R90).to_display(100.0, 842.0), (h, w));
        // R270 : le coin bas-gauche utilisateur passe en bas à droite.
        assert_eq!(geo(Rotation::R270).to_display(100.0, 50.0), (h, 0.0));
        assert_eq!(geo(Rotation::R90).display_size(), (h, w));
    }

    #[test]
    fn upright_matrix_maps_local_axes_to_screen_axes() {
        for r in ALL {
            let g = geo(r);
            let m = g.upright_matrix(10.0, 20.0);
            let apply = |u: f32, v: f32| (m[0] * u + m[2] * v + m[4], m[1] * u + m[3] * v + m[5]);
            let (x, y) = apply(5.0, 0.0);
            assert_eq!(g.to_display(x, y), (15.0, 20.0), "{r:?}");
            let (x, y) = apply(0.0, 5.0);
            assert_eq!(g.to_display(x, y), (10.0, 25.0), "{r:?}");
        }
    }
}
