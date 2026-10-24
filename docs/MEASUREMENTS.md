# Local calibrated measurements
[AEC backlog](AEC_BACKLOG.md)

The [0.60 plan workspace](PLAN_WORKSPACE.md) adds vector snapping, feet/inches entry, independent named regions and explicit atomic batch recalibration. See its workflow and qualification limits.

**Available in [0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0):** open **Tools → Review & measure → Measurements**. Its right-hand settings panel groups Create measurement, Page scale, Drawing and Saved data; **Saved measurements** is in Saved data. [Reader and tools guide](READER_WORKSPACE.md). [Evidence](qa/RELEASE_0.50.0.json).

**Available in [0.49.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.49.0):** add or remove saved polyline, perimeter and polygon-area vertices within their minimum and maximum counts; select a vertex by number even when handles overlap. Preview, Apply and Undo preserve the measurement’s own calibration and entered quantity fields. [Workflow and limits](MEASUREMENT_GEOMETRY.md). [Evidence](qa/RELEASE_0.49.0.json).

**Available in [0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0):** drag existing saved measurement vertices through **Edit geometry**, then Preview and Apply. Calibration and entered quantity fields are preserved. [Workflow and limits](MEASUREMENT_GEOMETRY.md). [Evidence](qa/RELEASE_0.48.0.json).

**Available in [0.44.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.44.0):** Edit one saved measurement's reference distance, unit and precision through Preview and Apply; the CLI also supports explicit geometry replacement with an expected record. Identity, replies, page defaults and other measurements are preserved. Temporary scales can be cleared, the inspector shares cached records, exports enforce copying permission and incompatible measured-page resizing is refused. [Evidence](qa/RELEASE_0.44.0.json).

**Available in [0.43.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.43.0):** persistent page scales, inspection and explicit scale reuse extend on-page drawing. Existing measurements retain their values. [Evidence](qa/RELEASE_0.43.0.json).

## 0.63 candidate extension

The candidate adds [deducted polygon areas and constant-depth volumes](MEASUREMENT_VOLUMES.md), with separate
interior contours, physical depth, cubic quantities and version-3 records. The published baseline remains
[0.62.0](RELEASE_0.62.0.md). Final-source, native and package qualification of the candidate are tracked
separately; the earlier release evidence does not qualify these additions.

## Measure directly on the page

1. Open a PDF and choose **Tools → Review & measure → Measurements**. Use the settings panel on the right; its sections remain available by scrolling when the window is smaller.
2. Choose `mm`, `cm`, `m`, `in` or `ft` and 0–4 decimal places. Select **Calibrate this page**, then click two different points spanning a distance you know from the drawing. Enter the positive **Known distance**, then select **Confirm scale**. Feet/inches notation is accepted: `10' 6"`, `10.5 ft` or `126 in` describe the same physical distance.
3. Choose a measurement type and click directly on the page. **Length** opens the preview after two points. **Angle** opens it after three points: first arm, vertex, second arm; angles do not require a physical scale. For **Polyline length**, **Polygon perimeter** and **Polygon area**, add the vertices, then select **Preview measurement**, press **Enter**, or double-click the last point. Polygons close automatically; do not repeat the first vertex.
4. Inspect the rendered candidate and its value, then select **Apply**. Nothing is added until Apply succeeds. **Cancel** or **Escape** discards the preview. Save the PDF to retain the measurement; one Undo removes the annotation and its metadata together. The tool stays active after applying, so another measurement on the calibrated page can be drawn immediately.

While drawing, **Backspace** or **Undo point** removes the last point. **Shift** constrains the next segment horizontally or vertically in the displayed page's coordinate axes; this is not snapping to existing PDF lines. **Escape** or **Cancel drawing** abandons the current draft. Finish or cancel a draft before drawing on another page. Drafts are also cancelled when the document changes.

## Dimensioned rectangles

The rectangle increment creates a four-vertex area measurement from a real width and height. Coordinates use the displayed page after crop and rotation: the anchor is its lower-left corner, and the rectangle extends right and up. Dimensions accept the same decimal, fraction and feet/inches input as calibration. The shared engine converts the chosen reference to the requested unit and checks all four corners; it never clips an oversized rectangle or substitutes another scale after a refusal.

