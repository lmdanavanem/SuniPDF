# SuniPDF 0.92.0

By Emmanuel Almeida.

Published stable on 2026-10-29 under [lmdanavanem](https://github.com/lmdanavanem).
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.0) | [Qualification evidence](qa/RELEASE_0.92.0.json) | [Project status](PROJECT_STATUS.md).

## About

The About window has a new layout in English and French, with a centered logo and version,
privacy information, update status, recovery settings, license links and author credits.
The update and recovery switches retain their existing behavior. Update status reflects the
last actual check. The header and Close button remain accessible in smaller windows.

## Hide and show objects

Click the eye in the Objects panel to hide a page object; click again to show it. Page tiles,
thumbnails and canvas picking follow the chosen visibility. The temporary state stays with
each open tab and does not remove objects from a saved PDF or create an Undo step.
Document edits reset these masks because page-object indices may change.
This is not a PDF optional-content-group editor. [Objects guide](OBJECT_ORDER.md).

## Revision overlay visibility

Click a layer name or its eye to hide or show it and refresh the preview automatically.
Visibility changes reuse the existing source rasters after checking source fingerprints again.
Every layer can be hidden to show a white canvas. PNG exports, reports and saved setups use
the displayed visibility, with the existing source-validation requirements.
[Overlay guide](REVISION_OVERLAY.md).

## Validation and scope

The exact tagged source is `306fbc25a39f86fad0aa24d31b130ce0dcd50562`.
All 11 jobs in the [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40803173375)
passed, including Windows packages, Ubuntu 22.04/24.04 installation checks and macOS arm64 checks.
The Windows x64 workspace passed 1,363 tests with zero failures and 16 tests ignored by default;
the release workflow runs its environment-specific gates separately. Strict Clippy, formatting
and 2,197 matching English/French translation keys passed. Upgrade checks cover 59 portable
baselines plus four guards and 57 MSI baselines, including 0.91.2.

The packaged overlay-stack corpus passed 137 checks across 25 commands and 13 expected refusals.
It independently checks every pixel of the all-hidden PNG, the retained source identities,
geometry, styles and transforms, matching JSON output, and refusal of missing or stale sources.
Three About scenes were regenerated on the final source using offscreen DX12 WARP: French/light,
English/dark and a 720 × 480 window. Their hashes match the visually reviewed frames.
These results do not qualify native window input or physical graphics hardware.

All five packages and their SHA256 files were downloaded anonymously after publication.
Every downloaded file matched its expected size and GitHub SHA256 digest; each package also
matched its checksum file. The stable Latest release was verified at 2026-10-29T02:55:40Z.

Windows packages remain unsigned. Windows ARM64 is cross-built without native hardware
execution. The macOS app is ad hoc signed, not notarized. Existing platform and interoperability
limits remain documented in [known issues](KNOWN_ISSUES.md) and [project status](PROJECT_STATUS.md).
