# SuniPDF 0.92.2

By Emanuel de Almeida.

Published stable on 2026-11-04 under [lmdanavanem](https://github.com/lmdanavanem).
[Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.2) | [Qualification evidence](qa/RELEASE_0.92.2.json) | [Project status](PROJECT_STATUS.md).

## Clipboard copy on Windows

Text copied from SuniPDF (Ctrl+C and the copy commands) now reaches the Windows clipboard, so it
can be pasted into other applications. Earlier builds kept the copied text inside the application
because the system clipboard support was not compiled into the Windows build. Fixes #157. A
regression test now checks that copied text reaches the Windows clipboard.

## Author name

The author name is spelled Emanuel de Almeida in the package metadata, the executable's copyright
resource, the About dialog and the documentation.

## Validation and scope

The exact tagged source is `1938aa3d71687b980ccbe568446418c5ea7b1b82`. All 11 jobs in the
[release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/41898358455) passed, including Windows packages, Ubuntu 22.04/24.04 installation
checks and macOS arm64 checks. Upgrade checks cover 61 portable baselines plus four guards and
59 MSI baselines, including the published 0.92.1 packages.

Locally, 578 desktop tests and 7 translation tests passed with strict Clippy, formatting and 2,202
matching English/French keys, and a real Ctrl+C placed selected text on the Windows clipboard.

All five packages and their SHA256 files were downloaded anonymously after publication. Every
package matched its checksum file and GitHub SHA256 digest; the stable Latest release was verified
at 2026-11-04T20:17:40Z. Windows packages remain unsigned. Windows ARM64 is cross-built without
native hardware execution. The macOS app is ad hoc signed, not notarized.

See the [changelog](../CHANGELOG.md) and [known limitations](KNOWN_ISSUES.md).
