use sunipdf_core::{security, Engine, EngineError, Protection, ProtectionProblem};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

#[test]
fn aes256_rejects_passwords_that_normalize_to_empty() {
    for owner in ["", "owner"] {
        let p = Protection {
            user_password: "\u{ad}".into(),
            owner_password: owner.into(),
            ..Default::default()
        };
        assert_eq!(p.validate(), Err(ProtectionProblem::NoPassword));
        assert!(security::protect(PDF, None, &p).is_err());
    }
}

#[test]
fn restrictions_require_distinct_normalized_passwords() {
    let p = Protection {
        user_password: "a b".into(),
        owner_password: "a\u{a0}b".into(),
        allow_copy: false,
        ..Default::default()
    };
    assert_eq!(p.validate(), Err(ProtectionProblem::OwnerNeeded));
    assert!(security::protect(PDF, None, &p).is_err());
}

#[test]
fn aes128_restrictions_require_distinct_first_32_encoded_bytes() {
    let prefix = "é".repeat(32);
    let p = Protection {
        user_password: format!("{prefix}user"),
        owner_password: format!("{prefix}owner"),
        allow_copy: false,
        aes256: false,
        ..Default::default()
    };
    assert_eq!(p.validate(), Err(ProtectionProblem::OwnerNeeded));
    assert!(security::protect(PDF, None, &p).is_err());
}

#[test]
fn aes128_padding_collision_is_not_a_distinct_owner_password() {
    let prefix = "a".repeat(31);
    let p = Protection {
        user_password: prefix.clone(),
        owner_password: format!("{prefix}("),
        allow_copy: false,
        aes256: false,
        ..Default::default()
    };
    assert_eq!(
        (p.validate(), security::protect(PDF, None, &p).is_err()),
        (Err(ProtectionProblem::OwnerNeeded), true)
    );
    let distinct = Protection {
        owner_password: format!("{prefix})"),
        ..p
    };
    assert_eq!(distinct.validate(), Ok(()));
    assert!(security::protect(PDF, None, &distinct).is_ok());
}

#[test]
fn aes256_restrictions_require_distinct_first_127_bytes() {
    let prefix = "a".repeat(127);
    let p = Protection {
        user_password: format!("{prefix}user"),
        owner_password: format!("{prefix}owner"),
        allow_copy: false,
        ..Default::default()
    };
    assert_eq!(p.validate(), Err(ProtectionProblem::OwnerNeeded));
    assert!(security::protect(PDF, None, &p).is_err());
}

#[test]
fn owner_only_protection_keeps_open_access() {
    let p = Protection {
        owner_password: "owner".into(),
        allow_copy: false,
        ..Default::default()
    };
    let bytes = security::protect(PDF, None, &p).unwrap();
    let doc = lopdf::Document::load_mem_with_options(&bytes, lopdf::LoadOptions::with_password(""))
        .unwrap();
    assert!(!doc.is_encrypted());
    assert!(matches!(
        security::unprotect(&bytes, ""),
        Err(EngineError::PasswordRequired)
    ));
    assert!(security::unprotect(&bytes, "owner").is_ok());
}

#[test]
fn long_aes256_passwords_round_trip_as_typed() {
    let engine = Engine::new().unwrap();
    for password in ["a".repeat(128), format!("{}é", "a".repeat(126))] {
        let p = Protection {
            user_password: password.clone(),
            owner_password: format!("owner-{password}"),
            allow_copy: false,
            ..Default::default()
        };
        let bytes = security::protect(PDF, None, &p).unwrap();
        let doc = engine
            .open_bytes(
                bytes.clone(),
                Some(&password),
                std::path::Path::new("long.pdf"),
            )
            .unwrap();
        assert_eq!(doc.page_count(), 2);
        assert!(!doc.properties().can_copy);
        assert!(matches!(
            security::unprotect(&bytes, &password),
            Err(EngineError::PasswordRequired)
        ));
        let unprotected = security::unprotect(&bytes, &p.owner_password).unwrap();
        let doc = engine
            .open_bytes(unprotected, None, std::path::Path::new("unprotected.pdf"))
            .unwrap();
        assert!(!doc.is_encrypted());
    }
}
