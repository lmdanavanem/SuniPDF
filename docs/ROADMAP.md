# SuniPDF roadmap

[Français](ROADMAP.fr.md)

This roadmap separates delivered features and future work.
Executed checks and release evidence belong in [PROJECT_STATUS.md](PROJECT_STATUS.md).

The accepted preparation track toward 1.0 has [explicit acceptance criteria](RELEASE_1.0_CRITERIA.md).
Partial corpus/mutation work does not close native, interoperability, authenticity or conformance P1 gates.

## 0.39.0: reusable decoration presets (stable Latest)

[0.39.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) under `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

Named local presets for text/image watermarks, page numbers and six-zone headers/footers. Choose to load a draft, save under a unique name or delete explicitly. Atomic writes preserve active preferences on failure; malformed preset entries do not reset other valid settings. Page scope stays explicit and image files remain path references. Published 0.38.0 packages join migration regression. Office version probes run headlessly with a cancellable 30-second cold-start bound; conversion and security guards remain enforced. [Usage and limits](PAGE_DECORATIONS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404372880) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404384844) passed: **628 tests, zero failures**, 10 default ignored, **1149 EN/FR keys**, **31 portable scenarios**, **25 MSI baselines/125 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **42 cases, 148 CLI calls and 2 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,879,736-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](qa/RELEASE_0.39.0.json).

## 0.38.0: six-zone headers and footers (published; Latest at the time)

[0.38.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) under `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six header/footer zones through Page numbers and the headers-footers CLI command. Absolute page/count placeholders, margins, size and selected pages respect displayed rotation/crop coordinates. Overflow is refused atomically; allowed encryption and copy-only CLI guards remain. Published 0.37.0 packages join migration regression. [Usage and limits](PAGE_DECORATIONS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) passed: **623 tests, zero failures**, 10 default ignored, **1143 EN/FR keys**, **30 portable scenarios**, **24 MSI baselines/120 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **42 cases, 148 CLI calls and 2 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,823,494-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](qa/RELEASE_0.38.0.json).

## 0.37.0: image watermarks (published; Latest at the time)

[0.37.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) under `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Image watermarks in Pages and the CLI: transparency, proportional width, angle, seven positions and margins. Rotated/cropped pages fit on both axes. Atomic selected-page edits, one desktop Undo and copy-only CLI output preserve allowed encryption. Published 0.36.0 packages join upgrade baselines. [Usage and limits](PAGE_DECORATIONS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) passed: **618 tests, zero failures**, 10 default ignored, **1134 EN/FR keys**, **29 portable scenarios**, **23 MSI baselines/115 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **24 cases, 113 CLI calls and 1 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included. Two fixture-password findings were reviewed and dismissed as “used in tests” under `lmdanavanem`; code scanning remains enabled.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,806,746-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](qa/RELEASE_0.37.0.json).

## 0.36.0: configurable keyboard shortcuts (published; Latest at the time)

[0.36.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) under `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Configurable keyboard shortcuts in Help and the palette: filter, assign, conflict refusal, disable and restore. Cancellable drafts, atomic preferences and consistent labels. Text input and reserved keys are preserved; published 0.35.0 packages join upgrade baselines. [Usage and bounds](KEYBOARD_SHORTCUTS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) passed: **613 tests, zero failures**, 10 default ignored, **1128 EN/FR keys**, **28 portable scenarios**, **22 MSI baselines/110 logs**. Seven new headless egui tests cover shortcuts, conflicts, focus, display, cancellation and preference compatibility. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; **20 image cases** and **14 PDF-inspection cases**, plus retained suites, were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,788,530-byte** x64 ZIP without installing it on the workstation.

Native Windows keyboard layouts and Narrator remain unqualified. Explorer remains separate; only commands in the existing shortcut table can be customized. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed. [Evidence](qa/RELEASE_0.36.0.json).

## 0.35.0: visual signature preparation (published; Latest at the time)

