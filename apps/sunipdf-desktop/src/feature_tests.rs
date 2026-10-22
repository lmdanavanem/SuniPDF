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
            page_labels: vec![],
            form: sunipdf_core::FormKind::AcroForm,
        },
        can_undo: modified,
        can_redo: false,
        modified,
        pages_unchanged: true,
        measurement_scales_unchanged: false,
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
fn close_cancels_running_tasks_and_waits_for_the_worker_before_closing() {
    for kind in [TaskKind::Print, TaskKind::Ocr, TaskKind::Signature] {
        let (mut app, requests, responses) = test_app();
        let ctx = egui::Context::default();
        match kind {
            TaskKind::Print => app.printing.begin(4),
            TaskKind::Ocr => app.ocr.begin(4),
            TaskKind::Signature => app.sign.begin(4),
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
            TaskKind::Signature => (app.sign.done, app.sign.total),
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
            TaskKind::Signature => app.sign.cancel.load(Ordering::Relaxed),
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
fn signature_page_scopes_are_invalidated_only_by_structural_document_changes() {
    use crate::sign::{Placing, SignKind};

    for pages_unchanged in [false, true] {
        for (captured, dialog) in [(true, false), (false, false), (false, true)] {
            let (mut app, requests, responses) = test_app();
            let ctx = egui::Context::default();
            if dialog {
                app.sign.open = Some(SignKind::Signature);
            } else {
                app.sign.placing = Some(Placing::Image {
                    image: std::sync::Arc::new(image::DynamicImage::ImageRgba8(
                        image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 0, 0, 255])),
                    )),
                    thumb: ctx.load_texture(
                        "signature-scope-test",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [2, 1],
                            &[0, 0, 0, 255, 0, 0, 0, 255],
                        ),
                        Default::default(),
                    ),
                    width_pt: 120.0,
                    aspect: 0.5,
                    pages: captured.then(|| vec![0]),
                });
            }
            let mut changed = state(app.document.as_ref().unwrap().info.path.clone(), true);
            changed.pages_unchanged = pages_unchanged;
            responses.send(Response::DocumentChanged(changed)).unwrap();
            app.handle_responses(&ctx);
            if dialog {
                assert_eq!(app.sign.open.is_some(), pages_unchanged);
            } else {
                assert_eq!(app.sign.placing.is_some(), pages_unchanged || !captured);
            }
            assert!(!requests.try_iter().any(|request| matches!(
                request,
                Request::Edit(_) | Request::PlaceSignature { .. }
            )));
        }
    }
}

#[test]
fn signature_completion_survives_document_change_and_preserves_repeat_hint() {
    for repeat_hint in [None, Some("Keep placing with Shift")] {
        let (mut app, _requests, responses) = test_app();
        let ctx = egui::Context::default();
        app.sign.begin(2);
        app.edit.pending_status = repeat_hint.map(str::to_owned);
        let expected = repeat_hint.unwrap_or(app.t("signature.done")).to_owned();
        responses
            .send(Response::TaskDone {
                kind: TaskKind::Signature,
                result: Ok(()),
            })
            .unwrap();
        responses
            .send(Response::DocumentChanged(state(
                app.document.as_ref().unwrap().info.path.clone(),
                true,
            )))
            .unwrap();
        app.handle_responses(&ctx);
        assert!(!app.sign.busy);
        assert!(app.dirty && app.can_undo);
        assert_eq!(app.status_msg.as_deref(), Some(expected.as_str()));
        assert!(app.edit.pending_status.is_none() && app.edit.hold_status);
    }
}

#[test]
fn failed_signature_batch_clears_repeat_hint_without_marking_document_modified() {
    for cancelled in [false, true] {
        let (mut app, _requests, responses) = test_app();
        let ctx = egui::Context::default();
        app.sign.begin(2);
        app.edit.pending_status = Some("Keep placing with Shift".into());
        responses
            .send(Response::TaskDone {
                kind: TaskKind::Signature,
                result: Err(if cancelled {
                    EngineError::Cancelled
                } else {
                    EngineError::Other("invalid signature target".into())
                }),
            })
            .unwrap();
        app.handle_responses(&ctx);
        assert!(!app.sign.busy && !app.dirty && !app.can_undo);
        assert!(app.edit.pending_status.is_none());
        if cancelled {
            assert_eq!(app.status_msg.as_deref(), Some(app.t("status.cancelled")));
        } else {
            assert!(app.error.is_some());
        }
        responses
            .send(Response::DocumentChanged(state(
                app.document.as_ref().unwrap().info.path.clone(),
                false,
            )))
            .unwrap();
        app.handle_responses(&ctx);
        assert!(app.status_msg.is_none() && !app.edit.hold_status);
        assert!(!app.dirty && !app.can_undo);
    }
}

