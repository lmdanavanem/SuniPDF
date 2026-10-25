# Reviewed annotation transfer

The published workflow transfers selected supported native shapes from one saved PDF revision to
another. It prepares a separate candidate, retains the revised PDF's existing contents and writes a
new PDF only after review. Source files remain unchanged. The 0.64 source, native development-build
workflow and downloaded-package checks passed within the [recorded qualification](RELEASE_0.64.0.md).

## Choose sources and pages

Open **Tools → Organize pages → Transfer revision annotations**, or find the command in the command palette.
Choose the saved old and new PDFs. Unsaved tabs for either source must be saved or resolved first;
the operation cannot substitute live edits for the reviewed disk files.

Enter explicit old/new physical page pairs, starting at 1. Each page may occur once, with at most
32 pairs. Confirm every pair. Sheet labels, page numbers and revision names do not establish that
two drawings represent the same physical sheet.

From **Plan sets**, choose **Transfer revision annotations**, then the old and new indexed files.
Choose **Suggest matches by discipline and sheet number** to propose only unique matches. Inspection
checks the manifest references. Ambiguous and unmatched sheets from both files remain visible with
their number, title, discipline, ID, physical page and reason for exclusion. They are not assigned
automatically. **Use in selected page pair** assigns only that sheet's side of the selected pair;
choose and verify its counterpart explicitly. Every proposed or manual match still needs confirmation;
revision labels are not sorted as an approval or revision chronology.

For protected sources, enter the old and new PDF passwords before each inspection or initial preview.
When previewing another candidate page, enter the new PDF password again. Each password is used for
one operation only and is not stored in the manifest, recipe or transfer history.

## Select supported annotations

Inspect the selected pages before choosing annotations. Supported original native shapes are
rectangles, ellipses, lines, arrows, polygons, polylines and revision clouds with reproducible styles.
The list identifies each source annotation and reports why other records cannot be transferred.
Annotations carrying measurements, prices, custom appearances, review threads or unsupported
semantics are excluded. Do not use transfer to strip those data from a record.

The transfer retains supported geometry, color, opacity, stroke width, dash patterns and author.
Transferred annotations receive new identities and creation/modification dates. The destination's
existing annotations and form fields are preserved. Transfer does not flatten the revised page.

## Alignment and candidate review

Use identity alignment only when the page coordinate systems already match. For manual alignment,
enter two points on the old page and the corresponding two points on the new page, in displayed
top-left page coordinates. Their order determines direction. The supported transform uses uniform
scale and quarter-turn rotation; arbitrary rotation, shear and different X/Y scales are refused.
Page rotation and crop coordinates are taken into account. Transformed geometry must remain inside
the destination page.

Choose **Preview page** and inspect the real rendered candidate for every mapped destination page.
The desktop preview uses a longest side of 1,024 pixels and retains one displayed page at a time.
Changing sources, pairings, selections or alignment invalidates the previous candidate. Closing or
cancelling inspection or preview discards it. Cancelling a save cannot remove a copy that has already
been committed; the app still reports that successful save. Saving rechecks the saved source identities; changed inputs require
inspection and preparation again. The UI enables saving only after every mapped page was reviewed.

Choose **Save new copy…** and a new output filename. Existing files, either source path and aliases of either source are
refused. Open the new PDF and check the saved annotations before using it for a review workflow.
A bounded history records source fingerprints and transferred counts. Repeating a source transfer
requires acknowledgement; this is a duplicate-warning mechanism, not geometric deduplication.

## Queue several reviewed transfers

In the 0.79 development workflow, **Add reviewed transfer to queue** retains the
exact prepared copy after every mapped page has been previewed and reviewed.
Choose a new output filename, then inspect another old/new PDF pair. Editing the
current pair does not remove previously queued transfers.

The queue holds at most 16 file pairs and 160 MiB of prepared PDF data in total.
Remove individual entries or clear the queue explicitly. Closing with queued or
in-flight work asks whether to discard it or keep reviewing. No source PDF is
overwritten, and this operation does not replace the revised PDF's page content.

**Save queued copies** rechecks every source and destination before the first
write and rechecks sources at publication. Outputs cannot overlap another
output or any batch source. Each new file is published atomically, but the whole
batch is not a filesystem transaction: if a later item fails or saving is
cancelled, completed copies remain on disk. The result lists saved and unsaved
items explicitly. An unsaved item needs inspection and review before retrying.

