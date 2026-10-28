//! Explicit reuse of portable register criteria; document scope stays in the register.

use super::RegisterState;
use crate::{dialog, settings::Settings};
use std::{io::Read, path::Path};
use sunipdf_core::{
    register_presets::{
        RegisterPreset, RegisterPresetLibrary, MAX_REGISTER_PRESETS, MAX_REGISTER_PRESET_JSON_BYTES,
    },
    EngineError,
};
use sunipdf_i18n::{tr, Language};

#[derive(Default)]
pub(super) struct Draft {
    pub open: bool,
    name: String,
    selected: Option<String>,
    error: Option<String>,
    notice: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Save(String),
    Load(String),
    Delete(String),
    Import,
    Export,
}

impl Draft {
    #[cfg(all(test, windows))]
    pub(super) fn evidence_open(&mut self, name: &str) {
        self.open = true;
        self.name = "Next review".into();
        self.selected = Some(name.into());
    }

    pub fn menu(&mut self, ui: &mut egui::Ui, lang: Language, enabled: bool) {
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(tr(lang, "register.presets.title")),
            )
            .clicked()
        {
            self.open = true;
            self.error = None;
            self.notice = None;
            ui.close();
        }
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        library: &RegisterPresetLibrary,
        lang: Language,
        dark: bool,
        has_icons: bool,
    ) -> Option<Action> {
        if !self.open {
            return None;
        }
        let mut action = None;
        let mut close = false;
        dialog::show(
            ctx,
            dialog::Spec::new(
                "register-presets",
                tr(lang, "register.presets.title"),
                dark,
                has_icons,
            )
            .width((ctx.content_rect().width() - 40.0).clamp(320.0, 500.0)),
            |ui| {
                ui.label(tr(lang, "register.presets.scope"));
                if let Some(error) = &self.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(notice) = &self.notice {
                    ui.label(notice);
                }
                ui.separator();
                let label = ui.label(tr(lang, "register.presets.name"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .char_limit(64)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !self.name.trim().is_empty()
                                && library.entries().len() < MAX_REGISTER_PRESETS,
                            egui::Button::new(tr(lang, "register.presets.save")),
                        )
                        .clicked()
                    {
                        action = Some(Action::Save(self.name.trim().into()));
                    }
                    ui.label(format!(
                        "{} / {MAX_REGISTER_PRESETS}",
                        library.entries().len()
                    ));
                });
                ui.separator();
                let saved_label = tr(lang, "register.presets.saved");
                ui.label(saved_label);
                let saved_response = egui::ComboBox::from_id_salt("register-preset-selection")
                    .selected_text(
                        self.selected
                            .as_deref()
                            .unwrap_or(tr(lang, "register.presets.choose")),
                    )
                    .width(ui.available_width().min(350.0))
                    .show_ui(ui, |ui| {
                        for preset in library.entries() {
                            ui.selectable_value(
                                &mut self.selected,
                                Some(preset.name.clone()),
                                &preset.name,
                            );
                        }
                    })
                    .response;
                let current = self
                    .selected
                    .clone()
                    .unwrap_or_else(|| tr(lang, "register.presets.choose").into());
                saved_response.widget_info(|| {
                    let mut info =
                        egui::WidgetInfo::labeled(egui::Role::ComboBox, true, saved_label);
                    info.current_text_value = Some(current.clone());
                    info
                });
                let selected = self.selected.as_deref().and_then(|name| library.get(name));
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            selected.is_some(),
                            egui::Button::new(tr(lang, "register.presets.load")),
                        )
                        .clicked()
                    {
                        action = selected.map(|preset| Action::Load(preset.name.clone()));
                    }
                    if ui
                        .add_enabled(
                            selected.is_some(),
                            egui::Button::new(tr(lang, "register.presets.delete")),
                        )
                        .clicked()
                    {
                        action = selected.map(|preset| Action::Delete(preset.name.clone()));
                    }
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr(lang, "register.presets.import")).clicked() {
                        action = Some(Action::Import);
                    }
                    if ui
                        .add_enabled(
                            !library.entries().is_empty(),
                            egui::Button::new(tr(lang, "register.presets.export")),
                        )
                        .clicked()
                    {
                        action = Some(Action::Export);
                    }
                });
                ui.small(tr(lang, "register.presets.import_help"));
            },
            |ui| {
                close = ui.button(tr(lang, "register.close")).clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::Escape));
            },
        );
        if close {
            self.open = false;
            return None;
        }
        action
    }
}

