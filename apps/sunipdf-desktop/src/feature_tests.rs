use super::tests::test_app;
use super::*;
use crate::recovery::{Entry, Session};
use crate::worker::{DocState, TaskKind};
use std::sync::atomic::Ordering;

struct RecoveredApp {
    app: SuniApp,
    requests: std::sync::mpsc::Receiver<Request>,
    responses: std::sync::mpsc::Sender<Response>,
    entry: Entry,
    original: PathBuf,
    _temp: tempfile::TempDir,
}

fn state(path: PathBuf, modified: bool) -> DocState {
    DocState {
        info: sunipdf_core::DocumentInfo {
            path,
            title: None,
            page_sizes: vec![sunipdf_core::PageSize {
                width_pt: 612.0,
                height_pt: 792.0,
            }],
            form: sunipdf_core::FormKind::AcroForm,
        },
        can_undo: modified,
        can_redo: false,
        modified,
        pages_unchanged: true,
        has_signatures: false,
    }
}

fn recovered_app() -> RecoveredApp {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original.pdf");
    std::fs::write(&original, b"original remains untouched").unwrap();
    let root = temp.path().join("recovery");
    let crashed = Session::create(root.clone()).unwrap();
    let slot = crashed.slot(42, original.clone(), 1);
    slot.save(b"recovered PDF snapshot").unwrap();
    let entry = Entry {
        snapshot: slot.base.with_extension("pdf"),
        original: Some(original.clone()),
        pages: Some(1),
        updated: 1,
    };
    drop(crashed);
    let (mut app, requests, responses) = test_app();
    app.document = None;
    app.tabs.active = None;
    app.settings.autosave = true;
    app.recovery.session = Some(Session::create(root).unwrap());
    app.recovery.pending_restore = Some(entry.clone());
    app.open_path(entry.snapshot.clone(), None);
    assert!(requests
        .try_iter()
        .any(|r| matches!(r,Request::Open { path, .. } if path == entry.snapshot)));
    responses
        .send(Response::Opened(state(entry.snapshot.clone(), false)))
        .unwrap();
    app.handle_responses(&egui::Context::default());
    RecoveredApp {
        app,
        requests,
        responses,
        entry,
        original,
        _temp: temp,
    }
}

#[test]
fn recovered_document_uses_original_name_and_stable_identity_but_requires_save_as() {
    let mut fixture = recovered_app();
    let app = &mut fixture.app;
    assert_eq!(app.document.as_ref().unwrap().info.path, fixture.original);
    assert!(app.dirty);
    assert!(app.requires_save_as(false));
    assert!(app.recovery.pending_restore.is_none());
    let id = app.tabs.tabs[app.tabs.active.unwrap()].recovery_id;
    let before = app.recovery_slot().unwrap();
    let requests = fixture.requests.try_iter().collect::<Vec<_>>();
    assert!(requests.iter().any(|r| matches!(r, Request::SetRecovery(Some(slot)) if slot.original == fixture.original && slot.base == before.base)));
    assert!(requests
        .iter()
        .any(|r| matches!(r, Request::MarkRecovered { path } if *path == fixture.original)));
    // Closing a preceding tab shifts indices; the recovery slot must still identify this document.
    assert_eq!(app.tabs.active, Some(1));
    app.tabs.tabs.remove(0);
    app.tabs.active = Some(0);
    assert_eq!(app.tabs.tabs[0].recovery_id, id);
    assert_eq!(app.recovery_slot().unwrap().base, before.base);
    assert!(app.recovery.restored.contains_key(&id));
    assert_eq!(
        std::fs::read(&fixture.original).unwrap(),
        b"original remains untouched"
    );
}

