# SuniPDF 0.91.2

Published stable on 2026-10-28. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.91.2) | [Qualification evidence](qa/RELEASE_0.91.2.json).

This corrective release follows [SuniPDF 0.91.0](RELEASE_0.91.0.md). It adds no new workflow.
It contains the same corrections as 0.91.1, which was withdrawn shortly after publication.
The tagged source passed all 11 release jobs, including upgrades from the published 0.91.0 MSI and
portable packages. All ten public downloads were checked anonymously against their SHA256 values.

## Saved text library

- Placing a saved text entry no longer depends on the exact click origin. The text box keeps its
  saved dimensions within 0.001 pt of f32 rounding, the tolerance already used for saved stamps.
  Previously, about half of ordinary click origins were refused.
- Saved native text on rotated or offset-crop pages is captured from its appearance box.
- Listing capture candidates verifies native text up to a fixed per-page budget. Beyond it, the
  check runs when the selection is captured, which still refuses an appearance that does not reproduce.

## Annotation register fields

- A batch or single-annotation field change is refused as a whole when its stored text would exceed
  the register limits (32 KiB per record, 4 MiB per document). The document stays unchanged.
- An unreadable saved-filter library in the preferences file is kept as it is until new filters are
  explicitly saved, instead of being erased when another preference is written.

## Interface

- Overlay, register-field, saved-filter and annotation-library messages and file-type labels are
  translated. Detailed engine refusal reasons remain in English inside a translated sentence; see
  [known limitations](KNOWN_ISSUES.md).
- Field-change controls and the saved-filter selector have accessible names.
- Overlay setups saved to a mapped network drive are refused with an explicit local-drive message.
  Checking unsaved overlay sources no longer accesses the file system from the window thread.

See the [changelog](../CHANGELOG.md) and the [0.91.0 qualification](RELEASE_0.91.0.md) for the
unchanged scope and limits.
