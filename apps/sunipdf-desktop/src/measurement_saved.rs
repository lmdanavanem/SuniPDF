//! Saved page scales and inspection of validated measurement records.

use crate::{
    annotate::Tool,
    app::SuniApp,
    dialog,
    worker::{EditOp, Request},
};
use std::collections::BTreeMap;
use std::sync::Arc;
use sunipdf_core::{
    measurement::{
        MeasurementKind, MeasurementRecord, MeasurementValidity, MeasurementViewport,
        PageCalibration, VectorSnapGeometry,
    },
    Command, EngineError,
};
use sunipdf_i18n::{tr, tr_args};

const MAX_CACHED_PAGES: usize = 8;
const MAX_PENDING_PAGES: usize = 4;

pub(crate) fn validity_key(validity: MeasurementValidity) -> &'static str {
    match validity {
        MeasurementValidity::Current => "measure.validity.current",
        MeasurementValidity::LegacyUnverified => "measure.validity.legacy_unverified",
        MeasurementValidity::Stale => "measure.validity.stale",
    }
}

#[derive(Clone)]
pub(crate) struct MeasurementPageData {
    pub calibration: Result<Option<PageCalibration>, String>,
    pub records: Result<Arc<[MeasurementRecord]>, String>,
    pub viewports: Result<Arc<[MeasurementViewport]>, String>,
    pub viewport_reset_target: Option<sunipdf_core::measurement::MeasurementViewportResetTarget>,
    pub snap_geometry: Result<Arc<VectorSnapGeometry>, String>,
    pub can_modify: bool,
    pub can_annotate: bool,
    labels: Arc<[String]>,
}

