//! Édition de texte sur la page : champ de saisie posé à l'emplacement du texte, à la taille du
//! zoom, avec une mini-barre (taille, couleur, police). Sélecteur de police partagé : trois
//! polices standard et les polices TrueType du système (incorporées).

use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::editor::{family_key, TextFont, TextPrompt};
use crate::theme;
use crate::worker::Request;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Stroke};
use sunipdf_core::{FontFamily, SystemFont};
use sunipdf_i18n::{tr, tr_args, Language};

pub struct ParagraphDraft {
    pub page: usize,
    pub indices: Vec<usize>,
    pub epoch: u64,
    pub generation: u64,
    pub text: String,
    pub width: f32,
    pub leading: f32,
    pub preview: Option<egui::TextureHandle>,
    pub preview_objects: usize,
    pub busy: bool,
    pub applying: bool,
    pub error: Option<String>,
    focus: bool,
}

/// Libellé d'une police dans la liste.
pub fn font_label(lang: Language, font: &TextFont) -> String {
    match font {
        TextFont::Standard(f) => tr(lang, family_key(*f)).to_string(),
        TextFont::System(f) => f.name.clone(),
    }
}

/// Liste déroulante des polices, avec recherche (focus automatique à l'ouverture). Renvoie
/// (`true` si la liste des polices du système doit être demandée au moteur, `true` si la liste
/// vient de se fermer : rendre le focus au champ de texte).
pub fn font_picker(
    ui: &mut egui::Ui,
    lang: Language,
    font: &mut TextFont,
    filter: &mut String,
    fonts: Option<&[SystemFont]>,
) -> (bool, bool) {
    let mut wanted = false;
    let muted = ui.visuals().weak_text_color();
    let focus_id = ui.make_persistent_id("font-search-focus");
    let resp = egui::ComboBox::from_id_salt("text-font")
        .selected_text(font_label(lang, font))
        .width(190.0)
        .height(360.0)
        .show_ui(ui, |ui| {
            ui.set_min_width(250.0);
            let search = ui.add(
                egui::TextEdit::singleline(filter)
                    .hint_text(tr(lang, "text.font.search"))
                    .desired_width(f32::INFINITY),
            );
            let first = ui.ctx().data_mut(|d| {
                !std::mem::replace(d.get_temp_mut_or_default::<bool>(focus_id), true)
            });
            if first {
                search.request_focus();
            }
            if search.changed() {
                // La liste change de hauteur : une image de plus pour que la fenêtre suive.
                ui.ctx().request_repaint();
            }
            let query = filter.to_lowercase();
            let standard: Vec<FontFamily> = [
                FontFamily::Helvetica,
                FontFamily::Times,
                FontFamily::Courier,
            ]
            .into_iter()
            .filter(|f| {
                query.is_empty() || tr(lang, family_key(*f)).to_lowercase().contains(&query)
            })
            .collect();
            if !standard.is_empty() {
                ui.label(
                    RichText::new(tr(lang, "text.font.standard"))
                        .small()
                        .color(muted),
                );
                ui.horizontal(|ui| {
                    for f in standard {
                        ui.selectable_value(font, TextFont::Standard(f), tr(lang, family_key(f)));
                    }
                });
                ui.separator();
            }
            ui.label(
                RichText::new(tr(lang, "text.font.system"))
                    .small()
                    .color(muted),
            );
            // Hauteur fixe : la fenêtre de la liste déroulante garde sa taille de la première image
            // (chargement, filtre), une liste qui grandit après coup serait tronquée.
            egui::ScrollArea::vertical()
                .max_height(210.0)
                .auto_shrink([false, false])
                .show(ui, |ui| match fonts {
                    None => {
                        wanted = true;
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(tr(lang, "text.font.loading"));
                        });
                    }
                    Some(list) => {
                        let shown: Vec<&SystemFont> = list
                            .iter()
                            .filter(|f| query.is_empty() || f.name.to_lowercase().contains(&query))
                            .collect();
                        if shown.is_empty() {
                            ui.label(RichText::new(tr(lang, "text.font.none")).color(muted));
                        }
                        for f in shown {
                            let selected = matches!(font, TextFont::System(s) if s.path == f.path && s.face_index == f.face_index);
                            if ui.selectable_label(selected, &f.name).clicked() {
                                *font = TextFont::System(f.clone());
                            }
                        }
                    }
                });
        });
    // Liste fermée : oublier le focus de la recherche, signaler la fermeture une fois.
    let closed = resp.inner.is_none()
        && ui
            .ctx()
            .data_mut(|d| d.remove_temp::<bool>(focus_id).is_some());
    (wanted, closed)
}

