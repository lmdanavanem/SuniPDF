use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::plan_links::{self, PlanLinkCredentials, PlanLinkReview, PlanLinkScanRequest};
use sunipdf_core::{plan_set, Engine, EngineError};
use zeroize::Zeroizing;

#[derive(Args)]
pub struct PlanLinksArgs {
    /// Read an indexed PDF's password from an environment variable: FILE_ID=ENV_NAME. Repeat per file.
    #[arg(long, global = true, value_name = "FILE_ID=ENV_NAME")]
    file_password_env: Vec<String>,
    #[command(subcommand)]
    pub command: PlanLinksCommand,
}

#[derive(Clone, Debug)]
pub struct PhysicalPages(Vec<usize>);

#[derive(Subcommand)]
pub enum PlanLinksCommand {
    /// Find exact indexed sheet references in searchable text; incomplete inspection exits with code 2.
    Inspect {
        manifest: PathBuf,
        #[arg(long)]
        source_file: String,
        /// Explicit comma-separated physical pages, starting at 1; at most 32, without duplicates.
        #[arg(long, value_parser = parse_pages)]
        pages: PhysicalPages,
        /// Explicit target sheet ID; repeat for every permitted destination.
        #[arg(long, required = true)]
        target_sheet: Vec<String>,
    },
    /// Prepare reviewed links and render a real candidate page; never write a PDF.
    Preview {
        manifest: PathBuf,
        /// Version-1 JSON review, at most 256 KiB; every selected occurrence must be confirmed.
        #[arg(long)]
        review: PathBuf,
        /// New PNG; existing files are refused, including with --overwrite.
        #[arg(long)]
        preview_out: PathBuf,
        /// Physical linked page, starting at 1; defaults to the first created link's page.
        #[arg(long, value_parser = positive_page, conflicts_with = "preview_target_sheet")]
        preview_page: Option<usize>,
        /// Render a verified destination sheet selected in this review, instead of a source candidate page.
        #[arg(long, conflicts_with = "preview_page")]
        preview_target_sheet: Option<String>,
        #[arg(long, default_value_t = 1200, value_parser = clap::value_parser!(u32).range(64..=1536))]
        preview_long_side: u32,
    },
    /// Recheck the manifest and PDFs, rebuild reviewed links and write a new copy beside the source PDF.
    Apply {
        manifest: PathBuf,
        #[arg(long)]
        review: PathBuf,
        /// New PDF in the source directory, preserving existing relative links; never overwrite.
        #[arg(long)]
        out: PathBuf,
    },
}

impl PlanLinksArgs {
    pub fn is_inspection(&self) -> bool {
        matches!(self.command, PlanLinksCommand::Inspect { .. })
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            PlanLinksCommand::Inspect { .. } => None,
            PlanLinksCommand::Preview { preview_out, .. } => Some(preview_out),
            PlanLinksCommand::Apply { out, .. } => Some(out),
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Plan links: {message}"))
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| "physical page numbers start at 1".into())
}

fn parse_pages(value: &str) -> Result<PhysicalPages, String> {
    if value.is_empty() || value.len() > 4096 {
        return Err("provide 1..32 explicit physical pages".into());
    }
    let mut pages = Vec::new();
    let mut seen = HashSet::new();
    for item in value.split(',') {
        if pages.len() == plan_links::MAX_SCAN_PAGES {
            return Err("at most 32 pages may be inspected".into());
        }
        let page = positive_page(item.trim())?;
        if !seen.insert(page) {
            return Err("physical pages must be unique".into());
        }
        pages.push(page);
    }
    Ok(PhysicalPages(pages))
}

fn read_review(path: &Path) -> Result<PlanLinkReview, EngineError> {
    let maximum = plan_links::MAX_RECIPE_BYTES as u64;
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
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(invalid("review must be a regular file of at most 256 KiB"));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    plan_links::parse_review(&bytes)
}

fn read_passwords(
    specs: &[String],
    set: &plan_set::PlanSet,
) -> Result<Vec<(String, Zeroizing<String>)>, EngineError> {
    if specs.len() > plan_set::MAX_FILES {
        return Err(invalid("too many password sources"));
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for spec in specs {
        let (id, name) = spec
            .split_once('=')
            .ok_or_else(|| invalid("password source must be FILE_ID=ENV_NAME"))?;
        if !set.files.iter().any(|file| file.id == id)
            || !seen.insert(id)
            || name.is_empty()
            || name.len() > 256
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(invalid("invalid or duplicate password source"));
        }
        let secret = Zeroizing::new(
            std::env::var(name)
                .map_err(|_| invalid("password environment variable is missing or not Unicode"))?,
        );
        if secret.len() > 65_536 || secret.contains('\0') {
            return Err(invalid(
                "password exceeds limits or contains a null character",
            ));
        }
        result.push((id.to_owned(), secret));
    }
    Ok(result)
}

