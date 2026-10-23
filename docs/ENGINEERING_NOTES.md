# Engineering Notes (read before modifying the code)

This document gathers what you need to know to ship a vertical slice without breaking the repository:
commands, guard rails, code patterns and pitfalls already encountered. It complements `CONTRIBUTING.md` and
`docs/ARCHITECTURE.md`.

## 1. Environment and commands

- Reference machine: Windows 11, 14 GB of RAM. **Always `-j 2`** with cargo, **never two cargo processes in
  parallel**, never `cargo clean` (a full rebuild costs more than ten minutes).
- `cargo` is not in the PATH by default:
  - Git Bash: `export PATH="$HOME/.cargo/bin:$PATH"`
  - PowerShell: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`
- Windows PowerShell 5.1 turns cargo's error output into an exception: run the packaging scripts
  from Git Bash (`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/x.ps1`).
- PDFium: `third_party/pdfium/win-x64/bin/pdfium.dll` (ignored by git, `scripts/fetch-pdfium.ps1`).

### Mandatory guard rails before any commit (all four must be green)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-i18n.ps1
```

`cargo fmt --all` reformats: re-read a file (or `grep -n`) before editing it after a formatting pass.

## 2. Writing code without trapping yourself

- Modify files with the editing tools, not with shell heredocs: heredocs eat
  `\n`, `\u{...}` and apostrophes. For a modification script, write it to a file and then
  run it with `python -I file.py`.
- No hard-coded user-visible string in `apps/`: `tr(lang, "key")` or
  `tr_args(lang, "key", &[("name", value)])`. Every key goes into `assets/i18n/fr-FR.json` **and**
  `assets/i18n/en-US.json` (same keys, JSON indented with 2 spaces, UTF-8 without BOM).
- The CLI speaks English and outputs JSON (`serde_json::json!`), errors via `EngineError`.
- The UI never imports `pdfium-render`: everything goes through `sunipdf-core` and the engine thread.

## 3. Patterns of this code base

### Adding an undoable edit operation
1. Engine: `impl Document<'_>` method in the relevant module of `crates/sunipdf-core/src/`
   (`edit.rs`, `annot.rs`, `stamp.rs`, `pages.rs`, etc.), `EngineError` errors, export in `lib.rs`.
2. Engine thread (`apps/sunipdf-desktop/src/worker.rs`): variant in `EditOp`, arm in
   `fn apply(doc, op)`. `State::edit` takes a byte snapshot beforehand, pushes it onto the undo stack and
   returns `Response::DocumentChanged`; on error the snapshot is restored.
3. Read-only: `Request` + `Response` variant (see `ObjectAt` / `ObjectSelected`).
4. Long file task (output to another file): `BatchOp` + `BatchResult`
   (see `tools.rs`, `on_batch_done`).

### Adding a command
1. `crates/sunipdf-core/src/command.rs`: variant in `enum Command`, entry `cmd!(Variant, "menu.id",
   "label.key", Some("shortcut.key") | None, needs_a_document, Menu)`.
2. `apps/sunipdf-desktop/src/shell.rs`: arm in `command_icon` (glyph from `icons.rs` + text fallback).
   `apps/sunipdf-desktop/src/workspace.rs`: place the command in the group of **its tool** (`TABLE`,
   a single tool per command; colour and bar follow from it) or, for a system command, in
   `SYSTEM_COMMANDS` of the `every_registry_command_has_a_home` test. Full contract: `docs/ROADMAP.md`.
3. `apps/sunipdf-desktop/src/app.rs`: arm in `execute`, shortcut in `handle_input`
   (`key(Modifiers::…, Key::…, Command::…)`). Single-letter shortcuts are ignored when an input
   field or a dialog has the keyboard.
4. i18n keys of the label and shortcut in both catalogues.
A command that requires a document and is launched without one first opens a file and then
runs (`after_open`).

