use crate::{annot_cmd, password_input};
use clap::{ArgGroup, Args};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::measurement::{
    Measurement, MeasurementDepth, MeasurementKind, MeasurementRecord, MeasurementUnit,
    MeasurementValidity, PageCalibration, MAX_HOLES, MAX_RING_VERTICES, MAX_TOTAL_VERTICES,
};
use sunipdf_core::{Document, Engine, EngineError};

type Calibration = ([(f32, f32); 2], f64);

#[derive(Args)]
#[command(group(ArgGroup::new("measurement_changes").required(true).multiple(true)
    .args(["kind", "points", "holes", "clear_holes", "depth", "clear_depth", "calibration", "use_page_calibration", "unit", "precision"])))]
pub struct EditArgs {
    pub file: PathBuf,
    /// Physical page number, starting at 1.
    #[arg(long, value_parser = clap::value_parser!(usize))]
    pub page: usize,
    /// Zero-based annotation index from measurements, not its position in the exported list.
    #[arg(long)]
    pub index: usize,
    /// One complete measurement record from measurements JSON (maximum 64 KiB).
    #[arg(long)]
    pub expected_record: PathBuf,
    #[arg(long, value_parser = ["length", "polyline", "perimeter", "area", "volume", "angle"])]
    pub kind: Option<String>,
    /// Replacement displayed-page vertices: "x,y x,y ..."; omitted vertices are preserved.
    #[arg(long)]
    pub points: Option<String>,
    /// Replace all deduction rings: "x,y x,y ...". Repeat for each hole; omission preserves existing holes.
    #[arg(long = "hole", conflicts_with = "clear_holes")]
    pub holes: Vec<String>,
    /// Explicitly remove every deduction ring; does not change the outer boundary or depth.
    #[arg(long, conflicts_with = "holes")]
    pub clear_holes: bool,
    /// Replace physical depth; requires --depth-unit. Omission preserves the existing depth.
    #[arg(long, requires = "depth_unit", conflicts_with = "clear_depth")]
    pub depth: Option<f64>,
    /// Unit of the replacement depth, independent of --unit; requires --depth.
    #[arg(long, requires = "depth", conflicts_with = "clear_depth", value_parser = ["mm", "cm", "m", "in", "ft"])]
    pub depth_unit: Option<String>,
    /// Explicitly remove depth, for example with --kind area; a volume must retain a depth.
    #[arg(long, conflicts_with_all = ["depth", "depth_unit"])]
    pub clear_depth: bool,
    /// Replacement reference points: "x,y x,y"; requires --known-distance.
    #[arg(
        long,
        requires = "known_distance",
        conflicts_with = "use_page_calibration"
    )]
    pub calibration: Option<String>,
    /// Replacement reference distance in the resulting unit.
    #[arg(
        long,
        requires = "calibration",
        conflicts_with = "use_page_calibration"
    )]
    pub known_distance: Option<f64>,
    /// Explicitly replace this annotation's calibration with the saved page scale.
    #[arg(long, conflicts_with_all = ["calibration", "known_distance"])]
    pub use_page_calibration: bool,
    /// Convert the existing scale to this unit unless an explicit calibration is supplied.
    #[arg(long, value_parser = ["mm", "cm", "m", "in", "ft"])]
    pub unit: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub precision: Option<u8>,
    /// New PDF copy; existing destinations are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

fn invalid_record() -> EngineError {
    EngineError::Other("expected record must be one complete supported measurements JSON record with a stable annotation identity".into())
}

fn json_points(value: &Value, maximum: usize) -> Result<Vec<(f32, f32)>, EngineError> {
    let points = value
        .as_array()
        .filter(|p| p.len() >= 2 && p.len() <= maximum)
        .ok_or_else(invalid_record)?;
    points
        .iter()
        .map(|point| {
            let pair = point
                .as_array()
                .filter(|p| p.len() == 2)
                .ok_or_else(invalid_record)?;
            let coordinate = |value: &Value| -> Result<f32, EngineError> {
                let number = value.as_f64().ok_or_else(invalid_record)? as f32;
                number
                    .is_finite()
                    .then_some(number)
                    .ok_or_else(invalid_record)
            };
            Ok((coordinate(&pair[0])?, coordinate(&pair[1])?))
        })
        .collect()
}

