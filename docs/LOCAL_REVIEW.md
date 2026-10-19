# Local PDF comparison and multi-document search

[Français](LOCAL_REVIEW.fr.md)

Implemented for 0.31.0. Executed release qualification is recorded in [Project status](PROJECT_STATUS.md).

## Search selected files

Choose **Edit → Search in PDF files…**, select PDF files explicitly, enter literal text and run the analysis.
Case-sensitive matching is optional. Case-insensitive matching uses Unicode lowercase conversion; it is not
linguistic stemming, accent folding or regular expressions. Each matching page has an occurrence count,
a bounded excerpt preserving the original case, and a button opening its physical page in a document tab.

Only saved files are inspected. Save edits before running the analysis. Results refer to the file bytes read
during that run; opening a result opens the current saved file. Rerun after changing a source.
There is no folder traversal, persistent index, automatic OCR, telemetry or upload. Text follows PDFium's
extraction order; untagged columns, image-only pages and hidden content do not gain semantic interpretation.
Image-only documents need the existing explicit OCR operation first.

The report distinguishes completed files with no matches from cancelled, limited, missing, malformed,
password-protected and copy-restricted files. A failed file does not prevent later files from being searched.
Cancellation retains completed page results and marks unfinished files explicitly. Unknown page counts
remain unknown, rather than zero. Search results remain in process memory until replaced or the app exits.

## Compare two versions

Choose **File → Compare PDF files…**, select the original and revised saved PDFs, and choose their first
physical pages. Pairs align sequentially from those offsets. Adjust the offsets when a page was inserted;
automatic page matching, moved-text detection and a semantic word diff are outside this operation.

The summary compares extracted page text, displayed page dimensions/presence, and rendered previews.
Select a changed pair to see original/revised previews and red pixels marking observed differences.
The same scale is used for both pages, with the longest side bounded to 768 pixels; pages align at the
top-left, and RGB channel differences of at most 16/255 are ignored. Missing page areas are white.
Changes in page presence and geometry remain reported even if the preview is blank.

No observed difference **does not prove identical PDFs**. Small/faint differences can be below the preview
resolution/tolerance. This does not compare PDF object graphs, metadata, attachments, scripts, hidden data,
signatures, accessibility or archival conformance. No OCR is performed. The JSON report explicitly identifies
pairs with no extracted text on either page. Both files must allow copying under the existing permission policy.
The desktop does not collect passwords for these jobs; protected-file failures remain explicit. CLI callers
may supply one credential for all selected inputs through the existing environment/stdin password sources.

Each comparison records SHA256 of the exact input bytes. Opening a preview rereads the files, checks both
digests and refuses stale sources before rendering. It never silently displays a newer revision as the old result.
These digests identify the analysed bytes; they are not independent publication signatures.

## Bounds and cancellation

- At most 64 explicitly selected search files, each at most 64 MiB; comparisons use two files with the same size bound.
- At most 250,000 PDFium characters per page before full text extraction.
- Search scans at most 10,000 pages and records at most 2,000 matching pages per run.
- Comparison processes 1–500 aligned pairs per run, default 200. Remaining pairs make the report incomplete.
- One engine thread processes files/pages sequentially. Only the requested preview pair is retained as textures.
- Cancellation is checked between native page operations; an active PDFium call is not forcibly interrupted.
  These bounds do not establish a native-process memory or CPU-time ceiling for hostile PDFs.

## Command-line interface

```powershell
sunipdf-cli search-files original.pdf revised.pdf --query "contract" --match-case
sunipdf-cli compare original.pdf revised.pdf --left-start 2 --right-start 3 --max-pages 200
sunipdf-cli search-files protected.pdf --query "contract" --password-env SUNIPDF_PASSWORD
```

Both commands print structured JSON without modifying sources. Page numbers are physical and one-based.
The common password applies to all selected inputs. No password is included in the report.
Exit 0 means the requested bounded analysis completed, including a completed search with no matches or
a completed comparison with differences. Exit 2 accompanies a full JSON report when analysis is partial.
Fatal comparison/open/argument errors retain the existing CLI error JSON and exit-code conventions.
Use `complete`, per-file status/errors and scanned-page counts; never interpret unanalysed files as clean.

## Qualification

Regression tests exercise changed/unchanged text and pixels, added pages, manual offsets, size changes,
blank pages, oversized inputs/text, malformed/missing files, AES-128/256 and copy permissions,
cancellation, original-byte preservation, stale-preview refusal, JSON partial-result exits and non-argv passwords.
Desktop tests cover command routing, cancellation on quit, and retry state. These checks do not establish
manual screen-reader, ARM64 hardware or universal third-party PDF interoperability.
