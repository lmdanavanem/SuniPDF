//! Reviewed, read-only spreadsheet snapshots. Candidate bytes remain on the worker.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    issue_report::SavedReportSource,
    quantity_table::{FormulaValue, QuantityOptions, QuantityTable},
    quantity_workbook::{
        PreparedQuantityWorkbook, QuantityWorkbookManifest, QuantityWorkbookModel,
        QuantityWorkbookOptions, WorkbookBaselineRow, WorkbookFormat, WorkbookPhase,
        WorkbookProgress, WorkbookReceipt, MAX_MANIFEST_BYTES,
    },
    Document, EngineError,
};
use sunipdf_i18n::{tr, Language};

pub enum Job {
    Prepare {
        table: Option<Arc<QuantityTable>>,
        options: Box<QuantityWorkbookOptions>,
        previous: Option<PathBuf>,
        include_new: bool,
        cancel: Arc<AtomicBool>,
    },
    Publish {
        token: u64,
        format: WorkbookFormat,
        out: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Discard,
}
pub enum Outcome {
    Preview {
        token: u64,
        model: Arc<QuantityWorkbookModel>,
        options: Box<QuantityWorkbookOptions>,
    },
    Published(Box<WorkbookReceipt>),
    Discarded,
}
#[derive(Default)]
pub struct WorkerCache {
    next_token: u64,
    prepared: Option<(u64, u64, PreparedQuantityWorkbook)>,
}
impl WorkerCache {
    pub fn clear(&mut self) {
        self.prepared = None;
    }
    fn candidate(&self, epoch: u64, token: u64) -> Result<&PreparedQuantityWorkbook, EngineError> {
        self.prepared
            .as_ref()
            .filter(|(e, t, _)| *e == epoch && *t == token)
            .map(|(_, _, p)| p)
            .ok_or_else(|| EngineError::Other("Workbook preview is stale; prepare it again".into()))
    }
    pub fn run(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        job: Job,
        saved_sha: Option<&str>,
        mut progress: impl FnMut(WorkbookProgress),
    ) -> Result<Outcome, EngineError> {
        match job {
            Job::Discard => {
                self.clear();
                Ok(Outcome::Discarded)
            }
            Job::Prepare {
                table,
                options,
                previous,
                include_new,
                cancel,
            } => {
                self.clear();
                let previous = previous
                    .as_deref()
                    .map(|p| load_manifest(p, &cancel))
                    .transpose()?;
                let options = previous.as_ref().map_or(*options, |m| m.options.clone());
                if cancel.load(Ordering::Relaxed) {
                    return Err(EngineError::Cancelled);
                }
                let table = match table {
                    Some(table) => table,
                    None => Arc::new(doc.quantity_table()?),
                };
                let source = saved_sha
                    .map(|sha| {
                        SavedReportSource::capture(&doc.info().path, Some(sha), &cancel)
                            .map(|(s, _)| s)
                    })
                    .transpose()?;
                let prepared = doc.prepare_quantity_workbook(
                    &table,
                    &options,
                    previous.as_ref(),
                    include_new,
                    source.as_ref(),
                    &cancel,
                    &mut progress,
                )?;
                let model = Arc::new(prepared.model().clone());
                self.next_token = self.next_token.wrapping_add(1);
                let token = self.next_token;
                self.prepared = Some((epoch, token, prepared));
                Ok(Outcome::Preview {
                    token,
                    model,
                    options: Box::new(options),
                })
            }
            Job::Publish {
                token,
                format,
                out,
                cancel,
            } => {
                let receipt = doc.publish_quantity_workbook(
                    self.candidate(epoch, token)?,
                    format,
                    &out,
                    &cancel,
                )?;
                Ok(Outcome::Published(Box::new(receipt)))
            }
        }
    }
}

fn load_manifest(
    path: &Path,
    cancel: &AtomicBool,
) -> Result<QuantityWorkbookManifest, EngineError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(EngineError::Cancelled);
    }
    if !std::fs::metadata(path)?.is_file() {
        return Err(EngineError::Other(
            "Workbook manifest must be a regular file".into(),
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(EngineError::Other(
            "Workbook manifest exceeds its size limit or is not a regular file".into(),
        ));
    }
    let mut reader = file.take(MAX_MANIFEST_BYTES as u64 + 1);
    let mut bytes = Vec::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(EngineError::Cancelled);
        }
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(EngineError::Other(
                "Workbook manifest exceeds its size limit".into(),
            ));
        }
    }
    QuantityWorkbookManifest::from_json(&bytes)
}

