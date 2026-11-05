# SuniPDF 0.93.0

By Emanuel de Almeida.

Published stable on 2026-11-05 under [lmdanavanem](https://github.com/lmdanavanem).
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.93.0) | [Qualification evidence](qa/RELEASE_0.93.0.json) | [Project status](PROJECT_STATUS.md) | [Changelog](../CHANGELOG.md) | [Known limitations](KNOWN_ISSUES.md).

## What changes for you

This release is about robustness when SuniPDF meets damaged, hostile or very large PDFs. There is no new screen or setting.

- `optimize` now reopens its result before publishing it. A copy that no longer opens, or that changes the page count, is refused with `invalid_format` and leaves no output file.
- `protect`, `sanitize`, `set-metadata`, `strip-metadata` and `sign` use the same check, so they no longer publish a copy that PDF readers cannot reopen.
- `stamp`, `page-numbers`, `headers-footers` and `image-watermark` no longer crash on a page whose content cannot be listed back; they fail with `invalid_format`.

## Validation added

- A generated corpus of 26 hostile and large PDFs (page-tree cycles, recursive forms, a 128 MiB Flate bomb, broken cross-references and encryption, 3,000 pages, a 600 dpi scan) runs `info`, `text`, `render`, `optimize` and `redact` under time, memory and output limits (Refs #97).
- Every PDF-writing CLI command (56) runs on that corpus: 1,315 runs, each ending in a typed error with no leftover file, or in a copy that reopens with the expected page count (Refs #98).
- Two cargo-fuzz targets (PDF structure, CMS signatures) and a fixed-seed mutation smoke test in `cargo test`. The `fuzz` workflow has not been run yet; no fuzzing campaign result is claimed.

## Validation and scope

The exact tagged source is `3ac860f5d976763a70f9572dd91562180b30228a`. All 11 jobs in the [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/41935987358) passed, and so did the ci, linux and macos runs on that commit. Upgrade checks include the published 0.92.2 packages. Locally, the full workspace tests, strict Clippy, formatting, 2,202 matching English/French keys and docs validation passed.

All five packages and their SHA256 files were downloaded after publication. Every package matched its checksum file and GitHub SHA256 digest.

## Limits

- `protect`, `set-metadata` and `strip-metadata` refuse PDFs whose MediaBox uses exponent notation, rather than publish a copy that cannot be reopened. See [known limitations](KNOWN_ISSUES.md).
- Issues #97 and #98 remain open: licensed real-world documents, forms, tagged/OCR files, cancellation, qpdf checks and hardware benchmarks are not covered yet.
- #106, #105, #154 and #99 move to 0.94.0.
- Windows packages remain unsigned. Windows ARM64 is cross-built without native hardware execution. The macOS app is ad hoc signed, not notarized.