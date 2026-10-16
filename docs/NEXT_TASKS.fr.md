# Prochaines tâches (ordre conseillé)

[English](NEXT_TASKS.md)

## Version 0.9.0 : les sept fonctions demandées sont publiées

Les sept fonctions sont dans la [release 0.9.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.0)
sous `lmdanavanem`. [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md) consigne les contrôles exécutés, preuves de publication
et limites matérielles/d'interopérabilité. Les anciens nombres de tests ne sont pas le critère de publication actuel.

| Fonction | Périmètre inclus |
|---|---|
| Sauvegarde et récupération | Instantanés 60 s/suspension, source intacte, PDF protégés toujours chiffrés, aucun mot de passe, restauration Enregistrer sous |
| Impression Windows | Plages/copies/orientation natives, modifications courantes, rendu tramé borné, progression/annulation |
| Sélection enrichie | Plages logiques PDFium entre pages, glissement inverse, double-clic sur mot et défilement au bord |
| Double page/Livre | Tailles mixtes, couverture seule, dernière page isolée, zoom partagé et caches bornés |
| Création de formulaires | Rectangles numériques texte/case, noms uniques, documents pivotés/recadrés/chiffrés, annulation/remplissage/enregistrement |
| OCR local FR/EN | Tesseract 5.5.3/Leptonica 1.87 embarqués, modèles rapides eng/fra, texte invisible, pages textuelles ignorées, annulation atomique et un undo |
| Signatures numériques | Intégrité des octets enregistrés, copie PFX/P12 incrémentale, avertissement de réécriture ; confiance de chaîne/révocation/horodatage fiable/LTV non implémentés |

Les corrections P1/P2 de la 0.8.3 restent incluses : coordonnées finies, miniatures Organiser bornées,
tampons sur une ligne et opacité CLI valide. Anglais par défaut ; français disponible.

## Version 0.9.1 : release de fiabilisation publiée

[SuniPDF 0.9.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) a été publiée en Latest sous `lmdanavanem`.

- Conserver les identifiants PDF/A et le XMP indépendant en modifiant les propriétés standard ; refuser clairement les XML malformés/DTD.
  Cela ne certifie pas la conformité PDF/A.
- Écrire explicitement la longueur AES-256 à l'enregistrement, y compris après métadonnées/nettoyage/optimisation.
- Contrôles qpdf figés en CI sur fixtures synthétiques/sorties ; le corpus réel ou hostile reste un travail distinct.
- Sept scénarios portables réels de mise à niveau/échec/restauration vers 0.9.1 et installation/mise à niveau/rollback transactionnel,
  refus de downgrade et désinstallation MSI réussis sur runners Windows jetables ; documents/paramètres conservés.
- Calcul SHA256 .NET fonctionnel quand Windows PowerShell 5 ne charge pas Get-FileHash ; erreurs de relance signalées.
- Icône Windows dédiée dérivée du visuel fourni par l'auteur et retouché.

Workspace (322 tests), qpdf (22 PDF/trois entrées malformées) et CI Windows finale réussis ; les six fichiers publics
correspondent aux paquets vérifiés et sont disponibles dans `dist/`.
Les preuves actuelles des paquets et de publication sont dans `PROJECT_STATUS.fr.md`.
Ces contrôles ne prouvent pas l'exécution ARM64, les imprimantes physiques, les lecteurs tiers ni l'UX interactive de mise à jour.

## Version 0.10.0 : lot P1/P2 demandé publié et vérifié

