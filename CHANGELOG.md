# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Releases are `vX.Y.Z` tags;
`release.yml` builds draft release assets (Windows portable x64/ARM64 ZIP, x64 MSI, Ubuntu amd64 DEB); publication follows exact-source and package qualification.

## [Unreleased]

Further work remains in the [roadmap](docs/ROADMAP.md).

## [0.69.0] - 2026-10-24

- Prepare readable PDF, CSV and JSON reports from the exact annotation-register or quantity-table selection. Show quantity and cost summaries, coverage, issue groups and actual style legends, with complete source details in a separate technical appendix. See the [report guide](docs/ISSUE_REPORTS.md).
- Export quantity workbooks with structured rows, totals, changes and provenance. Review refreshes from a separate manifest using stable annotation identities, preserving missing and ambiguous records for later review. See the [workbook guide](docs/QUANTITY_WORKBOOKS.md).

The report and workbook increments are combined in this release; 0.68 was an intermediate
development build. Windows: 1,083 passed, 15 ignored; Linux: 1,064 passed, 10 ignored. Eight independent package corpora passed 5,027 assertions across 326 commands. [Qualification evidence](docs/qa/RELEASE_0.69.0.json).

Native desktop interaction and native spreadsheet interoperability remain untested. AEC-018 and
AEC-019 remain In progress. Packages are unsigned, and the existing ARM64, physical-device,
accessibility and private-plan qualification limits remain open.

[Release notes and qualification limits](docs/RELEASE_0.69.0.md).

## [0.67.0] - 2026-10-24

- Review exact searchable sheet references against indexed plan destinations and create relative PDF links in a new copy, with ambiguous/missing targets and changed sources refused explicitly.
- Classify bounded raster differences between an explicit old/new page pair, review each selected decision and write editable revision outlines with source/region provenance in a new PDF copy.
- Define named rectangular page zones with parent zones, preview deterministic annotation assignments and use shared register/quantity filters with visible boundary, overlap, unknown and stale coverage.
- Keep zone filters in the compact Filters menu and group coverage and quantities under Totals.

Published as stable Latest after exact-source CI and package qualification. Windows recorded 1,045
passed / 15 ignored; Linux recorded 1,026 passed / 10 ignored. Portable upgrade checks cover 45
baselines plus four guards; MSI covers 43 baselines / 215 logs. All eight public assets were verified
anonymously; the downloaded Windows x64 package passed 238 recorded commands / 3,603 assertions.
Native interaction with the new
0.67 plan workflows remains untested. [Release notes and qualification limits](docs/RELEASE_0.67.0.md).

## [0.64.0] - 2026-10-24

- Transfer explicitly selected supported native annotations between confirmed revision pages, with real destination previews, fresh identities, source fingerprints and new-copy output.
- Filter and naturally sort indexed sheets; group by discipline, revision or source file without changing manifest order or physical-page identity.
- Preserve revised PDF content, supported encryption and original files; refuse stale sources, unsafe geometry, unsupported semantic records and output collisions.
- Add published 0.63.0 portable and MSI packages to upgrade regression baselines.

[Release notes and completed qualification](docs/RELEASE_0.64.0.md).

## [0.63.0] - 2026-10-24

Published as stable Latest after exact-source CI and package qualification. Windows: 963 passed / 15 ignored; Linux: 943 passed / 10 ignored. All eight public assets were verified anonymously; the downloaded x64 CLI passed 68 commands / 968 assertions. Portable upgrades cover 43 baselines plus 4 guards; MSI covers 41 baselines / 205 logs. Native development-build checks have their own documented scope.

- Add local plan-set manifests with stable sheet identities, editable metadata, source fingerprints, explicit validation and guarded sheet navigation. Create, refresh, relink and save new manifest copies through the desktop or CLI without modifying source PDFs; preserve unsaved tabs and use passwords only for the requested operation.
- Export supported form values as FDF and retain typed button appearance-state semantics during import. Bound serialized and expanded input, and refuse ambiguous or unsupported structures before writing.
- Add polygon area deductions and constant-depth volume measurements, with selectable contours, explicit depth units, gross/deducted/net summaries and guarded Preview/Apply/Undo. Preserve all rings and physical depth through editing, recalibration and reopening.
- Add cubic quantity units and dimension-safe pricing. Keep historical quantities inactive, reject incompatible prices atomically, and select measurement scales from the outer boundary without joining separate rings.
- Preserve button appearance states when displaying forms requesting refresh and when importing values repeatedly. Keep appearance writes transactional and preserve signed or restricted source files during display.
- Keep thin measurement contours visible in small preview images and bring volume/deduction controls closer together.

[Release notes and qualification limits](docs/RELEASE_0.63.0.md).

## [0.62.0] - 2026-10-24

Published as stable Latest with exact-source CI, Windows/Ubuntu packages, upgrade checks and
anonymous verification of all eight public assets. See the release notes for qualification limits.

- Create a dimensioned rectangle from a displayed-page anchor and real width/height, using local, page, region or automatic calibration. Preview its four-vertex area, apply with one Undo step, save and reopen; the CLI shares the same checks.

- Add formula subtotals for all filtered rows, grouped by formula, selected group, result unit and precision. Sum the rounded row results with checked arithmetic; retain explicit coverage and error counts without inventing a zero when every row fails.
- Evaluate formula cells once per table refresh or export and reuse them for summaries. Keep typed signed values numeric in CSV while neutralizing spreadsheet formulas in free text.
- Read document permissions once when building a quantity table and access each annotation without cloning the entire page annotation array. Keep fresh permission and source checks when exporting or applying edits.
- Preserve version 1 quantity coverage fields and add separate additive, angular and excluded-row counts.
- Bind new saved measurements to the current page bounds, user unit, decoded content, reachable resources and initial rendering state. Show legacy or changed-page records as historical; retain their points and values while excluding them from usable quantities, costs and quantity-dependent formulas.
- Require an explicit current reference and Preview/Apply to recalibrate historical lengths and areas; recreate historical angles. Unit, precision, price or custom-field edits cannot restore page provenance. Strengthen saved page scales and named regions, requiring older references to be recalibrated or removed.
- Reject measurement edits and batch recalibration atomically if the resulting dimension or quantity cannot use the preserved unit price, including pricing overflow. Compatible unit conversions retain the stored price and its explicit price basis.
- Enforce the CLI's no-overwrite policy at final output creation, including files created while processing; retain command-specific source protections and explicit replacement rules.
- Make document restoration transactional, retaining the current document and password when loading or refreshing the replacement fails. Refuse certificate signing of encrypted PDFs, including files automatically unlocked with an empty user password.
- Retain unsaved tab bytes, Undo/Redo history and recovery identity after a failed tab resume, allowing retry. Reload cancelled register work and invalidate obsolete search results and measurement caches.
- Synchronize page selection, labels, navigation history and scroll destination when an edit completes during tab suspension, including the suspension-failure path.
- Close measurement editors after successful Apply and keep drafts available after refusal. Keep strong text, active controls and historical-value warnings readable in light and dark themes; format quantity cells without exposing floating-point noise while retaining raw values in tooltips and exports.
- Add the published 0.61.0 portable and MSI packages to the upgrade-regression baselines.

[Release notes and qualification scope](docs/RELEASE_0.62.0.md).

## [0.61.0] - 2026-10-23

- Sort register custom text fields and typed quantity fields, keeping missing values last.
- Capture supported saved shape annotations as reusable groups with explicit source attribution, bounded appearance checks and unchanged originals.
- Calculate bounded quantity report formulas with explicit units, rounding and per-cell errors; reuse JSON configurations and export source-bound CSV/JSON results.
- Add pinned PDF/A-1b, PDF/A-2b and PDF/UA-1 positive/negative fixtures with independent veraPDF and Poppler checks. Validation applies to the recorded files and operations.
- Preserve the compression state of XML metadata during lossless optimization, fixing the PDF/A-1 regression exposed by the independent corpus.

[Release notes](docs/RELEASE_0.61.0.md) · [Evidence](docs/qa/RELEASE_0.61.0.json) · [Conformance gate](docs/CONFORMANCE.md).

- Maintain one English documentation set, consolidate current priorities in the roadmap and archive earlier project-status records. Packages include the English user guide; application translations and OCR remain available in English and French.

## [0.60.0] - 2026-10-23

0.60 implements cached page layouts and register views, cooperative page-by-page search/inventory, dockable tables, vector snapping, feet/inches entry, named independent scale regions and atomic batch recalibration with preview and Undo. Private real-project plan qualification is deferred at the owner's request; the remaining 1.0 qualification gates stay open.

[Release notes](docs/RELEASE_0.60.0.md) · [Evidence](docs/qa/RELEASE_0.60.0.json).

## [0.51.0] - 2026-10-23

Install SuniPDF on Ubuntu 22.04 or 24.04 amd64 with the new Debian package: desktop, CLI, private PDFium, local English/French OCR and application-menu entry. Linux settings follow XDG directories; recent paths retain case and recovery sessions use interprocess locks. Update notifications open the Ubuntu download; installation uses the system package manager. Windows packages remain available.

[Release notes](docs/RELEASE_0.51.0.md) · [Evidence](docs/qa/RELEASE_0.51.0.json).

## [0.50.0] - 2026-10-22

Find 18 tools in five searchable families from Tools, with Measurements and the annotation library under Review & measure. Switch one left dock between Tools, Pages and Bookmarks; use four measurement sections on the right. Page and zoom controls sit below the PDF, keep percentages such as 149% on one line and retain reading options in compact layouts. View rotation changes reading orientation only; the right rail opens panels. One context panel occupies the right side. An active measurement draft blocks opening other context panels; explicitly leaving or changing tools cancels that local draft and preserves confirmed scales.

