//! Annotations avancées écrites par SuniPDF : encre, texte libre, rectangle, ellipse, ligne, flèche,
//! lien, réponses et statuts de relecture, ainsi que leur édition (déplacer, redimensionner,
//! recolorer, modifier le texte, supprimer un fil).
//!
//! **Écriture par lopdf, rendu par PDFium** (D067) : `pdfium-render` 0.9.4 ne sait créer ni cercle,
//! ni ligne, ni trait d'encre, et n'expose ni `/DA`, ni `/BS`, ni `/IRT`. Chaque annotation reçoit
//! donc un dictionnaire standard et un flux d'apparence `/AP` explicite (elle s'affiche à
//! l'identique dans les autres lecteurs). Les écritures passent par `security::load` / `save` :
//! un document chiffré garde sa protection.
//!
//! Toutes les coordonnées publiques sont en **espace d'affichage** (voir `geometry.rs`).

use crate::appearance::{self, Ap};
use crate::geometry::PageGeometry;
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PointRect};
use lopdf::{Dictionary, Object, ObjectId, Stream};
use std::collections::{HashMap, HashSet};

/// Préfixe du nom `/NM` des annotations écrites par SuniPDF : seules celles-ci voient leur
/// apparence régénérée (redimensionnement, style), jamais celles d'un autre producteur.
pub const NAME_PREFIX: &str = "SuniPDF-";

/// Traits d'encre : plafonds contre un tracé pathologique.
const MAX_STROKES: usize = 64;
const MAX_POINTS_PER_STROKE: usize = 4_000;
const MAX_WIDTH_PT: f32 = 50.0;
/// Tolérance de simplification d'un trait (points).
const INK_EPSILON: f32 = 0.4;

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotStyle {
    pub rgb: (u8, u8, u8),
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub width_pt: f32,
    pub opacity: f32,
    /// Dash/gap lengths in points; empty means a solid border.
    pub dash_pattern: Vec<f32>,
}

impl Default for AnnotStyle {
    fn default() -> Self {
        Self {
            rgb: (0xDC, 0x26, 0x26),
            fill_rgb: None,
            width_pt: 2.0,
            opacity: 1.0,
            dash_pattern: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FreeTextStyle {
    pub size_pt: f32,
    pub rgb: (u8, u8, u8),
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub border_pt: f32,
}

impl Default for FreeTextStyle {
    fn default() -> Self {
        Self {
            size_pt: 12.0,
            rgb: (0, 0, 0),
            fill_rgb: None,
            border_pt: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotShape {
    Square,
    Circle,
    Line,
    Arrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Page de destination (index à partir de 0).
    Page(usize),
    /// Reference a unique existing named destination without changing its original key bytes.
    Named(String),
    /// Explicit target-page display coordinates and viewport retention semantics.
    Destination(crate::Destination),
    Uri(String),
    /// Unsupported action (launch, JavaScript, remote destination): never executed.
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewStatus {
    #[default]
    None,
    Accepted,
    Rejected,
    Cancelled,
    Completed,
}

impl ReviewStatus {
    pub const ALL: [ReviewStatus; 5] = [
        ReviewStatus::None,
        ReviewStatus::Accepted,
        ReviewStatus::Rejected,
        ReviewStatus::Cancelled,
        ReviewStatus::Completed,
    ];

    /// Identifiant stable (clés `comments.status.*`, CLI).
    pub fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }

    /// Valeur de `/State` (modèle `Review`, ISO 32000-1 tableau 172).
    pub fn pdf_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Accepted => "Accepted",
            Self::Rejected => "Rejected",
            Self::Cancelled => "Cancelled",
            Self::Completed => "Completed",
        }
    }
}

/// Une annotation telle que la lit le moteur (espace d'affichage), avec ce qu'on peut en faire.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotDetail {
    /// Position dans `/Annots` = indice PDFium.
    pub index: usize,
    /// Sous-type PDF (`Square`, `Ink`, `FreeText`, `Link`, `Text`, `Highlight`, `Popup`…) ;
    /// `Unknown` pour une entrée illisible.
    pub subtype: String,
    pub rect: PointRect,
    pub contents: Option<String>,
    pub author: Option<String>,
    pub modified: Option<String>,
    pub name: Option<String>,
    /// Couleur du trait (texte pour un texte libre).
    pub rgb: Option<(u8, u8, u8)>,
    /// Fond (`/IC` d'une forme, `/C` d'un texte libre).
    pub fill_rgb: Option<(u8, u8, u8)>,
    pub width_pt: Option<f32>,
    pub opacity: f32,
    /// Border dash/gap lengths in points; empty for solid or unsupported borders.
    pub dash_pattern: Vec<f32>,
    /// Taille de police d'un texte libre.
    pub font_size: Option<f32>,
    /// Annotation à laquelle celle-ci répond (`/IRT`), en indice.
    pub parent: Option<usize>,
    /// Renseigné seulement pour une annotation d'état de relecture (`/StateModel`).
    pub status: Option<ReviewStatus>,
    pub link: Option<LinkTarget>,
    pub arrow: bool,
    pub hidden: bool,
    pub locked: bool,
    /// Écrite par SuniPDF (`/NM` préfixé) : son apparence peut être régénérée.
    pub ours: bool,
    pub can_move: bool,
    pub can_resize: bool,
    pub can_restyle: bool,
    pub can_edit_text: bool,
    pub can_edit_link: bool,
    /// Nombre de descendants `/IRT` (réponses et états, à tous les niveaux).
    pub replies: usize,
}

impl AnnotDetail {
    /// Réponse ou état : ne s'affiche pas sur la page.
    pub fn is_reply_or_state(&self) -> bool {
        self.parent.is_some()
    }
}

/// Statut courant d'un fil : `State` de la dernière annotation d'état parmi les descendants.
pub fn thread_status(details: &[AnnotDetail], root: usize) -> ReviewStatus {
    let mut family: HashSet<usize> = HashSet::from([root]);
    let mut status = ReviewStatus::None;
    // Ordre du tableau = ordre chronologique d'écriture ; un parent précède ses enfants.
    for d in details {
        if d.parent.is_some_and(|p| family.contains(&p)) {
            family.insert(d.index);
            if let Some(s) = d.status {
                status = s;
            }
        }
    }
    status
}

// ----- Utilitaires PDF ----------------------------------------------------------------------

/// Date PDF `D:YYYYMMDDHHmmSSZ` d'un instant UNIX (algorithme de H. Hinnant : pas de `chrono`).
pub(crate) fn pdf_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "D:{y:04}{m:02}{d:02}{:02}{:02}{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

fn pdf_date_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    pdf_date(secs)
}

fn new_name() -> Result<String, EngineError> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|e| EngineError::Other(format!("random: {e}")))?;
    Ok(format!(
        "{NAME_PREFIX}{}",
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    ))
}

fn name(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

pub(crate) fn nums(v: &[f32]) -> Object {
    Object::Array(v.iter().map(|x| Object::Real(*x)).collect())
}

pub(crate) fn rect_obj(r: PointRect) -> Object {
    nums(&[r.left, r.bottom, r.right, r.top])
}

pub(crate) fn rgb_obj(c: (u8, u8, u8)) -> Object {
    nums(&[c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0])
}

fn deref<'a>(d: &'a lopdf::Document, o: &'a Object) -> Option<&'a Object> {
    d.dereference(o).ok().map(|(_, o)| o)
}

fn get<'a>(d: &'a lopdf::Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    dict.get(key).ok().and_then(|o| deref(d, o))
}

fn floats(d: &lopdf::Document, o: &Object) -> Option<Vec<f32>> {
    deref(d, o)?
        .as_array()
        .ok()?
        .iter()
        .map(|x| deref(d, x).and_then(|x| x.as_float().ok()))
        .collect()
}

pub(crate) fn text_of(d: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    let o = get(d, dict, key)?;
    let bytes = o.as_str().ok()?;
    // La table PDFDocEncoding de lopdf ignore les sauts de ligne : conservés à part.
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

fn color_of(d: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<(u8, u8, u8)> {
    let v = floats(d, dict.get(key).ok()?)?;
    let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    match v.as_slice() {
        [g] => Some((b(*g), b(*g), b(*g))),
        [r, g, bl] => Some((b(*r), b(*g), b(*bl))),
        [c, m, y, k] => {
            let f = |x: f32| b((1.0 - x) * (1.0 - k));
            Some((f(*c), f(*m), f(*y)))
        }
        _ => None,
    }
}

fn rect_of(d: &lopdf::Document, dict: &Dictionary) -> Option<PointRect> {
    let v = floats(d, dict.get(b"Rect").ok()?)?;
    let [a, b, c, e] = v.as_slice() else {
        return None;
    };
    Some(PointRect {
        left: a.min(*c),
        bottom: b.min(*e),
        right: a.max(*c),
        top: b.max(*e),
    })
}

fn border_width(d: &lopdf::Document, dict: &Dictionary) -> Option<f32> {
    if let Some(w) = get(d, dict, b"BS")
        .and_then(|o| o.as_dict().ok())
        .and_then(|bs| bs.get(b"W").ok())
        .and_then(|w| w.as_float().ok())
    {
        return Some(w);
    }
    floats(d, dict.get(b"Border").ok()?)?.get(2).copied()
}

pub(crate) fn validate_dash_pattern(pattern: &[f32]) -> Result<(), EngineError> {
    if pattern.len() > 16
        || pattern
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1000.0).contains(v) || (*v > 0.0 && *v < 0.1))
        || (!pattern.is_empty() && !pattern.iter().any(|v| *v > 0.0))
    {
        return Err(EngineError::Other("invalid border dash pattern".into()));
    }
    Ok(())
}

