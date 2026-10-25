# Dimensioned circles and ellipses

Development candidate for [0.91](RELEASE_0.91.0.md). Package qualification is pending.

Create a circular or elliptical area from real dimensions and an explicit scale.
The saved measurement is an editable polygon with 128 vertices. The interface
shows this approximation before application, and quantities use the area of the
vertices actually saved.

## Place a shape

Open **Measurements** and choose a dimensioned **Circle** or **Ellipse**. Enter
one diameter for a circle, or two diameters and a clockwise angle for an ellipse.
Choose the unit, precision and calibration reference, then place the centre on
the displayed page. The indicative outline follows the same geometry constructor
as the prepared PDF preview.

Review the prepared preview before applying. Changing the dimensions, centre,
angle or reference invalidates it. Application creates one area measurement and
one Undo step. Undo/Redo and save/reopen retain its ordinary editable vertices;
subsequent vertex editing can change its circular or elliptical shape.

The scale can come from the captured temporary calibration, the saved page scale,
a named calibrated region or automatic reference resolution. The complete contour
must fit the page and the permitted region. Missing, changed or ambiguous
references are refused. One uniform scale is used; independent horizontal and
vertical scales are not introduced.

## Precision and supported geometry

The 128 samples include all four diameter endpoints and rotate around the centre.
For ideal coordinates, the polygon area is approximately **0.0402% smaller** than
the corresponding analytic ellipse area. Coordinate storage introduces additional
rounding. The quantity table reports the stored polygon area, not an analytic
replacement value.

Non-finite, non-positive, collapsed, out-of-bounds or insufficiently representable
shapes are refused. Existing calibration and measurement precision checks remain
in effect. This workflow creates neither permanent geometric constraints nor an
analytic curve annotation. It does not establish physical survey accuracy or
editable measurement exchange with other PDF applications.

## CLI

The centre is expressed in displayed-page points; diameters and the known
calibration distance use the selected real unit. Existing outputs are refused.

```text
sunipdf-cli dimensioned-circle plan.pdf --page 1 --center 300,300 --diameter 4 --unit m --precision 2 --calibration "50,50 150,50" --known-distance 10 --out circle.pdf

sunipdf-cli dimensioned-ellipse plan.pdf --page 1 --center 300,300 --diameter-x 4 --diameter-y 2 --rotation-degrees 30 --unit m --precision 2 --calibration "50,50 150,50" --known-distance 10 --out ellipse.pdf
```

Both commands use the same constructor and save a new copy. Their JSON output
identifies the polygonal approximation and includes the actual measurement when
the document permits export of that information.
Passwords, when required, use the existing protected-input mechanism; do not put
them in command arguments.

[Plan workspace](PLAN_WORKSPACE.md) · [Measurements and quantities](QUANTITY_TABLE.md)
