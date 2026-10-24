# SuniPDF 0.75.0

Published stable on 2026-10-24: [download packages](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.75.0).
Source: `33c9116df1e2b35fed7337cf10ebcd6e5c0abc3d`.
See [qualification evidence](qa/RELEASE_0.75.0.json) and [project status](PROJECT_STATUS.md).

## Apple Silicon application

The macOS DMG contains SuniPDF, its command-line interface, the pinned PDFium engine,
license notices and the user guide. It targets Apple Silicon only. Install by dragging
SuniPDF to Applications. The bundle is ad hoc signed, not notarized.

The native menu bar, Finder PDF opening, Command shortcuts, system font discovery
and macOS application-data paths are provided. The native menu includes Theme settings.
The system clipboard is enabled. New-version notifications link to the official
release download page; replacing the application remains manual on macOS.

Unsupported OCR is disabled with a clear explanation. Printing, certificate signing,
Office import and persistent encrypted visual-signature storage retain their documented
platform limits. See [macOS installation and limits](MACOS.md).

## File and package integrity

JSON and plan-reference readers use platform-correct Unix nonblocking and no-follow
flags. On macOS, plan-reference checks verify the opened file descriptor's actual path,
including when a filename is replaced during a read.

The native package pipeline checks arm64 architecture, deployment requirements,
library dependencies, checksums, code sealing and license inventory. Installed-package
checks open and render a PDF, save and reopen a rotated copy and start the desktop
application. The input PDF must remain unchanged. These checks do not establish
notarization or physical Mac, Finder interaction and clipboard interoperability coverage.

## Executed qualification

The tagged release passed Windows x64 tests and package checks, the Windows ARM64 build, Ubuntu 22.04/24.04 installed checks and independent PDF readers, and native macOS build/installation checks. Windows upgrades cover 52 portable baselines plus four guards and 50 MSI baselines, including 0.74. Ten published files were downloaded without authentication and hash-verified.

On macOS 15.7.9 arm64, seven installed checks verify the checksum, bundle seal/licenses, architecture/deployment/dependencies, PDF reading/rendering, rotated-copy save/reopen, desktop launch and unchanged original. The captured PDF display was reviewed. The macOS 12 minimum is a binary deployment check, not execution on that version.

Ubuntu theme/window captures were reviewed on 22.04 and 24.04. Captures were compared against previously reviewed images using exact dimensions and RGBA pixels; changed captures require direct review. The evidence records the method for each platform. The installed update helper exercised pinned public upgrades and refusal cases. The graphical authorization prompt and complete desktop updater interaction remain unqualified.

The additional twelve historical CLI corpora were not rerun for this platform release. Earlier corpus counts and Windows offscreen appearance observations are not attributed to this build. No whole-roadmap, universal interoperability or general PDF/A/PDF/UA claim is made.
