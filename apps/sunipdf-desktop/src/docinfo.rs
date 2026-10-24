//! Signets, propriétés du document, protection par mot de passe.

use crate::app::SuniApp;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{BytesOp, EditOp, Request};
use egui::{Align, Button, Frame, Layout, RichText, Stroke};
use sunipdf_core::{Bookmark, Command, Metadata, Properties, Protection, ProtectionProblem};
use sunipdf_i18n::{tr, tr_args, Language};

#[derive(Default)]
pub struct DocInfoState {
    pub bookmarks_open: bool,
    pub bookmarks: Vec<Bookmark>,
    pub bookmark_selected: Option<usize>,
    pub bookmark_title: String,
    pub bookmark_page: usize,
    pub bookmark_child: bool,
    pub properties: Option<Properties>,
    pub properties_open: bool,
    pub inspection_epoch: Option<u64>,
    pub inspection: Option<Result<sunipdf_core::inspection::Inspection, String>>,
    pub metadata_edit: Metadata,
    pub protect_open: bool,
    pub protection: Protection,
    pub protect_error: Option<&'static str>,
    pub protection_epoch: Option<u64>,
    pub current_owner: Option<zeroize::Zeroizing<String>>,
    pub unprotect_open: bool,
    pub unprotect_password: String,
    pub unprotect_for_reprotect: bool,
}

/// Taille lisible, unités et séparateur décimal du catalogue (« 2,3 Mo » / « 2.3 MB »).
pub fn human_size(lang: Language, bytes: u64) -> String {
    if bytes >= 1_048_576 {
        let value = format!("{:.1}", bytes as f64 / 1_048_576.0)
            .replace('.', tr(lang, "unit.decimal_separator"));
        tr_args(lang, "unit.mb", &[("value", &value)])
    } else if bytes >= 1024 {
        let value = format!("{:.0}", bytes as f64 / 1024.0);
        tr_args(lang, "unit.kb", &[("value", &value)])
    } else {
        tr_args(lang, "unit.bytes", &[("value", &bytes.to_string())])
    }
}

/// Clé du message d'une protection refusée.
fn protection_error_key(p: ProtectionProblem) -> &'static str {
    match p {
        ProtectionProblem::NoPassword => "protect.error_empty",
        ProtectionProblem::OwnerNeeded => "protect.error_owner_needed",
        ProtectionProblem::NotLatin1 => "protect.error_latin1",
    }
}

impl SuniApp {
    pub(crate) fn inspection_received(
        &mut self,
        epoch: u64,
        result: Result<sunipdf_core::inspection::Inspection, sunipdf_core::EngineError>,
    ) {
        if self.docinfo.properties_open
            && self.docinfo.inspection_epoch == Some(epoch)
            && self.view.epoch == epoch
            && self.document.is_some()
            && !self.dirty
        {
            self.docinfo.inspection = Some(result.map_err(|error| error.code().to_owned()));
        }
    }

    pub(crate) fn protection_authorized(&mut self, epoch: u64, encrypted: bool) {
        if self.docinfo.protection_epoch != Some(epoch) || self.view.epoch != epoch {
            return;
        }
        self.docinfo.protection_epoch = None;
        if encrypted {
            self.docinfo.unprotect_password.clear();
            self.docinfo.unprotect_for_reprotect = true;
            self.docinfo.unprotect_open = true;
        } else {
            self.docinfo.protect_open = true;
        }
    }

