# Plan workflow backlog

[Roadmap](ROADMAP.md) | [Importable CSV](AEC_BACKLOG.csv) | [Current release](RELEASE_0.60.0.md)

This is the detailed capability backlog for the free, open-source local application. Status refers to the complete acceptance criteria, including qualification; an implemented subset does not close the whole card.

## Delivered in 0.60

Cached page layouts and register views, cooperative search and inventories, docked tables, eligible vector snapping, feet/inches input, named scale regions and atomic batch recalibration are available within the [plan workspace limits](PLAN_WORKSPACE.md). [Qualification records](qa/RELEASE_0.60.0.json) cover the exact published source. Private real-project plans remain deferred; physical devices, accessibility and complete interoperability stay open.

## Capabilities and acceptance

P1 covers foundations and qualification; P2 extends plan workflows; P3 requires a separate integration or research effort. The [1.0 gates](RELEASE_1.0_CRITERIA.md) apply independently.

| ID | Priority | Capability | Status | Acceptance criteria | Dependencies |
|---|---|---|---|---|---|
| AEC-001 | P1 | Page calibration and units | In progress | Two-point calibration from a known distance; metric and feet/inches input, visible scale/precision, persistence and Undo. Reject zero/ambiguous scales; verify rotation, CropBox and independent known dimensions. | Geometry baseline |
| AEC-002 | P1 | Plan measurements | In progress | Length, polyline, perimeter, polygon area and angle annotations, with editable geometry and explicit calibrated units. Recalibration recalculates values; stale or uncalibrated values never appear as valid takeoffs. | AEC-001 |
| AEC-003 | P1 | Structured quantities | In progress | Versioned measurement records link geometry, scale, unit and annotation identity. Reopen/export without changing values silently; preserve originals and report unsupported imported measurement metadata. | AEC-001, AEC-002 |
| AEC-004 | P1 | Annotation register | In progress | Sort/filter/search all markups by page, author, date, status, discipline and custom fields; navigate to the actual annotation. Add business statuses without confusing them with standard PDF review states. CSV/JSON exports preserve unknowns and neutralize spreadsheet formulas. | Existing threads |
| AEC-005 | P1 | Manual counting | Done | Place count markers, group by equipment/type, correct/delete with Undo and export totals tied to source markers. Counts remain distinct from proposed visual-search matches. | AEC-003, AEC-004 |
| AEC-006 | P1 | Reusable annotation libraries | In progress | Save/reuse/share styles, image stamps, symbols and supported annotation groups in a documented open format. Bound imports, validate assets/licenses and refuse scripts or unsafe references. Decoration presets alone do not satisfy this card. | Existing annotations and presets |
| AEC-007 | P2 | Custom columns and formulas | In progress | Typed text/choice/date/number/formula columns, grouped totals and unit-price calculations; bounded expressions without executable code, explicit units/currency/rounding and reproducible exports. Prices are supplied by the user. | AEC-003, AEC-004 |
| AEC-008 | P2 | Multiple scales and dimensioned drawing | In progress | Named viewports with independent scales, clear overlap rules and explicit scale selection; draw shapes to real dimensions and support snapping to eligible vectors. | AEC-001, AEC-002 |
| AEC-009 | P2 | Volumes and deducted areas | Planned | Explicit depth, cut-outs/holes and retained calculation inputs; unit-safe area/volume totals and tests for degenerate or self-intersecting geometry. | AEC-002, AEC-003 |
| AEC-010 | P2 | Rooms and named zones | Planned | Named/hierarchical plan regions, deterministic assignment of annotations to regions and filters/reports by room or floor; expose overlapping/unassigned cases. | AEC-004, AEC-008 |
| AEC-011 | P2 | Visual symbol search | Planned | Search from a selected symbol locally, with sensitivity, rotation choices, bounded processing and candidate previews. User-accepted matches drive counts; false positives and partial searches remain explicit. | AEC-005, AEC-026 |
| AEC-012 | P2 | Boundary-based region filling | Planned | Preview a bounded region from drawing edges, allow manual closure of openings and retain holes. Convert reviewed results to measurements; document resolution/tolerance and refuse leakage or ambiguous boundaries. | AEC-002, AEC-009, AEC-026 |
| AEC-013 | P2 | Aligned colour overlays | In progress | Register two or more plans using explicit control points and display/export colour-separated overlays. Keep input PDFs intact; ambiguous alignment and unmatched pages require review. | Existing comparison, AEC-026 |
| AEC-014 | P2 | Revision outlines and tracked differences | Planned | Convert reviewed comparison regions into editable revision outlines and register entries. Preserve original/revised identity and distinguish geometric changes from rendering noise. | Existing comparison, AEC-004, AEC-013 |
| AEC-015 | P2 | Plan sets | Planned | Navigate separate PDFs as one indexed set, grouped by discipline/sheet/revision. A portable manifest preserves references; missing, moved and duplicate sheets stay visible. | Labels/bookmarks baseline |
| AEC-016 | P2 | Batch sheet replacement | Planned | Match and review incoming revisions, transfer only supported markups under an explicit geometric mapping, retain previous files and produce a per-sheet result report. Refuse unsafe transfers without partial overwrites. | AEC-013, AEC-015 |
| AEC-017 | P2 | Batch cross-document links | Planned | Detect sheet references in searchable text/OCR, preview proposed destinations and create reviewed links in new copies. Ambiguous references, unsafe paths and missing files remain unresolved. | AEC-015, existing OCR/links |
| AEC-018 | P2 | Legends and issue reports | Planned | Generate legends, grouped quantity/issue summaries and PDF/CSV/JSON reports tied to the actual filtered annotations and document revisions. No invented or stale totals. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Spreadsheet quantity refresh | Planned | Export CSV/XLSX snapshots and refresh explicitly identified quantities with source revision and units. Use documented open spreadsheet formats; avoid macros, formula injection and silent spreadsheet overwrites. | AEC-003, AEC-007 |
| AEC-022 | P3 | CAD/BIM and room import | Planned | Start with documented open exchanges such as IFC and licensed fixtures. Import identifiable spaces/plan metadata with reviewed mapping. Optional adapters for proprietary formats require separate SDK and licensing checks. | AEC-010, AEC-015 |
| AEC-023 | P3 | 3D PDF workflows | Planned | Assess eligible U3D/PRC reading/rendering, annotations, resource isolation and compatible dependencies on licensed fixtures. Keep unsupported 3D content explicit; no automatic script execution. | AEC-026, separate renderer study |
| AEC-024 | P3 | Assisted multi-plan matching | Planned | Suggest page pairing and alignment across plan sets, with confidence, unmatched cases and user validation. Benchmark against reviewed examples before automating any replacement. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Local automation | Planned | Confined, opt-in access to local files; typed allowlisted commands, bounded resources, explicit write actions and an activity record. | Existing CLI, local permissions |
| AEC-026 | P1 | AEC qualification corpus | In progress | Licensed/synthetic plans with known dimensions, revisions, zones and expected counts. Cover unit conversions, four rotations/crops, encrypted PDFs, Undo/atomic failure, independent readers, accessibility and recorded physical-device performance. Synthetic tests do not close hardware or interoperability gaps. | Applies to every card |
| AEC-027 | P1 | Large-document layout reuse | In progress | Cache page/spread layout by document revision and geometry; scrolling and repaint reuse positions; record benchmark corpus and machine. | Existing desktop workspace |
| AEC-028 | P1 | Responsive search and annotation inventory | In progress | Advance long reads page by page with progress, cancellation and visible-page rendering between tasks; do not publish stale partial inventories as complete. | Existing desktop workspace |
| AEC-029 | P1 | Consistent plan workspace | In progress | Dock or float resizable annotation/quantity tables, persist layout preferences and navigate to actual selected annotations; retain context and small-window access. | Existing desktop workspace |

Implementation details and progress are retained in the CSV. Current delivery priorities are maintained only in the [roadmap](ROADMAP.md). GitHub Projects synchronization remains deferred.
