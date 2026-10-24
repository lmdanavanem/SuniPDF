//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.
//! Cache LRU partagé (tuiles, miniatures), file de tuiles voulues remplacée en bloc par l'UI
//! (`SetWanted`) et rendue une tuile à la fois entre deux requêtes, journal d'annulation par
//! instantanés (octets PDF) pour toutes les modifications de pages.

use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter, TryRecvError};
use std::sync::Arc;
use sunipdf_core::{
    form_data, security, write_new_copy, AnnotDetail, AnnotShape, AnnotStyle, AnnotationInfo,
    Bookmark, Document, DocumentInfo, Engine, EngineError, FieldValue, FillMode, FormInfo,
    FreeTextStyle, LinkInfo, LinkTarget, MarkupKind, Metadata, ObjectClip, ObjectInfo, ObjectStyle,
    PageLabelRange, PageMargins, PointRect, Properties, Protection, RectMode, RedactMark,
    RedactOptions, RedactReport, RenderCache, RenderedPage, ResizeMode, ReviewStatus, ShapeKind,
    ShapeStyle, Stamp, SystemFont, TextMatch, TextSelection, TextStyle, TileJob, ZOrder,
};

/// Plafond du cache de rendu. constante ; réglage utilisateur quand un besoin réel apparaîtra.
const CACHE_BYTES: usize = 256 * 1024 * 1024;
/// Plafonds du journal d'annulation (instantanés complets du PDF).
const UNDO_MAX_ENTRIES: usize = 50;
const UNDO_MAX_BYTES: usize = 512 * 1024 * 1024;

pub(crate) const CERTIFICATE_SOURCE_CHANGED: &str =
    "certificate source changed; reload the document";
pub(crate) const CERTIFICATE_SAVE_FIRST: &str = "save changes before signing or verifying";
pub(crate) const RECTANGLE_REFERENCE_REQUIRED: &str =
    "dimensioned rectangle: current local reference required";
pub(crate) const RECTANGLE_CAPTURE_FAILED: &str =
    "dimensioned rectangle: local reference capture failed: ";

/// Exact bytes last opened or saved, independent of PDFium's rewritten snapshots.
#[derive(Clone)]
pub struct SignatureSource {
    bytes: Arc<[u8]>,
    sha256: [u8; 32],
}

impl SignatureSource {
    pub(crate) fn sha256_hex(&self) -> String {
        self.sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub(crate) fn matches_sha256(&self, expected: &str) -> bool {
        self.sha256_hex().eq_ignore_ascii_case(expected)
    }
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            sha256: Sha256::digest(&bytes).into(),
            bytes: bytes.into(),
        }
    }

    fn checked_bytes(&self, path: &std::path::Path, modified: bool) -> Result<&[u8], EngineError> {
        if modified {
            return Err(EngineError::Other(CERTIFICATE_SAVE_FIRST.into()));
        }
        let changed = || EngineError::Other(CERTIFICATE_SOURCE_CHANGED.into());
        let file = std::fs::File::open(path).map_err(|_| changed())?;
        if file.metadata().map_err(|_| changed())?.len() != self.bytes.len() as u64 {
            return Err(changed());
        }
        let mut current = file.take(self.bytes.len() as u64 + 1);
        let mut hasher = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        let mut length = 0;
        loop {
            let count = current.read(&mut buffer).map_err(|_| changed())?;
            if count == 0 {
                break;
            }
            length += count;
            hasher.update(&buffer[..count]);
        }
        let digest: [u8; 32] = hasher.finalize().into();
        if length != self.bytes.len() || digest != self.sha256 {
            return Err(EngineError::Other(CERTIFICATE_SOURCE_CHANGED.into()));
        }
        // Only the captured revision is ever signed or verified, even if disk changes after checking.
        Ok(&self.bytes)
    }
}

/// Document suspendu (onglet inactif) : octets, mot de passe, chemin, historique d'annulation.
pub struct SuspendedDoc {
    pub bytes: Vec<u8>,
    pub password: Option<String>,
    pub path: PathBuf,
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
    /// Point d'enregistrement (voir `State::save_point`).
    pub save_point: Option<usize>,
    /// Modifié depuis l'ouverture ou le dernier enregistrement.
    pub modified: bool,
    pub signature_source: Option<SignatureSource>,
}

/// Instantané d'annulation : octets du document et mot de passe nécessaire pour les recharger
/// (il change avec Protéger / Retirer la protection).
pub struct Snapshot {
    bytes: Vec<u8>,
    password: Option<String>,
    pages_unchanged: bool,
    measurement_scales_unchanged: bool,
}

/// Tâche de fichiers (non annulable : produit de nouveaux fichiers, le document ouvert est intact).
pub enum BatchOp {
    Combine {
        inputs: Vec<PathBuf>,
        out: PathBuf,
    },
    Split {
        every: usize,
        out_dir: PathBuf,
    },
    Extract {
        pages: Vec<usize>,
        out: PathBuf,
    },
    CreateFromImages {
        images: Vec<PathBuf>,
        out: PathBuf,
    },
    ExportImages {
        out_dir: PathBuf,
        dpi: f32,
    },
    Optimize {
        out: PathBuf,
    },
    Sanitize {
        out: PathBuf,
    },
    /// Valeurs du formulaire en JSON (`{ nom: valeur }`) ; le document est intact.
    ExportFormData {
        out: PathBuf,
    },
}

pub enum BatchResult {
    Combined {
        out: PathBuf,
        pages: usize,
    },
    Split {
        files: Vec<PathBuf>,
        encrypted: bool,
    },
    Extracted {
        out: PathBuf,
        pages: usize,
        encrypted: bool,
    },
    Created {
        out: PathBuf,
        pages: usize,
    },
    Exported {
        files: Vec<PathBuf>,
    },
    Optimized {
        out: PathBuf,
        before: u64,
        after: u64,
    },
    Sanitized {
        out: PathBuf,
        report: sunipdf_core::sanitize::SanitizeReport,
    },
    FormExported {
        out: PathBuf,
        count: usize,
    },
    Failed(EngineError),
}

/// Transformation d'octets (lopdf) : chiffrement, métadonnées.
pub enum BytesOp {
    Bookmark(sunipdf_core::bookmarks::BookmarkEdit),
    Protect {
        protection: Protection,
        current_owner: Option<zeroize::Zeroizing<String>>,
    },
    /// Mot de passe propriétaire saisi par l'utilisateur.
    Unprotect(String),
    SetMetadata(Metadata),
    StripMetadata,
}

/// Modification de pages, appliquée sous instantané (annulable).
pub enum EditOp {
    CommitBoundaryFill {
        epoch: u64,
        generation: u64,
    },
    PreparedBoundaryFill(Box<sunipdf_core::boundary_fill::PreparedBoundaryFillMeasurement>),
    CommitVisualSearchCounts {
        epoch: u64,
        generation: u64,
        ids: Vec<u32>,
        group: String,
        size: f32,
    },
    PreparedVisualSearchCounts(sunipdf_core::visual_search::PreparedVisualSearchCounts),
    CommitPlanZones {
        epoch: u64,
        generation: u64,
    },
    PreparedPlanZones(sunipdf_core::plan_zones::PreparedPlanZones),
    SetPageLabels(PageLabelRange),
    ResetPageLabels,
    AddFormField {
        page: usize,
        name: String,
        kind: sunipdf_core::FieldKind,
        rect: PointRect,
        options: Vec<sunipdf_core::FieldOption>,
        multi_select: bool,
        editable: bool,
    },
    Rotate {
        pages: Vec<usize>,
        delta_degrees: i32,
    },
    Delete {
        pages: Vec<usize>,
    },
    Insert {
        source: PathBuf,
        password: Option<String>,
        pages: Vec<usize>,
        at: usize,
    },
    Move {
        from: usize,
        to: usize,
    },
    Markup {
        page: usize,
        kind: MarkupKind,
        rects: Vec<PointRect>,
        rgb: (u8, u8, u8),
    },
    Note {
        page: usize,
        x: f32,
        y: f32,
        text: String,
    },
    DeleteAnnotation {
        page: usize,
        index: usize,
    },
    Bytes(BytesOp),
    AddText {
        page: usize,
        x: f32,
        y: f32,
        text: String,
        style: TextStyle,
    },
    SetText {
        page: usize,
        index: usize,
        text: Option<String>,
        size: Option<f32>,
        rgb: Option<(u8, u8, u8)>,
    },
    ReflowParagraph {
        page: usize,
        indices: Vec<usize>,
        text: String,
        width: f32,
        leading: f32,
    },
    CommitRichAnnotationPreview {
        epoch: u64,
        generation: u64,
    },
    SetPageCalibration {
        page: usize,
        calibration: Option<sunipdf_core::measurement::PageCalibration>,
    },
    ApplyRecalibration(sunipdf_core::measurement::RecalibrationPlan),
    SetMeasurementViewports {
        page: usize,
        expected: Vec<sunipdf_core::measurement::MeasurementViewport>,
        replacement: Vec<sunipdf_core::measurement::MeasurementViewport>,
    },
    ClearMeasurementViewports(sunipdf_core::measurement::MeasurementViewportResetTarget),
    PreparedRichAnnotation(sunipdf_core::annot_rich::PreparedRichAnnotation),
    ResizeObjects {
        page: usize,
        indices: Vec<usize>,
        bounds: PointRect,
    },
    RotateObjects {
        page: usize,
        indices: Vec<usize>,
        degrees: f32,
    },
    SetObjectStyle {
        page: usize,
        indices: Vec<usize>,
        style: ObjectStyle,
    },
    MoveObjects {
        page: usize,
        indices: Vec<usize>,
        dx: f32,
        dy: f32,
    },
    /// Décalage au clavier : les décalages consécutifs des mêmes objets partagent une annulation.
    NudgeObjects {
        page: usize,
        indices: Vec<usize>,
        dx: f32,
        dy: f32,
    },
    DeleteObjects {
        page: usize,
        indices: Vec<usize>,
    },
    AddImage {
        page: usize,
        path: PathBuf,
        x: f32,
        y: f32,
        width_pt: f32,
        opacity: f32,
    },
    /// Image déjà décodée (signature visuelle) ; `Arc` : l'interface garde l'image pour la reposer.
    AddImageData {
        page: usize,
        image: Arc<image::DynamicImage>,
        x: f32,
        y: f32,
        width_pt: f32,
    },
    AddShape {
        page: usize,
        kind: ShapeKind,
        rect: PointRect,
        style: ShapeStyle,
    },
    DuplicateObjects {
        page: usize,
        indices: Vec<usize>,
    },
    ReorderObjects {
        page: usize,
        indices: Vec<usize>,
        op: ZOrder,
    },
    /// Colle le presse-papiers d'objets du moteur (décalage en points d'affichage).
    PasteObjects {
        page: usize,
        dx: f32,
        dy: f32,
    },
    PasteEditableObjects {
        page: usize,
        dx: f32,
        dy: f32,
    },
    ReplaceImage {
        page: usize,
        index: usize,
        path: PathBuf,
    },
    CropPages {
        pages: Vec<usize>,
        margins: PageMargins,
    },
    ResetCrop {
        pages: Vec<usize>,
    },
    ResizePages {
        pages: Vec<usize>,
        width_pt: f32,
        height_pt: f32,
        mode: ResizeMode,
    },
    InsertBlankPages {
        at: usize,
        count: usize,
        width_pt: f32,
        height_pt: f32,
    },
    HeadersFooters {
        pages: Option<Vec<usize>>,
        options: sunipdf_core::header_footer::HeaderFooter,
    },
    ImageWatermark {
        pages: Option<Vec<usize>>,
        path: PathBuf,
        options: sunipdf_core::image_watermark::ImageWatermark,
    },
    Stamp {
        pages: Option<Vec<usize>>,
        stamp: Stamp,
    },
    /// Valeurs de champs (mode strict : tout ou rien).
    FillForm {
        values: Vec<(String, FieldValue)>,
    },
    ClearForm,
    /// Champs et annotations intégrés au contenu (`None` = toutes les pages).
    FlattenForm {
        pages: Option<Vec<usize>>,
    },
    /// Annotations avancées (vraies annotations PDF, écrites par lopdf : `annot_write.rs`).
    AddInk {
        page: usize,
        strokes: Vec<Vec<(f32, f32)>>,
        style: AnnotStyle,
        author: Option<String>,
    },
    AddAnnotShape {
        page: usize,
        shape: AnnotShape,
        rect: PointRect,
        style: AnnotStyle,
        author: Option<String>,
    },
    AddFreeText {
        page: usize,
        rect: PointRect,
        text: String,
        style: FreeTextStyle,
        author: Option<String>,
    },
    AddLink {
        page: usize,
        rect: PointRect,
        target: LinkTarget,
    },
    SetLinkTarget {
        page: usize,
        index: usize,
        target: LinkTarget,
    },
    MoveAnnotation {
        page: usize,
        index: usize,
        dx: f32,
        dy: f32,
    },
    ResizeAnnotation {
        page: usize,
        index: usize,
        rect: PointRect,
    },
    StyleAnnotation {
        page: usize,
        index: usize,
        style: AnnotStyle,
    },
    SetAnnotationText {
        page: usize,
        index: usize,
        text: String,
    },
    AddCountMarker {
        page: usize,
        point: (f32, f32),
        size: f32,
        group: String,
        author: Option<String>,
    },
    SetQuantityFields {
        target: sunipdf_core::quantity_table::QuantityTarget,
        fields: sunipdf_core::quantity_table::QuantityFields,
    },
    SetAnnotationRegisterFields {
        target: sunipdf_core::annotation_register::RegisterTarget,
        fields: sunipdf_core::annotation_register::RegisterFields,
        count_group: Option<String>,
    },
    UpdateFreeText {
        page: usize,
        index: usize,
        text: String,
        style: FreeTextStyle,
    },
    DeleteAnnotationThread {
        page: usize,
        index: usize,
    },
    Reply {
        page: usize,
        parent: usize,
        text: String,
        author: Option<String>,
    },
    SetReviewStatus {
        page: usize,
        parent: usize,
        status: ReviewStatus,
        author: Option<String>,
    },
}

impl EditOp {
    /// Session calibration is valid only while both page geometry and plan content are unchanged.
    fn keeps_measurement_scales(&self) -> bool {
        matches!(
            self,
            Self::CommitBoundaryFill { .. }
                | Self::PreparedBoundaryFill(_)
                | Self::CommitVisualSearchCounts { .. }
                | Self::PreparedVisualSearchCounts(_)
                | Self::CommitPlanZones { .. }
                | Self::PreparedPlanZones(_)
                | Self::SetPageLabels(_)
                | Self::ResetPageLabels
                | Self::AddFormField { .. }
                | Self::Markup { .. }
                | Self::Note { .. }
                | Self::DeleteAnnotation { .. }
                | Self::Bytes(
                    BytesOp::Bookmark(_)
                        | BytesOp::Protect { .. }
                        | BytesOp::Unprotect(_)
                        | BytesOp::SetMetadata(_)
                        | BytesOp::StripMetadata,
                )
                | Self::SetPageCalibration { .. }
                | Self::PreparedRichAnnotation(_)
                | Self::CommitRichAnnotationPreview { .. }
                | Self::FillForm { .. }
                | Self::ClearForm
                | Self::AddInk { .. }
                | Self::AddAnnotShape { .. }
                | Self::AddFreeText { .. }
                | Self::AddLink { .. }
                | Self::SetLinkTarget { .. }
                | Self::MoveAnnotation { .. }
                | Self::ResizeAnnotation { .. }
                | Self::StyleAnnotation { .. }
                | Self::SetAnnotationText { .. }
                | Self::AddCountMarker { .. }
                | Self::SetAnnotationRegisterFields { .. }
                | Self::SetQuantityFields { .. }
                | Self::UpdateFreeText { .. }
                | Self::DeleteAnnotationThread { .. }
                | Self::Reply { .. }
                | Self::SetReviewStatus { .. }
        )
    }

    /// Vrai si l'opération ne change ni le nombre, ni l'ordre, ni la taille des pages (objets,
    /// annotations, tampon) : l'UI garde alors l'image affichée jusqu'à l'arrivée des tuiles neuves.
    pub fn keeps_pages(&self) -> bool {
        !matches!(
            self,
            EditOp::Rotate { .. }
                | EditOp::Delete { .. }
                | EditOp::Insert { .. }
                | EditOp::Move { .. }
                | EditOp::Bytes(_)
                | EditOp::CropPages { .. }
                | EditOp::ResetCrop { .. }
                | EditOp::ResizePages { .. }
                | EditOp::InsertBlankPages { .. }
        )
    }
}

/// Tuiles à rendre, remplacée en bloc à chaque `SetWanted` : c'est l'« annulation » des rendus
/// des pages sorties de l'écran (une tuile déjà commencée va jusqu'au bout).
#[derive(Default)]
struct WantedQueue {
    epoch: u64,
    jobs: VecDeque<TileJob>,
}

impl WantedQueue {
    fn replace(&mut self, epoch: u64, jobs: Vec<TileJob>) {
        self.epoch = epoch;
        self.jobs = jobs.into();
    }

    fn pop(&mut self) -> Option<(u64, TileJob)> {
        self.jobs.pop_front().map(|j| (self.epoch, j))
    }

    fn clear(&mut self) {
        self.jobs.clear();
    }
}

/// Objets à décrire (sélection).
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    At {
        x: f32,
        y: f32,
    },
    InRect {
        rect: PointRect,
        touching: bool,
    },
    Indices(Vec<usize>),
    /// Les `n` derniers objets de la page (ajoutés ou passés au premier plan).
    Last(usize),
    All,
}

/// Fusion de la réponse avec la sélection courante (faite par l'interface).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickMode {
    Replace,
    Toggle,
    Add,
}

