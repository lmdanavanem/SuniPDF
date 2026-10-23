//! Page defaults are independent of the immutable scale in each saved measurement annotation.

use crate::measurement::{Measurement, MeasurementKind, MeasurementUnit};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError, PageGeometry};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const KEY: &[u8] = b"SuniPDFPageCalibration";
const MAX_RECORD: usize = 4096;
const MAX_STREAM: usize = 8 * 1024 * 1024;
const MAX_CONTENT: usize = 32 * 1024 * 1024;
const MAX_RESOURCE_NODES: usize = 50_000;
const MAX_RESOURCE_DEPTH: usize = 64;

/// A two-point scale for future measurements on one page. Coordinates use the displayed page,
/// with its bottom-left origin, exactly like [`Measurement`]. Saving the PDF persists this default.
#[derive(Debug, Clone, PartialEq)]
pub struct PageCalibration {
    pub points: [(f32, f32); 2],
    pub known_distance: f64,
    pub unit: MeasurementUnit,
}

impl PageCalibration {
    pub fn validate(&self) -> Result<(), EngineError> {
        Measurement {
            kind: MeasurementKind::Length,
            points: self.points.to_vec(),
            calibration: self.points,
            known_distance: self.known_distance,
            unit: self.unit,
            precision: 2,
        }
        .value()
        .map(|_| ())
    }

