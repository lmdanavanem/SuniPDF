//! Portable, bounded references to explicitly selected local PDF sheets.
//!
//! A manifest stores metadata and source fingerprints, never PDF content or credentials.
//! Source inspection belongs on the same thread as the application's PDF engine.

use crate::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const VERSION: u32 = 1;
pub const MAX_FILES: usize = 64;
pub const MAX_SHEETS: usize = 1024;
pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PAGES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSet {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub files: Vec<PlanFile>,
    pub sheets: Vec<PlanSheet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanFile {
    pub id: String,
    /// Forward-slash path, relative to the manifest's parent directory.
    pub path: String,
    pub sha256: String,
    pub byte_len: u64,
    pub page_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSheet {
    pub id: String,
    pub file_id: String,
    /// Physical, one-based page number. Labels never override this address.
    pub page: usize,
    pub number: String,
    pub title: String,
    pub discipline: String,
    pub revision: String,
}

#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    pub discipline: String,
    pub revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceStatus {
    Ready,
    Missing,
    Changed,
    Unreadable,
    PasswordRequired,
    UnsafePath,
    PageOutOfRange,
    Duplicate,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub file_id: String,
    pub path: Option<PathBuf>,
    pub status: ReferenceStatus,
    pub message: Option<String>,
    pub actual_sha256: Option<String>,
    pub page_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SheetReport {
    pub sheet_id: String,
    pub file_id: String,
    pub page: usize,
    pub path: Option<PathBuf>,
    pub status: ReferenceStatus,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanSetReport {
    pub files: Vec<FileReport>,
    pub sheets: Vec<SheetReport>,
    pub cancelled: bool,
}

impl PlanSetReport {
    pub fn complete(&self) -> bool {
        !self.cancelled
            && self
                .files
                .iter()
                .all(|f| f.status == ReferenceStatus::Ready)
            && self
                .sheets
                .iter()
                .all(|s| s.status == ReferenceStatus::Ready)
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "complete": self.complete(), "cancelled": self.cancelled,
            "files": self.files, "sheets": self.sheets,
        })
    }
}

/// A source-bound read for navigation. Open these bytes, not the path again.
/// After opening, the caller must confirm the page against the actual document.
#[derive(Debug)]
pub struct VerifiedSheet {
    pub path: PathBuf,
    pub page: usize,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

fn failure(message: impl Into<String>) -> EngineError {
    EngineError::Other(format!("Plan set: {}", message.into()))
}

fn identifier(prefix: &str) -> Result<String, EngineError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| failure(error.to_string()))?;
    let value: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("{prefix}-{value}"))
}

impl PlanSet {
    pub fn new(name: &str) -> Result<Self, EngineError> {
        let set = Self {
            version: VERSION,
            id: identifier("set")?,
            name: name.to_owned(),
            files: Vec::new(),
            sheets: Vec::new(),
        };
        validate(&set)?;
        Ok(set)
    }
}

