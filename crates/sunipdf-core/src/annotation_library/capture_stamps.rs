//! Strict decoding of saved native stamps; no PDF resource is imported into a library.

use super::{
    capture_dictionary, capture_object, digest_hex, finite, invalid, CaptureBudget, EmbeddedPng,
    LibraryPrimitive, LibraryStamp,
};
use crate::geometry::PageGeometry;
use crate::{EngineError, PointRect};
use lopdf::{dictionary, Dictionary, Object, Stream};
use std::io::Cursor;

#[cfg(test)]
mod tests;

fn unsupported() -> EngineError {
    invalid("only unmodified native standard stamps and bounded native image stamps can be captured; custom appearances, resources, actions and review history are not supported")
}

fn resolve<'a>(pdf: &'a lopdf::Document, mut value: &'a Object) -> Result<&'a Object, EngineError> {
    for _ in 0..16 {
        match value {
            Object::Reference(id) => value = pdf.objects.get(id).ok_or_else(unsupported)?,
            _ => return Ok(value),
        }
    }
    Err(unsupported())
}

fn field<'a>(
    pdf: &'a lopdf::Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Result<&'a Object, EngineError> {
    resolve(pdf, dict.get(key).map_err(|_| unsupported())?)
}

/// Native streams have either no filter or a single FlateDecode, without predictor parameters.
/// Reject every other encoding before canonical comparison, including unused metadata keys.
fn native_stream<'a>(
    pdf: &'a lopdf::Document,
    value: &'a Object,
    keys: &[&[u8]],
) -> Result<&'a Stream, EngineError> {
    let stream = resolve(pdf, value)?
        .as_stream()
        .map_err(|_| unsupported())?;
    if stream.dict.iter().any(|(key, _)| {
        !keys.contains(&key.as_slice()) && !matches!(key.as_slice(), b"Length" | b"Filter")
    }) || stream
        .dict
        .get(b"Filter")
        .is_ok_and(|filter| !matches!(filter, Object::Name(name) if name == b"FlateDecode"))
        || stream
            .dict
            .get(b"Length")
            .is_ok_and(|length| length.as_i64().ok() != i64::try_from(stream.content.len()).ok())
    {
        return Err(unsupported());
    }
    Ok(stream)
}

fn decoded(stream: &Stream, budget: &mut CaptureBudget) -> Result<Vec<u8>, EngineError> {
    budget.bytes(stream.content.len())?;
    if stream.dict.has(b"Filter") {
        let bytes = stream
            .decompressed_content_with_limit(budget.bytes)
            .map_err(crate::security::lopdf_err)?;
        budget.bytes(bytes.len())?;
        Ok(bytes)
    } else {
        Ok(stream.content.clone())
    }
}

fn image_dictionary(width: u32, height: u32, gray: bool) -> Dictionary {
    dictionary! {
        "Type" => "XObject", "Subtype" => "Image",
        "Width" => i64::from(width), "Height" => i64::from(height),
        "ColorSpace" => if gray { "DeviceGray" } else { "DeviceRGB" },
        "BitsPerComponent" => 8,
    }
}

fn image_dimensions(
    pdf: &lopdf::Document,
    stream: &Stream,
    gray: bool,
) -> Result<(u32, u32), EngineError> {
    let integer = |key: &[u8]| -> Result<u32, EngineError> {
        u32::try_from(
            field(pdf, &stream.dict, key)?
                .as_i64()
                .map_err(|_| unsupported())?,
        )
        .map_err(|_| unsupported())
    };
    let (width, height) = (integer(b"Width")?, integer(b"Height")?);
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || u64::from(width) * u64::from(height) > 4_000_000
        || field(pdf, &stream.dict, b"Type")?.as_name().ok() != Some(b"XObject".as_slice())
        || field(pdf, &stream.dict, b"Subtype")?.as_name().ok() != Some(b"Image".as_slice())
        || field(pdf, &stream.dict, b"ColorSpace")?.as_name().ok()
            != Some(if gray {
                b"DeviceGray".as_slice()
            } else {
                b"DeviceRGB".as_slice()
            })
        || integer(b"BitsPerComponent")? != 8
    {
        return Err(unsupported());
    }
    Ok((width, height))
}

/// A native writer rounds each translated/reflected Rect endpoint once, and rounds
/// the display-space subtraction used for the AP dimension once. Compare only
/// within the resulting half-ULP error interval; never fuzz AP content/resources.
fn native_span_matches(low: f32, high: f32, dimension: f32) -> bool {
    let half_ulp = |value: f32| {
        ((f64::from(value.next_up()) - f64::from(value))
            .max(f64::from(value) - f64::from(value.next_down())))
            / 2.0
    };
    let uncertainty = half_ulp(low) + half_ulp(high) + half_ulp(dimension);
    // Refuse coordinates whose precision is too coarse to preserve the native
    // appearance's 0.001-point output resolution when placed on another page.
    uncertainty <= 0.001
        && (f64::from(high) - f64::from(low) - f64::from(dimension)).abs() <= uncertainty
}

