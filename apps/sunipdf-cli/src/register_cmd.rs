use crate::password_input;
use clap::Args;
use std::path::PathBuf;
use sunipdf_core::{
    annotation_register::{RegisterFieldFilter, RegisterFields, RegisterFilter, RegisterSort},
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
    /// Match one register text field exactly (case-sensitive); KEY= matches a present empty value.
    #[arg(long, value_parser = field_equals)]
    pub field_equals: Option<RegisterFieldFilter>,
    /// Match an assigned zone on one physical page: PAGE:ID.
    #[arg(long, value_parser = zone_selection)]
    pub zone: Option<sunipdf_core::plan_zones::ZoneFilter>,
    /// Include descendants of --zone; each matched annotation still occurs only once.
    #[arg(long, requires = "zone")]
    pub zone_descendants: bool,
    /// Match assignment state, without interpreting unknown geometry as an empty zone.
    #[arg(long, value_parser = zone_status)]
    pub zone_status: Option<sunipdf_core::plan_zones::ZoneAssignmentStatus>,
    #[arg(long)]
    pub include_replies: bool,
    #[arg(long,default_value="page",value_parser=["page","author","modified","kind","review_status","business_status","discipline","count_group"])]
    pub sort: String,
    /// Sort by a custom column instead; text in the register, typed values in quantity-table.
    /// Missing values remain last in both directions, tied values use physical page/index.
    #[arg(long, value_parser = custom_sort_name)]
    pub sort_custom: Option<String>,
    #[arg(long)]
    pub descending: bool,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
}

fn zone_selection(value: &str) -> Result<sunipdf_core::plan_zones::ZoneFilter, String> {
    let (page, id) = value.split_once(':').ok_or("zone requires PAGE:ID")?;
    let page = page
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or("zone physical page numbers start at 1")?;
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(
            "zone ID requires 1 to 64 ASCII letters, digits, underscores or hyphens".into(),
        );
    }
    Ok(sunipdf_core::plan_zones::ZoneFilter {
        page,
        id: id.into(),
        descendants: false,
    })
}

fn zone_status(value: &str) -> Result<sunipdf_core::plan_zones::ZoneAssignmentStatus, String> {
    sunipdf_core::plan_zones::ZoneAssignmentStatus::ALL
        .into_iter()
        .find(|status| status.id() == value)
        .ok_or_else(|| {
            "zone status must be assigned, boundary, overlap, unassigned, geometry_unknown or stale"
                .into()
        })
}

pub fn selected_zone(
    args: &RegisterArgs,
    pages: usize,
) -> Result<Option<sunipdf_core::plan_zones::ZoneFilter>, EngineError> {
    let mut zone = args.zone.clone();
    if let Some(selected) = &mut zone {
        crate::page_index(selected.page, pages)?;
        selected.descendants = args.zone_descendants;
    }
    Ok(zone)
}

pub(crate) fn field_equals(value: &str) -> Result<RegisterFieldFilter, String> {
    let (key, value) = value
        .split_once('=')
        .ok_or("field-equals requires KEY=VALUE")?;
    let field = RegisterFieldFilter {
        key: key.into(),
        value: value.into(),
    };
    field.validate().map_err(|error| error.to_string())?;
    Ok(field)
}

fn custom_sort_name(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > 64
        || value.chars().any(char::is_control)
    {
        return Err("custom sort requires a column name of 1 to 64 UTF-8 bytes".into());
    }
    Ok(value.to_owned())
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
    let zone = selected_zone(&args, doc.page_count())?;
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
        custom_sort: args.sort_custom,
        custom_field: args.field_equals,
        zone,
        zone_status: args.zone_status,
    };
    let sort = RegisterSort::ALL
        .into_iter()
        .find(|s| s.id() == args.sort)
        .ok_or(EngineError::InvalidFormat)?;
    report.validate_zone_filter(&filter)?;
    let json = report.json(&filter, sort, args.descending);
    if let Some(out) = args.out {
        let bytes = if args.format == "csv" {
            report.csv(&filter, sort, args.descending)?.into_bytes()
        } else {
            serde_json::to_vec_pretty(&json).map_err(|e| EngineError::Other(e.to_string()))?
        };
        sunipdf_core::write_new_copy(&out, &bytes)?;
        Ok(
            serde_json::json!({"out":out,"format":args.format,"complete":json["complete"],"zone_coverage":json["zone_coverage"],"matched_annotations":json["matched_annotations"],"source_sha256":report.source_sha256,"original_unchanged":true}),
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

#[cfg(test)]
mod field_filter_tests {
    use crate::tests::parse_cli;

    #[test]
    fn register_and_quantity_cli_preserve_exact_empty_and_equals_values() {
        for command in ["annotation-register", "quantity-table"] {
            for (argument, expected) in [
                ("Room=A-101", "A-101"),
                ("Room=", ""),
                ("Room= A=B ", " A=B "),
            ] {
                let parsed = parse_cli([
                    "sunipdf-cli",
                    command,
                    "source.pdf",
                    "--field-equals",
                    argument,
                ])
                .unwrap();
                let filter = match parsed.command {
                    crate::Command::AnnotationRegister(args) => args.field_equals,
                    crate::Command::QuantityTable(args) => args.report.field_equals,
                    _ => panic!("register filter command"),
                }
                .unwrap();
                assert_eq!(filter.key, "Room");
                assert_eq!(filter.value, expected);
            }
            for value in ["Room", "=A", " Room=A", "Room=A\nB"] {
                assert!(parse_cli([
                    "sunipdf-cli",
                    command,
                    "source.pdf",
                    "--field-equals",
                    value
                ])
                .is_err());
            }
            assert!(parse_cli([
                "sunipdf-cli",
                command,
                "source.pdf",
                "--field-equals",
                "Room=A",
                "--field-equals",
                "Room=B"
            ])
            .is_err());
        }
    }
}