impl SuniApp {
    fn paragraph_context_current(&mut self) -> bool {
        let current = self.edit.paragraph.as_ref().is_some_and(|draft| {
            self.document.is_some()
                && self.opening.is_none()
                && self.tabs.resuming.is_none()
                && draft.epoch == self.view.epoch
                && self.edit.selection.as_ref().is_some_and(|selection| {
                    selection.page == draft.page && selection.indices() == draft.indices
                })
        });
        if !current {
            let error = tr(self.lang(), "paragraph.stale").to_string();
            if let Some(draft) = self.edit.paragraph.as_mut() {
                draft.busy = false;
                draft.preview = None;
                draft.preview_objects = 0;
                draft.error = Some(error);
            }
        }
        current
    }

    pub fn open_paragraph_prompt(&mut self) {
        if self.document.is_none()
            || self.navigation_blocked()
            || self.worker.has_pending_mutations()
            || self.opening.is_some()
            || self.tabs.resuming.is_some()
            || self.edit.text_prompt.is_some()
        {
            return;
        }
        let Some(selection) = self.edit.selection.as_ref().filter(|s| {
            !s.objects.is_empty()
                && s.objects
                    .iter()
                    .all(|o| o.kind == sunipdf_core::ObjectKind::Text)
        }) else {
            return;
        };
        self.edit.paragraph_generation = self.edit.paragraph_generation.wrapping_add(1);
        self.edit.paragraph = Some(ParagraphDraft {
            page: selection.page,
            indices: selection.indices(),
            epoch: self.view.epoch,
            generation: self.edit.paragraph_generation,
            text: selection
                .objects
                .iter()
                .filter_map(|o| o.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n"),
            width: (selection.union_bounds().right - selection.union_bounds().left)
                .clamp(40.0, 1000.0),
            leading: 1.2,
            preview: None,
            preview_objects: 0,
            busy: false,
            applying: false,
            error: None,
            focus: true,
        });
    }

    pub fn request_paragraph_preview(&mut self) {
        if !self.paragraph_context_current() || self.worker.has_pending_mutations() {
            return;
        }
        let Some(draft) = self
            .edit
            .paragraph
            .as_mut()
            .filter(|d| !d.busy && !d.applying && d.epoch == self.view.epoch)
        else {
            return;
        };
        self.edit.paragraph_generation = self.edit.paragraph_generation.wrapping_add(1);
        draft.generation = self.edit.paragraph_generation;
        draft.busy = true;
        draft.preview = None;
        draft.error = None;
        self.worker.send(Request::PreviewParagraph {
            page: draft.page,
            indices: draft.indices.clone(),
            text: draft.text.clone(),
            width: draft.width,
            leading: draft.leading,
            epoch: draft.epoch,
            generation: draft.generation,
        });
    }

    pub fn on_paragraph_preview(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::paragraph::ParagraphPreview, sunipdf_core::EngineError>,
    ) {
        if !self.paragraph_context_current() || epoch != self.view.epoch {
            return;
        }
        let Some(draft) = self
            .edit
            .paragraph
            .as_mut()
            .filter(|d| d.epoch == epoch && d.generation == generation && d.busy)
        else {
            return;
        };
        if !self
            .edit
            .selection
            .as_ref()
            .is_some_and(|s| s.page == draft.page && s.indices() == draft.indices)
        {
            return;
        }
        draft.busy = false;
        match result {
            Ok(preview) => {
                draft.preview_objects = preview.objects;
                draft.preview = Some(ctx.load_texture(
                    "paragraph-preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [preview.image.width as usize, preview.image.height as usize],
                        &preview.image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(error) => draft.error = Some(error.to_string()),
        }
    }

    pub fn apply_paragraph_draft(&mut self) {
        if !self.paragraph_context_current() || self.worker.has_pending_mutations() {
            return;
        }
        let Some(draft) = self.edit.paragraph.as_mut().filter(|d| {
            d.preview.is_some() && !d.busy && !d.applying && d.epoch == self.view.epoch
        }) else {
            return;
        };
        if !self
            .edit
            .selection
            .as_ref()
            .is_some_and(|s| s.page == draft.page && s.indices() == draft.indices)
        {
            return;
        }
        draft.applying = true;
        let first = draft.indices[0];
        self.edit.reselect_many = Some((
            draft.page,
            crate::worker::Pick::Indices((first..first + draft.preview_objects).collect()),
        ));
        self.worker
            .send(Request::Edit(crate::worker::EditOp::ReflowParagraph {
                page: draft.page,
                indices: draft.indices.clone(),
                text: draft.text.clone(),
                width: draft.width,
                leading: draft.leading,
            }));
    }

    pub fn paragraph_dialog(&mut self, ctx: &egui::Context) {
        let context_current = self.paragraph_context_current();
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let pending_mutation = self.worker.has_pending_mutations();
        let Some(draft) = self.edit.paragraph.as_mut() else {
            return;
        };
        let mut preview = false;
        let mut apply = false;
        let mut cancel = false;
        let can_apply = context_current
            && draft.preview.is_some()
            && !draft.busy
            && !draft.applying
            && !pending_mutation;
        let can_preview = context_current
            && !pending_mutation
            && !draft.busy
            && !draft.applying
            && !draft.text.trim().is_empty();
        let can_cancel = !draft.applying;
        let spec = crate::dialog::Spec::new(
            "paragraph-editor",
            tr(lang, "paragraph.title"),
            dark,
            has_icons,
        )
        .subtitle(tr(lang, "paragraph.note"))
        .width(620.0);
        crate::dialog::show(
            ctx,
            spec,
            |ui| {
                let mut changed = false;
                ui.add_enabled_ui(!draft.busy && !draft.applying, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut draft.text)
                            .hint_text(tr(lang, "dialog.text.placeholder"))
                            .desired_rows(5)
                            .desired_width(f32::INFINITY),
                    );
                    changed |= response.changed();
                    if draft.focus {
                        response.request_focus();
                        draft.focus = false;
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr(lang, "paragraph.width"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut draft.width)
                                    .range(1.0..=1000.0)
                                    .suffix(" pt"),
                            )
                            .changed();
                        ui.label(tr(lang, "paragraph.leading"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut draft.leading)
                                    .range(1.0..=3.0)
                                    .speed(0.1),
                            )
                            .changed();
                    });
                });
                if changed {
                    draft.preview = None;
                    draft.preview_objects = 0;
                    draft.error = None;
                }
                if draft.busy || draft.applying {
                    ui.spinner();
                }
                if let Some(error) = &draft.error {
                    ui.colored_label(theme::palette_for(ctx).error, error);
                }
                if let Some(texture) = &draft.preview {
                    ui.add(
                        egui::Image::new(texture).fit_to_exact_size(
                            texture.size_vec2()
                                * (560.0 / texture.size_vec2().x)
                                    .min(260.0 / texture.size_vec2().y)
                                    .min(1.0),
                        ),
                    );
                }
            },
            |ui| {
                ui.add_enabled_ui(can_apply, |ui| {
                    if crate::dialog::primary(ui, tr(lang, "dialog.text.ok")) {
                        apply = true;
                    }
                });
                ui.add_enabled_ui(can_preview, |ui| {
                    if crate::dialog::secondary(ui, tr(lang, "paragraph.preview")) {
                        preview = true;
                    }
                });
                ui.add_enabled_ui(can_cancel, |ui| {
                    if crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|i| i.key_pressed(Key::Escape))
                    {
                        cancel = true;
                    }
                });
            },
        );
        if cancel {
            self.edit.paragraph = None;
            self.edit.paragraph_generation = self.edit.paragraph_generation.wrapping_add(1);
        } else if preview {
            self.request_paragraph_preview();
        } else if apply {
            self.apply_paragraph_draft();
        }
    }

    /// Éditeur de texte posé sur la page (nouveau texte, ou texte existant droit à l'écran).
    pub fn inline_text_editor(
        &mut self,
        ctx: &egui::Context,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let lang = self.lang();
        let p = theme::palette_for(ctx);
        let has_icons = self.has_icons;
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let fonts = self.edit.system_fonts.clone();
        let mut want_fonts = false;
        let Some(prompt) = self
            .edit
            .text_prompt
            .as_mut()
            .filter(|t| t.inline && t.page == page_index)
        else {
            return;
        };
        let size_px = (prompt.size * k).max(6.0);
        // Haut de la rangée de texte de l'éditeur (points) : au-dessus du haut des glyphes d'un
        // texte existant (marge de la police de l'interface), ou au-dessus de la ligne de base
        // cliquée pour un nouveau texte. Calibré sur capture (Segoe UI).
        let top_pt = match prompt.index {
            Some(_) => prompt.y + prompt.size * 0.3,
            None => prompt.y + prompt.size * 0.92,
        };
        let margin = egui::vec2(4.0, 2.0);
        let anchor = page_rect.min + egui::vec2(prompt.x * k, (page_h_pt - top_pt) * k) - margin;
        let family = match &prompt.font {
            TextFont::Standard(FontFamily::Courier) => egui::FontFamily::Monospace,
            _ => egui::FontFamily::Proportional,
        };
        let font_id = egui::FontId::new(size_px, family);
        let color = egui::Color32::from_rgb(prompt.rgb[0], prompt.rgb[1], prompt.rgb[2]);
        let text_w = ctx.fonts_mut(|f| {
            prompt
                .text
                .lines()
                .map(|l| {
                    f.layout_no_wrap(l.to_string(), font_id.clone(), color)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        });
        let width = (text_w + size_px * 1.5).max(140.0);
        let editor = egui::Area::new(egui::Id::unique("inline-text"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_white_alpha(250))
                    .stroke(Stroke::new(1.5, theme::BLUE))
                    .corner_radius(3)
                    .show(ui, |ui| {
                        // Largeur imposée : sans elle, la zone se limite à sa taille précédente et
                        // le texte passe à la ligne en cours de saisie.
                        ui.set_min_width(width);
                        let edit = ui.add(
                            egui::TextEdit::multiline(&mut prompt.text)
                                .font(font_id.clone())
                                .text_color(color)
                                .frame(egui::Frame::new())
                                .margin(egui::Margin::symmetric(4, 2))
                                .desired_width(width)
                                .desired_rows(1)
                                .hint_text(
                                    RichText::new(tr(lang, "dialog.text.placeholder"))
                                        .color(egui::Color32::from_gray(150)),
                                )
                                .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)),
                        );
                        if prompt.focus {
                            edit.request_focus();
                            prompt.focus = false;
                        }
                        edit
                    })
                    .inner
            });
        let edit_rect = editor.response.rect;
        // Mini-barre au-dessus du champ (hauteur connue depuis l'image précédente).
        let bar_id = egui::Id::unique("inline-text-bar");
        let bar_h = ctx
            .memory(|m| m.area_rect(bar_id))
            .map_or(64.0, |r| r.height());
        let bar = egui::Area::new(bar_id)
            .order(egui::Order::Foreground)
            .fixed_pos(edit_rect.left_top() - egui::vec2(0.0, bar_h + 6.0))
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 2],
                        blur: 10,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(36),
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(460.0);
                        ui.horizontal(|ui| {
                            ui.label(crate::icons::icon(
                                has_icons,
                                crate::icons::glyph::FONT,
                                "T",
                                14.0,
                            ));
                            ui.add(
                                egui::DragValue::new(&mut prompt.size)
                                    .range(4.0..=200.0)
                                    .speed(0.5)
                                    .suffix(" pt"),
                            )
                            .on_hover_text(tr(lang, "dialog.text.size"));
                            ui.color_edit_button_srgb(&mut prompt.rgb)
                                .on_hover_text(tr(lang, "dialog.text.color"));
                            if prompt.index.is_none() {
                                let (wanted, closed) = font_picker(
                                    ui,
                                    lang,
                                    &mut prompt.font,
                                    &mut self.edit.font_filter,
                                    fonts.as_deref().map(|v| v.as_slice()),
                                );
                                want_fonts = wanted;
                                if closed {
                                    prompt.focus = true;
                                }
                                let standard = matches!(prompt.font, TextFont::Standard(_));
                                ui.add_enabled_ui(standard, |ui| {
                                    ui.toggle_value(
                                        &mut prompt.bold,
                                        crate::icons::semibold("B", 14.0).color(p.text),
                                    )
                                    .on_hover_text(tr(lang, "dialog.text.bold"))
                                    .on_disabled_hover_text(tr(lang, "text.font.style_note"));
                                    ui.toggle_value(
                                        &mut prompt.italic,
                                        RichText::new("I").italics().size(14.0).color(p.text),
                                    )
                                    .on_hover_text(tr(lang, "dialog.text.italic"))
                                    .on_disabled_hover_text(tr(lang, "text.font.style_note"));
                                });
                            }
                        });
                        // Aide et note de police sur une seconde ligne : la barre reste étroite.
                        let note = match (&prompt.font, prompt.index) {
                            (_, Some(_)) => Some(tr(lang, "dialog.text.font_note").to_string()),
                            (TextFont::System(f), None) => Some(tr_args(
                                lang,
                                "text.font.embed_note",
                                &[("size", &format!("{:.1}", f.size_bytes as f32 / 1_048_576.0))],
                            )),
                            _ => None,
                        };
                        let small = |s: &str| RichText::new(s).size(11.5).color(p.text_muted);
                        ui.label(small(tr(lang, "text.inline.hint")));
                        if let Some(n) = note {
                            ui.add(egui::Label::new(small(&n)).wrap());
                        }
                    });
            });
        let bar_rect = bar.response.rect;
        let (enter, escape, pressed_outside, on_page) = ctx.input(|i| {
            let pos = i.pointer.interact_pos().filter(|_| i.pointer.any_pressed());
            (
                i.key_pressed(Key::Enter) && !i.modifiers.shift,
                i.key_pressed(Key::Escape),
                pos.is_some_and(|p| !edit_rect.contains(p) && !bar_rect.contains(p)),
                pos.is_some_and(|p| page_rect.contains(p)),
            )
        });
        let commit_click = prompt.armed && pressed_outside && !egui::Popup::is_any_open(ctx);
        prompt.armed = true;
        if want_fonts && !self.edit.fonts_requested {
            self.edit.fonts_requested = true;
            self.worker.send(Request::SystemFonts);
        }
        if escape && !egui::Popup::is_any_open(ctx) {
            self.edit.text_prompt = None;
        } else if enter || commit_click {
            let prompt: TextPrompt = self.edit.text_prompt.take().expect("prompt");
            // Le clic sur la page qui valide ne doit pas ouvrir un autre éditeur (outil Texte).
            self.edit.swallow_click =
                commit_click && on_page && self.annotate.tool == crate::annotate::Tool::AddText;
            self.commit_text_prompt(prompt);
        }
    }
}

