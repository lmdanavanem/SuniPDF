//! Explicit structural cleanup into a rewritten new copy; see SECURITY_MODEL.md.

use crate::security::{self, lopdf_err};
use crate::{Document, EngineError};
use lopdf::{content::Content, Dictionary, Object, Stream};
use std::collections::BTreeSet;

const REMOVE: &[&[u8]] = &[
    b"Outlines",
    b"Dests",
    b"EmbeddedFiles",
    b"JavaScript",
    b"JS",
    b"AA",
    b"OpenAction",
    b"AF",
    b"EF",
    b"FS",
    b"Collection",
    b"Metadata",
    b"PieceInfo",
    b"XFA",
];
const MAX_NODES: usize = 1_000_000;
const MAX_CONTENT: usize = 8 * 1024 * 1024;
const MAX_TOTAL_CONTENT: usize = 256 * 1024 * 1024;
const REMOVED_ANNOTATIONS: &[&[u8]] = &[
    b"FileAttachment",
    b"RichMedia",
    b"Movie",
    b"Sound",
    b"3D",
    b"Screen",
];

#[derive(Debug, Clone)]
pub struct SanitizeReport {
    pub removed_entries: usize,
    pub removed_annotations: usize,
    pub unused_font_entries: usize,
    pub unreachable_objects: usize,
    pub verified: bool,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("document sanitization: {reason}"))
}

fn dictionary<'a>(
    pdf: &'a lopdf::Document,
    object: &'a Object,
) -> Result<&'a Dictionary, EngineError> {
    pdf.dereference(object)
        .map_err(lopdf_err)?
        .1
        .as_dict()
        .map_err(lopdf_err)
}

fn name<'a>(pdf: &'a lopdf::Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a [u8]> {
    pdf.dereference(dict.get(key).ok()?).ok()?.1.as_name().ok()
}

fn stream_bytes(stream: &Stream, total: &mut usize) -> Result<Vec<u8>, EngineError> {
    let bytes = if stream.dict.has(b"Filter") {
        stream
            .decompressed_content_with_limit(MAX_CONTENT)
            .map_err(lopdf_err)?
    } else {
        if stream.content.len() > MAX_CONTENT {
            return Err(invalid("content stream limit"));
        }
        stream.content.clone()
    };
    *total = total
        .checked_add(bytes.len())
        .ok_or_else(|| invalid("content size overflow"))?;
    if *total > MAX_TOTAL_CONTENT {
        return Err(invalid("total content limit"));
    }
    Ok(bytes)
}

fn font_names(content: &[u8], fonts: &mut BTreeSet<Vec<u8>>) -> Result<(), EngineError> {
    let content =
        Content::decode_strict(content).map_err(|_| invalid("unsupported content syntax"))?;
    for op in content.operations.iter().filter(|op| op.operator == "Tf") {
        let name = op
            .operands
            .first()
            .and_then(|value| value.as_name().ok())
            .ok_or_else(|| invalid("invalid font selection"))?;
        fonts.insert(name.to_vec());
    }
    Ok(())
}

fn defaults(
    pdf: &lopdf::Document,
    object: &Object,
    fonts: &mut BTreeSet<Vec<u8>>,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), EngineError> {
    *nodes += 1;
    if depth > 32 || *nodes > MAX_NODES {
        return Err(invalid("structure limit"));
    }
    let dict = match object {
        Object::Array(values) => {
            for value in values {
                defaults(pdf, value, fonts, depth + 1, nodes)?;
            }
            return Ok(());
        }
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return Ok(()),
    };
    let external_font = dict
        .get(b"Font")
        .ok()
        .and_then(|value| pdf.dereference(value).ok())
        .is_some_and(|(_, value)| matches!(value, Object::Array(_)));
    if name(pdf, dict, b"Subtype") == Some(b"Type3") || external_font {
        return Err(invalid("Type3/ExtGState font selection is unsupported"));
    }
    if let Ok(value) = dict.get(b"DA") {
        let bytes = pdf
            .dereference(value)
            .map_err(lopdf_err)?
            .1
            .as_str()
            .map_err(lopdf_err)?;
        if bytes.len() > 65_536 {
            return Err(invalid("default appearance limit"));
        }
        font_names(bytes, fonts)?;
    }
    for (_, value) in dict.iter() {
        defaults(pdf, value, fonts, depth + 1, nodes)?;
    }
    Ok(())
}

