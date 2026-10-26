//! Bounded native-font FreeText and standard annotation appearances.

use crate::annot_write::{self, AnnotStyle};
use crate::geometry::PageGeometry;
use crate::security::lopdf_err;
use crate::{Document, EngineError, PixelRect, PointRect, RenderedPage, TextStyle};
use lopdf::{dictionary, Dictionary, Object, Stream};
use pdfium_render::prelude::*;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq)]
pub struct RichFreeTextStyle {
    pub text: TextStyle,
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub border_pt: f32,
}

impl Default for RichFreeTextStyle {
    fn default() -> Self {
        Self {
            text: TextStyle::default(),
            fill_rgb: None,
            border_pt: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexAnnotKind {
    Polygon,
    Polyline,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampKind {
    Approved,
    Draft,
    Confidential,
    Rejected,
}

impl StampKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Approved => "Approved",
            Self::Draft => "Draft",
            Self::Confidential => "Confidential",
            Self::Rejected => "Rejected",
        }
    }
}

#[derive(Debug, Clone)]
pub enum RichAnnotation {
    FreeText {
        index: Option<usize>,
        rect: PointRect,
        text: String,
        style: RichFreeTextStyle,
    },
    Vertices {
        kind: VertexAnnotKind,
        points: Vec<(f32, f32)>,
        style: AnnotStyle,
    },
    Stamp {
        rect: PointRect,
        kind: StampKind,
    },
    ImageStamp {
        rect: PointRect,
        image: std::path::PathBuf,
        opacity: f32,
    },
    LibraryPlacement {
        entry: crate::annotation_library::LibraryEntry,
        placement: crate::annotation_library::LibraryPlacement,
    },
    Measurement(crate::measurement::Measurement),
    DimensionedRectangle(crate::measurement::DimensionedRectangle),
    DimensionedEllipse(crate::measurement::DimensionedEllipse),
    DimensionedSegment(crate::measurement::DimensionedSegment),
    UpdateMeasurement {
        target: crate::measurement::MeasurementTarget,
        measurement: crate::measurement::Measurement,
    },
    RecalibrateMeasurement {
        target: crate::measurement::MeasurementTarget,
        measurement: crate::measurement::Measurement,
        calibration: crate::measurement::PageCalibration,
    },
    Attachment {
        rect: PointRect,
        filename: String,
        bytes: Vec<u8>,
    },
    EraseInk {
        index: usize,
        path: Vec<(f32, f32)>,
        radius: f32,
    },
}

pub struct RichAnnotationPreview {
    pub image: RenderedPage,
    pub scale: f32,
    pub region: PixelRect,
    pub index: usize,
    pub prepared: Option<PreparedRichAnnotation>,
}

/// A rendered candidate bound to the exact original document; fields cannot be supplied by callers.
pub struct PreparedRichAnnotation {
    original: Vec<u8>,
    candidate: Vec<u8>,
    index: usize,
}

type Point = (f32, f32);

fn segment_distance(point: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        ((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    (point.0 - a.0 - dx * t).hypot(point.1 - a.1 - dy * t)
}

fn segments_touch(a: Point, b: Point, c: Point, d: Point, radius: f32) -> bool {
    let cross = |p: Point, q: Point, r: Point| {
        f64::from(q.0 - p.0) * f64::from(r.1 - p.1) - f64::from(q.1 - p.1) * f64::from(r.0 - p.0)
    };
    let intersects = a.0.min(b.0) <= c.0.max(d.0)
        && c.0.min(d.0) <= a.0.max(b.0)
        && a.1.min(b.1) <= c.1.max(d.1)
        && c.1.min(d.1) <= a.1.max(b.1)
        && cross(a, b, c) * cross(a, b, d) <= 0.0
        && cross(c, d, a) * cross(c, d, b) <= 0.0;
    intersects
        || segment_distance(a, c, d)
            .min(segment_distance(b, c, d))
            .min(segment_distance(c, a, b))
            .min(segment_distance(d, a, b))
            <= radius
}

fn flatten_ink(stroke: &[Point], remaining: &mut usize) -> Result<Vec<Point>, EngineError> {
    if stroke.len() < 3 {
        return Ok(stroke.to_vec());
    }
    let mid = |a: Point, b: Point| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let mut result = vec![stroke[0]];
    let mut start = stroke[0];
    for index in 1..stroke.len() - 1 {
        let end = mid(stroke[index], stroke[index + 1]);
        let mut stack = vec![(start, stroke[index], end, 0usize)];
        while let Some((a, control, b, depth)) = stack.pop() {
            if *remaining == 0 || depth > 12 {
                return Err(bad());
            }
            *remaining -= 1;
            // A quadratic curve lies in its control triangle; 0.1 pt bounds hit-test approximation.
            if segment_distance(control, a, b) <= 0.1 {
                result.push(b);
            } else {
                let left = mid(a, control);
                let right = mid(control, b);
                let center = mid(left, right);
                stack.push((center, right, b, depth + 1));
                stack.push((a, left, center, depth + 1));
            }
        }
        start = end;
    }
    result.push(*stroke.last().ok_or_else(bad)?);
    Ok(result)
}

pub(crate) fn erase_touched_strokes(
    strokes: &[Vec<Point>],
    path: &[Point],
    radius: f32,
    width: f32,
) -> Result<Vec<Vec<Point>>, EngineError> {
    if strokes.len() > 64
        || strokes
            .iter()
            .any(|stroke| stroke.is_empty() || stroke.len() > 4000)
        || strokes.iter().map(Vec::len).sum::<usize>() > 16000
        || path.is_empty()
        || path.len() > 256
        || !radius.is_finite()
        || !(0.1..=100.0).contains(&radius)
        || !width.is_finite()
        || !(0.0..=50.0).contains(&width)
        || strokes
            .iter()
            .flatten()
            .chain(path.iter())
            .any(|point| !point.0.is_finite() || !point.1.is_finite())
    {
        return Err(bad());
    }
    let radius = radius + width / 2.0 + 0.1;
    let mut remaining = 20000usize;
    let mut checks = 2000000usize;
    let mut kept = Vec::new();
    for stroke in strokes {
        let flattened = flatten_ink(stroke, &mut remaining)?;
        let mut touched = false;
        let segments: Vec<_> = if flattened.len() == 1 {
            vec![(flattened[0], flattened[0])]
        } else {
            flattened
                .windows(2)
                .map(|points| (points[0], points[1]))
                .collect()
        };
        for (a, b) in segments {
            for (index, &c) in path.iter().enumerate() {
                if checks == 0 {
                    return Err(bad());
                }
                checks -= 1;
                let d = path.get(index + 1).copied().unwrap_or(c);
                if segments_touch(a, b, c, d, radius) {
                    touched = true;
                    break;
                }
            }
            if touched {
                break;
            }
        }
        if !touched {
            kept.push(stroke.clone());
        }
    }
    if kept.is_empty() {
        return Err(EngineError::Other("all ink strokes would be removed; use Delete annotation to remove its comment thread explicitly".into()));
    }
    if kept.len() == strokes.len() {
        return Err(EngineError::Other(
            "the eraser did not touch an ink stroke".into(),
        ));
    }
    Ok(kept)
}

fn bad() -> EngineError {
    EngineError::Other("invalid or excessive rich annotation geometry, style or resource".into())
}

fn prepared_document_graph(mut pdf: lopdf::Document) -> Result<lopdf::Document, EngineError> {
    // These are serialization details, not document roots. Reject references or unexpected
    // types before discarding them so they cannot hide a reachable object from this check.
    for key in [b"Size".as_slice(), b"Prev", b"XRefStm"] {
        if let Ok(value) = pdf.trailer.get(key) {
            if !matches!(value, Object::Integer(n) if *n >= 0) {
                return Err(bad());
            }
        }
        pdf.trailer.remove(key);
    }
    if let Ok(value) = pdf.trailer.get(b"ID") {
        let ids = value.as_array().map_err(lopdf_err)?;
        if ids.len() != 2
            || ids
                .iter()
                .any(|id| !matches!(id, Object::String(bytes, _) if bytes.len() <= 1024))
        {
            return Err(bad());
        }
    }
    // PDFium generates a new file ID on save. Encrypted key identity is checked separately.
    pdf.trailer.remove(b"ID");

    // Start at every remaining trailer entry, including Info and custom extension roots.
    // PDFium may create unused appearances while rendering and discard them when reopened.
    // Only unreachable objects may differ; page content, resources and annotations remain exact.
    let mut reachable = BTreeSet::new();
    let mut pending: Vec<_> = pdf
        .trailer
        .iter()
        .map(|(_, value)| (value, 0usize))
        .collect();
    let mut nodes = 0usize;
    while let Some((object, depth)) = pending.pop() {
        nodes += 1;
        if nodes > 1_000_000 || pending.len() > 1_000_000 - nodes || depth > 64 {
            return Err(bad());
        }
        match object {
            Object::Reference(id) => {
                if reachable.insert(*id) {
                    if reachable.len() > 100_000 {
                        return Err(bad());
                    }
                    let referenced = pdf.objects.get(id).ok_or_else(bad)?;
                    pending.push((referenced, 0));
                }
            }
            Object::Array(values) => {
                if values.len() > 1_000_000 - nodes - pending.len() {
                    return Err(bad());
                }
                pending.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Object::Dictionary(dictionary) => {
                if dictionary.len() > 1_000_000 - nodes - pending.len() {
                    return Err(bad());
                }
                pending.extend(dictionary.iter().map(|(_, value)| (value, depth + 1)));
            }
            Object::Stream(stream) => {
                if stream.dict.len() > 1_000_000 - nodes - pending.len() {
                    return Err(bad());
                }
                pending.extend(stream.dict.iter().map(|(_, value)| (value, depth + 1)));
            }
            _ => {}
        }
    }
    pdf.objects.retain(|id, _| reachable.contains(id));
    for object in pdf.objects.values_mut() {
        if let Object::Stream(stream) = object {
            stream.start_position = None;
            stream.allows_compression = true;
        }
    }
    Ok(pdf)
}

pub(crate) fn same_document(
    original: &[u8],
    current: &[u8],
    password: Option<&str>,
) -> Result<bool, EngineError> {
    same_document_context(original, current, password, false)
}

/// Local measurement references may survive annotation-only edits. Keep the page tree, content,
/// resources, scales, trailer roots and encryption exact; only page /Annots roots are ignored.
/// Prepared edits deliberately use `same_document` instead, including all annotations.
pub(crate) fn same_document_for_local_reference(
    original: &[u8],
    current: &[u8],
    password: Option<&str>,
) -> Result<bool, EngineError> {
    same_document_context(original, current, password, true)
}

fn same_document_context(
    original: &[u8],
    current: &[u8],
    password: Option<&str>,
    ignore_page_annotations: bool,
) -> Result<bool, EngineError> {
    if original == current {
        return Ok(true);
    }
    if original.len() > 128 * 1024 * 1024 || current.len() > 128 * 1024 * 1024 {
        return Ok(false);
    }
    type EncryptionEnvelope = (Dictionary, Option<Object>, Option<[u8; 32]>);
    let envelope = |bytes: &[u8]| -> Result<Option<EncryptionEnvelope>, EngineError> {
        let raw = lopdf::Document::load_mem(bytes).map_err(lopdf_err)?;
        let (dictionary, key_identity) = if raw.is_encrypted() {
            (raw.get_encrypted().map_err(lopdf_err)?.clone(), None)
        } else if let Some(state) = &raw.encryption_state {
            // Loading an empty-user-password PDF removes /Encrypt and its object. Compare the
            // retained effective security state and key identity instead of treating it as plain.
            use sha2::{Digest, Sha256};
            (
                state.encode().map_err(|_| bad())?,
                Some(Sha256::digest(state.file_encryption_key()).into()),
            )
        } else {
            return Ok(None);
        };
        let mut pending = vec![(&dictionary, 0usize)];
        let mut nodes = 0usize;
        while let Some((dictionary, depth)) = pending.pop() {
            if depth > 16 {
                return Err(bad());
            }
            for (_, value) in dictionary.iter() {
                nodes += 1;
                if nodes > 4096 {
                    return Err(bad());
                }
                match value {
                    Object::Dictionary(dictionary) => pending.push((dictionary, depth + 1)),
                    Object::Reference(_) | Object::Stream(_) | Object::Array(_) => {
                        return Err(EngineError::Other(
                            "cannot prove unchanged encryption for this prepared annotation".into(),
                        ))
                    }
                    _ => {}
                }
            }
        }
        let id = match raw.trailer.get(b"ID") {
            Ok(value) => Some(
                value
                    .as_array()
                    .map_err(lopdf_err)?
                    .first()
                    .ok_or_else(bad)?
                    .clone(),
            ),
            Err(_) => None,
        };
        Ok(Some((dictionary, id, key_identity)))
    };
    if envelope(original)? != envelope(current)? {
        return Ok(false);
    }
    let normalize = |bytes: &[u8]| -> Result<lopdf::Document, EngineError> {
        let mut pdf = crate::security::load(bytes, password)?;
        if ignore_page_annotations {
            for page in pdf.get_pages().values() {
                pdf.get_dictionary_mut(*page)
                    .map_err(lopdf_err)?
                    .remove(b"Annots");
            }
        }
        prepared_document_graph(pdf)
    };
    let original = normalize(original)?;
    let current = normalize(current)?;
    Ok(original.version == current.version
        && original.objects == current.objects
        && original.trailer == current.trailer)
}

pub(crate) fn checked_rect(geo: &PageGeometry, rect: PointRect) -> Result<(f32, f32), EngineError> {
    let (page_w, page_h) = geo.display_size();
    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
    if ![rect.left, rect.bottom, rect.right, rect.top, w, h]
        .iter()
        .all(|v| v.is_finite())
        || rect.left < 0.0
        || rect.bottom < 0.0
        || rect.right > page_w
        || rect.top > page_h
        || !(4.0..=10_000.0).contains(&w)
        || !(4.0..=10_000.0).contains(&h)
    {
        return Err(bad());
    }
    Ok((w, h))
}

pub(crate) fn rich_base(
    kind: &str,
    rect: PointRect,
    geo: &PageGeometry,
    author: Option<&str>,
) -> Result<Dictionary, EngineError> {
    if author.is_some_and(|a| a.len() > 4096 || a.chars().any(|c| c.is_control())) {
        return Err(bad());
    }
    let mut annotation = annot_write::base_annotation(kind, author)?;
    annotation.set("Rect", annot_write::rect_obj(geo.rect_to_user(rect)));
    annotation.set("SuniPDFRich", Object::Name(kind.as_bytes().to_vec()));
    Ok(annotation)
}

fn wrap_native(page: &mut PdfPage<'_>, text: &str, width: f32) -> Result<Vec<String>, EngineError> {
    let mut lines = Vec::new();
    for hard_line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let mut line = String::new();
        for word in hard_line.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if crate::paragraph::measure(page, &candidate)? <= width {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for character in word.chars() {
                let candidate = format!("{line}{character}");
                if crate::paragraph::measure(page, &candidate)? > width {
                    if line.is_empty() {
                        return Err(EngineError::Other(
                            "a FreeText glyph exceeds the chosen width".into(),
                        ));
                    }
                    lines.push(std::mem::take(&mut line));
                    line.push(character);
                } else {
                    line = candidate;
                }
                if lines.len() > 128 {
                    return Err(bad());
                }
            }
        }
        lines.push(line);
        if lines.len() > 128 {
            return Err(bad());
        }
    }
    Ok(lines)
}

pub(crate) fn vertex_annotation_components(
    geo: &PageGeometry,
    kind: VertexAnnotKind,
    points: &[(f32, f32)],
    style: &AnnotStyle,
    author: Option<&str>,
) -> Result<(Dictionary, Stream), EngineError> {
    let closed = kind != VertexAnnotKind::Polyline;
    if points.len() < if closed { 3 } else { 2 }
        || points.len() > 512
        || !style.width_pt.is_finite()
        || !(0.1..=50.0).contains(&style.width_pt)
        || !style.opacity.is_finite()
        || !(0.0..=1.0).contains(&style.opacity)
        || (!closed && style.fill_rgb.is_some())
    {
        return Err(bad());
    }
    annot_write::validate_dash_pattern(&style.dash_pattern)?;
    let cloud = kind == VertexAnnotKind::Cloud;
    let margin = style.width_pt / 2.0 + if cloud { 4.5 } else { 0.5 };
    let mut rect = PointRect {
        left: f32::INFINITY,
        bottom: f32::INFINITY,
        right: f32::NEG_INFINITY,
        top: f32::NEG_INFINITY,
    };
    let mut length = 0.0;
    for (index, point) in points.iter().enumerate() {
        if !point.0.is_finite() || !point.1.is_finite() {
            return Err(bad());
        }
        rect.left = rect.left.min(point.0);
        rect.right = rect.right.max(point.0);
        rect.bottom = rect.bottom.min(point.1);
        rect.top = rect.top.max(point.1);
        if index > 0 || closed {
            let previous = points[(index + points.len() - 1) % points.len()];
            let distance = (point.0 - previous.0).hypot(point.1 - previous.1);
            if distance < 0.01 {
                return Err(bad());
            }
            length += distance;
        }
    }
    if length > 50_000.0 {
        return Err(bad());
    }
    if !style.dash_pattern.is_empty() {
        let cycle: f64 = style
            .dash_pattern
            .iter()
            .map(|value| f64::from(*value))
            .sum();
        if 2.0 * f64::from(length) / cycle * style.dash_pattern.len() as f64 > 100_000.0 {
            return Err(bad());
        }
    }
    if cloud {
        let area: f32 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .map(|(a, b)| a.0 * b.1 - a.1 * b.0)
            .sum();
        if !area.is_finite() || area.abs() < 0.01 {
            return Err(bad());
        }
    }
    rect.left -= margin;
    rect.bottom -= margin;
    rect.right += margin;
    rect.top += margin;
    let (w, h) = checked_rect(geo, rect)?;
    let mut annotation = rich_base(
        if closed { "Polygon" } else { "PolyLine" },
        rect,
        geo,
        author,
    )?;
    annotation.set(
        "Vertices",
        annot_write::nums(
            &points
                .iter()
                .flat_map(|&(x, y)| {
                    let (x, y) = geo.to_user(x, y);
                    [x, y]
                })
                .collect::<Vec<_>>(),
        ),
    );
    annotation.set("C", annot_write::rgb_obj(style.rgb));
    if let Some(fill) = style.fill_rgb {
        annotation.set("IC", annot_write::rgb_obj(fill));
    }
    annotation.set("CA", style.opacity);
    annotation.set(
        "BS",
        annot_write::border_style(style.width_pt, &style.dash_pattern),
    );
    if cloud {
        annotation.set("IT", Object::Name(b"PolygonCloud".to_vec()));
        annotation.set("BE", dictionary! {"S"=>"C", "I"=>2});
    }
    let local: Vec<_> = points
        .iter()
        .map(|&(x, y)| (x - rect.left, y - rect.bottom))
        .collect();
    let ap = crate::appearance::vertices_ap(w, h, &local, closed, cloud, style);
    Ok((annotation, annot_write::ap_stream(&ap, geo)))
}

impl<'a> Document<'a> {
    pub(crate) fn rich_text_source(
        &self,
        w: f32,
        h: f32,
        text: &str,
        style: &RichFreeTextStyle,
    ) -> Result<Vec<u8>, EngineError> {
        if text.chars().any(|character|matches!(character as u32,0x0300..=0x036F|0x0590..=0x109F|0x1100..=0x11FF|0x1780..=0x18AF|0x1AB0..=0x1AFF|0x1DC0..=0x1DFF|0x200C..=0x200F|0x202A..=0x202E|0x2066..=0x2069|0x20D0..=0x20FF|0x3099..=0x309A|0xA960..=0xA97F|0xD7B0..=0xD7FF|0xFE00..=0xFE0F|0xFE20..=0xFE2F|0xE0100..=0xE01EF)) {
            return Err(EngineError::Other("rich FreeText currently supports horizontal Latin/CJK glyph runs; joining, bidirectional, combining and variation sequences require a shaping editor".into()));
        }
        if text.trim().is_empty()
            || text.chars().count() > 4096
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            || !(4.0..=200.0).contains(&style.text.size_pt)
            || !(0.0..=50.0).contains(&style.border_pt)
            || !style.text.opacity.is_finite()
            || !(0.0..=1.0).contains(&style.text.opacity)
        {
            return Err(bad());
        }
        let pad = crate::appearance::free_text_pad(style.border_pt);
        if w - 2.0 * pad < style.text.size_pt || h - 2.0 * pad < style.text.size_pt {
            return Err(bad());
        }
        let mut info = self.info.clone();
        info.page_sizes = vec![crate::PageSize {
            width_pt: w,
            height_pt: h,
        }];
        info.page_labels = vec![None];
        info.form = crate::FormKind::None;
        let mut tmp = Document {
            inner: self.pdfium.create_new_pdf()?,
            pdfium: self.pdfium,
            info,
            page_rotations: vec![None],
            password: None,
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        drop(
            tmp.inner
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::Custom(
                    PdfPoints::new(w),
                    PdfPoints::new(h),
                ))?,
        );
        let glyphs: String = text
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        tmp.add_text(0, 0.0, 0.0, &glyphs, &style.text)?;
        let mut probe = tmp.page_handle(0)?;
        let lines = wrap_native(&mut probe, text, w - 2.0 * pad)?;
        let needed =
            style.text.size_pt * (1.2 * (lines.len().saturating_sub(1) as f32) + 1.1) + 2.0 * pad;
        if needed > h {
            return Err(EngineError::Other(
                "FreeText does not fit the rectangle; enlarge it or reduce the font size".into(),
            ));
        }
        let first = probe.objects().get(0)?;
        let font = first.as_text_object().ok_or_else(bad)?.font().token();
        drop(first);
        let mut objects = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            crate::paragraph::measure(&mut probe, line)?;
            let text_page = probe.text()?;
            let current = probe.objects().get(0)?;
            let chars = current
                .as_text_object()
                .ok_or_else(bad)?
                .chars(&text_page)?;
            let mut left = 0.0f32;
            for index in 0..chars.len() {
                left = left.min(chars.get(index)?.loose_bounds()?.left().value);
            }
            let mut object =
                PdfPageTextObject::new(&tmp.inner, line, font, PdfPoints::new(style.text.size_pt))?;
            object.set_fill_color(PdfColor::new(
                style.text.rgb.0,
                style.text.rgb.1,
                style.text.rgb.2,
                (style.text.opacity * 255.0).round() as u8,
            ))?;
            object.translate(
                PdfPoints::new(pad - left),
                PdfPoints::new(
                    h - pad - style.text.size_pt - index as f32 * style.text.size_pt * 1.2,
                ),
            )?;
            objects.push(object);
        }
        probe.objects_mut().remove_object_at_index(0)?;
        crate::edit::regenerate_content(&mut probe)?;
        drop(probe);
        if style.fill_rgb.is_some() || style.border_pt > 0.0 {
            let border = style.border_pt / 2.0;
            tmp.add_shape(
                0,
                crate::ShapeKind::Rectangle,
                PointRect {
                    left: border,
                    bottom: border,
                    right: w - border,
                    top: h - border,
                },
                &crate::ShapeStyle {
                    stroke_rgb: style.text.rgb,
                    stroke_width: style.border_pt,
                    fill_rgb: style.fill_rgb,
                    opacity: style.text.opacity,
                },
            )?;
        }
        let mut page = tmp.page_handle(0)?;
        for object in objects {
            page.objects_mut().add_text_object(object)?;
        }
        crate::edit::regenerate_content(&mut page)?;
        drop(page);
        for object in tmp.page_objects(0)? {
            // The authored background/border is deliberately clipped by the appearance BBox.
            // PDFium reports conservative stroked-path bounds; only glyphs must fit unclipped.
            if object.kind == crate::ObjectKind::Path {
                continue;
            }
            if object.kind != crate::ObjectKind::Text {
                return Err(bad());
            }
            if object.bounds.left < -0.01
                || object.bounds.bottom < -0.01
                || object.bounds.right > w + 0.01
                || object.bounds.top > h + 0.01
            {
                return Err(EngineError::Other(format!(
                    "FreeText glyph bounds {:?} exceed its {w} x {h} pt appearance",
                    object.bounds
                )));
            }
        }
        let bytes = tmp.snapshot()?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(bad());
        }
        Ok(bytes)
    }

    pub fn add_rich_free_text_annotation(
        &mut self,
        page_index: usize,
        index: Option<usize>,
        rect: PointRect,
        text: &str,
        style: &RichFreeTextStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let (w, h) = checked_rect(&geo, rect)?;
        let source = crate::security::load(&self.rich_text_source(w, h, text, style)?, None)?;
        let source_page = source.get_pages()[&1];
        let content = source
            .get_page_content_with_limit(source_page, 32 * 1024 * 1024)
            .map_err(lopdf_err)?;
        let resources = source
            .get_dictionary(source_page)
            .map_err(lopdf_err)?
            .get(b"Resources")
            .map_err(lopdf_err)?
            .clone();
        self.edit_annotation_pdf(page_index, |pdf, page, geo| {
            let imported = crate::forms::deep_copy(&source, pdf, &HashMap::new(), &mut HashMap::new(), 0, &resources, 32)?;
            let (a, b) = geo.vec_to_user(1.0, 0.0);
            let (c, d) = geo.vec_to_user(0.0, 1.0);
            let stream = Stream::new(dictionary! { "Type"=>"XObject", "Subtype"=>"Form", "BBox"=>annot_write::nums(&[0.0, 0.0, w, h]), "Matrix"=>annot_write::nums(&[a,b,c,d,0.0,0.0]), "Resources"=>imported.clone() }, content);
            let mut annotation = match index {
                None => rich_base("FreeText", rect, geo, author)?,
                Some(index) => {
                    let (_, annotation) = annot_write::load_annot(pdf, page, index)?;
                    annot_write::check_unlocked(&annotation)?;
                    if annotation.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"FreeText".as_slice()) || !annotation.get(b"NM").and_then(lopdf::decode_text_string).is_ok_and(|name| name.starts_with(annot_write::NAME_PREFIX)) { return Err(bad()); }
                    annotation
                }
            };
            annotation.set("Rect", annot_write::rect_obj(geo.rect_to_user(rect)));
            annotation.set("SuniPDFRich", Object::Name(b"FreeText".to_vec()));
            annotation.set("Contents", lopdf::text_string(text));
            annotation.set("CA", style.text.opacity);
            annotation.set("BS", annot_write::border_style(style.border_pt, &[]));
            if let Some(fill) = style.fill_rgb { annotation.set("C", annot_write::rgb_obj(fill)); } else { annotation.remove(b"C"); }
            let (_, resources) = pdf.dereference(&imported).map_err(lopdf_err)?;
            let (_, fonts) = pdf.dereference(resources.as_dict().map_err(lopdf_err)?.get(b"Font").map_err(lopdf_err)?).map_err(lopdf_err)?;
            let font = fonts.as_dict().map_err(lopdf_err)?.iter().next().ok_or_else(bad)?.0;
            let font = String::from_utf8(font.clone()).map_err(|_| bad())?;
            annotation.set("DA", Object::string_literal(format!("/{font} {} Tf {} {} {} rg", style.text.size_pt, style.text.rgb.0 as f32 / 255.0, style.text.rgb.1 as f32 / 255.0, style.text.rgb.2 as f32 / 255.0)));
            if let Some(index) = index {
                let (slot, _) = annot_write::load_annot(pdf, page, index)?;
                annot_write::set_ap(pdf, &mut annotation, stream);
                annot_write::touch(&mut annotation);
                annot_write::store_annot(pdf, page, slot, annotation)?;
                Ok(index)
            } else { annot_write::push_annotation(pdf, page, annotation, Some(stream)) }
        })
    }

