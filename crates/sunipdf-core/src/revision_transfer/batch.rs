//! A bounded queue of already reviewed revision transfers, saved as separate new copies.
//! All inputs and destinations are preflighted before publication. Each file has its own
//! no-clobber commit; completed files remain reported when a later item cannot be saved.

use super::{
    cancelled, invalid, publish_revision_transfer_checked, PreparedRevisionTransfer,
    RevisionTransferReport,
};
use crate::{Engine, EngineError, RenderedPage};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub const MAX_BATCH_TRANSFERS: usize = 16;
pub const MAX_BATCH_CANDIDATE_BYTES: usize = super::MAX_CANDIDATE_BYTES;
pub const MAX_BATCH_REPORT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchEntrySummary {
    pub id: String,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub output: PathBuf,
    pub candidate_bytes: usize,
    pub report: RevisionTransferReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferBatchStatus {
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchCompleted {
    pub id: String,
    pub report: RevisionTransferReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchFailure {
    pub id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchProgress {
    pub completed: usize,
    pub total: usize,
    pub id: String,
    pub output: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchOutcome {
    pub version: u32,
    pub status: TransferBatchStatus,
    pub completed: Vec<TransferBatchCompleted>,
    pub failure: Option<TransferBatchFailure>,
    /// Entries without a committed output, including an entry that failed during saving.
    pub remaining: Vec<TransferBatchEntrySummary>,
}

impl TransferBatchOutcome {
    pub fn complete(&self) -> bool {
        self.status == TransferBatchStatus::Complete
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"complete":self.complete(), "outcome":self})
    }
}

/// Queue refusal returns ownership of the current preview to the caller.
#[derive(Debug)]
pub struct TransferBatchPushError {
    pub error: EngineError,
    pub prepared: PreparedRevisionTransfer,
}

impl std::fmt::Display for TransferBatchPushError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.error, formatter)
    }
}

impl std::error::Error for TransferBatchPushError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[derive(Debug, Clone)]
struct Destination {
    selected_parent: PathBuf,
    parent: PathBuf,
    output: PathBuf,
}

