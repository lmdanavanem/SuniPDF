# Feature matrix: expected of a full PDF editor / SuniPDF

[Français](FEATURE_MATRIX.fr.md)

## 0.32.0: object ordering (stable Latest)

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

**0.20.0 published; Latest at the time:** create/edit FreeText directly on the page, with an anchored multiline editor,
existing style controls and explicit Add/Save/Cancel (D111/A39). Local review, 468 workspace tests,
formatting and 912 EN/FR keys passed; Clippy, CI/upgrade, same-source ARM64, staged/public packages and
the production updater passed. **0.20.0 was stable Latest at publication**; evidence and limits are in `PROJECT_STATUS.md`.

**0.19.0 published; Latest at the time:** mouse-drawn placement for existing form-field types, with numeric review and
explicit Create (D110/A38). Implementation, local tests, formatting and Clippy passed; release gates passed.
**0.19.0 was stable Latest at publication**.

"Expected" column: capability expected of a full PDF editor (**Declared** = common in the editors on the market,
**Partial** = announced as incomplete by the project, **Absent** = announced as missing. Not verified by execution.

SuniPDF statuses: **Done** = implemented **and** covered by executed tests; **Delivered in 0.30.0** =
published within the documented limits and covered by the executed release gates; **In progress**;
**Planned (Pn)** = roadmap phase; **Out of scope**. P1 Roadmap items and Suggestions are deferred.

The 0.9.0 features below have automated regression coverage. `PROJECT_STATUS.md` records the executed checks;
these results do not establish physical printer, ARM64 or third-party reader compatibility.
The 0.9.1 workspace, qpdf and real portable/MSI regressions passed; package/publication outcomes are recorded there.

[0.10.0 is published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) with the requested P1/P2 lot.
The 362 workspace tests, 31 synthetic qpdf checks, four real NIST documents (744 pages), eight portable scenarios,
two MSI baselines and all six public assets passed verification; evidence is recorded in `PROJECT_STATUS.md`.

[0.11.0 is published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) under `lmdanavanem`; Latest at the time.

The expanded 0.11.0 source passed 390 workspace tests (zero failed, seven default ignored), formatting/Clippy with
`-D warnings`, 858 matching EN/FR keys, 44 qpdf checks across 42 distinct PDFs and four real NIST documents
(744 pages; 91.59 seconds, peak child memory 329,347,072 bytes). [Final Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) and
[same-source ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) passed, as did nine real portable scenarios from
0.8.2/0.8.3/0.9.0/0.9.1/0.10.0, three MSI baselines (0.8.0/0.9.1/0.10.0) and all six staged asset checks. Packaged
checks include owner-opened AES-128/256 copies, decoded content/qpdf, page labels, metadata, OCR, forms and compression.
Immutable release source: `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`. All six unauthenticated public downloads match API/sidecar/staged hashes; enhanced checks passed again on public
binaries. The production live updater verified the 19,687,375-byte Latest x64 ZIP without installation. Evidence
and remaining limits are recorded in `PROJECT_STATUS.md`; published 0.10.0 source/packages remain unchanged.

[0.12.0 is published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (Latest at the time), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation passed: 398 workspace tests (zero failed,
seven default ignored), formatting/Clippy, 859 EN/FR keys, 48 qpdf checks and four NIST/NBS documents (744 pages;
59.68 seconds). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) and the
[same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) passed, plus
ten portable scenarios, four MSI baselines, all six staged/public files and enhanced binary checks. Verified files
are in `dist/`. The production updater verified the 19,699,323-byte Latest x64 ZIP without installation.
Hardware, interactive UI and reader limits remain in `PROJECT_STATUS.md`.

[0.13.0 is published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) under `lmdanavanem` (Latest at the time),
release `414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation passed: 410 workspace tests
(zero failed, seven default ignored), formatting/Clippy, 870 EN/FR keys, 53 qpdf checks and four licensed documents
(744 pages; 76.40 seconds locally), Windows CI and same-source ARM64 compilation, eleven portable scenarios and
five MSI baselines. Six staged/public files and enhanced binary checks passed, including protected signature
batches and both passwords. Verified copies are in `dist/`; the production updater verified the 19,730,578-byte
Latest x64 ZIP without installation. Detailed evidence and remaining limits are in `PROJECT_STATUS.md`.

[0.14.0 is published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) under `lmdanavanem` (Latest at the time),
release `414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation passed: 420 workspace tests
(zero failed, seven default ignored), formatting/Clippy, 890 EN/FR keys, 63 qpdf checks and four licensed documents
(744 pages; 53.91 seconds locally), Windows CI and same-source ARM64 compilation, twelve portable scenarios and
six MSI baselines. Six staged/public files and enhanced binary checks passed, including plain/protected label
authoring/reset, both passwords, unaffected labels and unchanged text/page PNGs. Verified copies are in `dist/`;
the production updater verified the 19,785,200-byte Latest x64 ZIP without installation. Evidence and remaining
limits are in `PROJECT_STATUS.md`.

[0.15.0 was published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) under `lmdanavanem` (Latest at the time),
release `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation passed: 429 workspace tests
(zero failed, seven default ignored), formatting/Clippy, 894 EN/FR keys, 68 qpdf checks and four licensed documents
(744 pages; 48.891 seconds locally), Windows CI and same-source ARM64 compilation, thirteen portable scenarios and
seven MSI baselines. Six staged/public files and enhanced checks passed, including plain/protected link retargeting,
both passwords, preserved link identity/area and unchanged page PNGs. Verified copies are in `dist/`; the production
updater verified the 19,803,226-byte Latest x64 ZIP without installation. Evidence and remaining limits are in `PROJECT_STATUS.md`.

[0.16.0 was published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) under
`lmdanavanem` (Latest at the time), release `414934190`, source `6cd7843734f3924e87033d8b05539d493ccbcb95`. Validation passed:
439 workspace tests (zero failed, seven default ignored), formatting/Clippy, 895 EN/FR keys, 73 qpdf checks and
four licensed documents (744 pages; 53.542 seconds locally), Windows CI and same-source ARM64 compilation,
fourteen portable cases and eight MSI baselines. All six staged/public files and enhanced package checks passed;
verified copies are in `dist/`. The production updater verified the 19,821,719-byte Latest x64 ZIP without
installation. Evidence and remaining hardware/interoperability limits are in `PROJECT_STATUS.md`.

[0.17.0 was published (Latest at the time)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) under
`lmdanavanem`, release `415074108`, source `5785de282e126fb4666b28fb83340d0e9c6e9acf`. Validation passed:
445 workspace tests (zero failed, seven default ignored), formatting/Clippy, 896 EN/FR keys, 74 qpdf checks,
four licensed documents/744 pages (48.187 seconds locally; 45.791 seconds in CI), Windows CI and same-source
ARM64 compilation, fifteen portable cases and nine MSI baselines. All six staged/public files and enhanced
conversion checks passed, including nine exact JPEG streams, oriented corners, PNG transparency, budget/DPI
refusals and unchanged sources. Verified copies are in `dist/`; the production updater verified the
19,825,743-byte Latest x64 ZIP without installation. Evidence and remaining limits are in `PROJECT_STATUS.md`.

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

## A. Reading

| Feature | Expected | SuniPDF | Note |
|---|---|---|---|
| Open a PDF, display a page | Declared | **Done** | PDFium; tests `crates/sunipdf-core/tests/engine.rs` |
| Password-protected documents | Declared (all revisions) | **Done** | RC4, AES-128 and AES-256 covered by engine/protection tests and reopened by PDFium |
| Page navigation previous/next/go to | Declared | **Done** (UI) | Keyboard + toolbar |
| Zoom, fit width/page, actual size | Declared | **Done** (UI) | Rendering at physical resolution (DPI); Ctrl + wheel anchored under the pointer, 10 to 3200% |
| Rendering off the UI thread | Declared | **Done** | Single engine thread, wanted-tile list replaced as a block, anti-staleness epochs |
| Multi-document tabs | Declared | **Done** (inactive documents suspended, interface state kept) | |
| Continuous display, two-page view, view rotation | Declared | Continuous/two-page/cover-first Book **Done**; view-only rotation **Done in 0.10.0** | Shared tile planner/cache, mixed sizes, odd last page and zoom regression tests |
| Tiled rendering, capped cache | Declared | **Done** (1024 px tiles beyond 4096 px / 8 Mpx, 256 MB LRU cache, 192 MB textures) | `Document::render_region`, pixel tests (regions, stitching, ×40 zoom); CLI `render --region` |
| Full-text search | Declared | **Done** | Ctrl+F, case, highlighting, CLI `search` |
| Text selection / copy in reading order | Declared | **Done**: logical character range, cross-page/reverse selection, double-click words, edge autoscroll | PDFium order; core/headless tests; arbitrary untagged column order not inferred |
| Thumbnails | Declared | **Done** | Virtualised panel, shared cache |
| Bookmarks (outline) | Declared | **Delivered in 0.30.0**: Add/rename/retarget/move/delete with Undo; bounded merge/extract preservation. [Supported structures](BOOKMARKS_AND_PAGE_STRUCTURE.md). | Bounds: 4,096 bookmarks / 32 levels; cycles refused. |
| Page labels | Declared | **Delivered in 0.30.0**: Semantic rebasing follows page provenance; duplicate labels explicit and printed numbers unchanged. Earlier six-style author/reset remains. [Scope](RELEASE_0.30.0.md). | Exact unique label or physical `#N` navigation. |
| Navigation history, reopened tabs | Declared | Recent files and per-tab 256-entry history **Done**; same-page/zoom history **delivered in 0.11.0** | Local tests passed |
| Reading mode / full screen | Declared | **Done in 0.10.0** | View mode is a shared preference; rotation/history are retained per tab; editing returns to upright view |
| Windows printing | Declared (limited fidelity) | **Done** (layout/range tests and actual Microsoft Print to PDF output reopened) | Native dialog, ranges/copies, bounded raster printing, progress/cancel; physical printer and interactive dialog unverified |
| Crash recovery, autosave | Declared | **Done** (recovery/worker/tab regression tests) | 60-second and tab-suspend snapshots; source unchanged; encryption retained/no passwords; restore Save As |
| Light/dark themes | Declared | **Done** | Design system `theme.rs`, toggle in the tab bar |
| Translatable interface | Declared (EN, JA + 22 catalogues) | **Done** (en-US default, fr-FR available) | Hot switching, existing preferences preserved |

## B. Editing and annotation

| Feature | Expected | SuniPDF |
|---|---|---|
| Notes, highlight, underline, strikethrough, shapes, ink, text boxes | Declared | **Delivered in 0.30.0**: Earlier markup retained; embedded-font FreeText, polygons/polylines/clouds, standard stamps and attachments with actual preview. Uniform text style, bounded refusals; [details](RICH_ANNOTATIONS.md). |
| Reply threads, comment status | Declared | **Done in SuniPDF**: Comments panel as threads (replies, status Accepted / Rejected / Cancelled / Completed, author, date, thread deletion), written according to ISO 32000-1; **Acrobat / Foxit compatibility not verified**; CLI `reply` / `review-state` |
| Group several freehand strokes | Declared | **Delivered in 0.30.0**: Grouped ink shipped in 0.18.0. Whole touched strokes can be erased, untouched style/comments retained; no partial-stroke erasure, idle commit or draft recovery. [Limits](RICH_ANNOTATIONS.md). |
| On-page FreeText creation and editing | Declared | **Delivered in 0.30.0**: On-page baseline shipped in 0.20.0; actual embedded-font preview and selected bold/italic faces/horizontal CJK now present. No span editor or unsupported shaping. [Details](RICH_ANNOTATIONS.md). |
| Adding text / images | Declared | **Delivered in 0.30.0**: Earlier text/images retained; Unicode font discovery, eligible static TrueType/CFF1 subsets and selected TTF/OTF/TTC/OTC faces with rights/reuse checks. Variable/CFF2/color fonts refused. [Formats/bounds](FONT_EMBEDDING.md). |
| Modifying existing text | Partial (weak in CJK/bidi) | **Delivered in 0.30.0**: 0.21.1 missing-glyph guard retained; bounded multiline paragraph preview/reflow with native metrics and explicit Apply. Unsafe fonts/text/graphics states refused. [Scope](RELEASE_0.30.0.md). |
| Object transformation | Declared | **Delivered in 0.30.0**: Earlier object operations retained; stable remembered rotation pivot, explicit opacity replacement/readback and unknown state. [Scope](RELEASE_0.30.0.md). |
| Multiple selection, cut/copy/paste of objects | Declared | **Delivered in 0.30.0**: Grouped paste retained; explicit **Paste editable** recreates supported native objects in one Undo. Unsafe graphs refused without silent grouped fallback. [Scope](RELEASE_0.30.0.md). |
| Page objects panel | Declared | **Done** (F3: drawing order, selection, bring forward/send backward, deletion, marker on hover); object/group drag-and-drop delivered in 0.32.0 ([guide](OBJECT_ORDER.md)) |
| Links | Declared | **Delivered in 0.30.0**: Earlier page/URI links retained; named/XYZ/Fit/FitH authoring and bounded raw resolution preserve null axes/zoom and displayed crop/rotation. Active unsupported actions never run. [Scope](RELEASE_0.30.0.md). |
| Watermarks, headers/footers, numbering | Partial | **Done**: text stamp/watermark (angle, opacity, position) and `{n}`/`{total}` numbering (UI + CLI `stamp`, `page-numbers`); image watermark and multi-zone headers planned (P3) |
| Custom stamps | Not documented | **Delivered in 0.30.0**: Standard preset annotations present; arbitrary custom stamp design remains planned. [Types](RICH_ANNOTATIONS.md). |

## C. Organisation

| Feature | Expected | SuniPDF |
|---|---|---|
| Rotate, delete, insert, move, blank pages | Declared | **Done** (UI + CLI `insert-blank` for blank pages) |
| Crop pages (CropBox) | Declared | **Done** (dialog in mm with preview, restore, CLI `crop`); mask only, not redaction; mouse crop preview delivered in 0.10.0 |
| Resize pages | Declared | **Delivered in 0.30.0**: Earlier size/content controls retained; optional annotation/link scaling with content, unsupported/malformed appearances refused atomically. [Scope](RELEASE_0.30.0.md). |
| Merge N PDFs with bookmarks | Declared | **Delivered in 0.30.0**: Bounded outlines, destinations, fields, layers and attachments preserved; incoming name collisions deterministic. [Limits](BOOKMARKS_AND_PAGE_STRUCTURE.md). |
| Split (every n / chosen pages) | Declared | **Done** (UI dialog + CLI `split`, `extract`) |
| Extraction preserving links, fields, layers, attachments | Declared | **Delivered in 0.30.0**: Earlier encryption/source guards retained; bounded graph preservation and omitted-page destination removal. XFA/unsafe graphs refused; documented independent reader gates passed. [Limits](BOOKMARKS_AND_PAGE_STRUCTURE.md). |
| Drag-and-drop reordering | Declared | **Done** (thumbnails and Organize view) |
| Named undo/redo surviving save | Declared | Undo/redo **Done** (snapshots, unnamed); survives save. **Delivered in 0.14.0**: preserve navigation through label/content Undo/Redo when page geometry is unchanged; executed checks passed (D105) |
| Comparison of two versions | Declared | Delivered in 0.31.0: local text/geometry/768-pixel comparison, explicit physical offsets, bounded original/revised/difference previews. [Limits](LOCAL_REVIEW.md). |
| Search across selected PDFs | Declared | Delivered in 0.31.0: explicit files, literal text, per-file failures, page excerpts/navigation and cancellation. [Limits](LOCAL_REVIEW.md). |
| Batch processing | Declared (CLI batch) | **Delivered in 0.30.0**: Compression/OCR shipped in 0.10.0; Office batches now present, 1–128 inputs, 64 MiB/input, 120 s/conversion, cancel/new copies. [Limits](OFFICE_CONVERSION.md). |

## D. Conversion and OCR

| Feature | Expected | SuniPDF |
|---|---|---|
| PDF → images | Declared (CLI render) | **Done** (UI 150 dpi PNG export, CLI `render`, `export-images`) |
| PDF → text | Declared | **Done** (CLI `text`) |
| Images → PDF | Declared (create) | **Done** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP). **Delivered in 0.17.0**: direct embedding for all eight JPEG EXIF orientations; 512 MiB cumulative non-JPEG raster estimate `sum(max(decoded bytes, width × height × 4))` before allocation, checked arithmetic and finite/positive page geometry (D108). Quality/transparency and callers retained; estimate is not a process/native RSS ceiling or compressed-JPEG limit. Local/CI/upgrade, independent debug/staged/public conversion and production updater checks passed; evidence and remaining limits in `PROJECT_STATUS.md` |
| PDF → Word | Partial (loses the layout) | **Delivered in 0.30.0**: Simple editable single-column visible text reflowed at 11 pt; original fonts/layout/pagination not reconstructed. Complex inputs refused. [Scope](OFFICE_CONVERSION.md). |
| PDF → Excel / PowerPoint | Absent / partial pptx export | **Delivered in 0.30.0**: Aligned text grids as literal XLSX strings; PPTX as 150 dpi page images with margins. No formulas, semantic table reconstruction or editable PPTX content. [Limits](OFFICE_CONVERSION.md). |
| Office → PDF | Absent | **Delivered in 0.30.0**: Guarded macro-free inputs, installed LibreOffice 26.2/26.8, isolated profiles and new-copy outputs. Unavailable explicit, no auto-install. [Formats/profiles](OFFICE_CONVERSION.md). |
| HTML → PDF | Absent | Planned (P5, optional) |
| Image compression | Declared | **Done in 0.10.0**: supported RGB/Gray images, JPEG quality/max-side controls, no-gain byte copy, masks/profiles retained, signed/restricted inputs refused |
| OCR | Partial (Latin, CC-BY-SA models) | **Done**: bundled offline Tesseract 5.5.3 / Leptonica 1.87, fast eng/fra models, invisible text, skip existing text, progress/cancel/one undo; core and real CLI recognition tests; CLI `ocr` |

