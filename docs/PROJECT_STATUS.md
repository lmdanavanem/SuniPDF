# Project status

## Published 0.75.0

[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.75.0) · [Release notes](RELEASE_0.75.0.md) · [Qualification evidence](qa/RELEASE_0.75.0.json).

Apple Silicon Macs now have a native DMG, menus, Command shortcuts, Finder opening and macOS settings/recovery paths. Theme settings retain Classic and Liquid Glass, System/Light/Dark modes and reduced transparency. Mac update notifications link to manual installation; Ubuntu retains verified in-app installation.

The exact tagged packages passed Windows, Ubuntu and macOS qualification. Coverage includes 52 portable upgrade baselines plus four guards, 50 MSI baselines, Ubuntu 22.04/24.04 installation and native window/theme checks, and seven installed Mac checks on macOS 15. The Mac PDF display was reviewed. All ten public downloads were checked anonymously against their SHA256 values.

The Mac package targets Apple Silicon only, with a binary-checked macOS 12 deployment minimum. It is ad hoc signed, not notarized. Physical Macs, interactive Finder opening and clipboard interoperability remain unqualified. Graphical Linux authentication, physical Windows hardware and human accessibility remain separate gates.

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

[0.74 qualification](RELEASE_0.74.0.md) retains the original theme and Ubuntu-update evidence.

[0.73 qualification](RELEASE_0.73.0.md) retains its original counts, source and limits.

Earlier qualification stays with [0.72](RELEASE_0.72.0.md), [0.71](RELEASE_0.71.0.md),
[0.70](RELEASE_0.70.0.md), [0.69](RELEASE_0.69.0.md), [0.67](RELEASE_0.67.0.md) and the
[historical archive](archive/PROJECT_STATUS_PRE_0.60.md). Historical counts are not added to this release.
The 0.69/0.70 CLI performance comparison retains its original binary, machine and workload scope in
[0.70 evidence](qa/RELEASE_0.70.0.json); it does not qualify later releases.
