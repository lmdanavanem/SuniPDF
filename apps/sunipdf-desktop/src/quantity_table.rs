//! Per-document quantity inspection and explicit typed-field edits.

use crate::{
    app::SuniApp,
    dialog,
    worker::{EditOp, Request},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
};
use sunipdf_core::{
    quantity_table::{
        DecimalValue, QuantityFields, QuantityGrouping, QuantityOptions, QuantityRow,
        QuantitySummary, QuantityTable, QuantityTarget, QuantityUnit, TypedValue, UnitPrice,
    },
    EngineError,
};
use sunipdf_i18n::{tr, Language};

#[derive(Default)]
pub struct QuantityState {
    pub open: bool,
    epoch: Option<u64>,
    generation: u64,
    pending: Option<(u64, u64)>,
    exporting: Option<(u64, u64)>,
    report: Option<Arc<QuantityTable>>,
    options: QuantityOptions,
    selected: Vec<usize>,
    columns: Vec<String>,
    column_filter: String,
    summary: Option<QuantitySummary>,
    offset: usize,
    followed: Option<(usize, usize)>,
    page_filter: usize,
    error: Option<String>,
    message: Option<String>,
    edit: Option<RowDraft>,
}

impl QuantityState {
    pub(crate) fn blocks_document(&self) -> bool {
        self.edit.is_some() || self.exporting.is_some()
    }
    pub(crate) fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.report = None;
        self.summary = None;
        self.selected.clear();
        self.columns.clear();
        self.error = None;
        if self.edit.as_ref().is_some_and(|draft| draft.applying) {
            self.edit = None;
        }
    }

    fn refresh_selection(&mut self) {
        let Some(report) = &self.report else {
            return;
        };
        let positions: HashMap<_, _> = report
            .rows()
            .iter()
            .enumerate()
            .map(|(index, row)| ((row.register.page, row.register.index), index))
            .collect();
        self.selected = report
            .select(&self.options)
            .into_iter()
            .filter_map(|row| {
                positions
                    .get(&(row.register.page, row.register.index))
                    .copied()
            })
            .collect();
        let mut columns = BTreeSet::new();
        for index in &self.selected {
            columns.extend(report.rows()[*index].fields.custom.keys().cloned());
        }
        self.columns = columns.into_iter().collect();
        self.offset = 0;
        match report.summary(&self.options) {
            Ok(summary) => self.summary = Some(summary),
            Err(error) => {
                self.summary = None;
                self.error = Some(error.to_string());
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Text,
    Number,
    Date,
    Choice,
}
impl FieldKind {
    const ALL: [Self; 4] = [Self::Text, Self::Number, Self::Date, Self::Choice];
    fn key(self) -> &'static str {
        match self {
            Self::Text => "quantity.text",
            Self::Number => "quantity.number",
            Self::Date => "quantity.date",
            Self::Choice => "quantity.choice",
        }
    }
}
struct CustomDraft {
    name: String,
    kind: FieldKind,
    value: String,
    choices: String,
}
struct RowDraft {
    target: QuantityTarget,
    epoch: u64,
    price: String,
    currency: String,
    unit: QuantityUnit,
    rounding: u8,
    custom: Vec<CustomDraft>,
    applying: bool,
    error: Option<String>,
}
impl RowDraft {
    fn new(table: &QuantityTable, row: &QuantityRow, epoch: u64) -> Result<Self, EngineError> {
        Ok(Self {
            target: table.target(row)?,
            epoch,
            price: row
                .fields
                .unit_price
                .as_ref()
                .map(|p| p.amount.as_str().to_owned())
                .unwrap_or_default(),
            currency: row
                .fields
                .unit_price
                .as_ref()
                .map(|p| p.currency.clone())
                .unwrap_or_default(),
            unit: row
                .fields
                .unit_price
                .as_ref()
                .map(|p| p.per_unit)
                .or_else(|| row.quantity.as_ref().map(|q| q.unit))
                .unwrap_or(QuantityUnit::Each),
            rounding: row
                .fields
                .unit_price
                .as_ref()
                .map_or(2, |p| p.rounding_decimals),
            custom: row
                .fields
                .custom
                .iter()
                .map(|(name, value)| {
                    let (kind, value, choices) = match value {
                        TypedValue::Text { value } => {
                            (FieldKind::Text, value.clone(), String::new())
                        }
                        TypedValue::Date { value } => {
                            (FieldKind::Date, value.clone(), String::new())
                        }
                        TypedValue::Number { value } => {
                            (FieldKind::Number, value.as_str().to_owned(), String::new())
                        }
                        TypedValue::Choice { value, options } => {
                            (FieldKind::Choice, value.clone(), options.join("\n"))
                        }
                    };
                    CustomDraft {
                        name: name.clone(),
                        kind,
                        value,
                        choices,
                    }
                })
                .collect(),
            applying: false,
            error: None,
        })
    }
    fn fields(&self) -> Result<QuantityFields, EngineError> {
        let unit_price = if self.price.trim().is_empty() {
            None
        } else {
            Some(UnitPrice {
                amount: DecimalValue::parse(self.price.trim())?,
                currency: self.currency.trim().to_owned(),
                per_unit: self.unit,
                rounding_decimals: self.rounding,
            })
        };
        let mut custom = BTreeMap::new();
        for draft in &self.custom {
            let value = match draft.kind {
                FieldKind::Text => TypedValue::Text {
                    value: draft.value.clone(),
                },
                FieldKind::Date => TypedValue::Date {
                    value: draft.value.trim().into(),
                },
                FieldKind::Number => TypedValue::Number {
                    value: DecimalValue::parse(draft.value.trim())?,
                },
                FieldKind::Choice => TypedValue::Choice {
                    value: draft.value.clone(),
                    options: draft.choices.lines().map(str::to_owned).collect(),
                },
            };
            let name = draft.name.trim();
            if name.is_empty() || custom.insert(name.to_owned(), value).is_some() {
                return Err(EngineError::Other(
                    "custom column names must be nonempty and unique".into(),
                ));
            }
        }
        let fields = QuantityFields { unit_price, custom };
        fields.validate()?;
        Ok(fields)
    }
}

fn typed_text(value: &TypedValue) -> &str {
    match value {
        TypedValue::Text { value }
        | TypedValue::Date { value }
        | TypedValue::Choice { value, .. } => value,
        TypedValue::Number { value } => value.as_str(),
    }
}
fn short_label(ui: &mut egui::Ui, text: &str) {
    ui.add_sized(
        [170.0, 20.0],
        egui::Label::new(text.chars().take(60).collect::<String>()).truncate(),
    )
    .on_hover_text(text);
}

impl SuniApp {
    pub(crate) fn open_quantity_table(&mut self) {
        if !self.ink_can_start() || self.annotate.measure.is_active_draft() {
            return;
        }
        self.annotate.quantities.invalidate();
        self.annotate.quantities.edit = None;
        self.annotate.quantities.epoch = Some(self.view.epoch);
        self.annotate.quantities.open = true;
        self.annotate.register.open = false;
    }
    pub(crate) fn validate_quantity_context(&mut self) {
        let state = &mut self.annotate.quantities;
        if state.open && state.epoch != Some(self.view.epoch) {
            state.invalidate();
            state.epoch = Some(self.view.epoch);
            if let Some(edit) = &mut state.edit {
                edit.error = Some(tr(self.settings.language(), "quantity.stale").into());
            }
        }
    }
    pub(crate) fn on_quantity_table(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<QuantityTable, EngineError>,
    ) {
        let state = &mut self.annotate.quantities;
        if !state.open || epoch != self.view.epoch || state.pending != Some((epoch, generation)) {
            return;
        }
        state.pending = None;
        match result {
            Ok(table) => {
                state.report = Some(Arc::new(table));
                state.error = None;
                state.refresh_selection();
            }
            Err(error) => state.error = Some(error.to_string()),
        }
    }
    pub(crate) fn quantity_edit_failed(&mut self, error: &EngineError) {
        if let Some(edit) = self
            .annotate
            .quantities
            .edit
            .as_mut()
            .filter(|d| d.applying)
        {
            edit.applying = false;
            edit.error = Some(error.to_string());
        }
    }
    pub(crate) fn on_quantity_exported(
        &mut self,
        epoch: u64,
        generation: u64,
        result: Result<(), EngineError>,
    ) {
        let lang = self.lang();
        let state = &mut self.annotate.quantities;
        if state.exporting != Some((epoch, generation)) {
            return;
        }
        state.exporting = None;
        if !state.open || epoch != self.view.epoch {
            return;
        }
        match result {
            Ok(()) => state.message = Some(tr(lang, "quantity.exported").into()),
            Err(error) => state.error = Some(error.to_string()),
        }
    }
    fn export_quantity_path(&mut self, path: PathBuf, csv: bool) {
        let state = &mut self.annotate.quantities;
        let Some(table) = state.report.clone().filter(|table| table.can_export()) else {
            return;
        };
        if state.edit.is_some()
            || state.exporting.is_some()
            || state.epoch != Some(self.view.epoch)
            || self.worker.has_pending_mutations()
        {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        state.exporting = Some((self.view.epoch, state.generation));
        state.error = None;
        state.message = None;
        self.worker.send(Request::ExportQuantityTable {
            table,
            options: state.options.clone(),
            path,
            csv,
            epoch: self.view.epoch,
            generation: state.generation,
        });
    }
    pub(crate) fn quantity_table_dialog(&mut self, ctx: &egui::Context) {
        if self.settings.tables.placement == crate::workspace_table::Placement::Floating {
            self.quantity_table_ui(ctx, None);
        }
    }

    pub(crate) fn quantity_table_dock(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.settings.tables.placement == crate::workspace_table::Placement::Bottom {
            self.quantity_table_ui(ctx, Some(ui));
        }
    }

    fn quantity_table_ui(&mut self, ctx: &egui::Context, host: Option<&mut egui::Ui>) {
        if !self.annotate.quantities.open {
            return;
        }
        self.validate_quantity_context();
        let lang = self.lang();
        let epoch = self.view.epoch;
        let mut state = std::mem::take(&mut self.annotate.quantities);
        if state.report.is_none() && state.pending.is_none() && state.error.is_none() {
            state.generation = state.generation.wrapping_add(1);
            state.pending = Some((epoch, state.generation));
            self.worker.send(Request::QuantityTable {
                epoch,
                generation: state.generation,
            });
        }
        let report = state.report.clone();
        let idle = !self.worker.has_pending_mutations() && state.exporting.is_none();
        let can_refresh = idle && state.edit.is_none();
        let can_close =
            state.exporting.is_none() && state.edit.as_ref().is_none_or(|d| !d.applying);
        let can_export = can_refresh && report.as_ref().is_some_and(|r| r.can_export());
        let (mut close, mut refresh, mut apply, mut cancel, mut export) =
            (false, false, false, false, None);
        let mut navigate = None;
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
            dialog::Spec::new(
                "quantity-table",
                tr(lang, "quantity.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "quantity.help"))
            .width((ctx.content_rect().width() - 60.0).clamp(420.0, 1200.0)),
            |ui| {
                if let Some(error) = &state.error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
                if state.pending.is_some() || state.exporting.is_some() {
                    ui.spinner();
                    ui.label(tr(lang, "quantity.loading"));
                }
                if let Some(edit) = &mut state.edit {
                    ui.heading(tr(lang, "quantity.edit"));
                    draw_editor(ui, edit, lang);
                    ui.horizontal(|ui| {
                        apply = ui
                            .add_enabled(
                                idle && !edit.applying && edit.epoch == epoch,
                                egui::Button::new(tr(lang, "quantity.apply")),
                            )
                            .clicked();
                        cancel = ui
                            .add_enabled(
                                !edit.applying,
                                egui::Button::new(tr(lang, "quantity.cancel")),
                            )
                            .clicked();
                    });
                } else if let Some(report) = &report {
                    let mut changed = false;
                    ui.horizontal_wrapped(|ui| {
                        ui.menu_button(tr(lang, "workspace.table.filters"), |ui| {
                            ui.add_enabled_ui(idle, |ui| {
                                egui::Grid::new("quantity-filters")
                                    .num_columns(4)
                                    .show(ui, |ui| {
                                        for (i, (key, value)) in [
                                            ("register.search", &mut state.options.filter.query),
                                            ("register.kind", &mut state.options.filter.kind),
                                            (
                                                "register.discipline",
                                                &mut state.options.filter.discipline,
                                            ),
                                            (
                                                "register.business_status",
                                                &mut state.options.filter.business_status,
                                            ),
                                            (
                                                "register.count_group",
                                                &mut state.options.filter.count_group,
                                            ),
                                            ("register.author", &mut state.options.filter.author),
                                        ]
                                        .into_iter()
                                        .enumerate()
                                        {
                                            ui.label(tr(lang, key));
                                            changed |= ui
                                                .add(
                                                    egui::TextEdit::singleline(value)
                                                        .char_limit(512)
                                                        .desired_width(150.0),
                                                )
                                                .changed();
                                            if i % 2 == 1 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(lang, "register.page_filter"));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut state.page_filter)
                                                .range(0..=report.page_count()),
                                        )
                                        .changed();
                                    state.options.filter.page =
                                        (state.page_filter > 0).then_some(state.page_filter);
                                    let previous = state.options.group_by;
                                    ui.label(tr(lang, "quantity.group"));
                                    egui::ComboBox::from_id_salt("quantity-groups")
                                        .selected_text(tr(
                                            lang,
                                            &format!(
                                                "quantity.group.{}",
                                                state.options.group_by.id()
                                            ),
                                        ))
                                        .show_ui(ui, |ui| {
                                            for group in QuantityGrouping::ALL {
                                                ui.selectable_value(
                                                    &mut state.options.group_by,
                                                    group,
                                                    tr(
                                                        lang,
                                                        &format!("quantity.group.{}", group.id()),
                                                    ),
                                                );
                                            }
                                        });
                                    changed |= previous != state.options.group_by;
                                });
                            });
                        });
                        if let Some(summary) = &state.summary {
                            ui.menu_button(tr(lang, "workspace.table.summary"), |ui| {
                                draw_summary(ui, summary, lang);
                            });
                        }
                        ui.menu_button(tr(lang, "quantity.column_filter"), |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut state.column_filter)
                                    .hint_text(tr(lang, "quantity.column_filter"))
                                    .char_limit(128),
                            );
                        });
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
                            if state.selected.is_empty() {
                                0
                            } else {
                                state.offset + 1
                            },
                            (state.offset + 50).min(state.selected.len()),
                            state.selected.len()
                        ));
                        if ui
                            .add_enabled(
                                state.offset + 50 < state.selected.len(),
                                egui::Button::new(tr(lang, "register.next")),
                            )
                            .clicked()
                        {
                            state.offset += 50;
                        }
                    });
                    if changed {
                        state.refresh_selection();
                    }
                    if !report.can_export() {
                        ui.small(tr(lang, "quantity.no_export"));
                    }
                    let filter = state.column_filter.to_lowercase();
                    let columns: Vec<_> = state
                        .columns
                        .iter()
                        .filter(|name| name.to_lowercase().contains(&filter))
                        .take(16)
                        .collect();
                    if state.columns.len() > 16 {
                        ui.small(tr(lang, "quantity.more_columns"));
                    }
                    if selection != state.followed {
                        state.followed = selection;
                        if let Some(position) = state.selected.iter().position(|index| {
                            selection
                                == Some((
                                    report.rows()[*index].register.page,
                                    report.rows()[*index].register.index,
                                ))
                        }) {
                            state.offset = position / 50 * 50;
                        }
                    }
                    egui::ScrollArea::horizontal()
                        .id_salt("quantity-table-scroll")
                        .show(ui, |ui| {
                            egui::Grid::new("quantity-rows")
                                .striped(true)
                                .show(ui, |ui| {
                                    for key in [
                                        "register.page",
                                        "quantity.source",
                                        "register.kind",
                                        "register.contents",
                                        "quantity.quantity",
                                        "quantity.unit",
                                        "quantity.price",
                                        "quantity.cost",
                                    ] {
                                        ui.strong(tr(lang, key));
                                    }
                                    for name in &columns {
                                        ui.strong(name.as_str());
                                    }
                                    ui.strong(tr(lang, "register.actions"));
                                    ui.end_row();
                                    for index in state.selected.iter().skip(state.offset).take(50) {
                                        let row = &report.rows()[*index];
                                        if ui
                                            .add_enabled(
                                                idle && row.register.rect.is_some(),
                                                egui::Button::selectable(
                                                    selection
                                                        == Some((
                                                            row.register.page,
                                                            row.register.index,
                                                        )),
                                                    row.register.page.to_string(),
                                                ),
                                            )
                                            .on_hover_text(tr(lang, "register.view"))
                                            .clicked()
                                        {
                                            navigate = Some(row.register.clone());
                                        }
                                        ui.label(row.register.index.to_string());
                                        short_label(ui, &row.register.kind);
                                        short_label(
                                            ui,
                                            row.register
                                                .contents
                                                .as_deref()
                                                .unwrap_or(tr(lang, "quantity.missing")),
                                        );
                                        ui.label(
                                            row.quantity
                                                .as_ref()
                                                .map(|q| q.value.to_string())
                                                .unwrap_or_else(|| {
                                                    tr(lang, "quantity.missing").into()
                                                }),
                                        );
                                        ui.label(
                                            row.quantity.as_ref().map_or("—", |q| q.unit.id()),
                                        );
                                        ui.label(
                                            row.fields
                                                .unit_price
                                                .as_ref()
                                                .map(|p| {
                                                    format!(
                                                        "{} {} / {}",
                                                        p.amount.as_str(),
                                                        p.currency,
                                                        p.per_unit.id()
                                                    )
                                                })
                                                .unwrap_or_else(|| {
                                                    tr(lang, "quantity.missing").into()
                                                }),
                                        );
                                        ui.label(
                                            row.line_total
                                                .as_ref()
                                                .map(|cost| {
                                                    format!("{} {}", cost.amount, cost.currency)
                                                })
                                                .unwrap_or_else(|| {
                                                    tr(lang, "quantity.missing").into()
                                                }),
                                        );
                                        for name in &columns {
                                            short_label(
                                                ui,
                                                row.fields
                                                    .custom
                                                    .get(*name)
                                                    .map(typed_text)
                                                    .unwrap_or(tr(lang, "quantity.missing")),
                                            );
                                        }
                                        if ui
                                            .add_enabled(
                                                idle && row.editable,
                                                egui::Button::new(tr(lang, "quantity.edit")),
                                            )
                                            .on_disabled_hover_text(tr(lang, "quantity.readonly"))
                                            .clicked()
                                        {
                                            match RowDraft::new(report, row, epoch) {
                                                Ok(draft) => state.edit = Some(draft),
                                                Err(error) => state.error = Some(error.to_string()),
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                }
            },
            |ui| {
                refresh = ui
                    .add_enabled(can_refresh, egui::Button::new(tr(lang, "quantity.refresh")))
                    .clicked();
                for (key, csv) in [
                    ("quantity.export_csv", true),
                    ("quantity.export_json", false),
                ] {
                    if ui
                        .add_enabled(can_export, egui::Button::new(tr(lang, key)))
                        .clicked()
                    {
                        export = Some(csv);
                    }
                }
                close = ui
                    .add_enabled(can_close, egui::Button::new(tr(lang, "quantity.close")))
                    .clicked()
                    || (can_close
                        && ui.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                        }));
            },
        );
        if layout_changed {
            self.persist_settings();
        }
        if let Some(row) = navigate {
            self.follow_table_annotation(&row);
        }
        if apply {
            if let Some(edit) = &mut state.edit {
                match edit.fields() {
                    Ok(fields) => {
                        edit.applying = true;
                        edit.error = None;
                        self.worker.send(Request::Edit(EditOp::SetQuantityFields {
                            target: edit.target.clone(),
                            fields,
                        }));
                    }
                    Err(error) => edit.error = Some(error.to_string()),
                }
            }
        }
        if cancel {
            state.edit = None;
        }
        if refresh {
            state.invalidate();
        }
        if close {
            state.open = false;
            state.edit = None;
            state.invalidate();
        }
        self.annotate.quantities = state;
        if let Some(csv) = export {
            let extension = if csv { "csv" } else { "json" };
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("quantities.{extension}"))
                .add_filter(extension.to_uppercase(), &[extension])
                .save_file()
            {
                self.export_quantity_path(path, csv);
            }
        }
    }
}

