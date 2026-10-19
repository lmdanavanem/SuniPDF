# SuniPDF 0.31.0

[Français](RELEASE_0.31.0.fr.md)

## New features

- **Compare two saved PDF versions locally.** Align physical pages with explicit offsets, review extracted-text, page-size and preview-pixel differences, then open an original/revised/red-difference preview for a changed pair. A changed source is refused before previewing.
- **Search several selected PDFs in one operation.** Literal text search with optional case sensitivity, per-file outcomes, page excerpts and buttons opening matching pages. Missing, malformed, protected and copy-restricted files remain explicit.
- **Automate both operations.** New `compare` and `search-files` JSON commands reuse the same bounded engine. A partial analysis prints its report and exits with code 2. Existing environment/stdin password sources are supported.

## Improvements

- Progress and cancellation keep completed page results. Read-only jobs preserve the sources and never upload documents, crawl directories or save an index.
- English and French controls, including a warning for page pairs with no extractable text.
- Portable/MSI upgrade regression includes the public 0.30.1 packages as an additional baseline.

## Supported limits

The [local review guide](LOCAL_REVIEW.md) defines alignment, limits, preview resolution and failure behavior.
Saved files only; no automatic OCR or page alignment. A 768-pixel comparison with tolerance 16/255 cannot establish PDF identity, semantic equivalence, signature validity, hidden-data removal or archival conformance.
The desktop reports password-protected files; the CLI accepts one credential for all selected inputs.
Windows packages remain unsigned and update authenticity still depends on GitHub HTTPS plus same-channel SHA256. Independent publication signing awaits a supplied key. ARM64 is cross-built; runtime hardware/manual qualification remains separate.

## Validation

Release qualification is in progress. Published-source results, package checks and public download verification will be recorded in [Project status](PROJECT_STATUS.md).
