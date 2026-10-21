# SuniPDF 0.41.0 — manual plan counts and annotation registers

## Changes

Place persistent count markers directly on local PDF plans, group items, move/delete with Undo, regroup without changing identity and export totals linked to actual markers. The all-page annotation register adds search, named filters, sorting, navigation, business statuses, disciplines and custom text fields, with guarded CSV/JSON exports. Changes refuse stale snapshot targets and preserve originals and annotation permissions. Pinned test-only Pillow 13.0.0 replaces the vulnerable QA dependency. Published 0.40.0 packages join upgrade regressions; cloud and collaboration remain deferred. Shared annotation writers now enforce PDF annotation permission for moves, deletion, replies and other annotation edits; register exports also refuse incomplete page inventories or malformed count identities.

## Validation

[0.41.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) under `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681919685) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681938796) passed: **643 tests, zero failures**, 10 default ignored, **1198 EN/FR keys**, **33 portable scenarios**, **27 MSI baselines/135 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **100 cases, 339 CLI calls and 14 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included. 6 fixture-password findings were reviewed and dismissed as 'used in tests' under `lmdanavanem`; code scanning remains enabled.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **27,323,650-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed.

[Detailed evidence](qa/RELEASE_0.41.0.json).

## Limits

- Counts are explicit manual SuniPDF markers, one item per marker. No visual symbol detection, automatic counting or proprietary Bluebeam count metadata import is claimed.
- Register limits are 1000 pages, 10000 annotations in total, 4096 per page, 4 MiB of metadata and 64 levels of replies. Unsupported or malformed SuniPDF metadata refuses the complete export. Register viewing/export requires PDF copy permission.
- Named filters use exact text; search includes custom fields. Modification dates retain PDF syntax and sort lexically, without timezone normalization. Custom fields are text only, maximum 16; typed/formula columns and sorting by arbitrary custom keys remain future work.
- Exports describe the current in-memory PDF snapshot. JSON preserves null values and count-to-marker references. CSV uses empty cells for unknowns, quotes all values and escapes spreadsheet formula prefixes. Existing output files and originals are never replaced.
- Local functions stay free and open source. Cloud, collaboration, deployment, prices and Projects edits remain deferred.
- Packages remain unsigned without an independent publication key. ARM64 is cross-built only. Native hardware, physical printing, Narrator, installed updater interaction, broader hostile/customer corpus and PDF/A/PDF/UA conformance remain separate 1.0 qualification.
