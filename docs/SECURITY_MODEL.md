# Document sanitization and redaction

SuniPDF processes documents locally. Sanitization and visible-page redaction are separate operations.

## Sanitizing a new copy

**Sanitize a new copy** / `sunipdf-cli sanitize` rewrites the current document into a new file. It removes
metadata, bookmarks, named destinations, attachments, JavaScript and automatic/additional actions.
File-attachment and multimedia annotations are removed. Form values, ordinary annotations and visible
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
Sanitization does not securely erase disk sectors, cloud backups or files held by other applications.
Visible text/images, form values, OCR text and custom application data outside the cleaned structures
are not anonymized. Check the result before sharing. Existing digital signatures are invalidated by the
rewrite; retain the signed original if its signature must remain verifiable.

## Visible-page redaction

Applying redaction removes content intersecting the selected regions, rewrites the result and checks
for remaining extractable text under the marks. The existing conservative fallback can rasterize a
crossing object or page, so unredacted text can lose selectability. Merely drawing a black rectangle,
adding a note, cropping a page or saving unapplied marks does not redact the underlying content.

Redaction and sanitization are not PDF/A/PDF/UA validation or a guarantee against every hostile PDF
feature. Their structural checks, native rendering and independent-reader/corpus checks are separate
evidence. Hardware, archival signature validation and exhaustive hostile-input fuzzing remain separate QA.
