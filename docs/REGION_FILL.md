# Region fill

This workflow requires version 0.72 or later. See [project status](PROJECT_STATUS.md)
for the currently published release and its qualification.

## Trace and review an area

1. Open the page and establish a known [measurement scale](MEASUREMENTS.md). A saved
   page scale, named scale region or confirmed temporary reference can be used.
2. Open **Tools → Review & measure → Region fill**.
3. Use **Fit page**, zoom and **Pan image** to locate the room or region. Disable pan
   before drawing. Choose **Pick region** and click inside the intended area.
4. If a doorway or other opening needs a deliberate closure, choose **Close a gap**
   and drag a segment between its sides. Yellow segments are manual boundaries;
   **Undo closure** removes the last one. **Reset selection** clears the seed and closures.
5. Choose the resolution and **Dark boundary threshold**, then the **Scale**, **Unit**
   and **Decimal places**. **Automatic (saved regions/page)** refuses ambiguous scale
   regions. **Saved page scale** is an explicit override of named regions.
6. Choose **Find region**. Inspect the blue outer contour and red excluded islands.
   A region that leaks to the page edge, exceeds a limit or has ambiguous geometry
   cannot become a measurement. **Stop search** cancels processing.
7. For a complete result, choose **Preview area**. Check the actual area preview,
   deductions, net value and scale. Confirm **I reviewed the boundary, deductions and
   scale**, then choose **Create area**.
8. One Undo removes the created measurement. Save the PDF to retain it. The area is
   available through the existing measurement, annotation and quantity workflows.

Changing the seed, closures, resolution, threshold, scale, unit or precision discards
the previous result and preview. Run **Find region** again after these changes. Editing
or switching documents also invalidates the review. Close and Escape discard transient
tool state; reopening starts on the current page. Already created measurements remain
in the document.

## Boundaries, holes and scale

The engine uses page content, excluding annotation and form-widget appearances.
Pixels at or below the selected grayscale threshold are boundaries. Starting from
the selected free pixel, it follows four-connected free cells. Reaching a page edge
is reported as leakage. Blank pages, open boundaries and seeds on dark lines do not
silently produce a usable room measurement.

Manual closures cover the raster cells touched by the supplied line, including both
side cells when it crosses a grid corner. They are temporary detection inputs, not
changes to the page artwork. Their positions, raster resolution and threshold are
retained in historical generation metadata on the resulting annotation.

Contours follow the actual filled cells and retain enclosed holes. Only exactly
collinear vertices are removed. There is no smoothing, automatic gap closure or
conversion into a bounding rectangle. Ambiguous corner contacts and contours exceeding
the supported geometry limits are refused. Lowering resolution can change the result;
review it again instead of assuming the same area.

The displayed geometry is an approximation controlled by the raster resolution and
threshold. A calibrated value does not establish the physical accuracy of the drawing
or its detected boundary. Existing annotations are excluded from detection and retained
in the PDF. Printed text, hatching and other page-content marks can create additional
boundaries or holes; inspect their effect on deductions before accepting an area.
Historical fill metadata describes the original generation; later edits do
not make that record a current reconstruction of the edited measurement.

## Limits

- One page per operation, at 18, 36, 72 or 144 dpi in the desktop (0.25–2 pixels per
  displayed PDF point). Maximum raster size: 4,096 pixels per side and 8 Mi pixels.
- Up to 32 explicit closure segments; at most 262,144 frontier cells and boundary edges.
- Up to 16 holes, 128 vertices per ring and 512 vertices overall, with strict topology checks.
- The desktop uses a 32 million work-unit limit. The CLI exposes bounded resource
  settings up to 64 million work units. A limited or cancelled region is never applicable.
- Source snapshots are limited to 128 MiB. Copying and annotation permissions are
  required for the corresponding operations; digitally signed documents cannot be edited.

Raster processing advances between foreground work. Loading and rendering the page
remain non-preemptive native operations. Automated checks, rendered previews and native
desktop interaction are separate qualification scopes.

## Command line

Create a review with no area accepted. Coordinates are displayed-page points with a
bottom-left origin; command-line and review page numbers start at one.

```powershell
sunipdf-cli region-fill inspect plan.pdf --page 1 --seed 120,90 --scale 1 --threshold 160 --closure 80,40,110,40 --review-out region.json
```

The strict review retains the saved-file hash and the reproducible inspection hash.
Its `reference` can be `{"kind":"automatic"}`, `{"kind":"page"}`, a named region
such as `{"kind":"region","id":"room-scale"}`, or an explicit temporary reference:

```json
{"kind":"local","points":[[0,0],[100,0]],"known_distance":1,"unit":"m"}
```

Choose `unit` (`mm`, `cm`, `m`, `in` or `ft`), `precision` (0–4) and optional `author`.
Preview the calibrated area before setting `accepted` to `true`:

```powershell
sunipdf-cli region-fill preview plan.pdf --review region.json --out region-preview.png --long-side 1024
sunipdf-cli region-fill apply plan.pdf --review region.json --out measured-plan.pdf
```

Preview does not modify the PDF or accept the review. Apply repeats the inspection,
checks the source and results, and writes a new PDF only. Existing outputs are refused
even with `--overwrite`. Changed sources or inspection settings require a fresh review.
Incomplete inspections return exit code 2 with their explicit status. There is no
option to apply a leaked, partial, cancelled or otherwise refused region.
