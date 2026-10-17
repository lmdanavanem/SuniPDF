//! Windows chain policy and RFC3161 verification; only explicit options permit URL retrieval.

use super::{Cert, Store};
use crate::certificate::{
    ChainTrust, RevocationStatus, SignatureTrust, TimestampStatus, TimestampVerification,
    VerificationOptions,
};
use windows::core::{s, PSTR};
use windows::Win32::{Foundation::FILETIME, Security::Cryptography::*};

const INVALID: u32 = 0x8009200D;
const MAX_NATIVE_BYTES: usize = 16 * 1024 * 1024;

struct Message(*mut std::ffi::c_void);
impl Drop for Message {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the decoded-message handle.
        unsafe {
            let _ = CryptMsgClose(Some(self.0));
        }
    }
}

struct Chain(*mut CERT_CHAIN_CONTEXT);
impl Drop for Chain {
    fn drop(&mut self) {
        // SAFETY: the chain was allocated by CertGetCertificateChain and is owned here.
        unsafe {
            CertFreeCertificateChain(self.0);
        }
    }
}

struct Timestamp(*mut CRYPT_TIMESTAMP_CONTEXT);
impl Drop for Timestamp {
    fn drop(&mut self) {
        // SAFETY: timestamp contexts are released with the allocator specified by CryptoAPI.
        unsafe {
            CryptMemFree(Some(self.0.cast()));
        }
    }
}

fn query(cms: &[u8]) -> Result<(Store, Message), u32> {
    if cms.is_empty() || cms.len() > MAX_NATIVE_BYTES {
        return Err(INVALID);
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: cms.len() as u32,
        pbData: cms.as_ptr().cast_mut(),
    };
    let mut store = HCERTSTORE::default();
    let mut message = std::ptr::null_mut();
    // SAFETY: the blob remains live; successful handles are wrapped before any later failure.
    let result = unsafe {
        CryptQueryObject(
            CERT_QUERY_OBJECT_BLOB,
            (&input as *const CRYPT_INTEGER_BLOB).cast(),
            CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED,
            CERT_QUERY_FORMAT_FLAG_BINARY,
            0,
            None,
            None,
            None,
            Some(&mut store),
            Some(&mut message),
            None,
        )
    };
    let store = if store.0.is_null() {
        None
    } else {
        Some(Store(store))
    };
    let message = if message.is_null() {
        None
    } else {
        Some(Message(message))
    };
    result.map_err(|e| e.code().0 as u32)?;
    let store = store.ok_or(INVALID)?;
    let message = message.ok_or(INVALID)?;
    for (kind, maximum) in [
        (CMSG_SIGNER_COUNT_PARAM, 1),
        (CMSG_CERT_COUNT_PARAM, 256),
        (CMSG_CRL_COUNT_PARAM, 256),
    ] {
        let data = parameter(&message, kind)?;
        let count = within(&data, data.as_ptr().cast::<u32>(), 1)?[0];
        if count > maximum || (kind == CMSG_SIGNER_COUNT_PARAM && count != 1) {
            return Err(INVALID);
        }
    }
    Ok((store, message))
}

fn parameter(message: &Message, kind: u32) -> Result<Vec<usize>, u32> {
    let mut size = 0;
    // SAFETY: the message is live; the first query obtains the aligned buffer's required length.
    unsafe { CryptMsgGetParam(message.0, kind, 0, None, &mut size) }
        .map_err(|e| e.code().0 as u32)?;
    if size == 0 || size as usize > MAX_NATIVE_BYTES {
        return Err(INVALID);
    }
    let mut data = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        CryptMsgGetParam(
            message.0,
            kind,
            0,
            Some(data.as_mut_ptr().cast()),
            &mut size,
        )
    }
    .map_err(|e| e.code().0 as u32)?;
    if size as usize > data.len() * std::mem::size_of::<usize>() {
        return Err(INVALID);
    }
    Ok(data)
}

