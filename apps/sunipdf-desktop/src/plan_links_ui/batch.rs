//! Source-scoped review and publication of new plan-link copies.
use super::core::{
    PlanLinkBatchFilePlan, PlanLinkBatchFileReview, PlanLinkBatchInspection, PlanLinkBatchOutcome,
    PlanLinkBatchPlan, PlanLinkBatchProgress, PlanLinkBatchReview, PlanLinkBatchScanRequest,
    PlanLinkBatchStatus,
};
use super::*;

pub(super) struct BatchCached {
    token: u64,
    plan: PlanLinkBatchPlan,
    context: Context,
}

fn source_dimensions(
    engine: &Engine,
    context: &Context,
    source_file_id: &str,
    page: usize,
    credentials: &Credentials,
    cancel: &AtomicBool,
) -> Result<(f32, f32), EngineError> {
    cancelled(cancel)?;
    let sheet = context
        .set
        .sheets
        .iter()
        .find(|s| s.file_id == source_file_id)
        .ok_or_else(|| error("source sheet is unavailable"))?;
    let verified = plan_set::read_sheet(&context.path, &context.set, &sheet.id)?;
    let doc = engine.open_bytes(
        verified.bytes,
        password(credentials, source_file_id),
        &verified.path,
    )?;
    let size = doc.page_size(page.checked_sub(1).ok_or_else(|| error("invalid page"))?)?;
    cancelled(cancel)?;
    Ok((size.width_pt, size.height_pt))
}

impl WorkerCache {
    fn batch_get(&self, token: u64) -> Result<&BatchCached, EngineError> {
        self.batch_candidate
            .as_ref()
            .filter(|c| c.token == token)
            .ok_or_else(|| error("batch preview expired; validate it again"))
    }
    pub(super) fn run_batch(
        &mut self,
        engine: &Engine,
        job: Job,
        on_progress: impl FnMut(&PlanLinkBatchProgress),
    ) -> Result<Outcome, EngineError> {
        match job {
            Job::BatchInspect {
                context,
                scan,
                credentials,
                cancel,
            } => {
                self.batch_candidate = None;
                let values = borrowed(&credentials);
                Ok(Outcome::BatchInspected(core::inspect_plan_link_batch(
                    engine,
                    &context.path,
                    &context.set,
                    &scan,
                    &PlanLinkCredentials { passwords: &values },
                    &cancel,
                )?))
            }
            Job::BatchPrepare {
                context,
                review,
                credentials,
                cancel,
            } => {
                self.batch_candidate = None;
                let values = borrowed(&credentials);
                let access = PlanLinkCredentials { passwords: &values };
                let plan = core::prepare_plan_link_batch(
                    engine,
                    &context.path,
                    &context.set,
                    &review,
                    &access,
                    &cancel,
                )?;
                let first = plan
                    .files()
                    .first()
                    .ok_or_else(|| error("no approved source files"))?;
                let page = first
                    .preview_pages
                    .first()
                    .copied()
                    .ok_or_else(|| error("no approved pages"))?;
                let image = plan.preview_file(0, engine, &access, page, 1200, &cancel)?;
                let points = source_dimensions(
                    engine,
                    &context,
                    &first.source_file_id,
                    page,
                    &credentials,
                    &cancel,
                )?;
                self.next = self.next.wrapping_add(1);
                let files = plan.files().to_vec();
                let token = self.next;
                self.batch_candidate = Some(BatchCached {
                    token,
                    plan,
                    context,
                });
                Ok(Outcome::BatchPrepared {
                    token,
                    files,
                    preview: Preview { image, points },
                })
            }
            Job::BatchPreview {
                token,
                file_index,
                page,
                target_sheet_id,
                credentials,
                cancel,
            } => {
                let cached = self.batch_get(token)?;
                let file = cached
                    .plan
                    .files()
                    .get(file_index)
                    .ok_or_else(|| error("source selection changed"))?;
                let values = borrowed(&credentials);
                let access = PlanLinkCredentials { passwords: &values };
                let (image, points, shown_page) = if let Some(sheet_id) = target_sheet_id.as_deref()
                {
                    let sheet = cached
                        .context
                        .set
                        .sheets
                        .iter()
                        .find(|s| s.id == sheet_id)
                        .ok_or_else(|| error("destination disappeared"))?;
                    (
                        cached
                            .plan
                            .preview_target(file_index, engine, &access, sheet_id, 1200, &cancel)?,
                        (1.0, 1.0),
                        sheet.page,
                    )
                } else {
                    (
                        cached
                            .plan
                            .preview_file(file_index, engine, &access, page, 1200, &cancel)?,
                        source_dimensions(
                            engine,
                            &cached.context,
                            &file.source_file_id,
                            page,
                            &credentials,
                            &cancel,
                        )?,
                        page,
                    )
                };
                Ok(Outcome::BatchPreview {
                    token,
                    source_file_id: file.source_file_id.clone(),
                    page: shown_page,
                    target_sheet_id,
                    preview: Preview { image, points },
                })
            }
            Job::BatchSave {
                token,
                credentials,
                cancel,
            } => {
                self.batch_get(token)?;
                let cached = self
                    .batch_candidate
                    .take()
                    .ok_or_else(|| error("batch preview expired"))?;
                let values = borrowed(&credentials);
                Ok(Outcome::BatchSaved(core::publish_plan_link_batch(
                    cached.plan,
                    engine,
                    &PlanLinkCredentials { passwords: &values },
                    &cancel,
                    on_progress,
                )))
            }
            _ => Err(error("invalid batch job")),
        }
    }
}

