# SuniPDF 0.48.0 — edit saved measurement vertices

[Français](RELEASE_0.48.0.fr.md) · [Geometry editing guide](MEASUREMENT_GEOMETRY.md)

This build includes the scope below. Published assets and qualification are tracked on the [0.48.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0).

## Changes

- Select a saved measurement and edit its existing vertices directly on the page. Preview, adjust, Apply and one-step Undo keep acceptance explicit.
- Preserve the measurement's own calibration, units, precision, identity, replies and quantity fields. Measured quantities and compatible price totals follow the accepted geometry.
- Keep invalid or stale edits from being applied; support page rotation, crop geometry, cancellation and protected documents through the existing measurement guards.
- Validate documentation-only changes through a bounded documentation path in CI, while retaining full checks for source, scripts, workflows and release builds.
- Include the published 0.47.0 Windows packages in portable and MSI upgrade regressions.

## Qualification

Qualification records and distribution assets are tracked on the [0.48.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0). Results apply to the exact source identified there.

## Scope

Vertex count and measurement type stay fixed. Vector snapping, adding/removing vertices, batch recalculation and multiple-scale viewports remain open. The existing CLI command is unchanged. Packages remain unsigned; native ARM64, hardware and accessibility qualification remain separate from automated checks.
