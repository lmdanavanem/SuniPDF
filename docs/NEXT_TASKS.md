# Next tasks (recommended order)

[Français](NEXT_TASKS.fr.md)

## Version 0.9.0: seven requested features published

The seven requested features are included in the [0.9.0 release](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.0)
under `lmdanavanem`. [PROJECT_STATUS.md](PROJECT_STATUS.md) records executed checks, release evidence and remaining
hardware/interoperability limits. Historical test counts are not the current release gate.

| Feature | Included scope |
|---|---|
| Autosave and recovery | 60-second/tab-suspend snapshots, original unchanged, protected snapshots encrypted, no passwords, restore through Save As |
| Windows printing | Native ranges/copies/orientation, current edits, bounded raster output, progress/cancellation |
| Richer selection | PDFium logical character ranges across pages, reverse drag, double-click words and edge scrolling |
| Two-page/Book view | Mixed sizes, cover first, odd last page, shared zoom and bounded tile/texture caches |
| Form creation | Numeric text/checkbox rectangles, unique names, rotated/cropped/encrypted documents, undo/fill/save |
| Local FR/EN OCR | Bundled Tesseract 5.5.3/Leptonica 1.87, fast eng/fra models, invisible text, existing text skipped, atomic cancellation and one undo |
| Digital signatures | Saved-byte integrity, incremental PFX/P12 signed copy, rewrite warning; chain trust/revocation/trusted timestamps/LTV not implemented |

The 0.8.3 P1/P2 corrections remain included: finite text coordinates, bounded Organize thumbnails,
single-line stamps and valid CLI annotation opacity. English remains the default; French is available.

## Version 0.9.1: reliability release published

[SuniPDF 0.9.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) was published as Latest under `lmdanavanem`.

- Preserve PDF/A identifiers and unrelated XMP when editing standard properties; clearly refuse malformed XML/DTD.
  This does not certify PDF/A conformance.
- Emit the explicit AES-256 key length before saving, including metadata/cleanup/optimisation round trips.
- Pinned qpdf checks of synthetic fixtures/generated output in CI; a real or hostile PDF corpus remains separate work.
- Seven real portable upgrade/failure/rollback scenarios to 0.9.1 and real MSI installation/upgrade/transactional rollback,
  downgrade rejection and uninstall passed on disposable Windows runners; documents/settings remain preserved.
- .NET SHA256 hashing works when Windows PowerShell 5 cannot load Get-FileHash; restart failures are reported.
- Dedicated Windows icon derived from the retouched author-provided source.

Workspace (322 tests), qpdf (22 PDFs/three malformed inputs) and final Windows CI passed; all six public files match
the verified packages and are available in `dist/`.
Current package/publication evidence is recorded in `PROJECT_STATUS.md`.
Do not infer ARM64, physical printer, reader interoperability or interactive updater validation from these checks.

## Version 0.10.0: requested P1/P2 lot published and verified

The requested six-area lot is implemented: atomic-save failures/public corpus, image compression, view rotation/full
screen/history, new form types and interchange/grouped keyboard controls, mouse crop/group transformations, and
sequential compression/OCR batches. Exact limits and executed results belong in `PROJECT_STATUS.md`.
Local workspace: 362 passed, zero failed, seven default ignored; formatting, Clippy and 853 matching bilingual keys
passed. Explicit qpdf checks passed for 31 synthetic/generated PDFs; four licensed NIST/NBS publications (744 pages)
passed text/render/extract/optimize/compression/source-preservation checks. Upstream source warnings are pinned;
rewritten outputs must pass without warnings. The shared writer fixes stale storage/xref bounds/revision offsets.
[Final main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848137022), the same-source ARM64 build,
six staged packages/checksums, all eight real portable update scenarios and both real MSI baselines (0.8.0 and 0.9.1)
passed. Package checks include English metadata, embedded icons, real compression, choice forms/XFDF, grouped edits,
French OCR and matching MSI/ZIP payloads. [SuniPDF 0.10.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0)
was published under `lmdanavanem`; all six unauthenticated public downloads match API/sidecar/staged hashes
and are available in `dist/`. The enhanced checks passed again on public binaries, and the production live updater
verified the 19,651,167-byte Latest x64 ZIP without installation. The requested P1/P2 release gates are complete;
the independent hardware/interoperability work below remains open.
Build provenance and the immutable `f7510afd314846079bbdd838525cd28372fd3100` release source are recorded in status.

## 0.11.0: navigation, forms and protected copies (published)

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) under `lmdanavanem`; Latest at the time.

