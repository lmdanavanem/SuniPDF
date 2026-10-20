# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

**Free native PDF reader and editor for Windows 10/11. Private local processing, English and French.**

[Français](README.fr.md) · Created by **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Navanem](https://navanem.com/projets/sunipdf/)

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)

## Why SuniPDF

- **Your PDFs stay on your computer.** No account, telemetry or cloud processing. Update checks contact GitHub for release information and can be disabled.
- **Native Windows application.** Rust, egui and PDFium, distributed as a portable ZIP or MSI installer.
- **English by default, French available.** Change the interface language without restarting; existing language preferences are preserved.
- **Clear limits.** Unfinished features stay in the [roadmap](docs/ROADMAP.md).

## 0.38.0: six-zone headers and footers (stable Latest)

[0.38.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) under `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six header/footer zones through Page numbers and the headers-footers CLI command. Absolute page/count placeholders, margins, size and selected pages respect displayed rotation/crop coordinates. Overflow is refused atomically; allowed encryption and copy-only CLI guards remain. Published 0.37.0 packages join migration regression. [Usage and limits](docs/PAGE_DECORATIONS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) passed: **623 tests, zero failures**, 10 default ignored, **1143 EN/FR keys**, **30 portable scenarios**, **24 MSI baselines/120 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **42 cases, 148 CLI calls and 2 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,823,494-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](docs/qa/RELEASE_0.38.0.json).

## 0.37.0: image watermarks (published; Latest at the time)

[0.37.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) under `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Image watermarks in Pages and the CLI: transparency, proportional width, angle, seven positions and margins. Rotated/cropped pages fit on both axes. Atomic selected-page edits, one desktop Undo and copy-only CLI output preserve allowed encryption. Published 0.36.0 packages join upgrade baselines. [Usage and limits](docs/PAGE_DECORATIONS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) passed: **618 tests, zero failures**, 10 default ignored, **1134 EN/FR keys**, **29 portable scenarios**, **23 MSI baselines/115 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **24 cases, 113 CLI calls and 1 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included. Two fixture-password findings were reviewed and dismissed as “used in tests” under `lmdanavanem`; code scanning remains enabled.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,806,746-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](docs/qa/RELEASE_0.37.0.json).

## 0.36.0: configurable keyboard shortcuts (published; Latest at the time)

[0.36.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) under `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Configurable keyboard shortcuts in Help and the palette: filter, assign, conflict refusal, disable and restore. Cancellable drafts, atomic preferences and consistent labels. Text input and reserved keys are preserved; published 0.35.0 packages join upgrade baselines. [Usage and bounds](docs/KEYBOARD_SHORTCUTS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) passed: **613 tests, zero failures**, 10 default ignored, **1128 EN/FR keys**, **28 portable scenarios**, **22 MSI baselines/110 logs**. Seven new headless egui tests cover shortcuts, conflicts, focus, display, cancellation and preference compatibility. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; **20 image cases** and **14 PDF-inspection cases**, plus retained suites, were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,788,530-byte** x64 ZIP without installing it on the workstation.

Native Windows keyboard layouts and Narrator remain unqualified. Explorer remains separate; only commands in the existing shortcut table can be customized. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed. [Evidence](docs/qa/RELEASE_0.36.0.json).

## 0.35.0: visual signature preparation (published; Latest at the time)

[0.35.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) under `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Visual signatures: optional drawn-stroke smoothing, paper-shadow reduction and the `prepare-signature-image` PNG command. Previews bind to prepared pixels; original points and source files are retained, with explicit drawing budgets. Published 0.34.0 packages join the upgrade baselines. [Usage and bounds](docs/VISUAL_SIGNATURE_PREPARATION.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) passed: **606 tests, zero failures**, 10 default ignored, **1114 EN/FR keys**, **20 image cases**, **14 PDF-inspection cases**, **27 portable scenarios**, **21 MSI baselines/105 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, PDF, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; image and inspection suites were also rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,752,605-byte** x64 ZIP without installing it on the workstation.

