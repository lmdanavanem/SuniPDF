# SuniPDF 0.67.0

Published as stable Latest on 2026-10-24. [Download packages and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.67.0).
This release combines reviewed plan links, revision outlines and rooms/zones in one distribution.
Intermediate local builds are not separate published releases.

## Reviewed links between plan sheets

Find exact indexed sheet identifiers in existing searchable PDF text. Review source occurrences and
real destination previews, resolve ambiguous targets explicitly, and publish standard relative PDF
links in a new copy. Navigation verifies the selected manifest and target fingerprint. Protected
inputs, source changes, unsafe paths, existing annotations and output collisions retain explicit guards.

The scope is one indexed source PDF, up to 32 selected pages and 512 occurrences. OCR must already
exist. Multi-source batch orchestration and arbitrary external-reader behavior remain separate work.
See [Plan links](PLAN_LINKS.md).

## Reviewed revision outlines

Compare an explicit old/new physical page pair with a manual transform or two point pairs. Classify
proposals as content changes, rendering noise or uncertain; confirm each selected decision and inspect
the actual annotated candidate before saving. New revision outlines remain editable and carry source/region
provenance in the register. The revised PDF's other pages and original contents are preserved.

Raster comparison is bounded to 1,536 pixels on its longest side, 512 proposals and 64 selected
decisions. Filtered, omitted, unreviewed and unmapped areas remain visible. Noise-only reviews do not
publish empty copies. A partial report is not proof of full drawing equivalence. See
[Revision outlines](REVISION_OUTLINES.md).

## Rooms and zones

Define named rectangular zones with optional parent zones on the actual page. Preview annotation
assignments, apply one undoable edit and save the metadata in the PDF. Register and quantity filters
share the same full-bounds assignment and coverage rules. Boundary, overlap, unknown geometry and
stale page evidence cannot silently disappear into a filtered total.

The limits are 64 zones per page and eight hierarchy levels. Zones do not infer room area or change
measurement calibration. JSON replacement and CLI inspection/preview/new-copy publication share the
core. See [Rooms and zones](PLAN_ZONES.md).

Zone filters now share the compact **Filters** menu in the annotation register and quantity table.
The button shows the active filter count, with their values available on hover. Assignment coverage
and totals are grouped under **Totals**; incomplete coverage keeps a visible warning on the button.

## Source qualification

Source `b920a600e9c53c3a265246c385a58b77d5a7b42e` was merged through
[PR #43](https://github.com/lmdanavanem/SuniPDF/pull/43); its tree matches the frozen development candidate.

The frozen Windows development candidate passed formatting, strict Clippy and both builds, with
1,045 tests passed, no failures and 15 ignored across 94 suites. Both language catalogs contain 1,789 keys.

| Independent development corpus | SuniPDF CLI processes | qpdf processes | Total processes | Assertions |
|---|---:|---:|---:|---:|
| Plan links | 26 | 4 | 30 | 329 |
| Revision outlines | 34 | 15 | 49 | 542 |
| Rooms and zones | 68 | 9 | 77 | 1,670 |
| Total | 128 | 28 | 156 | 2,541 |

Process totals include qpdf; they are not all SuniPDF CLI invocations. These development results are
separate from downloaded-package checks and are not added to a later public-runtime total.

## Package qualification

Exact-source CI, release, Linux and CodeQL workflows passed. Windows recorded 1,045 passed,
no failures and 15 ignored across 94 suites; Linux recorded 1,026 passed, no failures and 10 ignored
across 94 suites. The two language catalogs contain 1,789 keys. All 17 checks passed, including the
15 required checks, with no open code-scanning alerts at qualification time.

Portable upgrade qualification covered 45 baselines plus four guards. MSI qualification covered
43 baselines / 215 logs. The exact DEB was installed and checked on Ubuntu 22.04 and 24.04.
These automated installation results retain the physical-device and native-interaction limits below.

All eight public assets were downloaded anonymously and verified against their SHA256 values.
The downloaded Windows x64 ZIP passed six runtime corpora: 238 recorded commands / 3,603 assertions.
The commands comprise 196 SuniPDF CLI, 40 qpdf and two Python/PowerShell validators; subprocesses
internal to those validators are not counted. The package remained unchanged and Latest was checked
before and after execution. Draft and public passes are repeated checks, not additive coverage.
[Release evidence](qa/RELEASE_0.67.0.json) preserves the source, hashes and separate result scopes.

## Native qualification

Native interaction with the new 0.67 plan-link, revision-outline and zone workflows has **not been
tested**: the local application-control channel was unavailable. Earlier 0.65 plan-link and 0.66
revision-outline development checks are historical results for their own binaries; they do not
qualify these workflows in 0.67. Offscreen UI tests, CLI outputs, source inspection and Ubuntu
container reader smoke tests do not replace this missing interactive check.

## Measured performance

A paired development-build benchmark compared 0.64 with 0.67 on six whole-CLI-process workloads,
with one warmup and five measured repetitions per case. Median times increased by 0.5% for document
information, 5.0% for dense-page rendering, 2.6% for search, 4.0% for the 1,500-row register and 3.4%
for 300 quantities with eight formulas. Optimization/output decreased by 0.8%. These bounded
measurements establish no overall speed improvement, GUI responsiveness result or release-package
performance claim. Rendering and table costs need further profiling without weakening source checks.

## Continuing limits

The three related backlog cards remain **In progress**. AEC-017 retains broader multi-source batch
scope; AEC-014 and AEC-010 retain the remaining native and interoperability qualification. No complete
P1/P2 category is closed. Packages remain unsigned. ARM64 execution, physical printers, human
accessibility and private project plans remain unqualified. Container/software-rendering checks do not
establish physical desktop/GPU coverage. Synthetic and licensed corpus results do not establish
universal interoperability, survey accuracy or PDF/A/PDF/UA compliance. Ignored tests are not passed
checks. English is the default interface; detailed engine refusals may remain English in French mode.
