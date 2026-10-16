# Roadmap SuniPDF

[English](ROADMAP.md)

Cette roadmap distingue les fonctions livrées et le travail futur.
Les contrôles exécutés et preuves de publication sont dans [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md).

## Travail actuel main/non publié

Correctifs P1 glyphes absents/récupération du brouillon et conservation non-Link validés localement.
GitHub CI/CodeQL en attente, les deux éléments In Progress. Voir l'[état du projet](PROJECT_STATUS.fr.md) ; 0.21.0 inchangée.

## Base livrée : 0.9.0

Les sept fonctions demandées sont livrées : sauvegarde/récupération, impression Windows, sélection enrichie,
vues double page/Livre, création de champs texte/case, OCR anglais/français hors ligne et signatures numériques CMS/PFX.
L’anglais est la langue par défaut ; le français reste disponible. Les 14 outils ont leurs commandes.
Voir [FEATURE_MATRIX.fr.md](FEATURE_MATRIX.fr.md) pour le périmètre exact et [KNOWN_ISSUES.fr.md](KNOWN_ISSUES.fr.md) pour les limites.

## 0.9.1 : fiabilité et distribution

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) sous `lmdanavanem` ;
validations exécutées et preuves des paquets suivies dans `PROJECT_STATUS.fr.md` :

- Préserver l’identification PDF/A et les propriétés XMP indépendantes lors de l’édition des métadonnées ; refuser les XML malformés et DTD.
  La conservation de l’identification ne certifie pas la conformité PDF/A.
- Écrire explicitement la longueur de clé AES-256 attendue par les lecteurs stricts, y compris après rechiffrement.
- Ajouter en CI des contrôles structurels qpdf figés sur les fixtures synthétiques et les sorties produites.
  Il s’agit d’une QA structurelle indépendante, pas d’une couverture de corpus réel ou hostile.
- Sept scénarios portables réels de mise à jour/échec/restauration vers 0.9.1 et régression MSI réelle d’installation/mise à niveau/rollback transactionnel,
  refus de downgrade et désinstallation réussis sur runners Windows jetables, avec conservation des données.
- Calculer SHA256 avec .NET dans le helper pour les hôtes Windows PowerShell 5 sans Get-FileHash et signaler les erreurs de relance.
- Dériver l’icône Windows du visuel dédié fourni par l’auteur et retouché (`assets/icon/sunipdf-source.png`).

Contrôles du workspace (322 tests), qpdf indépendant (22 PDF/trois entrées malformées), helper portable et régression MSI
sur runner réussis. Les six fichiers publics ont été téléchargés et correspondent aux contenus/empreintes vérifiés.
L’UX interactive de mise à jour et le matériel réel restent des vérifications distinctes.

## 0.10.0 : lot P1/P2 demandé publié

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) sous `lmdanavanem`.

| Domaine | Périmètre inclus |
|---|---|
| P1 sauvegardes/corpus | Remplacement atomique, régressions interruption/disque plein/échec, processus d’écriture partielle arrêté et quatre documents NIST publics figés |
| P2 compression | Rééchantillonnage/JPEG, tailles avant/après, progression/annulation, nouvelles copies et conservation prudente des images non prises en charge |
| P2 lecture | Rotation de vue, plein écran, historique borné par onglet et édition en vue droite |
| P2 formulaires | Création radio/liste déroulante/zone de liste, export XFDF, import de valeurs JSON/XFDF/FDF borné, Appliquer/Abandonner groupés et widgets clavier |
| P2 édition | Recadrage souris dans l’aperçu et rotation/redimensionnement groupés, une annulation |
| P2 lots | Compression/OCR local séquentiels, entrées bornées, annulation, erreurs individuelles et aucun écrasement |

Les 362 tests du workspace, 31 contrôles qpdf synthétiques indépendants, quatre documents NIST publics (744 pages),
huit scénarios portables de mise à jour/échec/restauration et deux bases MSI ont réussi. CI finale et six fichiers publics
vérifiés, y compris CLI/OCR embarqués, icônes, contenu MSI et mise à jour en ligne ; preuves dans `PROJECT_STATUS.fr.md`.
Matériel, dialogues natifs interactifs, Narrateur, lecteurs tiers et corpus client/hostile élargi restent indépendants.

## 0.11.0 : navigation, formulaires et copies protégées (publiée)

Implémentés et validés localement : étiquettes en lecture seule, navigation par étiquette exacte et unique ou
indice physique `#N` ; destinations nommées et `/XYZ`, `/Fit`, `/FitH` non nul des liens/signets ; historique des sauts
dans la même page et du zoom ; erreurs de lecture de formulaire préservant les saisies avec relance explicite.
`/FitH` zéro/null et les autres modes gardent le saut de page. Aussi implémentés pour 0.11.0 : texte libre des listes
déroulantes déjà modifiables dans le panneau/sur la page, et copies d'extraction/division conservant chiffrement,
mots de passe et permissions, avec refus du chemin source. Politique des permissions inchangée ; création du
drapeau de liste modifiable hors du périmètre 0.11.0.

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

