use clap::{Args, Subcommand};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::annotation_register::{RegisterFilter, RegisterSort};
use sunipdf_core::issue_report::{
    IssueReportFormat, IssueReportOptions, IssueReportSelection, SavedReportSource,
};
use sunipdf_core::quantity_table::{QuantityGrouping, QuantityOptions};
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroize;

#[derive(Args)]
pub struct IssueReportArgs {
    #[command(subcommand)]
    command: ReportCommand,
}
#[derive(Subcommand)]
enum ReportCommand {
    /// Prepare and inspect the complete report model and actual PDF pagination, without writing files.
    Inspect {
        #[command(flatten)]
        input: ReportInput,
    },
    /// Render one page of the actual report PDF into a new bounded PNG.
    Preview {
        #[command(flatten)]
        input: ReportInput,
        #[arg(long)]
        out: PathBuf,
        /// Report page, starting at 1 (independent of the source --page filter).
        #[arg(long, default_value_t=1, value_parser=positive)]
        preview_page: usize,
        #[arg(long, default_value_t=1024, value_parser=clap::value_parser!(u32).range(64..=1536))]
        long_side: u32,
    },
    /// Export a standalone report to a new file. Reports are not encrypted; source PDFs remain intact.
    Export {
        #[command(flatten)]
        input: ReportInput,
        #[arg(long)]
        out: PathBuf,
        #[arg(long,default_value="pdf",value_parser=["pdf","csv","json"])]
        format: String,
    },
}