## E. Security and signatures

| Feature | Expected | SuniPDF |
|---|---|---|
| Open encrypted, honour permissions | Declared | **Done** (opening; permissions shown in the properties) |
| Add/change password, AES-256 | Declared | **Done** (AES-128/256, permissions; AES-128 limited to Latin-1 passwords, typed refusal; accessibility always granted) through `lopdf`, re-read by PDFium; explicit AES-256 key length and synthetic qpdf gate added for 0.9.1; results in `PROJECT_STATUS.md` |
| Irreversible redaction | Declared (redact crate) | **Delivered in 0.30.0**: Native preview, fill/overlay, persistent unapplied marks, safe Base14 Latin glyph splitting and shared cleanup. Complex objects use raster fallback; marks alone do not redact. [Guarantees](REDACTION.md). |
| Metadata cleanup | Declared | **Delivered in 0.30.0**: Earlier PDF/A metadata preservation retained, without conformance claim. Separate sanitized copy removes supported hidden data/full rewrite; backups/Undo/disk sectors not erased. [Security model](SECURITY_MODEL.md). |
| Visual signature (image) | Declared | **Delivered in 0.30.0**: Earlier drawing/typing/import retained; optional user-DPAPI library with verified migration, opt-out/delete and profile/ACL safeguards. Profile keys required for recovery; session access permits reuse. No identity/integrity proof. [Details](SIGNATURE_SECURITY.md). |
| CMS/PKCS#12 digital signature, timestamping | Declared | **Delivered in 0.30.0**: Earlier integrity/PFX signing retained; separate Windows chain/revocation/RFC3161 verification. Offline default, per-check network consent, unknown never trusted. DSS unverified; no archival LTV/timestamp creation. [Limits](SIGNATURE_SECURITY.md). |
| PKCS#11, online LTV, AATL/EUTL | Absent | Out of initial scope |

