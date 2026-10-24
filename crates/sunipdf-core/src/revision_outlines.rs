//! Reviewed raster differences, recorded as editable native PDF revision outlines.
//!
//! A pixel difference is a proposal, never a semantic assertion. Both saved source files,
//! options and individual proposals are bound to the review. Publication creates a new copy
//! of the complete new PDF; neither saved input is changed.

use crate::annot_rich::{vertex_annotation_components, VertexAnnotKind};
use crate::annot_write::{self, AnnotStyle};
use crate::geometry::PageGeometry;
use crate::local_review::digest;
use crate::revision_overlay::{OverlayAlignment, OverlayTransform};
use crate::revision_transfer::read_source;
use crate::{Document, Engine, EngineError, RenderedPage};
use lopdf::Object;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_REGIONS: usize = 512;
pub const MAX_SELECTIONS: usize = 64;
pub const MAX_PREVIEW_SIDE: u32 = 1536;
const TRACE_KEY: &[u8] = b"SuniPDFRevisionOutline";
const MAX_REVIEW_BYTES: usize = 256 * 1024;
const MAX_CANDIDATE_BYTES: usize = 160 * 1024 * 1024;
const OUTLINE_MARGIN: f32 = 5.5;

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Revision outlines: {message}"))
}
fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}
fn hash_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[derive(Clone, Copy, Default)]
pub struct OutlinePasswords<'a> {
    pub old: Option<&'a str>,
    pub new: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineOptions {
    pub old_page: usize,
    pub new_page: usize,
    /// Maps OLD to NEW in displayed points with a top-left origin.
    pub alignment: OverlayAlignment,
    pub long_side: u32,
    pub channel_tolerance: u8,
    pub min_component_pixels: u32,
    pub merge_gap_pixels: u32,
}
impl Default for OutlineOptions {
    fn default() -> Self {
        Self {
            old_page: 1,
            new_page: 1,
            alignment: OverlayAlignment::default(),
            long_side: 1536,
            channel_tolerance: 16,
            min_component_pixels: 4,
            merge_gap_pixels: 2,
        }
    }
}
impl OutlineOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.old_page == 0
            || self.new_page == 0
            || !(64..=MAX_PREVIEW_SIDE).contains(&self.long_side)
            || !(1..=254).contains(&self.channel_tolerance)
            || !(1..=10_000).contains(&self.min_component_pixels)
            || self.merge_gap_pixels > 16
        {
            return Err(invalid(
                "invalid page, raster side, tolerance, minimum component size or merge gap",
            ));
        }
        self.transform().map(|_| ())
    }
    fn transform(&self) -> Result<OverlayTransform, EngineError> {
        match self.alignment {
            OverlayAlignment::Manual { transform } => {
                transform.validate()?;
                Ok(transform)
            }
            OverlayAlignment::TwoPoints { base, revision } => {
                OverlayTransform::from_anchors(base, revision)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutlineClassification {
    ContentChange,
    RenderingNoise,
    Uncertain,
}
impl OutlineClassification {
    pub fn label(self) -> &'static str {
        match self {
            Self::ContentChange => "Content change",
            Self::RenderingNoise => "Rendering noise",
            Self::Uncertain => "Uncertain",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineSelection {
    pub id: String,
    pub fingerprint: String,
    pub classification: OutlineClassification,
    pub confirmed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineReview {
    pub version: u32,
    pub old_sha256: String,
    pub new_sha256: String,
    pub options: OutlineOptions,
    pub selections: Vec<OutlineSelection>,
}
impl OutlineReview {
    fn validate(&self) -> Result<(), EngineError> {
        self.options.validate()?;
        if self.version != 1
            || !hash_valid(&self.old_sha256)
            || !hash_valid(&self.new_sha256)
            || self.selections.is_empty()
            || self.selections.len() > MAX_SELECTIONS
        {
            return Err(invalid(
                "review requires version 1, saved-source hashes and 1 to 64 decisions",
            ));
        }
        let mut seen = HashSet::new();
        if self.selections.iter().any(|s| {
            !s.confirmed
                || !hash_valid(&s.fingerprint)
                || s.id.len() > 80
                || !s.id.starts_with("region-")
                || !seen.insert(&s.id)
        }) {
            return Err(invalid(
                "every decision needs a unique proposal ID, fingerprint and explicit confirmation",
            ));
        }
        Ok(())
    }
}
pub fn parse_review(bytes: &[u8]) -> Result<OutlineReview, EngineError> {
    if bytes.len() > MAX_REVIEW_BYTES {
        return Err(invalid("review exceeds 256 KiB"));
    }
    let review: OutlineReview =
        serde_json::from_slice(bytes).map_err(|_| invalid("invalid review JSON"))?;
    review.validate()?;
    Ok(review)
}
pub fn encode_review(review: &OutlineReview) -> Result<Vec<u8>, EngineError> {
    review.validate()?;
    let bytes = serde_json::to_vec_pretty(review).map_err(|_| invalid("invalid review values"))?;
    if bytes.len() > MAX_REVIEW_BYTES {
        return Err(invalid("review exceeds 256 KiB"));
    }
    Ok(bytes)
}

#[derive(Clone, Copy, Debug)]
pub struct OutlineProgress {
    pub phase: &'static str,
    pub done: usize,
    pub total: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutlineGeometry {
    pub page_count: usize,
    pub page: usize,
    pub display_size_pt: [f32; 2],
    pub crop_box: [f32; 4],
    pub rotation_degrees: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutlineRegion {
    pub id: String,
    pub fingerprint: String,
    /// Half-open top-left raster bounds.
    pub pixel_bounds: [u32; 4],
    /// Bottom-left displayed NEW page coordinates, without the cloud stroke margin.
    pub rect: [f32; 4],
    pub changed_pixels: usize,
    pub status: String,
}
#[derive(Clone, Debug)]
pub struct OutlineImages {
    pub old: RenderedPage,
    pub new: RenderedPage,
    pub difference: RenderedPage,
}
#[derive(Clone, Debug, Serialize)]
pub struct OutlineInspection {
    pub version: u32,
    pub old_sha256: String,
    pub new_sha256: String,
    pub options: OutlineOptions,
    pub transform: OverlayTransform,
    pub old_geometry: OutlineGeometry,
    pub new_geometry: OutlineGeometry,
    pub raster_size: [u32; 2],
    pub changed_pixels: usize,
    pub compared_pixels: usize,
    pub unmapped_pixels: usize,
    /// Old raster pixels mapped outside the new canvas, also outside comparison coverage.
    pub unmapped_old_pixels: usize,
    pub filtered_pixels: usize,
    pub omitted_regions: usize,
    pub truncated: bool,
    pub regions: Vec<OutlineRegion>,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub images: Option<OutlineImages>,
}
impl OutlineInspection {
    pub fn complete(&self) -> bool {
        !self.truncated
            && self.omitted_regions == 0
            && self.unmapped_pixels == 0
            && self.unmapped_old_pixels == 0
            && self.filtered_pixels == 0
    }
    pub fn json(&self) -> Value {
        json!({"complete":self.complete(), "inspection":self})
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CreatedOutline {
    pub region_id: String,
    pub page: usize,
    pub index: usize,
    pub name: String,
    pub classification: OutlineClassification,
    pub rect: [f32; 4],
}
#[derive(Clone, Debug, Serialize)]
pub struct OutlineReport {
    pub version: u32,
    pub inspection: OutlineInspection,
    pub decisions: Vec<OutlineSelection>,
    pub created: Vec<CreatedOutline>,
    /// Detected proposals without a confirmed decision. Confirmed noise is in decisions.
    pub excluded_regions: Vec<String>,
    pub output_path: Option<PathBuf>,
    pub output_sha256: Option<String>,
}
impl OutlineReport {
    pub fn complete(&self) -> bool {
        self.inspection.complete() && self.excluded_regions.is_empty()
    }
    pub fn can_publish(&self) -> bool {
        !self.created.is_empty()
    }
    pub fn json(&self) -> Value {
        json!({"complete":self.complete(), "can_publish":self.can_publish(), "report":self})
    }
}

#[derive(Debug)]
struct SourceBinding {
    selection: PathBuf,
    canonical: PathBuf,
    sha256: String,
}
impl SourceBinding {
    fn read(path: &Path, cancel: &AtomicBool) -> Result<(Self, Vec<u8>), EngineError> {
        let selection = std::path::absolute(path)?;
        let canonical = selection.canonicalize()?;
        let bytes = read_source(&canonical, cancel)?;
        if selection.canonicalize()? != canonical {
            return Err(invalid("selected source path changed while reading"));
        }
        Ok((
            Self {
                selection,
                canonical,
                sha256: digest(&bytes),
            },
            bytes,
        ))
    }
    fn revalidate(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        if self.selection.canonicalize()? != self.canonical {
            return Err(invalid("selected source path changed; inspect again"));
        }
        if digest(&read_source(&self.canonical, cancel)?) != self.sha256 {
            return Err(invalid("source revision changed; inspect again"));
        }
        if self.selection.canonicalize()? != self.canonical {
            return Err(invalid("selected source path changed while reading"));
        }
        cancelled(cancel)
    }
}
#[derive(Debug)]
pub struct PreparedRevisionOutlines {
    old: SourceBinding,
    new: SourceBinding,
    bytes: Vec<u8>,
    report: OutlineReport,
}
impl PreparedRevisionOutlines {
    pub fn report(&self) -> &OutlineReport {
        &self.report
    }
    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        self.old.revalidate(cancel)?;
        self.new.revalidate(cancel)
    }
    pub fn render_preview(
        &self,
        engine: &Engine,
        new_password: Option<&str>,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        self.revalidate_sources(cancel)?;
        if !(64..=MAX_PREVIEW_SIDE).contains(&long_side) {
            return Err(invalid("preview side must be 64 to 1536 pixels"));
        }
        let doc = engine.open_bytes(self.bytes.clone(), new_password, &self.new.canonical)?;
        if !doc.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let image = render(&doc, self.report.inspection.options.new_page, long_side)?;
        self.revalidate_sources(cancel)?;
        Ok(image)
    }
}

fn geometry(
    doc: &Document<'_>,
    page: usize,
) -> Result<(PageGeometry, OutlineGeometry), EngineError> {
    if page == 0 || page > doc.page_count() {
        return Err(EngineError::PageOutOfRange(page.saturating_sub(1)));
    }
    let geo = doc.page_geometry(page - 1)?;
    let (w, h) = geo.display_size();
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(invalid("invalid page geometry"));
    }
    Ok((
        geo,
        OutlineGeometry {
            page_count: doc.page_count(),
            page,
            display_size_pt: [w, h],
            crop_box: [geo.bbox.left, geo.bbox.bottom, geo.bbox.right, geo.bbox.top],
            rotation_degrees: geo.rotation.degrees(),
        },
    ))
}
fn render(doc: &Document<'_>, page: usize, side: u32) -> Result<RenderedPage, EngineError> {
    let (_, geo) = geometry(doc, page)?;
    let [w, h] = geo.display_size_pt;
    let image = doc.render_page(page - 1, side as f32 / w.max(h))?;
    if image.width == 0
        || image.height == 0
        || image.width > MAX_PREVIEW_SIDE
        || image.height > MAX_PREVIEW_SIDE
        || image.rgba.len() != image.width as usize * image.height as usize * 4
    {
        return Err(EngineError::RenderTooLarge);
    }
    Ok(image)
}
fn check_permissions(
    old: &Document<'_>,
    new: &Document<'_>,
    raw_new: &[u8],
) -> Result<(), EngineError> {
    if !old.properties().can_copy || !new.properties().can_copy || !new.properties().can_annotate {
        return Err(EngineError::SecurityRestricted);
    }
    let pdf = crate::security::load(raw_new, new.password())?;
    if new.signature_count() > 0
        || pdf.objects.values().any(|o| {
            o.as_dict().is_ok_and(|d| {
                d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Sig".as_slice())
                    || (d.get(b"FT").and_then(Object::as_name).ok() == Some(b"Sig".as_slice())
                        && d.get(b"V").is_ok_and(|v| !matches!(v, Object::Null)))
            })
        })
    {
        return Err(invalid(
            "signed new documents cannot receive revision outlines",
        ));
    }
    Ok(())
}
fn anchors_inside(
    options: &OutlineOptions,
    old: &OutlineGeometry,
    new: &OutlineGeometry,
) -> Result<(), EngineError> {
    if let OverlayAlignment::TwoPoints { base, revision } = options.alignment {
        for (points, geo) in [(base, new), (revision, old)] {
            if points.iter().any(|p| {
                p[0] < 0.0
                    || p[1] < 0.0
                    || p[0] > f64::from(geo.display_size_pt[0])
                    || p[1] > f64::from(geo.display_size_pt[1])
            }) {
                return Err(invalid(
                    "alignment anchors must lie inside their displayed pages",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Component {
    bounds: [u32; 4],
    pixels: usize,
}
fn near(a: [u32; 4], b: [u32; 4], gap: u32) -> bool {
    a[0] <= b[2].saturating_add(gap)
        && b[0] <= a[2].saturating_add(gap)
        && a[1] <= b[3].saturating_add(gap)
        && b[1] <= a[3].saturating_add(gap)
}
fn components(
    mask: &mut [u8],
    w: u32,
    h: u32,
    options: &OutlineOptions,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(OutlineProgress),
) -> Result<(Vec<Component>, usize, usize), EngineError> {
    let mut found: Vec<Component> = Vec::new();
    let mut filtered = 0;
    let mut omitted = 0;
    let mut queue: Vec<u32> = Vec::new();
    for y in 0..h {
        if y % 32 == 0 {
            cancelled(cancel)?;
            progress(OutlineProgress {
                phase: "components",
                done: y as usize,
                total: h as usize,
            });
        }
        for x in 0..w {
            let start = y * w + x;
            if mask[start as usize] != 1 {
                continue;
            }
            queue.clear();
            queue.push(start);
            mask[start as usize] = 2;
            let mut head = 0;
            let mut bounds = [x, y, x + 1, y + 1];
            while head < queue.len() {
                if head % 4096 == 0 {
                    cancelled(cancel)?;
                }
                let index = queue[head];
                head += 1;
                let px = index % w;
                let py = index / w;
                bounds = [
                    bounds[0].min(px),
                    bounds[1].min(py),
                    bounds[2].max(px + 1),
                    bounds[3].max(py + 1),
                ];
                for ny in py.saturating_sub(1)..=(py + 1).min(h - 1) {
                    for nx in px.saturating_sub(1)..=(px + 1).min(w - 1) {
                        let next = ny * w + nx;
                        if mask[next as usize] == 1 {
                            mask[next as usize] = 2;
                            queue.push(next);
                        }
                    }
                }
            }
            if queue.len() < options.min_component_pixels as usize {
                filtered += queue.len();
            } else if found.len() < MAX_REGIONS {
                found.push(Component {
                    bounds,
                    pixels: queue.len(),
                });
            } else {
                omitted += 1;
            }
        }
    }
    // Deterministic transitive union of the retained bounded set. Pixels are disjoint.
    let mut i = 0;
    while i < found.len() {
        cancelled(cancel)?;
        let mut merged = false;
        for j in (i + 1..found.len()).rev() {
            if near(found[i].bounds, found[j].bounds, options.merge_gap_pixels) {
                let b = found.remove(j);
                let a = &mut found[i];
                a.bounds = [
                    a.bounds[0].min(b.bounds[0]),
                    a.bounds[1].min(b.bounds[1]),
                    a.bounds[2].max(b.bounds[2]),
                    a.bounds[3].max(b.bounds[3]),
                ];
                a.pixels += b.pixels;
                merged = true;
            }
        }
        if merged {
            i = 0;
        } else {
            i += 1;
        }
    }
    found.sort_by_key(|c| (c.bounds[1], c.bounds[0], c.bounds[3], c.bounds[2]));
    progress(OutlineProgress {
        phase: "components",
        done: h as usize,
        total: h as usize,
    });
    Ok((found, filtered, omitted))
}

/// Bilinear sampling at pixel centres, composited on white; comparison is RGB, not luminance.
fn sample(image: &RenderedPage, x: f64, y: f64) -> [u8; 3] {
    let x = x.clamp(0.0, f64::from(image.width - 1));
    let y = y.clamp(0.0, f64::from(image.height - 1));
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width - 1);
    let y1 = (y0 + 1).min(image.height - 1);
    let dx = x - f64::from(x0);
    let dy = y - f64::from(y0);
    let mut rgb = [0.0; 3];
    for (px, py, weight) in [
        (x0, y0, (1.0 - dx) * (1.0 - dy)),
        (x1, y0, dx * (1.0 - dy)),
        (x0, y1, (1.0 - dx) * dy),
        (x1, y1, dx * dy),
    ] {
        let i = (py as usize * image.width as usize + px as usize) * 4;
        let alpha = f64::from(image.rgba[i + 3]) / 255.0;
        for (channel, value) in rgb.iter_mut().enumerate() {
            *value += weight * (f64::from(image.rgba[i + channel]) * alpha + 255.0 * (1.0 - alpha));
        }
    }
    rgb.map(|v| v.round().clamp(0.0, 255.0) as u8)
}
fn points(rect: [f32; 4]) -> [(f32, f32); 4] {
    [
        (rect[0], rect[1]),
        (rect[2], rect[1]),
        (rect[2], rect[3]),
        (rect[0], rect[3]),
    ]
}
fn outline_fits(rect: [f32; 4], geo: PageGeometry) -> bool {
    let (w, h) = geo.display_size();
    let bounds = crate::PointRect {
        left: rect[0] - OUTLINE_MARGIN,
        bottom: rect[1] - OUTLINE_MARGIN,
        right: rect[2] + OUTLINE_MARGIN,
        top: rect[3] + OUTLINE_MARGIN,
    };
    let stored = geo.rect_to_display(geo.rect_to_user(bounds));
    rect[0] >= OUTLINE_MARGIN
        && rect[1] >= OUTLINE_MARGIN
        && rect[2] + OUTLINE_MARGIN <= w
        && rect[3] + OUTLINE_MARGIN <= h
        && rect[2] - rect[0] >= 0.01
        && rect[3] - rect[1] >= 0.01
        && rect[2] - rect[0] + 2.0 * OUTLINE_MARGIN <= 10_000.0
        && rect[3] - rect[1] + 2.0 * OUTLINE_MARGIN <= 10_000.0
        && [
            (stored.left, bounds.left),
            (stored.bottom, bounds.bottom),
            (stored.right, bounds.right),
            (stored.top, bounds.top),
        ]
        .iter()
        .all(|(actual, wanted)| (*actual - *wanted).abs() <= 0.01)
        && points(rect).iter().all(|&(x, y)| {
            let (ux, uy) = geo.to_user(x, y);
            let (rx, ry) = geo.to_display(ux, uy);
            (rx - x).abs() <= 0.01 && (ry - y).abs() <= 0.01
        })
}

#[allow(clippy::too_many_arguments)]
fn inspect_loaded(
    old: &Document<'_>,
    new: &Document<'_>,
    old_hash: &str,
    new_hash: &str,
    options: &OutlineOptions,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(OutlineProgress),
) -> Result<OutlineInspection, EngineError> {
    options.validate()?;
    let (_, old_geometry) = geometry(old, options.old_page)?;
    let (new_geo, new_geometry) = geometry(new, options.new_page)?;
    anchors_inside(options, &old_geometry, &new_geometry)?;
    let transform = options.transform()?;
    progress(OutlineProgress {
        phase: "render",
        done: 0,
        total: 2,
    });
    cancelled(cancel)?;
    let old_image = render(old, options.old_page, options.long_side)?;
    progress(OutlineProgress {
        phase: "render",
        done: 1,
        total: 2,
    });
    cancelled(cancel)?;
    let new_image = render(new, options.new_page, options.long_side)?;
    progress(OutlineProgress {
        phase: "render",
        done: 2,
        total: 2,
    });
    cancelled(cancel)?;
    let w = new_image.width;
    let h = new_image.height;
    let count = w as usize * h as usize;
    let mut mask = vec![0_u8; count];
    let mut difference = RenderedPage {
        page_index: options.new_page - 1,
        width: w,
        height: h,
        rgba: vec![255; count * 4],
    };
    let mut changed = 0;
    let mut unmapped = 0;
    let (sin, cos) = transform.rotation_degrees.to_radians().sin_cos();
    let [ow, oh] = old_geometry.display_size_pt.map(f64::from);
    let [nw, nh] = new_geometry.display_size_pt.map(f64::from);
    let mut unmapped_old = 0;
    for y in 0..old_image.height {
        if y % 32 == 0 {
            cancelled(cancel)?;
            progress(OutlineProgress {
                phase: "coverage",
                done: y as usize,
                total: old_image.height as usize,
            });
        }
        for x in 0..old_image.width {
            let mapped = transform.apply([
                (f64::from(x) + 0.5) * ow / f64::from(old_image.width),
                (f64::from(y) + 0.5) * oh / f64::from(old_image.height),
            ]);
            if mapped[0] < 0.0 || mapped[1] < 0.0 || mapped[0] >= nw || mapped[1] >= nh {
                unmapped_old += 1;
            }
        }
    }
    for y in 0..h {
        if y % 32 == 0 {
            cancelled(cancel)?;
            progress(OutlineProgress {
                phase: "compare",
                done: y as usize,
                total: h as usize,
            });
        }
        for x in 0..w {
            let nx = (f64::from(x) + 0.5) * nw / f64::from(w) - transform.translation[0];
            let ny = (f64::from(y) + 0.5) * nh / f64::from(h) - transform.translation[1];
            let ox = (cos * nx + sin * ny) / transform.scale;
            let oy = (-sin * nx + cos * ny) / transform.scale;
            let i = y as usize * w as usize + x as usize;
            if ox < 0.0 || oy < 0.0 || ox >= ow || oy >= oh {
                unmapped += 1;
                difference.rgba[i * 4..i * 4 + 4].copy_from_slice(&[220, 220, 220, 255]);
                continue;
            }
            let a = sample(
                &old_image,
                ox / ow * f64::from(old_image.width) - 0.5,
                oy / oh * f64::from(old_image.height) - 0.5,
            );
            let b = sample(&new_image, f64::from(x), f64::from(y));
            if (0..3).any(|c| a[c].abs_diff(b[c]) > options.channel_tolerance) {
                mask[i] = 1;
                changed += 1;
                difference.rgba[i * 4..i * 4 + 4].copy_from_slice(&[210, 30, 60, 255]);
            }
        }
    }
    progress(OutlineProgress {
        phase: "compare",
        done: h as usize,
        total: h as usize,
    });
    let (found, filtered, omitted) = components(&mut mask, w, h, options, cancel, progress)?;
    drop(mask);
    let mut regions = Vec::new();
    for c in found {
        let b = c.bounds;
        let rect = [
            (f64::from(b[0]) * nw / f64::from(w)) as f32,
            (nh - f64::from(b[3]) * nh / f64::from(h)) as f32,
            (f64::from(b[2]) * nw / f64::from(w)) as f32,
            (nh - f64::from(b[1]) * nh / f64::from(h)) as f32,
        ];
        let fingerprint=digest(&serde_json::to_vec(&json!({"version":1,"old":old_hash,"new":new_hash,"options":options,
            "transform":transform,"old_geometry":old_geometry,"new_geometry":new_geometry,"bounds":b,"pixels":c.pixels})).map_err(|_|invalid("invalid fingerprint values"))?);
        let status = if outline_fits(rect, new_geo) {
            "review_required"
        } else {
            "outline_out_of_bounds"
        };
        regions.push(OutlineRegion {
            id: format!("region-{}", &fingerprint[..24]),
            fingerprint,
            pixel_bounds: b,
            rect,
            changed_pixels: c.pixels,
            status: status.into(),
        });
    }
    let mut warnings=vec!["Visible rendered differences include annotations and forms; classification requires human review and is not a vector or semantic comparison.".into()];
    if unmapped > 0 || unmapped_old > 0 {
        warnings.push("Some new-page pixels have no mapped old-page coverage; they were not compared. Old content outside the new page is not inspected.".into());
    }
    if filtered > 0 {
        warnings.push("Small components were filtered; the inspection is partial.".into());
    }
    if omitted > 0 {
        warnings.push(
            "More than 512 components were found; omitted regions make this inspection partial."
                .into(),
        );
    }
    if regions.iter().any(|r| r.status != "review_required") {
        warnings.push("Some regions cannot receive a complete cloud within the page or precise storage bounds. Leave them unselected or adjust the inspection; classification must reflect the review, not a storage workaround.".into());
    }
    Ok(OutlineInspection {
        version: 1,
        old_sha256: old_hash.into(),
        new_sha256: new_hash.into(),
        options: options.clone(),
        transform,
        old_geometry,
        new_geometry,
        raster_size: [w, h],
        changed_pixels: changed,
        compared_pixels: count - unmapped,
        unmapped_pixels: unmapped,
        unmapped_old_pixels: unmapped_old,
        filtered_pixels: filtered,
        omitted_regions: omitted,
        truncated: omitted > 0,
        regions,
        warnings,
        images: Some(OutlineImages {
            old: old_image,
            new: new_image,
            difference,
        }),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn inspect_revision_outlines(
    engine: &Engine,
    old_path: &Path,
    new_path: &Path,
    options: &OutlineOptions,
    passwords: OutlinePasswords<'_>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(OutlineProgress),
) -> Result<OutlineInspection, EngineError> {
    options.validate()?;
    progress(OutlineProgress {
        phase: "read",
        done: 0,
        total: 2,
    });
    let (old_binding, old_bytes) = SourceBinding::read(old_path, cancel)?;
    progress(OutlineProgress {
        phase: "read",
        done: 1,
        total: 2,
    });
    let (new_binding, new_bytes) = SourceBinding::read(new_path, cancel)?;
    progress(OutlineProgress {
        phase: "read",
        done: 2,
        total: 2,
    });
    let old = engine.open_bytes(old_bytes, passwords.old, &old_binding.canonical)?;
    let new = engine.open_bytes(new_bytes.clone(), passwords.new, &new_binding.canonical)?;
    check_permissions(&old, &new, &new_bytes)?;
    let inspection = inspect_loaded(
        &old,
        &new,
        &old_binding.sha256,
        &new_binding.sha256,
        options,
        cancel,
        &mut progress,
    )?;
    old_binding.revalidate(cancel)?;
    new_binding.revalidate(cancel)?;
    Ok(inspection)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineTrace {
    pub version: u32,
    pub old_sha256: String,
    pub new_sha256: String,
    pub options: OutlineOptions,
    pub transform: OverlayTransform,
    pub region: OutlineRegion,
    pub classification: OutlineClassification,
    /// Original user-space vertices make later annotation movement observable.
    pub original_vertices_user: Vec<f32>,
    pub original_rect_user: [f32; 4],
    pub original_crop_box: [f32; 4],
    pub original_rotation_degrees: u16,
}
#[derive(Clone, Debug, Serialize)]
pub struct OutlineRecord {
    pub page: usize,
    pub index: usize,
    pub name: Option<String>,
    pub trace: OutlineTrace,
    pub geometry_matches_original: bool,
}
impl Document<'_> {
    /// Reads provenance as historical source identity. It does not verify external source files.
    pub fn revision_outline_records(&self, page: usize) -> Result<Vec<OutlineRecord>, EngineError> {
        let (_, geometry) = geometry(self, page)?;
        let bytes = self.snapshot()?;
        let pdf = crate::security::load(&bytes, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page as u32))
            .ok_or(EngineError::PageOutOfRange(page - 1))?;
        let dict = pdf
            .get_dictionary(page_id)
            .map_err(crate::security::lopdf_err)?;
        let Ok(annots) = dict.get(b"Annots") else {
            return Ok(Vec::new());
        };
        let (_, annots) = pdf
            .dereference(annots)
            .map_err(crate::security::lopdf_err)?;
        let annots = annots.as_array().map_err(crate::security::lopdf_err)?;
        if annots.len() > 10000 {
            return Err(invalid("annotation list exceeds inspection limit"));
        }
        let mut records = Vec::new();
        for (index, annot) in annots.iter().enumerate() {
            let (_, annot) = pdf.dereference(annot).map_err(crate::security::lopdf_err)?;
            let dict = annot.as_dict().map_err(crate::security::lopdf_err)?;
            let Ok(raw) = dict.get(TRACE_KEY) else {
                continue;
            };
            let raw = raw
                .as_str()
                .map_err(|_| invalid("unsupported revision outline provenance"))?;
            if raw.len() > 16 * 1024 {
                return Err(invalid("revision outline provenance is too large"));
            }
            if records.len() >= MAX_REGIONS {
                return Err(invalid(
                    "revision outline provenance exceeds 512 records on this page",
                ));
            }
            let trace: OutlineTrace = serde_json::from_slice(raw)
                .map_err(|_| invalid("malformed revision outline provenance"))?;
            trace.options.validate()?;
            trace.transform.validate()?;
            if trace.version != 1
                || !hash_valid(&trace.old_sha256)
                || !hash_valid(&trace.new_sha256)
                || !hash_valid(&trace.region.fingerprint)
                || trace.original_vertices_user.len() != 8
                || trace.original_vertices_user.iter().any(|n| !n.is_finite())
                || trace.region.rect.iter().any(|n| !n.is_finite())
                || trace.original_rect_user.iter().any(|n| !n.is_finite())
                || trace.original_crop_box.iter().any(|n| !n.is_finite())
                || !matches!(trace.original_rotation_degrees, 0 | 90 | 180 | 270)
                || trace.classification == OutlineClassification::RenderingNoise
                || trace.region.id != format!("region-{}", &trace.region.fingerprint[..24])
                || trace.region.changed_pixels == 0
            {
                return Err(invalid("unsupported revision outline provenance"));
            }
            let actual = dict
                .get(b"Vertices")
                .ok()
                .and_then(|v| v.as_array().ok())
                .and_then(|v| {
                    v.iter()
                        .map(|n| n.as_float().ok())
                        .collect::<Option<Vec<_>>>()
                });
            let geometry_matches_original = actual
                .as_ref()
                .is_some_and(|v| v == &trace.original_vertices_user)
                && dict
                    .get(b"Rect")
                    .ok()
                    .and_then(|v| v.as_array().ok())
                    .is_some_and(|v| {
                        v.len() == 4
                            && v.iter()
                                .zip(trace.original_rect_user)
                                .all(|(a, b)| a.as_float().ok() == Some(b))
                    })
                && geometry.crop_box == trace.original_crop_box
                && geometry.rotation_degrees == trace.original_rotation_degrees;
            records.push(OutlineRecord {
                page,
                index,
                name: annot_write::text_of(&pdf, dict, b"NM"),
                trace,
                geometry_matches_original,
            });
        }
        Ok(records)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_revision_outlines(
    engine: &Engine,
    old_path: &Path,
    new_path: &Path,
    review: &OutlineReview,
    passwords: OutlinePasswords<'_>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(OutlineProgress),
) -> Result<PreparedRevisionOutlines, EngineError> {
    review.validate()?;
    let (old_binding, old_bytes) = SourceBinding::read(old_path, cancel)?;
    let (new_binding, new_bytes) = SourceBinding::read(new_path, cancel)?;
    if old_binding.sha256 != review.old_sha256 || new_binding.sha256 != review.new_sha256 {
        return Err(invalid("review does not match the saved source revisions"));
    }
    let old = engine.open_bytes(old_bytes, passwords.old, &old_binding.canonical)?;
    let mut new = engine.open_bytes(new_bytes.clone(), passwords.new, &new_binding.canonical)?;
    check_permissions(&old, &new, &new_bytes)?;
    let mut inspection = inspect_loaded(
        &old,
        &new,
        &old_binding.sha256,
        &new_binding.sha256,
        &review.options,
        cancel,
        &mut progress,
    )?;
    inspection.images = None;
    let selected: HashSet<_> = review.selections.iter().map(|s| s.id.as_str()).collect();
    let excluded_regions = inspection
        .regions
        .iter()
        .filter(|r| !selected.contains(r.id.as_str()))
        .map(|r| r.id.clone())
        .collect();
    let mut pending = Vec::new();
    for selection in &review.selections {
        let region = inspection
            .regions
            .iter()
            .find(|r| r.id == selection.id && r.fingerprint == selection.fingerprint)
            .ok_or_else(|| invalid("review contains a stale or unknown proposal"))?;
        if selection.classification != OutlineClassification::RenderingNoise {
            if region.status != "review_required" {
                return Err(invalid(
                    "selected region cannot be outlined within the page and precise storage bounds",
                ));
            }
            pending.push((region.clone(), selection.classification));
        }
    }
    let mut created = Vec::new();
    let page_index = review.options.new_page - 1;
    if !pending.is_empty() {
        new.edit_annotation_pdf(page_index, |pdf, page, geo| {
            for (ordinal, (region, classification)) in pending.iter().enumerate() {
                cancelled(cancel)?;
                progress(OutlineProgress {
                    phase: "create",
                    done: ordinal,
                    total: pending.len(),
                });
                let style = AnnotStyle {
                    rgb: if *classification == OutlineClassification::Uncertain {
                        (208, 116, 0)
                    } else {
                        (210, 30, 60)
                    },
                    ..AnnotStyle::default()
                };
                let (mut annotation, stream) = vertex_annotation_components(
                    geo,
                    VertexAnnotKind::Cloud,
                    &points(region.rect),
                    &style,
                    None,
                )?;
                let trace = OutlineTrace {
                    version: 1,
                    old_sha256: review.old_sha256.clone(),
                    new_sha256: review.new_sha256.clone(),
                    options: review.options.clone(),
                    transform: inspection.transform,
                    region: region.clone(),
                    classification: *classification,
                    original_vertices_user: points(region.rect)
                        .iter()
                        .flat_map(|&(x, y)| {
                            let (x, y) = geo.to_user(x, y);
                            [x, y]
                        })
                        .collect(),
                    original_rect_user: {
                        let rect = annotation
                            .get(b"Rect")
                            .and_then(Object::as_array)
                            .map_err(crate::security::lopdf_err)?;
                        if rect.len() != 4 {
                            return Err(invalid("new outline rectangle is invalid"));
                        }
                        [
                            rect[0].as_float().map_err(crate::security::lopdf_err)?,
                            rect[1].as_float().map_err(crate::security::lopdf_err)?,
                            rect[2].as_float().map_err(crate::security::lopdf_err)?,
                            rect[3].as_float().map_err(crate::security::lopdf_err)?,
                        ]
                    },
                    original_crop_box: inspection.new_geometry.crop_box,
                    original_rotation_degrees: inspection.new_geometry.rotation_degrees,
                };
                let trace_bytes =
                    serde_json::to_vec(&trace).map_err(|_| invalid("invalid provenance values"))?;
                annotation.set(TRACE_KEY, Object::string_literal(trace_bytes));
                annotation.set("Subj", lopdf::text_string("Revision difference"));
                annotation.set(
                    "Contents",
                    lopdf::text_string(&format!(
                        "{} — reviewed raster region {}",
                        classification.label(),
                        region.id
                    )),
                );
                let fields = crate::annotation_register::RegisterFields {
                    business_status: Some(classification.label().into()),
                    discipline: None,
                    custom: BTreeMap::from([
                        ("Revision region".into(), region.id.clone()),
                        ("Old SHA256".into(), review.old_sha256.clone()),
                        ("New SHA256".into(), review.new_sha256.clone()),
                    ]),
                };
                fields.validate()?;
                annotation.set(
                    "SuniPDFRegister",
                    Object::string_literal(
                        serde_json::to_vec(&json!({"version":1,"fields":fields}))
                            .map_err(|_| invalid("invalid register fields"))?,
                    ),
                );
                let name = annot_write::text_of(pdf, &annotation, b"NM")
                    .ok_or_else(|| invalid("new outline has no identity"))?;
                let index = annot_write::push_annotation(pdf, page, annotation, Some(stream))?;
                created.push(CreatedOutline {
                    region_id: region.id.clone(),
                    page: review.options.new_page,
                    index,
                    name,
                    classification: *classification,
                    rect: region.rect,
                });
            }
            Ok(())
        })?;
    }
    progress(OutlineProgress {
        phase: "create",
        done: pending.len(),
        total: pending.len(),
    });
    cancelled(cancel)?;
    let bytes = if created.is_empty() {
        new_bytes
    } else {
        new.snapshot()?
    };
    if bytes.len() > MAX_CANDIDATE_BYTES {
        return Err(invalid("candidate exceeds 160 MiB"));
    }
    let reread = engine.open_bytes(bytes.clone(), passwords.new, &new_binding.canonical)?;
    let records = reread.revision_outline_records(review.options.new_page)?;
    if created.iter().any(|c| {
        !records.iter().any(|r| {
            r.index == c.index
                && r.name.as_deref() == Some(c.name.as_str())
                && r.trace.region.id == c.region_id
                && r.geometry_matches_original
        })
    }) {
        return Err(invalid(
            "candidate outline provenance did not survive serialization",
        ));
    }
    let prepared = PreparedRevisionOutlines {
        old: old_binding,
        new: new_binding,
        bytes,
        report: OutlineReport {
            version: 1,
            inspection,
            decisions: review.selections.clone(),
            created,
            excluded_regions,
            output_path: None,
            output_sha256: None,
        },
    };
    prepared.revalidate_sources(cancel)?;
    Ok(prepared)
}

/// The commit point is atomic no-clobber publication; a later cancellation cannot revoke it.
pub fn publish_revision_outlines(
    mut prepared: PreparedRevisionOutlines,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<OutlineReport, EngineError> {
    cancelled(cancel)?;
    if !prepared.report.can_publish() {
        return Err(invalid(
            "no outline was created; a noise-only review cannot publish a PDF",
        ));
    }
    if output.symlink_metadata().is_ok() {
        return Err(EngineError::AlreadyExists(output.into()));
    }
    let directory = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !directory.is_dir() {
        return Err(invalid("choose an existing destination directory"));
    }
    let directory_canonical = directory.canonicalize()?;
    let mut temporary = tempfile::NamedTempFile::new_in(&directory_canonical)?;
    for chunk in prepared.bytes.chunks(1024 * 1024) {
        cancelled(cancel)?;
        temporary.write_all(chunk)?;
    }
    temporary.as_file().sync_all()?;
    prepared.revalidate_sources(cancel)?;
    if directory.canonicalize()? != directory_canonical {
        return Err(invalid("destination directory changed during preparation"));
    }
    cancelled(cancel)?;
    let filename = output
        .file_name()
        .ok_or_else(|| invalid("output requires a filename"))?;
    let final_path = directory_canonical.join(filename);
    temporary.persist_noclobber(&final_path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            EngineError::AlreadyExists(output.into())
        } else {
            EngineError::Io(e.error)
        }
    })?;
    prepared.report.output_path = Some(final_path);
    prepared.report.output_sha256 = Some(digest(&prepared.bytes));
    Ok(prepared.report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_merge_transitively_and_report_filtered_and_omitted_work() {
        let cancel = AtomicBool::new(false);
        let options = OutlineOptions {
            min_component_pixels: 1,
            merge_gap_pixels: 1,
            ..Default::default()
        };
        let mut mask = vec![0; 100];
        for (x, y) in [(2, 2), (4, 2), (6, 2), (8, 8)] {
            mask[y * 10 + x] = 1;
        }
        let (regions, filtered, omitted) =
            components(&mut mask, 10, 10, &options, &cancel, &mut |_| {}).unwrap();
        assert_eq!((regions.len(), filtered, omitted), (2, 0, 0));
        assert_eq!(regions[0].bounds, [2, 2, 7, 3]);
        assert_eq!(regions[0].pixels, 3);
        let mut mask = vec![0; 100];
        mask[22] = 1;
        mask[23] = 1;
        mask[88] = 1;
        let (regions, filtered, omitted) = components(
            &mut mask,
            10,
            10,
            &OutlineOptions {
                min_component_pixels: 2,
                ..options.clone()
            },
            &cancel,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!((regions.len(), filtered, omitted), (1, 1, 0));
        let mut mask = vec![0; 100 * 100];
        for y in (1..100).step_by(4) {
            for x in (1..100).step_by(4) {
                mask[y * 100 + x] = 1;
            }
        }
        let (regions, filtered, omitted) = components(
            &mut mask,
            100,
            100,
            &OutlineOptions {
                merge_gap_pixels: 0,
                ..options
            },
            &cancel,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(regions.len(), MAX_REGIONS);
        assert_eq!(filtered, 0);
        assert_eq!(omitted, 625 - MAX_REGIONS);
    }

    #[test]
    fn sampler_preserves_color_differences_and_white_alpha_compositing() {
        let image = RenderedPage {
            page_index: 0,
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };
        assert_eq!(sample(&image, 0.0, 0.0), [255, 0, 0]);
        assert_eq!(sample(&image, 1.0, 0.0), [0, 255, 0]);
        assert_eq!(sample(&image, 0.5, 0.0), [128, 128, 0]);
        let transparent = RenderedPage {
            page_index: 0,
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 0],
        };
        assert_eq!(sample(&transparent, 0.0, 0.0), [255, 255, 255]);
    }
}
