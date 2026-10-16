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

/// Raison du refus d'une [`Protection`] (l'interface traduit, la CLI reçoit l'`EngineError`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionProblem {
    NoPassword,
    /// Des restrictions sans mot de passe propriétaire distinct seraient trompeuses.
    OwnerNeeded,
    /// AES-128 : un mot de passe contient un caractère hors PDFDocEncoding.
    NotLatin1,
}

impl From<ProtectionProblem> for EngineError {
    fn from(p: ProtectionProblem) -> Self {
        match p {
            ProtectionProblem::NoPassword => EngineError::Other("a password is required".into()),
            ProtectionProblem::OwnerNeeded => {
                EngineError::Other("permissions require a distinct owner password".into())
            }
            ProtectionProblem::NotLatin1 => EngineError::PasswordNotLatin1,
        }
    }
}

/// Caractère d'un mot de passe AES-128 (R4) que SuniPDF sait rouvrir : Latin-1 imprimable, sans
/// les contrôles C1, l'espace insécable (0xA0 est l'euro en PDFDocEncoding) ni le trait d'union
/// conditionnel (0xAD, non défini). lopdf écrit le mot de passe en PDFDocEncoding et abandonne sans
/// erreur les caractères qu'il ne code pas ; PDFium, lui, reçoit le mot de passe en UTF-8 et ne
/// réessaie qu'en Latin-1 : les symboles propres à PDFDocEncoding hors Latin-1 (€, œ, tirets et
/// guillemets typographiques, ligatures…) donneraient un fichier conforme que SuniPDF ne rouvre pas.
fn pdf_doc_encodable(c: char) -> bool {
    matches!(c as u32, 0x20..=0x7E | 0xA1..=0xAC | 0xAE..=0xFF)
}

impl Protection {
    /// Au moins une permission retirée.
    pub fn restricts(&self) -> bool {
        !(self.allow_print && self.allow_copy && self.allow_modify && self.allow_annotate)
    }

