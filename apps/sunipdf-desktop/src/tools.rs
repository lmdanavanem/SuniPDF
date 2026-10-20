//! Outils de fichiers : combiner, diviser, extraire, créer depuis des images, exporter en images,
//! optimiser. Les tâches tournent sur le thread moteur ; l'UI reçoit un `BatchDone`.

use crate::app::SuniApp;
use crate::worker::{BatchOp, BatchResult, EditOp, Request};
use egui::Key;
use std::path::{Path, PathBuf};
use sunipdf_core::{Stamp, StampPosition, TextStyle};
use sunipdf_i18n::Language;
use sunipdf_i18n::{tr, tr_args};

#[cfg(test)]
mod watermark_tests {
    use super::*;

    #[test]
    fn choosing_a_preset_then_cancelling_keeps_preferences_and_document_unchanged() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.tools.stamp.text = "Draft".into();
        let data =
            crate::stamp_presets::PresetData::capture(&app.tools.stamp, StampKind::Watermark)
                .unwrap();
        app.settings
            .stamp_presets
            .insert("Draft", data.clone())
            .unwrap();
        let before = app.settings.clone();
        let ctx = egui::Context::default();
        for language in Language::ALL {
            app.settings.set_language(language);
            let expected = app.settings.clone();
            data.apply(&mut app.tools.stamp);
            app.tools.preset_selected = Some("Draft".into());
            app.tools.stamp_open = Some(StampKind::Watermark);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| app.stamp_dialog(ui.ctx()),
            );
            out.textures_delta.clear();
            assert!(requests.try_recv().is_err() && app.tools.stamp_open.is_none());
            assert_eq!(app.settings, expected);
        }
        assert_eq!(app.settings.stamp_presets, before.stamp_presets);
    }

    #[test]
    fn header_mode_validates_its_own_draft_independently_of_page_numbers() {
        let mut form = StampForm {
            header_mode: true,
            ..Default::default()
        };
        assert!(!form.ready(StampKind::PageNumbers));
        form.headers.zones[4] = "{n} / {total}".into();
        assert!(form.ready(StampKind::PageNumbers));
        form.headers.zones[0] = "bad\nline".into();
        assert!(!form.ready(StampKind::PageNumbers));
        form.header_mode = false;
        assert!(form.ready(StampKind::PageNumbers));
    }

    #[test]
    fn header_footer_dialog_captures_six_zones_and_escape_never_edits() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        for language in Language::ALL {
            app.settings.set_language(language);
            app.tools.stamp_open = Some(StampKind::PageNumbers);
            app.tools.stamp.header_mode = true;
            app.tools.stamp.headers.zones[0] = "Report".into();
            app.tools.stamp.headers.zones[4] = "{n} / {total}".into();
            for key in [Key::Escape, Key::Enter] {
                app.tools.stamp_open = Some(StampKind::PageNumbers);
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        events: vec![
                            egui::Event::ModifiersChanged(if key == Key::Enter {
                                egui::Modifiers::COMMAND
                            } else {
                                egui::Modifiers::NONE
                            }),
                            egui::Event::Key {
                                key,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: if key == Key::Enter {
                                    egui::Modifiers::COMMAND
                                } else {
                                    egui::Modifiers::NONE
                                },
                            },
                        ],
                        ..Default::default()
                    },
                    |ui| app.stamp_dialog(ui.ctx()),
                );
                out.textures_delta.clear();
                assert!(app.tools.stamp_open.is_none());
                if key == Key::Escape {
                    assert!(requests.try_recv().is_err());
                } else {
                    assert!(
                        matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::HeadersFooters { pages: None, options }) if options.zones[0]=="Report" && options.zones[4]=="{n} / {total}")
                    );
                    assert!(matches!(
                        requests.try_recv().unwrap(),
                        Request::FinishMutation
                    ));
                    app.worker.finish_mutation();
                }
            }
        }
    }

    #[test]
    fn watermark_mode_changes_cannot_submit_an_empty_or_invalid_image_draft() {
        let mut form = StampForm {
            text: "Previous text".into(),
            ..Default::default()
        };
        assert!(form.ready(StampKind::Watermark));
        form.image_mode = true;
        assert!(!form.ready(StampKind::Watermark));
        form.image_path = Some(PathBuf::from("logo.png"));
        assert!(form.ready(StampKind::Watermark));
        form.image_width_percent = f32::NAN;
        assert!(!form.ready(StampKind::Watermark));
        assert!(form.ready(StampKind::PageNumbers));
    }

    #[test]
    fn image_watermark_dialog_submits_captured_options_and_escape_never_edits() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.tools.stamp_open = Some(StampKind::Watermark);
        app.tools.stamp.image_mode = true;
        app.tools.stamp.image_path = Some(PathBuf::from("selected-logo.png"));
        app.tools.stamp.image_width_percent = 42.0;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(egui::Modifiers::COMMAND),
                    egui::Event::Key {
                        key: Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::COMMAND,
                    },
                ],
                ..Default::default()
            },
            |ui| app.stamp_dialog(ui.ctx()),
        );
        out.textures_delta.clear();
        assert!(
            matches!(requests.try_recv().unwrap(),Request::Edit(EditOp::ImageWatermark{pages:None,path,options}) if path==Path::new("selected-logo.png") && options.width_percent==42.0)
        );
        assert!(app.tools.stamp_open.is_none());
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        for language in Language::ALL {
            app.settings.set_language(language);
            app.tools.stamp_open = Some(StampKind::Watermark);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| app.stamp_dialog(ui.ctx()),
            );
            out.textures_delta.clear();
            assert!(app.tools.stamp_open.is_none());
            assert!(requests.try_recv().is_err());
        }
    }
}

