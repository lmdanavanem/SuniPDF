//! Formulaires AcroForm : lecture du modèle de champs, remplissage, effacement, aplatissement.
//!
//! **Lecture et écriture par lopdf, rendu et apparences par PDFium** (D061). `pdfium-render` 0.9.4
//! sait lire un champ mais ne l'écrit pas correctement (pas de régénération d'apparence, valeur
//! écrite sur le widget au lieu du champ parent, aucun setter pour les listes) : on écrit `/V`,
//! `/AS` et `/I` sur les octets du document, on supprime les flux d'apparence `/AP` des widgets
//! texte et liste modifiés, on recharge, puis un rendu minuscule (`ensure_appearances`) force PDFium
//! à générer et mémoriser les apparences manquantes, qui sont alors écrites par `save_to_bytes`.
//!
//! Un document chiffré reste chiffré : `security::load` / `security::save` conservent la clé et
//! les permissions d'origine. Les formulaires XFA sont détectés et lus, jamais écrits ni aplatis.
//! Aucun script PDF n'est exécuté (`FormInfo.has_scripts` le signale).

use crate::geometry::PageGeometry;
use crate::security::lopdf_err;
use crate::{Document, EngineError, PointRect};
use lopdf::{dictionary, Dictionary, Object, ObjectId};
use pdfium_render::prelude::*;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormKind {
    #[default]
    None,
    AcroForm,
    Xfa,
}

impl FormKind {
    pub fn id(self) -> &'static str {
        match self {
            FormKind::None => "none",
            FormKind::AcroForm => "acroform",
            FormKind::Xfa => "xfa",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Checkbox,
    Radio,
    Combo,
    List,
    PushButton,
    Signature,
}

impl FieldKind {
    pub const ALL: [FieldKind; 7] = [
        FieldKind::Text,
        FieldKind::Checkbox,
        FieldKind::Radio,
        FieldKind::Combo,
        FieldKind::List,
        FieldKind::PushButton,
        FieldKind::Signature,
    ];

    /// Identifiant stable (CLI, JSON, clés i18n `forms.kind.*`).
    pub fn id(self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Checkbox => "checkbox",
            FieldKind::Radio => "radio",
            FieldKind::Combo => "combo",
            FieldKind::List => "list",
            FieldKind::PushButton => "button",
            FieldKind::Signature => "signature",
        }
    }

    pub fn from_id(id: &str) -> Option<FieldKind> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }

    /// Bouton-poussoir et signature : listés, jamais remplis.
    pub fn fillable(self) -> bool {
        !matches!(self, FieldKind::PushButton | FieldKind::Signature)
    }
}

