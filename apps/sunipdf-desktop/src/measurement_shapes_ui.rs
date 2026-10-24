//! Dimensioned rectangles keep their physical inputs through placement and preview.

use crate::{app::SuniApp, measure_tools::measurement_panel_button};
use egui::{Color32, Painter, Pos2, Rect, Stroke};
use sunipdf_core::measurement::{MeasurementUnit, PageCalibration};
use sunipdf_i18n::{tr, tr_args, Language};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RectangleScaleSelection {
    Local(PageCalibration),
    Page,
    Region(String),
    Automatic,
}

#[derive(Clone, Debug)]
pub(crate) struct RectanglePlacement {
    pub anchor: (f32, f32),
    pub width: f64,
    pub height: f64,
    pub unit: MeasurementUnit,
    pub precision: usize,
    pub scale: RectangleScaleSelection,
}

impl RectanglePlacement {
    pub(crate) fn valid(&self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && (self.width * self.height).is_finite()
            && self.anchor.0.is_finite()
            && self.anchor.1.is_finite()
            && self.precision <= 4
    }

    pub(crate) fn summary(&self, ui: &mut egui::Ui, lang: Language) {
        ui.strong(tr(lang, "measure.rectangle.title"));
        ui.label(tr_args(
            lang,
            "measure.rectangle.dimensions",
            &[
                ("width", &self.width.to_string()),
                ("height", &self.height.to_string()),
                ("unit", self.unit.id()),
            ],
        ));
        ui.label(tr_args(
            lang,
            "measure.rectangle.area",
            &[
                (
                    "value",
                    &format!("{:.*}", self.precision, self.width * self.height),
                ),
                ("unit", self.unit.id()),
            ],
        ));
        ui.label(tr_args(
            lang,
            "measure.rectangle.anchor",
            &[
                ("x", &format!("{:.2}", self.anchor.0)),
                ("y", &format!("{:.2}", self.anchor.1)),
            ],
        ));
        ui.horizontal(|ui| {
            ui.label(tr(lang, "measure.precision"));
            ui.label(self.precision.to_string());
        });
        let key = match self.scale {
            RectangleScaleSelection::Local(_) => "measure.rectangle.local",
            RectangleScaleSelection::Page => "measure.rectangle.page",
            RectangleScaleSelection::Region(_) => "measure.rectangle.region",
            RectangleScaleSelection::Automatic => "measure.rectangle.automatic",
        };
        ui.label(tr(lang, key));
        if let RectangleScaleSelection::Region(id) = &self.scale {
            ui.monospace(id);
        }
        ui.small(tr(lang, "measure.rectangle.preview_note"));
    }
}

#[derive(Clone, PartialEq, Default)]
enum ScaleChoice {
    Local,
    Page,
    Region(String),
    #[default]
    Automatic,
}

pub(crate) struct RectangleToolsState {
    width: String,
    height: String,
    unit: MeasurementUnit,
    precision: usize,
    scale: ScaleChoice,
    pub(crate) draft: Option<RectangleDraft>,
}

#[derive(Clone)]
pub(crate) struct RectangleDraft {
    pub(crate) page: usize,
    pub(crate) epoch: u64,
    pub(crate) placement: RectanglePlacement,
}

impl Default for RectangleToolsState {
    fn default() -> Self {
        Self {
            width: "2".into(),
            height: "3".into(),
            unit: MeasurementUnit::Metres,
            precision: 2,
            scale: ScaleChoice::Automatic,
            draft: None,
        }
    }
}

impl RectangleToolsState {
    fn dimensions(&self) -> Option<(f64, f64)> {
        let parse = |value: &str| {
            sunipdf_core::measurement::parse_measurement_distance(value, self.unit).ok()
        };
        let (width, height) = (parse(&self.width)?, parse(&self.height)?);
        (width > 0.0 && height > 0.0 && (width * height).is_finite()).then_some((width, height))
    }

    fn change_unit(&mut self, next: MeasurementUnit) {
        if next != self.unit {
            let factor = self.unit.metres() / next.metres();
            for value in [&mut self.width, &mut self.height] {
                if let Ok(distance) =
                    sunipdf_core::measurement::parse_measurement_distance(value, self.unit)
                {
                    *value = (distance * factor).to_string();
                }
            }
            self.unit = next;
        }
    }
}

