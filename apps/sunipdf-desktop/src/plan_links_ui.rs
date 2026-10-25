//! Explicitly reviewed links between saved, indexed sheets.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    plan_links::{
        self as core, PlanLinkCredentials, PlanLinkInspection, PlanLinkReceipt, PlanLinkReport,
        PlanLinkReview, PlanLinkScanRequest, PlanLinkSelection, PreparedPlanLinks,
        RemotePageTarget,
    },
    plan_set::{self, PlanSet},
    Engine, EngineError, RenderedPage,
};
use sunipdf_i18n::{tr, Language};
use zeroize::Zeroizing;

mod batch;
use batch::{BatchCached, BatchState};

pub type Credentials = Vec<(String, Zeroizing<String>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub path: PathBuf,
    pub set: PlanSet,
}
pub struct Preview {
    pub image: RenderedPage,
    pub points: (f32, f32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePreviewBinding {
    context: Context,
    source: PathBuf,
    target: RemotePageTarget,
    source_epoch: u64,
}
struct RemotePreviewReceipt {
    binding: RemotePreviewBinding,
    source_sha256: String,
}
pub enum Job {
    Load {
        path: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Inspect {
        context: Context,
        scan: PlanLinkScanRequest,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    Prepare {
        context: Context,
        review: PlanLinkReview,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    Preview {
        token: u64,
        page: usize,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    Target {
        token: u64,
        index: usize,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    Save {
        token: u64,
        output: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Navigate {
        context: Context,
        source: PathBuf,
        target: RemotePageTarget,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    RemotePreview {
        binding: RemotePreviewBinding,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    NavigatePreviewed {
        binding: RemotePreviewBinding,
        source_sha256: String,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    BatchInspect {
        context: Context,
        scan: core::PlanLinkBatchScanRequest,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    BatchPrepare {
        context: Context,
        review: core::PlanLinkBatchReview,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    BatchPreview {
        token: u64,
        file_index: usize,
        page: usize,
        target_sheet_id: Option<String>,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    BatchSave {
        token: u64,
        credentials: Credentials,
        cancel: Arc<AtomicBool>,
    },
    Clear,
}
pub enum Outcome {
    Loaded(Context),
    Inspected(PlanLinkInspection),
    Prepared {
        token: u64,
        report: PlanLinkReport,
        preview: Preview,
    },
    Preview {
        token: u64,
        page: usize,
        target: bool,
        preview: Preview,
    },
    Saved(PlanLinkReceipt),
    Navigate(crate::plan_sets::Navigation),
    RemotePreview {
        binding: RemotePreviewBinding,
        source_sha256: String,
        preview: Preview,
    },
    BatchInspected(core::PlanLinkBatchInspection),
    BatchPrepared {
        token: u64,
        files: Vec<core::PlanLinkBatchFilePlan>,
        preview: Preview,
    },
    BatchPreview {
        token: u64,
        source_file_id: String,
        page: usize,
        target_sheet_id: Option<String>,
        preview: Preview,
    },
    BatchSaved(core::PlanLinkBatchOutcome),
    Cleared,
}
struct Cached {
    token: u64,
    prepared: PreparedPlanLinks,
    context: Context,
}
#[derive(Default)]
pub struct WorkerCache {
    next: u64,
    candidate: Option<Cached>,
    batch_candidate: Option<BatchCached>,
}

fn error(message: &str) -> EngineError {
    EngineError::Other(format!("Plan links: {message}"))
}
fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}
fn borrowed(credentials: &Credentials) -> Vec<(&str, &str)> {
    credentials
        .iter()
        .map(|(id, secret)| (id.as_str(), secret.as_str()))
        .collect()
}
fn password<'a>(credentials: &'a Credentials, id: &str) -> Option<&'a str> {
    credentials
        .iter()
        .find_map(|(key, value)| (key == id).then_some(value.as_str()))
}
fn source_fingerprint(path: &std::path::Path, cancel: &AtomicBool) -> Result<String, EngineError> {
    let mut file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > core::MAX_CANDIDATE_BYTES as u64 {
        return Err(error("source PDF exceeds the preview limit"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_usize;
    loop {
        cancelled(cancel)?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read;
        if total > core::MAX_CANDIDATE_BYTES {
            return Err(error("source PDF exceeds the preview limit"));
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn unchanged_source(
    path: &std::path::Path,
    expected: &str,
    cancel: &AtomicBool,
) -> Result<(), EngineError> {
    if source_fingerprint(path, cancel)? != expected {
        return Err(error(
            "source PDF changed since the preview; preview it again",
        ));
    }
    Ok(())
}
fn remote_preview(
    engine: &Engine,
    binding: RemotePreviewBinding,
    credentials: Credentials,
    cancel: &AtomicBool,
) -> Result<Outcome, EngineError> {
    let values = borrowed(&credentials);
    let access = PlanLinkCredentials { passwords: &values };
    let verified = core::resolve_verified_remote_link(
        engine,
        &binding.context.path,
        &binding.context.set,
        &binding.source,
        &binding.target,
        &access,
        cancel,
    )?;
    let source_sha256 = source_fingerprint(&binding.source, cancel)?;
    let sheet = binding
        .context
        .set
        .sheets
        .iter()
        .find(|s| s.id == binding.target.sheet_id)
        .ok_or_else(|| error("destination disappeared"))?;
    let doc = engine.open_bytes(
        verified.bytes,
        password(&credentials, &sheet.file_id),
        &verified.path,
    )?;
    let page = verified.page - 1;
    let size = doc.page_size(page)?;
    if !size.width_pt.is_finite()
        || !size.height_pt.is_finite()
        || size.width_pt <= 0.0
        || size.height_pt <= 0.0
    {
        return Err(EngineError::RenderTooLarge);
    }
    cancelled(cancel)?;
    let image = doc.render_page(page, 1200.0 / size.width_pt.max(size.height_pt))?;
    if image.width > core::MAX_PREVIEW_SIDE
        || image.height > core::MAX_PREVIEW_SIDE
        || image.rgba.len() > core::MAX_PREVIEW_SIDE as usize * core::MAX_PREVIEW_SIDE as usize * 4
    {
        return Err(EngineError::RenderTooLarge);
    }
    // Recheck after rendering: the image must still describe the selected saved destination.
    let current = core::resolve_verified_remote_link(
        engine,
        &binding.context.path,
        &binding.context.set,
        &binding.source,
        &binding.target,
        &access,
        cancel,
    )?;
    if current.path != verified.path
        || current.page != verified.page
        || current.sha256 != verified.sha256
    {
        return Err(error(
            "destination changed during preview; preview it again",
        ));
    }
    unchanged_source(&binding.source, &source_sha256, cancel)?;
    Ok(Outcome::RemotePreview {
        binding,
        source_sha256,
        preview: Preview {
            image,
            points: (size.width_pt, size.height_pt),
        },
    })
}
fn source_path(context: &Context, file_id: &str) -> Result<PathBuf, EngineError> {
    let file = context
        .set
        .files
        .iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| error("source is unavailable"))?;
    Ok(context
        .path
        .parent()
        .ok_or_else(|| error("manifest parent is unavailable"))?
        .join(&file.path))
}
fn candidate_preview(
    engine: &Engine,
    cached: &Cached,
    page: usize,
    credentials: &Credentials,
    cancel: &AtomicBool,
) -> Result<Preview, EngineError> {
    let id = &cached.prepared.report().inspection.source_file_id;
    let secret = password(credentials, id);
    let image = cached
        .prepared
        .render_preview(engine, page, secret, 1200, cancel)?;
    let sheet = cached
        .context
        .set
        .sheets
        .iter()
        .find(|s| &s.file_id == id)
        .ok_or_else(|| error("source sheet is unavailable"))?;
    let verified = plan_set::read_sheet(&cached.context.path, &cached.context.set, &sheet.id)?;
    let doc = engine.open_bytes(verified.bytes, secret, &verified.path)?;
    let size = doc.page_size(page - 1)?;
    cached.prepared.revalidate_sources(cancel)?;
    Ok(Preview {
        image,
        points: (size.width_pt, size.height_pt),
    })
}
impl WorkerCache {
    fn get(&self, token: u64) -> Result<&Cached, EngineError> {
        self.candidate
            .as_ref()
            .filter(|c| c.token == token)
            .ok_or_else(|| error("preview expired; prepare it again"))
    }
    pub fn run(
        &mut self,
        engine: &Engine,
        job: Job,
        on_progress: impl FnMut(&core::PlanLinkBatchProgress),
    ) -> Result<Outcome, EngineError> {
        if matches!(
            job,
            Job::BatchInspect { .. }
                | Job::BatchPrepare { .. }
                | Job::BatchPreview { .. }
                | Job::BatchSave { .. }
        ) {
            self.candidate = None;
            return self.run_batch(engine, job, on_progress);
        }
        self.batch_candidate = None;
        match job {
            Job::Clear => {
                self.candidate = None;
                Ok(Outcome::Cleared)
            }
            Job::Load { path, cancel } => {
                self.candidate = None;
                cancelled(&cancel)?;
                let set = plan_set::load(&path)?;
                cancelled(&cancel)?;
                Ok(Outcome::Loaded(Context { path, set }))
            }
            Job::Inspect {
                context,
                scan,
                credentials,
                cancel,
            } => {
                self.candidate = None;
                let values = borrowed(&credentials);
                Ok(Outcome::Inspected(core::inspect_plan_links(
                    engine,
                    &context.path,
                    &context.set,
                    &scan,
                    &PlanLinkCredentials { passwords: &values },
                    &cancel,
                )?))
            }
            Job::Prepare {
                context,
                review,
                credentials,
                cancel,
            } => {
                self.candidate = None;
                let values = borrowed(&credentials);
                let prepared = core::prepare_plan_links(
                    engine,
                    &context.path,
                    &context.set,
                    &review,
                    &PlanLinkCredentials { passwords: &values },
                    &cancel,
                )?;
                self.next = self.next.wrapping_add(1);
                let cached = Cached {
                    token: self.next,
                    prepared,
                    context,
                };
                let page = cached
                    .prepared
                    .report()
                    .created
                    .first()
                    .ok_or_else(|| error("no approved links"))?
                    .page;
                let preview = candidate_preview(engine, &cached, page, &credentials, &cancel)?;
                let outcome = Outcome::Prepared {
                    token: cached.token,
                    report: cached.prepared.report().clone(),
                    preview,
                };
                self.candidate = Some(cached);
                Ok(outcome)
            }
            Job::Preview {
                token,
                page,
                credentials,
                cancel,
            } => {
                let cached = self.get(token)?;
                let preview = candidate_preview(engine, cached, page, &credentials, &cancel)?;
                Ok(Outcome::Preview {
                    token,
                    page,
                    target: false,
                    preview,
                })
            }
            Job::Target {
                token,
                index,
                credentials,
                cancel,
            } => {
                let cached = self.get(token)?;
                let link = cached
                    .prepared
                    .report()
                    .created
                    .get(index)
                    .ok_or_else(|| error("link selection changed"))?;
                let sheet = cached
                    .context
                    .set
                    .sheets
                    .iter()
                    .find(|s| s.id == link.target.sheet_id)
                    .ok_or_else(|| error("destination disappeared"))?;
                let image = cached.prepared.render_target_preview(
                    engine,
                    &sheet.id,
                    password(&credentials, &sheet.file_id),
                    1200,
                    &cancel,
                )?;
                Ok(Outcome::Preview {
                    token,
                    page: link.target.page + 1,
                    target: true,
                    preview: Preview {
                        image,
                        points: (1.0, 1.0),
                    },
                })
            }
            Job::Save {
                token,
                output,
                cancel,
            } => {
                self.get(token)?;
                let cached = self
                    .candidate
                    .take()
                    .ok_or_else(|| error("preview expired"))?;
                Ok(Outcome::Saved(core::publish_plan_links(
                    cached.prepared,
                    &output,
                    &cancel,
                )?))
            }
            Job::Navigate {
                context,
                source,
                target,
                credentials,
                cancel,
            } => {
                self.candidate = None;
                let values = borrowed(&credentials);
                let verified = core::resolve_verified_remote_link(
                    engine,
                    &context.path,
                    &context.set,
                    &source,
                    &target,
                    &PlanLinkCredentials { passwords: &values },
                    &cancel,
                )?;
                let sheet = context
                    .set
                    .sheets
                    .iter()
                    .find(|s| s.id == target.sheet_id)
                    .ok_or_else(|| error("destination disappeared"))?;
                let secret =
                    password(&credentials, &sheet.file_id).map(|p| Zeroizing::new(p.to_owned()));
                let doc = engine.open_bytes(
                    verified.bytes.clone(),
                    secret.as_ref().map(|p| p.as_str()),
                    &verified.path,
                )?;
                cancelled(&cancel)?;
                Ok(Outcome::Navigate(crate::plan_sets::Navigation {
                    sheet: verified,
                    password: secret,
                    page_count: doc.page_count(),
                    active_source: None,
                }))
            }
            Job::RemotePreview {
                binding,
                credentials,
                cancel,
            } => {
                self.candidate = None;
                remote_preview(engine, binding, credentials, &cancel)
            }
            Job::NavigatePreviewed {
                binding,
                source_sha256,
                credentials,
                cancel,
            } => {
                unchanged_source(&binding.source, &source_sha256, &cancel)?;
                let outcome = self.run(
                    engine,
                    Job::Navigate {
                        context: binding.context,
                        source: binding.source.clone(),
                        target: binding.target,
                        credentials,
                        cancel: cancel.clone(),
                    },
                    on_progress,
                )?;
                unchanged_source(&binding.source, &source_sha256, &cancel)?;
                Ok(outcome)
            }
            Job::BatchInspect { .. }
            | Job::BatchPrepare { .. }
            | Job::BatchPreview { .. }
            | Job::BatchSave { .. } => Err(error("invalid batch dispatch")),
        }
    }
}

struct Picture {
    texture: egui::TextureHandle,
    points: (f32, f32),
    page: usize,
    target: bool,
}
#[derive(Default)]
pub struct PlanLinksState {
    pub open: bool,
    batch_mode: bool,
    batch: BatchState,
    context: Option<Context>,
    manifest_input: String,
    focus_manifest: bool,
    source_id: String,
    pages: String,
    targets: HashSet<String>,
    passwords: Credentials,
    saved_only: bool,
    inspection: Option<PlanLinkInspection>,
    selections: BTreeMap<String, String>,
    candidate: Option<(u64, PlanLinkReport)>,
    picture: Option<Picture>,
    preview_page: usize,
    target_index: usize,
    error: Option<String>,
    message: Option<String>,
    generation: u64,
    pending: Option<(u64, bool)>,
    cancel: Arc<AtomicBool>,
    remote: Option<(PathBuf, RemotePageTarget)>,
    remote_epoch: Option<u64>,
    remote_preview: Option<RemotePreviewReceipt>,
    remote_request: Option<RemotePreviewBinding>,
}
impl PlanLinksState {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn clear_preview(&mut self) {
        self.batch.clear_preview();
        self.candidate = None;
        self.picture = None;
        self.remote_preview = None;
    }
    fn reset_scan(&mut self) {
        self.batch.reset_scan();
        self.inspection = None;
        self.selections.clear();
        self.clear_preview();
        self.remote_request = None;
    }
    fn review(&self) -> Option<PlanLinkReview> {
        let inspection = self.inspection.as_ref()?;
        let scan = self.scan().ok()?;
        let selections = inspection
            .occurrences
            .iter()
            .filter_map(|o| {
                self.selections.get(&o.id).map(|target| PlanLinkSelection {
                    occurrence_id: o.id.clone(),
                    fingerprint: o.fingerprint.clone(),
                    target_sheet_id: target.clone(),
                    confirmed: true,
                })
            })
            .collect::<Vec<_>>();
        (!selections.is_empty()).then(|| PlanLinkReview {
            version: 1,
            manifest_sha256: inspection.manifest_sha256.clone(),
            source_sha256: inspection.source_sha256.clone(),
            scan,
            selections,
        })
    }
    fn scan(&self) -> Result<PlanLinkScanRequest, ()> {
        let mut seen = HashSet::new();
        let pages = self
            .pages
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|p| *p > 0 && seen.insert(*p))
                    .ok_or(())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if pages.len() > core::MAX_SCAN_PAGES || self.targets.is_empty() {
            return Err(());
        }
        let context = self.context.as_ref().ok_or(())?;
        Ok(PlanLinkScanRequest {
            source_file_id: self.source_id.clone(),
            pages,
            target_sheet_ids: context
                .set
                .sheets
                .iter()
                .filter(|s| self.targets.contains(&s.id))
                .map(|s| s.id.clone())
                .collect(),
        })
    }
}

enum Action {
    Open,
    LoadManifest,
    Inspect,
    Reset,
    Prepare,
    SourcePreview,
    TargetPreview,
    Save,
    Navigate,
    RemotePreview,
}
fn manifest_input_path(input: &str) -> Result<PathBuf, EngineError> {
    let path = PathBuf::from(input.trim());
    if input.len() > 4096
        || input.chars().any(char::is_control)
        || !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(error("enter an absolute local JSON manifest path"));
    }
    Ok(path)
}
fn manifest_selector(
    ui: &mut egui::Ui,
    state: &mut PlanLinksState,
    lang: Language,
    ready: bool,
) -> Option<Action> {
    let mut action = None;
    let label = ui.label(tr(lang, "plan_links.manifest_path"));
    ui.add_enabled_ui(ready, |ui| {
        ui.horizontal_wrapped(|ui| {
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut state.manifest_input)
                        .id_salt("plan-links-manifest-path")
                        .hint_text(tr(lang, "plan_links.manifest_placeholder"))
                        .char_limit(4096)
                        .desired_width((ui.available_width() - 120.0).clamp(160.0, 520.0)),
                )
                .labelled_by(label.id);
            if state.focus_manifest {
                response.request_focus();
                state.focus_manifest = false;
            }
            if response.changed() {
                state.reset_scan();
                state.context = None;
                state.passwords.clear();
                state.error = None;
                state.message = None;
            }
            if ui.button(tr(lang, "plan_links.load_manifest")).clicked()
                || ((response.has_focus() || response.lost_focus())
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                action = Some(Action::LoadManifest);
            }
            if ui.button(tr(lang, "plan_links.open_manifest")).clicked() {
                action = Some(Action::Open);
            }
        });
    });
    action
}
impl SuniApp {
    pub(crate) fn open_plan_links(&mut self) {
        if self.navigation_blocked() || self.worker.has_pending_mutations() {
            return;
        }
        self.plan_links.open = true;
        self.plan_links.focus_manifest = self.plan_links.context.is_none();
        if self.plan_links.pages.is_empty() {
            self.plan_links.pages = "1".into();
        }
    }
    pub(crate) fn confirm_remote_plan_link(&mut self, target: RemotePageTarget) {
        if self.navigation_blocked() || self.worker.has_pending_mutations() {
            return;
        }
        let Some(source) = self.document.as_ref().map(|d| d.info.path.clone()) else {
            return;
        };
        self.close_plan_links();
        self.plan_links.remote = Some((source, target));
        self.plan_links.remote_epoch = Some(self.view.epoch);
        self.plan_links.open = true;
        self.plan_links.focus_manifest = self.plan_links.context.is_none();
    }
    fn remote_plan_link_binding(&self) -> Option<RemotePreviewBinding> {
        let (source, target) = self.plan_links.remote.as_ref()?;
        if !self.plan_links.open
            || self.document.as_ref()?.info.path != *source
            || self.plan_links.remote_epoch != Some(self.view.epoch)
        {
            return None;
        }
        Some(RemotePreviewBinding {
            context: self.plan_links.context.clone()?,
            source: source.clone(),
            target: target.clone(),
            source_epoch: self.view.epoch,
        })
    }
    fn matches_remote_plan_link(&self, binding: &RemotePreviewBinding) -> bool {
        self.plan_links.open
            && self.view.epoch == binding.source_epoch
            && self.plan_links.remote_epoch == Some(binding.source_epoch)
            && self
                .document
                .as_ref()
                .is_some_and(|document| document.info.path == binding.source)
            && self.plan_links.context.as_ref() == Some(&binding.context)
            && self
                .plan_links
                .remote
                .as_ref()
                .is_some_and(|(source, target)| {
                    source == &binding.source && target == &binding.target
                })
    }
    fn start_remote_plan_link(&mut self, preview: bool) {
        if self.plan_links.busy() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        let Some(binding) = self.remote_plan_link_binding() else {
            self.plan_links.clear_preview();
            self.plan_links.error = Some(tr(self.lang(), "plan_links.remote_changed").into());
            return;
        };
        let expected_source = self
            .plan_links
            .remote_preview
            .as_ref()
            .filter(|receipt| receipt.binding == binding)
            .map(|receipt| receipt.source_sha256.clone());
        self.plan_links.clear_preview();
        let request_binding = binding.clone();
        self.start_plan_links(false, |cancel, credentials| {
            if preview {
                Job::RemotePreview {
                    binding,
                    credentials,
                    cancel,
                }
            } else if let Some(source_sha256) = expected_source {
                Job::NavigatePreviewed {
                    binding,
                    source_sha256,
                    credentials,
                    cancel,
                }
            } else {
                Job::Navigate {
                    context: binding.context,
                    source: binding.source,
                    target: binding.target,
                    credentials,
                    cancel,
                }
            }
        });
        self.plan_links.remote_request = Some(request_binding);
        for (_, secret) in &mut self.plan_links.passwords {
            *secret = Zeroizing::new(String::new());
        }
    }
    fn close_plan_links(&mut self) {
        self.plan_links.cancel.store(true, Ordering::Relaxed);
        for (_, secret) in &mut self.plan_links.passwords {
            *secret = Zeroizing::new(String::new());
        }
        if self.plan_links.pending.is_some_and(|(_, save)| save) {
            return;
        }
        self.plan_links.generation = self.plan_links.generation.wrapping_add(1);
        self.plan_links.pending = None;
        self.plan_links.open = false;
        self.plan_links.remote = None;
        self.plan_links.remote_epoch = None;
        self.plan_links.reset_scan();
        self.worker.send(Request::PlanLinks {
            generation: self.plan_links.generation,
            job: Job::Clear,
        });
    }
    pub(crate) fn guard_plan_links_quit(&mut self) -> bool {
        if !self.plan_links.open && !self.plan_links.busy() {
            return false;
        }
        self.close_plan_links();
        self.plan_links.busy()
    }
    fn start_plan_links(
        &mut self,
        saving: bool,
        build: impl FnOnce(Arc<AtomicBool>, Credentials) -> Job,
    ) {
        if self.plan_links.busy() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        self.plan_links.generation = self.plan_links.generation.wrapping_add(1);
        self.plan_links.cancel = Arc::new(AtomicBool::new(false));
        self.plan_links.pending = Some((self.plan_links.generation, saving));
        self.plan_links.remote_request = None;
        self.plan_links.error = None;
        self.plan_links.message = None;
        let credentials = self
            .plan_links
            .passwords
            .iter()
            .filter(|(_, p)| !p.is_empty())
            .cloned()
            .collect();
        let job = build(self.plan_links.cancel.clone(), credentials);
        self.worker.send(Request::PlanLinks {
            generation: self.plan_links.generation,
            job,
        });
    }
    pub(crate) fn on_plan_links(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if self.plan_links.pending.map(|p| p.0) != Some(generation) {
            return;
        }
        self.plan_links.pending = None;
        let lang = self.lang();
        if let Some(binding) = self.plan_links.remote_request.take() {
            if self.plan_links.cancel.load(Ordering::Relaxed)
                || !self.matches_remote_plan_link(&binding)
            {
                self.plan_links.clear_preview();
                self.plan_links.message = Some(
                    tr(
                        lang,
                        if self.plan_links.cancel.load(Ordering::Relaxed) {
                            "status.cancelled"
                        } else {
                            "plan_links.remote_changed"
                        },
                    )
                    .into(),
                );
                return;
            }
        }
        match result {
            Ok(Outcome::Loaded(context)) => {
                self.plan_links.reset_scan();
                self.plan_links.targets.clear();
                self.plan_links.saved_only = false;
                self.plan_links.source_id = context
                    .set
                    .files
                    .first()
                    .map_or_else(String::new, |f| f.id.clone());
                self.plan_links.passwords = context
                    .set
                    .files
                    .iter()
                    .map(|f| (f.id.clone(), Zeroizing::new(String::new())))
                    .collect();
                self.plan_links.batch.load(&context);
                self.plan_links.manifest_input = context.path.display().to_string();
                self.plan_links.context = Some(context);
                self.plan_links.pages = "1".into();
            }
            Ok(Outcome::Inspected(report)) => {
                self.plan_links.selections.clear();
                self.plan_links.clear_preview();
                self.plan_links.inspection = Some(report);
            }
            Ok(Outcome::Prepared {
                token,
                report,
                preview,
            }) => {
                let page = report.created.first().map_or(1, |c| c.page);
                self.plan_links.preview_page = page;
                self.plan_links.target_index = 0;
                self.plan_links.candidate = Some((token, report));
                self.set_plan_link_picture(ctx, page, false, preview);
            }
            Ok(Outcome::Preview {
                token,
                page,
                target,
                preview,
            }) => {
                if self
                    .plan_links
                    .candidate
                    .as_ref()
                    .is_some_and(|(id, _)| *id == token)
                {
                    self.set_plan_link_picture(ctx, page, target, preview);
                }
            }
            Ok(Outcome::Saved(receipt)) => {
                self.plan_links.clear_preview();
                self.plan_links.message = Some(format!(
                    "{} {}",
                    tr(lang, "plan_links.saved"),
                    receipt.output.display()
                ));
            }
            Ok(Outcome::Navigate(navigation)) => {
                self.close_plan_links();
                self.navigate_plan_sheet(navigation);
            }
            Ok(Outcome::RemotePreview {
                binding,
                source_sha256,
                preview,
            }) => {
                if self.matches_remote_plan_link(&binding) {
                    self.set_plan_link_picture(ctx, binding.target.page + 1, true, preview);
                    self.plan_links.remote_preview = Some(RemotePreviewReceipt {
                        binding,
                        source_sha256,
                    });
                }
            }
            Ok(
                outcome @ (Outcome::BatchInspected(_)
                | Outcome::BatchPrepared { .. }
                | Outcome::BatchPreview { .. }
                | Outcome::BatchSaved(_)),
            ) => {
                self.on_plan_links_batch(ctx, outcome);
            }
            Ok(Outcome::Cleared) => {}
            Err(EngineError::Cancelled) => {
                self.plan_links.clear_preview();
                self.plan_links.message = Some(tr(lang, "status.cancelled").into());
            }
            Err(error) => {
                self.plan_links.clear_preview();
                self.plan_links.error = Some(error.to_string());
            }
        }
    }
    fn set_plan_link_picture(
        &mut self,
        ctx: &egui::Context,
        page: usize,
        target: bool,
        preview: Preview,
    ) {
        let image = preview.image;
        self.plan_links.picture = Some(Picture {
            texture: ctx.load_texture(
                "plan-link-preview",
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width as usize, image.height as usize],
                    &image.rgba,
                ),
                egui::TextureOptions::LINEAR,
            ),
            points: preview.points,
            page,
            target,
        });
    }
    pub(crate) fn plan_links_dialog(&mut self, ctx: &egui::Context) {
        if !self.plan_links.open {
            return;
        }
        if self
            .plan_links
            .remote_preview
            .as_ref()
            .is_some_and(|receipt| !self.matches_remote_plan_link(&receipt.binding))
        {
            self.plan_links.clear_preview();
            self.plan_links.error = Some(tr(self.lang(), "plan_links.remote_changed").into());
        }
        if self.plan_links.batch_mode && self.plan_links.remote.is_none() {
            self.plan_links_batch_dialog(ctx);
            return;
        }
        let lang = self.lang();
        let busy = self.plan_links.busy();
        let saving = self.plan_links.pending.is_some_and(|(_, saving)| saving);
        let ready = !busy && matches!(self.engine, EngineState::Ready);
        let mut action = None;
        let mut footer_action = None;
        let mut close = false;
        let mut cancel = false;
        let mut batch_mode = false;
        let spec = Spec::new(
            "plan_links",
            tr(lang, "plan_links.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .icon(crate::icons::glyph::PAGES, "↗")
        .width(930.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0)));
        let state = &mut self.plan_links;
        let can_prepare = state.review().is_some() && state.remote.is_none();
        let can_save = state.candidate.is_some();
        let remote = state.remote.is_some();
        let can_navigate = state.context.is_some();
        dialog::show(
            ctx,
            spec,
            |ui| {
                if state.remote.is_none()
                    && ui
                        .add_enabled(
                            ready,
                            egui::Button::new(tr(lang, "plan_links.batch.switch")),
                        )
                        .clicked()
                {
                    batch_mode = true;
                }
                ui.label(tr(
                    lang,
                    if state.remote.is_some() {
                        "plan_links.remote_help"
                    } else {
                        "plan_links.help"
                    },
                ));
                if let Some(selected) = manifest_selector(ui, state, lang, ready) {
                    action = Some(selected);
                }
                if let Some((_, target)) = &state.remote {
                    ui.label(format!(
                        "{} · {} {}",
                        target.relative_path,
                        tr(lang, "plan_links.page"),
                        target.page.saturating_add(1)
                    ));
                    ui.small(format!("{} · {}", target.sheet_id, target.sha256));
                }
                if let Some(context) = &state.context {
                    ui.label(context.set.name.as_str());
                    ui.small(context.path.display().to_string());
                    ui.collapsing(tr(lang, "plan_links.passwords"), |ui| {
                        ui.small(tr(
                            lang,
                            if state.remote.is_some() {
                                "plan_links.remote_password_help"
                            } else {
                                "plan_links.password_help"
                            },
                        ));
                        ui.add_enabled_ui(ready, |ui| {
                            for (id, secret) in &mut state.passwords {
                                let name = context
                                    .set
                                    .files
                                    .iter()
                                    .find(|f| &f.id == id)
                                    .map_or(id.as_str(), |f| f.path.as_str());
                                ui.horizontal(|ui| {
                                    ui.label(name);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut **secret)
                                            .password(true)
                                            .char_limit(256)
                                            .desired_width(220.0),
                                    );
                                });
                            }
                        });
                    });
                    if let Some((_, target)) = &state.remote {
                        if let Some(sheet) = context
                            .set
                            .sheets
                            .iter()
                            .find(|sheet| sheet.id == target.sheet_id)
                        {
                            for (key, value) in [
                                ("plan_sets.number", &sheet.number),
                                ("plan_sets.sheet_title", &sheet.title),
                                ("plan_sets.revision", &sheet.revision),
                            ] {
                                ui.label(format!(
                                    "{}: {}",
                                    tr(lang, key),
                                    if value.is_empty() { "—" } else { value }
                                ));
                            }
                        }
                        ui.label(tr(
                            lang,
                            if state.remote_preview.is_some() {
                                "plan_links.remote_verified"
                            } else {
                                "plan_links.remote_preview_hint"
                            },
                        ));
                    } else {
                        let scan_editable = ready && state.inspection.is_none();
                        ui.add_enabled_ui(scan_editable, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "plan_links.source"));
                                let label = context
                                    .set
                                    .files
                                    .iter()
                                    .find(|f| f.id == state.source_id)
                                    .map_or("", |f| f.path.as_str());
                                egui::ComboBox::from_id_salt("plan-links-source")
                                    .selected_text(label)
                                    .show_ui(ui, |ui| {
                                        for file in &context.set.files {
                                            if ui
                                                .selectable_value(
                                                    &mut state.source_id,
                                                    file.id.clone(),
                                                    &file.path,
                                                )
                                                .changed()
                                            {
                                                state.targets.clear();
                                            }
                                        }
                                    });
                                ui.label(tr(lang, "plan_links.pages"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.pages)
                                        .char_limit(4096)
                                        .desired_width(140.0),
                                );
                            });
                            ui.label(tr(lang, "plan_links.targets"));
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(tr(lang, "plan_links.select_targets")).clicked() {
                                    state.targets = context
                                        .set
                                        .sheets
                                        .iter()
                                        .filter(|s| s.file_id != state.source_id)
                                        .map(|s| s.id.clone())
                                        .collect();
                                }
                                if ui.button(tr(lang, "plan_links.clear_targets")).clicked() {
                                    state.targets.clear();
                                }
                            });
                            egui::ScrollArea::vertical()
                                .id_salt("plan-links-targets")
                                .max_height(135.0)
                                .show(ui, |ui| {
                                    for sheet in context
                                        .set
                                        .sheets
                                        .iter()
                                        .filter(|s| s.file_id != state.source_id)
                                    {
                                        let mut selected = state.targets.contains(&sheet.id);
                                        if ui
                                            .checkbox(
                                                &mut selected,
                                                format!(
                                                    "{} · {} · {} · {}",
                                                    sheet.number,
                                                    sheet.title,
                                                    sheet.discipline,
                                                    sheet.revision
                                                ),
                                            )
                                            .changed()
                                        {
                                            if selected {
                                                state.targets.insert(sheet.id.clone());
                                            } else {
                                                state.targets.remove(&sheet.id);
                                            }
                                        }
                                    }
                                });
                            ui.checkbox(&mut state.saved_only, tr(lang, "plan_links.saved_only"));
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_enabled(
                                    scan_editable && state.saved_only,
                                    egui::Button::new(tr(lang, "plan_links.inspect")),
                                )
                                .clicked()
                            {
                                action = Some(Action::Inspect);
                            }
                            if ui
                                .add_enabled(
                                    ready && state.inspection.is_some(),
                                    egui::Button::new(tr(lang, "plan_links.reset")),
                                )
                                .clicked()
                            {
                                action = Some(Action::Reset);
                            }
                        });
                    }
                }
                if let Some(report) = &state.inspection {
                    ui.separator();
                    ui.label(format!(
                        "{}: {}/{} · {}: {}",
                        tr(lang, "plan_links.pages_scanned"),
                        report.scanned_pages,
                        report.requested_pages.len(),
                        tr(lang, "plan_links.references"),
                        report.occurrences.len()
                    ));
                    if !report.complete() {
                        ui.label(
                            egui::RichText::new(tr(lang, "plan_links.partial"))
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                    if report.occurrences.is_empty() {
                        ui.label(tr(lang, "plan_links.no_matches"));
                    }
                    for issue in &report.issues {
                        ui.small(format!(
                            "{} {} · {}",
                            issue.page.map_or_else(String::new, |p| p.to_string()),
                            issue.sheet_id.as_deref().unwrap_or(""),
                            status_text(lang, &issue.reason)
                        ));
                    }
                    let mut changed = false;
                    ui.add_enabled_ui(ready, |ui| {
                        if ui.button(tr(lang, "plan_links.approve_ready")).clicked() {
                            for occurrence in
                                report.occurrences.iter().filter(|o| o.status == "ready")
                            {
                                state.selections.insert(
                                    occurrence.id.clone(),
                                    occurrence.target_sheet_ids[0].clone(),
                                );
                            }
                            changed = true;
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("plan-links-occurrences")
                            .max_height(235.0)
                            .show(ui, |ui| {
                                for occurrence in &report.occurrences {
                                    ui.push_id(&occurrence.id, |ui| {
                                        let available: Vec<_> = report
                                            .targets
                                            .iter()
                                            .filter(|t| {
                                                t.status == "ready"
                                                    && occurrence
                                                        .target_sheet_ids
                                                        .contains(&t.sheet_id)
                                            })
                                            .collect();
                                        let selectable = matches!(
                                            occurrence.status.as_str(),
                                            "ready" | "ambiguous"
                                        ) && !available.is_empty();
                                        let mut selected =
                                            state.selections.contains_key(&occurrence.id);
                                        ui.horizontal_wrapped(|ui| {
                                            if ui
                                                .add_enabled(
                                                    selectable,
                                                    egui::Checkbox::new(
                                                        &mut selected,
                                                        format!(
                                                            "{} · {} {}",
                                                            occurrence.text,
                                                            tr(lang, "plan_links.page"),
                                                            occurrence.page
                                                        ),
                                                    ),
                                                )
                                                .changed()
                                            {
                                                if selected && available.len() == 1 {
                                                    state.selections.insert(
                                                        occurrence.id.clone(),
                                                        available[0].sheet_id.clone(),
                                                    );
                                                } else if !selected {
                                                    state.selections.remove(&occurrence.id);
                                                }
                                                changed = true;
                                            }
                                            ui.small(status_text(lang, &occurrence.status));
                                            if selectable {
                                                let selected = state
                                                    .selections
                                                    .get(&occurrence.id)
                                                    .cloned()
                                                    .unwrap_or_default();
                                                let label = available
                                                    .iter()
                                                    .find(|t| t.sheet_id == selected)
                                                    .map_or_else(
                                                        || {
                                                            tr(lang, "plan_links.choose_target")
                                                                .to_owned()
                                                        },
                                                        |t| {
                                                            format!(
                                                                "{} · {} · {}",
                                                                t.number, t.title, t.revision
                                                            )
                                                        },
                                                    );
                                                egui::ComboBox::from_id_salt("destination")
                                                    .selected_text(label)
                                                    .show_ui(ui, |ui| {
                                                        for target in &available {
                                                            if ui
                                                                .selectable_label(
                                                                    selected == target.sheet_id,
                                                                    format!(
                                                                        "{} · {} · {} · {}",
                                                                        target.number,
                                                                        target.title,
                                                                        target.discipline,
                                                                        target.revision
                                                                    ),
                                                                )
                                                                .clicked()
                                                            {
                                                                state.selections.insert(
                                                                    occurrence.id.clone(),
                                                                    target.sheet_id.clone(),
                                                                );
                                                                changed = true;
                                                            }
                                                        }
                                                    });
                                            }
                                        });
                                    });
                                }
                            });
                    });
                    if changed {
                        state.clear_preview();
                    }
                }
                if let Some((_, report)) = &state.candidate {
                    ui.separator();
                    ui.label(format!(
                        "{}: {} · {}: {}",
                        tr(lang, "plan_links.approved"),
                        report.created.len(),
                        tr(lang, "plan_links.excluded"),
                        report.excluded_occurrences.len()
                    ));
                    ui.add_enabled_ui(ready, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr(lang, "plan_links.page"));
                            let mut pages: Vec<_> = report.created.iter().map(|c| c.page).collect();
                            pages.sort_unstable();
                            pages.dedup();
                            egui::ComboBox::from_id_salt("plan-links-preview-page")
                                .selected_text(state.preview_page.to_string())
                                .show_ui(ui, |ui| {
                                    for page in pages {
                                        ui.selectable_value(
                                            &mut state.preview_page,
                                            page,
                                            page.to_string(),
                                        );
                                    }
                                });
                            if ui.button(tr(lang, "plan_links.source_preview")).clicked() {
                                action = Some(Action::SourcePreview);
                            }
                            egui::ComboBox::from_id_salt("plan-links-target-preview")
                                .selected_text((state.target_index + 1).to_string())
                                .show_ui(ui, |ui| {
                                    for (index, link) in report.created.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut state.target_index,
                                            index,
                                            format!(
                                                "{} · {}",
                                                index + 1,
                                                link.target.relative_path
                                            ),
                                        );
                                    }
                                });
                            if ui.button(tr(lang, "plan_links.target_preview")).clicked() {
                                action = Some(Action::TargetPreview);
                            }
                        });
                    });
                }
                if let Some(picture) = &state.picture {
                    ui.label(tr(
                        lang,
                        if picture.target {
                            "plan_links.target_preview"
                        } else {
                            "plan_links.source_preview"
                        },
                    ));
                    let natural = picture.texture.size_vec2();
                    let factor = (ui.available_width() / natural.x)
                        .min(390.0 / natural.y)
                        .min(1.0);
                    let response = ui.add(
                        egui::Image::new(&picture.texture).fit_to_exact_size(natural * factor),
                    );
                    if !picture.target {
                        if let Some((_, report)) = &state.candidate {
                            for (index, link) in report
                                .created
                                .iter()
                                .enumerate()
                                .filter(|(_, link)| link.page == picture.page)
                            {
                                let [left, bottom, right, top] = link.rect;
                                let rect = egui::Rect::from_min_max(
                                    response.rect.min
                                        + egui::vec2(
                                            left / picture.points.0 * response.rect.width(),
                                            (1.0 - top / picture.points.1) * response.rect.height(),
                                        ),
                                    response.rect.min
                                        + egui::vec2(
                                            right / picture.points.0 * response.rect.width(),
                                            (1.0 - bottom / picture.points.1)
                                                * response.rect.height(),
                                        ),
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 110, 210)),
                                    egui::StrokeKind::Outside,
                                );
                                ui.painter().text(
                                    rect.min,
                                    egui::Align2::LEFT_BOTTOM,
                                    (index + 1).to_string(),
                                    egui::FontId::proportional(13.0),
                                    egui::Color32::from_rgb(0, 70, 140),
                                );
                            }
                        }
                    }
                }
                if busy {
                    ui.spinner();
                    ui.label(tr(
                        lang,
                        if saving {
                            "plan_links.saving"
                        } else {
                            "plan_links.working"
                        },
                    ));
                }
                if let Some(error) = &state.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
            },
            |ui| {
                if remote {
                    if ui
                        .add_enabled(
                            ready && can_navigate,
                            egui::Button::new(tr(lang, "plan_links.open_verified")),
                        )
                        .clicked()
                    {
                        footer_action = Some(Action::Navigate);
                    }
                    if ui
                        .add_enabled(
                            ready && can_navigate,
                            egui::Button::new(tr(lang, "plan_links.target_preview")),
                        )
                        .clicked()
                    {
                        footer_action = Some(Action::RemotePreview);
                    }
                } else {
                    if ui
                        .add_enabled(
                            ready && can_save,
                            egui::Button::new(tr(lang, "plan_links.save_copy")),
                        )
                        .clicked()
                    {
                        footer_action = Some(Action::Save);
                    }
                    if ui
                        .add_enabled(
                            ready && can_prepare,
                            egui::Button::new(tr(lang, "plan_links.prepare")),
                        )
                        .clicked()
                    {
                        footer_action = Some(Action::Prepare);
                    }
                }
                if busy && ui.button(tr(lang, "dialog.cancel")).clicked() {
                    cancel = true;
                }
                if ui
                    .add_enabled(!saving, egui::Button::new(tr(lang, "dialog.close")))
                    .clicked()
                    || (!saving && ui.input(|i| i.key_pressed(egui::Key::Escape)))
                {
                    close = true;
                }
            },
        );
        if batch_mode {
            self.plan_links.batch_mode = true;
            self.plan_links.reset_scan();
            self.worker.send(Request::PlanLinks {
                generation: self.plan_links.generation,
                job: Job::Clear,
            });
        }
        if cancel {
            self.plan_links.cancel.store(true, Ordering::Relaxed);
            self.plan_links
                .passwords
                .iter_mut()
                .for_each(|(_, p)| *p = Zeroizing::new(String::new()));
        }
        if close {
            self.close_plan_links();
            return;
        }
        if let Some(action) = footer_action.or(action) {
            self.plan_links_action(action);
        }
    }
    fn plan_links_action(&mut self, action: Action) {
        let lang = self.lang();
        match action {
            Action::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(lang, "plan_sets.manifest"), &["json"])
                    .pick_file()
                {
                    self.load_plan_link_manifest(path);
                }
            }
            Action::LoadManifest => match manifest_input_path(&self.plan_links.manifest_input) {
                Ok(path) => self.load_plan_link_manifest(path),
                Err(_) => {
                    self.plan_links.reset_scan();
                    self.plan_links.context = None;
                    self.plan_links.passwords.clear();
                    self.plan_links.error =
                        Some(tr(lang, "plan_links.manifest_path_invalid").into());
                }
            },
            Action::Reset => {
                self.plan_links.reset_scan();
                self.worker.send(Request::PlanLinks {
                    generation: self.plan_links.generation,
                    job: Job::Clear,
                });
            }
            Action::Inspect => {
                let (Some(context), Ok(scan)) =
                    (self.plan_links.context.clone(), self.plan_links.scan())
                else {
                    self.plan_links.error = Some(tr(lang, "plan_links.invalid_selection").into());
                    return;
                };
                self.start_plan_links(false, |cancel, credentials| Job::Inspect {
                    context,
                    scan,
                    credentials,
                    cancel,
                });
            }
            Action::Prepare => {
                if let (Some(context), Some(review)) =
                    (self.plan_links.context.clone(), self.plan_links.review())
                {
                    self.start_plan_links(false, |cancel, credentials| Job::Prepare {
                        context,
                        review,
                        credentials,
                        cancel,
                    });
                }
            }
            Action::SourcePreview | Action::TargetPreview => {
                if let Some((token, _)) = &self.plan_links.candidate {
                    let (token, page, index) = (
                        *token,
                        self.plan_links.preview_page,
                        self.plan_links.target_index,
                    );
                    self.start_plan_links(false, |cancel, credentials| {
                        if matches!(action, Action::SourcePreview) {
                            Job::Preview {
                                token,
                                page,
                                credentials,
                                cancel,
                            }
                        } else {
                            Job::Target {
                                token,
                                index,
                                credentials,
                                cancel,
                            }
                        }
                    });
                }
            }
            Action::Save => {
                if let (Some((token, report)), Some(context)) =
                    (&self.plan_links.candidate, &self.plan_links.context)
                {
                    let token = *token;
                    let Ok(source) = source_path(context, &report.inspection.source_file_id) else {
                        return;
                    };
                    let parent = source.parent().unwrap_or(&context.path);
                    if let Some(output) = rfd::FileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .set_directory(parent)
                        .set_file_name("linked-plans.pdf")
                        .save_file()
                    {
                        self.start_plan_links(true, |cancel, _| Job::Save {
                            token,
                            output,
                            cancel,
                        });
                    }
                }
            }
            Action::Navigate => self.start_remote_plan_link(false),
            Action::RemotePreview => self.start_remote_plan_link(true),
        }
    }
    fn load_plan_link_manifest(&mut self, path: PathBuf) {
        self.plan_links.reset_scan();
        self.plan_links.context = None;
        self.plan_links.passwords.clear();
        self.plan_links.manifest_input = path.display().to_string();
        self.start_plan_links(false, |cancel, _| Job::Load { path, cancel });
    }
}

