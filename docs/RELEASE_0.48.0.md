# SuniPDF 0.48.0 — edit saved measurement vertices

[Français](RELEASE_0.48.0.fr.md) · [Geometry editing guide](MEASUREMENT_GEOMETRY.md)

Status: published as stable Latest [0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0), source `6f32c20259f6bae6e1b60090fe74acfb5dc69ba7`, on 2026-10-22. [Previous release](RELEASE_0.47.0.md).

## Changes

- Select a saved measurement and edit its existing vertices directly on the page. Preview, adjust, Apply and one-step Undo keep acceptance explicit.
- Preserve the measurement's own calibration, units, precision, identity, replies and quantity fields. Measured quantities and compatible price totals follow the accepted geometry.
- Keep invalid or stale edits from being applied; support page rotation, crop geometry, cancellation and protected documents through the existing measurement guards.
- Validate documentation-only changes through a bounded documentation path in CI, while retaining full checks for source, scripts, workflows and release builds.
- Include the published 0.47.0 Windows packages in portable and MSI upgrade regressions.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760622965) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760636143) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760622897). 752 tests passed, 10 default ignored. 1388 EN/FR keys. 40 portable scenarios. 34 MSI baselines. 170 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 26 packaged CLI calls verify saved/reopened page scales, explicit geometry replacement, unit conversion, precision, recalibration, stable identity, independent measurements, preserved typed quantity fields and price basis, recalculated totals, invalid/stale refusals and unchanged original files. They do not establish native desktop or ARM64 qualification. [Evidence](qa/RELEASE_0.48.0.json).

## Scope

Vertex count and measurement type stay fixed. Vector snapping, adding/removing vertices, batch recalculation and multiple-scale viewports remain open. The existing CLI command is unchanged. Packages remain unsigned; native ARM64, hardware and accessibility qualification remain separate from automated checks.
