# SuniPDF 0.51.0 — Ubuntu desktop package
[Ubuntu installation and limits](UBUNTU.md)

[0.51.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.51.0) on 2026-10-23, under `lmdanavanem`.

- Linux desktop backends, application-menu entry, PDF file association and installed CLI.
- Bundled open-source interface icons keep toolbar controls readable without Windows fonts.
- Private PDFium library with pinned checksums; local English/French OCR through Ubuntu packages.
- Linux font discovery, case-sensitive recent paths, XDG preferences and recovery protected against concurrent instances.
- Linux update notifications point to the Ubuntu download. Installation uses the system package manager.
- Explicit availability for features which still require Windows; existing Windows workflows remain supported.
- The published 0.50.0 packages are included in Windows portable and MSI upgrade regressions.

The first Linux package targets Ubuntu 22.04 and 24.04 on amd64. Native printing, certificate signing/trust verification, protected Office import and the persistent encrypted visual-signature library remain Windows-only. Session visual signatures and PDF-to-Office exports remain available. Linux ARM64 and physical-device qualification are outside this release's scope. Windows packages remain unsigned.

## Qualification

Source `f046c51df5d2caa31d4fa8710edd48471b241809`, PR [#31](https://github.com/lmdanavanem/SuniPDF/pull/31). Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879765536) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879765566) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879777012) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879764800). Windows: 781 passed, 0 failed, 12 ignored. Linux: 763 passed, 0 failed, 7 ignored. 1417 EN/FR keys. 43 portable scenarios; 37 MSI baselines / 185 logs. Eight anonymous public downloads verified by SHA256. No open Code Scanning alerts.

Ubuntu qualification uses clean 22.04/24.04 containers, a normal user, Xvfb and software Glow. It covers the installed package, visible PDF rendering, save/reopen, English/French OCR and removal/reinstallation of the same candidate; it does not establish a previous-version Linux upgrade or physical-device qualification. Linux ARM64 is outside this release. Native printing, certificate signing/trust verification, protected Office import and the persistent encrypted visual-signature library remain Windows-only. Session visual signatures and PDF-to-Office exports remain available. Windows packages remain unsigned; Windows ARM64 is cross-built without native execution. Projects updates remain deferred.

[Evidence](qa/RELEASE_0.51.0.json).
