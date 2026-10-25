# SuniPDF 0.91.0 - candidate

This cumulative candidate adds reviewed bulk edits, saved register filters,
reusable rich-text annotations, exact custom-field filtering and dimensioned
circles, ellipses and straight segments. It follows 0.85.0; intermediate
development versions are not separate public releases. Publication and package
qualification are pending.

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
a filter preserves the current page and zone scope. It changes neither the PDF
nor the selected bulk-edit rows automatically.

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

The planned gates include source tests for atomic changes, stale selections,
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
