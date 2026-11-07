# Project status

## 0.94.0 in progress

Independent re-reads (#106, still open): `python scripts/test-conformance.py --readers --cli <sunipdf-cli> --qpdf <qpdf> [--poppler <dir>] [--openssl <openssl>] --out <dir>`
writes AES-256 and AES-128 copies, XMP metadata, filled and flattened forms, every annotation family and a PFX
signature with the CLI, and asserts facts with QPDF (objects, encryption, permissions), Poppler (info, text,
`pdfsig`, raster against PDFium) and OpenSSL (CMS over the signed byte ranges). Missing tools are listed as
skipped in `report.json`. Windows CI runs it with QPDF 12.4.2 and OpenSSL; the Linux conformance job with
Ubuntu's QPDF and Poppler, where signing is unavailable. Locally (Windows x64 debug, QPDF 12.4.2, OpenSSL 3.5.7)
six cases passed, also with QPDF JSON v1; Poppler was not run locally. It found highlight, underline, strikeout
and note annotations saved without an appearance stream; fixed in `crates/sunipdf-core/src/annot.rs`.
Remaining: Poppler results from CI, `pdfsig` on a signed copy, and visual checks in Adobe Acrobat.

## Published 0.93.0

By Emanuel de Almeida. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.93.0) | [Release notes](RELEASE_0.93.0.md) | [Qualification evidence](qa/RELEASE_0.93.0.json).

Test-hardening increment: no new user-facing feature. #106, #105, #154 and #99 move to 0.94.0; #97 and #98 stay open. The tagged source `3ac860f5d976763a70f9572dd91562180b30228a` passed all 11 release jobs and the ci, linux and macos runs on main. Locally, the full workspace tests, strict Clippy, formatting, 2,202 bilingual keys and docs validation passed. All five packages and their SHA256 files were checked against their checksum files and GitHub digests (anonymously at 2026-11-05T00:35Z).

Generated hostile and large PDF corpus (#97, still open): `apps/sunipdf-cli/tests/hostile_corpus.rs`
builds 26 PDFs at test time and runs `info`, `text`, `render`, `optimize` and `redact` on each
under the existing 60 s, 768 MiB and 2 MiB output guards. Local Windows x64 debug run: 182 commands
in 22 s, no crash or timeout, peak working set 439 MiB. It found two defects, listed in the
[known issues](KNOWN_ISSUES.md). Licensed real-world documents, complex forms, tagged/OCR files,
cancellation and qpdf checks of these outputs remain open.

Every PDF writer on the same corpus (#98, still open): a second test in that file runs 56 PDF-writing
commands on the 23 hostile cases and nine of them on the three large cases (1,315 runs, four child
processes at a time; local Windows x64 debug run 62 s). Each run ends in a typed error that leaves
its output directory empty, or in outputs that reopen with the expected page count. It found four
panicking writers and five lopdf/signing writers publishing unreadable copies; both are fixed.
Commands that need a non-PDF input, a review or precondition JSON, or produce no PDF are listed as
out of scope in the test.

Fuzzing (#98): `fuzz/` holds two cargo-fuzz targets (lopdf structure loading/inspection/rewriting,
signature dictionaries and CMS envelopes) run by the `fuzz` workflow on Linux nightly, weekly or on
demand; no local or CI fuzzing run has been recorded yet. `crates/sunipdf-core/tests/fuzz_smoke.rs`
runs the same entry points on 10,000 fixed-seed mutations (30 s locally) in `cargo test`; this is not
coverage-guided. PDFium itself is not instrumented. Hardware benchmarks remain open.

## Published 0.92.2

By Emanuel de Almeida. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.2) | [Release notes](RELEASE_0.92.2.md) | [Qualification evidence](qa/RELEASE_0.92.2.json).

Fixes copying text to the Windows clipboard (#157): the eframe system clipboard feature was missing
from the Windows build, so copied text stayed inside the application. A regression test and a real
Ctrl+C check now cover it. The author name spelling is corrected. The tagged source
`1938aa3d71687b980ccbe568446418c5ea7b1b82` passed all 11 release jobs, including 61 portable baselines plus
four guards and 59 MSI baselines with the published 0.92.1 packages. Locally, 585 desktop and
translation tests, strict Clippy, formatting and 2,202 bilingual keys passed. All ten public
downloads were checked anonymously against their SHA256 files and GitHub digests at 2026-11-04T20:17:40Z.
## Published 0.92.1

By Emanuel de Almeida. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.1) | [Release notes](RELEASE_0.92.1.md) | [Qualification evidence](qa/RELEASE_0.92.1.json).

Adds the **Window at startup** preference (last state, maximized or windowed, centered when windowed)
and keeps the MSI Start Menu shortcut icon stable so Windows taskbar pins survive updates.
CI now removes evidence folders restored with its build cache. Includes Dependabot #79 and #84.
The tagged source `cfc9f64a62971da04ef7224f394e19ebd7f1411b` passed all 11 release jobs, including 60 portable baselines plus four
guards and 58 MSI baselines with the published 0.92.0 packages and a shortcut-icon check. Locally,
584 desktop and translation tests, strict Clippy, formatting and 2,202 bilingual keys passed, and a
real window check confirmed centered, maximized and remembered starts. All ten public downloads were
checked anonymously against their SHA256 files and GitHub digests at 2026-11-03T23:06:06Z.

## Published 0.92.0

By Emanuel de Almeida. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.0) | [Release notes](RELEASE_0.92.0.md) | [Qualification evidence](qa/RELEASE_0.92.0.json).