The published 0.49.0 packages join upgrade regressions. Offscreen UI checks and retained CLI regressions remain separate from native qualification.

[Notes and limits](docs/RELEASE_0.50.0.md) · [Evidence](docs/qa/RELEASE_0.50.0.json).

## [0.49.0] - 2026-10-22

Add or remove vertices in saved polylines, perimeters and polygon areas, including closing edges, and select a point by number when handles coincide. Preview, adjust and Apply with one Undo step. The measurement keeps its own calibration, unit, precision, identity, replies and entered quantity fields; measured values and compatible price totals recalculate. Stale, invalid and protected edits remain guarded.

The published 0.48.0 packages join upgrade regressions. Insertion and removal sequences cover reopen, preserved data and recalculated quantities; fixed point counts and shape bounds remain guarded.

[Notes and limits](docs/RELEASE_0.49.0.md) · [Evidence](docs/qa/RELEASE_0.49.0.json).

## [0.48.0] - 2026-10-22

Edit existing vertices of a saved measurement directly on its original page, inspect Preview, adjust the draft and Apply with one Undo step. The measurement keeps its own calibration, unit, precision, identity, replies and entered quantity fields. Measured values and compatible price totals follow the accepted geometry; stale, invalid and protected edits remain guarded.

The documentation CI path checks actual packaged guides, links, backlog and evidence without repeating application tests for an eligible documentation-only diff; source, script and workflow changes retain full checks. The published 0.47.0 packages join upgrade regressions.

[Notes and limits](docs/RELEASE_0.48.0.md) · [Evidence](docs/qa/RELEASE_0.48.0.json).

## [0.47.0] - 2026-10-22

Overlay two saved PDF pages in magenta/cyan, align them through explicit translation, clockwise rotation and scale or two corresponding point pairs, then export a new PNG or source-bound JSON report. Both sources must permit copying; exact hashes, cropped/rotated geometry, bounded rasters and fresh-output checks preserve the originals.

[Notes and limits](docs/RELEASE_0.47.0.md) · [Evidence](docs/qa/RELEASE_0.47.0.json).

## [0.46.0] - 2026-10-22

Inspect actual counts and measurements in filtered quantity tables, edit typed fields and explicit unit prices with Undo, and export source-bound CSV/JSON. Totals distinguish missing values from zero, convert compatible price bases and keep currencies separate. Guarded CLI field changes require an expected record and a new PDF copy; copying and annotation permissions remain enforced.

[Notes and limits](docs/RELEASE_0.46.0.md) · [Evidence](docs/qa/RELEASE_0.46.0.json).

## [0.45.0] - 2026-10-22

Save supported local annotation drafts as named templates, combine up to 16 primitives in a flat group, then place a rendered preview at an explicit origin and scale with one Undo step and fresh identities. Strict self-contained JSON supports import, export and removal; CLI placement writes a new PDF copy and preserves encryption and permissions.

[Notes and limits](docs/RELEASE_0.45.0.md) · [Evidence](docs/qa/RELEASE_0.45.0.json).

## [0.44.0] - 2026-10-22

Edit a saved measurement's scale, unit and precision through Preview and Apply while retaining its identity, replies and register data. The CLI also edits geometry in a new copy guarded by an expected record. Measurement exports enforce copying permission; unsafe measured-page resizing is refused atomically. The inspector shares cached records and labels, and temporary scales can be cleared explicitly. No implicit manual calibration is supplied.

[Scope, audit and qualification status](docs/RELEASE_0.44.0.md).

## [0.43.0] - 2026-10-22

Save or remove a page scale explicitly in the PDF with Undo/Redo, inspect the current page's saved measurements, and reuse a non-angle measurement's physical scale for new measurements. Session calibration and existing annotation values remain separate. The CLI reads, writes and removes page defaults in new copies and converts units when using a saved scale.

[Release notes, validation and limits](docs/RELEASE_0.43.0.md).

## [0.42.0] - 2026-10-22

Measurements now has a labelled toolbar button and its own All tools workspace. Calibrate with two page clicks and a confirmed distance, then draw lengths, polylines, perimeters, areas and angles directly on the PDF. Preview and Apply control every insertion; point undo, cancellation and horizontal/vertical constraints support drawing. Each page keeps its session scale across annotation edits and normal tab switches.

[Release notes, validation and limits](docs/RELEASE_0.42.0.md).

## [0.41.0] - 2026-10-21

[0.41.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) under `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

Place persistent count markers directly on local PDF plans, group items, move/delete with Undo, regroup without changing identity and export totals linked to actual markers. The all-page annotation register adds search, named filters, sorting, navigation, business statuses, disciplines and custom text fields, with guarded CSV/JSON exports. Changes refuse stale snapshot targets and preserve originals and annotation permissions. Pinned test-only Pillow 13.0.0 replaces the vulnerable QA dependency. Published 0.40.0 packages join upgrade regressions. Shared annotation writers now enforce PDF annotation permission for moves, deletion, replies and other annotation edits; register exports also refuse incomplete page inventories or malformed count identities.

PR #16 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.41.0.md).

## [0.40.0] - 2026-10-21

[0.40.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) under `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

Local length, polyline, perimeter, simple-polygon area and angle annotations with explicit two-point calibration, metric/decimal imperial units, precision and JSON records. Each measurement embeds its own scale and geometry; preview/Undo and copy-only CLI output preserve originals. Image Stamp annotations embed aspect-fitted pixels and alpha. Text watermarks/numbering now preflight all pages and refuse invalid margins or overflow atomically. Published 0.39.0 packages join upgrade regressions. The local feature backlog prioritises plan workflows.

PR #14 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.40.0.md).

## [0.39.0] - 2026-10-20

[0.39.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) under `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

Named local presets for text/image watermarks, page numbers and six-zone headers/footers. Choose to load a draft, save under a unique name or delete explicitly. Atomic writes preserve active preferences on failure; malformed preset entries do not reset other valid settings. Page scope stays explicit and image files remain path references. Published 0.38.0 packages join migration regression. Office version probes run headlessly with a cancellable 30-second cold-start bound; conversion and security guards remain enforced.

PR #13 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.39.0.md).

## [0.38.0] - 2026-10-20

[0.38.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) under `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six header/footer zones through Page numbers and the headers-footers CLI command. Absolute page/count placeholders, margins, size and selected pages respect displayed rotation/crop coordinates. Overflow is refused atomically; allowed encryption and copy-only CLI guards remain. Published 0.37.0 packages join migration regression.

PR #12 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.38.0.md).

## [0.37.0] - 2026-10-20

[0.37.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) under `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Image watermarks in Pages and the CLI: transparency, proportional width, angle, seven positions and margins. Rotated/cropped pages fit on both axes. Atomic selected-page edits, one desktop Undo and copy-only CLI output preserve allowed encryption. Published 0.36.0 packages join upgrade baselines.

PR #11 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.37.0.md).

## [0.36.0] - 2026-10-20

[0.36.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) under `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Configurable keyboard shortcuts in Help and the palette: filter, assign, conflict refusal, disable and restore. Cancellable drafts, atomic preferences and consistent labels. Text input and reserved keys are preserved; published 0.35.0 packages join upgrade baselines.

PR #10 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.36.0.md).

## [0.35.0] - 2026-10-20

[0.35.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) under `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Visual signatures: optional drawn-stroke smoothing, paper-shadow reduction and the `prepare-signature-image` PNG command. Previews bind to prepared pixels; original points and source files are retained, with explicit drawing budgets. Published 0.34.0 packages join the upgrade baselines.

PR #9 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.35.0.md).

## [0.34.0] - 2026-10-20

[0.34.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) under `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspect the saved PDF from Properties and the `inspect` JSON command: seven declaration categories, source SHA256, explicit partial results, encrypted input and stale-reply refusal. The saved-revision guard hashes disk bytes in chunks.

PR #8 merged after exact-head CI/CodeQL.

[Release notes, validation and limits](docs/RELEASE_0.34.0.md).

## [0.33.0] - 2026-10-20

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) under `lmdanavanem`, release `417802490`, immutable source `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Preserve imported PDF 2.0 bookmarks containing tabs/line breaks while retaining strict new-title validation. Integrate egui/eframe 0.37 together and fix Back/Forward position after destination zoom changes. Add seven pinned licensed PDF Association examples, 64 bounded PDF/CMS mutations, executable guard controls and command measurements. The public 0.32.0 becomes an additional upgrade baseline.

PR #4, #5, #7 merged; #6 incorporated through #7 and closed.

[Release notes, validation and limits](docs/RELEASE_0.33.0.md).

## [0.32.0] - 2026-10-19

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) under `lmdanavanem`, release `417623635`, immutable source `8c492db35f1712648d5472593146077230befb7c`.

Drag one object or the selected group in the Objects panel, with an insertion line, page-cache/context guards, Escape and one Undo step. New `reorder-objects` JSON command shares the explicit final destination, relative order, permission, atomic-save and password safeguards.

[Release notes, validation and limits](docs/RELEASE_0.32.0.md).

## [0.31.0] - 2026-10-19

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) under `lmdanavanem`, release `417463135`, immutable source `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Local comparison of two saved PDFs (text, geometry and previews), explicit physical alignment and stale-preview refusal. Literal search across selected files, excerpts/page navigation, progress, cancellation and explicit partial results. New `compare` and `search-files` JSON commands; EN/FR interface.

[Release notes, validation and limits](docs/RELEASE_0.31.0.md).

## [0.30.0] - 2026-10-17

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) under `lmdanavanem`, release `415568680`, immutable source `4db31c0ee62cfae9d65239edf1eac93db147841e`. The five Bugs and fourteen Roadmap P2 items ship within their [documented supported limits](docs/RELEASE_0.30.0.md). P1 Roadmap items and Suggestions remain deferred.

