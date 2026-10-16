//! Certificate signing is a new saved copy; mathematical integrity never establishes trust.

use crate::dialog::{self, Spec};
use std::path::PathBuf;
use sunipdf_core::certificate::{CertSignOptions, DigitalSignature};
use sunipdf_i18n::{tr, Language};
use zeroize::Zeroizing;

pub enum CertificateAction {
    Verify,
    Sign {
        pfx: PathBuf,
        password: Zeroizing<String>,
        options: CertSignOptions,
        output: PathBuf,
    },
}

#[derive(Default)]
pub struct CertificateState {
    pub visible: bool,
    pub busy: bool,
    pub error: Option<String>,
    signing: bool,
    pfx: Option<PathBuf>,
    password: Zeroizing<String>,
    reason: String,
    location: String,
    signatures: Option<Vec<DigitalSignature>>,
}

impl CertificateState {
    pub fn open(&mut self, signing: bool) {
        self.visible = true;
        self.signing = signing;
        self.error = None;
        self.password = Zeroizing::new(String::new());
        self.signatures = None;
    }
    pub fn results(&mut self, signatures: Vec<DigitalSignature>) {
        self.busy = false;
        self.signatures = Some(signatures);
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        lang: Language,
        dark: bool,
        has_icons: bool,
    ) -> Option<CertificateAction> {
        if !self.visible {
            return None;
        }
        let mut action = None;
        let mut submit = false;
        let mut close = false;
        let signing = self.signing;
        let busy = self.busy;
        let has_pfx = self.pfx.is_some();
        dialog::show(
            ctx,
            Spec::new(
                "certificate-dialog",
                tr(
                    lang,
                    if self.signing {
                        "certificate.sign_title"
                    } else {
                        "certificate.verify_title"
                    },
                ),
                dark,
                has_icons,
            )
            .width(520.0),
            |ui| {
                ui.label(tr(lang, "certificate.trust_unknown"));
                ui.label(tr(lang, "certificate.scope"));
                if self.signing {
                    ui.label(tr(lang, "certificate.saved_copy"));
                    if ui
                        .add_enabled(
                            !self.busy,
                            egui::Button::new(tr(lang, "certificate.choose")),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PKCS#12", &["pfx", "p12"])
                            .pick_file()
                        {
                            self.pfx = Some(path);
                            self.password = Zeroizing::new(String::new());
                        }
                    }
                    if let Some(path) = &self.pfx {
                        ui.label(path.display().to_string());
                    }
                    ui.label(tr(lang, "certificate.password"));
                    ui.add_enabled(
                        !self.busy,
                        egui::TextEdit::singleline(&mut *self.password).password(true),
                    );
                    ui.label(tr(lang, "certificate.reason"));
                    ui.text_edit_singleline(&mut self.reason);
                    ui.label(tr(lang, "certificate.location"));
                    ui.text_edit_singleline(&mut self.location);
                }
                if let Some(signatures) = &self.signatures {
                    if signatures.is_empty() {
                        ui.label(tr(lang, "certificate.none"));
                    }
                    for signature in signatures {
                        ui.separator();
                        ui.strong(&signature.field_name);
                        ui.label(tr(
                            lang,
                            &format!("certificate.status.{}", signature.status.id()),
                        ));
                        if let Some(cert) = &signature.certificate {
                            ui.label(format!(
                                "{}: {}",
                                tr(lang, "certificate.subject"),
                                cert.subject
                            ));
                            ui.label(format!(
                                "{}: {}",
                                tr(lang, "certificate.issuer"),
                                cert.issuer
                            ));
                            ui.label(format!(
                                "{}: {} — {}",
                                tr(lang, "certificate.validity"),
                                cert.not_before,
                                cert.not_after
                            ));
                            ui.label(format!("SHA-256: {}", cert.sha256_hex));
                        }
                        for (key, value) in [
                            ("certificate.reason", &signature.reason),
                            ("certificate.location", &signature.location),
                            ("certificate.declared_time", &signature.signing_time),
                        ] {
                            if let Some(value) = value {
                                ui.label(format!("{}: {}", tr(lang, key), value));
                            }
                        }
                    }
                }
                if self.busy {
                    ui.spinner();
                }
                if let Some(error) = &self.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
            },
            |ui| {
                submit = ui
                    .add_enabled(
                        !busy && (!signing || has_pfx),
                        egui::Button::new(tr(
                            lang,
                            if signing {
                                "certificate.sign"
                            } else {
                                "certificate.verify"
                            },
                        )),
                    )
                    .clicked();
                close = ui
                    .add_enabled(!busy, egui::Button::new(tr(lang, "about.close")))
                    .clicked()
                    || (!busy && ui.input(|i| i.key_pressed(egui::Key::Escape)));
            },
        );
        if close {
            self.visible = false;
            self.password = Zeroizing::new(String::new());
        }
        if submit {
            if self.signing {
                if let Some(output) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .set_file_name("signed-copy.pdf")
                    .save_file()
                {
                    if let Some(pfx) = self.pfx.clone() {
                        self.busy = true;
                        self.error = None;
                        action = Some(CertificateAction::Sign {
                            pfx,
                            password: std::mem::take(&mut self.password),
                            options: CertSignOptions {
                                reason: (!self.reason.is_empty()).then(|| self.reason.clone()),
                                location: (!self.location.is_empty())
                                    .then(|| self.location.clone()),
                            },
                            output,
                        });
                    }
                }
            } else {
                self.busy = true;
                self.error = None;
                action = Some(CertificateAction::Verify);
            }
        }
        action
    }
}
