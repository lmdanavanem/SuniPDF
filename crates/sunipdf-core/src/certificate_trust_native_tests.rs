use super::*;

#[test]
fn offline_chain_flags_disable_all_native_url_retrieval() {
    let flags = chain_flags(VerificationOptions::default());
    for required in [
        CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL,
        CERT_CHAIN_REVOCATION_CHECK_CACHE_ONLY,
        CERT_CHAIN_DISABLE_AIA,
        CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE,
        CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        CERT_CHAIN_REVOCATION_ACCUMULATIVE_TIMEOUT,
    ] {
        assert_eq!(flags & required, required);
    }
    let online = chain_flags(VerificationOptions {
        allow_network: true,
    });
    assert_eq!(
        online
            & (CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL
                | CERT_CHAIN_REVOCATION_CHECK_CACHE_ONLY
                | CERT_CHAIN_DISABLE_AIA),
        0
    );
    assert_ne!(online & CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE, 0);
}

#[test]
fn revocation_unknown_offline_and_revoked_are_distinct_from_good() {
    assert_eq!(
        chain_result(0, 0, true),
        (ChainTrust::Trusted, RevocationStatus::Good)
    );
    assert_eq!(
        chain_result(0, 0, false),
        (ChainTrust::Trusted, RevocationStatus::NotChecked)
    );
    assert_eq!(
        chain_result(CERT_TRUST_IS_REVOKED, 0x800B010C, true),
        (ChainTrust::Untrusted, RevocationStatus::Revoked)
    );
    assert_eq!(
        chain_result(
            CERT_TRUST_IS_OFFLINE_REVOCATION | CERT_TRUST_REVOCATION_STATUS_UNKNOWN,
            0,
            true
        ),
        (ChainTrust::Trusted, RevocationStatus::Offline)
    );
    assert_eq!(
        chain_result(CERT_TRUST_REVOCATION_STATUS_UNKNOWN, 0, true),
        (ChainTrust::Trusted, RevocationStatus::Unknown)
    );
    assert_eq!(
        chain_result(
            CERT_TRUST_IS_UNTRUSTED_ROOT | CERT_TRUST_REVOCATION_STATUS_UNKNOWN,
            0x800B0109,
            true
        ),
        (ChainTrust::Untrusted, RevocationStatus::Unknown)
    );
}

#[test]
fn native_arena_bounds_reject_null_unaligned_and_overflowing_values() {
    let arena = [1usize, 2, 3];
    assert_eq!(within(&arena, arena.as_ptr(), 3).unwrap(), &arena);
    assert!(within(&arena, std::ptr::null::<u32>(), 1).is_err());
    assert!(within(&arena, arena.as_ptr(), usize::MAX).is_err());
    assert!(within(
        &arena,
        arena.as_ptr().cast::<u8>().wrapping_add(1).cast::<u32>(),
        1
    )
    .is_err());
    assert!(within(&arena, arena.as_ptr().wrapping_add(3), 1).is_err());
}

#[test]
fn native_certificate_guards_reject_missing_context_and_info() {
    let empty = CERT_CONTEXT::default();
    for cert in [std::ptr::null(), &empty as *const CERT_CONTEXT] {
        assert_eq!(signing_key_usage(cert), Err(INVALID));
        assert_eq!(timestamp_eku(cert), Err(INVALID));
    }
    assert_eq!(
        chain(
            std::ptr::null(),
            HCERTSTORE::default(),
            VerificationOptions::default(),
            None,
            false
        ),
        Err(INVALID)
    );
}

#[test]
fn tsa_usage_and_future_time_are_rejected_without_network() {
    let pfx = crate::certificate::test_identity::identity();
    let identity = super::super::Identity::from_pfx(&pfx, "test-only").unwrap();
    assert!(timestamp_eku(identity.cert.0).is_err());
    let future = chrono::Utc::now().timestamp() + 86400;
    let ticks = (future + 11_644_473_600) as u64 * 10_000_000;
    assert!(timestamp_date(FILETIME {
        dwHighDateTime: (ticks >> 32) as u32,
        dwLowDateTime: ticks as u32
    })
    .is_err());
}

fn openssl(dir: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new(crate::certificate::test_identity::openssl())
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "OpenSSL test fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn timestamp_fixture(dir: &std::path::Path, data: &[u8], digest: &str) -> Vec<u8> {
    std::fs::write(dir.join("signature.bin"), data).unwrap();
    openssl(
        dir,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-subj",
            "/CN=SuniPDF Test TSA",
            "-days",
            "2",
            "-addext",
            "extendedKeyUsage=critical,timeStamping",
            "-addext",
            "keyUsage=critical,digitalSignature",
            "-keyout",
            "tsa.key",
            "-out",
            "tsa.crt",
        ],
    );
    std::fs::write(dir.join("tsa.cnf"), "[tsa]\ndefault_tsa=local\n[local]\nserial=serial\ncrypto_device=builtin\nsigner_cert=tsa.crt\ncerts=tsa.crt\nsigner_key=tsa.key\nsigner_digest=sha256\ndefault_policy=1.2.3.4\ndigests=sha256,sha1\naccuracy=secs:1\nordering=yes\ntsa_name=yes\ness_cert_id_chain=yes\ness_cert_id_alg=sha256\n").unwrap();
    std::fs::write(dir.join("serial"), "01\n").unwrap();
    openssl(
        dir,
        &[
            "ts",
            "-query",
            "-data",
            "signature.bin",
            digest,
            "-cert",
            "-out",
            "request.tsq",
        ],
    );
    openssl(
        dir,
        &[
            "ts",
            "-reply",
            "-config",
            "tsa.cnf",
            "-queryfile",
            "request.tsq",
            "-token_out",
            "-out",
            "token.der",
        ],
    );
    std::fs::read(dir.join("token.der")).unwrap()
}

