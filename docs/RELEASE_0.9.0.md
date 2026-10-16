# SuniPDF 0.9.0

This release adds recovery, printing, richer reading controls, form creation, offline OCR and certificate signatures. English is the default language; French remains fully available.

- **Autosave and recovery:** recovery copies every 60 seconds and when switching documents. Restore work after an unexpected shutdown, then use Save As. Encrypted PDFs keep their protection; passwords are never stored.
- **Windows printing:** print the current document, including unsaved edits, using the Windows printer dialog, page ranges and driver settings. Progress and cancellation are available.
- **Text selection:** select logical text ranges across pages, drag in either direction, double-click a word and copy the selected text.
- **Two-page and Book views:** read paired pages, with an optional separate cover page, using the existing tiled renderer.
- **Prepare a form:** create text and checkbox fields, then fill, save and reopen them. Rotated and cropped pages are supported.
- **Offline English/French OCR:** the recognition engine and both language models are included. Add searchable text to scans without changing their appearance; pages with existing text are skipped.
- **Digital signatures:** verify the integrity of saved PDF signatures or sign a new copy with a PFX/P12 certificate. Modifying and saving a signed document requires confirmation.

The CLI adds `add-form-field`, `ocr`, `signatures` and `sign`. PFX passwords are supplied through an environment variable or standard input.

Validation: 318 automated tests passed, with no Clippy warnings and 804 matching English/French translation keys. A separate Windows Print to PDF check reopened the printed output and confirmed that unsaved edits were included and the original PDF was unchanged.

Download the **x64 MSI** for installation, or the **x64/ARM64 portable ZIP**. Matching SHA256 files are attached. Existing versions with the updater can notify you at startup and download the new version for installation.

**Limits:** certificate identity trust, revocation and trusted timestamps are not checked; encrypted PDF signing is refused. Printing is raster based, up to 300 dpi. OCR accuracy depends on the scan. Physical printers, native dialog interactions, installation/upgrade and ARM64 hardware remain unverified.
