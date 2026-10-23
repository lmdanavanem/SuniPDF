use crate::{measurement_cmd, password_input};
use clap::Args;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::{
    measurement::{
        parse_measurement_distance, MeasurementUnit, MeasurementViewport, PageCalibration,
        RecalibrationRequest, SnapKind,
    },
    Engine, EngineError, PointRect,
};

#[derive(Args)]
pub struct ViewportsArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long,help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}
#[derive(Args)]
pub struct SetViewportsArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    /// JSON array of current regions returned by measurement-viewports.
    #[arg(long)]
    pub expected: PathBuf,
    /// Complete replacement JSON array (maximum 64 named calibrated regions).
    #[arg(long)]
    pub regions: PathBuf,
    /// New PDF copy; existing output files are always refused.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long,help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}
#[derive(Args)]
pub struct RecalibrateArgs {
    pub file: PathBuf,
    /// JSON array: page (1-based), calibration, update_page_default, measurements (exact exported records).
    #[arg(long)]
    pub requests: PathBuf,
    /// Apply the preview to a new PDF copy. Omit for a read-only preview.
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long,help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}
#[derive(Args)]
pub struct SnapArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    /// Displayed-page point "x,y".
    #[arg(long)]
    pub point: String,
    #[arg(long, default_value_t = 5.0)]
    pub tolerance: f32,
    #[arg(long,help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

