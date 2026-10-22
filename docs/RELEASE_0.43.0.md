# SuniPDF 0.43.0 — saved page scales and measurement inspection

[Français](RELEASE_0.43.0.fr.md) · [Measurement guide](MEASUREMENTS.md)

Status: in preparation; not published.

Save a confirmed page scale inside the PDF and reload it when returning to the document. Saving or removing a page scale is an explicit document edit with Undo/Redo; save the PDF to retain the change on disk. A page's saved scale is separate from temporary calibration and from each existing measurement's immutable scale. Recalibrating new measurements never silently changes earlier quantities.

The Measurements workspace lists the current page's saved measurements, their values, units and calibration. Navigate to a measurement or explicitly reuse its physical scale for new measurements. Angles do not provide a physical scale. Reading the list leaves the PDF unchanged. Unsupported metadata is reported rather than guessed.

The CLI adds `page-calibration`, `set-page-calibration` and `remove-page-calibration`. `measure --use-page-calibration` explicitly uses the saved scale and converts it into the requested unit. Mutations write a new PDF copy, preserve supported encryption and refuse overwriting an existing destination or the original file. Saving page metadata requires document-modification permission; annotation-only PDFs can still use temporary calibration to add permitted measurements.

Page scales use a bounded, versioned open metadata record in PDF user coordinates. Rotation retains the physical scale; an incompatible crop or page geometry is refused. Page edits, document switches and delayed worker replies cannot silently transfer a scale to another page or overwrite a more recent local calibration. Existing saved annotations keep their own geometry and values.

Published 0.42.0 ZIP and MSI packages join the upgrade regression baselines. Validation results will be recorded after the checks complete.

Editing saved vertices, global recalculation, vector snapping, multiple-scale viewports and real-plan qualification remain open. Local features remain free and open source. Windows packages remain unsigned. ARM64 is cross-compiled; native ARM64 hardware and physical dimensional accuracy are not qualified by automated tests.
