# Reviewed revision outlines

See [release qualification](RELEASE_0.67.0.md) for executed checks and remaining limits.

Revision outlines turn explicitly reviewed raster differences into ordinary PDF cloud annotations
in a new copy of the revised PDF. A detected region is a proposal, not proof of a design change.
Text antialiasing, scan noise, changed fonts, different rendering and imperfect alignment can all
produce candidates. Inspect the source pages and classify each selected region before publishing.

## Sources and alignment

Choose one physical page in each of two saved local PDFs. The complete revised PDF is retained in
the output; only the selected revised page receives new annotations. The original files remain intact.
Unsaved tab edits are not inputs to a saved-file operation.

The old page is mapped onto the new page. Alignment uses either an explicit translation, clockwise
rotation and uniform scale, or two matching point pairs. Alignment coordinates use displayed-page
points with the origin at the **top left**, after crop and rotation. Result annotation rectangles
use displayed-page points with the origin at the **bottom left**. These coordinate systems are
reported explicitly and must not be interchanged.

The comparison raster is bounded to 1,536 pixels on its longest side. Channel tolerance, minimum
component size and component merge gap affect the proposals. Filtered pixels, unmapped new-page
pixels, old-page pixels falling outside the new canvas, omitted regions and truncation remain in
the report. Any of those uncovered or omitted pixels makes `complete` false. A partial comparison is not a complete
comparison, and an empty result at a selected tolerance does not prove identical document semantics.

## Desktop workflow

1. Choose **Revision outlines** in **File**, **Tools → Organize pages**, or the command palette.
2. Select the old and new saved PDFs and their physical page numbers. Acknowledge the saved-file
   scope. Enter each protected file's password separately when needed.
3. Open **Alignment and detection settings** to choose a manual transform or two corresponding
   point pairs, raster size and detection thresholds. Choose **Inspect differences**.
4. Examine the old page, new page and **Difference map**. Check uncovered and filtered pixels.
   Rendering includes existing annotations and forms, which can also create differences.
5. For each reviewed proposal, enable **Include decision**, choose its classification and enable
   **Confirm classification**. Unselected proposals remain explicitly unreviewed.
6. Choose **Preview copy**, inspect **Annotated copy**, and acknowledge that actual page before
   choosing **Save copy…**. Changing inputs or decisions invalidates the prepared copy.
7. Open the saved PDF and use the annotation register to locate the new clouds and their review
   fields. Cancelling before publication leaves both saved sources unchanged.

Progress and cancellation cover bounded processing stages. A currently running PDF engine call
finishes before cancellation is observed. A completed atomic save remains successful if a later
cancellation arrives. An unsaved source open in another tab blocks preparation or publication
instead of silently including its uncommitted edits.

## Review and output

For each selected region choose a classification and explicitly confirm it:

- **Content change** creates a cloud annotation with that classification.
- **Rendering noise** records the decision without creating an annotation.
- **Uncertain** creates an explicitly uncertain cloud for further review.

Unselected proposals remain excluded in the report. New annotation identities, both source hashes,
comparison settings and classification preserve the review's origin. A cloud can be selected in the
annotation register and edited within the existing cloud tools' supported operations; this feature
does not add arbitrary vertex editing to every imported cloud.

Prepare a candidate and inspect the actual revised-page rendering before saving. Publishing checks
the two source files again and writes a new PDF atomically. A changed source, stale selection,
unsupported geometry, output collision or cancelled operation cannot replace either source.
A review containing only noise decisions can produce a report and preview, but cannot publish an
empty PDF edit. Keep that report if a record of the rejected proposals is needed.

## CLI

Inspect two explicit pages, with identity alignment by default:

```text
sunipdf-cli revision-outlines inspect old.pdf new.pdf --old-page 1 --new-page 1 --long-side 1536 --channel-tolerance 16 --min-component-pixels 4 --merge-gap-pixels 2
```

The JSON includes `complete` and an `inspection` object. Retain its exact hashes, options, region IDs
and fingerprints when building a review. Incomplete inspections return exit code 2 with their report.
Do not infer human approval from a generated list.

```json
{
  "version": 1,
  "old_sha256": "COPY_THE_64_HEX_DIGIT_OLD_HASH",
  "new_sha256": "COPY_THE_64_HEX_DIGIT_NEW_HASH",
  "options": {
    "old_page": 1,
    "new_page": 1,
    "alignment": {
      "mode": "manual",
      "transform": {"translation": [0, 0], "rotation_degrees": 0, "scale": 1}
    },
    "long_side": 1536,
    "channel_tolerance": 16,
    "min_component_pixels": 4,
    "merge_gap_pixels": 2
  },
  "selections": [
    {
      "id": "COPY_THE_SELECTED_REGION_ID",
      "fingerprint": "COPY_THE_64_HEX_DIGIT_REGION_FINGERPRINT",
      "classification": "content_change",
      "confirmed": true
    }
  ]
}
```

The placeholders above are explanatory and must be replaced with actual inspection values.
Supported classification strings are `content_change`, `rendering_noise` and `uncertain`.

```text
sunipdf-cli revision-outlines preview old.pdf new.pdf --review review.json --preview-out candidate.png
sunipdf-cli revision-outlines apply old.pdf new.pdf --review review.json --out reviewed.pdf
```

Preview without `--preview-out` returns the review report without creating a file. A requested PNG
renders the actual candidate page; `--preview-long-side` accepts 64 through 1,536 pixels. Preview does
not create a PDF and cannot attest that a person examined the image. Apply rebuilds and revalidates
the candidate, returns the new output's identity and retains excluded/noise decisions in its report.
Existing files are refused even with the global `--overwrite` option.
Preview and apply also return exit code 2 when `complete` is false. An apply response can therefore
report `published: true` with exit code 2: the new copy was saved, but comparison coverage or the
review remains partial. Inspect the JSON receipt instead of treating that case as a failed write.

Manual alignment uses `--translate "x,y"`, `--rotation DEGREES` and `--scale FACTOR`, always old to new.
Alternatively, provide both `--old-points "x,y x,y"` and `--new-points "x,y x,y"` in corresponding order.
The two methods cannot be mixed. Scales must be from 0.01 to 100 and all coordinates finite.

For protected PDFs, use global `--password-env NAME` or `--password-stdin` for the old file, and
`--new-password-env NAME` or `--new-password-stdin` for the new file. Only one file can read its secret
from stdin in one call. Secret values are not command-line arguments, review data or annotation
metadata; operation-owned copies are cleared after use. The caller remains responsible for its
environment variables.

## Bounds and qualification

The first increment compares one explicitly selected old/new page pair per operation, proposes at
most 512 regions and accepts at most 64 selected regions. It uses existing PDF source and candidate
size bounds, rejects signed/restricted write targets, and preserves existing page content and
annotations. Channel tolerance accepts 1–254, minimum component size 1–10,000 pixels and merge gap
0–16 pixels. A high tolerance or large minimum can hide a small change; the settings are retained.

This is a reviewed raster workflow. It does not perform semantic CAD comparison, automatic matching
across an entire plan set or guaranteed noise removal. Native operation, saved-output inspection,
independent-reader behavior and final-package checks require their own recorded results.