[0.35.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) under `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Visual signatures: optional drawn-stroke smoothing, paper-shadow reduction and the `prepare-signature-image` PNG command. Previews bind to prepared pixels; original points and source files are retained, with explicit drawing budgets. Published 0.34.0 packages join the upgrade baselines. [Usage and bounds](VISUAL_SIGNATURE_PREPARATION.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) passed: **606 tests, zero failures**, 10 default ignored, **1114 EN/FR keys**, **20 image cases**, **14 PDF-inspection cases**, **27 portable scenarios**, **21 MSI baselines/105 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, PDF, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; image and inspection suites were also rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,752,605-byte** x64 ZIP without installing it on the workstation.

Visual signatures provide no identity proof; cleanup may remove faint ink and needs preview review. Native pointer/tablet/Narrator interaction and customer photos remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed. [Evidence](qa/RELEASE_0.35.0.json).

## 0.34.0: saved PDF inspection (published; Latest at the time)

[0.34.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) under `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspect the saved PDF from Properties and the `inspect` JSON command: seven declaration categories, source SHA256, explicit partial results, encrypted input and stale-reply refusal. The saved-revision guard hashes disk bytes in chunks. [Usage and bounds](PDF_INSPECTION.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) passed: **600 tests, zero failures**, 10 default ignored, **1108 EN/FR keys**, **14 packaged inspection cases**, **26 portable scenarios**, **20 MSI baselines/100 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent-reader, PDF-corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,741,242-byte** x64 ZIP without installation on the workstation.

Structural inventory without a safety verdict or signature/PDF/A/PDF/UA validation. Packages remain unsigned; native/hardware qualification and 1.0 criteria remain open. Projects edits stay deferred at the owner’s request, with no new synchronization claimed. [Evidence](qa/RELEASE_0.34.0.json).

## 0.33.0: 1.0 preparation and PDF compatibility (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) under `lmdanavanem`, release `417802490`, immutable source `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Preserve imported PDF 2.0 bookmarks containing tabs/line breaks while retaining strict new-title validation. Integrate egui/eframe 0.37 together and fix Back/Forward position after destination zoom changes. Add seven pinned licensed PDF Association examples, 64 bounded PDF/CMS mutations, executable guard controls and command measurements. The public 0.32.0 becomes an additional upgrade baseline. [1.0 criteria](RELEASE_1.0_CRITERIA.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) passed: **592 tests**, zero failures, 10 default ignored, **1092 EN/FR keys**; 84 qpdf corpus checks and 9 independent feature outputs, 4 font cases, DX12 software rendering, 14 Office round trips, four NIST documents/744 pages; **7 PDF 2.0 examples/8 pages/21 qpdf checks**, **64 mutations and 2 guard controls**, **25 portable scenarios**, **19 MSI baselines/95 logs**. The packaged CLI additionally passed the 7-example/21-qpdf suite, 11 object-order scenarios/3 qpdf checks, 11 local-review and 8 security scenarios. CodeQL Rust/Python/Actions passed with zero open alerts.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 test harness, with its unchanged updater module verified by hash, downloaded and verified the **26,726,762-byte x64 ZIP in 1.65 s**, without installation.

Project edits are deferred at the owner’s explicit request. The public readback shows **43 items, 25 Done, 18 Todo**; both P1 tasks have documented partial progress, with no claimed closure or status change.

Windows packages remain unsigned with no independent publication key. Teaching examples and deterministic mutations do not replace a customer/hostile corpus, coverage-guided fuzzing or hardware measurements. ARM64 remains experimental; native interactions, named third-party readers and PDF/A/PDF/UA conformance remain open. [Evidence](qa/RELEASE_0.33.0.json).

## 0.32.0: object ordering (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) under `lmdanavanem`, release `417623635`, immutable source `8c492db35f1712648d5472593146077230befb7c`.

Drag one object or the selected group in the Objects panel, with an insertion line, page-cache/context guards, Escape and one Undo step. New `reorder-objects` JSON command shares the explicit final destination, relative order, permission, atomic-save and password safeguards. [Scope and usage](OBJECT_ORDER.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) passed: **588 tests**, zero failures, 9 default ignored; **1092 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **24 portable scenarios**, **18 MSI baselines/90 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. The packaged CLI passed **11 object-order scenarios and 3 additional qpdf checks**, plus 11 retained local-review and 8 security scenarios.

Six public files and `dist/` copies verified by SHA256. The freshly built harness from this source downloaded and verified the **27,119,935-byte x64 ZIP in 1.75 s**, without installation. Projects: **43 items, 25 Done, 18 Todo, zero In Progress**; the P3 Suggestion is delivered, original criteria preserved, description read back after a server reload and all three saved filters verified.

