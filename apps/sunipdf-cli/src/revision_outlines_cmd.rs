use crate::revision_transfer_cmd::TransferSources;
use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_outlines::{self, OutlineOptions, OutlinePasswords, OutlineReview};
use sunipdf_core::revision_overlay::{OverlayAlignment, OverlayTransform};
use sunipdf_core::{Engine, EngineError};

#[derive(Args)]
pub struct RevisionOutlinesArgs {
    #[command(subcommand)]
    pub command: RevisionOutlinesCommand,
}

#[derive(Args)]
pub struct InspectionOptions {
    /// Physical page of the old saved PDF, starting at 1.
    #[arg(long, default_value_t = 1, value_parser = positive_page)]
    old_page: usize,
    /// Physical page of the new saved PDF, starting at 1.
    #[arg(long, default_value_t = 1, value_parser = positive_page)]
    new_page: usize,
    /// OLD-to-NEW translation x,y in displayed points; TOP LEFT origin, y grows down.
    #[arg(long, value_parser = point, allow_hyphen_values = true, conflicts_with_all = ["old_points", "new_points"])]
    translate: Option<[f64; 2]>,
    /// Clockwise OLD-to-NEW rotation about the old page's displayed top-left origin.
    #[arg(long, value_parser = finite, allow_negative_numbers = true, conflicts_with_all = ["old_points", "new_points"])]
    rotation: Option<f64>,
    /// Positive uniform OLD-to-NEW scale, from 0.01 to 100; defaults to 1.
    #[arg(long, value_parser = scale, conflicts_with_all = ["old_points", "new_points"])]
    scale: Option<f64>,
    /// Two corresponding OLD page points: "x,y x,y"; TOP LEFT origin.
    #[arg(long, value_parser = two_points, allow_hyphen_values = true, requires = "new_points")]
    old_points: Option<[[f64; 2]; 2]>,
    /// Two corresponding NEW page points in the same order; TOP LEFT origin.
    #[arg(long, value_parser = two_points, allow_hyphen_values = true, requires = "old_points")]
    new_points: Option<[[f64; 2]; 2]>,
    /// Longest comparison raster side, 64..1536 pixels; not a vector comparison.
    #[arg(long, default_value_t = 1536, value_parser = clap::value_parser!(u32).range(64..=1536))]
    long_side: u32,
    /// Per-channel pixel tolerance; interpretation of a detected region requires human review.
    #[arg(long, default_value_t = 16, value_parser = clap::value_parser!(u8).range(1..=254))]
    channel_tolerance: u8,
    /// Discard smaller raster components; filtered pixels remain reported.
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(1..=10000))]
    min_component_pixels: u32,
    /// Merge nearby raster components within this pixel gap.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(0..=16))]
    merge_gap_pixels: u32,
}

impl InspectionOptions {
    fn options(&self) -> OutlineOptions {
        let alignment = match (self.old_points, self.new_points) {
            (Some(old), Some(new)) => OverlayAlignment::TwoPoints {
                base: new,
                revision: old,
            },
            _ => OverlayAlignment::Manual {
                transform: OverlayTransform {
                    translation: self.translate.unwrap_or([0.0, 0.0]),
                    rotation_degrees: self.rotation.unwrap_or(0.0),
                    scale: self.scale.unwrap_or(1.0),
                },
            },
        };
        OutlineOptions {
            old_page: self.old_page,
            new_page: self.new_page,
            alignment,
            long_side: self.long_side,
            channel_tolerance: self.channel_tolerance,
            min_component_pixels: self.min_component_pixels,
            merge_gap_pixels: self.merge_gap_pixels,
        }
    }
}

#[derive(Subcommand)]
pub enum RevisionOutlinesCommand {
    /// Propose bounded raster difference regions; these are not confirmed content changes.
    Inspect {
        #[command(flatten)]
        sources: TransferSources,
        #[command(flatten)]
        options: InspectionOptions,
    },
    /// Rebuild confirmed decisions and optionally render the actual new PDF candidate to a PNG.
    Preview {
        #[command(flatten)]
        sources: TransferSources,
        /// Source-bound review JSON with confirmed classification for every selected region.
        #[arg(long)]
        review: PathBuf,
        /// New PNG only. Omit to inspect a report, including a review containing only rendering noise.
        #[arg(long)]
        preview_out: Option<PathBuf>,
        #[arg(long, requires = "preview_out", value_parser = clap::value_parser!(u32).range(64..=1536))]
        preview_long_side: Option<u32>,
    },
    /// Revalidate both PDFs and publish reviewed outlines in a new copy of the complete new PDF.
    Apply {
        #[command(flatten)]
        sources: TransferSources,
        #[arg(long)]
        review: PathBuf,
        /// New PDF only; --overwrite never authorizes replacement for this command.
        #[arg(long)]
        out: PathBuf,
    },
}

impl RevisionOutlinesArgs {
    pub fn sources_mut(&mut self) -> &mut TransferSources {
        match &mut self.command {
            RevisionOutlinesCommand::Inspect { sources, .. }
            | RevisionOutlinesCommand::Preview { sources, .. }
            | RevisionOutlinesCommand::Apply { sources, .. } => sources,
        }
    }