### Adding a dialog
Always `crate::dialog::show(ctx, dialog::Spec::new(id, title, dark, has_icons).subtitle(..).icon(..)
.width(..), |ui| body, |ui| footer)` with `dialog::primary`, `dialog::secondary`, `dialog::danger`
(the footer is right-aligned: the first button added is the primary action). No direct
`egui::Modal`. Give focus to the first field only once
(`if ui.memory(|m| m.focused().is_none()) { field.request_focus(); }`), otherwise the other fields
become uneditable. Add the "open" state to the guard at the top of `handle_input` to block
shortcuts during input.

### Per-tab state
Document-related interface state (page, zoom, selection, tools) is exchanged through `tabs.rs`
(`take_tab` / `put_tab`). Any new document-related state must be added there, otherwise it leaks from one
tab to another.

### Document view (`view.rs`)
A page on screen = `paint_slot` (screen rectangle `page_rect`, uniform zoom `effective_zoom`): every
per-page overlay (highlight, selection, handles, editor) takes `(painter, page_rect, page, page_h_pt)` and
filters on its page; it then works in continuous mode as well as in single-page mode. Never assume "displayed page =
`doc.page`": in continuous mode several pages are visible; `doc.page` is the current page (dominant or touched).
Scrolling = `self.view.request(ScrollRequest::…)` or `go_to_page` (never `scroll_offset` outside
`document_view`, egui would reapply it on every frame). After an edit, `DocState.pages_unchanged` decides whether
the textures stay displayed (stale): any new `EditOp` that changes the number, order or size of pages
must appear in `EditOp::keeps_pages`.

### Coordinates
The engine API speaks in **display space**: points, origin at the bottom left of the page as
displayed (after `/Rotate`, MediaBox ∩ CropBox box); the screen has its origin at the top left.
UI conversions: `annotate::to_pdf(pos, page_rect, zoom, page_h_pt)` and, in the other direction,
`page_rect.min + vec2(x * k, (page_h_pt - y) * k)` with `k = effective_zoom * SCREEN_PER_PDF_POINT`.
In the engine, every new function that touches objects, text or annotations converts with
`geometry::geometry_of(&page)` (`to_user`, `rect_to_user`, `vec_to_user`, `upright_matrix`, and the inverse on
output); test on `tests/fixtures/boxes.pdf` (pages rotated 90/180/270 and cropped).

## 4. Library pitfalls

### pdfium-render 0.9.4 / PDFium 7881
- PDFium is not thread-safe: a single engine thread. It does not reinitialise within the same
  process: integration tests share a single engine (`engine()` in `tests/engine.rs`),
  never `Engine::new()` directly in a test.
- A page object modified in place (text, matrix, colour) is only written after
  `page.regenerate_content()`; addition and removal regenerate on their own.
- `page.objects()` borrows the page; to keep an object while passing `&mut page` to another
  function, obtain it through `page.objects_mut().get(i)`.
- Drawing order follows the content streams: reordering objects only has an effect after merging the
  streams (`merge_content_streams`, lopdf).
- The bounds of a text object start at the baseline (descenders extend below).
- PDFium does not encrypt on write and does not modify bookmarks: these operations go through lopdf
  on the document bytes (`security.rs`), then `Document::restore`.

### egui / eframe 0.36
- `impl eframe::App { fn ui(&mut self, ui, frame) }`, panels `egui::Panel::top/left/right/bottom`,
  `Frame::new().corner_radius(..)`, `ComboBox::from_id_salt`, `StrokeKind`, `EllipseShape { angle, .. }`.
- Icons: `icons::icon(has_icons, glyph::X, "fallback", size)`; never concatenate an icon glyph
  into a text string (different font): use `Button::new((icon, label))` or paint
  with `painter.text(.., FontId::new(size, FontFamily::Name(icons::ICONS.into())), ..)`.