The expanded 0.11.0 source passed 390 workspace tests (zero failed, seven default ignored), formatting/Clippy with
`-D warnings`, 858 matching EN/FR keys, 44 qpdf checks across 42 distinct PDFs and four real NIST documents
(744 pages; 91.59 seconds, peak child memory 329,347,072 bytes). [Final Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) and
[same-source ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) passed, as did nine real portable scenarios from
0.8.2/0.8.3/0.9.0/0.9.1/0.10.0, three MSI baselines (0.8.0/0.9.1/0.10.0) and all six staged asset checks. Packaged
checks include owner-opened AES-128/256 copies, decoded content/qpdf, page labels, metadata, OCR, forms and compression.
Immutable release source: `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`. All six unauthenticated public downloads match API/sidecar/staged hashes; enhanced checks passed again on public
binaries. The production live updater verified the 19,687,375-byte Latest x64 ZIP without installation. Evidence
and remaining limits are recorded in `PROJECT_STATUS.md`; published 0.10.0 source/packages remain unchanged.

- Read PDF page labels through PDFium in the toolbar, thumbnails and CLI `info`. Go to accepts an exact unique
  label; `#N` always selects the physical page. Labels remain read-only; invalid labels fall back to physical numbering.
- Resolve named destinations and `/XYZ`, `/Fit` and nonzero `/FitH` positions for links and bookmarks, including
  crop/rotation transforms, retained null axes and zoom. History includes same-page destinations and zoom changes.
  `/FitH` zero/null and other fit modes retain page-only navigation; PDFium cannot distinguish FitH null from zero.
- Preserve form drafts and pending actions when reading fields fails, show the failure and require an explicit retry.
- Fill existing editable drop-down fields with custom text in the panel and on the page; preserve named choices,
  Apply/Discard, Save and undo. Creating the editable flag was outside the 0.11.0 slice.
- Preserve original encryption, passwords and permissions when extracting/splitting protected PDFs; refuse an output
  that resolves to the source path. CLI keeps `encryption_removed: false` and adds `encrypted`; the existing
  document-permission policy is unchanged. Local, CI, package and upgrade regression gates passed; evidence is in `PROJECT_STATUS.md`.

## 0.12.0: form authoring and radio export values (published as Latest)

- Create editable drop-downs through the desktop checkbox or CLI `add-form-field --kind combo --editable`;
  retain required predefined choices, unique names, existing geometry/protection checks and undo.
- Map radio `/Opt` export values independently of widget appearance states, including JSON/XFDF/FDF interchange.
  Add `widgets[].export_value` to CLI information without changing `on_state`; fields with malformed or duplicate
  export mappings stay read-only. Radios without `/Opt` retain state-name values.
- [Published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (release
  `414674886`), source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation passed: 398 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 859 EN/FR keys, 48 qpdf checks and four NIST/NBS documents (744 pages;
  59.68 seconds). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) and the
  [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) passed,
  plus ten portable scenarios, four MSI baselines (0.8.0/0.9.1/0.10.0/0.11.0), all six staged/public files and
  enhanced binary checks. The production updater verified the 19,699,323-byte Latest x64 ZIP without installation.
  Verified files are in `dist/`; evidence and limits are in `PROJECT_STATUS.md` (D100–D101).

## 0.13.0: typed visual signatures and page batches (in development)

- Local explicit preview of a typed name/initials, using the existing standard/system TrueType font picker,
  glyph/embedding checks and transparent PNG/library flow (D102). Times italic is the portable default;
  input is one line of at most 256 characters and preview sides are capped at 1600 pixels.
- Current/all/Organize-selected targets, captured before leaving the grid; one common display-space centre,
  aspect-preserving fitting, atomic cancellation/rollback and one undo for a multi-page placement (D103).
  CLI `add-image --pages` adds batch placement while preserving existing `--page` behaviour/JSON.
- Implementation validation is ongoing. No 0.13.0 publication is claimed; 0.12.0 remains Latest until its release
  gates pass. Executed evidence belongs in `PROJECT_STATUS.md`.

## Remaining work beyond this lot

1. Independent execution on ARM64, physical printers, Windows Narrator, interactive installed/portable update and save
   dialogs, and Acrobat/Foxit/Chrome. These require the actual hardware or readers and are not covered by scripted CI.
2. Expand the four public NIST fixtures with licensed customer/hostile/large scans and complex forms; add fuzzing and
   hardware-recorded performance measurements. Preserve the original on every failed or cancelled operation.
3. Font subsetting/CFF/OTF/TTC, paragraph editing and editable pasted objects.
4. Local Office conversion and additional batch conversion operations, with measured output fidelity.
5. Certificate-chain trust/revocation, trusted timestamps/LTV and executable/MSI signing when an identity is supplied.
6. Page-label authoring and semantic rebasing after page changes, complex untagged-column reading order,
   touch validation and optional integrations.

No future version/date is committed by this remaining list.

