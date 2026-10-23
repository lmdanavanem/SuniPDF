//! Bounded snapping to real PDF vector endpoints and straight-segment intersections.

use crate::{Document, EngineError, PageGeometry};
use pdfium_render::prelude::*;

type Point = (f32, f32);
const MAX_ENTRIES: usize = 50_000;
const MAX_LOCAL_SEGMENTS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Endpoint,
    Intersection,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapPoint {
    pub point: Point,
    pub kind: SnapKind,
}

#[derive(Debug, Clone, Default)]
pub struct VectorSnapGeometry {
    endpoints: Vec<Point>,
    segments: Vec<(Point, Point)>,
    /// Unsupported non-convex/curved clipped objects are omitted, never guessed.
    pub skipped_clipped_objects: usize,
    width: f32,
    height: f32,
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}
fn intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let delta = |u: Point, v: Point| {
        (
            f64::from(u.0) - f64::from(v.0),
            f64::from(u.1) - f64::from(v.1),
        )
    };
    let r = delta(b, a);
    let s = delta(d, c);
    let q = delta(c, a);
    let determinant = cross(r, s);
    // Coincident lines have no unique intersection; endpoints remain independently available.
    if determinant.abs() <= 1e-12 * (r.0.hypot(r.1) * s.0.hypot(s.1)).max(1.0) {
        return None;
    }
    let t = cross(q, s) / determinant;
    let u = cross(q, r) / determinant;
    if !(-1e-10..=1.0 + 1e-10).contains(&t) || !(-1e-10..=1.0 + 1e-10).contains(&u) {
        return None;
    }
    Some((
        (f64::from(a.0) + t * r.0) as f32,
        (f64::from(a.1) + t * r.1) as f32,
    ))
}

impl VectorSnapGeometry {
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty() && self.segments.is_empty()
    }
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
    fn inside(&self, p: Point) -> bool {
        p.0 >= 0.0 && p.1 >= 0.0 && p.0 <= self.width && p.1 <= self.height
    }

    /// Tolerance is in displayed PDF points, normally a fixed pixel radius / current zoom.
    /// Dense ambiguous neighborhoods exceeding 256 segments deliberately decline snapping.
    /// Curves contribute their real endpoints, never control handles or chord intersections.
    pub fn nearest(&self, point: Point, tolerance: f32) -> Option<SnapPoint> {
        if !point.0.is_finite()
            || !point.1.is_finite()
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || tolerance > 100.0
        {
            return None;
        }
        let nearby: Vec<_> = self
            .segments
            .iter()
            .filter(|(a, b)| {
                a.0.min(b.0) <= point.0 + tolerance
                    && a.0.max(b.0) >= point.0 - tolerance
                    && a.1.min(b.1) <= point.1 + tolerance
                    && a.1.max(b.1) >= point.1 - tolerance
            })
            .take(MAX_LOCAL_SEGMENTS + 1)
            .collect();
        if nearby.len() > MAX_LOCAL_SEGMENTS {
            return None;
        }
        let mut best = None;
        let mut best_distance = f64::from(tolerance).powi(2);
        let mut consider = |candidate: Point, kind: SnapKind| {
            if !self.inside(candidate) {
                return;
            }
            let distance = (f64::from(candidate.0) - f64::from(point.0)).powi(2)
                + (f64::from(candidate.1) - f64::from(point.1)).powi(2);
            if distance <= best_distance && (distance < best_distance || best.is_none()) {
                best_distance = distance;
                best = Some(SnapPoint {
                    point: candidate,
                    kind,
                });
            }
        };
        for &candidate in &self.endpoints {
            consider(candidate, SnapKind::Endpoint);
        }
        for (i, &&(a, b)) in nearby.iter().enumerate() {
            for &&(c, d) in &nearby[i + 1..] {
                if let Some(p) = intersection(a, b, c, d) {
                    consider(p, SnapKind::Intersection);
                }
            }
        }
        best
    }
}

fn invalid() -> EngineError {
    EngineError::Other(
        "vector snapping exceeds supported geometry limits; use manual points on this page".into(),
    )
}
fn consume(remaining: &mut usize) -> Result<(), EngineError> {
    *remaining = remaining.checked_sub(1).ok_or_else(invalid)?;
    Ok(())
}

fn polygon_side(polygon: &[Point]) -> Option<f64> {
    let mut sign = 0.0_f64;
    for i in 0..polygon.len() {
        let (a, b, c) = (
            polygon[i],
            polygon[(i + 1) % polygon.len()],
            polygon[(i + 2) % polygon.len()],
        );
        let turn = cross(
            (f64::from(b.0 - a.0), f64::from(b.1 - a.1)),
            (f64::from(c.0 - b.0), f64::from(c.1 - b.1)),
        );
        if turn.abs() < 1e-9 {
            continue;
        }
        if sign != 0.0 && sign != turn.signum() {
            return None;
        }
        sign = turn.signum();
    }
    if sign == 0.0 {
        return None;
    }
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let edge = (
            f64::from(b.0) - f64::from(a.0),
            f64::from(b.1) - f64::from(a.1),
        );
        if polygon.iter().any(|p| {
            sign * cross(
                edge,
                (
                    f64::from(p.0) - f64::from(a.0),
                    f64::from(p.1) - f64::from(a.1),
                ),
            ) < -1e-9
        }) {
            return None;
        }
    }
    Some(sign)
}

