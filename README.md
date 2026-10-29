# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="280" alt="SuniPDF">
  </picture>
</p>

<p align="center"><strong>Your PDFs. Your tools. On your computer.</strong></p>
<p align="center">A free, open-source PDF reader and editor for Windows, Ubuntu and macOS.<br>Read, annotate, edit and measure — with no account or telemetry.</p>

<p align="center">
  <a href="https://github.com/lmdanavanem/SuniPDF/releases/latest">Download SuniPDF</a> ·
  <a href="docs/USER_GUIDE.en.md">User guide</a> ·
  <a href="https://navanem.com/projets/sunipdf/">Website</a> ·
  <a href="https://github.com/lmdanavanem/SuniPDF/issues">Report an issue</a>
</p>

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases/latest)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)](#license-and-acknowledgements)

**From everyday documents to technical plans**, SuniPDF brings reading, annotation, page editing, calibrated measurements and quantity review into one desktop application. PDF processing stays local. The interface is available in **English and French**, with English as the default.

Created by **Emmanuel Almeida** ([@lmdanavanem](https://github.com/lmdanavanem)).

[Why SuniPDF?](#why-sunipdf) · [Download](#download) · [Get started](#get-started) · [Features](#features) · [What's new](#whats-new) · [FAQ](#faq) · [Contribute](#development-and-contributing)

## Why SuniPDF?

- **Keep PDF work local.** Read and process documents on your computer, without an account or telemetry.
- **Go beyond reading.** Annotate, edit, fill forms, organize pages and export documents from the same application.
- **Work with technical plans.** Calibrate scales, measure geometry, compare revisions and review quantities before exporting reports.
- **Reuse your work.** Save annotation templates, text entries, register filters and overlay setups for supported workflows.
- **Choose how you work.** Use the desktop interface or a scriptable JSON command-line interface.

## Download

**Current stable release: 0.92.0**, verified on 29 October 2026 at 02:55:40 UTC.

| Platform | Package |
|---|---|
| Windows 10/11 x64 | [MSI installer](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.92.0/SuniPDF-0.92.0-win-x64.msi) · [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.92.0/SuniPDF-0.92.0-win-x64-portable.zip) |
| Windows ARM64 | [Portable ZIP](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.92.0/SuniPDF-0.92.0-win-arm64-portable.zip) |
| Ubuntu 22.04 / 24.04 amd64 | [DEB package](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.92.0/SuniPDF-0.92.0-linux-amd64.deb) |
| macOS 12+ Apple silicon | [DMG image](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.92.0/SuniPDF-0.92.0-macos-arm64.dmg) |

[Release notes and SHA256 files](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.92.0) · [Release qualification](docs/RELEASE_0.92.0.md) · [Qualification evidence](docs/qa/RELEASE_0.92.0.json)

> **Platform notes:** Windows packages are unsigned. The macOS DMG is ad hoc signed but not notarized. Windows ARM64 is cross-built; execution on ARM64 hardware remains unqualified. Local OCR is available on Windows and Ubuntu.
>
> See [Ubuntu platform limits](docs/UBUNTU.md), [macOS platform limits](docs/MACOS.md) and [known issues](docs/KNOWN_ISSUES.md) before choosing a package.

## Get started

### 1. Install the application

| Platform | Installation |
|---|---|
| Windows | Run the MSI installer, or extract the **entire** portable ZIP and open **sunipdf.exe**. Keep the bundled files together. |
| Ubuntu | Download the DEB, open a terminal in its folder and run the command below. |
| macOS | Open the DMG and drag **SuniPDF** to **Applications**. For first-launch details, consult the [macOS guide](docs/MACOS.md). |

~~~bash
sudo apt install ./SuniPDF-0.92.0-linux-amd64.deb
~~~

### 2. Open a PDF and find your tools

1. Open a PDF in SuniPDF.
2. Choose **Tools** or press **F9**. Measurements, quantity tables and the annotation library are under **Review & measure**.
3. Use **Ctrl+K** (**⌘K** on macOS) to find a command.
4. Save your document to keep applied changes.

### 3. Make it yours

Open **Menu > Settings > Theme** to choose **Classic** or **Liquid Glass**, follow the system appearance or select light/dark colors, and reduce transparency.

Need more detail? Start with the [user guide](docs/USER_GUIDE.en.md) or browse the [documentation index](docs/README.md).

## Features

| Workflow | What you can do |
|---|---|
| **Read and navigate** | Work with tabs, independent synchronized views of one PDF, continuous and two-page layouts, bookmarks, search, text selection, page labels and tiled rendering. |
| **Annotate and edit** | Add notes, highlights, ink, shapes, text and images; transform objects, use reply threads and reuse annotation templates. |
| **Measure plans** | Measure lengths, perimeters, areas with deductions, constant-depth volumes, angles and counts. Use saved page scales, named scale regions, eligible vector snapping and saved-vertex editing. |
| **Review quantities** | Use docked or floating registers, typed fields, explicit unit prices and grouped formulas. Export PDF, CSV and JSON reports, or XLSX snapshots with reviewed refresh. |
| **Organize pages** | Reorder, rotate, crop, merge, split and extract pages. |
| **Compare revisions** | Work with grouped local plan sets, verified plan-link previews, reviewed annotation transfer, comparisons and aligned revision overlays. |
| **Fill and sign** | Fill and create AcroForms, exchange values through JSON/XFDF/FDF and add visual signatures. Certificate signatures are available on Windows within documented limits. |
| **Convert and protect** | Use local English/French OCR on Windows and Ubuntu, image compression, PDF/image/Office conversion, encryption, redaction and sanitized copies. |
| **Recover and automate** | Use automatic recovery snapshots, Undo/Redo and a JSON command-line interface. |

Feature availability and scope vary by platform and workflow. Consult the [feature matrix and limits](docs/FEATURE_MATRIX.md) for the precise support boundaries.

### A closer look at plan review

SuniPDF includes tools for moving from a drawing to reviewed measurements and quantities:

- **Calibrated geometry:** dimensioned rectangles, circles, ellipses and two-point segments with explicit dimensions and supported scale sources. Circles and ellipses use a visible 128-vertex polygon approximation; areas are computed from saved vertices.
- **Revision overlays:** compare two to four saved PDF pages with independent colors, visibility, opacity and reviewed alignment. Reusable setups retain exact source hashes and require a fresh preview when reopened.
- **Assisted takeoff:** review visual-symbol matches and region-fill areas before using the results.
- **Structured review:** apply keep/set/clear field changes to up to 256 explicitly selected annotations as one atomic Undo/Redo operation; stale, locked or invalid selections are refused as a complete batch.
- **Reusable filters:** save and exchange named register filters with criteria and sort order, including supported custom text-field matching.

Explore [plan links](docs/PLAN_LINKS.md), [register batch workflows](docs/REGISTER_BATCH.md), [region fill](docs/REGION_FILL.md) and [Find symbols](docs/VISUAL_SEARCH.md).

## What's new

### 0.92.0 — About and layer visibility

- A redesigned **About** window with privacy information, real update status, recovery settings and clear author credits in English and French.
- Click the eye in **Objects** to hide or show a page object while reading. This temporary per-tab view leaves saved PDF content intact and resets after document edits.
- Click a layer name or eye in **Revision overlay** to hide or show it and refresh the preview automatically. Exports and saved setups retain the visibility; hiding every layer produces a white canvas.

See the [0.92.0 release qualification](docs/RELEASE_0.92.0.md) for validation and platform limits.

### Highlights from 0.91.2

- More reliable placement of saved text-library entries, including capture from rotated or offset-crop pages.
- Guards against annotation-field changes that would make the register unreadable, while preserving unreadable saved-filter libraries instead of erasing them.
- More translated overlay, register and library messages, plus screen-reader names for field-change controls.

### Highlights from 0.91.0

Dimensioned geometry, multi-page revision overlays, reusable native stamp/image and supported rich-text annotations, explicit quantity-unit conversions, batch annotation-field updates and saved register filters.

For full technical details, see the [changelog](CHANGELOG.md), [0.91.0 release qualification](docs/RELEASE_0.91.0.md) and [latest release notes](https://github.com/lmdanavanem/SuniPDF/releases/latest).

## FAQ

### Does SuniPDF require an account or upload my PDFs?

No account is required. PDF processing stays on your computer, and SuniPDF has no telemetry. Update notifications appear at startup and can be disabled in **About**.

### Which languages are available?

The interface supports **English and French** and defaults to English. Local OCR uses English/French models on Windows and Ubuntu.

### How do updates work?

- **Windows:** SuniPDF downloads and verifies the update before offering installation.
- **Ubuntu:** version 0.74 introduced verified downloads, administrator authorization and save-protected restart. Users on 0.73 or earlier must install the current DEB manually once.
- **macOS:** download the new DMG from the [releases page](https://github.com/lmdanavanem/SuniPDF/releases/latest).

### Are visual signatures the same as certificate signatures?

They are separate workflows. SuniPDF supports visual signatures; certificate signatures are available on Windows within the [documented feature limits](docs/FEATURE_MATRIX.md).

### Is every feature qualified on every platform?

No. Check the [feature matrix](docs/FEATURE_MATRIX.md), [known issues](docs/KNOWN_ISSUES.md) and [project status](docs/PROJECT_STATUS.md). Private project-plan testing, physical-device checks and the remaining [1.0 release criteria](docs/RELEASE_1.0_CRITERIA.md) remain open.

## Documentation

| Looking for… | Start here |
|---|---|
| Everyday usage | [User guide](docs/USER_GUIDE.en.md) |
| All guides and references | [Documentation index](docs/README.md) |
| Supported features and limitations | [Feature matrix](docs/FEATURE_MATRIX.md) · [Known issues](docs/KNOWN_ISSUES.md) |
| Themes and appearance | [Appearance guide](docs/APPEARANCE.md) |
| Priorities and release readiness | [Roadmap](docs/ROADMAP.md) · [Project status](docs/PROJECT_STATUS.md) · [1.0 criteria](docs/RELEASE_1.0_CRITERIA.md) |
| Changes between versions | [Changelog](CHANGELOG.md) · [Releases](https://github.com/lmdanavanem/SuniPDF/releases) |

## Development and contributing

SuniPDF is built with **Rust**, **egui/eframe** and **PDFium**. For development setup, builds and validation, follow [CONTRIBUTING.md](CONTRIBUTING.md) and the [engineering notes](docs/ENGINEERING_NOTES.md).

- **Found a bug?** Open an [issue](https://github.com/lmdanavanem/SuniPDF/issues) with your operating system, SuniPDF version, reproduction steps, and expected versus actual behavior. Use a minimal, non-confidential sample where possible.
- **Have a question or idea?** Join the [discussions](https://github.com/lmdanavanem/SuniPDF/discussions).
- **Found a security vulnerability?** Follow the [security policy](SECURITY.md).

Please read the [code of conduct](CODE_OF_CONDUCT.md) when participating. If SuniPDF helps you, a GitHub star helps others discover the project.

## License and acknowledgements

SuniPDF is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice. Keep the bundled **licenses/** directory with redistributed packages. See the [third-party notices](docs/THIRD_PARTY_NOTICES.md) for included components and their licenses.

Built with [PDFium](https://pdfium.googlesource.com/pdfium/), [pdfium-render](https://github.com/ajrcarey/pdfium-render), [lopdf](https://github.com/J-F-Liu/lopdf) and [egui/eframe](https://github.com/emilk/egui). Local OCR uses [Tesseract](https://github.com/tesseract-ocr/tesseract), [Leptonica](https://github.com/DanBloomberg/leptonica) and English/French [tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) models.
