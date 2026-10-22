# Local calibrated measurements

[Français](MEASUREMENTS.fr.md) · [AEC backlog](AEC_BACKLOG.md)

**0.49.0 build:** add or remove saved polyline, perimeter and polygon-area vertices within their minimum and maximum counts; select a vertex by number even when handles overlap. Preview, Apply and Undo preserve the measurement’s own calibration and entered quantity fields. [Workflow and limits](MEASUREMENT_GEOMETRY.md).

**Available in [0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0):** drag existing saved measurement vertices through **Edit geometry**, then Preview and Apply. Calibration and entered quantity fields are preserved. [Workflow and limits](MEASUREMENT_GEOMETRY.md). [Evidence](qa/RELEASE_0.48.0.json).

**Available in [0.44.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.44.0):** Edit one saved measurement's reference distance, unit and precision through Preview and Apply; the CLI also supports explicit geometry replacement with an expected record. Identity, replies, page defaults and other measurements are preserved. Temporary scales can be cleared, the inspector shares cached records, exports enforce copying permission and incompatible measured-page resizing is refused. [Evidence](qa/RELEASE_0.44.0.json).

**Available in [0.43.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.43.0):** persistent page scales, inspection and explicit scale reuse extend on-page drawing. Existing measurements retain their values. [Evidence](qa/RELEASE_0.43.0.json).

## Measure directly on the page

1. Open a PDF and select **Measurements** beside **All tools** in the main toolbar. The same tool is available from **All tools → Measurements** and the first group of the **Add comments** workspace. Its toolbar label remains visible in narrow windows.
2. Choose `mm`, `cm`, `m`, `in` or `ft` and 0–4 decimal places. Select **Calibrate this page**, then click two different points spanning a distance you know from the drawing. Enter the positive **Known distance**, then select **Confirm scale**. Feet and inches use decimal values: 10 feet 6 inches is `10.5 ft` or `126 in`.
3. Choose a measurement type and click directly on the page. **Length** opens the preview after two points. **Angle** opens it after three points: first arm, vertex, second arm; angles do not require a physical scale. For **Polyline length**, **Polygon perimeter** and **Polygon area**, add the vertices, then select **Preview measurement**, press **Enter**, or double-click the last point. Polygons close automatically; do not repeat the first vertex.
4. Inspect the rendered candidate and its value, then select **Apply**. Nothing is added until Apply succeeds. **Cancel** or **Escape** discards the preview. Save the PDF to retain the measurement; one Undo removes the annotation and its metadata together. The tool stays active after applying, so another measurement on the calibrated page can be drawn immediately.

While drawing, **Backspace** or **Undo point** removes the last point. **Shift** constrains the next segment horizontally or vertically in the displayed page's coordinate axes; this is not snapping to existing PDF lines. **Escape** or **Cancel drawing** abandons the current draft. Finish or cancel a draft before drawing on another page. Drafts are also cancelled when the document changes.

## Session scale and saved page default

Each page can have a separate confirmed scale for the current session. Ordinary annotation edits and normal tab switches retain these scales; changes to page geometry, structure or content clear them. A session scale takes precedence over that page's saved default. Navigating between pages never assigns one page's scale to another. Changing the display unit converts the confirmed distance instead of reinterpreting its numeric value.

After confirming a scale, select **Save scale in PDF**, then save the PDF file to retain it on disk. The successful operation replaces the session override with a saved page default. Reopening the file loads the supported saved default automatically for new measurements. A session scale that was not saved is lost when the document closes.

**Remove saved scale** removes only that page's saved default; an explicitly chosen session scale remains available. Save the PDF to retain the removal. **Undo** and **Redo** restore the corresponding document state and reload its saved default; they do not restore a cleared session scale. Saving/removing a page default requires document-modification permission; drawing an annotation requires annotation permission. The save/remove controls stay disabled without document-modification permission. AES-128/AES-256 protection is retained.

The saved default records PDF user-space points, page bounds and a digest of decoded page content. Rotation and page reordering preserve an eligible default. Changed crop bounds, resized/content-modified pages, unsupported metadata and non-default `UserUnit` are refused rather than treated as valid scales. Recalibrate or remove a supported stale default; unknown or malformed records are preserved and reported as errors. These checks are not comprehensive revision authentication: always verify the plan and its known dimension.

Each applied annotation embeds its own two-point calibration, geometry, unit and precision. Saving, removing or replacing the page default never recalculates existing annotations. Global recalculation and named multi-scale viewports remain open.

