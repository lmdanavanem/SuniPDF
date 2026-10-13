//! Moteur documentaire SuniPDF.
//!
//! Façade unique au-dessus de PDFium (via `pdfium-render`). L'interface graphique,
//! la CLI et les tests passent tous par ce crate ; aucune logique PDF ne vit ailleurs.
//!
//! PDFium n'est pas thread-safe : un [`Engine`] et ses [`Document`]s doivent vivre
//! sur un seul thread (le crate `pdfium-render` sérialise les appels, mais sans gain
//! de parallélisme). L'application de bureau les confine dans un thread de travail.

pub mod annot;
pub mod cache;
pub mod command;
pub mod convert;
mod error;
pub mod pages;
pub mod security;
pub mod text;

pub use annot::{AnnotationInfo, MarkupKind, TextSelection};
pub use cache::{cache_key, RenderCache};
pub use command::{Command, CommandInfo, Menu, REGISTRY};
pub use convert::Optimized;
pub use error::EngineError;
pub use pages::{write_atomic, Rotation};
pub use security::{Bookmark, Metadata, Properties, Protection};
pub use text::{PointRect, TextMatch};

use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// Taille maximale (en pixels) d'un côté d'une page rendue. Protège la mémoire.
pub const MAX_RENDER_SIDE_PX: u32 = 16_384;

/// Dimensions d'une page en points PDF (1/72 pouce).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

/// Résultat d'un rendu : pixels RGBA 8 bits, non prémultipliés, ligne par ligne.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub page_index: usize,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Informations statiques d'un document ouvert (copiables vers d'autres threads).
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub path: PathBuf,
    pub title: Option<String>,
    pub page_sizes: Vec<PageSize>,
}

impl DocumentInfo {
    pub fn page_count(&self) -> usize {
        self.page_sizes.len()
    }
}

/// Instance du moteur (bibliothèque PDFium chargée).
pub struct Engine {
    pdfium: Pdfium,
}

/// Document ouvert, lié à la durée de vie de son [`Engine`].
pub struct Document<'a> {
    inner: PdfDocument<'a>,
    pdfium: &'a Pdfium,
    info: DocumentInfo,
    /// Mot de passe d'ouverture, réutilisé pour recharger un instantané (undo/redo).
    password: Option<String>,
}

/// Répertoires explorés pour trouver `pdfium.dll`, dans l'ordre.
pub fn default_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("SUNIPDF_PDFIUM_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }
    // Confort de développement : binaires récupérés par scripts/fetch-pdfium.ps1.
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    dirs.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/pdfium")
            .join(format!("win-{arch}"))
            .join("bin"),
    );
    dirs
}

impl Engine {
    /// Charge PDFium depuis [`default_library_dirs`].
    pub fn new() -> Result<Self, EngineError> {
        Self::with_library_dirs(&default_library_dirs())
    }

    /// Charge PDFium depuis le premier répertoire contenant la bibliothèque.
    pub fn with_library_dirs(dirs: &[PathBuf]) -> Result<Self, EngineError> {
        let mut last_load_error = None;
        for dir in dirs {
            let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
            if !lib.is_file() {
                continue;
            }
            match Pdfium::bind_to_library(&lib) {
                Ok(bindings) => {
                    log::info!("PDFium chargé depuis {}", lib.display());
                    return Ok(Self {
                        pdfium: Pdfium::new(bindings),
                    });
                }
                Err(e) => last_load_error = Some(format!("{}: {e:?}", lib.display())),
            }
        }
        match last_load_error {
            Some(msg) => Err(EngineError::LibraryLoad(msg)),
            None => Err(EngineError::LibraryNotFound(dirs.to_vec())),
        }
    }

    /// Ouvre un document depuis des octets en mémoire (instantané suspendu), en conservant `path`.
    pub fn open_bytes(
        &self,
        bytes: Vec<u8>,
        password: Option<&str>,
        path: &Path,
    ) -> Result<Document<'_>, EngineError> {
        let inner = self.pdfium.load_pdf_from_byte_vec(bytes, password)?;
        self.wrap(inner, path, password)
    }

    /// Ouvre un document. `password` : mot de passe utilisateur ou propriétaire.
    pub fn open(&self, path: &Path, password: Option<&str>) -> Result<Document<'_>, EngineError> {
        let inner = self.pdfium.load_pdf_from_file(path, password)?;
        self.wrap(inner, path, password)
    }

    fn wrap<'a>(
        &'a self,
        inner: PdfDocument<'a>,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Document<'a>, EngineError> {
        let page_sizes = inner
            .pages()
            .page_sizes()?
            .iter()
            .map(|r| PageSize {
                width_pt: r.width().value,
                height_pt: r.height().value,
            })
            .collect();
        let title = inner
            .metadata()
            .get(PdfDocumentMetadataTagType::Title)
            .map(|t| t.value().to_string())
            .filter(|s| !s.trim().is_empty());
        Ok(Document {
            inner,
            pdfium: &self.pdfium,
            password: password.map(str::to_owned),
            info: DocumentInfo {
                path: path.to_path_buf(),
                title,
                page_sizes,
            },
        })
    }
}

impl<'a> Document<'a> {
    pub(crate) fn pdfium(&self) -> &'a Pdfium {
        self.pdfium
    }
}

impl std::fmt::Debug for Document<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl Document<'_> {
    pub fn info(&self) -> &DocumentInfo {
        &self.info
    }

    pub fn page_count(&self) -> usize {
        self.info.page_count()
    }

    pub fn page_size(&self, page_index: usize) -> Result<PageSize, EngineError> {
        self.info
            .page_sizes
            .get(page_index)
            .copied()
            .ok_or(EngineError::PageOutOfRange(page_index))
    }

    /// Rend une page à `scale` pixels par point (1.0 = 72 dpi ; 96/72 = 100 % à l'écran).
    pub fn render_page(&self, page_index: usize, scale: f32) -> Result<RenderedPage, EngineError> {
        let size = self.page_size(page_index)?;
        if !(scale.is_finite() && scale > 0.0) {
            return Err(EngineError::RenderTooLarge);
        }
        let width = (size.width_pt * scale).round() as u32;
        let height = (size.height_pt * scale).round() as u32;
        if width == 0 || height == 0 || width > MAX_RENDER_SIDE_PX || height > MAX_RENDER_SIDE_PX {
            return Err(EngineError::RenderTooLarge);
        }
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        let page = self.inner.pages().get(index)?;
        let config = PdfRenderConfig::new()
            .set_target_size(width as Pixels, height as Pixels)
            .render_form_data(true)
            .render_annotations(true);
        let bitmap = page.render_with_config(&config)?;
        Ok(RenderedPage {
            page_index,
            width: bitmap.width() as u32,
            height: bitmap.height() as u32,
            rgba: bitmap.as_rgba_bytes(),
        })
    }
}
