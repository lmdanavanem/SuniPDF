# SuniPDF 0.77.0

Development candidate. Published packages remain on the version identified in
[Project status](PROJECT_STATUS.md) until qualification and publication finish.

## Preview linked plan sheets

This increment adds an explicit destination preview to saved links between plan
sheets. The reader can display the verified destination page and sheet metadata
before opening it. Loading a manifest remains an explicit action, with a keyboard
path field alongside the existing file browser.

Previewing does not open another document or approve navigation. Opening the
destination verifies the inputs again, while existing edited-tab handling remains
in effect. Changed or missing files must be reported instead of substituted.

See [reviewed plan links](PLAN_LINKS.md) for the complete workflow and limits.

## Release validation efficiency

Pinned historical Windows packages are cached separately from current build
outputs. Cached bytes must pass the same pinned digest and published-checksum
checks as newly downloaded files. Real upgrade, rollback and close-timeout cases
remain part of qualification; caching does not remove them.

## Qualification status

Source checks, installed Ubuntu link interaction and package qualification are
pending. Synthetic fixture preparation, native application interaction and
independent structural checks will be recorded separately. Broader external-reader
interoperability, private-plan accuracy and physical-device qualification remain
separate gates. This increment does not establish complete AEC-017 acceptance.
