# SuniPDF 0.92.1

By Emmanuel Almeida.

Published stable on 2026-11-03 under [lmdanavanem](https://github.com/lmdanavanem).
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.1) | [Qualification evidence](qa/RELEASE_0.92.1.json) | [Project status](PROJECT_STATUS.md).

## Window at startup

Open **Menu → Settings → Theme** and choose **Window at startup**. **Last state** (the default)
reopens SuniPDF maximized if it was maximized when closed; **Maximized** and **Windowed** always
open that way. A windowed start is centered on the screen. Ubuntu Wayland sessions choose the
position themselves. The choice applies at the next start. [Theme guide](APPEARANCE.md).

## Taskbar icon after Windows updates

The Start Menu shortcut installed by the MSI now uses the icon embedded in `sunipdf.exe`.
Earlier versions pointed it to an installer copy that each update removed, so taskbar pins
showed a blank document icon. A pin that is already blank must be unpinned and pinned again once
after installing this version. The real MSI upgrade regression now checks the shortcut icon.

## Build reliability

Continuous integration removes evidence folders restored with its build cache before running
the packaged regressions, which had refused to reuse an existing output folder.

## Validation and scope

The exact tagged source is `cfc9f64a62971da04ef7224f394e19ebd7f1411b`. It also contains the Dependabot GitHub Actions and Cargo
minor/patch updates (#79, #84), each merged after its own CI passed. All 11 jobs in the
[release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/41729556330) passed, including Windows packages, Ubuntu 22.04/24.04 installation
checks and macOS arm64 checks. Upgrade checks cover 60 portable baselines plus four guards and
58 MSI baselines, including the published 0.92.0 packages; each MSI upgrade verifies that the
Start Menu shortcut icon no longer comes from the per-version installer cache.

Locally, 584 desktop and translation tests passed with strict Clippy, formatting and 2,202
matching English/French keys. A debug build with isolated preferences opened centered when
windowed, maximized when requested, and restored the maximized state recorded at a graceful close.
Physical multi-monitor layouts and screen readers were not exercised.

All five packages and their SHA256 files were downloaded anonymously after publication. Every
package matched its checksum file and GitHub SHA256 digest; the stable Latest release was verified
at 2026-11-03T23:06:06Z. Windows packages remain unsigned. Windows ARM64 is cross-built without
native hardware execution. The macOS app is ad hoc signed, not notarized.

See the [changelog](../CHANGELOG.md) and [known limitations](KNOWN_ISSUES.md).
