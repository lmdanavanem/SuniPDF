//! Sequential, source-bound publication of reviewed links across several PDFs.
use super::*;

pub const MAX_BATCH_SOURCES: usize = 16;
pub const MAX_BATCH_SCAN_PAGES: usize = 128;
pub const MAX_BATCH_PAGES: usize = MAX_BATCH_SCAN_PAGES;
pub const MAX_BATCH_OCCURRENCES: usize = 4096;
pub const MAX_BATCH_RECIPE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BATCH_REPORT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkBatchScanRequest {
    pub sources: Vec<PlanLinkScanRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkBatchFileReview {
    pub review: PlanLinkReview,
    /// A new portable PDF basename in this source's existing directory.
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkBatchReview {
    pub version: u32,
    pub manifest_sha256: String,
    pub files: Vec<PlanLinkBatchFileReview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchInspection {
    pub version: u32,
    pub manifest_sha256: String,
    pub files: Vec<PlanLinkInspection>,
    pub scanned_pages: usize,
    pub occurrences: usize,
    pub scan_complete: bool,
    pub stop_reason: Option<String>,
    pub remaining_sources: Vec<String>,
}
impl PlanLinkBatchInspection {
    pub fn complete(&self) -> bool {
        self.scan_complete && self.stop_reason.is_none() && self.files.iter().all(|f| f.complete())
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"complete":self.complete(), "inspection":self})
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchFilePlan {
    pub source_file_id: String,
    pub output: PathBuf,
    pub selected_occurrences: usize,
    pub preview_pages: Vec<usize>,
}

/// This plan retains reviews and identities, never prepared PDF candidates or passwords.
pub struct PlanLinkBatchPlan {
    sources: Sources,
    review: PlanLinkBatchReview,
    files: Vec<PlanLinkBatchFilePlan>,
    occurrence_budgets: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanLinkBatchStatus {
    Complete,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchFailure {
    pub source_file_id: Option<String>,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchInputValidation {
    pub valid: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchProgress {
    pub completed: usize,
    pub total: usize,
    pub source_file_id: String,
    pub output: PathBuf,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkBatchOutcome {
    pub version: u32,
    pub status: PlanLinkBatchStatus,
    /// Committed files are retained even when a later file fails or is cancelled.
    pub completed: Vec<PlanLinkReceipt>,
    pub failure: Option<PlanLinkBatchFailure>,
    pub remaining: Vec<PlanLinkBatchFilePlan>,
    pub last_input_validation: Option<PlanLinkBatchInputValidation>,
}
impl PlanLinkBatchOutcome {
    pub fn complete(&self) -> bool {
        self.status == PlanLinkBatchStatus::Complete
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"complete":self.complete(), "outcome":self})
    }
}

fn json_size(value: &impl Serialize, limit: usize) -> Result<usize, EngineError> {
    struct Counter {
        size: usize,
        limit: usize,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.size) {
                return Err(std::io::Error::other("batch JSON size limit"));
            }
            self.size += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { size: 0, limit };
    serde_json::to_writer(&mut counter, value).map_err(|e| failure(e.to_string()))?;
    Ok(counter.size)
}

fn portable_name(name: &str) -> bool {
    !name.contains(['/', '\\']) && safe_relative(name)
}

fn validate_batch_scans(scans: &[PlanLinkScanRequest]) -> Result<(), EngineError> {
    if scans.is_empty() || scans.len() > MAX_BATCH_SOURCES {
        return Err(failure("select 1 to 16 distinct batch sources"));
    }
    let mut sources = HashSet::new();
    let mut pages = 0;
    for scan in scans {
        let mut seen_pages = HashSet::new();
        let mut seen_targets = HashSet::new();
        if scan.source_file_id.is_empty()
            || scan.source_file_id.len() > 128
            || !sources.insert(&scan.source_file_id)
            || scan.pages.is_empty()
            || scan.pages.len() > MAX_SCAN_PAGES
            || scan.pages.iter().any(|p| *p == 0 || !seen_pages.insert(p))
            || scan.target_sheet_ids.is_empty()
            || scan.target_sheet_ids.len() > plan_set::MAX_SHEETS
            || scan
                .target_sheet_ids
                .iter()
                .any(|id| id.is_empty() || id.len() > 128 || !seen_targets.insert(id))
        {
            return Err(failure(
                "invalid, duplicate or oversized batch source selection",
            ));
        }
        pages += scan.pages.len();
    }
    if pages > MAX_BATCH_SCAN_PAGES {
        return Err(failure("batch exceeds 128 requested pages"));
    }
    Ok(())
}

pub fn parse_batch_scan_request(bytes: &[u8]) -> Result<PlanLinkBatchScanRequest, EngineError> {
    if bytes.len() > MAX_BATCH_RECIPE_BYTES {
        return Err(failure("batch request exceeds 4 MiB"));
    }
    let request: PlanLinkBatchScanRequest = serde_json::from_slice(bytes)
        .map_err(|e| failure(format!("invalid batch request: {e}")))?;
    validate_batch_scans(&request.sources)?;
    Ok(request)
}

fn validate_batch_review(review: &PlanLinkBatchReview) -> Result<(), EngineError> {
    if review.version != 1
        || !valid_hash(&review.manifest_sha256)
        || review.files.is_empty()
        || review.files.len() > MAX_BATCH_SOURCES
    {
        return Err(failure("invalid batch review version or manifest hash"));
    }
    let scans: Vec<_> = review.files.iter().map(|f| f.review.scan.clone()).collect();
    validate_batch_scans(&scans)?;
    let mut selections = 0;
    for file in &review.files {
        validate_review(&file.review)?;
        encode_review(&file.review)?;
        if file.review.manifest_sha256 != review.manifest_sha256
            || !portable_name(&file.output_name)
        {
            return Err(failure("mixed manifests or nonportable batch output name"));
        }
        selections += file.review.selections.len();
    }
    if selections > MAX_BATCH_OCCURRENCES {
        return Err(failure("batch exceeds 4096 selected occurrences"));
    }
    json_size(review, MAX_BATCH_RECIPE_BYTES)?;
    Ok(())
}
pub fn parse_batch_review(bytes: &[u8]) -> Result<PlanLinkBatchReview, EngineError> {
    if bytes.len() > MAX_BATCH_RECIPE_BYTES {
        return Err(failure("batch review exceeds 4 MiB"));
    }
    let review =
        serde_json::from_slice(bytes).map_err(|e| failure(format!("invalid batch review: {e}")))?;
    validate_batch_review(&review)?;
    Ok(review)
}
pub fn encode_batch_review(review: &PlanLinkBatchReview) -> Result<Vec<u8>, EngineError> {
    validate_batch_review(review)?;
    let bytes = serde_json::to_vec_pretty(review).map_err(|e| failure(e.to_string()))?;
    if bytes.len() > MAX_BATCH_RECIPE_BYTES {
        return Err(failure("batch review exceeds 4 MiB"));
    }
    Ok(bytes)
}

pub fn inspect_plan_link_batch(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    request: &PlanLinkBatchScanRequest,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<PlanLinkBatchInspection, EngineError> {
    validate_batch_scans(&request.sources)?;
    json_size(request, MAX_BATCH_RECIPE_BYTES)?;
    for scan in &request.sources {
        validate_scan(scan, set)?;
    }
    let mut sources = Sources::load(manifest, set)?;
    let mut result = PlanLinkBatchInspection {
        version: 1,
        manifest_sha256: sources.manifest_sha256.clone(),
        files: Vec::new(),
        scanned_pages: 0,
        occurrences: 0,
        scan_complete: false,
        stop_reason: None,
        remaining_sources: request
            .sources
            .iter()
            .map(|s| s.source_file_id.clone())
            .collect(),
    };
    // Leave room for bounded failure text, counters and the JSON envelope.
    let mut retained_bytes = json_size(&result, MAX_BATCH_REPORT_BYTES)? + 16 * 1024;
    for scan in &request.sources {
        if result.occurrences == MAX_BATCH_OCCURRENCES {
            result.stop_reason = Some("batch_occurrence_limit".into());
            break;
        }
        let scanned = scan_internal_bounded(
            engine,
            manifest,
            set,
            scan,
            credentials,
            cancel,
            MAX_BATCH_OCCURRENCES - result.occurrences,
        );
        let (inspection, file_sources, _) = match scanned {
            Ok(value) => value,
            Err(error) => {
                result.stop_reason = Some(bounded_reason(&error));
                break;
            }
        };
        if inspection.manifest_sha256 != result.manifest_sha256 {
            result.stop_reason = Some("manifest changed during batch inspection".into());
            break;
        }
        let bytes = json_size(&inspection, MAX_REPORT_BYTES)?;
        if bytes + 1 > MAX_BATCH_REPORT_BYTES.saturating_sub(retained_bytes) {
            result.stop_reason = Some("batch_report_limit".into());
            break;
        }
        retained_bytes += bytes + 1;
        result.scanned_pages += inspection.scanned_pages;
        result.occurrences += inspection.occurrences.len();
        result.files.push(inspection);
        result.remaining_sources.remove(0);
        for binding in file_sources.bindings {
            if !sources.bindings.iter().any(|b| b.path == binding.path) {
                sources.bindings.push(binding);
            }
        }
    }
    if let Err(error) = sources.revalidate(cancel) {
        result.stop_reason = Some(bounded_reason(&error));
    }
    result.scan_complete = result.stop_reason.is_none()
        && result.remaining_sources.is_empty()
        && result.files.iter().all(|f| f.scan_complete && !f.truncated);
    json_size(&result, MAX_BATCH_REPORT_BYTES)?;
    Ok(result)
}

fn output_for(sources: &Sources, source_file_id: &str, name: &str) -> Result<PathBuf, EngineError> {
    if !portable_name(name) {
        return Err(failure("choose a portable new PDF basename"));
    }
    let source = sources.file_path(source_file_id)?;
    let parent = source
        .parent()
        .ok_or_else(|| failure("source has no parent"))?;
    no_aliases(parent)?;
    if parent.canonicalize()? != parent {
        return Err(failure("source directory identity changed"));
    }
    let output = parent.join(name);
    // Keep portability even on a case-sensitive host, including races checked again before commit.
    if std::fs::read_dir(parent)?.any(|entry| match entry {
        Ok(entry) => entry.file_name().to_string_lossy().to_lowercase() == name.to_lowercase(),
        Err(_) => true,
    }) || output.symlink_metadata().is_ok()
    {
        return Err(failure(
            "batch output already exists or has a case-equivalent name",
        ));
    }
    Ok(output)
}

pub fn prepare_plan_link_batch(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    review: &PlanLinkBatchReview,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<PlanLinkBatchPlan, EngineError> {
    validate_batch_review(review)?;
    cancelled(cancel)?;
    let mut sources = Sources::load(manifest, set)?;
    if sources.manifest_sha256 != review.manifest_sha256 {
        return Err(failure("batch manifest changed; inspect again"));
    }
    for file in &review.files {
        cancelled(cancel)?;
        validate_scan(&file.review.scan, set)?;
        let source_sheet = set
            .sheets
            .iter()
            .find(|s| s.file_id == file.review.scan.source_file_id)
            .ok_or_else(|| failure("source has no indexed sheet"))?;
        let source = sources.read(&source_sheet.id)?;
        if source.sha256 != file.review.source_sha256 {
            return Err(failure("batch source changed; inspect again"));
        }
        drop(source);
        for id in &file.review.scan.target_sheet_ids {
            cancelled(cancel)?;
            sources.read(id)?;
        }
    }
    let mut files = Vec::new();
    let mut output_keys = HashSet::new();
    for file in &review.files {
        let output = output_for(
            &sources,
            &file.review.scan.source_file_id,
            &file.output_name,
        )?;
        if !output_keys.insert(output.to_string_lossy().to_lowercase()) {
            return Err(failure("duplicate or case-equivalent batch outputs"));
        }
        files.push(PlanLinkBatchFilePlan {
            source_file_id: file.review.scan.source_file_id.clone(),
            output,
            selected_occurrences: file.review.selections.len(),
            preview_pages: Vec::new(),
        });
    }
    let mut report_bytes = 16 * 1024;
    let mut occurrence_budget = MAX_BATCH_OCCURRENCES;
    let mut occurrence_budgets = Vec::new();
    // Every file is dry-prepared before the first output. Each candidate is dropped here.
    for (file, info) in review.files.iter().zip(&mut files) {
        sources.revalidate(cancel)?;
        if occurrence_budget == 0 {
            return Err(failure(
                "batch occurrence budget exhausted before this source",
            ));
        }
        occurrence_budgets.push(occurrence_budget);
        let prepared = prepare_plan_links_bounded(
            engine,
            manifest,
            set,
            &file.review,
            credentials,
            cancel,
            occurrence_budget,
        )?;
        occurrence_budget -= prepared.report.inspection.occurrences.len();
        report_bytes += json_size(prepared.report(), MAX_REPORT_BYTES)? + 1;
        if report_bytes > MAX_BATCH_REPORT_BYTES {
            return Err(failure("batch reports exceed 8 MiB"));
        }
        info.preview_pages = prepared.report.created.iter().map(|c| c.page).collect();
        info.preview_pages.sort_unstable();
        info.preview_pages.dedup();
    }
    sources.revalidate(cancel)?;
    Ok(PlanLinkBatchPlan {
        sources,
        review: review.clone(),
        files,
        occurrence_budgets,
    })
}

impl PlanLinkBatchPlan {
    pub fn files(&self) -> &[PlanLinkBatchFilePlan] {
        &self.files
    }
    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        self.sources.revalidate(cancel)
    }
    fn candidate(
        &self,
        index: usize,
        engine: &Engine,
        credentials: &PlanLinkCredentials<'_>,
        cancel: &AtomicBool,
    ) -> Result<PreparedPlanLinks, EngineError> {
        self.revalidate_sources(cancel)?;
        let file = self
            .review
            .files
            .get(index)
            .ok_or_else(|| failure("batch preview file index is invalid"))?;
        prepare_plan_links_bounded(
            engine,
            &self.sources.selected_manifest,
            &self.sources.set,
            &file.review,
            credentials,
            cancel,
            self.occurrence_budgets[index],
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn preview_file(
        &self,
        index: usize,
        engine: &Engine,
        credentials: &PlanLinkCredentials<'_>,
        page: usize,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        let candidate = self.candidate(index, engine, credentials, cancel)?;
        let image = candidate.render_preview(
            engine,
            page,
            credentials.get(&self.files[index].source_file_id),
            long_side,
            cancel,
        )?;
        self.revalidate_sources(cancel)?;
        Ok(image)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn preview_target(
        &self,
        index: usize,
        engine: &Engine,
        credentials: &PlanLinkCredentials<'_>,
        sheet_id: &str,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        let candidate = self.candidate(index, engine, credentials, cancel)?;
        let sheet = self
            .sources
            .set
            .sheets
            .iter()
            .find(|s| s.id == sheet_id)
            .ok_or_else(|| failure("batch preview target is unknown"))?;
        let image = candidate.render_target_preview(
            engine,
            sheet_id,
            credentials.get(&sheet.file_id),
            long_side,
            cancel,
        )?;
        self.revalidate_sources(cancel)?;
        Ok(image)
    }
}

fn bounded_reason(error: &EngineError) -> String {
    let reason = error.to_string();
    let mut end = reason.len().min(1024);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    reason[..end].to_owned()
}

fn record_validation(
    sources: &Sources,
    cancel: &AtomicBool,
    outcome: &mut PlanLinkBatchOutcome,
) -> Result<(), EngineError> {
    // Cancellation is not a failed identity check; preserve the last actual check.
    cancelled(cancel)?;
    match sources.revalidate(cancel) {
        Ok(()) => {
            outcome.last_input_validation = Some(PlanLinkBatchInputValidation {
                valid: true,
                reason: None,
            });
            Ok(())
        }
        Err(error) => {
            if !matches!(error, EngineError::Cancelled) {
                outcome.last_input_validation = Some(PlanLinkBatchInputValidation {
                    valid: false,
                    reason: Some(bounded_reason(&error)),
                });
            }
            Err(error)
        }
    }
}
fn record_failure(outcome: &mut PlanLinkBatchOutcome, file: Option<String>, error: EngineError) {
    outcome.status = if matches!(error, EngineError::Cancelled) {
        PlanLinkBatchStatus::Cancelled
    } else {
        PlanLinkBatchStatus::Failed
    };
    outcome.failure = Some(PlanLinkBatchFailure {
        source_file_id: file,
        reason: bounded_reason(&error),
    });
}

/// Each output commits separately. A later error never rolls back completed files.
pub fn publish_plan_link_batch(
    plan: PlanLinkBatchPlan,
    engine: &Engine,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&PlanLinkBatchProgress),
) -> PlanLinkBatchOutcome {
    let mut outcome = PlanLinkBatchOutcome {
        version: 1,
        status: PlanLinkBatchStatus::Complete,
        completed: Vec::new(),
        failure: None,
        remaining: plan.files.clone(),
        last_input_validation: None,
    };
    let mut retained_bytes =
        16 * 1024 + json_size(&outcome, MAX_BATCH_REPORT_BYTES).unwrap_or(MAX_BATCH_REPORT_BYTES);
    for (index, file) in plan.files.iter().enumerate() {
        let operation = (|| -> Result<PlanLinkReceipt, EngineError> {
            record_validation(&plan.sources, cancel, &mut outcome)?;
            output_for(
                &plan.sources,
                &file.source_file_id,
                &plan.review.files[index].output_name,
            )?;
            let prepared = plan.candidate(index, engine, credentials, cancel)?;
            // Reserve the complete receipt before committing; no receipt is dropped afterward.
            let report_bytes = json_size(prepared.report(), MAX_REPORT_BYTES)?;
            let receipt_overhead = json_size(
                &serde_json::json!({"output":file.output,"bytes":prepared.bytes.len()}),
                16 * 1024,
            )? + 16;
            if report_bytes + receipt_overhead
                > MAX_BATCH_REPORT_BYTES.saturating_sub(retained_bytes)
            {
                return Err(failure("batch receipts exceed 8 MiB"));
            }
            let receipt = publish_plan_links_checked(prepared, &file.output, cancel, || {
                record_validation(&plan.sources, cancel, &mut outcome)?;
                output_for(
                    &plan.sources,
                    &file.source_file_id,
                    &plan.review.files[index].output_name,
                )?;
                Ok(())
            })?;
            retained_bytes += report_bytes + receipt_overhead;
            Ok(receipt)
        })();
        match operation {
            Ok(receipt) => {
                outcome.completed.push(receipt);
                outcome.remaining.remove(0);
                // Commit receipt is retained before a callback or the next cancellation check.
                on_progress(&PlanLinkBatchProgress {
                    completed: outcome.completed.len(),
                    total: plan.files.len(),
                    source_file_id: file.source_file_id.clone(),
                    output: file.output.clone(),
                });
            }
            Err(error) => {
                record_failure(&mut outcome, Some(file.source_file_id.clone()), error);
                return outcome;
            }
        }
    }
    if let Err(error) = record_validation(&plan.sources, cancel, &mut outcome) {
        record_failure(&mut outcome, None, error);
    }
    outcome
}
