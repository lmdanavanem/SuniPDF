//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.
//! Cache LRU partagé (pages, miniatures), pré-rendu des pages voisines quand la file est vide,
//! journal d'annulation par instantanés (octets PDF) pour toutes les modifications de pages.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter, TryRecvError};
use std::sync::Arc;
use sunipdf_core::{
    security, AnnotationInfo, Bookmark, Document, DocumentInfo, Engine, EngineError, MarkupKind,
    Metadata, ObjectInfo, ObjectStyle, PointRect, Properties, Protection, RenderCache,
    RenderedPage, ShapeKind, ShapeStyle, Stamp, TextMatch, TextSelection, TextStyle,
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
    },
    Extracted {
        out: PathBuf,
        pages: usize,
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
        index: usize,
        style: ObjectStyle,
    },
    MoveObject {
        page: usize,
        index: usize,
        dx: f32,
        dy: f32,
    },
    /// Décalage au clavier : les décalages consécutifs du même objet partagent une annulation.
    NudgeObject {
        page: usize,
        index: usize,
        dx: f32,
        dy: f32,
    },
    DeleteObject {
        page: usize,
        index: usize,
    },
    AddImage {
        page: usize,
        path: PathBuf,
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
    DuplicateObject {
        page: usize,
        index: usize,
    },
    ReorderObject {
        page: usize,
        index: usize,
        front: bool,
    },
    Stamp {
        pages: Option<Vec<usize>>,
        stamp: Stamp,
    },
}

pub enum Request {
    Open {
        path: PathBuf,
        password: Option<String>,
    },
    Render {
        page: usize,
        scale: f32,
        generation: u64,
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
}

pub enum Response {
    EngineReady,
    EngineFailed(EngineError),
    Opened(DocState),
    OpenFailed {
        path: PathBuf,
        error: EngineError,
    },
    Rendered {
        generation: u64,
        image: Arc<RenderedPage>,
    },
    RenderFailed {
        generation: u64,
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
    /// Pages voisines à pré-rendre (échelle courante) quand aucune requête n'attend.
    prefetch: VecDeque<(usize, f32)>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Modifié depuis l'ouverture, la reprise ou le dernier enregistrement.
    modified: bool,
    /// Dernier objet décalé au clavier (page, index).
    last_nudge: Option<(usize, usize)>,
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

    fn schedule_prefetch(&mut self, page: usize, scale: f32) {
        self.prefetch.clear();
        let count = self.document.as_ref().map_or(0, Document::page_count);
        for neighbour in [page + 1, page.wrapping_sub(1)] {
            if neighbour < count && !self.cache.contains(neighbour, scale) {
                self.prefetch.push_back((neighbour, scale));
            }
        }
    }

    fn invalidate(&mut self) {
        self.cache.clear();
        self.prefetch.clear();
    }

    fn doc_state(&self) -> Option<DocState> {
        self.document.as_ref().map(|d| DocState {
            info: d.info().clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            modified: self.modified,
        })
    }

    fn push_undo(&mut self, snapshot: Snapshot) {
        self.undo.push(snapshot);
        self.redo.clear();
        while self.undo.len() > UNDO_MAX_ENTRIES
            || (self.undo.len() > 1
                && self.undo.iter().map(|s| s.bytes.len()).sum::<usize>() > UNDO_MAX_BYTES)
        {
            self.undo.remove(0);
        }
    }

    /// Applique une modification sous instantané. Toute erreur laisse le document intact
    /// (les opérations échouent avant de modifier, ou l'instantané est restauré).
    fn edit(&mut self, op: EditOp) -> Response {
        let nudge = match &op {
            EditOp::NudgeObject { page, index, .. } => Some((*page, *index)),
            _ => None,
        };
        // Décalages clavier consécutifs du même objet : l'instantané pris avant le premier suffit.
        let coalesce = nudge.is_some() && nudge == self.last_nudge && !self.undo.is_empty();
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
        match apply(doc, op) {
            Ok(()) => {
                match snapshot {
                    Some(s) => self.push_undo(s),
                    None => self.redo.clear(),
                }
                self.last_nudge = nudge;
                self.modified = true;
                self.invalidate();
                Response::DocumentChanged(self.doc_state().expect("document ouvert"))
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
        self.modified = true;
        self.invalidate();
        Some(Response::DocumentChanged(self.doc_state()?))
    }
}

fn apply(doc: &mut Document<'_>, op: EditOp) -> Result<(), EngineError> {
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
        } => doc.add_text(page, x, y, &text, &style),
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
        EditOp::SetObjectStyle { page, index, style } => doc.set_object_style(page, index, &style),
        EditOp::MoveObject {
            page,
            index,
            dx,
            dy,
        } => doc.move_object(page, index, dx, dy),
        EditOp::NudgeObject {
            page,
            index,
            dx,
            dy,
        } => doc.move_object(page, index, dx, dy),
        EditOp::DeleteObject { page, index } => doc.delete_object(page, index),
        EditOp::AddImage {
            page,
            path,
            x,
            y,
            width_pt,
        } => doc.add_image(page, &path, x, y, width_pt),
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
        EditOp::DuplicateObject { page, index } => doc.duplicate_object(page, index).map(|_| ()),
        EditOp::ReorderObject { page, index, front } => {
            doc.reorder_object(page, index, front).map(|_| ())
        }
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
            Ok(BatchResult::Split { files })
        }
        BatchOp::Extract { pages, out } => {
            let doc = doc.ok_or(EngineError::Other("no document".into()))?;
            let n = doc.extract_to(&pages, &out)?;
            Ok(BatchResult::Extracted { out, pages: n })
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
        prefetch: VecDeque::new(),
        undo: Vec::new(),
        redo: Vec::new(),
        modified: false,
        last_nudge: None,
    };
    let mut queue: VecDeque<Request> = VecDeque::new();
    loop {
        // 1. Requête suivante : file locale, sinon pré-rendu si rien n'attend, sinon blocage.
        let request = match queue.pop_front() {
            Some(r) => r,
            None => match rx.try_recv() {
                Ok(r) => r,
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => match state.prefetch.pop_front() {
                    Some((page, scale)) => {
                        if let Err(e) = state.render_cached(page, scale) {
                            log::debug!("pré-rendu page {page} ignoré : {e}");
                        }
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
                    state.modified = false;
                    state.last_nudge = None;
                    Response::Opened(state.doc_state().expect("document ouvert"))
                }
                Err(error) => Response::OpenFailed { path, error },
            },
            Request::Render {
                page,
                scale,
                generation,
            } => {
                if state.document.is_none() {
                    continue;
                }
                match state.render_cached(page, scale) {
                    Ok(image) => {
                        state.schedule_prefetch(page, scale);
                        Response::Rendered { generation, image }
                    }
                    Err(error) => Response::RenderFailed { generation, error },
                }
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
                    undo: std::mem::take(&mut state.undo),
                    redo: std::mem::take(&mut state.redo),
                    modified: std::mem::replace(&mut state.modified, false),
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
                        state.modified = s.modified;
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
                        state.modified = false;
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
                state.modified = false;
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
