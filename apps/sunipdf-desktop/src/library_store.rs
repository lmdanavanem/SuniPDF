//! Bounded local storage with explicit reload and concurrent-change protection.

use std::io::Read;
use std::path::Path;
use sunipdf_core::EngineError;

pub(crate) const MAX_BYTES: u64 = 16 * 1024 * 1024;

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn redirected(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn check_path(path: &Path) -> Result<(), EngineError> {
    if path
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(failure(
            "annotation library path contains a parent component",
        ));
    }
    for (index, ancestor) in path.ancestors().enumerate() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata)
                if redirected(&metadata)
                    || (index == 0 && !metadata.is_file())
                    || (index > 0 && !metadata.is_dir()) =>
            {
                return Err(failure(
                    "annotation library path is redirected or has an incompatible file type",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(crate) fn read(path: &Path) -> Result<Option<Vec<u8>>, EngineError> {
    check_path(path)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || redirected(&metadata) || metadata.len() > MAX_BYTES {
        return Err(failure(
            "annotation library is redirected or exceeds 16 MiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(failure("annotation library exceeds 16 MiB"));
    }
    Ok(Some(bytes))
}

pub(crate) fn commit(
    path: &Path,
    expected: Option<&[u8]>,
    bytes: &[u8],
) -> Result<(), EngineError> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(failure("annotation library exceeds 16 MiB"));
    }
    check_path(path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut lock_name = path.as_os_str().to_os_string();
    lock_name.push(".lock");
    let lock_path = std::path::PathBuf::from(lock_name);
    check_path(&lock_path)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let lock = options.open(&lock_path)?;
    if redirected(&lock.metadata()?) || !lock.metadata()?.is_file() {
        return Err(failure("annotation library lock path is redirected"));
    }
    // The stable lock file is not removed: deleting a locked inode would allow another writer.
    // The OS releases the lock on close, including after a process exits unexpectedly.
    lock.try_lock()
        .map_err(|_| failure("annotation library is being saved by another instance; try again"))?;
    if read(path)?.as_deref() != expected {
        return Err(failure(
            "annotation library changed on disk; reload it before saving",
        ));
    }
    sunipdf_core::write_atomic(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_library_replacement_refuses_external_changes_and_oversized_inputs() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("annotations.json");
        assert!(read(&path).unwrap().is_none());
        commit(&path, None, b"first").unwrap();
        commit(&path, Some(b"first"), b"second").unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some(b"second".as_slice()));
        assert!(commit(&path, Some(b"first"), b"lost update").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert!(commit(&path, Some(b"second"), &vec![0; MAX_BYTES as usize + 1]).is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    #[test]
    fn concurrent_library_writers_cannot_overwrite_each_other() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("annotations.json");
        commit(&path, None, b"initial").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let writers: Vec<_> = [b"first".as_slice(), b"second".as_slice()]
            .into_iter()
            .map(|bytes| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    commit(&path, Some(b"initial"), bytes)
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = writers
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let saved = std::fs::read(&path).unwrap();
        assert!(saved == b"first" || saved == b"second");
        commit(&path, Some(&saved), b"next").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"next");
    }

    #[test]
    fn library_parent_traversal_and_directory_inputs_are_refused() {
        let directory = tempfile::tempdir().unwrap();
        assert!(read(directory.path()).is_err());
        assert!(read(&directory.path().join("..\u{2f}library.json")).is_err());
    }
}
