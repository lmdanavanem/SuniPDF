//! Local file comparison and multi-document search on the existing engine worker.

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
use sunipdf_core::{
    local_review::{self as core, CompareOptions, Comparison, ComparisonPreview, SearchReport},
    EngineError,
};
use sunipdf_i18n::{tr, tr_args};

pub enum Job {
    Search {
        files: Vec<PathBuf>,
        query: String,
        match_case: bool,
        cancel: Arc<AtomicBool>,
    },
    Compare {
        left: PathBuf,
        right: PathBuf,
        options: CompareOptions,
        cancel: Arc<AtomicBool>,
    },
    Preview {
        report: Arc<Comparison>,
        index: usize,
        cancel: Arc<AtomicBool>,
    },
}
pub enum Outcome {
    Search(SearchReport),
    Compare(Arc<Comparison>),
    Preview {
        index: usize,
        images: ComparisonPreview,
    },
}
pub fn run_job(
    engine: &sunipdf_core::Engine,
    job: Job,
    mut progress: impl FnMut(usize, usize, usize, usize),
) -> Result<Outcome, EngineError> {
    match job {
        Job::Search {
            files,
            query,
            match_case,
            cancel,
        } => core::search_files(engine, &files, &query, match_case, None, &cancel, progress)
            .map(Outcome::Search),
        Job::Compare {
            left,
            right,
            options,
            cancel,
        } => core::compare_files(
            engine,
            &left,
            &right,
            options,
            None,
            &cancel,
            |done, total| progress(done, total, 0, 0),
        )
        .map(|report| Outcome::Compare(Arc::new(report))),
        Job::Preview {
            report,
            index,
            cancel,
        } => {
            if cancel.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            let images = core::comparison_preview(engine, &report, index, None)?;
            if cancel.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            Ok(Outcome::Preview { index, images })
        }
    }
}

pub struct ReviewState {
    pub open: bool,
    pub busy: bool,
    pub compare: bool,
    pub cancel: Arc<AtomicBool>,
    pub progress: (usize, usize, usize, usize),
    pub pending_page: Option<(PathBuf, usize)>,
    files: Vec<PathBuf>,
    left: Option<PathBuf>,
    right: Option<PathBuf>,
    query: String,
    match_case: bool,
    options: CompareOptions,
    search: Option<SearchReport>,
    comparison: Option<Arc<Comparison>>,
    preview_index: Option<usize>,
    textures: Vec<Option<egui::TextureHandle>>,
    error: Option<String>,
}
impl Default for ReviewState {
    fn default() -> Self {
        Self {
            open: false,
            busy: false,
            compare: false,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: (0, 0, 0, 0),
            pending_page: None,
            files: Vec::new(),
            left: None,
            right: None,
            query: String::new(),
            match_case: false,
            options: CompareOptions::default(),
            search: None,
            comparison: None,
            preview_index: None,
            textures: Vec::new(),
            error: None,
        }
    }
}
impl SuniApp {
    pub fn open_local_review(&mut self, compare: bool) {
        if self.review.busy {
            return;
        }
        self.review.open = true;
        self.review.compare = compare;
        self.review.error = None;
    }

