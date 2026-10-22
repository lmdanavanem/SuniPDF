# SuniPDF 0.49.0 — add and remove saved measurement vertices

[Français](RELEASE_0.49.0.fr.md) · [Geometry editing guide](MEASUREMENT_GEOMETRY.md)

This build includes the scope below. Published assets and qualification are tracked on the [0.49.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.49.0).

## Changes

- Insert a vertex on a saved polyline, perimeter or polygon-area segment, including a polygon's closing edge. Remove a selected vertex within the shape's minimum and maximum counts.
- Select vertices by number, including points at the same position, and retain the existing on-page dragging workflow.
- Inspect Preview, adjust the draft and Apply with one Undo step. Preserve the measurement's type, calibration, unit, precision, identity, replies and entered quantity fields; measured values and compatible price totals follow the accepted geometry.
- Keep length and angle point counts fixed, refuse invalid or stale edits, and preserve the original page defaults and other measurements.
- Include the published 0.48.0 Windows packages in portable and MSI upgrade regressions.

## Qualification

Qualification records and distribution assets are tracked on the [0.49.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.49.0). Results apply to the exact source identified there.

## Scope

Polylines retain at least two vertices; perimeters and polygon areas retain at least three. These shapes allow at most 128 vertices. Length measurements keep two points and angles three. The existing CLI accepts explicit geometry replacement; this release adds no new CLI command.

Vector snapping, batch recalculation, multiple-scale viewports and real-plan qualification remain open. AEC-002 and AEC-003 remain partial. Packages remain unsigned; native ARM64, hardware and accessibility qualification remain separate from automated checks.