In 0.44, **Use saved page scale** discards the current page's temporary override when its saved default is valid. If no valid saved default is available, **Clear temporary scale** discards only the temporary override. These controls do not edit the PDF or other pages. An unavailable saved default remains unavailable; calibrate the page explicitly before drawing a scaled measurement.

## Inspect saved measurements

In **Measurements**, select **Saved measurements** to inspect the current page. Reading the list leaves the PDF unchanged. It shows the calculated label and unit, measurement type, annotation index, optional identity, vertex count, precision and known reference distance for non-angle measurements. Lists use batches of 50; **Refresh** rereads the current document state. Unsupported or changed annotation metadata is reported explicitly. In 0.44, records and labels are shared in memory instead of copying the complete list and generating result JSON every frame.

**View on page** selects the actual annotation and opens its comments. **Use this scale** copies a non-angle annotation's calibration into the current page's session, without editing the annotation or saving a page default. References outside the displayed page are refused. Use **Save scale in PDF** separately if this is the scale you want to retain for future sessions. Close the inspector before starting a drawing or changing documents.

Generic move, resize, restyle and value-text edits are refused to prevent stale labels. Delete and comment replies remain available within existing PDF permissions. Manual coordinate entry remains available in **Rich annotations → Calibrated measurements**; its `x,y` values use displayed-page points with the origin at the bottom left after rotation and cropping. In 0.44, this manual path no longer supplies an arbitrary positive reference distance: enter a valid calibration explicitly.

## Edit one saved measurement

1. Open **Saved measurements** on the relevant page and select **Edit measurement** beside the value.
2. Change the known reference distance, unit or precision. Changing `m` to `cm` converts a 10 m reference into 1000 cm without changing physical size. Changing the reference distance recalculates this measurement only. For angles, only precision is editable.
3. Select **Preview** and inspect the rendered candidate, then **Apply**. The original vertices and reference points stay fixed in this desktop editor. The annotation keeps its identity, author, replies and register fields; page defaults and other measurements stay unchanged.
4. **Cancel** leaves the PDF unchanged. A successful edit uses one Undo/Redo step. Save the PDF to retain it on disk.

Changing a parameter invalidates the previous preview. Changed document state, stale records, locked or ambiguous identities, unsupported metadata/appearances, and externally altered value text or geometry cause a refusal. Refresh or reopen the inspector before retrying; preserve unsupported annotations rather than replacing them by guesswork. Editing requires annotation permission and is limited to supported measurements in documents up to 128 MiB. The separate [geometry editor](MEASUREMENT_GEOMETRY.md) moves existing vertices on the page; the CLI also supports explicit replacements.

## CLI and quantities

```powershell
sunipdf-cli measure input.pdf --kind length --points "40,70 140,70" --calibration "40,40 140,40" --known-distance 10 --unit m --precision 2 --page 1 --out measured.pdf
sunipdf-cli measure input.pdf --kind area --points "40,70 140,70 140,120 40,120" --calibration "40,40 140,40" --known-distance 10 --unit m --out measured-area.pdf
sunipdf-cli measurements measured-area.pdf

sunipdf-cli set-page-calibration input.pdf --page 1 --calibration "40,40 140,40" --known-distance 10 --unit m --out calibrated.pdf
sunipdf-cli page-calibration calibrated.pdf --page 1
sunipdf-cli measure calibrated.pdf --kind length --points "40,70 140,70" --use-page-calibration --unit cm --out measured-cm.pdf
sunipdf-cli remove-page-calibration measured-cm.pdf --page 1 --out without-page-scale.pdf
```

For 0.44, export a fresh expected record before editing. The annotation index comes from the record, not its position in the list:

```powershell
$inventory = sunipdf-cli measurements measured.pdf --page 1 | ConvertFrom-Json
$record = $inventory.pages[0].measurements[0]
$record | ConvertTo-Json -Depth 16 | Set-Content -Encoding utf8 expected-measurement.json
sunipdf-cli edit-measurement measured.pdf --page 1 --index $record.index --expected-record expected-measurement.json --unit cm --precision 1 --out edited.pdf
```

This example converts the original 10 m length into 1000 cm without changing its physical size. `--expected-record` accepts one complete version-1 record, including its stable annotation identity, up to 64 KiB. The current page/index/identity/geometry/calibration/precision must match it. Re-export after an edit; stale records are refused. `--points` can replace up to 128 displayed-page vertices, and `--kind` can change the type when the geometry is valid. To replace a reference, provide `--calibration` together with `--known-distance` in the resulting unit, or explicitly select `--use-page-calibration`. A change from an angle to a scaled measurement needs a physical reference. `measure --kind angle` needs three points and no calibration.