#[derive(Default)]
pub struct ToolsState {
    /// Dialogue « Diviser » : taille des lots.
    pub split_open: bool,
    pub sanitize_open: bool,
    pub split_every: usize,
    pub running: Option<&'static str>,
    pub stamp_open: Option<StampKind>,
    pub stamp: StampForm,
    pub preset_name: String,
    pub preset_selected: Option<String>,
    pub preset_error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StampKind {
    Watermark,
    PageNumbers,
}

/// Valeurs saisies pour le tampon et les numéros de page (conservées entre deux ouvertures).
pub struct StampForm {
    pub text: String,
    pub image_mode: bool,
    pub image_path: Option<PathBuf>,
    pub image_width_percent: f32,
    pub image_margin: f32,
    pub size: f32,
    pub rgb: [u8; 3],
    pub opacity: f32,
    pub angle: f32,
    pub bold: bool,
    pub position: StampPosition,
    pub all_pages: bool,
    pub numbers_template: String,
    pub numbers_position: StampPosition,
    pub numbers_size: f32,
    pub header_mode: bool,
    pub headers: sunipdf_core::header_footer::HeaderFooter,
}

impl Default for StampForm {
    fn default() -> Self {
        Self {
            text: String::new(),
            image_mode: false,
            image_path: None,
            image_width_percent: 30.0,
            image_margin: 28.0,
            size: 48.0,
            rgb: [0x80, 0x80, 0x80],
            opacity: 0.3,
            angle: 45.0,
            bold: true,
            position: StampPosition::Center,
            all_pages: true,
            numbers_template: "{n} / {total}".into(),
            numbers_position: StampPosition::BottomCenter,
            numbers_size: 10.0,
            header_mode: false,
            headers: Default::default(),
        }
    }
}

impl StampForm {
    fn ready(&self, kind: StampKind) -> bool {
        if kind == StampKind::PageNumbers && self.header_mode {
            return self.headers.validate().is_ok();
        }
        if kind == StampKind::PageNumbers {
            return !self.numbers_template.trim().is_empty();
        }
        if self.image_mode {
            self.image_path.is_some()
                && self.image_width_percent.is_finite()
                && (1.0..=100.0).contains(&self.image_width_percent)
                && self.opacity.is_finite()
                && (0.01..=1.0).contains(&self.opacity)
                && self.angle.is_finite()
                && self.image_margin.is_finite()
                && self.image_margin >= 0.0
        } else {
            !self.text.trim().is_empty()
        }
    }
}

fn position_combo(ui: &mut egui::Ui, lang: Language, id: &str, value: &mut StampPosition) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(tr(lang, &format!("position.{}", value.id())))
        // Sept positions de 28 px : la hauteur par défaut (200 px) en cachait une sans ascenseur.
        .height(320.0)
        .show_ui(ui, |ui| {
            for p in StampPosition::ALL {
                ui.selectable_value(value, p, tr(lang, &format!("position.{}", p.id())));
            }
        });
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl SuniApp {
    fn pdf_filter(&self) -> rfd::FileDialog {
        rfd::FileDialog::new().add_filter(self.t("dialog.open.filter_pdf"), &["pdf"])
    }

    /// Nom proposé : `<nom>-<suffixe traduit>.pdf` (`suffix_key` = `tools.suffix.*`).
    fn suggest_output(&self, suffix_key: &str) -> PathBuf {
        let suffix = self.t(suffix_key);
        match &self.document {
            Some(doc) => {
                let stem = doc
                    .info
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "document".into());
                doc.info.path.with_file_name(format!("{stem}-{suffix}.pdf"))
            }
            None => PathBuf::from(format!("sunipdf-{suffix}.pdf")),
        }
    }

