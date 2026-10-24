# SuniPDF 0.72.0

Publication: candidate under qualification. See [project status](PROJECT_STATUS.md)
for the currently published release and its downloads.

## Review enclosed regions as area measurements

Open **Tools → Review & measure → Region fill**, select a point inside an enclosed
region and add explicit closure segments where required. Resolution and grayscale
threshold control the detected boundaries. Fit, zoom and pan help work on larger pages.

The result retains the actual raster contour and enclosed holes. Review its calibrated
area preview, deductions and selected scale before creating one editable measurement.
One Undo removes that operation. Existing page, named-region and confirmed temporary
scales are reused; ambiguous or stale references are refused.

Processing advances through bounded, cancellable phases. Leaks to the page edge,
ambiguous topology, excessive geometry and resource limits remain explicit refusals.
Incomplete results cannot be applied. There is no hidden gap closure, smoothing or
replacement of the region by its bounding rectangle.

The CLI provides inspection, a strict review file, a PNG measurement preview and
new-copy output. It binds the saved source and inspection results before application.
Historical generation metadata retains the seed, closures, geometry, raster settings
and measurement inputs separately from the existing measurement record.

[Workflow, limits and CLI examples](REGION_FILL.md).

## Qualification scope

Source tests and an independent synthetic corpus are being qualified. Downloaded
packages and publication have not been completed. Native desktop interaction is
untested in this session. AEC-012 remains **In progress**; no complete priority
category, universal interoperability or physical measurement accuracy is claimed.
