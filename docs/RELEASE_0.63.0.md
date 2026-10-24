# SuniPDF 0.63.0

Status: **candidate; local source and CLI checks passed, not published**. This candidate combines local
plan sets, bounded FDF value exchange, deducted areas and constant-depth volumes. Final native checks,
exact-source CI, package qualification and publication remain pending. Results recorded for 0.62.0 or
earlier releases do not qualify this candidate.

## Local plan sets

Open **Tools → Organize pages → Plan sets** to create or open a local JSON manifest, add selected PDFs,
edit sheet numbers, titles, disciplines and revisions, and validate references. Each file reference retains
its byte length, SHA256 and page count; sheets retain stable identities and explicit physical page numbers.
Missing, changed, protected, duplicate and out-of-range references remain visible.

**Save new manifest copy** writes a new file and rebases references only when all sources remain within
the destination's directory tree. PDFs stay separate and unchanged. The workflow accepts at most 64 files,
1,024 sheets and 64 MiB per source PDF. A password can be supplied for one operation and is never stored
in the manifest. A matching clean tab can be reused; unsaved tabs require an explicit choice, and their
different page counts prevent an indexed jump. A separate verified copy preserves existing tab edits.

The CLI provides creation, inspection, validation, metadata editing, explicit refresh/relink, removal and
manifest export. Refresh retains sheet metadata and physical page references; users must review them
after a revision changes page order. Automatic revision matching, sheet replacement, annotation transfer
and cross-document link creation remain separate backlog items. [Workflow and limits](PLAN_SETS.md).

## FDF form values

The Forms tool and CLI export supported text, checkbox, radio and choice values as FDF alongside JSON
and XFDF. Qualified names and Unicode text are retained. Checkbox and radio appearance-state names stay
distinct from text export values during import, so destination field definitions determine the mapping.

Imports and exports enforce shared size and expansion bounds. Ambiguous selected button states,
unsupported structures, invalid fields and invalid values are refused. Password fields, signatures,
attachments, actions and scripts are excluded from value export. An FDF value file does not contain the
source PDF or its annotations. [Form value exchange](FORM_DATA.md).

Forms requesting an appearance refresh retain their button states through display, repeated imports and
other edits. Display uses a temporary copy; explicit appearance writes commit only after successful rendering.
Restricted and signed source files remain unchanged. The bounded copy path has a per-region preparation cost;
see the form guide for size limits and qualification scope.

## Deducted areas and volumes

In **Measurements**, choose **Polygon area** or **Volume**, confirm the scale and draw the outer contour.
Use **Add deduction** for each interior ring. Select a contour to edit or undo its points, or remove a
deduction explicitly. A volume uses an entered physical depth and its own unit. The panel shows gross,
deducted and net area, then volume. Preview and Apply use the complete geometry; one Undo restores the
previous annotation and metadata.

The geometry editor selects the outer contour or an individual deduction. Up to 16 holes, 128 vertices
per contour and 512 vertices in total are supported. Holes must be strictly interior, with no touching,
intersections, overlaps or nesting. Scale selection uses the outer boundary; holes cannot hide an
ambiguous scale region. Invalid geometry and excessive storage precision loss are refused atomically.

Result units and physical depth units are independent. Compatible unit changes preserve the depth input
and explicit unit-price basis. Quantity tables support `mm3`, `cm3`, `m3`, `in3` and `ft3`; incompatible
prices or pricing overflow refuse the edit. Only current measurements contribute usable quantities,
costs and quantity-dependent formulas. Historical values remain visible and require explicit recalibration.

Saved deducted areas and volumes use measurement metadata version 3. Ordinary records retain version 2,
and older readable records retain their existing validity rules. The CLI can create and edit these
measurements in new PDF copies with expected-record guards. [Calculations and limits](MEASUREMENT_VOLUMES.md).

Small preview images use filtered reduction so thin contours remain visible. The rectangle controls remain available in every measurement mode except Volume; an active rectangle
placement keeps them visible even in Volume mode. This leaves the volume and deduction controls easier to reach.

## Candidate qualification

Local checks completed on 2026-10-24 against the final candidate source:

| Check | Recorded result |
|---|---|
| Windows workspace | 963 passed, 0 failed, 15 ignored |
| Formatting, workspace Clippy and desktop/CLI build | Passed |
| Whole-process CLI corpus | 20 commands, 680 assertions, 0 failures |
| Final native workflow checks | In progress |
| Exact-source CI and package/install/update qualification | Pending |
| Publication and downloaded-package verification | Pending |

The source fingerprint stayed unchanged during the final pipeline:
`74e6ad2caf2cf54172d0ab48ee6201e092f0c5bcf15c3bdb4550901a6d874964`.
The CLI corpus used SHA256 `750e713e2bf81c52cfadc22ce2498e6a466bafce7cda3efac8b8f44c72bf5583`
and confirmed its hash remained unchanged. These are local development-build checks, not public-package evidence.
Ignored tests are not passed checks, and failed attempts or separate runs are not added to the final total.

Regression coverage includes reference identity and relocation, protected PDFs, typed FDF values and
parser bounds, area/volume calculations, topology refusals, rotation/crop, prices, stale previews,
Undo/Redo and save/reopen. The CLI corpus additionally checks repeated FDF import, plan manifests,
volume pricing/unit conversion, stale exclusions, no-overwrite behavior and independent output inspection.
Final native checks and actual package, installation, update and download verification remain release gates.

AEC-009 and AEC-015 remain **In progress**. Constant-depth polygon volumes do not provide variable-depth
solids or automatic opening detection. Plan-set metadata does not establish revision equivalence.
Real-project A0/A1 qualification, physical printing and measurement checks, independent update authenticity,
code-signing and the remaining accessibility/interoperability gates stay open.
[Backlog](AEC_BACKLOG.md) · [1.0 criteria](RELEASE_1.0_CRITERIA.md).
