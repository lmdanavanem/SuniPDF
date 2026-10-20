# SuniPDF 0.35.0

## Changes

- Optional smoothing for drawn visual signatures, using the same curve geometry for preview and placement while retaining the original points for Undo and comparison.
- Optional local paper-shadow reduction for imported signatures, with a preview bound to every cleanup setting and placement using those exact prepared pixels.
- A `prepare-signature-image` command to create a separate PNG using keep, global-threshold or adaptive-background modes, without opening a PDF.
- Explicit drawing budgets and input/output guards, plus independent packaged image, EXIF, PDF-rendering and qpdf regression evidence.
- Upgrade checks retain previous release baselines and add the published 0.34.0 packages.

[Usage and limits](VISUAL_SIGNATURE_PREPARATION.md).

## Validation

Final-source Windows CI, CodeQL and release-package qualification are pending. No release publication, native GUI interaction or host installation is claimed by this preparation note.

## Limits

Visual signatures do not authenticate an identity or provide cryptographic integrity. Cleanup targets thin dark ink on light paper and may remove faint marks; review the preview. Native mouse/tablet/Narrator interaction and customer-photo coverage still need qualification.

Windows packages remain unsigned without an independent update-publication key. ARM64 is cross-built without execution on ARM64 hardware. Physical printing, broader hostile corpus/fuzzing coverage, named third-party reader checks and PDF/A/PDF/UA conformance remain open under the [1.0 criteria](RELEASE_1.0_CRITERIA.md). Projects edits remain deferred at the owner's request.
