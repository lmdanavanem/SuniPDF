# SuniPDF roadmap

[Français](ROADMAP.fr.md)

This roadmap separates delivered features and future work.
Executed checks and release evidence belong in [PROJECT_STATUS.md](PROJECT_STATUS.md).

## Delivered baseline: 0.9.0

The seven requested features are delivered: autosave/recovery, Windows printing, richer text selection,
two-page/Book view, text/checkbox form-field creation, offline English/French OCR, and CMS/PFX digital signatures.
English is the default language; French remains available. All 14 tools have working commands.
See [FEATURE_MATRIX.md](FEATURE_MATRIX.md) for the exact scope and [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for limits.

## 0.9.1: reliability and distribution

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) under `lmdanavanem`;
executed validation and package evidence are tracked in `PROJECT_STATUS.md`:

- Preserve PDF/A identification and unrelated XMP properties when editing metadata; reject malformed XML and DTDs.
  Preserving identification does not certify PDF/A conformance.
- Write the explicit AES-256 encryption key length required by strict readers, including after re-encryption.
- Add pinned qpdf structural checks for synthetic fixtures and generated output to CI.
  This is independent structural QA, not coverage of a real-world or hostile PDF corpus.
- Seven real portable update/failure/rollback scenarios to 0.9.1 and a real MSI installation/upgrade/transactional rollback,
  downgrade rejection and uninstall regression passed on disposable Windows runners, preserving user data.
- Use .NET SHA256 hashing in the helper for Windows PowerShell 5 hosts without Get-FileHash and report restart failures.
- Derive the Windows icon from the dedicated, retouched author-provided asset (`assets/icon/sunipdf-source.png`).

Workspace checks (322 tests), independent qpdf checks (22 PDFs/three malformed inputs), portable helper checks and
the disposable-runner MSI regression passed. All six public assets were downloaded and matched the verified
payloads/checksums. Interactive updater UX and real hardware remain separate checks.

## 0.10.0: requested P1/P2 lot published

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) under `lmdanavanem`.

| Area | Included scope |
|---|---|
| P1 data safety and corpus | Atomic temporary-file replacement, interrupted/full-disk/failure regressions, killed partial-save child and four pinned public NIST documents |
| P2 compression | Image resampling/JPEG quality, before/after size, progress/cancel, source-preserving new copies and conservative unsupported-image handling |
| P2 reading | View-only rotation, full screen, bounded per-tab navigation history and upright editing |
| P2 forms | Radio/combo/list creation, XFDF export, bounded JSON/XFDF/FDF values import, grouped Apply/Discard and keyboard widgets |
| P2 editing | Mouse crop preview and grouped rotation/resizing with one undo |
| P2 batches | Sequential compression/offline OCR, bounded inputs, cancellation, per-file errors and no overwriting |

The 362 workspace tests, 31 independent synthetic qpdf checks, four public NIST documents (744 pages), eight portable
update/failure/rollback cases and two MSI baselines passed. Final CI and all six public assets passed verification,
including packaged CLI/OCR, icons, MSI payloads and the live updater. Evidence is recorded in `PROJECT_STATUS.md`.
Hardware, native interactive flows, Narrator, third-party readers and broader customer/hostile corpus coverage remain independent work.

## 0.11.0: navigation, forms and protected copies (published)

Implemented and locally validated: read-only page labels with exact unique-label navigation and `#N` physical
indices; named destinations and `/XYZ`, `/Fit`, nonzero `/FitH` for links/bookmarks; same-page and zoom-aware history;
form-read errors that preserve inputs and require an explicit retry. `/FitH` zero/null and other fit modes fall back
to page navigation. Also implemented for 0.11.0: free text in existing editable drop-downs through the panel/on-page
editor, and extraction/split copies retaining source encryption/passwords/permissions with same-source-path refusal.
The existing permission policy is unchanged; editable drop-down authoring was outside the 0.11.0 scope.

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