    pub fn on_local_review(&mut self, ctx: &egui::Context, result: Result<Outcome, EngineError>) {
        self.review.busy = false;
        match result {
            Ok(Outcome::Search(report)) => self.review.search = Some(report),
            Ok(Outcome::Compare(report)) => self.review.comparison = Some(report),
            Ok(Outcome::Preview { index, images }) => {
                self.review.preview_index = Some(index);
                self.review.textures = [images.left, images.right, Some(images.difference)]
                    .into_iter()
                    .enumerate()
                    .map(|(i, image)| {
                        image.map(|im| {
                            ctx.load_texture(
                                format!("comparison-preview-{i}"),
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [im.width as usize, im.height as usize],
                                    &im.rgba,
                                ),
                                egui::TextureOptions::LINEAR,
                            )
                        })
                    })
                    .collect();
            }
            Err(error) => {
                self.review.error = Some(format!(
                    "{} ({error})",
                    tr(self.lang(), crate::app::error_key(&error))
                ))
            }
        }
    }

    pub fn local_review_dialog(&mut self, ctx: &egui::Context) {
        if !self.review.open && !self.review.busy {
            return;
        }
        let lang = self.lang();
        let compare = self.review.compare;
        let busy = self.review.busy;
        let (mut run, mut close, mut pick) = (false, false, None);
        let mut preview = None;
        let mut options_changed = false;
        let mut open_page = None;
        let ready = matches!(self.engine, crate::app::EngineState::Ready)
            && if compare {
                self.review.left.is_some() && self.review.right.is_some()
            } else {
                !self.review.files.is_empty()
                    && !self.review.query.trim().is_empty()
                    && self.review.query.chars().count() <= 256
            };
        dialog::show(
            ctx,
            Spec::new(
                "local_review",
                tr(
                    lang,
                    if compare {
                        "review.compare"
                    } else {
                        "review.search"
                    },
                ),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(650.0),
            |ui| {
                ui.label(tr(lang, "review.saved_files"));
                ui.small(tr(lang, "review.limits"));
                if busy {
                    let (done, total, page, pages) = self.review.progress;
                    ui.label(tr_args(
                        lang,
                        "review.progress",
                        &[
                            ("done", &done.to_string()),
                            ("total", &total.to_string()),
                            ("page", &page.to_string()),
                            ("pages", &pages.to_string()),
                        ],
                    ));
                    ui.add(
                        egui::ProgressBar::new(done as f32 / total.max(1) as f32).show_percentage(),
                    );
                    if self.review.cancel.load(Ordering::Relaxed) {
                        ui.label(tr(lang, "ocr.cancelling"));
                    }
                    return;
                }
                if compare {
                    for (index, path, key) in [
                        (0, &self.review.left, "review.left"),
                        (1, &self.review.right, "review.right"),
                    ] {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(tr(lang, key)).clicked() {
                                pick = Some(index);
                            }
                            if let Some(path) = path {
                                ui.label(path.display().to_string());
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "review.left_start"));
                        options_changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.review.options.left_start)
                                    .range(1..=65535),
                            )
                            .changed();
                        ui.label(tr(lang, "review.right_start"));
                        options_changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.review.options.right_start)
                                    .range(1..=65535),
                            )
                            .changed();
                        ui.label(tr(lang, "review.max_pairs"));
                        options_changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.review.options.max_pages)
                                    .range(1..=500),
                            )
                            .changed();
                    });
                    if options_changed {
                        self.review.comparison = None;
                        self.review.preview_index = None;
                        self.review.textures.clear();
                    }
                    ui.small(tr(lang, "review.compare_limits"));
                    if let Some(report) = &self.review.comparison {
                        ui.separator();
                        if report.pages.iter().any(|page| page.no_text_on_either_page) {
                            ui.label(tr(lang, "review.no_text"));
                        }
                        ui.strong(tr(
                            lang,
                            if report.complete() {
                                "review.complete"
                            } else {
                                "review.partial"
                            },
                        ));
                        ui.label(tr_args(
                            lang,
                            "review.comparison_summary",
                            &[
                                ("scanned", &report.pages.len().to_string()),
                                ("total", &report.aligned_pages.to_string()),
                                (
                                    "changed",
                                    &report
                                        .pages
                                        .iter()
                                        .filter(|p| p.changed())
                                        .count()
                                        .to_string(),
                                ),
                            ],
                        ));
                        if let Some(index) = self.review.preview_index {
                            if let Some(page) = report.pages.get(index) {
                                ui.label(tr_args(
                                    lang,
                                    "review.pair",
                                    &[
                                        (
                                            "left",
                                            &page
                                                .left_page
                                                .map_or_else(|| "—".into(), |p| p.to_string()),
                                        ),
                                        (
                                            "right",
                                            &page
                                                .right_page
                                                .map_or_else(|| "—".into(), |p| p.to_string()),
                                        ),
                                    ],
                                ));
                                ui.columns(3, |columns| {
                                    for (i, key) in
                                        ["review.original", "review.revised", "review.difference"]
                                            .into_iter()
                                            .enumerate()
                                    {
                                        columns[i].label(tr(lang, key));
                                        if let Some(Some(texture)) = self.review.textures.get(i) {
                                            let size = texture.size_vec2()
                                                * (columns[i].available_width()
                                                    / texture.size_vec2().x)
                                                    .min(1.0);
                                            columns[i].add(
                                                egui::Image::new(texture).fit_to_exact_size(size),
                                            );
                                        }
                                    }
                                });
                            }
                        }
                        for (index, page) in report.pages.iter().enumerate() {
                            if !page.changed() {
                                continue;
                            }
                            let left = page.left_page.map_or_else(|| "—".into(), |p| p.to_string());
                            let right = page
                                .right_page
                                .map_or_else(|| "—".into(), |p| p.to_string());
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .button(tr_args(
                                        lang,
                                        "review.pair",
                                        &[("left", &left), ("right", &right)],
                                    ))
                                    .clicked()
                                {
                                    preview = Some(index);
                                }
                                if page.text_changed {
                                    ui.label(tr(lang, "review.text_changed"));
                                }
                                if page.geometry_changed {
                                    ui.label(tr(lang, "review.geometry_changed"));
                                }
                                ui.label(tr_args(
                                    lang,
                                    "review.pixels",
                                    &[("count", &page.changed_pixels.to_string())],
                                ));
                            });
                        }
                    }
                } else {
                    if ui.button(tr(lang, "review.pick")).clicked() {
                        pick = Some(2);
                    }
                    for path in &self.review.files {
                        ui.small(path.display().to_string());
                    }
                    ui.label(tr(lang, "review.query"));
                    let edited = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.review.query)
                                .char_limit(256)
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                    let changed_case = ui
                        .checkbox(&mut self.review.match_case, tr(lang, "find.match_case"))
                        .changed();
                    if edited || changed_case {
                        self.review.search = None;
                    }
                    if let Some(report) = &self.review.search {
                        ui.separator();
                        ui.strong(tr(
                            lang,
                            if report.complete() {
                                "review.complete"
                            } else {
                                "review.partial"
                            },
                        ));
                        for file in &report.files {
                            ui.group(|ui| {
                                ui.label(file.path.display().to_string());
                                ui.label(tr(
                                    lang,
                                    match file.status {
                                        "complete" => "review.complete",
                                        "cancelled" => "review.cancelled",
                                        "limit_reached" => "review.limit_reached",
                                        _ => "review.failed",
                                    },
                                ));
                                if let Some(error) = &file.error {
                                    ui.label(tr(lang, crate::app::error_key(error)));
                                }
                                ui.small(tr_args(
                                    lang,
                                    "review.scanned",
                                    &[
                                        ("scanned", &file.scanned_pages.to_string()),
                                        (
                                            "total",
                                            &file
                                                .page_count
                                                .map_or_else(|| "?".into(), |p| p.to_string()),
                                        ),
                                    ],
                                ));
                                if file.status == "complete" && file.hits.is_empty() {
                                    ui.label(tr(lang, "review.no_matches"));
                                }
                                for hit in &file.hits {
                                    if ui
                                        .button(tr_args(
                                            lang,
                                            "review.hit",
                                            &[
                                                ("page", &hit.page.to_string()),
                                                ("count", &hit.occurrences.to_string()),
                                            ],
                                        ))
                                        .clicked()
                                    {
                                        open_page = Some((file.path.clone(), hit.page));
                                    }
                                    ui.label(&hit.snippet);
                                }
                            });
                        }
                    }
                }
                if let Some(error) = &self.review.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
            },
            |ui| {
                if busy {
                    if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        self.review.cancel.store(true, Ordering::Relaxed);
                    }
                } else {
                    ui.add_enabled_ui(ready, |ui| {
                        run = dialog::primary(ui, tr(lang, "review.run"));
                    });
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                }
            },
        );
        if let Some(index) = pick {
            let picker =
                rfd::FileDialog::new().add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"]);
            if index == 2 {
                if let Some(files) = picker.pick_files() {
                    if files.len() <= core::MAX_FILES {
                        self.review.files = files;
                        self.review.search = None;
                    } else {
                        self.review.error = Some(tr(lang, "review.limits").into());
                    }
                }
            } else if let Some(path) = picker.pick_file() {
                if index == 0 {
                    self.review.left = Some(path);
                } else {
                    self.review.right = Some(path);
                }
                self.review.comparison = None;
                self.review.textures.clear();
                self.review.preview_index = None;
            }
        }
        if run {
            self.review.cancel = Arc::new(AtomicBool::new(false));
            self.review.error = None;
            self.review.progress = (0, 0, 0, 0);
            self.review.textures.clear();
            self.review.preview_index = None;
            let job = if compare {
                self.review.comparison = None;
                Job::Compare {
                    left: self.review.left.clone().unwrap(),
                    right: self.review.right.clone().unwrap(),
                    options: self.review.options,
                    cancel: self.review.cancel.clone(),
                }
            } else {
                self.review.search = None;
                Job::Search {
                    files: self.review.files.clone(),
                    query: self.review.query.clone(),
                    match_case: self.review.match_case,
                    cancel: self.review.cancel.clone(),
                }
            };
            self.review.busy = true;
            self.worker.send(Request::LocalReview(job));
        } else if let Some(index) = preview {
            self.review.cancel = Arc::new(AtomicBool::new(false));
            self.review.progress = (0, 1, 0, 0);
            self.review.busy = true;
            self.review.error = None;
            self.review.preview_index = None;
            self.review.textures.clear();
            self.worker.send(Request::LocalReview(Job::Preview {
                report: self.review.comparison.clone().unwrap(),
                index,
                cancel: self.review.cancel.clone(),
            }));
        }
        if close {
            self.review.open = false;
        }
        if let Some((path, page)) = open_page {
            self.review.open = false;
            self.open_path(path.clone(), None);
            if self.opening.as_ref() == Some(&path) {
                self.review.pending_page = Some((path, page));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{tests::test_app, EngineState};
    use sunipdf_core::Command;

    #[test]
    fn review_commands_work_without_an_open_document_and_quit_cancels_the_job() {
        let (mut app, _, _) = test_app();
        let ctx = egui::Context::default();
        app.engine = EngineState::Ready;
        app.document = None;
        app.execute(&ctx, Command::CompareFiles);
        assert!(app.review.open && app.review.compare && app.document.is_none());
        app.review.open = false;
        app.execute(&ctx, Command::SearchFiles);
        assert!(app.review.open && !app.review.compare);
        assert!(app.dialog_open());
        app.review.busy = true;
        assert!(app.navigation_blocked());
        app.execute(&ctx, Command::Quit);
        assert!(app.review.cancel.load(Ordering::Relaxed));
        assert!(app.review.busy);
    }

    #[test]
    fn failed_review_clears_busy_and_keeps_the_file_selection_for_retry() {
        let (mut app, _, _) = test_app();
        app.review.files = vec!["selected.pdf".into()];
        app.review.busy = true;
        app.on_local_review(
            &egui::Context::default(),
            Err(EngineError::PasswordRequired),
        );
        assert!(!app.review.busy);
        assert_eq!(app.review.files, vec![PathBuf::from("selected.pdf")]);
        assert!(app.review.error.is_some());
    }

    #[test]
    fn search_result_navigates_to_its_physical_page_after_the_source_opens() {
        let (mut app, _requests, responses) = test_app();
        app.document = None;
        let path = PathBuf::from("search-source.pdf");
        app.review.pending_page = Some((path.clone(), 2));
        responses
            .send(crate::worker::Response::Opened(crate::worker::DocState {
                info: sunipdf_core::DocumentInfo {
                    path,
                    title: None,
                    page_sizes: vec![
                        sunipdf_core::PageSize {
                            width_pt: 200.0,
                            height_pt: 160.0
                        };
                        2
                    ],
                    page_labels: vec![],
                    form: sunipdf_core::FormKind::None,
                },
                can_undo: false,
                can_redo: false,
                modified: false,
                pages_unchanged: true,
                has_signatures: false,
            }))
            .unwrap();
        app.handle_responses(&egui::Context::default());
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(app.review.pending_page.is_none());
    }

    #[test]
    fn review_dialog_renders_translated_controls_at_minimum_window_size_and_escape_cancels() {
        fn text(shape: &egui::Shape, all: &mut String) {
            match shape {
                egui::Shape::Text(t) => {
                    all.push_str(t.galley.text());
                    all.push('\n');
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text(shape, all);
                    }
                }
                _ => {}
            }
        }
        for language in sunipdf_i18n::Language::ALL {
            for compare in [false, true] {
                let (mut app, _requests, _responses) = test_app();
                app.settings.set_language(language);
                app.open_local_review(compare);
                let ctx = egui::Context::default();
                crate::icons::install(&ctx);
                let input = || egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    ..Default::default()
                };
                let mut first = ctx.run_ui(input(), |ui| app.local_review_dialog(ui.ctx()));
                first.textures_delta.clear();
                let mut output = ctx.run_ui(input(), |ui| app.local_review_dialog(ui.ctx()));
                output.textures_delta.clear();
                let mut labels = String::new();
                for shape in output.shapes {
                    text(&shape.shape, &mut labels);
                }
                assert!(labels.contains(tr(language, "review.run")));
                assert!(!labels.contains("review.") && !labels.contains("dialog."));
                app.review.busy = true;
                let mut escape = input();
                escape.events.push(egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                let mut cancelled = ctx.run_ui(escape, |ui| app.local_review_dialog(ui.ctx()));
                cancelled.textures_delta.clear();
                assert!(app.review.cancel.load(Ordering::Relaxed));
            }
        }
    }
}
