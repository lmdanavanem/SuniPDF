# Revision overlays

[Français](REVISION_OVERLAY.fr.md) · [0.47 scope and qualification status](RELEASE_0.47.0.md)

An overlay displays one saved base page in magenta and one saved revision page in cyan. It helps you inspect their alignment and visible differences. The PDFs remain unchanged. See the release notes for qualification and publication status.

## Desktop

1. Open **Revision overlay** from **All tools**, or use the same button in **Compare files**. The latter preselects the comparison's files and starting pages. No open PDF tab is required.
2. Use **Choose base PDF** and **Choose revision PDF**, then set each **Page**. Page numbers are physical and start at 1. Each source has its own masked **Password (optional)** field. Select saved files: unsaved changes in open tabs are not included.
3. Choose **Manual alignment** and set **Horizontal offset**, **Vertical offset**, **Clockwise rotation** and **Scale factor**. Choose **Preview** to load the two source images and the combined result.
4. Alternatively, choose **Two-point alignment**, then **Preview** to load the sources. Click point 1 and point 2 on the base image, then the corresponding points in the same order on the revision. Choose **Preview** again to calculate the alignment. Each pair must contain two distinct points inside its page. **Reset alignment** clears all four points and restores zero offsets, zero rotation and scale 1.
5. Adjust **Revision opacity** and **Maximum image side** as needed. The base stays magenta at full density; the revision is cyan. Choose **Preview** again after any setting changes. Export stays disabled while the shown preview is out of date. Changing a source file or page also clears its previous images and alignment.
6. Inspect **Overlay preview** and, if needed, expand **Source fingerprints (SHA-256)**. Use **Export PNG** or **Export report JSON** and choose a new filename. Each export writes one file and checks both saved sources again. If either source changed, prepare a new preview.

Switching from a completed two-point alignment to manual alignment retains its calculated transform for refinement. The point selections remain available until reset or a source/page change. Resetting alignment leaves the opacity and resolution settings intact. No alignment settings are written into either PDF.

**Cancel** or **Escape** requests cancellation during work; the dialog waits for the worker to finish its current bounded operation. When idle, **Close** or **Escape** closes it. An export already committed before cancellation remains saved and is acknowledged. Source passwords stay in memory and are cleared when closing the dialog or replacing the corresponding file; the report does not contain them.

## Coordinates and image limits

Overlay coordinates start at the **top-left of the displayed page**, after its crop and page rotation. X increases rightwards and Y downwards. One PDF point is 1/72 inch. This differs from the lower-left coordinates used by annotation placement.

The transform maps the revision onto the base: first scale uniformly around the revision's displayed top-left origin, then rotate clockwise around that origin, then apply the horizontal and vertical offsets. With no adjustments, scale is 1 and both offsets and rotation are zero. Two point pairs determine translation, rotation and uniform scale; they do not introduce shear or perspective correction.

The base page fixes the output width, height and crop. Revision content outside that canvas is clipped; the canvas does not grow to include it. Each source is rendered independently within the selected pixel limit, then sampled into the overlay. Pages with very different physical sizes can therefore have different raster densities. Enlarging a source cannot recover its fine detail, and apparent line differences can include rasterization effects.

| Setting | Limit |
|---|---|
| Saved PDF size | 64 MiB per source |
| Selected pages | One physical page from each PDF |
| Maximum image side | Default 1536 px; desktop choices 512, 1024, 1536 or 2048; CLI 64–2048 |
| Revision opacity | 0–1; default 0.65 |
| Uniform scale | 0.01–100 |
| Clockwise rotation | −360° to 360° |
| Horizontal/vertical offset | −1,000,000 to 1,000,000 pt |
| Anchor pairs | Two points per page, at least 0.001 pt apart, within the displayed page |

Both sources must permit copying, including when an owner password is supplied. PNG and JSON exports are ordinary files; source PDF encryption is unchanged because no source is rewritten. Source hashes identify exact file bytes, not the accuracy of alignment or the completeness of a revision review.

## CLI

These examples assume that the selected pages contain the listed reference points. Start with manual alignment:

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --base-page 1 --revision-page 1 --translate "10,-10" --rotation 0 --scale 1 --opacity 0.65 --max-side 1536 --out overlay-manual.png
```

Or align two matching pairs and write the report:

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --base-points "50,50 150,50" --revision-points "40,60 140,60" --format json --out overlay-report.json
```

The point options require each other and cannot be combined with `--translate`, `--rotation` or `--scale`. `--format` is `png` by default; use `json` explicitly for a report. Existing destinations are refused, including with `--overwrite`. Each invocation creates one new file.

To require that a later PNG export uses the exact same source bytes as the report:

```powershell
$report = Get-Content -LiteralPath overlay-report.json -Raw | ConvertFrom-Json
sunipdf-cli revision-overlay base.pdf revision.pdf --base-points "50,50 150,50" --revision-points "40,60 140,60" --expected-base-sha256 $report.base.source_sha256 --expected-revision-sha256 $report.revision.source_sha256 --out overlay-aligned.png
```

For protected files, supply each password separately. With two existing environment variables:

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --password-env SUNIPDF_BASE_PASSWORD --revision-password-env SUNIPDF_REVISION_PASSWORD --out overlay-protected-source.png
```

`--password-stdin` reads the base password and `--revision-password-stdin` reads the revision password. Only one password may use stdin in the same invocation; use an environment variable for the other. Passwords are not automatically shared between sources.

## JSON report

The report uses `schema: "sunipdf-revision-overlay"` and `version: 1`. Its `base` and `revision` objects record canonical source paths, SHA-256, physical page number, page count, displayed dimensions in points, the effective crop box in PDF user coordinates, page rotation and source raster dimensions.

`transform` records the resolved translation, clockwise rotation and uniform scale. `alignment` records the chosen mode and its inputs. For example, this is the `alignment` section for the manual command above, not a complete report:

```json
{
  "mode": "manual",
  "transform": {
    "translation": [10, -10],
    "rotation_degrees": 0,
    "scale": 1
  }
}
```

For two-point alignment, `alignment.mode` is `two_points` and `alignment.base` and `alignment.revision` contain the two point pairs. The report also records the base canvas and clipping, colors, opacity, pixel limit, bilinear sampling and raster limitations. It contains no PDF content edits. Opening a report to restore desktop settings is not part of this release; the CLI accepts explicit parameters and optional expected source hashes.

AEC-013 remains partial. Automatic registration, automatic change outlines and tracked revision changes under AEC-014 remain open. This tool does not compare vectors, perform OCR, merge PDFs or recalibrate measurements.
