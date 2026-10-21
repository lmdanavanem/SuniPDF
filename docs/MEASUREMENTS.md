# Local calibrated measurements

[Français](MEASUREMENTS.fr.md) · [AEC backlog](AEC_BACKLOG.md)

**Unreleased desktop integration:** the direct toolbar entry and on-page drawing described below are being prepared after 0.41.0. They are not included in the published 0.41.0 packages. The existing PDF measurement records and CLI remain supported.

## Measure directly on the page

1. Open a PDF and select **Measurements** beside **All tools** in the main toolbar. The same tool is available from **All tools → Measurements** and the first group of the **Comment** workspace. Its toolbar label remains visible in narrow windows.
2. Choose `mm`, `cm`, `m`, `in` or `ft` and 0–4 decimal places. Select **Calibrate this page**, then click two different points spanning a distance you know from the drawing. Enter the positive **Known distance**, then select **Confirm scale**. Feet and inches use decimal values: 10 feet 6 inches is `10.5 ft` or `126 in`.
3. Choose a measurement type and click directly on the page. **Length** opens the preview after two points. **Angle** opens it after three points: first arm, vertex, second arm; angles do not require a physical scale. For **Polyline length**, **Polygon perimeter** and **Polygon area**, add the vertices, then select **Preview measurement**, press **Enter**, or double-click the last point. Polygons close automatically; do not repeat the first vertex.
4. Inspect the rendered candidate and its value, then select **Apply**. Nothing is added until Apply succeeds. **Cancel** or **Escape** discards the preview. Save the PDF to retain the measurement; one Undo removes the annotation and its metadata together. The tool stays active after applying, so another measurement on the calibrated page can be drawn immediately.

While drawing, **Backspace** or **Undo point** removes the last point. **Shift** constrains the next segment horizontally or vertically in the displayed page's coordinate axes; this is not snapping to existing PDF lines. **Escape** or **Cancel drawing** abandons the current draft. Finish or cancel a draft before drawing on another page. Drafts are also cancelled when the document changes.

## Scale lifetime and saved measurements

Each page can have a separate confirmed scale for the current document session. Ordinary annotation edits and normal tab switches retain these scales; page geometry or structural edits clear them. Closing and reopening a document requires recalibration. Merely navigating between pages does not assign one page's scale to another. Changing the display unit converts the confirmed distance instead of reinterpreting its numeric value.

The session scale is not saved as shared page-scale metadata. Each applied annotation embeds its own two-point calibration, geometry, unit and precision, which remain available after saving and reopening. Recalibrating for new measurements does not change existing annotations. Shared page-scale persistence, global recalculation and named multi-scale viewports remain open.

Recreate an applied measurement to change its scale or vertices. Generic move, resize, restyle and value-text edits are refused to prevent stale labels. Delete and comment replies remain available within existing PDF permissions. Manual coordinate entry remains available in **Rich annotations → Calibrated measurements**; its `x,y` values use displayed-page points with the origin at the bottom left after rotation and cropping.

## CLI and quantities

```powershell
sunipdf-cli measure input.pdf --kind length --points "40,70 140,70" --calibration "40,40 140,40" --known-distance 10 --unit m --precision 2 --page 1 --out measured.pdf
sunipdf-cli measure input.pdf --kind area --points "40,70 140,70 140,120 40,120" --calibration "40,40 140,40" --known-distance 10 --unit m --out measured-area.pdf
sunipdf-cli measurements measured-area.pdf
```

The examples measure 10 m and 50 m². The create command returns its annotation index and result; `measurements` exports per-page records as JSON, including annotation identity, geometry, calibration, unrounded numeric value, display label and precision. Areas use squared units (`m2`), angles use degrees (`deg`), and other values use the selected linear unit. JSON contains no spreadsheet formulas, macros or external references. Pages are 1-based and annotation indices are 0-based. Prefer `--password-env NAME` or `--password-stdin` for protected documents.

`measure` always writes a new copy and refuses an existing destination or the original path even with `--overwrite`. Creation requires annotation permission; permitted AES-128/AES-256 protection and both credentials are preserved. Annotation-only permissions suffice; full content-modification permission is not required.

## Open metadata contract

A real Stamp annotation carries a rendered appearance and a `SuniPDFMeasurement` JSON string. Version 1 stores `coordinate_system: "pdf-user-space"`, kind, user-space vertices, two calibration points, known distance, unit, precision and its user-space rectangle. This keeps the physical calculation invariant under page rotation/cropping. Export recalculates from those inputs rather than trusting editable comment text. Unsupported versions, malformed metadata, externally moved/resized rectangles and recognized imported `/Measure` metadata are refused explicitly. Original files are retained so that other software can be used when an imported format is unsupported.

This is a SuniPDF exchange contract, not Bluebeam proprietary-format compatibility. An annotation modified by another editor may need to be recreated. The metadata and ordinary PDF appearance are not a digital signature or proof of dimensional accuracy: the operator must verify the known distance and drawing revision.

## Bounds and remaining work

JSON export is bounded to 1000 pages and 4096 records, with 4 MiB of measurement metadata per page. At most 128 vertices, finite in-page coordinates, positive calibration, no zero-length segments, simple non-degenerate polygons and results between 1e-9 and 1e12. Self-intersecting or invisible geometry is refused. The annotation includes a label strip and may overlap existing drawing content; inspect the preview before applying.

Known-geometry fixtures cover arithmetic, metric/imperial equivalence, rotation/crop, reopen, preview/commit, snapshot Undo, allowed encryption, invalid geometry and stale-record refusals. CLI fixtures add independent MuPDF rendering and qpdf checks. These generated fixtures do not qualify survey accuracy, real customer plans, physical printing, native pointing/Narrator interaction, PDF/A or PDF/UA.

Editing saved vertices, persistent shared page scales and recalculation, snapping to PDF vectors, multi-scale viewports, volumes and advanced recognition remain open in AEC-001–026. [Manual counts and the annotation register](MARKUP_REGISTER.md) already exist within their documented 0.41.0 bounds. Local tools remain free and open source; cloud and collaboration are deferred.
