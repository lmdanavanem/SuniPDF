# SuniPDF 0.42.0 — measurements directly on the page

[Français](RELEASE_0.42.0.fr.md) · [Measurement guide](MEASUREMENTS.md)

Status: implementation complete; release qualification pending.

Measurements have a dedicated, labelled button beside All tools and their own workspace. Calibrate a page by clicking two points and entering the known distance. Draw lengths, polylines, polygon perimeters, polygon areas and angles directly on the PDF. A live overlay shows the draft geometry and result; Shift constrains a segment horizontally or vertically. Backspace removes the last point, and Escape cancels the drawing.

Lengths and angles open the PDF preview after their final point. Finish a polyline or polygon with Enter, a double-click or Preview measurement. Apply adds the actual annotation; Cancel leaves the document unchanged. Saving preserves geometry, calibration and the value in the PDF, and Undo removes them together. The tool remains active for repeated measurements.

Page scales are separate within the current document session. Changing units converts the calibration. Ordinary annotation edits preserve confirmed scales; page geometry or structural changes invalidate them. An unfinished drawing cannot silently follow a page, document or tool change.

Automated coverage includes English/French toolbar visibility at narrow widths, actual egui pointer events, calibration, preview/apply, duplicate double-click prevention, keyboard controls, page isolation and stale drafts. Validation results will be recorded after the checks complete.

The session calibration is not saved as a shared page property. Switching tabs preserves each document's scale. Closing and reopening a document requires recalibration for new measurements; saved annotations retain their own scale. Editing saved vertices, snapping to PDF vectors, multi-scale viewports and global recalculation remain separate backlog items. This release does not claim complete Bluebeam compatibility. Local features remain free and open source; cloud collaboration remains deferred.

Windows packages remain unsigned. ARM64 is cross-compiled; native ARM64 hardware and physical measurement accuracy are not qualified by automated tests.
