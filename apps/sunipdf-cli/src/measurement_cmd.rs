use crate::{annot_cmd, password_input};
use clap::{ArgGroup, Args};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementRecord, MeasurementUnit};
use sunipdf_core::{Document, Engine, EngineError};

type Calibration = ([(f32, f32); 2], f64);

#[derive(Args)]
#[command(group(ArgGroup::new("measurement_changes").required(true).multiple(true)
    .args(["kind", "points", "calibration", "use_page_calibration", "unit", "precision"])))]
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
    #[arg(long, value_parser = ["length", "polyline", "perimeter", "area", "angle"])]
    pub kind: Option<String>,
    /// Replacement displayed-page vertices: "x,y x,y ..."; omitted vertices are preserved.
    #[arg(long)]
    pub points: Option<String>,
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

fn parse_record(value: Value) -> Result<MeasurementRecord, EngineError> {
    if value["version"].as_u64() != Some(1) {
        return Err(invalid_record());
    }
    let annotation_id = value["annotation_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .ok_or_else(invalid_record)?;
    let record = MeasurementRecord {
        index: value["index"]
            .as_u64()
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(invalid_record)?,
        annotation_id: Some(annotation_id.into()),
        measurement: Measurement {
            kind: value["kind"]
                .as_str()
                .and_then(MeasurementKind::from_id)
                .ok_or_else(invalid_record)?,
            points: json_points(&value["points"], 128)?,
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
    let mut replacement = expected.measurement.clone();
    if let Some(kind) = args.kind {
        replacement.kind = MeasurementKind::from_id(&kind).ok_or_else(invalid_record)?;
    }
    if let Some(spec) = args.points {
        replacement.points = points(&spec, 128)?;
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
    let index = doc.update_measurement_annotation(&target, &replacement)?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    let record = MeasurementRecord {
        index,
        annotation_id: expected.annotation_id,
        measurement: replacement,
    };
    let export_unavailable = !doc.properties().can_copy;
    let exported = if export_unavailable {
        Value::Null
    } else {
        record.result()?
    };
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
            measurement: Measurement {
                kind: MeasurementKind::Area,
                points: vec![(10.0, 10.0), (50.0, 10.0), (50.0, 40.0)],
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
            ("version", 2.into()),
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
}