The About dialog follows the supplied mockup in French and English: centered identity and version,
privacy badges and panel, real update status, persisted update/recovery switches, third-party notices,
and Sunitech credits. Its header and Close button remain accessible when the body must scroll.

The Objects panel has reversible eye controls for reader-only visibility. Masks stay with each tab,
apply to page tiles and thumbnails, and exclude hidden objects from canvas selection. They never
remove objects from saved PDFs. Editing the document resets the masks because native object indices
can change; the interface explains this behavior.

Overlay layer names and eye controls toggle visibility and refresh the preview on the engine worker.
Visibility-only changes share the existing source rasters; source hashes are checked again, including
hidden layers. Hiding every layer produces a white canvas. Exports and saved setups retain the actual
visibility settings and the existing source-validation guards.

The tagged source `306fbc25a39f86fad0aa24d31b130ce0dcd50562` passed all 11 release jobs.
Windows x64 passed 1,363 workspace tests with zero failures and 16 default ignored tests,
strict Clippy, formatting and 2,197 bilingual keys. Migration checks passed 59 portable baselines
plus four guards and 57 MSI baselines, including the published 0.91.2 packages.
The packaged overlay corpus passed 137 checks across 25 commands, including all-white pixels
and retained source metadata when every layer is hidden, plus missing/stale hidden-source refusals.
Three About frames from the final source match the visually reviewed DX12 WARP images:
French/light, English/dark and French at 720 Ã— 480. These are offscreen renders, not native-window
or physical-GPU qualification. All ten public downloads were checked anonymously against their
sizes, SHA256 files and GitHub digests; stable Latest was verified at 2026-10-29T02:55:40Z.

## Published 0.91.2

[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.91.2) | [Release notes](RELEASE_0.91.2.md) | [Qualification evidence](qa/RELEASE_0.91.2.json).

Corrective release of 0.91.0 without new workflows: saved text placement and rotated-page capture,
register field size limits, retained unreadable saved filters, translated messages and accessible names.
It republishes the corrections of 0.91.1, which was withdrawn shortly after publication.
The tagged source passed all 11 release jobs, including 58 portable upgrade baselines plus 4 guards
and 56 MSI upgrade baselines, now starting from the published 0.91.0 packages. Locally, the workspace
passed 1,359 tests with clippy and formatting clean. All ten public downloads were checked anonymously
against their SHA256 values, verified 2026-10-28T23:24:07Z.

