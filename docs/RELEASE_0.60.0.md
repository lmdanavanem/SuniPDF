# SuniPDF 0.60.0 — Plan workspace and calibrated measurement workflow

[Français](RELEASE_0.60.0.fr.md) · [Workflow and limits](PLAN_WORKSPACE.md)

[0.60.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.60.0) on 2026-10-23, under `lmdanavanem`.

- Reuse page layout calculations while scrolling and cache unchanged register filters, sorting and quantity totals.
- Read document searches and annotation inventories page by page, report progress and accept cancellation between pages so visible-page requests can be serviced.
- Keep annotation and quantity tables in a resizable bottom dock or floating window; remember placement and height and navigate from a row to its annotation and contextual properties.
- Snap to eligible PDF vector endpoints and straight-segment intersections, with a visible candidate indicator. CropBox, page rotation, object transforms and supported convex clipping are applied; scans do not produce synthetic vector points.
- Enter known distances as decimals or feet/inches, including fractions such as `5' 6 1/2"`, with explicit unit conversion.
- Save named measurement regions with independent scales. Select overlapping regions explicitly; geometry crossing the selected region is refused. Existing annotations retain their own calibration.
- Preview recalibration on explicitly selected pages and records, then apply the entire batch with one Undo step. Stale documents, invalid later pages, unsupported records and signed documents are refused; AES permissions and annotation identities remain intact.
- Add CLI commands for region inventories and guarded replacement, recalibration previews/new copies and vector snap inspection.
- Extend Windows upgrade regressions with the published 0.51.0 portable and MSI packages. Preserve Ubuntu amd64 and Windows package formats.

## Qualification boundary

The new automated cases use synthetic PDFs with known dimensions and the retained, licensed/public regression corpus. They cover numeric geometry, crop/rotation, nested transforms, unsupported clips, expected records, reopen, atomic refusal, Undo and encryption. This is the implemented 0.60 P1 product scope; source-specific test results and package evidence are recorded separately.

Private real-project plans are deferred at the owner's request. Survey accuracy, physical pointing/printing, screen readers, native ARM64 hardware, complete independent-reader interoperability, update authenticity and the remaining [1.0 criteria](RELEASE_1.0_CRITERIA.md) remain open. A synthetic corpus does not close those gates. No frame-time or annotation-filter latency target is claimed without a recorded benchmark.

The batch is bounded to 512 pages, 4096 selected measurements and a 128 MiB document. A page supports at most 64 regions. Snapping contributes real curve endpoints and straight-segment intersections; curved intersections and unsupported non-convex/curved clipping are excluded. Dense neighborhoods may decline snapping. Region/page defaults do not silently recalculate saved annotations. Dimensioned drawing, volumes, deductions and automatic symbol recognition remain separate backlog work.

Windows packages remain unsigned; Windows ARM64 is cross-built. Ubuntu 22.04/24.04 amd64 retains the [documented Linux feature limits](UBUNTU.md). Projects updates remain deferred.

New saved page scales use the resource-aware version-2 guard. A legacy version-1 scale on a page with resource dependencies requires explicit recalibration or removal, because its old digest could not establish whether a resource changed. Supported stale region records can be cleared with a captured-state guard, then recreated; unknown or malformed metadata remains preserved. The CLI exposes this through `clear-measurement-viewports` into a new copy.

## Observed qualification

Source `90691d0bca160a9bfc22752dc2cf4269c5e7b1e4`, PR [#33](https://github.com/lmdanavanem/SuniPDF/pull/33). Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004268835). Windows: 827 passed, 0 failed, 14 ignored. Linux main: 808 passed, 0 failed, 9 ignored; Linux package build: 808 passed, 0 failed, 9 ignored. 1469 EN/FR keys. 44 portable scenarios; 38 MSI baselines / 190 logs. Eight anonymous public downloads verified by SHA256. No open Code Scanning alerts.

| Run | Operation | Workload | Samples | Observed p95 (µs) |
|---|---|---|---:|---:|
| [Windows main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) | Cached layout | 5000 pages | 1000 | 0 |
| [Windows main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) | Register filtering | 10000 annotations | 100 | 12534 |
| [Linux main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) | Cached layout | 5000 pages | 1000 | 0 |
| [Linux main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) | Register filtering | 10000 annotations | 100 | 10608 |
| [Linux release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) | Cached layout | 5000 pages | 1000 | 0 |
| [Linux release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) | Register filtering | 10000 annotations | 100 | 6399 |

Measurements cover bounded operations on known synthetic geometry and the retained licensed/public corpus. Private real-project plans remain deferred at the owner's request. Performance p95 values below measure cached model operations in debug test binaries on CI runners; complete UI frame time, physical hardware and a universal speed target are not established. A recorded 0 µs means the measured duration is below the 1 µs reporting precision, not zero duration: integer microsecond conversion truncates the fraction. No median is recorded. Native UI evidence uses actual PDFium tiles with offscreen DX12 WARP on Windows and installed Ubuntu packages with Xvfb/software Glow. Ubuntu 22.04/24.04 checks cover removal/reinstallation of the same candidate, not previous-version upgrades. Windows packages remain unsigned; Windows ARM64 is cross-built without native execution. Physical printing, accessibility, full interoperability, update authenticity and remaining 1.0 gates stay open. Projects updates remain deferred.

Five Windows scenes render the actual application and PDFium tiles, including the docked register and the 720 × 480 quantity table with its first row visible. [Evidence](qa/RELEASE_0.60.0.json).

## Published files

| File | Bytes | SHA256 |
|---|---:|---|
| [SuniPDF-0.60.0-linux-amd64.deb](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-linux-amd64.deb) | 17505156 | `5a7b5c8df7564230144c0bb27e5d14377b1c42e5aed4d32fe687d5bcf07d9492` |
| [SuniPDF-0.60.0-linux-amd64.deb.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-linux-amd64.deb.sha256) | 97 | `faa8e0219b27f0e26a36b2d8f67b0868a541fc1050341efd1fe201976086266b` |
| [SuniPDF-0.60.0-win-arm64-portable.zip](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-arm64-portable.zip) | 27893500 | `005747ba3fef243e943694a991b2a13af08fb7346f087d8aee3b75e9ebbaa310` |
| [SuniPDF-0.60.0-win-arm64-portable.zip.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-arm64-portable.zip.sha256) | 105 | `02e5dcbd98a915448ae02a53007045e04215eeac48489d7b9ceefea74890e6ed` |
| [SuniPDF-0.60.0-win-x64-portable.zip](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64-portable.zip) | 29647182 | `713cb24f5c06320a068be1c4a15f64d7e94ca051355ce98fe57fbee4be369598` |
| [SuniPDF-0.60.0-win-x64-portable.zip.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64-portable.zip.sha256) | 103 | `951cdee87da2537057f703c7e4ff3ded9b7b6b7210781a0460efe22c60fe6ff4` |
| [SuniPDF-0.60.0-win-x64.msi](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64.msi) | 27525120 | `7bc83b5cae907b178ddf09422fd2a301bbbebdd2bd09898b47cf2f3248190650` |
| [SuniPDF-0.60.0-win-x64.msi.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64.msi.sha256) | 94 | `52e7a4a1d6ecde021de7a35700a370b7930d21056e5b8eb75668248c5f71a5d3` |
