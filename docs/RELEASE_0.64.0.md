# SuniPDF 0.64.0

Status: **published as stable Latest on 2026-10-24**.
[Download packages](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.64.0) ·
[Qualification evidence](qa/RELEASE_0.64.0.json).

This release adds reviewed annotation transfer between saved revisions and grouped navigation of local plan sets.

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
revision outlines with reproducible appearances. Measurement, price, review-thread and unsupported
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

Release `422528678` was published at `2026-10-24T05:42:14Z` from
`71854ef09886c94519775c345de1b45078c9d706` through [PR #41](https://github.com/lmdanavanem/SuniPDF/pull/41).
Its tree matches the qualified local source. The final local Windows suite passed 994 tests,
with 15 ignored and no failures across 88 suites; formatting, Clippy and desktop/CLI builds passed.
The final local transfer CLI corpus passed 14 commands / 94 independent output assertions.

The exact release source passed all four workflows. Windows recorded 994 passed tests
and 15 ignored results; Linux recorded 974 passed and 10 ignored per run,
with no failures. All 1,670 English/French keys matched. Portable upgrades covered
44 baselines and 4 guards (48 scenarios); MSI lifecycle checks covered
42 baselines and 210 logs. Ubuntu 22.04/24.04 installation and the pinned conformance corpus passed.

Eight public assets and checksum sidecars were downloaded anonymously and verified against GitHub digests.
The downloaded Windows CLI passed 82 process commands and 1,062 assertions with its bundled runtime.
Its three corpora cover provenance/quantities, plans/forms/volumes and revision transfer. Packaged files,
test helpers and protected inputs remained unchanged. The DEB's embedded CLI matched both conformance runs;
that additional payload inspection executed no program. See [machine-readable evidence](qa/RELEASE_0.64.0.json).

The final development build also passed native English/light and French/dark workflows: natural sorting,
filtering, grouping, physical-page navigation, explicit matches, both destination previews, four transferred
annotations over two pages, Save new copy and reopening. Ambiguous/unmatched sheets remained explicit;
closing a candidate wrote no output, and changing a scratch destination after preview refused save.
Independent inspection of the saved native output passed 27 assertions with qpdf and pypdf.

These native checks used a development build with an explicitly supplied PDFium runtime. They do not
establish distributed-GUI execution or local installer qualification. Dirty-tab guards, cancellation races
and two-anchor alignment retain automated/CLI evidence; they were not repeated in the native run.
Detailed engine refusals can remain English inside the French interface.

Code scanning has zero open findings after individual review: two generated integration-test credentials
were classified as used in tests, and one full-PDF SHA256 identity digest was classified as a false positive
for password hashing. Exact source, locations and call paths were reviewed. The original failed aggregate
checks remain in history; scanners and repository rules were not disabled.

## Remaining limits

Windows packages remain unsigned. Physical ARM64, printers, production code signing, independent
update authenticity, human accessibility and private-project plans remain unqualified. Ubuntu checks use
containers and software rendering. Specified synthetic/licensed corpus results do not establish universal
PDF/A, PDF/UA, external-reader compatibility or survey accuracy. Ignored tests are not passed checks;
separate runs and earlier failures are not added to the final counts.
