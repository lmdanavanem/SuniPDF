# Stack technique

[English](TECH_STACK.md)

Les versions et fonctionnalités activées suivent `Cargo.toml`, `Cargo.lock` et les scripts de téléchargement épinglés.
Ce tableau décrit la source actuelle ; les qualifications de release sont consignées dans [l'état du projet](PROJECT_STATUS.fr.md).

| Couche | Technologie | Version | Licence | Statut |
|---|---|---|---|---|
| Langage | Rust stable | `rust-version` minimal 1.95 | — | Utilisé ; la CI sélectionne la toolchain stable |
| Cible de référence | `x86_64-pc-windows-msvc` (Visual Studio Build Tools et Windows SDK) | Selon le runner et la toolchain | — | CI Windows, tests et paquets x64 |
| Repli de développement | `x86_64-pc-windows-gnu` + MinGW-w64 | Détecté par `scripts/dev-env.ps1` | Notices de la toolchain | Aide au développement uniquement ; les paquets publiés utilisent MSVC |
| Cible compilée en croisé | `aarch64-pc-windows-msvc` | — | — | Compilée en CI ; non exécutée sur matériel ARM64 |
| Moteur PDF | PDFium (bblanchon/pdfium-binaries, build 7881, sans V8/XFA) | 151.0.7881.0 | BSD-3-Clause (PDFium), MIT (packaging) | Utilisé |
| Binding | `pdfium-render` | 0.9.4 | MIT OR Apache-2.0 | Utilisé |
| UI | `eframe` / `egui` | 0.37 | MIT OR Apache-2.0 | Utilisé |
| Dialogues natifs | `rfd` | 0.17 | MIT | Utilisé |
| Sérialisation | `serde`, `serde_json` | 1 | MIT OR Apache-2.0 | Utilisé |
| CLI | `clap` | 4 | MIT OR Apache-2.0 | Utilisé |
| Codecs d'image | `image` | 0.25 ; PNG, JPEG, BMP, GIF, TIFF, WebP | MIT OR Apache-2.0 | Utilisé |
| Erreurs / logs | `thiserror`, `log`, `env_logger` | 2 / 0.4 / 0.11 | MIT OR Apache-2.0 | Utilisé |
| Outil de vérification | qpdf | 12.4.2, épinglé par `scripts/fetch-qpdf.ps1` | Apache-2.0 | Contrôles CI/corpus actifs ; absent des dépendances d'exécution de l'application |
| OCR | Tesseract, Leptonica et `tessdata_fast` anglais/français | 5.5.3 / 1.87.0 ; révision des modèles épinglée par `scripts/fetch-ocr.ps1` | Voir les notices amont embarquées | OCR hors ligne embarqué dans les paquets x64 et ARM64 |
| Édition structurelle et chiffrement PDF | `lopdf` | 0.45 ; révision Git épinglée dans `Cargo.toml`/`Cargo.lock` | MIT | Utilisé |
| Mises à jour | `ureq`, `rustls`, `sha2`, `semver`, `tempfile` ; helper Windows PowerShell | Verrouillées dans Cargo.lock | Voir notices des dépendances de mise à jour | Utilisé depuis 0.8.2 |
| Installateur | WiX Toolset v3.14 | — | MS-RL (outil) | Utilisé |
| CI | GitHub Actions `windows-latest` | Actions et téléchargements épinglés dans la source | — | Contrôles workspace, lecteurs indépendants, corpus et régressions des paquets/mises à jour |

## Périmètre du moteur PDF

- PDFium assure lecture, rendu, extraction de texte, formulaires, annotations et ouverture des fichiers chiffrés.
- Le moteur expose des API communes pour lire, rendre, extraire le texte, chercher, remplir des formulaires,
  importer des pages et créer des annotations, utilisées par les applications de bureau et CLI.
- Les binaires Windows x64 et ARM64 sont épinglés ; l'exécution sur matériel ARM64 reste à qualifier.
- Un seul worker possède le document PDFium actif. Les éditions structurelles et le chiffrement utilisent aussi `lopdf`.
  Les sauvegardes ordinaires sont des réécritures atomiques complètes ; la signature par certificat utilise une copie incrémentale distincte.

## Environnement de développement

- Utiliser la cible Rust MSVC avec les outils C++ et le Windows SDK pour la compilation de référence.
- `scripts/dev-env.ps1` détecte MSVC ou le repli GNU local et configure le PATH de la session.
- `scripts/fetch-pdfium.ps1`, `scripts/fetch-ocr.ps1` et `scripts/fetch-qpdf.ps1` vérifient leurs entrées épinglées.
  Les commandes de compilation et de test figurent dans les [notes de développement](ENGINEERING_NOTES.fr.md).
