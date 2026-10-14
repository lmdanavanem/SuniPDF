//! Conversions : images → PDF, PDF → images, optimisation de taille.

use crate::{write_atomic, Document, Engine, EngineError};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// Résultat d'une optimisation : tailles avant/après en octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Optimized {
    pub before: u64,
    pub after: u64,
}

pub(crate) fn image_err(path: &Path, e: image::ImageError) -> EngineError {
    match e {
        image::ImageError::IoError(io) => EngineError::Io(io),
        other => EngineError::Other(format!("{}: {other}", path.display())),
    }
}

/// Lecteur au format deviné d'après le contenu (l'extension peut mentir).
fn reader(path: &Path) -> Result<ImageReader<std::io::BufReader<std::fs::File>>, EngineError> {
    ImageReader::open(path)?
        .with_guessed_format()
        .map_err(EngineError::Io)
}

/// Décode une image en appliquant son orientation EXIF (photos de téléphone en portrait) :
/// chargeur unique des images insérées, remplacées ou converties en pages.
pub(crate) fn load_image(path: &Path) -> Result<DynamicImage, EngineError> {
    let mut decoder = reader(path)?
        .into_decoder()
        .map_err(|e| image_err(path, e))?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| image_err(path, e))?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Matrice PDF qui affiche une image de `w` × `h` points (dimensions brutes, avant orientation)
/// avec son orientation EXIF appliquée, coin bas-gauche en (0, 0) ; `None` pour les deux
/// combinaisons rotation + miroir (passage par le décodage).
fn oriented_matrix(o: Orientation, w: f32, h: f32) -> Option<[f32; 6]> {
    Some(match o {
        Orientation::NoTransforms => [w, 0.0, 0.0, h, 0.0, 0.0],
        Orientation::Rotate90 => [0.0, -w, h, 0.0, 0.0, w],
        Orientation::Rotate180 => [-w, 0.0, 0.0, -h, w, h],
        Orientation::Rotate270 => [0.0, w, -h, 0.0, h, 0.0],
        Orientation::FlipHorizontal => [-w, 0.0, 0.0, h, w, 0.0],
        Orientation::FlipVertical => [w, 0.0, 0.0, -h, 0.0, h],
        Orientation::Rotate90FlipH | Orientation::Rotate270FlipH => return None,
    })
}

/// JPEG incorporé tel quel (`FPDFImageObj_LoadJpegFileInline`) : ni décodage en mémoire ni
/// recompression ; renvoie l'objet et la taille de page (points) après orientation.
fn jpeg_object<'a>(
    doc: &PdfDocument<'a>,
    path: &Path,
    dpi: f32,
) -> Result<Option<(PdfPageImageObject<'a>, f32, f32)>, EngineError> {
    let mut decoder = reader(path)?;
    if decoder.format() != Some(ImageFormat::Jpeg) {
        return Ok(None);
    }
    let (pw, ph) = decoder.into_dimensions().map_err(|e| image_err(path, e))?;
    decoder = reader(path)?;
    let orientation = decoder
        .into_decoder()
        .map_err(|e| image_err(path, e))?
        .orientation()
        .unwrap_or(Orientation::NoTransforms);
    let (w, h) = (pw as f32 * 72.0 / dpi, ph as f32 * 72.0 / dpi);
    let Some(m) = oriented_matrix(orientation, w, h) else {
        return Ok(None);
    };
    let mut object = PdfPageImageObject::new_from_jpeg_reader(doc, std::fs::File::open(path)?)?;
    object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
    let swapped = matches!(orientation, Orientation::Rotate90 | Orientation::Rotate270);
    Ok(Some(if swapped {
        (object, h, w)
    } else {
        (object, w, h)
    }))
}

impl Engine {
    /// Crée un PDF d'une page par image (PNG, JPEG, BMP, GIF, TIFF, WebP). La taille de page
    /// correspond à l'image à `dpi` points par pouce, orientation EXIF appliquée. Un JPEG est
    /// incorporé tel quel (fichier de la taille des JPEG d'entrée, mémoire constante) ; les
    /// autres formats sont décodés. Renvoie le nombre de pages.
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
            let (object, w, h) = match jpeg_object(&doc, path, dpi)? {
                Some(ready) => ready,
                None => {
                    let img = load_image(path)?;
                    let (w, h) = (
                        img.width() as f32 * 72.0 / dpi,
                        img.height() as f32 * 72.0 / dpi,
                    );
                    let object = PdfPageImageObject::new_with_size(
                        &doc,
                        &img,
                        PdfPoints::new(w),
                        PdfPoints::new(h),
                    )?;
                    (object, w, h)
                }
            };
            let mut page = doc
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::new_custom(
                    PdfPoints::new(w),
                    PdfPoints::new(h),
                ))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque orientation envoie les coins de l'image brute (unité) sur ceux de la boîte affichée.
    #[test]
    fn oriented_matrices_map_the_unit_square_onto_the_display_box() {
        let (w, h) = (300.0, 200.0);
        let apply =
            |m: [f32; 6], x: f32, y: f32| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        for (o, bw, bh) in [
            (Orientation::NoTransforms, w, h),
            (Orientation::Rotate90, h, w),
            (Orientation::Rotate180, w, h),
            (Orientation::Rotate270, h, w),
            (Orientation::FlipHorizontal, w, h),
            (Orientation::FlipVertical, w, h),
        ] {
            let m = oriented_matrix(o, w, h).unwrap();
            let mut seen: Vec<(i32, i32)> = corners
                .iter()
                .map(|&(x, y)| apply(m, x, y))
                .map(|(x, y)| (x.round() as i32, y.round() as i32))
                .collect();
            seen.sort_unstable();
            let (bw, bh) = (bw as i32, bh as i32);
            assert_eq!(seen, vec![(0, 0), (0, bh), (bw, 0), (bw, bh)], "{o:?}");
        }
        // Rotation 90° horaire (EXIF 6) : le coin haut-gauche brut (0, 1) passe en haut à droite.
        let m = oriented_matrix(Orientation::Rotate90, w, h).unwrap();
        assert_eq!(apply(m, 0.0, 1.0), (h, w));
        assert!(oriented_matrix(Orientation::Rotate90FlipH, w, h).is_none());
    }
}
