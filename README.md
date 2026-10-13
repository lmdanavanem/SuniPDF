# SuniPDF

Lecteur et éditeur PDF professionnel pour Windows 10/11, natif (Rust), hors ligne, sans télémétrie,
entièrement bilingue français / anglais. Moteur de rendu : PDFium.

État : **phase 1 (fondations)** — ouvrir, afficher, naviguer, zoomer, fermer un PDF, documents protégés par
mot de passe, glisser-déposer, thèmes clair/sombre, CLI `info`/`render`. Voir `docs/PROJECT_STATUS.md`
(fait vs planifié) et `docs/ROADMAP.md`.

## Prérequis

- Rust stable (rustup) : https://rustup.rs
- Éditeur de liens : **Build Tools pour Visual Studio** (charge « Développement Desktop en C++ »)
  ou, sans droits administrateur, MinGW-w64 WinLibs : `winget install -e --id BrechtSanders.WinLibs.POSIX.UCRT --scope user`
  puis `rustup toolchain install stable-x86_64-pc-windows-gnu` et `rustup override set stable-x86_64-pc-windows-gnu`.
- PDFium : téléchargé par `scripts\fetch-pdfium.ps1` (binaires bblanchon/pdfium-binaries, BSD-3-Clause).
- Python 3 (optionnel, régénération des PDF de test).

## Installation (utilisateurs)

- Portable : `scripts\package-portable.ps1` produit `dist\SuniPDF-<version>-win-x64-portable.zip` (+ SHA256). Décompresser et lancer `sunipdf.exe`.
- MSI : `scripts\package-msi.ps1` (WiX v3 requis) ou artefact de `release.yml`.
- Guides : `docs/USER_GUIDE.fr.md`, `docs/USER_GUIDE.en.md`.

## Démarrage (développeurs)

```powershell
cd C:\dev_suniworks\SuniPDF
. .\scripts\dev-env.ps1
.\scripts\start.ps1                      # fetch PDFium, check i18n, cargo test, lance l'application
.\scripts\start.ps1 -Pdf C:\docs\x.pdf   # ouvre directement un fichier
```

Ou manuellement : `cargo test --workspace -j 2`, `cargo run -p sunipdf-desktop`, `cargo run -p sunipdf-cli -- info fichier.pdf`.

## Raccourcis

Ctrl+O ouvrir · Ctrl+W fermer · Page suiv./préc., ← → naviguer · Origine/Fin première/dernière page ·
Ctrl + / Ctrl − zoom · Ctrl 0 taille réelle.

## Structure

```
apps/sunipdf-desktop   application (egui/eframe)      crates/sunipdf-core   moteur (PDFium)
apps/sunipdf-cli       CLI JSON                       crates/sunipdf-i18n   traductions fr-FR / en-US
assets/i18n            catalogues                     docs/                 architecture, audit, roadmap, décisions
scripts/               outils PowerShell / Python     .claude/agents/       12 sous-agents Claude Code
```

## Licence

Code SuniPDF : MIT OR Apache-2.0. Composants tiers : `docs/THIRD_PARTY_NOTICES.md`.
SuniPDF n'est affilié ni à Adobe, ni à Google/Foxit (PDFium), ni au projet PDFCraft/ArtCraft.