#[derive(Default)]
pub struct QuantityWorkbookState {
    pub open: bool,
    epoch: u64,
    generation: u64,
    pending: Option<(u64, u64)>,
    table: Option<Arc<QuantityTable>>,
    options: QuantityWorkbookOptions,
    initial_options: QuantityWorkbookOptions,
    previous: Option<PathBuf>,
    include_new: bool,
    token: Option<u64>,
    model: Option<Arc<QuantityWorkbookModel>>,
    offset: usize,
    format: usize,
    reviewed: bool,
    progress: Option<WorkbookProgress>,
    cancel: Arc<AtomicBool>,
    close_after: bool,
    error: Option<String>,
    message: Option<String>,
}
impl QuantityWorkbookState {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn invalidate_preview(&mut self) {
        self.token = None;
        self.model = None;
        self.reviewed = false;
        self.offset = 0;
    }
    fn can_export(&self) -> bool {
        !self.busy() && self.token.is_some() && self.model.is_some() && self.reviewed
    }
    fn start(&mut self) -> (u64, u64) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some((self.epoch, self.generation));
        self.cancel = Arc::new(AtomicBool::new(false));
        self.error = None;
        self.message = None;
        self.progress = None;
        (self.epoch, self.generation)
    }
    fn set_previous(&mut self, path: Option<PathBuf>) {
        self.previous = path;
        self.include_new = false;
        self.options = self.initial_options.clone();
        self.invalidate_preview();
    }
}