Windows packages remain unsigned; no independent publication key is available. Hardware, native/manual flows and reader interoperability remain separate qualification; no PDF/A/PDF/UA/LTV certification. [Evidence](qa/RELEASE_0.32.0.json).

## 0.31.0: local comparison and multi-document search (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) under `lmdanavanem`, release `417463135`, immutable source `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Local comparison of two saved PDFs (text, geometry and previews), explicit physical alignment and stale-preview refusal. Literal search across selected files, excerpts/page navigation, progress, cancellation and explicit partial results. New `compare` and `search-files` JSON commands; EN/FR interface. [Scope and limits](LOCAL_REVIEW.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) passed: **582 tests**, zero failures, 9 default ignored; **1091 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **23 portable scenarios**, **17 MSI baselines/85 logs**. CodeQL Rust/Python/Actions passed with zero open alerts after individual review of four synthetic test credentials.

Six public files and `dist/` copies verified by SHA256; eleven local-review and eight security scenarios passed with the packaged CLI. The freshly built harness from this source downloaded and verified the **27,090,504-byte x64 ZIP in 1.71 s**, without installation. Projects: **43 items, 24 Done, 19 Todo, zero In Progress**; both P2 Suggestions are delivered, their criteria retained and all three filters read back.

Windows packages remain unsigned; no independent publication key is available. ARM64 hardware and manual flows remain separate qualification. Bounded comparison does not establish PDF identity or conformance. [Evidence](qa/RELEASE_0.31.0.json).

## 0.30.1: security patch (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) under `lmdanavanem`, release `417149160`, immutable source `697a963c756b316e1eb1e678298886bc08521da1`.

XFA/Screen and accessibility-text cleanup, owner authorization before reprotection, stronger signature verification, environment/stdin PDF passwords and hardened Office/runtime inputs. Unresolved P1 Roadmap items and Suggestions remain deferred. [Details](RELEASE_0.30.1.md).

[Release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) and [main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) passed on this source: **575 tests**, zero failures, 9 default ignored; **1060 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature cases**, **4 font-reader cases**, DX12 software rendering, **14 Office round trips**, 4 licensed PDFs/744 pages, **22 portable scenarios**, **16 MSI baselines/80 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

The six public files and `dist/` copies match the inspected packages and SHA256 digests. Eight security scenarios also pass with the packaged CLI. The production updater verified the **27,029,229-byte** x64 ZIP in **1.21 s**, without installation on this host, using the previously built harness and unchanged updater module.

Updates still depend on GitHub HTTPS and same-channel SHA256: no independent publication key is available. Windows packages remain unsigned; ARM64 hardware and manual flows remain separate qualification. [Evidence and limits](qa/RELEASE_0.30.1.json).

## 0.30.0: editing, document safety and local conversion (published; Latest at the time)

Published (Latest at the time) as [0.30.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) under `lmdanavanem`, release `415568680`, immutable source `4db31c0ee62cfae9d65239edf1eac93db147841e`. The five Bugs and fourteen Roadmap P2 items ship within their [documented supported limits](RELEASE_0.30.0.md). P1 Roadmap items and Suggestions remain deferred.

The [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) and [MAIN CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) passed on this source: **551 tests**, zero failures, 9 default ignored; **1047 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature outputs**, **4 MuPDF font cases**, DX12 software WARP, **14 Office round trips**, 4 licensed PDFs/744 pages, **21 portable scenarios**, **15 MSI baselines/75 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

Six public assets match the inspected packages and SHA256 digests byte for byte. The production updater verified the 26,978,019-byte x64 ZIP in 2.55 seconds without installing on this host, using the previously built test harness with an unchanged updater module. Project readback: **42 unique items, 20 Todo, 0 In Progress, 22 Done**, three saved filters and zero unsaved changes. [Evidence and limits](PROJECT_STATUS.md).

Packages remain unsigned. ARM64 is cross-built and packaged, without execution on ARM64 hardware. Physical printers, Narrator, manual updater interaction and physical GPU/window behavior still need qualification; WARP proves software offscreen rendering only. Unsupported inputs remain refused. PDF/A metadata preservation does not certify PDF/A or PDF/UA conformance.

## 0.21.1: document preservation (published; Latest at the time)

