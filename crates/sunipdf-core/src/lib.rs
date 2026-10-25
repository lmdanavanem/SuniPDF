//! Moteur documentaire SuniPDF.
//!
//! Façade unique au-dessus de PDFium (via `pdfium-render`). L'interface graphique,
//! la CLI et les tests passent tous par ce crate ; aucune logique PDF ne vit ailleurs.
//!
//! PDFium n'est pas thread-safe : un [`Engine`] et ses [`Document`]s doivent vivre
//! sur un seul thread (le crate `pdfium-render` sérialise les appels, mais sans gain
//! de parallélisme). L'application de bureau les confine dans un thread de travail.

pub mod annot;
pub mod annot_rich;
pub mod annot_write;
pub mod annotation_library;

#[cfg(test)]
pub(crate) fn test_engine() -> std::sync::MutexGuard<'static, Engine> {
    static ENGINE: std::sync::OnceLock<std::sync::Mutex<Engine>> = std::sync::OnceLock::new();
    ENGINE
        .get_or_init(|| std::sync::Mutex::new(Engine::new().expect("native PDF engine")))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
pub mod annotation_register;
pub mod annotation_register_batch;
mod appearance;
pub mod bookmarks;
pub mod boundary_fill;
pub mod boxes;
pub mod cache;
pub mod certificate;
pub mod clipboard;
pub mod command;
pub mod compress;
pub mod convert;
pub mod edit;
mod error;
pub mod export_office;
pub mod file_io;
mod font_embed;
pub mod fonts;
pub mod form_data;
mod form_fdf;
pub mod forms;
pub mod geometry;
pub mod header_footer;
mod image_stamp;
pub mod image_watermark;
pub mod inspection;
pub mod issue_report;
mod issue_report_render;
mod link_destination;
pub mod links;
pub mod local_review;
pub mod measurement;
mod measurement_area;
mod measurement_batch;
mod measurement_distance;
mod measurement_ellipses;
mod measurement_shapes;
mod measurement_snap;
mod measurement_viewports;
pub mod ocr;
pub mod office;
mod page_calibration;
pub mod page_labels;
pub mod pages;
pub mod paragraph;
pub mod plan_links;
pub mod plan_set;
pub mod plan_zones;
pub mod quantity_table;
pub mod quantity_workbook;
mod quantity_workbook_render;
pub mod redact;
pub mod register_presets;
pub mod revision_outlines;
pub mod revision_overlay;
pub mod revision_transfer;
pub mod sanitize;
pub mod security;
pub mod signature_image;
pub mod stamp;
mod structure;
pub mod text;
pub mod tiles;
pub mod visual_search;

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
pub use links::{is_safe_external_uri, Destination, DestinationView, LinkInfo};
pub use measurement::{MeasurementRecord, MeasurementTarget, PageCalibration};
pub use page_labels::{PageLabelRange, PageLabelStyle};
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
    pub page_labels: Vec<Option<String>>,
    /// Formulaire du document (`None` pour la plupart des PDF) ; mis à jour à chaque rechargement.
    pub form: FormKind,
}

impl DocumentInfo {
    pub fn page_count(&self) -> usize {
        self.page_sizes.len()
    }

    pub fn page_label(&self, page: usize) -> Option<&str> {
        self.page_labels.get(page).and_then(Option::as_deref)
    }

    /// Exact unique labels take precedence; `#N` always means the physical one-based index.
    pub fn resolve_page_input(&self, input: &str) -> Option<usize> {
        let physical = |number: &str| {
            number
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1))
                .filter(|page| *page < self.page_count())
        };
        if let Some(number) = input.trim().strip_prefix('#') {
            return physical(number);
        }
        let mut matches = self
            .page_labels
            .iter()
            .enumerate()
            .filter_map(|(page, label)| {
                (label.as_deref() == Some(input) && page < self.page_count()).then_some(page)
            });
        match (matches.next(), matches.next()) {
            (Some(page), None) => Some(page),
            (Some(_), Some(_)) => None,
            _ => physical(input),
        }
    }

    pub fn page_input(&self, page: usize) -> String {
        if let Some(label) = self.page_label(page) {
            if self.resolve_page_input(label) == Some(page) {
                return label.to_owned();
            }
        }
        let number = (page + 1).to_string();
        if self.resolve_page_input(&number) == Some(page) {
            number
        } else {
            format!("#{number}")
        }
    }
}

