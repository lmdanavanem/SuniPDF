//! Named, content-bound measurement regions with deliberately explicit overlap selection.

use crate::measurement::PageCalibration;
use crate::page_calibration::{check_user_unit, content_digest};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PageGeometry, PointRect};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

const KEY: &[u8] = b"SuniPDFMeasurementViewports";
const MAX_VIEWPORTS: usize = 64;
const MAX_RECORD: usize = 128 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementViewport {
    /// Stable, unique identifier on this physical page; not the editable display name.
    pub id: String,
    pub name: String,
    /// Displayed-page coordinates, bottom-left origin.
    pub bounds: PointRect,
    pub calibration: PageCalibration,
}

/// Exact supported page metadata captured for an explicit reset, including stale scale records.
#[derive(Clone)]
pub struct MeasurementViewportResetTarget {
    page: usize,
    source: Arc<Vec<u8>>,
}
impl std::fmt::Debug for MeasurementViewportResetTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeasurementViewportResetTarget")
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}
impl MeasurementViewportResetTarget {
    pub fn page(&self) -> usize {
        self.page
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("measurement viewport: {reason}"))
}

fn contains(bounds: PointRect, (x, y): (f32, f32)) -> bool {
    x.is_finite()
        && y.is_finite()
        && x >= bounds.left
        && x <= bounds.right
        && y >= bounds.bottom
        && y <= bounds.top
}

fn segment_touches(bounds: PointRect, a: (f32, f32), b: (f32, f32)) -> bool {
    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    for (start, end, min, max) in [
        (a.0, b.0, bounds.left, bounds.right),
        (a.1, b.1, bounds.bottom, bounds.top),
    ] {
        let start = f64::from(start);
        let delta = f64::from(end) - start;
        if delta == 0.0 {
            if start < f64::from(min) || start > f64::from(max) {
                return false;
            }
        } else {
            let t1 = (f64::from(min) - start) / delta;
            let t2 = (f64::from(max) - start) / delta;
            low = low.max(t1.min(t2));
            high = high.min(t1.max(t2));
            if low > high {
                return false;
            }
        }
    }
    true
}

fn polygon_contains(points: &[(f32, f32)], point: (f32, f32)) -> bool {
    let mut inside = false;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
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

/// Select a region only when all geometry is inside it. A selected ID is always checked.
/// Without an ID, more than one matching region is an error: never guess an overlap scale.
/// Geometry touching a region without fitting completely also requires an explicit choice.
pub fn select_viewport<'a>(
    viewports: &'a [MeasurementViewport],
    points: &[(f32, f32)],
    selected_id: Option<&str>,
) -> Result<Option<&'a MeasurementViewport>, EngineError> {
    if points.is_empty()
        || points.len() > 128
        || viewports.len() > MAX_VIEWPORTS
        || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err(invalid("select finite measurement points"));
    }
    if let Some(id) = selected_id {
        let viewport = viewports
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| invalid("selected region no longer exists"))?;
        if !points.iter().all(|&p| contains(viewport.bounds, p)) {
            return Err(invalid("measurement crosses the selected region boundary"));
        }
        return Ok(Some(viewport));
    }
    let mut matching = viewports
        .iter()
        .filter(|v| points.iter().all(|&p| contains(v.bounds, p)));
    let first = matching.next();
    if matching.next().is_some() {
        return Err(invalid("overlapping regions require an explicit selection"));
    }
    let touches = |v: &MeasurementViewport| {
        points.iter().any(|&p| contains(v.bounds, p))
            || points
                .windows(2)
                .any(|p| segment_touches(v.bounds, p[0], p[1]))
            // Three or more points may be a closed area/perimeter. Conservative refusal also
            // covers a region wholly inside the area, with no measured vertex in the region.
            || (points.len()>=3 && (segment_touches(v.bounds,points[points.len()-1],points[0])
                || [(v.bounds.left,v.bounds.bottom),(v.bounds.left,v.bounds.top),(v.bounds.right,v.bounds.bottom),(v.bounds.right,v.bounds.top)].iter().any(|&p|polygon_contains(points,p))))
    };
    if viewports
        .iter()
        .any(|v| first.is_none_or(|selected| selected.id != v.id) && touches(v))
    {
        return Err(invalid(
            "measurement crosses a region boundary; choose a region or the page scale explicitly",
        ));
    }
    Ok(first)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredViewport {
    id: String,
    name: String,
    bounds: [f32; 4],
    points: [(f32, f32); 2],
    known_distance: f64,
    unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredViewports {
    version: u32,
    coordinate_system: String,
    page_bounds: [f32; 4],
    content_sha256: String,
    regions: Vec<StoredViewport>,
}

fn array(b: PointRect) -> [f32; 4] {
    [b.left, b.bottom, b.right, b.top]
}
fn rect(b: [f32; 4]) -> PointRect {
    PointRect {
        left: b[0],
        bottom: b[1],
        right: b[2],
        top: b[3],
    }
}

fn validate(viewports: &[MeasurementViewport], geo: &PageGeometry) -> Result<(), EngineError> {
    if viewports.len() > MAX_VIEWPORTS {
        return Err(invalid("at most 64 regions per page"));
    }
    let (w, h) = geo.display_size();
    let mut ids = HashSet::new();
    for v in viewports {
        if v.id.is_empty()
            || v.id.len() > 64
            || !v
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            || !ids.insert(&v.id)
            || v.name.trim().is_empty()
            || v.name.len() > 256
            || v.name.chars().any(char::is_control)
            || array(v.bounds).iter().any(|x| !x.is_finite())
            || v.bounds.left < 0.0
            || v.bounds.bottom < 0.0
            || v.bounds.right > w
            || v.bounds.top > h
            || v.bounds.left >= v.bounds.right
            || v.bounds.bottom >= v.bounds.top
        {
            return Err(invalid("invalid name, identity or in-page bounds"));
        }
        v.calibration.validate_on_page(geo)?;
        if !v.calibration.points.iter().all(|&p| contains(v.bounds, p)) {
            return Err(invalid("calibration reference must be inside its region"));
        }
    }
    Ok(())
}

fn read_stored(page: &Dictionary) -> Result<Option<StoredViewports>, EngineError> {
    let Ok(raw) = page.get(KEY) else {
        return Ok(None);
    };
    let raw = raw
        .as_str()
        .map_err(|_| invalid("unsupported record representation"))?;
    if raw.len() > MAX_RECORD {
        return Err(invalid("record exceeds size limit"));
    }
    let record: StoredViewports =
        serde_json::from_slice(raw).map_err(|_| invalid("malformed or unsupported record"))?;
    if record.version != 1
        || record.coordinate_system != "pdf-user-space"
        || record.regions.len() > MAX_VIEWPORTS
        || record
            .page_bounds
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 1e8)
        || record.page_bounds[0] >= record.page_bounds[2]
        || record.page_bounds[1] >= record.page_bounds[3]
        || record.content_sha256.len() != 64
        || !record
            .content_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("unsupported or malformed region record"));
    }
    let original = PageGeometry {
        bbox: rect(record.page_bounds),
        rotation: crate::Rotation::R0,
    };
    display_regions(&record, &original)?;
    Ok(Some(record))
}

