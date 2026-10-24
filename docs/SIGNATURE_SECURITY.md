# Signature storage and verification decisions

Implementation through 0.30.1. Test results belong in
`PROJECT_STATUS.md`; this document describes the guarantees and remaining limits.

## Reusable visual signatures

Saving remains optional. Clearing **Save on this computer for reuse** places the image without
writing a new library file. Reusable images are not digital signatures and do not establish
identity or prevent copying from a PDF.

New files in `%APPDATA%\SuniPDF\signatures` use the `.dpapi` extension. Windows DPAPI encrypts the
PNG for the current user, with application-specific entropy and no machine-wide key option.
The file has a protected DACL granting access only to that user's SID. The small versioned header
contains the public Windows SID, not a recovery key. Plain PNG bytes are decoded in memory;
there is no decrypted temporary file or network upload.

Legacy `signature-<digits>.png` and `initials-<digits>.png` entries migrate individually when loaded.
The application validates the bounded PNG, encrypts to a neighbouring temporary file with a private
DACL, flushes and publishes without overwriting an existing destination, decrypts the persisted file
and compares the exact original bytes. Only after verifying that the legacy file has not changed
does it remove that PNG. An existing different, inaccessible or corrupt encrypted file causes a
refusal: both originals remain. If migration fails, the legacy image remains usable with a warning;
there is no silent claim that it has been protected.

The library refuses parent traversal, reparse points, unrecognised names, oversized blobs and PNGs
above 1600 pixels per side / 32 MiB decode allocation. Encoded PNGs are limited to 16 MiB; encrypted
files have an additional bounded header/DPAPI allowance. Previously inserted larger foreign library
PNGs remain on disk and can be imported separately through the regular image import flow.

Keep the original image or a recovery backup of the Windows profile and its DPAPI keys. Copying only
`.dpapi` files to a new Windows user or losing/resetting the profile does not provide recovery.
The app preserves unreadable encrypted files and asks for the original profile or a new image import.
It does not reset or replace keys. The delete button still requires two clicks and deletes the selected
library entry; it is ordinary file deletion, not secure erasure of old disk sectors or backups.
Anyone able to operate the already signed-in user session can reuse the images.

## Certificate verification

PDF CMS integrity and signer trust are separate results. A valid detached CMS signature proves a
relationship to the saved byte ranges; it does not itself authenticate an identity. Later revisions,
malformed byte ranges and unsupported CMS remain separately reported.

Since 0.30.1, integrity requires detached CMS and a SHA-256, SHA-384 or SHA-512 signer digest.
MD5, SHA-1, other digests and CMS carrying attached content are refused. `identity_trusted` requires
valid current integrity, a signer certificate and successful chain/revocation checks together.
The lower-level `SignatureTrust::chain_and_revocation_trusted()` describes only chain/revocation.

The desktop checks the exact loaded/saved byte snapshot. Changed or missing disk files and unsaved
edits prevent signing/verification until reloading or saving; later disk changes cannot substitute
the bytes being checked. PDF field, reason, location and signing-time declarations are filtered and
bounded to 256 characters and shown separately from verifier results. Certificate subject/issuer
use bounded X.500 names. PFX files are limited to 16 MiB/256 certificates; multiple private-key
identities are refused rather than silently selecting one.

CLI `signatures` prints its full JSON report before exit code 2 for any failed signature verification.
An unsigned document retains exit code 0 with both verification flags false. `failure_cause` is a
machine-readable reason; `declared_signing_time`/`signing_time_source` identify declarative dates.
The legacy `signing_time` field remains an alias for that declaration.

The default check uses the current Windows trust store, document-signing usage, strong digest and
certificate policy, and separately reports revocation as good, revoked, offline, unknown or not checked.
Both certificate URL retrieval and revocation URL retrieval are cache-only; AIA and automatic root
updates are disabled. A trusted root without usable revocation evidence does not produce
`identity_trusted: true`. Windows trust-store configuration is part of the decision, not an AATL/EUTL
or qualified-signature assessment.

Online retrieval requires a checkbox for each explicit **Verify** action. It resets after that action
and on reopening the dialog. Windows can contact issuer/AIA/CRL/OCSP endpoints and disclose certificate
identifiers; the PDF is not uploaded. Retrieval has a five-second accumulative revocation timeout;
automatic trust-root updates stay disabled. Failure or stale evidence remains unavailable/unknown,
never a fabricated successful revocation check. CLI verification remains offline by default.

Supported unsigned CMS timestamp attributes are RFC3161 signature timestamps and the Windows RFC3161
OID. CryptoAPI verifies the token against the original CMS signature value. Strong token digest,
critical exclusive timestamping EKU, signing key usage, time sanity, TSA certificate validity at token
time, TSA chain and revocation are checked. The PDF `/M` value is only a signer declaration and is never
promoted to a trusted time. Legacy countersignatures are explicitly unsupported. A cryptographically
valid token from an untrusted TSA is shown as untrusted; absent revocation evidence prevents a trusted
timestamp result. No TSA request or timestamp creation is added to signing.

Embedded `/DSS` certificate/OCSP/CRL containers are inspected with count/size/type limits. Presence is
reported as **embedded, unverified**, not as authenticated archival LTV. Historical trust-list policy,
archival OCSP/CRL authentication and document timestamp renewal are not implemented. AATL/EUTL,
PKCS#11 hardware tokens, qualified signatures and third-party-reader certification remain separate work.

Windows primitives: [DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata),
[certificate chains](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certgetcertificatechain),
[timestamp verification](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptverifytimestampsignature).
