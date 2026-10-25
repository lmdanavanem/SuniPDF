# SuniPDF 0.90.0 - candidate

This candidate adds dimensioned circular and elliptical area creation to the
existing measurement workflow. Publication and package qualification are pending.

Choose a centre and real diameter, or two diameters and an angle, using a captured,
page, named-region or automatically resolved calibration. Review the prepared PDF
preview, then apply one undoable area measurement. Source changes or edited
parameters invalidate the preview.

The saved contour contains 128 editable vertices. Its approximation is explicit
in the interface; quantities use the actual stored polygon area. Save/reopen and
vertex editing use the existing area-measurement format. This does not create
analytic curves, permanent shape constraints or independent horizontal/vertical
scales.

The CLI adds `dimensioned-circle` and `dimensioned-ellipse`, with the same geometry
and scale checks. New copies preserve the original PDF and refuse existing output
paths. Source tests and a separate packaged CLI/QPDF corpus are planned to verify
rotated/cropped pages, geometry, appearances, quantities and refusal behavior.
Successful runtime qualification has not yet been recorded for this candidate.

[Circle and ellipse guide](DIMENSIONED_ELLIPSES.md) · [Plan workspace](PLAN_WORKSPACE.md) ·
[Project status](PROJECT_STATUS.md)