    pub fn add_vertex_annotation(
        &mut self,
        page_index: usize,
        kind: VertexAnnotKind,
        points: &[(f32, f32)],
        style: &AnnotStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        self.edit_annotation_pdf(page_index, |pdf, page, geo| {
            let (annotation, stream) =
                vertex_annotation_components(geo, kind, points, style, author)?;
            annot_write::push_annotation(pdf, page, annotation, Some(stream))
        })
    }

    pub fn add_stamp_annotation(
        &mut self,
        page_index: usize,
        rect: PointRect,
        kind: StampKind,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        self.edit_annotation_pdf(page_index, |pdf, page, geo| {
            let (w, h) = checked_rect(geo, rect)?;
            if w < 40.0 || h < 16.0 {
                return Err(bad());
            }
            let mut annotation = rich_base("Stamp", rect, geo, author)?;
            annotation.set("Name", Object::Name(kind.label().as_bytes().to_vec()));
            annotation.set("Contents", lopdf::text_string(kind.label()));
            annot_write::push_annotation(
                pdf,
                page,
                annotation,
                Some(annot_write::ap_stream(
                    &crate::appearance::stamp_ap(w, h, kind.label()),
                    geo,
                )),
            )
        })
    }

    pub fn add_file_attachment_annotation(
        &mut self,
        page_index: usize,
        rect: PointRect,
        filename: &str,
        bytes: &[u8],
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if filename.is_empty()
            || filename.chars().count() > 255
            || filename
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
            || matches!(filename, "." | "..")
            || bytes.len() > 16 * 1024 * 1024
        {
            return Err(bad());
        }
        self.edit_annotation_pdf(page_index, |pdf,page,geo| { let (w,h)=checked_rect(geo,rect)?; if w<12.0 || h<12.0 || w>96.0 || h>96.0 { return Err(bad()); } let embedded=pdf.add_object(Stream::new(dictionary! {"Type"=>"EmbeddedFile", "Params"=>dictionary! {"Size"=>bytes.len() as i64}},bytes.to_vec())); let fallback:String=filename.chars().map(|c| if c.is_ascii() {c} else {'_'}).collect(); let spec=pdf.add_object(dictionary! {"Type"=>"Filespec", "F"=>Object::string_literal(fallback), "UF"=>lopdf::text_string(filename), "EF"=>dictionary! {"F"=>embedded,"UF"=>embedded}}); let mut annotation=rich_base("FileAttachment",rect,geo,author)?; annotation.set("Name",Object::Name(b"Paperclip".to_vec())); annotation.set("FS",spec); annotation.set("Contents",lopdf::text_string(filename)); annot_write::push_annotation(pdf,page,annotation,Some(annot_write::ap_stream(&crate::appearance::attachment_ap(w,h),geo))) })
    }