fn used_fonts(pdf: &lopdf::Document) -> Result<BTreeSet<Vec<u8>>, EngineError> {
    let mut fonts = BTreeSet::new();
    let mut total = 0;
    for page in pdf.get_pages().into_values() {
        let mut content = Vec::new();
        let value = pdf
            .get_dictionary(page)
            .map_err(lopdf_err)?
            .get(b"Contents")
            .ok();
        let value = value
            .map(|value| pdf.dereference(value).map(|(_, value)| value))
            .transpose()
            .map_err(lopdf_err)?;
        let streams = match value {
            None | Some(Object::Null) => Vec::new(),
            Some(Object::Array(values)) => values.iter().collect(),
            Some(value @ Object::Stream(_)) => vec![value],
            _ => return Err(invalid("invalid page content structure")),
        };
        if streams.len() > 16_384 {
            return Err(invalid("page content count limit"));
        }
        for stream in streams {
            let bytes = stream_bytes(
                pdf.dereference(stream)
                    .map_err(lopdf_err)?
                    .1
                    .as_stream()
                    .map_err(lopdf_err)?,
                &mut total,
            )?;
            if content.len().saturating_add(bytes.len()) + 1 > MAX_CONTENT {
                return Err(invalid("page content limit"));
            }
            content.extend(bytes);
            content.push(b'\n');
        }
        font_names(&content, &mut fonts)?;
    }
    let mut nodes = 0;
    for object in pdf.objects.values() {
        if let Object::Stream(stream) = object {
            if name(pdf, &stream.dict, b"Subtype") == Some(b"Form")
                || stream.dict.has(b"PatternType")
            {
                font_names(&stream_bytes(stream, &mut total)?, &mut fonts)?;
            }
        }
        defaults(pdf, object, &mut fonts, 0, &mut nodes)?;
    }
    Ok(fonts)
}

fn clean(
    pdf: &lopdf::Document,
    object: &mut Object,
    fonts: &BTreeSet<Vec<u8>>,
    report: &mut SanitizeReport,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), EngineError> {
    *nodes += 1;
    if depth > 32 || *nodes > MAX_NODES {
        return Err(invalid("structure limit exceeded"));
    }
    let dict = match object {
        Object::Array(values) => {
            for value in values {
                clean(pdf, value, fonts, report, depth + 1, nodes)?;
            }
            return Ok(());
        }
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &mut stream.dict,
        _ => return Ok(()),
    };
    if name(pdf, dict, b"S") == Some(b"JavaScript") {
        report.removed_entries += 1;
        *object = Object::Null;
        return Ok(());
    }
    for key in REMOVE {
        report.removed_entries += usize::from(dict.remove(key).is_some());
    }
    if let Ok(value) = dict.get(b"Annots") {
        let original = pdf
            .dereference(value)
            .map_err(lopdf_err)?
            .1
            .as_array()
            .map_err(lopdf_err)?;
        let mut kept = Vec::new();
        for annotation in original {
            let subtype = name(pdf, dictionary(pdf, annotation)?, b"Subtype")
                .ok_or_else(|| invalid("invalid annotation subtype"))?;
            if REMOVED_ANNOTATIONS.contains(&subtype) {
                report.removed_annotations += 1;
            } else {
                kept.push(annotation.clone());
            }
        }
        dict.set("Annots", Object::Array(kept));
    }
    if let Ok(value) = dict.get(b"Resources") {
        let mut resources = dictionary(pdf, value)?.clone();
        if let Ok(value) = resources.get(b"Font") {
            let mut font_dictionary = dictionary(pdf, value)?.clone();
            // Ambiguous reused names are retained rather than risking visible text loss.
            let unused: Vec<_> = font_dictionary
                .iter()
                .filter(|(name, _)| !fonts.contains(*name))
                .map(|(name, _)| name.clone())
                .collect();
            for name in unused {
                font_dictionary.remove(&name);
                report.unused_font_entries += 1;
            }
            resources.set("Font", Object::Dictionary(font_dictionary));
        }
        dict.set("Resources", Object::Dictionary(resources));
    }
    for (_, value) in dict.iter_mut() {
        clean(pdf, value, fonts, report, depth + 1, nodes)?;
    }
    Ok(())
}

