# Project status

**0.62.0 is published as stable Latest**, verified on 2026-10-24.
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.62.0) · [Release notes](RELEASE_0.62.0.md) · [Machine-readable evidence](qa/RELEASE_0.62.0.json).

Source `a6281fd8d54e4bceddc53d43e9527b1cdaa3872e`, tag `v0.62.0`.
The published qualification below applies to that source and its exact packages.

## 0.63 candidate implementation

- Local plan-set manifests index selected PDFs with stable sheet IDs, physical page references, editable
  metadata and SHA256/length pins. Desktop and CLI operations expose unavailable or changed references,
  save new manifest copies with confined rebasing, and leave the source PDFs unchanged.
- Sheet navigation checks verified bytes. A matching clean tab can be reused; unsaved tabs require an
  explicit choice, with different page counts refusing an indexed jump. One-operation passwords remain
  in memory and never enter the manifest. Revision equivalence and automatic cross-document links are
  separate work. [Plan-set workflow](PLAN_SETS.md).
- FDF form export covers supported text, checkbox, radio and choice values. Import retains the distinction
  between button appearance-state names and text values, with shared serialization/expansion bounds and
  destination-field validation. [Value exchange and exclusions](FORM_DATA.md).
- Deducted areas and constant-depth volumes retain separate outer/interior contours and an explicit
  physical depth. Desktop selection, point editing, preview and history preserve the full geometry;
  the CLI writes guarded new copies. Gross/deducted/net results, cubic quantity units and compatible
  unit prices preserve physical dimensions. Historical quantities remain inactive.
- Automatic scale selection uses the outer boundary, and stored geometry is checked after coordinate
  conversion. Invalid topology, ambiguous scale regions and incompatible prices refuse the operation.
  [Calculation and geometry limits](MEASUREMENT_VOLUMES.md).

AEC-009 and AEC-015 are **In progress**. Final local candidate checks passed on 2026-10-24:
**963 workspace tests passed, 0 failed and 15 ignored**; formatting, workspace Clippy and the desktop/CLI
build passed. A whole-process CLI corpus passed **20 commands and 680 assertions**, with its binary hash
unchanged. The source fingerprint remained
`74e6ad2caf2cf54172d0ab48ee6201e092f0c5bcf15c3bdb4550901a6d874964` throughout the final pipeline.
Final native workflows are still being checked; exact-source CI, package/install/update checks and publication
remain pending. These local results are separate from the published 0.62 evidence below. See the
[candidate qualification and limits](RELEASE_0.63.0.md).

## Published qualification

| Check | Result |
|---|---|
| Windows workspace | 904 passed, 0 failed, 15 ignored |
| Linux main workspace | 885 passed, 0 failed, 10 ignored |
| Linux package-build workspace | 885 passed, 0 failed, 10 ignored |
| Interface translations | 1536 matching English/French keys |
| Portable updater | 46 scenarios: 42 upgrades and 4 guards |
| MSI lifecycle in CI | 40 baselines / 200 verified logs |
| Conformance corpus | 12 fixtures / 42 stages in each main and release run |
| Public downloads | Eight anonymous downloads verified against qualified SHA256 values and sidecars |
| Published Windows x64 CLI | 48 process commands / 288 assertions / 0 failures |
| Code Scanning | No open alerts at qualification |

Exact-source workflows: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40051795382) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40051795422) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40051802122) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40051795764).
Ignored tests are not passed qualification. Separate runs are not combined into a larger total.

The published Windows x64 portable package was executed with its adjacent runtimes and no development-runtime
overrides. The run covered historical and changed-page exclusions, explicit recalibration, retained prices and
custom fields, signed numeric CSV, save/reopen, independent PDF metadata, qpdf, rendering, license inventory
and native-library loading. All assertions passed, the package stayed unchanged, and anonymous Latest checks
identified 0.62.0 before and after. No GUI launch, installation or local update was performed in this run;
MSI lifecycle and upgrade results above come from CI.

Windows CI UI checks use offscreen DX12 WARP. Ubuntu 22.04/24.04 checks install and run the package as a normal
user in clean containers, with Xvfb/software Glow and same-version removal/reinstallation. These checks do
not establish physical GPU or previous-version Ubuntu upgrade behavior. The JSON retains actual UI,
performance and package-validation records.

The conformance corpus covers PDF/A-1b, PDF/A-2b and PDF/UA-1, source preservation, specified rewrites and
first-page PDFium/Poppler checks, with positive and expected-negative outcomes. A later DEB payload check
on 2026-10-24 independently matched the embedded SuniPDF CLI SHA256 to both recorded conformance runs.
The package remained unchanged. This in-memory inspection did not execute Linux programs or add runtime
or desktop qualification; the desktop hash was observed only. Historical run records remain unchanged
and the supplementary result is retained in the release evidence. This is bounded evidence, not universal
conformance, human accessibility or private-plan qualification.

## Delivered in 0.62.0

Published changes include:

