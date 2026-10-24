//! Contextual room/zone editing. Drawing changes a draft; only a reviewed worker candidate commits.
use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::{EditOp, Request},
};
use std::collections::BTreeMap;
use sunipdf_core::{plan_zones::*, Document, EngineError, RenderedPage};
use sunipdf_i18n::{tr, Language};

pub enum Job {
    Read {
        page: usize,
    },
    Preview {
        page: usize,
        zones: Vec<PlanZone>,
        reset_stale: bool,
    },
    Discard,
}
pub struct PageData {
    pub page: PagePlanZones,
    pub size: [f32; 2],
    pub image: RenderedPage,
    pub preview: Option<PlanZonePreview>,
    pub editable: bool,
}
#[derive(Default)]
pub struct WorkerCache {
    prepared: Option<(u64, u64, PreparedPlanZones)>,
}
impl WorkerCache {
    pub fn clear(&mut self) {
        self.prepared = None;
    }
    pub fn take(&mut self, epoch: u64, generation: u64) -> Result<PreparedPlanZones, EngineError> {
        match self.prepared.take() {
            Some((e, g, p)) if e == epoch && g == generation => Ok(p),
            _ => Err(EngineError::Other(
                "plan zone preview is stale; preview again".into(),
            )),
        }
    }
    pub fn run(
        &mut self,
        doc: &Document<'_>,
        epoch: u64,
        generation: u64,
        job: Job,
    ) -> Result<Option<PageData>, EngineError> {
        self.clear();
        let (page, preview, image) = match job {
            Job::Discard => return Ok(None),
            Job::Read { page } => {
                let size = doc.page_size(page)?;
                (
                    doc.plan_zones(page)?,
                    None,
                    doc.render_page(page, 1000. / size.width_pt.max(size.height_pt))?,
                )
            }
            Job::Preview {
                page,
                zones,
                reset_stale,
            } => {
                let prepared = doc.prepare_plan_zones(page, &zones, reset_stale)?;
                let image = doc.render_plan_zone_preview(&prepared, 1000)?;
                let report = prepared.report().clone();
                self.prepared = Some((epoch, generation, prepared));
                (report.after.clone(), Some(report), image)
            }
        };
        let size = doc.page_size(page.page - 1)?;
        let p = doc.properties();
        Ok(Some(PageData {
            page,
            size: [size.width_pt, size.height_pt],
            image,
            preview,
            editable: p.can_modify && p.can_copy && doc.signature_count() == 0,
        }))
    }
}
#[derive(Default)]
pub struct PlanZonesState {
    pub open: bool,
    generation: u64,
    pending: Option<(u64, u64)>,
    page: usize,
    size: [f32; 2],
    original: Option<PagePlanZones>,
    zones: Vec<PlanZone>,
    selected: Option<usize>,
    texture: Option<egui::TextureHandle>,
    preview: Option<PlanZonePreview>,
    token: Option<(u64, u64)>,
    error: Option<String>,
    editable: bool,
    reset_stale: bool,
    reviewed: bool,
    draw: bool,
    start: Option<egui::Pos2>,
}
impl PlanZonesState {
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.token = None;
        self.preview = None;
        self.reviewed = false;
        self.start = None;
    }
    fn add(&mut self, lang: Language) {
        if self.zones.len() >= MAX_ZONES {
            return;
        }
        let mut n = self.zones.len() + 1;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = loop {
            let id = format!("zone-{stamp:x}-{n}");
            if !self.zones.iter().any(|z| z.id == id) {
                break id;
            }
            n += 1;
        };
        self.zones.push(PlanZone {
            id,
            name: format!("{} {n}", tr(lang, "zones.zone")),
            parent_id: None,
            bounds: [
                self.size[0] * 0.2,
                self.size[1] * 0.2,
                self.size[0] * 0.8,
                self.size[1] * 0.8,
            ],
        });
        self.selected = Some(self.zones.len() - 1);
        self.draw = true;
        self.invalidate();
    }
}
fn path_label(path: &[ZonePathEntry]) -> String {
    path.iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}
