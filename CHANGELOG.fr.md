# Journal des versions

[English](CHANGELOG.md)

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions sont des tags `vX.Y.Z` ;
`release.yml` construit et publie les artefacts (ZIP portable Windows x64/ARM64, MSI x64, DEB Ubuntu amd64) sur la page Releases GitHub.

## [Non publié]

## [0.60.0] - 2026-10-23

La 0.60 ajoute la disposition et les vues du registre en cache, la recherche et l'inventaire coopératifs page par page, les tables ancrables, l'accrochage vectoriel, la saisie pieds/pouces, les zones d'échelles indépendantes et le recalcul atomique avec aperçu et Annuler. La qualification sur plans privés réels est reportée à la demande du propriétaire ; les autres validations 1.0 restent ouvertes.

[Notes de version](docs/RELEASE_0.60.0.fr.md) · [Preuves](docs/qa/RELEASE_0.60.0.json).

## [0.51.0] - 2026-10-23

Installez SuniPDF sur Ubuntu 22.04 ou 24.04 amd64 avec le nouveau paquet Debian : interface de bureau, CLI, PDFium privé, OCR local français/anglais et entrée dans le menu des applications. Les paramètres Linux suivent les dossiers XDG ; les chemins récents conservent la casse et les sessions de récupération utilisent un verrou entre processus. Les notifications de mise à jour ouvrent le téléchargement Ubuntu ; l’installation utilise le gestionnaire de paquets. Les paquets Windows restent disponibles.

[Notes de version](docs/RELEASE_0.51.0.fr.md) · [Preuves](docs/qa/RELEASE_0.51.0.json).

## [0.50.0] - 2026-10-22

Retrouvez 18 outils dans cinq familles filtrables depuis Outils, avec Mesures et la bibliothèque sous Réviser et mesurer. Basculez le volet gauche entre Outils, Pages et Signets ; utilisez quatre sections de mesure à droite. Les commandes de page et de zoom sont sous le PDF, les pourcentages comme 149% restent sur une ligne et les options de lecture restent accessibles dans les dispositions compactes. La rotation de lecture change uniquement l’affichage ; le rail droit ouvre les panneaux. Un seul panneau contextuel occupe le côté droit. Un brouillon de mesure actif bloque l’ouverture des autres panneaux ; quitter explicitement l’outil ou en changer annule ce brouillon local et conserve les échelles confirmées.

Les paquets publiés 0.49.0 rejoignent les régressions de mise à jour. Les contrôles hors écran de l’interface et les régressions CLI conservées restent distincts de la qualification native.

[Notes et limites](docs/RELEASE_0.50.0.fr.md) · [Preuves](docs/qa/RELEASE_0.50.0.json).

## [0.49.0] - 2026-10-22

Ajoutez ou supprimez des sommets de polylignes, périmètres et surfaces polygonales enregistrés, y compris sur les segments de fermeture, et sélectionnez un point par son numéro lorsque les poignées se confondent. Prévisualisez, ajustez puis appliquez avec une étape d’annulation. Calibration, unité, précision, identité, réponses et champs de quantité saisis sont conservés ; valeurs mesurées et totaux tarifaires compatibles sont recalculés. Les modifications périmées, invalides ou interdites restent contrôlées.

La version publiée 0.48.0 rejoint les régressions de mise à jour. Les séquences d’insertion et de suppression vérifient réouverture, conservation des données et recalcul des quantités ; les types à nombre fixe de points et les bornes restent contrôlés.

[Notes et limites](docs/RELEASE_0.49.0.fr.md) · [Preuves](docs/qa/RELEASE_0.49.0.json).

## [0.48.0] - 2026-10-22

Modifiez les sommets existants d’une mesure enregistrée sur sa page d’origine, examinez l’aperçu, ajustez le brouillon puis appliquez avec une seule étape d’annulation. La mesure conserve ses propres calibration, unité, précision, identité, réponses et champs de quantité saisis. Valeurs mesurées et totaux tarifaires compatibles suivent la géométrie validée ; les modifications périmées, invalides ou interdites restent contrôlées.

Le chemin CI documentaire contrôle les guides réellement empaquetés, liens, backlog et preuves sans relancer les tests applicatifs pour un diff exclusivement documentaire admissible ; toute modification du code, des scripts ou workflows conserve les contrôles complets. La version publiée 0.47.0 rejoint les régressions de mise à jour.

[Notes et limites](docs/RELEASE_0.48.0.fr.md) · [Preuves](docs/qa/RELEASE_0.48.0.json).

## [0.47.0] - 2026-10-22

Superposez deux pages PDF enregistrées en magenta/cyan, alignez-les par translation, rotation horaire et échelle explicites ou par deux paires de points correspondants, puis exportez un nouveau PNG ou rapport JSON lié aux sources. Les deux fichiers doivent autoriser la copie ; empreintes exactes, géométrie recadrée/pivotée, rasters bornés et sorties nouvelles préservent les originaux.

[Notes et limites](docs/RELEASE_0.47.0.fr.md) · [Preuves](docs/qa/RELEASE_0.47.0.json).

## [0.46.0] - 2026-10-22

Consultez les comptages et mesures réels dans des tables filtrées, modifiez champs typés et prix unitaires explicites avec Annuler, puis exportez CSV/JSON liés à la source. Les totaux distinguent absence et zéro, convertissent les bases de prix compatibles et séparent les monnaies. La CLI exige un enregistrement attendu et une nouvelle copie PDF ; droits de copie et d’annotation restent contrôlés.

[Notes et limites](docs/RELEASE_0.46.0.fr.md) · [Preuves](docs/qa/RELEASE_0.46.0.json).

## [0.45.0] - 2026-10-22

Enregistrez des brouillons d’annotations pris en charge comme modèles nommés, regroupez jusqu’à 16 primitives puis placez un aperçu à une origine et une échelle explicites, avec une seule étape Annuler et de nouvelles identités. Le JSON autonome strict permet import, export et suppression ; la CLI écrit une nouvelle copie PDF en conservant chiffrement et permissions.

[Notes et limites](docs/RELEASE_0.45.0.fr.md) · [Preuves](docs/qa/RELEASE_0.45.0.json).

## [0.44.0] - 2026-10-22

Modifiez l’échelle, l’unité et la précision d’une mesure enregistrée avec Aperçu puis Appliquer, en conservant son identité, ses réponses et les données du registre. La CLI modifie aussi la géométrie dans une nouvelle copie, protégée par un enregistrement attendu. Les exports respectent le droit de copie ; le redimensionnement incompatible de pages mesurées est refusé sans modification partielle. L’inspecteur partage ses données en cache et les échelles temporaires peuvent être retirées explicitement. Aucune calibration manuelle implicite n’est fournie.

[Périmètre, audit et état de qualification](docs/RELEASE_0.44.0.fr.md).