fn invalid() -> EngineError {
    EngineError::Other(
        "invalid measurement request: use explicit supported region/calibration/measurement JSON"
            .into(),
    )
}
fn read(path: &Path, max: usize) -> Result<Value, EngineError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(invalid());
    }
    serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
        .map_err(|_| invalid())
}
fn keys(value: &Value, allowed: &[&str]) -> Result<(), EngineError> {
    let object = value.as_object().ok_or_else(invalid)?;
    if object.len() != allowed.len() || object.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(invalid());
    }
    Ok(())
}
fn number(value: &Value) -> Result<f32, EngineError> {
    value
        .as_f64()
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .ok_or_else(invalid)
}
fn calibration(value: &Value) -> Result<PageCalibration, EngineError> {
    keys(value, &["points", "known_distance", "unit"])?;
    let unit = value["unit"]
        .as_str()
        .and_then(MeasurementUnit::from_id)
        .ok_or_else(invalid)?;
    let points = value["points"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(invalid)?;
    let point = |p: &Value| -> Result<(f32, f32), EngineError> {
        let p = p.as_array().filter(|a| a.len() == 2).ok_or_else(invalid)?;
        Ok((number(&p[0])?, number(&p[1])?))
    };
    let known_distance = if let Some(text) = value["known_distance"].as_str() {
        parse_measurement_distance(text, unit)?
    } else {
        value["known_distance"].as_f64().ok_or_else(invalid)?
    };
    let calibration = PageCalibration {
        points: [point(&points[0])?, point(&points[1])?],
        known_distance,
        unit,
    };
    calibration.validate()?;
    Ok(calibration)
}
fn regions(value: &Value) -> Result<Vec<MeasurementViewport>, EngineError> {
    value
        .as_array()
        .filter(|a| a.len() <= 64)
        .ok_or_else(invalid)?
        .iter()
        .map(|v| {
            keys(v, &["id", "name", "bounds", "calibration"])?;
            let b = v["bounds"]
                .as_array()
                .filter(|a| a.len() == 4)
                .ok_or_else(invalid)?;
            Ok(MeasurementViewport {
                id: v["id"].as_str().ok_or_else(invalid)?.into(),
                name: v["name"].as_str().ok_or_else(invalid)?.into(),
                bounds: PointRect {
                    left: number(&b[0])?,
                    bottom: number(&b[1])?,
                    right: number(&b[2])?,
                    top: number(&b[3])?,
                },
                calibration: calibration(&v["calibration"])?,
            })
        })
        .collect()
}
fn region_json(v: &MeasurementViewport) -> Value {
    json!({"id":v.id,"name":v.name,"bounds":[v.bounds.left,v.bounds.bottom,v.bounds.right,v.bounds.top],"calibration":{"points":v.calibration.points,"known_distance":v.calibration.known_distance,"unit":v.calibration.unit.id()}})
}
fn new_output(out: &Path) -> Result<(), EngineError> {
    if out.exists() {
        return Err(EngineError::AlreadyExists(out.to_path_buf()));
    }
    Ok(())
}

pub fn viewports(engine: &Engine, args: ViewportsArgs) -> Result<Value, EngineError> {
    let doc = engine.open(&args.file, args.password.as_deref())?;
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    let regions = doc.measurement_viewports(crate::page_index(args.page, doc.page_count())?)?;
    Ok(
        json!({"page":args.page,"regions":regions.iter().map(region_json).collect::<Vec<_>>(),"existing_measurements_changed":false}),
    )
}
pub fn set_viewports(engine: &Engine, args: SetViewportsArgs) -> Result<Value, EngineError> {
    new_output(&args.out)?;
    let expected = regions(&read(&args.expected, 128 * 1024)?)?;
    let regions = regions(&read(&args.regions, 128 * 1024)?)?;
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    let page = crate::page_index(args.page, doc.page_count())?;
    doc.set_measurement_viewports(page, &expected, &regions)?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(
        json!({"out":args.out,"page":args.page,"regions":regions.iter().map(region_json).collect::<Vec<_>>(),"original_unchanged":true,"encrypted":doc.properties().encrypted,"existing_measurements_changed":false}),
    )
}
pub fn recalibrate(engine: &Engine, args: RecalibrateArgs) -> Result<Value, EngineError> {
    if let Some(out) = &args.out {
        new_output(out)?;
    }
    let data = read(&args.requests, 4 * 1024 * 1024)?;
    let requests = data
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 512)
        .ok_or_else(invalid)?;
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    let requests = requests
        .iter()
        .map(|v| {
            keys(
                v,
                &["page", "calibration", "update_page_default", "measurements"],
            )?;
            let page = v["page"]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(invalid)?;
            let expected = v["measurements"]
                .as_array()
                .filter(|a| a.len() <= 4096)
                .ok_or_else(invalid)?;
            Ok(RecalibrationRequest {
                page: crate::page_index(page, doc.page_count())?,
                calibration: calibration(&v["calibration"])?,
                update_page_default: v["update_page_default"].as_bool().ok_or_else(invalid)?,
                measurements: expected
                    .iter()
                    .cloned()
                    .map(measurement_cmd::parse_record)
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, EngineError>>()?;
    let plan = doc.preview_recalibration(&requests)?;
    let changes:Vec<_>=plan.changes().iter().map(|c|json!({"page":c.page+1,"index":c.index,"before":c.before,"after":c.after,"unit":c.unit.id(),"kind":c.kind.id()})).collect();
    if let Some(out) = &args.out {
        doc.apply_recalibration(&plan)?;
        sunipdf_core::write_new_copy(out, &doc.snapshot()?)?;
    }
    Ok(
        json!({"out":args.out,"applied":args.out.is_some(),"pages":plan.page_count(),"changes":changes,"original_unchanged":true,"encrypted":doc.properties().encrypted}),
    )
}
pub fn snap(engine: &Engine, args: SnapArgs) -> Result<Value, EngineError> {
    let doc = engine.open(&args.file, args.password.as_deref())?;
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    let points = measurement_cmd::points(&args.point, 1)?;
    if points.len() != 1
        || !args.tolerance.is_finite()
        || !(0.0..=100.0).contains(&args.tolerance)
        || args.tolerance == 0.0
    {
        return Err(invalid());
    }
    let geometry = doc.vector_snap_geometry(crate::page_index(args.page, doc.page_count())?)?;
    let point=geometry.nearest(points[0],args.tolerance).map(|s|json!({"point":s.point,"kind":match s.kind {SnapKind::Endpoint=>"endpoint",SnapKind::Intersection=>"intersection"}}));
    Ok(
        json!({"page":args.page,"snap":point,"endpoints":geometry.endpoint_count(),"segments":geometry.segment_count(),"skipped_clipped_objects":geometry.skipped_clipped_objects,"coordinate_system":"display-bottom-left"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_request_requires_explicit_scale_and_rejects_unknown_fields() {
        let v = json!([{"id":"detail-a","name":"Detail A","bounds":[0,0,200,200],"calibration":{"points":[[10,10],[110,10]],"known_distance":"5' 6\"","unit":"ft"}}]);
        assert_eq!(regions(&v).unwrap()[0].calibration.known_distance, 5.5);
        let mut invalid = v.clone();
        invalid[0]["unknown"] = true.into();
        assert!(regions(&invalid).is_err());
        invalid = v;
        invalid[0]["calibration"]["known_distance"] = "0".into();
        assert!(regions(&invalid).is_err());
        assert!(regions(&json!({})).is_err());
    }
}
