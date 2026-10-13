# Notices tierces

À livrer avec chaque distribution (MSI, ZIP). Aucune marque, logo ni ressource graphique de PDFCraft/ArtCraft
n'est utilisée. Aucun code de PDFCraft n'a été copié à ce jour.

| Composant | Licence | Fichier de licence livré |
|---|---|---|
| PDFium (Google / Foxit) | BSD-3-Clause | `third_party/pdfium/win-*/licenses/pdfium.txt` → `LICENSE-pdfium.txt` |
| Bibliothèques incluses dans pdfium.dll (abseil, agg23, fast_float, freetype, icu, lcms, libjpeg-turbo, openjpeg, libpng, libtiff, llvm-libc, simdutf, zlib) | diverses (BSD, MIT, Apache-2.0, FreeType, IJG, zlib) | `third_party/pdfium/win-*/licenses/*` à copier dans le dossier `licenses/` de la distribution |
| pdfium-binaries (packaging, Benoit Blanchon) | MIT | `third_party/pdfium/win-*/LICENSE` → `LICENSE-pdfium-binaries.txt` |
| pdfium-render | MIT OR Apache-2.0 | généré par `cargo about` (P6) |
| egui / eframe et polices par défaut (Ubuntu Light : Ubuntu Font Licence 1.0 ; Hack : MIT/Bitstream Vera ; Noto Emoji : OFL-1.1 ; emoji-icon-font : MIT) | voir colonne | généré par `cargo about` (P6) |
| Autres crates Rust | MIT / Apache-2.0 | `cargo about generate` (P6) |

Interdits : modèles OCR `ocrs` (CC-BY-SA-4.0) ; icônes/polices non vérifiées ; captures d'écran de PDFCraft.