impl SuniApp {
    pub(crate) fn open_quantity_workbook(
        &mut self,
        table: Option<Arc<QuantityTable>>,
        quantities: QuantityOptions,
    ) {
        if self.document.is_none()
            || !matches!(self.engine, EngineState::Ready)
            || self.worker.has_pending_mutations()
            || self.saving
            || self.ink_blocks_action()
            || self.annotate.quantities.blocks_document()
            || self.annotate.register.dialog_open()
            || self.annotate.measure.is_active_draft()
            || self.issue_report.open
            || self.issue_report.busy()
        {
            return;
        }
        self.annotate.register.open = false;
        self.annotate.quantities.open = false;
        let options = QuantityWorkbookOptions {
            title: tr(self.lang(), "quantity_workbook.default_title").into(),
            quantities,
        };
        self.quantity_workbook = QuantityWorkbookState {
            open: true,
            epoch: self.view.epoch,
            generation: self.quantity_workbook.generation.wrapping_add(1),
            table,
            options: options.clone(),
            initial_options: options,
            ..Default::default()
        };
    }
    pub(crate) fn open_current_quantity_workbook(&mut self) {
        let (table, options) = if self.annotate.quantities.open {
            self.annotate.quantities.workbook_selection()
        } else {
            (None, QuantityOptions::default())
        };
        self.open_quantity_workbook(table, options);
    }
    pub(crate) fn close_quantity_workbook(&mut self) {
        if self.quantity_workbook.busy() {
            self.quantity_workbook.cancel.store(true, Ordering::Relaxed);
            self.quantity_workbook.close_after = true;
            return;
        }
        self.quantity_workbook.generation = self.quantity_workbook.generation.wrapping_add(1);
        self.quantity_workbook.open = false;
        self.quantity_workbook.invalidate_preview();
        self.worker.send(Request::QuantityWorkbook {
            epoch: self.view.epoch,
            generation: self.quantity_workbook.generation,
            job: Job::Discard,
        });
    }
    pub(crate) fn on_quantity_workbook_progress(
        &mut self,
        epoch: u64,
        generation: u64,
        progress: WorkbookProgress,
    ) {
        if self.quantity_workbook.pending == Some((epoch, generation)) && epoch == self.view.epoch {
            self.quantity_workbook.progress = Some(progress);
        }
    }
    pub(crate) fn on_quantity_workbook(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        let lang = self.lang();
        let state = &mut self.quantity_workbook;
        if !state.open || state.pending != Some((epoch, generation)) {
            return;
        }
        state.pending = None;
        state.progress = None;
        if let Ok(Outcome::Published(receipt)) = &result {
            let text = format!(
                "{} {}",
                tr(lang, "quantity_workbook.saved"),
                receipt.out.display()
            );
            self.status_msg = Some(text.clone());
            state.message = Some(text);
        }
        if epoch != self.view.epoch || state.close_after {
            self.close_quantity_workbook();
            return;
        }
        match result {
            Ok(Outcome::Preview {
                token,
                model,
                options,
            }) => {
                state.token = Some(token);
                state.model = Some(model);
                state.options = *options;
            }
            Ok(Outcome::Published(_) | Outcome::Discarded) => {}
            Err(error) => {
                state.invalidate_preview();
                state.error = Some(error.to_string());
            }
        }
    }
    fn prepare_quantity_workbook(&mut self) {
        let state = &mut self.quantity_workbook;
        if state.busy() {
            return;
        }
        state.invalidate_preview();
        let (epoch, generation) = state.start();
        self.worker.send(Request::QuantityWorkbook {
            epoch,
            generation,
            job: Job::Prepare {
                table: state.table.clone(),
                options: Box::new(state.options.clone()),
                previous: state.previous.clone(),
                include_new: state.include_new,
                cancel: state.cancel.clone(),
            },
        });
    }
    pub(crate) fn publish_quantity_workbook_path(&mut self, out: PathBuf) {
        let state = &mut self.quantity_workbook;
        if !state.can_export() {
            return;
        }
        let token = state.token.unwrap();
        let format = if state.format == 0 {
            WorkbookFormat::Xlsx
        } else {
            WorkbookFormat::Manifest
        };
        let (epoch, generation) = state.start();
        self.worker.send(Request::QuantityWorkbook {
            epoch,
            generation,
            job: Job::Publish {
                token,
                format,
                out,
                cancel: state.cancel.clone(),
            },
        });
    }
    pub(crate) fn quantity_workbook_dialog(&mut self, ctx: &egui::Context) {
        if !self.quantity_workbook.open {
            return;
        }
        if self.quantity_workbook.epoch != self.view.epoch {
            self.close_quantity_workbook();
            return;
        }
        let lang = self.lang();
        let (mut prepare, mut close, mut export, mut choose, mut remove) =
            (false, false, false, false, false);
        let state = &mut self.quantity_workbook;
        let idle = !state.busy();
        let can_export = state.can_export();
        dialog::show(
            ctx,
            Spec::new(
                "quantity-workbook",
                tr(lang, "quantity_workbook.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "quantity_workbook.help"))
            .width((ctx.content_rect().width() - 60.).clamp(420., 1000.)),
            |ui| {
                ui.add_enabled_ui(idle, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        choose = ui
                            .button(tr(lang, "quantity_workbook.load_manifest"))
                            .clicked();
                        if let Some(path) = &state.previous {
                            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                            remove = ui
                                .button(tr(lang, "quantity_workbook.new_snapshot"))
                                .clicked();
                        }
                    });
                    let changed = if state.previous.is_some() {
                        ui.label(tr(lang, "quantity_workbook.baseline_help"));
                        ui.checkbox(
                            &mut state.include_new,
                            tr(lang, "quantity_workbook.include_new"),
                        )
                        .changed()
                    } else {
                        ui.label(tr(lang, "quantity_workbook.workbook_title"));
                        ui.add(
                            egui::TextEdit::singleline(&mut state.options.title)
                                .char_limit(512)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    };
                    if changed {
                        state.invalidate_preview();
                    }
                });
                crate::issue_report_ui::selection_ui(ui, lang, &state.options.quantities);
                if let Some(error) = &state.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
                if state.busy() {
                    ui.spinner();
                    if let Some(p) = &state.progress {
                        let phase = match p.phase {
                            WorkbookPhase::Collecting => "collecting",
                            WorkbookPhase::Comparing => "comparing",
                            WorkbookPhase::Packaging => "packaging",
                            WorkbookPhase::Ready => "ready",
                        };
                        ui.label(format!(
                            "{} {}/{}",
                            tr(lang, &format!("quantity_workbook.{phase}")),
                            p.completed,
                            p.total
                        ));
                    }
                }
                if let Some(model) = &state.model {
                    preview_ui(ui, lang, model, &mut state.offset, idle);
                }
                ui.add_enabled_ui(idle && state.token.is_some(), |ui| {
                    ui.checkbox(&mut state.reviewed, tr(lang, "quantity_workbook.reviewed"));
                });
                ui.add_enabled_ui(idle, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "issue_report.format"));
                        egui::ComboBox::from_id_salt("quantity-workbook-format")
                            .selected_text(tr(
                                lang,
                                if state.format == 0 {
                                    "quantity_workbook.xlsx"
                                } else {
                                    "quantity_workbook.manifest"
                                },
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut state.format,
                                    0,
                                    tr(lang, "quantity_workbook.xlsx"),
                                );
                                ui.selectable_value(
                                    &mut state.format,
                                    1,
                                    tr(lang, "quantity_workbook.manifest"),
                                );
                            });
                    });
                    ui.label(tr(lang, "quantity_workbook.manifest_help"));
                });
            },
            |ui| {
                export = ui
                    .add_enabled(
                        can_export,
                        egui::Button::new(tr(lang, "quantity_workbook.export")),
                    )
                    .clicked();
                prepare = ui
                    .add_enabled(
                        idle,
                        egui::Button::new(tr(lang, "quantity_workbook.preview")),
                    )
                    .clicked();
                close = ui
                    .button(tr(
                        lang,
                        if idle {
                            "issue_report.close"
                        } else {
                            "issue_report.cancel"
                        },
                    ))
                    .clicked()
                    || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            },
        );
        if close {
            self.close_quantity_workbook();
            return;
        }
        if choose {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                self.quantity_workbook.set_previous(Some(path));
            }
        }
        if remove {
            self.quantity_workbook.set_previous(None);
        }
        if prepare {
            self.prepare_quantity_workbook();
        }
        if export {
            let extension = if self.quantity_workbook.format == 0 {
                "xlsx"
            } else {
                "json"
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("quantity-workbook.{extension}"))
                .add_filter(extension.to_uppercase(), &[extension])
                .save_file()
            {
                self.publish_quantity_workbook_path(path);
            }
        }
    }
}

