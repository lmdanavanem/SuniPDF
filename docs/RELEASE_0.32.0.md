# SuniPDF 0.32.0

## Features and corrections

- Drag one object or the selected group in the Objects panel, with an insertion line, stale-drop guards and Escape cancellation.
- JSON `reorder-objects` command with an explicit final destination, relative-order preservation and existing save/password safeguards.
- Check effective modify permission before changing drawing order; reject invalid destinations and return from unchanged permutations before mutation.
- The public 0.31.0 packages join portable/MSI upgrade regressions.

[Scope and usage](OBJECT_ORDER.md).

Windows packages remain unsigned; no independent publication key is available. ARM64 is cross-built without hardware execution. Native/manual flows, physical printers, independent readers and PDF/A/PDF/UA certification remain separate qualification.

## Validation and distribution

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) under `lmdanavanem`, release `417623635`, immutable source `8c492db35f1712648d5472593146077230befb7c`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) and [release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) passed: **588 tests**, zero failures, 9 default ignored; **1092 EN/FR keys**, 84 qpdf corpus checks + 9 independent feature cases, 4 font-reader cases, DX12 software rendering, 14 Office round trips, 4 licensed PDFs/744 pages, **24 portable scenarios**, **18 MSI baselines/90 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. The packaged CLI passed **11 object-order scenarios and 3 additional qpdf checks**, plus 11 retained local-review and 8 security scenarios.

Six public files and `dist/` copies verified by SHA256. The freshly built harness from this source downloaded and verified the **27,119,935-byte x64 ZIP in 1.75 s**, without installation. Projects: **43 items, 25 Done, 18 Todo, zero In Progress**; the P3 Suggestion is delivered, original criteria preserved, description read back after a server reload and all three saved filters verified.

[Detailed evidence](qa/RELEASE_0.32.0.json).
