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
            let img = image::open(path).map_err(|e| match e {
                image::ImageError::IoError(io) => EngineError::Io(io),
                other => EngineError::Other(format!("{}: {other}", path.display())),
            })?;
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
    /// Sans `overwrite`, échoue avant toute écriture si l'un des fichiers existe déjà.
    pub fn export_images(
        &self,
        output_dir: &Path,
        dpi: f32,
        overwrite: bool,
    ) -> Result<Vec<PathBuf>, EngineError> {
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
        let targets: Vec<PathBuf> = (0..self.page_count())
            .map(|page| output_dir.join(format!("{stem}-{}.png", page + 1)))
            .collect();
        if !overwrite {
            crate::pages::ensure_absent(&targets)?;
        }
        let mut out = Vec::new();
        for (page, path) in targets.into_iter().enumerate() {
            let img = self.render_page(page, dpi / 72.0)?;
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
    // `security::load` / `save` : un document protégé le reste après optimisation.
    let mut doc = crate::security::load(input, password)?;
    doc.compress();
    doc.delete_zero_length_streams();
    doc.prune_objects();
    doc.renumber_objects();
    let out = crate::security::save(doc)?;
    Ok(out)
}

/// Optimise `input` vers `output` (écriture atomique) et renvoie les tailles.
pub fn optimize_file(
    input: &Path,
    output: &Path,
    password: Option<&str>,
) -> Result<Optimized, EngineError> {
    let bytes = std::fs::read(input)?;
    let mut out = optimize(&bytes, password)?;
    if out.len() >= bytes.len() {
        // Jamais de fichier plus gros : sans gain, la sortie est une copie de l'entrée.
        out = bytes.clone();
    }
    write_atomic(output, &out)?;
    Ok(Optimized {
        before: bytes.len() as u64,
        after: out.len() as u64,
    })
}