fn row_value(row: Option<&WorkbookBaselineRow>) -> String {
    let Some(row) = row else {
        return "—".into();
    };
    let quantity = row
        .row
        .quantity
        .as_ref()
        .map(|q| format!("{} {}", q.value, q.unit.id()))
        .unwrap_or_else(|| "—".into());
    if let Some(cost) = &row.row.line_total {
        format!("{quantity} · {} {}", cost.amount, cost.currency)
    } else {
        quantity
    }
}
fn preview_ui(
    ui: &mut egui::Ui,
    lang: Language,
    model: &QuantityWorkbookModel,
    offset: &mut usize,
    idle: bool,
) {
    ui.separator();
    ui.heading(&model.title);
    ui.label(format!(
        "{}: {} · {}: {}",
        tr(lang, "issue_report.rows"),
        model.quantities["rows"].as_array().map_or(0, Vec::len),
        tr(lang, "quantity_workbook.refreshable"),
        model.refreshable_rows
    ));
    if !model.complete {
        ui.colored_label(
            crate::theme::ERROR,
            tr(lang, "quantity_workbook.incomplete"),
        );
    }
    if model.provenance.source_encrypted {
        ui.label(tr(lang, "quantity_workbook.clear_export"));
    }
    if model.provenance.source_signature_count > 0 {
        ui.label(tr(lang, "quantity_workbook.signatures"));
    }
    ui.label(tr(
        lang,
        if model.provenance.saved_file_sha256.is_some() {
            "issue_report.saved_source"
        } else {
            "issue_report.snapshot_source"
        },
    ));
    ui.collapsing(tr(lang, "quantity_workbook.provenance"), |ui| {
        ui.label(format!(
            "{}: {}",
            tr(lang, "quantity_workbook.snapshot"),
            model.provenance.snapshot_sha256
        ));
        if let Some(sha) = &model.provenance.saved_file_sha256 {
            ui.label(format!(
                "{}: {sha}",
                tr(lang, "quantity_workbook.saved_file")
            ));
        }
        if let Some(sha) = &model.previous_snapshot_sha256 {
            ui.label(format!(
                "{}: {sha}",
                tr(lang, "quantity_workbook.previous_snapshot")
            ));
        }
    });
    for warning in &model.warnings {
        ui.label(warning);
    }
    ui.collapsing(tr(lang, "workspace.table.summary"), |ui| {
        if let Some(groups) = model.quantities["summary"]["groups"].as_array() {
            for group in groups {
                ui.label(format!(
                    "{} · {} {}",
                    group["group"]
                        .as_str()
                        .unwrap_or(tr(lang, "quantity_workbook.all_rows")),
                    group["quantity"],
                    group["unit"].as_str().unwrap_or("")
                ));
                if let Some(costs) = group["costs"].as_array() {
                    for cost in costs {
                        ui.label(format!(
                            "{} {} · {}: {}",
                            cost["total"]["amount"].as_str().unwrap_or("—"),
                            cost["total"]["currency"].as_str().unwrap_or(""),
                            tr(lang, "quantity_workbook.priced_rows"),
                            cost["priced_rows"]
                        ));
                    }
                }
                if group["missing_prices"].as_u64().unwrap_or(0) > 0 {
                    ui.label(format!(
                        "{}: {}",
                        tr(lang, "quantity_workbook.missing_prices"),
                        group["missing_prices"]
                    ));
                }
            }
        }
        if let Some(groups) = model.quantities["summary"]["formula_groups"].as_array() {
            for group in groups {
                ui.strong(group["formula_name"].as_str().unwrap_or("—"));
                if let Some(totals) = group["totals"].as_array() {
                    for total in totals {
                        ui.label(format!(
                            "{} {}",
                            total["amount"].as_str().unwrap_or("—"),
                            total["unit"].as_str().unwrap_or("")
                        ));
                    }
                }
                if group["complete"] != true {
                    ui.colored_label(
                        crate::theme::ERROR,
                        format!(
                            "{}: {}",
                            tr(lang, "quantity_workbook.formula_errors"),
                            group["error_rows"]
                        ),
                    );
                }
            }
        }
        if let Some(counts) = model.quantities["zone_coverage"]["counts"].as_object() {
            let coverage = sunipdf_core::annotation_register::ZoneCoverage {
                complete: model.quantities["zone_coverage"]["complete"] == true,
                counts: counts
                    .iter()
                    .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize)))
                    .collect(),
            };
            crate::plan_zones_ui::coverage_ui(ui, lang, &coverage);
        }
    });
    *offset = (*offset).min(model.changes.len().saturating_sub(1) / 50 * 50);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                idle && *offset > 0,
                egui::Button::new(tr(lang, "issue_report.previous")),
            )
            .clicked()
        {
            *offset = offset.saturating_sub(50);
        }
        ui.label(format!(
            "{}–{} / {}",
            if model.changes.is_empty() {
                0
            } else {
                *offset + 1
            },
            (*offset + 50).min(model.changes.len()),
            model.changes.len()
        ));
        if ui
            .add_enabled(
                idle && *offset + 50 < model.changes.len(),
                egui::Button::new(tr(lang, "issue_report.next")),
            )
            .clicked()
        {
            *offset += 50;
        }
    });
    egui::ScrollArea::horizontal()
        .id_salt("workbook-changes-scroll")
        .show(ui, |ui| {
            egui::Grid::new("workbook-changes")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    for key in [
                        "issue_report.page",
                        "quantity_workbook.identity",
                        "quantity_workbook.status",
                        "quantity_workbook.before",
                        "quantity_workbook.after",
                    ] {
                        ui.strong(tr(lang, key));
                    }
                    ui.end_row();
                    for change in model.changes.iter().skip(*offset).take(50) {
                        let row = change.after.as_ref().or(change.before.as_ref());
                        ui.label(
                            row.map_or_else(|| "—".into(), |r| r.row.register.page.to_string()),
                        );
                        ui.label(change.annotation_id.as_deref().unwrap_or("—"));
                        ui.label(tr(
                            lang,
                            &format!("quantity_workbook.status.{}", change.status.id()),
                        ));
                        ui.push_id(
                            (
                                "before",
                                *offset,
                                change.annotation_id.as_deref(),
                                row.map(|r| (r.row.register.page, r.row.register.index)),
                            ),
                            |ui| {
                                row_preview(ui, lang, change.before.as_ref());
                            },
                        );
                        ui.push_id(
                            (
                                "after",
                                *offset,
                                change.annotation_id.as_deref(),
                                row.map(|r| (r.row.register.page, r.row.register.index)),
                            ),
                            |ui| {
                                row_preview(ui, lang, change.after.as_ref());
                            },
                        );
                        ui.end_row();
                    }
                });
        });
}