fn status_text(lang: Language, status: &str) -> &'static str {
    tr(
        lang,
        match status {
            "ready" => "plan_links.status.ready",
            "ambiguous" => "plan_links.status.ambiguous",
            "existing_link" => "plan_links.status.existing",
            "unsupported_geometry" => "plan_links.status.geometry",
            "no_searchable_text" => "plan_links.status.no_text",
            "unsupported_identifier" => "plan_links.status.identifier",
            "same_document_target" => "plan_links.status.same_document",
            "duplicate_reference" => "plan_links.status.duplicate",
            "copy_restricted" => "plan_links.status.restricted",
            "page_character_limit" => "plan_links.status.limit",
            "page_out_of_range" | "page_count_changed" => "plan_links.status.pages",
            "unreadable_or_protected" => "plan_links.status.password",
            _ => "plan_links.status.unavailable",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::test_app;
    use sunipdf_core::{
        plan_links::{PlanLinkOccurrence, PlanLinkTarget},
        plan_set::{PlanFile, PlanSheet},
        Command,
    };

    pub(super) fn context() -> Context {
        let mut set = PlanSet::new("Review sheets").unwrap();
        for (id, path) in [("source", "index.pdf"), ("target", "floor.pdf")] {
            set.files.push(PlanFile {
                id: id.into(),
                path: path.into(),
                sha256: "a".repeat(64),
                byte_len: 100,
                page_count: 1,
            });
            set.sheets.push(PlanSheet {
                id: format!("{id}-sheet"),
                file_id: id.into(),
                page: 1,
                number: if id == "target" { "A-10" } else { "Index" }.into(),
                title: path.into(),
                discipline: "Architecture".into(),
                revision: "A".into(),
            });
        }
        Context {
            path: "plans.json".into(),
            set,
        }
    }
    pub(super) fn inspection() -> PlanLinkInspection {
        PlanLinkInspection {
            version: 1,
            manifest_sha256: "b".repeat(64),
            source_sha256: "a".repeat(64),
            source_file_id: "source".into(),
            requested_pages: vec![1],
            scanned_pages: 1,
            scan_complete: true,
            truncated: false,
            issues: vec![],
            targets: vec![PlanLinkTarget {
                sheet_id: "target-sheet".into(),
                file_id: "target".into(),
                number: "A-10".into(),
                title: "Floor".into(),
                discipline: "Architecture".into(),
                revision: "A".into(),
                page: 1,
                sha256: "a".repeat(64),
                byte_len: 100,
                status: "ready".into(),
            }],
            occurrences: vec![PlanLinkOccurrence {
                id: "reference-1".into(),
                fingerprint: "c".repeat(64),
                page: 1,
                first_character: 0,
                end_character: 4,
                text: "A-10".into(),
                rect: Some([20.0, 30.0, 50.0, 40.0]),
                target_sheet_ids: vec!["target-sheet".into()],
                status: "ready".into(),
            }],
        }
    }
    fn populated() -> PlanLinksState {
        PlanLinksState {
            context: Some(context()),
            source_id: "source".into(),
            pages: "1".into(),
            targets: HashSet::from(["target-sheet".into()]),
            inspection: Some(inspection()),
            ..Default::default()
        }
    }
    fn remote_state(app: &mut SuniApp) {
        app.plan_links = populated();
        app.plan_links.open = true;
        app.plan_links.remote_epoch = Some(app.view.epoch);
        app.plan_links.remote = Some((
            app.document.as_ref().unwrap().info.path.clone(),
            RemotePageTarget {
                relative_path: "floor.pdf".into(),
                page: 0,
                sheet_id: "target-sheet".into(),
                sha256: "a".repeat(64),
                byte_len: 100,
            },
        ));
    }
    fn remote_result(binding: RemotePreviewBinding) -> Outcome {
        Outcome::RemotePreview {
            binding,
            source_sha256: "d".repeat(64),
            preview: Preview {
                image: RenderedPage {
                    page_index: 0,
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                },
                points: (100.0, 100.0),
            },
        }
    }
    #[test]
    fn remote_preview_is_explicit_and_opening_carries_its_source_binding() {
        let (mut app, requests, _) = test_app();
        remote_state(&mut app);
        app.plan_links.passwords.push((
            "target".into(),
            Zeroizing::new(format!("fixture-{}", std::process::id())),
        ));
        app.plan_links_action(Action::RemotePreview);
        let Request::PlanLinks {
            generation,
            job:
                Job::RemotePreview {
                    binding,
                    credentials,
                    ..
                },
        } = requests.try_recv().unwrap()
        else {
            panic!("preview must request rendering, not navigation");
        };
        assert_eq!(credentials.len(), 1);
        assert!(app
            .plan_links
            .passwords
            .iter()
            .all(|(_, value)| value.is_empty()));
        app.on_plan_links(
            &egui::Context::default(),
            generation,
            Ok(remote_result(binding.clone())),
        );
        assert!(app.plan_links.picture.is_some() && app.plan_links.remote_preview.is_some());
        assert!(
            requests.try_iter().next().is_none(),
            "preview cannot open a document"
        );
        app.plan_links_action(Action::Navigate);
        let Request::PlanLinks {
            job:
                Job::NavigatePreviewed {
                    binding: opening,
                    source_sha256,
                    ..
                },
            ..
        } = requests.try_recv().unwrap()
        else {
            panic!("opening must retain the reviewed source fingerprint");
        };
        assert_eq!(opening, binding);
        assert_eq!(source_sha256, "d".repeat(64));
        assert!(app.plan_links.picture.is_none());
    }
    #[test]
    fn changed_or_cancelled_remote_requests_never_publish_a_preview() {
        for changed in ["source", "epoch", "manifest", "target", "cancel", "closed"] {
            let (mut app, requests, _) = test_app();
            remote_state(&mut app);
            app.plan_links_action(Action::RemotePreview);
            let Request::PlanLinks {
                generation,
                job: Job::RemotePreview { binding, .. },
            } = requests.try_recv().unwrap()
            else {
                panic!("missing preview");
            };
            match changed {
                "source" => app.document.as_mut().unwrap().info.path = "other.pdf".into(),
                "epoch" => app.view.epoch += 1,
                "manifest" => {
                    app.plan_links.context.as_mut().unwrap().set.name = "Different set".into()
                }
                "target" => {
                    app.plan_links.remote.as_mut().unwrap().1.sheet_id = "other-sheet".into()
                }
                "cancel" => app.plan_links.cancel.store(true, Ordering::Relaxed),
                "closed" => app.close_plan_links(),
                _ => unreachable!(),
            }
            requests.try_iter().for_each(drop);
            app.on_plan_links(
                &egui::Context::default(),
                generation,
                Ok(remote_result(binding)),
            );
            assert!(
                app.plan_links.picture.is_none() && app.plan_links.remote_preview.is_none(),
                "{changed}"
            );
            assert!(
                requests.try_iter().next().is_none(),
                "{changed} caused navigation"
            );
        }
    }
    #[test]
    fn source_changes_invalidate_a_displayed_remote_preview_before_opening() {
        let (mut app, requests, _) = test_app();
        remote_state(&mut app);
        app.plan_links_action(Action::RemotePreview);
        let Request::PlanLinks {
            generation,
            job: Job::RemotePreview { binding, .. },
        } = requests.try_recv().unwrap()
        else {
            panic!("missing preview");
        };
        app.on_plan_links(
            &egui::Context::default(),
            generation,
            Ok(remote_result(binding)),
        );
        app.view.epoch += 1;
        app.plan_links_action(Action::Navigate);
        assert!(app.plan_links.picture.is_none() && app.plan_links.remote_preview.is_none());
        assert!(app.plan_links.error.is_some());
        assert!(requests.try_iter().next().is_none());
    }
    #[test]
    fn remote_source_fingerprint_refuses_changed_missing_and_cancelled_sources() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("source.pdf");
        std::fs::write(&path, b"saved source fixture").unwrap();
        let cancel = AtomicBool::new(false);
        let expected = source_fingerprint(&path, &cancel).unwrap();
        unchanged_source(&path, &expected, &cancel).unwrap();
        std::fs::write(&path, b"changed saved fixture").unwrap();
        assert!(unchanged_source(&path, &expected, &cancel).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(unchanged_source(&path, &expected, &cancel).is_err());
        std::fs::write(&path, b"saved source fixture").unwrap();
        cancel.store(true, Ordering::Relaxed);
        assert!(matches!(
            source_fingerprint(&path, &cancel),
            Err(EngineError::Cancelled)
        ));
    }
    #[test]
    fn manifest_paths_are_bounded_and_only_load_after_explicit_input() {
        let (mut app, requests, _) = test_app();
        let path = std::env::temp_dir().join("reviewed-plans.json");
        app.plan_links = populated();
        app.plan_links.manifest_input = path.display().to_string();
        assert!(requests.try_iter().next().is_none());
        app.plan_links_action(Action::LoadManifest);
        assert!(app.plan_links.context.is_none());
        let Request::PlanLinks {
            job: Job::Load { path: loaded, .. },
            ..
        } = requests.try_recv().unwrap()
        else {
            panic!("missing explicit manifest load");
        };
        assert_eq!(loaded, path);
        for invalid in [
            "relative.json".to_owned(),
            "https://example.invalid/set.json".into(),
            "\0manifest.json".into(),
            "a".repeat(4097),
        ] {
            assert!(manifest_input_path(&invalid).is_err(), "{invalid:?}");
        }
    }
    #[test]
    fn enter_from_the_focused_manifest_field_loads_the_selected_path() {
        let (mut app, requests, _) = test_app();
        app.plan_links.open = true;
        app.plan_links.focus_manifest = true;
        let ctx = egui::Context::default();
        let path = std::env::temp_dir()
            .join("keyboard-plans.json")
            .display()
            .to_string();
        for frame in 0..5 {
            let events = match frame {
                3 => vec![egui::Event::Text(path.clone())],
                4 => vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                _ => vec![],
            };
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    events,
                    time: Some(frame as f64 * 0.03),
                    ..Default::default()
                },
                |ui| app.plan_links_dialog(ui.ctx()),
            );
            output.textures_delta.clear();
        }
        let Request::PlanLinks {
            job: Job::Load { path: loaded, .. },
            ..
        } = requests.try_recv().unwrap()
        else {
            panic!("Enter did not load the manifest");
        };
        assert_eq!(loaded, PathBuf::from(path));
    }
    #[test]
    fn review_requires_explicit_selection_and_valid_page_list() {
        let mut state = populated();
        assert!(
            state.review().is_none(),
            "inspection must never approve references implicitly"
        );
        state
            .selections
            .insert("reference-1".into(), "target-sheet".into());
        let review = state.review().unwrap();
        assert_eq!(review.selections.len(), 1);
        assert!(review.selections[0].confirmed);
        for invalid in ["", "0", "1,1", "1,x", "1,"] {
            state.pages = invalid.into();
            assert!(state.review().is_none());
        }
        state.pages = (1..=33)
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert!(state.scan().is_err());
        state.pages = "1".into();
        state.reset_scan();
        assert!(
            state.inspection.is_none() && state.selections.is_empty() && state.candidate.is_none()
        );
    }
    #[test]
    fn close_invalidates_read_results_and_clears_secrets_but_keeps_password_fields() {
        let (mut app, requests, _) = test_app();
        app.plan_links = populated();
        app.plan_links.open = true;
        app.plan_links.passwords.push((
            "target".into(),
            Zeroizing::new(["temporary", "credential"].join("-")),
        ));
        app.plan_links.generation = 7;
        app.plan_links.pending = Some((7, false));
        let cancel = app.plan_links.cancel.clone();
        app.close_plan_links();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(!app.plan_links.open && !app.plan_links.busy());
        assert_eq!(app.plan_links.passwords.len(), 1);
        assert!(app.plan_links.passwords[0].1.is_empty());
        app.on_plan_links(
            &egui::Context::default(),
            7,
            Ok(Outcome::Inspected(inspection())),
        );
        assert!(app.plan_links.inspection.is_none());
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::PlanLinks {
                job: Job::Clear,
                ..
            }
        ));
        app.open_plan_links();
        assert!(app.plan_links.open && app.plan_links.context.is_some());
    }
    #[test]
    fn cancellation_during_save_retains_generation_and_announces_committed_copy() {
        let (mut app, requests, _) = test_app();
        app.plan_links = populated();
        app.plan_links.open = true;
        app.plan_links.generation = 12;
        app.plan_links.pending = Some((12, true));
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
        let mut output = ctx.run_ui(input.clone(), |ui| {
            eframe::App::ui(&mut app, ui, &mut frame)
        });
        output.textures_delta.clear();
        assert!(output
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
        assert!(!output
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::Close)));
        assert!(app.plan_links.open && app.plan_links.busy());
        assert_eq!(app.plan_links.generation, 12);
        assert!(!requests.try_iter().any(|request| matches!(
            request,
            Request::Close
                | Request::PlanLinks {
                    job: Job::Clear,
                    ..
                }
        )));
        let report = PlanLinkReport {
            version: 1,
            inspection: inspection(),
            created: vec![],
            excluded_occurrences: vec![],
            output_sha256: "d".repeat(64),
        };
        app.on_plan_links(
            &ctx,
            12,
            Ok(Outcome::Saved(PlanLinkReceipt {
                output: "linked.pdf".into(),
                bytes: 100,
                report,
            })),
        );
        assert!(!app.plan_links.busy());
        assert!(app
            .plan_links
            .message
            .as_deref()
            .unwrap()
            .contains("linked.pdf"));
        let mut output = ctx.run_ui(input, |ui| eframe::App::ui(&mut app, ui, &mut frame));
        output.textures_delta.clear();
        assert!(!output
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
    }
    fn text_bounds(shape: &egui::Shape, text: &str) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Text(value) if value.galley.text() == text => {
                Some(value.galley.rect.translate(value.pos.to_vec2()))
            }
            egui::Shape::Vec(values) => values.iter().find_map(|v| text_bounds(v, text)),
            _ => None,
        }
    }
    #[test]
    fn bilingual_dialog_has_reachable_footer_and_never_auto_opens_remote_links() {
        assert!(!Command::PlanLinks.info().needs_document);
        assert!(crate::workspace::TABLE
            .iter()
            .any(|w| w.contains(Command::PlanLinks)));
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            app.settings.set_language(lang);
            let ctx = egui::Context::default();
            app.execute(&ctx, Command::PlanLinks);
            assert!(app.plan_links.open && app.dialog_open() && app.navigation_blocked());
            app.plan_links = populated();
            app.plan_links.open = true;
            for frame in 0..4 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720.0, 480.0),
                        )),
                        time: Some(1.0 + frame as f64 * 0.03),
                        ..Default::default()
                    },
                    |ui| app.plan_links_dialog(ui.ctx()),
                );
                output.textures_delta.clear();
                if frame == 3 {
                    for key in [
                        "plan_links.title",
                        "plan_links.prepare",
                        "plan_links.save_copy",
                        "dialog.close",
                    ] {
                        let rect = output
                            .shapes
                            .iter()
                            .find_map(|s| text_bounds(&s.shape, tr(lang, key)))
                            .unwrap_or_else(|| panic!("missing {key} in {}", lang.code()));
                        assert!(
                            rect.left() >= 0.0
                                && rect.right() <= 720.0
                                && rect.top() >= 0.0
                                && rect.bottom() <= 480.0,
                            "{key} is unreachable: {rect:?}"
                        );
                    }
                }
            }
            assert!(requests.try_iter().next().is_none());
            app.plan_links.remote = Some((
                "index.pdf".into(),
                RemotePageTarget {
                    relative_path: "floor.pdf".into(),
                    page: 0,
                    sheet_id: "target-sheet".into(),
                    sha256: "a".repeat(64),
                    byte_len: 100,
                },
            ));
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720.0, 480.0),
                        )),
                        time: Some(2.0 + frame as f64 * 0.03),
                        ..Default::default()
                    },
                    |ui| app.plan_links_dialog(ui.ctx()),
                );
                output.textures_delta.clear();
                if frame == 2 {
                    for key in [
                        "plan_links.open_verified",
                        "plan_links.target_preview",
                        "dialog.close",
                    ] {
                        let rect = output
                            .shapes
                            .iter()
                            .find_map(|shape| text_bounds(&shape.shape, tr(lang, key)))
                            .unwrap_or_else(|| panic!("missing {key} in {}", lang.code()));
                        assert!(
                            rect.left() >= 0.0
                                && rect.right() <= 720.0
                                && rect.top() >= 0.0
                                && rect.bottom() <= 480.0,
                            "{key} is unreachable: {rect:?}"
                        );
                    }
                }
            }
            assert!(
                requests.try_iter().next().is_none(),
                "displaying a remote link never opens a file"
            );
        }
    }
}
