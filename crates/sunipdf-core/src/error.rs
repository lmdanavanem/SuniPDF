use pdfium_render::prelude::{PdfiumError, PdfiumInternalError};
use std::path::PathBuf;

/// Erreurs typées du moteur. L'interface les traduit ; le crate n'embarque aucun texte UI.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("PDFium library not found; searched {0:?}")]
    LibraryNotFound(Vec<PathBuf>),
    #[error("PDFium library failed to load: {0}")]
    LibraryLoad(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("password required or incorrect")]
    PasswordRequired,
    #[error("not a valid PDF document")]
    InvalidFormat,
    #[error("document security settings forbid this operation")]
    SecurityRestricted,
    #[error("page index {0} out of range")]
    PageOutOfRange(usize),
    #[error("requested render size exceeds limits")]
    RenderTooLarge,
    #[error("output already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("engine error: {0}")]
    Other(String),
}

impl EngineError {
    /// Code stable, utilisable par la CLI et les tests.
    pub fn code(&self) -> &'static str {
        match self {
            Self::LibraryNotFound(_) => "library_not_found",
            Self::LibraryLoad(_) => "library_load",
            Self::Io(_) => "io",
            Self::PasswordRequired => "password_required",
            Self::InvalidFormat => "invalid_format",
            Self::SecurityRestricted => "security_restricted",
            Self::PageOutOfRange(_) => "page_out_of_range",
            Self::RenderTooLarge => "render_too_large",
            Self::AlreadyExists(_) => "already_exists",
            Self::Other(_) => "other",
        }
    }
}

impl From<PdfiumError> for EngineError {
    fn from(e: PdfiumError) -> Self {
        match e {
            PdfiumError::IoError(io) => Self::Io(io),
            PdfiumError::LoadLibraryError(err) => Self::LibraryLoad(err.to_string()),
            PdfiumError::PdfiumLibraryInternalError(inner) => match inner {
                PdfiumInternalError::PasswordError => Self::PasswordRequired,
                PdfiumInternalError::FormatError => Self::InvalidFormat,
                PdfiumInternalError::SecurityError => Self::SecurityRestricted,
                PdfiumInternalError::FileError => {
                    Self::Io(std::io::Error::other("pdfium file error"))
                }
                other => Self::Other(format!("{other:?}")),
            },
            PdfiumError::PageIndexOutOfBounds => Self::PageOutOfRange(usize::MAX),
            other => Self::Other(format!("{other:?}")),
        }
    }
}
