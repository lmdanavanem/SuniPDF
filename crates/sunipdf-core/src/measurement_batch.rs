//! One preview and one atomic commit for explicit multi-page recalibration.

use crate::annot_write::{load_annot, set_ap, store_annot, touch};
use crate::measurement::{
    annotation_components, bounded_text, check_editable_local, MeasurementKind, MeasurementRecord,
    MeasurementUnit, PageCalibration,
};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct RecalibrationRequest {
    pub page: usize,
    pub calibration: PageCalibration,
    pub update_page_default: bool,
    /// Exact supported records explicitly selected by the user; an empty list never means all.
    pub measurements: Vec<MeasurementRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecalibrationChange {
    pub page: usize,
    pub index: usize,
    pub before: f64,
    pub after: f64,
    pub unit: MeasurementUnit,
    pub kind: MeasurementKind,
}

#[derive(Clone)]
pub struct RecalibrationPlan {
    source: Arc<Vec<u8>>,
    requests: Vec<RecalibrationRequest>,
    changes: Vec<RecalibrationChange>,
}

impl std::fmt::Debug for RecalibrationPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecalibrationPlan")
            .field("pages", &self.page_count())
            .field("changes", &self.changes)
            .finish_non_exhaustive()
    }
}
impl RecalibrationPlan {
    pub fn changes(&self) -> &[RecalibrationChange] {
        &self.changes
    }
    pub fn page_count(&self) -> usize {
        self.requests.len()
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("batch recalibration: {reason}"))
}

