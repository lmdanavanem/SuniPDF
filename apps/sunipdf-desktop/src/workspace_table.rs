//! Resizable document tables share a persistent, non-modal workspace host.

use crate::{dialog::Spec, theme};
use serde::{Deserialize, Serialize};
use sunipdf_i18n::{tr, Language};

impl crate::app::SuniApp {
    pub(crate) fn follow_table_annotation(
        &mut self,
        row: &sunipdf_core::annotation_register::RegisterRow,
    ) {
        let Some(page) = row.page.checked_sub(1) else {
            return;
        };
        let Some(rect) = row.rect else { return };
        if !rect.iter().all(|v| v.is_finite()) || rect[0] > rect[2] || rect[1] > rect[3] {
            return;
        }
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(page))
        else {
            return;
        };
        let height = size.height_pt;
        self.annotate.tool = crate::annotate::Tool::SelectAnnot;
        crate::workspace::follow(
            &mut self.annotate,
            &mut self.organize,
            &mut self.edit,
            sunipdf_core::Command::ToolSelectAnnot,
        );
        self.annotate.comments_open = true;
        self.annotate.adv.reselect = Some((page, row.index));
        self.go_to_page_point(page, rect[0], (height - rect[3]).max(0.0));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    Bottom,
    Floating,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub placement: Placement,
    pub height: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            placement: Placement::Bottom,
            height: 280.0,
        }
    }
}

impl Preferences {
    fn height_for(&self, available: f32) -> f32 {
        let maximum = (available * 0.48).clamp(120.0, 600.0);
        let height = if self.height.is_finite() {
            self.height
        } else {
            280.0
        };
        height.clamp(120.0, maximum)
    }
}

