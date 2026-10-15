# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

[English](README.md)

**Lecteur et éditeur PDF natif pour Windows 10/11, libre, traitement local, bilingue anglais / français.**

Créé par **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Navanem](https://navanem.com/projets/sunipdf/) ·
Licence MIT ou Apache-2.0, au choix.

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![Licence](https://img.shields.io/badge/licence-MIT%20ou%20Apache--2.0-blue)

## Pourquoi SuniPDF
- **Vos fichiers restent chez vous** : aucun PDF envoyé, aucun compte, aucune télémétrie, aucun traitement dans le cloud. La recherche de mises à jour contacte GitHub et peut être désactivée.
- **Natif et léger** : Rust, interface egui, moteur de rendu PDFium ; un exécutable, un ZIP portable ou un MSI.
- **Complet** : lecture, annotation, édition du contenu, organisation des pages, formulaires, signature visuelle,
  tampons, conversions, protection, ligne de commande scriptable.
- **Honnête** : ce qui n'est pas fini reste dans la feuille de route (`docs/ROADMAP.fr.md`), jamais annoncé comme terminé.

## Fonctionnalités
| Domaine | Détail |
|---|---|
| Lire | Onglets, défilement continu ou page unique, rendu en tuiles au zoom profond, miniatures, recherche, signets, thème clair / sombre |
| Commenter | Surligner, souligner, barrer, notes, panneau des commentaires |
| Modifier | Texte (polices standard et polices système incorporées), images, formes, sélection / déplacement / redimensionnement / rotation / opacité des objets existants, copier-coller entre pages et onglets, panneau Objets, recadrage et redimensionnement des pages |
| Organiser | Pivoter, supprimer, insérer, réordonner (glisser-déposer), combiner, diviser, extraire, pages vierges |
| Remplir et signer | Formulaires AcroForm (remplir, panneau, aplatir, import / export JSON), signature manuscrite ou image, paraphe, date (signature visuelle, non certifiée) |
| Tampons | Filigrane texte (angle, opacité, position), numérotation des pages |
| Convertir | Images → PDF, PDF → images PNG, optimisation sans perte |
| Protéger | Chiffrement AES-128 / AES-256 avec permissions, retrait de la protection, nettoyage des métadonnées, propriétés |
| Automatiser | `sunipdf-cli` : 42 commandes à sortie JSON et codes de retour stables |

Détail fait / planifié : `docs/FEATURE_MATRIX.fr.md`, `docs/PROJECT_STATUS.fr.md`, `docs/ROADMAP.fr.md` ; nouveautés : `CHANGELOG.fr.md`.

## Captures d'écran
| Accueil (thème sombre) | Accueil (thème clair) |
|---|---|
| ![Accueil, thème sombre](docs/screenshots/home-dark.png) | ![Accueil, thème clair](docs/screenshots/home-light.png) |

| Modifier un PDF : sélection et style d'un objet | Organiser les pages : grille de miniatures |
|---|---|
| ![Modifier un PDF](docs/screenshots/edit-objects.png) | ![Organiser les pages](docs/screenshots/organize-pages.png) |

<p align="center"><img src="docs/screenshots/about.png" width="720" alt="Boîte À propos"></p>

## Installation
Page [Releases](https://github.com/lmdanavanem/SuniPDF/releases) : **MSI x64** (installation classique) ou
**ZIP portable x64** (décompresser, lancer `sunipdf.exe`), avec empreintes SHA256. Un ZIP ARM64 est compilé en
intégration continue mais n'a pas encore été exécuté sur du matériel ARM64. Aucun redistribuable Visual C++ n'est requis.

Guides : [`docs/USER_GUIDE.fr.md`](docs/USER_GUIDE.fr.md) · [`docs/USER_GUIDE.en.md`](docs/USER_GUIDE.en.md).

## Compiler
Prérequis : [Rust stable](https://rustup.rs) et les **Build Tools pour Visual Studio** (charge « Développement Desktop en C++ »).
```powershell
git clone https://github.com/lmdanavanem/SuniPDF.git
cd SuniPDF
.\scripts\fetch-pdfium.ps1              # binaires PDFium (bblanchon/pdfium-binaries), empreinte vérifiée
cargo test --workspace -j 2              # moteur, interface, CLI, i18n
cargo run -p sunipdf-desktop -- fichier.pdf
cargo run -p sunipdf-cli -- info fichier.pdf
.\scripts\package-portable.ps1 -Arch x64 # ZIP portable + SHA256
.\scripts\package-msi.ps1 -Arch x64      # MSI (WiX 3.14, téléchargé automatiquement)
```
Notes d'ingénierie (patrons de code, pièges, vérification visuelle) : `docs/ENGINEERING_NOTES.fr.md`.
Contribuer : `CONTRIBUTING.md` · Sécurité : `SECURITY.md`.

## Structure
```
apps/sunipdf-desktop   application (egui/eframe)      crates/sunipdf-core   moteur (PDFium, lopdf)
apps/sunipdf-cli       CLI JSON                       crates/sunipdf-i18n   traductions fr-FR / en-US
assets/i18n            catalogues                     docs/                 architecture, guides, décisions
packaging/wix          installateur MSI               scripts/              outils PowerShell / Python
```

## Licence et remerciements
Code SuniPDF : [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE), au choix. Composants tiers et leurs licences :
`docs/THIRD_PARTY_NOTICES.fr.md`. SuniPDF s'appuie sur [PDFium](https://pdfium.googlesource.com/pdfium/) via
[pdfium-render](https://github.com/ajrcarey/pdfium-render), [lopdf](https://github.com/J-F-Liu/lopdf),
[egui / eframe](https://github.com/emilk/egui) et les polices système Segoe UI. SuniPDF n'est affilié ni à Adobe,
ni à Google ou Foxit (PDFium).
