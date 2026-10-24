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
    pub(crate) drawing: bool,
    pub(crate) drag: Option<crate::annotate::Drag>,
    context: Option<(
        u64,
        crate::annotate::Tool,
        Option<crate::workspace::Workspace>,
    )>,
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
            drawing: false,
            drag: None,
            context: None,
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
        self.cancel();
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

    pub(crate) fn active(&self) -> bool {
        self.visible || self.drawing
    }

    pub(crate) fn cancel(&mut self) {
        self.visible = false;
        self.drawing = false;
        self.drag = None;
        self.context = None;
    }

    pub(crate) fn bind(
        &mut self,
        epoch: u64,
        tool: crate::annotate::Tool,
        workspace: Option<crate::workspace::Workspace>,
    ) {
        self.context = Some((epoch, tool, workspace));
    }

    pub(crate) fn matches_context(
        &self,
        epoch: u64,
        tool: crate::annotate::Tool,
        workspace: Option<crate::workspace::Workspace>,
    ) -> bool {
        self.context == Some((epoch, tool, workspace))
    }

    pub(crate) fn start_drawing(&mut self) {
        self.visible = false;
        self.drawing = true;
        self.drag = None;
    }

    pub(crate) fn start_drag(&mut self, page: usize, pt: (f32, f32), size: PageSize) {
        if !self.drawing
            || self.drag.is_some()
            || ![pt.0, pt.1, size.width_pt, size.height_pt]
                .iter()
                .all(|v| v.is_finite())
            || size.width_pt < 4.0
            || size.height_pt < 4.0
        {
            return;
        }
        self.page = page;
        self.size = size;
        let pt = (
            pt.0.clamp(0.0, size.width_pt),
            pt.1.clamp(0.0, size.height_pt),
        );
        self.drag = Some(crate::annotate::Drag {
            page,
            start: pt,
            current: pt,
        });
    }

    pub(crate) fn move_drag(&mut self, page: usize, pt: (f32, f32)) {
        if let Some(drag) = self.drag.as_mut().filter(|d| d.page == page) {
            if pt.0.is_finite() && pt.1.is_finite() {
                drag.current = (
                    pt.0.clamp(0.0, self.size.width_pt),
                    pt.1.clamp(0.0, self.size.height_pt),
                );
            }
        }
    }

    pub(crate) fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        let rect = drag.rect();
        if rect.right - rect.left < 4.0 || rect.top - rect.bottom < 4.0 {
            return;
        }
        self.rect = rect;
        self.drawing = false;
        self.visible = true;
    }

    pub(crate) fn paint(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        k: f32,
        h: f32,
    ) {
        if let Some(drag) = self.drag.filter(|d| d.page == page) {
            let rect = drag.rect();
            let pt = |x: f32, y: f32| page_rect.min + egui::vec2(x * k, (h - y) * k);
            let rect =
                egui::Rect::from_two_pos(pt(rect.left, rect.top), pt(rect.right, rect.bottom));
            let color = crate::theme::palette_for(painter.ctx()).accent;
            painter.rect_filled(rect, 0.0, color.gamma_multiply(0.12));
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.5, color),
                egui::StrokeKind::Inside,
            );
        }
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
        let mut draw = false;
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
                draw = ui.button(tr(lang, "form_builder.draw")).clicked();
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
            self.cancel();
        } else if draw {
            self.start_drawing();
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

    #[test]
    fn draw_button_keeps_all_five_field_drafts_until_explicit_create() {
        for lang in [Language::English, Language::French] {
            for kind in [
                FieldKind::Text,
                FieldKind::Checkbox,
                FieldKind::Radio,
                FieldKind::Combo,
                FieldKind::List,
            ] {
                let ctx = egui::Context::default();
                crate::icons::install(&ctx);
                let mut builder = FormBuilderState::default();
                builder.open(0, kind, builder.size);
                builder.name = "DrawnField".into();
                builder.options = "FR|France\nBE|Belgique".into();
                builder.editable = kind == FieldKind::Combo;
                builder.multi_select = kind == FieldKind::List;
                for _ in 0..3 {
                    frame(&mut builder, &ctx, lang, Vec::new());
                }
                let (_, output) = frame(&mut builder, &ctx, lang, Vec::new());
                let draw = text_position(&output.shapes, tr(lang, "form_builder.draw")).unwrap();
                assert!(click(&mut builder, &ctx, lang, draw).is_none());
                assert!(builder.drawing && !builder.visible);
                builder.start_drag(1, (180.0, 120.0), builder.size);
                builder.move_drag(1, (40.0, 30.0));
                builder.finish_drag();
                assert!(builder.visible && !builder.drawing);
                assert_eq!(builder.name, "DrawnField");
                assert_eq!(builder.options, "FR|France\nBE|Belgique");
                for _ in 0..3 {
                    assert!(frame(&mut builder, &ctx, lang, Vec::new()).0.is_none());
                }
                let (_, output) = frame(&mut builder, &ctx, lang, Vec::new());
                let create =
                    text_position(&output.shapes, tr(lang, "form_builder.create")).unwrap();
                let draft =
                    click(&mut builder, &ctx, lang, create).expect("only Create returns the field");
                assert_eq!(draft.page, 1);
                assert_eq!(draft.kind, kind);
                assert_eq!(
                    draft.rect,
                    PointRect {
                        left: 40.0,
                        bottom: 30.0,
                        right: 180.0,
                        top: 120.0
                    }
                );
                assert_eq!(draft.editable, kind == FieldKind::Combo);
                assert_eq!(draft.multi_select, kind == FieldKind::List);
                assert_eq!(
                    draft.options.len(),
                    if matches!(kind, FieldKind::Radio | FieldKind::Combo | FieldKind::List) {
                        2
                    } else {
                        0
                    }
                );
                assert!(!builder.active());
            }
        }
    }

    #[test]
    fn drawing_rejects_tiny_and_nonfinite_moves_clamps_and_pins_one_page() {
        let mut builder = FormBuilderState::default();
        builder.open(0, FieldKind::Radio, builder.size);
        builder.name = "Group".into();
        builder.start_drawing();
        builder.finish_drag();
        assert!(builder.drawing && !builder.visible);
        builder.start_drag(0, (f32::NAN, 20.0), builder.size);
        assert!(builder.drag.is_none());
        builder.start_drag(0, (10.0, 20.0), builder.size);
        builder.move_drag(0, (13.9, 30.0));
        builder.finish_drag();
        assert!(builder.drawing && !builder.visible);
        builder.start_drag(0, (10.0, 20.0), builder.size);
        builder.move_drag(1, (50.0, 80.0));
        builder.move_drag(0, (f32::INFINITY, 80.0));
        assert_eq!(builder.drag.unwrap().current, (10.0, 20.0));
        builder.move_drag(0, (-90.0, 900.0));
        assert_eq!(builder.drag.unwrap().current, (0.0, 792.0));
        builder.finish_drag();
        assert_eq!(
            builder.rect,
            PointRect {
                left: 0.0,
                bottom: 20.0,
                right: 10.0,
                top: 792.0
            }
        );
        assert!(builder.visible);
        builder.cancel();
        assert!(!builder.active() && builder.drag.is_none());
    }

    #[test]
    fn drawing_keeps_radio_per_option_size_and_off_validation() {
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        let mut builder = FormBuilderState::default();
        builder.open(0, FieldKind::Radio, builder.size);
        builder.name = "Group".into();
        builder.start_drawing();
        builder.start_drag(0, (20.0, 20.0), builder.size);
        builder.move_drag(0, (100.0, 24.0));
        builder.finish_drag();
        for _ in 0..3 {
            frame(&mut builder, &ctx, Language::English, Vec::new());
        }
        let (_, output) = frame(&mut builder, &ctx, Language::English, Vec::new());
        let create =
            text_position(&output.shapes, tr(Language::English, "form_builder.create")).unwrap();
        assert!(click(&mut builder, &ctx, Language::English, create).is_none());
        builder.rect.top = 40.0;
        builder.options = "Off\nYes".into();
        frame(&mut builder, &ctx, Language::English, Vec::new());
        assert!(click(&mut builder, &ctx, Language::English, create).is_none());
        assert!(builder.visible);
    }

    #[test]
    fn real_document_widget_draws_over_existing_fill_widget_then_explicit_create_edits() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        app.zoom = crate::app::Zoom::Fixed(0.5);
        app.annotate.tool = crate::annotate::Tool::FillForm;
        app.view.reading.rotation = crate::reading::ViewRotation::Clockwise;
        let existing = PointRect {
            left: 40.0,
            bottom: 600.0,
            right: 200.0,
            top: 700.0,
        };
        app.forms.info = Some(sunipdf_core::FormInfo {
            kind: sunipdf_core::FormKind::AcroForm,
            has_scripts: false,
            fields: vec![sunipdf_core::FormField {
                name: "Existing".into(),
                kind: FieldKind::Checkbox,
                value: sunipdf_core::FieldValue::Bool(false),
                tooltip: None,
                read_only: false,
                required: false,
                multiline: false,
                password: false,
                comb: false,
                max_len: None,
                multi_select: false,
                editable: false,
                no_toggle_off: false,
                options: Vec::new(),
                widgets: vec![sunipdf_core::FieldWidget {
                    page: 0,
                    rect: existing,
                    display: existing,
                    on_state: Some("Yes".into()),
                    export_value: None,
                }],
            }],
        });
        app.execute(&ctx, sunipdf_core::Command::ToolAddTextField);
        app.form_builder.name = "DrawnOverWidget".into();
        let run = |app: &mut crate::app::SuniApp, events| {
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
                    app.document_view(&ctx, ui);
                    app.feature_dialogs(&ctx);
                },
            );
            output.textures_delta.clear();
            output
        };
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        assert_eq!(
            app.view.reading.rotation,
            crate::reading::ViewRotation::Upright
        );
        let output = run(&mut app, Vec::new());
        let draw =
            text_position(&output.shapes, tr(Language::English, "form_builder.draw")).unwrap();
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(
            &mut app,
            vec![egui::Event::PointerMoved(draw), button(draw, true)],
        );
        run(&mut app, vec![button(draw, false)]);
        assert!(app.form_builder.drawing);
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let (_, page_rect, h) = app.edit.page_view.unwrap();
        let k = app.effective_zoom * crate::app::SCREEN_PER_PDF_POINT;
        let pt = |x: f32, y: f32| page_rect.min + egui::vec2(x * k, (h - y) * k);
        let (start, end) = (pt(150.0, 650.0), pt(50.0, 610.0));
        run(
            &mut app,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        run(&mut app, vec![egui::Event::PointerMoved(end)]);
        assert!(
            app.form_builder.drag.is_some(),
            "the real page widget owns the primary drag"
        );
        run(&mut app, vec![button(end, false)]);
        assert!(app.form_builder.visible && !app.form_builder.drawing);
        let rect = app.form_builder.rect;
        for (actual, expected) in [
            (rect.left, 50.0),
            (rect.bottom, 610.0),
            (rect.right, 150.0),
            (rect.top, 650.0),
        ] {
            assert!((actual - expected).abs() < 0.01);
        }
        assert!(app.forms.editing.is_none() && app.forms.popup.is_none());
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, crate::worker::Request::Edit(_))),
            "draw and review do not change the existing checkbox or PDF"
        );
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let output = run(&mut app, Vec::new());
        let create =
            text_position(&output.shapes, tr(Language::English, "form_builder.create")).unwrap();
        run(
            &mut app,
            vec![egui::Event::PointerMoved(create), button(create, true)],
        );
        run(&mut app, vec![button(create, false)]);
        let edits: Vec<_> = requests
            .try_iter()
            .filter(|r| matches!(r, crate::worker::Request::Edit(_)))
            .collect();
        assert_eq!(edits.len(), 1);
        assert!(
            matches!(&edits[0], crate::worker::Request::Edit(crate::worker::EditOp::AddFormField { page: 0, name, rect: created, .. }) if name == "DrawnOverWidget" && *created == rect)
        );
    }
}