pub(crate) fn show(
    ctx: &egui::Context,
    host: Option<&mut egui::Ui>,
    prefs: &mut Preferences,
    lang: Language,
    spec: Spec<'_>,
    body: impl FnOnce(&mut egui::Ui),
    footer: impl FnOnce(&mut egui::Ui),
) -> bool {
    if (prefs.placement == Placement::Bottom) != host.is_some() {
        return false;
    }
    let render_id = egui::Id::unique((spec.id, "render-pass"));
    let stamp = (ctx.cumulative_frame_nr(), ctx.cumulative_pass_nr());
    let already_rendered = ctx.data_mut(|data| {
        if data.get_temp::<(u64, u64)>(render_id) == Some(stamp) {
            true
        } else {
            data.insert_temp(render_id, stamp);
            false
        }
    });
    if already_rendered {
        return false;
    }
    let previous = prefs.clone();
    let p = theme::palette(spec.dark);
    let placement = prefs.placement;
    let height = prefs.height_for(
        host.as_ref()
            .map_or(ctx.content_rect().height(), |ui| ui.available_height()),
    );
    let popup_open = egui::Popup::is_any_open(ctx);
    let content = |ui: &mut egui::Ui| {
        if placement == Placement::Bottom {
            // PanelState remembers the frame's used height. Keep the allocated
            // height even when the bounded body/footer leave a few pixels free,
            // otherwise each idle frame shrinks the dock and changes Fit Page.
            ui.set_min_height(ui.available_height());
        }
        let compact = ui.available_height() < 210.0;
        ui.spacing_mut().item_spacing.y = if compact { 2.0 } else { 4.0 };
        if compact {
            ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
            ui.spacing_mut().interact_size.y = 22.0;
        }
        ui.horizontal_wrapped(|ui| {
            ui.strong(&spec.title);
            ui.add_space(12.0);
            ui.selectable_value(
                &mut prefs.placement,
                Placement::Bottom,
                tr(lang, "workspace.table.dock"),
            );
            ui.selectable_value(
                &mut prefs.placement,
                Placement::Floating,
                tr(lang, "workspace.table.float"),
            );
        });
        let separator_height = if compact { 2.0 } else { 6.0 };
        ui.add(egui::Separator::default().spacing(separator_height));
        let footer_rows = if ui.available_width() < 500.0 {
            2.0
        } else {
            1.0
        };
        // Include the spacing after the scroll area and separator, and between
        // wrapped footer rows. Leave the remaining height to real table rows.
        let footer_height = ui.spacing().interact_size.y * footer_rows
            + separator_height
            + ui.spacing().item_spacing.y * (footer_rows + 2.0);
        let body_height = (ui.available_height() - footer_height).max(0.0);
        let body_width = ui.available_width();
        egui::ScrollArea::both()
            .id_salt((spec.id, "body"))
            .auto_shrink([false, false])
            .min_scrolled_width(0.0)
            .min_scrolled_height(0.0)
            .max_width(body_width)
            .max_height(body_height)
            .show(ui, body);
        if popup_open {
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        ui.add(egui::Separator::default().spacing(separator_height));
        ui.horizontal_wrapped(footer);
    };
    match (placement, host) {
        (Placement::Bottom, Some(ui)) => {
            let maximum = (ui.available_height() * 0.48).clamp(120.0, 600.0);
            let response = egui::Panel::bottom(spec.id)
                .resizable(true)
                .default_size(height)
                .size_range(120.0..=maximum)
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .inner_margin(egui::Margin::symmetric(6, 4)),
                )
                .show(ui, content);
            if !ctx.input(|i| i.pointer.any_down()) {
                prefs.height = response.response.rect.height().round();
            }
        }
        (Placement::Floating, None) => {
            let maximum =
                (ctx.content_rect().size() - egui::vec2(32.0, 32.0)).max(egui::vec2(320.0, 180.0));
            egui::Window::new(&spec.title)
                .id(egui::Id::unique((spec.id, "floating")))
                .title_bar(false)
                .resizable(true)
                .default_size(egui::vec2(spec.width.min(maximum.x), height.max(220.0)))
                .min_size(egui::vec2(320.0, 180.0))
                .max_size(maximum)
                .show(ctx, content);
        }
        _ => {}
    }
    *prefs != previous
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_docks_preserve_height_and_fit_page_zoom_across_frames() {
        for dark in [false, true] {
            for (size, command) in [
                (
                    egui::vec2(1280.0, 780.0),
                    sunipdf_core::Command::AnnotationRegister,
                ),
                (
                    egui::vec2(720.0, 480.0),
                    sunipdf_core::Command::QuantityTable,
                ),
            ] {
                let ctx = egui::Context::default();
                let (mut app, _requests, _responses) = crate::app::tests::test_app();
                app.has_icons = crate::icons::install(&ctx);
                theme::apply(&ctx, dark);
                app.settings.dark_theme = dark;
                app.settings.show_thumbnails = false;
                app.settings.autosave = false;
                app.show_tools = false;
                app.settings.show_tools = false;
                app.zoom = crate::app::Zoom::FitPage;
                app.execute(&ctx, command);
                let mut frames = Vec::new();
                for index in 0..32 {
                    let mut frame = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            time: Some(index as f64 * 0.25),
                            ..Default::default()
                        },
                        |ui| app.render_ui(ui),
                    );
                    frame.textures_delta.clear();
                    frames.push((app.settings.tables.height, app.effective_zoom));
                }
                assert!(app.error.is_none(), "{:?}", app.error);
                let settled = frames[4];
                assert!(settled.0 >= 120.0 && settled.1 > 0.05);
                assert!(
                    frames[4..].iter().all(|&(height, zoom)| {
                        (height - settled.0).abs() < 0.01 && (zoom - settled.1).abs() < 0.000_001
                    }),
                    "idle dock and Fit Page must settle: size={size:?}, dark={dark}, frames={frames:?}"
                );
            }
        }
    }

    #[test]
    fn persisted_table_layout_is_bounded_on_small_windows() {
        for requested in [f32::NAN, f32::INFINITY, -100.0, 100_000.0, 280.0] {
            let prefs = Preferences {
                height: requested,
                ..Default::default()
            };
            for height in [480.0, 768.0, 1080.0] {
                let actual = prefs.height_for(height);
                assert!(actual.is_finite() && actual >= 120.0 && actual <= height * 0.48);
            }
        }
        let prefs = Preferences {
            placement: Placement::Floating,
            height: 312.0,
        };
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(restored, prefs);
        assert_eq!(
            serde_json::from_str::<Preferences>("{}").unwrap(),
            Preferences::default()
        );
    }
    #[test]
    fn placement_switch_cannot_render_the_same_table_twice_in_a_pass() {
        let ctx = egui::Context::default();
        let mut prefs = Preferences::default();
        let dock_calls = std::cell::Cell::new(0);
        let duplicate_calls = std::cell::Cell::new(0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(720.0, 480.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input.clone(), |ui| {
            prefs.placement = Placement::Bottom;
            show(
                &ctx,
                Some(ui),
                &mut prefs,
                Language::English,
                Spec::new("one-table", "Rows", false, false),
                |ui| {
                    dock_calls.set(dock_calls.get() + 1);
                    ui.label("First row");
                },
                |ui| {
                    ui.label("Close");
                },
            );
            prefs.placement = Placement::Floating;
            show(
                &ctx,
                None,
                &mut prefs,
                Language::English,
                Spec::new("one-table", "Rows", false, false),
                |_| {
                    duplicate_calls.set(duplicate_calls.get() + 1);
                },
                |_| {},
            );
        });
        output.textures_delta.clear();
        assert!(dock_calls.get() > 0);
        assert_eq!(duplicate_calls.get(), 0);
        let mut output = ctx.run_ui(input, |_| {
            show(
                &ctx,
                None,
                &mut prefs,
                Language::English,
                Spec::new("one-table", "Rows", false, false),
                |ui| {
                    duplicate_calls.set(duplicate_calls.get() + 1);
                    ui.label("First row");
                },
                |_| {},
            );
        });
        output.textures_delta.clear();
        assert!(
            duplicate_calls.get() > 0,
            "the floating host takes over on the next frame"
        );
    }

    #[test]
    fn compact_dock_preserves_canvas_height_and_visible_first_row() {
        let ctx = egui::Context::default();
        let mut prefs = Preferences::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(720.0, 480.0),
            )),
            ..Default::default()
        };
        let mut remaining = 0.0;
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| {
                ui.allocate_space(egui::vec2(720.0, 95.0));
                show(
                    &ctx,
                    Some(ui),
                    &mut prefs,
                    Language::French,
                    Spec::new("small-table", "Quantities", false, false),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Filters");
                            ui.label("Totals");
                            ui.label("1-2 / 2");
                        });
                        egui::Grid::new("compact-grid").show(ui, |ui| {
                            ui.label("Page");
                            ui.label("Contents");
                            ui.end_row();
                            ui.label("1");
                            ui.label("Fixture equipment");
                            ui.end_row();
                        });
                    },
                    |ui| {
                        ui.label("Refresh");
                        ui.label("Export");
                        ui.label("Close");
                    },
                );
                remaining = ui.available_height();
            });
            output.textures_delta.clear();
            let visible = output.shapes.iter().any(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Fixture equipment" => shape
                    .clip_rect
                    .contains(text.pos + text.galley.size() / 2.0),
                _ => false,
            });
            assert!(
                remaining >= 120.0,
                "document viewport retained: {remaining}"
            );
            assert!(visible, "first row visible without scrolling");
        }
    }
}
