//! Dimensioned circular and elliptical areas stored as bounded, editable polygons.

use crate::measurement::{
    annotation_components, dimensioned_rectangle_point_dimensions, normalize_rectangle_rotation,
    Measurement, MeasurementKind, MeasurementUnit, PageCalibration, RectangleScale,
};
use crate::{Document, EngineError};

/// The saved outline and its quantity use these 128 straight segments, not an analytic ellipse.
pub const DIMENSIONED_ELLIPSE_SEGMENTS: usize = 128;
const RELATIVE_TOLERANCE: f64 = 1e-5;
const AREA_RELATIVE_TOLERANCE: f64 =
    2.0 * RELATIVE_TOLERANCE + RELATIVE_TOLERANCE * RELATIVE_TOLERANCE;

/// Physical diameters follow the local X/Y axes, rotated clockwise around the displayed-page
/// center. Equal diameters construct a circular polygon. The result is an ordinary area
/// measurement; later vertex edits do not preserve an ellipse or a dimensional constraint.
#[derive(Debug, Clone)]
pub struct DimensionedEllipse {
    pub center: (f32, f32),
    pub diameter_x: f64,
    pub diameter_y: f64,
    pub rotation_degrees: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
    pub scale: RectangleScale,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("dimensioned ellipse: {reason}"))
}

fn sin_cos(degrees: f64) -> (f64, f64) {
    match degrees {
        0.0 => (0.0, 1.0),
        90.0 => (1.0, 0.0),
        180.0 => (0.0, -1.0),
        270.0 => (-1.0, 0.0),
        _ => degrees.to_radians().sin_cos(),
    }
}

fn rotation(degrees: f64) -> Result<f64, EngineError> {
    normalize_rectangle_rotation(degrees).map_err(|_| invalid("rotation must be a finite angle"))
}

fn sample(index: usize) -> (f64, f64) {
    sin_cos(index as f64 * (360.0 / DIMENSIONED_ELLIPSE_SEGMENTS as f64))
}

fn sampled_area(diameter_x: f64, diameter_y: f64) -> f64 {
    (DIMENSIONED_ELLIPSE_SEGMENTS as f64 / 8.0)
        * diameter_x
        * diameter_y
        * (std::f64::consts::TAU / DIMENSIONED_ELLIPSE_SEGMENTS as f64).sin()
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && expected > 0.0
        && (actual - expected).abs() <= expected * tolerance
}

fn check_points(
    points: &[(f32, f32)],
    center: (f32, f32),
    diameter_x: f64,
    diameter_y: f64,
    degrees: f64,
) -> Result<(), EngineError> {
    if points.len() != DIMENSIONED_ELLIPSE_SEGMENTS {
        return Err(invalid("the outline requires 128 vertices"));
    }
    let (sin, cos) = sin_cos(degrees);
    for (index, &(x, y)) in points.iter().enumerate() {
        let dx = f64::from(x) - f64::from(center.0);
        let dy = f64::from(y) - f64::from(center.1);
        let (s, c) = sample(index);
        let error_x = dx * cos - dy * sin - diameter_x * 0.5 * c;
        let error_y = dx * sin + dy * cos - diameter_y * 0.5 * s;
        if !error_x.is_finite()
            || !error_y.is_finite()
            || error_x.abs() > diameter_x * RELATIVE_TOLERANCE
            || error_y.abs() > diameter_y * RELATIVE_TOLERANCE
        {
            return Err(invalid(
                "outline or center cannot be represented accurately at this page position",
            ));
        }
    }
    let distance = |a: usize, b: usize| {
        (f64::from(points[a].0) - f64::from(points[b].0))
            .hypot(f64::from(points[a].1) - f64::from(points[b].1))
    };
    if !close(distance(0, 64), diameter_x, RELATIVE_TOLERANCE)
        || !close(distance(32, 96), diameter_y, RELATIVE_TOLERANCE)
    {
        return Err(invalid(
            "diameters cannot be represented accurately at this page position",
        ));
    }
    // Topology validation retains the ordinary measurement's minimum edge and vertex bounds.
    let area = crate::measurement_area::ring_area(points)?;
    if !close(
        area,
        sampled_area(diameter_x, diameter_y),
        AREA_RELATIVE_TOLERANCE,
    ) {
        return Err(invalid(
            "polygon area cannot be represented accurately at this page position",
        ));
    }
    Ok(())
}

/// Resolve physical diameters with the same isotropic calibration arithmetic as rectangles.
/// The live-document constructor additionally checks source provenance and scale selection.
pub fn dimensioned_ellipse_point_diameters(
    diameter_x: f64,
    diameter_y: f64,
    unit: MeasurementUnit,
    calibration: &PageCalibration,
    page_size: (f32, f32),
) -> Result<(f64, f64), EngineError> {
    dimensioned_rectangle_point_dimensions(diameter_x, diameter_y, unit, calibration, page_size)
        .map_err(|error| match error {
            EngineError::Other(message) => invalid(
                message
                    .strip_prefix("dimensioned rectangle: ")
                    .unwrap_or(&message),
            ),
            error => error,
        })
}