fn within<T>(arena: &[usize], pointer: *const T, count: usize) -> Result<&[T], u32> {
    if count == 0 {
        return Ok(&[]);
    }
    let start = arena.as_ptr() as usize;
    let end = start
        .checked_add(std::mem::size_of_val(arena))
        .ok_or(INVALID)?;
    let address = pointer as usize;
    let size = count.checked_mul(std::mem::size_of::<T>()).ok_or(INVALID)?;
    if address < start
        || address.checked_add(size).is_none_or(|last| last > end)
        || !address.is_multiple_of(std::mem::align_of::<T>())
    {
        return Err(INVALID);
    }
    // SAFETY: alignment, checked multiplication and complete containment were validated above.
    Ok(unsafe { std::slice::from_raw_parts(pointer, count) })
}

fn oid(arena: &[usize], pointer: PSTR) -> Result<&[u8], u32> {
    let offset = (pointer.0 as usize)
        .checked_sub(arena.as_ptr() as usize)
        .ok_or(INVALID)?;
    let left = std::mem::size_of_val(arena)
        .checked_sub(offset)
        .ok_or(INVALID)?;
    let bytes = within(arena, pointer.0, left.min(128))?;
    let end = bytes.iter().position(|b| *b == 0).ok_or(INVALID)?;
    Ok(&bytes[..end])
}

fn chain_flags(options: VerificationOptions) -> u32 {
    let base = CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE
        | CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT
        | CERT_CHAIN_REVOCATION_ACCUMULATIVE_TIMEOUT;
    if options.allow_network {
        base
    } else {
        base | CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL
            | CERT_CHAIN_REVOCATION_CHECK_CACHE_ONLY
            | CERT_CHAIN_DISABLE_AIA
    }
}

fn chain_result(
    errors: u32,
    policy_error: u32,
    checked_leaf: bool,
) -> (ChainTrust, RevocationStatus) {
    let revocation = if errors & CERT_TRUST_IS_REVOKED != 0 {
        RevocationStatus::Revoked
    } else if errors & CERT_TRUST_IS_OFFLINE_REVOCATION != 0 {
        RevocationStatus::Offline
    } else if errors & CERT_TRUST_REVOCATION_STATUS_UNKNOWN != 0 {
        RevocationStatus::Unknown
    } else if checked_leaf {
        RevocationStatus::Good
    } else {
        RevocationStatus::NotChecked
    };
    let chain_errors =
        errors & !(CERT_TRUST_REVOCATION_STATUS_UNKNOWN | CERT_TRUST_IS_OFFLINE_REVOCATION);
    let trust = if chain_errors == 0 && policy_error == 0 {
        ChainTrust::Trusted
    } else {
        ChainTrust::Untrusted
    };
    (trust, revocation)
}

fn strong_digest(message: &Message) -> Result<(), u32> {
    let data = parameter(message, CMSG_SIGNER_INFO_PARAM)?;
    let signer = within(&data, data.as_ptr().cast::<CMSG_SIGNER_INFO>(), 1)?[0];
    if !matches!(
        oid(&data, signer.HashAlgorithm.pszObjId)?,
        b"2.16.840.1.101.3.4.2.1" | b"2.16.840.1.101.3.4.2.2" | b"2.16.840.1.101.3.4.2.3"
    ) {
        return Err(0x80090008);
    }
    Ok(())
}

fn signing_key_usage(cert: *const CERT_CONTEXT) -> Result<(), u32> {
    if cert.is_null() {
        return Err(INVALID);
    }
    let info = unsafe { (*cert).pCertInfo.as_ref() }.ok_or(INVALID)?;
    if info.cExtension > 256 || (info.cExtension != 0 && info.rgExtension.is_null()) {
        return Err(INVALID);
    }
    let extensions = if info.cExtension == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(info.rgExtension, info.cExtension as usize) }
    };
    if unsafe { CertFindExtension(szOID_KEY_USAGE, extensions) }.is_null() {
        return Ok(());
    }
    let mut usage = [0u8; 2];
    unsafe { CertGetIntendedKeyUsage(X509_ASN_ENCODING, info, &mut usage) }
        .map_err(|e| e.code().0 as u32)?;
    if u32::from(usage[0]) & (CERT_DIGITAL_SIGNATURE_KEY_USAGE | CERT_NON_REPUDIATION_KEY_USAGE)
        == 0
    {
        return Err(0x800B0110);
    }
    Ok(())
}

