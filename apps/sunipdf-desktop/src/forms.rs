//! Formulaires AcroForm, moitié « remplir » de l'outil Remplir et signer : champs surlignés sur la
//! page et saisie en place (outil `Tool::FillForm`), panneau « Champs du formulaire » (souris,
//! clavier, lecteur d'écran), bandeau d'information, boîte « Aplatir », export et import JSON.
//!
//! Aucune logique PDF ici : le modèle (`FormInfo`) vient du thread moteur (`Request::FormInfo`),
//! chaque validation est une `EditOp::FillForm` annulable, et le document reste la seule vérité
//! (la valeur locale n'est mise à jour qu'en attendant la relecture).

use crate::annotate::Tool;
use crate::app::{file_name, SuniApp, SCREEN_PER_PDF_POINT};
use crate::dialog;
use crate::icons::{self, glyph};
use crate::theme;
use crate::worker::{BatchOp, EditOp, Request};
use egui::{Align, Button, Color32, Frame, Key, Layout, Margin, RichText, Stroke, StrokeKind};
use std::collections::HashMap;
use sunipdf_core::{
    form_data, Command, FieldKind, FieldValue, FormField, FormInfo, FormKind, PointRect,
};
use sunipdf_i18n::{tr, tr_args, Language};

/// Éditeur posé sur un widget texte de la page.
pub struct InPlace {
    pub page: usize,
    pub field: String,
    pub widget: usize,
    pub text: String,
    pub multiline: bool,
    pub max_len: Option<u32>,
    pub focus: bool,
    /// Le clic qui a ouvert l'éditeur ne doit pas le valider.
    pub armed: bool,
}

/// Liste de choix flottante sous un widget liste déroulante ou zone de liste.
pub struct ChoicePopup {
    pub page: usize,
    pub field: String,
    pub widget: usize,
    pub armed: bool,
}

#[derive(Default)]
pub struct FormsState {
    /// Préservé à la fermeture et au changement d'onglet, comme `comments_open`.
    pub panel_open: bool,
    /// Panneau : tout le document (sinon la page courante).
    pub scope_all: bool,
    pub info: Option<FormInfo>,
    /// Relecture nécessaire (document modifié) ; `requested` : relecture en vol.
    pub stale: bool,
    pub requested: bool,
    /// Saisies du panneau non validées, par nom de champ.
    pub drafts: HashMap<String, String>,
    pub editing: Option<InPlace>,
    pub popup: Option<ChoicePopup>,
    pub bar_dismissed: bool,
    pub flatten_open: bool,
    pub flatten_all_pages: bool,
    /// Valeurs JSON à appliquer dès que le modèle est lu.
    pub pending_import: Option<serde_json::Value>,
    /// « Effacer les champs » demandé : exécuté dès que le modèle est lu (rien n'est envoyé au
    /// moteur s'il n'y a rien à effacer : pas de niveau d'annulation vide).
    pub pending_clear: bool,
    /// Champ mis en évidence sur la page (clic sur son nom dans le panneau).
    pub focus: Option<String>,
}

impl FormsState {
    pub fn has_unsaved_input(&self) -> bool {
        !self.drafts.is_empty()
            || self.editing.as_ref().is_some_and(|e| {
                self.info
                    .as_ref()
                    .and_then(|i| i.field(&e.field))
                    .is_none_or(|f| f.value != FieldValue::Text(e.text.clone()))
            })
    }

    /// Nouveau document : tout repart, sauf l'ouverture du panneau.
    pub fn reset_for_document(&mut self) {
        let panel_open = self.panel_open;
        *self = Self::default();
        self.panel_open = panel_open;
    }
}

/// (champ, widget) en ordre de lecture : page, puis de haut en bas (bandes de 6 pt), puis de
/// gauche à droite.
pub fn reading_order(info: &FormInfo) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = info
        .fields
        .iter()
        .enumerate()
        .flat_map(|(fi, f)| (0..f.widgets.len()).map(move |wi| (fi, wi)))
        .collect();
    let key = |(fi, wi): (usize, usize)| {
        let w = &info.fields[fi].widgets[wi];
        (
            w.page,
            -(w.display.top / 6.0).round() as i64,
            w.display.left,
        )
    };
    v.sort_by(|&a, &b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0)
            .then(ka.1.cmp(&kb.1))
            .then(ka.2.total_cmp(&kb.2))
    });
    v
}

/// Clé i18n du libellé d'un genre de champ.
pub fn kind_key(kind: FieldKind) -> String {
    format!("forms.kind.{}", kind.id())
}