impl Destination {
    fn new(output: &Path) -> Result<Self, EngineError> {
        if output
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(invalid("batch output cannot contain a parent traversal"));
        }
        let selected = std::path::absolute(output)?;
        if selected.to_str().is_none() {
            return Err(invalid("batch output requires a UTF-8 path"));
        }
        let name = selected
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|name| portable_pdf_name(name))
            .ok_or_else(|| invalid("choose a portable new PDF filename"))?;
        let selected_parent = selected
            .parent()
            .ok_or_else(|| invalid("batch output has no parent directory"))?
            .to_owned();
        let parent = selected_parent.canonicalize()?;
        regular_directory_chain(&parent)?;
        let destination = Self {
            output: parent.join(name),
            selected_parent,
            parent,
        };
        destination.revalidate()?;
        Ok(destination)
    }

    fn revalidate(&self) -> Result<(), EngineError> {
        regular_directory_chain(&self.parent)?;
        if self.selected_parent.canonicalize()? != self.parent
            || self.parent.canonicalize()? != self.parent
        {
            return Err(invalid(
                "batch output directory changed; review the queue again",
            ));
        }
        match self.output.symlink_metadata() {
            Ok(_) => Err(EngineError::AlreadyExists(self.output.clone())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

fn portable_pdf_name(name: &str) -> bool {
    if name.len() > 255
        || name.starts_with(' ')
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || !Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !stem.is_empty()
        && !matches!(
            stem.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "CONIN$"
                | "CONOUT$"
                | "COM¹"
                | "COM²"
                | "COM³"
                | "LPT¹"
                | "LPT²"
                | "LPT³"
        )
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn regular_directory_chain(path: &Path) -> Result<(), EngineError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(invalid("batch output cannot contain a parent traversal"));
        }
        #[cfg(windows)]
        if let Component::Prefix(prefix) = component {
            if !matches!(
                prefix.kind(),
                std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
            ) {
                return Err(invalid("batch outputs require local disk paths"));
            }
            current.push(component);
            continue;
        }
        current.push(component);
        let metadata = current.symlink_metadata()?;
        #[cfg(windows)]
        let alias = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let alias = metadata.file_type().is_symlink();
        if alias || !metadata.is_dir() {
            return Err(invalid(
                "batch output directories must exist without symbolic links or reparse points",
            ));
        }
    }
    Ok(())
}

fn path_key(path: &Path) -> String {
    // Portable batches refuse case-only aliases even on case-sensitive filesystems.
    path.to_string_lossy().to_lowercase()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

fn candidate_budget(current: usize, additional: usize) -> Result<usize, EngineError> {
    if additional == 0 || additional > MAX_BATCH_CANDIDATE_BYTES.saturating_sub(current) {
        return Err(invalid(
            "batch candidates exceed the combined 160 MiB limit",
        ));
    }
    Ok(current + additional)
}

#[derive(Debug)]
struct Entry {
    id: String,
    prepared: PreparedRevisionTransfer,
    destination: Destination,
    report_bytes: usize,
}

impl Entry {
    fn summary(&self) -> TransferBatchEntrySummary {
        TransferBatchEntrySummary {
            id: self.id.clone(),
            old_path: self.prepared.old_path.clone(),
            new_path: self.prepared.new_path.clone(),
            output: self.destination.output.clone(),
            candidate_bytes: self.prepared.bytes.len(),
            report: self.prepared.report.clone(),
        }
    }
}

/// Contains exact prepared PDF bytes, never passwords. Editing the next transfer does not
/// mutate queued candidates; removal or dropping the queue releases them without writing.
#[derive(Debug, Default)]
pub struct PreparedTransferBatch {
    entries: VecDeque<Entry>,
    candidate_bytes: usize,
    report_bytes: usize,
}

impl PreparedTransferBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn candidate_bytes(&self) -> usize {
        self.candidate_bytes
    }

    pub fn summaries(&self) -> Vec<TransferBatchEntrySummary> {
        self.entries.iter().map(Entry::summary).collect()
    }

    pub fn push(
        &mut self,
        id: String,
        prepared: PreparedRevisionTransfer,
        output: PathBuf,
    ) -> Result<(), Box<TransferBatchPushError>> {
        let validation = (|| {
            if self.len() >= MAX_BATCH_TRANSFERS {
                return Err(invalid("queue at most 16 transfers"));
            }
            if !valid_id(&id) || self.entries.iter().any(|entry| entry.id == id) {
                return Err(invalid("choose a distinct batch ID of 1 to 64 ASCII letters, digits, dots, underscores or hyphens, starting with a letter or digit"));
            }
            if prepared.old_path.to_str().is_none() || prepared.new_path.to_str().is_none() {
                return Err(invalid("batch sources require UTF-8 paths"));
            }
            let candidate_bytes = candidate_budget(self.candidate_bytes, prepared.bytes.len())?;
            let destination = Destination::new(&output)?;
            let old = path_key(&prepared.old_path);
            let new = path_key(&prepared.new_path);
            let out = path_key(&destination.output);
            if out == old || out == new {
                return Err(invalid("batch output cannot replace a source PDF"));
            }
            for existing in &self.entries {
                let existing_old = path_key(&existing.prepared.old_path);
                let existing_new = path_key(&existing.prepared.new_path);
                let existing_out = path_key(&existing.destination.output);
                if existing.id == id
                    || (existing_old == old && existing_new == new)
                    || existing_out == out
                    || existing_out == old
                    || existing_out == new
                    || out == existing_old
                    || out == existing_new
                {
                    return Err(invalid(
                        "batch contains duplicate revisions or colliding source/output paths",
                    ));
                }
            }
            let report_bytes = serde_json::to_vec(&TransferBatchEntrySummary {
                id: id.clone(),
                old_path: prepared.old_path.clone(),
                new_path: prepared.new_path.clone(),
                output: destination.output.clone(),
                candidate_bytes: prepared.bytes.len(),
                report: prepared.report.clone(),
            })
            .map_err(|error| invalid(error.to_string()))?
            .len();
            if report_bytes > MAX_BATCH_REPORT_BYTES.saturating_sub(self.report_bytes) {
                return Err(invalid("batch reports exceed the combined 8 MiB limit"));
            }
            Ok((destination, candidate_bytes, report_bytes))
        })();
        let (destination, candidate_bytes, report_bytes) = match validation {
            Ok(values) => values,
            Err(error) => return Err(Box::new(TransferBatchPushError { error, prepared })),
        };
        self.candidate_bytes = candidate_bytes;
        self.report_bytes += report_bytes;
        self.entries.push_back(Entry {
            id,
            prepared,
            destination,
            report_bytes,
        });
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Option<PreparedRevisionTransfer> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        let entry = self.entries.remove(index)?;
        self.candidate_bytes -= entry.prepared.bytes.len();
        self.report_bytes -= entry.report_bytes;
        Some(entry.prepared)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.candidate_bytes = 0;
        self.report_bytes = 0;
    }

    pub fn render_preview(
        &self,
        id: &str,
        engine: &Engine,
        page: usize,
        target_password: Option<&str>,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| invalid("batch transfer is no longer queued"))?
            .prepared
            .render_preview(engine, page, target_password, long_side, cancel)
    }

    pub fn preflight(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        self.preflight_checked(cancel).map_err(|(_, error)| error)
    }

    fn preflight_checked(&self, cancel: &AtomicBool) -> Result<(), (Option<String>, EngineError)> {
        if self.is_empty() {
            return Err((None, invalid("queue at least one reviewed transfer")));
        }
        cancelled(cancel).map_err(|error| (None, error))?;
        let mut outputs = HashSet::new();
        let sources: HashSet<_> = self
            .entries
            .iter()
            .flat_map(|entry| {
                [
                    path_key(&entry.prepared.old_path),
                    path_key(&entry.prepared.new_path),
                ]
            })
            .collect();
        for entry in &self.entries {
            let mut check = || {
                entry.prepared.revalidate_sources(cancel)?;
                entry.destination.revalidate()?;
                let output = path_key(&entry.destination.output);
                if sources.contains(&output) || !outputs.insert(output) {
                    return Err(invalid("batch source/output paths collide"));
                }
                Ok(())
            };
            check().map_err(|error| (Some(entry.id.clone()), error))?;
        }
        Ok(())
    }

    /// The callback runs before each item with the number of already committed outputs.
    /// Cancelling there prevents that next item. A successful final commit stays successful
    /// even if cancellation arrives immediately afterward.
    pub fn publish(
        mut self,
        cancel: &AtomicBool,
        mut progress: impl FnMut(TransferBatchProgress),
    ) -> TransferBatchOutcome {
        let mut outcome = TransferBatchOutcome {
            version: 1,
            status: TransferBatchStatus::Complete,
            completed: Vec::new(),
            failure: None,
            remaining: self.summaries(),
        };
        if let Err((id, error)) = self.preflight_checked(cancel) {
            outcome.fail(id, error);
            return outcome;
        }
        let total = self.len();
        while let Some(entry) = self.entries.pop_front() {
            progress(TransferBatchProgress {
                completed: outcome.completed.len(),
                total,
                id: entry.id.clone(),
                output: entry.destination.output.clone(),
            });
            let mut blocked_id = None;
            let result = publish_revision_transfer_checked(
                entry.prepared,
                &entry.destination.output,
                cancel,
                false,
                || {
                    entry.destination.revalidate()?;
                    if outcome.completed.is_empty() && !self.is_empty() {
                        if let Err((id, error)) = self.preflight_checked(cancel) {
                            blocked_id = id;
                            return Err(error);
                        }
                    }
                    Ok(())
                },
            );
            match result {
                Ok(report) => {
                    outcome.completed.push(TransferBatchCompleted {
                        id: entry.id,
                        report,
                    });
                    outcome.remaining.remove(0);
                }
                Err(error) => {
                    outcome.fail(blocked_id.or(Some(entry.id)), error);
                    return outcome;
                }
            }
        }
        outcome
    }
}

