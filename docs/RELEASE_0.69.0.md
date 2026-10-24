# SuniPDF 0.69.0

Published on 24 October 2026 as the stable Latest release, with Windows x64 MSI,
Windows x64/ARM64 portable ZIP and Ubuntu amd64 DEB packages.
[Downloads and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.69.0).
This release combines issue reports and quantity workbooks; the intermediate
0.68 development build is not a separate release.

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

## Qualification

Source `d09621b54d2ceb0addaa07e4b7115db009ef7dd7` passed exact-source Windows and
Linux CI, strict Clippy, formatting and CodeQL. Windows recorded 1,083 passed and
15 ignored tests; Linux recorded 1,064 passed and 10 ignored, with zero failures.
Both localization catalogs contain 1,868 matching keys.

Eight independent corpora passed against the downloaded package: 5,027 assertions
across 279 CLI invocations, 45 qpdf checks and two validators. Repeating the corpora
on the public download is recorded separately, without adding the repetitions to
coverage totals. Five pages rendered from packaged report outputs were visually
reviewed for summaries, partial coverage, technical provenance, CJK text and style
legends. This is artifact review, not native desktop interaction.

All four packages and their four SHA256 sidecars were verified. See the
[qualification evidence](qa/RELEASE_0.69.0.json) for workflow links, package hashes,
tool identities and the separate evidence scopes.

Interactive Windows and native spreadsheet validation remain unavailable in the
current test session. Structural PDF/OOXML checks and automated layout tests do not
replace those interactions. The related backlog cards retain their open criteria.

The upgrade regression matrices passed for 46 portable baselines plus four guards
and 44 MSI baselines, including the published 0.67.0 packages. The MSI checks
produced 220 installation logs. Ubuntu 22.04 and 24.04 package installation checks
also passed.

Packages remain unsigned. ARM64 execution, physical printing, human accessibility
and private project plans retain their separate qualification limits. No universal
interoperability or PDF/A/PDF/UA conformance is claimed for these exports.
