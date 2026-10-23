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
    let render_id = egui::Id::new((spec.id, "render-pass"));
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
        ui.spacing_mut().item_spacing.y = 4.0;
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
        ui.separator();
        let footer_height = if ui.available_width() < 600.0 {
            52.0
        } else {
            30.0
        };
        let body_height = (ui.available_height() - footer_height).max(40.0);
        egui::ScrollArea::both()
            .id_salt((spec.id, "body"))
            .auto_shrink([false, false])
            .max_height(body_height)
            .show(ui, body);
        if popup_open {
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        ui.separator();
        ui.horizontal_wrapped(footer);
    };
    match (placement, host) {
        (Placement::Bottom, Some(ui)) => {
            let maximum = (ui.available_height() * 0.48).clamp(120.0, 600.0);
            let response = egui::Panel::bottom(spec.id)
                .resizable(true)
                .default_size(height)
                .size_range(120.0..=maximum)
                .frame(egui::Frame::new().fill(p.surface).inner_margin(8))
                .show(ui, content);
            if !ctx.input(|i| i.pointer.any_down()) {
                prefs.height = response.response.rect.height().round();
            }
        }
        (Placement::Floating, None) => {
            let maximum =
                (ctx.content_rect().size() - egui::vec2(32.0, 32.0)).max(egui::vec2(320.0, 180.0));
            egui::Window::new(&spec.title)
                .id(egui::Id::new((spec.id, "floating")))
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
}
