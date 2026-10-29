# SuniPDF 0.92.0

By Emmanuel Almeida.

Prepared for publication on 2026-10-29. Tagged package checks and public download verification
are pending. [Project status](PROJECT_STATUS.md).

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

Before the version and author metadata update, local validation passed 1,363 workspace tests
with zero failures and 16 environment-specific tests ignored. Strict Clippy, formatting and
2,197 matching English/French translation keys passed. Three About scenes were rendered and
reviewed using offscreen DX12 WARP: French/light, English/dark and a 720 × 480 window.
These results do not qualify native window input or physical graphics hardware.

Windows packages remain unsigned. Windows ARM64 is cross-built without native hardware
execution. The macOS app is ad hoc signed, not notarized. Existing platform and interoperability
limits remain documented in [known issues](KNOWN_ISSUES.md) and [project status](PROJECT_STATUS.md).
