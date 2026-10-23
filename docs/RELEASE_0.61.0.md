# SuniPDF 0.61.0

Status: **release candidate; qualification and publication pending**. This page describes implemented scope,
not successful test execution. [Project status](PROJECT_STATUS.md) records verified results when available.

## Custom-field sorting

The annotation register can sort by a selected custom text field. The quantity table sorts its separate typed
number, date, text and choice fields using their actual types. Missing values and blank text remain last in
both directions; physical page/index resolves ties. The desktop selector and CLI `--sort-custom NAME` share
the same ordering, and exports retain the selected configuration.

[Register workflow](MARKUP_REGISTER.md) · [Quantity tables](QUANTITY_TABLE.md)

## Reuse saved annotation groups

**Annotation library → Capture from current page** lets users explicitly select 1–16 eligible saved annotations,
declare their source and save a portable group. Native rectangles, ellipses, lines/arrows, polygons, polylines
and clouds retain relative displayed-page geometry and supported stroke/fill/dash/opacity styles.

Capture checks copying permission, source fingerprints and reproducible appearances. Unsupported or changed
members refuse the complete operation; originals remain unchanged. Placement uses the existing rendered
preview, new annotation identities and one Undo step. CLI inventory/capture commands use the same checks and
write a new version-1 library JSON file.

Existing text, image/stamp, foreign/custom, reply and extra-semantic annotation records remain outside this
capture path. New-draft templates retain their previously supported types. [Workflow and limits](ANNOTATION_LIBRARY.md).

## Report formula columns

The quantity table adds up to eight calculated report columns. Expressions accept decimals, numeric custom
fields, `quantity`, parentheses and `+ - * /`. Quantity inputs require an exact declared unit; numeric fields
are dimensionless. Checked arithmetic uses six decimal places per operation, then the selected result precision,
rounding half away from zero.

Missing inputs, invalid units, division by zero and overflow remain explicit errors with null output values.
JSON/CSV carry definitions, values, units, errors and formula-evaluation completeness. Configuration is tab-local;
copy/load its JSON to reuse it, or pass `--formulas FILE` in the CLI. Formulas do not mutate PDF metadata, prices
or existing subtotals. [Examples and bounds](QUANTITY_TABLE.md#report-formula-columns).

## Qualification scope and remaining work

Lossless optimization preserves the existing compression state of Metadata/XML streams.
This corrects a PDF/A-1 regression found by the new corpus: adding a compression filter to
unfiltered catalog XMP can invalidate a previously conforming file. Existing invalid metadata
is not repaired, and this change makes no claim of automatic PDF/A conversion.

Regression additions cover typed sorting, formula arithmetic/error/export behavior, supported saved-group
roundtrips on rotated/cropped pages, stale selections, copy restrictions and source preservation.
Remote CI and corpus results must be verified before publication; no passing counts are claimed here.

The corpus/conformance work uses independent validators and reader checks for supported workflows.
Any recorded PDF/A or PDF/UA result applies to the tested file and operation, not universal application
conformance. SuniPDF does not gain a PDF/A conversion tool or PDF/UA tagging editor from these checks.

Private real-project plans, physical devices, installer signing and broader accessibility/interoperability
qualification remain separate [1.0 requirements](RELEASE_1.0_CRITERIA.md). Formula subtotals, unit-bearing custom
fields, automatic symbol recognition, arbitrary annotation capture and geometric area/volume tools remain
tracked in the [roadmap](ROADMAP.md).