fn clip_segment(mut a: Point, mut b: Point, clips: &[Vec<Point>]) -> Option<(Point, Point)> {
    for polygon in clips {
        let sign = polygon_side(polygon)?;
        let mut low = 0.0_f64;
        let mut high = 1.0_f64;
        for i in 0..polygon.len() {
            let start = polygon[i];
            let end = polygon[(i + 1) % polygon.len()];
            let edge = (
                f64::from(end.0) - f64::from(start.0),
                f64::from(end.1) - f64::from(start.1),
            );
            let side = |p: Point| {
                sign * cross(
                    edge,
                    (
                        f64::from(p.0) - f64::from(start.0),
                        f64::from(p.1) - f64::from(start.1),
                    ),
                )
            };
            let sa = side(a);
            let sb = side(b);
            if sa < 0.0 && sb < 0.0 {
                return None;
            }
            if sa < 0.0 {
                low = low.max(sa / (sa - sb));
            }
            if sb < 0.0 {
                high = high.min(sa / (sa - sb));
            }
            if low > high {
                return None;
            }
        }
        let interpolate = |t: f64| {
            (
                (f64::from(a.0) + t * (f64::from(b.0) - f64::from(a.0))) as f32,
                (f64::from(a.1) + t * (f64::from(b.1) - f64::from(a.1))) as f32,
            )
        };
        (a, b) = (interpolate(low), interpolate(high));
    }
    Some((a, b))
}

fn add_endpoint(result: &mut VectorSnapGeometry, point: Point, clips: &[Vec<Point>]) {
    if clip_segment(point, point, clips).is_some() {
        result.endpoints.push(point);
    }
}
fn add_segment(result: &mut VectorSnapGeometry, a: Point, b: Point, clips: &[Vec<Point>]) {
    add_endpoint(result, a, clips);
    add_endpoint(result, b, clips);
    if let Some((a, b)) = clip_segment(a, b, clips) {
        if a != b {
            result.segments.push((a, b));
        }
    }
}

fn object_clips(
    object: &PdfPageObject<'_>,
    parent: PdfMatrix,
    geo: &PageGeometry,
    remaining: &mut usize,
) -> Result<Option<Vec<Vec<Point>>>, EngineError> {
    let Some(clip) = object.get_clip_path() else {
        return Ok(Some(Vec::new()));
    };
    if clip.len() > 32 {
        return Ok(None);
    }
    let mut clips = Vec::new();
    for i in 0..clip.len() {
        let path = clip.get(i)?;
        if !(3..=129).contains(&path.len()) {
            return Ok(None);
        }
        let mut polygon = Vec::new();
        for j in 0..path.len() {
            consume(remaining)?;
            let segment = path.get(j)?;
            if (j == 0 && segment.segment_type() != PdfPathSegmentType::MoveTo)
                || (j != 0 && segment.segment_type() != PdfPathSegmentType::LineTo)
            {
                return Ok(None);
            }
            let (x, y) = segment.point();
            let (x, y) = parent.apply_to_points(x, y);
            let p = geo.to_display(x.value, y.value);
            if !p.0.is_finite() || !p.1.is_finite() || p.0.abs() > 1e8 || p.1.abs() > 1e8 {
                return Err(invalid());
            }
            if polygon.last() != Some(&p) {
                polygon.push(p);
            }
        }
        if polygon.first() == polygon.last() {
            polygon.pop();
        }
        if polygon.len() < 3 || polygon_side(&polygon).is_none() {
            return Ok(None);
        }
        clips.push(polygon);
    }
    Ok(Some(clips))
}

