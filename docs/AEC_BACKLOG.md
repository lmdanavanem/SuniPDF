# Plan workflow backlog

[Roadmap](ROADMAP.md) | [Importable CSV](AEC_BACKLOG.csv) | [0.71 release scope](RELEASE_0.71.0.md) · Published as a stable release.

This is the detailed capability backlog for the free, open-source local application. Status refers to the complete acceptance criteria, including qualification; an implemented subset does not close the whole card.

## Published visual symbol search

AEC-011 is **In progress**. The 0.71 release includes template selection, explicit
page/rotation/similarity settings, cancellable candidate review and an atomic batch
of accepted count markers. Windows: 1,114 passed, 15 ignored; Linux: 1,095 passed, 10 ignored; zero failures. 7,409 assertions across 493 recorded commands in ten downloaded-package corpora. See [qualification evidence](qa/RELEASE_0.71.0.json).
Native interaction and broader drawing qualification remain open.
[Workflow and limits](VISUAL_SEARCH.md).

## Current region-fill development

AEC-012 is **In progress** for the 0.72 region-fill increment: bounded raster filling,
explicit manual closures, retained holes and reviewed calibrated area creation.
Leakage, ambiguous topology and incomplete processing prevent application.
[Workflow and limits](REGION_FILL.md). Qualification remains in progress.

## 0.70 increment and remaining qualification

Declare optional units on Number custom fields and retain them in the table, formula previews,
PDF/CSV/JSON reports and XLSX refreshes. Columns with the same name require one type and unit.
Changing a unit changes its declaration without converting the value or calibrating a measurement.

Checked decimal formulas combine lengths, areas and volumes within one exact unit family:
`2 m * 3 m = 6 m2`, then `6 m2 * 0.25 m = 1.5 m3`. Mixed families, inverse dimensions,
powers above three, mass and density remain unsupported. Missing values do not become zero.

Version 2 XLSX Changes sheets expose typed **Before custom fields** and **After custom fields**
as inert text. Absent comparison sides stay empty. Unitless version 1 data keeps its serialization,
comparison hashes and original workbook columns. Unit-bearing data uses version 2; unsupported
versions are refused rather than treated as dimensionless data.

AEC-007, AEC-018 and AEC-019 remain **In progress**. Native desktop interaction, native spreadsheet interoperability and broader reader qualification remain open. Packages remain unsigned; ARM64 hardware, physical printing, human accessibility and private project plans retain their separate limits. No overall performance improvement, universal interoperability or PDF/A/PDF/UA conformance is claimed.

Published as a stable release. Package evidence: Windows: 1,094 passed, 15 ignored; Linux: 1,075 passed, 10 ignored; zero failures. 7,019 assertions in nine downloaded-package corpora. See [qualification evidence](qa/RELEASE_0.70.0.json). [0.70 release scope](RELEASE_0.70.0.md).

## Delivered in 0.60

Cached page layouts and register views, cooperative search and inventories, docked tables, eligible vector snapping, feet/inches input, named scale regions and atomic batch recalibration are available within the [plan workspace limits](PLAN_WORKSPACE.md). [Qualification records](qa/RELEASE_0.60.0.json) cover the exact published source. Private real-project plans remain deferred; physical devices, accessibility and complete interoperability stay open.

## Delivered in 0.61

Custom-field sorting, supported saved native shape groups and bounded report formulas are available.
[Release notes](RELEASE_0.61.0.md) describe supported inputs and refusals. Automated qualification passed for 12 pinned licensed fixtures / 42 recorded stages per run, PDF/A-1b, PDF/A-2b and PDF/UA-1; source preservation, specified rewrites and first-page PDFium/Poppler checks. Positive and expected-negative outcomes are retained in [release evidence](qa/RELEASE_0.61.0.json). This does not establish universal conformance, human accessibility, physical-device support or compatibility with private project plans.

Full cards retain their broader acceptance gates; no entire P1/P2 category is closed.

## Delivered in 0.62

AEC-007 includes formula subtotals with checked sums of rounded row results, separate formula/group/unit/precision buckets and explicit error coverage. Signed numeric CSV values remain numeric; free text retains spreadsheet-formula protection. At 0.62, unit-bearing custom fields and tax/currency conversion remained open. 0.70 adds declared field units; tax/currency conversion and the remaining qualification keep the card **In progress**. The later 0.69 workbook increment adds reviewed manifest refresh under AEC-019; its native spreadsheet qualification remains open.

