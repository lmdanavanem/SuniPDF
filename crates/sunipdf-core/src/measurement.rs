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
use serde_json::{json, Value};
use std::sync::Arc;

pub use crate::measurement_batch::{RecalibrationChange, RecalibrationPlan, RecalibrationRequest};
pub use crate::measurement_distance::parse_measurement_distance;
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
    Angle,
}

impl MeasurementKind {
    pub const ALL: [Self; 5] = [
        Self::Length,
        Self::Polyline,
        Self::Perimeter,
        Self::Area,
        Self::Angle,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Polyline => "polyline",
            Self::Perimeter => "perimeter",
            Self::Area => "area",
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
    pub calibration: [(f32, f32); 2],
    pub known_distance: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
}

/// A validated saved annotation and its own calibration, in displayed-page coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementRecord {
    pub index: usize,
    pub annotation_id: Option<String>,
    pub measurement: Measurement,
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
        result["version"] = 1.into();
        Ok(result)
    }
}

fn bad() -> EngineError {
    EngineError::Other("invalid measurement: use finite in-page points, a positive two-point calibration and simple non-degenerate geometry (at most 128 vertices)".into())
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
    if metadata["version"] != 1
        || metadata.as_object().is_none_or(|object| {
            object.len() != fields.len() || object.keys().any(|key| !fields.contains(&key.as_str()))
        })
    {
        return Err(bad());
    }
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

impl Measurement {
    /// The numerical result is in `unit`, squared units for area, and degrees for angles.
    pub fn value(&self) -> Result<f64, EngineError> {
        if self.precision > 4
            || self.points.len() > 128
            || self.points.len() < 2
            || self
                .points
                .iter()
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
            MeasurementKind::Area | MeasurementKind::Perimeter
        );
        if (self.kind == MeasurementKind::Length && n != 2)
            || (self.kind == MeasurementKind::Angle && n != 3)
            || (closed && n < 3)
        {
            return Err(bad());
        }
        if closed {
            if distance(self.points[0], self.points[n - 1]) < 0.01 {
                return Err(bad());
            }
            let twice_area = (1..n - 1)
                .map(|i| cross(self.points[0], self.points[i], self.points[i + 1]))
                .sum::<f64>()
                .abs();
            if twice_area < 1e-6 {
                return Err(bad());
            }
            for i in 0..n {
                let (a, b, c) = (
                    self.points[(i + n - 1) % n],
                    self.points[i],
                    self.points[(i + 1) % n],
                );
                let dot = (f64::from(a.0) - f64::from(b.0)) * (f64::from(c.0) - f64::from(b.0))
                    + (f64::from(a.1) - f64::from(b.1)) * (f64::from(c.1) - f64::from(b.1));
                if cross(a, b, c).abs() < 1e-6 && dot > 0.0 {
                    return Err(bad());
                }
                for j in i + 1..n {
                    if j == i + 1 || (i == 0 && j == n - 1) {
                        continue;
                    }
                    if intersects(
                        self.points[i],
                        self.points[(i + 1) % n],
                        self.points[j],
                        self.points[(j + 1) % n],
                    ) {
                        return Err(bad());
                    }
                }
            }
        }
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
            MeasurementKind::Area => {
                // Translating first avoids cancellation on drawings with large coordinate offsets.
                (1..n - 1)
                    .map(|i| cross(self.points[0], self.points[i], self.points[i + 1]))
                    .sum::<f64>()
                    .abs()
                    * 0.5
                    * scale
                    * scale
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
        Ok(value)
    }

    pub fn result(&self) -> Result<Value, EngineError> {
        let value = self.value()?;
        let unit = match self.kind {
            MeasurementKind::Angle => "deg".to_string(),
            MeasurementKind::Area => format!("{}2", self.unit.id()),
            _ => self.unit.id().to_string(),
        };
        Ok(json!({"kind":self.kind.id(), "value":value, "unit":unit,
            "precision":self.precision, "label":format!("{:.*} {}", self.precision, value, unit),
            "calibration":{"points":self.calibration, "known_distance":self.known_distance,
                "unit":self.unit.id(), "units_per_point":self.known_distance / distance(self.calibration[0], self.calibration[1])},
            "points":self.points}))
    }
}

pub(crate) fn annotation_components(
    geo: &PageGeometry,
    measurement: &Measurement,
    author: Option<&str>,
) -> Result<(Dictionary, Stream), EngineError> {
    let result = measurement.result()?;
    let label = result["label"].as_str().ok_or_else(bad)?;
    let (pw, ph) = geo.display_size();
    if measurement
        .points
        .iter()
        .chain(measurement.calibration.iter())
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
    let ur = geo.rect_to_user(rect);
    if user_points
        .iter()
        .chain(user_calibration.iter())
        .any(|p| !p.0.is_finite() || !p.1.is_finite() || p.0.abs() > 1e8 || p.1.abs() > 1e8)
        || [ur.left, ur.bottom, ur.right, ur.top]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
    {
        return Err(bad());
    }
    let metadata = json!({"version":1,"coordinate_system":"pdf-user-space", "kind":measurement.kind.id(),
            "points":user_points,"calibration":user_calibration,"known_distance":measurement.known_distance,
            "unit":measurement.unit.id(),"precision":measurement.precision,"rect":[ur.left,ur.bottom,ur.right,ur.top]});
    annotation.set(
        "SuniPDFMeasurement",
        Object::string_literal(metadata.to_string()),
    );
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
        MeasurementKind::Area | MeasurementKind::Perimeter
    ) {
        content.push_str("h\n");
    }
    content.push_str("S\n1 1 1 rg 2 1 ");
    content.push_str(&format!(
        "{} 14 re f\n0.05 0.25 0.75 rg BT /Helv 10 Tf 4 4 Td ({label}) Tj ET Q\n",
        fmt_num(label_width - 2.0)
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
        let (annotation, appearance) = annotation_components(&geo, measurement, author)?;
        self.edit_annotation_pdf(page_index, |pdf, page, _| {
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
        self.require_annotation_permission()?;
        if !crate::annot_rich::same_document(&target.source, &self.snapshot()?, self.password())? {
            return Err(stale());
        }
        let geo = self.page_geometry(target.page)?;
        let (components, appearance) = annotation_components(&geo, replacement, None)?;
        self.edit_annotation_pdf(target.page, |pdf, page, _| {
            check_editable(pdf, page, &target.record)?;
            let (slot, mut annotation) = load_annot(pdf, page, target.record.index)?;
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
            if value["version"] != 1 || value["coordinate_system"] != "pdf-user-space" {
                return Err(bad());
            }
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
            let measurement = Measurement {
                kind: MeasurementKind::from_id(value["kind"].as_str().ok_or_else(bad)?)
                    .ok_or_else(bad)?,
                points,
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