In 0.44, `measurements` and `page-calibration` require copying permission. A creation/edit permitted by annotation rights can still succeed without copying permission, but its JSON response contains `measurement: null` and `export_unavailable: true`; it does not export stored calibration or quantities. `edit-measurement` also requires a new destination even with `--overwrite`.

The manual examples measure 10 m and 50 m²; the saved-scale example produces 1000 cm. `--use-page-calibration` is mutually exclusive with `--calibration` and `--known-distance`; it fails if no valid saved default exists. `--unit` physically converts the saved reference distance. `page-calibration` returns JSON with a `calibration` object or `null` for each page; omit `--page` to read at most 1000 pages. Saving/removing requires an explicit page and never changes existing measurement values.

The create command returns its annotation index and result; `measurements` exports per-page records as JSON, including annotation identity, geometry, calibration, unrounded numeric value, display label and precision. Areas use squared units (`m2`), angles use degrees (`deg`), and other values use the selected linear unit. JSON contains no spreadsheet formulas, macros or external references. Pages are 1-based and annotation indices are 0-based. Prefer `--password-env NAME` or `--password-stdin` for protected documents.

`measure`, `set-page-calibration` and `remove-page-calibration` always write new copies and refuse existing destinations or the original path even with `--overwrite`. Page-default changes require document-modification permission; measurement creation requires annotation permission only. Permitted AES-128/AES-256 protection and both credentials are preserved.

## Open metadata contract

A real Stamp annotation carries a rendered appearance and a `SuniPDFMeasurement` JSON string. Version 1 stores `coordinate_system: "pdf-user-space"`, kind, user-space vertices, two calibration points, known distance, unit, precision and its user-space rectangle. This keeps the physical calculation invariant under page rotation/cropping. Export recalculates from those inputs rather than trusting editable comment text. Unsupported versions, malformed metadata, externally moved/resized rectangles and recognized imported `/Measure` metadata are refused explicitly. Original files are retained so that other software can be used when an imported format is unsupported.

A separate `SuniPDFPageCalibration` page string stores the versioned default for new measurements. It is bounded to 4096 bytes, uses PDF user-space points and includes its geometry/content guard. Content verification accepts at most 1024 page streams, 8 MiB decoded per stream and 32 MiB total. This metadata does not replace any annotation's own calibration.

This is a SuniPDF exchange contract, not third-party proprietary-format compatibility. An annotation modified by another editor may need to be recreated. The metadata and ordinary PDF appearance are not a digital signature or proof of dimensional accuracy: the operator must verify the known distance and drawing revision.

## Bounds and remaining work

JSON export is bounded to 1000 pages and 4096 records, with 4 MiB of measurement metadata per page. At most 128 vertices, finite in-page coordinates, positive calibration, no zero-length segments, simple non-degenerate polygons and results between 1e-9 and 1e12. Self-intersecting or invisible geometry is refused. The annotation includes a label strip and may overlap existing drawing content; inspect the preview before applying.

Known-geometry fixtures cover arithmetic, metric/imperial equivalence, rotation/crop, reopen, preview/commit, snapshot Undo, allowed encryption, invalid geometry and stale-record refusals. The 0.43 additions cover page-default persistence, permissions, physical unit conversion and independent qpdf checks within the existing corpus; executed results belong in Project status. Generated fixtures do not qualify survey accuracy, real customer plans, physical printing, native pointing/Narrator interaction, PDF/A or PDF/UA.

Resizing content with annotations refuses measured pages atomically rather than leaving inconsistent rectangles and quantities. Saved polylines support 2–128 vertices; perimeters and polygon areas support 3–128. Length and angle measurements keep their fixed point counts. Batch recalculation, snapping to PDF vectors, multi-scale viewports, volumes and advanced recognition remain open. AEC-001/002/003/004/026 remain partial; neither page defaults, the inspector nor individual editing close their wider criteria. [Manual counts and the annotation register](MARKUP_REGISTER.md) already exist within their documented 0.41.0 bounds. Local tools remain free and open source. The [roadmap](ROADMAP.md) records the qualified scopes of the 0.45 annotation library, 0.46 quantity tables and 0.47 revision overlays.
