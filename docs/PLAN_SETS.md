# Local plan sets

A plan set is a local JSON index of explicitly selected PDF files and their sheets. It records each source's
byte length and SHA256, its page count, and editable sheet number, title, discipline and revision fields.
The source PDFs stay separate and unchanged. The index does not merge PDFs or copy their page contents.

The published 0.63.0 release passed exact-source tests and public-package checks. A separate native
development-build run covered manifest opening and sheet navigation; see [release qualification](RELEASE_0.63.0.md).
The 0.64 development candidate adds grouped navigation and a separate
[reviewed annotation transfer](REVISION_TRANSFER.md) workflow. Its final local source checks passed;
native and package qualification remain pending.

## References and sheet metadata

Each sheet has a stable ID, a file ID and a physical page number starting at 1. User-facing numbers and titles
can change without changing that physical page reference. Missing files, changed bytes, unreadable or protected
files, invalid pages and duplicate references remain visible in validation results instead of being silently
treated as valid. Validation is a point-in-time observation; later file changes require another check.

Adding a PDF indexes all its pages. The bounded format accepts at most 64 files and 1,024 indexed sheets;
individual source PDFs are limited to 64 MiB and 10,000 pages, and the JSON manifest to 2 MiB. A failed import
does not publish a partial manifest. The format preserves explicitly selected pages when reading an existing
valid manifest; it does not infer extra references from folders or neighboring files.

Source PDFs must remain inside the manifest's parent directory or its subdirectories. Explicit file selection
resolves to the actual contained file before indexing. Stored manifest references refuse absolute paths,
parent traversal, symbolic links and reparse points. The output directory must already exist.
Manifest copies rebase paths when every source remains inside the new destination's parent directory; otherwise
the operation is refused. Keeping relative PDFs and the manifest together allows a folder to be relocated.
Exporting JSON alone does not package the PDFs: retain the referenced files or explicitly relink them.
A manifest contains paths and sheet metadata, but never a stored PDF password.

## Desktop workflow

Open **Tools → Organize pages → Plan sets**, or find **Plan sets** in the command palette. **New set** first
selects the root folder; **Open manifest** loads an existing index. Use **Add PDFs**, select a sheet and edit
**Sheet number**, **Sheet title**, **Discipline** or **Revision**, then choose **Apply metadata**. **Reset edits**
discards unapplied field edits, while **Remove sheet from set** removes only its indexed entry.

The development view filters sheet number, title, discipline, revision, filename and physical page by
all entered search terms. Group by discipline, revision or file, and select a column heading to change
the sort direction. Number sorting compares digit runs naturally (A-2 before A-10); revision labels do
not establish revision chronology. **Issues only** retains unresolved or invalid references. The view
shows its filtered count and displays at most 50 rows per page. Filtering or sorting never rewrites the
manifest, changes physical page identities or drops an unapplied metadata edit. An active filter that
hides the selected sheet disables **Open sheet** until the selection is visible again.

**Transfer revision annotations** opens the reviewed revision workflow using the current index. It requires
explicit old/new files, checked references and reviewed page matches. It does not replace source files
or automatically accept a revision. See [supported annotations and limits](REVISION_TRANSFER.md).

Use **Validate references** before **Open sheet**, then **Save new manifest copy**. **Close** retains the draft
in memory. Quitting with unsaved index changes offers a new copy or explicit discard. Existing document tabs
retain their own unsaved edits and history; an already edited PDF can have a different page sequence from the
indexed disk file, which the application reports. The index does not replace that live document automatically.

**Accept updated file** and **Relink selected file** require confirmation of the selected file and fingerprint.
They preserve stored sheet
identities and metadata; they do not determine whether a page in a revised PDF still represents the same drawing.
Review physical page references after changes, especially when pages were inserted, removed or reordered.

For a protected source, enable the optional password field before the operation. Its masked value is used
once and cleared when the request is sent; it is never saved in the manifest. One value applies to that operation's
files. References that need a different password remain explicitly unavailable. Opening a sheet retains the
verified bytes through the credential step, so a file replaced on disk cannot silently become the opened document.

A clean open tab is reused only when its saved identity and page count match the checked source. Unsaved tabs
require an explicit choice; if their page count differs, the indexed jump is unavailable. Opening a separate
verified copy keeps the existing tab and its history intact.

## Command-line workflow

All writing commands require a new output path. Existing outputs are refused even with the global
`--overwrite` flag. Both the input manifest and its source PDFs remain unchanged.

Create an index from named files, then inspect its metadata and references:

```text
sunipdf-cli plan-set create "plans/Ground floor.pdf" "plans/Details.pdf" --name "Building A" --discipline Architecture --revision A --out plans.json
sunipdf-cli plan-set inspect plans.json
sunipdf-cli plan-set validate plans.json
```

`inspect` and `validate` return the manifest and per-file/per-sheet results as JSON. A report with
`complete: false` is still written to stdout and exits with code 2. Invalid JSON or an invalid manifest
structure produces the normal JSON error on stderr. Use IDs from the returned manifest for later commands.

Add a PDF, change one sheet's metadata and remove a sheet from new copies:

```text
sunipdf-cli plan-set add plans.json --pdf "plans/Structure.pdf" --discipline Structure --revision B --out plans-added.json
sunipdf-cli plan-set edit-sheet plans-added.json --id SHEET_ID --number S-101 --title "First floor" --revision C --out plans-edited.json
sunipdf-cli plan-set remove-sheet plans-edited.json --id SHEET_ID --out plans-filtered.json
```

`--pdf` can be repeated. `edit-sheet` accepts `--number`, `--title`, `--discipline` and `--revision`;
at least one must be supplied. An empty value clears the title, discipline or revision; the number must remain nonblank.
Removing the last indexed sheet of a PDF also removes its unused file entry; it never deletes the PDF.

Explicitly accept a changed file, or relink it to another path:

```text
sunipdf-cli plan-set refresh plans.json --file-id FILE_ID --accept-changed --out plans-refreshed.json
sunipdf-cli plan-set refresh plans.json --file-id FILE_ID --source "revisions/Updated plan.pdf" --accept-changed --out plans-relinked.json
sunipdf-cli plan-set validate plans-relinked.json
```

`--accept-changed` is required. Refresh records the explicitly selected source's current identity and page
count. It keeps sheet metadata and physical page numbers; an out-of-range sheet remains a validation problem.
It neither transfers annotations nor performs automatic page matching.

Export an index to another directory with its references rebased:

```text
sunipdf-cli plan-set export plans.json --out plans-copy.json
```

Metadata-only edit, removal and export do not open source PDFs. They can preserve an index whose files are
temporarily unavailable; use `validate` to distinguish structural validity from usable source references.

## Protected PDFs

The CLI uses the common `--password-env NAME` and `--password-stdin` options for creation, inspection,
validation, adding PDFs and refresh. One supplied password applies to the sources in that operation and is
not persisted in the manifest or returned JSON. Prefer those sources over the unsafe `--password` argument.
Process separately when different documents require different credentials.

```text
sunipdf-cli plan-set create protected.pdf --name "Restricted plans" --password-env PLAN_PDF_PASSWORD --out protected-plans.json
sunipdf-cli plan-set validate protected-plans.json --password-env PLAN_PDF_PASSWORD
```

These local reference checks do not establish a drawing's approval, physical measurement accuracy or
revision equivalence. Retain earlier source files and review changed pages explicitly.
