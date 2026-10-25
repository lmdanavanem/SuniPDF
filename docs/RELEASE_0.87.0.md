# SuniPDF 0.87.0 — candidate

This candidate adds named reusable filters to the annotation register. Publication and package qualification are pending.

Save the current review criteria and sort order, explicitly apply a saved filter, or exchange the local library through bounded JSON import/export. Existing files and duplicate names are not overwritten. Imports validate completely before changing the library.

Applying a saved filter preserves the current page and zone scope and clears any selected bulk-edit rows and pending preview. It does not change a PDF or automatically select annotations. Older settings retain their current behavior.

Core tests cover actual filtering and sorting after serialization, scope preservation and atomic refusal. Desktop tests cover small-window controls, settings persistence and error handling. Runtime results have not yet been recorded for this candidate.

[Saved-filter guide](REGISTER_FILTERS.md) · [Annotation register](MARKUP_REGISTER.md) · [Project status](PROJECT_STATUS.md)