    pub(crate) fn validate_on_page(&self, geo: &PageGeometry) -> Result<(), EngineError> {
        self.validate()?;
        let (w, h) = geo.display_size();
        if self
            .points
            .iter()
            .any(|&(x, y)| x < 0.0 || y < 0.0 || x > w || y > h)
        {
            return Err(invalid("reference points are outside the displayed page"));
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredCalibration {
    version: u32,
    coordinate_system: String,
    points: [(f32, f32); 2],
    known_distance: f64,
    unit: String,
    bbox: [f32; 4],
    user_unit: f32,
    content_sha256: String,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("page calibration: {reason}"))
}

fn bounds(geo: &PageGeometry) -> [f32; 4] {
    [geo.bbox.left, geo.bbox.bottom, geo.bbox.right, geo.bbox.top]
}

fn read_record(dict: &Dictionary) -> Result<Option<StoredCalibration>, EngineError> {
    let Ok(raw) = dict.get(KEY) else {
        return Ok(None);
    };
    // A private record is a direct, bounded string. Unknown representations are never overwritten.
    let raw = raw
        .as_str()
        .map_err(|_| invalid("unsupported record representation"))?;
    if raw.len() > MAX_RECORD {
        return Err(invalid("record exceeds the size limit"));
    }
    let record: StoredCalibration =
        serde_json::from_slice(raw).map_err(|_| invalid("malformed or unsupported record"))?;
    if !matches!(record.version, 1 | 2) || record.coordinate_system != "pdf-user-space" {
        return Err(invalid("unsupported record version or coordinate system"));
    }
    if record
        .points
        .iter()
        .flat_map(|&(x, y)| [x, y])
        .chain(record.bbox)
        .any(|v| !v.is_finite() || v.abs() > 1e8)
        || record.bbox[0] >= record.bbox[2]
        || record.bbox[1] >= record.bbox[3]
        || record.points.iter().any(|&(x, y)| {
            x < record.bbox[0] || y < record.bbox[1] || x > record.bbox[2] || y > record.bbox[3]
        })
        || record.user_unit != 1.0
        || record.content_sha256.len() != 64
        || !record
            .content_sha256
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(invalid("invalid geometry or content guard"));
    }
    PageCalibration {
        points: record.points,
        known_distance: record.known_distance,
        unit: MeasurementUnit::from_id(&record.unit).ok_or_else(|| invalid("unsupported unit"))?,
    }
    .validate()?;
    Ok(Some(record))
}

pub(crate) fn check_user_unit(pdf: &lopdf::Document, page: &Dictionary) -> Result<(), EngineError> {
    if let Ok(raw) = page.get(b"UserUnit") {
        let value = pdf.dereference(raw).map_err(lopdf_err)?.1;
        if value.as_float().ok() != Some(1.0) {
            return Err(invalid(
                "non-default UserUnit is unsupported; use a session scale",
            ));
        }
    }
    Ok(())
}

fn decoded_content_guard(
    pdf: &lopdf::Document,
    page: &Dictionary,
) -> Result<(String, bool), EngineError> {
    let mut digest = Sha256::new();
    let mut total = 0usize;
    let mut contains_names = false;
    if let Ok(raw) = page.get(b"Contents") {
        let object = pdf.dereference(raw).map_err(lopdf_err)?.1;
        let streams = match object {
            Object::Array(entries) if entries.len() <= 1024 => entries.as_slice(),
            Object::Stream(_) => std::slice::from_ref(object),
            _ => return Err(invalid("unsupported page content")),
        };
        for raw in streams {
            let stream = pdf
                .dereference(raw)
                .map_err(lopdf_err)?
                .1
                .as_stream()
                .map_err(lopdf_err)?;
            let bytes = if stream.dict.has(b"Filter") {
                stream
                    .decompressed_content_with_limit(MAX_STREAM)
                    .map_err(lopdf_err)?
            } else {
                if stream.content.len() > MAX_STREAM {
                    return Err(invalid("page content exceeds the stream limit"));
                }
                stream.content.clone()
            };
            total = total.saturating_add(bytes.len());
            if total > MAX_CONTENT {
                return Err(invalid("page content exceeds the total limit"));
            }
            contains_names |= bytes.contains(&b'/');
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
    }
    let hash = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((hash, contains_names))
}

fn legacy_content_digest(pdf: &lopdf::Document, page: &Dictionary) -> Result<String, EngineError> {
    decoded_content_guard(pdf, page).map(|(hash, _)| hash)
}

/// Resolve the inherited resource dictionary without treating page-tree object IDs as content.
fn resources<'a>(
    pdf: &'a lopdf::Document,
    mut page: &'a Dictionary,
) -> Result<Option<&'a Dictionary>, EngineError> {
    let mut visited = HashSet::new();
    for _ in 0..MAX_RESOURCE_DEPTH {
        if let Ok(raw) = page.get(b"Resources") {
            let object = pdf.dereference(raw).map_err(lopdf_err)?.1;
            if !matches!(object, Object::Null) {
                return object.as_dict().map(Some).map_err(lopdf_err);
            }
        }
        let Ok(parent) = page.get(b"Parent") else {
            return Ok(None);
        };
        let (id, object) = pdf.dereference(parent).map_err(lopdf_err)?;
        if id.is_some_and(|id| !visited.insert(id)) {
            return Err(invalid("cyclic resource inheritance"));
        }
        page = object.as_dict().map_err(lopdf_err)?;
    }
    Err(invalid("resource inheritance exceeds the depth limit"))
}

struct ResourceDigest<'a> {
    pdf: &'a lopdf::Document,
    active: HashSet<lopdf::ObjectId>,
    memo: HashMap<lopdf::ObjectId, [u8; 32]>,
    nodes: usize,
    bytes: usize,
}

impl<'a> ResourceDigest<'a> {
    fn new(pdf: &'a lopdf::Document) -> Self {
        Self {
            pdf,
            active: HashSet::new(),
            memo: HashMap::new(),
            nodes: 0,
            bytes: 0,
        }
    }

    fn data(&mut self, digest: &mut Sha256, bytes: &[u8]) -> Result<(), EngineError> {
        self.bytes = self.bytes.saturating_add(bytes.len());
        if self.bytes > MAX_CONTENT {
            return Err(invalid("page resources exceed the total byte limit"));
        }
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
        Ok(())
    }

