//! Recovery snapshots never replace the user's original PDF or contain a password.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sunipdf_core::write_atomic;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn lock(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(path)
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub base: PathBuf,
    pub original: PathBuf,
    pub pages: usize,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    original: PathBuf,
    pages: usize,
    updated: u64,
}

impl Slot {
    pub fn save(&self, bytes: &[u8]) -> io::Result<()> {
        write_atomic(&self.base.with_extension("pdf"), bytes)?;
        let meta = Metadata {
            original: self.original.clone(),
            pages: self.pages,
            updated: now(),
        };
        write_atomic(
            &self.base.with_extension("json"),
            &serde_json::to_vec(&meta)?,
        )
    }

    pub fn remove(&self) -> io::Result<()> {
        remove(&self.base.with_extension("pdf"))?;
        remove(&self.base.with_extension("json"))
    }
}

fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub snapshot: PathBuf,
    pub original: Option<PathBuf>,
    pub pages: Option<usize>,
    pub updated: u64,
}

impl Entry {
    pub fn remove(&self) -> io::Result<()> {
        remove(&self.snapshot)?;
        remove(&self.snapshot.with_extension("json"))
    }
}

pub struct Session {
    root: PathBuf,
    dir: PathBuf,
    _lock: File,
}

impl Session {
    pub(crate) fn create(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&root)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = root.join(format!("{}-{stamp}-{}", std::process::id(), next_id()));
        fs::create_dir(&dir)?;
        let guard = lock(&dir.join("session.lock"))?;
        Ok(Self {
            root,
            dir,
            _lock: guard,
        })
    }

    pub fn slot(&self, id: u64, original: PathBuf, pages: usize) -> Slot {
        Slot {
            base: self.dir.join(format!("{id:016x}")),
            original,
            pages,
        }
    }

    pub fn scan(&self) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        let root = self.root.canonicalize()?;
        for item in fs::read_dir(&self.root)? {
            let Ok(item) = item else { continue };
            let path = item.path();
            if path == self.dir || !path.is_dir() {
                continue;
            }
            let Ok(canonical) = path.canonicalize() else {
                continue;
            };
            if !canonical.starts_with(&root) {
                continue;
            }
            // An exclusive Windows handle distinguishes a running instance from a crashed session.
            let Ok(_guard) = lock(&path.join("session.lock")) else {
                continue;
            };
            let Ok(files) = fs::read_dir(&path) else {
                continue;
            };
            for item in files {
                let Ok(item) = item else { continue };
                let snapshot = item.path();
                if snapshot.extension().is_none_or(|e| e != "pdf") || !snapshot.is_file() {
                    continue;
                }
                let metadata: Option<Metadata> = fs::read(snapshot.with_extension("json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok());
                let modified = fs::metadata(&snapshot)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                entries.push(Entry {
                    snapshot,
                    original: metadata.as_ref().map(|m| m.original.clone()),
                    pages: metadata.as_ref().map(|m| m.pages),
                    updated: metadata.map_or(modified, |m| m.updated),
                });
            }
        }
        entries.sort_by_key(|e| std::cmp::Reverse(e.updated));
        Ok(entries)
    }
}

pub struct RecoveryState {
    pub failed: bool,
    pub session: Option<Session>,
    pub entries: Vec<Entry>,
    pub open: bool,
    pub pending_restore: Option<Entry>,
    pub restored: HashMap<u64, Entry>,
    delete: Option<usize>,
    last: Instant,
    pub pending: bool,
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self {
            failed: false,
            session: None,
            entries: Vec::new(),
            open: false,
            pending_restore: None,
            restored: HashMap::new(),
            delete: None,
            last: Instant::now(),
            pending: false,
        }
    }
}

impl RecoveryState {
    pub fn load() -> Self {
        let root = std::env::var_os("SUNIPDF_RECOVERY_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("SuniPDF/recovery"))
            });
        let mut state = Self::default();
        if let Some(root) = root {
            match Session::create(root) {
                Ok(session) => {
                    state.entries = session.scan().unwrap_or_else(|e| {
                        log::warn!("Recovery scan failed: {e}");
                        state.failed = true;
                        Vec::new()
                    });
                    state.open = !state.entries.is_empty();
                    state.session = Some(session);
                }
                Err(e) => {
                    log::error!("Recovery storage unavailable: {e}");
                    state.failed = true;
                }
            }
        } else {
            state.failed = true;
        }
        state
    }

    pub fn due(&mut self) -> bool {
        if self.pending || self.session.is_none() || self.last.elapsed() < Duration::from_secs(60) {
            return false;
        }
        self.last = Instant::now();
        true
    }

    pub fn finish(&mut self, id: u64) -> io::Result<()> {
        if let Some(entry) = self.restored.get(&id) {
            entry.remove()?;
        }
        self.restored.remove(&id);
        Ok(())
    }
}

impl crate::app::SuniApp {
    pub(crate) fn recovery_slot(&self) -> Option<Slot> {
        if !self.settings.autosave {
            return None;
        }
        let tab = self.tabs.active.and_then(|i| self.tabs.tabs.get(i))?;
        let doc = self.document.as_ref()?;
        Some(self.recovery.session.as_ref()?.slot(
            tab.recovery_id,
            doc.info.path.clone(),
            doc.info.page_count(),
        ))
    }

