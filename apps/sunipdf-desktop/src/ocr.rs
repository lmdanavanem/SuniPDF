//! Options and cancellation for local text recognition.

use crate::dialog::{self, Spec};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sunipdf_core::ocr::{OcrLanguage, OcrOptions};
use sunipdf_i18n::{tr, tr_args, Language};

pub struct OcrState {
    pub open: bool,
    pub language: OcrLanguage,
    pub all_pages: bool,
    pub busy: bool,
    pub done: usize,
    pub total: usize,
    pub cancel: Arc<AtomicBool>,
}

impl Default for OcrState {
    fn default() -> Self {
        Self {
            open: false,
            language: OcrLanguage::EnglishFrench,
            all_pages: true,
            busy: false,
            done: 0,
            total: 0,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl OcrState {
    pub fn dialog_open(&self) -> bool {
        self.open || self.busy
    }

    pub fn begin(&mut self, total: usize) {
        self.open = false;
        self.busy = true;
        self.done = 0;
        self.total = total;
        self.cancel.store(false, Ordering::Relaxed);
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        lang: Language,
        dark: bool,
        has_icons: bool,
        page_count: usize,
        current_page: usize,
    ) -> Option<OcrOptions> {
        if self.busy {
            let mut cancel = false;
            dialog::show(
                ctx,
                Spec::new("ocr_progress", tr(lang, "ocr.title"), dark, has_icons),
                |ui| {
                    ui.label(tr_args(
                        lang,
                        "ocr.progress",
                        &[
                            ("done", &self.done.to_string()),
                            ("total", &self.total.to_string()),
                        ],
                    ));
                    ui.add(
                        egui::ProgressBar::new(self.done as f32 / self.total.max(1) as f32)
                            .show_percentage(),
                    );
                    if self.cancel.load(Ordering::Relaxed) {
                        ui.label(tr(lang, "ocr.cancelling"));
                    }
                },
                |ui| {
                    cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                },
            );
            if cancel {
                self.cancel.store(true, Ordering::Relaxed);
            }
            return None;
        }
        if !self.open {
            return None;
        }
        let mut close = false;
        let mut run = false;
        dialog::show(
            ctx,
            Spec::new("ocr_options", tr(lang, "ocr.title"), dark, has_icons).width(480.0),
            |ui| {
                ui.label(tr(lang, "ocr.description"));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(tr(lang, "ocr.language"));
                    let name = |language| {
                        tr(
                            lang,
                            match language {
                                OcrLanguage::English => "ocr.english",
                                OcrLanguage::French => "ocr.french",
                                OcrLanguage::EnglishFrench => "ocr.both",
                            },
                        )
                    };
                    egui::ComboBox::from_id_salt("ocr_language")
                        .selected_text(name(self.language))
                        .show_ui(ui, |ui| {
                            for language in [
                                OcrLanguage::EnglishFrench,
                                OcrLanguage::English,
                                OcrLanguage::French,
                            ] {
                                ui.selectable_value(&mut self.language, language, name(language));
                            }
                        });
                });
                ui.radio_value(&mut self.all_pages, true, tr(lang, "ocr.all_pages"));
                ui.radio_value(&mut self.all_pages, false, tr(lang, "ocr.current_page"));
            },
            |ui| {
                run = ui
                    .add_enabled(
                        page_count > 0 && current_page < page_count,
                        egui::Button::new(tr(lang, "ocr.run")),
                    )
                    .clicked();
                close = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if close {
            self.open = false;
        }
        if run {
            self.open = false;
            Some(OcrOptions {
                pages: if self.all_pages {
                    (0..page_count).collect()
                } else {
                    vec![current_page]
                },
                language: self.language,
            })
        } else {
            None
        }
    }
}
