//! Source-bound, reviewed raster boundary filling. No geometric precision is
//! inferred beyond the explicitly chosen raster resolution and luminance threshold.
//! Four-connected free cells form the region; exact cell-edge contours retain all
//! holes. Ambiguous contacts, leaks and excessive geometry are refused, never repaired.

use crate::measurement::{
    Measurement, MeasurementKind, MeasurementUnit, RectangleScale, MAX_HOLES, MAX_RING_VERTICES,
    MAX_TOTAL_VERTICES,
};
use crate::{Document, EngineError, PageGeometry, RenderedPage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::Arc;

pub const MAX_PIXELS: usize = 8 * 1024 * 1024;
pub const MAX_SIDE: u32 = 4096;
pub const MAX_CLOSURES: usize = 32;
pub const MAX_FRONTIER: usize = 262_144;
pub const MAX_EDGES: usize = 262_144;
pub const MAX_WORK: u64 = 64_000_000;
pub const MAX_STEP_WORK: usize = 16_384;
pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 160 * 1024 * 1024;
const MAX_PROVENANCE_BYTES: usize = 128 * 1024;
const KEY: &[u8] = b"SuniPDFBoundaryFill";

fn failure(message: &str) -> EngineError {
    EngineError::Other(format!("Boundary fill: {message}"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryClosure {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillOptions {
    pub scale: f32,
    /// Pixels at or below this grayscale value are barriers, after compositing on white.
    pub threshold: u8,
    /// Explicit displayed-page segments. One-pixel supercover includes both side
    /// cells at an exact grid-corner crossing; no hidden closing or dilation occurs.
    pub closures: Vec<BoundaryClosure>,
    pub max_pixels: usize,
    pub max_frontier: usize,
    pub max_edges: usize,
    pub max_work: u64,
}
impl Default for BoundaryFillOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            threshold: 160,
            closures: Vec::new(),
            max_pixels: MAX_PIXELS,
            max_frontier: MAX_FRONTIER,
            max_edges: MAX_EDGES,
            max_work: 32_000_000,
        }
    }
}
impl BoundaryFillOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if !self.scale.is_finite()
            || !(0.25..=2.0).contains(&self.scale)
            || self.closures.len() > MAX_CLOSURES
            || self.max_pixels == 0
            || self.max_pixels > MAX_PIXELS
            || self.max_frontier == 0
            || self.max_frontier > MAX_FRONTIER
            || self.max_edges == 0
            || self.max_edges > MAX_EDGES
            || self.max_work == 0
            || self.max_work > MAX_WORK
        {
            return Err(failure("invalid scale, closure count or resource limit"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryFillStatus {
    Running,
    Complete,
    Cancelled,
    Leaked,
    SeedBlocked,
    PixelLimit,
    FrontierLimit,
    EdgeLimit,
    WorkLimit,
    TopologyRejected,
    ComplexityLimit,
}
impl BoundaryFillStatus {
    pub fn is_terminal(self) -> bool {
        self != Self::Running
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryFillPhase {
    Threshold,
    Closures,
    Flood,
    Edges,
    Contours,
    Validation,
    Finished,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillProgress {
    pub status: BoundaryFillStatus,
    pub phase: BoundaryFillPhase,
    pub work: u64,
    pub filled_pixels: u64,
    pub frontier: usize,
    pub edges: usize,
    pub contours: usize,
}
impl BoundaryFillProgress {
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillGeometry {
    pub bounds: [f32; 4],
    pub rotation: u16,
    pub display_size: [f32; 2],
}
impl From<PageGeometry> for BoundaryFillGeometry {
    fn from(geo: PageGeometry) -> Self {
        let (w, h) = geo.display_size();
        Self {
            bounds: [geo.bbox.left, geo.bbox.bottom, geo.bbox.right, geo.bbox.top],
            rotation: geo.rotation.degrees(),
            display_size: [w, h],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillRegion {
    pub outer: Vec<[f32; 2]>,
    pub holes: Vec<Vec<[f32; 2]>>,
    pub pixel_count: u64,
    pub area_points2: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillReport {
    pub version: u8,
    pub source_sha256: String,
    pub page: usize,
    pub seed: [f32; 2],
    pub geometry: BoundaryFillGeometry,
    pub raster_width: u32,
    pub raster_height: u32,
    pub closure_width_pixels: u8,
    pub closure_rasterization: String,
    pub closure_pixels: u64,
    pub options: BoundaryFillOptions,
    pub progress: BoundaryFillProgress,
    pub region: Option<BoundaryFillRegion>,
}
/// Historical generation record. It does not certify a subsequently edited annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFillProvenance {
    pub version: u8,
    pub inspection: BoundaryFillReport,
    pub reference: String,
    pub measurement: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct BoundaryFillMeasurementOptions {
    pub scale: RectangleScale,
    pub unit: MeasurementUnit,
    pub precision: usize,
}

#[derive(Debug)]
struct ClosureCursor {
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    sx: i32,
    sy: i32,
    ix: i32,
    iy: i32,
    finished: bool,
}
impl ClosureCursor {
    fn new(a: (u32, u32), b: (u32, u32)) -> Self {
        Self {
            x: a.0 as i32,
            y: a.1 as i32,
            dx: a.0.abs_diff(b.0) as i32,
            dy: a.1.abs_diff(b.1) as i32,
            sx: if b.0 > a.0 { 1 } else { -1 },
            sy: if b.1 > a.1 { 1 } else { -1 },
            ix: 0,
            iy: 0,
            finished: false,
        }
    }
    fn next(&mut self) -> Vec<(u32, u32)> {
        let mut cells = vec![(self.x as u32, self.y as u32)];
        if self.ix == self.dx && self.iy == self.dy {
            self.finished = true;
            return cells;
        }
        let decision = (1 + 2 * self.ix) * self.dy - (1 + 2 * self.iy) * self.dx;
        if decision == 0 {
            cells.push(((self.x + self.sx) as u32, self.y as u32));
            cells.push((self.x as u32, (self.y + self.sy) as u32));
            self.x += self.sx;
            self.y += self.sy;
            self.ix += 1;
            self.iy += 1;
        } else if decision < 0 {
            self.x += self.sx;
            self.ix += 1;
        } else {
            self.y += self.sy;
            self.iy += 1;
        }
        cells
    }
}

/// No PDFium handle: thresholding, closure rasterization, flood, edge scanning and
/// contour following all resume in bounded steps. A work unit handles one pixel,
/// one closure step (at most three cells), one flood cell (four neighbors), one
/// edge-scan cell (four edges), or one contour edge. Final topology validation is
/// separately bounded by the existing 512-vertex measurement limit.
pub struct BoundaryFillJob {
    source: Arc<Vec<u8>>,
    source_sha256: String,
    page: usize,
    seed: [f32; 2],
    seed_pixel: (u32, u32),
    geometry: PageGeometry,
    width: u32,
    height: u32,
    options: BoundaryFillOptions,
    rgba: Option<Vec<u8>>,
    mask: Vec<u8>,
    cursor: usize,
    closures: Vec<ClosureCursor>,
    closure_index: usize,
    closure_pixels: u64,
    frontier: VecDeque<u32>,
    filled: u64,
    edges: BTreeMap<u32, u32>,
    incoming: HashSet<u32>,
    edge_count: usize,
    trace_start: Option<u32>,
    trace_next: u32,
    trace_points: Vec<(u32, u32)>,
    last_direction: Option<(i32, i32)>,
    first_direction: Option<(i32, i32)>,
    rings: Vec<Vec<(u32, u32)>>,
    status: BoundaryFillStatus,
    phase: BoundaryFillPhase,
    work: u64,
    region: Option<BoundaryFillRegion>,
}
impl std::fmt::Debug for BoundaryFillJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundaryFillJob")
            .field("page", &self.page)
            .field("progress", &self.progress())
            .finish_non_exhaustive()
    }
}
impl BoundaryFillJob {
    pub fn progress(&self) -> BoundaryFillProgress {
        BoundaryFillProgress {
            status: self.status,
            phase: self.phase,
            work: self.work,
            filled_pixels: self.filled,
            frontier: self.frontier.len(),
            edges: self.edge_count,
            contours: self.rings.len(),
        }
    }
    pub fn report(&self) -> BoundaryFillReport {
        BoundaryFillReport {
            version: 1,
            source_sha256: self.source_sha256.clone(),
            page: self.page,
            seed: self.seed,
            geometry: self.geometry.into(),
            raster_width: self.width,
            raster_height: self.height,
            closure_width_pixels: 1,
            closure_rasterization: "supercover-grid-cells".into(),
            closure_pixels: self.closure_pixels,
            options: self.options.clone(),
            progress: self.progress(),
            region: self.region.clone(),
        }
    }
    fn stop(&mut self, status: BoundaryFillStatus) {
        self.status = status;
        self.phase = BoundaryFillPhase::Finished;
        self.rgba = None;
        self.frontier.clear();
        self.edges.clear();
        self.incoming.clear();
        self.region = None;
    }
    pub fn cancel(&mut self) {
        if self.status == BoundaryFillStatus::Running {
            self.stop(BoundaryFillStatus::Cancelled);
        }
    }
    pub fn step(&mut self, budget: usize) -> BoundaryFillProgress {
        for _ in 0..budget.min(MAX_STEP_WORK) {
            if self.status != BoundaryFillStatus::Running {
                break;
            }
            if self.work == self.options.max_work {
                self.stop(BoundaryFillStatus::WorkLimit);
                break;
            }
            self.work += 1;
            match self.phase {
                BoundaryFillPhase::Threshold => self.threshold_step(),
                BoundaryFillPhase::Closures => self.closure_step(),
                BoundaryFillPhase::Flood => self.flood_step(),
                BoundaryFillPhase::Edges => self.edge_step(),
                BoundaryFillPhase::Contours => self.contour_step(),
                BoundaryFillPhase::Validation => self.validate_region(),
                BoundaryFillPhase::Finished => break,
            }
        }
        if self.status == BoundaryFillStatus::Running && self.work == self.options.max_work {
            self.stop(BoundaryFillStatus::WorkLimit);
        }
        self.progress()
    }
    fn threshold_step(&mut self) {
        if self.cursor == self.mask.len() {
            self.rgba = None;
            self.cursor = 0;
            self.phase = BoundaryFillPhase::Closures;
            return;
        }
        let at = self.cursor * 4;
        let rgba = self.rgba.as_ref().expect("threshold raster");
        let p = &rgba[at..at + 4];
        let gray =
            (77 * u32::from(p[0]) + 150 * u32::from(p[1]) + 29 * u32::from(p[2]) + 128) / 256;
        let gray = ((gray * u32::from(p[3]) + 255 * (255 - u32::from(p[3])) + 127) / 255) as u8;
        self.mask[self.cursor] = u8::from(gray <= self.options.threshold);
        self.cursor += 1;
    }
    fn closure_step(&mut self) {
        if self.closure_index == self.closures.len() {
            let seed = (self.seed_pixel.1 * self.width + self.seed_pixel.0) as usize;
            if self.mask[seed] != 0 {
                self.stop(BoundaryFillStatus::SeedBlocked);
                return;
            }
            self.mask[seed] = 2;
            self.frontier.push_back(seed as u32);
            self.filled = 1;
            self.phase = BoundaryFillPhase::Flood;
            return;
        }
        let closure = &mut self.closures[self.closure_index];
        for (x, y) in closure.next() {
            let slot = (y * self.width + x) as usize;
            if self.mask[slot] == 0 {
                self.mask[slot] = 1;
                self.closure_pixels += 1;
            }
        }
        if closure.finished {
            self.closure_index += 1;
        }
    }
    fn flood_step(&mut self) {
        let Some(at) = self.frontier.pop_front() else {
            self.cursor = 0;
            self.phase = BoundaryFillPhase::Edges;
            return;
        };
        let (x, y) = (at % self.width, at / self.width);
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            self.stop(BoundaryFillStatus::Leaked);
            return;
        }
        for next in [at - 1, at + 1, at - self.width, at + self.width] {
            if self.mask[next as usize] == 0 {
                if self.filled == self.options.max_pixels as u64 {
                    self.stop(BoundaryFillStatus::PixelLimit);
                    return;
                }
                if self.frontier.len() == self.options.max_frontier {
                    self.stop(BoundaryFillStatus::FrontierLimit);
                    return;
                }
                self.mask[next as usize] = 2;
                self.frontier.push_back(next);
                self.filled += 1;
            }
        }
    }
    fn edge_step(&mut self) {
        if self.cursor == self.mask.len() {
            self.phase = BoundaryFillPhase::Contours;
            return;
        }
        let at = self.cursor;
        self.cursor += 1;
        if self.mask[at] != 2 {
            return;
        }
        let (x, y) = (at as u32 % self.width, at as u32 / self.width);
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            self.stop(BoundaryFillStatus::Leaked);
            return;
        }
        let vertex = |x: u32, y: u32| y * (self.width + 1) + x;
        let candidates = [
            (at - self.width as usize, vertex(x, y), vertex(x + 1, y)),
            (at + 1, vertex(x + 1, y), vertex(x + 1, y + 1)),
            (
                at + self.width as usize,
                vertex(x + 1, y + 1),
                vertex(x, y + 1),
            ),
            (at - 1, vertex(x, y + 1), vertex(x, y)),
        ];
        for (neighbor, start, end) in candidates {
            if self.mask[neighbor] == 2 {
                continue;
            }
            if self.edge_count == self.options.max_edges {
                self.stop(BoundaryFillStatus::EdgeLimit);
                return;
            }
            if self.edges.contains_key(&start) || self.incoming.contains(&end) {
                self.stop(BoundaryFillStatus::TopologyRejected);
                return;
            }
            self.edges.insert(start, end);
            self.incoming.insert(end);
            self.edge_count += 1;
        }
    }
    fn contour_step(&mut self) {
        if self.trace_start.is_none() {
            let Some((&start, _)) = self.edges.first_key_value() else {
                self.phase = BoundaryFillPhase::Validation;
                return;
            };
            if self.rings.len() > MAX_HOLES {
                self.stop(BoundaryFillStatus::ComplexityLimit);
                return;
            }
            self.trace_start = Some(start);
            self.trace_next = start;
            self.trace_points.clear();
            self.last_direction = None;
            self.first_direction = None;
            self.trace_points
                .push((start % (self.width + 1), start / (self.width + 1)));
        }
        let current = self.trace_next;
        let Some(next) = self.edges.remove(&current) else {
            self.stop(BoundaryFillStatus::TopologyRejected);
            return;
        };
        let point = (current % (self.width + 1), current / (self.width + 1));
        let end = (next % (self.width + 1), next / (self.width + 1));
        let direction = (end.0 as i32 - point.0 as i32, end.1 as i32 - point.1 as i32);
        if self.last_direction.is_some_and(|last| last != direction) {
            self.trace_points.push(point);
        }
        if self.first_direction.is_none() {
            self.first_direction = Some(direction);
        }
        self.last_direction = Some(direction);
        self.trace_next = next;
        if self.trace_points.len() > MAX_RING_VERTICES + 1 {
            self.stop(BoundaryFillStatus::ComplexityLimit);
            return;
        }
        if Some(next) == self.trace_start {
            if self.first_direction == self.last_direction {
                self.trace_points.remove(0);
            }
            if self.trace_points.len() < 3 {
                self.stop(BoundaryFillStatus::TopologyRejected);
                return;
            }
            if self.trace_points.len() > MAX_RING_VERTICES
                || self.rings.iter().map(Vec::len).sum::<usize>() + self.trace_points.len()
                    > MAX_TOTAL_VERTICES
            {
                self.stop(BoundaryFillStatus::ComplexityLimit);
                return;
            }
            self.rings.push(std::mem::take(&mut self.trace_points));
            self.trace_start = None;
        }
    }
    fn validate_region(&mut self) {
        let mut outer = None;
        let mut holes = Vec::new();
        let (w, h) = self.geometry.display_size();
        for ring in &self.rings {
            let twice: i64 = ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .map(|(a, b)| i64::from(a.0) * i64::from(b.1) - i64::from(b.0) * i64::from(a.1))
                .sum();
            let points: Vec<[f32; 2]> = ring
                .iter()
                .map(|&(x, y)| {
                    [
                        x as f32 * w / self.width as f32,
                        (self.height - y) as f32 * h / self.height as f32,
                    ]
                })
                .collect();
            if twice > 0 {
                if outer.replace(points).is_some() {
                    self.stop(BoundaryFillStatus::TopologyRejected);
                    return;
                }
            } else if twice < 0 {
                holes.push(points);
            } else {
                self.stop(BoundaryFillStatus::TopologyRejected);
                return;
            }
        }
        let Some(outer) = outer else {
            self.stop(BoundaryFillStatus::TopologyRejected);
            return;
        };
        let measurement = raw_measurement(&outer, &holes);
        let Ok(area) = crate::measurement_area::breakdown(&measurement, 1.0) else {
            self.stop(BoundaryFillStatus::TopologyRejected);
            return;
        };
        let pixel_area = self.filled as f64 * f64::from(w) / f64::from(self.width) * f64::from(h)
            / f64::from(self.height);
        if (area.net - pixel_area).abs() > pixel_area.abs().max(1.0) * 1e-5 {
            self.stop(BoundaryFillStatus::TopologyRejected);
            return;
        }
        self.region = Some(BoundaryFillRegion {
            outer,
            holes,
            pixel_count: self.filled,
            area_points2: area.net,
        });
        self.status = BoundaryFillStatus::Complete;
        self.phase = BoundaryFillPhase::Finished;
        self.incoming.clear();
    }
}

fn raw_measurement(outer: &[[f32; 2]], holes: &[Vec<[f32; 2]>]) -> Measurement {
    Measurement {
        kind: MeasurementKind::Area,
        points: outer.iter().map(|p| (p[0], p[1])).collect(),
        holes: holes
            .iter()
            .map(|r| r.iter().map(|p| (p[0], p[1])).collect())
            .collect(),
        depth: None,
        calibration: [(0.0, 0.0), (1.0, 0.0)],
        known_distance: 1.0,
        unit: MeasurementUnit::Metres,
        precision: 2,
    }
}
fn pixel_point(
    point: [f32; 2],
    geo: PageGeometry,
    width: u32,
    height: u32,
    edge: bool,
) -> Result<(u32, u32), EngineError> {
    let (w, h) = geo.display_size();
    if !point.iter().all(|p| p.is_finite())
        || point[0] < 0.0
        || point[1] < 0.0
        || point[0] > w
        || point[1] > h
        || (!edge && (point[0] == w || point[1] == 0.0))
    {
        return Err(failure(
            "seed or closure endpoint is outside the displayed page",
        ));
    }
    Ok((
        ((f64::from(point[0]) * f64::from(width) / f64::from(w)).floor() as u32).min(width - 1),
        ((f64::from(h - point[1]) * f64::from(height) / f64::from(h)).floor() as u32)
            .min(height - 1),
    ))
}

pub struct PreparedBoundaryFillMeasurement {
    original: Arc<Vec<u8>>,
    candidate: Vec<u8>,
    page: usize,
    geometry: PageGeometry,
    index: usize,
    measurement: Measurement,
    report: BoundaryFillReport,
}
impl std::fmt::Debug for PreparedBoundaryFillMeasurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedBoundaryFillMeasurement")
            .field("page", &self.page)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}
impl PreparedBoundaryFillMeasurement {
    pub fn snapshot(&self) -> &[u8] {
        &self.candidate
    }
    pub fn measurement(&self) -> &Measurement {
        &self.measurement
    }
    pub fn report(&self) -> &BoundaryFillReport {
        &self.report
    }
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Document<'_> {
    pub fn prepare_boundary_fill(
        &self,
        page: usize,
        seed: [f32; 2],
        options: BoundaryFillOptions,
    ) -> Result<BoundaryFillJob, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        options.validate()?;
        let geometry = self.page_geometry(page)?;
        let source = self.snapshot()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(failure("source exceeds 128 MiB"));
        }
        let raster = self.render_visual_search_content(page, options.scale)?;
        let seed_pixel = pixel_point(seed, geometry, raster.width, raster.height, false)?;
        let mut closures = Vec::new();
        for closure in &options.closures {
            let a = pixel_point(closure.start, geometry, raster.width, raster.height, true)?;
            let b = pixel_point(closure.end, geometry, raster.width, raster.height, true)?;
            if closure.start == closure.end {
                return Err(failure("closure segment must have distinct endpoints"));
            }
            closures.push(ClosureCursor::new(a, b));
        }
        let source_sha256 = crate::local_review::digest(&source);
        Ok(BoundaryFillJob {
            source: Arc::new(source),
            source_sha256,
            page,
            seed,
            seed_pixel,
            geometry,
            width: raster.width,
            height: raster.height,
            options,
            mask: vec![0; (raster.width * raster.height) as usize],
            rgba: Some(raster.rgba),
            cursor: 0,
            closures,
            closure_index: 0,
            closure_pixels: 0,
            frontier: VecDeque::new(),
            filled: 0,
            edges: BTreeMap::new(),
            incoming: HashSet::new(),
            edge_count: 0,
            trace_start: None,
            trace_next: 0,
            trace_points: Vec::new(),
            last_direction: None,
            first_direction: None,
            rings: Vec::new(),
            status: BoundaryFillStatus::Running,
            phase: BoundaryFillPhase::Threshold,
            work: 0,
            region: None,
        })
    }
    pub fn verify_boundary_fill(&self, job: &BoundaryFillJob) -> Result<(), EngineError> {
        self.verify_boundary_source(&job.source, job.page, job.geometry)
    }
    fn verify_boundary_source(
        &self,
        source: &[u8],
        page: usize,
        geometry: PageGeometry,
    ) -> Result<(), EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let current = self.snapshot()?;
        if current.len() > MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(source, &current, self.password())?
            || self.page_geometry(page)? != geometry
        {
            return Err(failure("document changed; review the boundary again"));
        }
        Ok(())
    }
    fn boundary_write_permission(&self) -> Result<(), EngineError> {
        self.require_annotation_permission()?;
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(failure("signed PDFs cannot receive boundary measurements"));
        }
        Ok(())
    }
    pub fn prepare_boundary_fill_measurement(
        &self,
        job: &BoundaryFillJob,
        options: &BoundaryFillMeasurementOptions,
        author: Option<&str>,
    ) -> Result<PreparedBoundaryFillMeasurement, EngineError> {
        self.boundary_write_permission()?;
        self.verify_boundary_fill(job)?;
        if job.status != BoundaryFillStatus::Complete {
            return Err(failure(
                "only a complete, closed and reviewed region can become a measurement",
            ));
        }
        let region = job
            .region
            .as_ref()
            .ok_or_else(|| failure("no validated region"))?;
        let mut measurement = raw_measurement(&region.outer, &region.holes);
        let calibration =
            self.measurement_reference_for_geometry(job.page, &options.scale, &measurement.points)?;
        measurement.calibration = calibration.points;
        measurement.known_distance =
            calibration.known_distance * calibration.unit.metres() / options.unit.metres();
        measurement.unit = options.unit;
        measurement.precision = options.precision;
        measurement.value()?;
        let report = job.report();
        let reference = match &options.scale {
            RectangleScale::Automatic => "automatic".to_string(),
            RectangleScale::Page => "page".to_string(),
            RectangleScale::Region(id) => format!("region:{id}"),
            RectangleScale::Local(_) => "local".to_string(),
        };
        let provenance = BoundaryFillProvenance {
            version: 1,
            inspection: report.clone(),
            reference,
            measurement: measurement.result()?,
        };
        let bytes =
            serde_json::to_vec(&provenance).map_err(|_| failure("could not encode provenance"))?;
        if bytes.len() > MAX_PROVENANCE_BYTES {
            return Err(failure("provenance exceeds 128 KiB"));
        }
        let mut scratch = self.boundary_scratch(job.source.as_ref().clone())?;
        let index = scratch.add_measurement_annotation(job.page, &measurement, author)?;
        scratch.edit_annotation_pdf(job.page, |pdf, page, _| {
            let (slot, mut annotation) = crate::annot_write::load_annot(pdf, page, index)?;
            annotation.set(KEY, lopdf::Object::string_literal(bytes));
            crate::annot_write::store_annot(pdf, page, slot, annotation)
        })?;
        let candidate = scratch.snapshot()?;
        if candidate.len() > MAX_CANDIDATE_BYTES {
            return Err(failure("candidate exceeds 160 MiB"));
        }
        Ok(PreparedBoundaryFillMeasurement {
            original: job.source.clone(),
            candidate,
            page: job.page,
            geometry: job.geometry,
            index,
            measurement,
            report,
        })
    }
    pub fn apply_boundary_fill_measurement(
        &mut self,
        prepared: &PreparedBoundaryFillMeasurement,
    ) -> Result<usize, EngineError> {
        self.boundary_write_permission()?;
        self.verify_boundary_source(&prepared.original, prepared.page, prepared.geometry)?;
        self.restore(prepared.candidate.clone())?;
        Ok(prepared.index)
    }
    fn boundary_scratch(&self, bytes: Vec<u8>) -> Result<Document<'_>, EngineError> {
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
    pub fn render_boundary_fill_measurement_preview(
        &self,
        prepared: &PreparedBoundaryFillMeasurement,
        long_side: u32,
    ) -> Result<RenderedPage, EngineError> {
        if !(64..=1536).contains(&long_side) {
            return Err(failure("preview side must be 64..1536 pixels"));
        }
        self.verify_boundary_source(&prepared.original, prepared.page, prepared.geometry)?;
        let scratch = self.boundary_scratch(prepared.candidate.clone())?;
        let size = scratch.page_size(prepared.page)?;
        scratch.render_page(
            prepared.page,
            long_side as f32 / size.width_pt.max(size.height_pt),
        )
    }
    pub fn boundary_fill_provenance(
        &self,
        page: usize,
        index: usize,
    ) -> Result<Option<BoundaryFillProvenance>, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let snapshot = self.snapshot()?;
        if snapshot.len() > MAX_SOURCE_BYTES {
            return Err(failure("source exceeds 128 MiB"));
        }
        let pdf = crate::security::load(&snapshot, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page))?;
        let (_, annotation) = crate::annot_write::load_annot(&pdf, page_id, index)?;
        let Ok(raw) = annotation.get(KEY) else {
            return Ok(None);
        };
        let bytes = raw.as_str().map_err(crate::security::lopdf_err)?;
        if bytes.len() > MAX_PROVENANCE_BYTES {
            return Err(failure("provenance exceeds 128 KiB"));
        }
        let stored: BoundaryFillProvenance =
            serde_json::from_slice(bytes).map_err(|_| failure("invalid boundary provenance"))?;
        validate_provenance(&stored)?;
        Ok(Some(stored))
    }
}

fn validate_provenance(stored: &BoundaryFillProvenance) -> Result<(), EngineError> {
    let r = &stored.inspection;
    r.options.validate()?;
    let region = r
        .region
        .as_ref()
        .ok_or_else(|| failure("missing historical region"))?;
    let pixels = u64::from(r.raster_width) * u64::from(r.raster_height);
    let [w, h] = r.geometry.display_size;
    if stored.version != 1
        || r.version != 1
        || r.progress.status != BoundaryFillStatus::Complete
        || r.progress.phase != BoundaryFillPhase::Finished
        || r.source_sha256.len() != 64
        || !r.source_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || r.raster_width == 0
        || r.raster_height == 0
        || r.raster_width > MAX_SIDE
        || r.raster_height > MAX_SIDE
        || pixels > MAX_PIXELS as u64
        || !w.is_finite()
        || !h.is_finite()
        || w <= 0.0
        || h <= 0.0
        || ![0, 90, 180, 270].contains(&r.geometry.rotation)
        || !r.geometry.bounds.iter().all(|n| n.is_finite())
        || r.geometry.bounds[0] >= r.geometry.bounds[2]
        || r.geometry.bounds[1] >= r.geometry.bounds[3]
        || r.closure_width_pixels != 1
        || r.closure_rasterization != "supercover-grid-cells"
        || r.closure_pixels > pixels
        || region.pixel_count == 0
        || region.pixel_count > r.options.max_pixels as u64
        || region.pixel_count > pixels
        || region.pixel_count != r.progress.filled_pixels
        || r.progress.work > r.options.max_work
        || r.progress.edges > r.options.max_edges
        || r.progress.frontier != 0
        || r.progress.contours != region.holes.len() + 1
        || !region.area_points2.is_finite()
        || region.area_points2 <= 0.0
        || region
            .outer
            .iter()
            .chain(region.holes.iter().flatten())
            .chain(std::iter::once(&r.seed))
            .any(|p| {
                !p[0].is_finite()
                    || !p[1].is_finite()
                    || p[0] < 0.0
                    || p[0] > w
                    || p[1] < 0.0
                    || p[1] > h
            })
        || !(matches!(stored.reference.as_str(), "automatic" | "page" | "local")
            || stored.reference.strip_prefix("region:").is_some_and(|id| {
                !id.is_empty()
                    && id.len() <= 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            }))
    {
        return Err(failure("unsupported or inconsistent boundary provenance"));
    }
    let measurement = raw_measurement(&region.outer, &region.holes);
    let area = crate::measurement_area::breakdown(&measurement, 1.0)?;
    let pixel_area = region.pixel_count as f64 * f64::from(w) / f64::from(r.raster_width)
        * f64::from(h)
        / f64::from(r.raster_height);
    if (area.net - region.area_points2).abs() > area.net.max(1.0) * 1e-5
        || (area.net - pixel_area).abs() > area.net.max(1.0) * 1e-5
    {
        return Err(failure("historical region area is inconsistent"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PointRect, Rotation};

    fn raster_job(
        width: u32,
        height: u32,
        seed: (u32, u32),
        options: BoundaryFillOptions,
        blocked: impl Fn(u32, u32) -> bool,
    ) -> BoundaryFillJob {
        let geometry = PageGeometry {
            bbox: PointRect {
                left: 0.,
                bottom: 0.,
                right: width as f32,
                top: height as f32,
            },
            rotation: Rotation::R0,
        };
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let gray = if blocked(x, y) { 0 } else { 255 };
                rgba.extend_from_slice(&[gray, gray, gray, 255]);
            }
        }
        let closures = options
            .closures
            .iter()
            .map(|c| {
                ClosureCursor::new(
                    pixel_point(c.start, geometry, width, height, true).unwrap(),
                    pixel_point(c.end, geometry, width, height, true).unwrap(),
                )
            })
            .collect();
        BoundaryFillJob {
            source: Arc::new(Vec::new()),
            source_sha256: "0".repeat(64),
            page: 0,
            seed: [seed.0 as f32 + 0.5, height as f32 - seed.1 as f32 - 0.5],
            seed_pixel: seed,
            geometry,
            width,
            height,
            options,
            rgba: Some(rgba),
            mask: vec![0; (width * height) as usize],
            cursor: 0,
            closures,
            closure_index: 0,
            closure_pixels: 0,
            frontier: VecDeque::new(),
            filled: 0,
            edges: BTreeMap::new(),
            incoming: HashSet::new(),
            edge_count: 0,
            trace_start: None,
            trace_next: 0,
            trace_points: Vec::new(),
            last_direction: None,
            first_direction: None,
            rings: Vec::new(),
            status: BoundaryFillStatus::Running,
            phase: BoundaryFillPhase::Threshold,
            work: 0,
            region: None,
        }
    }
    fn finish(mut job: BoundaryFillJob, budget: usize) -> BoundaryFillJob {
        while !job.progress().is_terminal() {
            job.step(budget);
        }
        job
    }
    fn wall(x: u32, y: u32) -> bool {
        (x == 2 || x == 17) && (2..=17).contains(&y) || (y == 2 || y == 17) && (2..=17).contains(&x)
    }
    fn complete(job: &BoundaryFillJob) -> &BoundaryFillRegion {
        assert_eq!(
            job.status,
            BoundaryFillStatus::Complete,
            "{:?}",
            job.report()
        );
        job.region.as_ref().unwrap()
    }
    #[test]
    fn rectangle_holes_and_nonrectangular_rooms_retain_exact_cell_area() {
        let square = finish(raster_job(20, 20, (4, 4), Default::default(), wall), 7);
        let r = complete(&square);
        assert_eq!(r.outer.len(), 4);
        assert!(r.holes.is_empty());
        assert_eq!(r.pixel_count, 196);
        assert_eq!(r.area_points2, 196.);
        let holed = finish(
            raster_job(20, 20, (4, 4), Default::default(), |x, y| {
                wall(x, y) || (7..=9).contains(&x) && (7..=10).contains(&y)
            }),
            1,
        );
        let r = complete(&holed);
        assert_eq!(r.outer.len(), 4);
        assert_eq!(r.holes.len(), 1);
        assert_eq!(r.holes[0].len(), 4);
        assert_eq!(r.pixel_count, 184);
        assert_eq!(r.area_points2, 184.);
        let l = finish(
            raster_job(20, 20, (4, 4), Default::default(), |x, y| {
                wall(x, y) || x >= 10 && y >= 10
            }),
            29,
        );
        let r = complete(&l);
        assert_eq!(r.outer.len(), 6);
        assert!(r.holes.is_empty());
        assert_eq!(r.area_points2, 147.);
        let metadata = BoundaryFillProvenance {
            version: 1,
            inspection: holed.report(),
            reference: "page".into(),
            measurement: serde_json::json!({}),
        };
        validate_provenance(&metadata).unwrap();
        let mut invalid = metadata.clone();
        invalid.inspection.region.as_mut().unwrap().pixel_count += 1;
        assert!(validate_provenance(&invalid).is_err());
    }
    #[test]
    fn explicit_closure_seals_door_and_open_or_blocked_seeds_never_complete() {
        let open = |x, y| wall(x, y) && !(y == 2 && (8..=10).contains(&x));
        let leaked = finish(raster_job(20, 20, (4, 4), Default::default(), open), 31);
        assert_eq!(leaked.status, BoundaryFillStatus::Leaked);
        assert!(leaked.region.is_none());
        let options = BoundaryFillOptions {
            closures: vec![BoundaryClosure {
                start: [7.5, 17.5],
                end: [11.5, 17.5],
            }],
            ..Default::default()
        };
        let closed = finish(raster_job(20, 20, (4, 4), options, open), 3);
        assert_eq!(complete(&closed).area_points2, 196.);
        assert_eq!(closed.closure_pixels, 3);
        let blocked = finish(raster_job(20, 20, (2, 4), Default::default(), wall), 3);
        assert_eq!(blocked.status, BoundaryFillStatus::SeedBlocked);
        assert!(blocked.region.is_none());
        // Two free cells sharing only a corner are separate four-connected regions.
        let diagonal = finish(
            raster_job(8, 8, (3, 3), Default::default(), |x, y| {
                ![(3, 3), (4, 4)].contains(&(x, y))
            }),
            1,
        );
        assert_eq!(complete(&diagonal).pixel_count, 1);
        // Two internal blocked islands touching only at a corner have ambiguous contours.
        let ambiguous = finish(
            raster_job(20, 20, (4, 4), Default::default(), |x, y| {
                wall(x, y) || [(8, 8), (9, 9)].contains(&(x, y))
            }),
            19,
        );
        assert_eq!(ambiguous.status, BoundaryFillStatus::TopologyRejected);
        assert!(ambiguous.region.is_none());
    }
    #[test]
    fn budgets_and_cancellation_cover_each_incremental_phase() {
        let expected = finish(raster_job(20, 20, (4, 4), Default::default(), wall), 1).report();
        assert_eq!(
            expected,
            finish(
                raster_job(20, 20, (4, 4), Default::default(), wall),
                usize::MAX
            )
            .report()
        );
        for phase in [
            BoundaryFillPhase::Threshold,
            BoundaryFillPhase::Closures,
            BoundaryFillPhase::Flood,
            BoundaryFillPhase::Edges,
            BoundaryFillPhase::Contours,
            BoundaryFillPhase::Validation,
        ] {
            let mut job = raster_job(20, 20, (4, 4), Default::default(), wall);
            while job.phase != phase {
                assert!(!job.progress().is_terminal());
                let before = job.work;
                job.step(1);
                assert!(job.work <= before + 1);
            }
            job.cancel();
            let after = job.report();
            assert_eq!(after.progress.status, BoundaryFillStatus::Cancelled);
            assert!(after.region.is_none());
            job.step(usize::MAX);
            assert_eq!(job.report(), after);
        }
        let mut job = raster_job(20, 20, (4, 4), Default::default(), wall);
        let before = job.report();
        job.step(0);
        assert_eq!(before, job.report());
        let cases = [
            (
                BoundaryFillOptions {
                    max_pixels: 1,
                    ..Default::default()
                },
                BoundaryFillStatus::PixelLimit,
            ),
            (
                BoundaryFillOptions {
                    max_frontier: 1,
                    ..Default::default()
                },
                BoundaryFillStatus::FrontierLimit,
            ),
            (
                BoundaryFillOptions {
                    max_edges: 3,
                    ..Default::default()
                },
                BoundaryFillStatus::EdgeLimit,
            ),
            (
                BoundaryFillOptions {
                    max_work: 1,
                    ..Default::default()
                },
                BoundaryFillStatus::WorkLimit,
            ),
        ];
        for (options, status) in cases {
            let job = finish(raster_job(20, 20, (4, 4), options, wall), 17);
            assert_eq!(job.status, status);
            assert!(job.region.is_none());
        }
        // The edge pass must charge even unrelated background cells, not only the region.
        let job = finish(
            raster_job(
                200,
                200,
                (3, 3),
                BoundaryFillOptions {
                    max_work: 45_000,
                    ..Default::default()
                },
                |x, y| x == 2 || x == 4 || y == 2 || y == 4,
            ),
            4096,
        );
        assert_eq!(job.status, BoundaryFillStatus::WorkLimit);
        assert_eq!(job.work, 45_000);
    }
    #[test]
    fn complex_regions_are_refused_without_discarding_holes_or_stair_steps() {
        let holes = finish(
            raster_job(80, 80, (5, 5), Default::default(), |x, y| {
                x == 2 || x == 77 || y == 2 || y == 77 || (x % 8 == 0 && y % 8 == 0)
            }),
            8192,
        );
        assert_eq!(holes.status, BoundaryFillStatus::ComplexityLimit);
        assert!(holes.region.is_none());
        let stairs = finish(
            raster_job(160, 160, (4, 4), Default::default(), |x, y| {
                x <= 2 || y <= 2 || x >= 157 || y >= 157 || x > y + 4
            }),
            8192,
        );
        assert_eq!(stairs.status, BoundaryFillStatus::ComplexityLimit);
        assert!(stairs.region.is_none());
    }
    #[test]
    fn closure_supercover_is_bounded_symmetric_and_explicit() {
        for a in [(0, 0), (4, 0), (0, 4), (4, 4), (2, 2)] {
            for b in [(0, 0), (4, 0), (0, 4), (4, 4), (2, 2)] {
                let collect = |from, to| {
                    let mut cursor = ClosureCursor::new(from, to);
                    let mut cells = std::collections::BTreeSet::new();
                    for _ in 0..16 {
                        cells.extend(cursor.next());
                        if cursor.finished {
                            break;
                        }
                    }
                    assert!(cursor.finished);
                    assert!(cells.iter().all(|&(x, y)| x <= 4 && y <= 4));
                    cells
                };
                assert_eq!(collect(a, b), collect(b, a));
            }
        }
        let mut exact = ClosureCursor::new((1, 1), (2, 2));
        assert_eq!(exact.next(), vec![(1, 1), (2, 1), (1, 2)]);
        assert_eq!(exact.next(), vec![(2, 2)]);
        assert!(exact.finished);
    }
    #[test]
    fn alpha_threshold_and_strict_options_have_no_implicit_cleanup() {
        let mut job = raster_job(20, 20, (4, 4), Default::default(), wall);
        let at = (2 * 20 + 8) * 4;
        job.rgba.as_mut().unwrap()[at + 3] = 0;
        assert_eq!(
            finish(job, 8192).status,
            BoundaryFillStatus::Leaked,
            "transparent black is white after compositing"
        );
        let mut value = serde_json::to_value(BoundaryFillOptions::default()).unwrap();
        value["automatic_closure"] = true.into();
        assert!(serde_json::from_value::<BoundaryFillOptions>(value).is_err());
        for options in [
            BoundaryFillOptions {
                scale: f32::NAN,
                ..Default::default()
            },
            BoundaryFillOptions {
                max_work: 0,
                ..Default::default()
            },
            BoundaryFillOptions {
                max_edges: MAX_EDGES + 1,
                ..Default::default()
            },
            BoundaryFillOptions {
                closures: vec![
                    BoundaryClosure {
                        start: [1., 1.],
                        end: [2., 2.]
                    };
                    MAX_CLOSURES + 1
                ],
                ..Default::default()
            },
        ] {
            assert!(options.validate().is_err());
        }
    }
}
