# Feature matrix: expected of a full PDF editor / SuniPDF

[Français](FEATURE_MATRIX.fr.md)

"Expected" column: capability expected of a full PDF editor (**Declared** = common in the editors on the market,
**Partial** = announced as incomplete by the project, **Absent** = announced as missing. Not verified by execution.

SuniPDF statuses: **Done** = implemented **and** covered by executed tests; **In progress**; **Planned (Pn)** = roadmap
phase; **Out of scope**. Update this column only with real results.

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

[0.17.0 is published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) under
`lmdanavanem`, release `415074108`, source `5785de282e126fb4666b28fb83340d0e9c6e9acf`. Validation passed:
445 workspace tests (zero failed, seven default ignored), formatting/Clippy, 896 EN/FR keys, 74 qpdf checks,
four licensed documents/744 pages (48.187 seconds locally; 45.791 seconds in CI), Windows CI and same-source
ARM64 compilation, fifteen portable cases and nine MSI baselines. All six staged/public files and enhanced
conversion checks passed, including nine exact JPEG streams, oriented corners, PNG transparency, budget/DPI
refusals and unchanged sources. Verified copies are in `dist/`; the production updater verified the
19,825,743-byte Latest x64 ZIP without installation. Evidence and remaining limits are in `PROJECT_STATUS.md`.

**0.18.0 in development:** grouped desktop freehand strokes on one page, explicit Finish/Cancel, one annotation
and one Undo through the existing writer; retained drafts after worker failure and guarded document/save/close/
tab/tool transitions (D109). A16 uses the existing clap help template for literal `{n}` / `{total}`.
Local validation passed: 452 workspace tests (zero failed, seven default ignored), formatting/Clippy, 907 matching
EN/FR keys, 75 qpdf checks and the update-helper self-test. Six new headless UI tests, the existing extended
worker Undo/Redo/failure test and the new CLI help regression passed. CI/packages/upgrades and publication pending; 0.17.0 remains Latest. No new dependency or CLI command; 53 commands remain.
Local corpus passed: four licensed NIST/NBS documents, 744 pages in 50.796 seconds, sampled peak child
memory 329,445,376 bytes; all sources unchanged. Clippy with warnings denied passed.

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
| Bookmarks (outline) | Declared | **Done** (panel, page navigation; editing planned); precise destinations **delivered in 0.11.0** | Named, XYZ/Fit/nonzero FitH; local tests passed |
| Page labels | Declared | **Reading delivered in 0.11.0**; authoring/reset **delivered in 0.14.0**, local/CI/package/public/upgrade checks passed | Existing exact unique-label/physical `#N` navigation retained; Pages/Organize and two CLI commands author six styles over an inclusive physical range, preserve outside rules; reset all; no printed-number changes or semantic rebasing |
| Navigation history, reopened tabs | Declared | Recent files and per-tab 256-entry history **Done**; same-page/zoom history **delivered in 0.11.0** | Local tests passed |
| Reading mode / full screen | Declared | **Done in 0.10.0** | View mode is a shared preference; rotation/history are retained per tab; editing returns to upright view |
| Windows printing | Declared (limited fidelity) | **Done** (layout/range tests and actual Microsoft Print to PDF output reopened) | Native dialog, ranges/copies, bounded raster printing, progress/cancel; physical printer and interactive dialog unverified |
| Crash recovery, autosave | Declared | **Done** (recovery/worker/tab regression tests) | 60-second and tab-suspend snapshots; source unchanged; encryption retained/no passwords; restore Save As |
| Light/dark themes | Declared | **Done** | Design system `theme.rs`, toggle in the tab bar |
| Translatable interface | Declared (EN, JA + 22 catalogues) | **Done** (en-US default, fr-FR available) | Hot switching, existing preferences preserved |

## B. Editing and annotation

| Feature | Expected | SuniPDF |
|---|---|---|
| Notes, highlight, underline, strikethrough, shapes, ink, text boxes | Declared | **Done** (D067–D076): notes, highlight, underline, strikethrough; freehand **ink** (smoothed), **free text box** (Helvetica WinAnsi), **rectangle, ellipse, line, arrow** as real PDF annotations (explicit `/AP` appearance, rendered by other readers), colour, width, fill, opacity; select, move, resize (SuniPDF annotations), recolour, delete; rotated and cropped pages correct; encrypted document stays encrypted; CLI `annotate` / `annotations`. **Delivered in 0.16.0**: dashed borders for the four shapes, fixed UI/CLI `[6,3]`, preserved custom `dash_pattern`, malformed-style refusal and bounded regeneration; local/CI/upgrade, staged/public package and production updater checks passed (D107); manual reader checks remain separate |
| Reply threads, comment status | Declared | **Done in SuniPDF**: Comments panel as threads (replies, status Accepted / Rejected / Cancelled / Completed, author, date, thread deletion), written according to ISO 32000-1; **Acrobat / Foxit compatibility not verified**; CLI `reply` / `review-state` |
| Group several freehand strokes | Declared | **In progress for 0.18.0** (D109): one page-bound draft, explicit Finish/Cancel, one existing AddInk/Undo; failure retention and conflicting-action guards. Local regressions passed; no idle auto-commit or draft crash recovery; CI/packages/publication pending |
| Adding text / images | Declared | **Done** (text typed on the page, several lines, Helvetica/Times/Courier bold/italic or **embedded system TrueType font** (Unicode, missing character refused), size, colour, opacity; PNG/JPEG/BMP/GIF/TIFF/WebP image with opacity; rectangle/ellipse/line shapes with colours, thickness and opacity; upright on screen on rotated pages); font subsetting and `.otf`/`.ttc` planned |
| Modifying existing text | Partial (weak in CJK/bidi) | **Done** for simple text objects (editing on the page or F2: text, size, colour; original font kept, missing characters not guaranteed) |
| Object transformation | Declared | **Done**: move, resize, **free rotation** (handle, typed angle), duplication, Z-order (to the extreme or by one step), recolouring, **opacity**, deletion, **image replacement**; rendering tests on normal, rotated and cropped pages |
| Multiple selection, cut/copy/paste of objects | Declared | **Done**: rectangle, Shift/Ctrl+click, Ctrl+A, grouped move; clipboard across pages and tabs (pasted = group not editable as text); group rotation/resizing delivered in 0.10.0 (single undo) |
| Page objects panel | Declared | **Done** (F3: drawing order, selection, bring forward/send backward, deletion, marker on hover) |
| Links | Declared | **Done**: page/URI creation, confirmed allow-listed addresses and 0.11.0 PDFium-resolved named/XYZ/Fit/nonzero FitH navigation. **Delivered in 0.15.0**: edit existing targets through UI/CLI, preserving area/style/comments, with safe URI or physical-page Fit replacement and strict lock flags; local/CI/package/public/upgrade checks passed (D106). Detailed views are replaced explicitly; Launch/JavaScript/remote GoTo never execute |
| Watermarks, headers/footers, numbering | Partial | **Done**: text stamp/watermark (angle, opacity, position) and `{n}`/`{total}` numbering (UI + CLI `stamp`, `page-numbers`); image watermark and multi-zone headers planned (P3) |
| Custom stamps | Not documented | Planned (P3) |

## C. Organisation

| Feature | Expected | SuniPDF |
|---|---|---|
| Rotate, delete, insert, move, blank pages | Declared | **Done** (UI + CLI `insert-blank` for blank pages) |
| Crop pages (CropBox) | Declared | **Done** (dialog in mm with preview, restore, CLI `crop`); mask only, not redaction; mouse crop preview delivered in 0.10.0 |
| Resize pages | Declared | **Done** (A3–Legal or custom, content kept or scaled; CLI `resize-pages`); annotations not scaled |
| Merge N PDFs with bookmarks | Declared | Merge **Done** (UI "Combine files", CLI `merge`); merge bookmarks planned |
| Split (every n / chosen pages) | Declared | **Done** (UI dialog + CLI `split`, `extract`) |
| Extraction preserving links, fields, layers, attachments | Declared | Extraction **Done** (UI + CLI, `FPDF_ImportPages`); **implemented for 0.11.0, published; local, CI, package and upgrade gates passed**: retain source encryption/passwords/permissions and refuse output at the source path (D097 supersedes D046). Existing permission policy unchanged; CLI `encryption_removed: false`, additive `encrypted`; semantic preservation to verify beyond qpdf structural checking |
| Drag-and-drop reordering | Declared | **Done** (thumbnails and Organize view) |
| Named undo/redo surviving save | Declared | Undo/redo **Done** (snapshots, unnamed); survives save. **Delivered in 0.14.0**: preserve navigation through label/content Undo/Redo when page geometry is unchanged; executed checks passed (D105) |
| Comparison of two versions | Declared (compare crate) | Planned (P5) |
| Batch processing | Declared (CLI batch) | **Done in 0.10.0**: sequential compression/local OCR, 128 files, 256 MiB/input, progress/cancel/per-file errors, new copies only |

## D. Conversion and OCR

| Feature | Expected | SuniPDF |
|---|---|---|
| PDF → images | Declared (CLI render) | **Done** (UI 150 dpi PNG export, CLI `render`, `export-images`) |
| PDF → text | Declared | **Done** (CLI `text`) |
| Images → PDF | Declared (create) | **Done** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP). **Delivered in 0.17.0**: direct embedding for all eight JPEG EXIF orientations; 512 MiB cumulative non-JPEG raster estimate `sum(max(decoded bytes, width × height × 4))` before allocation, checked arithmetic and finite/positive page geometry (D108). Quality/transparency and callers retained; estimate is not a process/native RSS ceiling or compressed-JPEG limit. Local/CI/upgrade, independent debug/staged/public conversion and production updater checks passed; evidence and remaining limits in `PROJECT_STATUS.md` |
| PDF → Word | Partial (loses the layout) | Planned (P4), fidelity to be announced honestly |
| PDF → Excel / PowerPoint | Absent / partial pptx export | Planned (P4) |
| Office → PDF | Absent | Planned (P4) through an installed LibreOffice/Word if present; otherwise out of scope |
| HTML → PDF | Absent | Planned (P5, optional) |
| Image compression | Declared | **Done in 0.10.0**: supported RGB/Gray images, JPEG quality/max-side controls, no-gain byte copy, masks/profiles retained, signed/restricted inputs refused |
| OCR | Partial (Latin, CC-BY-SA models) | **Done**: bundled offline Tesseract 5.5.3 / Leptonica 1.87, fast eng/fra models, invisible text, skip existing text, progress/cancel/one undo; core and real CLI recognition tests; CLI `ocr` |