    pub fn apply_rich_annotation(
        &mut self,
        page: usize,
        annotation: &RichAnnotation,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        match annotation {
            RichAnnotation::FreeText {
                index,
                rect,
                text,
                style,
            } => self.add_rich_free_text_annotation(page, *index, *rect, text, style, author),
            RichAnnotation::Vertices {
                kind,
                points,
                style,
            } => self.add_vertex_annotation(page, *kind, points, style, author),
            RichAnnotation::Stamp { rect, kind } => {
                self.add_stamp_annotation(page, *rect, *kind, author)
            }
            RichAnnotation::ImageStamp {
                rect,
                image,
                opacity,
            } => self.add_image_stamp_annotation(page, *rect, image, *opacity, author),
            RichAnnotation::LibraryPlacement { entry, placement } => self
                .place_library_entry(page, entry, *placement, author)?
                .first()
                .copied()
                .ok_or_else(bad),
            RichAnnotation::Measurement(measurement) => {
                self.add_measurement_annotation(page, measurement, author)
            }
            RichAnnotation::DimensionedRectangle(request) => {
                self.add_dimensioned_rectangle(page, request, author)
            }
            RichAnnotation::DimensionedSegment(request) => {
                self.add_dimensioned_segment(page, request, author)
            }
            RichAnnotation::DimensionedEllipse(request) => {
                self.add_dimensioned_ellipse(page, request, author)
            }
            RichAnnotation::UpdateMeasurement {
                target,
                measurement,
            } => {
                if target.page() != page {
                    return Err(EngineError::Other(
                        "measurement belongs to another page".into(),
                    ));
                }
                self.update_measurement_annotation(target, measurement)
            }
            RichAnnotation::RecalibrateMeasurement {
                target,
                measurement,
                calibration,
            } => {
                if target.page() != page {
                    return Err(EngineError::Other(
                        "measurement belongs to another page".into(),
                    ));
                }
                self.recalibrate_measurement_annotation(target, measurement, calibration)
            }
            RichAnnotation::Attachment {
                rect,
                filename,
                bytes,
            } => self.add_file_attachment_annotation(page, *rect, filename, bytes, author),
            RichAnnotation::EraseInk {
                index,
                path,
                radius,
            } => {
                self.erase_ink_annotation(page, *index, path, *radius)?;
                Ok(*index)
            }
        }
    }