Visual signatures provide no identity proof; cleanup may remove faint ink and needs preview review. Native pointer/tablet/Narrator interaction and customer photos remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed. [Evidence](docs/qa/RELEASE_0.35.0.json).

## 0.34.0: saved PDF inspection (published; Latest at the time)

[0.34.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) under `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspect the saved PDF from Properties and the `inspect` JSON command: seven declaration categories, source SHA256, explicit partial results, encrypted input and stale-reply refusal. The saved-revision guard hashes disk bytes in chunks. [Usage and bounds](docs/PDF_INSPECTION.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) passed: **600 tests, zero failures**, 10 default ignored, **1108 EN/FR keys**, **14 packaged inspection cases**, **26 portable scenarios**, **20 MSI baselines/100 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent-reader, PDF-corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,741,242-byte** x64 ZIP without installation on the workstation.

Structural inventory without a safety verdict or signature/PDF/A/PDF/UA validation. Packages remain unsigned; native/hardware qualification and 1.0 criteria remain open. Projects edits stay deferred at the owner’s request, with no new synchronization claimed. [Evidence](docs/qa/RELEASE_0.34.0.json).

## 0.33.0: 1.0 preparation and PDF compatibility (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) under `lmdanavanem`, release `417802490`, immutable source `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Preserve imported PDF 2.0 bookmarks containing tabs/line breaks while retaining strict new-title validation. Integrate egui/eframe 0.37 together and fix Back/Forward position after destination zoom changes. Add seven pinned licensed PDF Association examples, 64 bounded PDF/CMS mutations, executable guard controls and command measurements. The public 0.32.0 becomes an additional upgrade baseline. [1.0 criteria](docs/RELEASE_1.0_CRITERIA.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) passed: **592 tests**, zero failures, 10 default ignored, **1092 EN/FR keys**; 84 qpdf corpus checks and 9 independent feature outputs, 4 font cases, DX12 software rendering, 14 Office round trips, four NIST documents/744 pages; **7 PDF 2.0 examples/8 pages/21 qpdf checks**, **64 mutations and 2 guard controls**, **25 portable scenarios**, **19 MSI baselines/95 logs**. The packaged CLI additionally passed the 7-example/21-qpdf suite, 11 object-order scenarios/3 qpdf checks, 11 local-review and 8 security scenarios. CodeQL Rust/Python/Actions passed with zero open alerts.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 test harness, with its unchanged updater module verified by hash, downloaded and verified the **26,726,762-byte x64 ZIP in 1.65 s**, without installation.

Project edits are deferred at the owner’s explicit request. The public readback shows **43 items, 25 Done, 18 Todo**; both P1 tasks have documented partial progress, with no claimed closure or status change.

Windows packages remain unsigned with no independent publication key. Teaching examples and deterministic mutations do not replace a customer/hostile corpus, coverage-guided fuzzing or hardware measurements. ARM64 remains experimental; native interactions, named third-party readers and PDF/A/PDF/UA conformance remain open. [Evidence](docs/qa/RELEASE_0.33.0.json).

## 0.32.0: object ordering (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) under `lmdanavanem`, release `417623635`, immutable source `8c492db35f1712648d5472593146077230befb7c`.

Drag one object or the selected group in the Objects panel, with an insertion line, page-cache/context guards, Escape and one Undo step. New `reorder-objects` JSON command shares the explicit final destination, relative order, permission, atomic-save and password safeguards. [Scope and usage](docs/OBJECT_ORDER.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) passed: **588 tests**, zero failures, 9 default ignored; **1092 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **24 portable scenarios**, **18 MSI baselines/90 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. The packaged CLI passed **11 object-order scenarios and 3 additional qpdf checks**, plus 11 retained local-review and 8 security scenarios.

Six public files and `dist/` copies verified by SHA256. The freshly built harness from this source downloaded and verified the **27,119,935-byte x64 ZIP in 1.75 s**, without installation. Projects: **43 items, 25 Done, 18 Todo, zero In Progress**; the P3 Suggestion is delivered, original criteria preserved, description read back after a server reload and all three saved filters verified.

Windows packages remain unsigned; no independent publication key is available. Hardware, native/manual flows and reader interoperability remain separate qualification; no PDF/A/PDF/UA/LTV certification. [Evidence](docs/qa/RELEASE_0.32.0.json).

