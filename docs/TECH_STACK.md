# Stack technique

| Couche | Technologie | Version | Licence | Statut |
|---|---|---|---|---|
| Langage | Rust stable | 1.99.0 (rust-version min 1.85) | — | Utilisé |
| Cible de référence | `x86_64-pc-windows-msvc` | — | — | **Non compilée sur ce poste** (Build Tools absents) |
| Cible de travail actuelle | `x86_64-pc-windows-gnu` + MinGW-w64 (WinLibs, winget, scope utilisateur) | gcc 16.2.0 | GPL (outil, pas lié au produit) | Utilisé pour tous les tests de cette session |
| Cible future | `aarch64-pc-windows-msvc` | — | — | Planifié (P6) |
| Moteur PDF | PDFium (bblanchon/pdfium-binaries, build 7881, sans V8/XFA) | 151.0.7881.0 | BSD-3-Clause (PDFium), MIT (packaging) | Utilisé |
| Binding | `pdfium-render` | 0.9.4 | MIT OR Apache-2.0 | Utilisé |
| UI | `eframe` / `egui` | 0.36 | MIT OR Apache-2.0 | Utilisé |
| Dialogues natifs | `rfd` | 0.17 | MIT | Utilisé |
| Sérialisation | `serde`, `serde_json` | 1 | MIT OR Apache-2.0 | Utilisé |
| CLI | `clap` | 4 | MIT OR Apache-2.0 | Utilisé |
| Images (CLI PNG) | `image` | 0.25 (png seulement) | MIT OR Apache-2.0 | Utilisé |
| Erreurs / logs | `thiserror`, `log`, `env_logger` | 2 / 0.4 / 0.11 | MIT OR Apache-2.0 | Utilisé |
| Outil de vérification | qpdf | — | Apache-2.0 | Planifié (CI P2) |
| OCR | Tesseract + tessdata fra/eng | — | Apache-2.0 | Planifié (P4) |
| Chiffrement à l'écriture | `lopdf` ou crate dédié | 0.45 | MIT | Planifié (P5) |
| Installateur | WiX Toolset v4 | — | MS-RL (outil) | Planifié (P6) |
| CI | GitHub Actions `windows-latest` | — | — | Écrite, non exécutée |

## Pourquoi PDFium et non un moteur pur Rust

- Fidélité et robustesse : moteur de Chrome, fuzzé en continu, gère chiffrement AES-256, polices système
  Windows, transparence, formulaires, annotations.
- Une API pour lire, rendre, extraire le texte, chercher, remplir des formulaires, importer des pages,
  créer des annotations : couvre les phases 1 à 4 sans multiplier les dépendances.
- ARM64 Windows disponible immédiatement.
- Limites connues : pas de chiffrement à l'écriture, pas d'écriture incrémentale (la sauvegarde réécrit
  le fichier → nous imposerons sauvegarde atomique + vérification), pas thread-safe (un thread moteur).
- Repli pur Rust : `hayro` 0.8 (rendu) + `lopdf` (objets) — réévaluation en phase 6.

## Environnement de développement sur ce poste

- Ni Visual Studio ni Windows SDK installés ; pas de droits administrateur dans la session.
- Solution retenue sans admin : toolchain rustup `stable-x86_64-pc-windows-gnu` (override de dossier,
  hors dépôt : `rustup override set stable-x86_64-pc-windows-gnu`) + MinGW-w64 WinLibs via winget
  (`--scope user`). Les binutils livrés par rustup ne suffisent pas (`dlltool` sans `as`) pour les crates
  `raw-dylib` (`windows-sys`, `libloading`, `chrono`).
- `scripts/dev-env.ps1` configure le PATH automatiquement et bascule sur MSVC dès que les Build Tools existent.