    fn save_dialog(&self, title_key: &str, suggested: &Path) -> Option<PathBuf> {
        let mut dialog = self
            .pdf_filter()
            .set_title(self.t(title_key))
            .set_file_name(file_name(suggested));
        if let Some(dir) = suggested.parent().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let mut path = dialog.save_file()?;
        if path.extension().is_none() {
            path.set_extension("pdf");
        }
        Some(path)
    }

    pub fn run_batch(&mut self, op: BatchOp, running_key: &'static str) {
        self.tools.running = Some(running_key);
        self.worker.send(Request::Batch(op));
    }

    pub fn tool_combine(&mut self) {
        let inputs = self
            .pdf_filter()
            .set_title(self.t("tools.combine.pick"))
            .pick_files()
            .unwrap_or_default();
        if inputs.is_empty() {
            return;
        }
        let Some(out) = self.save_dialog(
            "tools.combine.save",
            &self.suggest_output("tools.suffix.combine"),
        ) else {
            return;
        };
        self.run_batch(BatchOp::Combine { inputs, out }, "tools.running.combine");
    }

    pub fn stamp_dialog(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.tools.stamp_open else {
            return;
        };
        let lang = self.lang();
        let muted = crate::theme::palette(self.settings.dark_theme).text_muted;
        let current_page = self.document.as_ref().map(|d| d.page);
        let mut submit = false;
        let mut cancel = false;
        let numbers = kind == StampKind::PageNumbers;
        let spec = crate::dialog::Spec::new(
            "stamp",
            tr(
                lang,
                if numbers {
                    "numbers.title"
                } else {
                    "stamp.title"
                },
            ),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(
            lang,
            if numbers {
                "numbers.subtitle"
            } else {
                "stamp.subtitle"
            },
        ))
        .icon(
            if numbers {
                crate::icons::glyph::NUMBERS
            } else {
                crate::icons::glyph::STAMP
            },
            "#",
        )
        .width(520.0);
        let presets = self
            .settings
            .stamp_presets
            .entries()
            .iter()
            .filter(|p| p.data.matches(kind))
            .cloned()
            .collect::<Vec<_>>();
        if !presets
            .iter()
            .any(|p| Some(&p.name) == self.tools.preset_selected.as_ref())
        {
            self.tools.preset_selected = None;
        }
        let preset_error = self.tools.preset_error.clone();
        let selected = &mut self.tools.preset_selected;
        let preset_name = &mut self.tools.preset_name;
        let mut save_preset = false;
        let mut delete_preset = false;
        let form = &mut self.tools.stamp;
        let ready = std::cell::Cell::new(form.ready(kind));
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(tr(lang, "presets.scope"));
                if let Some(error) = &preset_error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                ui.horizontal_wrapped(|ui| {
                    let previous = selected.clone();
                    egui::ComboBox::from_id_salt("decoration-presets")
                        .selected_text(selected.as_deref().unwrap_or(tr(lang, "presets.choose")))
                        .show_ui(ui, |ui| {
                            for preset in &presets {
                                ui.selectable_value(
                                    selected,
                                    Some(preset.name.clone()),
                                    &preset.name,
                                );
                            }
                        });
                    if *selected != previous {
                        if let Some(preset) =
                            presets.iter().find(|p| Some(&p.name) == selected.as_ref())
                        {
                            if preset.data.apply(form) {
                                *preset_name = preset.name.clone();
                            }
                        }
                    }
                    ui.add(
                        egui::TextEdit::singleline(preset_name)
                            .hint_text(tr(lang, "presets.name"))
                            .char_limit(64)
                            .desired_width(145.0),
                    );
                    if ui
                        .add_enabled(
                            !preset_name.trim().is_empty()
                                && crate::stamp_presets::PresetData::capture(form, kind).is_some(),
                            egui::Button::new(tr(lang, "presets.save")),
                        )
                        .clicked()
                    {
                        save_preset = true;
                    }
                    if ui
                        .add_enabled(
                            selected.is_some(),
                            egui::Button::new(tr(lang, "presets.delete")),
                        )
                        .clicked()
                    {
                        delete_preset = true;
                    }
                });
                ui.separator();
                if numbers {
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(&mut form.header_mode, false, tr(lang, "headers.numbers"));
                        ui.radio_value(&mut form.header_mode, true, tr(lang, "headers.title"));
                    });
                }
                if numbers && form.header_mode {
                    ui.label(tr(lang, "headers.scope"));
                    egui::Grid::new("header-footer-zones")
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (index, key) in [
                                "headers.top_left",
                                "headers.top_center",
                                "headers.top_right",
                                "headers.bottom_left",
                                "headers.bottom_center",
                                "headers.bottom_right",
                            ]
                            .iter()
                            .enumerate()
                            {
                                ui.label(tr(lang, key));
                                ui.add(
                                    egui::TextEdit::singleline(&mut form.headers.zones[index])
                                        .char_limit(256)
                                        .desired_width(300.0),
                                );
                                ui.end_row();
                            }
                        });
                    ui.label(tr(lang, "stamp.placeholders"));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "stamp.size"));
                        ui.add(
                            egui::DragValue::new(&mut form.headers.style.size_pt)
                                .range(4.0..=72.0)
                                .suffix(" pt"),
                        );
                        ui.label(tr(lang, "watermark.margin"));
                        ui.add(
                            egui::DragValue::new(&mut form.headers.margin_pt)
                                .range(0.0..=1000.0)
                                .suffix(" pt"),
                        );
                    });
                } else if numbers {
                    crate::dialog::field_label(ui, tr(lang, "numbers.template"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut form.numbers_template)
                                .desired_width(200.0),
                        );
                        for (key, template) in [
                            ("numbers.preset.simple", "{n}"),
                            ("numbers.preset.total", "{n} / {total}"),
                            ("numbers.preset.page", "Page {n}"),
                        ] {
                            if ui.small_button(tr(lang, key)).clicked() {
                                form.numbers_template = template.into();
                            }
                        }
                    });
                    ui.label(
                        egui::RichText::new(tr(lang, "stamp.placeholders"))
                            .size(12.0)
                            .color(muted),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "stamp.position"));
                        position_combo(ui, lang, "numbers-pos", &mut form.numbers_position);
                        ui.label(tr(lang, "stamp.size"));
                        ui.add(
                            egui::DragValue::new(&mut form.numbers_size)
                                .range(4.0..=72.0)
                                .speed(0.5),
                        );
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut form.image_mode, false, tr(lang, "watermark.text"));
                        ui.radio_value(&mut form.image_mode, true, tr(lang, "watermark.image"));
                    });
                    if form.image_mode {
                        if ui.button(tr(lang, "watermark.choose_image")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Images",
                                    &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"],
                                )
                                .pick_file()
                            {
                                form.image_path = Some(path);
                            }
                        }
                        if let Some(path) = &form.image_path {
                            ui.label(file_name(path));
                        }
                        ui.label(tr(lang, "watermark.image_scope"));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr(lang, "watermark.width_percent"));
                            ui.add(
                                egui::DragValue::new(&mut form.image_width_percent)
                                    .range(1.0..=100.0)
                                    .suffix(" %"),
                            );
                            ui.label(tr(lang, "stamp.angle"));
                            ui.add(egui::DragValue::new(&mut form.angle).range(-180.0..=180.0));
                            ui.label(tr(lang, "watermark.margin"));
                            ui.add(
                                egui::DragValue::new(&mut form.image_margin)
                                    .range(0.0..=1000.0)
                                    .suffix(" pt"),
                            );
                        });
                    } else {
                        crate::dialog::field_label(ui, tr(lang, "stamp.text"));
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut form.text)
                                .hint_text(tr(lang, "stamp.text_placeholder"))
                                .desired_width(f32::INFINITY),
                        );
                        if ui.memory(|m| m.focused().is_none()) {
                            field.request_focus();
                        }
                        ui.label(
                            egui::RichText::new(tr(lang, "stamp.placeholders"))
                                .size(12.0)
                                .color(muted),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(tr(lang, "stamp.size"));
                            ui.add(
                                egui::DragValue::new(&mut form.size)
                                    .range(6.0..=300.0)
                                    .speed(1.0),
                            );
                            ui.label(tr(lang, "stamp.color"));
                            ui.color_edit_button_srgb(&mut form.rgb);
                            ui.checkbox(&mut form.bold, tr(lang, "stamp.bold"));
                            ui.label(tr(lang, "stamp.angle"));
                            ui.add(
                                egui::DragValue::new(&mut form.angle)
                                    .range(-90.0..=90.0)
                                    .speed(1.0),
                            );
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "stamp.opacity"));
                        ui.scope(|ui| {
                            // Rail visible : le fond « inactif » du thème est la couleur de surface.
                            let border = crate::theme::palette(ui.visuals().dark_mode).border;
                            ui.visuals_mut().widgets.inactive.bg_fill = border;
                            ui.spacing_mut().slider_width = 140.0;
                            ui.add(
                                egui::Slider::new(&mut form.opacity, 0.05..=1.0)
                                    .show_value(true)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.label(tr(lang, "stamp.position"));
                        position_combo(ui, lang, "stamp-pos", &mut form.position);
                    });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.radio_value(&mut form.all_pages, true, tr(lang, "stamp.scope.all"));
                    ui.radio_value(&mut form.all_pages, false, tr(lang, "stamp.scope.current"));
                });
                ready.set(form.ready(kind));
            },
            |ui| {
                ui.add_enabled_ui(ready.get(), |ui| {
                    if crate::dialog::primary(
                        ui,
                        tr(lang, if numbers { "numbers.ok" } else { "stamp.ok" }),
                    ) {
                        submit = true;
                    }
                });
                if ready.get() && ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter)) {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if !cancel && (save_preset || delete_preset) {
            let mut candidate = self.settings.stamp_presets.clone();
            let result = if save_preset {
                crate::stamp_presets::PresetData::capture(&self.tools.stamp, kind)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "invalid decoration draft",
                        )
                    })
                    .and_then(|data| candidate.insert(&self.tools.preset_name, data))
            } else if self
                .tools
                .preset_selected
                .as_deref()
                .is_some_and(|name| candidate.remove(name))
            {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "preset no longer exists",
                ))
            };
            let result = result.and_then(|()| {
                self.settings.commit_stamp_presets(candidate, |next| {
                    if crate::settings::Settings::path().is_none() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "user settings directory unavailable",
                        ));
                    }
                    next.save()
                })
            });
            match result {
                Ok(()) => {
                    self.tools.preset_error = None;
                    if save_preset {
                        self.tools.preset_selected =
                            Some(self.tools.preset_name.trim().to_string());
                    } else {
                        self.tools.preset_selected = None;
                        self.tools.preset_name.clear();
                    }
                }
                Err(error) => {
                    self.tools.preset_error = Some(tr_args(
                        lang,
                        "presets.error",
                        &[("error", &error.to_string())],
                    ))
                }
            }
        }
        if submit && !cancel && self.tools.stamp.ready(kind) {
            let form = &self.tools.stamp;
            if numbers && form.header_mode {
                let pages = if form.all_pages {
                    None
                } else {
                    current_page.map(|p| vec![p])
                };
                let options = form.headers.clone();
                self.tools.stamp_open = None;
                self.worker
                    .send(Request::Edit(EditOp::HeadersFooters { pages, options }));
                return;
            }
            if !numbers && form.image_mode {
                let pages = if form.all_pages {
                    None
                } else {
                    current_page.map(|p| vec![p])
                };
                let options = sunipdf_core::image_watermark::ImageWatermark {
                    width_percent: form.image_width_percent,
                    opacity: form.opacity,
                    angle: form.angle,
                    position: form.position,
                    margin_pt: form.image_margin,
                };
                let Some(path) = form.image_path.clone() else {
                    return;
                };
                self.tools.stamp_open = None;
                self.worker.send(Request::Edit(EditOp::ImageWatermark {
                    pages,
                    path,
                    options,
                }));
                return;
            }
            let stamp = if numbers {
                Stamp {
                    style: TextStyle {
                        size_pt: form.numbers_size,
                        ..Default::default()
                    },
                    ..Stamp::page_numbers(&form.numbers_template, form.numbers_position)
                }
            } else {
                Stamp {
                    text: form.text.clone(),
                    style: TextStyle {
                        size_pt: form.size,
                        bold: form.bold,
                        rgb: (form.rgb[0], form.rgb[1], form.rgb[2]),
                        opacity: form.opacity,
                        ..Default::default()
                    },
                    angle: form.angle,
                    position: form.position,
                    margin_pt: 28.0,
                }
            };
            if stamp.text.trim().is_empty() {
                return;
            }
            let pages = if form.all_pages {
                None
            } else {
                current_page.map(|p| vec![p])
            };
            self.tools.stamp_open = None;
            self.worker
                .send(Request::Edit(EditOp::Stamp { pages, stamp }));
        } else if cancel {
            self.tools.stamp_open = None;
        }
    }

    pub fn tool_split_submit(&mut self) {
        let every = self.tools.split_every.max(1);
        let Some(dir) = rfd::FileDialog::new()
            .set_title(self.t("tools.split.pick_dir"))
            .pick_folder()
        else {
            return;
        };
        self.tools.split_open = false;
        self.run_batch(
            BatchOp::Split {
                every,
                out_dir: dir,
            },
            "tools.running.split",
        );
    }

    pub fn tool_extract(&mut self) {
        let pages = self.target_pages();
        if pages.is_empty() {
            return;
        }
        let Some(out) = self.save_dialog(
            "tools.extract.save",
            &self.suggest_output("tools.suffix.extract"),
        ) else {
            return;
        };
        self.run_batch(BatchOp::Extract { pages, out }, "tools.running.extract");
    }

    pub fn tool_create_from_images(&mut self) {
        let images = rfd::FileDialog::new()
            .set_title(self.t("tools.images.pick"))
            .add_filter(
                self.t("tools.images.filter"),
                &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"],
            )
            .pick_files()
            .unwrap_or_default();
        if images.is_empty() {
            return;
        }
        let suggested = images[0].with_extension("pdf");
        let Some(out) = self.save_dialog("tools.images.save", &suggested) else {
            return;
        };
        self.run_batch(
            BatchOp::CreateFromImages { images, out },
            "tools.running.images",
        );
    }

    pub fn tool_export_images(&mut self) {
        let Some(dir) = rfd::FileDialog::new()
            .set_title(self.t("tools.export.pick_dir"))
            .pick_folder()
        else {
            return;
        };
        self.run_batch(
            BatchOp::ExportImages {
                out_dir: dir,
                dpi: 150.0,
            },
            "tools.running.export",
        );
    }

    pub fn tool_optimize(&mut self) {
        let Some(out) = self.save_dialog(
            "tools.optimize.save",
            &self.suggest_output("tools.suffix.optimize"),
        ) else {
            return;
        };
        self.run_batch(BatchOp::Optimize { out }, "tools.running.optimize");
    }

    pub fn split_dialog(&mut self, ctx: &egui::Context) {
        self.sanitize_dialog(ctx);
        if !self.tools.split_open {
            return;
        }
        let lang = self.lang();
        let count = self.document.as_ref().map_or(0, |d| d.info.page_count());
        let mut submit = false;
        let mut cancel = false;
        let spec = crate::dialog::Spec::new(
            "split",
            tr(lang, "tools.split.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "tools.split.subtitle"))
        .icon(crate::icons::glyph::SPLIT, "⫽")
        .width(440.0);
        let every = &mut self.tools.split_every;
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(tr_args(
                    lang,
                    "tools.split.text",
                    &[("total", &count.to_string())],
                ));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "tools.split.every"));
                    ui.add(egui::DragValue::new(every).range(1..=count.max(1)));
                });
                let parts = count.div_ceil((*every).max(1));
                ui.label(
                    egui::RichText::new(tr_args(
                        lang,
                        "tools.split.parts",
                        &[("count", &parts.to_string())],
                    ))
                    .color(crate::theme::palette(ui.visuals().dark_mode).text_muted),
                );
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "tools.split.ok"))
                    || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        if submit {
            self.tool_split_submit();
        } else if cancel {
            self.tools.split_open = false;
        }
    }

    /// Fin d'une tâche : message d'état, ouverture éventuelle du résultat.
    pub fn on_batch_done(&mut self, result: BatchResult) {
        self.tools.running = None;
        let lang = self.lang();
        match result {
            BatchResult::Combined { out, pages } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.combine",
                    &[("pages", &pages.to_string()), ("name", &file_name(&out))],
                ));
                self.open_path(out, None);
            }
            BatchResult::Split { files, encrypted } => {
                let mut msg = tr_args(
                    lang,
                    "tools.done.split",
                    &[("count", &files.len().to_string())],
                );
                if encrypted {
                    msg = format!("{msg} {}", tr(lang, "tools.done.encryption_preserved"));
                }
                self.status_msg = Some(msg);
            }
            BatchResult::Extracted {
                out,
                pages,
                encrypted,
            } => {
                let mut msg = tr_args(
                    lang,
                    "tools.done.extract",
                    &[("pages", &pages.to_string()), ("name", &file_name(&out))],
                );
                if encrypted {
                    msg = format!("{msg} {}", tr(lang, "tools.done.encryption_preserved"));
                }
                self.status_msg = Some(msg);
            }
            BatchResult::Created { out, pages } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.images",
                    &[("pages", &pages.to_string()), ("name", &file_name(&out))],
                ));
                self.open_path(out, None);
            }
            BatchResult::Exported { files } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.export",
                    &[("count", &files.len().to_string())],
                ));
            }
            BatchResult::Optimized { out, before, after } => {
                let percent = if before > 0 {
                    100.0 * (1.0 - after as f64 / before as f64)
                } else {
                    0.0
                };
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.optimize",
                    &[
                        ("name", &file_name(&out)),
                        ("before", &crate::docinfo::human_size(lang, before)),
                        ("after", &crate::docinfo::human_size(lang, after)),
                        ("percent", &format!("{percent:.0}")),
                    ],
                ));
            }
            BatchResult::Sanitized { out, report } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "sanitize.done",
                    &[
                        ("name", &file_name(&out)),
                        (
                            "entries",
                            &(report.removed_entries
                                + report.removed_annotations
                                + report.unused_font_entries)
                                .to_string(),
                        ),
                    ],
                ));
            }
            BatchResult::FormExported { out, count } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.form_export",
                    &[("name", &file_name(&out)), ("count", &count.to_string())],
                ));
            }
            BatchResult::Failed(e) => {
                log::warn!("tâche échouée : {e}");
                self.error = Some(crate::app::error_key(&e));
            }
        }
    }
}

impl SuniApp {
    fn sanitize_dialog(&mut self, ctx: &egui::Context) {
        if !self.tools.sanitize_open {
            return;
        }
        let lang = self.lang();
        let mut submit = false;
        let mut cancel = false;
        let spec = crate::dialog::Spec::new(
            "sanitize",
            tr(lang, "menu.security.sanitize"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "sanitize.subtitle"))
        .width(500.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(tr(lang, "sanitize.scope"));
                ui.add_space(8.0);
                ui.label(tr(lang, "sanitize.warning"));
                ui.add_space(8.0);
                ui.label(tr(lang, "sanitize.limits"));
            },
            |ui| {
                submit = crate::dialog::primary(ui, tr(lang, "sanitize.save"));
                cancel = crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|input| input.key_pressed(Key::Escape));
            },
        );
        if cancel {
            self.tools.sanitize_open = false;
        }
        if submit {
            self.tools.sanitize_open = false;
            if let Some(out) =
                self.save_dialog("sanitize.save", &self.suggest_output("sanitize.suffix"))
            {
                self.run_batch(BatchOp::Sanitize { out }, "sanitize.running");
            }
        }
    }
}