#[test]
fn page_label_changes_and_undo_redo_preserve_page_history_and_metadata() {
    let (mut app, _requests, responses) = test_app();
    let ctx = egui::Context::default();
    let doc = app.document.as_mut().unwrap();
    doc.info.page_sizes = vec![doc.info.page_sizes[0]; 3];
    doc.info.title = Some("Original document title".into());
    doc.page = 1;
    let original = doc.info.clone();
    app.docinfo.metadata_edit = sunipdf_core::Metadata {
        title: original.title.clone(),
        author: Some("Original author".into()),
        ..Default::default()
    };
    let metadata = app.docinfo.metadata_edit.clone();
    let offset = egui::vec2(12.0, 345.0);
    app.view.offset = offset;
    app.view
        .reading
        .history
        .jump(0, Some(egui::Vec2::ZERO), 1, 1.0);
    assert!(app.view.reading.history.can_back());
    for labelled in [true, false, true] {
        let mut info = original.clone();
        if labelled {
            info.page_labels = ["i", "ii", "iii"]
                .into_iter()
                .map(|label| Some(label.into()))
                .collect();
        }
        responses
            .send(Response::DocumentChanged(DocState {
                info,
                can_undo: labelled,
                can_redo: !labelled,
                modified: labelled,
                pages_unchanged: true,
                measurement_scales_unchanged: false,
                has_signatures: false,
            }))
            .unwrap();
        app.handle_responses(&ctx);
        assert_eq!(app.document.as_ref().unwrap().page, 1);
        assert_eq!(app.page_input, if labelled { "ii" } else { "2" });
        assert_eq!(app.view.offset, offset);
        assert!(app.view.reading.history.can_back());
        assert_eq!(app.docinfo.metadata_edit, metadata);
        assert_eq!(app.document.as_ref().unwrap().info.title, original.title);
        assert_eq!(
            (app.dirty, app.can_undo, app.can_redo),
            (labelled, labelled, !labelled)
        );
    }
    app.page_input = "#2".into();
    app.submit_page_input();
    assert_eq!(app.document.as_ref().unwrap().page, 1);
    assert_eq!(app.page_input, "ii");
}

#[test]
fn page_label_dialog_cannot_survive_tab_open_or_structural_changes() {
    let (mut app, _requests, responses) = test_app();
    let ctx = egui::Context::default();
    app.open_page_dialog(Command::PageLabels);
    let tab = app.take_tab();
    assert!(app.pagesetup.dialog.is_none());
    app.put_tab(tab);
    assert!(app.pagesetup.dialog.is_none());
    for pages_unchanged in [true, false] {
        app.open_page_dialog(Command::PageLabels);
        let mut changed = state(app.document.as_ref().unwrap().info.path.clone(), true);
        changed.pages_unchanged = pages_unchanged;
        responses.send(Response::DocumentChanged(changed)).unwrap();
        app.handle_responses(&ctx);
        assert_eq!(app.pagesetup.dialog.is_some(), pages_unchanged);
    }
    app.open_page_dialog(Command::PageLabels);
    responses
        .send(Response::Opened(state(
            PathBuf::from("test/new.pdf"),
            false,
        )))
        .unwrap();
    app.handle_responses(&ctx);
    assert!(app.pagesetup.dialog.is_none());
    assert_eq!(
        app.document.as_ref().unwrap().info.path,
        PathBuf::from("test/new.pdf")
    );
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
        .send(Response::FormInfo(Ok(sunipdf_core::FormInfo::default())))
        .unwrap();
    app.handle_responses(&ctx);
    assert!(!app.forms.requested);
}