Queued copies contain the prepared document data only for this session. They
are not a persistent recovery queue, and passwords are not retained in reports.

## Command line

Inspect explicit page pairs and use the reported source SHA256 values and annotation fingerprints:

```text
sunipdf-cli revision-transfer inspect old.pdf revised.pdf --pages 1:1,2:3
```

Prepare a version-1 JSON recipe with confirmed pairs and selected zero-based annotation indexes.
For example, replace the placeholder hashes/fingerprint with values from that inspection:

```json
{
  "version": 1,
  "old_sha256": "OLD_SOURCE_SHA256",
  "new_sha256": "NEW_SOURCE_SHA256",
  "pairs": [{
    "old_page": 1,
    "new_page": 1,
    "old_sheet_id": null,
    "new_sheet_id": null,
    "confirmed": true,
    "alignment": { "mode": "identity" },
    "selections": [{ "index": 0, "fingerprint": "ANNOTATION_SHA256" }]
  }],
  "acknowledge_repeated_transfer": false
}
```

Render a real candidate PNG, review it and apply the recipe to a new PDF:

```text
sunipdf-cli revision-transfer preview old.pdf revised.pdf --recipe transfer.json --preview-out preview.png --preview-page 1
sunipdf-cli revision-transfer apply old.pdf revised.pdf --recipe transfer.json --out reviewed-copy.pdf
```

CLI application independently rebuilds and verifies the candidate. A saved preview report cannot
authorize publication, and the CLI cannot attest that a person reviewed a PNG. Existing outputs
remain refused even with the global `--overwrite` option. `--password-env` or `--password-stdin`
supplies the old PDF password; `--new-password-env` or `--new-password-stdin` supplies the revised
PDF password. Only one input may read stdin per invocation. Passwords are not recipe fields.

### Multi-file CLI batches

Use a version-1 manifest of explicit old/new PDF pairs, existing reviewed recipes
and new output names. Relative paths resolve from the manifest's directory.

```json
{
  "version": 1,
  "items": [
    {
      "id": "floor-1",
      "old": "floor-1-old.pdf",
      "new": "floor-1-revised.pdf",
      "recipe": "floor-1-review.json",
      "out": "floor-1-reviewed.pdf"
    },
    {
      "id": "floor-2",
      "old": "floor-2-old.pdf",
      "new": "floor-2-revised.pdf",
      "recipe": "floor-2-review.json",
      "out": "floor-2-reviewed.pdf"
    }
  ]
}
```

```text
sunipdf-cli revision-transfer batch-preview --manifest transfers.json
sunipdf-cli revision-transfer batch-apply --manifest transfers.json
```

Batch preview prepares the candidates and checks sources/outputs without writing
PDFs. Use the existing single-pair preview command to inspect rendered pages;
the batch summary does not attest to visual review. Apply prepares every recipe
again before publishing. Its JSON result includes `complete`, `published` and an
`outcome` with committed receipts, a failure reason and uncommitted entries. An
incomplete result has a nonzero exit code even when some copies were committed.

The manifest is limited to 512 KiB and 16 entries. IDs must be unique, start with
an ASCII letter/digit and use at most 64 letters, digits, dots, underscores or
hyphens. Existing directories and new portable `.pdf` names are required;
remote paths, parent traversal in output paths and existing outputs are refused.
The global `--overwrite` option does not change these rules.

For protected PDFs, an entry may specify `old_password_env` and/or
`new_password_env`, naming environment variables with the respective passwords.
Values are read only for preparation and cleared afterward. Password values,
shell commands and unknown fields are not accepted in the manifest. Global
password flags are not used by batch commands because each entry has its own
explicit credential references.

## Bounds and refusals

- Two regular saved PDF files, at most 64 MiB each; identical saved revisions are refused for transfer.
- At most 32 page pairs, 16 selected annotations per page and 256 selected annotations in total.
- JSON recipe at most 256 KiB; candidate PDF at most 160 MiB.
- Preview longest side from 64 to 1,536 pixels in the core/CLI; the desktop uses 1,024 pixels.
- Uniform scale from 0.01 to 100 and quarter-turn rotation only.
- Source copying and destination copying/annotation permissions must allow the operation.
- A digitally signed destination is refused; compatible destination encryption is retained.

This workflow does not replace pages, infer drawing equivalence, migrate arbitrary annotations,
transfer review threads or establish independent-reader compatibility. Keep prior revisions and
qualify representative project documents separately.