fn read_dash_array(pdf: &lopdf::Document, value: &Object) -> Result<Vec<f32>, EngineError> {
    let bad = || EngineError::Other("unsupported or malformed annotation border".into());
    let values = deref(pdf, value)
        .and_then(|o| o.as_array().ok())
        .ok_or_else(bad)?;
    if values.len() > 16 {
        return Err(bad());
    }
    let pattern = values
        .iter()
        .map(|v| {
            deref(pdf, v)
                .and_then(|v| v.as_float().ok())
                .ok_or_else(bad)
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_dash_pattern(&pattern)?;
    Ok(pattern)
}

fn border_dash_pattern(pdf: &lopdf::Document, dict: &Dictionary) -> Result<Vec<f32>, EngineError> {
    let bad = || EngineError::Other("unsupported or malformed annotation border".into());
    let Ok(raw_bs) = dict.get(b"BS") else {
        if let Ok(border) = dict.get(b"Border") {
            let values = deref(pdf, border)
                .and_then(|o| o.as_array().ok())
                .ok_or_else(bad)?;
            if !matches!(values.len(), 3 | 4) {
                return Err(bad());
            }
            if values.len() == 4 {
                let pattern = deref(pdf, &values[3])
                    .and_then(|o| o.as_array().ok())
                    .ok_or_else(bad)?;
                if !pattern.is_empty() {
                    return read_dash_array(pdf, &values[3]);
                }
            }
        }
        return Ok(Vec::new());
    };
    let bs = deref(pdf, raw_bs)
        .and_then(|o| o.as_dict().ok())
        .ok_or_else(bad)?;
    let style = match bs.get(b"S") {
        Ok(value) => deref(pdf, value)
            .and_then(|o| o.as_name().ok())
            .ok_or_else(bad)?,
        Err(_) => b"S",
    };
    match style {
        b"S" => Ok(Vec::new()),
        b"D" => {
            let Ok(value) = bs.get(b"D") else {
                return Ok(vec![3.0]);
            };
            read_dash_array(pdf, value)
        }
        _ => Err(bad()),
    }
}

fn flags(dict: &Dictionary) -> i64 {
    dict.get(b"F")
        .ok()
        .and_then(|f| f.as_i64().ok())
        .unwrap_or(0)
}

/// Taille et couleur lues dans une chaîne `/DA` (`/Helv 12 Tf 0 0 1 rg`).
fn parse_da(da: &str) -> (Option<f32>, Option<(u8, u8, u8)>) {
    let t: Vec<&str> = da.split_whitespace().collect();
    let size = t
        .iter()
        .position(|x| *x == "Tf")
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| t[i].parse().ok());
    let color = t.iter().position(|x| *x == "rg").and_then(|i| {
        let v: Vec<f32> = t
            .get(i.checked_sub(3)?..i)?
            .iter()
            .filter_map(|x| x.parse().ok())
            .collect();
        let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        (v.len() == 3).then(|| (b(v[0]), b(v[1]), b(v[2])))
    });
    (size, color)
}

// ----- Droits -------------------------------------------------------------------------------

struct Caps {
    ours: bool,
    locked: bool,
    hidden: bool,
    can_move: bool,
    can_resize: bool,
    can_restyle: bool,
    can_edit_text: bool,
}

fn caps(subtype: &str, f: i64, nm: Option<&str>) -> Caps {
    let ours = nm.is_some_and(|n| n.starts_with(NAME_PREFIX));
    let locked = f & 128 != 0;
    let shape = matches!(subtype, "Square" | "Circle" | "Line" | "Ink" | "FreeText");
    Caps {
        ours,
        locked,
        hidden: f & 2 != 0,
        can_move: !locked
            && (shape
                || matches!(
                    subtype,
                    "Text" | "Link" | "Stamp" | "Polygon" | "PolyLine" | "FileAttachment"
                )),
        can_resize: !locked && ours && (shape || subtype == "Link"),
        can_restyle: !locked
            && ((ours && shape)
                || matches!(
                    subtype,
                    "Highlight" | "Underline" | "StrikeOut" | "Squiggly" | "Text"
                )),
        can_edit_text: !locked
            && !matches!(subtype, "Popup" | "Widget" | "Link" | "Unknown")
            && (subtype != "FreeText" || ours),
    }
}

// ----- Accès aux annotations d'une page --------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) enum Slot {
    Indirect(ObjectId),
    Direct(usize),
}