fn text(value: &str, limit: usize, required: bool) -> bool {
    value.len() <= limit
        && !value.chars().any(char::is_control)
        && (!required || !value.trim().is_empty())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Validates the data model without opening referenced files.
/// Unsafe imported paths remain inspectable as explicit `unsafe_path` rows.
pub fn validate(set: &PlanSet) -> Result<(), EngineError> {
    if set.version != VERSION {
        return Err(failure("unsupported manifest version"));
    }
    if !valid_id(&set.id) || !text(&set.name, 512, true) {
        return Err(failure("invalid set identity or name"));
    }
    if set.files.len() > MAX_FILES || set.sheets.len() > MAX_SHEETS {
        return Err(failure("manifest exceeds 64 files or 1024 sheets"));
    }
    let mut files = HashSet::new();
    for file in &set.files {
        if !valid_id(&file.id)
            || !files.insert(file.id.as_str())
            || !text(&file.path, 4096, true)
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || file.byte_len == 0
            || file.byte_len > MAX_FILE_BYTES
            || file.page_count == 0
            || file.page_count > MAX_PAGES
        {
            return Err(failure("invalid or duplicate file metadata"));
        }
    }
    let mut sheets = HashSet::new();
    for sheet in &set.sheets {
        if !valid_id(&sheet.id)
            || !sheets.insert(sheet.id.as_str())
            || !files.contains(sheet.file_id.as_str())
            || sheet.page == 0
            || sheet.page > MAX_PAGES
            || !text(&sheet.number, 256, true)
            || !text(&sheet.title, 1024, false)
            || !text(&sheet.discipline, 256, false)
            || !text(&sheet.revision, 128, false)
        {
            return Err(failure("invalid or duplicate sheet metadata"));
        }
    }
    Ok(())
}

pub fn parse(bytes: &[u8]) -> Result<PlanSet, EngineError> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(failure("manifest exceeds 2 MiB"));
    }
    let set: PlanSet = serde_json::from_slice(bytes)
        .map_err(|error| failure(format!("invalid manifest JSON: {error}")))?;
    validate(&set)?;
    Ok(set)
}

pub fn load(path: &Path) -> Result<PlanSet, EngineError> {
    let file = open_regular(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(failure("choose a manifest file no larger than 2 MiB"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    parse(&bytes)
}

pub fn encode(set: &PlanSet) -> Result<Vec<u8>, EngineError> {
    validate(set)?;
    let mut bytes = serde_json::to_vec_pretty(set).map_err(|e| failure(e.to_string()))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(failure("manifest exceeds 2 MiB"));
    }
    Ok(bytes)
}

fn root(manifest: &Path) -> Result<PathBuf, EngineError> {
    let parent = manifest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = parent.canonicalize()?;
    if !root.is_dir() {
        return Err(failure("manifest parent must be an existing directory"));
    }
    Ok(root)
}

fn portable_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| {
            if part.is_empty()
                || part.len() > 255
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
            {
                return false;
            }
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        value.replace('\\', "/").to_lowercase()
    } else {
        value
    }
}

fn within(root: &Path, path: &Path) -> bool {
    // Inputs are canonical paths. Exact components also protect NTFS directories
    // with per-directory case sensitivity; case folding is only for duplicate warnings.
    path.starts_with(root)
}

fn open_regular(path: &Path) -> std::io::Result<File> {
    if !path.metadata()?.is_file() {
        return Err(std::io::Error::other("Choose a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NONBLOCK | O_NOFOLLOW: a file replaced by a FIFO cannot hang
        // the reader between metadata and open, nor can a final symlink be followed.
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("Choose a regular file"));
    }
    Ok(file)
}

#[derive(Debug)]
struct Problem {
    status: ReferenceStatus,
    message: String,
}

impl Problem {
    fn unsafe_path() -> Self {
        Self { status: ReferenceStatus::UnsafePath, message: "Reference must stay inside the manifest directory without symbolic links or reparse points".into() }
    }

    fn engine(self) -> EngineError {
        failure(self.message)
    }
}

impl From<std::io::Error> for Problem {
    fn from(error: std::io::Error) -> Self {
        Self {
            status: if error.kind() == std::io::ErrorKind::NotFound {
                ReferenceStatus::Missing
            } else {
                ReferenceStatus::Unreadable
            },
            message: error.to_string(),
        }
    }
}

fn reparse(metadata: &std::fs::Metadata) -> bool {
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

/// Checks existing ancestors even when the final file is missing.
fn candidate_path(root: &Path, relative: &str) -> Result<PathBuf, Problem> {
    if !portable_path(relative) {
        return Err(Problem::unsafe_path());
    }
    let mut path = root.to_owned();
    for component in relative.split('/') {
        path.push(component);
        match path.symlink_metadata() {
            Ok(metadata) if reparse(&metadata) => return Err(Problem::unsafe_path()),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

fn opened_path(file: &File, expected: &Path) -> Result<PathBuf, Problem> {
    #[cfg(windows)]
    {
        use std::os::windows::{ffi::OsStringExt, io::AsRawHandle};
        use windows::Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED},
        };
        let mut buffer = vec![0_u16; 32_768];
        // Borrow the live file handle; this call neither transfers nor closes it.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                HANDLE(file.as_raw_handle()),
                &mut buffer,
                FILE_NAME_NORMALIZED,
            )
        } as usize;
        if length == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if length >= buffer.len() {
            return Err(Problem::unsafe_path());
        }
        let _ = expected;
        Ok(PathBuf::from(std::ffi::OsString::from_wide(
            &buffer[..length],
        )))
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let _ = expected;
        std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd())).map_err(Into::into)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = file;
        expected.canonicalize().map_err(Into::into)
    }
}

