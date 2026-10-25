# SuniPDF 0.83.0

Development candidate; publication and package qualification are pending.

## Save and reopen overlay setups

Save the current verified overlay as a new setup file, then reopen it to recover
the selected pages, layer colors, visibility, opacity and alignment. The setup
contains every source PDF's SHA256, including hidden layers. Source paths are
relative when they are below the setup directory; other selections stay explicit
local paths. Moving a setup together with its relative source files is supported.

Loading a setup clears the prepared result. A fresh preview checks permissions,
source hashes and current files before exports become available. Missing or
changed sources are refused; choosing a replacement explicitly clears that
source binding and its alignment. Replacing the base clears all alignments.
Modified open source documents remain protected by the existing guards.

Setups never contain passwords or environment-variable references. Protected
PDFs require their passwords again. Desktop import requires source hashes for
every layer. The existing version-1 CLI manifest keeps its optional password
environment references; `revision-overlay-stack --format setup` writes a clean,
fully bound setup from a verified preview.

## Qualification and limits

Source, platform, setup round-trip and package results will be recorded after
testing. A setup stores configuration and hashes; it does not embed PDF files,
rendered images or credentials. It creates a new file and never overwrites an
existing setup or PDF.

The existing two-to-four-layer, raster, source-size and permission limits remain.
This increment does not add automatic alignment, automatic rebinding of changed
sources or layered PDF output. Native desktop interaction and broader reader
interoperability retain separate qualification gates. AEC-013 remains In progress.

[Overlay workflow](REVISION_OVERLAY.md) · [Roadmap](ROADMAP.md).
