# SuniPDF 0.31.0

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

## Validation and distribution

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) under `lmdanavanem`, release `417463135`, immutable source `362c7f3979533000e454769e48a8cfbd4084dc0d`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) passed: **582 tests**, zero failures, 9 default ignored; **1091 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **23 portable scenarios**, **17 MSI baselines/85 logs**. CodeQL Rust/Python/Actions passed with zero open alerts after individual review of four synthetic test credentials.

Six public files and `dist/` copies verified by SHA256; eleven local-review and eight security scenarios passed with the packaged CLI. The freshly built harness from this source downloaded and verified the **27,090,504-byte x64 ZIP in 1.71 s**, without installation. Projects: **43 items, 24 Done, 19 Todo, zero In Progress**; both P2 Suggestions are delivered, their criteria retained and all three filters read back.

[Detailed evidence](qa/RELEASE_0.31.0.json).
