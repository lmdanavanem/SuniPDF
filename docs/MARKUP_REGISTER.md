# Manual plan counts and annotation register

[Français](MARKUP_REGISTER.fr.md) · [Measurements](MEASUREMENTS.md) · [AEC backlog](AEC_BACKLOG.md)

The local 0.41.0 slice adds persistent manual counts and an all-page markup register. These tools run offline and remain free and open source.

## Count items on a plan

1. Open **Comment → Manual count…**, enter an equipment/type group and a marker size of 8–72 PDF points, then choose **Start counting**.
2. Click each item’s centre. Each click creates one real PDF Stamp annotation, drawn as a blue circle and cross. The entire marker must fit inside the displayed page. A drag of more than two points does not place a marker.
3. Use **Select annotation** to move a marker, or delete it from the comments panel. Undo/Redo restores these changes.
4. Open **Annotation register** to inspect group totals. **Edit fields** can regroup a marker without changing its identity, appearance or position. Regrouping has one Undo step.

Each marker counts as one; a separate editable total cannot drift away from the markers. Markers survive save/reopen, rotation and CropBox offsets. Duplicated count `/NM` identities are refused on export. Standard replies and PDF review states remain available. Hidden markers remain in document totals and carry their hidden flag in exports.

Only supported manual SuniPDF markers are counted. Generic imported stamps and proposed visual-search matches are unquantified. Symbol detection, automatic counting and third-party proprietary count metadata import are not implemented.

## Review all annotations

**Comment → Annotation register** reads all pages on the document worker. Columns include page/index, type, author, PDF modification date, PDF Review state, business status, discipline, count group and comment. Enable **Include replies and review states** to include child records.

- Search ignores case and includes custom text keys/values. Named filters require exact text. The date filter searches the original PDF date, for example `D:202610`.
- Sort by page, author, date, type, PDF Review state, business status, discipline or count group, ascending/descending. Dates sort lexically in PDF syntax, without timezone normalization.
- **View on page** closes the table, navigates to the rectangle and selects the actual annotation in the comments panel. Unknown rectangles have no navigation action.
- **Edit fields** replaces business status, discipline and up to 16 custom text fields, entered one `KEY=VALUE` per line. Blank optional fields are cleared; duplicate keys are refused. Count markers also expose their group. Changes have one Undo step.
- Business statuses such as `Ready for site` do not replace standard PDF Review states such as `Accepted`. Custom fields are text; formulas are not evaluated. Typed columns and custom-key sorting remain AEC-004/007 work.

The table displays 50 rows at a time; exports include **every matching row**. Refresh reloads the current snapshot. An edit checks snapshot revision and annotation identity to prevent shifted-index or stale-target mistakes. Failed edits preserve the draft and add no successful Undo step.

## Export

JSON schema `sunipdf-annotation-register`, version 1, records the current in-memory snapshot SHA256, page count, filters/sort, rows and group totals. Every total references actual markers by physical page, zero-based annotation index and `/NM`. Optional unknowns remain `null`; rectangles use displayed-page points with bottom-left origin.

CSV contains one row per matching annotation, source revision and optional `quantity=1` for supported count markers. Empty cells mean unknown; JSON preserves null versus empty text. All cells are quoted. Leading spreadsheet formula prefixes are escaped with an apostrophe, including prefixes after whitespace, control characters or a BOM. Custom fields occupy a JSON-object cell. CSV has no synthetic subtotal rows; JSON contains totals and marker references.

Existing output files and originals are never replaced, even after a system overwrite prompt or with CLI `--overwrite`; choose a new path. Register viewing/export requires PDF copy permission. Adding, moving, deleting, regrouping and replying to annotations require annotation permission. Shared annotation writers enforce this separately from page-geometry maintenance. AES-128/AES-256 protection is retained, including when general modification is denied.

```powershell
sunipdf-cli count plan.pdf --group Lights --point 120,90 --page 1 --size 18 --out counted.pdf
sunipdf-cli annotation-register counted.pdf --count-group Lights
sunipdf-cli annotation-register counted.pdf --query A101 --sort author --descending --out register.json
sunipdf-cli annotation-register counted.pdf --format csv --out register.csv
sunipdf-cli annotation-fields counted.pdf --page 1 --index 0 --count-group Sockets --business-status "Ready for site" --discipline Electrical --field Room=A101 --out reviewed.pdf
```

Pages are physical and one-based; `--index` is zero-based in `/Annots`, as returned by the register. `annotation-fields` replaces register fields and clears omitted optional fields. CSV requires `--out`; JSON also supports stdout. Existing password environment/stdin flags work on all three commands.

## Bounds and evidence

The current PDF snapshot is limited to 128 MiB. Limits: 1000 pages, 10000 annotations in total, 4096 per page, 4 MiB of raw text/metadata and 64 levels of replies. Standard strings are limited to 16 KiB; stored SuniPDF records to 32 KiB. Count groups contain 1–512 UTF-8 bytes; custom fields use at most 16 keys of 64 bytes and values of 1024 bytes. Newly entered business/custom values refuse control characters.

Unreadable arrays, malformed/unsupported SuniPDF records, cyclic/deep replies and ambiguous count identities fail the complete register; they never become plausible zero or partial totals. This is a SuniPDF exchange format independent of proprietary extensions.

Regression coverage includes four rotations/CropBox offsets, PDFium/MuPDF rendering, qpdf, identities, regrouping, filters, Unicode, guarded CSV/JSON, original/output preservation, AES and both credentials. Core/worker tests cover stale edits, Undo/Redo and deletion. Release evidence records qualification separately; synthetic plans do not establish universal compatibility. [1.0 criteria remain open](RELEASE_1.0_CRITERIA.md).

[0.41.0 qualification evidence](qa/RELEASE_0.41.0.json).
