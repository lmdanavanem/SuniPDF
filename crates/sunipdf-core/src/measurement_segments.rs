//! A physical length and clockwise direction resolved against a current page reference.

use crate::measurement::{
    annotation_components, dimensioned_rectangle_point_dimensions, measurement_after_storage,
    normalize_rectangle_rotation, Measurement, MeasurementKind, MeasurementUnit, PageCalibration,
    RectangleScale,
};
use crate::{Document, EngineError};

const RELATIVE_TOLERANCE: f64 = 1e-5;

/// A straight length measurement starting at a displayed-page point. Zero degrees points
/// right; positive angles turn clockwise. The two saved vertices remain freely editable.
#[derive(Debug, Clone)]
pub struct DimensionedSegment {
    pub start: (f32, f32),
    pub length: f64,
    pub rotation_degrees: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
    pub scale: RectangleScale,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("dimensioned segment: {reason}"))
}

fn direction(degrees: f64) -> Result<(f64, f64), EngineError> {
    let degrees = normalize_rectangle_rotation(degrees)
        .map_err(|_| invalid("rotation must be a finite angle"))?;
    Ok(match degrees {
        0.0 => (1.0, 0.0),
        90.0 => (0.0, -1.0),
        180.0 => (-1.0, 0.0),
        270.0 => (0.0, 1.0),
        _ => {
            let (sin, cos) = degrees.to_radians().sin_cos();
            (cos, -sin)
        }
    })
}

fn checked_precision(
    points: &[(f32, f32)],
    start: (f32, f32),
    length_points: f64,
    rotation_degrees: f64,
) -> Result<(), EngineError> {
    if points.len() != 2 || !length_points.is_finite() || length_points <= 0.0 {
        return Err(invalid("enter a positive representable length"));
    }
    let (ux, uy) = direction(rotation_degrees)?;
    let dx = f64::from(points[1].0) - f64::from(points[0].0);
    let dy = f64::from(points[1].1) - f64::from(points[0].1);
    let actual = dx.hypot(dy);
    let tolerance = length_points * RELATIVE_TOLERANCE;
    if !actual.is_finite()
        || actual <= 0.0
        || (actual - length_points).abs() > tolerance
        || (dx - length_points * ux).abs() > tolerance
        || (dy - length_points * uy).abs() > tolerance
        || (f64::from(points[0].0) - f64::from(start.0)).abs() > 0.001
        || (f64::from(points[0].1) - f64::from(start.1)).abs() > 0.001
    {
        return Err(invalid(
            "length, direction or start cannot be represented accurately at this page position",
        ));
    }
    Ok(())
}

/// Convert the requested length using the same isotropic scale arithmetic as other
/// dimensioned shapes. Live-document construction additionally verifies reference provenance.
pub fn dimensioned_segment_point_length(
    length: f64,
    unit: MeasurementUnit,
    calibration: &PageCalibration,
    page_size: (f32, f32),
) -> Result<f64, EngineError> {
    dimensioned_rectangle_point_dimensions(length, length, unit, calibration, page_size)
        .map(|(length, _)| length)
        .map_err(|error| match error {
            EngineError::Other(message) => invalid(
                message
                    .strip_prefix("dimensioned rectangle: ")
                    .unwrap_or(&message),
            ),
            error => error,
        })
}

/// Construct an open two-vertex segment without snapping or clipping its computed endpoint.
pub fn dimensioned_segment_points(
    start: (f32, f32),
    length_points: f64,
    rotation_degrees: f64,
    page_size: (f32, f32),
) -> Result<[(f32, f32); 2], EngineError> {
    if !start.0.is_finite()
        || !start.1.is_finite()
        || !length_points.is_finite()
        || length_points <= 0.0
        || !page_size.0.is_finite()
        || !page_size.1.is_finite()
        || page_size.0 <= 0.0
        || page_size.1 <= 0.0
    {
        return Err(invalid("enter a positive finite length and a finite start"));
    }
    let (ux, uy) = direction(rotation_degrees)?;
    let end = (
        f64::from(start.0) + length_points * ux,
        f64::from(start.1) + length_points * uy,
    );
    if [(f64::from(start.0), f64::from(start.1)), end]
        .iter()
        .any(|&(x, y)| {
            !x.is_finite()
                || !y.is_finite()
                || x < 0.0
                || y < 0.0
                || x > f64::from(page_size.0)
                || y > f64::from(page_size.1)
        })
    {
        return Err(invalid("the segment extends outside the displayed page"));
    }
    let points = [start, (end.0 as f32, end.1 as f32)];
    checked_precision(&points, start, length_points, rotation_degrees)?;
    Ok(points)
}

impl Document<'_> {
    /// Resolve the current scale and exact stored geometry without changing the document.
    pub fn dimensioned_segment(
        &self,
        page: usize,
        request: &DimensionedSegment,
    ) -> Result<Measurement, EngineError> {
        if request.precision > 4 {
            return Err(invalid("choose 0 to 4 decimals"));
        }
        let calibration =
            self.measurement_reference_for_geometry(page, &request.scale, &[request.start])?;
        let geo = self.page_geometry(page)?;
        let length_points = dimensioned_segment_point_length(
            request.length,
            request.unit,
            &calibration,
            geo.display_size(),
        )?;
        let measurement = Measurement {
            kind: MeasurementKind::Length,
            points: dimensioned_segment_points(
                request.start,
                length_points,
                request.rotation_degrees,
                geo.display_size(),
            )?
            .to_vec(),
            holes: Vec::new(),
            depth: None,
            calibration: calibration.points,
            known_distance: calibration.known_distance * calibration.unit.metres()
                / request.unit.metres(),
            unit: request.unit,
            precision: request.precision,
        };
        let stored = measurement_after_storage(&geo, &measurement)?;
        checked_precision(
            &stored.points,
            request.start,
            length_points,
            request.rotation_degrees,
        )?;
        let value = stored.value()?;
        if (value - request.length).abs() > request.length * RELATIVE_TOLERANCE {
            return Err(invalid("physical length loses precision when stored"));
        }
        let complete =
            self.measurement_reference_for_geometry(page, &request.scale, &stored.points)?;
        if complete != calibration {
            return Err(invalid("scale selection changed along the segment"));
        }
        annotation_components(&geo, &stored, None)?;
        Ok(stored)
    }

    /// Add one ordinary calibrated length annotation through the shared atomic mutation.
    pub fn add_dimensioned_segment(
        &mut self,
        page: usize,
        request: &DimensionedSegment,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let measurement = self.dimensioned_segment(page, request)?;
        self.add_measurement_annotation(page, &measurement, author)
    }
}
