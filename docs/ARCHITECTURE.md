# SuniPDF Architecture

[Français](ARCHITECTURE.fr.md)

## Principles

1. **Engine first.** Every PDF operation lives in `crates/sunipdf-core` (and, later, in domain crates
   that expose only Rust types). The UI, the CLI, the tests and automation all call the same API.
2. **The UI never touches PDFium.** It carries neither a `pdfium-render` dependency nor any format logic.
3. **A single engine thread.** PDFium is not thread-safe: the application confines `Engine` and the open
   documents to a worker thread (`apps/sunipdf-desktop/src/worker.rs`) and communicates through `mpsc` channels.
   The UI thread never blocks on a PDF operation.
4. **Typed errors, text translated at the boundary.** `EngineError` carries a stable code (`code()`);
   only the UI maps it to an i18n key (`app.rs::error_key`), the CLI emits the code as JSON.
5. **Atomic writes.** Neighbouring temporary file then replacement (`sunipdf_core::write_atomic`) for PDF,
   recovery and settings output. Independent qpdf validation remains a separate QA task.
6. **Local PDF processing, no telemetry.** OCR and signatures run locally. The opt-out update check and
   explicit release download contact GitHub without sending PDF content; this is the only application network feature.
7. **No PDF JavaScript.** PDFium binaries compiled without V8 or XFA (`args.gn`: `pdf_enable_v8=false`).

## Current workspace

```
SuniPDF/
├── apps/
│   ├── sunipdf-desktop/      `sunipdf` binary (egui/eframe) — main.rs, app.rs (state + document), shell.rs (chrome: tabs,
│   │                         toolbar, "All tools" panel, home, rail, palette), workspace.rs (hub: tool table
│   │                         → registry commands, active tool, contextual bar), worker.rs, settings.rs, theme.rs, icons.rs,
│   │                         editor.rs (selection, handles, rotation, object clipboard, object bar), sign.rs
│   │                         (visual signature: pad/type/import, local library, fitted page batches, date), inline_text.rs
│   │                         (text on the page, font picker), objects.rs (Objects panel), pagesetup.rs (crop,
│   │                         resize, blank page), annot_adv.rs (ink, free text, shapes, links, annotation selection, reply threads), view.rs (continuous / single-page view: virtualised layout,
│   │                         wanted tiles, texture pool, anchored zoom)
│   └── sunipdf-cli/          `sunipdf-cli` binary (clap) — JSON output, stable error codes
├── crates/
│   ├── sunipdf-core/         Engine, Document, RenderedPage, EngineError; tiles.rs (tile geometry), cache.rs (LRU
│   │                         pages + tiles); edit.rs (objects), geometry.rs (display space), annot_write.rs / appearance.rs / links.rs
│   │                         (advanced annotations: lopdf writes with an explicit `/AP`, PDFium renders and reads back),
│   │                         clipboard.rs (copy/paste of objects), fonts.rs (system fonts), boxes.rs (page boxes),
│   │                         signature_image.rs (visual signature: strokes, background removal, PNG; native PDFium typed preview and atomic fitted page batches);
│   │                         tests/fixtures/*.pdf (including boxes.pdf: rotated and cropped pages)
│   └── sunipdf-i18n/         Language, tr(), tr_args(), embedded catalogues
├── assets/i18n/              fr-FR.json, en-US.json (source of truth for all texts)
├── third_party/pdfium/       PDFium binaries (ignored by git, scripts/fetch-pdfium.ps1)
├── scripts/                  dev-env.ps1, fetch-pdfium.ps1, start.ps1, check-i18n.ps1, make_fixtures.py, dev-screenshot.ps1 (window capture)
├── docs/                     this documentation
```

## "Open → display → navigate → close" flow

```
UI (main thread)                          Worker (thread "sunipdf-engine")
---------------------                     --------------------------------
Request::Open{path,pw}  ───────────────▶  Engine::open → Response::Opened(DocumentInfo)
                                          (or OpenFailed{PasswordRequired…} → password dialog)
computes zoom, layout, visible area
Request::SetWanted{epoch, [TileJob]} ──▶  replaces the queue of wanted tiles (nothing is rendered right away)
                                          between two requests, ONE tile: LRU cache or
                                          Document::render_region → Response::Tile{epoch, job, RGBA}
egui texture ← RGBA (one tile per response); different epoch → ignored
Request::Close ────────────────────────▶  drop(Document)
```

## Continuous view and tiles (SUNI-012b, D038)

- **Page window**: in continuous scrolling, the whole document; in single-page mode, the current page; a single
  machinery (`view.rs`: `Layout`, `paint_slot`, `handle_page_event`) serves both modes. `Layout` places the
  pages (margin 16, gap 12); only those intersecting the visible area create widgets, requests and
  textures (binary search), plus the page of an ongoing drag or of the active text editor.
