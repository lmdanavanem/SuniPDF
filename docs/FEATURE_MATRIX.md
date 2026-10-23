# SuniPDF feature matrix

Current scope: **0.60.0**. Features below are implemented within the stated limits.
[Project status](PROJECT_STATUS.md) records executed checks; implementation does not imply qualification on every document,
physical device or third-party reader. Planned work is tracked in the [roadmap](ROADMAP.md).

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
| Appearance and commands | Light/dark themes, English/French interface, configurable shortcuts, Ctrl+K command palette and 18 tools in five task families |
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
| Counts and register | Persistent count markers, groups, review status, discipline and custom fields; filtering, sorting, navigation and guarded CSV/JSON export |
| Quantity tables | Actual count/measurement quantities, typed fields and explicit prices; compatible-unit totals keep currencies and rounding groups separate; missing values stay distinct from zero |
| Table workspace | Docked or floating tables with saved placement/height, row-to-annotation navigation and cached filtering/sorting/totals |
| Annotation library | Local templates and flat groups of up to 16 supported primitives; rendered placement preview, one Undo and bounded self-contained JSON import/export; measurements and review threads excluded |
| Comparison | Local text/geometry comparison and two-page magenta/cyan overlays; manual or two-point alignment and new PNG/JSON exports; automatic registration and change outlines remain open |
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
| Form interchange | JSON/XFDF export and bounded JSON/XFDF/value-only FDF import; no scripts, attachments or rich-text interchange |
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
