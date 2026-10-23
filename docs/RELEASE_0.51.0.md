# SuniPDF 0.51.0 — Ubuntu desktop package

[Français](RELEASE_0.51.0.fr.md) · [Ubuntu installation and limits](UBUNTU.md)

This build adds an Ubuntu amd64 `.deb` alongside the Windows packages. Published files and exact-source qualification are tracked on the [0.51.0 release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.51.0).

- Linux desktop backends, application-menu entry, PDF file association and installed CLI.
- Bundled open-source interface icons keep toolbar controls readable without Windows fonts.
- Private PDFium library with pinned checksums; local English/French OCR through Ubuntu packages.
- Linux font discovery, case-sensitive recent paths, XDG preferences and recovery protected against concurrent instances.
- Linux update notifications point to the Ubuntu download. Installation uses the system package manager.
- Explicit availability for features which still require Windows; existing Windows workflows remain supported.
- The published 0.50.0 packages are included in Windows portable and MSI upgrade regressions.

The first Linux package targets Ubuntu 22.04 and 24.04 on amd64. Native printing, certificate signing/trust verification, protected Office import and the persistent encrypted visual-signature library remain Windows-only. Session visual signatures and PDF-to-Office exports remain available. Linux ARM64 and physical-device qualification are outside this release's scope. Windows packages remain unsigned.
