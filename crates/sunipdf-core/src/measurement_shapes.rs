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

/// The anchor is the bottom-left corner in displayed-page coordinates. The rectangle extends
/// right and up. Width and height are real distances in `unit`, not PDF points.
#[derive(Debug, Clone)]
pub struct DimensionedRectangle {
    pub anchor: (f32, f32),
    pub width: f64,
    pub height: f64,
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

fn rectangle(
    request: &DimensionedRectangle,
    calibration: &PageCalibration,
    geo: &PageGeometry,
) -> Result<Measurement, EngineError> {
    calibration.validate_on_page(geo)?;
    if !request.width.is_finite()
        || !request.height.is_finite()
        || request.width <= 0.0
        || request.height <= 0.0
        || !request.anchor.0.is_finite()
        || !request.anchor.1.is_finite()
        || request.precision > 4
    {
        return Err(invalid(
            "enter positive finite dimensions, a finite anchor and 0 to 4 decimals",
        ));
    }
    let reference = (f64::from(calibration.points[1].0) - f64::from(calibration.points[0].0))
        .hypot(f64::from(calibration.points[1].1) - f64::from(calibration.points[0].1));
    let known_distance =
        calibration.known_distance * calibration.unit.metres() / request.unit.metres();
    let units_per_point = known_distance / reference;
    let width = request.width / units_per_point;
    let height = request.height / units_per_point;
    let right = f64::from(request.anchor.0) + width;
    let top = f64::from(request.anchor.1) + height;
    let (page_width, page_height) = geo.display_size();
    if !width.is_finite()
        || !height.is_finite()
        || width < 0.01
        || height < 0.01
        || request.anchor.0 < 0.0
        || request.anchor.1 < 0.0
        || right > f64::from(page_width)
        || top > f64::from(page_height)
    {
        return Err(invalid(
            "rectangle is too small or extends outside the displayed page",
        ));
    }
    let points = vec![
        request.anchor,
        (right as f32, request.anchor.1),
        (right as f32, top as f32),
        (request.anchor.0, top as f32),
    ];
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
        (&points, units_per_point),
        (&roundtrip, stored_units_per_point),
    ] {
        let actual_width =
            (f64::from(represented[1].0) - f64::from(represented[0].0)) * units_per_point;
        let actual_height =
            (f64::from(represented[3].1) - f64::from(represented[0].1)) * units_per_point;
        if !close_side(actual_width, request.width) || !close_side(actual_height, request.height) {
            return Err(invalid(
                "dimensions cannot be represented accurately at this page position",
            ));
        }
    }
    if points.iter().zip(&roundtrip).any(|(a, b)| {
        (f64::from(a.0) - f64::from(b.0)).abs() > width * SIDE_RELATIVE_TOLERANCE
            || (f64::from(a.1) - f64::from(b.1)).abs() > height * SIDE_RELATIVE_TOLERANCE
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