/// Valeur d'un champ. Les choix sont des valeurs d'export : `/Opt` si présent, sinon le nom
/// d'état actif du bouton radio choisi.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldValue {
    #[default]
    None,
    Text(String),
    Bool(bool),
    Choice(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldWidget {
    /// Page (base 0).
    pub page: usize,
    /// `/Rect`, espace utilisateur de la page.
    pub rect: PointRect,
    /// Espace d'affichage (rotation et CropBox appliqués, origine en bas à gauche) : c'est lui que
    /// l'interface dessine.
    pub display: PointRect,
    /// Case ou bouton radio : nom d'état « coché » (clé de `/AP /N` autre que `Off`).
    pub on_state: Option<String>,
    /// Radio : valeur d'export du widget (`/Opt`, sinon `on_state`).
    pub export_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// Nom qualifié (`adresse.rue`).
    pub name: String,
    pub kind: FieldKind,
    pub value: FieldValue,
    /// Info-bulle `/TU`.
    pub tooltip: Option<String>,
    pub read_only: bool,
    pub required: bool,
    pub multiline: bool,
    pub password: bool,
    pub comb: bool,
    pub max_len: Option<u32>,
    pub multi_select: bool,
    pub editable: bool,
    /// Radio : une option reste toujours choisie.
    pub no_toggle_off: bool,
    /// Liste déroulante, zone de liste, radio : valeurs d'export et libellés.
    pub options: Vec<FieldOption>,
    /// Vide = champ sans emplacement visible (orphelin après un aplatissement partiel).
    pub widgets: Vec<FieldWidget>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormInfo {
    pub kind: FormKind,
    /// Calculs (`/CO`), formats ou actions JavaScript : jamais exécutés par SuniPDF.
    pub has_scripts: bool,
    /// Ordre de `/Fields` (profondeur d'abord).
    pub fields: Vec<FormField>,
}

impl FormInfo {
    pub fn field(&self, name: &str) -> Option<&FormField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    /// Première erreur = échec, rien n'est modifié.
    Strict,
    /// Les champs inconnus ou les valeurs refusées sont ignorés et signalés.
    Lenient,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillReport {
    pub applied: Vec<String>,
    pub skipped_unknown: Vec<String>,
    /// (nom, raison en anglais).
    pub skipped_invalid: Vec<(String, String)>,
}

// Masques de /Ff (ISO 32000-1, tableaux 221, 226, 228, 230) : bit n -> 1 << (n - 1). Ceux de
// `PdfFormFieldFlags` (pdfium-render) sont décalés d'un bit pour les bits que PDFium n'exporte pas.
const FF_READ_ONLY: u32 = 1 << 0;
const FF_REQUIRED: u32 = 1 << 1;
const FF_MULTILINE: u32 = 1 << 12;
const FF_PASSWORD: u32 = 1 << 13;
const FF_NO_TOGGLE_OFF: u32 = 1 << 14;
const FF_RADIO: u32 = 1 << 15;
const FF_PUSHBUTTON: u32 = 1 << 16;
const FF_COMBO: u32 = 1 << 17;
const FF_EDIT: u32 = 1 << 18;
const FF_MULTI_SELECT: u32 = 1 << 21;
const FF_COMB: u32 = 1 << 24;
/// Profondeur maximale de l'arbre `/Kids` (PDF hostile).
const MAX_DEPTH: usize = 32;

impl FormField {
    /// Option par valeur d'export, à défaut par libellé.
    pub fn option(&self, x: &str) -> Option<&FieldOption> {
        self.options
            .iter()
            .find(|o| o.value == x)
            .or_else(|| self.options.iter().find(|o| o.label == x))
    }

    /// Libellé d'une valeur d'export (listes), ou la valeur telle quelle.
    pub fn label_of<'a>(&'a self, value: &'a str) -> &'a str {
        self.options
            .iter()
            .find(|o| o.value == value)
            .map_or(value, |o| o.label.as_str())
    }

    /// Modifiable par l'utilisateur : ni bouton, ni signature, ni lecture seule, ni mot de passe
    /// (une valeur de champ mot de passe n'est jamais stockée).
    pub fn fillable(&self) -> bool {
        self.kind.fillable() && !self.read_only && !self.password
    }

    /// Valeur « vide » d'un champ que « Effacer les champs » peut changer : `None` si le champ est
    /// hors d'atteinte (lecture seule, mot de passe, bouton, signature, radio « toujours un
    /// choix ») ou déjà vide.
    pub fn cleared(&self) -> Option<FieldValue> {
        if !self.fillable() {
            return None;
        }
        let empty = match self.kind {
            FieldKind::Text => FieldValue::Text(String::new()),
            FieldKind::Checkbox => FieldValue::Bool(false),
            FieldKind::Radio if !self.no_toggle_off => FieldValue::Choice(Vec::new()),
            FieldKind::Combo | FieldKind::List => FieldValue::Choice(Vec::new()),
            _ => return None,
        };
        (self.value != empty).then_some(empty)
    }

    /// Vérifie une valeur sans rien modifier. La raison est en anglais (jamais affichée telle
    /// quelle par l'interface, qui n'a que la clé `error.form_value_invalid`). Un champ en lecture
    /// seule accepte sa valeur courante (aller-retour export → import sans erreur) et rien d'autre.
    pub fn validate(&self, v: &FieldValue) -> Result<(), String> {
        if self.read_only {
            return if *v == self.value {
                Ok(())
            } else {
                Err("read-only".into())
            };
        }
        match (self.kind, v) {
            (FieldKind::PushButton | FieldKind::Signature, _) => Err("not fillable".into()),
            (FieldKind::Text, FieldValue::Text(s)) => {
                if self.password {
                    Err("password fields are never stored".into())
                } else if !self.multiline && s.contains(['\n', '\r']) {
                    Err("line breaks need a multi-line field".into())
                } else if self.max_len.is_some_and(|n| s.chars().count() > n as usize) {
                    Err(format!(
                        "longer than {} characters",
                        self.max_len.unwrap_or(0)
                    ))
                } else {
                    Ok(())
                }
            }
            (FieldKind::Checkbox, FieldValue::Bool(_)) => Ok(()),
            (FieldKind::Radio, FieldValue::Choice(v)) => match v.as_slice() {
                [] if self.no_toggle_off => Err("one option must stay selected".into()),
                [] => Ok(()),
                [x] if self.options.iter().any(|o| o.value == *x) => Ok(()),
                [x] => Err(format!("unknown option: {x}")),
                _ => Err("a radio group takes one value".into()),
            },
            (FieldKind::Combo, FieldValue::Choice(v)) => match v.as_slice() {
                [] => Ok(()),
                [x] if self.editable || self.option(x).is_some() => Ok(()),
                [x] => Err(format!("unknown option: {x}")),
                _ => Err("a drop-down list takes one value".into()),
            },
            (FieldKind::List, FieldValue::Choice(v)) => {
                if v.len() > 1 && !self.multi_select {
                    Err("this list takes one value".into())
                } else if let Some(bad) = v.iter().find(|x| self.option(x).is_none()) {
                    Err(format!("unknown option: {bad}"))
                } else {
                    Ok(())
                }
            }
            _ => Err("value type does not match the field type".into()),
        }
    }
}

// ----- Modèle lopdf ----------------------------------------------------------------------------

struct RawWidget {
    id: ObjectId,
    index: usize,
    on_state: Option<String>,
    export_value: Option<String>,
    /// `/AS` courant.
    as_state: Option<String>,
    page: Option<usize>,
    rect: Option<PointRect>,
}

struct Raw {
    /// Dictionnaire qui porte `/V` (le champ, pas le widget).
    field_id: ObjectId,
    widgets: Vec<RawWidget>,
    radio_opt: bool,
    view: FormField,
}

struct Model {
    raws: Vec<Raw>,
    scripts: bool,
}

/// Attributs hérités du parent vers l'enfant (ISO 32000-1 §12.7.3.1).
#[derive(Clone, Default)]
struct Inherited {
    ft: Option<Vec<u8>>,
    ff: u32,
    v: Option<Object>,
    opt: Option<Object>,
    max_len: Option<u32>,
}

impl Inherited {
    fn merged(&self, d: &lopdf::Document, dict: &Dictionary) -> Inherited {
        Inherited {
            ft: get(d, dict, b"FT")
                .and_then(|o| o.as_name().ok())
                .map(<[u8]>::to_vec)
                .or_else(|| self.ft.clone()),
            ff: get(d, dict, b"Ff")
                .and_then(|o| o.as_i64().ok())
                .map_or(self.ff, |v| v as u32),
            v: get(d, dict, b"V").cloned().or_else(|| self.v.clone()),
            opt: get(d, dict, b"Opt").cloned().or_else(|| self.opt.clone()),
            max_len: get(d, dict, b"MaxLen")
                .and_then(|o| o.as_i64().ok())
                .map(|v| v.max(0) as u32)
                .or(self.max_len),
        }
    }
}

fn deref<'a>(d: &'a lopdf::Document, o: &'a Object) -> Option<&'a Object> {
    d.dereference(o).ok().map(|(_, o)| o)
}

fn get<'a>(d: &'a lopdf::Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    dict.get(key).ok().and_then(|o| deref(d, o))
}

/// Chaîne texte PDF décodée (PDFDocEncoding, UTF-16BE ou UTF-8 avec BOM) ; un flux est lu tel quel.
/// La table PDFDocEncoding de lopdf ignore les caractères de contrôle : les sauts de ligne d'un
/// champ multiligne (`\n`, `\r`, tabulation) sont conservés à part.
pub(crate) fn text_of(d: &lopdf::Document, o: &Object) -> Option<String> {
    let o = deref(d, o)?;
    let bytes = match o {
        Object::Stream(s) => {
            return Some(
                String::from_utf8_lossy(&s.decompressed_content().unwrap_or_default()).into_owned(),
            )
        }
        o => o.as_str().ok()?,
    };
    if bytes.starts_with(b"\xFE\xFF") || bytes.starts_with(b"\xEF\xBB\xBF") {
        return lopdf::decode_text_string(o).ok();
    }
    let mut out = String::new();
    for run in bytes.split_inclusive(|b| matches!(b, b'\n' | b'\r' | b'\t')) {
        let (body, control) = match run.last() {
            Some(c @ (b'\n' | b'\r' | b'\t')) => (&run[..run.len() - 1], Some(*c as char)),
            _ => (run, None),
        };
        if !body.is_empty() {
            let piece = Object::String(body.to_vec(), lopdf::StringFormat::Literal);
            out.push_str(&lopdf::decode_text_string(&piece).ok()?);
        }
        if let Some(c) = control {
            out.push(c);
        }
    }
    Some(out.replace("\r\n", "\n").replace('\r', "\n"))
}

fn text(d: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key).ok().and_then(|o| text_of(d, o))
}

fn name_of(o: &Object) -> Option<String> {
    o.as_name()
        .ok()
        .map(|n| String::from_utf8_lossy(n).into_owned())
}

/// Références d'un tableau (direct ou indirect).
fn ids_of(d: &lopdf::Document, o: &Object) -> Vec<ObjectId> {
    deref(d, o)
        .and_then(|a| a.as_array().ok())
        .map(|a| a.iter().filter_map(|x| x.as_reference().ok()).collect())
        .unwrap_or_default()
}

fn subtype_is(dict: &Dictionary, subtype: &[u8]) -> bool {
    dict.get(b"Subtype")
        .ok()
        .and_then(|s| s.as_name().ok())
        .is_some_and(|n| n == subtype)
}

fn is_widget(dict: &Dictionary) -> bool {
    subtype_is(dict, b"Widget")
}

fn is_link(d: &lopdf::Document, annot: &Object) -> bool {
    deref(d, annot)
        .and_then(|a| a.as_dict().ok())
        .is_some_and(|a| subtype_is(a, b"Link"))
}

/// Entrées de `/Annots` d'une page (tableau direct ou indirect), telles quelles.
fn annots_of(d: &lopdf::Document, page_id: ObjectId) -> Vec<Object> {
    d.get_dictionary(page_id)
        .ok()
        .and_then(|p| get(d, p, b"Annots"))
        .and_then(|a| a.as_array().ok())
        .cloned()
        .unwrap_or_default()
}

/// Annotations `/Link` d'une page, résolues et copiées : la navigation (URI, destination) n'a pas
/// d'apparence et `FPDFPage_Flatten` la supprimerait sans rien dessiner.
fn links_of(d: &lopdf::Document, page_id: ObjectId) -> Vec<Object> {
    annots_of(d, page_id)
        .iter()
        .filter(|a| is_link(d, a))
        .filter_map(|a| deref(d, a).cloned())
        .collect()
}

/// Copie profonde d'un objet de `src` (document avant aplatissement) vers `dst` (après) : PDFium
/// n'enregistre pas les objets que plus rien ne référence, l'action `/A` indirecte d'un lien
/// supprimé a donc disparu. Les références sont résolues et inlinées, sauf les pages (traduites
/// par `page_map`, numéro pour numéro) et les flux (réajoutés comme objets indirects). Profondeur
/// bornée : une chaîne d'actions cyclique s'arrête sur `Null`.
fn deep_copy(
    src: &lopdf::Document,
    dst: &mut lopdf::Document,
    page_map: &HashMap<ObjectId, ObjectId>,
    o: &Object,
    depth: u8,
) -> Object {
    if depth == 0 {
        return Object::Null;
    }
    let copy = |dst: &mut lopdf::Document, x: &Object| deep_copy(src, dst, page_map, x, depth - 1);
    match o {
        Object::Reference(id) => {
            if let Some(&page) = page_map.get(id) {
                return Object::Reference(page);
            }
            match src.get_object(*id) {
                Ok(Object::Stream(s)) => {
                    let mut s = s.clone();
                    s.dict = s
                        .dict
                        .iter()
                        .map(|(k, v)| (k.clone(), copy(dst, v)))
                        .collect();
                    Object::Reference(dst.add_object(Object::Stream(s)))
                }
                Ok(target) => copy(dst, target),
                Err(_) => Object::Null,
            }
        }
        Object::Dictionary(d) => {
            Object::Dictionary(d.iter().map(|(k, v)| (k.clone(), copy(dst, v))).collect())
        }
        Object::Array(a) => Object::Array(a.iter().map(|x| copy(dst, x)).collect()),
        o => o.clone(),
    }
}

/// Remet des liens relevés dans `before` dans `/Annots` d'une page aplatie de `ld` (les liens que
/// PDFium aurait laissés sont d'abord retirés : aucun doublon).
fn restore_links(
    before: &lopdf::Document,
    ld: &mut lopdf::Document,
    page_map: &HashMap<ObjectId, ObjectId>,
    page_id: ObjectId,
    links: &[Object],
) -> Result<(), EngineError> {
    let mut annots: Vec<Object> = annots_of(ld, page_id)
        .into_iter()
        .filter(|a| !is_link(ld, a))
        .collect();
    for link in links {
        let link = deep_copy(before, ld, page_map, link, 16);
        let id = ld.add_object(link);
        annots.push(Object::Reference(id));
    }
    dict_mut(ld, page_id)?.set("Annots", Object::Array(annots));
    Ok(())
}

fn kind_of_field(inh: &Inherited) -> Option<FieldKind> {
    Some(match inh.ft.as_deref()? {
        b"Tx" => FieldKind::Text,
        b"Btn" if inh.ff & FF_PUSHBUTTON != 0 => FieldKind::PushButton,
        b"Btn" if inh.ff & FF_RADIO != 0 => FieldKind::Radio,
        b"Btn" => FieldKind::Checkbox,
        b"Ch" if inh.ff & FF_COMBO != 0 => FieldKind::Combo,
        b"Ch" => FieldKind::List,
        b"Sig" => FieldKind::Signature,
        _ => return None,
    })
}

/// État « coché » d'une case ou d'un bouton radio : première clé de `/AP /N` autre que `Off`.
fn on_state(d: &lopdf::Document, w: &Dictionary) -> Option<String> {
    let ap = get(d, w, b"AP")?.as_dict().ok()?;
    let n = get(d, ap, b"N")?.as_dict().ok()?;
    n.iter()
        .map(|(k, _)| k)
        .find(|k| k.as_slice() != b"Off")
        .map(|k| String::from_utf8_lossy(k).into_owned())
}

fn rect_of(d: &lopdf::Document, w: &Dictionary) -> Option<PointRect> {
    let a = get(d, w, b"Rect")?.as_array().ok()?;
    let v: Vec<f32> = a
        .iter()
        .filter_map(|x| deref(d, x).and_then(|x| x.as_float().ok()))
        .collect();
    let [x1, y1, x2, y2] = v[..] else { return None };
    Some(PointRect {
        left: x1.min(x2),
        bottom: y1.min(y2),
        right: x1.max(x2),
        top: y1.max(y2),
    })
}

fn is_javascript(d: &lopdf::Document, action: &Object) -> bool {
    deref(d, action)
        .and_then(|a| a.as_dict().ok())
        .and_then(|a| get(d, a, b"S"))
        .and_then(|s| s.as_name().ok())
        .is_some_and(|s| s == b"JavaScript")
}

/// Action `/A` ou action additionnelle `/AA` de type JavaScript.
fn has_script(d: &lopdf::Document, dict: &Dictionary) -> bool {
    if dict.get(b"A").is_ok_and(|a| is_javascript(d, a)) {
        return true;
    }
    get(d, dict, b"AA")
        .and_then(|aa| aa.as_dict().ok())
        .is_some_and(|aa| aa.iter().any(|(_, a)| is_javascript(d, a)))
}

/// Widget → page (base 0), par les `/Annots` de chaque page (`/P` n'est pas fiable).
fn page_of_annots(d: &lopdf::Document) -> HashMap<ObjectId, usize> {
    let mut map = HashMap::new();
    for (num, page_id) in d.get_pages() {
        let Ok(page) = d.get_dictionary(page_id) else {
            continue;
        };
        if let Some(annots) = get(d, page, b"Annots").and_then(|a| a.as_array().ok()) {
            for a in annots {
                if let Ok(id) = a.as_reference() {
                    map.entry(id).or_insert(num as usize - 1);
                }
            }
        }
    }
    map
}

struct Ctx<'a> {
    d: &'a lopdf::Document,
    pages: HashMap<ObjectId, usize>,
    seen: HashSet<ObjectId>,
    out: Vec<Raw>,
    scripts: bool,
}

