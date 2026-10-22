# SuniPDF 0.39.0

## Changes

- Local named presets for text/image watermarks, page numbers and six-zone headers/footers.
- Load a draft, save under a unique name or delete explicitly; at most 16 presets.
- Atomic preference activation, bounded settings loading and isolation of malformed preset entries.
- Preserve current document/page scope and retain independent package, security and migration regression.
- Office version probes use headless startup and a cancellable 30-second cold-start budget; conversion and security guards remain enforced.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

[0.39.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) under `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404372880) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404384844) passed: **628 tests, zero failures**, 10 default ignored, **1149 EN/FR keys**, **31 portable scenarios**, **25 MSI baselines/125 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **42 cases, 148 CLI calls and 2 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,879,736-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed.

[Detailed evidence](qa/RELEASE_0.39.0.json).

## Limits

Image presets reference the original path; keep the file available. Scope is checked each time. Explicit preset save/delete persists independently of cancelling the document operation. No macro, CLI preset import/export or signing-identity storage. Packages remain unsigned and ARM64 is cross-built only. Other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