#[cfg(test)]
mod paragraph_tests {
    use super::*;
    use crate::worker::{EditOp, Response};
    use sunipdf_core::{
        Command, EngineError, ObjectInfo, ObjectKind, PixelRect, PointRect, RenderedPage,
    };

    fn selection() -> crate::editor::Selection {
        crate::editor::Selection {
            page: 0,
            objects: vec![ObjectInfo {
                index: 2,
                kind: ObjectKind::Text,
                bounds: PointRect {
                    left: 20.0,
                    bottom: 100.0,
                    right: 160.0,
                    top: 120.0,
                },
                quad: [(20.0, 100.0), (160.0, 100.0), (160.0, 120.0), (20.0, 120.0)],
                angle_deg: 25.0,
                opacity: Some(0.6),
                text: Some("Original text".into()),
                font_size: 16.0,
                font_name: Some("Original PDF font".into()),
                fill_rgb: Some((20, 50, 90)),
                stroke_rgb: None,
                stroke_width: 0.0,
            }],
        }
    }

    fn preview() -> sunipdf_core::paragraph::ParagraphPreview {
        sunipdf_core::paragraph::ParagraphPreview {
            image: RenderedPage {
                page_index: 0,
                width: 4,
                height: 2,
                rgba: vec![255; 32],
            },
            scale: 2.0,
            region: PixelRect::new(0, 0, 4, 2),
            objects: 2,
        }
    }

