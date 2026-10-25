//! Shared literal register-text criterion controls for the register and quantities tables.

use sunipdf_core::annotation_register::{RegisterFieldFilter, RegisterFilter};
use sunipdf_i18n::{tr, Language};

pub(crate) fn menu(ui: &mut egui::Ui, lang: Language, filter: &mut RegisterFilter) -> bool {
    let mut changed = false;
    ui.menu_button(tr(lang, "register.field_filter.title"), |ui| {
        changed = controls(ui, lang, filter);
    });
    changed
}

fn controls(ui: &mut egui::Ui, lang: Language, filter: &mut RegisterFilter) -> bool {
    ui.set_max_width(320.0);
    ui.label(tr(lang, "register.field_filter.help"));
    let mut changed = false;
    if let Some(field) = &mut filter.custom_field {
        for (key, value, limit) in [
            ("register.field_filter.key", &mut field.key, 64),
            ("register.field_filter.value", &mut field.value, 1024),
        ] {
            let text = tr(lang, key);
            let label = ui.label(text);
            let response = ui
                .add(
                    egui::TextEdit::singleline(value)
                        .char_limit(limit)
                        .desired_width(ui.available_width().min(300.0)),
                )
                .labelled_by(label.id);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::TextInput, ui.is_enabled(), text)
            });
            changed |= response.changed();
        }
        if field.validate().is_err() {
            ui.colored_label(
                crate::theme::ERROR,
                tr(lang, "register.field_filter.invalid"),
            );
        }
        if ui.button(tr(lang, "register.field_filter.clear")).clicked() {
            filter.custom_field = None;
            changed = true;
        }
    } else if ui.button(tr(lang, "register.field_filter.add")).clicked() {
        filter.custom_field = Some(RegisterFieldFilter {
            key: String::new(),
            value: String::new(),
        });
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation_register::{RegisterState, SelectionCache};
    use std::{collections::BTreeSet, sync::Arc};
    use sunipdf_core::{
        annotation_register::RegisterSort,
        register_presets::{RegisterPreset, RegisterPresetLibrary},
    };

    struct Controls {
        ctx: egui::Context,
        time: f64,
        lang: Language,
    }
    impl Controls {
        fn new(lang: Language, dark: bool) -> Self {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            crate::theme::apply(&ctx, dark);
            Self {
                ctx,
                time: 0.0,
                lang,
            }
        }
        fn frame(
            &mut self,
            filter: &mut RegisterFilter,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            self.time += 0.025;
            let mut frame = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 400.0),
                    )),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    controls(ui, self.lang, filter);
                },
            );
            frame.textures_delta.clear();
            frame
        }
        fn point(
            &mut self,
            filter: &mut RegisterFilter,
            key: &str,
            role: egui::accesskit::Role,
        ) -> egui::Pos2 {
            let label = tr(self.lang, key);
            let mut found = None;
            for _ in 0..3 {
                let frame = self.frame(filter, vec![]);
                if let Some(tree) = &frame.platform_output.accesskit_update {
                    found = tree.nodes.iter().find_map(|(_, node)| {
                        (node.role() == role && node.label() == Some(label))
                            .then(|| node.bounds())
                            .flatten()
                            .map(|rect| {
                                let bounds = egui::Rect::from_min_max(
                                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
                                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
                                );
                                assert!(
                                    egui::Rect::from_min_size(
                                        egui::Pos2::ZERO,
                                        egui::vec2(360.0, 400.0)
                                    )
                                    .contains_rect(bounds),
                                    "control fits a narrow window: {label}"
                                );
                                bounds.center()
                            })
                    });
                }
            }
            found.unwrap_or_else(|| panic!("accessible field filter control: {label}"))
        }
        fn click(&mut self, filter: &mut RegisterFilter, key: &str, role: egui::accesskit::Role) {
            let pos = self.point(filter, key, role);
            for pressed in [true, false] {
                self.frame(
                    filter,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        fn type_value(&mut self, filter: &mut RegisterFilter, key: &str, value: &str) {
            self.click(filter, key, egui::accesskit::Role::TextInput);
            self.frame(
                filter,
                vec![
                    egui::Event::Key {
                        key: egui::Key::A,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            ctrl: true,
                            command: true,
                            ..Default::default()
                        },
                    },
                    egui::Event::Text(value.into()),
                ],
            );
        }
    }

    #[test]
    fn narrow_localized_controls_keep_empty_values_and_spaces_and_clear_explicitly() {
        for lang in [Language::English, Language::French] {
            for dark in [false, true] {
                let mut ui = Controls::new(lang, dark);
                let mut filter = RegisterFilter::default();
                ui.click(
                    &mut filter,
                    "register.field_filter.add",
                    egui::accesskit::Role::Button,
                );
                assert!(filter.custom_field.is_some());
                assert!(filter.validate_custom_field().is_err());
                ui.type_value(&mut filter, "register.field_filter.key", "Room");
                assert_eq!(filter.custom_field.as_ref().unwrap().key, "Room");
                assert_eq!(filter.custom_field.as_ref().unwrap().value, "");
                assert!(filter.validate_custom_field().is_ok());
                ui.type_value(&mut filter, "register.field_filter.value", " A101 ");
                assert_eq!(filter.custom_field.as_ref().unwrap().value, " A101 ");
                let frame = ui.frame(&mut filter, vec![]);
                crate::workspace_table::assert_visible_text_contrast(
                    &frame,
                    &[
                        tr(lang, "register.field_filter.help"),
                        tr(lang, "register.field_filter.key"),
                        tr(lang, "register.field_filter.value"),
                        tr(lang, "register.field_filter.clear"),
                    ],
                    dark,
                );
                ui.click(
                    &mut filter,
                    "register.field_filter.clear",
                    egui::accesskit::Role::Button,
                );
                assert!(filter.custom_field.is_none());
            }
        }
    }

    #[test]
    fn literal_field_changes_rebuild_cached_rows_clear_bulk_and_keep_the_report_unchanged() {
        let mut register = (*super::super::tests::large_register()).clone();
        register.rows.truncate(6);
        for (row, value) in register.rows.iter_mut().zip([
            Some("A101"),
            Some("a101"),
            Some(""),
            Some(" "),
            None,
            Some("001"),
        ]) {
            if let Some(value) = value {
                row.fields.custom.insert("Room".into(), value.into());
            }
        }
        let before = serde_json::to_vec(&register.rows).unwrap();
        let report = Arc::new(register);
        let mut state = RegisterState {
            report: Some(report.clone()),
            ..Default::default()
        };
        let mut cache = SelectionCache::default();
        let original = cache.get(&report, &state.filter, RegisterSort::Page, false);
        assert_eq!(original.rows.len(), 6);
        for (key, value, expected) in [
            ("Room", "A101", vec![0]),
            ("Room", "a101", vec![1]),
            ("Room", "", vec![2]),
            ("Room", " ", vec![3]),
            ("room", "A101", vec![]),
            ("Room", "1", vec![]),
            ("Room", "001", vec![5]),
        ] {
            state.bulk_selected = BTreeSet::from([(1, 0)]);
            state.filter.custom_field = Some(RegisterFieldFilter {
                key: key.into(),
                value: value.into(),
            });
            state.sync_bulk_filter();
            assert!(state.bulk_selected.is_empty());
            let selected = cache.get(&report, &state.filter, RegisterSort::Page, false);
            assert_eq!(selected.rows, expected);
            assert!(!Arc::ptr_eq(&selected, &original));
        }
        state.filter.query = "no match".into();
        assert!(cache
            .get(&report, &state.filter, RegisterSort::Page, false)
            .rows
            .is_empty());
        state.filter.query.clear();
        state.bulk_selected.insert((1, 5));
        state.filter.custom_field = None;
        state.sync_bulk_filter();
        assert!(state.bulk_selected.is_empty());
        assert_eq!(
            cache
                .get(&report, &state.filter, RegisterSort::Page, false)
                .rows
                .len(),
            6
        );
        assert_eq!(serde_json::to_vec(&report.rows).unwrap(), before);
    }

    #[test]
    fn saved_field_filter_roundtrip_preserves_scope_and_clears_existing_selection() {
        use sunipdf_core::plan_zones::{ZoneAssignmentStatus, ZoneFilter};
        let filter = RegisterFilter {
            custom_field: Some(RegisterFieldFilter {
                key: "Room".into(),
                value: String::new(),
            }),
            ..Default::default()
        };
        let mut library = RegisterPresetLibrary::default();
        library
            .insert(
                RegisterPreset::capture("Empty room", &filter, RegisterSort::Author, true).unwrap(),
            )
            .unwrap();
        let bytes = library.to_json().unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
            2
        );
        let library = RegisterPresetLibrary::from_json(&bytes).unwrap();
        let mut state = RegisterState::default();
        state.filter.page = Some(3);
        state.filter.zone = Some(ZoneFilter {
            page: 3,
            id: "level-3".into(),
            descendants: true,
        });
        state.filter.zone_status = Some(ZoneAssignmentStatus::Assigned);
        let zone = state.filter.zone.clone();
        for _ in 0..2 {
            state.bulk_selected.insert((3, 0));
            state.offset = 50;
            state
                .load_preset(library.get("Empty room").unwrap())
                .unwrap();
            assert_eq!(state.filter.custom_field, filter.custom_field);
            assert_eq!(state.filter.page, Some(3));
            assert_eq!(state.filter.zone, zone);
            assert_eq!(
                state.filter.zone_status,
                Some(ZoneAssignmentStatus::Assigned)
            );
            assert_eq!(state.offset, 0);
            assert!(state.bulk_selected.is_empty());
        }
        let summary = super::super::filter_summary(&state.filter, &[], Language::English);
        assert!(summary.iter().any(|line| line.contains("\"Room\" = \"\"")));
    }

    #[test]
    fn changing_the_field_filter_cancels_a_pending_bulk_preview_without_edit_requests() {
        use crate::worker::Request;
        use std::sync::atomic::Ordering;
        let (app, requests, _) = crate::app::tests::test_app();
        let mut register = (*super::super::tests::large_register()).clone();
        register.rows.truncate(2);
        let register = Arc::new(register);
        let mut state = RegisterState::default();
        state.bulk_selected = BTreeSet::from([(1, 0), (1, 1)]);
        state
            .bulk
            .bind(register.clone(), register.rows.clone(), app.view.epoch);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut time = 0.0;
        let mut frame = |state: &mut RegisterState, events| {
            time += 0.025;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |_| {
                    state
                        .bulk
                        .show(&ctx, &app.worker, app.view.epoch, app.lang(), false, false)
                },
            );
            output.textures_delta.clear();
            output
        };
        let mut point = None;
        for _ in 0..3 {
            let output = frame(&mut state, vec![]);
            if let Some(tree) = &output.platform_output.accesskit_update {
                point = tree.nodes.iter().find_map(|(_, node)| {
                    (node.role() == egui::accesskit::Role::Button
                        && node.label() == Some(tr(app.lang(), "register.bulk.preview")))
                    .then(|| node.bounds())
                    .flatten()
                    .map(|rect| {
                        egui::pos2(
                            ((rect.x0 + rect.x1) / 2.0) as f32,
                            ((rect.y0 + rect.y1) / 2.0) as f32,
                        )
                    })
                });
            }
        }
        let pos = point.expect("actual preview control");
        for pressed in [true, false] {
            frame(
                &mut state,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let job = match requests.try_recv().unwrap() {
            Request::PreviewRegisterBatch { job, .. } => job,
            _ => panic!("bulk preview request"),
        };
        state.filter.custom_field = Some(RegisterFieldFilter {
            key: "Room".into(),
            value: String::new(),
        });
        state.sync_bulk_filter();
        assert!(state.bulk_selected.is_empty());
        assert!(job.cancel.load(Ordering::Acquire));
        assert!(job
            .reply
            .send(Err(sunipdf_core::EngineError::Other("late preview".into())))
            .is_err());
        assert!(
            requests.try_recv().is_err(),
            "filter changes do not send document edits"
        );
        assert!(!app.has_unsaved_changes());
    }
}
