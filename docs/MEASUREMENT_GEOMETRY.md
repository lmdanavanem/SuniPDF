# Edit saved measurement geometry

[Français](MEASUREMENT_GEOMETRY.fr.md) · [Measurements](MEASUREMENTS.md)

Available in [0.49.0](RELEASE_0.49.0.md). [Evidence](qa/RELEASE_0.49.0.json).

The workflow below uses the [reader and tools layout](READER_WORKSPACE.md) published in [0.50.0](RELEASE_0.50.0.md); the saved-geometry limits remain unchanged.

## Workflow

1. Open a PDF containing a supported saved measurement and choose **Tools → Review & measure → Measurements**.
2. In the right panel’s **Saved data** section, open **Saved measurements**, choose the measurement and **Edit geometry**.
3. Select the vertex by its number in **Vertex**, then drag its handle on the original page. The list also distinguishes vertices at the same position. Handles keep a constant screen size as you zoom; points stay within the page.
4. For a polyline, perimeter or polygon area, choose **Add vertex**, then click the segment where the new point belongs. A successful insertion adds one point to the draft and leaves insertion mode; choose **Add vertex** again for another. **Stop adding** cancels insertion mode. The closing segment of a perimeter or area is included. To remove a point, select its number and choose **Remove selected**.
5. Select **Preview** to review the recalculated measurement, then **Apply to object** to accept it. **Adjust vertices** returns to the draft when further changes are needed.
6. Save the PDF through the normal save flow. **Undo** reverses the accepted edit in one step; **Cancel** or Escape discards the uncommitted draft.

The measurement type stays fixed. A polyline must keep at least two vertices; a perimeter or polygon area must keep at least three. These shapes allow at most 128 vertices. Length measurements keep two points and angles keep three: their points can move, but insertion and removal are unavailable.

Geometry editing preserves the selected measurement's own calibration, reference distance, unit and precision. It does not replace them with a page default or the current drawing scale. Use the separate measurement settings editor to change those properties explicitly.

## Data and permissions

The existing annotation keeps its identity, replies, register fields and quantity metadata. Actual measured quantities and compatible unit-price totals are recalculated from the edited geometry; entered fields, price and price basis remain unchanged. Other measurements and page defaults are independent.

Preview and Apply use the existing guarded measurement operation. A changed source, unsupported record, locked annotation or missing annotation permission cannot be committed through this workflow. Invalid geometry, including collapsed or self-intersecting polygons, must be corrected before Apply. Page, tab and document changes invalidate stale drafts and previews.

## Scope

Use **Vertex** to identify a point explicitly when handles overlap. Zoom in when nearby segments make insertion ambiguous. A point is inserted on the selected measurement's segment; this does not snap to underlying PDF lines. Invalid geometry still has to be corrected before Apply.

This edits supported saved measurements one at a time. Vector snapping, batch recalculation and multiple-scale viewports remain separate work. Raster drawings do not acquire vector snapping from this feature. Automated interaction tests do not establish native hardware or accessibility qualification. AEC-002 and AEC-003 remain partial.

The existing `edit-measurement` CLI remains available for explicit geometry replacement in a new PDF copy with an expected record; see the [measurement guide](MEASUREMENTS.md). No new CLI command is introduced by the on-page workflow.
