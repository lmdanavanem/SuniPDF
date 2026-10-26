# SuniPDF 0.91.0

Published stable on 2026-10-26. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.91.0) | [Qualification evidence](qa/RELEASE_0.91.0.json).

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

The installed-Ubuntu qualification creates and reviews two linked source
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

## Qualification and limits

The tagged source passed all 11 release jobs. Windows qualification includes
20 offscreen interface scenes (3 Home and 17 PDF/settings scenes), 57 portable upgrade baselines plus
4 guards, and 55 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 27 installed runtime cases, including 6 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-26T04:09:34.597549+00:00.

Visual review covers 36 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.91.0.json).

The ten retained packaged-CLI corpora include register-field batches with 113 checks, 35 commands, 18 cases and 16 refusals. Native plan-link creation separately passed five cases and 32 retained scenes per Ubuntu version; the six navigation cases retain their separate scope.

Four additional Windows WARP scenes cover English and French register dialogs: two real batch previews and two saved-filter views. Their recorded source bindings, control states, text geometry and pixels were checked independently.

The new register-batch corpus covers 113 checks, 35 commands, 18 cases and 16 refusals; the saved-text corpus covers 388 checks, 45 commands, eight cases and four refusals. Actual plain PDF graphs, saved text glyph mappings and rendered pixels are independently checked. Encrypted register output retains independently inspected QPDF objects and permission dictionaries; ephemeral passwords are not retained for a second independent decryption. Four additional offscreen dialog scenes qualify recorded rendering and control states, not native mouse/keyboard, Undo/Redo interaction or physical accessibility. Exact field filtering adds no numeric ranges or regular expressions and has no separate packaged corpus. Foreign, embedded or arbitrary text fonts, unsupported semantics and incompatible text-box geometry remain refused. All affected backlog cards retain their remaining acceptance criteria. Rectangle rotation does not create persistent geometric constraints. Saved annotation capture retains its supported native appearance and image bounds. Overlay setups retain source hashes and configuration, not PDF data, rendered rasters or credentials; automatic registration and layered PDF output remain open. Formula conversion supports only the existing geometric unit families; each conversion rounds to six decimal places and sub-micro values can round to zero. Source quantities keep their exact unit guard. Every affected backlog card retains its remaining acceptance criteria. Circles and ellipses use 128 editable polygon vertices, about 0.0402% below ideal analytic area before coordinate rounding. Segments use two stored endpoints and linear quantities; none of these shapes has a persistent analytic constraint. The ellipse corpus verifies 327 checks, 62 commands, 27 cases and 19 refusals across eight PDFs/1024 vertices. The segment corpus verifies 359 checks, 67 commands, 29 cases and 20 refusals across nine PDFs/18 endpoints. Both retain actual PDF/QPDF, appearance and stored-quantity checks across four cropped rotations and five units. No new native shape mouse/keyboard, Undo/Redo, physical survey or external-reader qualification is claimed. Independent X/Y calibration, permanent constraints, analytic curves and broader interoperability remain open; AEC-008 remains In progress.

Installed Ubuntu creation qualification uses original synthetic searchable PDFs and actual X11/Openbox/Xvfb controls. It verifies two reviewed source copies, independent QPDF page/content/resource checks, an exported save receipt, occupied-output refusal and navigation to physical destination page two. The separate six-case navigation corpus retains its recorded scope. Image-only references, encrypted creation inputs, dirty-tab creation, Wayland, physical GPUs and external-reader interoperability remain unqualified. Windows offscreen scenes do not establish native plan-link creation.

Windows packages remain unsigned, and Windows ARM64 is cross-built without
native hardware execution. The Apple Silicon app is ad hoc signed, not
notarized; macOS 12 is a binary-checked deployment minimum. Physical Macs,
interactive Finder opening and clipboard interoperability remain unqualified.
Ubuntu updater-helper checks do not establish graphical authorization or a
complete desktop updater interaction. Physical GPUs, Wayland, printers and
human accessibility remain separate gates.

The corresponding annotation-workflow backlog card remains in progress; full acceptance needs its remaining independent criteria.

[Bulk editing](REGISTER_BATCH.md) | [Saved filters](REGISTER_FILTERS.md) | [Annotation library](ANNOTATION_LIBRARY.md) | [Project status](PROJECT_STATUS.md).