    pub fn new_password_stdin(&self) -> bool {
        match &self.command {
            RevisionOutlinesCommand::Inspect { sources, .. }
            | RevisionOutlinesCommand::Preview { sources, .. }
            | RevisionOutlinesCommand::Apply { sources, .. } => sources.new_password_stdin,
        }
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            RevisionOutlinesCommand::Inspect { .. } => None,
            RevisionOutlinesCommand::Preview { preview_out, .. } => preview_out.as_deref(),
            RevisionOutlinesCommand::Apply { out, .. } => Some(out),
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Revision outlines: {message}"))
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| "physical page numbers start at 1".into())
}

fn finite(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "coordinate or angle must be finite".into())
}

fn scale(value: &str) -> Result<f64, String> {
    finite(value).and_then(|v| {
        if (0.01..=100.0).contains(&v) {
            Ok(v)
        } else {
            Err("scale must be between 0.01 and 100".into())
        }
    })
}

fn point(value: &str) -> Result<[f64; 2], String> {
    if value.len() > 128 {
        return Err("point exceeds its size limit".into());
    }
    let (x, y) = value.split_once(',').ok_or("point must be x,y")?;
    Ok([finite(x)?, finite(y)?])
}

fn two_points(value: &str) -> Result<[[f64; 2]; 2], String> {
    if value.len() > 260 {
        return Err("reference points exceed their size limit".into());
    }
    let values: Vec<_> = value.split_whitespace().collect();
    let [a, b] = values.as_slice() else {
        return Err("provide exactly two points: x,y x,y".into());
    };
    let points = [point(a)?, point(b)?];
    if points[0] == points[1] {
        return Err("reference points must be distinct".into());
    }
    Ok(points)
}

fn read_review(path: &Path) -> Result<OutlineReview, EngineError> {
    const MAXIMUM: u64 = 256 * 1024;
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("review must be a regular file"));
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
    if !metadata.is_file() || metadata.len() > MAXIMUM {
        return Err(invalid("review must be a regular file of at most 256 KiB"));
    }
    let mut bytes = Vec::new();
    file.take(MAXIMUM + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAXIMUM {
        return Err(invalid("review exceeds 256 KiB"));
    }
    revision_outlines::parse_review(&bytes)
}

fn passwords(sources: &TransferSources) -> OutlinePasswords<'_> {
    OutlinePasswords {
        old: sources.password.as_deref(),
        new: sources.new_password.as_deref(),
    }
}

pub fn run(engine: &Engine, args: RevisionOutlinesArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    let cancel = AtomicBool::new(false);
    match &args.command {
        RevisionOutlinesCommand::Inspect { sources, options } => {
            let inspection = revision_outlines::inspect_revision_outlines(
                engine,
                &sources.old,
                &sources.new,
                &options.options(),
                passwords(sources),
                &cancel,
                |_| {},
            )?;
            let mut value = inspection.json();
            value["operation"] = json!("inspect");
            value["page_index_base"] = json!(1);
            value["originals_unchanged"] = json!(true);
            Ok(value)
        }
        RevisionOutlinesCommand::Preview {
            sources,
            review,
            preview_out,
            preview_long_side,
        } => {
            let review = read_review(review)?;
            let prepared = revision_outlines::prepare_revision_outlines(
                engine,
                &sources.old,
                &sources.new,
                &review,
                passwords(sources),
                &cancel,
                |_| {},
            )?;
            prepared.revalidate_sources(&cancel)?;
            let mut value = prepared.report().json();
            value["operation"] = json!("preview");
            value["published"] = json!(false);
            value["preview_image"] = Value::Null;
            if let Some(path) = preview_out {
                let maximum = preview_long_side.unwrap_or(1200);
                let image = prepared.render_preview(
                    engine,
                    sources.new_password.as_deref(),
                    maximum,
                    &cancel,
                )?;
                if image.width == 0
                    || image.height == 0
                    || image.width.max(image.height) > maximum
                    || u64::from(image.width) * u64::from(image.height) * 4
                        != image.rgba.len() as u64
                {
                    return Err(invalid("invalid candidate preview dimensions"));
                }
                let mut png = Vec::new();
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(
                        &image.rgba,
                        image.width,
                        image.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|_| invalid("candidate PNG encoding failed"))?;
                prepared.revalidate_sources(&cancel)?;
                sunipdf_core::write_new_copy(path, &png)?;
                value["preview_image"] = json!({"path":path,"width":image.width,"height":image.height,"bytes":png.len()});
            }
            Ok(value)
        }
        RevisionOutlinesCommand::Apply {
            sources,
            review,
            out,
        } => {
            let review = read_review(review)?;
            let prepared = revision_outlines::prepare_revision_outlines(
                engine,
                &sources.old,
                &sources.new,
                &review,
                passwords(sources),
                &cancel,
                |_| {},
            )?;
            let report = revision_outlines::publish_revision_outlines(prepared, out, &cancel)?;
            let mut value = report.json();
            value["operation"] = json!("apply");
            value["published"] = json!(true);
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_arguments_are_finite_and_bounded() {
        assert_eq!(point("-1.5,2").unwrap(), [-1.5, 2.0]);
        assert_eq!(two_points("0,0 10,20").unwrap(), [[0.0, 0.0], [10.0, 20.0]]);
        for text in ["NaN,1", "1,inf", "1", "1,2,3"] {
            assert!(point(text).is_err());
        }
        for text in ["0,0 0,0", "0,0", "0,0 1,1 2,2"] {
            assert!(two_points(text).is_err());
        }
        for text in ["0", "-1", "0.001", "101", "NaN"] {
            assert!(scale(text).is_err());
        }
        assert!(positive_page("0").is_err());
    }
}
