# SuniPDF 0.47.0 — aligned revision overlays

[Français](RELEASE_0.47.0.fr.md) · [Revision overlay guide](REVISION_OVERLAY.md)

This build includes the scope below. Published assets and qualification are tracked on the [0.47.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.47.0).

## Scope

**Revision overlay** opens from **All tools** or **Compare files**, without requiring an open document. Select one physical page from each of two saved local PDFs. The base appears in magenta and the revision in cyan, with adjustable revision opacity. Unsaved tab changes are not included.

Align the revision through explicit translation, clockwise rotation and uniform scale, or click two corresponding points on each source image. Preview again after changing the settings; reset clears the alignment. Coordinates are displayed PDF points from the top-left corner, with crop and page rotation accounted for. The base page defines the output canvas; revision content outside it is clipped.

Each PDF is limited to 64 MiB. Each source raster is bounded independently, with a default maximum side of 1536 pixels and an upper limit of 2048. The worker supports cancellation and retains no password in the prepared preview. Both PDFs must permit copying; separate passwords are supported.

Desktop and CLI export a new PNG or JSON report. The report records both source hashes, selected pages, geometry, alignment and rendering settings. Exports recheck the saved sources against their recorded SHA-256 hashes and refuse existing destinations. Neither PDF is modified.

## Qualification

Qualification records and distribution assets are tracked on the [0.47.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.47.0). Results apply to the exact source identified there.

## Remaining limits

This is a raster overlay of two explicitly selected pages. Alignment is manual or based on two point pairs; there is no automatic registration, vector comparison, OCR, merged PDF or automatic change outline. Scaling cannot restore detail lost in the source raster. The overlay does not change measurements, page calibration or PDF content. AEC-013 remains partial; AEC-014 remains open. Native/hardware and 1.0 qualification gates remain separate.