## To be done by the user (outside the automated scope)
1. Validate the interface on real PDFs (home, tools, right rail, Ctrl+K palette, dark theme, FR/EN) and report differences.
2. Run the ARM64 ZIP on real ARM64 hardware (not done yet) and check OpenGL rendering.
3. Check annotation and redaction interoperability in Acrobat, Foxit and Chrome.
4. Validate interactive installed-MSI and portable update dialogs, including cancellation and unsaved documents;
   scripted installation/upgrade/rollback scenarios passed in CI.
5. Check Windows Narrator and keyboard navigation in the real interface.

## Tool hub: contract for the next slices (tool-hub delivered, D021)
The `ToolTab` tabs and the `home.card.*` cards no longer exist: the plans in `docs/plans/` that cite them
(OCR, redaction, export, compression, forms, signatures…) go through `apps/sunipdf-desktop/src/workspace.rs`.
To make a tool visible, a slice does exactly this:
1. add its `Command`s (engine + `REGISTRY`, stable id, correct `needs_document`);
2. in `workspace::TABLE`, set `entry` to `Some(Entry::Direct(cmd))` or `Some(Entry::Mode(Command::HubXxx))`,
   fill in `groups` (each command in **one single** tool), choose the icon;
3. if the tool has a bar: add `HubXxx` to the registry (`Tools` menu), to the `Hub*` arm of `execute` and to
   `shell::command_icon`;
4. the names, descriptions and colours of the 14 tools already exist (`hub.*.name`, `hub.*.desc`).
The `every_registry_command_has_a_home` test fails if a command has neither a tool nor a place in the system list;
`bar_icons_are_distinct` fails if an icon repeats within a bar or reuses that of a quick access button.
All 14 tools have an implementation in 0.9.0, including Scan & OCR, Prepare a form and Use a certificate. "Fill & Sign" is complete: "sign" half (sign-image slice) and "fill" half
(forms slice, Fill form group / panel / Clear / Flatten / Export / Import).

## After redaction (irreversible redaction, slice delivered)
Remaining, not blocking (see `KNOWN_ISSUES.md` A40, decisions D077–D082):
- independent checks of a real corpus (scans with an OCR layer, heavily tagged PDFs, nested
  form XObjects, Type 3 fonts); measurement on a page with tens of thousands of objects (quadratic
  `prune_objects`: own reachability with a `BTreeSet` if slow);
- fine text splitting (keep the unredacted part of a line selectable): requires recreating fragments with the
  original font, out of reach of `pdfium-render` 0.9.4;
- fill colour and overlay text ("REDACTED"), persistent `/Redact` marks (no creator in `pdfium-render`);
- cleaning of bookmarks, attachments, JavaScript and named destinations (separate "Sanitise" tool);
- removal of a font subset that became unused; "before / after" page preview;
- `docs/SECURITY_MODEL.md` (A/B/C method, guarantees, assumptions); screenshot S3 (after applying) not replayed.

## After forms (AcroForm forms, slice delivered)
Remaining, non-blocking (see `KNOWN_ISSUES.md` A38, decisions D061–D066):
- JSON/XFDF/FDF interchange and grouped panel Apply are implemented for 0.10.0; validation is tracked in `PROJECT_STATUS.md`.
- Editable drop-down filling shipped in 0.11.0; new editable fields and radio `/Opt` mapping shipped in 0.12.0.
  Keyboard activation shipped in 0.10.0; Windows Narrator remains unchecked
  (`labelled_by` names set, not listened to).
- flattening: also keep the annotations without an appearance other than links (attachments without
  `/AP`, screens) if a real case calls for it; today only `/Link` entries are put back;

## After sign-image (visual signature, slice delivered)
Remaining, non-blocking (see `KNOWN_ISSUES.md` A35–A37, decisions D054–D060):
- Typed signatures and all/selected-page placement are implemented for 0.13.0; validation is ongoing (D102–D103).
- DPAPI encryption of the saved signatures (ADR to write, D055);
- stroke smoothing (Chaikin) if fast curves look angular; adaptive threshold for photos with a
  cast shadow; pen pressure if egui ever exposes it.

Rest of the hub (non-blocking): keyboard focus placed on the active row when the panel opens (F9);
final visual validation by the user. The short-window right-rail overlap is already fixed (A15).

## After review-fixes (findings of the 14/10 adversarial reviews, slice delivered)
The 39 findings of `docs/plans/review-fixes.md` are fixed or already covered (engine and application tests).
Remaining, non-blocking:
- expand the independent AES/XMP interoperability corpus beyond the synthetic qpdf gate (A4);
- validate preserved PDF/A metadata with a dedicated conformance validator (A34); the 0.9.1 preservation fix
  itself is implemented, and current execution evidence belongs in `PROJECT_STATUS.md`;