## 0.12.0 : création de listes modifiables et exports radio (publiée ; Latest à cette date)

Créer des listes déroulantes modifiables depuis Préparer un formulaire ou `add-form-field --kind combo --editable`,
avec choix prédéfinis toujours requis. Les exports radio `/Opt` sont distingués des états d'apparence pour remplir
et échanger JSON/XFDF/FDF ; informations CLI enrichies de `widgets[].export_value`. Une correspondance malformée ou
dupliquée garde le champ concerné en lecture seule ; sans `/Opt`, les noms d'état restent les valeurs radio.
Décisions D100–D101.

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) sous `lmdanavanem` (Latest à cette date), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Les 398 tests workspace (zéro échec, sept ignorés par
défaut), formatage/Clippy, 859 clés EN/FR, 48 contrôles qpdf et quatre documents NIST/NBS (744 pages ; 59,68 s)
sont réussis. [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688),
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878), dix scénarios
portables et quatre bases MSI réussis. Six fichiers préparés/publics et contrôles renforcés des binaires réussis ;
copies vérifiées dans `dist/`. L'updater de production a vérifié le ZIP Latest x64 de 19 699 323 octets sans installation.
Preuves exécutées et limites matérielles/d'interopérabilité dans `PROJECT_STATUS.fr.md`.

## 0.13.0 : améliorations de signature visuelle (publiée ; Latest à cette date)

