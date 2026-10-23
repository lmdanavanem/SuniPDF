# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

**Free, open-source PDF reader and editor for Windows and Ubuntu.**

Read, edit, annotate and measure PDFs on your computer. No account or telemetry.
The interface defaults to English; French is also available.

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases/latest)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)

Created by **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Project website](https://navanem.com/projets/sunipdf/)

## Download 0.61.0

| Platform | Package |
|---|---|
| Windows 10/11 x64 | [MSI installer](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.61.0/SuniPDF-0.61.0-win-x64.msi) · [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.61.0/SuniPDF-0.61.0-win-x64-portable.zip) |
| Windows ARM64 | [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.61.0/SuniPDF-0.61.0-win-arm64-portable.zip) |
| Ubuntu 22.04 / 24.04 amd64 | [DEB package](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.61.0/SuniPDF-0.61.0-linux-amd64.deb) |

[Release notes and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.61.0) · [Qualification evidence](docs/qa/RELEASE_0.61.0.json)

Packages are unsigned. Windows ARM64 is cross-built; execution on ARM64 hardware remains unqualified.
See [Ubuntu platform limits](docs/UBUNTU.md) and [known issues](docs/KNOWN_ISSUES.md).

## Get started

- **Windows:** run the MSI, or extract the whole portable ZIP and open `sunipdf.exe`.
- **Ubuntu:** download the DEB and install it with `sudo apt install ./SuniPDF-0.61.0-linux-amd64.deb`.
- Open a PDF, then choose **Tools** or press **F9**. Find measurements, quantity tables and the annotation library under **Review & measure**.
- Use **Ctrl+K** to find a command. Save your document to keep applied changes.

Update notifications appear at startup and can be disabled in About. Windows downloads and verifies the update before
offering installation; Ubuntu opens the release download for installation through APT. PDF processing and OCR stay local.

## What you can do

| Area | Included workflows |
|---|---|
| Read | Tabs, continuous and two-page views, bookmarks, search, text selection, page labels and tiled rendering |
| Annotate and edit | Notes, highlights, ink, shapes, text, images, object transforms, reply threads and reusable annotation templates |
| Measure plans | Lengths, perimeters, areas, angles, counts, saved page scales, named scale regions, eligible vector snapping and saved-vertex editing |
| Review quantities | Docked or floating registers, typed fields, explicit unit prices, grouped totals and CSV/JSON exports |
| Organize and compare | Reorder, rotate, crop, merge, split and extract pages; local comparison and aligned revision overlays |
| Fill and sign | AcroForm filling and creation, visual signatures; certificate signatures on Windows within documented limits |
| Convert and protect | Local English/French OCR, image compression, PDF/image/Office conversion, encryption, redaction and sanitized copies |
| Recover and automate | Automatic recovery snapshots, Undo/Redo and a JSON command-line interface |

[Feature matrix and limits](docs/FEATURE_MATRIX.md) · [User guide](docs/USER_GUIDE.en.md)

## New in 0.61.0

Sort custom register fields and typed quantity fields, reuse supported saved shape annotations as portable groups,
and calculate report formulas with explicit units, rounding and visible errors. Reuse formula configuration JSON
and export results tied to the source document.

Independent PDF/A/PDF/UA fixtures and PDFium/Poppler checks cover specified files and operations. Lossless
optimization preserves the existing compression state of XML metadata, fixing a PDF/A-1 regression found by this corpus.

[Release notes](docs/RELEASE_0.61.0.md) · [Qualification scope](docs/CONFORMANCE.md)

## Documentation and development

- [Documentation index](docs/README.md): usage guides, engineering references and release history.
- [Roadmap](docs/ROADMAP.md): priorities and remaining work.
- [Project status](docs/PROJECT_STATUS.md): executed checks and qualification limits.
- [Contributing](CONTRIBUTING.md) and [engineering notes](docs/ENGINEERING_NOTES.md): setup, builds and validation.
- [Security policy](SECURITY.md): reporting vulnerabilities.

SuniPDF uses Rust, egui and PDFium. Private project-plan testing, physical-device checks and the remaining
[1.0 release criteria](docs/RELEASE_1.0_CRITERIA.md) remain open.

## License and acknowledgements

SuniPDF is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice.
Keep the bundled `licenses/` directory with redistributed packages; [third-party notices](docs/THIRD_PARTY_NOTICES.md)
describe the included components and their licenses.

Built with [PDFium](https://pdfium.googlesource.com/pdfium/), [pdfium-render](https://github.com/ajrcarey/pdfium-render),
[lopdf](https://github.com/J-F-Liu/lopdf) and [egui/eframe](https://github.com/emilk/egui).
Local OCR uses [Tesseract](https://github.com/tesseract-ocr/tesseract),
[Leptonica](https://github.com/DanBloomberg/leptonica) and English/French
[tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) models.
