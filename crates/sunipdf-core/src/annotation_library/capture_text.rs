//! Capture only text whose complete native appearance can be reproduced from portable values.

use super::{
    capture_dictionary, capture_object, digest_hex, finite, invalid, CaptureBudget, LibraryFont,
    LibraryPrimitive, LibraryTextStyle,
};
use crate::{geometry::PageGeometry, Document, EngineError, PointRect};
use lopdf::{dictionary, Dictionary, Object, Stream};

fn unsupported() -> EngineError {
    invalid("only reproducible native FreeText with standard Helvetica, Times or Courier fonts can be captured; custom fonts, appearances, actions and review history are not supported")
}

fn resolve<'a>(
    pdf: &'a lopdf::Document,
    mut object: &'a Object,
) -> Result<&'a Object, EngineError> {
    for _ in 0..16 {
        match object {
            Object::Reference(id) => object = pdf.objects.get(id).ok_or_else(unsupported)?,
            _ => return Ok(object),
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

fn number(pdf: &lopdf::Document, object: &Object) -> Result<f32, EngineError> {
    let value = resolve(pdf, object)?
        .as_float()
        .map_err(|_| unsupported())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(unsupported())
    }
}

fn color(value: f32) -> Result<u8, EngineError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(unsupported());
    }
    let byte = (value * 255.0).round() as u8;
    if f32::from(byte) / 255.0 == value {
        Ok(byte)
    } else {
        Err(unsupported())
    }
}

fn font_style(name: &[u8]) -> Result<(LibraryFont, bool, bool), EngineError> {
    Ok(match name {
        b"Helvetica" => (LibraryFont::Helvetica, false, false),
        b"Helvetica-Bold" => (LibraryFont::Helvetica, true, false),
        b"Helvetica-Oblique" => (LibraryFont::Helvetica, false, true),
        b"Helvetica-BoldOblique" => (LibraryFont::Helvetica, true, true),
        b"Times-Roman" => (LibraryFont::Times, false, false),
        b"Times-Bold" => (LibraryFont::Times, true, false),
        b"Times-Italic" => (LibraryFont::Times, false, true),
        b"Times-BoldItalic" => (LibraryFont::Times, true, true),
        b"Courier" => (LibraryFont::Courier, false, false),
        b"Courier-Bold" => (LibraryFont::Courier, true, false),
        b"Courier-Oblique" => (LibraryFont::Courier, false, true),
        b"Courier-BoldOblique" => (LibraryFont::Courier, true, true),
        _ => return Err(unsupported()),
    })
}

/// Translation must not change the effective text box used by the native line wrapper.
/// Translated f32 endpoints round the recomputed span by about one ULP; refuse only drift
/// beyond the 0.001-point resolution that saved stamp capture also accepts.
pub(super) fn preserve_dimensions(
    before: [f32; 4],
    after: [f32; 4],
    scale: f32,
) -> Result<(), EngineError> {
    for (low, high) in [(0, 2), (1, 3)] {
        let expected = (before[high] - before[low]) * scale;
        let actual = after[high] - after[low];
        if !expected.is_finite()
            || expected <= 0.0
            || !actual.is_finite()
            || (actual - expected).abs() > 0.001
        {
            return Err(invalid(
                "text box precision would change its layout; choose a different origin or scale",
            ));
        }
    }
    Ok(())
}