pub(crate) fn parse_record(value: Value) -> Result<MeasurementRecord, EngineError> {
    let version = value["version"].as_u64();
    let validity = match version {
        Some(1) if value.get("validity").is_none() || value["validity"] == "legacy_unverified" => {
            MeasurementValidity::LegacyUnverified
        }
        Some(2 | 3) => {
            serde_json::from_value(value["validity"].clone()).map_err(|_| invalid_record())?
        }
        _ => return Err(invalid_record()),
    };
    let annotation_id = value["annotation_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .ok_or_else(invalid_record)?;
    let holes = match value.get("holes") {
        Some(Value::Array(rings)) if rings.len() <= MAX_HOLES => rings
            .iter()
            .map(|ring| json_points(ring, MAX_RING_VERTICES))
            .collect::<Result<Vec<_>, _>>()?,
        None if version != Some(3) => Vec::new(),
        _ => return Err(invalid_record()),
    };
    let depth = match value.get("depth") {
        Some(Value::Null) => None,
        Some(Value::Object(object))
            if object.len() == 2 && object.contains_key("value") && object.contains_key("unit") =>
        {
            Some(MeasurementDepth {
                value: object["value"].as_f64().ok_or_else(invalid_record)?,
                unit: object["unit"]
                    .as_str()
                    .and_then(MeasurementUnit::from_id)
                    .ok_or_else(invalid_record)?,
            })
        }
        None if version != Some(3) => None,
        _ => return Err(invalid_record()),
    };
    if version != Some(3) && (!holes.is_empty() || depth.is_some()) {
        return Err(invalid_record());
    }
    let record = MeasurementRecord {
        index: value["index"]
            .as_u64()
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(invalid_record)?,
        annotation_id: Some(annotation_id.into()),
        validity,
        measurement: Measurement {
            kind: value["kind"]
                .as_str()
                .and_then(MeasurementKind::from_id)
                .ok_or_else(invalid_record)?,
            points: json_points(&value["points"], 128)?,
            holes,
            depth,
            calibration: json_points(&value["calibration"]["points"], 2)?
                .try_into()
                .map_err(|_| invalid_record())?,
            known_distance: value["calibration"]["known_distance"]
                .as_f64()
                .ok_or_else(invalid_record)?,
            unit: value["calibration"]["unit"]
                .as_str()
                .and_then(MeasurementUnit::from_id)
                .ok_or_else(invalid_record)?,
            precision: value["precision"]
                .as_u64()
                .filter(|p| *p <= 4)
                .ok_or_else(invalid_record)? as usize,
        },
    };
    record.measurement.value()?;
    Ok(record)
}

fn read_record(path: &Path) -> Result<MeasurementRecord, EngineError> {
    const MAXIMUM: u64 = 64 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAXIMUM + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAXIMUM {
        return Err(EngineError::Other(
            "expected measurement record exceeds 64 KiB".into(),
        ));
    }
    let json = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    parse_record(serde_json::from_slice(json).map_err(|_| invalid_record())?)
}

pub fn points(spec: &str, maximum: usize) -> Result<Vec<(f32, f32)>, EngineError> {
    if spec.len() > maximum * 128 || spec.split_whitespace().take(maximum + 1).count() > maximum {
        return Err(EngineError::Other(format!(
            "measurement geometry is limited to {maximum} points"
        )));
    }
    annot_cmd::parse_stroke(spec)
}

pub fn holes(specs: &[String], outer_vertices: usize) -> Result<Vec<Vec<(f32, f32)>>, EngineError> {
    if specs.len() > MAX_HOLES {
        return Err(EngineError::Other(format!(
            "a surface is limited to {MAX_HOLES} deduction rings"
        )));
    }
    let mut total = outer_vertices;
    specs
        .iter()
        .map(|spec| {
            let ring = points(spec, MAX_RING_VERTICES)?;
            total = total.saturating_add(ring.len());
            if total > MAX_TOTAL_VERTICES {
                return Err(EngineError::Other(format!(
                    "a surface is limited to {MAX_TOTAL_VERTICES} total vertices"
                )));
            }
            Ok(ring)
        })
        .collect()
}

pub fn depth(
    value: Option<f64>,
    unit: Option<&str>,
) -> Result<Option<MeasurementDepth>, EngineError> {
    match (value, unit) {
        (None, None) => Ok(None),
        (Some(value), Some(unit)) if value.is_finite() && (1e-9..=1e9).contains(&value) => {
            Ok(Some(MeasurementDepth {
                value,
                unit: MeasurementUnit::from_id(unit).ok_or_else(invalid_record)?,
            }))
        }
        _ => Err(EngineError::Other(
            "depth requires a finite positive value from 1e-9 to 1e9 and an explicit --depth-unit"
                .into(),
        )),
    }
}

pub fn page_selection(page: Option<usize>, count: usize) -> Result<Vec<usize>, EngineError> {
    if let Some(page) = page {
        return Ok(vec![crate::page_index(page, count)?]);
    }
    if count > 1000 {
        return Err(EngineError::Other(
            "measurement export is limited to 1000 pages; select --page".into(),
        ));
    }
    Ok((0..count).collect())
}

pub fn saved_scale(
    doc: &Document,
    page: usize,
    unit: MeasurementUnit,
) -> Result<Calibration, EngineError> {
    let saved = doc.page_calibration(page)?.ok_or_else(|| EngineError::Other(
        "page has no persisted calibration; set-page-calibration first or provide an explicit calibration".into()))?;
    Ok((
        saved.points,
        saved.known_distance * saved.unit.metres() / unit.metres(),
    ))
}

pub fn angle_scale(points: &[(f32, f32)]) -> Result<Calibration, EngineError> {
    if points.len() != 3 {
        return Err(EngineError::Other(
            "angle requires three points; the middle point is its vertex".into(),
        ));
    }
    Ok(([points[0], points[1]], 1.0))
}

pub fn edit(engine: &Engine, args: EditArgs) -> Result<Value, EngineError> {
    if args.out.exists() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let expected = read_record(&args.expected_record)?;
    if expected.index != args.index {
        return Err(EngineError::Other(
            "expected record does not match --index".into(),
        ));
    }
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    let page = crate::page_index(args.page, doc.page_count())?;
    let target = doc.measurement_target(page, &expected)?;
    let explicit_calibration = args.use_page_calibration || args.calibration.is_some();
    let mut replacement = expected.measurement.clone();
    if let Some(kind) = args.kind {
        replacement.kind = MeasurementKind::from_id(&kind).ok_or_else(invalid_record)?;
    }
    if let Some(spec) = args.points {
        replacement.points = points(&spec, 128)?;
    }
    if args.clear_holes {
        replacement.holes.clear();
    } else if !args.holes.is_empty() {
        replacement.holes = holes(&args.holes, replacement.points.len())?;
    }
    if args.clear_depth {
        replacement.depth = None;
    } else if args.depth.is_some() || args.depth_unit.is_some() {
        replacement.depth = depth(args.depth, args.depth_unit.as_deref())?;
    }
    if let Some(unit) = args.unit {
        let unit = MeasurementUnit::from_id(&unit).ok_or_else(invalid_record)?;
        replacement.known_distance *= replacement.unit.metres() / unit.metres();
        replacement.unit = unit;
    }
    if let Some(precision) = args.precision {
        replacement.precision = usize::from(precision);
    }
    if args.use_page_calibration {
        (replacement.calibration, replacement.known_distance) =
            saved_scale(&doc, page, replacement.unit)?;
    } else if let Some(spec) = args.calibration {
        replacement.calibration = points(&spec, 2)?.try_into().map_err(|_| invalid_record())?;
        replacement.known_distance = args.known_distance.ok_or_else(invalid_record)?;
    } else if replacement.kind == MeasurementKind::Angle {
        (replacement.calibration, replacement.known_distance) = angle_scale(&replacement.points)?;
    } else if expected.measurement.kind == MeasurementKind::Angle {
        return Err(EngineError::Other(
            "changing an angle to a scaled measurement requires an explicit calibration".into(),
        ));
    }
    let index = if !expected.validity.is_current() && explicit_calibration {
        let calibration = PageCalibration {
            points: replacement.calibration,
            known_distance: replacement.known_distance,
            unit: replacement.unit,
        };
        doc.recalibrate_measurement_annotation(&target, &replacement, &calibration)?
    } else {
        doc.update_measurement_annotation(&target, &replacement)?
    };
    let record = MeasurementRecord {
        index,
        annotation_id: expected.annotation_id,
        measurement: replacement,
        validity: MeasurementValidity::Current,
    };
    let export_unavailable = !doc.properties().can_copy;
    let exported = if export_unavailable {
        Value::Null
    } else {
        record.result()?
    };
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(
        serde_json::json!({"out":args.out,"page":args.page,"index":index,"measurement":exported,"export_unavailable":export_unavailable,
        "original_unchanged":true,"encrypted":doc.properties().encrypted}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MeasurementRecord {
        MeasurementRecord {
            index: 3,
            annotation_id: Some("stable-identity".into()),
            validity: MeasurementValidity::Current,
            measurement: Measurement {
                kind: MeasurementKind::Area,
                points: vec![(10.0, 10.0), (50.0, 10.0), (50.0, 40.0)],
                holes: Vec::new(),
                depth: None,
                calibration: [(10.0, 10.0), (50.0, 10.0)],
                known_distance: 4.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            },
        }
    }

    #[test]
    fn exported_records_roundtrip_and_reject_incomplete_future_and_unidentified_inputs() {
        let record = sample();
        assert_eq!(parse_record(record.result().unwrap()).unwrap(), record);
        for (key, value) in [
            ("version", 4.into()),
            ("validity", Value::Null),
            ("validity", "unknown".into()),
            ("annotation_id", Value::Null),
            ("index", (-1).into()),
            ("precision", 5.into()),
            ("points", serde_json::json!([[1, 2], [3, 4], [null, 6]])),
        ] {
            let mut input = record.result().unwrap();
            input[key] = value;
            assert!(parse_record(input).is_err(), "accepted {key}");
        }
        let mut angle = record.clone();
        angle.measurement.kind = MeasurementKind::Angle;
        assert_eq!(parse_record(angle.result().unwrap()).unwrap(), angle);
        let mut legacy = record.result().unwrap();
        legacy["version"] = 1.into();
        assert!(parse_record(legacy.clone()).is_err());
        legacy.as_object_mut().unwrap().remove("validity");
        let parsed = parse_record(legacy).unwrap();
        assert_eq!(parsed.validity, MeasurementValidity::LegacyUnverified);
        assert_eq!(parsed.measurement, record.measurement);
    }

    #[test]
    fn inputs_and_page_selection_are_bounded_before_collection() {
        assert!(page_selection(None, usize::MAX).is_err());
        assert_eq!(page_selection(Some(1001), 1001).unwrap(), vec![1000]);
        assert_eq!(page_selection(None, 1000).unwrap().len(), 1000);
        assert!(points(&"1,2 ".repeat(129), 128).is_err());
        assert!(points(&" ".repeat(16_385), 128).is_err());
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("oversized.json");
        let mut utf8 = vec![0xef, 0xbb, 0xbf];
        utf8.extend(serde_json::to_vec(&sample().result().unwrap()).unwrap());
        std::fs::write(&path, utf8).unwrap();
        assert_eq!(read_record(&path).unwrap(), sample());
        std::fs::write(&path, vec![b' '; 65_537]).unwrap();
        assert!(read_record(&path)
            .unwrap_err()
            .to_string()
            .contains("64 KiB"));
    }

    #[test]
    fn volume_record_roundtrips_without_losing_deductions_or_physical_depth() {
        let mut record = sample();
        record.measurement.kind = MeasurementKind::Volume;
        record.measurement.points =
            vec![(10.0, 10.0), (110.0, 10.0), (110.0, 110.0), (10.0, 110.0)];
        record.measurement.holes =
            vec![vec![(30.0, 30.0), (50.0, 30.0), (50.0, 80.0), (30.0, 80.0)]];
        record.measurement.depth = Some(MeasurementDepth {
            value: 20.0,
            unit: MeasurementUnit::Centimetres,
        });
        let result = record.result().unwrap();
        assert_eq!(result["version"], 3);
        assert_eq!(parse_record(result.clone()).unwrap(), record);
        for key in ["holes", "depth"] {
            let mut missing = result.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(parse_record(missing).is_err(), "missing {key}");
        }
        let mut old_version = result.clone();
        old_version["version"] = 2.into();
        assert!(parse_record(old_version).is_err());
        for replacement in [
            serde_json::json!({"value":20,"unit":"m2"}),
            serde_json::json!({"value":0,"unit":"cm"}),
            serde_json::json!({"value":20,"unit":"cm","other":1}),
        ] {
            let mut malformed = result.clone();
            malformed["depth"] = replacement;
            assert!(parse_record(malformed).is_err());
        }
        assert!(holes(&vec!["1,1 2,1 2,2".into(); MAX_HOLES + 1], 4).is_err());
        assert!(holes(&["1,1 2,1 2,2".into()], MAX_TOTAL_VERTICES).is_err());
        assert!(depth(Some(20.0), None).is_err());
        assert!(depth(None, Some("cm")).is_err());
        assert!(depth(Some(f64::NAN), Some("cm")).is_err());
    }
}
