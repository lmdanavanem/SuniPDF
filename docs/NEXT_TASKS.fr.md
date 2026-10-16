# Prochaines tâches (ordre conseillé)

[English](NEXT_TASKS.md)

## Lot actuel : 0.21.1 en préparation

Implémentation Done, contrôlée sur sa source antérieure localement et par GitHub CI/CodeQL : refus P1 des glyphes absents/récupération du
brouillon et conservation non-Link à l'aplatissement. Le projet compte 39 Todo, zéro In Progress et trois Done ;
source et preuves dans l'[état du projet](PROJECT_STATUS.fr.md). Contrôles de la source de release et
publication 0.21.1 en attente ; paquets 0.21.0 publiés inchangés.

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

## 0.15.0 : modifier les destinations de liens existants (jalon publié)

- Réutiliser la boîte de lien via Modifier la destination/double-clic, préremplir les destinations nommées/XYZ
  résolues et remplacer explicitement les vues de page par Fit. Cible inconnue : adresse vide, jamais page 1 implicite.
- Moteur/UI/CLI acceptent `http`/`https`/`mailto` ou une page physique, remplacent ensemble champs de cible/actions
  additionnelles et conservent zone/style/noms/commentaires/réponses, données indépendantes et protection (D106).
- Respecter ReadOnly/Locked/LockedContents directs/indirects ; drapeaux malformés : édition refusée. Un Annuler conserve
  géométrie/navigation. Éditions/changements d'onglet abandonnent les boîtes périmées ; protections CLI existantes conservées.
