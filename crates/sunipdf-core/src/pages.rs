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

    pub(crate) fn to_pdfium(self) -> PdfPageRenderRotation {
        match self {
            Self::R0 => PdfPageRenderRotation::None,
            Self::R90 => PdfPageRenderRotation::Degrees90,
            Self::R180 => PdfPageRenderRotation::Degrees180,
            Self::R270 => PdfPageRenderRotation::Degrees270,
        }
    }

    pub(crate) fn from_pdfium(r: PdfPageRenderRotation) -> Self {
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
    atomic_write_with(path, |file| {
        use std::io::Write;
        file.write_all(bytes)
    })
}

fn atomic_write_with(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let mut temporary = tempfile::NamedTempFile::new_in(dir.unwrap_or_else(|| Path::new(".")))?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    // Use the platform rename after closing the temporary handle, including an open PDF on Windows.
    let temporary = temporary.into_temp_path();
    std::fs::rename(&temporary, path)
}

/// Atomically publishes a new copy without replacing a file created in the meantime.
pub fn write_new_copy(path: &Path, bytes: &[u8]) -> Result<(), EngineError> {
    use std::io::Write;
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|failure| {
        if failure.error.kind() == std::io::ErrorKind::AlreadyExists {
            EngineError::AlreadyExists(path.into())
        } else {
            EngineError::Io(failure.error)
        }
    })?;
    Ok(())
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
        overwrite: bool,
    ) -> Result<Vec<PathBuf>, EngineError> {
        let doc = self.open(input, password)?;
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "document".into());
        doc.split_to(chunk, output_dir, &stem, overwrite)
    }
}

/// Erreur [`EngineError::AlreadyExists`] si l'un des chemins existe (aucun écrasement silencieux).
pub fn ensure_absent(paths: &[PathBuf]) -> Result<(), EngineError> {
    match paths.iter().find(|p| p.exists()) {
        Some(p) => Err(EngineError::AlreadyExists(p.clone())),
        None => Ok(()),
    }
}