/// Valeur lisible (panneau, accessibilité) : libellés d'affichage pour les listes.
pub fn value_label(lang: Language, f: &FormField) -> String {
    match &f.value {
        FieldValue::None => String::new(),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Bool(b) => tr(
            lang,
            if *b {
                "a11y.forms.checked"
            } else {
                "a11y.forms.unchecked"
            },
        )
        .to_string(),
        FieldValue::Choice(v) if v.is_empty() => tr(lang, "forms.panel.choice_none").to_string(),
        FieldValue::Choice(v) => v
            .iter()
            .map(|x| f.label_of(x))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn to_screen(r: PointRect, page_rect: egui::Rect, k: f32, page_h_pt: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        page_rect.min + egui::vec2(r.left * k, (page_h_pt - r.top) * k),
        page_rect.min + egui::vec2(r.right * k, (page_h_pt - r.bottom) * k),
    )
}

fn chosen(f: &FormField) -> Vec<String> {
    match &f.value {
        FieldValue::Choice(v) => v.clone(),
        _ => Vec::new(),
    }
}

/// Valeurs d'export dans l'ordre des options (JSON et `/I` stables).
fn ordered_choice(f: &FormField, wanted: &[String]) -> FieldValue {
    FieldValue::Choice(
        f.options
            .iter()
            .filter(|o| wanted.contains(&o.value))
            .map(|o| o.value.clone())
            .collect(),
    )
}

impl SuniApp {
    /// Valide toutes les saisies avant d'en envoyer une ; une erreur garde les textes intacts.
    pub fn commit_form_input(&mut self) -> bool {
        let mut drafts = self.forms.drafts.clone();
        if let Some(e) = &self.forms.editing {
            drafts.insert(e.field.clone(), e.text.clone());
        }
        let mut values = Vec::new();
        for (name, text) in &drafts {
            let value = FieldValue::Text(text.clone());
            let field = self.forms.info.as_ref().and_then(|i| i.field(name));
            if field.is_none_or(|f| f.validate(&value).is_err()) {
                self.error = Some("error.form_value_invalid");
                return false;
            }
            if field.is_some_and(|f| f.value != value) {
                values.push((name.clone(), value));
            }
        }
        self.forms.editing = None;
        self.forms.drafts = drafts;
        if !values.is_empty() {
            self.commit_many(values);
        }
        true
    }

    /// Outil « Remplir le formulaire » : champs surlignés, panneau ouvert ; sans champ AcroForm,
    /// un message d'état seulement (pas d'outil qui ne ferait rien).
    pub fn enter_fill_tool(&mut self) {
        let lang = self.lang();
        let Some(doc) = &self.document else { return };
        if doc.info.form == FormKind::None {
            self.status_msg = Some(tr(lang, "forms.panel.empty_doc").to_string());
            return;
        }
        self.cancel_placing();
        self.annotate.tool = Tool::FillForm;
        self.annotate.drag = None;
        self.edit.selection = None;
        crate::workspace::follow(
            &mut self.annotate,
            &mut self.organize,
            &mut self.edit,
            Command::ToolFillForm,
        );
        self.forms.panel_open = true;
        self.forms.stale = true;
    }

    /// Demande le modèle au moteur s'il est utile (panneau, outil, import) et périmé.
    pub fn refresh_forms_if_needed(&mut self) {
        let Some(doc) = &self.document else { return };
        let f = &mut self.forms;
        if doc.info.form == FormKind::None {
            // Sans formulaire (ou aplati) : rien à lire, et le dernier modèle est faux.
            if f.info.as_ref().is_some_and(|i| !i.fields.is_empty()) {
                f.info = Some(FormInfo::default());
            }
            f.stale = false;
            f.pending_import = None;
            f.pending_clear = false;
            f.editing = None;
            f.popup = None;
            // Outil Remplir sans plus aucun champ (aplatissement complet) : il ne ferait rien.
            if self.annotate.tool == Tool::FillForm {
                self.annotate.tool = Tool::Select;
            }
            return;
        }
        let wanted = f.panel_open
            || self.annotate.tool == Tool::FillForm
            || !f.drafts.is_empty()
            || f.editing.is_some()
            || f.pending_import.is_some()
            || f.pending_clear;
        if wanted && (f.stale || f.info.is_none()) && !f.requested {
            f.requested = true;
            f.stale = false;
            self.worker.send(Request::FormInfo);
        } else if f.pending_clear && !f.stale && f.info.is_some() {
            self.clear_form_now();
        }
    }

    pub fn on_form_info(&mut self, info: FormInfo) {
        self.forms.requested = false;
        // Un brouillon égal au modèle n'a plus de raison d'être ; un brouillon refusé reste.
        self.forms.drafts.retain(|name, draft| {
            info.field(name)
                .is_some_and(|f| !matches!(&f.value, FieldValue::Text(t) if t == draft))
        });
        self.forms.info = Some(info);
        if let Some(json) = self.forms.pending_import.take() {
            self.apply_import(&json);
        }
        if self.forms.pending_clear {
            self.clear_form_now();
        }
    }

    /// « Effacer les champs » sur le modèle lu : rien à effacer = message d'état seulement.
    fn clear_form_now(&mut self) {
        let lang = self.lang();
        self.forms.pending_clear = false;
        let Some(info) = &self.forms.info else { return };
        if info.fields.iter().any(|f| f.cleared().is_some()) {
            self.edit.pending_status = Some(tr(lang, "status.form_cleared").to_string());
            self.worker.send(Request::Edit(EditOp::ClearForm));
        } else {
            self.status_msg = Some(tr(lang, "status.form_nothing_to_clear").to_string());
        }
    }

    /// Envoie des valeurs validées en une seule opération annulable ; la valeur locale est mise à
    /// jour aussitôt (surcouche et panneau) en attendant la relecture.
    fn commit_many(&mut self, values: Vec<(String, FieldValue)>) {
        let lang = self.lang();
        let Some(info) = &mut self.forms.info else {
            return;
        };
        let mut accepted = Vec::with_capacity(values.len());
        for (name, value) in values {
            let Some(f) = info.fields.iter_mut().find(|f| f.name == name) else {
                continue;
            };
            match f.validate(&value) {
                Ok(()) => {
                    f.value = value.clone();
                    accepted.push((name, value));
                }
                Err(reason) => {
                    log::warn!("valeur refusée pour {name} : {reason}");
                    self.error = Some("error.form_value_invalid");
                }
            }
        }
        let Some((name, _)) = accepted.last() else {
            return;
        };
        self.edit.pending_status = Some(tr_args(lang, "status.form_updated", &[("name", name)]));
        self.worker
            .send(Request::Edit(EditOp::FillForm { values: accepted }));
    }

    pub fn commit_form(&mut self, name: String, value: FieldValue) {
        self.commit_many(vec![(name, value)]);
    }

    // ----- Page : surcouche et saisie en place --------------------------------------------------

    /// Rectangles des widgets de la page (outil Remplir actif) ; renvoie le widget cliqué.
    pub fn forms_overlay(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) -> Option<(usize, usize)> {
        if self.annotate.tool != Tool::FillForm {
            return None;
        }
        let info = self.forms.info.as_ref()?;
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let clip = ui.clip_rect();
        let mut hit = None;
        for (fi, f) in info.fields.iter().enumerate() {
            let fillable = f.fillable() && info.kind != FormKind::Xfa;
            let color = if !fillable {
                Color32::from_gray(140)
            } else if f.required {
                theme::ERROR
            } else {
                theme::BLUE
            };
            let kind = tr(lang, &kind_key(f.kind));
            let value = value_label(lang, f);
            let label = if value.is_empty() {
                tr_args(
                    lang,
                    "a11y.forms.widget",
                    &[("name", &f.name), ("kind", kind)],
                )
            } else {
                tr_args(
                    lang,
                    "a11y.forms.widget_value",
                    &[("name", &f.name), ("kind", kind), ("value", &value)],
                )
            };
            let widget_type = match f.kind {
                FieldKind::Checkbox => egui::WidgetType::Checkbox,
                FieldKind::Radio => egui::WidgetType::RadioButton,
                FieldKind::Combo | FieldKind::List => egui::WidgetType::ComboBox,
                FieldKind::Text => egui::WidgetType::TextEdit,
                _ => egui::WidgetType::Button,
            };
            for (wi, w) in f.widgets.iter().enumerate() {
                if w.page != page {
                    continue;
                }
                let r = to_screen(w.display, page_rect, k, page_h_pt);
                if !clip.intersects(r) {
                    continue;
                }
                let resp = ui.interact(
                    r,
                    egui::Id::new(("form-widget", page, fi, wi)),
                    egui::Sense::click(),
                );
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(widget_type, fillable, label.clone())
                });
                let hovered = fillable && resp.hovered();
                painter.rect_filled(
                    r,
                    2.0,
                    color.gamma_multiply(if hovered { 0.22 } else { 0.12 }),
                );
                painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(if hovered { 1.5 } else { 1.0 }, color),
                    StrokeKind::Inside,
                );
                if self.forms.focus.as_deref() == Some(f.name.as_str()) || resp.has_focus() {
                    theme::focus_ring(painter, r.expand(3.0), &p, 4);
                }
                if fillable {
                    resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    log::debug!("clic sur le champ {} (widget {wi}, {r:?})", f.name);
                    hit = Some((fi, wi));
                    // Sinon les raccourcis à une lettre restent bloqués (widget focalisé).
                    resp.surrender_focus();
                }
            }
        }
        hit
    }

    /// Clic sur un widget : bascule (case, radio), éditeur (texte) ou liste (choix).
    pub fn forms_activate(&mut self, page: usize, fi: usize, wi: usize) {
        let lang = self.lang();
        let Some(info) = &self.forms.info else { return };
        let Some(f) = info.fields.get(fi) else { return };
        let Some(w) = f.widgets.get(wi) else { return };
        self.forms.focus = None;
        if !f.fillable() || info.kind == FormKind::Xfa {
            self.status_msg = Some(tr(lang, "status.form_unsupported_widget").to_string());
            return;
        }
        let name = f.name.clone();
        match f.kind {
            FieldKind::Checkbox => {
                let on = !matches!(f.value, FieldValue::Bool(true));
                self.commit_form(name, FieldValue::Bool(on));
            }
            FieldKind::Radio => {
                let Some(on) = w.on_state.clone() else { return };
                let already = chosen(f).first() == Some(&on);
                let value = if !already {
                    FieldValue::Choice(vec![on])
                } else if f.no_toggle_off {
                    return;
                } else {
                    FieldValue::Choice(Vec::new())
                };
                self.commit_form(name, value);
            }
            FieldKind::Text => {
                let text = match &f.value {
                    FieldValue::Text(t) => t.clone(),
                    _ => String::new(),
                };
                self.forms.popup = None;
                self.forms.editing = Some(InPlace {
                    page,
                    field: name,
                    widget: wi,
                    text,
                    multiline: f.multiline,
                    max_len: f.max_len,
                    focus: true,
                    armed: false,
                });
            }
            FieldKind::Combo | FieldKind::List => {
                self.forms.editing = None;
                self.forms.popup = Some(ChoicePopup {
                    page,
                    field: name,
                    widget: wi,
                    armed: false,
                });
            }
            FieldKind::PushButton | FieldKind::Signature => {}
        }
    }

    /// Valide la saisie en cours de l'éditeur posé sur la page (texte changé → `commit_form`) et
    /// le ferme ; renvoie l'éditeur fermé. Appelé par Entrée, Tab, un clic ailleurs, la sortie
    /// du widget de la zone visible et l'enregistrement (Ctrl+S).
    pub fn commit_editor(&mut self) -> Option<InPlace> {
        let e = self.forms.editing.as_ref()?;
        let value = FieldValue::Text(e.text.clone());
        if self
            .forms
            .info
            .as_ref()
            .and_then(|i| i.field(&e.field))
            .is_none_or(|f| f.validate(&value).is_err())
        {
            self.error = Some("error.form_value_invalid");
            return None;
        }
        let e = self.forms.editing.take()?;
        let current = self
            .forms
            .info
            .as_ref()
            .and_then(|i| i.field(&e.field))
            .map(|f| f.value.clone());
        if current != Some(FieldValue::Text(e.text.clone())) {
            self.forms.drafts.insert(e.field.clone(), e.text.clone());
            self.commit_form(e.field.clone(), FieldValue::Text(e.text.clone()));
        }
        Some(e)
    }

    /// Éditeur de texte posé sur le widget en cours de saisie (page `page`), découpé à la zone
    /// visible du document `view` (jamais par-dessus les onglets ou les panneaux) ; validé et fermé
    /// dès que le widget sort entièrement de cette zone.
    pub fn forms_editor(
        &mut self,
        ctx: &egui::Context,
        view: egui::Rect,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        if self.forms.info.is_none() {
            return;
        }
        let lang = self.lang();
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let Some(e) = self.forms.editing.as_ref().filter(|e| e.page == page) else {
            return;
        };
        let (field, widget) = (e.field.clone(), e.widget);
        let Some(display) = self
            .forms
            .info
            .as_ref()
            .and_then(|i| i.field(&field))
            .and_then(|f| f.widgets.get(widget))
            .map(|w| w.display)
        else {
            self.forms.editing = None;
            return;
        };
        let r = to_screen(display, page_rect, k, page_h_pt);
        if !view.intersects(r) {
            self.commit_editor();
            return;
        }
        let e = self.forms.editing.as_mut().expect("éditeur");
        let multiline = e.multiline;
        let font_px = if multiline {
            (11.0 * k).clamp(9.0, 30.0)
        } else {
            (r.height() * 0.62).clamp(9.0, 40.0)
        };
        let font = egui::FontId::proportional(font_px);
        let a11y = tr_args(lang, "a11y.forms.editor", &[("name", &field)]);
        let area = egui::Area::new(egui::Id::new(("form-editor", page)))
            .order(egui::Order::Foreground)
            .fixed_pos(r.min)
            // Collé au widget (jamais déplacé) et découpé à la vue : la partie hors de la zone du
            // document n'est ni dessinée ni cliquable.
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(view);
                Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.5, theme::BLUE))
                    .corner_radius(3)
                    .show(ui, |ui| {
                        ui.set_min_size(r.size());
                        let width = (r.width() - 8.0).max(40.0);
                        let mut edit = if multiline {
                            let rows = ((r.height() / (font_px * 1.3)).floor() as usize).max(1);
                            egui::TextEdit::multiline(&mut e.text).desired_rows(rows)
                        } else {
                            egui::TextEdit::singleline(&mut e.text)
                        }
                        .font(font)
                        .text_color(Color32::BLACK)
                        .frame(Frame::new())
                        .margin(Margin::symmetric(4, 2))
                        .desired_width(width)
                        // Un état de curseur par champ : Tab ne reprend pas la position du champ
                        // précédent.
                        .id_salt(("form-field-edit", &field));
                        if let Some(n) = e.max_len {
                            edit = edit.char_limit(n as usize);
                        }
                        let out = edit.show(ui);
                        let resp = &out.response.response;
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &a11y)
                        });
                        if e.focus {
                            // Comme un formulaire : le contenu est sélectionné à l'entrée.
                            resp.request_focus();
                            let mut state = out.state;
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(e.text.chars().count()),
                                )));
                            state.store(ui.ctx(), resp.id);
                            e.focus = false;
                        }
                    });
            });
        let edit_rect = area.response.rect.intersect(view);
        let (enter, escape, tab, shift, pressed_outside) = ctx.input(|i| {
            let press = i.pointer.interact_pos().filter(|_| i.pointer.any_pressed());
            (
                i.key_pressed(Key::Enter) && (!multiline || i.modifiers.command),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Tab),
                i.modifiers.shift,
                press.is_some_and(|p| !edit_rect.contains(p)),
            )
        });
        let commit_click = e.armed && pressed_outside;
        e.armed = true;
        if escape {
            self.forms.editing = None;
        } else if enter || tab || commit_click {
            if let Some(e) = self.commit_editor().filter(|_| tab) {
                self.open_next_field(&e.field, e.widget, shift);
            }
        }
    }

    /// Tab / Maj+Tab : champ suivant ou précédent en ordre de lecture (texte ou liste), amené à
    /// l'écran s'il est sur une autre page.
    fn open_next_field(&mut self, field: &str, widget: usize, backwards: bool) {
        let Some(info) = &self.forms.info else { return };
        let order = reading_order(info);
        let n = order.len();
        if n == 0 {
            return;
        }
        let fi = info.fields.iter().position(|f| f.name == field);
        let pos = order
            .iter()
            .position(|&(f, w)| Some(f) == fi && w == widget)
            .unwrap_or(if backwards { 0 } else { n - 1 });
        for step in 1..=n {
            let i = if backwards {
                (pos + n - step) % n
            } else {
                (pos + step) % n
            };
            let (f, w) = order[i];
            let target = &info.fields[f];
            if !target.fillable()
                || !matches!(
                    target.kind,
                    FieldKind::Text | FieldKind::Combo | FieldKind::List
                )
            {
                continue;
            }
            let widget = &target.widgets[w];
            let (page, display) = (widget.page, widget.display);
            let h = self
                .document
                .as_ref()
                .and_then(|d| d.info.page_sizes.get(page))
                .map_or(0.0, |s| s.height_pt);
            self.go_to_page_point(page, display.left, (h - display.top).max(0.0));
            self.forms_activate(page, f, w);
            return;
        }
    }

    /// Liste de choix flottante sous un widget liste déroulante ou zone de liste, maintenue dans
    /// la zone visible du document `view` ; fermée dès que le widget en sort entièrement.
    pub fn forms_popup(
        &mut self,
        ctx: &egui::Context,
        view: egui::Rect,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) {
        let lang = self.lang();
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let Some(p) = self.forms.popup.as_ref().filter(|p| p.page == page) else {
            return;
        };
        let (field, widget, armed) = (p.field.clone(), p.widget, p.armed);
        let Some(f) = self
            .forms
            .info
            .as_ref()
            .and_then(|i| i.field(&field))
            .cloned()
        else {
            self.forms.popup = None;
            return;
        };
        let Some(w) = f.widgets.get(widget) else {
            self.forms.popup = None;
            return;
        };
        let r = to_screen(w.display, page_rect, k, page_h_pt);
        if !view.intersects(r) {
            self.forms.popup = None;
            return;
        }
        let multi = f.kind == FieldKind::List && f.multi_select;
        let current = chosen(&f);
        let mut new_value: Option<Vec<String>> = None;
        let area = egui::Area::new(egui::Id::new(("form-choice", page)))
            .order(egui::Order::Foreground)
            .fixed_pos(r.left_bottom() + egui::vec2(0.0, 2.0))
            .constrain_to(view)
            .show(ctx, |ui| {
                theme::square_boxes(ui);
                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(r.width().max(180.0));
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            if !multi && !f.required {
                                let none = tr(lang, "forms.panel.choice_none");
                                if ui.selectable_label(current.is_empty(), none).clicked() {
                                    new_value = Some(Vec::new());
                                }
                            }
                            for o in &f.options {
                                let on = current.contains(&o.value);
                                let a11y = tr_args(
                                    lang,
                                    "a11y.forms.option",
                                    &[("name", &f.name), ("option", &o.label)],
                                );
                                if multi {
                                    let mut b = on;
                                    let resp = ui.checkbox(&mut b, &o.label);
                                    resp.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::WidgetType::Checkbox,
                                            true,
                                            b,
                                            &a11y,
                                        )
                                    });
                                    if resp.changed() {
                                        let mut v = current.clone();
                                        if b {
                                            v.push(o.value.clone());
                                        } else {
                                            v.retain(|x| x != &o.value);
                                        }
                                        new_value = Some(v);
                                    }
                                } else {
                                    let resp = ui.selectable_label(on, &o.label);
                                    resp.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::WidgetType::SelectableLabel,
                                            true,
                                            on,
                                            &a11y,
                                        )
                                    });
                                    if resp.clicked() {
                                        new_value = Some(vec![o.value.clone()]);
                                    }
                                }
                            }
                        });
                });
            });
        let popup_rect = area.response.rect;
        let (escape, pressed_outside) = ctx.input(|i| {
            let press = i.pointer.interact_pos().filter(|_| i.pointer.any_pressed());
            (
                i.key_pressed(Key::Escape),
                press.is_some_and(|p| !popup_rect.contains(p)),
            )
        });
        if let Some(p) = self.forms.popup.as_mut() {
            p.armed = true;
        }
        if let Some(v) = new_value {
            self.commit_form(f.name.clone(), ordered_choice(&f, &v));
            if !multi {
                self.forms.popup = None;
            }
        } else if escape || (armed && pressed_outside) {
            self.forms.popup = None;
        }
    }

    // ----- Bandeau, panneau, boîte « Aplatir », JSON ----------------------------------------------

    /// Bandeau « Ce document contient un formulaire » au-dessus de la page.
    pub fn forms_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(doc) = &self.document else { return };
        if doc.info.form != FormKind::AcroForm
            || self.forms.bar_dismissed
            || self.annotate.tool == Tool::FillForm
        {
            return;
        }
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        let (mut fill, mut dismiss) = (false, false);
        Frame::new()
            .fill(p.accent_soft)
            .inner_margin(Margin::symmetric(12, 7))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Rangée repliable : en zone étroite, les boutons passent sous le texte au lieu
                // de le recouvrir ou de sortir de l'écran.
                ui.horizontal_wrapped(|ui| {
                    ui.label(icons::icon(has_icons, glyph::FORM, "☑", 16.0).color(p.accent));
                    ui.label(RichText::new(tr(lang, "forms.bar.text")).size(13.5));
                    if dialog::primary(ui, tr(lang, "menu.forms.fill_tool")) {
                        fill = true;
                    }
                    if ui
                        .add(Button::new(tr(lang, "forms.bar.dismiss")).frame(false))
                        .clicked()
                    {
                        dismiss = true;
                    }
                });
            });
        if fill {
            self.execute(ctx, Command::ToolFillForm);
        }
        if dismiss {
            self.forms.bar_dismissed = true;
        }
    }

    /// Panneau « Champs du formulaire » : un contrôle accessible par champ.
    pub fn forms_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let has_icons = self.has_icons;
        let Some(doc) = &self.document else { return };
        let (current_page, form_kind) = (doc.page, doc.info.form);
        let page_sizes = doc.info.page_sizes.clone();
        let mut close = false;
        let mut go: Option<String> = None;
        let mut commits: Vec<(String, FieldValue)> = Vec::new();
        // Le modèle sort de `self` le temps du dessin (brouillons et contrôles empruntent le reste).
        let info = self.forms.info.take();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(icons::semibold(tr(lang, "forms.panel.title"), 15.0));
            if let Some(info) = &info {
                ui.label(
                    RichText::new(tr_args(
                        lang,
                        "forms.panel.count",
                        &[("count", &info.fields.len().to_string())],
                    ))
                    .color(p.text_muted)
                    .size(13.0),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r = ui
                    .add(Button::new(icons::icon(has_icons, glyph::CLOSE, "✕", 14.0)).frame(false))
                    .on_hover_text(tr(lang, "tools.close"));
                if r.clicked() {
                    close = true;
                }
            });
        });
        ui.separator();
        let banner = |ui: &mut egui::Ui, key: &str| {
            let (bg, fg) = if p.text == theme::palette(true).text {
                (
                    Color32::from_rgb(0x45, 0x30, 0x0E),
                    Color32::from_rgb(0xFC, 0xD3, 0x4D),
                )
            } else {
                (
                    Color32::from_rgb(0xFE, 0xF3, 0xC7),
                    Color32::from_rgb(0x92, 0x40, 0x0E),
                )
            };
            Frame::new()
                .fill(bg)
                .corner_radius(8)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        egui::Label::new(RichText::new(tr(lang, key)).size(12.5).color(fg)).wrap(),
                    );
                });
            ui.add_space(4.0);
        };
        match &info {
            None if form_kind == FormKind::None => {
                ui.label(RichText::new(tr(lang, "forms.panel.empty_doc")).color(p.text_muted));
            }
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(tr(lang, "forms.panel.loading")).color(p.text_muted));
                });
            }
            Some(info) => {
                if info.kind == FormKind::Xfa {
                    banner(ui, "forms.banner.xfa");
                }
                if info.has_scripts {
                    banner(ui, "forms.banner.scripts");
                }
                if info.fields.is_empty() {
                    ui.label(RichText::new(tr(lang, "forms.panel.empty_doc")).color(p.text_muted));
                } else {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.forms.scope_all,
                            false,
                            tr(lang, "forms.panel.scope_page"),
                        );
                        ui.selectable_value(
                            &mut self.forms.scope_all,
                            true,
                            tr(lang, "forms.panel.scope_all"),
                        );
                    });
                    let editable = info.kind == FormKind::AcroForm;
                    if editable {
                        ui.add(
                            egui::Label::new(
                                RichText::new(tr(lang, "forms.hint"))
                                    .size(12.0)
                                    .color(p.text_muted),
                            )
                            .wrap(),
                        );
                    }
                    ui.add_space(4.0);
                    let order = reading_order(info);
                    // Un champ par ligne (son premier widget), groupé par page.
                    let mut rows: Vec<(usize, usize)> = order
                        .iter()
                        .copied()
                        .filter(|&(_, w)| w == 0)
                        .filter(|&(f, _)| {
                            self.forms.scope_all || info.fields[f].widgets[0].page == current_page
                        })
                        .collect();
                    let orphans: Vec<usize> = (0..info.fields.len())
                        .filter(|&f| info.fields[f].widgets.is_empty() && self.forms.scope_all)
                        .collect();
                    if rows.is_empty() && orphans.is_empty() {
                        ui.label(RichText::new(tr(lang, "forms.panel.empty")).color(p.text_muted));
                    }
                    let scope_all = self.forms.scope_all;
                    let drafts = &mut self.forms.drafts;
                    let mut error = false;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut last_page = None;
                            rows.sort_by_key(|&(f, _)| info.fields[f].widgets[0].page);
                            // `reading_order` est déjà trié par page : le tri stable garde l'ordre.
                            for (fi, _) in rows {
                                let f = &info.fields[fi];
                                let page = f.widgets[0].page;
                                if scope_all && last_page != Some(page) {
                                    last_page = Some(page);
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(tr_args(
                                            lang,
                                            "forms.panel.page",
                                            &[("page", &(page + 1).to_string())],
                                        ))
                                        .size(12.0)
                                        .color(p.text_muted),
                                    );
                                }
                                field_card(
                                    ui,
                                    lang,
                                    &p,
                                    f,
                                    editable,
                                    drafts,
                                    &mut commits,
                                    &mut go,
                                    &mut error,
                                );
                            }
                            for fi in orphans {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(tr(lang, "forms.panel.no_widget"))
                                        .size(12.0)
                                        .color(p.text_muted),
                                );
                                field_card(
                                    ui,
                                    lang,
                                    &p,
                                    &info.fields[fi],
                                    editable,
                                    drafts,
                                    &mut commits,
                                    &mut go,
                                    &mut error,
                                );
                            }
                            ui.add_space(8.0);
                        });
                    if error {
                        self.error = Some("error.form_value_invalid");
                    }
                }
            }
        }
        self.forms.info = info;
        if close {
            self.execute(ctx, Command::ToggleForms);
        }
        if let Some(name) = go {
            let target = self
                .forms
                .info
                .as_ref()
                .and_then(|i| i.field(&name))
                .and_then(|f| f.widgets.first())
                .map(|w| (w.page, w.display));
            if let Some((page, display)) = target {
                let h = page_sizes.get(page).map_or(0.0, |s| s.height_pt);
                self.go_to_page_point(page, display.left, (h - display.top).max(0.0));
                self.forms.focus = Some(name);
                if self.annotate.tool != Tool::FillForm {
                    self.enter_fill_tool();
                }
            }
        }
        if !commits.is_empty() {
            self.commit_many(commits);
        }
    }

    pub fn flatten_dialog(&mut self, ctx: &egui::Context) {
        if !self.forms.flatten_open {
            return;
        }
        let lang = self.lang();
        let current_page = self.document.as_ref().map(|d| d.page);
        let (mut submit, mut cancel) = (false, false);
        let spec = dialog::Spec::new(
            "flatten",
            tr(lang, "dialog.flatten.title"),
            self.settings.dark_theme,
            self.has_icons,
        )
        .subtitle(tr(lang, "dialog.flatten.subtitle"))
        .icon(glyph::COMBINE, "⧉")
        .width(480.0);
        let all = &mut self.forms.flatten_all_pages;
        dialog::show(
            ctx,
            spec,
            |ui| {
                ui.add(egui::Label::new(tr(lang, "dialog.flatten.text")).wrap());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.radio_value(all, true, tr(lang, "stamp.scope.all"));
                    ui.radio_value(all, false, tr(lang, "stamp.scope.current"));
                });
            },
            |ui| {
                if dialog::primary(ui, tr(lang, "dialog.flatten.ok"))
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
        if submit {
            self.forms.flatten_open = false;
            self.forms.editing = None;
            self.forms.popup = None;
            let pages = if self.forms.flatten_all_pages {
                None
            } else {
                current_page.map(|p| vec![p])
            };
            self.edit.pending_status = Some(tr(lang, "status.form_flattened").to_string());
            self.worker
                .send(Request::Edit(EditOp::FlattenForm { pages }));
        } else if cancel {
            self.forms.flatten_open = false;
        }
    }

    fn json_dialog(&self, title_key: &str) -> rfd::FileDialog {
        rfd::FileDialog::new()
            .set_title(self.t(title_key))
            .add_filter(self.t("forms.filter_json"), &["json"])
    }

    /// Exporter les valeurs : `<document>.json` proposé à côté du PDF.
    pub fn tool_export_form(&mut self) {
        let Some(doc) = &self.document else { return };
        let suggested = doc.info.path.with_extension("json");
        let mut dialog = self
            .json_dialog("forms.save_export")
            .set_file_name(file_name(&suggested));
        if let Some(dir) = suggested.parent().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut out) = dialog.save_file() else {
            return;
        };
        if out.extension().is_none() {
            out.set_extension("json");
        }
        self.run_batch(BatchOp::ExportFormData { out }, "tools.running.form_export");
    }

    /// Importer un fichier `{ nom: valeur }` : appliqué dès que le modèle est lu.
    pub fn tool_import_form(&mut self) {
        let lang = self.lang();
        let Some(doc) = &self.document else { return };
        if doc.info.form != FormKind::AcroForm {
            self.status_msg = Some(tr(lang, "forms.panel.empty_doc").to_string());
            return;
        }
        let Some(path) = self.json_dialog("forms.pick_import").pick_file() else {
            return;
        };
        let value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .filter(serde_json::Value::is_object);
        match value {
            Some(v) => {
                self.forms.pending_import = Some(v);
                self.forms.stale = true;
            }
            None => self.error = Some("error.form_import_invalid"),
        }
    }

    fn apply_import(&mut self, json: &serde_json::Value) {
        let lang = self.lang();
        let Some(info) = &self.forms.info else { return };
        let plan = match form_data::plan_from_json(info, json) {
            Ok(plan) => plan,
            Err(_) => {
                self.error = Some("error.form_import_invalid");
                return;
            }
        };
        let skipped = plan.skipped_unknown.len() + plan.skipped_invalid.len();
        for (name, reason) in &plan.skipped_invalid {
            log::info!("import : {name} ignoré ({reason})");
        }
        let msg = tr_args(
            lang,
            "status.form_imported",
            &[
                ("applied", &plan.values.len().to_string()),
                ("skipped", &skipped.to_string()),
            ],
        );
        if plan.values.is_empty() {
            self.status_msg = Some(msg);
            return;
        }
        self.edit.pending_status = Some(msg);
        self.worker.send(Request::Edit(EditOp::FillForm {
            values: plan.values,
        }));
    }
}

