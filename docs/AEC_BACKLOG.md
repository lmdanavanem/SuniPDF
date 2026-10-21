# Construction PDF and enterprise collaboration backlog

[Français](AEC_BACKLOG.fr.md) · [Importable CSV](AEC_BACKLOG.csv) · [Cloud delivery plan](CLOUD_STRATEGY.md)

Owner direction, 21 October 2026: develop SuniPDF into a free, open-source local alternative for construction PDF workflows, with a paid managed cloud for companies working on shared PDFs. Pricing is undecided. Existing source licensing remains MIT OR Apache-2.0; charging for hosting does not introduce a local account requirement or restrict local PDF tools.

This is a product backlog, not a claim of feature parity, Bluebeam format compatibility or completed qualification. Bluebeam feature names below identify the requested reference workflows. SuniPDF implementations, names and exchange formats must be independently designed.

## Foundations already present

- Local editing, annotations, threaded comments, review states, OCR, forms, protection, recovery and PDF operations are covered within the [feature matrix](FEATURE_MATRIX.md).
- Local comparison and multi-file text search exist. Geometric plan alignment, automatic revision clouds and symbol search remain separate work.
- Decoration presets exist in 0.39.0. Image Stamp annotations and atomic text-decoration refusals are part of the 0.40.0 source slice; these do not constitute a shared annotation library.
- Cloud and collaboration are deferred at the owner's request. The cloud plan records future requirements only; no service is included in 0.40.0.

The 0.40.0 local source slice also adds self-contained two-point calibration and five measurement kinds with JSON records. [Usage and current limits](MEASUREMENTS.md). This is partial progress on AEC-001/002/003/026: coordinate entry and per-annotation scales do not complete page-wide recalibration, direct manipulation, viewport scales or the full qualification corpus. Those cards remain open.

## Priorities and acceptance

The 0.41.0 source slice adds [manual plan counts and an annotation register](MARKUP_REGISTER.md): click-to-place persistent markers, grouped totals linked to actual identities, regrouping, normal move/delete/Undo, all-page search/filters/navigation, separate business fields and CSV/JSON exports. AEC-005 is implemented within the documented limits, pending release qualification. AEC-004 remains partial: custom-key sorting, typed columns and normalized dates remain open. Cloud and collaboration remain deferred.

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
| AEC-014 | P2 | Revision clouds and tracked differences | Convert reviewed comparison regions into editable cloud annotations and register entries. Preserve original/revised identity and distinguish geometric changes from rendering noise. | Existing comparison, AEC-004, AEC-013 |
| AEC-015 | P2 | Plan sets | Navigate separate PDFs as one indexed set, grouped by discipline/sheet/revision. A portable manifest preserves references; missing, moved and duplicate sheets stay visible. | Labels/bookmarks baseline |
| AEC-016 | P2 | Batch sheet replacement | Match and review incoming revisions, transfer only supported markups under an explicit geometric mapping, retain previous files and produce a per-sheet result report. Refuse unsafe transfers without partial overwrites. | AEC-013, AEC-015 |
| AEC-017 | P2 | Batch cross-document links | Detect sheet references in searchable text/OCR, preview proposed destinations and create reviewed links in new copies. Ambiguous references, unsafe paths and missing files remain unresolved. | AEC-015, existing OCR/links |
| AEC-018 | P2 | Legends and issue reports | Generate legends, grouped quantity/issue summaries and PDF/CSV/JSON reports tied to the actual filtered annotations and document revisions. No invented or stale totals. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Spreadsheet quantity refresh | Export CSV/XLSX snapshots and refresh explicitly identified quantities with source revision and units. Work with open formats without requiring paid Excel; avoid macros, formula injection and silent spreadsheet overwrites. | AEC-003, AEC-007 |
| AEC-020 | P3 | Live shared review sessions | Multiple authenticated colleagues annotate the same immutable base PDF, with author identity, event history, reconnect recovery, conflict handling and tested multi-client permissions. A document reservation is not simultaneous annotation. | AEC-003, AEC-004, AEC-021 |
| AEC-021 | P3 | Enterprise cloud projects | Isolated companies, viewer/editor/admin roles, shared projects, immutable PDF versions, edit reservations, guarded publication and previous-version downloads. Optional desktop access; company data stays exportable. Paid managed hosting, prices later. Production SSO, backups, deployment and operations need separate evidence. | [Cloud plan](CLOUD_STRATEGY.md) |
| AEC-022 | P3 | CAD/BIM and room import | Start with documented open exchanges such as IFC and licensed fixtures. Import identifiable spaces/plan metadata with reviewed mapping. Revit/AutoCAD adapters are optional and require their own SDK and licensing checks. | AEC-010, AEC-015 |
| AEC-023 | P3 | 3D PDF workflows | Assess eligible U3D/PRC reading/rendering, annotations, resource isolation and compatible dependencies on licensed fixtures. Keep unsupported 3D content explicit; no automatic script execution. | AEC-026, separate renderer study |
| AEC-024 | P3 | Assisted multi-plan matching | Suggest page pairing and alignment across plan sets, with confidence, unmatched cases and user validation. Benchmark against reviewed examples before automating any replacement. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Optional automation/MCP | Confined, opt-in local/project access; typed allowlisted tools, bounded resources, explicit write actions and an activity record. No required commercial AI provider or cloud account for local use. | Existing CLI, enterprise permissions |
| AEC-026 | P1 | AEC qualification corpus | Licensed/synthetic plans with known dimensions, revisions, zones and expected counts. Cover unit conversions, four rotations/crops, encrypted PDFs, Undo/atomic failure, independent readers, accessibility and recorded physical-device performance. Synthetic tests do not close hardware or interoperability gaps. | Applies to every card |

## Delivery sequence

1. Qualify the 0.40.0 local tools first. Cloud, collaboration, hosting and billing are deferred at the owner's request.
2. Build AEC-001/002 with AEC-026, then quantity records, annotation register and counting (AEC-003/004/005).
3. Deliver reusable libraries and takeoff reports; extend overlays, plan sets and revision workflows in independently testable lots.
4. Revisit enterprise cloud and live review only after the local feature priorities. Identity, versioning, conflicts, tenant isolation and operations will need a separate implementation and qualification.
5. Evaluate advanced recognition, BIM/3D and optional automation against real demand and compatible licenses.

No prices, delivery dates or complete replacement claim are implied. This CSV is ready for a future Projects import; GitHub Projects edits remain deferred at the owner's request.

## Reference workflows

The user brief and Bluebeam's official documentation informed the capability list: [scales and viewports](https://support.bluebeam.com/user-manual/menus/window/create-manage-viewports.html), [custom columns](https://support.bluebeam.com/kr/se/user-manual/menus/window/custom-columns.html), [region filling](https://support.bluebeam.com/revu/features/dynamic-fill-overview.html), [overlays](https://support.bluebeam.com/online-help/revu20/Content/RevuHelp/Menus/Document/Overlay/Overlay-Pages--MT.htm), [sheet replacement](https://support.bluebeam.com/revu/how-to/transfer-markups-with-batch-slip-sheet.html), [batch links](https://support.bluebeam.com/revu/how-to/tips-and-tricks/manage-pdfs-efficiently-with-batch-link.html), and [shared sessions versus versioned projects](https://support.bluebeam.com/revu/subscription/studio-and-bluebeam-cloud-comparison-guide.html). These describe reference workflows; they do not grant compatibility or qualification to SuniPDF.