The [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) and [MAIN CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) passed on this source: **551 tests**, zero failures, 9 default ignored; **1047 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature outputs**, **4 MuPDF font cases**, DX12 software WARP, **14 Office round trips**, 4 licensed PDFs/744 pages, **21 portable scenarios**, **15 MSI baselines/75 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

Six public assets match the inspected packages and SHA256 digests byte for byte. The production updater verified the 26,978,019-byte x64 ZIP in 2.55 seconds without installing on this host, using the previously built test harness with an unchanged updater module. Project readback: **42 unique items, 20 Todo, 0 In Progress, 22 Done**, three saved filters and zero unsaved changes. [Evidence and limits](docs/PROJECT_STATUS.md).

Packages remain unsigned. ARM64 is cross-built and packaged, without execution on ARM64 hardware. Physical printers, Narrator, manual updater interaction and physical GPU/window behavior still need qualification; WARP proves software offscreen rendering only. Unsupported inputs remain refused. PDF/A metadata preservation does not certify PDF/A or PDF/UA conformance.

The authorized batch covers five Bugs and all fourteen P2 Roadmap items. Implemented and validated on the published source. [Exact scope and supported limits](docs/RELEASE_0.30.0.md)
include paragraph preview, editable paste, rich annotations, precise links, semantic page labels,
bookmark/structure preservation, redaction preview and persistent marks, sanitization, font subsets,
local Office conversion, signature trust states, protected signature storage and license inventory.
The five bug fixes address Windows font discovery, rotation pivot, opacity, resized annotations and
optional GPU compatibility. P1 Roadmap work and Suggestions remain outside this batch.
Results belong in [Project status](docs/PROJECT_STATUS.md); old release sections below are historical.

## [0.21.1] - 2026-10-17

Published as [stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) under `lmdanavanem`,
with x64 MSI and x64/ARM64 portable ZIPs plus SHA256 files. Release-source tests, upgrade evidence,
public downloads and the live updater passed; [evidence and limits](docs/PROJECT_STATUS.md).

### Fixed

- Refuse lossy edits when an existing text object's original font cannot represent the replacement;
  retain the original PDF and the current editing draft for retry.
- Preserve non-widget annotations without a usable normal appearance when flattening forms, including
  embedded files and shared annotation references; failures leave the original document unchanged.

## [0.21.0] - 2026-10-16

- Harden native signer-certificate metadata access with an explicit null-pointer guard and add a
  malformed-CMS verification regression, retaining native ownership and existing error behavior (D112).
- Declare release-workflow root `contents: read`; retain the separate Publish job's `contents: write`.
- Preserve deterministic security fixtures and scanning coverage while triaging the reviewed test-only
  cryptographic-value alerts with individual evidence; the empty PDF password is an authentication probe.
- Retain the Dependabot updates to `getrandom` 0.4.3, `xml-rs` 1.0.0 and the existing GitHub Actions upgrades.
  Migrate the two XML readers to `ParserConfig` and explicitly reject DTD events before entity processing;
  preserve parsing bounds, with new UTF-8/UTF-16 XMP/XFDF refusal regressions and accurate XML license notices.
- Verify individual triage of all 115 criticals by `lmdanavanem`: 114 `used in tests`, #13 `false positive`.
  No critical remains open; high #3 and reindexed medium #119 are fixed by fresh CodeQL. Old #1 is historical.
- Local gates passed: 471 tests, zero failures, seven default ignored; formatting, Clippy with warnings
  denied, 912 matching EN/FR keys, 75 qpdf checks, update-helper self-test and the four-document/744-page corpus.
  New regression coverage includes one malformed-CMS and two UTF-8/UTF-16 XML entity-refusal tests.
- Fresh [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) fixed high #3 and medium #119; zero open alerts, with no scanning exclusions.
- Published 0.21.0 as stable Latest under `lmdanavanem`; Windows CI, same-source ARM64, 19 portable cases,
  13 MSI baselines/65 logs, six staged/public files and production-updater verification passed.
- Populate the GitHub project's Bugs, Roadmap and Suggestions views with 42 prioritised drafts and saved category filters;
  the completed 0.21.0 security item is Done.
  Actual results and alert states belong in `docs/PROJECT_STATUS.md` and `docs/CODE_SCANNING_REVIEW.md`.

## [0.20.0] - 2026-10-16

- Added: create and edit FreeText annotations in an anchored multiline page editor, retaining click/drawn
  rectangles and existing size/colour/background/border controls. Explicit Add/Save/Cancel reuses the current
  AddFreeText/UpdateFreeText worker edits, validation, protection and one Undo (D111/A39).
- Guard page/document/tab/tool/workspace identity, pending operations and concurrent drafts. Allow zoom;
  block conflicting navigation until confirmation/cancellation; retain typed text after validation/worker failure.
- Helvetica WinAnsi appearances, wrap growth, core/CLI APIs and 53 commands remain unchanged; no new dependency.
- Fixed quick rectangle drags dropping the final pointer position at release; conflicting search destinations
  cannot scroll an active FreeText draft away.
- Local workspace passed 468 tests, zero failures and seven default ignored; formatting, 912 matching EN/FR keys,
  75 qpdf checks, the licensed corpus and update-helper checks passed; Clippy with warnings denied passed.
- Published as stable Latest under `lmdanavanem`; Windows CI, same-source ARM64, 18 portable cases,
  12 MSI baselines/60 logs, six staged/public files and production updater verification passed.
  Actual results and limits belong in `docs/PROJECT_STATUS.md`.

## [0.19.0] - 2026-10-16

- Added: draw a rectangle on the page for the five existing form-field types, then review it in the
  existing numeric dialog before explicit **Create Field**. Preserve name, choices, editable/multiple-selection flags,
  core validation, encryption and one Undo step; drawing alone must not mutate the PDF (D110).
- Added guards: finite in-page geometry with a four-point minimum, stale document/tab/tool cancellation,
  crosshair/preview and no interaction with existing form widgets during drawing.
- No new field type, dependency or CLI command. Implementation, local tests, formatting and Clippy passed;
  CI/upgrade, staged/public package and production updater checks passed;
  **0.19.0 was stable Latest at publication**. Executed evidence belongs in `docs/PROJECT_STATUS.md`.

## [0.18.0] - 2026-10-16

[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) under `lmdanavanem`, release `415140702`,
source `7e52de82244faf9a020ed3ed233176775c007e97`. Local gates passed: 452 workspace tests (zero failed, seven default ignored),
formatting/Clippy with warnings denied, 907 matching EN/FR keys, 75 qpdf checks and the update-helper self-test.
Four licensed NIST/NBS documents (744 pages) passed in 50.796 seconds, sampled peak child memory
329,445,376 bytes; all sources stayed unchanged. Six new headless UI tests, the extended native-worker
grouped Undo/Redo/failure regression and the CLI help regression passed.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082734) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082934/job/117057854520) passed,
with sixteen portable cases, ten MSI baselines and 50 verified MSI logs. The CI corpus took 44.494 seconds,
sampled peak child memory 320,299,008 bytes. All six staged/public files, API digests/sidecars and enhanced
package checks passed: five grouped-Ink readbacks (plain/AES-128/AES-256 with both passwords), four literal-help
checks and existing conversion, forms, labels, links, dashes, compression, OCR and MSI payload checks.
Verified copies are in `dist/`; the production updater verified the 19,834,488-byte Latest x64 ZIP without
installing it on the host. Executed evidence and remaining limits are in `PROJECT_STATUS.md`.

### Changed

Group freehand strokes on one page into one annotation and one Undo step through **Finish drawing** /
**Cancel drawing**. Worker failure retains the draft for retry; conflicting save/close/document/tool actions
wait until Finish/Cancel. Reuse existing AddInk, point simplification, display geometry and protection (D109).
At most 64 strokes and 4,000 points per stroke. Drafts stay in memory until Finish and are absent from crash
recovery. No idle auto-commit or eraser. `stamp` / `page-numbers` help preserves literal `{n}` / `{total}`
through the existing clap help template (A16). No new dependency or CLI command; 53 commands remain.

Native pen/touch, interactive dialogs, printers, Narrator, ARM64 hardware and third-party readers remain unverified. Packages are unsigned; unfinished ink drafts have no crash recovery.

## [0.17.0] - 2026-10-16

