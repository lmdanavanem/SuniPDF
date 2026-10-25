//! Dimensioned shapes resolved against a current, explicitly selected page reference.

use crate::measurement::{
    annotation_components, page_provenance, select_viewport, Measurement, MeasurementKind,
    MeasurementUnit, PageCalibration,
};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PageGeometry};
use std::sync::Arc;

const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
// Reject dimensions that cannot survive f32 page coordinates within 0.001% per side.
const SIDE_RELATIVE_TOLERANCE: f64 = 1e-5;
// The product of two sides that meet the side tolerance has this relative area bound.
const AREA_RELATIVE_TOLERANCE: f64 =
    2.0 * SIDE_RELATIVE_TOLERANCE + SIDE_RELATIVE_TOLERANCE * SIDE_RELATIVE_TOLERANCE;

/// A temporary reference captured when the user establishes its scale on the live document.
/// It cannot be rebound by editing the dimensions or by passing an old calibration at preview.
#[derive(Clone)]
pub struct LocalCalibrationReference {
    page: usize,
    calibration: PageCalibration,
    source: Arc<Vec<u8>>,
}

impl std::fmt::Debug for LocalCalibrationReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCalibrationReference")
            .field("page", &self.page)
            .field("calibration", &self.calibration)
            .finish_non_exhaustive()
    }
}

impl LocalCalibrationReference {
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn calibration(&self) -> &PageCalibration {
        &self.calibration
    }
}

/// A page scale is an explicit override of regions. Automatic selection never guesses when
/// regions overlap or when any part of the resulting rectangle crosses a region boundary.
#[derive(Debug, Clone)]
pub enum RectangleScale {
    Local(LocalCalibrationReference),
    Page,
    Region(String),
    Automatic,
}

/// At zero rotation the anchor is the bottom-left corner in displayed-page coordinates and
/// the rectangle extends right and up. Rotation is clockwise around that same anchor.
/// Width and height are real distances in `unit`, not PDF points. Saved area vertices retain
/// the orientation; this request does not create a persistent parametric constraint.
#[derive(Debug, Clone)]
pub struct DimensionedRectangle {
    pub anchor: (f32, f32),
    pub width: f64,
    pub height: f64,
    pub rotation_degrees: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
    pub scale: RectangleScale,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("dimensioned rectangle: {reason}"))
}

fn close_side(actual: f64, expected: f64) -> bool {
    actual.is_finite()
        && actual > 0.0
        && (actual - expected).abs() <= expected * SIDE_RELATIVE_TOLERANCE
}

/// Normalize a finite clockwise angle to [0, 360), including arbitrarily large finite inputs.
/// Reduction happens before radians/trigonometry so an extreme angle cannot overflow it.
pub fn normalize_rectangle_rotation(degrees: f64) -> Result<f64, EngineError> {
    if !degrees.is_finite() {
        return Err(invalid("rotation must be a finite angle"));
    }
    let normalized = degrees.rem_euclid(360.0);
    // Floating remainder can round a tiny negative input to the upper endpoint.
    Ok(if normalized == 0.0 || normalized == 360.0 {
        0.0
    } else {
        normalized
    })
}

fn rotation_sin_cos(normalized: f64) -> (f64, f64) {
    match normalized {
        0.0 => (0.0, 1.0),
        90.0 => (1.0, 0.0),
        180.0 => (0.0, -1.0),
        270.0 => (-1.0, 0.0),
        _ => normalized.to_radians().sin_cos(),
    }
}

struct ResolvedDimensions {
    width_points: f64,
    height_points: f64,
    known_distance: f64,
    units_per_point: f64,
}

fn resolved_dimensions(
    width: f64,
    height: f64,
    unit: MeasurementUnit,
    calibration: &PageCalibration,
    page_size: (f32, f32),
) -> Result<ResolvedDimensions, EngineError> {
    calibration.validate()?;
    if !page_size.0.is_finite()
        || !page_size.1.is_finite()
        || page_size.0 <= 0.0
        || page_size.1 <= 0.0
        || calibration
            .points
            .iter()
            .any(|&(x, y)| x < 0.0 || y < 0.0 || x > page_size.0 || y > page_size.1)
    {
        return Err(invalid("reference points are outside the displayed page"));
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(invalid("enter positive finite dimensions"));
    }
    let reference = (f64::from(calibration.points[1].0) - f64::from(calibration.points[0].0))
        .hypot(f64::from(calibration.points[1].1) - f64::from(calibration.points[0].1));
    let known_distance = calibration.known_distance * calibration.unit.metres() / unit.metres();
    let units_per_point = known_distance / reference;
    let width_points = width / units_per_point;
    let height_points = height / units_per_point;
    if !known_distance.is_finite()
        || known_distance <= 0.0
        || !units_per_point.is_finite()
        || units_per_point <= 0.0
        || !width_points.is_finite()
        || !height_points.is_finite()
    {
        return Err(invalid(
            "dimensions cannot be resolved against this reference",
        ));
    }
    Ok(ResolvedDimensions {
        width_points,
        height_points,
        known_distance,
        units_per_point,
    })
}

