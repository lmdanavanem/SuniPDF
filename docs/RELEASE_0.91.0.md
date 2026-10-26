# SuniPDF 0.91.0 - candidate

This cumulative candidate combines the plan-review changes since 0.79.0.
Development increments 0.80 through 0.90 are not separate public releases.
Publication and package qualification are pending.

## Rotated dimensioned rectangles

Place a rectangle with real width, height and clockwise rotation around its
lower-left anchor. Preview and saved geometry share the same corners; the entire
polygon must fit its page and selected scale region. Changing the angle requires
a new preview. The saved result is an ordinary editable four-vertex area
measurement, without permanent rectangular constraints.

## Compare multiple plan layers

Compare two to four saved PDF pages on a fixed base-page canvas. Each layer has
its own color, visibility, opacity and manual or two-point alignment. Review the
individual pages and composite before exporting PNG and JSON to new files.
Changed inputs or settings require a fresh preview. The existing two-page CLI
remains available.

## Capture saved stamps and images

Capture supported native standard and image stamps together with supported
shapes as a reusable local annotation-library group. Verified image pixels become
a self-contained PNG, retaining transparency and already-applied opacity.
Placement provides a preview, fresh annotation identities and one Undo step.
Changed fingerprints, missing copy permission, unsupported members and foreign
appearances refuse capture. The original PDF remains unchanged.

## Save reusable overlay setups

Save a reviewed overlay's pages, colors and alignments to a local setup file.
Every source hash is retained, including hidden layers. Relative paths support
moving a complete local bundle together. Missing, changed or retargeted inputs
refuse Preview; explicitly replacing a source clears its binding and alignment.
Replacing the base clears all alignments.

Setups contain no PDF data, rendered rasters or credentials. Limits remain two
to four layers, 64 MiB per source, a longest image side of 2048 pixels and 80 MiB
for retained rasters plus the composite. This is not a total process-memory cap.
Automatic registration and layered PDF output remain outside this scope.

## Desktop plan-link workflow and Ubuntu update checks

The planned installed-Ubuntu qualification creates and reviews two linked source
PDFs through native controls, saves new copies, exports the actual receipt and
reopens the physical destination page. It retains occupied-output refusal and
independent PDF content, resource and link checks. Its original synthetic
drawings do not establish behavior on private project plans.

The Ubuntu updater verifies the staged package's size and SHA256 before
requesting public release metadata. Corrupt downloads fail locally; the official
release identity must still match before parsing or installation. Network refusal
preserves the installation and removes the temporary copy.

## Explicit formula unit conversions

Use `convert(expression, unit)` to explicitly convert compatible lengths, areas
or volumes before combining values. For example, an area in `m2` multiplied by
`convert([Depth], m)` can use a depth field declared in `cm`. The declared source
quantity unit must still match exactly; there is no implicit conversion.

Conversions use exact unit ratios and checked integer arithmetic, then round
half away from zero at six decimal places. Sub-micro values can round to zero,
and repeated conversions need not recover the original value exactly. Existing
formula limits and explicit cell errors remain. Counts, angles, dimensionless
values, currencies, mass and density are outside these geometric conversions.
JSON, CSV, issue-report PDF and workbook outputs share the same calculated values.

## Review and edit annotation fields together

Select up to 256 annotation-register rows, then keep, set or clear their business
status, discipline and individual custom text fields. Inspect every before/after
value and confirm the review before applying. The complete batch is one Undo/Redo
operation. Stale, locked or invalid rows refuse the entire operation.

Selection is explicit: filtered or hidden rows are never selected implicitly.
Changing filters clears the selected rows and pending preview. The operation
preserves page contents, appearances, PDF review states and typed quantity fields.

The CLI provides the same bounded operation through `annotation-fields-batch
inspect`, `preview` and `apply`. Source-bound previews detect changed files or
annotations. Application saves a new copy and refuses an existing output.

## Save and exchange useful filters

