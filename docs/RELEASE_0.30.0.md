# SuniPDF 0.30.0

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

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) under `lmdanavanem`, release `415568680`, immutable source `4db31c0ee62cfae9d65239edf1eac93db147841e`. The five Bugs and fourteen Roadmap P2 items ship within their [documented supported limits](RELEASE_0.30.0.md). P1 Roadmap items and Suggestions remain deferred.

The [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) and [MAIN CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) passed on this source: **551 tests**, zero failures, 9 default ignored; **1047 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature outputs**, **4 MuPDF font cases**, DX12 software WARP, **14 Office round trips**, 4 licensed PDFs/744 pages, **21 portable scenarios**, **15 MSI baselines/75 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

Six public assets match the inspected packages and SHA256 digests byte for byte. The production updater verified the 26,978,019-byte x64 ZIP in 2.55 seconds without installing on this host, using the previously built test harness with an unchanged updater module. Project readback: **42 unique items, 20 Todo, 0 In Progress, 22 Done**, three saved filters and zero unsaved changes. [Evidence and limits](PROJECT_STATUS.md).

Packages remain unsigned. ARM64 is cross-built and packaged, without execution on ARM64 hardware. Physical printers, Narrator, manual updater interaction and physical GPU/window behavior still need qualification; WARP proves software offscreen rendering only. Unsupported inputs remain refused. PDF/A metadata preservation does not certify PDF/A or PDF/UA conformance.

Supported inputs and refusals: [fonts](FONT_EMBEDDING.md), [annotations](RICH_ANNOTATIONS.md), [redaction](REDACTION.md), [bookmarks and structures](BOOKMARKS_AND_PAGE_STRUCTURE.md), [Office conversion](OFFICE_CONVERSION.md), [signatures](SIGNATURE_SECURITY.md) and [sanitization](SECURITY_MODEL.md).
