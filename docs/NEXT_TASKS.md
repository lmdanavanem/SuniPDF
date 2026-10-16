# Next tasks (recommended order)

[Français](NEXT_TASKS.fr.md)

## Current lot: 0.21.1 in preparation

Done implementation, checked on the earlier source locally and in GitHub CI/CodeQL: the P1 missing-glyph refusal/draft-recovery and
non-Link flatten-preservation fixes. The project now has 39 Todo, zero In Progress and three Done;
see [Project status](PROJECT_STATUS.md) for source and evidence. 0.21.1 release-source checks and publication
are pending; published 0.21.0 packages are unchanged.

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

## 0.12.0: form authoring and radio export values (published; Latest at the time)

- Create editable drop-downs through the desktop checkbox or CLI `add-form-field --kind combo --editable`;
  retain required predefined choices, unique names, existing geometry/protection checks and undo.
- Map radio `/Opt` export values independently of widget appearance states, including JSON/XFDF/FDF interchange.
  Add `widgets[].export_value` to CLI information without changing `on_state`; fields with malformed or duplicate
  export mappings stay read-only. Radios without `/Opt` retain state-name values.
- [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (Latest at the time; release
  `414674886`), source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation passed: 398 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 859 EN/FR keys, 48 qpdf checks and four NIST/NBS documents (744 pages;
  59.68 seconds). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) and the
  [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) passed,
  plus ten portable scenarios, four MSI baselines (0.8.0/0.9.1/0.10.0/0.11.0), all six staged/public files and
  enhanced binary checks. The production updater verified the 19,699,323-byte Latest x64 ZIP without installation.
  Verified files are in `dist/`; evidence and limits are in `PROJECT_STATUS.md` (D100–D101).

## 0.13.0: typed visual signatures and page batches (published; Latest at the time)

- Local explicit preview of a typed name/initials, using the existing standard/system TrueType font picker,
  glyph/embedding checks and transparent PNG/library flow (D102). Times italic is the portable default;
  input is one line of at most 256 characters and preview sides are capped at 1600 pixels.
- Current/all/Organize-selected targets, captured before leaving the grid; one common display-space centre,
  aspect-preserving fitting, atomic cancellation/rollback and one undo for a multi-page placement (D103).
  CLI `add-image --pages` adds batch placement while preserving existing `--page` behaviour/JSON.
- [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) under `lmdanavanem` (Latest at the time; release
  `414763678`), source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation passed: 410 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 870 EN/FR keys, 53 qpdf checks and four licensed NIST/NBS documents
  (744 pages; 76.40 seconds locally). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
  and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
  passed. Eleven portable scenarios (seven upgrades including 0.12.0 and four failure guards), five MSI baselines
  (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0), all six staged/public files and enhanced binary checks passed. Packaged
  signature batches cover plain/owner-opened AES-128/256 PDFs, both passwords, qpdf and source preservation.
  Verified copies are in `dist/`; the production updater verified the 19,730,578-byte Latest x64 ZIP without
  installation. Evidence and remaining limits are in `PROJECT_STATUS.md` (D102–D103).

## 0.14.0: page-label authoring and navigation retention (published; Latest at the time)

- Pages/Organize and CLI `set-page-labels`/`reset-page-labels` author an inclusive physical range with six styles,
  prefix/positive start, preserve outside rules and restore their continuation at the following boundary (D104).
  Prefixes are one line of at most 256 UTF-8 bytes; generated labels are bounded to 4096 bytes. Malformed existing
  trees refuse Apply before mutation; Reset removes the catalog entry without secure metadata erasure.
- Label changes affect navigation, not printed numbers. Ranges remain positional after structural edits;
  semantic rebasing is future work. Existing duplicate-label/physical `#N` navigation and protection remain.
- Carry geometry-preservation state in Undo/Redo snapshots so content/label changes retain navigation;
  structural operations still invalidate it (D105).
- [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) under `lmdanavanem` (Latest at the time; release
  `414812776`), source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation passed: 420 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 890 EN/FR keys, 63 qpdf checks and four licensed NIST/NBS documents
  (744 pages; 53.91 seconds locally). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
  and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
  passed. Twelve portable scenarios (eight upgrades including 0.13.0 and four failure guards), six MSI baselines
  (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0), six staged/public files and enhanced binary checks passed, including
  protected label authoring/reset, both passwords, outside labels, Unicode and unchanged text/page PNGs.
  Verified copies are in `dist/`; the production updater verified the 19,785,200-byte Latest x64 ZIP without
  installation. Executed evidence and remaining limits are in `PROJECT_STATUS.md` (D104–D105).