fn display_regions(
    record: &StoredViewports,
    geo: &PageGeometry,
) -> Result<Vec<MeasurementViewport>, EngineError> {
    let result: Vec<_> = record
        .regions
        .iter()
        .map(|v| {
            if v.bounds
                .iter()
                .chain(v.points.iter().flat_map(|p| [&p.0, &p.1]))
                .any(|x| !x.is_finite())
                || v.bounds[0] >= v.bounds[2]
                || v.bounds[1] >= v.bounds[3]
            {
                return Err(invalid("invalid stored region geometry"));
            }
            Ok(MeasurementViewport {
                id: v.id.clone(),
                name: v.name.clone(),
                bounds: geo.rect_to_display(rect(v.bounds)),
                calibration: PageCalibration {
                    points: v.points.map(|(x, y)| geo.to_display(x, y)),
                    known_distance: v.known_distance,
                    unit: crate::measurement::MeasurementUnit::from_id(&v.unit)
                        .ok_or_else(|| invalid("unknown unit"))?,
                },
            })
        })
        .collect::<Result<_, EngineError>>()?;
    validate(&result, geo)?;
    Ok(result)
}

fn decode(
    pdf: &lopdf::Document,
    page: &Dictionary,
    geo: &PageGeometry,
) -> Result<Vec<MeasurementViewport>, EngineError> {
    let Some(record) = read_stored(page)? else {
        return Ok(Vec::new());
    };
    check_user_unit(pdf, page)?;
    if record
        .page_bounds
        .iter()
        .zip(array(geo.bbox))
        .any(|(a, b)| (a - b).abs() > 0.001)
        || record.content_sha256 != content_digest(pdf, page)?
    {
        return Err(invalid(
            "page geometry/content changed; explicitly clear supported regions and recalibrate",
        ));
    }
    display_regions(&record, geo)
}

