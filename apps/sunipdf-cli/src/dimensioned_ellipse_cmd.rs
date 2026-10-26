use crate::{measurement_cmd, password_input};
use clap::{ArgGroup, Args};
use std::path::PathBuf;
use sunipdf_core::{
    measurement::{
        normalize_rectangle_rotation, parse_measurement_distance, DimensionedEllipse,
        MeasurementUnit, RectangleScale, DIMENSIONED_ELLIPSE_SEGMENTS,
    },
    Engine, EngineError, PageCalibration,
};

#[derive(Args)]
#[command(group(ArgGroup::new("ellipse_scale").required(true).multiple(false)
    .args(["calibration", "use_page_calibration", "region", "automatic_scale"])))]
pub struct CommonArgs {
    pub file: PathBuf,
    /// Center "x,y" in displayed-page points, after page crop and rotation.
    #[arg(long)]
    pub center: String,
    /// Clockwise angle about the center. Finite angles normalize to [0, 360).
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true, value_parser = rotation)]
    pub rotation_degrees: f64,
    #[arg(long, value_parser = ["mm", "cm", "m", "in", "ft"], default_value = "m")]
    pub unit: String,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub precision: u8,
    /// Two displayed-page points "x,y x,y" defining a fresh reference.
    #[arg(long, requires = "known_distance")]
    pub calibration: Option<String>,
    /// Positive reference distance in --calibration-unit, or --unit when omitted.
    #[arg(long, requires = "calibration", allow_hyphen_values = true,
        conflicts_with_all = ["use_page_calibration", "region", "automatic_scale"])]
    pub known_distance: Option<String>,
    #[arg(long, requires = "calibration", value_parser = ["mm", "cm", "m", "in", "ft"],
        conflicts_with_all = ["use_page_calibration", "region", "automatic_scale"])]
    pub calibration_unit: Option<String>,
    /// Explicitly use the saved page scale, including when crossing named regions.
    #[arg(long)]
    pub use_page_calibration: bool,
    /// Use this saved region ID; the complete polygon must fit inside it.
    #[arg(long)]
    pub region: Option<String>,
    /// Select an unambiguous region, otherwise the page scale; refuse crossings and overlaps.
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

#[derive(Args)]
pub struct EllipseArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    /// Positive full diameter along the local X axis, in --unit.
    #[arg(long, allow_hyphen_values = true)]
    pub diameter_x: String,
    /// Positive full diameter along the local Y axis, in --unit.
    #[arg(long, allow_hyphen_values = true)]
    pub diameter_y: String,
}

#[derive(Args)]
pub struct CircleArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    /// Positive full diameter in --unit; decimal, fraction and feet/inches input are accepted.
    #[arg(long, allow_hyphen_values = true)]
    pub diameter: String,
}

fn rotation(value: &str) -> Result<f64, String> {
    let degrees = value
        .parse::<f64>()
        .map_err(|_| "enter a finite angle".to_owned())?;
    normalize_rectangle_rotation(degrees).map_err(|_| "enter a finite angle".to_owned())
}

pub fn ellipse(engine: &Engine, args: EllipseArgs) -> Result<serde_json::Value, EngineError> {
    construct(
        engine,
        args.common,
        &args.diameter_x,
        &args.diameter_y,
        "ellipse",
    )
}

pub fn circle(engine: &Engine, args: CircleArgs) -> Result<serde_json::Value, EngineError> {
    construct(
        engine,
        args.common,
        &args.diameter,
        &args.diameter,
        "circle",
    )
}