impl TransferBatchOutcome {
    fn fail(&mut self, id: Option<String>, error: EngineError) {
        self.status = if matches!(error, EngineError::Cancelled) {
            TransferBatchStatus::Cancelled
        } else {
            TransferBatchStatus::Failed
        };
        self.failure = Some(TransferBatchFailure {
            id,
            reason: error.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_review::digest;
    use std::sync::atomic::Ordering;

    // Private candidates isolate filesystem/queue orchestration from PDF parsing. The
    // revision-transfer integration suite separately prepares and reopens real PDFs.
    fn candidate(root: &Path, name: &str) -> PreparedRevisionTransfer {
        let old_path = root.join(format!("{name}-old.pdf"));
        let new_path = root.join(format!("{name}-new.pdf"));
        let old = format!("old revision {name}").into_bytes();
        let new = format!("new revision {name}").into_bytes();
        std::fs::write(&old_path, &old).unwrap();
        std::fs::write(&new_path, &new).unwrap();
        PreparedRevisionTransfer {
            old_selection_path: old_path.clone(),
            new_selection_path: new_path.clone(),
            old_path,
            new_path,
            bytes: format!("prepared revision {name}").into_bytes(),
            report: RevisionTransferReport {
                old_sha256: digest(&old),
                new_sha256: digest(&new),
                output_path: None,
                output_sha256: None,
                pages: Vec::new(),
                transferred: Vec::new(),
                exclusions: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }

    fn queue(root: &Path, names: &[&str]) -> PreparedTransferBatch {
        let mut batch = PreparedTransferBatch::new();
        for name in names {
            batch
                .push(
                    (*name).into(),
                    candidate(root, name),
                    root.join(format!("{name}-copy.pdf")),
                )
                .unwrap();
        }
        batch
    }

    #[test]
    fn queue_refusal_preserves_current_preview_and_remove_releases_its_budget() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let mut batch = queue(&root, &["first"]);
        let first_bytes = batch.candidate_bytes();
        let second = candidate(&root, "second");
        let expected = second.bytes.clone();
        let failure = batch
            .push("first".into(), second, root.join("second-copy.pdf"))
            .unwrap_err();
        assert_eq!(failure.prepared.bytes, expected);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.candidate_bytes(), first_bytes);
        batch
            .push(
                "second".into(),
                failure.prepared,
                root.join("second-copy.pdf"),
            )
            .unwrap();
        assert_eq!(batch.candidate_bytes(), first_bytes + expected.len());
        assert_eq!(batch.remove("second").unwrap().bytes, expected);
        assert!(batch.remove("missing").is_none());
        assert_eq!(batch.candidate_bytes(), first_bytes);
        batch.clear();
        assert!(batch.is_empty());
        assert_eq!(batch.candidate_bytes(), 0);
        assert_eq!(batch.report_bytes, 0);
        assert!(!root.join("first-copy.pdf").exists());
    }

    #[test]
    fn duplicate_pairs_case_only_outputs_sources_and_existing_files_are_refused() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let mut batch = queue(&root, &["first"]);
        assert!(batch
            .push(
                "duplicate-pair".into(),
                candidate(&root, "first"),
                root.join("other.pdf")
            )
            .is_err());
        assert!(batch
            .push(
                "duplicate-output".into(),
                candidate(&root, "second"),
                root.join("FIRST-copy.PDF")
            )
            .is_err());
        let third = candidate(&root, "third");
        assert!(batch
            .push("own-source".into(), third, root.join("third-old.pdf"))
            .is_err());
        assert!(batch
            .push(
                "other-source".into(),
                candidate(&root, "fourth"),
                root.join("first-new.pdf")
            )
            .is_err());
        let existing = root.join("existing.pdf");
        std::fs::write(&existing, b"user-owned output").unwrap();
        assert!(batch
            .push(
                "existing".into(),
                candidate(&root, "fifth"),
                existing.clone()
            )
            .is_err());
        assert_eq!(std::fs::read(existing).unwrap(), b"user-owned output");
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn batch_ids_output_names_count_and_combined_byte_limits_are_bounded() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let mut batch = PreparedTransferBatch::new();
        for id in ["", "../item", "_item", "item name", "item/one", "Ω"] {
            assert!(batch
                .push(id.into(), candidate(&root, "a"), root.join("a.pdf"))
                .is_err());
        }
        assert!(batch
            .push("x".repeat(65), candidate(&root, "a"), root.join("a.pdf"))
            .is_err());
        for output in [
            "one.txt",
            "one.pdf.",
            "one.pdf ",
            "AUX.pdf",
            "LPT9.pdf",
            "one:other.pdf",
            "one?.pdf",
        ] {
            assert!(batch
                .push("a".into(), candidate(&root, "a"), root.join(output))
                .is_err());
        }
        assert!(batch
            .push(
                "a".into(),
                candidate(&root, "a"),
                root.join("missing/new.pdf")
            )
            .is_err());
        assert!(batch
            .push("a".into(), candidate(&root, "a"), root.join("../new.pdf"))
            .is_err());
        assert!(!root.join("missing").exists());
        for index in 0..MAX_BATCH_TRANSFERS {
            let name = format!("pair-{index}");
            batch
                .push(
                    name.clone(),
                    candidate(&root, &name),
                    root.join(format!("{name}.pdf")),
                )
                .unwrap();
        }
        assert!(batch
            .push(
                "extra".into(),
                candidate(&root, "extra"),
                root.join("extra.pdf")
            )
            .is_err());
        assert_eq!(batch.len(), 16);
        assert_eq!(
            candidate_budget(MAX_BATCH_CANDIDATE_BYTES - 1, 1).unwrap(),
            MAX_BATCH_CANDIDATE_BYTES
        );
        assert!(candidate_budget(MAX_BATCH_CANDIDATE_BYTES - 1, 2).is_err());
        assert!(candidate_budget(usize::MAX, 1).is_err());
        assert!(candidate_budget(0, 0).is_err());
    }

    #[test]
    fn stale_later_source_blocks_every_output_before_saving_starts() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let batch = queue(&root, &["first", "second"]);
        std::fs::write(root.join("second-old.pdf"), b"changed later source").unwrap();
        let outcome = batch.publish(&AtomicBool::new(false), |_| {
            panic!("preflight must run first")
        });
        assert_eq!(outcome.status, TransferBatchStatus::Failed);
        assert_eq!(
            outcome.failure.as_ref().unwrap().id.as_deref(),
            Some("second")
        );
        assert!(outcome.completed.is_empty());
        assert_eq!(outcome.remaining.len(), 2);
        assert!(!root.join("first-copy.pdf").exists());
        assert!(!root.join("second-copy.pdf").exists());
    }

    #[test]
    fn combined_candidate_budget_refuses_a_second_candidate_without_losing_the_first() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let mut batch = PreparedTransferBatch::new();
        let mut first = candidate(&root, "first");
        first.bytes.resize(MAX_BATCH_CANDIDATE_BYTES / 2, 42);
        batch
            .push("first".into(), first, root.join("first-copy.pdf"))
            .unwrap();
        let mut second = candidate(&root, "second");
        second.bytes.resize(MAX_BATCH_CANDIDATE_BYTES / 2 + 1, 43);
        let error = batch
            .push("second".into(), second, root.join("second-copy.pdf"))
            .unwrap_err();
        assert_eq!(
            error.prepared.bytes.len(),
            MAX_BATCH_CANDIDATE_BYTES / 2 + 1
        );
        assert_eq!(batch.candidate_bytes(), MAX_BATCH_CANDIDATE_BYTES / 2);
        assert_eq!(batch.len(), 1);
        assert!(!root.join("first-copy.pdf").exists());
        assert!(!root.join("second-copy.pdf").exists());
    }