In **Measurements → Dimensioned rectangle**, choose Width, Height, Unit, Decimal places and the scale source, then **Place rectangle**. Only the anchor can snap to eligible vectors; snapping the remaining corners would change the entered dimensions. The placement outline is indicative. Review the source-bound candidate with **Preview**, then **Apply**. Dimensions, anchor and reference are read-only in this preview; **Cancel** returns to setup. Saving retains a normal area measurement with its identity, calibration and current provenance. Later polygon edits can change its shape: persistent rectangular constraints and circles are outside this rectangle increment. Deducted areas and constant-depth volumes are a separate 0.63 candidate workflow linked above.

In the 0.63 candidate, rectangle controls remain available in all measurement modes except **Volume**; an active rectangle placement keeps the controls visible even in Volume mode. Creating a rectangle still produces an area measurement.

The scale choice is explicit. A selected region must contain the entire rectangle. Automatic selection refuses overlaps, boundary crossings and regions enclosed by the rectangle; it uses the page default only when no region applies. Selecting the saved page scale explicitly permits crossing a region using that page scale. A confirmed temporary reference is bound to the document context captured at calibration. Annotation-only edits may retain it; changed content, geometry, scales or protection require a fresh reference. Preview-to-Apply binding still checks the complete document, including annotations. Missing, stale or unsupported selected scales are errors, never an implicit fallback.

```powershell
sunipdf-cli dimensioned-rectangle input.pdf --anchor "40,80" --width 2 --height 3 --unit m --calibration "40,40 140,40" --known-distance 1 --out rectangle.pdf
sunipdf-cli dimensioned-rectangle calibrated.pdf --anchor "40,80" --width 200 --height 300 --unit cm --use-page-calibration --out rectangle-cm.pdf
sunipdf-cli dimensioned-rectangle regions.pdf --anchor "40,80" --width 2 --height 3 --unit m --region detail-a --out rectangle-region.pdf
sunipdf-cli dimensioned-rectangle regions.pdf --anchor "40,80" --width 2 --height 3 --unit m --automatic-scale --out rectangle-auto.pdf
```

The first example creates 6 m² using a reference of 1 m per 100 page points. Select exactly one of `--calibration` with `--known-distance`, `--use-page-calibration`, `--region ID`, or `--automatic-scale`. An explicit reference uses `--unit` unless `--calibration-unit` supplies a different unit. `--precision` accepts 0–4; `--page` is 1-based and defaults to 1. `--author` is optional. Every write requires a new `--out`, even with `--overwrite`; source paths, aliases and existing destinations are refused.

Use `--password-env` or `--password-stdin` for encrypted inputs. Annotation permission is required, and permitted encryption is retained. With copying forbidden, an allowed annotation can be written, but the response contains `measurement: null` and `export_unavailable: true`; stored values and references are not exported. Successful readable results use the existing version-2 measurement JSON with `validity: current`. Reopen with `measurements` or `quantity-table` to inspect the saved area.

The implementation includes dedicated rotation/crop, scale-selection, no-clobber, encryption and quantity regression cases. Exact-source CI and earlier native development-build results are recorded in the [0.62.0 release qualification](RELEASE_0.62.0.md).

## Session scale and saved page default

Each page can have a separate confirmed scale for the current session. Ordinary annotation edits and normal tab switches retain these scales; changes to page geometry, structure or content clear them. A session scale takes precedence over that page's saved default. Navigating between pages never assigns one page's scale to another. Changing the display unit converts the confirmed distance instead of reinterpreting its numeric value.

After confirming a scale, select **Save scale in PDF**, then save the PDF file to retain it on disk. The successful operation replaces the session override with a saved page default. Reopening the file loads the supported saved default automatically for new measurements. A session scale that was not saved is lost when the document closes.

**Remove saved scale** removes only that page's saved default; an explicitly chosen session scale remains available. Save the PDF to retain the removal. **Undo** and **Redo** restore the corresponding document state and reload its saved default; they do not restore a cleared session scale. Saving/removing a page default requires document-modification permission; drawing an annotation requires annotation permission. The save/remove controls stay disabled without document-modification permission. AES-128/AES-256 protection is retained.

The current saved default records PDF user-space points, page bounds and a digest of page content, resources and rendering context, including transparency groups and optional-content settings. Rotation and page reordering preserve an eligible default. Changed crop bounds, resized/content-modified pages, unsupported metadata and non-default `UserUnit` are refused rather than treated as valid scales. Older defaults without the complete rendering guard require explicit recalibration or removal; they are not silently qualified from the current page. Recalibrate or remove a supported stale default; unknown or malformed records are preserved and reported as errors. These checks are not comprehensive revision authentication: always verify the plan and its known dimension.

