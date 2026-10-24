//! Bounded polygon topology and explicitly deducted areas.

use crate::{measurement::Measurement, EngineError};
use serde::Serialize;

pub const MAX_HOLES: usize = 16;
pub const MAX_RING_VERTICES: usize = 128;
pub const MAX_TOTAL_VERTICES: usize = 512;

/// All values use the measurement's squared length unit. Deductions retain ring order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeasurementArea {
    pub gross: f64,
    pub deductions: Vec<f64>,
    pub net: f64,
}

fn invalid() -> EngineError {
    EngineError::Other("invalid deducted area: use simple rings with at most 16 holes, 128 vertices per ring and 512 total; holes must be strictly inside without touching, overlap or nesting".into())
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f64 {
    (f64::from(b.0) - f64::from(a.0)).hypot(f64::from(b.1) - f64::from(a.1))
}

fn cross(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f64 {
    (f64::from(b.0) - f64::from(a.0)) * (f64::from(c.1) - f64::from(a.1))
        - (f64::from(b.1) - f64::from(a.1)) * (f64::from(c.0) - f64::from(a.0))
}

fn intersects(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    a.0.min(b.0) <= c.0.max(d.0)
        && c.0.min(d.0) <= a.0.max(b.0)
        && a.1.min(b.1) <= c.1.max(d.1)
        && c.1.min(d.1) <= a.1.max(b.1)
        && cross(a, b, c) * cross(a, b, d) <= 0.0
        && cross(c, d, a) * cross(c, d, b) <= 0.0
}

pub(crate) fn ring_area(ring: &[(f32, f32)]) -> Result<f64, EngineError> {
    let n = ring.len();
    if !(3..=MAX_RING_VERTICES).contains(&n)
        || ring.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err(invalid());
    }
    for i in 0..n {
        let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        if distance(b, c) < 0.01 {
            return Err(invalid());
        }
        let dot = (f64::from(a.0) - f64::from(b.0)) * (f64::from(c.0) - f64::from(b.0))
            + (f64::from(a.1) - f64::from(b.1)) * (f64::from(c.1) - f64::from(b.1));
        if cross(a, b, c).abs() < 1e-6 && dot > 0.0 {
            return Err(invalid());
        }
        for j in i + 1..n {
            if j != i + 1
                && !(i == 0 && j == n - 1)
                && intersects(ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n])
            {
                return Err(invalid());
            }
        }
    }
    // Translate before the shoelace sum to retain precision at large crop origins.
    let area = (1..n - 1)
        .map(|i| cross(ring[0], ring[i], ring[i + 1]))
        .sum::<f64>()
        .abs()
        * 0.5;
    if !area.is_finite() || area < 0.000_000_5 {
        return Err(invalid());
    }
    Ok(area)
}

fn strictly_inside(point: (f32, f32), ring: &[(f32, f32)]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if cross(a, b, point) == 0.0
            && point.0 >= a.0.min(b.0)
            && point.0 <= a.0.max(b.0)
            && point.1 >= a.1.min(b.1)
            && point.1 <= a.1.max(b.1)
        {
            return false;
        }
        if (a.1 > point.1) != (b.1 > point.1) {
            let x = f64::from(a.0)
                + (f64::from(point.1) - f64::from(a.1)) * (f64::from(b.0) - f64::from(a.0))
                    / (f64::from(b.1) - f64::from(a.1));
            if f64::from(point.0) < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn rings_intersect(a: &[(f32, f32)], b: &[(f32, f32)]) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len()).any(|j| intersects(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()]))
    })
}

pub(crate) fn breakdown(
    measurement: &Measurement,
    scale: f64,
) -> Result<MeasurementArea, EngineError> {
    if measurement.holes.len() > MAX_HOLES
        || measurement
            .points
            .len()
            .saturating_add(measurement.holes.iter().map(Vec::len).sum::<usize>())
            > MAX_TOTAL_VERTICES
    {
        return Err(invalid());
    }
    let gross_points = ring_area(&measurement.points)?;
    let mut holes = Vec::new();
    for (index, ring) in measurement.holes.iter().enumerate() {
        let area = ring_area(ring)?;
        if !ring
            .iter()
            .all(|&point| strictly_inside(point, &measurement.points))
            || rings_intersect(ring, &measurement.points)
        {
            return Err(invalid());
        }
        for other in &measurement.holes[..index] {
            if rings_intersect(ring, other)
                || strictly_inside(ring[0], other)
                || strictly_inside(other[0], ring)
            {
                return Err(invalid());
            }
        }
        holes.push(area);
    }
    let net_points = gross_points - holes.iter().sum::<f64>();
    let result = MeasurementArea {
        gross: gross_points * scale * scale,
        deductions: holes.into_iter().map(|area| area * scale * scale).collect(),
        net: net_points * scale * scale,
    };
    if !result.gross.is_finite()
        || !result.net.is_finite()
        || result.net <= 0.0
        || result
            .deductions
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(invalid());
    }
    Ok(result)
}
