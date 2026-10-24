# SuniPDF 0.70.0

Candidate qualification is in progress. The published download remains
[0.69.0](RELEASE_0.69.0.md).

## Units for numeric custom fields

Assign an explicit unit to a **Number** field in the quantity table. The default
is **No unit**. Fields with the same column name must use a consistent type and
unit throughout the document. Changing a unit declares what the entered number
means; it does not convert the value or calibrate a measurement.

The table, formula previews, reports and workbook exports retain those units.
Decimal calculations can combine lengths into areas and volumes within one exact
unit family: `2 m × 3 m = 6 m2`, then `6 m2 × 0.25 m = 1.5 m3`.
Mixed families, inverse dimensions and unsupported powers produce explicit errors.
Missing values remain distinct from zero. Custom fields do not create measured
annotation quantities or alter existing prices.

See [quantity tables](QUANTITY_TABLE.md) for the supported units, arithmetic limits
and examples.

## Revision history and compatibility

Workbook comparisons retain old and new field values and units. Version 2 XLSX
exports add **Before custom fields** and **After custom fields** to the Changes
sheet as inert text. Missing sides of a comparison stay empty. Existing version 1
exports keep their original columns.

Unit-bearing PDF fields and export manifests use version 2 metadata. Existing
unitless version 1 data remains readable, with its comparison identities preserved.
Older readers reject unsupported version 2 data rather than silently dropping
its units. Review unresolved and unquantified entries before using a refreshed
workbook; their presence does not establish complete quantity coverage.

## Qualification status

Workspace tests, independent Decimal-based output checks, package generation and
upgrade qualification remain in progress. Results from another source revision
do not qualify this candidate.

Interactive desktop and native spreadsheet validation remain unavailable in the
current session. Automated UI layout tests, PDF renders and OOXML parsing cover
different checks and do not replace those interactions. Packages remain unsigned;
physical ARM64 execution, printing, human accessibility and private project plans
retain their separate qualification limits.

This release does not add arbitrary spreadsheet formulas, mass or density units,
automatic unit conversion, authenticated manifests or XLSX import.
