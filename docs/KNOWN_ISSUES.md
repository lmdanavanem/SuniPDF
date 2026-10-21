# Known issues

[Français](KNOWN_ISSUES.fr.md)

## 0.40.0: calibrated measurements and image stamps (stable Latest)

[0.40.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) under `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

Local length, polyline, perimeter, simple-polygon area and angle annotations with explicit two-point calibration, metric/decimal imperial units, precision and JSON records. Each measurement embeds its own scale and geometry; preview/Undo and copy-only CLI output preserve originals. Image Stamp annotations embed aspect-fitted pixels and alpha. Text watermarks/numbering now preflight all pages and refuse invalid margins or overflow atomically. Published 0.39.0 packages join upgrade regressions. A 26-card AEC backlog prioritises local workflows; cloud and collaboration are deferred at the owner's request, with prices undecided. [Usage and limits](MEASUREMENTS.md).

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661526853) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661547803) passed: **637 tests, zero failures**, 10 default ignored, **1163 EN/FR keys**, **32 portable scenarios**, **26 MSI baselines/130 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Page decorations passed **80 cases, 256 CLI calls and 10 qpdf check(s)**. Independent readers, corpus, software rendering, Office, mutation and security gates remain included.

Six public files and `dist/` copies verified by SHA256; page decorations, **20 image cases**, **14 PDF-inspection cases** and retained suites were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **27,121,645-byte** x64 ZIP without installing it on the workstation.

Native Windows interaction, keyboard layouts and Narrator remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request; no fresh synchronization is claimed. [Evidence](qa/RELEASE_0.40.0.json).

## 0.39.0: reusable decoration presets (published; Latest at the time)

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

## 0.30.0: editing, document safety and local conversion (published; Latest at the time)

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) under `lmdanavanem`, release `415568680`, immutable source `4db31c0ee62cfae9d65239edf1eac93db147841e`. The five Bugs and fourteen Roadmap P2 items ship within their [documented supported limits](RELEASE_0.30.0.md). P1 Roadmap items and Suggestions remain deferred.

The [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) and [MAIN CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) passed on this source: **551 tests**, zero failures, 9 default ignored; **1047 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature outputs**, **4 MuPDF font cases**, DX12 software WARP, **14 Office round trips**, 4 licensed PDFs/744 pages, **21 portable scenarios**, **15 MSI baselines/75 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

Six public assets match the inspected packages and SHA256 digests byte for byte. The production updater verified the 26,978,019-byte x64 ZIP in 2.55 seconds without installing on this host, using the previously built test harness with an unchanged updater module. Project readback: **42 unique items, 20 Todo, 0 In Progress, 22 Done**, three saved filters and zero unsaved changes. [Evidence and limits](PROJECT_STATUS.md).

Packages remain unsigned. ARM64 is cross-built and packaged, without execution on ARM64 hardware. Physical printers, Narrator, manual updater interaction and physical GPU/window behavior still need qualification; WARP proves software offscreen rendering only. Unsupported inputs remain refused. PDF/A metadata preservation does not certify PDF/A or PDF/UA conformance.

## 0.21.1: document preservation (published; Latest at the time)

The two P1 fixes ship in [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1): refuse unsupported text without losing the correct
draft, and preserve non-widget annotations without usable appearances when flattening. 476 tests,
84 qpdf checks, 20 portable scenarios and 14 MSI baselines passed; six public files and the live updater
were verified. See [Project status](PROJECT_STATUS.md).

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

## Build environment (this machine)

1. ~~No Build Tools~~ **Resolved on 2026-10-13**: Build Tools installed, GNU override removed, tests green under MSVC.
   The GNU + WinLibs fallback remains documented in `scripts/dev-env.ps1` for a machine without admin rights.
2. **Limited memory (14 GB, ~1.5 GB free during the session).** With 16 cores, `cargo` launches 16 `rustc`
   and the system reaches its commit limit: `0xC0000409` aborts and misleading errors
   ("only metadata stub found", ICE). Workaround: `cargo build -j 2` (or `CARGO_BUILD_JOBS=2`)
   and `debug = "line-tables-only"` in the dev profile.
3. (GNU fallback only) **The binutils shipped by rustup are not enough** for `raw-dylib` crates
   (`dlltool` without `as`) and the GNU rustc aborts under clippy on the `windows` crate: full WinLibs required,
   clippy limited to crates without a `windows` dependency.

## Dependencies

4. **Optional GPU compatibility**: 0.30.0 aligns the Windows GPU dependencies and adds optional
   wgpu/DX12 plus a software-renderer gate. Release-source regression gates passed. glow/OpenGL stays the default/fallback;
   real GPU/window behavior and ARM64 hardware remain unverified. See [release scope](RELEASE_0.30.0.md).
5. **pdfium-render 0.9.4 targets PDFium build 7881**; newer builds (8100 as of 2026-10-12)
   probably work but are untested. Version pinned in `scripts/fetch-pdfium.ps1`.

11. ~~MSI not built~~ **Resolved on 2026-10-14**: standalone WiX 3.14 binaries (`third_party/wix314`, official zip, no
    installation or admin rights); `scripts/package-msi.ps1` detects them through `WIX_BIN`; x64 MSI produced and validated
    (ICE `smoke.exe`, administrative extraction).
12. **Local ARM64 build blocked by a missing Visual Studio component** (attempted on 2026-10-14): the Rust target
    `aarch64-pc-windows-msvc` is installed and the ARM64 PDFium downloads, but linking fails because
    the Build Tools only have the x64 and x86 tools (`VC\Tools\MSVC\<version>\bin\Hostx64\arm64` missing, ARM64 CRT
    missing). Installation (administrator rights required, refused during the unattended session):
    `& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify --installPath
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools" --add Microsoft.VisualStudio.Component.VC.Tools.ARM64
    --passive --norestart`, then `scripts\package-portable.ps1 -Arch arm64` (from Git Bash).
    GitHub `windows-latest` runners have this component: `release.yml` builds ARM64 in CI.

## Application

**Saved-PDF inspection (0.34.0):** the bounded dictionary inventory includes unreachable objects and
does not examine payloads, verify signature integrity or establish file safety/conformance. Partial
counts stay explicit. Save/reload is required for changed source files; native UI/Narrator qualification
remains separate. [Usage and limits](PDF_INSPECTION.md).

A1. **Fluent icons**: read from `C:\Windows\Fonts\SegoeIcons.ttf` (or `segmdl2.ttf`). On a Windows without these
    fonts (rare case: stripped-down LTSC), the icons are replaced by text symbols, less elegant but legible.
A2. ~~Page-by-page display~~ **Resolved (SUNI-012b)**: continuous scrolling by default, single page as an option (F5).
A3. ~~High-zoom rendering limited to 16,384 px~~ **Resolved for the screen (SUNI-012b)**: tiled rendering up to
    3200% (rendered page side <= 131,072 px). Also fixes a latent crash: a page rendered in one
    piece could exceed the GPU's maximum texture size (`egui_glow` panic if the driver limits to
    8192 px); no texture exceeds `max_texture_side` any more. The 16,384 px limit still holds for
    `Document::render_page`, image export and `sunipdf-cli render` without `--region`.
A4. **Encryption and independent QA**: AES-128/256 is written by lopdf and reopened by PDFium; 40-bit RC4 is covered
    by a fixture. In 0.9.1, the shared helper explicitly writes AES-256 `/Encrypt /Length 256`, previously omitted and
    reported by qpdf, including after re-encryption. Synthetic qpdf checks and qpdf-generated AES fixtures are added
    to CI. Current outcomes are in `PROJECT_STATUS.md`; this is not a real-world/hostile PDF corpus.
A5. **The open dialog (`rfd`) is modal and blocks the UI loop** for the duration of the choice:
    native Windows behaviour, acceptable.
A6. **Content editing: editing an existing text reuses its original font.** If that font
    (often an embedded subset) does not contain a typed character, PDFium substitutes or omits it.
    Workaround in published 0.21.0: delete the object and add a new one (Helvetica, Latin accents supported).
    **Published in 0.21.1:** keep that original font, atomically refuse
    unsupported text and reopen the correct draft. This fix is absent from 0.21.0 packages; see [executed checks](PROJECT_STATUS.md).
A7. **Rotation and pivot**: earlier free/group rotation remains. 0.30.0 remembers the selected
    rotation pivot so asymmetric paths do not acquire a new bounding-box centre at each turn; release-source regression gates passed.
    Corner resizing handles remain unavailable for non-orthogonal objects to avoid distortion. See [scope](RELEASE_0.30.0.md).
A8. **Reordering and clipboard**: Z-order changes merge page content streams before reordering. Existing grouped
    duplication/paste uses form XObjects and does not turn their contents into editable text; group color/opacity
    controls remain limited. 0.30.0 adds explicit **Paste editable** for supported native objects,
    with one Undo and atomic refusal of unsafe resource graphs, without silent grouped fallback. Release-source regression gates passed;
    [scope](RELEASE_0.30.0.md). The session clipboard and page-bound grouped path still have their existing limits.
A18. **Font formats, rights and reuse**: 0.30.0 replaces OEM `reg.exe` font-name discovery with
     Unicode Windows discovery, selected static TrueType/CFF1 TTF/OTF/TTC/OTC faces and eligible font subsetting.
     Embedding flags are checked before mutation; no-subsetting retains the complete selected face. Identical font
     programs can be reused after Undo/Redo/reopen. Variable/CFF2/color fonts, damaged faces, unsupported shaping
     and unavailable glyphs are refused. Size, face and glyph limits apply; internal rendering alone does not
     validate other readers. Runtime and documented independent-reader gates passed; [exact policy](FONT_EMBEDDING.md).
A19. **Crop hides, it does not erase**: the content outside the frame stays in the file (this is not
     redaction; stated in the dialog). Mouse cropping in the preview is implemented for 0.10.0; numeric margins remain available.
A20. **Text editing and paragraph preview**: ordinary inline input still uses the interface font; its draft
     display is not a PDF layout proof. 0.30.0 adds bounded multiline paragraph reflow with actual
     native font metrics, Preview then explicit Apply, draft preservation and one Undo. Unsupported fonts/text/
     graphics states are refused. General document reflow and bidirectional/shaping editing remain unsupported;
     release-source regression gates passed. [Scope](RELEASE_0.30.0.md) and [font limits](FONT_EMBEDDING.md).
A21. **Page resizing**: 0.30.0 optionally scales supported annotation/link geometry with page
     content. Malformed geometry and unsupported appearances cause atomic refusal rather than silently leaving
     misaligned annotations. This is bounded support, not every PDF extension; release-source regression gates passed. [Scope](RELEASE_0.30.0.md).
A22. **Image opacity**: 0.30.0 reads supported existing opacity and exposes an unknown state
     instead of carrying a previous object's value. Replacement is explicit. Complex masks/graphics states are not
     a universal editable alpha value; release-source regression gates passed. [Scope](RELEASE_0.30.0.md).
A23. **Displayed angle of objects on a rotated page**: an object's angle is that of its content on screen;
     a rectangle drawn in the PDF on a `/Rotate 90` page is therefore reported at 90° (CLI `objects`, Angle
     field). Corner handles remain available (multiple of 90°).
A24. **Object clipboard and system clipboard**: Ctrl+V only pastes objects if the marker written
     by the copy (the objects' text, or "[SuniPDF: n object(s) copied]") is still in the Windows
     clipboard; the Edit menu, the palette and the object bar paste without this condition. The object
     clipboard lives in memory for the session (lost when the application closes).
A9. **Engine tests serialized**: PDFium cannot be reinitialized within the same process after
    `FPDF_DestroyLibrary`; the tests share a single engine that is never destroyed (`tests/engine.rs`).
A10. **Distribution validation**: current tests, CI, package and release evidence are recorded in `PROJECT_STATUS.md`.
     Published 0.10.0 passed eight real portable update/failure/rollback scenarios and two disposable-runner MSI
     baselines covering installation/transactional rollback/upgrade/downgrade rejection/uninstall, preserving user data.
     Interactive flows, ARM64 hardware and physical printers remain separate checks.
A11. **All tools: 14 tools implemented for 0.9.0.** OCR, field creation and certificate signatures now have
     real commands. Automated and hardware coverage is recorded in `PROJECT_STATUS.md`.
A12. **Hub interface verified without automated pixel rendering**: no software renderer nor `egui_kittest`
     in the dependencies. The tests cover the data (table, registry, glyph contrast), tool
     switching, row geometry, the AccessKit tree and the focus ring (egui pass without a window); the
     actual rendering is checked through screenshots (`scripts/dev-screenshot.ps1`). The fallback glyphs (without Segoe
     Fluent Icons) of the new icons are not visually verified (see A1).
A13. **Tool bar in a narrow window**: the label widths are estimated before drawing; below the
     required width, icons only, then compact mode (name and "Leave tool" without label) and
     horizontal scrolling of the buttons (floating scrollbar, on hover; the vertical wheel does not
     scroll, Shift+wheel does). **Scrolling not demonstrated**: at 100% scale and at the minimum window
     width (720 px), the longest bar (Edit) fits in compact mode; scrolling is only
     reached with a larger Windows display scale and has been verified neither by a test nor by a
     screenshot.
A14. **Keyboard**: after a keyboard activation (Enter) of a hub row or button or of the quick
     access (Open, Save, Undo, Redo), focus is released so that the single-letter shortcuts stay
     active; Tab therefore starts again from the beginning. Standard egui buttons (tab bar, menu)
     only show a discreet focus hint; hub rows, cards and buttons have a 2 px ring.
     Esc acts in order (one Esc, one effect; `workspace::EscState::action`, tested): open menu or drop-down
     list, field or search, selected object, active tool. Two Esc received in the same
     frame (impossible from the keyboard, possible by injection) can still produce two effects.
A15. ~~Right rail in a short window: overlap~~ **Fixed (SUNI-012b)**: below ~700 px high, the top
     group (panels, view toggle, search) scrolls (wheel on hover) instead of overlapping the navigation;
     its last icons are then hidden until scrolled. Only whole icons are
     shown and a thin scrollbar, visible without hover, signals the rest (1000 x 560 screenshot).
A16. ~~CLI variable help for `stamp` and `page-numbers`~~ **Fixed in 0.18.0**: the existing clap help
     template retains literal `{n}` / `{total}` without command changes. Both commands' `-h` / `--help` regression
     and four staged/public help checks passed.

A17. ~~"Modified" indicator after undo~~ **Fixed (review-fixes, D052)**: save point in
     the history; lost only if one edits after an undo or if the saved snapshot leaves the 50
     levels (the document then stays "modified" until the next save).
A29. ~~Extraction/split removed PDF protection~~ **Resolved in 0.11.0**:
     copies retain original encryption, passwords and permissions through the shared protected writer (D097 replaces
     D046). Output at the resolved source path is refused. CLI reports `encryption_removed: false` and adds `encrypted`.
     Existing document-permission policy is unchanged; current validation belongs in `PROJECT_STATUS.md`.
A30b. **AES-256 and passwords**: the password is SASLprep-normalized before encryption (special spaces
    mapped to spaces, ligatures decomposed), then limited to a valid UTF-8 prefix of 127 bytes. SuniPDF applies
    the same transformation on opening; another reader may require this effective form (for example
    "pass word" for an entry with a non-breaking space). Since 0.8.0, an opening password that becomes empty is
    rejected; restrictions require distinct effective passwords.
A30. **AES-128 and passwords**: only printable Latin-1 is accepted (0x20–0x7E, 0xA1–0xFF except 0xAD;
     accented letters included). PDFDocEncoding-specific symbols outside Latin-1 (€, œ, Œ, ligatures,
     typographic dashes and quotes…) are refused (`password_not_latin1`) although lopdf encodes them: PDFium
     receives the password in UTF-8 and only retries in Latin-1, so SuniPDF would not reopen the file
     (verified by `review_aes128_latin1_passwords_round_trip_through_pdfium`). Choose AES-256 for these
     passwords. AES-128 uses the first 32 encoded bytes, padded according to the PDF format; since 0.8.0,
     owner/user differences are checked on these effective bytes.
A31. **Images → PDF**: published 0.17.0 embeds JPEGs in all eight EXIF orientations directly without
     recompression and preflights a cumulative non-JPEG raster estimate of 512 MiB before allocation:
     `sum(max(decoder.total_bytes(), width × height × 4))`. Arithmetic is checked, physical page dimensions must
     be finite/positive and loading repeats checks for simple file changes (D108). Local workspace, qpdf/corpus,
     independent debug/staged/public conversion checks, Windows CI, same-source ARM64 compilation, upgrades and
     production updater passed; evidence and remaining limits are in `PROJECT_STATUS.md`.
     PNG, TIFF, BMP, GIF and WebP still decode with quality/transparency preserved and are stored as compressed
     pixels (Flate); the PDF can be larger than the original. The estimate is not a process/native RSS ceiling
     or a limit on compressed JPEGs; decoder/PDFium/output overhead and machine memory remain separate constraints.
     A CMYK JPEG embedded as is is converted to RGB by PDFium at rendering (pure red → 238,29,35):
     normal conversion of a DCT stream, not an inversion.
A32. **Added text**: a tab counts as four spaces (no PDF font has a tab glyph).
     The Comments panel names 17 annotation subtypes; the others (sound, video, 3D, watermark,
     redaction…) are shown as "Other annotation".
A33. **Save point**: "modified" clears when undoing back to the saved state, unless an
     edit followed an undo (the saved state is then unreachable) or the saved snapshot
     has left the 50 history levels (D052).
A34. **Properties and PDF/A**: 0.9.1 preserves PDF/A identification (`pdfaid:part`, `pdfaid:conformance`) and unrelated
     XMP properties/namespaces when updating the four standard editable properties. Malformed XML, DTDs and unreadable
     streams are clearly refused, rather than silently replacing the packet. Preserving identification does not certify
     PDF/A conformance. A dedicated conformance validator and broader PDF/A corpus remain future work; current executed checks
     are recorded in `PROJECT_STATUS.md`.
A25. **Continuous view: limits (SUNI-012b)**.
     - Selection follows PDFium logical order across pages and scrolls near viewport edges; intended order
       of arbitrary untagged columns is not inferred. Text markup keeps per-page rectangular gestures.
     - Scrolling **without animation or inertia** for "go to" and the keyboard (direct jump).
     - **Pinch** (touchpad or touch screen): same path as Ctrl + wheel (`zoom_delta`), untested
       (no test on this machine).
       Ctrl + wheel verified by screenshot (`dev-screenshot.ps1 -Wheel`).
     - A **tile rendering already started is not interrupted**: when scrolling fast, the cancellation latency is
       the duration of one tile (measured: 1.2 ms per 1024² tile on `many-pages.pdf`, more for a heavy page).
     - Tile stitching: glyphs crossing a tile edge may differ by one anti-aliasing pixel
       from the one-piece rendering (121 pixels out of 1.9 million measured, D044).
     - Two-page/Book views share the existing caps; view-only rotation/full screen/per-tab history are implemented for 0.10.0. Editing returns to upright view; native interactive coverage remains separate.
A26. **"Fit" in continuous scrolling on a document of mixed sizes**: the zoom is shared by all the
     pages and fits the **largest** page (D041); a portrait page among landscape pages is therefore
     narrower than the screen. In single page, the fit follows the current page. Two-page/Book fit uses the
     widest spread and tallest page at one shared zoom.
A27. **Very long documents**: positions in `f32`; the zoom is capped so that the total height stays
     <= 8,000,000 logical px ("Maximum zoom reached for this document" message). A 5000-page Letter
     document is thus limited to about 150%.
A28. **View memory**: bounded but notable. Page textures 192 MiB (excluding screen textures), engine
     cache 256 MiB (tiles and thumbnails), 256 thumbnail textures at most in both the sidebar and Organize
     grid. Pending thumbnail requests also have a 256-entry cap; stale results are discarded.
     These caps are regression-tested on a 5,000-page grid. The following measurement predates 0.8.3 and
     covers the reading view: measured in a debug build, 500 pages,
     300 "Page Down" then End then 300 "Page Up": private memory stable around 810-830 MB (WorkingSet
     438 MB) after 175 MB at opening; no growth beyond (NVIDIA OpenGL driver, system copies of the
     textures included). Caps as constants (`view.rs`, `worker.rs`), no user setting.
A35. **Visual signature = image without cryptographic proof**: drawn, typed or imported, another editor can copy or delete it.
     CLI `add-image` reports `digital_signature: false`. CMS/PFX signing is a separate Use a certificate
     workflow (A44); the application warns before rewriting a signed PDF.
A36. **Saved visual signatures**: 0.30.0 uses current-user DPAPI `.dpapi` files with a private
     user-SID ACL. Legacy bounded PNG entries migrate atomically only after persisted encrypted bytes have been
     decrypted and compared; failures preserve the originals and show the remaining plaintext risk. Saving is
     optional; two-click deletion removes the selected entry without securely erasing sectors/backups. Unreadable
     files are retained; another user or a lost/reset profile cannot recover them from ciphertext alone. Anyone
     operating the signed-in session can still reuse an image. Release-source regression gates passed; [recovery and bounds](SIGNATURE_SECURITY.md).
A37. **Pad and import**: 0.35.0 adds optional quadratic-curve smoothing, adaptive light-paper cleanup and prepared-pixel preview binding.
     Raw points and source files are retained; drafts accept 20,000 points and 256 strokes, with placement blocked at the limit.
     Pen pressure remains unavailable; faint ink, broad strokes and textured backgrounds require preview review. Native interaction and customer photos remain unqualified;
     imported images limited to 16,384 px per side and 512 MiB decoded (refused when reading
     the header, same per-image 512 MiB bound for Insert Image, Replace Image and Images → PDF), then reduced
     to 1600 px per side; the placed date is kept inside the page over an **estimated** width (0.556 em per
     character, Helvetica digits), not measured. **0.13.0** adds typed name/initials with explicit
     native preview (one line, 256 characters, 1600 px/side; Times italic default, existing standard/system
     TrueType glyph/embedding checks), and current/all/selected-page placement. Preview is invalidated by
     edits, mode changes or dialog/document cancellation. Multi-page marks share a display-space centre in
     PDF points, clamp to visible page edges and shrink only when oversized; no relative-position scaling.
     Cancellation is checked between targets, never inside a native PDFium call; large batches also require
     a document snapshot and temporary in-memory copy. One undo restores the whole operation; source files
     change only on Save. CLI fitting is opt-in through `add-image --pages`; old `--page` semantics stay unchanged.
     Published in 0.13.0; executed checks and remaining limits are in `PROJECT_STATUS.md`. Full library:
     the dialog body scrolls and its floating bar gets a 10 px gutter
     (the content tightens by as much on the frame following the bar's appearance).
A38. **AcroForm forms**: XFA is read, never filled/flattened; JavaScript, calculations and actions never run.
     0.10.0 implements text/checkbox/radio/combo/list creation, XFDF export and strict bounded JSON/XFDF/FDF value import.
     Import validates every field/value before changing anything; unknown or rejected values refuse the whole GUI import.
     Panel changes remain pending until Apply or Save, then use one undo/reload; Discard restores the current values.
     On-page confirmation is immediate. Space/Enter activate focused widgets; choice popups receive keyboard focus.
     Names are unique, without dots/control characters, numeric rectangles stay inside the page (minimum 4 points).
     Radio buttons are arranged vertically; option labels are exposed in the panel, not inserted as page text.
     FDF supports normal xref tables and text/name/array values only; streams, actions, attachments, rich text and
     FDF export are unsupported. XML DTDs, processing instructions, unknown elements, duplicate names and excessive
     size/depth are refused; referenced document paths are never opened. Input cap: 16 MiB.
     0.11.0 delivers editable-combo free text in the panel/on page; local, CI and packaged checks passed.
     0.12.0 delivers editable combo creation (predefined choices still required) and radio `/Opt` mapping:
     export values are kept separate from appearance states. Malformed/duplicate mappings keep the affected field read-only;
     so do malformed raw `/Kids`, duplicate widget references and non-UTF-8 appearance names. Unchanged read-only
     values can be exported and imported, including literal `Off` or empty strings. Filling another field preserves
     raw `/V` and `/AS`; PDFium may regenerate a missing appearance and remove inferred read-only status once valid.
     No-`/Opt` radios retain state-name values. Local, CI, package/public and upgrade checks passed. Existing fields' editable flags cannot
     be changed. Mouse-drawn creation is published in 0.19.0 (D110): reuse the five existing types
     and numeric dialog, retain settings, clamp finite rectangles to one page (minimum 4 pt per dimension), then
     require explicit Create Field/one Undo. Document revision/tab/tool changes cancel stale placement; drawing does
     not edit existing fields. If Single-page navigation hides the original page during a drag, release uses the
     last known rectangle and reopens the numeric dialog; no field is created without explicit Create Field.
     Local tests, formatting and Clippy passed; CI/upgrade, staged/public package and production updater checks passed. 0.19.0 was stable Latest at publication.
     Rich text becomes plain text; password fields never stored/exported.
     Published 0.21.0: partial flattening can leave orphan fields, and non-Link annotations without appearances may disappear.
     **Published in 0.21.1:** preserve raw non-widget graphs, attachments and shared references
     when no usable appearance exists, while widgets/drawable appearances still flatten; excluded count
     and scratch-document atomicity are retained. Malformed entries or graphs beyond the 32-level copy limit
     refuse the operation without changing the active PDF. Orphan-field behavior is separate. Unicode
     field appearances can use a non-embedded system font, so other-machine rendering can differ. Narrator and
     third-party-reader interoperability remain unverified. Current executed evidence belongs in `PROJECT_STATUS.md`.


A39. **Advanced annotations**: 0.30.0 adds actual-preview embedded-font FreeText, polygons,
    polylines, clouds, standard stamps, attachments and whole-stroke ink erasure. Uniform text style only;
    unsupported shaping is refused. Rich appearance restyle/resize and arbitrary foreign rich-text data remain
    limited. Validation is pending; [supported inputs and bounds](RICH_ANNOTATIONS.md).
    **Historical 0.9.0–0.21.1 baseline follows; its limitations describe those releases:**
    **Advanced annotations (annotations-advanced slice, D067–D076)**: **free text in Helvetica WinAnsi** only
    (Latin accents, € and common punctuation; any other character shows as "?" in the appearance, `/Contents` keeps
    the exact text; no bold, italic, rich text `/RC`, CJK or right-to-left) and the wrapping width is computed with the
    Helvetica table (2 % margin); through 0.17.0, **one ink stroke = one annotation** and every operation (stroke, move, style)
    costs a full snapshot + lopdf + reload round trip: imperceptible on an ordinary PDF, about a second on a file of
    tens of MB; **reply threads and statuses** written according to ISO 32000-1 but **never tested in Acrobat, Foxit
    or Chrome** (replies without icon: empty appearance + `F 4`), no "Marked" state model; **detailed reading** of
    annotations (panel, selection) is requested on demand, per page, and re-reads the whole document. Named/XYZ
    navigation is delivered in 0.11.0 (A49). **0.15.0** adds existing-link target replacement via
    UI/CLI (D106): a safe URI or physical-page Fit, explicitly replacing detailed views. Area/style/names/comments,
    replies and unrelated document data remain; `A`/`Dest`/`AA` are replaced together. ReadOnly (64), Locked (128),
    LockedContents (512), negative/malformed or unreadable indirect flags refuse editing. Unsupported targets
    prefill an empty address, not page 1. One Undo preserves geometry/navigation; edits and tab switches close
    stale dialogs. Published in 0.15.0; local, CI, upgrade, staged/public package and production updater checks
    passed; evidence and remaining limits are in `PROJECT_STATUS.md`. Launch / JavaScript / GoToR
    actions are never executed; **resizing and styling reserved for
    SuniPDF's own annotations** (prefixed `/NM`): those from another producer can be moved, commented and deleted
    (highlights and notes can also be recoloured). **Published 0.16.0** adds dashed borders only for rectangle,
    ellipse, line and arrow annotations. UI/CLI choose `[6,3]` points; actual custom vectors survive selection,
    recolouring, move/resize and undo unless the checkbox is explicitly toggled. Phase is zero, fill and arrowheads
    stay solid. Nonempty patterns on ink/free text/link/markup are refused. Unsupported/malformed `BS` on a SuniPDF
    annotation refuses appearance-regenerating edits rather than resetting it. At most 16 finite nonnegative values,
    each at most 1000 pt, positive values at least 0.1 pt and positive total; conservative work cap 100,000 segments.
    An explicit `D []` is valid solid; missing `D` defaults to `[3]`. Malformed own-border Move/Resize/Style
    controls are disabled. Shape comments can keep the existing appearance, subject to normal edit permissions;
    free-text comments with malformed borders remain refused. Preview keeps valid subpixel patterns and omits
    invalid/excessive nonempty strokes rather than showing a solid fallback, under the same work cap.
    No custom-pattern/phase editor. Local/CI/upgrade, staged/public package and production updater checks passed;
    evidence and remaining limits are in `PROJECT_STATUS.md` (D107).
    **Delivered in 0.18.0 (D109).**
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
    Native pen/touch, interactive dialogs, printers, Narrator, ARM64 hardware and third-party readers remain unverified. Packages are unsigned; unfinished ink drafts have no crash recovery.

    **Published in 0.20.0 (D111):** create/edit FreeText in an anchored multiline page editor with explicit
    Add/Save/Cancel, retaining existing style controls, core validation/protection and one Undo. Original page
    pinned; zoom allowed; conflicting navigation blocked until confirmation/cancellation. Pending operations,
    concurrent drafts and stale document/tab/tool/workspace state are guarded; validation/worker failure
    retains text. The editor draft is memory-only and excluded from crash recovery. Helvetica WinAnsi,
    plain text and downward wrap growth remain the same; no embedded-font/rich-text expansion. Local review,
    workspace, formatting, i18n and Clippy passed; CI/upgrade, staged/public packages and production updater
    verification passed. 0.20.0 was stable Latest at publication. Native interaction/reader QA is separate.

    No eraser, polygon, cloud, stamp or attachment; caps:
    64 strokes of 4,000 points, width ≤ 50 pt, link area ≥ 6 pt; **verified in screenshots** (`dev-screenshot.ps1`): ink,
    selection of a SuniPDF annotation (frame, four handles, floating bar) then moving and resizing with the mouse,
    moving another producer's annotation (frame without handles), dialogs (text box, link, address confirmation),
    Comments panel with a thread, light and dark themes; **not replayed in screenshots**: the text box placed after
    its dialog, editing by double-click, keyboard arrows, resizing an ink stroke, a line or a text box (all covered by
    the engine tests and the pure tests of `annot_adv.rs`); synthetic structural qpdf checking is added for 0.9.1
    (A4); reader interoperability remains unestablished.
A40. **Redaction**: 0.30.0 adds native before/after preview, fill/overlay, persistent marks and
    shared sanitization. Validation is pending; [exact guarantees and bounds](REDACTION.md). Applied output removes
    covered content by full rewrite and residual checks, with these limits:
    1. **Granularity**: safe opaque Base14 Latin text can retain selectable unredacted glyphs at native positions.
       Complex fonts, shaping, OCR, images, clipping and nested objects retain the raster-patch fallback; their
       visible remainder may lose selection/search. Unsafe cleanup graphs are refused atomically.
    2. **Text in a scanned image without OCR**: invisible to "Find Text to Redact…" (draw the area). An OCR layer
       offset from the image can escape a drawn area: the "Convert redacted pages entirely to images" option removes
       it in every case.
    3. **Embedded fonts**: if the font of the redacted text is still used elsewhere, its glyph subset stays in the file
       (it reveals characters that were used, not their order or position).
    4. **Hidden data**: default cleanup removes supported metadata, structure text, attachments, scripts and
       outline/destination metadata through the verified sanitizer. Turning it off retains unrelated hidden data,
       which may duplicate redacted content. Unmarked page content is not a global text-erasure operation.
    5. **Form fields**: a field with a widget under an area disappears everywhere (all its widgets, all pages); `/XFA`
       is then removed.
    6. **Accessibility tags**: removed for the whole document when "hidden information" is checked (default); kept
       otherwise, with the risk that an alternate text describes the redacted content.
    7. **Digital signatures**: any existing signature becomes invalid (full rewrite).
    8. **Memory and disk**: undo and suspended tabs keep previous states in RAM. Successful redaction immediately
       refreshes the current recovery snapshot (or removes it if refresh fails), but older crash-session copies may
       still contain unredacted content. The original, backups and deleted copies are not securely wiped from storage.
    9. **Remaining assumption**: PDFium object bounds enclose the object's ink. For text they are enlarged by 30 % and
       checked at run time (residual text under an area = error, document unchanged); for images and drawings they
       are geometric.
    10. **Refused documents**: a content stream that does not decode strictly (lopdf) gives `redaction_incomplete`
        without changing anything; retry with "page as image".
    11. **Deleting an object with the Edit tool is not redaction**: PDFium may leave the object orphaned or the
        resource in the file; only the Redact tool sanitises and rewrites the document.
    12. In-memory drafts/candidates are invalidated by document changes. **Keep unapplied PDF marks** persists
        standard `/Redact` annotations recovered on opening; content remains present until Apply. Overlay fonts/text
        cannot be persisted in this slice. Removing a draft rectangle does not delete an existing persisted mark.
        Drawn areas have no individual accessibility node; their count is announced in the status bar.
    13. Not verified: real scans/heavily tagged PDFs and pages with tens of thousands of objects (lopdf's
        `prune_objects` is quadratic). Synthetic qpdf checking does not replace this corpus (A4).


## 0.9.0 feature limits

Executed validation results are tracked in `PROJECT_STATUS.md`.

A41. **Recovery**: snapshots every 60 seconds and on tab suspension can miss more recent changes after a forced stop.
     Unconfirmed dialog text is not a PDF edit. Protected snapshots stay encrypted, but filenames/paths and page counts
     are present in JSON metadata. No passwords are saved; restoring requires the password again. Recovery never replaces the original.
A42. **Windows printing**: raster output up to 300 dpi in bounded bands, fitted to the printable area; no vector output
     or professional colour guarantee. The driver handles copies/orientation. A running native operation can delay
     cancellation. Actual Microsoft Print to PDF output was reopened and checked with unsaved edits while source
     bytes/objects stayed unchanged. Physical printers and the interactive printer dialog remain unverified.
A43. **Local OCR**: English/French only, fast Tesseract models; accuracy varies by scan and language. Pages containing
     any extractable text are skipped, even if that text layer is poor. Images capped at 16 Mpx and 8192 px/side,
     TSV at 16 MiB, 50,000 words/page, 120 seconds/page; very large pages refused. No semantic table/column reconstruction.
     Cancellation/failure commits no pages; success is one undo step. Other readers still need interoperability checks.
A44. **Digital signatures and trust**: saved-byte detached CMS integrity and incremental new-copy PFX/P12
     signing remain separate from identity trust; encrypted signing is refused. 0.30.0 adds Windows
     chain/revocation and RFC3161 signature-timestamp verification. Retrieval is cache-only by default and requires
     explicit consent for each online verification; unavailable/unknown revocation never establishes trust.
     Strong digests, EKU/time/token binding and TSA trust are checked. PDF signing time remains declarative;
     timestamp creation is not added. Embedded DSS is reported as unverified, not authenticated archival LTV.
     AATL/EUTL, PKCS#11, qualified-signature assessment and third-party-reader certification remain unsupported.
     Release-source regression gates passed; [guarantees and limits](SIGNATURE_SECURITY.md).
A45. **0.10.0 image compression**: lossy; only supported 8-bit DeviceRGB/DeviceGray images up to 32 million pixels
     are decoded. Mask/soft-mask resource graphs, profiles, custom Decode/DecodeParms and other unsupported images
     remain unchanged. If the document would grow, the exact input bytes are returned. Maximum-side/JPEG controls
     do not guarantee a target file size. Signed or modification-restricted PDFs are refused; output is a new copy.
A46. **0.10.0 batches**: sequential compression or offline English/French OCR, at most 128 PDFs and 256 MiB/input.
     Existing outputs are never replaced; completed files remain if a later file fails or cancellation occurs.
     Password prompts are not provided per batch file; protected files that cannot open without a password are refused.
     Cancel takes effect at engine checkpoints; a codec or native call already running may delay it.
A47. **0.10.0 data-safety/corpus coverage**: injected storage failures and a killed partial-save child exercise original
     preservation. They do not simulate every physical disk/power-loss failure. Four public NIST PDFs supplement the
     synthetic corpus: 31 synthetic qpdf checks and all four real documents (744 pages) passed for 0.10.0.
     They do not establish broad customer/hostile/large-file compatibility; executed evidence is in `PROJECT_STATUS.md`.

## 0.11.0 published changes and limits

These additions are published in [0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0). Local, CI, package and upgrade regression
gates passed; public-download/live-updater checks also passed; evidence and remaining limits are recorded in `PROJECT_STATUS.md`.

A48. **Page labels**: optional navigation metadata, separate from printed numbering. Reading shipped in 0.11.0;
     six-style author/reset in 0.14.0. Blank/unreadable/control-containing labels or labels above 4,096 UTF-8 bytes
     fall back to physical indices; exact unique labels take precedence, `#N` is physical, duplicates require a
     physical index. New prefixes remain single-line/256 UTF-8 bytes and start numbers positive. 0.30.0
     rebases semantic labels by page provenance across merge/extract/reorder/delete/insert, without changing printed
     text. Malformed/unsafe trees refuse structural edits atomically; duplicates remain explicit. Reset removes
     metadata, not old disk copies. Release-source regression gates passed; [scope](RELEASE_0.30.0.md), [structure limits](BOOKMARKS_AND_PAGE_STRUCTURE.md).
A49. **Destinations**: 0.30.0 adds named/XYZ/Fit/FitH authoring and bounded raw resolution that
     distinguishes null axes from explicit zero, including FitH. Retained axes/zoom use the current viewport;
     coordinates follow displayed CropBox/rotation and explicit zoom is bounded. Cycles, ambiguity and malformed
     local destinations are refused; unsupported fit modes retain page-only navigation. Existing unsupported
     actions remain inert data and are never executed. Release-source regression gates passed; independent-reader and accessibility
     checks are distinct. [Scope](RELEASE_0.30.0.md) and [preservation limits](BOOKMARKS_AND_PAGE_STRUCTURE.md).
A50. **Form-read failures**: an error is kept distinct from an empty form, and the panel offers an explicit retry.
     Pending drafts and staged values are retained while the read fails. A waiting import/clear operation resumes
     only after a successful read; repeated automatic retries are suppressed.