impl MeasurementPageData {
    pub(crate) fn new(
        calibration: Result<Option<PageCalibration>, String>,
        records: Result<Vec<MeasurementRecord>, String>,
        can_modify: bool,
    ) -> Self {
        let labels = records.as_ref().map_or_else(
            |_| Vec::new(),
            |records| {
                records
                    .iter()
                    .map(|record| {
                        let m = &record.measurement;
                        m.value()
                            .map(|value| {
                                let unit = match m.kind {
                                    MeasurementKind::Angle => "deg".into(),
                                    MeasurementKind::Area => format!("{}2", m.unit.id()),
                                    MeasurementKind::Volume => format!("{}3", m.unit.id()),
                                    _ => m.unit.id().into(),
                                };
                                format!("{:.*} {}", m.precision, value, unit)
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            },
        );
        Self {
            calibration,
            records: records.map(Arc::from),
            viewports: Ok(Arc::from([])),
            viewport_reset_target: None,
            snap_geometry: Ok(Arc::new(VectorSnapGeometry::default())),
            can_modify,
            can_annotate: can_modify,
            labels: labels.into(),
        }
    }
}

#[derive(Default)]
pub(crate) struct SavedMeasurements {
    pages: BTreeMap<usize, MeasurementPageData>,
    pending: BTreeMap<usize, (u64, u64)>,
    generation: u64,
    pub(crate) applying: Option<(usize, bool)>,
    inspector: Option<usize>,
    offset: usize,
    error: Option<String>,
}

impl SavedMeasurements {
    pub(crate) fn can_edit(&self, page: usize, record: &MeasurementRecord) -> bool {
        record.validity.is_current()
            && self.pages.get(&page).is_some_and(|data| data.can_annotate)
            && self.contains(page, record)
    }

    pub(crate) fn contains(&self, page: usize, record: &MeasurementRecord) -> bool {
        self.pages
            .get(&page)
            .and_then(|data| data.records.as_ref().ok())
            .is_some_and(|records| records.contains(record))
    }

    pub(crate) fn invalidate(&mut self) {
        self.pages.clear();
        self.pending.clear();
        self.generation = self.generation.wrapping_add(1);
        self.error = None;
    }

    pub(crate) fn dialog_open(&self) -> bool {
        self.inspector.is_some() || self.applying.is_some()
    }

    pub(crate) fn page_data(&self, page: usize) -> Option<&MeasurementPageData> {
        self.pages.get(&page)
    }

    pub(crate) fn calibration(&self, page: usize) -> Option<&PageCalibration> {
        self.pages.get(&page)?.calibration.as_ref().ok()?.as_ref()
    }

    fn refresh(&mut self, page: usize) {
        self.pages.remove(&page);
        self.pending.remove(&page);
        self.error = None;
    }
}

impl SuniApp {
    pub(crate) fn request_measurement_page_if_needed(&mut self) {
        let page = self
            .annotate
            .measure
            .saved
            .inspector
            .or_else(|| self.document.as_ref().map(|d| d.page));
        if let Some(page) = page {
            self.request_measurement_page(page);
        }
    }

    pub(crate) fn request_measurement_page(&mut self, page: usize) {
        if self.annotate.tool != Tool::Measure
            || !self
                .document
                .as_ref()
                .is_some_and(|d| page < d.info.page_count())
            || self.worker.has_pending_mutations()
            || self.saving
            || self.processing.busy
            || self.office.busy
            || self.ocr.busy
            || self.printing.busy
            || self.opening.is_some()
            || self.tabs.resuming.is_some()
            || self.tabs.suspending.is_some()
            || self.document_transition_pending()
        {
            return;
        }
        let saved = &mut self.annotate.measure.saved;
        if saved.pages.contains_key(&page)
            || saved.pending.contains_key(&page)
            || saved.pending.len() >= MAX_PENDING_PAGES
        {
            return;
        }
        saved.generation = saved.generation.wrapping_add(1);
        saved
            .pending
            .insert(page, (self.view.epoch, saved.generation));
        self.worker.send(Request::MeasurementPage {
            page,
            epoch: self.view.epoch,
            generation: saved.generation,
        });
    }

    pub(crate) fn on_measurement_page(
        &mut self,
        page: usize,
        epoch: u64,
        generation: u64,
        result: Result<MeasurementPageData, EngineError>,
    ) {
        let saved = &mut self.annotate.measure.saved;
        if epoch != self.view.epoch || saved.pending.get(&page) != Some(&(epoch, generation)) {
            return;
        }
        saved.pending.remove(&page);
        let data = match result {
            Ok(data) => data,
            Err(error) => {
                let mut data =
                    MeasurementPageData::new(Err(error.to_string()), Err(error.to_string()), false);
                data.viewports = Err(error.to_string());
                data.snap_geometry = Err(error.to_string());
                data
            }
        };
        self.annotate.measure.scales_ui.accept_page(page, &data);
        let saved = &mut self.annotate.measure.saved;
        // A late read populates only the PDF cache; a newer local calibration remains authoritative.
        saved.pages.insert(page, data);
        while saved.pages.len() > MAX_CACHED_PAGES {
            let evicted = saved.pages.keys().copied().find(|candidate| {
                *candidate != page
                    && Some(*candidate) != saved.inspector
                    && !self
                        .document
                        .as_ref()
                        .is_some_and(|doc| doc.page == *candidate)
            });
            if let Some(evicted) = evicted {
                saved.pages.remove(&evicted);
            } else {
                break;
            }
        }
    }

    pub(crate) fn measurement_scale_changed(&mut self) {
        self.measurement_advanced_scale_changed();
        if let Some((page, remove)) = self.annotate.measure.saved.applying.take() {
            if !remove {
                self.annotate.measure.saved_scale_applied(page);
            }
            self.edit.pending_status = Some(tr(self.lang(), "measure.saved.changed").into());
        }
    }

    pub(crate) fn measurement_scale_failed(&mut self, error: &EngineError) {
        self.measurement_advanced_scale_failed(error);
        let saved = &mut self.annotate.measure.saved;
        if saved.applying.take().is_some() {
            saved.error = Some(error.to_string());
        }
    }

    pub(crate) fn save_measurement_scale(&mut self, page: usize, remove: bool) {
        self.validate_measurement_context();
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return;
        }
        let Some(data) = self.annotate.measure.saved.pages.get(&page) else {
            return;
        };
        if !data.can_modify {
            return;
        }
        let calibration = if remove {
            if matches!(data.calibration, Ok(None)) {
                return;
            }
            None
        } else {
            let Some(calibration) = self.annotate.measure.local_calibration(page) else {
                return;
            };
            Some(calibration)
        };
        self.annotate.measure.saved.applying = Some((page, remove));
        self.annotate.measure.saved.error = None;
        self.worker.send(Request::Edit(EditOp::SetPageCalibration {
            page,
            calibration,
        }));
    }

    pub(crate) fn open_saved_measurements(&mut self) {
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return;
        }
        if let Some(document) = &self.document {
            self.annotate.measure.saved.inspector = Some(document.page);
            self.annotate.measure.saved.offset = 0;
            self.request_measurement_page(document.page);
        }
    }

    pub(crate) fn measurement_saved_scale_controls(&mut self, ui: &mut egui::Ui) {
        use crate::measure_tools::measurement_panel_button;
        let Some(page) = self.document.as_ref().map(|d| d.page) else {
            return;
        };
        let lang = self.lang();
        let ready = self.measurement_ready() && !self.annotate.measure.is_active_draft();
        let local = self.annotate.measure.local_calibration(page).is_some();
        let data = self.annotate.measure.saved.pages.get(&page);
        let can_modify = data.is_some_and(|d| d.can_modify);
        let stored = data.is_some_and(|d| !matches!(d.calibration, Ok(None)));
        if self.annotate.measure.saved.applying.is_some()
            || self.annotate.measure.saved.pending.contains_key(&page)
        {
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.label(tr(lang, "measure.saved.loading"));
            });
        } else {
            ui.small(tr(
                lang,
                if local {
                    "measure.saved.session"
                } else if data.is_some_and(|d| d.calibration.is_err()) {
                    "measure.saved.unavailable"
                } else if stored {
                    "measure.saved.pdf"
                } else {
                    "measure.saved.none"
                },
            ));
        }
        let save = measurement_panel_button(
            ui,
            ready && local && can_modify,
            tr(lang, "measure.saved.save"),
        )
        .clicked();
        let reset = measurement_panel_button(
            ui,
            ready && local,
            tr(
                lang,
                if data.is_some_and(|d| matches!(d.calibration, Ok(Some(_)))) {
                    "measure.saved.restore"
                } else {
                    "measure.saved.clear_session"
                },
            ),
        )
        .on_hover_text(tr(lang, "measure.saved.reset_hint"))
        .clicked();
        let remove = measurement_panel_button(
            ui,
            ready && stored && can_modify,
            tr(lang, "measure.saved.remove"),
        )
        .clicked();
        let error_color = crate::theme::palette(self.settings.dark_theme).error;
        if let Some(data) = data {
            if !can_modify {
                ui.small(tr(lang, "measure.saved.restricted"));
            }
            if let Err(error) = &data.calibration {
                ui.colored_label(error_color, error);
            }
        }
        if let Some(error) = &self.annotate.measure.saved.error {
            ui.colored_label(error_color, error);
        }
        if save {
            self.save_measurement_scale(page, false);
        }
        if reset {
            self.reset_temporary_measurement_scale(page);
        }
        if remove {
            self.save_measurement_scale(page, true);
        }
    }

    pub(crate) fn measurement_saved_bar(&mut self, ui: &mut egui::Ui) {
        use crate::measure_tools::measurement_panel_button;
        let Some(page) = self.document.as_ref().map(|d| d.page) else {
            return;
        };
        let lang = self.lang();
        let ready = self.measurement_ready() && !self.annotate.measure.is_active_draft();
        if measurement_panel_button(ui, ready, tr(lang, "measure.saved.inspect")).clicked() {
            self.open_saved_measurements();
        }
        if measurement_panel_button(ui, ready, tr(lang, "register.refresh")).clicked() {
            self.annotate.measure.saved.refresh(page);
            self.request_measurement_page(page);
        }
    }

