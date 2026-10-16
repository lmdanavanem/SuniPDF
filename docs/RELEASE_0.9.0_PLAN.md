# SuniPDF 0.9.0 implementation scope

The seven items below were approved for implementation and publication. English remains the default
interface/documentation language, with matching French translations. PDF contents stay local.

| Feature | Scope and acceptance |
|---|---|
| Autosave/recovery | Periodic recovery snapshots and snapshots before tab suspension; original untouched; encrypted PDFs remain encrypted; no persisted passwords; restore as modified and require Save As |
| Windows printing | Current in-memory document, native printer/range/orientation/copies controls, progress and cancellation; Print to PDF output reopened and checked |
| Reading selection | Double-click word, logical reading ranges across pages, edge scrolling, forward/reverse selection, exact copied text on rotated/cropped pages |
| Two-page/book view | Shared tiled rendering, paired rows and cover-first book mode; mixed sizes, odd last page, navigation/zoom and existing memory limits |
| Form creation | Text and checkbox fields, valid unique names and display-space rectangles, rotated/cropped pages, save/reopen/fill/undo and encryption preservation |
| Local OCR | Bundled Tesseract and French/English data, progress/cancellation, searchable invisible text, appearance unchanged, skip existing text, one undo transaction |
| Digital signatures | Verify saved PDF bytes, sign a new copy with a PFX/P12 certificate, strict byte ranges, self-verification, ephemeral private keys and warnings before modifying signed documents |

The implementation reuses the single PDFium worker, atomic file writes, snapshot undo, page geometry,
command registry, native Windows APIs and existing dialogs. Expensive work stays off the UI thread.
Shared integration is serialized; local Rust/native builds use two jobs and never run cargo clean.

OCR is bundled because Windows.Media.Ocr does not officially support an unpackaged portable application.
Its runtime/data sources and checksums are pinned by `scripts/fetch-ocr.ps1`; packages include the
engine, English/French models and their licenses. The flat `licenses/` payload keeps portable updates
compatible with the 0.8.x updater.

Digital signatures establish mathematical integrity. Certificate-chain trust, revocation, trusted timestamps
and PAdES certification are outside this release; the UI must state that signer identity is not verified.
Encrypted signing may be explicitly refused. Existing signatures must not silently appear valid after edits.
Reading order follows the PDF text stream; arbitrary disordered multi-column inference is not claimed.

Validation includes engine round trips, file-preservation/error cases, headless UI interactions, native
crypto with disposable test certificates, actual local FR/EN recognition, static packaging checks, x64 CI
and ARM64 cross-compilation. Real native UI, physical printing and ARM64 hardware results are recorded only
when actually exercised. Publication requires verified packages and public SHA256 checks.
