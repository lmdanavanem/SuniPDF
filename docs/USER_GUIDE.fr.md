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
outils par tâche : Exporter un PDF, Remplir et signer, Modifier un PDF, Créer un PDF, Combiner
des fichiers, Organiser les pages, Commenter, Protéger un PDF, Compresser un PDF, Ajouter un tampon. Le menu **Outils** et
la palette Ctrl+K donnent les mêmes accès. Sans document ouvert, un outil qui en a besoin demande d'abord un
fichier puis démarre.

- **Exporter, Créer, Combiner, Compresser** ouvrent directement leur boîte de dialogue.
- **Remplir et signer, Modifier, Organiser, Commenter, Protéger, Ajouter un tampon** (marqués d'un chevron ›) affichent une
  **barre d'outil** sous la barre d'outils : boutons de l'outil, groupés, avec leur libellé si la largeur le
  permet (sinon l'icône seule, le libellé restant en info-bulle). Les actions qui demandent une sélection
  (objet, texte, pages de la grille) restent grisées sans elle.
- **Quitter l'outil** : bouton à droite de la barre, **Échap**, nouveau clic sur l'outil actif dans le panneau,
  ou la flèche de retour (qui rouvre aussi le panneau). Échap ferme d'abord ce qui est plus proche : un menu
  ou une liste déroulante ouverts, puis la recherche, puis l'objet sélectionné, puis l'outil (un Échap, un
  seul effet).
- Les raccourcis à une lettre ouvrent l'outil correspondant : H, U, X, N, A, D, F ouvrent **Commenter** ; E, T, I, R, O, L
  ouvrent **Modifier** ; S ouvre la boîte **Signer** ; F8 ouvre **Organiser** avec la grille ; V revient au curseur de lecture.
- Chaque onglet garde son outil actif.
- La barre d'outils principale garde un accès rapide : Ouvrir, Enregistrer, Annuler, Rétablir.

Les outils qui n'ont pas encore de fonction réelle dans SuniPDF (OCR, préparation de
formulaire — création de champs —, certificat) n'apparaissent pas : ils arriveront avec leur fonctionnalité.
Remplir et signer couvre le remplissage des formulaires AcroForm et la signature visuelle.

## Onglets

Chaque PDF ouvert occupe un onglet dans la barre du haut. Cliquez sur un onglet pour l'afficher, sur sa croix
pour le fermer, sur la maison pour revenir à l'accueil sans fermer. Ctrl+Tab et Ctrl+Maj+Tab passent d'un
onglet à l'autre. Les modifications non enregistrées de chaque onglet sont signalées par un point et
confirmées une à une à la fermeture de l'application.

## Lire un document

Par défaut les pages sont **empilées** : la molette, la barre de défilement ou le clavier font défiler tout le
document, et la page qui occupe l'écran devient la page courante (numéro du rail, barre d'état, miniature,
panneau des commentaires ; Pivoter, Supprimer et Extraire la visent ; un clic sur une page la rend courante).
**F5**, le bouton à flèches du rail droit ou le menu Affichage basculent entre **Défilement continu** et **Page
unique** ; le choix est mémorisé. Seules les pages visibles sont rendues : un document de plusieurs centaines de
pages s'ouvre et défile sans tout charger.

| Action | Comment |
|---|---|
| Défiler | Molette ; flèches ↑ ↓ (une ligne) ; Espace / Maj+Espace et Page suiv. / Page préc. (un écran ; en page unique, Page suiv. / préc. changent de page) |
| Page suivante / précédente | Flèches → ←, boutons du rail droit |
| Première / dernière page | Origine / Fin |
| Aller à une page | Saisir le numéro dans la case du rail droit puis Entrée, ou cliquer une miniature : la page vient en haut de l'écran |
| Zoom | Ctrl + molette (zoom **autour du pointeur** ; le pincement du pavé tactile passe par le même chemin, non testé), Ctrl + / Ctrl − (autour du centre), Ctrl 0 (taille réelle), liste de zoom (25 à 1600 %), « Largeur », « Page » ; jusqu'à 3200 % à la molette, la netteté est conservée (rendu en tuiles) |
| Défilement continu / page unique | F5, bouton du rail droit, menu Affichage |
| Miniatures | Panneau **Pages** à gauche (bouton grille du rail droit) ; glisser une miniature pour réordonner |
| Signets | F6 ou bouton liste du rail droit |
| Rechercher | Ctrl+F : saisie, respecter la casse, ▲ ▼, surbrillance des occurrences |
| Sélectionner du texte | Tracer un rectangle sur le texte (curseur de lecture, V), puis Ctrl+C |
| Thème clair / sombre | Icône lune / soleil en haut à droite |
| Langue | Icône globe en haut à droite (français, anglais) |

