# SuniPDF — Guide utilisateur

SuniPDF est un lecteur et éditeur PDF pour Windows 10/11. Le traitement des PDF reste local : aucun
compte, aucune télémétrie, aucun document envoyé sur Internet.

Ce guide intègre les changements SuniPDF 0.44.0 **en préparation et en cours de validation**. La précédente
version publiée reste [0.43.0](RELEASE_0.43.0.fr.md). État de publication et contrôles exécutés dans
[l’état du projet](PROJECT_STATUS.fr.md). Le [périmètre de préparation 0.44](RELEASE_0.44.0.fr.md) décrit
les entrées compatibles et les refus. L’anglais reste la langue par défaut ; le français est disponible
dans les réglages.

## Réutiliser des modèles locaux d’annotations

La bibliothèque d’annotations 0.45 est **en développement et en cours de validation** ; consulter son
[périmètre et son état de qualification](RELEASE_0.45.0.fr.md). Cliquez sur **Bibliothèque** dans la barre
principale, ou sur **Bibliothèque d’annotations** dans **Tous les outils** ou **Commenter**. Son fichier local
est distinct des PDF et des réglages.

Choisissez **Créer un modèle**, préparez un nouveau brouillon compatible puis développez **Enregistrer dans la bibliothèque**.
Saisissez un nom unique et cliquez sur **Enregistrer le modèle**. Crédit, source et licence sont des déclarations
facultatives ; une valeur vide reste inconnue. Enregistrer le modèle ne l’insère pas dans le PDF. La capture prend
en charge texte avec police standard, polygones/polylignes, nuages de révision, tampons prédéfinis et tampons image PNG.

Choisissez **Placer sur la page**, réglez l’échelle puis cliquez sur le PDF. Vérifiez l’aperçu rendu ; ajustez
**Origine X (pt)**, **Origine Y (pt)** ou **Échelle** si nécessaire, puis redemandez **Aperçu**. Cliquez sur
**Appliquer à l’objet** pour créer les annotations ou sur **Annuler** pour abandonner le placement. Un groupe
constitue une seule étape Annuler ; enregistrez le PDF pour le conserver. Pour composer un groupe, cochez au moins
deux entrées, choisissez **Créer un groupe**, renseignez un nouveau nom et les décalages des éléments, puis enregistrez.

Import/export, suppression de la sélection, rechargement et filtre sont disponibles dans la bibliothèque. Imports
invalides et écritures en conflit conservent son contenu actif. Le fichier de bibliothèque ne bénéficie pas de l’historique
Annuler du PDF ; exportez une copie avant de supprimer des modèles à conserver. Mesures, échelles de page, identités PDF
existantes et fils de revue sont exclus. [Notice complète, format, limites et exemples CLI](ANNOTATION_LIBRARY.fr.md).

## Mesurer directement sur un PDF

Ouvrez un PDF et cliquez sur **Mesures**, à côté de **Tous les outils** dans la barre principale.
Vous pouvez aussi choisir **Tous les outils → Mesures**, ou le raccourci Mesures dans **Commenter**.
Le bouton conserve son libellé dans les fenêtres étroites.

1. Choisissez l’unité (`mm`, `cm`, `m`, `in` ou `ft`) et 0 à 4 décimales. Cliquez sur **Calibrer cette page**,
   puis sur deux points distincts délimitant une distance connue. Saisissez la **Distance connue** réelle,
   puis cliquez sur **Confirmer l’échelle**. Pieds et pouces sont décimaux : 10 pieds 6 pouces correspondent
   à `10.5 ft` ou `126 in`.
2. Choisissez **Longueur**, **Polyligne**, **Périmètre**, **Surface polygonale** ou **Angle**, puis cliquez
   directement sur les points du PDF. Une longueur ouvre l’aperçu après deux points. Un angle utilise
   trois points dans cet ordre : premier côté, sommet, second côté ; aucune calibration physique n’est
   nécessaire. Terminez une polyligne ou un polygone avec **Prévisualiser la mesure**, **Entrée**, ou un
   double-clic sur le dernier point. Les polygones se ferment automatiquement : ne répétez pas le premier point.
3. Vérifiez l’aperçu PDF rendu et la valeur, puis cliquez sur **Appliquer**. **Annuler** ou **Échap**
   conserve le PDF intact. Enregistrez pour conserver l’annotation et sa calibration. **Annuler la dernière
   modification** retire l’annotation et ses données de mesure ensemble. L’outil reste actif pour la mesure suivante.

Pendant le tracé, **Retour arrière** ou **Annuler le point** retire le dernier point. **Maj** aligne un
segment horizontalement ou verticalement dans les axes de la page affichée, sans accrochage aux traits
existants. **Échap** ou **Annuler le tracé** abandonne le brouillon. Terminez ou annulez avant de changer
de page, de document ou d’outil.

Chaque page possède sa propre échelle de session. Les modifications ordinaires d’annotations et les
changements habituels d’onglet conservent les échelles confirmées ; les changements de géométrie, de
structure ou de contenu des pages les effacent. L'échelle de session est prioritaire sur l'échelle de page enregistrée.
Un changement d’unité convertit la distance confirmée. Les mesures enregistrées gardent leur propre
géométrie et échelle ; une nouvelle calibration ne modifie pas les mesures existantes.

