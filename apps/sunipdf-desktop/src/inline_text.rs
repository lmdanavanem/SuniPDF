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
                            let selected = matches!(font, TextFont::System(s) if s.path == f.path);
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
    /// Éditeur de texte posé sur la page (nouveau texte, ou texte existant droit à l'écran).
    pub fn inline_text_editor(
        &mut self,
        ctx: &egui::Context,
        page_rect: egui::Rect,
        page_index: usize,
        page_h_pt: f32,
    ) {
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let p = theme::palette(dark);
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
        // Point haut-gauche du texte : limites de l'objet existant, ou ligne de base cliquée.
        let top_pt = match prompt.index {
            Some(_) => prompt.y + prompt.size * 1.0,
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
        let editor = egui::Area::new(egui::Id::new("inline-text"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_white_alpha(235))
                    .stroke(Stroke::new(1.5, theme::BLUE))
                    .corner_radius(3)
                    .show(ui, |ui| {
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
        let bar_id = egui::Id::new("inline-text-bar");
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
        let (enter, escape, pressed_outside) = ctx.input(|i| {
            let outside = i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !edit_rect.contains(pos) && !bar_rect.contains(pos));
            (
                i.key_pressed(Key::Enter) && !i.modifiers.shift,
                i.key_pressed(Key::Escape),
                outside,
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
            // Le clic qui valide ne doit pas ouvrir un autre éditeur (outil Texte).
            self.edit.swallow_click = commit_click;
            self.commit_text_prompt(prompt);
        }
    }
}
