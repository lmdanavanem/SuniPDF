//! Presse-papiers d'objets : un clip est un PDF d'une page en mémoire qui ne contient que les
//! objets copiés (polices, images et ressources comprises). Le collage importe cette page comme
//! XObject formulaire (un groupe) dans la page cible, y compris d'un autre document. Aucun état
//! PDFium n'est gardé entre la copie et le collage.

use crate::edit::checked_indices;
use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;

#[derive(Debug, Clone)]
pub struct ObjectClip {
    pdf: Vec<u8>,
    /// Nombre d'objets copiés.
    pub count: usize,
    /// Texte des objets texte copiés, une ligne par objet.
    pub text: String,
    /// Union des limites d'affichage dans la page source.
    pub bounds: PointRect,
}

impl Document<'_> {
    /// Copie des objets d'une page. Le document n'est pas modifié (ordre de dessin compris).
    pub fn copy_objects(
        &self,
        page_index: usize,
        indices: &[usize],
    ) -> Result<ObjectClip, EngineError> {
        let page = self.page_handle(page_index)?;
        let count = page.objects().len();
        drop(page);
        let keep = checked_indices(indices, count)?;
        let infos: Vec<_> = self
            .page_objects(page_index)?
            .into_iter()
            .filter(|o| keep.binary_search(&o.index).is_ok())
            .collect();
        let bounds = infos
            .iter()
            .skip(1)
            .fold(infos[0].bounds, |u, o| PointRect {
                left: u.left.min(o.bounds.left),
                bottom: u.bottom.min(o.bounds.bottom),
                right: u.right.max(o.bounds.right),
                top: u.top.max(o.bounds.top),
            });
        let text = infos
            .iter()
            .filter_map(|o| o.text.as_deref())
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let mut tmp = self.pdfium().create_new_pdf()?;
        tmp.pages_mut()
            .copy_page_from_document(&self.inner, page_index as PdfPageIndex, 0)?;
        {
            let mut p = tmp.pages().get(0)?;
            // La copie relit le flux de contenu : mêmes objets, même ordre, sinon on refuse.
            if p.objects().len() != count {
                return Err(EngineError::Other(
                    "page content could not be copied".into(),
                ));
            }
            for i in (0..count).rev() {
                if keep.binary_search(&i).is_err() {
                    p.objects_mut()
                        .remove_object_at_index(i as PdfPageObjectIndex)?;
                }
            }
            p.regenerate_content()?;
        }
        Ok(ObjectClip {
            pdf: tmp.save_to_bytes()?,
            count: keep.len(),
            text,
            bounds,
        })
    }
}

impl<'a> Document<'a> {
    /// Colle un clip en un seul objet groupe sur `page_index`, à la position d'origine décalée de
    /// (dx, dy) points d'affichage (recentré sur la page s'il en sortirait). Renvoie son index.
    pub fn paste_objects(
        &mut self,
        page_index: usize,
        clip: &ObjectClip,
        dx: f32,
        dy: f32,
    ) -> Result<usize, EngineError> {
        if !(dx.is_finite() && dy.is_finite()) {
            return Err(EngineError::Other("invalid offset".into()));
        }
        let geo = self.page_geometry(page_index)?;
        let src = self
            .pdfium()
            .load_pdf_from_byte_vec(clip.pdf.clone(), None)?;
        let mut source_page = src.pages().get(0)?;
        // `objects_mut()` porte la durée de vie du document (exigée par l'import inter-documents).
        let form = source_page
            .objects_mut()
            .copy_into_x_object_form_object(&mut self.inner)?;
        let mut page = self.page_handle(page_index)?;
        let mut object = page.objects_mut().add_object(form)?;
        // Le formulaire reprend la page source telle qu'affichée : le poser « droit ».
        let m = geo.upright_matrix(0.0, 0.0);
        object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
        let index = page.objects().len() - 1;
        drop(object);
        page.regenerate_content()?;
        drop(page);
        drop(source_page);
        drop(src);
        let now = self.object_info(page_index, index)?.bounds;
        let (w, h) = geo.display_size();
        let mut target = (clip.bounds.left + dx, clip.bounds.bottom + dy);
        let (bw, bh) = (now.right - now.left, now.top - now.bottom);
        if target.0 > w || target.1 > h || target.0 + bw < 0.0 || target.1 + bh < 0.0 {
            target = ((w - bw) / 2.0, (h - bh) / 2.0);
        }
        self.move_object(
            page_index,
            index,
            target.0 - now.left,
            target.1 - now.bottom,
        )?;
        Ok(index)
    }
}