    /// Vérifications partagées par l'interface (avant d'envoyer) et par [`protect`].
    pub fn validate(&self) -> Result<(), ProtectionProblem> {
        if !self.aes256
            && [&self.user_password, &self.owner_password]
                .iter()
                .any(|s| !s.chars().all(pdf_doc_encodable))
        {
            return Err(ProtectionProblem::NotLatin1);
        }
        let effective = |password: &str| {
            if self.aes256 {
                normalized_password(password).into_bytes()
            } else {
                password
                    .chars()
                    .take(32)
                    .map(|c| c as u8)
                    .collect::<Vec<_>>()
            }
        };
        let mut user = effective(&self.user_password);
        let mut owner = effective(&self.owner_password);
        if user.is_empty() && (!self.user_password.is_empty() || owner.is_empty()) {
            return Err(ProtectionProblem::NoPassword);
        }
        let owner_empty = owner.is_empty();
        if !self.aes256 {
            const PADDING: [u8; 32] = [
                0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA,
                0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE,
                0x64, 0x53, 0x69, 0x7A,
            ];
            user.extend_from_slice(&PADDING[..32 - user.len()]);
            owner.extend_from_slice(&PADDING[..32 - owner.len()]);
        }
        if self.restricts() && (owner_empty || owner == user) {
            return Err(ProtectionProblem::OwnerNeeded);
        }
        Ok(())
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
    pub destination: Option<crate::Destination>,
}

pub(crate) fn lopdf_err(e: lopdf::Error) -> EngineError {
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
    let aes256 = doc
        .get_encrypted()
        .and_then(|d| d.get(b"R"))
        .and_then(Object::as_i64)
        .is_ok_and(|r| r >= 5);
    // Le lecteur déchiffre pendant l'analyse quand le mot de passe est fourni d'emblée.
    let raw = password.unwrap_or("");
    let options = lopdf::LoadOptions::with_password(raw);
    let mut result = lopdf::Document::load_mem_with_options(input, options);
    if aes256
        && matches!(
            result,
            Err(lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_))
        )
    {
        let norm = normalized_password(raw);
        if norm != raw {
            result = lopdf::Document::load_mem_with_options(
                input,
                lopdf::LoadOptions::with_password(&norm),
            );
        }
    }
    let doc = result.map_err(|e| match e {
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
    // Full rewrites discard old storage/hint objects and revision offsets; size must match emitted objects.
    doc.objects.retain(|_, object| {
        !object
            .type_name()
            .is_ok_and(|name| matches!(name, b"ObjStm" | b"XRef" | b"Linearized"))
    });
    doc.max_id = doc.objects.keys().map(|id| id.0).max().unwrap_or(0);
    doc.trailer.remove(b"Prev");
    doc.trailer.remove(b"XRefStm");
    if let Some(state) = doc.encryption_state.take() {
        doc.encrypt(&state).map_err(lopdf_err)?;
    }
    if doc.is_encrypted() {
        let id = doc
            .trailer
            .get(b"Encrypt")
            .and_then(Object::as_reference)
            .map_err(lopdf_err)?;
        let encryption = doc
            .get_object_mut(id)
            .and_then(Object::as_dict_mut)
            .map_err(lopdf_err)?;
        if encryption
            .get(b"V")
            .and_then(Object::as_i64)
            .is_ok_and(|v| v == 5)
        {
            // lopdf omits the V5 key length; strict readers require the explicit 256-bit value.
            encryption.set("Length", 256);
        }
    }
    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| EngineError::Other(format!("lopdf save: {e}")))?;
    Ok(out)
}

fn permissions(p: &Protection) -> Permissions {
    // Bit 10 (extraction pour l'accessibilité) toujours accordé : les lecteurs d'écran ne sont
    // jamais bloqués (ISO 32000-2 l'impose pour R6).
    let mut perms = Permissions::COPYABLE_FOR_ACCESSIBILITY;
    if p.allow_print {
        perms |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
    }
    if p.allow_copy {
        perms |= Permissions::COPYABLE;
    }
    if p.allow_modify {
        perms |= Permissions::MODIFIABLE | Permissions::ASSEMBLABLE;
    }
    if p.allow_annotate {
        perms |= Permissions::ANNOTABLE | Permissions::FILLABLE;
    }
    perms
}

/// Forme normalisée (SASLprep, RFC 4013) d'un mot de passe AES-256 : c'est elle que lopdf chiffre
/// (révision 6) et donc celle que PDFium doit recevoir à l'ouverture. Espaces spéciaux et
/// ligatures sont ramenés à leur forme simple ; le préfixe UTF-8 valide est limité à 127 octets.
pub fn normalized_password(password: &str) -> String {
    let mut password = stringprep::saslprep(password)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| password.to_string());
    let mut end = password.len().min(127);
    while !password.is_char_boundary(end) {
        end -= 1;
    }
    password.truncate(end);
    password
}

/// Chiffre un PDF (AES-128 ou AES-256) avec mots de passe et permissions.
pub fn protect(
    input: &[u8],
    current_password: Option<&str>,
    p: &Protection,
) -> Result<Vec<u8>, EngineError> {
    p.validate()?;
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
    let user = if p.aes256 {
        normalized_password(&p.user_password)
    } else {
        p.user_password.clone()
    };
    let owner = if p.aes256 {
        normalized_password(&p.owner_password)
    } else {
        p.owner_password.clone()
    };
    let owner = if owner.is_empty() { &user } else { &owner };
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
            user_password: &user,
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
            user_password: &user,
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
    Ok(probe.authenticate_owner_password(password).is_ok()
        || (state.revision() >= 5
            && probe
                .authenticate_owner_password(&normalized_password(password))
                .is_ok()))
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

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Paquet XMP minimal cohérent avec /Info : les lecteurs qui préfèrent XMP (Acrobat, indexeurs)
/// ne montrent plus l'ancien titre ou auteur.
fn xmp_packet(m: &Metadata) -> String {
    let field = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(xml_escape)
    };
    let alt = |tag: &str, v: Option<String>| match v {
        Some(v) => format!(
            "   <{tag}><rdf:Alt><rdf:li xml:lang=\"x-default\">{v}</rdf:li></rdf:Alt></{tag}>\n"
        ),
        None => String::new(),
    };
    let seq = |tag: &str, v: Option<String>| match v {
        Some(v) => format!("   <{tag}><rdf:Seq><rdf:li>{v}</rdf:li></rdf:Seq></{tag}>\n"),
        None => String::new(),
    };
    let simple = |tag: &str, v: Option<String>| match v {
        Some(v) => format!("   <{tag}>{v}</{tag}>\n"),
        None => String::new(),
    };
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n \
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
         <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n{}{}{}{}  </rdf:Description>\n \
         </rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>",
        alt("dc:title", field(&m.title)),
        seq("dc:creator", field(&m.author)),
        alt("dc:description", field(&m.subject)),
        simple("pdf:Keywords", field(&m.keywords)),
    )
}

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const PDF_NS: &str = "http://ns.adobe.com/pdf/1.3/";
const XMP_LIMIT: usize = 16 * 1024 * 1024;

