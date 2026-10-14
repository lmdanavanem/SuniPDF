# Journal des versions

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions sont des tags `vX.Y.Z` ;
`release.yml` construit et publie les artefacts (ZIP portable x64/ARM64, MSI x64) sur la page Releases GitHub.

## [Non publié]

### Ajouté
- **Tampon / filigrane** et **numérotation des pages** (catégories Pages et Éditer) : texte apposé sur
  toutes les pages ou la page courante, taille, couleur, gras, angle, opacité, 7 positions,
  variables `{n}` et `{total}` ; chaque tampon reste un objet modifiable et annulable.
  Moteur `sunipdf-core::stamp`, CLI `stamp` et `page-numbers`, 1 test moteur.

### Corrigé (revue de code adverse, 2026-10-14)
- **Sécurité** : modifier les métadonnées, les supprimer, optimiser ou mettre un objet à l'arrière-plan
  retirait silencieusement le chiffrement d'un PDF protégé ; la protection d'origine (clé, mots de passe,
  permissions) est désormais réappliquée. « Retirer la protection » exige le mot de passe propriétaire
  (n'importe quel mot de passe passait sur un document protégé par mot de passe propriétaire seul) et
  l'application le demande dans une boîte dédiée. Des restrictions sans mot de passe propriétaire distinct
  sont refusées. Les propriétés signalent correctement une protection par mot de passe propriétaire seul.
- **Perte de données** : après « Enregistrer sous », une modification puis Ctrl+S écrasait le fichier
  d'origine ; « Diviser » et « Exporter en images » remplaçaient des fichiers existants ; la sélection
  d'objet survivait à un changement d'onglet (Suppr agissait sur l'autre document) ; fermer la fenêtre avec
  un onglet modifié en arrière-plan réenregistrait le document actif intact ; une modification en vol
  pendant un changement d'onglet perdait l'indicateur « modifié ».
- **Onglets** : cliquer un onglet depuis l'accueil bloquait l'application ; ouvrir un fichier pendant une
  autre ouverture créait un onglet fantôme ; une recherche en cours restait bloquée au retour sur l'onglet ;
  des miniatures d'un document fermé pouvaient apparaître dans le suivant.
- **Annulation** : annuler un « Retirer la protection » échouait et vidait l'historique (les instantanés
  gardent maintenant leur mot de passe) ; maintenir une flèche sur un objet ne crée plus qu'une entrée.
- Les raccourcis clavier qui exigent un document sont ignorés sur l'accueil (ils ouvraient un sélecteur
  de fichier).
- **CLI** : aucune sortie existante n'est écrasée sans `--overwrite` (code 6) ; `render` écrit toujours du
  PNG de façon atomique (une extension `.jpg` tronquait le fichier) ; `rotate` refuse les angles non
  multiples de 90 ; couleur non ASCII refusée au lieu d'un plantage ; angles négatifs acceptés ; pages
  en double traitées une fois ; erreurs lopdf typées (format invalide, E/S) ; numéros de page des
  messages en base 1 ; `optimize` ne produit jamais un fichier plus gros que l'entrée.

## [0.3.0] — 2026-10-14

### Ajouté
- **Boîtes de dialogue repensées** (cadre commun `dialog.rs`) : en-tête avec pastille d'icône, titre et
  sous-titre, pied à boutons (action principale bleue, action destructrice rouge), ombre et coins
  arrondis ; appliqué à À propos, Modifications non enregistrées, Mot de passe, Note, Texte, Diviser,
  Propriétés, Protéger.
- **À propos** : logo, version, confidentialité, licences, crédit « Créé par Emanuel DE ALMEIDA -
  Navanem - www.navanem.com » (lien).
- **Panneau d'outils et accueil façon PDFCraft** : icônes colorées par catégorie (fichier, pages,
  annotation, édition, conversion, sécurité), rangées avec surlignage, cartes à pastilles colorées et
  ombre. Les outils restent actifs sans document : ils ouvrent d'abord un fichier puis s'exécutent.
- **Édition** : rotation d'objet (±90°), duplication (Ctrl+Maj+D), premier plan (Ctrl+]) et
  arrière-plan (Ctrl+[), déplacement au clavier (flèches, Maj ×10), Échap pour désélectionner,
  proportions conservées pour les images (Maj pour libérer) et libres pour les autres objets
  (Maj pour verrouiller), boutons d'action dans la barre d'objet.
- Moteur : `rotate_object`, `duplicate_object`, `reorder_object` (fusion des flux de contenu via
  lopdf pour que l'ordre de la liste d'objets soit l'ordre de dessin) ; 1 test de rendu supplémentaire.
- Tests moteur sérialisés sur un moteur PDFium unique (PDFium ne se réinitialise pas dans un même
  processus).

### Corrigé
- Titre de fenêtre non rafraîchi après fermeture d'un document.
- Thème sombre : surfaces et bordures légèrement plus contrastées.

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
