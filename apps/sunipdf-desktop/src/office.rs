//! Local Office jobs and explicit per-format export limits.

use crate::{
    app::SuniApp,
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    export_office::Format,
    office::{Converter, Profile},
    EngineError,
};
use sunipdf_i18n::{tr, tr_args};

pub enum Job {
    ToPdf {
        inputs: Vec<PathBuf>,
        outputs: Vec<PathBuf>,
        converter: Converter,
        profile: Profile,
        cancel: Arc<AtomicBool>,
    },
    FromPdf {
        output: PathBuf,
        format: Format,
        cancel: Arc<AtomicBool>,
    },
}
pub struct FileResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub result: Result<serde_json::Value, EngineError>,
}
pub struct OfficeState {
    pub open: bool,
    pub busy: bool,
    pub cancel: Arc<AtomicBool>,
    pub done: usize,
    pub total: usize,
    pub item_done: usize,
    pub item_total: usize,
    pub results: Vec<FileResult>,
    export: bool,
    converter: Option<Converter>,
    profile: Profile,
    format: Format,
    inputs: Vec<PathBuf>,
    directory: Option<PathBuf>,
}
impl Default for OfficeState {
    fn default() -> Self {
        Self {
            open: false,
            busy: false,
            cancel: Arc::new(AtomicBool::new(false)),
            done: 0,
            total: 0,
            item_done: 0,
            item_total: 0,
            results: Vec::new(),
            export: false,
            converter: None,
            profile: Profile::default(),
            format: Format::default(),
            inputs: Vec::new(),
            directory: None,
        }
    }
}
fn output_paths(inputs: &[PathBuf], directory: &Path) -> Result<Vec<PathBuf>, EngineError> {
    if inputs.is_empty() || inputs.len() > 128 {
        return Err(EngineError::Other(
            "Choose 1 to 128 local Office documents".into(),
        ));
    }
    let mut names = std::collections::HashSet::new();
    let mut outputs = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let mut name = format!("{stem}-converted.pdf");
        if !names.insert(name.to_lowercase()) {
            name = format!("{stem}-converted-{}.pdf", index + 1);
            if !names.insert(name.to_lowercase()) {
                return Err(EngineError::Other(
                    "Duplicate conversion output names".into(),
                ));
            }
        }
        outputs.push(directory.join(name));
    }
    Ok(outputs)
}
pub(crate) fn run_job<'a>(
    engine: &'a sunipdf_core::Engine,
    current: Option<&sunipdf_core::Document<'a>>,
    job: Job,
    mut progress: impl FnMut(usize, usize, usize, usize),
) -> Vec<FileResult> {
    match job {
        Job::ToPdf {
            inputs,
            outputs,
            converter,
            profile,
            cancel,
        } => {
            if inputs.is_empty() || inputs.len() > 128 || inputs.len() != outputs.len() {
                return Vec::new();
            }
            let total = inputs.len();
            let mut results = Vec::new();
            for (index, (input, output)) in inputs.into_iter().zip(outputs).enumerate() {
                progress(index, total, 0, 1);
                let result = sunipdf_core::office::convert(
                    engine, &converter, &input, &output, profile, &cancel,
                )
                .map(|report| report.json());
                results.push(FileResult {
                    input,
                    output,
                    result,
                });
                progress(index + 1, total, 0, 0);
            }
            results
        }
        Job::FromPdf {
            output,
            format,
            cancel,
        } => {
            let input = current.map(|d| d.info().path.clone()).unwrap_or_default();
            let result = current
                .ok_or_else(|| EngineError::Other("No PDF is open".into()))
                .and_then(|doc| {
                    doc.export_office(&output, format, &cancel, |done, total| {
                        progress(0, 1, done, total)
                    })
                })
                .map(|report| report.json());
            progress(1, 1, 0, 0);
            vec![FileResult {
                input,
                output,
                result,
            }]
        }
    }
}