fn editable_xmp(name: &xml::name::OwnedName) -> bool {
    match name.namespace.as_deref() {
        Some(DC_NS) => matches!(
            name.local_name.as_str(),
            "title" | "creator" | "description"
        ),
        Some(PDF_NS) => name.local_name == "Keywords",
        _ => false,
    }
}

fn xmp_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::Other(format!("XMP metadata could not be updated: {error}"))
}

fn update_xmp(input: &[u8], m: &Metadata) -> Result<Vec<u8>, EngineError> {
    use xml::reader::{ParserConfig, XmlEvent};
    let config = ParserConfig::new()
        .ignore_comments(false)
        .ignore_root_level_whitespace(false)
        .allow_multiple_root_elements(false)
        .max_data_length(XMP_LIMIT)
        .max_attribute_length(XMP_LIMIT)
        .max_entity_expansion_length(1024 * 1024);
    let mut reader = config.create_reader(input);
    let mut writer = xml::writer::EmitterConfig::new()
        .write_document_declaration(false)
        .autopad_comments(false)
        .create_writer(Vec::new());
    let (mut depth, mut skipped) = (0, 0);
    let (mut rdf_depth, mut description_depth) = (None, None);
    let mut seen_rdf = false;
    loop {
        let mut event = reader.next().map_err(xmp_error)?;
        match &mut event {
            XmlEvent::Doctype { .. } => {
                return Err(xmp_error("DTD declarations are not supported"));
            }
            XmlEvent::StartDocument { .. } => continue,
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                depth += 1;
                if depth > 256 {
                    return Err(xmp_error("XML nesting limit exceeded"));
                }
                if skipped != 0 {
                    skipped += 1;
                    continue;
                }
                if name.namespace.as_deref() == Some(RDF_NS) && name.local_name == "RDF" {
                    if seen_rdf {
                        return Err(xmp_error("multiple RDF containers"));
                    }
                    seen_rdf = true;
                    rdf_depth = Some(depth);
                } else if rdf_depth == Some(depth - 1)
                    && name.namespace.as_deref() == Some(RDF_NS)
                    && name.local_name == "Description"
                    && !attributes.iter().any(|attribute| {
                        attribute.name.namespace.as_deref() == Some(RDF_NS)
                            && attribute.name.local_name == "about"
                            && !attribute.value.is_empty()
                    })
                {
                    description_depth = Some(depth);
                    attributes.retain(|attribute| !editable_xmp(&attribute.name));
                } else if description_depth == Some(depth - 1) && editable_xmp(name) {
                    skipped = 1;
                    continue;
                }
            }
            XmlEvent::EndElement { .. } => {
                if skipped != 0 {
                    skipped -= 1;
                    depth -= 1;
                    continue;
                }
                if rdf_depth == Some(depth) {
                    // A separate description keeps existing namespace bindings and custom properties intact.
                    let mut writing = false;
                    for field in xml::reader::EventReader::from_str(&xmp_packet(m)) {
                        let field = field.map_err(xmp_error)?;
                        if let XmlEvent::StartElement { name, .. } = &field {
                            writing |= name.local_name == "Description";
                        }
                        if writing {
                            if let Some(field) = field.as_writer_event() {
                                writer.write(field).map_err(xmp_error)?;
                            }
                        }
                        if let XmlEvent::EndElement { name } = &field {
                            if name.local_name == "Description" {
                                break;
                            }
                        }
                    }
                    rdf_depth = None;
                }
                if description_depth == Some(depth) {
                    description_depth = None;
                }
                depth -= 1;
            }
            XmlEvent::EndDocument => break,
            _ => {}
        }
        if skipped == 0 {
            if let Some(event) = event.as_writer_event() {
                writer.write(event).map_err(xmp_error)?;
            }
        }
    }
    if !seen_rdf {
        return Err(xmp_error("RDF metadata container is missing"));
    }
    Ok(writer.into_inner())
}

