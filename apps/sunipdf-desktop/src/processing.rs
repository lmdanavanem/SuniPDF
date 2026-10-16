//! Cancellable image compression and sequential multi-file jobs on the engine thread.

use crate::{
    app::SuniApp,
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{compress::CompressionOptions, ocr::OcrLanguage, EngineError};
use sunipdf_i18n::{tr, tr_args};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Compress,
    Ocr,
}

pub struct ProcessingJob {
    pub inputs: Vec<PathBuf>,
    pub outputs: Vec<PathBuf>,
    pub operation: Operation,
    pub options: CompressionOptions,
    pub language: OcrLanguage,
    pub cancel: Arc<AtomicBool>,
}

pub struct FileResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub result: Result<(usize, usize), EngineError>,
}

pub struct ProcessingState {
    pub open: bool,
    pub batch: bool,
    pub busy: bool,
    pub operation: Operation,
    pub options: CompressionOptions,
    pub language: OcrLanguage,
    pub inputs: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub done: usize,
    pub total: usize,
    pub item_done: usize,
    pub item_total: usize,
    pub results: Vec<FileResult>,
    pub cancel: Arc<AtomicBool>,
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self {
            open: false,
            batch: false,
            busy: false,
            operation: Operation::Compress,
            options: Default::default(),
            language: OcrLanguage::EnglishFrench,
            inputs: Vec::new(),
            output_dir: None,
            done: 0,
            total: 0,
            item_done: 0,
            item_total: 0,
            results: Vec::new(),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub fn output_paths(
    inputs: &[PathBuf],
    directory: &std::path::Path,
    operation: Operation,
) -> Result<Vec<PathBuf>, EngineError> {
    if inputs.is_empty() || inputs.len() > 128 {
        return Err(EngineError::Other(
            "choose between 1 and 128 PDF files".into(),
        ));
    }
    let mut names = std::collections::HashSet::new();
    let suffix = if operation == Operation::Compress {
        "compressed"
    } else {
        "ocr"
    };
    let mut paths = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let mut name = format!("{stem}-{suffix}.pdf");
        if !names.insert(name.to_lowercase()) {
            name = format!("{stem}-{suffix}-{}.pdf", index + 1);
            if !names.insert(name.to_lowercase()) {
                return Err(EngineError::Other("duplicate output file names".into()));
            }
        }
        paths.push(directory.join(name));
    }
    Ok(paths)
}

pub(crate) fn run_job<'a>(
    engine: &'a sunipdf_core::Engine,
    current: Option<&sunipdf_core::Document<'a>>,
    job: ProcessingJob,
    mut progress: impl FnMut(usize, usize, usize, usize),
) -> Vec<FileResult> {
    let total = job.outputs.len();
    if total == 0 || total > 128 || (!job.inputs.is_empty() && job.inputs.len() != total) {
        return Vec::new();
    }
    let mut results = Vec::new();
    for (index, output) in job.outputs.iter().enumerate() {
        let input = job
            .inputs
            .get(index)
            .cloned()
            .or_else(|| current.map(|doc| doc.info().path.clone()))
            .unwrap_or_default();
        let result = (|| {
            if job.cancel.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            if output.exists() {
                return Err(EngineError::AlreadyExists(output.clone()));
            }
            let mut opened = None;
            let mut file_bytes = None;
            if !job.inputs.is_empty() {
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(&input)?
                    .take(256 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > 256 * 1024 * 1024 {
                    return Err(EngineError::Other("batch input exceeds 256 MiB".into()));
                }
                opened = Some(engine.open_bytes(bytes.clone(), None, &input)?);
                file_bytes = Some(bytes);
            }
            let doc = opened
                .as_ref()
                .or(current)
                .ok_or_else(|| EngineError::Other("no document".into()))?;
            if doc.signature_count() != 0 {
                return Err(EngineError::SecurityRestricted);
            }
            let before = match file_bytes {
                Some(bytes) => bytes,
                None => doc.snapshot()?,
            };
            let password = doc.password().map(str::to_owned);
            let page_count = doc.page_count();
            let (after, before_len) = match job.operation {
                Operation::Compress => {
                    let (bytes, report) = sunipdf_core::compress::compress_images(
                        &before,
                        password.as_deref(),
                        job.options,
                        &job.cancel,
                        |done, count| progress(index, total, done, count),
                    )?;
                    (bytes, report.before)
                }
                Operation::Ocr => {
                    let doc = opened.as_mut().ok_or_else(|| {
                        EngineError::Other("OCR batch requires input files".into())
                    })?;
                    doc.ocr(
                        &sunipdf_core::ocr::OcrOptions {
                            pages: (0..doc.page_count()).collect(),
                            language: job.language,
                        },
                        &job.cancel,
                        |done, count| progress(index, total, done, count),
                    )?;
                    (doc.snapshot()?, before.len())
                }
            };
            if job.cancel.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            let after_len = after.len();
            let reopened = engine.open_bytes(after.clone(), password.as_deref(), output)?;
            if reopened.page_count() != page_count {
                return Err(EngineError::InvalidFormat);
            }
            if job.operation == Operation::Compress {
                let original = opened
                    .as_ref()
                    .or(current)
                    .ok_or(EngineError::InvalidFormat)?;
                let old = original.properties();
                let new = reopened.properties();
                if original.info().page_sizes != reopened.info().page_sizes
                    || original.form_info()? != reopened.form_info()?
                    || old.encrypted != new.encrypted
                    || old.can_modify != new.can_modify
                    || old.can_copy != new.can_copy
                    || old.can_print != new.can_print
                    || old.can_annotate != new.can_annotate
                {
                    return Err(EngineError::Other(
                        "compressed PDF structure or protection changed".into(),
                    ));
                }
                for page in 0..page_count {
                    if job.cancel.load(Ordering::Relaxed) {
                        return Err(EngineError::Cancelled);
                    }
                    if original.page_text(page)? != reopened.page_text(page)?
                        || original.page_links(page)? != reopened.page_links(page)?
                    {
                        return Err(EngineError::Other(
                            "compressed PDF text or links changed".into(),
                        ));
                    }
                }
            }
            drop(reopened);
            sunipdf_core::write_new_copy(output, &after)?;
            Ok((before_len, after_len))
        })();
        results.push(FileResult {
            input,
            output: output.clone(),
            result,
        });
        progress(index + 1, total, 0, 0);
    }
    results
}

impl SuniApp {
    fn processing_source_ready(&mut self) -> bool {
        if !self.processing.batch && !self.annotate.redact.marks.is_empty() {
            self.error = Some("processing.finish_redaction");
            return false;
        }
        if !self.processing.batch && self.edit.text_prompt.is_some() {
            self.error = Some("reading.finish_edit");
            return false;
        }
        true
    }

    pub(crate) fn open_processing(&mut self, batch: bool) {
        self.processing.batch = batch;
        if !self.processing_source_ready() {
            return;
        }
        self.processing.open = true;
        self.processing.results.clear();
        if !batch {
            self.processing.operation = Operation::Compress;
        }
    }

    pub(crate) fn processing_dialog(&mut self, ctx: &egui::Context) {
        if !self.processing.open && !self.processing.busy {
            return;
        }
        let lang = self.lang();
        let (mut run, mut close, mut pick, mut folder) = (false, false, false, false);
        dialog::show(
            ctx,
            Spec::new(
                "processing",
                tr(
                    lang,
                    if self.processing.batch {
                        "processing.batch_title"
                    } else {
                        "processing.title"
                    },
                ),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(580.0),
            |ui| {
                if self.processing.busy {
                    ui.label(tr_args(
                        lang,
                        "processing.progress",
                        &[
                            ("done", &self.processing.done.to_string()),
                            ("total", &self.processing.total.to_string()),
                        ],
                    ));
                    ui.add(
                        egui::ProgressBar::new(
                            self.processing.done as f32 / self.processing.total.max(1) as f32,
                        )
                        .show_percentage(),
                    );
                    ui.add(
                        egui::ProgressBar::new(
                            self.processing.item_done as f32
                                / self.processing.item_total.max(1) as f32,
                        )
                        .show_percentage(),
                    );
                    if self.processing.cancel.load(Ordering::Relaxed) {
                        ui.label(tr(lang, "ocr.cancelling"));
                    }
                    return;
                }
                if !self.processing.results.is_empty() {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for item in &self.processing.results {
                                ui.group(|ui| {
                                    ui.label(
                                        item.input
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy(),
                                    );
                                    match &item.result {
                                        Ok((before, after)) => {
                                            ui.label(tr_args(
                                                lang,
                                                "processing.sizes",
                                                &[
                                                    ("before", &before.to_string()),
                                                    ("after", &after.to_string()),
                                                ],
                                            ));
                                            ui.label(item.output.display().to_string());
                                        }
                                        Err(error) => {
                                            ui.colored_label(
                                                crate::theme::palette(self.settings.dark_theme)
                                                    .error,
                                                tr(lang, crate::app::error_key(error)),
                                            );
                                        }
                                    }
                                });
                            }
                        });
                    return;
                }
                ui.label(tr(lang, "processing.description"));
                if self.processing.batch {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.processing.operation,
                            Operation::Compress,
                            tr(lang, "processing.compress"),
                        );
                        ui.selectable_value(
                            &mut self.processing.operation,
                            Operation::Ocr,
                            tr(lang, "processing.ocr"),
                        );
                    });
                    pick = ui.button(tr(lang, "processing.pick")).clicked();
                    ui.label(tr_args(
                        lang,
                        "processing.files",
                        &[("count", &self.processing.inputs.len().to_string())],
                    ));
                    folder = ui.button(tr(lang, "processing.folder")).clicked();
                    if let Some(dir) = &self.processing.output_dir {
                        ui.label(dir.display().to_string());
                    }
                }
                if self.processing.operation == Operation::Compress {
                    ui.add(
                        egui::Slider::new(&mut self.processing.options.max_image_side, 128..=8192)
                            .text(tr(lang, "processing.resolution")),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.processing.options.jpeg_quality, 10..=100)
                            .text(tr(lang, "processing.quality")),
                    );
                    ui.label(tr(lang, "processing.image_limits"));
                } else {
                    for (language, key) in [
                        (OcrLanguage::EnglishFrench, "ocr.both"),
                        (OcrLanguage::English, "ocr.english"),
                        (OcrLanguage::French, "ocr.french"),
                    ] {
                        ui.radio_value(&mut self.processing.language, language, tr(lang, key));
                    }
                }
            },
            |ui| {
                if !self.processing.busy && self.processing.results.is_empty() {
                    let ready = !self.processing.batch
                        || (!self.processing.inputs.is_empty()
                            && self.processing.output_dir.is_some());
                    run = ui
                        .add_enabled(ready, egui::Button::new(tr(lang, "processing.run")))
                        .clicked();
                }
                close = dialog::secondary(
                    ui,
                    tr(
                        lang,
                        if self.processing.busy {
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
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_files()
            {
                if files.len() > 128 {
                    self.error = Some("processing.too_many");
                } else {
                    self.processing.inputs = files;
                }
            }
        }
        if folder {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                self.processing.output_dir = Some(dir);
            }
        }
        if close {
            if self.processing.busy {
                self.processing.cancel.store(true, Ordering::Relaxed);
            } else {
                self.processing.open = false;
            }
        }
        if !run || !self.processing_source_ready() || !self.commit_form_input() {
            return;
        }
        let (inputs, outputs) = if self.processing.batch {
            let inputs = self.processing.inputs.clone();
            let Some(dir) = &self.processing.output_dir else {
                return;
            };
            match output_paths(&inputs, dir, self.processing.operation) {
                Ok(outputs) => (inputs, outputs),
                Err(error) => {
                    self.error = Some(crate::app::error_key(&error));
                    return;
                }
            }
        } else {
            let Some(doc) = &self.document else { return };
            let suggested = format!(
                "{}-compressed.pdf",
                doc.info
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            let Some(mut out) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .set_title(tr(lang, "processing.save"))
                .set_file_name(suggested)
                .save_file()
            else {
                return;
            };
            if out.extension().is_none() {
                out.set_extension("pdf");
            }
            (Vec::new(), vec![out])
        };
        self.processing.cancel = Arc::new(AtomicBool::new(false));
        self.processing.done = 0;
        self.processing.total = outputs.len();
        self.processing.item_done = 0;
        self.processing.item_total = 0;
        self.processing.busy = true;
        self.worker.send(Request::ProcessFiles(ProcessingJob {
            inputs,
            outputs,
            operation: self.processing.operation,
            options: self.processing.options,
            language: self.processing.language,
            cancel: self.processing.cancel.clone(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compression_cannot_export_unapplied_redaction_marks() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.annotate.redact.marks.push(sunipdf_core::RedactMark {
            page: 0,
            rect: sunipdf_core::PointRect {
                left: 20.0,
                bottom: 20.0,
                right: 80.0,
                top: 80.0,
            },
        });
        app.open_processing(false);
        assert!(!app.processing.open);
        assert_eq!(app.error, Some("processing.finish_redaction"));
        assert_eq!(app.annotate.redact.marks.len(), 1);
        assert!(requests.try_iter().next().is_none());
    }
    #[test]
    fn batch_outputs_are_distinct_and_never_reuse_input_names() {
        let inputs = vec![PathBuf::from("a/Report.pdf"), PathBuf::from("b/report.pdf")];
        let paths =
            output_paths(&inputs, std::path::Path::new("out"), Operation::Compress).unwrap();
        assert_ne!(
            paths[0].to_string_lossy().to_lowercase(),
            paths[1].to_string_lossy().to_lowercase()
        );
        assert!(paths.iter().all(|path| !inputs.contains(path)));
        assert!(output_paths(&[], std::path::Path::new("out"), Operation::Ocr).is_err());
    }

    #[test]
    fn real_batch_continues_after_failures_preserves_sources_and_cancels_between_files() {
        if std::env::var_os("SUNIPDF_BATCH_TEST_CHILD").is_none() {
            let output=std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact","processing::tests::real_batch_continues_after_failures_preserves_sources_and_cancels_between_files"])
                .env("SUNIPDF_BATCH_TEST_CHILD","1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let engine = Box::leak(Box::new(sunipdf_core::Engine::new().unwrap()));
        let directory = tempfile::tempdir().unwrap();
        let source = include_bytes!("../../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let inputs: Vec<_> = (0..3)
            .map(|i| directory.path().join(format!("source-{i}.pdf")))
            .collect();
        for path in &inputs {
            std::fs::write(path, source).unwrap();
        }
        std::fs::write(&inputs[1], b"not a PDF").unwrap();
        let outputs: Vec<_> = (0..3)
            .map(|i| directory.path().join(format!("output-{i}.pdf")))
            .collect();
        std::fs::write(&outputs[0], b"existing output").unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = ProcessingJob {
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            operation: Operation::Compress,
            options: Default::default(),
            language: OcrLanguage::EnglishFrench,
            cancel: cancel.clone(),
        };
        let results = run_job(engine, None, job, |_, _, _, _| {});
        assert_eq!(results.len(), 3);
        assert!(matches!(
            results[0].result,
            Err(EngineError::AlreadyExists(_))
        ));
        assert!(results[1].result.is_err());
        assert!(results[2].result.is_ok());
        assert_eq!(std::fs::read(&outputs[0]).unwrap(), b"existing output");
        assert!(!outputs[1].exists());
        assert_eq!(std::fs::read(&outputs[2]).unwrap(), source);
        for index in [0, 2] {
            assert_eq!(std::fs::read(&inputs[index]).unwrap(), source);
        }
        assert_eq!(std::fs::read(&inputs[1]).unwrap(), b"not a PDF");
        let next: Vec<_> = (0..2)
            .map(|i| directory.path().join(format!("cancel-{i}.pdf")))
            .collect();
        let job = ProcessingJob {
            inputs: vec![inputs[0].clone(), inputs[2].clone()],
            outputs: next.clone(),
            operation: Operation::Compress,
            options: Default::default(),
            language: OcrLanguage::EnglishFrench,
            cancel: cancel.clone(),
        };
        let results = run_job(engine, None, job, |done, _, _, _| {
            if done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert!(results[0].result.is_ok());
        assert!(matches!(results[1].result, Err(EngineError::Cancelled)));
        assert!(next[0].exists());
        assert!(!next[1].exists());
    }
}
