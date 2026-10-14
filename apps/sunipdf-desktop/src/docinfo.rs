//! Signets, propriétés du document, protection par mot de passe.

use crate::app::SuniApp;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{BytesOp, EditOp, Request};
use egui::{Align, Button, Frame, Layout, RichText, Stroke};
use sunipdf_core::{Bookmark, Command, Metadata, Properties, Protection};
use sunipdf_i18n::{tr, tr_args};

#[derive(Default)]
pub struct DocInfoState {
    pub bookmarks_open: bool,
    pub bookmarks: Vec<Bookmark>,
    pub properties: Option<Properties>,
    pub properties_open: bool,
    pub metadata_edit: Metadata,
    pub protect_open: bool,
    pub protection: Protection,
    pub protect_error: Option<&'static str>,
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} Mo", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} Ko", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} o")
    }
}

impl SuniApp {
    pub fn bookmarks_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, "bookmarks.title"), 15.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        Button::new(icons::icon(self.has_icons, glyph::CLOSE, "✕", 14.0))
                            .frame(false),
                    )
                    .on_hover_text(tr(lang, "tools.close"))
                    .clicked()
                {
                    self.execute(ctx, Command::ToggleBookmarks);
                }
            });
        });
        ui.separator();
        if self.docinfo.bookmarks.is_empty() {
            ui.label(RichText::new(tr(lang, "bookmarks.empty")).color(p.text_muted));
            return;
        }
        let current = self.document.as_ref().map(|d| d.page);
        let mut go = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for b in &self.docinfo.bookmarks {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0 + b.depth as f32 * 14.0);
                        let active = b.page.is_some() && b.page == current;
                        let text = if active {
                            icons::semibold(&b.title, 13.0).color(p.accent)
                        } else {
                            RichText::new(&b.title).size(13.0)
                        };
                        let resp = ui.add_enabled(b.page.is_some(), Button::new(text).frame(false));
                        let resp = match b.page {
                            Some(page) => resp.on_hover_text(tr_args(
                                lang,
                                "bookmarks.page",
                                &[("page", &(page + 1).to_string())],
                            )),
                            None => resp,
                        };
                        if resp.clicked() {
                            go = b.page;
                        }
                    });
                }
            });
        if let Some(page) = go {
            self.go_to_page(page);
        }
    }

    pub fn properties_dialog(&mut self, ctx: &egui::Context) {
        if !self.docinfo.properties_open {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let mut apply = false;
        let mut close = false;
        let spec = crate::dialog::Spec::new(
            "properties",
            tr(lang, "properties.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "properties.subtitle"))
        .icon(icons::glyph::INFO, "i")
        .width(560.0);
        let props_opt = self.docinfo.properties.clone();
        let edit = &mut self.docinfo.metadata_edit;
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                let Some(props) = props_opt else {
                    ui.spinner();
                    return;
                };
                egui::Grid::new("props-edit")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        for (key, field) in [
                            ("properties.doc_title", &mut edit.title),
                            ("properties.author", &mut edit.author),
                            ("properties.subject", &mut edit.subject),
                            ("properties.keywords", &mut edit.keywords),
                        ] {
                            ui.label(tr(lang, key));
                            let mut text = field.clone().unwrap_or_default();
                            if ui
                                .add(egui::TextEdit::singleline(&mut text).desired_width(360.0))
                                .changed()
                            {
                                *field = if text.trim().is_empty() {
                                    None
                                } else {
                                    Some(text)
                                };
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                Frame::new()
                    .fill(p.chrome)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        egui::Grid::new("props-info")
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                let muted =
                                    |t: String| RichText::new(t).color(p.text_muted).size(13.0);
                                let row = |ui: &mut egui::Ui, key: &str, value: String| {
                                    ui.label(RichText::new(tr(lang, key)).size(13.0));
                                    ui.label(muted(value));
                                    ui.end_row();
                                };
                                row(ui, "properties.pages", props.page_count.to_string());
                                row(ui, "properties.version", format!("PDF {}", props.version));
                                row(
                                    ui,
                                    "properties.size",
                                    props
                                        .file_size
                                        .map(human_size)
                                        .unwrap_or_else(|| "—".into()),
                                );
                                row(
                                    ui,
                                    "properties.creator",
                                    props.creator.clone().unwrap_or_else(|| "—".into()),
                                );
                                row(
                                    ui,
                                    "properties.producer",
                                    props.producer.clone().unwrap_or_else(|| "—".into()),
                                );
                                row(
                                    ui,
                                    "properties.created",
                                    props.created.clone().unwrap_or_else(|| "—".into()),
                                );
                                row(
                                    ui,
                                    "properties.modified",
                                    props.modified.clone().unwrap_or_else(|| "—".into()),
                                );
                                let yes = tr(lang, "properties.yes");
                                let no = tr(lang, "properties.no");
                                let yn = |b: bool| if b { yes } else { no }.to_string();
                                row(ui, "properties.encrypted", yn(props.encrypted));
                                row(ui, "properties.perm_print", yn(props.can_print));
                                row(ui, "properties.perm_copy", yn(props.can_copy));
                                row(ui, "properties.perm_modify", yn(props.can_modify));
                                row(ui, "properties.perm_annotate", yn(props.can_annotate));
                            });
                    });
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "properties.apply")) {
                    apply = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "about.close"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            },
        );
        if apply {
            let meta = self.docinfo.metadata_edit.clone();
            self.worker
                .send(Request::Edit(EditOp::Bytes(BytesOp::SetMetadata(meta))));
            self.docinfo.properties_open = false;
        } else if close {
            self.docinfo.properties_open = false;
        }
    }

    pub fn protect_dialog(&mut self, ctx: &egui::Context) {
        if !self.docinfo.protect_open {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let mut submit = false;
        let mut close = false;
        let spec = crate::dialog::Spec::new(
            "protect",
            tr(lang, "protect.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "protect.subtitle"))
        .icon(icons::glyph::SHIELD, "🔒")
        .width(500.0);
        let protect_error = self.docinfo.protect_error;
        let pr = &mut self.docinfo.protection;
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                ui.label(
                    RichText::new(tr(lang, "protect.intro"))
                        .color(p.text_muted)
                        .size(13.0),
                );
                ui.add_space(8.0);
                egui::Grid::new("protect-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(tr(lang, "protect.user_password"));
                        ui.add(
                            egui::TextEdit::singleline(&mut pr.user_password)
                                .password(true)
                                .desired_width(300.0),
                        );
                        ui.end_row();
                        ui.label(tr(lang, "protect.owner_password"));
                        ui.add(
                            egui::TextEdit::singleline(&mut pr.owner_password)
                                .password(true)
                                .desired_width(300.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.label(icons::semibold(tr(lang, "protect.permissions"), 13.0));
                ui.checkbox(&mut pr.allow_print, tr(lang, "protect.allow_print"));
                ui.checkbox(&mut pr.allow_copy, tr(lang, "protect.allow_copy"));
                ui.checkbox(&mut pr.allow_modify, tr(lang, "protect.allow_modify"));
                ui.checkbox(&mut pr.allow_annotate, tr(lang, "protect.allow_annotate"));
                ui.add_space(4.0);
                ui.checkbox(&mut pr.aes256, tr(lang, "protect.aes256"));
                if let Some(err) = protect_error {
                    ui.colored_label(theme::ERROR, tr(lang, err));
                }
            },
            |ui| {
                if crate::dialog::primary(ui, tr(lang, "protect.apply")) {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            },
        );
        if submit {
            let pr = &self.docinfo.protection;
            if pr.user_password.is_empty() && pr.owner_password.is_empty() {
                self.docinfo.protect_error = Some("protect.error_empty");
            } else {
                let protection = pr.clone();
                self.docinfo.protect_open = false;
                self.docinfo.protect_error = None;
                self.docinfo.protection = Protection::default();
                self.worker
                    .send(Request::Edit(EditOp::Bytes(BytesOp::Protect(protection))));
            }
        } else if close {
            self.docinfo.protect_open = false;
            self.docinfo.protect_error = None;
        }
    }
}
