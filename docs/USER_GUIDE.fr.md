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

L'accueil présente une grande carte **Ouvrir un fichier**, une carte par outil disponible (toute la carte est
cliquable) et les fichiers récents. Vous pouvez aussi glisser un PDF n'importe où dans la fenêtre, ou cliquer
sur **+ Ouvrir** dans la barre d'onglets.

## Tous les outils

Le panneau de gauche **Tous les outils** (F9, ou le bouton du même nom dans la barre d'outils) liste les
outils par tâche, comme PDFCraft : Exporter un PDF, Modifier un PDF, Créer un PDF, Combiner des fichiers,
Organiser les pages, Commenter, Protéger un PDF, Compresser un PDF, Ajouter un tampon. Le menu **Outils** et
la palette Ctrl+K donnent les mêmes accès. Sans document ouvert, un outil qui en a besoin demande d'abord un
fichier puis démarre.

- **Exporter, Créer, Combiner, Compresser** ouvrent directement leur boîte de dialogue.
- **Modifier, Organiser, Commenter, Protéger, Ajouter un tampon** (marqués d'un chevron ›) affichent une
  **barre d'outil** sous la barre d'outils : boutons de l'outil, groupés, avec leur libellé si la largeur le
  permet (sinon l'icône seule, le libellé restant en info-bulle). Les actions qui demandent une sélection
  (objet, texte, pages de la grille) restent grisées sans elle.
- **Quitter l'outil** : bouton à droite de la barre, **Échap**, nouveau clic sur l'outil actif dans le panneau,
  ou la flèche de retour (qui rouvre aussi le panneau). Échap ferme d'abord ce qui est plus proche : la
  recherche, puis l'objet sélectionné, puis l'outil.
- Les raccourcis à une lettre ouvrent l'outil correspondant : H, U, X, N ouvrent **Commenter** ; E, T, I, R, O, L
  ouvrent **Modifier** ; F8 ouvre **Organiser** avec la grille ; V revient au curseur de lecture.
- Chaque onglet garde son outil actif.
- La barre d'outils principale garde un accès rapide : Ouvrir, Enregistrer, Annuler, Rétablir.

Les outils PDFCraft qui n'ont pas encore de fonction réelle dans SuniPDF (remplir et signer, OCR, caviardage,
préparation de formulaire, certificat) n'apparaissent pas : ils arriveront avec leur fonctionnalité.

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
| Sélectionner du texte | Tracer un rectangle sur le texte (curseur de lecture, V), puis Ctrl+C |
| Thème clair / sombre | Icône lune / soleil en haut à droite |
| Langue | Icône globe en haut à droite (français, anglais) |

## Annoter

L'outil **Commenter** propose : sélectionner du texte (V), surligner (H), souligner (U), barrer (X),
note (N), le panneau Commentaires et la copie du texte sélectionné. Tracez un rectangle sur le texte pour appliquer un marquage ; cliquez sur la page avec
l'outil note pour saisir un commentaire. Le panneau **Commentaires** (F7) liste les annotations de la page
et permet de les supprimer. Toute annotation est annulable (Ctrl+Z).

## Éditer le contenu

L'outil **Modifier un PDF** modifie le contenu même des pages,
pas des annotations : sélectionner un objet (E), ajouter du texte (T), insérer une image (I), rectangle (R),
ellipse (O), ligne (L). Avec l'outil de sélection, cliquez sur un texte, une image ou un tracé : un cadre bleu
apparaît ; glissez-le pour le déplacer, tirez une poignée d'angle pour le redimensionner, appuyez sur **Suppr**
pour le supprimer, sur **F2** (ou double-clic) pour modifier un texte (contenu, taille, couleur). Cliquez sur la
page avec l'outil texte pour saisir un nouveau texte (police Helvetica / Times / Courier, gras, italique, taille,
couleur), avec l'outil image pour choisir un fichier (PNG, JPEG, BMP, GIF, TIFF, WebP ; le point cliqué devient
le coin haut-gauche), tracez un rectangle pour une forme. La petite barre en haut à gauche de la page règle la couleur et
l'épaisseur du trait et le remplissage des nouvelles formes ; **Appliquer à l'objet** recolore l'objet
sélectionné. Toutes ces modifications sont annulables (Ctrl+Z) et écrites dans le PDF à l'enregistrement.

