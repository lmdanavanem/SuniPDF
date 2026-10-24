use clap::{Args, Subcommand};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::issue_report::SavedReportSource;
use sunipdf_core::quantity_workbook::{
    QuantityWorkbookManifest, QuantityWorkbookOptions, WorkbookFormat, MAX_MANIFEST_BYTES,
};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroize;

#[derive(Args)]
pub struct QuantityWorkbookArgs {
    #[command(subcommand)]
    command: WorkbookCommand,
}
#[derive(Subcommand)]
enum WorkbookCommand {
    /// Print the complete default settings JSON for an editable, reproducible export configuration.
    Defaults,
    /// Inspect current quantities, totals, coverage and the explicit refresh comparison without writing files.
    Inspect {
        #[command(flatten)]
        input: WorkbookInput,
    },
    /// Write a new XLSX snapshot or refresh manifest. Existing files are never replaced.
    Export {
        #[command(flatten)]
        input: WorkbookInput,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value="xlsx", value_parser=["xlsx", "manifest"])]
        format: String,
    },
}
#[derive(Args)]
struct WorkbookInput {
    /// Current local PDF, at most 128 MiB. Passwords use --password-env or --password-stdin.
    file: PathBuf,
    /// Strict settings JSON (see defaults). Refreshes retain the settings in --previous instead.
    #[arg(long, conflicts_with = "previous")]
    settings: Option<PathBuf>,
    /// Previously exported SuniPDF JSON manifest, not an arbitrary spreadsheet.
    #[arg(long)]
    previous: Option<PathBuf>,
    /// Also include newly matching unique annotation IDs during this explicit refresh.
    #[arg(long, requires = "previous")]
    include_new: bool,
    /// Optional report title; quantity selection and formulas remain those of the manifest/settings.
    #[arg(long)]
    title: Option<String>,
    /// Require this SHA256 of saved PDF bytes (distinct from the serialized document snapshot).
    #[arg(long, value_parser=sha256)]
    expected_source_sha256: Option<String>,
    #[arg(skip)]
    password: Option<String>,
}
impl Drop for WorkbookInput {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}
impl QuantityWorkbookArgs {
    pub fn password_mut(&mut self) -> Option<&mut Option<String>> {
        match &mut self.command {
            WorkbookCommand::Defaults => None,
            WorkbookCommand::Inspect { input } | WorkbookCommand::Export { input, .. } => {
                Some(&mut input.password)
            }
        }
    }
    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            WorkbookCommand::Export { out, .. } => Some(out),
            _ => None,
        }
    }
}
fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Quantity workbook: {message}"))
}
fn sha256(value: &str) -> Result<String, String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("SHA256 requires exactly 64 hexadecimal digits".into())
    }
}
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>, EngineError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("configuration must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit as u64 {
        return Err(invalid("configuration exceeds its byte limit"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("configuration exceeds its byte limit"));
    }
    Ok(bytes)
}
pub fn run(engine: &Engine, args: QuantityWorkbookArgs) -> Result<Value, EngineError> {
    let input = match &args.command {
        WorkbookCommand::Defaults => {
            return serde_json::to_value(QuantityWorkbookOptions::default())
                .map_err(|error| invalid(&error.to_string()))
        }
        WorkbookCommand::Inspect { input } | WorkbookCommand::Export { input, .. } => input,
    };
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.into()));
        }
    }
    let previous = input
        .previous
        .as_ref()
        .map(|path| QuantityWorkbookManifest::from_json(&bounded(path, MAX_MANIFEST_BYTES)?))
        .transpose()?;
    let mut options = if let Some(previous) = &previous {
        previous.options.clone()
    } else if let Some(settings) = &input.settings {
        QuantityWorkbookOptions::from_json(&bounded(settings, 64 * 1024)?)?
    } else {
        QuantityWorkbookOptions::default()
    };
    if let Some(title) = &input.title {
        options.title.clone_from(title);
    }
    options.validate()?;
    let cancel = AtomicBool::new(false);
    let (source, bytes) = SavedReportSource::capture(
        &input.file,
        input.expected_source_sha256.as_deref(),
        &cancel,
    )?;
    let document = engine.open_bytes(bytes, input.password.as_deref(), &input.file)?;
    let table = document.quantity_table()?;
    let prepared = document.prepare_quantity_workbook(
        &table,
        &options,
        previous.as_ref(),
        input.include_new,
        Some(&source),
        &cancel,
        |_| {},
    )?;
    match &args.command {
        WorkbookCommand::Inspect { .. } => Ok(prepared.model().json()),
        WorkbookCommand::Export { out, format, .. } => {
            let format = match format.as_str() {
                "xlsx" => WorkbookFormat::Xlsx,
                "manifest" => WorkbookFormat::Manifest,
                _ => return Err(EngineError::InvalidFormat),
            };
            Ok(document
                .publish_quantity_workbook(&prepared, format, out, &cancel)?
                .json())
        }
        WorkbookCommand::Defaults => unreachable!(),
    }
}
