# Project status

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

[Backlog](AEC_BACKLOG.md) · [Roadmap](ROADMAP.md) · [Known issues](KNOWN_ISSUES.md) · [1.0 criteria](RELEASE_1.0_CRITERIA.md).
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
