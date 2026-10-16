# Feature matrix: expected of a full PDF editor / SuniPDF

[Français](FEATURE_MATRIX.fr.md)

"Expected" column: capability expected of a full PDF editor (**Declared** = common in the editors on the market,
**Partial** = announced as incomplete by the project, **Absent** = announced as missing. Not verified by execution.

SuniPDF statuses: **Done** = implemented **and** covered by executed tests; **In progress**; **Planned (Pn)** = roadmap
phase; **Out of scope**. Update this column only with real results.

The 0.9.0 features below have automated regression coverage. `PROJECT_STATUS.md` records the executed checks;
these results do not establish physical printer, ARM64 or third-party reader compatibility.
The 0.9.1 workspace, qpdf and real portable/MSI regressions passed; package/publication outcomes are recorded there.

[0.10.0 is published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) with the requested P1/P2 lot.
The 362 workspace tests, 31 synthetic qpdf checks, four real NIST documents (744 pages), eight portable scenarios,
two MSI baselines and all six public assets passed verification; evidence is recorded in `PROJECT_STATUS.md`.

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
| Bookmarks (outline) | Declared | **Done** (panel, navigation; editing planned) | |
| Page labels | Declared | Planned (P3) | |
| Navigation history, reopened tabs | Declared | Recent files **Done**; per-tab 256-entry navigation history **Done in 0.10.0** | |
| Reading mode / full screen | Declared | **Done in 0.10.0** | View mode is a shared preference; rotation/history are retained per tab; editing returns to upright view |
| Windows printing | Declared (limited fidelity) | **Done** (layout/range tests and actual Microsoft Print to PDF output reopened) | Native dialog, ranges/copies, bounded raster printing, progress/cancel; physical printer and interactive dialog unverified |
| Crash recovery, autosave | Declared | **Done** (recovery/worker/tab regression tests) | 60-second and tab-suspend snapshots; source unchanged; encryption retained/no passwords; restore Save As |
| Light/dark themes | Declared | **Done** | Design system `theme.rs`, toggle in the tab bar |
| Translatable interface | Declared (EN, JA + 22 catalogues) | **Done** (en-US default, fr-FR available) | Hot switching, existing preferences preserved |

## B. Editing and annotation

| Feature | Expected | SuniPDF |
|---|---|---|
| Notes, highlight, underline, strikethrough, shapes, ink, text boxes | Declared | **Done** (D067–D076): notes, highlight, underline, strikethrough; freehand **ink** (smoothed), **free text box** (Helvetica WinAnsi), **rectangle, ellipse, line, arrow** as real PDF annotations (explicit `/AP` appearance, rendered by other readers), colour, width, fill, opacity; select, move, resize (SuniPDF annotations), recolour, delete; rotated and cropped pages correct; encrypted document stays encrypted; CLI `annotate` / `annotations` |
| Reply threads, comment status | Declared | **Done in SuniPDF**: Comments panel as threads (replies, status Accepted / Rejected / Cancelled / Completed, author, date, thread deletion), written according to ISO 32000-1; **Acrobat / Foxit compatibility not verified**; CLI `reply` / `review-state` |
| Adding text / images | Declared | **Done** (text typed on the page, several lines, Helvetica/Times/Courier bold/italic or **embedded system TrueType font** (Unicode, missing character refused), size, colour, opacity; PNG/JPEG/BMP/GIF/TIFF/WebP image with opacity; rectangle/ellipse/line shapes with colours, thickness and opacity; upright on screen on rotated pages); font subsetting and `.otf`/`.ttc` planned |
| Modifying existing text | Partial (weak in CJK/bidi) | **Done** for simple text objects (editing on the page or F2: text, size, colour; original font kept, missing characters not guaranteed) |
| Object transformation | Declared | **Done**: move, resize, **free rotation** (handle, typed angle), duplication, Z-order (to the extreme or by one step), recolouring, **opacity**, deletion, **image replacement**; rendering tests on normal, rotated and cropped pages |
| Multiple selection, cut/copy/paste of objects | Declared | **Done**: rectangle, Shift/Ctrl+click, Ctrl+A, grouped move; clipboard across pages and tabs (pasted = group not editable as text); group rotation/resizing delivered in 0.10.0 (single undo) |
| Page objects panel | Declared | **Done** (F3: drawing order, selection, bring forward/send backward, deletion, marker on hover) |
| Links | Declared | **Done**: creation (go to a page, `http` / `https` / `mailto` address), click on an internal link (page jump), **external address only after a confirmation** showing the full address; Launch / JavaScript actions never executed; named destinations not resolved; CLI `annotate --kind link` |
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
| Extraction preserving links, fields, layers, attachments | Declared | Extraction **Done** (UI + CLI, `FPDF_ImportPages`); from a protected document: unencrypted result, announced (D046); semantic preservation to verify beyond qpdf structural checking |
| Drag-and-drop reordering | Declared | **Done** (thumbnails and Organize view) |
| Named undo/redo surviving save | Declared | Undo/redo **Done** (snapshots, unnamed); survives save |
| Comparison of two versions | Declared (compare crate) | Planned (P5) |
| Batch processing | Declared (CLI batch) | **Done in 0.10.0**: sequential compression/local OCR, 128 files, 256 MiB/input, progress/cancel/per-file errors, new copies only |

## D. Conversion and OCR

| Feature | Expected | SuniPDF |
|---|---|---|
| PDF → images | Declared (CLI render) | **Done** (UI 150 dpi PNG export, CLI `render`, `export-images`) |
| PDF → text | Declared | **Done** (CLI `text`) |
| Images → PDF | Declared (create) | **Done** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP; EXIF orientation applied, JPEGs embedded as is) |
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
| Handwritten signature (image) | Declared | **Done** (sign-image): drawn on the pad, imported (transparent PNG or photo with the light background removed by threshold) or reused from the local library (`%APPDATA%\SuniPDF\signatures`, plain PNG, optional saving), initials, today's date; alpha-masked image object, editable and undoable; CLI `add-image`; **visual, never presented as certified** (permanent box, `digital_signature: false`) |
| CMS/PKCS#12 digital signature, timestamping | Declared | Integrity verification/new-copy PFX/P12 signing **Done** (disposable-certificate tests); CLI `signatures`/`sign`; encrypted signing refused. Trust/revocation/trusted timestamps/LTV not implemented; Acrobat/Foxit interoperability unverified |
| PKCS#11, online LTV, AATL/EUTL | Absent | Out of initial scope |

