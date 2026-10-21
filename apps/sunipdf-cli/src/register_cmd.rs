use crate::password_input;
use clap::Args;
use std::path::PathBuf;
use sunipdf_core::{
    annotation_register::{RegisterFields, RegisterFilter, RegisterSort},
    Engine, EngineError,
};

#[derive(Args)]
pub struct CountArgs {
    pub file: PathBuf,
    /// Equipment/type group; each explicitly placed marker counts as one.
    #[arg(long)]
    pub group: String,
    /// Displayed-page centre in points: x,y (bottom-left origin).
    #[arg(long)]
    pub point: String,
    #[arg(long, default_value_t = 18.0)]
    pub size: f32,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct RegisterArgs {
    pub file: PathBuf,
    /// Without --out, return JSON on stdout. CSV requires an explicit output path.
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long, default_value="json",value_parser=["json","csv"])]
    pub format: String,
    #[arg(long, default_value = "")]
    pub query: String,
    #[arg(long)]
    pub page: Option<usize>,
    #[arg(long, default_value = "")]
    pub author: String,
    /// Substring of the original PDF modification date (for example D:202610).
    #[arg(long, default_value = "")]
    pub modified: String,
    #[arg(long, default_value = "")]
    pub review_status: String,
    #[arg(long, default_value = "")]
    pub business_status: String,
    #[arg(long, default_value = "")]
    pub discipline: String,
    #[arg(long, default_value = "")]
    pub kind: String,
    #[arg(long, default_value = "")]
    pub count_group: String,
    #[arg(long)]
    pub include_replies: bool,
    #[arg(long,default_value="page",value_parser=["page","author","modified","kind","review_status","business_status","discipline","count_group"])]
    pub sort: String,
    #[arg(long)]
    pub descending: bool,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct FieldsArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    /// Zero-based index in the page's /Annots array, as returned by annotation-register.
    #[arg(long)]
    pub index: usize,
    /// These fields replace the current register fields; omitted fields are cleared.
    #[arg(long)]
    pub business_status: Option<String>,
    #[arg(long)]
    pub discipline: Option<String>,
    /// Repeat KEY=VALUE, at most 16 text fields; duplicate keys are refused.
    #[arg(long = "field")]
    pub fields: Vec<String>,
    /// Regroup an existing count marker, preserving its identity and position.
    #[arg(long)]
    pub count_group: Option<String>,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

pub fn count(engine: &Engine, args: CountArgs) -> Result<serde_json::Value, EngineError> {
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    let point: Vec<f32> = args
        .point
        .split(',')
        .map(|s| {
            s.trim()
                .parse()
                .map_err(|_| EngineError::Other("--point requires x,y".into()))
        })
        .collect::<Result<_, _>>()?;
    let [x, y] = point.as_slice() else {
        return Err(EngineError::Other("--point requires x,y".into()));
    };
    let index = doc.add_count_marker(
        crate::page_index(args.page, doc.page_count())?,
        (*x, *y),
        args.size,
        &args.group,
        args.author.as_deref(),
    )?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(
        serde_json::json!({"out":args.out,"page":args.page,"index":index,"group":args.group,"quantity":1,"original_unchanged":true,"encrypted":doc.properties().encrypted}),
    )
}

pub fn register(engine: &Engine, args: RegisterArgs) -> Result<serde_json::Value, EngineError> {
    let doc = engine.open(&args.file, args.password.as_deref())?;
    if let Some(page) = args.page {
        crate::page_index(page, doc.page_count())?;
    }
    let report = doc.annotation_register()?;
    let filter = RegisterFilter {
        query: args.query,
        page: args.page,
        author: args.author,
        modified: args.modified,
        review_status: args.review_status,
        business_status: args.business_status,
        discipline: args.discipline,
        kind: args.kind,
        count_group: args.count_group,
        include_replies: args.include_replies,
    };
    let sort = RegisterSort::ALL
        .into_iter()
        .find(|s| s.id() == args.sort)
        .ok_or(EngineError::InvalidFormat)?;
    let json = report.json(&filter, sort, args.descending);
    if let Some(out) = args.out {
        let bytes = if args.format == "csv" {
            report.csv(&filter, sort, args.descending).into_bytes()
        } else {
            serde_json::to_vec_pretty(&json).map_err(|e| EngineError::Other(e.to_string()))?
        };
        sunipdf_core::write_new_copy(&out, &bytes)?;
        Ok(
            serde_json::json!({"out":out,"format":args.format,"matched_annotations":json["matched_annotations"],"source_sha256":report.source_sha256,"original_unchanged":true}),
        )
    } else if args.format == "csv" {
        Err(EngineError::Other("CSV export requires --out".into()))
    } else {
        Ok(json)
    }
}

pub fn fields(engine: &Engine, args: FieldsArgs) -> Result<serde_json::Value, EngineError> {
    let mut doc = engine.open(&args.file, args.password.as_deref())?;
    crate::page_index(args.page, doc.page_count())?;
    let report = doc.annotation_register()?;
    let row = report
        .rows
        .iter()
        .find(|r| r.page == args.page && r.index == args.index)
        .ok_or_else(|| EngineError::Other("annotation not found".into()))?;
    let mut fields = RegisterFields {
        business_status: args.business_status,
        discipline: args.discipline,
        ..Default::default()
    };
    for field in args.fields {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| EngineError::Other("--field requires KEY=VALUE".into()))?;
        if fields.custom.insert(key.into(), value.into()).is_some() {
            return Err(EngineError::Other("duplicate custom field".into()));
        }
    }
    doc.set_annotation_register_fields(&report.target(row), &fields, args.count_group.as_deref())?;
    sunipdf_core::write_new_copy(&args.out, &doc.snapshot()?)?;
    Ok(
        serde_json::json!({"out":args.out,"page":args.page,"index":args.index,"original_unchanged":true,"encrypted":doc.properties().encrypted}),
    )
}
