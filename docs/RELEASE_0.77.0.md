# SuniPDF 0.77.0

Published stable on 2026-10-25. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.77.0) · [Qualification evidence](qa/RELEASE_0.77.0.json).

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

## Qualification

The tagged source passed all 11 release jobs. Windows qualification includes
13 offscreen interface scenes, 54 portable upgrade baselines plus
4 guards, and 52 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 21 installed runtime cases, including 5 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-25T01:56:30.631866+00:00.

Visual review covers 11 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.77.0.json).

Native plan-link navigation uses synthetic PDFs prepared by the installed CLI.
It covers preview/cancel, changed or missing input refusals and the correct
physical destination page. Desktop link creation, desktop link saving,
dirty-destination interaction and broader external-reader interoperability
remain open. These results do not establish complete AEC-017 acceptance.

Windows packages remain unsigned, and Windows ARM64 is cross-built without
native hardware execution. The Apple Silicon app is ad hoc signed, not
notarized; macOS 12 is a binary-checked deployment minimum. Physical Macs,
interactive Finder opening and clipboard interoperability remain unqualified.
Ubuntu updater-helper checks do not establish graphical authorization or a
complete desktop updater interaction. Physical GPUs, Wayland, printers and
human accessibility remain separate gates.
