//! Regular-file input with platform-correct protection against final symlink and FIFO races.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

/// Opens a regular file for reading and checks the opened descriptor again.
///
/// On Unix, a final symlink is refused and replacement by a FIFO cannot block the open.
/// Callers still enforce their own ancestor, size, identity and content checks.
pub fn open_regular(path: &Path) -> io::Result<File> {
    if !path.metadata()?.is_file() {
        return Err(io::Error::other("Choose a regular file"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("Choose a regular file"));
    }
    Ok(file)
}

/// Why [`read_bounded`] refused a file; callers map each case to their own message.
#[derive(Debug)]
pub enum BoundedReadError {
    /// The path does not name a regular file.
    NotRegular,
    /// The opened file declares more than the allowed number of bytes.
    TooLarge,
    /// The file grew past the allowed number of bytes while it was read.
    Grew,
    Io(io::Error),
}

impl From<io::Error> for BoundedReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Reads a regular file of at most `max` bytes through [`open_regular`].
pub fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, BoundedReadError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(BoundedReadError::NotRegular);
    }
    let file = open_regular(path)?;
    if file.metadata()?.len() > max {
        return Err(BoundedReadError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(BoundedReadError::Grew);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reads_return_small_files_and_refuse_large_files_and_directories() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input.json");
        std::fs::write(&path, b"1234").unwrap();
        assert_eq!(read_bounded(&path, 4).unwrap(), b"1234");
        assert!(matches!(
            read_bounded(&path, 3),
            Err(BoundedReadError::TooLarge)
        ));
        assert!(matches!(
            read_bounded(temp.path(), 4),
            Err(BoundedReadError::NotRegular)
        ));
    }

    #[test]
    fn regular_file_contents_are_read_and_directories_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.pdf");
        std::fs::write(&path, b"source contents").unwrap();
        let mut bytes = Vec::new();
        open_regular(&path)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, b"source contents");
        assert!(open_regular(temp.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_descriptors_are_nonblocking_and_final_symlinks_are_refused() {
        use std::os::{fd::AsRawFd, unix::fs::symlink};

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.pdf");
        std::fs::write(&path, b"source contents").unwrap();
        let file = open_regular(&path).unwrap();
        // SAFETY: F_GETFL reads flags from the live borrowed descriptor and has no extra argument.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0, "{}", io::Error::last_os_error());
        assert_ne!(flags & libc::O_NONBLOCK, 0);

        let alias = temp.path().join("alias.pdf");
        symlink(&path, &alias).unwrap();
        assert!(open_regular(&alias).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn named_pipes_are_refused_without_waiting_for_a_writer() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("named.pdf");
        let name = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: name is a valid NUL-terminated path inside this test's private directory.
        let result = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
        assert_eq!(result, 0, "{}", io::Error::last_os_error());
        assert!(open_regular(&path).is_err());
    }
}