    #[test]
    fn first_commit_rechecks_later_sources_changed_after_the_initial_preflight() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let batch = queue(&root, &["first", "second"]);
        let outcome = batch.publish(&AtomicBool::new(false), |progress| {
            assert_eq!(progress.completed, 0);
            std::fs::write(root.join("second-new.pdf"), b"changed during first staging").unwrap();
        });
        assert_eq!(outcome.status, TransferBatchStatus::Failed);
        assert_eq!(
            outcome.failure.as_ref().unwrap().id.as_deref(),
            Some("second")
        );
        assert!(outcome.completed.is_empty());
        assert_eq!(outcome.remaining.len(), 2);
        assert!(!root.join("first-copy.pdf").exists());
        assert!(!root.join("second-copy.pdf").exists());
    }

    #[test]
    fn two_outputs_retain_individual_receipts_and_source_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let batch = queue(&root, &["first", "second"]);
        let summaries = batch.summaries();
        let mut progress_values = Vec::new();
        let outcome = batch.publish(&AtomicBool::new(false), |p| progress_values.push(p));
        assert!(outcome.complete());
        assert_eq!(outcome.completed.len(), 2);
        assert!(outcome.remaining.is_empty());
        assert!(outcome.failure.is_none());
        assert_eq!(
            progress_values
                .iter()
                .map(|p| p.completed)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        for (receipt, summary) in outcome.completed.iter().zip(&summaries) {
            let output = receipt.report.output_path.as_ref().unwrap();
            let bytes = std::fs::read(output).unwrap();
            assert_eq!(receipt.id, summary.id);
            let expected_hash = digest(&bytes);
            assert_eq!(
                receipt.report.output_sha256.as_deref(),
                Some(expected_hash.as_str())
            );
            assert_eq!(bytes.len(), summary.candidate_bytes);
            assert_eq!(
                digest(&std::fs::read(&summary.old_path).unwrap()),
                summary.report.old_sha256
            );
            assert_eq!(
                digest(&std::fs::read(&summary.new_path).unwrap()),
                summary.report.new_sha256
            );
        }
        assert_eq!(outcome.json()["complete"], true);
        assert_eq!(
            outcome.json()["outcome"]["completed"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn cancellation_keeps_committed_files_and_explicit_uncommitted_entries() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let cancel = AtomicBool::new(true);
        let before = queue(&root, &["before"]).publish(&cancel, |_| panic!("already cancelled"));
        assert_eq!(before.status, TransferBatchStatus::Cancelled);
        assert!(before.completed.is_empty());
        assert_eq!(before.remaining.len(), 1);
        cancel.store(false, Ordering::Relaxed);
        let batch = queue(&root, &["first", "second"]);
        let outcome = batch.publish(&cancel, |progress| {
            if progress.completed == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert_eq!(outcome.status, TransferBatchStatus::Cancelled);
        assert_eq!(outcome.completed.len(), 1);
        assert_eq!(outcome.remaining.len(), 1);
        assert_eq!(outcome.remaining[0].id, "second");
        assert!(root.join("first-copy.pdf").is_file());
        assert!(!root.join("second-copy.pdf").exists());
    }

    #[test]
    fn a_late_collision_does_not_erase_previous_success_or_the_colliding_file() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let batch = queue(&root, &["first", "second"]);
        let outcome = batch.publish(&AtomicBool::new(false), |progress| {
            if progress.completed == 1 {
                std::fs::write(&progress.output, b"new user file").unwrap();
            }
        });
        assert_eq!(outcome.status, TransferBatchStatus::Failed);
        assert_eq!(outcome.completed.len(), 1);
        assert_eq!(outcome.remaining.len(), 1);
        assert_eq!(
            outcome.failure.as_ref().unwrap().id.as_deref(),
            Some("second")
        );
        assert!(root.join("first-copy.pdf").is_file());
        assert_eq!(
            std::fs::read(root.join("second-copy.pdf")).unwrap(),
            b"new user file"
        );
    }

    #[test]
    fn empty_batches_are_refused_without_claiming_completion() {
        let empty = PreparedTransferBatch::new();
        assert!(empty.preflight(&AtomicBool::new(false)).is_err());
        let outcome = empty.publish(&AtomicBool::new(false), |_| panic!("empty batch"));
        assert!(!outcome.complete());
        assert!(outcome.completed.is_empty());
        assert!(outcome.failure.is_some());
    }

    #[test]
    fn a_parent_removed_after_preflight_is_not_recreated_by_saving() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let directory = root.join("outputs");
        std::fs::create_dir(&directory).unwrap();
        let mut batch = PreparedTransferBatch::new();
        batch
            .push(
                "first".into(),
                candidate(&root, "first"),
                directory.join("copy.pdf"),
            )
            .unwrap();
        let outcome = batch.publish(&AtomicBool::new(false), |_| {
            std::fs::remove_dir(&directory).unwrap();
        });
        assert_eq!(outcome.status, TransferBatchStatus::Failed);
        assert!(outcome.completed.is_empty());
        assert!(!directory.exists());
    }

    #[cfg(unix)]
    #[test]
    fn stable_output_directory_aliases_work_but_retargeting_is_refused() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let directory = root.join("outputs");
        let other = root.join("other");
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(&other).unwrap();
        let alias = root.join("alias");
        symlink(&directory, &alias).unwrap();
        let mut stable = PreparedTransferBatch::new();
        stable
            .push(
                "stable".into(),
                candidate(&root, "stable"),
                alias.join("stable.pdf"),
            )
            .unwrap();
        assert!(stable.publish(&AtomicBool::new(false), |_| {}).complete());
        assert!(directory.join("stable.pdf").exists());
        let mut batch = PreparedTransferBatch::new();
        batch
            .push(
                "first".into(),
                candidate(&root, "first"),
                alias.join("copy.pdf"),
            )
            .unwrap();
        batch.preflight(&AtomicBool::new(false)).unwrap();
        std::fs::remove_file(&alias).unwrap();
        symlink(&other, &alias).unwrap();
        let outcome = batch.publish(&AtomicBool::new(false), |_| panic!("redirected parent"));
        assert_eq!(outcome.status, TransferBatchStatus::Failed);
        assert!(outcome.completed.is_empty());
        assert!(!other.join("copy.pdf").exists());
        assert!(!directory.join("copy.pdf").exists());
    }
}
