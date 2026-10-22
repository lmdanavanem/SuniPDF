use super::*;

#[cfg(windows)]
#[test]
fn valid_saved_bytes_do_not_trust_a_self_signed_identity_or_declared_time() {
    let pdf = include_bytes!("../tests/fixtures/two-pages.pdf");
    let pfx = test_identity::identity();
    let signed = sign_pdf(pdf, &pfx, "test-only", &CertSignOptions::default()).unwrap();
    let result = verify_signatures(&signed, None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].status, SignatureStatus::Valid);
    assert_eq!(result[0].trust.chain, ChainTrust::Untrusted);
    assert!(!result[0].identity_trusted());
    assert!(!result[0].trust.network_allowed);
    assert_eq!(result[0].trust.timestamp.status, TimestampStatus::Absent);
    assert!(result[0].trust.timestamp.time.is_none());
}

#[test]
fn archival_material_is_never_treated_as_authenticated_ltv() {
    let mut pdf =
        lopdf::Document::load_mem(include_bytes!("../tests/fixtures/two-pages.pdf")).unwrap();
    assert_eq!(ltv_status(&pdf), LtvStatus::Absent);
    let cert = pdf.add_object(lopdf::Stream::new(
        Dictionary::new(),
        b"untrusted certificate bytes".to_vec(),
    ));
    let dss = pdf.add_object(dictionary! { "Certs" => vec![Object::Reference(cert)] });
    pdf.catalog_mut().unwrap().set("DSS", dss);
    assert_eq!(ltv_status(&pdf), LtvStatus::EmbeddedUnverified);
    pdf.get_dictionary_mut(dss)
        .unwrap()
        .set("OCSPs", Object::Integer(4));
    assert_eq!(ltv_status(&pdf), LtvStatus::Malformed);
    pdf.get_dictionary_mut(dss).unwrap().remove(b"OCSPs");
    pdf.get_dictionary_mut(dss)
        .unwrap()
        .set("Certs", vec![Object::Reference(cert); 257]);
    assert_eq!(ltv_status(&pdf), LtvStatus::Malformed);
}

#[test]
fn identity_requires_successful_chain_and_revocation_separately() {
    for chain in [
        ChainTrust::Trusted,
        ChainTrust::Untrusted,
        ChainTrust::Unavailable,
    ] {
        for revocation in [
            RevocationStatus::Good,
            RevocationStatus::Revoked,
            RevocationStatus::Offline,
            RevocationStatus::Unknown,
            RevocationStatus::NotChecked,
        ] {
            let report = SignatureTrust {
                chain,
                revocation,
                ..Default::default()
            };
            assert_eq!(
                report.chain_and_revocation_trusted(),
                chain == ChainTrust::Trusted && revocation == RevocationStatus::Good
            );
        }
    }
}
