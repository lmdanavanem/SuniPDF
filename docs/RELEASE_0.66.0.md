# SuniPDF 0.66.0 — reviewed revision outlines

Development candidate. This page records implemented scope and outstanding qualification;
it does not announce a published package.

## Changes

- Compare one explicitly selected page pair from saved old and new PDFs, with manual or
  two-point alignment, bounded rendering and visible coverage limits.
- Review proposed regions as content changes, rendering noise or uncertain. Confirm selected
  classifications and preview the actual annotated page before saving a new copy.
- Retain editable native cloud annotations, source identities, selected settings, classification
  and annotation-register fields. Noise decisions remain reportable without an empty PDF edit.
- Revalidate source files before saving; refuse stale selections, changed sources, signed or
  restricted targets, output collisions and incompatible provenance transfers.
- Use the same core operation through `revision-outlines inspect`, `preview` and `apply`.
  Partial coverage remains explicit in JSON and CLI exit code 2.

[Workflow, CLI and limits](REVISION_OUTLINES.md).

## Qualification

Source, desktop integration and regression cases are written. Formatting, documentation links
and 1,752 English/French interface keys pass their static checks. Compilation, full regression
execution, native workflows and final-package qualification remain pending. No result from an
earlier release qualifies this candidate automatically.

This is a reviewed raster comparison of one page pair. It does not establish semantic plan
comparison, complete noise removal, automatic matching across an entire plan set or physical
measurement accuracy. Production signing and hardware qualification retain their separate gates.
