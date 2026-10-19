# Matrice fonctionnelle : attendu d'un éditeur PDF complet / SuniPDF

[English](FEATURE_MATRIX.md)

## 0.31.0 : comparaison locale et recherche multi-PDF (qualification)

Deux Suggestions P2 sont implémentées : recherche dans des fichiers enregistrés explicitement sélectionnés et comparaison texte/visuelle bornée, avec progression, annulation et préservation des sources. [Périmètre et limites](LOCAL_REVIEW.fr.md). Tests de release et publication encore en cours.

## 0.30.0 : édition, protection des documents et conversion locale (publiée en Latest stable)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) sous `lmdanavanem`, release `415568680`, source immuable `4db31c0ee62cfae9d65239edf1eac93db147841e`. Les cinq Bugs et quatorze P2 Roadmap sont livrés dans leurs [limites documentées](RELEASE_0.30.0.fr.md). P1 Roadmap et Suggestions restent différés.

[Workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) et [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) réussis sur cette source : **551 tests**, zéro échec, 9 ignorés par défaut ; **1047 clés EN/FR**, **84 contrôles qpdf** plus **9 sorties de fonctions indépendantes**, **4 cas de polices MuPDF**, DX12 logiciel WARP, **14 allers-retours Office**, 4 PDF licenciés/744 pages, **21 scénarios portables**, **15 bases MSI/75 journaux**. CodeQL Rust, Python et Actions réussis ; zéro alerte ouverte.

Six fichiers publics vérifiés octet par octet avec les paquets préparés et leurs SHA256. Le metteur à jour de production a vérifié le ZIP x64 de 26,978,019 octets en 2.55 s, sans installation sur cet hôte, au moyen du banc de test déjà compilé dont le module de mise à jour est inchangé. Projet relu : **42 éléments uniques, 20 Todo, 0 In Progress, 22 Done**, trois filtres enregistrés et aucune modification non enregistrée. [Preuves et limites](PROJECT_STATUS.fr.md).

Les paquets restent non signés. ARM64 est compilé et empaqueté, sans exécution sur matériel ARM64. Imprimantes physiques, Narrateur, interactions manuelles de mise à jour et GPU/fenêtres physiques restent à qualifier ; WARP prouve uniquement le rendu logiciel hors écran. Les entrées non prises en charge restent refusées. La conservation PDF/A ne certifie pas PDF/A/PDF/UA.

## 0.21.1 : conservation des documents (publiée ; Latest à cette date)

Les deux correctifs P1 sont livrés dans la [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) : refus du texte incompatible
sans perdre le bon brouillon, et conservation des annotations non-widget sans apparence utilisable à
l'aplatissement. 476 tests, 84 contrôles qpdf, 20 scénarios portables et 14 bases MSI réussis ; six fichiers
publics et mise à jour en ligne vérifiés. Voir l'[état du projet](PROJECT_STATUS.fr.md).

## 0.21.0 : traitement des alertes Code Scanning (publiée ; Latest à cette date)

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

**0.20.0 publiée ; Latest à cette date :** créer/modifier FreeText sur la page, avec éditeur multiligne ancré,
réglages de style existants et Ajouter/Enregistrer/Annuler explicites (D111/A39). Relecture, 468 tests workspace,
formatage et 912 clés EN/FR réussis en local ; Clippy, CI/mises à niveau, ARM64 de même source,
paquets préparés/publics et metteur à jour de production réussis. **0.20.0 était Latest stable à sa publication** ; preuves et limites dans `PROJECT_STATUS.fr.md`.

**0.19.0 publiée ; Latest à cette date :** placement à la souris des types de champ existants, relecture numérique et
Créer explicite (D110/A38). Implémentation, tests locaux, formatage et Clippy réussis ; contrôles de publication réussis.
**0.19.0 était la Latest stable à sa publication**.

Colonne « Attendu » : capacité attendue d'un éditeur PDF complet (**Déclaré** = courante dans les éditeurs du marché,
**Partiel** = annoncé comme incomplet par le projet, **Absent** = annoncé manquant. Non vérifié par exécution.

Statuts SuniPDF : **Fait** = implémenté **et** couvert par des tests exécutés ; **Livré en 0.30.0** =
publié dans les limites documentées et couvert par les contrôles exécutés ; **En cours** ;
**Planifié (Pn)** = phase de roadmap ; **Hors périmètre**. P1 Roadmap et Suggestions restent différés.

