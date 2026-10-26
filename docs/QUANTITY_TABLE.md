# Quantity tables

[0.62 release scope](RELEASE_0.62.0.md) · [Measurements](MEASUREMENTS.md) · [Annotation register](MARKUP_REGISTER.md)

Pricing and typed fields shipped in [0.46.0](RELEASE_0.46.0.md). [That release's evidence](qa/RELEASE_0.46.0.json) does not qualify the new custom sorting and report formulas.

The [subtotal and coverage changes](#formula-subtotals-and-coverage)
and [measurement provenance checks](#measurement-provenance) are included in the published 0.62.0 packages.
See [qualification scope](RELEASE_0.62.0.md) for exact-source, native and package results.

Optional units on numeric custom fields and geometric formula products require 0.70.0 or later.
See [0.70 release scope](RELEASE_0.70.0.md) and [qualification evidence](qa/RELEASE_0.70.0.json); 0.70.0 is published as a stable release.

## Open and filter the table

Open a PDF, then choose **Quantity table** from **Tools → Review & measure**, the measurement panel's **Saved data** section, or inside the annotation register. Finish or cancel another active draft first. Reading the table leaves the PDF unchanged.

The rows show physical page, annotation index, type, contents, quantity, unit, unit price, line cost and available custom columns. Pages start at 1; annotation indices start at 0. An explicit count marker contributes `1 each`. A saved length, polyline, perimeter or area contributes its measured value and unit. An angle is shown in `deg` but is not added or priced. Ordinary annotations have no inferred quantity.

Filter by search text, annotation type, discipline, business status, count group, author or physical page. Page `0` means all pages in the desktop filter. **Subtotal groups** offers **No grouping**, **Count group**, **Discipline** and **Business status**. Subtotals cover all matching rows, including those beyond the current 50-row page. **Previous 50** and **Next 50** navigate table pages; **Filter column names** changes which custom columns are shown without changing row selection or totals. **Refresh** rereads the current document after an edit has been applied or cancelled.

Open **Filters → Custom column** to sort by a typed field. Numbers sort numerically, ISO dates chronologically, and text/choice values lexically with case preserved. Missing values and blank text stay last in both directions; ties use physical page and annotation index. Choose **Built-in order** to return to the ordinary sort. This is separate from the annotation register's custom text fields, which sort as text. Docked or floating tables can stay open while following a row to its annotation.

## Edit price and custom fields

1. Choose **Edit row** on an eligible annotation. Its identity must be unambiguous, its state editable and annotation permission available.
2. Enter **Unit price**, a three-letter uppercase **Currency code**, **Price per unit** and **Cost decimal places** (`0`–`4`). Use a decimal point, not an expression. A blank price is absent; `0` is a supplied zero price. The price basis must match the quantity dimension: for example, metres can be priced per foot, but an area cannot be priced per metre.
3. Under **Custom columns**, choose **Add custom column** and provide its name, type and value. Supported types are **Text**, **Number**, **Date (YYYY-MM-DD)** and **Choice**. Numbers default to **No unit**; select an explicit unit when needed. For a choice, enter the permitted options one per line and enter one of them as the value. A column name must use the same type, unit and choice options everywhere in the PDF. Existing register text fields remain separate from these typed fields.
4. Choose **Apply fields** to commit the row, or **Cancel edit** to discard the draft. **Remove column** removes that value only when the row is applied. An empty text value remains supplied; removing the column makes it absent. Invalid values or a stale target keep the PDF unchanged and preserve the draft for correction or cancellation.

Each successful row edit is one Undo step. Finish or cancel the row editor before using **Undo** or **Redo**. The read-only docked or floating table can stay open; refresh it to inspect restored values. Save the PDF to retain the fields after reopening. These edits do not change measured vertices, calibration, annotation identity or linked comments. A price cannot be attached to an ordinary annotation without a supported quantity or to an angle; typed custom values can be attached to eligible ordinary annotations.

### Numeric fields with declared units

For example, enter a Number named `Depth`, value `0.25`, unit `m`. This is a user-declared
value; it does not create a measured quantity, change the plan scale or replace a unit price.
Choosing a different unit changes the declaration without converting the number. Remove the
unit explicitly before changing the field to a different type.

The same column cannot mix metres, centimetres and numbers without a unit. Use a separate
column name for a different definition. A conflicting edit is refused before the PDF changes.
Sorting remains numeric, and searching can include the displayed unit. Missing fields remain
absent; an explicit zero or negative value keeps its ordinary decimal meaning.

## Quantities, costs and missing values

**Not supplied** means absent data; it is not a zero. No manual quantity override is provided here: edit the actual measurement or count markers in their own tools.

Quantity subtotals retain their original units. `m` and `ft`, or `m2` and `ft2`, remain separate even though compatible price-basis conversion exists. There is no combined length/area/count total. Quantity units are `each`, `mm`, `cm`, `m`, `in`, `ft`, their square and cubic forms (`mm2` through `ft2`, `mm3` through `ft3`), and `deg`. Angles are display-only for ordinary quantity and price totals.

For each priced row, the quantity is converted to the explicit price unit and quantized to six decimal places. Decimal unit price multiplication then uses checked integer arithmetic. Line cost is rounded to the selected `0`–`4` decimals, half away from zero; subtotals sum these rounded line costs. Monetary groups remain separate by currency code and rounding precision. A currency code is a declaration, not a checked exchange-rate lookup.

**Filtered subtotals** reports missing prices and partial cost coverage. A monetary subtotal is complete only when every row in that quantity group contributes to that same currency/precision subtotal. Consequently, mixed currencies can produce partial subtotals even when every row has a price. Ordinary unquantified rows and angles are excluded from additive totals. Version 1 `quantity_rows` includes angles; `unquantified_rows` counts only rows without a quantity. The screen shows at most 30 subtotal groups; the export contains the complete filtered summary. Apply filters to inspect additional groups.

## Report formula columns

Open **Custom columns → Formula columns**, enter a unique column name and expression, select the required quantity unit and `0`–`6` result decimal places, then choose **Add and preview**. Up to eight columns can be active. Formula configuration stays in the current tab and does not modify PDF fields, quantities or prices. Under **Reuse formula configuration**, **Copy configuration** copies its JSON; paste an array and choose **Load configuration** to reuse it. Keep a copy for another tab or session.

Expressions accept decimal constants, unary signs, `+ - * /`, parentheses, `quantity` and exact case-sensitive numeric field references such as `[Waste %]`. Only typed **Number** fields participate; register text fields and other formula results are not inputs. For example, `quantity * (1 + [Waste %] / 100)` applies a dimensionless allowance to each row's actual quantity.

- `quantity_unit` constrains the input token `quantity`, not the computed result unit. `quantity` requires the exact declared unit. SuniPDF 0.91 adds explicit `convert(expression, unit)` after this source check; the existing price-basis conversion is separate.
- Numeric fields use their explicitly declared unit; fields with **No unit** and constants are dimensionless. Addition/subtraction require identical units. A scalar can multiply a unit-bearing value, and division by a scalar retains the unit. Equal-unit division produces a dimensionless result.
- Products and quotients within one exact geometric family support length, area and volume: `m * m` gives `m2`, `m2 * m` gives `m3`, and `m3 / m` gives `m2`. The same rules apply separately to `mm`, `cm`, `in` and `ft`. Mixed families such as `m * cm`, inverse units and powers above three are refused. Counts and angles do not implicitly become dimensionless; mass and density units are unsupported.
- Arithmetic uses checked decimal values with six fractional places. Multiplication/division round half away from zero at each operation; the final result uses the selected `0`–`6` decimal places with the same rule. Intermediate values must remain within the supported one-trillion absolute limit.
- Missing fields/quantities, nonnumeric fields, incompatible units, division by zero and overflow produce explicit cell errors. They never become zero. Formula subtotals are separate from the existing quantity/cost totals.

Each expression is limited to 512 UTF-8 bytes, 128 tokens and 32 parser nesting levels. Names are unique and at most 64 bytes. The only supported function is the explicit unit conversion described below. Arbitrary calls, assignments, scripts, external references and spreadsheet formulas are refused. Invalid configuration preserves the currently active formulas.

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

`quantity_unit` constrains the source `quantity` keyword, not the result or every custom field.
With a current `8 m2` area and `Depth = 0.25 m`, `quantity * [Depth]` produces `2 m3`;
set `quantity_unit` to `m2`. A formula using only custom fields can leave it `null`.
For example, `[Length] * [Width]` with `2 m` and `3 m` produces `6 m2`, without
creating a measured annotation quantity. A field in centimetres needs an explicit
conversion before it is combined with metres; formulas never convert it implicitly.

### Explicit unit conversion

SuniPDF 0.91 adds `convert(expression, unit)`. The second argument is a
literal unit: `mm`, `cm`, `m`, `in`, `ft`, or its square/cubic form such as `m2`
or `ft3`. The expression must already carry a length, area or volume unit with
the same dimension. Counts, angles and dimensionless values cannot be converted.

For an area quantity in `m2` and a numeric `Depth` field declared in `cm`, use
`quantity * convert([Depth], m)`. An area of `8 m2` and a depth of `25 cm`
produce `2 m3`. Keep the required source quantity unit set to `m2`. Changing
the declaration of the stored field would change its meaning; conversion leaves
the field and PDF unchanged.

Unit ratios use checked integer arithmetic. Inches and feet use the international
definitions, with one inch equal to `0.0254 m` and one foot equal to `0.3048 m`.
These are exact ratios, as listed in the [NIST conversion guide](https://www.nist.gov/pml/special-publication-811/nist-guide-si-appendix-b-conversion-factors).
Each conversion rounds half away from zero to six decimal places, just like
other formula operations. For example, converting `1 mm3` to `m3` rounds to
zero at this precision; choose a smaller result unit when those values matter.
Repeated conversions can accumulate rounding and need not reproduce the
original value exactly.

Missing values, incompatible dimensions and overflow remain explicit errors.
The source `quantity` still must be current and match `quantity_unit` before
conversion. No measurement calibration, automatic unit mixing or exchange-rate
conversion is performed. Formula totals group converted results by their actual
result unit and retain the ordinary row-error coverage.

## Formula subtotals and coverage

**Formula totals** appear below the ordinary quantity/cost summary.

Each subtotal covers all rows selected by the current filters, including rows beyond the visible
table page. It groups results by formula name, the selected subtotal group, actual result unit
and decimal precision. A dimensionless result has `unit: null`; this is a valid scalar result,
not a missing unit. Distinct units remain separate and do not make otherwise successful
calculation coverage incomplete.

The subtotal sums each row's already-rounded formula result using checked integer arithmetic.
It does not apply the formula to aggregated inputs. For example, two successful row results
rounded to `0.33` contribute `0.66`, even if rounding an unrounded aggregate would give `0.67`.
Each row keeps its own numeric fields and evaluated result dimension. Arithmetic overflow
returns an error rather than wrapping or truncating the subtotal.

Each formula/group reports matching rows, contributing rows, error rows and error counts by
code. `complete` means every matching row evaluated successfully. A partial total includes
only successful results and is labelled incomplete; if every row fails, `totals` is empty,
not a fabricated zero. A genuine successful result of zero still contributes normally.
Angles remain excluded from ordinary additive quantity and price totals, but an eligible
formula can explicitly evaluate a current angle or dimensionless fields on those rows.
Historical measurement quantities produce `unverified_quantity` when a formula uses `quantity`.

JSON includes `summary.formula_groups`. Its schema is version `2` when selected rows contain
unit-bearing custom fields, otherwise version `1`. Each group contains
`formula_name`, `group`, `matched_rows`, `contributing_rows`, `error_rows`, `errors`, `complete`
and `totals`. Each total records `unit`, `rounding_decimals`, decimal-string `amount` and
`contributing_rows`. `formula_subtotal_policy` identifies
`sum_rounded_row_results_by_formula_group_unit_and_precision`. CSV adds `formula_coverage`
and `formula_subtotal` records with the same coverage information. Consumers should use the
record type and column names rather than fixed row positions.

The existing coverage counters retain their version 1 meanings:

| Counter | Meaning |
|---|---|
| `matched_rows` | All rows selected by the filters |
| `quantity_rows` | Rows with a usable `quantity`, including current angles |
| `unquantified_rows` | Rows without a usable `quantity`, including historical measurements |
| `additive_quantity_rows` | Rows eligible for ordinary additive quantity totals |
| `angular_rows` | Rows whose usable quantity is an angle |
| `excluded_quantity_rows` | Rows excluded from ordinary additive totals, including angles |
| `unverified_measurement_rows` | Historical measurement rows: legacy plus stale |
| `legacy_measurement_rows` | Measurement records without verifiable page provenance |
| `stale_measurement_rows` | Measurement records whose page evidence no longer matches |

Thus `quantity_rows + unquantified_rows == matched_rows`, and
`additive_quantity_rows + excluded_quantity_rows == matched_rows`.
Historical measurements count as unquantified and excluded; their separately retained historical
values do not count as usable quantities. Historical angles are included in the historical counts,
not `angular_rows`. These counters describe different coverage dimensions and must not all be summed.

Table refreshes and JSON/CSV exports each evaluate formula cells once and reuse those results
for summaries. This reduces repeated calculation; it is not a measured native UI speedup.
Typed negative numbers and calculated numeric values remain numeric CSV cells. Free-text
cells still receive spreadsheet-formula neutralization, including text that resembles a
negative number. No formula configuration or subtotal is written into the PDF.

## Measurement provenance

New saved measurements use version 2 metadata with bounded evidence for their page: effective
page bounds, user unit, decoded content, reachable resources, initial optional-content visibility
and page transparency state. The inspector and quantity table distinguish three states:

| State | Meaning and permitted use |
|---|---|
| `current` | The stored page evidence matches. The quantity can participate within its ordinary unit and permission limits. |
| `legacy_unverified` | An older measurement has no sufficient page evidence. Its value and points remain historical. |
| `stale` | The stored evidence no longer matches the page. Its value and points remain historical. |

For either historical state, JSON rows retain `measurement_validity` and `historical_quantity`,
while `quantity` and `line_total` are `null`. CSV adds `measurement_validity`, `historical_quantity`
and `historical_unit` columns. The table labels the old value and any stored price as inactive;
neither contributes to ordinary quantity or monetary totals. Custom fields and stored prices can
still be corrected or removed, but those edits do not establish page provenance.

A formula that reads `quantity` returns `unverified_quantity` for a historical measurement.
A formula that uses only valid custom fields can still evaluate on the same row. An
inventory with `complete: true` therefore does not imply that every measurement is current or
that formula or monetary coverage is complete. Ordinary explicit count markers keep their
existing behavior; they do not require measurement provenance.

To make a historical length or area usable, review its retained points against the drawing,
confirm an explicit current page reference, then preview and apply recalibration. Historical
angles must be recreated. Changing only unit, precision, fields or prices cannot make a record
current. [Desktop recalibration steps](USER_GUIDE.en.md#verify-saved-measurement-references)
describe the individual and batch guards. A successful recalibration retains stored fields;
an eligible stored price becomes active again only when a usable quantity is restored.

Measurement edits and recalibration validate any preserved unit price against the proposed
quantity before committing. A change from length to area, for example, cannot retain a price
per metre. A new calibration or geometry that makes the priced quantity or line cost exceed
the supported range is also refused. The failed edit leaves the document and history unchanged;
in a batch, one incompatible or overflowing row rejects the entire operation. Correct or clear
the price explicitly before retrying a change that requires a different price basis. Compatible
unit conversions, such as metres to feet, retain the stored price and its original price unit.

Quantity reports preserve the coverage fields above in schema versions `1` and `2`. The separate
measurement-record JSON is version `2`: non-current records export `value: null` and `label: null`,
with the old result in `historical_value` and `historical_label`. Consumers must check validity
instead of treating a historical number as a current result. These checks establish bounded
page consistency, not survey accuracy or universal revision authentication.

## Export and source checks

Choose **Export CSV** or **Export JSON** and a new filename. Export includes all matching rows, not just the visible page or columns. Copying permission is required for file export and CLI JSON output. On-screen inspection can remain available when export is prohibited. Annotation permission controls editing separately.

The export contains a `source_sha256` for the document snapshot, filter/sort/group options, row data and coverage indicators. This is traceability data, not a certificate of physical accuracy or a signature. A pending desktop edit or export is refused if its source snapshot has changed. The CLI requires an exact current row through `--expected-record`; it captures the source for the operation after comparing that row and page. This argument is not an independent expected-file-hash option.

JSON uses schema `sunipdf-quantity-table`, with `rows` and `summary`. Selected unit-bearing custom fields require version `2`; otherwise version `1` is retained. `complete: true` describes successful bounded table construction; individual monetary totals still have their own `complete` coverage flags. CSV includes `row`, `quantity_subtotal`, `cost_subtotal` and `metadata` records, typed custom-column data and spreadsheet-formula neutralization for text cells. Keep the metadata record when processing the export elsewhere.

CSV gives a unit-bearing column separate `custom:NAME:value` and `custom:NAME:unit` cells,
alongside its type. Numeric cells keep their raw decimal value. Columns without declared units
retain their earlier two-cell type/value layout. PDF reports and workbook previews show the
unit beside the value; JSON and XLSX preserve the full typed field.

PDF quantity-field records and workbook manifests use version `2` when they contain declared
custom-field units, including unresolved baseline entries retained for refresh. Version `1`
is retained without those units. Absent units are omitted from serialized Number fields,
preserving the comparison hashes of older manifests. A version `1` record claiming an actual
custom-field unit is refused. A refresh compares unit changes explicitly and recalculates
current values; it does not convert the old baseline into new totals. Older application
versions cannot interpret the new metadata and must not be used to edit those fields.

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
    "Depth": { "type": "number", "value": "0.25", "unit": "m" },
    "Due date": { "type": "date", "value": "2026-12-31" },
    "Stage": {
      "type": "choice",
      "value": "Reviewed",
      "options": ["Draft", "Reviewed"]
    }
  }
}
```

`amount` and typed number values are decimal strings. `per_unit` and an optional Number `unit` use the unit IDs above. Omitting a Number's unit, or supplying `unit: null`, means no declared unit; it is omitted on serialization. `unit_price: null` clears the price; missing custom keys are removed because `set-quantity-fields` replaces all quantity fields for that row. To clear the entire quantity-field record, use `clear-quantity-fields`; it does not remove the annotation or its measurement/count data.

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

The document register is bounded to 1,000 pages, 128 MiB snapshots, 10,000 annotation entries and 4,096 entries per page, with separate text budgets. Quantity-field JSON and expected-row files are limited to 64 KiB; quantity metadata is limited to 4 MiB per document. A document supports at most 16 typed column names. Names use at most 64 UTF-8 bytes; text values at most 1,024 bytes; choices have 1–32 distinct nonempty options of at most 64 bytes. Invalid dates, duplicate names, inconsistent column types/units/options, unsupported metadata versions and ambiguous identities are refused.

Decimal inputs allow at most six fractional digits and absolute values up to one trillion; unit prices are nonnegative. The quantity converted for pricing must also fit the supported finite, nonnegative range up to one trillion. Excessive values are refused rather than silently truncated.

AEC-007 remains partial. Formula-derived subtotals are delivered in 0.62.0 within the recorded qualification scope. The 0.70 extension adds declared custom-field units and exact geometric products/quotients. Currency exchange, taxes, mass/density or inverse units, cross-document totals, automatic symbol recognition and a full estimating workflow remain outside this slice. Verify the plan revision and calibration separately before using its quantities.
