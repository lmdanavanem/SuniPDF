# SuniPDF 0.46.0 — quantity tables and typed business fields

[Français](RELEASE_0.46.0.fr.md) · [Quantity-table guide](QUANTITY_TABLE.md)

Status: published as stable Latest [0.46.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.46.0), source `3e6763964d6b06f004a09fd7473ddf5bb4a6417f`, on 2026-10-22. [Previous release](RELEASE_0.45.0.md).

## Scope

**Quantity table** is available from **All tools**, the measurement controls and the annotation register. It reads saved count markers and measurements alongside their source annotations. Real filters and 50-row pagination keep the displayed rows and their subtotals aligned.

Each eligible annotation can carry an explicit unit price, currency, price-basis unit and rounding precision, plus typed text, number, date and choice columns. **Apply fields** changes the annotation metadata in one Undo step; **Cancel edit** discards the draft. Saving the PDF retains the fields. Measurement geometry, annotation identities and review threads are preserved.

Missing quantities and prices stay absent rather than becoming zero. Count, length and area totals retain their units. Prices can convert quantities to a compatible, explicitly selected price basis; currencies are never converted. Costs are rounded per line and summed separately by currency and rounding precision. Missing prices, excluded rows and incomplete cost coverage remain visible.

CSV and JSON exports include source-snapshot provenance, filters and subtotal coverage. Export requires copying permission and a new output file. Edits require annotation permission and a current target. The CLI provides `quantity-table`, `set-quantity-fields` and `clear-quantity-fields`; mutation output is a new PDF copy, including for supported protected inputs.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738061382) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738073755) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738061664). 732 tests passed, 10 default ignored. 1348 EN/FR keys. 38 portable scenarios. 32 MSI baselines. 160 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 32 packaged CLI calls verify count/measurement quantities, typed fields, exact costs, filters/groups, CSV guarding, unit conversion, clearing, stale and overwrite refusals, AES permissions and source preservation. [Evidence](qa/RELEASE_0.46.0.json).

## Remaining limits

AEC-007 remains partial. This release does not provide arbitrary formulas, tax rules, currency exchange, automatic symbol counting, cross-document aggregation or a full estimating system. Angles remain visible but are excluded from additive and priced totals. Ordinary annotations without a supported quantity are not assigned an invented quantity. Column types and choice options must remain consistent within a document; bounds and stale or unsupported metadata can cause an explicit refusal. Native/hardware and 1.0 qualification gates remain separate.
