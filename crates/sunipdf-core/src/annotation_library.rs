//! Portable, bounded annotation templates. No PDF objects, actions or external resources are imported.

use crate::annot_rich::{RichAnnotation, RichFreeTextStyle, StampKind, VertexAnnotKind};
use crate::{AnnotShape, AnnotStyle, Document, EngineError, FontFamily, PointRect, TextStyle};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

mod capture_stamps;
mod capture_text;

pub const MAX_LIBRARY_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LIBRARY_ENTRIES: usize = 64;
pub const MAX_GROUP_PRIMITIVES: usize = 16;
pub const MAX_PNG_BYTES: usize = 4 * 1024 * 1024;
const FORMAT: &str = "sunipdf-annotation-library";

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("invalid annotation library: {message}"))
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn finite(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}

fn text_valid(text: &str, max: usize, multiline: bool) -> bool {
    text.len() <= max
        && !text
            .chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LibraryProvenance {
    pub author: Option<String>,
    pub source: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryShape {
    Square,
    Circle,
    Line,
    Arrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryVertexKind {
    Polygon,
    Polyline,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryStamp {
    Approved,
    Draft,
    Confidential,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryFont {
    Helvetica,
    Times,
    Courier,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryStyle {
    pub rgb: [u8; 3],
    pub fill_rgb: Option<[u8; 3]>,
    pub width_pt: f32,
    pub opacity: f32,
    pub dash_pattern: Vec<f32>,
}

impl Default for LibraryStyle {
    fn default() -> Self {
        Self::from(&AnnotStyle::default())
    }
}

impl From<&AnnotStyle> for LibraryStyle {
    fn from(s: &AnnotStyle) -> Self {
        Self {
            rgb: s.rgb.into(),
            fill_rgb: s.fill_rgb.map(Into::into),
            width_pt: s.width_pt,
            opacity: s.opacity,
            dash_pattern: s.dash_pattern.clone(),
        }
    }
}

impl LibraryStyle {
    fn validate(&self) -> Result<(), EngineError> {
        if !finite(self.width_pt, 0.1, 50.0) || !finite(self.opacity, 0.01, 1.0) {
            return Err(invalid("stroke style is outside supported bounds"));
        }
        crate::annot_write::validate_dash_pattern(&self.dash_pattern)?;
        Ok(())
    }

    fn placed(&self, scale: f32) -> AnnotStyle {
        AnnotStyle {
            rgb: self.rgb.into(),
            fill_rgb: self.fill_rgb.map(Into::into),
            width_pt: self.width_pt * scale,
            opacity: self.opacity,
            dash_pattern: self.dash_pattern.iter().map(|v| v * scale).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryTextStyle {
    pub family: LibraryFont,
    pub size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub rgb: [u8; 3],
    pub opacity: f32,
    pub fill_rgb: Option<[u8; 3]>,
    pub border_pt: f32,
}

impl Default for LibraryTextStyle {
    fn default() -> Self {
        Self {
            family: LibraryFont::Helvetica,
            size_pt: 12.0,
            bold: false,
            italic: false,
            rgb: [0, 0, 0],
            opacity: 1.0,
            fill_rgb: None,
            border_pt: 0.0,
        }
    }
}

impl LibraryTextStyle {
    fn validate(&self) -> Result<(), EngineError> {
        if !finite(self.size_pt, 4.0, 200.0)
            || !finite(self.opacity, 0.01, 1.0)
            || !finite(self.border_pt, 0.0, 50.0)
        {
            return Err(invalid("text style is outside supported bounds"));
        }
        Ok(())
    }

    fn placed(&self, scale: f32) -> RichFreeTextStyle {
        RichFreeTextStyle {
            text: TextStyle {
                family: match self.family {
                    LibraryFont::Helvetica => FontFamily::Helvetica,
                    LibraryFont::Times => FontFamily::Times,
                    LibraryFont::Courier => FontFamily::Courier,
                },
                size_pt: self.size_pt * scale,
                bold: self.bold,
                italic: self.italic,
                rgb: self.rgb.into(),
                opacity: self.opacity,
                font_file: None,
                font_face_index: 0,
            },
            fill_rgb: self.fill_rgb.map(Into::into),
            border_pt: self.border_pt * scale,
        }
    }
}

/// Self-contained PNG; construction strips ancillary metadata by re-encoding decoded pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedPng {
    png_base64: String,
    width: u32,
    height: u32,
    sha256: String,
}

impl EmbeddedPng {
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > MAX_PNG_BYTES {
            return Err(invalid("PNG exceeds 4 MiB"));
        }
        let image = crate::image_stamp::decode_png(bytes, 1.0)?;
        let (width, height) = image.dimensions();
        let mut canonical = Cursor::new(Vec::new());
        image
            .write_to(&mut canonical, image::ImageFormat::Png)
            .map_err(|_| invalid("PNG encoding failed"))?;
        let bytes = canonical.into_inner();
        if bytes.len() > MAX_PNG_BYTES {
            return Err(invalid("encoded PNG exceeds 4 MiB"));
        }
        Ok(Self {
            png_base64: BASE64_STANDARD.encode(&bytes),
            width,
            height,
            sha256: digest_hex(&bytes),
        })
    }

    pub fn png_bytes(&self) -> Result<Vec<u8>, EngineError> {
        if self.png_base64.len() > MAX_PNG_BYTES.div_ceil(3) * 4 || self.sha256.len() != 64 {
            return Err(invalid("invalid embedded PNG size or digest"));
        }
        let bytes = BASE64_STANDARD
            .decode(&self.png_base64)
            .map_err(|_| invalid("invalid PNG base64"))?;
        if bytes.len() > MAX_PNG_BYTES || digest_hex(&bytes) != self.sha256 {
            return Err(invalid("PNG digest or byte count mismatch"));
        }
        let decoded = crate::image_stamp::decode_png(&bytes, 1.0)?;
        if decoded.dimensions() != (self.width, self.height) {
            return Err(invalid("PNG dimensions mismatch"));
        }
        Ok(bytes)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LibraryPrimitive {
    Shape {
        shape: LibraryShape,
        rect: [f32; 4],
        style: LibraryStyle,
    },
    FreeText {
        rect: [f32; 4],
        text: String,
        style: LibraryTextStyle,
    },
    Vertices {
        vertex_kind: LibraryVertexKind,
        points: Vec<(f32, f32)>,
        style: LibraryStyle,
    },
    Stamp {
        rect: [f32; 4],
        stamp: LibraryStamp,
    },
    Image {
        rect: [f32; 4],
        png: EmbeddedPng,
        opacity: f32,
    },
}

impl LibraryPrimitive {
    fn validate(&self) -> Result<(), EngineError> {
        let bounds = self.bounds();
        if bounds.iter().any(|&v| !finite(v, 0.0, 10000.0)) {
            return Err(invalid(
                "template coordinates must be finite and within 0–10000 points",
            ));
        }
        match self {
            Self::Shape { shape, rect, style } => {
                style.validate()?;
                if rect.iter().any(|&v| !finite(v, 0.0, 10000.0)) {
                    return Err(invalid("invalid shape coordinates"));
                }
                if matches!(shape, LibraryShape::Line | LibraryShape::Arrow) {
                    if style.fill_rgb.is_some() {
                        return Err(invalid("line styles cannot contain a fill"));
                    }
                    if (rect[2] - rect[0]).hypot(rect[3] - rect[1]) < 0.1 {
                        return Err(invalid("line is empty"));
                    }
                } else {
                    validate_rect(*rect)?;
                }
            }
            Self::FreeText { rect, text, style } => {
                validate_rect(*rect)?;
                style.validate()?;
                if text.trim().is_empty() || !text_valid(text, 4096, true) {
                    return Err(invalid("text is empty or too long"));
                }
            }
            Self::Vertices {
                vertex_kind,
                points,
                style,
            } => {
                style.validate()?;
                let min = if *vertex_kind == LibraryVertexKind::Polyline {
                    2
                } else {
                    3
                };
                if *vertex_kind == LibraryVertexKind::Polyline && style.fill_rgb.is_some() {
                    return Err(invalid("polyline styles cannot contain a fill"));
                }
                if !(min..=128).contains(&points.len())
                    || points
                        .iter()
                        .any(|&(x, y)| !finite(x, 0.0, 10000.0) || !finite(y, 0.0, 10000.0))
                {
                    return Err(invalid("invalid vertex count or coordinates"));
                }
                let distance = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).hypot(a.1 - b.1);
                if points.windows(2).any(|p| distance(p[0], p[1]) < 0.01)
                    || (*vertex_kind != LibraryVertexKind::Polyline
                        && distance(points[0], points[points.len() - 1]) < 0.01)
                {
                    return Err(invalid("consecutive vertices must be distinct"));
                }
            }
            Self::Stamp { rect, .. } => {
                validate_rect(*rect)?;
                if rect[2] - rect[0] < 40.0 || rect[3] - rect[1] < 16.0 {
                    return Err(invalid("stamp requires at least 40 by 16 points"));
                }
            }
            Self::Image { rect, png, opacity } => {
                validate_rect(*rect)?;
                if !finite(*opacity, 0.01, 1.0) {
                    return Err(invalid("image opacity is outside supported bounds"));
                }
                png.png_bytes()?;
            }
        }
        Ok(())
    }

    fn bounds(&self) -> [f32; 4] {
        match self {
            Self::Vertices { points, .. } => points.iter().fold(
                [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
                |b, &(x, y)| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
            ),
            Self::Shape { rect, .. }
            | Self::FreeText { rect, .. }
            | Self::Stamp { rect, .. }
            | Self::Image { rect, .. } => [
                rect[0].min(rect[2]),
                rect[1].min(rect[3]),
                rect[0].max(rect[2]),
                rect[1].max(rect[3]),
            ],
        }
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Result<Self, EngineError> {
        let mut result = self.clone();
        result.translate(dx, dy);
        result.validate()?;
        if matches!(self, Self::FreeText { .. }) {
            capture_text::preserve_dimensions(self.bounds(), result.bounds(), 1.0)?;
        }
        Ok(result)
    }

    fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Self::Vertices { points, .. } => {
                for p in points {
                    p.0 += dx;
                    p.1 += dy;
                }
            }
            Self::Shape { rect, .. }
            | Self::FreeText { rect, .. }
            | Self::Stamp { rect, .. }
            | Self::Image { rect, .. } => {
                rect[0] += dx;
                rect[2] += dx;
                rect[1] += dy;
                rect[3] += dy;
            }
        }
    }
}

fn validate_rect(rect: [f32; 4]) -> Result<(), EngineError> {
    if rect.iter().any(|&v| !finite(v, 0.0, 10000.0))
        || !finite(rect[2] - rect[0], 4.0, 10000.0)
        || !finite(rect[3] - rect[1], 4.0, 10000.0)
    {
        return Err(invalid(
            "rectangle must have positive width and height of at least 4 points",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    pub provenance: LibraryProvenance,
    pub primitives: Vec<LibraryPrimitive>,
}

impl LibraryEntry {
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > MAX_LIBRARY_BYTES {
            return Err(invalid("entry exceeds 16 MiB"));
        }
        let result: Self = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(invalid(
                "ID must contain 1–64 ASCII letters, digits, underscores or hyphens",
            ));
        }
        if self.name.trim() != self.name
            || self.name.is_empty()
            || !text_valid(&self.name, 128, false)
        {
            return Err(invalid("invalid entry name"));
        }
        for value in [
            &self.provenance.author,
            &self.provenance.source,
            &self.provenance.license,
        ]
        .into_iter()
        .flatten()
        {
            if value.trim().is_empty() || !text_valid(value, 1024, false) {
                return Err(invalid("invalid declared provenance"));
            }
        }
        if !(1..=MAX_GROUP_PRIMITIVES).contains(&self.primitives.len()) {
            return Err(invalid("entry must contain 1–16 primitives"));
        }
        let mut image_bytes = 0usize;
        let mut pixels = 0u64;
        for p in &self.primitives {
            if let LibraryPrimitive::Image { png, .. } = p {
                image_bytes = image_bytes.saturating_add(png.png_base64.len());
                pixels = pixels.saturating_add(u64::from(png.width) * u64::from(png.height));
            }
            if image_bytes > 12 * 1024 * 1024 || pixels > 16_000_000 {
                return Err(invalid("entry image budget exceeded"));
            }
            p.validate()?;
        }
        Ok(())
    }

    pub fn bounds(&self) -> Result<[f32; 4], EngineError> {
        self.validate()?;
        Ok(self
            .primitives
            .iter()
            .map(LibraryPrimitive::bounds)
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            })
            .expect("validated nonempty entry"))
    }

    pub fn normalized(&self) -> Result<Self, EngineError> {
        let bounds = self.bounds()?;
        let mut result = self.clone();
        for p in &mut result.primitives {
            let original = p.bounds();
            p.translate(-bounds[0], -bounds[1]);
            if matches!(p, LibraryPrimitive::FreeText { .. }) {
                capture_text::preserve_dimensions(original, p.bounds(), 1.0)?;
            }
        }
        result.validate()?;
        Ok(result)
    }

    /// Captures only explicitly supplied new drafts; edit targets and document identities are refused.
    pub fn from_rich_annotation(
        id: String,
        name: String,
        provenance: LibraryProvenance,
        annotation: &RichAnnotation,
    ) -> Result<Self, EngineError> {
        let rect = |r: &PointRect| [r.left, r.bottom, r.right, r.top];
        let primitive = match annotation {
            RichAnnotation::FreeText {
                index: None,
                rect: r,
                text,
                style,
            } if style.text.font_file.is_none() && style.text.font_face_index == 0 => {
                LibraryPrimitive::FreeText {
                    rect: rect(r),
                    text: text.clone(),
                    style: LibraryTextStyle {
                        family: match style.text.family {
                            FontFamily::Helvetica => LibraryFont::Helvetica,
                            FontFamily::Times => LibraryFont::Times,
                            FontFamily::Courier => LibraryFont::Courier,
                        },
                        size_pt: style.text.size_pt,
                        bold: style.text.bold,
                        italic: style.text.italic,
                        rgb: style.text.rgb.into(),
                        opacity: style.text.opacity,
                        fill_rgb: style.fill_rgb.map(Into::into),
                        border_pt: style.border_pt,
                    },
                }
            }
            RichAnnotation::Vertices {
                kind,
                points,
                style,
            } => LibraryPrimitive::Vertices {
                vertex_kind: match kind {
                    VertexAnnotKind::Polygon => LibraryVertexKind::Polygon,
                    VertexAnnotKind::Polyline => LibraryVertexKind::Polyline,
                    VertexAnnotKind::Cloud => LibraryVertexKind::Cloud,
                },
                points: points.clone(),
                style: style.into(),
            },
            RichAnnotation::Stamp { rect: r, kind } => LibraryPrimitive::Stamp {
                rect: rect(r),
                stamp: match kind {
                    StampKind::Approved => LibraryStamp::Approved,
                    StampKind::Draft => LibraryStamp::Draft,
                    StampKind::Confidential => LibraryStamp::Confidential,
                    StampKind::Rejected => LibraryStamp::Rejected,
                },
            },
            RichAnnotation::ImageStamp {
                rect: r,
                image,
                opacity,
            } => {
                let file = std::fs::File::open(image)?;
                let metadata = file.metadata()?;
                if !metadata.is_file() || metadata.len() > MAX_PNG_BYTES as u64 {
                    return Err(invalid("PNG exceeds 4 MiB"));
                }
                let mut bytes = Vec::new();
                file.take(MAX_PNG_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)?;
                LibraryPrimitive::Image {
                    rect: rect(r),
                    png: EmbeddedPng::from_png_bytes(&bytes)?,
                    opacity: *opacity,
                }
            }
            _ => {
                return Err(invalid(
                    "only new standard-font text, vertices, stamps and PNG drafts can be saved",
                ))
            }
        };
        Self {
            id,
            name,
            provenance,
            primitives: vec![primitive],
        }
        .normalized()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationLibrary {
    format: String,
    version: u32,
    entries: Vec<LibraryEntry>,
}

impl Default for AnnotationLibrary {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: 1,
            entries: Vec::new(),
        }
    }
}

impl AnnotationLibrary {
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > MAX_LIBRARY_BYTES {
            return Err(invalid("file exceeds 16 MiB"));
        }
        let result: Self = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), EngineError> {
        if self.format != FORMAT || self.version != 1 || self.entries.len() > MAX_LIBRARY_ENTRIES {
            return Err(invalid("unsupported format, version or entry count"));
        }
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        let mut image_bytes = 0usize;
        let mut pixels = 0u64;
        for e in &self.entries {
            if !ids.insert(e.id.clone()) || !names.insert(e.name.to_lowercase()) {
                return Err(invalid("entry IDs and names must be unique"));
            }
            for p in &e.primitives {
                if let LibraryPrimitive::Image { png, .. } = p {
                    image_bytes = image_bytes.saturating_add(png.png_base64.len());
                    pixels = pixels.saturating_add(u64::from(png.width) * u64::from(png.height));
                }
            }
            if image_bytes > 12 * 1024 * 1024 || pixels > 16_000_000 {
                return Err(invalid("library image budget exceeded"));
            }
            e.validate()?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<Vec<u8>, EngineError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| invalid(&e.to_string()))?;
        if bytes.len() > MAX_LIBRARY_BYTES {
            return Err(invalid("file exceeds 16 MiB"));
        }
        Ok(bytes)
    }

    pub fn entries(&self) -> &[LibraryEntry] {
        &self.entries
    }
    pub fn entry(&self, id: &str) -> Option<&LibraryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn insert(&mut self, entry: LibraryEntry) -> Result<(), EngineError> {
        let mut next = self.clone();
        next.entries.push(entry);
        next.to_json()?;
        *self = next;
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<LibraryEntry, EngineError> {
        let index = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| invalid("entry ID not found"))?;
        Ok(self.entries.remove(index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryPlacement {
    pub origin: (f32, f32),
    pub scale: f32,
}

/// A page annotation that may be copied into a portable group. Unavailable rows are retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryCaptureCandidate {
    pub index: usize,
    pub subtype: String,
    pub fingerprint: Option<String>,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryCaptureSelection {
    pub index: usize,
    pub fingerprint: String,
}

/// Validated native geometry in its original displayed coordinates. Unlike a library entry,
/// this representation has not been moved to a local origin.
#[derive(Debug, Clone)]
pub(crate) struct OriginalCapture {
    pub index: usize,
    pub subtype: String,
    pub fingerprint: Option<String>,
    pub source_nm: Option<String>,
    pub author: Option<String>,
    pub primitive: Option<LibraryPrimitive>,
    pub refusal: Option<String>,
}

fn capture_object(
    pdf: &lopdf::Document,
    object: &lopdf::Object,
    depth: usize,
    budget: &mut usize,
) -> Result<serde_json::Value, EngineError> {
    let mut limits = CaptureBudget {
        nodes: *budget,
        bytes: 2 * 1024 * 1024,
    };
    let result = capture_object_bounded(pdf, object, depth, &mut limits);
    *budget = limits.nodes;
    result
}

struct CaptureBudget {
    nodes: usize,
    bytes: usize,
}
impl CaptureBudget {
    fn node(&mut self) -> Result<(), EngineError> {
        self.nodes = self
            .nodes
            .checked_sub(1)
            .ok_or_else(|| invalid("annotation appearance exceeds capture limits"))?;
        Ok(())
    }
    fn bytes(&mut self, count: usize) -> Result<(), EngineError> {
        self.bytes = self
            .bytes
            .checked_sub(count)
            .ok_or_else(|| invalid("annotation appearance exceeds capture byte limits"))?;
        Ok(())
    }
}

fn capture_dictionary(
    pdf: &lopdf::Document,
    dict: &lopdf::Dictionary,
    depth: usize,
    budget: &mut CaptureBudget,
    ignored: &[&[u8]],
) -> Result<serde_json::Value, EngineError> {
    if depth > 16 || dict.len() > 256 {
        return Err(invalid("annotation dictionary exceeds capture limits"));
    }
    let mut values = serde_json::Map::new();
    for (key, value) in dict.iter() {
        if key.len() > 128 {
            return Err(invalid("annotation key exceeds capture limits"));
        }
        budget.bytes(key.len())?;
        if ignored.contains(&key.as_slice()) {
            continue;
        }
        let key = std::str::from_utf8(key).map_err(|_| invalid("unsupported annotation key"))?;
        values.insert(
            key.to_owned(),
            capture_object_bounded(pdf, value, depth + 1, budget)?,
        );
    }
    Ok(serde_json::Value::Object(values))
}

fn capture_object_bounded<'a>(
    pdf: &'a lopdf::Document,
    mut object: &'a lopdf::Object,
    depth: usize,
    budget: &mut CaptureBudget,
) -> Result<serde_json::Value, EngineError> {
    use lopdf::Object;
    if depth > 16 {
        return Err(invalid("annotation appearance exceeds capture limits"));
    }
    budget.node()?;
    let mut references = 0;
    while let Object::Reference(id) = object {
        references += 1;
        if references > 16 {
            return Err(invalid("annotation reference chain exceeds capture limits"));
        }
        budget.node()?;
        object = pdf
            .objects
            .get(id)
            .ok_or_else(|| invalid("missing annotation appearance reference"))?;
    }
    Ok(match object {
        Object::Null => serde_json::Value::Null,
        Object::Boolean(v) => serde_json::json!(v),
        Object::Integer(v) => serde_json::json!(*v as f64),
        Object::Real(v) if v.is_finite() => serde_json::json!(*v as f64),
        Object::Name(v) if v.len() <= 256 => {
            budget.bytes(v.len())?;
            serde_json::json!({"name": v})
        }
        Object::String(v, _) if v.len() <= 8192 => {
            budget.bytes(v.len())?;
            serde_json::json!({"string": v})
        }
        Object::Array(values) if values.len() <= 1024 => serde_json::Value::Array(
            values
                .iter()
                .map(|value| capture_object_bounded(pdf, value, depth + 1, budget))
                .collect::<Result<_, _>>()?,
        ),
        Object::Dictionary(dict) => capture_dictionary(pdf, dict, depth, budget, &[])?,
        Object::Stream(stream) => {
            // Charge every traversal, including repeated references, before cloning or decoding.
            budget.bytes(stream.content.len())?;
            let dictionary = capture_dictionary(
                pdf,
                &stream.dict,
                depth + 1,
                budget,
                &[b"Length", b"Filter", b"DecodeParms"],
            )?;
            let content = if stream.dict.has(b"Filter") {
                let filter = stream
                    .dict
                    .get(b"Filter")
                    .map_err(crate::security::lopdf_err)?;
                let valid_filter =
                    |value: &Object| matches!(value, Object::Name(name) if name.len() <= 256);
                if !valid_filter(filter)
                    && !matches!(filter, Object::Array(filters) if filters.len() <= 8 && filters.iter().all(valid_filter))
                {
                    return Err(invalid("unsupported annotation stream filters"));
                }
                // Filter and predictor metadata is bounded even though it is omitted from canonical content.
                capture_object_bounded(pdf, filter, depth + 1, budget)?;
                if let Ok(params) = stream.dict.get(b"DecodeParms") {
                    capture_object_bounded(pdf, params, depth + 1, budget)?;
                }
                stream
                    .decompressed_content_with_limit(budget.bytes)
                    .map_err(crate::security::lopdf_err)?
            } else {
                stream.content.clone()
            };
            if stream.dict.has(b"Filter") {
                budget.bytes(content.len())?;
            }
            serde_json::json!({"dictionary": dictionary, "content": digest_hex(&content)})
        }
        _ => return Err(invalid("unsupported annotation appearance value")),
    })
}

fn capture_primitive(
    pdf: &lopdf::Document,
    dict: &lopdf::Dictionary,
    geo: &crate::geometry::PageGeometry,
) -> Result<(LibraryPrimitive, String), EngineError> {
    use lopdf::Object;
    if dict.has(b"SuniPDFRevisionOutline") {
        return Err(invalid("revision outlines cannot be captured or transferred without preserving their source-bound provenance"));
    }
    let unsupported = || {
        invalid("only unmodified native shapes, lines, polygons and clouds can be captured; text, images, replies and custom appearances are not supported")
    };
    let field = |key: &[u8]| -> Result<&Object, EngineError> {
        pdf.dereference(dict.get(key).map_err(|_| unsupported())?)
            .map(|(_, value)| value)
            .map_err(crate::security::lopdf_err)
    };
    let numbers = |object: &Object| -> Result<Vec<f32>, EngineError> {
        let (_, object) = pdf
            .dereference(object)
            .map_err(crate::security::lopdf_err)?;
        let values = object.as_array().map_err(|_| unsupported())?;
        if values.len() > 1024 {
            return Err(unsupported());
        }
        values
            .iter()
            .map(|value| {
                let (_, value) = pdf.dereference(value).map_err(crate::security::lopdf_err)?;
                value
                    .as_float()
                    .ok()
                    .filter(|n| n.is_finite())
                    .ok_or_else(unsupported)
            })
            .collect()
    };
    let color = |key: &[u8]| -> Result<Option<[u8; 3]>, EngineError> {
        if !dict.has(key) {
            return Ok(None);
        }
        let values = numbers(field(key)?)?;
        let [r, g, b] = values.as_slice() else {
            return Err(unsupported());
        };
        if values.iter().any(|n| !(0.0..=1.0).contains(n)) {
            return Err(unsupported());
        }
        Ok(Some([
            (*r * 255.0).round() as u8,
            (*g * 255.0).round() as u8,
            (*b * 255.0).round() as u8,
        ]))
    };
    for key in [b"NM".as_slice(), b"T", b"M", b"CreationDate"] {
        if dict.has(key) && !matches!(field(key)?, Object::String(bytes, _) if bytes.len() <= 8192)
        {
            return Err(unsupported());
        }
    }
    if !crate::annot_write::text_of(pdf, dict, b"NM")
        .is_some_and(|name| name.starts_with(crate::annot_write::NAME_PREFIX))
        || field(b"F")?.as_i64().ok() != Some(4)
        || dict.iter().any(|(key, _)| {
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
                    | b"C"
                    | b"IC"
                    | b"CA"
                    | b"BS"
                    | b"L"
                    | b"LE"
                    | b"Vertices"
                    | b"BE"
                    | b"IT"
                    | b"SuniPDFRich"
            )
        })
    {
        return Err(unsupported());
    }
    let subtype = field(b"Subtype")?.as_name().map_err(|_| unsupported())?;
    let rect_values = numbers(field(b"Rect")?)?;
    let [left, bottom, right, top] = rect_values.as_slice() else {
        return Err(unsupported());
    };
    let rect = geo.rect_to_display(PointRect {
        left: *left,
        bottom: *bottom,
        right: *right,
        top: *top,
    });
    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
    let page_size = geo.display_size();
    if !finite(w, 0.1, 10000.0)
        || !finite(h, 0.1, 10000.0)
        || rect.left < 0.0
        || rect.bottom < 0.0
        || rect.right > page_size.0
        || rect.top > page_size.1
    {
        return Err(unsupported());
    }
    let border = field(b"BS")?.as_dict().map_err(|_| unsupported())?;
    let width = border
        .get(b"W")
        .and_then(Object::as_float)
        .map_err(|_| unsupported())?;
    let dash = if border.has(b"D") {
        numbers(border.get(b"D").map_err(|_| unsupported())?)?
    } else {
        Vec::new()
    };
    let expected_border = crate::annot_write::border_style(width, &dash);
    if capture_dictionary(
        pdf,
        border,
        0,
        &mut CaptureBudget {
            nodes: 128,
            bytes: 2 * 1024 * 1024,
        },
        &[],
    )? != capture_object(pdf, &expected_border, 0, &mut 128)?
    {
        return Err(unsupported());
    }
    let style = LibraryStyle {
        rgb: color(b"C")?.ok_or_else(unsupported)?,
        fill_rgb: color(b"IC")?,
        width_pt: width,
        opacity: field(b"CA")?.as_float().map_err(|_| unsupported())?,
        dash_pattern: dash,
    };
    style.validate()?;
    let placed = style.placed(1.0);
    let expected_bounds = |points: &[(f32, f32)], margin: f32| -> bool {
        let bounds = points.iter().fold(
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            |bounds, point| {
                [
                    bounds[0].min(point.0),
                    bounds[1].min(point.1),
                    bounds[2].max(point.0),
                    bounds[3].max(point.1),
                ]
            },
        );
        [rect.left, rect.bottom, rect.right, rect.top]
            .into_iter()
            .zip([
                bounds[0] - margin,
                bounds[1] - margin,
                bounds[2] + margin,
                bounds[3] + margin,
            ])
            .all(|(a, b)| (a - b).abs() < 0.002)
    };
    let local = |point: (f32, f32)| (point.0 - rect.left, point.1 - rect.bottom);
    let (primitive, appearance) = match subtype {
        b"Square" | b"Circle" => {
            if dict.has(b"BE") || dict.has(b"IT") || dict.has(b"SuniPDFRich") {
                return Err(unsupported());
            }
            let shape = if subtype == b"Square" {
                LibraryShape::Square
            } else {
                LibraryShape::Circle
            };
            let appearance = if subtype == b"Square" {
                crate::appearance::square_ap(w, h, &placed)
            } else {
                crate::appearance::circle_ap(w, h, &placed)
            };
            (
                LibraryPrimitive::Shape {
                    shape,
                    rect: [rect.left, rect.bottom, rect.right, rect.top],
                    style,
                },
                appearance,
            )
        }
        b"Line" => {
            if style.fill_rgb.is_some()
                || dict.has(b"BE")
                || dict.has(b"IT")
                || dict.has(b"SuniPDFRich")
            {
                return Err(unsupported());
            }
            let line = numbers(field(b"L")?)?;
            let [x1, y1, x2, y2] = line.as_slice() else {
                return Err(unsupported());
            };
            let ends = field(b"LE")?.as_array().map_err(|_| unsupported())?;
            if ends.len() != 2 || ends[0].as_name().ok() != Some(b"None".as_slice()) {
                return Err(unsupported());
            }
            let arrow = match ends[1].as_name().map_err(|_| unsupported())? {
                b"None" => false,
                b"OpenArrow" => true,
                _ => return Err(unsupported()),
            };
            let p1 = geo.to_display(*x1, *y1);
            let p2 = geo.to_display(*x2, *y2);
            let margin = placed.width_pt / 2.0
                + 1.0
                + if arrow {
                    crate::appearance::arrow_head(placed.width_pt)
                } else {
                    0.0
                };
            if !expected_bounds(&[p1, p2], margin) {
                return Err(unsupported());
            }
            (
                LibraryPrimitive::Shape {
                    shape: if arrow {
                        LibraryShape::Arrow
                    } else {
                        LibraryShape::Line
                    },
                    rect: [p1.0, p1.1, p2.0, p2.1],
                    style,
                },
                crate::appearance::line_ap(w, h, local(p1), local(p2), arrow, &placed),
            )
        }
        b"Polygon" | b"PolyLine" => {
            if field(b"SuniPDFRich")?.as_name().ok() != Some(subtype) {
                return Err(unsupported());
            }
            let cloud = dict.has(b"BE");
            if cloud {
                if subtype != b"Polygon"
                    || field(b"IT")?.as_name().ok() != Some(b"PolygonCloud".as_slice())
                    || capture_object(pdf, field(b"BE")?, 0, &mut 64)?
                        != capture_object(
                            pdf,
                            &Object::Dictionary(lopdf::dictionary! {"S" => "C", "I" => 2}),
                            0,
                            &mut 64,
                        )?
                {
                    return Err(unsupported());
                }
            } else if dict.has(b"IT") {
                return Err(unsupported());
            }
            let values = numbers(field(b"Vertices")?)?;
            if !values.len().is_multiple_of(2) {
                return Err(unsupported());
            }
            let points: Vec<_> = values
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| geo.to_display(p[0], p[1]))
                .collect();
            let closed = subtype == b"Polygon";
            let primitive = LibraryPrimitive::Vertices {
                vertex_kind: if cloud {
                    LibraryVertexKind::Cloud
                } else if closed {
                    LibraryVertexKind::Polygon
                } else {
                    LibraryVertexKind::Polyline
                },
                points: points.clone(),
                style,
            };
            primitive.validate()?;
            let margin = placed.width_pt / 2.0 + if cloud { 4.5 } else { 0.5 };
            if !expected_bounds(&points, margin)
                || points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .take(points.len() - usize::from(!closed))
                    .map(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1))
                    .sum::<f32>()
                    > 50_000.0
            {
                return Err(unsupported());
            }
            let appearance = crate::appearance::vertices_ap(
                w,
                h,
                &points.iter().map(|p| local(*p)).collect::<Vec<_>>(),
                closed,
                cloud,
                &placed,
            );
            (primitive, appearance)
        }
        _ => return Err(unsupported()),
    };
    primitive.validate()?;
    let ap = field(b"AP")?.as_dict().map_err(|_| unsupported())?;
    if ap.len() != 1 {
        return Err(unsupported());
    }
    let source = ap.get(b"N").map_err(|_| unsupported())?;
    let expected = Object::Stream(crate::annot_write::ap_stream(&appearance, geo));
    if capture_object(pdf, source, 0, &mut 4096)? != capture_object(pdf, &expected, 0, &mut 4096)? {
        return Err(unsupported());
    }
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
    Ok((
        primitive,
        digest_hex(&serde_json::to_vec(&fingerprint).map_err(|_| unsupported())?),
    ))
}

fn capture_annotation_entries(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
) -> Result<&[lopdf::Object], EngineError> {
    let page = pdf
        .get_dictionary(page)
        .map_err(crate::security::lopdf_err)?;
    let Ok(raw) = page.get(b"Annots") else {
        return Ok(&[]);
    };
    let (_, entries) = pdf.dereference(raw).map_err(crate::security::lopdf_err)?;
    let entries = entries.as_array().map_err(crate::security::lopdf_err)?;
    if entries.len() > 512 {
        return Err(invalid(
            "capture selection is limited to pages with 512 annotations",
        ));
    }
    Ok(entries)
}

fn capture_annotation(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    index: usize,
) -> Result<&lopdf::Dictionary, EngineError> {
    let entry = capture_annotation_entries(pdf, page)?
        .get(index)
        .ok_or_else(|| invalid("capture annotation is unavailable"))?;
    let (_, entry) = pdf.dereference(entry).map_err(crate::security::lopdf_err)?;
    entry.as_dict().map_err(crate::security::lopdf_err)
}

fn capture_subtype(dict: &lopdf::Dictionary) -> String {
    dict.get(b"Subtype")
        .and_then(lopdf::Object::as_name)
        .ok()
        .filter(|value| value.len() <= 64)
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .unwrap_or_default()
}

/// Finds incoming reply/review links across the whole object graph, including direct annotation
/// dictionaries on other pages. References are not traversed recursively: each indirect object
/// is visited once. Limits fail closed rather than losing an uninspected part of a thread.
fn capture_replied_indices(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
) -> Result<HashSet<usize>, EngineError> {
    use lopdf::Object;
    const MAX_NODES: usize = 200_000;
    const MAX_DEPTH: usize = 32;
    let mut targets: HashMap<lopdf::ObjectId, Vec<usize>> = HashMap::new();
    for (index, entry) in capture_annotation_entries(pdf, page)?.iter().enumerate() {
        if let Some(id) = pdf
            .dereference(entry)
            .map_err(crate::security::lopdf_err)?
            .0
        {
            targets.entry(id).or_default().push(index);
        }
    }
    if pdf.objects.len() > MAX_NODES {
        return Err(invalid(
            "reply inspection exceeds the document object limit",
        ));
    }
    fn visit(
        pdf: &lopdf::Document,
        object: &Object,
        targets: &HashMap<lopdf::ObjectId, Vec<usize>>,
        blocked: &mut HashSet<usize>,
        remaining: &mut usize,
        depth: usize,
    ) -> Result<(), EngineError> {
        if *remaining == 0 || depth > MAX_DEPTH {
            return Err(invalid(
                "reply inspection exceeds the document structure limit",
            ));
        }
        *remaining -= 1;
        let dictionary = match object {
            Object::Dictionary(dictionary) => Some(dictionary),
            Object::Stream(stream) => Some(&stream.dict),
            _ => None,
        };
        if let Some(dictionary) = dictionary {
            if let Ok(parent) = dictionary.get(b"IRT") {
                if !matches!(parent, Object::Reference(_)) {
                    return Err(invalid(
                        "reply target must be an indirect annotation reference",
                    ));
                }
                let (id, target) = pdf
                    .dereference(parent)
                    .map_err(crate::security::lopdf_err)?;
                if target.as_dict().is_err() {
                    return Err(invalid("reply target is not an annotation dictionary"));
                }
                if let Some(indices) = id.and_then(|id| targets.get(&id)) {
                    blocked.extend(indices.iter().copied());
                }
            }
            for (_, value) in dictionary.iter() {
                visit(pdf, value, targets, blocked, remaining, depth + 1)?;
            }
        } else if let Object::Array(values) = object {
            for value in values {
                visit(pdf, value, targets, blocked, remaining, depth + 1)?;
            }
        }
        Ok(())
    }
    let mut blocked = HashSet::new();
    let mut remaining = MAX_NODES;
    for object in pdf.objects.values() {
        visit(pdf, object, &targets, &mut blocked, &mut remaining, 0)?;
    }
    Ok(blocked)
}

fn capture_without_thread(
    document: &Document<'_>,
    pdf: &lopdf::Document,
    dict: &lopdf::Dictionary,
    geo: &crate::geometry::PageGeometry,
    replies: &HashSet<usize>,
    index: usize,
    purpose: CapturePurpose,
) -> Result<(LibraryPrimitive, String), EngineError> {
    if replies.contains(&index) {
        return Err(invalid(
            "annotations with replies or review history cannot be captured or transferred",
        ));
    }
    if matches!(
        purpose,
        CapturePurpose::Library | CapturePurpose::LibraryListing
    ) {
        match capture_subtype(dict).as_str() {
            "Stamp" => return capture_stamps::capture(pdf, dict, geo),
            "FreeText" => {
                let verify = matches!(purpose, CapturePurpose::Library);
                return capture_text::capture(document, pdf, dict, geo, verify);
            }
            _ => {}
        }
    }
    capture_primitive(pdf, dict, geo)
}

/// Revision transfer deliberately supports only the original shape/vertex subset.
#[derive(Clone, Copy)]
enum CapturePurpose {
    Original,
    Library,
    /// Candidate listing past its text budget: FreeText appearance regeneration is
    /// deferred to `capture_library_entry`, which always verifies it.
    LibraryListing,
}

/// Native FreeText verification regenerates each appearance with PDFium on the engine
/// thread. Bound that work per candidate listing: each text costs a fixed overhead plus
/// its characters.
const CANDIDATE_TEXT_BUDGET: usize = 64 * 1024;
const CANDIDATE_TEXT_OVERHEAD: usize = 1024;

impl Document<'_> {
    pub(crate) fn capture_original_annotations(
        &self,
        page: usize,
    ) -> Result<Vec<OriginalCapture>, EngineError> {
        self.capture_original_selection(page, None)
    }

    pub(crate) fn capture_original_annotation(
        &self,
        page: usize,
        index: usize,
    ) -> Result<OriginalCapture, EngineError> {
        self.capture_original_selection(page, Some(index))?
            .pop()
            .ok_or_else(|| invalid("capture annotation is unavailable"))
    }

    fn capture_original_selection(
        &self,
        page: usize,
        selected: Option<usize>,
    ) -> Result<Vec<OriginalCapture>, EngineError> {
        let (pdf, page_id, geo) = self.capture_source(page)?;
        let entries = capture_annotation_entries(&pdf, page_id)?;
        let replies = capture_replied_indices(&pdf, page_id)?;
        entries
            .iter()
            .enumerate()
            .filter(|(index, _)| selected.is_none_or(|wanted| wanted == *index))
            .map(|(index, _entry)| {
                let dict = capture_annotation(&pdf, page_id, index)?;
                let source_nm = crate::annot_write::text_of(&pdf, dict, b"NM");
                let author = crate::annot_write::text_of(&pdf, dict, b"T");
                let captured = (|| {
                    if (dict.has(b"T") && author.is_none())
                        || source_nm
                            .as_deref()
                            .is_none_or(|v| !text_valid(v, 1024, false) || v.is_empty())
                        || author.as_deref().is_some_and(|v| {
                            !text_valid(v, 1024, false) || v.trim() != v || v.is_empty()
                        })
                    {
                        return Err(invalid(
                            "annotation identity or author is not safely transferable",
                        ));
                    }
                    capture_without_thread(
                        self,
                        &pdf,
                        dict,
                        &geo,
                        &replies,
                        index,
                        CapturePurpose::Original,
                    )
                })();
                let (primitive, fingerprint, refusal) = match captured {
                    Ok((primitive, fingerprint)) => (Some(primitive), Some(fingerprint), None),
                    Err(error) => (None, None, Some(error.to_string())),
                };
                Ok(OriginalCapture {
                    index,
                    subtype: capture_subtype(dict),
                    fingerprint,
                    source_nm,
                    author,
                    primitive,
                    refusal,
                })
            })
            .collect()
    }

    fn capture_source(
        &self,
        page: usize,
    ) -> Result<
        (
            lopdf::Document,
            lopdf::ObjectId,
            crate::geometry::PageGeometry,
        ),
        EngineError,
    > {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let geometry = self.page_geometry(page)?;
        let bytes = self.snapshot()?;
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(invalid("capture supports documents up to 128 MiB"));
        }
        let pdf = crate::security::load(&bytes, self.password())?;
        let id = *pdf
            .get_pages()
            .get(&(page as u32 + 1))
            .ok_or_else(|| invalid("page is unavailable"))?;
        capture_annotation_entries(&pdf, id)?;
        Ok((pdf, id, geometry))
    }

    pub fn library_capture_candidates(
        &self,
        page: usize,
    ) -> Result<Vec<LibraryCaptureCandidate>, EngineError> {
        let (pdf, page_id, geo) = self.capture_source(page)?;
        let replies = capture_replied_indices(&pdf, page_id)?;
        let count = capture_annotation_entries(&pdf, page_id)?.len();
        let mut text_budget = CANDIDATE_TEXT_BUDGET;
        (0..count)
            .map(|index| {
                let dict = capture_annotation(&pdf, page_id, index)?;
                let subtype = capture_subtype(dict);
                let mut purpose = CapturePurpose::Library;
                if subtype == "FreeText" {
                    let cost = CANDIDATE_TEXT_OVERHEAD
                        + crate::annot_write::text_of(&pdf, dict, b"Contents")
                            .map_or(0, |text| text.chars().count());
                    match text_budget.checked_sub(cost) {
                        Some(left) => text_budget = left,
                        None => purpose = CapturePurpose::LibraryListing,
                    }
                }
                let result =
                    capture_without_thread(self, &pdf, dict, &geo, &replies, index, purpose);
                Ok(LibraryCaptureCandidate {
                    index,
                    subtype,
                    fingerprint: result.as_ref().ok().map(|(_, hash)| hash.clone()),
                    refusal: result.err().map(|error| error.to_string()),
                })
            })
            .collect()
    }

    /// Copies selected supported annotations without modifying the source or importing PDF objects.
    pub fn capture_library_entry(
        &self,
        page: usize,
        selections: &[LibraryCaptureSelection],
        id: String,
        name: String,
        provenance: LibraryProvenance,
    ) -> Result<LibraryEntry, EngineError> {
        if selections.is_empty() || selections.len() > MAX_GROUP_PRIMITIVES {
            return Err(invalid("select 1–16 annotations"));
        }
        if provenance
            .source
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(invalid("declare a source for captured annotations"));
        }
        let (pdf, page_id, geo) = self.capture_source(page)?;
        let replies = capture_replied_indices(&pdf, page_id)?;
        let mut seen = HashSet::new();
        let mut primitives = Vec::new();
        for selected in selections {
            if !seen.insert(selected.index) {
                return Err(invalid("duplicate annotation selection"));
            }
            let dict = capture_annotation(&pdf, page_id, selected.index)?;
            let (primitive, fingerprint) = capture_without_thread(
                self,
                &pdf,
                dict,
                &geo,
                &replies,
                selected.index,
                CapturePurpose::Library,
            )?;
            if fingerprint != selected.fingerprint {
                return Err(invalid("source annotation changed; refresh the selection"));
            }
            primitives.push(primitive);
        }
        LibraryEntry {
            id,
            name,
            provenance,
            primitives,
        }
        .normalized()
    }
}

impl Document<'_> {
    /// Places a complete flat group atomically. The caller saves the resulting PDF separately.
    pub fn place_library_entry(
        &mut self,
        page: usize,
        entry: &LibraryEntry,
        placement: LibraryPlacement,
        author: Option<&str>,
    ) -> Result<Vec<usize>, EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        entry.validate()?;
        let original = self.snapshot()?;
        if original.len() > 128 * 1024 * 1024 {
            return Err(invalid("placement supports documents up to 128 MiB"));
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
        let indices = scratch.apply_library_entry_in_place(page, entry, placement, author)?;
        let candidate = scratch.snapshot()?;
        if candidate.len() > 160 * 1024 * 1024 {
            return Err(invalid("placement exceeds document size limit"));
        }
        if !crate::annot_rich::same_document(&original, &self.snapshot()?, self.password())? {
            return Err(invalid("document changed during placement"));
        }
        self.restore(candidate)?;
        Ok(indices)
    }

    /// Only called on a disposable preview/placement scratch document.
    pub(crate) fn apply_library_entry_in_place(
        &mut self,
        page: usize,
        entry: &LibraryEntry,
        placement: LibraryPlacement,
        author: Option<&str>,
    ) -> Result<Vec<usize>, EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        entry.validate()?;
        if !finite(placement.scale, 0.01, 100.0)
            || !finite(placement.origin.0, 0.0, 100000.0)
            || !finite(placement.origin.1, 0.0, 100000.0)
        {
            return Err(invalid("invalid placement origin or scale"));
        }
        let point = |(x, y): (f32, f32)| {
            (
                placement.origin.0 + x * placement.scale,
                placement.origin.1 + y * placement.scale,
            )
        };
        let rect = |r: [f32; 4]| {
            let a = point((r[0], r[1]));
            let b = point((r[2], r[3]));
            PointRect {
                left: a.0,
                bottom: a.1,
                right: b.0,
                top: b.1,
            }
        };
        let mut result = Vec::with_capacity(entry.primitives.len());
        for p in &entry.primitives {
            let index = match p {
                LibraryPrimitive::Shape {
                    shape,
                    rect: r,
                    style,
                } => self.add_shape_annotation(
                    page,
                    match shape {
                        LibraryShape::Square => AnnotShape::Square,
                        LibraryShape::Circle => AnnotShape::Circle,
                        LibraryShape::Line => AnnotShape::Line,
                        LibraryShape::Arrow => AnnotShape::Arrow,
                    },
                    rect(*r),
                    &style.placed(placement.scale),
                    author,
                )?,
                LibraryPrimitive::FreeText {
                    rect: r,
                    text,
                    style,
                } => {
                    let placed = rect(*r);
                    capture_text::preserve_dimensions(
                        *r,
                        [placed.left, placed.bottom, placed.right, placed.top],
                        placement.scale,
                    )?;
                    self.add_rich_free_text_annotation(
                        page,
                        None,
                        placed,
                        text,
                        &style.placed(placement.scale),
                        author,
                    )?
                }
                LibraryPrimitive::Vertices {
                    vertex_kind,
                    points,
                    style,
                } => self.add_vertex_annotation(
                    page,
                    match vertex_kind {
                        LibraryVertexKind::Polygon => VertexAnnotKind::Polygon,
                        LibraryVertexKind::Polyline => VertexAnnotKind::Polyline,
                        LibraryVertexKind::Cloud => VertexAnnotKind::Cloud,
                    },
                    &points.iter().copied().map(point).collect::<Vec<_>>(),
                    &style.placed(placement.scale),
                    author,
                )?,
                LibraryPrimitive::Stamp { rect: r, stamp } => self.add_stamp_annotation(
                    page,
                    rect(*r),
                    match stamp {
                        LibraryStamp::Approved => StampKind::Approved,
                        LibraryStamp::Draft => StampKind::Draft,
                        LibraryStamp::Confidential => StampKind::Confidential,
                        LibraryStamp::Rejected => StampKind::Rejected,
                    },
                    author,
                )?,
                LibraryPrimitive::Image {
                    rect: r,
                    png,
                    opacity,
                } => self.add_image_stamp_annotation_bytes(
                    page,
                    rect(*r),
                    &png.png_bytes()?,
                    *opacity,
                    author,
                )?,
            };
            result.push(index);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_reply_inspection_is_bounded_and_does_not_follow_reference_cycles() {
        use lopdf::{dictionary, Object};
        let mut pdf = lopdf::Document::with_version("1.7");
        let parent = pdf.add_object(dictionary! {"Type"=>"Annot","Subtype"=>"Square"});
        let page =
            pdf.add_object(dictionary! {"Type"=>"Page","Annots"=>vec![Object::Reference(parent)]});
        let cycle = pdf.new_object_id();
        pdf.objects.insert(cycle, Object::Reference(cycle));
        // Non-IRT references are opaque leaves; cycles do not recurse or allocate.
        assert!(capture_replied_indices(&pdf, page).unwrap().is_empty());
        let reply = pdf.add_object(dictionary! {"Type"=>"Annot","Subtype"=>"Text","IRT"=>parent});
        assert_eq!(
            capture_replied_indices(&pdf, page).unwrap(),
            HashSet::from([0])
        );
        pdf.objects.remove(&reply);
        let mut nested = Object::Null;
        for _ in 0..34 {
            nested = Object::Array(vec![nested]);
        }
        let deep = pdf.add_object(nested);
        assert!(capture_replied_indices(&pdf, page)
            .unwrap_err()
            .to_string()
            .contains("structure limit"));
        pdf.objects.remove(&deep);
        pdf.add_object(Object::Array(vec![Object::Null; 200_001]));
        assert!(capture_replied_indices(&pdf, page)
            .unwrap_err()
            .to_string()
            .contains("structure limit"));
    }
    use serde_json::json;

    #[test]
    fn capture_inventory_bounds_page_count_and_borrows_oversized_subtypes() {
        use lopdf::{dictionary, Object};
        let mut pdf = lopdf::Document::with_version("1.7");
        let annotation =
            pdf.add_object(dictionary! {"Subtype" => Object::Name(vec![b'A'; 1024 * 1024])});
        let entries = pdf.add_object(Object::Array(vec![Object::Reference(annotation); 512]));
        let page = pdf.add_object(dictionary! {"Type" => "Page", "Annots" => entries});
        for index in 0..512 {
            let borrowed = capture_annotation(&pdf, page, index).unwrap();
            assert!(std::ptr::eq(
                borrowed,
                pdf.get_dictionary(annotation).unwrap()
            ));
            assert!(
                capture_subtype(borrowed).is_empty(),
                "oversized names must not be retained in inventory rows"
            );
        }
        pdf.get_object_mut(entries)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(Object::Reference(annotation));
        assert!(capture_annotation_entries(&pdf, page).is_err());
        assert!(capture_annotation(&pdf, page, 0).is_err());
    }

    #[test]
    fn capture_budget_rejects_large_names_keys_and_repeated_reference_payloads() {
        use lopdf::{Dictionary, Object, Stream};
        let mut pdf = lopdf::Document::with_version("1.7");
        assert!(capture_object(&pdf, &Object::Name(vec![b'N'; 257]), 0, &mut 4096).is_err());
        let mut dict = Dictionary::new();
        dict.set(vec![b'K'; 129], 1);
        assert!(capture_object(&pdf, &Object::Dictionary(dict), 0, &mut 4096).is_err());
        let text = pdf.add_object(Object::string_literal(vec![b'x'; 8192]));
        let repeated = Object::Array(vec![Object::Reference(text); 300]);
        assert!(capture_object(&pdf, &repeated, 0, &mut 4096).is_err());
        let mut compressed = Stream::new(Dictionary::new(), vec![b' '; 1024 * 1024]);
        compressed.compress().unwrap();
        let stream = pdf.add_object(compressed);
        assert!(capture_object(&pdf, &Object::Reference(stream), 0, &mut 4096).is_ok());
        assert!(capture_object(
            &pdf,
            &Object::Array(vec![Object::Reference(stream); 3]),
            0,
            &mut 4096
        )
        .is_err());
        let raw = pdf.add_object(Stream::new(Dictionary::new(), vec![b'x'; 600 * 1024]));
        assert!(capture_object(
            &pdf,
            &Object::Array(vec![Object::Reference(raw); 4]),
            0,
            &mut 4096
        )
        .is_err());
        let cycle = pdf.new_object_id();
        pdf.objects.insert(cycle, Object::Reference(cycle));
        assert!(capture_object(&pdf, &Object::Reference(cycle), 0, &mut 4096).is_err());
    }

    #[test]
    fn capture_budget_keeps_compressed_and_plain_native_content_canonical() {
        use lopdf::{dictionary, Object, Stream};
        let pdf = lopdf::Document::with_version("1.7");
        let stream = Stream::new(
            dictionary! {"Type" => "XObject", "Subtype" => "Form"},
            b"q 1 0 0 RG 0 0 20 20 re S Q\n".repeat(20),
        );
        let mut compressed = stream.clone();
        compressed.compress().unwrap();
        assert!(compressed.dict.has(b"Filter"));
        assert_eq!(
            capture_object(&pdf, &Object::Stream(stream), 0, &mut 4096).unwrap(),
            capture_object(&pdf, &Object::Stream(compressed), 0, &mut 4096).unwrap()
        );
    }

    #[test]
    fn saved_capture_refuses_malformed_vertices_and_custom_appearances() {
        use lopdf::{dictionary, Object};
        let geo = crate::geometry::PageGeometry {
            bbox: PointRect {
                left: 100.0,
                bottom: 50.0,
                right: 500.0,
                top: 650.0,
            },
            rotation: crate::Rotation::R90,
        };
        let rect = PointRect {
            left: 40.0,
            bottom: 60.0,
            right: 140.0,
            top: 110.0,
        };
        let style = AnnotStyle {
            rgb: (20, 60, 100),
            fill_rgb: Some((190, 230, 250)),
            width_pt: 1.5,
            opacity: 0.75,
            dash_pattern: vec![4.0, 2.0],
        };
        let mut pdf = lopdf::Document::with_version("1.7");
        let mut annotation = crate::annot_write::base_annotation("Square", None).unwrap();
        annotation.set("Rect", crate::annot_write::rect_obj(geo.rect_to_user(rect)));
        annotation.set("C", crate::annot_write::rgb_obj(style.rgb));
        annotation.set("IC", crate::annot_write::rgb_obj(style.fill_rgb.unwrap()));
        annotation.set(
            "BS",
            crate::annot_write::border_style(style.width_pt, &style.dash_pattern),
        );
        annotation.set("CA", style.opacity);
        let stream =
            crate::annot_write::ap_stream(&crate::appearance::square_ap(100.0, 50.0, &style), &geo);
        let appearance_id = pdf.add_object(stream);
        annotation.set("AP", dictionary! {"N" => appearance_id});
        let (captured, _) = capture_primitive(&pdf, &annotation, &geo).unwrap();
        assert_eq!(
            captured,
            LibraryPrimitive::Shape {
                shape: LibraryShape::Square,
                rect: [40.0, 60.0, 140.0, 110.0],
                style: LibraryStyle::from(&style)
            }
        );
        for key in ["NM", "T", "M", "CreationDate"] {
            let previous = annotation.get(key.as_bytes()).ok().cloned();
            annotation.set(
                key,
                Object::string_literal(format!("SuniPDF-{}", "x".repeat(8192))),
            );
            assert!(
                capture_primitive(&pdf, &annotation, &geo).is_err(),
                "oversized {key} must be refused before text decoding"
            );
            if let Some(previous) = previous {
                annotation.set(key, previous);
            } else {
                annotation.remove(key.as_bytes());
            }
        }
        annotation.set(
            "P",
            dictionary! {"IgnoredPagePayload" => Object::Name(vec![b'x'; 1024 * 1024])},
        );
        assert!(
            capture_primitive(&pdf, &annotation, &geo).is_ok(),
            "page references are excluded without cloning or traversing their payload"
        );
        annotation.remove(b"P");
        pdf.get_object_mut(appearance_id)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .content
            .extend_from_slice(b" q Q");
        assert!(capture_primitive(&pdf, &annotation, &geo).is_err());
        annotation.set("Subtype", Object::Name(b"Polygon".to_vec()));
        annotation.set("SuniPDFRich", Object::Name(b"Polygon".to_vec()));
        annotation.set("Vertices", Object::Array(vec![]));
        assert!(capture_primitive(&pdf, &annotation, &geo).is_err());
        annotation.set(
            "Vertices",
            crate::annot_write::nums(&[150.0, 100.0, 150.0, 100.0, 200.0, 150.0]),
        );
        assert!(capture_primitive(&pdf, &annotation, &geo).is_err());
        annotation.set(
            "A",
            dictionary! {"S" => "JavaScript", "JS" => Object::string_literal("void(0)")},
        );
        assert!(capture_primitive(&pdf, &annotation, &geo).is_err());
    }

    fn entry() -> LibraryEntry {
        LibraryEntry {
            id: "draft".into(),
            name: "Draft".into(),
            provenance: LibraryProvenance::default(),
            primitives: vec![LibraryPrimitive::Stamp {
                rect: [10.0, 20.0, 110.0, 50.0],
                stamp: LibraryStamp::Draft,
            }],
        }
    }

    #[test]
    fn strict_format_rejects_unsupported_fields_actions_targets_and_duplicate_fields() {
        let good = serde_json::to_vec(&entry()).unwrap();
        assert!(LibraryEntry::from_json(&good).is_ok());
        let duplicate = String::from_utf8(good.clone()).unwrap().replacen(
            "\"id\":\"draft\"",
            "\"id\":\"draft\",\"id\":\"other\"",
            1,
        );
        assert!(LibraryEntry::from_json(duplicate.as_bytes()).is_err());
        for (field, value) in [
            ("script", json!("alert(1)")),
            ("path", json!("relative.png")),
            ("action", json!({"kind":"launch"})),
            ("calibration", json!([1, 2])),
            ("NM", json!("copied-id")),
            ("measurement", json!({})),
            ("primitives", json!([])),
        ] {
            let mut payload = serde_json::to_value(entry()).unwrap();
            if field == "primitives" {
                payload[field] = value;
            } else {
                payload["primitives"][0][field] = value;
            }
            assert!(
                LibraryEntry::from_json(&serde_json::to_vec(&payload).unwrap()).is_err(),
                "{field}"
            );
        }
        let duplicate = String::from_utf8(good).unwrap().replacen(
            "\"stamp\":\"draft\"",
            "\"stamp\":\"draft\",\"stamp\":\"approved\"",
            1,
        );
        assert!(LibraryEntry::from_json(duplicate.as_bytes()).is_err());
        let mut library = AnnotationLibrary::default();
        library.insert(entry()).unwrap();
        let mut payload = serde_json::to_value(&library).unwrap();
        payload["version"] = json!(2);
        assert!(AnnotationLibrary::from_json(&serde_json::to_vec(&payload).unwrap()).is_err());
        payload["version"] = json!(1);
        payload["remote"] = json!("https://example.invalid/library");
        assert!(AnnotationLibrary::from_json(&serde_json::to_vec(&payload).unwrap()).is_err());
        let mut nested = entry();
        nested.primitives.clear();
        assert!(nested.validate().is_err());
        assert!(AnnotationLibrary::from_json(&vec![b' '; MAX_LIBRARY_BYTES + 1]).is_err());
    }

    #[test]
    fn insertion_is_atomic_unique_bounded_and_keeps_declared_provenance() {
        let mut library = AnnotationLibrary::default();
        let mut first = entry();
        first.provenance = LibraryProvenance {
            author: Some("Declared creator".into()),
            source: Some("Local drawing symbols".into()),
            license: Some("Unknown".into()),
        };
        library.insert(first).unwrap();
        let before = library.to_json().unwrap();
        assert!(library.insert(entry()).is_err());
        let mut duplicate_name = entry();
        duplicate_name.id = "other".into();
        duplicate_name.name = "DRAFT".into();
        assert!(library.insert(duplicate_name).is_err());
        assert_eq!(library.to_json().unwrap(), before);
        assert_eq!(AnnotationLibrary::from_json(&before).unwrap(), library);
        for i in 1..MAX_LIBRARY_ENTRIES {
            let mut next = entry();
            next.id = format!("e{i}");
            next.name = format!("Entry {i}");
            library.insert(next).unwrap();
        }
        let mut too_many = entry();
        too_many.id = "last".into();
        too_many.name = "Last".into();
        assert!(library.insert(too_many).is_err());
        assert_eq!(
            library
                .remove("draft")
                .unwrap()
                .provenance
                .license
                .as_deref(),
            Some("Unknown")
        );
        assert!(library.remove("draft").is_err());
    }

    #[test]
    fn geometry_is_finite_normalized_and_groups_are_flat_and_bounded() {
        let normalized = entry().normalized().unwrap();
        assert_eq!(normalized.bounds().unwrap(), [0.0, 0.0, 100.0, 30.0]);
        assert_eq!(entry().bounds().unwrap(), [10.0, 20.0, 110.0, 50.0]);
        assert!(normalized.primitives[0].translated(-1.0, 0.0).is_err());
        let mut large = entry();
        large.primitives = vec![large.primitives[0].clone(); 17];
        assert!(large.validate().is_err());
        let mut bad = entry();
        bad.primitives = vec![LibraryPrimitive::Shape {
            shape: LibraryShape::Line,
            rect: [f32::NAN, 0.0, 10.0, 0.0],
            style: LibraryStyle::default(),
        }];
        assert!(bad.validate().is_err());
        bad.primitives = vec![LibraryPrimitive::Vertices {
            vertex_kind: LibraryVertexKind::Polygon,
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 0.0)],
            style: LibraryStyle::default(),
        }];
        assert!(bad.validate().is_err());
    }

    #[test]
    fn png_is_self_contained_checked_and_decoding_is_bounded() {
        let encode = |image: image::RgbaImage| {
            let mut out = Cursor::new(Vec::new());
            image.write_to(&mut out, image::ImageFormat::Png).unwrap();
            out.into_inner()
        };
        let bytes = encode(image::RgbaImage::from_pixel(
            2,
            3,
            image::Rgba([200, 20, 30, 180]),
        ));
        let png = EmbeddedPng::from_png_bytes(&bytes).unwrap();
        assert_eq!(png.dimensions(), (2, 3));
        assert!(!png.png_bytes().unwrap().is_empty());
        let mut bad = png.clone();
        bad.sha256 = "0".repeat(64);
        assert!(bad.png_bytes().is_err());
        let mut bad = png.clone();
        bad.width = 999;
        assert!(bad.png_bytes().is_err());
        assert!(EmbeddedPng::from_png_bytes(b"not an image").is_err());
        assert!(EmbeddedPng::from_png_bytes(&encode(image::RgbaImage::new(2, 2))).is_err());
        assert!(
            EmbeddedPng::from_png_bytes(&encode(image::RgbaImage::from_pixel(
                4097,
                1,
                image::Rgba([1, 2, 3, 255])
            )))
            .is_err()
        );
        assert!(
            EmbeddedPng::from_png_bytes(&encode(image::RgbaImage::from_pixel(
                2048,
                2048,
                image::Rgba([1, 2, 3, 255])
            )))
            .is_err()
        );
        assert!(EmbeddedPng::from_png_bytes(&vec![0; MAX_PNG_BYTES + 1]).is_err());
        let mut bad = png;
        bad.png_base64 = "!".into();
        assert!(bad.png_bytes().is_err());
    }

    #[test]
    fn capture_only_accepts_new_explicit_drafts_without_paths_or_document_targets() {
        let draft = RichAnnotation::Stamp {
            rect: PointRect {
                left: 40.0,
                bottom: 50.0,
                right: 140.0,
                top: 80.0,
            },
            kind: StampKind::Approved,
        };
        let captured = LibraryEntry::from_rich_annotation(
            "approval".into(),
            "Approval".into(),
            LibraryProvenance::default(),
            &draft,
        )
        .unwrap();
        assert_eq!(captured.bounds().unwrap(), [0.0, 0.0, 100.0, 30.0]);
        let mut text = RichAnnotation::FreeText {
            index: Some(0),
            rect: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 100.0,
                top: 50.0,
            },
            text: "Note".into(),
            style: RichFreeTextStyle::default(),
        };
        assert!(LibraryEntry::from_rich_annotation(
            "n".into(),
            "Note".into(),
            LibraryProvenance::default(),
            &text
        )
        .is_err());
        if let RichAnnotation::FreeText { index, style, .. } = &mut text {
            *index = None;
            style.text.font_file = Some("font.ttf".into());
        }
        assert!(LibraryEntry::from_rich_annotation(
            "n".into(),
            "Note".into(),
            LibraryProvenance::default(),
            &text
        )
        .is_err());
        let file = tempfile::NamedTempFile::new().unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([50, 60, 70, 255]))
            .save_with_format(file.path(), image::ImageFormat::Png)
            .unwrap();
        let image = RichAnnotation::ImageStamp {
            rect: PointRect {
                left: 30.0,
                bottom: 30.0,
                right: 80.0,
                top: 80.0,
            },
            image: file.path().to_owned(),
            opacity: 0.5,
        };
        let captured = LibraryEntry::from_rich_annotation(
            "image".into(),
            "Image".into(),
            LibraryProvenance::default(),
            &image,
        )
        .unwrap();
        drop(file);
        captured.validate().unwrap();
        let encoded = serde_json::to_string(&captured).unwrap();
        assert!(!encoded.contains("path"));
        assert!(!encoded.contains("calibration"));
    }
}
