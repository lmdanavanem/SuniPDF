# SuniPDF 0.60.0 — Plan workspace and calibrated measurement workflow

[Français](RELEASE_0.60.0.fr.md) · [Workflow and limits](PLAN_WORKSPACE.md)

Status: release candidate. Native qualification is required before publication.

- Reuse page layout calculations while scrolling and cache unchanged register filters, sorting and quantity totals.
- Read document searches and annotation inventories page by page, report progress and accept cancellation between pages so visible-page requests can be serviced.
- Keep annotation and quantity tables in a resizable bottom dock or floating window; remember placement and height and navigate from a row to its annotation and contextual properties.
- Snap to eligible PDF vector endpoints and straight-segment intersections, with a visible candidate indicator. CropBox, page rotation, object transforms and supported convex clipping are applied; scans do not produce synthetic vector points.
- Enter known distances as decimals or feet/inches, including fractions such as `5' 6 1/2"`, with explicit unit conversion.
- Save named measurement regions with independent scales. Select overlapping regions explicitly; geometry crossing the selected region is refused. Existing annotations retain their own calibration.
- Preview recalibration on explicitly selected pages and records, then apply the entire batch with one Undo step. Stale documents, invalid later pages, unsupported records and signed documents are refused; AES permissions and annotation identities remain intact.
- Add CLI commands for region inventories and guarded replacement, recalibration previews/new copies and vector snap inspection.
- Extend Windows upgrade regressions with the published 0.51.0 portable and MSI packages. Preserve Ubuntu amd64 and Windows package formats.

## Qualification boundary

The new automated cases use synthetic PDFs with known dimensions and the retained, licensed/public regression corpus. They cover numeric geometry, crop/rotation, nested transforms, unsupported clips, expected records, reopen, atomic refusal, Undo and encryption. This is the implemented 0.60 P1 product scope; source-specific test results and package evidence are recorded separately.

Private real-project plans are deferred at the owner's request. Survey accuracy, physical pointing/printing, screen readers, native ARM64 hardware, complete independent-reader interoperability, update authenticity and the remaining [1.0 criteria](RELEASE_1.0_CRITERIA.md) remain open. A synthetic corpus does not close those gates. No frame-time or annotation-filter latency target is claimed without a recorded benchmark.

The batch is bounded to 512 pages, 4096 selected measurements and a 128 MiB document. A page supports at most 64 regions. Snapping contributes real curve endpoints and straight-segment intersections; curved intersections and unsupported non-convex/curved clipping are excluded. Dense neighborhoods may decline snapping. Region/page defaults do not silently recalculate saved annotations. Dimensioned drawing, volumes, deductions and automatic symbol recognition remain separate backlog work.

Windows packages remain unsigned; Windows ARM64 is cross-built. Ubuntu 22.04/24.04 amd64 retains the [documented Linux feature limits](UBUNTU.md). Projects updates remain deferred.

New saved page scales use the resource-aware version-2 guard. A legacy version-1 scale on a page with resource dependencies requires explicit recalibration or removal, because its old digest could not establish whether a resource changed. Supported stale region records can be cleared with a captured-state guard, then recreated; unknown or malformed metadata remains preserved. The CLI exposes this through `clear-measurement-viewports` into a new copy.