fn der(tag: u8, bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    if bytes.len() < 128 {
        out.push(bytes.len() as u8);
    } else {
        let length = (bytes.len() as u32).to_be_bytes();
        let start = length.iter().position(|b| *b != 0).unwrap();
        out.push(0x80 | (4 - start) as u8);
        out.extend_from_slice(&length[start..]);
    }
    out.extend_from_slice(bytes);
    out
}

fn add_timestamp(message: &Message, token: &[u8]) -> Vec<u8> {
    let mut attribute = der(6, &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 9, 16, 2, 14]);
    attribute.extend_from_slice(&der(0x31, token));
    let attribute = der(0x30, &attribute);
    let parameters = CMSG_CTRL_ADD_SIGNER_UNAUTH_ATTR_PARA {
        cbSize: std::mem::size_of::<CMSG_CTRL_ADD_SIGNER_UNAUTH_ATTR_PARA>() as u32,
        dwSignerIndex: 0,
        blob: CRYPT_INTEGER_BLOB {
            cbData: attribute.len() as u32,
            pbData: attribute.as_ptr().cast_mut(),
        },
    };
    unsafe {
        CryptMsgControl(
            message.0,
            0,
            CMSG_CTRL_ADD_SIGNER_UNAUTH_ATTR,
            Some((&parameters as *const CMSG_CTRL_ADD_SIGNER_UNAUTH_ATTR_PARA).cast()),
        )
    }
    .unwrap();
    let mut size = 0;
    unsafe { CryptMsgGetParam(message.0, CMSG_ENCODED_MESSAGE, 0, None, &mut size) }.unwrap();
    let mut cms = vec![0u8; size as usize];
    unsafe {
        CryptMsgGetParam(
            message.0,
            CMSG_ENCODED_MESSAGE,
            0,
            Some(cms.as_mut_ptr().cast()),
            &mut size,
        )
    }
    .unwrap();
    cms.truncate(size as usize);
    cms
}

#[test]
fn rfc3161_token_is_bound_to_the_cms_signature_and_self_signed_tsa_is_untrusted() {
    let pfx = crate::certificate::test_identity::identity();
    let identity = super::super::Identity::from_pfx(&pfx, "test-only").unwrap();
    let cms = identity.sign(&[b"signed PDF bytes"]).unwrap();
    let (store, message) = query(&cms).unwrap();
    let data = parameter(&message, CMSG_SIGNER_INFO_PARAM).unwrap();
    let signer = within(&data, data.as_ptr().cast::<CMSG_SIGNER_INFO>(), 1).unwrap()[0];
    let signature = within(
        &data,
        signer.EncryptedHash.pbData,
        signer.EncryptedHash.cbData as usize,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let token = timestamp_fixture(dir.path(), signature, "-sha256");
    let timestamp = verify_timestamp(&token, signature, store.0, VerificationOptions::default());
    assert_eq!(
        timestamp.status,
        TimestampStatus::Untrusted,
        "{timestamp:?}"
    );
    assert!(timestamp.time.is_some());
    assert_eq!(timestamp.chain, ChainTrust::Untrusted);
    let with_timestamp = add_timestamp(&message, &token);
    let (info, trust) = super::super::verify_trust(
        &with_timestamp,
        &[b"signed PDF bytes"],
        VerificationOptions::default(),
    )
    .unwrap();
    assert!(info.subject.starts_with("CN=") && info.subject.contains("SuniPDF Test Identity"));
    assert_eq!(trust.timestamp.status, TimestampStatus::Untrusted);
    assert!(!trust.chain_and_revocation_trusted() && !trust.network_allowed);
    let duplicate = add_timestamp(&message, &token);
    let (_, ambiguous) = super::super::verify_trust(
        &duplicate,
        &[b"signed PDF bytes"],
        VerificationOptions::default(),
    )
    .unwrap();
    assert_eq!(ambiguous.timestamp.status, TimestampStatus::Invalid);
    assert_eq!(
        verify_timestamp(
            &token,
            b"different signature",
            store.0,
            VerificationOptions::default()
        )
        .status,
        TimestampStatus::Invalid
    );
    let weak_dir = tempfile::tempdir().unwrap();
    let weak = timestamp_fixture(weak_dir.path(), signature, "-sha1");
    assert_eq!(
        verify_timestamp(&weak, signature, store.0, VerificationOptions::default()).status,
        TimestampStatus::Invalid
    );
    assert_eq!(
        verify_timestamp(
            &token[..token.len() - 10],
            signature,
            store.0,
            VerificationOptions::default()
        )
        .status,
        TimestampStatus::Invalid
    );
}