/// Remplace titre, auteur, sujet et mots-clés (`None` = supprimer la clé), en conservant
/// les autres propriétés et espaces de noms XMP du catalogue.
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
    let metadata = doc
        .catalog()
        .map_err(lopdf_err)?
        .get(b"Metadata")
        .ok()
        .cloned();
    if let Some(metadata) = metadata {
        let (id, object) = doc.dereference(&metadata).map_err(xmp_error)?;
        if !matches!(object, Object::Null) {
            let mut stream = object.as_stream().map_err(xmp_error)?.clone();
            let input = stream
                .get_plain_content_with_limit(XMP_LIMIT)
                .map_err(xmp_error)?;
            stream.set_plain_content(update_xmp(&input, m)?);
            if let Some(id) = id {
                doc.objects.insert(id, Object::Stream(stream));
            } else {
                let id = doc.add_object(stream);
                doc.catalog_mut()
                    .map_err(lopdf_err)?
                    .set("Metadata", Object::Reference(id));
            }
        }
    }
    save(doc)
}

/// Supprime toutes les métadonnées : dictionnaire /Info, flux XMP (/Metadata du catalogue, des
/// pages et des images) et données privées /PieceInfo.
pub fn strip_metadata(input: &[u8], password: Option<&str>) -> Result<Vec<u8>, EngineError> {
    let mut doc = load(input, password)?;
    strip_metadata_in(&mut doc);
    doc.prune_objects();
    save(doc)
}

/// Corps de [`strip_metadata`], partagé avec le caviardage (sans ramasse-miettes ni écriture).
pub(crate) fn strip_metadata_in(doc: &mut lopdf::Document) {
    if let Ok(id) = doc.trailer.get(b"Info").and_then(Object::as_reference) {
        doc.delete_object(id);
    }
    doc.trailer.remove(b"Info");
    for object in doc.objects.values_mut() {
        let dict = match object {
            Object::Dictionary(d) => d,
            Object::Stream(s) => &mut s.dict,
            _ => continue,
        };
        dict.remove(b"Metadata");
        dict.remove(b"PieceInfo");
    }
}