    fn dictionary(
        &mut self,
        digest: &mut Sha256,
        dict: &Dictionary,
        depth: usize,
        stream_decoded: Option<bool>,
    ) -> Result<(), EngineError> {
        if dict.len() > MAX_RESOURCE_NODES {
            return Err(invalid("page resources exceed the node limit"));
        }
        let mut entries: Vec<_> = dict
            .iter()
            .filter(|(key, _)| {
                !(stream_decoded.is_some() && key.as_slice() == b"Length"
                    || stream_decoded == Some(true)
                        && matches!(key.as_slice(), b"Filter" | b"DecodeParms"))
            })
            .collect();
        entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        digest.update((entries.len() as u64).to_le_bytes());
        for (key, value) in entries {
            self.data(digest, key)?;
            digest.update(self.object(value, depth + 1)?);
        }
        Ok(())
    }

    fn object(&mut self, object: &Object, depth: usize) -> Result<[u8; 32], EngineError> {
        self.nodes = self.nodes.saturating_add(1);
        if depth > MAX_RESOURCE_DEPTH || self.nodes > MAX_RESOURCE_NODES {
            return Err(invalid("page resources exceed the depth or node limit"));
        }
        if let Object::Reference(id) = object {
            if let Some(hash) = self.memo.get(id) {
                return Ok(*hash);
            }
            if !self.active.insert(*id) {
                return Err(invalid("cyclic page resources"));
            }
            let pdf = self.pdf;
            let value = pdf
                .objects
                .get(id)
                .ok_or_else(|| invalid("missing resource object"))?;
            let hash = self.object(value, depth + 1)?;
            self.active.remove(id);
            self.memo.insert(*id, hash);
            return Ok(hash);
        }
        let mut digest = Sha256::new();
        match object {
            Object::Null => digest.update(b"null"),
            Object::Boolean(value) => digest.update(if *value {
                b"true".as_slice()
            } else {
                b"false".as_slice()
            }),
            Object::Integer(value) => {
                digest.update(b"integer");
                digest.update(value.to_le_bytes());
            }
            Object::Real(value) => {
                if !value.is_finite() {
                    return Err(invalid("non-finite page resource number"));
                }
                // PDF writers may serialize 1.0 as 1; equal exact numeric values share a hash.
                let number = f64::from(*value);
                if number.fract() == 0.0
                    && (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&number)
                {
                    digest.update(b"integer");
                    digest.update((number as i64).to_le_bytes());
                } else {
                    digest.update(b"real");
                    digest.update(value.to_bits().to_le_bytes());
                }
            }
            Object::Name(value) => {
                digest.update(b"name");
                self.data(&mut digest, value)?;
            }
            Object::String(value, _) => {
                digest.update(b"string");
                self.data(&mut digest, value)?;
            }
            Object::Array(values) => {
                if values.len() > MAX_RESOURCE_NODES {
                    return Err(invalid("page resources exceed the node limit"));
                }
                digest.update(b"array");
                digest.update((values.len() as u64).to_le_bytes());
                for value in values {
                    digest.update(self.object(value, depth + 1)?);
                }
            }
            Object::Dictionary(dict) => {
                digest.update(b"dictionary");
                self.dictionary(&mut digest, dict, depth, None)?;
            }
            Object::Stream(stream) => {
                if stream.content.len() > MAX_STREAM {
                    return Err(invalid("page resources exceed the stream limit"));
                }
                // Normalize only plain Flate without predictors. Other encodings (including
                // JPEG/JPX) retain their exact bytes and decoder parameters in the guard.
                let flate = matches!(stream.dict.get(b"Filter"), Ok(Object::Name(name)) if name == b"FlateDecode")
                    && !stream.dict.has(b"DecodeParms");
                let decoded = !stream.dict.has(b"Filter") || flate;
                digest.update(if decoded {
                    b"decoded-stream".as_slice()
                } else {
                    b"encoded-stream".as_slice()
                });
                self.dictionary(&mut digest, &stream.dict, depth, Some(decoded))?;
                if flate {
                    let bytes = stream
                        .decompressed_content_with_limit(MAX_STREAM)
                        .map_err(lopdf_err)?;
                    self.data(&mut digest, &bytes)?;
                } else {
                    self.data(&mut digest, &stream.content)?;
                }
            }
            Object::Reference(_) => unreachable!(),
        }
        Ok(digest.finalize().into())
    }
}