The two P1 fixes ship in [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1): refuse unsupported text without losing the correct
draft, and preserve non-widget annotations without usable appearances when flattening. 476 tests,
84 qpdf checks, 20 portable scenarios and 14 MSI baselines passed; six public files and the live updater
were verified. See [Project status](PROJECT_STATUS.md).

The planned 0.30.0 batch after 0.21.1 is now published; current counts and remaining P1 work are recorded above.

## Delivered baseline: 0.9.0

The seven requested features are delivered: autosave/recovery, Windows printing, richer text selection,
two-page/Book view, text/checkbox form-field creation, offline English/French OCR, and CMS/PFX digital signatures.
English is the default language; French remains available. All 14 tools have working commands.
See [FEATURE_MATRIX.md](FEATURE_MATRIX.md) for the exact scope and [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for limits.

## 0.9.1: reliability and distribution

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) under `lmdanavanem`;
executed validation and package evidence are tracked in `PROJECT_STATUS.md`:

- Preserve PDF/A identification and unrelated XMP properties when editing metadata; reject malformed XML and DTDs.
  Preserving identification does not certify PDF/A conformance.
- Write the explicit AES-256 encryption key length required by strict readers, including after re-encryption.
- Add pinned qpdf structural checks for synthetic fixtures and generated output to CI.
  This is independent structural QA, not coverage of a real-world or hostile PDF corpus.
- Seven real portable update/failure/rollback scenarios to 0.9.1 and a real MSI installation/upgrade/transactional rollback,
  downgrade rejection and uninstall regression passed on disposable Windows runners, preserving user data.
- Use .NET SHA256 hashing in the helper for Windows PowerShell 5 hosts without Get-FileHash and report restart failures.
- Derive the Windows icon from the dedicated, retouched author-provided asset (`assets/icon/sunipdf-source.png`).

Workspace checks (322 tests), independent qpdf checks (22 PDFs/three malformed inputs), portable helper checks and
the disposable-runner MSI regression passed. All six public assets were downloaded and matched the verified
payloads/checksums. Interactive updater UX and real hardware remain separate checks.

## 0.10.0: requested P1/P2 lot published

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) under `lmdanavanem`.

| Area | Included scope |
|---|---|
| P1 data safety and corpus | Atomic temporary-file replacement, interrupted/full-disk/failure regressions, killed partial-save child and four pinned public NIST documents |
| P2 compression | Image resampling/JPEG quality, before/after size, progress/cancel, source-preserving new copies and conservative unsupported-image handling |
| P2 reading | View-only rotation, full screen, bounded per-tab navigation history and upright editing |
| P2 forms | Radio/combo/list creation, XFDF export, bounded JSON/XFDF/FDF values import, grouped Apply/Discard and keyboard widgets |
| P2 editing | Mouse crop preview and grouped rotation/resizing with one undo |
| P2 batches | Sequential compression/offline OCR, bounded inputs, cancellation, per-file errors and no overwriting |

The 362 workspace tests, 31 independent synthetic qpdf checks, four public NIST documents (744 pages), eight portable
update/failure/rollback cases and two MSI baselines passed. Final CI and all six public assets passed verification,
including packaged CLI/OCR, icons, MSI payloads and the live updater. Evidence is recorded in `PROJECT_STATUS.md`.
Hardware, native interactive flows, Narrator, third-party readers and broader customer/hostile corpus coverage remain independent work.

## 0.11.0: navigation, forms and protected copies (published)

Implemented and locally validated: read-only page labels with exact unique-label navigation and `#N` physical
indices; named destinations and `/XYZ`, `/Fit`, nonzero `/FitH` for links/bookmarks; same-page and zoom-aware history;
form-read errors that preserve inputs and require an explicit retry. `/FitH` zero/null and other fit modes fall back
to page navigation. Also implemented for 0.11.0: free text in existing editable drop-downs through the panel/on-page
editor, and extraction/split copies retaining source encryption/passwords/permissions with same-source-path refusal.
The existing permission policy is unchanged; editable drop-down authoring was outside the 0.11.0 scope.

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) under `lmdanavanem`; Latest at the time.

