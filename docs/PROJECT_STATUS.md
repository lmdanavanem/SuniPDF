# Project status

**0.67.0 is published as stable Latest.**
Updated 2026-10-24. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.67.0) ·
[Release scope](RELEASE_0.67.0.md) · [Qualification evidence](qa/RELEASE_0.67.0.json).

## Combined 0.67 scope

Source `b920a600e9c53c3a265246c385a58b77d5a7b42e` was merged through
[PR #43](https://github.com/lmdanavanem/SuniPDF/pull/43). Its tree matches the qualified development source.
The increment combines reviewed plan links, revision outlines and named rooms/zones.

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

## Published package qualification

Exact-source CI, release, Linux and CodeQL workflows passed. Windows recorded 1,045 passed,
no failures and 15 ignored across 94 suites; Linux recorded 1,026 passed, no failures and 10 ignored
across 94 suites. The two language catalogs contain 1,789 keys. All 17 checks passed, including the
15 required checks, with no open code-scanning alerts at qualification time.

Portable upgrade qualification covered 45 baselines plus four guards. MSI qualification covered
43 baselines / 215 logs. The exact DEB was installed and checked on Ubuntu 22.04 and 24.04.
These automated installation results retain the physical-device and native-interaction limits below.

All eight public assets were downloaded anonymously and their SHA256 values verified. The downloaded
Windows x64 package passed six runtime corpora: 238 recorded commands / 3,603 assertions. That total
consists of 196 SuniPDF CLI, 40 qpdf and two Python/PowerShell validator invocations; it excludes
subprocesses internal to those two validators. The package remained unchanged and Latest was
rechecked before and after execution. Draft and public passes cover the same corpora; neither they
nor the development-corpus totals are added together.

## Native and performance limits

Native interaction with the new 0.67 plan-link, revision-outline and zone workflows has **not been
tested**: the local application-control channel was unavailable. Earlier 0.65 plan-link and 0.66
revision-outline development checks are historical results for their own binaries; they do not
qualify these workflows in 0.67. Offscreen UI tests, CLI outputs, source inspection and Ubuntu
container reader smoke tests do not replace this missing interactive check.

A paired development-build benchmark compared 0.64 with 0.67 on six whole-CLI-process workloads,
with one warmup and five measured repetitions per case. Median times increased by 0.5% for document
information, 5.0% for dense-page rendering, 2.6% for search, 4.0% for the 1,500-row register and 3.4%
for 300 quantities with eight formulas. Optimization/output decreased by 0.8%. These bounded
measurements establish no overall speed improvement, GUI responsiveness result or release-package
performance claim. Rendering and table costs need further profiling without weakening source checks.

## Capability tracking

- [Plan links](PLAN_LINKS.md): one indexed source PDF, up to 32 selected pages and 512 exact searchable
  references, explicit destination review, guarded relative paths and new-copy output. AEC-017 remains
  In progress for broader batch processing and remaining qualification.
- [Revision outlines](REVISION_OUTLINES.md): one explicit old/new page pair, manual or two-point
  alignment, classified raster proposals and individually reviewed editable outlines. Uncertain,
  noise-only, filtered and unmapped regions remain explicit. AEC-014 remains In progress.
- [Rooms and zones](PLAN_ZONES.md): up to 64 rectangular zones per page, eight hierarchy levels,
  full-bounds annotation assignment and shared quantity/register coverage. No inferred room areas or
  measurement recalibration. AEC-010 remains In progress.
- AEC-015 remains Done for indexed plan-set navigation; AEC-009 remains Done for polygon deductions
  and constant physical depth. AEC-016 retains its broader batch-transfer criteria.

The [GitHub project](https://github.com/users/lmdanavanem/projects/1), [backlog](AEC_BACKLOG.md) and
[roadmap](ROADMAP.md) retain complete acceptance criteria separately from implemented increments.

## Historical 0.64 qualification

[0.64 release evidence](qa/RELEASE_0.64.0.json) records its own source, four workflows and eight verified
public assets. Windows recorded 994 passed / 15 ignored; Linux recorded 974 passed / 10 ignored per run.
The downloaded Windows CLI passed 82 processes / 1,062 assertions. Portable upgrades covered 44
baselines plus four guards; MSI covered 42 baselines / 210 logs. Its separate native development-build
plan navigation and transfer checks, including 27 independent saved-output assertions, remain scoped
to that release. None of those historical counts is a result for 0.67.

[0.63](RELEASE_0.63.0.md), [0.62](RELEASE_0.62.0.md), [0.61](RELEASE_0.61.0.md),
[0.60](RELEASE_0.60.0.md) and [earlier status history](archive/PROJECT_STATUS_PRE_0.60.md) remain available.

## Remaining gates

The three related backlog cards remain **In progress**. AEC-017 retains broader multi-source batch
scope; AEC-014 and AEC-010 retain the remaining native and interoperability qualification. No complete
P1/P2 category is closed. Packages remain unsigned. ARM64 execution, physical printers, human
accessibility and private project plans remain unqualified. Container/software-rendering checks do not
establish physical desktop/GPU coverage. Synthetic and licensed corpus results do not establish
universal interoperability, survey accuracy or PDF/A/PDF/UA compliance. Ignored tests are not passed
checks. English is the default interface; detailed engine refusals may remain English in French mode.

See [known issues](KNOWN_ISSUES.md) and [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md).
