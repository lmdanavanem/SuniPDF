# Issue and quantity reports

SuniPDF 0.69 adds reports derived from the current annotation register and quantity
table. See the [release notes](RELEASE_0.69.0.md) and
[qualification evidence](qa/RELEASE_0.69.0.json).

Native interaction with this workflow remains untested. Automated output checks
do not establish native desktop or universal reader interoperability; AEC-018 stays In progress.

## Review before export

Choose **Create report…** in the annotation register, or **Reports and spreadsheets**
then **Create report…** in the quantity table. You can also find **Issue and quantity
reports** in the command palette. The register entry retains
the register's current search, filters and row order, including its text custom-field
sorting. It does not add rows that match only a typed quantity field. The quantity
table entry retains that table's typed-field search and sorting, grouping and formulas.
Opening the command while either table is visible uses that table's settings;
otherwise it starts with the default quantity selection.

The report records its selection source as `register` or `quantities`. Both modes
use the same quantity and formula calculations after selecting the rows. The selected
rows, their order and calculated totals remain consistent across PDF, CSV and JSON.
The preview displays the generated PDF, including its page breaks, before export.
Changing the document or report settings invalidates the prepared report. External
source-file changes are checked when previewing or exporting, without a file watcher.

The PDF starts with a readable report summary: the selected annotations, calculated
quantities, costs and missing values. Units and currencies keep separate totals.
Issue groups, style legends and annotation details follow. A technical appendix on
separate pages retains the full source identities and calculation settings.

Location coverage for the source scope is distinguished from the filtered selection.
Its counts can include annotations excluded by other filters; they are not a
percentage of the selected rows.

Legend entries identify the selected annotations and their actual supported styles.
Unrecognized appearances remain explicit instead of receiving an invented symbol.
Issue summaries group the selected annotations by business status, review status and
discipline. Empty, missing, unsupported and stale values retain their existing
coverage and error information; a partial report does not establish complete totals.

The overall `complete` flag checks the applicable measurement, zone and formula
conditions. It does not imply that every annotation has a quantity, a recognized
legend style or a price. Check quantity coverage, each quantity group's
`missing_prices` count and each currency/precision subtotal's `complete` flag
before using the report as an estimate.

Export creates a new file. Existing destinations and the source document cannot be
overwritten. Generating a report does not edit the source PDF or add an Undo step.
Reopen the exported PDF in a reader to inspect the final report independently.

## Command line

Use `--selection register` to follow the register's search and text custom-field
ordering, or `--selection quantities` for typed-field quantity selection. The default
is `quantities`. Use the same selection, filters, sorting and formula options for
each output format.

```console
sunipdf-cli issue-report inspect plans.pdf --selection register --query "Level 1" --sort-custom Room
sunipdf-cli issue-report preview plans.pdf --selection register --query "Level 1" --sort-custom Room --preview-page 1 --out report-page-1.png
sunipdf-cli issue-report export plans.pdf --selection register --query "Level 1" --sort-custom Room --format pdf --out report.pdf
```

`inspect` prepares the model and real PDF pagination without writing an output.
`preview` renders a report page, numbered from 1 independently of the source `--page`
filter. `export` also accepts `--format csv` and `--format json`. An incomplete
report returns its coverage information and exit code 2; it must not be treated as
a complete quantity total. `--expected-source-sha256` binds the operation to the saved
PDF's bytes. Protected inputs use the global `--password-env` or `--password-stdin`
options; passwords are not report fields.

## Source identity and protection

The report records a serialized document snapshot SHA256. This identifies the
snapshot used for generation; it is not the hash of the original saved PDF bytes.
Separate openings can serialize different snapshots of the same saved PDF.
When a saved source is captured, its separate file SHA256 is recorded and rechecked
before publication. Absolute source paths and passwords are excluded from the report.
An edited, unsaved document uses its snapshot identity and does not claim that its
report represents the last saved file bytes.

Source copy permissions apply. A signed document can be read for a report when
copying is permitted; its original bytes and signature remain unchanged. The derived
report is a separate, unencrypted output and does not inherit the source signature.

## Bounds and current limits

The source limit is 128 MiB. Reports are limited to 250 PDF pages, 32 MiB per output
format, four MiB of report text and 256 issue/legend groups. Oversized work is refused
instead of silently truncating rows. Desktop previews use 1,000 pixels on their
longest side; the core and CLI permit up to 1,536. Cancellation and changes detected
before export prevent output.

Embedded Ubuntu Light and Noto Sans JP provide predictable font sources. Text that
requires unsupported glyphs or complex shaping is refused explicitly. This workflow
prepares its PDF preview before exporting any of the three formats, so that refusal
also prevents its CSV/JSON exports. The register and quantity table's direct data
exports remain available separately. These reports do not claim PDF/A or PDF/UA
conformance. [Quantity workbooks](QUANTITY_WORKBOOKS.md) provide a separate reviewed
spreadsheet refresh workflow. Editable report templates remain a backlog item.

See [quantity tables](QUANTITY_TABLE.md), [rooms and zones](PLAN_ZONES.md),
[third-party notices](THIRD_PARTY_NOTICES.md) and [the backlog](AEC_BACKLOG.md).
