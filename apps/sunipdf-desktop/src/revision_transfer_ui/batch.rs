use super::*;
use sunipdf_core::revision_transfer::batch::TransferBatchStatus;

#[derive(Default)]
pub(super) struct BatchState {
    pub entries: Vec<TransferBatchEntrySummary>,
    pub outcome: Option<TransferBatchOutcome>,
    pub progress: Option<TransferBatchProgress>,
    pub confirm_discard: bool,
    next_id: u64,
}

#[derive(Default)]
pub(super) struct BatchActions {
    pub remove: Option<String>,
    pub clear: bool,
    pub save: bool,
    pub discard_and_close: bool,
}

impl TransferState {
    pub(super) fn can_queue(&self) -> bool {
        self.can_save() && self.batch.entries.len() < core::batch::MAX_BATCH_TRANSFERS
    }

    fn batch_close_pending(&self) -> bool {
        !self.batch.entries.is_empty()
            || self.pending.is_some_and(|(_, kind)| {
                matches!(
                    kind,
                    Pending::Queue | Pending::SaveBatch | Pending::DiscardAll
                )
            })
    }

    pub fn needs_close_guard(&self) -> bool {
        self.batch_close_pending()
    }
}

impl SuniApp {
    /// A queued review is unsaved work even when neither input PDF is dirty.
    pub(crate) fn guard_revision_transfer_quit(&mut self) -> bool {
        if !self.revision_transfer.batch_close_pending() {
            return false;
        }
        self.revision_transfer.open = true;
        if self.revision_transfer.busy() {
            self.revision_transfer.cancel.store(true, Ordering::Relaxed);
        } else {
            self.revision_transfer.batch.confirm_discard = true;
        }
        true
    }

    pub(crate) fn on_revision_transfer_progress(
        &mut self,
        generation: u64,
        progress: TransferBatchProgress,
    ) {
        if self.revision_transfer.pending == Some((generation, Pending::SaveBatch)) {
            self.revision_transfer.batch.progress = Some(progress);
        }
    }

    fn transfer_batch_has_dirty_source(&self) -> bool {
        self.revision_transfer.batch.entries.iter().any(|entry| {
            [&entry.old_path, &entry.new_path].into_iter().any(|path| {
                (self.dirty
                    && self
                        .document
                        .as_ref()
                        .is_some_and(|doc| same_path(&doc.info.path, path)))
                    || self.tabs.tabs.iter().any(|tab| {
                        (tab.dirty || tab.ui.as_ref().is_some_and(|ui| ui.dirty))
                            && same_path(&tab.path, path)
                    })
            })
        })
    }

    pub(super) fn queue_revision_transfer(&mut self, output: PathBuf) {
        if !self.revision_transfer.can_queue() {
            return;
        }
        if self.transfer_has_dirty_source() {
            self.revision_transfer.error = Some(tr(self.lang(), "transfer.dirty_source").into());
            return;
        }
        let token = self
            .revision_transfer
            .token
            .expect("reviewed current transfer");
        self.revision_transfer.batch.next_id = self.revision_transfer.batch.next_id.wrapping_add(1);
        let id = format!("transfer-{}", self.revision_transfer.batch.next_id);
        let reviewed = self.revision_transfer.reviewed.iter().copied().collect();
        let generation = self.revision_transfer.begin(Pending::Queue);
        self.worker.send(Request::RevisionTransfer {
            generation,
            job: Job::QueueCurrent {
                token,
                id,
                output,
                reviewed,
                cancel: self.revision_transfer.cancel.clone(),
            },
        });
    }

    pub(super) fn apply_transfer_batch_action(&mut self, actions: BatchActions) {
        if self.revision_transfer.busy() {
            return;
        }
        let (kind, job) = if actions.discard_and_close {
            if !self.revision_transfer.batch.confirm_discard {
                return;
            }
            (Pending::DiscardAll, Job::DiscardAll)
        } else if let Some(id) = actions.remove {
            if !self
                .revision_transfer
                .batch
                .entries
                .iter()
                .any(|entry| entry.id == id)
            {
                return;
            }
            (Pending::RemoveQueued, Job::RemoveQueued { id })
        } else if actions.clear {
            (Pending::ClearBatch, Job::ClearBatch)
        } else if actions.save && !self.revision_transfer.batch.entries.is_empty() {
            if self.transfer_batch_has_dirty_source() {
                self.revision_transfer.error =
                    Some(tr(self.lang(), "transfer.dirty_source").into());
                return;
            }
            let generation = self.revision_transfer.begin(Pending::SaveBatch);
            self.revision_transfer.batch.progress = None;
            self.worker.send(Request::RevisionTransfer {
                generation,
                job: Job::SaveBatch {
                    cancel: self.revision_transfer.cancel.clone(),
                },
            });
            return;
        } else {
            return;
        };
        let generation = self.revision_transfer.begin(kind);
        self.worker
            .send(Request::RevisionTransfer { generation, job });
    }
}

