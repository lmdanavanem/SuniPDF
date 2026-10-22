# Local PDF plan workflows backlog

[Français](AEC_BACKLOG.fr.md) · [Importable CSV](AEC_BACKLOG.csv)

SuniPDF is a free, open-source local application for PDF plans, measurements and review, licensed under MIT OR Apache-2.0. This backlog defines planned capabilities and acceptance criteria; statuses distinguish partial implementation from qualified functionality.

## Quantity tables and typed fields — 0.46.0

Inspect actual counts and measurements in filtered quantity tables, edit typed fields and explicit unit prices with Undo, and export source-bound CSV/JSON. Totals distinguish missing values from zero, convert compatible price bases and keep currencies separate. Guarded CLI field changes require an expected record and a new PDF copy; copying and annotation permissions remain enforced. AEC-007 remains **In progress**. Arbitrary formulas, tax calculation, currency exchange, automatic symbol recognition, cross-document aggregation and the remaining full estimating acceptance criteria stay open. Angles are visible but excluded from additive quantities and pricing. Packages remain unsigned; native ARM64, accessibility and hardware qualification remain open. [Evidence](qa/RELEASE_0.46.0.json).

## Local annotation libraries — 0.45.0

Save supported local annotation drafts as named templates, combine up to 16 primitives in a flat group, then place a rendered preview at an explicit origin and scale with one Undo step and fresh identities. Strict self-contained JSON supports import, export and removal; CLI placement writes a new PDF copy and preserves encryption and permissions. AEC-006 remains **In progress**. Arbitrary capture from existing PDF annotations, nested groups, measurement/calibration transfer and the remaining complete library acceptance criteria stay open. Packages remain unsigned; native ARM64, accessibility and hardware qualification remain open. [Evidence](qa/RELEASE_0.45.0.json).

## Foundations already present

- Local editing, annotations, threaded comments, review states, OCR, forms, protection, recovery and PDF operations are covered within the [feature matrix](FEATURE_MATRIX.md).
- Local comparison and multi-file text search exist. Geometric plan alignment, automatic revision outlines and symbol search remain separate work.
- Decoration presets exist in 0.39.0. Image Stamp annotations and atomic text-decoration refusals are part of the 0.40.0 source slice; those earlier features did not by themselves provide a local annotation library.

The 0.40.0 local source slice introduced self-contained two-point calibration and five measurement kinds with JSON records through coordinate entry. [Usage and current limits](MEASUREMENTS.md). It provided a foundation for AEC-001/002/003/026; those cards remain open.

## Saved measurement editing delivered in 0.44.0

The published slice, within its documented limits, adds: edit reference distance, unit and precision in the desktop preview workflow, or geometry through the guarded CLI. Replacement preserves identity, replies and register data; existing page defaults and other measurements remain independent. Export permissions, ambiguous target refusals and measured-page resize refusals address audit findings. AEC-001/002/003/026 remain **In progress**; vertex dragging, batch recalculation, multiple scales and real-plan/native qualification remain open. [Scope and audit](RELEASE_0.44.0.md). [Evidence](qa/RELEASE_0.44.0.json).

## Page scale persistence and inspection delivered in 0.43.0

The published 0.43.0 slice adds an explicit saved default for each page, separate from the scale embedded in every measurement. Saving or removing that default participates in Undo and requires saving the PDF; it affects new measurements only. A read-only inspector exposes existing measurement values, units, precision and identity, with navigation and explicit reuse of a non-angle measurement's scale for the current session. CLI commands read, save and remove defaults in new copies and can use a saved scale with physical unit conversion.

Saved defaults require document-modification permission. Bounded, versioned metadata checks page bounds and decoded page content before reuse; crop/resize/content changes can require recalibration. Annotation data remains independent. AEC-001/002/003/026 stay **In progress**: saved-vertex editing, explicit recalculation, multiple scales, vector snapping and real-plan/native qualification remain open. The published slice is verified within its documented limits; partial acceptance criteria remain open. [Evidence](qa/RELEASE_0.43.0.json).

## Measurement integration delivered in 0.42.0

