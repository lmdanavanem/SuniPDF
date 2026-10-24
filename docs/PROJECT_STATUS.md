# Project status

**0.61.0 is published as stable Latest**, verified on 2026-10-23.
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.61.0) · [Release notes](RELEASE_0.61.0.md) · [Machine-readable evidence](qa/RELEASE_0.61.0.json).

Source `11cd879a4379d3cb757b0255dabc951087b42264`, merged through [PR #35](https://github.com/lmdanavanem/SuniPDF/pull/35), tag `v0.61.0`.
The published qualification below applies to that source and its exact packages.
The 0.62.0 candidate has separate local results and awaits release qualification.

## Unreleased working branch

The [0.62.0 candidate](RELEASE_0.62.0.md) has passed local Windows x64 development qualification
but is **not release-qualified or published**. Implemented changes include:

- Dimensioned rectangles use an explicit displayed-page anchor, real width/height and local, page, region or automatic calibration. Preview/Apply, one Undo step, save/reopen and CLI operations share geometry and storage-precision guards.
- Formula subtotals reuse the evaluated row cells, sum their rounded results and separate formula/group/unit/precision buckets. Coverage, error categories and missing totals stay explicit.
- Quantity exports retain version 1 coverage semantics, add additive/angular/excluded counters and preserve signed numeric CSV cells while protecting free-text cells.
- Quantity-table construction reuses one permissions read and avoids copying all page annotation entries for every row. A bounded same-machine development-build comparison is recorded below; export and edit permission checks remain explicit.
- Saved measurement provenance distinguishes current, legacy-unverified and changed-page records. Historical values remain inspectable but cannot supply reusable scales, active quantities, costs or quantity-dependent formulas. Explicit current-reference recalibration is required for lengths and areas; historical angles must be recreated.
- Page references now include initial layer visibility and the page transparency group as well as bounds, user unit, decoded content and reachable resources. Earlier saved defaults and named-region references require explicit recalibration or removal; metadata edits alone cannot promote them.
- Measurement edits and batch recalibration reject incompatible preserved price dimensions or pricing overflow before committing. Compatible unit conversions retain the explicit price basis; a failed batch leaves all its records unchanged.
- CLI output creation refuses a destination created during processing unless that command explicitly permits replacement. Document restoration preserves the live document and credential on failure; encrypted certificate signing also refuses automatically unlocked inputs.
- Failed tab resumes retain unsaved bytes, history and recovery identity for retry. Register reloads, search generations, measurement invalidation and page/navigation state follow edits completed during suspension.

Local qualification of the implementation preceding the release-version change is recorded separately:

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

The exact 0.62.0 source must still pass CI, platform packaging, upgrade checks and public-download verification.
Earlier release results do not qualify this candidate. Measurement page binding does not establish physical
accuracy, authenticate every drawing revision or replace independent reader qualification.

## Published 0.61.0 scope

- Custom-field sorting in the register and typed sorting in quantity tables, with stable ties and missing values last.
- Capture of supported saved native shapes into source-attributed, reusable annotation groups.
- Bounded report formulas, explicit unit/error handling and reusable configuration with source-bound CSV/JSON.
- Independent conformance/reader checks and preservation of the existing compression state of XML metadata during optimization.

## Published 0.61.0 qualification

| Check | Result |
|---|---|
| Windows workspace | 844 passed, 0 failed, 14 ignored |
| Linux main workspace | 825 passed, 0 failed, 9 ignored |
| Linux package-build workspace | 825 passed, 0 failed, 9 ignored |
| Interface translations | 1492 matching English/French keys |
| Portable updater | 45 scenarios: 41 upgrades and 4 guards |
| MSI lifecycle | 39 baselines / 195 verified logs |
| Conformance corpus | 12 fixtures / 42 stages in each main and release run |
| Public downloads | Eight anonymous downloads verified against qualified SHA256 values |
| Code Scanning | No open alerts at qualification |

Exact-source workflows: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40023757049) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40023757101) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40023771482) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40023756761). Ignored tests are not passed qualification.
The JSON evidence retains actual UI/performance records and Ubuntu 22.04/24.04 package checks.
UI uses offscreen DX12 WARP on Windows and Xvfb/software Glow on Ubuntu; same-version Ubuntu
removal/reinstallation does not establish previous-version upgrade behavior.

Automated qualification passed for 12 pinned licensed fixtures / 42 recorded stages per run, PDF/A-1b, PDF/A-2b and PDF/UA-1; source preservation, specified rewrites and first-page PDFium/Poppler checks. Positive and expected-negative outcomes are retained in [release evidence](qa/RELEASE_0.61.0.json). This does not establish universal conformance, human accessibility, physical-device support or compatibility with private project plans.

## Remaining work

Qualification remains limited to the specified synthetic/licensed inputs and operations. Private real-project
plans remain deferred. There is no universal PDF/A, PDF/UA, accessibility or survey-accuracy claim. The 0.61.0
release performance evidence covers cached model operations; the candidate adds bounded whole-command and
native action/capture observations, without complete frame-latency qualification. Windows remains unsigned;
ARM64 has no native runtime qualification. Ubuntu uses Xvfb/software Glow and same-version reinstall tests.
Physical printing, broader native interaction, human accessibility and independent update authenticity remain
open. Projects synchronization remains deferred.

Release packaging of the locally tested candidate, unit-bearing fields, broader annotation capture
and geometric area/volume extensions remain in the [roadmap](ROADMAP.md). Whole backlog cards and [1.0 gates](RELEASE_1.0_CRITERIA.md)
stay open until their full acceptance criteria are met. See [known limits](KNOWN_ISSUES.md).

[0.60 evidence](qa/RELEASE_0.60.0.json) and [earlier project history](archive/PROJECT_STATUS_PRE_0.60.md)
are preserved; their old status statements are historical.
