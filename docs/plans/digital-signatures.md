# Plan — tranche `digital-signatures` : signatures numériques par certificat

Rédigé le 2026-10-14 par l'agent `security-signatures`. **Plan uniquement : rien n'est implémenté, aucun
`cargo` n'a été exécuté** (contrainte de la tâche). Tout ce qui n'a pas pu être prouvé par lecture de source
est listé comme hypothèse `H1…H6` au §10.1, avec son repli.

Abréviations des sources citées (racine `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`) :

| Préfixe | Fichier |
|---|---|
| `W:` | `windows-0.62.2/src/Windows/Win32/Security/Cryptography/mod.rs` |
| `WF:` | `windows-0.62.2/src/Windows/Win32/Foundation/mod.rs` |
| `WS:` | `windows-0.62.2/src/Windows/Win32/System/` (sous-dossier précisé) |
| `L:` | `lopdf-0.45.0/src/` |
| `P:` | `pdfium-render-0.9.4/src/` |

Les sémantiques Win32 invisibles dans les liaisons Rust viennent des pages Microsoft Learn lues le 2026-10-14
(`learn.microsoft.com/windows/win32/api/wincrypt/…` et `…/ncrypt/…`) ; elles sont signalées « (MS Learn) ».
Les chemins sans préfixe sont relatifs à `C:/dev_suniworks/SuniPDF/`.

**Nommage, pour ne pas heurter la tranche `sign-image`** (`docs/plans/sign-image.md`), qui réserve
`Command::SignDocument`, `apps/sunipdf-desktop/src/sign.rs`, `SignState` et le préfixe i18n `signature.*` pour la
signature *manuscrite* et attribue `certificate.*` à celle-ci : tout ce qui suit s'appelle `certificate` /
`Certificate…`, jamais `sign` seul.

---

## 1. Objectif et comportement visible

Outil PDFCraft visé : **« Use a certificate »** (crate `sign` de PDFCraft : CMS, PKCS#12, horodatage, trousseau
Windows — `docs/PDFCRAFT_AUDIT.md` §2). Cette tranche livre le cœur (vérifier, et signer avec un `.pfx`), pas tout.

Ce que l'utilisateur voit :

1. **Vérifier les signatures numériques** (outil « Utiliser un certificat », menu Sécurité, palette Ctrl+K) : une
   boîte modale liste chaque signature du **fichier enregistré** avec
   - un statut typé : *intacte*, *intacte mais document modifié ensuite*, *invalide*, *format non pris en charge*,
     *illisible* ;
   - le signataire (sujet du certificat), l'émetteur, la validité du certificat, le numéro de série, l'empreinte
     SHA-256, la date déclarée, le motif, le lieu, le nom du champ ;
   - toujours la mention **« Identité du signataire non vérifiée »** et la note de périmètre (ni chaîne de
     confiance, ni révocation, ni horodatage contrôlés).
2. **Signer avec un certificat…** : une boîte modale demande le fichier `.pfx` / `.p12`, son mot de passe, un motif
   et un lieu facultatifs, et « Signature visible sur la page affichée » (position par ancre, comme le tampon).
   « Signer… » ouvre « Enregistrer la copie signée » ; le fichier signé est écrit atomiquement puis ouvert dans un
   nouvel onglet. Le document ouvert n'est jamais modifié (aucune entrée d'annulation).
