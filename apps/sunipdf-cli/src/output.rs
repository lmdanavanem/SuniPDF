//! Final CLI publication keeps the no-overwrite policy effective after processing finishes.

use std::path::Path;
use sunipdf_core::{Document, EngineError};

pub fn write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), EngineError> {
    if overwrite {
        sunipdf_core::write_atomic(path, bytes)?;
        Ok(())
    } else {
        sunipdf_core::write_new_copy(path, bytes)
    }
}

pub fn save(doc: &Document<'_>, path: &Path, overwrite: bool) -> Result<(), EngineError> {
    write(path, &doc.snapshot()?, overwrite)
}

/// Wrap an existing single-file writer without changing its format or loading its output into RAM.
/// Explicit replacement retains the writer's own path checks and atomic-write behavior.
pub fn staged<T>(
    path: &Path,
    overwrite: bool,
    produce: impl FnOnce(&Path) -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    if overwrite {
        return produce(path);
    }
    if path.exists() {
        return Err(EngineError::AlreadyExists(path.into()));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".sunipdf-output-")
        .tempdir_in(parent)?;
    let staged = temporary
        .path()
        .join(path.file_name().ok_or(EngineError::InvalidFormat)?);
    let result = produce(&staged)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&staged)?;
    file.sync_all()?;
    drop(file);
    tempfile::TempPath::try_from_path(staged)?
        .persist_noclobber(path)
        .map_err(|failure| {
            if failure.error.kind() == std::io::ErrorKind::AlreadyExists {
                EngineError::AlreadyExists(path.into())
            } else {
                EngineError::Io(failure.error)
            }
        })?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_byte_publication_preserves_a_late_file_unless_overwrite_is_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.pdf");
        assert!(!out.exists(), "command preflight saw an unused output");
        std::fs::write(&out, b"created while processing").unwrap();
        assert!(matches!(
            write(&out, b"processed bytes", false),
            Err(EngineError::AlreadyExists(path)) if path == out
        ));
        assert_eq!(std::fs::read(&out).unwrap(), b"created while processing");
        write(&out, b"processed bytes", true).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"processed bytes");
    }

    #[test]
    fn staged_publication_refuses_a_file_created_during_processing_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.pdf");
        let result = staged(&out, false, |path| {
            assert_ne!(path, out.as_path());
            assert_eq!(path.file_name(), out.file_name());
            sunipdf_core::write_atomic(path, b"processed bytes")?;
            std::fs::write(&out, b"concurrent output")?;
            Ok(17)
        });
        assert!(matches!(result, Err(EngineError::AlreadyExists(path)) if path == out));
        assert_eq!(std::fs::read(&out).unwrap(), b"concurrent output");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        let result = staged(&out, true, |path| {
            assert_eq!(path, out.as_path());
            sunipdf_core::write_atomic(path, b"explicit replacement")?;
            Ok(23)
        })
        .unwrap();
        assert_eq!(result, 23);
        assert_eq!(std::fs::read(&out).unwrap(), b"explicit replacement");
    }

    #[test]
    fn failed_staged_generation_does_not_publish_a_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.pdf");
        let result: Result<(), EngineError> = staged(&out, false, |path| {
            std::fs::write(path, b"partial output")?;
            Err(EngineError::InvalidFormat)
        });
        assert!(matches!(result, Err(EngineError::InvalidFormat)));
        assert!(!out.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
