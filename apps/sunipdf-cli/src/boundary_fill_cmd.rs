use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::boundary_fill::{
    BoundaryClosure, BoundaryFillJob, BoundaryFillMeasurementOptions, BoundaryFillOptions,
};
use sunipdf_core::measurement::{MeasurementUnit, RectangleScale};
use sunipdf_core::{Document, Engine, EngineError, PageCalibration};
use zeroize::Zeroize;

const MAX_SOURCE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_REVIEW_BYTES: u64 = 256 * 1024;

#[derive(Args)]
pub struct RegionFillArgs {
    #[command(subcommand)]
    command: RegionFillCommand,
}

#[derive(Args)]
struct Source {
    /// Saved PDF, at most 128 MiB. Use --password-env or --password-stdin if needed.
    file: PathBuf,
    #[arg(skip)]
    password: Option<String>,
}

impl Drop for Source {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

#[derive(Subcommand)]
enum RegionFillCommand {
    /// Inspect an enclosed region. No measurement is accepted or written automatically.
    Inspect {
        #[command(flatten)]
        source: Source,
        /// Physical page number, starting at 1.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Seed x,y in displayed-page points, with a bottom-left origin.
        #[arg(long, value_parser = parse_seed, allow_hyphen_values = true)]
        seed: [f32; 2],
        /// Fixed raster pixels per displayed PDF point, from 0.25 to 2.
        #[arg(long, value_parser = parse_scale)]
        scale: Option<f32>,
        /// Grayscale boundary threshold. The core validates the supported range.
        #[arg(long)]
        threshold: Option<u8>,
        /// Explicit one-pixel closure x1,y1,x2,y2; repeat for up to 32 segments.
        #[arg(long = "closure", value_parser = parse_closure, allow_hyphen_values = true)]
        closures: Vec<BoundaryClosure>,
        /// Limit visited pixels. A limited region can never be applied.
        #[arg(long)]
        max_pixels: Option<usize>,
        /// Limit the pending flood frontier.
        #[arg(long)]
        max_frontier: Option<usize>,
        /// Limit traced boundary edges.
        #[arg(long)]
        max_edges: Option<usize>,
        /// Limit cooperative work across thresholding, flood and contour phases.
        #[arg(long)]
        max_work: Option<u64>,
        /// New strict review JSON; accepted starts false.
        #[arg(long)]
        review_out: Option<PathBuf>,
    },
    /// Rebuild a complete reviewed region and render its calibrated measurement without saving it.
    Preview {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        review: PathBuf,
        /// New PNG preview; existing files are refused.
        #[arg(long)]
        out: PathBuf,
        /// Preview longest side, in pixels, from 64 to 1536.
        #[arg(long, default_value_t = 1024, value_parser = clap::value_parser!(u32).range(64..=1536))]
        long_side: u32,
    },
    /// Rebuild and verify an explicitly accepted complete region, then write a new measured PDF.
    Apply {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        review: PathBuf,
        /// New PDF only, even when --overwrite is supplied.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reference {
    Automatic {},
    Page {},
    Region {
        id: String,
    },
    Local {
        points: [[f32; 2]; 2],
        known_distance: f64,
        unit: String,
    },
}

impl Reference {
    fn resolve(&self, document: &Document<'_>, page: usize) -> Result<RectangleScale, EngineError> {
        Ok(match self {
            Self::Automatic {} => RectangleScale::Automatic,
            Self::Page {} => RectangleScale::Page,
            Self::Region { id } => RectangleScale::Region(id.clone()),
            Self::Local {
                points,
                known_distance,
                unit,
            } => {
                let calibration = PageCalibration {
                    points: points.map(|[x, y]| (x, y)),
                    known_distance: *known_distance,
                    unit: parse_unit(unit)?,
                };
                RectangleScale::Local(document.local_calibration_reference(page, &calibration)?)
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    version: u8,
    raw_file_sha256: String,
    inspection_sha256: String,
    /// Physical one-based page number, unlike the core inspection's page index.
    page: usize,
    seed: [f32; 2],
    options: BoundaryFillOptions,
    accepted: bool,
    reference: Reference,
    unit: String,
    precision: usize,
    author: Option<String>,
}

impl RegionFillArgs {
    fn source(&self) -> &Source {
        match &self.command {
            RegionFillCommand::Inspect { source, .. }
            | RegionFillCommand::Preview { source, .. }
            | RegionFillCommand::Apply { source, .. } => source,
        }
    }

    pub fn password_mut(&mut self) -> &mut Option<String> {
        match &mut self.command {
            RegionFillCommand::Inspect { source, .. }
            | RegionFillCommand::Preview { source, .. }
            | RegionFillCommand::Apply { source, .. } => &mut source.password,
        }
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            RegionFillCommand::Inspect { review_out, .. } => review_out.as_deref(),
            RegionFillCommand::Preview { out, .. } | RegionFillCommand::Apply { out, .. } => {
                Some(out)
            }
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Region fill: {message}"))
}

fn coordinates<const N: usize>(text: &str) -> Result<[f32; N], String> {
    if text.len() > 256 {
        return Err("coordinates exceed 256 bytes".into());
    }
    let mut values = [0.0; N];
    let mut parts = text.split(',');
    for value in &mut values {
        *value = parts
            .next()
            .ok_or("missing coordinate")?
            .trim()
            .parse::<f32>()
            .map_err(|_| "coordinates must be finite numbers")?;
        if !value.is_finite() {
            return Err("coordinates must be finite numbers".into());
        }
    }
    if parts.next().is_some() {
        return Err("too many coordinates".into());
    }
    Ok(values)
}

fn parse_seed(text: &str) -> Result<[f32; 2], String> {
    coordinates(text)
}

fn parse_closure(text: &str) -> Result<BoundaryClosure, String> {
    let [x1, y1, x2, y2] = coordinates(text)?;
    Ok(BoundaryClosure {
        start: [x1, y1],
        end: [x2, y2],
    })
}

fn parse_scale(text: &str) -> Result<f32, String> {
    text.parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && (0.25..=2.0).contains(value))
        .ok_or_else(|| "raster scale must be 0.25..2 pixels per point".into())
}

fn parse_unit(text: &str) -> Result<MeasurementUnit, EngineError> {
    MeasurementUnit::from_id(text).ok_or_else(|| invalid("unit must be mm, cm, m, in or ft"))
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, EngineError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("input must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(invalid(
            "input exceeds its size limit or is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(invalid("input exceeds its size limit"));
    }
    Ok(bytes)
}

fn hash(bytes: &[u8]) -> String {
    sunipdf_core::plan_zones::source_sha256(bytes)
}

fn revalidate(source: &Source, canonical: &Path, expected: &str) -> Result<(), EngineError> {
    if source.file.canonicalize()? != canonical
        || hash(&read_bounded(canonical, MAX_SOURCE_BYTES)?) != expected
    {
        return Err(invalid("saved source changed; inspect and review again"));
    }
    Ok(())
}

fn finish(job: &mut BoundaryFillJob) {
    while !job.progress().status.is_terminal() {
        job.step(4096);
    }
}

fn inspection(job: &BoundaryFillJob) -> Result<Value, EngineError> {
    serde_json::to_value(job.report()).map_err(|_| invalid("could not serialize inspection"))
}

fn inspection_hash(report: &Value) -> Result<String, EngineError> {
    let mut stable = report.clone();
    // PDFium snapshot trailer identities are not a canonical saved-file identity.
    stable
        .as_object_mut()
        .ok_or_else(|| invalid("invalid inspection report"))?
        .remove("source_sha256");
    let bytes =
        serde_json::to_vec(&stable).map_err(|_| invalid("could not fingerprint inspection"))?;
    Ok(hash(&bytes))
}

pub fn run(engine: &Engine, args: RegionFillArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    let source = args.source();
    let canonical = source.file.canonicalize()?;
    let original = read_bounded(&canonical, MAX_SOURCE_BYTES)?;
    let raw_file_sha256 = hash(&original);
    let mut document = engine.open_bytes(original, source.password.as_deref(), &canonical)?;
    match &args.command {
        RegionFillCommand::Inspect {
            page,
            seed,
            scale,
            threshold,
            closures,
            max_pixels,
            max_frontier,
            max_edges,
            max_work,
            review_out,
            ..
        } => {
            let page_index = crate::page_index(*page, document.page_count())?;
            let mut options = BoundaryFillOptions::default();
            if let Some(value) = scale {
                options.scale = *value;
            }
            if let Some(value) = threshold {
                options.threshold = *value;
            }
            if let Some(value) = max_pixels {
                options.max_pixels = *value;
            }
            if let Some(value) = max_frontier {
                options.max_frontier = *value;
            }
            if let Some(value) = max_edges {
                options.max_edges = *value;
            }
            if let Some(value) = max_work {
                options.max_work = *value;
            }
            options.closures = closures.clone();
            let mut job = document.prepare_boundary_fill(page_index, *seed, options.clone())?;
            finish(&mut job);
            let inspection = inspection(&job)?;
            let complete = inspection["progress"]["status"] == "complete";
            let review = Review {
                version: 1,
                raw_file_sha256: raw_file_sha256.clone(),
                inspection_sha256: inspection_hash(&inspection)?,
                page: *page,
                seed: *seed,
                options,
                accepted: false,
                reference: Reference::Automatic {},
                unit: "m".into(),
                precision: 2,
                author: None,
            };
            revalidate(source, &canonical, &raw_file_sha256)?;
            if let Some(path) = review_out {
                let bytes = serde_json::to_vec_pretty(&review)
                    .map_err(|_| invalid("could not serialize review"))?;
                sunipdf_core::write_new_copy(path, &bytes)?;
            }
            Ok(
                json!({"operation":"inspect", "complete":complete, "published":false,
                "source_unchanged":true, "raw_file_sha256":raw_file_sha256,
                "inspection":inspection, "review":review, "review_out":review_out,
                "inspection_page_index_base":0, "review_page_index_base":1,
                "measurement_created":false, "physical_accuracy_validated":false}),
            )
        }
        RegionFillCommand::Preview { review, out, .. }
        | RegionFillCommand::Apply { review, out, .. } => {
            let preview_long_side = match &args.command {
                RegionFillCommand::Preview { long_side, .. } => Some(*long_side),
                _ => None,
            };
            let review: Review =
                serde_json::from_slice(&read_bounded(review, MAX_REVIEW_BYTES)?)
                    .map_err(|error| invalid(&format!("invalid strict review JSON: {error}")))?;
            if review.version != 1 || review.raw_file_sha256 != raw_file_sha256 {
                return Err(invalid("review version or saved source identity differs"));
            }
            if preview_long_side.is_none() && !review.accepted {
                return Err(invalid(
                    "explicitly review and accept the complete region before applying",
                ));
            }
            let page_index = crate::page_index(review.page, document.page_count())?;
            let mut job =
                document.prepare_boundary_fill(page_index, review.seed, review.options)?;
            finish(&mut job);
            let inspection = inspection(&job)?;
            if inspection_hash(&inspection)? != review.inspection_sha256 {
                return Err(invalid(
                    "region or inspection options changed; inspect and review again",
                ));
            }
            if inspection["progress"]["status"] != "complete" {
                return Err(invalid(
                    "only a complete enclosed region can become a measurement",
                ));
            }
            let options = BoundaryFillMeasurementOptions {
                scale: review.reference.resolve(&document, page_index)?,
                unit: parse_unit(&review.unit)?,
                precision: review.precision,
            };
            let prepared = document.prepare_boundary_fill_measurement(
                &job,
                &options,
                review.author.as_deref(),
            )?;
            let measurement = prepared.measurement().result()?;
            if let Some(long_side) = preview_long_side {
                let image =
                    document.render_boundary_fill_measurement_preview(&prepared, long_side)?;
                let mut png = Vec::new();
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(
                        &image.rgba,
                        image.width,
                        image.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|error| invalid(&format!("could not encode preview: {error}")))?;
                revalidate(source, &canonical, &raw_file_sha256)?;
                sunipdf_core::write_new_copy(out, &png)?;
                return Ok(
                    json!({"operation":"preview", "complete":true, "published":false,
                    "preview_written":true, "out":out, "source_unchanged":true,
                    "raw_file_sha256":raw_file_sha256, "inspection":inspection,
                    "measurement":measurement, "measurement_created":false,
                    "physical_accuracy_validated":false}),
                );
            }
            let index = document.apply_boundary_fill_measurement(&prepared)?;
            let candidate = document.snapshot()?;
            revalidate(source, &canonical, &raw_file_sha256)?;
            sunipdf_core::write_new_copy(out, &candidate)?;
            Ok(
                json!({"operation":"apply", "complete":true, "published":true, "out":out,
                "index":index, "source_unchanged":true, "raw_file_sha256":raw_file_sha256,
                "output_sha256":hash(&candidate), "inspection":inspection,
                "measurement":measurement, "measurement_created":true,
                "physical_accuracy_validated":false}),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_inputs_are_finite_and_bounded_without_expanding_user_text() {
        assert_eq!(parse_seed(" 12.5, 24 ").unwrap(), [12.5, 24.0]);
        assert_eq!(parse_closure("1,2,3,4").unwrap().end, [3.0, 4.0]);
        for value in ["", "1", "1,2,3", "NaN,1", "inf,1", "1,1e999"] {
            assert!(parse_seed(value).is_err(), "{value}");
        }
        assert!(parse_closure(&"1,".repeat(1000)).is_err());
        for value in ["0", "NaN", "inf", "-1", "2.01"] {
            assert!(parse_scale(value).is_err(), "{value}");
        }
        assert!(parse_scale("0.25").is_ok());
        assert!(parse_scale("2").is_ok());
    }

    #[test]
    fn reference_json_refuses_unknown_variants_and_extra_fields() {
        for value in [
            json!({"kind":"automatic", "extra":true}),
            json!({"kind":"page", "unit":"m"}),
            json!({"kind":"region", "id":"room", "extra":true}),
            json!({"kind":"unsupported"}),
            json!({"kind":"local", "points":[[0,0],[100,0]], "known_distance":1,
                "unit":"m", "extra":true}),
        ] {
            assert!(serde_json::from_value::<Reference>(value).is_err());
        }
    }
}
