use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::plan_zones::{self, ZoneValidity};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroize;

const MAX_SOURCE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Args)]
pub struct PlanZonesArgs {
    #[command(subcommand)]
    command: PlanZonesCommand,
}

#[derive(Args)]
pub struct Source {
    /// Saved PDF, at most 128 MiB. Use --password-env or --password-stdin when needed.
    file: PathBuf,
    /// Physical page number, starting at 1.
    #[arg(long, default_value_t = 1, value_parser = positive_page)]
    page: usize,
    /// Optionally require the exact source hash from a previous inspection or preview.
    #[arg(long, value_parser = digest)]
    expected_source_sha256: Option<String>,
    #[arg(skip)]
    password: Option<String>,
}

impl Drop for Source {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

#[derive(Args)]
pub struct Replacement {
    #[command(flatten)]
    source: Source,
    /// Complete replacement: strict {"version":1,"zones":[...]} JSON, at most 128 KiB.
    #[arg(long)]
    zones: PathBuf,
    /// Explicitly review and replace stale historical zone geometry on the current page.
    #[arg(long)]
    reset_stale: bool,
}

#[derive(Subcommand)]
enum PlanZonesCommand {
    /// Read current or historical page zones and conservative annotation assignments.
    Inspect {
        #[command(flatten)]
        source: Source,
    },
    /// Prepare a replacement and report before/after zones and assignments without writing a PDF.
    Preview {
        #[command(flatten)]
        replacement: Replacement,
        /// Optional new PNG of the actual candidate page; zones remain metadata, not painted content.
        #[arg(long)]
        preview_out: Option<PathBuf>,
        #[arg(long, requires = "preview_out", value_parser = clap::value_parser!(u32).range(64..=1536))]
        preview_long_side: Option<u32>,
    },
    /// Replace page zones in a new atomic PDF copy, revalidating the saved source before publication.
    Apply {
        #[command(flatten)]
        replacement: Replacement,
        /// New PDF only. Existing files are refused even with global --overwrite.
        #[arg(long)]
        out: PathBuf,
    },
}

impl PlanZonesArgs {
    fn source(&self) -> &Source {
        match &self.command {
            PlanZonesCommand::Inspect { source } => source,
            PlanZonesCommand::Preview { replacement, .. }
            | PlanZonesCommand::Apply { replacement, .. } => &replacement.source,
        }
    }

    pub fn password_mut(&mut self) -> &mut Option<String> {
        match &mut self.command {
            PlanZonesCommand::Inspect { source } => &mut source.password,
            PlanZonesCommand::Preview { replacement, .. }
            | PlanZonesCommand::Apply { replacement, .. } => &mut replacement.source.password,
        }
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            PlanZonesCommand::Inspect { .. } => None,
            PlanZonesCommand::Preview { preview_out, .. } => preview_out.as_deref(),
            PlanZonesCommand::Apply { out, .. } => Some(out),
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Plan zones: {message}"))
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "physical page numbers start at 1".into())
}

fn digest(value: &str) -> Result<String, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("source SHA256 must contain exactly 64 hexadecimal digits".into());
    }
    Ok(value.to_ascii_lowercase())
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, EngineError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("input must be a regular file"));
    }
    let file = sunipdf_core::file_io::open_regular(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(invalid(
            "input is not a regular file within the supported size limit",
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(invalid("input exceeds the supported size limit"));
    }
    Ok(bytes)
}

fn revalidate(source: &Source, canonical: &Path, expected: &str) -> Result<(), EngineError> {
    if source.file.canonicalize()? != canonical
        || plan_zones::source_sha256(&read_bounded(canonical, MAX_SOURCE_BYTES)?) != expected
    {
        return Err(invalid("saved source changed; inspect and preview again"));
    }
    Ok(())
}

pub fn run(engine: &Engine, args: PlanZonesArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.to_owned()));
        }
    }
    let source = args.source();
    let canonical = source.file.canonicalize()?;
    let original = read_bounded(&canonical, MAX_SOURCE_BYTES)?;
    let source_sha256 = plan_zones::source_sha256(&original);
    if source
        .expected_source_sha256
        .as_ref()
        .is_some_and(|expected| *expected != source_sha256)
    {
        return Err(invalid(
            "source hash does not match the expected saved revision",
        ));
    }
    let mut document = engine.open_bytes(original, source.password.as_deref(), &canonical)?;
    let page = crate::page_index(source.page, document.page_count())?;
    let mut value = match &args.command {
        PlanZonesCommand::Inspect { .. } => {
            let zones = document.plan_zones(page)?;
            let register = document.annotation_register()?;
            let assignments: Vec<_> = register.rows.iter().filter(|row| row.page == source.page)
                .map(|row| json!({"page":row.page,"index":row.index,"annotation_id":row.annotation_id,"assignment":row.zone})).collect();
            json!({"operation":"inspect","complete":zones.validity == ZoneValidity::Current,"zones":zones,"assignments":assignments})
        }
        PlanZonesCommand::Preview {
            replacement,
            preview_out,
            preview_long_side,
        } => {
            let zones = plan_zones::parse_zones(&read_bounded(
                &replacement.zones,
                plan_zones::MAX_RECORD_BYTES as u64,
            )?)?;
            let prepared = document.prepare_plan_zones(page, &zones, replacement.reset_stale)?;
            let mut value = prepared.report().json();
            value["operation"] = json!("preview");
            value["published"] = json!(false);
            value["complete"] = json!(true);
            value["preview_image"] = Value::Null;
            if let Some(out) = preview_out {
                let image = document
                    .render_plan_zone_preview(&prepared, preview_long_side.unwrap_or(1024))?;
                let mut png = Vec::new();
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(
                        &image.rgba,
                        image.width,
                        image.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|_| invalid("could not encode preview PNG"))?;
                revalidate(source, &canonical, &source_sha256)?;
                sunipdf_core::write_new_copy(out, &png)?;
                value["preview_image"] = json!({"out":out,"width":image.width,"height":image.height,"zones_are_metadata":true});
            }
            value
        }
        PlanZonesCommand::Apply { replacement, out } => {
            let zones = plan_zones::parse_zones(&read_bounded(
                &replacement.zones,
                plan_zones::MAX_RECORD_BYTES as u64,
            )?)?;
            let prepared = document.prepare_plan_zones(page, &zones, replacement.reset_stale)?;
            let mut value = prepared.report().json();
            document.apply_plan_zones(prepared)?;
            let candidate = document.snapshot()?;
            revalidate(source, &canonical, &source_sha256)?;
            sunipdf_core::write_new_copy(out, &candidate)?;
            value["operation"] = json!("apply");
            value["published"] = json!(true);
            value["complete"] = json!(true);
            value["out"] = json!(out);
            value["output_sha256"] = json!(plan_zones::source_sha256(&candidate));
            value["encrypted"] = json!(document.properties().encrypted);
            value
        }
    };
    if args.output_path().is_none() {
        revalidate(source, &canonical, &source_sha256)?;
    }
    value["source_sha256"] = json!(source_sha256);
    value["page_index_base"] = json!(1);
    value["page"] = json!(source.page);
    value["original_unchanged"] = json!(true);
    Ok(value)
}
