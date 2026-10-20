# SuniPDF 0.36.0

## Changes

- Configurable desktop command shortcuts in Help and the command palette, with filtering, conflict/refusal, disable and restore controls.
- Atomic preferences; Cancel discards the draft and write failure preserves active settings. Existing installations retain their defaults.
- Effective bindings in menus, home, tooltips and palette; exact modifiers protect Save As and focused text editing.
- Published 0.35.0 ZIP/MSI packages join retained upgrade regressions. No new runtime dependency.

[Usage and limits](KEYBOARD_SHORTCUTS.md).

## Validation

Final-source CI, CodeQL, package qualification and publication remain pending. Headless egui tests cover input, settings and cancellation; no native keyboard, Narrator or hardware qualification is claimed.

## Limits

Only existing desktop shortcut-table commands can be rebound; system/navigation/clipboard/text editing keys remain reserved. Explorer integration stays open. Packages remain unsigned without an independent update-publication key. ARM64 is cross-built only; the other [1.0 criteria](RELEASE_1.0_CRITERIA.md) remain open. Projects edits remain deferred at the owner’s request.
