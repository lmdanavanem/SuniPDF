//! Portable, bounded annotation templates. No PDF objects, actions or external resources are imported.

use crate::annot_rich::{RichAnnotation, RichFreeTextStyle, StampKind, VertexAnnotKind};
use crate::{AnnotShape, AnnotStyle, Document, EngineError, FontFamily, PointRect, TextStyle};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Cursor, Read};

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
            p.translate(-bounds[0], -bounds[1]);
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
                } => self.add_rich_free_text_annotation(
                    page,
                    None,
                    rect(*r),
                    text,
                    &style.placed(placement.scale),
                    author,
                )?,
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
    use serde_json::json;

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
