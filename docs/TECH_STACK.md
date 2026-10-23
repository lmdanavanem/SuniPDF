# Tech Stack

Versions and enabled features follow `Cargo.toml`, `Cargo.lock` and the pinned download scripts.
The table describes the current source; release qualification is recorded in [Project status](PROJECT_STATUS.md).

| Layer | Technology | Version | Licence | Status |
|---|---|---|---|---|
| Language | Rust stable | Minimum `rust-version` 1.95 | — | In use; CI selects the stable toolchain |
| Reference target | `x86_64-pc-windows-msvc` (Visual Studio Build Tools and Windows SDK) | Runner/toolchain-dependent | — | Windows CI, tests and x64 packages |
| Development fallback | `x86_64-pc-windows-gnu` + MinGW-w64 | Detected by `scripts/dev-env.ps1` | Toolchain notices | Development helper only; published packages use MSVC |
| Cross-compiled target | `aarch64-pc-windows-msvc` | — | — | Built in CI; not run on ARM64 hardware |
| PDF engine | PDFium (bblanchon/pdfium-binaries, build 7881, without V8/XFA) | 151.0.7881.0 | BSD-3-Clause (PDFium), MIT (packaging) | In use |
| Binding | `pdfium-render` | 0.9.4 | MIT OR Apache-2.0 | In use |
| UI | `eframe` / `egui` | 0.37 | MIT OR Apache-2.0 | In use |
| Native dialogs | `rfd` | 0.17 | MIT | In use |
| Serialisation | `serde`, `serde_json` | 1 | MIT OR Apache-2.0 | In use |
| CLI | `clap` | 4 | MIT OR Apache-2.0 | In use |
| Image codecs | `image` | 0.25; PNG, JPEG, BMP, GIF, TIFF, WebP | MIT OR Apache-2.0 | In use |
| Errors / logs | `thiserror`, `log`, `env_logger` | 2 / 0.4 / 0.11 | MIT OR Apache-2.0 | In use |
| Verification tool | qpdf | 12.4.2, pinned by `scripts/fetch-qpdf.ps1` | Apache-2.0 | Active CI/corpus checks; not an application runtime dependency |
| OCR | Tesseract, Leptonica and English/French `tessdata_fast` | 5.5.3 / 1.87.0; model revision pinned by `scripts/fetch-ocr.ps1` | See bundled upstream notices | Bundled offline OCR for x64 and ARM64 packages |
| Structural PDF edits and encryption | `lopdf` | 0.45; Git revision pinned in `Cargo.toml`/`Cargo.lock` | MIT | In use |
| Updates | `ureq`, `rustls`, `sha2`, `semver`, `tempfile`; Windows PowerShell helper | Locked in Cargo.lock | See packaged update dependency notices | In use from 0.8.2 |
| Installer | WiX Toolset v3.14 | — | MS-RL (tool) | In use |
| CI | GitHub Actions `windows-latest` | Workflow actions and downloads pinned in source | — | Workspace checks, independent readers, corpus and package/update regressions |

## PDF engine boundary

- PDFium handles reading, rendering, text extraction, forms, annotations and encrypted input.
- The core exposes shared APIs to read, render, extract text, search, fill forms, import pages
  and create annotations for both the desktop and CLI applications.
- Windows x64 and ARM64 binaries are pinned; ARM64 execution on hardware remains unqualified.
- A single worker owns the active PDFium document. Structural/encryption edits also use `lopdf`.
  Ordinary saves use atomic full-file rewrites; certificate signing has a separate incremental-copy path.

## Development setup

- Use the MSVC Rust target with the C++ build tools and Windows SDK for the reference build.
- `scripts/dev-env.ps1` detects MSVC or the local GNU fallback and configures the session PATH.
- `scripts/fetch-pdfium.ps1`, `scripts/fetch-ocr.ps1` and `scripts/fetch-qpdf.ps1` verify their pinned inputs.
  See [Engineering notes](ENGINEERING_NOTES.md) for build and test commands.