pub(super) fn render_queue(
    ui: &mut egui::Ui,
    lang: Language,
    state: &mut BatchState,
    busy: bool,
) -> BatchActions {
    let mut actions = BatchActions::default();
    if state.entries.is_empty() && state.outcome.is_none() && state.progress.is_none() {
        return actions;
    }
    ui.separator();
    ui.strong(tr(lang, "transfer.batch.title"));
    ui.small(tr(lang, "transfer.batch.limits"));
    if state.confirm_discard {
        ui.colored_label(
            crate::theme::ERROR,
            tr(lang, "transfer.batch.close_warning"),
        );
        ui.horizontal_wrapped(|ui| {
            actions.discard_and_close = ui
                .add_enabled(
                    !busy,
                    egui::Button::new(tr(lang, "transfer.batch.discard_close")),
                )
                .clicked();
            if ui
                .add_enabled(!busy, egui::Button::new(tr(lang, "transfer.batch.keep")))
                .clicked()
            {
                state.confirm_discard = false;
            }
        });
    }
    for entry in &state.entries {
        ui.push_id((&entry.id, "queued_transfer"), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!(
                    "{} → {}",
                    crate::app::file_name(&entry.old_path),
                    crate::app::file_name(&entry.new_path)
                ));
                if ui
                    .add_enabled(!busy, egui::Button::new(tr(lang, "transfer.batch.remove")))
                    .clicked()
                {
                    actions.remove = Some(entry.id.clone());
                }
            });
            ui.label(tr_args(
                lang,
                "transfer.batch.output",
                &[("path", &entry.output.display().to_string())],
            ));
            ui.small(tr_args(
                lang,
                "transfer.batch.item_summary",
                &[
                    ("pages", &entry.report.pages.len().to_string()),
                    ("count", &entry.report.transferred.len().to_string()),
                    ("excluded", &entry.report.exclusions.len().to_string()),
                ],
            ));
            ui.collapsing(tr(lang, "overlay.fingerprints"), |ui| {
                ui.label(entry.old_path.display().to_string());
                ui.monospace(&entry.report.old_sha256);
                ui.label(entry.new_path.display().to_string());
                ui.monospace(&entry.report.new_sha256);
            });
        });
    }
    if !state.entries.is_empty() {
        ui.horizontal_wrapped(|ui| {
            actions.save = ui
                .add_enabled(!busy, egui::Button::new(tr(lang, "transfer.batch.save")))
                .clicked();
            actions.clear = ui
                .add_enabled(!busy, egui::Button::new(tr(lang, "transfer.batch.clear")))
                .clicked();
        });
    }
    if let Some(progress) = &state.progress {
        ui.label(tr_args(
            lang,
            "transfer.batch.progress",
            &[
                ("done", &progress.completed.to_string()),
                ("total", &progress.total.to_string()),
                ("path", &progress.output.display().to_string()),
            ],
        ));
    }
    if let Some(outcome) = &state.outcome {
        ui.separator();
        ui.strong(tr(
            lang,
            match outcome.status {
                TransferBatchStatus::Complete => "transfer.batch.complete",
                TransferBatchStatus::Failed => "transfer.batch.failed",
                TransferBatchStatus::Cancelled => "transfer.batch.cancelled",
            },
        ));
        ui.small(tr(lang, "transfer.batch.results_help"));
        for saved in &outcome.completed {
            if let Some(output) = &saved.report.output_path {
                ui.label(tr_args(
                    lang,
                    "transfer.batch.saved_output",
                    &[("path", &output.display().to_string())],
                ));
            }
            if let Some(hash) = &saved.report.output_sha256 {
                ui.monospace(hash);
            }
        }
        if let Some(failure) = &outcome.failure {
            ui.colored_label(crate::theme::ERROR, &failure.reason);
        }
        for entry in &outcome.remaining {
            ui.label(tr_args(
                lang,
                "transfer.batch.not_saved",
                &[("path", &entry.output.display().to_string())],
            ));
        }
    }
    ui.separator();
    actions
}