## 0.31.0: local comparison and multi-document search (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) under `lmdanavanem`, release `417463135`, immutable source `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Local comparison of two saved PDFs (text, geometry and previews), explicit physical alignment and stale-preview refusal. Literal search across selected files, excerpts/page navigation, progress, cancellation and explicit partial results. New `compare` and `search-files` JSON commands; EN/FR interface. [Scope and limits](docs/LOCAL_REVIEW.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) passed: **582 tests**, zero failures, 9 default ignored; **1091 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **23 portable scenarios**, **17 MSI baselines/85 logs**. CodeQL Rust/Python/Actions passed with zero open alerts after individual review of four synthetic test credentials.

Six public files and `dist/` copies verified by SHA256; eleven local-review and eight security scenarios passed with the packaged CLI. The freshly built harness from this source downloaded and verified the **27,090,504-byte x64 ZIP in 1.71 s**, without installation. Projects: **43 items, 24 Done, 19 Todo, zero In Progress**; both P2 Suggestions are delivered, their criteria retained and all three filters read back.

Windows packages remain unsigned; no independent publication key is available. ARM64 hardware and manual flows remain separate qualification. Bounded comparison does not establish PDF identity or conformance. [Evidence](docs/qa/RELEASE_0.31.0.json).

## 0.30.1: security patch (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) under `lmdanavanem`, release `417149160`, immutable source `697a963c756b316e1eb1e678298886bc08521da1`.

XFA/Screen and accessibility-text cleanup, owner authorization before reprotection, stronger signature verification, environment/stdin PDF passwords and hardened Office/runtime inputs. Unresolved P1 Roadmap items and Suggestions remain deferred. [Details](docs/RELEASE_0.30.1.md).

[Release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) and [main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) passed on this source: **575 tests**, zero failures, 9 default ignored; **1060 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature cases**, **4 font-reader cases**, DX12 software rendering, **14 Office round trips**, 4 licensed PDFs/744 pages, **22 portable scenarios**, **16 MSI baselines/80 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

The six public files and `dist/` copies match the inspected packages and SHA256 digests. Eight security scenarios also pass with the packaged CLI. The production updater verified the **27,029,229-byte** x64 ZIP in **1.21 s**, without installation on this host, using the previously built harness and unchanged updater module.

Updates still depend on GitHub HTTPS and same-channel SHA256: no independent publication key is available. Windows packages remain unsigned; ARM64 hardware and manual flows remain separate qualification. [Evidence and limits](docs/qa/RELEASE_0.30.1.json).

## 0.30.0: editing, document safety and local conversion (published; Latest at the time)

Published (Latest at the time) as [0.30.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) under `lmdanavanem`, release `415568680`, immutable source `4db31c0ee62cfae9d65239edf1eac93db147841e`. The five Bugs and fourteen Roadmap P2 items ship within their [documented supported limits](docs/RELEASE_0.30.0.md). P1 Roadmap items and Suggestions remain deferred.

The [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) and [MAIN CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) passed on this source: **551 tests**, zero failures, 9 default ignored; **1047 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature outputs**, **4 MuPDF font cases**, DX12 software WARP, **14 Office round trips**, 4 licensed PDFs/744 pages, **21 portable scenarios**, **15 MSI baselines/75 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

Six public assets match the inspected packages and SHA256 digests byte for byte. The production updater verified the 26,978,019-byte x64 ZIP in 2.55 seconds without installing on this host, using the previously built test harness with an unchanged updater module. Project readback: **42 unique items, 20 Todo, 0 In Progress, 22 Done**, three saved filters and zero unsaved changes. [Evidence and limits](docs/PROJECT_STATUS.md).

Packages remain unsigned. ARM64 is cross-built and packaged, without execution on ARM64 hardware. Physical printers, Narrator, manual updater interaction and physical GPU/window behavior still need qualification; WARP proves software offscreen rendering only. Unsupported inputs remain refused. PDF/A metadata preservation does not certify PDF/A or PDF/UA conformance.

