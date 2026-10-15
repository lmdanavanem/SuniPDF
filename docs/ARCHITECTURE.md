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
5. **Atomic writes.** Neighbouring temporary file then `rename` (`settings.rs::save_atomic`); the same rule
   applies to future PDF saves, with verification by an independent tool (qpdf) in CI.
6. **Offline, no telemetry.** No network dependency at runtime. The PDFium download script is a build
   tool, not an application feature.
7. **No PDF JavaScript.** PDFium binaries compiled without V8 or XFA (`args.gn`: `pdf_enable_v8=false`).

## Current workspace

```
SuniPDF/
├── apps/
│   ├── sunipdf-desktop/      `sunipdf` binary (egui/eframe) — main.rs, app.rs (state + document), shell.rs (chrome: tabs,
│   │                         toolbar, "All tools" panel, home, rail, palette), workspace.rs (hub: tool table
│   │                         → registry commands, active tool, contextual bar), worker.rs, settings.rs, theme.rs, icons.rs,
│   │                         editor.rs (selection, handles, rotation, object clipboard, object bar), sign.rs
│   │                         (visual signature: pad, import, local library, placement, date), inline_text.rs
│   │                         (text on the page, font picker), objects.rs (Objects panel), pagesetup.rs (crop,
│   │                         resize, blank page), annot_adv.rs (ink, free text, shapes, links, annotation selection, reply threads), view.rs (continuous / single-page view: virtualised layout,
│   │                         wanted tiles, texture pool, anchored zoom)
│   └── sunipdf-cli/          `sunipdf-cli` binary (clap) — JSON output, stable error codes
├── crates/
│   ├── sunipdf-core/         Engine, Document, RenderedPage, EngineError; tiles.rs (tile geometry), cache.rs (LRU
│   │                         pages + tiles); edit.rs (objects), geometry.rs (display space), annot_write.rs / appearance.rs / links.rs
│   │                         (advanced annotations: lopdf writes with an explicit `/AP`, PDFium renders and reads back),
│   │                         clipboard.rs (copy/paste of objects), fonts.rs (system fonts), boxes.rs (page boxes),
│   │                         signature_image.rs (visual signature: rasterised strokes, background removal, cropping, PNG; no PDFium);
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
| `sunipdf-forms` | not created (D061): AcroForm lives in `sunipdf-core::{forms, form_data}` (lopdf model and writing, PDFium appearances, JSON); FDF/XFDF not shipped | — |
| `sunipdf-convert` | PDF↔images, text, Office via a detected external converter | P4 |
| `sunipdf-ocr` | Tesseract (fra, eng) offline | P4 |
| `sunipdf-security` | encryption (outside PDFium), permissions, redaction, metadata | P5 |
| `sunipdf-signatures` | CMS, PKCS#12, timestamping, verification | P5 |
| `sunipdf-automation` | JSON-Schema command registry, batches, opt-in MCP | P5 |
| `sunipdf-platform` | Windows Explorer, file associations, printing, DPI | P5 |
| `sunipdf-ui` | reusable egui components (if the app grows) | P3 |

## Tabs (SUNI-012a)

The engine holds only one active document. A tab switch sends `Suspend` (the engine returns
the document bytes, its password and its undo history) then `Resume`/`ResumeDoc` (reload
from the bytes via `Engine::open_bytes`). The document's interface state (page, zoom, thumbnails, search,
selection, annotations, bookmarks, etc.) is exchanged through `tabs.rs::take_tab` / `put_tab`. Cost: one PDFium
serialisation per tab switch; memory: one snapshot per inactive tab. The object clipboard
(`ObjectClip`, a one-page PDF in memory) lives in the engine state, outside the document: it survives
tab switches, which allows copying in one document and pasting in another.

## Coordinates (SUNI-031c)

Every coordinate crossing the engine ↔ interface/CLI boundary is in **display space**: points,
origin at the bottom-left corner of the page as displayed (after `/Rotate`, MediaBox ∩ CropBox box), Y pointing
up; the space of `Document::page_size` and of rendering. `sunipdf-core::geometry::PageGeometry` converts
to PDF user space (objects, text, annotations); it is the only place that knows about `/Rotate` and
the boxes. The UI only converts screen ↔ display (`annotate::to_pdf`). The tests
`geometry_matches_rendered_pixels_on_rotated_and_cropped_pages` and following verify the convention to the pixel.

## Command registry (SUNI-010, shipped)

Undo: every page modification goes through `worker.rs::State::edit`, which takes a snapshot
(`Document::snapshot`, PDF bytes) before applying the operation; `Undo`/`Redo` reload the snapshot
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