## Annoter

L'outil **Commenter** réunit deux familles. Le marquage du texte : sélectionner du texte (V), surligner (H),
souligner (U), barrer (X), note (N) ; tracez un rectangle sur le texte pour appliquer un marquage, cliquez sur la
page avec l'outil note pour saisir un commentaire. Les **annotations avancées**, de vraies annotations PDF (pas
du contenu de page : les autres lecteurs les affichent aussi) :

| Outil | Geste |
|---|---|
| **Sélectionner une annotation** (A) | Clic sur une annotation : cadre, poignées, barre flottante. Glisser = déplacer ; poignées = redimensionner ; flèches = décaler (Maj : ×10) ; Suppr = supprimer ; double-clic ou bouton « Modifier le texte » = changer son texte ou son commentaire ; Échap = désélectionner |
| **Dessin à main levée** (D) | Glisser sur la page : trait lissé (un trait = une annotation, un clic = un point) ; couleur, épaisseur et opacité dans la barre flottante |
| **Zone de texte libre** (F) | Clic (boîte par défaut) ou glissement (boîte choisie), puis saisie dans la boîte de dialogue : taille, couleur, fond, bordure. La boîte grandit vers le bas si le texte est plus haut. Police Helvetica : les caractères hors alphabet latin s'affichent « ? », le texte exact reste dans l'annotation |
| **Rectangle, Ellipse, Ligne ou flèche** (annotations) | Glisser ; couleur, épaisseur, fond (rectangle et ellipse), opacité, case « Flèche » |
| **Lien** | Tracer une zone (au moins 6 points) puis choisir « Aller à une page » ou « Ouvrir une adresse web » (`http`, `https` ou `mailto` seulement) |

Une annotation créée par SuniPDF se redimensionne et se restyle (bouton « Appliquer à l'annotation » : les couleurs de
la barre lui sont appliquées) ; celle d'un autre logiciel se déplace, se commente et se supprime (un surlignage ou une
note se recolore aussi) mais n'est jamais réécrite, pour ne rien perdre de sa fidélité. Une annotation verrouillée
ne se modifie pas.