fn page_labels_and_rotations(
    inner: &PdfDocument<'_>,
) -> (Vec<Option<String>>, Vec<Option<Rotation>>) {
    let pages = inner.pages();
    (0..pages.len())
        .map(|index| {
            // Optional labels must not prevent opening a document with a damaged page.
            let Ok(page) = pages.get(index) else {
                return (None, None);
            };
            let label = page
                .label()
                .filter(|label| {
                    label.len() <= 4096
                        && !label.trim().is_empty()
                        && !label.chars().any(char::is_control)
                })
                .map(str::to_owned);
            (label, page.rotation().ok().map(Rotation::from_pdfium))
        })
        .unzip()
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
    page_rotations: Vec<Option<Rotation>>,
    /// Mot de passe d'ouverture, réutilisé pour recharger un instantané (undo/redo).
    password: Option<String>,
    /// Polices système déjà incorporées dans ce document (vidé à chaque rechargement).
    font_cache: std::collections::HashMap<PathBuf, Vec<PdfFontToken>>,
    /// Rendering may generate form appearances. Cache only the need for an isolated
    /// copy, never rendered PDF bytes; edits must always render the current document.
    form_render_need_appearances: std::cell::Cell<Option<bool>>,
}

/// Forme SASLprep du mot de passe si elle diffère de la saisie (sinon rien à réessayer).
fn normalized_fallback(password: Option<&str>) -> Option<String> {
    let raw = password?;
    let norm = security::normalized_password(raw);
    (norm != raw).then_some(norm)
}

