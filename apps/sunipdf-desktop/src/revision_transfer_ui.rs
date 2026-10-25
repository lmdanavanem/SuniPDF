//! Saved-revision transfers. Prepared PDF bytes never leave the engine worker.

use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    revision_transfer::{
        self as core, RevisionTransferRequest, TransferAlignment, TransferInspection,
        TransferPagePair, TransferSelection,
    },
    Engine, EngineError, RenderedPage,
};
use sunipdf_i18n::{tr, tr_args, Language};
use zeroize::Zeroizing;

mod batch;
use batch::BatchState;
use sunipdf_core::revision_transfer::batch::{
    PreparedTransferBatch, TransferBatchEntrySummary, TransferBatchOutcome, TransferBatchProgress,
};

type Passwords = [Zeroizing<String>; 2];

#[derive(Clone)]
pub enum Sources {
    Files([PathBuf; 2]),
    Plan {
        set: sunipdf_core::plan_set::PlanSet,
        manifest: PathBuf,
        files: [String; 2],
    },
}

impl Sources {
    fn resolve(&self) -> Result<[PathBuf; 2], EngineError> {
        match self {
            Self::Files(paths) => Ok(paths.clone()),
            Self::Plan {
                set,
                manifest,
                files,
            } => {
                let mut paths = [PathBuf::new(), PathBuf::new()];
                for index in 0..2 {
                    let sheet = set
                        .sheets
                        .iter()
                        .find(|s| s.file_id == files[index])
                        .ok_or_else(|| {
                            EngineError::Other("Choose a plan file containing a sheet".into())
                        })?;
                    paths[index] =
                        sunipdf_core::plan_set::read_sheet(manifest, set, &sheet.id)?.path;
                }
                Ok(paths)
            }
        }
    }
}

pub enum Job {
    Inspect {
        sources: Sources,
        pairs: Vec<(usize, usize)>,
        passwords: Passwords,
        cancel: Arc<AtomicBool>,
    },
    Prepare {
        paths: [PathBuf; 2],
        request: RevisionTransferRequest,
        page: usize,
        passwords: Passwords,
        cancel: Arc<AtomicBool>,
    },
    Render {
        token: u64,
        page: usize,
        password: Zeroizing<String>,
        cancel: Arc<AtomicBool>,
    },
    Save {
        token: u64,
        output: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    QueueCurrent {
        token: u64,
        id: String,
        output: PathBuf,
        reviewed: Vec<usize>,
        cancel: Arc<AtomicBool>,
    },
    RemoveQueued {
        id: String,
    },
    ClearBatch,
    SaveBatch {
        cancel: Arc<AtomicBool>,
    },
    DiscardAll,
    Discard,
}

pub enum Outcome {
    Inspected {
        paths: [PathBuf; 2],
        inspection: TransferInspection,
    },
    Preview {
        token: u64,
        page: usize,
        image: RenderedPage,
        report: core::RevisionTransferReport,
    },
    Saved(core::RevisionTransferReport),
    BatchChanged(Vec<TransferBatchEntrySummary>),
    BatchSaved(TransferBatchOutcome),
    Discarded,
}

/// Instantiated exclusively inside worker::run; prepared bytes stay worker-owned.
#[derive(Default)]
pub struct WorkerCache {
    prepared: Option<(u64, core::PreparedRevisionTransfer)>,
    paths: Option<[PathBuf; 2]>,
    rendered: HashSet<usize>,
    batch: PreparedTransferBatch,
}

fn passwords(values: &Passwords) -> core::TransferPasswords<'_> {
    core::TransferPasswords {
        old: (!values[0].is_empty()).then_some(values[0].as_str()),
        new: (!values[1].is_empty()).then_some(values[1].as_str()),
    }
}

impl WorkerCache {
    pub fn touches_modified(&self, job: &Job, path: &Path) -> bool {
        if matches!(job, Job::SaveBatch { .. }) {
            return self
                .batch
                .summaries()
                .iter()
                .any(|entry| same_path(&entry.old_path, path) || same_path(&entry.new_path, path));
        }
        let paths = match job {
            Job::Prepare { paths, .. } => Some(paths),
            Job::Render { .. } | Job::Save { .. } | Job::QueueCurrent { .. } => self.paths.as_ref(),
            _ => None,
        };
        paths.is_some_and(|paths| paths.iter().any(|source| same_path(source, path)))
    }

    pub fn run(
        &mut self,
        engine: &Engine,
        generation: u64,
        job: Job,
    ) -> Result<Outcome, EngineError> {
        self.run_with_progress(engine, generation, job, |_| {})
    }

    pub fn run_with_progress(
        &mut self,
        engine: &Engine,
        generation: u64,
        job: Job,
        progress: impl FnMut(TransferBatchProgress),
    ) -> Result<Outcome, EngineError> {
        let retain_current_on_error = matches!(job, Job::QueueCurrent { .. });
        let result = self.run_inner(engine, generation, job, progress);
        if result.is_err() && !retain_current_on_error {
            self.prepared = None;
            self.paths = None;
            self.rendered.clear();
        }
        result
    }

    fn run_inner(
        &mut self,
        engine: &Engine,
        generation: u64,
        job: Job,
        progress: impl FnMut(TransferBatchProgress),
    ) -> Result<Outcome, EngineError> {
        match job {
            Job::Inspect {
                sources,
                pairs,
                passwords: secrets,
                cancel,
            } => {
                self.prepared = None;
                self.paths = None;
                self.rendered.clear();
                if cancel.load(Ordering::Relaxed) {
                    return Err(EngineError::Cancelled);
                }
                let paths = sources.resolve()?;
                let inspection = core::inspect_transfer_candidates(
                    engine,
                    &paths[0],
                    &paths[1],
                    passwords(&secrets),
                    &pairs,
                    &cancel,
                )?;
                if let Sources::Plan { set, files, .. } = &sources {
                    for (index, actual) in [&inspection.old_sha256, &inspection.new_sha256]
                        .iter()
                        .enumerate()
                    {
                        if set
                            .files
                            .iter()
                            .find(|f| f.id == files[index])
                            .map(|f| &f.sha256)
                            != Some(*actual)
                        {
                            return Err(EngineError::Other("Plan reference changed during inspection; validate the manifest again".into()));
                        }
                    }
                }
                Ok(Outcome::Inspected { paths, inspection })
            }
            Job::Prepare {
                paths,
                request,
                page,
                passwords: secrets,
                cancel,
            } => {
                self.prepared = None;
                self.paths = None;
                self.rendered.clear();
                let prepared = core::prepare_revision_transfer(
                    engine,
                    &paths[0],
                    &paths[1],
                    passwords(&secrets),
                    &request,
                    &cancel,
                )?;
                let image = prepared.render_preview(
                    engine,
                    page,
                    passwords(&secrets).new,
                    1024,
                    &cancel,
                )?;
                let report = prepared.report().clone();
                self.prepared = Some((generation, prepared));
                self.paths = Some(paths);
                self.rendered.insert(page);
                Ok(Outcome::Preview {
                    token: generation,
                    page,
                    image,
                    report,
                })
            }
            Job::Render {
                token,
                page,
                password,
                cancel,
            } => {
                let (_, prepared) = self
                    .prepared
                    .as_ref()
                    .filter(|(id, _)| *id == token)
                    .ok_or_else(|| {
                        EngineError::Other("Prepared transfer expired; preview again".into())
                    })?;
                let image = prepared.render_preview(
                    engine,
                    page,
                    (!password.is_empty()).then_some(password.as_str()),
                    1024,
                    &cancel,
                )?;
                self.rendered.insert(page);
                Ok(Outcome::Preview {
                    token,
                    page,
                    image,
                    report: prepared.report().clone(),
                })
            }
            Job::Save {
                token,
                output,
                cancel,
            } => {
                let (id, prepared) = self.prepared.take().ok_or_else(|| {
                    EngineError::Other("Prepared transfer expired; preview again".into())
                })?;
                if id != token {
                    return Err(EngineError::Other("Prepared transfer token changed".into()));
                }
                core::publish_revision_transfer(prepared, &output, &cancel).map(Outcome::Saved)
            }
            Job::QueueCurrent {
                token,
                id,
                output,
                reviewed,
                cancel,
            } => {
                if cancel.load(Ordering::Relaxed) {
                    return Err(EngineError::Cancelled);
                }
                let (_, prepared) = self
                    .prepared
                    .as_ref()
                    .filter(|(id, _)| *id == token)
                    .ok_or_else(|| {
                        EngineError::Other("Prepared transfer expired; preview again".into())
                    })?;
                if prepared.report().pages.iter().any(|page| {
                    !self.rendered.contains(&page.new_page) || !reviewed.contains(&page.new_page)
                }) {
                    return Err(EngineError::Other(
                        "Preview and review every destination page before adding the transfer"
                            .into(),
                    ));
                }
                let (_, prepared) = self.prepared.take().expect("checked current candidate");
                if let Err(refused) = self.batch.push(id, prepared, output) {
                    let refused = *refused;
                    self.prepared = Some((token, refused.prepared));
                    return Err(refused.error);
                }
                self.paths = None;
                self.rendered.clear();
                Ok(Outcome::BatchChanged(self.batch.summaries()))
            }
            Job::RemoveQueued { id } => {
                self.batch
                    .remove(&id)
                    .ok_or_else(|| EngineError::Other("Queued transfer no longer exists".into()))?;
                Ok(Outcome::BatchChanged(self.batch.summaries()))
            }
            Job::ClearBatch => {
                self.batch.clear();
                Ok(Outcome::BatchChanged(self.batch.summaries()))
            }
            Job::SaveBatch { cancel } => Ok(Outcome::BatchSaved(
                std::mem::take(&mut self.batch).publish(&cancel, progress),
            )),
            Job::DiscardAll => {
                self.batch.clear();
                self.prepared = None;
                self.paths = None;
                self.rendered.clear();
                Ok(Outcome::Discarded)
            }
            Job::Discard => {
                self.prepared = None;
                self.paths = None;
                self.rendered.clear();
                Ok(Outcome::Discarded)
            }
        }
    }
}

