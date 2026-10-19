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

/// Plafond du cache de rendu. ponytail: constante ; réglage utilisateur quand un besoin réel apparaîtra.
const CACHE_BYTES: usize = 256 * 1024 * 1024;
/// Plafonds du journal d'annulation (instantanés complets du PDF).
const UNDO_MAX_ENTRIES: usize = 50;
const UNDO_MAX_BYTES: usize = 512 * 1024 * 1024;

pub(crate) const CERTIFICATE_SOURCE_CHANGED: &str =
    "certificate source changed; reload the document";
pub(crate) const CERTIFICATE_SAVE_FIRST: &str = "save changes before signing or verifying";

/// Exact bytes last opened or saved, independent of PDFium's rewritten snapshots.
#[derive(Clone)]
pub struct SignatureSource {
    bytes: Arc<[u8]>,
    sha256: [u8; 32],
}

impl SignatureSource {
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
    LocalReview(crate::local_review::Job),
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
    pub has_signatures: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    Print,
    Ocr,
    Signature,
}

pub enum Response {
    LocalReviewProgress {
        done: usize,
        total: usize,
        page: usize,
        pages: usize,
    },
    LocalReviewDone(Result<crate::local_review::Outcome, EngineError>),
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
    /// La reprise d'un onglet suspendu a échoué (le document de cet onglet est perdu).
    ResumeFailed(EngineError),
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
    redaction_preview: Option<(u64, u64, sunipdf_core::redact::PreparedRedaction)>,
}

impl State<'_> {
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
        self.redaction_preview = None;
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
                        ))
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
        to.push(current);
        self.last_nudge = None;
        self.invalidate();
        Some(Response::DocumentChanged(DocState {
            pages_unchanged,
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
            redaction_preview: None,
        }
    }

    fn snap(n: u8) -> Snapshot {
        Snapshot {
            bytes: vec![n],
            password: None,
            pages_unchanged: false,
        }
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
        redaction_preview: None,
    };
    let mut queue: VecDeque<Request> = VecDeque::new();
    let mut navigation_failed = false;
    loop {
        // 1. Travail suivant : file locale, sinon requête reçue, sinon une tuile voulue, sinon
        //    blocage. Entre deux tuiles le canal est relu : un nouveau `SetWanted` ou une édition
        //    passe avant la tuile suivante.
        let request = match queue.pop_front() {
            Some(r) => r,
            None => match rx.try_recv() {
                Ok(r) => r,
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => match state.wanted.pop() {
                    Some((epoch, job)) => {
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
                    None => match rx.recv() {
                        Ok(r) => r,
                        Err(_) => return,
                    },
                },
            },
        };
        // 2. Traitement.
        let response = match request {
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
            Request::Open { .. } | Request::Resume(_) | Request::ResumeDoc(_)
                if navigation_failed =>
            {
                continue
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
                let Some(doc) = &state.document else { continue };
                let matches = doc.search(&query, match_case).unwrap_or_else(|e| {
                    log::warn!("recherche impossible : {e}");
                    Vec::new()
                });
                Response::SearchResults {
                    generation,
                    matches,
                }
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
                match engine.open_bytes(s.bytes, s.password.as_deref(), &s.path) {
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
                    Err(error) => Response::ResumeFailed(error),
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
    fn organize_thumbnail_failures_are_acknowledged_by_the_engine_worker() {
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
        reprotection_requires_the_current_owner_without_removing_existing_restrictions(&worker);
        inspection_uses_saved_bytes_and_refuses_unsaved_changes(&worker);
    }

    fn next_response(worker: &Worker) -> Response {
        loop {
            let response = worker
                .rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("worker response");
            if matches!(response, Response::MutationFinished) {
                worker.finish_mutation();
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
