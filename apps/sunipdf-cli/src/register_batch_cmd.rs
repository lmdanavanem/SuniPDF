use clap::{Args, Subcommand};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::annotation_register_batch::{
    register_row_fingerprint, RegisterFieldsPatch, MAX_REGISTER_BATCH_ROWS,
};
use sunipdf_core::issue_report::SavedReportSource;
use sunipdf_core::{Engine, EngineError};
use zeroize::Zeroize;

const MAX_PLAN_BYTES: u64 = 1024 * 1024;

#[derive(Args)]
pub struct RegisterBatchArgs {
    #[command(subcommand)]
    command: BatchCommand,
}

#[derive(Args)]
struct Source {
    /// Saved PDF, at most 128 MiB. Protected inputs use --password-env or --password-stdin.
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
enum BatchCommand {
    /// List source-bound row fingerprints and current fields without selecting or editing rows.
    Inspect {
        #[command(flatten)]
        source: Source,
        /// Limit the inventory to one physical page, starting at 1.
        #[arg(long, value_parser = positive_page)]
        page: Option<usize>,
    },
    /// Validate all explicit targets and show each before/after field change without saving a PDF.
    Preview {
        #[command(flatten)]
        source: Source,
        /// Strict version-1 JSON, at most 1 MiB, binding the saved PDF and at most 256 exact rows.
        #[arg(long)]
        plan: PathBuf,
    },
    /// Revalidate and apply one complete field patch to explicit rows in a new PDF copy.
    Apply {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        plan: PathBuf,
        /// New PDF only. Existing destinations are refused, including with --overwrite.
        #[arg(long)]
        out: PathBuf,
    },
}

impl RegisterBatchArgs {
    pub fn password_mut(&mut self) -> &mut Option<String> {
        let source = match &mut self.command {
            BatchCommand::Inspect { source, .. }
            | BatchCommand::Preview { source, .. }
            | BatchCommand::Apply { source, .. } => source,
        };
        &mut source.password
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            BatchCommand::Apply { out, .. } => Some(out),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchPlan {
    version: u32,
    expected_source_sha256: String,
    targets: Vec<ExpectedRow>,
    patch: RegisterFieldsPatch,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedRow {
    /// Physical one-based page and zero-based annotation index.
    page: usize,
    index: usize,
    expected_row_sha256: String,
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Annotation field batch: {message}"))
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "physical page numbers start at 1".into())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_plan(bytes: &[u8]) -> Result<BatchPlan, EngineError> {
    if bytes.len() as u64 > MAX_PLAN_BYTES {
        return Err(invalid("plan exceeds 1 MiB"));
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let plan: BatchPlan = serde_json::from_slice(bytes)
        .map_err(|error| invalid(&format!("invalid plan JSON: {error}")))?;
    if plan.version != 1
        || !valid_hash(&plan.expected_source_sha256)
        || plan.targets.is_empty()
        || plan.targets.len() > MAX_REGISTER_BATCH_ROWS
    {
        return Err(invalid(
            "version 1, an exact source SHA256 and 1 to 256 explicit targets are required",
        ));
    }
    let mut selected = HashSet::new();
    for row in &plan.targets {
        if row.page == 0
            || !valid_hash(&row.expected_row_sha256)
            || !selected.insert((row.page, row.index))
        {
            return Err(invalid(
                "targets require distinct page/index pairs and exact row SHA256 values",
            ));
        }
    }
    plan.patch.validate()?;
    Ok(plan)
}

fn read_plan(path: &Path) -> Result<BatchPlan, EngineError> {
    let file = sunipdf_core::file_io::open_regular(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_PLAN_BYTES {
        return Err(invalid("plan must be a regular file of at most 1 MiB"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_PLAN_BYTES + 1).read_to_end(&mut bytes)?;
    parse_plan(&bytes)
}

pub fn run(engine: &Engine, args: RegisterBatchArgs) -> Result<Value, EngineError> {
    if let Some(output) = args.output_path() {
        if output.exists() {
            return Err(EngineError::AlreadyExists(output.to_path_buf()));
        }
    }
    let (source, plan_path) = match &args.command {
        BatchCommand::Inspect { source, .. } => (source, None),
        BatchCommand::Preview { source, plan } | BatchCommand::Apply { source, plan, .. } => {
            (source, Some(plan))
        }
    };
    let requested = plan_path.map(|path| read_plan(path)).transpose()?;
    let cancel = AtomicBool::new(false);
    let (saved, bytes) = SavedReportSource::capture(
        &source.file,
        requested
            .as_ref()
            .map(|plan| plan.expected_source_sha256.as_str()),
        &cancel,
    )?;
    let mut document = engine.open_bytes(bytes, source.password.as_deref(), &source.file)?;
    let register = document.annotation_register()?;
    if let BatchCommand::Inspect { page, .. } = &args.command {
        if let Some(page) = page {
            crate::page_index(*page, document.page_count())?;
        }
        let rows = register
            .rows
            .iter()
            .filter(|row| page.is_none_or(|page| row.page == page))
            .map(|row| {
                Ok(json!({
                    "page": row.page, "index": row.index,
                    "annotation_id": row.annotation_id, "kind": row.kind,
                    "locked": row.locked, "is_reply": row.is_reply,
                    "fields": row.fields, "expected_row_sha256": register_row_fingerprint(row)?,
                }))
            })
            .collect::<Result<Vec<_>, EngineError>>()?;
        saved.revalidate(&cancel)?;
        return Ok(json!({
            "schema": "sunipdf-annotation-field-batch-inventory", "version": 1,
            "source_file_sha256": saved.sha256(), "rows": rows,
            "selected_rows": 0, "original_unchanged": true,
        }));
    }
    let requested = requested.ok_or_else(|| invalid("an explicit plan is required"))?;
    let mut selected = Vec::with_capacity(requested.targets.len());
    for target in &requested.targets {
        let row = register
            .rows
            .iter()
            .find(|row| row.page == target.page && row.index == target.index)
            .ok_or_else(|| invalid("a selected annotation is missing; inspect the source again"))?;
        if !register_row_fingerprint(row)?.eq_ignore_ascii_case(&target.expected_row_sha256) {
            return Err(invalid(
                "a selected annotation changed; inspect the source again",
            ));
        }
        selected.push(row.clone());
    }
    let prepared =
        document.preview_annotation_register_batch(&register, &selected, &requested.patch)?;
    saved.revalidate(&cancel)?;
    if let Some(output) = args.output_path() {
        let report = document.apply_annotation_register_batch(&prepared)?;
        let bytes = document.snapshot()?;
        saved.revalidate(&cancel)?;
        sunipdf_core::write_new_copy(output, &bytes)?;
        Ok(json!({
            "schema": "sunipdf-annotation-field-batch-result", "version": 1,
            "source_file_sha256": saved.sha256(), "out": output,
            "changes": report, "committed": true, "original_unchanged": true,
            "encrypted": document.properties().encrypted,
        }))
    } else {
        Ok(json!({
            "schema": "sunipdf-annotation-field-batch-preview", "version": 1,
            "source_file_sha256": saved.sha256(), "changes": prepared.report(),
            "committed": false, "original_unchanged": true,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Value {
        json!({"version":1,"expected_source_sha256":"a".repeat(64),
            "targets":[{"page":1,"index":0,"expected_row_sha256":"b".repeat(64)}],
            "patch":{"business_status":{"operation":"set","value":"Reviewed"}}})
    }

    #[test]
    fn batch_manifest_requires_exact_bounded_explicit_targets() {
        let value = plan();
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(parse_plan(&bytes).unwrap().targets.len(), 1);
        let bom = [b"\xef\xbb\xbf".as_slice(), bytes.as_slice()].concat();
        assert!(parse_plan(&bom).is_ok());
        let mut invalids = Vec::new();
        for (key, replacement) in [
            ("version", json!(2)),
            ("version", json!(1.0)),
            ("expected_source_sha256", json!("a".repeat(63))),
            ("expected_source_sha256", json!("x".repeat(64))),
            ("targets", json!([])),
        ] {
            let mut invalid = value.clone();
            invalid[key] = replacement;
            invalids.push(invalid);
        }
        for (key, replacement) in [
            ("page", json!(0)),
            ("index", json!(-1)),
            ("expected_row_sha256", json!("stale")),
            ("unrecognized", json!(true)),
        ] {
            let mut invalid = value.clone();
            invalid["targets"][0][key] = replacement;
            invalids.push(invalid);
        }
        let mut duplicate = value.clone();
        duplicate["targets"] = json!([value["targets"][0], value["targets"][0]]);
        invalids.push(duplicate);
        let mut too_many = value.clone();
        too_many["targets"] = Value::Array(
            (0..257)
                .map(|index| {
                    let mut row = value["targets"][0].clone();
                    row["index"] = json!(index);
                    row
                })
                .collect(),
        );
        invalids.push(too_many);
        for invalid in invalids {
            assert!(
                parse_plan(&serde_json::to_vec(&invalid).unwrap()).is_err(),
                "{invalid}"
            );
        }
        assert!(parse_plan(&vec![b' '; MAX_PLAN_BYTES as usize + 1]).is_err());
        let duplicate_custom = String::from_utf8(bytes).unwrap().replace(
            "\"patch\":{", "\"patch\":{\"custom\":{\"Room\":{\"operation\":\"clear\"},\"Room\":{\"operation\":\"keep\"}},");
        assert!(parse_plan(duplicate_custom.as_bytes()).is_err());
    }
}
