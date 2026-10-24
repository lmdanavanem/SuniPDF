//! Local quantity snapshots and reviewed refreshes; spreadsheets never execute formulas.

use crate::issue_report::SavedReportSource;
use crate::quantity_table::{
    DecimalValue, FormulaValue, QuantityOptions, QuantityRow, QuantityTable,
};
use crate::{Document, EngineError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_MANIFEST_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_WORKBOOK_ROWS: usize = 10_000;
const SCHEMA: &str = "sunipdf-quantity-workbook";

pub(crate) fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Quantity workbook: {message}"))
}
pub(crate) fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}
fn digest(bytes: &[u8]) -> String {
    crate::plan_zones::source_sha256(bytes)
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityWorkbookOptions {
    pub title: String,
    pub quantities: QuantityOptions,
}
impl Default for QuantityWorkbookOptions {
    fn default() -> Self {
        Self {
            title: "Quantity workbook".into(),
            quantities: QuantityOptions::default(),
        }
    }
}
impl QuantityWorkbookOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.title.trim().is_empty()
            || self.title.len() > 512
            || self.title.chars().any(char::is_control)
        {
            return Err(invalid(
                "title requires 1..512 bytes without control characters",
            ));
        }
        let bytes = serde_json::to_vec(&self.quantities).map_err(|e| invalid(&e.to_string()))?;
        if bytes.len() > 64 * 1024 {
            return Err(invalid("configuration exceeds 64 KiB"));
        }
        let filter = &self.quantities.filter;
        if [
            &filter.query,
            &filter.author,
            &filter.modified,
            &filter.review_status,
            &filter.business_status,
            &filter.discipline,
            &filter.kind,
            &filter.count_group,
        ]
        .into_iter()
        .any(|value| value.len() > 4096)
            || filter
                .custom_sort
                .as_ref()
                .is_some_and(|value| value.len() > 4096)
            || filter.page == Some(0)
            || filter.zone.as_ref().is_some_and(|zone| {
                zone.page == 0
                    || zone.id.is_empty()
                    || zone.id.len() > 64
                    || !zone
                        .id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            })
        {
            return Err(invalid(
                "configuration contains an invalid or excessive filter",
            ));
        }
        self.quantities.validate_formulas()
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > 64 * 1024 {
            return Err(invalid("configuration exceeds 64 KiB"));
        }
        let options: Self = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
        options.validate()?;
        Ok(options)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbookProvenance {
    pub snapshot_sha256: String,
    pub saved_file_sha256: Option<String>,
    pub source_encrypted: bool,
    pub source_signature_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbookBaselineRow {
    pub row: QuantityRow,
    /// Prior calculated values are comparison evidence only, never calculation inputs.
    pub formulas: BTreeMap<String, FormulaValue>,
    pub comparison_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityWorkbookManifest {
    pub schema: String,
    pub version: u32,
    pub options: QuantityWorkbookOptions,
    pub provenance: WorkbookProvenance,
    pub rows: Vec<WorkbookBaselineRow>,
    /// Earlier identities that could not be resolved remain tracked in later refreshes.
    pub pending_rows: Vec<WorkbookBaselineRow>,
}
impl QuantityWorkbookManifest {
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("manifest exceeds 32 MiB"));
        }
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }
    fn validate(&self) -> Result<(), EngineError> {
        if self.schema != SCHEMA
            || self.version != 1
            || self.rows.len().saturating_add(self.pending_rows.len()) > MAX_WORKBOOK_ROWS
            || !valid_sha(&self.provenance.snapshot_sha256)
            || self
                .provenance
                .saved_file_sha256
                .as_deref()
                .is_some_and(|s| !valid_sha(s))
        {
            return Err(invalid("unsupported or malformed manifest"));
        }
        self.options.validate()?;
        let names: HashSet<_> = self
            .options
            .quantities
            .formulas
            .iter()
            .map(|formula| formula.name.as_str())
            .collect();
        for row in self.rows.iter().chain(&self.pending_rows) {
            let encoded = serde_json::to_vec(&row.row).map_err(|e| invalid(&e.to_string()))?;
            QuantityRow::from_json(&encoded)?;
            if row
                .formulas
                .keys()
                .any(|name| !names.contains(name.as_str()))
                || row.formulas.len() != names.len()
                || row.comparison_sha256 != comparison_hash(&row.row, &row.formulas)?
            {
                return Err(invalid("manifest comparison evidence is inconsistent"));
            }
            for cell in row.formulas.values() {
                if cell.value.is_some() == cell.error.is_some() {
                    return Err(invalid(
                        "manifest formula cell must contain a value or an error",
                    ));
                }
                if let Some(value) = &cell.value {
                    DecimalValue::parse(value)?;
                }
            }
            if row.row.register.annotation_id.as_deref().is_some_and(|id| {
                id.is_empty() || id.len() > 256 || id.chars().any(char::is_control)
            }) {
                return Err(invalid("invalid annotation identity in manifest"));
            }
        }
        Ok(())
    }
    pub fn json(&self) -> Value {
        serde_json::to_value(self).expect("workbook manifest")
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbookChangeStatus {
    Unchanged,
    Changed,
    Missing,
    Ambiguous,
    Unquantified,
    Added,
    ExcludedNew,
    Untracked,
}
impl WorkbookChangeStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Changed => "changed",
            Self::Missing => "missing",
            Self::Ambiguous => "ambiguous",
            Self::Unquantified => "unquantified",
            Self::Added => "added",
            Self::ExcludedNew => "excluded_new",
            Self::Untracked => "untracked",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkbookChange {
    pub annotation_id: Option<String>,
    pub status: WorkbookChangeStatus,
    pub before: Option<WorkbookBaselineRow>,
    pub after: Option<WorkbookBaselineRow>,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbookPhase {
    Collecting,
    Comparing,
    Packaging,
    Ready,
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WorkbookProgress {
    pub phase: WorkbookPhase,
    pub completed: usize,
    pub total: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct QuantityWorkbookModel {
    pub schema: &'static str,
    pub version: u32,
    pub title: String,
    pub provenance: WorkbookProvenance,
    pub quantities: Value,
    pub changes: Vec<WorkbookChange>,
    pub refresh: bool,
    pub include_new: bool,
    pub previous_snapshot_sha256: Option<String>,
    pub complete: bool,
    pub warnings: Vec<String>,
    pub refreshable_rows: usize,
}
impl QuantityWorkbookModel {
    pub fn json(&self) -> Value {
        serde_json::to_value(self).expect("workbook model")
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbookFormat {
    Xlsx,
    Manifest,
}
pub struct PreparedQuantityWorkbook {
    original: Vec<u8>,
    source: Option<SavedReportSource>,
    model: QuantityWorkbookModel,
    manifest: QuantityWorkbookManifest,
    manifest_bytes: Vec<u8>,
    xlsx: Vec<u8>,
}
impl std::fmt::Debug for PreparedQuantityWorkbook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedQuantityWorkbook")
            .field("rows", &self.manifest.rows.len())
            .finish_non_exhaustive()
    }
}
impl PreparedQuantityWorkbook {
    pub fn model(&self) -> &QuantityWorkbookModel {
        &self.model
    }
    pub fn manifest(&self) -> &QuantityWorkbookManifest {
        &self.manifest
    }
    pub fn manifest_json(&self) -> Value {
        self.manifest.json()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkbookReceipt {
    pub out: PathBuf,
    pub format: WorkbookFormat,
    pub output_sha256: String,
    pub bytes: usize,
    pub matched_rows: usize,
    pub complete: bool,
    pub provenance: WorkbookProvenance,
    pub source_unchanged: bool,
    pub output_encrypted: bool,
}
impl WorkbookReceipt {
    pub fn json(&self) -> Value {
        serde_json::to_value(self).expect("workbook receipt")
    }
}

fn comparison_hash(
    row: &QuantityRow,
    formulas: &BTreeMap<String, FormulaValue>,
) -> Result<String, EngineError> {
    let mut value = serde_json::to_value(row).map_err(|e| invalid(&e.to_string()))?;
    let register = value["register"].as_object_mut().expect("register row");
    register.remove("page");
    register.remove("index");
    value
        .as_object_mut()
        .expect("quantity row")
        .remove("editable");
    Ok(digest(
        &serde_json::to_vec(&json!({"row":value,"formulas":formulas}))
            .map_err(|e| invalid(&e.to_string()))?,
    ))
}
fn baseline_rows(quantities: &Value) -> Result<Vec<WorkbookBaselineRow>, EngineError> {
    let formulas: HashMap<_, _> = quantities["formula_rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            (
                (
                    row["page"].as_u64().unwrap_or(0),
                    row["index"].as_u64().unwrap_or(0),
                ),
                row["values"].clone(),
            )
        })
        .collect();
    quantities["rows"]
        .as_array()
        .ok_or_else(|| invalid("missing quantity rows"))?
        .iter()
        .map(|value| {
            let row: QuantityRow =
                serde_json::from_value(value.clone()).map_err(|e| invalid(&e.to_string()))?;
            let values = formulas
                .get(&(row.register.page as u64, row.register.index as u64))
                .cloned()
                .unwrap_or(json!({}));
            let formulas: BTreeMap<String, FormulaValue> =
                serde_json::from_value(values).map_err(|e| invalid(&e.to_string()))?;
            let comparison_sha256 = comparison_hash(&row, &formulas)?;
            Ok(WorkbookBaselineRow {
                row,
                formulas,
                comparison_sha256,
            })
        })
        .collect()
}
fn keyed<'a>(
    rows: impl Iterator<Item = &'a QuantityRow>,
) -> HashMap<&'a str, Vec<&'a QuantityRow>> {
    let mut result: HashMap<&str, Vec<&QuantityRow>> = HashMap::new();
    for row in rows {
        if let Some(id) = row.register.annotation_id.as_deref() {
            result.entry(id).or_default().push(row);
        }
    }
    result
}

impl Document<'_> {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_quantity_workbook(
        &self,
        table: &QuantityTable,
        options: &QuantityWorkbookOptions,
        previous: Option<&QuantityWorkbookManifest>,
        include_new: bool,
        source: Option<&SavedReportSource>,
        cancel: &AtomicBool,
        mut progress: impl FnMut(WorkbookProgress),
    ) -> Result<PreparedQuantityWorkbook, EngineError> {
        cancelled(cancel)?;
        options.validate()?;
        if let Some(previous) = previous {
            previous.validate()?;
            if previous.options.quantities != options.quantities {
                return Err(invalid(
                    "refresh must preserve the manifest's quantity configuration",
                ));
            }
        } else if include_new {
            return Err(invalid("include_new requires a previous manifest"));
        }
        if table.report_snapshot().len() > crate::issue_report::MAX_SOURCE_BYTES {
            return Err(invalid("snapshot exceeds 128 MiB"));
        }
        if previous.is_none()
            && options
                .quantities
                .filter
                .page
                .is_some_and(|page| page > table.page_count())
        {
            return Err(invalid("source page filter is outside the document"));
        }
        if let Some(source) = source {
            source.verify_snapshot(self, table.report_snapshot(), cancel)?;
        }
        progress(WorkbookProgress {
            phase: WorkbookPhase::Collecting,
            completed: 0,
            total: table.rows().len(),
        });
        let by_id = keyed(table.rows().iter());
        let selected = table.select(&options.quantities);
        let mut effective = options.quantities.clone();
        let mut included: HashSet<(usize, usize)> = HashSet::new();
        let mut previous_ids = HashSet::new();
        if let Some(previous) = previous {
            let baseline_ids = keyed(
                previous
                    .rows
                    .iter()
                    .chain(&previous.pending_rows)
                    .map(|row| &row.row),
            );
            for before in previous.rows.iter().chain(&previous.pending_rows) {
                cancelled(cancel)?;
                if let Some(id) = before.row.register.annotation_id.as_deref() {
                    previous_ids.insert(id);
                    if baseline_ids.get(id).is_some_and(|items| items.len() == 1) {
                        if let Some(current) = by_id.get(id).filter(|items| items.len() == 1) {
                            included.insert((current[0].register.page, current[0].register.index));
                        }
                    }
                }
            }
            if include_new {
                for row in &selected {
                    if row
                        .register
                        .annotation_id
                        .as_deref()
                        .is_some_and(|id| !previous_ids.contains(id) && by_id[id].len() == 1)
                    {
                        included.insert((row.register.page, row.register.index));
                    }
                }
            }
            // Tracked identities survive page moves and changes to originally filtered fields.
            let custom_sort = effective.filter.custom_sort.clone();
            effective.filter = Default::default();
            effective.filter.custom_sort = custom_sort;
            effective.filter.include_replies = true;
        } else {
            included.extend(
                selected
                    .iter()
                    .map(|row| (row.register.page, row.register.index)),
            );
        }
        let restricted = table.for_workbook_selection(&included);
        // One evaluation supplies every cell and subtotal in both the preview and workbook.
        let quantities = self.export_quantity_table(&restricted, &effective)?.json;
        let rows = baseline_rows(&quantities)?;
        if rows.len() > MAX_WORKBOOK_ROWS {
            return Err(invalid("more than 10000 selected rows"));
        }
        let after_ids: HashMap<_, _> = rows
            .iter()
            .filter_map(|r| r.row.register.annotation_id.as_deref().map(|id| (id, r)))
            .collect();
        let mut changes = Vec::new();
        if let Some(previous) = previous {
            let baseline_ids = keyed(
                previous
                    .rows
                    .iter()
                    .chain(&previous.pending_rows)
                    .map(|row| &row.row),
            );
            for (index, before) in previous
                .rows
                .iter()
                .chain(&previous.pending_rows)
                .enumerate()
            {
                cancelled(cancel)?;
                progress(WorkbookProgress {
                    phase: WorkbookPhase::Comparing,
                    completed: index,
                    total: previous.rows.len() + previous.pending_rows.len(),
                });
                let id = before.row.register.annotation_id.as_deref();
                let after = id.and_then(|id| after_ids.get(id)).copied();
                let status = match id {
                    None => WorkbookChangeStatus::Untracked,
                    Some(id)
                        if baseline_ids[id].len() != 1
                            || by_id.get(id).is_some_and(|items| items.len() != 1) =>
                    {
                        WorkbookChangeStatus::Ambiguous
                    }
                    Some(id) if !by_id.contains_key(id) => WorkbookChangeStatus::Missing,
                    Some(_) if after.is_some_and(|row| row.row.quantity.is_none()) => {
                        WorkbookChangeStatus::Unquantified
                    }
                    Some(_)
                        if after.is_some_and(|row| {
                            row.comparison_sha256 == before.comparison_sha256
                        }) =>
                    {
                        WorkbookChangeStatus::Unchanged
                    }
                    Some(_) => WorkbookChangeStatus::Changed,
                };
                changes.push(WorkbookChange {
                    annotation_id: before.row.register.annotation_id.clone(),
                    status,
                    before: Some(before.clone()),
                    after: after.cloned(),
                });
            }
            for row in selected {
                let id = row.register.annotation_id.as_deref();
                if id.is_some_and(|id| previous_ids.contains(id)) {
                    continue;
                }
                let status = if id.is_none() {
                    WorkbookChangeStatus::Untracked
                } else if id.is_some_and(|id| by_id[id].len() != 1) {
                    WorkbookChangeStatus::Ambiguous
                } else if include_new {
                    WorkbookChangeStatus::Added
                } else {
                    WorkbookChangeStatus::ExcludedNew
                };
                changes.push(WorkbookChange {
                    annotation_id: row.register.annotation_id.clone(),
                    status,
                    before: None,
                    after: id.and_then(|id| after_ids.get(id)).copied().cloned(),
                });
            }
        } else {
            for row in &rows {
                let id = row.row.register.annotation_id.as_deref();
                let status = if id.is_none() {
                    WorkbookChangeStatus::Untracked
                } else if id.is_some_and(|id| by_id[id].len() != 1) {
                    WorkbookChangeStatus::Ambiguous
                } else if row.row.quantity.is_none() {
                    WorkbookChangeStatus::Unquantified
                } else {
                    WorkbookChangeStatus::Added
                };
                changes.push(WorkbookChange {
                    annotation_id: row.row.register.annotation_id.clone(),
                    status,
                    before: None,
                    after: Some(row.clone()),
                });
            }
        }
        let costs_complete = quantities["summary"]["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|group| group["costs"].as_array().into_iter().flatten())
            .all(|cost| cost["complete"] == true);
        let complete = costs_complete
            && quantities["complete"] == true
            && quantities["formula_evaluation_complete"] == true
            && quantities["zone_coverage"]["complete"] == true
            && changes.iter().all(|change| {
                matches!(
                    change.status,
                    WorkbookChangeStatus::Unchanged
                        | WorkbookChangeStatus::Changed
                        | WorkbookChangeStatus::Added
                        | WorkbookChangeStatus::ExcludedNew
                )
            });
        let mut warnings = Vec::new();
        if !complete {
            warnings.push("Partial quantities, formula errors or unresolved row identities remain explicit; missing values are not zero.".into());
        }
        if previous.is_some() {
            warnings.push("The supplied baseline is comparison evidence, not authenticated source data. All current calculations come from the current PDF.".into());
        }
        if changes
            .iter()
            .any(|change| change.status == WorkbookChangeStatus::ExcludedNew)
        {
            warnings.push(
                "New matching identities are excluded until include-new is explicitly selected."
                    .into(),
            );
        }
        let provenance = WorkbookProvenance {
            snapshot_sha256: digest(table.report_snapshot()),
            saved_file_sha256: source.map(|s| s.sha256().to_owned()),
            source_encrypted: self.properties().encrypted,
            source_signature_count: self.signature_count(),
        };
        let refreshable_rows = rows
            .iter()
            .filter(|row| {
                row.row
                    .register
                    .annotation_id
                    .as_deref()
                    .is_some_and(|id| by_id[id].len() == 1)
            })
            .count();
        let model = QuantityWorkbookModel {
            schema: SCHEMA,
            version: 1,
            title: options.title.clone(),
            provenance: provenance.clone(),
            quantities,
            changes,
            refresh: previous.is_some(),
            include_new,
            previous_snapshot_sha256: previous.map(|p| p.provenance.snapshot_sha256.clone()),
            complete,
            warnings,
            refreshable_rows,
        };
        let pending_rows = model
            .changes
            .iter()
            .filter(|change| {
                matches!(
                    change.status,
                    WorkbookChangeStatus::Missing
                        | WorkbookChangeStatus::Ambiguous
                        | WorkbookChangeStatus::Untracked
                )
            })
            .filter_map(|change| change.before.clone())
            .collect::<Vec<_>>();
        if rows.len().saturating_add(pending_rows.len()) > MAX_WORKBOOK_ROWS {
            return Err(invalid("more than 10000 current and unresolved rows"));
        }
        let manifest = QuantityWorkbookManifest {
            schema: SCHEMA.into(),
            version: 1,
            options: options.clone(),
            provenance,
            rows,
            pending_rows,
        };
        let manifest_bytes =
            serde_json::to_vec_pretty(&manifest).map_err(|e| invalid(&e.to_string()))?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("manifest exceeds 32 MiB"));
        }
        progress(WorkbookProgress {
            phase: WorkbookPhase::Packaging,
            completed: 0,
            total: 1,
        });
        let xlsx = crate::quantity_workbook_render::render(&model, cancel)?;
        let prepared = PreparedQuantityWorkbook {
            original: table.report_snapshot().to_vec(),
            source: source.cloned(),
            model,
            manifest,
            manifest_bytes,
            xlsx,
        };
        self.validate_quantity_workbook(&prepared, cancel)?;
        progress(WorkbookProgress {
            phase: WorkbookPhase::Ready,
            completed: 1,
            total: 1,
        });
        Ok(prepared)
    }
    fn validate_quantity_workbook(
        &self,
        prepared: &PreparedQuantityWorkbook,
        cancel: &AtomicBool,
    ) -> Result<(), EngineError> {
        cancelled(cancel)?;
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let current = self.snapshot()?;
        if current.len() > crate::issue_report::MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(&prepared.original, &current, self.password())?
        {
            return Err(invalid("document changed; prepare the workbook again"));
        }
        if let Some(source) = &prepared.source {
            source.revalidate(cancel)?;
        }
        cancelled(cancel)
    }
    pub fn publish_quantity_workbook(
        &self,
        prepared: &PreparedQuantityWorkbook,
        format: WorkbookFormat,
        output: &Path,
        cancel: &AtomicBool,
    ) -> Result<WorkbookReceipt, EngineError> {
        self.validate_quantity_workbook(prepared, cancel)?;
        if output.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(output.into()));
        }
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let canonical = parent.canonicalize()?;
        let name = output
            .file_name()
            .ok_or_else(|| invalid("output requires a filename"))?;
        let bytes = match format {
            WorkbookFormat::Xlsx => &prepared.xlsx,
            WorkbookFormat::Manifest => &prepared.manifest_bytes,
        };
        let mut temporary = tempfile::NamedTempFile::new_in(&canonical)?;
        for chunk in bytes.chunks(64 * 1024) {
            cancelled(cancel)?;
            temporary.write_all(chunk)?;
        }
        temporary.as_file().sync_all()?;
        self.validate_quantity_workbook(prepared, cancel)?;
        if parent.canonicalize()? != canonical {
            return Err(invalid("destination directory changed"));
        }
        cancelled(cancel)?;
        temporary
            .persist_noclobber(canonical.join(name))
            .map_err(|e| {
                if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                    EngineError::AlreadyExists(output.into())
                } else {
                    EngineError::Io(e.error)
                }
            })?;
        Ok(WorkbookReceipt {
            out: output.into(),
            format,
            output_sha256: digest(bytes),
            bytes: bytes.len(),
            matched_rows: prepared.manifest.rows.len(),
            complete: prepared.model.complete,
            provenance: prepared.model.provenance.clone(),
            source_unchanged: true,
            output_encrypted: false,
        })
    }
}