## F. Forms and accessibility

| Feature | Expected | SuniPDF |
|---|---|---|
| Fill AcroForm, regenerated appearances | Declared | **Done** (forms slice, D061–D063): text (single line, multiline, maximum length), checkbox, radio buttons, drop-down list, list box (multiple choice), hierarchical and multi-widget fields; values written by lopdf, appearances regenerated by PDFium and present in the file; on-page tool + accessible panel + flattening (fields and annotations) + clear; encrypted document kept encrypted; CLI `form-fields` / `fill-form` / `flatten`; scripts never executed (reported) |
| Create fields | Not documented | **Done**: numeric text/checkbox creation in Prepare a form; radio/combo/list creation delivered in 0.10.0; unique names, rotated/cropped/encrypted pages, undo; core/CLI tests; CLI `add-form-field` |
| Draw new field rectangles on the page | Not documented | **Published in 0.19.0** (D110): same five types and numeric dialog, retained name/options/flags, one-page finite rectangle at least 4 pt per dimension, explicit Create/one Undo, stale document/tab/tool guards. Local tests, formatting and Clippy passed; release gates passed. No new field type or core/CLI API |
| FDF/XFDF import/export | Declared | **Done in 0.10.0**: JSON/XFDF export, strict bounded JSON/XFDF/FDF value import and grouped panel Apply/Discard with one undo; value-only FDF subset, no scripts/attachments/rich text |
| Form-read failure recovery | Not documented | **Delivered in 0.11.0**: explicit error/retry preserves drafts and pending actions; local tests passed |
| Existing editable drop-down free text | Not documented | **Delivered in 0.11.0**: panel/on-page custom text plus listed choices, Apply/Discard/Save and undo; local, CI, package and upgrade gates passed |
| Create editable drop-downs | Not documented | **Delivered in 0.12.0**: desktop checkbox and CLI `add-form-field --kind combo --editable`; nonempty predefined choices still required; local, CI, package and upgrade gates passed |
| Radio `/Opt` export values | Not documented | **Delivered in 0.12.0**: separate export values from raw widget states across filling/JSON/XFDF/FDF; additive CLI `widgets[].export_value`; malformed/duplicate mappings keep that field read-only; no-`/Opt` behaviour unchanged; local, CI, package and upgrade gates passed |
| XFA | Partial | **Detected, read-only** (AcroForm fields listed; filling and flattening refused, `form_xfa`) |
| Preserve non-Link annotations without usable appearances during flattening | Not documented | **Published in 0.21.1:** raw non-widget graphs, attachments and shared references preserved; widgets/drawable appearances still flatten, retained annotations excluded from the count, failures atomic. Release-source and public-package checks passed; absent from 0.21.0 packages; [limits and evidence](PROJECT_STATUS.md) |
| Tagging, PDF/UA checks | Partial / absent | Planned (P6, checks only) |

