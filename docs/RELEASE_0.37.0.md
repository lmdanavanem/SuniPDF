# SuniPDF 0.37.0

## Changes

- Image watermarks in the existing Watermark dialog and the `image-watermark` CLI command.
- Transparent foreground images, proportional width, angle, seven positions and displayed-page fitting.
- Atomic selected-page edits, one desktop Undo, copy-only CLI output and preserved allowed encryption.
- Independent geometry/pixel/refusal/protection regression with qpdf; published 0.36.0 upgrade baselines retained.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

[0.37.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) under `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) passed: **618 tests, zero failures**, 10 default ignored, **1134 EN/FR keys**, **29 portable scenarios**, **23 MSI baselines/115 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **24 cases, 113 CLI calls and 1 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included. Two fixture-password findings were reviewed and dismissed as “used in tests” under `lmdanavanem`; code scanning remains enabled.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,806,746-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed.

[Detailed evidence](qa/RELEASE_0.37.0.json).

## Limits

Foreground image watermarks are editable content. No background/tile layer, DRM or signature verification is implied. Packages remain unsigned; ARM64 is cross-built only. The other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
