# Prochaines tâches (ordre conseillé)

## À faire par l'utilisateur (hors de portée de l'agent)
1. Lancer `scripts\start.ps1 -Pdf <un vrai PDF>` et valider visuellement : rendu, zoom, DPI, menus FR/EN, thème sombre, mot de passe.
2. Installer « Build Tools pour Visual Studio » (C++ Desktop) puis `rustup override unset` dans le dossier : cible MSVC de référence.
3. Pousser sur GitHub pour exécuter la CI (`.github/workflows/ci.yml`) une première fois.

## SUNI-011 — Cache de rendu et miniatures (owner : RENDERING)
Crate `sunipdf-render` : LRU plafonné en octets, pré-rendu page suivante/précédente, miniatures 128 px.
Acceptation : navigation sans re-rendu complet ; test de plafond mémoire.

## SUNI-012 — Affichage continu + onglets (owner : UI-UX)
Vue continue avec virtualisation (seules les pages visibles sont rendues) ; onglets multi-documents ; raccourcis.
Acceptation : 500 pages sans chargement simultané ; clés i18n `tabs.*`, `view.*`.

## SUNI-013 — Recherche, sélection et copie de texte (owner : PDF-ENGINE + UI-UX)
`FPDFText_*` via pdfium-render : `Document::search`, `Document::text_rects` ; surbrillance ; copie presse-papiers.

## SUNI-014 — Pages : fusion, division, extraction, rotation, réorganisation, sauvegarde (owner : PAGE-MANAGER)
`FPDF_ImportPages`, `FPDFPage_SetRotation`, `save_to_file` → écriture atomique ; vérification qpdf en CI ;
fixtures AES-128/256 (qpdf) pour `encrypted_*` ; test « sauvegarde interrompue ».

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Icône originale (SVG → ICO), manifeste DPI-aware, ressources de version (`winres`/`embed-resource`), signature différée.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