The published 0.42.0 slice makes **Measurements** visible in the main toolbar, **All tools**, and **Add comments**. It adds two-click page calibration with explicit distance confirmation, direct drawing of all five measurement kinds, temporary outlines, rendered preview and Apply, point removal, cancellation and horizontal/vertical constraints. A confirmed scale can be reused on that page during the document session and survives annotation edits and normal tab switches; page geometry/structural changes invalidate it, and closing/reopening a document requires recalibration. Applied annotations retain their own saved calibration. This progress ships in 0.42.0 but does not complete AEC-001/002/003/026. Editing saved vertices, persistent shared scales and recalculation, snapping to PDF vectors, multi-scale viewports and the remaining qualification work are still open. Manual counts and the register retain their documented 0.41.0 status below.

## Priorities and acceptance

The 0.41.0 source slice adds [manual plan counts and an annotation register](MARKUP_REGISTER.md): click-to-place persistent markers, grouped totals linked to actual identities, regrouping, normal move/delete/Undo, all-page search/filters/navigation, separate business fields and CSV/JSON exports. AEC-005 is Done within the documented bounds and qualified in 0.41.0. AEC-004 remains partial: typed fields and normalized dates in 0.46 extend these foundations; custom-key sorting and other acceptance criteria remain open.

P1 is the next useful foundation; P2 builds on that foundation; P3 requires a separate integration or infrastructure lot. Cards remain open until their complete criteria are demonstrated; the CSV records implementation progress separately from qualification. Partial reuse of an existing function does not close a card. Existing [1.0 qualification gates](RELEASE_1.0_CRITERIA.md) remain open independently.

