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
    Drop(ObjectsDrag, usize, bool),
}

#[derive(Clone)]
struct ObjectsDrag {
    page: usize,
    epoch: u64,
    count: usize,
    indices: Vec<usize>,
}

impl ObjectsDrag {
    fn destination(
        &self,
        page: usize,
        epoch: u64,
        count: usize,
        target: usize,
        above: bool,
    ) -> Option<usize> {
        if self.page != page
            || self.epoch != epoch
            || self.count != count
            || target >= count
            || self.indices.is_empty()
            || self.indices.iter().any(|&i| i >= count)
            || self.indices.windows(2).any(|w| w[0] >= w[1])
            || self.indices.contains(&target)
        {
            return None;
        }
        // The panel shows front-to-back; the native drawing order runs in the opposite direction.
        let slot = target + usize::from(above);
        let at = slot - self.indices.iter().filter(|&&i| i < slot).count();
        (!self.indices.iter().enumerate().all(|(k, &i)| i == at + k)).then_some(at)
    }
}

impl SuniApp {
    fn drop_objects(&mut self, drag: &ObjectsDrag, target: usize, above: bool) {
        if !self.form_builder_ready() || self.dialog_open() || self.palette.open {
            return;
        }
        let Some(doc) = &self.document else { return };
        let page = doc.page;
        if self.edit.objects_page != Some(page) {
            return;
        }
        let Some(at) = drag.destination(
            page,
            self.view.epoch,
            self.edit.objects.len(),
            target,
            above,
        ) else {
            return;
        };
        self.edit.reselect_many =
            Some((page, Pick::Indices((at..at + drag.indices.len()).collect())));
        self.worker.send(Request::Edit(EditOp::ReorderObjects {
            page,
            indices: drag.indices.clone(),
            op: ZOrder::At(at),
        }));
    }

