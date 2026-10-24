# Project status

## 0.73.0 status

Published stable; verified 2026-10-24T17:32:54.306206Z. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.73.0) · [Release scope](RELEASE_0.73.0.md) · [qualification evidence](qa/RELEASE_0.73.0.json).

Review links across several source PDFs, explicitly confirm each destination and preview, then save new
copies with per-file receipts. Linux has round window controls with larger click targets and unsaved-work protection.

## Qualification

| Evidence scope | Verified result |
|---|---|
| Publication | Stable / Latest verified 2026-10-24T17:32:54.306206Z; published 2026-10-24T17:30:04Z; release 422857302 |
| Source | `ecb292f23cf360a2330879ddb6e5ceb4fa59024b`, tree `566fe9328645e1b072f18bb08b74a07930a11211` |
| Windows | 1,159 passed, zero failed, 15 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40113402153) |
| Linux | 1,140 passed, zero failed, 10 ignored; [CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/40113402204) |
| Packaging and scanning | [Release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40113416651) and [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/40113402451) passed; zero open scanning alerts at verification |
| Public assets | Eight anonymous downloads: four packages and four SHA256 files; retained copies match |
| Package corpus | 9,827 assertions / 668 recorded commands / twelve corpora |
| Updates | 50 portable baselines plus 4 guards; 48 MSI baselines / 240 retained installation logs |
| Native window controls | Ubuntu 22.04 / 24.04 X11, light theme: minimize, maximize/restore, title double-click, drag, resize and unsaved-close protection; image review passed |

The [qualification evidence](qa/RELEASE_0.73.0.json) records exact workflow, package and report identities. Published JSON SHA256:
`afc81f65b07ab3baa9288fcb34ad39bf2a017196067760616893b4bcba4a64b4`. Development, draft and public repetitions are not additive coverage.
Recorded commands comprise 566 current CLI, 4 legacy CLI,
96 qpdf and 2 validation helpers; internal helper subprocesses are excluded.

Window controls passed native interaction and image review on Ubuntu 22.04 and 24.04 under X11/Openbox/Xvfb with software rendering in the light theme. Dark-theme appearance, Wayland, physical GPUs and Windows interaction were not exercised. This does not qualify the plan-link interface's native workflow.

## Remaining work

AEC-017 remains **In progress** pending native plan-link interaction and broader interoperability. Batch saves are atomic per file; completed copies remain after a cancellation or failure. Original indexed PDFs remain the link destinations. See [workflow and limits](PLAN_LINKS.md).

Next development priority: independent synchronized reader views. AEC-011 visual search and AEC-012 region fill
retain their open native and broader-drawing qualification. Other backlog statuses are unchanged.
Packages remain unsigned; ARM64 hardware, physical printing, human accessibility, native spreadsheets and
private project plans retain their separate gates. No complete P1/P2 category, universal interoperability,
general PDF/A/PDF/UA conformance or overall performance claim is made.

[Backlog](AEC_BACKLOG.md) · [Roadmap](ROADMAP.md) · [Known issues](KNOWN_ISSUES.md) · [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Repository documentation does not establish GitHub Projects synchronization. English remains the default UI.

## History

Earlier qualification stays with [0.72](RELEASE_0.72.0.md), [0.71](RELEASE_0.71.0.md),
[0.70](RELEASE_0.70.0.md), [0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical counts are not added to this release.
The 0.69/0.70 CLI performance comparison retains its original binary, machine and workload scope in
[0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify 0.73.