    #[test]
    fn paragraph_preview_requires_current_document_and_keeps_draft_on_refusal() {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.edit.selection = Some(selection());
        app.open_paragraph_prompt();
        assert!(app.has_unsaved_changes() && app.navigation_blocked());
        for command in [Command::Save, Command::OpenDocument, Command::Quit] {
            app.execute(&ctx, command);
        }
        assert!(
            requests.try_recv().is_err(),
            "draft blocks unrelated commands"
        );
        app.edit.paragraph.as_mut().unwrap().text = "Edited paragraph words".into();
        app.apply_paragraph_draft();
        assert!(
            requests.try_recv().is_err(),
            "Apply requires a successful preview"
        );
        app.request_paragraph_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewParagraph {
                page: 0,
                indices,
                text,
                epoch,
                generation,
                ..
            } if indices == [2] && text == "Edited paragraph words" => (epoch, generation),
            _ => panic!("paragraph preview request"),
        };
        app.on_paragraph_preview(&ctx, epoch, generation.wrapping_sub(1), Ok(preview()));
        assert!(app.edit.paragraph.as_ref().unwrap().busy);
        app.on_paragraph_preview(
            &ctx,
            epoch,
            generation,
            Err(EngineError::MissingGlyphs("🚀".into())),
        );
        let draft = app.edit.paragraph.as_ref().unwrap();
        assert!(!draft.busy && draft.preview.is_none() && draft.error.is_some());
        assert_eq!(draft.text, "Edited paragraph words");
        app.request_paragraph_preview();
        let generation = match requests.recv().unwrap() {
            Request::PreviewParagraph { generation, .. } => generation,
            _ => panic!("retry"),
        };
        app.on_paragraph_preview(&ctx, epoch.wrapping_add(1), generation, Ok(preview()));
        assert!(app.edit.paragraph.as_ref().unwrap().busy);
        responses
            .send(Response::ParagraphPreview {
                epoch,
                generation,
                result: Ok(preview()),
            })
            .unwrap();
        app.handle_responses(&ctx);
        assert!(app.edit.paragraph.as_ref().unwrap().preview.is_some());
        app.apply_paragraph_draft();
        assert!(
            matches!(requests.recv().unwrap(),Request::Edit(EditOp::ReflowParagraph {page:0,indices,text,..}) if indices==[2] && text=="Edited paragraph words")
        );
        assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
        assert_eq!(
            app.edit.reselect_many,
            Some((0, crate::worker::Pick::Indices(vec![2, 3])))
        );
        responses
            .send(Response::OperationFailed(EngineError::Other(
                "refused".into(),
            )))
            .unwrap();
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        let draft = app.edit.paragraph.as_ref().unwrap();
        assert!(!draft.applying && draft.preview.is_none());
        assert_eq!(draft.text, "Edited paragraph words");
        app.view.bump_epoch();
        app.request_paragraph_preview();
        app.on_paragraph_preview(&ctx, epoch, generation, Ok(preview()));
        assert!(requests.try_recv().is_err());
        assert!(app.edit.paragraph.as_ref().unwrap().preview.is_none());
        assert!(!app.edit.paragraph.as_ref().unwrap().busy);
        assert!(app.edit.paragraph.as_ref().unwrap().error.is_some());
        app.edit.reset_for_tab();
        app.on_paragraph_preview(&ctx, epoch, generation, Ok(preview()));
        assert!(app.edit.paragraph.is_none());

