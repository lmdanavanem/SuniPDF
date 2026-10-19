//! Detached CMS signatures over the saved PDF bytes. Identity trust is never inferred.

use crate::security::{self, lopdf_err};
use crate::EngineError;
use lopdf::{dictionary, Dictionary, Object, ObjectId, StringFormat};
use std::collections::HashSet;

#[path = "certificate_native.rs"]
mod native;
#[cfg(test)]
#[path = "../tests/support/certificate_identity.rs"]
mod test_identity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub serial_hex: String,
    pub sha256_hex: String,
    pub not_before: String,
    pub not_after: String,
    pub valid_now: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    Valid,
    WeakDigest,
    ModifiedAfterSigning,
    Invalid,
    Unsupported,
    Unverifiable,
}
impl SignatureStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::WeakDigest => "weak_digest",
            Self::ModifiedAfterSigning => "modified_after_signing",
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
            Self::Unverifiable => "unverifiable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitalSignature {
    pub field_name: String,
    pub sub_filter: String,
    pub status: SignatureStatus,
    pub certificate: Option<CertificateInfo>,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub signing_time: Option<String>,
    pub signed_len: usize,
    pub detail: Option<u32>,
    pub trust: SignatureTrust,
}

impl DigitalSignature {
    pub fn identity_trusted(&self) -> bool {
        self.status == SignatureStatus::Valid
            && self.certificate.is_some()
            && self.trust.chain_and_revocation_trusted()
    }

    pub fn failure_cause(&self) -> Option<&'static str> {
        match self.status {
            SignatureStatus::Valid => None,
            SignatureStatus::WeakDigest => Some("digest_not_allowed"),
            SignatureStatus::ModifiedAfterSigning => Some("modified_after_signing"),
            SignatureStatus::Unsupported => Some("unsupported_signature_format"),
            SignatureStatus::Invalid if self.detail == Some(ATTACHED_CONTENT) => {
                Some("attached_content_not_allowed")
            }
            SignatureStatus::Invalid => Some("signature_mismatch"),
            SignatureStatus::Unverifiable => Some("verification_unavailable"),
        }
    }
}

// Dedicated policy outcomes must not be confused with a provider's unsupported key algorithm.
const WEAK_DIGEST: u32 = 0xA0040001;
const ATTACHED_CONTENT: u32 = 0xA0040002;
const BAD_CMS: u32 = 0x8009200D;

