//! Creation UI; the engine validates every field and performs the undoable edit.

use crate::dialog::{self, Spec};
use egui::{Button, DragValue};
use sunipdf_core::{FieldKind, PageSize, PointRect};
use sunipdf_i18n::{tr, Language};

pub struct FormFieldDraft {
    pub page: usize,
    pub name: String,
    pub kind: FieldKind,
    pub rect: PointRect,
}

pub struct FormBuilderState {
    pub visible: bool,
    page: usize,
    kind: FieldKind,
    name: String,
    rect: PointRect,
    size: PageSize,
}
impl Default for FormBuilderState {
    fn default() -> Self {
        Self {
            visible: false,
            page: 0,
            kind: FieldKind::Text,
            name: String::new(),
            rect: PointRect {
                left: 36.0,
                bottom: 36.0,
                right: 216.0,
                top: 60.0,
            },
            size: PageSize {
                width_pt: 612.0,
                height_pt: 792.0,
            },
        }
    }
}
impl FormBuilderState {
    pub fn open(&mut self, page: usize, kind: FieldKind, size: PageSize) {
        self.page = page;
        self.kind = kind;
        self.size = size;
        self.name.clear();
        let width: f32 = if kind == FieldKind::Text { 180.0 } else { 20.0 };
        self.rect = PointRect {
            left: size.width_pt.min(36.0) / 2.0,
            bottom: size.height_pt.min(36.0) / 2.0,
            right: (width + 18.0).min(size.width_pt),
            top: 42.0f32.min(size.height_pt),
        };
        self.visible = true;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        lang: Language,
        dark: bool,
        has_icons: bool,
    ) -> Option<FormFieldDraft> {
        if !self.visible {
            return None;
        }
        let mut create = false;
        let mut close = false;
        let valid = !self.name.trim().is_empty()
            && self.name.chars().count() <= 256
            && !self.name.chars().any(|c| c == '.' || c.is_control())
            && [
                self.rect.left,
                self.rect.bottom,
                self.rect.right,
                self.rect.top,
            ]
            .iter()
            .all(|v| v.is_finite())
            && self.rect.left >= 0.0
            && self.rect.bottom >= 0.0
            && self.rect.right <= self.size.width_pt
            && self.rect.top <= self.size.height_pt
            && self.rect.right - self.rect.left >= 4.0
            && self.rect.top - self.rect.bottom >= 4.0;
        dialog::show(
            ctx,
            Spec::new(
                "form-builder",
                tr(lang, "form_builder.title"),
                dark,
                has_icons,
            ),
            |ui| {
                ui.label(tr(
                    lang,
                    if self.kind == FieldKind::Text {
                        "form_builder.text"
                    } else {
                        "form_builder.checkbox"
                    },
                ));
                ui.label(tr(lang, "form_builder.name"));
                ui.text_edit_singleline(&mut self.name);
                ui.label(tr(lang, "form_builder.name_hint"));
                ui.label(tr(lang, "form_builder.coordinates"));
                egui::Grid::new("field-bounds").show(ui, |ui| {
                    for (key, v) in [
                        ("form_builder.left", &mut self.rect.left),
                        ("form_builder.bottom", &mut self.rect.bottom),
                        ("form_builder.right", &mut self.rect.right),
                        ("form_builder.top", &mut self.rect.top),
                    ] {
                        ui.label(tr(lang, key));
                        ui.add(DragValue::new(v).speed(1.0).suffix(" pt"));
                        ui.end_row();
                    }
                });
            },
            |ui| {
                create = ui
                    .add_enabled(valid, Button::new(tr(lang, "form_builder.create")))
                    .clicked();
                close = ui.button(tr(lang, "dialog.password.cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if close || create {
            self.visible = false;
        }
        create.then(|| FormFieldDraft {
            page: self.page,
            name: self.name.clone(),
            kind: self.kind,
            rect: self.rect,
        })
    }
}