/// Version 2 guards decoded page content and the complete inherited resource graph, including
/// Form XObject matrices/content, fonts and graphics states. IDs, dictionary order, annotations
/// and plain Flate recompression do not affect it. Other resource encodings are hashed exactly.
/// Content and resources each have a 32 MiB aggregate budget, with 8 MiB per stream; resources
/// also have 50,000 visited nodes and 64 levels. Cycles and unsupported bounds fail explicitly.
pub(crate) fn content_digest(
    pdf: &lopdf::Document,
    page: &Dictionary,
) -> Result<String, EngineError> {
    let mut digest = Sha256::new();
    digest.update(b"SuniPDF-page-content-v2\0");
    digest.update(legacy_content_digest(pdf, page)?);
    let mut graph = ResourceDigest::new(pdf);
    let empty = Dictionary::new();
    let dictionary = resources(pdf, page)?.unwrap_or(&empty);
    graph.dictionary(&mut digest, dictionary, 0, None)?;
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn record_digest(
    pdf: &lopdf::Document,
    page: &Dictionary,
    version: u32,
) -> Result<String, EngineError> {
    if version == 1 {
        // V1 did not bind resources. There is no trustworthy way to infer whether they changed
        // since calibration, so migration requires an explicit new scale instead of guessing.
        let (hash, contains_names) = decoded_content_guard(pdf, page)?;
        // Refuse names even if a resource dictionary was removed since V1 was saved. This
        // conservative check also includes slashes inside strings instead of parsing untrusted
        // drawing programs to guess which resources were present when the scale was recorded.
        if contains_names || resources(pdf, page)?.is_some_and(|resources| !resources.is_empty()) {
            return Err(invalid(
                "legacy saved scale does not verify page resources; recalibrate or remove the saved scale",
            ));
        }
        Ok(hash)
    } else {
        content_digest(pdf, page)
    }
}

impl Document<'_> {
    /// Reads a saved default for this physical page. Missing metadata is `None`; malformed,
    /// unsupported or stale data is an error rather than an apparently usable scale.
    pub fn page_calibration(
        &self,
        page_index: usize,
    ) -> Result<Option<PageCalibration>, EngineError> {
        let geo = self.page_geometry(page_index)?;
        let pdf = security::load(&self.snapshot()?, self.password())?;
        let page = *pdf
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
        let Some(record) = read_record(dict)? else {
            return Ok(None);
        };
        check_user_unit(&pdf, dict)?;
        if record
            .bbox
            .iter()
            .zip(bounds(&geo))
            .any(|(a, b)| (a - b).abs() > 0.001)
            || record.content_sha256 != record_digest(&pdf, dict, record.version)?
        {
            return Err(invalid(
                "page geometry or content changed; recalibrate or remove the saved scale",
            ));
        }
        let calibration = PageCalibration {
            points: record.points.map(|(x, y)| geo.to_display(x, y)),
            known_distance: record.known_distance,
            unit: MeasurementUnit::from_id(&record.unit)
                .ok_or_else(|| invalid("unsupported unit"))?,
        };
        calibration.validate_on_page(&geo)?;
        Ok(Some(calibration))
    }

    /// Updates only the page default, leaving every saved annotation and its value unchanged.
    /// Requires document modification permission. `None` removes a supported record, including
    /// a stale one. Unknown or malformed records are preserved and return an error.
    pub fn set_page_calibration(
        &mut self,
        page_index: usize,
        calibration: Option<&PageCalibration>,
    ) -> Result<(), EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        self.edit_pdf(page_index, |pdf, page, geo| {
            set_in_pdf(pdf, page, geo, calibration)
        })
    }
}

