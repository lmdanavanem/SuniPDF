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
        DecimalValue, FormulaColumn, FormulaValue, QuantityFields, QuantityGrouping,
        QuantityOptions, QuantityRow, QuantitySummary, QuantityTable, QuantityTarget, QuantityUnit,
        TypedValue, UnitPrice,
    },
    EngineError,
};
use sunipdf_i18n::{tr, tr_args, Language};

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
    sort_columns: BTreeSet<String>,
    column_filter: String,
    summary: Option<QuantitySummary>,
    offset: usize,
    followed: Option<(usize, usize)>,
    page_filter: usize,
    error: Option<String>,
    message: Option<String>,
    edit: Option<RowDraft>,
    formula_draft: FormulaDraft,
    computed: BTreeMap<(usize, usize), Vec<FormulaValue>>,
}

struct FormulaDraft {
    name: String,
    expression: String,
    quantity_unit: QuantityUnit,
    rounding: u8,
    json: String,
    error: Option<String>,
}
impl Default for FormulaDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            expression: String::new(),
            quantity_unit: QuantityUnit::Each,
            rounding: 2,
            json: String::new(),
            error: None,
        }
    }
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
        self.sort_columns.clear();
        self.computed.clear();
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
        match report.evaluate_selection(&self.options) {
            Ok(evaluated) => {
                self.computed = evaluated.formulas;
                self.summary = Some(evaluated.summary);
            }
            Err(error) => {
                self.computed.clear();
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
    unverified_measurement: bool,
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
                .or_else(|| row.historical_quantity.as_ref().map(|q| q.unit))
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
            unverified_measurement: row
                .measurement_validity
                .is_some_and(|validity| !validity.is_current()),
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

// Quantity rows do not carry measurement display precision. Keep this UI-only
// fallback at six decimals, without changing geometry, pricing or export values.
fn quantity_number(value: f64) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    if value == 0.0 {
        return "0".into();
    }
    let trim = |text: &str| text.trim_end_matches('0').trim_end_matches('.').to_owned();
    if value.abs() < 0.000_001 || value.abs() >= 1_000_000_000_000.0 {
        let scientific = format!("{value:.6e}");
        let (mantissa, exponent) = scientific.split_once('e').expect("scientific value");
        format!("{}e{exponent}", trim(mantissa))
    } else {
        trim(&format!("{value:.6}"))
    }
}

fn quantity_warning(dark: bool) -> egui::Color32 {
    if dark {
        egui::Color32::from_rgb(250, 204, 21)
    } else {
        egui::Color32::from_rgb(133, 77, 14)
    }
}

fn quantity_value_label(
    ui: &mut egui::Ui,
    quantity: &sunipdf_core::quantity_table::QuantityValue,
    historical: bool,
    lang: Language,
) -> egui::Response {
    let value = quantity_number(quantity.value);
    let text = if historical {
        egui::RichText::new(tr_args(
            lang,
            "quantity.historical",
            &[("value", &value), ("unit", quantity.unit.id())],
        ))
        .color(quantity_warning(ui.visuals().dark_mode))
    } else {
        egui::RichText::new(value)
    };
    ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend))
        .on_hover_text(format!("{} {}", quantity.value, quantity.unit.id()))
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
                state.sort_columns = table
                    .rows()
                    .iter()
                    .flat_map(|row| row.fields.custom.keys().cloned())
                    .collect();
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
        let text_color = crate::theme::palette(self.settings.dark_theme).text;
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
        let editing = state.edit.as_ref().map(|edit| (edit.applying, edit.epoch));
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
                    // A form wraps within the viewport even after a wide grid has
                    // been scrolled. Its actions remain in the fixed footer.
                    ui.set_max_width(ui.clip_rect().width());
                    ui.heading(egui::RichText::new(tr(lang, "quantity.edit")).color(text_color));
                    if edit.unverified_measurement {
                        ui.colored_label(
                            quantity_warning(ui.visuals().dark_mode),
                            tr(lang, "quantity.unverified_help"),
                        );
                    }
                    draw_editor(ui, edit, lang);
                } else if let Some(report) = &report {
                    let mut changed = false;
                    ui.horizontal_wrapped(|ui| {
                        let active_filters = crate::annotation_register::filter_summary(
                            &state.options.filter,
                            report.plan_zones(),
                            lang,
                        );
                        let filter_label = if active_filters.is_empty() {
                            tr(lang, "workspace.table.filters").into()
                        } else {
                            format!(
                                "{} · {}",
                                tr(lang, "workspace.table.filters"),
                                active_filters.len()
                            )
                        };
                        ui.menu_button(filter_label, |ui| {
                            ui.add_enabled_ui(idle, |ui| {
                                changed |= crate::plan_zones_ui::zone_filter_ui(
                                    ui,
                                    lang,
                                    &mut state.options.filter,
                                    report.plan_zones(),
                                );
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
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(lang, "quantity.sort"));
                                    egui::ComboBox::from_id_salt("quantity-sort")
                                        .selected_text(tr(
                                            lang,
                                            &format!("register.{}", state.options.sort.id()),
                                        ))
                                        .show_ui(ui, |ui| {
                                            for sort in
                                                sunipdf_core::annotation_register::RegisterSort::ALL
                                            {
                                                if ui
                                                    .selectable_value(
                                                        &mut state.options.sort,
                                                        sort,
                                                        tr(
                                                            lang,
                                                            &format!("register.{}", sort.id()),
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    state.options.filter.custom_sort = None;
                                                    changed = true;
                                                }
                                            }
                                        });
                                    changed |= custom_sort(
                                        ui,
                                        lang,
                                        &mut state.options.filter.custom_sort,
                                        &state.sort_columns,
                                    );
                                    changed |= ui
                                        .checkbox(
                                            &mut state.options.descending,
                                            tr(lang, "register.descending"),
                                        )
                                        .changed();
                                });
                            });
                        })
                        .response
                        .on_hover_text(active_filters.join("\n"));
                        let coverage = report.zone_coverage(&state.options.filter);
                        let coverage_label = egui::RichText::new(if coverage.complete {
                            tr(lang, "workspace.table.summary").into()
                        } else {
                            format!("{} !", tr(lang, "workspace.table.summary"))
                        })
                        .color(if coverage.complete {
                            text_color
                        } else {
                            crate::theme::ERROR
                        });
                        ui.menu_button(coverage_label, |ui| {
                            crate::plan_zones_ui::coverage_ui(ui, lang, &coverage);
                            if let Some(summary) = &state.summary {
                                ui.separator();
                                draw_summary(ui, summary, lang);
                            }
                        })
                        .response
                        .on_hover_text(tr(
                            lang,
                            if coverage.complete {
                                "zones.coverage"
                            } else {
                                "zones.incomplete"
                            },
                        ));
                        ui.menu_button(tr(lang, "quantity.custom"), |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut state.column_filter)
                                    .hint_text(tr(lang, "quantity.column_filter"))
                                    .char_limit(128),
                            );
                            ui.separator();
                            ui.menu_button(tr(lang, "quantity.formulas"), |ui| {
                                ui.add_enabled_ui(idle, |ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height(360.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            changed |= draw_formulas(
                                                ui,
                                                &mut state.options,
                                                &mut state.formula_draft,
                                                lang,
                                            );
                                        });
                                });
                            });
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
                    ui.scope(|ui| {
                        egui::Grid::new("quantity-rows")
                            .striped(true)
                            .show(ui, |ui| {
                                for key in [
                                    "register.page",
                                    "quantity.source",
                                    "register.kind",
                                    "register.contents",
                                    "register.zone",
                                    "quantity.quantity",
                                    "quantity.unit",
                                    "quantity.price",
                                    "quantity.cost",
                                ] {
                                    ui.strong(egui::RichText::new(tr(lang, key)).color(text_color));
                                }
                                for name in &columns {
                                    ui.strong(egui::RichText::new(name.as_str()).color(text_color));
                                }
                                for column in &state.options.formulas {
                                    ui.strong(egui::RichText::new(&column.name).color(text_color))
                                        .on_hover_text(&column.expression);
                                }
                                ui.strong(
                                    egui::RichText::new(tr(lang, "register.actions"))
                                        .color(text_color),
                                );
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
                                    short_label(
                                        ui,
                                        &crate::plan_zones_ui::assignment_label(
                                            lang,
                                            &row.register.zone,
                                        ),
                                    );
                                    ui.vertical(|ui| {
                                        ui.set_min_width(140.0);
                                        if let Some(quantity) = &row.quantity {
                                            quantity_value_label(ui, quantity, false, lang);
                                        } else {
                                            ui.label(tr(lang, "quantity.missing"));
                                        }
                                        if let Some(historical) = &row.historical_quantity {
                                            quantity_value_label(ui, historical, true, lang);
                                        }
                                        if let Some(validity) = row
                                            .measurement_validity
                                            .filter(|validity| !validity.is_current())
                                        {
                                            ui.small(tr(
                                                lang,
                                                crate::measurement_saved::validity_key(validity),
                                            ));
                                        }
                                    });
                                    ui.label(row.quantity.as_ref().map_or("—", |q| q.unit.id()));
                                    ui.vertical(|ui| {
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
                                        if row.fields.unit_price.is_some()
                                            && row
                                                .measurement_validity
                                                .is_some_and(|validity| !validity.is_current())
                                        {
                                            ui.small(tr(lang, "quantity.stored_price"));
                                        }
                                    });
                                    ui.label(
                                        row.line_total
                                            .as_ref()
                                            .map(|cost| {
                                                format!("{} {}", cost.amount, cost.currency)
                                            })
                                            .unwrap_or_else(|| tr(lang, "quantity.missing").into()),
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
                                    if let Some(values) =
                                        state.computed.get(&(row.register.page, row.register.index))
                                    {
                                        for value in values {
                                            if let Some(error) = value.error {
                                                ui.colored_label(
                                                    quantity_warning(ui.visuals().dark_mode),
                                                    tr(
                                                        lang,
                                                        &format!(
                                                            "quantity.formula_error.{}",
                                                            error.id()
                                                        ),
                                                    ),
                                                );
                                            } else {
                                                ui.label(format!(
                                                    "{} {}",
                                                    value
                                                        .value
                                                        .as_deref()
                                                        .unwrap_or(tr(lang, "quantity.missing")),
                                                    value.unit.map_or("", |unit| unit.id())
                                                ));
                                            }
                                        }
                                    } else {
                                        for _ in &state.options.formulas {
                                            ui.label(tr(lang, "quantity.missing"));
                                        }
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
                if let Some((applying, draft_epoch)) = editing {
                    apply = ui
                        .add_enabled(
                            idle && !applying && draft_epoch == epoch,
                            egui::Button::new(tr(lang, "quantity.apply")),
                        )
                        .clicked();
                    cancel = ui
                        .add_enabled(!applying, egui::Button::new(tr(lang, "quantity.cancel")))
                        .clicked();
                    close = can_close
                        && ui
                            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    return;
                }
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

pub(crate) fn custom_sort(
    ui: &mut egui::Ui,
    lang: Language,
    selected: &mut Option<String>,
    names: &BTreeSet<String>,
) -> bool {
    let mut changed = false;
    ui.label(tr(lang, "quantity.sort_custom"));
    egui::ComboBox::from_id_salt("custom-column-sort")
        .selected_text(
            selected
                .as_deref()
                .unwrap_or(tr(lang, "quantity.sort_builtin")),
        )
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(selected, None, tr(lang, "quantity.sort_builtin"))
                .changed();
            for name in names {
                changed |= ui
                    .selectable_value(selected, Some(name.clone()), name)
                    .changed();
            }
        });
    changed
}

fn draw_formulas(
    ui: &mut egui::Ui,
    options: &mut QuantityOptions,
    draft: &mut FormulaDraft,
    lang: Language,
) -> bool {
    let mut changed = false;
    ui.set_max_width(540.0);
    ui.label(tr(lang, "quantity.formula_help"));
    let mut remove = None;
    for (index, column) in options.formulas.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} = {}", column.name, column.expression));
            if ui
                .small_button(tr(lang, "quantity.remove_column"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        options.formulas.remove(index);
        changed = true;
    }
    ui.separator();
    ui.label(tr(lang, "quantity.column_name"));
    ui.add(
        egui::TextEdit::singleline(&mut draft.name)
            .char_limit(64)
            .desired_width(340.0),
    );
    ui.label(tr(lang, "quantity.formula_expression"));
    ui.add(
        egui::TextEdit::singleline(&mut draft.expression)
            .char_limit(512)
            .desired_width(340.0),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(lang, "quantity.formula_unit"));
        egui::ComboBox::from_id_salt("formula-unit")
            .selected_text(draft.quantity_unit.id())
            .show_ui(ui, |ui| {
                for unit in QuantityUnit::ALL {
                    ui.selectable_value(&mut draft.quantity_unit, unit, unit.id());
                }
            });
        ui.label(tr(lang, "quantity.formula_rounding"));
        ui.add(egui::DragValue::new(&mut draft.rounding).range(0..=6));
    });
    if ui
        .add_enabled(
            options.formulas.len() < 8,
            egui::Button::new(tr(lang, "quantity.formula_add")),
        )
        .clicked()
    {
        let mut candidate = options.clone();
        candidate.formulas.push(FormulaColumn {
            name: draft.name.trim().into(),
            expression: draft.expression.trim().into(),
            quantity_unit: Some(draft.quantity_unit),
            rounding_decimals: draft.rounding,
        });
        match candidate.validate_formulas() {
            Ok(()) => {
                options.formulas = candidate.formulas;
                draft.error = None;
                changed = true;
            }
            Err(error) => draft.error = Some(error.to_string()),
        }
    }
    ui.collapsing(tr(lang, "quantity.formula_config"), |ui| {
        ui.label(tr(lang, "quantity.formula_config_help"));
        ui.add(
            egui::TextEdit::multiline(&mut draft.json)
                .char_limit(16384)
                .desired_rows(4)
                .desired_width(500.0),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr(lang, "quantity.formula_copy")).clicked() {
                draft.json = serde_json::to_string_pretty(&options.formulas).unwrap_or_default();
                ui.ctx().copy_text(draft.json.clone());
            }
            if ui.button(tr(lang, "quantity.formula_load")).clicked() {
                match QuantityOptions::formulas_from_json(draft.json.as_bytes()) {
                    Ok(formulas) => {
                        options.formulas = formulas;
                        draft.error = None;
                        changed = true;
                    }
                    Err(error) => draft.error = Some(error.to_string()),
                }
            }
        });
    });
    if let Some(error) = &draft.error {
        ui.colored_label(egui::Color32::RED, error);
    }
    changed
}