pub(super) fn capture(
    document: &Document<'_>,
    pdf: &lopdf::Document,
    dict: &Dictionary,
    geo: &PageGeometry,
) -> Result<(LibraryPrimitive, String), EngineError> {
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
                | b"Contents"
                | b"SuniPDFRich"
                | b"DA"
                | b"CA"
                | b"BS"
                | b"C"
        )
    }) || field(pdf, dict, b"Type")?.as_name().ok() != Some(b"Annot".as_slice())
        || field(pdf, dict, b"Subtype")?.as_name().ok() != Some(b"FreeText".as_slice())
        || field(pdf, dict, b"SuniPDFRich")?.as_name().ok() != Some(b"FreeText".as_slice())
        || field(pdf, dict, b"F")?.as_i64().ok() != Some(4)
    {
        return Err(unsupported());
    }
    for key in [b"NM".as_slice(), b"M", b"CreationDate", b"T", b"Contents"] {
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
    // Bound every retained source property and every decoded resource before native work.
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
    let text = crate::annot_write::text_of(pdf, dict, b"Contents").ok_or_else(unsupported)?;
    let values = field(pdf, dict, b"Rect")?
        .as_array()
        .map_err(|_| unsupported())?;
    if values.len() != 4 {
        return Err(unsupported());
    }
    let user_rect = PointRect {
        left: number(pdf, &values[0])?,
        bottom: number(pdf, &values[1])?,
        right: number(pdf, &values[2])?,
        top: number(pdf, &values[3])?,
    };
    if user_rect.right <= user_rect.left || user_rect.top <= user_rect.bottom {
        return Err(unsupported());
    }
    let rect = geo.rect_to_display(user_rect);
    crate::annot_rich::checked_rect(geo, rect)?;
    let appearances = field(pdf, dict, b"AP")?
        .as_dict()
        .map_err(|_| unsupported())?;
    if appearances.len() != 1 {
        return Err(unsupported());
    }
    let source = appearances.get(b"N").map_err(|_| unsupported())?;
    let ap = resolve(pdf, source)?
        .as_stream()
        .map_err(|_| unsupported())?;
    if ap.dict.iter().any(|(key, _)| {
        !matches!(
            key.as_slice(),
            b"Type" | b"Subtype" | b"BBox" | b"Matrix" | b"Resources" | b"Length" | b"Filter"
        )
    }) || ap
        .dict
        .get(b"Filter")
        .is_ok_and(|filter| !matches!(filter, Object::Name(name) if name == b"FlateDecode"))
        || ap
            .dict
            .get(b"Length")
            .is_ok_and(|length| length.as_i64().ok() != i64::try_from(ap.content.len()).ok())
    {
        return Err(unsupported());
    }
    // BBox retains the writer's layout box. Recomputing it from Rect endpoints can differ
    // by an f32 ULP after CropBox/rotation conversion, as for saved stamps.
    let (w, h) = super::capture_stamps::appearance_dimensions(pdf, ap, user_rect, geo)
        .map_err(|_| invalid("saved text box precision cannot preserve its native layout"))?;
    let resources = field(pdf, &ap.dict, b"Resources")?
        .as_dict()
        .map_err(|_| unsupported())?;
    let fonts = field(pdf, resources, b"Font")?
        .as_dict()
        .map_err(|_| unsupported())?;
    if fonts.len() != 1 {
        return Err(unsupported());
    }
    let (font_name, font) = fonts.iter().next().ok_or_else(unsupported)?;
    if font_name.is_empty()
        || font_name.len() > 64
        || !font_name.iter().all(u8::is_ascii_alphanumeric)
    {
        return Err(unsupported());
    }
    let font_name = std::str::from_utf8(font_name).map_err(|_| unsupported())?;
    let font = resolve(pdf, font)?.as_dict().map_err(|_| unsupported())?;
    if field(pdf, font, b"Type")?.as_name().ok() != Some(b"Font".as_slice())
        || field(pdf, font, b"Subtype")?.as_name().ok() != Some(b"Type1".as_slice())
        || font
            .get(b"Encoding")
            .is_ok_and(|value| !matches!(resolve(pdf, value), Ok(Object::Name(_))))
        || [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
            .iter()
            .any(|key| font.has(key))
    {
        return Err(unsupported());
    }
    if font.has(b"FontDescriptor") {
        let descriptor = field(pdf, font, b"FontDescriptor")?
            .as_dict()
            .map_err(|_| unsupported())?;
        if [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
            .iter()
            .any(|key| descriptor.has(key))
        {
            return Err(unsupported());
        }
    }
    // Native font metrics and encoding names are compared to the regenerated graph below,
    // rather than assuming a particular PDFium serialization of a standard font.
    let (family, bold, italic) = font_style(
        field(pdf, font, b"BaseFont")?
            .as_name()
            .map_err(|_| unsupported())?,
    )?;
    let da = field(pdf, dict, b"DA")?
        .as_str()
        .map_err(|_| unsupported())?;
    if da.len() > 256 {
        return Err(unsupported());
    }
    let da = std::str::from_utf8(da).map_err(|_| unsupported())?;
    let parts: Vec<_> = da.split(' ').collect();
    if parts.len() != 7
        || parts[0] != format!("/{font_name}")
        || parts[2] != "Tf"
        || parts[6] != "rg"
    {
        return Err(unsupported());
    }
    let parse = |index: usize| -> Result<f32, EngineError> {
        parts[index].parse::<f32>().map_err(|_| unsupported())
    };
    let size_pt = parse(1)?;
    let rgb = [color(parse(3)?)?, color(parse(4)?)?, color(parse(5)?)?];
    if da
        != format!(
            "/{font_name} {size_pt} Tf {} {} {} rg",
            f32::from(rgb[0]) / 255.0,
            f32::from(rgb[1]) / 255.0,
            f32::from(rgb[2]) / 255.0
        )
    {
        return Err(unsupported());
    }
    let border = field(pdf, dict, b"BS")?
        .as_dict()
        .map_err(|_| unsupported())?;
    let border_pt = number(pdf, field(pdf, border, b"W")?)?;
    if capture_object(pdf, field(pdf, dict, b"BS")?, 0, &mut 64)?
        != capture_object(
            pdf,
            &crate::annot_write::border_style(border_pt, &[]),
            0,
            &mut 64,
        )?
    {
        return Err(unsupported());
    }
    let opacity = number(pdf, field(pdf, dict, b"CA")?)?;
    let fill_rgb = if dict.has(b"C") {
        let rgb = field(pdf, dict, b"C")?
            .as_array()
            .map_err(|_| unsupported())?;
        if rgb.len() != 3 {
            return Err(unsupported());
        }
        Some([
            color(number(pdf, &rgb[0])?)?,
            color(number(pdf, &rgb[1])?)?,
            color(number(pdf, &rgb[2])?)?,
        ])
    } else {
        None
    };
    let style = LibraryTextStyle {
        family,
        size_pt,
        bold,
        italic,
        rgb,
        opacity,
        fill_rgb,
        border_pt,
    };
    style.validate()?;
    if !finite(w, 4.0, 10000.0) || !finite(h, 4.0, 10000.0) {
        return Err(unsupported());
    }
    let primitive = LibraryPrimitive::FreeText {
        rect: [rect.left, rect.bottom, rect.right, rect.top],
        text: text.clone(),
        style: style.clone(),
    };
    primitive.validate()?;
    let expected_pdf = crate::security::load(
        &document.rich_text_source(w, h, &text, &style.placed(1.0))?,
        None,
    )?;
    let page = expected_pdf.get_pages()[&1];
    let expected_resources = expected_pdf
        .get_dictionary(page)
        .map_err(crate::security::lopdf_err)?
        .get(b"Resources")
        .map_err(crate::security::lopdf_err)?
        .clone();
    let content = expected_pdf
        .get_page_content_with_limit(page, 2 * 1024 * 1024)
        .map_err(crate::security::lopdf_err)?;
    let (a, b) = geo.vec_to_user(1.0, 0.0);
    let (c, d) = geo.vec_to_user(0.0, 1.0);
    let expected = Object::Stream(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => crate::annot_write::nums(&[0.0, 0.0, w, h]),
            "Matrix" => crate::annot_write::nums(&[a, b, c, d, 0.0, 0.0]),
            "Resources" => expected_resources,
        },
        content,
    ));
    if capture_object(pdf, source, 0, &mut 8192)?
        != capture_object(&expected_pdf, &expected, 0, &mut 8192)?
    {
        return Err(unsupported());
    }
    Ok((
        primitive,
        digest_hex(&serde_json::to_vec(&fingerprint).map_err(|_| unsupported())?),
    ))
}

#[cfg(test)]
mod tests;
