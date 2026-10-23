# Quantity tables

[0.61 candidate scope](RELEASE_0.61.0.md) · [Measurements](MEASUREMENTS.md) · [Annotation register](MARKUP_REGISTER.md)

Pricing and typed fields shipped in [0.46.0](RELEASE_0.46.0.md). [That release's evidence](qa/RELEASE_0.46.0.json) does not qualify the new custom sorting and report formulas.

## Open and filter the table

Open a PDF, then choose **Quantity table** from **All tools**, from the measurement controls, or inside the annotation register. Finish or cancel another active draft first. Reading the table leaves the PDF unchanged.

The rows show physical page, annotation index, type, contents, quantity, unit, unit price, line cost and available custom columns. Pages start at 1; annotation indices start at 0. An explicit count marker contributes `1 each`. A saved length, polyline, perimeter or area contributes its measured value and unit. An angle is shown in `deg` but is not added or priced. Ordinary annotations have no inferred quantity.

Filter by search text, annotation type, discipline, business status, count group, author or physical page. Page `0` means all pages in the desktop filter. **Subtotal groups** offers **No grouping**, **Count group**, **Discipline** and **Business status**. Subtotals cover all matching rows, including those beyond the current 50-row page. **Previous 50** and **Next 50** navigate table pages; **Filter column names** changes which custom columns are shown without changing row selection or totals. **Refresh** rereads the current document after an edit has been applied or cancelled.

Use **Custom column** to sort by a typed field. Numbers sort numerically, ISO dates chronologically, and text/choice values lexically with case preserved. Missing values and blank text stay last in both directions; ties use physical page and annotation index. Choose **Built-in order** to return to the ordinary sort. This is separate from the annotation register's custom text fields, which sort as text. Docked or floating tables can stay open while following a row to its annotation.

## Edit price and custom fields

1. Choose **Edit row** on an eligible annotation. Its identity must be unambiguous, its state editable and annotation permission available.
2. Enter **Unit price**, a three-letter uppercase **Currency code**, **Price per unit** and **Cost decimal places** (`0`–`4`). Use a decimal point, not an expression. A blank price is absent; `0` is a supplied zero price. The price basis must match the quantity dimension: for example, metres can be priced per foot, but an area cannot be priced per metre.
3. Under **Custom columns**, choose **Add custom column** and provide its name, type and value. Supported types are **Text**, **Number**, **Date (YYYY-MM-DD)** and **Choice**. For a choice, enter the permitted options one per line and enter one of them as the value. A column name must use the same type and choice options everywhere in the PDF. Existing register text fields remain separate from these typed fields.
4. Choose **Apply fields** to commit the row, or **Cancel edit** to discard the draft. **Remove column** removes that value only when the row is applied. An empty text value remains supplied; removing the column makes it absent. Invalid values or a stale target keep the PDF unchanged and preserve the draft for correction or cancellation.

Each successful row edit is one Undo step. Finish or cancel the row editor before using **Undo** or **Redo**. The read-only docked or floating table can stay open; refresh it to inspect restored values. Save the PDF to retain the fields after reopening. These edits do not change measured vertices, calibration, annotation identity or linked comments. A price cannot be attached to an ordinary annotation without a supported quantity or to an angle; typed custom values can be attached to eligible ordinary annotations.

## Quantities, costs and missing values

**Not supplied** means absent data; it is not a zero. No manual quantity override is provided here: edit the actual measurement or count markers in their own tools.

Quantity subtotals retain their original units. `m` and `ft`, or `m2` and `ft2`, remain separate even though compatible price-basis conversion exists. There is no combined length/area/count total. Quantity units are `each`, `mm`, `cm`, `m`, `in`, `ft`, `mm2`, `cm2`, `m2`, `in2`, `ft2`; `deg` is display-only for this calculation.

For each priced row, the quantity is converted to the explicit price unit and quantized to six decimal places. Decimal unit price multiplication then uses checked integer arithmetic. Line cost is rounded to the selected `0`–`4` decimals, half away from zero; subtotals sum these rounded line costs. Monetary groups remain separate by currency code and rounding precision. A currency code is a declaration, not a checked exchange-rate lookup.

**Filtered subtotals** reports missing prices and partial cost coverage. A monetary subtotal is complete only when every row in that quantity group contributes to that same currency/precision subtotal. Consequently, mixed currencies can produce partial subtotals even when every row has a price. Ordinary unquantified rows and angles are excluded from additive totals and counted in `unquantified_rows`. The screen shows at most 30 subtotal groups; the export contains the complete filtered summary. Apply filters to inspect additional groups.

## Report formula columns

Open **Formula columns**, enter a unique column name and expression, select the required quantity unit and `0`–`6` result decimal places, then choose **Add and preview**. Up to eight columns can be active. Formula configuration stays in the current tab and does not modify PDF fields, quantities or prices. Under **Reuse formula configuration**, **Copy configuration** copies its JSON; paste an array and choose **Load configuration** to reuse it. Keep a copy for another tab or session.

Expressions accept decimal constants, unary signs, `+ - * /`, parentheses, `quantity` and exact case-sensitive numeric field references such as `[Waste %]`. Only typed **Number** fields participate; register text fields and other formula results are not inputs. For example, `quantity * (1 + [Waste %] / 100)` applies a dimensionless allowance to each row's actual quantity.

- `quantity` requires the exact declared unit; metres and feet are not converted by formulas. The existing price-basis conversion is separate.
- Numeric fields and constants are dimensionless. Addition/subtraction require matching units. Multiplication permits at most one quantity operand; division by a scalar retains the quantity unit, while equal-unit division produces a dimensionless result. Squared/cubed units and inverse units are not inferred.
- Arithmetic uses checked decimal values with six fractional places. Multiplication/division round half away from zero at each operation; the final result uses the selected `0`–`6` decimal places with the same rule. Intermediate values must remain within the supported one-trillion absolute limit.
- Missing fields/quantities, nonnumeric fields, incompatible units, division by zero and overflow produce explicit cell errors. They never become zero. Formula columns do not add new subtotals or change quantity/cost totals.

Each expression is limited to 512 UTF-8 bytes, 128 tokens and 32 parser nesting levels. Names are unique and at most 64 bytes. Calls, assignments, scripts, external references and arbitrary spreadsheet formulas are refused. Invalid configuration preserves the currently active formulas.

Save this configuration as UTF-8 `formulas.json`; the source rows must have a numeric `Waste %` field and quantities in metres:

```json
[
  {
    "name": "Length with allowance",
    "expression": "quantity * (1 + [Waste %] / 100)",
    "quantity_unit": "m",
    "rounding_decimals": 3
  }
]
```

```powershell
sunipdf-cli quantity-table plan.pdf --sort-custom "Waste %" --formulas formulas.json --out calculated.json
```

For a scalar-only expression such as `[Allowance] * 2`, `quantity_unit` can be `null` in JSON. Formula definitions are a separate report configuration, not a new type accepted by `set-quantity-fields`.

## Export and source checks

Choose **Export CSV** or **Export JSON** and a new filename. Export includes all matching rows, not just the visible page or columns. Copying permission is required for file export and CLI JSON output. On-screen inspection can remain available when export is prohibited. Annotation permission controls editing separately.

The export contains a `source_sha256` for the document snapshot, filter/sort/group options, row data and coverage indicators. This is traceability data, not a certificate of physical accuracy or a signature. A pending desktop edit or export is refused if its source snapshot has changed. The CLI requires an exact current row through `--expected-record`; it captures the source for the operation after comparing that row and page. This argument is not an independent expected-file-hash option.

JSON uses schema `sunipdf-quantity-table`, version `1`, with `rows` and `summary`. `complete: true` describes successful bounded table construction; individual monetary totals still have their own `complete` coverage flags. CSV includes `row`, `quantity_subtotal`, `cost_subtotal` and `metadata` records, typed custom-column data and spreadsheet-formula neutralization for text cells. Keep the metadata record when processing the export elsewhere.

Formula exports also include definitions, arithmetic policy and `formula_evaluation_complete`, which is false if any matching row has a formula error. JSON `formula_rows` identifies each source page/index and gives each cell's `value`, `unit` and `error`; failed values are `null`. CSV adds value/unit/error columns for each formula, with the same completeness flag in metadata. A complete table inventory does not imply complete formula evaluation.

## CLI and typed JSON

The commands are `quantity-table`, `set-quantity-fields` and `clear-quantity-fields`. A report without `--out` returns JSON on stdout; CSV requires `--out`. Reports accept the register's filters, `--sort`, `--sort-custom NAME`, `--descending` and `--include-replies`, plus `--group-by none|count_group|discipline|business_status` and `--formulas FILE`. A custom sort overrides the built-in sort. `--page` is a positive physical page number in the CLI.

Save this complete field replacement as UTF-8 `fields.json`:

```json
{
  "unit_price": {
    "amount": "12.50",
    "currency": "CHF",
    "per_unit": "each",
    "rounding_decimals": 2
  },
  "custom": {
    "Package": { "type": "text", "value": "Electrical" },
    "Allowance": { "type": "number", "value": "1.25" },
    "Due date": { "type": "date", "value": "2026-12-31" },
    "Stage": {
      "type": "choice",
      "value": "Reviewed",
      "options": ["Draft", "Reviewed"]
    }
  }
}
```

`amount` and typed number values are decimal strings. `per_unit` uses the unit IDs above. `unit_price: null` clears the price; missing custom keys are removed because `set-quantity-fields` replaces all quantity fields for that row. To clear the entire quantity-field record, use `clear-quantity-fields`; it does not remove the annotation or its measurement/count data.

This PowerShell example adds one manual count marker to an existing `plan.pdf`, extracts the exact returned row, applies the fields above, exports costs, then clears them using a fresh expected row. Use fresh output filenames and stop if any command reports an error:

```powershell
$created = sunipdf-cli count plan.pdf --page 1 --group Devices --point "60,60" --out counted.pdf | ConvertFrom-Json
sunipdf-cli quantity-table counted.pdf --page 1 --group-by count_group --out quantities.json
$table = Get-Content -Raw quantities.json | ConvertFrom-Json
$row = $table.rows | Where-Object { $_.register.page -eq 1 -and $_.register.index -eq $created.index }
$row | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 expected-row.json
sunipdf-cli set-quantity-fields counted.pdf --page 1 --expected-record expected-row.json --fields fields.json --out priced.pdf
sunipdf-cli quantity-table priced.pdf --group-by count_group --format csv --out priced.csv
sunipdf-cli quantity-table priced.pdf --page 1 --out priced.json
$priced = Get-Content -Raw priced.json | ConvertFrom-Json
$updated = $priced.rows | Where-Object { $_.register.page -eq 1 -and $_.register.index -eq $created.index }
$updated | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 updated-row.json
sunipdf-cli clear-quantity-fields priced.pdf --page 1 --expected-record updated-row.json --out cleared.pdf
```

All PDF mutations and explicit exports require new destinations, even with `--overwrite`. Supported encryption is retained in PDF copies. Use `--password-env NAME` or `--password-stdin` for protected inputs. Mutation stdout reports the requested operation, without exporting source PDF contents.

## Bounds and remaining scope

The document register is bounded to 1,000 pages, 128 MiB snapshots, 10,000 annotation entries and 4,096 entries per page, with separate text budgets. Quantity-field JSON and expected-row files are limited to 64 KiB; quantity metadata is limited to 4 MiB per document. A document supports at most 16 typed column names. Names use at most 64 UTF-8 bytes; text values at most 1,024 bytes; choices have 1–32 distinct nonempty options of at most 64 bytes. Invalid dates, duplicate names, inconsistent column types/options, unsupported metadata versions and ambiguous identities are refused.

Decimal inputs allow at most six fractional digits and absolute values up to one trillion; unit prices are nonnegative. The quantity converted for pricing must also fit the supported finite, nonnegative range up to one trillion. Excessive values are refused rather than silently truncated.

AEC-007 remains partial: report arithmetic does not provide formula-derived subtotals, unit-bearing custom fields, currency exchange, a tax engine, cross-document totals, automatic symbol recognition or a full estimating workflow. Verify the plan revision and calibration separately before using its quantities.