impl Document<'_> {
    /// Le document ouvert porte un gestionnaire de sécurité (mot de passe ou restrictions).
    pub fn is_encrypted(&self) -> bool {
        !matches!(
            self.inner.permissions().security_handler_revision(),
            Ok(PdfSecurityHandlerRevision::Unprotected)
        )
    }

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
                let resolved = b
                    .action()
                    .map(|action| self.resolve_local_action(&action))
                    .or_else(|| b.destination().map(|d| self.resolve_destination(&d)))
                    .unwrap_or((None, None));
                Bookmark {
                    title: b.title().unwrap_or_default(),
                    page: resolved.0,
                    depth,
                    destination: resolved.1,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_rewrite_resets_stale_xref_size_and_revision_offsets() {
        let mut pdf = load(include_bytes!("../tests/fixtures/two-pages.pdf"), None).unwrap();
        pdf.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
        pdf.objects.insert(
            (9999, 0),
            lopdf::dictionary! {"Linearized"=>1,"L"=>100,"H"=>vec![0.into(),0.into()]}.into(),
        );
        pdf.max_id = 9999;
        pdf.trailer.set("Prev", 123);
        pdf.trailer.set("XRefStm", 456);
        let bytes = save(pdf).unwrap();
        let reopened = load(&bytes, None).unwrap();
        assert!(!reopened.trailer.has(b"Prev"));
        assert!(!reopened.trailer.has(b"XRefStm"));
        assert_eq!(
            reopened.trailer.get(b"Size").unwrap().as_i64().unwrap(),
            i64::from(reopened.objects.keys().map(|id| id.0).max().unwrap() + 1)
        );
        assert!(!reopened
            .objects
            .values()
            .any(|object| object.type_name().is_ok_and(|name| name == b"Linearized")));
    }

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
        assert_eq!(
            none,
            Permissions::COPYABLE_FOR_ACCESSIBILITY,
            "l'accessibilité reste accordée"
        );
        assert!(!none.contains(Permissions::COPYABLE));
    }

    #[test]
    fn aes128_refuses_passwords_outside_latin1() {
        let p = |user: &str, aes256: bool| Protection {
            user_password: user.into(),
            aes256,
            ..Default::default()
        };
        assert_eq!(
            p(" ~¡¬®ÿ Été 5 ¿", false).validate(),
            Ok(()),
            "Latin-1 accepté"
        );
        // Symboles de PDFDocEncoding hors Latin-1 : lopdf les code, PDFium ne les rouvre pas.
        for pw in [
            "пароль",
            "Пароль2026",
            "日本",
            "a\u{00AD}b",
            "x\u{A0}",
            "prix5€",
            "œuvre",
            "a—b",
            "“x”",
            "ﬁn",
        ] {
            assert_eq!(
                p(pw, false).validate(),
                Err(ProtectionProblem::NotLatin1),
                "{pw:?}"
            );
            assert_eq!(p(pw, true).validate(), Ok(()), "AES-256 (UTF-8) : {pw:?}");
        }
        assert_eq!(p("", true).validate(), Err(ProtectionProblem::NoPassword));
        assert!(matches!(
            EngineError::from(ProtectionProblem::NotLatin1),
            EngineError::PasswordNotLatin1
        ));
        let xmp = xmp_packet(&Metadata {
            title: Some("A & B <c>".into()),
            author: Some("Jean".into()),
            ..Default::default()
        });
        assert!(xmp.contains("A &amp; B &lt;c&gt;") && xmp.contains("<rdf:li>Jean</rdf:li>"));
        assert!(!xmp.contains("dc:description"));
    }

    #[test]
    fn xmp_edit_preserves_pdfa_custom_properties_and_namespace_aliases() {
        let original = r#"<?xpacket begin="" id="keep-packet"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<r:Description r:about="" xmlns:a="http://purl.org/dc/elements/1.1/"
 xmlns:p="http://ns.adobe.com/pdf/1.3/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/"
 xmlns:c="urn:example:custom" a:title="old attribute title" p:Keywords="old keywords"
 pdfaid:part="2" c:flag="keep &amp; stay">
 <pdfaid:conformance>B</pdfaid:conformance><!--keep-comment-->
 <a:title><r:Alt><r:li xml:lang="fr">old translated title</r:li></r:Alt></a:title>
 <a:creator><r:Seq><r:li>old author</r:li></r:Seq></a:creator>
 <a:description>old subject</a:description>
 <c:record><a:title>custom nested title</a:title><c:value>été &amp; hiver</c:value></c:record>
</r:Description></r:RDF></x:xmpmeta><?xpacket end="w"?>"#;
        let edited = update_xmp(
            original.as_bytes(),
            &Metadata {
                title: Some("New <title> & été".into()),
                keywords: Some("new keywords".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let text = String::from_utf8(edited).unwrap();
        for kept in [
            "pdfaid:part=\"2\"",
            "<pdfaid:conformance>B</pdfaid:conformance>",
            "urn:example:custom",
            "c:flag=\"keep &amp; stay\"",
            "<!--keep-comment-->",
            "<a:title>custom nested title</a:title>",
            "été &amp; hiver",
            "keep-packet",
            "New &lt;title&gt; &amp; été",
            "new keywords",
        ] {
            assert!(text.contains(kept), "missing {kept}: {text}");
        }
        for removed in [
            "old attribute title",
            "old translated title",
            "old author",
            "old subject",
            "old keywords",
            "dc:creator",
            "dc:description",
        ] {
            assert!(!text.contains(removed), "retained {removed}: {text}");
        }
        for event in xml::reader::EventReader::from_str(&text) {
            event.unwrap();
        }
    }

    #[test]
    fn xmp_edit_handles_utf16_and_refuses_malformed_xml_and_dtd() {
        let original = xmp_packet(&Metadata {
            title: Some("Original".into()),
            ..Default::default()
        });
        let utf16: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain(original.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let edited = update_xmp(
            &utf16,
            &Metadata {
                title: Some("Updated".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(String::from_utf8(edited).unwrap().contains("Updated"));
        for invalid in [
            "<x:xmpmeta/>",
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><unclosed></x:xmpmeta>",
            "<RDF xmlns=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" a=\"1\" a=\"2\"/>",
            "<RDF xmlns=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/><other/>",
            "<xmpmeta xmlns=\"adobe:ns:meta/\" xmlns:r=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><r:RDF/><r:RDF/></xmpmeta>",
            "<!DOCTYPE RDF [<!ENTITY external SYSTEM 'file:///does-not-exist'>]><RDF xmlns=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>",
            "<!DOCTYPE RDF [<!ENTITY broken ]><RDF/>",
            "<RDF xmlns=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">&unknown;</RDF>",
            "<xmpmeta xmlns=\"adobe:ns:meta/\"/>",
        ] {
            let error = update_xmp(invalid.as_bytes(), &Metadata::default()).unwrap_err();
            assert!(error.to_string().contains("XMP metadata could not be updated"), "{error}");
        }
        let invalid_value = Metadata {
            title: Some("invalid\0value".into()),
            ..Default::default()
        };
        assert!(update_xmp(original.as_bytes(), &invalid_value).is_err());
    }

    #[test]
    fn xmp_edit_rejects_dtd_before_processing_entity_values() {
        let xml = "<!DOCTYPE RDF [<!ENTITY replacement 'content'>]><RDF xmlns='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>&replacement;</RDF>";
        let utf16: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain(xml.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        for bytes in [xml.as_bytes(), utf16.as_slice()] {
            let error = update_xmp(bytes, &Metadata::default()).unwrap_err();
            assert!(error
                .to_string()
                .contains("DTD declarations are not supported"));
        }
    }

    #[test]
    fn xmp_edit_changes_only_document_subjects_inside_the_rdf_container() {
        let original = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"
 xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:p="http://ns.adobe.com/pdf/1.3/"
 xmlns:c="urn:custom">
<r:RDF>
 <r:Description r:about="" dc:title="old document attribute"><dc:creator>old author</dc:creator></r:Description>
 <r:Description><dc:title>old implicit document title</dc:title></r:Description>
 <r:Description r:about="urn:other-resource" dc:title="other subject attribute" p:Keywords="other subject keywords">
  <dc:title>other subject title</dc:title><dc:creator>other subject author</dc:creator>
 </r:Description>
</r:RDF>
<c:extra><r:Description r:about="" dc:title="outside RDF attribute"><dc:title>outside RDF title</dc:title></r:Description></c:extra>
</x:xmpmeta>"#;
        let edited = update_xmp(
            original,
            &Metadata {
                title: Some("new document title".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let text = String::from_utf8(edited).unwrap();
        for kept in [
            "other subject attribute",
            "other subject keywords",
            "other subject title",
            "other subject author",
            "outside RDF attribute",
            "outside RDF title",
        ] {
            assert!(text.contains(kept), "missing {kept}: {text}");
        }
        for removed in [
            "old document attribute",
            "old author",
            "old implicit document title",
        ] {
            assert!(!text.contains(removed), "retained {removed}: {text}");
        }
        assert_eq!(text.matches("new document title").count(), 1, "{text}");
        for event in xml::reader::EventReader::from_str(&text) {
            event.unwrap();
        }
    }
}