**Liens cliquables.** Avec le curseur de lecture, le pointeur devient une main au-dessus d'un lien. Un lien
interne amène à la page ; un lien externe ouvre d'abord une **boîte de confirmation qui montre l'adresse
complète** (Ouvrir dans le navigateur, Copier l'adresse, Annuler) : rien ne s'ouvre sans votre accord, et seules les
adresses `http`, `https` et `mailto` sont acceptées. Les actions de lancement de programme ou de script ne sont
jamais exécutées.

Le panneau **Commentaires** (F7) groupe les annotations de la page en **fils** : auteur, date, texte, statut de
relecture (Aucun, Accepté, Refusé, Annulé, Terminé), réponses en retrait. **Répondre** ajoute une réponse sans icône
sur la page, le menu de statut change l'état du fil, la corbeille supprime l'annotation et ses réponses, un clic sur
le titre d'une carte la sélectionne sur la page. Le champ **Auteur** (mémorisé dans vos préférences, vide par défaut)
est écrit dans les annotations, réponses et statuts créés ensuite. Fils et statuts suivent la norme PDF mais leur
affichage dans Acrobat ou Foxit n'a pas été vérifié. Toute annotation est annulable (Ctrl+Z) ; un document protégé
par mot de passe le reste après annotation, et les pages pivotées ou recadrées sont respectées.

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

**Panneau Objets** (F3 ou l'icône calques du rail droit, ou le bouton de la barre de l'outil) : liste des objets
de la page courante, du plus en avant au plus en arrière. Clic : sélectionner (Ctrl : ajouter ou retirer, Maj : ajouter) ;
survol : repère orange sur la page ; boutons avancer / reculer d'un plan et supprimer.

Toutes ces modifications sont annulables (Ctrl+Z) et écrites dans le PDF à l'enregistrement.

Limites : un groupe collé ou dupliqué n'est plus éditable comme texte ; rotation et redimensionnement d'un
groupe de plusieurs objets ne sont pas encore proposés (déplacement seulement) ; les polices `.otf` et `.ttc`
ne sont pas proposées ; la police est incorporée entière (pas de sous-ensemble) ; le champ de saisie sur la
page utilise la police de l'interface comme aperçu (largeurs approximatives) ; le texte modifié conserve sa
police d'origine, qui peut ne pas contenir tous les caractères saisis.

## Remplir un formulaire

Quand un PDF contient un formulaire (champs AcroForm), un bandeau « Ce document contient un formulaire à
remplir » apparaît au-dessus de la page avec le bouton **Remplir le formulaire** (aussi dans l'outil **Remplir
et signer**, le menu **Formulaires** et la palette). L'outil surligne chaque champ sur la page : bleu pour un
champ à remplir, **rouge** s'il est obligatoire, gris s'il est en lecture seule ou non modifiable (signature,
bouton). Les champs restent interactifs en défilement continu, sur toutes les pages visibles.

- **Case à cocher, bouton radio** : un clic bascule l'état ; chaque clic est une étape d'annulation (Ctrl+Z).
- **Champ texte** : un clic ouvre un éditeur posé sur le champ ; **Entrée** valide (Ctrl+Entrée dans un champ
  multiligne, où Entrée passe à la ligne), **Échap** annule, **Tab** / **Maj+Tab** valident et passent au champ
  suivant / précédent (son contenu est sélectionné) ; un clic ailleurs valide aussi. La longueur maximale du
  champ est respectée.
- **Liste déroulante, zone de liste** : un clic ouvre la liste des choix (cases à cocher pour une liste à
  choix multiple ; « (aucun) » vide une liste non obligatoire).
- La valeur est dessinée par le moteur PDF lui-même (apparence régénérée) : ce que vous voyez est ce que les
  autres lecteurs afficheront, et ce qui est enregistré avec Ctrl+S.

Le panneau **Champs du formulaire** (Ctrl+Maj+F, bouton de la barre de l'outil) liste les champs de la page
courante ou de tout le document, avec un contrôle par champ (zone de texte, case, boutons radio, liste) utilisable
au clavier et par un lecteur d'écran ; un clic sur le nom d'un champ l'amène à l'écran. Il signale les champs
obligatoires et en lecture seule, un formulaire **XFA** (lu, jamais modifié) et la présence de **scripts** (calculs
et formats JavaScript : SuniPDF ne les exécute jamais, les champs calculés ne se mettent pas à jour).

Menu **Formulaires** : **Effacer les champs** (textes vidés, cases décochées, listes désélectionnées ; les champs
en lecture seule et les groupes radio « toujours un choix » sont conservés), **Exporter les valeurs (JSON)…**
(fichier `{ "nom du champ": valeur }`), **Importer des valeurs (JSON)…** (même format ; les champs inconnus et les
valeurs refusées sont ignorés et comptés dans le message d'état ; un fichier exporté se réimporte tel quel, les
champs en lecture seule y figurent avec leur valeur et ne sont pas réécrits), **Aplatir le formulaire et les
annotations…** (toutes les pages ou la page courante : les valeurs, surlignages et notes deviennent du contenu
fixe ; les liens — adresses web, renvois vers une page — n'ont pas d'apparence et sont conservés tels quels ;
annulable par Ctrl+Z tant que le document est ouvert ; aplatir toutes les pages retire le formulaire du
document). Un champ **mot de passe** n'est ni remplissable ni exporté (sa valeur ne doit jamais être stockée). Un
document protégé par mot de passe se remplit comme les autres et reste protégé. Ctrl+S valide d'abord la saisie
en cours.

## Signer et parapher

L'outil **Remplir et signer** (barre : **Signer (signature manuscrite)…** — touche **S** —, **Parapher
(initiales)…**, **Insérer la date du jour** ; aussi dans le menu Édition et la palette) pose une **signature
visuelle** sur la page. **Ce que cette signature ne garantit pas** : c'est une image, elle ne prouve ni votre
identité ni l'intégrité du document, et toute personne qui modifie le fichier peut la copier ou la retirer. Ce
n'est **pas** une signature numérique certifiée (à venir avec l'outil Utiliser un certificat). La boîte le
rappelle en permanence.

**Dessiner.** Dans la boîte, signez dans le pavé blanc à la souris, au pavé tactile ou au stylet (encre noire ou
bleue, épaisseur réglable, **Annuler le dernier trait**, **Effacer**). **Importer une image** accepte un PNG
transparent tel quel, ou une photo de votre signature sur papier clair : cochez **Supprimer le fond clair** et
ajustez le **seuil** en regardant l'aperçu (baissez-le si le fond reste visible, montez-le si des traits
disparaissent). **Placer sur la page** ferme la boîte ; un fantôme suit le pointeur, un **clic** pose la
signature (centrée sous le clic, maintenue dans la page), **Maj+clic** la pose plusieurs fois, **Échap**
annule.