## F. Forms and accessibility

| Feature | Expected | SuniPDF |
|---|---|---|
| Fill AcroForm, regenerated appearances | Declared | **Done** (forms slice, D061–D063): text (single line, multiline, maximum length), checkbox, radio buttons, drop-down list, list box (multiple choice), hierarchical and multi-widget fields; values written by lopdf, appearances regenerated by PDFium and present in the file; on-page tool + accessible panel + flattening (fields and annotations) + clear; encrypted document kept encrypted; CLI `form-fields` / `fill-form` / `flatten`; scripts never executed (reported) |
| Create fields | Not documented | **Done**: numeric text/checkbox creation in Prepare a form; radio/combo/list creation delivered in 0.10.0; unique names, rotated/cropped/encrypted pages, undo; core/CLI tests; CLI `add-form-field` |
| FDF/XFDF import/export | Declared | **Done in 0.10.0**: JSON/XFDF export, strict bounded JSON/XFDF/FDF value import and grouped panel Apply/Discard with one undo; value-only FDF subset, no scripts/attachments/rich text |
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
| Windows x64 | Declared (MSI, zip) | **Done**: 0.10.0 ZIP/MSI published as Latest, public SHA256/payloads verified, packaged OCR/forms/XFDF/compression/group transforms/read/render and eight real portable scenarios passed; two MSI baselines passed actual install/transactional rollback/upgrade/downgrade rejection/uninstall; live updater passed; evidence in `PROJECT_STATUS.md`; interactive UX unverified |
| Windows ARM64 | Declared (dedicated workflow) | **Done**: 0.10.0 CI-built portable ZIP published, public SHA256/architecture/models/notices/icon resources checked; evidence in `PROJECT_STATUS.md`; hardware execution unverified |
| Code signing | Not documented | Planned (P6) if a certificate is provided |