The expanded 0.11.0 source passed 390 workspace tests (zero failed, seven default ignored), formatting/Clippy with
`-D warnings`, 858 matching EN/FR keys, 44 qpdf checks across 42 distinct PDFs and four real NIST documents
(744 pages; 91.59 seconds, peak child memory 329,347,072 bytes). [Final Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) and
[same-source ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) passed, as did nine real portable scenarios from
0.8.2/0.8.3/0.9.0/0.9.1/0.10.0, three MSI baselines (0.8.0/0.9.1/0.10.0) and all six staged asset checks. Packaged
checks include owner-opened AES-128/256 copies, decoded content/qpdf, page labels, metadata, OCR, forms and compression.
Immutable release source: `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`. All six unauthenticated public downloads match API/sidecar/staged hashes; enhanced checks passed again on public
binaries. The production live updater verified the 19,687,375-byte Latest x64 ZIP without installation. Evidence
and remaining limits are recorded in `PROJECT_STATUS.md`; published 0.10.0 source/packages remain unchanged.

## 0.12.0: editable field creation and radio exports (published; Latest at the time)

Create editable drop-downs from Prepare a form or `add-form-field --kind combo --editable`, with predefined choices
still required. Radio `/Opt` export values are separated from widget appearance states for filling and JSON/XFDF/FDF
interchange; CLI information adds `widgets[].export_value`. Malformed or duplicate mappings keep the affected
field read-only; no-`/Opt` radios retain state-name values (D100–D101).

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (Latest at the time), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. The 398 workspace tests (zero failed, seven default
ignored), formatting/Clippy, 859 EN/FR keys, 48 qpdf checks and four NIST/NBS documents (744 pages; 59.68 seconds)
passed. [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688), the
[same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878), ten portable
scenarios and four MSI baselines passed. All six staged/public files and enhanced binary checks passed; verified
files are in `dist/`. The production updater verified the 19,699,323-byte Latest x64 ZIP without installation.
Executed evidence and hardware/interoperability limits are recorded in `PROJECT_STATUS.md`.

## 0.13.0: visual signature improvements (published; Latest at the time)

Typed name/initials use an explicit native preview and the existing font/image/library paths. Current/all/selected
pages share fitted display-space geometry; multi-page placement is atomic, cancellable between targets and one
undo. CLI `add-image --pages` is additive; legacy `--page` behaviour remains compatible (D102–D103).
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) under `lmdanavanem` (Latest at the time), release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Local validation passed: 410 workspace tests,
formatting/Clippy, 870 EN/FR keys, 53 qpdf checks and four licensed documents (744 pages; 76.40 seconds locally).
Windows CI, same-source ARM64 compilation, eleven portable scenarios, five MSI baselines and all six staged/public
files with enhanced binary checks passed. Verified copies are in `dist/`; the production updater verified the
19,730,578-byte Latest x64 ZIP without installation. Evidence and hardware/reader limits: `PROJECT_STATUS.md`.

## 0.14.0: page-label authoring (published; Latest at the time)

Apply six navigation-label styles over an inclusive physical range through Pages/Organize and two CLI commands;
preserve labels outside the range, reject malformed existing trees before mutation, and reset all label rules
without claiming secure erasure. Printed numbers remain unchanged; structural operations do not semantically
rebase the ranges. Geometry-preserving label/content Undo/Redo retains navigation (D104–D105).
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) under `lmdanavanem` (Latest at the time), release
`414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation passed: 420 workspace tests,
formatting/Clippy, 890 EN/FR keys, 63 qpdf checks and four licensed documents (744 pages; 53.91 seconds locally).
Windows CI, same-source ARM64 compilation, twelve portable scenarios, six MSI baselines and six staged/public
files with enhanced checks passed. Verified copies are in `dist/`; the production updater verified the
19,785,200-byte Latest x64 ZIP without installation. Executed evidence and remaining limits: `PROJECT_STATUS.md`.

## 0.15.0: editing existing link targets (published milestone)

Reuse the annotation link dialog and protected writer to change an existing destination through Edit target,
double-click or CLI `set-link-target`. Safe web/mail URI or physical-page Fit replaces all previous target/action
fields, keeping the link's area/style/comments/replies and unrelated document data. Existing detailed/named views
are explicitly replaced by whole-page Fit; unknown targets never default silently to page 1. ReadOnly/Locked/
LockedContents and malformed flags are refused; one undo preserves geometry/navigation and stale dialogs close
on document changes (D106). [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) under
`lmdanavanem` (Latest at the time), release `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation passed: 429 workspace
tests, formatting/Clippy, 894 EN/FR keys, 68 qpdf checks and four licensed documents (744 pages; 48.891 seconds
locally). Windows CI, same-source ARM64 compilation, thirteen portable scenarios, seven MSI baselines and six
staged/public files with enhanced checks passed. Verified copies are in `dist/`; the production updater verified
the 19,803,226-byte Latest x64 ZIP without installation. Executed evidence and remaining limits: `PROJECT_STATUS.md`.