## E. Security and signatures

| Feature | Expected | SuniPDF |
|---|---|---|
| Open encrypted, honour permissions | Declared | **Done** (opening; permissions shown in the properties) |
| Add/change password, AES-256 | Declared | **Done** (AES-128/256, permissions; AES-128 limited to Latin-1 passwords, typed refusal; accessibility always granted) through `lopdf`, re-read by PDFium; explicit AES-256 key length and synthetic qpdf gate added for 0.9.1; results in `PROJECT_STATUS.md` |
| Irreversible redaction | Declared (redact crate) | **Done** (hub Redact tool: areas drawn with the mouse, text marking, confirmation, save guard; CLI `redact`): objects removed + blacked-out image patch, annotations / fields / metadata / earlier revisions removed, verification by reading back; absence proven by tests (extraction, decompressed bytes, pixels, rotated pages, encrypted document); limits `KNOWN_ISSUES.md` A40 |
| Metadata cleanup | Declared | **Done** (Info, XMP of the catalog, pages and images, `/PieceInfo`); 0.9.1 property editing preserves PDF/A identification/unrelated XMP and refuses malformed XML, validation tracked in `PROJECT_STATUS.md`; PDF/A conformance not certified |
| Visual signature (image) | Declared | **Done**: draw, type, import or reuse a local plain PNG, initials/date, editable alpha-masked image and undo. **Delivered in 0.13.0**: typed name/initials with explicit native preview and existing font checks; current/all/Organize-selected targets, fitted geometry, atomic cancel/one undo; additive CLI `add-image --pages`. Local, CI, package/public and upgrade checks passed. **Visual, without identity/integrity proof**; separate CMS/PFX workflow unchanged |
| CMS/PKCS#12 digital signature, timestamping | Declared | Integrity verification/new-copy PFX/P12 signing **Done** (disposable-certificate tests); CLI `signatures`/`sign`; encrypted signing refused. Trust/revocation/trusted timestamps/LTV not implemented; Acrobat/Foxit interoperability unverified |
| PKCS#11, online LTV, AATL/EUTL | Absent | Out of initial scope |

