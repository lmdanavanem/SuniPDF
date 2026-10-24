//! Read-only reports. Prepared PDF bytes stay on the document worker.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    annotation_register::AnnotationRegister,
    issue_report::{
        IssueReportFormat, IssueReportModel, IssueReportOptions, IssueReportProgress,
        IssueReportReceipt, IssueReportSelection, PreparedIssueReport, SavedReportSource,
    },
    quantity_table::{QuantityOptions, QuantityTable},
    Document, EngineError, RenderedPage,
};
use sunipdf_i18n::{tr, Language};

#[derive(Clone, Default)]
pub enum Input {
    #[default]
    Current,
    Register(Arc<AnnotationRegister>),
    Quantities(Arc<QuantityTable>),
}
pub enum Job {
    Prepare {
        input: Input,
        options: Box<IssueReportOptions>,
        cancel: Arc<AtomicBool>,
    },
    Render {
        token: u64,
        page: usize,
        cancel: Arc<AtomicBool>,
    },
    Publish {
        token: u64,
        format: IssueReportFormat,
        out: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Discard,
}
pub enum Outcome {
    Preview {
        token: u64,
        model: Arc<IssueReportModel>,
        page: usize,
        image: RenderedPage,
    },
    Published(Box<IssueReportReceipt>),
    Discarded,
}
#[derive(Default)]
pub struct WorkerCache {
    next_token: u64,
    prepared: Option<(u64, u64, PreparedIssueReport)>,
}
impl WorkerCache {
    pub fn clear(&mut self) {
        self.prepared = None;
    }
    fn candidate(&self, epoch: u64, token: u64) -> Result<&PreparedIssueReport, EngineError> {
        self.prepared
            .as_ref()
            .filter(|(e, t, _)| *e == epoch && *t == token)
            .map(|(_, _, p)| p)
            .ok_or_else(|| EngineError::Other("Report preview is stale; prepare it again".into()))
    }
    pub fn run(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        job: Job,
        saved_sha: Option<&str>,
        mut progress: impl FnMut(IssueReportProgress),
    ) -> Result<Outcome, EngineError> {
        match job {
            Job::Discard => {
                self.clear();
                Ok(Outcome::Discarded)
            }
            Job::Prepare {
                input,
                mut options,
                cancel,
            } => {
                self.clear();
                if cancel.load(Ordering::Relaxed) {
                    return Err(EngineError::Cancelled);
                }
                let table = match input {
                    Input::Quantities(table) => {
                        options.selection = IssueReportSelection::Quantities;
                        table
                    }
                    Input::Register(register) => {
                        doc.validate_report_register(&register)?;
                        options.selection = IssueReportSelection::Register;
                        Arc::new(doc.quantity_table()?)
                    }
                    Input::Current => {
                        options.selection = IssueReportSelection::Quantities;
                        Arc::new(doc.quantity_table()?)
                    }
                };
                let source = saved_sha
                    .map(|sha| {
                        SavedReportSource::capture(&doc.info().path, Some(sha), &cancel)
                            .map(|(s, _)| s)
                    })
                    .transpose()?;
                let prepared = doc.prepare_issue_report(
                    &table,
                    &options,
                    source.as_ref(),
                    &cancel,
                    &mut progress,
                )?;
                let image = doc.render_issue_report_preview(&prepared, 0, 1000, &cancel)?;
                let model = Arc::new(prepared.model().clone());
                self.next_token = self.next_token.wrapping_add(1);
                let token = self.next_token;
                self.prepared = Some((epoch, token, prepared));
                Ok(Outcome::Preview {
                    token,
                    model,
                    page: 0,
                    image,
                })
            }
            Job::Render {
                token,
                page,
                cancel,
            } => {
                let prepared = self.candidate(epoch, token)?;
                let image = doc.render_issue_report_preview(prepared, page, 1000, &cancel)?;
                Ok(Outcome::Preview {
                    token,
                    model: Arc::new(prepared.model().clone()),
                    page,
                    image,
                })
            }
            Job::Publish {
                token,
                format,
                out,
                cancel,
            } => {
                let receipt =
                    doc.publish_issue_report(self.candidate(epoch, token)?, format, &out, &cancel)?;
                Ok(Outcome::Published(Box::new(receipt)))
            }
        }
    }
}

#[derive(Default)]
pub struct IssueReportState {
    pub open: bool,
    epoch: u64,
    generation: u64,
    pending: Option<(u64, u64)>,
    input: Input,
    options: IssueReportOptions,
    token: Option<u64>,
    model: Option<Arc<IssueReportModel>>,
    texture: Option<egui::TextureHandle>,
    page: usize,
    format: usize,
    reviewed: bool,
    progress: Option<IssueReportProgress>,
    cancel: Arc<AtomicBool>,
    close_after: bool,
    error: Option<String>,
    message: Option<String>,
}
impl IssueReportState {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn invalidate_preview(&mut self) {
        self.token = None;
        self.model = None;
        self.texture = None;
        self.reviewed = false;
        self.page = 0;
    }
    fn can_export(&self) -> bool {
        !self.busy() && self.token.is_some() && self.texture.is_some() && self.reviewed
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
}

impl SuniApp {
    pub(crate) fn open_issue_report(&mut self, input: Input, quantities: QuantityOptions) {
        if self.document.is_none()
            || !matches!(self.engine, EngineState::Ready)
            || self.worker.has_pending_mutations()
            || self.saving
            || self.ink_blocks_action()
            || self.annotate.quantities.blocks_document()
            || self.annotate.register.dialog_open()
            || self.annotate.measure.is_active_draft()
        {
            return;
        }
        self.annotate.register.open = false;
        self.annotate.quantities.open = false;
        let selection = if matches!(&input, Input::Register(_)) {
            IssueReportSelection::Register
        } else {
            IssueReportSelection::Quantities
        };
        self.issue_report = IssueReportState {
            open: true,
            epoch: self.view.epoch,
            generation: self.issue_report.generation.wrapping_add(1),
            input,
            options: IssueReportOptions {
                title: tr(self.lang(), "issue_report.default_title").into(),
                quantities,
                include_legend: true,
                selection,
            },
            ..Default::default()
        };
    }
    pub(crate) fn open_current_issue_report(&mut self) {
        let (input, options) = if self.annotate.quantities.open {
            self.annotate.quantities.issue_report_selection()
        } else if self.annotate.register.open {
            self.annotate.register.issue_report_selection()
        } else {
            (Input::Current, QuantityOptions::default())
        };
        self.open_issue_report(input, options);
    }
    pub(crate) fn close_issue_report(&mut self) {
        if self.issue_report.busy() {
            self.issue_report.cancel.store(true, Ordering::Relaxed);
            self.issue_report.close_after = true;
            return;
        }
        self.issue_report.generation = self.issue_report.generation.wrapping_add(1);
        self.issue_report.open = false;
        self.issue_report.invalidate_preview();
        self.worker.send(Request::IssueReport {
            epoch: self.view.epoch,
            generation: self.issue_report.generation,
            job: Job::Discard,
        });
    }
    pub(crate) fn on_issue_report_progress(
        &mut self,
        epoch: u64,
        generation: u64,
        progress: IssueReportProgress,
    ) {
        if self.issue_report.pending == Some((epoch, generation)) && epoch == self.view.epoch {
            self.issue_report.progress = Some(progress);
        }
    }
    pub(crate) fn on_issue_report(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        let lang = self.lang();
        let state = &mut self.issue_report;
        if !state.open || state.pending != Some((epoch, generation)) {
            return;
        }
        state.pending = None;
        state.progress = None;
        // A successful atomic publication must be reported even when cancellation raced its commit.
        if let Ok(Outcome::Published(receipt)) = &result {
            let text = format!(
                "{} {}",
                tr(lang, "issue_report.saved"),
                receipt.out.display()
            );
            self.status_msg = Some(text.clone());
            state.message = Some(text);
        }
        if epoch != self.view.epoch || state.close_after {
            self.close_issue_report();
            return;
        }
        match result {
            Ok(Outcome::Preview {
                token,
                model,
                page,
                image,
            }) => {
                state.texture = Some(ctx.load_texture(
                    "issue-report-preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width as usize, image.height as usize],
                        &image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ));
                state.token = Some(token);
                state.model = Some(model);
                state.page = page;
            }
            Ok(Outcome::Published(_) | Outcome::Discarded) => {}
            Err(error) => {
                state.invalidate_preview();
                state.error = Some(error.to_string());
            }
        }
    }
    fn prepare_issue_report(&mut self) {
        let state = &mut self.issue_report;
        if state.busy() {
            return;
        }
        state.invalidate_preview();
        let (epoch, generation) = state.start();
        self.worker.send(Request::IssueReport {
            epoch,
            generation,
            job: Job::Prepare {
                input: state.input.clone(),
                options: Box::new(state.options.clone()),
                cancel: state.cancel.clone(),
            },
        });
    }
    fn render_issue_report_page(&mut self, page: usize) {
        let state = &mut self.issue_report;
        let Some(token) = state.token else {
            return;
        };
        if state.busy() || state.model.as_ref().is_none_or(|m| page >= m.page_count) {
            return;
        }
        let (epoch, generation) = state.start();
        self.worker.send(Request::IssueReport {
            epoch,
            generation,
            job: Job::Render {
                token,
                page,
                cancel: state.cancel.clone(),
            },
        });
    }
    pub(crate) fn publish_issue_report_path(&mut self, out: PathBuf) {
        let state = &mut self.issue_report;
        if !state.can_export() {
            return;
        }
        let token = state.token.unwrap();
        let format = [
            IssueReportFormat::Pdf,
            IssueReportFormat::Csv,
            IssueReportFormat::Json,
        ][state.format.min(2)];
        let (epoch, generation) = state.start();
        self.worker.send(Request::IssueReport {
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
    pub(crate) fn issue_report_dialog(&mut self, ctx: &egui::Context) {
        if !self.issue_report.open {
            return;
        }
        if self.issue_report.epoch != self.view.epoch {
            self.close_issue_report();
            return;
        }
        let lang = self.lang();
        let mut prepare = false;
        let mut close = false;
        let mut export = false;
        let mut target_page = None;
        let state = &mut self.issue_report;
        let idle = !state.busy();
        let can_export = state.can_export();
        dialog::show(
            ctx,
            Spec::new(
                "issue-report",
                tr(lang, "issue_report.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "issue_report.help"))
            .icon(crate::icons::glyph::FORM_LIST, "▥")
            .width((ctx.content_rect().width() - 60.).clamp(420., 1080.)),
            |ui| {
                let changed = ui
                    .add_enabled_ui(idle, |ui| {
                        ui.label(tr(lang, "issue_report.report_title"));
                        let title = ui
                            .add(
                                egui::TextEdit::singleline(&mut state.options.title)
                                    .char_limit(512)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed();
                        ui.checkbox(
                            &mut state.options.include_legend,
                            tr(lang, "issue_report.legend"),
                        )
                        .changed()
                            || title
                    })
                    .inner;
                if changed {
                    state.invalidate_preview();
                }
                selection_ui(ui, lang, &state.options.quantities);
                if let Some(error) = &state.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
                if state.busy() {
                    ui.spinner();
                    if let Some(p) = &state.progress {
                        let key = match p.phase {
                            sunipdf_core::issue_report::IssueReportPhase::Collecting => {
                                "collecting"
                            }
                            sunipdf_core::issue_report::IssueReportPhase::Layout => "layout",
                            sunipdf_core::issue_report::IssueReportPhase::Rendering => "rendering",
                            sunipdf_core::issue_report::IssueReportPhase::Ready => "ready",
                        };
                        ui.label(format!(
                            "{} {}/{}",
                            tr(lang, &format!("issue_report.{key}")),
                            p.completed,
                            p.total
                        ));
                    }
                }
                if let Some(model) = &state.model {
                    ui.separator();
                    ui.label(format!(
                        "{}: {} · {}: {}",
                        tr(lang, "issue_report.rows"),
                        model.quantities["rows"].as_array().map_or(0, Vec::len),
                        tr(lang, "issue_report.pages"),
                        model.page_count
                    ));
                    if !model.complete {
                        ui.colored_label(crate::theme::ERROR, tr(lang, "issue_report.incomplete"));
                    }
                    if model.provenance.source_encrypted {
                        ui.label(tr(lang, "issue_report.clear_export"));
                    }
                    if model.provenance.source_signature_count > 0 {
                        ui.label(tr(lang, "issue_report.signatures"));
                    }
                    ui.label(tr(
                        lang,
                        if model.provenance.saved_file_sha256.is_some() {
                            "issue_report.saved_source"
                        } else {
                            "issue_report.snapshot_source"
                        },
                    ));
                    for warning in &model.warnings {
                        ui.label(warning);
                    }
                    if let Some(counts) = model.quantities["zone_coverage"]["counts"].as_object() {
                        let coverage = sunipdf_core::annotation_register::ZoneCoverage {
                            complete: model.quantities["zone_coverage"]["complete"] == true,
                            counts: counts
                                .iter()
                                .filter_map(|(key, value)| {
                                    value.as_u64().map(|n| (key.clone(), n as usize))
                                })
                                .collect(),
                        };
                        crate::plan_zones_ui::coverage_ui(ui, lang, &coverage);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                idle && state.page > 0,
                                egui::Button::new(tr(lang, "issue_report.previous")),
                            )
                            .clicked()
                        {
                            target_page = Some(state.page - 1);
                        }
                        ui.label(format!(
                            "{} {} / {}",
                            tr(lang, "issue_report.page"),
                            state.page + 1,
                            model.page_count
                        ));
                        if ui
                            .add_enabled(
                                idle && state.page + 1 < model.page_count,
                                egui::Button::new(tr(lang, "issue_report.next")),
                            )
                            .clicked()
                        {
                            target_page = Some(state.page + 1);
                        }
                    });
                }
                if let Some(texture) = &state.texture {
                    ui.add(
                        egui::Image::from_texture(texture)
                            .max_width(ui.available_width())
                            .max_height(500.)
                            .maintain_aspect_ratio(true),
                    );
                }
                ui.add_enabled_ui(state.token.is_some() && idle, |ui| {
                    ui.checkbox(&mut state.reviewed, tr(lang, "issue_report.reviewed"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "issue_report.format"));
                    ui.add_enabled_ui(idle, |ui| {
                        egui::ComboBox::from_id_salt("issue-report-format")
                            .selected_text(["PDF", "CSV", "JSON"][state.format.min(2)])
                            .show_ui(ui, |ui| {
                                for (i, label) in ["PDF", "CSV", "JSON"].iter().enumerate() {
                                    ui.selectable_value(&mut state.format, i, *label);
                                }
                            });
                    });
                });
            },
            |ui| {
                export = ui
                    .add_enabled(
                        can_export,
                        egui::Button::new(tr(lang, "issue_report.export")),
                    )
                    .clicked();
                prepare = ui
                    .add_enabled(idle, egui::Button::new(tr(lang, "issue_report.preview")))
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
            self.close_issue_report();
            return;
        }
        if prepare {
            self.prepare_issue_report();
        }
        if let Some(page) = target_page {
            self.render_issue_report_page(page);
        }
        if export {
            let extension = ["pdf", "csv", "json"][self.issue_report.format.min(2)];
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("issue-report.{extension}"))
                .add_filter(extension.to_uppercase(), &[extension])
                .save_file()
            {
                self.publish_issue_report_path(path);
            }
        }
    }
}

fn selection_ui(ui: &mut egui::Ui, lang: Language, options: &QuantityOptions) {
    ui.collapsing(tr(lang, "issue_report.selection"), |ui| {
        ui.label(format!(
            "{}: {} · {}: {} · {}: {}",
            tr(lang, "issue_report.sort"),
            tr(lang, &format!("register.{}", options.sort.id())),
            tr(lang, "issue_report.descending"),
            options.descending,
            tr(lang, "issue_report.group"),
            tr(lang, &format!("quantity.group.{}", options.group_by.id()))
        ));
        let filter = &options.filter;
        for (key, value) in [
            ("register.search", &filter.query),
            ("register.author", &filter.author),
            ("register.modified", &filter.modified),
            ("register.review_status", &filter.review_status),
            ("register.business_status", &filter.business_status),
            ("register.discipline", &filter.discipline),
            ("register.kind", &filter.kind),
            ("register.count_group", &filter.count_group),
        ] {
            if !value.is_empty() {
                ui.label(format!("{}: {value}", tr(lang, key)));
            }
        }
        ui.label(format!(
            "{}: {}",
            tr(lang, "register.page_filter"),
            filter.page.unwrap_or(0)
        ));
        ui.label(format!(
            "{}: {}",
            tr(lang, "register.include_replies"),
            filter.include_replies
        ));
        if let Some(column) = &filter.custom_sort {
            ui.label(format!("{}: {column}", tr(lang, "quantity.sort_custom")));
        }
        if let Some(zone) = &filter.zone {
            ui.label(format!(
                "{}: {} / {} · {}: {}",
                tr(lang, "zones.filter"),
                zone.page,
                zone.id,
                tr(lang, "zones.include_children"),
                zone.descendants
            ));
        }
        if let Some(status) = filter.zone_status {
            ui.label(tr(lang, &format!("zones.status.{}", status.id())));
        }
        ui.label(format!(
            "{}: {}",
            tr(lang, "issue_report.formulas"),
            options.formulas.len()
        ));
        for formula in &options.formulas {
            ui.label(format!(
                "{} = {} · {}: {} · {}: {}",
                formula.name,
                formula.expression,
                tr(lang, "quantity.formula_unit"),
                formula
                    .quantity_unit
                    .map_or(tr(lang, "quantity.formula_scalar"), |u| u.id()),
                tr(lang, "quantity.formula_rounding"),
                formula.rounding_decimals
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> IssueReportModel {
        IssueReportModel {
            schema: "sunipdf-issue-report",
            version: 1,
            title: "Reviewed report".into(),
            selection: IssueReportSelection::Quantities,
            provenance: sunipdf_core::issue_report::ReportProvenance {
                snapshot_sha256: "a".repeat(64),
                snapshot_hash_kind: "test",
                saved_file_sha256: None,
                saved_file_name: None,
                source_encrypted: true,
                source_signature_count: 1,
                report_encrypted: false,
                source_unchanged: true,
            },
            quantities: serde_json::json!({"rows":[],"zone_coverage":{"complete":false,"counts":{"unknown":1}}}),
            legend_included: true,
            legend: Vec::new(),
            issue_groups: Vec::new(),
            complete: false,
            warnings: vec!["Coverage warning".into()],
            page_count: 2,
            pdf_font: "Test font".into(),
            pdf_font_embedding: "test",
        }
    }
    fn preview() -> Outcome {
        Outcome::Preview {
            token: 7,
            model: Arc::new(model()),
            page: 0,
            image: RenderedPage {
                page_index: 0,
                width: 100,
                height: 140,
                rgba: vec![255; 100 * 140 * 4],
            },
        }
    }
    #[test]
    fn closing_or_switching_documents_discards_late_report_and_cannot_export() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_current_issue_report();
        app.prepare_issue_report();
        let (epoch, generation, cancel) = match requests.recv().unwrap() {
            Request::IssueReport {
                epoch,
                generation,
                job: Job::Prepare { cancel, .. },
            } => (epoch, generation, cancel),
            _ => panic!("prepare"),
        };
        app.close_issue_report();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(
            app.issue_report.busy(),
            "keep the result observable until commit/cancellation resolves"
        );
        app.on_issue_report(&ctx, epoch, generation, Ok(preview()));
        assert!(!app.issue_report.open && !app.issue_report.can_export());
        app.publish_issue_report_path(PathBuf::from("unreachable.pdf"));
        app.open_current_issue_report();
        app.prepare_issue_report();
        let (epoch, generation) = app.issue_report.pending.unwrap();
        app.view.epoch += 1;
        app.on_issue_report(&ctx, epoch, generation, Ok(preview()));
        assert!(!app.issue_report.open && app.issue_report.model.is_none());
        assert!(!requests.try_iter().any(|r| matches!(
            r,
            Request::IssueReport {
                job: Job::Publish { .. },
                ..
            } | Request::Edit(_)
        )));
    }
    #[test]
    fn cancellation_after_atomic_export_still_reports_success() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_current_issue_report();
        app.prepare_issue_report();
        let (epoch, generation) = app.issue_report.pending.unwrap();
        app.on_issue_report(&ctx, epoch, generation, Ok(preview()));
        app.issue_report.reviewed = true;
        app.publish_issue_report_path(PathBuf::from("report.pdf"));
        let (epoch, generation) = app.issue_report.pending.unwrap();
        app.close_issue_report();
        app.on_issue_report(
            &ctx,
            epoch,
            generation,
            Ok(Outcome::Published(Box::new(IssueReportReceipt {
                out: "report.pdf".into(),
                format: IssueReportFormat::Pdf,
                output_sha256: "b".repeat(64),
                bytes: 100,
                page_count: 2,
                matched_annotations: 0,
                complete: false,
                provenance: model().provenance,
                warnings: vec![],
            }))),
        );
        assert!(!app.issue_report.open && app.status_msg.as_ref().unwrap().contains("report.pdf"));
        assert_eq!(
            requests
                .try_iter()
                .filter(|r| matches!(
                    r,
                    Request::IssueReport {
                        job: Job::Publish { .. },
                        ..
                    }
                ))
                .count(),
            1
        );
    }
    #[test]
    fn report_preview_and_export_actions_remain_accessible_in_both_languages() {
        for language in Language::ALL {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.settings.set_language(language);
            app.open_current_issue_report();
            app.prepare_issue_report();
            let (epoch, generation) = app.issue_report.pending.unwrap();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            app.on_issue_report(&ctx, epoch, generation, Ok(preview()));
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720., 480.),
                        )),
                        time: Some(frame as f64 * 0.025),
                        ..Default::default()
                    },
                    |_| app.issue_report_dialog(&ctx),
                );
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in [
                    "issue_report.export",
                    "issue_report.preview",
                    "issue_report.close",
                ] {
                    let button = tree
                        .nodes
                        .iter()
                        .map(|(_, n)| n)
                        .find(|n| n.label() == Some(tr(language, key)))
                        .unwrap();
                    let b = button.bounds().unwrap();
                    assert!(
                        b.x0 >= 0. && b.x1 <= 720. && b.y0 >= 0. && b.y1 <= 480.,
                        "{key}: {b:?}"
                    );
                    if key == "issue_report.export" {
                        assert!(button.is_disabled(), "review is required");
                    }
                }
            }
        }
    }
}