/// Unauthenticated document/certificate labels are bounded single-line display data.
pub fn display_declared_text(value: &str) -> String {
    let mut output = String::new();
    let mut count = 0;
    for character in value.chars() {
        if matches!(character as u32, 0x061C | 0x200E..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
        {
            continue;
        }
        let character = if character.is_whitespace() {
            ' '
        } else if character.is_control() {
            continue;
        } else {
            character
        };
        if character == ' ' && (output.is_empty() || output.ends_with(' ')) {
            continue;
        }
        if count == 256 {
            output.pop();
            output.push('…');
            break;
        }
        output.push(character);
        count += 1;
    }
    output.trim_end().to_string()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VerificationOptions {
    pub allow_network: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChainTrust {
    Trusted,
    Untrusted,
    #[default]
    Unavailable,
}
impl ChainTrust {
    pub fn id(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RevocationStatus {
    Good,
    Revoked,
    Offline,
    Unknown,
    #[default]
    NotChecked,
}
impl RevocationStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Revoked => "revoked",
            Self::Offline => "offline",
            Self::Unknown => "unknown",
            Self::NotChecked => "not_checked",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimestampStatus {
    #[default]
    Absent,
    Trusted,
    Untrusted,
    Invalid,
    Unsupported,
    Unavailable,
}
impl TimestampStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimestampVerification {
    pub status: TimestampStatus,
    pub time: Option<String>,
    pub chain: ChainTrust,
    pub revocation: RevocationStatus,
    pub detail: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LtvStatus {
    #[default]
    Absent,
    EmbeddedUnverified,
    Malformed,
}
impl LtvStatus {
    pub fn id(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::EmbeddedUnverified => "embedded_unverified",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureTrust {
    pub chain: ChainTrust,
    pub revocation: RevocationStatus,
    pub chain_detail: Option<u32>,
    pub timestamp: TimestampVerification,
    pub ltv: LtvStatus,
    pub network_allowed: bool,
}
impl SignatureTrust {
    pub fn chain_and_revocation_trusted(&self) -> bool {
        self.chain == ChainTrust::Trusted && self.revocation == RevocationStatus::Good
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CertSignOptions {
    pub reason: Option<String>,
    pub location: Option<String>,
}

pub fn identity_info(pfx: &[u8], password: &str) -> Result<CertificateInfo, EngineError> {
    native::Identity::from_pfx(pfx, password)?.info()
}

#[cfg(test)]
#[path = "certificate_trust_tests.rs"]
mod trust_tests;

#[cfg(test)]
#[path = "certificate_security_tests.rs"]
mod security_tests;

fn text(dict: &Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key)
        .ok()
        .and_then(|o| lopdf::decode_text_string(o).ok())
}

fn resolve<'a>(pdf: &'a lopdf::Document, object: &'a Object) -> Result<&'a Object, EngineError> {
    pdf.dereference(object).map(|(_, o)| o).map_err(lopdf_err)
}

fn entry<'a>(
    pdf: &'a lopdf::Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Result<Option<&'a Object>, EngineError> {
    dict.get(key)
        .ok()
        .map(|o| resolve(pdf, o))
        .transpose()
        .map(|value| value.filter(|o| !matches!(o, Object::Null)))
}

/// Only signatures reachable from the active AcroForm field hierarchy are authoritative.
#[derive(Default)]
struct Fields {
    signatures: Vec<(String, Dictionary)>,
    names: HashSet<String>,
}
fn fields(pdf: &lopdf::Document) -> Result<Fields, EngineError> {
    fn visit(
        pdf: &lopdf::Document,
        object: &Object,
        prefix: &str,
        inherited_sig: bool,
        seen: &mut HashSet<ObjectId>,
        depth: usize,
        out: &mut Fields,
    ) -> Result<(), EngineError> {
        if depth > 64 {
            return Err(EngineError::InvalidFormat);
        }
        if let Object::Reference(id) = object {
            if !seen.insert(*id) {
                return Err(EngineError::InvalidFormat);
            }
        }
        let dict = resolve(pdf, object)?.as_dict().map_err(lopdf_err)?;
        let partial = text(dict, b"T").unwrap_or_default();
        let name = if prefix.is_empty() {
            partial
        } else if partial.is_empty() {
            prefix.into()
        } else {
            format!("{prefix}.{partial}")
        };
        out.names.insert(name.clone());
        let sig =
            entry(pdf, dict, b"FT")?.map_or(inherited_sig, |ft| ft.as_name().ok() == Some(b"Sig"));
        if sig {
            if let Some(value) = entry(pdf, dict, b"V")? {
                out.signatures
                    .push((name.clone(), value.as_dict().map_err(lopdf_err)?.clone()));
            }
        }
        if let Some(kids) = entry(pdf, dict, b"Kids")? {
            for kid in kids.as_array().map_err(lopdf_err)? {
                visit(pdf, kid, &name, sig, seen, depth + 1, out)?;
            }
        }
        Ok(())
    }
    let mut out = Fields::default();
    if let Some(acro) = entry(pdf, pdf.catalog().map_err(lopdf_err)?, b"AcroForm")? {
        let acro = acro.as_dict().map_err(lopdf_err)?;
        if let Some(array) = entry(pdf, acro, b"Fields")? {
            let mut seen = HashSet::new();
            for field in array.as_array().map_err(lopdf_err)? {
                visit(pdf, field, "", false, &mut seen, 0, &mut out)?;
            }
        }
    }
    Ok(out)
}

fn pdf_space(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

type SignedBytes<'a> = (&'a [u8], &'a [u8], Vec<u8>, usize);

fn detached_cms(cms: &[u8]) -> Result<(), u32> {
    fn take<'a>(input: &mut &'a [u8], tag: u8) -> Option<&'a [u8]> {
        if input.first() != Some(&tag) {
            return None;
        }
        let length = *input.get(1)?;
        let (head, length) = if length < 128 {
            (2, usize::from(length))
        } else {
            let width = usize::from(length & 127);
            if !(1..=4).contains(&width) || *input.get(2)? == 0 {
                return None;
            }
            let mut length = 0usize;
            for byte in input.get(2..2 + width)? {
                length = length.checked_mul(256)?.checked_add(usize::from(*byte))?;
            }
            if length < 128 {
                return None;
            }
            (2 + width, length)
        };
        let end = head.checked_add(length)?;
        let value = input.get(head..end)?;
        *input = input.get(end..)?;
        Some(value)
    }
    if cms.len() > 16 * 1024 * 1024 {
        return Err(BAD_CMS);
    }
    let mut cms = cms;
    let mut outer = take(&mut cms, 0x30).ok_or(BAD_CMS)?;
    if !cms.is_empty()
        || take(&mut outer, 6) != Some(b"\x2a\x86\x48\x86\xf7\x0d\x01\x07\x02".as_slice())
    {
        return Err(BAD_CMS);
    }
    let mut explicit = take(&mut outer, 0xa0).ok_or(BAD_CMS)?;
    if !outer.is_empty() {
        return Err(BAD_CMS);
    }
    let mut data = take(&mut explicit, 0x30).ok_or(BAD_CMS)?;
    if !explicit.is_empty() {
        return Err(BAD_CMS);
    }
    take(&mut data, 2).ok_or(BAD_CMS)?;
    take(&mut data, 0x31).ok_or(BAD_CMS)?;
    let mut content = take(&mut data, 0x30).ok_or(BAD_CMS)?;
    if take(&mut content, 6) != Some(b"\x2a\x86\x48\x86\xf7\x0d\x01\x07\x01".as_slice()) {
        return Err(BAD_CMS);
    }
    if !content.is_empty() {
        return Err(ATTACHED_CONTENT);
    }
    Ok(())
}

fn signed_data<'a>(input: &'a [u8], dict: &Dictionary) -> Option<SignedBytes<'a>> {
    let range = dict.get(b"ByteRange").ok()?.as_array().ok()?;
    if range.len() != 4 {
        return None;
    }
    let n = range
        .iter()
        .map(|o| usize::try_from(o.as_i64().ok()?).ok())
        .collect::<Option<Vec<_>>>()?;
    let [start, first, second, last]: [usize; 4] = n.try_into().ok()?;
    let end = second.checked_add(last)?;
    if start != 0 || first == 0 || second < first.checked_add(2)? || end > input.len() {
        return None;
    }
    let gap = input.get(first..second)?;
    if gap.first() != Some(&b'<') || gap.last() != Some(&b'>') {
        return None;
    }
    // The excluded string must be the actual Contents token, rather than another string in the PDF.
    let before = input.get(..first)?;
    let before = before.trim_ascii_end();
    if !before.ends_with(b"/Contents") {
        return None;
    }
    let mut hex = gap[1..gap.len() - 1]
        .iter()
        .copied()
        .filter(|b| !pdf_space(*b));
    let mut cms = Vec::new();
    while let Some(a) = hex.next() {
        let b = hex.next()?;
        cms.push((char::from(a).to_digit(16)? * 16 + char::from(b).to_digit(16)?) as u8);
    }
    if dict.get(b"Contents").ok()?.as_str().ok()? != cms {
        return None;
    }
    if cms.first() != Some(&0x30) {
        return None;
    }
    let length = *cms.get(1)?;
    let (head, body) = if length < 128 {
        (2, length as usize)
    } else {
        let bytes = usize::from(length & 127);
        if !(1..=4).contains(&bytes) {
            return None;
        }
        let mut size = 0usize;
        for b in cms.get(2..2 + bytes)? {
            size = size.checked_mul(256)?.checked_add(usize::from(*b))?;
        }
        (2 + bytes, size)
    };
    let der_end = head.checked_add(body)?;
    if cms.get(der_end..)?.iter().any(|b| *b != 0) {
        return None;
    }
    cms.truncate(der_end);
    Some((&input[..first], &input[second..end], cms, end))
}

pub fn verify_signatures(
    input: &[u8],
    password: Option<&str>,
) -> Result<Vec<DigitalSignature>, EngineError> {
    verify_signatures_with_options(input, password, VerificationOptions::default())
}

pub fn verify_signatures_with_options(
    input: &[u8],
    password: Option<&str>,
    options: VerificationOptions,
) -> Result<Vec<DigitalSignature>, EngineError> {
    let pdf = security::load(input, password)?;
    let ltv = ltv_status(&pdf);
    let mut out = Vec::new();
    for (field_name, dict) in fields(&pdf)?.signatures {
        let sub_filter = dict
            .get(b"SubFilter")
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_default();
        let mut result = DigitalSignature {
            field_name: display_declared_text(&field_name),
            sub_filter,
            status: SignatureStatus::Unverifiable,
            certificate: None,
            reason: text(&dict, b"Reason").map(|value| display_declared_text(&value)),
            location: text(&dict, b"Location").map(|value| display_declared_text(&value)),
            signing_time: text(&dict, b"M").map(|value| display_declared_text(&value)),
            signed_len: 0,
            detail: None,
            trust: SignatureTrust {
                ltv,
                network_allowed: options.allow_network,
                ..Default::default()
            },
        };
        if !matches!(
            result.sub_filter.as_str(),
            "adbe.pkcs7.detached" | "ETSI.CAdES.detached"
        ) {
            result.status = SignatureStatus::Unsupported;
        } else if let Some((a, b, cms, end)) = signed_data(input, &dict) {
            result.signed_len = end;
            match native::verify_trust(&cms, &[a, b], options) {
                Ok((info, mut trust)) => {
                    trust.ltv = ltv;
                    result.trust = trust;
                    result.certificate = Some(info);
                    result.status = if input[end..].iter().all(|b| pdf_space(*b)) {
                        SignatureStatus::Valid
                    } else {
                        SignatureStatus::ModifiedAfterSigning
                    };
                }
                Err(code) => {
                    result.detail = Some(code);
                    // CNG can propagate STATUS_INVALID_SIGNATURE through the CryptoAPI call.
                    result.status = if code == WEAK_DIGEST {
                        SignatureStatus::WeakDigest
                    } else if matches!(
                        code,
                        ATTACHED_CONTENT | 0x80091007 | 0x80090006 | 0xC000A000
                    ) {
                        SignatureStatus::Invalid
                    } else {
                        SignatureStatus::Unverifiable
                    };
                }
            }
        }
        out.push(result);
    }
    out.sort_by_key(|s| s.signed_len);
    Ok(out)
}

fn ltv_status(pdf: &lopdf::Document) -> LtvStatus {
    let inspect = || -> Result<LtvStatus, EngineError> {
        let Some(dss) = entry(pdf, pdf.catalog().map_err(lopdf_err)?, b"DSS")? else {
            return Ok(LtvStatus::Absent);
        };
        let dss = dss.as_dict().map_err(lopdf_err)?;
        let mut count = 0usize;
        let mut size = 0usize;
        for key in [b"Certs".as_slice(), b"OCSPs", b"CRLs"] {
            if let Some(material) = entry(pdf, dss, key)? {
                for value in material.as_array().map_err(lopdf_err)? {
                    count += 1;
                    let stream = resolve(pdf, value)?.as_stream().map_err(lopdf_err)?;
                    size = size
                        .checked_add(stream.content.len())
                        .ok_or(EngineError::InvalidFormat)?;
                    if count > 256 || size > 16 * 1024 * 1024 {
                        return Err(EngineError::InvalidFormat);
                    }
                }
            }
        }
        Ok(LtvStatus::EmbeddedUnverified)
    };
    inspect().unwrap_or(LtvStatus::Malformed)
}

/// Produces a new signed copy without reserializing any preceding revision.
pub fn sign_pdf(
    input: &[u8],
    pfx: &[u8],
    password: &str,
    options: &CertSignOptions,
) -> Result<Vec<u8>, EngineError> {
    let prev = lopdf::Document::load_mem(input).map_err(lopdf_err)?;
    if prev.is_encrypted() {
        return Err(EngineError::Other(
            "signing encrypted PDFs is not supported".into(),
        ));
    }
    if prev.xref_start == 0 {
        return Err(EngineError::Other("damaged cross-reference table".into()));
    }
    if options
        .reason
        .iter()
        .chain(options.location.iter())
        .any(|s| s.len() > 4096 || s.contains('\0'))
    {
        return Err(EngineError::Other(
            "invalid certificate signature metadata".into(),
        ));
    }
    let identity = native::Identity::from_pfx(pfx, password)?;
    let info = identity.info()?;
    if !info.valid_now {
        return Err(EngineError::Other(
            "certificate is outside its validity period".into(),
        ));
    }
    let existing = fields(&prev)?;
    let mut name = format!("SuniPDF-Signature-{}", existing.signatures.len() + 1);
    while existing.names.contains(&name) {
        name.push('_');
    }
    let mut sig = dictionary! {
        "Type" => "Sig", "Filter" => "Adobe.PPKLite", "SubFilter" => "adbe.pkcs7.detached",
        "ByteRange" => vec![Object::Integer(0),Object::Integer(9_999_999_999),Object::Integer(9_999_999_999),Object::Integer(9_999_999_999)],
        "Contents" => Object::String(vec![0;16_384],StringFormat::Hexadecimal),
        "Name" => lopdf::text_string(&info.subject),
        "M" => Object::string_literal(chrono::Utc::now().format("D:%Y%m%d%H%M%SZ").to_string()),
    };
    if let Some(reason) = &options.reason {
        sig.set("Reason", lopdf::text_string(reason));
    }
    if let Some(location) = &options.location {
        sig.set("Location", lopdf::text_string(location));
    }
    let catalog_id = prev
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(lopdf_err)?;
    let mut catalog = prev.catalog().map_err(lopdf_err)?.clone();
    let mut acro = if let Some(a) = entry(&prev, &catalog, b"AcroForm")? {
        a.as_dict().map_err(lopdf_err)?.clone()
    } else {
        Dictionary::new()
    };
    if entry(&prev, &acro, b"XFA")?.is_some() {
        return Err(EngineError::FormXfa);
    }
    let mut roots = if let Some(f) = entry(&prev, &acro, b"Fields")? {
        f.as_array().map_err(lopdf_err)?.clone()
    } else {
        Vec::new()
    };
    let mut inc = lopdf::IncrementalDocument::create_from(input.to_vec(), prev);
    let sig_id = inc.new_document.add_object(sig);
    let field_id = inc
        .new_document
        .add_object(dictionary! { "FT"=>"Sig", "T"=>lopdf::text_string(&name), "V"=>sig_id });
    roots.push(Object::Reference(field_id));
    acro.set("Fields", roots);
    acro.set("SigFlags", 3);
    let acro_id = inc.new_document.add_object(acro);
    catalog.set("AcroForm", acro_id);
    inc.new_document.set_object(catalog_id, catalog);
    let mut out = Vec::new();
    inc.save_to(&mut out)?;
    let serialized = lopdf::Document::load_mem(&out).map_err(lopdf_err)?;
    let from = match serialized.reference_table.get(sig_id.0) {
        Some(lopdf::xref::XrefEntry::Normal { offset, .. }) => *offset as usize,
        _ => return Err(EngineError::InvalidFormat),
    };
    if from < input.len() {
        return Err(EngineError::InvalidFormat);
    }
    let contents = from
        + out[from..]
            .windows(9)
            .position(|w| w == b"/Contents")
            .ok_or(EngineError::InvalidFormat)?
        + 9;
    let first = contents
        + out[contents..]
            .iter()
            .position(|b| *b == b'<')
            .ok_or(EngineError::InvalidFormat)?;
    let second = first + 32_770;
    if out.get(second - 1) != Some(&b'>') {
        return Err(EngineError::InvalidFormat);
    }
    let range = from
        + out[from..]
            .windows(10)
            .position(|w| w == b"/ByteRange")
            .ok_or(EngineError::InvalidFormat)?
        + 10;
    let range_start = range
        + out[range..]
            .iter()
            .position(|b| *b == b'[')
            .ok_or(EngineError::InvalidFormat)?;
    let range_end = range_start
        + out[range_start..]
            .iter()
            .position(|b| *b == b']')
            .ok_or(EngineError::InvalidFormat)?
        + 1;
    let replacement = format!("[0 {first} {second} {}]", out.len() - second);
    if replacement.len() > range_end - range_start {
        return Err(EngineError::Other("PDF is too large to sign".into()));
    }
    out[range_start..range_end].fill(b' ');
    out[range_start..range_start + replacement.len()].copy_from_slice(replacement.as_bytes());
    let cms = identity.sign(&[&out[..first], &out[second..]])?;
    if cms.len() > 16_384 {
        return Err(EngineError::Other(
            "certificate signature is too large".into(),
        ));
    }
    const HEX: &[u8] = b"0123456789ABCDEF";
    for (i, b) in cms.into_iter().enumerate() {
        out[first + 1 + i * 2] = HEX[usize::from(b >> 4)];
        out[first + 2 + i * 2] = HEX[usize::from(b & 15)];
    }
    let verified = verify_signatures(&out, None)?;
    if !verified
        .iter()
        .any(|s| s.field_name == name && s.status == SignatureStatus::Valid)
    {
        return Err(EngineError::Other(
            "signed copy failed its integrity check".into(),
        ));
    }
    Ok(out)
}

impl crate::Document<'_> {
    pub fn signature_count(&self) -> usize {
        usize::from(self.inner.signatures().len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_rejects_empty_and_malformed_cms() {
        for cms in [
            b"".as_slice(),
            b"invalid",
            &[0x30, 0x00],
            &[0x30, 0x82, 0x01],
        ] {
            assert!(native::verify(cms, &[b"content"]).is_err());
        }
    }

    #[test]
    fn native_ephemeral_pfx_signs_and_verifies() {
        let bytes = test_identity::identity();
        let identity = native::Identity::from_pfx(&bytes, "test-only").unwrap();
        let cms = identity.sign(&[b"first", b"second"]).unwrap();
        assert_eq!(
            native::verify(&cms, &[b"first", b"second"])
                .unwrap()
                .subject,
            identity.info().unwrap().subject
        );
        assert!(native::verify(&cms, &[b"First", b"second"]).is_err());
        assert!(native::Identity::from_pfx(&bytes, "wrong").is_err());
        assert!(native::Identity::from_pfx(b"invalid", "test-only").is_err());
        let empty = test_identity::identity_with_password("");
        assert!(native::Identity::from_pfx(&empty, "").is_ok());
    }
}
