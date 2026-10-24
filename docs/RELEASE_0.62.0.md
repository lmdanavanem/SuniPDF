# SuniPDF 0.62.0

Status: **in preparation**, 2026-10-24. Local Windows x64 development qualification passed;
exact-version CI, release packages, public download verification and stable publication remain pending.
The current public download is [0.61.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.61.0).

## Rectangles from real dimensions

Create a rectangle by entering real width and height, choosing a unit and placing its anchor on the page.
Use an explicit local reference, a saved page scale, a named scale region or automatic region selection.
Preview the resulting area before Apply. The operation supports one Undo step, Redo, save and reopen;
the CLI uses the same geometry, calibration and permission checks.

Page bounds, crop/rotation, user units, region crossings and storage precision are checked before mutation.
The saved result is a four-vertex area measurement. Subsequent vertex edits can turn it into another polygon;
this is not a permanent rectangular constraint. Holes, deducted areas, depth/volume, analytic circles and
independent X/Y scales remain outside this workflow. See [Measurements](MEASUREMENTS.md).

## Current and historical measurements

New measurements bind their saved reference to page bounds, user units, decoded content, reachable resources,
initial layer visibility and page transparency state. Legacy or changed-page values remain inspectable as
historical data, but cannot contribute active quantities, prices or quantity-dependent formulas.

Lengths and areas require an explicit current reference followed by Preview and Apply to become current again.
Historical angles must be recreated. Unit, price and custom-field edits cannot silently promote historical data.
Older page and region references require explicit recalibration or removal. Recalibration retains compatible
prices and fields; incompatible price dimensions, overflow and unsafe stored geometry refuse the whole mutation.

## Quantity tables and report formulas

Formula subtotals sum the rounded row results with checked arithmetic, separating each formula, group, unit
and precision. Coverage and error counts remain explicit; an all-error group does not become a fabricated zero.
Cells are evaluated once and reused by the table, summaries and exports. Numeric signed CSV values stay numeric;
free text retains spreadsheet-formula protection.

Table construction reads document permissions once and avoids repeatedly copying the page annotation array.
Exports and edits still check permissions and source identity. Quantity cells suppress floating-point noise
without changing the stored value, price, formula input or export; the raw value remains available in a tooltip.
See [Quantity tables](QUANTITY_TABLE.md).

## Document safety and interface fixes

- Enforce CLI no-overwrite behavior when the output is finally created, including a destination created during processing.
- Restore documents transactionally and retain the active document and credential if replacement loading fails.
- Refuse certificate signing of encrypted PDFs, including inputs automatically unlocked with an empty user password.
- Retain unsaved bytes, Undo/Redo history and recovery identity after failed tab resume, and allow retry.
- Invalidate stale search/register results and synchronize page/navigation state after edits completed during suspension.
- Close applied editors correctly while retaining refused drafts, and improve labels, controls and warning contrast in both themes.
- Include the published 0.61.0 packages in portable and MSI upgrade regressions.

## Qualification recorded before release packaging

The local development source passed 904 workspace tests with zero failures; 15 tests were ignored by that suite.
Three independent corpus tests were also run explicitly before the final visual-only adjustments.
Formatting, Clippy, documentation and 1,536 matching English/French interface keys passed.
Final CLI qualification executed 44 commands and 276 assertions; 19 independent output/source checks passed.

Native Windows scenarios covered rectangle creation, Preview/Apply, Undo/Redo, save/reopen, historical-value
exclusion and explicit recalibration restoring a compatible price without losing custom fields. Final visual
checks covered English/French and light/dark themes. These observations concern the local development build;
they do not replace exact-version CI or downloaded-package checks.

A same-machine synthetic CLI comparison used five repetitions after warmup with matching development profiles.
For 300 annotations and eight constant formulas, the median complete command fell from 5.593 s to 0.279 s.
The optimization-and-output case increased from 0.053 s to 0.076 s; its cause has not been isolated.
These bounded results do not establish release-build performance or general UI smoothness. Native action/capture
timings include automation overhead and showed no demonstrated improvement in fluidity.

Windows x64/ARM64 ZIPs, x64 MSI and Ubuntu amd64 DEB remain subject to CI and exact-package qualification.
Packages remain unsigned; native ARM64 execution, physical printing, human accessibility, independent update
authenticity and private real-project plans retain their separate [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Existing PDF/A and PDF/UA evidence applies only to its recorded corpus and operations, not universal conformance.
