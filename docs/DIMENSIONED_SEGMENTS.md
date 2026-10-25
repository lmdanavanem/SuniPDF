# Dimensioned segments

Development candidate for [0.91](RELEASE_0.91.0.md). Package qualification is pending.

Create a straight calibrated length from a starting point, a real length and a
clockwise angle. The saved annotation is an ordinary length measurement with two
editable endpoints. Quantities use the actual stored line length and its calibration.

## Place a segment

Open **Tools > Review & measure > Measurements** and select **Dimensioned segment**.
Enter the length, unit, precision and angle. Choose a captured temporary reference,
a saved page scale, a named calibrated region or automatic reference resolution.
Click the starting point and review the prepared PDF preview before applying.

Zero degrees points right on the displayed page; 90 degrees points down. Negative
angles turn in the opposite direction, and finite angles normalize to [0, 360).
Only the starting point snaps. The computed endpoint follows the entered length
and angle without further snapping or clipping.

Application creates one measurement and one Undo step. Editing the angle in the
preview invalidates that preview until regenerated. Document changes also refuse
an outdated candidate. Undo/Redo and save/reopen retain the two ordinary endpoints;
subsequent endpoint editing can change both the length and direction.

## Scale and precision

The complete segment must fit the displayed page and its selected region. An
automatic scale refuses ambiguous regions and boundary crossings, including a
segment whose endpoints lie outside a region but whose interior crosses it. The
saved page scale remains an explicit override. Missing or changed calibration,
unsupported page units and insufficient annotation permission refuse creation.

Lengths must be positive, finite and accurately representable after PDF coordinate
storage. Length and direction use a relative tolerance of 0.001%; coordinates
must also survive the existing absolute-position and rounded-label checks.
Quantities and labels use the stored endpoints, so their last displayed decimal
may differ from the requested length. If coordinate conversion changes that
label, creation is refused; choose fewer decimals or another position. Existing
minimum-length, scale and value limits remain in effect.
The result is a straight measurement line with its existing measurement label;
it introduces no extension lines, architectural dimension arrows, permanent
length constraints or new measurement metadata format.

## CLI

```text
sunipdf-cli dimensioned-segment plan.pdf --page 1 --start 200,300 --length 4 --unit m --rotation-degrees 30 --precision 2 --calibration "50,50 150,50" --known-distance 10 --out segment.pdf

sunipdf-cli dimensioned-segment plan.pdf --start 200,300 --length "6' 6\"" --unit ft --use-page-calibration --out segment-feet.pdf
```

`--start` uses displayed-page points, with the origin at the lower left. Length
and calibration may use separate units through `--calibration-unit`. Use exactly
one reference: `--calibration` with `--known-distance`, `--use-page-calibration`,
`--region ID`, or `--automatic-scale`.

Existing destinations, including the source path, are refused even with
`--overwrite`. The JSON receipt identifies `shape: "segment"`, `vertices: 2`,
`segments: 1` and `persistent_shape_constraint: false`, and reports the normalized
angle. Measurement details are omitted when the PDF prohibits copying. Passwords
use the existing protected-input options, such as `--password-stdin`.

[Plan workspace](PLAN_WORKSPACE.md) | [Measurements](MEASUREMENTS.md)