## [0.43.0] - 2026-10-22

Enregistrez ou retirez explicitement une échelle de page dans le PDF avec Annuler/Rétablir, consultez les mesures de la page et réutilisez l’échelle physique d’une mesure non angulaire. Calibration de session et valeurs des annotations existantes restent distinctes. La CLI lit, écrit et retire les échelles dans de nouvelles copies et convertit les unités lors de leur utilisation.

[Notes, validation et limites](docs/RELEASE_0.43.0.fr.md).

## [0.42.0] - 2026-10-22

Mesures dispose d’un bouton libellé et d’un espace dans Tous les outils. Calibrez avec deux clics et une distance confirmée, puis tracez longueurs, polylignes, périmètres, surfaces et angles sur le PDF. Aperçu et Appliquer contrôlent chaque insertion ; retrait de points, annulation et alignement horizontal/vertical accompagnent le tracé. Chaque page conserve son échelle de session entre modifications d’annotations et changements habituels d’onglet.

[Notes, validation et limites](docs/RELEASE_0.42.0.fr.md).

## [0.41.0] - 2026-10-21

[0.41.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) sous `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

Repères de comptage persistants placés par clic sur les plans PDF locaux, regroupement, déplacement/suppression avec Annuler, changement de groupe sans changer l’identité et export des totaux reliés aux repères. Le tableau de toutes les annotations ajoute recherche, filtres nommés, tri, navigation, statuts métier, disciplines et champs texte personnalisés, avec exports CSV/JSON protégés. Les changements refusent les cibles périmées et préservent originaux et permissions. Pillow 13.0.0 remplace la dépendance vulnérable réservée aux tests. Les paquets 0.40.0 rejoignent les migrations. Les écritures communes vérifient le droit PDF d’annotation pour déplacements, suppressions, réponses et autres modifications ; l’export refuse aussi les inventaires de pages incomplets et les identités de comptage malformées.

PR #16 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.41.0.fr.md).

## [0.40.0] - 2026-10-21

[0.40.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) sous `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

Annotations locales de longueur, polyligne, périmètre, surface polygonale simple et angle : calibration explicite sur deux points, unités métriques/impériales décimales, précision et données JSON. Chaque mesure conserve son échelle et sa géométrie ; aperçu, Annuler et copies CLI préservent les originaux. Tampons Stamp avec image intégrée, proportions et transparence. Filigranes texte/numérotation : vérification de toutes les pages et refus atomique des marges invalides ou débordements. Paquets 0.39.0 ajoutés aux migrations. Backlog AEC de 26 cartes donnant priorité au local.

PR #14 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.40.0.fr.md).

## [0.39.0] - 2026-10-20

[0.39.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) sous `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

Modèles locaux nommés pour filigranes texte/image, numéros et six zones d’en-tête/pied. Charger un brouillon, enregistrer sous un nom unique ou supprimer explicitement. Écriture atomique, préférences préservées en cas d’échec et modèles invalides isolés. Portée des pages explicite et images référencées par chemin. Paquets 0.38.0 ajoutés aux migrations. Vérification de version Office sans interface avec une limite annulable de 30 secondes au démarrage à froid ; limites de conversion et gardes de sécurité conservées.

PR #13 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.39.0.fr.md).

## [0.38.0] - 2026-10-20

[0.38.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) sous `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six zones d’en-tête/pied via Numéros de page et la commande headers-footers. Numéro absolu, total, marges, taille et pages choisies respectent rotation et recadrage. Dépassement refusé atomiquement ; chiffrement autorisé et nouvelle copie en CLI conservés. Paquets 0.37.0 ajoutés aux migrations.

PR #12 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.38.0.fr.md).

## [0.37.0] - 2026-10-20

[0.37.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) sous `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Filigranes image dans Pages et la CLI : transparence, largeur proportionnelle, angle, sept positions et marges. Ajustement aux pages tournées/recadrées. Modification atomique, une annulation dans l’application et nouvelle copie en CLI ; chiffrement autorisé conservé. Paquets 0.36.0 ajoutés aux migrations.

PR #11 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.37.0.fr.md).

## [0.36.0] - 2026-10-20

[0.36.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) sous `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Raccourcis clavier personnalisables dans Aide et la palette : filtre, attribution, refus des conflits, désactivation et restauration. Brouillon annulable, préférences atomiques et libellés cohérents. Saisie et touches réservées préservées ; paquets 0.35.0 ajoutés aux références de migration.

PR #10 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.36.0.fr.md).

## [0.35.0] - 2026-10-20

[0.35.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) sous `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Signatures visuelles : lissage facultatif du dessin, réduction des ombres du papier et commande PNG `prepare-signature-image`. Aperçu lié aux pixels préparés, conservation des points bruts et fichiers sources, limites explicites du dessin. Les paquets publiés de la 0.34.0 rejoignent les références de migration.

PR #9 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.35.0.fr.md).

## [0.34.0] - 2026-10-20

[0.34.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) sous `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspection du PDF enregistré depuis Propriétés et la CLI JSON `inspect` : sept catégories de déclarations, SHA256, résultats partiels explicites, entrées chiffrées et refus des réponses périmées. La garde de version sauvegardée calcule les empreintes par blocs.

PR #8 fusionnée après CI/CodeQL sur la révision exacte.

[Notes, validation et limites](docs/RELEASE_0.34.0.fr.md).

## [0.33.0] - 2026-10-20

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) sous `lmdanavanem`, release `417802490`, source immuable `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Préservation des signets PDF 2.0 importés contenant des tabulations/sauts de ligne, garde stricte des nouveaux titres, migration groupée egui/eframe 0.37 et correction de la position Précédent/Suivant. Sept exemples PDF Association sous licence et empreintes fixes, 64 mutations PDF/CMS bornées, contrôles négatifs et mesures par commande. La 0.32.0 devient une référence de migration.

PR #4, #5, #7 fusionnées ; #6 intégrée via #7 et clôturée.

[Notes, validation et limites](docs/RELEASE_0.33.0.fr.md).

## [0.32.0] - 2026-10-19

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) sous `lmdanavanem`, release `417623635`, source immuable `8c492db35f1712648d5472593146077230befb7c`.

Réorganisation d’un objet ou de la sélection par glisser-déposer dans le panneau Objets, trait d’insertion, contrôle de la page du cache et du contexte, Échap et une seule étape Annuler. Commande JSON `reorder-objects` avec destination finale explicite, ordre relatif, permissions, sauvegarde et mots de passe existants.

[Notes, validation et limites](docs/RELEASE_0.32.0.fr.md).

## [0.31.0] - 2026-10-19

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) sous `lmdanavanem`, release `417463135`, source immuable `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Comparaison locale de deux PDF enregistrés (texte, dimensions et aperçus), alignement physique explicite et refus des aperçus obsolètes. Recherche littérale dans plusieurs fichiers sélectionnés, extraits et navigation, progression, annulation et résultats partiels explicites. Commandes JSON `compare` et `search-files` ; interface EN/FR.

