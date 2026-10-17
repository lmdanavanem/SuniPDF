# Bookmarks and page structure

The bookmarks panel supports adding top-level or child bookmarks, renaming, setting a page destination, reordering siblings and deleting a selected subtree. Edits use the existing Undo/Redo history. The core API additionally supports moving an item to a different parent. Limits are 4,096 bookmarks, 32 levels and 1,024 UTF-8 bytes per title. Cycles, invalid indices, excessive nesting and malformed outline links are refused before applying changes. Renaming and moving preserve the original action, destination coordinates, style and metadata. Setting a destination explicitly replaces that item's action with a local page destination. Existing unsupported actions are retained as PDF data and are never executed by the editor.

Merge, extraction and insertion preserve authored page labels and now rebuild outlines, local destinations, annotations, selected AcroForm fields, layer references, named attachments and associated files from the selected source graphs. Local XYZ/Fit destinations retain their original axes and null values. Destinations to omitted pages are removed; unselected page dictionaries and page-tree nodes are unavailable to the copy traversal, preventing hidden copies of omitted pages. A destination to a repeated source page resolves to its first output occurrence. Non-widget annotations on repeated pages receive separate owners; repeating a page containing form widgets is explicitly refused.

Original target names are retained. Incoming collisions receive a deterministic `S2:`-style prefix, and internal local destinations are converted to mapped page arrays so a duplicate name cannot redirect them to another source. Form field references remain linked to their copied widgets. Conflicting font resource names receive aliases and field default appearances are rewritten to use those aliases. Forms with colliding names and scripts or calculation dependencies are refused because arbitrary script text cannot be renamed reliably. XFA, unsupported conflicting non-font form resources and complex optional-content configurations are also refused rather than silently damaged. Basic optional-content group ordering and on/off state are preserved with matching page-resource references.

This is a bounded preservation operation, not a claim that every PDF extension or active workflow is supported. Name trees, outlines, page inheritance and field trees have explicit count/depth/cycle checks. Missing references and graphs that cannot be represented safely cause atomic failure. Original files remain unchanged. Extraction preserves the original encryption state and permissions; encrypted sources without copy permission cannot be imported. Insertion operates on a candidate and commits only after preservation succeeds. Existing output and overwrite rules of the page-operation API are unchanged.

Tests independently inspect output page destinations, form values and owners, layer/resource object identities, attachment stream bytes and absence of omitted page content. They cover AES-128/AES-256 copies, malformed outline cycles, duplicate widgets, copy restrictions, unchanged source files and snapshot restoration. A worker test checks bookmark Undo/Redo and failure history using the existing single engine. Runtime results belong in the release QA evidence; source tests alone are not a completed release gate.

## CLI authoring

`bookmarks INPUT.pdf` remains the read command. Authoring commands write new copies and refuse existing output files or replacement of the input, including with `--overwrite`. Indices use the current preorder, starting at 1. Omitting `--parent` selects the root; move positions are sibling slots after removing the moved item.

```text
bookmark-add INPUT.pdf --title "Chapter" --page 2 [--parent 1] --out NEW.pdf
bookmark-rename INPUT.pdf --index 1 --title "Renamed" --out NEW.pdf
bookmark-destination INPUT.pdf --index 1 --page 2 --out NEW.pdf
bookmark-move INPUT.pdf --index 2 [--parent 1] --position 1 --out NEW.pdf
bookmark-delete INPUT.pdf --index 1 --out NEW.pdf
```

All authoring commands accept `--password` for encrypted input and preserve its encryption in the output. Modify permission is required. [French notice](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md).
