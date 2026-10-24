# SuniPDF 0.33.0

## Changes

- Preserve bounded imported UTF-8 bookmark titles containing tabs and line breaks during page/destination
  operations. New or renamed titles keep their single-line control-character guard; source bytes and raw
  imported titles remain unchanged when creating a separate copy.
- Migrate egui/eframe together to 0.37, with supported accessibility roles, stable IDs and scroll APIs.
  Keep PDF coordinates bounded so Back/Forward restores the position after destination zoom changes.
  Dependabot now groups the coupled framework updates.
- Refresh all 17 affected framework license records against the locked crates. Full terms remain required
  for both Windows graphs; unavailable core egui VCS objects have an explicit official-release provenance
  exception in the [third-party notices](THIRD_PARTY_NOTICES.md).
  Include the fifth default-font source, the MIT-licensed `egui-icons` subset, with its modification notice;
  unknown font sets or missing notices still prevent packaging.
- Add seven independently authored PDF Association examples, pinned to an exact upstream revision and
  SHA256, under CC BY-SA 4.0. Text/pixels, annotations, qpdf output and immutable sources are checked during
  optimization/extraction; these are teaching examples, distinct from the four NIST documents.
- Add 32 PDF and 32 CMS mutations with typed-result/reopening/source checks, positive CMS verification and
  executable timeout/output negative controls. Native child processes have explicit time, sampled memory
  and per-stream output guards. This is deterministic mutation smoke testing, not coverage-guided fuzzing.
- Record per-command duration, sampled peak child working set and hardware/CI metadata in QA reports.
  Add the public 0.32.0 ZIP/MSI as real upgrade baselines, with verified digests and MSI product properties.
- Publish [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md), retain open P1 qualification work, and correct
  stale distribution/pivot status in the current documentation.

## Limits

Windows packages remain unsigned and no independent update publication key is configured. Updates still
depend on GitHub HTTPS and same-channel SHA256. ARM64 is cross-built only. Physical printers/GPU windows,
Narrator, interactive installation/update/save flows, other PDF readers interoperability, broader real/hostile
corpus coverage and coverage-guided fuzzing remain separate qualification. No PDF/A/PDF/UA or archival-LTV
certification is claimed. Downloaded examples and derived PDF copies are excluded from Git/packages.

## Validation and distribution

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) under `lmdanavanem`, release `417802490`, immutable source `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) passed: **592 tests**, zero failures, 10 default ignored, **1092 EN/FR keys**; 84 qpdf corpus checks and 9 independent feature outputs, 4 font cases, DX12 software rendering, 14 Office round trips, four NIST documents/744 pages; **7 PDF 2.0 examples/8 pages/21 qpdf checks**, **64 mutations and 2 guard controls**, **25 portable scenarios**, **19 MSI baselines/95 logs**. The packaged CLI additionally passed the 7-example/21-qpdf suite, 11 object-order scenarios/3 qpdf checks, 11 local-review and 8 security scenarios. CodeQL Rust/Python/Actions passed with zero open alerts.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 test harness, with its unchanged updater module verified by hash, downloaded and verified the **26,726,762-byte x64 ZIP in 1.65 s**, without installation.

Project edits are deferred at the owner’s explicit request. The public readback shows **43 items, 25 Done, 18 Todo**; both P1 tasks have documented partial progress, with no claimed closure or status change.

[Detailed evidence](qa/RELEASE_0.33.0.json).