[Notes, validation et limites](docs/RELEASE_0.31.0.fr.md).

## [0.30.0] - 2026-10-17

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) sous `lmdanavanem`, release `415568680`, source immuable `4db31c0ee62cfae9d65239edf1eac93db147841e`. Les cinq Bugs et quatorze P2 Roadmap sont livrés dans leurs [limites documentées](docs/RELEASE_0.30.0.fr.md). P1 Roadmap et Suggestions restent différés.

[Workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) et [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) réussis sur cette source : **551 tests**, zéro échec, 9 ignorés par défaut ; **1047 clés EN/FR**, **84 contrôles qpdf** plus **9 sorties de fonctions indépendantes**, **4 cas de polices MuPDF**, DX12 logiciel WARP, **14 allers-retours Office**, 4 PDF licenciés/744 pages, **21 scénarios portables**, **15 bases MSI/75 journaux**. CodeQL Rust, Python et Actions réussis ; zéro alerte ouverte.

Six fichiers publics vérifiés octet par octet avec les paquets préparés et leurs SHA256. Le metteur à jour de production a vérifié le ZIP x64 de 26,978,019 octets en 2.55 s, sans installation sur cet hôte, au moyen du banc de test déjà compilé dont le module de mise à jour est inchangé. Projet relu : **42 éléments uniques, 20 Todo, 0 In Progress, 22 Done**, trois filtres enregistrés et aucune modification non enregistrée. [Preuves et limites](docs/PROJECT_STATUS.fr.md).

Les paquets restent non signés. ARM64 est compilé et empaqueté, sans exécution sur matériel ARM64. Imprimantes physiques, Narrateur, interactions manuelles de mise à jour et GPU/fenêtres physiques restent à qualifier ; WARP prouve uniquement le rendu logiciel hors écran. Les entrées non prises en charge restent refusées. La conservation PDF/A ne certifie pas PDF/A/PDF/UA.

Le lot autorisé couvre cinq Bugs et les quatorze P2 Roadmap. Le code est validé et publié. Le [périmètre exact et ses limites](docs/RELEASE_0.30.0.fr.md)
comprend l’aperçu des paragraphes, le collage modifiable, les annotations enrichies, les liens précis,
les étiquettes sémantiques, les signets et structures, l’aperçu du caviardage et ses marques persistantes,
l’assainissement, les sous-ensembles de polices, les conversions Office locales, les états de confiance
des signatures, leur stockage protégé et l’inventaire des licences. Les cinq correctifs concernent les
polices Windows, le pivot de rotation, l’opacité, les annotations redimensionnées et le GPU facultatif.
P1 Roadmap et Suggestions restent hors de ce lot. Résultats dans [l’état du projet](docs/PROJECT_STATUS.fr.md) ;
les sections des anciennes versions ci-dessous sont historiques.

## [0.21.1] — 2026-10-17

Publiée en [Latest stable à cette date](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) sous `lmdanavanem`,
avec MSI x64, ZIP portables x64/ARM64 et SHA256. Tests de la source de release, preuves de mise à niveau,
téléchargements publics et mise à jour en ligne réussis ; [preuves et limites](docs/PROJECT_STATUS.fr.md).

### Corrections

- Refuser les modifications avec perte quand la police d'origine d'un objet texte ne peut pas représenter
  le remplacement ; conserver le PDF d'origine et le brouillon courant pour réessayer.
- Conserver les annotations non-widget sans apparence normale utilisable lors de l'aplatissement des
  formulaires, avec leurs fichiers incorporés et références partagées ; un échec laisse le document inchangé.

## [0.21.0] — 2026-10-16

- Renforcer l'accès natif aux métadonnées du certificat signataire par une garde explicite contre le
  pointeur nul et ajouter une régression CMS malformé ; propriété native et erreurs conservées (D112).
- Déclarer `contents: read` à la racine du workflow de release ; conserver `contents: write` sur Publish.
- Garder fixtures de sécurité déterministes et couverture du scanning, avec preuves individuelles lors du
  triage des valeurs cryptographiques de test ; le mot de passe PDF vide reste un essai d'authentification.
- Conserver les mises à jour Dependabot `getrandom` 0.4.3, `xml-rs` 1.0.0 et celles des GitHub Actions existantes.
  Migrer les deux lecteurs XML vers `ParserConfig`, refuser les DTD avant le traitement des entités et garder
  les bornes ; nouvelles régressions XMP/XFDF UTF-8/UTF-16 et notices de licence XML exactes.
- Vérifier le triage individuel des 115 critiques par `lmdanavanem` : 114 `used in tests`, #13 `false positive`.
  Aucune critique ouverte ; élevée #3 et moyenne réindexée #119 corrigées par CodeQL. Ancienne #1 historique.
- Contrôles locaux réussis : 471 tests, zéro échec, sept ignorés par défaut ; formatage, Clippy sans
  avertissement, 912 clés EN/FR, 75 contrôles qpdf, autotest du metteur à jour et corpus quatre documents/744 pages.
  Trois nouvelles régressions : CMS malformé et deux refus d'entités XML UTF-8/UTF-16.
- Nouveau [CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) : élevée #3 et moyenne #119 corrigées ; zéro alerte ouverte, aucune exclusion du scanning.
- 0.21.0 publiée (Latest à cette date) sous `lmdanavanem` ; CI Windows, ARM64 de même source, 19 scénarios portables,
  13 bases MSI/65 journaux, six fichiers préparés/publics et mise à jour de production vérifiés.
- Remplir les vues Bugs, Roadmap et Suggestions du projet GitHub avec 42 brouillons priorisés et filtres de catégorie enregistrés ;
  l'élément de sécurité 0.21.0 terminé est Terminé.
  Résultats réels et états des alertes dans `docs/PROJECT_STATUS.fr.md` et `docs/CODE_SCANNING_REVIEW.fr.md`.

## [0.20.0] — 2026-10-16

- Ajout : créer et modifier les annotations FreeText dans un éditeur de page multiligne ancré, avec rectangles
  cliqués/tracés et réglages taille/couleur/fond/bordure existants. Ajouter/Enregistrer/Annuler explicites réutilisent
  AddFreeText/UpdateFreeText, validation, protection et une étape Annuler (D111/A39).
- Protéger la page/le document/l'onglet/l'outil/l'espace de travail, les opérations en attente et les brouillons concurrents.
  Zoom disponible ; navigation incompatible bloquée jusqu'à validation/annulation ; texte conservé après erreur de validation/worker.
- Apparences Helvetica WinAnsi, agrandissement au renvoi, API moteur/CLI et 53 commandes inchangés ; aucune dépendance ajoutée.
- Correction des tracés rapides de rectangle qui perdaient la position finale au relâchement ; les destinations
  de recherche incompatibles ne font plus défiler un brouillon FreeText actif.