Les six domaines demandés sont implémentés : erreurs de sauvegarde/corpus public, compression des images,
rotation de vue/plein écran/historique, champs et échanges/groupes/clavier, recadrage souris/transformations groupées,
lots séquentiels compression/OCR. Limites exactes et résultats exécutés sont dans `PROJECT_STATUS.fr.md`.
Workspace local : 362 réussis, zéro échec, sept ignorés par défaut ; formatage, Clippy et 853 clés bilingues identiques
réussis. qpdf valide explicitement 31 PDF synthétiques/générés ; quatre publications NIST/NBS licenciées (744 pages)
réussissent texte/rendu/extraction/optimisation/compression/préservation source. Les avertissements d'origine sont figés ;
les réécritures passent sans avertissement. Le writer partagé corrige stockage/bornes xref/offsets de révision périmés.
[CI main finale](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848137022), compilation ARM64 de même source,
six paquets/empreintes préparés, huit scénarios portables réels et deux bases MSI réelles (0.8.0 et 0.9.1) réussissent.
Paquets : métadonnées anglaises, icônes incorporées, compression, champs de choix/XFDF, groupes, OCR français et
contenu MSI/ZIP identique. [SuniPDF 0.10.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) est publiée
en Latest sous `lmdanavanem` ; les six téléchargements publics sans authentification correspondent aux empreintes
API/SHA256/paquets préparés et figurent dans `dist/`. Les contrôles renforcés réussissent sur les binaires publics,
et l'updater de production valide le ZIP x64 Latest de 19 651 167 octets sans installation. Les critères de release
du lot P1/P2 demandé sont remplis ; les contrôles matériels/d'interopérabilité indépendants ci-dessous restent ouverts.
Provenance et source de release immuable `f7510afd314846079bbdd838525cd28372fd3100` figurent dans l'état du projet.

## 0.11.0 : navigation, formulaires et copies protégées (publiée)

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) sous `lmdanavanem` ; Latest à cette date.

La source 0.11.0 réussit 390 tests du workspace (zéro échec, sept ignorés par défaut), formatage/Clippy avec
`-D warnings`, 858 clés EN/FR, 44 contrôles qpdf sur 42 PDF distincts et quatre documents NIST réels
(744 pages ; 91,59 s, pic mémoire enfant 329 347 072 octets). [CI Windows finale](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) et
[build ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) réussis, ainsi que neuf scénarios portables réels
depuis 0.8.2/0.8.3/0.9.0/0.9.1/0.10.0, trois bases MSI (0.8.0/0.9.1/0.10.0) et les six fichiers préparés.
Contrôles embarqués : copies AES-128/256 ouvertes comme propriétaire, contenu déchiffré/qpdf, libellés, métadonnées,
OCR, formulaires et compression. Source immuable : `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`. Les six
téléchargements publics concordent avec les empreintes API/SHA256/paquets ; contrôles renforcés réussis sur les
binaires publics. Le metteur à jour de production a vérifié le ZIP Latest x64 de 19 687 375 octets sans installation.
Preuves et limites dans `PROJECT_STATUS.fr.md` ; source/paquets 0.10.0 publiés inchangés.

- Lire les étiquettes de pages via PDFium dans la barre, les miniatures et la CLI `info`. Aller à accepte une
  étiquette exacte et unique ; `#N` désigne toujours la page physique. Lecture seule ; étiquettes invalides remplacées par le numéro physique.
- Résoudre les destinations nommées et positions `/XYZ`, `/Fit`, `/FitH` non nulles des liens et signets, avec
  recadrage/rotation et conservation des axes nuls et du zoom. L'historique couvre les sauts dans la même page et le zoom.
  `/FitH` zéro/null et les autres modes gardent le saut de page ; PDFium ne distingue pas FitH null de zéro.
- Conserver les brouillons et actions de formulaire en cas d'échec de lecture, afficher l'erreur et demander une relance explicite.
- Saisir du texte libre dans les listes déroulantes déjà modifiables, depuis le panneau et sur la page ; garder les choix,
  Appliquer/Abandonner, Enregistrer et l'annulation. La création de ce drapeau était hors de la tranche 0.11.0.
- Conserver chiffrement, mots de passe et permissions d'origine en extrayant/divisant un PDF protégé ; refuser une sortie
  résolue au chemin source. La CLI garde `encryption_removed: false` et ajoute `encrypted` ; la politique existante
  des permissions ne change pas. Contrôles locaux, CI, paquets et régressions de mise à niveau réussis ; preuves dans `PROJECT_STATUS.fr.md`.

## 0.12.0 : création de champs et valeurs d'export radio (publiée ; Latest à cette date)