Les nouveautés 0.9.0 sont couvertes par des régressions automatisées. `PROJECT_STATUS.fr.md` suit les contrôles exécutés ;
ces résultats ne prouvent pas la compatibilité imprimante physique, ARM64 ou lecteur tiers.
Workspace, qpdf et régressions réelles portable/MSI 0.9.1 réussis ; résultats des paquets et de publication consignés dans ce suivi.

[La 0.10.0 est publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) avec le lot P1/P2 demandé.
Les 362 tests du workspace, 31 contrôles qpdf synthétiques, quatre documents NIST réels (744 pages), huit scénarios portables,
deux bases MSI et six fichiers publics ont été vérifiés ; preuves dans `PROJECT_STATUS.fr.md`.

[0.11.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) sous `lmdanavanem` ; Latest à cette date.

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

[0.12.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) sous `lmdanavanem` (Latest à cette date), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation réussie : 398 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 859 clés EN/FR, 48 contrôles qpdf et quatre documents NIST/NBS (744 pages ;
59,68 s). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) et
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) réussis,
ainsi que dix scénarios portables, quatre bases MSI, six fichiers préparés/publics et contrôles renforcés des binaires.
Copies vérifiées dans `dist/`. L'updater de production a vérifié le ZIP Latest x64 de 19 699 323 octets sans installation.
Limites matérielles, d'interface interactive et de lecteurs dans `PROJECT_STATUS.fr.md`.