fn walk(ctx: &mut Ctx<'_>, id: ObjectId, prefix: &str, parent: &Inherited, depth: usize) {
    if depth > MAX_DEPTH || !ctx.seen.insert(id) {
        return;
    }
    let d = ctx.d;
    let Ok(dict) = d.get_dictionary(id) else {
        return;
    };
    let inh = parent.merged(d, dict);
    let name = match text(d, dict, b"T") {
        Some(t) if prefix.is_empty() => t,
        Some(t) => format!("{prefix}.{t}"),
        None => prefix.to_string(),
    };
    if has_script(d, dict) {
        ctx.scripts = true;
    }
    let kids: Vec<ObjectId> = dict
        .get(b"Kids")
        .ok()
        .map_or_else(Vec::new, |k| ids_of(d, k));
    let is_field = |k: ObjectId| {
        d.get_dictionary(k)
            .is_ok_and(|kd| kd.has(b"T") || kd.has(b"Kids"))
    };
    if kids.iter().any(|&k| is_field(k)) {
        for k in kids {
            walk(ctx, k, &name, &inh, depth + 1);
        }
        return;
    }
    let widget_ids = if kids.is_empty() {
        if is_widget(dict) {
            vec![id]
        } else {
            Vec::new()
        }
    } else {
        kids
    };
    if let Some(raw) = terminal(ctx, id, name, &inh, dict, widget_ids) {
        ctx.out.push(raw);
    }
}