- The theme's "inactive" background is the surface colour: a `Slider` needs a local
  `ui.visuals_mut().widgets.inactive.bg_fill` for its rail to be visible.
- Keyboard: `ctx.input_mut(|i| i.consume_key(Modifiers::…, Key::…))` to prevent a key from being used
  twice in the same frame. `consume_key` ignores an extra Shift or Alt key (`matches_logically`):
  read the Shift variant **before** the plain variant (Shift+Space before Space).
- `ScrollArea::max_height(h)` does not go below 64 px (`min_scrolled_height`): for a shorter area,
  add `.min_scrolled_height(0.0)`, otherwise the neighbouring content covers it.
- Drag origin: `ui.input(|i| i.pointer.press_origin())`, not the position at the moment the
  drag threshold is crossed.
- Hand-drawn widget: `allocate_exact_size(.., Sense::click())` (focusable), then
  `resp.widget_info(..)` for a readable AccessKit label (never the icon glyph), painting after
  the allocation (hover is only known afterwards). After an activation, `resp.surrender_focus()`: a focused
  widget silences the single-letter shortcuts of `handle_input`.
- egui removes focus on Escape **before** the application reads the key: `SuniApp::focus_last_frame`
  keeps the state of the previous frame (Escape in a field does not leave the active tool).
- Escape goes through a single point: `workspace::take_escape` (order in `EscState::action`, tested). egui menus
  and lists (`menu_button`, `ComboBox`, colour picker) only read Escape at the moment they are
  drawn: `handle_input`, which runs before, would consume the key. `egui::Popup::is_any_open(ctx)` read at the
  start of a frame gives the state of the previous frame; `take_escape` then closes the menu itself
  (`Popup::close_all`) and consumes the key. Any new action on Escape is added to `EscState`.
- `Visuals::interact_cursor` only applies to `egui::Button`; home-made widgets set
  `on_hover_cursor(PointingHand)`. `ui.set_max_width(w)` can **widen**: bound it by `available_width()`.
- A `ui.horizontal` row starts at `interact_size.y` high; to centre taller buttons
  on a neighbouring chip, set `interact_size.y` to their height.
- Headless test: `Context::run_ui` + `enable_accesskit()`; clear `out.textures_delta` before
  discarding the output (epaint debug assertion).
- Ctrl+C / X / V **never** arrive as `Event::Key`: egui-winit turns them into `Event::Copy`, `Cut`,
  `Paste(text)` (Paste only if the system clipboard contains text). Read them from `i.events`.
- A popup window (`ComboBox`, `Area`) keeps the size of the previous frame: content that grows
  afterwards (list loaded lazily) is truncated. Give a fixed height to the variable part
  (`ScrollArea::max_height(..).auto_shrink([false, false])`).
- A `ui.horizontal` nested inside `horizontal_wrapped` does not wrap: it overflows. Prefer simple
  widgets, or one row per `horizontal_wrapped`.
- Inside an `Area` / `Modal`, the "available" space is the remembered size of the previous frame: a
  `with_layout(right_to_left(Align::Center))` that centres its buttons in that space makes the dialog grow
  on every frame (150 px footer, header off screen). Give a fixed height (`allocate_ui_with_layout`) and,
  for a bounded `ScrollArea`, also set `min_scrolled_height` (otherwise it only grows by a few
  pixels per frame and stops with the last repaint). `dialog::show` applies both, and reserves
  10 px for the theme's floating bar when the body overflows (`scroll.floating_allocated_width`, with
  `set_width(available_width())` inside the area): without that the bar covers the right edge of the content.
- A `Frame` nested in a `ui.horizontal` row that contains `ui.set_width(ui.available_width())` makes a panel grow
  from one frame to the next up to the window width: indent a card with `Frame::outer_margin`, not with
  `horizontal` + `add_space`. A `TextEdit` in a panel is bounded by `available_width()`.