- Créer des listes déroulantes modifiables par la case de l'interface ou `add-form-field --kind combo --editable` ;
  conserver choix prédéfinis requis, noms uniques, contrôles de géométrie/protection et annulation.
- Distinguer valeurs d'export radio `/Opt` et états d'apparence, y compris dans les échanges JSON/XFDF/FDF.
  Ajouter `widgets[].export_value` aux informations CLI sans modifier `on_state` ; correspondances malformées ou
  dupliquées : champ concerné en lecture seule. Sans `/Opt`, les radios conservent les noms d'état.
- [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) sous `lmdanavanem` (Latest à cette date ; release
  `414674886`), source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation réussie : 398 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 859 clés EN/FR, 48 contrôles qpdf et quatre documents NIST/NBS (744 pages ;
  59,68 s). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) et
  [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) réussis,
  ainsi que dix scénarios portables, quatre bases MSI (0.8.0/0.9.1/0.10.0/0.11.0), six fichiers préparés/publics et
  contrôles renforcés des binaires. L'updater de production a vérifié le ZIP Latest x64 de 19 699 323 octets sans installation.
  Fichiers vérifiés dans `dist/` ; preuves et limites dans `PROJECT_STATUS.fr.md` (D100–D101).

## 0.13.0 : signatures visuelles saisies et lots de pages (publiée ; Latest à cette date)

- Aperçu local explicite d'un nom/paraphe saisi, sélecteur standard/TrueType système existant,
  contrôles de glyphes/incorporation et parcours PNG transparent/bibliothèque réutilisés (D102). Times italique
  par défaut ; une ligne de 256 caractères au plus et aperçu limité à 1600 pixels de côté.
- Page courante/toutes les pages/sélection Organiser capturée avant de quitter la grille ; même centre en
  points affichés, proportions conservées, annulation/restauration atomiques et un undo pour le lot (D103).
  CLI `add-image --pages` ajoute le lot sans modifier le comportement/JSON historique de `--page`.