fn draw_editor(ui: &mut egui::Ui, draft: &mut RowDraft, lang: Language) {
    ui.label(tr(lang, "quantity.edit_help"));
    ui.add_enabled_ui(!draft.applying, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr(lang, "quantity.price"));
            ui.add(
                egui::TextEdit::singleline(&mut draft.price)
                    .char_limit(32)
                    .desired_width(110.0),
            );
            ui.label(tr(lang, "quantity.currency"));
            ui.add(
                egui::TextEdit::singleline(&mut draft.currency)
                    .char_limit(3)
                    .desired_width(50.0),
            );
            ui.label(tr(lang, "quantity.per_unit"));
            egui::ComboBox::from_id_salt("quantity-price-unit")
                .selected_text(draft.unit.id())
                .show_ui(ui, |ui| {
                    for unit in QuantityUnit::ALL
                        .into_iter()
                        .filter(|unit| *unit != QuantityUnit::Degrees)
                    {
                        ui.selectable_value(&mut draft.unit, unit, unit.id());
                    }
                });
            ui.label(tr(lang, "quantity.rounding"));
            ui.add(egui::DragValue::new(&mut draft.rounding).range(0..=4));
        });
        ui.heading(tr(lang, "quantity.custom"));
        ui.small(tr(lang, "quantity.blank_custom"));
        let mut remove = None;
        for (index, field) in draft.custom.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "quantity.column_name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut field.name)
                                .char_limit(64)
                                .desired_width(160.0),
                        );
                        egui::ComboBox::from_id_salt("column-type")
                            .selected_text(tr(lang, field.kind.key()))
                            .show_ui(ui, |ui| {
                                for kind in FieldKind::ALL {
                                    ui.selectable_value(
                                        &mut field.kind,
                                        kind,
                                        tr(lang, kind.key()),
                                    );
                                }
                            });
                        if ui.button(tr(lang, "quantity.remove_column")).clicked() {
                            remove = Some(index);
                        }
                    });
                    ui.label(tr(lang, "quantity.value"));
                    ui.add(egui::TextEdit::singleline(&mut field.value).char_limit(1024));
                    if field.kind == FieldKind::Choice {
                        ui.label(tr(lang, "quantity.choices"));
                        ui.add(
                            egui::TextEdit::multiline(&mut field.choices)
                                .char_limit(4096)
                                .desired_rows(3),
                        );
                    }
                });
            });
        }
        if let Some(index) = remove {
            draft.custom.remove(index);
        }
        if ui
            .add_enabled(
                draft.custom.len() < 16,
                egui::Button::new(tr(lang, "quantity.add_column")),
            )
            .clicked()
        {
            draft.custom.push(CustomDraft {
                name: String::new(),
                kind: FieldKind::Text,
                value: String::new(),
                choices: String::new(),
            });
        }
    });
    if let Some(error) = &draft.error {
        ui.colored_label(egui::Color32::RED, error);
    }
}

