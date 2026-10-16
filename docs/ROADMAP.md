# SuniPDF roadmap

[Français](ROADMAP.fr.md)

This roadmap separates delivered features, implemented changes awaiting validation, and future work.
Executed checks and release evidence belong in [PROJECT_STATUS.md](PROJECT_STATUS.md).

## Delivered baseline: 0.9.0

The seven requested features are delivered: autosave/recovery, Windows printing, richer text selection,
two-page/Book view, text/checkbox form-field creation, offline English/French OCR, and CMS/PFX digital signatures.
English is the default language; French remains available. All 14 tools have working commands.
See [FEATURE_MATRIX.md](FEATURE_MATRIX.md) for the exact scope and [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for limits.

## 0.9.1: reliability and distribution

[Published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) under `lmdanavanem`;
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

## Phase 0–1: planning and foundations — delivered

Rust workspace, PDFium, typed errors, opening/navigation/zoom/close, passwords, drag and drop, FR/EN localisation,
persisted settings, light/dark themes, command registry and Windows resources are in place.
Historical milestones and decisions remain in `DECISIONS.md` and `PROJECT_STATUS.md`.

## Phase 2: reading and core PDF operations — largely delivered

Delivered: tabs, bounded render/thumbnail caches, continuous/two-page/Book views, logical cross-page selection,
search, bookmarks, merge/split/extract/rotate/reorder, atomic save, undo/redo and main annotations.

Remaining: view-only rotation, navigation history, page labels, full-screen reading and intended reading order
for complex untagged columns. qpdf structural checks do not prove preservation of every link, field, layer or attachment.

## Phase 3: professional editing — partially delivered

Delivered: editable text/images and shapes, object transformations, grouped move and clipboard, system TrueType
fonts, stamps/watermarks/numbering, threaded comments, metadata, visual signatures, AcroForm filling/flattening,
text/checkbox creation and autosave/recovery.

Remaining: font subsetting and CFF/OTF/TTC support; paragraph editing; editable pasted objects; grouped rotation/resizing;
mouse cropping; more field types and FDF/XFDF interchange; keyboard activation of on-page widgets.
Acceptance: rendering, saving and reopening covered for each added operation, with applicable accessibility checks.

## Phase 4: conversion and OCR — partially delivered

Delivered: images ↔ PDF, PDF → text/PNG, lossless optimisation, CLI batches and bundled local English/French OCR.

Remaining: Office → PDF through a detected local converter, PDF → Word/Excel/PowerPoint with explicit measured
fidelity limits, image resampling/lossy compression, batch UI and reusable conversion profiles.

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

Remaining: real/hostile/large PDF corpus with licensed fixtures, interrupted/full-disk save scenarios, fuzzing,
published benchmarks with hardware, complete dependency licence inventory, Narrator and third-party reader checks,
interactive installed/portable update flows, physical printers and real ARM64/OpenGL execution.
Automated MSI regression covers its scripted scenario only; packaged cross-compilation is not ARM64 runtime validation.

## Recommended order after 0.9.1

| Priority | Work | Exit evidence |
|---|---|---|
| P1 | Real PDF corpus and data-safety scenarios | Reproducible fixtures, output checks and no original-file loss |
| P1 | Installed/portable updater UX, Narrator, printer and reader compatibility | Recorded outcomes on the actual Windows flows/readers/hardware |
| P1 | ARM64 runtime and performance baseline | Device/driver details and repeatable opening/rendering/memory measurements |
| P2, next feature | Image resampling/JPEG compression | Measured size, cancellation, source preservation, reopen/render/qpdf and semantic checks |
| P2 | View-only rotation, full screen and navigation history | Correct zoom/selection/navigation in continuous and two-page views |
| P2 | Fonts, paragraph/group editing, mouse cropping | Save/reopen/render checks on normal, rotated and cropped pages |
| P2 | Field types, XFDF/FDF and keyboard widgets | Interchange and keyboard/accessibility regressions |
| P2 | Comparison, batch UI and local Office conversion | Bounded jobs, cancellation, output fidelity and source preservation |
| Later | Trust services, timestamps/LTV, MCP and optional integrations | Explicit trust model, scope and security validation before delivery |

Recommended next feature slice: image compression with real-corpus and interrupted/full-disk save checks.
No future release date or version is committed by this backlog. Optional AI remains disabled by default.
