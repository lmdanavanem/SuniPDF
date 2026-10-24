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

Zone filters now share the compact **Filters** menu in the annotation register and quantity table.
The button shows the active filter count, with their values available on hover. Assignment coverage
and totals are grouped under **Totals**; incomplete coverage keeps a visible warning on the button.

## Qualification in progress

The combined Windows development candidate passed formatting, Clippy, compilation and 1,045
workspace tests across 94 suites; 15 tests were ignored. Both language catalogs contain 1,789 keys.
Against that same candidate, the independent plan-link corpus passed 30 CLI commands and 329
assertions, and the revision-outline corpus passed 49 commands and 542 assertions.

The zone corpus and final native checks remain incomplete. These development results do not yet
qualify the release packages. Final source and binary identities, native results and package evidence
will be recorded when those checks finish.

The release pipeline also checks Windows and Ubuntu packages, independent PDF readers, code scanning
and upgrades from 45 portable and 43 MSI baselines, including 0.64.0. These are configured gates;
their successful completion and the public package verification are still pending.

## Continuing limits

The three backlog cards retain their broader acceptance criteria until the corresponding evidence
is complete. Packages remain unsigned, and ARM64 hardware, physical printers, human accessibility
and private project plans remain unqualified. Synthetic and licensed corpus results do not establish
universal interoperability, survey accuracy, PDF/A or PDF/UA compliance. English is the default
interface; detailed engine refusals may remain English in the French interface.