/// Resolve real dimensions to displayed-page points using the same arithmetic as construction.
/// This validates the supplied calibration's geometry; live document methods additionally
/// validate its provenance and the selected named region before an annotation can be saved.
pub fn dimensioned_rectangle_point_dimensions(
    width: f64,
    height: f64,
    unit: MeasurementUnit,
    calibration: &PageCalibration,
    page_size: (f32, f32),
) -> Result<(f64, f64), EngineError> {
    let dimensions = resolved_dimensions(width, height, unit, calibration, page_size)?;
    Ok((dimensions.width_points, dimensions.height_points))
}

fn checked_rectangle_precision(
    points: &[(f32, f32)],
    units_per_point: f64,
    expected_width: f64,
    expected_height: f64,
) -> Result<(), EngineError> {
    if points.len() != 4 || !units_per_point.is_finite() || units_per_point <= 0.0 {
        return Err(invalid(
            "dimensions cannot be represented accurately at this page position",
        ));
    }
    let mut edges = [(0.0_f64, 0.0_f64); 4];
    for (index, edge) in edges.iter_mut().enumerate() {
        let next = (index + 1) % 4;
        let dx = f64::from(points[next].0) - f64::from(points[index].0);
        let dy = f64::from(points[next].1) - f64::from(points[index].1);
        let length = dx.hypot(dy);
        let expected = if index.is_multiple_of(2) {
            expected_width
        } else {
            expected_height
        };
        if !close_side(length * units_per_point, expected) {
            return Err(invalid(
                "dimensions cannot be represented accurately at this page position",
            ));
        }
        *edge = (dx / length, dy / length);
    }
    for (index, edge) in edges.iter().enumerate() {
        let next = (index + 1) % 4;
        let dot = edge.0 * edges[next].0 + edge.1 * edges[next].1;
        if !dot.is_finite() || dot.abs() > SIDE_RELATIVE_TOLERANCE {
            return Err(invalid(
                "right angles cannot be represented accurately at this page position",
            ));
        }
    }
    // Translate to the anchor before taking cross products; large crop origins must not
    // cause cancellation in an otherwise small rectangle's area calculation.
    let vector = |index: usize| {
        (
            f64::from(points[index].0) - f64::from(points[0].0),
            f64::from(points[index].1) - f64::from(points[0].1),
        )
    };
    let a = vector(1);
    let b = vector(2);
    let c = vector(3);
    let area = ((a.0 * b.1 - a.1 * b.0) + (b.0 * c.1 - b.1 * c.0))
        * 0.5
        * units_per_point
        * units_per_point;
    let expected_area = expected_width * expected_height;
    if !area.is_finite()
        || area <= 0.0
        || !expected_area.is_finite()
        || expected_area <= 0.0
        || (area - expected_area).abs() > expected_area * AREA_RELATIVE_TOLERANCE
    {
        return Err(invalid(
            "area cannot be represented accurately at this page position",
        ));
    }
    Ok(())
}

/// Four clockwise-rotated corners in displayed coordinates (positive Y points upward).
/// The first corner is always the original anchor. Exact cardinal rotations avoid tiny
/// trigonometric residues at page edges. No clipping or silent dimension adjustment occurs.
pub fn dimensioned_rectangle_corners(
    anchor: (f32, f32),
    width_points: f64,
    height_points: f64,
    rotation_degrees: f64,
    page_size: (f32, f32),
) -> Result<[(f32, f32); 4], EngineError> {
    let rotation = normalize_rectangle_rotation(rotation_degrees)?;
    if !anchor.0.is_finite()
        || !anchor.1.is_finite()
        || !width_points.is_finite()
        || !height_points.is_finite()
        || width_points < 0.01
        || height_points < 0.01
        || !page_size.0.is_finite()
        || !page_size.1.is_finite()
        || page_size.0 <= 0.0
        || page_size.1 <= 0.0
    {
        return Err(invalid(
            "enter finite dimensions of at least 0.01 points and a finite anchor",
        ));
    }
    let (sin, cos) = rotation_sin_cos(rotation);
    let width = (width_points * cos, -width_points * sin);
    let height = (height_points * sin, height_points * cos);
    let x = f64::from(anchor.0);
    let y = f64::from(anchor.1);
    let precise = [
        (x, y),
        (x + width.0, y + width.1),
        (x + width.0 + height.0, y + width.1 + height.1),
        (x + height.0, y + height.1),
    ];
    if precise.iter().any(|&(x, y)| {
        !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x > f64::from(page_size.0)
            || y > f64::from(page_size.1)
    }) {
        return Err(invalid(
            "rectangle is too small or extends outside the displayed page",
        ));
    }
    let points = precise.map(|(x, y)| (x as f32, y as f32));
    checked_rectangle_precision(&points, 1.0, width_points, height_points)?;
    Ok(points)
}

