//! Creation UI; the engine validates every field and performs the undoable edit.

use crate::dialog::{self, Spec};
use egui::{Button, DragValue};
use sunipdf_core::{FieldKind, FieldOption, PageSize, PointRect};
use sunipdf_i18n::{tr, Language};

pub struct FormFieldDraft {
    pub page: usize,
    pub name: String,
    pub kind: FieldKind,
    pub rect: PointRect,
    pub options: Vec<FieldOption>,
    pub multi_select: bool,
    pub editable: bool,
}

pub struct FormBuilderState {
    pub visible: bool,
    page: usize,
    kind: FieldKind,
    name: String,
    rect: PointRect,
    size: PageSize,
    options: String,
    multi_select: bool,
    editable: bool,
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
            options: String::new(),
            multi_select: false,
            editable: false,
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
        self.options = "1\n2".into();
        self.multi_select = false;
        self.editable = false;
        let width: f32 = if kind == FieldKind::Checkbox {
            20.0
        } else {
            180.0
        };
        let height: f32 = if matches!(kind, FieldKind::Radio | FieldKind::List) {
            72.0
        } else {
            24.0
        };
        self.rect = PointRect {
            left: size.width_pt.min(36.0) / 2.0,
            bottom: size.height_pt.min(36.0) / 2.0,
            right: (width + 18.0).min(size.width_pt),
            top: (18.0 + height).min(size.height_pt),
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
        let choice = matches!(
            self.kind,
            FieldKind::Radio | FieldKind::Combo | FieldKind::List
        );
        let options = if choice {
            parse_options(&self.options)
        } else {
            Some(Vec::new())
        };
        let valid = options.as_ref().is_some_and(|o| {
            self.kind != FieldKind::Radio
                || ((self.rect.top - self.rect.bottom) / o.len().max(1) as f32 >= 4.0
                    && o.iter().all(|o| o.value != "Off"))
        }) && !self.name.trim().is_empty()
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
                    match self.kind {
                        FieldKind::Text => "form_builder.text",
                        FieldKind::Checkbox => "form_builder.checkbox",
                        FieldKind::Radio => "form_builder.radio",
                        FieldKind::Combo => "form_builder.combo",
                        FieldKind::List => "form_builder.list",
                        _ => "form_builder.title",
                    },
                ));
                ui.label(tr(lang, "form_builder.name"));
                ui.text_edit_singleline(&mut self.name);
                ui.label(tr(lang, "form_builder.name_hint"));
                if choice {
                    ui.label(tr(lang, "form_builder.options"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.options)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label(tr(lang, "form_builder.options_hint"));
                    if self.kind == FieldKind::Radio {
                        ui.label(tr(lang, "form_builder.radio_hint"));
                    }
                    if self.kind == FieldKind::List {
                        ui.checkbox(
                            &mut self.multi_select,
                            tr(lang, "form_builder.multi_select"),
                        );
                    }
                    if self.kind == FieldKind::Combo {
                        ui.checkbox(&mut self.editable, tr(lang, "form_builder.editable"));
                    }
                }
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
            options: options.unwrap_or_default(),
            multi_select: self.multi_select,
            editable: self.kind == FieldKind::Combo && self.editable,
        })
    }
}

fn parse_options(text: &str) -> Option<Vec<FieldOption>> {
    let mut seen = std::collections::HashSet::new();
    let options: Vec<_> = text
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|line| {
            let (value, label) = line.split_once('|').unwrap_or((line, line));
            FieldOption {
                value: value.trim().into(),
                label: label.trim().into(),
            }
        })
        .collect();
    (!options.is_empty()
        && options.len() <= 1000
        && options.iter().all(|o| {
            !o.value.is_empty()
                && o.value.len() <= 1024
                && o.label.len() <= 4096
                && !o.value.chars().any(char::is_control)
                && !o.label.chars().any(char::is_control)
                && seen.insert(&o.value)
        }))
    .then_some(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        builder: &mut FormBuilderState,
        ctx: &egui::Context,
        lang: Language,
        events: Vec<egui::Event>,
    ) -> (Option<FormFieldDraft>, egui::FullOutput) {
        let mut draft = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1000.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                if let Some(created) = builder.show(ui.ctx(), lang, false, false) {
                    draft = Some(created);
                }
            },
        );
        output.textures_delta.clear();
        (draft, output)
    }

    fn text_position(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<egui::Pos2> {
        fn find(shape: &egui::Shape, text: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, text)),
                _ => None,
            }
        }
        shapes.iter().find_map(|s| find(&s.shape, text))
    }

    fn click(
        builder: &mut FormBuilderState,
        ctx: &egui::Context,
        lang: Language,
        position: egui::Pos2,
    ) -> Option<FormFieldDraft> {
        frame(
            builder,
            ctx,
            lang,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            builder,
            ctx,
            lang,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .0
    }

    #[test]
    fn editable_combo_creation_checkbox_validates_and_reaches_the_draft() {
        for lang in [Language::English, Language::French] {
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            let mut builder = FormBuilderState::default();
            builder.open(2, FieldKind::Combo, builder.size);
            assert!(!builder.editable);
            builder.options = "FR|France\nBE|Belgique".into();
            for _ in 0..3 {
                frame(&mut builder, &ctx, lang, Vec::new());
            }
            let (_, output) = frame(&mut builder, &ctx, lang, Vec::new());
            let toggle = text_position(&output.shapes, tr(lang, "form_builder.editable"))
                .expect("localized custom-text checkbox is visible");
            assert!(click(&mut builder, &ctx, lang, toggle).is_none());
            assert!(builder.editable);
            let (_, output) = frame(&mut builder, &ctx, lang, Vec::new());
            let create = text_position(&output.shapes, tr(lang, "form_builder.create")).unwrap();
            assert!(click(&mut builder, &ctx, lang, create).is_none());
            assert!(builder.visible, "an empty field name keeps Create disabled");
            builder.name = "Country".into();
            frame(&mut builder, &ctx, lang, Vec::new());
            let draft = click(&mut builder, &ctx, lang, create).unwrap();
            assert_eq!(draft.page, 2);
            assert_eq!(draft.name, "Country");
            assert_eq!(draft.kind, FieldKind::Combo);
            assert!(draft.editable);
            assert!(!draft.multi_select);
            assert_eq!(draft.options, parse_options(&builder.options).unwrap());
            assert!(!builder.visible);

            for kind in [
                FieldKind::Text,
                FieldKind::Checkbox,
                FieldKind::Radio,
                FieldKind::List,
            ] {
                builder.open(0, kind, builder.size);
                assert!(
                    !builder.editable,
                    "new fields start with custom text disabled"
                );
                builder.name = "NewField".into();
                for _ in 0..3 {
                    frame(&mut builder, &ctx, lang, Vec::new());
                }
                let (_, output) = frame(&mut builder, &ctx, lang, Vec::new());
                assert!(text_position(&output.shapes, tr(lang, "form_builder.editable")).is_none());
                let create =
                    text_position(&output.shapes, tr(lang, "form_builder.create")).unwrap();
                assert!(!click(&mut builder, &ctx, lang, create).unwrap().editable);
            }
        }
    }

    #[test]
    fn choice_options_have_unique_export_values_and_labels() {
        assert_eq!(
            parse_options("FR|France\nBE|Belgique").unwrap(),
            vec![
                FieldOption {
                    value: "FR".into(),
                    label: "France".into()
                },
                FieldOption {
                    value: "BE".into(),
                    label: "Belgique".into()
                }
            ]
        );
        for invalid in ["", "one\none", "|missing"] {
            assert!(parse_options(invalid).is_none());
        }
    }
}