pub fn run(args: &PlanLinksArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    let manifest = match &args.command {
        PlanLinksCommand::Inspect { manifest, .. }
        | PlanLinksCommand::Preview { manifest, .. }
        | PlanLinksCommand::Apply { manifest, .. } => manifest,
    };
    let set = plan_set::load(manifest)?;
    let passwords = read_passwords(&args.file_password_env, &set)?;
    let borrowed: Vec<_> = passwords
        .iter()
        .map(|(id, secret)| (id.as_str(), secret.as_str()))
        .collect();
    let credentials = PlanLinkCredentials {
        passwords: &borrowed,
    };
    let engine = Engine::new()?;
    let cancel = AtomicBool::new(false);
    match &args.command {
        PlanLinksCommand::Inspect {
            source_file,
            pages,
            target_sheet,
            ..
        } => {
            let scan = PlanLinkScanRequest {
                source_file_id: source_file.clone(),
                pages: pages.0.clone(),
                target_sheet_ids: target_sheet.clone(),
            };
            let report = plan_links::inspect_plan_links(
                &engine,
                manifest,
                &set,
                &scan,
                &credentials,
                &cancel,
            )?;
            let mut value = report.json();
            value["operation"] = json!("inspect");
            value["page_index_base"] = json!(1);
            value["source_documents_unchanged"] = json!(true);
            Ok(value)
        }
        PlanLinksCommand::Preview {
            review,
            preview_out,
            preview_page,
            preview_target_sheet,
            preview_long_side,
            ..
        } => {
            let review = read_review(review)?;
            let prepared = plan_links::prepare_plan_links(
                &engine,
                manifest,
                &set,
                &review,
                &credentials,
                &cancel,
            )?;
            let (page, rendered) = if let Some(sheet_id) = preview_target_sheet {
                let target = prepared
                    .report()
                    .inspection
                    .targets
                    .iter()
                    .find(|target| target.sheet_id == *sheet_id)
                    .ok_or_else(|| invalid("preview destination was not inspected"))?;
                let password = borrowed
                    .iter()
                    .find_map(|(id, secret)| (*id == target.file_id).then_some(*secret));
                (
                    target.page,
                    prepared.render_target_preview(
                        &engine,
                        sheet_id,
                        password,
                        *preview_long_side,
                        &cancel,
                    )?,
                )
            } else {
                let page = preview_page
                    .or_else(|| prepared.report().created.first().map(|c| c.page))
                    .ok_or_else(|| invalid("candidate has no linked page"))?;
                let password = borrowed.iter().find_map(|(id, secret)| {
                    (*id == review.scan.source_file_id).then_some(*secret)
                });
                (
                    page,
                    prepared.render_preview(
                        &engine,
                        page,
                        password,
                        *preview_long_side,
                        &cancel,
                    )?,
                )
            };
            if rendered.width == 0
                || rendered.height == 0
                || rendered.width.max(rendered.height) > *preview_long_side
                || u64::from(rendered.width) * u64::from(rendered.height) * 4
                    != rendered.rgba.len() as u64
            {
                return Err(invalid("invalid candidate preview dimensions"));
            }
            let mut png = Vec::new();
            image::codecs::png::PngEncoder::new(&mut png)
                .write_image(
                    &rendered.rgba,
                    rendered.width,
                    rendered.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|_| invalid("candidate PNG encoding failed"))?;
            prepared.revalidate_sources(&cancel)?;
            sunipdf_core::write_new_copy(preview_out, &png)?;
            let mut value = prepared.report().json();
            value["operation"] = json!("preview");
            value["published"] = json!(false);
            value["preview_image"] = json!({"path": preview_out, "page": page,
                "kind": if preview_target_sheet.is_some() {"destination"} else {"source_candidate"},
                "target_sheet_id":preview_target_sheet,
                "width": rendered.width, "height": rendered.height, "bytes": png.len()});
            Ok(value)
        }
        PlanLinksCommand::Apply { review, out, .. } => {
            let review = read_review(review)?;
            let prepared = plan_links::prepare_plan_links(
                &engine,
                manifest,
                &set,
                &review,
                &credentials,
                &cancel,
            )?;
            let receipt = plan_links::publish_plan_links(prepared, out, &cancel)?;
            let mut value = receipt.report.json();
            value["operation"] = json!("apply");
            value["published"] = json!(true);
            value["out"] = json!(receipt.output);
            value["bytes"] = json!(receipt.bytes);
            value["source_documents_unchanged"] = json!(true);
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_pages_preserve_order_and_refuse_duplicates_ranges_and_excess() {
        assert_eq!(parse_pages("3, 1,2").unwrap().0, [3, 1, 2]);
        for invalid in ["", "0", "1,1", "1-3", "all", "1,", "-1"] {
            assert!(parse_pages(invalid).is_err(), "{invalid}");
        }
        let excessive = (1..=33)
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_pages(&excessive).is_err());
    }
}
