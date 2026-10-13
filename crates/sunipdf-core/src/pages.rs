//! Organisation des pages : fusion, extraction, division, rotation, enregistrement atomique.
//!
//! Toute écriture passe par [`write_atomic`] : fichier temporaire voisin puis renommage.
//! Un fichier de sortie n'est donc jamais tronqué, et l'original n'est jamais écrasé par un
//! enregistrement interrompu.

use crate::{Document, Engine, EngineError, PageSize};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    pub fn degrees(self) -> u16 {
        match self {
            Self::R0 => 0,
            Self::R90 => 90,
            Self::R180 => 180,
            Self::R270 => 270,
        }
    }

    pub fn from_degrees(deg: i32) -> Rotation {
        match deg.rem_euclid(360) {
            90 => Self::R90,
            180 => Self::R180,
            270 => Self::R270,
            _ => Self::R0,
        }
    }

    /// Rotation relative (quart de tour horaire = +90).
    pub fn turned(self, delta_degrees: i32) -> Rotation {
        Self::from_degrees(self.degrees() as i32 + delta_degrees)
    }

    fn to_pdfium(self) -> PdfPageRenderRotation {
        match self {
            Self::R0 => PdfPageRenderRotation::None,
            Self::R90 => PdfPageRenderRotation::Degrees90,
            Self::R180 => PdfPageRenderRotation::Degrees180,
            Self::R270 => PdfPageRenderRotation::Degrees270,
        }
    }

    fn from_pdfium(r: PdfPageRenderRotation) -> Self {
        match r {
            PdfPageRenderRotation::None => Self::R0,
            PdfPageRenderRotation::Degrees90 => Self::R90,
            PdfPageRenderRotation::Degrees180 => Self::R180,
            PdfPageRenderRotation::Degrees270 => Self::R270,
        }
    }
}

/// Écrit `bytes` dans `path` sans jamais laisser un fichier partiel : `.tmp` voisin puis `rename`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp_name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    let result = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Chaîne de pages PDFium ("1,3-5", base 1) à partir d'indices base 0.
fn page_range_string(pages: &[usize]) -> String {
    pages
        .iter()
        .map(|p| (p + 1).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

impl Engine {
    /// Fusionne `inputs` (dans l'ordre) dans `output`. Renvoie le nombre de pages écrites.
    pub fn merge(&self, inputs: &[PathBuf], output: &Path) -> Result<usize, EngineError> {
        if inputs.is_empty() {
            return Err(EngineError::Other("no input documents".into()));
        }
        let mut target = self.pdfium.create_new_pdf()?;
        for input in inputs {
            let source = self.pdfium.load_pdf_from_file(input, None)?;
            target.pages_mut().append(&source)?;
        }
        let count = target.pages().len() as usize;
        write_atomic(output, &target.save_to_bytes()?)?;
        Ok(count)
    }

    /// Divise `input` en fichiers de `chunk` pages, nommés `<stem>-<n>.pdf` dans `output_dir`.
    pub fn split(
        &self,
        input: &Path,
        password: Option<&str>,
        chunk: usize,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, EngineError> {
        if chunk == 0 {
            return Err(EngineError::Other("chunk size must be at least 1".into()));
        }
        let doc = self.open(input, password)?;
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "document".into());
        let mut outputs = Vec::new();
        for (n, start) in (0..doc.page_count()).step_by(chunk).enumerate() {
            let pages: Vec<usize> = (start..(start + chunk).min(doc.page_count())).collect();
            let out = output_dir.join(format!("{stem}-{}.pdf", n + 1));
            doc.extract_to(&pages, &out)?;
            outputs.push(out);
        }
        Ok(outputs)
    }
}

impl Document<'_> {
    /// Extrait les pages (indices base 0, ordre conservé, doublons permis) vers `output`.
    pub fn extract_to(&self, pages: &[usize], output: &Path) -> Result<usize, EngineError> {
        if pages.is_empty() {
            return Err(EngineError::Other("no pages selected".into()));
        }
        if let Some(&bad) = pages.iter().find(|&&p| p >= self.page_count()) {
            return Err(EngineError::PageOutOfRange(bad));
        }
        let mut target = self.pdfium().create_new_pdf()?;
        target
            .pages_mut()
            .copy_pages_from_document(&self.inner, &page_range_string(pages), 0)?;
        let count = target.pages().len() as usize;
        write_atomic(output, &target.save_to_bytes()?)?;
        Ok(count)
    }

    pub fn page_rotation(&self, page_index: usize) -> Result<Rotation, EngineError> {
        let page = self.page_handle(page_index)?;
        Ok(Rotation::from_pdfium(page.rotation()?))
    }

    /// Fixe la rotation d'affichage d'une page (attribut /Rotate). Modification en mémoire
    /// jusqu'à [`Document::save_as`].
    pub fn set_page_rotation(
        &mut self,
        page_index: usize,
        rotation: Rotation,
    ) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        page.set_rotation(rotation.to_pdfium());
        drop(page);
        self.refresh_page_sizes()?;
        Ok(())
    }

    /// Supprime une page. Le document doit en conserver au moins une.
    pub fn delete_page(&mut self, page_index: usize) -> Result<(), EngineError> {
        if self.page_count() <= 1 {
            return Err(EngineError::Other(
                "a document must keep at least one page".into(),
            ));
        }
        let page = self.page_handle(page_index)?;
        page.delete()?;
        self.refresh_page_sizes()?;
        Ok(())
    }

    /// Enregistre l'état courant (atomiquement) dans `output`. Le chemin peut être l'original.
    pub fn save_as(&self, output: &Path) -> Result<(), EngineError> {
        let bytes = self.inner.save_to_bytes()?;
        write_atomic(output, &bytes)?;
        Ok(())
    }

    fn page_handle(&self, page_index: usize) -> Result<PdfPage<'_>, EngineError> {
        if page_index >= self.page_count() {
            return Err(EngineError::PageOutOfRange(page_index));
        }
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        Ok(self.inner.pages().get(index)?)
    }

    fn refresh_page_sizes(&mut self) -> Result<(), EngineError> {
        self.info.page_sizes = self
            .inner
            .pages()
            .page_sizes()?
            .iter()
            .map(|r| PageSize {
                width_pt: r.width().value,
                height_pt: r.height().value,
            })
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_arithmetic_wraps() {
        assert_eq!(Rotation::R0.turned(90), Rotation::R90);
        assert_eq!(Rotation::R270.turned(90), Rotation::R0);
        assert_eq!(Rotation::R0.turned(-90), Rotation::R270);
        assert_eq!(Rotation::from_degrees(450), Rotation::R90);
        assert_eq!(page_range_string(&[0, 2, 3]), "1,3,4");
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("sunipdf-atomic-{}", std::process::id()));
        let path = dir.join("out.pdf");
        write_atomic(&path, b"v1").unwrap();
        write_atomic(&path, b"v2").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"v2");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "aucun .tmp résiduel"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
