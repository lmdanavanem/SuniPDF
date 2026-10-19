# SuniPDF 0.32.0

## Features and corrections

- Drag one object or the selected group in the Objects panel, with an insertion line, stale-drop guards and Escape cancellation.
- JSON `reorder-objects` command with an explicit final destination, relative-order preservation and existing save/password safeguards.
- Check effective modify permission before changing drawing order; reject invalid destinations and return from unchanged permutations before mutation.
- The public 0.31.0 packages join portable/MSI upgrade regressions.

[Scope and usage](OBJECT_ORDER.md).

Windows packages remain unsigned; no independent publication key is available. ARM64 is cross-built without hardware execution. Native/manual flows, physical printers, independent readers and PDF/A/PDF/UA certification remain separate qualification.

## Validation

Qualification is in progress; observed source/package/public results will be recorded in Project status.