#[derive(Clone)]
struct PairDraft {
    old_page: usize,
    new_page: usize,
    old_sheet_id: Option<String>,
    new_sheet_id: Option<String>,
    confirmed: bool,
    anchors: bool,
    points: [[[f64; 2]; 2]; 2],
    selected: HashSet<usize>,
}

impl Default for PairDraft {
    fn default() -> Self {
        Self {
            old_page: 1,
            new_page: 1,
            old_sheet_id: None,
            new_sheet_id: None,
            confirmed: false,
            anchors: false,
            points: [[[0.0, 0.0], [100.0, 0.0]]; 2],
            selected: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Inspect,
    Preview,
    Save,
    Queue,
    RemoveQueued,
    ClearBatch,
    SaveBatch,
    DiscardAll,
}

struct PreviewImage {
    page: usize,
    texture: egui::TextureHandle,
}

struct UnmatchedSheet {
    source: usize,
    sheet: sunipdf_core::plan_set::PlanSheet,
    reason: &'static str,
}

pub struct TransferState {
    pub open: bool,
    generation: u64,
    pending: Option<(u64, Pending)>,
    cancel: Arc<AtomicBool>,
    paths: [Option<PathBuf>; 2],
    manifest: Option<(sunipdf_core::plan_set::PlanSet, PathBuf)>,
    files: [String; 2],
    unmatched: Vec<UnmatchedSheet>,
    passwords: Passwords,
    pairs: Vec<PairDraft>,
    selected_pair: usize,
    inspection: Option<TransferInspection>,
    token: Option<u64>,
    image: Option<PreviewImage>,
    reviewed: HashSet<usize>,
    report: Option<core::RevisionTransferReport>,
    repeated: bool,
    message: Option<String>,
    error: Option<String>,
    batch: BatchState,
}

impl Default for TransferState {
    fn default() -> Self {
        Self {
            open: false,
            generation: 0,
            pending: None,
            cancel: Arc::new(AtomicBool::new(false)),
            paths: [None, None],
            manifest: None,
            files: Default::default(),
            unmatched: Vec::new(),
            passwords: Default::default(),
            pairs: vec![PairDraft::default()],
            selected_pair: 0,
            inspection: None,
            token: None,
            image: None,
            reviewed: HashSet::new(),
            report: None,
            repeated: false,
            message: None,
            error: None,
            batch: BatchState::default(),
        }
    }
}

impl TransferState {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn begin(&mut self, kind: Pending) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some((self.generation, kind));
        self.cancel = Arc::new(AtomicBool::new(false));
        self.error = None;
        self.message = None;
        self.generation
    }
    fn invalidate(&mut self, sources: bool) {
        self.token = None;
        self.image = None;
        self.reviewed.clear();
        self.report = None;
        self.error = None;
        self.message = None;
        if sources {
            self.inspection = None;
            self.repeated = false;
            for pair in &mut self.pairs {
                pair.confirmed = false;
                pair.selected.clear();
            }
        }
    }
    fn request(&self) -> Option<RevisionTransferRequest> {
        let inspection = self.inspection.as_ref()?;
        let mut pairs = Vec::new();
        for draft in &self.pairs {
            let page = inspection
                .pages
                .iter()
                .find(|p| p.old_page == draft.old_page && p.new_page == draft.new_page)?;
            let selections: Vec<_> = page
                .candidates
                .iter()
                .filter(|c| draft.selected.contains(&c.index))
                .map(|c| {
                    Some(TransferSelection {
                        index: c.index,
                        fingerprint: c.fingerprint.clone()?,
                    })
                })
                .collect::<Option<_>>()?;
            pairs.push(TransferPagePair {
                old_page: draft.old_page,
                new_page: draft.new_page,
                old_sheet_id: draft.old_sheet_id.clone(),
                new_sheet_id: draft.new_sheet_id.clone(),
                confirmed: draft.confirmed,
                alignment: if draft.anchors {
                    TransferAlignment::Anchors {
                        old_points: draft.points[0],
                        new_points: draft.points[1],
                    }
                } else {
                    TransferAlignment::Identity
                },
                selections,
            });
        }
        Some(RevisionTransferRequest {
            version: 1,
            old_sha256: inspection.old_sha256.clone(),
            new_sha256: inspection.new_sha256.clone(),
            pairs,
            acknowledge_repeated_transfer: self.repeated,
        })
    }
    fn can_prepare(&self) -> bool {
        !self.busy()
            && self.request().is_some_and(|r| {
                r.pairs.iter().all(|p| p.confirmed)
                    && r.pairs.iter().any(|p| !p.selections.is_empty())
                    && core::encode_request(&r).is_ok()
            })
    }
    fn can_save(&self) -> bool {
        !self.busy()
            && self.token.is_some()
            && self
                .pairs
                .iter()
                .all(|p| self.reviewed.contains(&p.new_page))
    }
    fn cancel_pending(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        // A committed copy must still be reported even if cancellation raced publication.
        if self.pending.is_some_and(|(_, kind)| {
            matches!(
                kind,
                Pending::Save
                    | Pending::Queue
                    | Pending::RemoveQueued
                    | Pending::ClearBatch
                    | Pending::SaveBatch
                    | Pending::DiscardAll
            )
        }) {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.invalidate(false);
        self.passwords = Default::default();
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.into());
    let b = b.canonicalize().unwrap_or_else(|_| b.into());
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

impl SuniApp {
    pub(crate) fn cancel_revision_transfer_for_exit(&mut self) {
        self.revision_transfer.cancel.store(true, Ordering::Relaxed);
    }

    pub(crate) fn close_revision_transfer(&mut self) {
        if self.guard_revision_transfer_quit() {
            return;
        }
        if self.revision_transfer.busy() {
            self.cancel_revision_transfer_for_exit();
            return;
        }
        self.revision_transfer.cancel_pending();
        self.revision_transfer.open = false;
        self.discard_revision_transfer();
    }

    pub(crate) fn open_revision_transfer(&mut self) {
        if self.revision_transfer.busy()
            || self.ink_blocks_action()
            || self.worker.has_pending_mutations()
            || self.saving
        {
            return;
        }
        self.revision_transfer.manifest = None;
        self.revision_transfer.unmatched.clear();
        for pair in &mut self.revision_transfer.pairs {
            pair.old_sheet_id = None;
            pair.new_sheet_id = None;
        }
        self.revision_transfer.invalidate(true);
        self.discard_revision_transfer();
        self.revision_transfer.open = true;
    }

    pub(crate) fn open_revision_transfer_for_set(
        &mut self,
        set: sunipdf_core::plan_set::PlanSet,
        manifest_path: PathBuf,
    ) -> bool {
        if self.revision_transfer.busy()
            || self.ink_blocks_action()
            || self.worker.has_pending_mutations()
            || self.saving
        {
            return false;
        }
        self.discard_revision_transfer();
        self.revision_transfer.invalidate(true);
        self.revision_transfer.manifest = Some((set, manifest_path));
        self.revision_transfer.files = Default::default();
        self.revision_transfer.unmatched.clear();
        self.revision_transfer.paths = [None, None];
        self.revision_transfer.pairs = vec![PairDraft::default()];
        self.revision_transfer.selected_pair = 0;
        self.revision_transfer.passwords = Default::default();
        self.revision_transfer.open = true;
        true
    }

    fn transfer_has_dirty_source(&self) -> bool {
        self.revision_transfer.paths.iter().flatten().any(|path| {
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
    }

    fn discard_revision_transfer(&mut self) {
        self.revision_transfer.invalidate(false);
        self.worker.send(Request::RevisionTransfer {
            generation: self.revision_transfer.generation,
            job: Job::Discard,
        });
    }

    fn transfer_sources(&self) -> Option<Sources> {
        if let Some((set, manifest)) = &self.revision_transfer.manifest {
            if self.revision_transfer.files.iter().any(String::is_empty) {
                return None;
            }
            Some(Sources::Plan {
                set: set.clone(),
                manifest: manifest.clone(),
                files: self.revision_transfer.files.clone(),
            })
        } else {
            let [Some(old), Some(new)] = self.revision_transfer.paths.clone() else {
                return None;
            };
            Some(Sources::Files([old, new]))
        }
    }

    fn inspect_revision_transfer(&mut self) {
        if self.revision_transfer.busy() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        let Some(sources) = self.transfer_sources() else {
            return;
        };
        self.revision_transfer.invalidate(true);
        let generation = self.revision_transfer.begin(Pending::Inspect);
        let job = Job::Inspect {
            sources,
            pairs: self
                .revision_transfer
                .pairs
                .iter()
                .map(|p| (p.old_page, p.new_page))
                .collect(),
            passwords: std::mem::take(&mut self.revision_transfer.passwords),
            cancel: self.revision_transfer.cancel.clone(),
        };
        self.worker
            .send(Request::RevisionTransfer { generation, job });
    }

    fn preview_revision_transfer(&mut self) {
        if self.revision_transfer.busy() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        if self.transfer_has_dirty_source() {
            self.revision_transfer.error = Some(tr(self.lang(), "transfer.dirty_source").into());
            return;
        }
        let page = self.revision_transfer.pairs[self.revision_transfer.selected_pair].new_page;
        let job = if let Some(token) = self.revision_transfer.token {
            let generation = self.revision_transfer.begin(Pending::Preview);
            let job = Job::Render {
                token,
                page,
                password: std::mem::take(&mut self.revision_transfer.passwords[1]),
                cancel: self.revision_transfer.cancel.clone(),
            };
            self.revision_transfer.passwords[0] = Default::default();
            (generation, job)
        } else {
            if !self.revision_transfer.can_prepare() {
                return;
            }
            let Some(request) = self.revision_transfer.request() else {
                return;
            };
            let [Some(old), Some(new)] = self.revision_transfer.paths.clone() else {
                return;
            };
            let generation = self.revision_transfer.begin(Pending::Preview);
            (
                generation,
                Job::Prepare {
                    paths: [old, new],
                    request,
                    page,
                    passwords: std::mem::take(&mut self.revision_transfer.passwords),
                    cancel: self.revision_transfer.cancel.clone(),
                },
            )
        };
        self.worker.send(Request::RevisionTransfer {
            generation: job.0,
            job: job.1,
        });
    }

    fn save_revision_transfer(&mut self, output: PathBuf) {
        if !self.revision_transfer.can_save() {
            return;
        }
        if self.transfer_has_dirty_source() {
            self.revision_transfer.error = Some(tr(self.lang(), "transfer.dirty_source").into());
            return;
        }
        let token = self.revision_transfer.token.expect("checked candidate");
        let generation = self.revision_transfer.begin(Pending::Save);
        self.worker.send(Request::RevisionTransfer {
            generation,
            job: Job::Save {
                token,
                output,
                cancel: self.revision_transfer.cancel.clone(),
            },
        });
    }

    pub(crate) fn on_revision_transfer(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if self.revision_transfer.pending.map(|p| p.0) != Some(generation) {
            return;
        }
        let pending = self.revision_transfer.pending.take().map(|(_, kind)| kind);
        match result {
            Ok(Outcome::Inspected { paths, inspection }) => {
                self.revision_transfer.paths = paths.map(Some);
                self.revision_transfer.inspection = Some(inspection);
            }
            Ok(Outcome::Preview {
                token,
                page,
                image,
                report,
            }) => {
                self.revision_transfer.token = Some(token);
                self.revision_transfer.image = Some(PreviewImage {
                    page,
                    texture: ctx.load_texture(
                        "revision_transfer_candidate",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    ),
                });
                self.revision_transfer.report = Some(report);
            }
            Ok(Outcome::Saved(report)) => {
                self.revision_transfer.token = None;
                self.revision_transfer.image = None;
                self.revision_transfer.reviewed.clear();
                self.revision_transfer.passwords = Default::default();
                self.revision_transfer.message = Some(tr_args(
                    self.lang(),
                    "transfer.saved",
                    &[(
                        "path",
                        &report
                            .output_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                    )],
                ));
                self.revision_transfer.report = Some(report);
            }
            Ok(Outcome::BatchChanged(entries)) => {
                self.revision_transfer.batch.entries = entries;
                self.revision_transfer.batch.confirm_discard = false;
                if pending == Some(Pending::Queue) {
                    self.revision_transfer.invalidate(true);
                    self.revision_transfer.passwords = Default::default();
                    self.revision_transfer.message =
                        Some(tr(self.lang(), "transfer.batch.queued").into());
                }
            }
            Ok(Outcome::BatchSaved(outcome)) => {
                self.revision_transfer.batch.entries.clear();
                self.revision_transfer.batch.progress = None;
                self.revision_transfer.batch.confirm_discard = false;
                self.revision_transfer.batch.outcome = Some(outcome);
                self.revision_transfer.open = true;
            }
            Ok(Outcome::Discarded) => {
                if pending == Some(Pending::DiscardAll) {
                    self.revision_transfer.batch.entries.clear();
                    self.revision_transfer.batch.confirm_discard = false;
                    self.revision_transfer.open = false;
                    self.revision_transfer.invalidate(true);
                }
            }
            Err(error) => {
                self.revision_transfer.batch.progress = None;
                if pending != Some(Pending::Queue) {
                    self.revision_transfer.token = None;
                    self.revision_transfer.image = None;
                    self.revision_transfer.reviewed.clear();
                }
                self.revision_transfer.error = Some(format!(
                    "{} ({error})",
                    tr(self.lang(), crate::app::error_key(&error))
                ));
                if pending != Some(Pending::Queue) {
                    self.worker.send(Request::RevisionTransfer {
                        generation,
                        job: Job::Discard,
                    });
                }
            }
        }
    }

    fn suggest_transfer_pairs(&mut self) {
        let Some((set, _)) = &self.revision_transfer.manifest else {
            return;
        };
        let old: Vec<_> = set
            .sheets
            .iter()
            .filter(|s| s.file_id == self.revision_transfer.files[0])
            .cloned()
            .collect();
        let new: Vec<_> = set
            .sheets
            .iter()
            .filter(|s| s.file_id == self.revision_transfer.files[1])
            .cloned()
            .collect();
        self.revision_transfer.invalidate(true);
        self.revision_transfer.unmatched.clear();
        let suggestions = match core::suggest_sheet_pairs(&old, &new) {
            Ok(v) => v,
            Err(e) => {
                self.revision_transfer.error = Some(e.to_string());
                return;
            }
        };
        let reverse = match core::suggest_sheet_pairs(&new, &old) {
            Ok(v) => v,
            Err(e) => {
                self.revision_transfer.error = Some(e.to_string());
                return;
            }
        };
        self.revision_transfer.unmatched = [(0, &old, &suggestions), (1, &new, &reverse)]
            .into_iter()
            .flat_map(|(source, sheets, matches)| {
                matches.iter().filter_map(move |candidate| {
                    if candidate.new_sheet_id.is_some() {
                        return None;
                    }
                    let sheet = sheets.iter().find(|s| s.id == candidate.old_sheet_id)?;
                    Some(UnmatchedSheet {
                        source,
                        sheet: sheet.clone(),
                        reason: if candidate.readiness == core::TransferReadiness::Ambiguous {
                            "transfer.match_ambiguous"
                        } else {
                            "transfer.match_unmatched"
                        },
                    })
                })
            })
            .collect();
        let pairs: Vec<_> = suggestions
            .iter()
            .filter_map(|suggestion| {
                let old = old.iter().find(|s| s.id == suggestion.old_sheet_id)?;
                let new = new
                    .iter()
                    .find(|s| Some(&s.id) == suggestion.new_sheet_id.as_ref())?;
                Some(PairDraft {
                    old_page: old.page,
                    new_page: new.page,
                    old_sheet_id: Some(old.id.clone()),
                    new_sheet_id: Some(new.id.clone()),
                    ..Default::default()
                })
            })
            .collect();
        if pairs.is_empty() || pairs.len() > core::MAX_PAGE_PAIRS {
            self.revision_transfer.error =
                Some(tr(self.lang(), "transfer.suggestion_limit").into());
            return;
        }
        let excluded = self.revision_transfer.unmatched.len();
        self.revision_transfer.pairs = pairs;
        self.revision_transfer.selected_pair = 0;
        self.revision_transfer.invalidate(true);
        self.revision_transfer.message = Some(tr_args(
            self.lang(),
            "transfer.suggested",
            &[
                ("count", &self.revision_transfer.pairs.len().to_string()),
                ("excluded", &excluded.to_string()),
            ],
        ));
    }

    fn use_manual_transfer_sheet(&mut self, index: usize) {
        if self.revision_transfer.busy() {
            return;
        }
        let Some(sheet) = self.revision_transfer.unmatched.get(index) else {
            return;
        };
        let Some(pair) = self
            .revision_transfer
            .pairs
            .get_mut(self.revision_transfer.selected_pair)
        else {
            return;
        };
        if sheet.source == 0 {
            pair.old_page = sheet.sheet.page;
            pair.old_sheet_id = Some(sheet.sheet.id.clone());
        } else {
            pair.new_page = sheet.sheet.page;
            pair.new_sheet_id = Some(sheet.sheet.id.clone());
        }
        pair.confirmed = false;
        self.revision_transfer.invalidate(true);
        self.discard_revision_transfer();
    }

    pub(crate) fn revision_transfer_dialog(&mut self, ctx: &egui::Context) {
        if !self.revision_transfer.open {
            return;
        }
        let lang = self.lang();
        let busy = self.revision_transfer.busy();
        let can_inspect =
            !busy && self.transfer_sources().is_some() && matches!(self.engine, EngineState::Ready);
        let can_preview = !busy
            && (self.revision_transfer.can_prepare() || self.revision_transfer.token.is_some());
        let can_save = self.revision_transfer.can_save();
        let can_queue = self.revision_transfer.can_queue();
        let compact_footer = ctx.content_rect().width() < 680.0;
        let mut queue = false;
        let mut batch_actions = batch::BatchActions::default();
        let (mut pick, mut inspect, mut preview, mut save, mut close, mut cancel, mut suggest) =
            (None, false, false, false, false, false, false);
        let (mut source_changed, mut changed) = (false, false);
        let mut remove = None;
        let mut manual_sheet = None;
        dialog::show(
            ctx,
            Spec::new(
                "revision_transfer",
                tr(lang, "transfer.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(900.0_f32.min((ctx.content_rect().width() - 80.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "transfer.saved_files"));
                ui.small(tr(lang, "transfer.limits"));
                batch_actions =
                    batch::render_queue(ui, lang, &mut self.revision_transfer.batch, busy);
                ui.add_enabled_ui(!busy, |ui| {
                    ui.columns(2, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            let label = tr(
                                lang,
                                if index == 0 {
                                    "transfer.old"
                                } else {
                                    "transfer.new"
                                },
                            );
                            column.strong(label);
                            if let Some((set, _)) = &self.revision_transfer.manifest {
                                let selected = set
                                    .files
                                    .iter()
                                    .find(|f| f.id == self.revision_transfer.files[index])
                                    .map(|f| f.path.as_str())
                                    .unwrap_or(tr(lang, "transfer.choose"));
                                egui::ComboBox::from_id_salt(("transfer_file", index))
                                    .truncate()
                                    .selected_text(selected)
                                    .width(column.available_width().max(80.0))
                                    .show_ui(column, |ui| {
                                        for file in &set.files {
                                            source_changed |= ui
                                                .selectable_value(
                                                    &mut self.revision_transfer.files[index],
                                                    file.id.clone(),
                                                    &file.path,
                                                )
                                                .changed();
                                        }
                                    });
                            } else {
                                if column.button(tr(lang, "transfer.choose")).clicked() {
                                    pick = Some(index);
                                }
                                if let Some(path) = &self.revision_transfer.paths[index] {
                                    column
                                        .add(
                                            egui::Label::new(crate::app::file_name(path))
                                                .truncate(),
                                        )
                                        .on_hover_text(path.display().to_string());
                                }
                            }
                            column.label(tr(lang, "overlay.password"));
                            column.add(
                                egui::TextEdit::singleline(
                                    &mut *self.revision_transfer.passwords[index],
                                )
                                .password(true)
                                .char_limit(256)
                                .desired_width(column.available_width()),
                            );
                        }
                    });
                    ui.small(tr(lang, "transfer.password_hint"));
                    if source_changed {
                        self.revision_transfer.paths = [None, None];
                        self.revision_transfer.unmatched.clear();
                        for pair in &mut self.revision_transfer.pairs {
                            pair.old_sheet_id = None;
                            pair.new_sheet_id = None;
                        }
                    }
                    if self.revision_transfer.manifest.is_some() {
                        suggest = ui.button(tr(lang, "transfer.suggest")).clicked();
                    }
                    if !self.revision_transfer.unmatched.is_empty() {
                        ui.strong(tr(lang, "transfer.unmatched_title"));
                        ui.small(tr(lang, "transfer.manual_help"));
                        egui::ScrollArea::vertical()
                            .id_salt("transfer_unmatched")
                            .max_height(180.0)
                            .show(ui, |ui| {
                                for (index, item) in
                                    self.revision_transfer.unmatched.iter().enumerate()
                                {
                                    ui.push_id(("unmatched", index), |ui| {
                                        ui.strong(format!(
                                            "{} · {} · {}",
                                            tr(
                                                lang,
                                                if item.source == 0 {
                                                    "transfer.old"
                                                } else {
                                                    "transfer.new"
                                                }
                                            ),
                                            item.sheet.number,
                                            item.sheet.title,
                                        ));
                                        ui.label(tr_args(
                                            lang,
                                            "transfer.unmatched_details",
                                            &[
                                                ("discipline", &item.sheet.discipline),
                                                ("id", &item.sheet.id),
                                                ("page", &item.sheet.page.to_string()),
                                            ],
                                        ));
                                        ui.label(tr(lang, item.reason));
                                        if ui
                                            .button(tr(lang, "transfer.use_manual_sheet"))
                                            .clicked()
                                        {
                                            manual_sheet = Some(index);
                                        }
                                        ui.separator();
                                    });
                                }
                            });
                    }
                    ui.separator();
                    ui.strong(tr(lang, "transfer.pages"));
                    let old_max = self
                        .revision_transfer
                        .inspection
                        .as_ref()
                        .map_or(65535, |i| i.old_page_count);
                    let new_max = self
                        .revision_transfer
                        .inspection
                        .as_ref()
                        .map_or(65535, |i| i.new_page_count);
                    for (index, pair) in self.revision_transfer.pairs.iter_mut().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.selectable_value(
                                    &mut self.revision_transfer.selected_pair,
                                    index,
                                    format!("{}", index + 1),
                                );
                                ui.label(tr(lang, "transfer.old_page"));
                                let old_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut pair.old_page).range(1..=old_max),
                                    )
                                    .changed();
                                ui.label(tr(lang, "transfer.new_page"));
                                let new_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut pair.new_page).range(1..=new_max),
                                    )
                                    .changed();
                                if old_changed || new_changed {
                                    pair.old_sheet_id = None;
                                    pair.new_sheet_id = None;
                                    source_changed = true;
                                }
                                if ui.button(tr(lang, "transfer.remove_pair")).clicked() {
                                    remove = Some(index);
                                }
                            });
                        });
                    }
                    if ui
                        .add_enabled(
                            self.revision_transfer.pairs.len() < core::MAX_PAGE_PAIRS,
                            egui::Button::new(tr(lang, "transfer.add_pair")),
                        )
                        .clicked()
                    {
                        self.revision_transfer.pairs.push(PairDraft::default());
                        source_changed = true;
                    }
                    if source_changed {
                        self.revision_transfer.invalidate(true);
                    }
                    if let Some(inspection) = &self.revision_transfer.inspection {
                        ui.collapsing(tr(lang, "overlay.fingerprints"), |ui| {
                            ui.label(tr(lang, "transfer.old"));
                            ui.monospace(&inspection.old_sha256);
                            ui.label(tr(lang, "transfer.new"));
                            ui.monospace(&inspection.new_sha256);
                        });
                        for warning in &inspection.warnings {
                            ui.label(warning);
                        }
                        changed |= ui
                            .checkbox(
                                &mut self.revision_transfer.repeated,
                                tr(lang, "transfer.repeat_ack"),
                            )
                            .changed();
                        let total: usize = self
                            .revision_transfer
                            .pairs
                            .iter()
                            .map(|p| p.selected.len())
                            .sum();
                        let pair =
                            &mut self.revision_transfer.pairs[self.revision_transfer.selected_pair];
                        if let Some(page) = inspection
                            .pages
                            .iter()
                            .find(|p| p.old_page == pair.old_page && p.new_page == pair.new_page)
                        {
                            ui.separator();
                            ui.strong(tr(lang, "transfer.annotations"));
                            if page.candidates.is_empty() {
                                ui.label(tr(lang, "transfer.no_annotations"));
                            }
                            for candidate in &page.candidates {
                                let compatible =
                                    candidate.refusal.is_none() && candidate.fingerprint.is_some();
                                let mut selected = pair.selected.contains(&candidate.index);
                                let label =
                                    format!("#{} · {}", candidate.index + 1, candidate.subtype);
                                let enabled = compatible
                                    && (selected
                                        || (pair.selected.len() < core::MAX_SELECTIONS_PER_PAGE
                                            && total < core::MAX_SELECTIONS));
                                if ui
                                    .add_enabled(enabled, egui::Checkbox::new(&mut selected, label))
                                    .changed()
                                {
                                    if selected {
                                        pair.selected.insert(candidate.index);
                                    } else {
                                        pair.selected.remove(&candidate.index);
                                    }
                                    changed = true;
                                }
                                if let Some(reason) = &candidate.refusal {
                                    ui.small(reason);
                                }
                            }
                            ui.separator();
                            ui.strong(tr(lang, "transfer.alignment"));
                            let mut alignment_changed = ui
                                .radio_value(
                                    &mut pair.anchors,
                                    false,
                                    tr(lang, "transfer.identity"),
                                )
                                .changed();
                            alignment_changed |= ui
                                .radio_value(&mut pair.anchors, true, tr(lang, "transfer.anchors"))
                                .changed();
                            if pair.anchors {
                                alignment_changed |= anchor_controls(
                                    ui,
                                    lang,
                                    pair,
                                    [page.old_geometry, page.new_geometry],
                                );
                            }
                            if alignment_changed {
                                pair.confirmed = false;
                                changed = true;
                            }
                            ui.small(tr(lang, "transfer.coordinates"));
                            changed |= ui
                                .checkbox(&mut pair.confirmed, tr(lang, "transfer.confirm_pair"))
                                .changed();
                        }
                    }
                });
                if changed {
                    self.revision_transfer.invalidate(false);
                }
                if busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr(lang, "transfer.working"));
                    });
                }
                if let Some(image) = &self.revision_transfer.image {
                    ui.separator();
                    ui.strong(tr_args(
                        lang,
                        "transfer.candidate_page",
                        &[("page", &image.page.to_string())],
                    ));
                    let size = image.texture.size_vec2();
                    let scale = (ui.available_width() / size.x).min(380.0 / size.y).min(1.0);
                    ui.add(egui::Image::new((image.texture.id(), size * scale)));
                    let mut reviewed = self.revision_transfer.reviewed.contains(&image.page);
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Checkbox::new(&mut reviewed, tr(lang, "transfer.reviewed")),
                        )
                        .changed()
                    {
                        if reviewed {
                            self.revision_transfer.reviewed.insert(image.page);
                        } else {
                            self.revision_transfer.reviewed.remove(&image.page);
                        }
                    }
                }
                if let Some(report) = &self.revision_transfer.report {
                    ui.label(tr_args(
                        lang,
                        "transfer.summary",
                        &[
                            ("count", &report.transferred.len().to_string()),
                            ("excluded", &report.exclusions.len().to_string()),
                        ],
                    ));
                    for page in &report.pages {
                        ui.monospace(format!(
                            "{} → {} · x {:.3} pt · y {:.3} pt · {:.3}° · ×{:.6}",
                            page.old_page,
                            page.new_page,
                            page.transform.translation[0],
                            page.transform.translation[1],
                            page.transform.rotation_degrees,
                            page.transform.scale
                        ));
                    }
                    if let Some(hash) = &report.output_sha256 {
                        ui.monospace(hash);
                    }
                }
                if self.revision_transfer.token.is_some() {
                    ui.small(tr(lang, "transfer.review_all"));
                }
                queue = ui
                    .add_enabled(can_queue, egui::Button::new(tr(lang, "transfer.batch.add")))
                    .clicked();
                ui.small(tr(lang, "transfer.batch.add_help"));
                if let Some(error) = &self.revision_transfer.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = &self.revision_transfer.message {
                    ui.label(message);
                }
            },
            |ui| {
                if busy {
                    cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                } else {
                    save = ui
                        .add_enabled(
                            can_save,
                            egui::Button::new(tr(
                                lang,
                                if compact_footer {
                                    "transfer.save_short"
                                } else {
                                    "transfer.save"
                                },
                            )),
                        )
                        .clicked();
                    preview = ui
                        .add_enabled(
                            can_preview,
                            egui::Button::new(tr(
                                lang,
                                if compact_footer {
                                    "transfer.preview_short"
                                } else {
                                    "transfer.preview"
                                },
                            )),
                        )
                        .clicked();
                    inspect = ui
                        .add_enabled(
                            can_inspect,
                            egui::Button::new(tr(
                                lang,
                                if compact_footer {
                                    "transfer.inspect_short"
                                } else {
                                    "transfer.inspect"
                                },
                            )),
                        )
                        .clicked();
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                }
            },
        );
        if let Some(index) = remove {
            if self.revision_transfer.pairs.len() > 1 {
                self.revision_transfer.pairs.remove(index);
                self.revision_transfer.selected_pair = 0;
                self.revision_transfer.invalidate(true);
                source_changed = true;
            }
        }
        if source_changed || changed {
            self.worker.send(Request::RevisionTransfer {
                generation: self.revision_transfer.generation,
                job: Job::Discard,
            });
        }
        if let Some(index) = pick {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_file()
            {
                self.revision_transfer.paths[index] = Some(path);
                for pair in &mut self.revision_transfer.pairs {
                    pair.old_sheet_id = None;
                    pair.new_sheet_id = None;
                }
                self.revision_transfer.passwords[index] = Default::default();
                self.revision_transfer.invalidate(true);
                self.discard_revision_transfer();
            }
        }
        if suggest {
            self.suggest_transfer_pairs();
            self.worker.send(Request::RevisionTransfer {
                generation: self.revision_transfer.generation,
                job: Job::Discard,
            });
        }
        if let Some(index) = manual_sheet {
            self.use_manual_transfer_sheet(index);
        }
        if inspect {
            self.inspect_revision_transfer();
        }
        if preview {
            self.preview_revision_transfer();
        }
        if save {
            if let Some(output) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .set_file_name("revision-transfer.pdf")
                .save_file()
            {
                self.save_revision_transfer(output);
            }
        }
        if queue {
            if let Some(output) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .set_file_name("revision-transfer.pdf")
                .save_file()
            {
                self.queue_revision_transfer(output);
            }
        }
        self.apply_transfer_batch_action(batch_actions);
        if cancel {
            self.revision_transfer.cancel_pending();
            if !self.revision_transfer.busy() {
                self.discard_revision_transfer();
            }
        }
        if close {
            self.close_revision_transfer();
        }
    }
}

