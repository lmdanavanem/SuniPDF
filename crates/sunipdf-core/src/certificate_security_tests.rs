use super::*;

#[test]
fn signer_declared_text_is_single_line_bounded_and_cannot_carry_controls_or_bidi() {
    assert_eq!(
        display_declared_text("Reason\r\n\tverified\u{001b}\u{009b}\u{202e}text\u{2066}"),
        "Reason verifiedtext"
    );
    let long = display_declared_text(&"é".repeat(300));
    assert_eq!(long.chars().count(), 256);
    assert!(long.ends_with('…'));
    assert!(!long.chars().any(char::is_control));
}

#[test]
fn pdf_signature_declarations_are_sanitized_before_they_leave_the_engine() {
    let mut pdf =
        lopdf::Document::load_mem(include_bytes!("../tests/fixtures/two-pages.pdf")).unwrap();
    let value=pdf.add_object(dictionary! { "Type"=>"Sig","SubFilter"=>"adbe.pkcs7.detached", "Reason"=>lopdf::text_string("Reason\nVerifier text\u{202e}"), "Location"=>lopdf::text_string("Place\u{001b}\u{009b}"),"M"=>lopdf::text_string("Date\r\nClaim") });
    let field = pdf.add_object(
        dictionary! { "FT"=>"Sig","T"=>lopdf::text_string("Field\nClaim\u{2066}"),"V"=>value },
    );
    let acro = pdf.add_object(dictionary! { "Fields"=>vec![Object::Reference(field)] });
    pdf.catalog_mut().unwrap().set("AcroForm", acro);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let report = verify_signatures(&bytes, None).unwrap();
    assert_eq!(report[0].field_name, "Field Claim");
    assert_eq!(report[0].reason.as_deref(), Some("Reason Verifier text"));
    assert_eq!(report[0].location.as_deref(), Some("Place"));
    assert_eq!(report[0].signing_time.as_deref(), Some("Date Claim"));
    assert_ne!(report[0].status, SignatureStatus::Valid);
}

#[test]
fn identity_trust_always_requires_current_signature_integrity() {
    let mut signature = DigitalSignature {
        field_name: "Signature".into(),
        sub_filter: "adbe.pkcs7.detached".into(),
        status: SignatureStatus::Valid,
        certificate: Some(CertificateInfo {
            subject: "CN=Test".into(),
            issuer: "CN=Test".into(),
            serial_hex: "01".into(),
            sha256_hex: "00".repeat(32),
            not_before: String::new(),
            not_after: String::new(),
            valid_now: true,
        }),
        reason: None,
        location: None,
        signing_time: None,
        signed_len: 10,
        detail: None,
        trust: SignatureTrust {
            chain: ChainTrust::Trusted,
            revocation: RevocationStatus::Good,
            ..Default::default()
        },
    };
    assert!(signature.identity_trusted());
    for status in [
        SignatureStatus::Invalid,
        SignatureStatus::ModifiedAfterSigning,
        SignatureStatus::WeakDigest,
        SignatureStatus::Unsupported,
        SignatureStatus::Unverifiable,
    ] {
        signature.status = status;
        assert!(!signature.identity_trusted(), "{status:?}");
    }
    signature.status = SignatureStatus::Valid;
    signature.certificate = None;
    assert!(!signature.identity_trusted());
}

#[test]
fn detached_cms_layout_rejects_even_an_empty_explicit_content_value() {
    fn der(tag: u8, body: &[u8]) -> Vec<u8> {
        assert!(body.len() < 128);
        [vec![tag, body.len() as u8], body.to_vec()].concat()
    }
    for attached in [None, Some(b"".as_slice()), Some(b"content".as_slice())] {
        let mut content = der(6, b"\x2a\x86\x48\x86\xf7\x0d\x01\x07\x01");
        if let Some(bytes) = attached {
            content.extend(der(0xa0, &der(4, bytes)));
        }
        let data = [
            der(2, &[1]),
            der(0x31, &[]),
            der(0x30, &content),
            der(0x31, &[]),
        ]
        .concat();
        let outer = [
            der(6, b"\x2a\x86\x48\x86\xf7\x0d\x01\x07\x02"),
            der(0xa0, &der(0x30, &data)),
        ]
        .concat();
        assert_eq!(
            detached_cms(&der(0x30, &outer)),
            if attached.is_some() {
                Err(ATTACHED_CONTENT)
            } else {
                Ok(())
            }
        );
    }
}

