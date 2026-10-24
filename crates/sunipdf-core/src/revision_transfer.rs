//! Explicit, bounded transfer of supported native annotations between two saved PDF revisions.
//! Recipes contain identities and selections only. Prepared bytes are opaque and never trusted
//! when supplied by a caller; publication rechecks both input files and creates one new copy.

use crate::annotation_library::{
    LibraryEntry, LibraryPlacement, LibraryPrimitive, LibraryProvenance, LibraryShape,
    LibraryStyle, OriginalCapture,
};
use crate::geometry::PageGeometry;
use crate::local_review::digest;
use crate::plan_set::PlanSheet;
use crate::revision_overlay::OverlayTransform;
use crate::{Document, Engine, EngineError, RenderedPage};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_CANDIDATE_BYTES: usize = 160 * 1024 * 1024;
pub const MAX_RECIPE_BYTES: usize = 256 * 1024;
pub const MAX_PAGE_PAIRS: usize = 32;
pub const MAX_SELECTIONS_PER_PAGE: usize = 16;
pub const MAX_SELECTIONS: usize = 256;
pub const MAX_PREVIEW_SIDE: u32 = 1536;
const POSITION_TOLERANCE: f64 = 0.001;
const TRACE_KEY: &[u8] = b"SuniPDFRevisionTransfers";

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::Other(format!("Revision transfer: {}", message.into()))
}

fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

fn hash_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn sheet_id_valid(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|v| {
        !v.is_empty()
            && v.len() <= 128
            && v.bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    })
}