Local validation passed: 445 workspace tests (zero failed, seven default ignored), formatting/Clippy all-targets
with warnings denied, 896 EN/FR keys, 74 qpdf checks and four licensed documents (744 pages; 48.187 seconds,
peak child memory 330,264,576 bytes). The update-helper self-test passed. The independent debug-CLI conversion
gate passed: eight orientations, nine exact JPEG streams including the maximum-DPI round trip, 32 JPEG/four
PNG corner comparisons and soft mask, three qpdf checks, two budget refusals before decoding, ten invalid-DPI
refusals and thirteen unchanged sources (35 subprocesses, 1.063 seconds).
Source: `5785de282e126fb4666b28fb83340d0e9c6e9acf`. [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
passed. All fifteen portable cases, nine MSI baselines and 45 MSI logs passed verification. The CI corpus took
45.791 seconds, peak child memory 321,413,120 bytes.
[Published (Latest at the time)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) under `lmdanavanem`,
release `415074108`, with six assets. All staged/public files and enhanced package checks passed, including
the independent JPEG/transparency/budget/DPI checks above. Verified copies are in `dist/`; the production
updater downloaded and verified the 19,825,743-byte Latest x64 ZIP without installing it on the host.
Evidence and remaining limits are in `docs/PROJECT_STATUS.md`.

### Changed
- Images → PDF embeds JPEG EXIF orientations 5 and 7 directly, completing all eight orientations without
  image decoding or recompression. The object matrix applies the orientation; original JPEG bytes are retained.
- Before raster decoding, check a cumulative estimate of at most 512 MiB for non-JPEG inputs: sum of
  `max(decoder.total_bytes(), width × height × 4)`, with checked arithmetic. Repeat checks at loading to catch
  simple input-file changes. Reject nonfinite/nonpositive physical page geometry.
- Budget excess/overflow reports `image_batch_too_large` with a localized desktop explanation; invalid physical
  geometry retains `other`. One EN/FR error key is added; all other existing error/output guards remain.

The budget is a conservative raster-data estimate, not a native/process RSS ceiling or a compressed-JPEG limit.
Other formats still decode with their quality/transparency preserved. Existing UI/CLI callers, conversion method signature and
53 commands remain unchanged; no new dependency. Source/output guards remain in place (D108).

## [0.16.0] - 2026-10-16

Local validation passed: 439 workspace tests (zero failed, seven default ignored), formatting/Clippy,
895 EN/FR keys, 73 qpdf checks and four licensed documents (744 pages; 53.542 seconds, peak child memory
331,517,952 bytes). Source: `6cd7843734f3924e87033d8b05539d493ccbcb95`.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537) and the
[same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589) passed,
along with all fourteen portable cases (ten upgrades including 0.15.0 and four guards), eight MSI baselines and
40 verified MSI logs. The CI corpus took 47.628 seconds with peak child memory 321,306,624 bytes.
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) under `lmdanavanem` (Latest at the time),
release `414934190`, with six assets. All six staged/public files and enhanced packaged checks passed;
verified copies are in `dist/`. The production updater downloaded and verified the 19,821,719-byte Latest
x64 ZIP without installing it on the host. Executed evidence and remaining limits are in `docs/PROJECT_STATUS.md`.

### Added
- **Dashed border** for rectangle, ellipse, line and arrow annotations; CLI `annotate --dashed` uses the same
  fixed 6-point dash/3-point gap. The CLI still has 53 commands. Solid remains the default, fills stay continuous
  and arrowheads remain solid.
- Read back the actual `dash_pattern` vector and preserve custom patterns through selection, recolouring,
  move/resize and Undo/Redo. The checkbox changes the pattern only when explicitly toggled; it does not normalize
  a loaded custom pattern on selection. No custom-pattern editor is added.

### Safety
- Write matching PDF border dictionaries and explicit dashed appearances with phase zero. Nonempty patterns
  are refused for ink, free text, links and markup. Unsupported/malformed border styles on SuniPDF annotations
  refuse appearance-regenerating edits rather than silently resetting the original style.
- An explicit empty dashed-border array is valid solid; an absent array keeps the PDF default `[3]`. Malformed
  SuniPDF borders disable Move/Resize/Style controls. Shape comments can change without rebuilding the appearance;
  malformed free-text comments are refused because their edit regenerates it. Invalid/excessive nonempty preview
  patterns omit the stroke instead of showing a solid substitute; valid subpixel patterns remain intact.
- Bound patterns to 16 finite nonnegative values, at most 1000 points each; positive values must be at least
  0.1 point, with a positive total. A conservative 100,000-segment appearance limit rejects excessive work.

No new dependency. Native interactive and third-party-reader checks remain separate; packages remain unsigned.

## [0.15.0] - 2026-10-16

Local source validation passed: 429 workspace tests (zero failed, seven default ignored), formatting/Clippy,
894 EN/FR keys, 68 qpdf checks and four licensed documents (744 pages; 48.891 seconds, peak child memory
328,736,768 bytes). Source: `952f8699cac451e7c3004b6d816105d52e41f249`.
[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) under `lmdanavanem` (Latest at the time), release
`414866395`, with six assets. [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488) and the
[same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541) passed,
along with thirteen portable scenarios and seven MSI baselines. All six staged/public files and enhanced checks
passed, including plain/protected retargeting, both passwords, link identity/area and unchanged page PNGs. Verified
copies are in `dist/`; the production updater verified the 19,803,226-byte Latest x64 ZIP without installation.
Executed evidence and remaining limits are in `docs/PROJECT_STATUS.md`.

### Added
- Edit an existing link through **Edit target** or double-click with annotation selection. The reused dialog
  accepts `http`, `https`, `mailto` or a physical page. Resolved named/XYZ destinations prefill their page;
  replacement explicitly uses whole-page Fit rather than preserving detailed position/zoom. Unsupported targets
  start with an empty address and cannot silently become page 1.
- CLI `set-link-target --page 1 --index 0 --goto-page N` or `--url ADDRESS`, with exactly one destination,
  `--out` and optional `--password`, reuses the existing output guard. Page indices are physical and one-based;
  annotation indices are zero-based, as returned by `annotations`.

### Fixed
- Replace the link's `A`, `Dest` and `AA` action fields together, keeping its rectangle/style, names, comments,
  replies, unrelated annotations/forms/content and source protection. ReadOnly, Locked, LockedContents or
  malformed flags refuse the edit, including indirect flags. One Undo restores the change without losing page
  geometry/navigation; edits and tab switches discard stale dialogs.

No new dependency. Existing external-address confirmation and signed-document rewrite warnings remain.
Native interactive, hardware and third-party-reader checks are separate; packages remain unsigned.

## [0.14.0] - 2026-10-16

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) under `lmdanavanem` (Latest at the time), release
`414812776`, with six verified files in `dist/`. Source: `af532888804c196aabe216b8f6a2980c3d1e9e2d`.
Validation passed: 420 workspace tests (zero failed, seven default ignored), formatting/Clippy, 890 EN/FR keys,
63 qpdf checks and four licensed NIST/NBS documents (744 pages; 53.91 seconds locally). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
passed, along with twelve portable scenarios and six MSI baselines. All six staged/public files and enhanced
binary checks passed, including plain/protected label authoring/reset, both passwords, unaffected labels,
Unicode, unchanged text/page PNGs, source preservation and qpdf. The production updater verified the
19,785,200-byte Latest x64 ZIP without installation. Evidence and limits: `docs/PROJECT_STATUS.md`.

### Added
- **Page Labels…** in Pages/Organize: apply decimal, upper/lower Roman, upper/lower letters or prefix-only labels
  to an inclusive physical page range, with a prefix and positive start. Labels outside the range retain their
  existing numbering; this changes navigation metadata, not visible or printed page numbers. Reset All Labels
  removes the catalog entry, including malformed label trees; it is not secure erasure of old metadata objects.
- CLI `set-page-labels --from 1 --to N --style decimal --start 1 --prefix ""` and `reset-page-labels`, both with
  `--out` and optional `--password`, reuse existing overwrite safeguards. CLI page indices remain physical.

### Fixed
- Undo/Redo carries whether an edit preserved page geometry, so label/content edits retain navigation state.
  Structural edits still invalidate it.

New prefixes are one line of at most 256 UTF-8 bytes; generated labels are bounded to 4096 bytes, and invalid
ranges, numbering overflow or malformed existing trees refuse Apply before mutation. Label ranges remain
positional after page insertion/deletion/reordering; semantic rebasing is not implemented. Duplicate labels
still require physical `#N` navigation. Validation and hardware/reader limitations remain in Project status.

