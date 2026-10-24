# SuniPDF 0.75.0

Candidate under qualification. Published downloads and completed checks remain in
[project status](PROJECT_STATUS.md).

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
