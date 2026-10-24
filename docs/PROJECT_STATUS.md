# Project status

**SuniPDF 0.69.0 is published as the stable Latest release.**
Updated 2026-10-24. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.69.0) ·
[Release scope](RELEASE_0.69.0.md) · [Qualification evidence](qa/RELEASE_0.69.0.json).

## Current scope: reports and workbooks

0.69 combines the report and workbook increments. The intermediate 0.68 development build is not
described as a separate published release.

- Reports preserve the exact register or quantity-table selection across PDF, CSV and JSON. The PDF
  starts with readable quantities, separate cost totals and explicit coverage; legends and annotation
  details follow, with full source identities and settings in a technical appendix.
- XLSX snapshots contain structured rows, totals, changes and provenance. A separate refresh manifest
  supports reviewed comparisons through unique annotation identities. Missing and ambiguous entries
  remain explicit; current quantities are recalculated from the PDF.
- Outputs are new files. Copy permissions, source identity, cancellation and existing-file guards apply.
  A derived report or workbook does not inherit the source PDF's encryption or digital signatures.

See [issue reports](ISSUE_REPORTS.md) and [quantity workbooks](QUANTITY_WORKBOOKS.md) for supported
inputs, bounds and exclusions. These exports do not establish PDF/A or PDF/UA conformance.

## Release qualification

These results identify the exact release source and downloaded packages. Development, CI,
package checks and native interaction are recorded separately; repeated corpora are not added together.

| Evidence scope | Verified result |
|---|---|
| Frozen source and merge | [PR #45](https://github.com/lmdanavanem/SuniPDF/pull/45), source `d09621b54d2ceb0addaa07e4b7115db009ef7dd7`, tree `1cd7ecc368d2ae0cba6e55702dde256de9bcbd5d` |
| Exact-source Windows/Linux CI | [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40079860771): 1,083 passed, 15 ignored; [Linux CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40079860777): 1,064 passed, 10 ignored; zero failures |
| Formatting, Clippy and localization | Formatting and strict workspace Clippy passed; 1,868 localization keys matched. [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/40079860721) passed; zero open scanning alerts at verification |
| Independent PDF/OOXML checks | Eight corpora exercised the exact package: PDF structure/readers, quantity calculations, revisions, links, outlines, zones, reports and OOXML workbooks. See the versioned tools and hashes in [Qualification evidence](qa/RELEASE_0.69.0.json) |
| Published assets and hashes | Eight anonymous downloads verified against SHA256 and four sidecars; tag, source, stable Latest status and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40079873187) verified |
| Downloaded runtime checks | 5,027 assertions; 279 CLI invocations, 45 qpdf checks and two validators. Draft and public repetitions are separate runs, not additive coverage. Five rendered PDF pages visually reviewed; no native interaction |
| Upgrade and Ubuntu installation checks | 46 portable upgrades plus four guards; 44 MSI baselines with 220 logs; Ubuntu 22.04 and 24.04 installation checks passed |
| Native desktop and spreadsheet interaction | Not tested; no interactive qualification claim |

The release evidence records completed checks and retained limitations. Earlier development
results are not counted as qualification of the changed report layout.

## Native and interoperability limits

Native interaction with the 0.69 report and workbook workflows has **not been tested**. Native
spreadsheet interoperability also remains untested. PDF text/raster inspection, automated OOXML
validation and offscreen UI tests do not replace those interactions or establish universal reader
compatibility. Complex text shaping and editable report templates are outside the report workflow;
arbitrary XLSX import, external spreadsheet edit preservation and automatic synchronization are outside
the workbook refresh workflow.

Native interaction with the new 0.67 plan-link, revision-outline and zone workflows has **not been
tested**: the local application-control channel was unavailable. Earlier 0.65 plan-link and 0.66
revision-outline development checks are historical results for their own binaries; they do not
qualify these workflows in 0.67. Offscreen UI tests, CLI outputs, source inspection and Ubuntu
container reader smoke tests do not replace this missing interactive check.

## Capability tracking

- AEC-018 remains **In progress**. The report increment is implemented, while native interaction and
  remaining report interoperability qualification stay open. Editable report templates are a separate
  extension; this release does not claim them.
- AEC-019 remains **In progress**. XLSX snapshots and manifest-based refresh are implemented, while
  native desktop/spreadsheet interaction and remaining interoperability qualification stay open.
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

The [backlog](AEC_BACKLOG.md) and [roadmap](ROADMAP.md) retain complete acceptance criteria separately
from implemented increments. GitHub Projects requires its own live readback before claiming a current
synchronization; this page does not supply one.

## Historical performance baseline

A paired development-build benchmark compared 0.64 with 0.67 on six whole-CLI-process workloads,
with one warmup and five measured repetitions per case. Median times increased by 0.5% for document
information, 5.0% for dense-page rendering, 2.6% for search, 4.0% for the 1,500-row register and 3.4%
for 300 quantities with eight formulas. Optimization/output decreased by 0.8%. These bounded
measurements establish no overall speed improvement, GUI responsiveness result or release-package
performance claim. Rendering and table costs need further profiling without weakening source checks.

## Earlier release evidence

[0.67](RELEASE_0.67.0.md), [0.64](RELEASE_0.64.0.md), [0.63](RELEASE_0.63.0.md),
[0.62](RELEASE_0.62.0.md), [0.61](RELEASE_0.61.0.md) and [earlier status history](archive/PROJECT_STATUS_PRE_0.60.md)
retain their own source, package, test and native-validation records. Their counts are not results for 0.69.

## Remaining gates

No complete P1/P2 category is closed. Packages remain unsigned. ARM64 execution, physical printers,
human accessibility and private project plans retain their separate open or deferred qualification.
Container/software-rendering checks do not establish physical desktop/GPU coverage. Synthetic and
licensed corpora do not establish universal interoperability, survey accuracy or PDF/A/PDF/UA compliance.
Ignored tests are not passed checks. English is the default interface; detailed engine refusals may remain
English in French mode. See [known issues](KNOWN_ISSUES.md) and [1.0 criteria](RELEASE_1.0_CRITERIA.md).
