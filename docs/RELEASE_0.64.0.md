# SuniPDF 0.64.0

Status: **development candidate; not published**. This lot adds reviewed annotation transfer between
saved revisions and grouped navigation of local plan sets. Final-source validation, native workflows,
exact-source CI and package qualification remain release gates.

## Transfer supported annotations between revisions

Choose saved old/new PDFs, confirm physical page pairs and select supported native shapes. The app
prepares a separate candidate and requires review of every mapped destination page before saving a
new PDF copy. Optional two-point alignment supports uniform scaling and quarter-turn rotation.
The original revisions, destination page content, existing annotations and forms remain preserved.

The same operation is available through `revision-transfer inspect`, `preview` and `apply`. The CLI
binds recipes to both source fingerprints and the selected annotations, renders real candidate PNGs
and rebuilds the candidate before publication. Existing output paths are refused.

Plan-set entry points verify indexed source references and suggest unique discipline/number matches.
Every match needs confirmation. Unmatched and ambiguous sheets are identified for manual resolution;
revision names do not establish drawing equivalence or chronology.

The supported subset comprises native rectangles, ellipses, lines, arrows, polygons, polylines and
revision clouds with reproducible appearances. Measurement, price, review-thread and unsupported
semantic data cause an explicit exclusion. Source and destination permissions remain enforced;
signed destinations are refused and supported encryption retained. See [workflow and limits](REVISION_TRANSFER.md).

## Navigate indexed sheets

Filter by sheet metadata, filename and physical page. Group by discipline, revision or source file,
then sort columns in either direction. Numeric parts of identifiers sort naturally, so A-2 precedes
A-10. An issues filter retains unresolved references, and each view shows its filtered count.

Navigation uses stable sheet identities, regardless of display order. Filtering never rewrites the
manifest or discards a metadata draft. A hidden selection cannot open a different sheet accidentally.
See [local plan sets](PLAN_SETS.md).

## Integrity checks

Capture and transfer reject incoming review replies anywhere in the source document, including other
pages, with bounded traversal. Transferred annotations receive fresh identities while retaining their
supported style and author. A bounded transfer history remains readable at its limit and refuses
further additions; repeated source transfers need acknowledgement.

Changing either saved source or redirecting a selected filesystem alias invalidates preparation and
publication. Cancellation before commit creates no destination; a completed copy remains successful
if cancellation arrives after the atomic commit.

## Qualification

Final local formatting, workspace Clippy, build and the complete workspace suite passed:
994 test executions passed, none failed and 15 were ignored across 88 suite results. All 1,670
English/French translation keys were validated. The final CLI passed 14 independent process commands
and 94 output assertions, including rotated/cropped pages, retained destination content, styles,
fresh annotation identities, source changes, duplicate-transfer acknowledgement and refusal to
overwrite. Protected input files remained unchanged.

Native workflows and exact-source package qualification are still in progress. These local checks
do not qualify a published 0.64 package. Physical-platform, independent-reader, accessibility and
private-project document checks remain separate from automated structural and synthetic-corpus
coverage.
