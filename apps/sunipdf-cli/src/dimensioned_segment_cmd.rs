use crate::{measurement_cmd, password_input};
use clap::{ArgGroup, Args};
use std::path::PathBuf;
use sunipdf_core::{
    measurement::{
        normalize_rectangle_rotation, parse_measurement_distance, DimensionedSegment,
        MeasurementUnit, RectangleScale,
    },
    Engine, EngineError, PageCalibration,
};

#[derive(Args)]
#[command(group(ArgGroup::new("segment_scale").required(true).multiple(false)
    .args(["calibration", "use_page_calibration", "region", "automatic_scale"])))]
pub struct SegmentArgs {
    pub file: PathBuf,
    /// Start "x,y" in displayed-page points; zero degrees points right, 90 points down.
    #[arg(long)]
    pub start: String,
    /// Positive real length in --unit; feet/inches notation is also accepted.
    #[arg(long, allow_hyphen_values = true)]
    pub length: String,
    /// Clockwise angle around --start; finite angles normalize to [0, 360).
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true, value_parser = rotation_degrees)]
    pub rotation_degrees: f64,
    #[arg(long, value_parser = ["mm", "cm", "m", "in", "ft"], default_value = "m")]
    pub unit: String,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub precision: u8,
    /// Two displayed-page points "x,y x,y" defining a fresh reference.
    #[arg(long, requires = "known_distance")]
    pub calibration: Option<String>,
    /// Real reference distance in --calibration-unit, or --unit when omitted.
    #[arg(long, requires = "calibration", allow_hyphen_values = true,
        conflicts_with_all = ["use_page_calibration", "region", "automatic_scale"])]
    pub known_distance: Option<String>,
    /// Unit of the explicit reference, independent of the segment length.
    #[arg(long, requires = "calibration", value_parser = ["mm", "cm", "m", "in", "ft"],
        conflicts_with_all = ["use_page_calibration", "region", "automatic_scale"])]
    pub calibration_unit: Option<String>,
    /// Explicitly use the saved page scale, including when crossing named regions.
    #[arg(long)]
    pub use_page_calibration: bool,
    /// Use this saved region ID; the entire segment must fit inside it.
    #[arg(long)]
    pub region: Option<String>,
    /// Select an unambiguous region, otherwise the page scale; refuse crossing or overlaps.
    #[arg(long)]
    pub automatic_scale: bool,
    /// Physical page number, starting at 1.
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long)]
    pub author: Option<String>,
    /// New PDF copy; existing destinations are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

fn rotation_degrees(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "enter a finite angle".to_owned())?;
    normalize_rectangle_rotation(value).map_err(|error| error.to_string())
}

pub fn segment(engine: &Engine, args: SegmentArgs) -> Result<serde_json::Value, EngineError> {
    if args.out.exists() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let unit = MeasurementUnit::from_id(&args.unit)
        .ok_or_else(|| EngineError::Other("invalid measurement unit".into()))?;
    let start: [(f32, f32); 1] = measurement_cmd::points(&args.start, 1)?
        .try_into()
        .map_err(|_| EngineError::Other("segment start requires exactly one point".into()))?;
    let length = parse_measurement_distance(&args.length, unit)?;
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    let page = crate::page_index(args.page, doc.page_count())?;
    let scale = if let Some(points) = &args.calibration {
        let calibration_unit = args
            .calibration_unit
            .as_deref()
            .and_then(MeasurementUnit::from_id)
            .unwrap_or(unit);
        let reference = PageCalibration {
            points: measurement_cmd::points(points, 2)?
                .try_into()
                .map_err(|_| EngineError::Other("calibration requires two points".into()))?,
            known_distance: parse_measurement_distance(
                args.known_distance.as_deref().ok_or_else(|| {
                    EngineError::Other("calibration requires a known distance".into())
                })?,
                calibration_unit,
            )?,
            unit: calibration_unit,
        };
        RectangleScale::Local(doc.local_calibration_reference(page, &reference)?)
    } else if args.use_page_calibration {
        RectangleScale::Page
    } else if let Some(region) = args.region {
        RectangleScale::Region(region)
    } else if args.automatic_scale {
        RectangleScale::Automatic
    } else {
        return Err(EngineError::Other(
            "select an explicit segment scale source".into(),
        ));
    };
    let request = DimensionedSegment {
        start: start[0],
        length,
        rotation_degrees: args.rotation_degrees,
        unit,
        precision: usize::from(args.precision),
        scale,
    };
    let measurement = doc.dimensioned_segment(page, &request)?;
    let export_unavailable = !doc.properties().can_copy;
    let exported = if export_unavailable {
        serde_json::Value::Null
    } else {
        let mut result = measurement.result()?;
        result["version"] = 2.into();
        result["validity"] = "current".into();
        result["historical_value"] = serde_json::Value::Null;
        result["historical_label"] = serde_json::Value::Null;
        result["historical_area"] = serde_json::Value::Null;
        result
    };
    let index = doc.add_dimensioned_segment(page, &request, args.author.as_deref())?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(serde_json::json!({
        "out": args.out, "page": args.page, "index": index,
        "shape": "segment", "vertices": 2, "segments": 1,
        "persistent_shape_constraint": false,
        "measurement": exported, "export_unavailable": export_unavailable,
        "original_unchanged": true, "encrypted": doc.properties().encrypted,
        "rotation_degrees": normalize_rectangle_rotation(args.rotation_degrees)?,
    }))
}