- **Tiles** (`sunipdf-core::tiles`): whole page up to 4096 px and 8 Mpx, otherwise a 1024 px grid anchored at
  (0, 0), bounded by the GPU's `max_texture_side`; `Document::render_region` renders a region without allocating the page
  (negative PDFium origin), up to a page side of 131,072 px. Scale quantised to 1/200 (`quantize_milli`).
- **Cancellation by replacement**: the UI sends the list of wanted tiles (visible ones from the centre outwards,
  then the edge of neighbouring pages, 24 at most) only when it changes; a tile that has started runs to completion.
  The epoch changes on every open, tab switch or edit: responses from another epoch are discarded.
- **Texture pool** (`TexturePool`, 192 MiB, LRU, never a texture of the current frame); during a zoom
  or after an edit (`DocState.pages_unchanged`), textures at another scale or stale ones stay
  displayed, stretched, until the new tiles arrive; the engine cache (256 MiB) serves tiles and thumbnails.
- **Scrolling**: requests (`ScrollRequest`: page, point of a page, offset, relative) are resolved on the next
  frame; the current page follows the dominant page, except when locked after a "go to"; zoom keeps the anchored point
  (page, fraction) under the pointer or at the centre.
`render_page` (image export, CLI without `--region`) stays capped at `MAX_RENDER_SIDE_PX` (16,384 px).

## Planned crates (created only when a real feature justifies them)

| Crate | Role | Phase |
|---|---|---|
| `sunipdf-render` | not created (D038): tiles and cache live in `sunipdf-core::{tiles, cache}`; to be extracted if a second consumer appears | — |
| `sunipdf-pages` | merge, split, extract, rotate, reorder (PDFium `FPDF_ImportPages`, `FPDFPage_*`) | P2 |
| `sunipdf-editor` | annotations, text, images, watermarks | P2–P3 |
| `sunipdf-forms` | not created (D061): AcroForm lives in `sunipdf-core::{forms, form_data}` (lopdf model and writing, PDFium appearances, JSON/XFDF and value-only FDF import) | — |
| `sunipdf-convert` | PDF↔images, text, Office via a detected external converter | P4 |
| `sunipdf-ocr` | not created: offline OCR lives in `sunipdf-core::ocr`; bundled runtime and models | — |
| `sunipdf-security` | encryption (outside PDFium), permissions, redaction, metadata | P5 |
| `sunipdf-signatures` | not created: CMS/PKCS#12 verification and signing live in `sunipdf-core::certificate`; trusted timestamps remain planned | — |
| `sunipdf-automation` | JSON-Schema command registry, batches, opt-in MCP | P5 |
| `sunipdf-platform` | not created: native printing lives in desktop `printing.rs`; further platform integration remains planned | — |
| `sunipdf-ui` | reusable egui components (if the app grows) | P3 |

## Tabs (SUNI-012a)

The engine holds only one active document. A tab switch sends `Suspend` (the engine returns
the document bytes, its password and its undo history) then `Resume`/`ResumeDoc` (reload
from the bytes via `Engine::open_bytes`). The document's interface state (page, zoom, thumbnails, search,
selection, annotations, bookmarks, etc.) is exchanged through `tabs.rs::take_tab` / `put_tab`. Cost: one PDFium
serialisation per tab switch; memory: one snapshot per inactive tab. The object clipboard
(`ObjectClip`, a one-page PDF in memory) lives in the engine state, outside the document: it survives
tab switches, which allows copying in one document and pasting in another.

## 0.9.0 feature paths

Implementation validation is tracked in `PROJECT_STATUS.md`; this section describes the design, not a test result.

- **Recovery**: `recovery.rs` stores atomic PDF snapshots and JSON metadata in a per-process session under
  `%LOCALAPPDATA%\SuniPDF\recovery`. An exclusive Windows lock excludes live sessions from the recovery list.
  The worker snapshots modified documents every 60 seconds and on suspension; source bytes are not overwritten.
  Protected snapshots remain encrypted, metadata contains no password, and restoration requires Save As.
- **Printing**: desktop `printing.rs` obtains a native Windows printer DC and validated page selection.
  The engine thread renders the current document into bounded raster bands, reports page progress and checks
  an atomic cancellation flag. Printer handles have ownership/cleanup; aborting does not save the document.
- **Reading and spreads**: core `annot.rs` resolves display points into PDFium character indices and returns
  page-specific text/highlight rectangles. Requests and responses carry document epoch and selection generation;
  the UI keeps one request pending and the latest pointer endpoints. `view.rs::Layout` groups spread rows,
  cover-first Book and odd last pages; all modes reuse the 24-job planner, 192 MiB texture pool and 256 MiB engine cache.
