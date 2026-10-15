# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

**Free native PDF reader and editor for Windows 10/11. Private local processing, English and French.**

[Français](README.fr.md) · Created by **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Navanem](https://navanem.com/projets/sunipdf/)

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)

## Why SuniPDF

- **Your PDFs stay on your computer.** No account, telemetry or cloud processing. Update checks contact GitHub for release information and can be disabled.
- **Native Windows application.** Rust, egui and PDFium, distributed as a portable ZIP or MSI installer.
- **English by default, French available.** Change the interface language without restarting; existing language preferences are preserved.
- **Clear limits.** Unfinished features stay in the [roadmap](docs/ROADMAP.md).

## Features

| Area | Features |
|---|---|
| Read | Tabs, continuous or single-page view, tiled rendering at high zoom, thumbnails, search, bookmarks, light/dark themes |
| Annotate | Highlight, underline, strike-out, notes, ink, free text, shapes, links and reply threads |
| Edit | Text with standard or embedded system fonts, images, shapes, object selection/move/resize/rotation/opacity, clipboard across tabs, Objects panel, page crop and resize |
| Organize | Rotate, delete, insert, reorder, combine, split, extract and create blank pages |
| Fill and sign | AcroForm filling, flattening and JSON import/export; handwritten/image signatures, initials and date (visual signatures, not digital certificates) |
| Stamp | Text watermarks and page numbering |
| Convert | Images to PDF, PDF to PNG, lossless optimization |
| Protect | AES-128/256 encryption and permissions, metadata cleanup, irreversible redaction |
| Automate | `sunipdf-cli`: 42 commands with JSON output and stable exit codes |

[Feature matrix](docs/FEATURE_MATRIX.md) · [Project status](docs/PROJECT_STATUS.md) · [Changelog](CHANGELOG.md)

## Screenshots

| Dark home screen | Light home screen |
|---|---|
| ![Dark home screen](docs/screenshots/home-dark.png) | ![Light home screen](docs/screenshots/home-light.png) |

| Edit PDF objects | Organize pages |
|---|---|
| ![Object editing](docs/screenshots/edit-objects.png) | ![Page organization](docs/screenshots/organize-pages.png) |

## Install and update

Download from [GitHub Releases](https://github.com/lmdanavanem/SuniPDF/releases): **Windows x64 MSI** or **portable ZIP**.
An ARM64 portable ZIP is built in CI but has not been run on ARM64 hardware. SHA256 checksums are provided.
No Visual C++ redistributable is required.

From version 0.8.2, SuniPDF checks for updates at startup and shows a download/install button. You decide when to close your documents and install.
Installed x64 copies use the MSI; portable copies update their own folder with a backup.
Older versions need one manual upgrade to obtain this feature. Automatic checks can be disabled in About; manual checks remain available.

[English user guide](docs/USER_GUIDE.en.md) · [French user guide](docs/USER_GUIDE.fr.md)

## Build

Install [stable Rust](https://rustup.rs) and the **Visual Studio Build Tools** with the Desktop development with C++ workload.

```powershell
git clone https://github.com/lmdanavanem/SuniPDF.git
cd SuniPDF
.\scripts\fetch-pdfium.ps1
cargo test --workspace -j 2
cargo run -p sunipdf-desktop -- document.pdf
cargo run -p sunipdf-cli -- info document.pdf
.\scripts\package-portable.ps1 -Arch x64
.\scripts\package-msi.ps1 -Arch x64
```

[Engineering notes](docs/ENGINEERING_NOTES.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Documentation](docs/README.md)

## License and acknowledgements

SuniPDF is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice.
[Third-party notices](docs/THIRD_PARTY_NOTICES.md) accompany each distribution.
Built with [PDFium](https://pdfium.googlesource.com/pdfium/), [pdfium-render](https://github.com/ajrcarey/pdfium-render),
[lopdf](https://github.com/J-F-Liu/lopdf) and [egui/eframe](https://github.com/emilk/egui).
SuniPDF is not affiliated with Adobe, Google or Foxit.
