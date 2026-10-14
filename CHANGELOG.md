# Journal des versions

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions sont des tags `vX.Y.Z` ;
`release.yml` construit et publie les artefacts (ZIP portable x64/ARM64, MSI x64) sur la page Releases GitHub.

## [0.2.0] — 2026-10-14

### Ajouté
- **Édition du contenu des pages** (catégorie « Éditer », outils E/T/I/R/O/L dans la palette) :
  sélection d'un objet existant (texte, image, tracé) au clic, déplacement par glissement,
  redimensionnement par poignées d'angle, suppression (Suppr), modification du texte d'un objet
  existant (F2 ou double-clic : texte, taille, couleur), ajout de texte (Helvetica / Times / Courier,
  gras, italique, taille, couleur), insertion d'image (PNG, JPEG, BMP, GIF, TIFF, WebP, largeur 200 pt,
  proportions conservées), formes rectangle / ellipse / ligne avec aperçu pendant le tracé.
- **Barre des propriétés d'objet** (couleur et épaisseur du trait, remplissage) : s'applique aux
  nouvelles formes et, par « Appliquer à l'objet », à l'objet sélectionné (texte : couleur).
- Moteur `sunipdf-core::edit` : `page_objects`, `object_at`, `add_text`, `set_object_text`,
  `set_text_size`, `move_object`, `resize_object`, `set_object_style`, `delete_object`, `add_image`,
  `add_shape` ; 2 tests d'intégration avec rendu vérifié pixel par pixel.
- `scripts/dev-screenshot.ps1` : plusieurs glissements (`-Drag "a;b"`), touches après glissement
  (`-KeysAfter`), journal (`SUNIPDF_LOG`).

### Corrigé
- Origine réelle du glissement (position de la pression, pas celle du seuil de détection) : les
  déplacements et sélections commencent exactement sous le pointeur.
- Icône de l'outil Ligne.

### Limites connues
- Pas de rotation d'objet, pas de polices système incorporées (14 polices standard), le texte
  modifié conserve sa police d'origine (caractères absents non garantis). Voir `docs/KNOWN_ISSUES.md`.

## [0.1.0] — 2026-10-14

Première version : lecture (PDFium), onglets, miniatures, recherche, sélection/copie de texte,
annotations (surligner, souligner, barrer, notes, panneau Commentaires), organisation des pages
(rotation, suppression, insertion, déplacement, vue grille), fusion / division / extraction,
images ↔ PDF, export PNG, optimisation, signets, propriétés, protection AES-128/256 et retrait,
métadonnées, annuler/rétablir, palette de commandes, CLI JSON (20 commandes), interface bilingue
fr-FR / en-US, thèmes clair/sombre, ZIP portable et MSI x64.