- **Field creation**: core `forms.rs::add_form_field` validates names and display rectangles before writing
  AcroForm/Widget objects and generating appearances. Desktop `form_builder.rs` supplies a numeric dialog;
  `EditOp` makes creation undoable. CLI `add-form-field` uses the same method.
- **OCR**: core `ocr.rs` invokes only the bundled or explicitly selected Tesseract runtime, in a private
  temporary directory, with `eng`/`fra` fast models, bounded image/TSV output, timeout and cancellation.
  Recognition occurs on a scratch document; success commits once and the worker creates one undo step.
  Existing text is skipped. Invisible word objects use the common rotation/CropBox geometry.
- **Certificate signatures**: `certificate.rs` parses saved bytes and creates a CMS detached signature in an
  incremental revision; `certificate_native.rs` delegates private-key operations to Windows Cryptography.
  PFX/P12 credentials use cleared temporary buffers and no persistent key import. Verification reports integrity
  and certificate metadata without establishing trust, revocation or a trusted timestamp. UI `certificate.rs`
  and CLI `feature_cmd.rs` both require a new copy for signing; encrypted input is refused.

## 0.13.0 visual signature paths (in development)

`Engine::typed_signature` creates a disposable PDF, reuses `Document::add_text` and its existing font/glyph
checks, then renders a bounded transparent PNG through PDFium. Desktop `RenderSignature` runs on the engine
thread; a request ID prevents an old preview from replacing changed text, font, ink or dialog state. Preview
does not mutate the open document. The resulting image uses the existing optional PNG library and placement flow.

`signature_image::placement_rect` supplies both the placement ghost and fitted page rectangles. Core
`Document::place_signature_image` validates and deduplicates targets, edits a scratch copy using
`add_image_data`, checks cancellation between pages and before commit, then swaps the document only on success.
The desktop worker records one undo snapshot for the committed batch; failures leave the document and journal
unchanged. Selected Organize pages are captured when opening the dialog; structural page changes discard the
dialog or captured targets. CLI `add-image --pages` uses the same batch method, while existing `--page` keeps
its original placement behaviour. Validation is tracked in `PROJECT_STATUS.md` (D102–D103).

## Coordinates (SUNI-031c)

Every coordinate crossing the engine ↔ interface/CLI boundary is in **display space**: points,
origin at the bottom-left corner of the page as displayed (after `/Rotate`, MediaBox ∩ CropBox box), Y pointing
up; the space of `Document::page_size` and of rendering. `sunipdf-core::geometry::PageGeometry` converts
to PDF user space (objects, text, annotations); it is the only place that knows about `/Rotate` and
the boxes. The UI only converts screen ↔ display (`annotate::to_pdf`). The tests
`geometry_matches_rendered_pixels_on_rotated_and_cropped_pages` and following verify the convention to the pixel.

## Command registry (SUNI-010, shipped)

Undoable changes run on the engine worker. Ordinary edits use `worker.rs::State::edit`; atomic task paths,
including OCR and multi-page signatures, add one pre-operation snapshot only after a successful commit.
Snapshots use `Document::snapshot` (PDF bytes); `Undo`/`Redo` reload the snapshot
(`Document::restore`). Bounded journal (50 entries, 512 MB). Simple, generic, memory-bounded;
a journal of named commands will come if memory becomes a constraint.

`crates/sunipdf-core/src/command.rs`: `Command` (enum), `REGISTRY` (constant table: stable identifier,
i18n key of the label, i18n key of the shortcut, `needs_document`, menu). Pure data, no UI dependency.
- The application builds its File / View / Help menus from the registry and routes menus **and**
  keyboard shortcuts to a single entry point `SuniApp::execute(ctx, Command)`.
- The CLI exposes `sunipdf-cli commands` (JSON). The command palette, configurable shortcuts,
  undo/redo and automation will rely on these same identifiers.
- Tests: uniqueness of identifiers, `from_id` round trip, existence of the i18n keys in both catalogues.

## Infrastructure choices

| Need | Choice | Reason |
|---|---|---|
| Rendering / parsing | PDFium 7881 via `pdfium-render` 0.9.4, DLL loaded at runtime | Chrome fidelity, ARM64 available, no C++ compilation |
| UI | `eframe`/`egui` 0.36 | Native Rust, immediate mode, DPI handled, AccessKit accessibility built in |
| Preferences | JSON in `%APPDATA%\SuniPDF` | Zero dependencies; SQLite only if large metadata appears |
| Errors | `thiserror` | Explicit types, stable codes |
| Logging | `log` + `env_logger` | Sufficient; `tracing` if spans become necessary |
| Tests | `cargo test`, fixtures generated by `scripts/make_fixtures.py` | Reproducible without an external Python dependency |

## Security

- Path validation by the OS; no execution of PDF content; render size limits.
- Documents never leave the machine.
- Hostile files: PDFium is continuously fuzzed by Chromium; SuniPDF will add its own corpus (P6).
