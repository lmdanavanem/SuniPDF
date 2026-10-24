# SuniPDF 0.69.0

Candidate qualification is in progress. The published download remains
[0.67.0](RELEASE_0.67.0.md). This candidate combines issue reports and quantity
workbooks; the intermediate 0.68 development build is not a separate release.

## Reports from the current selection

Create a report from the annotation register or quantity table while keeping that
table's search, filters and ordering. PDF, CSV and JSON use the same selected rows,
quantities and formula calculations. Review the actual paginated PDF before export.

The PDF leads with readable summaries and separates units, currencies, missing
prices and calculation errors. Issue groups, supported annotation style legends
and selected annotation details follow. Full source identities and settings remain
available in a technical appendix. Source-scope location coverage is distinguished
from the filtered rows.

Exports create new files and preserve the source PDF. Copy restrictions apply;
changed sources, cancellation and destination collisions prevent publication.
The report is an unencrypted derivative and does not inherit digital signatures.
See [issue and quantity reports](ISSUE_REPORTS.md) for font and coverage limits.

## Quantity workbooks and reviewed refresh

Export local XLSX workbooks with rows, totals, changes and provenance. Calculations
use the shared quantity engine, with exact decimal values retained when ordinary
spreadsheet numbers would lose precision. Annotation text stays text; the workbook
does not execute formulas or macros.

Save a separate refresh manifest to compare a later PDF revision. Unique annotation
identities preserve matching across page changes. Missing, ambiguous and
unquantified entries remain explicit, including across later refreshes. New
annotations are excluded until deliberately included. Review the comparison before
exporting a new workbook.

The manifest is an unauthenticated comparison baseline. Current values are
recalculated from the PDF. Arbitrary XLSX imports, automatic synchronization and
changes made manually in another spreadsheet are outside this workflow.
See [quantity workbooks](QUANTITY_WORKBOOKS.md).

## Qualification status

Final source, workspace tests, independent output checks and package qualification
remain in progress. Results from earlier development builds do not qualify the
changed report layout. The final evidence must distinguish local checks, exact-source
CI, downloaded packages and native interaction.

Interactive Windows and native spreadsheet validation remain unavailable in the
current test session. Structural PDF/OOXML checks and automated layout tests do not
replace those interactions. The related backlog cards retain their open criteria.

The upgrade regression matrices include the published 0.67.0 packages: 46 portable
baselines plus four guards, and 44 MSI baselines. These are configured checks, not
successful results until the release workflows complete.

Packages remain unsigned. ARM64 execution, physical printing, human accessibility
and private project plans retain their separate qualification limits. No universal
interoperability or PDF/A/PDF/UA conformance is claimed for these exports.
