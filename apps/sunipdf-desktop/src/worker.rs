//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.
//! Cache LRU partagé (tuiles, miniatures), file de tuiles voulues remplacée en bloc par l'UI
//! (`SetWanted`) et rendue une tuile à la fois entre deux requêtes, journal d'annulation par
//! instantanés (octets PDF) pour toutes les modifications de pages.

use std::cell::Cell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter, TryRecvError};
use std::sync::Arc;
use sunipdf_core::{
    form_data, security, write_new_copy, AnnotDetail, AnnotShape, AnnotStyle, AnnotationInfo,
    Bookmark, Document, DocumentInfo, Engine, EngineError, FieldValue, FillMode, FormInfo,
    FreeTextStyle, LinkInfo, LinkTarget, MarkupKind, Metadata, ObjectClip, ObjectInfo, ObjectStyle,
    PageMargins, PointRect, Properties, Protection, RectMode, RedactMark, RedactOptions,
    RedactReport, RenderCache, RenderedPage, ResizeMode, ReviewStatus, ShapeKind, ShapeStyle,
    Stamp, SystemFont, TextMatch, TextSelection, TextStyle, TileJob, ZOrder,
};

/// Plafond du cache de rendu. ponytail: constante ; réglage utilisateur quand un besoin réel apparaîtra.
const CACHE_BYTES: usize = 256 * 1024 * 1024;
/// Plafonds du journal d'annulation (instantanés complets du PDF).
const UNDO_MAX_ENTRIES: usize = 50;
const UNDO_MAX_BYTES: usize = 512 * 1024 * 1024;

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
}

/// Instantané d'annulation : octets du document et mot de passe nécessaire pour les recharger
/// (il change avec Protéger / Retirer la protection).
pub struct Snapshot {
    bytes: Vec<u8>,
    password: Option<String>,
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
        /// Le document d'origine était protégé : les parties ne le sont pas (D046).
        encryption_removed: bool,
    },
    Extracted {
        out: PathBuf,
        pages: usize,
        encryption_removed: bool,
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
    FormExported {
        out: PathBuf,
        count: usize,
    },
    Failed(EngineError),
}

/// Transformation d'octets (lopdf) : chiffrement, métadonnées.
pub enum BytesOp {
    Protect(Protection),
    /// Mot de passe propriétaire saisi par l'utilisateur.
    Unprotect(String),
    SetMetadata(Metadata),
    StripMetadata,
}

