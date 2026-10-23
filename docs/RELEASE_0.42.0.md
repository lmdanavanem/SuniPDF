# SuniPDF 0.42.0 — measurements directly on the page
[Measurement guide](MEASUREMENTS.md)

Status: published as stable Latest [0.42.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.42.0), source `96ad768b24813e19c01de2881d3cb53c5b2fdc6c`.

Measurements have a dedicated, labelled button beside All tools and their own workspace. Calibrate a page by clicking two points and entering the known distance. Draw lengths, polylines, polygon perimeters, polygon areas and angles directly on the PDF. A live overlay shows the draft geometry and result; Shift constrains a segment horizontally or vertically. Backspace removes the last point, and Escape cancels the drawing.

Lengths and angles open the PDF preview after their final point. Finish a polyline or polygon with Enter, a double-click or Preview measurement. Apply adds the actual annotation; Cancel leaves the document unchanged. Saving preserves geometry, calibration and the value in the PDF, and Undo removes them together. The tool remains active for repeated measurements.

Page scales are separate within the current document session. Changing units converts the calibration. Ordinary annotation edits preserve confirmed scales; page geometry or structural changes invalidate them. An unfinished drawing cannot silently follow a page, document or tool change.

Automated coverage includes English/French toolbar visibility at narrow widths, actual egui pointer events, calibration, preview/apply, duplicate double-click prevention, keyboard controls, page isolation and stale drafts. Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696093211) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696104747) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696092839). 662 tests passed, 10 default ignored. 1224 EN/FR keys. 34 portable scenarios. 28 MSI baselines / 140 logs. No open Code Scanning alerts. Six unauthenticated public downloads and dist copies verified by SHA256; four packaged CLI calls check version, help, creating a 10 m length and reopening without changing the original.

The session calibration is not saved as a shared page property. Switching tabs preserves each document's scale. Closing and reopening a document requires recalibration for new measurements; saved annotations retain their own scale. Editing saved vertices, snapping to PDF vectors, multi-scale viewports and global recalculation remain separate backlog items. Local features remain free and open source.

Windows packages remain unsigned. ARM64 is cross-compiled; native ARM64 hardware and physical measurement accuracy are not qualified by automated tests.

[Publication evidence and checksums for all six files](qa/RELEASE_0.42.0.json).
