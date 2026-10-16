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
    /// Caractères absents de la police (liste dédoublonnée, sans espaces).
    #[error("font does not contain every requested character: {0:?}")]
    MissingGlyphs(String),
    /// AES-128 (R4) : seul un mot de passe Latin-1 imprimable fait l'aller-retour lopdf → PDFium.
    #[error("AES-128 passwords accept Latin-1 characters only; use AES-256")]
    PasswordNotLatin1,
    #[error("font cannot be embedded (licence, format or size)")]
    FontNotEmbeddable,
    #[error("form field not found: {0}")]
    FormFieldNotFound(String),
    /// Lecture seule, type de valeur, longueur, option inconnue (nom du champ puis raison, en anglais).
    #[error("invalid form value: {0}")]
    FormValueInvalid(String),
    #[error("XFA forms are not supported")]
    FormXfa,
    /// Caviardage : flux de contenu non décodable strictement, ou texte encore extractible sous une zone.
    #[error("redaction could not be verified; the document was left unchanged")]
    RedactionIncomplete,
    #[error("operation cancelled")]
    Cancelled,
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
            Self::MissingGlyphs(_) => "missing_glyphs",
            Self::PasswordNotLatin1 => "password_not_latin1",
            Self::FontNotEmbeddable => "font_not_embeddable",
            Self::FormFieldNotFound(_) => "form_field_not_found",
            Self::FormValueInvalid(_) => "form_value_invalid",
            Self::FormXfa => "form_xfa",
            Self::RedactionIncomplete => "redaction_incomplete",
            Self::Cancelled => "cancelled",
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
