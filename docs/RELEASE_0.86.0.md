# SuniPDF 0.86.0 — candidate

This candidate adds reviewed bulk editing of annotation-register fields. Publication and package qualification are pending.

## Explicit selection and reviewed changes

- Select up to 256 register rows without implicitly selecting filtered or hidden rows.
- Keep, set or clear business status, discipline and individual custom text keys.
- Preview before/after values and confirm the review before applying.
- Apply the complete batch as one Undo/Redo operation. Stale, locked or invalid rows refuse the complete operation.
- Use the same bounded source-bound operation through `annotation-fields-batch inspect`, `preview` and `apply`.

The CLI saves a new copy and refuses existing output files. Protected sources use environment or standard-input passwords and retain permission checks. Page content, appearances, count groups, PDF review states and typed quantity columns are outside this operation.

## Validation scope

Core, CLI and desktop worker tests cover atomicity, stale rows, cancellation, encryption permissions and source preservation. An independent synthetic corpus checks saved annotation dictionaries and decoded appearance/page streams. These tests are included in CI; successful runtime qualification has not yet been recorded for this candidate.

[Bulk field guide](REGISTER_BATCH.md) · [Annotation register](MARKUP_REGISTER.md) · [Project status](PROJECT_STATUS.md)
