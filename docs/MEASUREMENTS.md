# Local calibrated measurements (0.40.0)

[Français](MEASUREMENTS.fr.md) · [AEC backlog](AEC_BACKLOG.md)

Open **Comment → Rich annotations**, then choose **Calibrated measurements**. Select Length, Polyline length, Polygon perimeter, Polygon area or Angle. Enter one `x,y` vertex per line in displayed-page points, with the origin at the bottom left after rotation and cropping. Angles use three vertices with the angle vertex in the middle. Closed polygons are closed automatically: do not repeat the first point.

Enter two calibration points spanning a distance you know from the drawing, enter that positive distance, then choose `mm`, `cm`, `m`, `in` or `ft`. Feet and inches are decimal units; for 10 feet 6 inches enter `10.5 ft` or `126 in`. Set 0–4 decimal places. The numeric draft result updates as inputs change. Select **Preview**, inspect the actual rendered candidate, then **Apply**. Changing inputs requires a new preview. Cancel/Escape does not apply the draft. Save the PDF to keep it; one Undo removes the annotation and its metadata together.

Each annotation embeds its own two-point calibration, geometry, unit and precision. There is no shared page-wide scale: one measurement cannot silently change another's value. This first slice uses coordinate entry; interactive point picking, snapping, page calibration, named viewports and global recalculation remain in the backlog. Recreate a measurement to change its scale or geometry. Generic move, resize, restyle and value-text edits are refused to prevent stale labels. Delete and comment replies remain available within existing PDF permissions.

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

Page-wide calibration and recalculation, direct manipulation, multi-scale viewports, counts, volumes, annotation registers and advanced recognition remain open in AEC-001–026. Local tools remain free and open source; cloud and collaboration are deferred.
