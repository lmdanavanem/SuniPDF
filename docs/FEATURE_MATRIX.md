# SuniPDF feature matrix

See [project status](PROJECT_STATUS.md) for the current published version and downloads.
Versioned release notes distinguish implemented source from qualified packages and untested native workflows.
The historical sections retain the limits and results of their own releases.

## In development: 0.76.0

- Two reader panes for the same PDF, independent navigation and zoom, active-pane editing and optional synchronization.
- Shared document revision, Undo/Redo history and bounded rendering cache; per-tab session locations.
- [Workflow](READER_WORKSPACE.md#two-independent-views) and [pending qualification](RELEASE_0.76.0.md). Different-document split views and restart persistence are outside this increment.

## Added in 0.75.0

- Native Apple Silicon DMG, macOS menu bar, Finder opening, Command shortcuts and system clipboard support.
- Mac Theme settings and manual update notifications, native settings/recovery paths and guarded regular-file readers.
- Installed package, rendering and edit/save/reopen checks on macOS 15; no notarization or physical Mac/interactive Finder/clipboard qualification. [Platform limits](MACOS.md) · [Release evidence](RELEASE_0.75.0.md).

## Added in 0.74.0

- Classic and Liquid Glass appearance, System/Light/Dark modes, reduced transparency and persistent Theme settings.
- Verified Ubuntu update downloads, administrator-authorized installation and save-protected restart. One manual upgrade is needed from 0.73 or earlier.
- [Qualification and limits](RELEASE_0.74.0.md); full graphical authentication and physical-hardware acceptance remain open.

## Added in 0.73.0

| Area | Implemented behavior and limits |
|---|---|
| Multi-source plan links | Explicit per-source pages, destinations and confirmations; source/destination previews; all-output preflight; sequential new-copy saves and retained partial receipts |
| Batch safety | Up to 16 sources, 128 pages and 4,096 detected references; per-file atomicity; original indexed destinations retained; existing files never overwritten |
| Linux window controls | Round vector buttons with 40-pixel logical targets, native minimize/maximize/restore/move/resize and guarded close; title double-click independent of earlier button clicks |

Published stable with [qualification evidence](qa/RELEASE_0.73.0.json). AEC-017 remains **In progress**.
Window controls passed native interaction and image review on Ubuntu 22.04 and 24.04 under X11/Openbox/Xvfb with software rendering in the light theme. Dark-theme appearance, Wayland, physical GPUs and Windows interaction were not exercised. This does not qualify the plan-link interface's native workflow.
[Workflow and limits](PLAN_LINKS.md) · [Release scope](RELEASE_0.73.0.md).

## Added in 0.72.0

| Area | Implemented behavior and limits |
|---|---|
| Region detection | Explicit interior point, raster resolution and threshold; optional manual closure segments; actual outer contour with retained holes |
| Calibrated area | Review the boundary, deductions and selected scale in a real preview before creating one editable area measurement with one Undo |
| Refusals and history | Leaks, ambiguous boundaries, partial results and excessive geometry cannot be applied; source changes invalidate the candidate; generation inputs remain historical metadata |

Printed text and hatching can create deductions. Raster geometry does not establish physical measurement
accuracy. AEC-012 remains **In progress**. See the [workflow](REGION_FILL.md) and
[0.72 release scope](RELEASE_0.72.0.md).

Published stable with [qualification evidence](qa/RELEASE_0.72.0.json). Native interaction and broader drawing
qualification remain open.

## Added in 0.71.0

| Area | Implemented behavior and limits |
|---|---|
| Find symbols | Same-scale template matching on explicitly selected pages, similarity controls, quarter-turn rotations and candidate previews |
| Reviewed counts | Matches start unselected; accepted matches create one undoable count-marker batch; existing counts remain and are not automatically deduplicated |
| Bounded search | Cancellation, incomplete coverage and resource limits remain explicit; source changes invalidate the review; arbitrary angles and automatic scale matching are unsupported |

Published with [qualification evidence](qa/RELEASE_0.71.0.json). Native interaction and broader drawing
qualification remain open; AEC-011 is **In progress**. See [workflow and limits](VISUAL_SEARCH.md).

## Added in 0.70.0

| Area | Implemented behavior and limits |
|---|---|
| Numeric field units | Optional declared units on Number fields; one type/unit definition per column; explicit No unit; numeric sorting and value-with-unit search |
| Geometric formulas | Checked six-decimal arithmetic within mm, cm, m, in or ft families, exponents one to three; equal-unit division is dimensionless; no mixed-family conversion, inverse units, mass or density |
| Exports and history | Separate CSV unit cells only for unit-bearing columns; PDF/JSON/XLSX retain values and units; v2 Changes adds typed before/after custom fields as inert text |
| Compatibility | Unitless v1 serialization, comparison hashes and XLSX columns preserved; current or pending unit-bearing baseline rows require v2; unsupported/mislabeled metadata refused |

AEC-007, AEC-018 and AEC-019 remain **In progress**. Native desktop interaction, native spreadsheet interoperability and broader reader qualification remain open. Packages remain unsigned; ARM64 hardware, physical printing, human accessibility and private project plans retain their separate limits. No overall performance improvement, universal interoperability or PDF/A/PDF/UA conformance is claimed.

The local development source passed formatting, strict workspace Clippy and desktop/CLI compilation, with **1,094 tests passed, zero failed and 15 ignored**. The English and French catalogs contain **1,871 matching keys**. The independent units corpus passed **1,992 assertions across 96 commands** (71 development CLI 0.70 commands, four packaged CLI 0.69 compatibility commands and 21 qpdf checks). These results are scoped to the recorded development binaries; they do not qualify a downloaded 0.70 release package.

Qualification: Windows: 1,094 passed, 15 ignored; Linux: 1,075 passed, 10 ignored; zero failures. 7,019 assertions in nine downloaded-package corpora. See [qualification evidence](qa/RELEASE_0.70.0.json).

## Retained from 0.69.0

| Area | Implemented behavior and limits |
|---|---|
| Selection-bound reports | PDF/CSV/JSON from the current register or quantity selection, preserving filters, row order and calculated values; real paginated PDF preview; new files only |
| PDF presentation | Quantities, separate currency/precision subtotals and explicit missing/error coverage first; issue groups, actual supported style samples and annotation details; technical provenance/settings in an appendix |
| Report bounds | 250 PDF pages, 32 MiB per output, four MiB of text and 256 issue/legend groups; unsupported glyphs or complex shaping refused; no PDF/A/PDF/UA claim |
| Quantity workbooks | Local XLSX snapshots of rows, totals, changes and provenance; exact decimals retained when spreadsheet numbers would lose precision; annotation text does not become formulas or macros |
| Reviewed refresh | Separate manifest, unique annotation-ID matching, explicit missing/ambiguous/unquantified entries and opt-in new identities; recalculation from the current PDF; no arbitrary XLSX import or automatic synchronization |

Native desktop and native spreadsheet interaction remain untested. AEC-018 and AEC-019 remain
**In progress**. Windows: 1,083 passed, 15 ignored; Linux: 1,064 passed, 10 ignored. Eight independent package corpora passed 5,027 assertions across 326 commands. [Qualification evidence](qa/RELEASE_0.69.0.json).
See [reports](ISSUE_REPORTS.md) and [workbooks](QUANTITY_WORKBOOKS.md) for source and permission guards.

## Retained from 0.67.0

| Area | Implemented behavior and limits |
|---|---|
| Plan links | Exact indexed sheet identifiers in existing searchable text; explicit destination review, relative PDF links and new-copy output. Published 0.73 adds up to 16 sources, 128 pages and 4,096 occurrences per batch with per-file receipts; native plan-link interaction and broader interoperability remain open |
| Revision outlines | One old/new page pair; manual or two-point alignment; classified raster proposals and individual confirmation; at most 1,536 pixels on the longest side, 512 proposals and 64 selected decisions; no claim of full drawing equivalence |
| Rooms and zones | Up to 64 named rectangular zones per page and eight hierarchy levels; full-bounds assignment and shared register/quantity filters; boundary, overlap, unknown and stale records remain explicit; no area inference or scale changes |
| Compact tables | Active zone filters in Filters, with coverage and quantity summaries under Totals |

Exact-source CI recorded 1,045 Windows and 1,026 Linux tests passed, with ignored results separate.
Portable upgrades cover 45 baselines plus four guards; MSI covers 43 baselines / 215 logs. Independent
development checks recorded 128 SuniPDF CLI plus 28 qpdf processes / 2,541 assertions. Native interaction with the new 0.67
plan workflows remains untested. See [0.67 qualification and limits](RELEASE_0.67.0.md).

## Delivered in 0.64.0

Exact-source CI, package checks and public downloads are verified. The downloaded Windows CLI passed
82 commands / 1,062 assertions; native development-build workflows have separate bounded evidence.
See [0.64 qualification](RELEASE_0.64.0.md).

| Area | Implemented behavior and limits |
|---|---|
| Plan sets | Portable manifests, stable identities, fingerprints, validation/refresh/relink, metadata and filename/page filtering, natural sorting, discipline/revision/file grouping and guarded navigation; no automatic revision approval |
| Revision transfer | Explicit source-bound page matches, selected supported native shapes, bounded alignment, real destination previews, preserved revised contents and fresh-copy output; unsupported semantic data is excluded, broader batch processing remains open |

## Retained from 0.63.0

| Area | Implemented behavior and limits |
|---|---|
| Deducted areas and volumes | Strict interior holes, retained physical depth, gross/deducted/net calculations, cubic quantities/prices and full-contour editing; constant depth only, editable third-party measurement exchange unqualified |
| FDF | Value-only export/import with typed button states, Unicode, distinct empty-choice/clear semantics, bounded expansion and preserved appearances during repeated imports |

[0.63 evidence](qa/RELEASE_0.63.0.json) remains a historical record of that release's own checks.

## Delivered in 0.62.0

The additions below are included in the published release. Their bounded qualification is recorded
in [release evidence](qa/RELEASE_0.62.0.json).

| Area | Delivered behavior |
|---|---|
| Formula summaries | Sum rounded row results across all filtered rows; separate formula/group/unit/precision, expose contributing/error counts and error categories, and leave totals absent when all rows fail |
| Quantity exports | Reuse one formula evaluation for cells and summaries; preserve version 1 coverage meanings, add additive/angular/excluded counts, and retain signed numeric CSV values with free-text formula neutralization |
| Measurement provenance | Distinguish current, legacy-unverified and changed-page records using bounded page evidence; retain historical values and vertices for review while excluding them from usable quantities, costs and quantity-dependent formulas |
| Explicit recalibration | Historical lengths and areas require a current session reference, review and Preview/Apply; historical angles require recreation. Unit/precision/field changes do not restore provenance; non-current records cannot supply a reusable scale or enter geometry editing |
| Saved reference migration | New page defaults and named regions include initial layer visibility and page transparency state; older references require explicit recalibration or removal instead of automatic reuse |
| CLI output | Refuse an existing destination at final creation, including a file created during processing; explicit replacement remains subject to each command's source/output guards |
| Document restoration | Open and validate a replacement before changing the current document or password; failure retains the live state |
| Certificate signing | Reject encrypted inputs even when an empty user password allowed automatic decryption |
| Tab lifecycle | Failed resume retains bytes, history and recovery identity for retry; cancelled register work reloads and obsolete searches/caches are invalidated; edits during suspension synchronize page, label, selection and navigation state |

See [project status](PROJECT_STATUS.md) and
[formula subtotal details](QUANTITY_TABLE.md#formula-subtotals-and-coverage), plus
[historical measurement handling](QUANTITY_TABLE.md#measurement-provenance).

## Reading and recovery

| Feature | Supported behavior and limits |
|---|---|
| PDF opening | PDFium rendering, including RC4/AES-128/AES-256 documents with the required password |
| Navigation | Tabs, previous/next/go-to page, physical `#N` and unique page-label navigation, bookmarks and per-tab position/zoom history |
| Layout and zoom | Single, continuous, two-page and cover-first Book views; fit page/width, actual size, view rotation and full screen; 10–3200% zoom |
| Rendering | Engine work off the UI thread, tiled rendering, bounded tile/texture caches and cached page layout |
| Search and selection | Cancellable page-by-page full-text search, highlighting, logical cross-page selection, double-click words and edge autoscroll; arbitrary untagged column order is not inferred |
| Thumbnails and outlines | Virtualized thumbnails; bookmark add/rename/retarget/move/delete with Undo; bounded to 4,096 bookmarks and 32 levels, with cycles refused |
| Recovery | Snapshots every 60 seconds and on tab suspension; original files unchanged, encryption retained, no passwords stored; recovery through Save As |
| Appearance and commands | Light/dark themes, English/French interface, configurable shortcuts, Ctrl+K command palette and tools in five task families |
| Printing | Windows dialog, ranges/copies/orientation, current edits, bounded raster output and cancellation; physical printer qualification remains open; unavailable in Ubuntu |

[Reader workspace](READER_WORKSPACE.md) · [User guide](USER_GUIDE.en.md) · [Keyboard shortcuts](KEYBOARD_SHORTCUTS.md)

## Plan measurements and review

| Feature | Supported behavior and limits |
|---|---|
| On-page measurements | Length, polyline length, perimeter, simple-polygon area and angle; explicit calibration, metric/imperial units, decimal or feet/inches entry, preview and Undo |
| Saved scales | Independent saved measurement scales, session/page defaults and up to 64 named regions per page; overlap/crossing geometry requires an explicit scale choice |
| Vector snapping | Eligible endpoints and straight-line intersections with crop, rotation, transforms and supported convex clips; scans, text/image bounds and unsupported clips do not supply points |
| Individual edits | Change a saved scale/unit/precision; move vertices, add/remove eligible vertices, select coincident points by index; Preview/Apply and one Undo preserve identity, replies and quantity fields |
| Batch recalibration | Preview selected page defaults and supported saved measurements, then apply atomically with one Undo; up to 512 pages, 4,096 records and 128 MiB per document; signed or stale inputs refused |
| Counts and register | Persistent count markers, groups, review status, discipline and custom text fields; built-in or lexical custom-field sorting, missing values last, navigation and guarded CSV/JSON export |
| Quantity tables | Actual count/measurement quantities, typed fields, numeric/date/text/choice sorting and explicit prices; quantity totals retain their units, monetary totals keep currencies and rounding groups separate; missing values stay distinct from zero |
| Report formulas | Up to eight tab-local arithmetic columns using decimal constants, numeric fields and an exact-unit quantity; six-decimal checked arithmetic, explicit cell errors and JSON configuration reuse; subtotals separate groups, units and precision; no PDF field mutation |
| Issue reports | Real paginated PDF preview and shared PDF/CSV/JSON selection, totals, coverage, issue groups and supported style legends; technical appendix; new output files, source guards and explicit font limits |
| Quantity workbooks | XLSX snapshots and manifest-based reviewed refresh; exact values, separate units/currencies and explicit unresolved records; no workbook overwrite, macros, arbitrary XLSX import or silent synchronization |
| Table workspace | Docked or floating tables with saved placement/height, row-to-annotation navigation and cached filtering/sorting/totals |
| Annotation library | Local templates and flat groups of up to 16 primitives; capture supported saved native shapes/lines/polygons/clouds with declared provenance and exact appearance checks; placement preview, fresh identities, one Undo and bounded JSON import/export; arbitrary/text/image/stamp capture, measurements and review threads excluded |
| Comparison | Local text/geometry comparison and two-page magenta/cyan overlays; manual or two-point alignment and new PNG/JSON exports; automatic registration remains open; reviewed revision outlines are part of the 0.67 release, with native workflow interaction still unqualified |
| Multi-file search | Literal text search across explicitly selected local PDFs, page excerpts/navigation, per-file failures and cancellation; partial results remain explicit |

Saved scales validate page content and resources within documented bounds; unsupported or stale records require explicit
recalibration/removal. Changing a page/region scale alone affects future drawing; batch recalibration is a separate action.
Private real-project plan qualification remains deferred.

[Plan workspace](PLAN_WORKSPACE.md) · [Measurements](MEASUREMENTS.md) · [Geometry editing](MEASUREMENT_GEOMETRY.md) ·
[Register](MARKUP_REGISTER.md) · [Quantity tables](QUANTITY_TABLE.md) · [Library](ANNOTATION_LIBRARY.md) ·
[Revision overlays](REVISION_OVERLAY.md) · [Local review](LOCAL_REVIEW.md)

## Editing and page organization

| Feature | Supported behavior and limits |
|---|---|
| Annotations | Notes, highlight/underline/strikeout, shapes, grouped ink, FreeText, polygons/polylines, revision clouds, stamps and attachments; supported edits use preview and Undo |
| Comments | Replies, authors, dates, review states and thread deletion; third-party reader interoperability is qualified separately |
| Ink | Group up to 64 strokes, each up to 4,000 points, into one annotation; erase whole touched strokes; no partial-stroke erasure, pressure support or unfinished-draft recovery |
| Text and fonts | Standard/eligible embedded fonts, on-page FreeText and bounded paragraph preview/reflow; unsupported glyphs, fonts, shaping or graphics states are refused |
| Objects | Selection, move/resize/rotate/opacity, grouped transforms, object-order drag-and-drop, cut/copy/grouped paste and explicit editable paste of supported objects |
| Links | Safe URI and page destinations, including named/XYZ/Fit/FitH targets; unsupported active actions are not run |
| Page operations | Rotate, delete, insert, reorder, blank pages, merge, split and extract; bounded preservation of supported outlines, destinations, forms, layers and attachments |
| Crop and resize | Crop preview/restore and optional content/annotation resizing; cropping hides content without redacting it; incompatible measured-page resizing is refused |
| Page labels | Author/reset supported label styles and rebase ranges during page operations; printed page numbers are unchanged and duplicate labels remain explicit |
| Decorations | Text/image watermarks, page numbers, six-zone headers/footers and named local presets; explicit page scope, margins and overflow checks; no odd/even rules or background-layer mode |
| Image stamps | Embedded pixels with alpha, aspect fitting and preview |
| Undo/Redo | Snapshot-based changes retained across Save; independent local-library/preferences changes do not acquire PDF Undo history |

[Rich annotations](RICH_ANNOTATIONS.md) · [Fonts](FONT_EMBEDDING.md) · [Object order](OBJECT_ORDER.md) ·
[Page structures](BOOKMARKS_AND_PAGE_STRUCTURE.md) · [Page decorations](PAGE_DECORATIONS.md)

## Forms, conversion and OCR

| Feature | Supported behavior and limits |
|---|---|
| Fill forms | AcroForm text, checkbox, radio, ordinary/editable combo and single/multiple list widgets; hierarchical/multi-widget fields, regenerated appearances, Apply/Discard and read-error retry |
| Create fields | The same five field types through mouse-drawn or numeric rectangles, unique names, rotated/cropped pages and Undo; existing-field redesign is not included |
| Form interchange | JSON/XFDF/value-only FDF export and bounded import; typed button states, Unicode and explicit empty-choice/clear handling; no scripts, attachments or rich-text interchange |
| Flatten | Form fields and supported annotation appearances; non-widget annotations without usable appearances are preserved where supported; malformed/unsafe graphs are refused atomically |
| XFA and accessibility | XFA detected and read-only; scripts never run; PDF/UA tagging/conformance and full assistive-technology qualification remain open |
| Image/PDF conversion | PNG/JPEG/BMP/GIF/TIFF/WebP to PDF, all eight JPEG EXIF orientations, PDF to PNG and text; bounded raster allocations and positive finite page sizes |
| Image compression | Supported RGB/Gray images, JPEG quality/max-side controls, masks/profiles preserved, no-gain byte copy; signed/restricted inputs refused |
| PDF to Office | Simple single-column DOCX reflow, aligned text grids as literal XLSX strings, PPTX page images; original layout/fonts, formulas and editable slides are not reconstructed |
| Office to PDF | Windows workflow using an already-installed supported LibreOffice, guarded macro-free inputs, isolated profiles and new copies; unavailable explicitly when prerequisites are missing |
| Offline OCR | Local English/French recognition, invisible text, existing-text skipping, progress/cancellation and Undo; recognition accuracy depends on the scan |
| Batches | Sequential compression/OCR and bounded Office conversion to new copies; completed copies remain if later inputs fail or cancellation occurs |

[User guide](USER_GUIDE.en.md) · [Office conversion](OFFICE_CONVERSION.md) · [Ubuntu limits](UBUNTU.md)

## Security and signatures

| Feature | Supported behavior and limits |
|---|---|
| Encryption | Open supported encrypted documents and honor permissions; AES-128/256 protection; changing encryption requires the owner password; AES-128 passwords are limited to Latin-1 |
| Redaction | Preview, fill/replacement text, persistent unapplied marks and applied redaction; safe native reconstruction where supported, raster fallback otherwise; marks alone do not remove content |
| Sanitization | Separate cleaned copy with supported hidden-data removal and full rewrite; previous copies, recovery, backups and Undo are not erased |
| PDF inspection | Bounded read-only inventory of saved-file structures and actions; findings do not prove safety or PDF/A/PDF/UA conformance |
| PDF/A metadata | Supported metadata preservation; no conversion, validation or certification of PDF/A/PDF/UA conformance |
| Visual signatures | Drawn, typed or imported marks, optional smoothing and paper-shadow cleanup; no cryptographic identity proof; persistent encrypted storage requires Windows |
| Certificate signatures | Windows PFX/P12 signing to a new copy, CMS integrity checks, separate chain/revocation and supported RFC3161 verification; offline default with consent for network checks |
| Signature limits | Encrypted signing refused; no timestamp creation, archival LTV, PKCS#11 or AATL/EUTL qualification; unknown/offline results are never treated as trusted |

[Security model](SECURITY_MODEL.md) · [Redaction](REDACTION.md) · [Inspection](PDF_INSPECTION.md) ·
[Visual signatures](VISUAL_SIGNATURE_PREPARATION.md) · [Certificate signatures](SIGNATURE_SECURITY.md)

## Distribution and automation

| Feature | Supported behavior and limits |
|---|---|
| Windows x64 | MSI and portable ZIP, automatic update notification, verified download and user-triggered installation; packages unsigned |
| Windows ARM64 | Portable ZIP cross-built in CI; native ARM64 runtime remains unqualified |
| Ubuntu amd64 | DEB for Ubuntu 22.04/24.04, desktop and CLI, local OCR and XDG settings/recovery; update notification leads to manual package installation |
| CLI | JSON output, stable exit codes, shared command registry and guarded writes; run `sunipdf-cli --help` or `sunipdf-cli commands` for the current inventory |
| Rendering backends | OpenGL default, optional wgpu and software-renderer checks; physical GPU/window behavior requires separate qualification |
| Dependencies | Versioned license inventory and package-content checks; notices accompany distributions |

Remaining hardware, accessibility, interoperability and release requirements are tracked in
[known issues](KNOWN_ISSUES.md), the [roadmap](ROADMAP.md) and [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Historical results belong to the [changelog](../CHANGELOG.md) and [qualification records](qa/).
