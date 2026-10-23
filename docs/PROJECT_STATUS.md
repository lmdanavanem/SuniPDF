# Project status

**0.61.0 is published as stable Latest**, verified on 2026-10-23.
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.61.0) · [Release notes](RELEASE_0.61.0.md) · [Machine-readable evidence](qa/RELEASE_0.61.0.json).

Source `11cd879a4379d3cb757b0255dabc951087b42264`, merged through [PR #35](https://github.com/lmdanavanem/SuniPDF/pull/35), tag `v0.61.0`.
These results apply to that source and its exact packages, not later documentation edits.

## Delivered scope

- Custom-field sorting in the register and typed sorting in quantity tables, with stable ties and missing values last.
- Capture of supported saved native shapes into source-attributed, reusable annotation groups.
- Bounded report formulas, explicit unit/error handling and reusable configuration with source-bound CSV/JSON.
- Independent conformance/reader checks and preservation of the existing compression state of XML metadata during optimization.

## Observed qualification

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

Bounded synthetic/licensed corpus and specified operations only. Private real-project plans remain deferred. No universal PDF/A, PDF/UA, accessibility or survey-accuracy claim. Performance records cover cached model operations in debug CI binaries, not complete frame latency or physical devices. Windows remains unsigned; ARM64 has no native runtime qualification. Ubuntu uses Xvfb/software Glow and same-version reinstall tests. Physical printing, native interaction, human accessibility and independent update authenticity remain open. Projects synchronization remains deferred.

Formula subtotals, unit-bearing fields, broader annotation capture and geometric area/volume tools
remain in the [roadmap](ROADMAP.md). Whole backlog cards and [1.0 gates](RELEASE_1.0_CRITERIA.md)
stay open until their full acceptance criteria are met. See [known limits](KNOWN_ISSUES.md).

[0.60 evidence](qa/RELEASE_0.60.0.json) and [earlier project history](archive/PROJECT_STATUS_PRE_0.60.md)
are preserved; their old status statements are historical.
