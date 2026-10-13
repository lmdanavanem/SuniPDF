//! Conversions : images → PDF, PDF → images, optimisation de taille.

use crate::{write_atomic, Document, Engine, EngineError};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// Résultat d'une optimisation : tailles avant/après en octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Optimized {
    pub before: u64,
    pub after: u64,
}

impl Engine {
    /// Crée un PDF d'une page par image (PNG, JPEG, BMP, GIF, TIFF, WebP). La taille de page
    /// correspond à l'image à `dpi` points par pouce. Renvoie le nombre de pages.
    pub fn images_to_pdf(
        &self,
        images: &[PathBuf],
        output: &Path,
        dpi: f32,
    ) -> Result<usize, EngineError> {
        if images.is_empty() {
            return Err(EngineError::Other("no input images".into()));
        }
        if !(dpi.is_finite() && dpi > 0.0) {
            return Err(EngineError::Other("dpi must be positive".into()));
        }
        let mut doc = self.pdfium.create_new_pdf()?;
        for path in images {
            let img = image::open(path)
                .map_err(|e| EngineError::Other(format!("{}: {e}", path.display())))?;
            let (w, h) = (
                img.width() as f32 * 72.0 / dpi,
                img.height() as f32 * 72.0 / dpi,
            );
            let mut page = doc
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::new_custom(
                    PdfPoints::new(w),
                    PdfPoints::new(h),
                ))?;
            let object = PdfPageImageObject::new_with_size(
                &doc,
                &img,
                PdfPoints::new(w),
                PdfPoints::new(h),
            )?;
            page.objects_mut().add_image_object(object)?;
        }
        let count = doc.pages().len() as usize;
        write_atomic(output, &doc.save_to_bytes()?)?;
        Ok(count)
    }
}

impl Document<'_> {
    /// Exporte toutes les pages en PNG (`<stem>-<n>.png`) dans `output_dir` à `dpi`.
    pub fn export_images(&self, output_dir: &Path, dpi: f32) -> Result<Vec<PathBuf>, EngineError> {
        if !(dpi.is_finite() && dpi > 0.0) {
            return Err(EngineError::Other("dpi must be positive".into()));
        }
        std::fs::create_dir_all(output_dir)?;
        let stem = self
            .info
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "page".into());
        let mut out = Vec::new();
        for page in 0..self.page_count() {
            let img = self.render_page(page, dpi / 72.0)?;
            let path = output_dir.join(format!("{stem}-{}.png", page + 1));
            let mut png = Vec::new();
            image::write_buffer_with_format(
                &mut std::io::Cursor::new(&mut png),
                &img.rgba,
                img.width,
                img.height,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .map_err(|e| EngineError::Other(e.to_string()))?;
            write_atomic(&path, &png)?;
            out.push(path);
        }
        Ok(out)
    }
}

/// Recompresse les flux et supprime les objets inutilisés (lopdf). Sans perte : le contenu
/// est inchangé ; le gain dépend du fichier (souvent faible sur des PDF déjà compressés).
pub fn optimize(input: &[u8], password: Option<&str>) -> Result<Vec<u8>, EngineError> {
    let mut doc =
        lopdf::Document::load_mem(input).map_err(|e| EngineError::Other(format!("lopdf: {e}")))?;
    if doc.is_encrypted() {
        let options = lopdf::LoadOptions::with_password(password.unwrap_or(""));
        doc = lopdf::Document::load_mem_with_options(input, options)
            .map_err(|_| EngineError::PasswordRequired)?;
        if doc.is_encrypted() {
            return Err(EngineError::PasswordRequired);
        }
        if let Ok(id) = doc
            .trailer
            .get(b"Encrypt")
            .and_then(lopdf::Object::as_reference)
        {
            doc.delete_object(id);
        }
        doc.trailer.remove(b"Encrypt");
    }
    doc.compress();
    doc.delete_zero_length_streams();
    doc.prune_objects();
    doc.renumber_objects();
    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| EngineError::Other(format!("lopdf save: {e}")))?;
    Ok(out)
}

/// Optimise `input` vers `output` (écriture atomique) et renvoie les tailles.
pub fn optimize_file(
    input: &Path,
    output: &Path,
    password: Option<&str>,
) -> Result<Optimized, EngineError> {
    let bytes = std::fs::read(input)?;
    let out = optimize(&bytes, password)?;
    write_atomic(output, &out)?;
    Ok(Optimized {
        before: bytes.len() as u64,
        after: out.len() as u64,
    })
}
