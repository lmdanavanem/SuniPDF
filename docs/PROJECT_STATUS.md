# Project status

## 0.70.0 status

Publication: published stable, verified 2026-10-24T10:47:52.733228Z. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.70.0) · [Release scope](RELEASE_0.70.0.md) · [Qualification evidence](qa/RELEASE_0.70.0.json).

Declare optional units on Number custom fields and retain them in the table, formula previews,
PDF/CSV/JSON reports and XLSX refreshes. Columns with the same name require one type and unit.
Changing a unit changes its declaration without converting the value or calibrating a measurement.

Checked decimal formulas combine lengths, areas and volumes within one exact unit family:
`2 m * 3 m = 6 m2`, then `6 m2 * 0.25 m = 1.5 m3`. Mixed families, inverse dimensions,
powers above three, mass and density remain unsupported. Missing values do not become zero.

Version 2 XLSX Changes sheets expose typed **Before custom fields** and **After custom fields**
as inert text. Absent comparison sides stay empty. Unitless version 1 data keeps its serialization,
comparison hashes and original workbook columns. Unit-bearing data uses version 2; unsupported
versions are refused rather than treated as dimensionless data.

## Qualification scopes

The local development source passed formatting, strict workspace Clippy and desktop/CLI compilation, with **1,094 tests passed, zero failed and 15 ignored**. The English and French catalogs contain **1,871 matching keys**. The independent units corpus passed **1,992 assertions across 96 commands** (71 development CLI 0.70 commands, four packaged CLI 0.69 compatibility commands and 21 qpdf checks). These results are scoped to the recorded development binaries; they do not qualify a downloaded 0.70 release package.

| Evidence scope | Verified result |
|---|---|
| Publication | 2026-10-24T10:44:17Z, release 422651863, published stable / Latest verified |
| Immutable release source | `3909bfaab7ad0ce1e1b287040187eadf742e3657`, tree `656860a197516ecd1ba0cb5387b5972e091f7d1a`, [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408084), [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408057), [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084420845), [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408173) |
| Exact-source Windows CI | 1,094 passed, zero failed, 15 ignored; [workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408084) |
| Exact-source Linux CI | 1,075 passed, zero failed, 10 ignored; [workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408057) |
| Code scanning | [CodeQL passed](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408173); zero open alerts at verification |
| Public assets | Eight files: four packages and four SHA256 sidecars. Anonymous downloads, package hashes and matching sidecars verified; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Downloaded-package corpora | 7,019 assertions across 422 recorded commands in nine corpora. Tool identities and distinct current/legacy reader scopes are recorded in [qualification evidence](qa/RELEASE_0.70.0.json); repeated draft/public runs are not additive coverage |
| Portable/MSI qualification | 47 portable upgrades and four guards passed; [qualification evidence](qa/RELEASE_0.70.0.json), 45 MSI baselines passed with 225 installation logs; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Ubuntu installation | Ubuntu 22.04 and 24.04 installation checks passed in clean containers with software rendering; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Rendered-artifact review | Passed for the specified rendered pages from the exact package. Artifact inspection only; no native desktop interaction. See [qualification evidence](qa/RELEASE_0.70.0.json) |
| Native desktop and spreadsheet interaction | Not tested in the current session; no interactive qualification claim |

Upgrade qualification passed for **47 portable baselines plus four guards** and **45 MSI baselines**,
including published 0.69.0 packages. Repeated runs are not additive coverage.

## Open capabilities and release gates

AEC-007, AEC-018 and AEC-019 remain **In progress**. Native desktop and spreadsheet interaction
and broader reader qualification remain open. Editable report templates, arbitrary XLSX import,
external spreadsheet edit preservation and automatic synchronization are outside these increments.

The 0.67 plan-link, revision-outline and zone workflows also retain their native and interoperability
limits: AEC-017, AEC-014 and AEC-010 remain In progress. AEC-016 retains broader batch-transfer work.
AEC-009 and AEC-015 remain Done only for their stated constant-depth/deduction and indexed-plan-set
criteria. See the [backlog](AEC_BACKLOG.md) and [roadmap](ROADMAP.md) for the complete criteria.

No entire P1/P2 category is closed. Packages are unsigned; ARM64 hardware execution, physical
printing, human accessibility and private project plans retain their separate open or deferred gates.
Automated UI, PDF/OOXML and container checks do not replace native interaction or physical-device
validation. No universal interoperability, survey accuracy or PDF/A/PDF/UA conformance is claimed;
ignored tests remain ignored. See [known issues](KNOWN_ISSUES.md) and [1.0 criteria](RELEASE_1.0_CRITERIA.md).

GitHub Projects requires a fresh board readback before its synchronization can be claimed. This
page records no current synchronization. English is the default UI; detailed engine refusals may
remain English in French mode.

## Performance evidence

0.70 package benchmark: A paired comparison of the packaged 0.69/0.70 Windows CLIs retained identical semantic results. Median process times were 51.5 to 51.4 ms for document information, 80.6 to 78.9 ms for search, 352.1 to 352.2 ms for the 1,500-row register, and 113.6 to 108.4 ms for 300 rows with eight formulas. These four synthetic workloads do not establish overall or GUI performance. See [qualification evidence](qa/RELEASE_0.70.0.json).

A paired development-build benchmark compared 0.64 with 0.67 on six whole-CLI-process workloads,
with one warmup and five measured repetitions per case. Median times increased by 0.5% for document
information, 5.0% for dense-page rendering, 2.6% for search, 4.0% for the 1,500-row register and 3.4%
for 300 quantities with eight formulas. Optimization/output decreased by 0.8%. These bounded
measurements establish no overall speed improvement, GUI responsiveness result or release-package
performance claim. Rendering and table costs need further profiling without weakening source checks.

## Historical release evidence

The [0.69 release notes](RELEASE_0.69.0.md) and [0.69 qualification record](qa/RELEASE_0.69.0.json)
retain its publication, exact-source CI, packages, corpora and native-interaction limits.
[0.67](RELEASE_0.67.0.md), [0.64](RELEASE_0.64.0.md), [0.63](RELEASE_0.63.0.md),
[0.62](RELEASE_0.62.0.md), [0.61](RELEASE_0.61.0.md) and the
[earlier status archive](archive/PROJECT_STATUS_PRE_0.60.md) retain their own evidence.
Historical counts and native checks do not qualify 0.70 and are not added to its totals.
