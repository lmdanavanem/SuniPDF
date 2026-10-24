use lopdf::{dictionary, Object};
use sunipdf_core::certificate::{
    identity_info, sign_pdf, verify_signatures, CertSignOptions, SignatureStatus,
};
use sunipdf_core::{security, EngineError, Protection};
#[cfg(windows)]
#[path = "support/certificate_identity.rs"]
mod test_identity;

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

#[test]
fn encrypted_signing_is_refused_before_identity_loading_even_with_an_empty_user_password() {
    for aes256 in [false, true] {
        for user_password in ["", "reader"] {
            let input = security::protect(
                PDF,
                None,
                &Protection {
                    user_password: user_password.into(),
                    owner_password: "owner".into(),
                    allow_modify: false,
                    aes256,
                    ..Default::default()
                },
            )
            .unwrap();
            let original = input.clone();
            let parsed = lopdf::Document::load_mem(&input).unwrap();
            if user_password.is_empty() {
                assert!(!parsed.is_encrypted());
                assert!(parsed.encryption_state.is_some());
            } else {
                assert!(parsed.is_encrypted());
            }

            // An invalid identity distinguishes the PDF guard from a later PFX/provider error.
            let error = sign_pdf(
                &input,
                b"not a certificate",
                "not used",
                &CertSignOptions::default(),
            )
            .unwrap_err();
            assert!(
                matches!(error, EngineError::Other(ref message)
                    if message == "signing encrypted PDFs is not supported"),
                "aes256={aes256}, empty_user={}: {error}",
                user_password.is_empty()
            );
            assert_eq!(input, original);
            let restored = security::unprotect(&input, "owner").unwrap();
            let restored = lopdf::Document::load_mem(&restored).unwrap();
            let source = lopdf::Document::load_mem(PDF).unwrap();
            assert_eq!(restored.get_pages().len(), source.get_pages().len());
            for (actual, expected) in restored
                .get_pages()
                .into_values()
                .zip(source.get_pages().into_values())
            {
                assert_eq!(
                    restored.get_page_content(actual),
                    source.get_page_content(expected)
                );
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn signatures_preserve_original_bytes_and_detect_tampering_and_later_revisions() {
    let pfx = test_identity::identity();
    let info = identity_info(&pfx, "test-only").unwrap();
    assert!(info.subject.starts_with("CN=") && info.subject.contains("SuniPDF Test Identity"));
    assert!(info.valid_now && info.sha256_hex.len() == 64);
    assert!(identity_info(&pfx, "wrong-password").is_err());
    assert!(identity_info(b"not a certificate", "test-only").is_err());
    let signed = sign_pdf(
        PDF,
        &pfx,
        "test-only",
        &CertSignOptions {
            reason: Some("Approved É".into()),
            location: Some("Zürich".into()),
        },
    )
    .unwrap();
    assert_eq!(&signed[..PDF.len()], PDF);
    let result = verify_signatures(&signed, None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].status, SignatureStatus::Valid);
    assert_eq!(result[0].reason.as_deref(), Some("Approved É"));
    assert_eq!(
        result[0].certificate.as_ref().unwrap().sha256_hex,
        info.sha256_hex
    );
    let engine = sunipdf_core::Engine::new().unwrap();
    let doc = engine
        .open_bytes(signed.clone(), None, std::path::Path::new("signed.pdf"))
        .unwrap();
    assert_eq!(
        doc.signature_count(),
        1,
        "PDFium must recognize the active signature field"
    );
    // Independent CMS verification checks integrity, with identity trust explicitly disabled.
    let parsed = lopdf::Document::load_mem(&signed).unwrap();
    let signature = parsed
        .objects
        .values()
        .filter_map(|o| o.as_dict().ok())
        .find(|d| d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Sig"))
        .unwrap();
    let range = signature
        .get(b"ByteRange")
        .and_then(Object::as_array)
        .unwrap();
    let range: Vec<usize> = range.iter().map(|o| o.as_i64().unwrap() as usize).collect();
    let mut cms = signature
        .get(b"Contents")
        .and_then(Object::as_str)
        .unwrap()
        .to_vec();
    let length_bytes = usize::from(cms[1] & 127);
    let der_end = if cms[1] < 128 {
        2 + usize::from(cms[1])
    } else {
        2 + length_bytes
            + cms[2..2 + length_bytes]
                .iter()
                .fold(0usize, |n, b| n * 256 + usize::from(*b))
    };
    cms.truncate(der_end);
    let temp = tempfile::tempdir().unwrap();
    let cms_path = temp.path().join("signature.cms");
    let content_path = temp.path().join("signed-bytes");
    let recovered = temp.path().join("verified-bytes");
    let content = [&signed[..range[1]], &signed[range[2]..range[2] + range[3]]].concat();
    std::fs::write(&cms_path, cms).unwrap();
    std::fs::write(&content_path, &content).unwrap();
    let oracle = std::process::Command::new(test_identity::openssl())
        .args([
            "cms",
            "-verify",
            "-binary",
            "-inform",
            "DER",
            "-noverify",
            "-in",
        ])
        .arg(cms_path)
        .arg("-content")
        .arg(content_path)
        .arg("-out")
        .arg(&recovered)
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    assert_eq!(std::fs::read(recovered).unwrap(), content);
    let mut tampered = signed.clone();
    tampered[10] ^= 1;
    let tampered_result = verify_signatures(&tampered, None).unwrap();
    assert_eq!(
        tampered_result[0].status,
        SignatureStatus::Invalid,
        "native error: {:?}",
        tampered_result[0].detail
    );
    let mut appended = signed.clone();
    appended.extend_from_slice(b"\n% later revision\n");
    assert_eq!(
        verify_signatures(&appended, None).unwrap()[0].status,
        SignatureStatus::ModifiedAfterSigning
    );
    let twice = sign_pdf(&signed, &pfx, "test-only", &CertSignOptions::default()).unwrap();
    assert_eq!(&twice[..signed.len()], &signed);
    let result = verify_signatures(&twice, None).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].status, SignatureStatus::ModifiedAfterSigning);
    assert_eq!(result[1].status, SignatureStatus::Valid);
    assert_eq!(
        engine
            .open_bytes(twice.clone(), None, std::path::Path::new("twice.pdf"))
            .unwrap()
            .signature_count(),
        2
    );
    let mut collision = lopdf::Document::load_mem(PDF).unwrap();
    let existing = collision
        .add_object(dictionary! { "FT"=>"Tx","T"=>lopdf::text_string("SuniPDF-Signature-1") });
    let acro = collision.add_object(dictionary! { "Fields"=>vec![Object::Reference(existing)] });
    collision.catalog_mut().unwrap().set("AcroForm", acro);
    let mut bytes = Vec::new();
    collision.save_to(&mut bytes).unwrap();
    let signed = sign_pdf(&bytes, &pfx, "test-only", &CertSignOptions::default()).unwrap();
    assert_eq!(
        verify_signatures(&signed, None).unwrap()[0].field_name,
        "SuniPDF-Signature-1_"
    );
    let encrypted = security::protect(
        PDF,
        None,
        &Protection {
            user_password: "user".into(),
            owner_password: "owner".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(sign_pdf(&encrypted, &pfx, "test-only", &CertSignOptions::default()).is_err());
}

#[test]
fn inactive_signature_objects_are_not_presented_as_signatures() {
    let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
    pdf.add_object(dictionary! { "Type"=>"Sig","ByteRange"=>vec![Object::Integer(0),Object::Integer(1),Object::Integer(3),Object::Integer(1)],"Contents"=>Object::string_literal("bad"),"SubFilter"=>"adbe.pkcs7.detached" });
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    assert!(verify_signatures(&bytes, None).unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn empty_signature_fields_and_optional_null_entries_allow_signing() {
    let pfx = test_identity::identity();
    for indirect in [false, true] {
        let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
        let null = if indirect {
            Object::Reference(pdf.add_object(Object::Null))
        } else {
            Object::Null
        };
        let field = pdf.add_object(dictionary! {
            "FT"=>"Sig", "T"=>lopdf::text_string("SuniPDF-Signature-1"),
            "V"=>null.clone(), "Kids"=>null.clone(),
        });
        let acro = pdf.add_object(dictionary! {
            "Fields"=>vec![Object::Reference(field)], "XFA"=>null.clone(),
        });
        pdf.catalog_mut().unwrap().set("AcroForm", acro);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        assert!(verify_signatures(&bytes, None).unwrap().is_empty());
        let signed = sign_pdf(&bytes, &pfx, "test-only", &CertSignOptions::default()).unwrap();
        let signatures = verify_signatures(&signed, None).unwrap();
        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].status, SignatureStatus::Valid);
        assert_eq!(signatures[0].field_name, "SuniPDF-Signature-1_");
        let twice = sign_pdf(&signed, &pfx, "test-only", &CertSignOptions::default()).unwrap();
        let signatures = verify_signatures(&twice, None).unwrap();
        assert_eq!(signatures.len(), 2);
        assert_eq!(signatures[0].status, SignatureStatus::ModifiedAfterSigning);
        assert_eq!(signatures[1].status, SignatureStatus::Valid);

        for null_acro in [false, true] {
            let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
            let null = if indirect {
                Object::Reference(pdf.add_object(Object::Null))
            } else {
                Object::Null
            };
            let acro = if null_acro {
                null
            } else {
                Object::Reference(pdf.add_object(dictionary! {"Fields"=>null}))
            };
            pdf.catalog_mut().unwrap().set("AcroForm", acro);
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            assert!(verify_signatures(&bytes, None).unwrap().is_empty());
            let signed = sign_pdf(&bytes, &pfx, "test-only", &CertSignOptions::default()).unwrap();
            assert_eq!(
                verify_signatures(&signed, None).unwrap()[0].status,
                SignatureStatus::Valid
            );
        }
    }
}

#[cfg(not(windows))]
#[test]
fn unavailable_certificate_provider_never_claims_trust_and_keeps_signature_detection() {
    assert!(identity_info(b"not imported", "private input").is_err());
    assert!(sign_pdf(
        PDF,
        b"not imported",
        "private input",
        &CertSignOptions::default()
    )
    .is_err());
    let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
    let envelope = b"\x30\x23\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x02\xa0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0b\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x01\x31\x00";
    let signature = pdf.add_object(dictionary! {
        "Type" => "Sig", "Filter" => "Adobe.PPKLite", "SubFilter" => "adbe.pkcs7.detached",
        "ByteRange" => vec![Object::Integer(0), Object::Integer(9_999_999_999), Object::Integer(9_999_999_999), Object::Integer(9_999_999_999)],
        "Contents" => Object::String(envelope.to_vec(), lopdf::StringFormat::Hexadecimal),
    });
    let field = pdf.add_object(
        dictionary! { "FT" => "Sig", "T" => lopdf::text_string("Unverified"), "V" => signature },
    );
    let acro =
        pdf.add_object(dictionary! { "Fields" => vec![Object::Reference(field)], "SigFlags" => 3 });
    pdf.catalog_mut().unwrap().set("AcroForm", acro);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let serialized = lopdf::Document::load_mem(&bytes).unwrap();
    let offset = match serialized.reference_table.get(signature.0) {
        Some(lopdf::xref::XrefEntry::Normal { offset, .. }) => *offset as usize,
        other => panic!("unexpected signature reference: {other:?}"),
    };
    let contents = offset
        + bytes[offset..]
            .windows(9)
            .position(|w| w == b"/Contents")
            .unwrap()
        + 9;
    let first = contents + bytes[contents..].iter().position(|b| *b == b'<').unwrap();
    let second = first + envelope.len() * 2 + 2;
    assert_eq!(bytes[second - 1], b'>');
    let range = offset
        + bytes[offset..]
            .windows(10)
            .position(|w| w == b"/ByteRange")
            .unwrap()
        + 10;
    let start = range + bytes[range..].iter().position(|b| *b == b'[').unwrap();
    let end = start + bytes[start..].iter().position(|b| *b == b']').unwrap() + 1;
    let replacement = format!("[0 {first} {second} {}]", bytes.len() - second);
    assert!(replacement.len() <= end - start);
    bytes[start..end].fill(b' ');
    bytes[start..start + replacement.len()].copy_from_slice(replacement.as_bytes());
    let report = verify_signatures(&bytes, None).unwrap();
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].status, SignatureStatus::Unverifiable);
    assert_eq!(report[0].signed_len, bytes.len());
    assert_eq!(report[0].failure_cause(), Some("verification_unavailable"));
    assert!(report[0].certificate.is_none());
    assert!(!report[0].identity_trusted());
    assert_eq!(
        report[0].trust.chain,
        sunipdf_core::certificate::ChainTrust::Unavailable
    );
    let engine = sunipdf_core::Engine::new().unwrap();
    let document = engine
        .open_bytes(bytes, None, std::path::Path::new("unverified.pdf"))
        .unwrap();
    assert_eq!(
        document.signature_count(),
        1,
        "signed-document guards require signature discovery on every platform"
    );
}

#[test]
fn malformed_byte_ranges_and_unsupported_formats_never_pass_verification() {
    for (range, sub_filter, status) in [
        (
            vec![0, 1, 3, 1],
            "adbe.pkcs7.detached",
            SignatureStatus::Unverifiable,
        ),
        (
            vec![-1, 1, 3, 1],
            "adbe.pkcs7.detached",
            SignatureStatus::Unverifiable,
        ),
        (
            vec![0, 1, i64::MAX, i64::MAX],
            "adbe.pkcs7.detached",
            SignatureStatus::Unverifiable,
        ),
        (
            vec![0, 1, 3, 1],
            "ETSI.RFC3161",
            SignatureStatus::Unsupported,
        ),
    ] {
        let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
        let signature = pdf.add_object(dictionary! { "Type"=>"Sig","ByteRange"=>Object::Array(range.into_iter().map(Object::Integer).collect()),"Contents"=>Object::String(vec![0x30,0],lopdf::StringFormat::Hexadecimal),"SubFilter"=>sub_filter });
        let field = pdf.add_object(
            dictionary! { "FT"=>"Sig","T"=>lopdf::text_string("unreadable"),"V"=>signature },
        );
        let acro = pdf.add_object(dictionary! { "Fields"=>vec![Object::Reference(field)] });
        pdf.catalog_mut().unwrap().set("AcroForm", acro);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        assert_eq!(verify_signatures(&bytes, None).unwrap()[0].status, status);
    }
}
