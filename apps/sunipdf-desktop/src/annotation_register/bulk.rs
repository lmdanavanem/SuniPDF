//! Explicit selected-row field patches and their reviewed, source-bound preview.

use super::{core, RegisterFields};
use crate::{
    dialog,
    worker::{EditOp, Request, Worker},
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
};
use sunipdf_core::{
    annotation_register_batch::{FieldPatch, RegisterBatchReport, RegisterFieldsPatch},
    EngineError,
};
use sunipdf_i18n::{tr, Language};

pub(crate) struct PreviewJob {
    pub register: Arc<core::AnnotationRegister>,
    pub rows: Vec<core::RegisterRow>,
    pub patch: RegisterFieldsPatch,
    pub cancel: Arc<AtomicBool>,
    pub reply: Sender<Result<RegisterBatchReport, EngineError>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Action {
    #[default]
    Keep,
    Set,
    Clear,
}

impl Action {
    fn key(self) -> &'static str {
        match self {
            Self::Keep => "register.bulk.keep",
            Self::Set => "register.bulk.set",
            Self::Clear => "register.bulk.clear",
        }
    }
    fn patch(self, value: &str) -> FieldPatch {
        match self {
            Self::Keep => FieldPatch::Keep,
            Self::Set => FieldPatch::Set(value.to_owned()),
            Self::Clear => FieldPatch::Clear,
        }
    }
}

#[derive(Default)]
struct ValueDraft {
    action: Action,
    value: String,
}
#[derive(Default)]
struct CustomDraft {
    key: String,
    value: ValueDraft,
}

struct Pending {
    receiver: Receiver<Result<RegisterBatchReport, EngineError>>,
}

#[derive(Default)]
pub(super) struct Draft {
    pub open: bool,
    status: ValueDraft,
    discipline: ValueDraft,
    custom: Vec<CustomDraft>,
    register: Option<Arc<core::AnnotationRegister>>,
    rows: Vec<core::RegisterRow>,
    epoch: u64,
    generation: u64,
    pending: Option<Pending>,
    cancel: Option<Arc<AtomicBool>>,
    preview: Option<RegisterBatchReport>,
    reviewed: bool,
    pub applying: bool,
    error: Option<String>,
}

impl Draft {
    #[cfg(all(test, windows))]
    pub(super) fn evidence_start(&mut self, worker: &Worker) {
        self.status = ValueDraft {
            action: Action::Set,
            value: "Ready for site".into(),
        };
        self.start(worker);
    }

    #[cfg(all(test, windows))]
    pub(super) fn evidence_preview(&self) -> Option<(&RegisterBatchReport, bool)> {
        self.preview.as_ref().map(|report| (report, self.reviewed))
    }

    pub fn bind(
        &mut self,
        register: Arc<core::AnnotationRegister>,
        rows: Vec<core::RegisterRow>,
        epoch: u64,
    ) {
        self.invalidate();
        self.register = Some(register);
        self.rows = rows;
        self.epoch = epoch;
        self.open = true;
    }

    pub fn invalidate(&mut self) {
        self.cancel_preview();
        self.preview = None;
        self.reviewed = false;
        self.generation = self.generation.wrapping_add(1);
        self.applying = false;
        self.error = None;
    }