        app.edit.selection = Some(selection());
        app.open_paragraph_prompt();
        app.request_paragraph_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewParagraph {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("new document preview"),
        };
        app.edit.selection.as_mut().unwrap().objects[0].index = 3;
        app.on_paragraph_preview(&ctx, epoch, generation, Ok(preview()));
        let draft = app.edit.paragraph.as_ref().unwrap();
        assert!(!draft.busy && draft.preview.is_none() && draft.error.is_some());
        assert_eq!(draft.text, "Original text");
        app.apply_paragraph_draft();
        app.request_paragraph_preview();
        assert!(
            requests.try_recv().is_err(),
            "changed selection cannot preview or apply"
        );
        app.edit.selection = Some(selection());
        app.view.bump_epoch();
        app.on_paragraph_preview(&ctx, epoch, generation, Ok(preview()));
        assert!(
            app.edit.paragraph.as_ref().unwrap().preview.is_none(),
            "same IDs after Undo are still stale"
        );
    }

    #[test]
    fn paragraph_dialog_is_accessible_in_both_languages_and_escape_keeps_original_selection() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            let _ = crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.language = language.into();
            app.edit.selection = Some(selection());
            app.open_paragraph_prompt();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 780.0),
                )),
                ..Default::default()
            };
            let mut found = false;
            for _ in 0..2 {
                let mut out = ctx.run_ui(input.clone(), |_| app.paragraph_dialog(&ctx));
                if let Some(tree) = &out.platform_output.accesskit_update {
                    found |= tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(tr(app.lang(), "paragraph.preview")));
                }
                out.textures_delta.clear();
            }
            assert!(found);
            let mut input = input;
            input.events.push(egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            });
            let mut out = ctx.run_ui(input, |_| app.paragraph_dialog(&ctx));
            out.textures_delta.clear();
            assert!(app.edit.paragraph.is_none());
            assert_eq!(app.edit.selection.as_ref(), Some(&selection()));
            assert!(requests.try_recv().is_err());
        }
    }
}
