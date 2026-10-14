//! Outils de fichiers : combiner, diviser, extraire, créer depuis des images, exporter en images,
//! optimiser. Les tâches tournent sur le thread moteur ; l'UI reçoit un `BatchDone`.

use crate::app::SuniApp;
use crate::worker::{BatchOp, BatchResult, Request};
use egui::Key;
use std::path::{Path, PathBuf};
use sunipdf_i18n::{tr, tr_args};

#[derive(Default)]
pub struct ToolsState {
    /// Dialogue « Diviser » : taille des lots.
    pub split_open: bool,
    pub split_every: usize,
    pub running: Option<&'static str>,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} Mo", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.0} Ko", bytes as f64 / 1024.0)
    }
}

impl SuniApp {
    fn pdf_filter(&self) -> rfd::FileDialog {
        rfd::FileDialog::new().add_filter(self.t("dialog.open.filter_pdf"), &["pdf"])
    }

    fn suggest_output(&self, suffix: &str) -> PathBuf {
        match &self.document {
            Some(doc) => {
                let stem = doc
                    .info
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "document".into());
                doc.info.path.with_file_name(format!("{stem}{suffix}.pdf"))
            }
            None => PathBuf::from(format!("sunipdf{suffix}.pdf")),
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
        let Some(out) = self.save_dialog("tools.combine.save", &self.suggest_output("-combine"))
        else {
            return;
        };
        self.run_batch(BatchOp::Combine { inputs, out }, "tools.running.combine");
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
        let Some(out) = self.save_dialog("tools.extract.save", &self.suggest_output("-extrait"))
        else {
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
        let Some(out) = self.save_dialog("tools.optimize.save", &self.suggest_output("-optimise"))
        else {
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
            BatchResult::Split { files } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.split",
                    &[("count", &files.len().to_string())],
                ));
            }
            BatchResult::Extracted { out, pages } => {
                self.status_msg = Some(tr_args(
                    lang,
                    "tools.done.extract",
                    &[("pages", &pages.to_string()), ("name", &file_name(&out))],
                ));
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
                        ("before", &human_size(before)),
                        ("after", &human_size(after)),
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
