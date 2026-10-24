//! Named rectangular plan zones and conservative, snapshot-derived markup assignments.
//! Zones are location metadata, independent of measurement scales and quantities.

use crate::page_calibration::{check_user_unit, qualified_content_digest};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PageGeometry, PointRect, RenderedPage};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Identity of the supplied saved bytes; it is not a password derivation function.
pub fn source_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const KEY: &[u8] = b"SuniPDFPlanZones";
pub const MAX_ZONES: usize = 64;
pub const MAX_DEPTH: usize = 8;
pub const MAX_RECORD_BYTES: usize = 128 * 1024;
const MAX_SNAPSHOT: usize = 128 * 1024 * 1024;

fn bad(message: &str) -> EngineError {
    EngineError::Other(format!("plan zones: {message}"))
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanZone {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    /// Displayed-page points, bottom-left origin, [left, bottom, right, top].
    pub bounds: [f32; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneValidity {
    Current,
    Stale,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PagePlanZones {
    /// One-based physical page number.
    pub page: usize,
    pub validity: ZoneValidity,
    /// Historical displayed coordinates when stale. Never use them for assignment.
    pub zones: Vec<PlanZone>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneAssignmentStatus {
    Assigned,
    Boundary,
    Overlap,
    #[default]
    Unassigned,
    GeometryUnknown,
    Stale,
}
impl ZoneAssignmentStatus {
    pub const ALL: [Self; 6] = [
        Self::Assigned,
        Self::Boundary,
        Self::Overlap,
        Self::Unassigned,
        Self::GeometryUnknown,
        Self::Stale,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Boundary => "boundary",
            Self::Overlap => "overlap",
            Self::Unassigned => "unassigned",
            Self::GeometryUnknown => "geometry_unknown",
            Self::Stale => "stale",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZonePathEntry {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneAssignment {
    pub status: ZoneAssignmentStatus,
    pub zone_id: Option<String>,
    pub path: Vec<ZonePathEntry>,
    pub candidates: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneFilter {
    pub page: usize,
    pub id: String,
    pub descendants: bool,
}
impl ZoneFilter {
    pub fn matches(&self, page: usize, assignment: &ZoneAssignment) -> bool {
        self.page == page
            && assignment.status == ZoneAssignmentStatus::Assigned
            && if self.descendants {
                assignment.path.iter().any(|p| p.id == self.id)
            } else {
                assignment.zone_id.as_deref() == Some(self.id.as_str())
            }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ZoneRowAssignment {
    pub page: usize,
    pub index: usize,
    pub annotation_id: Option<String>,
    pub before: ZoneAssignment,
    pub after: ZoneAssignment,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanZonePreview {
    pub before: PagePlanZones,
    pub after: PagePlanZones,
    pub assignments: Vec<ZoneRowAssignment>,
}
impl PlanZonePreview {
    pub fn affected_rows(&self) -> usize {
        self.assignments
            .iter()
            .filter(|r| r.before != r.after)
            .count()
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"preview":self,"affected_rows":self.affected_rows()})
    }
}
pub struct PreparedPlanZones {
    original: Vec<u8>,
    candidate: Vec<u8>,
    report: PlanZonePreview,
}
impl std::fmt::Debug for PreparedPlanZones {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPlanZones")
            .field("page", &self.report.after.page)
            .finish_non_exhaustive()
    }
}
impl PreparedPlanZones {
    pub fn report(&self) -> &PlanZonePreview {
        &self.report
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ZoneFile {
    version: u32,
    zones: Vec<PlanZone>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredZones {
    version: u32,
    coordinate_system: String,
    page_bounds: [f32; 4],
    rotation_degrees: i32,
    content_sha256: String,
    zones: Vec<PlanZone>,
}
fn rect(a: [f32; 4]) -> PointRect {
    PointRect {
        left: a[0],
        bottom: a[1],
        right: a[2],
        top: a[3],
    }
}
fn array(r: PointRect) -> [f32; 4] {
    [r.left, r.bottom, r.right, r.top]
}
fn valid_rect(a: [f32; 4]) -> bool {
    a.iter().all(|x| x.is_finite() && x.abs() <= 1e8) && a[0] < a[2] && a[1] < a[3]
}
fn contains(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[0] && a[1] <= b[1] && a[2] >= b[2] && a[3] >= b[3]
}
fn strict_contains(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] && a[1] < b[1] && a[2] > b[2] && a[3] > b[3]
}
fn touches(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}
fn degrees(geo: &PageGeometry) -> i32 {
    match geo.rotation {
        crate::Rotation::R0 => 0,
        crate::Rotation::R90 => 90,
        crate::Rotation::R180 => 180,
        crate::Rotation::R270 => 270,
    }
}
fn geometry(bounds: [f32; 4], rotation: i32) -> Result<PageGeometry, EngineError> {
    if !valid_rect(bounds) {
        return Err(bad("invalid stored page bounds"));
    }
    let rotation = match rotation {
        0 => crate::Rotation::R0,
        90 => crate::Rotation::R90,
        180 => crate::Rotation::R180,
        270 => crate::Rotation::R270,
        _ => return Err(bad("invalid stored rotation")),
    };
    Ok(PageGeometry {
        bbox: rect(bounds),
        rotation,
    })
}
fn validate(zones: &[PlanZone], page_bounds: Option<[f32; 4]>) -> Result<(), EngineError> {
    if zones.len() > MAX_ZONES {
        return Err(bad("at most 64 zones per page"));
    }
    let mut ids = HashMap::new();
    for z in zones {
        if z.id.is_empty()
            || z.id.len() > 64
            || !z
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            || ids.insert(z.id.as_str(), z).is_some()
            || z.name.trim().is_empty()
            || z.name != z.name.trim()
            || z.name.len() > 256
            || z.name.chars().any(char::is_control)
            || !valid_rect(z.bounds)
            || page_bounds.is_some_and(|b| !contains(b, z.bounds))
        {
            return Err(bad("invalid zone identity, name or page bounds"));
        }
    }
    for z in zones {
        let mut current = z;
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(&current.id) || seen.len() > MAX_DEPTH {
                return Err(bad(
                    "zone hierarchy contains a cycle or exceeds eight levels",
                ));
            }
            let Some(id) = &current.parent_id else {
                break;
            };
            let parent = ids
                .get(id.as_str())
                .ok_or_else(|| bad("parent zone is missing"))?;
            if !contains(parent.bounds, current.bounds) || parent.bounds == current.bounds {
                return Err(bad("parent must contain a smaller child zone"));
            }
            current = parent;
        }
    }
    Ok(())
}
pub fn parse_zones(bytes: &[u8]) -> Result<Vec<PlanZone>, EngineError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(bad("zone import exceeds 128 KiB"));
    }
    let file: ZoneFile =
        serde_json::from_slice(bytes).map_err(|_| bad("invalid or unsupported zone import"))?;
    if file.version != 1 {
        return Err(bad("unsupported zone import version"));
    }
    validate(&file.zones, None)?;
    Ok(file.zones)
}
pub fn encode_zones(zones: &[PlanZone]) -> Result<Vec<u8>, EngineError> {
    validate(zones, None)?;
    let bytes = serde_json::to_vec_pretty(&ZoneFile {
        version: 1,
        zones: zones.to_vec(),
    })
    .map_err(|_| bad("could not encode zones"))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(bad("zone import exceeds 128 KiB"));
    }
    Ok(bytes)
}
fn read_stored(page: &Dictionary) -> Result<Option<StoredZones>, EngineError> {
    let Ok(value) = page.get(KEY) else {
        return Ok(None);
    };
    let bytes = value
        .as_str()
        .map_err(|_| bad("unsupported zone metadata representation"))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(bad("zone metadata exceeds 128 KiB"));
    }
    let stored: StoredZones =
        serde_json::from_slice(bytes).map_err(|_| bad("malformed zone metadata"))?;
    if stored.version != 1
        || stored.coordinate_system != "pdf-user-space"
        || stored.content_sha256.len() != 64
        || !stored
            .content_sha256
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(bad("unsupported zone metadata"));
    }
    let original = geometry(stored.page_bounds, stored.rotation_degrees)?;
    let display = display_zones(&stored.zones, &original);
    let (w, h) = original.display_size();
    validate(&display, Some([0., 0., w, h]))?;
    Ok(Some(stored))
}
fn display_zones(zones: &[PlanZone], geo: &PageGeometry) -> Vec<PlanZone> {
    zones
        .iter()
        .map(|z| PlanZone {
            bounds: array(geo.rect_to_display(rect(z.bounds))),
            ..z.clone()
        })
        .collect()
}
pub(crate) fn read_page(
    pdf: &lopdf::Document,
    page: &Dictionary,
    geo: &PageGeometry,
    page_number: usize,
) -> Result<PagePlanZones, EngineError> {
    let Some(stored) = read_stored(page)? else {
        return Ok(PagePlanZones {
            page: page_number,
            validity: ZoneValidity::Current,
            zones: Vec::new(),
        });
    };
    let valid = stored.page_bounds == array(geo.bbox)
        && stored.rotation_degrees == degrees(geo)
        && check_user_unit(pdf, page).is_ok()
        && stored.content_sha256 == qualified_content_digest(pdf, page)?;
    let original = geometry(stored.page_bounds, stored.rotation_degrees)?;
    Ok(PagePlanZones {
        page: page_number,
        validity: if valid {
            ZoneValidity::Current
        } else {
            ZoneValidity::Stale
        },
        zones: display_zones(&stored.zones, if valid { geo } else { &original }),
    })
}
/// This conservative rule uses the complete displayed annotation rectangle, never its centre.
/// Ancestors form one path; unrelated overlapping zones never win by size or sort order.
pub fn assign_zone(zones: &PagePlanZones, bounds: Option<[f32; 4]>) -> ZoneAssignment {
    let status = |status| ZoneAssignment {
        status,
        ..Default::default()
    };
    if zones.validity == ZoneValidity::Stale {
        return status(ZoneAssignmentStatus::Stale);
    }
    if validate(&zones.zones, None).is_err() {
        return status(ZoneAssignmentStatus::GeometryUnknown);
    }
    let Some(bounds) = bounds.filter(|b| valid_rect(*b)) else {
        return status(ZoneAssignmentStatus::GeometryUnknown);
    };
    let matching: Vec<_> = zones
        .zones
        .iter()
        .filter(|z| strict_contains(z.bounds, bounds))
        .collect();
    let touched: Vec<_> = zones
        .zones
        .iter()
        .filter(|z| touches(z.bounds, bounds) && !strict_contains(z.bounds, bounds))
        .collect();
    if !touched.is_empty() {
        return ZoneAssignment {
            status: ZoneAssignmentStatus::Boundary,
            candidates: touched.iter().map(|z| z.id.clone()).collect(),
            ..Default::default()
        };
    }
    if matching.is_empty() {
        return status(ZoneAssignmentStatus::Unassigned);
    }
    let by_id: HashMap<_, _> = zones.zones.iter().map(|z| (z.id.as_str(), z)).collect();
    let path = |z: &PlanZone| -> Option<Vec<ZonePathEntry>> {
        let mut result = Vec::new();
        let mut current = z;
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(&current.id) || result.len() >= MAX_DEPTH {
                return None;
            }
            result.push(ZonePathEntry {
                id: current.id.clone(),
                name: current.name.clone(),
            });
            let Some(id) = &current.parent_id else {
                break;
            };
            current = *by_id.get(id.as_str())?;
        }
        result.reverse();
        Some(result)
    };
    let paths: Option<Vec<_>> = matching.iter().map(|z| path(z)).collect();
    let Some(mut paths) = paths else {
        return status(ZoneAssignmentStatus::GeometryUnknown);
    };
    paths.sort_by_key(|p| p.len());
    let deepest = paths.pop().expect("nonempty matching set");
    if matching
        .iter()
        .any(|z| !deepest.iter().any(|p| p.id == z.id))
    {
        return ZoneAssignment {
            status: ZoneAssignmentStatus::Overlap,
            candidates: matching.iter().map(|z| z.id.clone()).collect(),
            ..Default::default()
        };
    }
    ZoneAssignment {
        status: ZoneAssignmentStatus::Assigned,
        zone_id: deepest.last().map(|p| p.id.clone()),
        path: deepest,
        candidates: Vec::new(),
    }
}

impl Document<'_> {
    pub fn plan_zones(&self, page_index: usize) -> Result<PagePlanZones, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let source = self.snapshot()?;
        if source.len() > MAX_SNAPSHOT {
            return Err(bad("document exceeds 128 MiB"));
        }
        let pdf = security::load(&source, self.password())?;
        let id = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        read_page(
            &pdf,
            pdf.get_dictionary(id).map_err(lopdf_err)?,
            &geo,
            page_index + 1,
        )
    }
    fn zone_write_permission(&self) -> Result<(), EngineError> {
        let properties = self.properties();
        if !properties.can_modify || !properties.can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(bad("signed PDFs cannot change plan zones"));
        }
        Ok(())
    }
    pub fn prepare_plan_zones(
        &self,
        page_index: usize,
        replacement: &[PlanZone],
        reset_stale: bool,
    ) -> Result<PreparedPlanZones, EngineError> {
        self.zone_write_permission()?;
        let before = self.plan_zones(page_index)?;
        if before.validity == ZoneValidity::Stale && !reset_stale {
            return Err(bad(
                "saved zones are stale; explicitly review and replace their historical geometry",
            ));
        }
        let geo = self.page_geometry(page_index)?;
        let (w, h) = geo.display_size();
        validate(replacement, Some([0., 0., w, h]))?;
        let original = self.snapshot()?;
        if original.len() > MAX_SNAPSHOT {
            return Err(bad("document exceeds 128 MiB"));
        }
        let mut scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(original.clone(), self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        scratch.edit_pdf(page_index, |pdf, page, geo| {
            let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
            read_stored(dict)?;
            check_user_unit(pdf, dict)?;
            let stored_zones: Vec<_> = replacement
                .iter()
                .map(|z| PlanZone {
                    bounds: array(geo.rect_to_user(rect(z.bounds))),
                    ..z.clone()
                })
                .collect();
            let reread = display_zones(&stored_zones, geo);
            validate(&reread, Some([0., 0., w, h]))?;
            if reread.iter().zip(replacement).any(|(a, b)| {
                a.bounds
                    .iter()
                    .zip(b.bounds)
                    .any(|(x, y)| (*x - y).abs() > 0.001)
            }) {
                return Err(bad("page origin loses zone coordinate precision"));
            }
            let stored = StoredZones {
                version: 1,
                coordinate_system: "pdf-user-space".into(),
                page_bounds: array(geo.bbox),
                rotation_degrees: degrees(geo),
                content_sha256: qualified_content_digest(pdf, dict)?,
                zones: stored_zones,
            };
            let bytes = serde_json::to_vec(&stored).map_err(|_| bad("could not encode zones"))?;
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(bad("zone metadata exceeds 128 KiB"));
            }
            let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
            if replacement.is_empty() {
                dict.remove(KEY);
            } else {
                dict.set(KEY, Object::string_literal(bytes));
            }
            Ok(())
        })?;
        let after = scratch.plan_zones(page_index)?;
        let register = self.annotation_register()?;
        let assignments = register
            .rows
            .iter()
            .filter(|r| r.page == page_index + 1)
            .map(|r| ZoneRowAssignment {
                page: r.page,
                index: r.index,
                annotation_id: r.annotation_id.clone(),
                before: assign_zone(&before, r.rect),
                after: assign_zone(&after, r.rect),
            })
            .collect();
        let candidate = scratch.snapshot()?;
        if candidate.len() > 160 * 1024 * 1024 {
            return Err(bad("candidate exceeds 160 MiB"));
        }
        Ok(PreparedPlanZones {
            original,
            candidate,
            report: PlanZonePreview {
                before,
                after,
                assignments,
            },
        })
    }
    pub fn apply_plan_zones(&mut self, prepared: PreparedPlanZones) -> Result<(), EngineError> {
        self.zone_write_permission()?;
        if !crate::annot_rich::same_document(
            &prepared.original,
            &self.snapshot()?,
            self.password(),
        )? {
            return Err(bad("document changed; preview plan zones again"));
        }
        self.restore(prepared.candidate)
    }
    pub fn render_plan_zone_preview(
        &self,
        prepared: &PreparedPlanZones,
        long_side: u32,
    ) -> Result<RenderedPage, EngineError> {
        if !(64..=1536).contains(&long_side) {
            return Err(bad("preview side must be 64..1536 pixels"));
        }
        if !crate::annot_rich::same_document(
            &prepared.original,
            &self.snapshot()?,
            self.password(),
        )? {
            return Err(bad("document changed; preview plan zones again"));
        }
        let scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(prepared.candidate.clone(), self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        let page = prepared.report.after.page - 1;
        let size = scratch.page_size(page)?;
        scratch.render_page(page, long_side as f32 / size.width_pt.max(size.height_pt))
    }
}
