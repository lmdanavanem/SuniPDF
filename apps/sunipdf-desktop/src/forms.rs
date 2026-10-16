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
    form_data, Command, EngineError, FieldKind, FieldValue, FormField, FormInfo, FormKind,
    PointRect,
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
    pub read_failed: bool,
    /// Saisies du panneau non validées, par nom de champ.
    pub drafts: HashMap<String, String>,
    pub staged: HashMap<String, FieldValue>,
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
    pub keyboard_focus: Option<(String, usize)>,
}

impl FormsState {
    pub fn has_unsaved_input(&self) -> bool {
        !self.drafts.is_empty()
            || !self.staged.is_empty()
            || self.editing.as_ref().is_some_and(|e| {
                self.info
                    .as_ref()
                    .and_then(|i| i.field(&e.field))
                    .is_none_or(|f| {
                        f.value
                            != input_value(
                                f,
                                &e.text,
                                self.staged.get(&e.field).unwrap_or(&f.value),
                            )
                    })
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

fn input_text(f: &FormField, value: &FieldValue) -> String {
    match value {
        FieldValue::Text(text) => text.clone(),
        FieldValue::Choice(values) => values
            .first()
            .map_or_else(String::new, |value| f.label_of(value).to_string()),
        _ => String::new(),
    }
}

fn input_value(f: &FormField, text: &str, current: &FieldValue) -> FieldValue {
    if f.kind == FieldKind::Combo {
        if text == input_text(f, current) {
            return current.clone();
        }
        FieldValue::Choice(if text.is_empty() {
            Vec::new()
        } else {
            vec![f
                .option(text)
                .map_or(text, |option| &option.value)
                .to_string()]
        })
    } else {
        FieldValue::Text(text.to_string())
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
    fn require_form_model(&mut self) -> bool {
        if self.forms.read_failed {
            self.error = Some("error.form_read_failed");
            return false;
        }
        true
    }

    /// Valide toutes les saisies avant d'en envoyer une ; une erreur garde les textes intacts.
    pub fn commit_form_input(&mut self) -> bool {
        if !self.require_form_model() {
            return false;
        }
        let mut drafts = self.forms.drafts.clone();
        if let Some(e) = &self.forms.editing {
            drafts.insert(e.field.clone(), e.text.clone());
        }
        let mut pending = self.forms.staged.clone();
        let mut combo_drafts = Vec::new();
        for (name, text) in &drafts {
            let Some(field) = self.forms.info.as_ref().and_then(|i| i.field(name)) else {
                self.error = Some("error.form_value_invalid");
                return false;
            };
            let value = input_value(field, text, pending.get(name).unwrap_or(&field.value));
            if field.kind == FieldKind::Combo {
                combo_drafts.push((name.clone(), value.clone()));
            }
            pending.insert(name.clone(), value);
        }
        let mut values = Vec::new();
        for (name, value) in pending {
            let field = self.forms.info.as_ref().and_then(|i| i.field(&name));
            if field.is_none_or(|f| f.validate(&value).is_err()) {
                self.error = Some("error.form_value_invalid");
                return false;
            }
            if field.is_some_and(|f| f.value != value) {
                values.push((name, value));
            }
        }
        self.forms.editing = None;
        self.forms.drafts = drafts;
        self.forms.staged.extend(combo_drafts);
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
        if self.error == Some("error.form_read_failed") {
            self.error = None;
        }
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
            f.read_failed = false;
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
            || !f.staged.is_empty()
            || f.editing.is_some()
            || f.pending_import.is_some()
            || f.pending_clear;
        if wanted && (f.stale || (f.info.is_none() && !f.read_failed)) && !f.requested {
            f.requested = true;
            f.stale = false;
            self.worker.send(Request::FormInfo);
        } else if f.pending_clear && !f.stale && !f.requested && !f.read_failed && f.info.is_some()
        {
            self.clear_form_now();
        }
    }

    pub fn on_form_read_failed(&mut self, error: EngineError) {
        log::warn!("form read failed: {error}");
        self.forms.requested = false;
        self.forms.stale = false;
        self.forms.read_failed = true;
        self.error = Some("error.form_read_failed");
    }

    pub fn retry_form_read(&mut self) {
        if self.error == Some("error.form_read_failed") {
            self.error = None;
        }
        self.forms.stale = true;
        self.refresh_forms_if_needed();
    }

    pub fn on_form_info(&mut self, info: FormInfo) {
        self.forms.requested = false;
        self.forms.stale = false;
        self.forms.read_failed = false;
        if self.error == Some("error.form_read_failed") {
            self.error = None;
        }
        // Un brouillon égal au modèle n'a plus de raison d'être ; un brouillon refusé reste.
        let staged = &self.forms.staged;
        self.forms.drafts.retain(|name, draft| {
            info.field(name).is_some_and(|f| {
                f.value != input_value(f, draft, staged.get(name).unwrap_or(&f.value))
            })
        });
        self.forms
            .staged
            .retain(|name, value| info.field(name).is_some_and(|f| f.value != *value));
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
        if !self.require_form_model() {
            return;
        }
        let lang = self.lang();
        let Some(info) = &mut self.forms.info else {
            return;
        };
        if values
            .iter()
            .any(|(name, value)| info.field(name).is_none_or(|f| f.validate(value).is_err()))
        {
            self.error = Some("error.form_value_invalid");
            return;
        }
        let accepted: Vec<_> = values
            .into_iter()
            .filter(|(name, value)| {
                info.field(name)
                    .is_some_and(|f| !f.read_only && f.value != *value)
            })
            .collect();
        let Some((name, _)) = accepted.last() else {
            return;
        };
        self.edit.pending_status = Some(tr_args(lang, "status.form_updated", &[("name", name)]));
        for (name, value) in &accepted {
            if let Some(f) = info.fields.iter_mut().find(|f| f.name == *name) {
                f.value = value.clone();
            }
        }
        self.worker
            .send(Request::Edit(EditOp::FillForm { values: accepted }));
    }

    pub fn commit_form(&mut self, name: String, value: FieldValue) {
        self.commit_many(vec![(name, value)]);
    }

    fn stage_form_values(&mut self, values: Vec<(String, FieldValue)>) {
        let Some(info) = &self.forms.info else { return };
        if values
            .iter()
            .any(|(name, value)| info.field(name).is_none_or(|f| f.validate(value).is_err()))
        {
            self.error = Some("error.form_value_invalid");
            return;
        }
        for (name, value) in values {
            if matches!(value, FieldValue::Choice(_)) {
                self.forms.drafts.remove(&name);
            }
            if info.field(&name).is_some_and(|f| f.value == value) {
                self.forms.staged.remove(&name);
                self.forms.drafts.remove(&name);
            } else {
                self.forms.staged.insert(name, value);
            }
        }
    }

    // ----- Page : surcouche et saisie en place --------------------------------------------------

    /// Rectangles des widgets de la page (outil Remplir actif) ; renvoie le widget cliqué.
    pub fn forms_overlay(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        page_h_pt: f32,
    ) -> Option<(usize, usize)> {
        if self.annotate.tool != Tool::FillForm || self.forms.read_failed || self.forms.requested {
            return None;
        }
        let info = self.forms.info.as_ref()?;
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let k = self.effective_zoom * SCREEN_PER_PDF_POINT;
        let clip = ui.clip_rect();
        let mut hit = None;
        let mut focused = false;
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
                    if fillable {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    },
                );
                resp.widget_info(|| match f.kind {
                    FieldKind::Checkbox => egui::WidgetInfo::selected(
                        widget_type,
                        fillable,
                        matches!(f.value, FieldValue::Bool(true)),
                        label.clone(),
                    ),
                    FieldKind::Radio => egui::WidgetInfo::selected(
                        widget_type,
                        fillable,
                        w.export_value
                            .as_ref()
                            .or(w.on_state.as_ref())
                            .is_some_and(|value| chosen(f).contains(value)),
                        format!(
                            "{}: {}",
                            label,
                            f.label_of(
                                w.export_value
                                    .as_deref()
                                    .or(w.on_state.as_deref())
                                    .unwrap_or("")
                            )
                        ),
                    ),
                    _ => egui::WidgetInfo::labeled(widget_type, fillable, label.clone()),
                });
                if fillable
                    && self
                        .forms
                        .keyboard_focus
                        .as_ref()
                        .is_some_and(|(name, widget)| name == &f.name && *widget == wi)
                {
                    resp.request_focus();
                    focused = true;
                }
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
                if fillable && resp.clicked() && !self.dialog_open() {
                    hit = Some((fi, wi));
                    // Keyboard and accessibility activation retain focus for Space/Enter and Tab.
                    if resp.clicked_by(egui::PointerButton::Primary) {
                        resp.surrender_focus();
                    }
                }
            }
        }
        if focused {
            self.forms.keyboard_focus = None;
        }
        hit
    }

    /// Clic sur un widget : bascule (case, radio), éditeur (texte) ou liste (choix).
    pub fn forms_activate(&mut self, page: usize, fi: usize, wi: usize) {
        if !self.require_form_model() {
            return;
        }
        let lang = self.lang();
        let Some(info) = &self.forms.info else { return };
        let Some(f) = info.fields.get(fi) else { return };
        let Some(w) = f.widgets.get(wi) else { return };
        self.forms.focus = None;
        if !f.fillable() || info.kind == FormKind::Xfa {
            self.status_msg = Some(tr(lang, "status.form_unsupported_widget").to_string());
            return;
        }
        let f = f.clone();
        let name = f.name.clone();
        match f.kind {
            FieldKind::Checkbox => {
                let on = !matches!(f.value, FieldValue::Bool(true));
                self.commit_form(name, FieldValue::Bool(on));
            }
            FieldKind::Radio => {
                let Some(on) = w.export_value.clone().or_else(|| w.on_state.clone()) else {
                    return;
                };
                let already = chosen(&f).first() == Some(&on);
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
                self.open_form_editor(f, page, wi);
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

    fn open_form_editor(&mut self, field: FormField, page: usize, widget: usize) {
        let text = self
            .forms
            .drafts
            .get(&field.name)
            .cloned()
            .unwrap_or_else(|| {
                input_text(
                    &field,
                    self.forms.staged.get(&field.name).unwrap_or(&field.value),
                )
            });
        self.forms.popup = None;
        self.forms.editing = Some(InPlace {
            page,
            field: field.name,
            widget,
            text,
            multiline: field.kind == FieldKind::Text && field.multiline,
            max_len: if field.kind == FieldKind::Text {
                field.max_len
            } else {
                None
            },
            focus: true,
            armed: false,
        });
    }

    /// Valide la saisie en cours de l'éditeur posé sur la page (texte changé → `commit_form`) et
    /// le ferme ; renvoie l'éditeur fermé. Appelé par Entrée, Tab, un clic ailleurs, la sortie
    /// du widget de la zone visible et l'enregistrement (Ctrl+S).
    pub fn commit_editor(&mut self) -> Option<InPlace> {
        if !self.require_form_model() {
            return None;
        }
        let e = self.forms.editing.as_ref()?;
        let Some(field) = self.forms.info.as_ref().and_then(|i| i.field(&e.field)) else {
            self.error = Some("error.form_value_invalid");
            return None;
        };
        let value = input_value(
            field,
            &e.text,
            self.forms.staged.get(&e.field).unwrap_or(&field.value),
        );
        let combo = field.kind == FieldKind::Combo;
        if field.validate(&value).is_err() {
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
        if current != Some(value.clone()) {
            if combo {
                self.forms.staged.insert(e.field.clone(), value.clone());
            } else {
                self.forms.staged.remove(&e.field);
            }
            self.forms.drafts.insert(e.field.clone(), e.text.clone());
            self.commit_form(e.field.clone(), value);
        } else if self.forms.drafts.contains_key(&e.field) {
            if combo {
                self.forms.staged.insert(e.field.clone(), value);
            }
            self.forms.drafts.insert(e.field.clone(), e.text.clone());
        } else {
            self.forms.staged.remove(&e.field);
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
        if self.forms.info.is_none() || self.forms.read_failed || self.forms.requested {
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
        let commit_keys = e.armed && (enter || tab);
        e.armed = true;
        if escape {
            self.forms.editing = None;
        } else if commit_keys || commit_click {
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
            if !target.fillable() {
                continue;
            }
            let widget = &target.widgets[w];
            let (page, display) = (widget.page, widget.display);
            let h = self
                .document
                .as_ref()
                .and_then(|d| d.info.page_sizes.get(page))
                .map_or(0.0, |s| s.height_pt);
            let name = target.name.clone();
            let kind = target.kind;
            self.go_to_page_point(page, display.left, (h - display.top).max(0.0));
            if matches!(kind, FieldKind::Checkbox | FieldKind::Radio) {
                self.forms.keyboard_focus = Some((name, w));
            } else {
                self.forms_activate(page, f, w);
            }
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
        if self.forms.read_failed || self.forms.requested {
            return;
        }
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
        let mut custom_input = false;
        let mut focused = false;
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
                            if f.kind == FieldKind::Combo && f.editable {
                                let response = ui.button(tr(lang, "forms.panel.custom_value"));
                                if !armed {
                                    response.request_focus();
                                    focused = true;
                                }
                                custom_input = response.clicked();
                                ui.separator();
                            }
                            if !multi && !f.required {
                                let none = tr(lang, "forms.panel.choice_none");
                                let response = ui.selectable_label(current.is_empty(), none);
                                if !armed && !focused {
                                    response.request_focus();
                                    focused = true;
                                }
                                if response.clicked() {
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
                                    if !armed && !focused {
                                        resp.request_focus();
                                        focused = true;
                                    }
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
                                    if !armed && !focused {
                                        resp.request_focus();
                                        focused = true;
                                    }
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
        if custom_input {
            self.open_form_editor(f, page, widget);
        } else if let Some(v) = new_value {
            self.forms.drafts.remove(&f.name);
            self.forms.staged.remove(&f.name);
            self.commit_form(f.name.clone(), ordered_choice(&f, &v));
            if !multi {
                self.forms.popup = None;
                self.forms.keyboard_focus = Some((field, widget));
            }
        } else if escape || (armed && pressed_outside) {
            self.forms.popup = None;
            if escape {
                self.forms.keyboard_focus = Some((field, widget));
            }
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
        let pending = self.forms.has_unsaved_input();
        let (mut apply, mut discard) = (false, false);
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
        if self.forms.read_failed {
            banner(ui, "error.form_read_failed");
            let retry = ui
                .add_enabled(!self.forms.requested, Button::new(tr(lang, "error.retry")))
                .clicked();
            if self.forms.requested {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr(lang, "forms.panel.loading"));
                });
            }
            self.forms.info = info;
            if close {
                self.execute(ctx, Command::ToggleForms);
            }
            if retry {
                self.retry_form_read();
            }
            return;
        }
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
                        ui.label(tr(lang, "forms.panel.grouped_hint"));
                        ui.horizontal(|ui| {
                            apply = ui
                                .add_enabled(pending, Button::new(tr(lang, "forms.panel.apply")))
                                .clicked();
                            discard = ui
                                .add_enabled(pending, Button::new(tr(lang, "forms.panel.discard")))
                                .clicked();
                        });
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
                    let staged = &self.forms.staged;
                    let mut error = false;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut last_page = None;
                            rows.sort_by_key(|&(f, _)| info.fields[f].widgets[0].page);
                            // `reading_order` est déjà trié par page : le tri stable garde l'ordre.
                            for (fi, _) in rows {
                                let mut f = info.fields[fi].clone();
                                if let Some(value) = staged.get(&f.name) {
                                    f.value = value.clone();
                                }
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
                                    &f,
                                    editable,
                                    drafts,
                                    &mut commits,
                                    &mut go,
                                    &mut error,
                                );
                            }
                            for fi in orphans {
                                let mut f = info.fields[fi].clone();
                                if let Some(value) = staged.get(&f.name) {
                                    f.value = value.clone();
                                }
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
                                    &f,
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
            self.stage_form_values(commits);
        }
        if discard {
            self.forms.staged.clear();
            self.forms.drafts.clear();
        } else if apply {
            self.commit_form_input();
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
            if !self.require_form_model() {
                return;
            }
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

    fn form_data_dialog(&self, title_key: &str, importing: bool) -> rfd::FileDialog {
        let dialog = rfd::FileDialog::new()
            .set_title(self.t(title_key))
            .add_filter(self.t("forms.filter_xfdf"), &["xfdf"])
            .add_filter(self.t("forms.filter_json"), &["json"]);
        if importing {
            dialog.add_filter(self.t("forms.filter_fdf"), &["fdf"])
        } else {
            dialog
        }
    }

    /// Exporter les valeurs : `<document>.json` proposé à côté du PDF.
    pub fn tool_export_form(&mut self) {
        let Some(doc) = &self.document else { return };
        let suggested = doc.info.path.with_extension("xfdf");
        let mut dialog = self
            .form_data_dialog("forms.save_export", false)
            .set_file_name(file_name(&suggested));
        if let Some(dir) = suggested.parent().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut out) = dialog.save_file() else {
            return;
        };
        if out.extension().is_none() {
            out.set_extension("xfdf");
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
        let Some(path) = self.form_data_dialog("forms.pick_import", true).pick_file() else {
            return;
        };
        let value = (|| {
            use std::io::Read;
            let file = std::fs::File::open(&path).ok()?;
            let mut bytes = Vec::new();
            file.take(form_data::IMPORT_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            form_data::parse_values(
                &bytes,
                path.extension().and_then(|s| s.to_str()).unwrap_or(""),
            )
            .ok()
        })();
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
        let plan = match form_data::strict_plan_from_json(info, json) {
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
            ui.add_enabled_ui(enabled, |ui| {
                if f.kind == FieldKind::Text || (f.kind == FieldKind::Combo && f.editable) {
                    let current = input_text(f, &f.value);
                    // Un brouillon n'existe que pendant la frappe : sinon le champ suit le modèle.
                    let mut text = drafts
                        .get(&f.name)
                        .cloned()
                        .unwrap_or_else(|| current.clone());
                    let mut edit = if f.kind == FieldKind::Text && f.multiline {
                        egui::TextEdit::multiline(&mut text).desired_rows(3)
                    } else {
                        egui::TextEdit::singleline(&mut text)
                    }
                    .desired_width(f32::INFINITY)
                    .id(egui::Id::new(("form-panel-input", &f.name)));
                    if f.kind == FieldKind::Combo {
                        edit = edit.hint_text(tr(lang, "forms.panel.custom_value"));
                    }
                    if let Some(n) = f.max_len.filter(|_| f.kind == FieldKind::Text) {
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
                            let value = input_value(f, &text, &f.value);
                            if f.validate(&value).is_ok() {
                                commits.push((f.name.clone(), value));
                            } else {
                                *error = true;
                            }
                        }
                    }
                }
                match f.kind {
                    FieldKind::Text => {}
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
                                    ui.selectable_value(
                                        &mut value,
                                        Some(o.value.clone()),
                                        &o.label,
                                    );
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
                    export_value: None,
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
    #[test]
    fn grouped_form_commit_is_atomic_and_sends_one_undoable_edit() {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let mut text = field("Text", FieldKind::Text, vec![]);
        text.value = FieldValue::Text("old".into());
        let mut choice = field("Choice", FieldKind::Combo, vec![]);
        choice.options = vec![sunipdf_core::FieldOption {
            value: "one".into(),
            label: "One".into(),
        }];
        choice.value = FieldValue::Choice(Vec::new());
        let info = FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![text, choice],
        };
        app.forms.info = Some(info.clone());
        app.commit_many(vec![
            ("Text".into(), FieldValue::Text("new".into())),
            ("Choice".into(), FieldValue::Choice(vec!["invalid".into()])),
        ]);
        assert_eq!(app.forms.info.as_ref().unwrap(), &info);
        assert!(requests.try_recv().is_err());
        app.apply_import(&serde_json::json!({"Text":"new", "missing":"value"}));
        assert_eq!(app.forms.info.as_ref().unwrap(), &info);
        assert!(requests.try_recv().is_err());
        app.forms.drafts.insert("Text".into(), "old".into());
        app.stage_form_values(vec![("Text".into(), FieldValue::Text("old".into()))]);
        assert!(!app.forms.has_unsaved_input());
        app.stage_form_values(vec![
            ("Text".into(), FieldValue::Text("new".into())),
            ("Choice".into(), FieldValue::Choice(vec!["one".into()])),
        ]);
        assert_eq!(app.forms.info.as_ref().unwrap(), &info);
        assert!(app.forms.has_unsaved_input());
        assert!(requests.try_recv().is_err());
        app.forms.drafts.insert("missing".into(), "invalid".into());
        assert!(!app.commit_form_input());
        assert_eq!(app.forms.staged.len(), 2);
        assert!(requests.try_recv().is_err());
        app.forms.drafts.clear();
        assert!(app.commit_form_input());
        let confirmed = app.forms.info.clone().unwrap();
        assert_eq!(app.forms.staged.len(), 2);
        assert!(
            matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values.len() == 2)
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        assert!(
            app.commit_form_input(),
            "the optimistic model prevents duplicate submission"
        );
        assert!(requests.try_recv().is_err());
        responses
            .send(crate::worker::Response::OperationFailed(
                sunipdf_core::EngineError::Other("injected form failure".into()),
            ))
            .unwrap();
        responses
            .send(crate::worker::Response::MutationFinished)
            .unwrap();
        app.handle_responses(&egui::Context::default());
        app.on_form_info(info);
        assert_eq!(
            app.forms.staged.len(),
            2,
            "failed engine commits keep every pending value"
        );
        assert!(app.forms.has_unsaved_input());
        assert!(app.commit_form_input());
        assert!(
            matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values.len() == 2)
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        app.on_form_info(confirmed);
        assert!(
            app.forms.staged.is_empty(),
            "only confirmed values remove pending changes"
        );
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn page_radio_uses_export_values_and_preserves_toggle_rules() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let mut radio = field(
            "Language",
            FieldKind::Radio,
            vec![(0, rect(40.0, 50.0, 60.0, 70.0))],
        );
        radio.value = FieldValue::Choice(Vec::new());
        radio.options = vec![sunipdf_core::FieldOption {
            value: "EN".into(),
            label: "English".into(),
        }];
        radio.widgets[0].on_state = Some("0".into());
        radio.widgets[0].export_value = Some("EN".into());
        app.forms.info = Some(FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![radio],
        });
        for values in [vec!["EN".into()], Vec::new()] {
            app.forms_activate(0, 0, 0);
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::FillForm { values: actual })
                    if actual == vec![("Language".into(), FieldValue::Choice(values))]
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            assert!(requests.try_recv().is_err());
        }
        let radio = &mut app.forms.info.as_mut().unwrap().fields[0];
        radio.value = FieldValue::Choice(vec!["EN".into()]);
        radio.no_toggle_off = true;
        app.forms_activate(0, 0, 0);
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn page_checkbox_space_and_enter_activate_and_preserve_keyboard_focus() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let mut checkbox = field(
            "Approved",
            FieldKind::Checkbox,
            vec![(0, rect(40.0, 50.0, 60.0, 70.0))],
        );
        checkbox.value = FieldValue::Bool(false);
        app.forms.info = Some(FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![checkbox],
        });
        app.annotate.tool = Tool::FillForm;
        app.effective_zoom = 1.0;
        let ctx = egui::Context::default();
        let id = egui::Id::new(("form-widget", 0usize, 0usize, 0usize));
        let frame = |app: &mut SuniApp, events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 1200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let painter = ui.painter().clone();
                    if let Some((f, w)) = app.forms_overlay(
                        ui,
                        &painter,
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 1100.0)),
                        0,
                        792.0,
                    ) {
                        app.forms_activate(0, f, w);
                    }
                },
            );
            out.textures_delta.clear();
        };
        frame(&mut app, Vec::new());
        ctx.memory_mut(|m| m.request_focus(id));
        for (key, expected) in [(Key::Space, true), (Key::Enter, false)] {
            frame(
                &mut app,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Approved".into(), FieldValue::Bool(expected))])
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            assert!(requests.try_recv().is_err());
            assert!(ctx.memory(|m| m.has_focus(id)));
            frame(
                &mut app,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
    }

    fn enter(pressed: bool) -> egui::Event {
        egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn editable_combo_panel_stages_typed_values_and_keeps_them_after_read_failure() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let mut combo = field("Country", FieldKind::Combo, vec![]);
        combo.editable = true;
        combo.value = FieldValue::Choice(Vec::new());
        combo.options = vec![sunipdf_core::FieldOption {
            value: "FR".into(),
            label: "France".into(),
        }];
        assert_eq!(
            input_value(&combo, "France", &combo.value),
            FieldValue::Choice(vec!["FR".into()])
        );
        assert_eq!(
            input_value(&combo, "", &combo.value),
            FieldValue::Choice(Vec::new())
        );
        let mut ambiguous = combo.clone();
        ambiguous.value = FieldValue::Choice(vec!["FR".into()]);
        ambiguous.options.push(sunipdf_core::FieldOption {
            value: "France".into(),
            label: "Other export".into(),
        });
        assert_eq!(
            input_value(
                &ambiguous,
                &input_text(&ambiguous, &ambiguous.value),
                &ambiguous.value
            ),
            ambiguous.value
        );
        let info = FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![combo.clone()],
        };
        app.forms.info = Some(info.clone());
        let ctx = egui::Context::default();
        icons::install(&ctx);
        let frame = |app: &mut SuniApp, events| {
            let mut commits = Vec::new();
            let mut error = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    field_card(
                        ui,
                        Language::English,
                        &theme::palette(false),
                        &combo,
                        true,
                        &mut app.forms.drafts,
                        &mut commits,
                        &mut None,
                        &mut error,
                    )
                },
            );
            output.textures_delta.clear();
            assert!(!error);
            app.stage_form_values(commits);
        };
        frame(&mut app, Vec::new());
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("form-panel-input", "Country"))));
        frame(&mut app, vec![egui::Event::Text("Zürich".into())]);
        assert_eq!(app.forms.drafts["Country"], "Zürich");
        frame(&mut app, vec![enter(true)]);
        let value = FieldValue::Choice(vec!["Zürich".into()]);
        assert_eq!(app.forms.staged["Country"], value);
        assert!(!app.forms.drafts.contains_key("Country"));
        assert!(requests.try_recv().is_err());
        app.on_form_read_failed(EngineError::Other("injected read failure".into()));
        assert!(!app.commit_form_input());
        assert_eq!(app.forms.staged["Country"], value);
        assert!(requests.try_recv().is_err());
        app.on_form_info(info);
        assert!(app.commit_form_input());
        assert!(
            matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Country".into(), value.clone())])
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        app.on_form_info(app.forms.info.clone().unwrap());
        assert!(!app.forms.has_unsaved_input());
        combo.editable = false;
        app.forms.info.as_mut().unwrap().fields[0] = combo;
        app.forms.drafts.insert("Country".into(), "unknown".into());
        assert!(!app.commit_form_input());
        assert!(requests.try_recv().is_err());
        assert_eq!(app.forms.drafts["Country"], "unknown");
    }

    #[test]
    fn staged_editable_combo_with_duplicate_labels_keeps_its_export_value() {
        for via_editor in [false, true] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let mut combo = field("Choice", FieldKind::Combo, vec![]);
            combo.editable = true;
            combo.value = FieldValue::Choice(vec!["A".into()]);
            combo.options = ["A", "B"]
                .into_iter()
                .map(|value| sunipdf_core::FieldOption {
                    value: value.into(),
                    label: "Same".into(),
                })
                .collect();
            let info = FormInfo {
                kind: FormKind::AcroForm,
                has_scripts: false,
                fields: vec![combo.clone()],
            };
            let wanted = FieldValue::Choice(vec!["B".into()]);
            app.forms.info = Some(info.clone());
            app.stage_form_values(vec![("Choice".into(), wanted.clone())]);
            app.open_form_editor(combo, 0, 0);
            assert_eq!(app.forms.editing.as_ref().unwrap().text, "Same");
            if via_editor {
                assert!(app.commit_editor().is_some());
            } else {
                assert!(app.commit_form_input());
            }
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Choice".into(), wanted.clone())])
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            let confirmed = app.forms.info.clone().unwrap();
            app.on_form_info(info);
            assert_eq!(app.forms.staged["Choice"], wanted);
            assert_eq!(app.forms.drafts["Choice"], "Same");
            assert!(app.commit_form_input());
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Choice".into(), wanted.clone())])
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            app.on_form_info(confirmed);
            assert!(!app.forms.has_unsaved_input());
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn editable_combo_popup_opens_text_editor_and_enter_commits_one_choice() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let mut combo = field(
            "Country",
            FieldKind::Combo,
            vec![(0, rect(40.0, 50.0, 200.0, 76.0))],
        );
        combo.editable = true;
        combo.value = FieldValue::Choice(vec!["FR".into()]);
        combo.options = vec![sunipdf_core::FieldOption {
            value: "FR".into(),
            label: "France".into(),
        }];
        let info = FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![combo],
        };
        app.forms.info = Some(info.clone());
        app.effective_zoom = 1.0;
        app.forms_activate(0, 0, 0);
        let ctx = egui::Context::default();
        let frame = |app: &mut SuniApp, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 1200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |_| {
                    let bounds =
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 1100.0));
                    app.forms_popup(&ctx, bounds, bounds, 0, 792.0);
                    app.forms_editor(&ctx, bounds, bounds, 0, 792.0);
                },
            );
            output.textures_delta.clear();
        };
        frame(&mut app, Vec::new());
        frame(&mut app, vec![enter(true)]);
        assert!(app.forms.popup.is_none());
        assert_eq!(app.forms.editing.as_ref().unwrap().text, "France");
        assert!(!app.forms.has_unsaved_input());
        assert!(
            requests.try_recv().is_err(),
            "the opening Enter must not immediately close the editor"
        );
        frame(
            &mut app,
            vec![enter(false), egui::Event::Text("Québec".into())],
        );
        assert_eq!(app.forms.editing.as_ref().unwrap().text, "Québec");
        frame(&mut app, vec![enter(true)]);
        let value = FieldValue::Choice(vec!["Québec".into()]);
        assert!(
            matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Country".into(), value)])
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        assert!(app.forms.editing.is_none());
        assert_eq!(app.forms.drafts["Country"], "Québec");
        let optimistic = app.forms.info.as_ref().unwrap().fields[0].clone();
        app.open_form_editor(optimistic, 0, 0);
        assert!(app.commit_editor().is_some());
        assert_eq!(
            app.forms.drafts["Country"], "Québec",
            "an unchanged editor cannot confirm an unconfirmed write"
        );
        assert!(requests.try_recv().is_err());
        app.on_form_read_failed(EngineError::Other("injected read failure".into()));
        assert!(!app.commit_form_input());
        assert_eq!(app.forms.drafts["Country"], "Québec");
        let confirmed = app.forms.info.clone().unwrap();
        app.on_form_info(info);
        assert_eq!(
            app.forms.drafts["Country"], "Québec",
            "a failed write keeps the raw draft"
        );
        app.on_form_info(confirmed);
        assert!(!app.forms.has_unsaved_input());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn editable_combo_desktop_values_roundtrip_rotated_and_encrypted_pdfs() {
        if std::env::var_os("SUNIPDF_EDITABLE_COMBO_TEST_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "forms::tests::editable_combo_desktop_values_roundtrip_rotated_and_encrypted_pdfs"])
                .env("SUNIPDF_EDITABLE_COMBO_TEST_CHILD", "1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let engine = Box::leak(Box::new(sunipdf_core::Engine::new().unwrap()));
        let source = include_bytes!("../../../crates/sunipdf-core/tests/fixtures/acroform.pdf");
        for rotation in [0, 90, 180, 270] {
            for encrypted in [false, true] {
                let mut pdf = lopdf::Document::load_mem(source).unwrap();
                let page = pdf.get_pages()[&1];
                let dict = pdf.get_dictionary_mut(page).unwrap();
                dict.set("Rotate", rotation);
                dict.set(
                    "CropBox",
                    vec![20.into(), 30.into(), 550.into(), 760.into()],
                );
                let combo = pdf
                    .objects
                    .values_mut()
                    .find_map(|object| {
                        let dict = object.as_dict_mut().ok()?;
                        (dict.get(b"T").ok()?.as_str().ok()? == b"pays").then_some(dict)
                    })
                    .unwrap();
                combo.set("Ff", (1 << 17) | (1 << 18));
                let mut bytes = Vec::new();
                pdf.save_to(&mut bytes).unwrap();
                if encrypted {
                    bytes = sunipdf_core::security::protect(
                        &bytes,
                        None,
                        &sunipdf_core::Protection {
                            user_password: "user".into(),
                            owner_password: "owner".into(),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                let password = encrypted.then_some("user");
                let path = std::path::Path::new("editable-combo.pdf");
                let mut doc = engine.open_bytes(bytes, password, path).unwrap();
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.forms.info = Some(doc.form_info().unwrap());
                let field = app.forms.info.as_ref().unwrap().field("pays").unwrap();
                assert!(field.editable);
                app.forms
                    .drafts
                    .insert("pays".into(), "Zürich libre".into());
                assert!(app.commit_form_input());
                let Request::Edit(EditOp::FillForm { values }) = requests.try_recv().unwrap()
                else {
                    panic!("expected a grouped form edit")
                };
                assert_eq!(
                    values,
                    vec![(
                        "pays".into(),
                        FieldValue::Choice(vec!["Zürich libre".into()])
                    )]
                );
                doc.fill_form(&values, sunipdf_core::FillMode::Strict)
                    .unwrap();
                let saved = doc.snapshot().unwrap();
                assert_eq!(
                    lopdf::Document::load_mem(&saved).unwrap().is_encrypted(),
                    encrypted
                );
                let reopened = engine.open_bytes(saved, password, path).unwrap();
                let confirmed = reopened.form_info().unwrap();
                assert_eq!(confirmed.field("pays").unwrap().value, values[0].1);
                for extension in ["json", "xfdf"] {
                    let exported = form_data::export_values(&confirmed, extension).unwrap();
                    let imported = form_data::parse_values(&exported, extension).unwrap();
                    let plan = form_data::strict_plan_from_json(&confirmed, &imported).unwrap();
                    assert!(plan.values.contains(&values[0]));
                }
                assert!(reopened.render_page(0, 1.0).is_ok());
                app.on_form_info(confirmed);
                assert!(!app.forms.has_unsaved_input());
            }
        }
    }

    #[test]
    fn keyboard_opened_choice_focuses_an_option_and_enter_applies_it_once() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let mut combo = field(
            "Country",
            FieldKind::Combo,
            vec![(0, rect(40.0, 50.0, 200.0, 76.0))],
        );
        combo.required = true;
        combo.value = FieldValue::Choice(Vec::new());
        combo.options = vec![sunipdf_core::FieldOption {
            value: "FR".into(),
            label: "France".into(),
        }];
        app.forms.info = Some(FormInfo {
            kind: FormKind::AcroForm,
            has_scripts: false,
            fields: vec![combo],
        });
        app.effective_zoom = 1.0;
        app.forms_activate(0, 0, 0);
        let ctx = egui::Context::default();
        let frame = |app: &mut SuniApp, events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 1200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |_| {
                    let bounds =
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 1100.0));
                    app.forms_popup(&ctx, bounds, bounds, 0, 792.0);
                },
            );
            out.textures_delta.clear();
        };
        frame(&mut app, Vec::new());
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::FillForm { values }) if values == vec![("Country".into(), FieldValue::Choice(vec!["FR".into()]))])
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Request::FinishMutation
        ));
        app.worker.finish_mutation();
        assert!(requests.try_recv().is_err());
        assert!(app.forms.popup.is_none());
        assert_eq!(app.forms.keyboard_focus, Some(("Country".into(), 0)));
    }
}