Each applied annotation embeds its own two-point calibration, geometry, unit and precision. Saving, removing or replacing the page default never recalculates existing annotations. Use the explicit [batch recalibration and named-region workflow](PLAN_WORKSPACE.md) to change selected records or future drawing scales.

In 0.44, **Use saved page scale** discards the current page's temporary override when its saved default is valid. If no valid saved default is available, **Clear temporary scale** discards only the temporary override. These controls do not edit the PDF or other pages. An unavailable saved default remains unavailable; calibrate the page explicitly before drawing a scaled measurement.

## Inspect saved measurements

In **Measurements**, select **Saved measurements** to inspect the current page. Reading the list leaves the PDF unchanged. It shows the calculated label and unit, measurement type, annotation index, optional identity, vertex count, precision and known reference distance for non-angle measurements. Lists use batches of 50; **Refresh** rereads the current document state. Unsupported or changed annotation metadata is reported explicitly. In 0.44, records and labels are shared in memory instead of copying the complete list and generating result JSON every frame.

**View on page** selects the actual annotation and opens its comments. **Use this scale** copies a current non-angle annotation's calibration into the current page's session, without editing the annotation or saving a page default. Historical or stale references and references outside the displayed page are refused. Use **Save scale in PDF** separately if this is the scale you want to retain for future sessions. Close the inspector before starting a drawing or changing documents.

Generic move, resize, restyle and value-text edits are refused to prevent stale labels. Delete and comment replies remain available within existing PDF permissions. Manual coordinate entry remains available in **Rich annotations → Calibrated measurements**; its `x,y` values use displayed-page points with the origin at the bottom left after rotation and cropping. In 0.44, this manual path no longer supplies an arbitrary positive reference distance: enter a valid calibration explicitly.

## Edit one saved measurement

These ordinary edits require a measurement verified against the current page. A `legacy_unverified` or `stale` measurement first requires an explicit fresh calibration; changing its unit or precision alone cannot restore its validity. Historical angles must be recreated on the current page.

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

This example converts a current 10 m length into 1000 cm without changing its physical size. `--expected-record` accepts one complete version-2 export, including validity and stable annotation identity, up to 64 KiB. The 0.63 candidate also accepts complete version-3 exports; `holes` and `depth` must be present even when empty or null. Legacy version-1 exports are accepted only as unverified records. The current page/index/identity/geometry/calibration/precision and derived validity must match the expected record, including version-3 holes and physical depth when present; editing its declared validity cannot qualify the measurement. Re-export after an edit. `--points` can replace up to 128 displayed-page vertices, and `--kind` can change the type when the geometry is valid and any retained unit price remains compatible. To replace a reference, provide `--calibration` together with `--known-distance` in the resulting unit, or explicitly select `--use-page-calibration`. An explicit reference is required to recalibrate a legacy or stale scaled measurement; unit-only edits are refused. A change from an angle to a scaled measurement needs a physical reference. `measure --kind angle` needs three points and no calibration.

In the 0.63 candidate, repeated `--hole` options replace the complete deduction list; omission preserves existing holes, and `--clear-holes` removes them explicitly. `--depth` and `--depth-unit` require each other; omission preserves the physical depth independently of `--unit`. Converting a volume to an area requires `--kind area --clear-depth`, and an incompatible retained price refuses the operation. See the [volume workflow](MEASUREMENT_VOLUMES.md) for topology, storage and quantity limits.

In 0.44, `measurements` and `page-calibration` require copying permission. A creation/edit permitted by annotation rights can still succeed without copying permission, but its JSON response contains `measurement: null` and `export_unavailable: true`; it does not export stored calibration or quantities. `edit-measurement` also requires a new destination even with `--overwrite`.

The manual examples measure 10 m and 50 m²; the saved-scale example produces 1000 cm. `--use-page-calibration` is mutually exclusive with `--calibration` and `--known-distance`; it fails if no valid saved default exists. `--unit` physically converts the saved reference distance. `page-calibration` returns JSON with a `calibration` object or `null` for each page; omit `--page` to read at most 1000 pages. Saving/removing requires an explicit page and never changes existing measurement values.

The create command returns its annotation index and result; `measurements` exports per-page records as JSON, including annotation identity, geometry, calibration, unrounded numeric value, display label and precision. Areas use squared units (`m2`), candidate volumes use cubic units (`m3`), angles use degrees (`deg`), and lengths/perimeters use the selected linear unit. JSON contains no spreadsheet formulas, macros or external references. Pages are 1-based and annotation indices are 0-based. Prefer `--password-env NAME` or `--password-stdin` for protected documents.