- Workspace local : 468 tests réussis, aucun échec et sept ignorés par défaut ; formatage, 912 clés EN/FR,
  75 contrôles qpdf, corpus licencié et autotest du metteur à jour réussis ; Clippy sans avertissement réussi.
- Publiée en Latest stable sous `lmdanavanem` ; CI Windows, ARM64 de même source, 18 scénarios portables,
  12 bases MSI/60 journaux, six fichiers préparés/publics et metteur à jour de production vérifiés.
  Résultats et limites dans `docs/PROJECT_STATUS.fr.md`.

## [0.19.0] — 2026-10-16

- Ajouté : tracer sur la page un rectangle pour les cinq types de champ existants, puis le relire dans
  la boîte numérique existante avant **Créer le champ** explicite. Conserver nom, choix, options modifiable/sélection multiple,
  validation moteur, chiffrement et une étape Annuler ; le tracé seul ne doit pas modifier le PDF (D110).
- Gardes ajoutées : géométrie finie dans la page et minimum de quatre points, annulation si document/onglet/outil périmé,
  réticule/aperçu et aucune interaction avec les widgets existants pendant le tracé.
- Aucun nouveau type, dépendance ou commande CLI. Implémentation, tests locaux, formatage et Clippy réussis ;
  contrôles CI/paquets/mise à jour/publication réussis ;
  **0.19.0 était la Latest stable à sa publication**. Preuves exécutées dans `docs/PROJECT_STATUS.fr.md`.

## [0.18.0] — 2026-10-16

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

### Changements

Regrouper les traits à main levée d'une page en une annotation et une étape Annuler via **Terminer le dessin** /
**Annuler le dessin**. Une erreur du worker conserve le brouillon pour réessayer ; les actions incompatibles
enregistrement/fermeture/document/outil attendent Terminer/Annuler. Réutiliser AddInk, simplification des points,
géométrie affichée et protection (D109). Au plus 64 traits et 4 000 points par trait. Brouillon en mémoire
jusqu'à Terminer, absent de la récupération après plantage. Aucune validation à l'inactivité ni gomme.
L'aide de `stamp` / `page-numbers` conserve `{n}` / `{total}` littéraux via le modèle clap existant (A16).
Aucune nouvelle dépendance ni commande CLI ; 53 commandes.

Stylet/tactile natifs, boîtes interactives, imprimantes, Narrator, matériel ARM64 et lecteurs tiers restent non vérifiés. Paquets non signés ; brouillons d'encre non validés sans récupération après plantage.

## [0.17.0] — 2026-10-16

Validation locale réussie : 445 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy all-targets
sans avertissement, 896 clés EN/FR, 74 contrôles qpdf et quatre documents licenciés (744 pages ; 48,187 s,
pic mémoire enfant 330 264 576 octets). Autotest du helper de mise à jour réussi. Contrôle indépendant de la CLI
debug réussi : huit orientations, neuf flux JPEG exacts dont aller-retour au DPI maximal, 32 coins JPEG/quatre
PNG et masque alpha, trois contrôles qpdf, deux refus de budget avant décodage, dix refus de DPI invalides,
treize sources inchangées (35 sous-processus, 1,063 s).
Source : `5785de282e126fb4666b28fb83340d0e9c6e9acf`. [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
réussis. Quinze scénarios portables, neuf bases MSI et 45 journaux MSI vérifiés. Corpus en CI : 45,791 s,
pic mémoire enfant 321 413 120 octets.
[Publiée (Latest à cette date)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) sous `lmdanavanem`,
release `415074108`, avec six fichiers. Fichiers préparés/publics et contrôles renforcés des paquets réussis,
dont contrôles indépendants JPEG/transparence/budget/DPI ci-dessus. Copies vérifiées dans `dist/` ; le metteur
à jour de production a téléchargé et vérifié le ZIP Latest x64 de 19 825 743 octets sans installation sur l'hôte.
Preuves et limites restantes dans `docs/PROJECT_STATUS.fr.md`.

### Modifications
- Images → PDF incorpore directement les JPEG d'orientations EXIF 5 et 7 : les huit orientations sont ainsi
  prises en charge sans décodage d'image ni recompression. La matrice applique l'orientation ; octets JPEG conservés.
- Avant décodage raster, contrôler une estimation cumulée de 512 Mio au plus pour les non-JPEG : somme de
  `max(decoder.total_bytes(), largeur × hauteur × 4)`, avec arithmétique vérifiée. Répéter les contrôles au chargement
  pour détecter les changements simples des fichiers. Refuser une géométrie physique non finie/non positive.
- Dépassement/overflow du budget : `image_batch_too_large`, avec explication localisée dans l'application ;
  géométrie physique invalide : `other` conservé. Une clé d'erreur EN/FR ajoutée ; autres protections conservées.

Budget conservateur de données raster, pas plafond RSS natif/processus ni limite des JPEG comprimés.
Autres formats toujours décodés avec qualité/transparence conservées. Accès UI/CLI, signature de méthode et 53 commandes inchangés ;
aucune nouvelle dépendance. Protections des sources/sorties conservées (D108).

## [0.16.0] — 2026-10-16

Validation locale réussie : 439 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy,
895 clés EN/FR, 73 contrôles qpdf et quatre documents licenciés (744 pages ; 53,542 s, pic mémoire enfant
331 517 952 octets). Source : `6cd7843734f3924e87033d8b05539d493ccbcb95`.
[CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537) et
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589) réussis,
ainsi que les quatorze scénarios portables (dix mises à niveau dont 0.15.0 et quatre protections), huit bases MSI
et 40 journaux MSI vérifiés. Corpus en CI : 47,628 s, pic mémoire enfant 321 306 624 octets.
[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) sous `lmdanavanem` (Latest à cette date),
release `414934190`, avec six fichiers. Six fichiers préparés/publics et contrôles renforcés des paquets réussis ;
copies vérifiées dans `dist/`. Le metteur à jour de production a téléchargé et vérifié le ZIP Latest x64 de
19 821 719 octets sans installation sur l'hôte. Preuves exécutées et limites dans `docs/PROJECT_STATUS.fr.md`.

### Ajouts
- **Bordure en tirets** pour les annotations rectangle, ellipse, ligne et flèche ; CLI `annotate --dashed`
  utilise le même motif fixe : tiret de 6 points, espace de 3 points. La CLI conserve 53 commandes. Trait continu
  par défaut, remplissages continus et pointes de flèche pleines.
- Relire le vecteur réel `dash_pattern` et conserver les motifs personnalisés lors de sélection, recoloration,
  déplacement/redimensionnement et Annuler/Rétablir. Seul un changement explicite de la case remplace le motif ;
  sélectionner un motif personnalisé ne le normalise pas. Aucun éditeur de motif personnalisé ajouté.