## 0.16.0: dashed shape annotations (published milestone)

Dashed rectangle/ellipse/line/arrow annotations reuse the existing toolbar, CLI and appearance writer. UI and
`annotate --dashed` choose `[6,3]` points; actual custom patterns remain through ordinary edits unless explicitly
toggled. Solid defaults, continuous fill and solid arrowheads stay unchanged. Unsupported types, malformed
SuniPDF border styles and excessive regeneration work are refused (D107). No dependency or command is added;
the CLI remains at 53 commands. [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0)
under `lmdanavanem` (Latest at the time), release `414934190`, source `6cd7843734f3924e87033d8b05539d493ccbcb95`. Local validation,
Windows CI and same-source ARM64 compilation passed: 439 tests, 895 EN/FR keys, 73 qpdf checks, four licensed
documents/744 pages (53.542 seconds locally), fourteen portable cases and eight MSI baselines. Six staged/public
files and enhanced checks passed; verified copies are in `dist/`. The production updater verified the
19,821,719-byte x64 ZIP without installation. Evidence and remaining limits are in `PROJECT_STATUS.md`.

## 0.17.0: image conversion reliability (published; Latest at the time)

Complete direct JPEG embedding for EXIF orientations 5 and 7, so all eight orientations avoid image decoding/
recompression. Preflight non-JPEG inputs against a cumulative 512 MiB raster-data estimate:
`sum(max(decoder.total_bytes(), width × height × 4))`, with checked arithmetic, finite/positive physical geometry
and repeated loading checks for simple file changes (D108). Quality/transparency and existing UI/CLI/API remain.
The estimate is not a native/process RSS ceiling or compressed-JPEG limit. No dependency or command
is added. Budget excess/overflow has a localized `image_batch_too_large` error; invalid geometry retains `other`.
Local validation passed: 445 workspace tests, formatting/Clippy, 896 EN/FR keys, 74 qpdf checks and four licensed
documents/744 pages (48.187 seconds; peak child memory 330,264,576 bytes), the update-helper self-test and
independent debug conversion gate. Source `5785de282e126fb4666b28fb83340d0e9c6e9acf` passed Windows CI,
same-source ARM64 compilation, fifteen portable cases, nine MSI baselines and 45 MSI logs; CI corpus took
45.791 seconds with peak child memory 321,413,120 bytes. [Published (Latest at the time)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0)
under `lmdanavanem`, release `415074108`, with six files. All staged/public files and enhanced conversion checks
passed; verified copies are in `dist/`. The production updater verified the 19,825,743-byte x64 ZIP without
installation. Evidence and remaining limits belong in `PROJECT_STATUS.md`.

## 0.18.0: grouped freehand drawing and literal CLI help (published)

Group freehand strokes on one page into one annotation and one Undo step through **Finish drawing** /
**Cancel drawing**. Worker failure retains the draft for retry; conflicting save/close/document/tool actions
wait until Finish/Cancel. Reuse existing AddInk, point simplification, display geometry and protection (D109).
At most 64 strokes and 4,000 points per stroke. Drafts stay in memory until Finish and are absent from crash
recovery. No idle auto-commit or eraser. `stamp` / `page-numbers` help preserves literal `{n}` / `{total}`
through the existing clap help template (A16). No new dependency or CLI command; 53 commands remain.

