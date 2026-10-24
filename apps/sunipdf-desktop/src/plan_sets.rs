//! Local plan-set manifests. PDF inspection stays on the single engine worker.

use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    plan_set::{self as core, PlanSet, PlanSetReport, PlanSheet, ReferenceStatus, VerifiedSheet},
    Engine, EngineError,
};
use sunipdf_i18n::{tr, tr_args};
use zeroize::Zeroizing;

type Password = Option<Zeroizing<String>>;

pub struct Navigation {
    pub sheet: VerifiedSheet,
    pub password: Password,
    pub page_count: usize,
    pub active_source: Option<(PathBuf, String)>,
}

#[derive(Clone, Copy)]
enum NavigationTarget {
    Active,
    Tab(usize),
    New,
}

struct PendingNavigation {
    candidate: Navigation,
    target: NavigationTarget,
    allow_existing: bool,
    epoch: u64,
}

pub enum Job {
    Open {
        path: PathBuf,
        password: Password,
        cancel: Arc<AtomicBool>,
    },
    Add {
        set: PlanSet,
        path: PathBuf,
        sources: Vec<PathBuf>,
        password: Password,
        cancel: Arc<AtomicBool>,
    },
    Inspect {
        set: PlanSet,
        path: PathBuf,
        password: Password,
        cancel: Arc<AtomicBool>,
    },
    Refresh {
        set: PlanSet,
        path: PathBuf,
        file_id: String,
        source: Option<PathBuf>,
        password: Password,
        cancel: Arc<AtomicBool>,
    },
    Save {
        set: PlanSet,
        path: PathBuf,
        destination: PathBuf,
    },
    Navigate {
        set: PlanSet,
        path: PathBuf,
        sheet_id: String,
        password: Password,
        cancel: Arc<AtomicBool>,
    },
}

