# SuniPDF 0.76.0

Development candidate. Published downloads remain on the version identified in
[Project status](PROJECT_STATUS.md) until package qualification and publication finish.

## Independent reader views

Split the current PDF into two reader panes. Each pane keeps its own page, zoom,
scroll position and reading orientation. Activate a pane to use the page controls,
search results and editing tools there. Both panes share the same document and
Undo/Redo history; splitting does not create a second PDF copy.

Optional synchronization follows the active view. Finish or cancel unfinished
editing input before switching views. Closing the split retains the active view
and leaves the document open.

Rendering shares a bounded tile cache and combines demand from both panes. Tab
changes preserve the reader workspace; document changes invalidate affected views.

See [reader controls](READER_WORKSPACE.md) for the workflow. This increment covers
two views of one PDF; simultaneous editing of different PDFs in a split workspace
is outside this scope.

## Qualification

Qualification is pending. The release must pass source checks, two-pane rendering
and interaction regressions, Windows upgrades and the existing Ubuntu and Apple
Silicon package checks before publication. Physical-device and human accessibility
coverage retain their separate limits; earlier release results do not qualify
this candidate.
