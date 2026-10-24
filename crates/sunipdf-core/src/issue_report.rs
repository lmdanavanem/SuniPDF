//! Bounded, snapshot-bound issue and quantity reports. Sources are never edited.

use crate::quantity_table::{QuantityOptions, QuantityTable};
use crate::{Document, EngineError, RenderedPage};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_REPORT_PAGES: usize = 250;
pub const MAX_GROUPS: usize = 256;

pub(crate) fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Issue report: {message}"))
}
pub(crate) fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}
fn digest(bytes: &[u8]) -> String {
    crate::plan_zones::source_sha256(bytes)
}

#[derive(Debug, Clone)]
pub struct IssueReportOptions {
    pub title: String,
    pub quantities: QuantityOptions,
    pub include_legend: bool,
    pub selection: IssueReportSelection,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueReportSelection {
    #[default]
    Quantities,
    Register,
}
impl IssueReportSelection {
    pub fn id(self) -> &'static str {
        match self {
            Self::Quantities => "quantities",
            Self::Register => "register",
        }
    }
}
impl Default for IssueReportOptions {
    fn default() -> Self {
        Self {
            title: "Issue and quantity report".into(),
            quantities: QuantityOptions::default(),
            include_legend: true,
            selection: IssueReportSelection::Quantities,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueReportFormat {
    Pdf,
    Csv,
    Json,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueReportPhase {
    Collecting,
    Layout,
    Rendering,
    Ready,
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct IssueReportProgress {
    pub phase: IssueReportPhase,
    pub completed: usize,
    pub total: usize,
}

/// A saved file identity, kept separate from a document's serialized snapshot hash.
/// No file contents, password or absolute path are included in the public report.
#[derive(Clone)]
pub struct SavedReportSource {
    selected: PathBuf,
    canonical: PathBuf,
    sha256: String,
}
impl std::fmt::Debug for SavedReportSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SavedReportSource")
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
}
fn read_source(path: &Path, cancel: &AtomicBool) -> Result<Vec<u8>, EngineError> {
    cancelled(cancel)?;
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("source must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_SOURCE_BYTES as u64 {
        return Err(invalid("source exceeds 128 MiB or is not a regular file"));
    }
    let mut bytes = Vec::new();
    let mut reader = file.take(MAX_SOURCE_BYTES as u64 + 1);
    let mut chunk = [0; 64 * 1024];
    loop {
        cancelled(cancel)?;
        let count = reader.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(invalid("source exceeds 128 MiB"));
        }
    }
    Ok(bytes)
}
impl SavedReportSource {
    pub fn capture(
        path: &Path,
        expected_sha256: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<(Self, Vec<u8>), EngineError> {
        let canonical = path.canonicalize()?;
        let selected = std::path::absolute(path)?;
        let bytes = read_source(&canonical, cancel)?;
        let sha256 = digest(&bytes);
        if expected_sha256.is_some_and(|expected| {
            expected.len() != 64
                || !expected.bytes().all(|b| b.is_ascii_hexdigit())
                || !sha256.eq_ignore_ascii_case(expected)
        }) {
            return Err(invalid("saved file does not match the expected SHA256"));
        }
        let source = Self {
            selected,
            canonical,
            sha256,
        };
        source.revalidate(cancel)?;
        Ok((source, bytes))
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub(crate) fn verify_snapshot(
        &self,
        document: &Document<'_>,
        snapshot: &[u8],
        cancel: &AtomicBool,
    ) -> Result<(), EngineError> {
        self.revalidate(cancel)?;
        // Saved bytes and editor snapshots have different serialization forms.
        // Normalize through the same PDFium, then compare the complete guarded graph.
        // The original file SHA256 is checked before and after this comparison.
        let saved = document
            .pdfium
            .load_pdf_from_byte_vec(read_source(&self.canonical, cancel)?, document.password())?;
        let saved_snapshot = saved.save_to_bytes()?;
        cancelled(cancel)?;
        if saved_snapshot.len() > MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(&saved_snapshot, snapshot, document.password())?
        {
            return Err(invalid("saved source and the current document differ; save first or prepare an unsaved-snapshot report"));
        }
        self.revalidate(cancel)
    }
    pub fn revalidate(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        if self.selected.canonicalize()? != self.canonical
            || digest(&read_source(&self.canonical, cancel)?) != self.sha256
        {
            return Err(invalid("saved source changed; prepare the report again"));
        }
        cancelled(cancel)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportProvenance {
    pub snapshot_sha256: String,
    pub snapshot_hash_kind: &'static str,
    pub saved_file_sha256: Option<String>,
    pub saved_file_name: Option<String>,
    pub source_encrypted: bool,
    pub source_signature_count: usize,
    pub report_encrypted: bool,
    pub source_unchanged: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ReportRowIdentity {
    pub page: usize,
    pub index: usize,
    pub annotation_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct LegendStyle {
    pub stroke_rgb: [u8; 3],
    pub fill_rgb: Option<[u8; 3]>,
    pub width_pt: f32,
    pub opacity: f32,
    pub dash_pattern: Vec<f32>,
    pub arrow: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ReportLegend {
    pub id: String,
    pub kind: String,
    pub count_group: Option<String>,
    pub style: Option<LegendStyle>,
    pub style_status: &'static str,
    pub reason: Option<String>,
    pub count: usize,
    pub members: Vec<ReportRowIdentity>,
}
#[derive(Debug, Clone, Serialize)]
pub struct IssueGroup {
    pub business_status: Option<String>,
    pub review_status: Option<String>,
    pub discipline: Option<String>,
    pub count: usize,
    pub members: Vec<ReportRowIdentity>,
}
#[derive(Debug, Clone, Serialize)]
pub struct IssueReportModel {
    pub schema: &'static str,
    pub version: u32,
    pub title: String,
    pub selection: IssueReportSelection,
    pub provenance: ReportProvenance,
    /// The common quantity export evaluated with the recorded selection semantics.
    pub quantities: Value,
    pub legend_included: bool,
    pub legend: Vec<ReportLegend>,
    pub issue_groups: Vec<IssueGroup>,
    pub complete: bool,
    pub warnings: Vec<String>,
    pub page_count: usize,
    pub pdf_font: String,
    pub pdf_font_embedding: &'static str,
}
impl IssueReportModel {
    pub fn json(&self) -> Value {
        serde_json::to_value(self).expect("validated report model")
    }
}
pub struct PreparedIssueReport {
    original: Vec<u8>,
    source: Option<SavedReportSource>,
    model: IssueReportModel,
    pdf: Vec<u8>,
    csv: Vec<u8>,
    json: Vec<u8>,
}
impl std::fmt::Debug for PreparedIssueReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedIssueReport")
            .field("page_count", &self.model.page_count)
            .finish_non_exhaustive()
    }
}
impl PreparedIssueReport {
    pub fn model(&self) -> &IssueReportModel {
        &self.model
    }
    pub fn page_count(&self) -> usize {
        self.model.page_count
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct IssueReportReceipt {
    pub out: PathBuf,
    pub format: IssueReportFormat,
    pub output_sha256: String,
    pub bytes: usize,
    pub page_count: usize,
    pub matched_annotations: usize,
    pub complete: bool,
    pub provenance: ReportProvenance,
    pub warnings: Vec<String>,
}
impl IssueReportReceipt {
    pub fn json(&self) -> Value {
        serde_json::to_value(self).expect("report receipt")
    }
}

fn identity(row: &crate::annotation_register::RegisterRow) -> ReportRowIdentity {
    ReportRowIdentity {
        page: row.page,
        index: row.index,
        annotation_id: row.annotation_id.clone(),
    }
}
fn legend_style(detail: &crate::annot_write::AnnotDetail) -> Option<LegendStyle> {
    if !detail.ours
        || !detail.can_restyle
        || !matches!(
            detail.subtype.as_str(),
            "Square" | "Circle" | "Line" | "Ink"
        )
    {
        return None;
    }
    let (r, g, b) = detail.rgb?;
    let width = detail.width_pt?;
    if !width.is_finite()
        || !(0.0..=50.0).contains(&width)
        || !detail.opacity.is_finite()
        || !(0.0..=1.0).contains(&detail.opacity)
        || crate::annot_write::validate_dash_pattern(&detail.dash_pattern).is_err()
    {
        return None;
    }
    Some(LegendStyle {
        stroke_rgb: [r, g, b],
        fill_rgb: detail.fill_rgb.map(|(r, g, b)| [r, g, b]),
        width_pt: width,
        opacity: detail.opacity,
        dash_pattern: detail.dash_pattern.clone(),
        arrow: detail.arrow,
    })
}

impl Document<'_> {
    pub fn validate_report_register(
        &self,
        register: &crate::annotation_register::AnnotationRegister,
    ) -> Result<(), EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let current = self.snapshot()?;
        if current.len() > MAX_SOURCE_BYTES
            || register.source_snapshot.len() > MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(
                &register.source_snapshot,
                &current,
                self.password(),
            )?
        {
            return Err(invalid(
                "annotation register is stale; refresh before preparing a report",
            ));
        }
        Ok(())
    }
    pub fn prepare_issue_report(
        &self,
        table: &QuantityTable,
        options: &IssueReportOptions,
        source: Option<&SavedReportSource>,
        cancel: &AtomicBool,
        mut progress: impl FnMut(IssueReportProgress),
    ) -> Result<PreparedIssueReport, EngineError> {
        cancelled(cancel)?;
        let register_table = (options.selection == IssueReportSelection::Register)
            .then(|| table.for_register_report());
        let table = register_table.as_ref().unwrap_or(table);
        if options.title.trim().is_empty()
            || options.title.len() > 512
            || options.title.chars().any(char::is_control)
        {
            return Err(invalid(
                "title must contain 1..512 bytes without control characters",
            ));
        }
        let filter = &options.quantities.filter;
        if [
            &filter.query,
            &filter.author,
            &filter.modified,
            &filter.review_status,
            &filter.business_status,
            &filter.discipline,
            &filter.kind,
            &filter.count_group,
        ]
        .into_iter()
        .any(|text| text.len() > 4096)
            || filter
                .custom_sort
                .as_ref()
                .is_some_and(|text| text.len() > 4096)
            || filter.zone.as_ref().is_some_and(|zone| zone.id.len() > 64)
        {
            return Err(invalid("report filter exceeds its bounded text fields"));
        }
        if table.report_snapshot().len() > MAX_SOURCE_BYTES {
            return Err(invalid("snapshot exceeds 128 MiB"));
        }
        if filter
            .page
            .is_some_and(|page| page == 0 || page > table.page_count())
        {
            return Err(invalid("source page filter is outside the document"));
        }
        // This API checks permissions, stale tables, zone completeness and formula bounds.
        let quantities = self.export_quantity_table(table, &options.quantities)?.json;
        let original = table.report_snapshot().to_vec();
        if let Some(source) = source {
            source.verify_snapshot(self, &original, cancel)?;
        }
        let selected = table.select(&options.quantities);
        if selected.len() > 10_000 {
            return Err(invalid("more than 10000 selected annotations"));
        }
        let mut legends = BTreeMap::<String, ReportLegend>::new();
        let mut groups =
            BTreeMap::<(Option<String>, Option<String>, Option<String>), IssueGroup>::new();
        let mut details = BTreeMap::new();
        for (done, row) in selected.iter().enumerate() {
            cancelled(cancel)?;
            progress(IssueReportProgress {
                phase: IssueReportPhase::Collecting,
                completed: done,
                total: selected.len(),
            });
            let r = &row.register;
            let key = (
                r.fields.business_status.clone(),
                r.review_status.clone(),
                r.fields.discipline.clone(),
            );
            let group = groups.entry(key.clone()).or_insert_with(|| IssueGroup {
                business_status: key.0,
                review_status: key.1,
                discipline: key.2,
                count: 0,
                members: Vec::new(),
            });
            group.count += 1;
            group.members.push(identity(r));
            if options.include_legend {
                if let std::collections::btree_map::Entry::Vacant(entry) = details.entry(r.page) {
                    entry.insert(self.annotation_details(r.page - 1)?);
                }
                let style = details[&r.page]
                    .iter()
                    .find(|d| d.index == r.index && d.name == r.annotation_id)
                    .and_then(legend_style);
                let key = serde_json::to_string(&json!([r.kind, r.count_group, style]))
                    .map_err(|e| invalid(&e.to_string()))?;
                let next_id = format!("legend-{}", legends.len() + 1);
                let legend = legends.entry(key).or_insert_with(|| ReportLegend { id:next_id,kind:r.kind.clone(),count_group:r.count_group.clone(),style_status:if style.is_some(){"declared_pdf_style"}else{"unknown"},reason:style.is_none().then(|| "The annotation's supported style is unavailable; no symbol is inferred.".into()),style,count:0,members:Vec::new() });
                legend.count += 1;
                legend.members.push(identity(r));
            }
            if groups.len() > MAX_GROUPS || legends.len() > MAX_GROUPS {
                return Err(invalid("more than 256 report groups; narrow the filter"));
            }
        }
        let mut warnings = Vec::new();
        let zone_complete = quantities["zone_coverage"]["complete"] == true;
        let formula_complete = quantities["formula_evaluation_complete"] == true;
        let verified_measurements =
            quantities["summary"]["unverified_measurement_rows"].as_u64() == Some(0);
        if !zone_complete {
            warnings.push(
                "Some annotation locations are unresolved; the location coverage is incomplete."
                    .into(),
            );
        }
        if !formula_complete {
            warnings.push("Some formula cells could not be evaluated; their errors and subtotal coverage are retained.".into());
        }
        if !verified_measurements {
            warnings.push(
                "Historical or stale measurements are excluded from verified quantity totals."
                    .into(),
            );
        }
        if legends.values().any(|l| l.style.is_none()) {
            warnings.push("Some legend styles are unknown. Their annotations remain counted and listed without an invented symbol.".into());
        }
        let properties = self.properties();
        if properties.encrypted {
            warnings.push("This standalone report is not encrypted. The source PDF retains its original encryption and permissions.".into());
        }
        if self.signature_count() > 0 {
            warnings.push(
                "Source signatures are not copied or represented as signatures of this report."
                    .into(),
            );
        }
        let mut model = IssueReportModel {
            schema: "sunipdf-issue-report",
            version: if quantities["version"] == 2 { 2 } else { 1 },
            title: options.title.clone(),
            selection: options.selection,
            provenance: ReportProvenance {
                snapshot_sha256: table.source_sha256().into(),
                snapshot_hash_kind: "exact_serialized_snapshot_not_canonical_file_identity",
                saved_file_sha256: source.map(|s| s.sha256.clone()),
                saved_file_name: source
                    .and_then(|s| s.selected.file_name())
                    .map(|s| s.to_string_lossy().into_owned()),
                source_encrypted: properties.encrypted,
                source_signature_count: self.signature_count(),
                report_encrypted: false,
                source_unchanged: true,
            },
            complete: quantities["complete"] == true
                && zone_complete
                && formula_complete
                && verified_measurements,
            quantities,
            legend_included: options.include_legend,
            legend: legends.into_values().collect(),
            issue_groups: groups.into_values().collect(),
            warnings,
            page_count: 0,
            pdf_font: String::new(),
            pdf_font_embedding: "pending",
        };
        let group_count = model.quantities["summary"]["groups"]
            .as_array()
            .map_or(0, Vec::len)
            + model.quantities["summary"]["formula_groups"]
                .as_array()
                .map_or(0, Vec::len);
        if group_count > MAX_GROUPS {
            return Err(invalid(
                "more than 256 quantity/formula groups; narrow the filter",
            ));
        }
        let (pdf, pages, font_name) =
            crate::issue_report_render::render(self.pdfium(), &model, cancel, &mut progress)?;
        model.page_count = pages;
        model.pdf_font = font_name;
        model.pdf_font_embedding = if model.pdf_font.starts_with("Ubuntu") {
            "complete original font program; copyright and licence retained in report metadata"
        } else {
            "subset and identity-CID mapping of the original font; copyright and licence retained in report metadata"
        };
        let json = serde_json::to_vec_pretty(&model).map_err(|e| invalid(&e.to_string()))?;
        let csv = csv(&model, cancel)?.into_bytes();
        if [pdf.len(), json.len(), csv.len()]
            .into_iter()
            .any(|n| n > MAX_REPORT_BYTES)
        {
            return Err(invalid("report exceeds 32 MiB; narrow the filter"));
        }
        let prepared = PreparedIssueReport {
            original,
            source: source.cloned(),
            model,
            pdf,
            csv,
            json,
        };
        self.validate_issue_report(&prepared, cancel)?;
        progress(IssueReportProgress {
            phase: IssueReportPhase::Ready,
            completed: pages,
            total: pages,
        });
        cancelled(cancel)?;
        Ok(prepared)
    }

    fn validate_issue_report(
        &self,
        prepared: &PreparedIssueReport,
        cancel: &AtomicBool,
    ) -> Result<(), EngineError> {
        cancelled(cancel)?;
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        let current = self.snapshot()?;
        if current.len() > MAX_SOURCE_BYTES
            || !crate::annot_rich::same_document(&prepared.original, &current, self.password())?
        {
            return Err(invalid("document changed; prepare the report again"));
        }
        if let Some(source) = &prepared.source {
            source.revalidate(cancel)?;
        }
        cancelled(cancel)
    }

    pub fn render_issue_report_preview(
        &self,
        prepared: &PreparedIssueReport,
        page_index: usize,
        long_side: u32,
        cancel: &AtomicBool,
    ) -> Result<RenderedPage, EngineError> {
        self.validate_issue_report(prepared, cancel)?;
        if page_index >= prepared.page_count() {
            return Err(EngineError::PageOutOfRange(page_index));
        }
        if !(64..=1536).contains(&long_side) {
            return Err(invalid("preview side must be 64..1536 pixels"));
        }
        let inner = self
            .pdfium()
            .load_pdf_from_byte_vec(prepared.pdf.clone(), None)?;
        let page = inner.pages().get(page_index as i32)?;
        let scale = long_side as f32 / page.width().value.max(page.height().value);
        drop(page);
        let mut info = self.info.clone();
        info.page_sizes = vec![
            crate::PageSize {
                width_pt: 595.0,
                height_pt: 842.0
            };
            prepared.page_count()
        ];
        info.page_labels = vec![None; prepared.page_count()];
        info.form = crate::FormKind::None;
        let scratch = Document {
            inner,
            pdfium: self.pdfium(),
            info,
            page_rotations: vec![None; prepared.page_count()],
            password: None,
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        let image = scratch.render_page(page_index, scale)?;
        self.validate_issue_report(prepared, cancel)?;
        Ok(image)
    }

    /// Atomic no-clobber publication. Cancellation after this commit point cannot revoke a file.
    pub fn publish_issue_report(
        &self,
        prepared: &PreparedIssueReport,
        format: IssueReportFormat,
        output: &Path,
        cancel: &AtomicBool,
    ) -> Result<IssueReportReceipt, EngineError> {
        self.validate_issue_report(prepared, cancel)?;
        if output.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(output.into()));
        }
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let canonical = parent.canonicalize()?;
        let name = output
            .file_name()
            .ok_or_else(|| invalid("output requires a filename"))?;
        let bytes = match format {
            IssueReportFormat::Pdf => &prepared.pdf,
            IssueReportFormat::Csv => &prepared.csv,
            IssueReportFormat::Json => &prepared.json,
        };
        let mut file = tempfile::NamedTempFile::new_in(&canonical)?;
        for chunk in bytes.chunks(64 * 1024) {
            cancelled(cancel)?;
            file.write_all(chunk)?;
        }
        file.as_file().sync_all()?;
        self.validate_issue_report(prepared, cancel)?;
        if parent.canonicalize()? != canonical {
            return Err(invalid("destination directory changed"));
        }
        cancelled(cancel)?;
        file.persist_noclobber(canonical.join(name)).map_err(|e| {
            if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                EngineError::AlreadyExists(output.into())
            } else {
                EngineError::Io(e.error)
            }
        })?;
        Ok(IssueReportReceipt {
            out: output.into(),
            format,
            output_sha256: digest(bytes),
            bytes: bytes.len(),
            page_count: prepared.page_count(),
            matched_annotations: prepared.model.quantities["rows"]
                .as_array()
                .map_or(0, Vec::len),
            complete: prepared.model.complete,
            provenance: prepared.model.provenance.clone(),
            warnings: prepared.model.warnings.clone(),
        })
    }
}

fn csv(model: &IssueReportModel, cancel: &AtomicBool) -> Result<String, EngineError> {
    let headers = [
        "record_type",
        "snapshot_sha256",
        "saved_file_sha256",
        "report_title",
        "page",
        "index",
        "annotation_id",
        "kind",
        "author",
        "subject",
        "contents",
        "review_status",
        "business_status",
        "discipline",
        "count_group",
        "zone_status",
        "zone_id",
        "zone_path",
        "quantity",
        "unit",
        "unit_price",
        "price_unit",
        "currency",
        "line_total",
        "details_json",
    ];
    let mut result = headers.join(",") + "\r\n";
    let mut emit = |kind: &str, value: &Value, row: bool| -> Result<(), EngineError> {
        cancelled(cancel)?;
        let mut cells = vec![String::new(); headers.len()];
        cells[0] = kind.into();
        cells[1] = model.provenance.snapshot_sha256.clone();
        cells[2] = model
            .provenance
            .saved_file_sha256
            .clone()
            .unwrap_or_default();
        cells[3] = model.title.clone();
        if row {
            for (i, key) in [
                "page",
                "index",
                "annotation_id",
                "kind",
                "author",
                "subject",
                "contents",
                "review_status",
            ]
            .iter()
            .enumerate()
            {
                cells[4 + i] = display(&value["register"][*key]);
            }
            cells[12] = display(&value["register"]["fields"]["business_status"]);
            cells[13] = display(&value["register"]["fields"]["discipline"]);
            cells[14] = display(&value["register"]["count_group"]);
            for (i, key) in ["status", "zone_id", "path"].iter().enumerate() {
                cells[15 + i] = display(&value["register"]["zone"][*key]);
            }
            cells[18] = display(&value["quantity"]["value"]);
            cells[19] = display(&value["quantity"]["unit"]);
            cells[20] = display(&value["fields"]["unit_price"]["amount"]);
            cells[21] = display(&value["fields"]["unit_price"]["per_unit"]);
            cells[22] = display(&value["fields"]["unit_price"]["currency"]);
            cells[23] = display(&value["line_total"]["amount"]);
        }
        cells[24] = serde_json::to_string(value).map_err(|e| invalid(&e.to_string()))?;
        result.push_str(
            &cells
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    if [4, 5, 18, 20, 23].contains(&i)
                        && !s.is_empty()
                        && s.parse::<f64>().is_ok_and(f64::is_finite)
                    {
                        s.clone()
                    } else {
                        crate::annotation_register::csv_cell(s)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        result.push_str("\r\n");
        if result.len() > MAX_REPORT_BYTES {
            return Err(invalid("CSV exceeds 32 MiB"));
        }
        Ok(())
    };
    let mut metadata = model.json();
    metadata
        .as_object_mut()
        .expect("report")
        .remove("quantities");
    metadata.as_object_mut().expect("report").remove("legend");
    metadata
        .as_object_mut()
        .expect("report")
        .remove("issue_groups");
    let mut options = model.quantities.clone();
    options.as_object_mut().expect("quantities").remove("rows");
    options
        .as_object_mut()
        .expect("quantities")
        .remove("summary");
    options
        .as_object_mut()
        .expect("quantities")
        .remove("formula_rows");
    emit("report", &metadata, false)?;
    emit("options", &options, false)?;
    for row in model.quantities["rows"]
        .as_array()
        .ok_or_else(|| invalid("missing report rows"))?
    {
        emit("annotation", row, true)?;
    }
    emit("quantity_summary", &model.quantities["summary"], false)?;
    for row in model.quantities["formula_rows"]
        .as_array()
        .into_iter()
        .flatten()
    {
        emit("formula", row, false)?;
    }
    for legend in &model.legend {
        emit(
            "legend",
            &serde_json::to_value(legend).expect("legend"),
            false,
        )?;
    }
    for group in &model.issue_groups {
        emit(
            "issue_group",
            &serde_json::to_value(group).expect("issue group"),
            false,
        )?;
    }
    Ok(result)
}
pub(crate) fn display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
