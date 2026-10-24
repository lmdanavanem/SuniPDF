# Project status

## Published 0.76.0

[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.76.0) | [Release notes](RELEASE_0.76.0.md) | [Qualification evidence](qa/RELEASE_0.76.0.json).

Independent reader views keep two locations in one PDF visible, with separate
navigation, zoom and scroll positions. Optional synchronization follows the active
pane. Reading and editing tools use one active context, with a shared document and
Undo/Redo history. Pane locations survive tab switches during the session; the
workspace is not stored in the PDF or restored after restarting the application.

The tagged source passed all 11 release jobs. Windows qualification includes 13
offscreen reader scenes, including three split-view scenes, 53 portable upgrade
baselines plus four guards, and 51 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 16 installed runtime cases, including three native split-reader cases.
Apple Silicon passed seven installed checks on macOS 15. The 12-fixture conformance
corpus passed within its recorded profiles and operations; the public DEB contains
the same CLI bytes as that conformance run. All ten public downloads were checked
anonymously against their recorded SHA256 values, and the release is marked Latest.

Visual review covers ten images: three Windows split scenes transferred from
directly reviewed PR captures after exact RGBA comparison, six Ubuntu split scenes
reviewed directly, and one Mac PDF-display capture reviewed directly. Other Windows
scenes retain automated checks. See the evidence for source identities and methods.

Windows packages remain unsigned. The Mac package targets Apple Silicon, with a
binary-checked macOS 12 deployment minimum; it is ad hoc signed, not notarized.
Physical Macs, interactive Finder opening and clipboard interoperability remain
unqualified. Ubuntu supports in-app installation; the packaged updater helper is
tested within the documented limits. Graphical authorization and complete desktop
updater interaction were not exercised. Physical Windows hardware, Wayland and
human accessibility remain separate gates.

## Remaining work

AEC-017 remains **In progress** pending native plan-link interaction and broader interoperability. Batch saves are atomic per file; completed copies remain after a cancellation or failure. Original indexed PDFs remain the link destinations. See [workflow and limits](PLAN_LINKS.md).

The reader increment is delivered. AEC-011 visual search and AEC-012 region fill
retain their open native and broader-drawing qualification. Other backlog statuses are unchanged.
Windows packages remain unsigned; the Mac package is ad hoc signed, not notarized.
ARM64 hardware, physical printing, human accessibility, native spreadsheets and
private project plans retain their separate gates. No complete P1/P2 category, universal interoperability,
general PDF/A/PDF/UA conformance or overall performance claim is made.

[Backlog](AEC_BACKLOG.md) · [Roadmap](ROADMAP.md) · [Known issues](KNOWN_ISSUES.md) · [1.0 criteria](RELEASE_1.0_CRITERIA.md).
Repository documentation does not establish GitHub Projects synchronization. English remains the default UI.

## History

[0.75 qualification](RELEASE_0.75.0.md) retains the original Apple Silicon release evidence.

[0.74 qualification](RELEASE_0.74.0.md) retains the original theme and Ubuntu-update evidence.

[0.73 qualification](RELEASE_0.73.0.md) retains its original counts, source and limits.

Earlier qualification stays with [0.72](RELEASE_0.72.0.md), [0.71](RELEASE_0.71.0.md),
[0.70](RELEASE_0.70.0.md), [0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical counts are not added to this release.
The 0.69/0.70 CLI performance comparison retains its original binary, machine and workload scope in
[0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify later releases.
