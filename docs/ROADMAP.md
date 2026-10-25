# Roadmap

## Next development

Following increment: [0.78 Glass refinement and return to the source plan](RELEASE_0.78.0.md),
with a calmer workspace and preserved reading positions. Implementation and qualification are
tracked separately from the preceding release.

In development: [0.77 linked-sheet previews and native navigation checks](RELEASE_0.77.0.md).
This lot also caches historical Windows packages to reduce repeated download work
without reducing the upgrade scenarios.

Independent synchronized reader views are delivered in [0.76.0](RELEASE_0.76.0.md):
two locations in one PDF, one active editing context and pane positions retained
when switching tabs during the session.
This increment does not close other plan-workflow cards.

Native plan-link interaction and broader drawing/interoperability qualification
remain priorities. AEC-011 visual search and AEC-012 region fill retain their open
native and broader-drawing criteria. Implementation and complete acceptance are
tracked separately.

## Published scope

[0.76.0](RELEASE_0.76.0.md) is the current stable release, with independent reader
views and optional page, zoom, scroll and orientation synchronization. Its
[qualification evidence](qa/RELEASE_0.76.0.json) records exact-source checks,
Windows upgrades, native Ubuntu split-reader checks and Apple Silicon packages.

[0.75](RELEASE_0.75.0.md) introduced Apple Silicon packages.
[0.74](RELEASE_0.74.0.md) introduced [Theme settings](APPEARANCE.md) and direct
[Ubuntu updates](UBUNTU.md#settings-recovery-and-updates). Earlier evidence below
retains its original scope.

Updated 2026-10-24T17:32:54.306206Z. [0.73.0](RELEASE_0.73.0.md) adds reviewed multi-source plan-link batches and clearer
Linux window controls. Published stable with [qualification evidence](qa/RELEASE_0.73.0.json). AEC-017 remains **In progress**.
Window controls passed native interaction and image review on Ubuntu 22.04 and 24.04 under X11/Openbox/Xvfb with software rendering in the light theme. Dark-theme appearance, Wayland, physical GPUs and Windows interaction were not exercised. This does not qualify the plan-link interface's native workflow.

[0.72 region fill](REGION_FILL.md) and [0.71 Find symbols](VISUAL_SEARCH.md) retain their open native
and broader-drawing criteria; AEC-012 and AEC-011 remain **In progress**. Their evidence stays with
[0.72](RELEASE_0.72.0.md) and [0.71](RELEASE_0.71.0.md). [0.70](RELEASE_0.70.0.md) retains declared
numeric-field units and explicit workbook field history.

AEC-007 remains **In progress**: exact units and bounded geometric formulas are implemented, while native interaction and broader qualification remain open. AEC-018 and AEC-019 also retain their open criteria. Currency exchange, taxes, mass/density and broader estimating workflows remain extensions.

Historical 0.69 record, updated 2026-10-24. [0.69.0](RELEASE_0.69.0.md) combines issue reports and quantity workbooks.
Published with [Qualification evidence](qa/RELEASE_0.69.0.json).
Full backlog cards stay open until their stated acceptance criteria are evidenced; implemented increments
do not close missing native, interoperability, hardware or accessibility qualification.

## Added in 0.69

AEC-018 adds selection-bound PDF/CSV/JSON reports with a real paginated preview, separate totals,
explicit coverage, supported style legends and a technical appendix. AEC-019 adds XLSX snapshots
and reviewed manifest refresh through stable annotation identities. Both remain **In progress**:
native desktop/spreadsheet interaction and remaining interoperability qualification are open.

See [reports](ISSUE_REPORTS.md), [workbooks](QUANTITY_WORKBOOKS.md) and [project status](PROJECT_STATUS.md).

## Delivered in 0.64.0

Grouped/searchable plan navigation completes AEC-015's indexed-set criteria: portable manifests,
stable identities, metadata, fingerprints, visible reference issues, natural sorting, discipline/revision/file
groups and guarded physical-page navigation. Native grouping/navigation, automated persistence and
downloaded-package manifest checks passed. Revision replacement and links are separate cards.

Reviewed transfer of supported native shapes uses explicit page matches and optional bounded alignment,
preserves originals and revised contents, requires each destination preview and writes a new copy.
AEC-016 remains In progress for broader batch processing and unsupported annotation semantics.

Exact-source CI passed 994 Windows tests / 15 ignored and 974 Linux tests /
10 ignored per run. Portable checks covered 44 baselines plus 4 guards;
MSI covered 42 baselines / 210 logs. The eight public assets were verified anonymously;
the downloaded Windows CLI passed 82 commands / 1,062 assertions. Native development-build
checks and independent saved-output inspection retain separate limits in [Project status](PROJECT_STATUS.md).

The [0.63 release](RELEASE_0.63.0.md) retains local manifests, FDF and deducted areas/constant-depth volumes.
AEC-009 remains Done for its stated geometry and calculation criteria. [0.62](RELEASE_0.62.0.md) added
dimensioned rectangles, formula subtotals and explicit historical-measurement recalibration.
An earlier bounded benchmark found a 23 ms optimization/output regression; investigate it without weakening atomic writes.

## Retained 0.67 scope and qualification

AEC-017 adds reviewed links from one indexed source PDF; broader multi-source batch processing remains
open. AEC-014 adds classified raster-difference proposals and editable reviewed outlines. AEC-010 adds
named hierarchical rectangular zones and shared annotation/quantity assignment with explicit unresolved
coverage. All three cards remain **In progress**; native interaction with these combined workflows is untested.

Before 0.67 publication, its development build passed 1,045 tests and three independent corpora
totaling 156 processes / 2,541 assertions (128 SuniPDF CLI plus 28 qpdf). These counts do not describe
a public-runtime run. See [0.67 qualification](RELEASE_0.67.0.md) for that release's separate source,
package and native evidence boundaries.

## Delivered foundation

SuniPDF provides local PDF reading, editing, annotations, forms, OCR, page operations,
measurements, quantities, annotation libraries and revision overlays. See the
[feature matrix](FEATURE_MATRIX.md) for supported operations and platform differences.

The 0.60 plan-workspace implementation is delivered:

- Eligible vector endpoints and straight-line intersections with visible snapping.
- Feet/inches entry and named regions with independent calibrated scales.
- Batch recalibration with before/after preview, atomic application and one Undo.
- Cached page layouts and register results, virtualized tables and adjustable docking.
- Page-by-page search and annotation inventory with progress and cancellation.

These features retain the bounds in [Plan workspace](PLAN_WORKSPACE.md).
Their automated qualification does not close the separate hardware, accessibility,
interoperability or private-plan checks below.

## P1: qualification and release reliability

These are the remaining release-readiness priorities. They need recorded evidence,
not only an implementation or a successful build.

| Priority | Work | Acceptance |
|---|---|---|
| P1 | Independent update authenticity | Sign release metadata and bind exact packages to an embedded public key; reject tampering, expiry and invalid versions; document key custody and rotation. No production key is configured yet. |
| P1 | Windows publisher identity | Sign and timestamp distributed EXE/MSI payloads using a production code-signing certificate, then verify the downloaded files. The certificate remains unavailable. |
| P1 | Native installation and interaction | Exercise installed-MSI and portable updates, unsaved-document prompts, cancellation, rollback, save/recovery dialogs and DPI behavior on physical devices. Keep scripted coverage separate. |
| P1 | Accessibility and input | Check keyboard-only operation, focus, Narrator, forms, measurement tools and tables in the real application. Qualify touch pinch, pen/tablet interaction and native pointer behavior separately. |
| P1 | Physical platform coverage | Run the actual ARM64 package on Windows ARM64; verify physical printers and default OpenGL presentation. Qualify optional wgpu and Ubuntu desktop/GPU combinations separately. |
| P1 | Broader document safety | Extend pinned licensed samples with complex forms, tagged/OCR/Type3 content, damaged and large PDFs; record rendering, save/reopen, source preservation and native memory. Add coverage-guided fuzzing beyond the bounded mutation smoke tests. |
| P1 | Independent interoperability | Record exact reader versions and outcomes for encrypted PDFs, XMP, forms, replies/statuses, ink, FreeText, shapes, redaction and signatures. Structural qpdf checks alone do not establish appearance or trust. |
| P1 | PDF/A and PDF/UA qualification | Select supported profiles, retain independent validator reports and complete applicable human accessibility checks. Preserving identification metadata is not conformance validation. |

Automated qualification passed for 12 pinned licensed fixtures / 42 recorded stages per run, PDF/A-1b, PDF/A-2b and PDF/UA-1; source preservation, specified rewrites and first-page PDFium/Poppler checks. Positive and expected-negative outcomes are retained in [release evidence](qa/RELEASE_0.64.0.json). This does not establish universal conformance, human accessibility, physical-device support or compatibility with private project plans. A later DEB payload inspection on 2026-10-24 independently matched the embedded SuniPDF CLI to both conformance-run hashes. The original run records remain unchanged. No program was executed and no Linux runtime or desktop qualification was added; the desktop hash was recorded only.

Private real-project plan qualification is **deferred at the owner's request**.
Known synthetic geometry and retained licensed/public PDFs remain the automated baseline;
they do not establish survey accuracy or customer-document compatibility.
The complete [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md) remain open until evidenced.

## P2: plan workflow extensions

The [detailed plan backlog](AEC_BACKLOG.md) retains stable capability IDs and acceptance
criteria. Existing features are foundations for these extensions, not unimplemented work.

| Area | Remaining extension | Required behavior |
|---|---|---|
| Register and libraries | Broaden saved-group capture only for reproducible appearances | Native shape-group capture retains relative geometry and supported styles. Foreign, text/image/stamp and extra-semantic records remain explicitly refused. |
| Quantity calculations | Qualify declared units and broaden explicit estimating rules | 0.70 adds optional field units and exact bounded geometry; 0.69 workbook refresh retains separate units and explicit errors. Native desktop/spreadsheet interoperability remains open; currency exchange, taxes and mass/density are not implemented. |
| Dimensioned drawing | Extend supported geometry | Published rectangles use real width/height, an explicit anchor and local/page/region/automatic scale. Preview, Undo/Redo and save/reopen retain editable polygon geometry. Permanent constraints, other analytic shapes and independent X/Y scales remain extensions. |
| Area and volume | Extend independent qualification of the delivered workflow | Polygon deductions and constant-depth quantities are delivered in 0.63. Editable external-reader interoperability and physical accuracy remain separate. |
| Rooms and zones | Complete qualification and broaden explicit region support | The 0.67 release provides named rectangular hierarchies and shared assignments; retain explicit overlap, boundary, unknown and stale coverage. Qualify native interaction and broader plan interoperability. |
| Visual recognition | Symbol search and boundary-based region proposals | Bounded work, candidate previews, explicit tolerance and user confirmation before creating counts or measurements. |
| Revisions | Complete reviewed-outline qualification and broaden alignment | The 0.67 release classifies raster proposals for one explicit page pair; preserve review decisions and original/revised identity. Broader comparison and native qualification remain separate. |
| Plan sets | Broader reviewed revision replacement and link batches | Grouped/searchable navigation is delivered. The 0.67 release links one indexed source PDF after review; multi-source batches and broader replacement retain separate acceptance criteria. |
| Reports | Complete report qualification and assess template extensions | 0.69 provides filtered PDF/CSV/JSON reports, actual supported legends and explicit coverage. Native interaction and remaining reader interoperability remain open; editable report templates are not delivered. |
| Spreadsheet refresh | Complete native desktop/spreadsheet qualification | 0.69 provides XLSX snapshots and reviewed manifest comparisons. Retain explicit changed/missing/ambiguous entries and new outputs; do not import arbitrary workbooks or carry external edits silently. |

Named scale regions, batch recalibration, vector snapping, manual counts, saved-geometry
editing, typed quantity fields, local libraries and aligned overlays are already available.
Each broader backlog card stays partial until its remaining acceptance criteria are met.

## P2: editing, reading and conversion

- **Text and fonts:** extend complex shaping, bidirectional paragraphs, variable/CFF2/color
  fonts and arbitrary editable Form XObjects only with verified render/save/reopen behavior.
  Unsupported inputs must preserve the PDF instead of silently substituting fonts or flattening.
- **Annotations:** assess rich-text spans and foreign rich-text appearances; broaden independent
  reader round trips. Retain the existing uniform-style FreeText, grouped ink and shape tools.
- **Reading order:** improve selection and copying for complex untagged columns; PDFium's
  logical character order does not establish the author's intended reading order.
- **Reader interaction:** evaluate animated navigation and touch interaction after native tests.
- **Visual signatures:** evaluate pen pressure and broaden image-cleanup fixtures. Keep preview
  review because faint ink or textured backgrounds can be misclassified.
- **Office conversion:** extend editable layout fidelity only for explicitly supported inputs
  and an available local converter. General layout reconstruction remains future work.
- **Compression and redaction:** expand large, hostile and licensed-document coverage, including
  native memory measurements and visual checks of raster fallback output.
- **Desktop integration:** assess Explorer integration under a separate implementation and
  installation scope, preserving the user's file associations.

## P2: measured performance and maintenance

A paired development-build benchmark compared 0.64 with 0.67 on six whole-CLI-process workloads,
with one warmup and five measured repetitions per case. Median times increased by 0.5% for document
information, 5.0% for dense-page rendering, 2.6% for search, 4.0% for the 1,500-row register and 3.4%
for 300 quantities with eight formulas. Optimization/output decreased by 0.8%. These bounded
measurements establish no overall speed improvement, GUI responsiveness result or release-package
performance claim. Rendering and table costs need further profiling without weakening source checks.

- Record opening, scrolling, rendering, searching, table interaction and editing on named
  physical devices, including memory, representative file sizes and complete frame latency.
- Isolate PDF processing and atomic output/synchronization costs behind the local optimization/output regression; retain collision and source-preservation guarantees.
- Profile heavy pages before batching adjacent tile renders through a shared loaded page.
  Consider adjustable memory caps only when measurements show a practical need.
- Reduce full-document annotation write/reload work without weakening Undo, permissions,
  encrypted-document preservation, stale-state checks or atomic failure behavior.
- Reduce duplicate CI work only when reused results and packages retain exact source,
  dependency and toolchain provenance. Preserve independent package qualification.
- Keep dependency/license inventories, security scans, source links and upgrade baselines
  current; retain observable failures and the original document in every new operation.

## Later or separately scoped

Archival signature validation and timestamp creation, further certificate integrations,
open CAD/BIM exchange, 3D content, assisted multi-plan matching and confined local automation
need their own design, compatible dependencies and qualification corpus.
A general workflow designer and optional human-readable CLI localization remain lower priority.

## Delivery rule

For each selected lot: define supported inputs and limits, implement and test failure paths,
qualify the frozen source and downloaded packages, then update the feature matrix, release
notes and [current evidence](PROJECT_STATUS.md). Do not close a card because a related
feature shipped. [GitHub Projects](https://github.com/users/lmdanavanem/projects/1) was synchronized on
2026-10-24: five existing P1 cards updated and 27 AEC cards added. The preserved views contain 51 Roadmap,
8 Bugs and 11 Suggestions cards. That is a historical synchronization record, not a fresh readback for 0.69. Partial criteria remain open; later release work must be reconciled against the actual board before claiming it is synchronized.