fn read_source(root: &Path, relative: &str) -> Result<(PathBuf, Vec<u8>), Problem> {
    let candidate = candidate_path(root, relative)?;
    let path = candidate.canonicalize()?;
    if !within(root, &path) {
        return Err(Problem::unsafe_path());
    }
    let file = open_regular(&path)?;
    let actual = opened_path(&file, &path)?;
    if !within(root, &actual) || path_key(&actual) != path_key(&path) {
        return Err(Problem::unsafe_path());
    }
    let before = file.metadata()?;
    if !before.is_file() || before.len() == 0 || before.len() > MAX_FILE_BYTES {
        return Err(Problem {
            status: ReferenceStatus::Unreadable,
            message: "Choose a regular PDF file no larger than 64 MiB".into(),
        });
    }
    let mut bytes = Vec::new();
    (&file).take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
        || bytes.len() as u64 > MAX_FILE_BYTES
    {
        return Err(Problem {
            status: ReferenceStatus::Unreadable,
            message: "Source changed during reading; retry inspection".into(),
        });
    }
    let final_path = candidate_path(root, relative)?.canonicalize()?;
    if path_key(&final_path) != path_key(&path) {
        return Err(Problem::unsafe_path());
    }
    Ok((path, bytes))
}

fn relative_path(root: &Path, source: &Path) -> Result<String, EngineError> {
    if !within(root, source) {
        return Err(Problem::unsafe_path().engine());
    }
    let root_len = root.components().count();
    let mut pieces = Vec::new();
    for part in source.components().skip(root_len) {
        pieces.push(
            part.as_os_str()
                .to_str()
                .ok_or_else(|| failure("paths must be valid Unicode"))?
                .to_owned(),
        );
    }
    let relative = pieces.join("/");
    if !portable_path(&relative) {
        return Err(Problem::unsafe_path().engine());
    }
    candidate_path(root, &relative).map_err(Problem::engine)?;
    Ok(relative)
}

fn inspect_pdf(
    engine: &Engine,
    bytes: Vec<u8>,
    path: &Path,
    password: Option<&str>,
) -> Result<usize, EngineError> {
    let document = engine.open_bytes(bytes, password, path)?;
    let pages = document.info().page_count();
    if pages == 0 || pages > MAX_PAGES {
        return Err(failure("source exceeds 10000 pages or has no pages"));
    }
    // Page count is view metadata. No text, label or annotation is extracted from protected PDFs.
    Ok(pages)
}

fn source_file(
    engine: &Engine,
    root: &Path,
    source: &Path,
    password: Option<&str>,
    id: String,
) -> Result<PlanFile, EngineError> {
    let canonical = source.canonicalize()?;
    let path = relative_path(root, &canonical)?;
    let (canonical, bytes) = read_source(root, &path).map_err(Problem::engine)?;
    let sha256 = crate::local_review::digest(&bytes);
    let byte_len = bytes.len() as u64;
    let page_count = inspect_pdf(engine, bytes, &canonical, password)?;
    Ok(PlanFile {
        id,
        path,
        sha256,
        byte_len,
        page_count,
    })
}

