# Plan workspace: precise scales, batches and responsive tables
[0.60 release scope](RELEASE_0.60.0.md) · [Measurements](MEASUREMENTS.md)

## Scales and vector snapping

Open **Tools → Review & measure → Measurements**. Calibrate from two known drawing points and confirm a positive real distance. Decimal input uses the chosen unit. Feet/inches notation such as `5' 6 1/2"`, `5 ft 6 in` or `6 1/2 in` converts to that unit; mixed signs, zero denominators and ambiguous trailing text are refused. Check a second known dimension before using the plan for quantities.

Enable vector snapping to display the nearest eligible endpoint or straight-line intersection while placing a point. The tolerance follows zoom so the indicator stays near the pointer. Curve control handles, text/image bounding rectangles and raster scans do not become measurement points. Page CropBox and rotation, nested form/object matrices and supported convex clipping are applied. Clipping does not invent new endpoint candidates where a line meets a clip boundary. Unsupported curved/non-convex clips are omitted and reported; excessive geometry or dense neighborhoods can decline snapping. Use explicit manual points in these cases.

Create a named region after calibrating its reference inside that region, then place its two opposite corners. Save the region and save the PDF to persist it. Each region has its own stable identity and calibration, independent of the page default and every saved annotation. The active scale remains visible. Automatic selection accepts only an unambiguous containing region; overlaps and crossing geometry require an explicit scale choice. Choose a region to constrain geometry to its bounds, or explicitly choose the page scale when appropriate. Updating/deleting a region changes future drawing only.

Region metadata is a bounded `SuniPDFMeasurementViewports` page record in PDF user space, with page bounds and decoded-content digest. It follows page rotation and eligible page operations. Changed content/bounds, unknown records and malformed metadata are refused rather than overwritten. Metadata stripping, sanitization and redaction remove this private page record. These checks are not comprehensive document authentication: confirm the drawing revision yourself.

The current content guard includes reachable page resources such as Form XObject streams and matrices, page transparency groups and catalog optional-content configuration. New saved page defaults use version 3 and region records use version 2. Older page-default versions 1/2 and region version 1 require explicit recalibration or removal; their historical rendering context cannot be established safely. If a supported region record becomes stale, explicitly clear the supported regions and recreate them. The reset captures the document state and refuses a later change; it does not remove measurements or their independent scales. Unknown/malformed records cannot obtain a reset target.

Content and resource traversal each have a 32 MiB aggregate limit, with an 8 MiB limit per stream. The resource guard also limits traversal to 50,000 nodes and 64 levels, rejects cycles, and resolves at most 64 inherited page parents. Files beyond these bounds require another workflow; a missing guard is never accepted as a valid saved scale.

## Place a rectangle with real dimensions

The [dimensioned rectangle workflow](MEASUREMENTS.md#dimensioned-rectangles) uses the same scale sources and page coordinates. Enter width and height, choose a scale explicitly and place the lower-left anchor. The rectangle remains aligned to the displayed page axes, including rotated or cropped pages. Only its anchor snaps; dimensions stay fixed through the placement preview. Review the candidate before applying it as an ordinary four-vertex area measurement.

Region selection validates the whole rectangle, including boundary crossings and enclosed regions. Automatic selection never chooses an arbitrary overlapping region. A saved page scale is an explicit override; a temporary reference must still match the captured content, geometry, scales and protection. Annotation-only changes may retain the local reference; applying a prepared candidate still requires its complete source state to match. Invalid geometry, an unavailable scale, unsupported page units or insufficient PDF permissions refuse the operation without adding an annotation. Save/reopen and quantity exports use the measurement's own persisted scale and provenance, independent of later page-default changes.

This increment covers axis-aligned rectangles. Subsequent polygon editing is unconstrained; circles, holes, deducted areas, depth and volumes remain separate backlog items. Automated cases are included; qualification of the exact candidate and native UI remains a separate release gate.

## Recalibrate a selected batch

1. Confirm the intended reference scale. Open batch recalibration and choose the physical page range.
2. Read the selected pages. The desktop batch replaces their page defaults; choose whether to include all their supported saved measurements. Angles have no length scale and are excluded. The CLI also supports explicit subsets of saved records and independently choosing whether to replace each page default.
3. Preview the before/after values. Each measurement retains its own display unit, precision, geometry, identity, replies and entered quantity fields; areas scale by the square of the linear ratio.
4. Apply once. All selected records are checked against the current document before one replacement. A stale document, invalid later page, protected/unsupported annotation or signed PDF refuses the operation. No earlier page is left partly recalibrated.
5. Use one **Undo** to restore the full batch, or save the PDF to retain it. Named region scales are unchanged by a page/measurement batch.

Page defaults require document-modification permission; saved measurements require annotation permission. Allowed AES protection is preserved. Bounds are 512 selected pages, 4096 selected records, 128 MiB per document, 128 vertices per measurement and 64 regions per page. Calibrations must fit every selected page; do not apply the same reference to differently scaled sheets without checking them.

## Tables and background reading

The annotation register and quantity table can be docked below the document or displayed in a floating window. Resize the bottom panel and select its placement; these preferences are saved. Selecting a table row follows the actual annotation and opens its contextual properties. Long tables display visible rows while unchanged filtering, sorting and totals reuse their cached results.

Search and annotation inventory advance one page at a time and expose progress/cancellation. Visible-page work can run between page tasks. Cancellation is cooperative: a single PDF engine call still finishes before the next page. Page layout is cached by document revision and layout parameters, so scrolling does not repeatedly rebuild all page rectangles. These changes reduce repeated work; recorded benchmarks define measured latency, not a universal speed guarantee.

## CLI

```text
sunipdf-cli measurement-viewports input.pdf --page 1
sunipdf-cli set-measurement-viewports input.pdf --page 1 --expected current-regions.json --regions next-regions.json --out regions.pdf
sunipdf-cli clear-measurement-viewports input.pdf --page 1 --out cleared-regions.pdf
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json --out recalibrated.pdf
sunipdf-cli measurement-snap input.pdf --page 1 --point "210,320" --tolerance 5
sunipdf-cli dimensioned-rectangle input.pdf --page 1 --anchor "40,80" --width 2 --height 3 --unit m --use-page-calibration --out rectangle.pdf
```

The `expected` and `regions` files contain arrays. A region has exactly `id`, `name`, `bounds` (`[left,bottom,right,top]`) and `calibration` (`points`, `known_distance`, `unit`). The reference points use displayed-page coordinates, bottom-left origin. Use the `regions` array from the read command as the exact expected state. Each JSON file is limited to 128 KiB.

`recalibration.json` is an array of objects containing exactly `page` (1-based), `calibration`, `update_page_default` (boolean), and `measurements` (complete expected records from `measurements` JSON, including stable annotation IDs). The request is bounded to 4 MiB. An empty measurement array means no saved measurements, never all. Omit `--out` for a read-only preview. With `--out`, the same validated plan is applied in a new copy; existing outputs and the original are refused even with `--overwrite`. Region inventories, snap exports and batch value previews require copying permission. Use `--password-env` or `--password-stdin` for credentials.

The automated qualification corpus uses known synthetic geometry and retained licensed/public PDFs. Private project-plan qualification is deferred at the owner's request. Physical devices, accessibility, complete interoperability and the remaining 1.0 gates stay open.
