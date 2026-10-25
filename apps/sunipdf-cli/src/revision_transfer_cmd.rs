use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_transfer::{self, RevisionTransferRequest, TransferPasswords};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroize;

mod batch;

#[derive(Args)]
pub struct RevisionTransferArgs {
    #[command(subcommand)]
    pub command: RevisionTransferCommand,
}

#[derive(Args)]
pub struct TransferSources {
    /// Saved old PDF, at most 64 MiB; use --password-env/--password-stdin for its password.
    pub old: PathBuf,
    /// Saved new PDF, at most 64 MiB; application writes a separate full PDF copy.
    pub new: PathBuf,
    /// Read the new PDF password from an environment variable; the value never appears in argv.
    #[arg(long, conflicts_with = "new_password_stdin")]
    pub new_password_env: Option<String>,
    /// Read the new PDF password from piped UTF-8 stdin; only one PDF may use stdin per call.
    #[arg(long)]
    pub new_password_stdin: bool,
    #[arg(skip)]
    pub password: Option<String>,
    #[arg(skip)]
    pub new_password: Option<String>,
}

impl Drop for TransferSources {
    fn drop(&mut self) {
        self.password.zeroize();
        self.new_password.zeroize();
    }
}

impl TransferSources {
    fn passwords(&self) -> TransferPasswords<'_> {
        TransferPasswords {
            old: self.password.as_deref(),
            new: self.new_password.as_deref(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PagePairs(Vec<(usize, usize)>);

#[derive(Subcommand)]
pub enum RevisionTransferCommand {
    /// Validate and prepare every reviewed transfer in a bounded JSON batch; write no PDFs.
    BatchPreview {
        /// Version 1 JSON manifest; paths are relative to this file unless absolute.
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Revalidate all reviewed transfers, then save separate new PDF copies with per-file results.
    BatchApply {
        /// Version 1 JSON manifest; existing outputs and source aliases are refused.
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Inspect explicitly paired physical pages. Use --password-env/--password-stdin for the old PDF.
    Inspect {
        #[command(flatten)]
        sources: TransferSources,
        /// Explicit old:new physical page pairs, e.g. "1:1,2:3"; 1-based, at most 32 unique pairs.
        #[arg(long, value_parser = parse_pairs)]
        pages: PagePairs,
    },
    /// Prepare a source-bound transfer without writing a PDF; optionally write a real candidate PNG.
    Preview {
        #[command(flatten)]
        sources: TransferSources,
        /// Versioned JSON recipe, at most 256 KiB; source hashes and every selected fingerprint are checked.
        #[arg(long)]
        recipe: PathBuf,
        /// New PNG only; existing files are refused even with --overwrite.
        #[arg(long)]
        preview_out: Option<PathBuf>,
        /// Physical page of the candidate to render (1-based); defaults to the first mapped target page.
        #[arg(long, requires = "preview_out", value_parser = positive_page)]
        preview_page: Option<usize>,
        /// Longest PNG side, 64..1536 pixels; defaults to 1200 when a PNG is requested.
        #[arg(long, requires = "preview_out", value_parser = clap::value_parser!(u32).range(64..=1536))]
        preview_long_side: Option<u32>,
    },
    /// Revalidate a recipe and both sources, prepare anew, and publish one new PDF copy.
    Apply {
        #[command(flatten)]
        sources: TransferSources,
        /// JSON recipe, not a preview report or PDF; no saved preview can authorize publication.
        #[arg(long)]
        recipe: PathBuf,
        /// New PDF only; originals, existing paths and --overwrite never permit replacement.
        #[arg(long)]
        out: PathBuf,
    },
}

impl RevisionTransferArgs {
    pub fn sources_mut(&mut self) -> Option<&mut TransferSources> {
        match &mut self.command {
            RevisionTransferCommand::Inspect { sources, .. }
            | RevisionTransferCommand::Preview { sources, .. }
            | RevisionTransferCommand::Apply { sources, .. } => Some(sources),
            RevisionTransferCommand::BatchPreview { .. }
            | RevisionTransferCommand::BatchApply { .. } => None,
        }
    }

    pub fn new_password_stdin(&self) -> bool {
        match &self.command {
            RevisionTransferCommand::Inspect { sources, .. }
            | RevisionTransferCommand::Preview { sources, .. }
            | RevisionTransferCommand::Apply { sources, .. } => sources.new_password_stdin,
            RevisionTransferCommand::BatchPreview { .. }
            | RevisionTransferCommand::BatchApply { .. } => false,
        }
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            RevisionTransferCommand::Inspect { .. }
            | RevisionTransferCommand::BatchPreview { .. }
            | RevisionTransferCommand::BatchApply { .. } => None,
            RevisionTransferCommand::Preview { preview_out, .. } => preview_out.as_deref(),
            RevisionTransferCommand::Apply { out, .. } => Some(out),
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "physical page numbers start at 1".into())
}

fn parse_pairs(value: &str) -> Result<PagePairs, String> {
    if value.is_empty() || value.len() > 4096 {
        return Err("provide 1..32 explicit old:new page pairs".into());
    }
    let mut pairs = Vec::new();
    let mut old_pages = HashSet::new();
    let mut new_pages = HashSet::new();
    for item in value.split(',') {
        if pairs.len() == 32 {
            return Err("at most 32 page pairs are supported".into());
        }
        let (old, new) = item
            .trim()
            .split_once(':')
            .ok_or_else(|| "page pairs must use old:new, e.g. 1:1,2:3".to_string())?;
        let pair = (positive_page(old.trim())?, positive_page(new.trim())?);
        if !old_pages.insert(pair.0) || !new_pages.insert(pair.1) {
            return Err("each old and new page may occur only once".into());
        }
        pairs.push(pair);
    }
    Ok(PagePairs(pairs))
}

fn read_recipe(path: &Path) -> Result<RevisionTransferRequest, EngineError> {
    const MAXIMUM: u64 = revision_transfer::MAX_RECIPE_BYTES as u64;
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("revision transfer recipe must be a regular file"));
    }
    let file = sunipdf_core::file_io::open_regular(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAXIMUM {
        return Err(invalid(
            "revision transfer recipe must be a regular file of at most 256 KiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAXIMUM + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAXIMUM {
        return Err(invalid("revision transfer recipe exceeds 256 KiB"));
    }
    revision_transfer::parse_request(&bytes)
}

pub fn run(engine: &Engine, args: RevisionTransferArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.exists() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    let cancel = AtomicBool::new(false);
    match &args.command {
        RevisionTransferCommand::BatchPreview { manifest } => batch::run(engine, manifest, false),
        RevisionTransferCommand::BatchApply { manifest } => batch::run(engine, manifest, true),
        RevisionTransferCommand::Inspect { sources, pages } => {
            let inspection = revision_transfer::inspect_transfer_candidates(
                engine,
                &sources.old,
                &sources.new,
                sources.passwords(),
                &pages.0,
                &cancel,
            )?;
            let mut value = inspection.json();
            value["operation"] = json!("inspect");
            value["complete"] = json!(inspection.complete());
            value["page_index_base"] = json!(1);
            value["annotation_index_base"] = json!(0);
            Ok(value)
        }
        RevisionTransferCommand::Preview {
            sources,
            recipe,
            preview_out,
            preview_page,
            preview_long_side,
        } => {
            let request = read_recipe(recipe)?;
            let prepared = revision_transfer::prepare_revision_transfer(
                engine,
                &sources.old,
                &sources.new,
                sources.passwords(),
                &request,
                &cancel,
            )?;
            prepared.revalidate_sources(&cancel)?;
            let mut value = prepared.report().json();
            value["operation"] = json!("preview");
            value["complete"] = json!(prepared.report().complete());
            value["published"] = json!(false);
            value["preview_image"] = Value::Null;
            if let Some(out) = preview_out {
                let page = preview_page.unwrap_or_else(|| request.pairs[0].new_page);
                let maximum = preview_long_side.unwrap_or(1200);
                let image = prepared.render_preview(
                    engine,
                    page,
                    sources.new_password.as_deref(),
                    maximum,
                    &cancel,
                )?;
                let expected = u64::from(image.width) * u64::from(image.height) * 4;
                if image.width == 0
                    || image.height == 0
                    || image.width.max(image.height) > maximum
                    || expected != image.rgba.len() as u64
                {
                    return Err(invalid(
                        "candidate preview dimensions or pixel buffer are invalid",
                    ));
                }
                let mut png = Vec::new();
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(
                        &image.rgba,
                        image.width,
                        image.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|_| invalid("candidate preview PNG encoding failed"))?;
                prepared.revalidate_sources(&cancel)?;
                sunipdf_core::write_new_copy(out, &png)?;
                value["preview_image"] = json!({
                    "path": out, "format": "png", "page": page,
                    "width": image.width, "height": image.height, "bytes": png.len(),
                });
            }
            Ok(value)
        }
        RevisionTransferCommand::Apply {
            sources,
            recipe,
            out,
        } => {
            let request = read_recipe(recipe)?;
            let prepared = revision_transfer::prepare_revision_transfer(
                engine,
                &sources.old,
                &sources.new,
                sources.passwords(),
                &request,
                &cancel,
            )?;
            let report = revision_transfer::publish_revision_transfer(prepared, out, &cancel)?;
            let mut value = report.json();
            value["operation"] = json!("apply");
            value["complete"] = json!(report.complete());
            value["published"] = json!(true);
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_commands_use_manifest_scoped_credentials_and_explicit_actions() {
        for action in ["batch-preview", "batch-apply"] {
            let mut cli = crate::tests::parse_cli([
                "sunipdf-cli",
                "revision-transfer",
                action,
                "--manifest",
                "transfers.json",
            ])
            .unwrap();
            let crate::Command::RevisionTransfer(args) = &mut cli.command else {
                panic!("wrong command")
            };
            assert!(args.sources_mut().is_none());
            assert!(!args.new_password_stdin());
            assert!(args.output_path().is_none());
            let mut unexpected = crate::tests::parse_cli([
                "sunipdf-cli",
                "--password-env",
                "UNUSED_BATCH_PASSWORD",
                "revision-transfer",
                action,
                "--manifest",
                "transfers.json",
            ])
            .unwrap();
            assert!(unexpected.passwords.apply(&mut unexpected.command).is_err());
        }
    }

    #[test]
    fn page_pairs_are_explicit_bounded_and_one_to_one() {
        assert_eq!(parse_pairs("1:3, 2:4").unwrap().0, vec![(1, 3), (2, 4)]);
        for text in [
            "", "all", "0:1", "1:0", "1", "1:2:3", "1:1,", "1:1,1:2", "1:1,2:1",
        ] {
            assert!(parse_pairs(text).is_err(), "{text}");
        }
        let too_many = (1..=33)
            .map(|page| format!("{page}:{page}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_pairs(&too_many).is_err());
    }
}