/// Directories searched for the platform PDFium library, in order.
pub fn default_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os("SUNIPDF_PDFIUM_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    // Confort de développement seulement : en version publiée, PDFium est cherché à côté de
    // l'exécutable, jamais dans une variable d'environnement, le répertoire courant ni dans un
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
        let platform = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else {
            "win"
        };
        dirs.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../third_party/pdfium")
                .join(format!("{platform}-{arch}"))
                .join(if cfg!(windows) { "bin" } else { "lib" }),
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
        let (page_labels, page_rotations) = page_labels_and_rotations(&inner);
        Ok(Document {
            inner,
            page_rotations,
            pdfium: &self.pdfium,
            password: password.map(str::to_owned),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
            info: DocumentInfo {
                path: path.to_path_buf(),
                title,
                page_sizes,
                page_labels,
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
        self.render_page_impl(page_index, scale, true)
    }

    fn render_page_impl(
        &self,
        page_index: usize,
        scale: f32,
        isolate_form_appearances: bool,
    ) -> Result<RenderedPage, EngineError> {
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
        let render_copy = if isolate_form_appearances {
            self.form_render_copy()?
        } else {
            None
        };
        let render_document = render_copy.as_ref().unwrap_or(&self.inner);
        let page = render_document.pages().get(index)?;
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
        self.render_region_impl(page_index, scale, region, true)
    }

    pub(crate) fn render_region_impl(
        &self,
        page_index: usize,
        scale: f32,
        region: PixelRect,
        include_annotations: bool,
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
        // PDFium's no-form matrix path ignores set_origin(). For content-only
        // previews, render the bounded whole page and crop its exact pixels below.
        // The ordinary reader keeps its existing tiled, origin-based path.
        if !include_annotations
            && (page_w > visual_search::MAX_PAGE_SIDE
                || page_h > visual_search::MAX_PAGE_SIDE
                || u64::from(page_w) * u64::from(page_h) > visual_search::MAX_PAGE_PIXELS)
        {
            return Err(EngineError::RenderTooLarge);
        }
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        let render_copy = if include_annotations {
            self.form_render_copy()?
        } else {
            None
        };
        let render_document = render_copy.as_ref().unwrap_or(&self.inner);
        let page = render_document.pages().get(index)?;
        // Taille cible = page entière ; l'origine négative place la région en (0, 0) du bitmap.
        let config = PdfRenderConfig::new()
            .set_target_size(page_w as Pixels, page_h as Pixels)
            .set_origin(
                if include_annotations {
                    -(region.x as Pixels)
                } else {
                    0
                },
                if include_annotations {
                    -(region.y as Pixels)
                } else {
                    0
                },
            )
            .render_form_data(include_annotations)
            .render_annotations(include_annotations);
        let mut bitmap = PdfBitmap::empty(
            if include_annotations {
                region.width
            } else {
                page_w
            } as Pixels,
            if include_annotations {
                region.height
            } else {
                page_h
            } as Pixels,
            PdfBitmapFormat::default(),
        )?;
        page.render_into_bitmap_with_config(&mut bitmap, &config)?;
        let mut rgba = bitmap.as_rgba_bytes();
        if !include_annotations && region != PixelRect::new(0, 0, page_w, page_h) {
            let mut cropped =
                Vec::with_capacity(region.width as usize * region.height as usize * 4);
            for row in region.y..region.bottom() {
                let start = (row as usize * page_w as usize + region.x as usize) * 4;
                cropped.extend_from_slice(&rgba[start..start + region.width as usize * 4]);
            }
            rgba = cropped;
        }
        Ok(RenderedPage {
            page_index,
            width: region.width,
            height: region.height,
            rgba,
        })
    }
}

#[cfg(test)]
mod document_info_tests {
    use super::*;

    #[test]
    fn page_input_resolves_exact_unique_labels_and_explicit_physical_indices() {
        let mut info = DocumentInfo {
            path: PathBuf::new(),
            title: None,
            page_sizes: vec![
                PageSize {
                    width_pt: 612.0,
                    height_pt: 792.0
                };
                9
            ],
            page_labels: [
                Some("i"),
                Some("ii"),
                Some("1"),
                Some("2"),
                Some("AA"),
                Some("Annexe-É7"),
                Some("Repeat"),
                Some("Repeat"),
                None,
            ]
            .into_iter()
            .map(|label| label.map(str::to_owned))
            .collect(),
            form: FormKind::None,
        };
        for (input, page) in [
            ("i", 0),
            ("ii", 1),
            ("1", 2),
            ("2", 3),
            ("AA", 4),
            ("Annexe-É7", 5),
            ("#1", 0),
            ("#2", 1),
            ("9", 8),
        ] {
            assert_eq!(info.resolve_page_input(input), Some(page), "{input}");
        }
        for input in [
            "",
            "0",
            "#0",
            "#10",
            "10",
            "Repeat",
            "II",
            "missing",
            "1.5",
            "-1",
            "999999999999999999999999999",
        ] {
            assert_eq!(info.resolve_page_input(input), None, "{input}");
        }
        for page in 0..info.page_count() {
            assert_eq!(info.resolve_page_input(&info.page_input(page)), Some(page));
        }
        info.page_labels[3] = Some("1".to_owned());
        assert_eq!(
            info.resolve_page_input("1"),
            None,
            "duplicate numeric labels must not fall back to a physical index"
        );
        assert_eq!(info.resolve_page_input("#3"), Some(2));
        info.page_labels[0] = Some(" i ".to_owned());
        assert_eq!(info.resolve_page_input(" i "), Some(0));
        assert_eq!(info.page_input(0), " i ");
        info.page_labels.clear();
        assert_eq!(info.page_label(0), None);
        assert_eq!(info.page_input(0), "1");
        assert_eq!(info.resolve_page_input("2"), Some(1));
    }
}