fn rectangle(
    request: &DimensionedRectangle,
    calibration: &PageCalibration,
    geo: &PageGeometry,
) -> Result<Measurement, EngineError> {
    if request.precision > 4 {
        return Err(invalid("choose 0 to 4 decimals"));
    }
    calibration.validate_on_page(geo)?;
    let dimensions = resolved_dimensions(
        request.width,
        request.height,
        request.unit,
        calibration,
        geo.display_size(),
    )?;
    let width = dimensions.width_points;
    let height = dimensions.height_points;
    let known_distance = dimensions.known_distance;
    let units_per_point = dimensions.units_per_point;
    let rotation = normalize_rectangle_rotation(request.rotation_degrees)?;
    let points =
        dimensioned_rectangle_corners(request.anchor, width, height, rotation, geo.display_size())?;
    // The PDF stores user-space f32 coordinates. Test that conversion as well as displayed
    // coordinates, especially on pages whose crop origin is far from zero.
    let to_stored_display = |(x, y)| {
        let (x, y) = geo.to_user(x, y);
        geo.to_display(x, y)
    };
    let roundtrip: Vec<_> = points.iter().copied().map(to_stored_display).collect();
    let stored_calibration = calibration.points.map(to_stored_display);
    let stored_reference = (f64::from(stored_calibration[1].0)
        - f64::from(stored_calibration[0].0))
    .hypot(f64::from(stored_calibration[1].1) - f64::from(stored_calibration[0].1));
    let stored_units_per_point = known_distance / stored_reference;
    for (represented, units_per_point) in [
        (points.as_slice(), units_per_point),
        (roundtrip.as_slice(), stored_units_per_point),
    ] {
        checked_rectangle_precision(represented, units_per_point, request.width, request.height)?;
    }
    let (sin, cos) = rotation_sin_cos(rotation);
    if points.iter().zip(&roundtrip).any(|(a, b)| {
        let dx = f64::from(a.0) - f64::from(b.0);
        let dy = f64::from(a.1) - f64::from(b.1);
        (dx * cos - dy * sin).abs() > width * SIDE_RELATIVE_TOLERANCE
            || (dx * sin + dy * cos).abs() > height * SIDE_RELATIVE_TOLERANCE
    }) {
        return Err(invalid(
            "anchor cannot be represented accurately at this page position",
        ));
    }
    let measurement = Measurement {
        kind: MeasurementKind::Area,
        holes: Vec::new(),
        depth: None,
        points: roundtrip,
        calibration: stored_calibration,
        known_distance,
        unit: request.unit,
        precision: request.precision,
    };
    measurement.value()?;
    annotation_components(geo, &measurement, None)?;
    Ok(measurement)
}

