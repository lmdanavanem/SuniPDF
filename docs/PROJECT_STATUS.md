# Project status

**0.60.0 is published as stable Latest.** Publication and anonymous download verification
were recorded on 2026-10-23. See the [release](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.60.0),
[release notes](RELEASE_0.60.0.md) and [machine-readable evidence](qa/RELEASE_0.60.0.json).

Source: `90691d0bca160a9bfc22752dc2cf4269c5e7b1e4`, merged through
[PR #33](https://github.com/lmdanavanem/SuniPDF/pull/33), tag `v0.60.0`.
The evidence below applies to that source and its packages, not to later documentation edits.

## Delivered in 0.60

- Eligible vector endpoint/intersection snapping and feet/inches input.
- Named page regions with independent scales and explicit overlap handling.
- Selected-page/measurement recalibration with preview, atomic application and Undo.
- Cached page layouts and register results, virtualized tables and adjustable docking.
- Cooperative page-by-page search and annotation inventory with progress/cancellation.

The [plan-workspace guide](PLAN_WORKSPACE.md) defines supported geometry, stale-data guards,
resource bounds and CLI usage. Existing local PDF workflows remain in the
[feature matrix](FEATURE_MATRIX.md).

## Qualification on the release source

| Check | Observed result |
|---|---|
| Windows workspace | 827 passed, 0 failed, 14 ignored, 78 suites |
| Linux main workspace | 808 passed, 0 failed, 9 ignored, 78 suites |
| Linux package-build workspace | 808 passed, 0 failed, 9 ignored, 78 suites |
| Application translations | 1469 matching English/French keys |
| Portable updater | 44 scenarios: 40 upgrades and 4 guards |
| MSI upgrades | 38 baselines and 190 verified logs |
| Code Scanning | No open alerts at qualification |
| Public downloads | Eight anonymously downloaded files verified by SHA256 |

Successful exact-source workflows:
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668),
[Linux CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704),
[release packaging](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990),
[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004268835).
Ignored tests are not counted as passed qualification.

## Packages and UI evidence

The published files are the Windows x64 portable ZIP and MSI, Windows ARM64 portable ZIP,
Ubuntu amd64 DEB, and their four SHA256 sidecars. Exact sizes, hashes and URLs are recorded
in the [release evidence](qa/RELEASE_0.60.0.json).

Windows UI qualification rendered five actual application/PDFium scenes through offscreen
DX12 WARP, including the register at 1280 x 780 and quantity table at 720 x 480.
It does not establish native window presentation or physical GPU/input behavior.

Ubuntu 22.04/24.04 checks installed the actual candidate under a normal user with Xvfb/software
Glow, exercised rendering, save/reopen and English/French OCR, then removed/reinstalled that
same candidate. They do not establish previous-version Linux upgrades or physical desktop support.

## Recorded performance

These are bounded model operations in debug test binaries on CI runners.

| Operation | Workload / samples | Windows p95 | Linux main p95 | Linux release p95 |
|---|---|---:|---:|---:|
| Cached layout | 5000 pages / 1000 samples | <1 us | <1 us | <1 us |
| Register filtering | 10000 annotations / 100 samples | 12534 us | 10608 us | 6399 us |

The stored layout value is 0 integer microseconds because conversion truncates sub-microsecond
durations; this does not mean zero elapsed time. No median was recorded. Complete UI frame time,
rendering latency, native memory and performance on physical devices remain separate measurements.

## What remains open

- Private real-project plan qualification is deferred at the owner's request. Automated checks
  use known synthetic geometry and the retained licensed/public corpus, without a survey-accuracy claim.
- Windows packages remain unsigned; no independent publication key is configured for updates.
- Windows ARM64 is cross-built without native execution. Physical printers, native GPU/window
  behavior, accessibility and interactive installation/update flows remain unqualified.
- Broader hostile/large-file coverage, independent-reader interoperability, PDF/A/PDF/UA
  conformance and archival signature trust require their own evidence.
- The remaining [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md) stay open.
  GitHub Projects updates remain deferred.

The [roadmap](ROADMAP.md) contains current work, and [known limitations](KNOWN_ISSUES.md)
describe current product boundaries. Historical release-by-release implementation and test
records are preserved in the [pre-0.60 archive](archive/PROJECT_STATUS_PRE_0.60.md);
their old "Latest", "pending" and "not done" statements are not current status.