fn chain(
    cert: *const CERT_CONTEXT,
    store: HCERTSTORE,
    options: VerificationOptions,
    time: Option<&FILETIME>,
    timestamp: bool,
) -> Result<(ChainTrust, RevocationStatus, Option<u32>), u32> {
    if cert.is_null() {
        return Err(INVALID);
    }
    let mut usages = if timestamp {
        vec![PSTR(szOID_PKIX_KP_TIMESTAMP_SIGNING.0.cast_mut())]
    } else {
        vec![
            PSTR(s!("1.3.6.1.5.5.7.3.36").0.cast_mut()),
            PSTR(s!("1.3.6.1.4.1.311.10.3.12").0.cast_mut()),
            PSTR(s!("1.2.840.113583.1.1.5").0.cast_mut()),
        ]
    };
    let mut strong = CERT_STRONG_SIGN_PARA {
        cbSize: std::mem::size_of::<CERT_STRONG_SIGN_PARA>() as u32,
        dwInfoChoice: CERT_STRONG_SIGN_OID_INFO_CHOICE,
        Anonymous: CERT_STRONG_SIGN_PARA_0 {
            pszOID: PSTR(s!("1.3.6.1.4.1.311.72.1.1").0.cast_mut()),
        },
    };
    let parameters = CERT_CHAIN_PARA {
        cbSize: std::mem::size_of::<CERT_CHAIN_PARA>() as u32,
        RequestedUsage: CERT_USAGE_MATCH {
            dwType: if timestamp {
                USAGE_MATCH_TYPE_AND
            } else {
                USAGE_MATCH_TYPE_OR
            },
            Usage: CTL_USAGE {
                cUsageIdentifier: usages.len() as u32,
                rgpszUsageIdentifier: usages.as_mut_ptr(),
            },
        },
        dwUrlRetrievalTimeout: 5000,
        pStrongSignPara: &mut strong,
        ..Default::default()
    };
    let mut context = std::ptr::null_mut();
    // SAFETY: all native buffers and certificate/store contexts live through the bounded chain build.
    let result = unsafe {
        CertGetCertificateChain(
            None,
            cert,
            time.map(|t| t as *const _),
            Some(store),
            &parameters,
            chain_flags(options),
            None,
            &mut context,
        )
    };
    let context = if context.is_null() {
        None
    } else {
        Some(Chain(context))
    };
    result.map_err(|e| e.code().0 as u32)?;
    let context = context.ok_or(INVALID)?;
    let policy = CERT_CHAIN_POLICY_PARA {
        cbSize: std::mem::size_of::<CERT_CHAIN_POLICY_PARA>() as u32,
        dwFlags: CERT_CHAIN_POLICY_IGNORE_ALL_REV_UNKNOWN_FLAGS,
        ..Default::default()
    };
    let mut status = CERT_CHAIN_POLICY_STATUS {
        cbSize: std::mem::size_of::<CERT_CHAIN_POLICY_STATUS>() as u32,
        ..Default::default()
    };
    // SAFETY: context and policy structures are live; unknown revocation remains separately reported.
    if !unsafe {
        CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_BASE, context.0, &policy, &mut status)
    }
    .as_bool()
    {
        return Err(windows::core::Error::from_thread().code().0 as u32);
    }
    let native = unsafe { context.0.as_ref() }.ok_or(INVALID)?;
    let checked_leaf = if native.cChain == 0 {
        false
    } else {
        let first = unsafe { native.rgpChain.as_ref() }.ok_or(INVALID)?;
        let first = unsafe { (*first).as_ref() }.ok_or(INVALID)?;
        first.cElement > 1
    };
    let (trust, revocation) = chain_result(
        native.TrustStatus.dwErrorStatus,
        status.dwError,
        checked_leaf,
    );
    let detail = (native.TrustStatus.dwErrorStatus != 0)
        .then_some(native.TrustStatus.dwErrorStatus)
        .or((status.dwError != 0).then_some(status.dwError));
    Ok((trust, revocation, detail))
}

