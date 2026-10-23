# SuniPDF 0.47.0 — aligned revision overlays
[Revision overlay guide](REVISION_OVERLAY.md)

Status: published as stable Latest [0.47.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.47.0), source `db79bc393cd6039a53b03319c503ab57483783d8`, on 2026-10-22. [Previous release](RELEASE_0.46.0.md).

## Scope

**Revision overlay** opens from **All tools** or **Compare files**, without requiring an open document. Select one physical page from each of two saved local PDFs. The base appears in magenta and the revision in cyan, with adjustable revision opacity. Unsaved tab changes are not included.

Align the revision through explicit translation, clockwise rotation and uniform scale, or click two corresponding points on each source image. Preview again after changing the settings; reset clears the alignment. Coordinates are displayed PDF points from the top-left corner, with crop and page rotation accounted for. The base page defines the output canvas; revision content outside it is clipped.

Each PDF is limited to 64 MiB. Each source raster is bounded independently, with a default maximum side of 1536 pixels and an upper limit of 2048. The worker supports cancellation and retains no password in the prepared preview. Both PDFs must permit copying; separate passwords are supported.

Desktop and CLI export a new PNG or JSON report. The report records both source hashes, selected pages, geometry, alignment and rendering settings. Exports recheck the saved sources against their recorded SHA-256 hashes and refuse existing destinations. Neither PDF is modified.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39743753731) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39743766468) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39743753810). 744 tests passed, 10 default ignored. 1382 EN/FR keys. 39 portable scenarios. 33 MSI baselines. 165 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 28 packaged CLI calls verify cropped pages at four rotations, PNG/JSON readback, explicit and two-point alignment, source hashes, stale/overwrite refusals, separate AES passwords, copy restrictions and unchanged sources. [Evidence](qa/RELEASE_0.47.0.json).

## Remaining limits

This is a raster overlay of two explicitly selected pages. Alignment is manual or based on two point pairs; there is no automatic registration, vector comparison, OCR, merged PDF or automatic change outline. Scaling cannot restore detail lost in the source raster. The overlay does not change measurements, page calibration or PDF content. AEC-013 remains partial; AEC-014 remains open. Native/hardware and 1.0 qualification gates remain separate.
