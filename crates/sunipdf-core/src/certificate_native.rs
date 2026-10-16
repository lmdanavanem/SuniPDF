//! Windows owns the private key and performs all CMS cryptography.

use super::CertificateInfo;
use crate::EngineError;
use windows::core::{w, PCWSTR};
use windows::Win32::Security::Cryptography::*;

struct Store(HCERTSTORE);
impl Drop for Store {
    fn drop(&mut self) {
        // SAFETY: this handle is owned by this wrapper; certificate contexts are released first.
        unsafe {
            let _ = CertCloseStore(Some(self.0), 0);
        }
    }
}

struct Cert(*mut CERT_CONTEXT);
impl Drop for Cert {
    fn drop(&mut self) {
        // SAFETY: this is the owned context returned by enumeration or CMS verification.
        unsafe {
            let _ = CertFreeCertificateContext(Some(self.0));
        }
    }
}

struct Password(Vec<u16>);
impl Drop for Password {
    fn drop(&mut self) {
        for c in &mut self.0 {
            // SAFETY: each initialized element is exclusively borrowed; volatile prevents elision.
            unsafe {
                std::ptr::write_volatile(c, 0);
            }
        }
    }
}

fn error(e: windows::core::Error) -> EngineError {
    EngineError::Other(format!(
        "certificate operation failed (0x{:08X})",
        e.code().0 as u32
    ))
}

pub(super) struct Identity {
    cert: Cert,
    _store: Store,
}

impl Identity {
    pub(super) fn from_pfx(pfx: &[u8], password: &str) -> Result<Self, EngineError> {
        if password.contains('\0') {
            return Err(EngineError::Other(
                "certificate password contains a null character".into(),
            ));
        }
        let size = u32::try_from(pfx.len())
            .map_err(|_| EngineError::Other("certificate file is too large".into()))?;
        let empty_password = password.is_empty();
        let password = Password(password.encode_utf16().chain(Some(0)).collect());
        let blob = CRYPT_INTEGER_BLOB {
            cbData: size,
            pbData: pfx.as_ptr().cast_mut(),
        };
        // SAFETY: the input and null-terminated password live through the call. No key is persisted.
        let imported = unsafe {
            PFXImportCertStore(
                &blob,
                PCWSTR(password.0.as_ptr()),
                PKCS12_NO_PERSIST_KEY | PKCS12_PREFER_CNG_KSP,
            )
        };
        let imported = if imported.is_err() && empty_password {
            // SAFETY: the same live input is retried with the alternate empty-password convention.
            unsafe {
                PFXImportCertStore(
                    &blob,
                    PCWSTR::null(),
                    PKCS12_NO_PERSIST_KEY | PKCS12_PREFER_CNG_KSP,
                )
            }
        } else {
            imported
        };
        let store = Store(imported.map_err(error)?);
        let mut previous = None;
        loop {
            // SAFETY: enumeration releases its previous context and returns a fresh owned context.
            let cert = unsafe { CertEnumCertificatesInStore(store.0, previous) };
            if cert.is_null() {
                return Err(EngineError::Other("certificate has no private key".into()));
            }
            let mut size = 0;
            // SAFETY: the live certificate context is queried for the ephemeral private-key property.
            if unsafe {
                CertGetCertificateContextProperty(cert, CERT_KEY_CONTEXT_PROP_ID, None, &mut size)
            }
            .is_ok()
            {
                return Ok(Self {
                    cert: Cert(cert),
                    _store: store,
                });
            }
            previous = Some(cert.cast_const());
        }
    }

    pub(super) fn info(&self) -> Result<CertificateInfo, EngineError> {
        cert_info(self.cert.0)
    }

    pub(super) fn sign(&self, parts: &[&[u8]]) -> Result<Vec<u8>, EngineError> {
        let (ptrs, sizes) = buffers(parts)?;
        let mut certificates = Vec::new();
        let mut previous = None;
        loop {
            // SAFETY: enumeration owns and releases the previous context; duplicates below are independent.
            let cert = unsafe { CertEnumCertificatesInStore(self._store.0, previous) };
            if cert.is_null() {
                break;
            }
            // SAFETY: cert is live; the duplicate has an independently owned reference.
            let duplicate = unsafe { CertDuplicateCertificateContext(Some(cert)) };
            if !duplicate.is_null() {
                certificates.push(Cert(duplicate));
            }
            previous = Some(cert.cast_const());
        }
        let mut cert_ptrs = certificates.iter().map(|c| c.0).collect::<Vec<_>>();
        let para = CRYPT_SIGN_MESSAGE_PARA {
            cbSize: std::mem::size_of::<CRYPT_SIGN_MESSAGE_PARA>() as u32,
            dwMsgEncodingType: X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0,
            pSigningCert: self.cert.0,
            HashAlgorithm: CRYPT_ALGORITHM_IDENTIFIER {
                pszObjId: windows::core::PSTR(szOID_NIST_sha256.0.cast_mut()),
                ..Default::default()
            },
            cMsgCert: cert_ptrs.len() as u32,
            rgpMsgCert: cert_ptrs.as_mut_ptr(),
            dwFlags: CRYPT_MESSAGE_SILENT_KEYSET_FLAG,
            ..Default::default()
        };
        let mut out = vec![0; 16_384];
        let mut size = out.len() as u32;
        // SAFETY: all pointed-to buffers, parameter structures and certificate live through this call.
        unsafe {
            CryptSignMessage(
                &para,
                true,
                ptrs.len() as u32,
                Some(ptrs.as_ptr()),
                sizes.as_ptr(),
                Some(out.as_mut_ptr()),
                &mut size,
            )
        }
        .map_err(error)?;
        out.truncate(size as usize);
        Ok(out)
    }
}