#[test]
fn failed_save_preserves_both_recovery_copies_and_the_original() {
    let mut fixture = recovered_app();
    let app = &mut fixture.app;
    let slot = app.recovery_slot().unwrap();
    slot.save(b"newer recovery snapshot").unwrap();
    fixture.requests.try_iter().for_each(drop);
    app.saving = true;
    app.after_save = Some(PendingAction::Close);
    fixture
        .responses
        .send(Response::SaveFailed(EngineError::Io(
            std::io::Error::other("disk full"),
        )))
        .unwrap();
    app.handle_responses(&egui::Context::default());
    assert!(app.document.is_some() && app.dirty);
    assert!(!app.saving && app.after_save.is_none());
    assert!(app.requires_save_as(false));
    assert_eq!(
        std::fs::read(&fixture.entry.snapshot).unwrap(),
        b"recovered PDF snapshot"
    );
    assert_eq!(
        std::fs::read(slot.base.with_extension("pdf")).unwrap(),
        b"newer recovery snapshot"
    );
    assert_eq!(
        std::fs::read(&fixture.original).unwrap(),
        b"original remains untouched"
    );
    assert!(!fixture
        .requests
        .try_iter()
        .any(|r| matches!(r, Request::Close)));
}

#[test]
fn successful_save_as_updates_tab_path_and_removes_only_the_old_recovery_entry() {
    let mut fixture = recovered_app();
    let app = &mut fixture.app;
    let id = app.tabs.tabs[app.tabs.active.unwrap()].recovery_id;
    let new_path = fixture.original.with_file_name("saved-copy.pdf");
    fixture.requests.try_iter().for_each(drop);
    app.saving = true;
    fixture
        .responses
        .send(Response::Saved {
            path: new_path.clone(),
        })
        .unwrap();
    app.handle_responses(&egui::Context::default());
    let tab = &app.tabs.tabs[app.tabs.active.unwrap()];
    assert_eq!(tab.path, new_path);
    assert_eq!(tab.recovery_id, id);
    assert_eq!(app.document.as_ref().unwrap().info.path, new_path);
    assert!(!app.dirty && !app.saving && !app.requires_save_as(false));
    assert!(!app.recovery.restored.contains_key(&id));
    assert!(!fixture.entry.snapshot.exists());
    assert!(!fixture.entry.snapshot.with_extension("json").exists());
    assert_eq!(
        std::fs::read(&fixture.original).unwrap(),
        b"original remains untouched"
    );
    assert!(fixture
        .requests
        .try_iter()
        .any(|r| matches!(r, Request::SetRecovery(Some(slot)) if slot.original == new_path)));
}

#[test]
fn close_cancels_print_or_ocr_and_waits_for_the_worker_before_closing() {
    for kind in [TaskKind::Print, TaskKind::Ocr] {
        let (mut app, requests, responses) = test_app();
        let ctx = egui::Context::default();
        match kind {
            TaskKind::Print => app.printing.begin(4),
            TaskKind::Ocr => app.ocr.begin(4),
        }
        responses
            .send(Response::TaskProgress {
                kind,
                done: 1,
                total: 4,
            })
            .unwrap();
        app.handle_responses(&ctx);
        let progress = match kind {
            TaskKind::Print => (app.printing.done, app.printing.total),
            TaskKind::Ocr => (app.ocr.done, app.ocr.total),
        };
        assert_eq!(progress, (1, 4));
        app.execute(&ctx, Command::RotateClockwise);
        assert!(
            requests.try_iter().next().is_none(),
            "editing during a running task must be blocked"
        );
        app.execute(&ctx, Command::CloseDocument);
        assert!(app.document.is_some());
        assert!(matches!(app.deferred_close, Some(PendingAction::Close)));
        let cancelled = match kind {
            TaskKind::Print => app.printing.cancel.load(Ordering::Relaxed),
            TaskKind::Ocr => app.ocr.cancel.load(Ordering::Relaxed),
        };
        assert!(cancelled);
        assert!(!requests.try_iter().any(|r| matches!(r, Request::Close)));
        responses
            .send(Response::TaskDone {
                kind,
                result: Err(EngineError::Cancelled),
            })
            .unwrap();
        app.handle_responses(&ctx);
        assert!(app.document.is_none());
        assert!(requests.try_iter().any(|r| matches!(r, Request::Close)));
    }
}

