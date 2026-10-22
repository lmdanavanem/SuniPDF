# Edit saved measurement geometry

[Français](MEASUREMENT_GEOMETRY.fr.md) · [Measurements](MEASUREMENTS.md)

This build adds on-page vertex editing for saved SuniPDF measurements. Distribution and qualification are tracked on the [0.48.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0).

## Workflow

1. Open a PDF containing a supported saved measurement and select **Measurements**.
2. In **Saved measurements**, choose the measurement and **Edit geometry**.
3. Drag an existing vertex on its original page. Handles keep a constant screen size as you zoom. Points stay within the page.
4. Select **Preview** to review the recalculated measurement, then **Apply** to accept it. **Adjust vertices** returns to the draft when further changes are needed.
5. Save the PDF through the normal save flow. **Undo** reverses the accepted edit in one step; **Cancel** or Escape discards the uncommitted draft.

Length, polyline, perimeter, polygon area and angle measurements retain their original type and vertex count. Geometry editing preserves the selected measurement's own calibration, reference distance, unit and precision. It does not replace them with a page default or the current drawing scale. Use the separate measurement settings editor to change those properties explicitly.

## Data and permissions

The existing annotation keeps its identity, replies, register fields and quantity metadata. Actual measured quantities and compatible unit-price totals are recalculated from the edited geometry; entered fields, price and price basis remain unchanged. Other measurements and page defaults are independent.

Preview and Apply use the existing guarded measurement operation. A changed source, unsupported record, locked annotation or missing annotation permission cannot be committed through this workflow. Invalid geometry, including collapsed or self-intersecting polygons, must be corrected before Apply. Page, tab and document changes invalidate stale drafts and previews.

## Scope

This edits existing vertices of supported saved measurements. Adding or removing vertices, vector snapping, batch recalculation and multiple-scale viewports remain separate work. Raster drawings do not acquire vector snapping from this feature. Automated interaction tests do not establish native hardware or accessibility qualification.

The existing `edit-measurement` CLI remains available for explicit geometry replacement in a new PDF copy with an expected record; see the [measurement guide](MEASUREMENTS.md). No new CLI command is introduced by the on-page workflow.
