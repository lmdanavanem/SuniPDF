# Prochaines tâches (ordre conseillé)

## À faire par l'utilisateur (hors de portée de l'agent)
1. Lancer `scripts\start.ps1 -Pdf <un vrai PDF>` et valider la nouvelle interface (accueil, panneau d'outils, rail droit, palette Ctrl+K, thème sombre, FR/EN). Dire ce qui s'éloigne encore de la cible.
2. ~~Installer les Build Tools~~ fait le 13/10 au soir.
3. Pousser sur GitHub pour exécuter la CI (`ci.yml`) puis créer un tag `v0.1.0` pour `release.yml` (portable x64/ARM64, MSI x64).
4. En session administrateur : `winget install WiXToolset.WiXToolset` pour construire le MSI localement (`scripts\package-msi.ps1`).
5. `rustup target add aarch64-pc-windows-msvc` puis `scripts\package-portable.ps1 -Arch arm64` pour vérifier la compilation ARM64.

## Hub d'outils : contrat pour les tranches suivantes (tool-hub livré, D021)
Les onglets `ToolTab` et les cartes `home.card.*` n'existent plus : les plans de `docs/plans/` qui les citent
(OCR, caviardage, export, compression, formulaires, signatures…) passent par `apps/sunipdf-desktop/src/workspace.rs`.
Pour rendre un outil visible, une tranche fait exactement :
1. ajouter ses `Command` (moteur + `REGISTRY`, id stable, `needs_document` correct) ;
2. dans `workspace::TABLE`, passer `entry` à `Some(Entry::Direct(cmd))` ou `Some(Entry::Mode(Command::HubXxx))`,
   renseigner `groups` (chaque commande dans **un seul** outil), choisir l'icône ;
3. si l'outil a une barre : ajouter `HubXxx` au registre (menu `Tools`), au bras `Hub*` d'`execute` et à
   `shell::command_icon` ;
4. les noms, descriptions et couleurs des 14 outils existent déjà (`hub.*.name`, `hub.*.desc`).
Le test `every_registry_command_has_a_home` échoue si une commande n'a ni outil ni place dans la liste système ;
`bar_icons_are_distinct` échoue si une icône se répète dans une barre ou reprend celle d'un bouton d'accès rapide.
Outils encore absents : Remplir et signer, Numériser et OCR, Caviarder, Préparer un formulaire, Utiliser un certificat.

Reste du hub (non bloquant) : focus clavier placé sur la ligne active à l'ouverture du panneau (F9) ; rail droit
qui se chevauche sous ~650 px de haut (`KNOWN_ISSUES.md` A15) ; validation visuelle finale par l'utilisateur.

## SUNI-012 — Affichage continu + onglets (owner : UI-UX)
Vue continue avec virtualisation (seules les pages visibles sont rendues) ; onglets multi-documents ; raccourcis.
Acceptation : 500 pages sans chargement simultané ; clés i18n `tabs.*`, `view.*`.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Sélection à la souris sur la page (`PdfPageText::chars_inside_rect`), copie presse-papiers, i18n `edit.copy`.

## SUNI-031c — Édition du contenu, suite (owner : PDF-EDITOR)
Fait en SUNI-031/031b/032 : sélection, déplacement (souris, flèches), redimensionnement (proportions), rotation ±90°, duplication, ordre Z, suppression, texte (ajout/modification, taille, couleur, 3 familles standard), image, rectangle/ellipse/ligne, couleurs et épaisseur.
Reste : rotation libre (angle), polices système incorporées (TrueType), opacité, sélection multiple, copier/coller entre pages, panneau « Objets » listant `page_objects`, édition de texte en place (sans dialogue).
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