[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) under `lmdanavanem`, release `415140702`,
source `7e52de82244faf9a020ed3ed233176775c007e97`. Local gates passed: 452 workspace tests (zero failed, seven default ignored),
formatting/Clippy with warnings denied, 907 matching EN/FR keys, 75 qpdf checks and the update-helper self-test.
Four licensed NIST/NBS documents (744 pages) passed in 50.796 seconds, sampled peak child memory
329,445,376 bytes; all sources stayed unchanged. Six new headless UI tests, the extended native-worker
grouped Undo/Redo/failure regression and the CLI help regression passed.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082734) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082934/job/117057854520) passed,
with sixteen portable cases, ten MSI baselines and 50 verified MSI logs. The CI corpus took 44.494 seconds,
sampled peak child memory 320,299,008 bytes. All six staged/public files, API digests/sidecars and enhanced
package checks passed: five grouped-Ink readbacks (plain/AES-128/AES-256 with both passwords), four literal-help
checks and existing conversion, forms, labels, links, dashes, compression, OCR and MSI payload checks.
Verified copies are in `dist/`; the production updater verified the 19,834,488-byte Latest x64 ZIP without
installing it on the host. Executed evidence and remaining limits are in `PROJECT_STATUS.md`.

## 0.19.0: mouse placement of existing form types (published; Latest at the time)

Add **Draw on page** to the current numeric form builder, preserving text/checkbox/radio/drop-down/list-box
types, names, choices and flags. Draw a finite rectangle clamped to one page, at least 4 pt wide/high, then
review the reopened numeric dialog and explicitly **Create Field**. Drawing itself does not mutate the PDF;
the existing worker edit supplies one Undo step, validation and encryption (D110/A38).

Crosshair/preview, widget exclusion and stale document/tab/tool guards are included. Numeric creation,
core/CLI APIs and 53 commands remain unchanged; no dependency or field type is added. Implementation and local tests
passed, along with formatting/Clippy; CI/upgrade, staged/public package and production updater checks passed. **0.19.0 was stable Latest at publication**. Actual evidence belongs in
`PROJECT_STATUS.md`. Existing-field geometry editing, XFA creation and native hardware checks remain separate.

## 0.20.0: on-page FreeText creation and editing (published; Latest at the time)

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
updater passed. **0.20.0 was published as stable Latest at the time**; [Project status](PROJECT_STATUS.md) records the executed evidence.

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
See [Code Scanning review](CODE_SCANNING_REVIEW.md)
and [Project status](PROJECT_STATUS.md). The 53 CLI commands are unchanged.

## Phase 0–1: planning and foundations — delivered

Rust workspace, PDFium, typed errors, opening/navigation/zoom/close, passwords, drag and drop, FR/EN localisation,
persisted settings, light/dark themes, command registry and Windows resources are in place.
Historical milestones and decisions remain in `DECISIONS.md` and `PROJECT_STATUS.md`.

## Phase 2: reading and core PDF operations — largely delivered

Delivered: tabs, bounded render/thumbnail caches, continuous/two-page/Book views, logical cross-page selection,
search, bookmarks, merge/split/extract/rotate/reorder, atomic save, undo/redo and main annotations.

Delivered in 0.10.0: view-only rotation, full-screen reading and bounded per-tab navigation history.
Delivered in 0.11.0: page-label reading and precise internal navigation. Authoring/reset and navigation-preserving
Undo/Redo were delivered in 0.14.0. 0.30.0 adds semantic label rebasing, bookmark authoring and
bounded preservation of destinations, fields, layers and attachments across page operations; see
[supported structure](BOOKMARKS_AND_PAGE_STRUCTURE.md). Runtime and documented independent-reader gates passed.
Intended reading order for complex untagged columns remains separate work; qpdf alone cannot prove appearance.

## Phase 3: professional editing — partially delivered

Delivered: editable text/images and shapes, object transformations, grouped move and clipboard, system TrueType
fonts, stamps/watermarks/numbering, threaded comments, metadata, visual signatures, AcroForm filling/flattening,
text/checkbox creation and autosave/recovery.

Delivered in 0.10.0: grouped rotation/resizing, mouse crop preview, radio/combo/list creation, strict value-only
FDF/XFDF interchange and keyboard page widgets.
Delivered in 0.12.0: editable drop-down creation and radio `/Opt` export mapping with conservative read-only safeguards.
Delivered in 0.13.0: typed visual signatures and atomic current/all/selected-page placement.
Delivered in 0.15.0: existing-link target replacement, preserving its area/appearance/comments.
Delivered in 0.16.0: dashed borders for the four existing shape annotations, with custom-pattern preservation.
Delivered in 0.18.0: grouped desktop ink with explicit Finish/Cancel and guarded transitions (D109).
0.30.0 adds [font subsets and selected static TrueType/CFF1 collection faces](FONT_EMBEDDING.md),
bounded paragraph preview/reflow, explicit editable object paste, stable rotation pivots, image-opacity readback,
annotation-aware page scaling, [rich annotations](RICH_ANNOTATIONS.md) and precise destination authoring.
Release-source regression gates passed. Variable/CFF2/color fonts, general shaping/bidirectional paragraph editing, arbitrary
editable XObjects and full rich-text span editing remain unsupported; unsupported inputs are refused.
Acceptance: rendering, saving and reopening covered for each added operation, with applicable accessibility checks.