fn anchor_controls(
    ui: &mut egui::Ui,
    lang: Language,
    pair: &mut PairDraft,
    geometry: [core::TransferGeometry; 2],
) -> bool {
    let mut changed = false;
    ui.columns(2, |columns| {
        for source in 0..2 {
            columns[source].label(tr(
                lang,
                if source == 0 {
                    "transfer.old"
                } else {
                    "transfer.new"
                },
            ));
            for anchor in 0..2 {
                columns[source].push_id((source, anchor), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}", anchor + 1));
                        for (axis, max) in [geometry[source].width_pt, geometry[source].height_pt]
                            .into_iter()
                            .enumerate()
                        {
                            ui.label(if axis == 0 { "x" } else { "y" });
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut pair.points[source][anchor][axis])
                                        .range(0.0..=f64::from(max))
                                        .speed(0.1)
                                        .max_decimals(3),
                                )
                                .changed();
                        }
                    });
                });
            }
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::test_app;

    fn inspection() -> TransferInspection {
        let geometry = core::TransferGeometry {
            width_pt: 600.0,
            height_pt: 800.0,
            crop_box_user_pt: [0.0, 0.0, 600.0, 800.0],
            rotation_degrees: 0,
        };
        TransferInspection {
            old_sha256: "a".repeat(64),
            new_sha256: "b".repeat(64),
            old_page_count: 2,
            new_page_count: 2,
            pages: (1..=2)
                .map(|page| core::TransferPageInspection {
                    old_page: page,
                    new_page: page,
                    old_geometry: geometry,
                    new_geometry: geometry,
                    candidates: vec![
                        core::TransferCandidate {
                            index: 0,
                            subtype: "Square".into(),
                            fingerprint: Some("c".repeat(64)),
                            source_nm: Some(format!("shape-{page}")),
                            author: None,
                            refusal: None,
                        },
                        core::TransferCandidate {
                            index: 1,
                            subtype: "Stamp".into(),
                            fingerprint: None,
                            source_nm: None,
                            author: None,
                            refusal: Some(
                                "Measurement annotations require recalibration on a new revision"
                                    .into(),
                            ),
                        },
                    ],
                })
                .collect(),
            warnings: vec![],
        }
    }

    fn state() -> TransferState {
        TransferState {
            open: true,
            paths: [Some("old.pdf".into()), Some("new.pdf".into())],
            inspection: Some(inspection()),
            pairs: (1..=2)
                .map(|page| PairDraft {
                    old_page: page,
                    new_page: page,
                    confirmed: true,
                    selected: HashSet::from([0]),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    fn report() -> core::RevisionTransferReport {
        core::RevisionTransferReport {
            old_sha256: "a".repeat(64),
            new_sha256: "b".repeat(64),
            output_path: None,
            output_sha256: None,
            pages: vec![],
            transferred: vec![],
            exclusions: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn mapping_selections_bind_fingerprints_and_all_pages_require_review() {
        let mut state = state();
        assert!(state.can_prepare());
        let request = state.request().unwrap();
        assert_eq!(request.pairs[1].selections[0].fingerprint, "c".repeat(64));
        assert_eq!(request.pairs[1].alignment, TransferAlignment::Identity);
        state.pairs[1].confirmed = false;
        assert!(!state.can_prepare());
        state.pairs[1].confirmed = true;
        state.token = Some(7);
        state.reviewed.insert(1);
        assert!(
            !state.can_save(),
            "a single preview does not review the complete mapping"
        );
        state.reviewed.insert(2);
        assert!(state.can_save());
        state.pairs[0].selected.clear();
        state.invalidate(false);
        assert!(!state.can_save());
        assert!(state.token.is_none() && state.reviewed.is_empty());
        assert!(
            state.inspection.is_some(),
            "selection edits retain the inspected inventory"
        );
        state.invalidate(true);
        assert!(state.inspection.is_none());
        assert!(state
            .pairs
            .iter()
            .all(|p| p.selected.is_empty() && !p.confirmed));
    }

    #[test]
    fn cancel_discards_late_preview_without_clearing_a_newer_inspection() {
        let (mut app, _, _) = test_app();
        app.revision_transfer = state();
        let old = app.revision_transfer.begin(Pending::Preview);
        let cancellation = app.revision_transfer.cancel.clone();
        app.revision_transfer.cancel_pending();
        assert!(cancellation.load(Ordering::Relaxed));
        let current = app.revision_transfer.begin(Pending::Inspect);
        app.on_revision_transfer(
            &egui::Context::default(),
            old,
            Ok(Outcome::Preview {
                token: old,
                page: 1,
                image: RenderedPage {
                    page_index: 0,
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                },
                report: report(),
            }),
        );
        assert_eq!(
            app.revision_transfer.pending,
            Some((current, Pending::Inspect))
        );
        assert!(app.revision_transfer.token.is_none() && app.revision_transfer.image.is_none());
        app.on_revision_transfer(
            &egui::Context::default(),
            current,
            Ok(Outcome::Inspected {
                paths: ["old.pdf".into(), "new.pdf".into()],
                inspection: inspection(),
            }),
        );
        assert!(!app.revision_transfer.busy());
        assert!(app.revision_transfer.inspection.is_some());
    }

    #[test]
    fn cancelling_publication_preserves_the_committed_copy_response() {
        let (mut app, _, _) = test_app();
        app.revision_transfer = state();
        let generation = app.revision_transfer.begin(Pending::Save);
        app.revision_transfer.cancel_pending();
        assert_eq!(
            app.revision_transfer.pending,
            Some((generation, Pending::Save))
        );
        let mut saved = report();
        saved.output_path = Some("new-copy.pdf".into());
        saved.output_sha256 = Some("d".repeat(64));
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::Saved(saved)),
        );
        assert!(!app.revision_transfer.busy());
        assert!(app
            .revision_transfer
            .message
            .as_ref()
            .unwrap()
            .contains("new-copy.pdf"));
        assert!(app
            .revision_transfer
            .report
            .as_ref()
            .unwrap()
            .output_sha256
            .is_some());
    }

    #[test]
    fn dirty_sources_block_preview_and_save_without_changing_document_history() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        let path = app.document.as_ref().unwrap().info.path.clone();
        app.revision_transfer.paths[0] = Some(path);
        app.dirty = true;
        app.can_undo = true;
        app.preview_revision_transfer();
        assert!(app.revision_transfer.error.is_some());
        assert!(requests.try_iter().next().is_none());
        app.revision_transfer.token = Some(7);
        app.revision_transfer.reviewed.extend([1, 2]);
        app.save_revision_transfer("copy.pdf".into());
        assert!(requests.try_iter().next().is_none());
        assert!(app.dirty && app.can_undo);
    }

    #[test]
    fn passwords_leave_the_ui_once_a_job_starts_and_are_cleared_on_cancel() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer.passwords = [
            Zeroizing::new(["old", "test"].join("-")),
            Zeroizing::new(["new", "test"].join("-")),
        ];
        app.inspect_revision_transfer();
        assert!(app.revision_transfer.passwords.iter().all(|p| p.is_empty()));
        match requests.recv().unwrap() {
            Request::RevisionTransfer {
                job: Job::Inspect { passwords, .. },
                ..
            } => assert!(passwords.iter().all(|p| !p.is_empty())),
            _ => panic!("inspection job"),
        }
        app.revision_transfer.passwords[0] = Zeroizing::new("local-temporary".into());
        app.revision_transfer.cancel_pending();
        assert!(app.revision_transfer.passwords.iter().all(|p| p.is_empty()));
    }

    fn manifest() -> sunipdf_core::plan_set::PlanSet {
        use sunipdf_core::plan_set::{PlanFile, PlanSet, PlanSheet};
        PlanSet {
            version: 1,
            id: "plans".into(),
            name: "Revisions".into(),
            files: ["old", "new"]
                .into_iter()
                .map(|id| PlanFile {
                    id: id.into(),
                    path: format!("{id}.pdf"),
                    sha256: "a".repeat(64),
                    byte_len: 100,
                    page_count: 4,
                })
                .collect(),
            sheets: [
                ("old-unique", "old", 1, "A1"),
                ("old-missing", "old", 2, "M1"),
                ("old-ambiguous", "old", 3, "D1"),
                ("new-unique", "new", 1, "A1"),
                ("new-duplicate-one", "new", 2, "D1"),
                ("new-duplicate-two", "new", 3, "D1"),
                ("new-missing", "new", 4, "N1"),
            ]
            .into_iter()
            .map(|(id, file_id, page, number)| PlanSheet {
                id: id.into(),
                file_id: file_id.into(),
                page,
                number: number.into(),
                title: format!("Drawing {id}"),
                discipline: "Architecture".into(),
                revision: file_id.into(),
            })
            .collect(),
        }
    }

    #[test]
    fn suggestions_keep_both_sides_of_ambiguous_and_unmatched_sheets_for_manual_mapping() {
        let (mut app, requests, _) = test_app();
        assert!(app.open_revision_transfer_for_set(manifest(), "set.json".into()));
        app.revision_transfer.files = ["old".into(), "new".into()];
        app.suggest_transfer_pairs();
        let state = &app.revision_transfer;
        assert_eq!(state.pairs.len(), 1);
        assert!(!state.pairs[0].confirmed);
        assert_eq!(state.pairs[0].old_sheet_id.as_deref(), Some("old-unique"));
        assert_eq!(state.pairs[0].new_sheet_id.as_deref(), Some("new-unique"));
        assert_eq!(state.unmatched.len(), 5);
        assert_eq!(
            state
                .unmatched
                .iter()
                .filter(|s| s.reason == "transfer.match_ambiguous")
                .count(),
            3,
        );
        let old = state
            .unmatched
            .iter()
            .position(|s| s.sheet.id == "old-missing")
            .unwrap();
        let new = state
            .unmatched
            .iter()
            .position(|s| s.sheet.id == "new-missing")
            .unwrap();
        app.revision_transfer.inspection = Some(inspection());
        app.revision_transfer.token = Some(99);
        app.revision_transfer.reviewed.insert(1);
        app.use_manual_transfer_sheet(old);
        let state = &app.revision_transfer;
        assert_eq!(state.pairs[0].old_page, 2);
        assert_eq!(
            state.pairs[0].new_page, 1,
            "the other side is never inferred"
        );
        assert_eq!(state.pairs[0].old_sheet_id.as_deref(), Some("old-missing"));
        assert!(state.inspection.is_none() && state.token.is_none() && state.reviewed.is_empty());
        assert!(!state.pairs[0].confirmed && !state.can_prepare());
        app.use_manual_transfer_sheet(new);
        assert_eq!(app.revision_transfer.pairs[0].new_page, 4);
        assert_eq!(app.revision_transfer.pairs[0].old_page, 2);
        assert_eq!(
            app.revision_transfer.unmatched.len(),
            5,
            "manual choice does not rewrite matching evidence"
        );
        assert!(requests.try_iter().all(|r| matches!(
            r,
            Request::RevisionTransfer {
                job: Job::Discard,
                ..
            }
        )));
    }

    #[test]
    fn empty_suggestions_remain_explained_without_reusing_a_previous_candidate() {
        let (mut app, _, _) = test_app();
        let mut set = manifest();
        set.sheets.retain(|s| s.id.ends_with("missing"));
        app.revision_transfer = state();
        app.revision_transfer.token = Some(88);
        app.revision_transfer.manifest = Some((set, "set.json".into()));
        app.revision_transfer.files = ["old".into(), "new".into()];
        app.suggest_transfer_pairs();
        assert_eq!(app.revision_transfer.unmatched.len(), 2);
        assert!(app.revision_transfer.error.is_some());
        assert!(app.revision_transfer.token.is_none());
        assert!(app.revision_transfer.inspection.is_none());
        assert!(!app.revision_transfer.can_prepare());
    }

    #[test]
    fn manifest_entry_reports_refusal_and_clears_previous_inspection_when_opened() {
        let (mut app, requests, _) = test_app();
        app.plan_sets.open = true;
        app.revision_transfer = state();
        app.revision_transfer.open = false;
        app.saving = true;
        if app.open_revision_transfer_for_set(manifest(), "set.json".into()) {
            app.plan_sets.open = false;
        }
        assert!(app.plan_sets.open && !app.revision_transfer.open);
        assert!(requests.try_iter().next().is_none());
        app.saving = false;
        app.revision_transfer.selected_pair = 1;
        app.revision_transfer.passwords[0] = Zeroizing::new("temporary-test".into());
        assert!(app.open_revision_transfer_for_set(manifest(), "set.json".into()));
        let state = &app.revision_transfer;
        assert!(state.open && state.inspection.is_none());
        assert_eq!(state.selected_pair, 0);
        assert_eq!(state.pairs.len(), 1);
        assert!(state.paths.iter().all(Option::is_none));
        assert!(state.passwords.iter().all(|p| p.is_empty()));
        assert!(!state.can_prepare() && !state.can_save());
    }

    #[test]
    fn queued_reviews_survive_current_draft_invalidation_and_inspection() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer
            .batch
            .entries
            .push(queued("first", "first-copy.pdf"));
        app.revision_transfer.token = Some(17);
        app.revision_transfer.reviewed.extend([1, 2]);
        app.inspect_revision_transfer();
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer {
                generation,
                job: Job::Inspect { .. },
            } => generation,
            _ => panic!("inspect the next pair"),
        };
        assert_eq!(app.revision_transfer.batch.entries[0].id, "first");
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::Inspected {
                paths: ["second-old.pdf".into(), "second-new.pdf".into()],
                inspection: inspection(),
            }),
        );
        assert_eq!(app.revision_transfer.batch.entries.len(), 1);
        app.discard_revision_transfer();
        assert!(matches!(
            requests.recv().unwrap(),
            Request::RevisionTransfer {
                job: Job::Discard,
                ..
            }
        ));
        assert_eq!(
            app.revision_transfer.batch.entries[0].output,
            PathBuf::from("first-copy.pdf")
        );
    }

    fn queued(id: &str, output: &str) -> TransferBatchEntrySummary {
        TransferBatchEntrySummary {
            id: id.into(),
            old_path: "queued-old.pdf".into(),
            new_path: "queued-new.pdf".into(),
            output: output.into(),
            candidate_bytes: 100,
            report: report(),
        }
    }

    #[test]
    fn queue_requires_every_review_and_cancellation_preserves_the_queue_acknowledgement() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer.token = Some(17);
        app.revision_transfer.reviewed.insert(1);
        app.queue_revision_transfer("queued.pdf".into());
        assert!(requests.try_iter().next().is_none());
        app.revision_transfer.reviewed.insert(2);
        app.queue_revision_transfer("queued.pdf".into());
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer {
                generation,
                job: Job::QueueCurrent {
                    token, reviewed, ..
                },
            } => {
                assert_eq!(token, 17);
                assert!(reviewed.contains(&1) && reviewed.contains(&2));
                generation
            }
            _ => panic!("queue the reviewed candidate"),
        };
        app.revision_transfer.cancel_pending();
        assert_eq!(
            app.revision_transfer.pending,
            Some((generation, Pending::Queue))
        );
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::BatchChanged(vec![queued("first", "queued.pdf")])),
        );
        assert_eq!(app.revision_transfer.batch.entries.len(), 1);
        assert!(app.revision_transfer.token.is_none());
        assert!(app.revision_transfer.needs_close_guard());
        app.close_revision_transfer();
        assert!(app.revision_transfer.open);
        assert!(app.revision_transfer.batch.confirm_discard);
        assert!(
            requests.try_iter().next().is_none(),
            "close must not silently discard queued candidates"
        );
        app.apply_transfer_batch_action(batch::BatchActions {
            discard_and_close: true,
            ..Default::default()
        });
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer {
                generation,
                job: Job::DiscardAll,
            } => generation,
            _ => panic!("explicit queue discard"),
        };
        assert!(
            app.revision_transfer.needs_close_guard(),
            "wait for the discard acknowledgement"
        );
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::Discarded),
        );
        assert!(!app.revision_transfer.open && !app.revision_transfer.needs_close_guard());
    }

    #[test]
    fn refused_queue_output_keeps_the_current_review_available_for_another_name() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer.token = Some(17);
        app.revision_transfer.reviewed.extend([1, 2]);
        app.queue_revision_transfer("existing.pdf".into());
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer { generation, .. } => generation,
            _ => panic!("queue operation"),
        };
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Err(EngineError::AlreadyExists("existing.pdf".into())),
        );
        assert!(app.revision_transfer.can_queue());
        assert_eq!(app.revision_transfer.token, Some(17));
        assert!(
            requests.try_iter().next().is_none(),
            "do not discard the worker candidate after a queue refusal"
        );
    }

    #[test]
    fn dirty_any_queued_source_blocks_batch_even_when_current_pair_is_unrelated() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        let mut entry = queued("first", "queued.pdf");
        entry.old_path = app.document.as_ref().unwrap().info.path.clone();
        app.revision_transfer.batch.entries.push(entry);
        app.dirty = true;
        app.can_undo = true;
        app.apply_transfer_batch_action(batch::BatchActions {
            save: true,
            ..Default::default()
        });
        assert!(requests.try_iter().next().is_none());
        assert!(app.revision_transfer.error.is_some());
        assert!(app.dirty && app.can_undo);
        app.dirty = false;
        app.revision_transfer.batch.entries[0].old_path = "inactive.pdf".into();
        app.tabs.tabs.push(crate::tabs::Tab {
            path: "inactive.pdf".into(),
            dirty: true,
            recovery_id: 99,
            ui: None,
            doc: None,
        });
        app.apply_transfer_batch_action(batch::BatchActions {
            save: true,
            ..Default::default()
        });
        assert!(
            requests.try_iter().next().is_none(),
            "inactive dirty input must also block publication"
        );
        assert_eq!(app.revision_transfer.batch.entries.len(), 1);
    }

    #[test]
    fn cancel_during_batch_publication_keeps_committed_and_unattempted_results() {
        use sunipdf_core::revision_transfer::batch::{
            TransferBatchCompleted, TransferBatchFailure, TransferBatchStatus,
        };
        let (mut app, _, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer
            .batch
            .entries
            .push(queued("first", "first.pdf"));
        app.revision_transfer
            .batch
            .entries
            .push(queued("second", "second.pdf"));
        let generation = app.revision_transfer.begin(Pending::SaveBatch);
        assert!(app.guard_revision_transfer_quit());
        assert_eq!(
            app.revision_transfer.pending,
            Some((generation, Pending::SaveBatch))
        );
        let mut saved = report();
        saved.output_path = Some("first.pdf".into());
        saved.output_sha256 = Some("d".repeat(64));
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::BatchSaved(TransferBatchOutcome {
                version: 1,
                status: TransferBatchStatus::Cancelled,
                completed: vec![TransferBatchCompleted {
                    id: "first".into(),
                    report: saved,
                }],
                failure: Some(TransferBatchFailure {
                    id: Some("second".into()),
                    reason: "Cancelled before the next copy".into(),
                }),
                remaining: vec![queued("second", "second.pdf")],
            })),
        );
        assert!(app.revision_transfer.open && !app.revision_transfer.busy());
        assert!(app.revision_transfer.batch.entries.is_empty());
        let result = app.revision_transfer.batch.outcome.as_ref().unwrap();
        assert_eq!(result.completed.len(), 1);
        assert_eq!(
            result.completed[0].report.output_path.as_deref(),
            Some(Path::new("first.pdf"))
        );
        assert_eq!(result.remaining[0].output, PathBuf::from("second.pdf"));
        app.revision_transfer.invalidate(true);
        assert!(
            app.revision_transfer.batch.outcome.is_some(),
            "current-draft changes must retain the batch result"
        );
    }

    #[test]
    fn queue_remove_and_clear_wait_for_matching_worker_acknowledgements() {
        let (mut app, requests, _) = test_app();
        app.revision_transfer = state();
        app.revision_transfer.batch.entries =
            vec![queued("first", "first.pdf"), queued("second", "second.pdf")];
        app.apply_transfer_batch_action(batch::BatchActions {
            remove: Some("first".into()),
            ..Default::default()
        });
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer {
                generation,
                job: Job::RemoveQueued { id },
            } => {
                assert_eq!(id, "first");
                generation
            }
            _ => panic!("remove queued item"),
        };
        assert_eq!(app.revision_transfer.batch.entries.len(), 2);
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::BatchChanged(vec![queued("second", "second.pdf")])),
        );
        app.apply_transfer_batch_action(batch::BatchActions {
            clear: true,
            ..Default::default()
        });
        let generation = match requests.recv().unwrap() {
            Request::RevisionTransfer {
                generation,
                job: Job::ClearBatch,
            } => generation,
            _ => panic!("clear queue"),
        };
        app.revision_transfer.cancel_pending();
        assert!(app.revision_transfer.busy());
        app.on_revision_transfer(
            &egui::Context::default(),
            generation,
            Ok(Outcome::BatchChanged(vec![])),
        );
        assert!(app.revision_transfer.batch.entries.is_empty());
    }

    #[test]
    fn native_window_close_and_close_document_preserve_queued_work_and_dirty_history() {
        for dirty in [false, true] {
            let (mut app, requests, _) = test_app();
            app.revision_transfer = state();
            app.revision_transfer.open = false;
            app.revision_transfer
                .batch
                .entries
                .push(queued("first", "first-copy.pdf"));
            app.dirty = dirty;
            app.can_undo = dirty;
            let ctx = egui::Context::default();
            app.has_icons = crate::icons::install(&ctx);
            crate::theme::apply(&ctx, app.settings.dark_theme);
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 780.0),
                )),
                ..Default::default()
            };
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .events
                .push(egui::ViewportEvent::Close);
            let mut frame = eframe::Frame::_new_kittest();
            let mut output = ctx.run_ui(input, |ui| eframe::App::ui(&mut app, ui, &mut frame));
            output.textures_delta.clear();
            assert!(output
                .viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
            assert!(!output
                .viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::Close)));
            assert!(app.revision_transfer.open && app.revision_transfer.batch.confirm_discard);
            assert_eq!(app.revision_transfer.batch.entries.len(), 1);
            assert_eq!(app.dirty, dirty);
            assert_eq!(app.can_undo, dirty);
            app.execute(&ctx, sunipdf_core::Command::CloseDocument);
            assert!(app.document.is_some());
            assert_eq!(app.revision_transfer.batch.entries.len(), 1);
            assert!(!requests.try_iter().any(|request| matches!(
                request,
                Request::Close
                    | Request::RevisionTransfer {
                        job: Job::DiscardAll | Job::ClearBatch,
                        ..
                    }
            )));
        }
    }

    #[test]
    fn narrow_bilingual_queue_dialog_keeps_close_and_review_actions_inside_window() {
        for language in Language::ALL {
            let (mut app, _, _) = test_app();
            app.settings.set_language(language);
            app.revision_transfer = state();
            app.revision_transfer
                .batch
                .entries
                .push(queued("first", "first-copy.pdf"));
            app.revision_transfer.batch.confirm_discard = true;
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(560.0, 680.0),
                        )),
                        time: Some(frame as f64 * 0.025),
                        ..Default::default()
                    },
                    |_| app.revision_transfer_dialog(&ctx),
                );
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in [
                    "transfer.save_short",
                    "transfer.preview_short",
                    "transfer.inspect_short",
                    "dialog.close",
                    "transfer.batch.discard_close",
                    "transfer.batch.keep",
                ] {
                    let button = tree
                        .nodes
                        .iter()
                        .map(|(_, node)| node)
                        .find(|node| node.label() == Some(tr(language, key)))
                        .unwrap_or_else(|| {
                            panic!("{key} must be reachable at 560 px in {}", language.code())
                        });
                    let bounds = button.bounds().expect("button bounds");
                    assert!(
                        bounds.x0 >= 0.0
                            && bounds.x1 <= 560.0
                            && bounds.y0 >= 0.0
                            && bounds.y1 <= 680.0,
                        "{key} must remain inside the window: {bounds:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn bilingual_dialog_exposes_explicit_inspection_and_no_save_before_preview() {
        for language in Language::ALL {
            let (mut app, _, _) = test_app();
            app.settings.set_language(language);
            app.revision_transfer = state();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(760.0, 620.0),
                        )),
                        time: Some(frame as f64 * 0.025),
                        ..Default::default()
                    },
                    |_| app.revision_transfer_dialog(&ctx),
                );
                // Headless rendering must consume textures even if an assertion panics.
                output.textures_delta.clear();
                if frame < 2 {
                    // Modal areas need their sizing pass before exposing the final footer.
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in [
                    "transfer.save",
                    "transfer.preview",
                    "transfer.inspect",
                    "dialog.close",
                ] {
                    let button = tree
                        .nodes
                        .iter()
                        .map(|(_, node)| node)
                        .find(|node| node.label() == Some(tr(language, key)))
                        .unwrap_or_else(|| {
                            panic!(
                                "{key} must be accessible in {}: {:?}",
                                language.code(),
                                tree.nodes
                                    .iter()
                                    .filter_map(|(_, n)| n.label())
                                    .collect::<Vec<_>>()
                            )
                        });
                    let bounds = button.bounds().expect("footer bounds");
                    assert!(
                        bounds.x0 >= 0.0
                            && bounds.x1 <= 760.0
                            && bounds.y0 >= 0.0
                            && bounds.y1 <= 620.0,
                        "{key} footer must remain inside the window: {bounds:?}"
                    );
                    if key == "transfer.save" {
                        assert!(button.is_disabled(), "saving requires a reviewed candidate");
                    }
                }
            }
        }
    }
}
