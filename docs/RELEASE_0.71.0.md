# SuniPDF 0.71.0

Publication: candidate under qualification. The currently published version and its
downloads are listed in [project status](PROJECT_STATUS.md).

## Find symbols and review counts

Open **Tools → Review & measure → Find symbols**, select one symbol on the page,
and search selected pages using a fixed resolution, similarity threshold and
optional quarter-turn rotations. Fit, zoom and pan controls help select the template
on larger drawings. The search advances between foreground work and exposes
cancellation, page coverage and resource limits.

Candidate previews include existing annotations. Matches start unselected; only a
reviewed selection becomes count markers, in one undoable batch. Existing counts
are retained, and repeated searches do not automatically deduplicate them. Editing
the source invalidates the review. Saved counts remain available in the annotation
register and quantity table.

The command-line workflow creates a review file with no accepted candidates. Applying
that review checks the saved source and repeats the inspection before writing a new
PDF. Unknown selections, changed sources and altered results are refused. Accepting
partial results requires an explicit choice and does not change their incomplete status.

See [visual symbol search](VISUAL_SEARCH.md) for the desktop controls, CLI examples,
resource limits and the distinction between proposed matches and accepted counts.

## Scope and qualification

This is same-scale raster matching with exact quarter-turns. Similarity is not a
probability or a recognition guarantee. Noise, closely related symbols, overlap and
changed backgrounds can cause missed or false matches. Automatic scale matching,
arbitrary angles and searching across separate files remain outside this increment.

Local tests and an independent synthetic corpus are being qualified against the
candidate. Downloaded-package qualification and publication have not been completed.
Native desktop interaction is untested in this session. AEC-011 remains **In progress**;
no entire priority category or complete feature parity is claimed.