    pub(crate) fn saved_measurements_dialog(&mut self, ctx: &egui::Context) {
        let Some(page) = self.annotate.measure.saved.inspector else {
            return;
        };
        if !self
            .document
            .as_ref()
            .is_some_and(|d| page < d.info.page_count())
        {
            self.annotate.measure.saved.inspector = None;
            return;
        }
        self.request_measurement_page(page);
        let lang = self.lang();
        let data = self.annotate.measure.saved.pages.get(&page).cloned();
        let enabled =
            !self.worker.has_pending_mutations() && self.annotate.measure.saved.applying.is_none();
        let mut close = false;
        let mut refresh = false;
        let mut view = None;
        let mut use_scale = None;
        let mut edit = None;
        let mut geometry = None;
        let mut offset = self.annotate.measure.saved.offset;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "saved-measurements",
                tr(lang, "measure.saved.inspect"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr_args(
                lang,
                "measure.saved.page",
                &[("page", &(page + 1).to_string())],
            ))
            .width((ctx.content_rect().width() - 60.0).clamp(420.0, 850.0)),
            |ui| {
                ui.label(tr(lang, "measure.saved.help"));
                match data.as_ref().map(|d| &d.records) {
                    None => {
                        ui.spinner();
                        ui.label(tr(lang, "measure.saved.loading"));
                    }
                    Some(Err(error)) => {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                    Some(Ok(records)) => {
                        if records.is_empty() {
                            ui.label(tr(lang, "measure.saved.empty"));
                        }
                        offset = offset.min(records.len().saturating_sub(1) / 50 * 50);
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    offset > 0,
                                    egui::Button::new(tr(lang, "register.previous")),
                                )
                                .clicked()
                            {
                                offset = offset.saturating_sub(50);
                            }
                            ui.label(format!(
                                "{}–{} / {}",
                                if records.is_empty() { 0 } else { offset + 1 },
                                (offset + 50).min(records.len()),
                                records.len()
                            ));
                            if ui
                                .add_enabled(
                                    offset + 50 < records.len(),
                                    egui::Button::new(tr(lang, "register.next")),
                                )
                                .clicked()
                            {
                                offset += 50;
                            }
                        });
                        for (row, record) in records.iter().enumerate().skip(offset).take(50) {
                            let measurement = &record.measurement;
                            let label = data
                                .as_ref()
                                .and_then(|d| d.labels.get(row))
                                .filter(|label| !label.is_empty())
                                .map_or_else(|| tr(lang, "measure.invalid"), String::as_str);
                            ui.group(|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(label);
                                    ui.label(tr(
                                        lang,
                                        &format!("measure.{}", measurement.kind.id()),
                                    ));
                                    ui.label(format!("#{}", record.index));
                                    if ui
                                        .add_enabled(
                                            enabled,
                                            egui::Button::new(tr(lang, "measure.edit.open")),
                                        )
                                        .clicked()
                                    {
                                        edit = Some(record.clone());
                                    }
                                    if ui
                                        .add_enabled(
                                            enabled
                                                && record.validity.is_current()
                                                && data.as_ref().is_some_and(|d| d.can_annotate),
                                            egui::Button::new(tr(lang, "measure.geometry.open")),
                                        )
                                        .on_disabled_hover_text(tr(
                                            lang,
                                            if record.validity.is_current() {
                                                "measure.saved.restricted"
                                            } else {
                                                "measure.validity.geometry_blocked"
                                            },
                                        ))
                                        .clicked()
                                    {
                                        geometry = Some(record.clone());
                                    }
                                    if ui
                                        .add_enabled(
                                            enabled,
                                            egui::Button::new(tr(lang, "register.view")),
                                        )
                                        .clicked()
                                    {
                                        view = Some(record.clone());
                                    }
                                    if ui
                                        .add_enabled(
                                            enabled && self.saved_scale_in_page(page, record),
                                            egui::Button::new(tr(lang, "measure.saved.use")),
                                        )
                                        .on_disabled_hover_text(tr(
                                            lang,
                                            "measure.saved.use_unavailable",
                                        ))
                                        .clicked()
                                    {
                                        use_scale = Some(record.clone());
                                    }
                                });
                                ui.label(tr(lang, validity_key(record.validity)));
                                if !record.validity.is_current() {
                                    ui.colored_label(
                                        crate::theme::palette(self.settings.dark_theme).error,
                                        tr(lang, "measure.validity.historical"),
                                    );
                                    ui.small(tr(
                                        lang,
                                        if measurement.kind == MeasurementKind::Angle {
                                            "measure.validity.recreate_angle"
                                        } else {
                                            "measure.validity.recalibrate_help"
                                        },
                                    ));
                                }
                                ui.label(tr_args(
                                    lang,
                                    "measure.saved.detail",
                                    &[
                                        ("points", &measurement.points.len().to_string()),
                                        ("precision", &measurement.precision.to_string()),
                                    ],
                                ));
                                if measurement.kind != MeasurementKind::Angle {
                                    ui.small(format!(
                                        "{}: {} {}",
                                        tr(lang, "measure.known_distance"),
                                        measurement.known_distance,
                                        measurement.unit.id()
                                    ));
                                }
                                if let Some(id) = &record.annotation_id {
                                    ui.small(id);
                                }
                            });
                        }
                    }
                }
            },
            |ui| {
                close = ui.button(tr(lang, "register.close")).clicked()
                    || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                refresh = ui
                    .add_enabled(enabled, egui::Button::new(tr(lang, "register.refresh")))
                    .clicked();
            },
        );
        self.annotate.measure.saved.offset = offset;
        if refresh {
            self.annotate.measure.saved.refresh(page);
            self.request_measurement_page(page);
        }
        if let Some(record) = use_scale {
            if self.use_saved_measurement_scale(page, &record) {
                close = true;
            }
        }
        if let Some(record) = view {
            close = true;
            self.annotate.measure.saved.inspector = None;
            self.annotate.tool = Tool::SelectAnnot;
            crate::workspace::follow(
                &mut self.annotate,
                &mut self.organize,
                &mut self.edit,
                Command::ToolSelectAnnot,
            );
            self.annotate.comments_open = true;
            self.annotate.annotations_page = None;
            self.annotate.adv.reselect = Some((page, record.index));
            if let Some(&(x, y)) = record.measurement.points.first() {
                let height = self.document.as_ref().unwrap().info.page_sizes[page].height_pt;
                self.go_to_page_point(page, x, (height - y).max(0.0));
            }
        }
        if let Some(record) = edit {
            self.annotate.measure.saved.inspector = None;
            if self.open_saved_measurement_editor(page, record) {
                close = true;
            } else {
                self.annotate.measure.saved.inspector = Some(page);
            }
        }
        if let Some(record) = geometry {
            self.annotate.measure.saved.inspector = None;
            if self.start_measurement_geometry(page, record) {
                close = true;
            } else {
                self.annotate.measure.saved.inspector = Some(page);
            }
        }
        if close {
            self.annotate.measure.saved.inspector = None;
        }
    }

    pub(crate) fn reset_temporary_measurement_scale(&mut self, page: usize) -> bool {
        self.validate_measurement_context();
        if !self.measurement_ready() || self.annotate.measure.is_active_draft() {
            return false;
        }
        self.annotate.measure.clear_temporary_scale(page)
    }

    pub(crate) fn use_saved_measurement_scale(
        &mut self,
        page: usize,
        record: &MeasurementRecord,
    ) -> bool {
        self.validate_measurement_context();
        if self.worker.has_pending_mutations()
            || self.annotate.measure.is_active_draft()
            || !self.saved_scale_in_page(page, record)
            || !self
                .annotate
                .measure
                .saved
                .pages
                .get(&page)
                .and_then(|d| d.records.as_ref().ok())
                .is_some_and(|records| records.contains(record))
        {
            return false;
        }
        self.annotate.measure.use_calibration(
            page,
            &PageCalibration {
                points: record.measurement.calibration,
                known_distance: record.measurement.known_distance,
                unit: record.measurement.unit,
            },
        )
    }

    fn saved_scale_in_page(&self, page: usize, record: &MeasurementRecord) -> bool {
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
        else {
            return false;
        };
        record.validity.is_current()
            && record.measurement.kind != MeasurementKind::Angle
            && record.measurement.calibration.iter().all(|&(x, y)| {
                x.is_finite()
                    && y.is_finite()
                    && x >= 0.0
                    && y >= 0.0
                    && x <= size.width_pt
                    && y <= size.height_pt
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::{DocState, Response};
    use egui::{Event, FullOutput, Modifiers, Pos2, Rect};
    use sunipdf_core::measurement::{Measurement, MeasurementUnit};

    fn calibration(distance: f64) -> PageCalibration {
        PageCalibration {
            points: [(50.0, 60.0), (150.0, 60.0)],
            known_distance: distance,
            unit: MeasurementUnit::Metres,
        }
    }

    fn record(kind: MeasurementKind) -> MeasurementRecord {
        MeasurementRecord {
            validity: MeasurementValidity::Current,
            index: 3,
            annotation_id: Some("plan-measurement-1".into()),
            measurement: Measurement {
                holes: Vec::new(),
                depth: None,
                kind,
                points: vec![(50.0, 100.0), (150.0, 100.0)],
                calibration: calibration(10.0).points,
                known_distance: 10.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            },
        }
    }

    fn data(
        saved: Option<PageCalibration>,
        records: Vec<MeasurementRecord>,
    ) -> MeasurementPageData {
        MeasurementPageData::new(Ok(saved), Ok(records), true)
    }

    fn ready(app: &mut SuniApp) {
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.open_measurements(&egui::Context::default());
        app.validate_measurement_context();
    }

    fn changed(app: &SuniApp) -> Response {
        Response::DocumentChanged(DocState {
            info: app.document.as_ref().unwrap().info.clone(),
            can_undo: true,
            can_redo: false,
            modified: true,
            pages_unchanged: true,
            measurement_scales_unchanged: true,
            has_signatures: false,
        })
    }

    fn query(requests: &std::sync::mpsc::Receiver<Request>) -> (usize, u64, u64) {
        let requests: Vec<_> = requests.try_iter().collect();
        assert_eq!(
            requests.len(),
            1,
            "reading calibration must not mutate the PDF"
        );
        match requests.into_iter().next().unwrap() {
            Request::MeasurementPage {
                page,
                epoch,
                generation,
            } => (page, epoch, generation),
            _ => panic!("measurement page query"),
        }
    }

    #[test]
    fn saved_measurement_reads_ignore_stale_tokens_and_do_not_replace_local_scales() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        app.request_measurement_page(0);
        let (page, epoch, generation) = query(&requests);
        app.annotate.measure.use_calibration(0, &calibration(25.0));
        app.on_measurement_page(
            page,
            epoch + 1,
            generation,
            Ok(data(Some(calibration(1.0)), vec![])),
        );
        app.on_measurement_page(
            page,
            epoch,
            generation + 1,
            Ok(data(Some(calibration(2.0)), vec![])),
        );
        assert!(app.annotate.measure.saved.calibration(0).is_none());
        app.on_measurement_page(
            page,
            epoch,
            generation,
            Ok(data(Some(calibration(10.0)), vec![])),
        );
        assert_eq!(
            app.annotate.measure.saved.calibration(0),
            Some(&calibration(10.0))
        );
        assert_eq!(
            app.annotate.measure.local_calibration(0),
            Some(calibration(25.0))
        );
        app.annotate.measure.saved.refresh(0);
        app.request_measurement_page(0);
        let (_, _, replacement) = query(&requests);
        assert_ne!(replacement, generation);
        app.on_measurement_page(
            page,
            epoch,
            generation,
            Ok(data(Some(calibration(3.0)), vec![])),
        );
        assert!(app.annotate.measure.saved.calibration(0).is_none());
        app.on_measurement_page(page, epoch, replacement, Ok(data(None, vec![])));
        assert_eq!(
            app.annotate.measure.local_calibration(0),
            Some(calibration(25.0))
        );
    }

    #[test]
    fn browsing_many_pages_bounds_pending_reads_and_cached_records_without_losing_session_scales() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 24];
        app.annotate.measure.use_calibration(0, &calibration(25.0));
        app.annotate.measure.use_calibration(23, &calibration(50.0));
        for page in 0..24 {
            app.request_measurement_page(page);
        }
        let jobs: Vec<_> = requests.try_iter().collect();
        assert_eq!(jobs.len(), MAX_PENDING_PAGES);
        assert_eq!(app.annotate.measure.saved.pending.len(), MAX_PENDING_PAGES);
        for request in jobs {
            let Request::MeasurementPage {
                page,
                epoch,
                generation,
            } = request
            else {
                panic!("read-only page request")
            };
            app.on_measurement_page(
                page,
                epoch,
                generation,
                Ok(data(
                    Some(calibration(10.0)),
                    vec![record(MeasurementKind::Length)],
                )),
            );
        }
        assert!(app.annotate.measure.saved.pending.is_empty());
        app.annotate.measure.saved.inspector = Some(1);
        for page in MAX_PENDING_PAGES..24 {
            app.request_measurement_page(page);
            let (_, epoch, generation) = query(&requests);
            app.on_measurement_page(
                page,
                epoch,
                generation,
                Ok(data(
                    Some(calibration(10.0)),
                    vec![record(MeasurementKind::Length)],
                )),
            );
            assert!(app.annotate.measure.saved.pages.len() <= MAX_CACHED_PAGES);
            assert!(app.annotate.measure.saved.pages.contains_key(&page));
            assert!(
                app.annotate.measure.saved.pages.contains_key(&0),
                "retain the current page"
            );
            assert!(
                app.annotate.measure.saved.pages.contains_key(&1),
                "retain the inspector page"
            );
        }
        assert_eq!(
            app.annotate.measure.local_calibration(0),
            Some(calibration(25.0))
        );
        assert_eq!(
            app.annotate.measure.local_calibration(23),
            Some(calibration(50.0))
        );
    }

    #[test]
    fn restored_tabs_reload_pdf_scales_and_reject_old_responses() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        app.annotate.measure.use_calibration(0, &calibration(25.0));
        app.annotate
            .measure
            .saved
            .pages
            .insert(0, data(Some(calibration(10.0)), vec![]));
        let original_epoch = app.view.epoch;
        let tab = app.take_tab();
        assert!(app.annotate.measure.local_calibration(0).is_none());
        app.put_tab(tab);
        assert!(app.annotate.measure.saved.calibration(0).is_none());
        assert_eq!(
            app.annotate.measure.local_calibration(0),
            Some(calibration(25.0))
        );
        app.request_measurement_page(0);
        let (_, epoch, generation) = query(&requests);
        assert_ne!(epoch, original_epoch);
        app.on_measurement_page(
            0,
            original_epoch,
            generation,
            Ok(data(Some(calibration(1.0)), vec![])),
        );
        assert!(app.annotate.measure.saved.calibration(0).is_none());
        app.on_measurement_page(
            0,
            epoch,
            generation,
            Ok(data(Some(calibration(10.0)), vec![])),
        );
        assert_eq!(
            app.annotate.measure.saved.calibration(0),
            Some(&calibration(10.0))
        );
    }

    struct Controls {
        ctx: egui::Context,
        time: f64,
    }

    impl Controls {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                time: 1.0,
            }
        }

        fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1100.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.measurement_bar(&self.ctx, ui);
                    app.saved_measurements_dialog(&self.ctx);
                    app.rich_annotation_dialog(&self.ctx);
                },
            );
            output.textures_delta.clear();
            output
        }

        fn click(&mut self, app: &mut SuniApp, key: &str) {
            let label = tr(app.lang(), key).to_string();
            self.click_text(app, &label);
        }

        fn click_text(&mut self, app: &mut SuniApp, label: &str) {
            fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let pos = output
                .shapes
                .iter()
                .rev()
                .find_map(|s| find(&s.shape, label))
                .unwrap_or_else(|| panic!("visible control: {label}"));
            self.time += 0.6;
            for pressed in [true, false] {
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
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

    #[test]
    fn historical_records_require_an_explicit_current_reference_before_preview() {
        for language in ["en-US", "fr-FR"] {
            for validity in [
                MeasurementValidity::LegacyUnverified,
                MeasurementValidity::Stale,
            ] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                ready(&mut app);
                app.settings.language = language.into();
                let mut original = record(MeasurementKind::Length);
                original.validity = validity;
                app.annotate
                    .measure
                    .saved
                    .pages
                    .insert(0, data(Some(calibration(10.0)), vec![original.clone()]));
                assert!(!app.annotate.measure.saved.can_edit(0, &original));
                assert!(!app.use_saved_measurement_scale(0, &original));
                assert!(!app.start_measurement_geometry(0, original.clone()));
                assert!(app.open_saved_measurement_editor(0, original.clone()));
                app.request_rich_preview();
                assert!(requests.try_recv().is_err());
                let mut controls = Controls::new();
                controls.click(&mut app, "measure.validity.recalibrate");
                app.request_rich_preview();
                assert!(
                    requests.try_recv().is_err(),
                    "a saved page default cannot silently verify an old record"
                );
                app.annotate.measure.use_calibration(0, &calibration(25.0));
                controls.click(&mut app, "measure.validity.recalibrate");
                controls.click(&mut app, "rich.preview");
                let request = requests.recv().unwrap();
                assert!(matches!(request, Request::PreviewMeasurementRecalibration {
                    page: 0, record, measurement, calibration: reference, ..
                } if record == original
                    && measurement.points == original.measurement.points
                    && reference == calibration(25.0)
                    && measurement.known_distance == 25.0));
                assert_eq!(
                    app.annotate.measure.saved.pages[&0]
                        .records
                        .as_ref()
                        .unwrap()[0],
                    original,
                    "preview must not promote the cached historical record",
                );
            }
        }
    }

    #[test]
    fn cached_measurement_rows_share_records_and_labels_without_serializing_per_frame() {
        let records = (0..4096)
            .map(|index| {
                let mut record = record(MeasurementKind::Length);
                record.index = index;
                record
            })
            .collect();
        let original = data(None, records);
        let cloned = original.clone();
        assert!(Arc::ptr_eq(
            original.records.as_ref().unwrap(),
            cloned.records.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(&original.labels, &cloned.labels));
        assert_eq!(cloned.labels.len(), 4096);
        assert_eq!(cloned.labels[4095], "10.00 m");
    }

    #[test]
    fn geometry_controls_preserve_original_scale_and_allow_preview_correction_then_commit() {
        use crate::view::DragEvent;
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            ready(&mut app);
            app.settings.language = language.into();
            app.effective_zoom = 1.0;
            app.annotate.measure.use_calibration(0, &calibration(25.0));
            let mut original = record(MeasurementKind::Length);
            original.measurement.unit = MeasurementUnit::Feet;
            original.measurement.precision = 3;
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(5.0)), vec![original.clone()]));
            let mut controls = Controls::new();
            controls.click(&mut app, "measure.saved.inspect");
            controls.click(&mut app, "measure.geometry.open");
            assert!(app.annotate.measure.is_active_draft());
            assert!(app.navigation_blocked());
            assert!(requests.try_recv().is_err());
            app.measurement_geometry_event(0, DragEvent::Start((150.0, 100.0)));
            app.measurement_geometry_event(0, DragEvent::Move((190.0, 100.0)));
            app.measurement_geometry_move(0, (225.0, 125.0));
            app.measurement_geometry_event(0, DragEvent::Stop);
            controls.click(&mut app, "measure.geometry.preview");
            let (replacement, epoch, generation) = match requests.recv().unwrap() {
                Request::PreviewMeasurementEdit {
                    page: 0,
                    record,
                    measurement,
                    epoch,
                    generation,
                } => {
                    assert_eq!(record, original);
                    assert_eq!(measurement.points, vec![(50.0, 100.0), (225.0, 125.0)]);
                    assert_eq!(measurement.calibration, original.measurement.calibration);
                    assert_eq!(
                        measurement.known_distance,
                        original.measurement.known_distance
                    );
                    assert_eq!(measurement.unit, original.measurement.unit);
                    assert_eq!(measurement.precision, original.measurement.precision);
                    (measurement, epoch, generation)
                }
                _ => panic!("geometry edit preview"),
            };
            assert!(!app.annotate.measure.is_active_draft());
            controls.click(&mut app, "bar.apply");
            assert!(
                requests.try_recv().is_err(),
                "pending preview cannot commit"
            );
            app.on_rich_preview(
                &controls.ctx,
                epoch,
                generation,
                Err(EngineError::Other("preview refused".into())),
            );
            controls.click(&mut app, "measure.geometry.adjust");
            assert!(
                matches!(requests.recv().unwrap(), Request::DiscardRichAnnotationPreview { epoch: e, generation: g } if e == epoch && g == generation)
            );
            assert_eq!(
                app.annotate.measure.geometry.as_ref().unwrap().replacement,
                replacement
            );
            assert!(app.annotate.adv.rich.is_none());
            controls.click(&mut app, "measure.geometry.preview");
            let Request::PreviewMeasurementEdit {
                epoch, generation, ..
            } = requests.recv().unwrap()
            else {
                panic!("second preview")
            };
            let preview = || sunipdf_core::annot_rich::RichAnnotationPreview {
                image: sunipdf_core::RenderedPage {
                    page_index: 0,
                    width: 4,
                    height: 2,
                    rgba: vec![255; 32],
                },
                scale: 1.0,
                region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
                index: original.index,
                prepared: None,
            };
            app.on_rich_preview(
                &controls.ctx,
                epoch,
                generation.wrapping_sub(1),
                Ok(preview()),
            );
            assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
            app.on_rich_preview(&controls.ctx, epoch, generation, Ok(preview()));
            controls.click(&mut app, "bar.apply");
            assert!(
                matches!(requests.recv().unwrap(), Request::Edit(EditOp::CommitRichAnnotationPreview { epoch: e, generation: g }) if e == epoch && g == generation)
            );
            assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
            assert!(requests.try_recv().is_err(), "exactly one guarded mutation");
        }
    }

    #[test]
    fn geometry_drafts_cancel_and_reject_readonly_stale_contexts_or_changed_identity() {
        use crate::view::DragEvent;
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        app.effective_zoom = 1.0;
        let original = record(MeasurementKind::Length);
        let mut readonly = data(None, vec![original.clone()]);
        readonly.can_annotate = false;
        app.annotate.measure.saved.pages.insert(0, readonly);
        assert!(!app.start_measurement_geometry(0, original.clone()));
        let mut annotatable = data(None, vec![original.clone()]);
        annotatable.can_modify = false;
        app.annotate.measure.saved.pages.insert(0, annotatable);
        assert!(
            app.start_measurement_geometry(0, original.clone()),
            "annotation rights are independent of page modification rights"
        );
        assert!(app.cancel_measurement());
        for cause in 0..5 {
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(None, vec![original.clone()]));
            assert!(app.start_measurement_geometry(0, original.clone()));
            assert!(app.ink_blocks_command(Command::ToolSelect));
            assert!(!app.ink_blocks_command(Command::ZoomIn));
            app.measurement_geometry_event(0, DragEvent::Start((150.0, 100.0)));
            app.measurement_geometry_move(1, (250.0, 100.0));
            assert_eq!(
                app.annotate.measure.geometry.as_ref().unwrap().replacement,
                original.measurement
            );
            app.measurement_geometry_move(0, (200.0, 100.0));
            app.measurement_geometry_event(0, DragEvent::Stop);
            match cause {
                0 => {
                    assert!(app.cancel_measurement());
                }
                1 => {
                    app.view.epoch = app.view.epoch.wrapping_add(1);
                    app.validate_measurement_context();
                }
                2 => {
                    app.annotate.measure.saved.invalidate();
                    app.validate_measurement_context();
                }
                3 => {
                    app.annotate
                        .measure
                        .saved
                        .pages
                        .get_mut(&0)
                        .unwrap()
                        .can_annotate = false;
                    app.validate_measurement_context();
                }
                _ => {
                    let size = app.document.as_ref().unwrap().info.page_sizes[0];
                    app.document.as_mut().unwrap().info.page_sizes.push(size);
                    app.go_to_page(1);
                    assert_eq!(
                        app.document.as_ref().unwrap().page,
                        0,
                        "navigation is blocked"
                    );
                    app.document.as_mut().unwrap().page = 1;
                    app.validate_measurement_context();
                    app.document.as_mut().unwrap().page = 0;
                }
            }
            assert!(!app.annotate.measure.is_active_draft());
            assert!(!app.finish_measurement_geometry());
            assert!(app.annotate.adv.rich.is_none());
            assert!(
                requests.try_recv().is_err(),
                "cancel and stale drafts never edit or preview"
            );
        }
    }

    #[test]
    fn temporary_scale_controls_restore_pdf_scale_or_clear_only_the_current_page() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            ready(&mut app);
            app.settings.language = language.into();
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(5.0)), vec![]));
            app.annotate.measure.use_calibration(0, &calibration(20.0));
            app.annotate.measure.use_calibration(1, &calibration(30.0));
            let mut controls = Controls::new();
            controls.click(&mut app, "measure.saved.restore");
            assert!(app.annotate.measure.local_calibration(0).is_none());
            assert_eq!(
                app.annotate.measure.saved.calibration(0),
                Some(&calibration(5.0))
            );
            assert_eq!(
                app.annotate.measure.local_calibration(1),
                Some(calibration(30.0))
            );
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(None, vec![]));
            app.annotate.measure.use_calibration(0, &calibration(20.0));
            controls.click(&mut app, "measure.saved.clear_session");
            assert!(app.annotate.measure.local_calibration(0).is_none());
            assert!(app.annotate.measure.saved.calibration(0).is_none());
            app.annotate.measure.use_calibration(0, &calibration(20.0));
            app.annotate.measure.start_calibration();
            assert!(
                !app.reset_temporary_measurement_scale(0),
                "active gestures keep their scale"
            );
            assert_eq!(
                app.annotate.measure.local_calibration(0),
                Some(calibration(20.0))
            );
            assert!(
                requests.try_recv().is_err(),
                "session controls must never edit the PDF"
            );
        }
    }

    #[test]
    fn inspector_edit_converts_units_and_requires_preview_before_apply_or_cancel() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            ready(&mut app);
            app.settings.language = language.into();
            let original = record(MeasurementKind::Length);
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(5.0)), vec![original.clone()]));
            let mut controls = Controls::new();
            controls.click(&mut app, "measure.saved.inspect");
            controls.click(&mut app, "measure.edit.open");
            assert!(!app.annotate.measure.saved.dialog_open());
            assert!(app.navigation_blocked());
            assert!(
                requests.try_recv().is_err(),
                "opening an editor has no side effects"
            );
            controls.click_text(&mut app, "m");
            controls.click_text(&mut app, "cm");
            let draft = app.annotate.adv.rich.as_ref().unwrap();
            assert_eq!(draft.measure_unit, MeasurementUnit::Centimetres);
            assert_eq!(draft.known_distance, 1000.0);
            controls.click(&mut app, "bar.apply");
            assert!(
                requests.try_recv().is_err(),
                "Apply requires a completed preview"
            );
            controls.click(&mut app, "rich.preview");
            let (epoch, generation) = match requests.recv().unwrap() {
                Request::PreviewMeasurementEdit {
                    page: 0,
                    record,
                    measurement,
                    epoch,
                    generation,
                } => {
                    assert_eq!(record, original);
                    assert_eq!(measurement.points, original.measurement.points);
                    assert_eq!(measurement.calibration, original.measurement.calibration);
                    assert_eq!(measurement.value().unwrap(), 1000.0);
                    (epoch, generation)
                }
                _ => panic!("guarded measurement edit preview"),
            };
            let preview = || sunipdf_core::annot_rich::RichAnnotationPreview {
                image: sunipdf_core::RenderedPage {
                    page_index: 0,
                    width: 4,
                    height: 2,
                    rgba: vec![255; 32],
                },
                scale: 1.0,
                region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
                index: original.index,
                prepared: None,
            };
            app.on_rich_preview(
                &controls.ctx,
                epoch,
                generation.wrapping_sub(1),
                Ok(preview()),
            );
            assert!(
                !app.annotate.adv.rich.as_ref().unwrap().prepared,
                "late response ignored"
            );
            app.on_rich_preview(&controls.ctx, epoch, generation, Ok(preview()));
            controls.click(&mut app, "bar.apply");
            assert!(
                matches!(requests.recv().unwrap(), Request::Edit(EditOp::CommitRichAnnotationPreview { epoch:e,generation:g }) if e==epoch && g==generation)
            );
            assert!(app.annotate.adv.rich.as_ref().unwrap().applying);
            assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
            assert!(app.worker.has_pending_mutations());
            responses
                .send(Response::OperationFailed(EngineError::Other(
                    "test refusal".into(),
                )))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&controls.ctx);
            assert!(!app.worker.has_pending_mutations());
            assert!(!app.annotate.adv.rich.as_ref().unwrap().applying);
            controls.click(&mut app, "dialog.password.cancel");
            assert!(app.annotate.adv.rich.is_none());
            assert!(matches!(
                requests.recv().unwrap(),
                Request::DiscardRichAnnotationPreview { epoch: e, generation: g } if e == epoch && g == generation
            ));
            assert!(app.annotate.measure.saved.contains(0, &original));
            assert_eq!(
                app.annotate.measure.saved.calibration(0),
                Some(&calibration(5.0))
            );
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn saved_scale_buttons_preserve_local_calibration_on_failure_and_reload_on_success() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            ready(&mut app);
            app.settings.language = language.into();
            app.annotate.measure.use_calibration(0, &calibration(25.0));
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(10.0)), vec![]));
            let mut controls = Controls::new();
            controls.click(&mut app, "measure.saved.save");
            assert!(app.navigation_blocked());
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::SetPageCalibration {page: 0, calibration: Some(ref c)}) if c == &calibration(25.0))
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            assert!(requests.try_recv().is_err());
            responses
                .send(Response::OperationFailed(EngineError::Other(
                    "scale write failed".into(),
                )))
                .unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&controls.ctx);
            assert_eq!(
                app.annotate.measure.local_calibration(0),
                Some(calibration(25.0))
            );
            assert!(!app.annotate.measure.saved.dialog_open());
            app.error = None;
            controls.click(&mut app, "measure.saved.save");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::SetPageCalibration { .. })
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            responses.send(changed(&app)).unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&controls.ctx);
            assert!(app.annotate.measure.local_calibration(0).is_none());
            assert!(app.annotate.measure.saved.calibration(0).is_none());
            app.request_measurement_page(0);
            let (_, epoch, generation) = requests
                .try_iter()
                .find_map(|r| match r {
                    Request::MeasurementPage {
                        page,
                        epoch,
                        generation,
                    } => Some((page, epoch, generation)),
                    _ => None,
                })
                .unwrap();
            app.on_measurement_page(
                0,
                epoch,
                generation,
                Ok(data(Some(calibration(25.0)), vec![])),
            );
            app.annotate.measure.use_calibration(0, &calibration(30.0));
            controls.click(&mut app, "measure.saved.remove");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::SetPageCalibration {
                    page: 0,
                    calibration: None
                })
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            responses.send(changed(&app)).unwrap();
            responses.send(Response::MutationFinished).unwrap();
            app.handle_responses(&controls.ctx);
            assert_eq!(
                app.annotate.measure.local_calibration(0),
                Some(calibration(30.0)),
                "removing the PDF default preserves an explicit session calibration"
            );
        }
    }

    #[test]
    fn inspector_controls_reuse_a_valid_scale_without_mutating_pdf_or_existing_records() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            ready(&mut app);
            app.settings.language = language.into();
            let record = record(MeasurementKind::Length);
            let mut stored = data(Some(calibration(5.0)), vec![record.clone()]);
            stored.can_modify = false;
            app.annotate.measure.saved.pages.insert(0, stored);
            let mut controls = Controls::new();
            controls.click(&mut app, "measure.saved.inspect");
            assert!(app.navigation_blocked());
            controls.click(&mut app, "measure.saved.use");
            assert!(!app.annotate.measure.saved.dialog_open());
            assert_eq!(
                app.annotate.measure.local_calibration(0),
                Some(calibration(10.0))
            );
            assert_eq!(
                app.annotate.measure.saved.calibration(0),
                Some(&calibration(5.0))
            );
            assert_eq!(
                app.annotate.measure.saved.pages[&0]
                    .records
                    .as_ref()
                    .unwrap()
                    .as_ref(),
                &[record]
            );
            controls.click(&mut app, "measure.saved.save");
            controls.click(&mut app, "measure.saved.remove");
            assert!(
                requests.try_recv().is_err(),
                "read-only controls and session reuse never edit the PDF"
            );
        }
    }

    #[test]
    fn inspector_rejects_angles_out_of_page_references_and_stale_records() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        let mut angle = record(MeasurementKind::Angle);
        angle.measurement.points.push((150.0, 200.0));
        let mut off_page = record(MeasurementKind::Length);
        off_page.measurement.calibration[0].0 = -5.0;
        for record in [angle, off_page] {
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(None, vec![record.clone()]));
            assert!(!app.use_saved_measurement_scale(0, &record));
        }
        let valid = record(MeasurementKind::Length);
        app.annotate
            .measure
            .saved
            .pages
            .insert(0, data(None, vec![valid.clone()]));
        app.view.epoch += 1;
        assert!(!app.use_saved_measurement_scale(0, &valid));
        assert!(app.annotate.measure.local_calibration(0).is_none());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn in_flight_edits_update_suspended_calibration_without_discarding_safe_session_scales() {
        for safe in [true, false] {
            let (mut app, _, responses) = crate::app::tests::test_app();
            ready(&mut app);
            let ctx = egui::Context::default();
            app.annotate.measure.use_calibration(0, &calibration(25.0));
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(10.0)), vec![]));
            let Response::DocumentChanged(mut state) = changed(&app) else {
                unreachable!()
            };
            state.measurement_scales_unchanged = safe;
            let info = state.info.clone();
            assert!(app.suspend_active());
            responses.send(Response::DocumentChanged(state)).unwrap();
            app.handle_responses(&ctx);
            let suspended_ui = app.tabs.tabs[0].ui.as_ref().unwrap();
            assert_eq!(
                suspended_ui.annotate.measure.local_calibration(0).is_some(),
                safe
            );
            assert!(suspended_ui.annotate.measure.saved.calibration(0).is_none());
            app.on_suspended(crate::worker::SuspendedDoc {
                bytes: vec![],
                password: None,
                path: info.path.clone(),
                undo: vec![],
                redo: vec![],
                save_point: None,
                modified: true,
                signature_source: None,
            });
            app.switch_to(0);
            app.on_resumed(
                DocState {
                    info,
                    can_undo: true,
                    can_redo: false,
                    pages_unchanged: false,
                    measurement_scales_unchanged: false,
                    modified: true,
                    has_signatures: false,
                },
                &ctx,
            );
            assert_eq!(app.annotate.measure.local_calibration(0).is_some(), safe);
            assert!(
                app.annotate.measure.saved.calibration(0).is_none(),
                "saved PDF scales must be reread, not removed from the document"
            );
        }
    }

    #[test]
    fn content_edits_and_their_history_discard_session_scales_even_when_pages_keep_their_size() {
        let (mut app, _, responses) = crate::app::tests::test_app();
        ready(&mut app);
        let ctx = egui::Context::default();
        for safe in [true, false, false] {
            app.annotate.measure.use_calibration(0, &calibration(25.0));
            app.annotate
                .measure
                .saved
                .pages
                .insert(0, data(Some(calibration(10.0)), vec![]));
            let Response::DocumentChanged(mut state) = changed(&app) else {
                unreachable!()
            };
            state.measurement_scales_unchanged = safe;
            responses.send(Response::DocumentChanged(state)).unwrap();
            app.handle_responses(&ctx);
            assert_eq!(app.annotate.measure.local_calibration(0).is_some(), safe);
            assert!(
                app.annotate.measure.saved.calibration(0).is_none(),
                "the stored default must always be reread"
            );
        }
    }

    #[test]
    fn inspector_view_selects_the_stored_annotation_and_leaves_measurement_mode() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        ready(&mut app);
        let page_size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document
            .as_mut()
            .unwrap()
            .info
            .page_sizes
            .push(page_size);
        app.document.as_mut().unwrap().page = 1;
        app.annotate.annotations_page = Some(1);
        app.annotate
            .measure
            .saved
            .pages
            .insert(1, data(None, vec![record(MeasurementKind::Length)]));
        let mut controls = Controls::new();
        controls.click(&mut app, "measure.saved.inspect");
        controls.click(&mut app, "register.view");
        assert!(!app.annotate.measure.saved.dialog_open());
        assert_eq!(app.annotate.tool, Tool::SelectAnnot);
        assert!(app.annotate.comments_open);
        assert_eq!(app.annotate.adv.reselect, Some((1, 3)));
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        app.refresh_annotations_if_needed();
        let queued: Vec<_> = requests.try_iter().collect();
        assert!(queued.iter().any(|r| matches!(
            r,
            Request::Annotations {
                page: 1,
                detailed: true
            }
        )));
        assert!(queued.iter().all(|r| !matches!(r, Request::Edit(_))));
    }
}
