//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.
//! Cache LRU partagé (tuiles, miniatures), file de tuiles voulues remplacée en bloc par l'UI
//! (`SetWanted`) et rendue une tuile à la fois entre deux requêtes, journal d'annulation par
//! instantanés (octets PDF) pour toutes les modifications de pages.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter, TryRecvError};
use std::sync::Arc;
use sunipdf_core::{
    security, AnnotationInfo, Bookmark, Document, DocumentInfo, Engine, EngineError, MarkupKind,
    Metadata, ObjectClip, ObjectInfo, ObjectStyle, PageMargins, PointRect, Properties, Protection,
    RectMode, RenderCache, RenderedPage, ResizeMode, ShapeKind, ShapeStyle, Stamp, SystemFont,
    TextMatch, TextSelection, TextStyle, TileJob, ZOrder,
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
    Combine { inputs: Vec<PathBuf>, out: PathBuf },
    Split { every: usize, out_dir: PathBuf },
    Extract { pages: Vec<usize>, out: PathBuf },
    CreateFromImages { images: Vec<PathBuf>, out: PathBuf },
    ExportImages { out_dir: PathBuf, dpi: f32 },
    Optimize { out: PathBuf },
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
    Annotations {
        page: usize,
    },
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
}

pub enum Response {
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
    },
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
}

pub struct Worker {
    tx: Sender<Request>,
    rx: Receiver<Response>,
}

impl Worker {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (tx, worker_rx) = mpsc::channel::<Request>();
        let (worker_tx, rx) = mpsc::channel::<Response>();
        std::thread::Builder::new()
            .name("sunipdf-engine".into())
            .spawn(move || run(worker_rx, worker_tx, ctx))
            .expect("thread moteur");
        Self { tx, rx }
    }

    pub fn send(&self, request: Request) {
        // Si le thread est mort, l'UI le constatera via EngineFailed déjà reçu.
        let _ = self.tx.send(request);
    }

    pub fn poll(&self) -> TryIter<'_, Response> {
        self.rx.try_iter()
    }
}

struct State<'a> {
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
        EditOp::Bytes(op) => {
            let current = doc.snapshot()?;
            let password = doc.password().map(str::to_owned);
            match op {
                BytesOp::Protect(p) => {
                    let bytes = security::protect(&current, password.as_deref(), &p)?;
                    let new_password = if p.user_password.is_empty() {
                        p.owner_password
                    } else {
                        p.user_password
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
                let Some(doc) = &state.document else { continue };
                let Ok(size) = doc.page_size(page) else {
                    continue;
                };
                let scale = width_px.max(1) as f32 / size.width_pt;
                match state.render_cached(page, scale) {
                    Ok(image) => Response::Thumbnail { page, image },
                    Err(e) => {
                        log::warn!("miniature page {page} impossible : {e}");
                        continue;
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
            Request::Batch(op) => {
                Response::BatchDone(run_batch(&engine, state.document.as_ref(), op))
            }
            Request::Suspend => {
                let Some(doc) = state.document.take() else {
                    continue;
                };
                state.invalidate();
                let suspended = SuspendedDoc {
                    bytes: match doc.snapshot() {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("suspension impossible : {e}");
                            continue;
                        }
                    },
                    password: doc.password().map(str::to_owned),
                    path: doc.info().path.clone(),
                    modified: state.modified(),
                    undo: std::mem::take(&mut state.undo),
                    redo: std::mem::take(&mut state.redo),
                    save_point: state.save_point.replace(0),
                };
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
            Request::Annotations { page } => {
                let Some(doc) = &state.document else { continue };
                let list = doc.annotations(page).unwrap_or_default();
                Response::Annotations { page, list }
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
                        Response::Saved { path }
                    }
                    Err(e) => Response::SaveFailed(e),
                }
            }
            Request::Close => {
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