### Protections
- Dictionnaires de bordure et apparences explicites cohérents, phase zéro. Motifs non vides refusés pour encre,
  texte libre, liens et marquages. Style de bordure malformé/non pris en charge sur une annotation SuniPDF : édition
  régénérant l'apparence refusée, sans remplacer silencieusement le style d'origine.
- Tableau de tirets explicitement vide : continu valide ; tableau absent : valeur PDF `[3]`. Bordure SuniPDF
  malformée : Déplacer/Redimensionner/Style désactivés. Commentaire de forme modifiable sans reconstruire l'apparence ;
  commentaire de texte libre malformé refusé car il la régénère. Aperçu non vide invalide/excessif : contour omis,
  jamais remplacé par un trait continu ; motifs valides plus petits qu'un pixel conservés.
- Motifs bornés à 16 valeurs finies non négatives, 1000 points au plus chacune ; valeur positive d'au moins
  0,1 point et somme positive. Une borne conservatrice de 100 000 segments d'apparence refuse un travail excessif.

Aucune nouvelle dépendance. Contrôles natifs interactifs et lecteurs tiers distincts ; paquets non signés.

## [0.15.0] — 2026-10-16

Validation locale réussie : 429 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy,
894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés (744 pages ; 48,891 s, pic mémoire enfant
328 736 768 octets). Source : `952f8699cac451e7c3004b6d816105d52e41f249`.
[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous `lmdanavanem` (Latest à cette date), release
`414866395`, avec six fichiers. [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488) et
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541) réussis,
ainsi que treize scénarios portables et sept bases MSI. Six fichiers préparés/publics et contrôles renforcés réussis,
dont cibles non chiffrées/protégées, les deux mots de passe, identité/zone des liens et PNG des pages inchangés.
Copies vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP Latest x64 de 19 803 226 octets
sans installation. Preuves exécutées et limites restantes dans `docs/PROJECT_STATUS.fr.md`.

### Ajouts
- Modifier un lien existant via **Modifier la destination** ou double-clic avec la sélection d'annotations.
  La boîte réutilisée accepte `http`, `https`, `mailto` ou une page physique. Les destinations nommées/XYZ résolues
  préremplissent leur page ; le remplacement affiche explicitement la page entière (Fit), sans conserver
  position/zoom détaillés. Une cible non prise en charge donne une adresse vide, jamais silencieusement la page 1.
- CLI `set-link-target --page 1 --index 0 --goto-page N` ou `--url ADRESSE`, avec une seule destination,
  `--out` et `--password` facultatif, réutilise la protection de sortie. Pages physiques à partir de 1 ; indices
  d'annotations à partir de 0, comme retournés par `annotations`.

### Corrections
- Remplacer ensemble les champs d'action `A`, `Dest`, `AA`, en conservant rectangle/style, noms, commentaires,
  réponses, autres annotations/formulaires/contenu et protection source. Drapeaux ReadOnly, Locked, LockedContents
  ou malformés : édition refusée, même indirects. Un Annuler restaure le changement sans perdre géométrie/navigation ;
  éditions et changements d'onglet abandonnent les boîtes périmées.

Aucune nouvelle dépendance. Confirmation des adresses externes et avertissement de réécriture des documents
signés conservés. Contrôles natifs interactifs, matériels et lecteurs tiers distincts ; paquets non signés.

## [0.14.0] — 2026-10-16

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) sous `lmdanavanem` (Latest à cette date), release
`414812776`, avec six fichiers vérifiés dans `dist/`. Source : `af532888804c196aabe216b8f6a2980c3d1e9e2d`.
Validation réussie : 420 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy, 890 clés EN/FR,
63 contrôles qpdf et quatre documents NIST/NBS licenciés (744 pages ; 53,91 s localement). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
réussis, ainsi que douze scénarios portables et six bases MSI. Six fichiers préparés/publics et contrôles renforcés
des binaires réussis, dont création/réinitialisation non chiffrée/protégée, les deux mots de passe, étiquettes hors
plage, Unicode, texte/PNG des pages inchangés, préservation source et qpdf. L'updater de production a vérifié le
ZIP Latest x64 de 19 785 200 octets sans installation. Preuves et limites : `docs/PROJECT_STATUS.fr.md`.

### Ajouts
- **Étiquettes de page…** dans Pages/Organiser : styles décimal, romain majuscule/minuscule, lettres
  majuscules/minuscules ou préfixe seul sur une plage physique inclusive, avec préfixe et départ positif.
  Numérotation hors plage conservée ; seules les métadonnées de navigation changent, pas les numéros visibles
  ou imprimés. Réinitialiser tout retire l'entrée du catalogue, même malformée ; ce n'est pas un effacement
  sécurisé des anciens objets de métadonnées.
- CLI `set-page-labels --from 1 --to N --style decimal --start 1 --prefix ""` et `reset-page-labels`, avec `--out`
  et `--password` facultatif, réutilisent les protections contre l'écrasement. Les indices CLI restent physiques.

### Corrections
- Annuler/Rétablir conserve l'information de géométrie inchangée, donc la navigation pour les éditions
  d'étiquettes/contenu. Les opérations structurelles continuent à l'invalider.

Nouveaux préfixes sur une ligne de 256 octets UTF-8 au plus, étiquettes générées bornées à 4096 octets ; plage
invalide, dépassement numérique ou arbre existant malformé : Appliquer refusé avant mutation. Les plages restent
positionnelles après insertion/suppression/réorganisation ; aucun recalage sémantique. Les doublons demandent
toujours `#N` physique. Validation et limites matérielles/lecteurs dans l'état du projet.