## Published 0.91.0

[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.91.0) | [Release notes](RELEASE_0.91.0.md) | [Qualification evidence](qa/RELEASE_0.91.0.json).

Reviewed registers, reusable text and dimensioned plan measurements. Create calibrated dimensioned rectangles at an explicit clockwise angle, with matching preview and saved measurement geometry. Compare two to four saved PDF pages with independent layer colors, visibility, opacity and reviewed alignment. Capture supported native stamp and image groups into the local annotation library, preserving pixels and transparency. Save reusable overlay setups with exact source hashes, including hidden layers, and require a fresh preview when reopening. Qualify reviewed two-source plan-link creation, guarded new-copy saves, exported receipts and verified destination navigation in installed Ubuntu packages. Convert formula inputs explicitly between supported length, area or volume units with checked decimal arithmetic and retained source-unit guards. Review and apply keep/set/clear changes to business status, discipline and individual custom text keys across up to 256 explicitly selected annotations as one atomic Undo/Redo operation. Refuse stale, locked or invalid selections as a complete batch; the annotation-fields-batch CLI previews changes and saves a new copy without overwriting existing files. Save and exchange named register filters with their criteria and sort order; the bounded local library supports 16 entries and 128 KiB. Match one stored custom text key and value literally and case-sensitively, distinguishing present empty values from missing fields; share the criterion across register, quantity and issue-report selections. Use version-2 saved-filter libraries only when the custom-field criterion is present, preserving the exact version-1 representation otherwise. Capture supported saved native rich-text annotations using twelve Helvetica, Times and Courier font/style variants with supported shapes and stamps, preserving verified appearances and geometry across cropped page rotations. Create dimensioned circles and ellipses from explicit real dimensions, rotation and a captured/page/region/automatic calibration; keep their 128-vertex polygon approximation visible and compute area from saved vertices. Create dimensioned two-point segments from a real length and clockwise angle with the same explicit scale sources; save an ordinary editable calibrated length without permanent geometric constraints. Review source-bound previews before applying the complete measurement, and refuse invalid dimensions, stale references, unavailable scales or existing CLI output files.

The tagged source passed all 11 release jobs. Windows qualification includes
20 offscreen interface scenes (3 Home and 17 PDF/settings scenes), 57 portable upgrade baselines plus
4 guards, and 55 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 27 installed runtime cases, including 6 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-26T04:09:34.597549+00:00.

Visual review covers 36 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.91.0.json).

The ten retained packaged-CLI corpora include register-field batches with 113 checks, 35 commands, 18 cases and 16 refusals. Native plan-link creation separately passed five cases and 32 retained scenes per Ubuntu version; the six navigation cases retain their separate scope.

Four additional Windows WARP scenes cover English and French register dialogs: two real batch previews and two saved-filter views. Their recorded source bindings, control states, text geometry and pixels were checked independently.