## G. Automation and integration

| Feature | Expected | SuniPDF |
|---|---|---|
| JSON CLI | Declared | **Done** (reading, pages, security, conversion, stamps and content editing: `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts`); P5 extension |
| Common UI/CLI command registry | Declared (tool table) | **Done** (`sunipdf-cli commands`) |
| Local MCP server, confined to `--root` | Declared | Planned (P5, opt-in) |
| Windows Explorer integration (association, context menu) | Not documented | Planned (P5) |
| Command palette | Declared | **Done** (Ctrl+K, filtering, keyboard, colour of the owning tool) |
| "All tools" hub by task, contextual bar per tool | Declared ("All tools" list) | **Done** for all 14 tools; Scan & OCR, Prepare a form and Use a certificate have real commands; registry and headless activation tests |
| Local workflows, conversion profiles | Absent | **Delivered in 0.30.0**: Remembered Standard/Print Office profiles. General workflow designer remains planned. [Scope](OFFICE_CONVERSION.md). |

## H. Distribution

| Feature | Expected | SuniPDF |
|---|---|---|
| Windows x64 | Declared (MSI, zip) | **Distributed in 0.32.0**: 24 portable cases and 18 MSI baselines/90 logs passed; six public files/SHA256 and packaged checks verified; live updater downloaded and verified the package without installation on the workstation. Native interactive qualification remains open. [Evidence](qa/RELEASE_0.32.0.json). |
| Windows ARM64 | Declared | **Cross-built and distributed in 0.32.0**: exact-source ZIP, public SHA256, PE architecture, models, notices and icons verified; runtime remains experimental until real-device qualification. [Evidence](qa/RELEASE_0.32.0.json). |
| Code signing | Not documented | Planned (P6) if a certificate is provided |
| Optional GPU renderer | Not documented | **Delivered in 0.30.0**: optional wgpu compatibility and DX12 software-renderer gate; OpenGL remains default. Real GPU/window and ARM64 hardware qualification remain separate |
| Dependency/component license inventory | Not documented | **Delivered in 0.30.0**: exact-version offline inventory and package-content checks; [scope](RELEASE_0.30.0.md). Ongoing license maintenance remains required |
