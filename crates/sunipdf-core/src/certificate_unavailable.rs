//! Explicitly unavailable cryptographic provider; document signature parsing remains active.

use super::{CertificateInfo, SignatureTrust, VerificationOptions};
use crate::EngineError;

const VERIFICATION_UNAVAILABLE: u32 = 0xA0040003;

fn unavailable() -> EngineError {
    EngineError::Other(
        "certificate signing and identity inspection are unavailable on this platform".into(),
    )
}

pub(super) struct Identity;

impl Identity {
    pub(super) fn from_pfx(_pfx: &[u8], _password: &str) -> Result<Self, EngineError> {
        Err(unavailable())
    }

    pub(super) fn info(&self) -> Result<CertificateInfo, EngineError> {
        Err(unavailable())
    }

    pub(super) fn sign(&self, _parts: &[&[u8]]) -> Result<Vec<u8>, EngineError> {
        Err(unavailable())
    }
}

pub(super) fn verify_trust(
    cms: &[u8],
    _parts: &[&[u8]],
    _options: VerificationOptions,
) -> Result<(CertificateInfo, SignatureTrust), u32> {
    super::detached_cms(cms)?;
    Err(VERIFICATION_UNAVAILABLE)
}

#[cfg(test)]
pub(super) fn verify(cms: &[u8], parts: &[&[u8]]) -> Result<CertificateInfo, u32> {
    verify_trust(cms, parts, VerificationOptions::default()).map(|(info, _)| info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_provider_never_imports_or_signs() {
        assert!(Identity::from_pfx(b"identity", "private input").is_err());
        assert!(Identity.info().is_err());
        assert!(Identity.sign(&[b"PDF bytes"]).is_err());
    }

    #[test]
    fn detached_envelope_never_receives_a_cryptographic_verdict() {
        let envelope = b"\x30\x23\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x02\xa0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0b\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x01\x31\x00";
        for allow_network in [false, true] {
            assert_eq!(
                verify_trust(
                    envelope,
                    &[b"PDF bytes"],
                    VerificationOptions { allow_network }
                ),
                Err(VERIFICATION_UNAVAILABLE)
            );
        }
    }
}
