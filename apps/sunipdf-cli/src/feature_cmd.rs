use std::io::{IsTerminal, Read};
use std::path::Path;
use sunipdf_core::certificate::{self, CertSignOptions, CertificateInfo, DigitalSignature};
use sunipdf_core::EngineError;
use zeroize::Zeroizing;

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

pub fn prepare_signature_image(
    file: &Path,
    out: &Path,
    background: &str,
    threshold: u8,
) -> Result<serde_json::Value, EngineError> {
    use sunipdf_core::signature_image as sig;
    if !out
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err(failure("Choose a PNG output file for the visual signature"));
    }
    let source = std::fs::canonicalize(file)?;
    if out.exists() && std::fs::canonicalize(out)? == source {
        return Err(failure(
            "Visual signature preparation cannot replace its input image",
        ));
    }
    let metadata = std::fs::metadata(&source)?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
        return Err(failure(
            "Visual signature input must be a regular image of at most 64 MiB",
        ));
    }
    let input = sig::load(&source, 1600)?;
    let prepared = match background {
        "keep" => input,
        "threshold" => sig::remove_background(&input, threshold),
        "adaptive" => sig::remove_uneven_background(&input, threshold)?,
        _ => return Err(failure("Unknown visual signature background mode")),
    };
    let prepared = sig::trim(&prepared, 6)
        .ok_or_else(|| failure("No visible signature remains; adjust the background settings"))?;
    sig::save_png(&prepared, out)?;
    Ok(
        serde_json::json!({"out":out,"width":prepared.width(),"height":prepared.height(),"background":background,"threshold":if background=="keep" {None}else {Some(threshold)},"input_written":false,"cryptographic_signature":false}),
    )
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
        "identity_trusted":!signatures.is_empty() && signatures.iter().all(DigitalSignature::identity_trusted),
        "verification_failed":signatures.iter().any(|s| s.status != certificate::SignatureStatus::Valid),
        "verification_passed":!signatures.is_empty() && signatures.iter().all(|s| s.status == certificate::SignatureStatus::Valid),
        "signatures":signatures.iter().map(|s|serde_json::json!({
            "field_name":s.field_name,"sub_filter":s.sub_filter,"status":s.status.id(),
            "certificate":s.certificate.as_ref().map(|c| certificate_json(c, s.identity_trusted())),
            "reason":s.reason,"location":s.location,"signing_time":s.signing_time,
            "declared_signing_time":s.signing_time,"signing_time_source":"signer_declared",
            "declared_by_signer":{"field_name":s.field_name,"reason":s.reason,"location":s.location,"signing_time":s.signing_time},
            "signed_len":s.signed_len,"detail":s.detail,"failure_cause":s.failure_cause(),"identity_trusted":s.identity_trusted(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_json_exposes_rejection_and_marks_signing_time_as_declared() {
        let mut signature = DigitalSignature {
            field_name: "Signature".into(),
            sub_filter: "adbe.pkcs7.detached".into(),
            status: certificate::SignatureStatus::WeakDigest,
            certificate: None,
            reason: None,
            location: None,
            signing_time: Some("D:20261019000000Z".into()),
            signed_len: 100,
            detail: Some(0x80090008),
            trust: Default::default(),
        };
        let report = signatures_json(&[signature.clone()]);
        assert_eq!(report["verification_failed"], true);
        assert_eq!(report["verification_passed"], false);
        assert_eq!(
            report["signatures"][0]["failure_cause"],
            "digest_not_allowed"
        );
        assert_eq!(
            report["signatures"][0]["signing_time_source"],
            "signer_declared"
        );
        assert_eq!(
            report["signatures"][0]["declared_signing_time"],
            "D:20261019000000Z"
        );
        signature.status = certificate::SignatureStatus::Valid;
        let report = signatures_json(&[signature]);
        assert_eq!(report["verification_failed"], false);
        assert_eq!(report["verification_passed"], true);
        assert_eq!(report["identity_trusted"], false);
        assert_eq!(signatures_json(&[])["verification_passed"], false);
    }
}
