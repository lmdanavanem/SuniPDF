//! Bounded local markup registers and manual count markers. No symbol detection is implied.

use crate::annot_rich::{checked_rect, rich_base};
use crate::annot_write::{ap_stream, load_annot, push_annotation, store_annot, touch};
use crate::appearance::{fmt_num, Ap};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PointRect};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

const MAX_ROWS: usize = 10_000;
const MAX_TEXT: usize = 4 * 1024 * 1024;

fn bad() -> EngineError {
    EngineError::Other("annotation register: unsupported, malformed or excessive metadata; export was not completed".into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn short(text: &str, limit: usize) -> bool {
    text.len() <= limit && !text.chars().any(char::is_control)
}

pub fn validate_count_group(group: &str) -> Result<(), EngineError> {
    if group.trim().is_empty() || group != group.trim() || !short(group, 512) {
        return Err(EngineError::Other("count group must contain 1–512 UTF-8 bytes, without surrounding whitespace or control characters".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterFields {
    pub business_status: Option<String>,
    pub discipline: Option<String>,
    pub custom: BTreeMap<String, String>,
}

impl RegisterFields {
    pub fn validate(&self) -> Result<(), EngineError> {
        if [&self.business_status, &self.discipline].iter().any(|v| {
            v.as_ref()
                .is_some_and(|s| s.trim().is_empty() || s != s.trim() || !short(s, 512))
        }) || self.custom.len() > 16
            || self.custom.iter().any(|(k, v)| {
                k.trim().is_empty()
                    || k != k.trim()
                    || k.contains('=')
                    || !short(k, 64)
                    || !short(v, 1024)
            })
        {
            return Err(bad());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredFields {
    version: u8,
    fields: RegisterFields,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCount {
    version: u8,
    group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegisterRow {
    /// Physical page number (one-based); annotation index is zero-based in /Annots.
    pub page: usize,
    pub index: usize,
    pub annotation_id: Option<String>,
    pub kind: String,
    pub author: Option<String>,
    pub modified: Option<String>,
    pub contents: Option<String>,
    pub subject: Option<String>,
    /// Displayed-page rectangle, or unknown. Never substitute a zero rectangle.
    pub rect: Option<[f32; 4]>,
    pub parent: Option<usize>,
    pub is_reply: bool,
    pub hidden: bool,
    pub locked: bool,
    /// Standard PDF Review state, kept separate from the business workflow field.
    pub review_status: Option<String>,
    pub fields: RegisterFields,
    pub count_group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RegisterFilter {
    pub query: String,
    pub page: Option<usize>,
    pub author: String,
    pub modified: String,
    pub review_status: String,
    pub business_status: String,
    pub discipline: String,
    pub kind: String,
    pub count_group: String,
    pub include_replies: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RegisterSort {
    #[default]
    Page,
    Author,
    Modified,
    Kind,
    ReviewStatus,
    BusinessStatus,
    Discipline,
    CountGroup,
}

impl RegisterSort {
    pub const ALL: [Self; 8] = [
        Self::Page,
        Self::Author,
        Self::Modified,
        Self::Kind,
        Self::ReviewStatus,
        Self::BusinessStatus,
        Self::Discipline,
        Self::CountGroup,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Author => "author",
            Self::Modified => "modified",
            Self::Kind => "kind",
            Self::ReviewStatus => "review_status",
            Self::BusinessStatus => "business_status",
            Self::Discipline => "discipline",
            Self::CountGroup => "count_group",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnnotationRegister {
    pub source_sha256: String,
    pub page_count: usize,
    pub rows: Vec<RegisterRow>,
    pub(crate) source_snapshot: Arc<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct RegisterTarget {
    pub source_sha256: String,
    pub page: usize,
    pub index: usize,
    pub annotation_id: Option<String>,
    source_snapshot: Arc<Vec<u8>>,
}

impl AnnotationRegister {
    pub fn target(&self, row: &RegisterRow) -> RegisterTarget {
        RegisterTarget {
            source_sha256: self.source_sha256.clone(),
            page: row.page,
            index: row.index,
            annotation_id: row.annotation_id.clone(),
            source_snapshot: self.source_snapshot.clone(),
        }
    }

    pub fn select(
        &self,
        filter: &RegisterFilter,
        sort: RegisterSort,
        descending: bool,
    ) -> Vec<&RegisterRow> {
        self.select_indices(filter, sort, descending)
            .into_iter()
            .map(|index| &self.rows[index])
            .collect()
    }

    /// Stable row offsets suitable for caching without copying annotation contents.
    pub fn select_indices(
        &self,
        filter: &RegisterFilter,
        sort: RegisterSort,
        descending: bool,
    ) -> Vec<usize> {
        let query = filter.query.to_lowercase();
        let exact =
            |wanted: &str, actual: Option<&str>| wanted.is_empty() || actual == Some(wanted);
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                (filter.include_replies || !r.is_reply)
                    && filter.page.is_none_or(|p| p == r.page)
                    && exact(&filter.author, r.author.as_deref())
                    && (filter.modified.is_empty()
                        || r.modified
                            .as_ref()
                            .is_some_and(|s| s.contains(&filter.modified)))
                    && exact(&filter.review_status, r.review_status.as_deref())
                    && exact(&filter.business_status, r.fields.business_status.as_deref())
                    && exact(&filter.discipline, r.fields.discipline.as_deref())
                    && exact(&filter.kind, Some(&r.kind))
                    && exact(&filter.count_group, r.count_group.as_deref())
                    && (query.is_empty()
                        || [
                            r.author.as_deref(),
                            r.modified.as_deref(),
                            r.contents.as_deref(),
                            r.subject.as_deref(),
                            r.annotation_id.as_deref(),
                            Some(&r.kind),
                            r.review_status.as_deref(),
                            r.fields.business_status.as_deref(),
                            r.fields.discipline.as_deref(),
                            r.count_group.as_deref(),
                        ]
                        .into_iter()
                        .flatten()
                        .any(|s| s.to_lowercase().contains(&query))
                        || r.fields.custom.iter().any(|(k, v)| {
                            k.to_lowercase().contains(&query) || v.to_lowercase().contains(&query)
                        }))
            })
            .map(|(index, _)| index)
            .collect();
        rows.sort_by(|a, b| {
            let (a, b) = (&self.rows[*a], &self.rows[*b]);
            let order = match sort {
                RegisterSort::Page => std::cmp::Ordering::Equal,
                RegisterSort::Author => a.author.cmp(&b.author),
                RegisterSort::Modified => a.modified.cmp(&b.modified),
                RegisterSort::Kind => a.kind.cmp(&b.kind),
                RegisterSort::ReviewStatus => a.review_status.cmp(&b.review_status),
                RegisterSort::BusinessStatus => {
                    a.fields.business_status.cmp(&b.fields.business_status)
                }
                RegisterSort::Discipline => a.fields.discipline.cmp(&b.fields.discipline),
                RegisterSort::CountGroup => a.count_group.cmp(&b.count_group),
            }
            .then_with(|| (a.page, a.index).cmp(&(b.page, b.index)));
            if descending {
                order.reverse()
            } else {
                order
            }
        });
        rows
    }

    pub fn json(
        &self,
        filter: &RegisterFilter,
        sort: RegisterSort,
        descending: bool,
    ) -> serde_json::Value {
        let rows = self.select(filter, sort, descending);
        let mut counts: BTreeMap<&str, Vec<serde_json::Value>> = BTreeMap::new();
        for r in &rows {
            if let Some(group) = &r.count_group {
                counts.entry(group).or_default().push(serde_json::json!({"page":r.page,"index":r.index,"annotation_id":r.annotation_id}));
            }
        }
        serde_json::json!({"schema":"sunipdf-annotation-register","version":1,"complete":true,
            "source_sha256":self.source_sha256,"page_count":self.page_count,"total_annotations":self.rows.len(),
            "coordinate_system":"displayed-page-points-bottom-left","filter":filter,"sort":sort.id(),"descending":descending,
            "matched_annotations":rows.len(),"rows":rows,
            "counts":counts.into_iter().map(|(group,markers)|serde_json::json!({"group":group,"quantity":markers.len(),"markers":markers})).collect::<Vec<_>>()})
    }

    /// Every value is quoted; dangerous spreadsheet prefixes are escaped even after whitespace/BOM.
    pub fn csv(&self, filter: &RegisterFilter, sort: RegisterSort, descending: bool) -> String {
        let mut output = String::from("source_sha256,page,index,annotation_id,kind,author,modified,review_status,business_status,discipline,subject,contents,count_group,quantity,rect,custom_fields,is_reply,hidden,locked\r\n");
        for r in self.select(filter, sort, descending) {
            let cells = [
                Some(self.source_sha256.clone()),
                Some(r.page.to_string()),
                Some(r.index.to_string()),
                r.annotation_id.clone(),
                Some(r.kind.clone()),
                r.author.clone(),
                r.modified.clone(),
                r.review_status.clone(),
                r.fields.business_status.clone(),
                r.fields.discipline.clone(),
                r.subject.clone(),
                r.contents.clone(),
                r.count_group.clone(),
                r.count_group.as_ref().map(|_| "1".to_string()),
                r.rect
                    .map(|v| serde_json::to_string(&v).expect("finite rectangle")),
                Some(serde_json::to_string(&r.fields.custom).expect("string map")),
                Some(r.is_reply.to_string()),
                Some(r.hidden.to_string()),
                Some(r.locked.to_string()),
            ];
            output.push_str(
                &cells
                    .into_iter()
                    .map(|v| csv_cell(v.as_deref().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            output.push_str("\r\n");
        }
        output
    }
}

pub(crate) fn csv_cell(text: &str) -> String {
    let first = text
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}')
        .chars()
        .next();
    let dangerous = first.is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
        || text.chars().next().is_some_and(char::is_control);
    format!(
        "\"{}{}\"",
        if dangerous { "'" } else { "" },
        text.replace('"', "\"\"")
    )
}

fn text(
    pdf: &lopdf::Document,
    dict: &Dictionary,
    key: &[u8],
    budget: &mut usize,
) -> Result<Option<String>, EngineError> {
    let Ok(raw) = dict.get(key) else {
        return Ok(None);
    };
    let (_, raw) = pdf.dereference(raw).map_err(lopdf_err)?;
    let bytes = match raw {
        Object::String(v, _) | Object::Name(v) => v,
        _ => return Err(bad()),
    };
    *budget = budget.checked_add(bytes.len()).ok_or_else(bad)?;
    if bytes.len() > 16384 || *budget > MAX_TEXT {
        return Err(bad());
    }
    match raw {
        Object::Name(v) => Ok(Some(String::from_utf8_lossy(v).into_owned())),
        _ => crate::annot_write::text_of(pdf, dict, key)
            .map(Some)
            .ok_or_else(bad),
    }
}

fn stored<T: serde::de::DeserializeOwned>(
    dict: &Dictionary,
    key: &[u8],
    budget: &mut usize,
) -> Result<Option<T>, EngineError> {
    let Ok(raw) = dict.get(key) else {
        return Ok(None);
    };
    let bytes = raw.as_str().map_err(lopdf_err)?;
    *budget = budget.checked_add(bytes.len()).ok_or_else(bad)?;
    if bytes.len() > 32768 || *budget > MAX_TEXT {
        return Err(bad());
    }
    serde_json::from_slice(bytes).map(Some).map_err(|_| bad())
}

pub(crate) fn editable(dict: &Dictionary) -> Result<(), EngineError> {
    let flags = dict
        .get(b"F")
        .map_or(Ok(0), Object::as_i64)
        .map_err(lopdf_err)?;
    if flags < 0
        || flags & (64 | 128 | 512) != 0
        || dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Widget")
    {
        return Err(EngineError::SecurityRestricted);
    }
    Ok(())
}

impl Document<'_> {
    /// One explicitly placed marker contributes exactly one item to its group.
    pub fn add_count_marker(
        &mut self,
        page_index: usize,
        point: (f32, f32),
        size: f32,
        group: &str,
        author: Option<&str>,
    ) -> Result<usize, EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        validate_count_group(group)?;
        if !size.is_finite() || !(8.0..=72.0).contains(&size) {
            return Err(bad());
        }
        let rect = PointRect {
            left: point.0 - size / 2.0,
            bottom: point.1 - size / 2.0,
            right: point.0 + size / 2.0,
            top: point.1 + size / 2.0,
        };
        self.edit_annotation_pdf(page_index, |pdf, page, geo| {
            checked_rect(geo, rect)?;
            let mut dict = rich_base("Stamp", rect, geo, author)?;
            dict.set("Name", Object::Name(b"SuniPDFCount".to_vec()));
            dict.set("Contents", lopdf::text_string(&format!("Count: {group}")));
            dict.set("SuniPDFCount", Object::string_literal(serde_json::to_vec(&StoredCount { version:1,group:group.to_string() }).map_err(|_|bad())?));
            let n = |v|fmt_num(v * size);
            let content = format!("q /GS gs 0.15 0.25 0.85 RG 1 1 1 rg 1.5 w {} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c B {} {} m {} {} l {} {} m {} {} l S Q\n", n(0.9), n(0.5), n(0.9),n(0.721),n(0.721),n(0.9),n(0.5),n(0.9), n(0.279),n(0.9),n(0.1),n(0.721),n(0.1),n(0.5), n(0.1),n(0.279),n(0.279),n(0.1),n(0.5),n(0.1), n(0.721),n(0.1),n(0.9),n(0.279),n(0.9),n(0.5), n(0.3),n(0.5),n(0.7),n(0.5),n(0.5),n(0.3),n(0.5),n(0.7));
            push_annotation(pdf,page,dict,Some(ap_stream(&Ap {w:size,h:size,content:content.into_bytes(),opacity:1.0,uses_font:false},geo)))
        })
    }

    /// Reads one snapshot; unreadable pages/records fail the complete export instead of reporting zero.
    pub fn annotation_register(&self) -> Result<AnnotationRegister, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.page_count() > 1000 {
            return Err(bad());
        }
        let mut reader = self.annotation_register_reader()?;
        while !reader.read_next(self)? {}
        reader.finish()
    }

    /// Starts a bounded snapshot reader. Call `read_next` between foreground tasks on the
    /// same engine thread and discard it whenever the document changes.
    pub fn annotation_register_reader(&self) -> Result<RegisterReader<'static>, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.page_count() > 1000 {
            return Err(bad());
        }
        let bytes = self.snapshot()?;
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(bad());
        }
        let pdf = security::load(&bytes, self.password())?;
        RegisterReader::new(bytes, std::borrow::Cow::Owned(pdf), self.page_count())
    }

    pub(crate) fn annotation_register_from_pdf(
        &self,
        bytes: Vec<u8>,
        pdf: &lopdf::Document,
    ) -> Result<AnnotationRegister, EngineError> {
        let mut reader =
            RegisterReader::new(bytes, std::borrow::Cow::Borrowed(pdf), self.page_count())?;
        while !reader.read_next(self)? {}
        reader.finish()
    }

    /// The snapshot hash and identity prevent applying an edit to shifted indices or another tab.
    pub fn set_annotation_register_fields(
        &mut self,
        target: &RegisterTarget,
        fields: &RegisterFields,
        count_group: Option<&str>,
    ) -> Result<(), EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        fields.validate()?;
        if let Some(group) = count_group {
            validate_count_group(group)?;
        }
        if target.source_sha256 != digest(&target.source_snapshot)
            || !crate::annot_rich::same_document(
                &target.source_snapshot,
                &self.snapshot()?,
                self.password(),
            )?
        {
            return Err(EngineError::Other(
                "annotation register is stale; refresh it before editing".into(),
            ));
        }
        let page = target.page.checked_sub(1).ok_or_else(bad)?;
        self.edit_annotation_pdf(page, |pdf, page, _| {
            let (slot, mut dict) = load_annot(pdf, page, target.index)?;
            editable(&dict)?;
            if let Some(previous) = stored::<StoredFields>(&dict, b"SuniPDFRegister", &mut 0)? {
                if previous.version != 1 {
                    return Err(bad());
                }
                previous.fields.validate()?;
            }
            if text(pdf, &dict, b"NM", &mut 0)? != target.annotation_id {
                return Err(bad());
            }
            let count: Option<StoredCount> = stored(&dict, b"SuniPDFCount", &mut 0)?;
            if let Some(group) = count_group {
                let Some(count) = count else {
                    return Err(bad());
                };
                if count.version != 1 {
                    return Err(bad());
                }
                dict.set(
                    "SuniPDFCount",
                    Object::string_literal(
                        serde_json::to_vec(&StoredCount {
                            version: 1,
                            group: group.into(),
                        })
                        .map_err(|_| bad())?,
                    ),
                );
                dict.set("Contents", lopdf::text_string(&format!("Count: {group}")));
            }
            dict.set(
                "SuniPDFRegister",
                Object::string_literal(
                    serde_json::to_vec(&StoredFields {
                        version: 1,
                        fields: fields.clone(),
                    })
                    .map_err(|_| bad())?,
                ),
            );
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_cells_neutralize_formulas_after_spaces_bom_and_controls() {
        for value in [
            "=1+2",
            " @SUM(A1)",
            "\u{feff}-2+3",
            "\tplain",
            "\r\n=1",
            "\0=1",
            "+cmd",
        ] {
            assert!(csv_cell(value).starts_with("\"'"), "{value:?}");
        }
        assert_eq!(csv_cell("a,\"b\"\nc"), "\"a,\"\"b\"\"\nc\"");
        assert_eq!(csv_cell("été 中文"), "\"été 中文\"");
        assert!(validate_count_group("  ").is_err());
        assert!(validate_count_group("Lights\n").is_err());
        assert!(RegisterFields {
            business_status: Some("Ready".into()),
            discipline: None,
            custom: BTreeMap::from([("Room".into(), "A-101".into())])
        }
        .validate()
        .is_ok());
        for key in ["Room=Floor", " Room", "Room "] {
            assert!(
                RegisterFields {
                    custom: BTreeMap::from([(key.into(), "A101".into())]),
                    ..Default::default()
                }
                .validate()
                .is_err(),
                "field keys must round-trip through the text editor without being renamed"
            );
        }
    }
}

/// Snapshot-owned parser whose individual steps read a single page. No PDFium handle is moved
/// between threads. Partial rows are never exposed as a complete register.
pub struct RegisterReader<'a> {
    pdf: std::borrow::Cow<'a, lopdf::Document>,
    bytes: Arc<Vec<u8>>,
    source_sha256: String,
    page_count: usize,
    pages: std::collections::VecDeque<(u32, lopdf::ObjectId)>,
    rows: Vec<RegisterRow>,
    budget: usize,
    names: HashMap<String, usize>,
    failed: bool,
}

impl<'a> RegisterReader<'a> {
    fn new(
        bytes: Vec<u8>,
        pdf: std::borrow::Cow<'a, lopdf::Document>,
        page_count: usize,
    ) -> Result<Self, EngineError> {
        if page_count > 1000 || bytes.len() > 128 * 1024 * 1024 {
            return Err(bad());
        }
        let pages = pdf.get_pages();
        if pages.len() != page_count || pages.keys().copied().ne(1..=page_count as u32) {
            return Err(bad());
        }
        Ok(Self {
            source_sha256: digest(&bytes),
            bytes: Arc::new(bytes),
            pdf,
            page_count,
            pages: pages.into_iter().collect(),
            rows: Vec::new(),
            budget: 0,
            names: HashMap::new(),
            failed: false,
        })
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.page_count - self.pages.len(), self.page_count)
    }

    pub fn read_next(&mut self, doc: &Document<'_>) -> Result<bool, EngineError> {
        if self.failed {
            return Err(bad());
        }
        let result = self.read_page(doc);
        self.failed = result.is_err();
        result
    }

    fn read_page(&mut self, doc: &Document<'_>) -> Result<bool, EngineError> {
        if doc.page_count() != self.page_count {
            return Err(bad());
        }
        let Some((page_no, page_id)) = self.pages.pop_front() else {
            return Ok(true);
        };
        let Self {
            pdf,
            rows,
            budget,
            names,
            ..
        } = self;
        let page = page_no as usize;
        let geo = doc.page_geometry(page - 1)?;
        let page_dict = pdf.get_dictionary(page_id).map_err(lopdf_err)?;
        let entries = match page_dict.get(b"Annots") {
            Err(_) => return Ok(self.pages.is_empty()),
            Ok(raw) => pdf
                .dereference(raw)
                .and_then(|(_, v)| v.as_array())
                .map_err(lopdf_err)?,
        };
        if entries.len() > 4096 || rows.len() + entries.len() > MAX_ROWS {
            return Err(bad());
        }
        let by_id: HashMap<_, _> = entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_reference().ok().map(|id| (id, i)))
            .collect();
        let start = rows.len();
        let mut states = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let (_, obj) = pdf.dereference(entry).map_err(lopdf_err)?;
            let dict = obj.as_dict().map_err(lopdf_err)?;
            pdf.dereference(dict.get(b"Subtype").map_err(lopdf_err)?)
                .and_then(|(_, v)| v.as_name())
                .map_err(lopdf_err)?;
            let kind = text(pdf, dict, b"Subtype", budget)?.ok_or_else(bad)?;
            let annotation_id = text(pdf, dict, b"NM", budget)?;
            if let Some(id) = &annotation_id {
                *names.entry(id.clone()).or_default() += 1;
            }
            let fields: Option<StoredFields> = stored(dict, b"SuniPDFRegister", budget)?;
            let fields = match fields {
                Some(v) if v.version == 1 => {
                    v.fields.validate()?;
                    v.fields
                }
                Some(_) => return Err(bad()),
                None => RegisterFields::default(),
            };
            let rect = match dict.get(b"Rect") {
                Err(_) => None,
                Ok(raw) => {
                    let values = pdf
                        .dereference(raw)
                        .and_then(|(_, v)| v.as_array())
                        .map_err(lopdf_err)?;
                    if values.len() != 4 {
                        return Err(bad());
                    }
                    let mut v = [0.0; 4];
                    for (out, raw) in v.iter_mut().zip(values) {
                        *out = pdf
                            .dereference(raw)
                            .and_then(|(_, v)| v.as_float())
                            .map_err(lopdf_err)?;
                    }
                    if v.iter().any(|n| !n.is_finite()) {
                        return Err(bad());
                    }
                    let r = geo.rect_to_display(PointRect {
                        left: v[0],
                        bottom: v[1],
                        right: v[2],
                        top: v[3],
                    });
                    Some([r.left, r.bottom, r.right, r.top])
                }
            };
            let parent = match dict.get(b"IRT") {
                Err(_) => None,
                Ok(raw) => {
                    let id = raw.as_reference().map_err(lopdf_err)?;
                    by_id.get(&id).copied()
                }
            };
            let is_reply = dict.has(b"IRT");
            let count: Option<StoredCount> = stored(dict, b"SuniPDFCount", budget)?;
            let count_group = match count {
                None => None,
                Some(v) => {
                    validate_count_group(&v.group)?;
                    pdf.dereference(dict.get(b"NM").map_err(lopdf_err)?)
                        .and_then(|(_, v)| v.as_str())
                        .map_err(lopdf_err)?;
                    if v.version != 1
                        || kind != "Stamp"
                        || is_reply
                        || annotation_id
                            .as_ref()
                            .is_none_or(|id| !id.starts_with("SuniPDF-") || !short(id, 128))
                        || rect.is_none_or(|r| {
                            !(8.0..=72.01).contains(&(r[2] - r[0]))
                                || !(8.0..=72.01).contains(&(r[3] - r[1]))
                        })
                    {
                        return Err(bad());
                    }
                    Some(v.group)
                }
            };
            let model = text(pdf, dict, b"StateModel", budget)?;
            let state = text(pdf, dict, b"State", budget)?;
            if model.as_deref() == Some("Review") {
                states.push((index, state.clone()));
            }
            let flags = match dict.get(b"F") {
                Err(_) => 0,
                Ok(v) => pdf
                    .dereference(v)
                    .and_then(|(_, v)| v.as_i64())
                    .map_err(lopdf_err)?,
            };
            rows.push(RegisterRow {
                page,
                index,
                annotation_id,
                kind,
                author: text(pdf, dict, b"T", budget)?,
                modified: text(pdf, dict, b"M", budget)?,
                contents: text(pdf, dict, b"Contents", budget)?,
                subject: text(pdf, dict, b"Subj", budget)?,
                rect,
                parent,
                is_reply,
                hidden: flags & (1 | 2 | 32) != 0,
                locked: flags & (64 | 128 | 512) != 0,
                review_status: if model.as_deref() == Some("Review") {
                    state
                } else {
                    None
                },
                fields,
                count_group,
            });
        }
        // Bound hostile reply cycles/depth before deriving the last standard Review state.
        for index in 0..entries.len() {
            let mut cursor = Some(index);
            let mut seen = HashSet::new();
            while let Some(i) = cursor {
                if !seen.insert(i) || seen.len() > 64 {
                    return Err(bad());
                }
                cursor = rows[start + i].parent;
            }
        }
        for (index, state) in states {
            let mut cursor = rows[start + index].parent;
            while let Some(i) = cursor {
                rows[start + i].review_status = state.clone();
                cursor = rows[start + i].parent;
            }
        }

        Ok(self.pages.is_empty())
    }

    pub fn finish(self) -> Result<AnnotationRegister, EngineError> {
        if self.failed || !self.pages.is_empty() {
            return Err(bad());
        }
        let Self {
            source_sha256,
            page_count,
            rows,
            names,
            bytes,
            ..
        } = self;
        if rows.iter().any(|r| {
            r.count_group.is_some()
                && r.annotation_id
                    .as_ref()
                    .is_none_or(|id| names.get(id) != Some(&1))
        }) {
            return Err(bad());
        }
        Ok(AnnotationRegister {
            source_sha256,
            page_count,
            rows,
            source_snapshot: bytes,
        })
    }
}
