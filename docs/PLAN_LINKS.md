# Reviewed links between plan sheets

This workflow finds exact indexed sheet numbers in searchable PDF text and creates standard links
to explicitly selected local sheets. [Release qualification](RELEASE_0.67.0.md) records the tested scope.

## Sources and destinations

Create a [local plan set](PLAN_SETS.md) and verify its references first. Select one indexed source
PDF, explicit physical pages and the destination sheets that may be linked. Sheet identities and
stored fingerprints determine destinations; a similarly named file is not a substitute.

The scan uses the source PDF's existing text layer. OCR text is usable when already present, but
the scan does not run OCR implicitly. Images without searchable text remain unresolved. Exact
identifier boundaries prevent a reference to `A-10` from matching `A-101` or `XA-10`.

The report retains occurrence locations, candidate destinations, ambiguous matches and refused
records. A matching number can refer to multiple sheets or revisions. Choose the intended target
explicitly; revision labels do not establish which drawing is approved or current.

## Review and publication

Inspect the source occurrence and a real rendering of the destination sheet. Confirm the target
for each selected occurrence. The prepared candidate contains only those confirmed links; the
report keeps the unselected occurrences visible. A complete selected subset does not mean that
every reference in the document was linked.

Review the source candidate as well as its destinations. A source or manifest change invalidates
the preparation. Publication rechecks the original inputs and writes a new PDF next to the source
PDF. Keeping the same directory preserves the meaning of existing relative links. Existing files,
source aliases and unsafe paths cannot be overwritten, including with the CLI's `--overwrite` flag.

The saved links use PDF `GoToR` actions and relative PDF paths. SuniPDF retains the destination's
sheet identity, page, byte length and SHA256 in bounded metadata. Its reader requires a selected,
verified plan-set manifest and explicit confirmation before navigating to that sheet. It does not
send local paths to a web browser or launch a shell. Missing or changed destinations remain
unresolved, even if the plan manifest was refreshed after creating the link.

Other PDF readers may interpret standard remote links according to their own policies. Their
ability to navigate a link does not establish that they verified SuniPDF's stored destination hash.
Moving the complete plan folder can preserve relative links; moving only the linked PDF can break
them. Reinspect before creating replacement links.

## Preview a saved link's destination

The [0.77 release](RELEASE_0.77.0.md) adds a preview to the reader's
saved-link confirmation. Click a plan link, enter the absolute path of its JSON
manifest in **Manifest path**, and choose **Load manifest**. The file browser remains
available. Editing the path clears the previous manifest selection and preview;
typing a path alone does not load it.

Choose **Preview destination** to render the verified destination page and inspect
its sheet number, title, revision and physical page. Previewing does not open a tab,
modify a PDF or approve navigation. **Open verified destination** performs fresh
checks before opening the sheet. If a source or destination changes after a preview,
review the error and preview again; a cached image does not authorize changed files.

Protected sources and destinations can require the password again for each operation.
Passwords are used once and are not retained in the manifest or preview receipt.
Closing the confirmation keeps the source PDF open. Existing dirty-tab choices are
preserved when opening a destination that is already being edited.

## Command line

Use file and sheet IDs from `plan-set inspect`, then select each permitted destination explicitly:

```text
sunipdf-cli plan-links inspect set.json --source-file FILE_ID --pages 1,3 --target-sheet SHEET_ID
```

Repeat `--target-sheet` for additional destinations. Physical page numbers start at 1. Inspection
prints JSON and returns exit code 2 for incomplete results; retain that report for manual review.
Errors do not create a destination PDF.

Build a version-1 review using the reported manifest/source hashes and occurrence fingerprints:

```json
{
  "version": 1,
  "manifest_sha256": "MANIFEST_SHA256",
  "source_sha256": "SOURCE_SHA256",
  "scan": {
    "source_file_id": "FILE_ID",
    "pages": [1, 3],
    "target_sheet_ids": ["SHEET_ID"]
  },
  "selections": [{
    "occurrence_id": "OCCURRENCE_ID",
    "fingerprint": "OCCURRENCE_SHA256",
    "target_sheet_id": "SHEET_ID",
    "confirmed": true
  }]
}
```

Render the source candidate and an explicitly selected destination to separate new PNGs:

```text
sunipdf-cli plan-links preview set.json --review links.json --preview-page 1 --preview-out source-preview.png
sunipdf-cli plan-links preview set.json --review links.json --preview-target-sheet SHEET_ID --preview-out target-preview.png
```

`--preview-long-side` accepts 64 through 1,536 pixels. Source previews default to the first page
receiving a link. Target previews require a destination selected in the review. Neither preview
creates or modifies a PDF. The CLI cannot attest that a person reviewed the images.

After review, rebuild and publish the candidate as a new sibling PDF:

```text
sunipdf-cli plan-links apply set.json --review links.json --out plans/index-linked.pdf
```

The example assumes the indexed source PDF is in `plans/`. A different output directory is refused.
The result includes the new path, output hash, created links and excluded occurrences. Open the new
copy and verify its navigation before distributing it.

For protected PDFs, repeat `--file-password-env FILE_ID=ENV_NAME` for each applicable indexed file.
Pass only the environment variable's name, never its secret value. The command clears its transient
credential copies after the operation; it does not remove the caller's environment variables.
Passwords are not stored in the manifest, review or PDF link. Global
`--password-env` and `--password-stdin` are not used by this multi-file command.

## Bounds and refusals

- One source PDF, at most 32 explicitly selected physical pages and 512 detected occurrences.
- Existing plan-set limits apply, including 64 MiB per indexed PDF and confinement to its root.
- At most 250,000 extracted characters per page; truncation remains visible in the report.
- JSON review at most 256 KiB, report at most 2 MiB and candidate PDF at most 160 MiB.
- Source text extraction and annotation permissions are required. Signed sources are refused.
- Destination copy permissions are required for inspection and previews; encryption is preserved.
- Unsupported text geometry, unsafe references, missing files and changed fingerprints are refused.
- Ambiguous targets need an explicit reviewed choice. No automatic approval or revision ordering.
- Existing annotations, page content and links remain preserved. The operation does not flatten
  the document, remove measurements or regenerate an OCR layer.

Native navigation and independent-reader interoperability have separate qualification scopes.
See [published navigation evidence](RELEASE_0.77.0.md) for the exact tested paths and remaining gaps.

## Multiple source PDFs

Batch processing uses the same indexed plan set and explicit reviews as the single-source workflow.
Choose the pages and permitted destination sheets separately for each source. Occurrence IDs belong
to their source PDF: the same ID in another PDF is a different occurrence. A source without an accepted
occurrence is excluded from publication.

Inspect a strict request with a `sources` array containing the scan objects shown above:

```text
sunipdf-cli plan-links batch-inspect set.json --request sources.json
```

The batch review contains `version: 1`, the inspected `manifest_sha256`, and a `files` array. Each
entry has a `review` containing the complete single-source review and an `output_name` such as
`floor-linked.pdf`. The name must be a new PDF basename; each copy stays beside its original source.
Do not use a path, an existing filename or the name of another output in the same folder.

```text
sunipdf-cli plan-links batch-preview set.json --review batch.json --source-file FILE_ID --preview-out preview.png
sunipdf-cli plan-links batch-preview set.json --review batch.json --source-file FILE_ID --preview-target-sheet SHEET_ID --preview-out target.png
sunipdf-cli plan-links batch-apply set.json --review batch.json
```

Preparation validates every output and prepares each source sequentially before publication starts.
Only one candidate PDF is retained at a time. Preview and publication prepare candidates separately;
an earlier preview does not establish byte identity with a later output. Publication checks the original
manifest, sources and destinations again before each copy is committed and after the batch.

Atomic publication applies to each file separately. If cancellation, an input change or another error
stops publication, the completed files remain saved. The JSON outcome retains their paths, hashes and
created links, plus the failed or remaining files. An incomplete outcome returns exit code 2. Review
these receipts before explicitly preparing a new batch for remaining sources. Existing outputs cannot
be overwritten, even with `--overwrite`.

Links always point to the original indexed destination PDFs. Generated copies are not automatically
added to the plan set and do not replace targets in subsequent files. Moving or replacing originals
requires another review.

The batch accepts up to 16 distinct sources, 128 selected pages and 4,096 occurrences. Per-source
limits still apply. Batch request/review JSON is bounded to 4 MiB and aggregate inspection/report data
to 8 MiB. Truncated scans and unresolved occurrences remain visible; explicit review can resolve an
ambiguous destination, but it cannot turn an uninspected reference into a confirmed link.
