//! Panneau Objets : objets de la page courante, du plus en avant (en haut) au plus en arrière.
//! Clic = sélectionner (Ctrl : ajouter ou retirer, Maj : ajouter), survol = repère sur la page,
//! boutons avancer / reculer d'un plan et supprimer.

use crate::app::SuniApp;
use crate::editor::kind_key;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{EditOp, Pick, PickMode, Request};
use egui::{Align, Button, Frame, Layout, RichText, Stroke};
use sunipdf_core::{Command, ObjectKind, ZOrder};
use sunipdf_i18n::{tr, tr_args};

/// Texte d'une ligne : coupé à `max` caractères (jamais au milieu d'un caractère).
pub fn truncate(s: &str, max: usize) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        format!("{}…", flat.chars().take(max).collect::<String>())
    }
}

fn kind_icon(kind: ObjectKind) -> (&'static str, &'static str) {
    match kind {
        ObjectKind::Text => (glyph::FONT, "T"),
        ObjectKind::Image => (glyph::PICTURE, "🖼"),
        ObjectKind::Path => (glyph::RECT, "▭"),
        ObjectKind::Form => (glyph::LAYERS, "⧉"),
        ObjectKind::Other => (glyph::DOCUMENT, "◇"),
    }
}

enum RowAction {
    Select(usize, PickMode),
    Reorder(usize, ZOrder),
    Delete(usize),
}

impl SuniApp {
    pub fn objects_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        let Some(doc) = &self.document else { return };
        let page = doc.page;
        let count = self.edit.objects.len();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, "objects.title"), 15.0));
            ui.label(
                RichText::new(count.to_string())
                    .color(p.text_muted)
                    .size(13.0),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(Button::new(icons::icon(has_icons, glyph::CLOSE, "✕", 14.0)).frame(false))
                    .on_hover_text(tr(lang, "tools.close"))
                    .clicked()
                {
                    self.execute(ctx, Command::ToggleObjects);
                }
            });
        });
        ui.separator();
        ui.label(
            RichText::new(tr_args(
                lang,
                "objects.page",
                &[("page", &(page + 1).to_string())],
            ))
            .size(12.0)
            .color(p.text_muted),
        );
        ui.label(
            RichText::new(tr(lang, "objects.hint"))
                .size(11.5)
                .color(p.text_muted),
        );
        ui.add_space(4.0);
        if self.edit.objects.is_empty() {
            ui.label(RichText::new(tr(lang, "objects.empty")).color(p.text_muted));
        }
        let selected: Vec<usize> = self
            .edit
            .selection
            .as_ref()
            .filter(|s| s.page == page)
            .map(|s| s.indices())
            .unwrap_or_default();
        let mut hover = None;
        let mut action = None;
        let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for o in self.edit.objects.iter().rev() {
                    let is_sel = selected.contains(&o.index);
                    let label = match &o.text {
                        Some(t) if !t.trim().is_empty() => truncate(t, 40),
                        _ => tr(lang, kind_key(o.kind)).to_string(),
                    };
                    let row = Frame::new()
                        .fill(if is_sel { p.accent_soft } else { p.surface })
                        .stroke(Stroke::new(
                            1.0,
                            if is_sel { theme::BLUE } else { p.border },
                        ))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (g, f) = kind_icon(o.kind);
                                ui.label(icons::icon(has_icons, g, f, 14.0).color(p.text_muted));
                                ui.add(
                                    egui::Label::new(RichText::new(&label).size(13.0)).truncate(),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let small = |ui: &mut egui::Ui, g: &str, f: &str, tip: &str| {
                                        ui.add(
                                            Button::new(icons::icon(has_icons, g, f, 12.0))
                                                .frame(false),
                                        )
                                        .on_hover_text(tip)
                                        .clicked()
                                    };
                                    if small(ui, glyph::DELETE, "🗑", tr(lang, "objects.delete"))
                                    {
                                        action = Some(RowAction::Delete(o.index));
                                    }
                                    if o.index > 0
                                        && small(ui, glyph::BACK, "▼", tr(lang, "objects.backward"))
                                    {
                                        action =
                                            Some(RowAction::Reorder(o.index, ZOrder::Backward));
                                    }
                                    if o.index + 1 < count
                                        && small(ui, glyph::FRONT, "▲", tr(lang, "objects.forward"))
                                    {
                                        action = Some(RowAction::Reorder(o.index, ZOrder::Forward));
                                    }
                                });
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    // Libellé lisible pour les lecteurs d'écran (sélection comprise).
                    row.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            is_sel,
                            &label,
                        )
                    });
                    if row.hovered() {
                        hover = Some(o.index);
                    }
                    if row.clicked() && action.is_none() {
                        let mode = if ctrl {
                            PickMode::Toggle
                        } else if shift {
                            PickMode::Add
                        } else {
                            PickMode::Replace
                        };
                        action = Some(RowAction::Select(o.index, mode));
                    }
                    ui.add_space(4.0);
                }
            });
        self.edit.hover_index = hover;
        match action {
            Some(RowAction::Select(i, mode)) => {
                if self.annotate.tool != crate::annotate::Tool::SelectObject {
                    self.execute(ctx, Command::ToolSelectObject);
                }
                self.worker.send(Request::Pick {
                    page,
                    pick: Pick::Indices(vec![i]),
                    mode,
                });
            }
            Some(RowAction::Reorder(i, op)) => {
                let to = if op == ZOrder::Forward { i + 1 } else { i - 1 };
                self.edit.reselect_many = Some((page, Pick::Indices(vec![to])));
                self.worker.send(Request::Edit(EditOp::ReorderObjects {
                    page,
                    indices: vec![i],
                    op,
                }));
            }
            Some(RowAction::Delete(i)) => {
                if selected.contains(&i) {
                    self.edit.selection = None;
                }
                self.worker.send(Request::Edit(EditOp::DeleteObjects {
                    page,
                    indices: vec![i],
                }));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_characters() {
        assert_eq!(truncate("court", 40), "court");
        assert_eq!(truncate("a  b\nc", 40), "a b c");
        assert_eq!(truncate("ééééé", 3), "ééé…");
        assert_eq!(truncate("日本語テキスト", 2), "日本…");
    }
}