pub enum Request {
    BoundaryFill {
        epoch: u64,
        generation: u64,
        job: crate::boundary_fill_ui::Job,
    },
    VisualSearch {
        epoch: u64,
        generation: u64,
        job: crate::visual_search_ui::Job,
    },
    QuantityWorkbook {
        epoch: u64,
        generation: u64,
        job: crate::quantity_workbook_ui::Job,
    },
    IssueReport {
        epoch: u64,
        generation: u64,
        job: crate::issue_report_ui::Job,
    },
    PlanZones {
        epoch: u64,
        generation: u64,
        job: crate::plan_zones_ui::Job,
    },
    RevisionOutlines {
        generation: u64,
        job: crate::revision_outlines_ui::Job,
    },
    PlanLinks {
        generation: u64,
        job: crate::plan_links_ui::Job,
    },
    RevisionTransfer {
        generation: u64,
        job: crate::revision_transfer_ui::Job,
    },
    PlanSet {
        generation: u64,
        job: crate::plan_sets::Job,
    },
    LocalReview(crate::local_review::Job),
    RevisionOverlay {
        generation: u64,
        job: crate::revision_overlay::Job,
    },
    Office(crate::office::Job),
    ProcessFiles(crate::processing::ProcessingJob),
    Print {
        job: crate::printing::PrintJob,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    Ocr {
        options: sunipdf_core::ocr::OcrOptions,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    RenderSignature {
        id: u64,
        text: String,
        style: TextStyle,
    },
    PlaceSignature {
        pages: Vec<usize>,
        image: Arc<image::DynamicImage>,
        center: (f32, f32),
        width_pt: f32,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    VerifySignatures,
    BeginProtection {
        epoch: u64,
    },
    VerifySignaturesWithOptions(sunipdf_core::certificate::VerificationOptions),
    SignCertificate {
        pfx: PathBuf,
        password: zeroize::Zeroizing<String>,
        options: sunipdf_core::certificate::CertSignOptions,
        output: PathBuf,
    },
    SetRecovery(Option<crate::recovery::Slot>),
    Autosave,
    MarkRecovered {
        path: PathBuf,
    },
    AcknowledgeSuspendFailure,
    Open {
        path: PathBuf,
        password: Option<String>,
    },
    OpenPlanSheet(crate::plan_sets::Navigation),
    /// Remplace la liste des tuiles voulues (priorité décroissante) ; `epoch` revient dans `Tile`.
    SetWanted {
        epoch: u64,
        jobs: Vec<TileJob>,
    },
    /// Miniature de `width_px` pixels de large (hauteur proportionnelle).
    Thumbnail {
        page: usize,
        width_px: u32,
    },
    Search {
        query: String,
        match_case: bool,
        generation: u64,
    },
    CancelSearch {
        generation: u64,
    },
    Edit(EditOp),
    TextInRect {
        page: usize,
        rect: PointRect,
    },
    ReadingSelection {
        epoch: u64,
        generation: u64,
        anchor: sunipdf_core::annot::ReadingPoint,
        focus: sunipdf_core::annot::ReadingPoint,
        word: bool,
    },
    /// Zones de caviardage pour chaque occurrence d'un texte (espace d'affichage). `verify` :
    /// contrôle après application (les occurrences restantes sont signalées, pas ajoutées).
    RedactSearch {
        query: String,
        match_case: bool,
        verify: bool,
    },
    /// Prepare a verified candidate without changing the document; commit uses its exact bytes.
    PreviewRedaction {
        marks: Vec<RedactMark>,
        options: RedactOptions,
        appearance: sunipdf_core::redact::RedactAppearance,
        page: usize,
        epoch: u64,
        generation: u64,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    DiscardRedactionPreview {
        epoch: u64,
        generation: u64,
    },
    CommitRedactionPreview {
        epoch: u64,
        generation: u64,
    },
    PersistRedactionMarks {
        marks: Vec<RedactMark>,
        appearance: sunipdf_core::redact::RedactAppearance,
        author: Option<String>,
    },
    GetRedactionMarks {
        epoch: u64,
    },
    /// Annotations d'une page : liste rapide PDFium et, si `detailed` et si la page porte autre chose
    /// que des liens, fenêtres et champs, lecture détaillée (droits, fils de réponses).
    Annotations {
        page: usize,
        detailed: bool,
    },
    LibraryCaptureCandidates {
        page: usize,
        epoch: u64,
        generation: u64,
    },
    CaptureLibraryEntry {
        page: usize,
        selections: Vec<sunipdf_core::annotation_library::LibraryCaptureSelection>,
        id: String,
        name: String,
        provenance: sunipdf_core::annotation_library::LibraryProvenance,
        epoch: u64,
        generation: u64,
    },
    QuantityTable {
        epoch: u64,
        generation: u64,
    },
    ExportQuantityTable {
        table: Arc<sunipdf_core::quantity_table::QuantityTable>,
        options: sunipdf_core::quantity_table::QuantityOptions,
        path: PathBuf,
        csv: bool,
        epoch: u64,
        generation: u64,
    },
    AnnotationRegister {
        epoch: u64,
        generation: u64,
    },
    CancelAnnotationRegister {
        epoch: u64,
        generation: u64,
    },
    MeasurementPage {
        page: usize,
        epoch: u64,
        generation: u64,
    },
    PreviewRecalibration {
        requests: Vec<sunipdf_core::measurement::RecalibrationRequest>,
        epoch: u64,
        generation: u64,
    },
    /// Zones de lien de la page (cliquables dans la vue).
    Links {
        page: usize,
    },
    /// Champs du formulaire du document actif (`FormInfo::default()` s'il n'en a pas).
    FormInfo,
    Properties,
    Inspect {
        epoch: u64,
    },
    Bookmarks,
    ObjectAt {
        page: usize,
        x: f32,
        y: f32,
    },
    ObjectInfo {
        page: usize,
        index: usize,
    },
    Pick {
        page: usize,
        pick: Pick,
        mode: PickMode,
    },
    /// Liste des objets d'une page (panneau Objets, prise directe au glisser).
    Objects {
        page: usize,
    },
    RotationCenter {
        page: usize,
        indices: Vec<usize>,
        epoch: u64,
        generation: u64,
    },
    PreviewParagraph {
        page: usize,
        indices: Vec<usize>,
        text: String,
        width: f32,
        leading: f32,
        epoch: u64,
        generation: u64,
    },
    PreviewRichAnnotation {
        page: usize,
        annotation: sunipdf_core::annot_rich::RichAnnotation,
        author: Option<String>,
        epoch: u64,
        generation: u64,
    },
    PreviewMeasurementEdit {
        page: usize,
        record: sunipdf_core::measurement::MeasurementRecord,
        measurement: sunipdf_core::measurement::Measurement,
        epoch: u64,
        generation: u64,
    },
    PreviewMeasurementRecalibration {
        page: usize,
        record: sunipdf_core::measurement::MeasurementRecord,
        measurement: sunipdf_core::measurement::Measurement,
        calibration: sunipdf_core::measurement::PageCalibration,
        epoch: u64,
        generation: u64,
    },
    CaptureRectangleCalibration {
        page: usize,
        calibration: sunipdf_core::measurement::PageCalibration,
    },
    PreviewDimensionedRectangle {
        page: usize,
        rectangle: crate::measurement_shapes_ui::RectanglePlacement,
        author: Option<String>,
        epoch: u64,
        generation: u64,
    },
    DiscardRichAnnotationPreview {
        epoch: u64,
        generation: u64,
    },
    /// Copie dans le presse-papiers d'objets du moteur (document inchangé, pas d'annulation).
    /// `cut` : les objets sont supprimés aussitôt après une copie réussie, sans qu'une autre
    /// requête puisse décaler leurs index entre les deux (`Copied` puis `DocumentChanged`).
    CopyObjects {
        page: usize,
        indices: Vec<usize>,
        cut: bool,
    },
    SystemFonts,
    /// Décode une image choisie par l'utilisateur (signature importée) hors du thread d'interface ;
    /// aucun document requis.
    LoadImage {
        path: PathBuf,
        max_side: u32,
    },
    Batch(BatchOp),
    /// Suspend le document actif (renvoie `Suspended`).
    Suspend,
    /// Annonce la reprise de l'onglet `usize` ; le document suit dans `ResumeDoc`.
    Resume(usize),
    ResumeDoc(SuspendedDoc),
    Undo,
    Redo,
    Save {
        path: PathBuf,
    },
    FinishMutation,
    Close,
}

#[derive(Clone)]
pub struct DocState {
    pub info: DocumentInfo,
    pub can_undo: bool,
    pub can_redo: bool,
    pub modified: bool,
    /// Modification de contenu seulement (`EditOp::keeps_pages`) : pages identiques en nombre,
    /// ordre et taille.
    pub pages_unchanged: bool,
    /// True only for changes that preserve page geometry and underlying plan content.
    pub measurement_scales_unchanged: bool,
    pub has_signatures: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    Print,
    Ocr,
    Signature,
}

pub enum Response {
    BoundaryFill {
        epoch: u64,
        generation: u64,
        result: Result<crate::boundary_fill_ui::Outcome, EngineError>,
    },
    VisualSearch {
        epoch: u64,
        generation: u64,
        result: Result<crate::visual_search_ui::Outcome, EngineError>,
    },
    QuantityWorkbook {
        epoch: u64,
        generation: u64,
        result: Result<crate::quantity_workbook_ui::Outcome, EngineError>,
    },
    QuantityWorkbookProgress {
        epoch: u64,
        generation: u64,
        progress: sunipdf_core::quantity_workbook::WorkbookProgress,
    },
    IssueReport {
        epoch: u64,
        generation: u64,
        result: Result<crate::issue_report_ui::Outcome, EngineError>,
    },
    IssueReportProgress {
        epoch: u64,
        generation: u64,
        progress: sunipdf_core::issue_report::IssueReportProgress,
    },
    PlanZones {
        epoch: u64,
        generation: u64,
        result: Result<Option<crate::plan_zones_ui::PageData>, EngineError>,
    },
    RevisionOutlinesProgress {
        generation: u64,
        progress: sunipdf_core::revision_outlines::OutlineProgress,
    },
    RevisionOutlinesDone {
        generation: u64,
        result: Box<Result<crate::revision_outlines_ui::Outcome, EngineError>>,
    },
    PlanLinksDone {
        generation: u64,
        result: Result<crate::plan_links_ui::Outcome, EngineError>,
    },
    RevisionTransferDone {
        generation: u64,
        result: Result<crate::revision_transfer_ui::Outcome, EngineError>,
    },
    PlanSetProgress {
        generation: u64,
        done: usize,
        total: usize,
    },
    PlanSetDone {
        generation: u64,
        result: Result<crate::plan_sets::Outcome, EngineError>,
    },
    LocalReviewProgress {
        done: usize,
        total: usize,
        page: usize,
        pages: usize,
    },
    LocalReviewDone(Result<crate::local_review::Outcome, EngineError>),
    RevisionOverlayDone {
        generation: u64,
        result: Result<crate::revision_overlay::Outcome, EngineError>,
    },
    ProtectionAuthorization {
        epoch: u64,
        encrypted: bool,
    },
    OfficeProgress {
        done: usize,
        total: usize,
        item_done: usize,
        item_total: usize,
    },
    OfficeDone(Vec<crate::office::FileResult>),
    ProcessingProgress {
        done: usize,
        total: usize,
        item_done: usize,
        item_total: usize,
    },
    ProcessingDone(Vec<crate::processing::FileResult>),
    TaskProgress {
        kind: TaskKind,
        done: usize,
        total: usize,
    },
    TaskDone {
        kind: TaskKind,
        result: Result<(), EngineError>,
    },
    SignatureRendered {
        id: u64,
        result: Result<image::RgbaImage, EngineError>,
    },
    Signatures(Result<Vec<sunipdf_core::certificate::DigitalSignature>, EngineError>),
    SignedCopy(Result<PathBuf, EngineError>),
    RecoverySaved(Result<(), EngineError>),
    SuspendFailed(EngineError),
    ReadingSelected {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::annot::ReadingSelection, EngineError>,
    },
    EngineReady,
    EngineFailed(EngineError),
    Opened(DocState),
    OpenFailed {
        path: PathBuf,
        error: EngineError,
    },
    Tile {
        epoch: u64,
        job: TileJob,
        image: Arc<RenderedPage>,
    },
    TileFailed {
        epoch: u64,
        job: TileJob,
        error: EngineError,
    },
    Thumbnail {
        page: usize,
        image: Arc<RenderedPage>,
    },
    ThumbnailFailed {
        page: usize,
        width_px: u32,
    },
    SearchResults {
        generation: u64,
        matches: Vec<TextMatch>,
    },
    SearchProgress {
        generation: u64,
        done: usize,
        total: usize,
    },
    SearchFailed {
        generation: u64,
        error: EngineError,
    },
    /// Pages modifiées en mémoire : tailles rafraîchies, rendus invalidés.
    DocumentChanged(DocState),
    OperationFailed(EngineError),
    TextSelected {
        page: usize,
        selection: TextSelection,
    },
    Annotations {
        page: usize,
        list: Vec<AnnotationInfo>,
        details: Option<Vec<AnnotDetail>>,
    },
    LibraryCaptureCandidates {
        epoch: u64,
        generation: u64,
        result: Result<Vec<sunipdf_core::annotation_library::LibraryCaptureCandidate>, EngineError>,
    },
    LibraryEntryCaptured {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::annotation_library::LibraryEntry, EngineError>,
    },
    QuantityTable {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::quantity_table::QuantityTable, EngineError>,
    },
    QuantityExported {
        epoch: u64,
        generation: u64,
        result: Result<(), EngineError>,
    },
    AnnotationRegister {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::annotation_register::AnnotationRegister, EngineError>,
    },
    AnnotationRegisterProgress {
        epoch: u64,
        generation: u64,
        done: usize,
        total: usize,
    },
    MeasurementPage {
        page: usize,
        epoch: u64,
        generation: u64,
        result: Result<crate::measurement_saved::MeasurementPageData, EngineError>,
    },
    RecalibrationPreview {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::measurement::RecalibrationPlan, EngineError>,
    },
    Links {
        page: usize,
        list: Vec<LinkInfo>,
    },
    FormInfo(Result<FormInfo, EngineError>),
    Properties(Properties),
    Inspection {
        epoch: u64,
        result: Result<sunipdf_core::inspection::Inspection, EngineError>,
    },
    Bookmarks(Vec<Bookmark>),
    BatchDone(BatchResult),
    ObjectSelected {
        page: usize,
        info: Option<ObjectInfo>,
    },
    ObjectsSelected {
        page: usize,
        infos: Vec<ObjectInfo>,
        mode: PickMode,
    },
    Objects {
        page: usize,
        list: Vec<ObjectInfo>,
    },
    RotationCenter {
        page: usize,
        indices: Vec<usize>,
        epoch: u64,
        generation: u64,
        center: Option<(f32, f32)>,
    },
    ParagraphPreview {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::paragraph::ParagraphPreview, EngineError>,
    },
    RichAnnotationPreview {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::annot_rich::RichAnnotationPreview, EngineError>,
    },
    /// Objets copiés : `text` = texte des objets texte (repère du presse-papiers système).
    Copied {
        count: usize,
        text: String,
    },
    SystemFonts(Arc<Vec<SystemFont>>),
    ImageLoaded {
        path: PathBuf,
        result: Result<image::RgbaImage, EngineError>,
    },
    Suspended(SuspendedDoc),
    Resumed(DocState),
    /// Return the complete suspended document so a failed resume remains retryable.
    ResumeFailed {
        error: EngineError,
        suspended: SuspendedDoc,
    },
    Saved {
        path: PathBuf,
    },
    SaveFailed(EngineError),
    RedactMatches {
        query: String,
        verify: bool,
        marks: Vec<RedactMark>,
    },
    /// Bilan d'un caviardage réussi (suivi de `DocumentChanged`).
    Redacted(RedactReport),
    RedactionPreview {
        epoch: u64,
        generation: u64,
        result: Result<sunipdf_core::redact::RedactionPreview, EngineError>,
    },
    RedactionMarks {
        epoch: u64,
        result: Result<Vec<RedactMark>, EngineError>,
    },
    MutationFinished,
}

pub struct Worker {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    pending_mutations: Cell<usize>,
}

impl Worker {
    #[cfg(test)]
    pub(crate) fn test_channels() -> (Self, Receiver<Request>, Sender<Response>) {
        let (tx, requests) = mpsc::channel();
        let (responses, rx) = mpsc::channel();
        (
            Self {
                tx,
                rx,
                pending_mutations: Cell::new(0),
            },
            requests,
            responses,
        )
    }

    pub fn spawn(ctx: egui::Context) -> Self {
        let (tx, worker_rx) = mpsc::channel::<Request>();
        let (worker_tx, rx) = mpsc::channel::<Response>();
        std::thread::Builder::new()
            .name("sunipdf-engine".into())
            .spawn(move || run(worker_rx, worker_tx, ctx))
            .expect("thread moteur");
        Self {
            tx,
            rx,
            pending_mutations: Cell::new(0),
        }
    }

    pub fn send(&self, request: Request) {
        let mutation = matches!(
            &request,
            Request::Edit(_)
                | Request::Undo
                | Request::Redo
                | Request::CommitRedactionPreview { .. }
                | Request::PersistRedactionMarks { .. }
                | Request::TextInRect { .. }
                | Request::FormInfo
                | Request::CopyObjects { cut: true, .. }
                | Request::Save { .. }
                | Request::Ocr { .. }
                | Request::PlaceSignature { .. }
                | Request::Print { .. }
                | Request::SignCertificate { .. }
                | Request::Autosave
                | Request::MarkRecovered { .. }
                | Request::ProcessFiles(_)
        );
        if self.tx.send(request).is_ok() && mutation {
            self.pending_mutations.set(self.pending_mutations.get() + 1);
            // La borne FIFO est acquittée même si l'opération ne change rien ou échoue.
            let _ = self.tx.send(Request::FinishMutation);
        }
    }

    pub fn has_pending_mutations(&self) -> bool {
        self.pending_mutations.get() != 0
    }

    pub fn finish_mutation(&self) {
        self.pending_mutations
            .set(self.pending_mutations.get().saturating_sub(1));
    }

    pub fn poll(&self) -> TryIter<'_, Response> {
        self.rx.try_iter()
    }
}

struct State<'a> {
    recovery: Option<crate::recovery::Slot>,
    document: Option<Document<'a>>,
    signature_source: Option<SignatureSource>,
    cache: RenderCache,
    /// Tuiles voulues par l'UI, rendues une à une quand aucune requête n'attend.
    wanted: WantedQueue,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Longueur de la pile d'annulation à l'ouverture ou au dernier enregistrement : le document
    /// est « modifié » tant qu'elle en diffère (annuler jusqu'à l'état enregistré l'efface).
    /// `None` : l'état enregistré n'est plus atteignable (modifié après une annulation, ou
    /// instantané sorti de l'historique).
    save_point: Option<usize>,
    /// Derniers objets décalés au clavier (page, index).
    last_nudge: Option<(usize, Vec<usize>)>,
    /// Presse-papiers d'objets : survit aux changements d'onglet (copier d'un document, coller
    /// dans un autre).
    clip: Option<ObjectClip>,
    fonts: Option<Arc<Vec<SystemFont>>>,
    rich_preview: Option<(u64, u64, sunipdf_core::annot_rich::PreparedRichAnnotation)>,
    plan_zones: crate::plan_zones_ui::WorkerCache,
    visual_search: crate::visual_search_ui::WorkerCache,
    boundary_fill: crate::boundary_fill_ui::WorkerCache,
    boundary_turn: bool,
    visual_turn: bool,
    quantity_workbook: crate::quantity_workbook_ui::WorkerCache,
    issue_report: crate::issue_report_ui::WorkerCache,
    rectangle_calibration: Option<RectangleCalibrationCapture>,
    redaction_preview: Option<(u64, u64, sunipdf_core::redact::PreparedRedaction)>,
    search: Option<SearchJob>,
    register: Option<RegisterJob>,
    register_turn: bool,
}

struct RectangleCalibrationCapture {
    page: usize,
    calibration: sunipdf_core::measurement::PageCalibration,
    result: Result<sunipdf_core::measurement::LocalCalibrationReference, String>,
}

impl RectangleCalibrationCapture {
    fn reference(
        &self,
        page: usize,
        calibration: &sunipdf_core::measurement::PageCalibration,
    ) -> Result<sunipdf_core::measurement::LocalCalibrationReference, EngineError> {
        if self.page != page || self.calibration != *calibration {
            return Err(EngineError::Other(RECTANGLE_REFERENCE_REQUIRED.into()));
        }
        self.result
            .as_ref()
            .cloned()
            .map_err(|reason| EngineError::Other(format!("{RECTANGLE_CAPTURE_FAILED}{reason}")))
    }
}

struct SearchJob {
    query: String,
    match_case: bool,
    generation: u64,
    next_page: usize,
    matches: Vec<TextMatch>,
    rectangles: usize,
}

struct RegisterJob {
    epoch: u64,
    generation: u64,
    reader: Option<sunipdf_core::annotation_register::RegisterReader<'static>>,
}

enum IdleWork {
    Tile(u64, TileJob),
    Background,
    Wait,
}

impl State<'_> {
    fn next_idle_work(&mut self) -> IdleWork {
        if let Some((epoch, job)) = self.wanted.pop() {
            IdleWork::Tile(epoch, job)
        } else if self.search.is_some()
            || self.register.is_some()
            || self.visual_search.active()
            || self.boundary_fill.active()
        {
            IdleWork::Background
        } else {
            IdleWork::Wait
        }
    }

    fn signature_bytes(&self) -> Result<&[u8], EngineError> {
        let doc = self
            .document
            .as_ref()
            .ok_or_else(|| EngineError::Other("no document".into()))?;
        self.signature_source
            .as_ref()
            .ok_or_else(|| EngineError::Other(CERTIFICATE_SOURCE_CHANGED.into()))?
            .checked_bytes(&doc.info().path, self.modified())
    }

    fn render_cached(&mut self, page: usize, scale: f32) -> Result<Arc<RenderedPage>, EngineError> {
        if let Some(hit) = self.cache.get(page, scale) {
            return Ok(hit);
        }
        let doc = self
            .document
            .as_ref()
            .ok_or(EngineError::PageOutOfRange(page))?;
        let image = doc.render_page(page, scale)?;
        Ok(self.cache.insert(scale, image))
    }

    fn render_tile(&mut self, job: &TileJob) -> Result<Arc<RenderedPage>, EngineError> {
        if let Some(hit) = self.cache.get_tile(job) {
            return Ok(hit);
        }
        let doc = self
            .document
            .as_ref()
            .ok_or(EngineError::PageOutOfRange(job.page))?;
        let image = doc.render_region(job.page, job.scale(), job.region)?;
        log::trace!("tuile {job:?} ; cache {} octets", self.cache.bytes());
        Ok(self.cache.insert_tile(*job, image))
    }

    fn invalidate(&mut self) {
        self.cache.clear();
        self.wanted.clear();
        self.rich_preview = None;
        self.plan_zones.clear();
        self.visual_search.clear();
        self.boundary_fill.clear();
        self.quantity_workbook.clear();
        self.issue_report.clear();
        self.redaction_preview = None;
        self.search = None;
        self.register = None;
    }

    /// One bounded unit of background work; called only after requests and visible tiles.
    /// PDFium remains confined to this thread. Jobs never hold a document borrow across turns.
    fn background_step(&mut self) -> Option<Response> {
        if self.boundary_fill.active()
            && (self.boundary_turn
                || (!self.visual_search.active()
                    && self.search.is_none()
                    && self.register.is_none()))
        {
            self.boundary_turn = false;
            let (epoch, generation) = self.boundary_fill.token()?;
            return self
                .boundary_fill
                .step()
                .map(|outcome| Response::BoundaryFill {
                    epoch,
                    generation,
                    result: Ok(outcome),
                });
        }
        self.boundary_turn = true;
        if self.visual_search.active()
            && (self.visual_turn || (self.search.is_none() && self.register.is_none()))
        {
            self.visual_turn = false;
            let (epoch, generation) = self.visual_search.token()?;
            let result = match self.document.as_ref() {
                Some(doc) => self.visual_search.step(doc),
                None => Err(EngineError::Other("no document".into())),
            };
            if result.is_err() {
                self.visual_search.clear();
            }
            return match result {
                Ok(None) => None,
                Ok(Some(outcome)) => Some(Response::VisualSearch {
                    epoch,
                    generation,
                    result: Ok(outcome),
                }),
                Err(error) => Some(Response::VisualSearch {
                    epoch,
                    generation,
                    result: Err(error),
                }),
            };
        }
        self.visual_turn = true;
        let register = self.register.is_some() && (self.register_turn || self.search.is_none());
        self.register_turn = !register;
        if register {
            let mut job = self.register.take()?;
            let result = (|| {
                let doc = self
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))?;
                if job.reader.is_none() {
                    job.reader = Some(doc.annotation_register_reader()?);
                    return Ok(None);
                }
                let reader = job.reader.as_mut().expect("initialized register reader");
                if reader.read_next(doc)? {
                    return job
                        .reader
                        .take()
                        .expect("completed reader")
                        .finish()
                        .map(Some);
                }
                Ok(None)
            })();
            return Some(match result {
                Ok(None) => {
                    let (done, total) = job.reader.as_ref().expect("initialized reader").progress();
                    let response = Response::AnnotationRegisterProgress {
                        epoch: job.epoch,
                        generation: job.generation,
                        done,
                        total,
                    };
                    self.register = Some(job);
                    response
                }
                result => Response::AnnotationRegister {
                    epoch: job.epoch,
                    generation: job.generation,
                    result: result.and_then(|report| {
                        report.ok_or_else(|| EngineError::Other("incomplete register".into()))
                    }),
                },
            });
        }
        let mut job = self.search.take()?;
        let result = (|| {
            let doc = self
                .document
                .as_ref()
                .ok_or_else(|| EngineError::Other("no document".into()))?;
            let total = doc.page_count();
            if job.next_page < total {
                let matches = doc.search_page(job.next_page, &job.query, job.match_case)?;
                job.rectangles += matches.iter().map(|item| item.rects.len()).sum::<usize>();
                if job.matches.len() + matches.len() > 100_000 || job.rectangles > 1_000_000 {
                    return Err(EngineError::Other(
                        "Search results exceed the display limit; use a more specific query."
                            .into(),
                    ));
                }
                job.matches.extend(matches);
                job.next_page += 1;
            }
            Ok(total)
        })();
        Some(match result {
            Err(error) => Response::SearchFailed {
                generation: job.generation,
                error,
            },
            Ok(total) if job.next_page >= total => Response::SearchResults {
                generation: job.generation,
                matches: job.matches,
            },
            Ok(total) => {
                let response = Response::SearchProgress {
                    generation: job.generation,
                    done: job.next_page,
                    total,
                };
                self.search = Some(job);
                response
            }
        })
    }

    fn modified(&self) -> bool {
        self.save_point != Some(self.undo.len())
    }

    fn doc_state(&self) -> Option<DocState> {
        self.document.as_ref().map(|d| DocState {
            info: d.info().clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            modified: self.modified(),
            pages_unchanged: false,
            measurement_scales_unchanged: false,
            has_signatures: d.signature_count() != 0,
        })
    }

    fn push_undo(&mut self, snapshot: Snapshot) {
        if self.save_point.is_some_and(|sp| sp > self.undo.len()) {
            // Modification après une annulation : l'état enregistré (dans la pile de rétablissement
            // qu'on vide) n'est plus atteignable.
            self.save_point = None;
        }
        self.undo.push(snapshot);
        self.redo.clear();
        while self.undo.len() > UNDO_MAX_ENTRIES
            || (self.undo.len() > 1
                && self.undo.iter().map(|s| s.bytes.len()).sum::<usize>() > UNDO_MAX_BYTES)
        {
            self.undo.remove(0);
            self.save_point = self.save_point.and_then(|sp| sp.checked_sub(1));
        }
    }

    /// Applique une modification sous instantané. Toute erreur laisse le document intact
    /// (les opérations échouent avant de modifier, ou l'instantané est restauré).
    fn edit(&mut self, op: EditOp) -> Response {
        let op = match op {
            EditOp::CommitBoundaryFill { epoch, generation } => {
                match self.boundary_fill.take(epoch, generation) {
                    Ok(prepared) => EditOp::PreparedBoundaryFill(Box::new(prepared)),
                    Err(error) => return Response::OperationFailed(error),
                }
            }
            EditOp::CommitVisualSearchCounts {
                epoch,
                generation,
                ids,
                group,
                size,
            } => {
                let result = match self.document.as_ref() {
                    Some(doc) => self
                        .visual_search
                        .prepare_counts(doc, epoch, generation, &ids, &group, size),
                    None => Err(EngineError::Other("no document".into())),
                };
                match result {
                    Ok(prepared) => EditOp::PreparedVisualSearchCounts(prepared),
                    Err(error) => return Response::OperationFailed(error),
                }
            }
            EditOp::CommitPlanZones { epoch, generation } => {
                match self.plan_zones.take(epoch, generation) {
                    Ok(p) => EditOp::PreparedPlanZones(p),
                    Err(e) => return Response::OperationFailed(e),
                }
            }
            EditOp::CommitRichAnnotationPreview { epoch, generation } => {
                match self.rich_preview.take() {
                    Some((saved_epoch, saved_generation, prepared))
                        if saved_epoch == epoch && saved_generation == generation =>
                    {
                        EditOp::PreparedRichAnnotation(prepared)
                    }
                    _ => {
                        return Response::OperationFailed(EngineError::Other(
                            "rich annotation preview is stale; preview again".into(),
                        ));
                    }
                }
            }
            op => op,
        };
        let nudge = match &op {
            EditOp::NudgeObjects { page, indices, .. } => Some((*page, indices.clone())),
            _ => None,
        };
        // Décalages clavier consécutifs du même objet : l'instantané pris avant le premier suffit.
        let coalesce = nudge.is_some() && nudge == self.last_nudge && !self.undo.is_empty();
        let keeps_pages = op.keeps_pages();
        let keeps_measurement_scales = op.keeps_measurement_scales();
        let Some(doc) = &mut self.document else {
            return Response::OperationFailed(EngineError::Other("no document".into()));
        };
        let snapshot = if coalesce {
            None
        } else {
            match doc.snapshot() {
                Ok(bytes) => Some(Snapshot {
                    bytes,
                    password: doc.password().map(str::to_owned),
                    pages_unchanged: keeps_pages,
                    measurement_scales_unchanged: keeps_measurement_scales,
                }),
                Err(e) => return Response::OperationFailed(e),
            }
        };
        match apply(doc, op, &self.clip) {
            Ok(()) => {
                match snapshot {
                    Some(s) => self.push_undo(s),
                    None => self.redo.clear(),
                }
                self.last_nudge = nudge;
                self.invalidate();
                Response::DocumentChanged(DocState {
                    pages_unchanged: keeps_pages,
                    measurement_scales_unchanged: keeps_measurement_scales,
                    ..self.doc_state().expect("document ouvert")
                })
            }
            Err(e) => {
                if let Some(s) = snapshot {
                    if let Err(restore) = doc.restore_with_password(s.bytes, s.password) {
                        log::error!("restauration impossible après échec : {restore}");
                    }
                }
                self.last_nudge = None;
                Response::OperationFailed(e)
            }
        }
    }

    /// Caviardage sous instantané (même règle que `edit`) : en cas d'échec, l'état d'avant est
    /// rechargé ; jamais d'écriture de fichier ici.
    fn redact_candidate(
        &mut self,
        apply: impl FnOnce(&mut Document<'_>) -> Result<RedactReport, EngineError>,
    ) -> Result<(RedactReport, DocState), EngineError> {
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| EngineError::Other("no document".into()))?;
        let snapshot = Snapshot {
            bytes: doc.snapshot()?,
            password: doc.password().map(str::to_owned),
            pages_unchanged: false,
            measurement_scales_unchanged: false,
        };
        match apply(doc) {
            Ok(report) => {
                self.push_undo(snapshot);
                self.last_nudge = None;
                self.invalidate();
                Ok((report, self.doc_state().expect("document ouvert")))
            }
            Err(e) => {
                if let Err(restore) = doc.restore_with_password(snapshot.bytes, snapshot.password) {
                    log::error!("restauration impossible après échec : {restore}");
                }
                Err(e)
            }
        }
    }

    fn undo_redo(&mut self, undo: bool) -> Option<Response> {
        let doc = self.document.as_mut()?;
        let (from, to) = if undo {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let target = from.pop()?;
        let current = match doc.snapshot() {
            Ok(bytes) => Snapshot {
                bytes,
                password: doc.password().map(str::to_owned),
                pages_unchanged: target.pages_unchanged,
                measurement_scales_unchanged: target.measurement_scales_unchanged,
            },
            Err(e) => {
                from.push(target);
                return Some(Response::OperationFailed(e));
            }
        };
        // L'instantané garde son mot de passe : annuler un « Retirer la protection » fonctionne.
        if let Err(e) = doc.restore_with_password(target.bytes.clone(), target.password.clone()) {
            from.push(target);
            return Some(Response::OperationFailed(e));
        }
        let pages_unchanged = target.pages_unchanged;
        let measurement_scales_unchanged = target.measurement_scales_unchanged;
        to.push(current);
        self.last_nudge = None;
        self.invalidate();
        Some(Response::DocumentChanged(DocState {
            pages_unchanged,
            measurement_scales_unchanged,
            ..self.doc_state()?
        }))
    }
}

/// Point d'enregistrement : le document n'est « modifié » que si la pile d'annulation n'a pas la
/// longueur mémorisée à l'ouverture ou au dernier enregistrement.
#[cfg(test)]
mod save_point_tests {
    use super::*;

    #[test]
    fn saved_revision_guard_refuses_large_replacements_before_loading_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("replacement.pdf");
        let original = b"saved revision".to_vec();
        let source = SignatureSource::new(original.clone());
        std::fs::write(&path, original.iter().rev().copied().collect::<Vec<_>>()).unwrap();
        assert!(
            matches!(source.checked_bytes(&path, false), Err(EngineError::Other(message)) if message == CERTIFICATE_SOURCE_CHANGED)
        );
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(512 * 1024 * 1024).unwrap();
        assert!(
            matches!(source.checked_bytes(&path, false), Err(EngineError::Other(message)) if message == CERTIFICATE_SOURCE_CHANGED)
        );
        drop(file);
        std::fs::write(&path, &original).unwrap();
        assert_eq!(source.checked_bytes(&path, false).unwrap(), original);
    }

    #[test]
    fn signature_source_rejects_changed_missing_and_unsaved_documents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loaded.pdf");
        let original = b"original displayed revision".to_vec();
        std::fs::write(&path, &original).unwrap();
        let source = SignatureSource::new(original.clone());
        assert_eq!(source.checked_bytes(&path, false).unwrap(), original);
        assert!(
            matches!(source.checked_bytes(&path, true), Err(EngineError::Other(message))
            if message == CERTIFICATE_SAVE_FIRST)
        );
        std::fs::write(&path, b"replacement revision").unwrap();
        assert!(
            matches!(source.checked_bytes(&path, false), Err(EngineError::Other(message))
            if message == CERTIFICATE_SOURCE_CHANGED)
        );
        std::fs::remove_file(&path).unwrap();
        assert!(
            matches!(source.checked_bytes(&path, false), Err(EngineError::Other(message))
            if message == CERTIFICATE_SOURCE_CHANGED)
        );
        // A concurrent change after the check cannot redirect operations to the replacement bytes.
        std::fs::write(&path, &original).unwrap();
        let captured = source.checked_bytes(&path, false).unwrap();
        std::fs::write(&path, b"changed after checking").unwrap();
        assert_eq!(captured, original);
        let resumed = source.clone();
        std::fs::write(&path, &original).unwrap();
        assert_eq!(resumed.checked_bytes(&path, false).unwrap(), original);
    }

    #[test]
    fn rectangle_capture_failures_keep_the_cause_and_are_scoped_to_the_requested_reference() {
        use sunipdf_core::measurement::{MeasurementUnit, PageCalibration};
        let calibration = PageCalibration {
            points: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        };
        for reason in [
            "local references are limited to documents up to 128 MiB",
            "document snapshot failed",
        ] {
            let capture = RectangleCalibrationCapture {
                page: 0,
                calibration: calibration.clone(),
                result: Err(reason.into()),
            };
            assert!(
                matches!(capture.reference(0, &calibration), Err(EngineError::Other(message))
                if message == format!("{RECTANGLE_CAPTURE_FAILED}{reason}"))
            );
            assert!(
                matches!(capture.reference(1, &calibration), Err(EngineError::Other(message))
                if message == RECTANGLE_REFERENCE_REQUIRED)
            );
            let mut other = calibration.clone();
            other.known_distance = 12.0;
            assert!(
                matches!(capture.reference(0, &other), Err(EngineError::Other(message))
                if message == RECTANGLE_REFERENCE_REQUIRED)
            );
        }
    }

    fn state() -> State<'static> {
        State {
            recovery: None,
            document: None,
            signature_source: None,
            cache: RenderCache::new(1024),
            wanted: WantedQueue::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            save_point: Some(0),
            last_nudge: None,
            clip: None,
            fonts: None,
            rich_preview: None,
            plan_zones: Default::default(),
            visual_search: Default::default(),
            boundary_fill: Default::default(),
            boundary_turn: false,
            visual_turn: false,
            quantity_workbook: Default::default(),
            issue_report: Default::default(),
            rectangle_calibration: None,
            redaction_preview: None,
            search: None,
            register: None,
            register_turn: false,
        }
    }

    fn snap(n: u8) -> Snapshot {
        Snapshot {
            bytes: vec![n],
            password: None,
            pages_unchanged: false,
            measurement_scales_unchanged: false,
        }
    }

    #[test]
    fn visible_tiles_precede_background_jobs_and_document_changes_discard_them() {
        let mut state = state();
        state.search = Some(SearchJob {
            query: "drawing".into(),
            match_case: false,
            generation: 31,
            next_page: 0,
            matches: Vec::new(),
            rectangles: 0,
        });
        state.register = Some(RegisterJob {
            epoch: 9,
            generation: 32,
            reader: None,
        });
        let tile = TileJob {
            page: 4,
            scale_milli: 1000,
            region: sunipdf_core::PixelRect::new(0, 0, 10, 10),
        };
        state.wanted.replace(9, vec![tile]);
        assert!(matches!(state.next_idle_work(), IdleWork::Tile(9, selected) if selected == tile));
        assert!(state.search.is_some() && state.register.is_some());
        assert!(matches!(state.next_idle_work(), IdleWork::Background));
        state.invalidate();
        assert!(matches!(state.next_idle_work(), IdleWork::Wait));
        assert!(state.search.is_none() && state.register.is_none());
    }

    #[test]
    fn page_render_reuse_does_not_imply_calibration_survives_object_transforms() {
        for op in [
            EditOp::ResizeObjects {
                page: 0,
                indices: vec![0],
                bounds: PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 200.0,
                    top: 100.0,
                },
            },
            EditOp::RotateObjects {
                page: 0,
                indices: vec![0],
                degrees: 90.0,
            },
            EditOp::MoveObjects {
                page: 0,
                indices: vec![0],
                dx: 5.0,
                dy: 0.0,
            },
            EditOp::FlattenForm { pages: None },
        ] {
            assert!(op.keeps_pages());
            assert!(!op.keeps_measurement_scales());
        }
        assert!(EditOp::SetPageCalibration {
            page: 0,
            calibration: None
        }
        .keeps_measurement_scales());
        assert!(EditOp::DeleteAnnotation { page: 0, index: 0 }.keeps_measurement_scales());
    }

    #[test]
    fn undoing_back_to_the_saved_state_clears_modified() {
        let mut s = state();
        assert!(!s.modified(), "ouverture");
        s.push_undo(snap(1));
        assert!(s.modified());
        // Annuler (le thread moteur déplace l'instantané vers la pile de rétablissement).
        let u = s.undo.pop().unwrap();
        s.redo.push(u);
        assert!(!s.modified(), "retour à l'état d'ouverture");
        let r = s.redo.pop().unwrap();
        s.undo.push(r);
        assert!(s.modified(), "rétabli");
        // Enregistrer ici, puis annuler : modifié ; rétablir : intact.
        s.save_point = Some(s.undo.len());
        assert!(!s.modified());
        let u = s.undo.pop().unwrap();
        s.redo.push(u);
        assert!(s.modified());
        // Nouvelle modification après l'annulation : l'état enregistré est perdu, même en revenant
        // à la même longueur de pile.
        s.push_undo(snap(2));
        assert_eq!(s.save_point, None);
        assert!(s.modified());
        let u = s.undo.pop().unwrap();
        s.redo.push(u);
        assert!(s.modified(), "même longueur, mais contenu différent");
    }

    #[test]
    fn trimming_the_history_shifts_or_drops_the_save_point() {
        let mut s = state();
        for i in 0..3 {
            s.push_undo(snap(i));
        }
        s.save_point = Some(2);
        for i in 3..UNDO_MAX_ENTRIES as u8 + 1 {
            s.push_undo(snap(i));
        }
        assert_eq!(s.undo.len(), UNDO_MAX_ENTRIES);
        assert_eq!(s.save_point, Some(1), "décalé d'un cran");
        s.push_undo(snap(99));
        s.push_undo(snap(100));
        assert_eq!(s.save_point, None, "instantané sorti de l'historique");
        assert!(s.modified());
    }
}

/// Pages triées, sans doublon, toutes valides.
fn checked_pages(mut pages: Vec<usize>, count: usize) -> Result<Vec<usize>, EngineError> {
    pages.sort_unstable();
    pages.dedup();
    match pages.iter().find(|&&p| p >= count) {
        Some(&bad) => Err(EngineError::PageOutOfRange(bad)),
        None if pages.is_empty() => Err(EngineError::Other("no pages selected".into())),
        None => Ok(pages),
    }
}

