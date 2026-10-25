# SuniPDF 0.89.0 - candidate

This cumulative candidate combines reviewed bulk edits, saved register filters,
reusable rich-text annotations and exact custom-field filtering. It follows
0.85.0; the intermediate development versions are not separate public releases.
Publication and package qualification are pending.

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

## Qualification scope

Source tests cover atomic changes, stale selection, settings persistence,
serialization compatibility, literal filtering and preserved source files. The
packaged CLI corpora independently inspect saved annotation dictionaries,
appearance streams, standard fonts, mixed groups, rotated/cropped geometry and
rendered pixels. Four additional Windows offscreen scenes exercise the bulk-edit
review and saved-filter dialogs in English/light and French/dark layouts.

These are the planned validation gates. Exact-source runtime results and public
packages remain pending. Offscreen scenes do not qualify native mouse, keyboard
or assistive-technology interaction. Existing hardware, interoperability and
private-plan limits remain open.

[Bulk editing](REGISTER_BATCH.md) · [Saved filters](REGISTER_FILTERS.md) ·
[Annotation register](MARKUP_REGISTER.md) · [Annotation library](ANNOTATION_LIBRARY.md) ·
[Project status](PROJECT_STATUS.md)