fn row_preview(ui: &mut egui::Ui, lang: Language, row: Option<&WorkbookBaselineRow>) {
    ui.vertical(|ui| {
        ui.label(row_value(row));
        let Some(row) = row else {
            return;
        };
        ui.collapsing(tr(lang, "quantity_workbook.details"), |ui| {
            let register = &row.row.register;
            for (key, value) in [
                ("register.author", register.author.as_deref()),
                ("register.contents", register.contents.as_deref()),
                (
                    "register.business_status",
                    register.fields.business_status.as_deref(),
                ),
                ("register.discipline", register.fields.discipline.as_deref()),
                ("register.review_status", register.review_status.as_deref()),
            ] {
                if let Some(value) = value {
                    ui.label(format!("{}: {value}", tr(lang, key)));
                }
            }
            for (name, value) in &register.fields.custom {
                ui.label(format!("{name}: {value}"));
            }
            if let Some(price) = &row.row.fields.unit_price {
                ui.label(format!(
                    "{}: {} {} / {}",
                    tr(lang, "quantity_workbook.unit_price"),
                    price.amount.as_str(),
                    price.currency,
                    price.per_unit.id()
                ));
            }
            field_and_formula_details(ui, lang, &row.row.fields, &row.formulas);
        });
    });
}

fn formula_value_text(lang: Language, value: &FormulaValue) -> String {
    if let Some(error) = value.error {
        return tr(lang, &format!("quantity.formula_error.{}", error.id())).into();
    }
    match (&value.value, value.unit) {
        (Some(value), Some(unit)) => format!("{value} {}", unit.id()),
        (Some(value), None) => value.clone(),
        (None, _) => tr(lang, "quantity.missing").into(),
    }
}