## [0.13.0] - 2026-10-16

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) under `lmdanavanem` (Latest at the time), release
`414763678`, with six verified files in `dist/`. Source: `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`.
Validation passed: 410 workspace tests (zero failed, seven default ignored), formatting/Clippy, 870 EN/FR keys,
53 qpdf checks and four licensed NIST/NBS documents (744 pages; 76.40 seconds locally). [Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
passed, along with eleven portable scenarios and five MSI baselines. All six staged/public files and enhanced
binary checks passed, including plain/protected signature batches, both encryption passwords, qpdf and source
preservation. The production updater verified the 19,730,578-byte Latest x64 ZIP without installation.
Evidence and limits: `docs/PROJECT_STATUS.md`.

### Added
- **Type** a visual signature or initials, select a standard/system TrueType font and black/blue ink, then
  explicitly **Preview** locally before placement. Times italic is the default; missing glyphs and restricted
  font embedding are refused. Input is one line of at most 256 characters; the transparent PNG preview is bounded
  to 1600 pixels per side. Text/font/ink/mode/dialog changes invalidate previous previews and pending responses.
- Place a visual mark on the current page, all pages or an Organize selection captured when the dialog opens.
  One click supplies the same display-space centre in PDF points for each target; marks retain aspect and physical
  size, shrinking only to fit smaller pages. Multi-page placement has progress, cancellation between targets,
  atomic rollback and one undo; save is still explicit.
- CLI `add-image --pages "all|1,3-5"` reuses atomic fitted placement. Existing `--page` behaviour and single-page
  JSON stay compatible; only the batch option clamps/fits placement.

### Fixed
- Keep oversized visual marks and their placement previews inside small pages using the same fitted geometry.
- Discard pending signature previews and page-target snapshots when cancelling, switching documents or changing page structure.

Visual signatures remain editable images, with no identity/integrity proof; CMS/PFX signing is a separate
existing function. Optional saved signatures remain plain local PNG files. Hardware/native interaction and
third-party reader checks remain separate; distribution packages are unsigned.

## [0.12.0] - 2026-10-16

[Published](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) under `lmdanavanem` (Latest at the time), release
`414674886`, with six verified files in `dist/`. Source: `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`.
Validation passed: 398 workspace tests (zero failed, seven default ignored), formatting/Clippy, 859 EN/FR keys,
48 qpdf checks, four NIST/NBS documents (744 pages; 59.68 seconds), Windows CI and same-source ARM64 compilation.
Ten portable scenarios and four MSI baselines passed, as did staged/public packages, enhanced binary checks and
the production live updater's verified download without installation. Evidence and limits: `docs/PROJECT_STATUS.md`.

### Added
- Create editable drop-downs in Prepare a form with **Allow custom text in addition to the listed options**, or
  `add-form-field --kind combo --editable`. A nonempty set of predefined choices is still required.

### Fixed
- Use radio-button `/Opt` export values for filling, clearing and JSON/XFDF/FDF interchange, while keeping each
  widget's appearance state separate. CLI field information adds `widgets[].export_value`; `on_state` stays unchanged.
- Keep the affected radio field read-only if its export mapping is malformed or ambiguous, including duplicate
  export values. Radio fields without `/Opt` retain their existing state-name values.

## [0.11.0] - 2026-10-16

Release validation and publication evidence are recorded in `docs/PROJECT_STATUS.md`.
Published under `lmdanavanem` (Latest at the time), with six verified public files in `dist/`. Validation passed: 390 workspace
tests, formatting/Clippy, 42 distinct qpdf PDFs (44 checks), four real documents (744 pages), final Windows CI and the
same-source ARM64 build. Nine portable scenarios, three MSI baselines, enhanced public-binary checks and the production
live updater passed. Immutable release source: `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`.

### Added
- Read PDF page labels in the toolbar, thumbnails, status and CLI `info`. Go to a unique exact label or use `#N`
  for a physical page; ambiguous labels leave navigation unchanged.
- Follow internal links and bookmarks to supported XYZ positions/zoom, Fit and nonzero FitH destinations,
  including names resolved by PDFium. Back/forward keeps same-page positions and zoom, including rapid jumps.
- Enter custom text in existing editable drop-down form fields, in the fields panel or on the page.
  Standard options remain selectable; typed values use the existing grouped Apply/Discard and undo mechanisms.

### Changed
- Extracted pages and split parts retain the source passwords, encryption key and permissions. The CLI keeps
  `encryption_removed` as `false` and adds `encrypted`. Plain sources still produce plain copies.
- Extraction and splitting refuse to replace their source, even with explicit overwrite. Split without overwrite
  also refuses an output created between its initial check and atomic publication.
- Actual Windows upgrade regressions now include portable and MSI upgrades from 0.10.0.

### Fixed
- Pin the official lopdf corrections for legacy owner-password key derivation and password encoding on load.
  Owner-opened protected copies and metadata edits retain readable content; both credential roles are regression-tested.
- The text-selection tool now follows link clicks while preserving the confirmation and scheme checks for external URLs.
- A failed form-model read preserves text and choice drafts, blocks changes based on stale values, and offers explicit Retry.
- Cache page rotations during the label scan, update only the affected metadata when rotating a page, and refresh
  grouped-deletion metadata once. CLI `info` avoids its previous duplicate page-loading loop.

### Limits
- Page-label authoring and semantic rebasing after structural edits remain unimplemented. Unreadable, control-containing
  or over-4096-byte labels use physical page numbers. Unsupported destination fit modes and ambiguous FitH zero/null
  fall back to page navigation. Reading labels adds an eager metadata pass on large documents. Custom drop-down text
  requires a field already marked editable; creating or changing that flag is not implemented. Split failures can
  leave earlier completed parts; no partially written output or changed source is accepted.

## [0.10.0] - 2026-10-16

Validation and publication are tracked in `docs/PROJECT_STATUS.md`; implementation alone is not a passed release gate.
Local validation passed: 362 workspace tests, Clippy/formatting, 31 qpdf fixtures/outputs and four authentic public
research documents (744 pages). Final main CI and the same-source ARM64 build passed; six staged files, all eight
real portable scenarios and both real MSI upgrade baselines passed. Published as Latest under `lmdanavanem` with
six verified public assets in `dist/`; enhanced public-binary checks and the production live updater passed.
Evidence and immutable release source are recorded in status.

### Added
- Image compression to a new copy: maximum image side and JPEG quality, measured before/after sizes, progress and cancellation.
  Supported 8-bit DeviceRGB/DeviceGray images are resampled; masked/profiled/non-default-decoded images stay untouched.
  No gain returns the original bytes. Signed documents and documents that forbid modification are refused.
- Sequential multi-file compression and offline FR/EN OCR: at most 128 PDFs, each at most 256 MiB, individual results,
  cancellation and atomic non-overwriting output copies. Original documents stay unchanged.
- View-only quarter-turn rotation, full-screen reading and a bounded 256-entry navigation history per tab.
  Editing returns to the document's upright view; view rotation never rewrites PDF page rotation.
- Form creation for radio groups, drop-down lists and single/multiple-selection list boxes. XFDF value export and
  JSON/XFDF/FDF value import validate the complete import before applying one undoable edit.
- Grouped form-panel Apply/Discard controls, keyboard Space/Enter activation of page widgets and focused choice popups.
- Mouse cropping in the page preview and grouped object rotation/resizing, each committed as one undo step.
- CLI `compress-images`, `form-export`, `rotate-objects` and `resize-objects`; `add-form-field` accepts choice options
  and `fill-form` accepts `.json`, `.xfdf` and the supported value-only `.fdf` subset.

### Changed
- Atomic saves use a neighbouring temporary file, flush its contents, then atomically replace the destination.
  Injected interrupted/full-disk/permission failures and a killed partial-save child have dedicated regression coverage.
- Independent QA adds four pinned, publicly licensed NIST PDFs, alongside the synthetic qpdf corpus.
  These public samples do not represent all customer, hostile or large documents.

### Fixed
- Full PDF rewrites remove obsolete object/xref streams and linearization hints, recalculate the emitted object-number
  bound and discard stale previous-revision offsets. This prevents invalid `/Size`/xref data on real producer files
  while preserving encryption and permissions. No-gain compression keeps the original bytes and their pinned diagnostics.

### Limits
- Image compression is lossy and may yield no reduction. Batch OCR keeps its existing recognition limits; files
  needing a password are refused by the batch UI rather than silently decrypted.
- FDF import supports normal cross-reference tables and text/name/array field values; streams, actions, attachments,
  rich text and FDF export are unsupported. Radio labels are shown in the fields panel, not added as PDF page text.
- ARM64 execution, physical printers, Narrator and interactive/third-party-reader compatibility remain separate checks.

## [0.9.1] — 2026-10-16

### Fixed
- Larger red-ribbon application icon with much less transparent padding, shared by Windows, the installer and the interface.
- Metadata edits preserve existing PDF/A identification and unrelated XMP properties. Malformed XMP is refused instead of silently discarded; this does not certify PDF/A conformance.
- AES-256 encryption dictionaries explicitly declare their 256-bit key length for independent readers.
- Update results are recorded after starting the new application, with backup details retained on installation or restart failure.
- Update SHA256 verification uses .NET directly, including when PowerShell module paths are inherited from PowerShell 7.

### Changed
- Pinned QPDF checks in Windows CI cover the synthetic regression corpus, generated edits, AES encryption, forms, redaction, OCR, digital signatures and malformed inputs. Structural warnings fail the gate.
- Real portable-update regression and disposable Windows MSI installation/upgrade/rollback checks supplement the existing unit tests. Executed results and remaining hardware limits are recorded in `docs/PROJECT_STATUS.md`.

## [0.9.0] — 2026-10-16

Validation results and remaining hardware checks are recorded in `docs/PROJECT_STATUS.md`.

### Added
- Automatic recovery snapshots every 60 seconds and when suspending a modified tab. Recovery keeps the
  original PDF unchanged, preserves PDF encryption without storing passwords and restores into a new copy.
- Windows printing through the native printer dialog: current edits, page ranges, copies, raster output,
  progress and cancellation. Physical printer validation remains pending.
- Reading selection across pages in either direction, word selection by double-click and edge autoscroll;
  copied text follows PDFium's logical character order, including on rotated and cropped pages.
- Two-page and cover-first Book views with mixed page sizes, anchored zoom and the existing bounded tile cache.
- Prepare a form: create named text fields and checkboxes with numeric page rectangles, undo, save and fill.
- Bundled offline English/French OCR: Tesseract 5.5.3, Leptonica 1.87 and fast `eng`/`fra` models;
  invisible searchable text, existing-text skipping, progress, cancellation and a single undo step.
- Use a certificate: verify saved PDF signatures and create an incrementally signed copy using a PFX/P12
  identity. Certificate identity trust, revocation and trusted timestamps are not established.
- CLI `add-form-field`, `ocr`, `signatures` and `sign`. Certificate passwords come from an environment
  variable or piped stdin, never a plaintext password argument. Signing cannot overwrite its source.

### Changed
- All 14 task tools have an implementation. Signed PDFs show a warning before a rewrite; encrypted PDFs
  cannot be digitally signed. Office conversion, view rotation and advanced font work remain on the backlog.

## [0.8.3] — 2026-10-16

### Fixed
- Adding text rejects NaN and infinite coordinates before changing the document.
- Organize pages bounds thumbnail textures and queued renders while browsing large documents;
  outdated results are discarded and failed thumbnails do not block the queue.
- Stamps and page-number templates explicitly reject line breaks.
- CLI free-text annotations reject unsupported opacity values; annotation opacity must be finite.
  Rejected commands leave existing output files untouched.

## [0.8.2] — 2026-10-16

### Added
- Update checks at startup, a download notification and an explicit “Close and install” confirmation.
  Checks can be disabled in About; manual checks remain available. PDF files are never sent to GitHub.
- Official release downloads are checked for architecture, size and SHA256 before installation. Installed
  x64 copies use the MSI; portable x64/ARM64 copies update their own folder with a rollback backup.

### Changed
- English is the default interface and primary repository/documentation language. French remains available;
  existing language preferences are preserved.
- English release notes and GitHub templates; repaired the malformed 0.8.0 release description.

### Fixed
- MSI upgrades can restore the previous installation if the new installation fails.
- Cancelling a document close or a failed save cancels the pending installation intent.

## [0.8.1] — 2026-10-15

### Fixed
- Tabs: the close button receives clicks and closes the document; clicking the title still switches tabs.
- Closing a background tab activates and closes that document, prompting to save its changes when needed.
  A tab switch refused during a save does not close any document.

## [0.8.0] — 2026-10-15

Six priority fixes, 26 regression tests added (248 tests passed, 2 measurements ignored).

### Fixed
- Protection: validate passwords after normalization and AES-128 / AES-256 length limits; reject an opening
  password that becomes empty and equivalent owner/user passwords.
- Forms: Ctrl+S includes panel drafts; invalid input stays editable and blocks saving. Closing also takes
  unfinished input into account.
- Tabs: selection responses from the previous document cannot target objects in the new tab.
- Closing: pending engine mutations finish before deciding whether to save or close.
- Moving pages preserves form fields, bookmarks, internal links and inherited page attributes; encrypted
  documents remain encrypted.
- CLI `form-fields`: password values are hidden in the full listing as well.

## [0.7.0] — 2026-10-15

Irreversible redaction (“Redact a PDF” tool, `redact` CLI command, 42 commands, 222 tests).

### Added
- **Irreversible redaction** ("Redact a PDF" hub tool): draw areas (B key), or mark every occurrence of a text,
  remove an area with a click, then **Apply** (Ctrl+Shift+B, confirmation). Text, images, drawings, annotations and
  fields under the areas are **removed from the document**, not covered; the rest of each affected element is kept
  as an image. The engine reads the result back and refuses (`redaction_incomplete`, nothing changed) if extractable
  text remains under an area. Options: convert redacted pages entirely to images, remove hidden information
  (metadata, tags; checked by default). The file is rewritten without earlier revisions; a protected document stays
  protected; undo with Ctrl+Z. Saving with unapplied areas asks for confirmation. Limits: `docs/KNOWN_ISSUES.md` A40.
- CLI: `sunipdf-cli redact file.pdf --out output.pdf [--rect PAGE:L,B,R,T]… [--text TEXT]… [--match-case]
  [--rasterize] [--keep-hidden-data] [--dpi N]` (JSON output, `redaction_incomplete` error with nothing written).

## [0.6.0] — 2026-10-15

Advanced annotations, new visual identity and open-source release preparation.

### Added (project)
- Open-source release preparation: MIT / Apache-2.0 licences, `CONTRIBUTING.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, issue and pull request templates, bilingual README, English translations of every
  document in `docs/` and of the changelog (`docs/README.md` is the index).
- New logo (light and dark variants): symbol in the tab strip, full logo on the home screen and in the About
  box, `.ico` regenerated by `scripts/make_icon.py`.
- Project website: https://navanem.com/projets/sunipdf/ (link in About, `Cargo.toml`, README).

### Added
- **Advanced annotations** (Add comments tool): freehand drawing (smoothed stroke), free text box, rectangle,
  ellipse, line and arrow, all as **real PDF annotations** (explicit appearance: other readers show them);
  selecting an existing annotation to move, resize, recolour, edit the text of or delete it (shortcuts A, D, F);
  floating bar (colour, width, fill, opacity, arrow, text size).
- **Clickable links**: creation (page or `http` / `https` / `mailto` address), click on an internal link, and for an
  external address a **confirmation dialog that shows the full address** before the browser opens; launch or script
  actions are never run.
- **Reply threads and review statuses** in the Comments panel (indented replies, status Accepted / Rejected /
  Cancelled / Completed, remembered author, date, thread deletion), written according to the PDF standard (display
  in other PDF readers not verified).
- CLI: `annotations`, `annotate` (`ink`, `square`, `circle`, `line`, `arrow`, `freetext`, `link`), `reply`,
  `review-state`, `annotation-delete` (41 commands).
- Fixture `annotated.pdf` (12 annotations from several producers); 40 more tests (207 in total).

### Changed
- Annotations are written by lopdf with their own appearance: rotated or cropped pages are right (free text upright
  on screen), a password-protected document stays protected.
- Annotation enumeration no longer stops at the first unreadable entry (indices aligned with `/Annots`).

## [0.5.0] — 2026-10-15

Backlog wave 2, each slice implemented, reviewed by an independent reviewer, fixed, then re-reviewed:
fixes for the 39 findings of the adversarial review, visual signature (the "sign" half of Fill & Sign),
AcroForm forms (the "fill" half), AES-256 encryption and normalized passwords, distribution.

### Fixed (review of the forms slice)
- **Flatten removed links** (web addresses, cross-references to a page): `FPDFPage_Flatten` removes the whole
  `/Annots` array and a link has no appearance. Links on flattened pages are collected beforehand (lopdf) and put
  back into `/Annots` afterwards, **deep-copied** (PDFium does not save the indirect `/A` action of a link it
  has removed: inlined references, translated pages, streams re-added); the returned annotation count excludes
  them. Fixture `acroform-links.pdf`, test `flatten_keeps_link_annotations`, Flatten dialog text and guides
  updated.
- The **editor placed over a text field** and the **choice list** followed their widget outside the document area
  (drawn over the tab bar, clicks captured): clipped to the visible area, confirmed and closed as soon as the
  widget leaves it entirely; the list stays within the document area.
- **Round trip `form-fields --values-only` → `fill-form --values`**: the current value of a read-only field is
  accepted without being rewritten (CLI, JSON import, engine); same rule in the interface ("0 skipped").
- **Password** field: neither highlighted, nor opened, nor exported (fixture and test); tolerant `--set`: reason
  without the "invalid form value: name:" prefix; `flatten(Some(..))` deduplicates pages.
- Interface: panel controls attached to the field name (`labelled_by`); square checkboxes; "form to fill in"
  banner collapsible in a narrow area; Fill tool left after a full flatten; Ctrl+S confirms the current entry;
  value rejected by the engine → model re-read; "Clear fields" with nothing to clear no longer adds an undo
  level (status message); typing hint hidden for an XFA form.

### Added (AcroForm forms, forms slice)
- **Fill & Sign** gains its "fill" half: **Fill form** tool (fields highlighted on the page: blue, red if
  required, grey if read-only; click toggles a checkbox or a radio button, editor placed over the text field with
  Enter / Esc / Tab / Shift+Tab and maximum length, floating choice list for drop-down lists and list boxes,
  multiple selection with checkboxes), working in continuous scrolling and in every tab; "This document
  contains a form to fill in" banner on opening.
- **Form fields** panel (Ctrl+Shift+F): one accessible control per field (text box, checkbox, radio buttons,
  drop-down list, list box; buttons and signatures listed without action), current page or whole document,
  XFA (read-only) and scripts (never run) banners, go to a field by its name.
- **Forms** menu: Clear fields, Flatten form and annotations… (all pages or the current page, confirmation
  dialog, undoable), Export form values (JSON)…, Import values into the form (JSON)… (unknown fields and
  rejected values counted in the status message).
- Engine: `Document::form_info` (lopdf model: hierarchy, inheritance, checkbox and radio states, lists with
  export values and labels, rectangles in display space), `fill_form` (strict or tolerant, full validation
  before any write, `/V` `/AS` `/I` written by lopdf, appearances regenerated by PDFium and written to the
  file), `clear_form`, `flatten` (Print flag set on the annotations, appearances generated beforehand,
  `/AcroForm` removed when every page is flattened), `ensure_appearances`, `DocumentInfo.form`, `form_data`
  module (JSON); an encrypted document can be filled and stays encrypted; typed errors `form_field_not_found`,
  `form_value_invalid`, `form_xfa`.
- CLI: `form-fields` (`--values-only`), `fill-form` (`--values`, `--set`, `--ignore-unknown`, `--flatten`),
  `flatten` (`--pages`), 36 commands. Fixtures `acroform.pdf`, `acroform-noap.pdf`, `acroform-xfa.pdf`
  (`scripts/make_fixtures.py`).

### Fixed (security, AES-256)
- An AES-256 password containing a special space or a ligature (SASLprep-normalized by lopdf at encryption
  time) did not reopen the file in SuniPDF: opening retries with the normalized form, and the password
  remembered for undo is that one (engine test added).

### Added (visual signature, sign-image slice)
- **Fill & Sign** (the "sign" half) becomes a real hub tool, with its bar: **Sign (handwritten
  signature)…** (S), **Add Initials…**, **Insert Today's Date**; also in the Edit menu and the palette.
  **Visual signature only**, stated everywhere (label, subtitle, permanent notice): it proves neither identity
  nor integrity; it is not a certified digital signature.
- "Handwritten Signature / Initials" dialog: drawing pad (mouse, touchpad, pen; black or blue ink, thickness,
  undo last stroke, clear), **image import** (transparent PNG as is, or a photo on a light background whose
  background is removed with an adjustable threshold and a preview, decoded off the interface thread),
  **local library** (`%APPDATA%\SuniPDF\signatures`, unencrypted PNG, optional saving, two-click deletion,
  thumbnails on white paper in both themes, also decoded by the engine thread).
- Placement: ghost under the pointer, click to place (centred, kept inside the page), Shift+click to place
  several times, Esc to cancel; the placed element is an **image object with an alpha mask** (the content stays
  visible between the strokes) or a text object (date), selected right away: handles, move, rotate, duplicate,
  delete, Ctrl+Z, editable with F2 for the date.
- Engine: `Document::add_image_data` (decoded image, alpha kept through `/SMask`), `signature_image` module
  (anti-aliased rasterized strokes, background removed, cropping, reading bounded to 16,384 px per side and
  512 MiB decoded, refused before any decoding, atomic PNG).
- CLI: `add-image <FILE> --image <IMG> --x --y [--width] [--remove-background <seuil>] --out` (response with
  `"digital_signature": false`), 33 commands.
- Automation: `SUNIPDF_SIGNATURES_DIR`, `SUNIPDF_PICK_FILE`.

### Fixed (sign-image review)
- Common dialog frame (`dialog.rs`): when the body overflows (full signature library), the floating scrollbar
  gets a 10 px gutter instead of covering the right edge of the notice, the pad and the Clear button
  (screenshot re-checked).
- **Memory**: an image whose decoding would exceed 512 MiB (a 2 MB PNG announcing 16,384² pixels in RGBA16
  gave 2 GiB) is refused when reading the header, without allocating anything; the same bound now applies to
  Insert Image, Replace Image and Images → PDF (single bounded decoder in the engine).
- The **Fill & Sign** card describes what exists (sign, initial, date) and no longer a form filling that is
  not shipped.
- The **date** placed near the right or top edge stays inside the page (estimated width, 0.556 em per character).
- Signature library decoded by the engine thread (no longer on the interface thread); after a deletion, the
  next signature beyond the cap of 12 appears right away; "Click again to delete" label that fits in the card
  (and nothing overflows the card any more); ink dots on a white disc, legible in the dark theme; Shift+click no
  longer shows handles on the placed object while placement mode continues; accessible name "Saved initials n"
  for initials; the log only contains the image file name, never its full path.
- End-to-end CLI test `add-image` (background removed, cropping, height, rendering re-read pixel by pixel,
  `already_exists`, empty image); the number of CLI commands announced in the documentation (21) is
  corrected: 33.
- CLI `add-image`: a fully transparent image without `--remove-background` is refused (`image is fully
  transparent`) instead of placing an invisible object.

### Fixed (review-fixes slice: findings of the adversarial reviews)
- **Security**: AES-128 refuses a password containing characters outside printable Latin-1 (error
  `password_not_latin1`, message in the Protect dialog) instead of silently dropping them, which produced a
  file that could be opened without a password; the symbols € œ — “ ” (PDFDocEncoding outside Latin-1) are
  refused too, because PDFium did not reopen the file with the entered password (round trip tested); the
  Protect dialog gives focus to the first field; the extraction bit for accessibility is always granted;
  editing the properties also updates the XMP packet (the former author no longer survives in it);
  "Remove Metadata" also removes the XMP of pages and images and the `/PieceInfo` data.
- **Files**: atomic saving flushes the data to disk before replacing the original (no more file of zeros after
  a power cut); **extracting or splitting a protected document** produces unencrypted files, now announced
  (status bar, `encryption_removed` in the CLI).
- **Images**: the EXIF orientation of photos is applied (Images → PDF, insert and replace an image); JPEGs
  are embedded as is in Images → PDF (constant memory, file the size of the JPEGs).
- **Editing**: "Bring to Front" holds after reload, undo and save (the content streams are merged in both
  directions); a 45° stamp placed in a corner stays inside the page; resizing an object rotated by an arbitrary
  angle scales it uniformly instead of shearing it; added text accepts tabs (four spaces) and the font error
  lists the missing characters; `sunipdf-cli text` returns the whole text of rotated pages.
- **Keyboard**: Ctrl+Shift+S no longer triggers "Save" (overwriting the original) when a field has focus;
  Ctrl+Shift+D duplicates (instead of opening Properties) and Ctrl+Shift+Z redoes (instead of undoing);
  Ctrl+Plus / Minus / 0 no longer change the scale of the whole interface; dialogs and text editors suspend
  every shortcut.
- **Dialogs**: Esc on a drop-down list or a colour picker only closes the list (the typed text is no longer
  lost) and no longer closes the search bar behind the dialog; the buttons stay visible in a short window
  (scrolling body); "Apply metadata" is greyed out while the properties load (it cleared title, author,
  subject and keywords); "Apply" is greyed out without text; error text legible in the dark theme; "Remove
  Protection" no longer shows "not protected" right after Protect.
- **Document**: undoing back to the saved state makes the document "intact" (save point in the history; the
  tab indicator follows that state instead of a logical OR); "Cut" copies then deletes in the engine thread,
  in one go (nothing is deleted if the copy fails, no edit can shift the indexes between the two); the font
  error lists each missing character once; the stamp position list shows its seven entries; the annotation
  of a drag is the one of the tool at the time of the gesture (switching tools before the engine answers no
  longer changes it); after deleting pages in the grid the selection is cleared (a second click did not delete
  the chosen pages); dropping a page at its own place no longer marks the document as modified; "Move
  Forward / Move Backward" greyed out when several pages are selected or at the end of the document.
- **Texts**: the Comments panel names every subtype ("Strikethrough" instead of "Strikeout", text boxes,
  shapes, ink, stamps, attachments…); file sizes and suffixes of the proposed names translated ("2.3 MB",
  `-extract`, `-optimized`).

## [0.4.0] — 2026-10-14

Backlog wave 1, delivered in slices with independent review: "All tools" hub, advanced content editing,
continuous view, then fixes from the adversarial review and hardened distribution.

### Added (continuous view, SUNI-012b)
- **Continuous scrolling**: all pages stacked in a single scroll area (default mode); **Single Page** remains
  available (F5, right rail button, View menu, palette), choice remembered. Only the visible pages are
  rendered and kept in graphics memory: a 500-page document scrolls without loading everything.
- The **current page follows the scrolling** (rail field, status bar, thumbnail brought into the panel,
  comments panel); clicking a page makes it current; "go to" (field, thumbnail, bookmark, Home / End, next /
  previous page, search result) brings the page to the top of the screen.
- **Anchored zoom**: Ctrl + wheel zooms around the pointer (touchpad pinch: same path, untested), buttons and
  list around the centre; list extended to 800, 1200 and 1600%, wheel up to 3200%; during the zoom the
  previous image stays displayed, stretched, until the new rendering.
- **Tiled rendering** (1024 px) beyond 4096 px per side: deep zoom stays sharp without the 16,384 px cap; an
  edit (object, highlight, stamp) keeps the displayed image until the new rendering instead of a blank page.
- Keyboard: Space / Shift+Space and Page Down / Page Up scroll one screen in continuous mode, ↑ ↓ arrows one
  line (in both modes).
- Engine: `Document::render_region`, `sunipdf_core::tiles` module, tile keys in the LRU cache; CLI
  `render --region x,y,w,h` (`region`, `page_pixels` output).
- Fixture `many-pages.pdf` (120 pages of mixed sizes, `make_fixtures.py --many N` for more); 7 integration
  tests (regions = crop of the full rendering, rotated page, ×40 zoom beyond 16,384 px, image page, errors,
  tile stitching), 14 view tests (layout, virtualization of 600 pages, zoom anchor, caps, bounded texture
  pool, wanted tiles, Space / Shift+Space); `dev-screenshot.ps1 -Wheel` for Ctrl + wheel.

### Changed (continuous view)
- The **Objects panel moves from F5 to F3** (F5 now toggles the view).
- In continuous scrolling, **Page Down / Page Up scroll one screen** instead of changing page; the → ← arrows
  still change page (shortcuts shown as "→" / "←").
- "Fit Width / Fit Page" in continuous scrolling fits the largest page of the document (same zoom for all
  pages).
- Right rail: the active toggles (thumbnails, Organize, continuous scrolling, panels, search, fit) are
  highlighted; in a short window the top group scrolls instead of overlapping the navigation. The active view
  mode is highlighted in the View menu.

### Fixed (continuous view)
- Possible crash at high zoom on a GPU limiting the texture size to 8192 px (page rendered in one piece larger
  than the maximum texture): no texture exceeds the GPU limit any more.
- Close button of the search bar shown as an empty square (glyph missing from Segoe UI).

### Added (advanced editing, SUNI-031c)
- **Free rotation** of objects: round handle above the frame (Shift: 15° steps, angle in the status bar),
  "Angle" field in the object bar; oriented frame for texts and images.
- **Opacity** of texts, shapes and images (object bar, new shapes and images); changing an object's colour now
  keeps its opacity.
- **Multiple selection**: Shift/Ctrl+click, selection rectangle (Alt: touched objects), Ctrl+A; move, delete,
  duplicate, order, colours and opacity applied to the group; direct grab of an unselected object on drag.
- **Copy / cut / paste objects** (Ctrl+C / X / V, Edit menu, object bar) between pages and between tabs;
  pasted objects form a group.
- **Objects panel** (F3 since the continuous view): list of the page from frontmost to backmost, selection,
  bring forward / send backward one level, deletion, marker on hover.
- **Text on the page**: direct typing at the clicked location (Enter, Shift+Enter, Esc), mini bar size /
  colour / font / bold / italic; several lines; in-place editing of an existing upright text.
- **System TrueType fonts** embedded (list with search, embedding licence checked); a character missing from
  the chosen font is refused with a message instead of being lost.
- **Replace Image** (same frame, new image fitted), **Insert Blank Page**, **Crop Pages** (margins in mm,
  preview, restore), **Resize Pages** (content kept or scaled).
- CLI: `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`,
  `resize-pages`, `insert-blank`, `fonts`; `info` gives the rotation of each page.
- 9 commands in the registry (78 in total); 11 engine integration tests (rendered pixels, re-read, reopening,
  errors) on a new `boxes.pdf` fixture (rotated and cropped pages).

### Fixed (advanced editing)
- **Rotated or cropped pages**: selection, handles, shapes, text, images, highlighting, notes and search
  results landed in the wrong place (the engine spoke in unrotated PDF coordinates). The whole engine API is
  now in displayed-page coordinates; **`sunipdf-cli search` also returns its rectangles in that space**
  (visible change only on rotated or cropped pages).
- **Ctrl+C did not copy** the selected text (egui turns Ctrl+C into a Copy event, without a key): Ctrl+C / X / V
  now go through these events.
- **Duplicate** brought the original object to the front: now only the copy changes.
- **Size of a rotated text**: reported as 0 (90°) or reduced (45°), it was multiplied on edit (a 90° stamp went
  from 48 to 12 pt, a vertical text from 12 to 144 pt); it now accounts for the rotation.
- **Stamps on a rotated page**: centred and at the requested angle on screen.
- **Text typed in a standard font**: a line break or a non-Latin character (Cyrillic, emoji) produced "ÿ"
  without an error; each line becomes an object and a missing character is refused.
- **Changing the colour** of a transparent object (30% stamp) made it opaque.
- The CLI overflowed its stack in debug builds; it runs on a 16 MiB thread.

### Added
- **"All tools" hub**: the left panel (F9) lists the tools by task with a coloured dot per tool (Export,
  Edit, Create, Combine, Organize, Comment, Protect, Compress, Add a stamp); the five tools with no real
  function in SuniPDF are not shown. The Edit, Organize, Comment, Protect and Stamp tools open a contextual
  **tool bar** (grouped buttons, labels depending on the width, actions greyed out without a selection, "Leave
  tool" or Esc); the others open their dialog directly. **Tools** menu, `hub.*` entries in the Ctrl+K palette
  (6 commands in the registry, 69 in total), single-letter shortcuts that open the matching tool, active tool
  kept per tab.
- **Home**: large "Open file" card and a fully clickable card per tool (border in the tool's colour on
  hover); **toolbar**: quick access Open / Save / Undo / Redo and "All tools" button.

### Changed
- The floating tool palette on the page is replaced by the tool bar; the object bar only keeps the properties
  (stroke, fill, Apply); the Organize grid bar keeps the counter, "Select All" and the help. The text tabs
  Read / Annotate / Edit / Pages / Convert / Security of the toolbar disappear (the hub replaces them).
- Chrome polish: primary dialog button legible in the dark theme (contrast 5.2:1 instead of 2.5:1), tool
  glyphs contrasted on their dot (≥ 3.5:1, tested), hover fade, keyboard focus ring, softer shadows, faster
  tooltips, floating scrollbars, active tab underlined, more telling icons (combine, compress, split, extract,
  insert, redo, open).
- **Stamp / watermark** and **page numbering** (Pages and Edit categories): text placed on all pages or the
  current page, size, colour, bold, angle, opacity, 7 positions, `{n}` and `{total}` variables; each stamp
  remains an editable, undoable object. Engine `sunipdf-core::stamp`, CLI `stamp` and `page-numbers`,
  1 engine test.

### Fixed (tool hub review, 2026-10-14)
- **Esc with a menu open**: with an active tool, Esc left the tool and kept the menu open; it now closes the
  menu (or the drop-down list, the colour picker) and nothing else. Full order, one effect per Esc: menu,
  field or search, selected object, tool. Search also comes before the selected object, as the guide announced.
- Icons: "Rotate Left" (pages and objects) no longer reuses the "Undo" glyph; "Move Forward / Move Backward"
  use horizontal chevrons; "Edit Selected Text" no longer reuses the "Add Text" one (tested: no icon repeated
  within a bar nor with the quick access).
- The object properties bar stays within the document area and wraps in a narrow window instead of covering
  the right rail.
- "Leave tool" no longer appears in the Ctrl+K palette when no tool is active.
- The quick access buttons release focus after a keyboard activation (the single-letter shortcuts stayed
  silent).
- A file passed as a relative argument is recorded in the recent files with its absolute path.

### Fixed (adversarial code review, 2026-10-14)
- **Security**: editing the metadata, removing it, optimizing or sending an object to the back silently
  removed the encryption of a protected PDF; the original protection (key, passwords, permissions) is now
  reapplied. "Remove Protection" requires the owner password (any password passed on a document protected by
  an owner password only) and the application asks for it in a dedicated dialog. Restrictions without a
  distinct owner password are refused. The properties correctly report a protection by owner password only.
- **Data loss**: after "Save As", an edit then Ctrl+S overwrote the original file; "Split" and "Export as
  images" replaced existing files; the object selection survived a tab switch (Del acted on the other
  document); closing the window with a modified tab in the background re-saved the intact active document; an
  edit in flight during a tab switch lost the "modified" indicator.
- **Tabs**: clicking a tab from the home screen froze the application; opening a file during another opening
  created a ghost tab; an ongoing search stayed stuck when returning to the tab; thumbnails of a closed
  document could appear in the next one.
- **Undo**: undoing a "Remove Protection" failed and emptied the history (snapshots now keep their
  password); holding an arrow key on an object now creates only one entry.
- Keyboard shortcuts that require a document are ignored on the home screen (they opened a file picker).
- **CLI**: no existing output is overwritten without `--overwrite` (code 6); `render` always writes PNG
  atomically (a `.jpg` extension truncated the file); `rotate` refuses angles that are not multiples of 90;
  non-ASCII colour refused instead of a crash; negative angles accepted; duplicate pages processed once; typed
  lopdf errors (invalid format, I/O); page numbers in messages are 1-based; `optimize` never produces a file
  larger than the input.
- **Distribution**: the executables no longer depend on `VCRUNTIME140.dll` (static CRT,
  `.cargo/config.toml`) and start on a fresh Windows; the MSI installs the third-party licences and notices
  (17 files) and a LISEZMOI (readme) specific to the installation, and accepts reinstallation at the same
  version; `release.yml` runs the tests before packaging, is allowed to publish the Release, recomputes the
  MSI hash after signing and announces ARM64 as untested; the `ci.yml` artefact is the official package;
  `fetch-pdfium.ps1` checks the pinned hash and the build number; PowerShell scripts in UTF-8 with BOM
  (LISEZMOI legible under Windows PowerShell 5.1); `start.ps1` with `-j 2`; `pdfium.dll` is no longer looked
  up in the current directory in release builds.

## [0.3.0] — 2026-10-14

### Added
- **Redesigned dialogs** (common `dialog.rs` frame): header with icon dot, title and subtitle, button footer
  (primary action blue, destructive action red), shadow and rounded corners; applied to About, Unsaved
  Changes, Password, Note, Text, Split, Properties, Protect.
- **About**: logo, version, privacy, licences, credit "Created by Emanuel DE ALMEIDA -
  Navanem - www.navanem.com" (link).
- **Tools panel and home screen**: icons coloured by category (file, pages, annotation, editing, conversion,
  security), rows with highlighting, cards with coloured dots and shadow. The tools remain active without a
  document: they first open a file, then run.
- **Editing**: object rotation (±90°), duplication (Ctrl+Shift+D), bring to front (Ctrl+]) and send to back
  (Ctrl+[), keyboard nudging (arrows, Shift ×10), Esc to deselect, proportions kept for images (Shift to free
  them) and free for the other objects (Shift to lock), action buttons in the object bar.
- Engine: `rotate_object`, `duplicate_object`, `reorder_object` (content streams merged through lopdf so that
  the order of the object list is the drawing order); 1 extra rendering test.
- Engine tests serialized on a single PDFium engine (PDFium cannot be reinitialized within the same process).

### Fixed
- Window title not refreshed after closing a document.
- Dark theme: slightly more contrasted surfaces and borders.

## [0.2.0] — 2026-10-14

### Added
- **Page content editing** ("Edit" category, E/T/I/R/O/L tools in the palette): selection of an existing
  object (text, image, path) on click, drag to move, resize with corner handles, deletion (Del), editing the
  text of an existing object (F2 or double-click: text, size, colour), adding text (Helvetica / Times /
  Courier, bold, italic, size, colour), image insertion (PNG, JPEG, BMP, GIF, TIFF, WebP, 200 pt wide,
  proportions kept), rectangle / ellipse / line shapes with a preview while drawing.
- **Object properties bar** (stroke colour and width, fill): applies to new shapes and, through "Apply to
  object", to the selected object (text: colour).
- Engine `sunipdf-core::edit`: `page_objects`, `object_at`, `add_text`, `set_object_text`,
  `set_text_size`, `move_object`, `resize_object`, `set_object_style`, `delete_object`, `add_image`,
  `add_shape`; 2 integration tests with pixel-by-pixel verified rendering.
- `scripts/dev-screenshot.ps1`: several drags (`-Drag "a;b"`), keys after a drag (`-KeysAfter`), log
  (`SUNIPDF_LOG`).

### Fixed
- Real drag origin (press position, not the detection threshold position): moves and selections start exactly
  under the pointer.
- Line tool icon.

### Known limitations
- No object rotation, no embedded system fonts (14 standard fonts), edited text keeps its original font
  (missing characters not guaranteed). See `docs/KNOWN_ISSUES.md`.

## [0.1.0] — 2026-10-14

First release: reading (PDFium), tabs, thumbnails, search, text selection/copy, annotations (highlight,
underline, strike through, notes, Comments panel), page organization (rotation, deletion, insertion, moving,
grid view), merge / split / extract, images ↔ PDF, PNG export, optimization, bookmarks, properties,
AES-128/256 protection and removal, metadata, undo/redo, command palette, JSON CLI (20 commands), bilingual
fr-FR / en-US interface, light/dark themes, portable ZIP and x64 MSI.