fn appearance_dimensions(
    pdf: &lopdf::Document,
    ap: &Stream,
    user_rect: PointRect,
    geo: &PageGeometry,
) -> Result<(f32, f32), EngineError> {
    let bbox = field(pdf, &ap.dict, b"BBox")?
        .as_array()
        .map_err(|_| unsupported())?;
    if bbox.len() != 4 {
        return Err(unsupported());
    }
    let number = |index| {
        resolve(pdf, &bbox[index])?
            .as_float()
            .map_err(|_| unsupported())
    };
    let (w, h) = (number(2)?, number(3)?);
    if number(0)? != 0.0
        || number(1)? != 0.0
        || !finite(w, 4.0, 10000.0)
        || !finite(h, 4.0, 10000.0)
    {
        return Err(unsupported());
    }
    let (user_w, user_h) = match geo.rotation {
        crate::Rotation::R0 | crate::Rotation::R180 => (w, h),
        crate::Rotation::R90 | crate::Rotation::R270 => (h, w),
    };
    if !native_span_matches(user_rect.left, user_rect.right, user_w)
        || !native_span_matches(user_rect.bottom, user_rect.top, user_h)
    {
        return Err(unsupported());
    }
    Ok((w, h))
}

fn image(
    pdf: &lopdf::Document,
    ap: &Stream,
    rect: PointRect,
    geo: &PageGeometry,
    dimensions: (f32, f32),
) -> Result<(LibraryPrimitive, Stream), EngineError> {
    let resources = field(pdf, &ap.dict, b"Resources")?
        .as_dict()
        .map_err(|_| unsupported())?;
    let xobjects = field(pdf, resources, b"XObject")?
        .as_dict()
        .map_err(|_| unsupported())?;
    if xobjects.len() != 1 {
        return Err(unsupported());
    }
    let image = native_stream(
        pdf,
        xobjects.get(b"Image").map_err(|_| unsupported())?,
        &[
            b"Type",
            b"Subtype",
            b"Width",
            b"Height",
            b"ColorSpace",
            b"BitsPerComponent",
            b"SMask",
        ],
    )?;
    let (width, height) = image_dimensions(pdf, image, false)?;
    let pixels = width as usize * height as usize;
    let mut budget = CaptureBudget {
        nodes: 8192,
        bytes: 2 * 1024 * 1024,
    };
    // Check declared dimensions before allocating or decompressing pixel buffers.
    if pixels > budget.bytes / 3 {
        return Err(invalid("annotation appearance exceeds capture byte limits"));
    }
    let rgb = decoded(image, &mut budget)?;
    if rgb.len() != pixels * 3 {
        return Err(unsupported());
    }
    let mut expected_image = Stream::new(image_dictionary(width, height, false), rgb.clone());
    let alpha = if let Ok(mask) = image.dict.get(b"SMask") {
        let mask = native_stream(
            pdf,
            mask,
            &[
                b"Type",
                b"Subtype",
                b"Width",
                b"Height",
                b"ColorSpace",
                b"BitsPerComponent",
            ],
        )?;
        if image_dimensions(pdf, mask, true)? != (width, height) {
            return Err(unsupported());
        }
        let alpha = decoded(mask, &mut budget)?;
        if alpha.len() != pixels || alpha.iter().all(|value| *value == 255) {
            return Err(unsupported());
        }
        expected_image.dict.set(
            "SMask",
            Object::Stream(Stream::new(
                image_dictionary(width, height, true),
                alpha.clone(),
            )),
        );
        Some(alpha)
    } else {
        None
    };
    let mut rgba = Vec::with_capacity(pixels * 4);
    let mut visible = false;
    for (index, color) in rgb.as_chunks::<3>().0.iter().enumerate() {
        let opacity = alpha.as_ref().map_or(255, |values| values[index]);
        if opacity == 0 && color.iter().any(|value| *value != 0) {
            return Err(unsupported());
        }
        visible |= opacity != 0;
        rgba.extend_from_slice(color);
        rgba.push(opacity);
    }
    if !visible {
        return Err(unsupported());
    }
    let mut expected = crate::annot_write::ap_stream(
        &crate::image_stamp::image_ap(dimensions.0, dimensions.1, width, height),
        geo,
    );
    expected
        .dict
        .get_mut(b"Resources")
        .map_err(|_| unsupported())?
        .as_dict_mut()
        .map_err(|_| unsupported())?
        .set(
            "XObject",
            dictionary! { "Image" => Object::Stream(expected_image) },
        );
    let rgba = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(unsupported)?;
    let mut encoded = Cursor::new(Vec::new());
    rgba.write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|_| invalid("PNG encoding failed"))?;
    let png = EmbeddedPng::from_png_bytes(encoded.get_ref())?;
    Ok((
        LibraryPrimitive::Image {
            rect: [rect.left, rect.bottom, rect.right, rect.top],
            png,
            // The writer already baked the original opacity into the saved soft mask.
            opacity: 1.0,
        },
        expected,
    ))
}