fn verify(
    pdf: &lopdf::Document,
    object: &Object,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), EngineError> {
    *nodes += 1;
    if depth > 32 || *nodes > MAX_NODES {
        return Err(invalid("verification structure limit"));
    }
    let dict = match object {
        Object::Array(values) => {
            for value in values {
                verify(pdf, value, depth + 1, nodes)?;
            }
            return Ok(());
        }
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return Ok(()),
    };
    if REMOVE.iter().any(|key| dict.has(key))
        || name(pdf, dict, b"Type") == Some(b"EmbeddedFile")
        || name(pdf, dict, b"S") == Some(b"JavaScript")
        || name(pdf, dict, b"Subtype").is_some_and(|subtype| REMOVED_ANNOTATIONS.contains(&subtype))
    {
        return Err(invalid("cleanup could not be verified"));
    }
    for (_, value) in dict.iter() {
        verify(pdf, value, depth + 1, nodes)?;
    }
    Ok(())
}

pub fn sanitize_document(
    input: &[u8],
    password: Option<&str>,
) -> Result<(Vec<u8>, SanitizeReport), EngineError> {
    if input.len() > 128 * 1024 * 1024 {
        return Err(invalid("input size limit"));
    }
    let mut pdf = security::load(input, password)?;
    if pdf.objects.len() > 250_000 {
        return Err(invalid("too many objects"));
    }
    let fonts = used_fonts(&pdf)?;
    let mut report = SanitizeReport {
        removed_entries: 0,
        removed_annotations: 0,
        unused_font_entries: 0,
        unreachable_objects: 0,
        verified: false,
    };
    let mut nodes = 0;
    let mut replacements = Vec::with_capacity(pdf.objects.len());
    for (&id, object) in &pdf.objects {
        let mut object = object.clone();
        clean(&pdf, &mut object, &fonts, &mut report, 0, &mut nodes)?;
        replacements.push((id, object));
    }
    for (id, object) in replacements {
        pdf.objects.insert(id, object);
    }
    security::strip_metadata_in(&mut pdf);
    let original_objects = pdf.objects.len();
    pdf.prune_objects();
    report.unreachable_objects = original_objects - pdf.objects.len();
    let bytes = security::save(pdf)?;
    let reopened = security::load(&bytes, password)?;
    let mut nodes = 0;
    for object in reopened.objects.values() {
        verify(&reopened, object, 0, &mut nodes)?;
    }
    report.verified = true;
    Ok((bytes, report))
}

impl Document<'_> {
    pub fn sanitized_copy(&self) -> Result<(Vec<u8>, SanitizeReport), EngineError> {
        sanitize_document(&self.snapshot()?, self.password())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn cleanup_verification_rejects_remaining_xfa_and_screen_annotations() {
        let pdf = lopdf::Document::new();
        for object in [
            Object::Dictionary(
                dictionary! {"AcroForm"=>dictionary! {"XFA"=>Object::string_literal("remaining")}},
            ),
            Object::Dictionary(dictionary! {"Subtype"=>"Screen"}),
        ] {
            assert!(verify(&pdf, &object, 0, &mut 0).is_err());
        }
    }
}
