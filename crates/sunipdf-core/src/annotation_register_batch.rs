//! Explicit, source-bound field changes with one atomic document replacement.

use crate::annot_write::{load_annot, store_annot, touch, Slot};
use crate::annotation_register::{editable, AnnotationRegister, RegisterFields, RegisterRow};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError};
use lopdf::{Object, ObjectId};
use serde::{de, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

pub const MAX_REGISTER_BATCH_ROWS: usize = 256;
const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("annotation field batch: {reason}"))
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Keep leaves a field unchanged; Clear removes it; Set retains its literal text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FieldPatch {
    #[default]
    Keep,
    Set(String),
    Clear,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterFieldsPatch {
    #[serde(default)]
    pub business_status: FieldPatch,
    #[serde(default)]
    pub discipline: FieldPatch,
    #[serde(default, deserialize_with = "custom_patches")]
    pub custom: BTreeMap<String, FieldPatch>,
}

fn custom_patches<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, FieldPatch>, D::Error> {
    struct Patches;
    impl<'de> de::Visitor<'de> for Patches {
        type Value = BTreeMap<String, FieldPatch>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 16 distinct bounded custom field operations")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut fields = BTreeMap::new();
            while let Some(key) = access.next_key::<String>()? {
                if fields.len() >= 16 || fields.contains_key(&key) {
                    return Err(de::Error::custom(
                        "duplicate or excessive custom field operations",
                    ));
                }
                let operation = access.next_value::<FieldPatch>()?;
                validate_custom(&key, &operation).map_err(de::Error::custom)?;
                fields.insert(key, operation);
            }
            Ok(fields)
        }
    }
    deserializer.deserialize_map(Patches)
}

fn validate_custom(key: &str, operation: &FieldPatch) -> Result<(), EngineError> {
    let value = match operation {
        FieldPatch::Set(value) => value.as_str(),
        _ => "",
    };
    RegisterFields {
        custom: BTreeMap::from([(key.to_owned(), value.to_owned())]),
        ..Default::default()
    }
    .validate()
}

impl RegisterFieldsPatch {
    pub fn validate(&self) -> Result<(), EngineError> {
        let value = |operation: &FieldPatch| match operation {
            FieldPatch::Set(value) => Some(value.clone()),
            _ => None,
        };
        RegisterFields {
            business_status: value(&self.business_status),
            discipline: value(&self.discipline),
            custom: BTreeMap::new(),
        }
        .validate()?;
        if self.custom.len() > 16 {
            return Err(invalid("at most 16 custom field operations are allowed"));
        }
        for (key, operation) in &self.custom {
            validate_custom(key, operation)?;
        }
        Ok(())
    }

    fn apply(&self, before: &RegisterFields) -> Result<RegisterFields, EngineError> {
        before.validate()?;
        let mut after = before.clone();
        for (field, operation) in [
            (&mut after.business_status, &self.business_status),
            (&mut after.discipline, &self.discipline),
        ] {
            match operation {
                FieldPatch::Keep => {}
                FieldPatch::Clear => *field = None,
                FieldPatch::Set(value) => *field = Some(value.clone()),
            }
        }
        for (key, operation) in &self.custom {
            match operation {
                FieldPatch::Keep => {}
                FieldPatch::Clear => {
                    after.custom.remove(key);
                }
                FieldPatch::Set(value) => {
                    after.custom.insert(key.clone(), value.clone());
                }
            }
        }
        after.validate()?;
        Ok(after)
    }
}

