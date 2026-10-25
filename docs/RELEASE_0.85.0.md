# SuniPDF 0.85.0

Development candidate; publication and package qualification are pending.

This cumulative release brings together the work since 0.79.0: rotated
dimensioned rectangles, multi-layer overlays, saved stamp capture, reusable
overlay setups, desktop plan-link workflow checks and explicit formula unit
conversion.

## Rotated dimensioned rectangles

Place a rectangle with real width, height and clockwise rotation around its
placed lower-left corner. The preview and saved result use the same corners;
width and height follow the rotated sides. The entire polygon must fit its
permitted page or calibrated region. Changing the angle requires a new preview.

The saved result remains a four-vertex area measurement, retaining rotation
through its vertices alongside quantities, Undo/Redo and explicit recalibration.
The CLI accepts `--rotation-degrees`; finite angles are normalized to the range
from zero inclusive to 360 exclusive. This adds rotated creation, without
permanent rectangle constraints or a separate post-save angle editor.

## Multi-layer overlays and reusable setups

Compare two to four saved PDF pages on a fixed base-page canvas. Each layer has
its own color, visibility and opacity, with manual or two-point alignment to the
base. The desktop shows the base and selected layer alongside the composite.
Changing a setting requires a fresh preview before export. PNG and JSON outputs
retain source identities and resolved alignments; the original two-page CLI
command remains available.

Save the reviewed overlay as a new local setup file, then reopen its pages,
styles and alignments. Every source hash is retained, including hidden layers.
Paths are relative when the source PDFs are below the setup's directory, so a
complete local bundle can move together. Missing, changed or retargeted sources
refuse Preview. Choosing a replacement explicitly clears that source binding
and alignment; replacing the base clears all alignments.

All sources remain subject to permissions, hash checks and modified-open-file
guards. Saved setups contain no credentials or environment references. Desktop
loading requires every source hash; the version-1 CLI manifest keeps its optional
hashes and password environment references. `revision-overlay-stack --format
setup` saves a clean setup from the verified preview. New outputs never replace
existing files.

The existing limits remain: two to four layers, 64 MiB per source PDF, a longest
image side of 2048 pixels, and 80 MiB for retained rasters plus the composite.
That raster budget is not a total process-memory limit. Setups embed no PDFs or
rendered images. Automatic registration and layered PDF output remain outside
this scope.

## Capture saved stamps and images

Capture supported native standard and image stamps already saved in a PDF,
together with supported shapes, as one reusable annotation-library group.
The placement workflow provides a rendered preview, fresh annotation identities
and one Undo step for the group.

Capture verifies the native appearance and resource graph. Standard stamps keep
their kind; accepted image pixels become a self-contained PNG with transparency
and already-applied opacity preserved. Provenance is supplied explicitly and
the source PDF remains unchanged. Changed fingerprints, missing copy permission
and unsupported selected members refuse capture. Existing capture/image budgets
and the version-1 library format remain unchanged. Saved FreeText and arbitrary
foreign appearances remain unsupported; revision transfer keeps its existing
supported annotation types.

## Desktop plan-link workflow and Ubuntu update checks

The Ubuntu qualification workflow exercises link creation, candidate review and
saving through the installed desktop controls, using original synthetic drawings
and a separate destination PDF. Source fixtures and an indexed plan manifest are
prepared separately. Retained source hashes and independent destination checks
support the saved results; navigation, cancellation and changed-source checks
remain separate cases.

The Ubuntu installer now verifies the private staged package's size and SHA256
before requesting public release metadata. Corrupt downloads fail locally. The
official release identity must still match before the DEB is parsed or installed;
a network refusal preserves the installation and removes the temporary copy.
Exact-source Ubuntu 22.04/24.04 package results remain pending. The qualification
environment uses X11, Openbox and software rendering; it does not establish
physical-GPU, Wayland or private-project behavior.

## Explicit formula unit conversions

Quantity formulas accept `convert(expression, unit)` for lengths, areas and
volumes. Convert a numeric field or an intermediate result to a literal unit
before combining it with another value. For example, with a quantity in `m2`
and a depth field in `cm`, `quantity * convert([Depth], m)` produces a volume
in `m3`.

Supported units are `mm`, `cm`, `m`, `in` and `ft`, with their square and cubic
forms. Conversion requires the same physical dimension; it cannot turn a number
without a unit into a measurement. Existing expressions keep their behavior,
and the declared source quantity unit still has to match exactly.

Conversions use checked integer arithmetic and exact unit ratios, then round
half away from zero at six decimal places. Values below that precision can round
to zero. This does not add arbitrary functions, implicit conversion, currencies,
mass or density calculations. Source fields, measured geometry and PDFs remain
unchanged.

The formula editor keeps a short example visible and groups detailed syntax and
rounding rules in an expandable section.

## Formula limits

There are at most eight formula columns. Each expression is limited to 512 UTF-8
bytes, 128 tokens and 32 parser nesting levels; column names are unique and at
most 64 bytes. The final precision is `0`–`6` decimal places, and the absolute
value of each intermediate result cannot exceed one trillion. Every conversion
rounds at six decimal places before the column's final rounding, so repeated
conversions need not recover the original value exactly.

The target unit is a case-sensitive literal. Counts, angles, dimensionless
values and conversions between different dimensions are refused. Missing or
invalid inputs retain explicit cell errors and incomplete subtotal coverage.

## Qualification

Exact-source tests and package results are pending. The planned CLI corpus has
**77 commands and 49 cases**, with **106 formula vectors** in fifteen JSON/CSV
batches. It covers all 75 ordered same-dimension pairs among the fifteen length,
area and volume units, every final precision from zero to six, signed values,
zero, tiny values, intermediate limits, nested rounding, explicit errors and
the existing formula behavior. These counts describe the producer's plan;
they are not a claim that a packaged run has passed.

The corpus creates an original synthetic PDF, then uses the CLI to add a count,
a calibrated length, an angle and typed numeric fields. Independent rational
unit ratios and decimal half-away-from-zero rounding supply the expected values.
It checks JSON and CSV rows and subtotals, the issue report model and actual PDF
text, and exact values in a formula-free XLSX workbook and its refresh manifest.
Twenty-three malformed configurations, six existing-output guards and three
stale-source or stale-row guards must refuse while preserving the original files.
Incomplete issue reports and workbooks keep their explicit coverage and CLI exit
code `2`; that outcome is distinct from a refused operation.

Windows CI runs the corpus against its selected QA executable. The tagged x64
release workflow runs it against the CLI inside the actual portable package.
Both retain `formula-unit-conversion-evidence`, including observed check and
case counts, command arguments, bounded stdout/stderr, executable and producer
hashes, source identities and output hashes. The producer requires a new evidence
directory and bounds the run to 240 seconds, each process to 30 seconds, each
transcript stream to 2,000,000 bytes and each retained file to 32 MiB.

Formula configuration remains a report setting. This synthetic corpus does not
establish native desktop interaction, external spreadsheet recalculation or
private-plan accuracy. Those and broader reader qualification retain their
separate scope.

[Plan workspace](PLAN_WORKSPACE.md) · [Revision overlays](REVISION_OVERLAY.md) ·
[Annotation library](ANNOTATION_LIBRARY.md) · [Plan links](PLAN_LINKS.md) ·
[Quantity tables](QUANTITY_TABLE.md) · [Roadmap](ROADMAP.md).