fn timestamp_eku(cert: *const CERT_CONTEXT) -> Result<(), u32> {
    if cert.is_null() {
        return Err(INVALID);
    }
    let info = unsafe { (*cert).pCertInfo.as_ref() }.ok_or(INVALID)?;
    if info.cExtension > 256 || (info.cExtension != 0 && info.rgExtension.is_null()) {
        return Err(INVALID);
    }
    let extensions = if info.cExtension == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(info.rgExtension, info.cExtension as usize) }
    };
    let extension = unsafe { CertFindExtension(szOID_ENHANCED_KEY_USAGE, extensions).as_ref() }
        .ok_or(INVALID)?;
    if !extension.fCritical.as_bool() {
        return Err(INVALID);
    }
    let mut size = 0;
    unsafe {
        CertGetEnhancedKeyUsage(
            cert,
            CERT_FIND_EXT_ONLY_ENHKEY_USAGE_FLAG.0,
            None,
            &mut size,
        )
    }
    .map_err(|e| e.code().0 as u32)?;
    if size as usize > 65536 || size < std::mem::size_of::<CTL_USAGE>() as u32 {
        return Err(INVALID);
    }
    let mut data = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        CertGetEnhancedKeyUsage(
            cert,
            CERT_FIND_EXT_ONLY_ENHKEY_USAGE_FLAG.0,
            Some(data.as_mut_ptr().cast()),
            &mut size,
        )
    }
    .map_err(|e| e.code().0 as u32)?;
    let usage = within(&data, data.as_ptr().cast::<CTL_USAGE>(), 1)?[0];
    if usage.cUsageIdentifier != 1 {
        return Err(INVALID);
    }
    let pointer = within(&data, usage.rgpszUsageIdentifier, 1)?[0];
    if oid(&data, pointer)? != b"1.3.6.1.5.5.7.3.8" {
        return Err(INVALID);
    }
    Ok(())
}

fn timestamp_date(time: FILETIME) -> Result<chrono::DateTime<chrono::Utc>, u32> {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    let seconds = i64::try_from(ticks / 10_000_000).map_err(|_| INVALID)? - 11_644_473_600;
    let date = chrono::DateTime::from_timestamp(seconds, 0).ok_or(INVALID)?;
    if date > chrono::Utc::now() + chrono::Duration::minutes(5) {
        return Err(INVALID);
    }
    Ok(date)
}

fn verify_timestamp(
    token: &[u8],
    signature: &[u8],
    store: HCERTSTORE,
    options: VerificationOptions,
) -> TimestampVerification {
    let inspect = || -> Result<TimestampVerification, u32> {
        if token.is_empty()
            || token.len() > 1024 * 1024
            || signature.is_empty()
            || signature.len() > 16384
        {
            return Err(INVALID);
        }
        let (_token_certificates, token_message) = query(token)?;
        strong_digest(&token_message)?;
        let mut context = std::ptr::null_mut();
        let mut signer = std::ptr::null_mut();
        let mut token_store = HCERTSTORE::default();
        // SAFETY: the token is cryptographically bound to the original CMS signature value by CryptoAPI.
        let result = unsafe {
            CryptVerifyTimeStampSignature(
                token,
                Some(signature),
                Some(store),
                &mut context,
                &mut signer,
                Some(&mut token_store),
            )
        };
        let context = if context.is_null() {
            None
        } else {
            Some(Timestamp(context))
        };
        let signer = if signer.is_null() {
            None
        } else {
            Some(Cert(signer))
        };
        let token_store = if token_store.0.is_null() {
            None
        } else {
            Some(Store(token_store))
        };
        result.map_err(|e| e.code().0 as u32)?;
        let context = context.ok_or(INVALID)?;
        let signer = signer.ok_or(INVALID)?;
        let token_store = token_store.ok_or(INVALID)?;
        let native = unsafe { context.0.as_ref() }.ok_or(INVALID)?;
        let time = unsafe { native.pTimeStamp.as_ref() }.ok_or(INVALID)?;
        let algorithm = time.HashAlgorithm.pszObjId;
        if algorithm.is_null() {
            return Err(INVALID);
        }
        // SAFETY: the timestamp verifier returns a terminated decoded OID string.
        let algorithm = unsafe { std::ffi::CStr::from_ptr(algorithm.0.cast()) }.to_bytes();
        if !matches!(
            algorithm,
            b"2.16.840.1.101.3.4.2.1" | b"2.16.840.1.101.3.4.2.2" | b"2.16.840.1.101.3.4.2.3"
        ) {
            return Err(0x80090008);
        }
        let date = timestamp_date(time.ftTime)?;
        timestamp_eku(signer.0)?;
        signing_key_usage(signer.0)?;
        let native = unsafe { signer.0.as_ref() }.ok_or(INVALID)?;
        let info = unsafe { native.pCertInfo.as_ref() }.ok_or(INVALID)?;
        if unsafe { CertVerifyTimeValidity(Some(&time.ftTime), info) } != 0 {
            return Err(0x800B0101);
        }
        let (chain, revocation, detail) =
            match chain(signer.0, token_store.0, options, Some(&time.ftTime), true) {
                Ok(result) => result,
                Err(code) => {
                    return Ok(TimestampVerification {
                        status: TimestampStatus::Unavailable,
                        time: Some(date.to_rfc3339()),
                        detail: Some(code),
                        ..Default::default()
                    })
                }
            };
        let status = if chain == ChainTrust::Trusted && revocation == RevocationStatus::Good {
            TimestampStatus::Trusted
        } else if chain == ChainTrust::Trusted {
            TimestampStatus::Unavailable
        } else {
            TimestampStatus::Untrusted
        };
        Ok(TimestampVerification {
            status,
            time: Some(date.to_rfc3339()),
            chain,
            revocation,
            detail,
        })
    };
    inspect().unwrap_or_else(|code| TimestampVerification {
        status: TimestampStatus::Invalid,
        detail: Some(code),
        ..Default::default()
    })
}