Save the current review criteria and sort order with a name, explicitly apply a
saved filter, or exchange a local library through JSON import/export. Applying
a filter preserves the current page and zone scope, clears the bulk-edit
selection and preview, and never selects rows automatically. It does not change
the PDF.

Imports validate completely before modifying the library. Duplicate names and
existing output files are refused. The library is bounded to 16 filters and
128 KiB. Older settings retain their existing behavior.

## Filter by an exact custom text value

Combine a named text field, such as `Room = A-101`, with existing review, author,
page and zone criteria. Values are literal and case-sensitive. Leading or trailing
spaces remain significant, and an empty stored value differs from an absent
field. Clearing the criterion restores the previous broader selection.

The annotation register, quantity table and their selection-bound reports share
the criterion. The CLI accepts `--field-equals KEY=VALUE`; `KEY=` selects a stored
empty value. This does not add numeric ranges or regular expressions.

Saved filters containing this criterion use format version 2. Version 1 remains
supported and keeps its existing representation when the criterion is absent.

## Reuse saved rich-text annotations

Capture supported native rich-text annotations using the twelve standard
Helvetica, Times and Courier font variants, alone or with supported shapes and
stamps. Text, font, color, border, fill, opacity and displayed-page geometry pass
through the existing version-1 annotation library. Each placement receives fresh
annotation identities and one Undo step.

Capture compares the saved appearance and resources with a regenerated native
appearance. It describes the current reproducible appearance, not an earlier font
or edit history. Foreign or embedded fonts, altered appearances, legacy simple
text and unsupported semantics remain refused. Geometry that would change text
box dimensions or wrapping during normalization or placement is also refused.

## Draw circles and ellipses to a measured size

Choose a centre and real diameter, or two diameters and a clockwise angle, using
a captured calibration, saved page scale, named region or automatic reference
resolution. Review the prepared PDF preview before applying one undoable area
measurement. Edited parameters or source changes invalidate the prepared result.

The saved contour contains 128 editable vertices. Its approximation is explicit
in the interface; quantities use the actual stored polygon area. It does not
create analytic curves, permanent shape constraints or independent horizontal
and vertical scales. The CLI provides the same guarded construction through
`dimensioned-circle` and `dimensioned-ellipse`, writing new copies only.

## Draw a segment to a measured length

Choose a starting point, physical length and clockwise angle. The indicative
outline is an open line. Review the prepared PDF preview, then apply one undoable
length measurement. The same captured, page, region and automatic references
retain their source and geometry guards.

The saved line has two editable endpoints. Quantities use their actual stored
distance; coordinate-precision checks refuse a change of the displayed value
during storage. Save/reopen and subsequent endpoint edits do not impose a
permanent length or angle constraint. Architectural extension lines and dimension
arrows remain outside this increment. `dimensioned-segment` uses the same
constructor and refuses existing output paths.

## Qualification scope

Ten packaged CLI corpora are required. The planned gates include source tests
for atomic changes, stale selections,
settings persistence, literal filtering, geometry, permissions and source
preservation. Separate packaged CLI/QPDF corpora inspect actual annotation
dictionaries, complete appearance paths, standard fonts, mixed groups, calibrated
lengths and areas, rotated/cropped pages and rendered pixels. Four Windows
offscreen scenes exercise the bulk-edit review and saved-filter dialogs in
English/light and French/dark layouts.

Exact-source runtime results and public packages remain pending. Offscreen scenes
do not qualify native mouse, keyboard or assistive-technology interaction. The
shape corpora do not establish native placement, external-reader measurement
editing or physical survey accuracy. Existing hardware and private-plan limits
remain open.

[Bulk editing](REGISTER_BATCH.md) | [Saved filters](REGISTER_FILTERS.md) |
[Annotation register](MARKUP_REGISTER.md) | [Annotation library](ANNOTATION_LIBRARY.md) |
[Circles and ellipses](DIMENSIONED_ELLIPSES.md) | [Segments](DIMENSIONED_SEGMENTS.md) |
[Project status](PROJECT_STATUS.md)
