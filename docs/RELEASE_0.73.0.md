# SuniPDF 0.73.0

Candidate under qualification. Stable publication and downloaded-package results are pending.

## Reviewed links across several PDFs

Open **Tools → Review & measure → Plan links**, then choose **Link multiple PDFs…**.
Select source PDFs from an indexed plan set, choose explicit pages and destinations for each source,
inspect the references and confirm the intended destination occurrence by occurrence. Review the
source and destination previews before saving new sibling copies.

The batch accepts up to 16 sources, 128 selected pages and 4,096 detected references within its
document and JSON limits. All output names and reviews are checked before the first copy is saved.
Only one prepared PDF candidate is retained at a time. The manifest and original PDFs are checked
again before every commit and after the batch.

Each file is saved atomically. Cancellation or failure preserves completed copies and their receipts;
remaining files are reported separately. Existing files are never overwritten. Destinations remain
the original indexed PDFs, including when sources link to each other. Generated copies are not
automatically substituted into the plan set.

The CLI adds `plan-links batch-inspect`, `batch-preview` and `batch-apply`. Existing single-source
commands remain available. [Workflow, limits and examples](PLAN_LINKS.md).

## Clearer Linux window controls

Linux uses round minimize, maximize/restore and close controls with 40-pixel logical click targets,
vector symbols, localized accessible labels and a red close hover. Title dragging, double-click
maximizing and edge/corner resizing use native window commands. Closing uses the application's
existing protection for unsaved changes and active operations.

The welcome and About text now correctly includes Windows and Linux. Windows retains its native
window decoration. Ubuntu/X11 interaction tests are part of candidate qualification; Wayland desktop
interaction remains a separate check.

## Qualification

Source tests, downloaded-package corpus results, native Linux evidence and publication identities
will be recorded after they complete. No unexecuted test or complete feature category is claimed.
AEC-017 remains **In progress** until its outstanding native and interoperability criteria are evidenced.