fn timestamp(
    message: &Message,
    store: HCERTSTORE,
    options: VerificationOptions,
) -> TimestampVerification {
    let inspect = || -> Result<TimestampVerification, u32> {
        let data = parameter(message, CMSG_SIGNER_INFO_PARAM)?;
        let signer = within(&data, data.as_ptr().cast::<CMSG_SIGNER_INFO>(), 1)?[0];
        if signer.UnauthAttrs.cAttr > 64 {
            return Err(INVALID);
        }
        let attributes = within(
            &data,
            signer.UnauthAttrs.rgAttr,
            signer.UnauthAttrs.cAttr as usize,
        )?;
        let mut token = None;
        let mut unsupported = false;
        for attribute in attributes {
            let name = oid(&data, attribute.pszObjId)?;
            if name == b"1.2.840.113549.1.9.6" {
                unsupported = true;
            }
            if !matches!(
                name,
                b"1.2.840.113549.1.9.16.2.14" | b"1.3.6.1.4.1.311.3.3.1"
            ) {
                continue;
            }
            if token.is_some() || attribute.cValue != 1 {
                return Err(INVALID);
            }
            let value = within(&data, attribute.rgValue, 1)?[0];
            token = Some(within(&data, value.pbData, value.cbData as usize)?);
        }
        let Some(token) = token else {
            return Ok(TimestampVerification {
                status: if unsupported {
                    TimestampStatus::Unsupported
                } else {
                    TimestampStatus::Absent
                },
                ..Default::default()
            });
        };
        let signature = within(
            &data,
            signer.EncryptedHash.pbData,
            signer.EncryptedHash.cbData as usize,
        )?;
        Ok(verify_timestamp(token, signature, store, options))
    };
    inspect().unwrap_or_else(|code| TimestampVerification {
        status: TimestampStatus::Invalid,
        detail: Some(code),
        ..Default::default()
    })
}

pub(super) fn assess(
    cert: *const CERT_CONTEXT,
    cms: &[u8],
    options: VerificationOptions,
) -> SignatureTrust {
    let mut result = SignatureTrust {
        network_allowed: options.allow_network,
        ..Default::default()
    };
    match query(cms) {
        Ok((store, message)) => {
            match strong_digest(&message)
                .and_then(|()| signing_key_usage(cert))
                .and_then(|()| chain(cert, store.0, options, None, false))
            {
                Ok((chain, revocation, detail)) => {
                    result.chain = chain;
                    result.revocation = revocation;
                    result.chain_detail = detail;
                }
                Err(code) => {
                    result.chain_detail = Some(code);
                }
            }
            result.timestamp = timestamp(&message, store.0, options);
        }
        Err(code) => {
            result.chain_detail = Some(code);
            result.timestamp = TimestampVerification {
                status: TimestampStatus::Unavailable,
                detail: Some(code),
                ..Default::default()
            };
        }
    }
    result
}

#[cfg(test)]
#[path = "certificate_trust_native_tests.rs"]
mod tests;
