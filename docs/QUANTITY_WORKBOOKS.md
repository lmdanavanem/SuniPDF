# Quantity workbooks

The 0.69 candidate adds local XLSX quantity snapshots and reviewed refreshes.
Qualification is in progress; this guide does not describe a published package.
See [release qualification](RELEASE_0.69.0.md).

## Create a snapshot

Open the quantity table, choose **Reports and spreadsheets**, then **Spreadsheet…**.
You can also find **Quantity workbooks** in the command palette. The snapshot uses the table's filters, sorting,
grouping and formulas. Review the selected rows, totals, units, coverage warnings
and source identity before exporting a new XLSX file.

The workbook contains calculated values. It does not run Excel formulas or macros.
Text from annotations remains text, including strings beginning with formula
characters. Exact decimal values are retained when a spreadsheet number would
lose precision. Missing values and calculation errors stay visible.

Save a separate refresh manifest if you want to compare these quantities with a
later PDF revision. Keep it alongside the workbook. The manifest stores the
configuration, annotation identities, baseline values and source fingerprints.
It does not contain a password or an absolute document path.

## Review a later revision

Open the later PDF and load the saved manifest in **Quantity workbooks**. Review the
comparison before exporting a new workbook. Matching uses unique annotation IDs,
so a changed page order does not turn a different annotation into a match.

The comparison distinguishes changed, unchanged, missing, ambiguous and
unquantified entries. New identities are excluded by default; including them is
an explicit choice. Unresolved baseline entries remain available for later
refreshes. The current values and totals are recalculated from the open PDF.

The imported manifest is comparison evidence, not an authenticated certificate of
origin. Verify that it belongs to the revision you intend to compare. A serialized
document snapshot hash and a saved PDF file hash identify different byte sequences;
the interface keeps those identities separate.

## Preservation and limits

Export creates a new file and refuses an existing destination. The source PDF and
previous workbook are preserved. Changes made manually in an external spreadsheet
are not imported into SuniPDF or carried into the new workbook. Refresh accepts a
SuniPDF manifest rather than an arbitrary XLSX file.

Copy permissions apply. A derived workbook does not inherit source encryption or
its digital signatures. Changes to the document or saved source invalidate the
prepared export. Cancellation and oversized inputs are refused explicitly.

The development bounds are 10,000 rows, a 32 MiB manifest and a 128 MiB OOXML
package. Incomplete coverage or unresolved entries do not establish complete
quantities. Native spreadsheet interoperability and final package qualification
remain pending.

See [quantity tables](QUANTITY_TABLE.md), [issue reports](ISSUE_REPORTS.md) and
[the backlog](AEC_BACKLOG.md).
