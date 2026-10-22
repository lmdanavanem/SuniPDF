use crate::{annot_cmd, password_input};
use clap::Args;
use std::path::{Path, PathBuf};
use sunipdf_core::{measurement::MeasurementUnit, Engine, EngineError, PageCalibration};

#[derive(Args)]
pub struct ReadArgs {
    pub file: PathBuf,
    /// Physical page number, starting at 1. Omit to read at most 1000 pages.
    #[arg(long, value_parser = positive_page)]
    pub page: Option<usize>,
    #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct SetArgs {
    pub file: PathBuf,
    /// Physical page number, starting at 1.
    #[arg(long, value_parser = positive_page)]
    pub page: usize,
    /// Two displayed-page points delimiting the known distance: "x,y x,y".
    #[arg(long)]
    pub calibration: String,
    /// Positive real distance between the two reference points, in --unit.
    #[arg(long, value_parser = positive_distance)]
    pub known_distance: f64,
    #[arg(long, value_parser = ["mm", "cm", "m", "in", "ft"], default_value = "m")]
    pub unit: String,
    /// New PDF copy; existing destinations are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct RemoveArgs {
    pub file: PathBuf,
    /// Physical page number, starting at 1.
    #[arg(long, value_parser = positive_page)]
    pub page: usize,
    /// New PDF copy; existing destinations are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "page numbers start at 1".into())
}

fn positive_distance(value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|distance| distance.is_finite() && (1e-9..=1e9).contains(distance))
        .ok_or_else(|| "known distance must be finite and between 1e-9 and 1e9".into())
}

fn ensure_new_copy(out: &Path) -> Result<(), EngineError> {
    if out.exists() {
        return Err(EngineError::AlreadyExists(out.to_path_buf()));
    }
    Ok(())
}

fn record(calibration: &PageCalibration) -> serde_json::Value {
    serde_json::json!({
        "points": calibration.points,
        "known_distance": calibration.known_distance,
        "unit": calibration.unit.id(),
    })
}

pub fn read(engine: &Engine, args: ReadArgs) -> Result<serde_json::Value, EngineError> {
    let doc = engine.open(&args.file, args.password.as_deref())?;
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    let pages = if let Some(page) = args.page {
        vec![crate::page_index(page, doc.page_count())?]
    } else {
        if doc.page_count() > 1000 {
            return Err(EngineError::Other(
                "page calibration export is limited to 1000 pages; select --page".into(),
            ));
        }
        (0..doc.page_count()).collect()
    };
    let mut records = Vec::with_capacity(pages.len());
    for page in pages {
        let calibration = doc.page_calibration(page)?;
        records.push(serde_json::json!({
            "page": page + 1,
            "calibration": calibration.as_ref().map(record),
        }));
    }
    Ok(serde_json::json!({"pages": records, "applies_to": "new_measurements"}))
}

pub fn set(engine: &Engine, args: SetArgs) -> Result<serde_json::Value, EngineError> {
    ensure_new_copy(&args.out)?;
    if args.calibration.len() > 256 {
        return Err(EngineError::Other(
            "calibration requires two points in at most 256 bytes".into(),
        ));
    }
    let calibration = PageCalibration {
        points: annot_cmd::parse_stroke(&args.calibration)?
            .try_into()
            .map_err(|_| EngineError::Other("calibration requires two points".into()))?,
        known_distance: args.known_distance,
        unit: MeasurementUnit::from_id(&args.unit)
            .ok_or_else(|| EngineError::Other("invalid calibration unit".into()))?,
    };
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    doc.set_page_calibration(
        crate::page_index(args.page, doc.page_count())?,
        Some(&calibration),
    )?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(serde_json::json!({
        "out": args.out, "page": args.page, "calibration": record(&calibration),
        "applies_to": "new_measurements", "existing_measurements_changed": false,
        "original_unchanged": true, "encrypted": doc.properties().encrypted,
    }))
}

pub fn remove(engine: &Engine, args: RemoveArgs) -> Result<serde_json::Value, EngineError> {
    ensure_new_copy(&args.out)?;
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    doc.set_page_calibration(crate::page_index(args.page, doc.page_count())?, None)?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(serde_json::json!({
        "out": args.out, "page": args.page, "calibration": null,
        "applies_to": "new_measurements", "existing_measurements_changed": false,
        "original_unchanged": true, "encrypted": doc.properties().encrypted,
    }))
}
