# SuniPDF 0.94.0

By Emanuel de Almeida.

Published stable on 2026-11-07 under [lmdanavanem](https://github.com/lmdanavanem).
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.94.0) | [Qualification evidence](qa/RELEASE_0.94.0.json) | [Project status](PROJECT_STATUS.md) | [Changelog](../CHANGELOG.md) | [Known limitations](KNOWN_ISSUES.md).

## What changes for you

- Highlight, underline, strikeout and note annotations now carry an appearance stream. Before, some PDF readers that do not draw these themselves showed nothing, although PDF 2.0 requires it.
- `annotate --kind` accepts `highlight`, `underline` and `strikeout`.
- Internal clean-up only otherwise: shared helpers replace duplicated code, with no change of behaviour.

## Validation added

- `scripts/test-conformance.py --readers` writes encrypted copies, XMP metadata, filled and flattened forms, every annotation family and a PFX signature with the CLI, then checks concrete facts with QPDF, Poppler and OpenSSL (Refs #106). It runs in the Windows CI and in the Linux conformance job. Tool versions and SHA-256 are recorded in `report.json`.
- The highlight and note appearance was also checked by hand in Adobe Acrobat by the maintainer.

## Validation and scope

The exact tagged source is `3fcd85522133fae996810e6e7b2795c2eb7870ad`. All 11 jobs in the [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/42353667586) passed, and so did the ci, linux and macos runs on that commit. Upgrade checks include the published 0.93.0 packages. Locally, the full workspace tests (1,371 passed), strict Clippy, formatting, 2,202 matching English/French keys and docs validation passed.

All five packages and their SHA256 files were downloaded anonymously after publication. Every package matched its checksum file and GitHub SHA256 digest.

## Limits

- `pdfsig` has not been run on a signed copy: signing is available on Windows only, while Poppler runs on Linux.
- #105, #154 and #99 move to 0.95.0.
- Windows packages remain unsigned. Windows ARM64 is cross-built without native hardware execution. The macOS app is ad hoc signed, not notarized.