Pour conserver l'échelle après fermeture/réouverture, cliquez sur **Enregistrer l’échelle dans le PDF**
après calibration, puis enregistrez le fichier PDF. L'échelle est réutilisée lorsqu'elle reste valide.
**Retirer l’échelle enregistrée** retire l'échelle par défaut de cette page, en conservant une calibration
de session choisie explicitement ; enregistrez le fichier pour conserver le retrait. **Annuler**/**Rétablir**
restaurent l'état du document et relisent son échelle ; ils ne restaurent pas une échelle de session effacée. Ajout et retrait exigent le droit de modification
du document ; créer une mesure exige le droit d'annotation. Rotation et réorganisation conservent une
échelle compatible. Limites de rognage ou contenu décodé modifiés, données incompatibles et `UserUnit`
non standard provoquent un refus ; recalibrez ou retirez une échelle périmée reconnue. Les échelles de
session non enregistrées disparaissent à la fermeture. Ces gardes n'authentifient pas toutes les révisions.

Ouvrez **Mesures enregistrées** dans la barre de mesure pour inspecter la page courante. La consultation
ne modifie pas le PDF. La liste présente valeur/unité, type, indice, identifiant éventuel, nombre de sommets, précision et
distance connue des mesures non angulaires, par groupes de 50. **Voir sur la page** sélectionne l'annotation réelle
et ouvre ses commentaires. **Utiliser cette échelle** copie la calibration d'une mesure non angulaire
dans la session de la page pour les prochaines mesures, sans changer les données existantes. Les points
de référence hors page sont refusés. **Enregistrer l’échelle dans le PDF** reste une action distincte.
**Actualiser** relit l'état courant ; fermez l'inspecteur avant de tracer ou de changer de document.

La CLI propose `page-calibration`, `set-page-calibration` et `remove-page-calibration`.
`measure --use-page-calibration` utilise une échelle enregistrée valide et la convertit vers `--unit` ;
les options manuelles `--calibration`/`--known-distance` sont alors exclues. Les écritures exigent une
nouvelle destination, même avec `--overwrite`. Pour les mots de passe, utilisez les options environnement
ou stdin. [Exemples et bornes](MEASUREMENTS.fr.md#cli-et-données).

En 0.44, cliquez sur **Modifier la mesure** dans l'inspecteur pour changer sa distance de référence,
son unité ou sa précision. Un changement d'unité conserve la taille physique ; une nouvelle distance
recalcule uniquement cette mesure. Pour les angles, seule la précision est proposée. Demandez un
**Aperçu**, vérifiez le candidat rendu, puis cliquez sur **Appliquer** ; **Annuler**
ne modifie rien. Une étape Annuler/Rétablir restaure l'ancienne/nouvelle mesure avec son identité et
ses commentaires liés. Enregistrez le PDF pour conserver la modification. Sommets et points de référence
restent fixes dans cet éditeur ; les cibles périmées ou incompatibles sont refusées.

**Utiliser l'échelle enregistrée** abandonne la calibration temporaire lorsqu'un défaut valide existe.
Sinon, **Effacer l'échelle temporaire** retire uniquement la calibration de session de la page. Ces
contrôles ne modifient pas le PDF. La saisie manuelle des annotations enrichies exige désormais une
calibration explicite au lieu de fournir une distance positive par défaut.

La CLI 0.44 ajoute `edit-measurement` : fournissez un `--expected-record` récent issu de `measurements`,
la page physique `--page`, l'indice d'annotation `--index` du record et une nouvelle copie `--out`.
Elle permet de remplacer explicitement les sommets ; le guide des mesures détaille références et
identité attendue. L'export exige le droit de copie. Une création/modification autorisée sans ce droit
renvoie `measurement: null` et `export_unavailable: true`. Le redimensionnement du contenu et des
annotations des pages mesurées est refusé avant toute application.

Recalcul global, déplacement des sommets à la souris, accrochage aux vecteurs et vues multi-échelles restent indisponibles. Vérifiez la distance connue et la révision du
plan : une valeur calculée ne garantit pas la justesse physique. Le [guide des mesures](MEASUREMENTS.fr.md)
détaille format ouvert, permissions et limites géométriques.

## Préparer une signature visuelle

Dans **Remplir et signer**, activer éventuellement **Lisser les traits** pour un dessin ou **Réduire les ombres
du papier** après le retrait du fond d’une photo importée. Comparer l’aperçu avant placement.
Les traits bruts et les fichiers image sources sont conservés. Ces marques sont visuelles ; la signature par
certificat est distincte. La CLI peut exporter un PNG séparé avec `prepare-signature-image`.
[Utilisation, limites de dessin et de nettoyage](VISUAL_SIGNATURE_PREPARATION.fr.md).

## Inspecter un PDF enregistré

Choisir **Fichier → Propriétés** (`Ctrl+D`), développer **Inspecter le PDF enregistré**, puis **Inspecter**.
Enregistrer les modifications d’abord ; si le fichier a changé hors de l’app, le recharger. Le rapport
affiche son SHA256 et les déclarations de JavaScript, actions automatiques/externes, fichiers embarqués,
XFA, multimédia et signatures. Les compteurs portent sur les dictionnaires correspondants, y compris
les objets inutilisés. Aucune action exécutée ni modification du PDF. Un rapport incomplet conserve
explicitement les compteurs partiels. Les brouillons de métadonnées sont appliqués uniquement par
**Appliquer**, indépendamment de l’inspection.

En automatisation : `sunipdf-cli inspect document.pdf`, avec `--password-env NOM` ou `--password-stdin`
si nécessaire. Code JSON `0` : inventaire borné terminé ; `2` : résultats partiels. Cet inventaire ne
garantit ni l’innocuité ni la conformité PDF/A/PDF/UA et ne vérifie pas l’intégrité des signatures.
Les contenus ne sont pas inspectés. Entrée limitée à 128 Mio ; objets et parcours ont aussi des limites.

## Nouveaux outils d’édition et de documents

- **Édition → Annotations enrichies** ouvre les contrôles de FreeText à police incorporée, polygones,
  polylignes, nuages, tampons et pièces jointes. Prévisualiser avant d’appliquer. La gomme retire
  les traits d’encre complets touchés ; Annuler conserve le PDF.
- L’édition de paragraphes prévisualise le réagencement avec la police PDF d’origine. Un glyphe ou un
  état graphique incompatible conserve le document et le brouillon. **Coller modifiable** importe
  les objets PDF compatibles séparément ; le collage groupé reste disponible.
- Les liens prennent en charge les destinations nommées existantes et Fit/XYZ/FitH. Un axe ou zoom
  vide conserve la vue du lecteur. Les coordonnées suivent la page cible affichée ; une position
  FitH tournée peut utiliser X au lieu de Y. Les actions externes dangereuses sont refusées.
- Le panneau Signets permet création, renommage, changement de destination, déplacement et suppression.
  Les étiquettes suivent le contenu réorganisé ; `#N` reste un numéro physique.
- La confirmation de caviardage propose un aperçu avant/après, une couleur et un texte de remplacement.
  **Enregistrer les marques** stocke des annotations `/Redact` visibles non appliquées : le contenu
  sensible demeure présent. Ce stockage exige actuellement un texte de remplacement vide. Appliquer
  utilise la copie exacte prévisualisée et une seule étape Annuler. Le texte extérieur reste sélectionnable
  lorsque sa reconstruction est prouvée sûre ; le rapport indique les replis en image.
- **Assainir** écrit une copie distincte et contrôle la suppression des données cachées compatibles.
  Les anciennes copies, récupérations, sauvegardes et l’historique Annuler sont à traiter séparément.
- Les conversions détectent LibreOffice local compatible déjà installé ; son absence est explicite.
  Profils Standard/Print et lots annulables écrivent de nouvelles copies. L’export PDF propose du texte
  simple modifiable en DOCX, des cellules de texte aligné en XLSX et des diapositives raster en PPTX.
  Voir les [limites de conversion](OFFICE_CONVERSION.fr.md).
- Les images de signature enregistrées facultativement utilisent DPAPI pour l’utilisateur Windows courant.
  Conserver l’image originale ou le profil et ses clés pour récupération. La vérification numérique
  sépare intégrité, confiance Windows, révocation et horodatages RFC3161. Le réseau exige un accord explicite ;
  les états hors ligne ou indisponibles ne valent pas validation réussie.

Exemples (pages et signets en base 1 ; indices d’annotation en base 0) :

```powershell
sunipdf-cli bookmark-add input.pdf --title "Chapitre 1" --page 2 --out bookmarked.pdf
sunipdf-cli link-destinations input.pdf
sunipdf-cli set-link-target input.pdf --index 0 --goto-page 2 --view xyz --y 300 --zoom 1.5 --out linked.pdf
sunipdf-cli redact input.pdf --text SECRET --color 000000 --out redacted.pdf
sunipdf-cli redact input.pdf --text SECRET --marks-only --out unapplied-marks.pdf
sunipdf-cli sanitize input.pdf --out sanitized.pdf
```

Consulter les notices [annotations](RICH_ANNOTATIONS.fr.md), [caviardage](REDACTION.fr.md),
[signets et structures](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md), [polices](FONT_EMBEDDING.fr.md),
[signatures](SIGNATURE_SECURITY.fr.md) et [assainissement](SECURITY_MODEL.fr.md) pour les limites exactes.

## Installation

- **Portable** : décompressez le fichier `SuniPDF-<version>-win-x64-portable.zip` où vous voulez et lancez
  `sunipdf.exe`. Aucune installation système ; préférences dans `%APPDATA%\SuniPDF\settings.json` et téléchargements de mise à jour dans un dossier temporaire.
- **Installateur MSI** : double-cliquez sur `SuniPDF-<version>-win-x64.msi`. SuniPDF apparaît dans le menu
  Démarrer et dans « Ouvrir avec » pour les fichiers PDF ; il ne devient pas le lecteur par défaut sans votre accord.
- Désinstallation : Paramètres Windows › Applications, ou suppression du dossier pour la version portable.

## Mises à jour et langue

L'anglais est la langue par défaut ; le français est disponible dans Paramètres. Une préférence existante est conservée.
À partir de la 0.8.2, l'application recherche une nouvelle version sur GitHub au démarrage sans envoyer de PDF.
La recherche automatique peut être désactivée dans À propos, où la recherche manuelle reste disponible.
Un échec de recherche automatique ne bloque pas le travail sur les PDF.

Choisissez **Télécharger la mise à jour**, puis **Fermer et installer** après vérification de la taille et du SHA256.
Enregistrez ou abandonnez les modifications de chaque document comme d'habitude ; annuler ou échouer à enregistrer annule l'installation.
Une version x64 installée utilise le MSI Windows, qui peut demander les droits administrateur. Les portables x64/ARM64 remplacent
les fichiers de l'application dans leur dossier et conservent une sauvegarde de retour arrière. Un échec restaure les anciens fichiers.
Les anciennes versions nécessitent une première mise à niveau manuelle. Le programme ne redémarre jamais automatiquement l'ordinateur.

## Écran d'accueil

L'accueil présente une grande carte **Ouvrir un fichier**, une carte par outil disponible (toute la carte est
cliquable) et les fichiers récents. Vous pouvez aussi glisser un PDF n'importe où dans la fenêtre, ou cliquer
sur **+ Ouvrir** dans la barre d'onglets.

## Tous les outils

Le panneau de gauche **Tous les outils** (F9, ou le bouton du même nom dans la barre d'outils) liste les
outils par tâche : Exporter un PDF, Remplir et signer, Modifier un PDF, Créer un PDF, Combiner
des fichiers, Organiser les pages, Mesures, Bibliothèque d’annotations, Commenter, Protéger un PDF, Caviarder un PDF, Compresser un PDF, Ajouter un tampon,
Numériser et OCR, Préparer un formulaire, Utiliser un certificat. Le menu **Outils** et
la palette Ctrl+K donnent les mêmes accès. Sans document ouvert, un outil qui en a besoin demande d'abord un
fichier puis démarre.

- **Exporter, Créer, Combiner, Compresser, Bibliothèque d’annotations** ouvrent directement leur boîte de dialogue. La bibliothèque peut être gérée sans PDF ouvert.
- **Remplir et signer, Modifier, Organiser, Mesures, Commenter, Protéger, Ajouter un tampon** (marqués d'un chevron ›) affichent une
  **barre d'outil** sous la barre d'outils : boutons de l'outil, groupés, avec leur libellé si la largeur le
  permet (sinon l'icône seule, le libellé restant en info-bulle). Les actions qui demandent une sélection
  (objet, texte, pages de la grille) restent grisées sans elle. Mesures conserve son libellé et présente
  ses contrôles de calibration et de tracé.
- **Quitter l'outil** : bouton à droite de la barre, **Échap**, nouveau clic sur l'outil actif dans le panneau,
  ou la flèche de retour (qui rouvre aussi le panneau). Échap ferme d'abord ce qui est plus proche : un menu
  ou une liste déroulante ouverts, puis la recherche, puis l'objet sélectionné, puis l'outil (un Échap, un
  seul effet). Un brouillon de mesure actif est annulé avant de quitter son outil.
- Les raccourcis à une lettre ouvrent l'outil correspondant : H, U, X, N, A, D, F ouvrent **Commenter** ; E, T, I, R, O, L
  ouvrent **Modifier** ; S ouvre la boîte **Signer** ; F8 ouvre **Organiser** avec la grille ; V revient au curseur de lecture.
- Chaque onglet garde son outil actif.
- La barre d'outils principale garde un accès rapide : Ouvrir, Enregistrer, Annuler, Rétablir, Mesures, Bibliothèque.

L’interface 0.45 ajoute **Bibliothèque d’annotations** aux côtés de **Mesures**, **Numériser et OCR**, **Préparer un formulaire** et
**Utiliser un certificat**. Remplir et signer conserve le remplissage et la signature visuelle ;
la signature numérique dispose de son propre outil.

## Onglets

Chaque PDF ouvert occupe un onglet dans la barre du haut. Cliquez sur un onglet pour l'afficher, sur sa croix
pour le fermer, sur la maison pour revenir à l'accueil sans fermer. Ctrl+Tab et Ctrl+Maj+Tab passent d'un
onglet à l'autre. Les modifications non enregistrées de chaque onglet sont signalées par un point et
confirmées une à une à la fermeture de l'application.

## Récupération automatique

La récupération automatique est activée par défaut. Un document modifié reçoit un instantané toutes les
60 secondes et à la suspension de son onglet, sans remplacer le PDF d'origine. Après un arrêt inattendu,
le démarrage suivant propose les instantanés disponibles ; **Récupérer des documents** permet d'y revenir.
Choisissez **Restaurer**, puis **Enregistrer sous** pour conserver le travail dans une nouvelle copie.
Un instantané protégé redemande le mot de passe PDF : le chiffrement reste présent et aucun mot de passe
n'est écrit dans les métadonnées. Le dossier est `%LOCALAPPDATA%\SuniPDF\recovery` ; les métadonnées
contiennent le chemin d'origine et le nombre de pages. Un abandon explicite ou un enregistrement réussi
retire la copie correspondante. Les modifications déjà appliquées sont couvertes, pas une saisie encore
non validée dans une boîte de dialogue ; les brouillons de formulaire valides sont inclus.
La case dans À propos désactive les instantanés automatiques. **Fichier → Récupérer des documents** rouvre la liste.

## Lire un document

Par défaut les pages sont **empilées** : la molette, la barre de défilement ou le clavier font défiler tout le
document, et la page qui occupe l'écran devient la page courante (numéro du rail, barre d'état, miniature,
panneau des commentaires ; Pivoter, Supprimer et Extraire la visent ; un clic sur une page la rend courante).
**F5**, le bouton à flèches du rail droit ou le menu Affichage basculent entre **Défilement continu** et **Page
unique** ; le choix est mémorisé. Seules les pages visibles sont rendues : un document de plusieurs centaines de
pages s'ouvre et défile sans rendre toutes les pages. Le menu Affichage propose aussi **Deux pages** (paires à partir
de la première page) et **Vue livre** (couverture seule, puis paires). Une dernière page non appariée est centrée.
Les tailles mixtes partagent un zoom ; l'ajustement utilise la plus large double page et la page la plus haute.
Ces modes conservent les mêmes plafonds de tuiles et de textures.

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
| Sélectionner du texte | Curseur de lecture (V) : glisser au fil du texte, y compris entre pages dans les deux sens, puis Ctrl+C. Double-clic : un mot. Près du bord haut/bas : défilement automatique |
| Thème clair / sombre | Icône lune / soleil en haut à droite |
| Langue | Icône globe en haut à droite (français, anglais) |

## Annoter

La sélection de lecture suit la séquence logique de caractères PDFium, normalise les fins de ligne et
conserve les apostrophes de mots comme `l'amour` et `l’amour`. L'ordre voulu de colonnes arbitraires non
balisées n'est pas déduit. Surlignage, soulignement et texte barré gardent leur geste de sélection rectangulaire.

L'outil **Commenter** réunit deux familles. Le marquage du texte : sélectionner du texte (V), surligner (H),
souligner (U), barrer (X), note (N) ; tracez un rectangle sur le texte pour appliquer un marquage, cliquez sur la
page avec l'outil note pour saisir un commentaire. Les **annotations avancées**, de vraies annotations PDF (pas
du contenu de page : les autres lecteurs les affichent aussi) :

| Outil | Geste |
|---|---|
| **Sélectionner une annotation** (A) | Clic sur une annotation : cadre, poignées, barre flottante. Glisser = déplacer ; poignées = redimensionner ; flèches = décaler (Maj : ×10) ; Suppr = supprimer ; double-clic ou bouton « Modifier le texte » = changer son texte ou son commentaire ; Échap = désélectionner |
| **Dessin à main levée** (D) | Tracer plusieurs traits sur une page ; couleur, épaisseur et opacité pour le groupe. Terminer le dessin = une annotation/un Annuler ; Annuler le dessin ou Échap avant envoi = abandonner le brouillon ; un clic = un point |
| **Zone de texte libre** (F) | Clic (boîte par défaut) ou glissement (boîte choisie), puis saisie dans l'éditeur multiligne ancré à la page. Régler taille, couleur, fond et bordure ; Ajouter ou Annuler. Pour modifier un texte libre, le sélectionner puis Modifier le texte/double-clic et Enregistrer ou Annuler. Boîte agrandie vers le bas si besoin. Apparence Helvetica WinAnsi ; texte exact conservé |
| **Rectangle, Ellipse, Ligne ou flèche** (annotations) | Glisser ; couleur, épaisseur, fond (rectangle et ellipse), opacité, case « Flèche » |
| **Lien** | Tracer une zone (au moins 6 points) puis choisir « Aller à une page » ou « Ouvrir une adresse web » (`http`, `https` ou `mailto` seulement) |

**Texte libre sur la page — à partir de 0.20.0.** Avec **Zone de texte libre** (F), cliquer pour une boîte
par défaut ou tracer son rectangle, puis saisir dans l'éditeur multiligne ancré. Régler taille, couleur du texte,
fond et bordure avec les contrôles existants (taille 6–96 pt, bordure 0–10 pt). **Ajouter** crée l'annotation. Pour modifier une annotation
FreeText, la sélectionner puis **Modifier le texte**, ou double-cliquer, et choisir **Enregistrer**.
**Annuler** ou Échap abandonne le brouillon sans modifier le PDF ; clic extérieur et Entrée ne le valident pas.

Valider ou annuler avant les actions incompatibles de page/navigation/document/onglet/outil/espace de travail. Le zoom conserve l'éditeur
sur la page d'origine ; révision du document/changement d'onglet/outil/espace de travail : édition périmée annulée.
Ouverture/enregistrement/mutation déjà en cours ou autre brouillon actif peuvent empêcher le démarrage.
Erreur de validation/worker : texte et style conservés pour réessayer. Texte non vide, rectangle fini
d'une largeur minimale de deux fois la taille et d'au moins 2 pt de haut. Ajouter/Enregistrer réussi produit une étape Annuler ;
enregistrer ensuite le PDF explicitement. Texte non validé en mémoire, absent de la récupération après plantage.

La boîte peut grandir vers le bas au renvoi à la ligne. Le rendu reste Helvetica WinAnsi (accents latins, € et
ponctuation courante) ; caractères non pris en charge affichés « ? », texte exact conservé dans `/Contents`.
Gras/italique, texte enrichi, CJK et mise en page droite à gauche restent indisponibles. Annotations verrouillées
et bordures FreeText malformées conservent les restrictions existantes. Cet éditeur ne modifie pas les objets texte de page.

**Dessin groupé — disponible en 0.18.0.** Tracer plusieurs traits sur une page, puis cliquer sur
**Terminer le dessin** pour créer une annotation et une étape Annuler, ou **Annuler le dessin** pour abandonner
ce brouillon. Échap annule aussi un brouillon non envoyé. Couleur, épaisseur et opacité modifient tout le
brouillon jusqu'à leur capture par Terminer. Terminer/annuler avant enregistrement/fermeture, changement de
document/outil ou édition incompatible ; navigation, zoom et modes de lecture conservent le brouillon sur sa
page d'origine. Une erreur du worker conserve tous les traits pour réessayer ou abandonner. Pendant la validation,
dessin supplémentaire, Terminer et Annuler sont bloqués.

Au plus 64 traits et 4 000 points par trait ; atteindre la limite de points arrête ce trait et permet d'en
commencer un autre. Un clic pose un point. Aucune validation à l'inactivité ; les traits non validés ne sont ni
du contenu PDF ni des instantanés de récupération. Simplification des points, géométrie et protection existantes conservées.
Contrôles locaux/CI/paquets/mises à jour réussis (D109) ; stylet/tactile natifs non vérifiés.

Une annotation créée par SuniPDF se redimensionne et se restyle (bouton « Appliquer à l'annotation » : les couleurs de
la barre lui sont appliquées) ; celle d'un autre logiciel se déplace, se commente et se supprime (un surlignage ou une
note se recolore aussi). Une annotation verrouillée
ne se modifie pas.

**Formes en tirets — disponibles en 0.16.0.** Pour les annotations Rectangle, Ellipse, Ligne ou Flèche,
choisissez **Bordure en tirets** avant de dessiner. Sur une forme SuniPDF existante modifiable, changez la case
puis **Appliquer à l'annotation**. Case cochée : tirets de 6 points, espaces de 3 points ; décochée : trait continu.
Remplissage continu et pointes de flèche pleines. Le motif personnalisé chargé reste conservé à la sélection,
recoloration, au déplacement/redimensionnement et à l'annulation ; changer la case le remplace explicitement
par le motif fixe ou continu. Aucun éditeur de motif personnalisé. Encre, texte libre, liens et marquages ne
proposent pas les tirets. Style malformé/non pris en charge : édition régénérant l'apparence refusée ; motif trop
dense ou tracé trop grand également refusable. Enregistrez explicitement pour garder les changements. Ces ajouts
sont inclus dans les paquets 0.16.0 publiés ; contrôles exécutés et limites dans l'[état du projet](PROJECT_STATUS.fr.md).

Un tableau de tirets explicitement vide signifie continu. Une bordure malformée désactive Déplacer/Redimensionner/
Style sur une forme SuniPDF ; son commentaire reste modifiable selon les permissions ordinaires. Le commentaire
d'un texte libre à bordure malformée est refusé car il change son apparence. Les petits tirets valides restent
dans l'aperçu ; un motif invalide/excessif omet le contour au lieu de montrer un trait continu.

**Liens cliquables.** Avec le curseur de lecture, le pointeur devient une main au-dessus d'un lien. Un lien
interne amène à la page ; un lien externe ouvre d'abord une **boîte de confirmation qui montre l'adresse
complète** (Ouvrir dans le navigateur, Copier l'adresse, Annuler) : rien ne s'ouvre sans votre accord, et seules les
adresses `http`, `https` et `mailto` sont acceptées. Les actions de lancement de programme ou de script ne sont
jamais exécutées.

**Modifier une cible existante — disponible en 0.15.0.** Avec **Sélectionner une annotation** (A), sélectionnez
le lien puis **Modifier la destination**, ou double-cliquez dessus. Choisissez une adresse `http`, `https` ou
`mailto`, ou un numéro de page physique. Une destination nommée/XYZ résolue préremplit sa page. Remplacer une
cible de page affiche la page entière ; position/zoom détaillés ne sont pas conservés, comme indiqué dans la boîte.
Une cible inconnue donne une adresse vide : entrez explicitement une destination valide. **Remplacer la destination**
conserve zone, apparence, nom, commentaires et réponses. Liens en lecture seule/verrouillés/contenu verrouillé
non modifiables ; drapeaux malformés également refusés. Annuler/Échap conserve la cible d'origine. Un Ctrl+Z annule
le changement ; enregistrez explicitement pour le garder. Autres éditions ou changement d'onglet ferment la boîte
pour éviter un indice d'annotation périmé. L'avertissement existant de réécriture des PDF signés s'applique.
Ces ajouts figurent dans les paquets 0.15.0 publiés ; contrôles exécutés et limites restantes dans l'[état du projet](PROJECT_STATUS.fr.md).

Le panneau **Commentaires** (F7) groupe les annotations de la page en **fils** : auteur, date, texte, statut de
relecture (Aucun, Accepté, Refusé, Annulé, Terminé), réponses en retrait. **Répondre** ajoute une réponse sans icône
sur la page, le menu de statut change l'état du fil, la corbeille supprime l'annotation et ses réponses, un clic sur
le titre d'une carte la sélectionne sur la page. Le champ **Auteur** (mémorisé dans vos préférences, vide par défaut)
est écrit dans les annotations, réponses et statuts créés ensuite. Fils et statuts suivent la norme PDF mais leur
affichage dans d'autres lecteurs PDF n'a pas été vérifié. Toute annotation est annulable (Ctrl+Z) ; un document protégé
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

Limites : le collage groupé reste un groupe ; **Coller modifiable** importe les objets individuels compatibles.
Les faces statiques TTF/OTF/TTC compatibles sont incorporées en sous-ensembles et réutilisées, sauf restriction
exigeant la face complète. La saisie ordinaire utilise l’aperçu de l’interface ; l’aperçu de paragraphe utilise
les métriques PDF. Le texte existant garde sa police ; un glyphe incompatible est refusé atomiquement.

**Livré en 0.21.1 :** remplacement incompatible refusé avant modification du PDF,
avec réouverture du bon brouillon sur la page ou du texte pivoté. Les
paquets 0.21.0 gardent la limite ci-dessus.

**Correctif d'aplatissement livré en 0.21.1 :** annotations non-widget sans apparence
utilisable conservant données brutes, pièces jointes et références partagées. Widgets/apparences dessinables
restent aplatis ; annotations conservées exclues du compte. Un échec du document temporaire garde le PDF
actif inchangé. Correctif livré en 0.21.1 ; anciens paquets 0.21.0 inchangés.
Voir les [contrôles exécutés et limites](PROJECT_STATUS.fr.md).

## Remplir un formulaire

Quand un PDF contient un formulaire (champs AcroForm), un bandeau « Ce document contient un formulaire à
remplir » apparaît au-dessus de la page avec le bouton **Remplir le formulaire** (aussi dans l'outil **Remplir
et signer**, le menu **Formulaires** et la palette). L'outil surligne chaque champ sur la page : bleu pour un
champ à remplir, **rouge** s'il est obligatoire, gris s'il est en lecture seule ou non modifiable (signature,
bouton). Les champs restent interactifs en défilement continu, sur toutes les pages visibles.

- **Case à cocher, bouton radio** : un clic, ou Espace/Entrée sur le widget focalisé, bascule l’état ; chaque clic est une étape d'annulation (Ctrl+Z).
- **Champ texte** : un clic ouvre un éditeur posé sur le champ ; **Entrée** valide (Ctrl+Entrée dans un champ
  multiligne, où Entrée passe à la ligne), **Échap** annule, **Tab** / **Maj+Tab** valident et passent au champ
  suivant / précédent (son contenu est sélectionné) ; un clic ailleurs valide aussi. La longueur maximale du
  champ est respectée.
- **Liste déroulante, zone de liste** : un clic ouvre la liste des choix (cases à cocher pour une liste à
  choix multiple ; « (aucun) » vide une liste non obligatoire).
- **Liste déroulante déjà modifiable (0.11.0)** : choisissez **Saisir une valeur…** dans la liste sur la page, puis
  confirmez comme un champ texte. Le panneau accepte aussi du texte libre à côté des choix, avec Appliquer/Abandonner/
  Enregistrer. Une liste ordinaire reste limitée aux choix.
- La valeur est dessinée par le moteur PDF lui-même (apparence régénérée) : ce que vous voyez est ce que les
  autres lecteurs afficheront, et ce qui est enregistré avec Ctrl+S.

Le panneau **Champs du formulaire** (Ctrl+Maj+F, bouton de la barre de l'outil) liste les champs de la page
courante ou de tout le document, avec un contrôle par champ (zone de texte, case, boutons radio, liste) utilisable
au clavier et par un lecteur d'écran ; un clic sur le nom d'un champ l'amène à l'écran. Il signale les champs
obligatoires et en lecture seule, un formulaire **XFA** (lu, jamais modifié) et la présence de **scripts** (calculs
et formats JavaScript : SuniPDF ne les exécute jamais, les champs calculés ne se mettent pas à jour).

Menu **Formulaires** : **Effacer les champs** (textes vidés, cases décochées, listes désélectionnées ; les champs
en lecture seule et les groupes radio « toujours un choix » sont conservés), **Exporter les valeurs…** (`.xfdf` ou `.json`), **Importer des valeurs…** (`.json`, `.xfdf` ou sous-ensemble `.fdf`).
Un champ inconnu ou une valeur invalide refuse tout l’import de l’interface, sans mutation partielle. Une valeur en lecture
seule se réimporte inchangée ; les mots de passe ne sont jamais exportés. Le panneau garde les modifications en attente :
**Appliquer les modifications** les applique en une annulation, **Abandonner** revient aux valeurs courantes et Enregistrer
applique puis sauvegarde. L’édition sur la page reste immédiate. **Aplatir le formulaire et les
annotations…** (toutes les pages ou la page courante : les valeurs, surlignages et notes deviennent du contenu
fixe ; les liens — adresses web, renvois vers une page — n'ont pas d'apparence et sont conservés tels quels ;
annulable par Ctrl+Z tant que le document est ouvert ; aplatir toutes les pages retire le formulaire du
document). Un champ **mot de passe** n'est ni remplissable ni exporté (sa valeur ne doit jamais être stockée). Un
document protégé par mot de passe se remplit comme les autres et reste protégé. Ctrl+S valide d'abord la saisie
en cours.

**0.11.0 :** un échec de lecture du formulaire affiche une erreur et un bouton **Réessayer**
dans le panneau des champs. Vos textes et modifications en attente restent disponibles ; l'échec n'est pas traité
comme un formulaire vide. Réessayer relit les champs courants avant un import ou un effacement en attente.
Aucune nouvelle tentative ne tourne en boucle.

**0.12.0 :** les champs radio avec `/Opt` utilisent leurs valeurs d'export dans le panneau et les
échanges JSON/XFDF/FDF, même si le nom d'état coché du widget PDF diffère. `form-fields` CLI conserve `on_state` et
ajoute `widgets[].export_value`. Une correspondance malformée ou ambiguë place ce champ en lecture seule ; les
autres restent remplissables. Sans `/Opt`, les radios gardent leurs valeurs fondées sur les noms d'état.

## Signer et parapher

L'outil **Remplir et signer** (barre : **Signer (signature visuelle)…** — touche **S** —, **Parapher
(initiales)…**, **Insérer la date du jour** ; aussi dans le menu Édition et la palette) pose une **signature
visuelle** sur la page. **Ce que cette signature ne garantit pas** : c'est une image, elle ne prouve ni votre
identité ni l'intégrité du document, et toute personne qui modifie le fichier peut la copier ou la retirer. Ce
n'est pas une preuve cryptographique. La boîte le rappelle en permanence. Le parcours de signature numérique
distinct se trouve dans **Utiliser un certificat**, décrit plus bas.

**Dessiner.** Dans la boîte, signez dans le pavé blanc à la souris, au pavé tactile ou au stylet (encre noire ou
bleue, épaisseur réglable, **Annuler le dernier trait**, **Effacer**). **Importer une image** accepte un PNG
transparent tel quel, ou une photo de votre signature sur papier clair : cochez **Supprimer le fond clair** et
ajustez le **seuil** en regardant l'aperçu (baissez-le si le fond reste visible, montez-le si des traits
disparaissent). **Placer sur la page** ferme la boîte ; un fantôme suit le pointeur, un **clic** pose la
signature (centrée sous le clic, maintenue dans la page), **Maj+clic** la pose plusieurs fois, **Échap**
annule.

**Saisir — disponible en 0.13.0.** Choisissez **Saisir**, entrez votre nom ou vos initiales, choisissez une
police standard ou TrueType installée et une encre noire ou bleue, puis cliquez sur **Aperçu**. Times italique
est la police par défaut. L'aperçu est rendu localement avant la pose ; changer texte, police ou encre demande un
nouvel aperçu. La saisie tient sur une ligne de 256 caractères au plus. Glyphes absents ou incorporation de police
interdite : une erreur est affichée. Le résultat est une image transparente, avec le même placement et
enregistrement facultatif que le dessin/import.

**Pages ciblées — disponible en 0.13.0.** Choisissez **Page courante**, **Toutes les pages** ou **Pages
sélectionnées** avant de placer. Pour une sélection, sélectionnez les pages dans **Organiser les pages**, puis
ouvrez Signer/Parapher ; sinon Pages sélectionnées reste désactivé. Un clic appose la même marque sur chaque
cible, au même centre en points PDF. Taille physique et proportions sont conservées sauf si une petite page
impose une réduction ; la pose est maintenue dans chaque page visible, y compris pivotée/rognée. Il ne s'agit pas
d'un placement proportionnel relatif à la taille des pages. Le lot affiche sa progression ; **Annuler** ou
**Échap** abandonne le lot entier, avec vérification entre les pages. Un **Ctrl+Z** annule le lot terminé.
Insertion/suppression/réorganisation des pages abandonne la boîte ou les cibles capturées : rouvrez-la après une
modification de structure. Enregistrez explicitement le document pour conserver les marques. Ces ajouts figurent
dans les paquets 0.13.0 publiés ; contrôles exécutés et limites dans l'[état du projet](PROJECT_STATUS.fr.md).

**Réutiliser.** Avec **Enregistrer sur cet ordinateur pour la réutiliser** (coché par défaut), la signature
rognée est protégée par DPAPI pour l’utilisateur courant dans `%APPDATA%\SuniPDF\signatures` et apparaît ensuite
dans la boîte : cliquez pour la placer ; la corbeille (deux clics) la supprime. La migration des anciens PNG
est vérifiée avant suppression du fichier initial. Aucune image n’est envoyée. Toute personne capable d’utiliser
votre session Windows ouverte peut réutiliser ces images ; décochez la case pour ne
rien enregistrer. Le **Paraphe** a sa propre liste, plus petite sur la page.

**Après la pose**, la signature est un **objet image** ordinaire de la page. La pose sur la page courante la sélectionne : glissez-la,
redimensionnez-la par les poignées (proportions conservées), pivotez-la, dupliquez-la, supprimez-la, Ctrl+Z
l'annule ; le contenu de la page reste visible entre les traits (fond transparent). Le lot ne sélectionne pas
toutes les images ajoutées ; sélectionnez les objets individuellement avec Modifier un PDF. **Insérer la date du jour**
pose un objet texte (`15/10/2026` en français, `10/15/2026` en anglais), modifiable par F2. Pour remplir un
champ en texte libre, utilisez l'outil texte (**T**) de Modifier un PDF.

## Tampons, filigranes et numéros de page

**Tampon / filigrane…** (outil **Ajouter un tampon**) appose un texte sur toutes les pages ou sur la page
courante : taille, couleur, gras, angle, opacité et position (centre ou l'un des six bords). **Numéroter les
pages…** ajoute un numéro selon un modèle : `{n}` est le numéro de la page, `{total}` le nombre de pages
(par exemple `Page {n} / {total}`). Chaque tampon est un objet de la page : il se sélectionne, se déplace, se
supprime et s'annule comme les autres.
Les textes de tampon et modèles de numérotation tiennent sur une seule ligne ; les sauts CR/LF sont refusés.

## Organiser les pages

- **Organiser les pages** (F8 ou l'outil du même nom) : grille des pages avec sélection multiple (clic,
  Ctrl+clic, Maj+clic) ; la barre de l'outil pivote, supprime, insère depuis un autre PDF, déplace
  avant/arrière, divise et extrait ; glisser-déposer pour réordonner.
- Depuis la vue normale : Ctrl+R / Ctrl+Maj+R pour pivoter, Ctrl+Suppr pour supprimer la page courante,
  Ctrl+Maj+I pour insérer des pages d'un autre fichier.
- **Annuler / Rétablir** : Ctrl+Z / Ctrl+Y (jusqu'à 50 étapes).
- **Diviser le document** : choisissez le nombre de pages par fichier et un dossier de destination.
- **Extraire les pages sélectionnées** : crée un nouveau PDF avec la sélection. Depuis un document
  protégé, le fichier et les parties d'une division **conservent chiffrement, mots de passe et permissions d'origine
  en 0.11.0**. Sortie résolue au fichier source refusée, même avec `--overwrite` en CLI ; politique des permissions inchangée.
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

## Commandes de lecture

Le menu **Affichage** propose rotation de vue par quarts de tour et plein écran, sans modifier la rotation des pages PDF.
L’activation d’un outil d’édition revient à une vue droite. Retour/Avance (Alt+Gauche/Alt+Droite) conserve 256 positions
maximum par onglet, avec page et position de défilement. Cet historique n’est pas du contenu PDF.

### Navigation 0.11.0

Les libellés PDF tels que `iv`, `1` ou `Annexe-É7` apparaissent avec les indices physiques dans les miniatures,
la barre d'état et les descriptions d'accessibilité. Saisissez un libellé exact, sensible à la casse et unique,
puis Entrée. Un libellé correspondant prime sur le numéro : si le libellé `1` désigne la page physique 3,
saisir `1` ouvre la page 3. Saisissez `#1` pour choisir explicitement la page physique 1. Sans libellé correspondant,
un nombre désigne la page physique. Une saisie inconnue, ambiguë ou hors limites conserve la page et affiche une explication.

La création d'étiquettes est disponible en 0.14.0, comme décrit plus bas. Aucune
renumérotation sémantique après insertion, suppression, déplacement ou extraction : les plages existantes peuvent
rester liées aux positions physiques, et les fichiers extraits peuvent perdre leurs libellés. Un libellé vide, illisible, contenant
un caractère de contrôle ou dépassant 4096 octets UTF-8 revient à l'indice physique. La commande CLI `info` fournit
`pages[].label` lorsqu'il existe ; tous les numéros de page CLI restent physiques et commencent à 1.

Les liens internes et signets suivent aussi les destinations PDF prises en charge : position et zoom `/XYZ`,
page entière `/Fit`, largeur `/FitH` avec une position exploitable. Recadrage et rotation sont pris en compte.
Un axe `/XYZ` omis ou un zoom nul/omis conserve la valeur courante ; le zoom reste dans les limites de l'application.
Alt+Gauche/Alt+Droite parcourt l'historique de page, position et zoom de l'onglet, y compris les destinations sur
la même page. Terminez les saisies de texte ou formulaire en attente avant de suivre une destination. Les modes
d'ajustement non pris en charge, les détails illisibles et une position `/FitH` nulle/absente reviennent à la page
cible sans appliquer le zoom ni la position de destination.

### Création d'étiquettes — disponible en 0.14.0

Ouvrez **Pages → Étiquettes de page…**, ou la commande Étiquettes de page d'**Organiser les pages**. Définissez
**Première page physique** et **Dernière page physique** : les deux bornes sont incluses. Choisissez décimal,
romain majuscule/minuscule, lettres majuscules/minuscules ou préfixe seul, entrez le préfixe facultatif et le
numéro de départ, puis **Appliquer**. Exemple romain : `i, ii, iii` ; les lettres suivent `A…Z, AA, BB…ZZ`,
selon PDF. Préfixe seul répète le même texte sur chaque cible. Départ par défaut à 1 et toujours positif ;
préfixe seul exige un texte non vide. Nouveaux préfixes sur une ligne de 256 octets UTF-8 au plus ; étiquettes
générées bornées à 4096 octets. Saisie invalide ou arbre existant malformé : Appliquer refusé sans changement partiel.

Les étiquettes hors plage conservent leur numérotation d'origine. **Réinitialiser tout** retire toutes les règles,
même un arbre malformé, et rétablit la navigation par indices physiques. Cela n'efface pas de manière sécurisée
les anciens objets de métadonnées. Aucun numéro imprimé n'est ajouté ou modifié : utilisez Numéroter les pages
pour cela. Appliquer/Réinitialiser sont annulables ; enregistrez explicitement pour conserver les changements.
L'avertissement existant de réécriture des documents signés s'applique aussi à ces métadonnées. Annuler/Rétablir
une édition de contenu/étiquettes conserve vue/navigation si la géométrie est inchangée ; une opération structurelle
continue à l'invalider.

Après insertion, suppression ou déplacement, les plages suivent les positions physiques sans recalage sémantique.
Vérifiez-les après réorganisation. Un doublon reste ambigu ; utilisez `#N` physique pour naviguer. Ces ajouts figurent
dans les paquets 0.14.0 publiés ; contrôles exécutés et limites dans l'[état du projet](PROJECT_STATUS.fr.md).

## Compression des images et lots

Ouvrez **Compresser un PDF**, choisissez le côté maximal (128..8192 px) et la qualité JPEG (10..100), puis une nouvelle
copie. Tailles réelles avant/après et annulation sont proposées ; la source reste intacte. Compression avec perte sur
images RGB/gris bornées ; gain non garanti, masques/profils/décodages particuliers conservés. PDF signés/restreints refusés.
Aucun fichier existant remplacé.

Choisissez **Traitement par lots**, compression ou OCR anglais/français, 1..128 PDF et un dossier de sortie. Chaque entrée
est limitée à 256 Mio. Progression/annulation et résultat par fichier ; noms `-compressed` ou `-ocr`. Les copies terminées
restent après erreur/annulation. Pas de demande de mot de passe par fichier : ceux qui en nécessitent un sont signalés.

## Convertir

- **Créer un PDF à partir d'images** (PNG, JPEG, BMP, GIF, TIFF, WebP) : une page par image, 150 ppp ;
  l'orientation EXIF des photos est respectée. Les huit orientations JPEG sont incorporées sans recompression.
- **Exporter les pages en images** : un PNG par page (150 ppp) dans le dossier choisi.
- **Optimiser la taille du fichier** : recompression sans perte ; le gain dépend du fichier et peut être faible.

**Disponible en 0.17.0 :** les huit orientations EXIF JPEG, dont 5 et 7, sont incorporées directement sans
recompression. Avant de décoder les autres formats, Images → PDF contrôle un budget raster cumulé estimé de
512 Mio : somme du maximum entre taille annoncée par chaque décodeur et `largeur × hauteur × 4` octets.
Contrôles répétés au chargement, tailles physiques invalides refusées. Qualité/transparence conservées ; ce budget
estime les données raster, pas la mémoire totale/native du processus, et ne limite pas les JPEG comprimés.
Usage UI et CLI `images2pdf` inchangé. Lot raster excessif : `image_batch_too_large` ; choisissez moins d'images
ou des images plus petites. Workspace local, qpdf/corpus et contrôle de conversion debug indépendant réussis ;
CI Windows, compilation ARM64 de même source, mises à niveau, paquets préparés/publics et metteur à jour de
production également réussis. Preuves et limites matérielles/d'interopérabilité dans l'[état du projet](PROJECT_STATUS.fr.md).

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

## Impression

Choisissez **Fichier → Imprimer** pour ouvrir la boîte native Windows. Sélectionnez l'imprimante, la plage,
la page courante ou les pages sélectionnées, les copies et l'orientation du pilote. L'état courant du document,
y compris les modifications appliquées, annotations et valeurs de formulaire, est imprimé en pages tramées
ajustées à la zone imprimable. La progression propose l'annulation ; un appel au pilote déjà lancé peut tarder à s'arrêter.
Le rendu utilise jusqu'à 300 dpi par bandes bornées. Le pilote peut modifier l'échelle ou les couleurs.
La sortie réelle Microsoft Print to PDF est couverte par un test automatisé. Imprimantes physiques et boîtes
interactives du pilote restent à vérifier sur le poste cible.

## Créer des champs de formulaire

Ouvrez **Préparer un formulaire**, puis choisissez texte, case à cocher, groupe radio, liste déroulante ou zone de liste.
Nom unique sans point/contrôle ; rectangle en points PDF affichés, dans la page et de 4 points minimum par dimension.
Pour les choix, une ligne `valeur exportée | libellé affiché` par option, valeurs uniques. La zone de liste peut autoriser
plusieurs sélections. Le rectangle radio contient les boutons verticaux ; libellés dans le panneau, pas comme texte ajouté
à la page PDF. Création annulable, chiffrement conservé. Création XFA non prise en charge.

**Disponible en 0.19.0 :** **Dessiner sur la page** masque cette boîte ; tracer un rectangle
sur une page la rouvre en conservant nom, choix et options. Relisez le rectangle numérique, puis choisissez
**Créer le champ** ; le tracé seul ne crée aucun champ. Le rectangle reste dans une page et mesure au moins
4 points PDF par dimension. Glissement inverse accepté ; clic simple ou rectangle minuscule : mode dessin
conservé pour réessayer. Relâcher hors page borne le rectangle à la page initiale. **Échap** annule la boîte sans écriture.
Révision du document/changement d'onglet/outil/espace de travail : placement périmé annulé ; zoom/navigation restent
possibles, avec finalisation au relâchement si un saut en page unique masque l'origine. Dans ce cas, le dernier rectangle
connu revient dans la boîte numérique pour relecture ; seul **Créer le champ** le crée. Création numérique conservée.
Aucun nouveau type ni éditeur de champs existants. Souris/stylet/tactile natifs et Narrateur restent distincts.

**0.12.0 :** pour une liste déroulante, cochez **Autoriser du texte libre en plus des options proposées**
pour accepter les choix et des valeurs saisies. Gardez au moins un choix prédéfini avec une valeur d'export unique
et non vide. Ce réglage concerne les nouvelles listes ; changer le drapeau d'un champ existant n'est pas proposé.

FDF accepte les valeurs texte/nom/tableau et tables xref ordinaires ; flux/actions/pièces jointes/texte enrichi refusés.
Les chemins de documents référencés ne sont jamais ouverts.

## OCR hors ligne

Ouvrez **Numériser et OCR**, choisissez anglais, français ou les deux, puis la page courante ou toutes les pages.
Tesseract 5.5.3, Leptonica 1.87 et les modèles rapides `eng`/`fra` embarqués fonctionnent localement, sans
téléchargement de modèle ni envoi du PDF. Les pages ayant déjà du texte extractible sont ignorées. Les mots reconnus
forment une couche invisible recherchable et copiable, sans changer l'aspect de la page.
La reconnaissance vise 300 dpi, réduits sur les grandes pages (16 millions de pixels, 8192 pixels par côté) ;
les pages trop grandes sont refusées. La progression avance par page. Annulation ou erreur laisse le document intact ;
une opération réussie forme un seul niveau d'annulation. Enregistrez le résultat et relisez le texte reconnu avant utilisation.

## Signatures par certificat

**Utiliser un certificat → Vérifier les signatures** contrôle les octets du PDF enregistré et affiche le résultat,
le sujet, l'émetteur, les dates de validité et l'empreinte SHA256 du certificat. Les statuts distinguent révision signée
valide, modification postérieure, contenu invalide, format non pris en charge et signature non vérifiable.
L’intégrité CMS seule ne prouve ni la confiance de l’identité ni la révocation. La vérification expose ces
décisions et les horodatages RFC3161 compatibles séparément, hors ligne par défaut. Créer une copie signée
n’ajoute ni horodatage fiable ni preuve d’archivage LTV. Voir la [sécurité des signatures](SIGNATURE_SECURITY.fr.md).
L'heure affichée est celle déclarée par le signataire.

Enregistrez d'abord les modifications, puis choisissez **Signer avec un certificat**, l'identité PFX/P12, son mot de passe,
un motif/lieu éventuel et un nouveau PDF de sortie. Une révision CMS incrémentale est ajoutée sans réécrire les octets
précédents. La source n'est jamais remplacée ; les PDF chiffrés ne peuvent pas être signés.
Modifier et enregistrer un PDF signé peut invalider ses signatures : ne confirmez l'avertissement de réécriture que
si c'est voulu. Les copies de la clé et du mot de passe sont effacées des buffers après usage ; SuniPDF n'enregistre
pas le mot de passe et n'importe pas la clé dans un magasin persistant.

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
Les sorties sont en JSON. En 0.11.0, `extract` et `split` conservent la protection : `encryption_removed` reste `false`
et l'ajout `encrypted` indique si les copies sont chiffrées. Sortie résolue au chemin source refusée même avec `--overwrite` ;
les codes de retour distinguent mot de passe requis (3), fichier invalide (4),
fichier illisible (5) et fichier de sortie déjà existant (6). Aucune commande n'écrase un fichier existant
sans l'option `--overwrite`. `render` écrit toujours du PNG (`--region x,y,l,h` : une zone en pixels de la page
rendue à `--scale`, pour les échelles où la page entière dépasserait 16 384 px) ; `rotate` accepte 90, 180, 270 ou -90. Les
coordonnées (`search`, `objects`, `add-text`…) sont en points, origine en bas à gauche de la page **telle
qu'affichée** (après rotation et recadrage).
`add-text` exige des coordonnées `--x` et `--y` finies. La CLI `annotate --kind freetext` accepte
uniquement `--opacity 1` ; les autres opacités d'annotation doivent être finies. Les valeurs invalides
sont refusées avant l'enregistrement, y compris avec `--overwrite`.

**Disponible en 0.13.0 :** `add-image --pages all` ou `--pages "1,3-5"` place l'image une fois par page
physique sélectionnée. Cette option exclut `--page` explicite ; sans les deux, le placement historique reste sur
une page par défaut. `--x`, `--y` et `--width` décrivent le coin bas-gauche et la taille demandés en points PDF
affichés. Le lot ajuste cette demande dans chaque page et conserve la protection source ; la pose historique
sur une page reste inchangée. Le JSON de lot ajoute `pages` et `count` ; coordonnées/taille décrivent la demande,
pas chaque rectangle ajusté. Exemple :

```powershell
sunipdf-cli add-image input.pdf --image signature.png --pages "1,3-5" --x 100 --y 40 --width 150 --out initialled.pdf
```

**Disponible en 0.14.0 :** `set-page-labels` accepte `--from` (1 par défaut), `--to` (dernière page par défaut),
`--style decimal|roman-upper|roman-lower|letters-upper|letters-lower|prefix-only` (décimal par défaut), `--start`
(1 par défaut) et `--prefix` (vide par défaut). `reset-page-labels` retire toutes les règles. Les deux commandes
acceptent `--out` et `--password` facultatif, conservent chiffrement/permissions et les protections contre
l'écrasement. Une saisie invalide préserve une sortie existante, même avec `--overwrite`. Les arguments de pages
restent physiques à partir de 1 ; `info` relit les étiquettes via `pages[].label`.

```powershell
sunipdf-cli set-page-labels input.pdf --from 1 --to 4 --style roman-lower --out labelled.pdf
sunipdf-cli set-page-labels labelled.pdf --from 5 --style decimal --prefix "Annexe-" --start 1 --out appendix.pdf
sunipdf-cli info appendix.pdf
sunipdf-cli reset-page-labels appendix.pdf --out unlabelled.pdf
```

**Disponible en 0.15.0 :** `set-link-target` utilise `--page` (page physique 1 par défaut) et `--index`
obligatoire (indice d'annotation à partir de 0, retourné par `annotations`). Fournissez exactement une option
`--goto-page N` ou `--url ADRESSE`, plus `--out` et `--password` facultatif. Une cible de page utilise Fit ; seules
les adresses `http`, `https`, `mailto` sont acceptées. Protections de sortie existantes conservées ; une saisie
invalide préserve la sortie existante, même avec `--overwrite`. Rectangle/style/commentaires du lien et protection
du document sont conservés.

```powershell
sunipdf-cli annotations input.pdf --page 1
sunipdf-cli set-link-target input.pdf --page 1 --index 0 --url "https://example.org/updated" --out updated-link.pdf
sunipdf-cli set-link-target updated-link.pdf --page 1 --index 0 --goto-page 2 --out page-link.pdf
```

**Disponible en 0.16.0 :** ajoutez `--dashed` à `annotate --kind square|circle|line|arrow` pour le motif fixe
`[6,3]` points. `annotations` renvoie le tableau réel `dash_pattern`. Sans ce drapeau, comportement continu
existant inchangé. Drapeau refusé pour les types non pris en charge ; requête invalide : aucune sortie créée ou
sortie existante préservée, même avec `--overwrite`. Consulter `sunipdf-cli --help` pour la liste actuelle des commandes.

```powershell
sunipdf-cli annotate input.pdf --kind square --rect 40,40,240,140 --dashed --out dashed.pdf
sunipdf-cli annotations dashed.pdf --page 1
```

Nouvelles commandes 0.9.0 :

```powershell
sunipdf-cli add-form-field input.pdf --kind text --name FullName --page 1 --rect 40,40,220,65 --out form.pdf
sunipdf-cli add-form-field form.pdf --kind checkbox --name Consent --rect 30,30,50,50 --out form-with-checkbox.pdf
sunipdf-cli ocr scan.pdf --language eng+fra --pages 1,3-5 --out searchable.pdf
sunipdf-cli signatures saved.pdf
sunipdf-cli sign saved.pdf --certificate identity.pfx --certificate-password-env SUNIPDF_CERT_PASSWORD --out signed-copy.pdf
```

Pour `sign`, définissez la variable d'environnement de manière sûre avant l'exécution, ou choisissez
`--certificate-password-stdin` avec une entrée UTF-8 redirigée. Les options s'excluent et l'une est obligatoire ;
une fin de ligne finale est retirée de l'entrée standard. Aucun argument de mot de passe de certificat en clair
n'existe. `--reason` et `--location` sont facultatifs. Le JSON sépare intégrité, confiance et révocation ;
`identity_trusted` exige la réussite de la politique de confiance. La vérification est hors ligne par défaut ;
`--allow-network` autorise explicitement les échanges de certificats pour ce contrôle.
Même `--overwrite` ne peut pas remplacer le PDF source de la signature. `ocr --language` accepte `eng`, `fra`
ou `eng+fra` ; `--pages` vaut `all` par défaut. Sans texte ajouté, les octets source sont copiés à l'identique.

Commandes ajoutées en 0.10.0 :

```powershell
sunipdf-cli compress-images scan.pdf --max-image-side 1800 --jpeg-quality 75 --out scan-compressed.pdf
sunipdf-cli add-form-field formulaire.pdf --kind combo --name Pays --option FR=France --option BE=Belgique --rect 40,40,220,65 --out choix.pdf
sunipdf-cli form-export choix.pdf --out valeurs.xfdf
sunipdf-cli fill-form choix.pdf --values valeurs.xfdf --out rempli.pdf
```

`rotate-objects` et `resize-objects` transforment un groupe ; `--help` décrit les options effectives.
`compress-images` exige toujours une nouvelle copie, même avec `--overwrite`.

Dans la CLI **0.12.0**, ajoutez `--editable` à `add-form-field --kind combo` pour accepter le
texte libre ; les autres types refusent ce drapeau. Des choix prédéfinis restent requis.

## Limites actuelles

Depuis 0.30.1, préférez `--password-env NOM` ou `--password-stdin` avec une entrée UTF-8 redirigée.
Les nouveaux mots de passe de protection acceptent aussi `--user-password-env`/`--user-password-stdin`
et `--owner-password-env`/`--owner-password-stdin`. Une seule lecture stdin par appel ; une fin de ligne
finale est retirée. Définissez les variables depuis une source de secrets adaptée puis effacez-les.
Les anciens arguments restent visibles dans les outils de processus/historique. Remplacer un chiffrement
demande le mot de passe propriétaire actuel. `signatures` imprime son JSON complet puis retourne 2 en
cas d'échec ; un PDF sans signature conserve un listing réussi. [Limites de sécurité](SECURITY_MODEL.fr.md).

Conversion Office locale bornée comme décrit plus haut ; formulaires XFA modifiables indisponibles et scripts jamais exécutés.
L'ordre complexe de colonnes n'est pas déduit. Création texte/case/radio/listes disponible ; FDF limité au sous-ensemble de valeurs décrit ci-dessus.
La précision OCR dépend de la numérisation et de la langue ; le texte existant n'est pas remplacé. Confiance Windows,
révocation et horodatages RFC3161 compatibles ont des états explicites ; validation d’archivage LTV et qualification
AATL/EUTL restent indisponibles. Imprimante réelle, fenêtres natives interactives et matériel ARM64
restent des validations séparées des tests automatisés. Voir `docs/KNOWN_ISSUES.fr.md` et `docs/ROADMAP.fr.md`.


## Comparaison locale et recherche dans plusieurs PDF (0.31.0)

Utilisez **Fichier → Comparer des fichiers PDF…** ou **Édition → Rechercher dans des fichiers PDF…**. Enregistrez les modifications avant de sélectionner les fichiers. Comparez les pages physiques avec des positions initiales ajustables ; cliquez sur une paire modifiée pour ses trois aperçus. Un résultat de recherche ouvre sa page physique. Opérations locales, en lecture seule et annulables. Un résultat partiel ne valide jamais les fichiers restants. Consultez le [guide complet et ses limites](LOCAL_REVIEW.fr.md) pour les fichiers protégés, scans, commandes JSON et résolution.

## Réorganiser les objets par glisser-déposer

Ouvrez le panneau Objets avec F3. Le haut de la liste correspond au premier plan. Glissez une ligne au-dessus ou au-dessous d’une autre ; le trait bleu indique la destination. Glisser une ligne sélectionnée déplace toute la sélection en un bloc, dans son ordre relatif d’origine. Glisser une ligne non sélectionnée déplace cet objet uniquement.

Échap annule. Un dépôt hors d’une ligne valide, sur une ligne sélectionnée ou à la même position ne fait rien. Un dépôt dans une autre page/document ou après une modification intermédiaire est refusé. La sélection suit les objets déplacés. Les boutons avancer/reculer et commandes clavier existants restent disponibles ; un déplacement terminé utilise une seule étape Annuler et le Rétablir de la transaction existante.

[Guide](OBJECT_ORDER.fr.md).
