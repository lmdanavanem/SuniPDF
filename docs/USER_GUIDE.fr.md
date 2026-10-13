# SuniPDF — Guide utilisateur

SuniPDF est un lecteur et éditeur PDF pour Windows 10/11. Il fonctionne entièrement hors ligne : aucun
compte, aucune télémétrie, aucun document envoyé sur Internet.

## Installation

- **Portable** : décompressez le fichier `SuniPDF-<version>-win-x64-portable.zip` où vous voulez et lancez
  `sunipdf.exe`. Rien n'est écrit en dehors de vos préférences (`%APPDATA%\SuniPDF\settings.json`).
- **Installateur MSI** : double-cliquez sur `SuniPDF-<version>-win-x64.msi`. SuniPDF apparaît dans le menu
  Démarrer et dans « Ouvrir avec » pour les fichiers PDF ; il ne devient pas le lecteur par défaut sans votre accord.
- Désinstallation : Paramètres Windows › Applications, ou suppression du dossier pour la version portable.

## Écran d'accueil

L'accueil présente les outils disponibles, les fichiers récents et un bouton **Ouvrir un fichier**. Vous
pouvez aussi glisser un PDF n'importe où dans la fenêtre, ou cliquer sur **+ Ouvrir** dans la barre d'onglets.

## Onglets

Chaque PDF ouvert occupe un onglet dans la barre du haut. Cliquez sur un onglet pour l'afficher, sur sa croix
pour le fermer, sur la maison pour revenir à l'accueil sans fermer. Ctrl+Tab et Ctrl+Maj+Tab passent d'un
onglet à l'autre. Les modifications non enregistrées de chaque onglet sont signalées par un point et
confirmées une à une à la fermeture de l'application.

## Lire un document

| Action | Comment |
|---|---|
| Page suivante / précédente | Page suiv. / Page préc., flèches ← →, boutons du rail droit |
| Première / dernière page | Origine / Fin |
| Aller à une page | Saisir le numéro dans la case du rail droit puis Entrée |
| Zoom | Ctrl + / Ctrl −, Ctrl 0 (taille réelle), liste de zoom, « Largeur », « Page » |
| Miniatures | Panneau **Pages** à gauche (bouton grille du rail droit) ; glisser une miniature pour réordonner |
| Signets | F6 ou bouton liste du rail droit |
| Rechercher | Ctrl+F : saisie, respecter la casse, ▲ ▼, surbrillance des occurrences |
| Sélectionner du texte | Outil flèche de la palette flottante, tracer un rectangle, puis Ctrl+C |
| Thème clair / sombre | Icône lune / soleil en haut à droite |
| Langue | Icône globe en haut à droite (français, anglais) |

## Annoter

La palette flottante en haut à gauche de la page propose : sélection (V), surligner (H), souligner (U),
barrer (X), note (N). Tracez un rectangle sur le texte pour appliquer un marquage ; cliquez sur la page avec
l'outil note pour saisir un commentaire. Le panneau **Commentaires** (F7) liste les annotations de la page
et permet de les supprimer. Toute annotation est annulable (Ctrl+Z).

## Éditer le contenu

La catégorie **Éditer** (et la seconde moitié de la palette flottante) modifie le contenu même des pages,
pas des annotations : sélectionner un objet (E), ajouter du texte (T), insérer une image (I), rectangle (R),
ellipse (O), ligne (L). Avec l'outil de sélection, cliquez sur un texte, une image ou un tracé : un cadre bleu
apparaît ; glissez-le pour le déplacer, appuyez sur **Suppr** pour le supprimer, sur **F2** (ou double-clic) pour
modifier un texte. Cliquez sur la page avec l'outil texte pour saisir un nouveau texte (taille, gras), avec
l'outil image pour choisir un fichier (PNG, JPEG, BMP, GIF, TIFF, WebP), tracez un rectangle pour une forme.
Toutes ces modifications sont annulables (Ctrl+Z) et écrites dans le PDF à l'enregistrement.

Limites : pas de redimensionnement ni de rotation, pas de choix de couleur ni de police ; le texte modifié
conserve sa police d'origine, qui peut ne pas contenir tous les caractères saisis.

## Organiser les pages

- **Organiser les pages** (F8) : grille des pages avec sélection multiple (clic, Ctrl+clic, Maj+clic),
  rotation, suppression, insertion depuis un autre PDF, déplacement avant/arrière, glisser-déposer.
- Depuis la vue normale : Ctrl+R / Ctrl+Maj+R pour pivoter, Ctrl+Suppr pour supprimer la page courante,
  Ctrl+Maj+I pour insérer des pages d'un autre fichier.
- **Annuler / Rétablir** : Ctrl+Z / Ctrl+Y (jusqu'à 50 étapes).
- **Diviser le document** : choisissez le nombre de pages par fichier et un dossier de destination.
- **Extraire les pages sélectionnées** : crée un nouveau PDF avec la sélection.
- **Combiner des fichiers PDF** : choisissez les fichiers dans l'ordre, puis le fichier de sortie ; il s'ouvre ensuite.

## Convertir

- **Créer un PDF à partir d'images** (PNG, JPEG, BMP, GIF, TIFF, WebP) : une page par image, 150 ppp.
- **Exporter les pages en images** : un PNG par page (150 ppp) dans le dossier choisi.
- **Optimiser la taille du fichier** : recompression sans perte ; le gain dépend du fichier et peut être faible.

## Sécurité

- **Protéger par mot de passe** : mot de passe d'ouverture, mot de passe propriétaire, permissions
  (impression, copie, modification, annotation), AES-256 par défaut (AES-128 pour les lecteurs anciens).
  La protection s'applique au fichier lors de l'enregistrement.
- **Retirer la protection** : disponible quand le document a été ouvert avec son mot de passe.
- **Supprimer les métadonnées** : retire le dictionnaire d'informations et les métadonnées XMP.
- **Propriétés du document** (Ctrl+D) : titre, auteur, sujet, mots-clés modifiables ; version, taille,
  chiffrement et permissions en lecture.

## Enregistrer

Ctrl+S enregistre dans le fichier ouvert, Ctrl+Maj+S dans un nouveau fichier. L'écriture est atomique :
le fichier d'origine n'est jamais laissé dans un état partiel. Un point après le nom de l'onglet signale
des modifications non enregistrées ; SuniPDF demande confirmation avant de fermer ou de quitter.

## Palette de commandes

Ctrl+K ouvre la palette : tapez quelques lettres, choisissez avec ↑ ↓, validez avec Entrée.

## Ligne de commande

`sunipdf-cli.exe --help` liste les commandes : `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `commands`. Les sorties sont en JSON ;
les codes de retour distinguent mot de passe requis (3), fichier invalide (4) et fichier illisible (5).

## Limites actuelles

Affichage page par page ; pas d'OCR, pas de conversion Office, pas de
signature numérique, pas d'impression directe. Voir `docs/ROADMAP.md`.
