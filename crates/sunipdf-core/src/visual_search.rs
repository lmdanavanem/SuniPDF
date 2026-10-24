//! Reviewed, bounded visual symbol matching on page-content rasters.
//!
//! This is same-scale grayscale template matching, not object recognition. Scores
//! measure raster similarity, not probability. Every integer pixel position at each
//! requested clockwise quarter-turn is considered until an explicit limit/cancel.
//! Overlapping matches are suppressed; candidates never become quantities implicitly.
//! Similarity combines global absolute error with spatially balanced error on fixed
//! 2x2 and 4x4 grids. A shared outline cannot hide a mismatched small internal feature.

use crate::{Document, EngineError, PageGeometry, PixelRect, PointRect, RenderedPage, Rotation};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub const MAX_SCAN_PAGES: usize = 16;
pub const MAX_CANDIDATES: usize = 512;
pub const MAX_PAGE_SIDE: u32 = 4096;
pub const MAX_PAGE_PIXELS: u64 = 8 * 1024 * 1024;
pub const MAX_TEMPLATE_SIDE: u32 = 128;
pub const MIN_TEMPLATE_SIDE: u32 = 8;
pub const MAX_COMPARISONS: u64 = 500_000_000;
pub const MAX_STEP_COMPARISONS: usize = 65_536;
pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 160 * 1024 * 1024;

fn failure(message: &str) -> EngineError {
    EngineError::Other(format!("Visual search: {message}"))
}