- embed JPEGs with EXIF orientation 5 and 7 without decoding (A31); limit the memory of PNG batches;
- CLI help for `stamp` / `page-numbers` (A16, clap replaces `{n}`).

## SUNI-012c — Display, continued (owner: UI-UX)
Done: tabs (SUNI-012a); virtualized continuous view, tiles, anchored zoom, single page as an option (SUNI-012b,
`docs/plans/continuous-view.md`, D038–D045).
Remaining (see `KNOWN_ISSUES.md` A25–A28):
- view-only rotation/full-screen/history are implemented in 0.10.0; current validation is in `PROJECT_STATUS.md`;
- animated scrolling for "go to"; touch pinch to check on a touch screen;
- if a heavy page freezes scrolling: `Document::render_regions` (one page loaded for 3 to 4 neighbouring
  tiles), to be added only after measurement; adjustable memory caps if a need arises.

## SUNI-013b — Text selection and copy (owner: UI-UX)
Done: rectangular selection, copy (Ctrl+C fixed in SUNI-031c through egui's Copy event).
Implemented for 0.9.0: PDFium logical ranges, words, cross-page selection and edge scrolling. Remaining: inference for complex untagged columns.

## SUNI-031d — Content editing, continued (owner: PDF-EDITOR)
Done in SUNI-031/031b/032/031c: single and multiple selection (rectangle, Shift/Ctrl+click, Ctrl+A), group
move, resizing, free rotation, duplication, Z order (one step or extreme), deletion, colours,
stroke width, opacity, copy/cut/paste between pages and tabs (group), Objects panel, text on the page
(standard and system TrueType fonts), image replacement, blank pages, cropping, page resizing,
rotated and cropped pages.
Remaining (see `KNOWN_ISSUES.md` A7, A8, A18–A22):
- subsetting of embedded fonts (`subsetter` crate MIT/Apache + lopdf post-processing, ADR to write);
  `.otf` fonts (CFF outlines) and `.ttc` collections;
- "native" editable paste (objects recreated one by one instead of an XObject); opacity and colour of groups;
- remembered pivot for asymmetric paths (group rotation/resizing implemented in 0.10.0);
- drag and drop in the Objects panel (mouse crop preview implemented in 0.10.0);
- reading the opacity of an existing image (PDFium workaround: read the stream's ExtGState with lopdf).
Acceptance: each operation covered by an engine test with verified rendering.

## SUNI-021b — Advanced annotations: delivered, what remains (owner: PDF-EDITOR)
Delivered (D067–D076): ink, free text box, rectangle, ellipse, line, arrow, clickable links, select / move / resize /
recolour / delete, reply threads and statuses, remembered author, CLI `annotations` / `annotate` / `reply` /
`review-state` / `annotation-delete`. What remains:
- **verify interoperability** of replies and statuses, ink, free text and shape appearances in Acrobat, Foxit and
  Chrome (not available on the development machine); structural checking alone does not prove visual interoperability;
- grouping of ink strokes (one annotation for N strokes, committed on idle) with guaranteed write at save and close
  time; eraser;
- polygon, polyline, cloud, stamp, attachment, dashed border, rich free text (bold, italic, CJK: embedded font);
- change the target of an existing link; named destinations and `/XYZ` reading are implemented in the 0.11.0 lot above;
- lighter round trip for big files (write the annotation without reloading the whole document);
- free text placed directly on the page (in-place editor, like the Text tool) instead of a dialog.

## SUNI-025 — Lossy compression and qpdf verification (owner: OCR-CONVERSION + QA)
Delivered in 0.10.0: bounded image-stream resampling/JPEG re-encoding, four pinned public NIST documents and
interrupted/full-disk/failed-save regressions. Pinned synthetic qpdf checks and the AES-256 key-length correction
remain included. Expand coverage to licensed customer/hostile/large documents; execution evidence is in `PROJECT_STATUS.md`.

## SUNI-015 — Visual identity and Windows resources (owner: WINDOWS-RELEASE)
Delivered: DPI-aware manifest and version resources. The 0.9.1 Windows icon is derived from the dedicated,
retouched author-provided `assets/icon/sunipdf-source.png` (ICO + PNG sizes).
Remaining: executable/MSI code signing when a certificate is available.

## SUNI-016 — Independent QA (owner: QA-PERFORMANCE)
Corpus of free PDFs (pdf.js test corpus, licences checked), hostile file tests (no pages, broken xref,
Unicode paths, simulated full disk), first benchmark (opening + page 1 rendering) with hardware noted.

## Known technical debt
- Re-enable wgpu/DX12 (D010); check OpenGL on ARM64.
- `-j 2` imposed by the machine's memory; document the reference machine.
- CLI `--lang` option (D009) if human-readable messages become necessary.