- [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous `lmdanavanem` (Latest à cette date), release
  `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Contrôles locaux réussis : 429 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés
  (744 pages ; 48,891 s, pic mémoire enfant 328 736 768 octets). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541) réussis,
  ainsi que treize scénarios portables (neuf mises à niveau dont 0.14.0 et quatre protections contre les échecs), sept
  bases MSI (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0/0.14.0), six fichiers préparés/publics et contrôles renforcés.
  Copies vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP Latest x64 de 19 803 226 octets
  sans installation. Preuves exécutées et limites restantes dans `PROJECT_STATUS.fr.md` (D106).

## 0.16.0 : annotations de formes en tirets (jalon publié)

- Rectangle/ellipse/ligne/flèche seulement : case Bordure en tirets et CLI `annotate --dashed` utilisent `[6,3]`
  points ; vecteur `dash_pattern` réel conservé lors de sélection/recoloration/déplacement/redimensionnement/
  annulation. Seul un changement explicite de case le remplace. Trait continu par défaut, remplissage continu et
  pointes pleines conservés (D107) ; toujours 53 commandes CLI.
- Dictionnaire de bordure et apparence explicite cohérents, phase zéro. Refuser les motifs non vides sur types
  non pris en charge et les régénérations de styles SuniPDF malformés/non pris en charge plutôt que les perdre.
  Validation du motif et bornes conservatrices du travail d'apparence empêchent une régénération excessive.
- Contrôles locaux réussis sur `6cd7843734f3924e87033d8b05539d493ccbcb95` : 439 tests workspace (zéro échec,
  sept ignorés par défaut), formatage/Clippy, 895 clés EN/FR, 73 contrôles qpdf et quatre documents licenciés
  (744 pages ; 53,542 s, pic mémoire enfant 331 517 952 octets). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589) réussis.
  Les quatorze scénarios portables (dix mises à niveau dont 0.15.0 et quatre protections), huit bases MSI
  (0.8.0/0.9.1/0.10.0/0.11.0/0.12.0/0.13.0/0.14.0/0.15.0) et 40 journaux MSI ont été vérifiés. Corpus en CI :
  47,628 s, pic mémoire enfant 321 306 624 octets. [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0)
  sous `lmdanavanem` (Latest à cette date), release `414934190`, avec six fichiers. Fichiers préparés/publics et contrôles renforcés réussis ;
  copies vérifiées dans `dist/`. Le metteur à jour de production a vérifié le ZIP x64 de 19 821 719 octets sans
  installation sur l'hôte. Preuves exécutées et limites dans `PROJECT_STATUS.fr.md`.

## 0.17.0 : fiabilité de la conversion d'images (publiée ; Latest à cette date)

- Incorporer directement les JPEG EXIF 5 et 7 sans recompression : huit orientations via les matrices existantes.
  Conserver qualité/transparence des non-JPEG et les accès bureau/CLI (D108).
- Avant allocation raster, borner à 512 Mio l'estimation non-JPEG cumulée :
  `somme(max(decoder.total_bytes(), largeur × hauteur × 4))`, arithmétique vérifiée et tailles physiques finies/
  positives. Répéter les contrôles au chargement pour les changements simples de fichiers. Aucun plafond RSS
  natif/processus ni limite JPEG comprimé ; coût du décodeur, de PDFium et de sortie distinct.
- Méthode de conversion/accès et 53 commandes CLI conservés ; aucune nouvelle dépendance. Dépassement/overflow
  du budget : `image_batch_too_large`, avec une clé d'erreur EN/FR ; géométrie invalide : `other` conservé.
- Contrôles locaux réussis : 445 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy all-targets,
  896 clés EN/FR, 74 contrôles qpdf, quatre documents licenciés/744 pages (48,187 s ; pic mémoire enfant
  330 264 576 octets) et autotest du helper de mise à jour. Conversion debug indépendante réussie : huit orientations,
  neuf flux JPEG exacts, 32 coins JPEG/quatre PNG et masque alpha, trois contrôles qpdf, deux refus budget/dix DPI,
  treize sources inchangées (35 sous-processus, 1,063 s).
- Source `5785de282e126fb4666b28fb83340d0e9c6e9acf` : [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
  et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
  réussis, ainsi que quinze scénarios portables, neuf bases MSI dont 0.16.0 et 45 journaux MSI vérifiés.
  Corpus en CI : 45,791 s, pic mémoire enfant 321 413 120 octets. [Publiée (Latest à cette date)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0)
  sous `lmdanavanem`, release `415074108`, avec six fichiers. Fichiers préparés/publics et contrôles renforcés de
  conversion réussis, dont flux JPEG exacts, coins/transparence et refus de budget sans allocation raster. Copies
  vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP x64 de 19 825 743 octets sans installation
  sur l'hôte. Preuves et limites dans `PROJECT_STATUS.fr.md`.

## 0.18.0 : dessin à main levée groupé et aide CLI littérale (publiée)

Regrouper les traits à main levée d'une page en une annotation et une étape Annuler via **Terminer le dessin** /
**Annuler le dessin**. Une erreur du worker conserve le brouillon pour réessayer ; les actions incompatibles
enregistrement/fermeture/document/outil attendent Terminer/Annuler. Réutiliser AddInk, simplification des points,
géométrie affichée et protection (D109). Au plus 64 traits et 4 000 points par trait. Brouillon en mémoire
jusqu'à Terminer, absent de la récupération après plantage. Aucune validation à l'inactivité ni gomme.
L'aide de `stamp` / `page-numbers` conserve `{n}` / `{total}` littéraux via le modèle clap existant (A16).
Aucune nouvelle dépendance ni commande CLI ; 53 commandes.

[Publiée en Latest stable à cette date](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) sous `lmdanavanem`, release `415140702`,
source `7e52de82244faf9a020ed3ed233176775c007e97`. Contrôles locaux réussis : 452 tests workspace (zéro échec, sept ignorés par défaut),
formatage/Clippy sans avertissement, 907 clés EN/FR correspondantes, 75 qpdf et autotest du metteur à jour.
Quatre documents NIST/NBS licenciés (744 pages) réussis en 50,796 s, pic mémoire enfant échantillonné
329 445 376 octets ; toutes les sources inchangées. Six nouveaux tests UI sans fenêtre, régression worker native
étendue Annuler/Rétablir groupés/échec et régression d'aide CLI réussis.
[CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082734) et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082934/job/117057854520) réussis,
avec seize scénarios portables, dix bases MSI et 50 journaux MSI vérifiés. Corpus CI : 44,494 s, pic mémoire
enfant échantillonné 320 299 008 octets. Six fichiers préparés/publics, empreintes API/sidecars et contrôles renforcés
réussis : cinq relectures Ink groupé (clair/AES-128/AES-256 avec les deux mots de passe), quatre aides littérales
et contrôles existants conversion, formulaires, étiquettes, liens, tirets, compression, OCR et contenu MSI.
Copies vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP Latest x64 de 19 834 488 octets
sans installation sur l'hôte. Preuves exécutées et limites dans `PROJECT_STATUS.fr.md`.

## 0.19.0 : placement de champs à la souris (publiée ; Latest à cette date)

1. Réutiliser **Préparer un formulaire** et les cinq types existants. **Dessiner sur la page** masque la boîte
   numérique ; le geste rouvre le même brouillon avec nom, choix et options modifiable/sélection multiple conservés.
2. Garder un rectangle fini, borné à une page et d'au moins quatre points par dimension. Afficher réticule/aperçu,
   exclure les widgets existants et annuler un placement périmé après changement de révision/onglet/outil.
3. Modifier seulement par **Créer le champ** explicite, avec validation moteur, chiffrement et une étape Annuler.
   Création numérique et API moteur/CLI conservées ; toujours 53 commandes, aucun nouveau type ni dépendance.
4. Interactions sans fenêtre, paramètres conservés, gestes minuscules/hors page, cibles périmées, sauvegarde automatique
   et régression étendue création/annulation des cinq types réussis dans le workspace. Revue indépendante, formatage et Clippy réussis.
   Contrôles CI/mises à niveau, paquets préparés/publics et metteur à jour réussis ; six fichiers vérifiés publiés. Résultats exécutés dans l'état du projet.

**0.19.0 était la Latest stable à sa publication.** D110/A38 définissent le périmètre ; création XFA et édition de la géométrie/des types
de champs existants exclues. Souris/stylet/tactile natifs et Narrateur restent distincts.
Preuves exécutées dans `PROJECT_STATUS.fr.md`.

## 0.20.0 : création et édition FreeText sur la page (publiée ; Latest à cette date)

Le lot 0.20.0 implémenté crée et modifie les annotations FreeText directement sur la page (D111/A39).
Cliquer ou tracer la boîte existante, saisir dans un éditeur multiligne ancré et utiliser **Ajouter** /
**Enregistrer** / **Annuler** explicites avec les réglages existants de taille, couleur, fond et bordure.
Les opérations worker AddFreeText / UpdateFreeText conservent validation, protection et une étape Annuler ;
aucune nouvelle dépendance, type d'annotation ou commande CLI ajouté (53 commandes conservées).

Le brouillon reste lié à sa page d'origine. Le zoom reste disponible ; la navigation incompatible attend
validation/annulation. Révision du document/changement d'onglet/outil/espace de travail : édition périmée
annulée. Ouverture/enregistrement/mutations en attente et brouillons concurrents sont protégés ; erreur de
validation ou du worker : texte saisi conservé pour réessayer. Apparence Helvetica WinAnsi et agrandissement
vers le bas au renvoi à la ligne inchangés. Brouillon en mémoire jusqu'à validation, sans récupération après plantage.

Implémentation et relecture indépendante réussies. Workspace local : 468 tests réussis, aucun échec, sept
ignorés par défaut ; formatage, Clippy toutes cibles sans avertissement et 912 clés EN/FR correspondantes réussis.
Les 75 contrôles qpdf, corpus réel de quatre documents/744 pages et autotest du metteur à jour ont réussi.
CI Windows, 18 scénarios portables, 12 bases MSI/60 journaux, ARM64 de même source, six fichiers préparés/publics
et metteur à jour de production réussis. **0.20.0 était publiée en Latest stable à cette date** ; l'[état du projet](PROJECT_STATUS.fr.md) consigne les preuves exécutées.

La [revue Code Scanning](CODE_SCANNING_REVIEW.fr.md) initiale couvrait 117 alertes alors ouvertes, dont 115 critiques ;
aucun secret de production confirmé. À cet instantané, aucune alerte n'avait été écartée et la déclaration
de permissions du workflow de release restait à faire. Le résultat vérifié de la 0.21.0 figure ci-dessous.

## 0.21.0 : traitement des alertes Code Scanning (publiée en Latest stable)

Contrôle explicite du certificat signataire natif non nul et régression CMS malformé ; permissions
`contents: read` à la racine de release, `contents: write` réservé à Publish (D112). Lecteurs XML 1.0
migrés vers `ParserConfig`, bornes conservées et DTD refusées ; régressions XMP/XFDF UTF-8/UTF-16 réussies.
Les fixtures déterministes et toute la couverture de scanning sont conservées.

471 tests, zéro échec, sept ignorés par défaut ; formatage/Clippy, 912 clés EN/FR, 75 qpdf, corpus 4/744 et
autotest réussis. CI, 19 scénarios portables (15 mises à niveau/quatre gardes), 13 bases MSI/65 journaux,
ARM64 de même source, six fichiers préparés/publics, copies `dist/` et metteur à jour de production vérifiés.
CodeQL a corrigé #3 et #119 ; aucune alerte ouverte. Les 115 critiques ont été écartées individuellement
par `lmdanavanem` avec preuve : 114 `used in tests`, #13 `false positive`. Aucun dossier ni requête exclu.

Le [projet GitHub](https://github.com/users/lmdanavanem/projects/1) contient 42 brouillons priorisés : Bugs (8), Roadmap (23), Suggestions (11),
avec filtres de catégorie enregistrés. À la publication 0.21.0, la sécurité était Terminé ; l'état actuel
des autres éléments est suivi dans PROJECT_STATUS.fr.md. Voir la [revue Code Scanning](CODE_SCANNING_REVIEW.fr.md)
et l'[état du projet](PROJECT_STATUS.fr.md). 53 commandes CLI conservées.

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
- aplatissement : conservation des annotations non-Link sans apparence utilisable, pièces jointes et
  références partagées vérifiée localement et par GitHub CI/CodeQL sur sa source d'implémentation.
  Contrôles de release et publication 0.21.1 en attente ; 0.21.0 publiée inchangée ;
  [contrôles exécutés et limites](PROJECT_STATUS.fr.md).

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
- incorporation directe JPEG EXIF 5/7 et estimation PNG/raster cumulée livrées en 0.17.0 (A31/D108) ;
  mesure de la mémoire native/processus et entrées licenciées volumineuses restent distinctes ;
- aide CLI de `stamp` / `page-numbers` incluse dans le lot 0.18.0 publié (A16) ; contrôles locaux/CI/paquets réussis.

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
- encre groupée livrée en 0.18.0, avec Terminer/Annuler explicites et transitions
  enregistrement/fermeture protégées (D109), remplaçant la proposition à l'inactivité ; régressions locales/CI/paquets réussies.
  La gomme reste future ;
- polygone, polyligne, nuage, tampon, pièce jointe et texte libre riche (gras, italique, CJK : police incorporée) ;
  bordures en tirets des quatre formes livrées en 0.16.0, autres types toujours non pris en charge ;
- création de vues de destination détaillées au-delà de Fit ; remplacement de cible livré en 0.15.0 (D106) ;
- aller-retour allégé pour les gros fichiers (écrire l'annotation sans recharger tout le document) ;
- création et édition FreeText sur la page livrées en 0.20.0 (D111) ; contrôles locaux/CI/mises à niveau, paquets préparés/publics et metteur à jour de production réussis.

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