pub(crate) fn set_in_pdf(
    pdf: &mut lopdf::Document,
    page: lopdf::ObjectId,
    geo: &PageGeometry,
    calibration: Option<&PageCalibration>,
) -> Result<(), EngineError> {
    let dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
    read_record(dict)?;
    let metadata = if let Some(calibration) = calibration {
        calibration.validate_on_page(geo)?;
        check_user_unit(pdf, dict)?;
        let record = StoredCalibration {
            version: 2,
            coordinate_system: "pdf-user-space".into(),
            points: calibration.points.map(|(x, y)| geo.to_user(x, y)),
            known_distance: calibration.known_distance,
            unit: calibration.unit.id().into(),
            bbox: bounds(geo),
            user_unit: 1.0,
            content_sha256: content_digest(pdf, dict)?,
        };
        let text =
            serde_json::to_string(&record).map_err(|_| invalid("could not encode the scale"))?;
        let mut check = Dictionary::new();
        check.set(KEY, Object::string_literal(text.clone()));
        read_record(&check)?;
        Some(text)
    } else {
        None
    };
    let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
    if let Some(text) = metadata {
        dict.set(KEY, Object::string_literal(text));
    } else {
        dict.remove(KEY);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    fn fixture() -> (lopdf::Document, Dictionary, lopdf::ObjectId) {
        let mut pdf = lopdf::Document::with_version("1.7");
        let form = pdf.add_object(Stream::new(dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(),0.into(),300.into(),300.into()],
            "Matrix" => vec![1.into(),0.into(),0.into(),1.into(),0.into(),0.into()],
        }, b"40 80 m 140 80 l S".to_vec()));
        let resources = pdf.add_object(dictionary! {"XObject" => dictionary! {"Fm0" => form}});
        let parent = pdf.add_object(dictionary! {"Resources" => resources});
        let contents = pdf.add_object(Stream::new(Dictionary::new(), b"/Fm0 Do".to_vec()));
        (
            pdf,
            dictionary! {"Contents" => contents, "Parent" => parent},
            form,
        )
    }

    #[test]
    fn resource_guard_detects_form_stream_matrix_and_inherited_font_changes() {
        let (pdf, page, form) = fixture();
        let before = content_digest(&pdf, &page).unwrap();
        let content_only = legacy_content_digest(&pdf, &page).unwrap();
        for change in ["stream", "matrix", "font"] {
            let mut changed = pdf.clone();
            match change {
                "stream" => {
                    changed
                        .get_object_mut(form)
                        .unwrap()
                        .as_stream_mut()
                        .unwrap()
                        .content = b"40 80 m 240 80 l S".to_vec()
                }
                "matrix" => changed
                    .get_object_mut(form)
                    .unwrap()
                    .as_stream_mut()
                    .unwrap()
                    .dict
                    .set(
                        "Matrix",
                        vec![2.into(), 0.into(), 0.into(), 2.into(), 0.into(), 0.into()],
                    ),
                "font" => {
                    let resources = resources(&pdf, &page).unwrap().unwrap();
                    let mut replacement = resources.clone();
                    replacement.set("Font", dictionary! {"F1" => dictionary! {"Type" => "Font", "Widths" => vec![600.into(),700.into()]}});
                    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
                    changed
                        .get_dictionary_mut(parent)
                        .unwrap()
                        .set("Resources", replacement);
                }
                _ => unreachable!(),
            }
            assert_eq!(
                legacy_content_digest(&changed, &page).unwrap(),
                content_only
            );
            assert_ne!(content_digest(&changed, &page).unwrap(), before, "{change}");
        }
    }

    #[test]
    fn resource_guard_ignores_ids_dictionary_order_annotations_and_plain_flate_encoding() {
        let (pdf, page, form) = fixture();
        let before = content_digest(&pdf, &page).unwrap();
        let mut changed = pdf.clone();
        changed
            .get_object_mut(form)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .compress()
            .unwrap();
        let inherited = resources(&changed, &page).unwrap().unwrap();
        let mut direct = page.clone();
        direct.set("Resources", inherited.clone());
        direct.remove(b"Parent");
        direct.set("Rotate", 90);
        direct.set("Annots", vec![Object::Null]);
        assert_eq!(content_digest(&changed, &direct).unwrap(), before);
        let relocated = changed.add_object(changed.get_object(form).unwrap().clone());
        let mut dict = Dictionary::new();
        dict.set("XObject", dictionary! {"Fm0" => relocated});
        direct.set("Resources", dict);
        assert_eq!(content_digest(&changed, &direct).unwrap(), before);
        let stream = changed
            .get_object_mut(relocated)
            .unwrap()
            .as_stream_mut()
            .unwrap();
        let mut reordered = Dictionary::new();
        for (key, value) in stream.dict.iter().rev() {
            reordered.set(key.clone(), value.clone());
        }
        stream.dict = reordered;
        assert_eq!(content_digest(&changed, &direct).unwrap(), before);
    }

    #[test]
    fn resource_guard_bounds_cycles_depth_bytes_and_nodes_and_hashes_encoded_images() {
        let (mut pdf, mut page, form) = fixture();
        let resources_id = pdf.add_object(dictionary! {"Recursive" => Object::Null});
        pdf.get_dictionary_mut(resources_id)
            .unwrap()
            .set("Recursive", resources_id);
        page.set("Resources", resources_id);
        assert!(content_digest(&pdf, &page)
            .unwrap_err()
            .to_string()
            .contains("cyclic"));
        let mut nested = Object::Null;
        for _ in 0..=MAX_RESOURCE_DEPTH {
            nested = Object::Array(vec![nested]);
        }
        page.set("Resources", dictionary! {"Deep" => nested});
        assert!(content_digest(&pdf, &page)
            .unwrap_err()
            .to_string()
            .contains("depth"));
        page.set(
            "Resources",
            dictionary! {"Wide" => vec![Object::Null; MAX_RESOURCE_NODES + 1]},
        );
        assert!(content_digest(&pdf, &page)
            .unwrap_err()
            .to_string()
            .contains("node"));
        page.set(
            "Resources",
            dictionary! {"XObject" => dictionary! {"Fm0" => form}},
        );
        let stream = pdf.get_object_mut(form).unwrap().as_stream_mut().unwrap();
        stream.dict.set("Filter", "DCTDecode");
        stream.content = vec![0xff, 0xd8, 0xff, 0xd9];
        let encoded = content_digest(&pdf, &page).unwrap();
        pdf.get_object_mut(form)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .dict
            .set("DecodeParms", dictionary! {"ColorTransform" => 0});
        assert_ne!(content_digest(&pdf, &page).unwrap(), encoded);
        pdf.get_object_mut(form)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .content = vec![0; MAX_STREAM + 1];
        assert!(content_digest(&pdf, &page)
            .unwrap_err()
            .to_string()
            .contains("stream limit"));
        let mut graph = ResourceDigest::new(&pdf);
        graph.bytes = MAX_CONTENT;
        assert!(graph
            .object(&Object::Name(b"x".to_vec()), 0)
            .unwrap_err()
            .to_string()
            .contains("total byte limit"));
    }

    #[test]
    fn legacy_guards_require_explicit_recalibration_for_any_resource_dependency() {
        let (mut pdf, mut page, _) = fixture();
        assert!(record_digest(&pdf, &page, 1)
            .unwrap_err()
            .to_string()
            .contains("recalibrate or remove"));
        page.set("Resources", Dictionary::new());
        assert!(record_digest(&pdf, &page, 1).is_err());
        let content = pdf.add_object(Stream::new(Dictionary::new(), b"0 0 m 100 0 l S".to_vec()));
        page.set("Contents", content);
        assert_eq!(
            record_digest(&pdf, &page, 1).unwrap(),
            legacy_content_digest(&pdf, &page).unwrap()
        );
        assert_eq!(
            record_digest(&pdf, &page, 2).unwrap(),
            content_digest(&pdf, &page).unwrap()
        );
    }
}