## 0.12.0: editable field creation and radio exports (published; Latest at the time)

Create editable drop-downs from Prepare a form or `add-form-field --kind combo --editable`, with predefined choices
still required. Radio `/Opt` export values are separated from widget appearance states for filling and JSON/XFDF/FDF
interchange; CLI information adds `widgets[].export_value`. Malformed or duplicate mappings keep the affected
field read-only; no-`/Opt` radios retain state-name values (D100–D101).

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (Latest at the time), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. The 398 workspace tests (zero failed, seven default
ignored), formatting/Clippy, 859 EN/FR keys, 48 qpdf checks and four NIST/NBS documents (744 pages; 59.68 seconds)
passed. [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688), the
[same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878), ten portable
scenarios and four MSI baselines passed. All six staged/public files and enhanced binary checks passed; verified
files are in `dist/`. The production updater verified the 19,699,323-byte Latest x64 ZIP without installation.
Executed evidence and hardware/interoperability limits are recorded in `PROJECT_STATUS.md`.

## 0.13.0: visual signature improvements (published; Latest at the time)

Typed name/initials use an explicit native preview and the existing font/image/library paths. Current/all/selected
pages share fitted display-space geometry; multi-page placement is atomic, cancellable between targets and one
undo. CLI `add-image --pages` is additive; legacy `--page` behaviour remains compatible (D102–D103).
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) under `lmdanavanem` (Latest at the time), release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Local validation passed: 410 workspace tests,
formatting/Clippy, 870 EN/FR keys, 53 qpdf checks and four licensed documents (744 pages; 76.40 seconds locally).
Windows CI, same-source ARM64 compilation, eleven portable scenarios, five MSI baselines and all six staged/public
files with enhanced binary checks passed. Verified copies are in `dist/`; the production updater verified the
19,730,578-byte Latest x64 ZIP without installation. Evidence and hardware/reader limits: `PROJECT_STATUS.md`.

## 0.14.0: page-label authoring (published; Latest at the time)

Apply six navigation-label styles over an inclusive physical range through Pages/Organize and two CLI commands;
preserve labels outside the range, reject malformed existing trees before mutation, and reset all label rules
without claiming secure erasure. Printed numbers remain unchanged; structural operations do not semantically
rebase the ranges. Geometry-preserving label/content Undo/Redo retains navigation (D104–D105).
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) under `lmdanavanem` (Latest at the time), release
`414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation passed: 420 workspace tests,
formatting/Clippy, 890 EN/FR keys, 63 qpdf checks and four licensed documents (744 pages; 53.91 seconds locally).
Windows CI, same-source ARM64 compilation, twelve portable scenarios, six MSI baselines and six staged/public
files with enhanced checks passed. Verified copies are in `dist/`; the production updater verified the
19,785,200-byte Latest x64 ZIP without installation. Executed evidence and remaining limits: `PROJECT_STATUS.md`.

## 0.15.0: editing existing link targets (published milestone)

Reuse the annotation link dialog and protected writer to change an existing destination through Edit target,
double-click or CLI `set-link-target`. Safe web/mail URI or physical-page Fit replaces all previous target/action
fields, keeping the link's area/style/comments/replies and unrelated document data. Existing detailed/named views
are explicitly replaced by whole-page Fit; unknown targets never default silently to page 1. ReadOnly/Locked/
LockedContents and malformed flags are refused; one undo preserves geometry/navigation and stale dialogs close
on document changes (D106). [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) under
`lmdanavanem` (Latest at the time), release `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation passed: 429 workspace
tests, formatting/Clippy, 894 EN/FR keys, 68 qpdf checks and four licensed documents (744 pages; 48.891 seconds
locally). Windows CI, same-source ARM64 compilation, thirteen portable scenarios, seven MSI baselines and six
staged/public files with enhanced checks passed. Verified copies are in `dist/`; the production updater verified
the 19,803,226-byte Latest x64 ZIP without installation. Executed evidence and remaining limits: `PROJECT_STATUS.md`.

