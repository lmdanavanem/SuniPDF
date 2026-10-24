# SuniPDF 0.73.0

Published stable on 2026-10-24T17:30:04Z. [Downloads and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.73.0) · [qualification evidence](qa/RELEASE_0.73.0.json).

## Reviewed links across several PDFs

Open **Tools → Review & measure → Plan links**, then choose **Link multiple PDFs…**.
Select source PDFs from an indexed plan set, choose explicit pages and destinations for each source,
inspect the references and confirm the intended destination occurrence by occurrence. Review the
source and destination previews before saving new sibling copies.

The batch accepts up to 16 sources, 128 selected pages and 4,096 detected references within its
document and JSON limits. All output names and reviews are checked before the first copy is saved.
Only one prepared PDF candidate is retained at a time. The manifest and original PDFs are checked
again before every commit and after the batch.

Each file is saved atomically. Cancellation or failure preserves completed copies and their receipts;
remaining files are reported separately. Existing files are never overwritten. Destinations remain
the original indexed PDFs, including when sources link to each other. Generated copies are not
automatically substituted into the plan set.

The CLI adds `plan-links batch-inspect`, `batch-preview` and `batch-apply`. Existing single-source
commands remain available. [Workflow, limits and examples](PLAN_LINKS.md).

## Clearer Linux window controls

Linux uses round minimize, maximize/restore and close controls with 40-pixel logical click targets,
vector symbols, localized accessible labels and a red close hover. Title dragging, double-click
maximizing and edge/corner resizing use native window commands. Closing uses the application's
existing protection for unsaved changes and active operations.

The welcome and About text now correctly includes Windows and Linux. Windows retains its native
window decoration. Window controls passed native interaction and image review on Ubuntu 22.04 and 24.04 under X11/Openbox/Xvfb with software rendering in the light theme. Dark-theme appearance, Wayland, physical GPUs and Windows interaction were not exercised. This does not qualify the plan-link interface's native workflow.

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

AEC-017 remains **In progress** pending native plan-link interaction and broader interoperability. Batch saves are atomic per file; completed copies remain after a cancellation or failure. Original indexed PDFs remain the link destinations. See [workflow and limits](PLAN_LINKS.md).

Packages remain unsigned. ARM64 hardware, physical printing, human accessibility, private project
plans and native spreadsheet qualification remain separate open gates. No universal PDF/A/PDF/UA
conformance, physical measurement accuracy or complete P1/P2 category is claimed.
Next priority: independent synchronized reader views. [Roadmap](ROADMAP.md).
