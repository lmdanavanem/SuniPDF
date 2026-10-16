//! Moteur documentaire SuniPDF.
//!
//! Façade unique au-dessus de PDFium (via `pdfium-render`). L'interface graphique,
//! la CLI et les tests passent tous par ce crate ; aucune logique PDF ne vit ailleurs.
//!
//! PDFium n'est pas thread-safe : un [`Engine`] et ses [`Document`]s doivent vivre
//! sur un seul thread (le crate `pdfium-render` sérialise les appels, mais sans gain
//! de parallélisme). L'application de bureau les confine dans un thread de travail.

pub mod annot;
pub mod annot_write;
mod appearance;
pub mod boxes;
pub mod cache;
pub mod certificate;
pub mod clipboard;
pub mod command;
pub mod compress;
pub mod convert;
pub mod edit;
mod error;
pub mod fonts;
pub mod form_data;
pub mod forms;
pub mod geometry;
pub mod links;
pub mod ocr;
pub mod pages;
pub mod redact;
pub mod security;
pub mod signature_image;
pub mod stamp;
pub mod text;
pub mod tiles;

pub use annot::{AnnotationInfo, MarkupKind, TextSelection, ANNOTATION_KINDS};
pub use annot_write::{
    thread_status, AnnotDetail, AnnotShape, AnnotStyle, FreeTextStyle, LinkTarget, ReviewStatus,
};
pub use appearance::smooth_polyline;
pub use boxes::{PageMargins, ResizeMode};
pub use cache::{cache_key, tile_key, RenderCache};
pub use clipboard::ObjectClip;
pub use command::{Command, CommandInfo, Menu, REGISTRY};
pub use convert::Optimized;
pub use edit::{
    FontFamily, ObjectInfo, ObjectKind, ObjectStyle, RectMode, ShapeKind, ShapeStyle, TextStyle,
    ZOrder,
};
pub use error::EngineError;
pub use fonts::SystemFont;
pub use form_data::ImportPlan;
pub use forms::{
    FieldKind, FieldOption, FieldValue, FieldWidget, FillMode, FillReport, FormField, FormInfo,
    FormKind,
};
pub use geometry::PageGeometry;
pub use links::{is_safe_external_uri, LinkInfo};
pub use pages::{write_atomic, write_new_copy, Rotation};
pub use redact::{RedactMark, RedactOptions, RedactReport};
pub use security::{Bookmark, Metadata, Properties, Protection, ProtectionProblem};
pub use stamp::{Stamp, StampPosition};
pub use text::{PointRect, TextMatch};
pub use tiles::{PixelRect, TileJob, TileLimits};

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
    /// Formulaire du document (`None` pour la plupart des PDF) ; mis à jour à chaque rechargement.
    pub form: FormKind,
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
    /// Polices système déjà incorporées dans ce document (vidé à chaque rechargement).
    font_cache: std::collections::HashMap<PathBuf, PdfFontToken>,
}

/// Forme SASLprep du mot de passe si elle diffère de la saisie (sinon rien à réessayer).
fn normalized_fallback(password: Option<&str>) -> Option<String> {
    let raw = password?;
    let norm = security::normalized_password(raw);
    (norm != raw).then_some(norm)
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
    // Confort de développement seulement : en version publiée, pdfium.dll est cherché à côté de
    // l'exécutable (ou dans SUNIPDF_PDFIUM_DIR), jamais dans le répertoire courant ni dans un
    // chemin de la machine de build.
    #[cfg(debug_assertions)]
    {
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd);
        }
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
    }
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
        let fallback = normalized_fallback(password);
        let retry_bytes = fallback.as_ref().map(|_| bytes.clone());
        match self.pdfium.load_pdf_from_byte_vec(bytes, password) {
            Ok(inner) => self.wrap(inner, path, password),
            Err(e) => match (EngineError::from(e), fallback, retry_bytes) {
                (EngineError::PasswordRequired, Some(norm), Some(bytes)) => {
                    let inner = self.pdfium.load_pdf_from_byte_vec(bytes, Some(&norm))?;
                    self.wrap(inner, path, Some(&norm))
                }
                (err, _, _) => Err(err),
            },
        }
    }

    /// Ouvre un document. `password` : mot de passe utilisateur ou propriétaire. Un mot de passe
    /// AES-256 est réessayé sous sa forme normalisée SASLprep (celle qui a servi au chiffrement).
    pub fn open(&self, path: &Path, password: Option<&str>) -> Result<Document<'_>, EngineError> {
        match self.pdfium.load_pdf_from_file(path, password) {
            Ok(inner) => self.wrap(inner, path, password),
            Err(e) => match (EngineError::from(e), normalized_fallback(password)) {
                (EngineError::PasswordRequired, Some(norm)) => {
                    let inner = self.pdfium.load_pdf_from_file(path, Some(&norm))?;
                    self.wrap(inner, path, Some(&norm))
                }
                (err, _) => Err(err),
            },
        }
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
        let form = forms::kind_of(&inner);
        Ok(Document {
            inner,
            pdfium: &self.pdfium,
            password: password.map(str::to_owned),
            font_cache: Default::default(),
            info: DocumentInfo {
                path: path.to_path_buf(),
                title,
                page_sizes,
                form,
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

    /// Chemin associé au document après « Enregistrer sous » (les enregistrements suivants y vont).
    pub fn set_path(&mut self, path: PathBuf) {
        self.info.path = path;
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

    /// Rend la zone `region` (pixels, origine en haut à gauche de la page rendue à `scale`) sans
    /// allouer la page entière : la vue continue et le fort zoom passent tous par ici.
    pub fn render_region(
        &self,
        page_index: usize,
        scale: f32,
        region: PixelRect,
    ) -> Result<RenderedPage, EngineError> {
        let size = self.page_size(page_index)?;
        let (page_w, page_h) =
            tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
        if region.is_empty() || region.right() > page_w || region.bottom() > page_h {
            return Err(EngineError::Other(
                "render region is empty or outside the page".into(),
            ));
        }
        if region.width > MAX_RENDER_SIDE_PX
            || region.height > MAX_RENDER_SIDE_PX
            || u64::from(region.width) * u64::from(region.height) > tiles::MAX_REGION_PIXELS
        {
            return Err(EngineError::RenderTooLarge);
        }
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        let page = self.inner.pages().get(index)?;
        // Taille cible = page entière ; l'origine négative place la région en (0, 0) du bitmap.
        let config = PdfRenderConfig::new()
            .set_target_size(page_w as Pixels, page_h as Pixels)
            .set_origin(-(region.x as Pixels), -(region.y as Pixels))
            .render_form_data(true)
            .render_annotations(true);
        let mut bitmap = PdfBitmap::empty(
            region.width as Pixels,
            region.height as Pixels,
            PdfBitmapFormat::default(),
        )?;
        page.render_into_bitmap_with_config(&mut bitmap, &config)?;
        Ok(RenderedPage {
            page_index,
            width: bitmap.width() as u32,
            height: bitmap.height() as u32,
            rgba: bitmap.as_rgba_bytes(),
        })
    }
}
