//! UI routing for long-running operations; the single engine worker owns every PDF operation.
use crate::app::SuniApp;
use crate::worker::{EditOp, Request};
use sunipdf_i18n::{tr, tr_args};

impl SuniApp {
    pub(crate) fn start_print(&mut self) {
        if !self.commit_form_input() {
            return;
        }
        let Some(doc) = &self.document else { return };
        let selected = if self.organize.active {
            self.organize.selected.iter().copied().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        #[cfg(windows)]
        // SAFETY: this is queried on the UI thread while handling the app's print action.
        let owner =
            unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow() as usize };
        #[cfg(not(windows))]
        let owner = 0;
        match crate::printing::choose(owner, doc.info.page_count(), doc.page, &selected) {
            Ok(Some(job)) => {
                self.printing.begin(job.pages.len());
                self.worker.send(Request::Print {
                    job,
                    cancel: self.printing.cancel.clone(),
                });
            }
            Ok(None) => {}
            Err(error) => {
                log::error!("Print setup failed: {error}");
                self.error = Some(crate::app::error_key(&error));
            }
        }
    }

    pub(crate) fn feature_dialogs(&mut self, ctx: &egui::Context) {
        self.processing_dialog(ctx);
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let icons = self.has_icons;
        self.printing.show(ctx, lang, dark, icons);
        if let Some(doc) = &self.document {
            if let Some(options) =
                self.ocr
                    .show(ctx, lang, dark, icons, doc.info.page_count(), doc.page)
            {
                if self.commit_form_input() {
                    self.ocr.begin(options.pages.len());
                    self.worker.send(Request::Ocr {
                        options,
                        cancel: self.ocr.cancel.clone(),
                    });
                }
            }
        }
        if let Some(field) = self.form_builder.show(ctx, lang, dark, icons) {
            self.worker.send(Request::Edit(EditOp::AddFormField {
                page: field.page,
                name: field.name,
                kind: field.kind,
                rect: field.rect,
                options: field.options,
                multi_select: field.multi_select,
            }));
            self.forms.panel_open = true;
        }
        if let Some(action) = self.certificate.show(ctx, lang, dark, icons) {
            if self.has_unsaved_changes() || self.worker.has_pending_mutations() {
                self.certificate.busy = false;
                self.certificate.error = Some(tr(lang, "error.certificate_save_first").into());
            } else {
                self.certificate.busy = true;
                match action {
                    crate::certificate::CertificateAction::Verify => {
                        self.worker.send(Request::VerifySignatures)
                    }
                    crate::certificate::CertificateAction::Sign {
                        pfx,
                        password,
                        options,
                        output,
                    } => self.worker.send(Request::SignCertificate {
                        pfx,
                        password,
                        options,
                        output,
                    }),
                }
            }
        }
        self.recovery_dialog(ctx);
    }

    pub(crate) fn autosave_tick(&mut self) {
        if self.settings.autosave
            && self.has_unsaved_changes()
            && self.recovery.session.is_none()
            && self.recovery.failed
        {
            self.error = Some("error.recovery_failed");
            return;
        }
        if !self.settings.autosave
            || !self.has_unsaved_changes()
            || self.printing.busy
            || self.ocr.busy
            || self.certificate.busy
            || self.processing.busy
            || !self.recovery.due()
        {
            return;
        }
        if self.commit_form_input() && self.recovery_slot().is_some() {
            self.recovery.pending = true;
            self.worker.send(Request::Autosave);
        }
    }

    pub(crate) fn recovery_restored(&mut self, id: u64, entry: crate::recovery::Entry) {
        self.recovery.restored.insert(id, entry);
        self.status_msg = Some(tr_args(self.lang(), "recovery.restored", &[]));
    }
}
