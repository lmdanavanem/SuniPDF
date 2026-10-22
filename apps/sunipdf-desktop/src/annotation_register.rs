//! Per-tab markup table and explicit, on-plan manual counting.

use crate::{
    annotate::Tool,
    app::SuniApp,
    dialog::{self, Spec},
    worker::{EditOp, Request},
};
use std::{collections::BTreeMap, sync::Arc};
use sunipdf_core::{
    annotation_register::{
        self as core, AnnotationRegister, RegisterFields, RegisterFilter, RegisterSort,
        RegisterTarget,
    },
    Command, EngineError,
};
use sunipdf_i18n::tr;

pub struct RegisterState {
    pub open: bool,
    pub count_open: bool,
    pub count_group: String,
    pub count_size: f32,
    pub report: Option<Arc<AnnotationRegister>>,
    pub pending: Option<(u64, u64)>,
    generation: u64,
    filter: RegisterFilter,
    sort: RegisterSort,
    descending: bool,
    page: usize,
    offset: usize,
    error: Option<String>,
    edit: Option<FieldDraft>,
}

impl Default for RegisterState {
    fn default() -> Self {
        Self {
            open: false,
            count_open: false,
            count_group: String::new(),
            count_size: 18.0,
            report: None,
            pending: None,
            generation: 0,
            filter: RegisterFilter::default(),
            sort: RegisterSort::Page,
            descending: false,
            page: 0,
            offset: 0,
            error: None,
            edit: None,
        }
    }
}

impl RegisterState {
    pub fn dialog_open(&self) -> bool {
        self.open || self.count_open
    }
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.report = None;
        self.pending = None;
        self.error = None;
        if self.edit.as_ref().is_some_and(|e| e.applying) {
            self.edit = None;
        }
    }
}

struct FieldDraft {
    target: RegisterTarget,
    business_status: String,
    discipline: String,
    custom: String,
    count_group: Option<String>,
    applying: bool,
    error: Option<String>,
}

fn fields(status: &str, discipline: &str, custom: &str) -> Result<RegisterFields, EngineError> {
    let optional = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_owned());
    let mut out = RegisterFields {
        business_status: optional(status),
        discipline: optional(discipline),
        ..Default::default()
    };
    for line in custom.lines().filter(|s| !s.trim().is_empty()) {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            EngineError::Other("custom fields require one KEY=VALUE per line".into())
        })?;
        if out.custom.insert(key.trim().into(), value.into()).is_some() {
            return Err(EngineError::Other("duplicate custom field".into()));
        }
    }
    out.validate()?;
    Ok(out)
}

impl SuniApp {
    pub(crate) fn open_annotation_register(&mut self) {
        self.annotate.register.invalidate();
        self.annotate.register.open = true;
        self.annotate.register.edit = None;
    }

    pub(crate) fn on_annotation_register(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<AnnotationRegister, EngineError>,
    ) {
        let state = &mut self.annotate.register;
        if epoch != self.view.epoch || state.pending != Some((epoch, generation)) {
            return;
        }
        state.pending = None;
        match result {
            Ok(report) => {
                state.report = Some(Arc::new(report));
                state.error = None;
            }
            Err(error) => state.error = Some(error.to_string()),
        }
    }

    pub(crate) fn register_edit_failed(&mut self, error: &EngineError) {
        if let Some(edit) = self.annotate.register.edit.as_mut().filter(|e| e.applying) {
            edit.applying = false;
            edit.error = Some(error.to_string());
        }
    }

    pub(crate) fn place_count_marker(&mut self, page: usize, point: (f32, f32)) {
        if self.annotate.register.dialog_open() || self.annotate.tool != Tool::Count {
            return;
        }
        let state = &self.annotate.register;
        if core::validate_count_group(&state.count_group).is_err() {
            self.annotate.register.count_open = true;
            return;
        }
        self.worker.send(Request::Edit(EditOp::AddCountMarker {
            page,
            point,
            size: state.count_size,
            group: state.count_group.clone(),
            author: self.annot_author(),
        }));
    }