fn annots_entries(pdf: &lopdf::Document, page: ObjectId) -> Vec<Object> {
    pdf.get_dictionary(page)
        .ok()
        .and_then(|p| get(pdf, p, b"Annots"))
        .and_then(|a| a.as_array().ok())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn annots_mut(
    pdf: &mut lopdf::Document,
    page: ObjectId,
) -> Result<&mut Vec<Object>, EngineError> {
    let indirect = pdf
        .get_dictionary(page)
        .map_err(lopdf_err)?
        .get(b"Annots")
        .ok()
        .and_then(|o| o.as_reference().ok());
    match indirect {
        Some(id) => pdf
            .get_object_mut(id)
            .and_then(Object::as_array_mut)
            .map_err(lopdf_err),
        None => {
            let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
            if dict.get(b"Annots").is_err() {
                dict.set("Annots", Object::Array(vec![]));
            }
            dict.get_mut(b"Annots")
                .and_then(Object::as_array_mut)
                .map_err(lopdf_err)
        }
    }
}

fn not_found(index: usize) -> EngineError {
    EngineError::Other(format!("annotation {index} not found"))
}

pub(crate) fn load_annot(
    pdf: &lopdf::Document,
    page: ObjectId,
    index: usize,
) -> Result<(Slot, Dictionary), EngineError> {
    let entries = annots_entries(pdf, page);
    match entries.get(index) {
        Some(Object::Reference(id)) => pdf
            .get_dictionary(*id)
            .map(|d| (Slot::Indirect(*id), d.clone()))
            .map_err(|_| not_found(index)),
        Some(Object::Dictionary(d)) => Ok((Slot::Direct(index), d.clone())),
        _ => Err(not_found(index)),
    }
}

pub(crate) fn store_annot(
    pdf: &mut lopdf::Document,
    page: ObjectId,
    slot: Slot,
    dict: Dictionary,
) -> Result<(), EngineError> {
    match slot {
        Slot::Indirect(id) => {
            *pdf.get_object_mut(id).map_err(lopdf_err)? = Object::Dictionary(dict);
        }
        Slot::Direct(i) => annots_mut(pdf, page)?[i] = Object::Dictionary(dict),
    }
    Ok(())
}

fn subtype_of(d: &Dictionary) -> String {
    d.get(b"Subtype")
        .ok()
        .and_then(|s| s.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_else(|| "Unknown".into())
}

fn link_target_editable(pdf: &lopdf::Document, dict: &Dictionary) -> bool {
    let flags = match dict.get(b"F") {
        Err(_) => Some(0),
        Ok(_) => get(pdf, dict, b"F").and_then(|f| f.as_i64().ok()),
    };
    flags.is_some_and(|f| f >= 0 && f & (64 | 128 | 512) == 0)
}

fn write_link_target(
    pdf: &lopdf::Document,
    dict: &mut Dictionary,
    target: &LinkTarget,
    target_geo: Option<(usize, PageGeometry)>,
) -> Result<(), EngineError> {
    let (key, value) = match target {
        LinkTarget::Page(_) | LinkTarget::Named(_) | LinkTarget::Destination(_) => (
            "Dest",
            crate::link_destination::authored(pdf, target, dict, &|page| {
                target_geo
                    .filter(|(index, _)| *index == page)
                    .map(|(_, geometry)| geometry)
                    .ok_or(EngineError::PageOutOfRange(page))
            })?,
        ),
        LinkTarget::Uri(u) if crate::links::is_safe_external_uri(u) => {
            let mut action = Dictionary::new();
            action.set("S", name("URI"));
            action.set("URI", Object::string_literal(u.trim().as_bytes().to_vec()));
            ("A", Object::Dictionary(action))
        }
        LinkTarget::Uri(_) => return Err(EngineError::Other("unsafe uri".into())),
        LinkTarget::Unsupported => {
            return Err(EngineError::Other("unsupported link target".into()))
        }
    };
    // Retargeting replaces every action, including additional mouse-triggered actions.
    for key in [b"A".as_slice(), b"Dest", b"AA"] {
        dict.remove(key);
    }
    dict.set(key, value);
    Ok(())
}

// ----- Modèle des annotations de SuniPDF ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Model {
    Square(PointRect),
    Circle(PointRect),
    Ink(Vec<Vec<(f32, f32)>>),
    Line {
        p1: (f32, f32),
        p2: (f32, f32),
        arrow: bool,
    },
    FreeText {
        rect: PointRect,
        text: String,
        fts: FreeTextStyle,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Parsed {
    model: Model,
    style: AnnotStyle,
}

fn bbox(points: impl Iterator<Item = (f32, f32)>) -> Option<PointRect> {
    points.fold(None, |acc: Option<PointRect>, p| {
        Some(match acc {
            None => PointRect {
                left: p.0,
                bottom: p.1,
                right: p.0,
                top: p.1,
            },
            Some(r) => PointRect {
                left: r.left.min(p.0),
                bottom: r.bottom.min(p.1),
                right: r.right.max(p.0),
                top: r.top.max(p.1),
            },
        })
    })
}

fn expand(r: PointRect, m: f32) -> PointRect {
    PointRect {
        left: r.left - m,
        bottom: r.bottom - m,
        right: r.right + m,
        top: r.top + m,
    }
}

impl Parsed {
    /// Marge entre la boîte des points et le rectangle de l'annotation (épaisseur du trait, pointe).
    fn margin(&self) -> f32 {
        let lw = self.style.width_pt;
        match &self.model {
            Model::Ink(_) => lw / 2.0 + 0.5,
            Model::Line { arrow, .. } => {
                lw / 2.0
                    + 1.0
                    + if *arrow {
                        appearance::arrow_head(lw)
                    } else {
                        0.0
                    }
            }
            _ => 0.0,
        }
    }

    fn points(&self) -> Vec<(f32, f32)> {
        match &self.model {
            Model::Ink(s) => s.iter().flatten().copied().collect(),
            Model::Line { p1, p2, .. } => vec![*p1, *p2],
            _ => Vec::new(),
        }
    }

    /// Rectangle d'affichage de l'annotation.
    fn rect(&self) -> PointRect {
        match &self.model {
            Model::Square(r) | Model::Circle(r) => *r,
            Model::FreeText { rect, .. } => *rect,
            _ => expand(
                bbox(self.points().into_iter()).unwrap_or(PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                    top: 0.0,
                }),
                self.margin(),
            ),
        }
    }

    /// Ramène l'annotation dans `to` (rectangle d'affichage) : rectangle étiré, points d'encre ou
    /// de ligne mis à l'échelle de leur boîte.
    fn fit_to(&mut self, to: PointRect) {
        let m = self.margin();
        let from = bbox(self.points().into_iter());
        match &mut self.model {
            Model::Square(r) | Model::Circle(r) => *r = to,
            Model::FreeText { rect, .. } => *rect = to,
            Model::Ink(_) | Model::Line { .. } => {
                let Some(from) = from else {
                    return;
                };
                let inner = expand(to, -m);
                let axis = |lo: f32, hi: f32, nlo: f32, nhi: f32| {
                    // Étendue nulle (trait horizontal) : seulement recentré.
                    let (nlo, nhi) = if nhi - nlo < 0.5 {
                        let c = (nlo + nhi) / 2.0;
                        (c - 0.25, c + 0.25)
                    } else {
                        (nlo, nhi)
                    };
                    move |v: f32| {
                        if hi - lo < 1e-3 {
                            (nlo + nhi) / 2.0
                        } else {
                            nlo + (v - lo) * (nhi - nlo) / (hi - lo)
                        }
                    }
                };
                let fx = axis(from.left, from.right, inner.left, inner.right);
                let fy = axis(from.bottom, from.top, inner.bottom, inner.top);
                let map = |p: &mut (f32, f32)| *p = (fx(p.0), fy(p.1));
                match &mut self.model {
                    Model::Ink(strokes) => strokes.iter_mut().flatten().for_each(map),
                    Model::Line { p1, p2, .. } => {
                        map(p1);
                        map(p2);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_ours(
    pdf: &lopdf::Document,
    dict: &Dictionary,
    geo: &PageGeometry,
) -> Result<Parsed, EngineError> {
    let bad = || EngineError::Other("annotation cannot be edited".into());
    if dict.has(b"SuniPDFRich") {
        return Err(EngineError::Other(
            "use the rich annotation editor to preserve this appearance and its embedded fonts"
                .into(),
        ));
    }
    let ours = text_of(pdf, dict, b"NM").is_some_and(|n| n.starts_with(NAME_PREFIX));
    if !ours {
        return Err(bad());
    }
    let rect = rect_of(pdf, dict)
        .map(|r| geo.rect_to_display(r))
        .ok_or_else(bad)?;
    let opacity = get(pdf, dict, b"CA")
        .and_then(|o| o.as_float().ok())
        .unwrap_or(1.0);
    let c = color_of(pdf, dict, b"C");
    let mut style = AnnotStyle {
        rgb: c.unwrap_or(AnnotStyle::default().rgb),
        fill_rgb: color_of(pdf, dict, b"IC"),
        width_pt: border_width(pdf, dict).unwrap_or(1.0),
        opacity,
        dash_pattern: border_dash_pattern(pdf, dict)?,
    };
    let to_display = |x: f32, y: f32| geo.to_display(x, y);
    let model = match subtype_of(dict).as_str() {
        "Square" => Model::Square(rect),
        "Circle" => Model::Circle(rect),
        "Ink" => {
            let list = get(pdf, dict, b"InkList")
                .and_then(|o| o.as_array().ok())
                .ok_or_else(bad)?;
            let strokes = list
                .iter()
                .filter_map(|s| floats(pdf, s))
                .map(|v| {
                    v.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|p| to_display(p[0], p[1]))
                        .collect()
                })
                .collect();
            Model::Ink(strokes)
        }
        "Line" => {
            let l = floats(pdf, dict.get(b"L").map_err(|_| bad())?).ok_or_else(bad)?;
            let [x1, y1, x2, y2] = l.as_slice() else {
                return Err(bad());
            };
            let arrow = get(pdf, dict, b"LE")
                .and_then(|o| o.as_array().ok())
                .is_some_and(|a| {
                    a.iter().any(|e| {
                        deref(pdf, e)
                            .and_then(|e| e.as_name().ok())
                            .is_some_and(|n| n == b"OpenArrow")
                    })
                });
            Model::Line {
                p1: to_display(*x1, *y1),
                p2: to_display(*x2, *y2),
                arrow,
            }
        }
        "FreeText" => {
            let (size, text_rgb) = text_of(pdf, dict, b"DA")
                .map(|da| parse_da(&da))
                .unwrap_or_default();
            let fts = FreeTextStyle {
                size_pt: size.unwrap_or(12.0),
                rgb: text_rgb.unwrap_or((0, 0, 0)),
                fill_rgb: c,
                border_pt: border_width(pdf, dict).unwrap_or(0.0),
            };
            // Pour le texte libre, `/C` est le fond : le style générique porte texte et bordure.
            style.rgb = fts.rgb;
            style.fill_rgb = fts.fill_rgb;
            style.width_pt = fts.border_pt;
            Model::FreeText {
                rect,
                text: text_of(pdf, dict, b"Contents").unwrap_or_default(),
                fts,
            }
        }
        _ => return Err(bad()),
    };
    Ok(Parsed { model, style })
}

pub(crate) fn ap_stream(ap: &Ap, geo: &PageGeometry) -> Stream {
    let (a, b) = geo.vec_to_user(1.0, 0.0);
    let (c, d) = geo.vec_to_user(0.0, 1.0);
    let mut gs = Dictionary::new();
    gs.set("Type", name("ExtGState"));
    gs.set("CA", Object::Real(ap.opacity));
    gs.set("ca", Object::Real(ap.opacity));
    let mut gs_map = Dictionary::new();
    gs_map.set("GS", Object::Dictionary(gs));
    let mut res = Dictionary::new();
    res.set("ExtGState", Object::Dictionary(gs_map));
    if ap.uses_font {
        let mut f = Dictionary::new();
        f.set("Type", name("Font"));
        f.set("Subtype", name("Type1"));
        f.set("BaseFont", name("Helvetica"));
        f.set("Encoding", name("WinAnsiEncoding"));
        let mut fonts = Dictionary::new();
        fonts.set("Helv", Object::Dictionary(f));
        res.set("Font", Object::Dictionary(fonts));
    }
    let mut dict = Dictionary::new();
    dict.set("Type", name("XObject"));
    dict.set("Subtype", name("Form"));
    dict.set("BBox", nums(&[0.0, 0.0, ap.w, ap.h]));
    dict.set("Matrix", nums(&[a, b, c, d, 0.0, 0.0]));
    dict.set("Resources", Object::Dictionary(res));
    Stream::new(dict, ap.content.clone())
}

/// Flux d'apparence vide (réponses et états : rien à dessiner sur la page).
fn empty_ap() -> Stream {
    let mut dict = Dictionary::new();
    dict.set("Type", name("XObject"));
    dict.set("Subtype", name("Form"));
    dict.set("BBox", nums(&[0.0, 0.0, 1.0, 1.0]));
    dict.set("Resources", Object::Dictionary(Dictionary::new()));
    Stream::new(dict, Vec::new())
}

/// Pose `stream` comme `/AP /N` : l'objet existant est remplacé sur place (pas d'orphelin).
pub(crate) fn set_ap(pdf: &mut lopdf::Document, dict: &mut Dictionary, stream: Stream) {
    let existing = get(pdf, dict, b"AP")
        .and_then(|ap| ap.as_dict().ok())
        .and_then(|ap| ap.get(b"N").ok())
        .and_then(|n| n.as_reference().ok());
    match existing {
        Some(id) => {
            pdf.objects.insert(id, Object::Stream(stream));
        }
        None => {
            let id = pdf.add_object(stream);
            let mut ap = Dictionary::new();
            ap.set("N", Object::Reference(id));
            dict.set("AP", Object::Dictionary(ap));
        }
    }
    dict.remove(b"PDFIUM_HasGeneratedAP");
}

pub(crate) fn border_style(width: f32, dash_pattern: &[f32]) -> Object {
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(width));
    bs.set("S", name(if dash_pattern.is_empty() { "S" } else { "D" }));
    if !dash_pattern.is_empty() {
        bs.set("D", nums(dash_pattern));
    }
    Object::Dictionary(bs)
}

/// Écrit géométrie, style et apparence d'un modèle dans `dict` ; renvoie le flux `/AP`.
fn write_model(
    dict: &mut Dictionary,
    p: &mut Parsed,
    geo: &PageGeometry,
) -> Result<Stream, EngineError> {
    let style = p.style.clone();
    validate_dash_pattern(&style.dash_pattern)?;
    if !style.dash_pattern.is_empty()
        && !matches!(
            p.model,
            Model::Square(_) | Model::Circle(_) | Model::Line { .. }
        )
    {
        return Err(EngineError::Other(
            "dashed borders require a shape annotation".into(),
        ));
    }
    if !(0.0..=MAX_WIDTH_PT).contains(&style.width_pt) || !style.width_pt.is_finite() {
        return Err(EngineError::Other("invalid line width".into()));
    }
    // Texte libre : renvoi à la ligne, la boîte grandit vers le bas si le texte est plus haut.
    let mut text_lines = Vec::new();
    if let Model::FreeText { rect, text, fts } = &mut p.model {
        if text.trim().is_empty() {
            return Err(EngineError::Other("empty text".into()));
        }
        if !(4.0..=200.0).contains(&fts.size_pt) || !(0.0..=MAX_WIDTH_PT).contains(&fts.border_pt) {
            return Err(EngineError::Other("invalid text style".into()));
        }
        if rect.right - rect.left < 2.0 * fts.size_pt || rect.top - rect.bottom < 2.0 {
            return Err(EngineError::Other("invalid rectangle".into()));
        }
        let pad = appearance::free_text_pad(fts.border_pt);
        text_lines = appearance::wrap_text(text, rect.right - rect.left - 2.0 * pad, fts.size_pt);
        let needed =
            appearance::free_text_needed_height(text_lines.len(), fts.size_pt, fts.border_pt);
        if rect.top - rect.bottom < needed {
            rect.bottom = rect.top - needed;
        }
    }
    let rect = p.rect();
    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
    if ![rect.left, rect.bottom, rect.right, rect.top, w, h]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(EngineError::Other("invalid rectangle".into()));
    }
    if !style.dash_pattern.is_empty() {
        let cycle: f64 = style.dash_pattern.iter().map(|v| f64::from(*v)).sum();
        let segments = 2.0 * (f64::from(w).abs() + f64::from(h).abs()) / cycle
            * style.dash_pattern.len() as f64;
        if segments > 100_000.0 {
            return Err(EngineError::Other(
                "border dash pattern is too dense for this geometry".into(),
            ));
        }
    }
    let to_user = |q: (f32, f32)| geo.to_user(q.0, q.1);
    let local = |q: (f32, f32)| (q.0 - rect.left, q.1 - rect.bottom);
    dict.set("Rect", rect_obj(geo.rect_to_user(rect)));
    let ap = match &p.model {
        Model::Square(_) | Model::Circle(_) => {
            if w < 2.0 || h < 2.0 {
                return Err(EngineError::Other("invalid rectangle".into()));
            }
            if matches!(p.model, Model::Square(_)) {
                appearance::square_ap(w, h, &style)
            } else {
                appearance::circle_ap(w, h, &style)
            }
        }
        Model::Ink(strokes) => {
            let strokes_local: Vec<Vec<(f32, f32)>> = strokes
                .iter()
                .map(|s| s.iter().map(|q| local(*q)).collect())
                .collect();
            let list: Vec<Object> = strokes
                .iter()
                .map(|s| {
                    Object::Array(
                        s.iter()
                            .flat_map(|q| {
                                let (x, y) = to_user(*q);
                                [Object::Real(x), Object::Real(y)]
                            })
                            .collect(),
                    )
                })
                .collect();
            dict.set("InkList", Object::Array(list));
            appearance::ink_ap(w, h, &strokes_local, &style)
        }
        Model::Line { p1, p2, arrow } => {
            if ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt() < 1.0 {
                return Err(EngineError::Other("line too short".into()));
            }
            let (u1, u2) = (to_user(*p1), to_user(*p2));
            dict.set("L", nums(&[u1.0, u1.1, u2.0, u2.1]));
            dict.set(
                "LE",
                Object::Array(vec![
                    name("None"),
                    name(if *arrow { "OpenArrow" } else { "None" }),
                ]),
            );
            appearance::line_ap(w, h, local(*p1), local(*p2), *arrow, &style)
        }
        Model::FreeText { text, fts, .. } => {
            dict.set("Contents", lopdf::text_string(text));
            let c = fts.rgb;
            dict.set(
                "DA",
                Object::string_literal(format!(
                    "/Helv {} Tf {} {} {} rg",
                    appearance::fmt_num(fts.size_pt),
                    appearance::fmt_num(c.0 as f32 / 255.0),
                    appearance::fmt_num(c.1 as f32 / 255.0),
                    appearance::fmt_num(c.2 as f32 / 255.0),
                )),
            );
            dict.set("Q", Object::Integer(0));
            appearance::free_text_ap(w, h, &text_lines, fts)
        }
    };
    // Clés de style : le texte libre range son fond dans `/C`, les formes leur trait.
    match &p.model {
        Model::FreeText { fts, .. } => {
            match fts.fill_rgb {
                Some(f) => dict.set("C", rgb_obj(f)),
                None => {
                    dict.remove(b"C");
                }
            }
            dict.set("BS", border_style(fts.border_pt, &[]));
        }
        _ => {
            dict.set("C", rgb_obj(style.rgb));
            match style.fill_rgb {
                Some(f) if matches!(p.model, Model::Square(_) | Model::Circle(_)) => {
                    dict.set("IC", rgb_obj(f))
                }
                _ => {
                    dict.remove(b"IC");
                }
            }
            dict.set("BS", border_style(style.width_pt, &style.dash_pattern));
        }
    }
    dict.set("CA", Object::Real(style.opacity.clamp(0.0, 1.0)));
    Ok(ap_stream(&ap, geo))
}

/// Dictionnaire de base : sous-type, indicateurs, nom unique, dates, auteur.
pub(crate) fn base_annotation(
    subtype: &str,
    author: Option<&str>,
) -> Result<Dictionary, EngineError> {
    let now = pdf_date_now();
    let mut d = Dictionary::new();
    d.set("Type", name("Annot"));
    d.set("Subtype", name(subtype));
    d.set("F", Object::Integer(4));
    d.set("NM", lopdf::text_string(&new_name()?));
    d.set("M", Object::string_literal(now.clone()));
    d.set("CreationDate", Object::string_literal(now));
    if let Some(a) = author.map(str::trim).filter(|a| !a.is_empty()) {
        d.set("T", lopdf::text_string(a));
    }
    Ok(d)
}

/// Ajoute l'annotation à `/Annots` et renvoie son indice.
pub(crate) fn push_annotation(
    pdf: &mut lopdf::Document,
    page: ObjectId,
    mut dict: Dictionary,
    ap: Option<Stream>,
) -> Result<usize, EngineError> {
    if let Some(stream) = ap {
        set_ap(pdf, &mut dict, stream);
    }
    dict.set("P", Object::Reference(page));
    let id = pdf.add_object(dict);
    let annots = annots_mut(pdf, page)?;
    annots.push(Object::Reference(id));
    Ok(annots.len() - 1)
}

pub(crate) fn touch(dict: &mut Dictionary) {
    dict.set("M", Object::string_literal(pdf_date_now()));
}

fn scaled_annotation_pairs(
    pdf: &lopdf::Document,
    value: &Object,
    matrix: [f32; 6],
    valid_length: impl FnOnce(usize) -> bool,
) -> Result<Object, EngineError> {
    let values = deref(pdf, value)
        .ok_or_else(|| EngineError::Other("invalid annotation geometry reference".into()))?
        .as_array()
        .map_err(lopdf_err)?;
    if values.len() > 200_000 || !values.len().is_multiple_of(2) || !valid_length(values.len()) {
        return Err(EngineError::Other(
            "invalid annotation geometry length".into(),
        ));
    }
    let mut output = Vec::with_capacity(values.len());
    for pair in values.as_chunks::<2>().0 {
        let number = |value| {
            deref(pdf, value)
                .and_then(|value| value.as_float().ok())
                .filter(|value| value.is_finite())
                .ok_or_else(|| EngineError::Other("invalid annotation coordinate".into()))
        };
        let (x, y) = (number(&pair[0])?, number(&pair[1])?);
        let (x, y) = (
            matrix[0] * x + matrix[2] * y + matrix[4],
            matrix[1] * x + matrix[3] * y + matrix[5],
        );
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Other("annotation coordinate overflow".into()));
        }
        output.extend([Object::Real(x), Object::Real(y)]);
    }
    Ok(Object::Array(output))
}

impl Document<'_> {
    pub(crate) fn scale_annotation_geometry(
        &mut self,
        page_index: usize,
        matrix: [f32; 6],
    ) -> Result<(), EngineError> {
        self.edit_pdf(page_index, |pdf, page, _| {
            let entries = match pdf.get_dictionary(page).map_err(lopdf_err)?.get(b"Annots") {
                Ok(value) => deref(pdf, value)
                    .ok_or_else(|| EngineError::Other("invalid annotation list".into()))?
                    .as_array()
                    .map_err(lopdf_err)?
                    .clone(),
                Err(_) => return Ok(()),
            };
            if entries.len() > 100_000 {
                return Err(EngineError::Other("too many annotations".into()));
            }
            let page_annotation_ids = entries
                .iter()
                .filter_map(|entry| pdf.dereference(entry).ok().and_then(|(id, _)| id))
                .collect::<HashSet<_>>();
            let mut other_page_ids = HashSet::new();
            for id in pdf.get_pages().values().copied().filter(|id| *id != page) {
                for entry in annots_entries(pdf, id) {
                    if let Ok((Some(id), _)) = pdf.dereference(&entry) {
                        other_page_ids.insert(id);
                    }
                }
            }
            let mut seen = HashSet::new();
            let mut output = Vec::with_capacity(entries.len());
            for entry in entries {
                let (id, value) = pdf.dereference(&entry).map_err(lopdf_err)?;
                if id.is_some_and(|id| other_page_ids.contains(&id)) {
                    return Err(EngineError::Other(
                        "annotation is shared by multiple pages".into(),
                    ));
                }
                if id.is_some_and(|id| !seen.insert(id)) {
                    output.push(entry);
                    continue;
                }
                let mut dict = value.as_dict().map_err(lopdf_err)?.clone();
                if let Ok(popup) = dict.get(b"Popup") {
                    let (popup_id, popup) = pdf.dereference(popup).map_err(lopdf_err)?;
                    if !popup_id.is_some_and(|id| page_annotation_ids.contains(&id))
                        || popup.as_dict().is_err()
                    {
                        return Err(EngineError::Other(
                            "linked popup must be listed on the same page".into(),
                        ));
                    }
                }
                let flags = match dict.get(b"F") {
                    Ok(value) => deref(pdf, value)
                        .ok_or_else(|| EngineError::Other("invalid annotation flags".into()))?
                        .as_i64()
                        .map_err(lopdf_err)?,
                    Err(_) => 0,
                };
                if flags < 0 || flags & (64 | 128 | 512) != 0 {
                    return Err(EngineError::SecurityRestricted);
                }
                let rect = dict.get(b"Rect").map_err(lopdf_err)?;
                dict.set(
                    "Rect",
                    scaled_annotation_pairs(pdf, rect, matrix, |n| n == 4)?,
                );
                for key in [b"QuadPoints".as_slice(), b"L", b"CL", b"Vertices"] {
                    if let Ok(value) = dict.get(key) {
                        let transformed =
                            scaled_annotation_pairs(pdf, value, matrix, |n| match key {
                                b"QuadPoints" => n >= 8 && n.is_multiple_of(8),
                                b"L" => n == 4,
                                b"CL" => n == 4 || n == 6,
                                _ => n >= 4,
                            })?;
                        dict.set(key, transformed);
                    }
                }
                if let Ok(value) = dict.get(b"InkList") {
                    let strokes = deref(pdf, value)
                        .ok_or_else(|| EngineError::Other("invalid ink list reference".into()))?
                        .as_array()
                        .map_err(lopdf_err)?;
                    if strokes.len() > MAX_STROKES {
                        return Err(EngineError::Other("too many ink strokes".into()));
                    }
                    let transformed = strokes
                        .iter()
                        .map(|stroke| scaled_annotation_pairs(pdf, stroke, matrix, |n| n >= 2))
                        .collect::<Result<Vec<_>, _>>()?;
                    dict.set("InkList", Object::Array(transformed));
                }
                if let Some(id) = id {
                    pdf.objects.insert(id, Object::Dictionary(dict));
                    output.push(entry);
                } else {
                    output.push(Object::Dictionary(dict));
                }
            }
            // A shared indirect Annots array must not move annotations on another page.
            pdf.get_dictionary_mut(page)
                .map_err(lopdf_err)?
                .set("Annots", Object::Array(output));
            Ok(())
        })
    }

    /// Instantané → lopdf → rechargement PDFium. Un document chiffré garde son chiffrement.
    pub(crate) fn edit_pdf<R>(
        &mut self,
        page_index: usize,
        f: impl FnOnce(&mut lopdf::Document, ObjectId, &PageGeometry) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let bytes = self.snapshot()?;
        let mut pdf = security::load(&bytes, self.password())?;
        let page_id = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        let out = f(&mut pdf, page_id, &geo)?;
        self.restore(security::save(pdf)?)?;
        Ok(out)
    }

    fn add_model(
        &mut self,
        page_index: usize,
        model: Model,
        style: &AnnotStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let subtype = match &model {
            Model::Square(_) => "Square",
            Model::Circle(_) => "Circle",
            Model::Ink(_) => "Ink",
            Model::Line { .. } => "Line",
            Model::FreeText { .. } => "FreeText",
        };
        self.edit_pdf(page_index, |pdf, page, geo| {
            let mut dict = base_annotation(subtype, author)?;
            let mut parsed = Parsed {
                model,
                style: style.clone(),
            };
            let ap = write_model(&mut dict, &mut parsed, geo)?;
            push_annotation(pdf, page, dict, Some(ap))
        })
    }

    /// Annotation d'encre : un ou plusieurs traits en points d'affichage. Renvoie son indice.
    pub fn add_ink_annotation(
        &mut self,
        page_index: usize,
        strokes: &[Vec<(f32, f32)>],
        style: &AnnotStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if strokes.len() > MAX_STROKES {
            return Err(EngineError::Other("too many strokes".into()));
        }
        let simplified: Vec<Vec<(f32, f32)>> = strokes
            .iter()
            .map(|s| {
                let s: Vec<(f32, f32)> = s
                    .iter()
                    .copied()
                    .filter(|p| p.0.is_finite() && p.1.is_finite())
                    .collect();
                let mut out = appearance::simplify_polyline(&s, INK_EPSILON);
                // Un point isolé reste un point ; un tracé démesuré est tronqué.
                out.truncate(MAX_POINTS_PER_STROKE);
                out
            })
            .filter(|s| !s.is_empty())
            .collect();
        let total: usize = simplified.iter().map(Vec::len).sum();
        let dot = simplified.len() == 1 && total == 1;
        if simplified.is_empty() || (total < 2 && !dot) {
            return Err(EngineError::Other("empty stroke".into()));
        }
        self.add_model(page_index, Model::Ink(simplified), style, author)
    }

    /// Erases entire touched strokes, preserving the remaining control points, style and comment thread.
    pub fn erase_ink_annotation(
        &mut self,
        page_index: usize,
        index: usize,
        path: &[(f32, f32)],
        radius: f32,
    ) -> Result<(), EngineError> {
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (w, h) = geo.display_size();
            if path.iter().any(|&(x, y)| {
                !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x > w || y > h
            }) {
                return Err(EngineError::Other(
                    "eraser path must stay inside the displayed page".into(),
                ));
            }
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            check_unlocked(&dict)?;
            let mut parsed = parse_ours(pdf, &dict, geo)?;
            let Model::Ink(strokes) = &parsed.model else {
                return Err(EngineError::Other(
                    "select a SuniPDF ink annotation to erase strokes".into(),
                ));
            };
            let kept = crate::annot_rich::erase_touched_strokes(
                strokes,
                path,
                radius,
                parsed.style.width_pt,
            )?;
            parsed.model = Model::Ink(kept);
            let ap = write_model(&mut dict, &mut parsed, geo)?;
            set_ap(pdf, &mut dict, ap);
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Rectangle, ellipse, ligne ou flèche. Pour une ligne, `rect` donne les extrémités :
    /// `(left, bottom)` puis `(right, top)`, sans normalisation. Renvoie l'indice.
    pub fn add_shape_annotation(
        &mut self,
        page_index: usize,
        shape: AnnotShape,
        rect: PointRect,
        style: &AnnotStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let norm = PointRect {
            left: rect.left.min(rect.right),
            bottom: rect.bottom.min(rect.top),
            right: rect.left.max(rect.right),
            top: rect.bottom.max(rect.top),
        };
        let model = match shape {
            AnnotShape::Square => Model::Square(norm),
            AnnotShape::Circle => Model::Circle(norm),
            AnnotShape::Line | AnnotShape::Arrow => Model::Line {
                p1: (rect.left, rect.bottom),
                p2: (rect.right, rect.top),
                arrow: shape == AnnotShape::Arrow,
            },
        };
        self.add_model(page_index, model, style, author)
    }

    /// Zone de texte libre (Helvetica WinAnsi). La boîte grandit vers le bas si le texte est plus
    /// haut ; `Contents` garde le texte exact. Renvoie l'indice.
    pub fn add_free_text_annotation(
        &mut self,
        page_index: usize,
        rect: PointRect,
        text: &str,
        style: &FreeTextStyle,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        let rect = PointRect {
            left: rect.left.min(rect.right),
            bottom: rect.bottom.min(rect.top),
            right: rect.left.max(rect.right),
            top: rect.bottom.max(rect.top),
        };
        let generic = AnnotStyle {
            rgb: style.rgb,
            fill_rgb: style.fill_rgb,
            width_pt: style.border_pt,
            opacity: 1.0,
            dash_pattern: Vec::new(),
        };
        self.add_model(
            page_index,
            Model::FreeText {
                rect,
                text: text.to_string(),
                fts: style.clone(),
            },
            &generic,
            author,
        )
    }

    /// Zone cliquable : saut de page ou adresse (`http`, `https`, `mailto` seulement). Renvoie l'indice.
    pub fn add_link_annotation(
        &mut self,
        page_index: usize,
        rect: PointRect,
        target: &LinkTarget,
    ) -> Result<usize, EngineError> {
        let target_geo = self.link_target_geometry(target)?;
        let rect = PointRect {
            left: rect.left.min(rect.right),
            bottom: rect.bottom.min(rect.top),
            right: rect.left.max(rect.right),
            top: rect.bottom.max(rect.top),
        };
        if rect.right - rect.left < 2.0 || rect.top - rect.bottom < 2.0 {
            return Err(EngineError::Other("invalid rectangle".into()));
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            let mut dict = base_annotation("Link", None)?;
            dict.set("Rect", rect_obj(geo.rect_to_user(rect)));
            dict.set("Border", nums(&[0.0, 0.0, 0.0]));
            write_link_target(pdf, &mut dict, target, target_geo)?;
            push_annotation(pdf, page, dict, None)
        })
    }

    /// Replace a link's action with a safe URI or a page-fit destination, preserving its zone.
    pub fn set_link_target(
        &mut self,
        page_index: usize,
        index: usize,
        target: &LinkTarget,
    ) -> Result<(), EngineError> {
        let target_geo = self.link_target_geometry(target)?;
        self.edit_pdf(page_index, |pdf, page, _| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            if subtype_of(&dict) != "Link" {
                return Err(EngineError::Other("annotation is not a link".into()));
            }
            if !link_target_editable(pdf, &dict) {
                return Err(EngineError::SecurityRestricted);
            }
            write_link_target(pdf, &mut dict, target, target_geo)?;
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Toutes les annotations d'une page, alignées sur `/Annots` (indice = indice PDFium), avec
    /// leurs droits. Lecture seule, mais lit tout le document : à demander à la demande.
    pub fn annotation_details(&self, page_index: usize) -> Result<Vec<AnnotDetail>, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let bytes = self.snapshot()?;
        let pdf = security::load(&bytes, self.password())?;
        let pages = pdf.get_pages();
        let page_id = *pages
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        let resolver = crate::link_destination::Resolver::new(&pdf);
        Ok(details_of(&pdf, page_id, &geo, &resolver, &|page| {
            self.page_geometry(page)
        }))
    }

    /// Déplace une annotation de `(dx, dy)` points d'affichage (toute annotation déplaçable).
    pub fn move_annotation(
        &mut self,
        page_index: usize,
        index: usize,
        dx: f32,
        dy: f32,
    ) -> Result<(), EngineError> {
        if !(dx.is_finite() && dy.is_finite()) {
            return Err(EngineError::Other("invalid offset".into()));
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            let st = subtype_of(&dict);
            let c = caps(&st, flags(&dict), text_of(pdf, &dict, b"NM").as_deref());
            if c.locked {
                return Err(EngineError::SecurityRestricted);
            }
            if !c.can_move || dict.has(b"SuniPDFMeasurement") {
                return Err(EngineError::Other("annotation cannot be moved".into()));
            }
            if c.ours
                && matches!(
                    st.as_str(),
                    "Square" | "Circle" | "Line" | "Ink" | "FreeText"
                )
            {
                border_dash_pattern(pdf, &dict)?;
            }
            let (ux, uy) = geo.vec_to_user(dx, dy);
            for key in ["Rect", "QuadPoints", "InkList", "L", "CL", "Vertices"] {
                if let Some(o) = dict.get(key.as_bytes()).ok().cloned() {
                    let moved = translate(pdf, &o, ux, uy);
                    dict.set(key, moved);
                }
            }
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Redimensionne une annotation écrite par SuniPDF (apparence régénérée).
    pub fn resize_annotation(
        &mut self,
        page_index: usize,
        index: usize,
        rect: PointRect,
    ) -> Result<(), EngineError> {
        let to = PointRect {
            left: rect.left.min(rect.right),
            bottom: rect.bottom.min(rect.top),
            right: rect.left.max(rect.right),
            top: rect.bottom.max(rect.top),
        };
        if to.right - to.left < 4.0 || to.top - to.bottom < 4.0 {
            return Err(EngineError::Other("invalid rectangle".into()));
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            check_unlocked(&dict)?;
            if subtype_of(&dict) == "Link" {
                // Un lien n'a pas d'apparence : seul `/Rect` change (annotation de SuniPDF).
                require_ours(pdf, &dict)?;
                dict.set("Rect", rect_obj(geo.rect_to_user(to)));
                touch(&mut dict);
                return store_annot(pdf, page, slot, dict);
            }
            let mut parsed = parse_ours(pdf, &dict, geo)?;
            parsed.fit_to(to);
            let ap = write_model(&mut dict, &mut parsed, geo)?;
            set_ap(pdf, &mut dict, ap);
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Change couleurs, épaisseur et opacité. Annotation de SuniPDF : apparence régénérée
    /// (texte libre : trait = texte et bordure, fond = fond). Surlignage, soulignement, barré,
    /// ondulé et note : couleur et opacité, apparence recréée par PDFium.
    pub fn set_annotation_style(
        &mut self,
        page_index: usize,
        index: usize,
        style: &AnnotStyle,
    ) -> Result<(), EngineError> {
        let mut markup = false;
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            check_unlocked(&dict)?;
            let st = subtype_of(&dict);
            validate_dash_pattern(&style.dash_pattern)?;
            if !style.dash_pattern.is_empty()
                && !matches!(st.as_str(), "Square" | "Circle" | "Line")
            {
                return Err(EngineError::Other(
                    "dashed borders require a shape annotation".into(),
                ));
            }
            if matches!(
                st.as_str(),
                "Highlight" | "Underline" | "StrikeOut" | "Squiggly" | "Text"
            ) {
                dict.set("C", rgb_obj(style.rgb));
                dict.set("CA", Object::Real(style.opacity.clamp(0.0, 1.0)));
                // PDFium régénère l'apparence au rendu quand elle manque.
                dict.remove(b"AP");
                dict.remove(b"PDFIUM_HasGeneratedAP");
                markup = true;
            } else {
                let mut parsed = parse_ours(pdf, &dict, geo)?;
                if let Model::FreeText { fts, .. } = &mut parsed.model {
                    fts.rgb = style.rgb;
                    fts.fill_rgb = style.fill_rgb;
                    fts.border_pt = style.width_pt;
                }
                parsed.style = style.clone();
                let ap = write_model(&mut dict, &mut parsed, geo)?;
                set_ap(pdf, &mut dict, ap);
            }
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })?;
        if markup {
            // Génère et mémorise l'apparence avant le prochain instantané.
            self.ensure_appearances(page_index)?;
        }
        Ok(())
    }

    /// Texte, taille et couleurs d'un texte libre de SuniPDF.
    pub fn update_free_text(
        &mut self,
        page_index: usize,
        index: usize,
        text: &str,
        style: &FreeTextStyle,
    ) -> Result<(), EngineError> {
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            check_unlocked(&dict)?;
            let mut parsed = parse_ours(pdf, &dict, geo)?;
            let Model::FreeText { text: t, fts, .. } = &mut parsed.model else {
                return Err(EngineError::Other("annotation is not a free text".into()));
            };
            *t = text.to_string();
            *fts = style.clone();
            parsed.style.rgb = style.rgb;
            parsed.style.fill_rgb = style.fill_rgb;
            parsed.style.width_pt = style.border_pt;
            let ap = write_model(&mut dict, &mut parsed, geo)?;
            set_ap(pdf, &mut dict, ap);
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Texte (`/Contents`) d'une annotation : le commentaire d'un surlignage, d'une forme…
    pub fn set_annotation_contents(
        &mut self,
        page_index: usize,
        index: usize,
        text: &str,
    ) -> Result<(), EngineError> {
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            check_unlocked(&dict)?;
            let st = subtype_of(&dict);
            let c = caps(&st, flags(&dict), text_of(pdf, &dict, b"NM").as_deref());
            if !c.can_edit_text || dict.has(b"SuniPDFMeasurement") || dict.has(b"SuniPDFCount") {
                return Err(EngineError::Other(
                    "annotation text cannot be edited".into(),
                ));
            }
            if st == "FreeText" {
                let mut parsed = parse_ours(pdf, &dict, geo)?;
                if let Model::FreeText { text: t, .. } = &mut parsed.model {
                    *t = text.to_string();
                }
                let ap = write_model(&mut dict, &mut parsed, geo)?;
                set_ap(pdf, &mut dict, ap);
            } else if text.is_empty() {
                dict.remove(b"Contents");
            } else {
                dict.set("Contents", lopdf::text_string(text));
            }
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }

    /// Supprime l'annotation, ses réponses et états (à tous les niveaux) et les fenêtres
    /// contextuelles qui en dépendent. Renvoie le nombre d'entrées retirées.
    pub fn delete_annotation_thread(
        &mut self,
        page_index: usize,
        index: usize,
    ) -> Result<usize, EngineError> {
        self.edit_pdf(page_index, |pdf, page, _geo| {
            let entries = annots_entries(pdf, page);
            let ids: Vec<Option<ObjectId>> =
                entries.iter().map(|e| e.as_reference().ok()).collect();
            if index >= entries.len() {
                return Err(not_found(index));
            }
            let irt = |i: usize| -> Option<ObjectId> {
                let d = entry_dict(pdf, &entries[i])?;
                d.get(b"IRT").ok().and_then(|o| o.as_reference().ok())
            };
            let mut gone: HashSet<usize> = HashSet::from([index]);
            // Descendants : l'ordre du tableau place un parent avant ses enfants, mais deux passes
            // suffisent aussi pour un tableau désordonné jusqu'à une profondeur raisonnable.
            loop {
                let before = gone.len();
                for (i, entry) in entries.iter().enumerate() {
                    if gone.contains(&i) {
                        continue;
                    }
                    let child_of_gone =
                        irt(i).is_some_and(|p| gone.iter().any(|g| ids[*g] == Some(p)));
                    // Fenêtre contextuelle dont le parent part.
                    let popup_of_gone = entry_dict(pdf, entry).is_some_and(|d| {
                        subtype_of(d) == "Popup"
                            && d.get(b"Parent")
                                .ok()
                                .and_then(|o| o.as_reference().ok())
                                .is_some_and(|p| gone.iter().any(|g| ids[*g] == Some(p)))
                    });
                    if child_of_gone || popup_of_gone {
                        gone.insert(i);
                    }
                }
                if gone.len() == before {
                    break;
                }
            }
            let kept: Vec<Object> = entries
                .into_iter()
                .enumerate()
                .filter_map(|(i, e)| (!gone.contains(&i)).then_some(e))
                .collect();
            let removed = gone.len();
            *annots_mut(pdf, page)? = kept;
            Ok(removed)
        })
    }

    /// Réponse à une annotation (fil de discussion, ISO 32000-1 §12.5.6.4), sans icône sur la page.
    /// Renvoie l'indice de la réponse.
    pub fn add_reply(
        &mut self,
        page_index: usize,
        parent: usize,
        text: &str,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if text.trim().is_empty() {
            return Err(EngineError::Other("empty text".into()));
        }
        self.add_child(page_index, parent, author, |d| {
            d.set("Contents", lopdf::text_string(text));
            d.set("Name", name("Comment"));
        })
    }

    /// Change le statut de relecture d'un fil (annotation d'état liée au parent).
    pub fn set_review_status(
        &mut self,
        page_index: usize,
        parent: usize,
        status: ReviewStatus,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        self.add_child(page_index, parent, author, |d| {
            d.set("StateModel", lopdf::text_string("Review"));
            d.set("State", lopdf::text_string(status.pdf_name()));
        })
    }

    fn add_child(
        &mut self,
        page_index: usize,
        parent: usize,
        author: Option<&str>,
        fill: impl FnOnce(&mut Dictionary),
    ) -> Result<usize, EngineError> {
        self.edit_pdf(page_index, |pdf, page, geo| {
            let (slot, pdict) = load_annot(pdf, page, parent)?;
            let Slot::Indirect(parent_id) = slot else {
                return Err(EngineError::Other("annotation cannot be replied to".into()));
            };
            if matches!(subtype_of(&pdict).as_str(), "Popup" | "Widget") {
                return Err(EngineError::Other("annotation cannot be replied to".into()));
            }
            let pr = rect_of(pdf, &pdict).unwrap_or(PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
            });
            // Coin haut-gauche d'affichage du parent.
            let disp = geo.rect_to_display(pr);
            let at = PointRect {
                left: disp.left,
                bottom: disp.top - 20.0,
                right: disp.left + 20.0,
                top: disp.top,
            };
            let mut d = base_annotation("Text", author)?;
            d.set("Rect", rect_obj(geo.rect_to_user(at)));
            d.set("IRT", Object::Reference(parent_id));
            d.set("RT", name("R"));
            fill(&mut d);
            push_annotation(pdf, page, d, Some(empty_ap()))
        })
    }
}

pub(crate) fn check_unlocked(dict: &Dictionary) -> Result<(), EngineError> {
    if flags(dict) & 128 != 0 {
        return Err(EngineError::SecurityRestricted);
    }
    Ok(())
}

fn require_ours(pdf: &lopdf::Document, dict: &Dictionary) -> Result<(), EngineError> {
    if text_of(pdf, dict, b"NM").is_some_and(|n| n.starts_with(NAME_PREFIX)) {
        Ok(())
    } else {
        Err(EngineError::Other("annotation cannot be edited".into()))
    }
}

fn entry_dict<'a>(pdf: &'a lopdf::Document, e: &'a Object) -> Option<&'a Dictionary> {
    deref(pdf, e)?.as_dict().ok()
}

/// Translate récursivement les nombres d'un tableau de géométrie (paires x y, ou tableaux de
/// tableaux pour `/InkList`) : `Rect`, `QuadPoints`, `L`, `CL`, `Vertices` alternent x et y.
fn translate(pdf: &lopdf::Document, o: &Object, dx: f32, dy: f32) -> Object {
    let Some(o) = deref(pdf, o) else {
        return Object::Null;
    };
    match o {
        Object::Array(a) => {
            let nested = a
                .iter()
                .any(|x| matches!(deref(pdf, x), Some(Object::Array(_))));
            if nested {
                Object::Array(a.iter().map(|x| translate(pdf, x, dx, dy)).collect())
            } else {
                Object::Array(
                    a.iter()
                        .enumerate()
                        .map(
                            |(i, x)| match deref(pdf, x).and_then(|x| x.as_float().ok()) {
                                Some(v) => Object::Real(v + if i % 2 == 0 { dx } else { dy }),
                                None => x.clone(),
                            },
                        )
                        .collect(),
                )
            }
        }
        other => other.clone(),
    }
}

fn details_of(
    pdf: &lopdf::Document,
    page_id: ObjectId,
    geo: &PageGeometry,
    resolver: &crate::link_destination::Resolver<'_>,
    target_geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
) -> Vec<AnnotDetail> {
    let entries = annots_entries(pdf, page_id);
    let ids: Vec<Option<ObjectId>> = entries.iter().map(|e| e.as_reference().ok()).collect();
    let by_id: HashMap<ObjectId, usize> = ids
        .iter()
        .enumerate()
        .filter_map(|(i, id)| id.map(|id| (id, i)))
        .collect();
    let mut out: Vec<AnnotDetail> = entries
        .iter()
        .enumerate()
        .map(|(index, e)| match entry_dict(pdf, e) {
            Some(d) => detail_of(pdf, d, index, geo, resolver, target_geometry, &by_id),
            None => AnnotDetail {
                index,
                subtype: "Unknown".into(),
                rect: PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                    top: 0.0,
                },
                contents: None,
                author: None,
                modified: None,
                name: None,
                rgb: None,
                fill_rgb: None,
                width_pt: None,
                opacity: 1.0,
                dash_pattern: Vec::new(),
                font_size: None,
                parent: None,
                status: None,
                link: None,
                arrow: false,
                hidden: false,
                locked: false,
                ours: false,
                can_move: false,
                can_resize: false,
                can_restyle: false,
                can_edit_text: false,
                can_edit_link: false,
                replies: 0,
            },
        })
        .collect();
    // Descendants : on remonte la chaîne `/IRT` de chaque annotation (bornée contre un cycle).
    let parents: Vec<Option<usize>> = out.iter().map(|d| d.parent).collect();
    for i in 0..parents.len() {
        let (mut cur, mut hops) = (parents[i], 0);
        while let Some(p) = cur {
            if hops > parents.len() {
                break;
            }
            out[p].replies += 1;
            cur = parents[p];
            hops += 1;
        }
    }
    out
}

fn detail_of(
    pdf: &lopdf::Document,
    d: &Dictionary,
    index: usize,
    geo: &PageGeometry,
    resolver: &crate::link_destination::Resolver<'_>,
    target_geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
    by_id: &HashMap<ObjectId, usize>,
) -> AnnotDetail {
    let subtype = subtype_of(d);
    let nm = text_of(pdf, d, b"NM");
    let c = caps(&subtype, flags(d), nm.as_deref());
    let dash_pattern = border_dash_pattern(pdf, d);
    let invalid_border = c.ours
        && matches!(
            subtype.as_str(),
            "Square" | "Circle" | "Line" | "Ink" | "FreeText"
        )
        && dash_pattern.is_err();
    let rect = rect_of(pdf, d).map_or(
        PointRect {
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
            top: 0.0,
        },
        |r| geo.rect_to_display(r),
    );
    let stroke = color_of(pdf, d, b"C");
    let mut detail = AnnotDetail {
        index,
        rect,
        contents: text_of(pdf, d, b"Contents").filter(|t| !t.is_empty()),
        author: text_of(pdf, d, b"T").filter(|t| !t.is_empty()),
        modified: text_of(pdf, d, b"M").filter(|t| !t.is_empty()),
        name: nm,
        rgb: stroke,
        fill_rgb: color_of(pdf, d, b"IC"),
        width_pt: border_width(pdf, d),
        opacity: get(pdf, d, b"CA")
            .and_then(|o| o.as_float().ok())
            .unwrap_or(1.0),
        dash_pattern: dash_pattern.unwrap_or_default(),
        font_size: None,
        parent: d
            .get(b"IRT")
            .ok()
            .and_then(|o| o.as_reference().ok())
            .and_then(|id| by_id.get(&id).copied())
            .filter(|p| *p != index),
        status: None,
        link: None,
        arrow: false,
        hidden: c.hidden,
        locked: c.locked,
        ours: c.ours,
        can_move: c.can_move && !invalid_border,
        can_resize: c.can_resize && !invalid_border,
        can_restyle: c.can_restyle && !invalid_border,
        can_edit_text: c.can_edit_text && !(invalid_border && subtype == "FreeText"),
        can_edit_link: subtype == "Link" && link_target_editable(pdf, d),
        replies: 0,
        subtype: subtype.clone(),
    };
    if d.has(b"SuniPDFRich") {
        detail.can_resize = false;
        detail.can_restyle = false;
        if subtype == "FreeText" {
            detail.can_edit_text = false;
        }
    }
    if d.has(b"SuniPDFMeasurement") {
        detail.can_move = false;
        detail.can_edit_text = false;
    }
    if d.has(b"SuniPDFCount") {
        detail.can_edit_text = false;
    }
    if get(pdf, d, b"StateModel").is_some() {
        let state = text_of(pdf, d, b"State").unwrap_or_default();
        detail.status = Some(
            ReviewStatus::ALL
                .into_iter()
                .find(|s| s.pdf_name().eq_ignore_ascii_case(&state))
                .unwrap_or_default(),
        );
    }
    match subtype.as_str() {
        "FreeText" => {
            let (size, color) = text_of(pdf, d, b"DA")
                .map(|da| parse_da(&da))
                .unwrap_or_default();
            detail.font_size = size;
            detail.rgb = color.or(Some((0, 0, 0)));
            detail.fill_rgb = stroke;
        }
        "Line" => {
            detail.arrow = get(pdf, d, b"LE")
                .and_then(|o| o.as_array().ok())
                .is_some_and(|a| {
                    a.iter().any(|e| {
                        deref(pdf, e)
                            .and_then(|e| e.as_name().ok())
                            .is_some_and(|n| n == b"OpenArrow")
                    })
                });
        }
        "Link" => detail.link = Some(link_of(resolver, d, target_geometry)),
        _ => {}
    }
    detail
}

/// Cible d'un lien : `/Dest` ou action `GoTo` (page seulement), `URI` ; le reste n'est pas exécuté.
fn link_of(
    resolver: &crate::link_destination::Resolver<'_>,
    d: &Dictionary,
    target_geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
) -> LinkTarget {
    resolver
        .link(d, target_geometry)
        .map(|(target, _)| target)
        .unwrap_or(LinkTarget::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rotation;

    #[test]
    fn dates_and_names() {
        assert_eq!(pdf_date(0), "D:19700101000000Z");
        assert_eq!(pdf_date(1_000_000_000), "D:20010909014640Z");
        assert_eq!(pdf_date(1_760_000_000), "D:20251009085320Z");
        let n = new_name().unwrap();
        assert!(n.starts_with(NAME_PREFIX) && n.len() == NAME_PREFIX.len() + 16);
        assert_ne!(n, new_name().unwrap());
    }

    #[test]
    fn da_strings_are_parsed() {
        assert_eq!(
            parse_da("/Helv 12 Tf 0 0 1 rg"),
            (Some(12.0), Some((0, 0, 255)))
        );
        assert_eq!(parse_da("0 g"), (None, None));
    }

    #[test]
    fn dash_patterns_are_bounded_and_resolve_standard_defaults() {
        for valid in [vec![], vec![0.1], vec![0.0, 3.0], vec![1000.0; 16]] {
            validate_dash_pattern(&valid).unwrap();
        }
        for invalid in [
            vec![0.0],
            vec![0.099],
            vec![1000.1],
            vec![-1.0],
            vec![f32::NAN],
            vec![f32::INFINITY],
            vec![1.0; 17],
        ] {
            assert!(validate_dash_pattern(&invalid).is_err(), "{invalid:?}");
        }
        let mut pdf = lopdf::Document::new();
        let mut dict = Dictionary::new();
        assert!(border_dash_pattern(&pdf, &dict).unwrap().is_empty());
        dict.set(
            "Border",
            Object::Array(vec![0.into(), 0.into(), 1.into(), nums(&[6.0, 2.0, 3.0])]),
        );
        assert_eq!(border_dash_pattern(&pdf, &dict).unwrap(), [6.0, 2.0, 3.0]);
        dict.remove(b"Border");
        dict.set("BS", lopdf::dictionary! {"S" => "D"});
        assert_eq!(border_dash_pattern(&pdf, &dict).unwrap(), [3.0]);
        dict.set(
            "BS",
            lopdf::dictionary! {"S" => "D", "D" => Vec::<Object>::new()},
        );
        assert!(border_dash_pattern(&pdf, &dict).unwrap().is_empty());
        let gap = pdf.add_object(Object::Real(4.25));
        let pattern = pdf.add_object(Object::Array(vec![
            Object::Real(0.0),
            Object::Reference(gap),
        ]));
        let border =
            pdf.add_object(lopdf::dictionary! {"S" => "D", "D" => Object::Reference(pattern)});
        dict.set("BS", Object::Reference(border));
        assert_eq!(border_dash_pattern(&pdf, &dict).unwrap(), [0.0, 4.25]);
        for invalid in [
            Object::Null,
            lopdf::dictionary! {"S" => "B"}.into(),
            lopdf::dictionary! {"S" => Object::Integer(1)}.into(),
            lopdf::dictionary! {"S" => "D", "D" => Object::Null}.into(),
            lopdf::dictionary! {"S" => "D", "D" => vec![Object::Real(0.0)]}.into(),
            lopdf::dictionary! {"S" => "D", "D" => vec![Object::Real(0.01)]}.into(),
        ] {
            dict.set("BS", invalid);
            assert!(border_dash_pattern(&pdf, &dict).is_err());
        }
        let cycle = pdf.new_object_id();
        pdf.objects.insert(cycle, Object::Reference(cycle));
        dict.set("BS", Object::Reference(cycle));
        assert!(border_dash_pattern(&pdf, &dict).is_err());
        let geo = PageGeometry {
            bbox: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 1.0e9,
                top: 1.0e9,
            },
            rotation: Rotation::R0,
        };
        let mut parsed = Parsed {
            model: Model::Square(geo.bbox),
            style: AnnotStyle {
                dash_pattern: vec![0.1, 0.1],
                ..Default::default()
            },
        };
        assert!(write_model(&mut Dictionary::new(), &mut parsed, &geo).is_err());
    }

    #[test]
    fn rights_follow_subtype_producer_and_lock() {
        let c = caps("Square", 4, Some("SuniPDF-00"));
        assert!(c.ours && c.can_move && c.can_resize && c.can_restyle && c.can_edit_text);
        let c = caps("Square", 4, Some("Acrobat-1"));
        assert!(!c.ours && c.can_move && !c.can_resize && !c.can_restyle);
        let c = caps("Highlight", 4, None);
        assert!(!c.can_move && c.can_restyle && c.can_edit_text);
        let c = caps("FreeText", 4, None);
        assert!(c.can_move && !c.can_edit_text);
        let c = caps("Square", 128 | 2, Some("SuniPDF-00"));
        assert!(c.locked && c.hidden && !c.can_move && !c.can_resize && !c.can_restyle);
        assert!(!caps("Popup", 0, None).can_edit_text);
    }

    #[test]
    fn link_target_rights_resolve_flags_and_reject_malformed_values() {
        let mut pdf = lopdf::Document::new();
        let mut dict = Dictionary::new();
        assert!(link_target_editable(&pdf, &dict));
        for flag in [0, 4, 64, 128, 512, -1024] {
            let editable = matches!(flag, 0 | 4);
            dict.set("F", flag);
            assert_eq!(link_target_editable(&pdf, &dict), editable);
            let reference = pdf.add_object(Object::Integer(flag));
            dict.set("F", Object::Reference(reference));
            assert_eq!(link_target_editable(&pdf, &dict), editable);
        }
        for flag in [Object::Null, Object::Real(0.0), name("ReadOnly")] {
            dict.set("F", flag);
            assert!(!link_target_editable(&pdf, &dict));
        }
        let cyclic = pdf.new_object_id();
        pdf.objects.insert(cyclic, Object::Reference(cyclic));
        dict.set("F", Object::Reference(cyclic));
        assert!(!link_target_editable(&pdf, &dict));
    }

    #[test]
    fn fitting_scales_ink_and_keeps_the_margin() {
        let mut p = Parsed {
            model: Model::Ink(vec![vec![(10.0, 10.0), (20.0, 30.0)]]),
            style: AnnotStyle::default(),
        };
        let m = p.margin();
        let r = p.rect();
        assert_eq!((r.left, r.bottom), (10.0 - m, 10.0 - m));
        let to = PointRect {
            left: 100.0,
            bottom: 100.0,
            right: 200.0,
            top: 300.0,
        };
        p.fit_to(to);
        let r = p.rect();
        assert!(
            (r.left - 100.0).abs() < 1e-3 && (r.top - 300.0).abs() < 1e-3,
            "{r:?}"
        );
    }

    #[test]
    fn translation_moves_pairs_and_nested_lists() {
        let pdf = lopdf::Document::new();
        let rect = Object::Array(vec![1.into(), 2.into(), 3.into(), 4.into()]);
        let moved = translate(&pdf, &rect, 10.0, 20.0);
        assert_eq!(
            moved
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o.as_float().unwrap())
                .collect::<Vec<_>>(),
            [11.0, 22.0, 13.0, 24.0]
        );
        let ink = Object::Array(vec![Object::Array(vec![
            0.into(),
            0.into(),
            5.into(),
            5.into(),
        ])]);
        let moved = translate(&pdf, &ink, 1.0, 2.0);
        let inner = moved.as_array().unwrap()[0].as_array().unwrap().clone();
        assert_eq!(
            inner
                .iter()
                .map(|o| o.as_float().unwrap())
                .collect::<Vec<_>>(),
            [1.0, 2.0, 6.0, 7.0]
        );
    }

    #[test]
    fn ap_streams_carry_the_page_rotation_in_their_matrix() {
        let geo = PageGeometry {
            bbox: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 100.0,
                top: 200.0,
            },
            rotation: Rotation::R90,
        };
        let ap = appearance::square_ap(10.0, 20.0, &AnnotStyle::default());
        let s = ap_stream(&ap, &geo);
        let m: Vec<f32> = s
            .dict
            .get(b"Matrix")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o.as_float().unwrap())
            .collect();
        assert_eq!(m, [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn thread_status_is_the_last_state_of_the_family() {
        let d = |index, parent, status| AnnotDetail {
            index,
            subtype: "Text".into(),
            rect: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
            },
            contents: None,
            author: None,
            modified: None,
            name: None,
            rgb: None,
            fill_rgb: None,
            width_pt: None,
            opacity: 1.0,
            dash_pattern: Vec::new(),
            font_size: None,
            parent,
            status,
            link: None,
            arrow: false,
            hidden: false,
            locked: false,
            ours: false,
            can_move: false,
            can_resize: false,
            can_restyle: false,
            can_edit_text: false,
            can_edit_link: false,
            replies: 0,
        };
        let list = vec![
            d(0, None, None),
            d(1, Some(0), Some(ReviewStatus::Accepted)),
            d(2, Some(1), None),
            d(3, Some(0), Some(ReviewStatus::Rejected)),
            d(4, None, Some(ReviewStatus::Completed)),
        ];
        assert_eq!(thread_status(&list, 0), ReviewStatus::Rejected);
        assert_eq!(thread_status(&list, 2), ReviewStatus::None);
        assert_eq!(
            ReviewStatus::from_id("cancelled"),
            Some(ReviewStatus::Cancelled)
        );
        assert_eq!(ReviewStatus::from_id("x"), None);
    }
}
