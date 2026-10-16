//! Lossy image compression without rebuilding page content, fields or annotations.

use crate::{security, EngineError};
use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageReader};
use lopdf::{Object, Stream};
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_PIXELS: u64 = 32_000_000;

#[derive(Debug, Clone, Copy)]
pub struct CompressionOptions {
    pub max_image_side: u32,
    pub jpeg_quality: u8,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            max_image_side: 2000,
            jpeg_quality: 80,
        }
    }
}

impl CompressionOptions {
    pub fn validate(self) -> Result<(), EngineError> {
        if !(128..=8192).contains(&self.max_image_side) || !(10..=100).contains(&self.jpeg_quality)
        {
            return Err(EngineError::Other(
                "image side must be 128..8192 pixels and JPEG quality 10..100".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CompressionReport {
    pub before: usize,
    pub after: usize,
    pub images_changed: usize,
    pub images_skipped: usize,
}

fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

fn is_image(stream: &Stream) -> bool {
    stream
        .dict
        .get(b"Subtype")
        .and_then(Object::as_name)
        .is_ok_and(|name| name == b"Image")
}

fn decode_image(stream: &Stream) -> Option<DynamicImage> {
    let dict = &stream.dict;
    // Keep masks, profiles and non-default decoding untouched rather than changing their appearance.
    if [
        b"SMask".as_slice(),
        b"Mask",
        b"Decode",
        b"SMaskInData",
        b"Alternates",
        b"OPI",
    ]
    .iter()
    .any(|key| dict.has(key))
        || dict
            .get(b"ImageMask")
            .and_then(Object::as_bool)
            .unwrap_or(false)
        || dict
            .get(b"BitsPerComponent")
            .and_then(Object::as_i64)
            .ok()?
            != 8
    {
        return None;
    }
    let channels = match dict.get(b"ColorSpace").and_then(Object::as_name).ok()? {
        b"DeviceRGB" => 3,
        b"DeviceGray" => 1,
        _ => return None,
    };
    let width = u32::try_from(dict.get(b"Width").and_then(Object::as_i64).ok()?).ok()?;
    let height = u32::try_from(dict.get(b"Height").and_then(Object::as_i64).ok()?).ok()?;
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    if pixels == 0 || pixels > MAX_PIXELS {
        return None;
    }
    let filters = stream.filters().unwrap_or_default();
    if filters == [b"DCTDecode".as_slice()] {
        if dict.has(b"DecodeParms") {
            return None;
        }
        let mut reader =
            ImageReader::with_format(Cursor::new(&stream.content), image::ImageFormat::Jpeg);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(width);
        limits.max_image_height = Some(height);
        limits.max_alloc = Some(MAX_PIXELS * 4);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().ok()?;
        if decoder.total_bytes() > MAX_PIXELS * 4
            || decoder.dimensions() != (width, height)
            || decoder.icc_profile().ok()?.is_some()
        {
            return None;
        }
        let image = DynamicImage::from_decoder(decoder).ok()?;
        if image.width() != width
            || image.height() != height
            || (channels == 3 && image.color() != image::ColorType::Rgb8)
            || (channels == 1 && image.color() != image::ColorType::L8)
        {
            return None;
        }
        return Some(image);
    }
    let expected = usize::try_from(pixels.checked_mul(channels)?).ok()?;
    let bytes = stream.decompressed_content_with_limit(expected).ok()?;
    if bytes.len() != expected {
        return None;
    }
    if channels == 3 {
        image::RgbImage::from_raw(width, height, bytes).map(DynamicImage::ImageRgb8)
    } else {
        image::GrayImage::from_raw(width, height, bytes).map(DynamicImage::ImageLuma8)
    }
}

/// Unsupported images are retained byte-for-byte. A result never exceeds the input size.
pub fn compress_images(
    input: &[u8],
    password: Option<&str>,
    options: CompressionOptions,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<(Vec<u8>, CompressionReport), EngineError> {
    options.validate()?;
    cancelled(cancel)?;
    let mut pdf = security::load(input, password)?;
    if pdf.encryption_state.as_ref().is_some_and(|state| {
        !state
            .permissions()
            .contains(lopdf::encryption::Permissions::MODIFIABLE)
    }) {
        return Err(EngineError::SecurityRestricted);
    }
    if pdf.objects.values().any(|object| {
        object
            .as_dict()
            .is_ok_and(|dict| dict.has(b"ByteRange") && dict.has(b"Contents"))
    }) {
        return Err(EngineError::SecurityRestricted);
    }
    let mut mask_objects = HashSet::new();
    let mut pending = Vec::new();
    for object in pdf.objects.values() {
        let dict = object
            .as_dict()
            .ok()
            .or_else(|| object.as_stream().ok().map(|stream| &stream.dict));
        if let Some(dict) = dict {
            for key in [b"SMask".as_slice(), b"Mask"] {
                if let Ok(mask) = dict.get(key) {
                    pending.push(mask);
                }
            }
        }
    }
    while let Some(object) = pending.pop() {
        match object {
            Object::Reference(id) if mask_objects.insert(*id) => {
                if let Ok(object) = pdf.get_object(*id) {
                    pending.push(object);
                }
            }
            Object::Array(items) => pending.extend(items),
            Object::Dictionary(dict) => pending.extend(dict.iter().map(|(_, value)| value)),
            Object::Stream(stream) => pending.extend(stream.dict.iter().map(|(_, value)| value)),
            _ => {}
        }
    }
    let ids: Vec<_> = pdf
        .objects
        .iter()
        .filter_map(|(&id, object)| {
            object
                .as_stream()
                .ok()
                .filter(|stream| is_image(stream))
                .map(|_| id)
        })
        .collect();
    let mut report = CompressionReport {
        before: input.len(),
        ..Default::default()
    };
    progress(0, ids.len());
    for (index, id) in ids.iter().enumerate() {
        cancelled(cancel)?;
        if mask_objects.contains(id) {
            report.images_skipped += 1;
            progress(index + 1, ids.len());
            continue;
        }
        let stream = pdf
            .get_object_mut(*id)
            .and_then(Object::as_stream_mut)
            .map_err(security::lopdf_err)?;
        let Some(mut image) = decode_image(stream) else {
            report.images_skipped += 1;
            progress(index + 1, ids.len());
            continue;
        };
        if image.width().max(image.height()) > options.max_image_side {
            image = image.resize(
                options.max_image_side,
                options.max_image_side,
                FilterType::Lanczos3,
            );
        }
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, options.jpeg_quality)
            .encode_image(&image)
            .map_err(|error| EngineError::Other(error.to_string()))?;
        cancelled(cancel)?;
        if jpeg.len() < stream.content.len() {
            stream.set_content(jpeg);
            stream
                .dict
                .set("Filter", Object::Name(b"DCTDecode".to_vec()));
            stream.dict.remove(b"DecodeParms");
            stream.dict.remove(b"DL");
            stream.dict.set("Width", image.width());
            stream.dict.set("Height", image.height());
            report.images_changed += 1;
        } else {
            report.images_skipped += 1;
        }
        progress(index + 1, ids.len());
    }
    cancelled(cancel)?;
    if report.images_changed == 0 {
        report.after = input.len();
        return Ok((input.to_vec(), report));
    }
    let output = security::save(pdf)?;
    cancelled(cancel)?;
    if output.len() >= input.len() {
        report.images_skipped += report.images_changed;
        report.images_changed = 0;
        report.after = input.len();
        Ok((input.to_vec(), report))
    } else {
        report.after = output.len();
        Ok((output, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Document};

    fn image_pdf(mask: bool) -> Vec<u8> {
        let mut pdf = Document::with_version("1.7");
        let pages = pdf.new_object_id();
        let mut dict = dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>512,"Height"=>512,
        "BitsPerComponent"=>8,"ColorSpace"=>"DeviceRGB"};
        if mask {
            dict.set(
                "Mask",
                vec![0.into(), 0.into(), 0.into(), 0.into(), 0.into(), 0.into()],
            );
        }
        let image = pdf.add_object(Stream::new(dict, vec![180; 512 * 512 * 3]));
        let contents = pdf.add_object(Stream::new(
            dictionary! {},
            b"q 400 0 0 400 0 0 cm /I Do Q".to_vec(),
        ));
        let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),400.into(),400.into()],
            "Resources"=>dictionary!{"XObject"=>dictionary!{"I"=>image}},"Contents"=>contents});
        pdf.objects.insert(
            pages,
            dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
        );
        let root = pdf.add_object(dictionary!{"Type"=>"Catalog","Pages"=>pages,"CustomMarker"=>Object::string_literal("keep")});
        pdf.trailer.set("Root", root);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn resamples_only_safe_images_and_preserves_catalog_and_protection() {
        let source = image_pdf(false);
        let options = CompressionOptions {
            max_image_side: 128,
            jpeg_quality: 70,
        };
        for encrypted in [false, true] {
            let input = if encrypted {
                security::protect(
                    &source,
                    None,
                    &crate::Protection {
                        user_password: "secret".into(),
                        owner_password: "owner".into(),
                        ..Default::default()
                    },
                )
                .unwrap()
            } else {
                source.clone()
            };
            let password = encrypted.then_some("secret");
            let (output, report) = compress_images(
                &input,
                password,
                options,
                &AtomicBool::new(false),
                |_, _| {},
            )
            .unwrap();
            assert_eq!(report.images_changed, 1);
            assert!(output.len() < input.len() / 2);
            let pdf = security::load(&output, password).unwrap();
            assert_eq!(
                pdf.catalog()
                    .unwrap()
                    .get(b"CustomMarker")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                b"keep"
            );
            let image = pdf
                .objects
                .values()
                .filter_map(|o| o.as_stream().ok())
                .find(|s| is_image(s))
                .unwrap();
            assert_eq!(image.dict.get(b"Width").unwrap().as_i64().unwrap(), 128);
            assert_eq!(
                Document::load_mem(&output).unwrap().is_encrypted(),
                encrypted
            );
        }
        let masked = image_pdf(true);
        let (output, report) =
            compress_images(&masked, None, options, &AtomicBool::new(false), |_, _| {}).unwrap();
        assert_eq!(output, masked);
        assert_eq!(report.images_skipped, 1);
    }

    #[test]
    fn cancellation_and_invalid_options_produce_no_result() {
        let input = image_pdf(false);
        assert!(matches!(
            compress_images(
                &input,
                None,
                CompressionOptions::default(),
                &AtomicBool::new(true),
                |_, _| {}
            ),
            Err(EngineError::Cancelled)
        ));
        let cancel = AtomicBool::new(false);
        assert!(matches!(
            compress_images(
                &input,
                None,
                CompressionOptions::default(),
                &cancel,
                |done, _| {
                    if done > 0 {
                        cancel.store(true, Ordering::Relaxed)
                    }
                }
            ),
            Err(EngineError::Cancelled)
        ));
        assert!(CompressionOptions {
            max_image_side: 0,
            jpeg_quality: 0
        }
        .validate()
        .is_err());
    }

    #[test]
    fn indirect_masks_keep_the_parent_and_mask_untouched_when_other_images_shrink() {
        for key in [b"Mask".as_slice(), b"SMask"] {
            let mut pdf = Document::load_mem(&image_pdf(false)).unwrap();
            let parent = pdf
                .objects
                .iter()
                .find(|(_, object)| object.as_stream().is_ok_and(is_image))
                .map(|(&id, _)| id)
                .unwrap();
            let mask = pdf.add_object(Stream::new(
                dictionary! {
                    "Type"=>"XObject", "Subtype"=>"Image", "Width"=>512, "Height"=>512,
                    "ColorSpace"=>"DeviceGray", "BitsPerComponent"=>8,
                },
                vec![128; 512 * 512],
            ));
            pdf.get_object_mut(parent)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .dict
                .set(key, mask);
            let mut unrelated = pdf.get_object(parent).unwrap().as_stream().unwrap().clone();
            unrelated.dict.remove(key);
            pdf.add_object(unrelated);
            let before_parent = pdf.get_object(parent).unwrap().clone();
            let before_mask = pdf.get_object(mask).unwrap().clone();
            let mut input = Vec::new();
            pdf.save_to(&mut input).unwrap();
            let (output, report) = compress_images(
                &input,
                None,
                CompressionOptions {
                    max_image_side: 128,
                    jpeg_quality: 70,
                },
                &AtomicBool::new(false),
                |_, _| {},
            )
            .unwrap();
            assert_eq!(report.images_changed, 1);
            assert_eq!(report.images_skipped, 2);
            let after = Document::load_mem(&output).unwrap();
            assert_eq!(after.get_object(parent).unwrap(), &before_parent);
            assert_eq!(after.get_object(mask).unwrap(), &before_mask);
        }
    }

    #[test]
    fn explicit_jpeg_color_transform_and_signed_or_restricted_documents_are_refused() {
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode_image(&DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                32,
                32,
                image::Rgb([20, 80, 200]),
            )))
            .unwrap();
        let stream = Stream::new(
            dictionary! { "Type"=>"XObject", "Subtype"=>"Image", "Width"=>32, "Height"=>32,
            "BitsPerComponent"=>8, "ColorSpace"=>"DeviceRGB", "Filter"=>"DCTDecode", "DecodeParms"=>dictionary! { "ColorTransform"=>0 } },
            jpeg,
        );
        assert!(decode_image(&stream).is_none());
        let input = image_pdf(false);
        let restricted = security::protect(
            &input,
            None,
            &crate::Protection {
                user_password: "user".into(),
                owner_password: "owner".into(),
                allow_modify: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(
            compress_images(
                &restricted,
                Some("user"),
                CompressionOptions::default(),
                &AtomicBool::new(false),
                |_, _| {}
            ),
            Err(EngineError::SecurityRestricted)
        ));
        let mut signed = Document::load_mem(&input).unwrap();
        signed.add_object(dictionary! { "Type"=>"Sig", "ByteRange"=>vec![0.into(),1.into(),2.into(),3.into()], "Contents"=>Object::string_literal("signature") });
        let mut signed_bytes = Vec::new();
        signed.save_to(&mut signed_bytes).unwrap();
        assert!(matches!(
            compress_images(
                &signed_bytes,
                None,
                CompressionOptions::default(),
                &AtomicBool::new(false),
                |_, _| {}
            ),
            Err(EngineError::SecurityRestricted)
        ));
    }

    #[test]
    fn embedded_jpeg_icc_profile_keeps_the_original_stream_and_pdf() {
        use image::ImageEncoder;
        let image = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            32,
            32,
            image::Rgb([20, 80, 200]),
        ));
        let dictionary = dictionary! { "Type"=>"XObject", "Subtype"=>"Image", "Width"=>32, "Height"=>32,
        "BitsPerComponent"=>8, "ColorSpace"=>"DeviceRGB", "Filter"=>"DCTDecode" };
        let mut plain = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut plain)
            .encode_image(&image)
            .unwrap();
        assert!(decode_image(&Stream::new(dictionary.clone(), plain)).is_some());