impl Document<'_> {
    /// Captures supported metadata for explicit removal even when its content/bounds guard is
    /// stale. Unknown versions or malformed records cannot obtain a reset target.
    pub fn measurement_viewport_reset_target(
        &self,
        page_index: usize,
    ) -> Result<MeasurementViewportResetTarget, EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(invalid("signed PDFs cannot change regions"));
        }
        self.page_geometry(page_index)?;
        let source = self.snapshot()?;
        if source.len() > 128 * 1024 * 1024 {
            return Err(invalid("document exceeds the 128 MiB editing limit"));
        }
        let pdf = security::load(&source, self.password())?;
        let page = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        if read_stored(pdf.get_dictionary(page).map_err(lopdf_err)?)?.is_none() {
            return Err(invalid("no supported regions to clear"));
        }
        Ok(MeasurementViewportResetTarget {
            page: page_index,
            source: Arc::new(source),
        })
    }

    /// One atomic, undoable removal of the captured supported region metadata. Annotation
    /// geometry/scales remain untouched; a later document edit invalidates the reset target.
    pub fn clear_measurement_viewports(
        &mut self,
        target: &MeasurementViewportResetTarget,
    ) -> Result<(), EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(invalid("signed PDFs cannot change regions"));
        }
        if !crate::annot_rich::same_document(&target.source, &self.snapshot()?, self.password())? {
            return Err(invalid("document changed; refresh before clearing regions"));
        }
        self.edit_pdf(target.page, |pdf, page, _| {
            if read_stored(pdf.get_dictionary(page).map_err(lopdf_err)?)?.is_none() {
                return Err(invalid("no supported regions to clear"));
            }
            pdf.get_dictionary_mut(page).map_err(lopdf_err)?.remove(KEY);
            Ok(())
        })
    }

    pub fn measurement_viewports(
        &self,
        page_index: usize,
    ) -> Result<Vec<MeasurementViewport>, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let pdf = security::load(&self.snapshot()?, self.password())?;
        let page = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        decode(&pdf, pdf.get_dictionary(page).map_err(lopdf_err)?, &geo)
    }

    /// Replaces only this page's supported, unchanged region set. Existing measurement
    /// annotations retain their own scales. Unknown/stale metadata and signed PDFs are refused.
    pub fn set_measurement_viewports(
        &mut self,
        page_index: usize,
        expected: &[MeasurementViewport],
        replacement: &[MeasurementViewport],
    ) -> Result<(), EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(invalid("signed PDFs cannot be recalibrated"));
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            validate(replacement, geo)?;
            let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
            if decode(pdf, dict, geo)? != expected {
                return Err(invalid("regions changed; refresh before editing"));
            }
            check_user_unit(pdf, dict)?;
            let stored = StoredViewports {
                version: 1,
                coordinate_system: "pdf-user-space".into(),
                page_bounds: array(geo.bbox),
                content_sha256: content_digest(pdf, dict)?,
                regions: replacement
                    .iter()
                    .map(|v| StoredViewport {
                        id: v.id.clone(),
                        name: v.name.clone(),
                        bounds: array(geo.rect_to_user(v.bounds)),
                        points: v.calibration.points.map(|(x, y)| geo.to_user(x, y)),
                        known_distance: v.calibration.known_distance,
                        unit: v.calibration.unit.id().into(),
                    })
                    .collect(),
            };
            let data =
                serde_json::to_vec(&stored).map_err(|_| invalid("could not encode regions"))?;
            if data.len() > MAX_RECORD {
                return Err(invalid("record exceeds size limit"));
            }
            let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
            if replacement.is_empty() {
                dict.remove(KEY);
            } else {
                dict.set(KEY, Object::string_literal(data));
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::MeasurementUnit;
    fn region(id: &str, left: f32, right: f32) -> MeasurementViewport {
        MeasurementViewport {
            id: id.into(),
            name: id.into(),
            bounds: PointRect {
                left,
                bottom: 0.0,
                right,
                top: 100.0,
            },
            calibration: PageCalibration {
                points: [(left + 1.0, 1.0), (right - 1.0, 1.0)],
                known_distance: 1.0,
                unit: MeasurementUnit::Metres,
            },
        }
    }
    #[test]
    fn overlap_requires_choice_and_choice_rejects_cross_boundary() {
        let v = vec![region("a", 0.0, 60.0), region("b", 40.0, 100.0)];
        assert!(select_viewport(&v, &[(45.0, 5.0), (55.0, 5.0)], None).is_err());
        assert_eq!(
            select_viewport(&v, &[(45.0, 5.0), (55.0, 5.0)], Some("b"))
                .unwrap()
                .unwrap()
                .id,
            "b"
        );
        assert!(select_viewport(&v, &[(5.0, 5.0), (95.0, 5.0)], Some("b")).is_err());
        assert!(select_viewport(&v, &[(5.0, 5.0), (95.0, 5.0)], None).is_err());
        assert!(select_viewport(&v, &[], None).is_err());
        assert!(select_viewport(&v, &[(f32::NAN, 0.0)], None).is_err());
    }

    #[test]
    fn closed_geometry_never_silently_falls_back_when_it_encloses_or_crosses_a_region() {
        let mut v = region("detail", 40.0, 60.0);
        v.bounds.bottom = 40.0;
        v.bounds.top = 60.0;
        let regions = vec![v];
        assert!(select_viewport(
            &regions,
            &[
                (-10.0, -10.0),
                (110.0, -10.0),
                (110.0, 110.0),
                (-10.0, 110.0)
            ],
            None
        )
        .is_err());
        assert!(select_viewport(
            &regions,
            &[(-10.0, -10.0), (110.0, -10.0), (110.0, 110.0)],
            None
        )
        .is_err());
        assert!(select_viewport(
            &regions,
            &[(-10.0, -10.0), (110.0, -10.0), (110.0, 110.0)],
            Some("detail")
        )
        .is_err());
    }
}