## Phase 4: conversion and OCR — partially delivered

Delivered: images ↔ PDF, PDF → text/PNG, lossless optimisation, CLI batches and bundled local English/French OCR.

Delivered in 0.17.0: all eight JPEG EXIF orientations embedded directly and cumulative raster-data estimates;
measured native/process memory limits remain separate work (D108).

0.30.0 adds [local Office conversion and saved Standard/Print profiles](OFFICE_CONVERSION.md):
installed supported LibreOffice only, simple editable DOCX text, literal-string XLSX grids and raster PPTX pages.
Converter absence is explicit; the app does not install it. Complex editable exports are refused rather than
promised layout fidelity. Real-converter/corpus gates passed; arbitrary Office layout reconstruction
and a general workflow designer remain future work. Compression/OCR batch UI was delivered in 0.10.0.

## Phase 5: security and productivity — partially delivered

Delivered: AES-128/256 and permissions, owner-authorised protection removal, metadata cleanup, irreversible redaction,
CMS integrity verification and incremental PFX/P12 signed copies, JSON CLI and automation commands.

0.31.0 implements bounded local comparison and search across selected saved files; [scope](LOCAL_REVIEW.md).
0.36.0 adds [configurable command shortcuts](KEYBOARD_SHORTCUTS.md). Remaining: confined opt-in MCP and Explorer integration; native keyboard/Narrator qualification stays open.
0.30.0 adds [Windows chain/revocation and RFC3161 timestamp verification, plus per-user DPAPI
signature storage](SIGNATURE_SECURITY.md), and [redaction preview, persistent marks, bounded text splitting
and hidden-data sanitization](REDACTION.md). Release-source regression gates passed. Network retrieval needs explicit consent
for each verification; offline/unknown is never trusted. Embedded DSS is unverified evidence, not archival LTV.
AATL/EUTL, PKCS#11, qualified-signature assessment and timestamp creation remain unsupported. PDF signing time
is still a signer declaration. Acrobat/Foxit interoperability is a separate, unverified property.
Executable/MSI code signing needs an appropriate supplied identity.

## Phase 6: quality and distribution — ongoing

Delivered baseline: x64 portable/MSI and ARM64 portable releases, SHA256 files, startup update notifications with
explicit installation, FR/EN guides, third-party notices and GitHub build workflows.

Delivered in 0.10.0: four pinned public NIST samples and interrupted/full-disk/failed-save scenarios.
0.30.0 adds an offline license inventory and optional GPU compatibility changes; release gates passed. OpenGL remains the default. Remaining: customer/hostile/large corpus expansion, fuzzing,
published benchmarks with hardware, ongoing license-inventory maintenance, Narrator and third-party reader checks,
interactive installed/portable update flows, physical printers and real ARM64/OpenGL execution.
Automated MSI regression covers its scripted scenario only; packaged cross-compilation is not ARM64 runtime validation.

The historical [Code Scanning review](CODE_SCANNING_REVIEW.md) examined 117 alerts, including 115 critical
findings, without confirming a production secret. Remediation/triage and explicit workflow permissions were
included in the published 0.21.x work; executed closure evidence is in [Project status](PROJECT_STATUS.md).
This historical result does not establish the scan status of 0.30.0.

## Work beyond 0.39.0

1. Run native updater/save flows, Narrator, printers, ARM64 and third-party readers on the appropriate equipment.
2. Extend real-corpus coverage and performance measurements; public NIST samples do not establish universal compatibility.
3. Keep the remaining P1 Roadmap items and Suggestions open: dedicated PDF/A/PDF/UA validation, broader corpus/fuzzing/
   hardware measurement and integrations need their own accepted scope. Extend editing, Office fidelity and
   archival trust only beyond the explicit limits of this release.



No future release date or version is committed by this backlog. Optional AI remains disabled by default.