fn terminal(
    ctx: &mut Ctx<'_>,
    field_id: ObjectId,
    name: String,
    inh: &Inherited,
    dict: &Dictionary,
    widget_ids: Vec<ObjectId>,
) -> Option<Raw> {
    let d = ctx.d;
    let Some(kind) = kind_of_field(inh) else {
        log::debug!("champ {name:?} de type inconnu ignoré");
        return None;
    };
    let kids = get(d, dict, b"Kids").and_then(|o| o.as_array().ok());
    let widget_indices: HashMap<_, _> = kids
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, o)| o.as_reference().ok().map(|id| (id, index)))
        .collect();
    let mut widgets = Vec::with_capacity(widget_ids.len());
    for (index, wid) in widget_ids.into_iter().enumerate() {
        let wd = d.get_dictionary(wid).ok();
        if wd.is_some_and(|w| has_script(d, w)) {
            ctx.scripts = true;
        }
        widgets.push(RawWidget {
            id: wid,
            index: widget_indices.get(&wid).copied().unwrap_or(index),
            on_state: wd.and_then(|w| on_state(d, w)),
            export_value: None,
            as_state: wd.and_then(|w| get(d, w, b"AS")).and_then(name_of),
            page: ctx.pages.get(&wid).copied(),
            rect: wd.and_then(|w| rect_of(d, w)),
        });
    }
    let ff = inh.ff;
    let radio_opt = kind == FieldKind::Radio && inh.opt.is_some();
    let mut radio_read_only = false;
    if kind == FieldKind::Radio {
        let exports = inh
            .opt
            .as_ref()
            .and_then(|o| deref(d, o))
            .and_then(|o| o.as_array().ok());
        let mut seen = HashSet::new();
        let mut widget_ids = HashSet::new();
        radio_read_only = radio_opt
            && (exports.is_none_or(|a| a.len() != widgets.len())
                || (dict.has(b"Kids") && kids.is_none_or(|a| a.len() != widgets.len()))
                || widgets.iter().any(|w| !widget_ids.insert(w.id)));
        for w in &mut widgets {
            w.export_value = if radio_opt {
                exports
                    .and_then(|a| a.get(w.index))
                    .and_then(|o| deref(d, o))
                    .filter(|o| matches!(o, Object::String(_, _)))
                    .and_then(|o| text_of(d, o))
            } else {
                w.on_state.clone()
            };
            if radio_opt
                && (d
                    .get_dictionary(w.id)
                    .ok()
                    .and_then(|w| get(d, w, b"AP"))
                    .and_then(|ap| ap.as_dict().ok())
                    .and_then(|ap| get(d, ap, b"N"))
                    .and_then(|n| n.as_dict().ok())
                    .is_none_or(|n| {
                        let mut states =
                            n.iter().map(|(k, _)| k).filter(|k| k.as_slice() != b"Off");
                        states
                            .next()
                            .is_none_or(|k| std::str::from_utf8(k).is_err())
                            || states.next().is_some()
                    })
                    || w.export_value
                        .as_ref()
                        .is_none_or(|s| s.is_empty() || s == "Off" || !seen.insert(s.clone())))
            {
                // shortcut: scalar radio values cannot identify duplicate/malformed exports; keep this field read-only until widget-specific filling exists.
                radio_read_only = true;
            }
        }
    }
    let options = match kind {
        FieldKind::Radio => widgets
            .iter()
            .filter_map(|w| {
                let value = w.export_value.clone()?;
                let label = d
                    .get_dictionary(w.id)
                    .ok()
                    .and_then(|w| text(d, w, b"TU"))
                    .unwrap_or_else(|| value.clone());
                Some(FieldOption { value, label })
            })
            .collect(),
        FieldKind::Combo | FieldKind::List => inh
            .opt
            .as_ref()
            .and_then(|o| deref(d, o))
            .and_then(|o| o.as_array().ok())
            .map(|a| {
                a.iter()
                    .filter_map(|item| match deref(d, item)? {
                        Object::Array(pair) => {
                            let value = text_of(d, pair.first()?)?;
                            let label = pair
                                .get(1)
                                .and_then(|l| text_of(d, l))
                                .unwrap_or_else(|| value.clone());
                            Some(FieldOption { value, label })
                        }
                        o => {
                            let s = text_of(d, o)?;
                            Some(FieldOption {
                                value: s.clone(),
                                label: s,
                            })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let v = inh.v.as_ref().and_then(|v| deref(d, v));
    let value = match kind {
        FieldKind::Text => FieldValue::Text(v.and_then(|v| text_of(d, v)).unwrap_or_default()),
        FieldKind::Checkbox => {
            let on = match widgets.first().and_then(|w| w.as_state.as_deref()) {
                Some(s) => s != "Off",
                None => v.and_then(name_of).is_some_and(|n| n != "Off"),
            };
            FieldValue::Bool(on)
        }
        FieldKind::Radio => {
            let chosen = if radio_opt {
                let checked: Vec<_> = widgets
                    .iter()
                    .filter(|w| w.on_state.is_some() && w.as_state == w.on_state)
                    .collect();
                radio_read_only |= checked.len() > 1;
                checked
                    .first()
                    .and_then(|w| w.export_value.clone())
                    .or_else(|| {
                        let state = v.and_then(name_of)?;
                        if let Ok(index) = state.parse::<usize>() {
                            widgets
                                .iter()
                                .find(|w| w.index == index)?
                                .export_value
                                .clone()
                        } else {
                            let mut matching = widgets
                                .iter()
                                .filter(|w| w.on_state.as_deref() == Some(state.as_str()));
                            let export = matching.next()?.export_value.clone();
                            matching.next().is_none().then_some(export).flatten()
                        }
                    })
            } else {
                v.and_then(name_of).filter(|n| n != "Off").or_else(|| {
                    widgets
                        .iter()
                        .filter_map(|w| w.as_state.clone())
                        .find(|s| s != "Off")
                })
            };
            FieldValue::Choice(chosen.into_iter().collect())
        }
        FieldKind::Combo | FieldKind::List => FieldValue::Choice(match v {
            Some(Object::Array(items)) => items.iter().filter_map(|i| text_of(d, i)).collect(),
            Some(o) => text_of(d, o).into_iter().collect(),
            None => Vec::new(),
        }),
        FieldKind::PushButton | FieldKind::Signature => FieldValue::None,
    };
    let view = FormField {
        name,
        kind,
        value,
        tooltip: text(d, dict, b"TU"),
        read_only: ff & FF_READ_ONLY != 0 || radio_read_only,
        required: ff & FF_REQUIRED != 0,
        multiline: ff & FF_MULTILINE != 0,
        password: ff & FF_PASSWORD != 0,
        comb: ff & FF_COMB != 0,
        max_len: inh.max_len,
        multi_select: ff & FF_MULTI_SELECT != 0,
        editable: ff & FF_EDIT != 0,
        no_toggle_off: ff & FF_NO_TOGGLE_OFF != 0,
        options,
        widgets: Vec::new(),
    };
    Some(Raw {
        field_id,
        widgets,
        radio_opt,
        view,
    })
}

fn build_model(d: &lopdf::Document) -> Model {
    let acro = d
        .catalog()
        .ok()
        .and_then(|c| get(d, c, b"AcroForm"))
        .and_then(|a| a.as_dict().ok());
    let mut ctx = Ctx {
        d,
        pages: page_of_annots(d),
        seen: HashSet::new(),
        out: Vec::new(),
        scripts: acro
            .and_then(|a| get(d, a, b"CO"))
            .and_then(|co| co.as_array().ok())
            .is_some_and(|co| !co.is_empty()),
    };
    if let Some(fields) = acro.and_then(|a| a.get(b"Fields").ok()) {
        for id in ids_of(d, fields) {
            walk(&mut ctx, id, "", &Inherited::default(), 0);
        }
    }
    Model {
        raws: ctx.out,
        scripts: ctx.scripts,
    }
}

// ----- Écriture lopdf ---------------------------------------------------------------------------

fn dict_mut(ld: &mut lopdf::Document, id: ObjectId) -> Result<&mut Dictionary, EngineError> {
    ld.get_dictionary_mut(id).map_err(lopdf_err)
}

/// Retire le flux d'apparence : PDFium le régénère depuis `/V`, `/DA`, `/MK` et `/Opt`.
fn drop_ap(ld: &mut lopdf::Document, widget: ObjectId) -> Result<(), EngineError> {
    dict_mut(ld, widget)?.remove(b"AP");
    Ok(())
}

fn name(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Applique une valeur déjà validée. Texte : `/V` sur le champ, `/AP` retiré des widgets ; case
/// et radio : `/V` + `/AS` de chaque widget, apparences conservées ; listes : `/V` (export), `/I`.
fn apply(ld: &mut lopdf::Document, raw: &Raw, value: &FieldValue) -> Result<(), EngineError> {
    let field = raw.field_id;
    match (raw.view.kind, value) {
        (FieldKind::Text, FieldValue::Text(s)) => {
            let f = dict_mut(ld, field)?;
            f.set("V", lopdf::text_string(s));
            f.remove(b"RV");
            for w in &raw.widgets {
                drop_ap(ld, w.id)?;
            }
        }
        (FieldKind::Checkbox, FieldValue::Bool(b)) => {
            let on = raw
                .widgets
                .iter()
                .find_map(|w| w.on_state.clone())
                .unwrap_or_else(|| "Yes".into());
            dict_mut(ld, field)?.set("V", name(if *b { &on } else { "Off" }));
            for w in &raw.widgets {
                let state = if *b {
                    w.on_state.as_deref().unwrap_or(&on)
                } else {
                    "Off"
                };
                dict_mut(ld, w.id)?.set("AS", name(state));
            }
        }
        (FieldKind::Radio, FieldValue::Choice(v)) => {
            let chosen = v.first().map(String::as_str);
            let selected = chosen.and_then(|c| {
                raw.widgets
                    .iter()
                    .position(|w| w.export_value.as_deref() == Some(c))
            });
            let field_value = if raw.radio_opt {
                selected.map(|i| raw.widgets[i].index.to_string())
            } else {
                chosen.map(str::to_string)
            };
            dict_mut(ld, field)?.set("V", name(field_value.as_deref().unwrap_or("Off")));
            for (index, w) in raw.widgets.iter().enumerate() {
                let on = if raw.radio_opt {
                    selected == Some(index)
                } else {
                    chosen.is_some() && w.on_state.as_deref() == chosen
                };
                let state = if on {
                    w.on_state.as_deref().unwrap_or("Off")
                } else {
                    "Off"
                };
                dict_mut(ld, w.id)?.set("AS", name(state));
            }
        }
        (FieldKind::Combo | FieldKind::List, FieldValue::Choice(v)) => {
            let exports: Vec<String> = v
                .iter()
                .map(|x| {
                    raw.view
                        .option(x)
                        .map_or_else(|| x.clone(), |o| o.value.clone())
                })
                .collect();
            let mut indices: Vec<i64> = exports
                .iter()
                .filter_map(|e| raw.view.options.iter().position(|o| o.value == *e))
                .map(|i| i as i64)
                .collect();
            indices.sort_unstable();
            let f = dict_mut(ld, field)?;
            match exports.as_slice() {
                [] => {
                    f.remove(b"V");
                    f.remove(b"I");
                }
                [one] if !(raw.view.kind == FieldKind::List && raw.view.multi_select) => {
                    f.set("V", lopdf::text_string(one));
                    f.remove(b"I");
                }
                many => {
                    f.set(
                        "V",
                        Object::Array(many.iter().map(|e| lopdf::text_string(e)).collect()),
                    );
                    f.set(
                        "I",
                        Object::Array(indices.into_iter().map(Object::Integer).collect()),
                    );
                }
            }
            for w in &raw.widgets {
                drop_ap(ld, w.id)?;
            }
        }
        _ => {
            return Err(EngineError::FormValueInvalid(format!(
                "{}: value type does not match the field type",
                raw.view.name
            )))
        }
    }
    Ok(())
}

// ----- API du document --------------------------------------------------------------------------

pub(crate) fn kind_of(doc: &PdfDocument<'_>) -> FormKind {
    match doc.form().map(|f| f.form_type()) {
        Some(PdfFormType::Acrobat) => FormKind::AcroForm,
        Some(PdfFormType::XfaFull | PdfFormType::XfaForeground) => FormKind::Xfa,
        _ => FormKind::None,
    }
}

fn radio_appearance(pdf: &mut lopdf::Document, width: f32, height: f32, checked: bool) -> ObjectId {
    let side = width.min(height);
    let mut content = format!("q {side} 0 0 {side} 0 0 cm 1 g 0 G 0.05 w 0.95 0.5 m 0.95 0.7485 0.7485 0.95 0.5 0.95 c 0.2515 0.95 0.05 0.7485 0.05 0.5 c 0.05 0.2515 0.2515 0.05 0.5 0.05 c 0.7485 0.05 0.95 0.2515 0.95 0.5 c B ");
    if checked {
        content.push_str("0 g 0.75 0.5 m 0.75 0.638 0.638 0.75 0.5 0.75 c 0.362 0.75 0.25 0.638 0.25 0.5 c 0.25 0.362 0.362 0.25 0.5 0.25 c 0.638 0.25 0.75 0.362 0.75 0.5 c f ");
    }
    content.push('Q');
    pdf.add_object(lopdf::Stream::new(dictionary! { "Type"=>"XObject", "Subtype"=>"Form", "BBox"=>vec![0.into(),0.into(),width.into(),height.into()], "Resources"=>Dictionary::new() }, content.into_bytes()))
}

impl Document<'_> {
    /// Adds a fillable text field or checkbox in display coordinates, preserving protection.
    pub fn add_form_field(
        &mut self,
        page_index: usize,
        name: &str,
        kind: FieldKind,
        rect: PointRect,
    ) -> Result<(), EngineError> {
        self.add_form_field_with_options(page_index, name, kind, rect, &[], false)
    }

    /// Creates a field or radio group in display points, with custom combo text disabled.
    pub fn add_form_field_with_options(
        &mut self,
        page_index: usize,
        name: &str,
        kind: FieldKind,
        rect: PointRect,
        options: &[FieldOption],
        multi_select: bool,
    ) -> Result<(), EngineError> {
        self.add_form_field_with_flags(page_index, name, kind, rect, options, multi_select, false)
    }

    /// Creates a field or radio group in display points. Editable text is supported only for combos.
    #[allow(clippy::too_many_arguments)]
    pub fn add_form_field_with_flags(
        &mut self,
        page_index: usize,
        name: &str,
        kind: FieldKind,
        rect: PointRect,
        options: &[FieldOption],
        multi_select: bool,
        editable: bool,
    ) -> Result<(), EngineError> {
        let choice = matches!(kind, FieldKind::Radio | FieldKind::Combo | FieldKind::List);
        if !matches!(
            kind,
            FieldKind::Text
                | FieldKind::Checkbox
                | FieldKind::Radio
                | FieldKind::Combo
                | FieldKind::List
        ) {
            return Err(EngineError::FormValueInvalid(
                "this field type cannot be created".into(),
            ));
        }
        let mut seen = HashSet::new();
        if (choice && options.is_empty())
            || options.len() > 1000
            || (!choice && !options.is_empty())
            || (multi_select && kind != FieldKind::List)
            || (editable && kind != FieldKind::Combo)
            || options.iter().any(|o| {
                o.value.trim().is_empty()
                    || o.value.len() > 1024
                    || o.value.chars().any(char::is_control)
                    || o.label.len() > 4096
                    || o.label.chars().any(char::is_control)
                    || !seen.insert(o.value.as_str())
                    || (kind == FieldKind::Radio && o.value == "Off")
            })
            || (kind == FieldKind::Radio
                && (rect.top - rect.bottom) / (options.len().max(1) as f32) < 4.0)
        {
            return Err(EngineError::FormValueInvalid(
                "invalid, duplicate or missing field options".into(),
            ));
        }
        if name.trim().is_empty()
            || name.chars().count() > 256
            || name.chars().any(|c| c == '.' || c.is_control())
        {
            return Err(EngineError::FormValueInvalid("invalid field name".into()));
        }
        if self.form_kind() == FormKind::Xfa {
            return Err(EngineError::FormXfa);
        }
        let geometry = self.page_geometry(page_index)?;
        let (width, height) = geometry.display_size();
        if [rect.left, rect.bottom, rect.right, rect.top]
            .iter()
            .any(|v| !v.is_finite())
            || rect.left < 0.0
            || rect.bottom < 0.0
            || rect.right > width
            || rect.top > height
            || rect.right - rect.left < 4.0
            || rect.top - rect.bottom < 4.0
        {
            return Err(EngineError::FormValueInvalid(
                "field rectangle must be inside the page and at least 4 points wide and high"
                    .into(),
            ));
        }
        let mut pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let model = build_model(&pdf);
        if model
            .raws
            .iter()
            .any(|f| f.view.name == name || f.view.name.starts_with(&format!("{name}.")))
        {
            return Err(EngineError::FormValueInvalid(
                "field name already exists".into(),
            ));
        }
        let page_id = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        let mut acro = match pdf.catalog().map_err(lopdf_err)?.get(b"AcroForm") {
            Ok(o) => deref(&pdf, o)
                .and_then(|o| o.as_dict().ok())
                .ok_or(EngineError::InvalidFormat)?
                .clone(),
            Err(_) => Dictionary::new(),
        };
        let mut fields = match acro.get(b"Fields") {
            Ok(o) => deref(&pdf, o)
                .and_then(|o| o.as_array().ok())
                .ok_or(EngineError::InvalidFormat)?
                .clone(),
            Err(_) => Vec::new(),
        };
        let mut dr = match acro.get(b"DR") {
            Ok(o) => deref(&pdf, o)
                .and_then(|o| o.as_dict().ok())
                .ok_or(EngineError::InvalidFormat)?
                .clone(),
            Err(_) => Dictionary::new(),
        };
        let mut fonts = match dr.get(b"Font") {
            Ok(o) => deref(&pdf, o)
                .and_then(|o| o.as_dict().ok())
                .ok_or(EngineError::InvalidFormat)?
                .clone(),
            Err(_) => Dictionary::new(),
        };
        let mut font_name = "SuniPDFFormHelv".to_owned();
        while fonts.has(font_name.as_bytes()) {
            font_name.push('_');
        }
        let font = pdf.add_object(lopdf::dictionary! { "Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica", "Encoding"=>"WinAnsiEncoding" });
        fonts.set(font_name.as_bytes(), font);
        dr.set("Font", fonts);
        acro.set("DR", dr);
        let user = geometry.rect_to_user(rect);
        let mut widget = lopdf::dictionary! {
            "Type"=>"Annot", "Subtype"=>"Widget", "FT"=>match kind { FieldKind::Text => "Tx", FieldKind::Combo | FieldKind::List => "Ch", _ => "Btn" },
            "T"=>lopdf::text_string(name), "P"=>page_id, "F"=>4, "Ff"=>0,
            "Rect"=>vec![user.left.into(),user.bottom.into(),user.right.into(),user.top.into()],
            "DA"=>Object::string_literal(format!("/{font_name} 12 Tf 0 g")),
            "MK"=>lopdf::dictionary! { "BC"=>vec![0.into(),0.into(),0.into()], "BG"=>vec![1.into(),1.into(),1.into()] },
            "BS"=>lopdf::dictionary! { "W"=>1,"S"=>"S" },
        };
        if choice {
            widget.set(
                "Opt",
                Object::Array(
                    options
                        .iter()
                        .map(|o| {
                            Object::Array(vec![
                                lopdf::text_string(&o.value),
                                lopdf::text_string(&o.label),
                            ])
                        })
                        .collect(),
                ),
            );
            widget.set(
                "Ff",
                match kind {
                    FieldKind::Radio => FF_RADIO,
                    FieldKind::Combo => FF_COMBO | if editable { FF_EDIT } else { 0 },
                    _ if multi_select => FF_MULTI_SELECT,
                    _ => 0,
                } as i64,
            );
        }
        if kind == FieldKind::Text {
            widget.set("V", lopdf::text_string(""));
        } else if kind == FieldKind::Checkbox {
            widget.set("V", Object::Name(b"Off".to_vec()));
            widget.set("AS", Object::Name(b"Off".to_vec()));
            widget
                .get_mut(b"MK")
                .and_then(Object::as_dict_mut)
                .map_err(lopdf_err)?
                .set("CA", Object::string_literal("4"));
        }
        if kind == FieldKind::Radio {
            widget.remove(b"Subtype");
            widget.remove(b"Rect");
            widget.remove(b"Opt");
            widget.set("V", Object::Name(b"Off".to_vec()));
            let parent_id = pdf.add_object(widget);
            let mut kids = Vec::new();
            let row_height = (rect.top - rect.bottom) / options.len() as f32;
            for (i, option) in options.iter().enumerate() {
                let top = rect.top - i as f32 * row_height;
                let side = row_height.min(rect.right - rect.left).min(20.0);
                let row = geometry.rect_to_user(PointRect {
                    left: rect.left,
                    bottom: top - side,
                    right: rect.left + side,
                    top,
                });
                let off =
                    radio_appearance(&mut pdf, row.right - row.left, row.top - row.bottom, false);
                let on =
                    radio_appearance(&mut pdf, row.right - row.left, row.top - row.bottom, true);
                let mut appearances = dictionary! { "Off" => off };
                appearances.set(option.value.as_bytes(), on);
                let child = pdf.add_object(dictionary! {
                    "Type"=>"Annot", "Subtype"=>"Widget", "Parent"=>parent_id,
                    "P"=>page_id, "F"=>4, "TU"=>lopdf::text_string(&option.label),
                    "Rect"=>vec![row.left.into(),row.bottom.into(),row.right.into(),row.top.into()],
                    "AS"=>Object::Name(b"Off".to_vec()), "AP"=>dictionary! { "N"=>appearances },
                });
                crate::annot_write::annots_mut(&mut pdf, page_id)?.push(Object::Reference(child));
                kids.push(Object::Reference(child));
            }
            pdf.get_dictionary_mut(parent_id)
                .map_err(lopdf_err)?
                .set("Kids", kids);
            fields.push(Object::Reference(parent_id));
        } else {
            let widget_id = pdf.add_object(widget);
            crate::annot_write::annots_mut(&mut pdf, page_id)?.push(Object::Reference(widget_id));
            fields.push(Object::Reference(widget_id));
        }
        acro.set("Fields", fields);
        let acro_id = pdf.add_object(acro);
        pdf.catalog_mut()
            .map_err(lopdf_err)?
            .set("AcroForm", acro_id);
        let bytes = crate::security::save(pdf)?;
        self.restore(bytes)?;
        self.ensure_appearances(page_index)
    }

    pub fn form_kind(&self) -> FormKind {
        kind_of(&self.inner)
    }

    /// Champs du formulaire (instantané → lopdf → modèle) ; `FormInfo::default()` sans AcroForm.
    /// Les rectangles d'affichage viennent de la géométrie PDFium de chaque page.
    pub fn form_info(&self) -> Result<FormInfo, EngineError> {
        let kind = self.form_kind();
        if kind == FormKind::None {
            return Ok(FormInfo::default());
        }
        let bytes = self.snapshot()?;
        let ld = crate::security::load(&bytes, self.password())?;
        let model = build_model(&ld);
        let mut geos: HashMap<usize, Option<PageGeometry>> = HashMap::new();
        let mut fields = Vec::with_capacity(model.raws.len());
        for raw in model.raws {
            let mut view = raw.view;
            for w in raw.widgets {
                let (Some(page), Some(rect)) = (w.page, w.rect) else {
                    continue;
                };
                let geo = *geos
                    .entry(page)
                    .or_insert_with(|| self.page_geometry(page).ok());
                let Some(geo) = geo else { continue };
                view.widgets.push(FieldWidget {
                    page,
                    rect,
                    display: geo.rect_to_display(rect),
                    on_state: w.on_state,
                    export_value: w.export_value,
                });
            }
            fields.push(view);
        }
        Ok(FormInfo {
            kind,
            has_scripts: model.scripts,
            fields,
        })
    }

    /// Rend la page en quelques pixels : PDFium crée sa vue de formulaire et génère puis mémorise
    /// les apparences manquantes, écrites ensuite par `snapshot` / `save_as`.
    pub fn ensure_appearances(&self, page_index: usize) -> Result<(), EngineError> {
        let size = self.page_size(page_index)?;
        let scale = 8.0 / size.width_pt.min(size.height_pt).max(1.0);
        self.render_page(page_index, scale).map(|_| ())
    }

    /// Écrit des valeurs (validation complète avant toute mutation) ; `Strict` : première erreur =
    /// `FormFieldNotFound` / `FormValueInvalid`, document intact. Un document chiffré reste chiffré.
    pub fn fill_form(
        &mut self,
        values: &[(String, FieldValue)],
        mode: FillMode,
    ) -> Result<FillReport, EngineError> {
        if self.form_kind() == FormKind::Xfa {
            return Err(EngineError::FormXfa);
        }
        let bytes = self.snapshot()?;
        let mut ld = crate::security::load(&bytes, self.password())?;
        let model = build_model(&ld);
        let mut report = FillReport::default();
        let mut plan: Vec<(usize, &FieldValue)> = Vec::new();
        for (name, value) in values {
            let Some(i) = model.raws.iter().position(|r| r.view.name == *name) else {
                match mode {
                    FillMode::Strict => return Err(EngineError::FormFieldNotFound(name.clone())),
                    FillMode::Lenient => report.skipped_unknown.push(name.clone()),
                }
                continue;
            };
            match model.raws[i].view.validate(value) {
                Ok(()) => plan.push((i, value)),
                Err(reason) => match mode {
                    FillMode::Strict => {
                        return Err(EngineError::FormValueInvalid(format!("{name}: {reason}")))
                    }
                    FillMode::Lenient => report.skipped_invalid.push((name.clone(), reason)),
                },
            }
        }
        // Lecture seule : la validation n'a accepté que la valeur courante, rien à écrire (et
        // surtout pas d'apparence à régénérer).
        plan.retain(|&(i, _)| !model.raws[i].view.read_only);
        if plan.is_empty() {
            return Ok(report);
        }
        let mut pages = BTreeSet::new();
        for (i, value) in plan {
            let raw = &model.raws[i];
            apply(&mut ld, raw, value)?;
            pages.extend(raw.widgets.iter().filter_map(|w| w.page));
            report.applied.push(raw.view.name.clone());
        }
        let out = crate::security::save(ld)?;
        self.restore(out)?;
        for p in pages {
            if p < self.page_count() {
                self.ensure_appearances(p)?;
            }
        }
        Ok(report)
    }

    /// Vide les champs modifiables (texte, case, listes, radio sans « toujours un choix ») ;
    /// renvoie le nombre de champs changés.
    pub fn clear_form(&mut self) -> Result<usize, EngineError> {
        let info = self.form_info()?;
        let values: Vec<(String, FieldValue)> = info
            .fields
            .iter()
            .filter_map(|f| f.cleared().map(|v| (f.name.clone(), v)))
            .collect();
        Ok(self.fill_form(&values, FillMode::Lenient)?.applied.len())
    }

    /// Aplatit champs **et** annotations dans le contenu des pages (`FPDFPage_Flatten`, mode
    /// impression) ; `None` = toutes les pages, et `/AcroForm` est alors retiré du catalogue.
    /// Renvoie le nombre d'annotations aplaties. Les annotations sans indicateur « Print » le
    /// reçoivent d'abord (sinon PDFium les supprime sans les dessiner), les apparences manquantes
    /// sont générées (sinon un champ jamais affiché disparaît) et les liens (`/Link`, sans
    /// apparence : PDFium les supprimerait) sont relevés avant puis remis dans `/Annots`.
    pub fn flatten(&mut self, pages: Option<&[usize]>) -> Result<usize, EngineError> {
        if self.form_kind() == FormKind::Xfa {
            return Err(EngineError::FormXfa);
        }
        let total = self.page_count();
        let targets: BTreeSet<usize> = match pages {
            Some(p) => p.iter().copied().collect(),
            None => (0..total).collect(),
        };
        if let Some(&bad) = targets.iter().find(|&&p| p >= total) {
            return Err(EngineError::PageOutOfRange(bad));
        }
        let before = crate::security::load(&self.snapshot()?, self.password())?;
        let page_ids = before.get_pages();
        let links: Vec<(usize, Vec<Object>)> = targets
            .iter()
            .filter_map(|&p| {
                let id = page_ids.get(&(p as u32 + 1))?;
                let l = links_of(&before, *id);
                (!l.is_empty()).then_some((p, l))
            })
            .collect();
        let mut count = 0;
        for &p in &targets {
            self.ensure_appearances(p)?;
            let mut page = self.page_handle(p)?;
            for mut a in page.annotations().iter() {
                if a.annotation_type() == PdfPageAnnotationType::Link {
                    continue;
                }
                count += 1;
                if !a.is_hidden() && !a.is_printed() {
                    a.set_is_printed(true)?;
                }
            }
            page.flatten()?;
        }
        // Rechargement : l'état de formulaire de PDFium ne survit pas aux widgets disparus, et le
        // catalogue perd son /AcroForm quand plus aucune page n'a de champ.
        let bytes = self.snapshot()?;
        let all = targets.len() == total && self.form_kind() != FormKind::None;
        let bytes = if all || !links.is_empty() {
            let mut ld = crate::security::load(&bytes, self.password())?;
            if all {
                ld.catalog_mut().map_err(lopdf_err)?.remove(b"AcroForm");
            }
            // Pages d'avant → pages d'après, numéro pour numéro (PDFium garde les numéros
            // d'objet, la traduction ne dépend pas de cette hypothèse).
            let after_ids = ld.get_pages();
            let page_map: HashMap<ObjectId, ObjectId> = page_ids
                .iter()
                .filter_map(|(n, &id)| Some((id, *after_ids.get(n)?)))
                .collect();
            for (p, l) in &links {
                if let Some(&id) = after_ids.get(&(*p as u32 + 1)) {
                    restore_links(&before, &mut ld, &page_map, id, l)?;
                }
            }
            ld.prune_objects();
            crate::security::save(ld)?
        } else {
            bytes
        };
        self.restore(bytes)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Oracle PDFium : les widgets énumérés par `FPDFAnnot_GetFormField*` (nom qualifié, type)
    /// sont exactement ceux du modèle lopdf. Seul test unitaire du crate à charger PDFium.
    #[test]
    fn lopdf_model_matches_pdfium_enumeration() {
        let engine = crate::Engine::new().expect("pdfium.dll : scripts/fetch-pdfium.ps1");
        let doc = engine.open(&fixture("acroform.pdf"), None).unwrap();
        let bytes = doc.snapshot().unwrap();
        let ld = lopdf::Document::load_mem(&bytes).unwrap();
        let model = build_model(&ld);
        let mut ours: Vec<(String, &str)> = model
            .raws
            .iter()
            .flat_map(|r| {
                r.widgets
                    .iter()
                    .map(move |_| (r.view.name.clone(), r.view.kind.id()))
            })
            .collect();
        let page = doc.page_handle(0).unwrap();
        let mut theirs: Vec<(String, &str)> = page
            .annotations()
            .iter()
            .filter_map(|a| {
                let f = a.as_form_field()?;
                let kind = match f.field_type() {
                    PdfFormFieldType::Text => "text",
                    PdfFormFieldType::Checkbox => "checkbox",
                    PdfFormFieldType::RadioButton => "radio",
                    PdfFormFieldType::ComboBox => "combo",
                    PdfFormFieldType::ListBox => "list",
                    PdfFormFieldType::PushButton => "button",
                    PdfFormFieldType::Signature => "signature",
                    other => panic!("{other:?}"),
                };
                Some((f.name().unwrap_or_default(), kind))
            })
            .collect();
        ours.sort();
        theirs.sort();
        assert_eq!(ours.len(), 16);
        assert_eq!(ours, theirs);
        assert!(!model.scripts);
    }

    fn field(kind: FieldKind) -> FormField {
        FormField {
            name: "f".into(),
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
            options: vec![
                FieldOption {
                    value: "FR".into(),
                    label: "France".into(),
                },
                FieldOption {
                    value: "BE".into(),
                    label: "Belgique".into(),
                },
            ],
            widgets: Vec::new(),
        }
    }

    #[test]
    fn validation_rules() {
        let t = |s: &str| FieldValue::Text(s.into());
        let c = |v: &[&str]| FieldValue::Choice(v.iter().map(|s| s.to_string()).collect());
        let text = field(FieldKind::Text);
        assert!(text.validate(&t("a")).is_ok());
        assert!(text.validate(&t("a\nb")).is_err(), "une ligne");
        assert!(text.validate(&FieldValue::Bool(true)).is_err(), "type");
        let multi = FormField {
            multiline: true,
            max_len: Some(3),
            ..field(FieldKind::Text)
        };
        assert!(multi.validate(&t("a\nb")).is_ok());
        assert!(multi.validate(&t("éàù")).is_ok(), "3 caractères");
        assert!(multi.validate(&t("abcd")).is_err());
        let ro = FormField {
            read_only: true,
            value: FieldValue::Bool(false),
            ..field(FieldKind::Checkbox)
        };
        assert_eq!(
            ro.validate(&FieldValue::Bool(true)),
            Err("read-only".into())
        );
        assert!(
            ro.validate(&FieldValue::Bool(false)).is_ok(),
            "valeur courante acceptée (aller-retour export → import)"
        );
        assert!(!ro.fillable() && ro.cleared().is_none());
        let secret = FormField {
            password: true,
            value: FieldValue::Text("x".into()),
            ..field(FieldKind::Text)
        };
        assert!(!secret.fillable() && secret.cleared().is_none());
        let filled = FormField {
            value: FieldValue::Text("x".into()),
            ..field(FieldKind::Text)
        };
        assert_eq!(filled.cleared(), Some(FieldValue::Text(String::new())));
        let empty = FormField {
            value: FieldValue::Text(String::new()),
            ..field(FieldKind::Text)
        };
        assert_eq!(empty.cleared(), None, "déjà vide");
        let combo = field(FieldKind::Combo);
        assert!(combo.validate(&c(&["Belgique"])).is_ok(), "libellé accepté");
        assert!(combo.validate(&c(&["XX"])).is_err());
        assert!(combo.validate(&c(&["FR", "BE"])).is_err());
        assert!(combo.validate(&c(&[])).is_ok());
        let editable = FormField {
            editable: true,
            ..field(FieldKind::Combo)
        };
        assert!(editable.validate(&c(&["libre"])).is_ok());
        let list = field(FieldKind::List);
        assert!(list.validate(&c(&["FR", "BE"])).is_err(), "choix unique");
        let multi_list = FormField {
            multi_select: true,
            ..field(FieldKind::List)
        };
        assert!(multi_list.validate(&c(&["FR", "BE"])).is_ok());
        assert!(multi_list.validate(&c(&["FR", "XX"])).is_err());
        let radio = FormField {
            no_toggle_off: true,
            ..field(FieldKind::Radio)
        };
        assert!(radio.validate(&c(&[])).is_err());
        assert!(radio.validate(&c(&["FR"])).is_ok());
        assert!(field(FieldKind::Radio).validate(&c(&[])).is_ok());
        assert_eq!(
            FormField {
                value: c(&["FR"]),
                ..field(FieldKind::Radio)
            }
            .cleared(),
            Some(c(&[]))
        );
        assert_eq!(
            FormField {
                no_toggle_off: true,
                value: c(&["FR"]),
                ..field(FieldKind::Radio)
            }
            .cleared(),
            None
        );
        for k in [FieldKind::PushButton, FieldKind::Signature] {
            assert!(field(k).validate(&FieldValue::None).is_err());
        }
        assert_eq!(FieldKind::from_id("combo"), Some(FieldKind::Combo));
        assert_eq!(FieldKind::from_id("nope"), None);
        assert_eq!(combo.label_of("BE"), "Belgique");
        assert_eq!(combo.label_of("??"), "??");
    }
}
