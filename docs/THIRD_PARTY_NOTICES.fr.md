# Notices tierces

[English](THIRD_PARTY_NOTICES.md)

À livrer avec chaque distribution (MSI, ZIP). Aucune marque, logo ni ressource graphique d'un autre lecteur PDF
n'est utilisée ; aucun code d'un autre lecteur n'a été copié.

| Composant | Licence | Fichier de licence livré |
|---|---|---|
| PDFium (Google / Foxit) | BSD-3-Clause | `third_party/pdfium/win-*/licenses/pdfium.txt` → `LICENSE-pdfium.txt` |
| Bibliothèques incluses dans pdfium.dll (abseil, agg23, fast_float, freetype, icu, lcms, libjpeg-turbo, openjpeg, libpng, libtiff, llvm-libc, simdutf, zlib) | diverses (BSD, MIT, Apache-2.0, FreeType, IJG, zlib) | `third_party/pdfium/win-*/licenses/*` à copier dans le dossier `licenses/` de la distribution |
| pdfium-binaries (packaging, Benoit Blanchon) | MIT | `third_party/pdfium/win-*/LICENSE` → `LICENSE-pdfium-binaries.txt` |
| pdfium-render | MIT OR Apache-2.0 | généré par `cargo about` (P6) |
| lopdf 0.45.0, révision officielle `6ce451328c5caa65d50462daaf4ee8db9c85bb8e` (mot de passe propriétaire et encodage) | MIT | `assets/licenses/lopdf-MIT.txt` → `licenses/lopdf-MIT.txt` |
| chrono (date locale du jour, dépendance directe de `sunipdf-desktop` ; déjà dans l'arbre via lopdf et pdfium-render) | MIT OR Apache-2.0 | généré par `cargo about` (P6) |
| egui / eframe et polices par défaut (Ubuntu Light : Ubuntu Font Licence 1.0 ; Hack : MIT/Bitstream Vera ; Noto Emoji : OFL-1.1 ; emoji-icon-font : MIT) | voir colonne | généré par `cargo about` (P6) |
| Dépendances HTTP/TLS et helper de mise à jour | Licences par crate, dont ISC, BSD-3-Clause et CDLA-Permissive-2.0 | `assets/licenses/update-dependencies.txt` → `licenses/update-dependencies.txt` |
| Tesseract 5.5.3 (OCR hors ligne) | Apache-2.0 | `licenses/ocr-tesseract.txt`, `ocr-AUTHORS.txt`, `ocr-COPYRIGHTS.txt`, `ocr-NOTICE.txt` |
| Leptonica 1.87.0 (décodage PNM) | BSD-2-Clause | `licenses/ocr-leptonica.txt` |
| Modèles anglais et français tessdata_fast (révision figée) | Apache-2.0 | `licenses/ocr-tessdata-fast.txt`, `ocr-NOTICE.txt` |
| xml-rs 1.0.0 (dépendance directe, compatibilité), xml 1.4.0 (implémentation de lecture/écriture XMP/XFDF), xml-rs 0.8.29 (dépendance transitive) | MIT, mêmes mentions de copyright et texte de licence | `assets/licenses/xml-rs-MIT.txt` → `licenses/xml-rs-MIT.txt` |
| Autres crates Rust | Consulter la licence de chaque crate | Inventaire complet via `cargo about generate` (P6) |

Interdits : modèles OCR `ocrs` (CC-BY-SA-4.0) ; icônes/polices non vérifiées ; captures d'écran d'applications tierces.

L’icône SuniPDF est dérivée du visuel dédié fourni par l’auteur et retouché, `assets/icon/sunipdf-source.png`,
avec les tailles PNG et ICO produites par `scripts/make_icon.py`. Aucun élément graphique tiers supplémentaire.
