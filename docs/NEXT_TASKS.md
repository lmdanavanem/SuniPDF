# Prochaines tâches (ordre conseillé)

## À faire par l'utilisateur (hors de portée de l'agent)
1. Lancer `scripts\start.ps1 -Pdf <un vrai PDF>` et valider visuellement : rendu, zoom, DPI, menus FR/EN, thème sombre, mot de passe.
2. Installer « Build Tools pour Visual Studio » (C++ Desktop) puis `rustup override unset` dans le dossier : cible MSVC de référence.
3. Pousser sur GitHub pour exécuter la CI (`.github/workflows/ci.yml`) une première fois.

## SUNI-012 — Affichage continu + onglets (owner : UI-UX)
Vue continue avec virtualisation (seules les pages visibles sont rendues) ; onglets multi-documents ; raccourcis.
Acceptation : 500 pages sans chargement simultané ; clés i18n `tabs.*`, `view.*`.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Sélection à la souris sur la page (`PdfPageText::chars_inside_rect`), copie presse-papiers, i18n `edit.copy`.

## SUNI-014b — Interface des opérations de pages (owner : UI-UX + PAGE-MANAGER)
Menu Pages (rotation de la page courante, suppression, extraction, fusion, division, Enregistrer sous) branché sur `sunipdf-core::pages` via le registre ; état « modifié » et invite à l'enregistrement ; vérification qpdf en CI ; fixtures AES-128/256 ; test « sauvegarde interrompue ».

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Icône originale (SVG → ICO), manifeste DPI-aware, ressources de version (`winres`/`embed-resource`), signature différée.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
