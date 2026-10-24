//! Reviewed, source-bound links to explicitly indexed local PDF sheets.
//!
//! Remote targets are inert data. Opening one requires a fresh plan-set check and
//! explicit confirmation. This module never launches a shell or external viewer.
use crate::{plan_set, Document, Engine, EngineError, LinkTarget, PointRect, RenderedPage};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_SCAN_PAGES: usize = 32;
pub const MAX_PAGE_CHARS: usize = 250_000;
pub const MAX_OCCURRENCES: usize = 512;
pub const MAX_RECIPE_BYTES: usize = 256 * 1024;
pub const MAX_REPORT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 160 * 1024 * 1024;
pub const MAX_PREVIEW_SIDE: u32 = 1536;

mod batch;
pub use batch::*;

fn failure(message: impl Into<String>) -> EngineError {
    EngineError::Other(format!("Plan links: {}", message.into()))
}
fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}
fn hash(bytes: &[u8]) -> String {
    crate::local_review::digest(bytes)
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|v| v.is_ascii_hexdigit())
}
fn identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}
fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.chars().all(identifier_char)
}
fn text_object(value: &str) -> Object {
    let mut bytes = vec![0xfe, 0xff];
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::string_literal(bytes)
}
fn safe_relative(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 4096
        || value.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        || value.chars().any(char::is_control)
    {
        return false;
    }
    let mut normal_seen = false;
    for part in value.split('/') {
        if part == ".." && !normal_seen {
            continue;
        }
        normal_seen = true;
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.len() > 255
            || part.ends_with(['.', ' '])
        {
            return false;
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return false;
        }
    }
    normal_seen && value.to_ascii_lowercase().ends_with(".pdf")
}

/// A page is zero-based here, as in the PDF remote destination array.
/// A stored fingerprint is an expectation, never permission to open a path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemotePageTarget {
    pub relative_path: String,
    pub page: usize,
    pub sheet_id: String,
    pub sha256: String,
    pub byte_len: u64,
}
impl RemotePageTarget {
    fn validate(&self) -> Result<(), EngineError> {
        if !safe_relative(&self.relative_path)
            || self.page >= plan_set::MAX_PAGES
            || self.sheet_id.is_empty()
            || self.sheet_id.len() > 128
            || self.sheet_id.chars().any(char::is_control)
            || !valid_hash(&self.sha256)
            || self.byte_len == 0
            || self.byte_len > plan_set::MAX_FILE_BYTES
        {
            return Err(failure("invalid confined remote target"));
        }
        Ok(())
    }
}

pub(crate) fn remote_action(target: &RemotePageTarget) -> Result<Object, EngineError> {
    target.validate()?;
    let mut file = Dictionary::new();
    file.set("Type", Object::Name(b"Filespec".to_vec()));
    file.set("F", text_object(&target.relative_path));
    file.set("UF", text_object(&target.relative_path));
    let mut action = Dictionary::new();
    action.set("S", Object::Name(b"GoToR".to_vec()));
    action.set("F", file);
    action.set(
        "D",
        vec![
            Object::Integer(target.page as i64),
            Object::Name(b"Fit".to_vec()),
        ],
    );
    action.set("NewWindow", true);
    action.set(
        "SuniPDFPlanTarget",
        Object::string_literal(serde_json::to_vec(target).map_err(|e| failure(e.to_string()))?),
    );
    Ok(Object::Dictionary(action))
}

/// Only the bounded authored form is recognized. Unknown actions stay unsupported.
pub(crate) fn parse_remote_action(action: &Dictionary) -> Result<RemotePageTarget, EngineError> {
    if action.iter().any(|(k, _)| {
        !matches!(
            k.as_slice(),
            b"S" | b"F" | b"D" | b"NewWindow" | b"SuniPDFPlanTarget"
        )
    }) {
        return Err(failure("additional remote action data"));
    }
    let raw = action
        .get(b"SuniPDFPlanTarget")
        .and_then(Object::as_str)
        .map_err(crate::security::lopdf_err)?;
    if raw.len() > 16 * 1024 {
        return Err(failure("remote metadata limit"));
    }
    let target: RemotePageTarget =
        serde_json::from_slice(raw).map_err(|e| failure(e.to_string()))?;
    target.validate()?;
    if action.get(b"S").and_then(Object::as_name).ok() != Some(b"GoToR".as_slice())
        || action.get(b"NewWindow").and_then(Object::as_bool).ok() != Some(true)
    {
        return Err(failure("unsupported remote action"));
    }
    let file = action
        .get(b"F")
        .and_then(Object::as_dict)
        .map_err(crate::security::lopdf_err)?;
    if file.len() != 3
        || file.get(b"Type").and_then(Object::as_name).ok() != Some(b"Filespec".as_slice())
    {
        return Err(failure("unsupported file specification"));
    }
    for key in [b"F".as_slice(), b"UF"] {
        let object = file.get(key).map_err(crate::security::lopdf_err)?;
        if object.as_str().map_err(crate::security::lopdf_err)?.len() > 8194
            || lopdf::decode_text_string(object).map_err(crate::security::lopdf_err)?
                != target.relative_path
        {
            return Err(failure("remote file metadata mismatch"));
        }
    }
    let dest = action
        .get(b"D")
        .and_then(Object::as_array)
        .map_err(crate::security::lopdf_err)?;
    if dest.len() != 2
        || dest[0].as_i64().ok() != Some(target.page as i64)
        || dest[1].as_name().ok() != Some(b"Fit".as_slice())
    {
        return Err(failure("remote page metadata mismatch"));
    }
    Ok(target)
}

