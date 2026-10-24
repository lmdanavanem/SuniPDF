# Project status

**0.63.0 is published as stable Latest, with source, package and public-download verification complete.**
Updated 2026-10-24. [Published downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.63.0) ·
[0.63 evidence](qa/RELEASE_0.63.0.json) · [0.63 scope](RELEASE_0.63.0.md).

## 0.63 qualification

The release source was merged as `f35c2c7930546b9a2bc1e24a886ca0c19621941f` through
[PR #39](https://github.com/lmdanavanem/SuniPDF/pull/39). Its tree matches the locally qualified source.
Release ID `422499992` was published at 04:28:05 UTC on 2026-10-24; anonymous Latest checks passed before and after public-package execution.

| Check | Recorded result |
|---|---|
| Final local Windows workspace | 963 passed, 0 failed, 15 ignored |
| Formatting, workspace Clippy and desktop/CLI build | Passed |
| Whole-process CLI corpus | 20 commands / 680 assertions / 0 failures |
| Native Windows plan set | Manifest open, reference statuses and existing clean-tab navigation passed |
| Native Windows volume | Deduction vertex insertion, Preview/Apply, Undo/Redo and save/reopen passed |
| Native Windows FDF | Repeated imports, clear, save/reopen and FDF export passed |
| Independent native-volume output | 65 assertions passed; source, geometry, depth, appearance and quantity retained |
| Independent native-FDF output | 8 commands / 129 assertions passed; 13 protected files unchanged |
| Main CI / Linux / CodeQL / release workflows | All four passed on the release source |
| Windows / Linux source tests | Windows 963 passed / 15 ignored; Linux 943 passed / 10 ignored per run; zero failures |
| Translation catalogues | 1,619 matching keys |
| Portable updates | 43 baselines plus 4 guards: 47 scenarios passed |
| MSI lifecycle | 41 baselines / 205 logs passed |
| Ubuntu package | 22.04 and 24.04 clean-container installation, CLI, OCR, reader, remove/reinstall checks passed |
| Public downloads | Eight assets and checksum sidecars verified anonymously; stable Latest verified |
| Actual downloaded Windows CLI | 68 commands / 968 assertions, zero failures; package and helpers unchanged |
| DEB payload identity | Embedded CLI matched both conformance runs; no execution performed by this supplemental check |
| Reader UI CI | Five actual application/PDFium scenes; offscreen DX12 WARP, not physical GPU interaction |
| Security scanning | Zero open alerts after individual fixture/false-positive review |

The final local source fingerprint was
`74e6ad2caf2cf54172d0ab48ee6201e092f0c5bcf15c3bdb4550901a6d874964`.
The CLI SHA256 was `750e713e2bf81c52cfadc22ce2498e6a466bafce7cda3efac8b8f44c72bf5583`;
the desktop SHA256 was `ad827ad2f2180948e5c8d549c3fa34d15690e0d7300a657a7e109b87d078b235`.
These hashes identify the local development builds. The separately downloaded CLI has SHA256
`cf7b8a03da2fecfca99388065d835c6760178dcedb849b07fc5ee8ee3574e658`. Its runtime check did not
launch the distributed GUI or install the package locally; installer evidence is from CI. Ignored tests are not passed checks;
separate runs and earlier failed attempts are not summed.

## Implemented scope

- [Local plan sets](PLAN_SETS.md): selected PDFs, stable sheet identities, editable metadata, confined
  relative paths, source fingerprints and guarded navigation. Refresh/relink and manifest copies preserve
  source PDFs and existing tab edits. Grouped/searchable navigation remains open in AEC-015.
- [FDF exchange](FORM_DATA.md): typed button states, Unicode values, explicit empty-choice/clear behavior,
  bounded expansion and transactional appearance refresh, including repeated imports.
- [Deducted areas and volumes](MEASUREMENT_VOLUMES.md): separate contours, physical depth,
  gross/deducted/net results, cubic prices and formulas, source provenance and atomic validation.
  AEC-009 is Done for its stated polygon deductions and constant-depth calculation criteria. Editable external-reader interoperability
  and physical measurement accuracy remain separate, unqualified scopes.

The native volume run validated vertex insertion; it did not establish handle dragging. The native form
panel lacked one tested CJK glyph, while independent inspection confirmed the exact stored Unicode value.
The full [release notes](RELEASE_0.63.0.md) retain behavior and limits.

## Previous release evidence

[0.62.0](RELEASE_0.62.0.md) was published from `a6281fd8d54e4bceddc53d43e9527b1cdaa3872e`.
Its [evidence](qa/RELEASE_0.62.0.json) retains exact-source CI, eight verified public downloads, Windows
x64 CLI qualification and the later DEB payload identity check. Those results belong to 0.62.0.
Earlier local benchmarks found a faster 300-row/eight-formula case and a 23 ms regression in the
optimization/output case; the cause remains unisolated. They do not establish release-build or UI performance.
[0.61 evidence](qa/RELEASE_0.61.0.json), [0.60 evidence](qa/RELEASE_0.60.0.json) and
[earlier status history](archive/PROJECT_STATUS_PRE_0.60.md) remain available.

## Project tracking and remaining gates

[GitHub Projects](https://github.com/users/lmdanavanem/projects/1) was synchronized after 0.62 publication
on 2026-10-24: five existing P1 cards updated and 27 AEC cards added without duplication. The preserved
views contained 51 Roadmap, 8 Bugs and 11 Suggestions cards. This is the dated tracking baseline;
subsequent publication updates mark AEC-009 Done for its stated scope and keep AEC-015, AEC-016 and
AEC-017 in progress for later 0.64/0.65 work. The 27 AEC cards comprise 15 In progress, 10 Planned and
2 Done. Those candidates
are not part of 0.63, and partial evidence does not close broader P1 cards.

Private real-project plans remain deferred. Independent update signing, production code signing,
physical printing, native ARM64 execution, human accessibility and broader reader/conformance checks
remain open. Windows packages are unsigned; Ubuntu CI uses containers and software rendering.
There is no universal PDF/A, PDF/UA, survey-accuracy or physical-device claim.
See the [roadmap](ROADMAP.md), [capability backlog](AEC_BACKLOG.md),
[known limits](KNOWN_ISSUES.md) and [1.0 gates](RELEASE_1.0_CRITERIA.md).