## 0.15.0: editing existing link targets (published milestone)

- Reuse the link dialog through Edit target/double-click, prefill resolved named/XYZ destinations and explicitly
  replace page views with whole-page Fit. Unsupported targets start as an empty address, never implicit page 1.
- Core/UI/CLI accept safe `http`/`https`/`mailto` or a physical page, replace all target/additional-action fields
  together and retain area/style/names/comments/replies, unrelated data and protection (D106).
- Respect direct/indirect ReadOnly/Locked/LockedContents flags; malformed flags refuse editing. One undo preserves
  geometry/navigation. Edits/tab switches discard stale dialogs. CLI `set-link-target` keeps existing output guards.
- [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) under `lmdanavanem` (Latest at the time), release
  `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Local gates passed: 429 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 894 EN/FR keys, 68 qpdf checks and four licensed documents (744 pages;
  48.891 seconds, peak child memory 328,736,768 bytes). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488)
  and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541)
  passed, along with thirteen portable scenarios (nine upgrades including 0.14.0 and four failure guards), seven
  MSI baselines (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0/0.14.0), six staged/public files and enhanced binary checks.
  Verified copies are in `dist/`; the production updater verified the 19,803,226-byte Latest x64 ZIP without
  installation. Executed evidence and remaining limits are in `PROJECT_STATUS.md` (D106).

## 0.16.0: dashed shape annotations (published milestone)

- Rectangle/ellipse/line/arrow only: UI Dashed border and CLI `annotate --dashed` use `[6,3]` points, while the
  actual custom `dash_pattern` survives selection/recolour/move/resize/undo. Only an explicit checkbox toggle
  replaces it. Solid defaults, continuous fills and solid arrowheads remain (D107); still 53 CLI commands.
- Match border dictionaries and explicit appearances at phase zero. Reject nonempty patterns on unsupported
  annotation kinds and refuse regenerating malformed/unsupported SuniPDF border styles rather than losing them.
  Pattern validation and conservative appearance-work bounds prevent excessive regeneration.
- Local checks passed on source `6cd7843734f3924e87033d8b05539d493ccbcb95`: 439 workspace tests (zero failed,
  seven default ignored), formatting/Clippy, 895 EN/FR keys, 73 qpdf checks and four licensed documents (744 pages;
  53.542 seconds, peak child memory 331,517,952 bytes). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537)
  and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589)
  passed. All fourteen portable cases (ten upgrades including 0.15.0 and four guards), eight MSI baselines
  (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0/0.14.0/0.15.0) and 40 MSI logs passed verification. CI corpus:
  47.628 seconds, peak child memory 321,306,624 bytes. [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0)
  under `lmdanavanem` (Latest at the time), release `414934190`, with six files. All staged/public files and enhanced checks passed;
  verified copies are in `dist/`. The production updater verified the 19,821,719-byte x64 ZIP without installing
  it on the host. Executed evidence and remaining limits are in `PROJECT_STATUS.md`.

## 0.17.0: image conversion reliability (published; Latest at the time)

- Embed JPEG EXIF orientations 5 and 7 directly without recompression, completing all eight orientations through
  the existing object matrix path. Keep non-JPEG quality/transparency and existing desktop/CLI callers (D108).
- Before raster allocation, bound the cumulative non-JPEG estimate to 512 MiB:
  `sum(max(decoder.total_bytes(), width × height × 4))`, with checked arithmetic and finite/positive physical
  page sizes. Repeat checks at loading for simple input changes. This is not a native/process RSS ceiling or
  compressed-JPEG limit; decoder, PDFium and output overhead remain separate.
- Conversion method/callers and 53 CLI commands remain unchanged; no new dependency. Budget excess/overflow
  uses `image_batch_too_large` with one localized EN/FR error key; invalid geometry retains `other`.
- Local gates passed: 445 workspace tests (zero failed, seven default ignored), formatting/Clippy all-targets,
  896 EN/FR keys, 74 qpdf checks, four licensed documents/744 pages (48.187 seconds; peak child memory
  330,264,576 bytes) and the update-helper self-test. Independent debug conversion checks passed: eight orientations,
  nine exact JPEG streams, 32 JPEG/four PNG corners and alpha mask, three qpdf checks, two budget/ten DPI refusals,
  thirteen unchanged sources (35 subprocesses, 1.063 seconds).
- Source `5785de282e126fb4666b28fb83340d0e9c6e9acf`: [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
  and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
  passed, along with fifteen portable cases, nine MSI baselines including 0.16.0 and 45 verified MSI logs.
  CI corpus: 45.791 seconds, peak child memory 321,413,120 bytes. [Published (Latest at the time)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0)
  under `lmdanavanem`, release `415074108`, with six files. All staged/public files and enhanced conversion checks
  passed, including exact JPEG streams, corners/transparency and budget refusals before raster decoding. Verified copies
  are in `dist/`; the production updater verified the 19,825,743-byte x64 ZIP without installing it on the host.
  Evidence and remaining limits belong in `PROJECT_STATUS.md`.

## 0.18.0: grouped freehand drawing and literal CLI help (published)

Group freehand strokes on one page into one annotation and one Undo step through **Finish drawing** /
**Cancel drawing**. Worker failure retains the draft for retry; conflicting save/close/document/tool actions
wait until Finish/Cancel. Reuse existing AddInk, point simplification, display geometry and protection (D109).
At most 64 strokes and 4,000 points per stroke. Drafts stay in memory until Finish and are absent from crash
recovery. No idle auto-commit or eraser. `stamp` / `page-numbers` help preserves literal `{n}` / `{total}`
through the existing clap help template (A16). No new dependency or CLI command; 53 commands remain.

[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) under `lmdanavanem`, release `415140702`,
source `7e52de82244faf9a020ed3ed233176775c007e97`. Local gates passed: 452 workspace tests (zero failed, seven default ignored),
formatting/Clippy with warnings denied, 907 matching EN/FR keys, 75 qpdf checks and the update-helper self-test.
Four licensed NIST/NBS documents (744 pages) passed in 50.796 seconds, sampled peak child memory
329,445,376 bytes; all sources stayed unchanged. Six new headless UI tests, the extended native-worker
grouped Undo/Redo/failure regression and the CLI help regression passed.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082734) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082934/job/117057854520) passed,
with sixteen portable cases, ten MSI baselines and 50 verified MSI logs. The CI corpus took 44.494 seconds,
sampled peak child memory 320,299,008 bytes. All six staged/public files, API digests/sidecars and enhanced
package checks passed: five grouped-Ink readbacks (plain/AES-128/AES-256 with both passwords), four literal-help
checks and existing conversion, forms, labels, links, dashes, compression, OCR and MSI payload checks.
Verified copies are in `dist/`; the production updater verified the 19,834,488-byte Latest x64 ZIP without
installing it on the host. Executed evidence and remaining limits are in `PROJECT_STATUS.md`.

## 0.19.0: mouse placement of form fields (published; Latest at the time)

1. Reuse **Prepare a form** and its five existing types. **Draw on page** hides the numeric dialog;
   a rectangle gesture reopens the same draft with name, choices and editable/multiple-selection flags retained.
2. Keep the rectangle finite, clamped to one page and at least four points per dimension. Show a crosshair and
   preview, exclude existing widget interactions and cancel stale placement after document revision/tab/tool changes.
3. Mutate only through explicit **Create Field**, retaining existing core validation, encryption and one Undo step.
   Numeric creation and core/CLI APIs remain available; still 53 commands and no new dependency or field type.
4. Headless interactions, settings retention, tiny/out-of-page gestures, stale targets, autosave and the extended
   five-type creation/undo regression passed in the workspace suite. Independent review, formatting and Clippy passed.
   CI/upgrade, staged/public package and production updater gates passed; six verified files are published. Executed results belong in status.

**0.19.0 was stable Latest at publication.** D110/A38 define the selected scope; XFA authoring and editing existing field
geometry/types are not included. Native mouse/pen/touch and Narrator checks remain separate.
Executed evidence belongs in `PROJECT_STATUS.md`.

## 0.20.0: on-page FreeText creation and editing (published; Latest at the time)

The implemented 0.20.0 lot creates and edits FreeText annotations directly on the page (D111/A39).
Click or draw the existing box, type in an anchored multiline editor and use explicit **Add** / **Save** /
**Cancel** with the existing size, colour, background and border controls. The existing AddFreeText /
UpdateFreeText worker operations keep validation, protection and one Undo step; no new dependency,
annotation type or CLI command is added (53 commands remain).

The draft stays bound to its original page. Zoom remains available; conflicting navigation waits for
confirmation/cancellation. Document revision/tab/tool/workspace changes cancel stale editing. Pending
open/save/mutations and concurrent drafts are guarded; validation or worker failure retains typed text
for retry. Helvetica WinAnsi appearance and downward wrap growth remain unchanged. The editor draft
exists only in memory until confirmation and has no crash recovery.

Implementation and independent review passed. Local workspace: 468 passed tests, zero failures, seven
default ignored; formatting, all-target Clippy with warnings denied and 912 matching EN/FR keys passed.
The 75-check qpdf gate, four-document/744-page real corpus and update-helper self-test passed. Windows CI,
18 portable cases, 12 MSI baselines/60 logs, same-source ARM64, all six staged/public files and the production
updater passed. **0.20.0 was published as stable Latest at the time**; [Project status](PROJECT_STATUS.md) records the executed evidence.

The initial [Code Scanning review](CODE_SCANNING_REVIEW.md) covered 117 then-open alerts, including 115 criticals;
no production secret was confirmed. At that snapshot, no alert had been dismissed and the release-workflow
permission declaration remained future work. The verified 0.21.0 outcome is recorded below.

## 0.21.0: Code Scanning remediation (published stable Latest)

Explicit native signer-certificate null checks and malformed-CMS regression; release root
`contents: read` with `contents: write` restricted to Publish (D112). XML 1.0 readers use `ParserConfig`,
retain bounds and reject DTD events; UTF-8/UTF-16 XMP/XFDF regressions passed. Deterministic fixtures
and complete scanning coverage are preserved.

471 tests, zero failures, seven default ignored; formatting/Clippy, 912 EN/FR keys, 75 qpdf checks,
four-document/744-page corpus and helper passed. CI, 19 portable cases (15 upgrades/four guards), 13 MSI
baselines/65 logs, same-source ARM64, six staged/public files, `dist/` copies and production updater verified.
CodeQL fixed #3 and #119; zero alerts remain open. All 115 criticals were individually dismissed with
evidence by `lmdanavanem`: 114 `used in tests`, #13 `false positive`. No folder or query is excluded.

The [GitHub project](https://github.com/users/lmdanavanem/projects/1) contains 42 prioritised drafts: Bugs (8), Roadmap (23), Suggestions (11),
with saved category filters. At 0.21.0 publication the security item was Done; current implementation
status for the other items is tracked in PROJECT_STATUS.md.
See [Code Scanning review](CODE_SCANNING_REVIEW.md)
and [Project status](PROJECT_STATUS.md). The 53 CLI commands are unchanged.

## Remaining work beyond these lots

1. Independent execution on ARM64, physical printers, Windows Narrator, interactive installed/portable update and save
   dialogs, and Acrobat/Foxit/Chrome. These require the actual hardware or readers and are not covered by scripted CI.
2. Expand the four public NIST fixtures with licensed customer/hostile/large scans and complex forms; add fuzzing and
   hardware-recorded performance measurements. Preserve the original on every failed or cancelled operation.
3. Font subsetting/CFF/OTF/TTC, paragraph editing and editable pasted objects.
4. Local Office conversion and additional batch conversion operations, with measured output fidelity.
5. Certificate-chain trust/revocation, trusted timestamps/LTV and executable/MSI signing when an identity is supplied.
6. Semantic page-label rebasing after page changes, complex untagged-column reading order,
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
- flattening: preserving non-Link annotations without usable appearances, including attachments and shared
  references, passed local and GitHub CI/CodeQL checks on its implementation source. 0.21.1 release checks
  and publication are pending; published 0.21.0 is unchanged;
  [executed checks and limits](PROJECT_STATUS.md).

## After sign-image (visual signature, slice delivered)

Typed signatures and all/selected-page placement were delivered in 0.13.0 (D102–D103).

Remaining, non-blocking (see `KNOWN_ISSUES.md` A35–A37, decisions D054–D060):
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
- JPEG EXIF 5/7 direct embedding and cumulative PNG/raster estimates are delivered in 0.17.0 (A31/D108);
  measured native/process memory and larger licensed inputs remain separate work;
- CLI help for `stamp` / `page-numbers` is included in the 0.18.0 release (A16); local/CI/package checks passed.

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
- grouped ink shipped in 0.18.0 with explicit Finish/Cancel and guarded save/close
  transitions (D109), replacing the idle-commit proposal; local/CI/package regressions passed. Eraser remains future work;
- polygon, polyline, cloud, stamp, attachment and rich free text (bold, italic, CJK: embedded font); dashed borders
  for the four existing shapes are delivered in 0.16.0, while other annotation types remain unsupported;
- richer destination-view authoring beyond whole-page Fit; existing-link replacement was delivered in 0.15.0 (D106);
- lighter round trip for big files (write the annotation without reloading the whole document);
- on-page FreeText creation and editing shipped in 0.20.0 (D111); local/CI/upgrade, staged/public package and production updater checks passed.

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