    fn cancel_preview(&mut self) {
        self.pending = None;
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Release);
        }
    }

    pub fn source_changed(&mut self) {
        self.invalidate();
        self.register = None;
        self.rows.clear();
        self.open = false;
    }

    pub fn failed(&mut self, error: &EngineError) {
        if self.applying {
            self.invalidate();
            self.error = Some(error.to_string());
        }
    }

    fn patch(&self) -> Result<RegisterFieldsPatch, EngineError> {
        let mut custom = BTreeMap::new();
        for row in &self.custom {
            if custom
                .insert(row.key.clone(), row.value.action.patch(&row.value.value))
                .is_some()
            {
                return Err(EngineError::Other("duplicate custom field".into()));
            }
        }
        let patch = RegisterFieldsPatch {
            business_status: self.status.action.patch(&self.status.value),
            discipline: self.discipline.action.patch(&self.discipline.value),
            custom,
        };
        patch.validate()?;
        Ok(patch)
    }

    fn start(&mut self, worker: &Worker) {
        self.invalidate();
        match self.patch() {
            Ok(patch) => {
                let Some(register) = self.register.clone() else {
                    return;
                };
                let (reply, receiver) = mpsc::channel();
                let cancel = Arc::new(AtomicBool::new(false));
                worker.send(Request::PreviewRegisterBatch {
                    epoch: self.epoch,
                    generation: self.generation,
                    job: PreviewJob {
                        register,
                        rows: self.rows.clone(),
                        patch,
                        cancel: cancel.clone(),
                        reply,
                    },
                });
                self.cancel = Some(cancel);
                self.pending = Some(Pending { receiver });
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        worker: &Worker,
        epoch: u64,
        lang: Language,
        dark: bool,
        icons: bool,
    ) {
        if !self.open {
            return;
        }
        if epoch != self.epoch {
            self.source_changed();
            return;
        }
        if let Some(result) =
            self.pending
                .as_ref()
                .and_then(|pending| match pending.receiver.try_recv() {
                    Ok(result) => Some(result),
                    Err(mpsc::TryRecvError::Disconnected) => Some(Err(EngineError::Other(
                        "field preview was interrupted; preview again".into(),
                    ))),
                    Err(mpsc::TryRecvError::Empty) => None,
                })
        {
            self.pending = None;
            match result {
                Ok(report) => self.preview = Some(report),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let busy = self.pending.is_some() || self.applying;
        let applying = self.applying;
        let enabled = !worker.has_pending_mutations();
        let mut changed = false;
        let mut preview = false;
        let mut apply = false;
        let mut back = false;
        let mut cancel = false;
        let selected = self.rows.len();
        let can_apply = enabled
            && !busy
            && self.reviewed
            && self
                .preview
                .as_ref()
                .is_some_and(|report| report.changed_count > 0);
        let title = tr(lang, "register.bulk.title").replace("{count}", &selected.to_string());
        dialog::show(
            ctx,
            dialog::Spec::new("register-bulk-fields", title, dark, icons)
                .width((ctx.content_rect().width() - 48.0).clamp(420.0, 680.0)),
            |ui| {
                ui.label(tr(lang, "register.bulk.help"));
                ui.add_enabled_ui(!busy, |ui| {
                    changed |= value_ui(
                        ui,
                        lang,
                        "bulk-status",
                        tr(lang, "register.business_status"),
                        &mut self.status,
                    );
                    changed |= value_ui(
                        ui,
                        lang,
                        "bulk-discipline",
                        tr(lang, "register.discipline"),
                        &mut self.discipline,
                    );
                    ui.separator();
                    ui.label(tr(lang, "register.bulk.custom_help"));
                    let mut remove = None;
                    for (index, row) in self.custom.iter_mut().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "register.bulk.key"));
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut row.key)
                                            .char_limit(64)
                                            .desired_width(190.0),
                                    )
                                    .changed();
                                if ui.button(tr(lang, "register.bulk.remove")).clicked() {
                                    remove = Some(index);
                                }
                            });
                            changed |= value_ui(ui, lang, "custom-value", "", &mut row.value);
                        });
                    }
                    if let Some(index) = remove {
                        self.custom.remove(index);
                        changed = true;
                    }
                    if ui
                        .add_enabled(
                            self.custom.len() < 16,
                            egui::Button::new(tr(lang, "register.bulk.add")),
                        )
                        .clicked()
                    {
                        self.custom.push(CustomDraft::default());
                        changed = true;
                    }
                });
                if changed {
                    self.invalidate();
                }
                if let Some(error) = &self.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if self.pending.is_some() {
                    ui.spinner();
                    ui.label(tr(lang, "register.bulk.preparing"));
                }
                if let Some(report) = &self.preview {
                    ui.separator();
                    ui.strong(
                        tr(lang, "register.bulk.summary")
                            .replace("{changed}", &report.changed_count.to_string())
                            .replace("{selected}", &report.selected_count.to_string()),
                    );
                    if report.changed_count == 0 {
                        ui.label(tr(lang, "register.bulk.no_changes"));
                    }
                    for change in &report.changes {
                        egui::CollapsingHeader::new(format!(
                            "{} {} / {}",
                            tr(lang, "register.page"),
                            change.page,
                            change.index
                        ))
                        .id_salt((change.page, change.index))
                        .default_open(report.selected_count <= 3)
                        .show(ui, |ui| {
                            ui.strong(tr(lang, "register.bulk.before"));
                            fields_ui(ui, lang, &change.before);
                            ui.strong(tr(lang, "register.bulk.after"));
                            fields_ui(ui, lang, &change.after);
                        });
                    }
                    ui.checkbox(&mut self.reviewed, tr(lang, "register.bulk.reviewed"));
                }
            },
            |ui| {
                apply = ui
                    .add_enabled(
                        can_apply,
                        egui::Button::new(
                            tr(lang, "register.bulk.apply")
                                .replace("{count}", &selected.to_string()),
                        ),
                    )
                    .clicked();
                preview = ui
                    .add_enabled(
                        enabled && !busy,
                        egui::Button::new(tr(lang, "register.bulk.preview")),
                    )
                    .clicked();
                if busy && !applying {
                    cancel = ui.button(tr(lang, "register.cancel")).clicked();
                }
                back = ui
                    .add_enabled(!applying, egui::Button::new(tr(lang, "register.bulk.back")))
                    .clicked()
                    || (!applying && ui.input(|input| input.key_pressed(egui::Key::Escape)));
            },
        );
        if preview {
            self.start(worker);
        }
        // Settings may have invalidated the report earlier in this very frame.
        if apply
            && self.reviewed
            && self
                .preview
                .as_ref()
                .is_some_and(|report| report.changed_count > 0)
        {
            self.applying = true;
            worker.send(Request::Edit(EditOp::CommitRegisterBatch {
                epoch: self.epoch,
                generation: self.generation,
            }));
        }
        if cancel {
            self.invalidate();
        }
        if back {
            self.invalidate();
            self.open = false;
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}

