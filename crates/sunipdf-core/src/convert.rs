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
/// chargeur des images insérées ou remplacées. Allocation bornée
/// à 512 Mio (`image::Limits::default`), dimensions libres.
pub(crate) fn load_image(path: &Path) -> Result<DynamicImage, EngineError> {
    decode(path, image::Limits::default())
}

/// Décodage borné d'un fichier choisi par l'utilisateur (frontière de confiance). `into_decoder`
/// applique les limites de dimensions mais, contrairement à `ImageReader::decode`, ne réserve pas
/// l'allocation de l'image décodée (`max_alloc`) : la réservation se fait ici, avant tout
/// décodage, pour qu'un PNG de 2 Mo annonçant 16 384² pixels en RGBA16 (2 Gio) soit refusé sans
/// rien allouer. L'orientation EXIF est appliquée.
pub(crate) fn decode(path: &Path, limits: image::Limits) -> Result<DynamicImage, EngineError> {
    let err = |e| image_err(path, e);
    let mut reader = reader(path)?;
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(err)?;
    let mut limits = limits;
    limits.reserve(decoder.total_bytes()).map_err(err)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(err)?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Matrice PDF qui affiche une image de `w` × `h` points (dimensions brutes, avant orientation)
/// avec son orientation EXIF appliquée, coin bas-gauche en (0, 0).
fn oriented_matrix(o: Orientation, w: f32, h: f32) -> [f32; 6] {
    match o {
        Orientation::NoTransforms => [w, 0.0, 0.0, h, 0.0, 0.0],
        Orientation::Rotate90 => [0.0, -w, h, 0.0, 0.0, w],
        Orientation::Rotate180 => [-w, 0.0, 0.0, -h, w, h],
        Orientation::Rotate270 => [0.0, w, -h, 0.0, h, 0.0],
        Orientation::FlipHorizontal => [-w, 0.0, 0.0, h, w, 0.0],
        Orientation::FlipVertical => [w, 0.0, 0.0, -h, 0.0, h],
        Orientation::Rotate90FlipH => [0.0, -w, -h, 0.0, h, w],
        Orientation::Rotate270FlipH => [0.0, w, h, 0.0, 0.0, 0.0],
    }
}

fn page_dimensions(width: u32, height: u32, dpi: f32) -> Result<(f32, f32), EngineError> {
    let (w, h) = (width as f32 * 72.0 / dpi, height as f32 * 72.0 / dpi);
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return Err(EngineError::Other(
            "image page size must be finite and positive".into(),
        ));
    }
    Ok((w, h))
}

fn reserve_raster(
    budget: &mut image::Limits,
    width: u32,
    height: u32,
    decoded_bytes: u64,
) -> Result<(), EngineError> {
    let rgba_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(EngineError::ImageBatchTooLarge)?;
    budget
        .reserve(decoded_bytes.max(rgba_bytes))
        .map_err(|_| EngineError::ImageBatchTooLarge)
}

fn preflight_images(images: &[PathBuf], dpi: f32) -> Result<(), EngineError> {
    let mut budget = image::Limits::default();
    for path in images {
        let reader = reader(path)?;
        let jpeg = reader.format() == Some(ImageFormat::Jpeg);
        let decoder = reader.into_decoder().map_err(|e| image_err(path, e))?;
        let (width, height) = decoder.dimensions();
        page_dimensions(width, height, dpi)?;
        if !jpeg {
            reserve_raster(&mut budget, width, height, decoder.total_bytes())?;
        }
    }
    Ok(())
}

/// JPEG incorporé tel quel (`FPDFImageObj_LoadJpegFileInline`) : ni décodage des pixels ni
/// recompression ; renvoie l'objet et la taille de page (points) après orientation.
fn jpeg_object<'a>(
    doc: &PdfDocument<'a>,
    path: &Path,
    w: f32,
    h: f32,
    orientation: Orientation,
) -> Result<(PdfPageImageObject<'a>, f32, f32), EngineError> {
    let m = oriented_matrix(orientation, w, h);
    let mut object = PdfPageImageObject::new_from_jpeg_reader(doc, std::fs::File::open(path)?)?;
    object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
    let swapped = matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    );
    Ok(if swapped {
        (object, h, w)
    } else {
        (object, w, h)
    })
}