#[derive(Clone)]
struct SourceRow {
    file_id: String,
    included: bool,
    pages: String,
    targets: HashSet<String>,
    target_filter: String,
    output_name: String,
}
struct PictureIdentity {
    source_file_id: String,
    source_filename: String,
    rendered_filename: String,
    target_sheet_id: Option<String>,
    target_sheet_number: Option<String>,
}
impl PictureIdentity {
    fn new(
        context: &Context,
        source_file_id: String,
        target_sheet_id: Option<String>,
    ) -> Option<Self> {
        let source_filename = context
            .set
            .files
            .iter()
            .find(|f| f.id == source_file_id)?
            .path
            .clone();
        let (rendered_filename, target_sheet_number) = if let Some(id) = &target_sheet_id {
            let sheet = context.set.sheets.iter().find(|s| &s.id == id)?;
            let file = context.set.files.iter().find(|f| f.id == sheet.file_id)?;
            (file.path.clone(), Some(sheet.number.clone()))
        } else {
            (source_filename.clone(), None)
        };
        Some(Self {
            source_file_id,
            source_filename,
            rendered_filename,
            target_sheet_id,
            target_sheet_number,
        })
    }
}
#[derive(Default)]
pub(super) struct BatchState {
    rows: Vec<SourceRow>,
    inspection: Option<PlanLinkBatchInspection>,
    selections: BTreeMap<(String, String), String>,
    candidate: Option<(u64, Vec<PlanLinkBatchFilePlan>)>,
    preview_source: usize,
    preview_page: usize,
    target_sheet_id: String,
    picture: Option<(PictureIdentity, Picture)>,
    progress: Option<PlanLinkBatchProgress>,
    outcome: Option<PlanLinkBatchOutcome>,
}