impl SuniApp {
    pub(crate) fn measurement_rectangle_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let ready = self.measurement_ready();
        let Some(page) = self.document.as_ref().map(|doc| doc.page) else {
            return;
        };
        let local = self.annotate.measure.local_calibration(page);
        let regions = self
            .annotate
            .measure
            .saved
            .page_data(page)
            .and_then(|data| data.viewports.as_ref().ok())
            .map(|regions| {
                regions
                    .iter()
                    .map(|region| (region.id.clone(), region.name.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let can_start = ready && !self.annotate.measure.is_active_draft();
        let tools = &mut self.annotate.measure.rectangle;
        ui.separator();
        ui.label(egui::RichText::new(tr(lang, "measure.rectangle.title")).strong());
        ui.scope(|ui| {
            ui.columns(2, |columns| {
                let width_label = columns[0].label(tr(lang, "measure.rectangle.width"));
                columns[0]
                    .add_enabled(
                        can_start,
                        egui::TextEdit::singleline(&mut tools.width)
                            .char_limit(64)
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(width_label.id);
                let height_label = columns[1].label(tr(lang, "measure.rectangle.height"));
                columns[1]
                    .add_enabled(
                        can_start,
                        egui::TextEdit::singleline(&mut tools.height)
                            .char_limit(64)
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(height_label.id);
            });
            ui.columns(2, |columns| {
                let label = columns[0].label(tr(lang, "measure.unit"));
                let mut unit = tools.unit;
                columns[0].add_enabled_ui(can_start, |ui| {
                    egui::ComboBox::from_id_salt("dimensioned-rectangle-unit")
                        .width(ui.available_width())
                        .selected_text(unit.id())
                        .show_ui(ui, |ui| {
                            for candidate in MeasurementUnit::ALL {
                                ui.selectable_value(&mut unit, candidate, candidate.id());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                tools.change_unit(unit);
                let label = columns[1].label(tr(lang, "measure.precision"));
                columns[1]
                    .add_enabled(
                        can_start,
                        egui::DragValue::new(&mut tools.precision).range(0..=4),
                    )
                    .labelled_by(label.id);
            });
            let selected = match &tools.scale {
                ScaleChoice::Local => tr(lang, "measure.rectangle.local").to_string(),
                ScaleChoice::Page => tr(lang, "measure.rectangle.page").to_string(),
                ScaleChoice::Automatic => tr(lang, "measure.rectangle.automatic").to_string(),
                ScaleChoice::Region(id) => regions
                    .iter()
                    .find(|(candidate, _)| candidate == id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_else(|| tr(lang, "measure.rectangle.region").into()),
            };
            let label = ui.label(tr(lang, "measure.rectangle.scale"));
            ui.add_enabled_ui(can_start, |ui| {
                let width = ui.available_width();
                egui::ComboBox::from_id_salt("dimensioned-rectangle-scale")
                    .width(width)
                    .truncate()
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        ui.set_max_width(width);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        ui.selectable_value(
                            &mut tools.scale,
                            ScaleChoice::Automatic,
                            tr(lang, "measure.rectangle.automatic"),
                        );
                        ui.selectable_value(
                            &mut tools.scale,
                            ScaleChoice::Page,
                            tr(lang, "measure.rectangle.page"),
                        );
                        ui.add_enabled_ui(local.is_some(), |ui| {
                            ui.selectable_value(
                                &mut tools.scale,
                                ScaleChoice::Local,
                                tr(lang, "measure.rectangle.local"),
                            );
                        });
                        for (id, name) in &regions {
                            ui.selectable_value(
                                &mut tools.scale,
                                ScaleChoice::Region(id.clone()),
                                name,
                            );
                        }
                    })
                    .response
                    .labelled_by(label.id)
                    .on_hover_text(selected);
            });
        });
        let dimensions = tools.dimensions();
        let source_available = tools.scale != ScaleChoice::Local || local.is_some();
        if measurement_panel_button(
            ui,
            can_start && dimensions.is_some() && source_available,
            tr(lang, "measure.rectangle.place"),
        )
        .clicked()
        {
            let (width, height) = dimensions.unwrap();
            let scale = match &tools.scale {
                ScaleChoice::Local => RectangleScaleSelection::Local(local.unwrap()),
                ScaleChoice::Page => RectangleScaleSelection::Page,
                ScaleChoice::Region(id) => RectangleScaleSelection::Region(id.clone()),
                ScaleChoice::Automatic => RectangleScaleSelection::Automatic,
            };
            tools.draft = Some(RectangleDraft {
                page,
                epoch: self.view.epoch,
                placement: RectanglePlacement {
                    anchor: (0.0, 0.0),
                    width,
                    height,
                    unit: tools.unit,
                    precision: tools.precision,
                    scale,
                },
            });
        }
        if tools.scale == ScaleChoice::Local {
            ui.small(tr(lang, "measure.rectangle.local_note"));
        }
        if dimensions.is_none() {
            ui.colored_label(
                crate::theme::palette(self.settings.dark_theme).error,
                tr(lang, "measure.rectangle.invalid_dimensions"),
            );
        }
        if tools.draft.is_some() {
            ui.label(tr(lang, "measure.rectangle.place_hint"));
        }
    }

    pub(crate) fn place_dimensioned_rectangle(&mut self, page: usize, point: (f32, f32)) {
        let Some(draft) = self.annotate.measure.rectangle.draft.clone() else {
            return;
        };
        if draft.page != page || draft.epoch != self.view.epoch {
            return;
        }
        let mut placement = draft.placement;
        placement.anchor = point;
        self.annotate.measure.cancel();
        if !self.open_dimensioned_rectangle_preview(page, placement.clone()) {
            self.annotate.measure.rectangle.draft = Some(RectangleDraft {
                page,
                epoch: draft.epoch,
                placement,
            });
        }
    }

    pub(crate) fn paint_dimensioned_rectangle(
        &self,
        painter: &Painter,
        rect: Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        let state = &self.annotate.measure;
        let Some(draft) = state
            .rectangle
            .draft
            .as_ref()
            .filter(|draft| draft.page == page && draft.epoch == self.view.epoch)
        else {
            return;
        };
        let Some((_, anchor)) = state.hover.filter(|(hover_page, _)| *hover_page == page) else {
            return;
        };
        let regions = state
            .saved
            .page_data(page)
            .and_then(|data| data.viewports.as_ref().ok());
        let calibration = match &draft.placement.scale {
            RectangleScaleSelection::Local(calibration) => Some(calibration),
            RectangleScaleSelection::Page => state.saved.calibration(page),
            RectangleScaleSelection::Region(id) => {
                let Some(regions) = regions else {
                    return;
                };
                sunipdf_core::measurement::select_viewport(regions, &[anchor], Some(id))
                    .ok()
                    .flatten()
                    .map(|region| &region.calibration)
            }
            RectangleScaleSelection::Automatic => {
                let Some(regions) = regions else {
                    return;
                };
                match sunipdf_core::measurement::select_viewport(regions, &[anchor], None) {
                    Ok(Some(region)) => Some(&region.calibration),
                    Ok(None) => state.saved.calibration(page),
                    Err(_) => None,
                }
            }
        };
        let Some(calibration) = calibration else {
            return;
        };
        let Some(points) = indicative_outline(&draft.placement, anchor, calibration) else {
            return;
        };
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|doc| doc.info.page_sizes.get(page))
        else {
            return;
        };
        let within_page = points.iter().all(|point| {
            point.0 >= 0.0
                && point.1 >= 0.0
                && point.0 <= size.width_pt
                && point.1 <= size.height_pt
        });
        let within_region = match &draft.placement.scale {
            RectangleScaleSelection::Region(id) => regions.is_some_and(|regions| {
                sunipdf_core::measurement::select_viewport(regions, &points, Some(id)).is_ok()
            }),
            RectangleScaleSelection::Automatic => regions.is_some_and(|regions| {
                sunipdf_core::measurement::select_viewport(regions, &points, None).is_ok()
            }),
            _ => true,
        };
        let color = if within_page && within_region {
            Color32::from_rgb(20, 115, 225)
        } else {
            Color32::from_rgb(200, 45, 55)
        };
        let k = self.effective_zoom * crate::app::SCREEN_PER_PDF_POINT;
        let screen = |point: (f32, f32)| {
            Pos2::new(
                rect.left() + point.0 * k,
                rect.top() + (page_h_pt - point.1) * k,
            )
        };
        for index in 0..4 {
            painter.line_segment(
                [screen(points[index]), screen(points[(index + 1) % 4])],
                Stroke::new(2.0, color),
            );
        }
        painter.circle_filled(screen(anchor), 4.0, color);
    }
}

fn indicative_outline(
    placement: &RectanglePlacement,
    anchor: (f32, f32),
    calibration: &PageCalibration,
) -> Option<[(f32, f32); 4]> {
    let span = f64::from(calibration.points[1].0 - calibration.points[0].0)
        .hypot(f64::from(calibration.points[1].1 - calibration.points[0].1));
    let units_per_point =
        calibration.known_distance / span * calibration.unit.metres() / placement.unit.metres();
    let width = placement.width / units_per_point;
    let height = placement.height / units_per_point;
    if !placement.valid()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width > f64::from(f32::MAX)
        || height > f64::from(f32::MAX)
    {
        return None;
    }
    let right = anchor.0 + width as f32;
    let top = anchor.1 + height as f32;
    (right.is_finite() && top.is_finite() && right > anchor.0 && top > anchor.1).then_some([
        anchor,
        (right, anchor.1),
        (right, top),
        (anchor.0, top),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_local_and_page_outlines_do_not_depend_on_readable_regions() {
        let calibration = PageCalibration {
            points: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        };
        for (scale, expected_segments) in [
            (RectangleScaleSelection::Local(calibration.clone()), 4),
            (RectangleScaleSelection::Page, 4),
            (RectangleScaleSelection::Region("detail".into()), 0),
            (RectangleScaleSelection::Automatic, 0),
        ] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            app.open_measurements(&ctx);
            app.request_measurement_page(0);
            let (epoch, generation) = requests
                .try_iter()
                .find_map(|request| match request {
                    crate::worker::Request::MeasurementPage {
                        epoch, generation, ..
                    } => Some((epoch, generation)),
                    _ => None,
                })
                .unwrap();
            let mut data = crate::measurement_saved::MeasurementPageData::new(
                Ok(Some(calibration.clone())),
                Ok(Vec::new()),
                true,
            );
            data.viewports = Err("unsupported region metadata".into());
            app.on_measurement_page(0, epoch, generation, Ok(data));
            app.annotate.measure.rectangle.draft = Some(RectangleDraft {
                page: 0,
                epoch,
                placement: RectanglePlacement {
                    anchor: (0.0, 0.0),
                    width: 2.0,
                    height: 3.0,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                    scale,
                },
            });
            app.annotate.measure.hover = Some((0, (50.0, 100.0)));
            let mut output = ctx.run_ui(Default::default(), |ui| {
                app.paint_dimensioned_rectangle(
                    ui.painter(),
                    Rect::from_min_size(Pos2::ZERO, egui::vec2(600.0, 800.0)),
                    0,
                    800.0,
                );
            });
            assert_eq!(
                output
                    .shapes
                    .iter()
                    .filter(|shape| matches!(shape.shape, egui::Shape::LineSegment { .. }))
                    .count(),
                expected_segments
            );
            output.textures_delta.clear();
        }
    }

    #[test]
    fn changing_units_preserves_physical_dimensions_and_invalid_inputs_stay_invalid() {
        let mut state = RectangleToolsState::default();
        state.change_unit(MeasurementUnit::Centimetres);
        assert_eq!(state.dimensions(), Some((200.0, 300.0)));
        state.change_unit(MeasurementUnit::Feet);
        let (width, height) = state.dimensions().unwrap();
        assert!((width * 0.3048 - 2.0).abs() < 1e-10);
        assert!((height * 0.3048 - 3.0).abs() < 1e-10);
        for value in ["0", "-2", "NaN", "inf", ""] {
            state.width = value.into();
            assert!(state.dimensions().is_none(), "{value}");
        }
        state.width = "invalid".into();
        state.height = "3".into();
        state.unit = MeasurementUnit::Metres;
        state.change_unit(MeasurementUnit::Centimetres);
        assert_eq!(state.width, "invalid");
        assert_eq!(state.height, "300");
    }

    #[test]
    fn rectangle_placement_is_cancelled_when_the_page_or_document_context_changes() {
        for change_epoch in [false, true] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.open_measurements(&egui::Context::default());
            let size = app.document.as_ref().unwrap().info.page_sizes[0];
            app.document.as_mut().unwrap().info.page_sizes.push(size);
            app.annotate.measure.rectangle.draft = Some(RectangleDraft {
                page: 0,
                epoch: app.view.epoch,
                placement: RectanglePlacement {
                    anchor: (50.0, 100.0),
                    width: 2.0,
                    height: 3.0,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                    scale: RectangleScaleSelection::Page,
                },
            });
            assert!(app.annotate.measure.is_active_draft());
            if change_epoch {
                app.view.bump_epoch();
            } else {
                app.document.as_mut().unwrap().page = 1;
            }
            app.validate_measurement_context();
            assert!(!app.annotate.measure.is_active_draft());
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests.try_iter().all(|request| !matches!(
                request,
                crate::worker::Request::PreviewDimensionedRectangle { .. }
                    | crate::worker::Request::Edit(_)
            )));
        }
    }
}