impl SuniApp {
    fn office_source_ready(&mut self) -> bool {
        if self.office.export && !self.annotate.redact.marks.is_empty() {
            self.error = Some("processing.finish_redaction");
            return false;
        }
        if self.office.export && self.edit.text_prompt.is_some() {
            self.error = Some("reading.finish_edit");
            return false;
        }
        true
    }
    pub(crate) fn open_office(&mut self, export: bool) {
        if self.office.busy {
            return;
        }
        self.office.export = export;
        if !self.office_source_ready() {
            return;
        }
        self.office.open = true;
        self.office.results.clear();
        self.office.converter = Converter::detect();
        self.office.profile = Profile::from_id(&self.settings.office_profile).unwrap_or_default();
    }
    pub(crate) fn office_dialog(&mut self, ctx: &egui::Context) {
        if !self.office.open && !self.office.busy {
            return;
        }
        let lang = self.lang();
        let mut pick = false;
        let mut folder = false;
        let mut run = false;
        let mut close = false;
        let busy = self.office.busy;
        let ready = self.office.export
            || (self.office.converter.is_some()
                && !self.office.inputs.is_empty()
                && self.office.directory.is_some());
        dialog::show(
            ctx,
            Spec::new(
                "office",
                tr(
                    lang,
                    if self.office.export {
                        "office.export_title"
                    } else {
                        "office.import_title"
                    },
                ),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(620.0),
            |ui| {
                if busy {
                    ui.label(tr_args(
                        lang,
                        "processing.progress",
                        &[
                            ("done", &self.office.done.to_string()),
                            ("total", &self.office.total.to_string()),
                        ],
                    ));
                    ui.add(
                        egui::ProgressBar::new(
                            self.office.done as f32 / self.office.total.max(1) as f32,
                        )
                        .show_percentage(),
                    );
                    if self.office.item_total > 0 {
                        ui.add(
                            egui::ProgressBar::new(
                                self.office.item_done as f32 / self.office.item_total as f32,
                            )
                            .show_percentage(),
                        );
                    }
                    if self.office.cancel.load(Ordering::Relaxed) {
                        ui.label(tr(lang, "ocr.cancelling"));
                    }
                    return;
                }
                if !self.office.results.is_empty() {
                    for item in &self.office.results {
                        ui.group(|ui| {
                            ui.label(item.input.file_name().unwrap_or_default().to_string_lossy());
                            match &item.result {
                                Ok(report) => {
                                    ui.label(item.output.display().to_string());
                                    ui.label(tr_args(
                                        lang,
                                        "office.completed",
                                        &[(
                                            "pages",
                                            &report
                                                .get("pages")
                                                .or_else(|| report.get("source_pages"))
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or(0)
                                                .to_string(),
                                        )],
                                    ));
                                }
                                Err(error) => {
                                    ui.colored_label(
                                        crate::theme::palette_for(ctx).error,
                                        error.to_string(),
                                    );
                                }
                            }
                        });
                    }
                    return;
                }
                ui.label(tr(lang, "office.new_copy"));
                if self.office.export {
                    for (format, key) in [
                        (Format::Word, "office.word"),
                        (Format::Excel, "office.excel"),
                        (Format::PowerPoint, "office.powerpoint"),
                    ] {
                        ui.radio_value(&mut self.office.format, format, tr(lang, key));
                    }
                    ui.separator();
                    ui.label(tr(
                        lang,
                        match self.office.format {
                            Format::Word => "office.word_limits",
                            Format::Excel => "office.excel_limits",
                            Format::PowerPoint => "office.powerpoint_limits",
                        },
                    ));
                    ui.label(tr(lang, "office.export_limits"));
                } else {
                    ui.label(tr(
                        lang,
                        if self.office.converter.is_some() {
                            "office.detected"
                        } else {
                            "office.unavailable"
                        },
                    ));
                    ui.label(tr(lang, "office.import_limits"));
                    pick = ui.button(tr(lang, "office.pick")).clicked();
                    ui.label(tr_args(
                        lang,
                        "processing.files",
                        &[("count", &self.office.inputs.len().to_string())],
                    ));
                    folder = ui.button(tr(lang, "processing.folder")).clicked();
                    if let Some(dir) = &self.office.directory {
                        ui.label(dir.display().to_string());
                    }
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.office.profile,
                            Profile::Standard,
                            tr(lang, "office.standard"),
                        );
                        ui.selectable_value(
                            &mut self.office.profile,
                            Profile::Print,
                            tr(lang, "office.print"),
                        );
                    });
                    ui.label(tr(lang, "office.profile_limits"));
                }
            },
            |ui| {
                if !busy && self.office.results.is_empty() {
                    run = ui
                        .add_enabled(ready, egui::Button::new(tr(lang, "office.run")))
                        .clicked();
                }
                close = dialog::secondary(
                    ui,
                    tr(
                        lang,
                        if busy {
                            "dialog.password.cancel"
                        } else {
                            "dialog.close"
                        },
                    ),
                ) || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if pick {
            if let Some(files) = rfd::FileDialog::new()
                .add_filter(
                    tr(lang, "office.input_filter"),
                    &["docx", "xlsx", "pptx", "odt", "ods", "odp"],
                )
                .pick_files()
            {
                if files.len() > 128 {
                    self.error = Some("processing.too_many");
                } else {
                    self.office.inputs = files;
                }
            }
        }
        if folder {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                self.office.directory = Some(dir);
            }
        }
        if close {
            if busy {
                self.office.cancel.store(true, Ordering::Relaxed);
            } else {
                self.office.open = false;
            }
        }
        if !run || !self.office_source_ready() || !self.commit_form_input() {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let job = if self.office.export {
            let Some(doc) = &self.document else {
                return;
            };
            let format = self.office.format;
            let suggested = format!(
                "{}-export.{}",
                doc.info
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy(),
                format.extension()
            );
            let Some(mut output) = rfd::FileDialog::new()
                .add_filter(tr(lang, "office.output_filter"), &[format.extension()])
                .set_file_name(suggested)
                .save_file()
            else {
                return;
            };
            if output.extension().is_none() {
                output.set_extension(format.extension());
            }
            Job::FromPdf {
                output,
                format,
                cancel: cancel.clone(),
            }
        } else {
            let (Some(converter), Some(directory)) =
                (&self.office.converter, &self.office.directory)
            else {
                return;
            };
            let outputs = match output_paths(&self.office.inputs, directory) {
                Ok(outputs) => outputs,
                Err(error) => {
                    self.error = Some(crate::app::error_key(&error));
                    return;
                }
            };
            self.settings.office_profile = self.office.profile.id().into();
            if let Err(error) = self.settings.save() {
                log::warn!("Could not save Office conversion profile: {error}");
            }
            Job::ToPdf {
                inputs: self.office.inputs.clone(),
                outputs,
                converter: converter.clone(),
                profile: self.office.profile,
                cancel: cancel.clone(),
            }
        };
        self.office.total = if self.office.export {
            1
        } else {
            self.office.inputs.len()
        };
        self.office.done = 0;
        self.office.item_done = 0;
        self.office.item_total = 0;
        self.office.cancel = cancel;
        self.office.busy = true;
        self.worker.send(Request::Office(job));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batch_names_cannot_collide_and_empty_or_oversized_batches_are_refused() {
        let paths = output_paths(
            &["C:/one/file.docx".into(), "C:/two/FILE.xlsx".into()],
            Path::new("C:/out"),
        )
        .unwrap();
        assert_ne!(
            paths[0].to_string_lossy().to_lowercase(),
            paths[1].to_string_lossy().to_lowercase()
        );
        assert!(output_paths(&[], Path::new("C:/out")).is_err());
        assert!(output_paths(&vec![PathBuf::from("a.docx"); 129], Path::new("C:/out")).is_err());
    }
    #[test]
    fn unapplied_redaction_blocks_pdf_export() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.redact.marks.push(sunipdf_core::RedactMark {
            page: 0,
            rect: sunipdf_core::PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 10.0,
                top: 10.0,
            },
        });
        app.open_office(true);
        assert!(!app.office.open);
        assert_eq!(app.error, Some("processing.finish_redaction"));
        assert!(requests.try_recv().is_err());
    }
}