/// Adds all pages of one explicitly selected PDF. Failure leaves the set unchanged.
pub fn add_pdf(
    set: &mut PlanSet,
    engine: &Engine,
    manifest_path: &Path,
    source: &Path,
    password: Option<&str>,
    options: &AddOptions,
) -> Result<Vec<String>, EngineError> {
    validate(set)?;
    if set.files.len() == MAX_FILES {
        return Err(failure("manifest exceeds 64 files"));
    }
    if !text(&options.discipline, 256, false) || !text(&options.revision, 128, false) {
        return Err(failure("invalid discipline or revision"));
    }
    let root = root(manifest_path)?;
    let file = source_file(engine, &root, source, password, identifier("file")?)?;
    if set.sheets.len().saturating_add(file.page_count) > MAX_SHEETS {
        return Err(failure("adding all source pages would exceed 1024 sheets"));
    }
    let path = root.join(&file.path);
    if set
        .files
        .iter()
        .any(|existing| path_key(&root.join(&existing.path)) == path_key(&path))
    {
        return Err(failure("this source path is already in the set"));
    }
    let title = Path::new(&file.path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sheet");
    let mut next = set.clone();
    let mut added = Vec::new();
    for page in 1..=file.page_count {
        let id = identifier("sheet")?;
        added.push(id.clone());
        next.sheets.push(PlanSheet {
            id,
            file_id: file.id.clone(),
            page,
            number: format!("{title}-{page}"),
            title: format!("{title} — {page}"),
            discipline: options.discipline.clone(),
            revision: options.revision.clone(),
        });
    }
    next.files.push(file);
    encode(&next)?;
    *set = next;
    Ok(added)
}

/// Explicitly accepts the selected file's current revision or new location.
/// Existing sheet identities and metadata survive; out-of-range pages remain visible.
pub fn refresh_exact(
    set: &mut PlanSet,
    engine: &Engine,
    manifest_path: &Path,
    file_id: &str,
    source: Option<&Path>,
    password: Option<&str>,
) -> Result<(), EngineError> {
    validate(set)?;
    let index = set
        .files
        .iter()
        .position(|file| file.id == file_id)
        .ok_or_else(|| failure("file identity was not found"))?;
    let root = root(manifest_path)?;
    let selected = match source {
        Some(path) => path.to_owned(),
        None => candidate_path(&root, &set.files[index].path).map_err(Problem::engine)?,
    };
    let file = source_file(engine, &root, &selected, password, file_id.to_owned())?;
    let new_path = path_key(&root.join(&file.path));
    if set
        .files
        .iter()
        .enumerate()
        .any(|(i, other)| i != index && path_key(&root.join(&other.path)) == new_path)
    {
        return Err(failure("this source path is already in the set"));
    }
    let mut next = set.clone();
    next.files[index] = file;
    encode(&next)?;
    *set = next;
    Ok(())
}

fn duplicates(set: &PlanSet) -> HashSet<&str> {
    let mut keys: HashMap<(String, String, String), Vec<&str>> = HashMap::new();
    let mut addresses: HashMap<(&str, usize), Vec<&str>> = HashMap::new();
    for sheet in &set.sheets {
        keys.entry((
            sheet.discipline.trim().to_lowercase(),
            sheet.number.trim().to_lowercase(),
            sheet.revision.trim().to_lowercase(),
        ))
        .or_default()
        .push(&sheet.id);
        addresses
            .entry((&sheet.file_id, sheet.page))
            .or_default()
            .push(&sheet.id);
    }
    keys.into_values()
        .chain(addresses.into_values())
        .filter(|ids| ids.len() > 1)
        .flatten()
        .collect()
}

fn inspect_file(
    engine: &Engine,
    root: &Path,
    file: &PlanFile,
    password: Option<&str>,
) -> FileReport {
    let mut report = FileReport {
        file_id: file.id.clone(),
        path: None,
        status: ReferenceStatus::Unreadable,
        message: None,
        actual_sha256: None,
        page_count: None,
    };
    let (path, bytes) = match read_source(root, &file.path) {
        Ok(value) => value,
        Err(error) => {
            report.status = error.status;
            report.message = Some(error.message);
            return report;
        }
    };
    report.path = Some(path.clone());
    let digest = crate::local_review::digest(&bytes);
    let matches = digest.eq_ignore_ascii_case(&file.sha256) && bytes.len() as u64 == file.byte_len;
    report.actual_sha256 = Some(digest);
    if !matches {
        report.status = ReferenceStatus::Changed;
        report.message = Some(
            "Source fingerprint changed; accept the revision explicitly before navigation".into(),
        );
        return report;
    }
    match inspect_pdf(engine, bytes, &path, password) {
        Ok(pages) => {
            report.page_count = Some(pages);
            if pages != file.page_count {
                report.status = ReferenceStatus::Changed;
                report.message =
                    Some("Stored page count differs from the source; refresh explicitly".into());
            } else {
                report.status = ReferenceStatus::Ready;
            }
        }
        Err(EngineError::PasswordRequired) => {
            report.status = ReferenceStatus::PasswordRequired;
            report.message = Some(
                "Source requires a valid PDF password; credentials are never stored in the set"
                    .into(),
            );
        }
        Err(error) => {
            report.message = Some(error.to_string());
        }
    }
    report
}

/// Checks only manifest-listed files. Cancellation is observed between file operations.
pub fn inspect(
    engine: &Engine,
    manifest_path: &Path,
    set: &PlanSet,
    password: Option<&str>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<PlanSetReport, EngineError> {
    validate(set)?;
    let root = root(manifest_path)?;
    let duplicate_sheets = duplicates(set);
    let mut files = Vec::new();
    let mut paths: HashMap<String, Vec<usize>> = HashMap::new();
    for file in &set.files {
        let report = if cancel.load(Ordering::Relaxed) {
            FileReport {
                file_id: file.id.clone(),
                path: None,
                status: ReferenceStatus::Cancelled,
                message: Some("Inspection cancelled".into()),
                actual_sha256: None,
                page_count: None,
            }
        } else {
            inspect_file(engine, &root, file, password)
        };
        if let Some(path) = &report.path {
            paths.entry(path_key(path)).or_default().push(files.len());
        }
        files.push(report);
        progress(files.len(), set.files.len());
    }
    for indices in paths.into_values().filter(|indices| indices.len() > 1) {
        for index in indices {
            if files[index].status == ReferenceStatus::Ready {
                files[index].status = ReferenceStatus::Duplicate;
                files[index].message = Some("Multiple file entries refer to the same path".into());
            }
        }
    }
    let by_id: HashMap<_, _> = files
        .iter()
        .map(|file| (file.file_id.as_str(), file))
        .collect();
    let sheets = set
        .sheets
        .iter()
        .map(|sheet| {
            let file = by_id[sheet.file_id.as_str()];
            let mut status = file.status;
            let mut issues: Vec<String> = file.message.iter().cloned().collect();
            if file.page_count.is_some_and(|count| sheet.page > count) {
                issues.push("Physical page is outside the current source".into());
                if status == ReferenceStatus::Ready {
                    status = ReferenceStatus::PageOutOfRange;
                }
            }
            if duplicate_sheets.contains(sheet.id.as_str()) {
                issues.push("Duplicate sheet number/revision or physical page reference".into());
                if status == ReferenceStatus::Ready {
                    status = ReferenceStatus::Duplicate;
                }
            }
            SheetReport {
                sheet_id: sheet.id.clone(),
                file_id: sheet.file_id.clone(),
                page: sheet.page,
                path: file.path.clone(),
                status,
                issues,
            }
        })
        .collect();
    Ok(PlanSetReport {
        files,
        sheets,
        cancelled: cancel.load(Ordering::Relaxed),
    })
}

pub fn read_sheet(
    manifest_path: &Path,
    set: &PlanSet,
    sheet_id: &str,
) -> Result<VerifiedSheet, EngineError> {
    validate(set)?;
    let sheet = set
        .sheets
        .iter()
        .find(|s| s.id == sheet_id)
        .ok_or_else(|| failure("sheet identity was not found"))?;
    if duplicates(set).contains(sheet_id) {
        return Err(failure(
            "resolve duplicate sheet references before navigation",
        ));
    }
    let file = set
        .files
        .iter()
        .find(|f| f.id == sheet.file_id)
        .ok_or_else(|| failure("file identity was not found"))?;
    if sheet.page > file.page_count {
        return Err(EngineError::PageOutOfRange(sheet.page - 1));
    }
    let root = root(manifest_path)?;
    let (path, bytes) = read_source(&root, &file.path).map_err(Problem::engine)?;
    if set
        .files
        .iter()
        .filter(|f| path_key(&root.join(&f.path)) == path_key(&path))
        .count()
        != 1
    {
        return Err(failure(
            "resolve duplicate file references before navigation",
        ));
    }
    let sha256 = crate::local_review::digest(&bytes);
    if !sha256.eq_ignore_ascii_case(&file.sha256) || bytes.len() as u64 != file.byte_len {
        return Err(failure(
            "source fingerprint changed; explicitly refresh this file",
        ));
    }
    Ok(VerifiedSheet {
        path,
        page: sheet.page,
        bytes,
        sha256,
    })
}

/// Preserves file targets when saving elsewhere; refuses references outside the new root.
/// Missing files remain representable and are never created by this operation.
pub fn rebase(
    set: &PlanSet,
    old_manifest: &Path,
    new_manifest: &Path,
) -> Result<PlanSet, EngineError> {
    validate(set)?;
    let old_root = root(old_manifest)?;
    let new_root = root(new_manifest)?;
    let mut rebased = set.clone();
    for file in &mut rebased.files {
        let source = candidate_path(&old_root, &file.path).map_err(Problem::engine)?;
        file.path = relative_path(&new_root, &source)?;
    }
    encode(&rebased)?;
    Ok(rebased)
}

/// Writes only a new manifest. It never overwrites a manifest or a referenced PDF.
pub fn save_new(set: &PlanSet, old_manifest: &Path, destination: &Path) -> Result<(), EngineError> {
    let set = rebase(set, old_manifest, destination)?;
    let root = root(destination)?;
    let name = destination
        .file_name()
        .ok_or_else(|| failure("choose a manifest filename"))?;
    let output = root.join(name);
    if set
        .files
        .iter()
        .any(|file| path_key(&root.join(&file.path)) == path_key(&output))
    {
        return Err(failure("manifest destination is a referenced source file"));
    }
    crate::write_new_copy(&output, &encode(&set)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn canonical_containment_does_not_fold_case_sensitive_siblings() {
        let root = Path::new(r"\\?\C:\project\Plans");
        assert!(within(root, Path::new(r"\\?\C:\project\Plans\a.pdf")));
        assert!(!within(root, Path::new(r"\\?\C:\project\plans\a.pdf")));
        assert!(!within(
            root,
            Path::new(r"\\?\C:\project\Plans-other\a.pdf")
        ));
    }

    #[test]
    fn directories_are_not_regular_input_files() {
        let temp = tempfile::tempdir().unwrap();
        assert!(open_regular(temp.path()).is_err());
        assert!(load(temp.path()).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn fifo_inputs_are_refused_before_opening() {
        let temp = tempfile::tempdir().unwrap();
        let fifo = temp.path().join("named.pdf");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        assert!(open_regular(&fifo).is_err());
        assert!(load(&fifo).is_err());
    }
}