#[derive(Args)]
pub struct ReportInput {
    /// Saved PDF (at most 128 MiB). Passwords use global --password-env or --password-stdin.
    file: PathBuf,
    #[arg(long, default_value = "Issue and quantity report")]
    title: String,
    /// Match register text/custom-field ordering, or the quantity table's typed-field search and ordering.
    #[arg(long, default_value="quantities", value_parser=["register","quantities"])]
    selection: String,
    #[arg(long)]
    no_legend: bool,
    /// Require this exact hash of the saved source PDF, not a serialized snapshot hash.
    #[arg(long,value_parser=sha256)]
    expected_source_sha256: Option<String>,
    #[arg(long, default_value = "")]
    query: String,
    #[arg(long,value_parser=positive)]
    page: Option<usize>,
    #[arg(long, default_value = "")]
    author: String,
    #[arg(long, default_value = "")]
    modified: String,
    #[arg(long, default_value = "")]
    review_status: String,
    #[arg(long, default_value = "")]
    business_status: String,
    #[arg(long, default_value = "")]
    discipline: String,
    #[arg(long, default_value = "")]
    kind: String,
    #[arg(long, default_value = "")]
    count_group: String,
    /// Match an assigned named zone on one physical page: PAGE:ID.
    #[arg(long,value_parser=zone)]
    zone: Option<sunipdf_core::plan_zones::ZoneFilter>,
    #[arg(long, requires = "zone")]
    zone_descendants: bool,
    #[arg(long,value_parser=zone_status)]
    zone_status: Option<sunipdf_core::plan_zones::ZoneAssignmentStatus>,
    #[arg(long)]
    include_replies: bool,
    #[arg(long,default_value="page",value_parser=["page","author","modified","kind","review_status","business_status","discipline","count_group"])]
    sort: String,
    #[arg(long)]
    sort_custom: Option<String>,
    #[arg(long)]
    descending: bool,
    #[arg(long,default_value="none",value_parser=["none","count_group","discipline","business_status"])]
    group_by: String,
    /// Existing bounded JSON array of up to eight report-only formula definitions.
    #[arg(long)]
    formulas: Option<PathBuf>,
    #[arg(skip)]
    password: Option<String>,
}
impl Drop for ReportInput {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}
impl IssueReportArgs {
    fn input(&self) -> &ReportInput {
        match &self.command {
            ReportCommand::Inspect { input }
            | ReportCommand::Preview { input, .. }
            | ReportCommand::Export { input, .. } => input,
        }
    }
    pub fn password_mut(&mut self) -> &mut Option<String> {
        match &mut self.command {
            ReportCommand::Inspect { input }
            | ReportCommand::Preview { input, .. }
            | ReportCommand::Export { input, .. } => &mut input.password,
        }
    }
    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            ReportCommand::Inspect { .. } => None,
            ReportCommand::Preview { out, .. } | ReportCommand::Export { out, .. } => Some(out),
        }
    }
}
fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Issue report: {message}"))
}
fn positive(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| "page numbers start at 1".into())
}
fn sha256(value: &str) -> Result<String, String> {
    if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("SHA256 must contain exactly 64 hexadecimal digits".into())
    }
}
fn zone(value: &str) -> Result<sunipdf_core::plan_zones::ZoneFilter, String> {
    let (page, id) = value
        .split_once(':')
        .ok_or_else(|| "zone requires PAGE:ID".to_string())?;
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("zone requires a valid zone ID".into());
    }
    Ok(sunipdf_core::plan_zones::ZoneFilter {
        page: positive(page)?,
        id: id.into(),
        descendants: false,
    })
}
fn zone_status(value: &str) -> Result<sunipdf_core::plan_zones::ZoneAssignmentStatus, String> {
    use sunipdf_core::plan_zones::ZoneAssignmentStatus as S;
    [
        S::Assigned,
        S::Boundary,
        S::Overlap,
        S::Unassigned,
        S::GeometryUnknown,
        S::Stale,
    ]
    .into_iter()
    .find(|s| s.id() == value)
    .ok_or_else(|| "unknown zone assignment status".into())
}
fn formulas(path: &Path) -> Result<Vec<sunipdf_core::quantity_table::FormulaColumn>, EngineError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("formula definitions must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > 64 * 1024 {
        return Err(invalid("formula definitions exceed 64 KiB"));
    }
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 {
        return Err(invalid("formula definitions exceed 64 KiB"));
    }
    QuantityOptions::formulas_from_json(&bytes)
}
fn options(input: &ReportInput, pages: usize) -> Result<IssueReportOptions, EngineError> {
    if let Some(page) = input.page {
        crate::page_index(page, pages)?;
    }
    let mut zone = input.zone.clone();
    if let Some(zone) = &mut zone {
        crate::page_index(zone.page, pages)?;
        zone.descendants = input.zone_descendants;
    }
    Ok(IssueReportOptions {
        title: input.title.clone(),
        selection: if input.selection == "register" {
            IssueReportSelection::Register
        } else {
            IssueReportSelection::Quantities
        },
        include_legend: !input.no_legend,
        quantities: QuantityOptions {
            filter: RegisterFilter {
                query: input.query.clone(),
                page: input.page,
                author: input.author.clone(),
                modified: input.modified.clone(),
                review_status: input.review_status.clone(),
                business_status: input.business_status.clone(),
                discipline: input.discipline.clone(),
                kind: input.kind.clone(),
                count_group: input.count_group.clone(),
                include_replies: input.include_replies,
                custom_sort: input.sort_custom.clone(),
                zone,
                zone_status: input.zone_status,
            },
            sort: RegisterSort::ALL
                .into_iter()
                .find(|v| v.id() == input.sort)
                .ok_or(EngineError::InvalidFormat)?,
            descending: input.descending,
            group_by: QuantityGrouping::ALL
                .into_iter()
                .find(|v| v.id() == input.group_by)
                .ok_or(EngineError::InvalidFormat)?,
            formulas: input
                .formulas
                .as_deref()
                .map(formulas)
                .transpose()?
                .unwrap_or_default(),
        },
    })
}
pub fn run(engine: &Engine, args: IssueReportArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.into()));
        }
    }
    let input = args.input();
    let cancel = AtomicBool::new(false);
    let (source, bytes) = SavedReportSource::capture(
        &input.file,
        input.expected_source_sha256.as_deref(),
        &cancel,
    )?;
    let document = engine.open_bytes(bytes, input.password.as_deref(), &input.file)?;
    let options = options(input, document.page_count())?;
    let table = document.quantity_table()?;
    let prepared =
        document.prepare_issue_report(&table, &options, Some(&source), &cancel, |_| {})?;
    match &args.command {
        ReportCommand::Inspect { .. } => Ok(prepared.model().json()),
        ReportCommand::Preview {
            out,
            preview_page,
            long_side,
            ..
        } => {
            let page = crate::page_index(*preview_page, prepared.page_count())?;
            let image =
                document.render_issue_report_preview(&prepared, page, *long_side, &cancel)?;
            let mut png = Vec::new();
            image::codecs::png::PngEncoder::new(&mut png)
                .write_image(
                    &image.rgba,
                    image.width,
                    image.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|_| invalid("could not encode report preview"))?;
            source.revalidate(&cancel)?;
            sunipdf_core::write_new_copy(out, &png)?;
            Ok(
                json!({"out":out,"preview_page":preview_page,"page_count":prepared.page_count(),"width":image.width,"height":image.height,"complete":prepared.model().complete,"provenance":prepared.model().provenance,"warnings":prepared.model().warnings,"report_published":false}),
            )
        }
        ReportCommand::Export { out, format, .. } => {
            let format = match format.as_str() {
                "pdf" => IssueReportFormat::Pdf,
                "csv" => IssueReportFormat::Csv,
                "json" => IssueReportFormat::Json,
                _ => return Err(EngineError::InvalidFormat),
            };
            Ok(document
                .publish_issue_report(&prepared, format, out, &cancel)?
                .json())
        }
    }
}
