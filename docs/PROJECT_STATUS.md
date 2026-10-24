# Project status

## 0.71.0 status

Published stable; verified 2026-10-24T13:39:38.272401Z. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.71.0) · [Release scope](RELEASE_0.71.0.md) · [qualification evidence](qa/RELEASE_0.71.0.json).

The release adds local visual template matching and reviewed count-marker batches.
Page selection, quarter-turn rotations, similarity, previews, cancellation and resource
limits are explicit. See [Find symbols](VISUAL_SEARCH.md).

## Qualification

| Evidence | Verified result |
|---|---|
| Publication | Stable / Latest verified, 2026-10-24T13:38:02Z; release 422718479 |
| Source | `fe6b6e912152d605e65a5ffb8b106a05007e0a24`, tree `5c8763bcdc4772b9131fad22eaf9428e1c59fb49` |
| Windows | 1,114 passed, zero failed, 15 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40094035265) |
| Linux | 1,095 passed, zero failed, 10 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40094035241) |
| Release and scanning | [Packaging](https://github.com/lmdanavanem/SuniPDF/actions/runs/40094043637) and [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/40094035364) passed; zero open scanning alerts at verification |
| Downloads | Eight assets: four packages and four SHA256 sidecars, verified through anonymous downloads |
| Package regression corpus | 7,409 assertions across 493 recorded commands in ten corpora |
| Updates | 48 portable baselines plus 4 guards; 46 MSI baselines and 230 installation logs |
| Ubuntu installation | Clean-container installation checks on Ubuntu 22.04 and 24.04 with software rendering |
| Visual inspection | Specified rendered report and symbol-search artifacts reviewed; no native desktop interaction |

Exact binary identities, corpus scopes and retained hashes are in [qualification evidence](qa/RELEASE_0.71.0.json).
Development, draft and public repetitions are not additive coverage. The legacy 0.69
reader controls are recorded separately from the current release CLI.

## Remaining work

AEC-011 remains **In progress**: native interaction and broader real-drawing
qualification remain open. Matching uses one physical scale and exact quarter-turns;
similarity is not a probability. Arbitrary angles, automatic scale matching and
cross-file searches are outside this increment.

No entire P1/P2 category is closed. Windows packages remain unsigned; ARM64 hardware,
physical printing, human accessibility, private plans and native spreadsheet checks
remain separate open or deferred gates. No overall performance, universal interoperability,
survey-accuracy or PDF/A/PDF/UA conformance claim is made.

AEC-007, AEC-010, AEC-014, AEC-016, AEC-017, AEC-018 and AEC-019 retain their
remaining criteria. AEC-005, AEC-009 and AEC-015 are Done only for their stated
manual-count, constant-depth/deduction and indexed-plan-set criteria.
See the [backlog](AEC_BACKLOG.md), [roadmap](ROADMAP.md), [known issues](KNOWN_ISSUES.md)
and [1.0 criteria](RELEASE_1.0_CRITERIA.md).

GitHub Projects synchronization has not been verified. English is the default UI;
detailed engine refusals may remain English in French mode.

## Performance and history

No paired 0.71 package performance result is claimed. The bounded 0.69/0.70 CLI
comparison remains in [0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify
0.71 or establish overall GUI performance.

Earlier qualification is retained with [0.70](RELEASE_0.70.0.md),
[0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical status archive](archive/PROJECT_STATUS_PRE_0.60.md).
Historical checks are not added to the current totals.
