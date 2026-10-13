//! Thread de travail propriétaire du moteur PDF. L'UI n'appelle jamais PDFium directement.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryIter};
use sunipdf_core::{DocumentInfo, Engine, EngineError, RenderedPage};

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
        image: RenderedPage,
    },
    RenderFailed {
        generation: u64,
        error: EngineError,
    },
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
    let mut document = None;
    for request in rx {
        let response = match request {
            Request::Open { path, password } => match engine.open(&path, password.as_deref()) {
                Ok(doc) => {
                    let info = doc.info().clone();
                    document = Some(doc);
                    Response::Opened(info)
                }
                Err(error) => Response::OpenFailed { path, error },
            },
            Request::Render {
                page,
                scale,
                generation,
            } => match &document {
                Some(doc) => match doc.render_page(page, scale) {
                    Ok(image) => Response::Rendered { generation, image },
                    Err(error) => Response::RenderFailed { generation, error },
                },
                None => continue,
            },
            Request::Close => {
                document = None;
                continue;
            }
        };
        if tx.send(response).is_err() {
            break;
        }
        ctx.request_repaint();
    }
}