## 0.21.1: document preservation (published; Latest at the time)

The two P1 fixes ship in [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1): refuse unsupported text without losing the correct
draft, and preserve non-widget annotations without usable appearances when flattening. 476 tests,
84 qpdf checks, 20 portable scenarios and 14 MSI baselines passed; six public files and the live updater
were verified. See [Project status](docs/PROJECT_STATUS.md).

## Features

| Area | Features |
|---|---|
| Read | Tabs, continuous/single-page/two-page/Book views, page-label authoring/reset and physical `#N` navigation, precise link/bookmark destinations, view rotation, full screen, per-tab position/zoom history, tiled rendering, search and logical cross-page selection |
| Annotate | Highlight, underline, strike-out, notes, ink, free text, shapes with solid/dashed borders, links with editable safe destinations, reply threads |
| Edit | Text with standard or embedded system fonts, images, shapes, object selection/move/resize/rotation/opacity, grouped rotation/resizing, clipboard across tabs, Objects panel, mouse page crop and resize |
| Organize | Rotate, delete, insert, reorder, combine, split, extract and create blank pages; extraction/split preserve source encryption, passwords and permissions |
| Fill and sign | AcroForm filling, editable drop-down text, radio `/Opt` export values, retained drafts/retry after read errors, grouped Apply/Discard, keyboard controls, flattening, JSON/XFDF export and JSON/XFDF/value-only FDF import; drawn/typed/imported visual signatures and initials, atomic all/selected-page placement, date |
| Create forms | Named text fields, checkboxes, radio groups, ordinary/editable drop-downs and single/multiple-selection lists through mouse-drawn or numeric page rectangles, including rotated/cropped/protected documents; undo and fill/save |
| Recover | Automatic snapshots every 60 seconds and on tab suspension; original unchanged, PDF encryption preserved, no passwords stored, restore through Save As |
| Print | Native Windows printer selection, page ranges/copies/orientation, current edits, bounded raster output, progress and cancellation |
| Stamp | Text watermarks and page numbering |
| Convert | Images to PDF, PDF to PNG, lossless optimization, lossy image compression to a new copy; bundled offline English/French OCR with invisible searchable text, existing-text skipping, cancellation and undo; sequential multi-file compression/OCR |
| Protect | AES-128/256 encryption and permissions, metadata cleanup, irreversible redaction |
| Certificates | Detached CMS over exact loaded/saved bytes (SHA-256/384/512), Windows trust/revocation and supported RFC3161 results; incremental PFX/P12 signed copy; encrypted PDF signing refused |
| Automate | `sunipdf-cli`: 62 commands with JSON output and stable exit codes, including `set-link-target`, `set-page-labels`, `reset-page-labels`, `compress-images`, `add-form-field`, `ocr`, `signatures` and `sign` |

All 14 task tools have real commands. Reading follows PDFium's logical character order; arbitrary untagged columns
are not reconstructed. Physical printing, ARM64 hardware and third-party signature interoperability remain separate QA checks.

[Feature matrix](docs/FEATURE_MATRIX.md) · [Project status](docs/PROJECT_STATUS.md) · [Changelog](CHANGELOG.md)

## 0.21.0: Code Scanning remediation (published; Latest at the time)

Explicit native signer-certificate null checks and malformed-CMS regression; release root
`contents: read` with `contents: write` restricted to Publish (D112). XML 1.0 readers use `ParserConfig`,
retain bounds and reject DTD events; UTF-8/UTF-16 XMP/XFDF regressions passed. Deterministic fixtures
and complete scanning coverage are preserved.

471 tests, zero failures, seven default ignored; formatting/Clippy, 912 EN/FR keys, 75 qpdf checks,
four-document/744-page corpus and helper passed. CI, 19 portable cases (15 upgrades/four guards), 13 MSI
baselines/65 logs, same-source ARM64, six staged/public files, `dist/` copies and production updater verified.
CodeQL fixed #3 and #119; zero alerts remain open. All 115 criticals were individually dismissed with
evidence by `lmdanavanem`: 114 `used in tests`, #13 `false positive`. No folder or query is excluded.