/// Carte d'un champ dans le panneau : nom (cliquable : aller au champ), badges, contrôle.
#[allow(clippy::too_many_arguments)]
fn field_card(
    ui: &mut egui::Ui,
    lang: Language,
    p: &theme::Palette,
    f: &FormField,
    editable: bool,
    drafts: &mut HashMap<String, String>,
    commits: &mut Vec<(String, FieldValue)>,
    go: &mut Option<String>,
    error: &mut bool,
) {
    let enabled = editable && f.fillable();
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Le libellé du champ nomme chaque contrôle pour les lecteurs d'écran (`labelled_by`).
            let label_id = ui
                .horizontal_wrapped(|ui| {
                    let title = f.tooltip.clone().unwrap_or_else(|| f.name.clone());
                    let name = ui
                        .add(
                            egui::Label::new(icons::semibold(title, 13.0).color(p.text))
                                .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(&f.name);
                    if name.clicked() && !f.widgets.is_empty() {
                        *go = Some(f.name.clone());
                    }
                    ui.label(
                        RichText::new(tr(lang, &kind_key(f.kind)))
                            .size(11.5)
                            .color(p.text_muted),
                    );
                    if f.required {
                        ui.label(
                            RichText::new(tr(lang, "forms.panel.required"))
                                .size(11.5)
                                .color(p.error),
                        );
                    }
                    if f.read_only {
                        ui.label(
                            RichText::new(tr(lang, "forms.panel.read_only"))
                                .size(11.5)
                                .color(p.text_muted),
                        );
                    }
                    name.id
                })
                .inner;
            ui.add_space(2.0);
            theme::square_boxes(ui);
            ui.add_enabled_ui(enabled, |ui| match f.kind {
                FieldKind::Text => {
                    let current = match &f.value {
                        FieldValue::Text(t) => t.clone(),
                        _ => String::new(),
                    };
                    // Un brouillon n'existe que pendant la frappe : sinon le champ suit le modèle.
                    let mut text = drafts
                        .get(&f.name)
                        .cloned()
                        .unwrap_or_else(|| current.clone());
                    let mut edit = if f.multiline {
                        egui::TextEdit::multiline(&mut text).desired_rows(3)
                    } else {
                        egui::TextEdit::singleline(&mut text)
                    }
                    .desired_width(f32::INFINITY);
                    if let Some(n) = f.max_len {
                        edit = edit.char_limit(n as usize);
                    }
                    let resp = ui.add(edit).labelled_by(label_id);
                    if resp.changed() {
                        drafts.insert(f.name.clone(), text.clone());
                    }
                    if resp.lost_focus() {
                        let escape = ui.input(|i| i.key_pressed(Key::Escape));
                        if escape || text == current {
                            drafts.remove(&f.name);
                        } else {
                            let value = FieldValue::Text(text.clone());
                            if f.validate(&value).is_ok() {
                                commits.push((f.name.clone(), value));
                            } else {
                                *error = true;
                            }
                        }
                    }
                }
                FieldKind::Checkbox => {
                    let mut on = matches!(f.value, FieldValue::Bool(true));
                    let label = tr(
                        lang,
                        if on {
                            "a11y.forms.checked"
                        } else {
                            "a11y.forms.unchecked"
                        },
                    );
                    if ui.checkbox(&mut on, label).labelled_by(label_id).changed() {
                        commits.push((f.name.clone(), FieldValue::Bool(on)));
                    }
                }
                FieldKind::Radio => {
                    let mut value = chosen(f).first().cloned();
                    let before = value.clone();
                    for o in &f.options {
                        ui.radio_value(&mut value, Some(o.value.clone()), &o.label)
                            .labelled_by(label_id);
                    }
                    if !f.no_toggle_off {
                        ui.radio_value(&mut value, None, tr(lang, "forms.panel.choice_none"))
                            .labelled_by(label_id);
                    }
                    if value != before {
                        commits.push((
                            f.name.clone(),
                            FieldValue::Choice(value.into_iter().collect()),
                        ));
                    }
                }
                FieldKind::Combo => {
                    let current = chosen(f).first().cloned();
                    let selected = current
                        .as_deref()
                        .map_or_else(|| tr(lang, "forms.panel.choice_none"), |v| f.label_of(v));
                    let mut value = current.clone();
                    egui::ComboBox::from_id_salt(("form-combo", &f.name))
                        .selected_text(selected)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            if !f.required {
                                ui.selectable_value(
                                    &mut value,
                                    None,
                                    tr(lang, "forms.panel.choice_none"),
                                );
                            }
                            for o in &f.options {
                                ui.selectable_value(&mut value, Some(o.value.clone()), &o.label);
                            }
                        })
                        .response
                        .labelled_by(label_id);
                    if value != current {
                        commits.push((
                            f.name.clone(),
                            FieldValue::Choice(value.into_iter().collect()),
                        ));
                    }
                }
                FieldKind::List => {
                    let current = chosen(f);
                    if f.multi_select {
                        let mut wanted = current.clone();
                        let mut changed = false;
                        for o in &f.options {
                            let mut on = wanted.contains(&o.value);
                            if ui
                                .checkbox(&mut on, &o.label)
                                .labelled_by(label_id)
                                .changed()
                            {
                                changed = true;
                                if on {
                                    wanted.push(o.value.clone());
                                } else {
                                    wanted.retain(|x| x != &o.value);
                                }
                            }
                        }
                        if changed {
                            commits.push((f.name.clone(), ordered_choice(f, &wanted)));
                        }
                    } else {
                        let mut value = current.first().cloned();
                        let before = value.clone();
                        ui.radio_value(&mut value, None, tr(lang, "forms.panel.choice_none"))
                            .labelled_by(label_id);
                        for o in &f.options {
                            ui.radio_value(&mut value, Some(o.value.clone()), &o.label)
                                .labelled_by(label_id);
                        }
                        if value != before {
                            commits.push((
                                f.name.clone(),
                                FieldValue::Choice(value.into_iter().collect()),
                            ));
                        }
                    }
                }
                FieldKind::PushButton | FieldKind::Signature => {
                    let key = if f.kind == FieldKind::Signature {
                        "forms.panel.unsupported_signature"
                    } else {
                        "forms.panel.unsupported_button"
                    };
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr(lang, key)).size(12.0).color(p.text_muted),
                        )
                        .wrap(),
                    );
                }
            });
        });
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::FieldWidget;

    fn rect(left: f32, bottom: f32, right: f32, top: f32) -> PointRect {
        PointRect {
            left,
            bottom,
            right,
            top,
        }
    }

    fn field(name: &str, kind: FieldKind, widgets: Vec<(usize, PointRect)>) -> FormField {
        FormField {
            name: name.into(),
            kind,
            value: FieldValue::None,
            tooltip: None,
            read_only: false,
            required: false,
            multiline: false,
            password: false,
            comb: false,
            max_len: None,
            multi_select: false,
            editable: false,
            no_toggle_off: false,
            options: Vec::new(),
            widgets: widgets
                .into_iter()
                .map(|(page, r)| FieldWidget {
                    page,
                    rect: r,
                    display: r,
                    on_state: None,
                })
                .collect(),
        }
    }

    #[test]
    fn reading_order_sorts_by_page_then_top_then_left() {
        let info = FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![
                field(
                    "bas",
                    FieldKind::Text,
                    vec![(0, rect(10.0, 100.0, 50.0, 120.0))],
                ),
                field(
                    "page2",
                    FieldKind::Text,
                    vec![(1, rect(10.0, 700.0, 50.0, 720.0))],
                ),
                field(
                    "haut-droite",
                    FieldKind::Text,
                    vec![(0, rect(300.0, 702.0, 400.0, 722.0))],
                ),
                field(
                    "haut-gauche",
                    FieldKind::Text,
                    vec![(0, rect(10.0, 700.0, 50.0, 720.0))],
                ),
                field("orphelin", FieldKind::Text, vec![]),
                field(
                    "deux",
                    FieldKind::Text,
                    vec![
                        (0, rect(10.0, 400.0, 50.0, 420.0)),
                        (0, rect(10.0, 500.0, 50.0, 520.0)),
                    ],
                ),
            ],
        };
        let names: Vec<String> = reading_order(&info)
            .into_iter()
            .map(|(f, w)| format!("{}:{w}", info.fields[f].name))
            .collect();
        assert_eq!(
            names,
            [
                "haut-gauche:0",
                "haut-droite:0",
                "deux:1",
                "deux:0",
                "bas:0",
                "page2:0"
            ]
        );
    }

    #[test]
    fn kind_keys_and_value_labels_exist_in_both_catalogs() {
        for kind in FieldKind::ALL {
            let key = kind_key(kind);
            for lang in Language::ALL {
                assert_ne!(tr(lang, &key), key, "{key} absente en {}", lang.code());
            }
        }
        let mut f = field("f", FieldKind::Checkbox, vec![]);
        f.value = FieldValue::Bool(true);
        assert_eq!(value_label(Language::French, &f), "coché");
        f.kind = FieldKind::Combo;
        f.options = vec![sunipdf_core::FieldOption {
            value: "BE".into(),
            label: "Belgique".into(),
        }];
        f.value = FieldValue::Choice(vec!["BE".into()]);
        assert_eq!(value_label(Language::English, &f), "Belgique");
        f.value = FieldValue::Choice(vec![]);
        assert_eq!(value_label(Language::English, &f), "(none)");
        assert_eq!(
            ordered_choice(&f, &["XX".into(), "BE".into()]),
            FieldValue::Choice(vec!["BE".into()])
        );
    }

    #[test]
    fn reset_keeps_the_panel_open_flag() {
        let mut s = FormsState {
            panel_open: true,
            stale: true,
            scope_all: true,
            ..Default::default()
        };
        s.drafts.insert("a".into(), "b".into());
        s.reset_for_document();
        assert!(s.panel_open);
        assert!(!s.scope_all && !s.stale && s.drafts.is_empty());
    }
}
