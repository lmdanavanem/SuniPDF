//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.
//! Cache LRU partagé (pages, miniatures) et pré-rendu des pages voisines quand la file est vide.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter, TryRecvError};
use std::sync::Arc;
use sunipdf_core::{
    Document, DocumentInfo, Engine, EngineError, RenderCache, RenderedPage, TextMatch,
};

/// Plafond du cache de rendu. ponytail: constante ; réglage utilisateur quand un besoin réel apparaîtra.
const CACHE_BYTES: usize = 256 * 1024 * 1024;

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
    Rotate {
        page: usize,
        delta_degrees: i32,
    },
    DeletePage {
        page: usize,
    },
    Save {
        path: PathBuf,
    },
    Close,
}

pub enum Response {
    EngineReady,
    EngineFailed(EngineError),
    Opened(DocumentInfo),
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
    /// Pages modifiées en mémoire (rotation, suppression) : tailles rafraîchies, rendus invalidés.
    DocumentChanged(DocumentInfo),
    OperationFailed(EngineError),
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
                    let info = doc.info().clone();
                    state.document = Some(doc);
                    state.cache.clear();
                    state.prefetch.clear();
                    Response::Opened(info)
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
                match doc.search(&query, match_case) {
                    Ok(matches) => Response::SearchResults {
                        generation,
                        matches,
                    },
                    Err(e) => {
                        log::warn!("recherche impossible : {e}");
                        Response::SearchResults {
                            generation,
                            matches: Vec::new(),
                        }
                    }
                }
            }
            Request::Rotate {
                page,
                delta_degrees,
            } => {
                let Some(doc) = &mut state.document else {
                    continue;
                };
                let result = doc
                    .page_rotation(page)
                    .and_then(|r| doc.set_page_rotation(page, r.turned(delta_degrees)));
                match result {
                    Ok(()) => {
                        state.cache.clear();
                        state.prefetch.clear();
                        Response::DocumentChanged(doc.info().clone())
                    }
                    Err(e) => Response::OperationFailed(e),
                }
            }
            Request::DeletePage { page } => {
                let Some(doc) = &mut state.document else {
                    continue;
                };
                match doc.delete_page(page) {
                    Ok(()) => {
                        state.cache.clear();
                        state.prefetch.clear();
                        Response::DocumentChanged(doc.info().clone())
                    }
                    Err(e) => Response::OperationFailed(e),
                }
            }
            Request::Save { path } => {
                let Some(doc) = &state.document else { continue };
                match doc.save_as(&path) {
                    Ok(()) => Response::Saved { path },
                    Err(e) => Response::SaveFailed(e),
                }
            }
            Request::Close => {
                state.document = None;
                state.cache.clear();
                state.prefetch.clear();
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
