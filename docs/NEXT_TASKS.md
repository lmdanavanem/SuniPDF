# Prochaines tâches (ordre conseillé)

## À faire par l'utilisateur (hors de portée de l'agent)
1. Lancer `scripts\start.ps1 -Pdf <un vrai PDF>` et valider la nouvelle interface (accueil, panneau d'outils, rail droit, palette Ctrl+K, thème sombre, FR/EN). Dire ce qui s'éloigne encore de la cible.
2. ~~Installer les Build Tools~~ fait le 13/10 au soir.
3. Pousser sur GitHub pour exécuter la CI (`ci.yml`) puis créer un tag `v0.1.0` pour `release.yml` (portable x64/ARM64, MSI x64).
4. En session administrateur : `winget install WiXToolset.WiXToolset` pour construire le MSI localement (`scripts\package-msi.ps1`).
5. `rustup target add aarch64-pc-windows-msvc` puis `scripts\package-portable.ps1 -Arch arm64` pour vérifier la compilation ARM64.

## SUNI-012 — Affichage continu + onglets (owner : UI-UX)
Vue continue avec virtualisation (seules les pages visibles sont rendues) ; onglets multi-documents ; raccourcis.
Acceptation : 500 pages sans chargement simultané ; clés i18n `tabs.*`, `view.*`.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Sélection à la souris sur la page (`PdfPageText::chars_inside_rect`), copie presse-papiers, i18n `edit.copy`.

## SUNI-031b — Édition du contenu, suite (owner : PDF-EDITOR)
Fait en SUNI-031 : sélection, déplacement, suppression, texte (ajout/modification), image, rectangle/ellipse/ligne.
Reste : redimensionnement par poignées (matrice d'objet), rotation, couleur de texte et de trait, choix de police (polices standard + système), opacité, sélection multiple, copier/coller d'objets, panneau « Objets » listant `page_objects`.
Acceptation : chaque opération couverte par un test moteur avec rendu vérifié ; clés i18n `dialog.object.*`.

## SUNI-021b — Annotations avancées (owner : PDF-EDITOR)
Dessin libre (ink), zone de texte libre (FreeText), déplacement/redimensionnement des annotations existantes, fils de réponses.

## SUNI-025 — Compression avec perte et vérification qpdf (owner : OCR-CONVERSION + QA)
Rééchantillonnage des images (PDFium : objets image → DynamicImage → réencodage JPEG), vérification qpdf en CI, fixtures AES produites par qpdf, test « sauvegarde interrompue ».

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Icône originale (SVG → ICO), manifeste DPI-aware, ressources de version (`winres`/`embed-resource`), signature différée.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
