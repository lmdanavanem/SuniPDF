//! Calibrated local measurements with an explicit, self-contained per-annotation scale.

use crate::annot_rich::{checked_rect, rich_base};
use crate::annot_write::{
    ap_stream, load_annot, push_annotation, set_ap, store_annot, text_of, touch,
};
use crate::appearance::{fmt_num, Ap};
use crate::geometry::PageGeometry;
use crate::security::lopdf_err;
use crate::{Document, EngineError, PointRect};
use lopdf::{Dictionary, Object, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub use crate::measurement_area::{
    MeasurementArea, MAX_HOLES, MAX_RING_VERTICES, MAX_TOTAL_VERTICES,
};
pub use crate::measurement_batch::{RecalibrationChange, RecalibrationPlan, RecalibrationRequest};
pub use crate::measurement_distance::parse_measurement_distance;
pub use crate::measurement_shapes::{
    dimensioned_rectangle_corners, dimensioned_rectangle_point_dimensions,
    normalize_rectangle_rotation, DimensionedRectangle, LocalCalibrationReference, RectangleScale,
};
pub use crate::measurement_snap::{SnapKind, SnapPoint, VectorSnapGeometry};
pub use crate::measurement_viewports::{
    select_viewport, MeasurementViewport, MeasurementViewportResetTarget,
};
pub use crate::page_calibration::PageCalibration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementKind {
    Length,
    Polyline,
    Perimeter,
    Area,
    Volume,
    Angle,
}

impl MeasurementKind {
    pub const ALL: [Self; 6] = [
        Self::Length,
        Self::Polyline,
        Self::Perimeter,
        Self::Area,
        Self::Volume,
        Self::Angle,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Polyline => "polyline",
            Self::Perimeter => "perimeter",
            Self::Area => "area",
            Self::Volume => "volume",
            Self::Angle => "angle",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementUnit {
    Millimetres,
    Centimetres,
    Metres,
    Inches,
    Feet,
}

impl MeasurementUnit {
    pub const ALL: [Self; 5] = [
        Self::Millimetres,
        Self::Centimetres,
        Self::Metres,
        Self::Inches,
        Self::Feet,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Millimetres => "mm",
            Self::Centimetres => "cm",
            Self::Metres => "m",
            Self::Inches => "in",
            Self::Feet => "ft",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|unit| unit.id() == id)
    }
    pub fn metres(self) -> f64 {
        match self {
            Self::Millimetres => 0.001,
            Self::Centimetres => 0.01,
            Self::Metres => 1.0,
            Self::Inches => 0.0254,
            Self::Feet => 0.3048,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub kind: MeasurementKind,
    /// Displayed-page points, bottom-left origin. Angle uses the middle point as its vertex.
    pub points: Vec<(f32, f32)>,
    /// Strictly interior, disjoint deductions in displayed-page coordinates.
    pub holes: Vec<Vec<(f32, f32)>>,
    /// Real, constant physical depth. Changing the result unit never changes this input.
    pub depth: Option<MeasurementDepth>,
    pub calibration: [(f32, f32); 2],
    pub known_distance: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasurementDepth {
    pub value: f64,
    pub unit: MeasurementUnit,
}

impl MeasurementDepth {
    fn in_unit(self, unit: MeasurementUnit) -> Result<f64, EngineError> {
        if !self.value.is_finite() || !(1e-9..=1e9).contains(&self.value) {
            return Err(bad());
        }
        let value = self.value * self.unit.metres() / unit.metres();
        if !value.is_finite() || value <= 0.0 {
            return Err(bad());
        }
        Ok(value)
    }

    fn json(self) -> Value {
        json!({"value":self.value,"unit":self.unit.id()})
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementValidity {
    Current,
    LegacyUnverified,
    Stale,
}

impl MeasurementValidity {
    pub fn id(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::LegacyUnverified => "legacy_unverified",
            Self::Stale => "stale",
        }
    }

    pub fn is_current(self) -> bool {
        self == Self::Current
    }
}

/// A saved annotation and its historical calibration, in displayed-page coordinates.
/// Only Current records qualify as measurements of the current page.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementRecord {
    pub index: usize,
    pub annotation_id: Option<String>,
    pub measurement: Measurement,
    pub validity: MeasurementValidity,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeasurementPageProvenance {
    page_bounds: [f32; 4],
    user_unit: f32,
    content_sha256: String,
}

pub(crate) fn page_provenance(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    geo: &PageGeometry,
) -> Result<MeasurementPageProvenance, EngineError> {
    let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
    crate::page_calibration::check_user_unit(pdf, dict)?;
    current_page_provenance(pdf, page, geo)
}

fn current_page_provenance(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    geo: &PageGeometry,
) -> Result<MeasurementPageProvenance, EngineError> {
    let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
    let user_unit = match dict.get(b"UserUnit") {
        Ok(raw) => pdf
            .dereference(raw)
            .and_then(|(_, value)| value.as_float())
            .map_err(lopdf_err)?,
        Err(_) => 1.0,
    };
    if !user_unit.is_finite() || user_unit <= 0.0 {
        return Err(bad());
    }
    Ok(MeasurementPageProvenance {
        page_bounds: [geo.bbox.left, geo.bbox.bottom, geo.bbox.right, geo.bbox.top],
        user_unit,
        content_sha256: crate::page_calibration::qualified_content_digest(pdf, dict)?,
    })
}

pub(crate) fn bind_provenance(
    annotation: &mut Dictionary,
    provenance: &MeasurementPageProvenance,
) -> Result<(), EngineError> {
    let raw = annotation
        .get(b"SuniPDFMeasurement")
        .and_then(Object::as_str)
        .map_err(lopdf_err)?;
    let mut metadata: Value = serde_json::from_slice(raw).map_err(|_| bad())?;
    if metadata["version"] != 3 {
        metadata["version"] = 2.into();
    }
    metadata["provenance"] = serde_json::to_value(provenance).map_err(|_| bad())?;
    annotation.set(
        "SuniPDFMeasurement",
        Object::string_literal(metadata.to_string()),
    );
    Ok(())
}

fn stored_provenance(metadata: &Value) -> Result<Option<MeasurementPageProvenance>, EngineError> {
    let version = metadata["version"].as_u64().ok_or_else(bad)?;
    if !matches!(version, 1..=3) || metadata["coordinate_system"] != "pdf-user-space" {
        return Err(bad());
    }
    let fields = [
        "version",
        "coordinate_system",
        "kind",
        "points",
        "calibration",
        "known_distance",
        "unit",
        "precision",
        "rect",
    ];
    if metadata.as_object().is_none_or(|object| {
        object.len() != fields.len() + usize::from(version >= 2) + 2 * usize::from(version == 3)
            || object.keys().any(|key| {
                !fields.contains(&key.as_str())
                    && !(version >= 2 && key == "provenance")
                    && !(version == 3 && matches!(key.as_str(), "holes" | "depth"))
            })
    }) {
        return Err(bad());
    }
    if version == 1 {
        return Ok(None);
    }
    let provenance: MeasurementPageProvenance =
        serde_json::from_value(metadata["provenance"].clone()).map_err(|_| bad())?;
    if provenance.user_unit != 1.0
        || provenance
            .page_bounds
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
        || provenance.page_bounds[0] >= provenance.page_bounds[2]
        || provenance.page_bounds[1] >= provenance.page_bounds[3]
        || provenance.content_sha256.len() != 64
        || !provenance
            .content_sha256
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(bad());
    }
    Ok(Some(provenance))
}

/// An editable annotation bound to its validated source document. Obtain it from
/// `Document::measurement_target`; indices alone are not sufficient to identify an edit.
#[derive(Clone)]
pub struct MeasurementTarget {
    page: usize,
    record: MeasurementRecord,
    source: Arc<Vec<u8>>,
}

impl std::fmt::Debug for MeasurementTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeasurementTarget")
            .field("page", &self.page)
            .field("index", &self.record.index)
            .field("annotation_id", &self.record.annotation_id)
            .finish_non_exhaustive()
    }
}

impl MeasurementTarget {
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn index(&self) -> usize {
        self.record.index
    }
}

impl MeasurementRecord {
    pub fn result(&self) -> Result<Value, EngineError> {
        let mut result = self.measurement.result()?;
        result["index"] = self.index.into();
        result["annotation_id"] = self
            .annotation_id
            .as_ref()
            .map_or(Value::Null, |name| name.as_str().into());
        result["version"] = self.measurement.metadata_version().into();
        result["validity"] = self.validity.id().into();
        result["historical_value"] = Value::Null;
        result["historical_label"] = Value::Null;
        result["historical_area"] = Value::Null;
        if !self.validity.is_current() {
            result["historical_value"] = result["value"].take();
            result["historical_label"] = result["label"].take();
            result["historical_area"] = result["area"].take();
        }
        Ok(result)
    }
}

fn bad() -> EngineError {
    EngineError::Other("invalid measurement: use finite in-page points, a positive two-point calibration and simple geometry; surfaces allow 16 strictly interior holes, 128 vertices per ring and 512 total, and volumes require a positive physical depth".into())
}

fn stale() -> EngineError {
    EngineError::Other("measurement changed; refresh it before editing".into())
}

pub(crate) fn bounded_text(
    pdf: &lopdf::Document,
    dict: &Dictionary,
    key: &[u8],
    limit: usize,
) -> Result<Option<String>, EngineError> {
    let Ok(raw) = dict.get(key) else {
        return Ok(None);
    };
    let bytes = pdf
        .dereference(raw)
        .and_then(|(_, value)| value.as_str())
        .map_err(lopdf_err)?;
    if bytes.len() > limit {
        return Err(bad());
    }
    text_of(pdf, dict, key).map(Some).ok_or_else(bad)
}

/// Editing is deliberately narrower than reading: retain unknown records without rewriting them.
pub(crate) fn check_editable(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    record: &MeasurementRecord,
) -> Result<(), EngineError> {
    check_editable_inner(pdf, page, record, true)
}

pub(crate) fn check_editable_local(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    record: &MeasurementRecord,
) -> Result<(), EngineError> {
    check_editable_inner(pdf, page, record, false)
}

fn check_editable_inner(
    pdf: &lopdf::Document,
    page: lopdf::ObjectId,
    record: &MeasurementRecord,
    check_identity: bool,
) -> Result<(), EngineError> {
    let (_, dict) = load_annot(pdf, page, record.index)?;
    if dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Stamp".as_slice())
        || dict.has(b"Measure")
    {
        return Err(bad());
    }
    let appearance = pdf
        .dereference(dict.get(b"AP").map_err(lopdf_err)?)
        .and_then(|(_, o)| o.as_dict())
        .map_err(lopdf_err)?;
    if appearance.len() != 1
        || !appearance.has(b"N")
        || pdf
            .dereference(appearance.get(b"N").map_err(lopdf_err)?)
            .and_then(|(_, o)| o.as_stream())
            .is_err()
    {
        return Err(bad());
    }
    let flags = match dict.get(b"F") {
        Ok(raw) => pdf
            .dereference(raw)
            .and_then(|(_, o)| o.as_i64())
            .map_err(lopdf_err)?,
        Err(_) => 0,
    };
    if flags < 0 || flags & (64 | 128 | 512) != 0 {
        return Err(EngineError::SecurityRestricted);
    }
    let id = record
        .annotation_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .ok_or_else(bad)?;
    if bounded_text(pdf, &dict, b"NM", 256)?.as_deref() != Some(id) {
        return Err(stale());
    }
    // Replies live in separate /IRT annotations. A changed root comment must not be silently
    // overwritten by a regenerated value label, even when its private geometry is unchanged.
    let expected = record.measurement.result()?;
    if bounded_text(pdf, &dict, b"Contents", 4096)?.as_deref() != expected["label"].as_str() {
        return Err(EngineError::Other(
            "measurement text does not match its stored value; preserve the comment and recreate the measurement before editing its value".into(),
        ));
    }
    if let Ok(parent) = dict.get(b"P") {
        if parent.as_reference().map_err(lopdf_err)? != page {
            return Err(bad());
        }
    }
    let raw = dict
        .get(b"SuniPDFMeasurement")
        .and_then(Object::as_str)
        .map_err(lopdf_err)?;
    if raw.len() > 32768 {
        return Err(bad());
    }
    let metadata: Value = serde_json::from_slice(raw).map_err(|_| bad())?;
    stored_provenance(&metadata)?;
    if !check_identity {
        return Ok(());
    }
    // Shared annotation objects, shared Annots arrays and duplicate identities cannot be edited
    // without risking an unrelated page. Count occurrences, including direct dictionaries.
    let mut seen = 0usize;
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
            return Err(bad());
        }
        for entry in entries {
            let annotation = pdf
                .dereference(entry)
                .and_then(|(_, o)| o.as_dict())
                .map_err(lopdf_err)?;
            if bounded_text(pdf, annotation, b"NM", 256)?.as_deref() == Some(id) {
                seen += 1;
            }
        }
    }
    if seen != 1 {
        return Err(bad());
    }
    Ok(())
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f64 {
    (f64::from(b.0) - f64::from(a.0)).hypot(f64::from(b.1) - f64::from(a.1))
}

impl Measurement {
    fn computation(&self) -> Result<(f64, Option<MeasurementArea>), EngineError> {
        let surface = matches!(self.kind, MeasurementKind::Area | MeasurementKind::Volume);
        if self.precision > 4
            || self.points.len() > MAX_RING_VERTICES
            || self.points.len() < 2
            || self.holes.len() > MAX_HOLES
            || self.holes.iter().any(|ring| ring.len() > MAX_RING_VERTICES)
            || self.points.len() + self.holes.iter().map(Vec::len).sum::<usize>()
                > MAX_TOTAL_VERTICES
            || (!surface && (!self.holes.is_empty() || self.depth.is_some()))
            || (self.kind == MeasurementKind::Area && self.depth.is_some())
            || (self.kind == MeasurementKind::Volume && self.depth.is_none())
            || self
                .points
                .iter()
                .chain(self.holes.iter().flatten())
                .chain(self.calibration.iter())
                .any(|p| !p.0.is_finite() || !p.1.is_finite())
            || !self.known_distance.is_finite()
            || !(1e-9..=1e9).contains(&self.known_distance)
            || distance(self.calibration[0], self.calibration[1]) < 0.01
            || self
                .points
                .windows(2)
                .any(|pair| distance(pair[0], pair[1]) < 0.01)
        {
            return Err(bad());
        }
        let scale = self.known_distance / distance(self.calibration[0], self.calibration[1]);
        if !(1e-9..=1e9).contains(&scale) {
            return Err(bad());
        }
        let n = self.points.len();
        let closed = matches!(
            self.kind,
            MeasurementKind::Area | MeasurementKind::Volume | MeasurementKind::Perimeter
        );
        if (self.kind == MeasurementKind::Length && n != 2)
            || (self.kind == MeasurementKind::Angle && n != 3)
            || (closed && n < 3)
        {
            return Err(bad());
        }
        if self.kind == MeasurementKind::Perimeter {
            crate::measurement_area::ring_area(&self.points)?;
        }
        let area = surface
            .then(|| crate::measurement_area::breakdown(self, scale))
            .transpose()?;
        let value = match self.kind {
            MeasurementKind::Length | MeasurementKind::Polyline | MeasurementKind::Perimeter => {
                let mut length: f64 = self
                    .points
                    .windows(2)
                    .map(|pair| distance(pair[0], pair[1]))
                    .sum();
                if closed {
                    length += distance(self.points[n - 1], self.points[0]);
                }
                length * scale
            }
            MeasurementKind::Area => area.as_ref().ok_or_else(bad)?.net,
            MeasurementKind::Volume => {
                area.as_ref().ok_or_else(bad)?.net
                    * self.depth.ok_or_else(bad)?.in_unit(self.unit)?
            }
            MeasurementKind::Angle => {
                let [a, b, c] = [self.points[0], self.points[1], self.points[2]];
                let dot = (f64::from(a.0) - f64::from(b.0)) * (f64::from(c.0) - f64::from(b.0))
                    + (f64::from(a.1) - f64::from(b.1)) * (f64::from(c.1) - f64::from(b.1));
                (dot / (distance(a, b) * distance(c, b)))
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees()
            }
        };
        if !value.is_finite() || !(1e-9..=1e12).contains(&value) {
            return Err(bad());
        }
        Ok((value, area))
    }

    /// Length units, squared units for area, cubic units for volume, or degrees for angles.
    pub fn value(&self) -> Result<f64, EngineError> {
        self.computation().map(|(value, _)| value)
    }

    pub fn area_breakdown(&self) -> Result<MeasurementArea, EngineError> {
        self.computation()?.1.ok_or_else(bad)
    }

    fn metadata_version(&self) -> u32 {
        if self.kind == MeasurementKind::Volume || !self.holes.is_empty() {
            3
        } else {
            2
        }
    }

    pub fn result(&self) -> Result<Value, EngineError> {
        let (value, area) = self.computation()?;
        let unit = match self.kind {
            MeasurementKind::Angle => "deg".to_string(),
            MeasurementKind::Area => format!("{}2", self.unit.id()),
            MeasurementKind::Volume => format!("{}3", self.unit.id()),
            _ => self.unit.id().to_string(),
        };
        Ok(json!({"kind":self.kind.id(), "value":value, "unit":unit,
            "precision":self.precision, "label":format!("{:.*} {}", self.precision, value, unit),
            "calibration":{"points":self.calibration, "known_distance":self.known_distance,
                "unit":self.unit.id(), "units_per_point":self.known_distance / distance(self.calibration[0], self.calibration[1])},
            "points":self.points, "holes":self.holes, "depth":self.depth.map(MeasurementDepth::json),
            "area":area, "area_unit":area.as_ref().map(|_|format!("{}2",self.unit.id()))}))
    }
}

/// Computes exactly the geometry that the reader will recover from serialized user-space f32
/// points. Refuse excessive loss instead of committing a different scale or rounded label.
pub(crate) fn measurement_after_storage(
    geo: &PageGeometry,
    measurement: &Measurement,
) -> Result<Measurement, EngineError> {
    let before = measurement.value()?;
    let roundtrip = |(x, y)| {
        let (x, y) = geo.to_user(x, y);
        geo.to_display(x, y)
    };
    let mut stored = measurement.clone();
    stored.points = measurement.points.iter().copied().map(roundtrip).collect();
    stored.holes = measurement
        .holes
        .iter()
        .map(|ring| ring.iter().copied().map(roundtrip).collect())
        .collect();
    stored.calibration = measurement.calibration.map(roundtrip);
    let after = stored.value()?;
    let close = |original: f64, saved: f64| {
        saved.is_finite() && (original - saved).abs() <= original.abs() * 1e-5
    };
    // Lengths and areas alone cannot detect a uniform translation caused by large crop
    // origins. Match the absolute per-coordinate tolerance used for saved page/rect guards.
    let positions_match = measurement
        .points
        .iter()
        .chain(measurement.holes.iter().flatten())
        .chain(measurement.calibration.iter())
        .zip(
            stored
                .points
                .iter()
                .chain(stored.holes.iter().flatten())
                .chain(stored.calibration.iter()),
        )
        .all(|(original, saved)| {
            (f64::from(original.0) - f64::from(saved.0)).abs() <= 0.001
                && (f64::from(original.1) - f64::from(saved.1)).abs() <= 0.001
        });
    let mut precise = positions_match
        && close(before, after)
        && close(
            distance(measurement.calibration[0], measurement.calibration[1]),
            distance(stored.calibration[0], stored.calibration[1]),
        )
        && format!("{:.*}", measurement.precision, before)
            == format!("{:.*}", measurement.precision, after);
    for (original, saved) in measurement.points.windows(2).zip(stored.points.windows(2)) {
        precise &= close(
            distance(original[0], original[1]),
            distance(saved[0], saved[1]),
        );
    }
    if matches!(
        measurement.kind,
        MeasurementKind::Area | MeasurementKind::Volume | MeasurementKind::Perimeter
    ) {
        let last = measurement.points.len() - 1;
        precise &= close(
            distance(measurement.points[last], measurement.points[0]),
            distance(stored.points[last], stored.points[0]),
        );
    }
    for (original, saved) in measurement.holes.iter().zip(&stored.holes) {
        for i in 0..original.len() {
            let next = (i + 1) % original.len();
            precise &= close(
                distance(original[i], original[next]),
                distance(saved[i], saved[next]),
            );
        }
    }
    if matches!(
        measurement.kind,
        MeasurementKind::Area | MeasurementKind::Volume
    ) {
        let original = measurement.area_breakdown()?;
        let saved = stored.area_breakdown()?;
        precise &= close(original.gross, saved.gross)
            && close(original.net, saved.net)
            && original
                .deductions
                .iter()
                .zip(saved.deductions)
                .all(|(a, b)| close(*a, b));
    }
    if !precise {
        return Err(EngineError::Other("measurement coordinates lose precision when stored; use a longer calibration reference or a different page position".into()));
    }
    Ok(stored)
}

pub(crate) fn annotation_components(
    geo: &PageGeometry,
    measurement: &Measurement,
    author: Option<&str>,
) -> Result<(Dictionary, Stream), EngineError> {
    let stored = measurement_after_storage(geo, measurement)?;
    let result = stored.result()?;
    let label = result["label"].as_str().ok_or_else(bad)?;
    let (pw, ph) = geo.display_size();
    if measurement
        .points
        .iter()
        .chain(measurement.holes.iter().flatten())
        .chain(measurement.calibration.iter())
        .chain(stored.points.iter())
        .chain(stored.holes.iter().flatten())
        .chain(stored.calibration.iter())
        .any(|&(x, y)| x < 0.0 || y < 0.0 || x > pw || y > ph)
    {
        return Err(bad());
    }
    let min_x = measurement
        .points
        .iter()
        .map(|p| p.0)
        .fold(f32::INFINITY, f32::min);
    let max_x = measurement
        .points
        .iter()
        .map(|p| p.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = measurement
        .points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min);
    let max_y = measurement
        .points
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max);
    let label_width = label.len() as f32 * 6.2 + 8.0;
    if label_width > pw {
        return Err(bad());
    }
    let left = (min_x - 6.0).max(0.0).min(pw - label_width);
    let rect = PointRect {
        left,
        right: (max_x + 6.0).max(left + label_width).min(pw),
        bottom: (min_y - 22.0).max(0.0),
        top: (max_y + 6.0).max(26.0).min(ph),
    };
    let (w, h) = checked_rect(geo, rect)?;
    let mut annotation = rich_base("Stamp", rect, geo, author)?;
    let user_points: Vec<_> = measurement
        .points
        .iter()
        .map(|&(x, y)| geo.to_user(x, y))
        .collect();
    let user_calibration = measurement.calibration.map(|(x, y)| geo.to_user(x, y));
    let user_holes: Vec<Vec<_>> = measurement
        .holes
        .iter()
        .map(|ring| ring.iter().map(|&(x, y)| geo.to_user(x, y)).collect())
        .collect();
    let ur = geo.rect_to_user(rect);
    if user_points
        .iter()
        .chain(user_holes.iter().flatten())
        .chain(user_calibration.iter())
        .any(|p| !p.0.is_finite() || !p.1.is_finite() || p.0.abs() > 1e8 || p.1.abs() > 1e8)
        || [ur.left, ur.bottom, ur.right, ur.top]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
    {
        return Err(bad());
    }
    let mut metadata = json!({"version":1,"coordinate_system":"pdf-user-space", "kind":measurement.kind.id(),
            "points":user_points,"calibration":user_calibration,"known_distance":measurement.known_distance,
            "unit":measurement.unit.id(),"precision":measurement.precision,"rect":[ur.left,ur.bottom,ur.right,ur.top]});
    if measurement.metadata_version() == 3 {
        metadata["version"] = 3.into();
        metadata["holes"] = json!(user_holes);
        metadata["depth"] = json!(measurement.depth.map(MeasurementDepth::json));
    }
    let metadata = metadata.to_string();
    if metadata.len() > 32768 {
        return Err(bad());
    }
    annotation.set("SuniPDFMeasurement", Object::string_literal(metadata));
    annotation.set("Name", Object::Name(b"SuniPDFMeasurement".to_vec()));
    annotation.set("Contents", lopdf::text_string(label));
    let mut content = String::from("q /GS gs 0.05 0.25 0.75 RG 1.25 w 1 J 1 j\n");
    for (i, &(x, y)) in measurement.points.iter().enumerate() {
        content.push_str(&format!(
            "{} {} {}\n",
            fmt_num(x - rect.left),
            fmt_num(y - rect.bottom),
            if i == 0 { "m" } else { "l" }
        ));
    }
    if matches!(
        measurement.kind,
        MeasurementKind::Area | MeasurementKind::Volume | MeasurementKind::Perimeter
    ) {
        content.push_str("h\n");
    }
    content.push_str("S\n");
    for ring in &measurement.holes {
        for (i, &(x, y)) in ring.iter().enumerate() {
            content.push_str(&format!(
                "{} {} {}\n",
                fmt_num(x - rect.left),
                fmt_num(y - rect.bottom),
                if i == 0 { "m" } else { "l" }
            ));
        }
        content.push_str("h S\n");
    }
    // Deducted surfaces and volumes remain outline-only, including a transparent label.
    if measurement.metadata_version() != 3 {
        content.push_str(&format!(
            "1 1 1 rg 2 1 {} 14 re f\n",
            fmt_num(label_width - 2.0)
        ));
    }
    content.push_str(&format!(
        "0.05 0.25 0.75 rg BT /Helv 10 Tf 4 4 Td ({label}) Tj ET Q\n"
    ));
    Ok((
        annotation,
        ap_stream(
            &Ap {
                w,
                h,
                content: content.into_bytes(),
                opacity: 1.0,
                uses_font: true,
            },
            geo,
        ),
    ))
}

impl Document<'_> {
    /// Adds a self-contained measurement. Its scale never depends on an external page setting.
    pub fn add_measurement_annotation(
        &mut self,
        page_index: usize,
        measurement: &Measurement,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        let geo = self.page_geometry(page_index)?;
        let (mut annotation, appearance) = annotation_components(&geo, measurement, author)?;
        self.edit_annotation_pdf(page_index, |pdf, page, _| {
            bind_provenance(&mut annotation, &page_provenance(pdf, page, &geo)?)?;
            push_annotation(pdf, page, annotation, Some(appearance))
        })
    }

    /// Captures an exact current record before an explicit replacement. Read access remains
    /// separate from editing, so documents that permit annotation but prohibit copying work.
    pub fn measurement_target(
        &self,
        page: usize,
        expected: &MeasurementRecord,
    ) -> Result<MeasurementTarget, EngineError> {
        self.require_annotation_permission()?;
        let records = self.measurement_records(page)?;
        if records.iter().find(|record| record.index == expected.index) != Some(expected) {
            return Err(stale());
        }
        let source = self.snapshot()?;
        if source.len() > 128 * 1024 * 1024 {
            return Err(EngineError::Other(
                "measurement editing is limited to documents up to 128 MiB".into(),
            ));
        }
        let pdf = crate::security::load(&source, self.password())?;
        let page_id = *pdf.get_pages().get(&(page as u32 + 1)).ok_or_else(bad)?;
        check_editable(&pdf, page_id, expected)?;
        Ok(MeasurementTarget {
            page,
            record: expected.clone(),
            source: Arc::new(source),
        })
    }

    /// Replaces only this measurement's geometry, calibrated value and appearance. Annotation
    /// identity, author, replies, popup and register fields survive; page defaults are unchanged.
    /// Any document change after target creation requires a fresh target and preview.
    pub fn update_measurement_annotation(
        &mut self,
        target: &MeasurementTarget,
        replacement: &Measurement,
    ) -> Result<usize, EngineError> {
        if !target.record.validity.is_current() {
            return Err(EngineError::Other("measurement is not verified against this page; explicitly recalibrate it, or recreate an angle, before editing".into()));
        }
        self.replace_measurement_annotation(target, replacement)
    }

    /// Explicitly re-establishes a selected measurement against the current page and supplied
    /// reference points. Units/precision/geometry edits alone never certify old records.
    /// Angles must be recreated because they do not use a length calibration.
    pub fn recalibrate_measurement_annotation(
        &mut self,
        target: &MeasurementTarget,
        replacement: &Measurement,
        calibration: &PageCalibration,
    ) -> Result<usize, EngineError> {
        if target.record.measurement.kind == MeasurementKind::Angle
            || replacement.kind == MeasurementKind::Angle
        {
            return Err(EngineError::Other(
                "angles must be recreated on the current page instead of recalibrated".into(),
            ));
        }
        calibration.validate_on_page(&self.page_geometry(target.page)?)?;
        let mut replacement = replacement.clone();
        replacement.calibration = calibration.points;
        replacement.known_distance =
            calibration.known_distance * calibration.unit.metres() / replacement.unit.metres();
        self.replace_measurement_annotation(target, &replacement)
    }

    fn replace_measurement_annotation(
        &mut self,
        target: &MeasurementTarget,
        replacement: &Measurement,
    ) -> Result<usize, EngineError> {
        self.require_annotation_permission()?;
        if !crate::annot_rich::same_document(&target.source, &self.snapshot()?, self.password())? {
            return Err(stale());
        }
        let geo = self.page_geometry(target.page)?;
        let stored = measurement_after_storage(&geo, replacement)?;
        let (mut components, appearance) = annotation_components(&geo, replacement, None)?;
        self.edit_annotation_pdf(target.page, |pdf, page, _| {
            check_editable(pdf, page, &target.record)?;
            bind_provenance(&mut components, &page_provenance(pdf, page, &geo)?)?;
            let (slot, mut annotation) = load_annot(pdf, page, target.record.index)?;
            crate::quantity_table::validate_measurement_price(pdf, &annotation, &stored)?;
            for key in [b"Rect".as_slice(), b"Contents", b"SuniPDFMeasurement"] {
                annotation.set(key, components.get(key).map_err(lopdf_err)?.clone());
            }
            // Appearance streams may be shared. Detach this annotation before installing its
            // new normal appearance instead of overwriting a stream used on another page.
            annotation.remove(b"AP");
            set_ap(pdf, &mut annotation, appearance);
            touch(&mut annotation);
            store_annot(pdf, page, slot, annotation)?;
            Ok(target.record.index)
        })
    }

    /// Exports only supported self-contained records; malformed or moved metadata is an error.
    pub fn measurements(&self, page_index: usize) -> Result<Vec<Value>, EngineError> {
        self.measurement_records(page_index)?
            .iter()
            .map(MeasurementRecord::result)
            .collect()
    }

    /// Reads supported annotations without discarding their calibration or page coordinates.
    pub fn measurement_records(
        &self,
        page_index: usize,
    ) -> Result<Vec<MeasurementRecord>, EngineError> {
        let pdf = crate::security::load(&self.snapshot()?, self.password())?;
        self.measurement_records_from_pdf(page_index, &pdf)
    }

    pub(crate) fn measurement_records_from_pdf(
        &self,
        page_index: usize,
        pdf: &lopdf::Document,
    ) -> Result<Vec<MeasurementRecord>, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let page = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or_else(bad)?;
        let page_dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
        let entries = match page_dict.get(b"Annots") {
            Err(_) => Vec::new(),
            Ok(raw) => pdf
                .dereference(raw)
                .and_then(|(_, object)| object.as_array())
                .map_err(lopdf_err)?
                .clone(),
        };
        if entries.len() > 10000 {
            return Err(bad());
        }
        let mut records = Vec::new();
        let mut current_provenance = None;
        let mut metadata_bytes = 0usize;
        for (index, entry) in entries.iter().enumerate() {
            let dict = match entry {
                Object::Reference(id) => pdf.get_dictionary(*id).map_err(lopdf_err)?,
                Object::Dictionary(dict) => dict,
                _ => return Err(bad()),
            };
            let Ok(raw) = dict.get(b"SuniPDFMeasurement") else {
                if dict.has(b"Measure") {
                    return Err(EngineError::Other("imported measurement metadata is unsupported; recalibrate it in SuniPDF before exporting quantities".into()));
                }
                continue;
            };
            let raw = raw.as_str().map_err(lopdf_err)?;
            metadata_bytes = metadata_bytes.saturating_add(raw.len());
            if raw.len() > 32768 || metadata_bytes > 4 * 1024 * 1024 || records.len() >= 4096 {
                return Err(bad());
            }
            let value: Value = serde_json::from_slice(raw).map_err(|_| bad())?;
            let provenance = stored_provenance(&value)?;
            let validity = if let Some(provenance) = provenance {
                // One digest per page, shared by every saved record on this page.
                if current_provenance.is_none() {
                    current_provenance = Some(current_page_provenance(pdf, page, &geo)?);
                }
                let current = current_provenance
                    .as_ref()
                    .expect("page provenance was computed");
                if provenance.content_sha256 == current.content_sha256
                    && provenance
                        .page_bounds
                        .iter()
                        .zip(current.page_bounds)
                        .all(|(a, b)| (a - b).abs() <= 0.001)
                    && provenance.user_unit == current.user_unit
                {
                    MeasurementValidity::Current
                } else {
                    MeasurementValidity::Stale
                }
            } else {
                MeasurementValidity::LegacyUnverified
            };
            let scalar = |v: &Value| -> Result<f32, EngineError> {
                let number = v.as_f64().ok_or_else(bad)?;
                if !number.is_finite() || number.abs() > 1e8 {
                    return Err(bad());
                }
                Ok(number as f32)
            };
            let point = |v: &Value| -> Result<(f32, f32), EngineError> {
                let a = v.as_array().filter(|a| a.len() == 2).ok_or_else(bad)?;
                Ok(geo.to_display(scalar(&a[0])?, scalar(&a[1])?))
            };
            let rect = dict
                .get(b"Rect")
                .and_then(Object::as_array)
                .map_err(lopdf_err)?;
            let expected = value["rect"]
                .as_array()
                .filter(|a| a.len() == 4)
                .ok_or_else(bad)?;
            if rect.len() != 4
                || rect.iter().zip(expected).any(|(a, b)| {
                    a.as_float()
                        .ok()
                        .zip(scalar(b).ok())
                        .is_none_or(|(a, b)| !a.is_finite() || (a - b).abs() > 0.001)
                })
            {
                return Err(EngineError::Other("measurement geometry was changed externally; recreate and calibrate it before using this value".into()));
            }
            let points = value["points"]
                .as_array()
                .filter(|a| a.len() <= 128)
                .ok_or_else(bad)?
                .iter()
                .map(point)
                .collect::<Result<Vec<_>, _>>()?;
            let calibration = value["calibration"]
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(bad)?;
            let (holes, depth) = if value["version"] == 3 {
                let holes = value["holes"]
                    .as_array()
                    .filter(|rings| rings.len() <= MAX_HOLES)
                    .ok_or_else(bad)?
                    .iter()
                    .map(|ring| {
                        ring.as_array()
                            .filter(|points| points.len() <= MAX_RING_VERTICES)
                            .ok_or_else(bad)?
                            .iter()
                            .map(point)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let depth = if value["depth"].is_null() {
                    None
                } else {
                    let object = value["depth"]
                        .as_object()
                        .filter(|o| {
                            o.len() == 2 && o.contains_key("value") && o.contains_key("unit")
                        })
                        .ok_or_else(bad)?;
                    Some(MeasurementDepth {
                        value: object["value"].as_f64().ok_or_else(bad)?,
                        unit: MeasurementUnit::from_id(object["unit"].as_str().ok_or_else(bad)?)
                            .ok_or_else(bad)?,
                    })
                };
                (holes, depth)
            } else {
                (Vec::new(), None)
            };
            let measurement = Measurement {
                kind: MeasurementKind::from_id(value["kind"].as_str().ok_or_else(bad)?)
                    .ok_or_else(bad)?,
                points,
                holes,
                depth,
                calibration: [point(&calibration[0])?, point(&calibration[1])?],
                known_distance: value["known_distance"].as_f64().ok_or_else(bad)?,
                unit: MeasurementUnit::from_id(value["unit"].as_str().ok_or_else(bad)?)
                    .ok_or_else(bad)?,
                precision: value["precision"]
                    .as_u64()
                    .filter(|&n| n <= 4)
                    .ok_or_else(bad)? as usize,
            };
            measurement.value()?;
            let annotation_id = bounded_text(pdf, dict, b"NM", 256).ok().flatten();
            records.push(MeasurementRecord {
                index,
                annotation_id,
                measurement,
                validity,
            });
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibrated_geometry_and_unit_conversions_refuse_ambiguous_quantities() {
        let mut m = Measurement {
            kind: MeasurementKind::Area,
            holes: Vec::new(),
            depth: None,
            points: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)],
            calibration: [(0.0, 0.0), (100.0, 0.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
        };
        assert!((m.value().unwrap() - 50.0).abs() < 1e-8);
        m.kind = MeasurementKind::Perimeter;
        assert!((m.value().unwrap() - 30.0).abs() < 1e-8);
        m.kind = MeasurementKind::Polyline;
        assert!((m.value().unwrap() - 25.0).abs() < 1e-8);
        m.points = vec![(0.0, 0.0), (100.0, 0.0)];
        m.kind = MeasurementKind::Length;
        for unit in MeasurementUnit::ALL {
            m.unit = unit;
            m.known_distance = 10.0 / unit.metres();
            assert!((m.value().unwrap() * unit.metres() - 10.0).abs() < 1e-8);
        }
        m.points = vec![(0.0, 100.0), (0.0, 0.0), (100.0, 0.0)];
        m.kind = MeasurementKind::Angle;
        assert!((m.value().unwrap() - 90.0).abs() < 1e-8);
        m.kind = MeasurementKind::Area;
        m.points = vec![(0.0, 0.0), (100.0, 100.0), (0.0, 100.0), (100.0, 0.0)];
        assert!(m.value().is_err());
        m.points = vec![(0.0, 0.0), (50.0, 0.0), (100.0, 0.0)];
        assert!(m.value().is_err());
        m.kind = MeasurementKind::Length;
        m.points.truncate(2);
        m.calibration[1] = m.calibration[0];
        assert!(m.value().is_err());
        m.calibration[1] = (100.0, 0.0);
        m.known_distance = f64::NAN;
        assert!(m.value().is_err());
    }
}