3. Garde-fou : dès qu'un document signé est modifié dans SuniPDF, un bandeau prévient que l'enregistrer
   invalidera ses signatures (SuniPDF n'enregistre pas de façon incrémentale, §2.4). C'est l'avertissement que
   `sign-image.md` (« Questions ouvertes ») demande à cette tranche de porter.
4. CLI : `signatures`, `sign`, `cert-create` (JSON, anglais).

---

## 2. Faisabilité

### 2.1 Verdict : **partiel**

Faisable hors ligne, sans nouvelle bibliothèque cryptographique, avec des licences permissives :
vérification mathématique + intégrité + détails du certificat, et signature `adbe.pkcs7.detached`
(RSA / SHA-256) par mise à jour **incrémentale**. « Partiel » parce que la confiance (chaîne, révocation),
l'horodatage et le magasin de certificats Windows restent en roadmap (§2.3).

### 2.2 Choix de la bibliothèque (à consigner : ADR **D021**)

| Critère | RustCrypto pur Rust | API Windows CryptoAPI / CNG via le crate `windows` |
|---|---|---|
| Disponibilité | `cms` : dernière version stable **0.2.3 (2024-01-08)**, la suivante est `0.3.0-rc.0` (2026-10-13) ; `rsa` : stable 0.9.10, `0.10.0-rc.19` ; `pkcs12` 0.1.0 (format seul, `0.2.0-pre.0`) ; `p12-keystore` 0.4.1 (auteur tiers) ; `x509-cert` 0.3.0, `der` 0.8.2, `p256` 0.14.0 stables (API crates.io interrogée le 2026-10-14) | `windows` **0.62.2 déjà dans `Cargo.lock`** (via `accesskit_windows`) ; il suffit d'activer des *features* |
| Cohérence des versions | `cms` 0.2 tire l'ancienne génération (`der` 0.7, `const-oid` 0.9, `digest` / `sha2` 0.10) alors que `lopdf` 0.45 utilise déjà `sha2` 0.11, `digest` 0.11, `const-oid` 0.10 (`Cargo.lock`) : deux générations en double, ou des *release candidates* pour une fonction de sécurité | aucune |
| Avis de sécurité | `rsa` : RUSTSEC-2023-0071 (attaque Marvin, canal temporel), **aucune version corrigée** d'après rustsec.org et osv.dev consultés le 2026-10-14 ; statut de la 0.10 non vérifié | implémentation du système (modules validés FIPS 140 par Microsoft, non revérifié ici), corrigée par Windows Update |
| Nouveaux crates | ≈ 25 (cms, x509-cert, der, spki, pkcs8, rsa, num-bigint-dig, p256, ecdsa, elliptic-curve, signature, pkcs12 ou p12-keystore…) | aucun crate cryptographique |
| Licence | MIT OR Apache-2.0 | `windows` : MIT OR Apache-2.0 (`windows-0.62.2/Cargo.toml:28`) |
| Compilation MSVC | pur Rust, sans difficulté connue | liaisons `raw-dylib` (`windows_core::link!`, ex. `W:436`), aucune bibliothèque d'import |
| Magasin Windows, cartes à puce | impossible sans FFI | naturel (`CertOpenStore`, `W:888`) — roadmap |
| Coût | pas d'`unsafe` | ≈ 350 lignes d'`unsafe` FFI à isoler et tester |
| Portabilité | toutes plateformes | Windows seulement — c'est la cible unique du produit (`CLAUDE.md`) |

**Décision proposée : CryptoAPI / CNG.** Conforme à la règle de l'agent (« bibliothèque auditée, jamais
d'implémentation crypto maison »). À réévaluer quand `cms` 0.3 et `rsa` 0.10 seront stables et sans avis ouvert.
Aucun appel réseau : `CryptVerifyDetachedMessageSignature`, `CryptSignMessage` et `PFXImportCertStore` ne
construisent pas de chaîne et ne contactent ni OCSP ni CRL (aucune des pages MS Learn ne le mentionne, et aucun
`CertGetCertificateChain` n'est appelé dans cette tranche).

Variante plus légère si la compilation du crate `windows` gêne (poste à 14 Go ; clippy en repli GNU,
`docs/KNOWN_ISSUES.md` point 3) : `windows-sys` 0.61.2, déjà dans `Cargo.lock`, expose les mêmes fonctions en FFI
brut (`windows-sys-0.61.2/src/Windows/Win32/Security/Cryptography/mod.rs:309`, `:324`, `:413`).

### 2.3 Livré / laissé en roadmap

| Sujet | Cette tranche | Roadmap (raison) |
|---|---|---|
| Vérification `adbe.pkcs7.detached` et `ETSI.CAdES.detached` | oui : empreinte sur le `ByteRange` + signature CMS | |
| Détails du certificat signataire | oui si la signature est mathématiquement valide | détails aussi pour une signature invalide (API bas niveau `CryptMsgOpenToDecode`, `W:1626`) |
| « Document modifié après signature » | oui : octets au-delà du `ByteRange` signalés | analyse du *contenu* ajouté (DocMDP / FieldMDP, attaques « shadow », simple contre-signature ou vraie modification) |
| Chaîne de confiance | **non**, affiché « identité non vérifiée » | `CertGetCertificateChain` (`W:778`) avec `CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL` (`W:4682`) et `CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE` (`W:4711`) pour rester hors ligne ; décision par ADR, test positif à concevoir |
| Révocation OCSP / CRL | **non** | exige le réseau → ADR (fiche de l'agent) |
| Horodatage RFC 3161 (signature et vérification `ETSI.RFC3161`), LTV / DSS | **non** | exige un serveur d'horodatage en ligne |
| `adbe.pkcs7.sha1`, `adbe.x509.rsa_sha1` | statut « non pris en charge » | formats obsolètes |
| PAdES de base (attribut `signing-certificate-v2`), certification DocMDP | non | encodage d'attributs ESS à écrire et tester |
| Certificat du magasin Windows, carte à puce, jeton | non | suite naturelle : même type `Identity`, autre constructeur |
| ECDSA, RSA-PSS | vérification déléguée à CryptoAPI, **non testée** ; seule la signature RSA est testée | la structure `CRYPT_SIGN_MESSAGE_PARA` des liaisons (`W:9548`) n'a pas le champ `HashEncryptionAlgorithm` |
| Document chiffré | signature **refusée** (erreur typée) ; vérification possible | `/Contents` ne doit pas être chiffré, or lopdf chiffrerait l'objet ajouté (`L:writer.rs:394-398`) |
| Signature visible | rectangle ancré (7 positions), texte, page non pivotée | placement à la souris, image dans l'apparence ; pages pivotées dès que `PageGeometry` de `edit-advanced.md` existe |
| Signer un champ de signature vide existant | non (un nouveau champ est créé) | dépend de la tranche formulaires |
| Créer une identité auto-signée | moteur + CLI (`cert-create`), nécessaires aux tests | bouton dans l'interface |

### 2.4 Contraintes découvertes dans le code (elles dictent la conception)

1. **PDFium réécrit tout le fichier à chaque enregistrement.** `PdfDocument::save_to_writer` passe `flags = 0`
   (`P:pdf/document.rs:334`) : ni `FPDF_INCREMENTAL`, ni préservation des octets. Or `Document::snapshot` et
   `save_as` (`crates/sunipdf-core/src/pages.rs:246`, `:276`), la suspension d'onglet
   (`apps/sunipdf-desktop/src/worker.rs:731-766`) et l'annulation passent tous par `save_to_bytes`. Conséquences :
   (a) un PDF signé ne doit **jamais** repasser par PDFium avant d'être écrit → la signature est une tâche de
   fichier (`BatchOp`), pas un `EditOp` ; (b) la vérification porte sur **les octets du fichier sur disque**,
   jamais sur `snapshot()` ; (c) tout enregistrement SuniPDF d'un document signé casse ses signatures → bandeau
   et entrée dans `KNOWN_ISSUES`.
2. **pdfium-render n'expose ni `ByteRange` ni `SubFilter`.** `PdfSignature` n'offre que `bytes()`, `reason()`,
   `signing_date()`, `modification_detection_permission()` (`P:pdf/document/signature.rs:69`, `:107`, `:149`,
   `:194`). Les liaisons brutes existent (`P:bindings.rs:1376`, `:1397`) mais sont inatteignables : le trait
   d'accès est `pub(crate)` (`P:pdfium.rs:72-84`), `PdfDocument::handle()` aussi (`P:pdf/document.rs:206`).
   → l'énumération passe par **lopdf** ; de PDFium on ne garde que `signatures().len()`
   (`P:pdf/document/signatures.rs:31`, `P:pdf/document.rs:320`) comme compteur et contre-vérification.
3. **lopdf sait écrire une mise à jour incrémentale** : `IncrementalDocument` recopie les octets précédents tels
   quels (`L:writer.rs:356-358`) puis n'ajoute que les nouveaux objets, la table et le *trailer*
   (`L:writer.rs:387-440`). → la signature est un ajout en fin de fichier : les octets d'origine restent
   identiques, les signatures déjà présentes sont préservées, et lopdf ne re-sérialise jamais le contenu de
   l'utilisateur.

---

## 3. API moteur

### 3.0 Fichiers

| Fichier | Rôle | `unsafe` |
|---|---|---|
| `crates/sunipdf-signatures/Cargo.toml`, `src/lib.rs` (**nouveau crate**, prévu par `docs/ARCHITECTURE.md` ; création à valider par `architect`) | CMS détaché, PKCS#12, certificat auto-signé : octets en entrée, octets en sortie, aucune notion de PDF | oui : tout l'`unsafe` de la tranche est ici (`#![deny(unsafe_op_in_unsafe_fn)]`, un commentaire `// SAFETY:` par bloc) |
| `crates/sunipdf-core/src/certificate.rs` (**nouveau module**) | côté PDF : énumération, `ByteRange`, champ, apparence, mise à jour incrémentale | non |
| `crates/sunipdf-core/src/error.rs`, `lib.rs`, `stamp.rs` | variante d'erreur, ré-exports, compteur de signatures, ancre partagée | non |

`sunipdf-core` dépend de `sunipdf-signatures` et reste la seule façade vue par l'interface et la CLI.

### 3.1 Crate `sunipdf-signatures`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateInfo {
    pub subject: String,      // nom d'affichage simple (souvent le CN)
    pub issuer: String,
    pub serial_hex: String,
    pub not_before: String,   // "AAAA-MM-JJ" (UTC)
    pub not_after: String,
    pub sha256_hex: String,   // empreinte du certificat
    pub self_signed: bool,
    pub valid_now: bool,      // période de validité à l'instant de l'appel
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmsVerdict {
    /// Empreinte et signature correctes pour le certificat embarqué. Aucune validation de chaîne.
    Valid(CertificateInfo),
    /// L'empreinte ou la signature ne correspond pas.
    Invalid,
    /// Message illisible ou algorithme inconnu ; HRESULT Windows pour le diagnostic.
    Unverifiable(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignError {
    #[error("wrong PKCS#12 password")]
    BadPassword,
    #[error("not a readable PKCS#12 file")]
    NotPkcs12,
    #[error("no certificate with a private key")]
    NoPrivateKey,
    #[error("certificate is expired or not yet valid")]
    CertificateNotValid,
    #[error("signing an encrypted document is not supported")]
    EncryptedDocument,
    #[error("visible signature on a rotated page is not supported")]
    RotatedPage,
    #[error("signature larger than the reserved space")]
    TooLarge,
    #[error("Windows cryptography error 0x{0:08X}")]
    Os(u32),
}
impl SignError {
    /// Codes stables : cert_bad_password, cert_not_pkcs12, cert_no_private_key, cert_not_valid,
    /// cert_encrypted_document, cert_rotated_page, cert_too_large, cert_os.
    pub fn code(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTime { pub year: u16, pub month: u8, pub day: u8, pub hour: u8, pub minute: u8, pub second: u8 }
pub fn utc_now() -> UtcTime;

/// Identité de signature (certificat + clé privée). Non `Send` : créée, utilisée et détruite dans un seul appel.
pub struct Identity { /* HCERTSTORE + *const CERT_CONTEXT ; Drop libère les deux */ }
impl Identity {
    pub fn from_pfx(pfx: &[u8], password: &str) -> Result<Self, SignError>;
    pub fn info(&self) -> CertificateInfo;
    /// CMS SignedData détaché (DER), SHA-256, attribut signé `signingTime = at`. `max_len` : taille réservée.
    pub fn sign_detached(&self, parts: &[&[u8]], at: UtcTime, max_len: usize) -> Result<Vec<u8>, SignError>;
}

pub fn verify_detached(cms_der: &[u8], parts: &[&[u8]]) -> CmsVerdict;

/// PKCS#12 d'un certificat RSA 2048 auto-signé. Décalages en jours par rapport à maintenant
/// (ex. `(0, 365)` ; `(-730, -365)` produit un certificat expiré, utile aux tests).
pub fn create_self_signed_pfx(common_name: &str, password: &str,
                              start_offset_days: i32, end_offset_days: i32) -> Result<Vec<u8>, SignError>;
```

Appels sous-jacents (tous vérifiés présents dans `windows` 0.62.2) :

**`Identity::from_pfx`**
1. `CRYPT_INTEGER_BLOB { cbData, pbData }` sur les octets (`W:8986`) ; `PFXIsPFXBlob` (`W:2527`) faux → `NotPkcs12`.
2. Mot de passe copié en `Vec<u16>` terminé par 0, passé comme `PCWSTR(ptr)` (`windows-strings-0.5.1/src/pcwstr.rs:6` ;
   un `PCWSTR` est accepté tel quel comme paramètre, `windows-core-0.62.2/src/param.rs:67`), puis **effacé par
   `zeroize`** dans tous les cas (MS Learn demande `SecureZeroMemory`).
3. `PFXImportCertStore(&blob, pw, PKCS12_NO_PERSIST_KEY | PKCS12_ALWAYS_CNG_KSP)` (`W:2518` ; constantes `W:13592`,
   `W:13579` ; `BitOr` `W:9024`). **`PKCS12_NO_PERSIST_KEY` est obligatoire** : sans lui « keys are persisted on
   disk » (MS Learn), dans le profil de l'utilisateur. Avec lui la clé reste en mémoire et la propriété
   `CERT_KEY_CONTEXT_PROP_ID` est posée sur le certificat (MS Learn).
4. Échec : la liaison applique déjà `Error::from_thread()` (`W:2524`) ; `e.code()`
   (`windows-result-0.4.1/src/error.rs:123`) comparé à `ERROR_INVALID_PASSWORD.to_hresult()` (`WF:2939` ;
   `windows-0.62.2/src/extensions/Win32/Foundation/WIN32_ERROR.rs:13`) → `BadPassword` ; sinon `Os(code)`.
   Mot de passe vide : réessayer une fois avec `PCWSTR::null()` (les deux conventions existent).
5. `CertFindCertificateInStore(store, X509_ASN_ENCODING | PKCS_7_ASN_ENCODING, 0, CERT_FIND_HAS_PRIVATE_KEY, None, None)`
   (`W:692` ; `W:5254` ; `W:14465`, `W:13620` ; `BitOr` `W:6014`). « The key can be ephemeral or saved on disk »
   (MS Learn, Windows 8+). Nul → fermer le magasin, `NoPrivateKey`.
6. `Drop` : `CertFreeCertificateContext(Some(cert))` (`W:753`) puis `CertCloseStore(Some(store), 0)` (`W:514`).

**`Identity::info`, via une fonction commune `cert_info(*const CERT_CONTEXT)`** (structures `CERT_CONTEXT` `W:5016`, `CERT_INFO` `W:5362`)
- sujet : `CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, Some(&mut buf))` (`W:808`, `W:5670`) ;
  émetteur : idem avec `CERT_NAME_ISSUER_FLAG` (`W:5667`) ;
- série : `pCertInfo.SerialNumber` (petit-boutiste → inverser, hexadécimal) ;
- dates : `NotBefore` / `NotAfter` (`FILETIME`, `WF:4905`) → `FileTimeToSystemTime` (`WS:Time/mod.rs:7`, `SYSTEMTIME` `WF:9900`) ;
- empreinte : `CertGetCertificateContextProperty(cert, CERT_SHA256_HASH_PROP_ID, Some(buf32), &mut len)` (`W:783`, `W:6375`) ;
- auto-signé : `CertCompareCertificateName(X509_ASN_ENCODING, &Issuer, &Subject)` (`W:524`) ;
- validité : `CertVerifyTimeValidity(None, pCertInfo) == 0` (`W:1081`).

**`Identity::sign_detached`**
1. Certificats à embarquer : tous ceux du magasin PFX (signataire + intermédiaires, sinon un lecteur tiers ne peut
   pas reconstruire la chaîne) : boucle `CertEnumCertificatesInStore(store, prev)` (`W:644`) +
   `CertDuplicateCertificateContext` (`W:609`), libérés en fin de fonction.
2. Attribut signé `signingTime` : valeur DER `UTCTime` écrite directement (`0x17 0x0D` + `"AAMMJJHHMMSSZ"`,
   15 octets ; ce n'est pas de la cryptographie) dans un
   `CRYPT_ATTRIBUTE { pszObjId: szOID_RSA_signingTime, cValue: 1, rgValue }` (`W:8387`, `W:15024`).
   « If there are authenticated attributes present […] the content type […] and the hash of the message itself
   […] are automatically added by the system » (MS Learn, `CRYPT_SIGN_MESSAGE_PARA`).
   `// ponytail: UTCTime, valable jusqu'en 2049 ; GeneralizedTime ensuite.`
3. `CRYPT_SIGN_MESSAGE_PARA` (`W:9548`) : `cbSize = size_of`, `dwMsgEncodingType = X509 | PKCS7`, `pSigningCert`,
   `HashAlgorithm.pszObjId = szOID_NIST_sha256` (`W:14830`, `CRYPT_ALGORITHM_IDENTIFIER` `W:8363`),
   `cMsgCert` / `rgpMsgCert`, `cAuthAttr = 1`, `dwFlags = CRYPT_MESSAGE_SILENT_KEYSET_FLAG` (`W:9146`, aucune
   fenêtre du fournisseur de clés).
4. **Un seul appel** `CryptSignMessage(&para, true, n, Some(ptrs), lens, Some(buf), &mut size)` (`W:1842`) avec un
   tampon de `max_len` octets : `ERROR_MORE_DATA` (`WF:3323`) → `TooLarge`. `cToBeSigned > 1` est permis parce
   que la signature est détachée (MS Learn) : les deux tranches du `ByteRange` sont passées telles quelles, sans
   copie. `CRYPT_E_NO_KEY_PROPERTY` (`WF:626`) → `NoPrivateKey` ; autre → `Os(code)`. Les tailles sont des `u32` :
   refuser une tranche de plus de 4 Gio (`TooLarge`).

**`verify_detached`**
- `CRYPT_VERIFY_MESSAGE_PARA { cbSize, dwMsgAndCertEncodingType: X509 | PKCS7, hCryptProv: Default::default(),
  pfnGetSignerCertificate: None, pvGetArg: null }` (`W:9815`, `HCRYPTPROV_LEGACY` `W:10754`, type du rappel
  `W:13517`). Rappel nul = « the default callback tries to get the signer certificate context from the message's
  certificate store » (MS Learn).
- `CryptVerifyDetachedMessageSignature(&para, 0, cms, n, ptrs, lens, Some(&mut signer))` (`W:1928`).
  - `Ok` → `Valid(cert_info(signer))`, puis `CertFreeCertificateContext` (`W:753`).
  - `CRYPT_E_HASH_VALUE` (`WF:610`) ou `NTE_BAD_SIGNATURE` (`WF:6047`) → `Invalid`.
  - tout autre code (`CRYPT_E_UNEXPECTED_MSG_TYPE` `WF:648`, `CRYPT_E_NO_SIGNER` `WF:631`, `NTE_BAD_ALGID`
    `WF:6034`, erreurs ASN.1…) → `Unverifiable(code)`. **Jamais `Valid` par défaut.**
- Seul le signataire d'index 0 est contrôlé (une signature PDF n'en a qu'un).

**`create_self_signed_pfx`** (clé éphémère, rien n'est écrit dans le profil)
1. `NCryptOpenStorageProvider(&mut prov, MS_KEY_STORAGE_PROVIDER, 0)` (`W:2400`, `W:12317`).
2. `NCryptCreatePersistedKey(prov, &mut key, BCRYPT_RSA_ALGORITHM, PCWSTR::null(), CERT_KEY_SPEC(0), NCRYPT_FLAGS(0))`
   (`W:2264`, `W:3918`). Nom nul = « an ephemeral key that is not persisted » (MS Learn).
3. `NCryptSetProperty` : longueur 2048 (`NCRYPT_LENGTH_PROPERTY`, `W:12680`) et politique d'export
   `NCRYPT_ALLOW_EXPORT_FLAG | NCRYPT_ALLOW_PLAINTEXT_EXPORT_FLAG` (`W:2435`, `W:12477`, `W:12384`, `W:12388`),
   puis `NCryptFinalizeKey` (`W:2333`).
4. Sujet : `CertStrToNameW(X509_ASN_ENCODING, "CN=\"…\"", CERT_X500_NAME_STR, …)` en deux appels (taille puis
   données) (`W:1027`, `W:6862`) ; guillemets du nom doublés.
5. Dates : `GetSystemTimeAsFileTime()` (`WS:SystemInformation/mod.rs:146`) ± jours × 864 000 000 000 →
   `FileTimeToSystemTime`.
6. `CertCreateSelfSignCertificate(Some(HCRYPTPROV_OR_NCRYPT_KEY_HANDLE(key.0)), &name, CERT_CREATE_SELFSIGN_NO_KEY_INFO,
   None, Some(&alg), Some(&start), Some(&end), None)` (`W:574`, `W:5086`) avec l'algorithme `szOID_RSA_SHA256RSA`
   (`W:15000`), le défaut étant SHA1RSA (MS Learn).
7. Rattacher la clé : `CertSetCertificateContextProperty(cert, CERT_NCRYPT_KEY_HANDLE_PROP_ID,
   CERT_STORE_NO_CRYPT_RELEASE_FLAG.0, Some(&key))` (`W:1004`, `W:5692`, `W:6445`) pour garder la propriété de la
   poignée (sémantique du drapeau citée de mémoire : à relire sur MS Learn à l'implémentation).
8. Magasin mémoire : `CertOpenStore(CERT_STORE_PROV_MEMORY, CERT_QUERY_ENCODING_TYPE(0), None,
   CERT_STORE_CREATE_NEW_FLAG, None)` (`W:888`, `W:6538`, `W:6426`) + `CertAddCertificateLinkToStore(store, cert,
   CERT_STORE_ADD_ALWAYS, None)` (`W:440`, `W:6416`) — un *lien* partage les propriétés du contexte, dont la clé.
9. `PFXExportCertStoreEx(store, &mut blob, pw, &pbes2, EXPORT_PRIVATE_KEYS | REPORT_NO_PRIVATE_KEY |
   REPORT_NOT_ABLE_TO_EXPORT_PRIVATE_KEY | PKCS12_EXPORT_PBES2_PARAMS)` en deux appels (`pbData` nul → taille ;
   MS Learn) (`W:2510`, `W:10626`, `W:13829-13830`, `W:13586`) avec
   `PKCS12_PBES2_EXPORT_PARAMS { dwSize, hNcryptDescriptor: null, pwszPbes2Alg: PKCS12_PBES2_ALG_AES256_SHA256 }`
   (`W:13601`, `W:13598`) : AES-256 / SHA-256 au lieu du 3DES-SHA1 par défaut (Windows 10 1709+, MS Learn).
10. Nettoyage : `CertCloseStore`, `CertFreeCertificateContext`, `NCryptFreeObject(key)`, `NCryptFreeObject(prov)`
    (`W:2343`) ; mot de passe UTF-16 effacé.

**`utc_now`** : `GetSystemTime()` (`WS:SystemInformation/mod.rs:127`).

### 3.2 `sunipdf_core::certificate` — vérification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signature mathématiquement correcte et couvrant tout le fichier.
    Valid,
    /// Correcte pour la révision signée ; des octets ont été ajoutés ensuite.
    ModifiedAfterSigning,
    /// Empreinte ou signature fausse.
    Invalid,
    /// Sous-filtre non géré : rien n'a été vérifié.
    Unsupported,
    /// `ByteRange` ou CMS illisible : rien n'a pu être vérifié.
    Unverifiable,
}
impl SignatureStatus {
    /// valid, modified_after_signing, invalid, unsupported, unverifiable
    pub fn id(self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalSignature {
    pub field_name: Option<String>,
    pub sub_filter: String,
    pub status: SignatureStatus,
    pub signer_name: Option<String>,   // /Name déclaré, non authentifié
    pub reason: Option<String>,
    pub location: Option<String>,
    pub signing_time: Option<String>,  // /M brut ("D:…"), déclaré par le signataire
    pub certificate: Option<CertificateInfo>, // seulement si la signature est mathématiquement valide
    pub signed_len: usize,             // fin de la zone couverte (offset dans le fichier)
    pub detail: Option<u32>,           // HRESULT quand `Unverifiable`
}

/// Signatures du PDF, dans l'ordre de signature. `password` : seulement pour lire un document chiffré.
pub fn verify_signatures(input: &[u8], password: Option<&str>) -> Result<Vec<DigitalSignature>, EngineError>;

/// "D:20261014153000+02'00'" → "2026-10-14 15:30:00 +02:00" (chaîne rendue telle quelle si le format surprend).
pub fn pdf_date_display(raw: &str) -> String;
```

Algorithme :
1. `let doc = crate::security::load(input, password)?` (`crates/sunipdf-core/src/security.rs:78`, déjà
   `pub(crate)` ; renvoie `PasswordRequired` si besoin).
2. Parcourir `doc.objects` (`L:document.rs:34`). Candidat = dictionnaire (`Object::as_dict`, `L:object.rs:248`)
   possédant `ByteRange` **et** `Contents` (`Dictionary::has`, `L:object.rs:348`) ; traiter aussi le cas d'un `/V`
   qui serait un dictionnaire direct. Nom du champ : dictionnaire dont `/V` référence l'objet
   (`Object::as_reference`, `L:object.rs:218`), `/T` décodé par `lopdf::decode_text_string`
   (`L:common_data_structures/mod.rs:22`, exporté `L:lib.rs:43`).
3. `ByteRange` : `get_deref` + `as_array` + `as_i64` (`L:object.rs:360`, `:228`, `:155`). Validation stricte, sinon
   `Unverifiable` : 4 entiers ≥ 0, premier = 0, `l1 ≥ 1`, `b ≥ l1 + 2`, `b + l2 ≤ input.len()` (`checked_add`).
4. **Le CMS est lu dans le trou du fichier brut, pas dans l'objet lopdf** : `input[l1..b]` doit commencer par `<`,
   finir par `>` et ne contenir que des chiffres hexadécimaux (et blancs PDF). C'est ce que la signature désigne
   réellement ; cela ferme les attaques où `/Contents` et le trou divergent, et reste juste pour un document
   chiffré (lopdf « déchiffrerait » `/Contents` à tort).
5. Décodage hexadécimal puis rognage du remplissage : longueur DER lue sur l'en-tête (`0x30`, longueur courte ou
   `0x81..0x84`) ; longueur indéfinie (`0x80`) → passer le tampon entier.
6. `SubFilter` (`as_name`, `L:object.rs:188`) : `adbe.pkcs7.detached` ou `ETSI.CAdES.detached` →
   `verify_detached(&der, &[&input[..l1], &input[b..b + l2]])` ; autre → `Unsupported`.
7. Statut : `Valid(cert)` → `Valid` si `input[b + l2..]` ne contient que des blancs ASCII, sinon
   `ModifiedAfterSigning` ; `Invalid` → `Invalid` ; `Unverifiable(c)` → `Unverifiable` + `detail`.
8. `Name`, `Reason`, `Location` : `decode_text_string` ; `M` : `as_str` (`L:object.rs:198`) en UTF-8 tolérant.
9. Tri par `b + l2` croissant. Aucun `unwrap`, aucune indexation non bornée sur des données du fichier
   (entrées hostiles).

### 3.3 `sunipdf_core::certificate` — signature

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CertSignOptions {
    pub reason: Option<String>,
    pub location: Option<String>,
    /// `None` : signature invisible (widget de taille nulle sur la page 1).
    pub visible: Option<VisibleSignature>,
    /// Accepte un certificat hors période de validité (tests, CLI `--allow-expired`).
    pub allow_expired: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleSignature {
    pub page: usize,
    /// Espace d'affichage (origine en bas à gauche de la page telle qu'affichée), la convention de tout le moteur
    /// et la règle posée par `edit-advanced.md` §3.0.
    pub rect: PointRect,
    /// Lignes de l'apparence, fournies par l'appelant (le moteur n'embarque aucun texte) ;
    /// `{name}` = sujet du certificat, `{date}` = date de signature UTC. Même convention que `Stamp` (`{n}`, `{total}`).
    pub lines: Vec<String>,
}

/// Ajoute une signature `adbe.pkcs7.detached` par mise à jour incrémentale. Fonction pure sur des octets,
/// comme `security::protect` : l'appelant écrit le résultat avec `write_atomic`.
pub fn sign_pdf(input: &[u8], pfx: &[u8], pfx_password: &str, options: &CertSignOptions)
    -> Result<Vec<u8>, EngineError>;

/// Rectangle ancré sur une page (même calcul que `stamp_pages`).
pub fn anchored_rect(page: PageSize, position: StampPosition, width_pt: f32, height_pt: f32, margin_pt: f32) -> PointRect;

/// `Identity::from_pfx` + `info` (affichage CLI, contrôle avant signature).
pub fn identity_info(pfx: &[u8], password: &str) -> Result<CertificateInfo, EngineError>;

pub use sunipdf_signatures::{create_self_signed_pfx, CertificateInfo, SignError};
```

Constantes : `CONTENTS_BYTES = 16 384` (32 768 caractères hexadécimaux réservés ; un CMS RSA-4096 avec trois
certificats tient sous 8 Kio) ; rectangle visible par défaut 220 × 60 pt, marge 28 pt.

Étapes de `sign_pdf` :
1. `let prev = lopdf::Document::load_mem(input)` (`L:reader.rs:95`). `prev.is_encrypted()` (`L:document.rs:268`) →
   `SignError::EncryptedDocument`. `prev.xref_start == 0` (table reconstruite par balayage, `L:reader.rs:1438`) →
   `EngineError::Other("damaged cross-reference table")`, car `new_from_prev` omettrait `/Prev`
   (`L:document.rs:81-85`).
2. `let identity = Identity::from_pfx(pfx, pfx_password)?` ; `!info.valid_now && !allow_expired` →
   `CertificateNotValid`.
3. Page : `prev.get_pages().get(&(page as u32 + 1))` (`L:document.rs:533`, même usage que
   `crates/sunipdf-core/src/edit.rs:404-408`) sinon `PageOutOfRange`. Signature visible : remonter `/Parent`
   (32 niveaux au plus, `get_dictionary` `L:document.rs:202`) pour lire `/Rotate` hérité (≠ 0 →
   `SignError::RotatedPage`) et l'origine de `/CropBox` ou `/MediaBox`, ajoutée au rectangle.
4. `let mut inc = lopdf::IncrementalDocument::create_from(input.to_vec(), prev)`
   (`L:incremental_document.rs:30` ; champ public `new_document`, `:13` ; `max_id` repris du document précédent,
   `L:document.rs:92`).
5. Dictionnaire de signature : `inc.new_document.add_object(dictionary!{…})` (`L:creator.rs:21`, macro
   `L:object.rs:677`, `&str` → `Object::Name` `L:object.rs:106`) :
   `Type=Sig`, `Filter=Adobe.PPKLite`, `SubFilter=adbe.pkcs7.detached`,
   `ByteRange = [0, 9999999999, 9999999999, 9999999999]` (`Object::Integer`, écrits par `itoa`,
   `L:writer.rs:633-636`),
   `Contents = Object::String(vec![0; CONTENTS_BYTES], StringFormat::Hexadecimal)` (`L:object.rs:44`, `:53`),
   `M = "D:AAAAMMJJHHMMSSZ"` (`Object::string_literal`, `L:object.rs:137`),
   `Name`, `Reason`, `Location` par `lopdf::text_string` (`L:common_data_structures/mod.rs:9`).
6. Apparence (seulement si visible) : `Stream::new(dict, contenu)` (`L:object.rs:780`) avec `Type=XObject`,
   `Subtype=Form`, `BBox=[0 0 w h]`,
   `Resources/Font/Helv = {Type Font, Subtype Type1, BaseFont Helvetica, Encoding WinAnsiEncoding}`.
   Contenu construit par `lopdf::content::{Content, Operation}` (`L:content.rs:13`, `:28` — l'échappement des
   chaînes est fait par le sérialiseur de lopdf, rien n'est écrit à la main) : fond clair, cadre,
   `BT /Helv taille Tf … Tj T* … ET`. Texte : `{name}` et `{date}` remplacés, puis caractères hors Latin-1 remplacés
   par `?` (`// ponytail: WinAnsi seulement ; police incorporée quand un nom non latin sera signalé`).
7. Champ + widget fusionnés : `Type=Annot`, `Subtype=Widget`, `FT=Sig`, `T=Signature{n}` (premier `n` libre parmi
   les `/T` existants), `V=sig_id`, `P=page_id`, `F=132` (Print + Locked), `Rect=[0 0 0 0]` ou le rectangle,
   `AP={N: ap_id}` si visible.
8. Page : `inc.opt_clone_object_to_new_document(page_id)` (`L:incremental_document.rs:52`) puis `/Annots` : tableau
   direct → `as_array_mut().push` (`L:object.rs:238`) ; référence → cloner l'objet tableau puis `push` ; absent →
   `set` (`L:object.rs:370`).
9. Catalogue : identifiant lu dans `trailer["Root"]` (`L:document.rs:28`), cloné de la même façon ; `/AcroForm`
   absent → `{Fields: [widget], SigFlags: 3}` ; direct → modifier ; référence → cloner l'objet. `SigFlags = 3`.
   Si `prev.version < "1.6"` (`L:document.rs:21`) : `catalog.set("Version", "1.6")` (SHA-256).
10. Hygiène du *trailer* cloné (`new_from_prev` copie celui de la révision précédente, `L:document.rs:80`) :
    `inc.new_document.trailer.remove(b"DecodeParms")` et `remove(b"XRefStm")` (`L:object.rs:386`).
11. `inc.save_to(&mut out)` (`L:writer.rs:308`). Contrôle : `out.starts_with(input)`.
12. Repérage **dans `out[input.len()..]` uniquement** : la suite de `2 × CONTENTS_BYTES` zéros encadrée par `<`
    et `>` (lopdf écrit `<` + hexadécimal majuscule + `>`, `L:writer.rs:702-708`) et le motif
    `9999999999 9999999999 9999999999` entre `[` et `]` (`L:writer.rs:713-726`). Exactement une occurrence de
    chaque, sinon `EngineError::Other`.
13. `l1 = pos('<')`, `b = pos('>') + 1`, `l2 = out.len() - b`. Réécrire l'intérieur des crochets par
    `"0 {l1} {b} {l2}"` complété d'espaces à longueur identique (aucun décalage d'octet).
14. `let cms = identity.sign_detached(&[&out[..l1], &out[b..]], now, CONTENTS_BYTES)?` puis écrire son hexadécimal
    majuscule à partir de `l1 + 1` (le reste demeure à `0`).
15. **Auto-contrôle avant de rendre les octets** : `verify_signatures(&out, None)` ; la dernière entrée doit être
    `Valid`, sinon `EngineError::Other("signature self-check failed")`. Un fichier signé non vérifiable n'est
    jamais écrit.

### 3.4 Autres retouches dans `sunipdf-core`

- `error.rs` : `#[error("signature error: {0}")] Signature(#[from] sunipdf_signatures::SignError)` ;
  `code()` → `Self::Signature(e) => e.code()`.
- `lib.rs` : `pub mod certificate;`, ré-exports `CertSignOptions`, `DigitalSignature`, `SignatureStatus`,
  `VisibleSignature`, `CertificateInfo`, `SignError` ; `DocumentInfo` (`lib.rs:56`) gagne
  `pub signature_count: usize`, rempli dans `Engine::wrap` (`lib.rs:159-196`) par
  `inner.signatures().len() as usize` (`P:pdf/document.rs:320`, `P:pdf/document/signatures.rs:31`).
- `stamp.rs` : extraire le `match` des lignes 126-134 en
  `pub(crate) fn origin(self, page: PageSize, w: f32, h: f32, margin: f32) -> (f32, f32)` sur `StampPosition`,
  appelé par `stamp_pages` et par `certificate::anchored_rect` (aucune duplication).
- `Cargo.toml` du workspace : membre `crates/sunipdf-signatures`, dépendances de workspace (§9).

### 3.5 Instantanés, `regenerate_content`, annulation

- `verify_signatures` et `sign_pdf` sont des fonctions pures sur des octets : aucun objet PDFium n'est modifié,
  donc **pas de `regenerate_content`, pas d'instantané d'annulation, pas d'invalidation du cache de rendu**.
- `Document::snapshot()` n'est que *lu*, comme base de signature quand le document a des modifications non
  enregistrées (§4).
- Interdit : `Document::restore(octets_signés)` suivi de `save_as`. Les octets signés vont directement de
  `sign_pdf` à `write_atomic` (`crates/sunipdf-core/src/pages.rs:63`).

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

```rust
pub enum BatchOp {
    // …
    CertificateSign { out: PathBuf, from_disk: bool, pfx: PathBuf, password: String, options: CertSignOptions },
}
pub enum BatchResult {
    // …
    CertificateSigned { out: PathBuf },
}
pub enum Request  { /* … */ VerifySignatures }
pub enum Response { /* … */ SignaturesVerified { path: PathBuf, result: Result<Vec<DigitalSignature>, EngineError> } }
```

- `run_batch` (`worker.rs:559`), bras `BatchOp::CertificateSign`, de la même forme que `Optimize`
  (`worker.rs:603-614`) : base = `std::fs::read(&doc.info().path)?` si `from_disk`, sinon `doc.snapshot()?` ;
  `std::fs::read(&pfx)?` ; `sunipdf_core::certificate::sign_pdf(&base, &pfx_bytes, &password, &options)` ;
  `password.zeroize()` **avant** de propager l'erreur ; `sunipdf_core::write_atomic(&out, &signed)?` ;
  `Ok(BatchResult::CertificateSigned { out })`.
  - `from_disk = !dirty` (décidé par l'interface) : un document non modifié est signé à partir de ses octets
    exacts (les signatures existantes sont conservées) ; un document modifié est signé à partir de son état
    courant.
- Boucle `run`, bras `Request::VerifySignatures`, à côté de `Request::Properties` (`worker.rs:787`) :
  `std::fs::read(&path)` puis `sunipdf_core::certificate::verify_signatures(&bytes, doc.password())` ; le chemin
  est renvoyé pour que l'interface ignore une réponse arrivée après un changement d'onglet.
- Pas de nouvel `EditOp`, pas de `BytesOp` : §2.4 point 1.
- Le mot de passe du PFX n'est jamais journalisé (aucun `log::` ne formate un `BatchOp`).

---

## 5. Interface

### 5.1 Commandes (`crates/sunipdf-core/src/command.rs`)

| Variante | id | clé de libellé | raccourci | `needs_document` | Menu |
|---|---|---|---|---|---|
| `CertificateSign` | `security.sign_certificate` | `menu.security.sign_certificate` | aucun | `true` | `Security` |
| `VerifySignatures` | `security.verify_signatures` | `menu.security.verify_signatures` | aucun | `true` | `Security` |

À insérer dans `REGISTRY` après `StripMetadata` (l'ordre du tableau est l'ordre du menu). Sans document, le
mécanisme `after_open` existant ouvre d'abord un fichier (`app.rs:795-800`).

### 5.2 Rangement dans les outils — deux cas selon l'ordre d'intégration

**Cas A — `tool-hub` est déjà intégré (recommandé).** Suivre son contrat (`docs/plans/tool-hub.md` §5.9), en
quatre gestes : (1) les deux commandes ci-dessus ; (2) dans `TABLE`, ligne `Workspace::Certificate` :
`entry: Some(Entry::Mode(C::HubCertificate))`, `commands: &[C::CertificateSign, C::VerifySignatures]`,
`icon: (glyph::CERTIFICATE, "✒")` ; (3) `HubCertificate` au registre (`hub.certificate`, libellé
`hub.certificate.name`, menu `Tools`), bras dans `execute`, entrée dans `command_icon` ; le test
`Command::in_menu(Menu::Tools).count()` de ce plan passe de 6 à 7 ; (4) rien d'autre : `hub.certificate.name` et
`hub.certificate.desc` existent déjà dans son §7. Son test `every_registry_command_has_a_home` impose que les
deux commandes figurent dans `commands`.

**Cas B — code actuel (`ToolTab`, `shell.rs:13-146`).** `ToolTab::Security` (`shell.rs:103`) devient
`[Protect, Unprotect, CertificateSign, VerifySignatures, StripMetadata, Properties]` ; `ToolTab::All` reçoit
`CertificateSign` et `VerifySignatures` après `Protect`. `command_color` : rien à faire
(`Menu::Security => CAT_SECURITY`, `shell.rs:180`).

Dans les deux cas : `command_icon` (`shell.rs:246`, `match` exhaustif) reçoit
`CertificateSign => (glyph::CERTIFICATE, "✒")` et `VerifySignatures => (glyph::VERIFIED, "✔")`.

### 5.3 `apps/sunipdf-desktop/src/icons.rs`

`pub const CERTIFICATE: &str = "\u{EB95}";` et `pub const VERIFIED: &str = "\u{E930}";` — présence des deux
points de code vérifiée le 2026-10-14 dans `C:\Windows\Fonts\SegoeIcons.ttf` **et** `segmdl2.ttf`
(`GlyphTypeface.CharacterToGlyphMap`) ; leur dessin exact reste à confirmer sur capture.

### 5.4 Nouveau module `apps/sunipdf-desktop/src/certificate.rs` (+ `mod certificate;` dans `main.rs`)

```rust
#[derive(Default)]
pub struct CertificateState {
    pub verify_open: bool,
    /// `None` : vérification en cours ; `Err` : clé i18n de l'erreur.
    pub report: Option<Result<Vec<DigitalSignature>, &'static str>>,
    pub sign_open: bool,
    pub form: CertificateForm,
    pub sign_error: Option<&'static str>,
}
#[derive(Default)]
pub struct CertificateForm {
    pub pfx_path: String,
    pub password: String,
    pub reason: String,
    pub location: String,
    pub visible: bool,
    pub position: StampPosition, // mise à BottomRight à la première ouverture
}
```

`SuniApp` gagne `pub certificate: crate::certificate::CertificateState` (niveau application, comme `tools` : les
deux boîtes sont modales, on ne change pas d'onglet pendant qu'elles sont ouvertes ; **pas** dans `TabState`).

**`certificate_verify_dialog(&mut self, ctx)`** —
`dialog::Spec::new("certificate-verify", tr("certificate.verify.title"), …).subtitle(nom du fichier)
.icon(glyph::CERTIFICATE, "✔").width(600.0)`, corps dans un `ScrollArea` (hauteur max 420) :
- `report == None` : `ui.spinner()` + `certificate.verify.checking` ;
- `Err(clé)` : libellé rouge ;
- liste vide : `certificate.verify.none` ;
- sinon une carte par signature (`Frame` bordé, comme le bloc d'informations de `docinfo.rs:146-213`) :
  pastille de couleur + texte du statut (`theme::SUCCESS` pour `Valid` ; `theme::CAT_PAGES` (ambre) pour
  `ModifiedAfterSigning`, `Unsupported`, `Unverifiable` ; `theme::ERROR` pour `Invalid` — la couleur n'est jamais
  le seul porteur d'information, le texte du statut est toujours affiché) ; pour `Valid` et
  `ModifiedAfterSigning`, ligne ambre `certificate.identity_unverified` ; puis une `Grid` à deux colonnes :
  signataire (`certificate.subject`, à défaut `signer_name`, à défaut `certificate.unknown_signer`), émetteur,
  date déclarée (`pdf_date_display`), motif, lieu, validité, numéro de série, empreinte, champ ; mentions
  `certificate.info.self_signed` et `certificate.info.expired` le cas échéant ;
- sous la liste, texte atténué `certificate.verify.scope`, et `certificate.verify.dirty_note` si `self.dirty`.
Pied : `dialog::primary(tr("about.close"))` ou Échap → fermeture, `report = None`.

**`certificate_sign_dialog(&mut self, ctx)`** —
`Spec::new("certificate-sign", tr("certificate.sign.title"), …).subtitle(tr("certificate.sign.subtitle"))
.icon(glyph::CERTIFICATE, "✒").width(540.0)` :
- `field_label("certificate.sign.pfx")` ; ligne : `TextEdit::singleline(&mut form.pfx_path)` (reçoit le focus à
  l'ouverture, même motif que `tools.rs:226-228` — permet de coller un chemin et de piloter la capture d'écran) +
  bouton `certificate.sign.browse` →
  `rfd::FileDialog::new().set_title(t("certificate.sign.pick")).add_filter(t("certificate.sign.filter"), &["pfx", "p12"]).pick_file()` ;
- `field_label("certificate.sign.password")` ; `TextEdit::singleline(..).password(true)` ;
- motif, lieu ;
- `ui.checkbox(&mut form.visible, tr("certificate.sign.visible"))` ; si coché : `tr("stamp.position")` +
  `position_combo` (rendre `pub(crate)` la fonction de `tools.rs:61` ; clés `position.*` existantes) ;
- notes atténuées : `certificate.sign.note` ; `certificate.sign.dirty_note` si `self.dirty` ;
  `certificate.warn_save` si `self.dirty` et `doc.info.signature_count > 0` ;
- `sign_error` en rouge.
Pied : `dialog::primary(tr("certificate.sign.ok"))` (ou Ctrl+Entrée) ;
`dialog::secondary(tr("dialog.password.cancel"))` ou Échap.

Soumission :
1. chemin vide ou fichier absent → `sign_error = Some("certificate.sign.error_no_pfx")` ;
2. `save_dialog("certificate.sign.save", &self.suggest_output("-signe"))` (rendre `pub(crate)` `save_dialog`,
   `suggest_output` et `run_batch` de `tools.rs:90-123`) ; annulé → la boîte reste ouverte ;
3. chemin choisi = chemin du document ouvert → `sign_error = Some("certificate.sign.error_same_file")` ;
4. options : `visible` → `VisibleSignature { page: doc.page,
   rect: sunipdf_core::certificate::anchored_rect(doc.info.page_sizes[doc.page], form.position, 220.0, 60.0, 28.0), lines }`
   où `lines = [tr("certificate.appearance.signed_by"), tr("certificate.appearance.date")]` (les marqueurs `{name}`
   et `{date}` sont laissés au moteur) et, si un motif est saisi,
   `tr_args("certificate.appearance.reason", &[("reason", …)])` ;
5. `self.run_batch(BatchOp::CertificateSign { out, from_disk: !self.dirty, pfx,
   password: std::mem::take(&mut form.password), options }, "tools.running.certificate_sign")` ; `sign_open = false`.

`tools.rs::on_batch_done` (`tools.rs:470`) :
- `BatchResult::CertificateSigned { out }` → `status_msg = tr_args("tools.done.certificate_sign", name)` puis
  `self.open_path(out, None)` (même enchaînement que `Combined`, `tools.rs:474-481`) ;
- `BatchResult::Failed(e)` quand `e` est `EngineError::Signature(_)` → rouvrir la boîte avec
  `certificate.sign_error = Some(error_key(&e))` (le formulaire est conservé, sauf le mot de passe) ; autres
  erreurs : comportement actuel (`tools.rs:528`).

### 5.5 `apps/sunipdf-desktop/src/app.rs`

- `execute` (`app.rs:793`) :
  `Command::CertificateSign => { self.certificate.sign_error = None; self.certificate.form.password.zeroize(); self.certificate.sign_open = true; }` ;
  `Command::VerifySignatures => { self.certificate.report = None; self.certificate.verify_open = true; self.worker.send(Request::VerifySignatures); }`.
- `handle_responses` : `Response::SignaturesVerified { path, result }` → si `verify_open` et `path` = chemin du
  document actif, `self.certificate.report = Some(result.map_err(|e| error_key(&e)))`.
- `handle_input` : ajouter `|| self.certificate.sign_open || self.certificate.verify_open` à la garde des boîtes
  modales (`app.rs:591-595`).
- `ui()` : appeler `self.certificate_sign_dialog(&ctx); self.certificate_verify_dialog(&ctx);` avec les autres
  boîtes (`app.rs:1875-1884`).
- `close_document` (`app.rs:239`) : `self.certificate.verify_open = false; self.certificate.report = None;`.
- `error_key` : `EngineError::Signature(e) => match e { BadPassword => "error.certificate.bad_password",
  NotPkcs12 => "error.certificate.not_pkcs12", NoPrivateKey => "error.certificate.no_private_key",
  CertificateNotValid => "error.certificate.not_valid", EncryptedDocument => "error.certificate.encrypted",
  RotatedPage => "error.certificate.rotated_page", TooLarge | Os(_) => "error.certificate.failed" }`.
- **Garde-fous pour les documents signés** (trois retouches courtes) :
  1. `Command::Save` (`app.rs:839`) : ne rien faire si `!self.dirty`. Aujourd'hui Ctrl+S sur un PDF signé intact
     le réécrit via PDFium et **détruit sa signature sans rien dire** (`save_document` → `Request::Save` →
     `Document::save_as`).
  2. `Response::DocumentChanged` (`app.rs:422`) : si `doc.info.signature_count > 0`,
     `self.error = Some("certificate.warn_save")` (bandeau existant, refermable).
  3. `save_document(true)` (Enregistrer sous) d'un document signé : même bandeau avant l'envoi.

### 5.6 Onglets et annulation

- Signer ne touche pas le document actif : ni `dirty`, ni annulation, ni invalidation du rendu. La copie signée
  arrive dans un nouvel onglet par `open_path` ; l'onglet d'origine est suspendu comme pour « Combiner ».
- Un document signé suspendu puis repris est rechargé depuis un instantané PDFium (`worker.rs:755`) : son image
  en mémoire n'est plus identique au fichier. Sans conséquence tant que rien n'est enregistré (garde-fou 1), et la
  vérification relit toujours le disque.
- Vérification : lecture seule.

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

`cert-create` ne charge pas PDFium : à traiter avant `Engine::new()`, comme `Commands` (`main.rs:268-278`).

```
sunipdf-cli signatures <file> [--password <pw>]
sunipdf-cli sign <file> --pfx <id.pfx> (--pfx-password <pw> | --pfx-password-env <VAR>) --out <signed.pdf>
                 [--reason <txt>] [--location <txt>]
                 [--visible [--page <n, 1-based, default 1>] [--position <StampPosition id, default bottom-right>]]
                 [--allow-expired]
sunipdf-cli cert-create --name <common name> (--pfx-password <pw> | --pfx-password-env <VAR>) --out <id.pfx>
                 [--days <n, default 365>]
```

`--pfx-password-env` évite d'exposer le mot de passe dans la liste des processus : c'est la forme à documenter en
premier. Aide de `sign` : « Add a certificate-based digital signature (PKCS#12). Signer identity is not validated
against any trust list. »

Sorties :

```json
{ "file": "signed.pdf", "count": 1, "trust": "not_checked",
  "signatures": [ {
    "field": "Signature1", "status": "valid", "sub_filter": "adbe.pkcs7.detached",
    "covers_whole_file": true, "signed_len": 48211,
    "signer_name": "Test SuniPDF", "reason": null, "location": null, "signing_time": "D:20261014153000Z",
    "certificate": { "subject": "Test SuniPDF", "issuer": "Test SuniPDF", "serial": "4A…",
                     "not_before": "2026-10-14", "not_after": "2027-10-14", "sha256": "9F…",
                     "self_signed": true, "valid_now": true },
    "detail": null } ] }
```
```json
{ "out": "signed.pdf", "signer": "Test SuniPDF", "sub_filter": "adbe.pkcs7.detached", "digest": "SHA-256",
  "visible": false, "size": 48211 }
```
```json
{ "out": "id.pfx", "subject": "Test SuniPDF", "not_after": "2027-10-14", "sha256": "9F…" }
```

`"trust": "not_checked"` est une constante assumée : elle empêche un script de confondre « intacte » et « de
confiance » (même idée que le champ `digital_signature: false` de `sign-image.md` §6). `status` prend les valeurs
de `SignatureStatus::id()`. Code de sortie 0 dès que la commande a tourné (le statut est dans le JSON) ; erreurs :
`{"error": "<code>", "message": …}` avec les codes `cert_*` (sortie 1, branche `_ => 1` existante).
Avec `--visible`, la CLI ouvre le document (`engine.open`) pour `page_size` puis appelle
`certificate::anchored_rect` ; lignes d'apparence en anglais : `"Digitally signed by {name}"`, `"Date: {date}"`.

---

## 7. i18n — nouvelles clés (`assets/i18n/fr-FR.json` **et** `en-US.json`)

Préfixe `certificate.*` (réservé à cette tranche par `sign-image.md` §7) et `error.certificate.*`.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.security.sign_certificate` | Signer avec un certificat… | Sign with a certificate… |
| `menu.security.verify_signatures` | Vérifier les signatures numériques | Verify digital signatures |
| `certificate.sign.title` | Signer avec un certificat | Sign with a certificate |
| `certificate.sign.subtitle` | Signature numérique à partir d'un fichier PKCS#12 (.pfx, .p12). | Digital signature from a PKCS#12 file (.pfx, .p12). |
| `certificate.sign.pfx` | Fichier de certificat (.pfx / .p12) | Certificate file (.pfx / .p12) |
| `certificate.sign.browse` | Parcourir… | Browse… |
| `certificate.sign.pick` | Choisir le fichier de certificat | Choose the certificate file |
| `certificate.sign.filter` | Certificats PKCS#12 | PKCS#12 certificates |
| `certificate.sign.password` | Mot de passe du certificat | Certificate password |
| `certificate.sign.reason` | Motif (facultatif) | Reason (optional) |
| `certificate.sign.location` | Lieu (facultatif) | Location (optional) |
| `certificate.sign.visible` | Signature visible sur la page affichée | Visible signature on the displayed page |
| `certificate.sign.note` | Une copie signée est enregistrée dans un nouveau fichier ; le document ouvert n'est pas modifié. Signature RSA / SHA-256 sans horodatage certifié : la date est celle de cet ordinateur. | A signed copy is saved to a new file; the open document is left untouched. RSA / SHA-256 signature without a trusted timestamp: the date comes from this computer's clock. |
| `certificate.sign.dirty_note` | Les modifications non enregistrées seront incluses dans la copie signée. | Unsaved changes will be included in the signed copy. |
| `certificate.sign.ok` | Signer… | Sign… |
| `certificate.sign.save` | Enregistrer la copie signée | Save the signed copy |
| `certificate.sign.error_no_pfx` | Choisissez un fichier de certificat existant. | Choose an existing certificate file. |
| `certificate.sign.error_same_file` | Choisissez un autre nom : la copie signée ne peut pas remplacer le document ouvert. | Choose another name: the signed copy cannot replace the open document. |
| `certificate.appearance.signed_by` | Signé numériquement par {name} | Digitally signed by {name} |
| `certificate.appearance.date` | Date : {date} | Date: {date} |
| `certificate.appearance.reason` | Motif : {reason} | Reason: {reason} |
| `tools.running.certificate_sign` | Signature en cours… | Signing… |
| `tools.done.certificate_sign` | Copie signée enregistrée : {name} | Signed copy saved: {name} |
| `certificate.verify.title` | Signatures numériques | Digital signatures |
| `certificate.verify.checking` | Vérification en cours… | Verifying… |
| `certificate.verify.none` | Ce document ne contient aucune signature numérique. | This document contains no digital signature. |
| `certificate.verify.scope` | Vérification locale du fichier enregistré : intégrité du document et signature cryptographique. La chaîne de confiance, la révocation du certificat et l'horodatage ne sont pas contrôlés. | Local check of the saved file: document integrity and cryptographic signature. The trust chain, certificate revocation and timestamps are not checked. |
| `certificate.verify.dirty_note` | Le document a des modifications non enregistrées : elles ne sont pas prises en compte ici. | The document has unsaved changes: they are not taken into account here. |
| `certificate.status.valid` | Signature intacte : le document n'a pas été modifié depuis sa signature. | Signature intact: the document has not been changed since it was signed. |
| `certificate.status.modified_after` | Signature intacte pour la version signée, mais le document a été modifié ou complété ensuite. Les ajouts ne sont pas couverts par cette signature. | Signature intact for the signed version, but the document was changed or extended afterwards. The additions are not covered by this signature. |
| `certificate.status.invalid` | Signature invalide : le document a été altéré ou la signature ne lui correspond pas. | Invalid signature: the document was altered or the signature does not match it. |
| `certificate.status.unsupported` | Format de signature non pris en charge ({format}) : rien n'a été vérifié. | Unsupported signature format ({format}): nothing was verified. |
| `certificate.status.unverifiable` | Signature illisible : rien n'a pu être vérifié. | Unreadable signature: nothing could be verified. |
| `certificate.identity_unverified` | Identité du signataire non vérifiée. | Signer identity not verified. |
| `certificate.unknown_signer` | Signataire inconnu | Unknown signer |
| `certificate.field.signer` | Signataire | Signer |
| `certificate.field.issuer` | Émetteur | Issuer |
| `certificate.field.date` | Date déclarée | Claimed date |
| `certificate.field.reason` | Motif | Reason |
| `certificate.field.location` | Lieu | Location |
| `certificate.field.validity` | Validité du certificat | Certificate validity |
| `certificate.field.validity_value` | du {from} au {to} | {from} to {to} |
| `certificate.field.serial` | Numéro de série | Serial number |
| `certificate.field.thumbprint` | Empreinte SHA-256 | SHA-256 thumbprint |
| `certificate.field.name` | Champ | Field |
| `certificate.info.self_signed` | Certificat auto-signé | Self-signed certificate |
| `certificate.info.expired` | Certificat expiré ou pas encore valide à ce jour | Certificate expired or not yet valid today |
| `certificate.warn_save` | Ce document est signé : l'enregistrer avec SuniPDF invalidera ses signatures. | This document is signed: saving it with SuniPDF will invalidate its signatures. |
| `error.certificate.bad_password` | Mot de passe du certificat incorrect. | Incorrect certificate password. |
| `error.certificate.not_pkcs12` | Ce fichier n'est pas un certificat PKCS#12 (.pfx, .p12) lisible. | This file is not a readable PKCS#12 certificate (.pfx, .p12). |
| `error.certificate.no_private_key` | Ce fichier ne contient aucun certificat avec clé privée. | This file contains no certificate with a private key. |
| `error.certificate.not_valid` | Le certificat est expiré ou pas encore valide. | The certificate is expired or not yet valid. |
| `error.certificate.encrypted` | Signer un document protégé par mot de passe n'est pas encore pris en charge. Retirez la protection, enregistrez, puis signez. | Signing a password-protected document is not supported yet. Remove the protection, save, then sign. |
| `error.certificate.rotated_page` | La signature visible n'est pas encore prise en charge sur une page pivotée. Choisissez une signature invisible. | A visible signature is not supported yet on a rotated page. Choose an invisible signature. |
| `error.certificate.failed` | La signature a échoué (erreur du système de chiffrement Windows). | Signing failed (Windows cryptography error). |

55 clés. Réutilisées sans changement : `about.close`, `dialog.password.cancel`, `stamp.position`, `position.*`
(et `hub.certificate.*` si `tool-hub` est intégré). Le test `placeholders_match_between_languages` de
`sunipdf-i18n` impose les mêmes `{…}` dans les deux langues : c'est le cas. Propriété des catalogues : agent
`ui-ux` (D013) ; ces clés sont une proposition.

---

## 8. Tests

### 8.1 Unitaires du crate (`cargo test -p sunipdf-signatures`, sans PDFium)

1. `self_signed_identity_round_trip` : `create_self_signed_pfx("SuniPDF Test", "pw", 0, 365)` →
   `Identity::from_pfx` ; `info()` : sujet exact, `self_signed`, `valid_now`, empreinte de 64 caractères. Mauvais
   mot de passe → `Err(BadPassword)` ; octets quelconques → `Err(NotPkcs12)`.
2. `detached_sign_verify_and_tamper` : signer `[b"hello ", b"world"]` ; `verify_detached` → `Valid(cert)` avec la
   même empreinte ; même contenu découpé autrement (`[b"hel", b"lo world"]`) → `Valid` (l'empreinte porte sur la
   concaténation) ; un octet du contenu inversé → `Invalid` ; un octet inversé au milieu du CMS → **différent de
   `Valid`** ; CMS tronqué → `Unverifiable(_)` ; `max_len = 16` → `Err(TooLarge)`.
3. `expired_identity_is_reported` : `(-730, -365)` → `valid_now == false`.

### 8.2 Intégration (`crates/sunipdf-core/tests/engine.rs`, assistants `engine()` et `fixture()` existants)

**A. `certificate_sign_verify_tamper_and_modify_after`** (test obligatoire de la tranche)
- identité jetable générée dans le test :
  `sunipdf_core::certificate::create_self_signed_pfx("SuniPDF Test Signer", "secret", 0, 365)` ;
- `verify_signatures(&original, None)` → liste vide (document non signé) ;
- `signed = sign_pdf(&original, &pfx, "secret", &CertSignOptions { reason: Some("Approbation".into()),
  location: Some("Genève".into()), ..Default::default() })` ;
- `signed.starts_with(&original)` : la signature est un ajout, pas une réécriture ;
- relecture : une entrée, `status == Valid`, `sub_filter == "adbe.pkcs7.detached"`,
  `field_name == Some("Signature1")`, `certificate.subject == "SuniPDF Test Signer"`, `self_signed`, motif et lieu
  relus à l'identique (accent compris), `signing_time` présent ;
- **contre-vérification par PDFium** : `engine.open_bytes(signed.clone(), None, &chemin)` → 2 pages,
  `page_text(0)` contient « SuniPDF page 1 », `info().signature_count == 1` ;
- **pixels** : `render_page(0, 1.0).rgba` du document signé == celui de l'original (une signature invisible ne
  change pas le rendu) ;
- **un octet inversé dans la zone signée** : remplacer le `1` de `(SuniPDF page 1)` par `7` (octet de texte d'un
  flux non compressé, le PDF reste lisible) → `status == Invalid` ;
- **un octet inversé dans le CMS** : changer un chiffre hexadécimal 200 caractères après `/Contents<` →
  statut ≠ `Valid` ;
- **document modifié après signature** : ajouter une vraie révision incrémentale avec lopdf
  (`IncrementalDocument::load_from(&signed[..])` `L:reader.rs:379`, `new_document.add_object(dictionary!{…})`,
  `save_to`) → une entrée, `status == ModifiedAfterSigning`, `signed_len < modifié.len()`, et PDFium ouvre
  toujours le fichier ;
- tolérance : `signed + b"\r\n"` → toujours `Valid` ;
- **deuxième signature** avec une autre identité sur `signed` → deux entrées : la première
  `ModifiedAfterSigning` (mathématiquement intacte : la signature précédente est préservée), la seconde `Valid`,
  champs `Signature1` / `Signature2`.

**B. `certificate_visible_signature_renders`**
- `visible = Some(VisibleSignature { page: 0, rect: PointRect { left: 364.0, bottom: 28.0, right: 584.0, top: 88.0 },
  lines: vec!["Signed by {name}".into(), "{date}".into()] })` ;
- statut `Valid` ; rendu PDFium à l'échelle 1 : dans la zone du rectangle (lignes `792 − 88 .. 792 − 28`, colonnes
  `364 .. 584`) au moins 200 pixels non blancs (cadre + texte), alors que la même zone est entièrement blanche sur
  l'original (le rectangle bleu de la fixture couvre x 72..272, y 500..650) ; hors de la zone, les pixels sont
  identiques à l'original ;
- `anchored_rect(page 612 × 792, BottomRight, 220, 60, 28)` == le rectangle ci-dessus (test du calcul partagé avec
  `stamp_pages`).

**C. `certificate_sign_error_cases`** (codes d'erreur stables)
- mauvais mot de passe → `Err(EngineError::Signature(SignError::BadPassword))`, `code() == "cert_bad_password"` ;
- fichier non PKCS#12 → `NotPkcs12` ; `encrypted-rc4.pdf` → `EncryptedDocument` ;
- identité expirée sans `allow_expired` → `CertificateNotValid` ; avec → signe, et la vérification rapporte
  `certificate.valid_now == false` avec `status == Valid` (intégrité et validité du certificat sont deux
  informations distinctes) ;
- page visible hors limites → `PageOutOfRange` ; page pivotée (fixture tournée de 90° par `set_page_rotation`
  puis `snapshot`) → `RotatedPage`.

**D. `certificate_verify_rejects_malformed_and_unsupported`** (jamais de panique, jamais « valide » par défaut)
- sous-filtre remplacé par un nom de même longueur (`adbe.pkcs7.detached` → `adbe.x509.rsa_sha1_`) →
  `Unsupported` ;
- `ByteRange` dont une valeur dépasse la taille du fichier → `Unverifiable` ;
- zone `/Contents` restée à zéro (espace réservé non signé) → `Unverifiable` ;
- fichier tronqué à dix positions réparties, octets aléatoires, fichier vide : `Ok` ou `Err`, sans panique.

**E. `certificate_sign_modern_xref_stream_document`** : base produite dans le test par
`lopdf::Document::save_modern` (`L:writer.rs:57`, table de références en flux + flux d'objets) → `sign_pdf` →
`Valid` → PDFium l'ouvre et compte 1 signature.

**F. `certificate_verify_external_fixture`** (interopérabilité) : `signed-external.pdf`, signé par une autre
implémentation (OpenSSL via Python `cryptography`) → `Valid`, sujet « SuniPDF External Fixture ».

### 8.3 Fixtures

- `crates/sunipdf-core/tests/fixtures/signed-external.pdf`, produit par un nouveau script
  `scripts/make_signed_fixture.py` (Python 3.11 + `cryptography` 50.0.1, tous deux présents sur ce poste) :
  reprend le PDF `two-pages`, ajoute champ et dictionnaire de signature, signe avec
  `pkcs7.PKCS7SignatureBuilder` (détaché, DER, binaire), certificat auto-signé valable 100 ans.
  `scripts/make_fixtures.py` reste sans dépendance externe : script séparé, fixture versionnée. Le fichier ne
  contient que le certificat : **aucune clé privée n'est versionnée**.
- Aucune autre fixture : les identités sont générées par les tests.

### 8.4 Application et CLI

- `apps/sunipdf-desktop` : ajouter les variantes `EngineError::Signature(…)` à
  `every_error_key_exists_in_both_catalogs` et `include_str!("certificate.rs")` à la source balayée par
  `ui_keys_used_by_app_exist` (`app.rs:1965`). **Piège** : ce test prend pour clé i18n toute chaîne littérale en
  minuscules contenant un point ; ne mettre dans `app.rs` ni `certificate.rs` aucune chaîne de ce type qui ne
  soit pas une clé (les extensions `"pfx"` et `"p12"`, sans point, conviennent ; `adbe.pkcs7.detached` contient un
  chiffre, donc est ignorée, mais doit de toute façon rester dans le moteur).
- `registry_keys_exist_in_both_catalogs` couvre automatiquement les nouvelles commandes.
- Commandes de validation à exécuter par l'intégrateur : `cargo test -p sunipdf-signatures`,
  `cargo test --workspace -j 2`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, `.\scripts\check-i18n.ps1`.

### 8.5 Scénario manuel et captures (`scripts\dev-screenshot.ps1`)

```powershell
$id  = "C:\Temp\suni-test.pfx" ; $out = "C:\Temp\suni-signed.pdf"
cargo run -p sunipdf-cli -- cert-create --name "Test SuniPDF" --pfx-password test --out $id
cargo run -p sunipdf-cli -- sign crates\sunipdf-core\tests\fixtures\two-pages.pdf --pfx $id --pfx-password test --visible --position bottom-right --out $out
cargo run -p sunipdf-cli -- signatures $out            # attendu : "status": "valid", "trust": "not_checked"

# 1. Boîte de vérification, signature intacte ; la page montre le cartouche en bas à droite
.\scripts\dev-screenshot.ps1 -Out sig-valid.png -Pdf $out -Commands 'security.verify_signatures'
# 2. Fichier altéré : statut rouge « invalide »
$b = [IO.File]::ReadAllBytes($out); $i = [Text.Encoding]::ASCII.GetString($b).IndexOf("page 1"); $b[$i + 5] = 0x37
[IO.File]::WriteAllBytes("C:\Temp\suni-tampered.pdf", $b)
.\scripts\dev-screenshot.ps1 -Out sig-invalid.png -Pdf C:\Temp\suni-tampered.pdf -Commands 'security.verify_signatures'
# 3. Boîte de signature remplie au clavier (le champ du chemin a le focus à l'ouverture)
.\scripts\dev-screenshot.ps1 -Out sig-sign.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands 'security.sign_certificate' -KeysAfter 'C:\Temp\suni-test.pfx{TAB}{TAB}test'
# 4. Outil « Utiliser un certificat » (ou panneau « Sécurité ») avec les deux rangées
.\scripts\dev-screenshot.ps1 -Out sig-tools.png -Pdf $out
```
`-Drag` n'est pas utilisé : le placement se fait par ancre, pas à la souris. Le nombre de `{TAB}` du point 3 est à
ajuster à l'ordre de focus réel. Refaire les captures 1 et 3 en thème sombre et en anglais.

**Validation indépendante avant d'annoncer la fonction** (règle « fichier de sortie vérifié ») :
- OpenSSL 3.5.7 est présent (`C:\Program Files\Git\mingw64\bin\openssl.exe`) : extraire le CMS (octets du trou) et
  les deux tranches du `ByteRange` dans deux fichiers, puis
  `openssl cms -verify -binary -inform DER -in sig.der -content signed.bin -noverify -out NUL` → « Verification
  successful » ;
- Adobe Acrobat Reader DC est installé (`C:\Program Files (x86)\Adobe\Acrobat DC\Acrobat\AcroRd32.exe`) : ouvrir
  le fichier signé, panneau Signatures → signature reconnue, « identité inconnue » attendu pour un certificat
  auto-signé (contrôle visuel par l'utilisateur) ;
- `qpdf` et `pdfsig` sont absents de ce poste.

---

## 9. Nouvelles dépendances

| Crate | Version | Licence | Pourquoi | Nouveau dans `Cargo.lock` ? |
|---|---|---|---|---|
| `windows` | 0.62 (résout 0.62.2) | MIT OR Apache-2.0 | liaisons CryptoAPI / CNG ; *features* `Win32_Foundation`, `Win32_Security_Cryptography`, `Win32_System_SystemInformation`, `Win32_System_Time` (`windows-0.62.2/Cargo.toml:416`, `:518`, `:647`, `:651`) | **non** (déjà présent via `accesskit_windows`) ; devient une dépendance directe et entre dans la compilation de la CLI |
| `zeroize` | 1 (1.9.1 au 2026-10-14) | Apache-2.0 OR MIT | effacement du mot de passe du PFX (`String` et copie UTF-16), exigé par la fiche de l'agent ; `impl Zeroize for String` et `Vec<Z>` vérifiés sur docs.rs (crate absent du registre local) | **oui**, seul nouveau crate, sans dépendance |
| `thiserror` | workspace (2) | MIT OR Apache-2.0 | `SignError` | non |

`[workspace.dependencies]` : `sunipdf-signatures = { path = "crates/sunipdf-signatures" }`,
`windows = { version = "0.62", features = [...] }`, `zeroize = "1"`. `sunipdf-desktop` dépend aussi de `zeroize`
(effacement dans le worker et le formulaire). Aucune bibliothèque cryptographique ajoutée, aucun binaire tiers.

---

## 10. Risques, pièges, ordre, estimation

### 10.1 Hypothèses non prouvées par la lecture (à lever par les tests, dans cet ordre)

| # | Hypothèse | Preuve attendue | Repli |
|---|---|---|---|
| H1 | `CryptSignMessage` accepte un certificat importé avec `PKCS12_NO_PERSIST_KEY` (MS Learn : la propriété `CERT_KEY_CONTEXT_PROP_ID` est posée, et `CryptSignMessage` l'accepte) | test unitaire 2 | API bas niveau avec clé explicite : `CryptAcquireCertificatePrivateKey(cert, CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG \| CRYPT_ACQUIRE_SILENT_FLAG, …)` (`W:1096`, `W:8327`, `W:8329`), `CMSG_SIGNER_ENCODE_INFO { hNCryptKey, dwKeySpec: CERT_NCRYPT_KEY_SPEC }` (`W:7798-7820`, `W:5694`), `CMSG_SIGNED_ENCODE_INFO` (`W:7778`), `CryptMsgOpenToEncode(…, CMSG_DETACHED_FLAG, CMSG_SIGNED, …)` (`W:1631`, `W:7391`, `W:7762`), `CryptMsgUpdate` par tranche (`W:1644`), `CryptMsgGetParam(CMSG_CONTENT_PARAM)` (`W:1621`, `W:7236`), `CryptMsgClose` (`W:1586`) |
| H2 | `PFXExportCertStoreEx` exporte une clé CNG éphémère rattachée par `CERT_NCRYPT_KEY_HANDLE_PROP_ID` | test unitaire 1 | clé nommée `SuniPDF-tmp-<aléa>` puis `NCryptDeleteKey` (`W:2289`) ; à défaut, PFX de test produit par `scripts/make_signed_fixture.py` et `cert-create` reporté en roadmap |
| H3 | PDFium ouvre les mises à jour incrémentales de lopdf et compte la signature | test A | réécriture complète (`security::save`) en refusant les documents déjà signés — perte de la multi-signature |
| H4 | `FPDF_FFLDraw` dessine l'apparence d'un widget de signature (le rendu passe bien par là : `render_form_data(true)` `crates/sunipdf-core/src/lib.rs:239`, `P:pdf/document/page.rs:653-701`, formulaire initialisé dès qu'un AcroForm existe `P:pdf/document/form.rs:92-177`) | test B | l'apparence reste correcte pour les autres lecteurs ; documenter que SuniPDF ne l'affiche pas |
| H5 | une altération renvoie `CRYPT_E_HASH_VALUE` ou `NTE_BAD_SIGNATURE` | tests 2 et A | compléter la table de correspondance avec le code observé ; ne jamais élargir vers `Valid` |
| H6 | l'incrément lopdf est lisible quand la révision précédente utilise une table en flux | test E | retirer davantage de clés du *trailer* cloné, ou refuser ces documents avec une erreur typée |

### 10.2 Pièges connus

1. **Ne jamais faire transiter des octets signés par PDFium** (`snapshot`, `restore`, `save_as`, suspension
   d'onglet) : §2.4.
2. **`PKCS12_NO_PERSIST_KEY` est une exigence de sécurité**, pas une optimisation : sans lui la clé privée de
   l'utilisateur est copiée durablement dans son profil Windows. Contrôle manuel à faire une fois : le nombre de
   fichiers de `%APPDATA%\Microsoft\Crypto\Keys` ne change pas après une signature.
3. Le mot de passe traverse l'état egui, un canal `mpsc` et une copie UTF-16. `zeroize` efface nos copies ;
   **l'historique d'annulation interne du `TextEdit` d'egui n'est pas effacé** : limite à documenter, pas à masquer.
4. `IncrementalDocument::create_from` copie les octets d'entrée et `load_mem` charge tous les objets : mémoire
   ≈ 3 × la taille du fichier pendant la signature (même ordre de grandeur que `protect`).
5. Tailles en `u32` côté CryptoAPI et décalages `u32` côté lopdf (`L:writer.rs:605`) : fichiers de plus de 4 Gio
   refusés.
6. La date de signature est celle de l'ordinateur ; le libellé « Date déclarée » et la note de la boîte le disent.
7. `CertGetNameStringW` rend le nom d'affichage simple (souvent le CN) : deux certificats peuvent afficher le même
   nom ; l'empreinte SHA-256 est montrée pour lever le doute.
8. `windows` devient une dépendance directe : compilation de la CLI plus longue ; en repli GNU, clippy sur le
   crate `windows` est déjà signalé fragile (`docs/KNOWN_ISSUES.md` point 3).
9. Conflits de fusion certains avec les tranches parallèles sur `command.rs`, `shell.rs` (`tool-hub.md` supprime
   `ToolTab`), `app.rs` (`execute`, `error_key`, `handle_input`), `worker.rs` (énumérations) et les deux
   catalogues : intégrer l'étape 6 **après `tool-hub`**, seule.
10. Le garde-fou « Ctrl+S sans modification ne fait rien » touche un comportement que `compare-autosave.md`
    manipule aussi (`!self.dirty`) : à arbitrer par l'intégrateur.
11. Le test `ui_keys_used_by_app_exist` (§8.4) et le `match` exhaustif de `command_icon` cassent la compilation ou
    les tests si une clé ou une variante manque : c'est voulu, ne pas contourner.

### 10.3 Ordre d'implémentation (chaque étape compile et passe ses tests)

0. ADR **D021** (CryptoAPI plutôt que RustCrypto ; aucun réseau ; signature incrémentale ; périmètre) et accord de
   `architect` pour créer `crates/sunipdf-signatures`.
1. Crate `sunipdf-signatures` complet + tests 8.1. **Lève H1, H2, H5 avant tout travail PDF.**
2. `sunipdf-core` : `EngineError::Signature`, `certificate::verify_signatures`, `DigitalSignature`,
   `SignatureStatus`, `DocumentInfo::signature_count`, fixture externe + tests D et F.
3. `certificate::sign_pdf` invisible + tests A, C, E (lève H3, H6).
4. Apparence visible, `StampPosition::origin`, `anchored_rect` + test B (lève H4).
5. CLI `signatures`, `sign`, `cert-create` ; validation OpenSSL du §8.5.
6. Worker, commandes, icônes, `certificate.rs` de l'application, garde-fous d'enregistrement, i18n, tests de
   l'application (cas A ou B du §5.2).
7. Captures, contrôle Acrobat Reader par l'utilisateur, documentation (§11).

Les étapes 1 à 5 sont indépendantes des autres tranches et relèvent de `security-signatures` (+ `pdf-engine`
pour `sunipdf-core`, `automation` pour la CLI) ; l'étape 6 relève de `ui-ux`.

### 10.4 Estimation : **L**

≈ 350 lignes d'FFI `unsafe` à relire ligne par ligne, ≈ 350 lignes côté PDF, ≈ 350 lignes d'interface, ≈ 120
lignes de CLI, ≈ 350 lignes de tests, 55 clés × 2 langues, un script de fixture. Passe à **XL** si H1 et H2
tombent toutes les deux.

### 10.5 Questions ouvertes

- `architect` : nouveau crate `sunipdf-signatures` (proposé) ou module `unsafe` dans `sunipdf-core` ?
- Accepte-t-on `zeroize` comme seul nouveau crate ?
- Garde-fou n° 1 (Ctrl+S sans modification ne fait rien) : d'accord, ou préférer une confirmation explicite pour
  les documents signés ?
- Bouton « Créer une identité auto-signée » dans l'interface dès cette tranche (le moteur et la CLI l'auront), ou
  roadmap ?
- Tranche suivante : magasin de certificats Windows et cartes à puce d'abord, ou confiance hors ligne
  (`CertGetCertificateChain` sans réseau) d'abord ?

---

## 11. Documentation à mettre à jour

- `docs/DECISIONS.md` : **D021** (bibliothèque de signature, pas de réseau, signature incrémentale, statuts typés).
- `docs/SECURITY_MODEL.md` (à créer, périmètre de l'agent) : ce que « intacte » garantit et ne garantit pas ; clé
  privée jamais persistée ; mot de passe jamais journalisé ; aucun appel réseau ; limites du §2.3.
- `docs/ARCHITECTURE.md` : `sunipdf-signatures` passe de « planifié » à « actuel » ; paragraphe « signature =
  ajout incrémental, jamais via PDFium ».
- `docs/FEATURE_MATRIX.md` §E : ligne « Signature numérique CMS/PKCS#12, horodatage » → fait pour PKCS#12 +
  vérification ; horodatage, confiance et magasin Windows planifiés.
- `docs/ROADMAP.md` (phase 5) et `docs/NEXT_TASKS.md` : entrées de suite — confiance hors ligne, magasin Windows et
  cartes à puce, horodatage et révocation (ADR réseau), PAdES, placement à la souris, documents chiffrés,
  **enregistrement incrémental** (prérequis pour modifier un document signé sans casser ses signatures).
- `docs/KNOWN_ISSUES.md` : « tout enregistrement SuniPDF d'un document signé invalide ses signatures » ; limite
  egui sur le mot de passe ; signature ECDSA non testée.
- `docs/PROJECT_STATUS.md` : commandes réellement exécutées et résultats réels, nombre de tests ; ne rien annoncer
  « sûr » ni « terminé » sans la validation indépendante du §8.5.
- `docs/THIRD_PARTY_NOTICES.md` : `zeroize` ; mention de l'usage des API cryptographiques de Windows.
- `docs/USER_GUIDE.fr.md` / `.en.md` : « Signer avec un certificat », « Vérifier les signatures », lecture des
  statuts, différence avec la signature manuscrite, comment obtenir un `.pfx` (`sunipdf-cli cert-create` pour un
  essai).
- `docs/I18N.md` : préfixes `certificate.*` et `error.certificate.*`.
- `CHANGELOG.md`, et `CLAUDE.md` (liste des modules : `crates/sunipdf-signatures`, `certificate` dans le moteur
  et l'application, nombre de commandes CLI).
- `docs/AGENT_STRATEGY.md` : journal des invocations.