/// Credentials are borrowed for one operation, never stored in a recipe, report or candidate.
#[derive(Clone, Copy, Default)]
pub struct TransferPasswords<'a> {
    pub old: Option<&'a str>,
    pub new: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransferAlignment {
    Identity,
    /// Correspondences in displayed points, with the origin at the top-left corner.
    Anchors {
        old_points: [[f64; 2]; 2],
        new_points: [[f64; 2]; 2],
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferSelection {
    /// Zero-based annotation index in the old page's /Annots array.
    pub index: usize,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferPagePair {
    /// Physical, one-based page numbers. No label or title is an address.
    pub old_page: usize,
    pub new_page: usize,
    pub old_sheet_id: Option<String>,
    pub new_sheet_id: Option<String>,
    pub confirmed: bool,
    pub alignment: TransferAlignment,
    pub selections: Vec<TransferSelection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionTransferRequest {
    pub version: u32,
    pub old_sha256: String,
    pub new_sha256: String,
    pub pairs: Vec<TransferPagePair>,
    pub acknowledge_repeated_transfer: bool,
}

fn validate_request(request: &RevisionTransferRequest) -> Result<(), EngineError> {
    if request.version != 1 || !hash_valid(&request.old_sha256) || !hash_valid(&request.new_sha256)
    {
        return Err(invalid(
            "recipe requires version 1 and lowercase SHA-256 source identities",
        ));
    }
    let pairs: Vec<_> = request
        .pairs
        .iter()
        .map(|p| (p.old_page, p.new_page))
        .collect();
    validate_pairs(&pairs)?;
    let mut total = 0;
    for pair in &request.pairs {
        if !sheet_id_valid(&pair.old_sheet_id) || !sheet_id_valid(&pair.new_sheet_id) {
            return Err(invalid("invalid sheet identity"));
        }
        if pair.selections.len() > MAX_SELECTIONS_PER_PAGE {
            return Err(invalid("select at most 16 annotations per page"));
        }
        let mut seen = HashSet::new();
        for selection in &pair.selections {
            if selection.index >= 512
                || !hash_valid(&selection.fingerprint)
                || !seen.insert(selection.index)
            {
                return Err(invalid("invalid or duplicate annotation selection"));
            }
        }
        if let TransferAlignment::Anchors {
            old_points,
            new_points,
        } = pair.alignment
        {
            OverlayTransform::from_anchors(new_points, old_points)?;
        }
        total += pair.selections.len();
    }
    if total > MAX_SELECTIONS {
        return Err(invalid("select at most 256 annotations"));
    }
    Ok(())
}

pub fn parse_request(bytes: &[u8]) -> Result<RevisionTransferRequest, EngineError> {
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(invalid("recipe exceeds 256 KiB"));
    }
    let request =
        serde_json::from_slice(bytes).map_err(|e| invalid(format!("invalid recipe: {e}")))?;
    validate_request(&request)?;
    Ok(request)
}

pub fn encode_request(request: &RevisionTransferRequest) -> Result<Vec<u8>, EngineError> {
    validate_request(request)?;
    let bytes = serde_json::to_vec_pretty(request).map_err(|e| invalid(e.to_string()))?;
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(invalid("recipe exceeds 256 KiB"));
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferReadiness {
    Unmatched,
    Ambiguous,
    NeedsReview,
    Excluded,
    Ready,
    Refused,
}

#[derive(Debug, Clone, Serialize)]
pub struct SheetRevisionPair {
    pub old_sheet_id: String,
    pub new_sheet_id: Option<String>,
    pub readiness: TransferReadiness,
}

/// Suggests only unique discipline/number matches; titles and revision labels do not infer order.
pub fn suggest_sheet_pairs(
    old: &[PlanSheet],
    new: &[PlanSheet],
) -> Result<Vec<SheetRevisionPair>, EngineError> {
    if old.len() > 4096 || new.len() > 4096 {
        return Err(invalid("too many sheets for matching"));
    }
    let key = |s: &PlanSheet| {
        (
            s.discipline.trim().to_lowercase(),
            s.number.trim().to_lowercase(),
        )
    };
    let mut old_counts = HashMap::new();
    let mut new_matches: HashMap<_, Vec<&PlanSheet>> = HashMap::new();
    for s in old.iter().chain(new) {
        if s.discipline.len() > 128 || s.number.len() > 128 || !sheet_id_valid(&Some(s.id.clone()))
        {
            return Err(invalid("invalid sheet metadata"));
        }
    }
    for s in old {
        *old_counts.entry(key(s)).or_insert(0) += 1;
    }
    for s in new {
        new_matches.entry(key(s)).or_default().push(s);
    }
    Ok(old
        .iter()
        .map(|s| {
            let k = key(s);
            let matches = new_matches.get(&k);
            let readiness = if k.1.is_empty() || matches.is_none() {
                TransferReadiness::Unmatched
            } else if old_counts[&k] != 1 || matches.is_some_and(|v| v.len() != 1) {
                TransferReadiness::Ambiguous
            } else {
                TransferReadiness::NeedsReview
            };
            SheetRevisionPair {
                old_sheet_id: s.id.clone(),
                new_sheet_id: (readiness == TransferReadiness::NeedsReview)
                    .then(|| matches.expect("unique match")[0].id.clone()),
                readiness,
            }
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TransferGeometry {
    pub width_pt: f32,
    pub height_pt: f32,
    pub crop_box_user_pt: [f32; 4],
    pub rotation_degrees: u16,
}

impl From<PageGeometry> for TransferGeometry {
    fn from(g: PageGeometry) -> Self {
        let (width_pt, height_pt) = g.display_size();
        Self {
            width_pt,
            height_pt,
            crop_box_user_pt: [g.bbox.left, g.bbox.bottom, g.bbox.right, g.bbox.top],
            rotation_degrees: g.rotation.degrees(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferCandidate {
    pub index: usize,
    pub subtype: String,
    pub fingerprint: Option<String>,
    pub source_nm: Option<String>,
    pub author: Option<String>,
    pub refusal: Option<String>,
}

impl From<&OriginalCapture> for TransferCandidate {
    fn from(c: &OriginalCapture) -> Self {
        Self {
            index: c.index,
            subtype: c.subtype.clone(),
            fingerprint: c.fingerprint.clone(),
            source_nm: c.source_nm.clone(),
            author: c.author.clone(),
            refusal: c.refusal.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferPageInspection {
    pub old_page: usize,
    pub new_page: usize,
    pub old_geometry: TransferGeometry,
    pub new_geometry: TransferGeometry,
    pub candidates: Vec<TransferCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferInspection {
    pub old_sha256: String,
    pub new_sha256: String,
    pub old_page_count: usize,
    pub new_page_count: usize,
    pub pages: Vec<TransferPageInspection>,
    pub warnings: Vec<String>,
}

impl TransferInspection {
    pub fn complete(&self) -> bool {
        self.pages.iter().any(|p| {
            p.candidates
                .iter()
                .any(|c| c.fingerprint.is_some() && c.refusal.is_none())
        })
    }
    pub fn json(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("finite inspection");
        value["complete"] = self.complete().into();
        value
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferPageReport {
    pub old_page: usize,
    pub new_page: usize,
    pub old_geometry: TransferGeometry,
    pub new_geometry: TransferGeometry,
    pub transform: OverlayTransform,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferredAnnotation {
    pub old_page: usize,
    pub new_page: usize,
    pub source_index: usize,
    pub target_index: usize,
    pub source_nm: String,
    pub target_nm: String,
    pub source_fingerprint: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferExclusion {
    pub old_page: usize,
    pub index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionTransferReport {
    pub old_sha256: String,
    pub new_sha256: String,
    pub output_path: Option<PathBuf>,
    pub output_sha256: Option<String>,
    pub pages: Vec<TransferPageReport>,
    pub transferred: Vec<TransferredAnnotation>,
    pub exclusions: Vec<TransferExclusion>,
    pub warnings: Vec<String>,
}

impl RevisionTransferReport {
    pub fn complete(&self) -> bool {
        !self.transferred.is_empty()
    }
    pub fn json(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("finite transfer report");
        value["complete"] = self.complete().into();
        value
    }
}

/// The candidate cannot be constructed or deserialized by a caller and contains no credentials.
pub struct PreparedRevisionTransfer {
    old_path: PathBuf,
    new_path: PathBuf,
    old_selection_path: PathBuf,
    new_selection_path: PathBuf,
    bytes: Vec<u8>,
    report: RevisionTransferReport,
}

impl std::fmt::Debug for PreparedRevisionTransfer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedRevisionTransfer")
            .field("byte_len", &self.bytes.len())
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

impl PreparedRevisionTransfer {
    pub fn report(&self) -> &RevisionTransferReport {
        &self.report
    }

    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        for (selection, path, expected) in [
            (
                &self.old_selection_path,
                &self.old_path,
                &self.report.old_sha256,
            ),
            (
                &self.new_selection_path,
                &self.new_path,
                &self.report.new_sha256,
            ),
        ] {
            if selection.canonicalize()? != *path {
                return Err(invalid(
                    "selected source path changed after preview; prepare again",
                ));
            }
            if digest(&read_source(path, cancel)?) != *expected {
                return Err(invalid(
                    "source revision changed after preview; prepare again",
                ));
            }
            if selection.canonicalize()? != *path {
                return Err(invalid(
                    "selected source path changed while reading; prepare again",
                ));
            }
        }
        cancelled(cancel)
    }

    /// Renders only an existing mapped target page. No raster is retained in the candidate.
    pub fn render_preview(
        &self,
        engine: &Engine,
        page: usize,
        target_password: Option<&str>,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        self.revalidate_sources(cancel)?;
        if !(64..=MAX_PREVIEW_SIDE).contains(&long_side)
            || !self.report.pages.iter().any(|p| p.new_page == page)
        {
            return Err(invalid(
                "preview requires a mapped target page and a long side from 64 to 1536 pixels",
            ));
        }
        let doc = engine.open_bytes(self.bytes.clone(), target_password, &self.new_path)?;
        if !doc.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let size = doc.page_size(page - 1)?;
        let rendered = doc.render_page(
            page - 1,
            long_side as f32 / size.width_pt.max(size.height_pt),
        )?;
        if rendered.rgba.len() > MAX_PREVIEW_SIDE as usize * MAX_PREVIEW_SIDE as usize * 4 {
            return Err(EngineError::RenderTooLarge);
        }
        self.revalidate_sources(cancel)?;
        Ok(rendered)
    }
}

fn validate_pairs(pairs: &[(usize, usize)]) -> Result<(), EngineError> {
    if pairs.is_empty() || pairs.len() > MAX_PAGE_PAIRS {
        return Err(invalid("map 1 to 32 page pairs"));
    }
    let mut old = HashSet::new();
    let mut new = HashSet::new();
    if pairs
        .iter()
        .any(|&(a, b)| a == 0 || b == 0 || !old.insert(a) || !new.insert(b))
    {
        return Err(invalid(
            "page addresses must be positive and unique on both sides",
        ));
    }
    Ok(())
}

pub(crate) fn read_source(path: &Path, cancel: &AtomicBool) -> Result<Vec<u8>, EngineError> {
    cancelled(cancel)?;
    let before = path.metadata()?;
    if !before.is_file() || before.len() == 0 || before.len() > MAX_SOURCE_BYTES {
        return Err(invalid("choose a regular PDF no larger than 64 MiB"));
    }
    let file = crate::file_io::open_regular(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_SOURCE_BYTES {
        return Err(invalid("source is not a bounded regular file"));
    }
    let mut bytes = Vec::new();
    (&file).take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != metadata.len()
        || after.len() != metadata.len()
        || after.modified().ok() != metadata.modified().ok()
    {
        return Err(invalid("source changed while reading"));
    }
    cancelled(cancel)?;
    Ok(bytes)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferTrace {
    old_sha256: String,
    annotations: usize,
}

fn traces(pdf: &lopdf::Document) -> Result<Vec<TransferTrace>, EngineError> {
    let catalog = pdf.catalog().map_err(crate::security::lopdf_err)?;
    let Ok(object) = catalog.get(TRACE_KEY) else {
        return Ok(Vec::new());
    };
    let bytes = object
        .as_str()
        .map_err(|_| invalid("unsupported existing transfer history"))?;
    if bytes.len() > 16 * 1024 {
        return Err(invalid("existing transfer history is too large"));
    }
    let result: Vec<TransferTrace> =
        serde_json::from_slice(bytes).map_err(|_| invalid("invalid existing transfer history"))?;
    if result.len() > 64
        || result
            .iter()
            .any(|v| !hash_valid(&v.old_sha256) || !(1..=MAX_SELECTIONS).contains(&v.annotations))
    {
        return Err(invalid(
            "existing transfer history exceeds supported limits",
        ));
    }
    Ok(result)
}

fn check_documents(
    old: &Document<'_>,
    new: &Document<'_>,
    raw_new: &[u8],
) -> Result<Vec<TransferTrace>, EngineError> {
    if !old.properties().can_copy || !new.properties().can_copy || !new.properties().can_annotate {
        return Err(EngineError::SecurityRestricted);
    }
    let pdf = crate::security::load(raw_new, new.password())?;
    let signed_object = pdf.objects.values().any(|object| {
        object.as_dict().is_ok_and(|d| {
            d.get(b"Type").and_then(lopdf::Object::as_name).ok() == Some(b"Sig".as_slice())
                || (d.get(b"FT").and_then(lopdf::Object::as_name).ok() == Some(b"Sig".as_slice())
                    && d.get(b"V").is_ok_and(|v| !matches!(v, lopdf::Object::Null)))
        })
    });
    if new.signature_count() > 0 || signed_object {
        return Err(invalid(
            "signed target documents cannot be used for annotation transfer",
        ));
    }
    traces(&pdf)
}

fn geometry(doc: &Document<'_>, page: usize) -> Result<PageGeometry, EngineError> {
    if page == 0 || page > doc.page_count() {
        return Err(EngineError::PageOutOfRange(page.saturating_sub(1)));
    }
    let geo = doc.page_geometry(page - 1)?;
    let (w, h) = geo.display_size();
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(invalid("invalid page geometry"));
    }
    Ok(geo)
}

pub fn inspect_transfer_candidates(
    engine: &Engine,
    old_path: &Path,
    new_path: &Path,
    passwords: TransferPasswords<'_>,
    page_pairs: &[(usize, usize)],
    cancel: &AtomicBool,
) -> Result<TransferInspection, EngineError> {
    validate_pairs(page_pairs)?;
    let old_path = old_path.canonicalize()?;
    let new_path = new_path.canonicalize()?;
    let old_bytes = read_source(&old_path, cancel)?;
    let new_bytes = read_source(&new_path, cancel)?;
    let old_sha256 = digest(&old_bytes);
    let new_sha256 = digest(&new_bytes);
    let old = engine.open_bytes(old_bytes, passwords.old, &old_path)?;
    let new = engine.open_bytes(new_bytes.clone(), passwords.new, &new_path)?;
    let history = check_documents(&old, &new, &new_bytes)?;
    let mut pages = Vec::new();
    for &(old_page, new_page) in page_pairs {
        cancelled(cancel)?;
        let old_geometry = geometry(&old, old_page)?.into();
        let new_geometry = geometry(&new, new_page)?.into();
        let candidates = old
            .capture_original_annotations(old_page - 1)?
            .iter()
            .map(TransferCandidate::from)
            .collect();
        pages.push(TransferPageInspection {
            old_page,
            new_page,
            old_geometry,
            new_geometry,
            candidates,
        });
    }
    let warnings = if history.iter().any(|v| v.old_sha256 == old_sha256) {
        vec!["This source revision has already been transferred to the target; repeating can create duplicates and requires explicit acknowledgement.".into()]
    } else {
        Vec::new()
    };
    Ok(TransferInspection {
        old_sha256,
        new_sha256,
        old_page_count: old.page_count(),
        new_page_count: new.page_count(),
        pages,
        warnings,
    })
}

fn resolved_transform(
    alignment: TransferAlignment,
    old: PageGeometry,
    new: PageGeometry,
) -> Result<OverlayTransform, EngineError> {
    let mut transform = match alignment {
        TransferAlignment::Identity => OverlayTransform::default(),
        TransferAlignment::Anchors {
            old_points,
            new_points,
        } => {
            for (points, g) in [(old_points, old), (new_points, new)] {
                let (w, h) = g.display_size();
                if points.iter().any(|p| {
                    !p[0].is_finite()
                        || !p[1].is_finite()
                        || p[0] < 0.0
                        || p[1] < 0.0
                        || p[0] > f64::from(w)
                        || p[1] > f64::from(h)
                }) {
                    return Err(invalid("anchors must be inside their displayed page"));
                }
            }
            OverlayTransform::from_anchors(new_points, old_points)?
        }
    };
    let quarter = (transform.rotation_degrees / 90.0).round() * 90.0;
    if (quarter - transform.rotation_degrees).abs() > 0.001 {
        return Err(invalid(
            "transfer supports quarter-turn rotations only; choose new anchors",
        ));
    }
    // An almost-quarter-turn is accepted only within the explicit angular tolerance.
    // Re-anchor the exact supported rotation rather than silently moving the first anchor.
    transform.rotation_degrees = quarter;
    if let TransferAlignment::Anchors {
        old_points,
        new_points,
    } = alignment
    {
        transform.translation = [0.0, 0.0];
        let p = apply_quarter_turn(transform, old_points[0]);
        transform.translation = [new_points[0][0] - p[0], new_points[0][1] - p[1]];
    }
    transform.validate()?;
    Ok(transform)
}

/// Only used after resolved_transform has admitted an exact quarter turn. Exact coefficients
/// avoid false out-of-page refusals caused by sin(pi/2) and cos(pi/2) at the page boundary.
fn apply_quarter_turn(transform: OverlayTransform, point: [f64; 2]) -> [f64; 2] {
    let [x, y] = point;
    let [x, y] = match (transform.rotation_degrees / 90.0).round() as i32 % 4 {
        0 => [x, y],
        1 | -3 => [-y, x],
        2 | -2 => [-x, -y],
        3 | -1 => [y, -x],
        _ => unreachable!("validated quarter turn"),
    };
    [
        transform.translation[0] + transform.scale * x,
        transform.translation[1] + transform.scale * y,
    ]
}

fn transformed_primitive(
    primitive: &LibraryPrimitive,
    transform: OverlayTransform,
    old: PageGeometry,
    new: PageGeometry,
) -> Result<LibraryPrimitive, EngineError> {
    let (_, old_h) = old.display_size();
    let (w, new_h) = new.display_size();
    let point = |(x, y): (f32, f32)| -> Result<(f32, f32), EngineError> {
        let p = apply_quarter_turn(transform, [f64::from(x), f64::from(old_h) - f64::from(y)]);
        let p = [p[0], f64::from(new_h) - p[1]];
        if p.iter().any(|v| !v.is_finite())
            || p[0] < 0.0
            || p[1] < 0.0
            || p[0] > f64::from(w)
            || p[1] > f64::from(new_h)
        {
            return Err(invalid("transformed annotation is outside the target page"));
        }
        let result = (p[0] as f32, p[1] as f32);
        let user = new.to_user(result.0, result.1);
        let stored = new.to_display(user.0, user.1);
        if [
            (p[0], result.0),
            (p[1], result.1),
            (p[0], stored.0),
            (p[1], stored.1),
        ]
        .iter()
        .any(|(a, b)| (a - f64::from(*b)).abs() > POSITION_TOLERANCE)
        {
            return Err(invalid(
                "target coordinates lose more than 0.001 point when stored",
            ));
        }
        Ok(result)
    };
    let style = |s: &LibraryStyle| -> LibraryStyle {
        let mut s = s.clone();
        s.width_pt = (f64::from(s.width_pt) * transform.scale) as f32;
        for d in &mut s.dash_pattern {
            *d = (f64::from(*d) * transform.scale) as f32;
        }
        s
    };
    Ok(match primitive {
        LibraryPrimitive::Shape {
            shape,
            rect,
            style: s,
        } => {
            let a = point((rect[0], rect[1]))?;
            let b = point((rect[2], rect[3]))?;
            let rect = if matches!(shape, LibraryShape::Line | LibraryShape::Arrow) {
                [a.0, a.1, b.0, b.1]
            } else {
                [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]
            };
            LibraryPrimitive::Shape {
                shape: *shape,
                rect,
                style: style(s),
            }
        }
        LibraryPrimitive::Vertices {
            vertex_kind,
            points,
            style: s,
        } => LibraryPrimitive::Vertices {
            vertex_kind: *vertex_kind,
            points: points
                .iter()
                .copied()
                .map(point)
                .collect::<Result<_, _>>()?,
            style: style(s),
        },
        _ => return Err(invalid("only native shapes and vertices are transferable")),
    })
}

fn same_stored_primitive(expected: &LibraryPrimitive, actual: &LibraryPrimitive) -> bool {
    let close = |a: f32, b: f32| (f64::from(a) - f64::from(b)).abs() <= POSITION_TOLERANCE;
    match (expected, actual) {
        (
            LibraryPrimitive::Shape {
                shape: a,
                rect: ar,
                style: ast,
            },
            LibraryPrimitive::Shape {
                shape: b,
                rect: br,
                style: bst,
            },
        ) => a == b && ast == bst && ar.iter().zip(br).all(|(a, b)| close(*a, *b)),
        (
            LibraryPrimitive::Vertices {
                vertex_kind: a,
                points: ap,
                style: ast,
            },
            LibraryPrimitive::Vertices {
                vertex_kind: b,
                points: bp,
                style: bst,
            },
        ) => {
            a == b
                && ast == bst
                && ap.len() == bp.len()
                && ap
                    .iter()
                    .zip(bp)
                    .all(|(a, b)| close(a.0, b.0) && close(a.1, b.1))
        }
        _ => false,
    }
}

pub fn prepare_revision_transfer(
    engine: &Engine,
    old_path: &Path,
    new_path: &Path,
    passwords: TransferPasswords<'_>,
    request: &RevisionTransferRequest,
    cancel: &AtomicBool,
) -> Result<PreparedRevisionTransfer, EngineError> {
    validate_request(request)?;
    cancelled(cancel)?;
    if request.old_sha256 == request.new_sha256 {
        return Err(invalid(
            "identical saved revisions cannot be used for annotation transfer",
        ));
    }
    if request.pairs.iter().any(|p| !p.confirmed)
        || request.pairs.iter().all(|p| p.selections.is_empty())
    {
        return Err(invalid(
            "confirm every page mapping and select at least one annotation",
        ));
    }
    let old_selection_path = std::path::absolute(old_path)?;
    let new_selection_path = std::path::absolute(new_path)?;
    let old_path = old_path.canonicalize()?;
    let new_path = new_path.canonicalize()?;
    let old_bytes = read_source(&old_path, cancel)?;
    let new_bytes = read_source(&new_path, cancel)?;
    if digest(&old_bytes) != request.old_sha256 || digest(&new_bytes) != request.new_sha256 {
        return Err(invalid(
            "source revision changed; inspect and confirm a new recipe",
        ));
    }
    let old = engine.open_bytes(old_bytes, passwords.old, &old_path)?;
    let mut new = engine.open_bytes(new_bytes.clone(), passwords.new, &new_path)?;
    let mut history = check_documents(&old, &new, &new_bytes)?;
    if history.len() == 64 {
        return Err(invalid(
            "target already contains the maximum of 64 transfer records",
        ));
    }
    let repeated = history.iter().any(|v| v.old_sha256 == request.old_sha256);
    if repeated && !request.acknowledge_repeated_transfer {
        return Err(invalid(
            "this source was previously transferred; explicitly acknowledge possible duplicates",
        ));
    }
    let mut report=RevisionTransferReport{old_sha256:request.old_sha256.clone(),new_sha256:request.new_sha256.clone(),output_path:None,output_sha256:None,pages:Vec::new(),transferred:Vec::new(),exclusions:Vec::new(),warnings:vec!["Repeated transfers can create duplicate annotations; no proximity-based deduplication is performed.".into()]};
    for pair in &request.pairs {
        cancelled(cancel)?;
        let old_geometry = geometry(&old, pair.old_page)?;
        let new_geometry = geometry(&new, pair.new_page)?;
        let transform = resolved_transform(pair.alignment, old_geometry, new_geometry)?;
        let captured = old.capture_original_annotations(pair.old_page - 1)?;
        for candidate in &captured {
            if !pair.selections.iter().any(|s| s.index == candidate.index) {
                report.exclusions.push(TransferExclusion {
                    old_page: pair.old_page,
                    index: candidate.index,
                    reason: candidate
                        .refusal
                        .clone()
                        .unwrap_or_else(|| "Not selected".into()),
                });
            }
        }
        for selection in &pair.selections {
            cancelled(cancel)?;
            let source = captured
                .iter()
                .find(|c| c.index == selection.index)
                .ok_or_else(|| invalid("selected annotation is unavailable"))?;
            if source.fingerprint.as_deref() != Some(selection.fingerprint.as_str())
                || source.primitive.is_none()
            {
                return Err(invalid(source.refusal.clone().unwrap_or_else(|| {
                    "selected annotation changed; refresh the selection".into()
                })));
            }
            let primitive = transformed_primitive(
                source.primitive.as_ref().expect("checked primitive"),
                transform,
                old_geometry,
                new_geometry,
            )?;
            let entry = LibraryEntry {
                id: "revision-transfer".into(),
                name: "Revision transfer".into(),
                provenance: LibraryProvenance::default(),
                primitives: vec![primitive.clone()],
            };
            let bounds = entry.bounds()?;
            let entry = entry.normalized()?;
            let indices = new.apply_library_entry_in_place(
                pair.new_page - 1,
                &entry,
                LibraryPlacement {
                    origin: (bounds[0], bounds[1]),
                    scale: 1.0,
                },
                source.author.as_deref(),
            )?;
            let target_index = indices[0];
            let target = new.capture_original_annotation(pair.new_page - 1, target_index)?;
            if target.refusal.is_some()
                || target.source_nm.is_none()
                || target.source_nm == source.source_nm
                || target.author != source.author
                || target
                    .primitive
                    .as_ref()
                    .is_none_or(|written| !same_stored_primitive(&primitive, written))
            {
                return Err(invalid(
                    "written annotation did not retain a new native identity",
                ));
            }
            report.transferred.push(TransferredAnnotation {
                old_page: pair.old_page,
                new_page: pair.new_page,
                source_index: source.index,
                target_index,
                source_nm: source.source_nm.clone().expect("checked source identity"),
                target_nm: target.source_nm.expect("checked target identity"),
                source_fingerprint: selection.fingerprint.clone(),
                author: source.author.clone(),
            });
            if new.snapshot()?.len() > MAX_CANDIDATE_BYTES {
                return Err(invalid("candidate exceeds 160 MiB"));
            }
        }
        report.pages.push(TransferPageReport {
            old_page: pair.old_page,
            new_page: pair.new_page,
            old_geometry: old_geometry.into(),
            new_geometry: new_geometry.into(),
            transform,
        });
    }
    cancelled(cancel)?;
    history.push(TransferTrace {
        old_sha256: request.old_sha256.clone(),
        annotations: report.transferred.len(),
    });
    let mut pdf = crate::security::load(&new.snapshot()?, new.password())?;
    let trace = serde_json::to_vec(&history).map_err(|e| invalid(e.to_string()))?;
    pdf.catalog_mut()
        .map_err(crate::security::lopdf_err)?
        .set(TRACE_KEY, lopdf::Object::string_literal(trace));
    let bytes = crate::security::save(pdf)?;
    if bytes.len() > MAX_CANDIDATE_BYTES {
        return Err(invalid("candidate exceeds 160 MiB"));
    }
    cancelled(cancel)?;
    let prepared = PreparedRevisionTransfer {
        old_path,
        new_path,
        old_selection_path,
        new_selection_path,
        bytes,
        report,
    };
    prepared.revalidate_sources(cancel)?;
    Ok(prepared)
}

/// Commit point is the atomic no-clobber publication. Cancellation after this succeeds must not
/// turn the created output into a failure or remove it.
pub fn publish_revision_transfer(
    mut prepared: PreparedRevisionTransfer,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<RevisionTransferReport, EngineError> {
    cancelled(cancel)?;
    if output.symlink_metadata().is_ok() {
        return Err(EngineError::AlreadyExists(output.into()));
    }
    let output_sha256 = digest(&prepared.bytes);
    // Use the same temporary-file/no-clobber primitive as write_new_copy, with cancellation
    // during staging and immediately before its single commit point.
    use std::io::Write;
    let directory = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    for chunk in prepared.bytes.chunks(1024 * 1024) {
        cancelled(cancel)?;
        temporary.write_all(chunk)?;
    }
    temporary.as_file().sync_all()?;
    prepared.revalidate_sources(cancel)?;
    cancelled(cancel)?;
    temporary.persist_noclobber(output).map_err(|failure| {
        if failure.error.kind() == std::io::ErrorKind::AlreadyExists {
            EngineError::AlreadyExists(output.into())
        } else {
            EngineError::Io(failure.error)
        }
    })?;
    prepared.report.output_path = Some(output.to_owned());
    prepared.report.output_sha256 = Some(output_sha256);
    Ok(prepared.report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PointRect, Rotation};

    #[test]
    fn quarter_turns_keep_exact_page_edges_but_never_clip_outside_geometry() {
        let page = |w, h| PageGeometry {
            bbox: PointRect {
                left: 0.0,
                bottom: 0.0,
                right: w,
                top: h,
            },
            rotation: Rotation::R0,
        };
        let old = page(600.0, 800.0);
        let original = LibraryPrimitive::Shape {
            shape: LibraryShape::Square,
            rect: [0.0, 0.0, 4.0, 4.0],
            style: LibraryStyle::default(),
        };
        for (angle, translation, new, expected) in [
            (0.0, [0.0, 0.0], old, [0.0, 0.0, 4.0, 4.0]),
            (
                90.0,
                [800.0, 0.0],
                page(800.0, 600.0),
                [0.0, 596.0, 4.0, 600.0],
            ),
            (180.0, [600.0, 800.0], old, [596.0, 796.0, 600.0, 800.0]),
            (
                270.0,
                [0.0, 600.0],
                page(800.0, 600.0),
                [796.0, 0.0, 800.0, 4.0],
            ),
        ] {
            let transform = OverlayTransform {
                translation,
                rotation_degrees: angle,
                scale: 1.0,
            };
            let actual = transformed_primitive(&original, transform, old, new).unwrap();
            let LibraryPrimitive::Shape { rect, .. } = actual else {
                panic!("shape expected")
            };
            assert_eq!(rect, expected);
        }
        let outside = OverlayTransform {
            translation: [799.99, 0.0],
            rotation_degrees: 90.0,
            scale: 1.0,
        };
        assert!(transformed_primitive(&original, outside, old, page(800.0, 600.0)).is_err());
    }
}
