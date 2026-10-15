# Journal des versions

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions sont des tags `vX.Y.Z` ;
`release.yml` construit et publie les artefacts (ZIP portable x64/ARM64, MSI x64) sur la page Releases GitHub.

## [Non publié]

(rien pour l'instant)

## [0.5.0] — 2026-10-15

Vague 2 du backlog, chaque tranche implémentée, revue par un agent indépendant, corrigée puis re-revue :
correctifs des 39 constats de la revue adverse, signature visuelle (moitié « signer » de Remplir et signer),
formulaires AcroForm (moitié « remplir »), chiffrement AES-256 et mots de passe normalisés, distribution.

### Corrigé (revue de la tranche forms)
- **Aplatir supprimait les liens** (adresses web, renvois vers une page) : `FPDFPage_Flatten` retire tout
  `/Annots` et un lien n'a pas d'apparence. Les liens des pages aplaties sont relevés avant (lopdf) et remis
  dans `/Annots` après, **copiés en profondeur** (PDFium n'enregistre pas l'action `/A` indirecte d'un lien
  qu'il a supprimé : références inlinées, pages traduites, flux réajoutés) ; le compte d'annotations renvoyé les
  exclut. Fixture `acroform-links.pdf`, test `flatten_keeps_link_annotations`, texte de la boîte Aplatir et
  guides mis à jour.
- L'**éditeur posé sur un champ texte** et la **liste de choix** suivaient leur widget hors de la zone du document
  (dessinés par-dessus la barre d'onglets, clics capturés) : découpés à la zone visible, validés et fermés dès que
  le widget en sort entièrement ; la liste reste dans la zone du document.
- **Aller-retour `form-fields --values-only` → `fill-form --values`** : la valeur courante d'un champ en lecture
  seule est acceptée sans être réécrite (CLI, import JSON, moteur) ; même règle dans l'interface (« 0 ignorée »).
- Champ **mot de passe** : ni surligné, ni ouvert, ni exporté (fixture et test) ; `--set` tolérant : raison sans
  le préfixe « invalid form value: nom: » ; `flatten(Some(..))` dédoublonne les pages.
- Interface : contrôles du panneau rattachés au nom du champ (`labelled_by`) ; cases à cocher carrées ; bandeau
  « formulaire à remplir » repliable en zone étroite ; outil Remplir quitté après un aplatissement complet ;
  Ctrl+S valide la saisie en cours ; valeur refusée par le moteur → modèle relu ; « Effacer les champs » sans
  rien à effacer n'ajoute plus de niveau d'annulation (message d'état) ; indication de saisie masquée pour un
  formulaire XFA.

### Ajouté (formulaires AcroForm, tranche forms)
- **Remplir et signer** gagne sa moitié « remplir » : outil **Remplir le formulaire** (champs surlignés sur la
  page — bleu, rouge si obligatoire, gris si lecture seule —, clic = case ou bouton radio basculés, éditeur posé
  sur le champ texte avec Entrée / Échap / Tab / Maj+Tab et longueur maximale, liste de choix flottante pour les
  listes déroulantes et zones de liste, à choix multiple par cases), fonctionnel en défilement continu et dans
  chaque onglet ; bandeau « Ce document contient un formulaire à remplir » à l'ouverture.
- Panneau **Champs du formulaire** (Ctrl+Maj+F) : un contrôle accessible par champ (zone de texte, case, boutons
  radio, liste déroulante, zone de liste ; boutons et signatures listés sans action), page courante ou tout le
  document, bandeaux XFA (lecture seule) et scripts (jamais exécutés), aller au champ par son nom.
- Menu **Formulaires** : Effacer les champs, Aplatir le formulaire et les annotations… (toutes les pages ou la
  page courante, boîte de confirmation, annulable), Exporter les valeurs (JSON)…, Importer des valeurs (JSON)…
  (champs inconnus et valeurs refusées comptés dans le message d'état).
- Moteur : `Document::form_info` (modèle lopdf : hiérarchie, héritage, états des cases et radios, listes avec
  valeurs d'export et libellés, rectangles en espace d'affichage), `fill_form` (strict ou tolérant, validation
  complète avant toute écriture, `/V` `/AS` `/I` écrits par lopdf, apparences régénérées par PDFium et écrites
  dans le fichier), `clear_form`, `flatten` (indicateur Print posé sur les annotations, apparences générées avant,
  `/AcroForm` retiré quand toutes les pages sont aplaties), `ensure_appearances`, `DocumentInfo.form`, module
  `form_data` (JSON) ; un document chiffré se remplit et reste chiffré ; erreurs typées `form_field_not_found`,
  `form_value_invalid`, `form_xfa`.
- CLI : `form-fields` (`--values-only`), `fill-form` (`--values`, `--set`, `--ignore-unknown`, `--flatten`),
  `flatten` (`--pages`), 36 commandes. Fixtures `acroform.pdf`, `acroform-noap.pdf`, `acroform-xfa.pdf`
  (`scripts/make_fixtures.py`).

### Corrigé (sécurité, AES-256)
- Un mot de passe AES-256 contenant un espace spécial ou une ligature (normalisés SASLprep par lopdf au
  chiffrement) ne rouvrait pas le fichier dans SuniPDF : l'ouverture réessaie avec la forme normalisée et le
  mot de passe mémorisé pour l'annulation est celle-ci (test moteur ajouté).

### Ajouté (signature visuelle, tranche sign-image)
- **Remplir et signer** (moitié « signer ») devient un outil réel du hub, avec sa barre : **Signer (signature
  manuscrite)…** (S), **Parapher (initiales)…**, **Insérer la date du jour** ; aussi dans le menu Édition et la
  palette. **Signature visuelle uniquement**, dit partout (libellé, sous-titre, encadré permanent) : elle ne
  prouve ni l'identité ni l'intégrité, ce n'est pas une signature numérique certifiée.
- Boîte « Signature manuscrite / Paraphe » : pavé de dessin (souris, pavé tactile, stylet ; encre noire ou
  bleue, épaisseur, annuler le dernier trait, effacer), **import d'une image** (PNG transparent tel quel, ou
  photo sur fond clair dont le fond est retiré par un seuil réglable avec aperçu, décodage hors thread
  d'interface), **bibliothèque locale** (`%APPDATA%\SuniPDF\signatures`, PNG en clair, enregistrement
  facultatif, suppression en deux clics, vignettes sur papier blanc dans les deux thèmes, décodée elle aussi
  par le thread moteur).
- Placement : fantôme sous le pointeur, clic pour poser (centré, maintenu dans la page), Maj+clic pour poser
  plusieurs fois, Échap pour annuler ; l'élément posé est un **objet image à masque alpha** (le contenu reste
  visible entre les traits) ou un objet texte (date), aussitôt sélectionné : poignées, déplacement, rotation,
  dupliquer, supprimer, Ctrl+Z, modifiable par F2 pour la date.
- Moteur : `Document::add_image_data` (image décodée, alpha conservé par `/SMask`), module
  `signature_image` (traits tramés anti-crénelés, fond retiré, rognage, lecture bornée à 16 384 px de côté et
  512 Mio décodés, refusée avant tout décodage, PNG atomique).
- CLI : `add-image <FILE> --image <IMG> --x --y [--width] [--remove-background <seuil>] --out` (réponse avec
  `"digital_signature": false`), 33 commandes.
- Automatisation : `SUNIPDF_SIGNATURES_DIR`, `SUNIPDF_PICK_FILE`.

### Corrigé (revue de sign-image)
- Cadre commun des boîtes (`dialog.rs`) : quand le corps déborde (bibliothèque de signatures remplie), la barre
  de défilement flottante reçoit une gouttière de 10 px au lieu de recouvrir le bord droit de l'encadré, du pavé
  et du bouton Effacer (capture revérifiée).
- **Mémoire** : une image dont le décodage dépasserait 512 Mio (un PNG de 2 Mo annonçant 16 384² pixels en
  RGBA16 donnait 2 Gio) est refusée à la lecture de l'en-tête, sans rien allouer ; la même borne vaut
  désormais pour Insérer une image, Remplacer l'image et Images → PDF (décodeur borné unique dans le moteur).
- La carte **Remplir et signer** décrit ce qui existe (signer, parapher, dater) et non plus un remplissage de
  formulaire qui n'est pas livré.
- La **date** posée près du bord droit ou haut reste dans la page (largeur estimée, 0,556 em par caractère).
- Bibliothèque des signatures décodée par le thread moteur (plus sur le thread d'interface) ; après une
  suppression, la signature suivante au-delà du plafond de 12 apparaît aussitôt ; libellé « Recliquer pour
  supprimer » qui tient dans la carte (et rien ne déborde plus de la carte) ; pastilles d'encre sur disque
  blanc, lisibles en thème sombre ; Maj+clic ne montre plus de poignées sur l'objet posé pendant que le mode
  de placement continue ; nom accessible « Paraphe enregistré n » pour les paraphes ; le journal ne contient
  plus que le nom du fichier image, jamais son chemin complet.
- Test CLI de bout en bout `add-image` (fond retiré, rognage, hauteur, rendu relu au pixel, `already_exists`,
  image vide) ; le nombre de commandes CLI annoncé dans la documentation (21) est corrigé : 33.
- CLI `add-image` : une image entièrement transparente sans `--remove-background` est refusée (`image is fully
  transparent`) au lieu de poser un objet invisible.

### Corrigé (tranche review-fixes : constats des revues adverses)
- **Sécurité** : AES-128 refuse un mot de passe contenant des caractères hors Latin-1 imprimable
  (erreur `password_not_latin1`, message dans la boîte Protéger) au lieu de les abandonner sans bruit, ce qui
  donnait un fichier ouvrable sans mot de passe ; les symboles € œ — “ ” (PDFDocEncoding hors Latin-1) sont
  refusés aussi, car PDFium ne rouvrait pas le fichier avec le mot de passe saisi (aller-retour testé) ; la
  boîte Protéger donne le focus au premier champ ; le bit d'extraction pour l'accessibilité reste toujours
  accordé ; modifier les propriétés met aussi à jour le paquet XMP (l'ancien auteur n'y survit plus) ;
  « Supprimer les métadonnées » retire aussi les XMP des pages et des images et les données `/PieceInfo`.
- **Fichiers** : l'enregistrement atomique synchronise les données sur le disque avant de remplacer
  l'original (plus de fichier de zéros après une coupure de courant) ; **extraire ou diviser un document
  protégé** produit des fichiers non chiffrés, désormais annoncé (barre d'état, `encryption_removed` en CLI).
- **Images** : l'orientation EXIF des photos est appliquée (Images → PDF, insérer et remplacer une image) ;
  les JPEG sont incorporés tels quels dans Images → PDF (mémoire constante, fichier de la taille des JPEG).
- **Édition** : « Premier plan » tient après rechargement, annulation et enregistrement (les flux de contenu
  sont fusionnés dans les deux sens) ; un tampon à 45° placé dans un coin reste dans la page ; redimensionner
  un objet pivoté d'un angle quelconque le met à l'échelle uniformément au lieu de le cisailler ; le texte
  ajouté accepte les tabulations (quatre espaces) et l'erreur de police liste les caractères absents ;
  `sunipdf-cli text` renvoie tout le texte des pages pivotées.
- **Clavier** : Ctrl+Maj+S ne déclenche plus « Enregistrer » (écrasement de l'original) quand un champ a le
  focus ; Ctrl+Maj+D duplique (au lieu d'ouvrir Propriétés) et Ctrl+Maj+Z rétablit (au lieu d'annuler) ;
  Ctrl+Plus / Moins / 0 ne changent plus l'échelle de toute l'interface ; les boîtes de dialogue et les
  éditeurs de texte suspendent tous les raccourcis.
- **Boîtes de dialogue** : Échap sur une liste déroulante ou un sélecteur de couleur ne ferme que la liste (le
  texte saisi n'est plus perdu) et ne ferme plus la barre de recherche derrière la boîte ; les boutons restent
  visibles dans une fenêtre basse (corps défilant) ; « Appliquer les métadonnées » est grisé tant que les
  propriétés chargent (il effaçait titre, auteur, sujet et mots-clés) ; « Apposer » est grisé sans texte ;
  texte d'erreur lisible en thème sombre ; « Retirer la protection » n'affiche plus « non protégé » juste après
  Protéger.
- **Document** : annuler jusqu'à l'état enregistré rend le document « intact » (point d'enregistrement dans
  l'historique ; l'indicateur des onglets suit cet état au lieu d'un OU logique) ; « Couper » copie puis
  supprime dans le thread moteur, d'un seul tenant (rien n'est supprimé si la copie échoue, aucune édition ne
  peut décaler les index entre les deux) ; l'erreur de police liste chaque caractère absent une seule fois ;
  la liste des positions de tampon affiche ses sept entrées ; l'annotation d'un glissement
  est celle de l'outil au moment du geste (changer d'outil avant la réponse du moteur ne la change plus) ;
  après une suppression de pages dans la grille la sélection est vidée (un second clic ne supprimait pas les
  pages choisies) ; déposer une page à sa propre place ne marque plus le document modifié ; « Déplacer à
  gauche / droite » grisés quand plusieurs pages sont sélectionnées ou en bout de document.
- **Textes** : le panneau Commentaires nomme tous les sous-types (« Barré » au lieu de « Strikeout », zones
  de texte, formes, encre, tampons, pièces jointes…) ; tailles de fichier et suffixes des noms proposés
  traduits (« 2.3 MB », `-extract`, `-optimized`).

## [0.4.0] — 2026-10-14

Vague 1 du backlog, livrée par tranches avec revue indépendante : hub « Tous les outils » façon PDFCraft,
édition avancée du contenu, vue continue, puis correctifs de la revue adverse et distribution durcie.


### Ajouté (vue continue, SUNI-012b)
- **Défilement continu** : toutes les pages empilées dans une seule zone de défilement (mode par défaut) ;
  **Page unique** reste disponible (F5, bouton du rail droit, menu Affichage, palette), choix mémorisé.
  Seules les pages visibles sont rendues et gardées en mémoire graphique : un document de 500 pages défile sans
  chargement d'ensemble.
- La **page courante suit le défilement** (champ du rail, barre d'état, miniature ramenée dans le panneau,
  panneau des commentaires) ; un clic sur une page la rend courante ; « aller à » (champ, miniature, signet,
  Origine / Fin, page suivante / précédente, résultat de recherche) amène la page en haut de l'écran.
- **Zoom ancré** : Ctrl + molette zoome autour du pointeur (pincement du pavé tactile : même chemin, non testé), boutons et liste
  autour du centre ; liste étendue à 800, 1200 et 1600 %, molette jusqu'à 3200 % ; pendant le zoom l'image
  précédente reste affichée, étirée, jusqu'au nouveau rendu.
- **Rendu en tuiles** (1024 px) au-delà de 4096 px de côté : le zoom profond reste net sans plafond de
  16 384 px ; une retouche (objet, surlignage, tampon) garde l'image affichée jusqu'au nouveau rendu au lieu
  d'une page blanche.
- Clavier : Espace / Maj+Espace et Page suiv. / Page préc. défilent d'un écran en continu, flèches ↑ ↓ d'une
  ligne (dans les deux modes).
- Moteur : `Document::render_region`, module `sunipdf_core::tiles`, clés de tuiles dans le cache LRU ;
  CLI `render --region x,y,w,h` (sortie `region`, `page_pixels`).
- Fixture `many-pages.pdf` (120 pages de tailles mixtes, `make_fixtures.py --many N` pour plus) ; 7 tests
  d'intégration (régions = découpe du rendu complet, page pivotée, zoom ×40 au-delà de 16 384 px, page image,
  erreurs, recollage des tuiles), 14 tests de la vue (mise en page, virtualisation de 600 pages, ancre de zoom,
  plafonds, pool de textures borné, tuiles voulues, Espace / Maj+Espace) ; `dev-screenshot.ps1 -Wheel` pour Ctrl + molette.

### Modifié (vue continue)
- Le **panneau Objets passe de F5 à F3** (F5 bascule désormais la vue).
- En défilement continu, **Page suiv. / Page préc. défilent d'un écran** au lieu de changer de page ; les
  flèches → ← changent toujours de page (raccourcis affichés « → » / « ← »).
- « Ajuster à la largeur / à la page » en défilement continu ajuste la plus grande page du document (zoom
  identique pour toutes les pages).
- Rail droit : les bascules actives (miniatures, Organiser, défilement continu, panneaux, recherche, ajustements)
  sont surlignées ; en fenêtre basse le groupe du haut défile au lieu de chevaucher la navigation. Le mode de vue
  actif est surligné dans le menu Affichage.

### Corrigé (vue continue)
- Plantage possible à fort zoom sur un GPU limitant la taille des textures à 8192 px (page rendue d'un seul
  tenant plus grande que la texture maximale) : aucune texture ne dépasse plus la limite du GPU.
- Bouton de fermeture de la barre de recherche affiché comme un carré vide (glyphe absent de Segoe UI).

### Ajouté (édition avancée, SUNI-031c)
- **Rotation libre** des objets : poignée ronde au-dessus du cadre (Maj : pas de 15°, angle dans la barre
  d'état), champ « Angle » de la barre d'objet ; cadre orienté pour les textes et images.
- **Opacité** des textes, formes et images (barre d'objet, nouvelles formes et images) ; changer la couleur
  d'un objet conserve désormais son opacité.
- **Sélection multiple** : Maj/Ctrl+clic, rectangle de sélection (Alt : objets touchés), Ctrl+A ; déplacement,
  suppression, duplication, ordre, couleurs et opacité appliqués au groupe ; prise directe d'un objet non
  sélectionné au glisser.
- **Copier / couper / coller des objets** (Ctrl+C / X / V, menu Édition, barre d'objet) entre pages et entre
  onglets ; les objets collés forment un groupe.
- **Panneau Objets** (F3 depuis la vue continue) : liste de la page du plus en avant au plus en arrière, sélection, avancer / reculer
  d'un plan, suppression, repère au survol.
- **Texte sur la page** : saisie directe à l'emplacement cliqué (Entrée, Maj+Entrée, Échap), mini-barre taille /
  couleur / police / gras / italique ; plusieurs lignes ; modification sur place d'un texte existant droit.
- **Polices TrueType du système** incorporées (liste avec recherche, licence d'incorporation vérifiée) ; un
  caractère absent de la police choisie est refusé avec un message au lieu d'être perdu.
- **Remplacer l'image** (même cadre, nouvelle image ajustée), **Insérer une page vierge**, **Recadrer les
  pages** (marges en mm, aperçu, rétablissement), **Redimensionner les pages** (contenu conservé ou mis à
  l'échelle).
- CLI : `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`,
  `resize-pages`, `insert-blank`, `fonts` ; `info` donne la rotation de chaque page.
- 9 commandes au registre (78 au total) ; 11 tests d'intégration du moteur (pixels rendus, relecture,
  réouverture, erreurs) sur une nouvelle fixture `boxes.pdf` (pages pivotées et rognées).

### Corrigé (édition avancée)
- **Pages pivotées ou recadrées** : sélection, poignées, formes, texte, images, surlignage, notes et résultats de
  recherche tombaient au mauvais endroit (le moteur parlait en coordonnées PDF non pivotées). Toute l'API du
  moteur est désormais en coordonnées de la page affichée ; **`sunipdf-cli search` rend aussi ses rectangles
  dans cet espace** (changement visible uniquement sur les pages pivotées ou rognées).
- **Ctrl+C ne copiait pas** le texte sélectionné (egui transforme Ctrl+C en événement Copier, sans touche) :
  Ctrl+C / X / V passent maintenant par ces événements.
- **Dupliquer** faisait passer l'objet d'origine au premier plan : la copie est désormais seule à changer.
- **Taille d'un texte pivoté** : annoncée 0 (90°) ou réduite (45°), elle était multipliée à la modification
  (un tampon à 90° passait de 48 à 12 pt, un texte vertical de 12 à 144 pt) ; elle tient compte de la rotation.
- **Tampons sur une page pivotée** : centrés et à l'angle demandé à l'écran.
- **Texte saisi en police standard** : un retour à la ligne ou un caractère hors latin (cyrillique, emoji)
  produisait des « ÿ » sans erreur ; chaque ligne devient un objet et un caractère absent est refusé.
- **Changer la couleur** d'un objet transparent (tampon à 30 %) le rendait opaque.
- La CLI débordait de sa pile en compilation de débogage ; elle tourne sur un fil de 16 Mio.

### Ajouté
- **Hub « Tous les outils »** façon PDFCraft : le panneau de gauche (F9) liste les outils par tâche avec une
  pastille colorée par outil (Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger,
  Compresser, Ajouter un tampon) ; les cinq outils PDFCraft sans fonction réelle dans SuniPDF ne sont pas
  affichés. Les outils Modifier, Organiser, Commenter, Protéger et Tampon ouvrent une **barre d'outil**
  contextuelle (boutons groupés, libellés selon la largeur, actions grisées sans sélection, « Quitter
  l'outil » ou Échap) ; les autres ouvrent directement leur boîte de dialogue. Menu **Outils**, entrées
  `hub.*` dans la palette Ctrl+K (6 commandes au registre, 69 au total), raccourcis à une lettre qui ouvrent
  l'outil correspondant, outil actif conservé par onglet.
- **Accueil** : grande carte « Ouvrir un fichier » et une carte entièrement cliquable par outil (bordure de la
  couleur de l'outil au survol) ; **barre d'outils** : accès rapide Ouvrir / Enregistrer / Annuler / Rétablir et
  bouton « Tous les outils ».

### Modifié
- La palette d'outils flottante sur la page est remplacée par la barre d'outil ; la barre d'objet ne garde que
  les propriétés (trait, remplissage, Appliquer) ; la barre de la grille Organiser garde le compteur, « Tout
  sélectionner » et l'aide. Les onglets texte Lire / Annoter / Éditer / Pages / Convertir / Sécurité de la barre
  d'outils disparaissent (le hub les remplace).
- Finition du chrome : bouton principal des dialogues lisible en thème sombre (contraste 5,2:1 au lieu de
  2,5:1), glyphes d'outils contrastés sur leur pastille (≥ 3,5:1, testé), fondu au survol, anneau de focus
  clavier, ombres plus douces, infobulles plus rapides, barres de défilement flottantes, onglet actif souligné,
  icônes plus parlantes (combiner, compresser, diviser, extraire, insérer, rétablir, ouvrir).
- **Tampon / filigrane** et **numérotation des pages** (catégories Pages et Éditer) : texte apposé sur
  toutes les pages ou la page courante, taille, couleur, gras, angle, opacité, 7 positions,
  variables `{n}` et `{total}` ; chaque tampon reste un objet modifiable et annulable.
  Moteur `sunipdf-core::stamp`, CLI `stamp` et `page-numbers`, 1 test moteur.

### Corrigé (revue du hub d'outils, 2026-10-14)
- **Échap avec un menu ouvert** : avec un outil actif, Échap quittait l'outil et laissait le menu ouvert ; il
  ferme désormais le menu (ou la liste déroulante, le sélecteur de couleur) et rien d'autre. Ordre complet,
  un effet par Échap : menu, champ ou recherche, objet sélectionné, outil. La recherche passe aussi avant
  l'objet sélectionné, comme l'annonçait le guide.
- Icônes : « Pivoter à gauche » (pages et objets) ne reprend plus le glyphe d'« Annuler » ; « Déplacer la
  page à gauche / à droite » utilisent des chevrons horizontaux ; « Modifier le texte » ne reprend plus celui
  d'« Ajouter du texte » (testé : aucune icône répétée dans une barre ni avec l'accès rapide).
- La barre de propriétés d'objet reste dans la zone du document et passe à la ligne en fenêtre étroite au
  lieu de recouvrir le rail droit.
- « Quitter l'outil » n'apparaît plus dans la palette Ctrl+K quand aucun outil n'est actif.
- Les boutons d'accès rapide relâchent le focus après une activation au clavier (les raccourcis à une lettre
  restaient muets).
- Un fichier passé en argument relatif est enregistré dans les fichiers récents avec son chemin absolu.

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
- **Distribution** : les exécutables ne dépendent plus de `VCRUNTIME140.dll` (CRT statique,
  `.cargo/config.toml`) et démarrent sur un Windows neuf ; le MSI installe les licences et notices tierces
  (17 fichiers) et un LISEZMOI propre à l'installation, et accepte la réinstallation à version égale ;
  `release.yml` exécute les tests avant l'empaquetage, a le droit de publier la Release, recalcule
  l'empreinte du MSI après signature et annonce l'ARM64 comme non testé ; l'artefact de `ci.yml` est le
  paquet officiel ; `fetch-pdfium.ps1` vérifie l'empreinte épinglée et le numéro de build ; scripts
  PowerShell en UTF-8 avec BOM (LISEZMOI lisible sous Windows PowerShell 5.1) ; `start.ps1` en `-j 2` ;
  `pdfium.dll` n'est plus cherché dans le répertoire courant en version publiée.

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