fn construct(
    engine: &Engine,
    args: CommonArgs,
    diameter_x: &str,
    diameter_y: &str,
    shape: &str,
) -> Result<serde_json::Value, EngineError> {
    if args.out.exists() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let unit = MeasurementUnit::from_id(&args.unit)
        .ok_or_else(|| EngineError::Other("invalid measurement unit".into()))?;
    let center: [(f32, f32); 1] = measurement_cmd::points(&args.center, 1)?
        .try_into()
        .map_err(|_| EngineError::Other("ellipse center requires exactly one point".into()))?;
    let diameter_x = parse_measurement_distance(diameter_x, unit)?;
    let diameter_y = parse_measurement_distance(diameter_y, unit)?;
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    let page = crate::page_index(args.page, doc.page_count())?;
    let scale = if let Some(points) = &args.calibration {
        let calibration_unit = args
            .calibration_unit
            .as_deref()
            .and_then(MeasurementUnit::from_id)
            .unwrap_or(unit);
        let calibration = PageCalibration {
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
        RectangleScale::Local(doc.local_calibration_reference(page, &calibration)?)
    } else if args.use_page_calibration {
        RectangleScale::Page
    } else if let Some(region) = args.region {
        RectangleScale::Region(region)
    } else if args.automatic_scale {
        RectangleScale::Automatic
    } else {
        return Err(EngineError::Other(
            "select an explicit ellipse scale source".into(),
        ));
    };
    let request = DimensionedEllipse {
        center: center[0],
        diameter_x,
        diameter_y,
        rotation_degrees: args.rotation_degrees,
        unit,
        precision: usize::from(args.precision),
        scale,
    };
    let measurement = doc.dimensioned_ellipse(page, &request)?;
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
    let index = doc.add_dimensioned_ellipse(page, &request, args.author.as_deref())?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(serde_json::json!({
        "out": args.out, "page": args.page, "index": index,
        "shape": shape, "segments": DIMENSIONED_ELLIPSE_SEGMENTS,
        "approximation": "polygonal_area", "persistent_shape_constraint": false,
        "measurement": exported, "export_unavailable": export_unavailable,
        "original_unchanged": true, "encrypted": doc.properties().encrypted,
        "rotation_degrees": normalize_rectangle_rotation(args.rotation_degrees)?,
    }))
}

#[cfg(test)]
mod tests {
    use crate::{tests::parse_cli, Command};

    #[test]
    fn circle_and_ellipse_require_complete_dimensions_one_scale_and_new_output() {
        for (action, dimensions) in [
            ("dimensioned-circle", vec!["--diameter", "2"]),
            (
                "dimensioned-ellipse",
                vec!["--diameter-x", "2", "--diameter-y", "3"],
            ),
        ] {
            let mut base = vec![
                "sunipdf-cli",
                action,
                "source.pdf",
                "--center",
                "220,260",
                "--out",
                "copy.pdf",
            ];
            base.extend(dimensions);
            assert!(parse_cli(base.clone()).is_err());
            for scale in [
                vec!["--use-page-calibration"],
                vec!["--automatic-scale"],
                vec!["--region", "detail"],
                vec![
                    "--calibration",
                    "40,40 140,40",
                    "--known-distance",
                    "100",
                    "--calibration-unit",
                    "cm",
                ],
            ] {
                let parsed = parse_cli(base.iter().copied().chain(scale)).unwrap();
                assert_eq!(
                    crate::output_path(&parsed.command),
                    Some(std::path::Path::new("copy.pdf"))
                );
                let common = match parsed.command {
                    Command::DimensionedCircle(args) => args.common,
                    Command::DimensionedEllipse(args) => args.common,
                    _ => panic!("dimensioned shape command"),
                };
                assert_eq!(common.rotation_degrees, 0.0);
                assert_eq!(common.precision, 2);
                assert_eq!(common.page, 1);
            }
            for invalid in [
                vec!["--calibration", "40,40 140,40"],
                vec!["--known-distance", "1", "--use-page-calibration"],
                vec!["--calibration-unit", "cm", "--use-page-calibration"],
                vec!["--use-page-calibration", "--automatic-scale"],
                vec!["--use-page-calibration", "--region", "detail"],
                vec!["--automatic-scale", "--region", "detail"],
                vec!["--use-page-calibration", "--precision", "5"],
                vec!["--use-page-calibration", "--unit", "yards"],
                vec!["--use-page-calibration", "--rotation-degrees", "NaN"],
                vec!["--use-page-calibration", "--rotation-degrees", "inf"],
                vec!["--use-page-calibration", "--rotation-degrees", "-inf"],
            ] {
                assert!(parse_cli(base.iter().copied().chain(invalid)).is_err());
            }
            for (angle, expected) in [("-90", 270.0), ("393.5", 33.5), ("720", 0.0)] {
                let parsed = parse_cli(base.iter().copied().chain([
                    "--use-page-calibration",
                    "--rotation-degrees",
                    angle,
                ]))
                .unwrap();
                let rotation = match parsed.command {
                    Command::DimensionedCircle(args) => args.common.rotation_degrees,
                    Command::DimensionedEllipse(args) => args.common.rotation_degrees,
                    _ => unreachable!(),
                };
                assert_eq!(rotation, expected);
            }
            let without_output = base
                .iter()
                .copied()
                .filter(|arg| !["--out", "copy.pdf"].contains(arg));
            assert!(parse_cli(without_output.chain(["--use-page-calibration"])).is_err());
        }
        assert!(parse_cli([
            "sunipdf-cli",
            "dimensioned-ellipse",
            "source.pdf",
            "--center",
            "220,260",
            "--diameter-x",
            "2",
            "--use-page-calibration",
            "--out",
            "copy.pdf"
        ])
        .is_err());
        assert!(parse_cli([
            "sunipdf-cli",
            "dimensioned-circle",
            "source.pdf",
            "--center",
            "220,260",
            "--diameter",
            "2",
            "--diameter-y",
            "3",
            "--use-page-calibration",
            "--out",
            "copy.pdf"
        ])
        .is_err());
    }
}