/// Modification de pages, appliquée sous instantané (annulable).
pub enum EditOp {
    AddFormField {
        page: usize,
        name: String,
        kind: sunipdf_core::FieldKind,
        rect: PointRect,
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
    ResizeObject {
        page: usize,
        index: usize,
        bounds: PointRect,
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
    RotateObject {
        page: usize,
        index: usize,
        degrees: f32,
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
    Print {
        job: crate::printing::PrintJob,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    Ocr {
        options: sunipdf_core::ocr::OcrOptions,
        cancel: Arc<std::sync::atomic::AtomicBool>,
    },
    VerifySignatures,
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
    /// Caviardage irréversible des zones (annulable : instantané) ; `Redacted` puis `DocumentChanged`.
    Redact {
        marks: Vec<RedactMark>,
        options: RedactOptions,
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
}

pub enum Response {
    TaskProgress {
        kind: TaskKind,
        done: usize,
        total: usize,
    },
    TaskDone {
        kind: TaskKind,
        result: Result<(), EngineError>,
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
    FormInfo(FormInfo),
    Properties(Properties),
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
                | Request::Redact { .. }
                | Request::TextInRect { .. }
                | Request::FormInfo
                | Request::CopyObjects { cut: true, .. }
                | Request::Save { .. }
                | Request::Ocr { .. }
                | Request::Print { .. }
                | Request::SignCertificate { .. }
                | Request::Autosave
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
}

impl State<'_> {
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
    fn redact(
        &mut self,
        marks: &[RedactMark],
        options: &RedactOptions,
    ) -> Result<(RedactReport, DocState), EngineError> {
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| EngineError::Other("no document".into()))?;
        let snapshot = Snapshot {
            bytes: doc.snapshot()?,
            password: doc.password().map(str::to_owned),
        };
        match doc.redact(marks, options) {
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
        to.push(current);
        self.last_nudge = None;
        self.invalidate();
        Some(Response::DocumentChanged(self.doc_state()?))
    }
}

/// Point d'enregistrement : le document n'est « modifié » que si la pile d'annulation n'a pas la
/// longueur mémorisée à l'ouverture ou au dernier enregistrement.
#[cfg(test)]
mod save_point_tests {
    use super::*;

    fn state() -> State<'static> {
        State {
            recovery: None,
            document: None,
            cache: RenderCache::new(1024),
            wanted: WantedQueue::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            save_point: Some(0),
            last_nudge: None,
            clip: None,
            fonts: None,
        }
    }

    fn snap(n: u8) -> Snapshot {
        Snapshot {
            bytes: vec![n],
            password: None,
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
        EditOp::AddFormField {
            page,
            name,
            kind,
            rect,
        } => doc.add_form_field(page, &name, kind, rect),
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
        EditOp::Delete { mut pages } => {
            pages.sort_unstable();
            pages.dedup();
            if pages.len() >= doc.page_count() {
                return Err(EngineError::Other(
                    "a document must keep at least one page".into(),
                ));
            }
            // Du plus grand au plus petit pour garder les indices valides.
            for page in pages.into_iter().rev() {
                doc.delete_page(page)?;
            }
            Ok(())
        }
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
        EditOp::ResizeObject {
            page,
            index,
            bounds,
        } => doc.resize_object(page, index, bounds),
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
        EditOp::RotateObject {
            page,
            index,
            degrees,
        } => doc.rotate_object(page, index, degrees),
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
                BytesOp::Protect(p) => {
                    let bytes = security::protect(&current, password.as_deref(), &p)?;
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
                encryption_removed: doc.is_encrypted(),
            })
        }
        BatchOp::Extract { pages, out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let n = doc.extract_to(&pages, &out)?;
            Ok(BatchResult::Extracted {
                out,
                pages: n,
                encryption_removed: doc.is_encrypted(),
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
        BatchOp::ExportFormData { out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let values = form_data::values_to_json(&doc.form_info()?);
            let count = values.as_object().map_or(0, |m| m.len());
            let text = serde_json::to_string_pretty(&values)
                .map_err(|e| EngineError::Other(format!("json: {e}")))?;
            sunipdf_core::write_atomic(&out, text.as_bytes())?;
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
        cache: RenderCache::new(CACHE_BYTES),
        wanted: WantedQueue::default(),
        undo: Vec::new(),
        redo: Vec::new(),
        save_point: Some(0),
        last_nudge: None,
        clip: None,
        fonts: None,
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
            Request::VerifySignatures => {
                let result = state
                    .document
                    .as_ref()
                    .ok_or_else(|| EngineError::Other("no document".into()))
                    .and_then(|doc| {
                        let bytes = std::fs::read(&doc.info().path)?;
                        sunipdf_core::certificate::verify_signatures(&bytes, doc.password())
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
                    let doc = state
                        .document
                        .as_ref()
                        .ok_or_else(|| EngineError::Other("no document".into()))?;
                    if state.modified() {
                        return Err(EngineError::Other("save changes before signing".into()));
                    }
                    // A fresh path also excludes hard-link aliases of the source.
                    if output.exists() {
                        return Err(EngineError::AlreadyExists(output));
                    }
                    let source = std::fs::read(&doc.info().path)?;
                    let identity = zeroize::Zeroizing::new(std::fs::read(pfx)?);
                    let signed = sunipdf_core::certificate::sign_pdf(
                        &source, &identity, &password, &options,
                    )?;
                    write_new_copy(&output, &signed)?;
                    Ok(output)
                })();
                Response::SignedCopy(result)
            }
            Request::Open { path, password } => match engine.open(&path, password.as_deref()) {
                Ok(doc) => {
                    state.document = Some(doc);
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
            Request::Redact { marks, options } => match state.redact(&marks, &options) {
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
            },
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
            Request::LoadImage { path, max_side } => {
                let result = sunipdf_core::signature_image::load(&path, max_side);
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
                // Une lecture impossible renvoie un formulaire vide (pas de boucle de relectures).
                Response::FormInfo(doc.form_info().unwrap_or_else(|e| {
                    log::warn!("lecture du formulaire impossible : {e}");
                    FormInfo::default()
                }))
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
                match doc.save_as(&path) {
                    Ok(()) => {
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
    use sunipdf_core::PixelRect;

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
        worker.send(Request::Open {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
            password: None,
        });
        assert!(matches!(
            worker.rx.recv_timeout(timeout).unwrap(),
            Response::Opened(_)
        ));
        for (page, width_px) in [(usize::MAX, 144), (0, u32::MAX)] {
            worker.send(Request::Thumbnail { page, width_px });
            assert!(matches!(
                worker.rx.recv_timeout(timeout).expect("failed thumbnails must release their request quota"),
                Response::ThumbnailFailed { page: failed_page, width_px: failed_width }
                    if failed_page == page && failed_width == width_px
            ));
        }
        encrypted_recovery_after_redaction(&worker);
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
        worker.send(Request::Redact {
            marks: marks.clone(),
            options: RedactOptions::default(),
        });
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

        // Force the first atomic write to fail while leaving the stale PDF deletable.
        let failed_slot = session.slot(2, original.clone(), 2);
        failed_slot.save(&bytes).unwrap();
        let stale = failed_slot.base.with_extension("pdf");
        let blocked = stale.with_file_name(format!(
            "{}.{}.tmp",
            stale.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ));
        std::fs::create_dir(blocked).unwrap();
        worker.send(Request::Open {
            path: original.clone(),
            password: Some("recovery-password".into()),
        });
        assert!(matches!(next_response(worker), Response::Opened(_)));
        worker.send(Request::SetRecovery(Some(failed_slot.clone())));
        worker.send(Request::Redact {
            marks,
            options: RedactOptions::default(),
        });
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
        assert!(!stale.with_extension("json").exists());
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