pub(super) fn capture(
    pdf: &lopdf::Document,
    dict: &Dictionary,
    geo: &PageGeometry,
) -> Result<(LibraryPrimitive, String), EngineError> {
    if dict.has(b"SuniPDFRevisionOutline") {
        return Err(invalid(
            "revision outlines cannot be captured without preserving their source-bound provenance",
        ));
    }
    if dict.iter().any(|(key, _)| {
        !matches!(
            key.as_slice(),
            b"Type"
                | b"Subtype"
                | b"Rect"
                | b"F"
                | b"NM"
                | b"M"
                | b"CreationDate"
                | b"T"
                | b"P"
                | b"AP"
                | b"Name"
                | b"Contents"
                | b"SuniPDFRich"
        )
    }) || field(pdf, dict, b"Type")?.as_name().ok() != Some(b"Annot".as_slice())
        || field(pdf, dict, b"Subtype")?.as_name().ok() != Some(b"Stamp".as_slice())
        || field(pdf, dict, b"SuniPDFRich")?.as_name().ok() != Some(b"Stamp".as_slice())
        || field(pdf, dict, b"F")?.as_i64().ok() != Some(4)
    {
        return Err(unsupported());
    }
    for key in [b"NM".as_slice(), b"T", b"M", b"CreationDate", b"Contents"] {
        if dict.has(key)
            && !matches!(field(pdf, dict, key)?, Object::String(bytes, _) if bytes.len() <= 8192)
        {
            return Err(unsupported());
        }
    }
    if !crate::annot_write::text_of(pdf, dict, b"NM")
        .is_some_and(|name| name.starts_with(crate::annot_write::NAME_PREFIX))
    {
        return Err(unsupported());
    }
    let values = field(pdf, dict, b"Rect")?
        .as_array()
        .map_err(|_| unsupported())?;
    if values.len() != 4 {
        return Err(unsupported());
    }
    let number = |index| {
        resolve(pdf, &values[index])?
            .as_float()
            .map_err(|_| unsupported())
    };
    let user_rect = PointRect {
        left: number(0)?,
        bottom: number(1)?,
        right: number(2)?,
        top: number(3)?,
    };
    if [
        user_rect.left,
        user_rect.bottom,
        user_rect.right,
        user_rect.top,
    ]
    .iter()
    .any(|value| !value.is_finite())
        || user_rect.right <= user_rect.left
        || user_rect.top <= user_rect.bottom
    {
        return Err(unsupported());
    }
    let rect = geo.rect_to_display(user_rect);
    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
    let page = geo.display_size();
    if !finite(w, 4.0, 10000.0)
        || !finite(h, 4.0, 10000.0)
        || rect.left < 0.0
        || rect.bottom < 0.0
        || rect.right > page.0
        || rect.top > page.1
    {
        return Err(unsupported());
    }
    let name = field(pdf, dict, b"Name")?
        .as_name()
        .map_err(|_| unsupported())?;
    let (stamp, label) = match name {
        b"Approved" => (Some(LibraryStamp::Approved), "Approved"),
        b"Draft" => (Some(LibraryStamp::Draft), "Draft"),
        b"Confidential" => (Some(LibraryStamp::Confidential), "Confidential"),
        b"Rejected" => (Some(LibraryStamp::Rejected), "Rejected"),
        b"SuniPDFImage" => (None, "Custom image stamp"),
        _ => return Err(unsupported()),
    };
    if crate::annot_write::text_of(pdf, dict, b"Contents").as_deref() != Some(label) {
        return Err(unsupported());
    }
    let appearance = field(pdf, dict, b"AP")?
        .as_dict()
        .map_err(|_| unsupported())?;
    if appearance.len() != 1 {
        return Err(unsupported());
    }
    let source = appearance.get(b"N").map_err(|_| unsupported())?;
    let ap = native_stream(
        pdf,
        source,
        &[b"Type", b"Subtype", b"BBox", b"Matrix", b"Resources"],
    )?;
    // BBox retains the writer's dimensions. Recomputing them from translated
    // Rect endpoints can differ by an f32 ULP after CropBox/rotation conversion.
    let (w, h) = appearance_dimensions(pdf, ap, user_rect, geo)?;
    // Budget the complete source graph, including compressed and decompressed payloads,
    // before reconstructing pixels. This also binds stale selections to all source fields.
    let fingerprint = capture_dictionary(
        pdf,
        dict,
        0,
        &mut CaptureBudget {
            nodes: 8192,
            bytes: 2 * 1024 * 1024,
        },
        &[b"P"],
    )?;
    let (primitive, expected) = if let Some(stamp) = stamp {
        (
            LibraryPrimitive::Stamp {
                rect: [rect.left, rect.bottom, rect.right, rect.top],
                stamp,
            },
            crate::annot_write::ap_stream(&crate::appearance::stamp_ap(w, h, label), geo),
        )
    } else {
        image(pdf, ap, rect, geo, (w, h))?
    };
    if capture_object(pdf, source, 0, &mut 4096)?
        != capture_object(pdf, &Object::Stream(expected), 0, &mut 4096)?
    {
        return Err(unsupported());
    }
    primitive.validate()?;
    Ok((
        primitive,
        digest_hex(&serde_json::to_vec(&fingerprint).map_err(|_| unsupported())?),
    ))
}
