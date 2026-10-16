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
| Read | Tabs, continuous/single-page/two-page/Book views, page labels and physical `#N` navigation, precise link/bookmark destinations, view rotation, full screen, per-tab position/zoom history, tiled rendering, search and logical cross-page selection |
| Annotate | Highlight, underline, strike-out, notes, ink, free text, shapes, links and reply threads |
| Edit | Text with standard or embedded system fonts, images, shapes, object selection/move/resize/rotation/opacity, grouped rotation/resizing, clipboard across tabs, Objects panel, mouse page crop and resize |
| Organize | Rotate, delete, insert, reorder, combine, split, extract and create blank pages; extraction/split preserve source encryption, passwords and permissions |
| Fill and sign | AcroForm filling, custom text in existing editable drop-downs, retained drafts/retry after read errors, grouped Apply/Discard, keyboard controls, flattening, JSON/XFDF export and JSON/XFDF/value-only FDF import; visual signatures, initials and date |
| Create forms | Named text fields, checkboxes, radio groups, drop-downs and single/multiple-selection lists through numeric page rectangles, including rotated/cropped/protected documents; undo and fill/save |
| Recover | Automatic snapshots every 60 seconds and on tab suspension; original unchanged, PDF encryption preserved, no passwords stored, restore through Save As |
| Print | Native Windows printer selection, page ranges/copies/orientation, current edits, bounded raster output, progress and cancellation |
| Stamp | Text watermarks and page numbering |
| Convert | Images to PDF, PDF to PNG, lossless optimization, lossy image compression to a new copy; bundled offline English/French OCR with invisible searchable text, existing-text skipping, cancellation and undo; sequential multi-file compression/OCR |
| Protect | AES-128/256 encryption and permissions, metadata cleanup, irreversible redaction |
| Certificates | Saved-byte integrity verification and incremental PFX/P12 signing into a new copy; certificate trust, revocation and trusted timestamps are unchecked; encrypted PDF signing is refused |
| Automate | `sunipdf-cli`: 50 commands with JSON output and stable exit codes, including `compress-images`, `form-export`, `rotate-objects`, `resize-objects`, `add-form-field`, `ocr`, `signatures` and `sign` |

All 14 task tools have real commands. Reading follows PDFium's logical character order; arbitrary untagged columns
are not reconstructed. Physical printing, ARM64 hardware and third-party signature interoperability remain separate QA checks.

[Feature matrix](docs/FEATURE_MATRIX.md) · [Project status](docs/PROJECT_STATUS.md) · [Changelog](CHANGELOG.md)

**[0.11.0 is published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0)** under `lmdanavanem`.
390 workspace tests, formatting/Clippy, 858 EN/FR keys, 44 qpdf checks across 42 distinct PDFs, four real NIST documents
(744 pages), final Windows CI, ARM64 compilation, nine portable scenarios and three MSI upgrade baselines passed.
Staged package checks include protected owner-opened AES-128/256 copies, page labels, OCR, forms and compression.
All six public downloads and enhanced public-binary checks passed; the production updater verified the Latest x64
ZIP without installation. [Project status](docs/PROJECT_STATUS.md) records evidence and hardware/interoperability limits.

In development for 0.12.0: create editable drop-downs and use radio-button `/Opt` export values consistently in
the app and JSON/XFDF/FDF interchange. Validation and publication are pending; see the [roadmap](docs/ROADMAP.md).

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
Offline OCR includes Tesseract 5.5.3, Leptonica 1.87.0 and pinned English/French fast models; keep the bundled
`licenses/` folder with the application. Recognition does not download models or send PDFs anywhere.

From version 0.8.2, SuniPDF checks for updates at startup and shows a download/install button. You decide when to close your documents and install.
Installed x64 copies use the MSI; portable copies update their own folder with a backup.
Older versions need one manual upgrade to obtain this feature. Automatic checks can be disabled in About; manual checks remain available.

[English user guide](docs/USER_GUIDE.en.md) · [French user guide](docs/USER_GUIDE.fr.md)

## Build

Install [stable Rust](https://rustup.rs), **Visual Studio Build Tools** with the Desktop development with C++ workload,
and **CMake** on PATH. The first OCR setup builds pinned native sources and checks their SHA256 hashes.

```powershell
git clone https://github.com/lmdanavanem/SuniPDF.git
cd SuniPDF
.\scripts\fetch-pdfium.ps1
.\scripts\fetch-ocr.ps1 -Arch x64
cargo test --workspace -j 2
cargo run -p sunipdf-desktop -j 2 -- document.pdf
cargo run -p sunipdf-cli -j 2 -- info document.pdf
.\scripts\package-portable.ps1 -Arch x64
.\scripts\package-msi.ps1 -Arch x64
```

[Engineering notes](docs/ENGINEERING_NOTES.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Documentation](docs/README.md)

## License and acknowledgements

SuniPDF is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice.
[Third-party notices](docs/THIRD_PARTY_NOTICES.md) accompany each distribution.
Built with [PDFium](https://pdfium.googlesource.com/pdfium/), [pdfium-render](https://github.com/ajrcarey/pdfium-render),
[lopdf](https://github.com/J-F-Liu/lopdf) and [egui/eframe](https://github.com/emilk/egui).
OCR uses [Tesseract](https://github.com/tesseract-ocr/tesseract) (Apache-2.0),
[Leptonica](https://github.com/DanBloomberg/leptonica) (BSD-2-Clause) and
[tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) English/French models (Apache-2.0).
Their licences, authors, copyright inventory and build notice ship in `licenses/`.
SuniPDF is not affiliated with Adobe, Google or Foxit.
