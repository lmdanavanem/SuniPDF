# Feature matrix: expected of a full PDF editor / SuniPDF

[Français](FEATURE_MATRIX.fr.md)

"Expected" column: capability expected of a full PDF editor (**Declared** = common in the editors on the market,
**Partial** = announced as incomplete by the project, **Absent** = announced as missing. Not verified by execution.

SuniPDF statuses: **Done** = implemented **and** covered by executed tests; **In progress**; **Planned (Pn)** = roadmap
phase; **Out of scope**. Update this column only with real results.

## A. Reading

| Feature | Expected | SuniPDF | Note |
|---|---|---|---|
| Open a PDF, display a page | Declared | **Done** | PDFium; tests `crates/sunipdf-core/tests/engine.rs` |
| Password-protected documents | Declared (all revisions) | **Done** | RC4, AES-128 and AES-256 covered by engine/protection tests and reopened by PDFium |
| Page navigation previous/next/go to | Declared | **Done** (UI) | Keyboard + toolbar |
| Zoom, fit width/page, actual size | Declared | **Done** (UI) | Rendering at physical resolution (DPI); Ctrl + wheel anchored under the pointer, 10 to 3200% |
| Rendering off the UI thread | Declared | **Done** | Single engine thread, wanted-tile list replaced as a block, anti-staleness epochs |
| Multi-document tabs | Declared | **Done** (inactive documents suspended, interface state kept) | |
| Continuous display, two-page view, view rotation | Declared | Continuous display **Done** (virtualised, current page tracked, single page optional with F5); two-page view and view rotation planned | Tests `view.rs` (layout, virtualisation, zoom anchor), screenshots |
| Tiled rendering, capped cache | Declared | **Done** (1024 px tiles beyond 4096 px / 8 Mpx, 256 MB LRU cache, 192 MB textures) | `Document::render_region`, pixel tests (regions, stitching, ×40 zoom); CLI `render --region` |
| Full-text search | Declared | **Done** | Ctrl+F, case, highlighting, CLI `search` |
| Text selection / copy in reading order | Declared | **Done** (rectangular selection, merged lines) | complex reading order planned |
| Thumbnails | Declared | **Done** | Virtualised panel, shared cache |
| Bookmarks (outline) | Declared | **Done** (panel, navigation; editing planned) | |
| Page labels | Declared | Planned (P3) | |
| Navigation history, reopened tabs | Declared | Recent files **Done** (home); navigation history planned (P3) | |
| Reading mode / full screen | Declared | Planned (P3) | |
| Windows printing | Declared (limited fidelity) | Planned (P5) | Windows API (`sunipdf-platform`) |
| Crash recovery, autosave | Declared | Planned (P3) | |
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
| Multiple selection, cut/copy/paste of objects | Declared | **Done**: rectangle, Shift/Ctrl+click, Ctrl+A, grouped move; clipboard across pages and tabs (pasted = group not editable as text); group rotation/resizing planned |
| Page objects panel | Declared | **Done** (F3: drawing order, selection, bring forward/send backward, deletion, marker on hover) |
| Links | Declared | **Done**: creation (go to a page, `http` / `https` / `mailto` address), click on an internal link (page jump), **external address only after a confirmation** showing the full address; Launch / JavaScript actions never executed; named destinations not resolved; CLI `annotate --kind link` |
| Watermarks, headers/footers, numbering | Partial | **Done**: text stamp/watermark (angle, opacity, position) and `{n}`/`{total}` numbering (UI + CLI `stamp`, `page-numbers`); image watermark and multi-zone headers planned (P3) |
| Custom stamps | Not documented | Planned (P3) |

## C. Organisation

| Feature | Expected | SuniPDF |
|---|---|---|
| Rotate, delete, insert, move, blank pages | Declared | **Done** (UI + CLI `insert-blank` for blank pages) |
| Crop pages (CropBox) | Declared | **Done** (dialog in mm with preview, restore, CLI `crop`); mask only, not redaction; mouse cropping planned |
| Resize pages | Declared | **Done** (A3–Legal or custom, content kept or scaled; CLI `resize-pages`); annotations not scaled |
| Merge N PDFs with bookmarks | Declared | Merge **Done** (UI "Combine files", CLI `merge`); merge bookmarks planned |
| Split (every n / chosen pages) | Declared | **Done** (UI dialog + CLI `split`, `extract`) |
| Extraction preserving links, fields, layers, attachments | Declared | Extraction **Done** (UI + CLI, `FPDF_ImportPages`); from a protected document: unencrypted result, announced (D046); preservation to verify with qpdf |
| Drag-and-drop reordering | Declared | **Done** (thumbnails and Organize view) |
| Named undo/redo surviving save | Declared | Undo/redo **Done** (snapshots, unnamed); survives save |
| Comparison of two versions | Declared (compare crate) | Planned (P5) |
| Batch processing | Declared (CLI batch) | Planned (P4/P5) |

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
| OCR | Partial (Latin, CC-BY-SA models) | Planned (P4): Tesseract (Apache-2.0) fra+eng, offline |

## E. Security and signatures

| Feature | Expected | SuniPDF |
|---|---|---|
| Open encrypted, honour permissions | Declared | **Done** (opening; permissions shown in the properties) |
| Add/change password, AES-256 | Declared | **Done** (AES-128/256, permissions; AES-128 limited to Latin-1 passwords, typed refusal; accessibility always granted) through `lopdf`, re-read by PDFium; qpdf verification to add |
| Irreversible redaction | Declared (redact crate) | **Done** (hub Redact tool: areas drawn with the mouse, text marking, confirmation, save guard; CLI `redact`): objects removed + blacked-out image patch, annotations / fields / metadata / earlier revisions removed, verification by reading back; absence proven by tests (extraction, decompressed bytes, pixels, rotated pages, encrypted document); limits `KNOWN_ISSUES.md` A40 |
| Metadata cleanup | Declared | **Done** (Info, XMP of the catalog, pages and images, `/PieceInfo`; modified properties regenerate the XMP) |
| Handwritten signature (image) | Declared | **Done** (sign-image): drawn on the pad, imported (transparent PNG or photo with the light background removed by threshold) or reused from the local library (`%APPDATA%\SuniPDF\signatures`, plain PNG, optional saving), initials, today's date; alpha-masked image object, editable and undoable; CLI `add-image`; **visual, never presented as certified** (permanent box, `digital_signature: false`) |
| CMS/PKCS#12 digital signature, timestamping | Declared | Planned (P5) |
| PKCS#11, online LTV, AATL/EUTL | Absent | Out of initial scope |

## F. Forms and accessibility

| Feature | Expected | SuniPDF |
|---|---|---|
| Fill AcroForm, regenerated appearances | Declared | **Done** (forms slice, D061–D063): text (single line, multiline, maximum length), checkbox, radio buttons, drop-down list, list box (multiple choice), hierarchical and multi-widget fields; values written by lopdf, appearances regenerated by PDFium and present in the file; on-page tool + accessible panel + flattening (fields and annotations) + clear; encrypted document kept encrypted; CLI `form-fields` / `fill-form` / `flatten`; scripts never executed (reported) |
| Create fields | Not documented | Planned (roadmap: lopdf alone, D066); "Prepare a form" absent from the hub |
| FDF/XFDF import/export | Declared (xfdf) | **JSON done** (`{ nom: valeur }`, export per task, import validated field by field); FDF/XFDF not (D064) |
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
| "All tools" hub by task, contextual bar per tool | Declared ("All tools" list) | **Done** for 11 tools out of 14 (Export, Fill & Sign (fill and sign), Edit, Create, Combine, Organize, Comment, Protect, Redact, Compress, Stamp): F9 panel, home cards, Tools menu, palette, tool bar with Esc; OCR, Form, Certificate planned (absent as long as their feature does not exist) |
| Local workflows, conversion profiles | Absent | Planned (P5) |

## H. Distribution

| Feature | Expected | SuniPDF |
|---|---|---|
| Windows x64 | Declared (MSI, zip) | Planned (P6); CI written, not executed |
| Windows ARM64 | Declared (dedicated workflow) | Planned (P6); PDFium ARM64 binaries available |
| Code signing | Not documented | Planned (P6) if a certificate is provided |
