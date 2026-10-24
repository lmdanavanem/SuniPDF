# SuniPDF 0.36.0

## Changes

- Configurable desktop command shortcuts in Help and the command palette, with filtering, conflict/refusal, disable and restore controls.
- Atomic preferences; Cancel discards the draft and write failure preserves active settings. Existing installations retain their defaults.
- Effective bindings in menus, home, tooltips and palette; exact modifiers protect Save As and focused text editing.
- Published 0.35.0 ZIP/MSI packages join retained upgrade regressions. No new runtime dependency.

[Usage and limits](KEYBOARD_SHORTCUTS.md).

## Validation

[0.36.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) under `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) passed: **613 tests, zero failures**, 10 default ignored, **1128 EN/FR keys**, **28 portable scenarios**, **22 MSI baselines/110 logs**. Seven new headless egui tests cover shortcuts, conflicts, focus, display, cancellation and preference compatibility. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; **20 image cases** and **14 PDF-inspection cases**, plus retained suites, were rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,788,530-byte** x64 ZIP without installing it on the workstation.

Native Windows keyboard layouts and Narrator remain unqualified. Explorer remains separate; only commands in the existing shortcut table can be customized. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed.

[Detailed evidence](qa/RELEASE_0.36.0.json).

## Limits

Only existing desktop shortcut-table commands can be rebound; system/navigation/clipboard/text editing keys remain reserved. Explorer integration stays open. Packages remain unsigned without an independent update-publication key. ARM64 is cross-built only; the other [1.0 criteria](RELEASE_1.0_CRITERIA.md) remain open. Projects edits remain deferred at the owner’s request.