fn value_ui(
    ui: &mut egui::Ui,
    lang: Language,
    id: &str,
    label: &str,
    draft: &mut ValueDraft,
) -> bool {
    if !label.is_empty() {
        ui.label(label);
    }
    let previous = draft.action;
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(tr(lang, draft.action.key()))
            .width(165.0)
            .show_ui(ui, |ui| {
                for action in [Action::Keep, Action::Set, Action::Clear] {
                    ui.selectable_value(&mut draft.action, action, tr(lang, action.key()));
                }
            });
        if draft.action == Action::Set {
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut draft.value)
                        .char_limit(1024)
                        .desired_width(ui.available_width().min(380.0)),
                )
                .changed();
        }
    });
    changed || previous != draft.action
}

fn fields_ui(ui: &mut egui::Ui, lang: Language, fields: &RegisterFields) {
    for (key, value) in [
        (
            "register.business_status",
            fields.business_status.as_deref(),
        ),
        ("register.discipline", fields.discipline.as_deref()),
    ] {
        ui.label(format!(
            "{}: {}",
            tr(lang, key),
            value.unwrap_or(tr(lang, "register.bulk.empty"))
        ));
    }
    for (key, value) in &fields.custom {
        ui.label(format!("{key}: {value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::annotation_register_batch::RegisterBatchChange;

    struct Controls {
        ctx: egui::Context,
        time: f64,
    }
    impl Controls {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
            Self { ctx, time: 1.0 }
        }
        fn frame(
            &mut self,
            draft: &mut Draft,
            worker: &Worker,
            lang: Language,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |_| draft.show(&self.ctx, worker, 86, lang, false, false),
            );
            output.textures_delta.clear();
            output
        }
        fn click(&mut self, draft: &mut Draft, worker: &Worker, lang: Language, label: &str) {
            fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                    _ => None,
                }
            }
            self.frame(draft, worker, lang, vec![]);
            let mut point = None;
            for _ in 0..24 {
                let output = self.frame(draft, worker, lang, vec![]);
                point = output.shapes.iter().rev().find_map(|shape| {
                    find(&shape.shape, label).filter(|point| shape.clip_rect.contains(*point))
                });
                if point.is_some() {
                    break;
                }
                self.frame(
                    draft,
                    worker,
                    lang,
                    vec![
                        egui::Event::PointerMoved(egui::pos2(350.0, 250.0)),
                        egui::Event::MouseWheel {
                            source: egui::MouseWheelSource::Wheel,
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, -160.0),
                            phase: egui::TouchPhase::Move,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            let point = point.unwrap_or_else(|| panic!("visible bulk control at 720x480: {label}"));
            for pressed in [true, false] {
                self.frame(
                    draft,
                    worker,
                    lang,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
    }

    fn fixture() -> Draft {
        let mut register = (*super::super::tests::large_register()).clone();
        register.rows.truncate(2);
        let rows = register.rows.clone();
        let mut draft = Draft::default();
        draft.bind(Arc::new(register), rows, 86);
        draft.status = ValueDraft {
            action: Action::Set,
            value: "Installed".into(),
        };
        draft
    }

    fn report(job: &PreviewJob, changed: usize) -> RegisterBatchReport {
        RegisterBatchReport {
            source_sha256: "a".repeat(64),
            selected_count: job.rows.len(),
            changed_count: changed,
            changes: job
                .rows
                .iter()
                .map(|row| RegisterBatchChange {
                    page: row.page,
                    index: row.index,
                    annotation_id: row.annotation_id.clone(),
                    expected_row_sha256: "b".repeat(64),
                    before: row.fields.clone(),
                    after: RegisterFields {
                        business_status: Some("Installed".into()),
                        ..row.fields.clone()
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn localized_small_window_previews_requires_review_and_applies_only_the_current_selection() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let lang = app.lang();
            let mut draft = fixture();
            let mut controls = Controls::new();
            controls.click(&mut draft, &app.worker, lang, tr(lang, "register.bulk.set"));
            controls.click(
                &mut draft,
                &app.worker,
                lang,
                tr(lang, "register.bulk.clear"),
            );
            assert!(draft.status.action == Action::Clear);
            controls.click(
                &mut draft,
                &app.worker,
                lang,
                tr(lang, "register.bulk.clear"),
            );
            controls.click(&mut draft, &app.worker, lang, tr(lang, "register.bulk.set"));
            assert!(draft.status.action == Action::Set);
            let apply = tr(lang, "register.bulk.apply").replace("{count}", "2");
            controls.click(&mut draft, &app.worker, lang, &apply);
            assert!(
                requests.try_recv().is_err(),
                "Apply requires an actual preview"
            );
            controls.click(
                &mut draft,
                &app.worker,
                lang,
                tr(lang, "register.bulk.preview"),
            );
            let (epoch, generation, job) = match requests.try_recv().unwrap() {
                Request::PreviewRegisterBatch {
                    epoch,
                    generation,
                    job,
                } => (epoch, generation, job),
                _ => panic!("preview"),
            };
            assert_eq!(
                job.rows
                    .iter()
                    .map(|row| (row.page, row.index))
                    .collect::<Vec<_>>(),
                vec![(1, 0), (1, 1)]
            );
            assert_eq!(
                job.patch.business_status,
                FieldPatch::Set("Installed".into())
            );
            job.reply.send(Ok(report(&job, 2))).unwrap();
            controls.frame(&mut draft, &app.worker, lang, vec![]);
            controls.click(&mut draft, &app.worker, lang, &apply);
            assert!(
                requests.try_recv().is_err(),
                "before/after must be reviewed explicitly"
            );
            controls.click(
                &mut draft,
                &app.worker,
                lang,
                tr(lang, "register.bulk.reviewed"),
            );
            assert!(draft.reviewed);
            controls.click(&mut draft, &app.worker, lang, &apply);
            assert!(
                matches!(requests.try_recv().unwrap(),Request::Edit(EditOp::CommitRegisterBatch { epoch:e,generation:g }) if e==epoch&&g==generation)
            );
            assert!(draft.applying);
        }
    }

    #[test]
    fn cancellation_settings_and_source_changes_invalidate_preview_but_keep_the_field_draft() {
        let (app, requests, _) = crate::app::tests::test_app();
        let lang = app.lang();
        let mut draft = fixture();
        let mut controls = Controls::new();
        controls.click(
            &mut draft,
            &app.worker,
            lang,
            tr(lang, "register.bulk.preview"),
        );
        let job = match requests.try_recv().unwrap() {
            Request::PreviewRegisterBatch { job, .. } => job,
            _ => panic!("preview"),
        };
        controls.click(&mut draft, &app.worker, lang, tr(lang, "register.cancel"));
        assert!(job.cancel.load(Ordering::Acquire));
        assert_eq!(draft.status.value, "Installed");
        assert!(draft.pending.is_none() && draft.preview.is_none());
        assert!(
            job.reply.send(Ok(report(&job, 2))).is_err(),
            "late cancelled preview is discarded"
        );
        controls.click(
            &mut draft,
            &app.worker,
            lang,
            tr(lang, "register.bulk.preview"),
        );
        let job = match requests.try_recv().unwrap() {
            Request::PreviewRegisterBatch { job, .. } => job,
            _ => panic!("preview"),
        };
        job.reply.send(Ok(report(&job, 0))).unwrap();
        controls.frame(&mut draft, &app.worker, lang, vec![]);
        draft.reviewed = true;
        controls.click(
            &mut draft,
            &app.worker,
            lang,
            &tr(lang, "register.bulk.apply").replace("{count}", "2"),
        );
        assert!(
            requests.try_recv().is_err(),
            "no-op preview cannot be applied"
        );
        controls.click(&mut draft, &app.worker, lang, tr(lang, "register.bulk.add"));
        assert!(draft.preview.is_none() && !draft.reviewed && job.cancel.load(Ordering::Acquire));
        draft.source_changed();
        assert!(!draft.open && draft.rows.is_empty());
        assert_eq!(draft.status.value, "Installed");
    }

    #[test]
    fn custom_patch_only_merges_explicit_keys_and_refuses_duplicates() {
        let mut draft = fixture();
        draft.discipline.action = Action::Clear;
        draft.custom = vec![
            CustomDraft {
                key: "Room".into(),
                value: ValueDraft {
                    action: Action::Set,
                    value: "A101".into(),
                },
            },
            CustomDraft {
                key: "Old".into(),
                value: ValueDraft {
                    action: Action::Clear,
                    value: String::new(),
                },
            },
        ];
        let patch = draft.patch().unwrap();
        assert_eq!(patch.discipline, FieldPatch::Clear);
        assert_eq!(patch.custom.len(), 2);
        assert_eq!(patch.custom["Old"], FieldPatch::Clear);
        draft.custom.push(CustomDraft {
            key: "Room".into(),
            ..Default::default()
        });
        assert!(draft.patch().is_err());
    }
}