    pub fn preview_rich_annotation(
        &self,
        page: usize,
        annotation: &RichAnnotation,
        author: Option<&str>,
    ) -> Result<RichAnnotationPreview, EngineError> {
        let original = self.snapshot()?;
        if original.len() > 128 * 1024 * 1024 {
            return Err(EngineError::Other(
                "rich annotation preview is limited to documents up to 128 MiB".into(),
            ));
        }
        let mut scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(original.clone(), self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        let indices = if let RichAnnotation::LibraryPlacement { entry, placement } = annotation {
            scratch.apply_library_entry_in_place(page, entry, *placement, author)?
        } else {
            vec![scratch.apply_rich_annotation(page, annotation, author)?]
        };
        let index = *indices.first().ok_or_else(bad)?;
        let bounds = scratch
            .annotation_details(page)?
            .into_iter()
            .filter(|detail| indices.contains(&detail.index))
            .map(|detail| detail.rect)
            .reduce(|a, b| PointRect {
                left: a.left.min(b.left),
                bottom: a.bottom.min(b.bottom),
                right: a.right.max(b.right),
                top: a.top.max(b.top),
            })
            .ok_or_else(bad)?;
        let size = scratch.page_size(page)?;
        let scale = 2.0f32
            .min(1200.0 / (bounds.right - bounds.left + 8.0).max(bounds.top - bounds.bottom + 8.0));
        let (pw, ph) = crate::tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
        let x = ((bounds.left - 4.0).max(0.0) * scale).floor() as u32;
        let y = ((size.height_pt - bounds.top - 4.0).max(0.0) * scale).floor() as u32;
        let right = (((bounds.right + 4.0).min(size.width_pt) * scale).ceil() as u32).min(pw);
        let bottom = (((size.height_pt - bounds.bottom + 4.0).min(size.height_pt) * scale).ceil()
            as u32)
            .min(ph);
        let region = PixelRect::new(x, y, right - x, bottom - y);
        let candidate = scratch.snapshot()?;
        if candidate.len() > 160 * 1024 * 1024 {
            return Err(bad());
        }
        Ok(RichAnnotationPreview {
            image: scratch.render_region(page, scale, region)?,
            scale,
            region,
            index,
            prepared: Some(PreparedRichAnnotation {
                original,
                candidate,
                index,
            }),
        })
    }

    pub fn commit_rich_annotation_preview(
        &mut self,
        prepared: PreparedRichAnnotation,
    ) -> Result<usize, EngineError> {
        if !same_document(&prepared.original, &self.snapshot()?, self.password())? {
            return Err(EngineError::Other(
                "document changed since the rich annotation preview; preview again".into(),
            ));
        }
        self.restore(prepared.candidate)?;
        Ok(prepared.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_binding_ignores_orphans_but_checks_content_annotations_and_all_trailer_roots() {
        fn save(mut pdf: lopdf::Document) -> Vec<u8> {
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            bytes
        }
        let mut pdf =
            lopdf::Document::load_mem(include_bytes!("../tests/fixtures/two-pages.pdf")).unwrap();
        let page = pdf.get_pages()[&1];
        let info = pdf.add_object(dictionary! { "Title" => Object::string_literal("Original") });
        let extension = pdf.add_object(dictionary! { "Value" => 7 });
        let annotation = pdf.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Text", "Contents" => Object::string_literal("Keep"),
        });
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Annots", vec![Object::Reference(annotation)]);
        pdf.trailer.set("Info", info);
        pdf.trailer.set("PrivateRoot", extension);
        let original = save(pdf.clone());

        let mut unused = pdf.clone();
        let orphan = unused.add_object(Stream::new(
            Dictionary::new(),
            b"unused appearance".to_vec(),
        ));
        unused.trailer.set(
            "ID",
            vec![
                Object::string_literal("new-first"),
                Object::string_literal("new-second"),
            ],
        );
        assert!(same_document(&original, &save(unused.clone()), None).unwrap());
        unused.trailer.set("AnotherRoot", orphan);
        assert!(!same_document(&original, &save(unused), None).unwrap());

        for root in [info, extension, annotation] {
            let mut changed = pdf.clone();
            changed
                .get_dictionary_mut(root)
                .unwrap()
                .set("Changed", true);
            assert!(!same_document(&original, &save(changed), None).unwrap());
        }
        let mut changed = pdf.clone();
        let contents = changed.add_object(Stream::new(
            Dictionary::new(),
            b"q 2 0 0 2 0 0 cm Q".to_vec(),
        ));
        changed
            .get_dictionary_mut(page)
            .unwrap()
            .set("Contents", contents);
        assert!(!same_document(&original, &save(changed), None).unwrap());
        let mut changed = pdf;
        changed.trailer.set("CustomValue", 42);
        assert!(!same_document(&original, &save(changed), None).unwrap());
    }

    #[test]
    fn prepared_binding_graph_bounds_cycles_and_refuses_missing_or_hidden_references() {
        let mut pdf = lopdf::Document::new();
        let first = pdf.add_object(dictionary! {});
        let second = pdf.add_object(dictionary! { "Back" => first });
        pdf.get_dictionary_mut(first).unwrap().set("Next", second);
        pdf.trailer.set("Root", first);
        assert_eq!(
            prepared_document_graph(pdf.clone()).unwrap().objects.len(),
            2
        );

        let mut missing = pdf.clone();
        missing
            .trailer
            .set("PrivateRoot", Object::Reference((999, 0)));
        assert!(prepared_document_graph(missing).is_err());
        let mut missing = pdf.clone();
        missing
            .get_dictionary_mut(second)
            .unwrap()
            .set("Missing", Object::Reference((999, 0)));
        assert!(prepared_document_graph(missing).is_err());
        let mut hidden = pdf.clone();
        hidden.trailer.set("Size", second);
        assert!(prepared_document_graph(hidden).is_err());
        let mut hidden = pdf.clone();
        hidden
            .trailer
            .set("ID", vec![Object::Reference(second), Object::Null]);
        assert!(prepared_document_graph(hidden).is_err());

        let mut deep = Object::Null;
        for _ in 0..66 {
            deep = Object::Array(vec![deep]);
        }
        pdf.trailer.set("Nested", deep);
        assert!(prepared_document_graph(pdf).is_err());
    }

    #[test]
    fn prepared_binding_refuses_identical_content_with_different_protection() {
        let input = include_bytes!("../tests/fixtures/two-pages.pdf");
        let a = crate::security::protect(
            input,
            None,
            &crate::Protection {
                user_password: "same-user".into(),
                owner_password: "first-owner".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let b = crate::security::protect(
            input,
            None,
            &crate::Protection {
                user_password: "same-user".into(),
                owner_password: "second-owner".into(),
                allow_print: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(same_document(&a, &a, Some("same-user")).unwrap());
        assert!(
            !same_document(&a, &b, Some("same-user")).unwrap(),
            "the encryption dictionary, key identity and permissions belong to the prepared source"
        );
        assert!(!same_document(input, &a, Some("same-user")).unwrap());
    }

    #[test]
    fn prepared_binding_tracks_auto_decrypted_empty_user_protection() {
        let input = include_bytes!("../tests/fixtures/two-pages.pdf");
        for aes256 in [false, true] {
            let protection = crate::Protection {
                user_password: String::new(),
                owner_password: "first-owner".into(),
                aes256,
                ..Default::default()
            };
            let original = crate::security::protect(input, None, &protection).unwrap();
            let parsed = lopdf::Document::load_mem(&original).unwrap();
            assert!(!parsed.is_encrypted());
            assert!(parsed.encryption_state.is_some());
            assert!(same_document(&original, &original, None).unwrap());
            let resaved =
                crate::security::save(crate::security::load(&original, None).unwrap()).unwrap();
            assert!(same_document(&original, &resaved, None).unwrap());
            assert!(!same_document(input, &original, None).unwrap());
            for change_owner in [false, true] {
                let replacement = crate::Protection {
                    owner_password: if change_owner {
                        "second-owner".into()
                    } else {
                        protection.owner_password.clone()
                    },
                    allow_print: change_owner,
                    ..protection.clone()
                };
                let reprotected = crate::security::protect(input, None, &replacement).unwrap();
                assert!(
                    !same_document(&original, &reprotected, None).unwrap(),
                    "aes256={aes256}, changed_owner={change_owner}"
                );
            }
        }
    }

    #[test]
    fn eraser_removes_only_touched_whole_strokes_and_preserves_control_points() {
        let a = vec![(10.0, 10.0), (30.0, 30.0), (50.0, 10.0)];
        let b = vec![(10.0, 50.0), (50.0, 50.0)];
        let strokes = vec![a, b.clone()];
        assert_eq!(
            erase_touched_strokes(&strokes, &[(30.0, 23.0)], 1.0, 2.0).unwrap(),
            vec![b]
        );
        assert!(erase_touched_strokes(&strokes, &[(0.0, 100.0)], 1.0, 2.0).is_err());
        assert!(
            erase_touched_strokes(&strokes, &[(30.0, 0.0), (30.0, 70.0)], 1.0, 2.0).is_err(),
            "empty result needs explicit Delete"
        );
        assert!(erase_touched_strokes(&strokes, &[(f32::NAN, 0.0)], 1.0, 2.0).is_err());
    }
}