    pub(crate) fn count_marker_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.register.count_open {
            return;
        }
        let lang = self.lang();
        let state = &mut self.annotate.register;
        let mut start = false;
        let mut cancel = false;
        let valid = core::validate_count_group(&state.count_group).is_ok();
        dialog::show(
            ctx,
            Spec::new(
                "count-marker",
                tr(lang, "register.count_tool"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(440.0),
            |ui| {
                ui.label(tr(lang, "register.count_help"));
                ui.label(tr(lang, "register.count_group"));
                ui.add(
                    egui::TextEdit::singleline(&mut state.count_group)
                        .char_limit(512)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "register.marker_size"));
                    ui.add(
                        egui::DragValue::new(&mut state.count_size)
                            .range(8.0..=72.0)
                            .suffix(" pt"),
                    );
                });
            },
            |ui| {
                start = ui
                    .add_enabled(
                        valid,
                        egui::Button::new(tr(lang, "register.start_counting")),
                    )
                    .clicked();
                cancel = dialog::secondary(ui, tr(lang, "register.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if start || cancel {
            state.count_open = false;
        }
        if cancel {
            self.annotate.tool = Tool::Select;
        }
        if start {
            self.status_msg = Some(format!(
                "{}: {}",
                tr(lang, "register.count_help"),
                state.count_group
            ));
        }
    }

    pub(crate) fn annotation_register_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.register.open {
            return;
        }
        let lang = self.lang();
        let mut state = std::mem::take(&mut self.annotate.register);
        if state.report.is_none() && state.pending.is_none() && state.error.is_none() {
            state.generation = state.generation.wrapping_add(1);
            state.pending = Some((self.view.epoch, state.generation));
            self.worker.send(Request::AnnotationRegister {
                epoch: self.view.epoch,
                generation: state.generation,
            });
        }
        let mut close = false;
        let mut refresh = false;
        let mut export = None;
        let mut navigate = None;
        let mut apply = false;
        let mut cancel_edit = false;
        let mut quantities = false;
        let report = state.report.clone();
        let enabled = !self.worker.has_pending_mutations();
        let row_actions_enabled = enabled && state.edit.is_none();
        let can_refresh = enabled && state.edit.is_none();
        let can_export = can_refresh && report.is_some();
        let can_close = state.edit.as_ref().is_none_or(|e| !e.applying);
        let width = (ctx.content_rect().width() - 60.0).clamp(420.0, 1050.0);
        dialog::show(
            ctx,
            Spec::new(
                "annotation-register",
                tr(lang, "register.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(width),
            |ui| {
                ui.label(tr(lang, "register.help"));
                if let Some(error) = &state.error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                if state.pending.is_some() {
                    ui.spinner();
                    ui.label(tr(lang, "register.loading"));
                }
                if let Some(report) = &report {
                    egui::Grid::new("register-filters")
                        .num_columns(4)
                        .show(ui, |ui| {
                            for (key, text) in [
                                ("register.search", &mut state.filter.query),
                                ("register.author", &mut state.filter.author),
                                (
                                    "register.business_status",
                                    &mut state.filter.business_status,
                                ),
                                ("register.discipline", &mut state.filter.discipline),
                                ("register.review_status", &mut state.filter.review_status),
                                ("register.modified", &mut state.filter.modified),
                                ("register.kind", &mut state.filter.kind),
                                ("register.count_group", &mut state.filter.count_group),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                ui.label(tr(lang, text.0));
                                ui.add(
                                    egui::TextEdit::singleline(text.1)
                                        .char_limit(512)
                                        .desired_width(150.0),
                                );
                                if key % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "register.page_filter"));
                        ui.add(egui::DragValue::new(&mut state.page).range(0..=report.page_count));
                        state.filter.page = (state.page > 0).then_some(state.page);
                        ui.checkbox(
                            &mut state.filter.include_replies,
                            tr(lang, "register.include_replies"),
                        );
                        egui::ComboBox::from_id_salt("register-sort")
                            .selected_text(tr(lang, &format!("register.{}", state.sort.id())))
                            .show_ui(ui, |ui| {
                                for sort in RegisterSort::ALL {
                                    ui.selectable_value(
                                        &mut state.sort,
                                        sort,
                                        tr(lang, &format!("register.{}", sort.id())),
                                    );
                                }
                            });
                        ui.checkbox(&mut state.descending, tr(lang, "register.descending"));
                    });
                    let rows = report.select(&state.filter, state.sort, state.descending);
                    state.offset = state.offset.min(rows.len().saturating_sub(1) / 50 * 50);
                    let mut totals = BTreeMap::<&str, usize>::new();
                    for row in &rows {
                        if let Some(group) = &row.count_group {
                            *totals.entry(group).or_default() += 1;
                        }
                    }
                    ui.label(format!(
                        "{}: {} / {}",
                        tr(lang, "register.matches"),
                        rows.len(),
                        report.rows.len()
                    ));
                    for (group, count) in totals.iter().take(20) {
                        ui.label(format!("{group}: {count}"));
                    }
                    if totals.len() > 20 {
                        ui.label(tr(lang, "register.more_groups"));
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                state.offset > 0,
                                egui::Button::new(tr(lang, "register.previous")),
                            )
                            .clicked()
                        {
                            state.offset = state.offset.saturating_sub(50);
                        }
                        ui.label(format!(
                            "{}–{} / {}",
                            if rows.is_empty() { 0 } else { state.offset + 1 },
                            (state.offset + 50).min(rows.len()),
                            rows.len()
                        ));
                        if ui
                            .add_enabled(
                                state.offset + 50 < rows.len(),
                                egui::Button::new(tr(lang, "register.next")),
                            )
                            .clicked()
                        {
                            state.offset += 50;
                        }
                    });
                    egui::ScrollArea::horizontal()
                        .id_salt("register-table-scroll")
                        .show(ui, |ui| {
                            egui::Grid::new("register-table")
                                .striped(true)
                                .show(ui, |ui| {
                                    for key in [
                                        "page",
                                        "kind",
                                        "author",
                                        "modified",
                                        "review_status",
                                        "business_status",
                                        "discipline",
                                        "count_group",
                                        "contents",
                                        "actions",
                                    ] {
                                        ui.strong(tr(lang, &format!("register.{key}")));
                                    }
                                    ui.end_row();
                                    for row in rows.iter().skip(state.offset).take(50) {
                                        ui.label(format!("{} / {}", row.page, row.index));
                                        ui.label(&row.kind);
                                        for text in [
                                            row.author.as_deref(),
                                            row.modified.as_deref(),
                                            row.review_status.as_deref(),
                                            row.fields.business_status.as_deref(),
                                            row.fields.discipline.as_deref(),
                                            row.count_group.as_deref(),
                                            row.contents.as_deref(),
                                        ] {
                                            let text = text.unwrap_or("—");
                                            let display: String = text.chars().take(70).collect();
                                            ui.label(display).on_hover_text(text);
                                        }
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_enabled(
                                                    row_actions_enabled && row.rect.is_some(),
                                                    egui::Button::new(tr(lang, "register.view")),
                                                )
                                                .clicked()
                                            {
                                                navigate = Some((**row).clone());
                                            }
                                            if ui
                                                .add_enabled(
                                                    row_actions_enabled
                                                        && !row.locked
                                                        && row.kind != "Widget"
                                                        && !row.is_reply,
                                                    egui::Button::new(tr(lang, "register.fields")),
                                                )
                                                .clicked()
                                            {
                                                state.edit = Some(FieldDraft {
                                                    target: report.target(row),
                                                    business_status: row
                                                        .fields
                                                        .business_status
                                                        .clone()
                                                        .unwrap_or_default(),
                                                    discipline: row
                                                        .fields
                                                        .discipline
                                                        .clone()
                                                        .unwrap_or_default(),
                                                    custom: row
                                                        .fields
                                                        .custom
                                                        .iter()
                                                        .map(|(k, v)| format!("{k}={v}"))
                                                        .collect::<Vec<_>>()
                                                        .join("\n"),
                                                    count_group: row.count_group.clone(),
                                                    applying: false,
                                                    error: None,
                                                });
                                            }
                                        });
                                        ui.end_row();
                                    }
                                });
                        });
                }
                if let Some(edit) = &mut state.edit {
                    ui.separator();
                    ui.heading(tr(lang, "register.fields"));
                    ui.add_enabled_ui(!edit.applying, |ui| {
                        for (key, text) in [
                            ("register.business_status", &mut edit.business_status),
                            ("register.discipline", &mut edit.discipline),
                        ] {
                            ui.label(tr(lang, key));
                            ui.add(egui::TextEdit::singleline(text).char_limit(512));
                        }
                        if let Some(group) = &mut edit.count_group {
                            ui.label(tr(lang, "register.count_group"));
                            ui.add(egui::TextEdit::singleline(group).char_limit(512));
                        }
                        ui.label(tr(lang, "register.custom_help"));
                        ui.add(
                            egui::TextEdit::multiline(&mut edit.custom)
                                .char_limit(18000)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                        );
                        if let Some(error) = &edit.error {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                        ui.horizontal(|ui| {
                            apply = ui
                                .add_enabled(enabled, egui::Button::new(tr(lang, "register.apply")))
                                .clicked();
                            cancel_edit = ui.button(tr(lang, "register.cancel")).clicked();
                        });
                    });
                }
            },
            |ui| {
                quantities = ui
                    .add_enabled(can_refresh, egui::Button::new(tr(lang, "quantity.title")))
                    .clicked();
                refresh = ui
                    .add_enabled(can_refresh, egui::Button::new(tr(lang, "register.refresh")))
                    .clicked();
                for format in ["json", "csv"] {
                    if ui
                        .add_enabled(
                            can_export,
                            egui::Button::new(tr(lang, &format!("register.export_{format}"))),
                        )
                        .clicked()
                    {
                        export = Some(format);
                    }
                }
                close = ui
                    .add_enabled(can_close, egui::Button::new(tr(lang, "register.close")))
                    .clicked()
                    || (can_close && ui.input(|i| i.key_pressed(egui::Key::Escape)));
            },
        );
        if apply {
            if let Some(edit) = &mut state.edit {
                let value = fields(&edit.business_status, &edit.discipline, &edit.custom).and_then(
                    |value| {
                        if let Some(group) = &edit.count_group {
                            core::validate_count_group(group)?;
                        }
                        Ok(value)
                    },
                );
                match value {
                    Ok(fields) => {
                        edit.applying = true;
                        edit.error = None;
                        self.worker
                            .send(Request::Edit(EditOp::SetAnnotationRegisterFields {
                                target: edit.target.clone(),
                                fields,
                                count_group: edit.count_group.clone(),
                            }));
                    }
                    Err(error) => edit.error = Some(error.to_string()),
                }
            }
        }
        if cancel_edit {
            state.edit = None;
        }
        if refresh {
            state.invalidate();
        }
        if let (Some(format), Some(report)) = (export, &report) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(format.to_uppercase(), &[format])
                .set_file_name(format!("annotation-register.{format}"))
                .save_file()
            {
                let result = if format == "csv" {
                    Ok(report
                        .csv(&state.filter, state.sort, state.descending)
                        .into_bytes())
                } else {
                    serde_json::to_vec_pretty(&report.json(
                        &state.filter,
                        state.sort,
                        state.descending,
                    ))
                    .map_err(|e| EngineError::Other(e.to_string()))
                };
                match result.and_then(|bytes| sunipdf_core::write_new_copy(&path, &bytes)) {
                    Ok(()) => self.status_msg = Some(tr(lang, "register.exported").into()),
                    Err(error) => state.error = Some(error.to_string()),
                }
            }
        }
        if let Some(row) = navigate {
            if let Some(rect) = row.rect {
                close = true;
                state.edit = None;
                self.annotate.tool = Tool::SelectAnnot;
                crate::workspace::follow(
                    &mut self.annotate,
                    &mut self.organize,
                    &mut self.edit,
                    Command::ToolSelectAnnot,
                );
                self.annotate.comments_open = true;
                self.annotate.adv.reselect = Some((row.page - 1, row.index));
                let height = self
                    .document
                    .as_ref()
                    .and_then(|d| d.info.page_sizes.get(row.page - 1))
                    .map_or(0.0, |s| s.height_pt);
                self.go_to_page_point(row.page - 1, rect[0], (height - rect[3]).max(0.0));
            }
        }
        if close {
            state.open = false;
            state.edit = None;
        }
        self.annotate.register = state;
        if quantities {
            self.annotate.register.open = false;
            self.open_quantity_table();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn register_rejects_late_results_and_count_clicks_use_the_selected_group() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.tool = Tool::Count;
        app.annotate.register.count_group = "Lights".into();
        app.place_count_marker(1, (80.0, 90.0));
        assert!(
            matches!(requests.try_recv().unwrap(),Request::Edit(EditOp::AddCountMarker {page:1,point:(80.0,90.0),size:18.0,group,..}) if group=="Lights")
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        app.annotate.register.count_open = true;
        app.place_count_marker(0, (20.0, 30.0));
        assert!(
            requests.try_recv().is_err(),
            "settings dialog prevents accidental counts"
        );
        app.annotate.register.count_open = false;
        app.annotate.register.pending = Some((app.view.epoch, 4));
        let report = || {
            let mut report = AnnotationRegister::default();
            report.source_sha256 = "a".repeat(64);
            report.page_count = 2;
            report
        };
        app.on_annotation_register(app.view.epoch, 3, Ok(report()));
        assert!(app.annotate.register.report.is_none());
        app.on_annotation_register(app.view.epoch.wrapping_add(1), 4, Ok(report()));
        assert!(app.annotate.register.report.is_none());
        app.on_annotation_register(app.view.epoch, 4, Ok(report()));
        assert!(app.annotate.register.report.is_some() && app.annotate.register.pending.is_none());
    }

    #[test]
    fn register_and_count_dialogs_have_localized_accessible_actions_and_cancel_without_edits() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            let _ = crate::icons::install(&ctx);
            ctx.enable_accesskit();
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 900.0),
                )),
                ..Default::default()
            };
            app.annotate.register.count_open = true;
            app.annotate.tool = Tool::Count;
            let mut labels = Vec::new();
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |_| app.count_marker_dialog(&ctx));
                if let Some(tree) = output.platform_output.accesskit_update.as_ref() {
                    labels.extend(
                        tree.nodes
                            .iter()
                            .filter_map(|(_, n)| n.label().map(str::to_owned)),
                    );
                }
                output.textures_delta.clear();
            }
            assert!(labels
                .iter()
                .any(|s| s == tr(app.lang(), "register.start_counting")));
            input.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let mut output = ctx.run_ui(input.clone(), |_| app.count_marker_dialog(&ctx));
            output.textures_delta.clear();
            assert!(!app.annotate.register.count_open && app.annotate.tool == Tool::Select);
            assert!(requests.try_recv().is_err());
            input.events.clear();
            app.annotate.register.open = true;
            let mut report = AnnotationRegister::default();
            report.source_sha256 = "a".repeat(64);
            report.page_count = 2;
            app.annotate.register.report = Some(Arc::new(report));
            labels.clear();
            for _ in 0..2 {
                let mut output =
                    ctx.run_ui(input.clone(), |_| app.annotation_register_dialog(&ctx));
                if let Some(tree) = output.platform_output.accesskit_update.as_ref() {
                    labels.extend(
                        tree.nodes
                            .iter()
                            .filter_map(|(_, n)| n.label().map(str::to_owned)),
                    );
                }
                output.textures_delta.clear();
            }
            assert!(labels
                .iter()
                .any(|s| s == tr(app.lang(), "register.export_csv")));
            assert!(
                requests.try_recv().is_err(),
                "reading the cached register does not edit the PDF"
            );
        }
    }
    #[test]
    fn register_fields_refuse_duplicates_and_keep_business_states_separate() {
        let value = fields("Ready for site", "Electrical", "Room=A101\nCircuit=R1").unwrap();
        assert_eq!(value.business_status.as_deref(), Some("Ready for site"));
        assert_eq!(value.custom["Room"], "A101");
        assert!(fields("", "", "Room=A\nRoom=B").is_err());
        assert!(fields("", "", "not a pair").is_err());
        assert!(fields("", "", "=empty key").is_err());
        let mut state = RegisterState {
            open: true,
            ..Default::default()
        };
        state.pending = Some((4, 2));
        state.invalidate();
        assert!(state.open && state.pending.is_none() && state.report.is_none());
    }
}
