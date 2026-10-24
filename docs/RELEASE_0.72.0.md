# SuniPDF 0.72.0

Published stable on 2026-10-24T15:09:46Z; verified 2026-10-24T15:11:57.923546Z. [Downloads and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.72.0) · [qualification evidence](qa/RELEASE_0.72.0.json).

## Review enclosed regions as area measurements

Open **Tools → Review & measure → Region fill**, select a point inside an enclosed
region and add explicit closure segments where required. Resolution and grayscale
threshold control the detected boundaries. Fit, zoom and pan help work on larger pages.

The result retains the actual raster contour and enclosed holes. Review its calibrated
area preview, deductions and selected scale before creating one editable measurement.
One Undo removes that operation. Existing page, named-region and confirmed temporary
scales are reused; ambiguous or stale references are refused.

Processing advances through bounded, cancellable phases. Leaks to the page edge,
ambiguous topology, excessive geometry and resource limits remain explicit refusals.
Incomplete results cannot be applied. There is no hidden gap closure, smoothing or
replacement of the region by its bounding rectangle.

The CLI provides inspection, a strict review file, a PNG measurement preview and
new-copy output. It binds the saved source and inspection results before application.
Historical generation metadata retains the seed, closures, geometry, raster settings
and measurement inputs separately from the existing measurement record.

[Workflow, limits and CLI examples](REGION_FILL.md).

## Closing with unsaved plan work

The window-close action now waits for an active plan-link copy operation and retains
its completion result. This also protects a clean document whose external copy is
being saved. An automated viewport-close regression covers the cancellation request
and the subsequent committed-copy result; native interaction remains separately untested.

Plan sets use the same quit checks when closing the window. Unsaved manifests and
unapplied metadata remain available for review. An active manifest save finishes
before quitting; a failed save keeps the draft open for retry.

Printed text, hatching and other page content can create boundaries or holes. Manual closures affect detection only; annotation and form-widget appearances are excluded. Resolution and threshold change the approximation. Changing the source or settings requires a fresh review.

## Qualification scope

| Evidence scope | Verified result |
|---|---|
| Publication | Stable / Latest verified, published 2026-10-24T15:09:46Z; release 422778152 |
| Immutable source | `23c3f1846f8870b34865266a35ba6bd25753128b`, tree `95fc9123c2b9d93af5f0f4bf4d997912bc1bb1d9` |
| Windows | 1,135 passed, zero failed, 15 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40102458058) |
| Linux | 1,116 passed, zero failed, 10 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40102458013) |
| Packaging and scanning | [Release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/40102464218) and [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/40102458663) passed; zero open scanning alerts at verification |
| Public assets and retention | Eight anonymous downloads: four packages and four sidecars; retained copies match verified asset hashes |
| Downloaded-package corpus | 8,432 assertions / 590 recorded commands / eleven corpora |
| Updates | 49 executed portable baselines plus 4 guards; 47 MSI baselines / 235 retained installation logs |
| Rendered artifacts | 7 report pages, 3 symbol-search images and 5 region-fill images reviewed; no native interaction |
| Native region-fill interaction | Not qualified in this session |

The [qualification evidence](qa/RELEASE_0.72.0.json) binds workflow, binary and report identities. Its published JSON SHA256 is
`06ee7d01c14e09b6df95031392f62584ec1dad0e7b110af108facfcfab5d70cf`. Development, draft and public repetitions are separate checks,
not additive coverage. The 510 current-release CLI commands,
4 legacy-reader commands, 74 qpdf commands
and 2 recorded validation helpers retain their separate scopes;
subprocesses inside the helpers are not added to these recorded-command totals.

AEC-012 remains **In progress** pending native interaction and broader drawing/interoperability
qualification. Raster resolution and threshold affect the geometry; printed text and hatching
can create deductions. Inspect every boundary and hole. Calibration does not establish physical accuracy.

No complete P1/P2 category, universal interoperability or general PDF/A/PDF/UA conformance is claimed.
Packages remain unsigned; ARM64 hardware, physical printing, human accessibility and native spreadsheet
checks retain their separate open gates. Private project-plan qualification remains deferred at the
owner's request. Repository documentation does not establish GitHub Projects synchronization.

The earlier [0.71 Find symbols workflow](VISUAL_SEARCH.md) retains its
[release scope and limits](RELEASE_0.71.0.md). Next priorities are AEC-017 multi-source
plan-link batches, then independent synchronized reader views.
