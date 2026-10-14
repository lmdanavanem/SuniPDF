//! Protection par mot de passe, permissions, métadonnées, signets, propriétés.
//!
//! PDFium ne sait pas chiffrer à l'écriture : les transformations d'octets (chiffrement,
//! déchiffrement, métadonnées) passent par `lopdf`, puis le résultat est relu par PDFium.

use crate::{Document, EngineError};
use lopdf::encryption::crypt_filters::{Aes128CryptFilter, Aes256CryptFilter, CryptFilter};
use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use lopdf::{Dictionary, Object};
use pdfium_render::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct Protection {
    /// Mot de passe d'ouverture (vide = ouverture libre, seules les permissions s'appliquent).
    pub user_password: String,
    /// Mot de passe propriétaire (vide = identique au mot de passe utilisateur).
    pub owner_password: String,
    pub allow_print: bool,
    pub allow_copy: bool,
    pub allow_modify: bool,
    pub allow_annotate: bool,
    /// AES-256 (PDF 2.0, R6) sinon AES-128 (R4).
    pub aes256: bool,
}

impl Protection {
    /// Au moins une permission retirée.
    pub fn restricts(&self) -> bool {
        !(self.allow_print && self.allow_copy && self.allow_modify && self.allow_annotate)
    }
}