The new register-batch corpus covers 113 checks, 35 commands, 18 cases and 16 refusals; the saved-text corpus covers 388 checks, 45 commands, eight cases and four refusals. Actual plain PDF graphs, saved text glyph mappings and rendered pixels are independently checked. Encrypted register output retains independently inspected QPDF objects and permission dictionaries; ephemeral passwords are not retained for a second independent decryption. Four additional offscreen dialog scenes qualify recorded rendering and control states, not native mouse/keyboard, Undo/Redo interaction or physical accessibility. Exact field filtering adds no numeric ranges or regular expressions and has no separate packaged corpus. Foreign, embedded or arbitrary text fonts, unsupported semantics and incompatible text-box geometry remain refused. All affected backlog cards retain their remaining acceptance criteria. Rectangle rotation does not create persistent geometric constraints. Saved annotation capture retains its supported native appearance and image bounds. Overlay setups retain source hashes and configuration, not PDF data, rendered rasters or credentials; automatic registration and layered PDF output remain open. Formula conversion supports only the existing geometric unit families; each conversion rounds to six decimal places and sub-micro values can round to zero. Source quantities keep their exact unit guard. Every affected backlog card retains its remaining acceptance criteria. Circles and ellipses use 128 editable polygon vertices, about 0.0402% below ideal analytic area before coordinate rounding. Segments use two stored endpoints and linear quantities; none of these shapes has a persistent analytic constraint. The ellipse corpus verifies 327 checks, 62 commands, 27 cases and 19 refusals across eight PDFs/1024 vertices. The segment corpus verifies 359 checks, 67 commands, 29 cases and 20 refusals across nine PDFs/18 endpoints. Both retain actual PDF/QPDF, appearance and stored-quantity checks across four cropped rotations and five units. No new native shape mouse/keyboard, Undo/Redo, physical survey or external-reader qualification is claimed. Independent X/Y calibration, permanent constraints, analytic curves and broader interoperability remain open; AEC-008 remains In progress.

Installed Ubuntu creation qualification uses original synthetic searchable PDFs and actual X11/Openbox/Xvfb controls. It verifies two reviewed source copies, independent QPDF page/content/resource checks, an exported save receipt, occupied-output refusal and navigation to physical destination page two. The separate six-case navigation corpus retains its recorded scope. Image-only references, encrypted creation inputs, dirty-tab creation, Wayland, physical GPUs and external-reader interoperability remain unqualified. Windows offscreen scenes do not establish native plan-link creation.

Windows packages remain unsigned, and Windows ARM64 is cross-built without
native hardware execution. The Apple Silicon app is ad hoc signed, not
notarized; macOS 12 is a binary-checked deployment minimum. Physical Macs,
interactive Finder opening and clipboard interoperability remain unqualified.
Ubuntu updater-helper checks do not establish graphical authorization or a
complete desktop updater interaction. Physical GPUs, Wayland, printers and
human accessibility remain separate gates.

The corresponding annotation-workflow backlog card remains in progress; full acceptance needs its remaining independent criteria.

## Remaining work

AEC-017 remains **In progress**. Native navigation is qualified within the
synthetic scope above; desktop link creation/saving, broader batch interaction
and external-reader interoperability remain open. Batch saves are atomic per
file, completed copies remain after cancellation or failure, and original
indexed PDFs remain the link destinations. See [workflow and limits](PLAN_LINKS.md).

AEC-011 visual search and AEC-012 region fill retain their open native and
broader-drawing criteria. Other backlog statuses remain unchanged. Independent
update signing, publisher certificates, native spreadsheets, private project
plans and applicable human accessibility checks retain their separate gates.
No complete P1/P2 category, universal interoperability, general PDF/A/PDF/UA
conformance or overall performance claim is made.

[Backlog](AEC_BACKLOG.md) Â· [Roadmap](ROADMAP.md) Â· [Known issues](KNOWN_ISSUES.md) Â· [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Repository documentation does not establish GitHub Projects synchronization. English remains the default UI.

## History

[0.77.0 qualification](RELEASE_0.77.0.md) retains its original source, counts and limits.

[0.76.0 qualification](RELEASE_0.76.0.md) retains its original source, counts and limits.

[0.75 qualification](RELEASE_0.75.0.md) retains the original Apple Silicon release evidence.

[0.74 qualification](RELEASE_0.74.0.md) retains the original theme and Ubuntu-update evidence.

[0.73 qualification](RELEASE_0.73.0.md) retains its original counts, source and limits.

Earlier qualification stays with [0.72](RELEASE_0.72.0.md), [0.71](RELEASE_0.71.0.md),
[0.70](RELEASE_0.70.0.md), [0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical counts are not added to this release.
The 0.69/0.70 CLI performance comparison retains its original binary, machine and workload scope in
[0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify later releases.
