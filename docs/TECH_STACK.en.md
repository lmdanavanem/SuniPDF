# Tech Stack

*English translation of [`TECH_STACK.md`](TECH_STACK.md); the French version is the reference.*

| Layer | Technology | Version | Licence | Status |
|---|---|---|---|---|
| Language | Rust stable | 1.99.0 (rust-version min 1.85) | — | In use |
| Reference target | `x86_64-pc-windows-msvc` (Build Tools 2026 / MSVC 14.51, SDK 10.0.26100) | — | — | **In use** since the evening of 2026-10-13: `cargo test --workspace` green |
| Fallback target | `x86_64-pc-windows-gnu` + MinGW-w64 (WinLibs, winget, user scope) | gcc 16.2.0 | GPL (tool, not linked into the product) | Used before the Build Tools were installed; kept as a fallback without admin rights |
| Future target | `aarch64-pc-windows-msvc` | — | — | Planned (P6) |
| PDF engine | PDFium (bblanchon/pdfium-binaries, build 7881, without V8/XFA) | 151.0.7881.0 | BSD-3-Clause (PDFium), MIT (packaging) | In use |
| Binding | `pdfium-render` | 0.9.4 | MIT OR Apache-2.0 | In use |
| UI | `eframe` / `egui` | 0.36 | MIT OR Apache-2.0 | In use |
| Native dialogs | `rfd` | 0.17 | MIT | In use |
| Serialisation | `serde`, `serde_json` | 1 | MIT OR Apache-2.0 | In use |
| CLI | `clap` | 4 | MIT OR Apache-2.0 | In use |
| Images (CLI PNG) | `image` | 0.25 (png only) | MIT OR Apache-2.0 | In use |
| Errors / logs | `thiserror`, `log`, `env_logger` | 2 / 0.4 / 0.11 | MIT OR Apache-2.0 | In use |
| Verification tool | qpdf | — | Apache-2.0 | Planned (CI P2) |
| OCR | Tesseract + tessdata fra/eng | — | Apache-2.0 | Planned (P4) |
| Encryption on write | `lopdf` or a dedicated crate | 0.45 | MIT | Planned (P5) |
| Installer | WiX Toolset v4 | — | MS-RL (tool) | Planned (P6) |
| CI | GitHub Actions `windows-latest` | — | — | Written, not run |

## Why PDFium rather than a pure Rust engine

- Fidelity and robustness: Chrome's engine, continuously fuzzed, handles AES-256 encryption, Windows
  system fonts, transparency, forms, annotations.
- One API to read, render, extract text, search, fill forms, import pages,
  create annotations: covers phases 1 to 4 without multiplying dependencies.
- Windows ARM64 available immediately.
- Known limits: no encryption on write, no incremental writing (saving rewrites
  the file → we will enforce atomic save + verification), not thread-safe (a single engine thread).
- Pure Rust fallback: `hayro` 0.8 (rendering) + `lopdf` (objects); re-evaluation in phase 6.

## Development environment on this machine

- Visual Studio Build Tools installed on 2026-10-13 (evening): MSVC target active (`rustup override unset`).
- Before that, the no-admin solution: rustup toolchain `stable-x86_64-pc-windows-gnu` (directory override,
  outside the repository: `rustup override set stable-x86_64-pc-windows-gnu`) + MinGW-w64 WinLibs via winget
  (`--scope user`). The binutils shipped by rustup are not enough (`dlltool` without `as`) for the
  `raw-dylib` crates (`windows-sys`, `libloading`, `chrono`).
- `scripts/dev-env.ps1` configures the PATH automatically and switches to MSVC as soon as the Build Tools exist.
