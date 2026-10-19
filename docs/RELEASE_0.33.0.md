# SuniPDF 0.33.0

[Français](RELEASE_0.33.0.fr.md)

## Changes

- Preserve bounded imported UTF-8 bookmark titles containing tabs and line breaks during page/destination
  operations. New or renamed titles keep their single-line control-character guard; source bytes and raw
  imported titles remain unchanged when creating a separate copy.
- Migrate egui/eframe together to 0.37, with supported accessibility roles, stable IDs and scroll APIs.
  Keep PDF coordinates bounded so Back/Forward restores the position after destination zoom changes.
  Dependabot now groups the coupled framework updates.
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
Narrator, interactive installation/update/save flows, Acrobat/Foxit interoperability, broader real/hostile
corpus coverage and coverage-guided fuzzing remain separate qualification. No PDF/A/PDF/UA or archival-LTV
certification is claimed. Downloaded examples and derived PDF copies are excluded from Git/packages.

## Validation

Release qualification is in progress. Exact CI, package, public-download, updater and Projects results
will be recorded after successful validation; the stable public release is still 0.32.0.