- Dimensioned rectangles use an explicit displayed-page anchor, real width/height and local, page, region or automatic calibration. Preview/Apply, one Undo step, save/reopen and CLI operations share geometry and storage-precision guards.
- Formula subtotals reuse the evaluated row cells, sum their rounded results and separate formula/group/unit/precision buckets. Coverage, error categories and missing totals stay explicit.
- Quantity exports retain version 1 coverage semantics, add additive/angular/excluded counters and preserve signed numeric CSV cells while protecting free-text cells.
- Quantity-table construction reuses one permissions read and avoids copying all page annotation entries for every row. A bounded same-machine development-build comparison is recorded below; export and edit permission checks remain explicit.
- Saved measurement provenance distinguishes current, legacy-unverified and changed-page records. Historical values remain inspectable but cannot supply reusable scales, active quantities, costs or quantity-dependent formulas. Explicit current-reference recalibration is required for lengths and areas; historical angles must be recreated.
- Page references now include initial layer visibility and the page transparency group as well as bounds, user unit, decoded content and reachable resources. Earlier saved defaults and named-region references require explicit recalibration or removal; metadata edits alone cannot promote them.
- Measurement edits and batch recalibration reject incompatible preserved price dimensions or pricing overflow before committing. Compatible unit conversions retain the explicit price basis; a failed batch leaves all its records unchanged.
- CLI output creation refuses a destination created during processing unless that command explicitly permits replacement. Document restoration preserves the live document and credential on failure; encrypted certificate signing also refuses automatically unlocked inputs.
- Failed tab resumes retain unsaved bytes, history and recovery identity for retry. Register reloads, search generations, measurement invalidation and page/navigation state follow edits completed during suspension.

## Earlier local development qualification

Qualification of the implementation preceding the release-version change is recorded separately:

| Local check | Recorded result |
|---|---|
| Windows workspace | 904 passed, 0 failed, 15 ignored |
| Independent corpus runs | Three explicitly executed tests passed before the final visual-only changes |
| Format, Clippy and documentation | Passed |
| Interface translations | 1536 matching English/French keys |
| Final CLI qualification | 44 commands / 276 assertions |
| Independent output and source checks | 19 passed |
| Native Windows | Rectangle creation, Preview/Apply, Undo/Redo, save/reopen and historical-price recalibration exercised |
| Final visual checks | English/French, light/dark controls and formatted quantity values checked |

The quantity-table benchmark used 300 annotations and eight constant formulas, five samples after warmup,
and matching local development profiles: median complete-command time changed from 5.593 s to 0.279 s.
The optimization-and-output case regressed from 0.053 s to 0.076 s; its cause remains unisolated.
These observations do not qualify optimized release packages or establish general UI smoothness.

Exact-source CI, platform packaging, upgrade checks and public-download verification are recorded above.
The earlier local measurements do not qualify release-build performance. Measurement page binding does not establish physical
accuracy, authenticate every drawing revision or replace independent reader qualification.

## Previous release

- Custom-field sorting in the register and typed sorting in quantity tables, with stable ties and missing values last.
- Capture of supported saved native shapes into source-attributed, reusable annotation groups.
- Bounded report formulas, explicit unit/error handling and reusable configuration with source-bound CSV/JSON.
- Independent conformance/reader checks and preservation of the existing compression state of XML metadata during optimization.

The complete [0.61.0 evidence](qa/RELEASE_0.61.0.json) remains available as a historical record.

## Project tracking

[GitHub Projects](https://github.com/users/lmdanavanem/projects/1) was synchronized after publication on
2026-10-24: five existing P1 cards updated and 27 AEC cards added without duplication. Roadmap contains
51 cards, Bugs 8 and Suggestions 11. Existing bug/suggestion statuses and saved views were preserved.
Partial capabilities remain open; candidate 0.63 work is not marked as a published release.

## Remaining work

Qualification remains limited to the specified synthetic/licensed inputs and operations. Private real-project
plans remain deferred. There is no universal PDF/A, PDF/UA, accessibility or survey-accuracy claim. The 0.62.0
CI performance evidence covers cached model operations; earlier local development checks add bounded
whole-command and native action/capture observations, without complete frame-latency qualification. Windows remains unsigned;
ARM64 has no native runtime qualification. Ubuntu uses Xvfb/software Glow and same-version reinstall tests.
Physical printing, broader native interaction, human accessibility and independent update authenticity remain
open. The repository backlog records the acceptance criteria for each remaining item.

Unit-bearing fields, broader annotation capture and geometric area/volume extensions remain in the
[roadmap](ROADMAP.md). Whole backlog cards and [1.0 gates](RELEASE_1.0_CRITERIA.md)
stay open until their full acceptance criteria are met. See [known limits](KNOWN_ISSUES.md).

[0.60 evidence](qa/RELEASE_0.60.0.json) and [earlier project history](archive/PROJECT_STATUS_PRE_0.60.md)
are preserved; their old status statements are historical.