## F. Forms and accessibility

| Feature | Expected | SuniPDF |
|---|---|---|
| Fill AcroForm, regenerated appearances | Declared | **Done** (forms slice, D061–D063): text (single line, multiline, maximum length), checkbox, radio buttons, drop-down list, list box (multiple choice), hierarchical and multi-widget fields; values written by lopdf, appearances regenerated by PDFium and present in the file; on-page tool + accessible panel + flattening (fields and annotations) + clear; encrypted document kept encrypted; CLI `form-fields` / `fill-form` / `flatten`; scripts never executed (reported) |
| Create fields | Not documented | **Done**: numeric text/checkbox creation in Prepare a form; radio/combo/list creation delivered in 0.10.0; unique names, rotated/cropped/encrypted pages, undo; core/CLI tests; CLI `add-form-field` |
| FDF/XFDF import/export | Declared | **Done in 0.10.0**: JSON/XFDF export, strict bounded JSON/XFDF/FDF value import and grouped panel Apply/Discard with one undo; value-only FDF subset, no scripts/attachments/rich text |
| Form-read failure recovery | Not documented | **Delivered in 0.11.0**: explicit error/retry preserves drafts and pending actions; local tests passed |
| Existing editable drop-down free text | Not documented | **Delivered in 0.11.0**: panel/on-page custom text plus listed choices, Apply/Discard/Save and undo; local, CI, package and upgrade gates passed |
| Create editable drop-downs | Not documented | **Delivered in 0.12.0**: desktop checkbox and CLI `add-form-field --kind combo --editable`; nonempty predefined choices still required; local, CI, package and upgrade gates passed |
| Radio `/Opt` export values | Not documented | **Delivered in 0.12.0**: separate export values from raw widget states across filling/JSON/XFDF/FDF; additive CLI `widgets[].export_value`; malformed/duplicate mappings keep that field read-only; no-`/Opt` behaviour unchanged; local, CI, package and upgrade gates passed |
| XFA | Partial | **Detected, read-only** (AcroForm fields listed; filling and flattening refused, `form_xfa`) |
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
| Local workflows, conversion profiles | Absent | Planned (P5) |

## H. Distribution

| Feature | Expected | SuniPDF |
|---|---|---|
| Windows x64 | Declared (MSI, zip) | **Done**: 0.17.0 ZIP/MSI published as Latest; fifteen real portable cases and nine MSI baselines passed; six public files/SHA256 and packaged checks verified; production updater verified without installation; evidence in `PROJECT_STATUS.md`; interactive UX unverified |
| Windows ARM64 | Declared | **Done**: same-source 0.17.0 portable ZIP built in CI and published; public SHA256, architecture, models, notices and icons verified; evidence in `PROJECT_STATUS.md`; hardware execution unverified |
| Code signing | Not documented | Planned (P6) if a certificate is provided |