fn apply(doc: &mut Document<'_>, op: EditOp, clip: &Option<ObjectClip>) -> Result<(), EngineError> {
    match op {
        EditOp::PreparedBoundaryFill(p) => doc.apply_boundary_fill_measurement(&p).map(|_| ()),
        EditOp::CommitBoundaryFill { .. } => unreachable!("resolved by worker"),
        EditOp::PreparedVisualSearchCounts(p) => doc.apply_visual_search_counts(&p).map(|_| ()),
        EditOp::CommitVisualSearchCounts { .. } => unreachable!("resolved by worker"),
        EditOp::PreparedPlanZones(p) => doc.apply_plan_zones(p),
        EditOp::CommitPlanZones { .. } => unreachable!("resolved by worker"),
        EditOp::SetPageLabels(range) => doc.set_page_labels(&range),
        EditOp::ResetPageLabels => doc.reset_page_labels(),
        EditOp::AddFormField {
            page,
            name,
            kind,
            rect,
            options,
            multi_select,
            editable,
        } => {
            doc.add_form_field_with_flags(page, &name, kind, rect, &options, multi_select, editable)
        }
        EditOp::ResizeObjects {
            page,
            indices,
            bounds,
        } => doc.resize_objects(page, &indices, bounds),
        EditOp::RotateObjects {
            page,
            indices,
            degrees,
        } => doc.rotate_objects(page, &indices, degrees),
        EditOp::Rotate {
            pages,
            delta_degrees,
        } => {
            for page in pages {
                let current = doc.page_rotation(page)?;
                doc.set_page_rotation(page, current.turned(delta_degrees))?;
            }
            Ok(())
        }
        EditOp::Delete { pages } => doc.delete_pages(&pages),
        EditOp::Insert {
            source,
            password,
            pages,
            at,
        } => doc
            .insert_pages_from(&source, password.as_deref(), &pages, at)
            .map(|_| ()),
        EditOp::Move { from, to } => doc.move_page(from, to),
        EditOp::Markup {
            page,
            kind,
            rects,
            rgb,
        } => doc.add_markup(page, kind, &rects, rgb),
        EditOp::Note { page, x, y, text } => doc.add_note(page, x, y, &text),
        EditOp::DeleteAnnotation { page, index } => doc.delete_annotation(page, index),
        EditOp::AddInk {
            page,
            strokes,
            style,
            author,
        } => doc
            .add_ink_annotation(page, &strokes, &style, author.as_deref())
            .map(|_| ()),
        EditOp::AddAnnotShape {
            page,
            shape,
            rect,
            style,
            author,
        } => doc
            .add_shape_annotation(page, shape, rect, &style, author.as_deref())
            .map(|_| ()),
        EditOp::AddFreeText {
            page,
            rect,
            text,
            style,
            author,
        } => doc
            .add_free_text_annotation(page, rect, &text, &style, author.as_deref())
            .map(|_| ()),
        EditOp::AddLink { page, rect, target } => {
            doc.add_link_annotation(page, rect, &target).map(|_| ())
        }
        EditOp::SetLinkTarget {
            page,
            index,
            target,
        } => doc.set_link_target(page, index, &target),
        EditOp::MoveAnnotation {
            page,
            index,
            dx,
            dy,
        } => doc.move_annotation(page, index, dx, dy),
        EditOp::ResizeAnnotation { page, index, rect } => doc.resize_annotation(page, index, rect),
        EditOp::StyleAnnotation { page, index, style } => {
            doc.set_annotation_style(page, index, &style)
        }
        EditOp::SetAnnotationText { page, index, text } => {
            doc.set_annotation_contents(page, index, &text)
        }
        EditOp::AddCountMarker {
            page,
            point,
            size,
            group,
            author,
        } => doc
            .add_count_marker(page, point, size, &group, author.as_deref())
            .map(|_| ()),
        EditOp::SetPageCalibration { page, calibration } => {
            doc.set_page_calibration(page, calibration.as_ref())
        }
        EditOp::ApplyRecalibration(plan) => doc.apply_recalibration(&plan),
        EditOp::SetMeasurementViewports {
            page,
            expected,
            replacement,
        } => doc.set_measurement_viewports(page, &expected, &replacement),
        EditOp::ClearMeasurementViewports(target) => doc.clear_measurement_viewports(&target),
        EditOp::SetQuantityFields { target, fields } => doc.set_quantity_fields(&target, &fields),
        EditOp::SetAnnotationRegisterFields {
            target,
            fields,
            count_group,
        } => doc.set_annotation_register_fields(&target, &fields, count_group.as_deref()),
        EditOp::UpdateFreeText {
            page,
            index,
            text,
            style,
        } => doc.update_free_text(page, index, &text, &style),
        EditOp::DeleteAnnotationThread { page, index } => {
            doc.delete_annotation_thread(page, index).map(|_| ())
        }
        EditOp::Reply {
            page,
            parent,
            text,
            author,
        } => doc
            .add_reply(page, parent, &text, author.as_deref())
            .map(|_| ()),
        EditOp::SetReviewStatus {
            page,
            parent,
            status,
            author,
        } => doc
            .set_review_status(page, parent, status, author.as_deref())
            .map(|_| ()),
        EditOp::AddText {
            page,
            x,
            y,
            text,
            style,
        } => doc.add_text(page, x, y, &text, &style).map(|_| ()),
        EditOp::SetText {
            page,
            index,
            text,
            size,
            rgb,
        } => {
            if let Some(text) = text {
                doc.set_object_text(page, index, &text)?;
            }
            if let Some(size) = size {
                doc.set_text_size(page, index, size)?;
            }
            if let Some(rgb) = rgb {
                doc.set_object_style(
                    page,
                    index,
                    &ObjectStyle {
                        fill_rgb: Some(rgb),
                        ..Default::default()
                    },
                )?;
            }
            Ok(())
        }
        EditOp::ReflowParagraph {
            page,
            indices,
            text,
            width,
            leading,
        } => doc
            .reflow_paragraph(page, &indices, &text, width, leading)
            .map(|_| ()),
        EditOp::PreparedRichAnnotation(prepared) => {
            doc.commit_rich_annotation_preview(prepared).map(|_| ())
        }
        EditOp::CommitRichAnnotationPreview { .. } => Err(EngineError::Other(
            "rich annotation preview was not prepared".into(),
        )),
        EditOp::SetObjectStyle {
            page,
            indices,
            style,
        } => doc.set_objects_style(page, &indices, &style),
        EditOp::MoveObjects {
            page,
            indices,
            dx,
            dy,
        }
        | EditOp::NudgeObjects {
            page,
            indices,
            dx,
            dy,
        } => doc.move_objects(page, &indices, dx, dy),
        EditOp::DeleteObjects { page, indices } => doc.delete_objects(page, &indices),
        EditOp::AddImage {
            page,
            path,
            x,
            y,
            width_pt,
            opacity,
        } => doc.add_image_with(page, &path, x, y, width_pt, opacity),
        EditOp::AddImageData {
            page,
            image,
            x,
            y,
            width_pt,
        } => doc.add_image_data(page, &image, x, y, width_pt),
        EditOp::AddShape {
            page,
            kind,
            rect,
            style,
        } => doc.add_shape(page, kind, rect, &style),
        EditOp::DuplicateObjects { page, indices } => doc
            .duplicate_objects(page, &indices, 12.0, -12.0)
            .map(|_| ()),
        EditOp::ReorderObjects { page, indices, op } => {
            doc.reorder_objects(page, &indices, op).map(|_| ())
        }
        EditOp::PasteObjects { page, dx, dy } => {
            let clip = clip
                .as_ref()
                .ok_or_else(|| EngineError::Other("empty object clipboard".into()))?;
            doc.paste_objects(page, clip, dx, dy).map(|_| ())
        }
        EditOp::PasteEditableObjects { page, dx, dy } => {
            let clip = clip
                .as_ref()
                .ok_or_else(|| EngineError::Other("empty object clipboard".into()))?;
            doc.paste_editable_objects(page, clip, dx, dy).map(|_| ())
        }
        EditOp::ReplaceImage { page, index, path } => doc.replace_image(page, index, &path),
        EditOp::CropPages { pages, margins } => {
            for p in checked_pages(pages, doc.page_count())? {
                doc.crop_page(p, margins)?;
            }
            Ok(())
        }
        EditOp::ResetCrop { pages } => {
            for p in checked_pages(pages, doc.page_count())? {
                doc.reset_crop(p)?;
            }
            Ok(())
        }
        EditOp::ResizePages {
            pages,
            width_pt,
            height_pt,
            mode,
        } => {
            for p in checked_pages(pages, doc.page_count())? {
                doc.resize_page(p, width_pt, height_pt, mode)?;
            }
            Ok(())
        }
        EditOp::InsertBlankPages {
            at,
            count,
            width_pt,
            height_pt,
        } => doc.insert_blank_pages(at, count, width_pt, height_pt),
        EditOp::HeadersFooters { pages, options } => {
            doc.headers_footers(pages.as_deref(), &options).map(|_| ())
        }
        EditOp::ImageWatermark {
            pages,
            path,
            options,
        } => doc
            .image_watermark(pages.as_deref(), &path, &options)
            .map(|_| ()),
        EditOp::Stamp { pages, stamp } => doc.stamp_pages(pages.as_deref(), &stamp).map(|_| ()),
        EditOp::FillForm { values } => doc.fill_form(&values, FillMode::Strict).map(|_| ()),
        EditOp::ClearForm => doc.clear_form().map(|_| ()),
        EditOp::FlattenForm { pages } => doc.flatten(pages.as_deref()).map(|_| ()),
        EditOp::Bytes(op) => {
            let current = doc.snapshot()?;
            let password = doc.password().map(str::to_owned);
            match op {
                BytesOp::Bookmark(edit) => doc.edit_bookmark(&edit),
                BytesOp::Protect {
                    protection: p,
                    current_owner,
                } => {
                    let password = current_owner
                        .as_ref()
                        .map(|value| value.as_str())
                        .or(password.as_deref());
                    let bytes = security::protect(&current, password, &p)?;
                    let typed = if p.user_password.is_empty() {
                        p.owner_password
                    } else {
                        p.user_password
                    };
                    // AES-256 : lopdf chiffre la forme SASLprep ; c'est elle qui rouvre le document.
                    let new_password = if p.aes256 {
                        security::normalized_password(&typed)
                    } else {
                        typed
                    };
                    doc.restore_with_password(bytes, Some(new_password))
                }
                BytesOp::Unprotect(owner) => {
                    let bytes = security::unprotect(&current, &owner)?;
                    doc.restore_with_password(bytes, None)
                }
                BytesOp::SetMetadata(m) => {
                    let bytes = security::set_metadata(&current, password.as_deref(), &m)?;
                    doc.restore(bytes)
                }
                BytesOp::StripMetadata => {
                    let bytes = security::strip_metadata(&current, password.as_deref())?;
                    doc.restore(bytes)
                }
            }
        }
    }
}

fn run_batch(engine: &Engine, doc: Option<&Document<'_>>, op: BatchOp) -> BatchResult {
    let result: Result<BatchResult, EngineError> = (|| match op {
        BatchOp::Combine { inputs, out } => {
            let pages = engine.merge(&inputs, &out)?;
            Ok(BatchResult::Combined { out, pages })
        }
        BatchOp::Split { every, out_dir } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            // L'état courant (éventuellement non enregistré) est divisé ; aucun fichier existant
            // n'est remplacé (erreur AlreadyExists avant toute écriture).
            let stem = doc
                .info()
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "document".into());
            let files = doc.split_to(every, &out_dir, &stem, false)?;
            Ok(BatchResult::Split {
                files,
                encrypted: doc.is_encrypted(),
            })
        }
        BatchOp::Extract { pages, out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let n = doc.extract_to(&pages, &out)?;
            Ok(BatchResult::Extracted {
                out,
                pages: n,
                encrypted: doc.is_encrypted(),
            })
        }
        BatchOp::CreateFromImages { images, out } => {
            let pages = engine.images_to_pdf(&images, &out, 150.0)?;
            Ok(BatchResult::Created { out, pages })
        }
        BatchOp::ExportImages { out_dir, dpi } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            Ok(BatchResult::Exported {
                files: doc.export_images(&out_dir, dpi, false)?,
            })
        }
        BatchOp::Optimize { out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let before = doc.snapshot()?;
            let after = sunipdf_core::convert::optimize(&before, doc.password())?;
            sunipdf_core::write_atomic(&out, &after)?;
            Ok(BatchResult::Optimized {
                out,
                before: before.len() as u64,
                after: after.len() as u64,
            })
        }
        BatchOp::Sanitize { out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let (bytes, report) = doc.sanitized_copy()?;
            write_new_copy(&out, &bytes)?;
            Ok(BatchResult::Sanitized { out, report })
        }
        BatchOp::ExportFormData { out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let values = form_data::values_to_json(&doc.form_info()?);
            let count = values.as_object().map_or(0, |m| m.len());
            let bytes = form_data::export_values(
                &doc.form_info()?,
                out.extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("json"),
            )?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(BatchResult::FormExported { out, count })
        }
    })();
    result.unwrap_or_else(BatchResult::Failed)
}

