# SuniPDF 0.38.0

## Changes

- Six editable header/footer zones in Pages → Page numbers and the `headers-footers` CLI command.
- Absolute page/count placeholders, font size, margins, page selection and displayed rotation/crop coordinates.
- Refuse overflowing text atomically, preserve allowed encryption and keep CLI output copy-only.
- Extend independent package regression to 42 cases, 148 CLI calls and two qpdf checks.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

Final-source Windows CI, CodeQL, package qualification and publication remain pending. Local engine and CLI regressions passed; native GUI interaction remains unqualified.

## Limits

Single-line zones use equal-width columns and foreground text. Existing content can overlap; inspect before saving. No automatic dates, odd/even rules, complex-script qualification or conformance certification. Packages remain unsigned and ARM64 is cross-built only. Other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
