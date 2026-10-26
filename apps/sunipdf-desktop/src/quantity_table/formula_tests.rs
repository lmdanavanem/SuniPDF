use super::{draw_formulas, FormulaDraft, QuantityOptions, QuantityUnit};
use egui::{Event, FullOutput, Modifiers, PointerButton, Pos2, Rect};
use sunipdf_core::quantity_table::FormulaColumn;
use sunipdf_i18n::{tr, Language};

struct Controls {
    ctx: egui::Context,
    time: f64,
    changed_frames: usize,
    copied: Vec<String>,
}

impl Controls {
    fn new(dark: bool) -> Self {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx, dark);
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        Self {
            ctx,
            time: 1.0,
            changed_frames: 0,
            copied: Vec::new(),
        }
    }

    fn viewport() -> Rect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(560.0, 400.0))
    }

    fn frame(
        &mut self,
        options: &mut QuantityOptions,
        draft: &mut FormulaDraft,
        lang: Language,
        events: Vec<Event>,
    ) -> FullOutput {
        self.time += 0.025;
        let mut changed = false;
        let mut output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Self::viewport()),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                // Match the real formula menu's available width and scrolling height.
                ui.set_max_width(540.0);
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        changed = draw_formulas(ui, options, draft, lang);
                    });
            },
        );
        self.changed_frames += usize::from(changed);
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied.push(text.clone());
            }
        }
        output.textures_delta.clear();
        output
    }

    fn click(
        &mut self,
        options: &mut QuantityOptions,
        draft: &mut FormulaDraft,
        lang: Language,
        key: &str,
    ) {
        let label = tr(lang, key);
        for direction in [-1.0, 1.0] {
            for _ in 0..20 {
                self.time += 0.6;
                let output = self.frame(options, draft, lang, vec![]);
                if let Some(rect) = visible_text(&output, label) {
                    let pos = rect.center();
                    for pressed in [true, false] {
                        self.frame(
                            options,
                            draft,
                            lang,
                            vec![
                                Event::PointerMoved(pos),
                                Event::PointerButton {
                                    pos,
                                    button: PointerButton::Primary,
                                    pressed,
                                    modifiers: Modifiers::NONE,
                                },
                            ],
                        );
                    }
                    self.frame(options, draft, lang, vec![]);
                    return;
                }
                self.frame(
                    options,
                    draft,
                    lang,
                    vec![
                        Event::PointerMoved(Self::viewport().center()),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, direction * 90.0),
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Wheel,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        panic!("formula control is not reachable: {lang:?}: {key}");
    }
}

fn text_rects(shape: &egui::Shape, label: &str, found: &mut Vec<Rect>) {
    match shape {
        egui::Shape::Text(text) if text.galley.text() == label => {
            found.push(Rect::from_min_size(text.pos, text.galley.size()));
        }
        egui::Shape::Vec(shapes) => {
            for shape in shapes {
                text_rects(shape, label, found);
            }
        }
        _ => {}
    }
}

fn visible_text(output: &FullOutput, label: &str) -> Option<Rect> {
    output.shapes.iter().find_map(|shape| {
        let mut found = Vec::new();
        text_rects(&shape.shape, label, &mut found);
        found.into_iter().find(|rect| {
            Controls::viewport().contains_rect(*rect) && shape.clip_rect.contains_rect(*rect)
        })
    })
}

fn rendered_text(output: &FullOutput, label: &str) -> bool {
    output.shapes.iter().any(|shape| {
        let mut found = Vec::new();
        text_rects(&shape.shape, label, &mut found);
        !found.is_empty()
    })
}

#[test]
fn compact_formula_menu_accepts_conversion_and_preserves_valid_configuration() {
    for lang in Language::ALL {
        for dark in [false, true] {
            let mut controls = Controls::new(dark);
            let mut options = QuantityOptions::default();
            let mut draft = FormulaDraft {
                name: "Converted length".into(),
                expression: "convert(quantity, m) * 1.1".into(),
                quantity_unit: QuantityUnit::Feet,
                rounding: 4,
                ..Default::default()
            };
            for _ in 0..2 {
                controls.frame(&mut options, &mut draft, lang, vec![]);
            }
            let output = controls.frame(&mut options, &mut draft, lang, vec![]);
            for key in [
                "quantity.formula_help",
                "quantity.formula_rules_title",
                "quantity.formula_add",
            ] {
                assert!(
                    visible_text(&output, tr(lang, key)).is_some(),
                    "initial action/help is clipped: {lang:?}, dark={dark}, {key}"
                );
            }
            assert!(
                !rendered_text(&output, tr(lang, "quantity.formula_rules")),
                "detailed rules start collapsed"
            );
            crate::workspace_table::assert_visible_text_contrast(
                &output,
                &[tr(lang, "quantity.formula_add")],
                dark,
            );
            controls.click(&mut options, &mut draft, lang, "quantity.formula_add");
            assert_eq!(
                controls.changed_frames, 1,
                "actual Add click requests a report refresh"
            );
            let expected = vec![FormulaColumn {
                name: "Converted length".into(),
                expression: "convert(quantity, m) * 1.1".into(),
                quantity_unit: Some(QuantityUnit::Feet),
                rounding_decimals: 4,
            }];
            assert_eq!(options.formulas, expected);
            assert!(draft.error.is_none());

            draft.name = "Malformed conversion".into();
            draft.expression = "convert(quantity, kg)".into();
            controls.click(&mut options, &mut draft, lang, "quantity.formula_add");
            assert_eq!(
                options.formulas, expected,
                "invalid syntax must not replace or append formulas"
            );
            assert_eq!(controls.changed_frames, 1);
            assert!(draft.error.is_some());

            controls.click(&mut options, &mut draft, lang, "quantity.formula_config");
            controls.click(&mut options, &mut draft, lang, "quantity.formula_copy");
            let copied = controls
                .copied
                .last()
                .expect("actual Copy action emits clipboard text")
                .clone();
            assert_eq!(copied, draft.json);
            assert_eq!(
                QuantityOptions::formulas_from_json(copied.as_bytes()).unwrap(),
                expected
            );
            assert_eq!(
                controls.changed_frames, 1,
                "Copy does not request a report refresh"
            );

            controls.click(&mut options, &mut draft, lang, "quantity.remove_column");
            assert!(options.formulas.is_empty());
            assert_eq!(controls.changed_frames, 2);
            controls.click(&mut options, &mut draft, lang, "quantity.formula_load");
            assert_eq!(options.formulas, expected);
            assert_eq!(controls.changed_frames, 3);
            assert!(draft.error.is_none());

            draft.json = r#"[{"name":"Bad","expression":"convert(quantity,m,m)","quantity_unit":"ft","rounding_decimals":4}]"#.into();
            controls.click(&mut options, &mut draft, lang, "quantity.formula_load");
            assert_eq!(options.formulas, expected);
            assert_eq!(controls.changed_frames, 3);
            assert!(draft.error.is_some());
            draft.json = copied;
            controls.click(&mut options, &mut draft, lang, "quantity.formula_load");
            assert_eq!(options.formulas, expected);
            assert_eq!(controls.changed_frames, 4);
            assert!(draft.error.is_none());
        }
    }
}