## 0.16.0: dashed shape annotations (published milestone)

Dashed rectangle/ellipse/line/arrow annotations reuse the existing toolbar, CLI and appearance writer. UI and
`annotate --dashed` choose `[6,3]` points; actual custom patterns remain through ordinary edits unless explicitly
toggled. Solid defaults, continuous fill and solid arrowheads stay unchanged. Unsupported types, malformed
SuniPDF border styles and excessive regeneration work are refused (D107). No dependency or command is added;
the CLI remains at 53 commands. [Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0)
under `lmdanavanem` (Latest at the time), release `414934190`, source `6cd7843734f3924e87033d8b05539d493ccbcb95`. Local validation,
Windows CI and same-source ARM64 compilation passed: 439 tests, 895 EN/FR keys, 73 qpdf checks, four licensed
documents/744 pages (53.542 seconds locally), fourteen portable cases and eight MSI baselines. Six staged/public
files and enhanced checks passed; verified copies are in `dist/`. The production updater verified the
19,821,719-byte x64 ZIP without installation. Evidence and remaining limits are in `PROJECT_STATUS.md`.

## 0.17.0: image conversion reliability (published; Latest at the time)

Complete direct JPEG embedding for EXIF orientations 5 and 7, so all eight orientations avoid image decoding/
recompression. Preflight non-JPEG inputs against a cumulative 512 MiB raster-data estimate:
`sum(max(decoder.total_bytes(), width × height × 4))`, with checked arithmetic, finite/positive physical geometry
and repeated loading checks for simple file changes (D108). Quality/transparency and existing UI/CLI/API remain.
The estimate is not a native/process RSS ceiling or compressed-JPEG limit. No dependency or command
is added. Budget excess/overflow has a localized `image_batch_too_large` error; invalid geometry retains `other`.
Local validation passed: 445 workspace tests, formatting/Clippy, 896 EN/FR keys, 74 qpdf checks and four licensed
documents/744 pages (48.187 seconds; peak child memory 330,264,576 bytes), the update-helper self-test and
independent debug conversion gate. Source `5785de282e126fb4666b28fb83340d0e9c6e9acf` passed Windows CI,
same-source ARM64 compilation, fifteen portable cases, nine MSI baselines and 45 MSI logs; CI corpus took
45.791 seconds with peak child memory 321,413,120 bytes. [Published (Latest at the time)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0)
under `lmdanavanem`, release `415074108`, with six files. All staged/public files and enhanced conversion checks
passed; verified copies are in `dist/`. The production updater verified the 19,825,743-byte x64 ZIP without
installation. Evidence and remaining limits belong in `PROJECT_STATUS.md`.

## 0.18.0: grouped freehand drawing and literal CLI help (published)

Group freehand strokes on one page into one annotation and one Undo step through **Finish drawing** /
**Cancel drawing**. Worker failure retains the draft for retry; conflicting save/close/document/tool actions
wait until Finish/Cancel. Reuse existing AddInk, point simplification, display geometry and protection (D109).
At most 64 strokes and 4,000 points per stroke. Drafts stay in memory until Finish and are absent from crash
recovery. No idle auto-commit or eraser. `stamp` / `page-numbers` help preserves literal `{n}` / `{total}`
through the existing clap help template (A16). No new dependency or CLI command; 53 commands remain.

[Published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) under `lmdanavanem`, release `415140702`,
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

## Phase 0–1: planning and foundations — delivered

Rust workspace, PDFium, typed errors, opening/navigation/zoom/close, passwords, drag and drop, FR/EN localisation,
persisted settings, light/dark themes, command registry and Windows resources are in place.
Historical milestones and decisions remain in `DECISIONS.md` and `PROJECT_STATUS.md`.

## Phase 2: reading and core PDF operations — largely delivered

Delivered: tabs, bounded render/thumbnail caches, continuous/two-page/Book views, logical cross-page selection,
search, bookmarks, merge/split/extract/rotate/reorder, atomic save, undo/redo and main annotations.

