# Contributing to SuniPDF

[Français](CONTRIBUTING.fr.md)

SuniPDF is maintained by Emanuel DE ALMEIDA ([@lmdanavanem](https://github.com/lmdanavanem)) and is dual-licensed under MIT or Apache-2.0.

## Before you start

Read [Engineering notes](docs/ENGINEERING_NOTES.md) and [Architecture](docs/ARCHITECTURE.md).
Open an issue before a significant change; small fixes can go straight to a pull request.

## Project rules

- PDF processing stays local. No telemetry or PDF uploads; the optional update check contacts GitHub for release information only.
- English is the primary language. Every visible UI string belongs in both English and French catalogs.
- Keep the engine independent from the interface and CLI. Only one thread accesses PDFium.
- Save atomically, never overwrite silently, and keep unfinished features in the roadmap.
- Ship changes with meaningful tests, changelog entries, user guides and documented limits.

## Checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-i18n.ps1
```

## Pull requests

Keep one topic per PR. Describe the resulting behavior, validation and remaining limits.
Use clear English commit messages. Contributions are published under both project licenses.
