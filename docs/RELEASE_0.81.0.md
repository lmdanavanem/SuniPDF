# SuniPDF 0.81.0

Development candidate; publication and package qualification are pending.

## Multi-layer revision overlays

Compare two to four saved PDF pages on a fixed base-page canvas. Each layer has
its own color, visibility and opacity. Align each additional layer manually or
with two matching point pairs. The existing two-layer defaults retain magenta
and cyan; changing any setting requires a new preview before export.

The desktop shows the base and selected layer alongside the composite. Source
changes clear the corresponding alignment, while changing the base clears all
relative alignments. Open modified source documents must be saved or closed
before preparing or exporting their saved-file overlay.

PNG and JSON exports retain the source hashes, selected paths and resolved
alignments. Hidden layers are also permission-checked and revalidated. Changed,
missing or retargeted sources require a new preview. Sources remain unchanged;
new outputs never replace existing files. A completed export remains reported
if cancellation arrives after its commit.

`revision-overlay-stack` accepts a bounded local JSON manifest with two to four
layers and optional independent password environment references. The original
two-page `revision-overlay` command and report format remain available.

## Qualification and limits

Source, platform, image and package results will be recorded after qualification.
Retained source rasters and the composite are bounded to 80 MiB; this is not a
total process-memory limit. PDF sources are opened sequentially, with a separate
64 MiB limit per file. Each image's longest side is at most 2048 pixels.

This increment does not provide automatic registration, a vector comparison,
layered PDF output, desktop recipe import or a physical accuracy guarantee.
Native workflow interaction and broader interoperability retain separate gates.
AEC-013 remains In progress.

[Overlay workflow](REVISION_OVERLAY.md) · [Roadmap](ROADMAP.md).
