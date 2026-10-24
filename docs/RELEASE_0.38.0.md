# SuniPDF 0.38.0

## Changes

- Six editable header/footer zones in Pages → Page numbers and the `headers-footers` CLI command.
- Absolute page/count placeholders, font size, margins, page selection and displayed rotation/crop coordinates.
- Refuse overflowing text atomically, preserve allowed encryption and keep CLI output copy-only.
- Extend independent package regression to 42 cases, 148 CLI calls and two qpdf checks.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

[0.38.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) under `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) passed: **623 tests, zero failures**, 10 default ignored, **1143 EN/FR keys**, **30 portable scenarios**, **24 MSI baselines/120 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **42 cases, 148 CLI calls and 2 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,823,494-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed.

[Detailed evidence](qa/RELEASE_0.38.0.json).

## Limits

Single-line zones use equal-width columns and foreground text. Existing content can overlap; inspect before saving. No automatic dates, odd/even rules, complex-script qualification or conformance certification. Packages remain unsigned and ARM64 is cross-built only. Other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
