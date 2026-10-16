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

[Published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) under `lmdanavanem`.

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

## Phase 0–1: planning and foundations — delivered

Rust workspace, PDFium, typed errors, opening/navigation/zoom/close, passwords, drag and drop, FR/EN localisation,
persisted settings, light/dark themes, command registry and Windows resources are in place.
Historical milestones and decisions remain in `DECISIONS.md` and `PROJECT_STATUS.md`.

## Phase 2: reading and core PDF operations — largely delivered

Delivered: tabs, bounded render/thumbnail caches, continuous/two-page/Book views, logical cross-page selection,
search, bookmarks, merge/split/extract/rotate/reorder, atomic save, undo/redo and main annotations.

Delivered in 0.10.0: view-only rotation, full-screen reading and bounded per-tab navigation history.
Remaining: page labels and intended reading order
for complex untagged columns. qpdf structural checks do not prove preservation of every link, field, layer or attachment.

## Phase 3: professional editing — partially delivered

Delivered: editable text/images and shapes, object transformations, grouped move and clipboard, system TrueType
fonts, stamps/watermarks/numbering, threaded comments, metadata, visual signatures, AcroForm filling/flattening,
text/checkbox creation and autosave/recovery.

Delivered in 0.10.0: grouped rotation/resizing, mouse crop preview, radio/combo/list creation, strict value-only
FDF/XFDF interchange and keyboard page widgets. Remaining: font subsetting/CFF/OTF/TTC, paragraph editing and editable pasted objects.
Acceptance: rendering, saving and reopening covered for each added operation, with applicable accessibility checks.

## Phase 4: conversion and OCR — partially delivered

Delivered: images ↔ PDF, PDF → text/PNG, lossless optimisation, CLI batches and bundled local English/French OCR.

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

## Next work after the 0.10.0 lot

1. Run native updater/save flows, Narrator, printers, ARM64 and third-party readers on the appropriate equipment.
2. Extend real-corpus coverage and performance measurements; public NIST samples do not establish universal compatibility.
3. Address fonts/paragraph editing, local Office conversion and trust services as separately scoped future work.



No future release date or version is committed by this backlog. Optional AI remains disabled by default.
