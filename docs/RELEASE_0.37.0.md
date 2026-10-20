# SuniPDF 0.37.0

## Changes

- Image watermarks in the existing Watermark dialog and the `image-watermark` CLI command.
- Transparent foreground images, proportional width, angle, seven positions and displayed-page fitting.
- Atomic selected-page edits, one desktop Undo, copy-only CLI output and preserved allowed encryption.
- Independent geometry/pixel/refusal/protection regression with qpdf; published 0.36.0 upgrade baselines retained.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

Final-source Windows CI, CodeQL, package qualification and publication remain pending. Local core and generated-fixture CLI checks passed; native GUI interaction remains unqualified.

## Limits

Foreground image watermarks are editable content. No background/tile layer, DRM or signature verification is implied. Packages remain unsigned; ARM64 is cross-built only. The other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