Quand un objet est sélectionné, la barre de l'outil active aussi : modifier le texte (F2), pivoter à gauche / à droite (90°), dupliquer
(Ctrl+Maj+D), mettre au premier plan (Ctrl+]) ou à l'arrière-plan (Ctrl+[), supprimer. Les flèches du
clavier décalent l'objet d'un point (Maj : dix points), Échap le désélectionne. Les images gardent leurs
proportions au redimensionnement (Maj pour les libérer) ; les autres objets sont libres (Maj pour
verrouiller).

Limites : pas de rotation à angle libre ; polices limitées aux 14 polices standard ; le texte modifié
conserve sa police d'origine, qui peut ne pas contenir tous les caractères saisis.

## Tampons, filigranes et numéros de page

**Tampon / filigrane…** (outil **Ajouter un tampon**) appose un texte sur toutes les pages ou sur la page
courante : taille, couleur, gras, angle, opacité et position (centre ou l'un des six bords). **Numéroter les
pages…** ajoute un numéro selon un modèle : `{n}` est le numéro de la page, `{total}` le nombre de pages
(par exemple `Page {n} / {total}`). Chaque tampon est un objet de la page : il se sélectionne, se déplace, se
supprime et s'annule comme les autres.

## Organiser les pages

- **Organiser les pages** (F8 ou l'outil du même nom) : grille des pages avec sélection multiple (clic,
  Ctrl+clic, Maj+clic) ; la barre de l'outil pivote, supprime, insère depuis un autre PDF, déplace
  avant/arrière, divise et extrait ; glisser-déposer pour réordonner.
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
- **Retirer la protection** : demande le mot de passe propriétaire (ou le mot de passe d'ouverture quand
  aucun mot de passe propriétaire distinct n'a été défini). Des restrictions (impression, copie…) exigent
  un mot de passe propriétaire différent du mot de passe d'ouverture.
- Les opérations sur un document protégé (métadonnées, optimisation, ordre des objets) **conservent** sa
  protection.
- **Supprimer les métadonnées** : retire le dictionnaire d'informations et les métadonnées XMP.
- **Propriétés du document** (Ctrl+D) : titre, auteur, sujet, mots-clés modifiables ; version, taille,
  chiffrement et permissions en lecture.

## Enregistrer

Ctrl+S enregistre dans le fichier ouvert, Ctrl+Maj+S dans un nouveau fichier. L'écriture est atomique :
le fichier d'origine n'est jamais laissé dans un état partiel. Un point après le nom de l'onglet signale
des modifications non enregistrées ; SuniPDF demande confirmation avant de fermer ou de quitter.

## Palette de commandes

Ctrl+K ouvre la palette : tapez quelques lettres, choisissez avec ↑ ↓, validez avec Entrée. Chaque
commande porte la couleur de son outil ; les entrées « Modifier un PDF », « Commenter »… ouvrent l'outil.

## Ligne de commande

`sunipdf-cli.exe --help` liste les commandes : `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `stamp`, `page-numbers`, `commands`.
Les sorties sont en JSON ; les codes de retour distinguent mot de passe requis (3), fichier invalide (4),
fichier illisible (5) et fichier de sortie déjà existant (6). Aucune commande n'écrase un fichier existant
sans l'option `--overwrite`. `render` écrit toujours du PNG ; `rotate` accepte 90, 180, 270 ou -90.

## Limites actuelles

Affichage page par page ; pas d'OCR, pas de conversion Office, pas de
signature numérique, pas d'impression directe. Voir `docs/ROADMAP.md`.