## [0.13.0] — 2026-10-16

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem` (Latest à cette date), release
`414763678`, avec six fichiers vérifiés dans `dist/`. Source : `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`.
Validation réussie : 410 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy, 870 clés EN/FR,
53 contrôles qpdf et quatre documents NIST/NBS licenciés (744 pages ; 76,40 s localement). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
réussis, ainsi que onze scénarios portables et cinq bases MSI. Six fichiers préparés/publics et contrôles renforcés
des binaires réussis, dont lots de signatures non chiffrés/protégés, les deux mots de passe, qpdf et préservation de
la source. L'updater de production a vérifié le ZIP Latest x64 de 19 730 578 octets sans installation.
Preuves et limites : `docs/PROJECT_STATUS.fr.md`.

### Ajouts
- **Saisir** une signature visuelle ou un paraphe, choisir une police standard/TrueType système et une encre
  noire/bleue, puis afficher explicitement l'**Aperçu** local avant la pose. Times italique par défaut ; glyphes
  absents et incorporation de police interdite refusés. Une ligne de 256 caractères au plus, aperçu PNG transparent
  limité à 1600 pixels de côté. Texte/police/encre/mode/boîte modifiés : anciens aperçus et réponses invalidés.
- Poser une marque sur la page courante, toutes les pages ou la sélection Organiser capturée à l'ouverture.
  Un clic fournit le même centre en points PDF affichés sur chaque cible ; proportions et taille physique
  conservées, réduction seulement pour tenir dans une petite page. Progression, annulation entre cibles,
  restauration atomique et un undo pour le lot ; enregistrement toujours explicite.
- CLI `add-image --pages "all|1,3-5"` utilise la pose atomique ajustée. Comportement `--page` et JSON d'une seule
  page compatibles ; seul le mode lot ajuste la position/taille aux pages.

### Corrections
- Garder les marques trop grandes et leur fantôme dans les petites pages avec la même géométrie ajustée.
- Abandonner aperçus asynchrones et cibles de signature à l'annulation, au changement de document ou de structure des pages.

Les signatures visuelles restent des images modifiables sans preuve d'identité/intégrité ; CMS/PFX est une
fonction existante distincte. Les signatures enregistrées restent des PNG locaux en clair. Matériel, dialogues
natifs et lecteurs tiers demandent des vérifications séparées ; les paquets distribués ne sont pas signés.

## [0.12.0] — 2026-10-16

[Publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) sous `lmdanavanem` (Latest à cette date), release
`414674886`, avec six fichiers vérifiés dans `dist/`. Source : `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`.
Validation réussie : 398 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy, 859 clés EN/FR,
48 contrôles qpdf, quatre documents NIST/NBS (744 pages ; 59,68 s), CI Windows et compilation ARM64 de même source.
Dix scénarios portables, quatre bases MSI, paquets préparés/publics, contrôles renforcés des binaires et téléchargement
vérifié par l'updater de production sans installation réussis. Preuves et limites : `docs/PROJECT_STATUS.fr.md`.

### Ajouts
- Créer une liste déroulante modifiable dans Préparer un formulaire en cochant **Autoriser du texte libre en plus
  des options proposées**, ou avec `add-form-field --kind combo --editable`. Des choix prédéfinis non vides restent requis.

### Corrections
- Utiliser les valeurs d'export `/Opt` des radios pour remplir, effacer et échanger JSON/XFDF/FDF, séparément des
  états d'apparence. Les informations CLI ajoutent `widgets[].export_value` ; `on_state` reste inchangé.
- Garder le champ radio concerné en lecture seule si sa correspondance d'export est malformée ou ambiguë, notamment
  en présence de valeurs dupliquées. Sans `/Opt`, les radios conservent leurs valeurs fondées sur les noms d'état.

## [0.11.0] — 2026-10-16

Validation et preuves de publication figurent dans `docs/PROJECT_STATUS.fr.md`.
Publiée sous `lmdanavanem` (Latest à cette date), avec six fichiers publics vérifiés dans `dist/`. Validation réussie : 390 tests
workspace, formatage/Clippy, 42 PDF distincts qpdf (44 contrôles), quatre documents réels (744 pages), CI Windows finale
et ARM64 de même source. Neuf scénarios portables, trois bases MSI, binaires publics renforcés et updater de production
réussis. Source immuable de la release : `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`.

### Ajouts
- Libellés PDF dans la barre de navigation, les miniatures, le statut et `info` CLI. Accès par libellé exact unique
  ou `#N` pour la position physique ; un libellé ambigu ne change pas la page.
- Liens internes et signets vers les positions/zooms XYZ, Fit et FitH non nul pris en charge, y compris les noms
  résolus par PDFium. Retour/avance conserve positions sur une même page et zoom, même lors de sauts rapides.
- Texte libre dans les listes déroulantes déjà éditables, depuis le panneau ou directement sur la page.
  Les options restent disponibles ; les valeurs saisies utilisent Appliquer/Abandonner et l’annulation existants.

### Changements
- Extraits et parties découpées conservent mots de passe, clé de chiffrement et permissions de la source.
  La CLI garde `encryption_removed` à `false` et ajoute `encrypted`. Une source non protégée reste non protégée.
- Extraction et découpage refusent de remplacer la source, même avec écrasement explicite. Sans écrasement,
  le découpage refuse aussi un fichier créé entre le contrôle initial et la publication atomique.
- Les tests Windows réels incluent désormais les mises à jour portables et MSI depuis 0.10.0.

### Corrections
- Révision officielle de lopdf fixée pour le calcul de clé avec mot de passe propriétaire ancien et son encodage.
  Copies protégées et métadonnées restent lisibles après ouverture propriétaire ; les deux rôles sont testés.
- L’outil de sélection de texte suit les clics sur les liens, avec confirmation et contrôle des adresses externes conservés.
- Une lecture de formulaire échouée conserve textes et choix, bloque les changements fondés sur un modèle périmé
  et propose un réessai explicite.
- Rotations mises en cache avec les libellés, mise à jour ciblée lors d’une rotation de page et une seule relecture
  des métadonnées par suppression groupée. `info` CLI évite sa précédente double boucle de chargement des pages.

### Limites
- Création de libellés et recalage sémantique après réorganisation non implémentés. Les libellés illisibles, contenant
  des contrôles ou dépassant 4096 octets utilisent les numéros physiques. Les modes de destination non pris en charge
  et FitH zéro/null ambigus reviennent à la navigation par page. La lecture des libellés ajoute du travail à l’ouverture
  des gros documents. Le texte libre nécessite une liste déjà marquée éditable ; créer/modifier ce drapeau reste à faire.
  Un découpage échoué peut laisser des parties déjà terminées ; aucun fichier partiel ni source modifiée n’est accepté.

## [0.10.0] — 2026-10-16

Validation et publication sont suivies dans `docs/PROJECT_STATUS.fr.md` ; le code présent ne prouve pas une release validée.
Validation locale réussie : 362 tests workspace, Clippy/formatage, 31 PDF qpdf et quatre publications authentiques
(744 pages). CI main finale et ARM64 de même source réussis ; six fichiers préparés, huit scénarios portables réels et
deux bases MSI réelles vérifiés. Publiée en Latest sous `lmdanavanem`, avec six fichiers publics vérifiés dans `dist/` ;
contrôles renforcés des binaires publics et updater de production réussis. Preuves et source immuable figurent dans l'état du projet.

### Ajouts
- Compression réelle des images vers une nouvelle copie : dimension maximale, qualité JPEG, taille avant/après,
  progression et annulation. Images RGB/gris 8 bits bornées ; masques, profils et décodages particuliers conservés.
  Sans gain, copie exacte des octets source. PDF signés ou interdisant la modification refusés.
- Lots séquentiels de compression et d’OCR local FR/EN : 128 PDF maximum, 256 Mio par entrée, résultats individuels,
  annulation et copies atomiques sans écrasement. Les originaux restent intacts.
