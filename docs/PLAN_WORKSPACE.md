# Plan workspace: precise scales, batches and responsive tables

[Français](PLAN_WORKSPACE.fr.md) · [0.60 release scope](RELEASE_0.60.0.md) · [Measurements](MEASUREMENTS.md)

## Scales and vector snapping

Open **Tools → Review & measure → Measurements**. Calibrate from two known drawing points and confirm a positive real distance. Decimal input uses the chosen unit. Feet/inches notation such as `5' 6 1/2"`, `5 ft 6 in` or `6 1/2 in` converts to that unit; mixed signs, zero denominators and ambiguous trailing text are refused. Check a second known dimension before using the plan for quantities.

Enable vector snapping to display the nearest eligible endpoint or straight-line intersection while placing a point. The tolerance follows zoom so the indicator stays near the pointer. Curve control handles, text/image bounding rectangles and raster scans do not become measurement points. Page CropBox and rotation, nested form/object matrices and supported convex clipping are applied. Clipping does not invent new endpoint candidates where a line meets a clip boundary. Unsupported curved/non-convex clips are omitted and reported; excessive geometry or dense neighborhoods can decline snapping. Use explicit manual points in these cases.

Create a named region after calibrating its reference inside that region, then place its two opposite corners. Save the region and save the PDF to persist it. Each region has its own stable identity and calibration, independent of the page default and every saved annotation. The active scale remains visible. Automatic selection accepts only an unambiguous containing region; overlaps and crossing geometry require an explicit scale choice. Choose a region to constrain geometry to its bounds, or explicitly choose the page scale when appropriate. Updating/deleting a region changes future drawing only.

Region metadata is a bounded `SuniPDFMeasurementViewports` page record in PDF user space, with page bounds and decoded-content digest. It follows page rotation and eligible page operations. Changed content/bounds, unknown records and malformed metadata are refused rather than overwritten. Metadata stripping, sanitization and redaction remove this private page record. These checks are not comprehensive document authentication: confirm the drawing revision yourself.

## Recalibrate a selected batch

1. Confirm the intended reference scale. Open batch recalibration and choose the physical page range.
2. Read the selected pages. Choose the expected saved measurements and whether to replace page defaults. Angles have no length scale and are excluded.
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
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json --out recalibrated.pdf
sunipdf-cli measurement-snap input.pdf --page 1 --point "210,320" --tolerance 5
```

The `expected` and `regions` files contain arrays. A region has exactly `id`, `name`, `bounds` (`[left,bottom,right,top]`) and `calibration` (`points`, `known_distance`, `unit`). The reference points use displayed-page coordinates, bottom-left origin. Use the `regions` array from the read command as the exact expected state. Each JSON file is limited to 128 KiB.

`recalibration.json` is an array of objects containing exactly `page` (1-based), `calibration`, `update_page_default` (boolean), and `measurements` (complete expected records from `measurements` JSON, including stable annotation IDs). The request is bounded to 4 MiB. An empty measurement array means no saved measurements, never all. Omit `--out` for a read-only preview. With `--out`, the same validated plan is applied in a new copy; existing outputs and the original are refused even with `--overwrite`. Region inventories, snap exports and batch value previews require copying permission. Use `--password-env` or `--password-stdin` for credentials.

The automated qualification corpus uses known synthetic geometry and retained licensed/public PDFs. Private project-plan qualification is deferred at the owner's request. Physical devices, accessibility, complete interoperability and the remaining 1.0 gates stay open.