pub(crate) fn zone_filter_ui(
    ui: &mut egui::Ui,
    lang: Language,
    filter: &mut sunipdf_core::annotation_register::RegisterFilter,
    pages: &[PagePlanZones],
) -> bool {
    let before = (filter.zone.clone(), filter.zone_status);
    let mut zones = BTreeMap::new();
    for page in pages {
        for zone in &page.zones {
            let mut names = vec![zone.name.as_str()];
            let mut parent = zone.parent_id.as_deref();
            for _ in 1..MAX_DEPTH {
                let Some(found) = parent.and_then(|id| page.zones.iter().find(|z| z.id == id))
                else {
                    break;
                };
                names.push(&found.name);
                parent = found.parent_id.as_deref();
            }
            names.reverse();
            zones.insert(
                (page.page, zone.id.clone()),
                format!("{} · {}", page.page, names.join(" / ")),
            );
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(lang, "zones.filter"));
        egui::ComboBox::from_id_salt("zone-filter")
            .selected_text(
                filter
                    .zone
                    .as_ref()
                    .map(|z| {
                        zones
                            .get(&(z.page, z.id.clone()))
                            .cloned()
                            .unwrap_or_else(|| format!("{} · {}", z.page, z.id))
                    })
                    .unwrap_or_else(|| tr(lang, "zones.all").into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filter.zone, None, tr(lang, "zones.all"));
                for ((page, id), name) in &zones {
                    let selected = filter
                        .zone
                        .as_ref()
                        .is_some_and(|z| z.page == *page && z.id == *id);
                    if ui.selectable_label(selected, name).clicked() {
                        filter.zone = Some(ZoneFilter {
                            page: *page,
                            id: id.clone(),
                            descendants: true,
                        });
                    }
                }
            });
        if let Some(zone) = &mut filter.zone {
            ui.checkbox(&mut zone.descendants, tr(lang, "zones.include_children"));
        }
        egui::ComboBox::from_id_salt("zone-status-filter")
            .selected_text(
                filter
                    .zone_status
                    .map(|s| tr(lang, &format!("zones.status.{}", s.id())))
                    .unwrap_or(tr(lang, "zones.all_statuses")),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut filter.zone_status,
                    None,
                    tr(lang, "zones.all_statuses"),
                );
                for status in ZoneAssignmentStatus::ALL {
                    ui.selectable_value(
                        &mut filter.zone_status,
                        Some(status),
                        tr(lang, &format!("zones.status.{}", status.id())),
                    );
                }
            });
    });
    before != (filter.zone.clone(), filter.zone_status)
}
pub(crate) fn coverage_ui(
    ui: &mut egui::Ui,
    lang: Language,
    coverage: &sunipdf_core::annotation_register::ZoneCoverage,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(lang, "zones.coverage"));
        for (status, count) in &coverage.counts {
            ui.label(format!(
                "{}: {count}",
                tr(lang, &format!("zones.status.{status}"))
            ));
        }
    });
    if !coverage.complete {
        ui.colored_label(crate::theme::ERROR, tr(lang, "zones.incomplete"));
    }
}
pub(crate) fn assignment_label(lang: Language, assignment: &ZoneAssignment) -> String {
    if assignment.status == ZoneAssignmentStatus::Assigned {
        path_label(&assignment.path)
    } else {
        tr(lang, &format!("zones.status.{}", assignment.status.id())).into()
    }
}
fn point_to_pdf(point: egui::Pos2, rect: egui::Rect, size: [f32; 2]) -> [f32; 2] {
    [
        ((point.x - rect.left()) / rect.width()).clamp(0., 1.) * size[0],
        ((rect.bottom() - point.y) / rect.height()).clamp(0., 1.) * size[1],
    ]
}
fn screen_rect(bounds: [f32; 4], rect: egui::Rect, size: [f32; 2]) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            rect.left() + bounds[0] / size[0] * rect.width(),
            rect.bottom() - bounds[3] / size[1] * rect.height(),
        ),
        egui::pos2(
            rect.left() + bounds[2] / size[0] * rect.width(),
            rect.bottom() - bounds[1] / size[1] * rect.height(),
        ),
    )
}
impl SuniApp {
    pub(crate) fn open_plan_zones(&mut self) {
        if !matches!(self.engine, EngineState::Ready)
            || self.worker.has_pending_mutations()
            || self.saving
            || self.ink_blocks_action()
            || self.annotate.quantities.blocks_document()
            || self.annotate.measure.is_active_draft()
        {
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let page = doc.page;
        self.annotate.register.open = false;
        self.annotate.quantities.open = false;
        let generation = self.plan_zones.generation.wrapping_add(1);
        self.plan_zones = PlanZonesState {
            open: true,
            page,
            generation,
            pending: Some((self.view.epoch, generation)),
            ..Default::default()
        };
        self.worker.send(Request::PlanZones {
            epoch: self.view.epoch,
            generation,
            job: Job::Read { page },
        });
    }
    pub(crate) fn close_plan_zones(&mut self) {
        self.plan_zones.invalidate();
        self.plan_zones.open = false;
        self.plan_zones.texture = None;
        self.worker.send(Request::PlanZones {
            epoch: self.view.epoch,
            generation: self.plan_zones.generation,
            job: Job::Discard,
        });
    }
    pub(crate) fn on_plan_zones(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<Option<PageData>, EngineError>,
    ) {
        let state = &mut self.plan_zones;
        if !state.open || epoch != self.view.epoch || state.pending != Some((epoch, generation)) {
            return;
        }
        state.pending = None;
        match result {
            Ok(Some(data)) => {
                state.size = data.size;
                state.editable = data.editable;
                if data.preview.is_none() {
                    state.original = Some(data.page.clone());
                    state.zones = data.page.zones.clone();
                    state.selected = (!state.zones.is_empty()).then_some(0);
                } else {
                    state.zones = data.page.zones.clone();
                    state.token = Some((epoch, generation));
                }
                state.preview = data.preview;
                state.error = None;
                state.reviewed = false;
                state.texture = Some(ctx.load_texture(
                    "plan-zone-page",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [data.image.width as usize, data.image.height as usize],
                        &data.image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ));
            }
            Ok(None) => {}
            Err(e) => {
                state.token = None;
                state.preview = None;
                state.error = Some(e.to_string());
            }
        }
    }
    pub(crate) fn plan_zones_dialog(&mut self, ctx: &egui::Context) {
        if !self.plan_zones.open {
            return;
        }
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let width = (ctx.content_rect().width() - 40.).clamp(620., 1100.);
        let state = &mut self.plan_zones;
        let busy = state.pending.is_some();
        let can_preview = !busy
            && state.editable
            && state
                .original
                .as_ref()
                .is_some_and(|p| p.validity == ZoneValidity::Current || state.reset_stale);
        let can_apply = !busy && state.token.is_some() && state.reviewed;
        let (mut close, mut preview, mut apply, mut changed) = (false, false, false, false);
        dialog::show(
            ctx,
            Spec::new("plan-zones", tr(lang, "zones.title"), dark, has_icons)
                .subtitle(tr(lang, "zones.help"))
                .width(width),
            |ui| {
                if let Some(error) = &state.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if busy {
                    ui.spinner();
                    ui.label(tr(lang, "zones.loading"));
                }
                if state
                    .original
                    .as_ref()
                    .is_some_and(|p| p.validity == ZoneValidity::Stale)
                {
                    ui.colored_label(crate::theme::ERROR, tr(lang, "zones.stale"));
                    changed |= ui
                        .checkbox(&mut state.reset_stale, tr(lang, "zones.reset_stale"))
                        .changed();
                }
                if !state.editable && state.original.is_some() {
                    ui.label(tr(lang, "zones.read_only"));
                }
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{} {} · {} / {MAX_ZONES}",
                            tr(lang, "register.page"),
                            state.page + 1,
                            state.zones.len()
                        ));
                        if ui
                            .add_enabled(
                                state.editable && state.zones.len() < MAX_ZONES,
                                egui::Button::new(tr(lang, "zones.add")),
                            )
                            .clicked()
                        {
                            state.add(lang);
                            changed = true;
                        }
                        if ui
                            .add_enabled(
                                state.editable && state.selected.is_some(),
                                egui::Button::new(tr(lang, "zones.remove")),
                            )
                            .clicked()
                        {
                            if let Some(i) = state.selected.take() {
                                state.zones.remove(i);
                                changed = true;
                            }
                        }
                    });
                    egui::ComboBox::from_id_salt("zone-selection")
                        .selected_text(
                            state
                                .selected
                                .and_then(|i| state.zones.get(i))
                                .map(|z| z.name.as_str())
                                .unwrap_or(tr(lang, "zones.select")),
                        )
                        .show_ui(ui, |ui| {
                            for (i, z) in state.zones.iter().enumerate() {
                                ui.selectable_value(
                                    &mut state.selected,
                                    Some(i),
                                    format!("{} · {}", z.name, z.id),
                                );
                            }
                        });
                    if let Some(index) = state.selected.filter(|i| *i < state.zones.len()) {
                        let parents: Vec<_> = state
                            .zones
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != index)
                            .map(|(_, z)| (z.id.clone(), z.name.clone()))
                            .collect();
                        ui.add_enabled_ui(state.editable, |ui| {
                            let zone = &mut state.zones[index];
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "zones.name"));
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut zone.name)
                                            .char_limit(256)
                                            .desired_width(240.),
                                    )
                                    .changed();
                                ui.label(format!("ID: {}", zone.id));
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "zones.parent"));
                                egui::ComboBox::from_id_salt("zone-parent")
                                    .selected_text(
                                        zone.parent_id.as_deref().unwrap_or(tr(lang, "zones.none")),
                                    )
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(
                                                &mut zone.parent_id,
                                                None,
                                                tr(lang, "zones.none"),
                                            )
                                            .changed();
                                        for (id, name) in &parents {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut zone.parent_id,
                                                    Some(id.clone()),
                                                    name,
                                                )
                                                .changed();
                                        }
                                    });
                                ui.checkbox(&mut state.draw, tr(lang, "zones.draw"));
                            });
                            ui.horizontal_wrapped(|ui| {
                                for (i, key) in
                                    ["zones.left", "zones.bottom", "zones.right", "zones.top"]
                                        .iter()
                                        .enumerate()
                                {
                                    ui.label(tr(lang, key));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut zone.bounds[i])
                                                .speed(0.5)
                                                .max_decimals(3),
                                        )
                                        .changed();
                                }
                            });
                        });
                    }
                    if let Some(texture) = &state.texture {
                        let image_size = egui::vec2(state.size[0], state.size[1]);
                        let scale = (ui.available_width() / image_size.x).min(400. / image_size.y);
                        let response = ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(image_size * scale)
                                .sense(egui::Sense::drag()),
                        );
                        let painter = ui.painter_at(response.rect);
                        for (i, z) in state.zones.iter().enumerate() {
                            let rect = screen_rect(z.bounds, response.rect, state.size);
                            let color = if state.selected == Some(i) {
                                egui::Color32::from_rgb(20, 120, 235)
                            } else {
                                egui::Color32::from_rgb(180, 85, 10)
                            };
                            painter.rect_stroke(
                                rect,
                                0.,
                                egui::Stroke::new(2., color),
                                egui::StrokeKind::Inside,
                            );
                            painter.text(
                                rect.left_top() + egui::vec2(3., 3.),
                                egui::Align2::LEFT_TOP,
                                &z.name,
                                egui::FontId::proportional(12.),
                                color,
                            );
                        }
                        if state.draw && state.editable && state.selected.is_some() {
                            if response.drag_started() {
                                state.start = response.interact_pointer_pos();
                            }
                            if let (Some(start), Some(current)) =
                                (state.start, response.interact_pointer_pos())
                            {
                                let rect = egui::Rect::from_two_pos(start, current);
                                painter.rect_stroke(
                                    rect,
                                    0.,
                                    egui::Stroke::new(2., egui::Color32::LIGHT_BLUE),
                                    egui::StrokeKind::Inside,
                                );
                                if response.drag_stopped() {
                                    let a = point_to_pdf(start, response.rect, state.size);
                                    let b = point_to_pdf(current, response.rect, state.size);
                                    if (a[0] - b[0]).abs() >= 1. && (a[1] - b[1]).abs() >= 1. {
                                        if let Some(z) =
                                            state.selected.and_then(|i| state.zones.get_mut(i))
                                        {
                                            z.bounds = [
                                                a[0].min(b[0]),
                                                a[1].min(b[1]),
                                                a[0].max(b[0]),
                                                a[1].max(b[1]),
                                            ];
                                            changed = true;
                                        }
                                    }
                                    state.start = None;
                                    state.draw = false;
                                }
                            }
                        }
                        ui.small(tr(lang, "zones.overlay_only"));
                    }
                    if let Some(report) = &state.preview {
                        ui.label(format!(
                            "{}: {}",
                            tr(lang, "zones.affected"),
                            report.affected_rows()
                        ));
                        egui::ScrollArea::vertical()
                            .id_salt("zone-assignments")
                            .max_height(140.)
                            .show(ui, |ui| {
                                for row in &report.assignments {
                                    ui.label(format!(
                                        "{} · #{}: {} → {}",
                                        row.page,
                                        row.index,
                                        assignment_label(lang, &row.before),
                                        assignment_label(lang, &row.after)
                                    ));
                                }
                            });
                        ui.checkbox(&mut state.reviewed, tr(lang, "zones.reviewed"));
                    }
                });
            },
            |ui| {
                apply = ui
                    .add_enabled(can_apply, egui::Button::new(tr(lang, "zones.apply")))
                    .clicked();
                preview = ui
                    .add_enabled(can_preview, egui::Button::new(tr(lang, "zones.preview")))
                    .clicked();
                close = ui.button(tr(lang, "register.cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if changed {
            state.invalidate();
            self.worker.send(Request::PlanZones {
                epoch: self.view.epoch,
                generation: state.generation,
                job: Job::Discard,
            });
        }
        if close {
            self.close_plan_zones();
            return;
        }
        if preview && can_preview {
            state.invalidate();
            let generation = state.generation;
            state.pending = Some((self.view.epoch, generation));
            self.worker.send(Request::PlanZones {
                epoch: self.view.epoch,
                generation,
                job: Job::Preview {
                    page: state.page,
                    zones: state.zones.clone(),
                    reset_stale: state.reset_stale,
                },
            });
        }
        if apply {
            if let Some((epoch, generation)) = state.token.take() {
                state.open = false;
                state.texture = None;
                self.worker
                    .send(Request::Edit(EditOp::CommitPlanZones { epoch, generation }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawing_uses_displayed_bottom_left_coordinates_and_changes_invalidate_review() {
        let rect = egui::Rect::from_min_max(egui::pos2(20., 30.), egui::pos2(220., 430.));
        assert_eq!(
            point_to_pdf(egui::pos2(70., 330.), rect, [100., 200.]),
            [25., 50.]
        );
        assert_eq!(
            screen_rect([25., 50., 75., 150.], rect, [100., 200.]),
            egui::Rect::from_min_max(egui::pos2(70., 130.), egui::pos2(170., 330.))
        );
        let mut s = PlanZonesState {
            token: Some((4, 8)),
            reviewed: true,
            size: [100., 200.],
            ..Default::default()
        };
        s.add(Language::English);
        assert!(s.token.is_none() && !s.reviewed);
        assert_eq!(s.zones.len(), 1);
        let id = s.zones[0].id.clone();
        s.zones[0].name = "Renamed".into();
        s.invalidate();
        assert_eq!(s.zones[0].id, id);
    }
    #[test]
    fn late_results_after_close_or_tab_epoch_change_do_not_restore_drafts() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.register.open = true;
        app.open_plan_zones();
        assert!(!app.annotate.register.open);
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PlanZones {
                epoch,
                generation,
                job: Job::Read { .. },
            } => (epoch, generation),
            _ => panic!("read request"),
        };
        app.close_plan_zones();
        app.on_plan_zones(
            &ctx,
            epoch,
            generation,
            Err(EngineError::Other("late result".into())),
        );
        assert!(!app.plan_zones.open && app.plan_zones.error.is_none());
        assert!(!requests.try_iter().any(|r| matches!(r, Request::Edit(_))));
    }
    #[test]
    fn bilingual_zone_actions_remain_accessible_at_minimum_window_size() {
        for language in Language::ALL {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.settings.set_language(language);
            app.plan_zones = PlanZonesState {
                open: true,
                editable: true,
                size: [200., 200.],
                original: Some(PagePlanZones {
                    page: 1,
                    validity: ZoneValidity::Current,
                    zones: Vec::new(),
                }),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let page = PagePlanZones {
                page: 1,
                validity: ZoneValidity::Current,
                zones: vec![PlanZone {
                    id: "room".into(),
                    name: "Review room".into(),
                    parent_id: None,
                    bounds: [20., 20., 180., 180.],
                }],
            };
            app.plan_zones.zones = page.zones.clone();
            app.plan_zones.selected = Some(0);
            app.plan_zones.token = Some((app.view.epoch, 1));
            app.plan_zones.preview = Some(PlanZonePreview {
                before: app.plan_zones.original.clone().unwrap(),
                after: page,
                assignments: (0..20)
                    .map(|index| ZoneRowAssignment {
                        page: 1,
                        index,
                        annotation_id: Some(format!("annotation-{index}")),
                        before: ZoneAssignment::default(),
                        after: ZoneAssignment {
                            status: ZoneAssignmentStatus::Assigned,
                            zone_id: Some("room".into()),
                            path: vec![ZonePathEntry {
                                id: "room".into(),
                                name: "Review room".into(),
                            }],
                            candidates: Vec::new(),
                        },
                    })
                    .collect(),
            });
            app.plan_zones.texture = Some(ctx.load_texture(
                "zone-qa-page",
                egui::ColorImage::from_rgba_unmultiplied([200, 200], &vec![255; 200 * 200 * 4]),
                egui::TextureOptions::LINEAR,
            ));
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
                    |_| app.plan_zones_dialog(&ctx),
                );
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                for key in ["zones.apply", "zones.preview", "register.cancel"] {
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
                    if key == "zones.apply" {
                        assert!(button.is_disabled());
                    }
                }
            }
        }
    }
}
