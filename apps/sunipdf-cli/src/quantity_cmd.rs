use crate::{password_input, register_cmd};
use clap::Args;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::annotation_register::{RegisterFilter, RegisterSort};
use sunipdf_core::quantity_table::{
    QuantityFields, QuantityGrouping, QuantityOptions, QuantityRow,
};
use sunipdf_core::{Engine, EngineError};

#[derive(Args)]
pub struct TableArgs {
    #[command(flatten)]
    pub report: register_cmd::RegisterArgs,
    /// Group totals without combining incompatible units or converting currencies.
    #[arg(long, default_value="none", value_parser=["none", "count_group", "discipline", "business_status"])]
    pub group_by: String,
    /// JSON array of up to eight report-only formula definitions. No scripts or implicit unit conversion.
    #[arg(long)]
    pub formulas: Option<PathBuf>,
}

#[derive(Args)]
pub struct TargetArgs {
    pub file: PathBuf,
    /// Physical page number, starting at 1; must match the expected row.
    #[arg(long, value_parser=positive_page)]
    pub page: usize,
    /// One complete row from quantity-table JSON (maximum 64 KiB).
    #[arg(long)]
    pub expected_record: PathBuf,
    /// New PDF copy; existing destinations are refused even with --overwrite.
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct SetArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    /// Strict JSON fields. Number columns accept an optional exact unit, e.g. {"type":"number","value":"0.25","unit":"m"}; omitted/null means no unit.
    /// Replaces all quantity fields. Custom values have type text, date, number or choice.
    #[arg(long)]
    pub fields: PathBuf,
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "page numbers start at 1".into())
}

fn ensure_new(path: &Path) -> Result<(), EngineError> {
    if path.exists() {
        return Err(EngineError::AlreadyExists(path.to_path_buf()));
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Vec<u8>, EngineError> {
    const MAXIMUM: u64 = 64 * 1024;
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(EngineError::Other(
            "quantity JSON must be a regular file".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAXIMUM + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAXIMUM {
        return Err(EngineError::Other("quantity JSON exceeds 64 KiB".into()));
    }
    Ok(bytes
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .unwrap_or(&bytes)
        .to_vec())
}

pub fn table(engine: &Engine, args: TableArgs) -> Result<Value, EngineError> {
    let args_report = args.report;
    if let Some(out) = &args_report.out {
        ensure_new(out)?;
    } else if args_report.format == "csv" {
        return Err(EngineError::Other("CSV export requires --out".into()));
    }
    let document = engine.open(&args_report.file, args_report.password.as_deref())?;
    if let Some(page) = args_report.page {
        crate::page_index(page, document.page_count())?;
    }
    let options = QuantityOptions {
        filter: RegisterFilter {
            zone: register_cmd::selected_zone(&args_report, document.page_count())?,
            zone_status: args_report.zone_status,
            query: args_report.query,
            page: args_report.page,
            author: args_report.author,
            modified: args_report.modified,
            review_status: args_report.review_status,
            business_status: args_report.business_status,
            discipline: args_report.discipline,
            kind: args_report.kind,
            count_group: args_report.count_group,
            include_replies: args_report.include_replies,
            custom_sort: args_report.sort_custom,
        },
        sort: RegisterSort::ALL
            .into_iter()
            .find(|sort| sort.id() == args_report.sort)
            .ok_or(EngineError::InvalidFormat)?,
        descending: args_report.descending,
        group_by: QuantityGrouping::ALL
            .into_iter()
            .find(|group| group.id() == args.group_by)
            .ok_or(EngineError::InvalidFormat)?,
        formulas: args
            .formulas
            .as_ref()
            .map(|path| QuantityOptions::formulas_from_json(&read_json(path)?))
            .transpose()?
            .unwrap_or_default(),
    };
    let table = document.quantity_table()?;
    // The core read API is also used for on-screen display; only its export API authorizes output.
    let export = document.export_quantity_table(&table, &options)?;
    if let Some(out) = args_report.out {
        let bytes = if args_report.format == "csv" {
            export.csv.into_bytes()
        } else {
            serde_json::to_vec_pretty(&export.json)
                .map_err(|error| EngineError::Other(error.to_string()))?
        };
        sunipdf_core::write_new_copy(&out, &bytes)?;
        Ok(serde_json::json!({
            "out":out,"format":args_report.format,"source_sha256":table.source_sha256(),
            "complete":export.json["complete"],"zone_coverage":export.json["zone_coverage"],
            "original_unchanged":true
        }))
    } else {
        Ok(export.json)
    }
}

pub fn set(engine: &Engine, args: SetArgs) -> Result<Value, EngineError> {
    ensure_new(&args.target.out)?;
    let fields = QuantityFields::from_json(&read_json(&args.fields)?)?;
    update(engine, args.target, Some(fields))
}

pub fn clear(engine: &Engine, args: TargetArgs) -> Result<Value, EngineError> {
    update(engine, args, None)
}

fn update(
    engine: &Engine,
    args: TargetArgs,
    fields: Option<QuantityFields>,
) -> Result<Value, EngineError> {
    ensure_new(&args.out)?;
    let expected = QuantityRow::from_json(&read_json(&args.expected_record)?)?;
    let mut document = engine.open(&args.file, args.password.as_deref())?;
    let page = crate::page_index(args.page, document.page_count())?;
    let target = document.quantity_target(page, &expected)?;
    let cleared = fields.is_none();
    if let Some(fields) = fields {
        document.set_quantity_fields(&target, &fields)?;
    } else {
        document.clear_quantity_fields(&target)?;
    }
    sunipdf_core::write_new_copy(&args.out, &document.snapshot()?)?;
    Ok(serde_json::json!({
        "out":args.out,"page":args.page,"cleared":cleared,"original_unchanged":true,
        "encrypted":document.properties().encrypted,"source_content_exported":false
    }))
}