fn identities(pdf: &lopdf::Document) -> Result<HashMap<String, usize>, EngineError> {
    let mut counts = HashMap::new();
    let mut visited = 0usize;
    for page in pdf.get_pages().values() {
        let page = pdf.get_dictionary(*page).map_err(lopdf_err)?;
        let Ok(raw) = page.get(b"Annots") else {
            continue;
        };
        let entries = pdf
            .dereference(raw)
            .and_then(|(_, o)| o.as_array())
            .map_err(lopdf_err)?;
        visited = visited.saturating_add(entries.len());
        if visited > 100_000 {
            return Err(invalid("annotation identity scan exceeds 100000 records"));
        }
        for raw in entries {
            let dict = pdf
                .dereference(raw)
                .and_then(|(_, o)| o.as_dict())
                .map_err(lopdf_err)?;
            if let Some(id) = bounded_text(pdf, dict, b"NM", 256)? {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
    }
    Ok(counts)
}

fn replacement(
    record: &MeasurementRecord,
    calibration: &PageCalibration,
) -> Result<crate::measurement::Measurement, EngineError> {
    if record.measurement.kind == MeasurementKind::Angle {
        return Err(invalid(
            "angles have no length scale; exclude them from recalibration",
        ));
    }
    let mut m = record.measurement.clone();
    m.calibration = calibration.points;
    // Preserve each annotation's chosen display unit, geometry and precision.
    m.known_distance = calibration.known_distance * calibration.unit.metres() / m.unit.metres();
    m.value()?;
    Ok(m)
}

impl Document<'_> {
    fn recalibration_permission(
        &self,
        requests: &[RecalibrationRequest],
    ) -> Result<(), EngineError> {
        if self.signature_count() != 0 {
            return Err(invalid("signed PDFs cannot be recalibrated"));
        }
        if requests.iter().any(|r| r.update_page_default) && !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        if requests.iter().any(|r| !r.measurements.is_empty()) {
            self.require_annotation_permission()?;
        }
        Ok(())
    }

    /// Validates every page and selected record before returning values for confirmation.
    /// No mutation occurs. The returned opaque plan is tied to this exact document revision.
    pub fn preview_recalibration(
        &self,
        requests: &[RecalibrationRequest],
    ) -> Result<RecalibrationPlan, EngineError> {
        if requests.is_empty()
            || requests.len() > 512
            || requests.iter().map(|r| r.measurements.len()).sum::<usize>() > 4096
        {
            return Err(invalid(
                "select 1 to 512 pages and at most 4096 measurements",
            ));
        }
        self.recalibration_permission(requests)?;
        let source = self.snapshot()?;
        if source.len() > 128 * 1024 * 1024 {
            return Err(invalid("document exceeds the 128 MiB editing limit"));
        }
        let mut pdf = security::load(&source, self.password())?;
        let pages = pdf.get_pages();
        let identities = identities(&pdf)?;
        let mut selected = HashSet::new();
        let mut changes = Vec::new();
        for request in requests {
            if !selected.insert(request.page)
                || (!request.update_page_default && request.measurements.is_empty())
            {
                return Err(invalid("duplicate or empty page selection"));
            }
            let geo = self.page_geometry(request.page)?;
            request.calibration.validate_on_page(&geo)?;
            let page = *pages
                .get(&(request.page as u32 + 1))
                .ok_or(EngineError::PageOutOfRange(request.page))?;
            crate::page_calibration::check_user_unit(
                &pdf,
                pdf.get_dictionary(page).map_err(lopdf_err)?,
            )?;
            let records = self.measurement_records_from_pdf(request.page, &pdf)?;
            let mut indices = HashSet::new();
            for expected in &request.measurements {
                if !indices.insert(expected.index)
                    || records.iter().find(|r| r.index == expected.index) != Some(expected)
                {
                    return Err(invalid(
                        "measurement changed or selected twice; refresh the preview",
                    ));
                }
                check_editable_local(&pdf, page, expected)?;
                if expected
                    .annotation_id
                    .as_ref()
                    .and_then(|id| identities.get(id))
                    != Some(&1)
                {
                    return Err(invalid("measurement identity is shared or ambiguous"));
                }
                let next = replacement(expected, &request.calibration)?;
                annotation_components(&geo, &next, None)?;
                changes.push(RecalibrationChange {
                    page: request.page,
                    index: expected.index,
                    before: expected.measurement.value()?,
                    after: next.value()?,
                    unit: next.unit,
                    kind: next.kind,
                });
            }
            if request.update_page_default {
                // Exercise the exact writer on the private parsed snapshot as part of preflight.
                crate::page_calibration::set_in_pdf(
                    &mut pdf,
                    page,
                    &geo,
                    Some(&request.calibration),
                )?;
            }
        }
        Ok(RecalibrationPlan {
            source: Arc::new(source),
            requests: requests.to_vec(),
            changes,
        })
    }

    /// Applies a previously confirmed plan in one document replacement. Callers can use their
    /// normal snapshot history to undo the entire operation with one Undo action.
    pub fn apply_recalibration(&mut self, plan: &RecalibrationPlan) -> Result<(), EngineError> {
        self.recalibration_permission(&plan.requests)?;
        let source = self.snapshot()?;
        if !crate::annot_rich::same_document(&plan.source, &source, self.password())? {
            return Err(invalid("document changed; refresh the preview"));
        }
        let mut pdf = security::load(&source, self.password())?;
        let pages = pdf.get_pages();
        // Complete validation before replacing the live PDF, including unsupported metadata,
        // shared annotation identities, page bounds and security permissions.
        for request in &plan.requests {
            let geo = self.page_geometry(request.page)?;
            let page = *pages
                .get(&(request.page as u32 + 1))
                .ok_or(EngineError::PageOutOfRange(request.page))?;
            for expected in &request.measurements {
                check_editable_local(&pdf, page, expected)?;
                let next = replacement(expected, &request.calibration)?;
                let (components, appearance) = annotation_components(&geo, &next, None)?;
                let (slot, mut annotation) = load_annot(&pdf, page, expected.index)?;
                for key in [b"Rect".as_slice(), b"Contents", b"SuniPDFMeasurement"] {
                    annotation.set(key, components.get(key).map_err(lopdf_err)?.clone());
                }
                annotation.remove(b"AP");
                set_ap(&mut pdf, &mut annotation, appearance);
                touch(&mut annotation);
                store_annot(&mut pdf, page, slot, annotation)?;
            }
            if request.update_page_default {
                crate::page_calibration::set_in_pdf(
                    &mut pdf,
                    page,
                    &geo,
                    Some(&request.calibration),
                )?;
            }
        }
        self.restore(security::save(pdf)?)
    }
}