        let mut profile = vec![0; 128];
        profile[..4].copy_from_slice(&128u32.to_be_bytes());
        profile[16..20].copy_from_slice(b"RGB ");
        profile[20..24].copy_from_slice(b"XYZ ");
        profile[36..40].copy_from_slice(b"acsp");
        let mut jpeg = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut jpeg);
        encoder.set_icc_profile(profile.clone()).unwrap();
        encoder.encode_image(&image).unwrap();
        let mut decoder = ImageReader::with_format(Cursor::new(&jpeg), image::ImageFormat::Jpeg)
            .into_decoder()
            .unwrap();
        assert_eq!(decoder.icc_profile().unwrap(), Some(profile));
        drop(decoder);
        let stream = Stream::new(dictionary, jpeg);
        assert!(decode_image(&stream).is_none());
        let mut pdf = Document::load_mem(&image_pdf(false)).unwrap();
        let id = pdf
            .objects
            .iter()
            .find(|(_, object)| object.as_stream().is_ok_and(is_image))
            .map(|(&id, _)| id)
            .unwrap();
        pdf.objects.insert(id, Object::Stream(stream));
        let mut input = Vec::new();
        pdf.save_to(&mut input).unwrap();
        let (output, report) = compress_images(
            &input,
            None,
            CompressionOptions::default(),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(output, input);
        assert_eq!(report.images_changed, 0);
        assert_eq!(report.images_skipped, 1);
    }
}
