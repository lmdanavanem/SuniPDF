# Project status

## Published 0.77.0

[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.77.0) | [Release notes](RELEASE_0.77.0.md) | [Qualification evidence](qa/RELEASE_0.77.0.json).

Saved plan links can show a verified destination preview before opening. The preview displays the drawing and sheet metadata; opening rechecks the saved inputs. Manifest paths can be entered directly or selected with the file browser.

The tagged source passed all 11 release jobs. Windows qualification includes
13 offscreen interface scenes, 54 portable upgrade baselines plus
4 guards, and 52 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 21 installed runtime cases, including 5 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-25T01:56:30.631866+00:00.

Visual review covers 11 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.77.0.json).

Native plan-link navigation uses synthetic PDFs prepared by the installed CLI.
It covers preview/cancel, changed or missing input refusals and the correct
physical destination page. Desktop link creation, desktop link saving,
dirty-destination interaction and broader external-reader interoperability
remain open. These results do not establish complete AEC-017 acceptance.

Windows packages remain unsigned, and Windows ARM64 is cross-built without
native hardware execution. The Apple Silicon app is ad hoc signed, not
notarized; macOS 12 is a binary-checked deployment minimum. Physical Macs,
interactive Finder opening and clipboard interoperability remain unqualified.
Ubuntu updater-helper checks do not establish graphical authorization or a
complete desktop updater interaction. Physical GPUs, Wayland, printers and
human accessibility remain separate gates.

## Remaining work

AEC-017 remains **In progress**. Native navigation is qualified within the
synthetic scope above; desktop link creation/saving, broader batch interaction
and external-reader interoperability remain open. Batch saves are atomic per
file, completed copies remain after cancellation or failure, and original
indexed PDFs remain the link destinations. See [workflow and limits](PLAN_LINKS.md).

AEC-011 visual search and AEC-012 region fill retain their open native and
broader-drawing criteria. Other backlog statuses remain unchanged. Independent
update signing, publisher certificates, native spreadsheets, private project
plans and applicable human accessibility checks retain their separate gates.
No complete P1/P2 category, universal interoperability, general PDF/A/PDF/UA
conformance or overall performance claim is made.

[Backlog](AEC_BACKLOG.md) · [Roadmap](ROADMAP.md) · [Known issues](KNOWN_ISSUES.md) · [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Repository documentation does not establish GitHub Projects synchronization. English remains the default UI.

## History

[0.76.0 qualification](RELEASE_0.76.0.md) retains its original source, counts and limits.

[0.75 qualification](RELEASE_0.75.0.md) retains the original Apple Silicon release evidence.

[0.74 qualification](RELEASE_0.74.0.md) retains the original theme and Ubuntu-update evidence.

[0.73 qualification](RELEASE_0.73.0.md) retains its original counts, source and limits.

Earlier qualification stays with [0.72](RELEASE_0.72.0.md), [0.71](RELEASE_0.71.0.md),
[0.70](RELEASE_0.70.0.md), [0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical counts are not added to this release.
The 0.69/0.70 CLI performance comparison retains its original binary, machine and workload scope in
[0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify later releases.