impl Default for Protection {
    fn default() -> Self {
        Self {
            user_password: String::new(),
            owner_password: String::new(),
            allow_print: true,
            allow_copy: true,
            allow_modify: true,
            allow_annotate: true,
            aes256: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Properties {
    pub metadata: Metadata,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub version: String,
    pub page_count: usize,
    pub encrypted: bool,
    pub can_print: bool,
    pub can_copy: bool,
    pub can_modify: bool,
    pub can_annotate: bool,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub title: String,
    pub page: Option<usize>,
    pub depth: usize,
}

fn lopdf_err(e: lopdf::Error) -> EngineError {
    EngineError::Other(format!("lopdf: {e}"))
}

/// Charge un PDF avec lopdf et le déchiffre si nécessaire. Le chiffrement d'origine reste
/// mémorisé dans `doc.encryption_state` : [`save`] le réapplique, si bien qu'un aller-retour
/// lopdf (métadonnées, optimisation, fusion des flux) conserve la protection du document.
pub(crate) fn load(input: &[u8], password: Option<&str>) -> Result<lopdf::Document, EngineError> {
    let doc = lopdf::Document::load_mem(input).map_err(|_| EngineError::InvalidFormat)?;
    if !doc.is_encrypted() {
        return Ok(doc);
    }
    // Le lecteur déchiffre pendant l'analyse quand le mot de passe est fourni d'emblée.
    let options = lopdf::LoadOptions::with_password(password.unwrap_or(""));
    let doc = lopdf::Document::load_mem_with_options(input, options).map_err(|e| match e {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => {
            EngineError::PasswordRequired
        }
        _ => EngineError::InvalidFormat,
    })?;
    if doc.is_encrypted() {
        return Err(EngineError::PasswordRequired);
    }
    Ok(doc)
}

/// Enregistre ; un document déchiffré par [`load`] est rechiffré avec la même clé, les mêmes
/// mots de passe et les mêmes permissions (dictionnaire /Encrypt d'origine).
pub(crate) fn save(mut doc: lopdf::Document) -> Result<Vec<u8>, EngineError> {
    if let Some(state) = doc.encryption_state.take() {
        doc.encrypt(&state).map_err(lopdf_err)?;
    }
    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| EngineError::Other(format!("lopdf save: {e}")))?;
    Ok(out)
}

fn permissions(p: &Protection) -> Permissions {
    let mut perms = Permissions::empty();
    if p.allow_print {
        perms |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
    }
    if p.allow_copy {
        perms |= Permissions::COPYABLE | Permissions::COPYABLE_FOR_ACCESSIBILITY;
    }
    if p.allow_modify {
        perms |= Permissions::MODIFIABLE | Permissions::ASSEMBLABLE;
    }
    if p.allow_annotate {
        perms |= Permissions::ANNOTABLE | Permissions::FILLABLE;
    }
    perms
}

/// Chiffre un PDF (AES-128 ou AES-256) avec mots de passe et permissions.
pub fn protect(
    input: &[u8],
    current_password: Option<&str>,
    p: &Protection,
) -> Result<Vec<u8>, EngineError> {
    if p.user_password.is_empty() && p.owner_password.is_empty() {
        return Err(EngineError::Other("a password is required".into()));
    }
    if p.restricts() && (p.owner_password.is_empty() || p.owner_password == p.user_password) {
        // Sans mot de passe propriétaire distinct, quiconque ouvre le fichier a tous les droits :
        // des restrictions seraient trompeuses.
        return Err(EngineError::Other(
            "permissions require a distinct owner password".into(),
        ));
    }
    let mut doc = load(input, current_password)?;
    // Remplacer la protection existante, pas la superposer.
    doc.encryption_state = None;
    // Les algorithmes R4 dérivent la clé de l'identifiant de fichier : en créer un s'il manque.
    if doc.trailer.get(b"ID").is_err() {
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).map_err(|e| EngineError::Other(format!("random: {e}")))?;
        let s = |v: &[u8]| Object::String(v.to_vec(), lopdf::StringFormat::Hexadecimal);
        doc.trailer.set("ID", Object::Array(vec![s(&id), s(&id)]));
    }
    let owner = if p.owner_password.is_empty() {
        p.user_password.as_str()
    } else {
        p.owner_password.as_str()
    };
    let mut filters: BTreeMap<Vec<u8>, Arc<dyn CryptFilter>> = BTreeMap::new();
    let key: Vec<u8>;
    let version = if p.aes256 {
        filters.insert(b"StdCF".to_vec(), Arc::new(Aes256CryptFilter));
        let mut k = [0u8; 32];
        getrandom::fill(&mut k).map_err(|e| EngineError::Other(format!("random: {e}")))?;
        key = k.to_vec();
        EncryptionVersion::V5 {
            encrypt_metadata: true,
            crypt_filters: filters,
            file_encryption_key: &key,
            stream_filter: b"StdCF".to_vec(),
            string_filter: b"StdCF".to_vec(),
            owner_password: owner,
            user_password: &p.user_password,
            permissions: permissions(p),
        }
    } else {
        filters.insert(b"StdCF".to_vec(), Arc::new(Aes128CryptFilter));
        EncryptionVersion::V4 {
            document: &doc,
            encrypt_metadata: true,
            crypt_filters: filters,
            stream_filter: b"StdCF".to_vec(),
            string_filter: b"StdCF".to_vec(),
            owner_password: owner,
            user_password: &p.user_password,
            permissions: permissions(p),
        }
    };
    let state = EncryptionState::try_from(version).map_err(lopdf_err)?;
    doc.encrypt(&state).map_err(lopdf_err)?;
    save(doc)
}

/// Retire le chiffrement. Le mot de passe fourni doit être le mot de passe propriétaire (qui est
/// aussi le mot de passe d'ouverture quand aucun mot de passe propriétaire distinct n'existe) :
/// un document protégé par mot de passe propriétaire seul s'ouvre sans mot de passe, mais ses
/// restrictions ne se lèvent pas sans lui.
pub fn unprotect(input: &[u8], password: &str) -> Result<Vec<u8>, EngineError> {
    let mut doc = load(input, Some(password))?;
    let Some(state) = doc.encryption_state.take() else {
        return Err(EngineError::Other("document is not encrypted".into()));
    };
    if !owner_password_matches(&doc, &state, password)? {
        return Err(EngineError::PasswordRequired);
    }
    save(doc)
}

/// Vérifie un mot de passe propriétaire avec les algorithmes de lopdf, sur un document minimal
/// qui porte le dictionnaire /Encrypt d'origine (reconstruit depuis l'état décodé) et l'/ID.
fn owner_password_matches(
    doc: &lopdf::Document,
    state: &EncryptionState,
    password: &str,
) -> Result<bool, EngineError> {
    let mut probe = lopdf::Document::new();
    if let Ok(id) = doc.trailer.get(b"ID") {
        probe.trailer.set("ID", id.clone());
    }
    let dict = state
        .encode()
        .map_err(|e| EngineError::Other(format!("lopdf: {e}")))?;
    let id = probe.add_object(Object::Dictionary(dict));
    probe.trailer.set("Encrypt", Object::Reference(id));
    Ok(probe.authenticate_owner_password(password).is_ok())
}

fn info_dict(doc: &mut lopdf::Document) -> Result<&mut Dictionary, EngineError> {
    let id = match doc.trailer.get(b"Info").and_then(Object::as_reference) {
        Ok(id) => id,
        Err(_) => {
            let id = doc.add_object(Dictionary::new());
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };
    doc.get_object_mut(id)
        .and_then(Object::as_dict_mut)
        .map_err(lopdf_err)
}

fn set_or_remove(dict: &mut Dictionary, key: &str, value: &Option<String>) {
    match value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) if v.is_ascii() => dict.set(key, Object::string_literal(v)),
        // Hors ASCII : chaîne texte PDF en UTF-16BE avec BOM, lisible par tous les lecteurs.
        Some(v) => {
            let mut bytes = vec![0xFE, 0xFF];
            for unit in v.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            dict.set(key, Object::String(bytes, lopdf::StringFormat::Hexadecimal));
        }
        None => {
            dict.remove(key.as_bytes());
        }
    }
}