pub enum Outcome {
    Loaded {
        set: PlanSet,
        path: PathBuf,
        report: PlanSetReport,
    },
    Changed {
        set: PlanSet,
        report: PlanSetReport,
    },
    Inspected(PlanSetReport),
    Saved {
        set: PlanSet,
        path: PathBuf,
    },
    Navigate(Navigation),
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

pub fn run_job(
    engine: &Engine,
    job: Job,
    mut progress: impl FnMut(usize, usize),
) -> Result<Outcome, EngineError> {
    match job {
        Job::Open {
            path,
            password,
            cancel,
        } => {
            check_cancel(&cancel)?;
            let set = core::load(&path)?;
            let report = core::inspect(
                engine,
                &path,
                &set,
                password.as_ref().map(|p| p.as_str()),
                &cancel,
                progress,
            )?;
            check_cancel(&cancel)?;
            Ok(Outcome::Loaded { set, path, report })
        }
        Job::Add {
            mut set,
            path,
            sources,
            password,
            cancel,
        } => {
            if sources.is_empty() || sources.len() > 64 {
                return Err(EngineError::Other(
                    "select between 1 and 64 PDF files".into(),
                ));
            }
            for (index, source) in sources.iter().enumerate() {
                check_cancel(&cancel)?;
                core::add_pdf(
                    &mut set,
                    engine,
                    &path,
                    source,
                    password.as_ref().map(|p| p.as_str()),
                    &Default::default(),
                )?;
                progress(index + 1, sources.len());
            }
            let report = core::inspect(
                engine,
                &path,
                &set,
                password.as_ref().map(|p| p.as_str()),
                &cancel,
                progress,
            )?;
            check_cancel(&cancel)?;
            Ok(Outcome::Changed { set, report })
        }
        Job::Inspect {
            set,
            path,
            password,
            cancel,
        } => {
            let report = core::inspect(
                engine,
                &path,
                &set,
                password.as_ref().map(|p| p.as_str()),
                &cancel,
                progress,
            )?;
            check_cancel(&cancel)?;
            Ok(Outcome::Inspected(report))
        }
        Job::Refresh {
            mut set,
            path,
            file_id,
            source,
            password,
            cancel,
        } => {
            check_cancel(&cancel)?;
            core::refresh_exact(
                &mut set,
                engine,
                &path,
                &file_id,
                source.as_deref(),
                password.as_ref().map(|p| p.as_str()),
            )?;
            let report = core::inspect(
                engine,
                &path,
                &set,
                password.as_ref().map(|p| p.as_str()),
                &cancel,
                progress,
            )?;
            check_cancel(&cancel)?;
            Ok(Outcome::Changed { set, report })
        }
        Job::Save {
            set,
            path,
            destination,
        } => {
            let rebased = core::rebase(&set, &path, &destination)?;
            core::save_new(&set, &path, &destination)?;
            Ok(Outcome::Saved {
                set: rebased,
                path: destination,
            })
        }
        Job::Navigate {
            set,
            path,
            sheet_id,
            password,
            cancel,
        } => {
            check_cancel(&cancel)?;
            let verified = core::read_sheet(&path, &set, &sheet_id)?;
            let doc = engine.open_bytes(
                verified.bytes.clone(),
                password.as_ref().map(|p| p.as_str()),
                &verified.path,
            )?;
            let expected_pages = set
                .sheets
                .iter()
                .find(|s| s.id == sheet_id)
                .and_then(|s| set.files.iter().find(|f| f.id == s.file_id))
                .map(|f| f.page_count);
            if expected_pages != Some(doc.page_count())
                || verified.page == 0
                || verified.page > doc.page_count()
            {
                return Err(EngineError::PageOutOfRange(verified.page.saturating_sub(1)));
            }
            check_cancel(&cancel)?;
            Ok(Outcome::Navigate(Navigation {
                sheet: verified,
                password,
                page_count: doc.page_count(),
                active_source: None,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::tests::test_app,
        worker::{DocState, Response},
    };
    use sunipdf_core::{plan_set::PlanFile, Command};
    use sunipdf_i18n::Language;

    fn sample() -> PlanSet {
        let mut set = PlanSet::new("Drawing package").unwrap();
        set.files.push(PlanFile {
            id: "file-one".into(),
            path: "plans/site.pdf".into(),
            sha256: "a".repeat(64),
            byte_len: 10,
            page_count: 2,
        });
        for page in 1..=2 {
            set.sheets.push(PlanSheet {
                id: format!("sheet-{page}"),
                file_id: "file-one".into(),
                page,
                number: format!("A-{page}"),
                title: format!("Sheet {page}"),
                discipline: "Architecture".into(),
                revision: "A".into(),
            });
        }
        core::validate(&set).unwrap();
        set
    }

    fn empty_report() -> PlanSetReport {
        PlanSetReport {
            files: vec![],
            sheets: vec![],
            cancelled: false,
        }
    }

    #[test]
    fn metadata_edits_preserve_source_identity_and_invalid_drafts_are_atomic() {
        let original = sample();
        let mut state = PlanSetState::default();
        state.replace(original.clone(), PathBuf::from("set.json"), false);
        state.report = Some(empty_report());
        state.name = "Project package".into();
        state.draft.as_mut().unwrap().revision = "B".into();
        assert!(state.edited_fields());
        state.apply_fields().unwrap();
        let updated = state.set.clone().unwrap();
        assert_eq!(updated.files, original.files);
        assert_eq!(updated.id, original.id);
        assert_eq!(updated.sheets[0].id, original.sheets[0].id);
        assert_eq!(updated.sheets[0].page, original.sheets[0].page);
        assert_eq!(updated.sheets[0].revision, "B");
        assert_eq!(updated.sheets[1], original.sheets[1]);
        assert!(state.dirty && !state.edited_fields() && state.report.is_none());
        state.draft.as_mut().unwrap().number.clear();
        assert!(state.apply_fields().is_err());
        assert_eq!(state.set.as_ref(), Some(&updated));
        assert!(
            state.edited_fields(),
            "invalid text remains available for correction"
        );
    }

    #[test]
    fn cancellation_discards_old_results_without_losing_the_manifest_or_new_job() {
        let (mut app, _, _) = test_app();
        let ctx = egui::Context::default();
        let original = sample();
        app.plan_sets
            .replace(original.clone(), PathBuf::from("set.json"), true);
        let first = app.plan_sets.begin(PendingKind::Read);
        let cancel = app.plan_sets.cancel.clone();
        app.plan_sets.cancel_read();
        assert!(cancel.load(Ordering::Relaxed));
        let second = app.plan_sets.begin(PendingKind::Read);
        app.on_plan_set_result(
            &ctx,
            first,
            Ok(Outcome::Loaded {
                set: PlanSet::new("Stale response").unwrap(),
                path: "wrong.json".into(),
                report: empty_report(),
            }),
        );
        app.on_plan_set_progress(first, 50, 50);
        assert_eq!(app.plan_sets.pending.map(|p| p.0), Some(second));
        assert_eq!(app.plan_sets.progress, (0, 0));
        assert_eq!(app.plan_sets.set.as_ref(), Some(&original));
        app.on_plan_set_result(&ctx, second, Err(EngineError::PasswordRequired));
        assert!(!app.plan_sets.busy() && app.plan_sets.dirty);
        assert_eq!(app.plan_sets.set.as_ref(), Some(&original));
        assert_eq!(
            app.plan_sets.error.as_deref(),
            Some(tr(app.lang(), "plan_sets.password_unavailable"))
        );
        let save = app.plan_sets.begin(PendingKind::Save);
        app.plan_sets.cancel_read();
        assert_eq!(
            app.plan_sets.pending.map(|p| p.0),
            Some(save),
            "atomic save must finish before closing"
        );
    }

    fn verified(path: PathBuf, page: usize) -> Navigation {
        Navigation {
            sheet: VerifiedSheet {
                path,
                page,
                bytes: b"already inspected by the worker".to_vec(),
                sha256: "b".repeat(64),
            },
            password: None,
            page_count: 2,
            active_source: None,
        }
    }

    #[test]
    fn navigating_an_existing_dirty_tab_keeps_its_memory_history_and_never_reopens_disk() {
        let (mut app, requests, _) = test_app();
        app.dirty = true;
        app.can_undo = true;
        let doc = app.document.as_mut().unwrap();
        doc.info.page_sizes.push(doc.info.page_sizes[0]);
        let path = doc.info.path.clone();
        app.plan_sets.open = true;
        app.navigate_plan_sheet(verified(path.clone(), 2));
        assert!(app.plan_sets.open && app.plan_sets.navigation_confirmation.is_some());
        assert_eq!(
            app.document.as_ref().unwrap().page,
            0,
            "dirty tabs require an explicit choice before a page jump"
        );
        app.confirm_plan_navigation(true);
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert_eq!(app.document.as_ref().unwrap().info.path, path);
        assert!(app.dirty && app.can_undo && !app.plan_sets.open);
        assert_eq!(app.plan_sets.message, Some("plan_sets.opened_dirty"));
        assert!(!requests.try_iter().any(|r| matches!(
            r,
            Request::Open { .. } | Request::OpenPlanSheet(_) | Request::Resume(_)
        )));

        app.navigate_plan_sheet(verified(path, 3));
        assert!(
            !app.plan_sets
                .navigation_confirmation
                .as_ref()
                .unwrap()
                .allow_existing
        );
        app.confirm_plan_navigation(true);
        assert!(
            app.plan_sets.open,
            "missing in-memory page must not be silently clamped"
        );
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(app.dirty && app.can_undo);
    }

    #[test]
    fn inactive_dirty_tab_is_resumed_and_navigation_finishes_only_after_its_response() {
        let (mut app, requests, _) = test_app();
        let (mut other, _, _) = test_app();
        let path = PathBuf::from("test/second.pdf");
        let doc = other.document.as_mut().unwrap();
        doc.info.path = path.clone();
        doc.info.page_sizes.push(doc.info.page_sizes[0]);
        let info = doc.info.clone();
        other.dirty = true;
        other.can_undo = true;
        app.tabs.tabs.push(crate::tabs::Tab {
            path: path.clone(),
            dirty: true,
            recovery_id: 99,
            ui: Some(other.take_tab()),
            doc: None,
        });
        app.plan_sets.open = true;
        app.navigate_plan_sheet(verified(path.clone(), 2));
        assert!(app.plan_sets.navigation_confirmation.is_some());
        app.confirm_plan_navigation(true);
        assert_eq!(app.tabs.resuming, Some(1));
        let requests: Vec<_> = requests.try_iter().collect();
        assert!(requests.iter().any(|r| matches!(r, Request::Resume(1))));
        assert!(!requests
            .iter()
            .any(|r| matches!(r, Request::Open { .. } | Request::OpenPlanSheet(_))));
        app.on_resumed(
            DocState {
                info,
                can_undo: true,
                can_redo: true,
                modified: true,
                pages_unchanged: true,
                measurement_scales_unchanged: true,
                has_signatures: false,
            },
            &egui::Context::default(),
        );
        assert_eq!(app.document.as_ref().unwrap().info.path, path);
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(app.dirty && app.can_undo && app.can_redo);
        assert!(app.plan_sets.navigation.is_none());
    }

    #[test]
    fn new_sheet_opens_verified_bytes_and_suspension_failure_preserves_the_draft() {
        let (mut app, requests, responses) = test_app();
        let set = sample();
        app.plan_sets.replace(set.clone(), "set.json".into(), true);
        app.plan_sets.open = true;
        let payload = verified("test/new.pdf".into(), 1);
        let expected = payload.sheet.bytes.clone();
        app.navigate_plan_sheet(payload);
        let requests: Vec<_> = requests.try_iter().collect();
        assert!(requests
            .iter()
            .any(|r| matches!(r, Request::OpenPlanSheet(navigation) if navigation.sheet.bytes == expected)));
        assert!(!requests.iter().any(|r| matches!(r, Request::Open { .. })));
        responses
            .send(Response::SuspendFailed(EngineError::InvalidFormat))
            .unwrap();
        app.handle_responses(&egui::Context::default());
        assert!(app.plan_sets.navigation.is_none());
        assert!(app.plan_sets.open && app.plan_sets.dirty);
        assert_eq!(app.plan_sets.set.as_ref(), Some(&set));
    }

    fn contains_text(shape: &egui::Shape, text: &str) -> bool {
        match shape {
            egui::Shape::Text(value) => value.galley.text() == text,
            egui::Shape::Vec(shapes) => shapes.iter().any(|s| contains_text(s, text)),
            _ => false,
        }
    }

    #[test]
    fn clean_tab_requires_the_same_saved_hash_and_changed_page_counts_cannot_jump() {
        let (mut app, requests, _) = test_app();
        let doc = app.document.as_mut().unwrap();
        doc.info.page_sizes.push(doc.info.page_sizes[0]);
        let path = doc.info.path.clone();
        let mut candidate = verified(path.clone(), 2);
        candidate.active_source = Some((path.clone(), "c".repeat(64)));
        app.navigate_plan_sheet(candidate);
        assert!(app.plan_sets.navigation_confirmation.is_some());
        assert!(
            !app.plan_sets
                .navigation_confirmation
                .as_ref()
                .unwrap()
                .allow_existing
        );
        assert_eq!(app.document.as_ref().unwrap().page, 0);
        assert!(!requests
            .try_iter()
            .any(|r| matches!(r, Request::OpenPlanSheet(_))));
        app.plan_sets.navigation_confirmation = None;
        let mut candidate = verified(path.clone(), 2);
        candidate.active_source = Some((path.clone(), "b".repeat(64)));
        app.navigate_plan_sheet(candidate);
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert!(app.plan_sets.navigation_confirmation.is_none());
        app.dirty = true;
        app.document.as_mut().unwrap().info.page_sizes.pop();
        app.navigate_plan_sheet(verified(path, 1));
        assert!(
            !app.plan_sets
                .navigation_confirmation
                .as_ref()
                .unwrap()
                .allow_existing
        );
        app.confirm_plan_navigation(false);
        assert!(
            requests
                .try_iter()
                .any(|r| matches!(r, Request::OpenPlanSheet(_))),
            "open a verified copy without replacing the dirty tab"
        );
        assert!(app.tabs.tabs[0].dirty);
        assert!(app.tabs.tabs[0].ui.as_ref().unwrap().dirty);
    }

    #[test]
    fn one_use_password_is_consumed_by_the_job_and_never_enters_the_manifest() {
        let (mut app, requests, _) = test_app();
        let set = sample();
        app.plan_sets.replace(set.clone(), "set.json".into(), true);
        app.plan_sets.use_password = true;
        app.plan_sets.password = Zeroizing::new(["temporary", "fixture", "credential"].join("-"));
        let expected = app.plan_sets.password.to_string();
        app.start_plan_set_job(PendingKind::Read, |cancel, password| Job::Inspect {
            set,
            path: "set.json".into(),
            password,
            cancel,
        });
        assert!(!app.plan_sets.use_password && app.plan_sets.password.is_empty());
        let request = requests.try_recv().unwrap();
        let Request::PlanSet {
            job: Job::Inspect { password, set, .. },
            ..
        } = request
        else {
            panic!("expected an inspection job")
        };
        assert_eq!(
            password.as_ref().map(|p| p.as_str()),
            Some(expected.as_str())
        );
        assert!(!String::from_utf8(core::encode(&set).unwrap())
            .unwrap()
            .contains(&expected));
        app.plan_sets.cancel_read();
        assert!(app.plan_sets.password.is_empty());
        let path = app.document.as_ref().unwrap().info.path.clone();
        let mut candidate = verified(path.clone(), 1);
        candidate.password = Some(Zeroizing::new(expected));
        candidate.active_source = Some((path, "c".repeat(64)));
        app.navigate_plan_sheet(candidate);
        assert!(app
            .plan_sets
            .navigation_confirmation
            .as_ref()
            .unwrap()
            .candidate
            .password
            .is_some());
        app.plan_sets.navigation_confirmation = None;
        assert!(
            !requests
                .try_iter()
                .any(|r| matches!(r, Request::Open { .. })),
            "protected plan navigation must never fall back to re-reading a path through the generic password dialog"
        );
    }

    #[test]
    fn plan_set_tool_is_reachable_without_a_pdf_and_escape_keeps_unsaved_metadata() {
        let home = crate::workspace::TABLE
            .iter()
            .find(|info| info.contains(Command::PlanSets))
            .unwrap();
        assert_eq!(home.ws, crate::workspace::Workspace::Organize);
        assert!(!Command::PlanSets.info().needs_document);
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            app.settings.set_language(lang);
            app.document = None;
            let ctx = egui::Context::default();
            app.execute(&ctx, Command::PlanSets);
            assert!(app.plan_sets.open && app.dialog_open() && app.navigation_blocked());
            app.plan_sets.replace(sample(), "set.json".into(), true);
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720.0, 480.0),
                        )),
                        time: Some(1.0 + frame as f64 * 0.025),
                        ..Default::default()
                    },
                    |ui| app.plan_sets_dialog(ui.ctx()),
                );
                output.textures_delta.clear();
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    time: Some(1.1),
                    ..Default::default()
                },
                |ui| app.plan_sets_dialog(ui.ctx()),
            );
            output.textures_delta.clear();
            for key in ["plan_sets.title", "plan_sets.save_copy", "dialog.close"] {
                assert!(
                    output
                        .shapes
                        .iter()
                        .any(|s| contains_text(&s.shape, tr(lang, key))),
                    "missing {key} in {}",
                    lang.code()
                );
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| app.plan_sets_dialog(ui.ctx()),
            );
            output.textures_delta.clear();
            assert!(!app.plan_sets.open && app.plan_sets.dirty && app.plan_sets.set.is_some());
            assert!(requests
                .try_iter()
                .all(|r| !matches!(r, Request::PlanSet { .. })));
            app.execute(&ctx, Command::Quit);
            assert!(
                app.plan_sets.open && app.plan_sets.quit_requested,
                "Quit must not discard the manifest draft"
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingKind {
    Read,
    Save,
}

#[derive(Clone, Copy)]
enum Replacement {
    New,
    Open,
}

#[derive(Clone, PartialEq)]
struct SheetDraft {
    id: String,
    number: String,
    title: String,
    discipline: String,
    revision: String,
}

impl From<&PlanSheet> for SheetDraft {
    fn from(sheet: &PlanSheet) -> Self {
        Self {
            id: sheet.id.clone(),
            number: sheet.number.clone(),
            title: sheet.title.clone(),
            discipline: sheet.discipline.clone(),
            revision: sheet.revision.clone(),
        }
    }
}

pub struct PlanSetState {
    pub open: bool,
    set: Option<PlanSet>,
    path: Option<PathBuf>,
    name: String,
    selected: Option<String>,
    draft: Option<SheetDraft>,
    dirty: bool,
    report: Option<PlanSetReport>,
    generation: u64,
    pending: Option<(u64, PendingKind)>,
    cancel: Arc<AtomicBool>,
    progress: (usize, usize),
    error: Option<String>,
    message: Option<&'static str>,
    replacement: Option<Replacement>,
    refresh_confirmation: Option<bool>,
    quit_requested: bool,
    use_password: bool,
    password: Zeroizing<String>,
    page: usize,
    navigation: Option<(PathBuf, usize)>,
    navigation_confirmation: Option<PendingNavigation>,
}

impl Default for PlanSetState {
    fn default() -> Self {
        Self {
            open: false,
            set: None,
            path: None,
            name: String::new(),
            selected: None,
            draft: None,
            dirty: false,
            report: None,
            generation: 0,
            pending: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: (0, 0),
            error: None,
            message: None,
            replacement: None,
            refresh_confirmation: None,
            quit_requested: false,
            use_password: false,
            password: Zeroizing::new(String::new()),
            page: 0,
            navigation: None,
            navigation_confirmation: None,
        }
    }
}

impl PlanSetState {
    pub(crate) fn busy(&self) -> bool {
        self.pending.is_some()
    }

    fn edited_fields(&self) -> bool {
        let Some(set) = &self.set else { return false };
        self.name != set.name
            || self.draft.as_ref().is_some_and(|draft| {
                set.sheets
                    .iter()
                    .find(|sheet| sheet.id == draft.id)
                    .is_none_or(|sheet| *draft != SheetDraft::from(sheet))
            })
    }

    fn select(&mut self, id: Option<String>) {
        self.selected = id;
        self.draft = self
            .set
            .as_ref()
            .and_then(|set| {
                set.sheets
                    .iter()
                    .find(|sheet| Some(&sheet.id) == self.selected.as_ref())
            })
            .map(SheetDraft::from);
    }

    fn replace(&mut self, set: PlanSet, path: PathBuf, dirty: bool) {
        self.quit_requested = false;
        self.replacement = None;
        self.refresh_confirmation = None;
        self.error = None;
        self.message = None;
        self.clear_password();
        self.navigation_confirmation = None;
        self.name = set.name.clone();
        let selection = set.sheets.first().map(|sheet| sheet.id.clone());
        self.set = Some(set);
        self.path = Some(path);
        self.dirty = dirty;
        self.report = None;
        self.page = 0;
        self.select(selection);
    }

    fn begin(&mut self, kind: PendingKind) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some((self.generation, kind));
        self.cancel = Arc::new(AtomicBool::new(false));
        self.progress = (0, 0);
        self.error = None;
        self.message = None;
        self.generation
    }

    fn cancel_read(&mut self) {
        self.clear_password();
        if self
            .pending
            .is_some_and(|(_, kind)| kind == PendingKind::Read)
        {
            self.cancel.store(true, Ordering::Relaxed);
            self.generation = self.generation.wrapping_add(1);
            self.pending = None;
            self.message = Some("plan_sets.cancelled");
        }
    }

    fn clear_password(&mut self) {
        self.password = Zeroizing::new(String::new());
        self.use_password = false;
    }

    fn take_password(&mut self) -> Password {
        let password = self
            .use_password
            .then(|| std::mem::replace(&mut self.password, Zeroizing::new(String::new())));
        self.clear_password();
        password
    }

    fn apply_fields(&mut self) -> Result<(), EngineError> {
        let Some(set) = &self.set else { return Ok(()) };
        let mut candidate = set.clone();
        candidate.name = self.name.trim().to_owned();
        if let Some(draft) = &self.draft {
            let sheet = candidate
                .sheets
                .iter_mut()
                .find(|s| s.id == draft.id)
                .ok_or_else(|| EngineError::Other("sheet selection changed".into()))?;
            sheet.number = draft.number.trim().to_owned();
            sheet.title = draft.title.trim().to_owned();
            sheet.discipline = draft.discipline.trim().to_owned();
            sheet.revision = draft.revision.trim().to_owned();
        }
        core::validate(&candidate)?;
        self.name = candidate.name.clone();
        self.set = Some(candidate);
        self.select(self.selected.clone());
        self.dirty = true;
        self.report = None;
        self.error = None;
        self.message = Some("plan_sets.fields_applied");
        Ok(())
    }

    pub(crate) fn navigation_failed(&mut self) {
        if self.navigation.take().is_some() {
            self.open = true;
            self.message = Some("plan_sets.navigation_failed");
        }
    }
}