#[test]
fn failed_form_read_preserves_input_and_actions_until_explicit_successful_retry() {
    use sunipdf_core::{FieldKind, FieldValue, FormField, FormInfo, FormKind};

    let (mut app, requests, responses) = test_app();
    let ctx = egui::Context::default();
    let field = |name: &str, kind, value| FormField {
        name: name.into(),
        kind,
        value,
        tooltip: None,
        read_only: false,
        required: false,
        multiline: false,
        password: false,
        comb: false,
        max_len: None,
        multi_select: false,
        editable: false,
        no_toggle_off: false,
        options: vec![],
        widgets: vec![],
    };
    let confirmed = FormInfo {
        kind: FormKind::AcroForm,
        has_scripts: false,
        fields: vec![
            field(
                "Confirmed",
                FieldKind::Text,
                FieldValue::Text("saved".into()),
            ),
            field(
                "Changed",
                FieldKind::Text,
                FieldValue::Text("original".into()),
            ),
            field("Approved", FieldKind::Checkbox, FieldValue::Bool(false)),
        ],
    };
    let mut optimistic = confirmed.clone();
    optimistic.fields[2].value = FieldValue::Bool(true);
    app.forms.info = Some(optimistic.clone());
    app.forms.panel_open = true;
    app.forms.requested = true;
    app.forms.drafts.insert("Confirmed".into(), "saved".into());
    app.forms.drafts.insert("Changed".into(), "draft".into());
    app.forms
        .staged
        .insert("Approved".into(), FieldValue::Bool(true));
    app.forms.editing = Some(crate::forms::InPlace {
        page: 0,
        field: "Changed".into(),
        widget: 0,
        text: "in-place draft".into(),
        multiline: false,
        max_len: None,
        focus: false,
        armed: true,
    });
    app.forms.pending_import = Some(serde_json::json!({}));
    app.forms.pending_clear = true;
    responses
        .send(Response::FormInfo(Err(EngineError::Other(
            "read failed".into(),
        ))))
        .unwrap();
    app.handle_responses(&ctx);
    assert!(app.forms.read_failed && !app.forms.requested);
    assert_eq!(app.error, Some("error.form_read_failed"));
    assert_eq!(app.forms.info.as_ref(), Some(&optimistic));
    assert_eq!(app.forms.drafts.len(), 2);
    assert_eq!(
        app.forms.staged.get("Approved"),
        Some(&FieldValue::Bool(true))
    );
    assert_eq!(app.forms.editing.as_ref().unwrap().text, "in-place draft");
    assert!(app.forms.pending_import.is_some() && app.forms.pending_clear);
    for _ in 0..3 {
        app.refresh_forms_if_needed();
    }
    app.execute(&ctx, Command::ToggleForms);
    assert!(!app.forms.panel_open);
    app.refresh_forms_if_needed();
    assert!(!app.commit_form_input());
    assert!(app.commit_editor().is_none());
    app.commit_form("Approved".into(), FieldValue::Bool(false));
    app.dirty = true;
    app.save_document(false);
    assert!(!app.saving && app.pending_save.is_none());
    assert!(
        requests.try_recv().is_err(),
        "failed reads must neither loop nor apply an optimistic model"
    );
    assert_eq!(app.forms.info.as_ref(), Some(&optimistic));
    assert_eq!(app.forms.editing.as_ref().unwrap().text, "in-place draft");
    app.retry_form_read();
    assert!(matches!(requests.try_recv().unwrap(), Request::FormInfo));
    assert!(matches!(
        requests.try_recv().unwrap(),
        Request::FinishMutation
    ));
    assert!(app.forms.read_failed && app.forms.requested);
    assert!(
        !app.commit_form_input(),
        "retry must finish before values can be applied"
    );
    responses
        .send(Response::FormInfo(Ok(confirmed.clone())))
        .unwrap();
    responses.send(Response::MutationFinished).unwrap();
    app.handle_responses(&ctx);
    assert!(!app.forms.read_failed && !app.forms.requested);
    assert!(app.error.is_none());
    assert_eq!(app.forms.info.as_ref(), Some(&confirmed));
    assert!(!app.forms.drafts.contains_key("Confirmed"));
    assert_eq!(
        app.forms.drafts.get("Changed").map(String::as_str),
        Some("draft")
    );
    assert_eq!(
        app.forms.staged.get("Approved"),
        Some(&FieldValue::Bool(true))
    );
    assert_eq!(app.forms.editing.as_ref().unwrap().text, "in-place draft");
    assert!(app.forms.pending_import.is_none() && !app.forms.pending_clear);
    assert!(matches!(
        requests.try_recv().unwrap(),
        Request::Edit(EditOp::ClearForm)
    ));
    assert!(matches!(
        requests.try_recv().unwrap(),
        Request::FinishMutation
    ));
    assert!(requests.try_recv().is_err());
}

#[test]
fn fill_tool_retries_failed_initial_reads_and_accepts_a_confirmed_empty_form() {
    let (mut app, requests, responses) = test_app();
    let ctx = egui::Context::default();
    app.forms.panel_open = true;
    app.forms.requested = true;
    responses
        .send(Response::FormInfo(Err(EngineError::Other(
            "initial read failed".into(),
        ))))
        .unwrap();
    app.handle_responses(&ctx);
    for _ in 0..3 {
        app.refresh_forms_if_needed();
    }
    assert!(requests.try_recv().is_err());
    assert!(app.forms.info.is_none() && app.forms.read_failed);
    app.execute(&ctx, Command::ToolFillForm);
    app.refresh_forms_if_needed();
    assert!(matches!(requests.try_recv().unwrap(), Request::FormInfo));
    assert!(matches!(
        requests.try_recv().unwrap(),
        Request::FinishMutation
    ));
    assert!(app.forms.requested && app.forms.read_failed);
    responses
        .send(Response::FormInfo(Ok(sunipdf_core::FormInfo::default())))
        .unwrap();
    responses.send(Response::MutationFinished).unwrap();
    app.handle_responses(&ctx);
    assert!(!app.forms.read_failed && !app.forms.requested);
    assert_eq!(app.forms.info, Some(sunipdf_core::FormInfo::default()));
    assert!(app.error.is_none());
    app.refresh_forms_if_needed();
    assert!(requests.try_recv().is_err());
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