- Rotation de vue par quarts de tour, lecture plein écran et historique de navigation de 256 positions par onglet.
  L’édition revient à une vue droite ; la rotation de lecture ne modifie jamais les pages du PDF.
- Création de groupes radio, listes déroulantes et zones de liste à choix simple/multiple. Export XFDF et import
  JSON/XFDF/FDF des valeurs, validé intégralement avant une seule opération annulable.
- Boutons Appliquer/Abandonner dans le panneau des champs, activation clavier Espace/Entrée et focus des listes.
- Recadrage souris dans l’aperçu et rotation/redimensionnement groupés des objets, une annulation par opération.
- CLI `compress-images`, `form-export`, `rotate-objects` et `resize-objects` ; options de choix dans `add-form-field`
  et entrées `.json`, `.xfdf`, `.fdf` prises en charge dans `fill-form`.

### Fiabilité
- Sauvegardes par fichier temporaire voisin, synchronisation du contenu puis remplacement atomique. Régressions
  d’interruption, disque plein, permission/publication refusée et arrêt forcé d’un processus écrivant une copie partielle.
- Quatre documents NIST publics, licenciés et figés par empreinte complètent le corpus qpdf synthétique.
  Ils ne représentent pas tous les PDF clients, hostiles ou volumineux.

### Corrigé
- Les réécritures PDF retirent les anciens flux d'objets/xref et indices de linéarisation, recalculent la borne des
  numéros d'objets émis et suppriment les offsets de révision périmés. Les `/Size`/xref invalides de fichiers réels
  sont corrigés en conservant chiffrement et permissions. Une compression sans gain garde les octets et diagnostics source figés.

### Limites
- Compression avec perte, gain non garanti. L’interface de lots refuse les PDF demandant un mot de passe.
- FDF limité aux tables xref ordinaires et valeurs texte/nom/tableau ; flux, actions, pièces jointes, texte enrichi et
  export FDF non pris en charge. Les libellés radio figurent dans le panneau, pas comme texte ajouté dans la page.
- Matériel ARM64, imprimantes physiques, Narrateur, parcours interactifs et lecteurs tiers restent des contrôles distincts.

## [0.9.1] — 2026-10-16

### Corrigé
- Icône au ruban rouge plus grande, avec beaucoup moins de marge transparente, commune à Windows, au MSI et à l'interface.
- La modification des métadonnées conserve l'identification PDF/A et les propriétés XMP indépendantes. Le XMP malformé est refusé au lieu d'être supprimé silencieusement ; cela ne certifie pas la conformité PDF/A.
- Les dictionnaires de chiffrement AES-256 indiquent explicitement la longueur de clé de 256 bits pour les lecteurs indépendants.
- Le résultat de mise à jour est enregistré après la relance de l'application, avec le chemin de sauvegarde conservé en cas d'échec d'installation ou de relance.
- La vérification SHA256 de l'updater utilise directement .NET, y compris quand les chemins de modules proviennent de PowerShell 7.

### Modifié
- Contrôles QPDF figés dans la CI Windows : corpus synthétique, modifications, chiffrement AES, formulaires, caviardage, OCR, signatures numériques et entrées malformées. Les avertissements structurels font échouer le contrôle.
- Régressions de mise à jour portable réelle et d'installation/upgrade/rollback MSI sur Windows jetable, en complément des tests existants. Résultats exécutés et limites matérielles dans `docs/PROJECT_STATUS.fr.md`.

## [0.9.0] — 2026-10-16

Résultats de validation et contrôles matériels restants dans `docs/PROJECT_STATUS.fr.md`.

### Ajouté
- Instantanés de récupération automatiques toutes les 60 secondes et à la suspension d'un onglet modifié.
  Le PDF d'origine reste intact ; son chiffrement est conservé sans enregistrer de mot de passe,
  et la restauration se termine par un enregistrement dans une nouvelle copie.
- Impression Windows par la boîte native : modifications courantes, plages de pages, copies, rendu
  tramé, progression et annulation. La validation sur imprimante physique reste à faire.
- Sélection de lecture entre plusieurs pages dans les deux sens, double-clic sur un mot et défilement
  automatique au bord ; la copie suit l'ordre logique PDFium, y compris sur pages pivotées et recadrées.
- Vue double page et mode Livre avec couverture seule, tailles mixtes, zoom ancré et cache de tuiles borné.
- Préparer un formulaire : création de champs texte et de cases nommés par coordonnées numériques,
  avec annulation, enregistrement et remplissage.
- OCR anglais/français embarqué et hors ligne : Tesseract 5.5.3, Leptonica 1.87 et modèles rapides `eng`/`fra` ;
  texte invisible recherchable, pages déjà textuelles ignorées, progression, annulation et un seul undo.
- Utiliser un certificat : vérification des signatures du fichier enregistré et création d'une copie signée
  incrémentale avec une identité PFX/P12. Confiance de l'identité, révocation et horodatage fiable non établis.
- CLI `add-form-field`, `ocr`, `signatures` et `sign`. Le mot de passe du certificat vient d'une variable
  d'environnement ou de l'entrée standard, jamais d'un argument en clair. La signature ne remplace pas sa source.

### Modifié
- Les 14 outils disposent d'une implémentation. Un avertissement précède la réécriture d'un PDF signé ;
  la signature numérique refuse les PDF chiffrés. Conversion Office, rotation de vue et polices avancées restent prévues.

## [0.8.3] — 2026-10-16

### Corrigé
- L'ajout de texte refuse les coordonnées NaN et infinies avant de modifier le document.
- Organiser les pages borne les textures des miniatures et les rendus en attente sur les gros documents ;
  les résultats périmés sont écartés et les échecs de miniature ne bloquent pas la file.
- Les tampons et modèles de numérotation refusent explicitement les sauts de ligne.
- La CLI refuse les opacités non prises en charge pour le texte libre ; l'opacité d'une annotation doit être finie.
  Les commandes refusées laissent les fichiers de sortie existants intacts.

## [0.8.2] — 2026-10-16

### Ajouté
- Recherche de mises à jour au démarrage, notification de téléchargement et confirmation « Fermer et installer ».
  La recherche automatique peut être désactivée dans À propos ; la recherche manuelle reste disponible.
  Aucun PDF n'est envoyé à GitHub.
- Vérification de l'architecture, de la taille et du SHA256 du paquet officiel avant installation. Les versions
  x64 installées utilisent le MSI ; les portables x64/ARM64 mettent à jour leur dossier avec une sauvegarde de retour arrière.

### Modifié
- Anglais par défaut dans l'interface et langue principale du dépôt et de la documentation. Le français reste
  disponible et les préférences de langue existantes sont conservées.
- Notes de release et modèles GitHub en anglais ; description malformée de la 0.8.0 réparée.

