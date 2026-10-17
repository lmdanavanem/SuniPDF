use std::io::{IsTerminal, Read};
use std::path::Path;
use sunipdf_core::certificate::{self, CertSignOptions, CertificateInfo, DigitalSignature};
use sunipdf_core::EngineError;
use zeroize::Zeroizing;

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn certificate_json(info: &CertificateInfo, trusted: bool) -> serde_json::Value {
    serde_json::json!({
        "subject":info.subject,"issuer":info.issuer,"serial_hex":info.serial_hex,
        "sha256_hex":info.sha256_hex,"not_before":info.not_before,"not_after":info.not_after,
        "valid_now":info.valid_now,"identity_trusted":trusted
    })
}

fn signatures_json(signatures: &[DigitalSignature]) -> serde_json::Value {
    serde_json::json!({
        "identity_trusted":!signatures.is_empty() && signatures.iter().all(|s| s.status == certificate::SignatureStatus::Valid && s.trust.identity_trusted()),
        "signatures":signatures.iter().map(|s|serde_json::json!({
            "field_name":s.field_name,"sub_filter":s.sub_filter,"status":s.status.id(),
            "certificate":s.certificate.as_ref().map(|c| certificate_json(c, s.status == certificate::SignatureStatus::Valid && s.trust.identity_trusted())),
            "reason":s.reason,"location":s.location,"signing_time":s.signing_time,
            "signed_len":s.signed_len,"detail":s.detail,"identity_trusted":s.status == certificate::SignatureStatus::Valid && s.trust.identity_trusted(),
            "trust": {
                "chain":s.trust.chain.id(),"revocation":s.trust.revocation.id(),"chain_detail":s.trust.chain_detail,
                "network_allowed":s.trust.network_allowed,"ltv":s.trust.ltv.id(),
                "timestamp":{"status":s.trust.timestamp.status.id(),"time":s.trust.timestamp.time,
                    "chain":s.trust.timestamp.chain.id(),"revocation":s.trust.timestamp.revocation.id(),"detail":s.trust.timestamp.detail}
            }
        })).collect::<Vec<_>>()
    })
}

pub fn signatures(
    file: &Path,
    password: Option<&str>,
    allow_network: bool,
) -> Result<serde_json::Value, EngineError> {
    let input = std::fs::read(file)?;
    Ok(signatures_json(
        &certificate::verify_signatures_with_options(
            &input,
            password,
            certificate::VerificationOptions { allow_network },
        )?,
    ))
}

fn password(env: Option<&str>, stdin: bool) -> Result<Zeroizing<String>, EngineError> {
    if let Some(name) = env {
        return std::env::var(name).map(Zeroizing::new).map_err(|_| {
            failure("certificate password environment variable is missing or invalid")
        });
    }
    if !stdin || std::io::stdin().is_terminal() {
        return Err(failure(
            "provide the certificate password through an environment variable or piped stdin",
        ));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin().take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(failure("certificate password exceeds limits"));
    }
    if bytes.ends_with(b"\r\n") {
        let length = bytes.len() - 2;
        bytes.truncate(length);
    } else if bytes.ends_with(b"\n") || bytes.ends_with(b"\r") {
        bytes.pop();
    }
    let text =
        std::str::from_utf8(&bytes).map_err(|_| failure("certificate password must be UTF-8"))?;
    Ok(Zeroizing::new(text.to_string()))
}

pub fn sign(
    file: &Path,
    certificate_path: &Path,
    password_env: Option<&str>,
    password_stdin: bool,
    options: CertSignOptions,
    out: &Path,
) -> Result<serde_json::Value, EngineError> {
    let source = std::fs::canonicalize(file)?;
    if out.exists() && std::fs::canonicalize(out)? == source {
        return Err(failure(
            "digital signatures must be saved to a new copy; the input cannot be replaced",
        ));
    }
    let input = std::fs::read(&source)?;
    let pfx = Zeroizing::new(std::fs::read(certificate_path)?);
    let password = password(password_env, password_stdin)?;
    let signed = certificate::sign_pdf(&input, &pfx, &password, &options)?;
    if !signed.starts_with(&input) {
        return Err(failure("signing did not preserve the original PDF bytes"));
    }
    let mut report = signatures_json(&certificate::verify_signatures(&signed, None)?);
    sunipdf_core::write_new_copy(out, &signed)?;
    report["out"] = serde_json::json!(out);
    Ok(report)
}