fn draw_summary(ui: &mut egui::Ui, summary: &QuantitySummary, lang: Language) {
    ui.label(format!(
        "{}: {} · {}: {}",
        tr(lang, "register.matches"),
        summary.matched_rows,
        tr(lang, "quantity.missing"),
        summary.unquantified_rows
    ));
    egui::CollapsingHeader::new(tr(lang, "quantity.totals"))
        .default_open(true)
        .show(ui, |ui| {
            for group in summary.groups.iter().take(30) {
                ui.label(format!(
                    "{}: {} {} ({})",
                    group.group.as_deref().unwrap_or("—"),
                    group.quantity,
                    group.unit.id(),
                    group.rows
                ));
                if group.missing_prices > 0 {
                    ui.colored_label(egui::Color32::YELLOW, tr(lang, "quantity.partial"));
                }
                for cost in &group.costs {
                    ui.label(format!(
                        "{} {} · {} {}",
                        cost.total.amount,
                        cost.total.currency,
                        cost.priced_rows,
                        tr(lang, "quantity.quantity")
                    ));
                    if !cost.complete {
                        ui.small(tr(lang, "quantity.partial"));
                    }
                }
            }
            if summary.groups.len() > 30 {
                ui.label(tr(lang, "register.more_groups"));
            }
            if summary.unquantified_rows > 0 {
                ui.label(tr(lang, "quantity.partial"));
            }
        });
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::worker::Response;
    use egui::{Event, Modifiers, Pos2, Rect};
    struct Controls {
        ctx: egui::Context,
        time: f64,
    }
    impl Controls {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|s| s.animation_time = 0.0);
            Self { ctx, time: 1.0 }
        }
        fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> egui::FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1100.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.quantity_table_dock(&self.ctx, ui);
                    egui::Panel::left("quantity-test-measurements")
                        .exact_size(280.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .show(ui, |ui| app.measurement_bar(&self.ctx, ui));
                        });
                    app.quantity_table_dialog(&self.ctx);
                },
            );
            output.textures_delta.clear();
            output
        }
        fn click(&mut self, app: &mut SuniApp, key: &str) {
            fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().rev().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            self.frame(app, vec![]);
            let label = tr(app.lang(), key);
            let mut point = None;
            for attempt in 0..20 {
                let output = self.frame(app, vec![]);
                point = output
                    .shapes
                    .iter()
                    .rev()
                    .find_map(|s| find(&s.shape, label).filter(|p| s.clip_rect.contains(*p)));
                if point.is_some() {
                    break;
                }
                let at = if app.annotate.quantities.open {
                    Pos2::new(800.0, 990.0)
                } else {
                    Pos2::new(140.0, 600.0)
                };
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(at),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, if attempt < 10 { -150.0 } else { 150.0 }),
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Wheel,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            let point = point.unwrap_or_else(|| panic!("visible quantity control {key}"));
            self.time += 0.6;
            for pressed in [true, false] {
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(point),
                        Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            self.frame(app, vec![]);
        }
    }

    pub(crate) fn verify_restricted_controls(table: &QuantityTable) {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_quantity_table();
        let mut controls = Controls::new();
        controls.frame(&mut app, vec![]);
        let (epoch, generation) = app.annotate.quantities.pending.unwrap();
        app.on_quantity_table(epoch, generation, Ok(table.clone()));
        requests.try_iter().for_each(drop);
        controls.click(&mut app, "quantity.edit");
        assert!(app.annotate.quantities.edit.is_none());
        controls.click(&mut app, "quantity.export_csv");
        controls.click(&mut app, "quantity.export_json");
        let dir = tempfile::tempdir().unwrap();
        app.export_quantity_path(dir.path().join("refused.csv"), true);
        assert!(requests
            .try_iter()
            .all(|r| !matches!(r, Request::Edit(_) | Request::ExportQuantityTable { .. })));
        assert!(!dir.path().join("refused.csv").exists());
    }

    pub(crate) fn verify_controls(table: &QuantityTable) {
        assert!(table.rows().len() >= 2);
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            app.settings.language = language.into();
            app.annotate.tool = crate::annotate::Tool::Measure;
            let mut controls = Controls::new();
            controls.click(&mut app, "quantity.title");
            let (epoch, generation) = requests
                .try_iter()
                .find_map(|request| match request {
                    Request::QuantityTable { epoch, generation } => Some((epoch, generation)),
                    Request::Edit(_) => panic!("opening is read only"),
                    _ => None,
                })
                .unwrap();
            app.on_quantity_table(epoch, generation.wrapping_sub(1), Ok(table.clone()));
            assert!(app.annotate.quantities.report.is_none());
            app.on_quantity_table(epoch, generation, Ok(table.clone()));
            assert!(app.navigation_blocked());
            assert!(app.ink_blocks_command(sunipdf_core::Command::Undo));
            let active = app.annotate.quantities.report.as_ref().unwrap().clone();
            controls.frame(&mut app, vec![]);
            assert!(Arc::ptr_eq(
                &active,
                app.annotate.quantities.report.as_ref().unwrap()
            ));
            app.annotate.quantities.options.filter.page = Some(99);
            app.annotate.quantities.refresh_selection();
            assert!(app.annotate.quantities.selected.is_empty());
            app.annotate.quantities.options.filter.page = None;
            app.annotate.quantities.refresh_selection();
            controls.click(&mut app, "quantity.edit");
            let draft = app.annotate.quantities.edit.as_mut().unwrap();
            draft.price = "0".into();
            draft.currency = "CHF".into();
            draft.custom.push(CustomDraft {
                name: "Estimator".into(),
                kind: FieldKind::Text,
                value: String::new(),
                choices: String::new(),
            });
            let zero = draft.fields().unwrap();
            assert_eq!(zero.unit_price.unwrap().amount.as_str(), "0");
            assert_eq!(
                zero.custom.get("Estimator"),
                Some(&TypedValue::Text {
                    value: String::new()
                })
            );
            draft.price.clear();
            assert!(draft.fields().unwrap().unit_price.is_none());
            draft.price = "12.5".into();
            draft.custom.push(CustomDraft {
                name: "Inspection date".into(),
                kind: FieldKind::Date,
                value: "2026-02-30".into(),
                choices: String::new(),
            });
            assert!(draft.fields().is_err());
            draft.custom.pop();
            controls.click(&mut app, "quantity.cancel");
            assert!(app.annotate.quantities.edit.is_none());
            assert!(!requests.try_iter().any(|r| matches!(r, Request::Edit(_))));
            controls.click(&mut app, "quantity.edit");
            let draft = app.annotate.quantities.edit.as_mut().unwrap();
            draft.price = "12.5".into();
            draft.currency = "CHF".into();
            controls.click(&mut app, "quantity.apply");
            let edits: Vec<_> = requests
                .try_iter()
                .filter(|r| matches!(r, Request::Edit(_) | Request::FinishMutation))
                .collect();
            assert!(
                matches!(edits.as_slice(),[Request::Edit(EditOp::SetQuantityFields{fields,..}),Request::FinishMutation] if fields.unit_price.as_ref().unwrap().amount.as_str()=="12.5")
            );
            assert!(app.worker.has_pending_mutations());
            responses
                .send(Response::OperationFailed(EngineError::Other(
                    "simulated stale target".into(),
                )))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&controls.ctx);
            let draft = app.annotate.quantities.edit.as_ref().unwrap();
            assert!(!draft.applying);
            assert_eq!(draft.price, "12.5");
            assert!(draft.error.is_some());
            controls.click(&mut app, "quantity.cancel");
            let directory = tempfile::tempdir().unwrap();
            app.export_quantity_path(directory.path().join("quantities.csv"), true);
            let export = requests
                .try_iter()
                .find_map(|r| match r {
                    Request::ExportQuantityTable {
                        epoch,
                        generation,
                        table: sent,
                        csv,
                        ..
                    } => {
                        assert!(csv);
                        assert_eq!(sent.source_sha256(), table.source_sha256());
                        Some((epoch, generation))
                    }
                    _ => None,
                })
                .unwrap();
            controls.click(&mut app, "quantity.close");
            assert!(app.annotate.quantities.open, "wait for the accepted export");
            app.on_quantity_exported(export.0, export.1, Ok(()));
            controls.click(&mut app, "quantity.close");
            assert!(!app.annotate.quantities.open);
            app.on_quantity_table(epoch, generation, Ok(table.clone()));
            assert!(app.annotate.quantities.report.is_none());
            app.open_quantity_table();
            controls.frame(&mut app, vec![]);
            let pending = app.annotate.quantities.pending.unwrap();
            app.on_quantity_table(pending.0, pending.1, Ok(table.clone()));
            controls.click(&mut app, "quantity.edit");
            app.view.epoch = app.view.epoch.wrapping_add(1);
            app.validate_quantity_context();
            assert!(app
                .annotate
                .quantities
                .edit
                .as_ref()
                .unwrap()
                .error
                .is_some());
            controls.click(&mut app, "quantity.apply");
            assert!(!requests.try_iter().any(|r| matches!(r, Request::Edit(_))));
            controls.click(&mut app, "quantity.cancel");
            controls.click(&mut app, "quantity.close");
            assert!(!app.dirty);
        }
    }
}
