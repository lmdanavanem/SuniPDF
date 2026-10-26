# Saved annotation filters

Save frequently used annotation-register criteria as named filters, then apply them to another review without rebuilding each field. Saved filters are local preferences; they do not change a PDF.

## Save and reuse

1. Open **Annotation register**, set its search and field filters, and choose the required sort order.
2. Open **Filters → Saved filters**, enter a unique **Filter name**, then choose **Save current filter**.
3. Select an existing saved filter and choose **Apply** to restore its criteria and sort order.
4. Use **Delete** to remove a saved filter you no longer need.

A saved filter includes search text, author, modification-date text, PDF review state, business status, discipline, annotation type, count group, the replies option and sorting. It excludes page numbers and document-specific zone identities. Applying it keeps the current page and zone scope, returns the table to its first result page and clears the explicit batch-edit selection and preview. It never selects annotations automatically.

Filter matching is unchanged: search is case-insensitive, named fields use their existing exact-text matching, and dates remain PDF date text. Saving a filter does not reinterpret its criteria or change custom text sorting into numeric sorting.

## Import and export

Use **Export…** to write the saved-filter library to a new JSON file. Existing files are refused. **Import…** validates the complete file before adding any entries; a duplicate name, unsupported version or invalid entry leaves the current library unchanged.

The [0.91.0 release](RELEASE_0.91.0.md) adds an optional exact custom-text-field criterion. Libraries that contain this criterion use version 2. Version 1 libraries remain supported and retain their previous representation when the criterion is absent. Older applications may refuse version 2; import it only into a supporting version.

The portable file stores criteria and sort order, not PDFs, annotation selections or document identities. Review search and author text before sharing it. Imported filters do not apply themselves to the open document.

The library supports at most 16 filters and 128 KiB of serialized JSON. Names contain 1–64 UTF-8 bytes. Name collisions use Unicode lowercase comparison without Unicode normalization. Oversized fields, control characters, unknown fields and duplicate JSON keys are refused.

Settings without saved filters keep their existing behavior. An invalid saved-filter preference does not reset unrelated preferences; an import or save error is reported without replacing valid data.

Published qualification: [SuniPDF 0.91.0](RELEASE_0.91.0.md). Native operating-system file dialogs and physical accessibility retain separate qualification scopes.

[Annotation register](MARKUP_REGISTER.md) · [Bulk field editing](REGISTER_BATCH.md)