Nom/paraphe saisi avec aperçu natif explicite et parcours police/image/bibliothèque existants. Page courante,
toutes les pages et sélection partagent une géométrie affichée ajustée ; lot atomique, annulable entre cibles et
un undo. CLI `add-image --pages` additive ; comportement historique de `--page` compatible (D102–D103).
[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem` (Latest à cette date), release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation locale réussie : 410 tests workspace,
formatage/Clippy, 870 clés EN/FR, 53 contrôles qpdf et quatre documents licenciés (744 pages ; 76,40 s localement).
CI Windows, compilation ARM64 de même source, onze scénarios portables, cinq bases MSI et six fichiers préparés/publics
avec contrôles renforcés des binaires réussis. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le
ZIP Latest x64 de 19 730 578 octets sans installation. Preuves et limites matérielles/lecteurs : `PROJECT_STATUS.fr.md`.

## 0.14.0 : création d'étiquettes de page (publiée ; Latest à cette date)

Appliquer six styles de navigation sur une plage physique inclusive via Pages/Organiser et deux commandes CLI ;
conserver les étiquettes hors plage, refuser un arbre existant malformé avant mutation, réinitialiser toutes les
règles sans annoncer d'effacement sécurisé. Numéros imprimés inchangés ; aucun recalage sémantique des plages
après opérations structurelles. Annuler/Rétablir contenu/étiquettes conserve la navigation si géométrie inchangée
(D104–D105). [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) sous `lmdanavanem` (Latest à cette date),
release `414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation réussie : 420 tests workspace,
formatage/Clippy, 890 clés EN/FR, 63 contrôles qpdf et quatre documents licenciés (744 pages ; 53,91 s localement).
CI Windows, compilation ARM64 de même source, douze scénarios portables, six bases MSI et six fichiers préparés/publics
avec contrôles renforcés réussis. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le ZIP Latest x64
de 19 785 200 octets sans installation. Preuves exécutées et limites restantes : `PROJECT_STATUS.fr.md`.

## 0.15.0 : modifier les destinations de liens existants (jalon publié)

Réutiliser boîte de lien et writer protégé pour changer une destination via Modifier la destination, double-clic
ou CLI `set-link-target`. URI web/mail autorisée ou page physique Fit remplace tous les anciens champs de cible/
actions, en conservant zone/style/commentaires/réponses et données indépendantes. Vue détaillée/nommée explicitement
remplacée par Fit ; une cible inconnue ne devient jamais silencieusement page 1. ReadOnly/Locked/LockedContents
et drapeaux malformés refusés ; un Annuler conserve géométrie/navigation, boîtes périmées fermées au changement
de document (D106). [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous
`lmdanavanem` (Latest à cette date), release `414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation réussie : 429 tests
workspace, formatage/Clippy, 894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés (744 pages ; 48,891 s
localement). CI Windows, compilation ARM64 de même source, treize scénarios portables, sept bases MSI et six fichiers
préparés/publics avec contrôles renforcés réussis. Copies vérifiées dans `dist/` ; le metteur à jour de production
a vérifié le ZIP Latest x64 de 19 803 226 octets sans installation. Preuves exécutées et limites : `PROJECT_STATUS.fr.md`.

## 0.16.0 : annotations de formes en tirets (jalon publié)

Rectangles/ellipses/lignes/flèches en tirets réutilisent barre, CLI et writer d'apparences. UI et `annotate --dashed`
choisissent `[6,3]` points ; motifs personnalisés réels conservés lors des éditions ordinaires sauf changement
explicite de case. Trait continu par défaut, remplissage continu et pointes pleines inchangés. Types non pris en
charge, styles SuniPDF malformés et travail de régénération excessif refusés (D107). Aucune dépendance ni commande
ajoutée, toujours 53 commandes CLI. [Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0)
sous `lmdanavanem` (Latest à cette date), release `414934190`, source `6cd7843734f3924e87033d8b05539d493ccbcb95`. Validation locale,
CI Windows et compilation ARM64 de même source réussies : 439 tests, 895 clés EN/FR, 73 contrôles qpdf, quatre
documents licenciés/744 pages (53,542 s localement), quatorze scénarios portables et huit bases MSI. Six fichiers
préparés/publics et contrôles renforcés réussis ; copies vérifiées dans `dist/`. Le metteur à jour de production a
vérifié le ZIP x64 de 19 821 719 octets sans installation. Preuves et limites dans `PROJECT_STATUS.fr.md`.

## 0.17.0 : fiabilité de la conversion d'images (publiée ; Latest à cette date)

Compléter l'incorporation directe JPEG EXIF 5 et 7 : les huit orientations évitent décodage/recompression d'image.
Contrôler les non-JPEG avant allocation avec une estimation raster cumulée de 512 Mio :
`somme(max(decoder.total_bytes(), largeur × hauteur × 4))`, arithmétique vérifiée, géométrie physique finie/positive
et contrôles répétés au chargement pour les changements simples de fichiers (D108). Qualité/transparence et
UI/CLI/API existantes conservées. Estimation distincte d'un plafond RSS natif/processus ou d'une limite JPEG
comprimé. Aucune dépendance ni commande ajoutée ; erreur localisée `image_batch_too_large` pour dépassement/overflow
du budget, `other` conservé pour géométrie invalide. Validation locale réussie : 445 tests workspace, formatage/
Clippy, 896 clés EN/FR, 74 contrôles qpdf et quatre documents licenciés/744 pages (48,187 s ; pic mémoire enfant
330 264 576 octets), autotest du helper de mise à jour et contrôle de conversion debug indépendant. Source
`5785de282e126fb4666b28fb83340d0e9c6e9acf` : CI Windows, compilation ARM64 de même source, quinze scénarios
portables, neuf bases MSI et 45 journaux MSI vérifiés ; corpus en CI 45,791 s/pic mémoire enfant 321 413 120 octets.
[Publiée (Latest à cette date)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) sous `lmdanavanem`, release
`415074108`, avec six fichiers. Fichiers préparés/publics et contrôles renforcés de conversion réussis ; copies
vérifiées dans `dist/`. Le metteur à jour de production a vérifié le ZIP x64 de 19 825 743 octets sans installation.
Preuves et limites dans `PROJECT_STATUS.fr.md`.

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

## 0.19.0 : placement à la souris des types de formulaire existants (publiée ; Latest à cette date)

Ajouter **Dessiner sur la page** au builder numérique, en conservant texte/case/radio/listes déroulantes/zones
de liste, noms, choix et options. Tracer un rectangle fini borné à une page, d'au moins 4 pt par dimension,
puis relire la boîte numérique rouverte et **Créer le champ** explicitement. Le tracé seul ne modifie pas le PDF ;
l'édition worker existante fournit une étape Annuler, validation et chiffrement (D110/A38).

Réticule/aperçu, exclusion des widgets et gardes de document/onglet/outil périmés inclus. Création numérique,
API moteur/CLI et 53 commandes inchangés ; aucune dépendance ni type ajouté. Implémentation et tests locaux
réussis, avec formatage/Clippy ; contrôles CI/paquets/mise à jour/publication réussis. **0.19.0 était la Latest stable à sa publication**.
Preuves réelles dans `PROJECT_STATUS.fr.md`.
Édition de la géométrie existante, création XFA et contrôles matériels natifs restent distincts.

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

## Phases 0–1 : planification et fondations — livrées

Workspace Rust, PDFium, erreurs typées, ouverture/navigation/zoom/fermeture, mots de passe, glisser-déposer,
localisation FR/EN, paramètres persistés, thèmes clair/sombre, registre de commandes et ressources Windows sont en place.
Les jalons et décisions historiques restent dans `DECISIONS.fr.md` et `PROJECT_STATUS.fr.md`.

## Phase 2 : lecture et opérations PDF essentielles — largement livrée

Livrés : onglets, caches de rendu/miniatures bornés, vues continue/double page/Livre, sélection logique entre pages,
recherche, signets, fusion/division/extraction/rotation/réorganisation, enregistrement atomique, annulation/rétablissement et annotations.

Livrés en 0.10.0 : rotation de vue, plein écran et historique borné par onglet.
Livrés en 0.11.0 : lecture des étiquettes et navigation interne précise. Création/réinitialisation et navigation
conservée par Annuler/Rétablir livrées en 0.14.0. Restent : recalage sémantique des étiquettes et ordre voulu des colonnes complexes non balisées. qpdf ne prouve pas la préservation de
chaque lien, champ, calque ou pièce jointe.

## Phase 3 : édition professionnelle — partiellement livrée

Livrés : texte/images/formes, transformations d’objets, déplacement groupé et presse-papiers, polices système TrueType,
tampons/filigranes/numérotation, commentaires, métadonnées, signatures visuelles, remplissage/aplatissement AcroForm,
création de champs texte/case et sauvegarde/récupération.

Livrés en 0.10.0 : transformations groupées, recadrage souris, champs radio/listes, échanges FDF/XFDF de valeurs et clavier.
Livrés en 0.12.0 : création de listes modifiables et exports radio `/Opt`, avec lecture seule conservatrice si nécessaire.
Livrés en 0.13.0 : signatures visuelles saisies et pose atomique sur page courante/toutes/sélection.
Livré en 0.15.0 : remplacement de destination existante, zone/apparence/commentaires conservés.
Livré en 0.16.0 : tirets des quatre annotations de formes, motifs personnalisés conservés.
Livré en 0.18.0 : encre groupée du bureau, Terminer/Annuler explicites et transitions protégées (D109).
Restent : sous-ensembles de polices/CFF/OTF/TTC, paragraphes et collage éditable.
Critère : rendu, enregistrement et réouverture couverts pour chaque opération, avec contrôles d’accessibilité applicables.

## Phase 4 : conversion et OCR — partiellement livrée

Livrés : images ↔ PDF, PDF → texte/PNG, optimisation sans perte, lots CLI et OCR anglais/français local embarqué.

Livré en 0.17.0 : huit orientations JPEG EXIF incorporées directement et estimation raster cumulée ;
mesure des limites de mémoire native/processus distincte (D108).

Restent : Office → PDF via convertisseur local détecté, PDF → Word/Excel/PowerPoint avec limites de fidélité mesurées
et explicites, profils réutilisables. Compression des images et lots compression/OCR livrés en 0.10.0.

## Phase 5 : sécurité et productivité — partiellement livrée

Livrés : AES-128/256 et permissions, retrait de protection autorisé par le propriétaire, nettoyage des métadonnées,
caviardage irréversible, vérification CMS et copies signées PFX/P12 incrémentales, CLI JSON et automatisation.

Restent : comparaison, recherche multi-documents, MCP opt-in confiné, intégration Explorateur et raccourcis configurables.
Confiance de chaîne, révocation, horodatage fiable, AATL/EUTL, PKCS#11 et LTV sont **non implémentés** ;
l’heure de signature PDF est déclarative. L’interopérabilité Acrobat/Foxit est une propriété distincte, non vérifiée.
La signature du code/MSI demande une identité appropriée fournie.

## Phase 6 : qualité et distribution — en cours

Base livrée : releases x64 portable/MSI et ARM64 portable, SHA256, notification au démarrage avec installation explicite,
guides FR/EN, notices tierces et workflows de compilation GitHub.

Livrés en 0.10.0 : quatre documents NIST publics et scénarios d’enregistrement interrompu/disque plein/échec.
Restent : corpus client/hostile/volumineux plus large, fuzzing,
benchmarks avec matériel, inventaire complet des licences, Narrateur et lecteurs tiers, parcours interactifs de mise à
jour MSI/portable, imprimantes physiques et exécution ARM64/OpenGL réelle.
La régression MSI ne couvre que son scénario scripté ; compiler un paquet ARM64 ne valide pas son exécution matérielle.

La [revue Code Scanning](CODE_SCANNING_REVIEW.fr.md) couvre les 117 alertes ouvertes, dont les 115 critiques,
sans secret de production confirmé parmi ces critiques et sans alerte écartée. Déclarer explicitement les permissions
du workflow de release reste un durcissement futur ; droits du dépôt observés en lecture seule par défaut.

## Suite au-delà du lot 0.20.0 publié

1. Exécuter les dialogues natifs de mise à jour/enregistrement, Narrateur, imprimantes, ARM64 et lecteurs tiers.
2. Élargir corpus réel et mesures de performance ; les échantillons NIST ne prouvent pas une compatibilité universelle.
3. Définir séparément polices/paragraphes, conversion Office locale et services de confiance.

Aucune date/version suivante n’est engagée. L’IA facultative reste désactivée par défaut.