mod rotations_serde {
    use super::*;
    pub fn serialize<S: serde::Serializer>(value: &[Rotation], s: S) -> Result<S::Ok, S::Error> {
        value
            .iter()
            .map(|v| v.degrees())
            .collect::<Vec<_>>()
            .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Rotation>, D::Error> {
        let degrees = Vec::<u16>::deserialize(d)?;
        if degrees.len() > 4 {
            return Err(serde::de::Error::custom(
                "at most four quarter-turns are supported",
            ));
        }
        degrees
            .into_iter()
            .map(|v| match v {
                0 => Ok(Rotation::R0),
                90 => Ok(Rotation::R90),
                180 => Ok(Rotation::R180),
                270 => Ok(Rotation::R270),
                _ => Err(serde::de::Error::custom(
                    "rotation must be 0, 90, 180 or 270",
                )),
            })
            .collect()
    }
}
fn serialize_rotation<S: serde::Serializer>(v: &Rotation, s: S) -> Result<S::Ok, S::Error> {
    v.degrees().serialize(s)
}
fn serialize_rect<S: serde::Serializer>(v: &PointRect, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut out = s.serialize_struct("PointRect", 4)?;
    out.serialize_field("left", &v.left)?;
    out.serialize_field("bottom", &v.bottom)?;
    out.serialize_field("right", &v.right)?;
    out.serialize_field("top", &v.top)?;
    out.end()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualSearchOptions {
    /// Explicit, unique zero-based page indices, scanned in the given order.
    pub pages: Vec<usize>,
    /// Fixed pixels per displayed PDF point for both template and every page.
    pub scale: f32,
    #[serde(with = "rotations_serde")]
    pub rotations: Vec<Rotation>,
    /// Minimum similarity in 0.5..=1.0. Higher values accept less raster difference.
    pub sensitivity: f32,
    pub max_candidates: usize,
    /// Counts actual grayscale pixel comparisons, including early-rejected positions.
    pub max_comparisons: u64,
}

impl Default for VisualSearchOptions {
    fn default() -> Self {
        Self {
            pages: vec![0],
            scale: 1.0,
            rotations: vec![Rotation::R0],
            sensitivity: 0.9,
            max_candidates: 256,
            max_comparisons: 100_000_000,
        }
    }
}
impl VisualSearchOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.pages.is_empty()
            || self.pages.len() > MAX_SCAN_PAGES
            || self.pages.iter().copied().collect::<HashSet<_>>().len() != self.pages.len()
        {
            return Err(failure("select 1..16 distinct pages"));
        }
        if !self.scale.is_finite() || !(0.25..=2.0).contains(&self.scale) {
            return Err(failure("render scale must be 0.25..2 pixels per point"));
        }
        if self.rotations.is_empty()
            || self.rotations.len() > 4
            || self
                .rotations
                .iter()
                .map(|r| r.degrees())
                .collect::<HashSet<_>>()
                .len()
                != self.rotations.len()
        {
            return Err(failure("select 1..4 distinct clockwise quarter-turns"));
        }
        if !self.sensitivity.is_finite() || !(0.5..=1.0).contains(&self.sensitivity) {
            return Err(failure("similarity threshold must be 0.5..1"));
        }
        if self.max_candidates == 0 || self.max_candidates > MAX_CANDIDATES {
            return Err(failure("candidate limit must be 1..512"));
        }
        if self.max_comparisons == 0 || self.max_comparisons > MAX_COMPARISONS {
            return Err(failure("pixel comparison limit must be 1..500000000"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSearchStatus {
    Running,
    Complete,
    Cancelled,
    ComparisonLimit,
    CandidateLimit,
}
impl VisualSearchStatus {
    pub fn is_terminal(self) -> bool {
        self != Self::Running
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VisualSearchProgress {
    pub status: VisualSearchStatus,
    /// Entire pages searched at every requested orientation. A partial page is not included.
    pub pages_scanned: usize,
    pub total_pages: usize,
    pub comparisons: u64,
    pub candidates: usize,
}
impl VisualSearchProgress {
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisualSearchCandidate {
    /// Stable within this inspection, independent of incremental step size. Not a PDF annotation ID.
    pub id: u32,
    /// Zero-based physical page index.
    pub page: usize,
    #[serde(serialize_with = "serialize_rect")]
    pub rect: PointRect,
    #[serde(serialize_with = "serialize_rotation")]
    pub rotation: Rotation,
    /// One minus the largest normalized loss over the full template and fixed
    /// 2x2/4x4 spatial grids, clamped to 0..1. Local losses use RMS, with each
    /// cell's contrast denominator floored at the mean cell contrast to keep
    /// low-contrast background noise from dominating. This is not a probability.
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualSearchReport {
    pub version: u8,
    /// Exact captured serialization, not a canonical file identity. PDFium may vary trailer IDs.
    pub source_sha256: String,
    pub template_page: usize,
    #[serde(serialize_with = "serialize_rect")]
    pub template_rect: PointRect,
    pub options: VisualSearchOptions,
    pub progress: VisualSearchProgress,
    pub candidates: Vec<VisualSearchCandidate>,
}

#[derive(Debug, Clone)]
struct BoundPage {
    index: usize,
    geometry: PageGeometry,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct TemplatePixel {
    x: u32,
    y: u32,
    value: u8,
    coarse_cell: u8,
    fine_cell: u8,
}

#[derive(Debug)]
struct Template {
    width: u32,
    height: u32,
    rotation: Rotation,
    /// Highest-contrast pixels first. Rejecting once the permitted total error is
    /// exceeded is exact; no approximate sampling skips a possible accepted match.
    pixels: Vec<TemplatePixel>,
    contrast: u64,
    allowed_error: u64,
    coarse_denominators: [f64; 4],
    fine_denominators: [f64; 16],
}

impl Template {
    fn similarity(&self, error: u64, coarse_error: &[u64; 4], fine_error: &[u64; 16]) -> f64 {
        fn local_loss<const N: usize>(errors: &[u64; N], denominators: &[f64; N]) -> f64 {
            (errors
                .iter()
                .zip(denominators)
                .map(|(&error, &denominator)| {
                    let loss = error as f64 / denominator;
                    loss * loss
                })
                .sum::<f64>()
                / N as f64)
                .sqrt()
        }
        let loss = (error as f64 / self.contrast as f64)
            .max(local_loss(coarse_error, &self.coarse_denominators))
            .max(local_loss(fine_error, &self.fine_denominators));
        (1.0 - loss).clamp(0.0, 1.0)
    }
}

#[derive(Debug)]
struct RasterCursor {
    gray: Vec<u8>,
    width: u32,
    height: u32,
    template: usize,
    x: u32,
    y: u32,
    pixel: usize,
    error: u64,
    coarse_error: [u64; 4],
    fine_error: [u64; 16],
}

/// Owns the captured source, geometries and bounded raster state; never contains a
/// PDFium handle. `step` is pure and caps comparisons even for an oversized budget.
/// Rendering one page is a separate operation on the caller's PDFium worker thread.
#[derive(Debug)]
pub struct VisualSearchJob {
    source: Arc<Vec<u8>>,
    source_sha256: String,
    template_page: BoundPage,
    template_rect: PointRect,
    preview: RenderedPage,
    options: VisualSearchOptions,
    pages: Vec<BoundPage>,
    templates: Vec<Template>,
    raster: Option<RasterCursor>,
    page_cursor: usize,
    status: VisualSearchStatus,
    comparisons: u64,
    candidates: Vec<VisualSearchCandidate>,
}

impl VisualSearchJob {
    pub fn pending_page(&self) -> Option<usize> {
        (self.status == VisualSearchStatus::Running && self.raster.is_none())
            .then(|| self.pages.get(self.page_cursor).map(|p| p.index))
            .flatten()
    }
    pub fn template_preview(&self) -> &RenderedPage {
        &self.preview
    }
    pub fn progress(&self) -> VisualSearchProgress {
        VisualSearchProgress {
            status: self.status,
            pages_scanned: self.page_cursor,
            total_pages: self.pages.len(),
            comparisons: self.comparisons,
            candidates: self.candidates.len(),
        }
    }
    pub fn report(&self) -> VisualSearchReport {
        VisualSearchReport {
            version: 1,
            source_sha256: self.source_sha256.clone(),
            template_page: self.template_page.index,
            template_rect: self.template_rect,
            options: self.options.clone(),
            progress: self.progress(),
            candidates: self.candidates.clone(),
        }
    }
    pub fn cancel(&mut self) {
        if self.status == VisualSearchStatus::Running {
            self.status = VisualSearchStatus::Cancelled;
            self.raster = None;
        }
    }
    fn supply_raster(&mut self, page: RenderedPage) -> Result<(), EngineError> {
        let expected = self
            .pages
            .get(self.page_cursor)
            .ok_or_else(|| failure("no page is pending"))?;
        if self.pending_page() != Some(page.page_index)
            || page.width != expected.width
            || page.height != expected.height
        {
            return Err(failure("raster does not match the pending source page"));
        }
        self.raster = Some(RasterCursor {
            gray: grayscale(&page)?,
            width: page.width,
            height: page.height,
            template: 0,
            x: 0,
            y: 0,
            pixel: 0,
            error: 0,
            coarse_error: [0; 4],
            fine_error: [0; 16],
        });
        Ok(())
    }

    /// Performs at most min(budget, MAX_STEP_COMPARISONS) pixel comparisons.
    /// A Running result with pending_page() requires one separately bounded render.
    /// No render, snapshot, write, sleep or native call occurs in this method.
    pub fn step(&mut self, budget: usize) -> VisualSearchProgress {
        if self.status != VisualSearchStatus::Running || budget == 0 {
            return self.progress();
        }
        let mut remaining = budget.min(MAX_STEP_COMPARISONS);
        while remaining > 0 && self.status == VisualSearchStatus::Running {
            let Some(cursor) = self.raster.as_mut() else {
                break;
            };
            if cursor.template == self.templates.len() {
                self.raster = None;
                self.page_cursor += 1;
                if self.page_cursor == self.pages.len() {
                    self.status = VisualSearchStatus::Complete;
                }
                break;
            }
            let template = &self.templates[cursor.template];
            if template.width > cursor.width || template.height > cursor.height {
                cursor.template += 1;
                continue;
            }
            if self.comparisons >= self.options.max_comparisons {
                self.status = VisualSearchStatus::ComparisonLimit;
                self.raster = None;
                break;
            }
            let sample = &template.pixels[cursor.pixel];
            let value =
                cursor.gray[((cursor.y + sample.y) * cursor.width + cursor.x + sample.x) as usize];
            let error = u64::from(value.abs_diff(sample.value));
            cursor.error += error;
            cursor.coarse_error[sample.coarse_cell as usize] += error;
            cursor.fine_error[sample.fine_cell as usize] += error;
            cursor.pixel += 1;
            self.comparisons += 1;
            remaining -= 1;
            if cursor.error > template.allowed_error || cursor.pixel == template.pixels.len() {
                // The global filter is a conservative lower bound on the final
                // loss. Spatial checks reuse the same compared pixels, so their
                // fixed-size accounting adds no hidden raster comparisons.
                let score = if cursor.error <= template.allowed_error {
                    template.similarity(cursor.error, &cursor.coarse_error, &cursor.fine_error)
                } else {
                    0.0
                };
                let matched = cursor.error <= template.allowed_error
                    && score >= f64::from(self.options.sensitivity);
                let found = matched.then(|| {
                    let page = &self.pages[self.page_cursor];
                    let (width, height) = page.geometry.display_size();
                    VisualSearchCandidate {
                        id: 0,
                        page: page.index,
                        rect: PointRect {
                            left: cursor.x as f32 * width / cursor.width as f32,
                            bottom: (cursor.height - cursor.y - template.height) as f32 * height
                                / cursor.height as f32,
                            right: (cursor.x + template.width) as f32 * width / cursor.width as f32,
                            top: (cursor.height - cursor.y) as f32 * height / cursor.height as f32,
                        },
                        rotation: template.rotation,
                        score: score as f32,
                    }
                });
                cursor.pixel = 0;
                cursor.error = 0;
                cursor.coarse_error = [0; 4];
                cursor.fine_error = [0; 16];
                cursor.x += 1;
                if cursor.x > cursor.width - template.width {
                    cursor.x = 0;
                    cursor.y += 1;
                    if cursor.y > cursor.height - template.height {
                        cursor.y = 0;
                        cursor.template += 1;
                    }
                }
                if let Some(candidate) = found {
                    self.add_candidate(candidate);
                }
            }
        }
        // Finishing the last pixel exactly at the budget is still complete; conversely
        // a limit reached mid-page is never presented as an exhaustive search.
        if self.status == VisualSearchStatus::Running
            && self
                .raster
                .as_ref()
                .is_some_and(|r| r.template == self.templates.len())
        {
            self.raster = None;
            self.page_cursor += 1;
            if self.page_cursor == self.pages.len() {
                self.status = VisualSearchStatus::Complete;
            }
        }
        if self.status == VisualSearchStatus::Running
            && self.comparisons >= self.options.max_comparisons
        {
            self.status = VisualSearchStatus::ComparisonLimit;
            self.raster = None;
        }
        self.progress()
    }

    fn add_candidate(&mut self, mut candidate: VisualSearchCandidate) {
        if let Some(previous) = self
            .candidates
            .iter_mut()
            .find(|c| c.page == candidate.page && overlap(c.rect, candidate.rect) >= 0.5)
        {
            if candidate.score > previous.score {
                candidate.id = previous.id;
                *previous = candidate;
            }
            return;
        }
        if self.candidates.len() == self.options.max_candidates {
            self.status = VisualSearchStatus::CandidateLimit;
            self.raster = None;
            return;
        }
        candidate.id = self.candidates.len() as u32 + 1;
        self.candidates.push(candidate);
    }
}

fn overlap(a: PointRect, b: PointRect) -> f32 {
    let area = |r: PointRect| (r.right - r.left) * (r.top - r.bottom);
    let intersection = (a.right.min(b.right) - a.left.max(b.left)).max(0.0)
        * (a.top.min(b.top) - a.bottom.max(b.bottom)).max(0.0);
    intersection / (area(a) + area(b) - intersection)
}

fn grayscale(page: &RenderedPage) -> Result<Vec<u8>, EngineError> {
    let pixels = u64::from(page.width) * u64::from(page.height);
    if page.width == 0
        || page.height == 0
        || page.width > MAX_PAGE_SIDE
        || page.height > MAX_PAGE_SIDE
        || pixels > MAX_PAGE_PIXELS
        || page.rgba.len() as u64 != pixels * 4
    {
        return Err(failure(
            "raster dimensions or buffer exceed visual-search limits",
        ));
    }
    Ok(page
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let gray =
                (77 * u32::from(p[0]) + 150 * u32::from(p[1]) + 29 * u32::from(p[2]) + 128) / 256;
            ((gray * u32::from(p[3]) + 255 * (255 - u32::from(p[3])) + 127) / 255) as u8
        })
        .collect())
}

fn templates(
    preview: &RenderedPage,
    options: &VisualSearchOptions,
) -> Result<Vec<Template>, EngineError> {
    if !(MIN_TEMPLATE_SIDE..=MAX_TEMPLATE_SIDE).contains(&preview.width)
        || !(MIN_TEMPLATE_SIDE..=MAX_TEMPLATE_SIDE).contains(&preview.height)
    {
        return Err(failure(
            "template must be 8..128 pixels on each side at the chosen scale",
        ));
    }
    let gray = grayscale(preview)?;
    let mut histogram = [0usize; 256];
    for &value in &gray {
        histogram[value as usize] += 1;
    }
    let mut cumulative = 0;
    let background = histogram
        .iter()
        .position(|&n| {
            cumulative += n;
            cumulative > gray.len() / 2
        })
        .unwrap_or(255) as u8;
    let contrast: u64 = gray.iter().map(|v| u64::from(v.abs_diff(background))).sum();
    let informative = gray.iter().filter(|v| v.abs_diff(background) >= 32).count();
    if contrast < gray.len() as u64 * 8
        || informative < 4
        || gray
            .iter()
            .max()
            .unwrap_or(&0)
            .saturating_sub(*gray.iter().min().unwrap_or(&0))
            < 32
    {
        return Err(failure("template has too little visual information; select a contrasting symbol with a small margin"));
    }
    let allowed_error = ((1.0 - f64::from(options.sensitivity)) * contrast as f64).floor() as u64;
    Ok(options
        .rotations
        .iter()
        .map(|&rotation| {
            let swapped = matches!(rotation, Rotation::R90 | Rotation::R270);
            let (width, height) = if swapped {
                (preview.height, preview.width)
            } else {
                (preview.width, preview.height)
            };
            let mut pixels = Vec::with_capacity(gray.len());
            let mut coarse_contrast = [0u64; 4];
            let mut fine_contrast = [0u64; 16];
            for y in 0..preview.height {
                for x in 0..preview.width {
                    let (dx, dy) = match rotation {
                        Rotation::R0 => (x, y),
                        Rotation::R90 => (preview.height - 1 - y, x),
                        Rotation::R180 => (preview.width - 1 - x, preview.height - 1 - y),
                        Rotation::R270 => (y, preview.width - 1 - x),
                    };
                    let value = gray[(y * preview.width + x) as usize];
                    // Assign cells after rotating the template, including swapped
                    // width/height. No corpus-specific coordinates or shape labels.
                    let coarse_cell = (dy * 2 / height * 2 + dx * 2 / width) as u8;
                    let fine_cell = (dy * 4 / height * 4 + dx * 4 / width) as u8;
                    coarse_contrast[coarse_cell as usize] += u64::from(value.abs_diff(background));
                    fine_contrast[fine_cell as usize] += u64::from(value.abs_diff(background));
                    pixels.push(TemplatePixel {
                        x: dx,
                        y: dy,
                        value,
                        coarse_cell,
                        fine_cell,
                    });
                }
            }
            pixels.sort_by_key(|p| std::cmp::Reverse(p.value.abs_diff(background)));
            Template {
                width,
                height,
                rotation,
                pixels,
                contrast,
                allowed_error,
                coarse_denominators: coarse_contrast
                    .map(|mass| (mass as f64).max(contrast as f64 / 4.0)),
                fine_denominators: fine_contrast
                    .map(|mass| (mass as f64).max(contrast as f64 / 16.0)),
            }
        })
        .collect())
}

fn bound_page(doc: &Document<'_>, index: usize, scale: f32) -> Result<BoundPage, EngineError> {
    let geometry = doc.page_geometry(index)?;
    let size = doc.page_size(index)?;
    let (width, height) =
        crate::tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
    if width > MAX_PAGE_SIDE
        || height > MAX_PAGE_SIDE
        || u64::from(width) * u64::from(height) > MAX_PAGE_PIXELS
    {
        return Err(failure(
            "page exceeds 4096 pixels per side or 8 Mi pixels; choose a lower fixed scale",
        ));
    }
    let (gw, gh) = geometry.display_size();
    if !gw.is_finite()
        || !gh.is_finite()
        || gw <= 0.0
        || gh <= 0.0
        || (gw - size.width_pt).abs() > 0.01
        || (gh - size.height_pt).abs() > 0.01
    {
        return Err(failure(
            "page geometry cannot be mapped consistently to its raster",
        ));
    }
    Ok(BoundPage {
        index,
        geometry,
        width,
        height,
    })
}

fn template_region(page: &BoundPage, rect: PointRect) -> Result<PixelRect, EngineError> {
    crate::annot_rich::checked_rect(&page.geometry, rect)?;
    let (width, height) = page.geometry.display_size();
    let x = (rect.left * page.width as f32 / width).floor() as u32;
    let y = ((height - rect.top) * page.height as f32 / height).floor() as u32;
    let right = ((rect.right * page.width as f32 / width).ceil() as u32).min(page.width);
    let bottom =
        (((height - rect.bottom) * page.height as f32 / height).ceil() as u32).min(page.height);
    let region = PixelRect::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y));
    if !(MIN_TEMPLATE_SIDE..=MAX_TEMPLATE_SIDE).contains(&region.width)
        || !(MIN_TEMPLATE_SIDE..=MAX_TEMPLATE_SIDE).contains(&region.height)
    {
        return Err(failure(
            "template must be 8..128 pixels on each side at the chosen scale",
        ));
    }
    Ok(region)
}

/// A source-bound all-or-nothing replacement snapshot. It is only constructible
/// from an explicit reviewed selection, and its bytes cannot be altered through this API.
#[derive(Debug)]
pub struct PreparedVisualSearchCounts {
    original: Arc<Vec<u8>>,
    candidate: Vec<u8>,
    pages: Vec<BoundPage>,
    count: usize,
}
impl PreparedVisualSearchCounts {
    pub fn snapshot(&self) -> &[u8] {
        &self.candidate
    }
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Document<'_> {
    /// Page content only: existing markups and form widgets are excluded.
    /// These bounds also apply to previews; no silently reduced scale is used.
    pub fn render_visual_search_content(
        &self,
        page: usize,
        scale: f32,
    ) -> Result<RenderedPage, EngineError> {
        let bound = bound_page(self, page, scale)?;
        self.render_visual_search_content_region(
            page,
            scale,
            PixelRect::new(0, 0, bound.width, bound.height),
        )
    }
    pub fn render_visual_search_content_region(
        &self,
        page: usize,
        scale: f32,
        region: PixelRect,
    ) -> Result<RenderedPage, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        // Cropping content-only rasters first renders the whole page, so a tiny
        // requested region must not bypass the full-page allocation/geometry bound.
        bound_page(self, page, scale)?;
        if !scale.is_finite()
            || scale <= 0.0
            || scale > 2.0
            || region.width > MAX_PAGE_SIDE
            || region.height > MAX_PAGE_SIDE
            || u64::from(region.width) * u64::from(region.height) > MAX_PAGE_PIXELS
        {
            return Err(failure(
                "content preview exceeds visual-search raster limits",
            ));
        }
        self.render_region_impl(page, scale, region, false)
    }

    pub fn prepare_visual_search(
        &self,
        template_page: usize,
        rect: PointRect,
        options: VisualSearchOptions,
    ) -> Result<VisualSearchJob, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        options.validate()?;
        let template_page = bound_page(self, template_page, options.scale)?;
        let pages = options
            .pages
            .iter()
            .map(|&page| bound_page(self, page, options.scale))
            .collect::<Result<Vec<_>, _>>()?;
        let region = template_region(&template_page, rect)?;
        let source = self.snapshot()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(failure("source exceeds 128 MiB"));
        }
        let source_sha256 = crate::local_review::digest(&source);
        let preview =
            self.render_visual_search_content_region(template_page.index, options.scale, region)?;
        let templates = templates(&preview, &options)?;
        Ok(VisualSearchJob {
            source: Arc::new(source),
            source_sha256,
            template_page,
            template_rect: rect,
            preview,
            options,
            pages,
            templates,
            raster: None,
            page_cursor: 0,
            status: VisualSearchStatus::Running,
            comparisons: 0,
            candidates: Vec::new(),
        })
    }

    pub fn verify_visual_search(&self, job: &VisualSearchJob) -> Result<(), EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        self.verify_visual_source(
            &job.source,
            job.pages.iter().chain(std::iter::once(&job.template_page)),
        )
    }
    fn verify_visual_source<'a>(
        &self,
        source: &[u8],
        pages: impl Iterator<Item = &'a BoundPage>,
    ) -> Result<(), EngineError> {
        let current = self.snapshot()?;
        if current.len() > MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(source, &current, self.password())?
        {
            return Err(failure("document changed; inspect the symbol again"));
        }
        for page in pages {
            if self.page_geometry(page.index)? != page.geometry {
                return Err(failure("page geometry changed; inspect the symbol again"));
            }
        }
        Ok(())
    }
    pub fn load_visual_search_page(&self, job: &mut VisualSearchJob) -> Result<(), EngineError> {
        let page = job
            .pending_page()
            .ok_or_else(|| failure("no raster page is pending"))?;
        self.verify_visual_search(job)?;
        let raster = self.render_visual_search_content(page, job.options.scale)?;
        job.supply_raster(raster)
    }

    fn visual_count_permission(&self) -> Result<(), EngineError> {
        self.require_annotation_permission()?;
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(failure("signed PDFs cannot receive visual-search counts"));
        }
        Ok(())
    }
    pub fn prepare_visual_search_counts(
        &self,
        job: &VisualSearchJob,
        selected_ids: &[u32],
        group: &str,
        marker_size: f32,
        author: Option<&str>,
    ) -> Result<PreparedVisualSearchCounts, EngineError> {
        self.visual_count_permission()?;
        if !job.status.is_terminal() {
            return Err(failure("finish or stop the search before reviewing counts"));
        }
        if selected_ids.is_empty()
            || selected_ids.len() > MAX_CANDIDATES
            || selected_ids.iter().copied().collect::<HashSet<_>>().len() != selected_ids.len()
        {
            return Err(failure("select at least one distinct candidate ID"));
        }
        crate::annotation_register::validate_count_group(group)?;
        if !marker_size.is_finite()
            || !(8.0..=72.0).contains(&marker_size)
            || author.is_some_and(|a| a.len() > 512 || a.chars().any(char::is_control))
        {
            return Err(failure(
                "marker size must be 8..72 points and author at most 512 plain UTF-8 bytes",
            ));
        }
        self.verify_visual_search(job)?;
        let selected = selected_ids
            .iter()
            .map(|id| {
                job.candidates
                    .iter()
                    .find(|c| c.id == *id)
                    .ok_or_else(|| failure("selected candidate does not belong to this inspection"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut pdf = crate::security::load(&job.source, self.password())?;
        let page_ids = pdf.get_pages();
        for candidate in &selected {
            let page = job
                .pages
                .iter()
                .find(|p| p.index == candidate.page)
                .ok_or_else(|| failure("candidate page is not in the captured search"))?;
            let page_id = *page_ids
                .get(&(candidate.page as u32 + 1))
                .ok_or(EngineError::PageOutOfRange(candidate.page))?;
            let point = (
                (candidate.rect.left + candidate.rect.right) / 2.0,
                (candidate.rect.bottom + candidate.rect.top) / 2.0,
            );
            crate::annotation_register::push_count_marker(
                &mut pdf,
                page_id,
                &page.geometry,
                point,
                marker_size,
                group,
                author,
            )?;
        }
        let candidate = crate::security::save(pdf)?;
        if candidate.len() > MAX_CANDIDATE_BYTES {
            return Err(failure("count candidate exceeds 160 MiB"));
        }
        // Validate the whole replacement before exposing it or mutating the original.
        let mut scratch = self.visual_search_scratch(candidate.clone())?;
        scratch.refresh_page_sizes()?;
        Ok(PreparedVisualSearchCounts {
            original: job.source.clone(),
            candidate,
            pages: job.pages.clone(),
            count: selected.len(),
        })
    }
    pub fn apply_visual_search_counts(
        &mut self,
        prepared: &PreparedVisualSearchCounts,
    ) -> Result<usize, EngineError> {
        self.visual_count_permission()?;
        self.verify_visual_source(&prepared.original, prepared.pages.iter())?;
        self.restore(prepared.candidate.clone())?;
        Ok(prepared.count)
    }
    fn visual_search_scratch(&self, bytes: Vec<u8>) -> Result<Document<'_>, EngineError> {
        Ok(Document {
            inner: self.pdfium.load_pdf_from_byte_vec(bytes, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        })
    }
    pub fn render_visual_search_counts_preview(
        &self,
        prepared: &PreparedVisualSearchCounts,
        page: usize,
        long_side: u32,
    ) -> Result<RenderedPage, EngineError> {
        if !(64..=1536).contains(&long_side) {
            return Err(failure("count preview side must be 64..1536 pixels"));
        }
        self.visual_count_permission()?;
        self.verify_visual_source(&prepared.original, prepared.pages.iter())?;
        let scratch = self.visual_search_scratch(prepared.candidate.clone())?;
        let size = scratch.page_size(page)?;
        scratch.render_page(page, long_side as f32 / size.width_pt.max(size.height_pt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, value: u8) -> RenderedPage {
        RenderedPage {
            page_index: 0,
            width,
            height,
            rgba: [value, value, value, 255].repeat((width * height) as usize),
        }
    }
    fn pixel(image: &mut RenderedPage, x: u32, y: u32, value: u8) {
        let at = ((y * image.width + x) * 4) as usize;
        image.rgba[at..at + 3].fill(value);
    }
    fn symbol() -> RenderedPage {
        let mut template = image(8, 12, 255);
        for y in 2..10 {
            for x in 1..3 {
                pixel(&mut template, x, y, 0);
            }
        }
        for y in 8..10 {
            for x in 1..7 {
                pixel(&mut template, x, y, 0);
            }
        }
        pixel(&mut template, 6, 2, 40);
        template
    }
    fn bound(width: u32, height: u32) -> BoundPage {
        BoundPage {
            index: 0,
            geometry: PageGeometry {
                bbox: PointRect {
                    left: 0.,
                    bottom: 0.,
                    right: width as f32,
                    top: height as f32,
                },
                rotation: Rotation::R0,
            },
            width,
            height,
        }
    }
    fn job(options: VisualSearchOptions, width: u32, height: u32) -> VisualSearchJob {
        job_for(symbol(), options, width, height)
    }
    fn job_for(
        preview: RenderedPage,
        options: VisualSearchOptions,
        width: u32,
        height: u32,
    ) -> VisualSearchJob {
        VisualSearchJob {
            source: Arc::new(vec![]),
            source_sha256: "test".into(),
            template_page: bound(width, height),
            template_rect: PointRect {
                left: 0.,
                bottom: 0.,
                right: preview.width as f32,
                top: preview.height as f32,
            },
            templates: templates(&preview, &options).unwrap(),
            preview,
            pages: vec![bound(width, height)],
            options,
            raster: None,
            page_cursor: 0,
            status: VisualSearchStatus::Running,
            comparisons: 0,
            candidates: vec![],
        }
    }
    fn paste(
        page: &mut RenderedPage,
        template: &RenderedPage,
        left: u32,
        top: u32,
        rotation: Rotation,
    ) {
        for y in 0..template.height {
            for x in 0..template.width {
                let (dx, dy) = match rotation {
                    Rotation::R0 => (x, y),
                    Rotation::R90 => (template.height - 1 - y, x),
                    Rotation::R180 => (template.width - 1 - x, template.height - 1 - y),
                    Rotation::R270 => (y, template.width - 1 - x),
                };
                pixel(
                    page,
                    left + dx,
                    top + dy,
                    template.rgba[((y * template.width + x) * 4) as usize],
                );
            }
        }
    }
    fn finish(job: &mut VisualSearchJob, budget: usize) {
        for _ in 0..2_000_000 {
            if job.progress().is_terminal() {
                return;
            }
            let before = job.comparisons;
            job.step(budget);
            assert!(job.comparisons - before <= budget.min(MAX_STEP_COMPARISONS) as u64);
        }
        panic!("incremental job did not terminate");
    }

    #[test]
    fn raster_matching_is_same_scale_clockwise_and_independent_of_step_size() {
        let rotations = vec![Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270];
        let options = VisualSearchOptions {
            sensitivity: 1.0,
            rotations: rotations.clone(),
            max_candidates: 4,
            ..Default::default()
        };
        let mut page = image(64, 56, 255);
        let positions = [(4, 4), (34, 4), (4, 34), (34, 34)];
        for (&(x, y), &rotation) in positions.iter().zip(&rotations) {
            paste(&mut page, &symbol(), x, y, rotation);
        }
        // A filled dark rectangle is not accepted just because most of the page is white.
        for y in 23..28 {
            for x in 22..29 {
                pixel(&mut page, x, y, 0);
            }
        }
        let mut slow = job(options.clone(), 64, 56);
        let mut fast = job(options, 64, 56);
        assert_eq!(slow.pending_page(), Some(0));
        slow.supply_raster(page.clone()).unwrap();
        fast.supply_raster(page).unwrap();
        finish(&mut slow, 1);
        finish(&mut fast, usize::MAX);
        assert_eq!(slow.progress(), fast.progress());
        assert_eq!(slow.candidates, fast.candidates);
        assert_eq!(slow.status, VisualSearchStatus::Complete);
        assert_eq!(slow.candidates.len(), 4);
        for (&(x, y), &rotation) in positions.iter().zip(&rotations) {
            let candidate = slow
                .candidates
                .iter()
                .find(|c| c.rotation == rotation)
                .unwrap();
            assert_eq!(candidate.rect.left, x as f32);
            assert_eq!(candidate.rect.top, (56 - y) as f32);
            assert_eq!(candidate.score, 1.0);
        }
        assert_eq!(slow.pending_page(), None);
        let terminal = slow.progress();
        slow.cancel();
        assert_eq!(slow.step(10), terminal);
    }

    #[test]
    fn incomplete_searches_report_limits_and_cancellation_without_automatic_counts() {
        let mut page = image(48, 20, 255);
        paste(&mut page, &symbol(), 2, 4, Rotation::R0);
        paste(&mut page, &symbol(), 28, 4, Rotation::R0);
        let mut limited = job(
            VisualSearchOptions {
                max_comparisons: 1,
                ..Default::default()
            },
            48,
            20,
        );
        limited.supply_raster(page.clone()).unwrap();
        assert_eq!(limited.step(0).comparisons, 0);
        let progress = limited.step(usize::MAX);
        assert_eq!(progress.status, VisualSearchStatus::ComparisonLimit);
        assert_eq!(progress.comparisons, 1);
        assert_eq!(progress.pages_scanned, 0);
        assert!(limited.pending_page().is_none());
        let mut cap = job(
            VisualSearchOptions {
                sensitivity: 1.,
                max_candidates: 1,
                ..Default::default()
            },
            48,
            20,
        );
        cap.supply_raster(page.clone()).unwrap();
        finish(&mut cap, 1000);
        assert_eq!(cap.status, VisualSearchStatus::CandidateLimit);
        assert_eq!(cap.candidates.len(), 1);
        assert_eq!(cap.page_cursor, 0);
        let mut cancelled = job(VisualSearchOptions::default(), 48, 20);
        cancelled.supply_raster(page).unwrap();
        cancelled.step(10);
        cancelled.cancel();
        let stopped = cancelled.progress();
        assert_eq!(stopped.status, VisualSearchStatus::Cancelled);
        assert_eq!(cancelled.step(1000), stopped);
        assert_eq!(cancelled.pending_page(), None);
    }

    #[test]
    fn strict_options_and_low_information_templates_are_rejected_before_scanning() {
        let options = VisualSearchOptions::default();
        let mut value = serde_json::to_value(&options).unwrap();
        value["rotations"] = serde_json::json!([0, 90, 180, 270]);
        assert_eq!(
            serde_json::from_value::<VisualSearchOptions>(value.clone())
                .unwrap()
                .rotations
                .len(),
            4
        );
        value["rotations"] = serde_json::json!([45]);
        assert!(serde_json::from_value::<VisualSearchOptions>(value).is_err());
        let mut value = serde_json::to_value(&options).unwrap();
        value["ignored_instruction"] = true.into();
        assert!(serde_json::from_value::<VisualSearchOptions>(value).is_err());
        for value in [0, 128, 255] {
            assert!(templates(&image(8, 12, value), &options).is_err());
        }
        let mut transparent = symbol();
        for p in transparent.rgba.as_chunks_mut::<4>().0.iter_mut() {
            p[3] = 0;
        }
        assert!(templates(&transparent, &options).is_err());
        let mut invalid = symbol();
        invalid.rgba.pop();
        assert!(templates(&invalid, &options).is_err());
        for invalid in [
            VisualSearchOptions {
                pages: vec![],
                ..options.clone()
            },
            VisualSearchOptions {
                pages: vec![0, 0],
                ..options.clone()
            },
            VisualSearchOptions {
                pages: (0..17).collect(),
                ..options.clone()
            },
            VisualSearchOptions {
                scale: f32::NAN,
                ..options.clone()
            },
            VisualSearchOptions {
                sensitivity: f32::INFINITY,
                ..options.clone()
            },
            VisualSearchOptions {
                rotations: vec![],
                ..options.clone()
            },
            VisualSearchOptions {
                rotations: vec![Rotation::R0, Rotation::R0],
                ..options.clone()
            },
            VisualSearchOptions {
                max_candidates: 513,
                ..options.clone()
            },
            VisualSearchOptions {
                max_comparisons: MAX_COMPARISONS + 1,
                ..options.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    fn framed_fork() -> RenderedPage {
        let mut result = image(32, 24, 255);
        for y in 2..22 {
            for x in 2..30 {
                let frame = !(4..28).contains(&x) || !(4..20).contains(&y);
                let stem = (14..18).contains(&x) && (7..17).contains(&y);
                let arms = (11..24).contains(&x) && (13..15).contains(&y);
                if frame || stem || arms {
                    pixel(&mut result, x, y, 0);
                }
            }
        }
        result
    }
    fn compared_scores(template: &RenderedPage, candidate: &RenderedPage) -> (f64, f64) {
        let descriptors = templates(template, &VisualSearchOptions::default()).unwrap();
        let descriptor = &descriptors[0];
        let gray = grayscale(candidate).unwrap();
        let mut error = 0;
        let mut coarse = [0; 4];
        let mut fine = [0; 16];
        for sample in &descriptor.pixels {
            let delta = u64::from(
                gray[(sample.y * candidate.width + sample.x) as usize].abs_diff(sample.value),
            );
            error += delta;
            coarse[sample.coarse_cell as usize] += delta;
            fine[sample.fine_cell as usize] += delta;
        }
        (
            1.0 - error as f64 / descriptor.contrast as f64,
            descriptor.similarity(error, &coarse, &fine),
        )
    }

    #[test]
    fn spatial_similarity_distinguishes_local_structure_from_shared_outline_and_mild_noise() {
        // Original non-square fixture: a common frame and a fork with unequal
        // arms. The same global error can represent a local structural difference
        // or a small intensity change spread across the shape.
        let template = framed_fork();
        let mut mirror = image(32, 24, 255);
        let mut missing_arm = template.clone();
        for y in 0..24 {
            for x in 0..32 {
                pixel(
                    &mut mirror,
                    31 - x,
                    y,
                    template.rgba[((y * 32 + x) * 4) as usize],
                );
                if (18..24).contains(&x) && (13..15).contains(&y) {
                    pixel(&mut missing_arm, x, y, 255);
                }
            }
        }
        for different in [&mirror, &missing_arm] {
            let (global, spatial) = compared_scores(&template, different);
            assert!(
                global > 0.92,
                "shared outline dilutes the global error: {global}"
            );
            assert!(
                spatial < 0.92,
                "a local structural change must remain visible: {spatial}"
            );
            assert!(spatial <= global);
        }
        assert_eq!(compared_scores(&template, &template), (1.0, 1.0));
        let mut previous = 1.0;
        for intensity in [1, 3, 6] {
            let mut faded = template.clone();
            for p in faded.rgba.as_chunks_mut::<4>().0.iter_mut() {
                if p[0] == 0 {
                    p[..3].fill(intensity);
                }
            }
            let (global, spatial) = compared_scores(&template, &faded);
            assert!(
                spatial < previous && spatial > 0.92,
                "scores must vary continuously for mild intensity changes: {spatial}"
            );
            assert!(spatial <= global + f64::EPSILON);
            previous = spatial;
        }

        // Anti-aliased edge pixels and bounded intensity noise remain grayscale
        // observations, not thresholded binary masks or inferred vector objects.
        let mut antialiased = template.clone();
        for y in 1..23 {
            for x in 1..31 {
                let at = ((y * 32 + x) * 4) as usize;
                if template.rgba[at] == 0
                    && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                        .iter()
                        .any(|&(nx, ny)| template.rgba[((ny * 32 + nx) * 4) as usize] == 255)
                {
                    pixel(&mut antialiased, x, y, 80);
                }
            }
        }
        let mut noisy = antialiased.clone();
        for (index, p) in noisy.rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let noise = (index * 13 % 7) as i16 - 3;
            let value = (i16::from(p[0]) + noise).clamp(0, 255) as u8;
            p[..3].fill(value);
        }
        let score = compared_scores(&antialiased, &noisy).1;
        assert!(
            score > 0.92 && score < 1.0,
            "mild noisy antialiased copy: {score}"
        );
        let rotations = vec![Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270];
        let options = VisualSearchOptions {
            rotations: rotations.clone(),
            sensitivity: 0.92,
            ..Default::default()
        };
        let mut page = image(184, 88, 255);
        for (&rotation, x) in rotations.iter().zip([8, 52, 96, 140]) {
            paste(&mut page, &noisy, x, 8, rotation);
        }
        paste(&mut page, &mirror, 72, 52, Rotation::R0);
        let mut small_steps = job_for(antialiased.clone(), options.clone(), 184, 88);
        let mut large_steps = job_for(antialiased, options, 184, 88);
        small_steps.supply_raster(page.clone()).unwrap();
        large_steps.supply_raster(page).unwrap();
        finish(&mut small_steps, 37);
        finish(&mut large_steps, usize::MAX);
        assert_eq!(small_steps.progress(), large_steps.progress());
        assert_eq!(small_steps.candidates, large_steps.candidates);
        assert_eq!(small_steps.status, VisualSearchStatus::Complete);
        assert_eq!(small_steps.candidates.len(), 4);
        for rotation in rotations {
            assert!(small_steps
                .candidates
                .iter()
                .any(|c| c.rotation == rotation));
        }
        assert!(small_steps
            .candidates
            .iter()
            .all(|c| c.score >= 0.92 && c.score < 1.0));
    }
}
