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
    EngineError,
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
    followed: Option<(usize, usize)>,
    error: Option<String>,
    edit: Option<FieldDraft>,
    selection: SelectionCache,
    progress: Option<(usize, usize)>,
}

#[derive(Default)]
struct SelectionCache {
    key: Option<(Arc<AnnotationRegister>, RegisterFilter, RegisterSort, bool)>,
    selected: Arc<SelectedRows>,
}

#[derive(Default)]
struct SelectedRows {
    rows: Vec<usize>,
    totals: BTreeMap<String, usize>,
}

impl SelectionCache {
    fn get(
        &mut self,
        report: &Arc<AnnotationRegister>,
        filter: &RegisterFilter,
        sort: RegisterSort,
        descending: bool,
    ) -> Arc<SelectedRows> {
        if self
            .key
            .as_ref()
            .is_none_or(|(previous, old_filter, old_sort, old_descending)| {
                !Arc::ptr_eq(previous, report)
                    || old_filter != filter
                    || *old_sort != sort
                    || *old_descending != descending
            })
        {
            let rows = report.select_indices(filter, sort, descending);
            let mut totals = BTreeMap::new();
            for index in &rows {
                if let Some(group) = &report.rows[*index].count_group {
                    *totals.entry(group.clone()).or_default() += 1;
                }
            }
            self.selected = Arc::new(SelectedRows { rows, totals });
            self.key = Some((report.clone(), filter.clone(), sort, descending));
        }
        self.selected.clone()
    }
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
            followed: None,
            error: None,
            edit: None,
            selection: SelectionCache::default(),
            progress: None,
        }
    }
}