fn status_key(status: &ReferenceStatus) -> &'static str {
    match status {
        ReferenceStatus::Ready => "plan_sets.status.ready",
        ReferenceStatus::Missing => "plan_sets.status.missing",
        ReferenceStatus::Changed => "plan_sets.status.changed",
        ReferenceStatus::Unreadable => "plan_sets.status.unreadable",
        ReferenceStatus::PasswordRequired => "plan_sets.status.password",
        ReferenceStatus::UnsafePath => "plan_sets.status.unsafe",
        ReferenceStatus::PageOutOfRange => "plan_sets.status.page",
        ReferenceStatus::Duplicate => "plan_sets.status.duplicate",
        ReferenceStatus::Cancelled => "plan_sets.status.cancelled",
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let b = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

impl SuniApp {
    pub(crate) fn open_plan_sets(&mut self) {
        if self.navigation_blocked() || self.worker.has_pending_mutations() {
            return;
        }
        self.plan_sets.open = true;
    }

    pub(crate) fn guard_plan_set_quit(&mut self) -> bool {
        if self.plan_sets.busy() {
            self.plan_sets.cancel_read();
            if self.plan_sets.busy() {
                self.plan_sets.open = true;
                self.plan_sets.quit_requested = true;
                return true;
            }
        }
        if self.plan_sets.dirty || self.plan_sets.edited_fields() {
            self.plan_sets.open = true;
            self.plan_sets.quit_requested = true;
            true
        } else {
            self.plan_sets.open = false;
            false
        }
    }

    fn start_plan_set_job(
        &mut self,
        kind: PendingKind,
        make: impl FnOnce(Arc<AtomicBool>, Password) -> Job,
    ) {
        if self.plan_sets.busy() || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        let generation = self.plan_sets.begin(kind);
        let password = if kind == PendingKind::Read {
            self.plan_sets.take_password()
        } else {
            None
        };
        let job = make(self.plan_sets.cancel.clone(), password);
        self.worker.send(Request::PlanSet { generation, job });
    }

    pub(crate) fn on_plan_set_progress(&mut self, generation: u64, done: usize, total: usize) {
        if self
            .plan_sets
            .pending
            .is_some_and(|(id, _)| id == generation)
        {
            self.plan_sets.progress = (done, total);
        }
    }

    pub(crate) fn on_plan_set_result(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if !self
            .plan_sets
            .pending
            .is_some_and(|(id, _)| id == generation)
        {
            return;
        }
        self.plan_sets.pending = None;
        match result {
            Ok(Outcome::Loaded { set, path, report }) => {
                self.plan_sets.replace(set, path, false);
                self.plan_sets.report = Some(report);
            }
            Ok(Outcome::Changed { set, report }) => {
                let selected = self.plan_sets.selected.clone();
                self.plan_sets.set = Some(set);
                let selected = selected
                    .filter(|id| {
                        self.plan_sets
                            .set
                            .as_ref()
                            .is_some_and(|set| set.sheets.iter().any(|sheet| &sheet.id == id))
                    })
                    .or_else(|| {
                        self.plan_sets
                            .set
                            .as_ref()
                            .and_then(|set| set.sheets.first().map(|s| s.id.clone()))
                    });
                self.plan_sets.select(selected);
                self.plan_sets.dirty = true;
                self.plan_sets.report = Some(report);
            }
            Ok(Outcome::Inspected(report)) => self.plan_sets.report = Some(report),
            Ok(Outcome::Saved { set, path }) => {
                let selected = self.plan_sets.selected.clone();
                let quit_requested = self.plan_sets.quit_requested;
                self.plan_sets.replace(set, path, false);
                self.plan_sets.select(selected);
                self.plan_sets.message = Some("plan_sets.saved");
                if quit_requested {
                    self.plan_sets.open = false;
                    self.execute(ctx, sunipdf_core::Command::Quit);
                }
            }
            Ok(Outcome::Navigate(sheet)) => self.navigate_plan_sheet(sheet),
            Err(EngineError::Cancelled) => self.plan_sets.message = Some("plan_sets.cancelled"),
            Err(EngineError::PasswordRequired) => {
                self.plan_sets.error =
                    Some(tr(self.lang(), "plan_sets.password_unavailable").into());
            }
            Err(error) => self.plan_sets.error = Some(error.to_string()),
        }
    }

    pub(crate) fn navigate_plan_sheet(&mut self, navigation: Navigation) {
        let sheet = &navigation.sheet;
        self.plan_sets.open = false;
        if self.navigation_blocked()
            || self.worker.has_pending_mutations()
            || self.document_transition_pending()
            || self.opening.is_some()
            || self.tabs.resuming.is_some()
            || self.tabs.suspending.is_some()
        {
            self.plan_sets.open = true;
            self.plan_sets.message = Some("plan_sets.navigation_busy");
            return;
        }
        let mut target = NavigationTarget::New;
        let mut known_match = false;
        let mut dirty = false;
        let mut page_count = 0;
        if self
            .document
            .as_ref()
            .is_some_and(|doc| same_path(&doc.info.path, &sheet.path))
        {
            target = NavigationTarget::Active;
            dirty = self.dirty;
            page_count = self.document.as_ref().unwrap().info.page_count();
            known_match = navigation
                .active_source
                .as_ref()
                .is_some_and(|(path, hash)| {
                    same_path(path, &sheet.path) && hash.eq_ignore_ascii_case(&sheet.sha256)
                });
        } else if let Some(index) = self
            .tabs
            .tabs
            .iter()
            .position(|tab| same_path(&tab.path, &sheet.path))
        {
            target = NavigationTarget::Tab(index);
            let tab = &self.tabs.tabs[index];
            dirty = tab.dirty || tab.doc.as_ref().is_some_and(|doc| doc.modified);
            page_count = tab
                .ui
                .as_ref()
                .and_then(|ui| ui.document.as_ref())
                .map_or(0, |doc| doc.info.page_count());
            known_match = tab
                .doc
                .as_ref()
                .and_then(|doc| doc.signature_source.as_ref())
                .is_some_and(|source| source.matches_sha256(&sheet.sha256));
        }
        if !matches!(target, NavigationTarget::New)
            && (dirty || !known_match || page_count != navigation.page_count)
        {
            let allow_existing = dirty
                && page_count == navigation.page_count
                && sheet.page > 0
                && sheet.page <= page_count;
            self.plan_sets.navigation_confirmation = Some(PendingNavigation {
                candidate: navigation,
                target,
                allow_existing,
                epoch: self.view.epoch,
            });
            self.plan_sets.open = true;
            return;
        }
        self.perform_plan_navigation(navigation, target);
    }

    fn perform_plan_navigation(&mut self, navigation: Navigation, target: NavigationTarget) {
        self.plan_sets.open = false;
        if self.navigation_blocked()
            || self.worker.has_pending_mutations()
            || self.opening.is_some()
            || self.tabs.resuming.is_some()
            || self.tabs.suspending.is_some()
        {
            self.plan_sets.open = true;
            self.plan_sets.message = Some("plan_sets.navigation_busy");
            return;
        }
        let sheet = &navigation.sheet;
        self.plan_sets.navigation = Some((sheet.path.clone(), sheet.page.saturating_sub(1)));
        match target {
            NavigationTarget::Active => {
                self.finish_plan_set_navigation();
                return;
            }
            NavigationTarget::Tab(index) => {
                self.switch_to(index);
                if self.tabs.resuming != Some(index) {
                    self.plan_sets.navigation_failed();
                }
                return;
            }
            NavigationTarget::New => {}
        }
        if !self.suspend_active() {
            self.plan_sets.navigation_failed();
            return;
        }
        self.opening = Some(sheet.path.clone());
        self.worker.send(Request::OpenPlanSheet(navigation));
    }

    fn confirm_plan_navigation(&mut self, use_existing: bool) {
        let Some(pending) = self.plan_sets.navigation_confirmation.take() else {
            return;
        };
        if pending.epoch != self.view.epoch || (use_existing && !pending.allow_existing) {
            self.plan_sets.message = Some("plan_sets.navigation_failed");
            return;
        }
        let target = if use_existing {
            pending.target
        } else {
            NavigationTarget::New
        };
        self.perform_plan_navigation(pending.candidate, target);
    }

    pub(crate) fn finish_plan_set_navigation(&mut self) {
        let Some((path, page)) = self.plan_sets.navigation.take() else {
            return;
        };
        if !self
            .document
            .as_ref()
            .is_some_and(|doc| same_path(&doc.info.path, &path) && page < doc.info.page_count())
        {
            self.plan_sets.open = true;
            self.plan_sets.message = Some("plan_sets.navigation_failed");
            return;
        }
        self.go_to_page(page);
        self.plan_sets.message = Some(if self.dirty {
            "plan_sets.opened_dirty"
        } else {
            "plan_sets.opened"
        });
        if self.dirty {
            self.status_msg = Some(tr(self.lang(), "plan_sets.opened_dirty").into());
        }
    }

    pub(crate) fn plan_set_navigation_failed(&mut self, path: &Path) -> bool {
        let failed = self
            .plan_sets
            .navigation
            .as_ref()
            .is_some_and(|(expected, _)| same_path(expected, path));
        if failed {
            self.plan_sets.navigation_failed();
        }
        failed
    }

    fn replace_plan_set(&mut self, action: Replacement) {
        match action {
            Replacement::New => {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title(tr(self.lang(), "plan_sets.choose_root"))
                    .pick_folder()
                {
                    match PlanSet::new(tr(self.lang(), "plan_sets.untitled")) {
                        Ok(set) => self
                            .plan_sets
                            .replace(set, folder.join("plan-set.json"), true),
                        Err(error) => self.plan_sets.error = Some(error.to_string()),
                    }
                }
            }
            Replacement::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(self.lang(), "plan_sets.manifest"), &["json"])
                    .pick_file()
                {
                    self.start_plan_set_job(PendingKind::Read, |cancel, password| Job::Open {
                        path,
                        password,
                        cancel,
                    });
                }
            }
        }
    }

    pub(crate) fn plan_sets_dialog(&mut self, ctx: &egui::Context) {
        if !self.plan_sets.open {
            return;
        }
        let lang = self.lang();
        let busy = self.plan_sets.busy();
        let saving = self
            .plan_sets
            .pending
            .is_some_and(|(_, kind)| kind == PendingKind::Save);
        let ready = !busy
            && self.plan_sets.navigation_confirmation.is_none()
            && matches!(self.engine, EngineState::Ready);
        let fields_edited = self.plan_sets.edited_fields();
        let has_set = self.plan_sets.set.is_some();
        let has_selection = self.plan_sets.selected.is_some();
        let (mut replacement, mut add, mut inspect, mut save, mut navigate, mut close, mut cancel) =
            (None, false, false, false, false, false, false);
        let (mut apply, mut reset, mut remove, mut refresh, mut confirmed_refresh) =
            (false, false, false, None, None);
        let mut select = None;
        let mut discard = false;
        let mut discard_quit = false;
        let mut navigate_existing = None;
        let mut cancel_navigation = false;
        dialog::show(
            ctx,
            Spec::new(
                "plan_sets",
                tr(lang, "plan_sets.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .icon(crate::icons::glyph::PAGES, "▦")
            .width(900.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "plan_sets.help"));
                if let Some(pending) = &self.plan_sets.navigation_confirmation {
                    ui.strong(tr(lang, "plan_sets.existing_tab_warning"));
                    ui.label(tr_args(
                        lang,
                        "plan_sets.existing_page",
                        &[("page", &pending.candidate.sheet.page.to_string())],
                    ));
                    if !pending.allow_existing {
                        ui.label(tr(lang, "plan_sets.existing_unavailable"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                pending.allow_existing,
                                egui::Button::new(tr(lang, "plan_sets.use_existing")),
                            )
                            .clicked()
                        {
                            navigate_existing = Some(true);
                        }
                        if ui
                            .button(tr(lang, "plan_sets.open_verified_copy"))
                            .clicked()
                        {
                            navigate_existing = Some(false);
                        }
                        cancel_navigation = ui.button(tr(lang, "dialog.password.cancel")).clicked();
                    });
                }
                ui.small(tr(lang, "plan_sets.password_help"));
                ui.add_enabled_ui(ready, |ui| {
                    ui.checkbox(
                        &mut self.plan_sets.use_password,
                        tr(lang, "plan_sets.use_password"),
                    );
                    if self.plan_sets.use_password {
                        ui.label(tr(lang, "dialog.password.label"));
                        ui.add(
                            egui::TextEdit::singleline(&mut *self.plan_sets.password)
                                .password(true)
                                .char_limit(256)
                                .desired_width(f32::INFINITY),
                        );
                    } else if !self.plan_sets.password.is_empty() {
                        self.plan_sets.clear_password();
                    }
                });
                if self.plan_sets.quit_requested && !busy {
                    ui.label(tr(lang, "plan_sets.quit_warning"));
                    ui.horizontal_wrapped(|ui| {
                        discard_quit = ui.button(tr(lang, "plan_sets.discard_quit")).clicked();
                        if ui.button(tr(lang, "plan_sets.keep")).clicked() {
                            self.plan_sets.quit_requested = false;
                        }
                    });
                }
                ui.add_enabled_ui(ready, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr(lang, "plan_sets.new")).clicked() {
                            replacement = Some(Replacement::New);
                        }
                        if ui.button(tr(lang, "plan_sets.open")).clicked() {
                            replacement = Some(Replacement::Open);
                        }
                        add = ui
                            .add_enabled(
                                self.plan_sets.set.is_some() && !fields_edited,
                                egui::Button::new(tr(lang, "plan_sets.add")),
                            )
                            .clicked();
                        inspect = ui
                            .add_enabled(
                                has_set && !fields_edited,
                                egui::Button::new(tr(lang, "plan_sets.validate")),
                            )
                            .clicked();
                        navigate = ui
                            .add_enabled(
                                has_selection && !fields_edited,
                                egui::Button::new(tr(lang, "plan_sets.open_sheet")),
                            )
                            .clicked();
                    });
                });
                if self.plan_sets.replacement.is_some() {
                    ui.label(tr(lang, "plan_sets.discard_warning"));
                    ui.horizontal_wrapped(|ui| {
                        discard = ui.button(tr(lang, "plan_sets.discard_continue")).clicked();
                        if ui.button(tr(lang, "plan_sets.keep")).clicked() {
                            self.plan_sets.replacement = None;
                        }
                    });
                }
                if let Some(path) = &self.plan_sets.path {
                    ui.add(egui::Label::new(path.display().to_string()).truncate())
                        .on_hover_text(path.display().to_string());
                }
                if let Some(set) = self.plan_sets.set.as_ref() {
                    ui.add_enabled_ui(ready, |ui| {
                        ui.label(tr(lang, "plan_sets.name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.plan_sets.name)
                                .char_limit(128)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.small(tr_args(
                        lang,
                        "plan_sets.counts",
                        &[
                            ("files", &set.files.len().to_string()),
                            ("sheets", &set.sheets.len().to_string()),
                        ],
                    ));
                    ui.label(tr(
                        lang,
                        if self.plan_sets.dirty {
                            "plan_sets.unsaved"
                        } else {
                            "plan_sets.saved_state"
                        },
                    ));
                    if self.plan_sets.report.is_none() {
                        ui.small(tr(lang, "plan_sets.unvalidated"));
                    } else {
                        ui.small(tr(lang, "plan_sets.checked_snapshot"));
                    }
                    if let Some(report) = &self.plan_sets.report {
                        ui.label(tr(
                            lang,
                            if report.complete() {
                                "plan_sets.all_ready"
                            } else {
                                "plan_sets.has_issues"
                            },
                        ));
                        ui.collapsing(tr(lang, "plan_sets.file_checks"), |ui| {
                            for file in &report.files {
                                let name = set
                                    .files
                                    .iter()
                                    .find(|f| f.id == file.file_id)
                                    .map_or(file.file_id.as_str(), |f| f.path.as_str());
                                ui.label(format!(
                                    "{} — {}",
                                    name,
                                    tr(lang, status_key(&file.status))
                                ));
                                if let Some(message) = &file.message {
                                    ui.small(message);
                                }
                                if let Some(hash) = &file.actual_sha256 {
                                    ui.add(egui::Label::new(format!("SHA-256: {hash}")).truncate())
                                        .on_hover_text(hash);
                                }
                            }
                        });
                    }
                    let max_page = set.sheets.len().saturating_sub(1) / 50;
                    self.plan_sets.page = self.plan_sets.page.min(max_page);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                ready && self.plan_sets.page > 0 && !fields_edited,
                                egui::Button::new("←"),
                            )
                            .on_hover_text(tr(lang, "plan_sets.previous"))
                            .clicked()
                        {
                            self.plan_sets.page -= 1;
                        }
                        ui.label(format!("{} / {}", self.plan_sets.page + 1, max_page + 1));
                        if ui
                            .add_enabled(
                                ready && self.plan_sets.page < max_page && !fields_edited,
                                egui::Button::new("→"),
                            )
                            .on_hover_text(tr(lang, "plan_sets.next"))
                            .clicked()
                        {
                            self.plan_sets.page += 1;
                        }
                    });
                    egui::ScrollArea::both()
                        .id_salt("plan_set_sheets")
                        .max_height(210.0)
                        .show(ui, |ui| {
                            egui::Grid::new("plan_set_sheet_grid")
                                .striped(true)
                                .min_col_width(70.0)
                                .show(ui, |ui| {
                                    for key in [
                                        "plan_sets.number",
                                        "plan_sets.sheet_title",
                                        "plan_sets.discipline",
                                        "plan_sets.revision",
                                        "plan_sets.page",
                                        "plan_sets.status",
                                    ] {
                                        ui.strong(tr(lang, key));
                                    }
                                    ui.end_row();
                                    for sheet in
                                        set.sheets.iter().skip(self.plan_sets.page * 50).take(50)
                                    {
                                        let selected =
                                            self.plan_sets.selected.as_deref() == Some(&sheet.id);
                                        if ui
                                            .add_enabled(
                                                ready && !fields_edited,
                                                egui::Button::selectable(selected, &sheet.number),
                                            )
                                            .clicked()
                                        {
                                            select = Some(sheet.id.clone());
                                        }
                                        ui.add(egui::Label::new(&sheet.title).truncate())
                                            .on_hover_text(&sheet.title);
                                        ui.label(&sheet.discipline);
                                        ui.label(&sheet.revision);
                                        ui.label(sheet.page.to_string());
                                        if let Some(row) =
                                            self.plan_sets.report.as_ref().and_then(|r| {
                                                r.sheets.iter().find(|r| r.sheet_id == sheet.id)
                                            })
                                        {
                                            ui.label(tr(lang, status_key(&row.status)))
                                                .on_hover_text(row.issues.join("\n"));
                                        } else {
                                            ui.label(tr(lang, "plan_sets.status.unchecked"));
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    ui.add_enabled_ui(ready, |ui| {
                        if let Some(draft) = &mut self.plan_sets.draft {
                            ui.separator();
                            ui.strong(tr(lang, "plan_sets.edit_sheet"));
                            for (key, value, limit) in [
                                ("plan_sets.number", &mut draft.number, 128),
                                ("plan_sets.sheet_title", &mut draft.title, 256),
                                ("plan_sets.discipline", &mut draft.discipline, 128),
                                ("plan_sets.revision", &mut draft.revision, 128),
                            ] {
                                ui.label(tr(lang, key));
                                ui.add(
                                    egui::TextEdit::singleline(value)
                                        .char_limit(limit)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                            if let Some(sheet) = set.sheets.iter().find(|s| s.id == draft.id) {
                                if let Some(file) = set.files.iter().find(|f| f.id == sheet.file_id)
                                {
                                    ui.small(&file.path).on_hover_text(format!(
                                        "{}\nSHA-256: {}",
                                        file.path, file.sha256
                                    ));
                                }
                            }
                        }
                        ui.horizontal_wrapped(|ui| {
                            apply = ui
                                .add_enabled(
                                    fields_edited,
                                    egui::Button::new(tr(lang, "plan_sets.apply_fields")),
                                )
                                .clicked();
                            reset = ui
                                .add_enabled(
                                    fields_edited,
                                    egui::Button::new(tr(lang, "plan_sets.reset_fields")),
                                )
                                .clicked();
                            remove = ui
                                .add_enabled(
                                    self.plan_sets.selected.is_some() && !fields_edited,
                                    egui::Button::new(tr(lang, "plan_sets.remove_sheet")),
                                )
                                .clicked();
                        });
                        if fields_edited {
                            ui.small(tr(lang, "plan_sets.pending_fields"));
                        }
                        ui.horizontal_wrapped(|ui| {
                            let enabled = self.plan_sets.selected.is_some() && !fields_edited;
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(tr(lang, "plan_sets.refresh_file")),
                                )
                                .clicked()
                            {
                                refresh = Some(false);
                            }
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(tr(lang, "plan_sets.relink_file")),
                                )
                                .clicked()
                            {
                                refresh = Some(true);
                            }
                        });
                    });
                    if let Some(relink) = self.plan_sets.refresh_confirmation {
                        ui.label(tr(lang, "plan_sets.refresh_warning"));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(tr(lang, "plan_sets.accept_file")).clicked() {
                                confirmed_refresh = Some(relink);
                            }
                            if ui.button(tr(lang, "plan_sets.keep")).clicked() {
                                self.plan_sets.refresh_confirmation = None;
                            }
                        });
                    }
                }
                if busy {
                    ui.spinner();
                    ui.label(tr_args(
                        lang,
                        "plan_sets.progress",
                        &[
                            ("done", &self.plan_sets.progress.0.to_string()),
                            ("total", &self.plan_sets.progress.1.to_string()),
                        ],
                    ));
                }
                if let Some(error) = &self.plan_sets.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = self.plan_sets.message {
                    ui.label(tr(lang, message));
                }
            },
            |ui| {
                if busy {
                    if !saving {
                        cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                            || ui.input(|i| i.key_pressed(egui::Key::Escape));
                    } else {
                        ui.label(tr(lang, "plan_sets.saving"));
                    }
                } else {
                    save = ui
                        .add_enabled(
                            ready && has_set && !fields_edited,
                            egui::Button::new(tr(lang, "plan_sets.save_copy")),
                        )
                        .clicked();
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                }
            },
        );
        if let Some(id) = select {
            self.plan_sets.select(Some(id));
        }
        if apply {
            if let Err(error) = self.plan_sets.apply_fields() {
                self.plan_sets.error = Some(error.to_string());
            }
        }
        if reset {
            if let Some(set) = &self.plan_sets.set {
                self.plan_sets.name = set.name.clone();
            }
            self.plan_sets.select(self.plan_sets.selected.clone());
        }
        if let Some(action) = replacement {
            if self.plan_sets.dirty || self.plan_sets.edited_fields() {
                self.plan_sets.replacement = Some(action);
            } else {
                self.replace_plan_set(action);
            }
        }
        if discard {
            if let Some(action) = self.plan_sets.replacement.take() {
                self.replace_plan_set(action);
            }
        }
        if let Some(relink) = refresh {
            self.plan_sets.refresh_confirmation = Some(relink);
        }
        if remove {
            if let (Some(set), Some(id)) = (&mut self.plan_sets.set, self.plan_sets.selected.take())
            {
                set.sheets.retain(|sheet| sheet.id != id);
                set.files
                    .retain(|file| set.sheets.iter().any(|sheet| sheet.file_id == file.id));
                let next = set.sheets.first().map(|s| s.id.clone());
                self.plan_sets.select(next);
                self.plan_sets.dirty = true;
                self.plan_sets.report = None;
            }
        }
        if add {
            if let Some(sources) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_files()
            {
                if let (Some(set), Some(path)) =
                    (self.plan_sets.set.clone(), self.plan_sets.path.clone())
                {
                    self.start_plan_set_job(PendingKind::Read, |cancel, password| Job::Add {
                        set,
                        path,
                        sources,
                        password,
                        cancel,
                    });
                }
            }
        }
        if let Some(relink) = confirmed_refresh {
            self.plan_sets.refresh_confirmation = None;
            let source = if relink {
                rfd::FileDialog::new()
                    .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                    .pick_file()
            } else {
                None
            };
            if !relink || source.is_some() {
                if let (Some(set), Some(path), Some(id)) = (
                    self.plan_sets.set.clone(),
                    self.plan_sets.path.clone(),
                    self.plan_sets.selected.clone(),
                ) {
                    if let Some(file_id) = set
                        .sheets
                        .iter()
                        .find(|s| s.id == id)
                        .map(|s| s.file_id.clone())
                    {
                        self.start_plan_set_job(PendingKind::Read, |cancel, password| {
                            Job::Refresh {
                                set,
                                path,
                                file_id,
                                source,
                                password,
                                cancel,
                            }
                        });
                    }
                }
            }
        }
        if inspect {
            if let (Some(set), Some(path)) =
                (self.plan_sets.set.clone(), self.plan_sets.path.clone())
            {
                self.start_plan_set_job(PendingKind::Read, |cancel, password| Job::Inspect {
                    set,
                    path,
                    password,
                    cancel,
                });
            }
        }
        if navigate {
            if let (Some(set), Some(path), Some(sheet_id)) = (
                self.plan_sets.set.clone(),
                self.plan_sets.path.clone(),
                self.plan_sets.selected.clone(),
            ) {
                self.start_plan_set_job(PendingKind::Read, |cancel, password| Job::Navigate {
                    set,
                    path,
                    sheet_id,
                    password,
                    cancel,
                });
            }
        }
        if save {
            if let (Some(set), Some(path)) =
                (self.plan_sets.set.clone(), self.plan_sets.path.clone())
            {
                let dialog = rfd::FileDialog::new()
                    .add_filter(tr(lang, "plan_sets.manifest"), &["json"])
                    .set_file_name("plan-set-copy.json");
                let dialog = if let Some(parent) = path.parent() {
                    dialog.set_directory(parent)
                } else {
                    dialog
                };
                if let Some(destination) = dialog.save_file() {
                    self.start_plan_set_job(PendingKind::Save, |_, _| Job::Save {
                        set,
                        path,
                        destination,
                    });
                }
            }
        }
        if cancel {
            self.plan_sets.cancel_read();
        }
        if close {
            self.plan_sets.navigation_confirmation = None;
            self.plan_sets.clear_password();
            self.plan_sets.open = false;
            self.plan_sets.quit_requested = false;
        }
        if discard_quit {
            self.plan_sets.navigation_confirmation = None;
            self.plan_sets.clear_password();
            self.plan_sets.set = None;
            self.plan_sets.path = None;
            self.plan_sets.draft = None;
            self.plan_sets.dirty = false;
            self.plan_sets.quit_requested = false;
            self.plan_sets.open = false;
            self.execute(ctx, sunipdf_core::Command::Quit);
        }
        if cancel_navigation {
            self.plan_sets.navigation_confirmation = None;
        }
        if let Some(use_existing) = navigate_existing {
            self.confirm_plan_navigation(use_existing);
        }
    }
}