AEC-008 includes a shared dimensioned-rectangle constructor, CLI scale selection and a Measurements-panel placement workflow. The result is an editable four-vertex area measurement with current provenance and quantity-table support. Explicit page/region/reference selection, ambiguity refusal and storage-precision checks preserve the requested dimensions. Exact-source CI passed; earlier native development-build checks covered Preview/Apply, Undo/Redo and save/reopen. AEC-008 stays **In progress**: permanent rectangular constraints, arbitrary rotated shapes, dimensioned circles, independent X/Y scales and broader plan interoperability remain open. AEC-009 depth, holes and volumes are outside 0.62.

[Release evidence](qa/RELEASE_0.62.0.json) records verified public packages and the published Windows x64 CLI run: 48 process commands / 288 assertions. This post-download run did not repeat GUI or local installation testing. Physical-device and broader interoperability gates remain separate.

## Delivered in 0.63

AEC-009 is **Done for its stated polygon deductions and constant-depth calculation criteria**. Core,
CLI and desktop retain contours and physical depth, show gross/deducted/net results and support cubic
quantity units. Invalid topology and incompatible prices refuse atomically. Source tests, native
development-build workflows and public-package checks passed; independent native-volume output inspection
passed 65 assertions. Editable measurement exchange with external readers and physical survey accuracy
remain separate unqualified scopes. Variable-depth solids are not requirements of this card.

AEC-015 includes local manifests for selected PDFs, stable identities, metadata, source fingerprints,
validation, refresh/relink, new-copy export and guarded navigation. Source PDFs and unsaved tabs are
preserved. Grouped/searchable navigation was not part of 0.63; the qualified 0.64 increment below
completes these indexed-set criteria. Revision replacement and links remain separate cards.

[Release evidence](qa/RELEASE_0.63.0.json) records 963 Windows tests / 15 ignored, 943 Linux tests /
10 ignored per run, qualified packages and eight verified public downloads. The downloaded Windows CLI
passed 68 commands / 968 assertions, including the 20-command / 680-assertion plan/form/volume corpus.
[Release notes](RELEASE_0.63.0.md) preserve native, installer, hardware and interoperability limits.

## Delivered in 0.64

AEC-015 is **Done for its stated indexed plan-set criteria**. Filtering, natural sorting and groups by
discipline/revision/file retain stable IDs and physical-page navigation. Native group/navigation checks,
automated manifest persistence and downloaded-package reference checks passed. Missing, changed and
duplicate references remain explicit. This does not close the separate revision-replacement or link cards.

AEC-016 includes reviewed transfer of supported native shapes between explicit page pairs, real previews,
preserved originals and new-copy output. It remains **In progress** for broader batch scope. Unsupported
semantic annotations are excluded rather than silently copied.

[Release evidence](qa/RELEASE_0.64.0.json) records exact-source CI, eight verified downloads and
82 commands / 1,062 assertions on the downloaded Windows CLI. Native development-build output
inspection passed 27 independent assertions. See [qualification limits](RELEASE_0.64.0.md).

## Retained from 0.67

AEC-017 provides reviewed links from one indexed source PDF; broader multi-source batches remain
open. AEC-014 provides classified raster proposals and reviewed outlines for one old/new page pair.
AEC-010 provides named rectangular zone hierarchies and shared annotation/quantity coverage. All
three remain **In progress**, with native interaction and broader qualification still open.

## Added in 0.69

AEC-018 adds shared filtered PDF/CSV/JSON reports, actual paginated previews, readable quantity/cost
summaries, issue groups and supported style legends. A technical appendix retains full provenance
and settings. AEC-019 adds XLSX snapshots and reviewed refresh from a separate manifest, preserving
missing/ambiguous baseline entries and recalculating current values from the PDF.