    pub fn bookmarks_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette_for(ctx);
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
        self.bookmark_author(ui);
        if self.docinfo.bookmarks.is_empty() {
            ui.label(RichText::new(tr(lang, "bookmarks.empty")).color(p.text_muted));
            return;
        }
        let current = self.document.as_ref().map(|d| d.page);
        let mut go = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, b) in self.docinfo.bookmarks.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0 + b.depth as f32 * 14.0);
                        let active = b.page.is_some() && b.page == current;
                        let text = if active {
                            icons::semibold(&b.title, 13.0).color(p.accent)
                        } else {
                            RichText::new(&b.title).size(13.0)
                        };
                        let resp = ui.add(Button::new(text).frame(false));
                        let resp = match b.page {
                            Some(page) => resp.on_hover_text(tr_args(
                                lang,
                                "bookmarks.page",
                                &[("page", &(page + 1).to_string())],
                            )),
                            None => resp,
                        };
                        if resp.clicked() {
                            self.docinfo.bookmark_selected = Some(index);
                            self.docinfo.bookmark_title = b.title.clone();
                            self.docinfo.bookmark_page = b.page.map_or(1, |page| page + 1);
                            go = b.page.map(|page| (page, b.destination));
                        }
                    });
                }
            });
        if let Some((page, destination)) = go {
            match destination {
                Some(destination) => self.go_to_destination(destination),
                None if self.reading_navigation_ready() => self.go_to_page(page),
                None => {}
            }
        }
    }

    fn bookmark_author(&mut self, ui: &mut egui::Ui) {
        use sunipdf_core::bookmarks::BookmarkEdit;
        let lang = self.lang();
        let Some(doc) = &self.document else {
            return;
        };
        let count = doc.info.page_count();
        let current = doc.page;
        let selected = self
            .docinfo
            .bookmark_selected
            .filter(|index| *index < self.docinfo.bookmarks.len());
        self.docinfo.bookmark_selected = selected;
        if self.docinfo.bookmark_page == 0 || self.docinfo.bookmark_page > count {
            self.docinfo.bookmark_page = current + 1;
        }
        let mut operation = None;
        ui.collapsing(tr(lang, "bookmarks.edit"), |ui| {
            ui.label(tr(lang, "bookmarks.title_label"));
            ui.add(
                egui::TextEdit::singleline(&mut self.docinfo.bookmark_title)
                    .char_limit(1024)
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                ui.label(tr(lang, "bookmarks.target_page"));
                ui.add(egui::DragValue::new(&mut self.docinfo.bookmark_page).range(1..=count));
                if ui.button(tr(lang, "bookmarks.current_page")).clicked() {
                    self.docinfo.bookmark_page = current + 1;
                }
            });
            ui.add_enabled_ui(selected.is_some(), |ui| {
                ui.checkbox(
                    &mut self.docinfo.bookmark_child,
                    tr(lang, "bookmarks.as_child"),
                );
            });
            let ready = !self.docinfo.bookmark_title.trim().is_empty()
                && self.docinfo.bookmark_title.len() <= 1024;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        ready
                            && self.docinfo.bookmarks.len()
                                < sunipdf_core::bookmarks::MAX_BOOKMARKS,
                        Button::new(tr(lang, "bookmarks.add")),
                    )
                    .clicked()
                {
                    operation = Some(BookmarkEdit::Add {
                        parent: if self.docinfo.bookmark_child {
                            selected
                        } else {
                            None
                        },
                        title: self.docinfo.bookmark_title.clone(),
                        page: self.docinfo.bookmark_page - 1,
                    });
                }
                if let Some(index) = selected {
                    if ui
                        .add_enabled(ready, Button::new(tr(lang, "bookmarks.rename")))
                        .clicked()
                    {
                        operation = Some(BookmarkEdit::Rename {
                            index,
                            title: self.docinfo.bookmark_title.clone(),
                        });
                    }
                    if ui.button(tr(lang, "bookmarks.set_destination")).clicked() {
                        operation = Some(BookmarkEdit::Destination {
                            index,
                            page: self.docinfo.bookmark_page - 1,
                        });
                    }
                    let depth = self.docinfo.bookmarks[index].depth;
                    let parent = (0..index)
                        .rev()
                        .find(|i| self.docinfo.bookmarks[*i].depth < depth);
                    let siblings: Vec<_> = self
                        .docinfo
                        .bookmarks
                        .iter()
                        .enumerate()
                        .filter_map(|(i, item)| {
                            if item.depth != depth {
                                return None;
                            }
                            let own_parent = (0..i)
                                .rev()
                                .find(|p| self.docinfo.bookmarks[*p].depth < depth);
                            (own_parent == parent).then_some(i)
                        })
                        .collect();
                    let position = siblings.iter().position(|i| *i == index).unwrap_or(0);
                    if ui
                        .add_enabled(position > 0, Button::new(tr(lang, "bookmarks.up")))
                        .clicked()
                    {
                        operation = Some(BookmarkEdit::Move {
                            index,
                            parent,
                            position: position - 1,
                        });
                    }
                    if ui
                        .add_enabled(
                            position + 1 < siblings.len(),
                            Button::new(tr(lang, "bookmarks.down")),
                        )
                        .clicked()
                    {
                        operation = Some(BookmarkEdit::Move {
                            index,
                            parent,
                            position: position + 1,
                        });
                    }
                    if ui.button(tr(lang, "bookmarks.delete_subtree")).clicked() {
                        operation = Some(BookmarkEdit::Delete { index });
                    }
                }
            });
            ui.label(tr(lang, "bookmarks.edit_limits"));
        });
        if let Some(operation) = operation {
            if self.commit_form_input() {
                self.docinfo.bookmark_selected = None;
                self.worker
                    .send(Request::Edit(EditOp::Bytes(BytesOp::Bookmark(operation))));
            }
        }
    }

    pub fn properties_dialog(&mut self, ctx: &egui::Context) {
        if !self.docinfo.properties_open {
            return;
        }
        let lang = self.lang();
        let p = theme::palette_for(ctx);
        let mut apply = false;
        let mut close = false;
        let mut inspect = false;
        if self.docinfo.inspection_epoch != Some(self.view.epoch) || self.dirty {
            self.docinfo.inspection_epoch = None;
            self.docinfo.inspection = None;
        }
        let inspection = self.docinfo.inspection.clone();
        let inspecting = self.docinfo.inspection_epoch.is_some() && inspection.is_none();
        let can_inspect = self.document.is_some() && !self.dirty && !inspecting;
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
        let loaded = props_opt.is_some();
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
                                        .map(|b| human_size(lang, b))
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
                ui.add_space(8.0);
                ui.collapsing(tr(lang, "inspection.title"), |ui| {
                    ui.label(tr(lang, "inspection.scope"));
                    if self.dirty {
                        ui.label(tr(lang, "inspection.save_first"));
                    }
                    if ui
                        .add_enabled(can_inspect, Button::new(tr(lang, "inspection.run")))
                        .clicked()
                    {
                        inspect = true;
                    }
                    if inspecting {
                        ui.spinner();
                    }
                    if let Some(result) = &inspection {
                        match result {
                            Ok(report) => {
                                ui.label(tr(
                                    lang,
                                    if report.complete {
                                        "inspection.complete"
                                    } else {
                                        "inspection.incomplete"
                                    },
                                ));
                                egui::Grid::new("inspection-findings").num_columns(2).show(
                                    ui,
                                    |ui| {
                                        for finding in &report.findings {
                                            ui.label(tr(
                                                lang,
                                                &format!("inspection.{}", finding.kind),
                                            ));
                                            ui.label(finding.dictionaries.to_string());
                                            ui.end_row();
                                        }
                                    },
                                );
                                ui.label(tr_args(
                                    lang,
                                    "inspection.sha256",
                                    &[("hash", &report.source_sha256)],
                                ));
                            }
                            Err(_) => {
                                ui.label(tr(lang, "inspection.failed"));
                            }
                        }
                    }
                    ui.label(RichText::new(tr(lang, "inspection.limits")).color(p.text_muted));
                });
            },
            |ui| {
                // Avant l'arrivée des propriétés, « Appliquer » enverrait des champs vides et
                // effacerait titre, auteur, sujet et mots-clés.
                ui.add_enabled_ui(loaded, |ui| {
                    if crate::dialog::primary(ui, tr(lang, "properties.apply")) {
                        apply = true;
                    }
                });
                if crate::dialog::secondary(ui, tr(lang, "about.close"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            },
        );
        if inspect {
            self.docinfo.inspection_epoch = Some(self.view.epoch);
            self.docinfo.inspection = None;
            self.worker.send(Request::Inspect {
                epoch: self.view.epoch,
            });
        }
        if apply && loaded {
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
        let p = theme::palette_for(ctx);
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
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut pr.user_password)
                                .password(true)
                                .desired_width(300.0),
                        );
                        if ui.memory(|m| m.focused().is_none()) {
                            field.request_focus();
                        }
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
                    ui.colored_label(p.error, tr(lang, err));
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
            // Mêmes règles que le moteur (`Protection::validate`) : le message reste dans la boîte.
            match self.docinfo.protection.validate() {
                Err(problem) => self.docinfo.protect_error = Some(protection_error_key(problem)),
                Ok(()) => {
                    let protection = std::mem::take(&mut self.docinfo.protection);
                    self.docinfo.protect_open = false;
                    self.docinfo.protect_error = None;
                    self.worker
                        .send(Request::Edit(EditOp::Bytes(BytesOp::Protect {
                            protection,
                            current_owner: self.docinfo.current_owner.take(),
                        })));
                }
            }
        } else if close {
            self.docinfo.protect_open = false;
            self.docinfo.protect_error = None;
            self.docinfo.current_owner = None;
        }
    }

    pub fn unprotect_dialog(&mut self, ctx: &egui::Context) {
        if !self.docinfo.unprotect_open {
            return;
        }
        let lang = self.lang();
        let muted = theme::palette_for(ctx).text_muted;
        let mut submit = false;
        let mut close = false;
        let reprotect = self.docinfo.unprotect_for_reprotect;
        let spec = crate::dialog::Spec::new(
            "unprotect",
            tr(
                lang,
                if reprotect {
                    "protect.current_owner.title"
                } else {
                    "unprotect.title"
                },
            ),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(
            lang,
            if reprotect {
                "protect.current_owner.subtitle"
            } else {
                "unprotect.subtitle"
            },
        ))
        .icon(icons::glyph::UNLOCK, "🔓")
        .width(440.0);
        let password = &mut self.docinfo.unprotect_password;
        let mut enter = false;
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                crate::dialog::field_label(ui, tr(lang, "unprotect.label"));
                let field = ui.add(
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter = true;
                }
                ui.label(
                    RichText::new(tr(
                        lang,
                        if reprotect {
                            "protect.current_owner.hint"
                        } else {
                            "unprotect.hint"
                        },
                    ))
                    .size(12.5)
                    .color(muted),
                );
            },
            |ui| {
                if crate::dialog::primary(
                    ui,
                    tr(
                        lang,
                        if reprotect {
                            "protect.current_owner.ok"
                        } else {
                            "unprotect.ok"
                        },
                    ),
                ) {
                    submit = true;
                }
                if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            },
        );
        if submit || enter {
            let owner = std::mem::take(&mut self.docinfo.unprotect_password);
            self.docinfo.unprotect_open = false;
            self.docinfo.unprotect_for_reprotect = false;
            if reprotect {
                self.docinfo.current_owner = Some(zeroize::Zeroizing::new(owner));
                self.docinfo.protect_open = true;
            } else {
                self.worker
                    .send(Request::Edit(EditOp::Bytes(BytesOp::Unprotect(owner))));
            }
        } else if close {
            self.docinfo.unprotect_open = false;
            self.docinfo.unprotect_password.clear();
            self.docinfo.unprotect_for_reprotect = false;
            self.docinfo.current_owner = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_ignores_closed_changed_and_stale_document_replies() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let epoch = app.view.epoch;
        app.docinfo.inspection_epoch = Some(epoch);
        app.inspection_received(epoch, Err(sunipdf_core::EngineError::InvalidFormat));
        assert!(app.docinfo.inspection.is_none());
        app.docinfo.properties_open = true;
        app.inspection_received(
            epoch.wrapping_add(1),
            Err(sunipdf_core::EngineError::InvalidFormat),
        );
        assert!(app.docinfo.inspection.is_none());
        app.dirty = true;
        app.inspection_received(epoch, Err(sunipdf_core::EngineError::InvalidFormat));
        assert!(app.docinfo.inspection.is_none());
        app.dirty = false;
        let document = app.document.take();
        app.inspection_received(epoch, Err(sunipdf_core::EngineError::InvalidFormat));
        assert!(app.docinfo.inspection.is_none());
        app.document = document;
        app.inspection_received(epoch, Err(sunipdf_core::EngineError::InvalidFormat));
        assert_eq!(
            app.docinfo
                .inspection
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err(),
            "invalid_format"
        );
    }

    #[test]
    fn reprotection_prompts_for_owner_without_unprotecting_and_ignores_stale_replies() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let epoch = app.view.epoch;
        app.docinfo.protection_epoch = Some(epoch);
        app.protection_authorized(epoch.wrapping_add(1), true);
        assert!(!app.docinfo.unprotect_open && !app.docinfo.protect_open);
        app.protection_authorized(epoch, true);
        assert!(app.docinfo.unprotect_open && app.docinfo.unprotect_for_reprotect);
        assert!(!app.docinfo.protect_open);
        assert!(
            requests.try_recv().is_err(),
            "authorization never removes protection"
        );
        app.docinfo = Default::default();
        app.docinfo.protection_epoch = Some(epoch);
        app.protection_authorized(epoch, false);
        assert!(app.docinfo.protect_open && !app.docinfo.unprotect_open);
        app.protection_authorized(epoch, true);
        assert!(
            !app.docinfo.unprotect_open,
            "a consumed reply cannot reopen another prompt"
        );
    }

    #[test]
    fn sizes_follow_the_catalog_units_and_decimal_separator() {
        assert_eq!(human_size(Language::French, 2_411_724), "2,3 Mo");
        assert_eq!(human_size(Language::English, 2_411_724), "2.3 MB");
        assert_eq!(human_size(Language::French, 812 * 1024), "812 Ko");
        assert_eq!(human_size(Language::English, 812 * 1024), "812 KB");
        assert_eq!(human_size(Language::French, 900), "900 o");
        assert_eq!(human_size(Language::English, 900), "900 B");
    }

    #[test]
    fn every_protection_problem_has_a_message() {
        for p in [
            ProtectionProblem::NoPassword,
            ProtectionProblem::OwnerNeeded,
            ProtectionProblem::NotLatin1,
        ] {
            let key = protection_error_key(p);
            for lang in Language::ALL {
                assert_ne!(tr(lang, key), key, "{key} en {}", lang.code());
            }
        }
    }
}