The [GitHub project](https://github.com/users/lmdanavanem/projects/1) contains 42 prioritised drafts: Bugs (8), Roadmap (23), Suggestions (11),
with saved category filters. At 0.21.0 publication the security item was Done; current implementation
status for the other items is tracked in PROJECT_STATUS.md.
See [Code Scanning review](docs/CODE_SCANNING_REVIEW.md)
and [Project status](docs/PROJECT_STATUS.md). The 53 CLI commands are unchanged.

[Previous stable release](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.0) under `lmdanavanem`, release `415338013`,
source `f006b7dd027fd5a3084c2bfffc2fceaa1b66be46`. Local validation: 471 passed tests, zero failures, seven default ignored;
formatting/Clippy with warnings denied, 912 EN/FR keys, 75 qpdf checks, update-helper self-test and four PDFs/744 pages.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648173) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957647839/job/117164444654) passed:
19 portable cases, 13 MSI baselines and 65 verified logs. Six staged/public files and `dist/` copies match API/SHA256 digests; complete package checks passed.
The production updater verified the 19,846,878-byte Latest x64 ZIP in 1.76 seconds without installing on the host.
[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) fixed #3/#119 on this source; zero alerts remain open after individual triage of 115 criticals.
Detailed evidence and limits are in [Project status](docs/PROJECT_STATUS.md).

Available since **0.15.0**: edit an existing link's destination through **Edit target**, double-click or
`set-link-target`, preserving its area, appearance and comments. Choose a safe web/mail address or a physical
page fitted to the window. Locked/read-only links are refused. Page-label authoring and navigation-preserving
Undo/Redo remain available from 0.14.0; see the [roadmap](docs/ROADMAP.md).

Available since **0.16.0**: dashed borders for rectangle, ellipse, line and arrow annotations through
**Dashed border** and CLI `annotate --dashed`. Existing custom dash patterns survive selection, recolouring and
geometry edits; only explicitly toggling the checkbox changes the pattern. Invalid styles are refused rather
than silently replaced. See [Project status](docs/PROJECT_STATUS.md) for validation and remaining limits.

Available since **0.17.0**: embed JPEGs with all eight EXIF orientations without recompression, including
the previously decoded orientations 5 and 7. Images → PDF also checks a cumulative **512 MiB estimated raster
budget** before decoding non-JPEG inputs. This estimates decoded image data, not total process memory.
UI/CLI entry points and 53 commands are unchanged. Independent debug and staged/public conversion checks passed:
exact JPEG streams, oriented renders, PNG transparency, budget refusals before raster decoding and unchanged inputs.
See [Project status](docs/PROJECT_STATUS.md) for executed evidence and remaining limits.

### New in 0.18.0

Group freehand strokes on one page into one annotation and one Undo step through **Finish drawing** /
**Cancel drawing**. Worker failure retains the draft for retry; conflicting save/close/document/tool actions
wait until Finish/Cancel. Reuse existing AddInk, point simplification, display geometry and protection (D109).
At most 64 strokes and 4,000 points per stroke. Drafts stay in memory until Finish and are absent from crash
recovery. No idle auto-commit or eraser. `stamp` / `page-numbers` help preserves literal `{n}` / `{total}`
through the existing clap help template (A16). No new dependency or CLI command; 53 commands remain.

### New in 0.19.0

The implemented lot adds mouse placement to **Prepare a form** for the existing text, checkbox, radio,
drop-down and list-box types. **Draw on page** temporarily hides the numeric dialog; drawing a rectangle
reopens it with the name, choices and flags retained. Only **Create Field** changes the PDF, through the existing
undoable creation path. Numeric creation remains available; no new field type, dependency or CLI command.
Local tests, formatting and Clippy passed; CI/upgrade, staged/public package and production updater checks passed. **0.19.0 was stable Latest at publication**. See
[next tasks](docs/NEXT_TASKS.md) and [project status](docs/PROJECT_STATUS.md).

### New in 0.20.0