impl Document<'_> {
    /// Resolve an existing current reference for an already reviewed polygon. The
    /// outer ring must fit the selected region; automatic selection also rejects
    /// enclosed or crossed regions. Page and Local are explicit overrides.
    pub fn measurement_reference_for_geometry(
        &self,
        page: usize,
        scale: &RectangleScale,
        points: &[(f32, f32)],
    ) -> Result<PageCalibration, EngineError> {
        self.require_annotation_permission()?;
        let geo = self.page_geometry(page)?;
        let (width, height) = geo.display_size();
        if points.is_empty()
            || points.len() > crate::measurement::MAX_RING_VERTICES
            || points.iter().any(|&(x, y)| {
                !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x > width || y > height
            })
        {
            return Err(invalid(
                "reference requires bounded finite in-page geometry",
            ));
        }
        let source = self.snapshot()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(invalid("reference source exceeds 128 MiB"));
        }
        let pdf = security::load(&source, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page))?;
        page_provenance(&pdf, page_id, &geo)?;
        let page_scale = || {
            self.page_calibration(page)?.ok_or_else(|| {
                invalid("no saved page scale; calibrate the page or choose a reference")
            })
        };
        let calibration = match scale {
            RectangleScale::Local(reference) => {
                if reference.page != page
                    || !crate::annot_rich::same_document_for_local_reference(
                        &reference.source,
                        &source,
                        self.password(),
                    )?
                {
                    return Err(invalid(
                        "local reference belongs to another page or document revision; recalibrate",
                    ));
                }
                reference.calibration.clone()
            }
            RectangleScale::Page => page_scale()?,
            RectangleScale::Region(id) => {
                let regions = self.measurement_viewports(page)?;
                select_viewport(&regions, points, Some(id))?
                    .ok_or_else(|| invalid("selected region no longer exists"))?
                    .calibration
                    .clone()
            }
            RectangleScale::Automatic => {
                let regions = self.measurement_viewports(page)?;
                match select_viewport(&regions, points, None)? {
                    Some(region) => region.calibration.clone(),
                    None => page_scale()?,
                }
            }
        };
        calibration.validate_on_page(&geo)?;
        Ok(calibration)
    }

    /// Capture at the moment the user establishes a new local reference, not later at preview.
    /// Annotation-only edits may reuse it; page content, geometry, scale and protection changes
    /// require explicit recalibration. No PDF metadata is written.
    pub fn local_calibration_reference(
        &self,
        page: usize,
        calibration: &PageCalibration,
    ) -> Result<LocalCalibrationReference, EngineError> {
        self.require_annotation_permission()?;
        let geo = self.page_geometry(page)?;
        calibration.validate_on_page(&geo)?;
        let source = self.snapshot()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(invalid(
                "local references are limited to documents up to 128 MiB",
            ));
        }
        let pdf = security::load(&source, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page))?;
        page_provenance(&pdf, page_id, &geo)?;
        Ok(LocalCalibrationReference {
            page,
            calibration: calibration.clone(),
            source: Arc::new(source),
        })
    }

    /// Resolve the selected scale on the live page, validate all four vertices and return the
    /// exact area measurement used by both desktop and CLI. This operation does not mutate.
    pub fn dimensioned_rectangle(
        &self,
        page: usize,
        request: &DimensionedRectangle,
    ) -> Result<Measurement, EngineError> {
        self.require_annotation_permission()?;
        let geo = self.page_geometry(page)?;
        let source = self.snapshot()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(invalid(
                "shape construction is limited to documents up to 128 MiB",
            ));
        }
        let pdf = security::load(&source, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page))?;
        crate::page_calibration::check_user_unit(
            &pdf,
            pdf.get_dictionary(page_id).map_err(lopdf_err)?,
        )?;
        let page_scale = || {
            self.page_calibration(page)?.ok_or_else(|| {
                invalid("no saved page scale; calibrate the page or choose a reference")
            })
        };
        let regions = if matches!(
            request.scale,
            RectangleScale::Region(_) | RectangleScale::Automatic
        ) {
            self.measurement_viewports(page)?
        } else {
            Vec::new()
        };
        let selected_region = match &request.scale {
            RectangleScale::Region(id) => select_viewport(&regions, &[request.anchor], Some(id))?,
            RectangleScale::Automatic => select_viewport(&regions, &[request.anchor], None)?,
            _ => None,
        };
        let calibration = match &request.scale {
            RectangleScale::Local(reference) => {
                if reference.page != page
                    || !crate::annot_rich::same_document_for_local_reference(
                        &reference.source,
                        &source,
                        self.password(),
                    )?
                {
                    return Err(invalid(
                        "local reference belongs to another page or document revision; recalibrate",
                    ));
                }
                reference.calibration.clone()
            }
            RectangleScale::Page => page_scale()?,
            RectangleScale::Region(_) | RectangleScale::Automatic => {
                if let Some(region) = selected_region {
                    region.calibration.clone()
                } else {
                    page_scale()?
                }
            }
        };
        let measurement = rectangle(request, &calibration, &geo)?;
        match &request.scale {
            RectangleScale::Region(id) => {
                select_viewport(&regions, &measurement.points, Some(id))?;
            }
            RectangleScale::Automatic => {
                let final_region = select_viewport(&regions, &measurement.points, None)?;
                if final_region.map(|region| &region.id) != selected_region.map(|region| &region.id)
                {
                    return Err(invalid("scale selection changed across the rectangle"));
                }
            }
            _ => {}
        }
        Ok(measurement)
    }

    /// Create an ordinary four-vertex area measurement with current provenance and the resolved
    /// reference embedded in its metadata. The existing annotation transaction remains atomic.
    pub fn add_dimensioned_rectangle(
        &mut self,
        page: usize,
        request: &DimensionedRectangle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let measurement = self.dimensioned_rectangle(page, request)?;
        self.add_measurement_annotation(page, &measurement, author)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_rotations_keep_exact_page_edges_and_clockwise_corner_order() {
        for (angle, anchor, page, expected) in [
            (
                0.0,
                (0.0, 0.0),
                (100.0, 50.0),
                [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)],
            ),
            (
                90.0,
                (0.0, 100.0),
                (50.0, 100.0),
                [(0.0, 100.0), (0.0, 0.0), (50.0, 0.0), (50.0, 100.0)],
            ),
            (
                180.0,
                (100.0, 50.0),
                (100.0, 50.0),
                [(100.0, 50.0), (0.0, 50.0), (0.0, 0.0), (100.0, 0.0)],
            ),
            (
                270.0,
                (50.0, 0.0),
                (50.0, 100.0),
                [(50.0, 0.0), (50.0, 100.0), (0.0, 100.0), (0.0, 0.0)],
            ),
        ] {
            assert_eq!(
                dimensioned_rectangle_corners(anchor, 100.0, 50.0, angle, page).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn arbitrary_angles_are_normalized_before_trigonometry_without_changing_the_anchor() {
        let expected =
            dimensioned_rectangle_corners((100.0, 100.0), 40.0, 20.0, 45.0, (300.0, 300.0))
                .unwrap();
        assert_eq!(expected[0], (100.0, 100.0));
        assert!(
            expected[1].0 > 100.0 && expected[1].1 < 100.0,
            "positive rotation is clockwise"
        );
        for equivalent in [-315.0, 405.0, 1125.0] {
            assert_eq!(
                dimensioned_rectangle_corners(
                    (100.0, 100.0),
                    40.0,
                    20.0,
                    equivalent,
                    (300.0, 300.0)
                )
                .unwrap(),
                expected
            );
        }
        assert_eq!(normalize_rectangle_rotation(-90.0).unwrap(), 270.0);
        assert_eq!(
            normalize_rectangle_rotation(-720.0).unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        for extreme in [f64::MAX, -f64::MAX, 1e300, -1e300] {
            let normalized = normalize_rectangle_rotation(extreme).unwrap();
            assert!((0.0..360.0).contains(&normalized));
            let corners =
                dimensioned_rectangle_corners((150.0, 150.0), 40.0, 20.0, extreme, (300.0, 300.0))
                    .unwrap();
            assert_eq!(
                corners,
                dimensioned_rectangle_corners(
                    (150.0, 150.0),
                    40.0,
                    20.0,
                    normalized,
                    (300.0, 300.0)
                )
                .unwrap()
            );
        }
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(normalize_rectangle_rotation(invalid).is_err());
            assert!(dimensioned_rectangle_corners(
                (100.0, 100.0),
                40.0,
                20.0,
                invalid,
                (300.0, 300.0)
            )
            .is_err());
        }
    }

    #[test]
    fn every_edge_right_angle_and_signed_area_must_survive_coordinate_rounding() {
        assert!(checked_rectangle_precision(
            &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            1.0,
            100.0,
            100.0
        )
        .is_ok());
        // The two edges formerly checked still have exact lengths; the opposite edge does not.
        assert!(checked_rectangle_precision(
            &[(0.0, 0.0), (100.0, 0.0), (100.002, 100.0), (0.0, 100.0)],
            1.0,
            100.0,
            100.0
        )
        .is_err());
        // All four side lengths satisfy the tolerance, but this is a sheared parallelogram.
        assert!(checked_rectangle_precision(
            &[(0.0, 0.0), (100.0, 0.0), (100.01, 100.0), (0.01, 100.0)],
            1.0,
            100.0,
            100.0
        )
        .is_err());
        assert!(checked_rectangle_precision(
            &[(0.0, 0.0), (100.0, 0.0), (100.0, -100.0), (0.0, -100.0)],
            1.0,
            100.0,
            100.0
        )
        .is_err());
        assert!(
            dimensioned_rectangle_corners((100.0, 100.0), 0.0099, 20.0, 45.0, (300.0, 300.0))
                .is_err()
        );
        assert!(dimensioned_rectangle_corners(
            (1_000_000.0, 1_000_000.0),
            10.0,
            10.0,
            45.0,
            (2_000_000.0, 2_000_000.0)
        )
        .is_err());
        assert!(dimensioned_rectangle_corners(
            (1_000_000.0, 1_000_000.0),
            10.0,
            10.0,
            0.0,
            (2_000_000.0, 2_000_000.0)
        )
        .is_ok());
        assert!(
            dimensioned_rectangle_corners((0.0, 0.0), 40.0, 20.0, 45.0, (300.0, 300.0)).is_err()
        );
    }
}
