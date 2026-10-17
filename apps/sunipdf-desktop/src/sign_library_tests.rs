use super::*;

fn mark(color: u8) -> RgbaImage {
    RgbaImage::from_pixel(16, 8, image::Rgba([color, 20, 30, 255]))
}

fn png(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::write_buffer_with_format(
        &mut Cursor::new(&mut bytes),
        image.as_raw(),
        image.width(),
        image.height(),
        image::ColorType::Rgba8,
        ImageFormat::Png,
    )
    .unwrap();
    bytes
}

#[test]
#[cfg(windows)]
fn saved_signature_is_dpapi_protected_round_trips_and_deletes() {
    let dir = tempfile::tempdir().unwrap();
    let original = mark(10);
    let saved = save(dir.path(), "signature-1700000000000.png", &original).unwrap();
    assert_eq!(saved.extension().unwrap(), "dpapi");
    assert_eq!(
        super::super::list_saved(dir.path()),
        vec![(saved.clone(), super::super::SignKind::Signature)]
    );
    let bytes = std::fs::read(&saved).unwrap();
    assert!(bytes.starts_with(MAGIC));
    assert!(!bytes.windows(8).any(|b| b == b"\x89PNG\r\n\x1a\n"));
    assert!(!saved.with_extension("png").exists());
    assert_eq!(load(dir.path(), &saved).unwrap(), (saved.clone(), original));
    let mut tampered = bytes;
    *tampered.last_mut().unwrap() ^= 1;
    assert!(unprotect(&tampered).is_err());

    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };
    let name: Vec<u16> = saved.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut needed = 0;
    unsafe {
        GetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
    }
    assert!(needed > 0);
    let mut security = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    assert_ne!(
        unsafe {
            GetFileSecurityW(
                name.as_ptr(),
                DACL_SECURITY_INFORMATION,
                security.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        },
        0
    );
    let mut control = 0;
    let mut revision = 0;
    assert_ne!(
        unsafe {
            GetSecurityDescriptorControl(
                security.as_ptr().cast_mut().cast(),
                &mut control,
                &mut revision,
            )
        },
        0
    );
    assert_ne!(control & SE_DACL_PROTECTED, 0);
    let mut text = std::ptr::null_mut();
    assert_ne!(
        unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                security.as_ptr().cast_mut().cast(),
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                std::ptr::null_mut(),
            )
        },
        0
    );
    let mut len = 0;
    while unsafe { *text.add(len) } != 0 {
        len += 1;
    }
    let sddl = String::from_utf16(unsafe { std::slice::from_raw_parts(text, len) }).unwrap();
    unsafe {
        LocalFree(text.cast());
    }
    let expected: Vec<u16> = format!("D:P(A;;FA;;;{})", native::sid().unwrap())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut expected_descriptor = std::ptr::null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                expected.as_ptr(),
                SDDL_REVISION_1,
                &mut expected_descriptor,
                std::ptr::null_mut(),
            )
        },
        0
    );
    assert!(!expected_descriptor.is_null());
    let mut expected_text = std::ptr::null_mut();
    let converted = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            expected_descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut expected_text,
            std::ptr::null_mut(),
        )
    };
    unsafe { LocalFree(expected_descriptor) };
    assert_ne!(converted, 0);
    assert!(!expected_text.is_null());
    let mut expected_len = 0;
    while unsafe { *expected_text.add(expected_len) } != 0 {
        expected_len += 1;
    }
    let expected_sddl =
        String::from_utf16(unsafe { std::slice::from_raw_parts(expected_text, expected_len) })
            .unwrap();
    unsafe { LocalFree(expected_text.cast()) };
    assert_eq!(sddl, expected_sddl, "exact protected current-user DACL");
    delete(dir.path(), &saved).unwrap();
    assert!(!saved.exists());
}

#[test]
#[cfg(windows)]
fn legacy_migration_verifies_persisted_bytes_before_removing_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("initials-1700000000000.png");
    let image = mark(50);
    let original = png(&image);
    std::fs::write(&legacy, &original).unwrap();
    let (saved, decoded) = load(dir.path(), &legacy).unwrap();
    assert_eq!(decoded, image);
    assert_eq!(saved, legacy.with_extension("dpapi"));
    assert!(!legacy.exists());
    assert_eq!(
        &*unprotect(&std::fs::read(&saved).unwrap()).unwrap(),
        &original
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
#[cfg(windows)]
fn conflicting_encrypted_copy_preserves_both_originals_and_keeps_legacy_usable() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("signature-1700000000000.png");
    let image = mark(50);
    let original = png(&image);
    std::fs::write(&legacy, &original).unwrap();
    let encrypted = save(
        dir.path(),
        legacy.file_name().unwrap().to_str().unwrap(),
        &mark(100),
    )
    .unwrap();
    let previous = std::fs::read(&encrypted).unwrap();
    assert_eq!(load(dir.path(), &legacy).unwrap(), (legacy.clone(), image));
    assert_eq!(std::fs::read(&legacy).unwrap(), original);
    assert_eq!(std::fs::read(&encrypted).unwrap(), previous);
    assert!(migrate(dir.path(), &legacy, &original).is_err());
}

#[test]
#[cfg(windows)]
fn profile_identifier_mismatch_and_corruption_never_replace_or_delete_ciphertext() {
    let dir = tempfile::tempdir().unwrap();
    let saved = save(dir.path(), "signature-1700000000000.png", &mark(40)).unwrap();
    let mut bytes = std::fs::read(&saved).unwrap();
    let length = u16::from_le_bytes([bytes[MAGIC.len()], bytes[MAGIC.len() + 1]]) as usize;
    let last_sid_byte = MAGIC.len() + 2 + length - 1;
    bytes[last_sid_byte] = if bytes[last_sid_byte] == b'1' {
        b'2'
    } else {
        b'1'
    };
    std::fs::write(&saved, &bytes).unwrap();
    assert!(load(dir.path(), &saved)
        .unwrap_err()
        .to_string()
        .contains("another Windows profile"));
    assert_eq!(std::fs::read(&saved).unwrap(), bytes);
    for invalid in [b"bad".as_slice(), MAGIC, b"SUNISIG\x01\xff\xff"] {
        assert!(unprotect(invalid).is_err());
    }
}

#[test]
fn unsafe_paths_invalid_png_and_oversized_blob_preserve_originals() {
    let dir = tempfile::tempdir().unwrap();
    let invalid = dir.path().join("signature-1700000000000.png");
    std::fs::write(&invalid, b"not a PNG").unwrap();
    assert!(load(dir.path(), &invalid).is_err());
    assert_eq!(std::fs::read(&invalid).unwrap(), b"not a PNG");
    assert!(!invalid.with_extension("dpapi").exists());
    assert!(save(dir.path(), "../signature-1700000000001.png", &mark(40)).is_err());
    assert!(delete(
        dir.path(),
        &dir.path().join("../signature-1700000000001.png")
    )
    .is_err());
    let large = dir.path().join("initials-1700000000000.dpapi");
    let file = std::fs::File::create(&large).unwrap();
    file.set_len(MAX_BLOB + 1).unwrap();
    drop(file);
    assert!(load(dir.path(), &large).is_err());
    assert_eq!(std::fs::metadata(&large).unwrap().len(), MAX_BLOB + 1);
}
