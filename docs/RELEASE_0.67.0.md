# SuniPDF 0.67.0

Development qualification is in progress. The published download remains [0.64.0](RELEASE_0.64.0.md).
This release combines reviewed plan links, revision outlines and rooms/zones in one distribution.
Intermediate local builds are not separate published releases.

## Reviewed links between plan sheets

Find exact indexed sheet identifiers in existing searchable PDF text. Review source occurrences and
real destination previews, resolve ambiguous targets explicitly, and publish standard relative PDF
links in a new copy. Navigation verifies the selected manifest and target fingerprint. Protected
inputs, source changes, unsafe paths, existing annotations and output collisions retain explicit guards.

The scope is one indexed source PDF, up to 32 selected pages and 512 occurrences. OCR must already
exist. Multi-source batch orchestration and arbitrary external-reader behavior remain separate work.
See [Plan links](PLAN_LINKS.md).

## Reviewed revision outlines

Compare an explicit old/new physical page pair with a manual transform or two point pairs. Classify
proposals as content changes, rendering noise or uncertain; confirm each selected decision and inspect
the actual annotated candidate before saving. New clouds remain editable and carry source/region
provenance in the register. The revised PDF's other pages and original contents are preserved.

Raster comparison is bounded to 1,536 pixels on its longest side, 512 proposals and 64 selected
decisions. Filtered, omitted, unreviewed and unmapped areas remain visible. Noise-only reviews do not
publish empty copies. A partial report is not proof of full drawing equivalence. See
[Revision outlines](REVISION_OUTLINES.md).

## Rooms and zones

Define named rectangular zones with optional parent zones on the actual page. Preview annotation
assignments, apply one undoable edit and save the metadata in the PDF. Register and quantity filters
share the same full-bounds assignment and coverage rules. Boundary, overlap, unknown geometry and
stale page evidence cannot silently disappear into a filtered total.

The limits are 64 zones per page and eight hierarchy levels. Zones do not infer room area or change
measurement calibration. JSON replacement and CLI inspection/preview/new-copy publication share the
core. See [Rooms and zones](PLAN_ZONES.md).

## Qualification in progress

The isolated plan-link lot passed 1,013 workspace tests (15 ignored), 30 independent CLI commands
and 329 assertions, plus native English/French save/reopen/navigation and refusal checks. Independently
inspected native outputs passed 95 checks each; a final overlapping variant passed 96 checks and is
not added to those counts.

The isolated revision-outline lot passed 1,029 workspace tests (15 ignored), formatting, Clippy and
builds, plus 49 independent CLI commands and 542 assertions. Native interaction checks are in progress.
The combined room/zone source and its tests are written but have not yet passed the final workspace,
native and package gates. Earlier build results do not qualify the combined 0.67 binaries.

## Continuing limits

The three backlog cards retain their broader acceptance criteria until the corresponding evidence
is complete. Packages remain unsigned, and ARM64 hardware, physical printers, human accessibility
and private project plans remain unqualified. Synthetic and licensed corpus results do not establish
universal interoperability, survey accuracy, PDF/A or PDF/UA compliance. English is the default
interface; detailed engine refusals may remain English in the French interface.