`measure`, `set-page-calibration` and `remove-page-calibration` always write new copies and refuse existing destinations or the original path even with `--overwrite`. Page-default changes require document-modification permission; measurement creation requires annotation permission only. Permitted AES-128/AES-256 protection and both credentials are preserved.

## Open metadata contract

A real Stamp annotation carries a rendered appearance and a `SuniPDFMeasurement` JSON string. Ordinary measurement writes use version 2: `coordinate_system: "pdf-user-space"`, kind, user-space vertices, two calibration points, known distance, unit, precision and the user-space rectangle, plus `provenance` containing page bounds, `UserUnit` and a rendering-context digest. Export recalculates from these inputs rather than trusting editable comment text. Rotation preserves the physical model; changes to guarded content, resources, rendering context or crop bounds invalidate the stored provenance.

The 0.63 candidate writes version 3 for a deducted area or a volume, adding `holes` and `depth` to the same guarded model. The outer boundary and each hole have separate user-space vertices; depth retains its explicit physical value and unit. Readable exports add gross/deducted/net `area` and `area_unit`; stale or unverified surface results expose `historical_area` while the current `area` is null. Older readable records retain their validity rules.

Exports use `validity: current`, `legacy_unverified` or `stale`. Version-1 metadata remains readable as historical data because it has no provenance. Non-current records expose `value` and `label` as `null`, retaining `historical_value` and `historical_label`; their prices and entered fields are preserved, while their quantities and costs are excluded from totals. A formula requesting an excluded quantity returns an explicit error. Reading, changing units or editing a label never silently promotes a historical record. Explicit recalibration establishes a fresh reference for scaled measurements; historical angles must be recreated. Unsupported versions, malformed metadata, externally moved/resized rectangles and recognized imported `/Measure` metadata are refused explicitly. Original files are retained so that other software can be used when an imported format is unsupported.

A separate `SuniPDFPageCalibration` page string stores the default for new measurements. Current writes use version 3 and include the geometry and rendering-context guard. Earlier version-1/2 defaults require explicit recalibration or removal before reuse. The record is bounded to 4096 bytes and uses PDF user-space points. Content verification accepts at most 1024 page streams, 8 MiB decoded per stream and 32 MiB total. This metadata does not replace any annotation's own calibration.

This is a SuniPDF exchange contract, not third-party proprietary-format compatibility. An annotation modified by another editor may need to be recreated. The metadata and ordinary PDF appearance are not a digital signature or proof of dimensional accuracy: the operator must verify the known distance and drawing revision.

## Bounds and remaining work

JSON export is bounded to 1000 pages and 4096 records, with 4 MiB of measurement metadata per page. Ordinary measurements have at most 128 vertices. Candidate deducted areas and volumes accept at most 16 holes, 128 vertices per ring and 512 vertices in total. Geometry requires finite in-page coordinates, positive calibration, no zero-length segments, simple non-degenerate polygons and results between 1e-9 and 1e12. Self-intersecting or invisible geometry is refused. The annotation includes a label strip and may overlap existing drawing content; inspect the preview before applying.

Known-geometry fixtures cover arithmetic, metric/imperial equivalence, rotation/crop, reopen, preview/commit, snapshot Undo, allowed encryption, invalid geometry and stale-record refusals. The 0.43 additions cover page-default persistence, permissions, physical unit conversion and independent qpdf checks within the existing corpus; executed results belong in Project status. Generated fixtures do not qualify survey accuracy, real customer plans, physical printing, native pointing/Narrator interaction, PDF/A or PDF/UA.

Resizing content with annotations refuses measured pages atomically rather than leaving inconsistent rectangles and quantities. Saved polylines support 2–128 vertices; perimeters and polygon areas support 3–128. Length and angle measurements keep their fixed point counts. 0.60 adds bounded batch recalibration, eligible vector snapping and named multi-scale regions. Dimensioned drawing now has the limited rectangle increment described above, with final qualification tracked separately. Deducted areas and constant-depth volumes are implemented in the 0.63 candidate and still await final qualification. Persistent shape constraints, circles, variable-depth solids and advanced recognition remain separate work. AEC-001/002/003/004/008/009/026 remain partial; neither page defaults, the inspector, individual editing nor this rectangle close their wider criteria. [Manual counts and the annotation register](MARKUP_REGISTER.md) already exist within their documented 0.41.0 bounds. Local tools remain free and open source. The [roadmap](ROADMAP.md) records the qualified scopes of the 0.45 annotation library, 0.46 quantity tables and 0.47 revision overlays.