| ID | Priority | Capability | Definition of done | Dependencies |
|---|---|---|---|---|
| AEC-001 | P1 | Page calibration and units | Two-point calibration from a known distance; metric and feet/inches input, visible scale/precision, persistence and Undo. Reject zero/ambiguous scales; verify rotation, CropBox and independent known dimensions. | Geometry baseline |
| AEC-002 | P1 | Plan measurements | Length, polyline, perimeter, polygon area and angle annotations, with editable geometry and explicit calibrated units. Recalibration recalculates values; stale or uncalibrated values never appear as valid takeoffs. | AEC-001 |
| AEC-003 | P1 | Structured quantities | Versioned measurement records link geometry, scale, unit and annotation identity. Reopen/export without changing values silently; preserve originals and report unsupported imported measurement metadata. | AEC-001, AEC-002 |
| AEC-004 | P1 | Annotation register | Sort/filter/search all markups by page, author, date, status, discipline and custom fields; navigate to the actual annotation. Add business statuses without confusing them with standard PDF review states. CSV/JSON exports preserve unknowns and neutralize spreadsheet formulas. | Existing threads |
| AEC-005 | P1 | Manual counting | Place count markers, group by equipment/type, correct/delete with Undo and export totals tied to source markers. Counts remain distinct from proposed visual-search matches. | AEC-003, AEC-004 |
| AEC-006 | P1 | Reusable annotation libraries | Save/reuse/share styles, image stamps, symbols and supported annotation groups in a documented open format. Bound imports, validate assets/licenses and refuse scripts or unsafe references. Decoration presets alone do not satisfy this card. | Existing annotations and presets |
| AEC-007 | P2 | Custom columns and formulas | Typed text/choice/date/number/formula columns, grouped totals and unit-price calculations; bounded expressions without executable code, explicit units/currency/rounding and reproducible exports. Prices are supplied by the user. | AEC-003, AEC-004 |
| AEC-008 | P2 | Multiple scales and dimensioned drawing | Named viewports with independent scales, clear overlap rules and explicit scale selection; draw shapes to real dimensions and support snapping to eligible vectors. | AEC-001, AEC-002 |
| AEC-009 | P2 | Volumes and deducted areas | Explicit depth, cut-outs/holes and retained calculation inputs; unit-safe area/volume totals and tests for degenerate or self-intersecting geometry. | AEC-002, AEC-003 |
| AEC-010 | P2 | Rooms and named zones | Named/hierarchical plan regions, deterministic assignment of annotations to regions and filters/reports by room or floor; expose overlapping/unassigned cases. | AEC-004, AEC-008 |
| AEC-011 | P2 | Visual symbol search | Search from a selected symbol locally, with sensitivity, rotation choices, bounded processing and candidate previews. User-accepted matches drive counts; false positives and partial searches remain explicit. | AEC-005, AEC-026 |
| AEC-012 | P2 | Boundary-based region filling | Preview a bounded region from drawing edges, allow manual closure of openings and retain holes. Convert reviewed results to measurements; document resolution/tolerance and refuse leakage or ambiguous boundaries. | AEC-002, AEC-009, AEC-026 |
| AEC-013 | P2 | Aligned colour overlays | Register two or more plans using explicit control points and display/export colour-separated overlays. Keep input PDFs intact; ambiguous alignment and unmatched pages require review. | Existing comparison, AEC-026 |
| AEC-014 | P2 | Revision outlines and tracked differences | Convert reviewed comparison regions into editable revision outlines and register entries. Preserve original/revised identity and distinguish geometric changes from rendering noise. | Existing comparison, AEC-004, AEC-013 |
| AEC-015 | P2 | Plan sets | Navigate separate PDFs as one indexed set, grouped by discipline/sheet/revision. A portable manifest preserves references; missing, moved and duplicate sheets stay visible. | Labels/bookmarks baseline |
| AEC-016 | P2 | Batch sheet replacement | Match and review incoming revisions, transfer only supported markups under an explicit geometric mapping, retain previous files and produce a per-sheet result report. Refuse unsafe transfers without partial overwrites. | AEC-013, AEC-015 |
| AEC-017 | P2 | Batch cross-document links | Detect sheet references in searchable text/OCR, preview proposed destinations and create reviewed links in new copies. Ambiguous references, unsafe paths and missing files remain unresolved. | AEC-015, existing OCR/links |
| AEC-018 | P2 | Legends and issue reports | Generate legends, grouped quantity/issue summaries and PDF/CSV/JSON reports tied to the actual filtered annotations and document revisions. No invented or stale totals. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Spreadsheet quantity refresh | Export CSV/XLSX snapshots and refresh explicitly identified quantities with source revision and units. Use documented open spreadsheet formats; avoid macros, formula injection and silent spreadsheet overwrites. | AEC-003, AEC-007 |
| AEC-022 | P3 | CAD/BIM and room import | Start with documented open exchanges such as IFC and licensed fixtures. Import identifiable spaces/plan metadata with reviewed mapping. Optional adapters for proprietary formats require separate SDK and licensing checks. | AEC-010, AEC-015 |
| AEC-023 | P3 | 3D PDF workflows | Assess eligible U3D/PRC reading/rendering, annotations, resource isolation and compatible dependencies on licensed fixtures. Keep unsupported 3D content explicit; no automatic script execution. | AEC-026, separate renderer study |
| AEC-024 | P3 | Assisted multi-plan matching | Suggest page pairing and alignment across plan sets, with confidence, unmatched cases and user validation. Benchmark against reviewed examples before automating any replacement. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Local automation | Confined, opt-in access to local files; typed allowlisted commands, bounded resources, explicit write actions and an activity record. | Existing CLI, local permissions |
| AEC-026 | P1 | AEC qualification corpus | Licensed/synthetic plans with known dimensions, revisions, zones and expected counts. Cover unit conversions, four rotations/crops, encrypted PDFs, Undo/atomic failure, independent readers, accessibility and recorded physical-device performance. Synthetic tests do not close hardware or interoperability gaps. | Applies to every card |

## Delivery sequence

1. Extend direct measurement qualification on real plans and native interfaces.
2. Extend page-default and inspector qualification, then extend AEC-001/002 with AEC-026: saved geometry editing, explicit recalculation and remaining real-plan/accessibility checks. Build on the existing quantity records, register and counts.
3. Deliver reusable libraries and takeoff reports; extend overlays, plan sets and revision workflows in independently testable lots.
4. Evaluate advanced recognition, BIM/3D and local automation against demonstrated needs and compatible licenses.

The CSV preserves capability identifiers and is ready for a future Projects import. GitHub Projects edits remain deferred at the owner's request.

[Release evidence](qa/RELEASE_0.42.0.json).