Delivered in 0.10.0: view-only rotation, full-screen reading and bounded per-tab navigation history.
Delivered in 0.11.0: page-label reading and precise internal navigation. Authoring/reset and navigation-preserving
Undo/Redo were delivered in 0.14.0. Remaining: semantic label rebasing and intended reading order for complex untagged columns. qpdf structural checks do not
prove preservation of every link, field, layer or attachment.

## Phase 3: professional editing — partially delivered

Delivered: editable text/images and shapes, object transformations, grouped move and clipboard, system TrueType
fonts, stamps/watermarks/numbering, threaded comments, metadata, visual signatures, AcroForm filling/flattening,
text/checkbox creation and autosave/recovery.

Delivered in 0.10.0: grouped rotation/resizing, mouse crop preview, radio/combo/list creation, strict value-only
FDF/XFDF interchange and keyboard page widgets.
Delivered in 0.12.0: editable drop-down creation and radio `/Opt` export mapping with conservative read-only safeguards.
Delivered in 0.13.0: typed visual signatures and atomic current/all/selected-page placement.
Delivered in 0.15.0: existing-link target replacement, preserving its area/appearance/comments.
Delivered in 0.16.0: dashed borders for the four existing shape annotations, with custom-pattern preservation.
Delivered in 0.18.0: grouped desktop ink with explicit Finish/Cancel and guarded transitions (D109).
Remaining: font subsetting/CFF/OTF/TTC, paragraph editing and editable pasted objects.
Acceptance: rendering, saving and reopening covered for each added operation, with applicable accessibility checks.

## Phase 4: conversion and OCR — partially delivered

Delivered: images ↔ PDF, PDF → text/PNG, lossless optimisation, CLI batches and bundled local English/French OCR.

Delivered in 0.17.0: all eight JPEG EXIF orientations embedded directly and cumulative raster-data estimates;
measured native/process memory limits remain separate work (D108).

Remaining: Office → PDF through a detected local converter, PDF → Word/Excel/PowerPoint with explicit measured
fidelity limits and reusable conversion profiles. Image compression and sequential compression/OCR batch UI are delivered in 0.10.0.

## Phase 5: security and productivity — partially delivered

Delivered: AES-128/256 and permissions, owner-authorised protection removal, metadata cleanup, irreversible redaction,
CMS integrity verification and incremental PFX/P12 signed copies, JSON CLI and automation commands.

Remaining: comparison, multi-document search, confined opt-in MCP, Explorer integration and configurable shortcuts.
Certificate-chain trust, revocation, trusted timestamping, AATL/EUTL, PKCS#11 and LTV are **not implemented**;
PDF signing time is a signer declaration. Acrobat/Foxit interoperability is a separate, unverified property.
Executable/MSI code signing needs an appropriate supplied identity.

## Phase 6: quality and distribution — ongoing

Delivered baseline: x64 portable/MSI and ARM64 portable releases, SHA256 files, startup update notifications with
explicit installation, FR/EN guides, third-party notices and GitHub build workflows.

Delivered in 0.10.0: four pinned public NIST samples and interrupted/full-disk/failed-save scenarios.
Remaining: customer/hostile/large corpus expansion, fuzzing,
published benchmarks with hardware, complete dependency licence inventory, Narrator and third-party reader checks,
interactive installed/portable update flows, physical printers and real ARM64/OpenGL execution.
Automated MSI regression covers its scripted scenario only; packaged cross-compilation is not ARM64 runtime validation.

## Work beyond the selected 0.18.0 lot

1. Run native updater/save flows, Narrator, printers, ARM64 and third-party readers on the appropriate equipment.
2. Extend real-corpus coverage and performance measurements; public NIST samples do not establish universal compatibility.
3. Address fonts/paragraph editing, local Office conversion and trust services as separately scoped future work.



No future release date or version is committed by this backlog. Optional AI remains disabled by default.