- `consume_key(COMMAND, k)` also accepts Ctrl+Shift+k: the `FIELD_SHORTCUTS` / `SHORTCUTS` tables of `app.rs`
  read the Shift variant first (tested invariant); every new shortcut goes into these tables.

### Annotations (lopdf writes, PDFium renders)
- An advanced annotation is written by `annot_write.rs`: `edit_pdf` (snapshot → `security::load` → lopdf →
  `security::save` → `restore`) then an explicit `/AP` stream; never through `pdfium-render`'s annotation setters
  (no circle or line, colour fallback that casts an annotation handle to a page object). Appearances are drawn in
  **local display coordinates** with `/Matrix` = linear part of the page rotation: upright text on a `/Rotate` page.
- Annotation indices are those of `/Annots` (= `FPDFPage_GetAnnot(i)`): never `pdfium-render`'s `iter()` (stops at
  the first unreadable entry), never `page.links()` (duplicates), never `lopdf::get_page_annotations` (skips
  invalid entries). Only `/NM SuniPDF-…` annotations have their appearance regenerated.
- An external address only opens after `is_safe_external_uri` **and** the confirmation dialog (`link_action`).

## 5. Visual verification

`scripts/dev-screenshot.ps1 -Out capture.png [-Pdf "a.pdf;b.pdf"] [-Commands "id1,id2"]
[-Drag "x1,y1,x2,y2;x1,y1,x2,y2"] [-Keys "^k"] [-KeysAfter "{F2}|^c|^v"] [-Delay 5]` launches `target\debug\sunipdf.exe`,
runs the commands at startup (`SUNIPDF_STARTUP_COMMANDS`), simulates drags (coordinates
relative to the window, page displayed from x≈440, y≈140 with the tools panel open, y≈190 when
a tool's bar is displayed; a zero-height drag does not select text), `-Width`/`-Height`
force the window size, `-Wheel "x,y,notches[,ctrl]"` turns the mouse wheel (Ctrl + wheel = zoom
anchored under the pointer), captures
the window and then closes the application. Since the continuous view, pages are stacked and the first page is
centred horizontally: locate the text on a capture without `-Drag` before replaying a drag. One launch at a time: the script takes the foreground and
moves the mouse. Log: `$env:SUNIPDF_LOG = "file.log"` and `$env:RUST_LOG = "debug"`.
Drags: the `-Drag` / `-Wheel` coordinates are window coordinates in **capture pixels divided by 1.5** (Windows at
150 %); a tool bar shifts the page by 72 px; the **first press** starts from the stale position of the window centre
(do a warm-up drag on the grey background); a **click** is a zero-length drag (`x,y,x,y`); an open dialog swallows the
following drags; a 335 × 39 px capture is a window minimised by a stray click (run again). The `log::debug!` lines of
`annot_adv` give the PDF coordinates of a gesture start, to calibrate. Preferences are in `%APPDATA%\SuniPDF\settings.json`: restore the file if a test modifies it.
Visual signature: `SUNIPDF_SIGNATURES_DIR` replaces `%APPDATA%\SuniPDF\signatures` (do not pollute the real
profile during a capture) and `SUNIPDF_PICK_FILE` gives the path that "Choose an image…" returns instead
of the native picker, which cannot be driven. A dialog whose height depends on its content (library
cards) shifts its controls: note the coordinates on a capture A before replaying a drag.

## 6. Definition of "done" for a slice

Real end-to-end code (engine, engine thread, interface, CLI if relevant), command reachable
from the tools panel and the palette, fr and en i18n keys, integration test that verifies the result
(text re-read, rendered pixels, file reopened), error cases handled, screenshot looked at,
documentation up to date (`PROJECT_STATUS.md`, `FEATURE_MATRIX.md`, `CHANGELOG.md` "Unreleased" section,
fr and en user guides, `KNOWN_ISSUES.md` for the limits), the four guard rails green.
What is not done stays in the roadmap, with the reason.
