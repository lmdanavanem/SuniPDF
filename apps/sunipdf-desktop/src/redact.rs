//! Outil « Caviarder » : zones marquées à la souris ou par recherche de texte, confirmation, garde
//! d'enregistrement. Les zones sont un état d'interface : rien n'est supprimé avant « Appliquer »,
//! qui envoie `Request::Redact` au thread moteur (suppression réelle, vérifiée, annulable).

use crate::annotate::{Drag, Tool};
use crate::app::{SuniApp, SCREEN_PER_PDF_POINT};
use crate::dialog;
use crate::icons::glyph;
use crate::theme;
use crate::worker::Request;
use egui::{Key, Modifiers, Stroke};
use std::collections::BTreeSet;
use sunipdf_core::{Command, PointRect, RedactMark, RedactOptions, RedactReport};
use sunipdf_i18n::{tr, tr_args};

/// Côté minimal d'une zone tracée (points) : en dessous, c'est un clic.
const MIN_SIDE_PT: f32 = 2.0;

pub struct RedactState {
    /// Zones non appliquées (espace d'affichage, comme l'API du moteur).
    pub marks: Vec<RedactMark>,
    pub search_open: bool,
    pub search_query: String,
    pub search_match_case: bool,
    /// Dernier texte marqué : recherché à nouveau après application (contrôle).
    pub last_query: Option<(String, bool)>,
    pub confirm_open: bool,
    pub rasterize: bool,
    pub clean_hidden: bool,
    pub applying: bool,
    /// Enregistrement demandé avec des zones non appliquées : `Some(ask_path)`.
    pub save_prompt: Option<bool>,
    /// « Enregistrer sans caviarder » choisi : le prochain enregistrement passe la garde.
    pub save_confirmed: bool,
}

impl Default for RedactState {
    fn default() -> Self {
        Self {
            marks: Vec::new(),
            search_open: false,
            search_query: String::new(),
            search_match_case: false,
            last_query: None,
            confirm_open: false,
            rasterize: false,
            clean_hidden: true,
            applying: false,
            save_prompt: None,
            save_confirmed: false,
        }
    }
}

impl RedactState {
    pub fn dialog_open(&self) -> bool {
        self.search_open || self.confirm_open || self.save_prompt.is_some()
    }

    pub fn page_count(&self) -> usize {
        self.marks
            .iter()
            .map(|m| m.page)
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Ajoute des zones sans doublon exact ; renvoie le nombre ajouté.
    pub fn add(&mut self, marks: impl IntoIterator<Item = RedactMark>) -> usize {
        let before = self.marks.len();
        for m in marks {
            if !self.marks.contains(&m) {
                self.marks.push(m);
            }
        }
        self.marks.len() - before
    }

    /// Retire la zone la plus récente qui contient le point ; `true` si une zone a été retirée.
    pub fn remove_at(&mut self, page: usize, pt: (f32, f32)) -> bool {
        let hit = self.marks.iter().rposition(|m| {
            m.page == page
                && (m.rect.left..=m.rect.right).contains(&pt.0)
                && (m.rect.bottom..=m.rect.top).contains(&pt.1)
        });
        hit.map(|i| self.marks.remove(i)).is_some()
    }
}

/// Rectangle tracé (affichage), borné à la page ; `None` s'il est trop petit.
pub fn drawn_rect(rect: PointRect, page_w: f32, page_h: f32) -> Option<PointRect> {
    let r = PointRect {
        left: rect.left.clamp(0.0, page_w),
        right: rect.right.clamp(0.0, page_w),
        bottom: rect.bottom.clamp(0.0, page_h),
        top: rect.top.clamp(0.0, page_h),
    };
    (r.right - r.left >= MIN_SIDE_PT && r.top - r.bottom >= MIN_SIDE_PT).then_some(r)
}

impl SuniApp {
    pub(crate) fn redact_status(&mut self) {
        let r = &self.annotate.redact;
        self.status_msg = Some(if r.marks.is_empty() {
            tr(self.lang(), "security.redact.bar.empty").to_string()
        } else {
            tr_args(
                self.lang(),
                "security.redact.bar.count",
                &[
                    ("count", &r.marks.len().to_string()),
                    ("pages", &r.page_count().to_string()),
                ],
            )
        });
    }