fn collect(
    object: &PdfPageObject<'_>,
    parent: PdfMatrix,
    geo: &PageGeometry,
    result: &mut VectorSnapGeometry,
    remaining: &mut usize,
    depth: usize,
    inherited_clips: &[Vec<Point>],
) -> Result<(), EngineError> {
    consume(remaining)?;
    if depth > 32 {
        return Err(invalid());
    }
    if !object.is_active()? {
        return Ok(());
    }
    let Some(mut clips) = object_clips(object, parent, geo, remaining)? else {
        result.skipped_clipped_objects += 1;
        return Ok(());
    };
    clips.extend_from_slice(inherited_clips);
    if clips.len() > 64 {
        return Err(invalid());
    }
    let matrix = object.matrix()?.multiply(parent);
    if let Some(path) = object.as_path_object() {
        if !path.is_stroked()? && path.fill_mode()? == PdfPathFillMode::None {
            return Ok(());
        }
        let segments = path.segments();
        let clipping_work = clips
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_mul(segments.len() as usize);
        *remaining = remaining.checked_sub(clipping_work).ok_or_else(invalid)?;
        let mut current = None;
        let mut start = None;
        let mut curve_control = 0;
        for i in 0..segments.len() {
            consume(remaining)?;
            let segment = segments.get(i)?;
            let (x, y) = segment.point();
            let (x, y) = matrix.apply_to_points(x, y);
            let p = geo.to_display(x.value, y.value);
            if !p.0.is_finite() || !p.1.is_finite() || p.0.abs() > 1e8 || p.1.abs() > 1e8 {
                return Err(invalid());
            }
            match segment.segment_type() {
                PdfPathSegmentType::MoveTo => {
                    if curve_control != 0 {
                        return Err(invalid());
                    }
                    current = Some(p);
                    start = Some(p);
                }
                PdfPathSegmentType::LineTo => {
                    if curve_control != 0 {
                        return Err(invalid());
                    }
                    if let Some(previous) = current {
                        add_segment(result, previous, p, &clips);
                    }
                    current = Some(p);
                }
                PdfPathSegmentType::BezierTo => {
                    curve_control += 1;
                    if curve_control == 3 {
                        if let Some(previous) = current {
                            add_endpoint(result, previous, &clips);
                            add_endpoint(result, p, &clips);
                        }
                        current = Some(p);
                        curve_control = 0;
                    }
                }
                PdfPathSegmentType::Unknown => return Err(invalid()),
            }
            if segment.is_close() {
                if curve_control != 0 {
                    return Err(invalid());
                }
                if let (Some(previous), Some(first)) = (current, start) {
                    add_segment(result, previous, first, &clips);
                    current = Some(first);
                }
            }
        }
        if curve_control != 0 {
            return Err(invalid());
        }
    } else if let Some(form) = object.as_x_object_form_object() {
        for i in 0..form.len() {
            collect(
                &form.get(i)?,
                matrix,
                geo,
                result,
                remaining,
                depth + 1,
                &clips,
            )?;
        }
    }
    Ok(())
}

impl Document<'_> {
    /// Reads native vector paths through PDFium, including nested transformed forms. Page
    /// rotation and CropBox offsets are converted by the same geometry used for rendering.
    /// Scans and text/image bounding boxes never synthesize measurement snap points.
    pub fn vector_snap_geometry(
        &self,
        page_index: usize,
    ) -> Result<VectorSnapGeometry, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let (width, height) = geo.display_size();
        let mut result = VectorSnapGeometry {
            width,
            height,
            ..Default::default()
        };
        let page = self.page_handle(page_index)?;
        let mut remaining = MAX_ENTRIES;
        for i in 0..page.objects().len() {
            collect(
                &page.objects().get(i)?,
                PdfMatrix::identity(),
                &geo,
                &mut result,
                &mut remaining,
                0,
                &[],
            )?;
        }
        result
            .endpoints
            .retain(|&(x, y)| x >= 0.0 && y >= 0.0 && x <= width && y <= height);
        result
            .endpoints
            .sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        result.endpoints.dedup();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipping_never_creates_endpoint_candidates_at_hidden_geometry_or_clip_edges() {
        let clips = vec![vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]];
        let mut g = VectorSnapGeometry {
            width: 100.0,
            height: 100.0,
            ..Default::default()
        };
        add_segment(&mut g, (-10.0, 10.0), (30.0, 10.0), &clips);
        assert!(g.endpoints.is_empty());
        assert_eq!(g.segments, vec![((0.0, 10.0), (20.0, 10.0))]);
        assert!(g.nearest((20.0, 10.0), 1.0).is_none());
        add_segment(&mut g, (10.0, 0.0), (10.0, 20.0), &clips);
        assert_eq!(
            g.nearest((10.0, 10.0), 1.0).unwrap().kind,
            SnapKind::Intersection
        );
        assert!(polygon_side(&[
            (0.0, 0.0),
            (20.0, 0.0),
            (10.0, 5.0),
            (20.0, 20.0),
            (0.0, 20.0)
        ])
        .is_none());
    }
    #[test]
    fn intersections_are_bounded_not_infinite_line_extensions() {
        let g = VectorSnapGeometry {
            endpoints: vec![(0.0, 0.0), (100.0, 100.0), (0.0, 100.0), (100.0, 0.0)],
            segments: vec![((0.0, 0.0), (100.0, 100.0)), ((0.0, 100.0), (100.0, 0.0))],
            width: 100.0,
            height: 100.0,
            ..Default::default()
        };
        assert_eq!(
            g.nearest((51.0, 49.0), 2.0),
            Some(SnapPoint {
                point: (50.0, 50.0),
                kind: SnapKind::Intersection
            })
        );
        assert_eq!(g.nearest((1.0, 1.0), 2.0).unwrap().kind, SnapKind::Endpoint);
        assert!(g.nearest((51.0, 49.0), 0.5).is_none());
        assert!(intersection((0.0, 0.0), (1.0, 1.0), (2.0, 0.0), (2.0, 3.0)).is_none());
        assert!(intersection((0.0, 0.0), (1.0, 1.0), (0.0, 0.0), (2.0, 2.0)).is_none());
        assert!(g.nearest((f32::NAN, 0.0), 2.0).is_none());
    }
}
