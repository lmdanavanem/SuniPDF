# Revision overlays
[Original two-page qualification](RELEASE_0.47.0.md) · [Multi-layer scope and saved setups](RELEASE_0.91.0.md).

## Desktop

1. Open **Revision overlay** from **Tools**, or use the same button in **Compare files**. The latter preselects the comparison's files and starting pages. No open PDF tab is required.
2. Choose the base and second saved PDF, then set each **Page**. Use **Add layer** for up to four layers. Pages are physical and start at 1; each source has its own masked password field. Save or close modified source tabs first, since the overlay reads saved files.
3. Select a non-base layer to align. Choose **Manual alignment** and set **Horizontal offset**, **Vertical offset**, **Clockwise rotation** and **Scale factor**. **Preview** loads the base, selected source and combined result.
4. Alternatively, choose **Two-point alignment**, then **Preview** to load the sources. Click two points on the base image and the corresponding points in the same order on the selected source. **Preview** recalculates alignment. Each pair must contain two distinct in-page points. **Reset alignment** clears the selected layer's points and restores the identity transform.
5. Adjust each layer's color, visibility and opacity. The initial pair remains magenta/full density and cyan/0.65 density. At least one layer must remain visible. **Maximum image side** limits the source and output rasters. Changed settings disable export until a fresh preview succeeds. Changing a non-base source or page clears its alignment; changing the base clears every relative alignment.
6. Inspect **Overlay preview** and **Source fingerprints (SHA-256)**. **Export PNG** or **Export report JSON** creates one new file, rechecking every source, including hidden layers. Changed, missing or retargeted sources require a new preview.

Switching from a completed two-point alignment to manual alignment retains its calculated transform for refinement. The point selections remain available until reset or a source/page change. Resetting alignment leaves the opacity and resolution settings intact. No alignment settings are written into either PDF.

**Cancel** or **Escape** requests cancellation during work; the dialog waits for the worker to finish its current bounded operation. When idle, **Close** or **Escape** closes it. An export already committed before cancellation remains saved and is acknowledged. Source passwords stay in memory and are cleared when closing the dialog or replacing the corresponding file; the report does not contain them.

## Save and reopen a setup

After a successful **Preview**, choose **Save setup copy** to create a new JSON
file with the pages, colors, visibility, opacity, alignment and every PDF's
fingerprint. Use **Open setup** to restore it, enter any required passwords, then
**Preview** again before exporting. Loading a setup does not modify a PDF.

The file stores configuration and source paths, without embedding PDFs, preview
images or credentials. Paths below the setup directory are relative, so the
setup and those PDFs can move together. Other paths remain explicit local paths.
Every source is checked, including hidden layers. Missing or changed files are
refused; choose a replacement explicitly if intended. Replacing a source clears
its saved fingerprint and alignment; replacing the base resets all alignments.
Changing colors or other display settings retains source fingerprints.

Desktop setup files require a fingerprint for every layer and cannot contain
password environment references. A CLI report remains a separate output; open
the saved setup file to restore the controls.

## Coordinates and image limits

Overlay coordinates start at the **top-left of the displayed page**, after its crop and page rotation. X increases rightwards and Y downwards. One PDF point is 1/72 inch. This differs from the lower-left coordinates used by annotation placement.

The transform maps the revision onto the base: first scale uniformly around the revision's displayed top-left origin, then rotate clockwise around that origin, then apply the horizontal and vertical offsets. With no adjustments, scale is 1 and both offsets and rotation are zero. Two point pairs determine translation, rotation and uniform scale; they do not introduce shear or perspective correction.

The base page fixes the output width, height and crop. Revision content outside that canvas is clipped; the canvas does not grow to include it. Each source is rendered independently within the selected pixel limit, then sampled into the overlay. Pages with very different physical sizes can therefore have different raster densities. Enlarging a source cannot recover its fine detail, and apparent line differences can include rasterization effects.

| Setting | Limit |
|---|---|
| Saved PDF size | 64 MiB per source |
| Selected pages | One physical page from each of 2–4 layers |
| Maximum image side | Default 1536 px; desktop choices 512, 1024, 1536 or 2048; CLI 64–2048 |
| Layer opacity | 0–1; default 1 for the base and 0.65 for others |
| Retained RGBA rasters | Up to 80 MiB for all sources and the composite; excludes native-document and PNG-encoding memory |
| Uniform scale | 0.01–100 |
| Clockwise rotation | −360° to 360° |
| Horizontal/vertical offset | −1,000,000 to 1,000,000 pt |
| Anchor pairs | Two points per page, at least 0.001 pt apart, within the displayed page |

Every source must permit copying, including hidden layers and when an owner password is supplied. Sources are opened, rendered and released sequentially. PNG and JSON exports are ordinary files; source PDF encryption is unchanged because no source is rewritten. Source hashes identify exact file bytes, not the accuracy of alignment or the completeness of a revision review.

Composition multiplies each visible layer's tinted darkness over white. Layer order does not change the pixels. The first layer still defines the canvas when hidden, so isolating another layer does not change its alignment or output dimensions.

## Multi-layer CLI

Use `revision-overlay-stack` with a UTF-8 manifest of at most 256 KiB:

```json
{
  "version": 1,
  "long_side": 1536,
  "layers": [
    {"path": "base.pdf", "page": 1, "color": [255, 0, 255], "opacity": 1},
    {
      "path": "revision.pdf",
      "page": 1,
      "color": [0, 255, 255],
      "opacity": 0.65,
      "alignment": {
        "mode": "manual",
        "transform": {"translation": [10, -10], "rotation_degrees": 0, "scale": 1}
      }
    },
    {"path": "services.pdf", "page": 1, "color": [0, 180, 70], "visible": true}
  ]
}
```

```powershell
sunipdf-cli revision-overlay-stack --manifest layers.json --out coordinated.png
sunipdf-cli revision-overlay-stack --manifest layers.json --format json --out coordinated-report.json
sunipdf-cli revision-overlay-stack --manifest layers.json --format setup --out coordinated-setup.json
sunipdf-cli revision-overlay-stack --manifest coordinated-setup.json --out coordinated-again.png
```

There must be two to four layers. Relative paths resolve against the manifest directory; explicit local absolute paths are accepted. Remote paths, unknown fields and unsupported versions are refused. The first layer requires identity manual alignment. Other layers may use the same `two_points` representation described below. Omitted pages default to 1, omitted visibility to true, and omitted styles use the four-layer palette. All-hidden stacks are refused.

Each input layer may add `expected_sha256` to require exact saved source bytes. Protected sources may name an existing environment variable in `password_env`; password values are never stored in the manifest or report. Global password flags are refused for this command. Each invocation writes one new PNG, JSON report or setup file and refuses existing destinations, even with `--overwrite`. A setup export adds every verified source hash and omits all credential references; protected sources require passwords again when reused.

The stack report has `schema: "sunipdf-revision-overlay-stack"`, `version: 1` and a `layers` array. Each zero-based layer entry records its source identity, selected path, resolved transform, alignment, color, visibility and opacity. `canvas.base_layer` is 0. This report describes the reviewed snapshot; it is not a desktop-importable recipe.

## Two-page CLI

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

AEC-013 remains In progress. Automatic registration, layered/vector PDF output and broader native/interoperability qualification remain open. Reviewed change-outline proposals are a separate comparison workflow. This tool does not compare vectors, perform OCR, merge PDFs or recalibrate measurements.