impl BatchState {
    pub(super) fn load(&mut self, context: &Context) {
        *self = Self::default();
        self.rows = context
            .set
            .files
            .iter()
            .map(|file| {
                let stem = std::path::Path::new(&file.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("plans");
                SourceRow {
                    file_id: file.id.clone(),
                    included: false,
                    pages: "1".into(),
                    targets: HashSet::new(),
                    target_filter: String::new(),
                    output_name: format!("{stem}-linked.pdf"),
                }
            })
            .collect();
    }
    pub(super) fn clear_preview(&mut self) {
        self.candidate = None;
        self.picture = None;
        self.progress = None;
    }
    pub(super) fn reset_scan(&mut self) {
        self.clear_preview();
        self.inspection = None;
        self.selections.clear();
    }
    fn scan(&self, context: &Context) -> Result<PlanLinkBatchScanRequest, SelectionError> {
        let included = self.rows.iter().filter(|row| row.included).count();
        if included == 0 {
            return Err(SelectionError::Sources);
        }
        if included > core::MAX_BATCH_SOURCES {
            return Err(SelectionError::SourceLimit);
        }
        let mut pages_total = 0;
        let sources = self
            .rows
            .iter()
            .filter(|row| row.included)
            .map(|row| {
                let file = context
                    .set
                    .files
                    .iter()
                    .find(|file| file.id == row.file_id)
                    .ok_or(SelectionError::Source)?;
                let pages = parse_pages(&row.pages, file.page_count)?;
                pages_total += pages.len();
                if pages_total > core::MAX_BATCH_PAGES {
                    return Err(SelectionError::TotalPages);
                }
                if row.targets.is_empty() {
                    return Err(SelectionError::Targets);
                }
                let targets: Vec<_> = context
                    .set
                    .sheets
                    .iter()
                    .filter(|s| row.targets.contains(&s.id) && s.file_id != row.file_id)
                    .map(|s| s.id.clone())
                    .collect();
                if targets.len() != row.targets.len() {
                    return Err(SelectionError::Targets);
                }
                Ok(PlanLinkScanRequest {
                    source_file_id: row.file_id.clone(),
                    pages,
                    target_sheet_ids: targets,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PlanLinkBatchScanRequest { sources })
    }
    fn review(&self, context: &Context) -> Option<PlanLinkBatchReview> {
        let inspection = self.inspection.as_ref()?;
        let request = self.scan(context).ok()?;
        let files = inspection
            .files
            .iter()
            .filter_map(|file| {
                let row = self
                    .rows
                    .iter()
                    .find(|r| r.included && r.file_id == file.source_file_id)?;
                let scan = request
                    .sources
                    .iter()
                    .find(|s| s.source_file_id == file.source_file_id)?
                    .clone();
                let selections: Vec<_> = file
                    .occurrences
                    .iter()
                    .filter_map(|occurrence| {
                        let target = self
                            .selections
                            .get(&(file.source_file_id.clone(), occurrence.id.clone()))?;
                        (matches!(occurrence.status.as_str(), "ready" | "ambiguous")
                            && occurrence.target_sheet_ids.contains(target)
                            && file
                                .targets
                                .iter()
                                .any(|t| &t.sheet_id == target && t.status == "ready"))
                        .then(|| PlanLinkSelection {
                            occurrence_id: occurrence.id.clone(),
                            fingerprint: occurrence.fingerprint.clone(),
                            target_sheet_id: target.clone(),
                            confirmed: true,
                        })
                    })
                    .collect();
                if selections.is_empty() {
                    return None;
                }
                Some(PlanLinkBatchFileReview {
                    review: PlanLinkReview {
                        version: 1,
                        manifest_sha256: inspection.manifest_sha256.clone(),
                        source_sha256: file.source_sha256.clone(),
                        scan,
                        selections,
                    },
                    output_name: row.output_name.clone(),
                })
            })
            .collect::<Vec<_>>();
        (!files.is_empty()).then(|| PlanLinkBatchReview {
            version: 1,
            manifest_sha256: inspection.manifest_sha256.clone(),
            files,
        })
    }
    fn selected_count(&self, source: &str) -> usize {
        self.selections
            .keys()
            .filter(|(id, _)| id == source)
            .count()
    }
    fn invalidate_changed_preview(&mut self, previous_page: usize, previous_target: &str) {
        if self.preview_page != previous_page || self.target_sheet_id != previous_target {
            self.picture = None;
        }
    }
    fn accept_preview(
        &mut self,
        ctx: &egui::Context,
        identity: PictureIdentity,
        page: usize,
        preview: Preview,
    ) {
        let image = preview.image;
        let target = identity.target_sheet_id.is_some();
        self.picture = Some((
            identity,
            Picture {
                texture: ctx.load_texture(
                    "plan-link-batch-preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width as usize, image.height as usize],
                        &image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ),
                points: preview.points,
                page,
                target,
            },
        ));
    }
}

enum BatchAction {
    Open,
    LoadManifest,
    Single,
    Inspect,
    Reset,
    Prepare,
    Preview(bool),
    Save,
    OpenSavedCopy(SavedCopy),
    ExportReceipt,
}

impl SuniApp {
    pub(crate) fn on_plan_links_progress(
        &mut self,
        generation: u64,
        progress: PlanLinkBatchProgress,
    ) {
        if self.plan_links.pending == Some((generation, true)) {
            self.plan_links.batch.progress = Some(progress);
        }
    }
    pub(super) fn on_plan_links_batch(&mut self, ctx: &egui::Context, outcome: Outcome) {
        let context = self.plan_links.context.as_ref();
        let batch = &mut self.plan_links.batch;
        match outcome {
            Outcome::BatchInspected(inspection) => {
                batch.reset_scan();
                batch.inspection = Some(inspection);
            }
            Outcome::BatchPrepared {
                token,
                files,
                preview,
            } => {
                let Some(first) = files.first() else {
                    return;
                };
                let source = first.source_file_id.clone();
                let page = first.preview_pages.first().copied().unwrap_or(1);
                batch.preview_source = 0;
                batch.preview_page = page;
                batch.target_sheet_id.clear();
                batch.candidate = Some((token, files));
                if let Some(identity) = context.and_then(|c| PictureIdentity::new(c, source, None))
                {
                    batch.accept_preview(ctx, identity, page, preview);
                }
            }
            Outcome::BatchPreview {
                token,
                source_file_id,
                page,
                target_sheet_id,
                preview,
            } => {
                let selection_matches = target_sheet_id.as_ref().map_or_else(
                    || batch.preview_page == page,
                    |id| &batch.target_sheet_id == id,
                );
                if selection_matches
                    && batch.candidate.as_ref().is_some_and(|(id, files)| {
                        *id == token
                            && files
                                .get(batch.preview_source)
                                .is_some_and(|f| f.source_file_id == source_file_id)
                    })
                {
                    if let Some(identity) = context
                        .and_then(|c| PictureIdentity::new(c, source_file_id, target_sheet_id))
                    {
                        batch.accept_preview(ctx, identity, page, preview);
                    }
                }
            }
            Outcome::BatchSaved(outcome) => {
                batch.clear_preview();
                batch.outcome = Some(outcome);
            }
            _ => {}
        }
    }

    pub(super) fn plan_links_batch_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        let busy = self.plan_links.busy();
        let saving = self.plan_links.pending.is_some_and(|(_, save)| save);
        let ready = !busy && matches!(self.engine, EngineState::Ready);
        let can_prepare = self
            .plan_links
            .context
            .as_ref()
            .and_then(|context| self.plan_links.batch.review(context))
            .is_some();
        let can_save = self.plan_links.batch.candidate.is_some();
        let mut action = None;
        let mut footer_action = None;
        let mut close = false;
        let mut cancel = false;
        let spec = Spec::new(
            "plan_links",
            tr(lang, "plan_links.batch.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .icon(crate::icons::glyph::PAGES, "↗")
        .width(980.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0)));
        let state = &mut self.plan_links;
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            ready,
                            egui::Button::new(tr(lang, "plan_links.batch.single")),
                        )
                        .clicked()
                    {
                        action = Some(BatchAction::Single);
                    }
                });
                if let Some(selected) = manifest_selector(ui, state, lang, ready) {
                    action = Some(match selected {
                        Action::LoadManifest => BatchAction::LoadManifest,
                        _ => BatchAction::Open,
                    });
                }
                ui.label(tr(lang, "plan_links.batch.help"));
                ui.small(tr(lang, "plan_links.batch.limits"));
                if let Some(context) = &state.context {
                    ui.label(&context.set.name);
                    ui.small(context.path.display().to_string());
                    ui.collapsing(tr(lang, "plan_links.passwords"), |ui| {
                        ui.small(tr(lang, "plan_links.password_help"));
                        ui.add_enabled_ui(ready, |ui| {
                            for (id, secret) in &mut state.passwords {
                                let name = context
                                    .set
                                    .files
                                    .iter()
                                    .find(|f| &f.id == id)
                                    .map_or(id.as_str(), |f| f.path.as_str());
                                ui.horizontal_wrapped(|ui| {
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
                    let editable = ready && state.batch.inspection.is_none();
                    let mut scan_changed = false;
                    ui.add_enabled_ui(editable, |ui| {
                        for row in &mut state.batch.rows {
                            let Some(file) = context.set.files.iter().find(|f| f.id == row.file_id)
                            else {
                                continue;
                            };
                            ui.push_id(&row.file_id, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    scan_changed |=
                                        ui.checkbox(&mut row.included, &file.path).changed();
                                    if row.included {
                                        ui.label(tr(lang, "plan_links.pages"));
                                        scan_changed |= ui
                                            .add(
                                                egui::TextEdit::singleline(&mut row.pages)
                                                    .char_limit(4096)
                                                    .hint_text("1-8,12")
                                                    .desired_width(110.0),
                                            )
                                            .changed();
                                        if ui.button(tr(lang, "plan_links.indexed_pages")).clicked()
                                        {
                                            match indexed_pages(context, &row.file_id) {
                                                Ok(pages) => {
                                                    row.pages = pages;
                                                    state.error = None;
                                                    scan_changed = true;
                                                }
                                                Err(reason) => {
                                                    state.error =
                                                        Some(tr(lang, reason.key()).into())
                                                }
                                            }
                                        }
                                    }
                                });
                                if row.included {
                                    ui.indent("targets", |ui| {
                                        ui.collapsing(
                                            format!(
                                                "{} ({})",
                                                tr(lang, "plan_links.targets"),
                                                row.targets.len()
                                            ),
                                            |ui| {
                                                scan_changed |= selection::target_controls(
                                                    ui,
                                                    context,
                                                    &row.file_id,
                                                    &mut row.target_filter,
                                                    &mut row.targets,
                                                    lang,
                                                );
                                            },
                                        );
                                    });
                                }
                            });
                        }
                        ui.checkbox(&mut state.saved_only, tr(lang, "plan_links.saved_only"));
                    });
                    if scan_changed {
                        state.batch.reset_scan();
                        state.error = None;
                    }
                    if editable {
                        ui.small(tr(lang, "plan_links.pages_help"));
                        if let Err(reason) = state.batch.scan(context) {
                            ui.colored_label(ui.visuals().warn_fg_color, tr(lang, reason.key()));
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                editable && state.saved_only && state.batch.scan(context).is_ok(),
                                egui::Button::new(tr(lang, "plan_links.inspect")),
                            )
                            .clicked()
                        {
                            action = Some(BatchAction::Inspect);
                        }
                        if ui
                            .add_enabled(
                                ready && state.batch.inspection.is_some(),
                                egui::Button::new(tr(lang, "plan_links.reset")),
                            )
                            .clicked()
                        {
                            action = Some(BatchAction::Reset);
                        }
                    });
                    render_review(ui, &mut state.batch, context, lang, ready);
                    render_candidate(ui, &mut state.batch, context, lang, ready, &mut action);
                }
                render_picture(ui, &state.batch, lang);
                if busy {
                    ui.spinner();
                    ui.label(tr(
                        lang,
                        if saving {
                            "plan_links.batch.saving"
                        } else {
                            "plan_links.working"
                        },
                    ));
                    if let Some(progress) = &state.batch.progress {
                        ui.label(format!(
                            "{}/{} · {}",
                            progress.completed,
                            progress.total,
                            progress.output.display()
                        ));
                    }
                }
                render_outcome(ui, state.batch.outcome.as_ref(), lang, ready, &mut action);
                if let Some(error) = &state.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
            },
            |ui| {
                if ui
                    .add_enabled(
                        ready && can_save,
                        egui::Button::new(tr(lang, "plan_links.batch.save")),
                    )
                    .clicked()
                {
                    footer_action = Some(BatchAction::Save);
                }
                if ui
                    .add_enabled(
                        ready && can_prepare,
                        egui::Button::new(tr(lang, "plan_links.batch.prepare")),
                    )
                    .clicked()
                {
                    footer_action = Some(BatchAction::Prepare);
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
            self.plan_links_batch_action(action);
        }
    }

    fn plan_links_batch_action(&mut self, action: BatchAction) {
        match action {
            BatchAction::Open => self.plan_links_action(Action::Open),
            BatchAction::LoadManifest => self.plan_links_action(Action::LoadManifest),
            BatchAction::Single => {
                self.plan_links.batch_mode = false;
                self.plan_links.reset_scan();
                self.worker.send(Request::PlanLinks {
                    generation: self.plan_links.generation,
                    job: Job::Clear,
                });
            }
            BatchAction::Reset => self.plan_links_action(Action::Reset),
            BatchAction::Inspect => {
                let Some(context) = self.plan_links.context.clone() else {
                    return;
                };
                let scan = match self.plan_links.batch.scan(&context) {
                    Ok(scan) => scan,
                    Err(reason) => {
                        self.plan_links.error = Some(tr(self.lang(), reason.key()).into());
                        return;
                    }
                };
                self.start_plan_links(false, |cancel, credentials| Job::BatchInspect {
                    context,
                    scan,
                    credentials,
                    cancel,
                });
            }
            BatchAction::Prepare => {
                let Some(context) = self.plan_links.context.clone() else {
                    return;
                };
                let Some(review) = self.plan_links.batch.review(&context) else {
                    return;
                };
                self.start_plan_links(false, |cancel, credentials| Job::BatchPrepare {
                    context,
                    review,
                    credentials,
                    cancel,
                });
            }
            BatchAction::Preview(target) => {
                let batch = &self.plan_links.batch;
                let Some((token, _)) = &batch.candidate else {
                    return;
                };
                let token = *token;
                let file_index = batch.preview_source;
                let page = batch.preview_page;
                let target_sheet_id = target.then(|| batch.target_sheet_id.clone());
                self.start_plan_links(false, |cancel, credentials| Job::BatchPreview {
                    token,
                    file_index,
                    page,
                    target_sheet_id,
                    credentials,
                    cancel,
                });
            }
            BatchAction::Save => {
                let Some((token, _)) = &self.plan_links.batch.candidate else {
                    return;
                };
                let token = *token;
                self.start_plan_links(true, |cancel, credentials| Job::BatchSave {
                    token,
                    credentials,
                    cancel,
                });
            }
            BatchAction::OpenSavedCopy(copy) => self.plan_links_action(Action::OpenSavedCopy(copy)),
            BatchAction::ExportReceipt => {
                if let Some(outcome) = self.plan_links.batch.outcome.clone() {
                    self.plan_links_action(Action::ExportReceipt(ReceiptExport::Batch(Box::new(
                        outcome,
                    ))));
                }
            }
        }
    }
}

fn source_label<'a>(context: &'a Context, id: &'a str) -> &'a str {
    context
        .set
        .files
        .iter()
        .find(|f| f.id == id)
        .map_or(id, |f| f.path.as_str())
}

fn render_review(
    ui: &mut egui::Ui,
    batch: &mut BatchState,
    context: &Context,
    lang: Language,
    ready: bool,
) {
    let Some(inspection) = &batch.inspection else {
        return;
    };
    ui.separator();
    ui.label(format!(
        "{}: {} · {}: {}",
        tr(lang, "plan_links.pages_scanned"),
        inspection.scanned_pages,
        tr(lang, "plan_links.references"),
        inspection.occurrences
    ));
    if !inspection.complete() {
        ui.colored_label(ui.visuals().warn_fg_color, tr(lang, "plan_links.partial"));
        if let Some(reason) = &inspection.stop_reason {
            ui.small(reason);
        }
        for source in &inspection.remaining_sources {
            ui.small(format!(
                "{}: {}",
                tr(lang, "plan_links.batch.not_scanned"),
                source_label(context, source)
            ));
        }
    }
    let mut changed = false;
    for file in &inspection.files {
        let count = batch.selected_count(&file.source_file_id);
        ui.collapsing(
            format!(
                "{} · {} {}",
                source_label(context, &file.source_file_id),
                count,
                tr(lang, "plan_links.approved")
            ),
            |ui| {
                ui.small(format!(
                    "{}: {}/{}",
                    tr(lang, "plan_links.pages_scanned"),
                    file.scanned_pages,
                    file.requested_pages.len()
                ));
                if !file.complete() {
                    ui.colored_label(ui.visuals().warn_fg_color, tr(lang, "plan_links.partial"));
                }
                for issue in &file.issues {
                    ui.small(format!(
                        "{} {}",
                        issue.page.map_or_else(String::new, |p| p.to_string()),
                        status_text(lang, &issue.reason)
                    ));
                }
                if file.occurrences.is_empty() {
                    ui.label(tr(lang, "plan_links.no_matches"));
                }
                ui.add_enabled_ui(ready, |ui| {
                    for occurrence in &file.occurrences {
                        let key = (file.source_file_id.clone(), occurrence.id.clone());
                        let available: Vec<_> = file
                            .targets
                            .iter()
                            .filter(|t| {
                                t.status == "ready"
                                    && occurrence.target_sheet_ids.contains(&t.sheet_id)
                            })
                            .collect();
                        let selectable =
                            matches!(occurrence.status.as_str(), "ready" | "ambiguous")
                                && !available.is_empty();
                        ui.push_id(&key, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let mut selected = batch.selections.contains_key(&key);
                                if ui
                                    .add_enabled(
                                        selectable && available.len() == 1,
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
                                    if selected {
                                        batch
                                            .selections
                                            .insert(key.clone(), available[0].sheet_id.clone());
                                    } else {
                                        batch.selections.remove(&key);
                                    }
                                    changed = true;
                                }
                                ui.small(status_text(lang, &occurrence.status));
                                if selectable {
                                    let selected_id =
                                        batch.selections.get(&key).cloned().unwrap_or_default();
                                    let label = available
                                        .iter()
                                        .find(|t| t.sheet_id == selected_id)
                                        .map_or_else(
                                            || tr(lang, "plan_links.choose_target").to_owned(),
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
                                            if ui
                                                .selectable_label(
                                                    selected_id.is_empty(),
                                                    tr(lang, "plan_links.batch.exclude"),
                                                )
                                                .clicked()
                                            {
                                                batch.selections.remove(&key);
                                                changed = true;
                                            }
                                            for target in &available {
                                                if ui
                                                    .selectable_label(
                                                        selected_id == target.sheet_id,
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
                                                    batch.selections.insert(
                                                        key.clone(),
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
                    if let Some(row) = batch
                        .rows
                        .iter_mut()
                        .find(|r| r.file_id == file.source_file_id)
                    {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr(lang, "plan_links.batch.output"));
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut row.output_name)
                                        .char_limit(220)
                                        .desired_width(260.0),
                                )
                                .changed();
                        });
                    }
                });
                if count == 0 {
                    ui.small(tr(lang, "plan_links.batch.excluded_file"));
                }
            },
        );
    }
    if changed {
        batch.clear_preview();
    }
}

fn render_candidate(
    ui: &mut egui::Ui,
    batch: &mut BatchState,
    context: &Context,
    lang: Language,
    ready: bool,
    action: &mut Option<BatchAction>,
) {
    let previous_page = batch.preview_page;
    let previous_target = batch.target_sheet_id.clone();
    let Some((_, files)) = &batch.candidate else {
        return;
    };
    ui.separator();
    ui.label(tr(lang, "plan_links.batch.validated"));
    ui.small(tr(lang, "plan_links.batch.preview_help"));
    for file in files {
        ui.small(format!(
            "{} → {} · {} {}",
            source_label(context, &file.source_file_id),
            file.output.display(),
            file.selected_occurrences,
            tr(lang, "plan_links.approved")
        ));
    }
    ui.add_enabled_ui(ready, |ui| {
        ui.horizontal_wrapped(|ui| {
            let old_source = batch.preview_source;
            egui::ComboBox::from_id_salt("batch-preview-source")
                .selected_text(
                    files
                        .get(batch.preview_source)
                        .map_or("", |f| source_label(context, &f.source_file_id)),
                )
                .show_ui(ui, |ui| {
                    for (index, file) in files.iter().enumerate() {
                        ui.selectable_value(
                            &mut batch.preview_source,
                            index,
                            source_label(context, &file.source_file_id),
                        );
                    }
                });
            if old_source != batch.preview_source {
                batch.picture = None;
                batch.target_sheet_id.clear();
                batch.preview_page = files
                    .get(batch.preview_source)
                    .and_then(|f| f.preview_pages.first())
                    .copied()
                    .unwrap_or(1);
            }
            let Some(file) = files.get(batch.preview_source) else {
                return;
            };
            egui::ComboBox::from_id_salt("batch-preview-page")
                .selected_text(batch.preview_page.to_string())
                .show_ui(ui, |ui| {
                    for page in &file.preview_pages {
                        ui.selectable_value(&mut batch.preview_page, *page, page.to_string());
                    }
                });
            if ui.button(tr(lang, "plan_links.source_preview")).clicked() {
                *action = Some(BatchAction::Preview(false));
            }
            let selected_targets: HashSet<_> = batch
                .selections
                .iter()
                .filter(|((source, _), _)| source == &file.source_file_id)
                .map(|(_, target)| target)
                .collect();
            egui::ComboBox::from_id_salt("batch-preview-target")
                .selected_text(
                    context
                        .set
                        .sheets
                        .iter()
                        .find(|s| s.id == batch.target_sheet_id)
                        .map_or(tr(lang, "plan_links.choose_target"), |s| s.number.as_str()),
                )
                .show_ui(ui, |ui| {
                    for sheet in context
                        .set
                        .sheets
                        .iter()
                        .filter(|s| selected_targets.contains(&s.id))
                    {
                        ui.selectable_value(
                            &mut batch.target_sheet_id,
                            sheet.id.clone(),
                            format!("{} · {} · {}", sheet.number, sheet.title, sheet.revision),
                        );
                    }
                });
            if ui
                .add_enabled(
                    !batch.target_sheet_id.is_empty(),
                    egui::Button::new(tr(lang, "plan_links.target_preview")),
                )
                .clicked()
            {
                *action = Some(BatchAction::Preview(true));
            }
        });
    });
    batch.invalidate_changed_preview(previous_page, &previous_target);
}

fn render_picture(ui: &mut egui::Ui, batch: &BatchState, lang: Language) {
    let Some((identity, picture)) = &batch.picture else {
        return;
    };
    ui.label(tr(
        lang,
        if picture.target {
            "plan_links.target_preview"
        } else {
            "plan_links.source_preview"
        },
    ));
    ui.small(format!(
        "{}: {}",
        tr(lang, "plan_links.source"),
        identity.source_filename
    ));
    ui.label(format!(
        "{} · {} {}",
        identity.rendered_filename,
        tr(lang, "plan_links.page"),
        picture.page
    ));
    if let (Some(id), Some(number)) = (&identity.target_sheet_id, &identity.target_sheet_number) {
        ui.small(format!("{number} · {id}"));
    }
    let source = &identity.source_file_id;
    let natural = picture.texture.size_vec2();
    let factor = (ui.available_width() / natural.x)
        .min(390.0 / natural.y)
        .min(1.0);
    let response = ui.add(egui::Image::new(&picture.texture).fit_to_exact_size(natural * factor));
    if picture.target {
        return;
    }
    let Some(file) = batch
        .inspection
        .as_ref()
        .and_then(|i| i.files.iter().find(|f| &f.source_file_id == source))
    else {
        return;
    };
    for occurrence in file.occurrences.iter().filter(|o| {
        o.page == picture.page
            && batch
                .selections
                .contains_key(&(source.clone(), o.id.clone()))
    }) {
        let Some([left, bottom, right, top]) = occurrence.rect else {
            continue;
        };
        let rect = egui::Rect::from_min_max(
            response.rect.min
                + egui::vec2(
                    left / picture.points.0 * response.rect.width(),
                    (1.0 - top / picture.points.1) * response.rect.height(),
                ),
            response.rect.min
                + egui::vec2(
                    right / picture.points.0 * response.rect.width(),
                    (1.0 - bottom / picture.points.1) * response.rect.height(),
                ),
        );
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 110, 210)),
            egui::StrokeKind::Outside,
        );
    }
}

fn render_outcome(
    ui: &mut egui::Ui,
    outcome: Option<&PlanLinkBatchOutcome>,
    lang: Language,
    ready: bool,
    action: &mut Option<BatchAction>,
) {
    let Some(outcome) = outcome else {
        return;
    };
    ui.separator();
    ui.heading(tr(
        lang,
        match outcome.status {
            PlanLinkBatchStatus::Complete => "plan_links.batch.complete",
            PlanLinkBatchStatus::Failed => "plan_links.batch.failed",
            PlanLinkBatchStatus::Cancelled => "plan_links.batch.cancelled",
        },
    ));
    ui.label(tr(lang, "plan_links.batch.receipt_help"));
    if ui
        .add_enabled(
            ready,
            egui::Button::new(tr(lang, "plan_links.export_receipt")),
        )
        .clicked()
    {
        *action = Some(BatchAction::ExportReceipt);
    }
    for receipt in &outcome.completed {
        ui.collapsing(
            format!(
                "{} · {}",
                tr(lang, "plan_links.saved"),
                receipt.output.display()
            ),
            |ui| {
                ui.label(receipt.output.display().to_string());
                ui.small(format!("SHA-256: {}", receipt.report.output_sha256));
                ui.small(format!(
                    "{}: {}",
                    tr(lang, "plan_links.approved"),
                    receipt.report.created.len()
                ));
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(tr(lang, "plan_links.open_saved_copy")),
                    )
                    .clicked()
                {
                    *action = Some(BatchAction::OpenSavedCopy(SavedCopy::from(receipt)));
                }
            },
        );
    }
    if let Some(failure) = &outcome.failure {
        ui.colored_label(ui.visuals().error_fg_color, &failure.reason);
    }
    for file in &outcome.remaining {
        ui.small(format!(
            "{}: {}",
            tr(lang, "plan_links.batch.remaining"),
            file.output.display()
        ));
    }
    if let Some(validation) = &outcome.last_input_validation {
        ui.label(tr(
            lang,
            if validation.valid {
                "plan_links.batch.inputs_valid"
            } else {
                "plan_links.batch.inputs_changed"
            },
        ));
        if let Some(reason) = &validation.reason {
            ui.small(reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::test_app;
    use crate::plan_links_ui::tests::{context, inspection};

    fn reviewed() -> (Context, BatchState) {
        let context = context();
        let mut batch = BatchState::default();
        batch.load(&context);
        for row in &mut batch.rows {
            row.included = true;
            row.targets.insert(
                if row.file_id == "source" {
                    "target-sheet"
                } else {
                    "source-sheet"
                }
                .into(),
            );
        }
        let first = inspection();
        let mut second = first.clone();
        second.source_file_id = "target".into();
        second.targets[0].sheet_id = "source-sheet".into();
        second.targets[0].file_id = "source".into();
        second.occurrences[0].target_sheet_ids = vec!["source-sheet".into()];
        batch.inspection = Some(PlanLinkBatchInspection {
            version: 1,
            manifest_sha256: first.manifest_sha256.clone(),
            files: vec![first, second],
            scanned_pages: 2,
            occurrences: 2,
            scan_complete: true,
            stop_reason: None,
            remaining_sources: vec![],
        });
        (context, batch)
    }

    #[test]
    fn batch_review_scopes_identical_occurrence_ids_and_excludes_unapproved_sources() {
        let (context, mut batch) = reviewed();
        assert!(
            batch.review(&context).is_none(),
            "inspection never approves references"
        );
        batch.selections.insert(
            ("source".into(), "reference-1".into()),
            "target-sheet".into(),
        );
        let review = batch.review(&context).unwrap();
        assert_eq!(review.files.len(), 1);
        assert_eq!(review.files[0].review.scan.source_file_id, "source");
        assert_eq!(review.files[0].output_name, "index-linked.pdf");
        assert!(review.files[0].review.selections[0].confirmed);
        batch.selections.insert(
            ("target".into(), "reference-1".into()),
            "source-sheet".into(),
        );
        let review = batch.review(&context).unwrap();
        assert_eq!(review.files.len(), 2);
        assert_eq!(review.files[1].review.scan.source_file_id, "target");
        assert_eq!(
            review.files[1].review.selections[0].target_sheet_id,
            "source-sheet"
        );
        assert_eq!(
            review.files[0].review.selections[0].target_sheet_id,
            "target-sheet"
        );
        batch.selections.insert(
            ("target".into(), "reference-1".into()),
            "target-sheet".into(),
        );
        assert_eq!(
            batch.review(&context).unwrap().files.len(),
            1,
            "invalid target never becomes confirmed"
        );
    }

    #[test]
    fn batch_source_targets_and_limits_are_explicit() {
        let context = context();
        let mut batch = BatchState::default();
        batch.load(&context);
        assert!(batch
            .rows
            .iter()
            .all(|r| !r.included && r.targets.is_empty()));
        assert!(batch.scan(&context).is_err());
        batch.rows[0].included = true;
        batch.rows[0].targets.insert("source-sheet".into());
        assert!(
            batch.scan(&context).is_err(),
            "same-file targets must not be silently removed"
        );
        batch.rows[0].targets = HashSet::from(["target-sheet".into()]);
        for invalid in ["", "0", "1,1", "1,x", "1,"] {
            batch.rows[0].pages = invalid.into();
            assert!(batch.scan(&context).is_err());
        }
        batch.rows[0].pages = "1".into();
        assert!(batch.scan(&context).is_ok());
        let row = batch.rows[0].clone();
        batch.rows = (0..=core::MAX_BATCH_SOURCES).map(|_| row.clone()).collect();
        assert!(batch.scan(&context).is_err());
        batch.rows = (0..5)
            .map(|_| {
                let mut row = row.clone();
                row.pages = (1..=32)
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                row
            })
            .collect();
        assert!(batch.scan(&context).is_err());
    }

    #[test]
    fn batch_range_validation_reports_source_and_total_limits_without_approving_links() {
        let mut context = context();
        let template = context.set.files[0].clone();
        context.set.files[0].page_count = 40;
        for index in 2..6 {
            let mut file = template.clone();
            file.id = format!("source-{index}");
            file.path = format!("source-{index}.pdf");
            file.page_count = 40;
            context.set.files.push(file);
        }
        let mut batch = BatchState::default();
        batch.load(&context);
        assert_eq!(batch.scan(&context).unwrap_err(), SelectionError::Sources);
        for row in batch.rows.iter_mut().filter(|row| row.file_id != "target") {
            row.included = true;
            row.pages = "1-32".into();
            row.targets.insert("target-sheet".into());
        }
        assert_eq!(
            batch.scan(&context).unwrap_err(),
            SelectionError::TotalPages
        );
        batch.rows.last_mut().unwrap().included = false;
        let scan = batch.scan(&context).unwrap();
        assert_eq!(
            scan.sources.iter().map(|s| s.pages.len()).sum::<usize>(),
            128
        );
        assert!(batch.selections.is_empty() && batch.review(&context).is_none());
        batch.rows[0].pages = "1-33".into();
        assert_eq!(batch.scan(&context).unwrap_err(), SelectionError::PageLimit);
        batch.rows[0].pages = "41".into();
        assert_eq!(batch.scan(&context).unwrap_err(), SelectionError::Pages);
        batch.rows[0].pages = "1".into();
        batch.rows[0].targets.clear();
        assert_eq!(batch.scan(&context).unwrap_err(), SelectionError::Targets);
    }

    #[test]
    fn explicit_ambiguous_resolution_is_accepted_without_hiding_partial_inspection() {
        let (context, mut batch) = reviewed();
        let report = batch.inspection.as_mut().unwrap();
        report.files[0].occurrences[0].status = "ambiguous".into();
        report.scan_complete = false;
        report.stop_reason = Some("occurrence budget reached".into());
        assert!(!report.complete());
        batch.selections.insert(
            ("source".into(), "reference-1".into()),
            "target-sheet".into(),
        );
        assert_eq!(batch.review(&context).unwrap().files.len(), 1);
        assert!(!batch.inspection.as_ref().unwrap().complete());
    }

    fn outcome() -> PlanLinkBatchOutcome {
        PlanLinkBatchOutcome {
            version: 1,
            status: PlanLinkBatchStatus::Cancelled,
            completed: vec![PlanLinkReceipt {
                output: "index-linked.pdf".into(),
                bytes: 100,
                report: PlanLinkReport {
                    version: 1,
                    inspection: inspection(),
                    created: vec![],
                    excluded_occurrences: vec![],
                    output_sha256: "d".repeat(64),
                },
            }],
            failure: None,
            remaining: vec![PlanLinkBatchFilePlan {
                source_file_id: "target".into(),
                output: "floor-linked.pdf".into(),
                selected_occurrences: 1,
                preview_pages: vec![1],
            }],
            last_input_validation: Some(core::PlanLinkBatchInputValidation {
                valid: true,
                reason: None,
            }),
        }
    }

    #[test]
    fn batch_close_keeps_generation_and_committed_receipts_after_cancellation() {
        let (mut app, requests, _) = test_app();
        let (context, batch) = reviewed();
        app.plan_links.context = Some(context);
        app.plan_links.batch = batch;
        app.plan_links.batch_mode = true;
        app.plan_links.open = true;
        app.plan_links.generation = 77;
        app.plan_links.pending = Some((77, true));
        app.plan_links.passwords.push((
            "source".into(),
            Zeroizing::new(["temporary", "credential"].join("-")),
        ));
        app.close_plan_links();
        assert!(app.plan_links.busy() && app.plan_links.open);
        assert!(app.plan_links.cancel.load(Ordering::Relaxed));
        assert_eq!(app.plan_links.generation, 77);
        assert!(app.plan_links.passwords[0].1.is_empty());
        assert!(requests.try_iter().next().is_none());
        app.on_plan_links_progress(
            76,
            PlanLinkBatchProgress {
                completed: 1,
                total: 2,
                source_file_id: "source".into(),
                output: "wrong.pdf".into(),
            },
        );
        assert!(app.plan_links.batch.progress.is_none());
        app.on_plan_links_progress(
            77,
            PlanLinkBatchProgress {
                completed: 1,
                total: 2,
                source_file_id: "source".into(),
                output: "index-linked.pdf".into(),
            },
        );
        assert_eq!(app.plan_links.batch.progress.as_ref().unwrap().completed, 1);
        app.on_plan_links(
            &egui::Context::default(),
            76,
            Ok(Outcome::BatchSaved(outcome())),
        );
        assert!(app.plan_links.busy() && app.plan_links.batch.outcome.is_none());
        app.on_plan_links(
            &egui::Context::default(),
            77,
            Ok(Outcome::BatchSaved(outcome())),
        );
        assert!(!app.plan_links.busy());
        let saved = app.plan_links.batch.outcome.as_ref().unwrap();
        assert_eq!(saved.status, PlanLinkBatchStatus::Cancelled);
        assert_eq!(saved.completed[0].output, PathBuf::from("index-linked.pdf"));
        assert_eq!(saved.completed[0].report.output_sha256, "d".repeat(64));
        assert_eq!(saved.remaining[0].output, PathBuf::from("floor-linked.pdf"));
        app.close_plan_links();
        assert!(!app.plan_links.open);
        assert_eq!(
            app.plan_links
                .batch
                .outcome
                .as_ref()
                .unwrap()
                .completed
                .len(),
            1
        );
        app.open_plan_links();
        assert_eq!(
            app.plan_links
                .batch
                .outcome
                .as_ref()
                .unwrap()
                .completed
                .len(),
            1
        );
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
    fn changing_preview_destination_or_page_removes_old_raster_and_rejects_late_results() {
        let (mut app, _, _) = test_app();
        let (mut context, mut batch) = reviewed();
        let mut other_file = context.set.files[1].clone();
        other_file.id = "other-target".into();
        other_file.path = "other-floor.pdf".into();
        context.set.files.push(other_file);
        let mut other_sheet = context.set.sheets[1].clone();
        other_sheet.id = "other-sheet".into();
        other_sheet.file_id = "other-target".into();
        other_sheet.number = "B-20".into();
        other_sheet.page = 2;
        context.set.sheets.push(other_sheet);
        batch.candidate = Some((
            9,
            vec![PlanLinkBatchFilePlan {
                source_file_id: "source".into(),
                output: "index-linked.pdf".into(),
                selected_occurrences: 1,
                preview_pages: vec![1, 2],
            }],
        ));
        batch.preview_page = 1;
        batch.target_sheet_id = "target-sheet".into();
        app.plan_links.context = Some(context);
        app.plan_links.batch = batch;
        let ctx = egui::Context::default();
        let preview = || Preview {
            image: RenderedPage {
                page_index: 0,
                width: 1,
                height: 1,
                rgba: vec![255; 4],
            },
            points: (1.0, 1.0),
        };
        let result = |sheet: &str, page| Outcome::BatchPreview {
            token: 9,
            source_file_id: "source".into(),
            page,
            target_sheet_id: Some(sheet.into()),
            preview: preview(),
        };
        app.on_plan_links_batch(&ctx, result("target-sheet", 1));
        let (identity, picture) = app.plan_links.batch.picture.as_ref().unwrap();
        assert_eq!(identity.source_filename, "index.pdf");
        assert_eq!(identity.rendered_filename, "floor.pdf");
        assert_eq!(identity.target_sheet_id.as_deref(), Some("target-sheet"));
        assert_eq!(picture.page, 1);

        // The selector changes before the next destination has been rendered.
        app.plan_links.batch.target_sheet_id = "other-sheet".into();
        app.plan_links
            .batch
            .invalidate_changed_preview(1, "target-sheet");
        assert!(app.plan_links.batch.picture.is_none());
        app.on_plan_links_batch(&ctx, result("target-sheet", 1));
        assert!(
            app.plan_links.batch.picture.is_none(),
            "late destination A cannot appear under destination B"
        );
        app.on_plan_links_batch(&ctx, result("other-sheet", 2));
        let (identity, picture) = app.plan_links.batch.picture.as_ref().unwrap();
        assert_eq!(identity.rendered_filename, "other-floor.pdf");
        assert_eq!(identity.target_sheet_number.as_deref(), Some("B-20"));
        assert_eq!(picture.page, 2);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            render_picture(ui, &app.plan_links.batch, Language::English)
        });
        output.textures_delta.clear();
        assert!(output
            .shapes
            .iter()
            .any(|s| text_bounds(&s.shape, "other-floor.pdf · Page 2").is_some()));
        assert!(output
            .shapes
            .iter()
            .any(|s| text_bounds(&s.shape, "B-20 · other-sheet").is_some()));

        app.plan_links.batch.preview_page = 2;
        app.plan_links
            .batch
            .invalidate_changed_preview(1, "other-sheet");
        assert!(app.plan_links.batch.picture.is_none());
        app.on_plan_links_batch(
            &ctx,
            Outcome::BatchPreview {
                token: 9,
                source_file_id: "source".into(),
                page: 1,
                target_sheet_id: None,
                preview: preview(),
            },
        );
        assert!(
            app.plan_links.batch.picture.is_none(),
            "late source page cannot replace the selected page"
        );
    }

    #[test]
    fn batch_footer_remains_reachable_in_both_languages() {
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            let (context, batch) = reviewed();
            app.settings.set_language(lang);
            app.plan_links.context = Some(context);
            app.plan_links.batch = batch;
            app.plan_links.batch_mode = true;
            app.plan_links.open = true;
            let ctx = egui::Context::default();
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
                        "plan_links.batch.title",
                        "plan_links.batch.prepare",
                        "plan_links.batch.save",
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
            assert!(
                requests.try_iter().next().is_none(),
                "opening batch review never writes a file"
            );
        }
    }
}
