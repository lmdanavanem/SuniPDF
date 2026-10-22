# SuniPDF 0.50.0 — reader controls and organized tools

[Français](RELEASE_0.50.0.fr.md) · [Reader and tools guide](READER_WORKSPACE.md)

Status: published as stable Latest [0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0), source `fb5f66b30ae30506dcd53f31ed03a5377260eb20`, on 2026-10-22. [Previous release](RELEASE_0.49.0.md).

## Changes

- Find 18 tools in five searchable families through **Tools**. Measurements, the annotation library, quantity table and revision overlay share **Review & measure**.
- Switch the left dock between Tools, Pages and Bookmarks. The main toolbar keeps a generic Tools entry rather than separate measurement and library shortcuts.
- Use four sections in the right measurement panel: Create measurement, Page scale, Drawing and Saved data. The measurement workspace no longer adds a horizontal bar above the PDF.
- Show one context panel at a time on the right. An active measurement draft blocks opening other context panels; explicitly leaving or changing tools cancels the local draft and preserves confirmed scales.
- Navigate pages and adjust zoom below the PDF. Percentage values such as **149%** stay on one line; page/width fit and a compact Reading options menu remain available as the window narrows.
- Keep view rotation separate from PDF page rotation. The right icon rail opens panels; the status area shows operation messages.
- Retain saved-vertex editing and its public-package regression checks. Include the published 0.49.0 packages in portable and MSI upgrade regressions.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854366778) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854380539) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854366334). 776 tests passed, 11 default ignored. 1413 EN/FR keys. 42 portable scenarios. 36 MSI baselines. 180 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 63 retained packaged CLI calls verify insertion and removal across polyline, perimeter and area measurements, reopened vertex counts, calibration and quantity preservation, recalculated totals, minimum/maximum and fixed-kind bounds, invalid/stale refusals and the retained measurement lifecycle. They do not establish native desktop or ARM64 qualification. 3 application/PDFium scenes rendered offscreen with DX12 WARP, including the French 720 × 480 layout at 149% zoom. Reports, PNGs and decoded pixels rechecked by SHA256; native windows and physical devices remain unqualified. [Evidence](qa/RELEASE_0.50.0.json).

## Scope

This release reorganizes existing reader and tool workflows; it introduces no new CLI command or measurement type. Existing permissions, preview/apply guards and saved-measurement limits remain in force. AEC-002 and AEC-003 remain partial.

Automated offscreen UI evidence applies only to its tested layouts and interactions. It does not qualify native windows, physical devices, screen readers or native ARM64. Packages remain unsigned; broader 1.0 acceptance criteria remain open.
