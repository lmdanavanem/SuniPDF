# SuniPDF 0.50.0 — reader controls and organized tools

[Français](RELEASE_0.50.0.fr.md) · [Reader and tools guide](READER_WORKSPACE.md)

This build includes the scope below. Published assets and qualification are tracked on the [0.50.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0).

## Changes

- Find 18 tools in five searchable families through **Tools**. Measurements, the annotation library, quantity table and revision overlay share **Review & measure**.
- Switch the left dock between Tools, Pages and Bookmarks. The main toolbar keeps a generic Tools entry rather than separate measurement and library shortcuts.
- Use four sections in the right measurement panel: Create measurement, Page scale, Drawing and Saved data. The measurement workspace no longer adds a horizontal bar above the PDF.
- Show one context panel at a time on the right. An active measurement draft blocks opening other context panels; explicitly leaving or changing tools cancels the local draft and preserves confirmed scales.
- Navigate pages and adjust zoom below the PDF. Percentage values such as **149%** stay on one line; page/width fit and a compact Reading options menu remain available as the window narrows.
- Keep view rotation separate from PDF page rotation. The right icon rail opens panels; the status area shows operation messages.
- Retain saved-vertex editing and its public-package regression checks. Include the published 0.49.0 packages in portable and MSI upgrade regressions.

## Qualification

Qualification records and distribution assets are tracked on the [0.50.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0). Results apply to the exact source identified there.

## Scope

This release reorganizes existing reader and tool workflows; it introduces no new CLI command or measurement type. Existing permissions, preview/apply guards and saved-measurement limits remain in force. AEC-002 and AEC-003 remain partial.

Automated offscreen UI evidence applies only to its tested layouts and interactions. It does not qualify native windows, physical devices, screen readers or native ARM64. Packages remain unsigned; broader 1.0 acceptance criteria remain open.