- [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem` (Latest à cette date ; release
  `414763678`), source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation réussie : 410 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 870 clés EN/FR, 53 contrôles qpdf et quatre documents NIST/NBS licenciés
  (744 pages ; 76,40 s localement). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
  réussis. Onze scénarios portables (sept mises à niveau dont 0.12.0 et quatre protections contre les échecs), cinq
  bases MSI (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0), six fichiers préparés/publics et contrôles renforcés des binaires réussis.
  Lots de signatures embarqués : PDF non chiffrés/AES-128/256 ouverts comme propriétaire, les deux mots de passe,
  qpdf et préservation source. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le ZIP Latest x64
  de 19 730 578 octets sans installation. Preuves et limites restantes dans `PROJECT_STATUS.fr.md` (D102–D103).

## 0.14.0 : création d'étiquettes et navigation conservée (publiée ; Latest à cette date)

- Pages/Organiser et CLI `set-page-labels`/`reset-page-labels` appliquent six styles, préfixe/départ positif sur
  une plage physique inclusive, conservent les règles hors plage et leur continuation à la frontière suivante
  (D104). Préfixes sur une ligne de 256 octets UTF-8 au plus ; étiquettes générées bornées à 4096 octets. Arbre
  existant malformé : Appliquer refusé avant mutation ; Réinitialiser retire l'entrée du catalogue sans effacement sécurisé.
- Les étiquettes modifient la navigation, pas les numéros imprimés. Les plages restent positionnelles après
  opérations structurelles ; recalage sémantique futur. Navigation par libellé/`#N` physique et protection conservées.
- Les instantanés Annuler/Rétablir conservent l'information de géométrie inchangée pour préserver la navigation
  lors d'éditions de contenu/étiquettes ; opérations structurelles toujours invalidantes (D105).
- [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) sous `lmdanavanem` (Latest à cette date ; release
  `414812776`), source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation réussie : 420 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 890 clés EN/FR, 63 contrôles qpdf et quatre documents NIST/NBS licenciés
  (744 pages ; 53,91 s localement). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
  réussis. Douze scénarios portables (huit mises à niveau dont 0.13.0 et quatre protections contre les échecs), six
  bases MSI (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0), six fichiers préparés/publics et contrôles renforcés des binaires
  réussis, dont création/réinitialisation protégée, les deux mots de passe, étiquettes hors plage, Unicode et texte/PNG
  inchangés. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le ZIP Latest x64 de 19 785 200 octets
  sans installation. Preuves exécutées et limites restantes dans `PROJECT_STATUS.fr.md` (D104–D105).

## 0.15.0 : modifier les destinations de liens existants (publiée en Latest)

- Réutiliser la boîte de lien via Modifier la destination/double-clic, préremplir les destinations nommées/XYZ
  résolues et remplacer explicitement les vues de page par Fit. Cible inconnue : adresse vide, jamais page 1 implicite.
- Moteur/UI/CLI acceptent `http`/`https`/`mailto` ou une page physique, remplacent ensemble champs de cible/actions
  additionnelles et conservent zone/style/noms/commentaires/réponses, données indépendantes et protection (D106).
- Respecter ReadOnly/Locked/LockedContents directs/indirects ; drapeaux malformés : édition refusée. Un Annuler conserve
  géométrie/navigation. Éditions/changements d'onglet abandonnent les boîtes périmées ; protections CLI existantes conservées.
- [Publiée en Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous `lmdanavanem`, release
  `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Contrôles locaux réussis : 429 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés
  (744 pages ; 48,891 s, pic mémoire enfant 328 736 768 octets). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541) réussis,
  ainsi que treize scénarios portables (neuf mises à niveau dont 0.14.0 et quatre protections contre les échecs), sept
  bases MSI (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0/0.14.0), six fichiers préparés/publics et contrôles renforcés.
  Copies vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP Latest x64 de 19 803 226 octets
  sans installation. Preuves exécutées et limites restantes dans `PROJECT_STATUS.fr.md` (D106).

## Travail restant au-delà de ces lots

1. Matériel ARM64, imprimantes physiques, Narrateur Windows, dialogues interactifs de mise à jour/enregistrement et
   Acrobat/Foxit/Chrome. Le matériel et les lecteurs réels sont nécessaires ; la CI scriptée ne les remplace pas.
2. Élargir les quatre fixtures NIST à des PDF clients/hostiles/volumineux licenciés, scans et formulaires complexes ;
   fuzzing et mesures de performance avec matériel identifié. Toute erreur/annulation doit préserver l’original.
3. Sous-ensembles de polices/CFF/OTF/TTC, édition de paragraphes et objets collés éditables.
4. Conversion Office locale et autres lots de conversion, avec fidélité mesurée.
5. Confiance/révocation des certificats, horodatage fiable/LTV et signature du code/MSI si identité fournie.
6. Recalage sémantique des étiquettes après changement de pages, colonnes complexes non balisées,
   tactile et intégrations facultatives.

Aucune date/version future n’est engagée par cette liste.

## À faire par l'utilisateur (hors de portée de l'agent)
1. Valider l'interface sur de vrais PDF (accueil, outils, rail droit, palette Ctrl+K, thème sombre, FR/EN) et signaler les écarts.
2. Exécuter le ZIP ARM64 sur une machine ARM64 réelle (jamais fait) et vérifier le rendu OpenGL.
3. Vérifier l'interopérabilité des annotations et du caviardage dans Acrobat, Foxit et Chrome.
4. Valider les boîtes interactives de mise à jour MSI et portable, avec annulation et documents modifiés ;
   les scénarios scriptés d'installation/mise à niveau/restauration réussissent en CI.
5. Vérifier le Narrateur Windows et la navigation clavier dans l'interface réelle.

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
Les 14 outils possèdent une implémentation en 0.9.0, dont Numériser et OCR, Préparer un formulaire et Utiliser un certificat. « Remplir et signer » est complet : moitié « signer » (tranche sign-image) et moitié « remplir »
(tranche forms, groupe Remplir le formulaire / panneau / Effacer / Aplatir / Exporter / Importer).

## Après redaction (caviardage irréversible, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A40, décisions D077–D082) :
- contrôles indépendants sur un corpus réel (numérisations avec couche OCR, PDF balisés lourds,
  formulaires XObject imbriqués, polices Type 3) ; mesure sur une page de plusieurs dizaines de milliers d'objets
  (`prune_objects` quadratique : accessibilité maison par `BTreeSet` si lent) ;
- découpe fine du texte (garder sélectionnable la partie non caviardée d'une ligne) : demande de recréer des
  fragments avec la police d'origine, hors de portée de `pdfium-render` 0.9.4 ;
- couleur de remplissage et texte en surimpression (« CAVIARDÉ »), marques `/Redact` persistantes (pas de créateur
  dans `pdfium-render`) ;
- nettoyage des signets, pièces jointes, JavaScript et destinations nommées (outil « Assainir » séparé) ;
- suppression d'un sous-ensemble de police devenu inutile ; aperçu « avant / après » de la page ;
- `docs/SECURITY_MODEL.md` (méthode A/B/C, garanties, hypothèses) ; capture S3 (après application) non rejouée.

## Après forms (formulaires AcroForm, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A38, décisions D061–D066) :
- échanges JSON/XFDF/FDF et panneau Appliquer groupé implémentés en 0.10.0 ; validation dans `PROJECT_STATUS.fr.md` ;
- Texte libre livré en 0.11.0 ; création de listes modifiables et correspondances radio `/Opt` livrées en 0.12.0.
  Activation clavier livrée en 0.10.0 ; Narrateur Windows non vérifié
  (noms `labelled_by` posés, non écoutés).
- aplatissement : conserver aussi les annotations sans apparence autres que les liens (pièces jointes sans
  `/AP`, écrans) si un cas réel le demande ; aujourd'hui seuls les `/Link` sont remis en place ;

## Après sign-image (signature visuelle, tranche livrée)

Signatures saisies et pose sur toutes les pages/la sélection livrées en 0.13.0 (D102–D103).

Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A35–A37, décisions D054–D060) :
- chiffrement DPAPI des signatures enregistrées (ADR à écrire, D055) ;
- lissage des traits (Chaikin) si les courbes rapides paraissent anguleuses ; seuil adaptatif pour les photos à
  ombre portée ; pression du stylet si egui l'expose un jour.

Reste du hub (non bloquant) : focus clavier placé sur la ligne active à l'ouverture du panneau (F9) ;
validation visuelle finale par l'utilisateur. Le chevauchement du rail droit en fenêtre basse est déjà corrigé (A15).

## Après review-fixes (constats des revues adverses du 14/10, tranche livrée)
Les 39 constats de `docs/plans/review-fixes.md` sont corrigés ou déjà couverts (tests moteur et application).
Reste, non bloquant :
- élargir le corpus d'interopérabilité AES/XMP au-delà du contrôle qpdf synthétique (A4) ;
- contrôler les métadonnées PDF/A préservées avec un validateur dédié (A34) ; la correction de préservation 0.9.1
  est implémentée et les preuves d'exécution actuelles sont dans `PROJECT_STATUS.fr.md` ;
- incorporer les JPEG d'orientation EXIF 5 et 7 sans décodage (A31) ; limiter la mémoire des lots de PNG ;
- aide CLI de `stamp` / `page-numbers` (A16, clap remplace `{n}`).

## SUNI-012c — Affichage, suite (owner : UI-UX)
Fait : onglets (SUNI-012a) ; vue continue virtualisée, tuiles, zoom ancré, page unique en option (SUNI-012b,
`docs/plans/continuous-view.md`, D038–D045).
Reste (voir `KNOWN_ISSUES.fr.md` A25–A28) :
- rotation de vue/plein écran/historique implémentés en 0.10.0 ; validation dans `PROJECT_STATUS.fr.md` ;
- défilement animé des « aller à » ; pincement tactile à vérifier sur un écran tactile ;
- si une page lourde fige le défilement : `Document::render_regions` (une page chargée pour 3 à 4 tuiles
  voisines), à n'ajouter qu'après mesure ; plafonds mémoire réglables si un besoin apparaît.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Fait : sélection rectangulaire, copie (Ctrl+C réparé en SUNI-031c via l'événement Copier d'egui).
Implémenté en 0.9.0 : plages logiques PDFium, mots, sélection entre pages et défilement au bord. Reste : déduire un ordre complexe de colonnes non balisées.

## SUNI-031d — Édition du contenu, suite (owner : PDF-EDITOR)
Fait en SUNI-031/031b/032/031c : sélection simple et multiple (rectangle, Maj/Ctrl+clic, Ctrl+A), déplacement
groupé, redimensionnement, rotation libre, duplication, ordre Z (un cran ou extrême), suppression, couleurs,
épaisseur, opacité, copier/couper/coller entre pages et onglets (groupe), panneau Objets, texte sur la page
(polices standard et TrueType du système), remplacement d'image, pages vierges, recadrage, redimensionnement
des pages, pages pivotées et rognées.
Reste (voir `KNOWN_ISSUES.fr.md` A7, A8, A18–A22) :
- sous-ensemble des polices incorporées (crate `subsetter` MIT/Apache + post-traitement lopdf, ADR à écrire) ;
  polices `.otf` (contours CFF) et collections `.ttc` ;
- collage « natif » éditable (objets recréés un par un au lieu d'un XObject) ; opacité et couleur des groupes ;
- pivot mémorisé pour les tracés asymétriques (transformations groupées implémentées en 0.10.0) ;
- glisser-déposer dans le panneau Objets (recadrage souris implémenté en 0.10.0) ;
- lecture de l'opacité d'une image existante (contournement PDFium : lire l'ExtGState du flux avec lopdf).
Acceptation : chaque opération couverte par un test moteur avec rendu vérifié.

## SUNI-021b — Annotations avancées : livré, reste (owner : PDF-EDITOR)
Livré (D067–D076) : encre, zone de texte libre, rectangle, ellipse, ligne, flèche, liens cliquables, sélection /
déplacement / redimensionnement / recoloration / suppression, fils de réponses et statuts, auteur mémorisé, CLI
`annotations` / `annotate` / `reply` / `review-state` / `annotation-delete`. Reste :
- **vérifier l'interopérabilité** des réponses et statuts, des apparences d'encre, de texte libre et de formes dans
  Acrobat, Foxit et Chrome (non disponibles sur le poste de développement) ; un contrôle structurel ne prouve pas l’interopérabilité visuelle ;
- regroupement des traits d'encre (une annotation pour N traits, validée à l'inactivité) avec écriture garantie à
  l'enregistrement et à la fermeture ; gomme ;
- polygone, polyligne, nuage, tampon, pièce jointe, bordure pointillée, texte libre riche (gras, italique, CJK : police
  incorporée) ;
- création de vues de destination détaillées au-delà de Fit ; remplacement de cible livré en 0.15.0 (D106) ;
- aller-retour allégé pour les gros fichiers (écrire l'annotation sans recharger tout le document) ;
- zone de texte posée directement sur la page (éditeur en place, comme l'outil Texte) au lieu d'une boîte de dialogue.

## SUNI-025 — Compression avec perte et vérification qpdf (owner : OCR-CONVERSION + QA)
Livrés en 0.10.0 : rééchantillonnage borné des flux image/JPEG, quatre documents NIST publics figés et régressions
d'enregistrement interrompu/disque plein/échec. qpdf synthétique figé et longueur AES-256 explicite restent inclus.
Élargir aux documents clients/hostiles/volumineux licenciés ; preuves d'exécution dans `PROJECT_STATUS.fr.md`.

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Livrés : manifeste DPI-aware et ressources de version. L'icône Windows 0.9.1 dérive du visuel dédié fourni par l'auteur
et retouché `assets/icon/sunipdf-source.png` (ICO + tailles PNG).
Reste : signature du code des exécutables/MSI lorsqu'un certificat sera disponible.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
