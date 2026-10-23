# Roadmap

Updated for [0.60.0](RELEASE_0.60.0.md), published on 2026-10-23.
This page is the current work list. Release history belongs in the
[changelog](../CHANGELOG.md) and [status archive](archive/PROJECT_STATUS_PRE_0.60.md).
No future release date or version is committed here.

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

Private real-project plan qualification is **deferred at the owner's request**.
Known synthetic geometry and retained licensed/public PDFs remain the automated baseline;
they do not establish survey accuracy or customer-document compatibility.
The complete [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md) remain open until evidenced.

## P2: plan workflow extensions

The [detailed plan backlog](AEC_BACKLOG.md) retains stable capability IDs and acceptance
criteria. Existing features are foundations for these extensions, not unimplemented work.

| Area | Remaining extension | Required behavior |
|---|---|---|
| Register and libraries | Complete custom-field sorting, supported annotation-group capture and reuse | Keep stable identities, bounds and explicit import refusals; preserve original annotations. |
| Quantity calculations | Formula columns, unit-price calculations and spreadsheet refresh | Bounded non-executable expressions, explicit units/currency/rounding and exports tied to the source revision. |
| Dimensioned drawing | Draw supported shapes from real dimensions | Use the existing scale regions and snapping; show the selected scale and retain editable geometry. |
| Area and volume | Depth, deducted areas and retained holes | Unit-safe calculations; reject degenerate or self-intersecting geometry and retain all inputs. |
| Rooms and zones | Named hierarchical regions and assignment rules | Expose overlapping and unassigned annotations; filter and report by room or floor. |
| Visual recognition | Symbol search and boundary-based region proposals | Bounded work, candidate previews, explicit tolerance and user confirmation before creating counts or measurements. |
| Revisions | Reviewed change outlines and broader alignment workflows | Distinguish drawing changes from rendering noise and retain original/revised identities. |
| Plan sets | Indexed sheets, batch replacement and cross-document links | Preview matches; retain previous files; report missing, duplicate or ambiguous references and refuse unsafe transfers. |
| Reports | Legends, quantity summaries and issue reports | Generate from actual filtered annotations and document revisions, with no invented or stale totals. |

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

- Record opening, scrolling, rendering, searching, table interaction and editing on named
  physical devices, including memory, representative file sizes and complete frame latency.
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
feature shipped. GitHub Projects updates remain deferred; repository documentation is the
current record until those tables are synchronized.
