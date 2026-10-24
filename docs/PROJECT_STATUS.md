# Project status

## Candidate under qualification

[0.73.0](RELEASE_0.73.0.md) implements reviewed links across several source PDFs and round Linux
window controls. Source tests, package qualification and native Ubuntu checks are in progress.
The stable release below remains the published reference until this candidate is qualified.

## 0.72.0 status

Published stable; verified 2026-10-24T15:11:57.923546Z. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.72.0) · [Release scope](RELEASE_0.72.0.md) · [qualification evidence](qa/RELEASE_0.72.0.json).

Region fill traces one page from an explicit interior point and optional manual closures. Review the
actual outer contour and holes, choose an existing calibration, then preview and explicitly accept one
editable area measurement. Changed source or settings invalidate the candidate; incomplete results
cannot be applied. [Workflow and limits](REGION_FILL.md).

## Qualification

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

## Remaining work

AEC-012 remains **In progress** pending native interaction and broader drawing/interoperability
qualification. Raster resolution and threshold affect the geometry; printed text and hatching
can create deductions. Inspect every boundary and hole. Calibration does not establish physical accuracy.

No complete P1/P2 category, universal interoperability or general PDF/A/PDF/UA conformance is claimed.
Packages remain unsigned; ARM64 hardware, physical printing, human accessibility and native spreadsheet
checks retain their separate open gates. Private project-plan qualification remains deferred at the
owner's request. Repository documentation does not establish GitHub Projects synchronization.

The next implementation priorities are multi-source plan-link batches (AEC-017), followed by
independent synchronized reader views. Full criteria remain in the [backlog](AEC_BACKLOG.md),
[roadmap](ROADMAP.md), [known issues](KNOWN_ISSUES.md) and [1.0 criteria](RELEASE_1.0_CRITERIA.md).

The [0.71 Find symbols workflow](VISUAL_SEARCH.md) and [release scope](RELEASE_0.71.0.md) remain
available; AEC-011 retains its own open native and broader-drawing qualification. Other card statuses
are unchanged by this increment. English remains the default UI; detailed engine refusals can remain English in French mode.

## Performance and history

No overall performance claim is made. Cooperative processing does not make page loading or a native
render preemptible. The separately recorded 0.69/0.70 CLI comparison keeps its original binary,
machine and workload scope in [0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify 0.72.

Earlier qualification remains with [0.71](RELEASE_0.71.0.md), [0.70](RELEASE_0.70.0.md),
[0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical checks are not added to current totals.