/// Fingerprints every visible and derived register field using deterministic JSON field order.
/// This supplements, and never replaces, the saved-file or prepared-snapshot source guard.
pub fn register_row_fingerprint(row: &RegisterRow) -> Result<String, EngineError> {
    row.fields.validate()?;
    if row
        .rect
        .is_some_and(|rect| rect.iter().any(|value| !value.is_finite()))
    {
        return Err(invalid("record geometry is not finite"));
    }
    let bytes = serde_json::to_vec(row).map_err(|_| invalid("record cannot be serialized"))?;
    if bytes.len() > 128 * 1024 {
        return Err(invalid("record fingerprint exceeds its metadata bound"));
    }
    Ok(digest(&bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisterBatchChange {
    pub page: usize,
    pub index: usize,
    pub annotation_id: Option<String>,
    pub expected_row_sha256: String,
    pub before: RegisterFields,
    pub after: RegisterFields,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisterBatchReport {
    pub source_sha256: String,
    pub selected_count: usize,
    pub changed_count: usize,
    /// Includes selected unchanged rows so the complete reviewed selection stays bound.
    pub changes: Vec<RegisterBatchChange>,
}

#[derive(Clone)]
pub struct RegisterBatchPlan {
    source: Arc<Vec<u8>>,
    selected: Vec<RegisterRow>,
    patch: RegisterFieldsPatch,
    report: RegisterBatchReport,
}

impl std::fmt::Debug for RegisterBatchPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisterBatchPlan")
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

impl RegisterBatchPlan {
    pub fn report(&self) -> &RegisterBatchReport {
        &self.report
    }
}

#[derive(Default)]
struct References {
    annotations: HashMap<ObjectId, usize>,
    arrays: HashMap<ObjectId, usize>,
}

fn references(pdf: &lopdf::Document) -> Result<References, EngineError> {
    let mut refs = References::default();
    let mut visited = 0usize;
    for page in pdf.get_pages().values() {
        let dict = pdf.get_dictionary(*page).map_err(lopdf_err)?;
        let Ok(raw) = dict.get(b"Annots") else {
            continue;
        };
        let (array_id, array) = pdf.dereference(raw).map_err(lopdf_err)?;
        if let Some(id) = array_id {
            *refs.arrays.entry(id).or_default() += 1;
        }
        let entries = array.as_array().map_err(lopdf_err)?;
        visited = visited
            .checked_add(entries.len())
            .ok_or_else(|| invalid("annotation inventory overflow"))?;
        if visited > 10_000 {
            return Err(invalid("annotation inventory exceeds 10000 records"));
        }
        for entry in entries {
            let (id, dict) = pdf.dereference(entry).map_err(lopdf_err)?;
            dict.as_dict().map_err(lopdf_err)?;
            if let Some(id) = id {
                *refs.annotations.entry(id).or_default() += 1;
            }
        }
    }
    Ok(refs)
}

fn validate_target(
    pdf: &lopdf::Document,
    page: ObjectId,
    row: &RegisterRow,
    refs: &References,
) -> Result<(), EngineError> {
    let page_dict = pdf.get_dictionary(page).map_err(lopdf_err)?;
    let raw = page_dict.get(b"Annots").map_err(lopdf_err)?;
    let (array_id, _) = pdf.dereference(raw).map_err(lopdf_err)?;
    if array_id.is_some_and(|id| refs.arrays.get(&id) != Some(&1)) {
        return Err(invalid("shared annotation arrays cannot be edited"));
    }
    let (slot, dict) = load_annot(pdf, page, row.index)?;
    editable(&dict)?;
    // Resolve subtype too: an indirect Widget subtype must not bypass the edit restriction.
    let subtype = pdf
        .dereference(dict.get(b"Subtype").map_err(lopdf_err)?)
        .map_err(lopdf_err)?
        .1;
    if subtype.as_name().map_err(lopdf_err)? == b"Widget" {
        return Err(EngineError::SecurityRestricted);
    }
    if let Slot::Indirect(id) = slot {
        if refs.annotations.get(&id) != Some(&1) {
            return Err(invalid("shared annotation objects cannot be edited"));
        }
    }
    if let Ok(parent) = dict.get(b"P") {
        if parent.as_reference().map_err(lopdf_err)? != page {
            return Err(invalid("annotation page ownership differs"));
        }
    }
    Ok(())
}

impl Document<'_> {
    fn register_batch_permission(&self) -> Result<(), EngineError> {
        self.require_annotation_permission()?;
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if self.signature_count() != 0 {
            return Err(invalid("signed PDFs cannot receive batch field edits"));
        }
        Ok(())
    }

    pub fn preview_annotation_register_batch(
        &self,
        register: &AnnotationRegister,
        selected: &[RegisterRow],
        patch: &RegisterFieldsPatch,
    ) -> Result<RegisterBatchPlan, EngineError> {
        self.register_batch_permission()?;
        patch.validate()?;
        if selected.is_empty() || selected.len() > MAX_REGISTER_BATCH_ROWS {
            return Err(invalid("select 1 to 256 explicit records"));
        }
        let current = self.snapshot()?;
        if register.source_snapshot.len() > MAX_SOURCE_BYTES
            || current.len() > MAX_SOURCE_BYTES
            || digest(&register.source_snapshot) != register.source_sha256
            || !crate::annot_rich::same_document(
                &register.source_snapshot,
                &current,
                self.password(),
            )?
        {
            return Err(invalid("register is stale; refresh it before previewing"));
        }
        // Public row vectors can be changed by callers; rebuild the authoritative rows from the captured bytes.
        let pdf = security::load(&register.source_snapshot, self.password())?;
        let actual =
            self.annotation_register_from_pdf(register.source_snapshot.as_ref().clone(), &pdf)?;
        let refs = references(&pdf)?;
        let pages = pdf.get_pages();
        let mut identities = HashMap::<&str, usize>::new();
        for row in &actual.rows {
            if let Some(id) = &row.annotation_id {
                *identities.entry(id).or_default() += 1;
            }
        }
        let mut seen = HashSet::new();
        let mut changes = Vec::with_capacity(selected.len());
        for row in selected {
            if !seen.insert((row.page, row.index)) {
                return Err(invalid("duplicate selected record"));
            }
            if !actual.rows.iter().any(|candidate| candidate == row)
                || !register.rows.iter().any(|candidate| candidate == row)
            {
                return Err(invalid("selected record differs from the source register"));
            }
            if row
                .annotation_id
                .as_deref()
                .is_some_and(|id| id.is_empty() || identities.get(id) != Some(&1))
            {
                return Err(invalid("annotation identity is empty or shared"));
            }
            let page_number =
                u32::try_from(row.page).map_err(|_| invalid("page number is out of range"))?;
            let page = *pages
                .get(&page_number)
                .ok_or_else(|| invalid("selected page is missing"))?;
            validate_target(&pdf, page, row, &refs)?;
            changes.push(RegisterBatchChange {
                page: row.page,
                index: row.index,
                annotation_id: row.annotation_id.clone(),
                expected_row_sha256: register_row_fingerprint(row)?,
                before: row.fields.clone(),
                after: patch.apply(&row.fields)?,
            });
        }
        let report = RegisterBatchReport {
            source_sha256: register.source_sha256.clone(),
            selected_count: changes.len(),
            changed_count: changes
                .iter()
                .filter(|change| change.before != change.after)
                .count(),
            changes,
        };
        if serde_json::to_vec(&report)
            .map_err(|_| invalid("preview cannot be serialized"))?
            .len()
            > MAX_REPORT_BYTES
        {
            return Err(invalid("preview exceeds 16 MiB"));
        }
        Ok(RegisterBatchPlan {
            source: register.source_snapshot.clone(),
            selected: selected.to_vec(),
            patch: patch.clone(),
            report,
        })
    }

    /// Commit every reviewed field change with one replacement, suitable for a single Undo entry.
    pub fn apply_annotation_register_batch(
        &mut self,
        plan: &RegisterBatchPlan,
    ) -> Result<RegisterBatchReport, EngineError> {
        self.register_batch_permission()?;
        if plan.report.changed_count == 0 {
            return Err(invalid("the preview contains no changed fields"));
        }
        let current = self.snapshot()?;
        if current.len() > MAX_SOURCE_BYTES {
            return Err(invalid("document exceeds the 128 MiB editing limit"));
        }
        if !crate::annot_rich::same_document(&plan.source, &current, self.password())? {
            return Err(invalid("document changed; refresh the preview"));
        }
        let mut pdf = security::load(&current, self.password())?;
        let pages = pdf.get_pages();
        let refs = references(&pdf)?;
        for (row, change) in plan.selected.iter().zip(&plan.report.changes) {
            let page = *pages
                .get(&(row.page as u32))
                .ok_or_else(|| invalid("selected page is missing"))?;
            validate_target(&pdf, page, row, &refs)?;
            if plan.patch.apply(&row.fields)? != change.after {
                return Err(invalid("reviewed fields differ"));
            }
            if change.before == change.after {
                continue;
            }
            let (slot, mut dict) = load_annot(&pdf, page, row.index)?;
            dict.set(
                "SuniPDFRegister",
                Object::string_literal(
                    serde_json::to_vec(&serde_json::json!({
                        "version": 1, "fields": change.after
                    }))
                    .map_err(|_| invalid("fields cannot be serialized"))?,
                ),
            );
            touch(&mut dict);
            store_annot(&mut pdf, page, slot, dict)?;
        }
        // Valid fields can still exceed the reader's per-record or document-wide text budget.
        self.annotation_register_from_pdf(Vec::new(), &pdf)
            .map_err(|_| invalid("edited fields would make the annotation register unreadable"))?;
        let bytes = security::save(pdf)?;
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(invalid("edited PDF exceeds 128 MiB"));
        }
        self.restore(bytes)?;
        Ok(plan.report.clone())
    }
}