    /// Zones de la page : noir translucide à liseré rouge, opaque au survol (aperçu du résultat).
    pub fn paint_redact_marks(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let to_screen = |r: &PointRect| {
            egui::Rect::from_min_max(
                page_rect.min + egui::vec2(r.left * k, (page_h_pt - r.top) * k),
                page_rect.min + egui::vec2(r.right * k, (page_h_pt - r.bottom) * k),
            )
        };
        let hover = painter.ctx().pointer_hover_pos();
        let paint = |r: &PointRect| {
            let rect = to_screen(r);
            let alpha = if hover.is_some_and(|p| rect.contains(p)) {
                255
            } else {
                110
            };
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(alpha));
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.5, theme::ERROR),
                egui::StrokeKind::Outside,
            );
        };
        for m in self.annotate.redact.marks.iter().filter(|m| m.page == page) {
            paint(&m.rect);
        }
        if self.annotate.tool == Tool::Redact {
            if let Some(d) = self.annotate.drag.filter(|d| d.page == page) {
                paint(&d.rect());
            }
        }
    }

    /// Fin d'un tracé : nouvelle zone (aucune requête au moteur).
    pub fn redact_drag_finished(&mut self, drag: Drag) {
        let Some(size) = self
            .document
            .as_ref()
            .and_then(|d| d.info.page_sizes.get(drag.page).copied())
        else {
            return;
        };
        if let Some(rect) = drawn_rect(drag.rect(), size.width_pt, size.height_pt) {
            self.annotate.redact.add([RedactMark {
                page: drag.page,
                rect,
            }]);
            self.redact_status();
        }
    }

    pub fn redact_click(&mut self, page: usize, pt: (f32, f32)) {
        if self.annotate.redact.remove_at(page, pt) {
            self.redact_status();
            if !self.annotate.redact.marks.is_empty() {
                self.status_msg = Some(self.t("security.redact.status.removed").to_string());
            }
        }
    }

    pub fn redact_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::RedactSearch => {
                if self.annotate.tool != Tool::Redact {
                    self.execute(ctx, Command::ToolRedact);
                }
                let r = &mut self.annotate.redact;
                if r.search_query.is_empty() {
                    r.search_query = self.find.query.clone();
                }
                r.search_open = true;
            }
            Command::RedactClear => {
                self.annotate.redact.marks.clear();
                self.status_msg = Some(self.t("security.redact.status.all_removed").to_string());
            }
            Command::RedactApply => {
                if self.annotate.redact.marks.is_empty() {
                    self.error = Some("error.redact_no_marks");
                } else if !self.annotate.redact.applying {
                    self.annotate.redact.confirm_open = true;
                }
            }
            _ => {}
        }
    }

    /// Le document a changé (rotation, page supprimée, annulation…) : les zones ne désignent plus
    /// forcément le même contenu. Règle simple et sûre : elles sont effacées.
    pub fn redact_on_document_changed(&mut self) {
        if !self.annotate.redact.marks.is_empty() {
            self.annotate.redact.marks.clear();
            self.status_msg = Some(self.t("security.redact.status.cleared").to_string());
        }
    }

    pub fn on_redact_matches(&mut self, query: String, verify: bool, marks: Vec<RedactMark>) {
        let lang = self.lang();
        let pages = marks.iter().map(|m| m.page).collect::<BTreeSet<_>>();
        if verify {
            if !marks.is_empty() {
                self.status_msg = Some(tr_args(
                    lang,
                    "security.redact.status.remaining",
                    &[("count", &marks.len().to_string()), ("query", &query)],
                ));
                self.annotate.redact.add(marks);
            }
            return;
        }
        let first = marks.first().map(|m| m.page);
        let added = self.annotate.redact.add(marks);
        self.status_msg = Some(if added == 0 {
            tr(lang, "security.redact.status.none").to_string()
        } else {
            tr_args(
                lang,
                "security.redact.status.marked",
                &[
                    ("count", &added.to_string()),
                    ("pages", &pages.len().to_string()),
                ],
            )
        });
        if let Some(page) = first {
            self.go_to_page(page);
        }
    }

    /// Bilan d'un caviardage réussi ; `DocumentChanged` suit et affiche le message.
    pub fn on_redacted(&mut self, report: RedactReport) {
        let r = &mut self.annotate.redact;
        r.applying = false;
        r.marks.clear();
        self.tools.running = None;
        self.edit.pending_status = Some(tr_args(
            self.lang(),
            "security.redact.status.done",
            &[
                ("marks", &report.marks.to_string()),
                ("objects", &report.objects_removed.to_string()),
                (
                    "annotations",
                    &(report.annotations_removed + report.fields_removed).to_string(),
                ),
            ],
        ));
        if let Some((query, match_case)) = self.annotate.redact.last_query.take() {
            self.worker.send(Request::RedactSearch {
                query,
                match_case,
                verify: true,
            });
        }
    }

    /// Échec du moteur pendant l'application : les zones restent pour réessayer (page en image).
    pub fn redact_failed(&mut self) {
        if self.annotate.redact.applying {
            self.annotate.redact.applying = false;
            self.tools.running = None;
        }
    }

    /// Garde d'enregistrement : `true` si l'enregistrement doit attendre la réponse de l'utilisateur.
    pub fn redact_blocks_save(&mut self, ask_path: bool) -> bool {
        let r = &mut self.annotate.redact;
        if r.marks.is_empty() || std::mem::take(&mut r.save_confirmed) {
            return false;
        }
        r.save_prompt = Some(ask_path);
        true
    }

    pub fn redact_dialogs(&mut self, ctx: &egui::Context) {
        self.redact_search_dialog(ctx);
        self.redact_confirm_dialog(ctx);
        self.redact_unapplied_dialog(ctx);
    }

    fn redact_search_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.redact.search_open {
            return;
        }
        let lang = self.lang();
        let spec = dialog::Spec::new(
            "redact-search",
            tr(lang, "security.redact.search.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "security.redact.search.subtitle"))
        .icon(glyph::FIND, "🔍")
        .width(460.0);
        let (mut submit, mut cancel) = (false, false);
        let r = &mut self.annotate.redact;
        dialog::show(
            ctx,
            spec,
            |ui| {
                dialog::field_label(ui, tr(lang, "security.redact.search.label"));
                let field = ui.add(
                    egui::TextEdit::singleline(&mut r.search_query).desired_width(f32::INFINITY),
                );
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                ui.checkbox(&mut r.search_match_case, tr(lang, "find.match_case"));
                ui.add_space(4.0);
                ui.add(egui::Label::new(tr(lang, "security.redact.search.note")).wrap());
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "security.redact.search.ok"))
                    || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    submit = true;
                }
                if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        let r = &mut self.annotate.redact;
        if submit && !r.search_query.trim().is_empty() {
            r.search_open = false;
            let query = r.search_query.trim().to_string();
            r.last_query = Some((query.clone(), r.search_match_case));
            self.worker.send(Request::RedactSearch {
                query,
                match_case: r.search_match_case,
                verify: false,
            });
        } else if cancel {
            r.search_open = false;
        }
    }

    fn redact_confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.annotate.redact.confirm_open {
            return;
        }
        let lang = self.lang();
        let r = &self.annotate.redact;
        let spec = dialog::Spec::new(
            "redact-confirm",
            tr(lang, "security.redact.confirm.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr_args(
            lang,
            "security.redact.confirm.subtitle",
            &[
                ("count", &r.marks.len().to_string()),
                ("pages", &r.page_count().to_string()),
            ],
        ))
        .icon(glyph::REDACT, "█")
        .warning()
        .width(520.0);
        let (mut submit, mut cancel) = (false, false);
        let r = &mut self.annotate.redact;
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.add(egui::Label::new(tr(lang, "security.redact.confirm.text")).wrap());
                ui.add_space(6.0);
                ui.checkbox(
                    &mut r.rasterize,
                    tr(lang, "security.redact.confirm.rasterize"),
                );
                ui.checkbox(
                    &mut r.clean_hidden,
                    tr(lang, "security.redact.confirm.clean"),
                );
                ui.add_space(6.0);
                ui.add(egui::Label::new(tr(lang, "security.redact.confirm.note")).wrap());
            },
            |ui| {
                // Ctrl+Entrée lu sur l'événement même (jamais Entrée seule : action destructrice).
                if dialog::danger(ui, tr(lang, "security.redact.confirm.ok"))
                    || ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                {
                    submit = true;
                }
                if dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape))
                {
                    cancel = true;
                }
            },
        );
        let r = &mut self.annotate.redact;
        if submit {
            r.confirm_open = false;
            r.applying = true;
            self.tools.running = Some("security.redact.running");
            let options = RedactOptions {
                rasterize_pages: r.rasterize,
                clean_hidden_data: r.clean_hidden,
                ..Default::default()
            };
            self.worker.send(Request::Redact {
                marks: r.marks.clone(),
                options,
            });
        } else if cancel {
            r.confirm_open = false;
        }
    }

    fn redact_unapplied_dialog(&mut self, ctx: &egui::Context) {
        let Some(ask_path) = self.annotate.redact.save_prompt else {
            return;
        };
        let lang = self.lang();
        let spec = dialog::Spec::new(
            "redact-unapplied",
            tr(lang, "security.redact.unapplied.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .icon(glyph::WARNING, "⚠")
        .warning()
        .width(480.0);
        let count = self.annotate.redact.marks.len().to_string();
        let (mut apply, mut save, mut cancel) = (false, false, false);
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.add(
                    egui::Label::new(tr_args(
                        lang,
                        "security.redact.unapplied.text",
                        &[("count", &count)],
                    ))
                    .wrap(),
                );
            },
            |ui| {
                apply = dialog::primary(ui, tr(lang, "security.redact.unapplied.apply"));
                save = dialog::secondary(ui, tr(lang, "security.redact.unapplied.save"));
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if apply || save || cancel {
            self.annotate.redact.save_prompt = None;
        }
        if apply {
            self.cancel_after_save();
            self.annotate.redact.confirm_open = true;
        } else if save {
            self.annotate.redact.save_confirmed = true;
            self.save_document(ask_path);
        } else if cancel {
            self.cancel_after_save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(l: f32, b: f32, r: f32, t: f32) -> PointRect {
        PointRect {
            left: l,
            bottom: b,
            right: r,
            top: t,
        }
    }

    #[test]
    fn marks_are_added_once_and_removed_by_click() {
        let mut s = RedactState::default();
        assert!(s.clean_hidden, "informations cachées supprimées par défaut");
        let a = RedactMark {
            page: 0,
            rect: rect(10.0, 10.0, 50.0, 50.0),
        };
        let b = RedactMark {
            page: 2,
            rect: rect(30.0, 30.0, 80.0, 80.0),
        };
        let c = RedactMark {
            page: 0,
            rect: rect(20.0, 20.0, 60.0, 60.0),
        };
        assert_eq!(s.add([a, b, a]), 2);
        assert_eq!(s.add([c]), 1);
        assert_eq!(s.page_count(), 2);
        // Le clic retire la zone la plus récente qui contient le point, sur sa page seulement.
        assert!(s.remove_at(0, (40.0, 40.0)));
        assert_eq!(s.marks, vec![a, b]);
        assert!(!s.remove_at(1, (40.0, 40.0)));
        assert!(!s.remove_at(0, (5.0, 5.0)));
        assert!(!s.dialog_open());
    }

    #[test]
    fn drawn_rect_is_clamped_and_tiny_ones_ignored() {
        assert_eq!(
            drawn_rect(rect(-5.0, 10.0, 700.0, 20.0), 612.0, 792.0),
            Some(rect(0.0, 10.0, 612.0, 20.0))
        );
        assert!(drawn_rect(rect(10.0, 10.0, 11.0, 40.0), 612.0, 792.0).is_none());
        assert!(drawn_rect(rect(10.0, 10.0, 40.0, 11.5), 612.0, 792.0).is_none());
        assert!(drawn_rect(rect(700.0, 10.0, 720.0, 40.0), 612.0, 792.0).is_none());
    }
}
