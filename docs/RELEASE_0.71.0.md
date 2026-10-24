# SuniPDF 0.71.0

Published stable on 2026-10-24T13:38:02Z. [Downloads and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.71.0) · [qualification evidence](qa/RELEASE_0.71.0.json).

## Find symbols and review counts

Open **Tools → Review & measure → Find symbols**, select one symbol on the page,
and search selected pages using a fixed resolution, similarity threshold and
optional quarter-turn rotations. Fit, zoom and pan controls help select the template
on larger drawings. The search advances between foreground work and exposes
cancellation, page coverage and resource limits.

Candidate previews include existing annotations. Matches start unselected; only a
reviewed selection becomes count markers, in one undoable batch. Existing counts
are retained, and repeated searches do not automatically deduplicate them. Editing
the source invalidates the review. Saved counts remain available in the annotation
register and quantity table.

The command-line workflow creates a review file with no accepted candidates. Applying
that review checks the saved source and repeats the inspection before writing a new
PDF. Unknown selections, changed sources and altered results are refused. Accepting
partial results requires an explicit choice and does not change their incomplete status.

See [visual symbol search](VISUAL_SEARCH.md) for the desktop controls, CLI examples,
resource limits and the distinction between proposed matches and accepted counts.

## Scope and qualification

This is same-scale raster matching with exact quarter-turns. Similarity is not a
probability or a recognition guarantee. Noise, closely related symbols, overlap and
changed backgrounds can cause missed or false matches. Automatic scale matching,
arbitrary angles and searching across separate files remain outside this increment.

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

AEC-011 remains **In progress**: native interaction and broader real-drawing
qualification remain open. Matching uses one physical scale and exact quarter-turns;
similarity is not a probability. Arbitrary angles, automatic scale matching and
cross-file searches are outside this increment.

No entire P1/P2 category is closed. Windows packages remain unsigned; ARM64 hardware,
physical printing, human accessibility, private plans and native spreadsheet checks
remain separate open or deferred gates. No overall performance, universal interoperability,
survey-accuracy or PDF/A/PDF/UA conformance claim is made.