fn run(rx: Receiver<Request>, tx: Sender<Response>, ctx: egui::Context) {
    let engine = match Engine::new() {
        Ok(engine) => {
            let _ = tx.send(Response::EngineReady);
            engine
        }
        Err(e) => {
            let _ = tx.send(Response::EngineFailed(e));
            ctx.request_repaint();
            return;
        }
    };
    ctx.request_repaint();
    let mut state = State {
        recovery: None,
        document: None,
        signature_source: None,
        cache: RenderCache::new(CACHE_BYTES),
        wanted: WantedQueue::default(),
        undo: Vec::new(),
        redo: Vec::new(),
        save_point: Some(0),
        last_nudge: None,
        clip: None,
        fonts: None,
        rich_preview: None,
        plan_zones: Default::default(),
        visual_search: Default::default(),
        boundary_fill: Default::default(),
        boundary_turn: false,
        visual_turn: false,
        quantity_workbook: Default::default(),
        issue_report: Default::default(),
        rectangle_calibration: None,
        redaction_preview: None,
        search: None,
        register: None,
        register_turn: false,
    };
    let mut plan_links = crate::plan_links_ui::WorkerCache::default();
    let mut queue: VecDeque<Request> = VecDeque::new();
    let mut navigation_failed = false;
    let mut revision_outlines = crate::revision_outlines_ui::WorkerCache::default();
    let mut revision_transfers = crate::revision_transfer_ui::WorkerCache::default();
    loop {
        // 1. Travail suivant : file locale, sinon requête reçue, sinon une tuile voulue, sinon
        //    blocage. Entre deux tuiles le canal est relu : un nouveau `SetWanted` ou une édition
        //    passe avant la tuile suivante.
        let request = match queue.pop_front() {
            Some(r) => r,
            None => match rx.try_recv() {
                Ok(r) => r,
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => match state.next_idle_work() {
                    IdleWork::Tile(epoch, job) => {
                        let response = match state.render_tile(&job) {
                            Ok(image) => Response::Tile { epoch, job, image },
                            Err(error) => Response::TileFailed { epoch, job, error },
                        };
                        if tx.send(response).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                        continue;
                    }
                    IdleWork::Background => {
                        if let Some(response) = state.background_step() {
                            if tx.send(response).is_err() {
                                return;
                            }
                            ctx.request_repaint();
                        }
                        continue;
                    }
                    IdleWork::Wait => match rx.recv() {
                        Ok(r) => r,
                        Err(_) => return,
                    },
                },
            },
        };
        // 2. Traitement.
        if matches!(
            &request,
            Request::Open { .. }
                | Request::OpenPlanSheet(_)
                | Request::Resume(_)
                | Request::ResumeDoc(_)
                | Request::Suspend
                | Request::Close
        ) {
            state.rectangle_calibration = None;
        }
        let response = match request {
            Request::PlanLinks { generation, job } => {
                let mut result = plan_links.run(&engine, job);
                if let Ok(crate::plan_links_ui::Outcome::Navigate(navigation)) = &mut result {
                    navigation.active_source = state
                        .document
                        .as_ref()
                        .zip(state.signature_source.as_ref())
                        .map(|(doc, source)| (doc.info().path.clone(), source.sha256_hex()));
                }
                Response::PlanLinksDone { generation, result }
            }
            Request::RevisionOutlines { generation, job } => Response::RevisionOutlinesDone {
                generation,
                result: Box::new(
                    if state.modified()
                        && state.document.as_ref().is_some_and(|doc| {
                            revision_outlines.touches_modified(&job, &doc.info().path)
                        })
                    {
                        let _ = revision_outlines.run(
                            &engine,
                            generation,
                            crate::revision_outlines_ui::Job::Discard,
                            |_| {},
                        );
                        Err(EngineError::Other(
                            "A selected revision has unsaved changes; save it and inspect again"
                                .into(),
                        ))
                    } else {
                        revision_outlines.run(&engine, generation, job, |progress| {
                            let _ = tx.send(Response::RevisionOutlinesProgress {
                                generation,
                                progress,
                            });
                            ctx.request_repaint();
                        })
                    },
                ),
            },
            Request::RevisionTransfer { generation, job } => Response::RevisionTransferDone {
                generation,
                result: if state.modified()
                    && state.document.as_ref().is_some_and(|doc| {
                        revision_transfers.touches_modified(&job, &doc.info().path)
                    }) {
                    let _ = revision_transfers.run(
                        &engine,
                        generation,
                        crate::revision_transfer_ui::Job::Discard,
                    );
                    Err(EngineError::Other(
                        "A selected revision has unsaved changes; save it and inspect again".into(),
                    ))
                } else {
                    revision_transfers.run(&engine, generation, job)
                },
            },
            Request::PlanSet { generation, job } => {
                let mut result = crate::plan_sets::run_job(&engine, job, |done, total| {
                    let _ = tx.send(Response::PlanSetProgress {
                        generation,
                        done,
                        total,
                    });
                    ctx.request_repaint();
                });
                if let Ok(crate::plan_sets::Outcome::Navigate(navigation)) = &mut result {
                    navigation.active_source = state
                        .document
                        .as_ref()
                        .zip(state.signature_source.as_ref())
                        .map(|(doc, source)| (doc.info().path.clone(), source.sha256_hex()));
                }
                Response::PlanSetDone { generation, result }
            }
            Request::RevisionOverlay { generation, job } => Response::RevisionOverlayDone {
                generation,
                result: crate::revision_overlay::run_job(&engine, job),
            },
            Request::LocalReview(job) => {
                let result =
                    crate::local_review::run_job(&engine, job, |done, total, page, pages| {
                        let _ = tx.send(Response::LocalReviewProgress {
                            done,
                            total,
                            page,
                            pages,
                        });
                        ctx.request_repaint();
                    });
                Response::LocalReviewDone(result)
            }
            Request::AcknowledgeSuspendFailure => {
                navigation_failed = false;
                continue;
            }
            Request::Open { .. }
            | Request::OpenPlanSheet(_)
            | Request::Resume(_)
            | Request::ResumeDoc(_)
                if navigation_failed =>
            {
                continue;
            }
            Request::SetRecovery(slot) => {
                state.recovery = slot;
                continue;
            }
            Request::Autosave => {
                let result = if state.modified() {
                    match (state.document.as_ref(), state.recovery.as_ref()) {
                        (Some(doc), Some(slot)) => doc
                            .snapshot()
                            .and_then(|bytes| slot.save(&bytes).map_err(EngineError::from)),
                        _ => Ok(()),
                    }
                } else {
                    Ok(())
                };
                Response::RecoverySaved(result)
            }
            Request::MarkRecovered { path } => {
                if let Some(doc) = &mut state.document {
                    doc.set_path(path);
                }
                state.save_point = None;
                match state.doc_state() {
                    Some(doc) => Response::DocumentChanged(doc),
                    None => continue,
                }
            }
            Request::ProcessFiles(job) => {
                let results = crate::processing::run_job(
                    &engine,
                    state.document.as_ref(),
                    job,
                    |done, total, item_done, item_total| {
                        let _ = tx.send(Response::ProcessingProgress {
                            done,
                            total,
                            item_done,
                            item_total,
                        });
                        ctx.request_repaint();
                    },
                );
                Response::ProcessingDone(results)
            }
            Request::Office(job) => {
                let results = crate::office::run_job(
                    &engine,
                    state.document.as_ref(),
                    job,
                    |done, total, item_done, item_total| {
                        let _ = tx.send(Response::OfficeProgress {
                            done,
                            total,
                            item_done,
                            item_total,
                        });
                        ctx.request_repaint();
                    },
                );
                Response::OfficeDone(results)
            }
            Request::Print { job, cancel } => {
                let result = state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| {
                        crate::printing::print(
                            doc,
                            job,
                            &doc.info().path.display().to_string(),
                            &cancel,
                            |done, total| {
                                let _ = tx.send(Response::TaskProgress {
                                    kind: TaskKind::Print,
                                    done,
                                    total,
                                });
                                ctx.request_repaint();
                            },
                        )
                    })
                    .map(|report| {
                        log::debug!("Submitted {} pages to Windows printing", report.pages)
                    });
                Response::TaskDone {
                    kind: TaskKind::Print,
                    result,
                }
            }
            Request::Ocr { options, cancel } => {
                let result = (|| {
                    let doc = state
                        .document
                        .as_mut()
                        .ok_or_else(|| EngineError::Other("no document".into()))?;
                    let snapshot = Snapshot {
                        bytes: doc.snapshot()?,
                        password: doc.password().map(str::to_owned),
                        pages_unchanged: true,
                        measurement_scales_unchanged: false,
                    };
                    let report = doc.ocr(&options, &cancel, |done, total| {
                        let _ = tx.send(Response::TaskProgress {
                            kind: TaskKind::Ocr,
                            done,
                            total,
                        });
                        ctx.request_repaint();
                    })?;
                    Ok::<_, EngineError>((snapshot, report))
                })();
                match result {
                    Ok((snapshot, report)) => {
                        if report.words != 0 {
                            state.push_undo(snapshot);
                            state.invalidate();
                            state.last_nudge = None;
                            let _ = tx.send(Response::TaskDone {
                                kind: TaskKind::Ocr,
                                result: Ok(()),
                            });
                            let mut changed = state.doc_state().expect("OCR document");
                            changed.pages_unchanged = true;
                            Response::DocumentChanged(changed)
                        } else {
                            Response::TaskDone {
                                kind: TaskKind::Ocr,
                                result: Ok(()),
                            }
                        }
                    }
                    Err(error) => Response::TaskDone {
                        kind: TaskKind::Ocr,
                        result: Err(error),
                    },
                }
            }
            Request::RenderSignature { id, text, style } => Response::SignatureRendered {
                id,
                result: engine.typed_signature(&text, &style),
            },
            Request::PlaceSignature {
                pages,
                image,
                center,
                width_pt,
                cancel,
            } => {
                let result = (|| {
                    let doc = state
                        .document
                        .as_mut()
                        .ok_or_else(|| EngineError::Other("no document".into()))?;
                    let snapshot = Snapshot {
                        bytes: doc.snapshot()?,
                        password: doc.password().map(str::to_owned),
                        pages_unchanged: true,
                        measurement_scales_unchanged: false,
                    };
                    doc.place_signature_image(
                        &pages,
                        &image,
                        center,
                        width_pt,
                        &cancel,
                        |done, total| {
                            let _ = tx.send(Response::TaskProgress {
                                kind: TaskKind::Signature,
                                done,
                                total,
                            });
                            ctx.request_repaint();
                        },
                    )?;
                    Ok::<_, EngineError>(snapshot)
                })();
                match result {
                    Ok(snapshot) => {
                        state.push_undo(snapshot);
                        state.invalidate();
                        state.last_nudge = None;
                        let _ = tx.send(Response::TaskDone {
                            kind: TaskKind::Signature,
                            result: Ok(()),
                        });
                        let mut changed = state.doc_state().expect("signature document");
                        changed.pages_unchanged = true;
                        Response::DocumentChanged(changed)
                    }
                    Err(error) => Response::TaskDone {
                        kind: TaskKind::Signature,
                        result: Err(error),
                    },
                }
            }
            Request::BeginProtection { epoch } => {
                let Some(doc) = state.document.as_ref() else {
                    continue;
                };
                Response::ProtectionAuthorization {
                    epoch,
                    encrypted: doc.is_encrypted(),
                }
            }
            request @ (Request::VerifySignatures | Request::VerifySignaturesWithOptions(_)) => {
                let options = match request {
                    Request::VerifySignaturesWithOptions(options) => options,
                    _ => sunipdf_core::certificate::VerificationOptions::default(),
                };
                let result = state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| {
                        let bytes = state.signature_bytes()?;
                        sunipdf_core::certificate::verify_signatures_with_options(
                            bytes,
                            doc.password(),
                            options,
                        )
                    });
                Response::Signatures(result)
            }
            Request::SignCertificate {
                pfx,
                password,
                options,
                output,
            } => {
                let result = (|| {
                    let source = state.signature_bytes()?;
                    // A fresh path also excludes hard-link aliases of the source.
                    if output.exists() {
                        return Err(EngineError::AlreadyExists(output));
                    }
                    let identity = zeroize::Zeroizing::new(std::fs::read(pfx)?);
                    let signed = sunipdf_core::certificate::sign_pdf(
                        source, &identity, &password, &options,
                    )?;
                    write_new_copy(&output, &signed)?;
                    Ok(output)
                })();
                Response::SignedCopy(result)
            }
            Request::OpenPlanSheet(navigation) => {
                let sheet = navigation.sheet;
                let path = sheet.path;
                let result = engine
                    .open_bytes(
                        sheet.bytes.clone(),
                        navigation.password.as_ref().map(|p| p.as_str()),
                        &path,
                    )
                    .and_then(|doc| {
                        if sheet.page == 0 || sheet.page > doc.page_count() {
                            return Err(EngineError::PageOutOfRange(sheet.page.saturating_sub(1)));
                        }
                        Ok((doc, SignatureSource::new(sheet.bytes)))
                    });
                match result {
                    Ok((doc, source)) => {
                        state.document = Some(doc);
                        state.signature_source = Some(source);
                        state.invalidate();
                        state.undo.clear();
                        state.redo.clear();
                        state.save_point = Some(0);
                        state.last_nudge = None;
                        Response::Opened(state.doc_state().expect("verified plan sheet"))
                    }
                    Err(error) => Response::OpenFailed { path, error },
                }
            }
            Request::Open { path, password } => match std::fs::read(&path)
                .map_err(EngineError::from)
                .and_then(|bytes| {
                    let doc = engine.open_bytes(bytes.clone(), password.as_deref(), &path)?;
                    Ok((doc, SignatureSource::new(bytes)))
                }) {
                Ok((doc, source)) => {
                    state.document = Some(doc);
                    state.signature_source = Some(source);
                    state.invalidate();
                    state.undo.clear();
                    state.redo.clear();
                    state.save_point = Some(0);
                    state.last_nudge = None;
                    Response::Opened(state.doc_state().expect("document ouvert"))
                }
                Err(error) => Response::OpenFailed { path, error },
            },
            Request::SetWanted { epoch, jobs } => {
                if state.document.is_some() {
                    state.wanted.replace(epoch, jobs);
                }
                continue;
            }
            Request::Thumbnail { page, width_px } => {
                let image = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| doc.page_size(page))
                    .and_then(|size| {
                        state.render_cached(page, width_px.max(1) as f32 / size.width_pt)
                    });
                match image {
                    Ok(image) => Response::Thumbnail { page, image },
                    Err(e) => {
                        log::warn!("miniature page {page} impossible : {e}");
                        Response::ThumbnailFailed { page, width_px }
                    }
                }
            }
            Request::Search {
                query,
                match_case,
                generation,
            } => {
                state.search = Some(SearchJob {
                    query,
                    match_case,
                    generation,
                    next_page: 0,
                    matches: Vec::new(),
                    rectangles: 0,
                });
                continue;
            }
            Request::CancelSearch { generation } => {
                if state
                    .search
                    .as_ref()
                    .is_some_and(|job| job.generation == generation)
                {
                    state.search = None;
                }
                continue;
            }
            Request::Edit(op) => state.edit(op),
            Request::CommitRedactionPreview { epoch, generation } => {
                let result = match state.redaction_preview.take() {
                    Some((saved_epoch, saved_generation, prepared))
                        if saved_epoch == epoch && saved_generation == generation =>
                    {
                        state.redact_candidate(|doc| doc.commit_redaction_preview(prepared))
                    }
                    _ => Err(EngineError::Other(
                        "redaction preview is stale; preview again".into(),
                    )),
                };
                match result {
                    Ok((report, doc_state)) => {
                        if let Some(slot) = &state.recovery {
                            let refreshed = state
                                .document
                                .as_ref()
                                .expect("redacted document")
                                .snapshot()
                                .and_then(|bytes| slot.save(&bytes).map_err(EngineError::from));
                            if let Err(error) = refreshed {
                                // A failed refresh must not retain a snapshot containing the removed data.
                                if let Err(cleanup) = slot.remove() {
                                    log::error!("Redaction recovery cleanup failed: {cleanup}");
                                }
                                let _ = tx.send(Response::RecoverySaved(Err(error)));
                            }
                        }
                        if tx.send(Response::Redacted(report)).is_err() {
                            break;
                        }
                        Response::DocumentChanged(doc_state)
                    }
                    Err(e) => Response::OperationFailed(e),
                }
            }
            Request::PreviewRedaction {
                marks,
                options,
                appearance,
                page,
                epoch,
                generation,
                cancel,
            } => {
                state.redaction_preview = None;
                let result = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| {
                        doc.preview_redaction_cancelable(
                            &marks,
                            &options,
                            &appearance,
                            page,
                            cancel,
                        )
                    });
                let result = result.and_then(|mut preview| {
                    let prepared = preview
                        .prepared
                        .take()
                        .ok_or_else(|| EngineError::Other("missing prepared redaction".into()))?;
                    state.redaction_preview = Some((epoch, generation, prepared));
                    Ok(preview)
                });
                Response::RedactionPreview {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::DiscardRedactionPreview { epoch, generation } => {
                if state.redaction_preview.as_ref().is_some_and(
                    |(saved_epoch, saved_generation, _)| {
                        *saved_epoch == epoch && *saved_generation == generation
                    },
                ) {
                    state.redaction_preview = None;
                }
                continue;
            }
            Request::GetRedactionMarks { epoch } => Response::RedactionMarks {
                epoch,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(Document::redaction_marks),
            },
            Request::PersistRedactionMarks {
                marks,
                appearance,
                author,
            } => {
                match state.redact_candidate(|doc| {
                    doc.persist_redaction_marks(&marks, &appearance, author.as_deref())?;
                    Ok(RedactReport::default())
                }) {
                    Ok((_, doc_state)) => Response::DocumentChanged(doc_state),
                    Err(error) => Response::OperationFailed(error),
                }
            }
            Request::RedactSearch {
                query,
                match_case,
                verify,
            } => {
                let Some(doc) = &state.document else { continue };
                let marks = doc
                    .redact_marks_for_text(&query, match_case)
                    .unwrap_or_else(|e| {
                        log::warn!("recherche à caviarder impossible : {e}");
                        Vec::new()
                    });
                Response::RedactMatches {
                    query,
                    verify,
                    marks,
                }
            }
            Request::TextInRect { page, rect } => {
                let Some(doc) = &state.document else { continue };
                match doc.text_in_rect(page, rect) {
                    Ok(selection) => Response::TextSelected { page, selection },
                    Err(e) => {
                        log::warn!("sélection impossible : {e}");
                        continue;
                    }
                }
            }
            Request::ReadingSelection {
                epoch,
                generation,
                anchor,
                focus,
                word,
            } => {
                let result = state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| doc.reading_selection(anchor, focus, word));
                Response::ReadingSelected {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::Batch(op) => {
                Response::BatchDone(run_batch(&engine, state.document.as_ref(), op))
            }
            Request::Suspend => {
                let Some(doc) = state.document.as_ref() else {
                    continue;
                };
                let bytes = match doc.snapshot() {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        navigation_failed = true;
                        let _ = tx.send(Response::SuspendFailed(error));
                        ctx.request_repaint();
                        continue;
                    }
                };
                if state.modified() {
                    if let Some(slot) = &state.recovery {
                        if let Err(error) = slot.save(&bytes) {
                            let _ = tx.send(Response::RecoverySaved(Err(error.into())));
                        }
                    }
                }
                let doc = state.document.take().expect("snapshotted document");
                let suspended = SuspendedDoc {
                    bytes,
                    password: doc.password().map(str::to_owned),
                    path: doc.info().path.clone(),
                    modified: state.modified(),
                    undo: std::mem::take(&mut state.undo),
                    redo: std::mem::take(&mut state.redo),
                    save_point: state.save_point.replace(0),
                    signature_source: state.signature_source.take(),
                };
                state.recovery = None;
                state.invalidate();
                Response::Suspended(suspended)
            }
            Request::Resume(index) => {
                log::debug!("reprise de l'onglet {index}");
                continue;
            }
            Request::ResumeDoc(s) => {
                // PDFium consumes its input even on failure. Keep the original bytes and
                // journal until a candidate document has opened successfully.
                match engine.open_bytes(s.bytes.clone(), s.password.as_deref(), &s.path) {
                    Ok(doc) => {
                        state.document = Some(doc);
                        state.signature_source = s.signature_source;
                        state.invalidate();
                        state.undo = s.undo;
                        state.redo = s.redo;
                        state.save_point = s.save_point;
                        state.last_nudge = None;
                        Response::Resumed(state.doc_state().expect("document repris"))
                    }
                    Err(error) => Response::ResumeFailed {
                        error,
                        suspended: s,
                    },
                }
            }
            Request::ObjectAt { page, x, y } => {
                let Some(doc) = &state.document else { continue };
                let info = doc.object_at(page, x, y).unwrap_or_default();
                Response::ObjectSelected { page, info }
            }
            Request::ObjectInfo { page, index } => {
                let Some(doc) = &state.document else { continue };
                // `usize::MAX` = dernier objet (après duplication ou passage au premier plan).
                let count = doc.page_objects(page).map(|v| v.len()).unwrap_or(0);
                let index = if count > 0 {
                    index.min(count - 1)
                } else {
                    index
                };
                let info = doc.object_info(page, index).ok();
                Response::ObjectSelected { page, info }
            }
            Request::Pick { page, pick, mode } => {
                let Some(doc) = &state.document else { continue };
                let infos = match pick {
                    Pick::At { x, y } => doc.object_at(page, x, y).map(|o| o.into_iter().collect()),
                    Pick::InRect { rect, touching } => doc.objects_in_rect(
                        page,
                        rect,
                        if touching {
                            RectMode::Touching
                        } else {
                            RectMode::Contained
                        },
                    ),
                    Pick::Indices(v) => Ok(v
                        .into_iter()
                        .filter_map(|i| doc.object_info(page, i).ok())
                        .collect()),
                    Pick::Last(n) => doc.page_objects(page).map(|all| {
                        let skip = all.len().saturating_sub(n);
                        all.into_iter().skip(skip).collect()
                    }),
                    Pick::All => doc.page_objects(page),
                };
                Response::ObjectsSelected {
                    page,
                    infos: infos.unwrap_or_default(),
                    mode,
                }
            }
            Request::Objects { page } => {
                let Some(doc) = &state.document else { continue };
                Response::Objects {
                    page,
                    list: doc.page_objects(page).unwrap_or_default(),
                }
            }
            Request::RotationCenter {
                page,
                indices,
                epoch,
                generation,
            } => {
                let center = state
                    .document
                    .as_ref()
                    .and_then(|doc| doc.rotation_center(page, &indices).ok());
                Response::RotationCenter {
                    page,
                    indices,
                    epoch,
                    generation,
                    center,
                }
            }
            Request::PreviewParagraph {
                page,
                indices,
                text,
                width,
                leading,
                epoch,
                generation,
            } => {
                let result = match state.document.as_ref() {
                    Some(doc) => doc.preview_paragraph(page, &indices, &text, width, leading),
                    None => Err(EngineError::PageOutOfRange(page)),
                };
                Response::ParagraphPreview {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::CopyObjects { page, indices, cut } => {
                let Some(doc) = &state.document else { continue };
                match doc.copy_objects(page, &indices) {
                    Ok(clip) => {
                        let (count, text) = (clip.count, clip.text.clone());
                        state.clip = Some(clip);
                        if !cut {
                            Response::Copied { count, text }
                        } else if tx.send(Response::Copied { count, text }).is_err() {
                            break;
                        } else {
                            state.edit(EditOp::DeleteObjects { page, indices })
                        }
                    }
                    Err(e) => Response::OperationFailed(e),
                }
            }
            Request::CaptureRectangleCalibration { page, calibration } => {
                state.rich_preview = None;
                // Capture at confirmation, never reconstruct evidence from a cached UI scale at preview.
                // Retain only the most recently confirmed page to bound snapshot memory.
                let result = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| doc.local_calibration_reference(page, &calibration))
                    .map_err(|error| error.to_string());
                // An unsuccessful capture replaces the previous token and is reported only
                // when this rectangle reference is requested; ordinary measurement remains usable.
                state.rectangle_calibration = Some(RectangleCalibrationCapture {
                    page,
                    calibration,
                    result,
                });
                continue;
            }
            Request::PreviewDimensionedRectangle {
                page,
                rectangle,
                author,
                epoch,
                generation,
            } => {
                use crate::measurement_shapes_ui::RectangleScaleSelection;
                use sunipdf_core::measurement::{DimensionedRectangle, RectangleScale};
                state.rich_preview = None;
                let result = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| {
                        let scale = match rectangle.scale {
                            RectangleScaleSelection::Local(calibration) => {
                                let capture =
                                    state.rectangle_calibration.as_ref().ok_or_else(|| {
                                        EngineError::Other(RECTANGLE_REFERENCE_REQUIRED.into())
                                    })?;
                                RectangleScale::Local(capture.reference(page, &calibration)?)
                            }
                            RectangleScaleSelection::Page => RectangleScale::Page,
                            RectangleScaleSelection::Region(id) => RectangleScale::Region(id),
                            RectangleScaleSelection::Automatic => RectangleScale::Automatic,
                        };
                        let annotation =
                            sunipdf_core::annot_rich::RichAnnotation::DimensionedRectangle(
                                DimensionedRectangle {
                                    anchor: rectangle.anchor,
                                    width: rectangle.width,
                                    height: rectangle.height,
                                    unit: rectangle.unit,
                                    precision: rectangle.precision,
                                    scale,
                                },
                            );
                        doc.preview_rich_annotation(page, &annotation, author.as_deref())
                    })
                    .and_then(|mut preview| {
                        let prepared = preview.prepared.take().ok_or_else(|| {
                            EngineError::Other(
                                "missing prepared dimensioned rectangle preview".into(),
                            )
                        })?;
                        state.rich_preview = Some((epoch, generation, prepared));
                        Ok(preview)
                    });
                Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::PreviewRichAnnotation {
                page,
                annotation,
                author,
                epoch,
                generation,
            } => {
                state.rich_preview = None;
                let result = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| {
                        doc.preview_rich_annotation(page, &annotation, author.as_deref())
                    });
                let result = result.and_then(|mut preview| {
                    let prepared = preview.prepared.take().ok_or_else(|| {
                        EngineError::Other("missing prepared annotation preview".into())
                    })?;
                    state.rich_preview = Some((epoch, generation, prepared));
                    Ok(preview)
                });
                Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::DiscardRichAnnotationPreview { epoch, generation } => {
                if state
                    .rich_preview
                    .as_ref()
                    .is_some_and(|(saved_epoch, saved_generation, _)| {
                        *saved_epoch == epoch && *saved_generation == generation
                    })
                {
                    state.rich_preview = None;
                }
                continue;
            }
            request @ (Request::PreviewMeasurementEdit { .. }
            | Request::PreviewMeasurementRecalibration { .. }) => {
                let (page, record, measurement, calibration, epoch, generation) = match request {
                    Request::PreviewMeasurementEdit {
                        page,
                        record,
                        measurement,
                        epoch,
                        generation,
                    } => (page, record, measurement, None, epoch, generation),
                    Request::PreviewMeasurementRecalibration {
                        page,
                        record,
                        measurement,
                        calibration,
                        epoch,
                        generation,
                    } => (
                        page,
                        record,
                        measurement,
                        Some(calibration),
                        epoch,
                        generation,
                    ),
                    _ => unreachable!(),
                };
                state.rich_preview = None;
                let result = state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .and_then(|doc| {
                        let target = doc.measurement_target(page, &record)?;
                        let annotation = if let Some(calibration) = calibration {
                            sunipdf_core::annot_rich::RichAnnotation::RecalibrateMeasurement {
                                target,
                                measurement,
                                calibration,
                            }
                        } else {
                            sunipdf_core::annot_rich::RichAnnotation::UpdateMeasurement {
                                target,
                                measurement,
                            }
                        };
                        doc.preview_rich_annotation(page, &annotation, None)
                    })
                    .and_then(|mut preview| {
                        let prepared = preview.prepared.take().ok_or_else(|| {
                            EngineError::Other("missing prepared measurement preview".into())
                        })?;
                        state.rich_preview = Some((epoch, generation, prepared));
                        Ok(preview)
                    });
                Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::LoadImage { path, max_side } => {
                let (path, result) = if let Some(dir) =
                    crate::sign::signatures_dir().filter(|dir| {
                        path.parent() == Some(dir.as_path())
                            && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                                n.starts_with("signature-") || n.starts_with("initials-")
                            })
                    }) {
                    match crate::sign::library::load(&dir, &path) {
                        Ok((path, image)) => (path, Ok(image)),
                        Err(error) => (path, Err(error)),
                    }
                } else {
                    let result = sunipdf_core::signature_image::load(&path, max_side);
                    (path, result)
                };
                Response::ImageLoaded { path, result }
            }
            Request::SystemFonts => Response::SystemFonts(
                state
                    .fonts
                    .get_or_insert_with(|| Arc::new(sunipdf_core::fonts::system_fonts()))
                    .clone(),
            ),
            Request::Properties => {
                let Some(doc) = &state.document else { continue };
                Response::Properties(doc.properties())
            }
            Request::Inspect { epoch } => {
                let result = state
                    .signature_source
                    .as_ref()
                    .filter(|source| {
                        source.bytes.len() <= sunipdf_core::inspection::MAX_INPUT_BYTES
                    })
                    .ok_or_else(|| {
                        EngineError::Other(
                            "PDF inspection input unavailable or exceeds 128 MiB".into(),
                        )
                    })
                    .and_then(|_| state.signature_bytes())
                    .and_then(|bytes| {
                        sunipdf_core::inspection::inspect(
                            bytes,
                            state.document.as_ref().and_then(|doc| doc.password()),
                        )
                    });
                Response::Inspection { epoch, result }
            }
            Request::Bookmarks => {
                let Some(doc) = &state.document else { continue };
                Response::Bookmarks(doc.bookmarks())
            }
            Request::PreviewRecalibration {
                requests,
                epoch,
                generation,
            } => Response::RecalibrationPreview {
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| doc.preview_recalibration(&requests)),
            },
            Request::MeasurementPage {
                page,
                epoch,
                generation,
            } => Response::MeasurementPage {
                page,
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or(EngineError::PageOutOfRange(page))
                    .map(|doc| {
                        let mut data = crate::measurement_saved::MeasurementPageData::new(
                            doc.page_calibration(page).map_err(|e| e.to_string()),
                            doc.measurement_records(page).map_err(|e| e.to_string()),
                            doc.properties().can_modify,
                        );
                        data.can_annotate = doc.properties().can_annotate;
                        data.viewports = doc
                            .measurement_viewports(page)
                            .map(Arc::from)
                            .map_err(|e| e.to_string());
                        if data.viewports.is_err() {
                            data.viewport_reset_target =
                                doc.measurement_viewport_reset_target(page).ok();
                        }
                        data.snap_geometry = doc
                            .vector_snap_geometry(page)
                            .map(Arc::new)
                            .map_err(|e| e.to_string());
                        data
                    }),
            },
            Request::LibraryCaptureCandidates {
                page,
                epoch,
                generation,
            } => Response::LibraryCaptureCandidates {
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| doc.library_capture_candidates(page)),
            },
            Request::CaptureLibraryEntry {
                page,
                selections,
                id,
                name,
                provenance,
                epoch,
                generation,
            } => Response::LibraryEntryCaptured {
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| {
                        doc.capture_library_entry(page, &selections, id, name, provenance)
                    }),
            },
            Request::QuantityWorkbook {
                epoch,
                generation,
                job,
            } => {
                let saved_sha = if state.modified() {
                    None
                } else {
                    state
                        .signature_source
                        .as_ref()
                        .map(SignatureSource::sha256_hex)
                };
                let result = match state.document.as_ref() {
                    Some(doc) => state.quantity_workbook.run(
                        doc,
                        epoch,
                        job,
                        saved_sha.as_deref(),
                        |progress| {
                            let _ = tx.send(Response::QuantityWorkbookProgress {
                                epoch,
                                generation,
                                progress,
                            });
                        },
                    ),
                    None => {
                        state.quantity_workbook.clear();
                        Err(EngineError::Other("no document".into()))
                    }
                };
                Response::QuantityWorkbook {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::IssueReport {
                epoch,
                generation,
                job,
            } => {
                let saved_sha = if state.modified() {
                    None
                } else {
                    state
                        .signature_source
                        .as_ref()
                        .map(SignatureSource::sha256_hex)
                };
                let result = match state.document.as_ref() {
                    Some(doc) => {
                        state
                            .issue_report
                            .run(doc, epoch, job, saved_sha.as_deref(), |progress| {
                                let _ = tx.send(Response::IssueReportProgress {
                                    epoch,
                                    generation,
                                    progress,
                                });
                            })
                    }
                    None => {
                        state.issue_report.clear();
                        Err(EngineError::Other("no document".into()))
                    }
                };
                Response::IssueReport {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::BoundaryFill {
                epoch,
                generation,
                job,
            } => {
                use crate::boundary_fill_ui::{Job, Outcome};
                use crate::measurement_shapes_ui::RectangleScaleSelection;
                use sunipdf_core::{
                    boundary_fill::BoundaryFillMeasurementOptions, measurement::RectangleScale,
                };
                let result = if matches!(&job, Job::Discard) {
                    state.boundary_fill.discard(epoch, generation);
                    Ok(Outcome::Discarded)
                } else {
                    (|| {
                        let doc = state
                            .document
                            .as_ref()
                            .ok_or_else(|| EngineError::Other("no document".into()))?;
                        if let Job::Prepare {
                            scale,
                            unit,
                            precision,
                        } = job
                        {
                            let page = state.boundary_fill.page().ok_or_else(|| {
                                EngineError::Other(
                                    "Find a region before preparing its measurement".into(),
                                )
                            })?;
                            let scale = match scale {
                                RectangleScaleSelection::Automatic => RectangleScale::Automatic,
                                RectangleScaleSelection::Page => RectangleScale::Page,
                                RectangleScaleSelection::Region(id) => RectangleScale::Region(id),
                                RectangleScaleSelection::Local(calibration) => {
                                    let capture =
                                        state.rectangle_calibration.as_ref().ok_or_else(|| {
                                            EngineError::Other(RECTANGLE_REFERENCE_REQUIRED.into())
                                        })?;
                                    RectangleScale::Local(capture.reference(page, &calibration)?)
                                }
                            };
                            state.boundary_fill.prepare(
                                doc,
                                epoch,
                                generation,
                                BoundaryFillMeasurementOptions {
                                    scale,
                                    unit,
                                    precision,
                                },
                            )
                        } else {
                            state.boundary_fill.run(doc, epoch, generation, job)
                        }
                    })()
                };
                Response::BoundaryFill {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::VisualSearch {
                epoch,
                generation,
                job,
            } => {
                let result = if matches!(&job, crate::visual_search_ui::Job::Discard) {
                    state.visual_search.discard(epoch, generation);
                    Ok(crate::visual_search_ui::Outcome::Discarded)
                } else {
                    match state.document.as_ref() {
                        Some(doc) => state.visual_search.run(doc, epoch, generation, job),
                        None => {
                            state.visual_search.clear();
                            Err(EngineError::Other("no document".into()))
                        }
                    }
                };
                Response::VisualSearch {
                    epoch,
                    generation,
                    result,
                }
            }
            Request::PlanZones {
                epoch,
                generation,
                job,
            } => Response::PlanZones {
                epoch,
                generation,
                result: match state.document.as_ref() {
                    Some(doc) => state.plan_zones.run(doc, epoch, generation, job),
                    None => {
                        state.plan_zones.clear();
                        Err(EngineError::Other("no document".into()))
                    }
                },
            },
            Request::QuantityTable { epoch, generation } => Response::QuantityTable {
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| doc.quantity_table()),
            },
            Request::ExportQuantityTable {
                table,
                options,
                path,
                csv,
                epoch,
                generation,
            } => Response::QuantityExported {
                epoch,
                generation,
                result: state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| doc.export_quantity_table(&table, &options))
                    .and_then(|export| {
                        let bytes = if csv {
                            export.csv.into_bytes()
                        } else {
                            serde_json::to_vec_pretty(&export.json)
                                .map_err(|error| EngineError::Other(error.to_string()))?
                        };
                        sunipdf_core::write_new_copy(&path, &bytes)
                    }),
            },
            Request::AnnotationRegister { epoch, generation } => {
                state.register = Some(RegisterJob {
                    epoch,
                    generation,
                    reader: None,
                });
                continue;
            }
            Request::CancelAnnotationRegister { epoch, generation } => {
                if state
                    .register
                    .as_ref()
                    .is_some_and(|job| job.epoch == epoch && job.generation == generation)
                {
                    state.register = None;
                }
                continue;
            }
            Request::Annotations { page, detailed } => {
                let Some(doc) = &state.document else { continue };
                let list = doc.annotations(page).unwrap_or_default();
                // La lecture détaillée relit tout le document : seulement si la page porte autre
                // chose que des fenêtres contextuelles et des champs de formulaire.
                let details = (detailed
                    && list.iter().any(|a| !matches!(a.kind, "popup" | "widget")))
                .then(|| doc.annotation_details(page).ok())
                .flatten();
                Response::Annotations {
                    page,
                    list,
                    details,
                }
            }
            Request::Links { page } => {
                let Some(doc) = &state.document else { continue };
                Response::Links {
                    page,
                    list: doc.page_links(page).unwrap_or_default(),
                }
            }
            Request::FormInfo => {
                let Some(doc) = &state.document else { continue };
                Response::FormInfo(doc.form_info())
            }
            Request::Undo => match state.undo_redo(true) {
                Some(r) => r,
                None => continue,
            },
            Request::Redo => match state.undo_redo(false) {
                Some(r) => r,
                None => continue,
            },
            Request::Save { path } => {
                let Some(doc) = &mut state.document else {
                    continue;
                };
                match doc.save_as_bytes(&path) {
                    Ok(bytes) => {
                        state.signature_source = Some(SignatureSource::new(bytes));
                        // « Enregistrer sous » : le document suit son nouveau chemin (Ctrl+S, onglets).
                        doc.set_path(path.clone());
                        state.save_point = Some(state.undo.len());
                        // Le prochain décalage clavier prend son propre instantané (sinon il serait
                        // fusionné avec celui d'avant l'enregistrement et passerait pour « intact »).
                        state.last_nudge = None;
                        if let Some(slot) = &state.recovery {
                            if let Err(error) = slot.remove() {
                                let _ = tx.send(Response::RecoverySaved(Err(error.into())));
                            }
                        }
                        Response::Saved { path }
                    }
                    Err(e) => Response::SaveFailed(e),
                }
            }
            Request::FinishMutation => Response::MutationFinished,
            Request::Close => {
                if let Some(slot) = state.recovery.take() {
                    if let Err(error) = slot.remove() {
                        let _ = tx.send(Response::RecoverySaved(Err(error.into())));
                    }
                }
                state.document = None;
                state.signature_source = None;
                state.invalidate();
                state.undo.clear();
                state.redo.clear();
                state.save_point = Some(0);
                state.last_nudge = None;
                continue;
            }
        };
        if tx.send(response).is_err() {
            break;
        }
        ctx.request_repaint();
        // 3. Vider la file d'attente avant tout pré-rendu.
        loop {
            match rx.try_recv() {
                Ok(r) => queue.push_back(r),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::{FieldKind, FieldOption, PixelRect};

    #[test]
    fn document_worker_integrations_preserve_history_permissions_and_responsiveness() {
        let worker = Worker::spawn(egui::Context::default());
        let timeout = std::time::Duration::from_secs(10);
        match worker
            .rx
            .recv_timeout(timeout)
            .expect("engine startup response")
        {
            Response::EngineReady => {}
            Response::EngineFailed(error) => panic!("engine startup failed: {error}"),
            _ => panic!("unexpected startup response"),
        }
        let signature = render_signature_preview(&worker, 41);
        assert!(!worker.has_pending_mutations());
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(
            worker.rx.recv_timeout(timeout).unwrap(),
            Response::Opened(_)
        ));
        worker.send(Request::FormInfo);
        assert!(matches!(
            worker.rx.recv_timeout(timeout).unwrap(),
            Response::FormInfo(Ok(info)) if info == FormInfo::default()
        ));
        assert!(matches!(
            worker.rx.recv_timeout(timeout).unwrap(),
            Response::MutationFinished
        ));
        worker.finish_mutation();
        for (page, width_px) in [(usize::MAX, 144), (0, u32::MAX)] {
            worker.send(Request::Thumbnail { page, width_px });
            assert!(matches!(
                worker.rx.recv_timeout(timeout).expect("failed thumbnails must release their request quota"),
                Response::ThumbnailFailed { page: failed_page, width_px: failed_width }
                    if failed_page == page && failed_width == width_px
            ));
        }
        page_label_edits_and_history_keep_page_geometry(&worker);
        bookmark_edits_have_one_undo_and_failed_edits_keep_history(&worker);
        link_target_edits_have_one_undo_and_preserve_geometry(&worker);
        existing_text_missing_glyphs_keep_history_and_original_objects(&worker);
        grouped_ink_has_one_undo_redo_and_failed_edits_keep_history(&worker);
        dashed_annotation_styles_survive_edits_and_history(&worker);
        signature_batch_placement_has_one_undo_and_no_partial_failures(&worker, signature);
        drawn_form_rectangles_keep_history_and_saved_fields(&worker);
        inline_free_text_has_exact_styles_geometry_history_and_saved_content(&worker);
        paragraph_preview_and_reflow_keep_one_undo_and_failed_edit_history(&worker);
        native_clipboard_paste_across_documents_has_one_undo_and_no_silent_group_fallback(&worker);
        rich_annotation_previews_commit_exactly_once_and_reject_stale_or_discarded_candidates(
            &worker,
        );
        editable_combo_creation_and_filling_undo_redo(&worker);
        prepared_redaction_has_one_undo_and_cancellation_keeps_history(&worker);
        encrypted_recovery_after_redaction(&worker);
        signatures_bind_to_loaded_saved_and_resumed_bytes(&worker);
        failed_resume_returns_unsaved_bytes_and_journal_for_a_retry(&worker);
        reprotection_requires_the_current_owner_without_removing_existing_restrictions(&worker);
        inspection_uses_saved_bytes_and_refuses_unsaved_changes(&worker);
        count_register_edits_have_one_undo_and_stale_edits_add_no_history(&worker);
        page_calibration_edits_have_history_and_survive_reopening(&worker);
        saved_measurement_edits_replace_in_place_and_keep_history(&worker);
        deducted_measurements_round_trip_through_guarded_worker_history(&worker);
        dimensioned_rectangles_preview_commit_and_revalidate_the_captured_reference(&worker);
        library_groups_use_one_preview_and_one_history_step(&worker);
        revision_overlay_jobs_preserve_sources_and_guard_exports(&worker);
        quantity_fields_have_history_exports_and_guarded_desktop_controls(&worker);
        protected_plan_sets_use_one_operation_credentials_and_open_verified_bytes(&worker);
        reviewed_plan_links_preview_both_pages_and_publish_only_fresh_copies(&worker);
        cooperative_search_and_register_publish_complete_generation_scoped_results(&worker);
        revision_transfer_candidates_stay_in_worker_and_revalidate_before_publication(&worker);
        revision_outlines_keep_candidates_in_worker_and_revalidate_saved_sources(&worker);
        plan_zones_preview_apply_history_reopen_and_stale_guard(&worker);
        issue_reports_preserve_filters_history_source_and_refuse_stale_publication(&worker);
        quantity_workbooks_refresh_reviewed_identities_and_keep_history(&worker);
        visual_search_reviewed_counts_have_one_undo_and_reject_stale_tokens(&worker);
        boundary_fill_preserves_deductions_history_and_rejects_incomplete_or_stale_results(&worker);
    }

    fn quantity_workbooks_refresh_reviewed_identities_and_keep_history(worker: &Worker) {
        use crate::quantity_workbook_ui::{Job, Outcome};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::quantity_workbook::{
            QuantityWorkbookOptions, WorkbookChangeStatus, WorkbookFormat,
        };
        fn run(
            worker: &Worker,
            epoch: u64,
            generation: u64,
            job: Job,
        ) -> Result<Outcome, EngineError> {
            worker.send(Request::QuantityWorkbook {
                epoch,
                generation,
                job,
            });
            match next_response(worker) {
                Response::QuantityWorkbook { result, .. } => result,
                _ => panic!("workbook response"),
            }
        }
        fn prepare(previous: Option<PathBuf>, include_new: bool) -> Job {
            Job::Prepare {
                table: None,
                options: Box::new(QuantityWorkbookOptions::default()),
                previous,
                include_new,
                cancel: Arc::new(AtomicBool::new(false)),
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.pdf");
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&source, &original).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        for x in [40., 80.] {
            worker.send(Request::Edit(EditOp::AddCountMarker {
                page: 0,
                point: (x, 70.),
                size: 10.,
                group: "Lighting".into(),
                author: None,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        let epoch = 1200;
        let token = match run(worker, epoch, 1, prepare(None, false)).unwrap() {
            Outcome::Preview { token, model, .. } => {
                assert_eq!(model.quantities["rows"].as_array().unwrap().len(), 2);
                assert_eq!(model.refreshable_rows, 2);
                assert!(
                    model.provenance.saved_file_sha256.is_none(),
                    "unsaved edits use snapshot identity"
                );
                token
            }
            _ => panic!("preview"),
        };
        let manifest = dir.path().join("baseline.json");
        let workbook = dir.path().join("quantities.xlsx");
        for (generation, format, out) in [
            (2, WorkbookFormat::Manifest, manifest.clone()),
            (3, WorkbookFormat::Xlsx, workbook.clone()),
        ] {
            let outcome = run(
                worker,
                epoch,
                generation,
                Job::Publish {
                    token,
                    format,
                    out,
                    cancel: Arc::new(AtomicBool::new(false)),
                },
            )
            .unwrap();
            assert!(matches!(outcome, Outcome::Published(_)));
        }
        let baseline = std::fs::read(&manifest).unwrap();
        let xlsx = std::fs::read(&workbook).unwrap();
        assert!(xlsx.starts_with(b"PK") && xlsx.len() > 200);
        assert!(run(
            worker,
            epoch,
            4,
            Job::Publish {
                token,
                format: WorkbookFormat::Xlsx,
                out: workbook.clone(),
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        assert_eq!(std::fs::read(&workbook).unwrap(), xlsx);
        let stale = dir.path().join("stale.xlsx");
        assert!(run(
            worker,
            epoch + 1,
            5,
            Job::Publish {
                token,
                format: WorkbookFormat::Xlsx,
                out: stale.clone(),
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        assert!(!stale.exists());
        worker.send(Request::Edit(EditOp::AddCountMarker {
            page: 0,
            point: (120., 70.),
            size: 10.,
            group: "New".into(),
            author: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(run(
            worker,
            epoch,
            6,
            Job::Publish {
                token,
                format: WorkbookFormat::Xlsx,
                out: stale.clone(),
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        for (generation, include_new, expected) in [(7, false, 2), (8, true, 3)] {
            match run(
                worker,
                epoch,
                generation,
                prepare(Some(manifest.clone()), include_new),
            )
            .unwrap()
            {
                Outcome::Preview { model, .. } => {
                    assert!(model.refresh);
                    assert_eq!(model.quantities["rows"].as_array().unwrap().len(), expected);
                    assert!(model.changes.iter().any(|change| change.status
                        == if include_new {
                            WorkbookChangeStatus::Added
                        } else {
                            WorkbookChangeStatus::ExcludedNew
                        }));
                }
                _ => panic!("refresh preview"),
            }
        }
        assert!(matches!(
            run(
                worker,
                epoch,
                9,
                Job::Prepare {
                    table: None,
                    options: Box::default(),
                    previous: None,
                    include_new: false,
                    cancel: Arc::new(AtomicBool::new(true))
                }
            ),
            Err(EngineError::Cancelled)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        match run(worker, epoch, 10, prepare(None, false)).unwrap() {
            Outcome::Preview { model, .. } => assert_eq!(
                model.quantities["rows"].as_array().unwrap().len(),
                2,
                "report operations do not add history steps"
            ),
            _ => panic!("after undo"),
        }
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&manifest).unwrap(), baseline);
        assert_eq!(std::fs::read(&workbook).unwrap(), xlsx);
    }

    fn issue_reports_preserve_filters_history_source_and_refuse_stale_publication(worker: &Worker) {
        use crate::issue_report_ui::{Input, Job, Outcome};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::{
            annotation_register::{RegisterFilter, RegisterSort},
            issue_report::{IssueReportFormat, IssueReportOptions, IssueReportSelection},
            quantity_table::{FormulaColumn, QuantityGrouping, QuantityOptions, QuantityUnit},
        };
        fn run(worker: &Worker, generation: u64, job: Job) -> Result<Outcome, EngineError> {
            worker.send(Request::IssueReport {
                epoch: 1100,
                generation,
                job,
            });
            match next_response(worker) {
                Response::IssueReport { result, .. } => result,
                _ => panic!("report response"),
            }
        }
        fn table(worker: &Worker) -> Arc<sunipdf_core::quantity_table::QuantityTable> {
            worker.send(Request::QuantityTable {
                epoch: 1100,
                generation: 1,
            });
            match next_response(worker) {
                Response::QuantityTable {
                    result: Ok(table), ..
                } => Arc::new(table),
                _ => panic!("table"),
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.pdf");
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&source, &bytes).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        for (x, group) in [(40., "Included"), (80., "Excluded")] {
            worker.send(Request::Edit(EditOp::AddCountMarker {
                page: 0,
                point: (x, 70.),
                size: 10.,
                group: group.into(),
                author: None,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        let original = table(worker);
        let options = IssueReportOptions {
            title: "Selected report".into(),
            include_legend: true,
            selection: IssueReportSelection::Quantities,
            quantities: QuantityOptions {
                filter: RegisterFilter {
                    count_group: "Included".into(),
                    include_replies: true,
                    ..Default::default()
                },
                sort: RegisterSort::Kind,
                descending: true,
                group_by: QuantityGrouping::CountGroup,
                formulas: vec![FormulaColumn {
                    name: "Double".into(),
                    expression: "quantity * 2".into(),
                    quantity_unit: Some(QuantityUnit::Each),
                    rounding_decimals: 2,
                }],
            },
        };
        let expected = original.json(&options.quantities).unwrap();
        let (token, pages) = match run(
            worker,
            1,
            Job::Prepare {
                input: Input::Quantities(original.clone()),
                options: Box::new(options.clone()),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        )
        .unwrap()
        {
            Outcome::Preview {
                token,
                model,
                page,
                image,
            } => {
                assert_eq!(page, 0);
                assert!(model.page_count > 0);
                assert_eq!(
                    model.quantities, expected,
                    "same visible selection, ordering, groups and formulas"
                );
                assert!(
                    model.provenance.saved_file_sha256.is_none(),
                    "dirty snapshot is not a saved file hash"
                );
                assert!(
                    image.width > 0
                        && image.height > 0
                        && image.rgba.len() == image.width as usize * image.height as usize * 4
                );
                assert!(
                    image
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| pixel[0] < 220 && pixel[3] > 0),
                    "actual report text must be rendered"
                );
                assert_eq!(model.legend.iter().map(|l| l.count).sum::<usize>(), 1);
                (token, model.page_count)
            }
            _ => panic!("preview"),
        };
        assert!(run(
            worker,
            2,
            Job::Render {
                token,
                page: pages,
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        assert!(matches!(
            run(
                worker,
                3,
                Job::Render {
                    token,
                    page: 0,
                    cancel: Arc::new(AtomicBool::new(false))
                }
            )
            .unwrap(),
            Outcome::Preview { .. }
        ));
        for (generation, format, extension) in [
            (4, IssueReportFormat::Pdf, "pdf"),
            (5, IssueReportFormat::Csv, "csv"),
            (6, IssueReportFormat::Json, "json"),
        ] {
            let out = dir.path().join(format!("report.{extension}"));
            let receipt = match run(
                worker,
                generation,
                Job::Publish {
                    token,
                    format,
                    out: out.clone(),
                    cancel: Arc::new(AtomicBool::new(false)),
                },
            )
            .unwrap()
            {
                Outcome::Published(r) => r,
                _ => panic!("published"),
            };
            assert_eq!(receipt.matched_annotations, 1);
            let data = std::fs::read(&out).unwrap();
            assert_eq!(receipt.bytes, data.len());
            assert_eq!(
                receipt.output_sha256,
                Sha256::digest(&data)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            if extension == "pdf" {
                assert_eq!(
                    lopdf::Document::load(&out).unwrap().get_pages().len(),
                    pages
                );
            }
            if extension == "json" {
                let value: serde_json::Value = serde_json::from_slice(&data).unwrap();
                assert_eq!(value["quantities"], expected);
            }
            if extension == "csv" {
                let text = String::from_utf8(data.clone()).unwrap();
                assert!(text.contains("Included") && !text.contains("Excluded"));
            }
            assert!(run(
                worker,
                generation + 10,
                Job::Publish {
                    token,
                    format,
                    out: out.clone(),
                    cancel: Arc::new(AtomicBool::new(false))
                }
            )
            .is_err());
            assert_eq!(std::fs::read(&out).unwrap(), data);
        }
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        assert_eq!(
            table(worker).json(&options.quantities).unwrap()["rows"],
            expected["rows"]
        );
        worker.send(Request::Undo);
        assert!(
            matches!(next_response(worker),Response::DocumentChanged(ref state) if state.modified && state.can_redo)
        );
        assert_eq!(
            table(worker).rows().len(),
            1,
            "exports did not add undo history"
        );
        let stale_out = dir.path().join("stale.pdf");
        assert!(run(
            worker,
            30,
            Job::Publish {
                token,
                format: IssueReportFormat::Pdf,
                out: stale_out.clone(),
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        assert!(!stale_out.exists());
        assert!(
            run(
                worker,
                31,
                Job::Prepare {
                    input: Input::Quantities(original),
                    options: Box::new(options.clone()),
                    cancel: Arc::new(AtomicBool::new(false))
                }
            )
            .is_err(),
            "old visible table is rejected"
        );
        assert!(matches!(
            run(
                worker,
                32,
                Job::Prepare {
                    input: Input::Current,
                    options: Box::new(options.clone()),
                    cancel: Arc::new(AtomicBool::new(true))
                }
            ),
            Err(EngineError::Cancelled)
        ));
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let token = match run(
            worker,
            40,
            Job::Prepare {
                input: Input::Current,
                options: Box::default(),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        )
        .unwrap()
        {
            Outcome::Preview { token, model, .. } => {
                assert_eq!(
                    model.provenance.saved_file_sha256.as_deref(),
                    Some(
                        Sha256::digest(&bytes)
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                            .as_str()
                    )
                );
                token
            }
            _ => panic!("saved preview"),
        };
        let mut changed = bytes.clone();
        changed.extend_from_slice(b"\n% externally modified\n");
        std::fs::write(&source, &changed).unwrap();
        let out = dir.path().join("changed-source.pdf");
        assert!(run(
            worker,
            41,
            Job::Publish {
                token,
                format: IssueReportFormat::Pdf,
                out: out.clone(),
                cancel: Arc::new(AtomicBool::new(false))
            }
        )
        .is_err());
        assert!(!out.exists());
        assert_eq!(std::fs::read(&source).unwrap(), changed);

        // Register text fields and quantity typed fields have different search and sort rules.
        // The report must retain the visible register selection, rather than broadening it.
        use sunipdf_core::annotation_register::{AnnotationRegister, RegisterFields};
        use sunipdf_core::quantity_table::{DecimalValue, QuantityFields, TypedValue, UnitPrice};
        fn register(worker: &Worker) -> Arc<AnnotationRegister> {
            worker.send(Request::AnnotationRegister {
                epoch: 1100,
                generation: 50,
            });
            match next_response(worker) {
                Response::AnnotationRegister {
                    result: Ok(report), ..
                } => Arc::new(report),
                _ => panic!("register"),
            }
        }
        let register_source = dir.path().join("register-source.pdf");
        std::fs::write(&register_source, &bytes).unwrap();
        worker.send(Request::Open {
            path: register_source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        for x in [40., 80., 120.] {
            worker.send(Request::Edit(EditOp::AddCountMarker {
                page: 0,
                point: (x, 70.),
                size: 10.,
                group: "Shared".into(),
                author: None,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        for (index, room, typed_room) in [(0, "Z", "A"), (1, "A", "Z"), (2, "M", "M")] {
            let current = register(worker);
            worker.send(Request::Edit(EditOp::SetAnnotationRegisterFields {
                target: current.target(&current.rows[index]),
                fields: RegisterFields {
                    custom: [
                        ("Room".into(), room.into()),
                        (
                            "Selection".into(),
                            if index < 2 {
                                "visible-selection"
                            } else {
                                "other"
                            }
                            .into(),
                        ),
                    ]
                    .into(),
                    ..Default::default()
                },
                count_group: Some("Shared".into()),
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            let current = table(worker);
            worker.send(Request::Edit(EditOp::SetQuantityFields {
                target: current.target(&current.rows()[index]).unwrap(),
                fields: QuantityFields {
                    unit_price: Some(UnitPrice {
                        amount: DecimalValue::parse("10").unwrap(),
                        currency: "CHF".into(),
                        per_unit: QuantityUnit::Each,
                        rounding_decimals: 2,
                    }),
                    custom: [
                        (
                            "Room".into(),
                            TypedValue::Text {
                                value: typed_room.into(),
                            },
                        ),
                        (
                            "TypedOnly".into(),
                            TypedValue::Text {
                                value: if index == 2 {
                                    "visible-selection"
                                } else {
                                    "other"
                                }
                                .into(),
                            },
                        ),
                    ]
                    .into(),
                },
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        let visible = register(worker);
        let mut register_options = options.clone();
        register_options.quantities.filter = RegisterFilter {
            query: "visible-selection".into(),
            custom_sort: Some("Room".into()),
            ..Default::default()
        };
        register_options.quantities.sort = RegisterSort::Page;
        register_options.quantities.descending = false;
        let expected = visible
            .select(
                &register_options.quantities.filter,
                RegisterSort::Page,
                false,
            )
            .iter()
            .map(|row| row.annotation_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), 2);
        assert_eq!(expected[0], visible.rows[1].annotation_id);
        assert_eq!(expected[1], visible.rows[0].annotation_id);
        let (register_token, register_model) = match run(
            worker,
            51,
            Job::Prepare {
                input: Input::Register(visible),
                options: Box::new(register_options.clone()),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        )
        .unwrap()
        {
            Outcome::Preview { token, model, .. } => (token, model),
            _ => panic!("register report preview"),
        };
        assert!(matches!(
            register_model.selection,
            IssueReportSelection::Register
        ));
        let actual = register_model.quantities["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["register"]["annotation_id"].as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "visible register rows and text-field order remain exact"
        );
        assert_eq!(register_model.quantities["summary"]["matched_rows"], 2);
        assert_eq!(
            register_model.quantities["summary"]["groups"][0]["quantity"],
            2.0
        );
        assert_eq!(
            register_model.quantities["summary"]["groups"][0]["costs"][0]["total"]["amount"],
            "20.00"
        );
        assert_eq!(
            register_model.quantities["formula_rows"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            register_model.quantities["summary"]["formula_groups"][0]["totals"][0]["amount"],
            "4.00"
        );
        let out = dir.path().join("register-report.json");
        assert!(matches!(
            run(
                worker,
                52,
                Job::Publish {
                    token: register_token,
                    format: IssueReportFormat::Json,
                    out: out.clone(),
                    cancel: Arc::new(AtomicBool::new(false)),
                }
            )
            .unwrap(),
            Outcome::Published(_)
        ));
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out).unwrap()).unwrap();
        assert_eq!(saved["quantities"], register_model.quantities);
        register_options.selection = IssueReportSelection::Register;
        match run(
            worker,
            53,
            Job::Prepare {
                input: Input::Quantities(table(worker)),
                options: Box::new(register_options),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        )
        .unwrap()
        {
            Outcome::Preview { model, .. } => {
                assert!(matches!(model.selection, IssueReportSelection::Quantities));
                assert_eq!(model.quantities["rows"].as_array().unwrap().len(), 3);
            }
            _ => panic!("quantity report preview"),
        }
        assert_eq!(std::fs::read(register_source).unwrap(), bytes);
    }

    fn plan_zones_preview_apply_history_reopen_and_stale_guard(worker: &Worker) {
        use crate::plan_zones_ui::Job;
        use sunipdf_core::plan_zones::{PlanZone, ZoneAssignmentStatus};
        fn read(worker: &Worker, generation: u64, job: Job) -> crate::plan_zones_ui::PageData {
            worker.send(Request::PlanZones {
                epoch: 980,
                generation,
                job,
            });
            match next_response(worker) {
                Response::PlanZones {
                    result: Ok(Some(data)),
                    ..
                } => data,
                _ => panic!("expected zone page"),
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.pdf");
        let output = dir.path().join("zones.pdf");
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&source, &original).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Edit(EditOp::AddCountMarker {
            page: 0,
            point: (40., 40.),
            size: 8.,
            group: "Lights".into(),
            author: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let zones = vec![PlanZone {
            id: "room-1".into(),
            name: "Room one".into(),
            parent_id: None,
            bounds: [10., 10., 100., 100.],
        }];
        let preview = read(
            worker,
            1,
            Job::Preview {
                page: 0,
                zones: zones.clone(),
                reset_stale: false,
            },
        );
        assert_eq!(
            preview.preview.unwrap().assignments[0].after.status,
            ZoneAssignmentStatus::Assigned
        );
        worker.send(Request::Edit(EditOp::CommitPlanZones {
            epoch: 980,
            generation: 1,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        assert_eq!(read(worker, 2, Job::Read { page: 0 }).page.zones, zones);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(read(worker, 3, Job::Read { page: 0 }).page.zones.is_empty());
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(worker, 4, Job::Read { page: 0 }).page.zones, zones);
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read(worker, 5, Job::Read { page: 0 }).page.zones, zones);
        read(
            worker,
            6,
            Job::Preview {
                page: 0,
                zones: Vec::new(),
                reset_stale: false,
            },
        );
        worker.send(Request::Edit(EditOp::Note {
            page: 0,
            x: 140.,
            y: 140.,
            text: "Later edit".into(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitPlanZones {
            epoch: 980,
            generation: 6,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(read(worker, 7, Job::Read { page: 0 }).page.zones, zones);
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }

    fn revision_outlines_keep_candidates_in_worker_and_revalidate_saved_sources(worker: &Worker) {
        use crate::revision_outlines_ui::{Job, Outcome};
        use sunipdf_core::revision_outlines::{
            OutlineClassification, OutlineOptions, OutlineReview, OutlineSelection,
        };
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.pdf");
        let new = dir.path().join("new.pdf");
        let fixture = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&old, &fixture).unwrap();
        std::fs::write(&new, &fixture).unwrap();
        worker.send(Request::Open {
            path: new.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Edit(EditOp::AddAnnotShape {
            page: 0,
            shape: AnnotShape::Square,
            rect: PointRect {
                left: 100.0,
                bottom: 100.0,
                right: 140.0,
                top: 140.0,
            },
            style: AnnotStyle::default(),
            author: Some("Local QA".into()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Save { path: new.clone() });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        let new_bytes = std::fs::read(&new).unwrap();
        let cancel = || Arc::new(std::sync::atomic::AtomicBool::new(false));
        let run = |generation, job| {
            worker.send(Request::RevisionOutlines { generation, job });
            loop {
                match next_response(worker) {
                    Response::RevisionOutlinesProgress {
                        generation: actual,
                        progress,
                    } => {
                        assert_eq!(actual, generation);
                        assert!(progress.done <= progress.total);
                    }
                    Response::RevisionOutlinesDone {
                        generation: actual,
                        result,
                    } => {
                        assert_eq!(actual, generation);
                        break *result;
                    }
                    _ => panic!("revision outline response"),
                }
            }
        };
        let inspection = match run(
            1200,
            Job::Inspect {
                paths: [old.clone(), new.clone()],
                options: OutlineOptions {
                    long_side: 256,
                    min_component_pixels: 1,
                    ..Default::default()
                },
                passwords: Default::default(),
                cancel: cancel(),
            },
        )
        .unwrap()
        {
            Outcome::Inspected(i) => i,
            _ => panic!("inspection"),
        };
        assert!(!inspection.regions.is_empty());
        let review = OutlineReview {
            version: 1,
            old_sha256: inspection.old_sha256.clone(),
            new_sha256: inspection.new_sha256.clone(),
            options: inspection.options.clone(),
            selections: inspection
                .regions
                .iter()
                .map(|r| OutlineSelection {
                    id: r.id.clone(),
                    fingerprint: r.fingerprint.clone(),
                    classification: OutlineClassification::Uncertain,
                    confirmed: true,
                })
                .collect(),
        };
        let prepare = |generation| {
            run(
                generation,
                Job::Prepare {
                    paths: [old.clone(), new.clone()],
                    review: review.clone(),
                    passwords: Default::default(),
                    cancel: cancel(),
                },
            )
        };
        assert!(
            matches!(prepare(1201).unwrap(),Outcome::Preview{token:1201,image,report}if image.width>0&&!report.created.is_empty())
        );
        assert!(matches!(
            run(1202, Job::Discard).unwrap(),
            Outcome::Discarded
        ));
        let output = dir.path().join("outlines.pdf");
        assert!(run(
            1203,
            Job::Save {
                token: 1201,
                output: output.clone(),
                cancel: cancel()
            }
        )
        .is_err());
        assert!(!output.exists());
        assert!(prepare(1204).is_ok());
        let mut changed = new_bytes.clone();
        changed.extend_from_slice(b"\n% after-preview change\n");
        std::fs::write(&new, &changed).unwrap();
        assert!(run(
            1205,
            Job::Save {
                token: 1204,
                output: output.clone(),
                cancel: cancel()
            }
        )
        .is_err());
        assert!(!output.exists());
        std::fs::write(&new, &new_bytes).unwrap();
        assert!(prepare(1206).is_ok());
        let report = match run(
            1207,
            Job::Save {
                token: 1206,
                output: output.clone(),
                cancel: cancel(),
            },
        )
        .unwrap()
        {
            Outcome::Saved(r) => r,
            _ => panic!("saved"),
        };
        assert_eq!(report.output_path.unwrap(), output.canonicalize().unwrap());
        assert_eq!(std::fs::read(&old).unwrap(), fixture);
        assert_eq!(std::fs::read(&new).unwrap(), new_bytes);
        assert!(!worker.has_pending_mutations());
        // A loaded source mutation after inspection is refused even when its saved file is unchanged.
        worker.send(Request::Edit(EditOp::AddAnnotShape {
            page: 0,
            shape: AnnotShape::Square,
            rect: PointRect {
                left: 200.0,
                bottom: 200.0,
                right: 240.0,
                top: 240.0,
            },
            style: AnnotStyle::default(),
            author: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(prepare(1208).is_err());
    }

    fn revision_transfer_candidates_stay_in_worker_and_revalidate_before_publication(
        worker: &Worker,
    ) {
        use crate::revision_transfer_ui::{Job, Outcome, Sources};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::revision_transfer::{
            RevisionTransferRequest, TransferAlignment, TransferPagePair, TransferSelection,
        };
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.pdf");
        let new = dir.path().join("new.pdf");
        let fixture = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&old, &fixture).unwrap();
        std::fs::write(&new, &fixture).unwrap();
        worker.send(Request::Open {
            path: old.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Edit(EditOp::AddAnnotShape {
            page: 0,
            shape: AnnotShape::Square,
            rect: PointRect {
                left: 50.0,
                bottom: 50.0,
                right: 100.0,
                top: 100.0,
            },
            style: AnnotStyle::default(),
            author: Some("Local QA".into()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Save { path: old.clone() });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        let old_bytes = std::fs::read(&old).unwrap();
        let run = |generation, job| {
            worker.send(Request::RevisionTransfer { generation, job });
            match next_response(worker) {
                Response::RevisionTransferDone {
                    generation: actual,
                    result,
                } => {
                    assert_eq!(actual, generation);
                    result
                }
                _ => panic!("revision transfer response"),
            }
        };
        let cancel = || Arc::new(AtomicBool::new(false));
        let inspection = match run(
            1100,
            Job::Inspect {
                sources: Sources::Files([old.clone(), new.clone()]),
                pairs: vec![(1, 1), (2, 2)],
                passwords: Default::default(),
                cancel: cancel(),
            },
        )
        .unwrap()
        {
            Outcome::Inspected { inspection, .. } => inspection,
            _ => panic!("inspection"),
        };
        let candidate = inspection.pages[0]
            .candidates
            .iter()
            .find(|c| c.refusal.is_none() && c.fingerprint.is_some())
            .unwrap();
        let request = RevisionTransferRequest {
            version: 1,
            old_sha256: inspection.old_sha256,
            new_sha256: inspection.new_sha256,
            acknowledge_repeated_transfer: false,
            pairs: vec![
                TransferPagePair {
                    old_page: 1,
                    new_page: 1,
                    old_sheet_id: None,
                    new_sheet_id: None,
                    confirmed: true,
                    alignment: TransferAlignment::Identity,
                    selections: vec![TransferSelection {
                        index: candidate.index,
                        fingerprint: candidate.fingerprint.clone().unwrap(),
                    }],
                },
                TransferPagePair {
                    old_page: 2,
                    new_page: 2,
                    old_sheet_id: None,
                    new_sheet_id: None,
                    confirmed: true,
                    alignment: TransferAlignment::Identity,
                    selections: vec![],
                },
            ],
        };
        let prepare = |generation| {
            run(
                generation,
                Job::Prepare {
                    paths: [old.clone(), new.clone()],
                    request: request.clone(),
                    page: 1,
                    passwords: Default::default(),
                    cancel: cancel(),
                },
            )
        };
        assert!(
            matches!(prepare(1101).unwrap(), Outcome::Preview { token: 1101, page: 1, image, report } if image.width > 0 && report.transferred.len() == 1)
        );
        assert!(matches!(
            run(
                1102,
                Job::Render {
                    token: 1101,
                    page: 2,
                    password: Default::default(),
                    cancel: cancel()
                }
            )
            .unwrap(),
            Outcome::Preview {
                token: 1101,
                page: 2,
                ..
            }
        ));
        assert!(matches!(
            run(1103, Job::Discard).unwrap(),
            Outcome::Discarded
        ));
        assert!(run(
            1104,
            Job::Save {
                token: 1101,
                output: dir.path().join("discarded.pdf"),
                cancel: cancel()
            }
        )
        .is_err());
        assert!(!dir.path().join("discarded.pdf").exists());
        assert!(prepare(1105).is_ok());
        let mut changed = fixture.clone();
        changed.extend_from_slice(b"\n% changed after preview\n");
        std::fs::write(&new, &changed).unwrap();
        assert!(run(
            1106,
            Job::Save {
                token: 1105,
                output: dir.path().join("stale.pdf"),
                cancel: cancel()
            }
        )
        .is_err());
        assert!(!dir.path().join("stale.pdf").exists());
        std::fs::write(&new, &fixture).unwrap();
        assert!(prepare(1107).is_ok());
        assert!(matches!(
            run(
                1108,
                Job::Save {
                    token: 1107,
                    output: dir.path().join("cancelled.pdf"),
                    cancel: Arc::new(AtomicBool::new(true))
                }
            ),
            Err(EngineError::Cancelled)
        ));
        assert!(!dir.path().join("cancelled.pdf").exists());
        assert!(prepare(1109).is_ok());
        let output = dir.path().join("transferred.pdf");
        assert!(
            matches!(run(1110, Job::Save { token: 1109, output: output.clone(), cancel: cancel() }).unwrap(), Outcome::Saved(report) if report.output_path == Some(output.clone()) && report.transferred.len() == 1)
        );
        assert!(output.is_file());
        assert_eq!(std::fs::read(&old).unwrap(), old_bytes);
        assert_eq!(std::fs::read(&new).unwrap(), fixture);
        assert!(
            !worker.has_pending_mutations(),
            "file transfers do not create source undo steps"
        );
    }

    fn protected_plan_sets_use_one_operation_credentials_and_open_verified_bytes(worker: &Worker) {
        use crate::plan_sets::{Job, Outcome};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::plan_set::PlanSet;
        use zeroize::Zeroizing;

        fn next_plan(worker: &Worker, expected: u64) -> Result<Outcome, EngineError> {
            loop {
                match next_response(worker) {
                    Response::PlanSetProgress { generation, .. } => {
                        assert_eq!(generation, expected)
                    }
                    Response::PlanSetDone { generation, result } => {
                        assert_eq!(generation, expected);
                        return result;
                    }
                    _ => panic!("unexpected plan-set response"),
                }
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plans.json");
        let source = dir.path().join("protected.pdf");
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        let password = ["local", "plan", "fixture"].join("-");
        let encrypted = security::protect(
            &bytes,
            None,
            &Protection {
                user_password: password.clone(),
                owner_password: format!("{password}-owner"),
                allow_copy: false,
                aes256: true,
                ..Protection::default()
            },
        )
        .unwrap();
        std::fs::write(&source, &encrypted).unwrap();
        let original = PlanSet::new("Protected plan set").unwrap();
        worker.send(Request::PlanSet {
            generation: 701,
            job: Job::Add {
                set: original.clone(),
                path: path.clone(),
                sources: vec![source.clone()],
                password: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        });
        assert!(matches!(
            next_plan(worker, 701),
            Err(EngineError::PasswordRequired)
        ));
        worker.send(Request::PlanSet {
            generation: 702,
            job: Job::Add {
                set: original,
                path: path.clone(),
                sources: vec![source.clone()],
                password: Some(Zeroizing::new(password.clone())),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        });
        let Outcome::Changed { set, report } = next_plan(worker, 702).unwrap() else {
            panic!("expected added sheets")
        };
        assert!(report.complete());
        assert_eq!(set.sheets.len(), 2);
        assert!(
            !String::from_utf8(sunipdf_core::plan_set::encode(&set).unwrap())
                .unwrap()
                .contains(&password)
        );
        assert_eq!(std::fs::read(&source).unwrap(), encrypted);
        worker.send(Request::PlanSet {
            generation: 703,
            job: Job::Navigate {
                sheet_id: set.sheets[1].id.clone(),
                set,
                path,
                password: Some(Zeroizing::new(password)),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        });
        let Outcome::Navigate(navigation) = next_plan(worker, 703).unwrap() else {
            panic!("expected verified navigation")
        };
        assert_eq!(navigation.sheet.bytes, encrypted);
        assert_eq!(navigation.sheet.page, 2);
        assert!(navigation.password.is_some());
        // The selected PDF changes after verification: opening must use the captured bytes.
        std::fs::write(&source, b"changed after the plan-set check").unwrap();
        worker.send(Request::OpenPlanSheet(navigation));
        assert!(
            matches!(next_response(worker), Response::Opened(state) if state.info.page_count() == 2 && !state.modified)
        );
        assert_eq!(
            std::fs::read(source).unwrap(),
            b"changed after the plan-set check"
        );
    }

    fn reviewed_plan_links_preview_both_pages_and_publish_only_fresh_copies(worker: &Worker) {
        use crate::plan_links_ui::{Context, Job, Outcome};
        use lopdf::{dictionary, Object, Stream};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::plan_links::{PlanLinkReview, PlanLinkScanRequest, PlanLinkSelection};

        fn pdf(text: &str, width: i32, height: i32) -> Vec<u8> {
            let mut pdf = lopdf::Document::with_version("1.7");
            let pages = pdf.new_object_id();
            let font = pdf.add_object(
                dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
            );
            let contents = pdf.add_object(Stream::new(
                dictionary! {},
                format!("BT /F1 12 Tf 30 120 Td ({text}) Tj ET").into_bytes(),
            ));
            let page = pdf.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(),0.into(),width.into(),height.into()], "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}}, "Contents" => contents });
            pdf.objects.insert(pages, dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }.into());
            let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            pdf.trailer.set("Root", catalog);
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            bytes
        }
        fn run(worker: &Worker, generation: u64, job: Job) -> Result<Outcome, EngineError> {
            worker.send(Request::PlanLinks { generation, job });
            match next_response(worker) {
                Response::PlanLinksDone {
                    generation: actual,
                    result,
                } => {
                    assert_eq!(actual, generation);
                    result
                }
                _ => panic!("unexpected plan-link response"),
            }
        }
        let cancel = || Arc::new(AtomicBool::new(false));
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("index.pdf");
        let target = dir.path().join("floor.pdf");
        let output = dir.path().join("linked.pdf");
        let manifest = dir.path().join("set.json");
        let source_bytes = pdf("A-10 A-101 A-10", 600, 300);
        let target_bytes = pdf("Destination drawing", 300, 600);
        std::fs::write(&source, &source_bytes).unwrap();
        std::fs::write(&target, &target_bytes).unwrap();
        worker.send(Request::PlanSet {
            generation: 801,
            job: crate::plan_sets::Job::Add {
                set: sunipdf_core::plan_set::PlanSet::new("Reviewed links").unwrap(),
                path: manifest.clone(),
                sources: vec![source.clone(), target.clone()],
                password: None,
                cancel: cancel(),
            },
        });
        let mut set = loop {
            match next_response(worker) {
                Response::PlanSetProgress {
                    generation: 801, ..
                } => {}
                Response::PlanSetDone {
                    generation: 801,
                    result: Ok(crate::plan_sets::Outcome::Changed { set, report }),
                } => {
                    assert!(report.complete());
                    break set;
                }
                _ => panic!("expected indexed test PDFs"),
            }
        };
        let source_id = set
            .files
            .iter()
            .find(|f| f.path == "index.pdf")
            .unwrap()
            .id
            .clone();
        let target_id = set
            .files
            .iter()
            .find(|f| f.path == "floor.pdf")
            .unwrap()
            .id
            .clone();
        let sheet = set
            .sheets
            .iter_mut()
            .find(|s| s.file_id == target_id)
            .unwrap();
        sheet.number = "A-10".into();
        let target_sheet_id = sheet.id.clone();
        sunipdf_core::plan_set::save_new(&set, &manifest, &manifest).unwrap();
        let manifest_bytes = std::fs::read(&manifest).unwrap();
        let context = Context {
            path: manifest.clone(),
            set,
        };
        let scan = PlanLinkScanRequest {
            source_file_id: source_id,
            pages: vec![1],
            target_sheet_ids: vec![target_sheet_id],
        };
        let Outcome::Inspected(inspection) = run(
            worker,
            802,
            Job::Inspect {
                context: context.clone(),
                scan: scan.clone(),
                credentials: vec![],
                cancel: cancel(),
            },
        )
        .unwrap() else {
            panic!("expected inspection")
        };
        assert_eq!(inspection.occurrences.len(), 2);
        assert!(inspection.complete());
        assert!(!output.exists());
        let review = PlanLinkReview {
            version: 1,
            manifest_sha256: inspection.manifest_sha256.clone(),
            source_sha256: inspection.source_sha256.clone(),
            scan,
            selections: inspection
                .occurrences
                .iter()
                .map(|o| PlanLinkSelection {
                    occurrence_id: o.id.clone(),
                    fingerprint: o.fingerprint.clone(),
                    target_sheet_id: o.target_sheet_ids[0].clone(),
                    confirmed: true,
                })
                .collect(),
        };
        let prepare = || Job::Prepare {
            context: context.clone(),
            review: review.clone(),
            credentials: vec![],
            cancel: cancel(),
        };
        let Outcome::Prepared {
            token,
            report,
            preview,
        } = run(worker, 803, prepare()).unwrap()
        else {
            panic!("expected prepared preview")
        };
        assert_eq!(report.created.len(), 2);
        assert!(preview.image.width > preview.image.height);
        assert!(preview
            .image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] < 150));
        let Outcome::Preview {
            target: is_target,
            preview: destination,
            ..
        } = run(
            worker,
            804,
            Job::Target {
                token,
                index: 0,
                credentials: vec![],
                cancel: cancel(),
            },
        )
        .unwrap()
        else {
            panic!("expected actual destination image")
        };
        assert!(is_target);
        assert!(destination.image.height > destination.image.width);
        assert_ne!(preview.image.rgba, destination.image.rgba);
        assert!(!output.exists());
        let Outcome::Saved(receipt) = run(
            worker,
            805,
            Job::Save {
                token,
                output: output.clone(),
                cancel: cancel(),
            },
        )
        .unwrap() else {
            panic!("expected saved copy")
        };
        assert_eq!(receipt.report.output_sha256, report.output_sha256);
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&target).unwrap(), target_bytes);
        assert_eq!(std::fs::read(&manifest).unwrap(), manifest_bytes);
        assert!(
            run(
                worker,
                806,
                Job::Preview {
                    token,
                    page: 1,
                    credentials: vec![],
                    cancel: cancel()
                }
            )
            .is_err(),
            "saved candidates expire"
        );

        worker.send(Request::Open {
            path: output.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Links { page: 0 });
        let Response::Links { list, .. } = next_response(worker) else {
            panic!("expected reopened links")
        };
        assert_eq!(list.len(), 2);
        assert!(list
            .iter()
            .all(|l| matches!(l.target, LinkTarget::RemotePage(_))));
        let remote = report.created[0].target.clone();
        let Outcome::Navigate(navigation) = run(
            worker,
            807,
            Job::Navigate {
                context: context.clone(),
                source: output.clone(),
                target: remote.clone(),
                credentials: vec![],
                cancel: cancel(),
            },
        )
        .unwrap() else {
            panic!("expected verified navigation")
        };
        assert_eq!(navigation.sheet.bytes, target_bytes);
        assert_eq!(navigation.sheet.page, 1);
        assert!(navigation.active_source.is_some());
        worker.send(Request::OpenPlanSheet(navigation));
        assert!(
            matches!(next_response(worker), Response::Opened(doc) if doc.info.path == target.canonicalize().unwrap() && !doc.modified)
        );

        let Outcome::Prepared { token, .. } = run(worker, 808, prepare()).unwrap() else {
            panic!("expected prepared preview")
        };
        let saved = std::fs::read(&output).unwrap();
        assert!(run(
            worker,
            809,
            Job::Save {
                token,
                output: output.clone(),
                cancel: cancel()
            }
        )
        .is_err());
        assert_eq!(std::fs::read(&output).unwrap(), saved);
        let Outcome::Prepared { token, .. } = run(worker, 810, prepare()).unwrap() else {
            panic!("expected prepared preview")
        };
        std::fs::write(&target, pdf("Changed destination", 300, 600)).unwrap();
        let refused = dir.path().join("stale.pdf");
        assert!(run(
            worker,
            811,
            Job::Target {
                token,
                index: 0,
                credentials: vec![],
                cancel: cancel()
            }
        )
        .is_err());
        assert!(run(
            worker,
            812,
            Job::Save {
                token,
                output: refused.clone(),
                cancel: cancel()
            }
        )
        .is_err());
        assert!(!refused.exists());
        assert!(run(
            worker,
            813,
            Job::Navigate {
                context: context.clone(),
                source: output.clone(),
                target: remote,
                credentials: vec![],
                cancel: cancel()
            }
        )
        .is_err());
        std::fs::write(&target, &target_bytes).unwrap();
        let Outcome::Prepared { token, .. } = run(worker, 814, prepare()).unwrap() else {
            panic!("expected prepared preview")
        };
        assert!(matches!(
            run(
                worker,
                815,
                Job::Save {
                    token,
                    output: refused.clone(),
                    cancel: Arc::new(AtomicBool::new(true))
                }
            ),
            Err(EngineError::Cancelled)
        ));
        assert!(!refused.exists());
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&manifest).unwrap(), manifest_bytes);
        assert!(!worker.has_pending_mutations());
    }

    fn cooperative_search_and_register_publish_complete_generation_scoped_results(worker: &Worker) {
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Search {
            query: "sunipdf".into(),
            match_case: false,
            generation: 901,
        });
        let mut progress = Vec::new();
        loop {
            match worker
                .rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap()
            {
                Response::SearchProgress {
                    generation: 901,
                    done,
                    total,
                } => progress.push((done, total)),
                Response::SearchResults {
                    generation: 901,
                    matches,
                } => {
                    assert_eq!(
                        matches.iter().map(|item| item.page).collect::<Vec<_>>(),
                        vec![0, 1]
                    );
                    assert!(matches.iter().all(|item| !item.rects.is_empty()));
                    break;
                }
                _ => panic!("unexpected search response"),
            }
        }
        assert_eq!(progress, vec![(1, 2)], "one PDF page is processed per turn");
        worker.send(Request::AnnotationRegister {
            epoch: 55,
            generation: 902,
        });
        let mut progress = Vec::new();
        loop {
            match worker
                .rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap()
            {
                Response::AnnotationRegisterProgress {
                    epoch: 55,
                    generation: 902,
                    done,
                    total,
                } => progress.push((done, total)),
                Response::AnnotationRegister {
                    epoch: 55,
                    generation: 902,
                    result,
                } => {
                    let report = result.unwrap();
                    assert_eq!(report.page_count, 2);
                    assert_eq!(report.source_sha256.len(), 64);
                    break;
                }
                _ => panic!("unexpected register response"),
            }
        }
        assert_eq!(progress, vec![(0, 2), (1, 2)]);
        // Cancellation of an older generation must not remove a replacement job.
        worker.send(Request::Search {
            query: "sunipdf".into(),
            match_case: true,
            generation: 903,
        });
        worker.send(Request::CancelSearch { generation: 901 });
        match next_response(worker) {
            Response::SearchResults {
                generation: 903,
                matches,
            } => assert!(matches.is_empty()),
            _ => panic!("a stale cancellation removed the current search"),
        }
        assert!(
            !worker.has_pending_mutations(),
            "read-only jobs do not enter document history"
        );
    }

    fn revision_overlay_jobs_preserve_sources_and_guard_exports(worker: &Worker) {
        use crate::revision_overlay::{Job, Outcome};
        use std::sync::atomic::AtomicBool;
        use sunipdf_core::revision_overlay::OverlayOptions;
        let dir = tempfile::tempdir().unwrap();
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        let base = dir.path().join("base.pdf");
        let revision = dir.path().join("revision.pdf");
        std::fs::write(&base, &bytes).unwrap();
        std::fs::write(&revision, &bytes).unwrap();
        let prepare = |generation,
                       base: PathBuf,
                       passwords: [zeroize::Zeroizing<String>; 2],
                       cancel: bool| {
            worker.send(Request::RevisionOverlay {
                generation,
                job: Job::Preview {
                    base,
                    revision: revision.clone(),
                    options: OverlayOptions {
                        long_side: 512,
                        ..Default::default()
                    },
                    passwords,
                    cancel: Arc::new(AtomicBool::new(cancel)),
                },
            });
            match next_response(worker) {
                Response::RevisionOverlayDone {
                    generation: returned,
                    result,
                } => {
                    assert_eq!(returned, generation);
                    result
                }
                _ => panic!("overlay preview response"),
            }
        };
        let preview = match prepare(501, base.clone(), Default::default(), false).unwrap() {
            Outcome::Preview(preview) => preview,
            _ => panic!("prepared overlay"),
        };
        assert!(!worker.has_pending_mutations());
        crate::revision_overlay::tests::verify_prepared_controls(&preview);
        let export = |generation, path: PathBuf, json, cancel: bool| {
            worker.send(Request::RevisionOverlay {
                generation,
                job: Job::Export {
                    preview: preview.clone(),
                    path,
                    json,
                    cancel: Arc::new(AtomicBool::new(cancel)),
                },
            });
            match next_response(worker) {
                Response::RevisionOverlayDone {
                    generation: returned,
                    result,
                } => {
                    assert_eq!(returned, generation);
                    result
                }
                _ => panic!("overlay export response"),
            }
        };
        let png = dir.path().join("overlay.png");
        assert!(matches!(
            export(502, png.clone(), false, false),
            Ok(Outcome::Exported(_))
        ));
        let png_bytes = std::fs::read(&png).unwrap();
        assert_eq!(
            image::load_from_memory(&png_bytes).unwrap().width(),
            preview.image().width
        );
        assert!(export(503, png.clone(), false, false).is_err());
        assert_eq!(std::fs::read(&png).unwrap(), png_bytes);
        let json = dir.path().join("overlay.json");
        assert!(matches!(
            export(504, json.clone(), true, false),
            Ok(Outcome::Exported(_))
        ));
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(json).unwrap()).unwrap();
        assert_eq!(report["schema"], "sunipdf-revision-overlay");
        assert_eq!(
            report["base"]["source_sha256"],
            preview.report()["base"]["source_sha256"]
        );
        assert_eq!(std::fs::read(&base).unwrap(), bytes);
        assert_eq!(std::fs::read(&revision).unwrap(), bytes);
        let cancelled = dir.path().join("cancelled.png");
        assert!(matches!(
            export(505, cancelled.clone(), false, true),
            Err(EngineError::Cancelled)
        ));
        assert!(!cancelled.exists());
        assert!(matches!(
            prepare(506, base.clone(), Default::default(), true),
            Err(EngineError::Cancelled)
        ));
        let mut replacement = bytes.clone();
        replacement.extend_from_slice(b"\n% changed revision\n");
        std::fs::write(&revision, replacement).unwrap();
        let stale = dir.path().join("stale.json");
        assert!(export(507, stale.clone(), true, false).is_err());
        assert!(!stale.exists());
        std::fs::write(&revision, &bytes).unwrap();
        for aes256 in [false, true] {
            let protected = dir.path().join(format!("restricted-{aes256}.pdf"));
            let user = format!(
                "viewer-{}",
                dir.path().file_name().unwrap().to_string_lossy()
            );
            let restricted = security::protect(
                &bytes,
                None,
                &security::Protection {
                    user_password: user.clone(),
                    owner_password: format!("owner-{user}"),
                    allow_print: true,
                    allow_copy: false,
                    allow_modify: true,
                    allow_annotate: true,
                    aes256,
                },
            )
            .unwrap();
            std::fs::write(&protected, restricted).unwrap();
            assert!(matches!(
                prepare(508, protected.clone(), Default::default(), false),
                Err(EngineError::PasswordRequired)
            ));
            assert!(matches!(
                prepare(
                    509,
                    protected,
                    [zeroize::Zeroizing::new(user), Default::default()],
                    false
                ),
                Err(EngineError::SecurityRestricted)
            ));
        }
        assert_eq!(std::fs::read(base).unwrap(), bytes);
        assert_eq!(std::fs::read(revision).unwrap(), bytes);
        assert!(!worker.has_pending_mutations());
    }

    fn quantity_fields_have_history_exports_and_guarded_desktop_controls(worker: &Worker) {
        use sunipdf_core::quantity_table::{
            DecimalValue, QuantityFields, QuantityOptions, QuantityUnit, TypedValue, UnitPrice,
        };
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        worker.send(Request::Open {
            path: source,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        for (point, group) in [((50.0, 100.0), "First"), ((90.0, 100.0), "Second")] {
            worker.send(Request::Edit(EditOp::AddCountMarker {
                page: 0,
                point,
                size: 18.0,
                group: group.into(),
                author: None,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        let read = || {
            worker.send(Request::QuantityTable {
                epoch: 101,
                generation: 1,
            });
            match next_response(worker) {
                Response::QuantityTable {
                    result: Ok(table), ..
                } => table,
                _ => panic!("quantity report"),
            }
        };
        let original = read();
        assert_eq!(original.rows().len(), 2);
        assert!(original
            .rows()
            .iter()
            .all(|r| r.quantity.as_ref().unwrap().value == 1.0 && r.line_total.is_none()));
        crate::quantity_table::tests::verify_controls(&original);
        let fields = QuantityFields {
            unit_price: Some(UnitPrice {
                amount: DecimalValue::parse("3.5").unwrap(),
                currency: "CHF".into(),
                per_unit: QuantityUnit::Each,
                rounding_decimals: 2,
            }),
            custom: std::collections::BTreeMap::from([
                ("Package".into(), TypedValue::Text { value: "A".into() }),
                (
                    "Depth".into(),
                    TypedValue::Number {
                        value: DecimalValue::parse("0.25").unwrap(),
                        unit: Some(QuantityUnit::Metres),
                    },
                ),
            ]),
        };
        let target = original.target(&original.rows()[0]).unwrap();
        worker.send(Request::Edit(EditOp::SetQuantityFields {
            target: target.clone(),
            fields: fields.clone(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: true,
                measurement_scales_unchanged: true,
                ..
            })
        ));
        let edited = read();
        assert_eq!(edited.rows()[0].fields, fields);
        crate::quantity_table::tests::verify_loaded_unit_draft(&edited);
        assert_eq!(edited.rows()[0].line_total.as_ref().unwrap().amount, "3.50");
        assert_eq!(
            edited.rows()[0].register.annotation_id,
            original.rows()[0].register.annotation_id
        );
        let summary = edited.summary(&QuantityOptions::default()).unwrap();
        assert_eq!(summary.groups[0].missing_prices, 1);
        assert!(!summary.groups[0].costs[0].complete);
        worker.send(Request::Edit(EditOp::SetQuantityFields {
            target,
            fields: QuantityFields::default(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read().rows(), original.rows());
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read().rows(), edited.rows());
        let directory = tempfile::tempdir().unwrap();
        let csv = directory.path().join("quantities.csv");
        let export = |table, path: PathBuf| {
            worker.send(Request::ExportQuantityTable {
                table: Arc::new(table),
                options: QuantityOptions::default(),
                path,
                csv: true,
                epoch: 101,
                generation: 2,
            });
            next_response(worker)
        };
        assert!(matches!(
            export(original, csv.clone()),
            Response::QuantityExported { result: Err(_), .. }
        ));
        assert!(!csv.exists());
        assert!(matches!(
            export(read(), csv.clone()),
            Response::QuantityExported { result: Ok(()), .. }
        ));
        let saved_csv = std::fs::read(&csv).unwrap();
        assert!(String::from_utf8(saved_csv.clone())
            .unwrap()
            .contains("CHF"));
        assert!(matches!(
            export(read(), csv.clone()),
            Response::QuantityExported { result: Err(_), .. }
        ));
        assert_eq!(std::fs::read(csv).unwrap(), saved_csv);
        let pdf = directory.path().join("quantities.pdf");
        worker.send(Request::Save { path: pdf.clone() });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: pdf,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read().rows(), edited.rows());
        let saved_pdf = std::fs::read(directory.path().join("quantities.pdf")).unwrap();
        let protected = security::protect(
            &saved_pdf,
            None,
            &Protection {
                owner_password: format!("quantity-{}", directory.path().display()),
                allow_copy: false,
                allow_annotate: false,
                ..Default::default()
            },
        )
        .unwrap();
        let protected_path = directory.path().join("restricted-quantities.pdf");
        std::fs::write(&protected_path, &protected).unwrap();
        worker.send(Request::Open {
            path: protected_path,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let restricted = read();
        assert!(!restricted.can_export());
        assert!(restricted.rows().iter().all(|row| !row.editable));
        crate::quantity_table::tests::verify_restricted_controls(&restricted);
        let forbidden = directory.path().join("forbidden.csv");
        assert!(matches!(
            export(restricted, forbidden.clone()),
            Response::QuantityExported {
                result: Err(EngineError::SecurityRestricted),
                ..
            }
        ));
        assert!(!forbidden.exists());
    }

    fn library_groups_use_one_preview_and_one_history_step(worker: &Worker) {
        use sunipdf_core::annotation_library::{LibraryPlacement, LibraryPrimitive, LibraryStamp};
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        worker.send(Request::Open {
            path: source,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let details = || {
            worker.send(Request::Annotations {
                page: 0,
                detailed: true,
            });
            match next_response(worker) {
                Response::Annotations {
                    details: Some(details),
                    ..
                } => details,
                Response::Annotations {
                    list,
                    details: None,
                    ..
                } if list.is_empty() => vec![],
                _ => panic!("library group annotations"),
            }
        };
        let before = details();
        let mut entry = crate::annotation_library::tests::entry("group", "Inspection symbol");
        entry.primitives.push(LibraryPrimitive::Stamp {
            rect: [80.0, 0.0, 200.0, 40.0],
            stamp: LibraryStamp::Approved,
        });
        let preview = |generation| {
            worker.send(Request::PreviewRichAnnotation {
                page: 0,
                annotation: sunipdf_core::annot_rich::RichAnnotation::LibraryPlacement {
                    entry: entry.clone(),
                    placement: LibraryPlacement {
                        origin: (50.0, 100.0),
                        scale: 1.0,
                    },
                },
                author: Some("lmdanavanem".into()),
                epoch: 92,
                generation,
            });
            assert!(
                matches!(next_response(worker), Response::RichAnnotationPreview { epoch: 92, generation: g, result: Ok(_) } if g == generation)
            );
        };
        preview(1);
        assert_eq!(
            details(),
            before,
            "preview has no document or history effects"
        );
        worker.send(Request::DiscardRichAnnotationPreview {
            epoch: 92,
            generation: 1,
        });
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 1,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(details(), before);
        preview(2);
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 2,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        let after = details();
        assert_eq!(after.len(), before.len() + 2);
        let added = &after[before.len()..];
        assert_eq!(added[0].subtype, "Square");
        assert_eq!(added[1].subtype, "Stamp");
        assert!(added
            .iter()
            .all(|detail| detail.name.is_some() && detail.parent.is_none()));
        assert_ne!(added[0].name, added[1].name);
        worker.send(Request::LibraryCaptureCandidates {
            page: 0,
            epoch: 92,
            generation: 20,
        });
        let candidates = match next_response(worker) {
            Response::LibraryCaptureCandidates {
                epoch: 92,
                generation: 20,
                result: Ok(candidates),
            } => candidates,
            _ => panic!("library capture inventory"),
        };
        let square = candidates
            .iter()
            .find(|row| row.subtype == "Square")
            .unwrap();
        let selections = vec![sunipdf_core::annotation_library::LibraryCaptureSelection {
            index: square.index,
            fingerprint: square.fingerprint.clone().unwrap(),
        }];
        let capture = || {
            worker.send(Request::CaptureLibraryEntry {
                page: 0,
                selections: selections.clone(),
                id: "captured-square".into(),
                name: "Captured square".into(),
                provenance: sunipdf_core::annotation_library::LibraryProvenance {
                    source: Some("Test drawing".into()),
                    ..Default::default()
                },
                epoch: 92,
                generation: 21,
            });
            next_response(worker)
        };
        assert!(matches!(
            capture(),
            Response::LibraryEntryCaptured {
                epoch: 92,
                generation: 21,
                result: Ok(ref entry),
            } if entry.primitives.len() == 1
        ));
        assert_eq!(
            details(),
            after,
            "capture is read-only and preserves history"
        );
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(details(), before, "one Undo removes the complete group");
        assert!(matches!(
            capture(),
            Response::LibraryEntryCaptured { result: Err(_), .. }
        ));
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(details(), after);
        preview(3);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 3,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(
            details(),
            before,
            "history invalidates the entire prepared group"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("grouped.pdf");
        worker.send(Request::Save { path: path.clone() });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(details(), after);
    }

    fn dimensioned_rectangles_preview_commit_and_revalidate_the_captured_reference(
        worker: &Worker,
    ) {
        use crate::measurement_shapes_ui::{RectanglePlacement, RectangleScaleSelection};
        use sunipdf_core::measurement::{MeasurementKind, MeasurementUnit, PageCalibration};
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let original_bytes = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let calibration = PageCalibration {
            points: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        };
        let rectangle = RectanglePlacement {
            anchor: (50.0, 100.0),
            width: 2.0,
            height: 3.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
            scale: RectangleScaleSelection::Local(calibration.clone()),
        };
        let preview = |generation| {
            worker.send(Request::PreviewDimensionedRectangle {
                page: 0,
                rectangle: rectangle.clone(),
                author: Some("lmdanavanem".into()),
                epoch: 92,
                generation,
            });
            match next_response(worker) {
                Response::RichAnnotationPreview { result, .. } => result,
                _ => panic!("dimensioned rectangle preview response"),
            }
        };
        let read = || {
            worker.send(Request::MeasurementPage {
                page: 0,
                epoch: 92,
                generation: 1,
            });
            match next_response(worker) {
                Response::MeasurementPage {
                    result: Ok(data), ..
                } => data.records.unwrap(),
                _ => panic!("rectangle inventory"),
            }
        };
        assert!(preview(1).is_err(), "old UI calibration alone is not proof");
        assert!(read().is_empty());
        worker.send(Request::CaptureRectangleCalibration {
            page: 0,
            calibration: calibration.clone(),
        });
        assert!(preview(2).is_ok());
        assert!(read().is_empty(), "preview leaves the document unchanged");
        let mut invalid = calibration.clone();
        invalid.known_distance = 0.0;
        worker.send(Request::CaptureRectangleCalibration {
            page: 0,
            calibration: invalid.clone(),
        });
        assert!(
            matches!(preview(20), Err(EngineError::Other(message)) if message == RECTANGLE_REFERENCE_REQUIRED),
            "failed capture must replace the earlier valid token"
        );
        let mut rejected_rectangle = rectangle.clone();
        rejected_rectangle.scale = RectangleScaleSelection::Local(invalid);
        worker.send(Request::PreviewDimensionedRectangle {
            page: 0,
            rectangle: rejected_rectangle,
            author: None,
            epoch: 92,
            generation: 21,
        });
        assert!(
            matches!(next_response(worker), Response::RichAnnotationPreview {result: Err(EngineError::Other(message)), ..}
            if message.starts_with(RECTANGLE_CAPTURE_FAILED)),
            "capture failure retains its actual cause"
        );
        worker.send(Request::PreviewRichAnnotation {
            page: 0,
            annotation: sunipdf_core::annot_rich::RichAnnotation::Measurement(
                sunipdf_core::measurement::Measurement {
                    holes: Vec::new(),
                    depth: None,
                    kind: MeasurementKind::Length,
                    points: vec![(40.0, 100.0), (140.0, 100.0)],
                    calibration: calibration.points,
                    known_distance: calibration.known_distance,
                    unit: calibration.unit,
                    precision: 2,
                },
            ),
            author: None,
            epoch: 92,
            generation: 22,
        });
        assert!(
            matches!(
                next_response(worker),
                Response::RichAnnotationPreview { result: Ok(_), .. }
            ),
            "a rectangle capture failure does not disable ordinary measurements"
        );
        assert!(read().is_empty());
        worker.send(Request::CaptureRectangleCalibration {
            page: 0,
            calibration: calibration.clone(),
        });
        assert!(preview(2).is_ok());
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 2,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let records = read();
        assert_eq!(records.len(), 1);
        assert!(records[0].validity.is_current());
        assert_eq!(records[0].measurement.kind, MeasurementKind::Area);
        assert_eq!(
            records[0].measurement.points,
            vec![(50.0, 100.0), (70.0, 100.0), (70.0, 130.0), (50.0, 130.0)]
        );
        assert!((records[0].measurement.value().unwrap() - 6.0).abs() < 1e-8);
        assert!(
            preview(3).is_ok(),
            "annotation-only changes retain a verified local scale"
        );
        worker.send(Request::DiscardRichAnnotationPreview {
            epoch: 92,
            generation: 3,
        });
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 3,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(read().is_empty(), "one Undo removes the only rectangle");
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(), records);
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("dimensioned-rectangle.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read(), records);
        assert!(
            preview(4).is_err(),
            "opening a document clears temporary proof"
        );
        worker.send(Request::CaptureRectangleCalibration {
            page: 0,
            calibration: calibration.clone(),
        });
        assert!(preview(5).is_ok());
        worker.send(Request::Edit(EditOp::AddText {
            page: 0,
            x: 200.0,
            y: 200.0,
            text: "Changed drawing".into(),
            style: TextStyle::default(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 92,
            generation: 5,
        }));
        assert!(
            matches!(next_response(worker), Response::OperationFailed(_)),
            "a mutation discards the prepared candidate"
        );
        assert!(
            preview(6).is_err(),
            "page content changes invalidate the earlier reference"
        );
        assert_eq!(read().len(), 1);
        assert_eq!(std::fs::read(source).unwrap(), original_bytes);
    }

    fn saved_measurement_edits_replace_in_place_and_keep_history(worker: &Worker) {
        use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementUnit};
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let bytes = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let measurement = Measurement {
            holes: Vec::new(),
            depth: None,
            kind: MeasurementKind::Length,
            points: vec![(50.0, 100.0), (150.0, 100.0)],
            calibration: [(50.0, 60.0), (150.0, 60.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
        };
        worker.send(Request::PreviewRichAnnotation {
            page: 0,
            annotation: sunipdf_core::annot_rich::RichAnnotation::Measurement(measurement),
            author: Some("lmdanavanem".into()),
            epoch: 81,
            generation: 1,
        });
        assert!(matches!(
            next_response(worker),
            Response::RichAnnotationPreview { result: Ok(_), .. }
        ));
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 81,
            generation: 1,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let read = || {
            worker.send(Request::MeasurementPage {
                page: 0,
                epoch: 81,
                generation: 1,
            });
            match next_response(worker) {
                Response::MeasurementPage {
                    result: Ok(data), ..
                } => data.records.unwrap(),
                _ => panic!("saved measurement records"),
            }
        };
        let original = read();
        assert_eq!(original.len(), 1);
        let mut replacement = original[0].measurement.clone();
        replacement.known_distance = 2500.0;
        replacement.unit = MeasurementUnit::Centimetres;
        replacement.precision = 1;
        let edit_preview = |generation| {
            worker.send(Request::PreviewMeasurementEdit {
                page: 0,
                record: original[0].clone(),
                measurement: replacement.clone(),
                epoch: 81,
                generation,
            });
            match next_response(worker) {
                Response::RichAnnotationPreview {
                    result: Ok(preview),
                    ..
                } => {
                    assert_eq!(preview.index, original[0].index);
                }
                _ => panic!("measurement replacement preview"),
            }
        };
        edit_preview(2);
        assert_eq!(read(), original, "preview does not replace the measurement");
        worker.send(Request::DiscardRichAnnotationPreview {
            epoch: 81,
            generation: 2,
        });
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 81,
            generation: 2,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(read(), original, "cancellation keeps the saved measurement");
        edit_preview(3);
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 81,
            generation: 3,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: true,
                measurement_scales_unchanged: true,
                ..
            })
        ));
        let edited = read();
        assert_eq!(
            edited.len(),
            1,
            "editing must never append a second measurement"
        );
        assert_eq!(edited[0].index, original[0].index);
        assert_eq!(edited[0].annotation_id, original[0].annotation_id);
        assert_eq!(edited[0].measurement, replacement);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(), original, "one Undo restores the exact measurement");
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(), edited);
        worker.send(Request::PreviewMeasurementEdit {
            page: 0,
            record: original[0].clone(),
            measurement: replacement.clone(),
            epoch: 81,
            generation: 4,
        });
        assert!(
            matches!(
                next_response(worker),
                Response::RichAnnotationPreview { result: Err(_), .. }
            ),
            "an outdated inspector record is refused"
        );
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_redo: true, .. })
        ));
        assert_eq!(read(), original, "failed previews must not create history");
        edit_preview(5);
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 81,
            generation: 5,
        }));
        assert!(
            matches!(next_response(worker), Response::OperationFailed(_)),
            "history changes invalidate prepared edits"
        );
        assert_eq!(read(), edited);
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("edited-measurement.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read(), edited);
        worker.send(Request::Edit(EditOp::AddText {
            page: 0,
            text: "Changed plan revision".into(),
            x: 30.0,
            y: 190.0,
            style: TextStyle::default(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let historical = read();
        assert_eq!(
            historical[0].validity,
            sunipdf_core::measurement::MeasurementValidity::Stale
        );
        worker.send(Request::PreviewMeasurementEdit {
            page: 0,
            record: historical[0].clone(),
            measurement: historical[0].measurement.clone(),
            epoch: 81,
            generation: 6,
        });
        assert!(matches!(
            next_response(worker),
            Response::RichAnnotationPreview { result: Err(_), .. }
        ));
        let calibration = sunipdf_core::measurement::PageCalibration {
            points: [(30.0, 40.0), (130.0, 40.0)],
            known_distance: 5000.0,
            unit: MeasurementUnit::Centimetres,
        };
        worker.send(Request::PreviewMeasurementRecalibration {
            page: 0,
            record: historical[0].clone(),
            measurement: historical[0].measurement.clone(),
            calibration: calibration.clone(),
            epoch: 81,
            generation: 7,
        });
        assert!(matches!(
            next_response(worker),
            Response::RichAnnotationPreview { result: Ok(_), .. }
        ));
        assert_eq!(read(), historical, "preview retains the historical state");
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 81,
            generation: 7,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let recalibrated = read();
        assert_eq!(recalibrated.len(), 1);
        assert!(recalibrated[0].validity.is_current());
        assert_eq!(recalibrated[0].annotation_id, historical[0].annotation_id);
        assert_eq!(
            recalibrated[0].measurement.points,
            historical[0].measurement.points
        );
        assert_eq!(recalibrated[0].measurement.calibration, calibration.points);
        assert_eq!(recalibrated[0].measurement.value().unwrap(), 5000.0);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(
            read(),
            historical,
            "one Undo restores the unverified provenance"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(), recalibrated);
        let output = temp.path().join("recalibrated-measurement.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read(), recalibrated);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }

    fn deducted_measurements_round_trip_through_guarded_worker_history(worker: &Worker) {
        use sunipdf_core::measurement::{
            Measurement, MeasurementDepth, MeasurementKind, MeasurementUnit,
        };
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let source_bytes = std::fs::read(&source).unwrap();
        let temp = tempfile::tempdir().unwrap();
        for kind in [MeasurementKind::Area, MeasurementKind::Volume] {
            worker.send(Request::Open {
                path: source.clone(),
                password: None,
            });
            assert!(matches!(next_response(worker), Response::Opened(_)));
            let measurement = Measurement {
                kind,
                points: vec![(40.0, 40.0), (240.0, 40.0), (240.0, 240.0), (40.0, 240.0)],
                holes: vec![vec![
                    (90.0, 90.0),
                    (140.0, 90.0),
                    (140.0, 140.0),
                    (90.0, 140.0),
                ]],
                depth: (kind == MeasurementKind::Volume).then_some(MeasurementDepth {
                    value: 20.0,
                    unit: MeasurementUnit::Centimetres,
                }),
                calibration: [(40.0, 20.0), (140.0, 20.0)],
                known_distance: 10.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            };
            worker.send(Request::PreviewRichAnnotation {
                page: 0,
                annotation: sunipdf_core::annot_rich::RichAnnotation::Measurement(
                    measurement.clone(),
                ),
                author: None,
                epoch: 171,
                generation: 1,
            });
            assert!(matches!(
                next_response(worker),
                Response::RichAnnotationPreview { result: Ok(_), .. }
            ));
            worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
                epoch: 171,
                generation: 1,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(DocState { can_undo: true, .. })
            ));
            let read = || {
                worker.send(Request::MeasurementPage {
                    page: 0,
                    epoch: 171,
                    generation: 1,
                });
                match next_response(worker) {
                    Response::MeasurementPage {
                        result: Ok(data), ..
                    } => data.records.unwrap(),
                    _ => panic!("deducted measurement records"),
                }
            };
            let original = read();
            assert_eq!(original.len(), 1);
            assert!(original[0].validity.is_current());
            assert_eq!(original[0].measurement, measurement);
            let mut replacement = original[0].measurement.clone();
            replacement.holes[0][1].0 = 150.0;
            worker.send(Request::PreviewMeasurementEdit {
                page: 0,
                record: original[0].clone(),
                measurement: replacement.clone(),
                epoch: 171,
                generation: 2,
            });
            assert!(matches!(
                next_response(worker),
                Response::RichAnnotationPreview { result: Ok(_), .. }
            ));
            assert_eq!(
                read(),
                original,
                "a hole-only preview leaves the original unchanged"
            );
            worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
                epoch: 171,
                generation: 2,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            let edited = read();
            assert_eq!(edited.len(), 1);
            assert_eq!(edited[0].annotation_id, original[0].annotation_id);
            assert_eq!(edited[0].measurement, replacement);
            worker.send(Request::Undo);
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            assert_eq!(
                read(),
                original,
                "one Undo restores every ring and the physical depth"
            );
            worker.send(Request::Redo);
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            assert_eq!(read(), edited);
            worker.send(Request::PreviewMeasurementEdit {
                page: 0,
                record: original[0].clone(),
                measurement: measurement.clone(),
                epoch: 171,
                generation: 3,
            });
            assert!(
                matches!(
                    next_response(worker),
                    Response::RichAnnotationPreview { result: Err(_), .. }
                ),
                "old hole geometry cannot target the new record"
            );
            assert_eq!(read(), edited);
            let output = temp.path().join(format!("deducted-{}.pdf", kind.id()));
            worker.send(Request::Save {
                path: output.clone(),
            });
            assert!(matches!(next_response(worker), Response::Saved { .. }));
            worker.send(Request::Open {
                path: output,
                password: None,
            });
            assert!(matches!(next_response(worker), Response::Opened(_)));
            assert_eq!(
                read(),
                edited,
                "all rings, depth and provenance survive saving and reopening"
            );
        }
        assert_eq!(std::fs::read(source).unwrap(), source_bytes);
    }

    fn page_calibration_edits_have_history_and_survive_reopening(worker: &Worker) {
        use sunipdf_core::measurement::{MeasurementUnit, PageCalibration};
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let original = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let read = |page| {
            worker.send(Request::MeasurementPage {
                page,
                epoch: 73,
                generation: 8,
            });
            match next_response(worker) {
                Response::MeasurementPage {
                    page: actual,
                    epoch: 73,
                    generation: 8,
                    result: Ok(data),
                } if page == actual => {
                    assert!(data.can_modify);
                    assert!(data.records.unwrap().is_empty());
                    data.calibration.unwrap()
                }
                _ => panic!("page calibration response"),
            }
        };
        let calibration = PageCalibration {
            points: [(50.0, 60.0), (150.0, 60.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
        };
        assert!(read(0).is_none());
        worker.send(Request::Edit(EditOp::SetPageCalibration {
            page: 0,
            calibration: Some(calibration.clone()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: true,
                modified: true,
                pages_unchanged: true,
                measurement_scales_unchanged: true,
                ..
            })
        ));
        assert_eq!(read(0), Some(calibration.clone()));
        assert!(read(1).is_none(), "the other page has no inherited scale");
        let mut invalid = calibration.clone();
        invalid.known_distance = 0.0;
        worker.send(Request::Edit(EditOp::SetPageCalibration {
            page: 0,
            calibration: Some(invalid),
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(read(0), Some(calibration.clone()));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert!(
            read(0).is_none(),
            "a failed setter must not add an undo step"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(0), Some(calibration.clone()));
        worker.send(Request::Edit(EditOp::SetPageCalibration {
            page: 0,
            calibration: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(read(0).is_none());
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read(0), Some(calibration.clone()));
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("page-calibration.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(read(0), Some(calibration.clone()));
        assert!(read(1).is_none());
        worker.send(Request::Edit(EditOp::AddText {
            page: 0,
            x: 50.0,
            y: 200.0,
            text: "Changed plan".into(),
            style: TextStyle::default(),
        }));
        for request in [None, Some(Request::Undo), Some(Request::Redo)] {
            if let Some(request) = request {
                worker.send(request);
            }
            assert!(
                matches!(
                    next_response(worker),
                    Response::DocumentChanged(DocState {
                        pages_unchanged: true,
                        measurement_scales_unchanged: false,
                        ..
                    })
                ),
                "content history must revoke session scales without changing render reuse"
            );
        }
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    fn count_register_edits_have_one_undo_and_stale_edits_add_no_history(worker: &Worker) {
        use sunipdf_core::annotation_register::{RegisterFields, RegisterFilter, RegisterSort};
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let read = || {
            worker.send(Request::AnnotationRegister {
                epoch: 71,
                generation: 4,
            });
            match next_response(worker) {
                Response::AnnotationRegister {
                    epoch: 71,
                    generation: 4,
                    result: Ok(report),
                } => report,
                _ => panic!("register response"),
            }
        };
        assert!(read().rows.is_empty());
        worker.send(Request::Edit(EditOp::AddCountMarker {
            page: 0,
            point: (80.0, 90.0),
            size: 18.0,
            group: "Lights".into(),
            author: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        let report = read();
        assert_eq!(
            report.json(&RegisterFilter::default(), RegisterSort::Page, false)["counts"][0]
                ["quantity"],
            1
        );
        let id = report.rows[0].annotation_id.clone();
        let target = report.target(&report.rows[0]);
        worker.send(Request::Edit(EditOp::SetAnnotationRegisterFields {
            target: target.clone(),
            fields: RegisterFields {
                business_status: Some("Installed".into()),
                ..Default::default()
            },
            count_group: Some("Sockets".into()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read().rows[0].annotation_id, id);
        assert_eq!(read().rows[0].count_group.as_deref(), Some("Sockets"));
        worker.send(Request::Edit(EditOp::SetAnnotationRegisterFields {
            target,
            fields: RegisterFields::default(),
            count_group: Some("Wrong stale edit".into()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(
            read().rows[0].count_group.as_deref(),
            Some("Lights"),
            "failed stale edit did not create history"
        );
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                ..
            })
        ));
        assert!(read().rows.is_empty());
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read().rows[0].annotation_id, id);
        worker.send(Request::Edit(EditOp::DeleteAnnotationThread {
            page: 0,
            index: 0,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(read().rows.is_empty());
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(read().rows[0].annotation_id, id);
    }

    fn visual_search_reviewed_counts_have_one_undo_and_reject_stale_tokens(worker: &Worker) {
        use crate::visual_search_ui::{Job, Outcome};
        use sunipdf_core::visual_search::{
            VisualSearchOptions, VisualSearchReport, VisualSearchStatus,
        };
        fn start(worker: &Worker, generation: u64) {
            worker.send(Request::VisualSearch {
                epoch: 1700,
                generation,
                job: Job::Start {
                    page: 0,
                    rect: PointRect {
                        left: 18.,
                        bottom: 18.,
                        right: 34.,
                        top: 34.,
                    },
                    options: VisualSearchOptions {
                        pages: vec![0],
                        scale: 1.,
                        sensitivity: 0.99,
                        max_comparisons: 10_000_000,
                        ..Default::default()
                    },
                },
            });
            assert!(matches!(
                next_response(worker),
                Response::VisualSearch {
                    result: Ok(Outcome::Started { .. }),
                    ..
                }
            ));
        }
        fn finish(worker: &Worker) -> VisualSearchReport {
            match next_response(worker) {
                Response::VisualSearch {
                    result: Ok(Outcome::Finished(report)),
                    ..
                } => report,
                Response::VisualSearch {
                    result: Err(error), ..
                } => panic!("visual search failed: {error}"),
                _ => panic!("completed symbol search"),
            }
        }
        fn count(worker: &Worker) -> usize {
            worker.send(Request::AnnotationRegister {
                epoch: 1700,
                generation: 51,
            });
            match next_response(worker) {
                Response::AnnotationRegister {
                    result: Ok(report), ..
                } => report.rows.len(),
                _ => panic!("complete register"),
            }
        }
        fn preview(worker: &Worker, generation: u64, id: u32) -> RenderedPage {
            worker.send(Request::VisualSearch {
                epoch: 1700,
                generation,
                job: Job::Preview { id },
            });
            match next_response(worker) {
                Response::VisualSearch {
                    result: Ok(Outcome::Preview { image, .. }),
                    ..
                } => image,
                _ => panic!("candidate review preview"),
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("symbols.pdf");
        let output = dir.path().join("counted.pdf");
        let mut pdf = lopdf::Document::with_version("1.7");
        let pages = pdf.new_object_id();
        let stream = pdf.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"0 g 20 20 3 10 re f 20 20 10 3 re f 70 70 3 10 re f 70 70 10 3 re f".to_vec(),
        ));
        let page = pdf.add_object(lopdf::dictionary! { "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), 120.into(), 120.into()], "Contents" => stream,
        "Resources" => lopdf::Dictionary::new() });
        pdf.objects.insert(
            pages,
            lopdf::Object::Dictionary(lopdf::dictionary! {
            "Type" => "Pages", "Kids" => vec![lopdf::Object::Reference(page)], "Count" => 1 }),
        );
        let catalog = pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
        pdf.trailer.set("Root", catalog);
        pdf.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        start(worker, 1);
        // Foreground rendering must be served before the scan can complete.
        worker.send(Request::Thumbnail {
            page: 0,
            width_px: 120,
        });
        assert!(matches!(next_response(worker), Response::Thumbnail { .. }));
        let report = finish(worker);
        assert_eq!(report.progress.status, VisualSearchStatus::Complete);
        assert_eq!(report.candidates.len(), 2);
        assert_eq!(count(worker), 0, "suggestions do not create counts");
        let first_bounds = report.candidates[0].rect;
        let uncounted_preview = preview(worker, 1, report.candidates[0].id);
        let selected: Vec<_> = report.candidates.iter().map(|c| c.id).collect();
        worker.send(Request::Edit(EditOp::CommitVisualSearchCounts {
            epoch: 1700,
            generation: 1,
            ids: selected.clone(),
            group: "Equipment".into(),
            size: 8.,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        assert_eq!(count(worker), 2);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(
            count(worker),
            0,
            "one Undo removes the whole accepted batch"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(count(worker), 2);
        worker.send(Request::Edit(EditOp::CommitVisualSearchCounts {
            epoch: 1700,
            generation: 1,
            ids: selected,
            group: "Equipment".into(),
            size: 8.,
        }));
        assert!(
            matches!(next_response(worker), Response::OperationFailed(_)),
            "consumed token cannot add duplicates"
        );
        assert_eq!(count(worker), 2);
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(count(worker), 2);
        start(worker, 2);
        worker.send(Request::VisualSearch {
            epoch: 1700,
            generation: 1,
            job: Job::Discard,
        });
        assert!(matches!(
            next_response(worker),
            Response::VisualSearch {
                result: Ok(Outcome::Discarded),
                ..
            }
        ));
        let report = finish(worker);
        assert_eq!(
            report.candidates.len(),
            2,
            "existing count appearances are excluded from search"
        );
        assert_eq!(report.candidates[0].rect, first_bounds);
        let counted_preview = preview(worker, 2, report.candidates[0].id);
        assert_eq!(counted_preview.width, uncounted_preview.width);
        assert_eq!(counted_preview.height, uncounted_preview.height);
        assert_ne!(
            counted_preview.rgba, uncounted_preview.rgba,
            "review previews show existing counts even though detection excludes them"
        );
        worker.send(Request::Edit(EditOp::Note {
            page: 0,
            x: 100.,
            y: 100.,
            text: "later edit".into(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitVisualSearchCounts {
            epoch: 1700,
            generation: 2,
            ids: report.candidates.iter().map(|c| c.id).collect(),
            group: "Equipment".into(),
            size: 8.,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                ..
            })
        ));
        assert_eq!(count(worker), 2, "rejected stale apply adds no history");
        start(worker, 3);
        worker.send(Request::VisualSearch {
            epoch: 1700,
            generation: 3,
            job: Job::Cancel,
        });
        assert_eq!(
            finish(worker).progress.status,
            VisualSearchStatus::Cancelled
        );
        assert_eq!(count(worker), 2);
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    fn boundary_fill_preserves_deductions_history_and_rejects_incomplete_or_stale_results(
        worker: &Worker,
    ) {
        use crate::boundary_fill_ui::{Job, Outcome};
        use crate::measurement_shapes_ui::RectangleScaleSelection;
        use sunipdf_core::boundary_fill::{
            BoundaryFillOptions, BoundaryFillReport, BoundaryFillStatus,
        };
        use sunipdf_core::measurement::{MeasurementRecord, MeasurementUnit, PageCalibration};
        fn start(worker: &Worker, generation: u64, seed: [f32; 2]) {
            worker.send(Request::BoundaryFill {
                epoch: 1800,
                generation,
                job: Job::Start {
                    page: 0,
                    seed,
                    options: BoundaryFillOptions::default(),
                },
            });
        }
        fn started(worker: &Worker) {
            assert!(matches!(
                next_response(worker),
                Response::BoundaryFill {
                    result: Ok(Outcome::Started(_)),
                    ..
                }
            ));
        }
        fn finish(worker: &Worker) -> BoundaryFillReport {
            match next_response(worker) {
                Response::BoundaryFill {
                    result: Ok(Outcome::Finished(report)),
                    ..
                } => report,
                Response::BoundaryFill {
                    result: Err(error), ..
                } => panic!("region fill failed: {error}"),
                _ => panic!("completed region response"),
            }
        }
        fn records(worker: &Worker) -> Vec<MeasurementRecord> {
            worker.send(Request::MeasurementPage {
                page: 0,
                epoch: 1800,
                generation: 50,
            });
            match next_response(worker) {
                Response::MeasurementPage {
                    result: Ok(data), ..
                } => data.records.unwrap().to_vec(),
                _ => panic!("measurement records"),
            }
        }
        fn prepare(worker: &Worker, generation: u64) {
            worker.send(Request::BoundaryFill {
                epoch: 1800,
                generation,
                job: Job::Prepare {
                    scale: RectangleScaleSelection::Automatic,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                },
            });
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("enclosed-room.pdf");
        let calibrated = dir.path().join("calibrated-room.pdf");
        let counted = dir.path().join("measured-room.pdf");
        let mut pdf = lopdf::Document::with_version("1.7");
        let pages = pdf.new_object_id();
        let stream = pdf.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"0 G 0 g 2 w 20 20 80 80 re S 45 45 15 15 re f".to_vec(),
        ));
        let page = pdf.add_object(lopdf::dictionary! { "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 120.into(), 120.into()], "Contents" => stream, "Resources" => lopdf::Dictionary::new() });
        pdf.objects.insert(pages, lopdf::Object::Dictionary(lopdf::dictionary! { "Type" => "Pages", "Kids" => vec![lopdf::Object::Reference(page)], "Count" => 1 }));
        let catalog = pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
        pdf.trailer.set("Root", catalog);
        pdf.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Edit(EditOp::SetPageCalibration {
            page: 0,
            calibration: Some(PageCalibration {
                points: [(20., 20.), (100., 20.)],
                known_distance: 8.,
                unit: MeasurementUnit::Metres,
            }),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Save {
            path: calibrated.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: calibrated,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        start(worker, 1, [30., 30.]);
        worker.send(Request::Thumbnail {
            page: 0,
            width_px: 120,
        });
        started(worker);
        assert!(
            matches!(next_response(worker), Response::Thumbnail { .. }),
            "foreground rendering precedes background fill"
        );
        let report = finish(worker);
        assert_eq!(report.progress.status, BoundaryFillStatus::Complete);
        assert_eq!(report.region.unwrap().holes.len(), 1);
        assert!(
            records(worker).is_empty(),
            "finding a region does not create a measurement"
        );
        prepare(worker, 1);
        let expected = match next_response(worker) {
            Response::BoundaryFill {
                result: Ok(Outcome::Prepared { measurement, image }),
                ..
            } => {
                assert!(image.width > 0 && image.height > 0);
                assert_eq!(measurement.holes.len(), 1);
                measurement
            }
            Response::BoundaryFill {
                result: Err(error), ..
            } => panic!("region preview failed: {error}"),
            _ => panic!("prepared area response"),
        };
        assert!(
            records(worker).is_empty(),
            "preview has no document side effects"
        );
        worker.send(Request::Edit(EditOp::CommitBoundaryFill {
            epoch: 1800,
            generation: 1,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        assert_eq!(records(worker)[0].measurement, expected);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert!(
            records(worker).is_empty(),
            "one Undo removes the area and all deductions"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(records(worker)[0].measurement, expected);
        worker.send(Request::Edit(EditOp::CommitBoundaryFill {
            epoch: 1800,
            generation: 1,
        }));
        assert!(
            matches!(next_response(worker), Response::OperationFailed(_)),
            "a consumed preview cannot apply twice"
        );
        worker.send(Request::Save {
            path: counted.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: counted,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(
            records(worker)[0].measurement,
            expected,
            "deductions and calibration survive reopening"
        );
        start(worker, 2, [30., 30.]);
        worker.send(Request::BoundaryFill {
            epoch: 1800,
            generation: 1,
            job: Job::Discard,
        });
        started(worker);
        assert!(matches!(
            next_response(worker),
            Response::BoundaryFill {
                result: Ok(Outcome::Discarded),
                ..
            }
        ));
        assert_eq!(finish(worker).progress.status, BoundaryFillStatus::Complete);
        prepare(worker, 2);
        assert!(matches!(
            next_response(worker),
            Response::BoundaryFill {
                result: Ok(Outcome::Prepared { .. }),
                ..
            }
        ));
        worker.send(Request::Edit(EditOp::Note {
            page: 0,
            x: 110.,
            y: 110.,
            text: "later edit".into(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitBoundaryFill {
            epoch: 1800,
            generation: 2,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                ..
            })
        ));
        assert_eq!(
            records(worker).len(),
            1,
            "stale commit adds no history or area"
        );
        start(worker, 3, [30., 30.]);
        worker.send(Request::BoundaryFill {
            epoch: 1800,
            generation: 3,
            job: Job::Cancel,
        });
        started(worker);
        assert_eq!(
            finish(worker).progress.status,
            BoundaryFillStatus::Cancelled
        );
        prepare(worker, 3);
        assert!(matches!(
            next_response(worker),
            Response::BoundaryFill { result: Err(_), .. }
        ));
        start(worker, 4, [5., 5.]);
        started(worker);
        assert_eq!(finish(worker).progress.status, BoundaryFillStatus::Leaked);
        prepare(worker, 4);
        assert!(matches!(
            next_response(worker),
            Response::BoundaryFill { result: Err(_), .. }
        ));
        assert_eq!(
            records(worker).len(),
            1,
            "cancelled and leaking results cannot add measurements"
        );
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    fn next_response(worker: &Worker) -> Response {
        loop {
            let response = worker
                .rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("worker response");
            if matches!(response, Response::MutationFinished) {
                worker.finish_mutation();
            } else if matches!(
                response,
                Response::SearchProgress { .. }
                    | Response::BoundaryFill {
                        result: Ok(crate::boundary_fill_ui::Outcome::Progress(_)),
                        ..
                    }
                    | Response::VisualSearch {
                        result: Ok(crate::visual_search_ui::Outcome::Progress(_)),
                        ..
                    }
                    | Response::AnnotationRegisterProgress { .. }
                    | Response::IssueReportProgress { .. }
                    | Response::QuantityWorkbookProgress { .. }
            ) {
                continue;
            } else {
                return response;
            }
        }
    }

    fn inspection_uses_saved_bytes_and_refuses_unsaved_changes(worker: &Worker) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inspect.pdf");
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&path, &original).unwrap();
        worker.send(Request::Open {
            path: path.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Inspect { epoch: 197 });
        match next_response(worker) {
            Response::Inspection {
                epoch: 197,
                result: Ok(report),
            } => {
                assert!(report.complete);
                assert_eq!(report.source_bytes, original.len());
            }
            _ => panic!("saved inspection response"),
        }
        worker.send(Request::Edit(EditOp::Bytes(BytesOp::SetMetadata(
            Metadata {
                title: Some("unsaved inspection draft".into()),
                ..Default::default()
            },
        ))));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Inspect { epoch: 198 });
        assert!(
            matches!(next_response(worker), Response::Inspection { epoch: 198, result: Err(EngineError::Other(message)) } if message == CERTIFICATE_SAVE_FIRST)
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    fn signatures_bind_to_loaded_saved_and_resumed_bytes(worker: &Worker) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loaded.pdf");
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        std::fs::write(&path, &original).unwrap();
        worker.send(Request::Open {
            path: path.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let mut replaced = original.clone();
        replaced.extend_from_slice(b"\n% external revision\n");
        std::fs::write(&path, &replaced).unwrap();
        worker.send(Request::VerifySignatures);
        assert!(
            matches!(next_response(worker), Response::Signatures(Err(EngineError::Other(message)))
            if message == CERTIFICATE_SOURCE_CHANGED)
        );
        let output = dir.path().join("signed.pdf");
        let sign = || Request::SignCertificate {
            pfx: dir.path().join("identity-must-not-be-read.pfx"),
            password: zeroize::Zeroizing::new(String::new()),
            options: Default::default(),
            output: output.clone(),
        };
        worker.send(sign());
        assert!(
            matches!(next_response(worker), Response::SignedCopy(Err(EngineError::Other(message)))
            if message == CERTIFICATE_SOURCE_CHANGED)
        );
        assert!(!output.exists());
        std::fs::write(&path, &original).unwrap();
        worker.send(Request::Suspend);
        let suspended = match next_response(worker) {
            Response::Suspended(suspended) => suspended,
            _ => panic!("suspend loaded signature source"),
        };
        assert_eq!(
            suspended.signature_source.as_ref().unwrap().bytes.as_ref(),
            original
        );
        worker.send(Request::ResumeDoc(suspended));
        assert!(matches!(next_response(worker), Response::Resumed(_)));
        worker.send(Request::VerifySignatures);
        assert!(matches!(next_response(worker), Response::Signatures(Ok(list)) if list.is_empty()));
        worker.send(Request::Edit(EditOp::AddText {
            page: 0,
            text: "Saved signing revision".into(),
            x: 30.0,
            y: 60.0,
            style: TextStyle::default(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::VerifySignatures);
        assert!(
            matches!(next_response(worker), Response::Signatures(Err(EngineError::Other(message)))
            if message == CERTIFICATE_SAVE_FIRST)
        );
        worker.send(sign());
        assert!(
            matches!(next_response(worker), Response::SignedCopy(Err(EngineError::Other(message)))
            if message == CERTIFICATE_SAVE_FIRST)
        );
        worker.send(Request::Save { path: path.clone() });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::VerifySignatures);
        assert!(
            matches!(next_response(worker), Response::Signatures(Ok(list)) if list.is_empty()),
            "saving must capture the exact written bytes, not a separately rewritten snapshot"
        );
        worker.send(Request::Suspend);
        let suspended = match next_response(worker) {
            Response::Suspended(suspended) => suspended,
            _ => panic!("suspend saved signature source"),
        };
        assert_eq!(
            suspended.signature_source.as_ref().unwrap().bytes.as_ref(),
            std::fs::read(&path).unwrap()
        );
        worker.send(Request::ResumeDoc(suspended));
        assert!(matches!(next_response(worker), Response::Resumed(_)));
    }

    fn failed_resume_returns_unsaved_bytes_and_journal_for_a_retry(worker: &Worker) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("resume-source.pdf");
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        let protected = security::protect(
            &original,
            None,
            &Protection {
                user_password: "reader".into(),
                owner_password: "owner".into(),
                ..Default::default()
            },
        )
        .unwrap();
        std::fs::write(&path, &protected).unwrap();
        worker.send(Request::Open {
            path: path.clone(),
            password: Some("owner".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        for (text, y) in [("Recovered tab", 60.0), ("Redo retained", 90.0)] {
            worker.send(Request::Edit(EditOp::AddText {
                page: 0,
                text: text.into(),
                x: 30.0,
                y,
                style: TextStyle::default(),
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
        }
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Suspend);
        let mut suspended = match next_response(worker) {
            Response::Suspended(suspended) => suspended,
            _ => panic!("suspended unsaved document"),
        };
        let bytes = suspended.bytes.clone();
        let undo = suspended.undo[0].bytes.clone();
        let redo = suspended.redo[0].bytes.clone();
        assert!(suspended.modified);
        assert_eq!(suspended.save_point, Some(0));
        suspended.password = Some("wrong credential".into());
        worker.send(Request::ResumeDoc(suspended));
        let mut retained = match next_response(worker) {
            Response::ResumeFailed { error, suspended } => {
                assert!(matches!(error, EngineError::PasswordRequired));
                suspended
            }
            _ => panic!("failed resume must return the complete suspended document"),
        };
        assert_eq!(retained.bytes, bytes);
        assert_eq!(retained.path, path);
        assert!(retained.modified);
        assert_eq!(retained.password.as_deref(), Some("wrong credential"));
        assert_eq!(retained.save_point, Some(0));
        assert_eq!(retained.undo.len(), 1);
        assert_eq!(retained.redo.len(), 1);
        assert_eq!(retained.undo[0].bytes, undo);
        assert_eq!(retained.redo[0].bytes, redo);
        assert_eq!(
            retained.signature_source.as_ref().unwrap().bytes.as_ref(),
            protected
        );
        retained.password = Some("owner".into());
        worker.send(Request::ResumeDoc(retained));
        assert!(matches!(
            next_response(worker),
            Response::Resumed(DocState {
                can_undo: true,
                can_redo: true,
                modified: true,
                ..
            })
        ));
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let output = dir.path().join("resumed-copy.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: Some("owner".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::Search {
            query: "Recovered tab".into(),
            match_case: true,
            generation: 991,
        });
        assert!(
            matches!(next_response(worker), Response::SearchResults { generation: 991, matches } if matches.len() == 1)
        );
        assert_eq!(std::fs::read(&path).unwrap(), protected);
    }

    fn reprotection_requires_the_current_owner_without_removing_existing_restrictions(
        worker: &Worker,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let original = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        )
        .unwrap();
        for aes256 in [false, true] {
            let path = dir.path().join(format!("restricted-{aes256}.pdf"));
            let protected = security::protect(
                &original,
                None,
                &Protection {
                    owner_password: "current-owner".into(),
                    allow_copy: false,
                    allow_modify: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
            std::fs::write(&path, &protected).unwrap();
            worker.send(Request::Open {
                path: path.clone(),
                password: None,
            });
            assert!(matches!(next_response(worker), Response::Opened(_)));
            worker.send(Request::BeginProtection { epoch: 81 });
            assert!(matches!(
                next_response(worker),
                Response::ProtectionAuthorization {
                    epoch: 81,
                    encrypted: true
                }
            ));
            let replacement = || Protection {
                owner_password: "new-owner".into(),
                aes256,
                ..Default::default()
            };
            worker.send(Request::Edit(EditOp::Bytes(BytesOp::Protect {
                protection: replacement(),
                current_owner: None,
            })));
            assert!(matches!(
                next_response(worker),
                Response::OperationFailed(EngineError::PasswordRequired)
            ));
            worker.send(Request::Properties);
            assert!(
                matches!(
                    next_response(worker),
                    Response::Properties(Properties {
                        encrypted: true,
                        can_copy: false,
                        can_modify: false,
                        ..
                    })
                ),
                "refused reprotection must preserve restrictions"
            );
            worker.send(Request::Edit(EditOp::Bytes(BytesOp::Protect {
                protection: replacement(),
                current_owner: Some(zeroize::Zeroizing::new("current-owner".into())),
            })));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            worker.send(Request::Undo);
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            worker.send(Request::Properties);
            assert!(matches!(
                next_response(worker),
                Response::Properties(Properties {
                    encrypted: true,
                    can_copy: false,
                    can_modify: false,
                    ..
                })
            ));
            assert_eq!(std::fs::read(&path).unwrap(), protected);
        }
    }

    fn prepared_redaction_has_one_undo_and_cancellation_keeps_history(worker: &Worker) {
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::RedactSearch {
            query: "SuniPDF page 1".into(),
            match_case: true,
            verify: false,
        });
        let marks = match next_response(worker) {
            Response::RedactMatches { marks, .. } => marks,
            _ => panic!("redaction marks"),
        };
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        worker.send(Request::PreviewRedaction {
            marks: marks.clone(),
            options: RedactOptions::default(),
            appearance: Default::default(),
            page: 0,
            epoch: 42,
            generation: 1,
            cancel: cancel.clone(),
        });
        assert!(matches!(
            next_response(worker),
            Response::RedactionPreview { result: Ok(_), .. }
        ));
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        worker.send(Request::CommitRedactionPreview {
            epoch: 42,
            generation: 1,
        });
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(EngineError::Cancelled)
        ));
        worker.send(Request::PreviewRedaction {
            marks,
            options: RedactOptions::default(),
            appearance: Default::default(),
            page: 0,
            epoch: 42,
            generation: 2,
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        assert!(matches!(
            next_response(worker),
            Response::RedactionPreview { result: Ok(_), .. }
        ));
        worker.send(Request::CommitRedactionPreview {
            epoch: 42,
            generation: 2,
        });
        assert!(matches!(next_response(worker), Response::Redacted(_)));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        worker.send(Request::CommitRedactionPreview {
            epoch: 42,
            generation: 2,
        });
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: true,
                can_redo: false,
                ..
            })
        ));
    }

    fn rich_annotation_previews_commit_exactly_once_and_reject_stale_or_discarded_candidates(
        worker: &Worker,
    ) {
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let details = || {
            worker.send(Request::Annotations {
                page: 0,
                detailed: true,
            });
            match next_response(worker) {
                Response::Annotations {
                    details: Some(details),
                    ..
                } => details,
                Response::Annotations {
                    list,
                    details: None,
                    ..
                } if list.is_empty() => Vec::new(),
                _ => panic!("rich annotations"),
            }
        };
        let preview = |generation| {
            worker.send(Request::PreviewRichAnnotation {
                page: 0,
                annotation: sunipdf_core::annot_rich::RichAnnotation::Stamp {
                    rect: PointRect {
                        left: 40.0,
                        bottom: 180.0,
                        right: 200.0,
                        top: 230.0,
                    },
                    kind: sunipdf_core::annot_rich::StampKind::Approved,
                },
                author: Some("lmdanavanem".into()),
                epoch: 7,
                generation,
            });
            assert!(
                matches!(next_response(worker),Response::RichAnnotationPreview {epoch:7,generation:sent,result:Ok(_)} if sent==generation)
            );
        };
        let before = details();
        preview(11);
        assert_eq!(
            details(),
            before,
            "preview is read-only and creates no history"
        );
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 7,
            generation: 11,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        let after = details();
        assert_eq!(after.len(), before.len() + 1);
        assert_eq!(after.last().unwrap().subtype, "Stamp");
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(details(), before);
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(details(), after);
        preview(12);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 7,
            generation: 12,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(details(), before, "Undo invalidates the prepared candidate");
        preview(13);
        worker.send(Request::DiscardRichAnnotationPreview {
            epoch: 7,
            generation: 13,
        });
        worker.send(Request::Edit(EditOp::CommitRichAnnotationPreview {
            epoch: 7,
            generation: 13,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(details(), before);
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        assert_eq!(details(), after, "refusals do not consume Redo history");
    }

    fn native_clipboard_paste_across_documents_has_one_undo_and_no_silent_group_fallback(
        worker: &Worker,
    ) {
        let open = || {
            worker.send(Request::Open {
                path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
                password: None,
            });
            assert!(matches!(next_response(worker), Response::Opened(_)));
        };
        let objects = |page| {
            worker.send(Request::Objects { page });
            match next_response(worker) {
                Response::Objects { list, .. } => list,
                _ => panic!("clipboard objects"),
            }
        };
        open();
        let selected: Vec<_> = objects(0)
            .iter()
            .filter(|object| {
                matches!(
                    object.kind,
                    sunipdf_core::ObjectKind::Text | sunipdf_core::ObjectKind::Path
                )
            })
            .take(2)
            .map(|object| object.index)
            .collect();
        assert_eq!(selected.len(), 2);
        worker.send(Request::CopyObjects {
            page: 0,
            indices: selected,
            cut: false,
        });
        assert!(matches!(
            next_response(worker),
            Response::Copied { count: 2, .. }
        ));
        open();
        let before = objects(1);
        worker.send(Request::Edit(EditOp::PasteEditableObjects {
            page: 1,
            dx: 0.0,
            dy: 0.0,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { can_undo: true, .. })
        ));
        let after = objects(1);
        assert_eq!(after.len(), before.len() + 2);
        assert!(after[before.len()..]
            .iter()
            .all(|object| object.kind != sunipdf_core::ObjectKind::Form));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(objects(1), before, "one Undo removes every pasted object");
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(objects(1), after);
        worker.send(Request::Edit(EditOp::PasteObjects {
            page: 1,
            dx: 0.0,
            dy: 0.0,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let grouped = objects(1);
        let form = grouped.last().unwrap();
        assert_eq!(form.kind, sunipdf_core::ObjectKind::Form);
        worker.send(Request::CopyObjects {
            page: 1,
            indices: vec![form.index],
            cut: false,
        });
        assert!(matches!(
            next_response(worker),
            Response::Copied { count: 1, .. }
        ));
        worker.send(Request::Edit(EditOp::PasteEditableObjects {
            page: 1,
            dx: 0.0,
            dy: 0.0,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(objects(1), grouped);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(
            objects(1),
            after,
            "refusal creates no Undo entry or grouped fallback"
        );
    }

    fn paragraph_preview_and_reflow_keep_one_undo_and_failed_edit_history(worker: &Worker) {
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let objects = || {
            worker.send(Request::Objects { page: 0 });
            match next_response(worker) {
                Response::Objects { list, .. } => list,
                _ => panic!("page objects"),
            }
        };
        let before = objects();
        let index = before
            .iter()
            .find(|o| o.kind == sunipdf_core::ObjectKind::Text)
            .unwrap()
            .index;
        let text = "Native paragraph words across multiple editable lines.";
        worker.send(Request::PreviewParagraph {
            page: 0,
            indices: vec![index],
            text: text.into(),
            width: 180.0,
            leading: 1.2,
            epoch: 7,
            generation: 9,
        });
        let count = match next_response(worker) {
            Response::ParagraphPreview {
                epoch: 7,
                generation: 9,
                result: Ok(preview),
            } => preview.objects,
            _ => panic!("native paragraph preview"),
        };
        assert!(count > 1);
        assert_eq!(objects(), before, "preview is read-only");
        let edit = |indices: Vec<usize>, value: &str| {
            worker.send(Request::Edit(EditOp::ReflowParagraph {
                page: 0,
                indices,
                text: value.into(),
                width: 180.0,
                leading: 1.2,
            }))
        };
        edit(vec![index], text);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: true,
                can_undo: true,
                ..
            })
        ));
        let after = objects();
        assert_eq!(after.len(), before.len() - 1 + count);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: false,
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(objects(), before, "one Undo restores all replaced lines");
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(objects(), after);
        edit((index..index + count).collect(), "Missing 🚀");
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(EngineError::MissingGlyphs(_))
        ));
        assert_eq!(objects(), after);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                can_undo: false,
                ..
            })
        ));
        assert_eq!(objects(), before, "refusal creates no undo entry");
    }

    fn existing_text_missing_glyphs_keep_history_and_original_objects(worker: &Worker) {
        let objects = || {
            worker.send(Request::Objects { page: 0 });
            match next_response(worker) {
                Response::Objects { list, .. } => list,
                _ => panic!("text objects"),
            }
        };
        let before = objects();
        let title = before
            .iter()
            .find(|object| object.text.as_deref() == Some("SuniPDF page 1"))
            .unwrap();
        let set_text = |text: &str| {
            worker.send(Request::Edit(EditOp::SetText {
                page: 0,
                index: title.index,
                text: Some(text.into()),
                size: None,
                rgb: None,
            }));
        };
        set_text("Updated PDF text");
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: true,
                can_undo: true,
                ..
            })
        ));
        let updated = objects();
        set_text("Updated Ж");
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(EngineError::MissingGlyphs(chars)) if chars == "Ж"
        ));
        assert_eq!(objects(), updated);
        worker.send(Request::Undo);
        assert!(
            matches!(
                next_response(worker),
                Response::DocumentChanged(DocState {
                    modified: false,
                    can_undo: false,
                    can_redo: true,
                    ..
                })
            ),
            "rejected text must not add history or mark the document modified"
        );
        assert_eq!(objects(), before);
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(objects(), updated);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(objects(), before);
    }

    fn grouped_ink_has_one_undo_redo_and_failed_edits_keep_history(worker: &Worker) {
        let changed = || match next_response(worker) {
            Response::DocumentChanged(state) => state,
            _ => panic!("ink document state"),
        };
        let annotations = || {
            worker.send(Request::Annotations {
                page: 0,
                detailed: true,
            });
            match next_response(worker) {
                Response::Annotations { details, .. } => details.unwrap_or_default(),
                _ => panic!("ink annotations"),
            }
        };
        let before = annotations();
        let strokes = vec![
            vec![(30.0, 40.0), (80.0, 90.0), (130.0, 40.0)],
            vec![(30.0, 110.0), (130.0, 110.0)],
            vec![(150.0, 110.0)],
        ];
        let send = |page, strokes| {
            worker.send(Request::Edit(EditOp::AddInk {
                page,
                strokes,
                style: AnnotStyle::default(),
                author: Some("lmdanavanem".into()),
            }))
        };
        send(0, strokes.clone());
        let state = changed();
        assert!(state.pages_unchanged && state.modified && state.can_undo && !state.can_redo);
        let sizes = state.info.page_sizes;
        let grouped = annotations();
        assert_eq!(grouped.len(), before.len() + 1);
        assert_eq!(grouped.last().unwrap().subtype, "Ink");
        worker.send(Request::Undo);
        let state = changed();
        assert!(state.pages_unchanged && !state.modified && !state.can_undo && state.can_redo);
        assert_eq!(state.info.page_sizes, sizes);
        assert_eq!(annotations(), before);
        worker.send(Request::Redo);
        assert!(changed().modified);
        assert_eq!(annotations(), grouped);
        for (page, paths) in [(usize::MAX, strokes), (0, vec![vec![(20.0, 30.0)]; 65])] {
            send(page, paths);
            assert!(matches!(
                next_response(worker),
                Response::OperationFailed(_)
            ));
            assert_eq!(annotations(), grouped);
        }
        worker.send(Request::Undo);
        let state = changed();
        assert!(!state.modified && !state.can_undo && state.can_redo);
        assert_eq!(
            annotations(),
            before,
            "failed edits must not add undo entries"
        );
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn dashed_annotation_styles_survive_edits_and_history(worker: &Worker) {
        let changed = || match next_response(worker) {
            Response::DocumentChanged(state) => {
                assert!(state.pages_unchanged && state.modified && state.can_undo);
                state.info.page_sizes
            }
            _ => panic!("annotation document state"),
        };
        let rect = PointRect {
            left: 50.0,
            bottom: 100.0,
            right: 150.0,
            top: 160.0,
        };
        worker.send(Request::Edit(EditOp::AddAnnotShape {
            page: 0,
            shape: AnnotShape::Square,
            rect,
            style: AnnotStyle::default(),
            author: Some("lmdanavanem".into()),
        }));
        let sizes = changed();
        let detail = || {
            worker.send(Request::Annotations {
                page: 0,
                detailed: true,
            });
            match next_response(worker) {
                Response::Annotations {
                    details: Some(details),
                    ..
                } => details.into_iter().last().expect("annotation"),
                _ => panic!("annotation details"),
            }
        };
        let original = detail();
        assert!(original.dash_pattern.is_empty());
        let style = AnnotStyle {
            dash_pattern: vec![6.0, 3.0],
            ..Default::default()
        };
        worker.send(Request::Edit(EditOp::StyleAnnotation {
            page: 0,
            index: original.index,
            style: style.clone(),
        }));
        assert_eq!(changed(), sizes);
        assert_eq!(detail().dash_pattern, style.dash_pattern);
        worker.send(Request::Undo);
        assert_eq!(changed(), sizes);
        assert!(detail().dash_pattern.is_empty());
        worker.send(Request::Redo);
        assert_eq!(changed(), sizes);
        assert_eq!(detail().dash_pattern, style.dash_pattern);
        worker.send(Request::Edit(EditOp::MoveAnnotation {
            page: 0,
            index: original.index,
            dx: 5.0,
            dy: 7.0,
        }));
        changed();
        let moved = detail();
        assert_eq!(moved.dash_pattern, style.dash_pattern);
        worker.send(Request::Edit(EditOp::ResizeAnnotation {
            page: 0,
            index: original.index,
            rect,
        }));
        changed();
        assert_eq!(detail().dash_pattern, style.dash_pattern);
        worker.send(Request::Edit(EditOp::StyleAnnotation {
            page: 0,
            index: original.index,
            style: AnnotStyle {
                dash_pattern: vec![f32::NAN],
                ..style
            },
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        worker.send(Request::Undo);
        changed();
        assert_eq!(detail().rect, moved.rect);
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn link_target_edits_have_one_undo_and_preserve_geometry(worker: &Worker) {
        let changed = || match next_response(worker) {
            Response::DocumentChanged(state) => {
                assert!(state.pages_unchanged && state.modified && state.can_undo);
                state.info.page_sizes
            }
            _ => panic!("link target document state"),
        };
        let before = LinkTarget::Uri("https://example.org/original".into());
        worker.send(Request::Edit(EditOp::AddLink {
            page: 0,
            rect: PointRect {
                left: 50.0,
                bottom: 100.0,
                right: 150.0,
                top: 125.0,
            },
            target: before.clone(),
        }));
        let sizes = changed();
        let link = || {
            worker.send(Request::Links { page: 0 });
            match next_response(worker) {
                Response::Links { list, .. } => list.into_iter().last().expect("link"),
                _ => panic!("link response"),
            }
        };
        let original = link();
        assert_eq!(original.target, before);
        worker.send(Request::Edit(EditOp::SetLinkTarget {
            page: 0,
            index: original.index,
            target: LinkTarget::Page(1),
        }));
        assert_eq!(changed(), sizes);
        assert_eq!(link().target, LinkTarget::Page(1));
        worker.send(Request::Undo);
        assert_eq!(changed(), sizes);
        assert_eq!(link().target, before);
        worker.send(Request::Redo);
        assert_eq!(changed(), sizes);
        let updated = link();
        assert_eq!(updated.target, LinkTarget::Page(1));
        assert_eq!(updated.rect, original.rect);
        worker.send(Request::Edit(EditOp::SetLinkTarget {
            page: 0,
            index: original.index,
            target: LinkTarget::Uri("javascript:alert(1)".into()),
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(link().target, LinkTarget::Page(1));
        worker.send(Request::Undo);
        changed();
        assert_eq!(link().target, before);
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn page_label_edits_and_history_keep_page_geometry(worker: &Worker) {
        let changed = || match next_response(worker) {
            Response::DocumentChanged(state) => state,
            _ => panic!("page label document state"),
        };
        let range = PageLabelRange {
            first: 0,
            last: 0,
            style: sunipdf_core::PageLabelStyle::RomanLower,
            prefix: String::new(),
            start: 1,
        };
        worker.send(Request::Edit(EditOp::SetPageLabels(range)));
        let state = changed();
        assert!(state.pages_unchanged && state.modified && state.can_undo);
        assert_eq!(state.info.page_label(0), Some("i"));
        assert_eq!(state.info.page_label(1), Some("2"));
        let sizes = state.info.page_sizes;

        worker.send(Request::Undo);
        let state = changed();
        assert!(state.pages_unchanged && !state.modified && state.can_redo);
        assert_eq!(state.info.page_labels, vec![None, None]);
        assert_eq!(state.info.page_sizes, sizes);
        worker.send(Request::Redo);
        let state = changed();
        assert!(state.pages_unchanged && state.modified && state.can_undo);
        assert_eq!(state.info.page_label(0), Some("i"));

        worker.send(Request::Edit(EditOp::ResetPageLabels));
        assert_eq!(changed().info.page_labels, vec![None, None]);
        worker.send(Request::Undo);
        let state = changed();
        assert!(state.pages_unchanged);
        assert_eq!(state.info.page_label(0), Some("i"));
        worker.send(Request::Redo);
        let state = changed();
        assert!(state.pages_unchanged);
        assert_eq!(state.info.page_labels, vec![None, None]);

        worker.send(Request::Edit(EditOp::Rotate {
            pages: vec![0],
            delta_degrees: 90,
        }));
        assert!(!changed().pages_unchanged);
        worker.send(Request::Undo);
        let state = changed();
        assert!(!state.pages_unchanged);
        assert_eq!(state.info.page_sizes, sizes);
        worker.send(Request::Redo);
        assert!(!changed().pages_unchanged);

        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn render_signature_preview(worker: &Worker, id: u64) -> Arc<image::DynamicImage> {
        worker.send(Request::RenderSignature {
            id,
            text: "Emanuel".into(),
            style: TextStyle {
                size_pt: 48.0,
                family: sunipdf_core::FontFamily::Times,
                italic: true,
                ..Default::default()
            },
        });
        match next_response(worker) {
            Response::SignatureRendered {
                id: received,
                result: Ok(image),
            } => {
                assert_eq!(received, id);
                assert!(image.width() > 0 && image.height() > 0);
                assert!(image.pixels().any(|pixel| pixel[3] > 0));
                Arc::new(image::DynamicImage::ImageRgba8(image))
            }
            _ => panic!("signature preview response"),
        }
    }

    fn signature_batch_placement_has_one_undo_and_no_partial_failures(
        worker: &Worker,
        image: Arc<image::DynamicImage>,
    ) {
        let objects = || {
            (0..2)
                .map(|page| {
                    worker.send(Request::Objects { page });
                    match next_response(worker) {
                        Response::Objects {
                            page: received,
                            list,
                        } if received == page => list,
                        _ => panic!("page objects response"),
                    }
                })
                .collect::<Vec<_>>()
        };
        let before = objects();
        let preview = render_signature_preview(worker, 42);
        assert_eq!(preview.as_rgba8(), image.as_rgba8());
        assert!(!worker.has_pending_mutations());
        assert_eq!(objects(), before, "preview must not change page content");

        for (pages, cancelled) in [(vec![0, 1], true), (vec![0, usize::MAX], false)] {
            worker.send(Request::PlaceSignature {
                pages,
                image: image.clone(),
                center: (200.0, 200.0),
                width_pt: 120.0,
                cancel: Arc::new(std::sync::atomic::AtomicBool::new(cancelled)),
            });
            assert!(worker.has_pending_mutations());
            match next_response(worker) {
                Response::TaskDone {
                    kind: TaskKind::Signature,
                    result: Err(error),
                } => {
                    assert_eq!(matches!(error, EngineError::Cancelled), cancelled);
                }
                _ => panic!("failed signature batch response"),
            }
            assert_eq!(
                objects(),
                before,
                "failed batch must not change either page"
            );
            assert!(!worker.has_pending_mutations());
        }

        worker.send(Request::PlaceSignature {
            pages: vec![0, 1, 0],
            image,
            center: (200.0, 200.0),
            width_pt: 120.0,
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        for expected in 1..=2 {
            assert!(matches!(next_response(worker), Response::TaskProgress {
                kind: TaskKind::Signature, done, total: 2,
            } if done == expected));
        }
        assert!(matches!(
            next_response(worker),
            Response::TaskDone {
                kind: TaskKind::Signature,
                result: Ok(()),
            }
        ));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: true,
                can_undo: true,
                can_redo: false,
                pages_unchanged: true,
                ..
            })
        ));
        let placed = objects();
        for (original, changed) in before.iter().zip(&placed) {
            assert_eq!(changed.len(), original.len() + 1);
            assert_eq!(
                changed.last().unwrap().kind,
                sunipdf_core::ObjectKind::Image
            );
        }

        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: false,
                can_undo: false,
                can_redo: true,
                ..
            })
        ));
        assert_eq!(objects(), before, "one undo must remove the entire batch");
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState {
                modified: true,
                can_undo: true,
                can_redo: false,
                ..
            })
        ));
        assert_eq!(objects(), placed, "one redo must restore both images");
    }

    fn drawn_form_rectangles_keep_history_and_saved_fields(worker: &Worker) {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let source_bytes = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        let size = match next_response(worker) {
            Response::Opened(state) => state.info.page_sizes[0],
            _ => panic!("fresh form document"),
        };
        let model = || {
            worker.send(Request::FormInfo);
            match next_response(worker) {
                Response::FormInfo(Ok(info)) => info,
                _ => panic!("form model"),
            }
        };
        let kinds = [
            FieldKind::Text,
            FieldKind::Checkbox,
            FieldKind::Radio,
            FieldKind::Combo,
            FieldKind::List,
        ];
        for (index, kind) in kinds.into_iter().enumerate() {
            let mut builder = crate::form_builder::FormBuilderState::default();
            builder.open(0, kind, size);
            builder.start_drawing();
            builder.start_drag(0, (200.0, 180.0), size);
            builder.move_drag(0, (40.0, 60.0));
            let rect = builder.drag.unwrap().rect();
            builder.finish_drag();
            assert!(builder.visible && !builder.drawing);
            let choice = matches!(kind, FieldKind::Radio | FieldKind::Combo | FieldKind::List);
            worker.send(Request::Edit(EditOp::AddFormField {
                page: 0,
                name: format!("Drawn{index}"),
                kind,
                rect,
                options: if choice {
                    vec![
                        FieldOption {
                            value: "FR".into(),
                            label: "France".into(),
                        },
                        FieldOption {
                            value: "BE".into(),
                            label: "Belgique".into(),
                        },
                    ]
                } else {
                    Vec::new()
                },
                multi_select: kind == FieldKind::List,
                editable: kind == FieldKind::Combo,
            }));
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(DocState {
                    pages_unchanged: true,
                    modified: true,
                    can_undo: true,
                    ..
                })
            ));
            let created = model();
            let field = created.field(&format!("Drawn{index}")).unwrap();
            assert_eq!(field.kind, kind);
            assert_eq!(field.editable, kind == FieldKind::Combo);
            assert_eq!(field.multi_select, kind == FieldKind::List);
            assert_eq!(
                field.widgets.len(),
                if kind == FieldKind::Radio { 2 } else { 1 }
            );
            for widget in &field.widgets {
                assert_eq!(widget.page, 0);
                assert!(widget.display.left >= rect.left && widget.display.right <= rect.right);
                assert!(widget.display.bottom >= rect.bottom && widget.display.top <= rect.top);
            }
            worker.send(Request::Undo);
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            assert!(model().field(&format!("Drawn{index}")).is_none());
            worker.send(Request::Redo);
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(_)
            ));
            assert_eq!(model(), created);
        }
        let before = model();
        worker.send(Request::Edit(EditOp::AddFormField {
            page: 0,
            name: "Drawn0".into(),
            kind: FieldKind::Text,
            rect: PointRect {
                left: 40.0,
                bottom: 60.0,
                right: 200.0,
                top: 180.0,
            },
            options: Vec::new(),
            multi_select: false,
            editable: false,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(model(), before);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(
            model().field("Drawn4").is_none(),
            "a failed creation must not add history"
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("drawn-fields.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(model(), before);
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        worker.send(Request::Open {
            path: source,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn inline_free_text_has_exact_styles_geometry_history_and_saved_content(worker: &Worker) {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let original_bytes = std::fs::read(&source).unwrap();
        worker.send(Request::Open {
            path: source.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let annotations = || {
            worker.send(Request::Annotations {
                page: 0,
                detailed: true,
            });
            match next_response(worker) {
                Response::Annotations {
                    details: Some(details),
                    ..
                } => details,
                Response::Annotations {
                    list,
                    details: None,
                    ..
                } if list.is_empty() => Vec::new(),
                _ => panic!("free text annotation details"),
            }
        };
        let changed = || {
            assert!(matches!(
                next_response(worker),
                Response::DocumentChanged(DocState {
                    pages_unchanged: true,
                    ..
                })
            ));
        };
        let before = annotations();
        let rect = PointRect {
            left: 40.0,
            bottom: 600.0,
            right: 220.0,
            top: 624.0,
        };
        let style = FreeTextStyle {
            size_pt: 18.0,
            rgb: (20, 40, 60),
            fill_rgb: Some((240, 230, 220)),
            border_pt: 1.5,
        };
        let text =
            "Hello from the page\nBonjour depuis la page\nA third line that wraps in the box";
        worker.send(Request::Edit(EditOp::AddFreeText {
            page: 0,
            rect,
            text: text.into(),
            style: style.clone(),
            author: Some("lmdanavanem".into()),
        }));
        changed();
        let created = annotations();
        assert_eq!(created.len(), before.len() + 1);
        let detail = created.last().unwrap();
        assert_eq!(detail.subtype, "FreeText");
        assert_eq!(detail.contents.as_deref(), Some(text));
        assert_eq!(detail.font_size, Some(style.size_pt));
        assert_eq!(detail.rgb, Some(style.rgb));
        assert_eq!(detail.fill_rgb, style.fill_rgb);
        assert_eq!(detail.width_pt, Some(style.border_pt));
        assert_eq!(detail.author.as_deref(), Some("lmdanavanem"));
        assert_eq!(
            (detail.rect.left, detail.rect.right, detail.rect.top),
            (rect.left, rect.right, rect.top)
        );
        assert!(
            detail.rect.bottom < rect.bottom,
            "multiline wrapping must grow downwards"
        );
        let index = detail.index;
        worker.send(Request::Edit(EditOp::AddFreeText {
            page: 0,
            rect: PointRect {
                right: rect.left + 35.0,
                ..rect
            },
            text: "Too narrow".into(),
            style: style.clone(),
            author: None,
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(annotations(), created);
        worker.send(Request::Undo);
        changed();
        assert_eq!(
            annotations(),
            before,
            "failed addition must not add history"
        );
        worker.send(Request::Redo);
        changed();
        assert_eq!(annotations(), created);
        let edited_text = "Edited on page\nModifie sur la page";
        let edited_style = FreeTextStyle {
            size_pt: 14.0,
            rgb: (80, 60, 40),
            fill_rgb: None,
            border_pt: 0.0,
        };
        worker.send(Request::Edit(EditOp::UpdateFreeText {
            page: 0,
            index,
            text: edited_text.into(),
            style: edited_style.clone(),
        }));
        changed();
        let edited = annotations();
        let detail = edited.last().unwrap();
        assert_eq!(detail.contents.as_deref(), Some(edited_text));
        assert_eq!(detail.font_size, Some(edited_style.size_pt));
        assert_eq!(detail.rgb, Some(edited_style.rgb));
        assert_eq!(detail.fill_rgb, edited_style.fill_rgb);
        assert_eq!(detail.width_pt, Some(edited_style.border_pt));
        assert_eq!(detail.rect, created.last().unwrap().rect);
        worker.send(Request::Edit(EditOp::UpdateFreeText {
            page: 0,
            index,
            text: "Invalid style".into(),
            style: FreeTextStyle {
                size_pt: f32::NAN,
                ..edited_style
            },
        }));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(annotations(), edited);
        worker.send(Request::Undo);
        changed();
        assert_eq!(annotations(), created, "failed update must not add history");
        worker.send(Request::Redo);
        changed();
        assert_eq!(annotations(), edited);
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("inline-free-text.pdf");
        worker.send(Request::Save {
            path: output.clone(),
        });
        assert!(matches!(next_response(worker), Response::Saved { .. }));
        worker.send(Request::Open {
            path: output,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        assert_eq!(annotations(), edited);
        assert_eq!(std::fs::read(&source).unwrap(), original_bytes);
        worker.send(Request::Open {
            path: source,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn bookmark_edits_have_one_undo_and_failed_edits_keep_history(worker: &Worker) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        worker.send(Request::Open {
            path: path.clone(),
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        let bookmarks = || {
            worker.send(Request::Bookmarks);
            match next_response(worker) {
                Response::Bookmarks(bookmarks) => bookmarks,
                _ => panic!("bookmarks response"),
            }
        };
        let original_bookmarks = bookmarks();
        worker.send(Request::Edit(EditOp::Bytes(BytesOp::Bookmark(
            sunipdf_core::bookmarks::BookmarkEdit::Add {
                parent: None,
                title: "Undo bookmark".into(),
                page: 1,
            },
        ))));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let edited_bookmarks = bookmarks();
        assert_eq!(edited_bookmarks.len(), original_bookmarks.len() + 1);
        assert_eq!(
            edited_bookmarks[..original_bookmarks.len()],
            original_bookmarks
        );
        assert_eq!(edited_bookmarks.last().unwrap().page, Some(1));
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(bookmarks(), original_bookmarks);
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(bookmarks(), edited_bookmarks);
        assert_eq!(edited_bookmarks.last().unwrap().title, "Undo bookmark");
        worker.send(Request::Edit(EditOp::Bytes(BytesOp::Bookmark(
            sunipdf_core::bookmarks::BookmarkEdit::Delete { index: 999 },
        ))));
        assert!(matches!(
            next_response(worker),
            Response::OperationFailed(_)
        ));
        assert_eq!(bookmarks(), edited_bookmarks);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(bookmarks(), original_bookmarks);
        worker.send(Request::Open {
            path,
            password: None,
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
    }

    fn editable_combo_creation_and_filling_undo_redo(worker: &Worker) {
        let form_info = || {
            worker.send(Request::FormInfo);
            match next_response(worker) {
                Response::FormInfo(Ok(info)) => info,
                _ => panic!("form model response"),
            }
        };
        worker.send(Request::Edit(EditOp::AddFormField {
            page: 0,
            name: "Country".into(),
            kind: FieldKind::Combo,
            rect: PointRect {
                left: 36.0,
                bottom: 36.0,
                right: 216.0,
                top: 60.0,
            },
            options: vec![FieldOption {
                value: "FR".into(),
                label: "France".into(),
            }],
            multi_select: false,
            editable: true,
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let info = form_info();
        let field = info.field("Country").unwrap();
        assert!(field.editable);
        assert_eq!(field.kind, FieldKind::Combo);
        assert_eq!(field.value, FieldValue::Choice(Vec::new()));

        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(form_info().field("Country").is_none());
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(form_info().field("Country").unwrap().editable);

        let custom = FieldValue::Choice(vec!["Qu\u{e9}bec".into()]);
        worker.send(Request::Edit(EditOp::FillForm {
            values: vec![("Country".into(), custom.clone())],
        }));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(form_info().field("Country").unwrap().value, custom);
        worker.send(Request::Undo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(
            form_info().field("Country").unwrap().value,
            FieldValue::Choice(Vec::new())
        );
        worker.send(Request::Redo);
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert_eq!(form_info().field("Country").unwrap().value, custom);
    }

    fn prepare_and_commit_redaction(worker: &Worker, marks: Vec<RedactMark>, generation: u64) {
        let page = marks.first().expect("nonempty redaction marks").page;
        worker.send(Request::PreviewRedaction {
            marks,
            options: RedactOptions::default(),
            appearance: Default::default(),
            page,
            epoch: 73,
            generation,
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        assert!(
            matches!(next_response(worker), Response::RedactionPreview { epoch: 73, generation: returned, result: Ok(_) } if returned == generation)
        );
        worker.send(Request::CommitRedactionPreview {
            epoch: 73,
            generation,
        });
    }

    fn encrypted_recovery_after_redaction(worker: &Worker) {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("original.pdf");
        let bytes = sunipdf_core::security::protect(
            include_bytes!("../../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            None,
            &Protection {
                user_password: "recovery-password".into(),
                owner_password: "owner-password".into(),
                ..Default::default()
            },
        )
        .unwrap();
        std::fs::write(&original, &bytes).unwrap();
        let session = crate::recovery::Session::create(temp.path().join("recovery")).unwrap();
        let slot = session.slot(1, original.clone(), 2);
        worker.send(Request::Open {
            path: original.clone(),
            password: Some("recovery-password".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::SetRecovery(Some(slot.clone())));
        worker.send(Request::MarkRecovered {
            path: original.clone(),
        });
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(DocState { modified: true, .. })
        ));
        let extracted = temp.path().join("extracted.pdf");
        worker.send(Request::Batch(BatchOp::Extract {
            pages: vec![0],
            out: extracted.clone(),
        }));
        assert!(matches!(
            next_response(worker),
            Response::BatchDone(BatchResult::Extracted {
                encrypted: true,
                pages: 1,
                ..
            })
        ));
        assert!(lopdf_encrypted(&std::fs::read(&extracted).unwrap()));
        worker.send(Request::Batch(BatchOp::Split {
            every: 1,
            out_dir: temp.path().join("split"),
        }));
        match next_response(worker) {
            Response::BatchDone(BatchResult::Split {
                files,
                encrypted: true,
            }) => {
                assert_eq!(files.len(), 2);
                for file in files {
                    assert!(lopdf_encrypted(&std::fs::read(file).unwrap()));
                }
            }
            _ => panic!("protected split result"),
        }
        assert_eq!(std::fs::read(&original).unwrap(), bytes);
        worker.send(Request::Autosave);
        assert!(matches!(
            next_response(worker),
            Response::RecoverySaved(Ok(()))
        ));
        let before = std::fs::read(slot.base.with_extension("pdf")).unwrap();
        assert!(lopdf_encrypted(&before));
        worker.send(Request::RedactSearch {
            query: "SuniPDF page 1".into(),
            match_case: true,
            verify: false,
        });
        let marks = match next_response(worker) {
            Response::RedactMatches { marks, .. } => marks,
            _ => panic!("redaction search response"),
        };
        assert_eq!(marks.len(), 1);
        prepare_and_commit_redaction(worker, marks.clone(), 1);
        assert!(matches!(next_response(worker), Response::Redacted(_)));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        let recovered = slot.base.with_extension("pdf");
        let after = std::fs::read(&recovered).unwrap();
        assert!(lopdf_encrypted(&after));
        assert_ne!(before, after, "redaction must refresh recovery immediately");
        worker.send(Request::Open {
            path: recovered.clone(),
            password: None,
        });
        assert!(matches!(
            next_response(worker),
            Response::OpenFailed {
                error: EngineError::PasswordRequired,
                ..
            }
        ));
        worker.send(Request::Open {
            path: recovered,
            password: Some("recovery-password".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::RedactSearch {
            query: "SuniPDF page 1".into(),
            match_case: true,
            verify: true,
        });
        assert!(
            matches!(next_response(worker),Response::RedactMatches { marks,.. } if marks.is_empty())
        );
        assert_eq!(std::fs::read(&original).unwrap(), bytes);

        // Fail recovery metadata publication while keeping the sensitive PDF deletable.
        let failed_slot = session.slot(2, original.clone(), 2);
        failed_slot.save(&bytes).unwrap();
        let stale = failed_slot.base.with_extension("pdf");
        let blocked = stale.with_extension("json");
        std::fs::remove_file(&blocked).unwrap();
        std::fs::create_dir(&blocked).unwrap();
        worker.send(Request::Open {
            path: original.clone(),
            password: Some("recovery-password".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::SetRecovery(Some(failed_slot.clone())));
        prepare_and_commit_redaction(worker, marks, 2);
        assert!(matches!(
            next_response(worker),
            Response::RecoverySaved(Err(_))
        ));
        assert!(matches!(next_response(worker), Response::Redacted(_)));
        assert!(matches!(
            next_response(worker),
            Response::DocumentChanged(_)
        ));
        assert!(
            !stale.exists(),
            "failed refresh must remove the unredacted recovery PDF"
        );
        assert!(
            !blocked.is_file(),
            "no recoverable metadata may remain after a failed refresh"
        );
        std::fs::remove_dir(blocked).unwrap();
        assert_eq!(std::fs::read(original).unwrap(), bytes);
    }

    fn lopdf_encrypted(bytes: &[u8]) -> bool {
        // The raw encryption dictionary is detectable without a password or another PDFium engine.
        bytes.windows(b"/Encrypt".len()).any(|w| w == b"/Encrypt")
    }

    #[test]
    fn signed_copy_final_write_never_replaces_an_existing_file_or_hard_link() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("signed-copy.pdf");
        write_new_copy(&output, b"first signed bytes").unwrap();
        assert!(matches!(
            write_new_copy(&output, b"second signed bytes"),
            Err(EngineError::AlreadyExists(_))
        ));
        assert_eq!(std::fs::read(&output).unwrap(), b"first signed bytes");
        let alias = temp.path().join("alias.pdf");
        std::fs::hard_link(&output, &alias).unwrap();
        assert!(matches!(
            write_new_copy(&alias, b"replacement"),
            Err(EngineError::AlreadyExists(_))
        ));
        assert_eq!(std::fs::read(output).unwrap(), b"first signed bytes");
    }

    fn job(page: usize) -> TileJob {
        TileJob {
            page,
            scale_milli: 1000,
            region: PixelRect::new(0, 0, 10, 10),
        }
    }

    #[test]
    fn wanted_queue_is_replaced_not_appended() {
        let mut q = WantedQueue::default();
        q.replace(1, vec![job(0), job(1), job(2)]);
        assert_eq!(q.pop(), Some((1, job(0))));
        // Défilement : la nouvelle liste remplace le reste de l'ancienne (pages 1 et 2 annulées).
        q.replace(2, vec![job(7), job(8)]);
        assert_eq!(q.pop(), Some((2, job(7))));
        assert_eq!(q.pop(), Some((2, job(8))));
        assert_eq!(q.pop(), None);
        q.replace(3, vec![job(4)]);
        q.clear();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn content_edits_keep_pages_structural_edits_do_not() {
        let rect = PointRect {
            left: 0.0,
            bottom: 0.0,
            right: 1.0,
            top: 1.0,
        };
        let keeps = [
            EditOp::MoveObjects {
                page: 0,
                indices: vec![0],
                dx: 1.0,
                dy: 0.0,
            },
            EditOp::Markup {
                page: 0,
                kind: MarkupKind::Highlight,
                rects: vec![rect],
                rgb: (255, 255, 0),
            },
            EditOp::Stamp {
                pages: None,
                stamp: Stamp::default(),
            },
        ];
        assert!(keeps.iter().all(EditOp::keeps_pages));
        let changes = [
            EditOp::Rotate {
                pages: vec![0],
                delta_degrees: 90,
            },
            EditOp::Delete { pages: vec![0] },
            EditOp::Insert {
                source: PathBuf::new(),
                password: None,
                pages: vec![],
                at: 0,
            },
            EditOp::Move { from: 0, to: 1 },
            EditOp::Bytes(BytesOp::StripMetadata),
            EditOp::ResetCrop { pages: vec![0] },
        ];
        assert!(!changes.iter().any(EditOp::keeps_pages));
    }
}