/// Borrowed, operation-only credentials keyed by the manifest's file IDs.
#[derive(Default)]
pub struct PlanLinkCredentials<'a> {
    pub passwords: &'a [(&'a str, &'a str)],
}
impl<'a> PlanLinkCredentials<'a> {
    fn get(&self, file: &str) -> Option<&'a str> {
        self.passwords
            .iter()
            .find_map(|(id, value)| (*id == file).then_some(*value))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkScanRequest {
    pub source_file_id: String,
    /// Explicit physical pages, starting at one.
    pub pages: Vec<usize>,
    pub target_sheet_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkSelection {
    pub occurrence_id: String,
    pub fingerprint: String,
    pub target_sheet_id: String,
    pub confirmed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLinkReview {
    pub version: u32,
    pub manifest_sha256: String,
    pub source_sha256: String,
    pub scan: PlanLinkScanRequest,
    pub selections: Vec<PlanLinkSelection>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkTarget {
    pub sheet_id: String,
    pub file_id: String,
    pub number: String,
    pub title: String,
    pub discipline: String,
    pub revision: String,
    pub page: usize,
    pub sha256: String,
    pub byte_len: u64,
    pub status: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkOccurrence {
    pub id: String,
    pub fingerprint: String,
    pub page: usize,
    pub first_character: usize,
    pub end_character: usize,
    pub text: String,
    /// Display coordinates: left, bottom, right, top; absent when geometry is refused.
    pub rect: Option<[f32; 4]>,
    pub target_sheet_ids: Vec<String>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkIssue {
    pub page: Option<usize>,
    pub sheet_id: Option<String>,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkInspection {
    pub version: u32,
    pub manifest_sha256: String,
    pub source_sha256: String,
    pub source_file_id: String,
    pub requested_pages: Vec<usize>,
    pub scanned_pages: usize,
    pub scan_complete: bool,
    pub truncated: bool,
    pub targets: Vec<PlanLinkTarget>,
    pub occurrences: Vec<PlanLinkOccurrence>,
    pub issues: Vec<PlanLinkIssue>,
}
impl PlanLinkInspection {
    pub fn complete(&self) -> bool {
        self.scan_complete
            && !self.truncated
            && self.issues.is_empty()
            && self.occurrences.iter().all(|v| v.status == "ready")
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"complete":self.complete(),"inspection":self})
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CreatedPlanLink {
    pub occurrence_id: String,
    pub page: usize,
    pub annotation_index: usize,
    pub name: String,
    pub rect: [f32; 4],
    pub target: RemotePageTarget,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkReport {
    pub version: u32,
    pub inspection: PlanLinkInspection,
    pub created: Vec<CreatedPlanLink>,
    pub excluded_occurrences: Vec<String>,
    pub output_sha256: String,
}
impl PlanLinkReport {
    pub fn complete(&self) -> bool {
        !self.created.is_empty()
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"complete":self.complete(),"report":self})
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanLinkReceipt {
    pub output: PathBuf,
    pub bytes: usize,
    pub report: PlanLinkReport,
}

fn validate_scan(scan: &PlanLinkScanRequest, set: &plan_set::PlanSet) -> Result<(), EngineError> {
    plan_set::validate(set)?;
    let file = set
        .files
        .iter()
        .find(|f| f.id == scan.source_file_id)
        .ok_or_else(|| failure("source file ID was not found"))?;
    if scan.pages.is_empty()
        || scan.pages.len() > MAX_SCAN_PAGES
        || scan.target_sheet_ids.is_empty()
        || scan.target_sheet_ids.len() > plan_set::MAX_SHEETS
    {
        return Err(failure("scan selection exceeds limits"));
    }
    let mut pages = HashSet::new();
    if scan
        .pages
        .iter()
        .any(|p| *p == 0 || *p > file.page_count || !pages.insert(*p))
    {
        return Err(failure("invalid or duplicate source page"));
    }
    let mut sheets = HashSet::new();
    if scan
        .target_sheet_ids
        .iter()
        .any(|id| !sheets.insert(id) || !set.sheets.iter().any(|s| s.id == *id))
    {
        return Err(failure("invalid or duplicate target sheet"));
    }
    Ok(())
}
fn validate_review(review: &PlanLinkReview) -> Result<(), EngineError> {
    if review.version != 1
        || !valid_hash(&review.manifest_sha256)
        || !valid_hash(&review.source_sha256)
        || review.selections.is_empty()
        || review.selections.len() > MAX_OCCURRENCES
    {
        return Err(failure("invalid review version, hashes or selection count"));
    }
    let mut seen = HashSet::new();
    if review.selections.iter().any(|s| {
        !s.confirmed
            || !valid_hash(&s.fingerprint)
            || !seen.insert(&s.occurrence_id)
            || s.occurrence_id.len() > 128
            || s.target_sheet_id.len() > 128
    }) {
        return Err(failure(
            "review selections require unique IDs and explicit confirmation",
        ));
    }
    Ok(())
}
pub fn parse_review(bytes: &[u8]) -> Result<PlanLinkReview, EngineError> {
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(failure("review exceeds 256 KiB"));
    }
    let review =
        serde_json::from_slice(bytes).map_err(|e| failure(format!("invalid review: {e}")))?;
    validate_review(&review)?;
    Ok(review)
}
pub fn encode_review(review: &PlanLinkReview) -> Result<Vec<u8>, EngineError> {
    validate_review(review)?;
    let bytes = serde_json::to_vec_pretty(review).map_err(|e| failure(e.to_string()))?;
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(failure("review exceeds 256 KiB"));
    }
    Ok(bytes)
}
fn reparse(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
fn absolute(path: &Path) -> Result<PathBuf, EngineError> {
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    })
}
fn no_aliases(path: &Path) -> Result<(), EngineError> {
    let mut current = PathBuf::new();
    for c in path.components() {
        if matches!(c, Component::ParentDir) {
            return Err(failure("unresolved parent component"));
        }
        #[cfg(windows)]
        if let Component::Prefix(prefix) = c {
            if !matches!(
                prefix.kind(),
                std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
            ) {
                return Err(failure("local disk paths are required"));
            }
            current.push(c);
            continue;
        }
        current.push(c);
        if reparse(&current.symlink_metadata()?) {
            return Err(failure("symbolic links and reparse points are refused"));
        }
    }
    Ok(())
}
fn manifest_snapshot(path: &Path) -> Result<(PathBuf, Vec<u8>), EngineError> {
    let selected = absolute(path)?;
    no_aliases(&selected)?;
    let canonical = selected.canonicalize()?;
    if !canonical.metadata()?.is_file() {
        return Err(failure("choose a regular manifest"));
    }
    let file = crate::file_io::open_regular(&canonical)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > plan_set::MAX_MANIFEST_BYTES as u64 {
        return Err(failure("manifest exceeds 2 MiB"));
    }
    let mut bytes = Vec::new();
    (&file)
        .take(plan_set::MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    no_aliases(&selected)?;
    if bytes.len() as u64 != before.len()
        || bytes.len() > plan_set::MAX_MANIFEST_BYTES
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || selected.canonicalize()? != canonical
    {
        return Err(failure("manifest changed while reading"));
    }
    Ok((canonical, bytes))
}
#[derive(Clone)]
struct Binding {
    sheet_id: String,
    file_id: String,
    path: PathBuf,
    sha256: String,
}
struct Sources {
    selected_manifest: PathBuf,
    manifest: PathBuf,
    manifest_sha256: String,
    set: plan_set::PlanSet,
    bindings: Vec<Binding>,
}
impl Sources {
    fn load(path: &Path, set: &plan_set::PlanSet) -> Result<Self, EngineError> {
        let (manifest, bytes) = manifest_snapshot(path)?;
        if plan_set::parse(&bytes)? != *set {
            return Err(failure("manifest no longer matches the selected set"));
        }
        Ok(Self {
            selected_manifest: absolute(path)?,
            manifest,
            manifest_sha256: hash(&bytes),
            set: set.clone(),
            bindings: Vec::new(),
        })
    }
    fn read(&mut self, sheet_id: &str) -> Result<plan_set::VerifiedSheet, EngineError> {
        let verified = plan_set::read_sheet(&self.manifest, &self.set, sheet_id)?;
        if !self.bindings.iter().any(|b| b.path == verified.path) {
            self.bindings.push(Binding {
                sheet_id: sheet_id.to_owned(),
                file_id: self
                    .set
                    .sheets
                    .iter()
                    .find(|s| s.id == sheet_id)
                    .ok_or_else(|| failure("sheet missing"))?
                    .file_id
                    .clone(),
                path: verified.path.clone(),
                sha256: verified.sha256.clone(),
            });
        }
        Ok(verified)
    }
    fn file_path(&self, file_id: &str) -> Result<&Path, EngineError> {
        self.bindings
            .iter()
            .find(|b| b.file_id == file_id)
            .map(|b| b.path.as_path())
            .ok_or_else(|| failure("file was not inspected"))
    }
    fn revalidate(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        cancelled(cancel)?;
        let (path, bytes) = manifest_snapshot(&self.selected_manifest)?;
        if path != self.manifest || hash(&bytes) != self.manifest_sha256 {
            return Err(failure("manifest changed; inspect again"));
        }
        for binding in &self.bindings {
            cancelled(cancel)?;
            let actual = plan_set::read_sheet(&self.manifest, &self.set, &binding.sheet_id)?;
            if actual.path != binding.path || actual.sha256 != binding.sha256 {
                return Err(failure("source identity changed; inspect again"));
            }
        }
        Ok(())
    }
}
fn writable(doc: &Document<'_>) -> Result<(), EngineError> {
    let p = doc.properties();
    if !p.can_copy || !p.can_annotate {
        return Err(EngineError::SecurityRestricted);
    }
    let pdf = crate::security::load(&doc.snapshot()?, doc.password())?;
    if doc.signature_count() > 0
        || pdf.objects.values().any(|o| {
            o.as_dict().is_ok_and(|d| {
                d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Sig".as_slice())
                    || d.get(b"FT").and_then(Object::as_name).ok() == Some(b"Sig".as_slice())
            })
        })
    {
        return Err(failure("signed source PDFs are not eligible"));
    }
    Ok(())
}
fn rect_array(r: PointRect) -> [f32; 4] {
    [r.left, r.bottom, r.right, r.top]
}
fn rect_value(r: [f32; 4]) -> PointRect {
    PointRect {
        left: r[0],
        bottom: r[1],
        right: r[2],
        top: r[3],
    }
}
fn overlap(a: PointRect, b: PointRect) -> bool {
    a.left < b.right && b.left < a.right && a.bottom < b.top && b.bottom < a.top
}
fn valid_rect(r: PointRect, w: f32, h: f32) -> bool {
    [r.left, r.bottom, r.right, r.top, w, h]
        .into_iter()
        .all(f32::is_finite)
        && r.left >= 0.0
        && r.bottom >= 0.0
        && r.right <= w
        && r.top <= h
        && r.right > r.left
        && r.top > r.bottom
}
fn occurrence_rect(boxes: &[Option<PointRect>], w: f32, h: f32) -> Option<PointRect> {
    let first = boxes.first().copied().flatten()?;
    if !valid_rect(first, w, h) {
        return None;
    }
    let mut r = first;
    let mut previous = first;
    let last = boxes.last().copied().flatten()?;
    let vertical = ((last.top + last.bottom) - (first.top + first.bottom)).abs()
        > ((last.left + last.right) - (first.left + first.right)).abs();
    for current in boxes.iter().copied() {
        let current = current?;
        if !valid_rect(current, w, h) {
            return None;
        }
        let (axis_error, gap, allowed) = if vertical {
            (
                ((current.left + current.right) - (first.left + first.right)).abs() / 2.0,
                ((current.bottom + current.top) - (previous.bottom + previous.top)).abs() / 2.0,
                (current.top - current.bottom).max(previous.top - previous.bottom),
            )
        } else {
            (
                ((current.bottom + current.top) - (first.bottom + first.top)).abs() / 2.0,
                ((current.left + current.right) - (previous.left + previous.right)).abs() / 2.0,
                (current.right - current.left).max(previous.right - previous.left),
            )
        };
        let transverse = if vertical {
            first.right - first.left
        } else {
            first.top - first.bottom
        };
        if axis_error > (transverse * 0.35).max(1.0) || gap > allowed * 2.0 + 1.0 {
            return None;
        }
        r.left = r.left.min(current.left);
        r.right = r.right.max(current.right);
        r.bottom = r.bottom.min(current.bottom);
        r.top = r.top.max(current.top);
        previous = current;
    }
    r.left = (r.left - 0.25).max(0.0);
    r.bottom = (r.bottom - 0.25).max(0.0);
    r.right = (r.right + 0.25).min(w);
    r.top = (r.top + 0.25).min(h);
    (r.right - r.left >= 2.0 && r.top - r.bottom >= 2.0).then_some(r)
}
fn relative_between(parent: &Path, target: &Path) -> Result<String, EngineError> {
    let a: Vec<_> = parent.components().collect();
    let b: Vec<_> = target.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return Err(failure("remote destination is on another root"));
    }
    let mut parts = vec!["..".to_owned(); a.len() - common];
    for c in &b[common..] {
        let Component::Normal(value) = c else {
            return Err(failure("invalid remote path"));
        };
        parts.push(
            value
                .to_str()
                .ok_or_else(|| failure("remote path is not Unicode"))?
                .to_owned(),
        );
    }
    let value = parts.join("/");
    if !safe_relative(&value) {
        return Err(failure("remote path is not portable"));
    }
    Ok(value)
}

fn scan_internal(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    scan: &PlanLinkScanRequest,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<(PlanLinkInspection, Sources, plan_set::VerifiedSheet), EngineError> {
    scan_internal_bounded(
        engine,
        manifest,
        set,
        scan,
        credentials,
        cancel,
        MAX_OCCURRENCES,
    )
}

#[allow(clippy::too_many_arguments)]
fn scan_internal_bounded(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    scan: &PlanLinkScanRequest,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
    occurrence_budget: usize,
) -> Result<(PlanLinkInspection, Sources, plan_set::VerifiedSheet), EngineError> {
    validate_scan(scan, set)?;
    cancelled(cancel)?;
    let mut sources = Sources::load(manifest, set)?;
    let source_sheet = set
        .sheets
        .iter()
        .find(|s| s.file_id == scan.source_file_id)
        .ok_or_else(|| failure("source file has no indexed sheet"))?;
    let source = sources.read(&source_sheet.id)?;
    let doc = engine.open_bytes(
        source.bytes.clone(),
        credentials.get(&scan.source_file_id),
        &source.path,
    )?;
    writable(&doc)?;
    let source_file = set
        .files
        .iter()
        .find(|f| f.id == scan.source_file_id)
        .ok_or_else(|| failure("source file missing"))?;
    if doc.page_count() != source_file.page_count {
        return Err(failure("source page count mismatch"));
    }
    let mut report = PlanLinkInspection {
        version: 1,
        manifest_sha256: sources.manifest_sha256.clone(),
        source_sha256: source.sha256.clone(),
        source_file_id: scan.source_file_id.clone(),
        requested_pages: scan.pages.clone(),
        scanned_pages: 0,
        scan_complete: false,
        truncated: false,
        targets: Vec::new(),
        occurrences: Vec::new(),
        issues: Vec::new(),
    };
    let mut file_checks: HashMap<String, Result<usize, String>> = HashMap::new();
    for id in &scan.target_sheet_ids {
        cancelled(cancel)?;
        let sheet = set
            .sheets
            .iter()
            .find(|s| s.id == *id)
            .ok_or_else(|| failure("target disappeared"))?;
        let file = set
            .files
            .iter()
            .find(|f| f.id == sheet.file_id)
            .ok_or_else(|| failure("target file disappeared"))?;
        let mut target = PlanLinkTarget {
            sheet_id: id.clone(),
            file_id: file.id.clone(),
            number: sheet.number.clone(),
            title: sheet.title.clone(),
            discipline: sheet.discipline.clone(),
            revision: sheet.revision.clone(),
            page: sheet.page,
            sha256: file.sha256.clone(),
            byte_len: file.byte_len,
            status: "ready".into(),
        };
        let check = if !identifier(&sheet.number) {
            Err("unsupported_identifier".to_owned())
        } else if file.id == scan.source_file_id {
            Err("same_document_target".to_owned())
        } else if set
            .sheets
            .iter()
            .filter(|s| {
                (s.file_id == sheet.file_id && s.page == sheet.page)
                    || (s.number.trim().to_lowercase() == sheet.number.trim().to_lowercase()
                        && s.discipline.trim().to_lowercase()
                            == sheet.discipline.trim().to_lowercase()
                        && s.revision.trim().to_lowercase() == sheet.revision.trim().to_lowercase())
            })
            .count()
            != 1
        {
            Err("duplicate_reference".to_owned())
        } else {
            let result = file_checks.entry(file.id.clone()).or_insert_with(|| {
                let verified = sources
                    .read(id)
                    .map_err(|_| "unavailable_reference".to_owned())?;
                engine
                    .open_bytes(verified.bytes, credentials.get(&file.id), &verified.path)
                    .map_err(|_| "unreadable_or_protected".to_owned())
                    .and_then(|target_doc| {
                        if !target_doc.properties().can_copy {
                            return Err("copy_restricted".to_owned());
                        }
                        if target_doc.page_count() != file.page_count {
                            return Err("page_count_changed".to_owned());
                        }
                        Ok(target_doc.page_count())
                    })
            });
            result.clone().and_then(|n| {
                if sheet.page <= n {
                    Ok(())
                } else {
                    Err("page_out_of_range".to_owned())
                }
            })
        };
        if let Err(reason) = check {
            target.status = reason.clone();
            report.issues.push(PlanLinkIssue {
                page: None,
                sheet_id: Some(id.clone()),
                reason,
            });
        }
        report.targets.push(target);
    }
    let mut numbers: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, target) in report.targets.iter().enumerate() {
        if identifier(&target.number) {
            numbers
                .entry(target.number.clone())
                .or_default()
                .push(index);
        }
    }
    let mut retained_report_bytes = serde_json::to_vec(&report)
        .map_err(|e| failure(e.to_string()))?
        .len();
    if retained_report_bytes > MAX_REPORT_BYTES {
        return Err(failure("inspection exceeds 2 MiB"));
    }
    for &physical in &scan.pages {
        cancelled(cancel)?;
        let page = doc.page_handle(physical - 1)?;
        let geometry = doc.page_geometry(physical - 1)?;
        let size = doc.page_size(physical - 1)?;
        let text = page.text()?;
        if text.len() < 0 || text.len() as usize > MAX_PAGE_CHARS {
            report.issues.push(PlanLinkIssue {
                page: Some(physical),
                sheet_id: None,
                reason: "page_character_limit".into(),
            });
            continue;
        }
        let chars = text.chars();
        let mut values = Vec::with_capacity(chars.len());
        let mut boxes = Vec::with_capacity(chars.len());
        for (index, ch) in chars.iter().enumerate() {
            if index % 1024 == 0 {
                cancelled(cancel)?;
            }
            values.push(ch.unicode_char().unwrap_or('\0'));
            boxes.push(ch.loose_bounds().ok().map(|r| {
                geometry.rect_to_display(PointRect {
                    left: r.left().value,
                    bottom: r.bottom().value,
                    right: r.right().value,
                    top: r.top().value,
                })
            }));
        }
        if values.iter().all(|c| c.is_whitespace() || *c == '\0') {
            report.issues.push(PlanLinkIssue {
                page: Some(physical),
                sheet_id: None,
                reason: "no_searchable_text".into(),
            });
        }
        let existing = doc.page_links(physical - 1)?;
        let mut index = 0;
        while index < values.len() {
            if index % 1024 == 0 {
                cancelled(cancel)?;
            }
            if !identifier_char(values[index]) {
                index += 1;
                continue;
            }
            let start = index;
            while index < values.len() && identifier_char(values[index]) {
                index += 1;
            }
            if index - start > 64
                || start
                    .checked_sub(1)
                    .is_some_and(|i| values[i].is_alphanumeric())
                || values.get(index).is_some_and(|c| c.is_alphanumeric())
            {
                continue;
            }
            let token: String = values[start..index].iter().collect();
            let Some(targets) = numbers.get(&token) else {
                continue;
            };
            if report.occurrences.len() == occurrence_budget.min(MAX_OCCURRENCES) {
                report.truncated = true;
                break;
            }
            let rect = occurrence_rect(&boxes[start..index], size.width_pt, size.height_pt);
            let available = targets
                .iter()
                .filter(|&&i| report.targets[i].status == "ready")
                .count();
            let status = if rect.is_none() {
                "unsupported_geometry"
            } else if existing
                .iter()
                .any(|link| overlap(link.rect, rect.unwrap()))
            {
                "existing_link"
            } else if available == 0 {
                "unavailable_target"
            } else if targets.len() != 1 {
                "ambiguous"
            } else {
                "ready"
            };
            let target_ids: Vec<_> = targets
                .iter()
                .map(|&i| report.targets[i].sheet_id.clone())
                .collect();
            let rect = rect.map(rect_array);
            let fingerprint = hash(
                &serde_json::to_vec(&serde_json::json!([
                    source.sha256,
                    physical,
                    start,
                    index,
                    token,
                    rect,
                    target_ids,
                    targets
                        .iter()
                        .map(|&i| (&report.targets[i].sha256, &report.targets[i].status))
                        .collect::<Vec<_>>()
                ]))
                .map_err(|e| failure(e.to_string()))?,
            );
            let occurrence = PlanLinkOccurrence {
                id: format!("p{physical}-c{start}-{index}"),
                fingerprint,
                page: physical,
                first_character: start,
                end_character: index,
                text: token,
                rect,
                target_sheet_ids: target_ids,
                status: status.into(),
            };
            retained_report_bytes = retained_report_bytes.saturating_add(
                serde_json::to_vec(&occurrence)
                    .map_err(|e| failure(e.to_string()))?
                    .len()
                    + 1,
            );
            if retained_report_bytes > MAX_REPORT_BYTES {
                return Err(failure("inspection exceeds 2 MiB"));
            }
            report.occurrences.push(occurrence);
        }
        report.scanned_pages += 1;
        if report.truncated {
            break;
        }
    }
    report.scan_complete = report.scanned_pages == scan.pages.len() && !report.truncated;
    if serde_json::to_vec(&report)
        .map_err(|e| failure(e.to_string()))?
        .len()
        > MAX_REPORT_BYTES
    {
        return Err(failure("inspection exceeds 2 MiB"));
    }
    sources.revalidate(cancel)?;
    Ok((report, sources, source))
}
pub fn inspect_plan_links(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    scan: &PlanLinkScanRequest,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<PlanLinkInspection, EngineError> {
    scan_internal(engine, manifest, set, scan, credentials, cancel).map(|(report, _, _)| report)
}

/// Prepared PDF bytes never leave the engine worker except through a new-copy commit.
pub struct PreparedPlanLinks {
    bytes: Vec<u8>,
    sources: Sources,
    source_path: PathBuf,
    report: PlanLinkReport,
}
impl PreparedPlanLinks {
    pub fn report(&self) -> &PlanLinkReport {
        &self.report
    }
    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        self.sources.revalidate(cancel)
    }
    pub fn render_preview(
        &self,
        engine: &Engine,
        page: usize,
        password: Option<&str>,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        if !(64..=MAX_PREVIEW_SIDE).contains(&long_side)
            || page == 0
            || !self.report.created.iter().any(|c| c.page == page)
        {
            return Err(failure("select a linked page and a bounded preview size"));
        }
        self.revalidate_sources(cancel)?;
        let doc = engine.open_bytes(self.bytes.clone(), password, &self.source_path)?;
        if !doc.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let size = doc.page_size(page - 1)?;
        let image = doc.render_page(
            page - 1,
            long_side as f32 / size.width_pt.max(size.height_pt),
        )?;
        if image.rgba.len() > MAX_PREVIEW_SIDE as usize * MAX_PREVIEW_SIDE as usize * 4 {
            return Err(failure("preview pixel limit"));
        }
        self.revalidate_sources(cancel)?;
        Ok(image)
    }
    /// Renders only a destination already included in this reviewed candidate.
    pub fn render_target_preview(
        &self,
        engine: &Engine,
        sheet_id: &str,
        password: Option<&str>,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        if !(64..=MAX_PREVIEW_SIDE).contains(&long_side) {
            return Err(failure("select a bounded target preview size"));
        }
        let target = self
            .report
            .created
            .iter()
            .find(|link| link.target.sheet_id == sheet_id)
            .map(|link| &link.target)
            .ok_or_else(|| failure("destination is not part of this reviewed candidate"))?;
        let sheet = self
            .sources
            .set
            .sheets
            .iter()
            .find(|sheet| sheet.id == sheet_id)
            .ok_or_else(|| failure("destination sheet is unavailable"))?;
        let passwords: Vec<_> = password
            .into_iter()
            .map(|value| (sheet.file_id.as_str(), value))
            .collect();
        self.revalidate_sources(cancel)?;
        let verified = resolve_verified_remote_link(
            engine,
            &self.sources.manifest,
            &self.sources.set,
            &self.source_path,
            target,
            &PlanLinkCredentials {
                passwords: &passwords,
            },
            cancel,
        )?;
        let doc = engine.open_bytes(verified.bytes, password, &verified.path)?;
        let size = doc.page_size(verified.page - 1)?;
        let image = doc.render_page(
            verified.page - 1,
            long_side as f32 / size.width_pt.max(size.height_pt),
        )?;
        if image.rgba.len() > MAX_PREVIEW_SIDE as usize * MAX_PREVIEW_SIDE as usize * 4 {
            return Err(failure("preview pixel limit"));
        }
        self.revalidate_sources(cancel)?;
        Ok(image)
    }
}
pub fn prepare_plan_links(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    review: &PlanLinkReview,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<PreparedPlanLinks, EngineError> {
    prepare_plan_links_bounded(
        engine,
        manifest,
        set,
        review,
        credentials,
        cancel,
        MAX_OCCURRENCES,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_plan_links_bounded(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    review: &PlanLinkReview,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
    occurrence_budget: usize,
) -> Result<PreparedPlanLinks, EngineError> {
    validate_review(review)?;
    let (inspection, sources, source) = scan_internal_bounded(
        engine,
        manifest,
        set,
        &review.scan,
        credentials,
        cancel,
        occurrence_budget,
    )?;
    if review.manifest_sha256 != inspection.manifest_sha256
        || review.source_sha256 != inspection.source_sha256
    {
        return Err(failure("review sources changed; inspect again"));
    }
    let mut doc = engine.open_bytes(
        source.bytes,
        credentials.get(&review.scan.source_file_id),
        &source.path,
    )?;
    writable(&doc)?;
    let parent = source
        .path
        .parent()
        .ok_or_else(|| failure("source has no parent"))?;
    let mut pending = BTreeMap::<usize, Vec<_>>::new();
    for selection in &review.selections {
        cancelled(cancel)?;
        let occurrence = inspection
            .occurrences
            .iter()
            .find(|v| v.id == selection.occurrence_id)
            .ok_or_else(|| failure("occurrence no longer exists"))?;
        if occurrence.fingerprint != selection.fingerprint
            || !matches!(occurrence.status.as_str(), "ready" | "ambiguous")
            || !occurrence
                .target_sheet_ids
                .contains(&selection.target_sheet_id)
        {
            return Err(failure("stale, unsupported or unreviewed occurrence"));
        }
        let target = inspection
            .targets
            .iter()
            .find(|v| v.sheet_id == selection.target_sheet_id && v.status == "ready")
            .ok_or_else(|| failure("destination is unavailable"))?;
        let target = RemotePageTarget {
            relative_path: relative_between(parent, sources.file_path(&target.file_id)?)?,
            page: target.page - 1,
            sheet_id: target.sheet_id.clone(),
            sha256: target.sha256.clone(),
            byte_len: target.byte_len,
        };
        let rect = occurrence
            .rect
            .ok_or_else(|| failure("occurrence geometry is unavailable"))?;
        let geo = doc.page_geometry(occurrence.page - 1)?;
        let restored = geo.rect_to_display(geo.rect_to_user(rect_value(rect)));
        if rect_array(restored)
            .iter()
            .zip(rect)
            .any(|(a, b)| (a - b).abs() > 0.001)
        {
            return Err(failure("link rectangle loses storage precision"));
        }
        pending
            .entry(occurrence.page)
            .or_default()
            .push((occurrence, rect, target));
    }
    let mut created = Vec::with_capacity(review.selections.len());
    for (page, pending) in pending {
        cancelled(cancel)?;
        let links: Vec<_> = pending
            .iter()
            .map(|(_, rect, target)| (rect_value(*rect), target.clone()))
            .collect();
        let indices = doc.add_plan_link_batch(page - 1, &links, cancel)?;
        let mut details: HashMap<_, _> = doc
            .annotation_details(page - 1)?
            .into_iter()
            .map(|d| (d.index, d))
            .collect();
        for ((occurrence, rect, target), index) in pending.into_iter().zip(indices) {
            let detail = details
                .remove(&index)
                .ok_or_else(|| failure("created link missing"))?;
            if detail.link != Some(LinkTarget::RemotePage(target.clone()))
                || rect_array(detail.rect)
                    .iter()
                    .zip(rect)
                    .any(|(a, b)| (a - b).abs() > 0.001)
            {
                return Err(failure("created link failed readback"));
            }
            let name = detail
                .name
                .ok_or_else(|| failure("created link has no identity"))?;
            created.push(CreatedPlanLink {
                occurrence_id: occurrence.id.clone(),
                page,
                annotation_index: index,
                name,
                rect,
                target,
            });
        }
        if doc.snapshot()?.len() > MAX_CANDIDATE_BYTES {
            return Err(failure("prepared PDF exceeds 160 MiB"));
        }
    }
    let bytes = doc.snapshot()?;
    if bytes.len() > MAX_CANDIDATE_BYTES {
        return Err(failure("prepared PDF exceeds 160 MiB"));
    }
    let selected: HashSet<_> = review
        .selections
        .iter()
        .map(|s| s.occurrence_id.as_str())
        .collect();
    let excluded_occurrences = inspection
        .occurrences
        .iter()
        .filter(|v| !selected.contains(v.id.as_str()))
        .map(|v| v.id.clone())
        .collect();
    let report = PlanLinkReport {
        version: 1,
        inspection,
        created,
        excluded_occurrences,
        output_sha256: hash(&bytes),
    };
    if serde_json::to_vec(&report)
        .map_err(|e| failure(e.to_string()))?
        .len()
        > MAX_REPORT_BYTES
    {
        return Err(failure("report exceeds 2 MiB"));
    }
    sources.revalidate(cancel)?;
    Ok(PreparedPlanLinks {
        bytes,
        sources,
        source_path: source.path,
        report,
    })
}
pub fn publish_plan_links(
    prepared: PreparedPlanLinks,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<PlanLinkReceipt, EngineError> {
    publish_plan_links_checked(prepared, output, cancel, || Ok(()))
}

fn publish_plan_links_checked(
    prepared: PreparedPlanLinks,
    output: &Path,
    cancel: &AtomicBool,
    before_commit: impl FnOnce() -> Result<(), EngineError>,
) -> Result<PlanLinkReceipt, EngineError> {
    prepared.revalidate_sources(cancel)?;
    let output = absolute(output)?;
    let parent = output
        .parent()
        .ok_or_else(|| failure("output has no parent"))?;
    no_aliases(parent)?;
    let parent = parent.canonicalize()?;
    if Some(parent.as_path()) != prepared.source_path.parent() {
        return Err(failure("new copy must stay in the source PDF directory"));
    }
    let name = output
        .file_name()
        .ok_or_else(|| failure("output has no filename"))?;
    if name.to_str().is_none_or(|name| !safe_relative(name)) {
        return Err(failure("choose a portable new PDF filename"));
    }
    let output = parent.join(name);
    if output
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("pdf"))
        || output.symlink_metadata().is_ok()
    {
        return Err(failure("choose a new PDF filename"));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(&parent)?;
    for chunk in prepared.bytes.chunks(1024 * 1024) {
        cancelled(cancel)?;
        temporary.write_all(chunk)?;
    }
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    prepared.revalidate_sources(cancel)?;
    before_commit()?;
    cancelled(cancel)?;
    // A collision appearing after the earlier check is refused at the actual commit.
    temporary
        .persist_noclobber(&output)
        .map_err(|e| EngineError::Io(e.error))?;
    Ok(PlanLinkReceipt {
        output,
        bytes: prepared.bytes.len(),
        report: prepared.report,
    })
}

/// Validates against a user-selected manifest; never opens an external program.
/// The caller must obtain confirmation before presenting the returned bytes in a tab.
pub fn resolve_verified_remote_link(
    engine: &Engine,
    manifest: &Path,
    set: &plan_set::PlanSet,
    source_pdf: &Path,
    target: &RemotePageTarget,
    credentials: &PlanLinkCredentials<'_>,
    cancel: &AtomicBool,
) -> Result<plan_set::VerifiedSheet, EngineError> {
    target.validate()?;
    let mut sources = Sources::load(manifest, set)?;
    cancelled(cancel)?;
    let source = absolute(source_pdf)?;
    no_aliases(&source)?;
    let source = source.canonicalize()?;
    if !source.is_file()
        || !source.starts_with(
            sources
                .manifest
                .parent()
                .ok_or_else(|| failure("manifest parent missing"))?,
        )
    {
        return Err(failure("source copy is outside the selected plan root"));
    }
    let sheet = set
        .sheets
        .iter()
        .find(|s| s.id == target.sheet_id)
        .ok_or_else(|| failure("target sheet not indexed"))?;
    let file = set
        .files
        .iter()
        .find(|f| f.id == sheet.file_id)
        .ok_or_else(|| failure("target file not indexed"))?;
    if sheet.page != target.page + 1
        || file.byte_len != target.byte_len
        || !file.sha256.eq_ignore_ascii_case(&target.sha256)
    {
        return Err(failure(
            "linked destination has changed; review the link again",
        ));
    }
    let verified = sources.read(&target.sheet_id)?;
    if relative_between(
        source
            .parent()
            .ok_or_else(|| failure("source parent missing"))?,
        &verified.path,
    )? != target.relative_path
    {
        return Err(failure("remote path does not match the selected sheet"));
    }
    let doc = engine.open_bytes(
        verified.bytes.clone(),
        credentials.get(&file.id),
        &verified.path,
    )?;
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    if doc.page_count() != file.page_count || verified.page > doc.page_count() {
        return Err(failure("remote physical page is unavailable"));
    }
    sources.revalidate(cancel)?;
    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_actions_are_bounded_inert_and_exact() {
        let target = RemotePageTarget {
            relative_path: "../plans/Étage.pdf".into(),
            page: 1,
            sheet_id: "sheet-1".into(),
            sha256: "ab".repeat(32),
            byte_len: 100,
        };
        let action = remote_action(&target).unwrap();
        let action = action.as_dict().unwrap();
        assert_eq!(parse_remote_action(action).unwrap(), target);
        let mut forged = action.clone();
        forged.set("Next", Dictionary::new());
        assert!(parse_remote_action(&forged).is_err());
        let mut forged = action.clone();
        forged.set("D", vec![Object::Integer(2), Object::Name(b"Fit".to_vec())]);
        assert!(parse_remote_action(&forged).is_err());
        for bad in [
            "file:///a.pdf",
            "C:/a.pdf",
            "//host/a.pdf",
            "a/../b.pdf",
            "CON.pdf",
            "../NUL.pdf",
            "a.pdf\n",
        ] {
            let mut value = target.clone();
            value.relative_path = bad.into();
            assert!(remote_action(&value).is_err(), "{bad}");
        }
    }
    #[test]
    fn rectangle_continuity_and_review_bounds_are_explicit() {
        let a = Some(PointRect {
            left: 20.0,
            bottom: 20.0,
            right: 26.0,
            top: 30.0,
        });
        let b = Some(PointRect {
            left: 26.0,
            bottom: 20.0,
            right: 32.0,
            top: 30.0,
        });
        assert!(occurrence_rect(&[a, b], 100.0, 100.0).is_some());
        assert!(occurrence_rect(&[a, None], 100.0, 100.0).is_none());
        assert!(occurrence_rect(
            &[
                a,
                Some(PointRect {
                    left: 80.0,
                    bottom: 20.0,
                    right: 86.0,
                    top: 30.0
                })
            ],
            100.0,
            100.0
        )
        .is_none());
        assert!(parse_review(&vec![b' '; MAX_RECIPE_BYTES + 1]).is_err());
        assert!(parse_review(br#"{"version":2}"#).is_err());
    }
}
