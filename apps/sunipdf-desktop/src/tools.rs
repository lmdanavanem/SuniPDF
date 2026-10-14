//! Outils de fichiers : combiner, diviser, extraire, créer depuis des images, exporter en images,
//! optimiser. Les tâches tournent sur le thread moteur ; l'UI reçoit un `BatchDone`.

use crate::app::SuniApp;
use crate::worker::{BatchOp, BatchResult, EditOp, Request};
use egui::Key;
use std::path::{Path, PathBuf};
use sunipdf_core::{Stamp, StampPosition, TextStyle};
use sunipdf_i18n::Language;
use sunipdf_i18n::{tr, tr_args};

#[derive(Default)]
pub struct ToolsState {
    /// Dialogue « Diviser » : taille des lots.
    pub split_open: bool,
    pub split_every: usize,
    pub running: Option<&'static str>,
    pub stamp_open: Option<StampKind>,
    pub stamp: StampForm,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StampKind {
    Watermark,
    PageNumbers,
}

/// Valeurs saisies pour le tampon et les numéros de page (conservées entre deux ouvertures).
pub struct StampForm {
    pub text: String,
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
}

impl Default for StampForm {
    fn default() -> Self {
        Self {
            text: String::new(),
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
        }
    }
}

fn position_combo(ui: &mut egui::Ui, lang: Language, id: &str, value: &mut StampPosition) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(tr(lang, &format!("position.{}", value.id())))
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

    fn run_batch(&mut self, op: BatchOp, running_key: &'static str) {
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
        let form = &mut self.tools.stamp;
        // Sans texte, rien à apposer : bouton grisé plutôt qu'un clic sans effet (le
        // « CONFIDENTIEL » du champ n'est qu'une suggestion). Valeur de l'image précédente.
        let has_text = !if numbers {
            form.numbers_template.trim().is_empty()
        } else {
            form.text.trim().is_empty()
        };
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                if numbers {
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
            },
            |ui| {
                ui.add_enabled_ui(has_text, |ui| {
                    if crate::dialog::primary(
                        ui,
                        tr(lang, if numbers { "numbers.ok" } else { "stamp.ok" }),
                    ) {
                        submit = true;
                    }
                });
                if has_text && ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter)) {
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
            let form = &self.tools.stamp;
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
            BatchResult::Split {
                files,
                encryption_removed,
            } => {
                let mut msg = tr_args(
                    lang,
                    "tools.done.split",
                    &[("count", &files.len().to_string())],
                );
                if encryption_removed {
                    // D046 : les parties d'un document protégé ne le sont pas, dit clairement.
                    msg = format!("{msg} {}", tr(lang, "tools.done.encryption_removed"));
                }
                self.status_msg = Some(msg);
            }
            BatchResult::Extracted {
                out,
                pages,
                encryption_removed,
            } => {
                let mut msg = tr_args(
                    lang,
                    "tools.done.extract",
                    &[("pages", &pages.to_string()), ("name", &file_name(&out))],
                );
                if encryption_removed {
                    msg = format!("{msg} {}", tr(lang, "tools.done.encryption_removed"));
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
            BatchResult::Failed(e) => {
                log::warn!("tâche échouée : {e}");
                self.error = Some(crate::app::error_key(&e));
            }
        }
    }
}
