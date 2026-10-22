# SuniPDF 0.49.0 — add and remove saved measurement vertices

[Français](RELEASE_0.49.0.fr.md) · [Geometry editing guide](MEASUREMENT_GEOMETRY.md)

Status: published as stable Latest [0.49.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.49.0), source `a95774d56c3b9ce02a9743431a5d0c2b079b330e`, on 2026-10-22. [Previous release](RELEASE_0.48.0.md).

## Changes

- Insert a vertex on a saved polyline, perimeter or polygon-area segment, including a polygon's closing edge. Remove a selected vertex within the shape's minimum and maximum counts.
- Select vertices by number, including points at the same position, and retain the existing on-page dragging workflow.
- Inspect Preview, adjust the draft and Apply with one Undo step. Preserve the measurement's type, calibration, unit, precision, identity, replies and entered quantity fields; measured values and compatible price totals follow the accepted geometry.
- Keep length and angle point counts fixed, refuse invalid or stale edits, and preserve the original page defaults and other measurements.
- Include the published 0.48.0 Windows packages in portable and MSI upgrade regressions.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831660301) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831682434) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831660552). 761 tests passed, 10 default ignored. 1397 EN/FR keys. 41 portable scenarios. 35 MSI baselines. 175 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 63 packaged CLI calls verify insertion and removal across polyline, perimeter and area measurements, reopened vertex counts, calibration and quantity preservation, recalculated totals, minimum/maximum and fixed-kind bounds, invalid/stale refusals and the retained measurement lifecycle. They do not establish native desktop or ARM64 qualification. [Evidence](qa/RELEASE_0.49.0.json).

## Scope

Polylines retain at least two vertices; perimeters and polygon areas retain at least three. These shapes allow at most 128 vertices. Length measurements keep two points and angles three. The existing CLI accepts explicit geometry replacement; this release adds no new CLI command.

Vector snapping, batch recalculation, multiple-scale viewports and real-plan qualification remain open. AEC-002 and AEC-003 remain partial. Packages remain unsigned; native ARM64, hardware and accessibility qualification remain separate from automated checks.
