# Local Office conversion and export scope

SuniPDF writes separate files and never overwrites an existing output. PDF exports use the current in-memory document. Unapplied redaction marks and unfinished inline text edits block desktop export; finish or cancel them first. PDF copy restrictions apply. Office formats do not preserve PDF digital signatures or encryption.

## Office to PDF

Windows conversion uses an already-installed LibreOffice **26.2 or 26.8**, found in the standard Program Files directories. No converter is downloaded or installed by the application. Missing converters have an explicit unavailable state. Older or unexpected versions are refused before opening the document. `soffice.com` uses a fresh `UserInstallation` for each document; existing user Office sessions and profiles are not attached, modified or closed.

Supported inputs are macro-free DOCX, XLSX, PPTX, ODT, ODS and ODP. Legacy binary formats and macro-enabled extensions are refused. Input checks reject macros, embedded objects, formulas, active fields, external relationships, DTDs/entities, ODF external references, disguised image files, traversal, duplicate archive paths and unsupported package parts. This conservative policy can also refuse otherwise valid complex documents. Images are limited to PNG/JPEG. Only local drives with no reparse-point ancestors are accepted.

Each input is limited to 64 MiB, 4,096 archive entries, 256 MiB expanded data, 16 MiB per XML part and 64 MiB XML total. Conversion has a 120-second deadline, a bounded diagnostic file and a 256 MiB output limit. Batches contain 1–128 documents, run sequentially, continue after individual refusals and support cancellation. Cancellation terminates only the launched process tree. The generated PDF is reopened and checked before atomic publication; the source bytes must still match.

Fresh profiles disable macro execution, active content, Python scripting and OLE automation. External content is refused before launch and an unusable local proxy is configured. These are application controls, **not an operating-system network sandbox**. Documents are not uploaded. LibreOffice may substitute missing fonts or change pagination: review each PDF.

| Saved profile | Image policy | Compliance |
| --- | --- | --- |
| Standard | JPEG quality 90, reduce to 150 dpi | Ordinary PDF; no PDF/A or PDF/UA claim |
| Print | Lossless, no image reduction | Ordinary PDF; no PDF/A or PDF/UA claim |

The profile is remembered in application settings; no document contents are saved in that setting.

## PDF to Office

| Format | Implemented result | Exact limits |
| --- | --- | --- |
| Word | Editable visible text, one paragraph per inferred line, 11 pt | Simple single-column, unrotated text only; reflow replaces fonts, indentation, spacing and pagination. Graphics, complex layout, forms, clipping, optional layers, effects, bidirectional text and unknown or pure-white text fill are refused. No semantic table reconstruction. |
| Excel | One sheet per source page; aligned text objects become string cells | Values are always `inlineStr`, including values beginning with `=`, `+`, `-` or `@`. No formula evaluation, type inference, merged cells, original styling or semantic table reconstruction. Ambiguous columns and complex pages are refused. |
| PowerPoint | One 150 dpi PNG per page, fitted without cropping | Preserves pixels at the export resolution, not editable text, vectors or tables. Mixed page sizes receive margins. Maximum 8 million pixels and 8,192 pixels per side. |

All formats are bounded to 1–128 source pages and 128 MiB packages. Editable exports have additional text/object/content limits. Refusal, failure or cancellation leaves the original and existing outputs untouched. Word page count can change after reflow; `source_pages` is not a claim about the resulting Word pagination.

## Validation and measured fidelity

Synthetic tests inspect package XML with an independent XML reader, check literal formula-like cells, compare exported PNG pixels to PDFium rendering, and exercise unsupported content, existing outputs, cancellation and source preservation. Windows input-guard tests cover active and malformed packages. These checks do not by themselves prove a particular Office application's rendering compatibility.

The ignored `office_fidelity_corpus` test measures the existing four hash-pinned NIST/NBS publications using each manifest-selected sample page. It records source hashes and licensing evidence, accepted/refused exports, real text sequence comparisons, image pixel equality and actual converter roundtrips when available. Complex publication outcomes remain separate from a clearly labeled controlled simple-text fixture. When a pinned converter is required in CI, Word, Excel and PowerPoint must each complete a real roundtrip with both profiles. Actual text, page geometry and pixel errors are recorded; unsupported table/font/layout reconstruction is explicit and fidelity percentages are never invented. Reports and downloaded PDFs stay in ignored QA storage and are excluded from packages.

Run the test with `SUNIPDF_REAL_CORPUS` set to the verified corpus directory and `SUNIPDF_OFFICE_REPORT` set to a new ignored JSON path. The parent release verification must record actual results before marking roadmap acceptance complete. No installed LibreOffice was detected on the development host at implementation time; real converter compatibility remains a separate disposable-runner check.

Sources: [LibreOffice command-line parameters](https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html), [PDF filter options](https://help.libreoffice.org/latest/en-GB/text/shared/guide/pdf_params.html?DbPAR=C&System=WIN), [security configuration schema](https://github.com/LibreOffice/core/blob/master/officecfg/registry/schema/org/openoffice/Office/Common.xcs), [proxy configuration schema](https://github.com/LibreOffice/core/blob/master/officecfg/registry/schema/org/openoffice/Inet.xcs), [pinned corpus and licensing](../scripts/regression-corpus.json).

[French notice](OFFICE_CONVERSION.fr.md).