#[test]
fn signed_pdf_clean_save_is_noop_and_changed_save_can_be_cancelled() {
    let (mut app, requests, _responses) = test_app();
    let ctx = egui::Context::default();
    crate::icons::install(&ctx);
    app.has_signatures = true;
    app.save_document(false);
    assert!(app.signed_save_confirm.is_none());
    assert!(requests.try_iter().next().is_none());
    app.dirty = true;
    app.after_save = Some(PendingAction::Close);
    app.save_document(false);
    assert_eq!(app.signed_save_confirm, Some(false));
    assert!(!app.saving);
    assert!(!requests
        .try_iter()
        .any(|r| matches!(r, Request::Save { .. })));
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |_| app.signed_save_dialog(&ctx),
    );
    output.textures_delta.clear();
    assert!(app.signed_save_confirm.is_none() && app.after_save.is_none());
    assert!(app.document.is_some() && app.dirty && app.has_signatures);
    assert!(!requests
        .try_iter()
        .any(|r| matches!(r, Request::Save { .. } | Request::Close)));
}

#[test]
fn field_creation_commands_open_dialog_and_document_change_refreshes_the_form_model() {
    let (mut app, requests, responses) = test_app();
    let ctx = egui::Context::default();
    for command in [Command::ToolAddTextField, Command::ToolAddCheckbox] {
        app.execute(&ctx, command);
        assert!(app.form_builder.visible);
        assert!(
            !requests.try_iter().any(|r| matches!(r, Request::Edit(_))),
            "opening the creation dialog must not mutate the document"
        );
        app.form_builder.visible = false;
    }
    app.forms.panel_open = true;
    app.forms.info = Some(sunipdf_core::FormInfo::default());
    app.forms.stale = false;
    responses
        .send(Response::DocumentChanged(state(
            app.document.as_ref().unwrap().info.path.clone(),
            true,
        )))
        .unwrap();
    app.handle_responses(&ctx);
    assert!(app.dirty && app.forms.stale);
    app.refresh_forms_if_needed();
    assert!(requests.try_iter().any(|r| matches!(r, Request::FormInfo)));
    assert!(app.forms.requested);
    responses
        .send(Response::FormInfo(sunipdf_core::FormInfo::default()))
        .unwrap();
    app.handle_responses(&ctx);
    assert!(!app.forms.requested);
}

#[derive(Debug)]
struct DroppedPdf(PathBuf);
impl egui::DroppedFile for DroppedPdf {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Err("a modal dialog must not read the dropped file".into())
    }
}

#[test]
fn document_dialogs_reject_file_drops_instead_of_retargeting_pending_actions() {
    let temp = tempfile::tempdir().unwrap();
    let other = temp.path().join("other.pdf");
    std::fs::write(&other, b"%PDF-1.4").unwrap();
    for command in [
        Command::ToolAddTextField,
        Command::SignWithCertificate,
        Command::Save,
    ] {
        let (mut app, requests, _responses) = test_app();
        let ctx = egui::Context::default();
        let original = app.document.as_ref().unwrap().info.path.clone();
        if command == Command::Save {
            app.has_signatures = true;
            app.dirty = true;
        }
        app.execute(&ctx, command);
        assert!(app.dialog_open());
        let mut output = ctx.run_ui(
            egui::RawInput {
                dropped_files: vec![std::sync::Arc::new(DroppedPdf(other.clone()))],
                ..Default::default()
            },
            |_| app.handle_input(&ctx),
        );
        output.textures_delta.clear();
        assert_eq!(app.document.as_ref().unwrap().info.path, original);
        assert!(app.dialog_open());
        assert!(app.opening.is_none() && app.tabs.suspending.is_none());
        assert!(!requests
            .try_iter()
            .any(|r| matches!(r, Request::Open { .. } | Request::Suspend)));
    }
}

#[test]
fn unavailable_recovery_storage_is_visible_before_any_autosave_request() {
    let (mut app, requests, _responses) = test_app();
    app.settings.autosave = true;
    app.recovery.failed = true;
    app.recovery.session = None;
    app.dirty = true;
    app.autosave_tick();
    assert_eq!(app.error, Some("error.recovery_failed"));
    assert!(!app.recovery.pending);
    assert!(!requests.try_iter().any(|r| matches!(r, Request::Autosave)));
    app.settings.autosave = false;
    app.error = None;
    app.autosave_tick();
    assert!(
        app.error.is_none(),
        "disabled autosave must not repeatedly raise an unavailable-storage error"
    );
}