fn draw_editor(ui: &mut egui::Ui, draft: &mut RowDraft, lang: Language) {
    let text_color = crate::theme::palette(ui.visuals().dark_mode).text;
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
        ui.heading(egui::RichText::new(tr(lang, "quantity.custom")).color(text_color));
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
    let text_color = crate::theme::palette(ui.visuals().dark_mode).text;
    ui.label(format!(
        "{}: {} · {}: {}",
        tr(lang, "register.matches"),
        summary.matched_rows,
        tr(lang, "quantity.excluded_rows"),
        summary.excluded_quantity_rows
    ));
    ui.label(format!(
        "{}: {} · {}: {}",
        tr(lang, "quantity.additive_rows"),
        summary.additive_quantity_rows,
        tr(lang, "quantity.angular_rows"),
        summary.angular_rows
    ));
    if summary.unverified_measurement_rows > 0 {
        ui.colored_label(
            quantity_warning(ui.visuals().dark_mode),
            tr_args(
                lang,
                "quantity.unverified_rows",
                &[
                    ("total", &summary.unverified_measurement_rows.to_string()),
                    ("legacy", &summary.legacy_measurement_rows.to_string()),
                    ("stale", &summary.stale_measurement_rows.to_string()),
                ],
            ),
        );
        ui.small(tr(lang, "quantity.unverified_help"));
    }
    egui::CollapsingHeader::new(egui::RichText::new(tr(lang, "quantity.totals")).color(text_color))
        .default_open(true)
        .show(ui, |ui| {
            for group in summary.groups.iter().take(30) {
                ui.label(format!(
                    "{}: {} {} ({})",
                    group.group.as_deref().unwrap_or("—"),
                    quantity_number(group.quantity),
                    group.unit.id(),
                    group.rows
                ))
                .on_hover_text(format!("{} {}", group.quantity, group.unit.id()));
                if group.missing_prices > 0 {
                    ui.colored_label(
                        quantity_warning(ui.visuals().dark_mode),
                        tr(lang, "quantity.partial"),
                    );
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
            if summary.excluded_quantity_rows > 0 {
                ui.label(tr(lang, "quantity.partial"));
            }
        });
    if !summary.formula_groups.is_empty() {
        egui::CollapsingHeader::new(
            egui::RichText::new(tr(lang, "quantity.formula_totals")).color(text_color),
        )
        .default_open(true)
        .show(ui, |ui| {
            ui.small(tr(lang, "quantity.formula_subtotal_help"));
            for group in summary.formula_groups.iter().take(30) {
                ui.strong(format!(
                    "{} · {}",
                    group.formula_name,
                    group.group.as_deref().unwrap_or("—")
                ));
                ui.label(tr_args(
                    lang,
                    "quantity.formula_coverage",
                    &[
                        ("contributing", &group.contributing_rows.to_string()),
                        ("matched", &group.matched_rows.to_string()),
                        ("errors", &group.error_rows.to_string()),
                    ],
                ));
                for total in &group.totals {
                    ui.label(format!(
                        "{} {} ({})",
                        total.amount,
                        total
                            .unit
                            .map_or(tr(lang, "quantity.formula_scalar"), |unit| unit.id()),
                        total.contributing_rows
                    ));
                }
                if group.totals.is_empty() {
                    ui.label(tr(lang, "quantity.formula_no_results"));
                }
                if !group.complete {
                    ui.colored_label(
                        quantity_warning(ui.visuals().dark_mode),
                        tr(lang, "quantity.formula_partial"),
                    );
                }
                for (error, count) in &group.errors {
                    ui.small(format!(
                        "{}: {}",
                        tr(lang, &format!("quantity.formula_error.{error}")),
                        count
                    ));
                }
                ui.separator();
            }
            if summary.formula_groups.len() > 30 {
                ui.label(tr(lang, "register.more_groups"));
            }
        });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::worker::Response;
    use egui::{Event, Modifiers, Pos2, Rect};

    #[test]
    fn quantity_display_bounds_float_noise_without_hiding_small_nonzero_values() {
        for (value, expected) in [
            (6.000000457763672, "6"),
            (12.3456789, "12.345679"),
            (123.25, "123.25"),
            (42.0, "42"),
            (0.0, "0"),
            (-0.0, "0"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (1.23456789e-10, "1.234568e-10"),
            (1e12, "1e12"),
            (f64::MAX, "1.797693e308"),
        ] {
            assert_eq!(quantity_number(value), expected);
        }
        for value in [
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            1e-20,
            0.000000499999,
            0.000000999999,
            999_999_999_999.9,
            -f64::MAX,
        ] {
            let display = quantity_number(value);
            assert!(display.len() <= 20, "unbounded display: {display}");
            assert_ne!(display.parse::<f64>().unwrap(), 0.0);
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(quantity_number(value), "—");
        }
    }

    #[test]
    fn quantity_cells_stay_on_one_line_with_raw_tooltips_and_readable_history() {
        use sunipdf_core::quantity_table::QuantityValue;

        fn collect(shape: &egui::Shape, labels: &mut Vec<(String, usize)>) {
            match shape {
                egui::Shape::Text(text) => {
                    labels.push((text.galley.text().to_owned(), text.galley.rows.len()));
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, labels);
                    }
                }
                _ => {}
            }
        }
        let quantity = QuantityValue {
            value: 6.000000457763672,
            unit: QuantityUnit::SquareMetres,
        };
        let original = serde_json::to_vec(&quantity).unwrap();
        for lang in Language::ALL {
            for dark in [false, true] {
                for historical in [false, true] {
                    let ctx = egui::Context::default();
                    crate::theme::apply(&ctx, dark);
                    ctx.all_styles_mut(|style| {
                        style.animation_time = 0.0;
                        style.interaction.tooltip_delay = 0.0;
                    });
                    let display = if historical {
                        tr_args(
                            lang,
                            "quantity.historical",
                            &[("value", "6"), ("unit", quantity.unit.id())],
                        )
                    } else {
                        "6".into()
                    };
                    let raw = format!("{} {}", quantity.value, quantity.unit.id());
                    let mut pointer = None;
                    let mut saw_tooltip = false;
                    for frame in 0..6 {
                        let input = egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                egui::vec2(360.0, 180.0),
                            )),
                            time: Some(frame as f64 * 0.5),
                            events: pointer.map(Event::PointerMoved).into_iter().collect(),
                            ..Default::default()
                        };
                        let mut output = ctx.run_ui(input, |ui| {
                            ui.set_max_width(32.0);
                            ui.vertical(|ui| {
                                ui.set_min_width(140.0);
                                pointer = Some(
                                    quantity_value_label(ui, &quantity, historical, lang)
                                        .rect
                                        .center(),
                                );
                            });
                        });
                        let mut labels = Vec::new();
                        for shape in &output.shapes {
                            collect(&shape.shape, &mut labels);
                        }
                        assert!(labels
                            .iter()
                            .any(|(text, rows)| text == &display && *rows == 1));
                        crate::workspace_table::assert_visible_text_contrast(
                            &output,
                            &[&display],
                            dark,
                        );
                        saw_tooltip |= labels.iter().any(|(text, _)| text == &raw);
                        output.textures_delta.clear();
                    }
                    assert!(saw_tooltip, "raw quantity remains inspectable");
                    assert_eq!(serde_json::to_vec(&quantity).unwrap(), original);
                }
            }
        }
    }

    #[test]
    fn formula_summary_displays_partial_coverage_and_separate_valid_units_in_both_languages() {
        use sunipdf_core::quantity_table::{FormulaGroupSummary, FormulaSubtotal};
        fn collect(shape: &egui::Shape, text: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(value) => text.push(value.galley.text().to_owned()),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
                _ => {}
            }
        }
        let summary = QuantitySummary {
            matched_rows: 2,
            quantity_rows: 2,
            additive_quantity_rows: 1,
            angular_rows: 1,
            unquantified_rows: 0,
            excluded_quantity_rows: 1,
            unverified_measurement_rows: 0,
            legacy_measurement_rows: 0,
            stale_measurement_rows: 0,
            groups: Vec::new(),
            formula_groups: vec![
                FormulaGroupSummary {
                    formula_name: "Separated".into(),
                    group: Some("Room A".into()),
                    matched_rows: 2,
                    contributing_rows: 2,
                    error_rows: 0,
                    errors: BTreeMap::new(),
                    complete: true,
                    totals: vec![
                        FormulaSubtotal {
                            unit: Some(QuantityUnit::Metres),
                            rounding_decimals: 2,
                            amount: "1.00".into(),
                            contributing_rows: 1,
                        },
                        FormulaSubtotal {
                            unit: Some(QuantityUnit::Feet),
                            rounding_decimals: 2,
                            amount: "2.00".into(),
                            contributing_rows: 1,
                        },
                    ],
                },
                FormulaGroupSummary {
                    formula_name: "Missing".into(),
                    group: None,
                    matched_rows: 2,
                    contributing_rows: 0,
                    error_rows: 2,
                    errors: BTreeMap::from([("missing_field".into(), 2)]),
                    complete: false,
                    totals: Vec::new(),
                },
            ],
        };
        for lang in Language::ALL {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
            let mut labels = Vec::new();
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(1200.0, 1200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| draw_summary(ui, &summary, lang),
                );
                labels.clear();
                for shape in &output.shapes {
                    collect(&shape.shape, &mut labels);
                }
                output.textures_delta.clear();
            }
            assert!(labels.iter().any(|text| text == "1.00 m (1)"));
            assert!(labels.iter().any(|text| text == "2.00 ft (1)"));
            assert_eq!(
                labels
                    .iter()
                    .filter(|text| text.as_str() == tr(lang, "quantity.formula_partial"))
                    .count(),
                1,
                "separated valid units are not presented as a calculation failure"
            );
            assert!(labels
                .iter()
                .any(|text| text == tr(lang, "quantity.formula_no_results")));
            assert!(labels
                .iter()
                .any(|text| text.contains(tr(lang, "quantity.angular_rows"))));
            assert!(labels
                .iter()
                .any(|text| text.contains(&format!("{}: 1", tr(lang, "quantity.excluded_rows")))));
        }
    }

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
            let mut candidate_bounds = Vec::new();
            for attempt in 0..20 {
                let output = self.frame(app, vec![]);
                candidate_bounds = output
                    .shapes
                    .iter()
                    .rev()
                    .filter_map(|shape| {
                        find(&shape.shape, label).map(|point| (point, shape.clip_rect))
                    })
                    .collect();
                point = candidate_bounds
                    .iter()
                    .find_map(|(point, clip)| clip.contains(*point).then_some(*point));
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
            let point = point.unwrap_or_else(|| {
                panic!(
                    "visible quantity control {key}; language={}, editing={}, candidates={candidate_bounds:?}",
                    app.settings.language,
                    app.annotate.quantities.edit.is_some()
                )
            });
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

    fn verify_compact_table(table: &QuantityTable) {
        fn text_rects(shape: &egui::Shape, label: &str, rects: &mut Vec<Rect>) {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    rects.push(Rect::from_min_size(text.pos, text.galley.size()));
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_rects(shape, label, rects);
                    }
                }
                _ => {}
            }
        }
        for language in ["en-US", "fr-FR"] {
            for dark in [false, true] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.settings.language = language.into();
                app.settings.dark_theme = dark;
                app.open_quantity_table();
                let ctx = egui::Context::default();
                crate::theme::apply(&ctx, dark);
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(720.0, 480.0))),
                    ..Default::default()
                };
                for editing in [false, true] {
                    if editing {
                        let mut draft =
                            RowDraft::new(table, &table.rows()[0], app.view.epoch).unwrap();
                        draft.custom = (0..16)
                            .map(|index| CustomDraft {
                                name: format!("Field {index}"),
                                kind: FieldKind::Text,
                                value: "Long form fixture".into(),
                                choices: String::new(),
                            })
                            .collect();
                        draft.error = Some(
                        "The selected annotation changed; review this draft before applying it."
                            .into(),
                    );
                        app.annotate.quantities.edit = Some(draft);
                    }
                    let mut canvas = Rect::NOTHING;
                    let mut output = None;
                    for _ in 0..6 {
                        let mut frame = ctx.run_ui(input.clone(), |ui| {
                            egui::Panel::top("compact-reader-top")
                                .exact_size(95.0)
                                .frame(egui::Frame::NONE)
                                .show(ui, |_| {});
                            egui::Panel::bottom("compact-reader-bottom")
                                .exact_size(79.0)
                                .frame(egui::Frame::NONE)
                                .show(ui, |_| {});
                            egui::Panel::right("compact-reader-rail")
                                .exact_size(48.0)
                                .frame(egui::Frame::NONE)
                                .show(ui, |_| {});
                            app.quantity_table_dock(&ctx, ui);
                            egui::CentralPanel::no_frame().show(ui, |ui| {
                                canvas = ui.max_rect();
                                ui.painter().rect_filled(canvas, 0.0, egui::Color32::GRAY);
                            });
                        });
                        frame.textures_delta.clear();
                        output = Some(frame);
                        if let Some((epoch, generation)) = app.annotate.quantities.pending {
                            app.on_quantity_table(epoch, generation, Ok(table.clone()));
                        }
                    }
                    let output = output.unwrap();
                    let contrast_keys = if editing {
                        vec!["quantity.title"]
                    } else {
                        vec![
                            "quantity.title",
                            "register.page",
                            "quantity.source",
                            "register.kind",
                            "register.contents",
                        ]
                    };
                    crate::workspace_table::assert_visible_text_contrast(
                        &output,
                        &contrast_keys
                            .iter()
                            .map(|key| tr(app.lang(), key))
                            .collect::<Vec<_>>(),
                        dark,
                    );
                    assert!(
                        canvas.height() >= 150.0,
                        "reader canvas retained: {canvas:?}"
                    );
                    let contents = table.rows()[0]
                        .register
                        .contents
                        .as_deref()
                        .unwrap_or(tr(app.lang(), "quantity.missing"))
                        .chars()
                        .take(60)
                        .collect::<String>();
                    let labels = if editing {
                        vec![
                            tr(app.lang(), "quantity.apply"),
                            tr(app.lang(), "quantity.cancel"),
                        ]
                    } else {
                        vec![
                            tr(app.lang(), "quantity.title"),
                            tr(app.lang(), "workspace.table.dock"),
                            tr(app.lang(), "workspace.table.float"),
                            tr(app.lang(), "quantity.close"),
                            tr(app.lang(), "quantity.custom"),
                            contents.as_str(),
                        ]
                    };
                    for label in labels {
                        let mut bounds = Vec::new();
                        for shape in &output.shapes {
                            let mut rects = Vec::new();
                            text_rects(&shape.shape, label, &mut rects);
                            bounds.extend(rects.into_iter().map(|rect| (rect, shape.clip_rect)));
                        }
                        assert!(
                            bounds.iter().any(|(rect, clip)| {
                                clip.contains_rect(*rect)
                                    && rect.top() >= canvas.bottom()
                                    && rect.bottom() <= 401.0
                            }),
                            "full table label must remain visible outside the canvas: {language}, {label}, editing={editing}; \
                     canvas={canvas:?}, text/clip bounds={bounds:?}"
                        );
                    }
                    if editing {
                        // The compact form scrolls independently of its fixed
                        // actions. Check its headings with enough bounded room,
                        // without assuming they stay on screen during scrolling.
                        let editor_ctx = egui::Context::default();
                        crate::theme::apply(&editor_ctx, dark);
                        app.settings.tables.height = 360.0;
                        let mut editor_output = None;
                        for _ in 0..4 {
                            let mut frame = editor_ctx.run_ui(
                                egui::RawInput {
                                    screen_rect: Some(Rect::from_min_size(
                                        Pos2::ZERO,
                                        egui::vec2(1600.0, 1100.0),
                                    )),
                                    ..Default::default()
                                },
                                |ui| app.quantity_table_dock(&editor_ctx, ui),
                            );
                            frame.textures_delta.clear();
                            editor_output = Some(frame);
                        }
                        crate::workspace_table::assert_visible_text_contrast(
                            &editor_output.unwrap(),
                            &["quantity.title", "quantity.edit", "quantity.custom"]
                                .map(|key| tr(app.lang(), key)),
                            dark,
                        );
                    }
                }
                assert!(requests
                    .try_iter()
                    .all(|request| !matches!(request, Request::Edit(_))));
            }
        }
    }

    pub(crate) fn verify_controls(table: &QuantityTable) {
        assert!(table.rows().len() >= 2);
        verify_compact_table(table);
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
            assert!(!app.navigation_blocked());
            assert!(!app.ink_blocks_command(sunipdf_core::Command::ZoomIn));
            app.zoom = crate::app::Zoom::Fixed(1.0);
            app.execute(&controls.ctx, sunipdf_core::Command::ZoomIn);
            assert_eq!(app.zoom, crate::app::Zoom::Fixed(1.25));
            assert!(app.annotate.quantities.open, "reading keeps the table open");
            let active = app.annotate.quantities.report.as_ref().unwrap().clone();
            controls.frame(&mut app, vec![]);
            assert!(Arc::ptr_eq(
                &active,
                app.annotate.quantities.report.as_ref().unwrap()
            ));
            app.annotate.quantities.options.filter.page = Some(99);
            let available_sort_columns = app.annotate.quantities.sort_columns.clone();
            app.annotate.quantities.refresh_selection();
            assert!(app.annotate.quantities.selected.is_empty());
            assert_eq!(
                app.annotate.quantities.sort_columns, available_sort_columns,
                "custom sort options stay available when filters hide all rows"
            );
            app.annotate.quantities.options.filter.page = None;
            app.annotate.quantities.refresh_selection();
            controls.click(&mut app, "quantity.edit");
            assert!(app.navigation_blocked());
            assert!(app.ink_blocks_command(sunipdf_core::Command::Undo));
            let zoom = app.zoom;
            app.execute(&controls.ctx, sunipdf_core::Command::ZoomIn);
            assert_eq!(
                app.zoom, zoom,
                "a pending field draft keeps document guards"
            );
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
            assert!(!app.navigation_blocked());
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
            app.annotate.quantities.options.formulas = vec![FormulaColumn {
                name: "Preview".into(),
                expression: "1 + 2".into(),
                quantity_unit: None,
                rounding_decimals: 2,
            }];
            app.annotate.quantities.refresh_selection();
            assert!(!app.annotate.quantities.computed.is_empty());
            assert!(app
                .annotate
                .quantities
                .computed
                .values()
                .all(|values| values[0].value.as_deref() == Some("3.00")));
            app.export_quantity_path(directory.path().join("quantities.csv"), true);
            assert!(app.navigation_blocked());
            assert!(app.ink_blocks_command(sunipdf_core::Command::Undo));
            let zoom = app.zoom;
            app.execute(&controls.ctx, sunipdf_core::Command::ZoomIn);
            assert_eq!(app.zoom, zoom, "an accepted export keeps document guards");
            let export = requests
                .try_iter()
                .find_map(|r| match r {
                    Request::ExportQuantityTable {
                        epoch,
                        generation,
                        table: sent,
                        csv,
                        options,
                        ..
                    } => {
                        assert!(csv);
                        assert_eq!(sent.source_sha256(), table.source_sha256());
                        assert_eq!(options.formulas[0].expression, "1 + 2");
                        Some((epoch, generation))
                    }
                    _ => None,
                })
                .unwrap();
            controls.click(&mut app, "quantity.close");
            assert!(app.annotate.quantities.open, "wait for the accepted export");
            app.on_quantity_exported(export.0, export.1, Ok(()));
            assert!(!app.navigation_blocked());
            controls.click(&mut app, "quantity.close");
            assert!(!app.annotate.quantities.open);
            assert!(
                app.annotate.quantities.computed.is_empty(),
                "closed snapshots discard computed values"
            );
            assert!(app.annotate.quantities.sort_columns.is_empty());
            app.annotate.quantities.options.formulas.clear();
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
