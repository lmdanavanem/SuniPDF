use super::{cancelled, core, error, PlanLinkReceipt};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::atomic::AtomicBool,
};
use sunipdf_core::EngineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedCopy {
    pub output: PathBuf,
    pub bytes: usize,
    pub sha256: String,
}

impl From<&PlanLinkReceipt> for SavedCopy {
    fn from(receipt: &PlanLinkReceipt) -> Self {
        Self {
            output: receipt.output.clone(),
            bytes: receipt.bytes,
            sha256: receipt.report.output_sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "receipt", rename_all = "snake_case")]
pub enum ReceiptExport {
    Single(Box<PlanLinkReceipt>),
    Batch(Box<core::PlanLinkBatchOutcome>),
}

pub(super) fn verify_saved_copy(
    copy: &SavedCopy,
    cancel: &AtomicBool,
) -> Result<PathBuf, EngineError> {
    cancelled(cancel)?;
    if copy.bytes == 0
        || copy.bytes > core::MAX_CANDIDATE_BYTES
        || copy.sha256.len() != 64
        || !copy.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(error("saved-copy receipt is invalid"));
    }
    let path = std::fs::canonicalize(&copy.output)?;
    let mut file = sunipdf_core::file_io::open_regular(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() != copy.bytes as u64 {
        return Err(error(
            "saved copy changed; its size differs from the save receipt",
        ));
    }
    let mut hash = Sha256::new();
    let mut total = 0_usize;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        cancelled(cancel)?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if read > copy.bytes.saturating_sub(total) {
            return Err(error("saved copy changed during verification"));
        }
        total += read;
        hash.update(&buffer[..read]);
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if total != copy.bytes
        || !actual.eq_ignore_ascii_case(&copy.sha256)
        || std::fs::canonicalize(&copy.output)? != path
    {
        return Err(error(
            "saved copy changed; its content differs from the save receipt",
        ));
    }
    cancelled(cancel)?;
    Ok(path)
}

fn receipt_bytes(receipt: &ReceiptExport) -> Result<Vec<u8>, EngineError> {
    struct BoundedBytes(Vec<u8>);
    impl Write for BoundedBytes {
        fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
            if value.len() > core::MAX_BATCH_REPORT_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other(
                    "save receipt exceeds the JSON size limit",
                ));
            }
            self.0.extend_from_slice(value);
            Ok(value.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut bytes = BoundedBytes(Vec::new());
    serde_json::to_writer_pretty(&mut bytes, receipt)
        .map_err(|failure| error(&format!("cannot encode save receipt: {failure}")))?;
    bytes.write_all(b"\n")?;
    Ok(bytes.0)
}

pub(super) fn export_receipt(
    receipt: &ReceiptExport,
    output: &std::path::Path,
    cancel: &AtomicBool,
) -> Result<(), EngineError> {
    cancelled(cancel)?;
    let bytes = receipt_bytes(receipt)?;
    cancelled(cancel)?;
    sunipdf_core::write_new_copy(output, &bytes)
}

#[cfg(test)]
mod tests {
    use super::super::tests::inspection;
    use super::*;

    fn receipt(output: PathBuf, contents: &[u8]) -> PlanLinkReceipt {
        PlanLinkReceipt {
            output,
            bytes: contents.len(),
            report: core::PlanLinkReport {
                version: 1,
                inspection: inspection(),
                created: vec![],
                excluded_occurrences: vec![],
                output_sha256: Sha256::digest(contents)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
            },
        }
    }

    #[test]
    fn opening_a_saved_copy_refuses_changed_missing_and_cancelled_outputs() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("linked.pdf");
        let bytes = b"%PDF-1.7 saved content";
        std::fs::write(&path, bytes).unwrap();
        let copy = SavedCopy::from(&receipt(path.clone(), bytes));
        assert_eq!(
            verify_saved_copy(&copy, &AtomicBool::new(false)).unwrap(),
            path.canonicalize().unwrap()
        );
        assert!(matches!(
            verify_saved_copy(&copy, &AtomicBool::new(true)),
            Err(EngineError::Cancelled)
        ));
        let mut same_size = bytes.to_vec();
        same_size[9] ^= 1;
        std::fs::write(&path, same_size).unwrap();
        assert!(verify_saved_copy(&copy, &AtomicBool::new(false)).is_err());
        std::fs::write(&path, b"short").unwrap();
        assert!(verify_saved_copy(&copy, &AtomicBool::new(false)).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(verify_saved_copy(&copy, &AtomicBool::new(false)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_saved_copy_replaced_by_a_named_pipe_is_refused_without_a_writer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("linked.pdf");
        let copy = SavedCopy::from(&receipt(path.clone(), b"saved copy"));
        assert!(std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert!(verify_saved_copy(&copy, &AtomicBool::new(false)).is_err());
    }

    #[test]
    fn exported_partial_batch_retains_commits_failure_and_remaining_without_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("receipt.json");
        let outcome = core::PlanLinkBatchOutcome {
            version: 1,
            status: core::PlanLinkBatchStatus::Cancelled,
            completed: vec![receipt(directory.path().join("first.pdf"), b"first")],
            failure: Some(core::PlanLinkBatchFailure {
                source_file_id: Some("second".into()),
                reason: "Cancelled before second publication".into(),
            }),
            remaining: vec![core::PlanLinkBatchFilePlan {
                source_file_id: "second".into(),
                output: directory.path().join("second.pdf"),
                selected_occurrences: 2,
                preview_pages: vec![1, 3],
            }],
            last_input_validation: Some(core::PlanLinkBatchInputValidation {
                valid: true,
                reason: None,
            }),
        };
        let export = ReceiptExport::Batch(Box::new(outcome));
        export_receipt(&export, &output, &AtomicBool::new(false)).unwrap();
        let saved = std::fs::read(&output).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        assert_eq!(json["kind"], "batch");
        assert_eq!(json["receipt"]["completed"].as_array().unwrap().len(), 1);
        assert_eq!(json["receipt"]["failure"]["source_file_id"], "second");
        assert!(json["receipt"]["remaining"].is_array());
        assert_eq!(json["receipt"]["remaining"][0]["source_file_id"], "second");
        assert_eq!(
            json["receipt"]["remaining"][0]["preview_pages"],
            serde_json::json!([1, 3])
        );
        assert_eq!(json["receipt"]["last_input_validation"]["valid"], true);
        assert!(export_receipt(&export, &output, &AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::read(&output).unwrap(), saved);
        let cancelled_path = directory.path().join("cancelled.json");
        assert!(matches!(
            export_receipt(&export, &cancelled_path, &AtomicBool::new(true)),
            Err(EngineError::Cancelled)
        ));
        assert!(!cancelled_path.exists());
    }
}