The implemented 0.20.0 lot creates and edits FreeText annotations directly on the page (D111/A39).
Click or draw the existing box, type in an anchored multiline editor and use explicit **Add** / **Save** /
**Cancel** with the existing size, colour, background and border controls. The existing AddFreeText /
UpdateFreeText worker operations keep validation, protection and one Undo step; no new dependency,
annotation type or CLI command is added (53 commands remain).

The draft stays bound to its original page. Zoom remains available; conflicting navigation waits for
confirmation/cancellation. Document revision/tab/tool/workspace changes cancel stale editing. Pending
open/save/mutations and concurrent drafts are guarded; validation or worker failure retains typed text
for retry. Helvetica WinAnsi appearance and downward wrap growth remain unchanged. The editor draft
exists only in memory until confirmation and has no crash recovery.

Implementation and independent review passed. Local workspace: 468 passed tests, zero failures, seven
default ignored; formatting, all-target Clippy with warnings denied and 912 matching EN/FR keys passed.
The 75-check qpdf gate, four-document/744-page real corpus and update-helper self-test passed. Windows CI,
18 portable cases, 12 MSI baselines/60 logs, same-source ARM64, all six staged/public files and the production
updater passed. **0.20.0 was published as stable Latest at the time**; [Project status](docs/PROJECT_STATUS.md) records the executed evidence.

## Screenshots

| Dark home screen | Light home screen |
|---|---|
| ![Dark home screen](docs/screenshots/home-dark.png) | ![Light home screen](docs/screenshots/home-light.png) |

| Edit PDF objects | Organize pages |
|---|---|
| ![Object editing](docs/screenshots/edit-objects.png) | ![Page organization](docs/screenshots/organize-pages.png) |

## Install and update

Download from [GitHub Releases](https://github.com/lmdanavanem/SuniPDF/releases): **Windows x64 MSI** or **portable ZIP**.
An ARM64 portable ZIP is built in CI but has not been run on ARM64 hardware. SHA256 checksums are provided.
No Visual C++ redistributable is required.
Offline OCR includes Tesseract 5.5.3, Leptonica 1.87.0 and pinned English/French fast models; keep the bundled
`licenses/` folder with the application. Recognition does not download models or send PDFs anywhere.

From version 0.8.2, SuniPDF checks for updates at startup and shows a download/install button. You decide when to close your documents and install.
Installed x64 copies use the MSI; portable copies update their own folder with a backup.
Older versions need one manual upgrade to obtain this feature. Automatic checks can be disabled in About; manual checks remain available.

[English user guide](docs/USER_GUIDE.en.md) · [French user guide](docs/USER_GUIDE.fr.md)

## Build

Install [stable Rust](https://rustup.rs), **Visual Studio Build Tools** with the Desktop development with C++ workload,
and **CMake** on PATH. The first OCR setup builds pinned native sources and checks their SHA256 hashes.

```powershell
git clone https://github.com/lmdanavanem/SuniPDF.git
cd SuniPDF
.\scripts\fetch-pdfium.ps1
.\scripts\fetch-ocr.ps1 -Arch x64
cargo test --workspace -j 2
cargo run -p sunipdf-desktop -j 2 -- document.pdf
cargo run -p sunipdf-cli -j 2 -- info document.pdf
.\scripts\package-portable.ps1 -Arch x64
.\scripts\package-msi.ps1 -Arch x64
```

[Engineering notes](docs/ENGINEERING_NOTES.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Documentation](docs/README.md)

## License and acknowledgements

SuniPDF is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice.
[Third-party notices](docs/THIRD_PARTY_NOTICES.md) accompany each distribution.
Built with [PDFium](https://pdfium.googlesource.com/pdfium/), [pdfium-render](https://github.com/ajrcarey/pdfium-render),
[lopdf](https://github.com/J-F-Liu/lopdf) and [egui/eframe](https://github.com/emilk/egui).
OCR uses [Tesseract](https://github.com/tesseract-ocr/tesseract) (Apache-2.0),
[Leptonica](https://github.com/DanBloomberg/leptonica) (BSD-2-Clause) and
[tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) English/French models (Apache-2.0).
Their licences, authors, copyright inventory and build notice ship in `licenses/`.
SuniPDF is not affiliated with Adobe, Google or Foxit.