impl Document<'_> {
    /// Divise le document (état courant, y compris non enregistré) en fichiers de `chunk` pages
    /// nommés `<stem>-<n>.pdf`. Sans `overwrite`, échoue avant toute écriture si l'un existe.
    pub fn split_to(
        &self,
        chunk: usize,
        output_dir: &Path,
        stem: &str,
        overwrite: bool,
    ) -> Result<Vec<PathBuf>, EngineError> {
        if chunk == 0 {
            return Err(EngineError::Other("chunk size must be at least 1".into()));
        }
        std::fs::create_dir_all(output_dir)?;
        let count = self.page_count();
        let parts: Vec<(Vec<usize>, PathBuf)> = (0..count)
            .step_by(chunk)
            .enumerate()
            .map(|(n, start)| {
                (
                    (start..(start + chunk).min(count)).collect(),
                    output_dir.join(format!("{stem}-{}.pdf", n + 1)),
                )
            })
            .collect();
        if !overwrite {
            ensure_absent(&parts.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>())?;
        }
        let mut outputs = Vec::new();
        for (pages, out) in parts {
            self.extract_to(&pages, &out)?;
            outputs.push(out);
        }
        Ok(outputs)
    }

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
        if page_index >= self.page_count() {
            return Err(EngineError::PageOutOfRange(page_index));
        }
        if let Some(rotation) = self.page_rotations.get(page_index).copied().flatten() {
            return Ok(rotation);
        }
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
        let size = PageSize {
            width_pt: page.width().value,
            height_pt: page.height().value,
        };
        drop(page);
        self.info.page_sizes[page_index] = size;
        self.page_rotations[page_index] = Some(rotation);
        Ok(())
    }

    /// Supprime une page. Le document doit en conserver au moins une.
    pub fn delete_page(&mut self, page_index: usize) -> Result<(), EngineError> {
        self.delete_pages(&[page_index])
    }

    pub fn delete_pages(&mut self, pages: &[usize]) -> Result<(), EngineError> {
        let mut pages = pages.to_vec();
        pages.sort_unstable();
        pages.dedup();
        if pages.is_empty() {
            return Ok(());
        }
        if pages.len() >= self.page_count() {
            return Err(EngineError::Other(
                "a document must keep at least one page".into(),
            ));
        }
        if let Some(&page) = pages.iter().find(|&&page| page >= self.page_count()) {
            return Err(EngineError::PageOutOfRange(page));
        }
        for page in pages.into_iter().rev() {
            self.page_handle(page)?.delete()?;
        }
        self.refresh_page_sizes()?;
        Ok(())
    }

    /// Insère des pages d'un autre fichier à la position `at` (`pages` vide = toutes les pages).
    /// Renvoie le nombre de pages insérées.
    pub fn insert_pages_from(
        &mut self,
        source: &Path,
        password: Option<&str>,
        pages: &[usize],
        at: usize,
    ) -> Result<usize, EngineError> {
        if at > self.page_count() {
            return Err(EngineError::PageOutOfRange(at));
        }
        let src = self.pdfium.load_pdf_from_file(source, password)?;
        let src_count = src.pages().len() as usize;
        let selection: Vec<usize> = if pages.is_empty() {
            (0..src_count).collect()
        } else {
            pages.to_vec()
        };
        if let Some(&bad) = selection.iter().find(|&&p| p >= src_count) {
            return Err(EngineError::PageOutOfRange(bad));
        }
        if selection.is_empty() {
            return Err(EngineError::Other("no pages selected".into()));
        }
        let index = PdfPageIndex::try_from(at).map_err(|_| EngineError::PageOutOfRange(at))?;
        self.inner.pages_mut().copy_pages_from_document(
            &src,
            &page_range_string(&selection),
            index,
        )?;
        self.refresh_page_sizes()?;
        Ok(selection.len())
    }

    /// Déplace la page `from` pour qu'elle occupe l'index `to` (indices avant déplacement).
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), EngineError> {
        let count = self.page_count();
        if from >= count {
            return Err(EngineError::PageOutOfRange(from));
        }
        if to >= count {
            return Err(EngineError::PageOutOfRange(to));
        }
        if from == to {
            return Ok(());
        }
        let mut pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let mut pages: Vec<_> = pdf.get_pages().into_values().collect();
        if pages.len() != count {
            return Err(EngineError::InvalidFormat);
        }
        let root = pdf
            .catalog()
            .and_then(|c| c.get(b"Pages"))
            .and_then(lopdf::Object::as_reference)
            .map_err(crate::security::lopdf_err)?;
        // Aplatir l'arbre exige de conserver les attributs hérités avant de changer /Parent.
        for &page_id in &pages {
            let mut inherited = lopdf::Dictionary::new();
            let mut node = Some(page_id);
            let mut seen = std::collections::BTreeSet::new();
            while let Some(id) = node {
                if !seen.insert(id) {
                    return Err(EngineError::InvalidFormat);
                }
                let dict = pdf.get_dictionary(id).map_err(crate::security::lopdf_err)?;
                for key in [b"Resources".as_slice(), b"MediaBox", b"CropBox", b"Rotate"] {
                    if !inherited.has(key) {
                        if let Ok(value) = dict.get(key) {
                            inherited.set(key, value.clone());
                        }
                    }
                }
                node = match dict.get(b"Parent") {
                    Ok(parent) => Some(parent.as_reference().map_err(crate::security::lopdf_err)?),
                    Err(_) => None,
                };
            }
            let page = pdf
                .get_dictionary_mut(page_id)
                .map_err(crate::security::lopdf_err)?;
            for (key, value) in inherited {
                page.set(key, value);
            }
            page.set("Parent", root);
        }
        let moved = pages.remove(from);
        pages.insert(to, moved);
        let tree = pdf
            .get_dictionary_mut(root)
            .map_err(crate::security::lopdf_err)?;
        tree.set(
            "Kids",
            pages
                .into_iter()
                .map(lopdf::Object::Reference)
                .collect::<Vec<_>>(),
        );
        tree.set("Count", count as i64);
        self.restore(crate::security::save(pdf)?)
    }

    /// Instantané complet du document (octets PDF), base de l'annulation.
    pub fn snapshot(&self) -> Result<Vec<u8>, EngineError> {
        Ok(self.inner.save_to_bytes()?)
    }

    /// Mot de passe utilisé à l'ouverture (ou défini par une protection appliquée en mémoire).
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Recharge depuis un instantané en changeant le mot de passe mémorisé (protection/déprotection).
    pub fn restore_with_password(
        &mut self,
        bytes: Vec<u8>,
        password: Option<String>,
    ) -> Result<(), EngineError> {
        self.password = password;
        self.restore(bytes)
    }

    /// Recharge le document depuis un instantané produit par [`Document::snapshot`].
    pub fn restore(&mut self, bytes: Vec<u8>) -> Result<(), EngineError> {
        let inner = self
            .pdfium
            .load_pdf_from_byte_vec(bytes, self.password.as_deref())?;
        self.inner = inner;
        // Les jetons de police appartenaient à l'ancien document.
        self.font_cache.clear();
        self.refresh_page_sizes()?;
        Ok(())
    }

    /// Enregistre l'état courant (atomiquement) dans `output`. Le chemin peut être l'original.
    pub fn save_as(&self, output: &Path) -> Result<(), EngineError> {
        let bytes = self.inner.save_to_bytes()?;
        write_atomic(output, &bytes)?;
        Ok(())
    }

    pub(crate) fn page_handle(&self, page_index: usize) -> Result<PdfPage<'_>, EngineError> {
        if page_index >= self.page_count() {
            return Err(EngineError::PageOutOfRange(page_index));
        }
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        Ok(self.inner.pages().get(index)?)
    }

    pub(crate) fn refresh_page_sizes(&mut self) -> Result<(), EngineError> {
        self.info.form = crate::forms::kind_of(&self.inner);
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
        (self.info.page_labels, self.page_rotations) =
            crate::page_labels_and_rotations(&self.inner);
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

    #[test]
    fn interrupted_full_disk_and_failed_publish_keep_the_original() {
        use std::io::{ErrorKind, Write};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.pdf");
        std::fs::write(&path, b"original saved bytes").unwrap();
        for kind in [
            ErrorKind::StorageFull,
            ErrorKind::Interrupted,
            ErrorKind::PermissionDenied,
        ] {
            let error = atomic_write_with(&path, |file| {
                file.write_all(b"partial new revision")?;
                Err(std::io::Error::new(kind, "injected write failure"))
            })
            .unwrap_err();
            assert_eq!(error.kind(), kind);
            assert_eq!(std::fs::read(&path).unwrap(), b"original saved bytes");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        }
        let blocked = dir.path().join("directory.pdf");
        std::fs::create_dir(&blocked).unwrap();
        assert!(write_atomic(&blocked, b"new").is_err());
        assert!(blocked.is_dir());
        assert_eq!(std::fs::read(&path).unwrap(), b"original saved bytes");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    #[ignore = "Child process used by killed_save_process_preserves_original; requires its isolated test target."]
    fn atomic_write_crash_child() {
        use std::io::Write;
        let path = PathBuf::from(
            std::env::var_os("SUNIPDF_ATOMIC_TEST_TARGET").expect("isolated child target"),
        );
        atomic_write_with(&path, |file| {
            file.write_all(&vec![42; 1024 * 1024])?;
            file.sync_all()?;
            std::fs::write(path.with_extension("ready"), b"partial write ready")?;
            std::thread::sleep(std::time::Duration::from_secs(60));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn killed_save_process_preserves_original() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.pdf");
        std::fs::write(&path, b"original complete revision").unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "pages::tests::atomic_write_crash_child",
                "--ignored",
            ])
            .env("SUNIPDF_ATOMIC_TEST_TARGET", &path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !path.with_extension("ready").exists() {
            if child.try_wait().unwrap().is_some() {
                panic!("save child exited before partial write");
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("save child timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"original complete revision");
        assert!(
            std::fs::read_dir(directory.path()).unwrap().count() >= 2,
            "temporary data never replaced the original"
        );
    }
}