/// Remplace titre, auteur, sujet et mots-clés (`None` = supprimer la clé).
pub fn set_metadata(
    input: &[u8],
    password: Option<&str>,
    m: &Metadata,
) -> Result<Vec<u8>, EngineError> {
    let mut doc = load(input, password)?;
    {
        let info = info_dict(&mut doc)?;
        set_or_remove(info, "Title", &m.title);
        set_or_remove(info, "Author", &m.author);
        set_or_remove(info, "Subject", &m.subject);
        set_or_remove(info, "Keywords", &m.keywords);
    }
    save(doc)
}

/// Supprime toutes les métadonnées : dictionnaire /Info et flux XMP /Metadata du catalogue.
pub fn strip_metadata(input: &[u8], password: Option<&str>) -> Result<Vec<u8>, EngineError> {
    let mut doc = load(input, password)?;
    if let Ok(id) = doc.trailer.get(b"Info").and_then(Object::as_reference) {
        doc.delete_object(id);
    }
    doc.trailer.remove(b"Info");
    if let Ok(catalog) = doc.catalog_mut() {
        if let Ok(id) = catalog.get(b"Metadata").and_then(Object::as_reference) {
            catalog.remove(b"Metadata");
            doc.delete_object(id);
        } else {
            catalog.remove(b"Metadata");
        }
    }
    doc.prune_objects();
    save(doc)
}

impl Document<'_> {
    pub fn properties(&self) -> Properties {
        let meta = self.inner.metadata();
        let get = |t: PdfDocumentMetadataTagType| {
            meta.get(t)
                .map(|v| v.value().to_string())
                .filter(|s| !s.trim().is_empty())
        };
        // pdfium-render ne mappe pas les révisions 5/6 (AES-256) : les permissions sont lues
        // dans le dictionnaire /Encrypt (valeur /P) de l'état courant, via lopdf.
        let (encrypted, p_bits) = self
            .snapshot()
            .ok()
            .and_then(|bytes| lopdf::Document::load_mem(&bytes).ok())
            .map(|d| match d.trailer.get(b"Encrypt") {
                // Mot de passe d'ouverture vide : lopdf déchiffre au chargement et retire /Encrypt ;
                // l'état d'origine (permissions comprises) reste dans `encryption_state`.
                _ if d.encryption_state.is_some() => (
                    true,
                    d.encryption_state
                        .as_ref()
                        .map(|s| s.permissions().bits() as i64)
                        .unwrap_or(-1),
                ),
                Ok(obj) => {
                    let p = obj
                        .as_reference()
                        .ok()
                        .and_then(|id| d.get_object(id).ok())
                        .or(Some(obj))
                        .and_then(|o| o.as_dict().ok())
                        .and_then(|dict| dict.get(b"P").ok())
                        .and_then(|v| v.as_i64().ok())
                        .unwrap_or(-1);
                    (true, p)
                }
                Err(_) => (false, -1),
            })
            .unwrap_or((false, -1));
        let bit = |n: u32| p_bits & (1 << (n - 1)) != 0;
        Properties {
            metadata: Metadata {
                title: get(PdfDocumentMetadataTagType::Title),
                author: get(PdfDocumentMetadataTagType::Author),
                subject: get(PdfDocumentMetadataTagType::Subject),
                keywords: get(PdfDocumentMetadataTagType::Keywords),
            },
            creator: get(PdfDocumentMetadataTagType::Creator),
            producer: get(PdfDocumentMetadataTagType::Producer),
            created: get(PdfDocumentMetadataTagType::CreationDate),
            modified: get(PdfDocumentMetadataTagType::ModificationDate),
            version: format!("{:?}", self.inner.version())
                .replace("Pdf", "")
                .replace('_', "."),
            page_count: self.page_count(),
            encrypted,
            can_print: !encrypted || bit(3),
            can_copy: !encrypted || bit(5),
            can_modify: !encrypted || bit(4),
            can_annotate: !encrypted || bit(6),
            file_size: std::fs::metadata(&self.info.path).ok().map(|m| m.len()),
        }
    }

    /// Signets (table des matières), en profondeur préfixe.
    pub fn bookmarks(&self) -> Vec<Bookmark> {
        self.inner
            .bookmarks()
            .iter()
            .map(|b| {
                let mut depth = 0;
                let mut parent = b.parent();
                while let Some(p) = parent {
                    depth += 1;
                    parent = p.parent();
                }
                Bookmark {
                    title: b.title().unwrap_or_default(),
                    page: b
                        .destination()
                        .and_then(|d| d.page_index().ok())
                        .map(|i| i as usize),
                    depth,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_map_to_pdf_bits() {
        let all = permissions(&Protection::default());
        assert!(all.contains(Permissions::PRINTABLE) && all.contains(Permissions::COPYABLE));
        let none = permissions(&Protection {
            allow_print: false,
            allow_copy: false,
            allow_modify: false,
            allow_annotate: false,
            ..Default::default()
        });
        assert!(none.is_empty());
    }
}