[0.13.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem` (Latest à cette date), release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation réussie : 410 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 870 clés EN/FR, 53 contrôles qpdf et quatre documents licenciés
(744 pages ; 76,40 s localement), CI Windows et compilation ARM64 de même source, onze scénarios portables et
cinq bases MSI. Six fichiers préparés/publics et contrôles renforcés des binaires réussis, dont lots de signatures
protégés et les deux mots de passe. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le ZIP Latest
x64 de 19 730 578 octets sans installation. Preuves détaillées et limites restantes dans `PROJECT_STATUS.fr.md`.

[0.14.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) sous `lmdanavanem` (Latest à cette date), release
`414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation réussie : 420 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 890 clés EN/FR, 63 contrôles qpdf et quatre documents licenciés
(744 pages ; 53,91 s localement), CI Windows et compilation ARM64 de même source, douze scénarios portables et
six bases MSI. Six fichiers préparés/publics et contrôles renforcés des binaires réussis, dont création/réinitialisation
non chiffrée/protégée, les deux mots de passe, étiquettes hors plage et texte/PNG inchangés. Copies vérifiées dans
`dist/` ; l'updater de production a vérifié le ZIP Latest x64 de 19 785 200 octets sans installation.
Preuves et limites restantes dans `PROJECT_STATUS.fr.md`.

[0.15.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous `lmdanavanem` (Latest à cette date), release
`414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation réussie : 429 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés
(744 pages ; 48,891 s localement), CI Windows et compilation ARM64 de même source, treize scénarios portables et
sept bases MSI. Six fichiers préparés/publics et contrôles renforcés réussis, dont cibles non chiffrées/protégées,
les deux mots de passe, identité/zone des liens et PNG des pages inchangés. Copies vérifiées dans `dist/` ; le metteur
à jour de production a vérifié le ZIP Latest x64 de 19 803 226 octets sans installation.
Preuves et limites restantes dans `PROJECT_STATUS.fr.md`.

[0.16.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) sous
`lmdanavanem` (Latest à cette date), release `414934190`, source `6cd7843734f3924e87033d8b05539d493ccbcb95`. Validation réussie :
439 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy, 895 clés EN/FR, 73 contrôles qpdf et
quatre documents licenciés (744 pages ; 53,542 s localement), CI Windows et compilation ARM64 de même source,
quatorze scénarios portables et huit bases MSI. Six fichiers préparés/publics et contrôles renforcés des paquets
réussis ; copies vérifiées dans `dist/`. Le metteur à jour de production a vérifié le ZIP Latest x64 de
19 821 719 octets sans installation. Preuves et limites matérielles/d'interopérabilité dans `PROJECT_STATUS.fr.md`.

[0.17.0 publiée (Latest à cette date)](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) sous
`lmdanavanem`, release `415074108`, source `5785de282e126fb4666b28fb83340d0e9c6e9acf`. Validation réussie :
445 tests workspace (zéro échec, sept ignorés par défaut), formatage/Clippy, 896 clés EN/FR, 74 contrôles qpdf,
quatre documents licenciés/744 pages (48,187 s localement ; 45,791 s en CI), CI Windows et compilation ARM64
de même source, seize scénarios portables et dix bases MSI. Six fichiers préparés/publics et contrôles renforcés
de conversion réussis, dont neuf flux JPEG exacts, coins orientés, transparence PNG, refus budget/DPI et sources
inchangées. Copies vérifiées dans `dist/` ; le metteur à jour de production a vérifié le ZIP Latest x64 de
19 825 743 octets sans installation. Preuves et limites dans `PROJECT_STATUS.fr.md`.

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

## A. Lecture

| Fonction | Attendu | SuniPDF | Note |
|---|---|---|---|
| Ouvrir un PDF, afficher une page | Déclaré | **Fait** | PDFium ; tests `crates/sunipdf-core/tests/engine.rs` |
| Documents protégés par mot de passe | Déclaré (toutes révisions) | **Fait** | RC4, AES-128 et AES-256 couverts par les tests moteur/protection et rouverts par PDFium |
| Navigation page/précédente/suivante/aller à | Déclaré | **Fait** (UI) | Clavier + barre d'outils |
| Zoom, ajuster largeur/page, taille réelle | Déclaré | **Fait** (UI) | Rendu à la résolution physique (DPI) ; Ctrl + molette ancré sous le pointeur, 10 à 3200 % |
| Rendu hors thread UI | Déclaré | **Fait** | Thread moteur unique, liste de tuiles voulues remplacée en bloc, époques anti-obsolescence |
| Onglets multi-documents | Déclaré | **Fait** (suspension des documents inactifs, état d'interface conservé) | |
| Affichage continu, double page, rotation de vue | Déclaré | Continu/double page/Livre avec couverture seule **Faits** ; rotation de vue **faite en 0.10.0** | Tuiles/cache communs, régressions tailles mixtes, dernière page et zoom |
| Rendu en tuiles, cache plafonné | Déclaré | **Fait** (tuiles de 1024 px au-delà de 4096 px / 8 Mpx, cache LRU 256 Mo, textures 192 Mo) | `Document::render_region`, tests de pixels (régions, recollage, zoom ×40) ; CLI `render --region` |
| Recherche plein texte | Déclaré | **Fait** | Ctrl+F, casse, surbrillance, CLI `search` |
| Sélection / copie de texte en ordre de lecture | Déclaré | **Fait** : plage logique, sélection entre pages/dans les deux sens, double-clic sur mot, défilement au bord | Ordre PDFium ; tests moteur/UI sans fenêtre ; colonnes arbitraires non balisées non déduites |
| Miniatures | Déclaré | **Fait** | Panneau virtualisé, cache partagé |
| Signets (outline) | Déclaré | **Livré en 0.30.0** : Ajout/renommage/destination/déplacement/suppression avec Annuler ; conservation bornée en fusion/extraction. [Structures](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md). | Bornes : 4 096 signets / 32 niveaux ; cycles refusés. |
| Étiquettes de pages | Déclaré | **Livré en 0.30.0** : Recalage sémantique lié à la provenance ; doublons explicites et numéros imprimés inchangés. Création/réinitialisation antérieures à six styles conservées. [Périmètre](RELEASE_0.30.0.fr.md). | Navigation par libellé unique exact ou `#N` physique. |
| Historique de navigation, onglets rouverts | Déclaré | Fichiers récents et historique borné à 256 **Faits** ; historique même page/zoom **livré en 0.11.0** | Tests locaux réussis |
| Mode lecture / plein écran | Déclaré | **Fait en 0.10.0** | Mode de vue partagé comme préférence ; rotation/historique conservés par onglet ; retour à une vue droite pour éditer |
| Impression Windows | Déclaré (fidélité limitée) | **Fait** (tests géométrie/plages et sortie réelle Microsoft Print to PDF rouverte) | Boîte native, plages/copies, rendu tramé borné, progression/annulation ; imprimante physique et boîte interactive non vérifiées |
| Récupération après plantage, autosave | Déclaré | **Fait** (régressions récupération/worker/onglets) | Instantanés 60 s/suspension d'onglet ; source intacte ; chiffrement conservé/sans mots de passe ; restauration Enregistrer sous |
| Thèmes clair/sombre | Déclaré | **Fait** | Design system `theme.rs`, bascule dans la barre d'onglets |
| Interface traduisible | Déclaré (EN, JA + 22 catalogues) | **Fait** (en-US par défaut, fr-FR disponible) | Changement à chaud, préférences existantes conservées |

## B. Édition et annotation

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Notes, surlignage, souligné, barré, formes, encre, zones de texte | Déclaré | **Livré en 0.30.0** : Marquage antérieur conservé ; FreeText à police incorporée, polygones/polylignes/nuages, tampons standard et pièces jointes avec aperçu réel. Style uniforme, refus bornés ; [détails](RICH_ANNOTATIONS.fr.md). |
| Fils de réponses, statut des commentaires | Déclaré | **Fait dans SuniPDF** : panneau Commentaires en fils (réponses, statut Accepté / Refusé / Annulé / Terminé, auteur, date, suppression du fil), écrits selon ISO 32000-1 ; **compatibilité Acrobat / Foxit non vérifiée** ; CLI `reply` / `review-state` |
| Regrouper plusieurs traits à main levée | Déclaré | **Livré en 0.30.0** : Encre groupée livrée en 0.18.0. Gomme des traits entiers touchés, style/commentaires restants conservés ; pas de découpe de trait, validation à l'inactivité ni récupération du brouillon. [Limites](RICH_ANNOTATIONS.fr.md). |
| Création et édition FreeText sur la page | Déclaré | **Livré en 0.30.0** : Base sur la page livrée en 0.20.0 ; aperçu réel à police incorporée, faces gras/italique choisies et CJK horizontal présents. Sans styles par portion ni composition complexe. [Détails](RICH_ANNOTATIONS.fr.md). |
| Ajout de texte / d'images | Déclaré | **Livré en 0.30.0** : Texte/images antérieurs conservés ; découverte Unicode, sous-ensembles statiques TrueType/CFF1 et faces TTF/OTF/TTC/OTC choisies avec contrôles de droits/réutilisation. Polices variables/CFF2/couleur refusées. [Formats/bornes](FONT_EMBEDDING.fr.md). |
| Modification de texte existant | Partiel (faible en CJK/bidi) | **Livré en 0.30.0** : Garde des glyphes absents 0.21.1 conservée ; aperçu/réagencement multiligne borné avec métriques natives et Appliquer explicite. Polices/texte/états graphiques dangereux refusés. [Périmètre](RELEASE_0.30.0.fr.md). |
| Transformation d'objets | Déclaré | **Livré en 0.30.0** : Opérations antérieures conservées ; pivot mémorisé stable, remplacement/lecture explicites de l'opacité et état inconnu. [Périmètre](RELEASE_0.30.0.fr.md). |
| Sélection multiple, couper/copier/coller d'objets | Déclaré | **Livré en 0.30.0** : Collage groupé conservé ; **Coller éditable** explicite recrée les objets natifs pris en charge en un Annuler. Graphes dangereux refusés sans repli groupé silencieux. [Périmètre](RELEASE_0.30.0.fr.md). |
| Panneau des objets de la page | Déclaré | **Fait** (F3 : ordre de dessin, sélection, avancer/reculer, suppression, repère au survol) |
| Liens | Déclaré | **Livré en 0.30.0** : Liens page/URI antérieurs conservés ; création nommée/XYZ/Fit/FitH et résolution brute bornée, axes/zoom nuls et cadrage/rotation affichés conservés. Actions actives non prises en charge jamais exécutées. [Périmètre](RELEASE_0.30.0.fr.md). |
| Filigranes, en-têtes/pieds de page, numérotation | Partiel | **Fait** : tampon/filigrane texte (angle, opacité, position) et numérotation `{n}`/`{total}` (UI + CLI `stamp`, `page-numbers`) ; filigrane image et en-têtes multi-zones planifiés (P3) |
| Tampons personnalisés | Non documenté | **Livré en 0.30.0** : Annotations standard prédéfinies présentes ; création arbitraire de tampons toujours planifiée. [Types](RICH_ANNOTATIONS.fr.md). |

## C. Organisation

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Rotation, suppression, insertion, déplacement, pages vierges | Déclaré | **Fait** (UI + CLI `insert-blank` pour les pages vierges) |
| Recadrer les pages (CropBox) | Déclaré | **Fait** (dialogue en mm avec aperçu, rétablissement, CLI `crop`) ; masque seulement, pas de caviardage ; recadrage souris dans l’aperçu livré en 0.10.0 |
| Redimensionner les pages | Déclaré | **Livré en 0.30.0** : Commandes taille/contenu conservées ; mise à l'échelle facultative des annotations/liens, apparences non prises en charge/malformées refusées atomiquement. [Périmètre](RELEASE_0.30.0.fr.md). |
| Fusion de N PDF avec signets | Déclaré | **Livré en 0.30.0** : Conservation bornée des signets, destinations, champs, calques et pièces jointes ; collisions de noms entrants déterministes. [Limites](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md). |
| Division (tous les n / pages choisies) | Déclaré | **Fait** (UI dialogue + CLI `split`, `extract`) |
| Extraction préservant liens, champs, calques, pièces jointes | Déclaré | **Livré en 0.30.0** : Gardes chiffrement/source conservées ; graphes bornés préservés, destinations de pages omises retirées. XFA/graphes dangereux refusés ; contrôles documentés du lecteur indépendant réussis. [Limites](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md). |
| Réorganisation par glisser-déposer | Déclaré | **Fait** (miniatures et vue Organiser) |
| Undo/redo nommé survivant à la sauvegarde | Déclaré | Undo/redo **Fait** (instantanés, non nommé) ; survit à la sauvegarde. **Livré en 0.14.0** : navigation conservée lors d'Annuler/Rétablir contenu/étiquettes si géométrie inchangée ; contrôles exécutés réussis (D105) |
| Comparaison de deux versions | Déclaré | Implémenté pour la 0.31.0 : comparaison locale texte/dimensions/aperçus 768 pixels, décalages physiques explicites et aperçus bornés original/révision/différences. [Limites](LOCAL_REVIEW.fr.md). |
| Recherche dans les PDF sélectionnés | Déclaré | Implémenté pour la 0.31.0 : fichiers explicites, texte littéral, échecs par fichier, extraits/navigation et annulation. [Limites](LOCAL_REVIEW.fr.md). |
| Traitement par lots | Déclaré (CLI batch) | **Livré en 0.30.0** : Compression/OCR livrés en 0.10.0 ; lots Office présents, 1–128 entrées, 64 Mio/entrée, 120 s/conversion, annulation/nouvelles copies. [Limites](OFFICE_CONVERSION.fr.md). |

## D. Conversion et OCR

| Fonction | Attendu | SuniPDF |
|---|---|---|
| PDF → images | Déclaré (CLI render) | **Fait** (UI export PNG 150 ppp, CLI `render`, `export-images`) |
| PDF → texte | Déclaré | **Fait** (CLI `text`) |
| Images → PDF | Déclaré (create) | **Fait** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP). **Livré en 0.17.0** : incorporation directe des huit orientations JPEG EXIF ; estimation raster non-JPEG cumulée de 512 Mio `somme(max(octets décodés, largeur × hauteur × 4))` avant allocation, arithmétique vérifiée et géométrie finie/positive (D108). Qualité/transparence et accès conservés ; estimation distincte d'un plafond RSS natif/processus ou d'une limite JPEG comprimé. Contrôles locaux/CI/mises à niveau, conversion indépendante debug/paquets préparés/publics et metteur à jour de production réussis ; preuves et limites dans `PROJECT_STATUS.fr.md` |
| PDF → Word | Partiel (perd la mise en page) | **Livré en 0.30.0** : Texte visible simple monocolonne éditable réagencé à 11 pt ; polices/mise en page/pagination originales non reconstruites. Entrées complexes refusées. [Périmètre](OFFICE_CONVERSION.fr.md). |
| PDF → Excel / PowerPoint | Absent / export pptx partiel | **Livré en 0.30.0** : Grilles alignées en chaînes XLSX littérales ; PPTX en images de pages 150 ppp avec marges. Sans formules, reconstruction sémantique de tableaux ni contenu PPTX éditable. [Limites](OFFICE_CONVERSION.fr.md). |
| Office → PDF | Absent | **Livré en 0.30.0** : Entrées sans macros contrôlées, LibreOffice 26.2/26.8 installé, profils isolés et nouvelles copies. Indisponibilité explicite, aucune installation automatique. [Formats/profils](OFFICE_CONVERSION.fr.md). |
| HTML → PDF | Absent | Planifié (P5, optionnel) |
| Compression des images | Déclaré | **Faite en 0.10.0** : RGB/gris bornés, qualité JPEG/côté maximal, copie exacte sans gain, masques/profils conservés, PDF signés/restreints refusés |
| OCR | Partiel (latin, modèles CC-BY-SA) | **Fait** : Tesseract 5.5.3 / Leptonica 1.87 embarqués hors ligne, modèles rapides eng/fra, texte invisible, pages textuelles ignorées, progression/annulation/un undo ; tests moteur et reconnaissance CLI réelle ; CLI `ocr` |

## E. Sécurité et signatures

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Ouvrir chiffré, honorer les permissions | Déclaré | **Fait** (ouverture ; permissions affichées dans les propriétés) |
| Ajouter/modifier mot de passe, AES-256 | Déclaré | **Fait** (AES-128/256, permissions ; AES-128 limité aux mots de passe Latin-1, refus typé ; accessibilité toujours accordée) via `lopdf`, relu par PDFium ; longueur AES-256 explicite et contrôle qpdf synthétique ajoutés pour 0.9.1 ; résultats dans `PROJECT_STATUS.fr.md` |
| Caviardage irréversible | Déclaré (crate redact) | **Livré en 0.30.0** : Aperçu natif, remplissage/texte, marques non appliquées persistantes, découpe sûre Base14 latine et nettoyage partagé. Repli image des objets complexes ; marques seules sans caviardage. [Garanties](REDACTION.fr.md). |
| Nettoyage des métadonnées | Déclaré | **Livré en 0.30.0** : Préservation des métadonnées PDF/A conservée, sans promesse de conformité. Copie assainie séparée/données cachées prises en charge retirées/réécriture ; sauvegardes/Annuler/secteurs non effacés. [Modèle de sécurité](SECURITY_MODEL.fr.md). |
| Signature visuelle (image) | Déclaré | **Livré en 0.30.0** : Dessin/saisie/import conservés ; bibliothèque DPAPI utilisateur facultative, migration vérifiée, désactivation/suppression et gardes profil/ACL. Clés du profil requises pour récupérer ; session ouverte permettant réutilisation. Sans preuve d'identité/intégrité. [Détails](SIGNATURE_SECURITY.fr.md). |
| Signature numérique CMS/PKCS#12, horodatage | Déclaré | **Livré en 0.30.0** : Intégrité/signature PFX conservées ; chaînes/révocations Windows/RFC3161 distincts. Hors ligne par défaut, réseau consenti par vérification, inconnu jamais fiable. DSS non vérifié ; sans LTV d'archivage/création d'horodatage. [Limites](SIGNATURE_SECURITY.fr.md). |
| PKCS#11, LTV en ligne, AATL/EUTL | Absent | Hors périmètre initial |

## F. Formulaires et accessibilité

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Remplir AcroForm, apparences régénérées | Déclaré | **Fait** (tranche forms, D061–D063) : texte (une ligne, multiligne, longueur maximale), case à cocher, boutons radio, liste déroulante, zone de liste (choix multiple), champs hiérarchiques et à plusieurs widgets ; valeurs écrites par lopdf, apparences régénérées par PDFium et présentes dans le fichier ; outil sur la page + panneau accessible + aplatissement (champs et annotations) + effacer ; document chiffré conservé chiffré ; CLI `form-fields` / `fill-form` / `flatten` ; scripts jamais exécutés (signalés) |
| Créer des champs | Non documenté | **Fait** : boîte numérique texte/case ; radio/listes livrés en 0.10.0 ; noms uniques, pages pivotées/recadrées/chiffrées, annulation ; tests moteur/CLI ; CLI `add-form-field` |
| Tracer les rectangles de nouveaux champs sur la page | Non documenté | **Publié en 0.19.0** (D110) : mêmes cinq types et boîte numérique, nom/choix/options conservés, rectangle fini sur une page d'au moins 4 pt par dimension, Créer explicite/un Annuler, gardes de document/onglet/outil périmés. Tests locaux, formatage et Clippy réussis ; contrôles de publication réussis. Aucun nouveau type ni API moteur/CLI |
| Import/export FDF/XFDF | Déclaré | **Fait en 0.10.0** : export JSON/XFDF, import strict borné JSON/XFDF/FDF de valeurs, Appliquer/Abandonner groupés et une annulation ; sous-ensemble FDF, aucun script/pièce jointe/texte enrichi |
| Récupération après erreur de lecture des champs | Non documenté | **Livrée en 0.11.0** : erreur/relance explicites préservent brouillons et actions en attente ; tests locaux réussis |
| Texte libre des listes déroulantes déjà modifiables | Non documenté | **Livré en 0.11.0** : texte libre/choix dans le panneau et sur la page, Appliquer/Abandonner/Enregistrer et annulation ; contrôles locaux, CI, paquets et mises à niveau réussis |
| Créer des listes déroulantes modifiables | Non documenté | **Livré en 0.12.0** : case de l'interface et CLI `add-form-field --kind combo --editable` ; choix prédéfinis non vides requis ; contrôles locaux, CI, paquets et mises à niveau réussis |
| Exports radio `/Opt` | Non documenté | **Livrés en 0.12.0** : valeurs d'export distinctes des états des widgets pour remplissage/JSON/XFDF/FDF ; ajout CLI `widgets[].export_value` ; correspondances malformées/dupliquées : champ en lecture seule ; radios sans `/Opt` inchangés ; contrôles locaux, CI, paquets et mises à niveau réussis |
| XFA | Partiel | **Détecté, lecture seule** (champs AcroForm listés ; remplissage et aplatissement refusés, `form_xfa`) |
| Conserver les annotations non-Link sans apparence utilisable à l'aplatissement | Non documenté | **Livré en 0.21.1 :** graphes bruts non-widget, pièces jointes et références partagées conservés ; widgets/apparences dessinables aplatis, annotations conservées exclues du compte, échecs atomiques. Contrôles de la source de release et des paquets publics réussis ; absent des paquets 0.21.0 ; [limites et preuves](PROJECT_STATUS.fr.md) |
| Balisage, vérifications PDF/UA | Partiel / absent | Planifié (P6, vérifications seulement) |

## G. Automatisation et intégration

| Fonction | Attendu | SuniPDF |
|---|---|---|
| CLI JSON | Déclaré | **Fait** (lecture, pages, sécurité, conversion, tampons et édition du contenu : `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts`) ; extension P5 |
| Registre de commandes commun UI/CLI | Déclaré (tool table) | **Fait** (`sunipdf-cli commands`) |
| Serveur MCP local, confiné à `--root` | Déclaré | Planifié (P5, opt-in) |
| Intégration Explorateur Windows (association, menu contextuel) | Non documenté | Planifié (P5) |
| Palette de commandes | Déclaré | **Fait** (Ctrl+K, filtrage, clavier, couleur de l'outil propriétaire) |
| Hub « Tous les outils » par tâches, barre contextuelle par outil | Déclaré (liste « All tools ») | **Fait** pour les 14 outils ; Numériser et OCR, Préparer un formulaire et Utiliser un certificat ont leurs commandes ; tests registre et activation sans fenêtre |
| Workflows locaux, profils de conversion | Absent | **Livré en 0.30.0** : Profils Office Standard/Impression mémorisés. Concepteur général de workflows toujours planifié. [Périmètre](OFFICE_CONVERSION.fr.md). |

## H. Distribution

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Windows x64 | Déclaré (MSI, zip) | **Fait** : 0.20.0 ZIP/MSI publiés en Latest ; dix-huit scénarios portables et douze bases MSI réussis (60 journaux MSI) ; six fichiers publics/SHA256 et contrôles embarqués vérifiés ; mise à jour de production vérifiée sans installation ; preuves dans `PROJECT_STATUS.fr.md` ; UX interactive non vérifiée |
| Windows ARM64 | Déclaré | **Fait** : ZIP portable 0.19.0 de même source compilé en CI et publié ; SHA256 public, architecture, modèles, notices et icônes vérifiés ; preuves dans `PROJECT_STATUS.fr.md` ; exécution matérielle non vérifiée |
| Signature de code | Non documenté | Planifié (P6) si certificat fourni |
| Moteur GPU facultatif | Non documenté | **Livré en 0.30.0** : compatibilité wgpu facultative et contrôle DX12 logiciel ; OpenGL reste le défaut. Qualification GPU/fenêtres et matériel ARM64 réels distincte |
| Inventaire licences des dépendances/composants | Non documenté | **Livré en 0.30.0** : inventaire hors ligne des versions exactes et contrôles du contenu des paquets ; [périmètre](RELEASE_0.30.0.fr.md). Maintenance des licences toujours nécessaire |
