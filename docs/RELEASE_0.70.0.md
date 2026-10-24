# SuniPDF 0.70.0

Publication: published stable, 2026-10-24T10:44:17Z. Release 422651863, source `3909bfaab7ad0ce1e1b287040187eadf742e3657`.
[Downloads and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.70.0) · [Qualification evidence](qa/RELEASE_0.70.0.json).

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

## Qualification

The local development source passed formatting, strict workspace Clippy and desktop/CLI compilation, with **1,094 tests passed, zero failed and 15 ignored**. The English and French catalogs contain **1,871 matching keys**. The independent units corpus passed **1,992 assertions across 96 commands** (71 development CLI 0.70 commands, four packaged CLI 0.69 compatibility commands and 21 qpdf checks). These results are scoped to the recorded development binaries; they do not qualify a downloaded 0.70 release package.

| Evidence scope | Verified result |
|---|---|
| Publication | 2026-10-24T10:44:17Z, release 422651863, published stable / Latest verified |
| Immutable release source | `3909bfaab7ad0ce1e1b287040187eadf742e3657`, tree `656860a197516ecd1ba0cb5387b5972e091f7d1a`, [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408084), [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408057), [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084420845), [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408173) |
| Exact-source Windows CI | 1,094 passed, zero failed, 15 ignored; [workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408084) |
| Exact-source Linux CI | 1,075 passed, zero failed, 10 ignored; [workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408057) |
| Code scanning | [CodeQL passed](https://github.com/lmdanavanem/SuniPDF/actions/runs/40084408173); zero open alerts at verification |
| Public assets | Eight files: four packages and four SHA256 sidecars. Anonymous downloads, package hashes and matching sidecars verified; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Downloaded-package corpora | 7,019 assertions across 422 recorded commands in nine corpora. Tool identities and distinct current/legacy reader scopes are recorded in [qualification evidence](qa/RELEASE_0.70.0.json); repeated draft/public runs are not additive coverage |
| Portable/MSI qualification | 47 portable upgrades and four guards passed; [qualification evidence](qa/RELEASE_0.70.0.json), 45 MSI baselines passed with 225 installation logs; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Ubuntu installation | Ubuntu 22.04 and 24.04 installation checks passed in clean containers with software rendering; [qualification evidence](qa/RELEASE_0.70.0.json) |
| Rendered-artifact review | Passed for the specified rendered pages from the exact package. Artifact inspection only; no native desktop interaction. See [qualification evidence](qa/RELEASE_0.70.0.json) |
| Native desktop and spreadsheet interaction | Not tested in the current session; no interactive qualification claim |

Upgrade qualification passed for **47 portable baselines plus four guards** and **45 MSI baselines**,
including published 0.69.0 packages. Repeated runs are not additive coverage.

AEC-007, AEC-018 and AEC-019 remain **In progress**. Native desktop interaction, native spreadsheet interoperability and broader reader qualification remain open. Packages remain unsigned; ARM64 hardware, physical printing, human accessibility and private project plans retain their separate limits. No overall performance improvement, universal interoperability or PDF/A/PDF/UA conformance is claimed.

This release does not add arbitrary spreadsheet formulas, mass or density units, automatic unit conversion, authenticated manifests or XLSX import.
