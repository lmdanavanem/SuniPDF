# Page object ordering

[Français](OBJECT_ORDER.fr.md)

Published in 0.32.0; release qualification is recorded in [Project status](PROJECT_STATUS.md).

## Desktop

Open the Objects panel with F3. Rows at the top are in front. Drag a row above or below another row; the blue insertion line shows the destination. Dragging a selected row moves the entire selection as one block, keeping its original relative drawing order. Dragging an unselected row moves that object only.

Escape cancels. Dropping outside a valid row, onto a selected row or at the same position does nothing. A drop into another page/document or after an intervening edit is refused. Selection follows the moved objects. Existing forward/backward buttons and keyboard commands remain available; one completed drag uses one Undo step and supports Redo through the existing worker transaction.

## CLI

List indices with `sunipdf-cli objects input.pdf --page 1`, then use:

```powershell
sunipdf-cli reorder-objects input.pdf --page 1 --objects "3,1" --at 0 --out output.pdf
```

Pages are one-based. Object indices and `--at` are zero-based drawing indices, with zero at the back. `--at` is the final index of the first moved object after removing the selected objects. The selected indices are sorted and deduplicated; original relative order is retained. JSON returns `new_indices` in that order. The largest destination is the object count minus the distinct selection count.

The existing output/overwrite policy, atomic save and environment/stdin password sources apply. Invalid indices, destinations or pages do not publish an output. Reordering checks the document's effective modify permission before changing content, including existing front/back operations.

## Scope

This changes top-level page-content drawing order, including text, paths, images and whole Form XObjects. Annotation/widget overlays and objects inside a Form are separate. Existing geometry, styles and content stay attached to each object; overlapping content can become covered or uncovered as intended. Source files remain unchanged when saving a separate copy. Existing encryption and permissions remain preserved. As with other content edits, existing digital signatures can be invalidated; save a separate copy. A general Layers/OCG editor, native accessibility and hardware/manual qualification remain separate work.