**Réutiliser.** Avec **Enregistrer sur cet ordinateur pour la réutiliser** (coché par défaut), la signature
rognée est écrite en PNG dans `%APPDATA%\SuniPDF\signatures` et apparaît ensuite dans la boîte, sur fond
blanc : un clic la place, la corbeille (deux clics) la supprime. Rien n'est chiffré ni envoyé sur Internet :
toute personne ayant accès à votre session Windows peut réutiliser ces fichiers ; décochez la case pour ne
rien enregistrer. Le **Paraphe** a sa propre liste, plus petite sur la page.

**Après la pose**, la signature est un **objet image** ordinaire de la page, aussitôt sélectionné : glissez-la,
redimensionnez-la par les poignées (proportions conservées), pivotez-la, dupliquez-la, supprimez-la, Ctrl+Z
l'annule ; le contenu de la page reste visible entre les traits (fond transparent). **Insérer la date du jour**
pose un objet texte (`15/10/2026` en français, `10/15/2026` en anglais), modifiable par F2. Pour remplir un
champ en texte libre, utilisez l'outil texte (**T**) de Modifier un PDF.

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
- **Extraire les pages sélectionnées** : crée un nouveau PDF avec la sélection. Depuis un document
  protégé, le fichier créé (comme les parties d'une division) **n'est pas chiffré** : la barre d'état le
  signale ; ouvrez-le et appliquez **Protéger** si besoin.
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

- **Créer un PDF à partir d'images** (PNG, JPEG, BMP, GIF, TIFF, WebP) : une page par image, 150 ppp ;
  l'orientation EXIF des photos est respectée et les JPEG sont incorporés sans recompression.
- **Exporter les pages en images** : un PNG par page (150 ppp) dans le dossier choisi.
- **Optimiser la taille du fichier** : recompression sans perte ; le gain dépend du fichier et peut être faible.

## Sécurité

- **Protéger par mot de passe** : mot de passe d'ouverture, mot de passe propriétaire, permissions
  (impression, copie, modification, annotation), AES-256 par défaut (AES-128 pour les lecteurs anciens).
  La protection s'applique au fichier lors de l'enregistrement. AES-128 n'accepte que des mots de passe en
  caractères Latin-1 (lettres accentuées comprises, mais ni €, ni œ, ni les tirets et guillemets
  typographiques) ; pour un mot de passe cyrillique, grec, CJK, avec emoji ou avec ces symboles, gardez
  AES-256. L'extraction de texte pour l'accessibilité (lecteurs d'écran) reste toujours permise.
- **Retirer la protection** : demande le mot de passe propriétaire (ou le mot de passe d'ouverture quand
  aucun mot de passe propriétaire distinct n'a été défini). Des restrictions (impression, copie…) exigent
  un mot de passe propriétaire différent du mot de passe d'ouverture.
- Les opérations sur un document protégé (métadonnées, optimisation, ordre des objets) **conservent** sa
  protection.
- **Supprimer les métadonnées** : retire le dictionnaire d'informations, les métadonnées XMP (document,
  pages, images) et les données privées des applications (`PieceInfo`). Modifier les propriétés met aussi à
  jour le XMP.
- **Propriétés du document** (Ctrl+D) : titre, auteur, sujet, mots-clés modifiables ; version, taille,
  chiffrement et permissions en lecture.
- **Caviarder un PDF** (touche B) : tracez des rectangles sur ce qui doit disparaître, ou **Caviarder un
  texte…** pour marquer toutes les occurrences d'un texte ; un clic sur une zone la retire, **Effacer les zones**
  les retire toutes. Rien n'est supprimé tant que vous n'avez pas choisi **Appliquer le caviardage…**
  (Ctrl+Maj+B, puis le bouton rouge ou Ctrl+Entrée). Le texte, les images, les tracés, les annotations et les
  champs situés sous les zones sont alors **supprimés du document** (pas seulement masqués) ; ce qui dépasse
  d'une zone reste visible sous forme d'image. SuniPDF vérifie ensuite qu'aucun texte n'est encore extractible
  sous les zones, sinon rien n'est modifié : cochez alors « Convertir entièrement les pages caviardées en
  image ». Ctrl+Z annule tant que le document est ouvert ; le fichier n'est modifié qu'à l'enregistrement (un
  enregistrement avec des zones non appliquées demande confirmation). Limites : le texte d'une image numérisée
  sans OCR ne se trouve pas par recherche, les signets et pièces jointes ne sont pas modifiés, une signature
  numérique devient invalide (détail : `docs/KNOWN_ISSUES.md` A40). En ligne de commande :
  `sunipdf-cli redact entree.pdf --text "Jean Dupont" --rect "1:72,500,272,560" --out caviarde.pdf`.

## Enregistrer

Ctrl+S enregistre dans le fichier ouvert, Ctrl+Maj+S dans un nouveau fichier. L'écriture est atomique :
le fichier d'origine n'est jamais laissé dans un état partiel (données écrites sur le disque avant le
remplacement). Un point après le nom de l'onglet signale des modifications non enregistrées ; annuler jusqu'à
l'état enregistré l'efface ; SuniPDF demande confirmation avant de fermer ou de quitter.

## Palette de commandes

Ctrl+K ouvre la palette : tapez quelques lettres, choisissez avec ↑ ↓, validez avec Entrée. Chaque
commande porte la couleur de son outil ; les entrées « Modifier un PDF », « Commenter »… ouvrent l'outil.

## Ligne de commande

`sunipdf-cli.exe --help` liste les commandes : `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `stamp`, `page-numbers`, `commands`, pour
les annotations : `annotations` (liste JSON : indice, sous-type, rectangle en points de page affichée, texte, auteur, style, lien, fil), `annotate` (`--kind ink|square|circle|line|arrow|freetext|link`, `--rect gauche,bas,droite,haut`, `--stroke "x,y x,y …"` répétable, `--text`, `--color`, `--fill`, `--width`, `--opacity`, `--goto-page` ou `--url`, `--author`), `reply`, `review-state` (`none|accepted|rejected|cancelled|completed`) et `annotation-delete` (`--thread`), et pour
l'édition du contenu : `objects` (objets d'une page, index à partir de 0), `add-text` (`--font-name "Arial"` ou
`--font-file` pour une police système incorporée ; `\n` pour passer à la ligne), `set-opacity`, `rotate-object`
(angle libre), `copy-objects` (vers une autre page, `--into` pour un autre document), `replace-image`, `crop`
(`--unit mm`, `--reset`), `resize-pages` (`--mode keep|scale`), `insert-blank`, `fonts`, et `add-image`
(`--image signature.png --x 100 --y 520 --width 150`, `--remove-background 200` pour une photo sur fond clair :
une image posée sur la page, par exemple une signature scannée ; la réponse porte `"digital_signature": false`,
ce n'est jamais une signature numérique). Formulaires : `form-fields` (champs, types, valeurs, options,
widgets avec pages en base 1 ; `--values-only` pour le seul objet `{ nom: valeur }`), `fill-form` (`--values
fichier.json` ou `-` pour l'entrée standard, `--set nom=valeur` répétable — case : `true`/`false`, zone de liste :
choix séparés par `;` —, strict par défaut : champ inconnu ou valeur refusée = erreur `form_field_not_found` /
`form_value_invalid` sans rien écrire, `--ignore-unknown` pour ignorer et compter, `--flatten`) et `flatten`
(`--pages all` ou `1,3-5` ; toutes les pages = formulaire retiré). Un formulaire XFA renvoie `form_xfa`.
Les sorties sont en JSON (`extract` et `split` signalent `encryption_removed` quand la source était protégée) ;
les codes de retour distinguent mot de passe requis (3), fichier invalide (4),
fichier illisible (5) et fichier de sortie déjà existant (6). Aucune commande n'écrase un fichier existant
sans l'option `--overwrite`. `render` écrit toujours du PNG (`--region x,y,l,h` : une zone en pixels de la page
rendue à `--scale`, pour les échelles où la page entière dépasserait 16 384 px) ; `rotate` accepte 90, 180, 270 ou -90. Les
coordonnées (`search`, `objects`, `add-text`…) sont en points, origine en bas à gauche de la page **telle
qu'affichée** (après rotation et recadrage).

## Limites actuelles

Pas de double page ni de rotation de la vue ; une sélection de texte reste sur sa page (pas de sélection à
cheval sur deux pages, pas de défilement automatique en glissant au bord) ; en défilement continu, « Largeur »
et « Page » ajustent la plus grande page du document. Pas d'OCR, pas de conversion Office, pas de signature
numérique (la signature manuscrite est une image sans valeur cryptographique), pas de création de champs de
formulaire ni de formulaires XFA modifiables (lecture seule ; les scripts des formulaires ne sont jamais
exécutés), pas d'impression directe. Voir `docs/ROADMAP.md`.