fn read_import(path: &Path) -> Result<Vec<u8>, EngineError> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_REGISTER_PRESET_JSON_BYTES as u64 {
        return Err(EngineError::Other("register.presets.file_limit".into()));
    }
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_REGISTER_PRESET_JSON_BYTES as u64 {
        return Err(EngineError::Other("register.presets.file_limit".into()));
    }
    let mut bytes = Vec::new();
    file.take((MAX_REGISTER_PRESET_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REGISTER_PRESET_JSON_BYTES {
        return Err(EngineError::Other("register.presets.file_limit".into()));
    }
    Ok(bytes)
}

fn commit(settings: &mut Settings, library: RegisterPresetLibrary) -> Result<(), EngineError> {
    settings
        .commit_register_presets(library, |next| {
            if Settings::path().is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "preferences directory is unavailable",
                ));
            }
            next.save()
        })
        .map_err(|_| EngineError::Other("register.presets.save_failed".into()))
}

pub(super) fn show(
    state: &mut RegisterState,
    settings: &mut Settings,
    ctx: &egui::Context,
    lang: Language,
    has_icons: bool,
) {
    let Some(action) = state.presets.show(
        ctx,
        &settings.register_presets,
        lang,
        settings.dark_theme,
        has_icons,
    ) else {
        return;
    };
    state.presets.error = None;
    state.presets.notice = None;
    let result = (|| -> Result<Option<&'static str>, EngineError> {
        match action {
            Action::Save(name) => {
                let preset =
                    RegisterPreset::capture(&name, &state.filter, state.sort, state.descending)?;
                let mut candidate = settings.register_presets.clone();
                candidate.insert(preset)?;
                commit(settings, candidate)?;
                state.presets.selected = Some(name);
                state.presets.name.clear();
                Ok(Some("register.presets.saved_notice"))
            }
            Action::Load(name) => {
                if let Some(preset) = settings.register_presets.get(&name) {
                    state.load_preset(preset)?;
                    Ok(Some("register.presets.loaded"))
                } else {
                    Err(EngineError::Other("register.presets.missing".into()))
                }
            }
            Action::Delete(name) => {
                let mut candidate = settings.register_presets.clone();
                if !candidate.remove(&name) {
                    return Err(EngineError::Other("register.presets.missing".into()));
                }
                commit(settings, candidate)?;
                state.presets.selected = None;
                Ok(Some("register.presets.deleted"))
            }
            Action::Import => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(lang, "register.presets.file_type"), &["json"])
                    .pick_file()
                else {
                    return Ok(None);
                };
                let bytes = read_import(&path)?;
                let mut candidate = settings.register_presets.clone();
                candidate.import_json(&bytes)?;
                commit(settings, candidate)?;
                Ok(Some("register.presets.imported"))
            }
            Action::Export => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(lang, "register.presets.file_type"), &["json"])
                    .set_file_name("register-filters.json")
                    .save_file()
                else {
                    return Ok(None);
                };
                let bytes = settings.register_presets.to_json()?;
                sunipdf_core::write_new_copy(&path, &bytes)?;
                Ok(Some("register.presets.exported"))
            }
        }
    })();
    match result {
        Ok(Some(key)) => state.presets.notice = Some(tr(lang, key).into()),
        Ok(None) => {}
        Err(error) => state.presets.error = Some(crate::app::error_text(lang, &error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::Request;
    use std::{
        collections::BTreeSet,
        sync::{atomic::Ordering, Arc},
    };
    use sunipdf_core::{
        annotation_register::{RegisterFilter, RegisterSort},
        plan_zones::{ZoneAssignmentStatus, ZoneFilter},
    };

    struct Controls {
        ctx: egui::Context,
        time: f64,
    }

    impl Controls {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            Self { ctx, time: 0.0 }
        }

        fn frame(
            &mut self,
            events: Vec<egui::Event>,
            draw: &mut impl FnMut(&egui::Context),
        ) -> egui::FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |_| draw(&self.ctx),
            );
            output.textures_delta.clear();
            output
        }

        fn click(&mut self, label: &str, draw: &mut impl FnMut(&egui::Context)) {
            fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                    _ => None,
                }
            }
            self.frame(vec![], draw);
            let mut point = None;
            for _ in 0..20 {
                let output = self.frame(vec![], draw);
                point = output.shapes.iter().rev().find_map(|shape| {
                    find(&shape.shape, label).filter(|point| shape.clip_rect.contains(*point))
                });
                if point.is_some() {
                    break;
                }
                self.frame(
                    vec![
                        egui::Event::PointerMoved(egui::pos2(360.0, 260.0)),
                        egui::Event::MouseWheel {
                            source: egui::MouseWheelSource::Wheel,
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, -120.0),
                            phase: egui::TouchPhase::Move,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    draw,
                );
            }
            let point = point.unwrap_or_else(|| panic!("visible control at 720x480: {label}"));
            assert!(
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 480.0))
                    .contains(point)
            );
            for pressed in [true, false] {
                self.frame(
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    draw,
                );
            }
        }
    }

    fn library() -> RegisterPresetLibrary {
        let mut library = RegisterPresetLibrary::default();
        library
            .insert(
                RegisterPreset::capture(
                    "Electrical",
                    &RegisterFilter {
                        discipline: "Electrical".into(),
                        custom_sort: Some("Room".into()),
                        ..Default::default()
                    },
                    RegisterSort::Author,
                    true,
                )
                .unwrap(),
            )
            .unwrap();
        library
    }

    #[test]
    fn localized_small_window_has_explicit_save_load_delete_import_export_controls() {
        for lang in [Language::English, Language::French] {
            let library = library();
            let mut draft = Draft {
                open: true,
                name: "Review".into(),
                ..Default::default()
            };
            let mut controls = Controls::new();
            let mut actions = Vec::new();
            let mut draw = |ctx: &egui::Context| {
                if let Some(action) = draft.show(ctx, &library, lang, false, false) {
                    actions.push(action);
                }
            };
            for _ in 0..3 {
                controls.frame(vec![], &mut draw);
            }
            controls.click(tr(lang, "register.presets.load"), &mut draw);
            controls.click(tr(lang, "register.presets.delete"), &mut draw);
            // No preset is implicitly selected, including the single-entry case.
            assert!(actions.is_empty());
            let mut draw = |ctx: &egui::Context| {
                if let Some(action) = draft.show(ctx, &library, lang, false, false) {
                    actions.push(action);
                }
            };
            controls.click(tr(lang, "register.presets.save"), &mut draw);
            controls.click(tr(lang, "register.presets.choose"), &mut draw);
            controls.click("Electrical", &mut draw);
            controls.click(tr(lang, "register.presets.load"), &mut draw);
            controls.click(tr(lang, "register.presets.delete"), &mut draw);
            controls.click(tr(lang, "register.presets.import"), &mut draw);
            controls.click(tr(lang, "register.presets.export"), &mut draw);
            controls.click(tr(lang, "register.close"), &mut draw);
            assert_eq!(
                actions,
                vec![
                    Action::Save("Review".into()),
                    Action::Load("Electrical".into()),
                    Action::Delete("Electrical".into()),
                    Action::Import,
                    Action::Export
                ]
            );
            assert!(!draft.open);
        }
    }

    #[test]
    fn filters_menu_opens_saved_filters_without_selecting_or_editing_annotations() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            app.settings.tables.placement = crate::workspace_table::Placement::Floating;
            app.settings.register_presets = library();
            app.annotate.register.open = true;
            let mut report = (*super::super::tests::large_register()).clone();
            report.rows.truncate(2);
            app.annotate.register.report = Some(Arc::new(report));
            let lang = app.lang();
            let mut controls = Controls::new();
            controls.click(tr(lang, "workspace.table.filters"), &mut |ctx| {
                app.annotation_register_dialog(ctx)
            });
            controls.click(tr(lang, "register.presets.title"), &mut |ctx| {
                app.annotation_register_dialog(ctx)
            });
            assert!(app.annotate.register.presets.open);
            assert!(app.annotate.register.dialog_open());
            controls.click(tr(lang, "register.presets.choose"), &mut |ctx| {
                app.annotation_register_dialog(ctx)
            });
            controls.click("Electrical", &mut |ctx| app.annotation_register_dialog(ctx));
            assert!(
                app.annotate.register.filter.discipline.is_empty(),
                "choosing a name alone does not load it"
            );
            controls.click(tr(lang, "register.presets.load"), &mut |ctx| {
                app.annotation_register_dialog(ctx)
            });
            assert_eq!(app.annotate.register.filter.discipline, "Electrical");
            assert_eq!(
                app.annotate.register.filter.custom_sort.as_deref(),
                Some("Room")
            );
            assert!(app.annotate.register.bulk_selected.is_empty());
            controls.click(tr(lang, "register.close"), &mut |ctx| {
                app.annotation_register_dialog(ctx)
            });
            assert!(!app.annotate.register.presets.open);
            assert!(
                requests.try_recv().is_err(),
                "saved filters do not read or edit the PDF"
            );
        }
    }

    #[test]
    fn loading_identical_criteria_preserves_scope_but_cancels_bulk_preview_and_selection() {
        let (app, requests, _) = crate::app::tests::test_app();
        let mut state = RegisterState::default();
        state.filter.page = Some(3);
        state.page = 3;
        state.filter.zone = Some(ZoneFilter {
            page: 3,
            id: "room-301".into(),
            descendants: true,
        });
        state.filter.zone_status = Some(ZoneAssignmentStatus::Assigned);
        state.filter.discipline = "Electrical".into();
        state.filter.custom_sort = Some("Room".into());
        state.sort = RegisterSort::Author;
        state.descending = true;
        state.sync_bulk_filter();
        let original_filter = state.filter.clone();
        state.offset = 50;
        state.bulk_selected = BTreeSet::from([(1, 0), (1, 1)]);
        let mut report = (*super::super::tests::large_register()).clone();
        report.rows.truncate(2);
        let rows = report.rows.clone();
        state.bulk.bind(Arc::new(report), rows, app.view.epoch);
        let mut controls = Controls::new();
        controls.click(tr(app.lang(), "register.bulk.preview"), &mut |ctx| {
            state
                .bulk
                .show(ctx, &app.worker, app.view.epoch, app.lang(), false, false);
        });
        let job = match requests.try_recv().unwrap() {
            Request::PreviewRegisterBatch { job, .. } => job,
            _ => panic!("actual bulk preview request"),
        };
        assert!(!job.cancel.load(Ordering::Acquire));
        let preset =
            RegisterPreset::capture("Current", &state.filter, state.sort, state.descending)
                .unwrap();
        state.load_preset(&preset).unwrap();
        assert_eq!(state.filter, original_filter);
        assert_eq!(state.page, 3);
        assert_eq!(state.offset, 0);
        assert_eq!(state.sort, RegisterSort::Author);
        assert!(state.descending && state.bulk_selected.is_empty());
        assert!(!state.bulk.open);
        assert!(job.cancel.load(Ordering::Acquire));
        assert!(job
            .reply
            .send(Err(EngineError::Other("late preview".into())))
            .is_err());
        assert!(
            requests.try_recv().is_err(),
            "loading criteria never edits annotations"
        );
        let mut invalid = preset;
        invalid.name.clear();
        state.bulk_selected.insert((1, 0));
        assert!(state.load_preset(&invalid).is_err());
        assert_eq!(state.filter, original_filter);
        assert_eq!(state.bulk_selected, BTreeSet::from([(1, 0)]));
    }

    #[test]
    fn import_is_bounded_and_atomic_and_export_cannot_overwrite() {
        let dir =
            std::env::temp_dir().join(format!("sunipdf-filter-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("filters.json");
        let original = library();
        let bytes = original.to_json().unwrap();
        sunipdf_core::write_new_copy(&path, &bytes).unwrap();
        let read = read_import(&path).unwrap();
        let mut imported = RegisterPresetLibrary::default();
        assert_eq!(imported.import_json(&read).unwrap(), 1);
        assert_eq!(imported, original);
        assert!(imported.import_json(&read).is_err());
        assert_eq!(imported, original);
        assert!(imported.import_json(b"{\"version\":2}").is_err());
        assert_eq!(imported, original);
        assert!(sunipdf_core::write_new_copy(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let large = dir.join("large.json");
        std::fs::write(&large, vec![b' '; MAX_REGISTER_PRESET_JSON_BYTES + 1]).unwrap();
        assert!(read_import(&large).is_err());
        assert!(read_import(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