    pub fn objects_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        let Some(doc) = &self.document else { return };
        let page = doc.page;
        let count = self.edit.objects.len();
        let epoch = self.view.epoch;
        let ready = self.form_builder_ready()
            && !self.dialog_open()
            && !self.palette.open
            && ui.is_enabled()
            && self.edit.objects_page == Some(page);
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
                                        ui.add_enabled(
                                            ready,
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
                        .interact(if ready {
                            egui::Sense::click_and_drag()
                        } else {
                            egui::Sense::hover()
                        })
                        .on_hover_cursor(egui::CursorIcon::Grab)
                        .on_hover_text(tr(lang, "objects.drag_hint"));
                    // Libellé lisible pour les lecteurs d'écran (sélection comprise).
                    row.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            ready,
                            is_sel,
                            &label,
                        )
                    });
                    if row.hovered() {
                        hover = Some(o.index);
                    }
                    if ready && row.drag_started() {
                        let mut indices = if is_sel {
                            selected.clone()
                        } else {
                            vec![o.index]
                        };
                        indices.sort_unstable();
                        indices.dedup();
                        row.dnd_set_drag_payload(ObjectsDrag {
                            page,
                            epoch,
                            count,
                            indices,
                        });
                    }
                    if ready {
                        if let Some(drag) = row.dnd_hover_payload::<ObjectsDrag>() {
                            let above = ui
                                .input(|i| i.pointer.latest_pos())
                                .is_some_and(|pos| pos.y < row.rect.center().y);
                            if drag
                                .destination(page, epoch, count, o.index, above)
                                .is_some()
                            {
                                ui.painter().hline(
                                    row.rect.x_range(),
                                    if above {
                                        row.rect.top() - 2.0
                                    } else {
                                        row.rect.bottom() + 2.0
                                    },
                                    Stroke::new(3.0, p.accent),
                                );
                            }
                        }
                        if let Some(drag) = row.dnd_release_payload::<ObjectsDrag>() {
                            let above = ui
                                .input(|i| i.pointer.latest_pos())
                                .is_some_and(|pos| pos.y < row.rect.center().y);
                            action = Some(RowAction::Drop((*drag).clone(), o.index, above));
                        }
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
            Some(RowAction::Drop(drag, target, above)) => self.drop_objects(&drag, target, above),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(index: usize) -> sunipdf_core::ObjectInfo {
        sunipdf_core::ObjectInfo {
            index,
            kind: ObjectKind::Path,
            bounds: sunipdf_core::PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 10.0,
                top: 10.0,
            },
            quad: [(0.0, 0.0); 4],
            angle_deg: 0.0,
            opacity: Some(1.0),
            text: Some(format!("Object{index}")),
            font_size: 0.0,
            font_name: None,
            fill_rgb: None,
            stroke_rgb: None,
            stroke_width: 1.0,
        }
    }

    #[test]
    fn object_drop_destinations_follow_front_to_back_and_refuse_stale_or_noop_drags() {
        let drag = ObjectsDrag {
            page: 0,
            epoch: 7,
            count: 5,
            indices: vec![1, 3],
        };
        assert_eq!(drag.destination(0, 7, 5, 4, true), Some(3));
        assert_eq!(drag.destination(0, 7, 5, 0, false), Some(0));
        assert_eq!(drag.destination(0, 7, 5, 2, true), Some(2));
        for (page, epoch, count, target) in [
            (1, 7, 5, 4),
            (0, 8, 5, 4),
            (0, 7, 6, 4),
            (0, 7, 5, 3),
            (0, 7, 5, 5),
        ] {
            assert_eq!(drag.destination(page, epoch, count, target, true), None);
        }
        let contiguous = ObjectsDrag {
            indices: vec![1, 2],
            ..drag.clone()
        };
        assert_eq!(contiguous.destination(0, 7, 5, 3, false), None);
        assert_eq!(contiguous.destination(0, 7, 5, 0, true), None);
        for indices in [vec![], vec![3, 1], vec![1, 1], vec![99]] {
            assert_eq!(
                ObjectsDrag {
                    indices,
                    ..drag.clone()
                }
                .destination(0, 7, 5, 4, true),
                None
            );
        }
    }

    #[test]
    fn object_drop_dispatches_one_group_edit_and_keeps_pending_editor_guards() {
        for blocked in 0..6 {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.edit.objects = (0..5).map(object).collect();
            app.edit.objects_page = Some(0);
            let drag = ObjectsDrag {
                page: 0,
                epoch: app.view.epoch,
                count: 5,
                indices: vec![1, 3],
            };
            match blocked {
                1 => app.review.busy = true,
                2 => app.saving = true,
                3 => app.tabs.suspending = Some(0),
                4 => app.view.bump_epoch(),
                5 => app.edit.objects_page = Some(1),
                _ => {}
            }
            app.drop_objects(&drag, 4, true);
            let edits: Vec<_> = requests
                .try_iter()
                .filter(|r| matches!(r, Request::Edit(_)))
                .collect();
            if blocked == 0 {
                assert_eq!(edits.len(), 1);
                assert!(
                    matches!(&edits[0],Request::Edit(EditOp::ReorderObjects {page:0,indices,op:ZOrder::At(3)}) if indices==&vec![1,3])
                );
                assert_eq!(app.edit.reselect_many, Some((0, Pick::Indices(vec![3, 4]))));
            } else {
                assert!(edits.is_empty());
                assert!(app.edit.reselect_many.is_none());
            }
        }
    }

    #[test]
    fn objects_panel_drag_and_escape_work_in_both_languages() {
        fn position(shapes: &[egui::epaint::ClippedShape], label: &str) -> egui::Pos2 {
            fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            shapes
                .iter()
                .find_map(|s| find(&s.shape, label))
                .expect("visible object row")
        }
        for language in sunipdf_i18n::Language::ALL {
            for cancel in [false, true] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.settings.set_language(language);
                app.edit.objects = (0..5).map(object).collect();
                app.edit.objects_page = Some(0);
                let ctx = egui::Context::default();
                crate::icons::install(&ctx);
                let frame = |app: &mut SuniApp, events: Vec<egui::Event>| {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(320.0, 480.0),
                            )),
                            events,
                            ..Default::default()
                        },
                        |ui| app.objects_panel(&ctx, ui),
                    );
                    output.textures_delta.clear();
                    output
                };
                frame(&mut app, vec![]);
                let output = frame(&mut app, vec![]);
                let from = position(&output.shapes, "Object0");
                let to = position(&output.shapes, "Object4") - egui::vec2(0.0, 2.0);
                let button = |pos, pressed| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                };
                frame(
                    &mut app,
                    vec![egui::Event::PointerMoved(from), button(from, true)],
                );
                frame(
                    &mut app,
                    vec![egui::Event::PointerMoved(from + egui::vec2(10.0, 0.0))],
                );
                assert!(egui::DragAndDrop::has_payload_of_type::<ObjectsDrag>(&ctx));
                if cancel {
                    frame(
                        &mut app,
                        vec![egui::Event::Key {
                            key: egui::Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                    );
                    assert!(!egui::DragAndDrop::has_any_payload(&ctx));
                }
                frame(&mut app, vec![egui::Event::PointerMoved(to)]);
                frame(&mut app, vec![button(to, false)]);
                let edits: Vec<_> = requests
                    .try_iter()
                    .filter(|r| matches!(r, Request::Edit(_)))
                    .collect();
                if cancel {
                    assert!(edits.is_empty());
                } else {
                    assert_eq!(edits.len(), 1);
                    assert!(
                        matches!(&edits[0],Request::Edit(EditOp::ReorderObjects{page:0,indices,op:ZOrder::At(4)}) if indices==&vec![0])
                    );
                }
            }
        }
    }

    #[test]
    fn truncates_on_characters() {
        assert_eq!(truncate("court", 40), "court");
        assert_eq!(truncate("a  b\nc", 40), "a b c");
        assert_eq!(truncate("ééééé", 3), "ééé…");
        assert_eq!(truncate("日本語テキスト", 2), "日本…");
    }
}