Both cards remain **In progress**. Native desktop interaction, native spreadsheet interoperability
and remaining report-reader qualification are untested or open. The report font/bounds limits and
the workbook's no-arbitrary-XLSX-import/no-external-edit-preservation limits remain explicit.
Windows: 1,083 passed, 15 ignored; Linux: 1,064 passed, 10 ignored. Eight independent package corpora passed 5,027 assertions across 326 commands. [Qualification evidence](qa/RELEASE_0.69.0.json). No entire P1/P2 category is closed.

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
| AEC-009 | P2 | Volumes and deducted areas | Done | Explicit depth, cut-outs/holes and retained calculation inputs; unit-safe area/volume totals and tests for degenerate or self-intersecting geometry. | AEC-002, AEC-003 |
| AEC-010 | P2 | Rooms and named zones | In progress | Named/hierarchical plan regions, deterministic assignment of annotations to regions and filters/reports by room or floor; expose overlapping/unassigned cases. | AEC-004, AEC-008 |
| AEC-011 | P2 | Visual symbol search | In progress | Search from a selected symbol locally, with sensitivity, rotation choices, bounded processing and candidate previews. User-accepted matches drive counts; false positives and partial searches remain explicit. | AEC-005, AEC-026 |
| AEC-012 | P2 | Boundary-based region filling | In progress | Preview a bounded region from drawing edges, allow manual closure of openings and retain holes. Convert reviewed results to measurements; document resolution/tolerance and refuse leakage or ambiguous boundaries. | AEC-002, AEC-009, AEC-026 |
| AEC-013 | P2 | Aligned colour overlays | In progress | Register two or more plans using explicit control points and display/export colour-separated overlays. Keep input PDFs intact; ambiguous alignment and unmatched pages require review. | Existing comparison, AEC-026 |
| AEC-014 | P2 | Revision outlines and tracked differences | In progress | Convert reviewed comparison regions into editable revision outlines and register entries. Preserve original/revised identity and distinguish geometric changes from rendering noise. | Existing comparison, AEC-004, AEC-013 |
| AEC-015 | P2 | Plan sets | Done | Navigate separate PDFs as one indexed set, grouped by discipline/sheet/revision. A portable manifest preserves references; missing, moved and duplicate sheets stay visible. | Labels/bookmarks baseline |
| AEC-016 | P2 | Batch sheet replacement | In progress | Match and review incoming revisions, transfer only supported markups under an explicit geometric mapping, retain previous files and produce a per-sheet result report. Refuse unsafe transfers without partial overwrites. | AEC-013, AEC-015 |
| AEC-017 | P2 | Batch cross-document links | In progress | Detect sheet references in searchable text/OCR, preview proposed destinations and create reviewed links in new copies. Ambiguous references, unsafe paths and missing files remain unresolved. | AEC-015, existing OCR/links |
| AEC-018 | P2 | Legends and issue reports | In progress | Generate legends, grouped quantity/issue summaries and PDF/CSV/JSON reports tied to the actual filtered annotations and document revisions. No invented or stale totals. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Spreadsheet quantity refresh | In progress | Export CSV/XLSX snapshots and refresh explicitly identified quantities with source revision and units. Use documented open spreadsheet formats; avoid macros, formula injection and silent spreadsheet overwrites. | AEC-003, AEC-007 |
| AEC-022 | P3 | CAD/BIM and room import | Planned | Start with documented open exchanges such as IFC and licensed fixtures. Import identifiable spaces/plan metadata with reviewed mapping. Optional adapters for proprietary formats require separate SDK and licensing checks. | AEC-010, AEC-015 |
| AEC-023 | P3 | 3D PDF workflows | Planned | Assess eligible U3D/PRC reading/rendering, annotations, resource isolation and compatible dependencies on licensed fixtures. Keep unsupported 3D content explicit; no automatic script execution. | AEC-026, separate renderer study |
| AEC-024 | P3 | Assisted multi-plan matching | Planned | Suggest page pairing and alignment across plan sets, with confidence, unmatched cases and user validation. Benchmark against reviewed examples before automating any replacement. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Local automation | Planned | Confined, opt-in access to local files; typed allowlisted commands, bounded resources, explicit write actions and an activity record. | Existing CLI, local permissions |
| AEC-026 | P1 | AEC qualification corpus | In progress | Licensed/synthetic plans with known dimensions, revisions, zones and expected counts. Cover unit conversions, four rotations/crops, encrypted PDFs, Undo/atomic failure, independent readers, accessibility and recorded physical-device performance. Synthetic tests do not close hardware or interoperability gaps. | Applies to every card |
| AEC-027 | P1 | Large-document layout reuse | In progress | Cache page/spread layout by document revision and geometry; scrolling and repaint reuse positions; record benchmark corpus and machine. | Existing desktop workspace |
| AEC-028 | P1 | Responsive search and annotation inventory | In progress | Advance long reads page by page with progress, cancellation and visible-page rendering between tasks; do not publish stale partial inventories as complete. | Existing desktop workspace |
| AEC-029 | P1 | Consistent plan workspace | In progress | Dock or float resizable annotation/quantity tables, persist layout preferences and navigate to actual selected annotations; retain context and small-window access. | Existing desktop workspace |

Implementation details and progress are retained in the CSV. Current delivery priorities are maintained only in the [roadmap](ROADMAP.md). GitHub Projects synchronization requires a fresh board readback and is not currently verified. Full card acceptance remains distinct from an implemented subset.
