//! Page defaults are independent of the immutable scale in each saved measurement annotation.

use crate::measurement::{Measurement, MeasurementKind, MeasurementUnit};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PageGeometry};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KEY: &[u8] = b"SuniPDFPageCalibration";
const MAX_RECORD: usize = 4096;
const MAX_STREAM: usize = 8 * 1024 * 1024;
const MAX_CONTENT: usize = 32 * 1024 * 1024;

/// A two-point scale for future measurements on one page. Coordinates use the displayed page,
/// with its bottom-left origin, exactly like [`Measurement`]. Saving the PDF persists this default.
#[derive(Debug, Clone, PartialEq)]
pub struct PageCalibration {
    pub points: [(f32, f32); 2],
    pub known_distance: f64,
    pub unit: MeasurementUnit,
}

impl PageCalibration {
    pub fn validate(&self) -> Result<(), EngineError> {
        Measurement {
            kind: MeasurementKind::Length,
            points: self.points.to_vec(),
            calibration: self.points,
            known_distance: self.known_distance,
            unit: self.unit,
            precision: 2,
        }
        .value()
        .map(|_| ())
    }

    pub(crate) fn validate_on_page(&self, geo: &PageGeometry) -> Result<(), EngineError> {
        self.validate()?;
        let (w, h) = geo.display_size();
        if self
            .points
            .iter()
            .any(|&(x, y)| x < 0.0 || y < 0.0 || x > w || y > h)
        {
            return Err(invalid("reference points are outside the displayed page"));
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredCalibration {
    version: u32,
    coordinate_system: String,
    points: [(f32, f32); 2],
    known_distance: f64,
    unit: String,
    bbox: [f32; 4],
    user_unit: f32,
    content_sha256: String,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("page calibration: {reason}"))
}

fn bounds(geo: &PageGeometry) -> [f32; 4] {
    [geo.bbox.left, geo.bbox.bottom, geo.bbox.right, geo.bbox.top]
}

fn read_record(dict: &Dictionary) -> Result<Option<StoredCalibration>, EngineError> {
    let Ok(raw) = dict.get(KEY) else {
        return Ok(None);
    };
    // A private record is a direct, bounded string. Unknown representations are never overwritten.
    let raw = raw
        .as_str()
        .map_err(|_| invalid("unsupported record representation"))?;
    if raw.len() > MAX_RECORD {
        return Err(invalid("record exceeds the size limit"));
    }
    let record: StoredCalibration =
        serde_json::from_slice(raw).map_err(|_| invalid("malformed or unsupported record"))?;
    if record.version != 1 || record.coordinate_system != "pdf-user-space" {
        return Err(invalid("unsupported record version or coordinate system"));
    }
    if record
        .points
        .iter()
        .flat_map(|&(x, y)| [x, y])
        .chain(record.bbox)
        .any(|v| !v.is_finite() || v.abs() > 1e8)
        || record.bbox[0] >= record.bbox[2]
        || record.bbox[1] >= record.bbox[3]
        || record.points.iter().any(|&(x, y)| {
            x < record.bbox[0] || y < record.bbox[1] || x > record.bbox[2] || y > record.bbox[3]
        })
        || record.user_unit != 1.0
        || record.content_sha256.len() != 64
        || !record
            .content_sha256
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(invalid("invalid geometry or content guard"));
    }
    PageCalibration {
        points: record.points,
        known_distance: record.known_distance,
        unit: MeasurementUnit::from_id(&record.unit).ok_or_else(|| invalid("unsupported unit"))?,
    }
    .validate()?;
    Ok(Some(record))
}

pub(crate) fn check_user_unit(pdf: &lopdf::Document, page: &Dictionary) -> Result<(), EngineError> {
    if let Ok(raw) = page.get(b"UserUnit") {
        let value = pdf.dereference(raw).map_err(lopdf_err)?.1;
        if value.as_float().ok() != Some(1.0) {
            return Err(invalid(
                "non-default UserUnit is unsupported; use a session scale",
            ));
        }
    }
    Ok(())
}

/// Hash decoded content without object IDs, stream filters or annotations. This rejects a stale
/// default after content edits and resizing, while preserving it through rotation and reordering.
pub(crate) fn content_digest(
    pdf: &lopdf::Document,
    page: &Dictionary,
) -> Result<String, EngineError> {
    let mut digest = Sha256::new();
    let mut total = 0usize;
    if let Ok(raw) = page.get(b"Contents") {
        let object = pdf.dereference(raw).map_err(lopdf_err)?.1;
        let streams = match object {
            Object::Array(entries) if entries.len() <= 1024 => entries.as_slice(),
            Object::Stream(_) => std::slice::from_ref(object),
            _ => return Err(invalid("unsupported page content")),
        };
        for raw in streams {
            let stream = pdf
                .dereference(raw)
                .map_err(lopdf_err)?
                .1
                .as_stream()
                .map_err(lopdf_err)?;
            let bytes = if stream.dict.has(b"Filter") {
                stream
                    .decompressed_content_with_limit(MAX_STREAM)
                    .map_err(lopdf_err)?
            } else {
                if stream.content.len() > MAX_STREAM {
                    return Err(invalid("page content exceeds the stream limit"));
                }
                stream.content.clone()
            };
            total = total.saturating_add(bytes.len());
            if total > MAX_CONTENT {
                return Err(invalid("page content exceeds the total limit"));
            }
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

impl Document<'_> {
    /// Reads a saved default for this physical page. Missing metadata is `None`; malformed,
    /// unsupported or stale data is an error rather than an apparently usable scale.
    pub fn page_calibration(
        &self,
        page_index: usize,
    ) -> Result<Option<PageCalibration>, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let pdf = security::load(&self.snapshot()?, self.password())?;
        let page = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
        let Some(record) = read_record(dict)? else {
            return Ok(None);
        };
        check_user_unit(&pdf, dict)?;
        if record
            .bbox
            .iter()
            .zip(bounds(&geo))
            .any(|(a, b)| (a - b).abs() > 0.001)
            || record.content_sha256 != content_digest(&pdf, dict)?
        {
            return Err(invalid(
                "page geometry or content changed; recalibrate or remove the saved scale",
            ));
        }
        let calibration = PageCalibration {
            points: record.points.map(|(x, y)| geo.to_display(x, y)),
            known_distance: record.known_distance,
            unit: MeasurementUnit::from_id(&record.unit)
                .ok_or_else(|| invalid("unsupported unit"))?,
        };
        calibration.validate_on_page(&geo)?;
        Ok(Some(calibration))
    }

    /// Updates only the page default, leaving every saved annotation and its value unchanged.
    /// Requires document modification permission. `None` removes a supported record, including
    /// a stale one. Unknown or malformed records are preserved and return an error.
    pub fn set_page_calibration(
        &mut self,
        page_index: usize,
        calibration: Option<&PageCalibration>,
    ) -> Result<(), EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            set_in_pdf(pdf, page, geo, calibration)
        })
    }
}

pub(crate) fn set_in_pdf(
    pdf: &mut lopdf::Document,
    page: lopdf::ObjectId,
    geo: &PageGeometry,
    calibration: Option<&PageCalibration>,
) -> Result<(), EngineError> {
    let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
    read_record(dict)?;
    let metadata = if let Some(calibration) = calibration {
        calibration.validate_on_page(geo)?;
        check_user_unit(pdf, dict)?;
        let record = StoredCalibration {
            version: 1,
            coordinate_system: "pdf-user-space".into(),
            points: calibration.points.map(|(x, y)| geo.to_user(x, y)),
            known_distance: calibration.known_distance,
            unit: calibration.unit.id().into(),
            bbox: bounds(geo),
            user_unit: 1.0,
            content_sha256: content_digest(pdf, dict)?,
        };
        let text =
            serde_json::to_string(&record).map_err(|_| invalid("could not encode the scale"))?;
        let mut check = Dictionary::new();
        check.set(KEY, Object::string_literal(text.clone()));
        read_record(&check)?;
        Some(text)
    } else {
        None
    };
    let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
    if let Some(text) = metadata {
        dict.set(KEY, Object::string_literal(text));
    } else {
        dict.remove(KEY);
    }
    Ok(())
}