/// Counterclockwise samples, rotated clockwise about the center. Samples 0/32/64/96 retain
/// the exact local axis directions before f32 storage. There is no clipping or adaptive
/// resampling: tiny, collapsed, out-of-page or imprecise geometry is refused.
pub fn dimensioned_ellipse_points(
    center: (f32, f32),
    diameter_x_points: f64,
    diameter_y_points: f64,
    rotation_degrees: f64,
    page_size: (f32, f32),
) -> Result<[(f32, f32); DIMENSIONED_ELLIPSE_SEGMENTS], EngineError> {
    let degrees = rotation(rotation_degrees)?;
    if !center.0.is_finite()
        || !center.1.is_finite()
        || !diameter_x_points.is_finite()
        || !diameter_y_points.is_finite()
        || diameter_x_points < 0.01
        || diameter_y_points < 0.01
        || !page_size.0.is_finite()
        || !page_size.1.is_finite()
        || page_size.0 <= 0.0
        || page_size.1 <= 0.0
    {
        return Err(invalid(
            "enter positive finite diameters and a finite in-page center",
        ));
    }
    let (sin, cos) = sin_cos(degrees);
    let mut points = [(0.0, 0.0); DIMENSIONED_ELLIPSE_SEGMENTS];
    for (index, point) in points.iter_mut().enumerate() {
        let (s, c) = sample(index);
        let local_x = diameter_x_points * 0.5 * c;
        let local_y = diameter_y_points * 0.5 * s;
        let x = f64::from(center.0) + local_x * cos + local_y * sin;
        let y = f64::from(center.1) - local_x * sin + local_y * cos;
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x > f64::from(page_size.0)
            || y > f64::from(page_size.1)
        {
            return Err(invalid("the polygon extends outside the displayed page"));
        }
        *point = (x as f32, y as f32);
    }
    check_points(
        &points,
        center,
        diameter_x_points,
        diameter_y_points,
        degrees,
    )?;
    Ok(points)
}

impl Document<'_> {
    /// Resolve a current explicit reference and construct a 128-vertex area without mutation.
    /// Quantity, preview, appearance and saved metadata all use the actual polygon vertices.
    pub fn dimensioned_ellipse(
        &self,
        page: usize,
        request: &DimensionedEllipse,
    ) -> Result<Measurement, EngineError> {
        if request.precision > 4 {
            return Err(invalid("choose 0 to 4 decimals"));
        }
        // The shared resolver checks annotation permission, the 128 MiB source bound,
        // page units, page/region provenance and the captured local reference.
        let calibration =
            self.measurement_reference_for_geometry(page, &request.scale, &[request.center])?;
        let geo = self.page_geometry(page)?;
        let (diameter_x, diameter_y) = dimensioned_ellipse_point_diameters(
            request.diameter_x,
            request.diameter_y,
            request.unit,
            &calibration,
            geo.display_size(),
        )?;
        let degrees = rotation(request.rotation_degrees)?;
        let points = dimensioned_ellipse_points(
            request.center,
            diameter_x,
            diameter_y,
            degrees,
            geo.display_size(),
        )?;
        // As for rectangles, construct the coordinates that the PDF will actually retain,
        // then validate their dimensions. Comparing each short chord before this conversion
        // would reject ordinary f32 rounding even when the entire stored outline is accurate.
        let roundtrip = |(x, y)| {
            let (x, y) = geo.to_user(x, y);
            geo.to_display(x, y)
        };
        let stored = Measurement {
            kind: MeasurementKind::Area,
            points: points.into_iter().map(roundtrip).collect(),
            holes: Vec::new(),
            depth: None,
            calibration: calibration.points.map(roundtrip),
            known_distance: calibration.known_distance * calibration.unit.metres()
                / request.unit.metres(),
            unit: request.unit,
            precision: request.precision,
        };
        check_points(
            &stored.points,
            request.center,
            diameter_x,
            diameter_y,
            degrees,
        )?;
        let reference = (f64::from(stored.calibration[1].0) - f64::from(stored.calibration[0].0))
            .hypot(f64::from(stored.calibration[1].1) - f64::from(stored.calibration[0].1));
        let units_per_point = stored.known_distance / reference;
        let physical_diameter = |a: usize, b: usize| {
            (f64::from(stored.points[a].0) - f64::from(stored.points[b].0))
                .hypot(f64::from(stored.points[a].1) - f64::from(stored.points[b].1))
                * units_per_point
        };
        if !close(
            physical_diameter(0, 64),
            request.diameter_x,
            RELATIVE_TOLERANCE,
        ) || !close(
            physical_diameter(32, 96),
            request.diameter_y,
            RELATIVE_TOLERANCE,
        ) || !close(
            stored.value()?,
            sampled_area(request.diameter_x, request.diameter_y),
            AREA_RELATIVE_TOLERANCE,
        ) {
            return Err(invalid(
                "physical diameters or polygon area lose precision when stored",
            ));
        }
        // Revalidate the complete saved ring. Automatic selection cannot fall back to
        // another reference when the center and outline occupy different scale regions.
        let complete =
            self.measurement_reference_for_geometry(page, &request.scale, &stored.points)?;
        if complete != calibration {
            return Err(invalid("scale selection changed across the polygon"));
        }
        annotation_components(&geo, &stored, None)?;
        Ok(stored)
    }

    /// Add the resolved polygon as one ordinary, source-qualified area annotation.
    pub fn add_dimensioned_ellipse(
        &mut self,
        page: usize,
        request: &DimensionedEllipse,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let measurement = self.dimensioned_ellipse(page, request)?;
        self.add_measurement_annotation(page, &measurement, author)
    }
}