fn buffers(parts: &[&[u8]]) -> Result<(Vec<*const u8>, Vec<u32>), EngineError> {
    let sizes = parts
        .iter()
        .map(|p| u32::try_from(p.len()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| EngineError::Other("signed data is too large".into()))?;
    Ok((parts.iter().map(|p| p.as_ptr()).collect(), sizes))
}

pub(super) fn verify(cms: &[u8], parts: &[&[u8]]) -> Result<CertificateInfo, u32> {
    if cms.len() > u32::MAX as usize {
        return Err(0x80070057);
    }
    let (ptrs, sizes) = buffers(parts).map_err(|_| 0x80070057u32)?;
    let para = CRYPT_VERIFY_MESSAGE_PARA {
        cbSize: std::mem::size_of::<CRYPT_VERIFY_MESSAGE_PARA>() as u32,
        dwMsgAndCertEncodingType: X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0,
        ..Default::default()
    };
    let mut cert = std::ptr::null_mut();
    // SAFETY: CMS and input slices remain alive; the successful call returns an owned context.
    let result = unsafe {
        CryptVerifyDetachedMessageSignature(
            &para,
            0,
            cms,
            ptrs.len() as u32,
            ptrs.as_ptr(),
            sizes.as_ptr(),
            Some(&mut cert),
        )
    };
    let cert = (!cert.is_null()).then(|| Cert(cert));
    result.map_err(|e| e.code().0 as u32)?;
    cert_info(cert.as_ref().ok_or(0x80092004u32)?.0).map_err(|_| 0x80092004u32)
}

fn cert_info(cert: *const CERT_CONTEXT) -> Result<CertificateInfo, EngineError> {
    let name = |flags| {
        // SAFETY: the caller holds a live context. First query obtains the required UTF-16 length.
        let n =
            unsafe { CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, flags, None, None) };
        let mut text = vec![0; n as usize];
        // SAFETY: text is initialized and sized according to the native length query.
        let written = unsafe {
            CertGetNameStringW(
                cert,
                CERT_NAME_SIMPLE_DISPLAY_TYPE,
                flags,
                None,
                Some(&mut text),
            )
        };
        String::from_utf16_lossy(&text[..written.saturating_sub(1) as usize])
    };
    // SAFETY: certificates returned by CryptoAPI have live CERT_INFO and encoded/serial buffers.
    let info = unsafe { &*(*cert).pCertInfo };
    let encoded = unsafe {
        std::slice::from_raw_parts((*cert).pbCertEncoded, (*cert).cbCertEncoded as usize)
    };
    let serial = unsafe {
        std::slice::from_raw_parts(info.SerialNumber.pbData, info.SerialNumber.cbData as usize)
    };
    let mut digest = [0u8; 32];
    let mut size = digest.len() as u32;
    // SAFETY: encoded is a live certificate buffer and digest has the advertised output capacity.
    unsafe {
        CryptHashCertificate2(
            w!("SHA256"),
            0,
            None,
            Some(encoded),
            Some(digest.as_mut_ptr()),
            &mut size,
        )
    }
    .map_err(error)?;
    let date = |t: windows::Win32::Foundation::FILETIME| {
        let ticks = (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        let seconds = (ticks / 10_000_000) as i64 - 11_644_473_600;
        chrono::DateTime::from_timestamp(seconds, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_default()
    };
    Ok(CertificateInfo {
        subject: name(0),
        issuer: name(CERT_NAME_ISSUER_FLAG),
        serial_hex: serial.iter().rev().map(|b| format!("{b:02X}")).collect(),
        sha256_hex: digest.iter().map(|b| format!("{b:02X}")).collect(),
        not_before: date(info.NotBefore),
        not_after: date(info.NotAfter),
        // SAFETY: info belongs to the live certificate context; a null time means current UTC.
        valid_now: unsafe { CertVerifyTimeValidity(None, info) } == 0,
    })
}
