# SuniPDF 0.30.0

[Français](RELEASE_0.30.0.fr.md)

This version covers five Bugs and fourteen P2 Roadmap items. Supported inputs and limits are described
below; executed validation and public download evidence are recorded in [Project status](PROJECT_STATUS.md).

## Editing and annotations

- Preview multiline paragraph reflow with actual PDF font metrics before applying it. Unsupported
  text, fonts or graphics states are refused while keeping the PDF and draft intact.
- Paste copied PDF content as individually editable objects through **Paste editable**. The
  existing grouped paste remains available. Unsafe or unsupported resource graphs are refused.
- Create embedded-font FreeText, polygons, polylines, clouds, standard stamps and file attachments.
  Erase complete ink strokes touched by the eraser while preserving untouched strokes and comments.
- Author named destinations and precise XYZ/Fit/FitH link views. Empty axes and zoom retain the
  reader's current values; target coordinates use the displayed page's crop and rotation.
- Preserve the selected rotation pivot, handle opacity replacement explicitly and optionally scale
  annotation geometry when resizing page content.

## Pages and document safety

- Add, rename, move, retarget and delete bookmarks through the desktop panel and CLI.
- Preserve bounded links, form fields, layers, attachments and outlines during merge, insertion and
  extraction; malformed or unrepresentable structures fail before the document changes.
- Keep semantic page labels with their page content after structural operations. Duplicate labels
  remain explicit; `#N` always selects a physical page number.
- Preview redaction before applying it, save unapplied standard redaction marks, choose fill and overlay,
  and retain unredacted selectable text when a safe native split can be established.
- Save a separate sanitized copy that removes supported hidden metadata, scripts, bookmarks, named
  destinations, attachments and unused font resources, followed by a full rewrite and absence checks.
  Previous copies, backups, recovery files and Undo history need their own cleanup.

## Local conversion and security

- Convert supported macro-free Office files with an already-installed supported local LibreOffice,
  using cancellable batches, new-copy output and reusable Standard/Print profiles.
- Export simple PDF text to DOCX, aligned text rows to XLSX and rendered pages to PPTX. These are
  bounded exports with measured limits, rather than promises of original layout reconstruction.
- Separate digital-signature integrity from Windows certificate trust, revocation and RFC3161
  timestamp results. Network retrieval requires explicit consent; offline/unavailable results stay
  explicit. Embedded LTV data is reported without claiming archival validation.
- Protect optional local signature-image storage with current-user Windows DPAPI and guarded migration.
- Discover Unicode Windows font paths; subset eligible static TTF/OTF/TTC faces and reuse identical
  font programs. Respect embedding restrictions and refuse unsupported font formats or shaping.
- Keep OpenGL as the default renderer, with optional wgpu and a native DX12 software-renderer check.
- Ship exact-version dependency/component license evidence checked against ZIP and MSI contents.

## Validation and distribution

Publication requires x64 formatting/Clippy/workspace tests, ARM64 cross-build, independent font and
feature-reader checks, qpdf/corpus regression, measured Office conversions, portable upgrades and MSI
transaction checks. The Windows x64 MSI and x64/ARM64 ZIPs each include a SHA256 sidecar.

Executed results and public asset hashes will be recorded in [Project status](PROJECT_STATUS.md).
Packages remain unsigned; ARM64 hardware, physical printers and manual native GPU/window behavior
require separate qualification. Detailed supported inputs and refusals are documented in
[font embedding](FONT_EMBEDDING.md), [rich annotations](RICH_ANNOTATIONS.md),
[redaction](REDACTION.md),
[bookmarks and structure](BOOKMARKS_AND_PAGE_STRUCTURE.md), [Office conversion](OFFICE_CONVERSION.md),
[signature security](SIGNATURE_SECURITY.md) and [document sanitization](SECURITY_MODEL.md).