impl Engine {
    /// Crée un PDF d'une page par image (PNG, JPEG, BMP, GIF, TIFF, WebP). La taille de page
    /// correspond à l'image à `dpi` points par pouce, orientation EXIF appliquée. Un JPEG est
    /// incorporé tel quel, sans recompression ; les autres formats sont décodés. Leur coût cumulé
    /// estimé (max des octets décodés et RGBA8) est limité à 512 Mio avant tout bitmap.
    /// Ce budget n'est pas un plafond de mémoire native, de copies temporaires ou du PDF sérialisé.
    /// Renvoie le nombre de pages.
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
        preflight_images(images, dpi)?;
        let mut budget = image::Limits::default();
        let mut doc = self.pdfium.create_new_pdf()?;
        for path in images {
            let reader = reader(path)?;
            let jpeg = reader.format() == Some(ImageFormat::Jpeg);
            let mut decoder = reader.into_decoder().map_err(|e| image_err(path, e))?;
            let (width, height) = decoder.dimensions();
            let (w, h) = page_dimensions(width, height, dpi)?;
            let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
            let (object, w, h) = if jpeg {
                // JPEG metadata parsing owns compressed bytes; release them before PDFium's copy.
                drop(decoder);
                jpeg_object(&doc, path, w, h, orientation)?
            } else {
                // Revalidate the decoder used for loading if a file changed after preflight.
                reserve_raster(&mut budget, width, height, decoder.total_bytes())?;
                let mut img =
                    DynamicImage::from_decoder(decoder).map_err(|e| image_err(path, e))?;
                img.apply_orientation(orientation);
                let (w, h) = page_dimensions(img.width(), img.height(), dpi)?;
                let object = PdfPageImageObject::new_with_size(
                    &doc,
                    &img,
                    PdfPoints::new(w),
                    PdfPoints::new(h),
                )?;
                (object, w, h)
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
    preserve_xml_metadata_compression(&mut doc);
    doc.compress();
    doc.delete_zero_length_streams();
    doc.prune_objects();
    doc.renumber_objects();
    let out = crate::security::save(doc)?;
    Ok(out)
}

fn preserve_xml_metadata_compression(doc: &mut lopdf::Document) {
    // PDF/A-1 forbids a Filter on catalog metadata. Preserve all Metadata/XML
    // streams conservatively, without trusting an XMP conformance declaration.
    // Existing filters and bytes remain untouched; this is not a repair pass.
    let metadata_ids: Vec<_> = doc
        .objects
        .iter()
        .filter_map(|(id, object)| {
            let stream = object.as_stream().ok()?;
            let is_metadata = stream
                .dict
                .get_deref(b"Type", doc)
                .and_then(lopdf::Object::as_name)
                .is_ok_and(|name| name == b"Metadata");
            let is_xml = stream
                .dict
                .get_deref(b"Subtype", doc)
                .and_then(lopdf::Object::as_name)
                .is_ok_and(|name| name == b"XML");
            (is_metadata || is_xml).then_some(*id)
        })
        .collect();
    for id in metadata_ids {
        if let Ok(stream) = doc
            .get_object_mut(id)
            .and_then(lopdf::Object::as_stream_mut)
        {
            stream.allows_compression = false;
        }
    }
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
    use lopdf::{dictionary, Object, Stream};

    fn optimization_metadata_fixture(compressed_metadata: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut doc = lopdf::Document::with_version("1.4");
        let pages = doc.new_object_id();
        let content = b"q 0 0 10 10 re f Q\n".repeat(512);
        let contents = doc.add_object(Stream::new(dictionary! {}, content.clone()));
        let xml = format!(
            "<x:xmpmeta xmlns:x='adobe:ns:meta/'><rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'><rdf:Description xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/' pdfaid:part='1' pdfaid:conformance='B'/></rdf:RDF>{}</x:xmpmeta>",
            " ".repeat(4096)
        )
        .into_bytes();
        let indirect_type = doc.add_object(Object::Name(b"Metadata".to_vec()));
        let indirect_subtype = doc.add_object(Object::Name(b"XML".to_vec()));
        let mut metadata_ids = Vec::new();
        for dict in [
            dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
            dictionary! { "Type" => "Metadata" },
            dictionary! { "Subtype" => "XML" },
            dictionary! { "Type" => indirect_type, "Subtype" => indirect_subtype },
        ] {
            let mut stream = Stream::new(dict, xml.clone());
            if compressed_metadata {
                stream.compress().unwrap();
                assert!(stream.dict.has(b"Filter"));
            }
            metadata_ids.push(doc.add_object(stream));
        }
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {}, "Contents" => contents,
            "Metadata" => metadata_ids[1],
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 1, "Kids" => vec![page.into()],
            }),
        );
        let root = doc.add_object(dictionary! {
            "Type" => "Catalog", "Pages" => pages, "Metadata" => metadata_ids[0],
            "TestMetadata" => metadata_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
        });
        doc.trailer.set("Root", root);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        (bytes, xml, content)
    }

    #[test]
    fn optimize_preserves_xml_metadata_without_filters_and_compresses_page_content() {
        let (source, xml, content) = optimization_metadata_fixture(false);
        let original = source.clone();
        let output = optimize(&source, None).unwrap();
        assert_eq!(source, original);
        assert!(output.len() < source.len());
        let doc = lopdf::Document::load_mem(&output).unwrap();
        let catalog = doc.catalog().unwrap();
        for reference in catalog.get(b"TestMetadata").unwrap().as_array().unwrap() {
            let stream = doc.dereference(reference).unwrap().1.as_stream().unwrap();
            assert!(!stream.dict.has(b"Filter"));
            assert_eq!(stream.content, xml);
        }
        let page = doc
            .get_dictionary(*doc.get_pages().get(&1).unwrap())
            .unwrap();
        let stream = page
            .get_deref(b"Contents", &doc)
            .unwrap()
            .as_stream()
            .unwrap();
        assert_eq!(
            stream.dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"FlateDecode"
        );
        assert_eq!(
            stream
                .decompressed_content_with_limit(content.len())
                .unwrap(),
            content
        );
    }

    #[test]
    fn optimize_keeps_existing_metadata_filters_and_does_not_claim_to_repair_them() {
        let (source, xml, _) = optimization_metadata_fixture(true);
        let before = lopdf::Document::load_mem(&source).unwrap();
        let previous = before
            .catalog()
            .unwrap()
            .get_deref(b"Metadata", &before)
            .unwrap()
            .as_stream()
            .unwrap();
        let output = optimize(&source, None).unwrap();
        let after = lopdf::Document::load_mem(&output).unwrap();
        let metadata = after
            .catalog()
            .unwrap()
            .get_deref(b"Metadata", &after)
            .unwrap()
            .as_stream()
            .unwrap();
        assert_eq!(
            metadata.dict.get(b"Filter").unwrap(),
            previous.dict.get(b"Filter").unwrap()
        );
        assert_eq!(metadata.content, previous.content);
        assert_eq!(
            metadata.decompressed_content_with_limit(xml.len()).unwrap(),
            xml
        );
    }

    #[test]
    fn optimize_file_preserves_source_and_reuses_bytes_when_no_smaller_output_exists() {
        let (source, _, _) = optimization_metadata_fixture(false);
        let stable = optimize(&source, None).unwrap();
        assert!(optimize(&stable, None).unwrap().len() >= stable.len());
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.pdf");
        let output = dir.path().join("optimized.pdf");
        std::fs::write(&input, &stable).unwrap();
        let result = optimize_file(&input, &output, None).unwrap();
        assert_eq!(result.before, stable.len() as u64);
        assert_eq!(result.after, result.before);
        assert_eq!(std::fs::read(&input).unwrap(), stable);
        assert_eq!(std::fs::read(&output).unwrap(), stable);
    }

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
            (Orientation::Rotate90FlipH, h, w),
            (Orientation::Rotate270FlipH, h, w),
        ] {
            let m = oriented_matrix(o, w, h);
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
        let m = oriented_matrix(Orientation::Rotate90, w, h);
        assert_eq!(apply(m, 0.0, 1.0), (h, w));
        assert_eq!(
            apply(oriented_matrix(Orientation::Rotate90FlipH, w, h), 0.0, 1.0),
            (0.0, w)
        );
        assert_eq!(
            apply(oriented_matrix(Orientation::Rotate270FlipH, w, h), 0.0, 1.0),
            (h, 0.0)
        );
    }

    #[test]
    fn raster_budget_counts_normalized_pixels_decoded_bytes_and_cumulative_limits() {
        let mut budget = image::Limits::default();
        assert_eq!(budget.max_alloc, Some(512 * 1024 * 1024));
        reserve_raster(&mut budget, 4096, 4096, 4096 * 4096).unwrap();
        assert_eq!(budget.max_alloc, Some(448 * 1024 * 1024));
        reserve_raster(&mut budget, 4096, 4096, 4096 * 4096 * 8).unwrap();
        assert_eq!(budget.max_alloc, Some(320 * 1024 * 1024));
        reserve_raster(&mut budget, 1, 1, 320 * 1024 * 1024).unwrap();
        assert_eq!(budget.max_alloc, Some(0));
        assert!(matches!(
            reserve_raster(&mut budget, 1, 1, 1),
            Err(EngineError::ImageBatchTooLarge)
        ));
        assert_eq!(budget.max_alloc, Some(0));
        assert!(matches!(
            reserve_raster(&mut image::Limits::default(), u32::MAX, u32::MAX, 0),
            Err(EngineError::ImageBatchTooLarge)
        ));
        assert!(matches!(
            reserve_raster(&mut image::Limits::default(), 1, 1, u64::MAX),
            Err(EngineError::ImageBatchTooLarge)
        ));
    }

    #[test]
    fn image_page_geometry_refuses_zero_and_overflow_without_losing_normal_dpi() {
        assert_eq!(page_dimensions(300, 150, 150.0).unwrap(), (144.0, 72.0));
        for (w, h, dpi) in [
            (0, 1, 72.0),
            (1, 0, 72.0),
            (1, 1, f32::from_bits(1)),
            (1, 1, f32::NAN),
        ] {
            assert!(page_dimensions(w, h, dpi).is_err());
        }
    }
}
