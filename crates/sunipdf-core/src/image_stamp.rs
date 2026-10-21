//! Image-backed Stamp annotations with bounded decoding and atomic appearance creation.

use crate::annot_rich::{checked_rect, rich_base};
use crate::annot_write::{ap_stream, push_annotation};
use crate::appearance::{fmt_num, Ap};
use crate::security::lopdf_err;
use crate::{Document, EngineError, PointRect};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use lopdf::{dictionary, Object, Stream};
use std::io::{Cursor, Read};
use std::path::Path;

fn invalid() -> EngineError {
    EngineError::Other("image stamps require a visible image up to 64 MiB, 4096 pixels per side and 4 million pixels, with opacity between 0.01 and 1".into())
}

fn decode(path: &Path, opacity: f32) -> Result<RgbaImage, EngineError> {
    if !opacity.is_finite() || !(0.01..=1.0).contains(&opacity) {
        return Err(invalid());
    }
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    const MAX_BYTES: u64 = 64 * 1024 * 1024;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(32 * 1024 * 1024);
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(limits.clone());
    let err = |error| crate::convert::image_err(path, error);
    let mut decoder = reader.into_decoder().map_err(err)?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 4_000_000 {
        return Err(invalid());
    }
    limits.reserve(decoder.total_bytes()).map_err(err)?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(err)?;
    image.apply_orientation(orientation);
    let mut rgba = image.into_rgba8();
    let mut visible = false;
    for pixel in rgba.pixels_mut() {
        pixel[3] = (f32::from(pixel[3]) * opacity).round() as u8;
        visible |= pixel[3] != 0;
        if pixel[3] == 0 {
            pixel.0[..3].fill(0);
        }
    }
    if !visible {
        return Err(invalid());
    }
    Ok(rgba)
}

impl Document<'_> {
    /// Fits an image inside a displayed-page rectangle, retaining its aspect ratio and alpha.
    /// The image is embedded in a real Stamp annotation; failure preserves the current document.
    pub fn add_image_stamp_annotation(
        &mut self,
        page_index: usize,
        rect: PointRect,
        image: &Path,
        opacity: f32,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        let geo = self.page_geometry(page_index)?;
        let (w, h) = checked_rect(&geo, rect)?;
        let mut annotation = rich_base("Stamp", rect, &geo, author)?;
        let rgba = decode(image, opacity)?;
        let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
        let mut alpha = Vec::with_capacity(rgba.len() / 4);
        for pixel in rgba.pixels() {
            rgb.extend_from_slice(&pixel.0[..3]);
            alpha.push(pixel[3]);
        }
        self.edit_annotation_pdf(page_index, |pdf, page, geo| {
            let image_dict = dictionary! {
                "Type" => "XObject", "Subtype" => "Image",
                "Width" => rgba.width() as i64, "Height" => rgba.height() as i64,
                "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            };
            let mut image = Stream::new(image_dict, rgb);
            if alpha.iter().any(|&value| value != 255) {
                let mut mask = Stream::new(
                    dictionary! {
                        "Type" => "XObject", "Subtype" => "Image",
                        "Width" => rgba.width() as i64, "Height" => rgba.height() as i64,
                        "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                    },
                    alpha,
                );
                mask.compress().map_err(lopdf_err)?;
                image.dict.set("SMask", pdf.add_object(mask));
            }
            image.compress().map_err(lopdf_err)?;
            let image_id = pdf.add_object(image);
            let fit = (w / rgba.width() as f32).min(h / rgba.height() as f32);
            let (draw_w, draw_h) = (rgba.width() as f32 * fit, rgba.height() as f32 * fit);
            let values = [draw_w, draw_h, (w - draw_w) / 2.0, (h - draw_h) / 2.0].map(fmt_num);
            let mut ap = ap_stream(
                &Ap {
                    w,
                    h,
                    opacity: 1.0,
                    uses_font: false,
                    content: format!(
                        "q /GS gs {} 0 0 {} {} {} cm /Image Do Q\n",
                        values[0], values[1], values[2], values[3]
                    )
                    .into_bytes(),
                },
                geo,
            );
            ap.dict
                .get_mut(b"Resources")
                .map_err(lopdf_err)?
                .as_dict_mut()
                .map_err(lopdf_err)?
                .set("XObject", dictionary! { "Image" => image_id });
            annotation.set("Name", Object::Name(b"SuniPDFImage".to_vec()));
            annotation.set("Contents", lopdf::text_string("Custom image stamp"));
            push_annotation(pdf, page, annotation, Some(ap))
        })
    }
}
