# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

**Free, open-source PDF reader and editor for Windows, Ubuntu and macOS.**

Read, edit, annotate and measure PDFs on your computer. No account or telemetry.
The interface defaults to English; French is also available.

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases/latest)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)

Created by **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Project website](https://navanem.com/projets/sunipdf/)

## Download 0.91.2

SuniPDF 0.91.2 is the latest stable release, verified 2026-10-28T23:24:07Z.

| Platform | Package |
|---|---|
| Windows 10/11 x64 | [MSI installer](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.91.2/SuniPDF-0.91.2-win-x64.msi) · [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.91.2/SuniPDF-0.91.2-win-x64-portable.zip) |
| Windows ARM64 | [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.91.2/SuniPDF-0.91.2-win-arm64-portable.zip) |
| Ubuntu 22.04 / 24.04 amd64 | [DEB package](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.91.2/SuniPDF-0.91.2-linux-amd64.deb) |
| macOS 12+ Apple silicon | [DMG image](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.91.2/SuniPDF-0.91.2-macos-arm64.dmg) |

[Release notes and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.91.2) · [qualification evidence](docs/qa/RELEASE_0.91.2.json)

Windows packages are unsigned; the macOS DMG is ad hoc signed but not notarized. Windows ARM64 is cross-built; execution on ARM64 hardware remains unqualified.
See [Ubuntu platform limits](docs/UBUNTU.md), [macOS platform limits](docs/MACOS.md) and [known issues](docs/KNOWN_ISSUES.md).

Open **Menu > Settings > Theme** for Classic or Liquid Glass, System/Light/Dark colors and reduced transparency.

## Get started

- **Windows:** run the MSI, or extract the whole portable ZIP and open `sunipdf.exe`.
- **Ubuntu:** download the DEB and install it with `sudo apt install ./SuniPDF-0.91.2-linux-amd64.deb`.
- **macOS:** open the DMG and drag **SuniPDF** to **Applications**. On first launch, allow it in **System Settings > Privacy & Security > Open Anyway**.
- Open a PDF, then choose **Tools** or press **F9**. Find measurements, quantity tables and the annotation library under **Review & measure**.
- Use **Ctrl+K** (**⌘K** on macOS) to find a command. Save your document to keep applied changes.

Update notifications appear at startup and can be disabled in About. Windows downloads and verifies the update before
offering installation. Ubuntu 0.74 adds verified downloads, administrator authorization and save-protected restart; users on 0.73 or earlier install the current DEB manually once. On macOS, download the new DMG from the releases page. PDF processing stays local. Local OCR is available on Windows and Ubuntu.

## What you can do

| Area | Included workflows |
|---|---|
| Read | Tabs, independent synchronized views of one PDF, continuous and two-page layouts, bookmarks, search, text selection, page labels and tiled rendering |
| Annotate and edit | Notes, highlights, ink, shapes, text, images, object transforms, reply threads and reusable annotation templates |
| Measure plans | Lengths, perimeters, areas with deductions, constant-depth volumes, dimensioned rectangles, angles, counts, reviewed visual-symbol matches, reviewed region-fill areas, saved page scales, named scale regions, eligible vector snapping and saved-vertex editing |
| Review quantities | Docked or floating registers, typed fields, explicit unit prices, grouped formulas, PDF/CSV/JSON reports and XLSX snapshots with reviewed refresh |
| Organize and compare | Reorder, rotate, crop, merge, split and extract pages; grouped local plan sets, verified plan-link previews and return to the source, reviewed annotation transfer, comparison and aligned revision overlays |
| Fill and sign | AcroForm filling and creation, JSON/XFDF/FDF value exchange, visual signatures; certificate signatures on Windows within documented limits |
| Convert and protect | Local English/French OCR, image compression, PDF/image/Office conversion, encryption, redaction and sanitized copies |
| Recover and automate | Automatic recovery snapshots, Undo/Redo and a JSON command-line interface |

[Feature matrix and limits](docs/FEATURE_MATRIX.md) · [User guide](docs/USER_GUIDE.en.md)

## New in 0.91.2

- Place saved text-library entries at ordinary click origins, and capture saved text on rotated or offset-crop pages.
- Refuse annotation-field changes that would make the register unreadable, and keep unreadable saved-filter libraries instead of erasing them.
- Translate more overlay, register and library messages, and name the field-change controls for screen readers.

[Release qualification](docs/RELEASE_0.91.2.md).

## New in 0.91.0

- Create calibrated dimensioned rectangles at an explicit clockwise angle, with matching preview and saved measurement geometry.
- Compare two to four saved PDF pages with independent layer colors, visibility, opacity and reviewed alignment.
- Capture supported native stamp and image groups into the local annotation library, preserving pixels and transparency.
- Save reusable overlay setups with exact source hashes, including hidden layers, and require a fresh preview when reopening.
- Qualify reviewed two-source plan-link creation, guarded new-copy saves, exported receipts and verified destination navigation in installed Ubuntu packages.
- Convert formula inputs explicitly between supported length, area or volume units with checked decimal arithmetic and retained source-unit guards.
- Review and apply keep/set/clear changes to business status, discipline and individual custom text keys across up to 256 explicitly selected annotations as one atomic Undo/Redo operation.
- Refuse stale, locked or invalid selections as a complete batch; the annotation-fields-batch CLI previews changes and saves a new copy without overwriting existing files.
- Save and exchange named register filters with their criteria and sort order; the bounded local library supports 16 entries and 128 KiB.
- Match one stored custom text key and value literally and case-sensitively, distinguishing present empty values from missing fields; share the criterion across register, quantity and issue-report selections.
- Use version-2 saved-filter libraries only when the custom-field criterion is present, preserving the exact version-1 representation otherwise.
- Capture supported saved native rich-text annotations using twelve Helvetica, Times and Courier font/style variants with supported shapes and stamps, preserving verified appearances and geometry across cropped page rotations.
- Create dimensioned circles and ellipses from explicit real dimensions, rotation and a captured/page/region/automatic calibration; keep their 128-vertex polygon approximation visible and compute area from saved vertices.
- Create dimensioned two-point segments from a real length and clockwise angle with the same explicit scale sources; save an ordinary editable calibrated length without permanent geometric constraints.
- Review source-bound previews before applying the complete measurement, and refuse invalid dimensions, stale references, unavailable scales or existing CLI output files.

[Feature guide](docs/REGISTER_BATCH.md) | [Release qualification](docs/RELEASE_0.91.0.md).

[Plan-link workflow](docs/PLAN_LINKS.md) | [Appearance](docs/APPEARANCE.md) | [Release qualification](docs/RELEASE_0.91.0.md) | [Project status](docs/PROJECT_STATUS.md).

Earlier additions include [independent reader views](docs/RELEASE_0.76.0.md),
[Apple Silicon support](docs/RELEASE_0.75.0.md), [themes and Ubuntu updates](docs/RELEASE_0.74.0.md),
[region fill](docs/REGION_FILL.md), [Find symbols](docs/VISUAL_SEARCH.md),
[quantity units](docs/RELEASE_0.70.0.md) and [reports and workbook refresh](docs/RELEASE_0.69.0.md).

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
