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
  ou la flèche de retour (qui rouvre aussi le panneau). Échap ferme d'abord ce qui est plus proche : un menu
  ou une liste déroulante ouverts, puis la recherche, puis l'objet sélectionné, puis l'outil (un Échap, un
  seul effet).
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
ellipse (O), ligne (L). Tout fonctionne aussi sur les pages pivotées ou recadrées : les objets tombent là où
vous cliquez et un texte ajouté est toujours droit à l'écran.

**Sélectionner.** Avec l'outil de sélection, cliquez sur un texte, une image ou un tracé (ou appuyez dessus et
glissez directement pour le déplacer). **Maj+clic** ou **Ctrl+clic** ajoute ou retire un objet de la sélection ;
glisser sur une zone vide trace un **rectangle de sélection** (objets entièrement inclus ; **Alt** : objets
simplement touchés) ; **Ctrl+A** sélectionne tous les objets de la page. Un groupe sélectionné se déplace d'un
bloc (souris ou flèches), se supprime, se duplique, passe au premier plan ou à l'arrière-plan et reçoit les
mêmes couleurs et la même opacité.

**Transformer.** Glissez la sélection pour la déplacer, tirez une poignée d'angle pour redimensionner (images :
proportions conservées, Maj pour les libérer ; autres objets : Maj pour les verrouiller), tirez la **poignée
ronde** au-dessus du cadre pour pivoter librement (Maj : pas de 15°, l'angle s'affiche dans la barre d'état) ou
saisissez un **angle** dans la barre d'objet. Les flèches décalent d'un point (Maj : dix points), **Suppr**
supprime, Échap désélectionne. La barre de l'outil propose aussi pivoter de 90°, dupliquer (Ctrl+Maj+D),
premier plan (Ctrl+]) et arrière-plan (Ctrl+[).

**Barre d'objet.** Sous la sélection, une barre flottante règle le trait (couleur, épaisseur), le remplissage,
l'**opacité** (0 à 100 %) et l'angle ; **Appliquer à l'objet** applique couleurs et opacité. Sans sélection, avec
un outil de forme ou d'image, elle règle les prochaines formes et images. Elle porte aussi Copier, Couper,
Coller et, pour une image, **Remplacer l'image** (même cadre, nouvelle image ajustée et centrée).

**Copier, couper, coller.** Ctrl+C / Ctrl+X / Ctrl+V (ou le menu Édition) copient des objets d'une page à une
autre, et d'un onglet à un autre. Les objets collés forment un **groupe** : il se déplace, se redimensionne,
pivote et se supprime, mais son texte n'est plus modifiable comme texte. Coller au même endroit décale chaque
copie de 12 points.

**Texte.** Cliquez sur la page avec l'outil texte : un champ s'ouvre **sur la page**, à la taille du zoom, avec
une mini-barre (taille, couleur, police, gras, italique). **Entrée** valide, **Maj+Entrée** passe à la ligne,
**Échap** annule, un clic ailleurs valide. La liste des polices propose Helvetica, Times et Courier, puis les
**polices TrueType du système** (recherche par nom, par exemple « Segoe UI » ou « Arial Bold ») : elles
permettent le cyrillique, le grec ou le latin étendu ; la police est **incorporée en entier** dans le PDF
(la mini-barre indique sa taille). Si la police choisie ne contient pas un caractère saisi (par exemple du
cyrillique avec Helvetica), SuniPDF le signale et rouvre le champ au lieu de perdre le caractère. Pour un
texte existant droit à l'écran, **F2** ou double-clic ouvre le même champ sur la page (contenu, taille,
couleur ; sa police d'origine est conservée) ; un texte pivoté s'édite dans une boîte de dialogue.

**Panneau Objets** (F5 ou l'icône calques du rail droit, ou le bouton de la barre de l'outil) : liste des objets
de la page, du plus en avant au plus en arrière. Clic : sélectionner (Ctrl : ajouter ou retirer, Maj : ajouter) ;
survol : repère orange sur la page ; boutons avancer / reculer d'un plan et supprimer.

Toutes ces modifications sont annulables (Ctrl+Z) et écrites dans le PDF à l'enregistrement.

Limites : un groupe collé ou dupliqué n'est plus éditable comme texte ; rotation et redimensionnement d'un
groupe de plusieurs objets ne sont pas encore proposés (déplacement seulement) ; les polices `.otf` et `.ttc`
ne sont pas proposées ; la police est incorporée entière (pas de sous-ensemble) ; le champ de saisie sur la
page utilise la police de l'interface comme aperçu (largeurs approximatives) ; le texte modifié conserve sa
police d'origine, qui peut ne pas contenir tous les caractères saisis.

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
- **Insérer une page vierge…** : taille de la page courante, A3, A4, A5, Letter, Legal ou personnalisée
  (portrait ou paysage), nombre de pages, avant ou après la page courante ou à la fin.
- **Recadrer les pages…** : marges gauche, droite, haute et basse en millimètres, avec un aperçu sur le rendu
  de la page ; page courante (ou sélection de la grille) ou toutes les pages ; **Rétablir le cadre d'origine**
  annule un recadrage. Le recadrage **masque** les bords : le contenu hors cadre reste dans le fichier (ce
  n'est pas un caviardage).
- **Redimensionner les pages…** : format (A3 à Legal ou personnalisé, paysage) ; le contenu est soit conservé
  tel quel (ancré en haut à gauche), soit mis à l'échelle et centré. Les annotations et liens existants ne
  suivent pas la mise à l'échelle.

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
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `stamp`, `page-numbers`, `commands`, et pour
l'édition du contenu : `objects` (objets d'une page, index à partir de 0), `add-text` (`--font-name "Arial"` ou
`--font-file` pour une police système incorporée ; `\n` pour passer à la ligne), `set-opacity`, `rotate-object`
(angle libre), `copy-objects` (vers une autre page, `--into` pour un autre document), `replace-image`, `crop`
(`--unit mm`, `--reset`), `resize-pages` (`--mode keep|scale`), `insert-blank`, `fonts`.
Les sorties sont en JSON ; les codes de retour distinguent mot de passe requis (3), fichier invalide (4),
fichier illisible (5) et fichier de sortie déjà existant (6). Aucune commande n'écrase un fichier existant
sans l'option `--overwrite`. `render` écrit toujours du PNG ; `rotate` accepte 90, 180, 270 ou -90. Les
coordonnées (`search`, `objects`, `add-text`…) sont en points, origine en bas à gauche de la page **telle
qu'affichée** (après rotation et recadrage).

## Limites actuelles

Affichage page par page ; pas d'OCR, pas de conversion Office, pas de
signature numérique, pas d'impression directe. Voir `docs/ROADMAP.md`.
