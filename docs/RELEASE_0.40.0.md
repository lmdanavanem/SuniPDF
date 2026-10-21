# SuniPDF 0.40.0

## Changes

Local length, polyline, perimeter, simple-polygon area and angle annotations with explicit two-point calibration, metric/decimal imperial units, precision and JSON records. Each measurement embeds its own scale and geometry; preview/Undo and copy-only CLI output preserve originals. Image Stamp annotations embed aspect-fitted pixels and alpha. Text watermarks/numbering now preflight all pages and refuse invalid margins or overflow atomically. Published 0.39.0 packages join upgrade regressions. A 26-card AEC backlog prioritises local workflows; cloud and collaboration are deferred at the owner's request, with prices undecided.

[Measurements](MEASUREMENTS.md) · [Image stamps and decorations](PAGE_DECORATIONS.md) · [AEC backlog](AEC_BACKLOG.md).

## Validation

Final qualification will be recorded after exact-source CI and package checks on the published revision.

## Limits

- Measurement vertices and calibration are entered as displayed-page coordinates in Rich annotations. Maximum 128 vertices and 0–4 decimal places; simple polygons only. Feet/inches are decimal units.
- Each annotation has its own calibration. Recreate it to change geometry or scale; generic move/resize/label edits are refused to prevent stale takeoffs. No page-wide recalibration, vector snapping, viewport scales, volume, symbol recognition or automatic counting is claimed.
- Measurement JSON records use a documented SuniPDF format; Bluebeam proprietary measurement compatibility and universal imported-metadata support are not claimed. The AEC backlog acceptance criteria remain open where incomplete.
- Image stamps are limited to 64 MiB encoded, 4096 pixels per side and four million pixels. Embedded pixels are independent of the source image after Apply. No digital-signature or identity verification is implied.
- Local tools remain free and open source. Cloud/collaboration implementation, deployment and billing are deferred; no prices are defined. Projects edits remain deferred.
- Windows packages remain unsigned without an independent publication key; ARM64 is cross-built only. Native hardware, printing, Narrator, installed updater interaction, broader hostile/customer corpus and PDF/A/PDF/UA conformance remain separate 1.0 qualification.