impl RegisterState {
    pub fn dialog_open(&self) -> bool {
        self.count_open || self.edit.is_some()
    }
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.report = None;
        self.pending = None;
        self.progress = None;
        self.selection = SelectionCache::default();
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
        if self.annotate.quantities.blocks_document() || self.annotate.measure.is_active_draft() {
            return;
        }
        self.annotate.register.invalidate();
        self.annotate.register.open = true;
        self.annotate.register.edit = None;
        self.annotate.quantities.open = false;
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
        state.progress = None;
        match result {
            Ok(report) => {
                state.report = Some(Arc::new(report));
                state.error = None;
            }
            Err(error) => state.error = Some(error.to_string()),
        }
    }

    pub(crate) fn on_annotation_register_progress(
        &mut self,
        epoch: u64,
        generation: u64,
        done: usize,
        total: usize,
    ) {
        let state = &mut self.annotate.register;
        if epoch == self.view.epoch && state.pending == Some((epoch, generation)) {
            state.progress = Some((done, total));
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
        if self.settings.tables.placement == crate::workspace_table::Placement::Floating {
            self.annotation_register_ui(ctx, None);
        }
    }

    pub(crate) fn annotation_register_dock(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.settings.tables.placement == crate::workspace_table::Placement::Bottom {
            self.annotation_register_ui(ctx, Some(ui));
        }
    }

    fn annotation_register_ui(&mut self, ctx: &egui::Context, host: Option<&mut egui::Ui>) {
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
        let mut cancel_collection = false;
        let report = state.report.clone();
        let enabled = !self.worker.has_pending_mutations();
        let row_actions_enabled = enabled && state.edit.is_none();
        let can_refresh = enabled && state.edit.is_none();
        let can_export = can_refresh && report.is_some();
        let can_close = state.edit.as_ref().is_none_or(|e| !e.applying);
        let width = (ctx.content_rect().width() - 60.0).clamp(420.0, 1050.0);
        let selection = self
            .annotate
            .adv
            .selected
            .as_ref()
            .map(|(page, detail)| (*page + 1, detail.index));
        let layout_changed = crate::workspace_table::show(
            ctx,
            host,
            &mut self.settings.tables,
            lang,
            Spec::new(
                "annotation-register",
                tr(lang, "register.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(width),
            |ui| {
                if let Some(error) = &state.error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                if state.pending.is_some() {
                    ui.spinner();
                    ui.label(tr(lang, "register.loading"));
                    if let Some((done, total)) = state.progress {
                        ui.label(format!("{done} / {total}"));
                    }
                    cancel_collection = ui.button(tr(lang, "register.cancel")).clicked();
                }
                if let Some(report) = &report {
                    let selected =
                        state
                            .selection
                            .get(report, &state.filter, state.sort, state.descending);
                    let rows = &selected.rows;
                    if selection != state.followed {
                        state.followed = selection;
                        if let Some(position) = rows.iter().position(|index| {
                            selection == Some((report.rows[*index].page, report.rows[*index].index))
                        }) {
                            state.offset = position / 50 * 50;
                        }
                    }
                    state.offset = state.offset.min(rows.len().saturating_sub(1) / 50 * 50);
                    let totals = &selected.totals;
                    // Group totals are available without pushing the first result below the fold.
                    ui.horizontal_wrapped(|ui| {
                        ui.menu_button(tr(lang, "workspace.table.filters"), |ui| {
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
                                ui.add(
                                    egui::DragValue::new(&mut state.page)
                                        .range(0..=report.page_count),
                                );
                                state.filter.page = (state.page > 0).then_some(state.page);
                                ui.checkbox(
                                    &mut state.filter.include_replies,
                                    tr(lang, "register.include_replies"),
                                );
                                egui::ComboBox::from_id_salt("register-sort")
                                    .selected_text(tr(
                                        lang,
                                        &format!("register.{}", state.sort.id()),
                                    ))
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
                        });

                        if !totals.is_empty() {
                            ui.menu_button(tr(lang, "workspace.table.summary"), |ui| {
                                for (group, count) in totals.iter().take(20) {
                                    ui.label(format!("{group}: {count}"));
                                }
                                if totals.len() > 20 {
                                    ui.label(tr(lang, "register.more_groups"));
                                }
                            });
                        }
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
                                        "contents",
                                        "count_group",
                                        "author",
                                        "modified",
                                        "review_status",
                                        "business_status",
                                        "discipline",
                                        "actions",
                                    ] {
                                        ui.strong(tr(lang, &format!("register.{key}")));
                                    }
                                    ui.end_row();
                                    for index in rows.iter().skip(state.offset).take(50) {
                                        let row = &report.rows[*index];
                                        if ui
                                            .add_enabled(
                                                row_actions_enabled && row.rect.is_some(),
                                                egui::Button::selectable(
                                                    selection == Some((row.page, row.index)),
                                                    format!("{} / {}", row.page, row.index),
                                                ),
                                            )
                                            .on_hover_text(tr(lang, "register.view"))
                                            .clicked()
                                        {
                                            navigate = Some(row.clone());
                                        }
                                        ui.label(&row.kind);
                                        for text in [
                                            row.contents.as_deref(),
                                            row.count_group.as_deref(),
                                            row.author.as_deref(),
                                            row.modified.as_deref(),
                                            row.review_status.as_deref(),
                                            row.fields.business_status.as_deref(),
                                            row.fields.discipline.as_deref(),
                                        ] {
                                            let text = text.unwrap_or("—");
                                            let display: String = text.chars().take(70).collect();
                                            ui.add_sized(
                                                [160.0, 20.0],
                                                egui::Label::new(display).truncate(),
                                            )
                                            .on_hover_text(text);
                                        }
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_enabled(
                                                    row_actions_enabled && row.rect.is_some(),
                                                    egui::Button::new(tr(lang, "register.view")),
                                                )
                                                .clicked()
                                            {
                                                navigate = Some(row.clone());
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
        if layout_changed {
            self.persist_settings();
        }
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
        if cancel_collection || close {
            if let Some((epoch, generation)) = state.pending.take() {
                self.worker
                    .send(Request::CancelAnnotationRegister { epoch, generation });
                state.progress = None;
                state.error = Some(tr(lang, "register.cancelled").into());
            }
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
            state.edit = None;
            self.follow_table_annotation(&row);
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

    fn large_register() -> Arc<AnnotationRegister> {
        let mut report = AnnotationRegister::default();
        report.page_count = 1000;
        report.rows = (0..10_000)
            .map(|index| core::RegisterRow {
                page: index / 10 + 1,
                index: index % 10,
                annotation_id: Some(format!("SuniPDF-test-{index}")),
                kind: "Stamp".into(),
                author: Some(format!("Reviewer {}", index % 4)),
                modified: None,
                contents: Some(format!("Level {} electrical review", index % 10)),
                subject: None,
                rect: Some([10.0, 20.0, 30.0, 40.0]),
                parent: None,
                is_reply: false,
                hidden: false,
                locked: false,
                review_status: None,
                fields: RegisterFields::default(),
                count_group: Some(format!("Group {}", index % 20)),
            })
            .collect();
        Arc::new(report)
    }

    #[test]
    fn cached_register_rows_totals_and_order_change_only_with_semantic_inputs() {
        let report = large_register();
        let mut cache = SelectionCache::default();
        let mut filter = RegisterFilter::default();
        let original = cache.get(&report, &filter, RegisterSort::Page, false);
        assert_eq!(original.rows.len(), 10_000);
        assert_eq!(original.totals.len(), 20);
        assert!(original.totals.values().all(|count| *count == 500));
        for _ in 0..100 {
            assert!(Arc::ptr_eq(
                &original,
                &cache.get(&report, &filter, RegisterSort::Page, false)
            ));
        }
        filter.query = "LEVEL 3".into();
        let selected = cache.get(&report, &filter, RegisterSort::Author, true);
        assert_eq!(selected.rows.len(), 1000);
        assert_eq!(
            selected.rows,
            report.select_indices(&filter, RegisterSort::Author, true)
        );
        assert_eq!(selected.totals.values().sum::<usize>(), 1000);
        assert!(!Arc::ptr_eq(&original, &selected));
        let resorted = cache.get(&report, &filter, RegisterSort::Page, false);
        assert!(resorted.rows.windows(2).all(|pair| pair[0] < pair[1]));
        let mut changed = (*report).clone();
        changed.rows.clear();
        let changed = Arc::new(changed);
        let refreshed = cache.get(&changed, &filter, RegisterSort::Page, false);
        assert!(refreshed.rows.is_empty() && refreshed.totals.is_empty());
    }

    #[test]
    fn register_progress_rejects_stale_epochs_generations_and_completion() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let epoch = app.view.epoch;
        app.annotate.register.pending = Some((epoch, 7));
        app.on_annotation_register_progress(epoch, 7, 1, 20);
        assert_eq!(app.annotate.register.progress, Some((1, 20)));
        app.on_annotation_register_progress(epoch, 6, 19, 20);
        app.on_annotation_register_progress(epoch + 1, 7, 19, 20);
        assert_eq!(app.annotate.register.progress, Some((1, 20)));
        app.annotate.register.invalidate();
        app.on_annotation_register_progress(epoch, 7, 20, 20);
        assert!(app.annotate.register.progress.is_none());
    }

    #[test]
    #[ignore]
    fn performance_register_filter_10000_annotations() {
        let report = large_register();
        let mut cache = SelectionCache::default();
        let mut timings = Vec::new();
        for index in 0..100 {
            let filter = RegisterFilter {
                query: format!("level {}", index % 10),
                ..Default::default()
            };
            let start = std::time::Instant::now();
            std::hint::black_box(cache.get(&report, &filter, RegisterSort::Author, index % 2 == 0));
            timings.push(start.elapsed());
        }
        timings.sort();
        println!(
            "register_filter rows=10000 samples=100 p95_us={}",
            timings[94].as_micros()
        );
    }
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
                    ctx.run_ui(input.clone(), |ui| app.annotation_register_dock(&ctx, ui));
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
    #[test]
    fn docked_register_row_navigation_keeps_table_and_document_open() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 3];
        app.annotate.register.open = true;
        let report = large_register();
        let mut row = report.rows[0].clone();
        row.page = 2;
        row.index = 7;
        assert!(
            !app.dialog_open(),
            "a read-only register does not block the document"
        );
        app.follow_table_annotation(&row);
        assert!(app.annotate.register.open);
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert_eq!(app.annotate.adv.reselect, Some((1, 7)));
        assert!(!app.dirty);
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, Request::Edit(_) | Request::Save { .. })));
        row.page = 0;
        app.follow_table_annotation(&row);
        assert_eq!(
            app.document.as_ref().unwrap().page,
            1,
            "invalid pages do not change selection"
        );
        row.page = 3;
        row.rect = Some([f32::NAN, 0.0, 20.0, 20.0]);
        app.follow_table_annotation(&row);
        assert_eq!(
            app.document.as_ref().unwrap().page,
            1,
            "invalid coordinates cannot navigate"
        );
    }

    #[test]
    fn compact_register_keeps_first_row_visible_and_close_preserves_document() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.register.open = true;
        let mut report = (*large_register()).clone();
        report.rows.truncate(2);
        app.annotate.register.report = Some(Arc::new(report));
        let pages = app.document.as_ref().unwrap().info.page_sizes.clone();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(720.0, 480.0),
            )),
            ..Default::default()
        };
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| {
                ui.allocate_space(egui::vec2(720.0, 95.0));
                app.annotation_register_dock(&ctx, ui);
            });
            output.textures_delta.clear();
            assert!(
                output.shapes.iter().any(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == "1 / 0" => shape
                        .clip_rect
                        .contains(text.pos + text.galley.size() / 2.0),
                    _ => false,
                }),
                "first result must be visible at 720x480"
            );
        }
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = ctx.run_ui(input, |ui| app.annotation_register_dock(&ctx, ui));
        output.textures_delta.clear();
        assert!(!app.annotate.register.open);
        assert_eq!(app.document.as_ref().unwrap().info.page_sizes, pages);
        assert!(!app.dirty);
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, Request::Edit(_) | Request::Save { .. })));
    }
}