    pub(crate) fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery.open {
            return;
        }
        let lang = self.lang();
        let (mut restore, mut discard, mut later) = (None, None, false);
        crate::dialog::show(
            ctx,
            crate::dialog::Spec::new(
                "recovery",
                sunipdf_i18n::tr(lang, "recovery.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(560.0),
            |ui| {
                ui.label(sunipdf_i18n::tr(lang, "recovery.explain"));
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (index, entry) in self.recovery.entries.iter().enumerate() {
                            ui.group(|ui| {
                                ui.label(entry.original.as_ref().map_or_else(
                                    || sunipdf_i18n::tr(lang, "recovery.unknown").into(),
                                    |p| p.display().to_string(),
                                ));
                                if let Some(pages) = entry.pages {
                                    ui.label(sunipdf_i18n::tr_args(
                                        lang,
                                        "recovery.pages",
                                        &[("count", &pages.to_string())],
                                    ));
                                }
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(sunipdf_i18n::tr(lang, "recovery.restore"))
                                        .clicked()
                                    {
                                        restore = Some(index);
                                    }
                                    if ui
                                        .button(sunipdf_i18n::tr(lang, "recovery.delete"))
                                        .clicked()
                                    {
                                        self.recovery.delete = Some(index);
                                    }
                                });
                                if self.recovery.delete == Some(index) {
                                    ui.label(sunipdf_i18n::tr(lang, "recovery.confirm_delete"));
                                    if ui
                                        .button(sunipdf_i18n::tr(lang, "recovery.confirm"))
                                        .clicked()
                                    {
                                        discard = Some(index);
                                    }
                                }
                            });
                        }
                    });
            },
            |ui| {
                later = crate::dialog::secondary(ui, sunipdf_i18n::tr(lang, "recovery.later"));
            },
        );
        if let Some(index) = discard {
            match self.recovery.entries[index].remove() {
                Ok(()) => {
                    self.recovery.entries.remove(index);
                    self.recovery.delete = None;
                }
                Err(e) => {
                    log::error!("Recovery removal failed: {e}");
                    self.error = Some("error.recovery_failed");
                }
            }
        }
        if let Some(index) = restore {
            let entry = self.recovery.entries[index].clone();
            self.recovery.pending_restore = Some(entry.clone());
            self.recovery.open = false;
            self.open_path(entry.snapshot, None);
        }
        if later || self.recovery.entries.is_empty() {
            self.recovery.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_autosave_waits_for_numeric_or_drawn_field_draft_without_consuming_deadline() {
        for drawing in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.autosave = true;
            app.dirty = true;
            app.recovery.session = Some(Session::create(dir.path().join("recovery")).unwrap());
            app.recovery.last = Instant::now() - Duration::from_secs(61);
            let deadline = app.recovery.last;
            app.execute(
                &egui::Context::default(),
                sunipdf_core::Command::ToolAddTextField,
            );
            if drawing {
                app.form_builder.start_drawing();
                app.form_builder.start_drag(
                    0,
                    (20.0, 30.0),
                    app.document.as_ref().unwrap().info.page_sizes[0],
                );
            }
            app.autosave_tick();
            assert!(app.form_builder.active() && !app.recovery.pending);
            assert_eq!(app.recovery.last, deadline);
            assert!(requests.try_recv().is_err());
            app.form_builder.cancel();
            app.autosave_tick();
            assert!(app.recovery.pending);
            assert!(matches!(
                requests.recv().unwrap(),
                crate::worker::Request::Autosave
            ));
        }
    }
    #[test]
    fn snapshots_preserve_original_and_corrupt_metadata_is_recoverable() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.pdf");
        fs::write(&original, b"original").unwrap();
        let session = Session::create(dir.path().join("recovery")).unwrap();
        let slot = session.slot(1, original.clone(), 2);
        slot.save(b"encrypted snapshot bytes").unwrap();
        assert_eq!(fs::read(&original).unwrap(), b"original");
        let meta = fs::read_to_string(slot.base.with_extension("json")).unwrap();
        assert!(!meta.contains("password"));
        fs::write(slot.base.with_extension("json"), b"corrupt").unwrap();
        drop(session);
        let next = Session::create(dir.path().join("recovery")).unwrap();
        let entries = next.scan().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].original.is_none());
        assert_eq!(
            fs::read(&entries[0].snapshot).unwrap(),
            b"encrypted snapshot bytes"
        );
        entries[0].remove().unwrap();
        assert!(next.scan().unwrap().is_empty());
    }

    #[test]
    #[cfg(windows)]
    fn live_sessions_are_skipped_and_slots_do_not_depend_on_tab_indices() {
        let dir = tempfile::tempdir().unwrap();
        let first = Session::create(dir.path().into()).unwrap();
        let slot = first.slot(next_id(), PathBuf::from("confidential.pdf"), 1);
        slot.save(b"snapshot").unwrap();
        let second = Session::create(dir.path().into()).unwrap();
        assert!(second.scan().unwrap().is_empty());
        drop(first);
        assert_eq!(second.scan().unwrap().len(), 1);
        slot.remove().unwrap();
        assert!(second.scan().unwrap().is_empty());
    }
}