fn openssl(dir: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new(test_identity::openssl())
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "OpenSSL certificate fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn weak_digests_and_attached_cms_are_rejected_before_a_valid_pdf_verdict() {
    let pfx = test_identity::identity();
    let signed = sign_pdf(
        include_bytes!("../tests/fixtures/two-pages.pdf"),
        &pfx,
        "test-only",
        &CertSignOptions::default(),
    )
    .unwrap();
    let parsed = lopdf::Document::load_mem(&signed).unwrap();
    let signature = parsed
        .objects
        .values()
        .filter_map(|o| o.as_dict().ok())
        .find(|d| d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Sig"))
        .unwrap();
    let range = signature
        .get(b"ByteRange")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_i64().unwrap() as usize)
        .collect::<Vec<_>>();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("identity.pfx"), pfx).unwrap();
    openssl(
        dir.path(),
        &[
            "pkcs12",
            "-in",
            "identity.pfx",
            "-passin",
            "pass:test-only",
            "-nodes",
            "-out",
            "identity.pem",
        ],
    );
    std::fs::write(
        dir.path().join("content.bin"),
        [&signed[..range[1]], &signed[range[2]..range[2] + range[3]]].concat(),
    )
    .unwrap();
    for digest in ["sha256", "sha1", "md5"] {
        openssl(
            dir.path(),
            &[
                "cms",
                "-sign",
                "-binary",
                "-md",
                digest,
                "-signer",
                "identity.pem",
                "-inkey",
                "identity.pem",
                "-in",
                "content.bin",
                "-outform",
                "DER",
                "-out",
                "signature.cms",
            ],
        );
        let cms = std::fs::read(dir.path().join("signature.cms")).unwrap();
        let report = verify_signatures(&replace_cms(&signed, &range, &cms), None).unwrap();
        if digest == "sha256" {
            assert_eq!(report[0].status, SignatureStatus::Valid);
        } else {
            assert_eq!(report[0].status, SignatureStatus::WeakDigest, "{digest}");
            assert_eq!(report[0].failure_cause(), Some("digest_not_allowed"));
            assert!(!report[0].identity_trusted());
        }
    }
    std::fs::write(dir.path().join("other.bin"), b"Other signed content").unwrap();
    openssl(
        dir.path(),
        &[
            "cms",
            "-sign",
            "-binary",
            "-nodetach",
            "-md",
            "sha256",
            "-signer",
            "identity.pem",
            "-inkey",
            "identity.pem",
            "-in",
            "other.bin",
            "-outform",
            "DER",
            "-out",
            "attached.cms",
        ],
    );
    let cms = std::fs::read(dir.path().join("attached.cms")).unwrap();
    let report = verify_signatures(&replace_cms(&signed, &range, &cms), None).unwrap();
    assert_eq!(report[0].status, SignatureStatus::Invalid);
    assert_eq!(
        report[0].failure_cause(),
        Some("attached_content_not_allowed")
    );
}

fn replace_cms(signed: &[u8], range: &[usize], cms: &[u8]) -> Vec<u8> {
    assert!(cms.len() * 2 <= range[2] - range[1] - 2);
    let mut output = signed.to_vec();
    let gap = &mut output[range[1] + 1..range[2] - 1];
    gap.fill(b'0');
    for (index, byte) in cms.iter().enumerate() {
        const HEX: &[u8] = b"0123456789ABCDEF";
        gap[index * 2] = HEX[usize::from(byte >> 4)];
        gap[index * 2 + 1] = HEX[usize::from(byte & 15)];
    }
    output
}

#[test]
fn certificate_names_include_distinguished_fields_and_multiple_private_identities_are_refused() {
    let first = test_identity::identity_with_subject(
        "test-only",
        "/C=CH/O=SuniPDF Test Organization/CN=First Identity",
    );
    let info = identity_info(&first, "test-only").unwrap();
    assert!(
        info.subject.contains("CN=")
            && info.subject.contains("First Identity")
            && info.subject.contains("O=")
            && info.subject.contains("SuniPDF Test Organization")
            && info.subject.contains("C=CH"),
        "{}",
        info.subject
    );
    let second = test_identity::identity_with_subject(
        "test-only",
        "/C=CH/O=SuniPDF Test Organization/CN=Second Identity",
    );
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("first.pfx"), first).unwrap();
    std::fs::write(dir.path().join("second.pfx"), second).unwrap();
    let script = r#"$ErrorActionPreference='Stop'; $flags=[System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::Exportable -bor [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet; $a=[System.Security.Cryptography.X509Certificates.X509Certificate2]::new([IO.File]::ReadAllBytes((Join-Path (Get-Location) 'first.pfx')),'test-only',$flags); $b=[System.Security.Cryptography.X509Certificates.X509Certificate2]::new([IO.File]::ReadAllBytes((Join-Path (Get-Location) 'second.pfx')),'test-only',$flags); try { $collection=[System.Security.Cryptography.X509Certificates.X509Certificate2Collection]::new(); [void]$collection.Add($a); [void]$collection.Add($b); [IO.File]::WriteAllBytes((Join-Path (Get-Location) 'multiple.pfx'),$collection.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pkcs12,'test-only')) } finally { $a.Dispose(); $b.Dispose() }"#;
    let output = std::process::Command::new("powershell.exe")
        .current_dir(dir.path())
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "PFX fixture export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let multiple = std::fs::read(dir.path().join("multiple.pfx")).unwrap();
    let error = identity_info(&multiple, "test-only").unwrap_err();
    assert!(
        error.to_string().contains("multiple signing identities"),
        "{error}"
    );
    assert!(sign_pdf(
        include_bytes!("../tests/fixtures/two-pages.pdf"),
        &multiple,
        "test-only",
        &CertSignOptions::default()
    )
    .is_err());
}
