# Document sanitization and redaction

SuniPDF processes documents locally. Sanitization and visible-page redaction are separate operations.

## Sanitizing a new copy

**Sanitize a new copy** / `sunipdf-cli sanitize` rewrites the current document into a new file. It removes
metadata, bookmarks, named destinations, attachments, JavaScript and automatic/additional actions.
XFA packets, file-attachment and multimedia annotations (including Screen) are removed and their absence
is rechecked. Ordinary AcroForm values, ordinary annotations and visible
page content are retained. Removing destinations can disable links that used their names.

Font-resource names used by page/form/pattern content or default appearances are retained. Safely
identified unused entries are detached; unreachable objects, including their font programs and embedded
files, are pruned before a full rewrite. A reused font name is conservatively retained across contexts.
This does not prove that every unused subset has been removed. Type3/ExtGState font selection and
malformed content are refused rather than risking visible text loss.

The result is reopened and its cleaned structural keys are checked before publication. Encryption,
passwords and permissions are preserved. Existing files are never replaced, including when the CLI uses
`--force`. A failed operation does not change the active document or publish a partial output.
Limits: 128 MiB input, 250,000 objects, one million inline nodes, depth 32, 8 MiB decoded content per
stream/page and 256 MiB cumulatively. Unsupported filters/structures are explicit errors.

## What remains

The original, previous copies, backups, recovery snapshots and Undo history retain the original data.
Sanitization does not securely erase disk sectors, external backups or files held by other applications.
Visible text/images, form values, OCR text and custom application data outside the cleaned structures
are not anonymized. Check the result before sharing. Existing digital signatures are invalidated by the
rewrite; retain the signed original if its signature must remain verifiable.

## Visible-page redaction

Applying redaction removes content intersecting the selected regions, rewrites the result and checks
for remaining extractable text under the marks. The existing conservative fallback can rasterize a
crossing object or page, so unredacted text can lose selectability. Merely drawing a black rectangle,
adding a note, cropping a page or saving unapplied marks does not redact the underlying content.

Hidden-data cleanup is on by default. It removes document-wide metadata/XMP, accessibility structures,
bookmarks/destinations, attachments, scripts, XFA and replacement `ActualText`/`Alt` values in retained
page, form and pattern content/resources. Unsupported ambiguous inline-image rewrites fail without
changing the document. Turning cleanup off preserves the chosen hidden data, which can retain sensitive
information outside the marked regions; inspect it separately. Neither choice erases previous files,
backups, recovery snapshots or other copies.

Replacing encryption requires the existing owner credential, even when the opening password permits
some edits. The operation does not temporarily remove protection or publish an unprotected intermediate.

## Command-line passwords and update trust

Prefer `--password-env NAME` or `--password-stdin` over a literal PDF password argument. `protect` also
accepts `--user-password-env`/`--user-password-stdin` and `--owner-password-env`/`--owner-password-stdin`
for its new credentials. Only one credential can use stdin per invocation; input must be piped UTF-8,
is bounded to 65536 bytes and loses at most one trailing line ending. Do not mix sources for a role.
Environment variables still belong to the process environment: populate them through an appropriate
secret source and clear them afterwards. Legacy argv secrets remain visible to process/history tools.

Release builds ignore the development `SUNIPDF_PDFIUM_DIR`, `SUNIPDF_OCR_DIR`, `SUNIPDF_SIGNATURES_DIR`
and `SUNIPDF_PICK_FILE` overrides. Explicit core library-directory APIs remain available to callers.
GitHub HTTPS and same-channel SHA256 checks remain the updater's distribution trust boundary; there
is no independently signed update manifest or publisher key. Checksums alone do not provide that
independent authenticity. Windows packages remain unsigned.

Redaction and sanitization are not PDF/A/PDF/UA validation or a guarantee against every hostile PDF
feature. Their structural checks, native rendering and independent-reader/corpus checks are separate
evidence. Hardware, archival signature validation and exhaustive hostile-input fuzzing remain separate QA.