fn field_and_formula_details(
    ui: &mut egui::Ui,
    lang: Language,
    fields: &sunipdf_core::quantity_table::QuantityFields,
    formulas: &std::collections::BTreeMap<String, FormulaValue>,
) {
    for (name, value) in &fields.custom {
        ui.label(format!("{name}: {}", value.display()));
    }
    for (name, value) in formulas {
        ui.label(format!("{name}: {}", formula_value_text(lang, value)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbook_details_keep_custom_and_calculated_units_visible_without_inventing_zero() {
        use std::collections::BTreeMap;
        use sunipdf_core::quantity_table::{
            DecimalValue, FormulaError, QuantityFields, QuantityUnit, TypedValue,
        };
        fn visible(
            shape: &egui::Shape,
            label: &str,
            viewport: egui::Rect,
            clip: egui::Rect,
        ) -> bool {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                    viewport.contains_rect(rect) && clip.contains_rect(rect)
                }
                egui::Shape::Vec(shapes) => shapes
                    .iter()
                    .any(|shape| visible(shape, label, viewport, clip)),
                _ => false,
            }
        }
        let fields = QuantityFields {
            custom: BTreeMap::from([
                (
                    "Depth".into(),
                    TypedValue::Number {
                        value: DecimalValue::parse("0.25").unwrap(),
                        unit: Some(QuantityUnit::Metres),
                    },
                ),
                (
                    "Factor".into(),
                    TypedValue::Number {
                        value: DecimalValue::parse("2").unwrap(),
                        unit: None,
                    },
                ),
            ]),
            ..Default::default()
        };
        let formulas = BTreeMap::from([
            (
                "Area".into(),
                FormulaValue {
                    value: Some("6.00".into()),
                    unit: Some(QuantityUnit::SquareMetres),
                    error: None,
                },
            ),
            (
                "Volume".into(),
                FormulaValue {
                    value: Some("1.50".into()),
                    unit: Some(QuantityUnit::CubicMetres),
                    error: None,
                },
            ),
            (
                "Ratio".into(),
                FormulaValue {
                    value: Some("0.00".into()),
                    unit: None,
                    error: None,
                },
            ),
            (
                "Unavailable".into(),
                FormulaValue {
                    value: None,
                    unit: None,
                    error: Some(FormulaError::MissingField),
                },
            ),
        ]);
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 240.0));
        for lang in Language::ALL {
            assert_eq!(formula_value_text(lang, &formulas["Ratio"]), "0.00");
            assert_eq!(
                formula_value_text(lang, &formulas["Unavailable"]),
                tr(lang, "quantity.formula_error.missing_field")
            );
            assert_eq!(
                formula_value_text(
                    lang,
                    &FormulaValue {
                        value: None,
                        unit: None,
                        error: None
                    }
                ),
                tr(lang, "quantity.missing")
            );
            for dark in [false, true] {
                let ctx = egui::Context::default();
                crate::theme::apply(&ctx, dark);
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(viewport),
                        ..Default::default()
                    },
                    |ui| {
                        field_and_formula_details(ui, lang, &fields, &formulas);
                    },
                );
                for label in [
                    "Depth: 0.25 m",
                    "Factor: 2",
                    "Area: 6.00 m2",
                    "Volume: 1.50 m3",
                    "Ratio: 0.00",
                ] {
                    assert!(
                        output.shapes.iter().any(|shape| visible(
                            &shape.shape,
                            label,
                            viewport,
                            shape.clip_rect
                        )),
                        "Missing or clipped workbook value: {lang:?}, dark={dark}, {label}"
                    );
                }
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn compact_workbook_keeps_actions_and_first_difference_fully_visible() {
        use egui::{Event, Modifiers, Pos2, Rect};
        use sunipdf_core::quantity_workbook::{
            WorkbookChange, WorkbookChangeStatus, WorkbookProvenance,
        };

        fn text_rects(shape: &egui::Shape, label: &str, found: &mut Vec<Rect>) {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    found.push(Rect::from_min_size(text.pos, text.galley.size()));
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_rects(shape, label, found);
                    }
                }
                _ => {}
            }
        }
        fn visible(output: &egui::FullOutput, label: &str, viewport: Rect) -> bool {
            output.shapes.iter().any(|shape| {
                let mut found = Vec::new();
                text_rects(&shape.shape, label, &mut found);
                found
                    .into_iter()
                    .any(|rect| viewport.contains_rect(rect) && shape.clip_rect.contains_rect(rect))
            })
        }
        let viewport = Rect::from_min_size(Pos2::ZERO, egui::vec2(720., 480.));
        let first_id = "SuniPDF-first-row";
        for language in Language::ALL {
            for dark in [false, true] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.settings.set_language(language);
                app.settings.dark_theme = dark;
                app.open_current_quantity_workbook();
                app.quantity_workbook.token = Some(1);
                app.quantity_workbook.model = Some(Arc::new(QuantityWorkbookModel {
                    schema: "sunipdf-quantity-workbook",
                    version: 1,
                    title: "Reviewed quantities".into(),
                    provenance: WorkbookProvenance {
                        snapshot_sha256: "a".repeat(64),
                        saved_file_sha256: Some("b".repeat(64)),
                        source_encrypted: false,
                        source_signature_count: 0,
                    },
                    quantities: serde_json::json!({"rows":[],"summary":{"groups":[],"formula_groups":[]}}),
                    changes: vec![WorkbookChange {
                        annotation_id: Some(first_id.into()),
                        status: WorkbookChangeStatus::ExcludedNew,
                        before: None,
                        after: None,
                    }],
                    refresh: true,
                    include_new: false,
                    previous_snapshot_sha256: Some("c".repeat(64)),
                    complete: true,
                    warnings: vec![],
                    refreshable_rows: 0,
                }));
                let ctx = egui::Context::default();
                crate::theme::apply(&ctx, dark);
                ctx.enable_accesskit();
                let mut frame = 0;
                let mut draw = |app: &mut SuniApp, events| {
                    frame += 1;
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(viewport),
                            time: Some(frame as f64 * 0.1),
                            events,
                            ..Default::default()
                        },
                        |_| app.quantity_workbook_dialog(&ctx),
                    );
                    output.textures_delta.clear();
                    output
                };
                for reviewed in [false, true] {
                    app.quantity_workbook.reviewed = reviewed;
                    let mut output = draw(&mut app, vec![]);
                    for _ in 0..3 {
                        output = draw(&mut app, vec![]);
                    }
                    for attempt in 0..12 {
                        let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                        for key in [
                            "quantity_workbook.export",
                            "quantity_workbook.preview",
                            "issue_report.close",
                        ] {
                            let label = tr(language, key);
                            let button = tree
                                .nodes
                                .iter()
                                .map(|(_, node)| node)
                                .find(|node| node.label() == Some(label))
                                .unwrap_or_else(|| {
                                    panic!("missing action {language:?}, dark={dark}: {key}")
                                });
                            let bounds = button.bounds().unwrap();
                            assert!(bounds.x0 >= 0. && bounds.x1 <= 720. && bounds.y0 >= 0. && bounds.y1 <= 480.,
                                "complete action bounds {language:?}, dark={dark}, {key}: {bounds:?}");
                            assert!(
                                visible(&output, label, viewport),
                                "action text is not clipped: {language:?}, dark={dark}, {key}"
                            );
                            if key == "quantity_workbook.export" {
                                assert_eq!(
                                    button.is_disabled(),
                                    !reviewed,
                                    "export requires review"
                                );
                            }
                        }
                        if visible(&output, first_id, viewport) {
                            assert!(
                                visible(
                                    &output,
                                    tr(language, "quantity_workbook.status.excluded_new"),
                                    viewport
                                ),
                                "the first difference's status must also be visible"
                            );
                            break;
                        }
                        assert!(attempt < 11, "first difference must become fully visible by scrolling: {language:?}, dark={dark}");
                        output = draw(
                            &mut app,
                            vec![
                                Event::PointerMoved(Pos2::new(300., 250.)),
                                Event::MouseWheel {
                                    unit: egui::MouseWheelUnit::Point,
                                    delta: egui::vec2(0., -60.),
                                    phase: egui::TouchPhase::Move,
                                    source: egui::MouseWheelSource::Wheel,
                                    modifiers: Modifiers::NONE,
                                },
                            ],
                        );
                    }
                }
                assert!(
                    requests.try_iter().all(|request| !matches!(
                        request,
                        Request::Edit(_)
                            | Request::QuantityWorkbook {
                                job: Job::Publish { .. },
                                ..
                            }
                    )),
                    "reviewing a compact preview must not write or publish"
                );
            }
        }
    }

    #[test]
    fn cancellation_blocks_close_until_acknowledged_and_stale_preview_cannot_export() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_current_quantity_workbook();
        app.prepare_quantity_workbook();
        let (epoch, generation, cancel) = match requests.recv().unwrap() {
            Request::QuantityWorkbook {
                epoch,
                generation,
                job: Job::Prepare { cancel, .. },
            } => (epoch, generation, cancel),
            _ => panic!("prepare"),
        };
        app.close_quantity_workbook();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(app.quantity_workbook.busy());
        app.on_quantity_workbook(epoch, generation, Err(EngineError::Cancelled));
        assert!(!app.quantity_workbook.open);
        assert!(!app.quantity_workbook.can_export());
        assert!(matches!(
            requests.recv().unwrap(),
            Request::QuantityWorkbook {
                job: Job::Discard,
                ..
            }
        ));
        app.open_current_quantity_workbook();
        app.prepare_quantity_workbook();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::QuantityWorkbook {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("prepare"),
        };
        app.view.epoch = epoch.wrapping_add(1);
        app.on_quantity_workbook(epoch, generation, Err(EngineError::Cancelled));
        assert!(!app.quantity_workbook.open);
        assert!(!app.quantity_workbook.can_export());
    }
    #[test]
    fn changing_baseline_clears_review_and_excludes_new_ids() {
        let mut state = QuantityWorkbookState {
            token: Some(3),
            reviewed: true,
            include_new: true,
            ..Default::default()
        };
        state.set_previous(Some(PathBuf::from("baseline.json")));
        assert!(!state.reviewed && !state.include_new && state.token.is_none());
        state.set_previous(None);
        assert!(state.previous.is_none() && !state.can_export());
    }
    #[test]
    fn loading_a_cancelled_or_non_regular_manifest_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(true);
        assert!(matches!(
            load_manifest(&dir.path().join("absent"), &cancel),
            Err(EngineError::Cancelled)
        ));
        cancel.store(false, Ordering::Relaxed);
        assert!(load_manifest(dir.path(), &cancel).is_err());
    }
}