### Corrigé
- Une mise à niveau MSI peut restaurer l'installation précédente si la nouvelle échoue.
- Annuler la fermeture d'un document ou échouer à l'enregistrer annule l'intention d'installer la mise à jour.

## [0.8.1] — 2026-10-15

### Corrigé
- Onglets : la croix reçoit le clic et ferme le document ; le titre permet toujours de changer d'onglet.
- La croix d'un onglet en arrière-plan ferme ce document après son activation, en demandant d'enregistrer
  ses modifications si nécessaire. Une activation refusée pendant un enregistrement ne ferme aucun document.

## [0.8.0] — 2026-10-15

Six corrections prioritaires, 26 tests de régression ajoutés (248 tests réussis, 2 mesures ignorées).

### Corrigé
- Protection : vérification des mots de passe après normalisation et application des limites AES-128 / AES-256 ;
  refus d'un mot de passe d'ouverture devenu vide et des mots de passe propriétaire/utilisateur équivalents.
- Formulaires : Ctrl+S prend en compte les brouillons du panneau ; une saisie invalide reste éditable et bloque
  l'enregistrement. Les saisies en cours sont prises en compte à la fermeture.
- Onglets : les réponses de sélection d'un ancien document ne peuvent plus viser les objets du nouvel onglet.
- Fermeture : les modifications en attente du moteur sont traitées avant de décider d'enregistrer ou de fermer.
- Déplacement des pages : conservation des champs de formulaire, signets, liens internes et attributs hérités ;
  le document reste chiffré quand il l'était déjà.
- CLI `form-fields` : les valeurs des champs mot de passe sont masquées aussi dans le listing complet.

## [0.7.0] — 2026-10-15

Caviardage irréversible (outil « Caviarder un PDF », CLI `redact`, 42 commandes, 222 tests).

### Ajouté
- **Caviardage irréversible** (outil « Caviarder un PDF » du hub) : tracer des zones (touche B), ou marquer toutes
  les occurrences d'un texte, retirer une zone d'un clic, puis **Appliquer** (Ctrl+Maj+B, confirmation). Le texte,
  les images, les tracés, les annotations et les champs situés sous les zones sont **supprimés du document**, pas
  masqués ; le reste de chaque élément touché est conservé en image. Le moteur relit le résultat et refuse
  (`redaction_incomplete`, rien n'est modifié) s'il reste du texte extractible sous une zone. Options : convertir
  les pages caviardées entièrement en image, supprimer les informations cachées (métadonnées, balisage, cochée par
  défaut). Le fichier est réécrit sans révision antérieure ; un document protégé reste protégé ; annulable par Ctrl+Z.
  Enregistrer avec des zones non appliquées demande confirmation. Limites : `docs/KNOWN_ISSUES.fr.md` A40.
- CLI : `sunipdf-cli redact fichier.pdf --out sortie.pdf [--rect PAGE:G,B,D,H]… [--text TEXTE]… [--match-case]
  [--rasterize] [--keep-hidden-data] [--dpi N]` (sortie JSON, erreur `redaction_incomplete` sans écriture).

## [0.6.0] — 2026-10-15

Annotations avancées, nouvelle identité visuelle et préparation de la publication en source ouverte.

### Ajouté (projet)
- Préparation de la publication en source ouverte : licences MIT / Apache-2.0, `CONTRIBUTING.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, modèles d'issues et de pull request, README bilingue, traductions anglaises de tous les
  documents de `docs/` et du journal des versions (`docs/README.md` sert d'index).
- Nouveau logo (variantes claire et sombre) : symbole dans la barre d'onglets, logo complet sur l'accueil et dans
  la boîte À propos, icône `.ico` régénérée par `scripts/make_icon.py`.
- Site du projet : https://navanem.com/projets/sunipdf/ (lien dans À propos, `Cargo.toml`, README).

### Ajouté
- **Annotations avancées** (outil Commenter) : dessin à main levée (trait lissé), zone de texte libre, rectangle,
  ellipse, ligne et flèche, tous en **vraies annotations PDF** (apparence explicite : les autres lecteurs les
  affichent) ; sélection d'une annotation existante pour la déplacer, la redimensionner, la recolorer, modifier son
  texte ou la supprimer (raccourcis A, D, F) ; barre flottante (couleur, épaisseur, fond, opacité, flèche, taille du
  texte).
- **Liens cliquables** : création (page ou adresse `http` / `https` / `mailto`), clic sur un lien interne, et pour une
  adresse externe une **boîte de confirmation qui montre l'adresse complète** avant d'ouvrir le navigateur ; les
  actions de lancement ou de script ne sont jamais exécutées.
- **Fils de réponses et statuts de relecture** dans le panneau Commentaires (réponses en retrait, statut Accepté /
  Refusé / Annulé / Terminé, auteur mémorisé, date, suppression du fil), écrits selon la norme PDF (affichage dans
  d'autres lecteurs PDF non vérifié).
- CLI : `annotations`, `annotate` (`ink`, `square`, `circle`, `line`, `arrow`, `freetext`, `link`), `reply`,
  `review-state`, `annotation-delete` (41 commandes).
- Fixture `annotated.pdf` (12 annotations de plusieurs producteurs) ; 40 tests de plus (207 au total).

### Modifié
- Les annotations s'écrivent par lopdf avec leur propre apparence : pages pivotées ou recadrées justes (texte libre
  droit à l'écran), document protégé par mot de passe conservé protégé.
- L'énumération des annotations ne s'arrête plus à la première entrée illisible (indices alignés sur `/Annots`).

## [0.5.0] — 2026-10-15

Vague 2 du backlog, chaque tranche implémentée, revue indépendamment, corrigée puis re-revue :
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

Vague 1 du backlog, livrée par tranches avec revue indépendante : hub « Tous les outils »,
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
- **Hub « Tous les outils »** : le panneau de gauche (F9) liste les outils par tâche avec une
  pastille colorée par outil (Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger,
  Compresser, Ajouter un tampon) ; les cinq outils sans fonction réelle dans SuniPDF ne sont pas
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
- **Panneau d'outils et accueil** : icônes colorées par catégorie (fichier, pages,
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
  modifié conserve sa police d'origine (caractères absents non garantis). Voir `docs/KNOWN_ISSUES.fr.md`.

## [0.1.0] — 2026-10-14

Première version : lecture (PDFium), onglets, miniatures, recherche, sélection/copie de texte,
annotations (surligner, souligner, barrer, notes, panneau Commentaires), organisation des pages
(rotation, suppression, insertion, déplacement, vue grille), fusion / division / extraction,
images ↔ PDF, export PNG, optimisation, signets, propriétés, protection AES-128/256 et retrait,
métadonnées, annuler/rétablir, palette de commandes, CLI JSON (20 commandes), interface bilingue
fr-FR / en-US, thèmes clair/sombre, ZIP portable et MSI x64.
