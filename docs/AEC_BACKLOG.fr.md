# Backlog PDF pour le bâtiment et collaboration en entreprise

[English](AEC_BACKLOG.md) · [CSV importable](AEC_BACKLOG.csv) · [Plan cloud](CLOUD_STRATEGY.fr.md)

Orientation du propriétaire, 21 octobre 2026 : une application locale gratuite et open source pour les plans, métrés et revues PDF, avec un cloud géré payant pour les entreprises travaillant sur des documents communs. Les tarifs seront définis plus tard. La licence actuelle MIT OR Apache-2.0 reste inchangée ; le cloud est un service facultatif et aucun compte n'est nécessaire pour les fonctions locales.

Le cloud et la collaboration sont **reportés** à la demande du propriétaire. Aucun service cloud ne fait partie de la 0.40.0 ; les fonctionnalités locales passent en premier.

Ce backlog reprend les besoins métier du texte fourni, sans annoncer de parité complète avec Bluebeam Revu ni de compatibilité avec ses formats propriétaires. Les interfaces, implémentations et formats d'échange SuniPDF seront indépendants.

## Base existante et statuts

L'édition, les annotations, les fils de commentaires, les statuts PDF, l'OCR, les formulaires, la protection, la récupération et les opérations locales existent dans les limites de la [matrice](FEATURE_MATRIX.fr.md). La comparaison et la recherche multi-PDF existent ; le recalage de plans, les nuages automatiques et la reconnaissance de symboles restent distincts. Les modèles de décoration de la 0.39 et les tampons image du lot 0.40 ne constituent pas encore une bibliothèque partagée d'annotations.

Le lot local 0.40.0 ajoute aussi calibration sur deux points, cinq types de mesures et données JSON. [Guide et limites](MEASUREMENTS.fr.md). Ce progrès partiel sur AEC-001/002/003/026 ne ferme pas les cartes : saisie de coordonnées et échelles par annotation ne réalisent ni recalcul global de page, ni manipulation directe, vues multi-échelles ou corpus complet.

P1 : fondations prioritaires. P2 : extensions métier. P3 : intégrations ou infrastructures distinctes. Les 26 cartes restent **À planifier** tant que leurs critères complets ne sont pas démontrés. Les progrès partiels ne ferment aucune carte ; les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent indépendants.

| ID | Priorité | Fonction prévue | Critères de réalisation | Dépendances |
|---|---|---|---|---|
| AEC-001 | P1 | Calibration et unités | Calibration sur deux points et distance connue, unités métriques/pieds-pouces, échelle et précision visibles, sauvegarde et Annuler. Refuser les échelles nulles/ambiguës ; vérifier rotations, rognage et dimensions connues. | Géométrie existante |
| AEC-002 | P1 | Mesures sur plans | Longueur, polyligne, périmètre, surface polygonale et angle avec géométrie modifiable et unités calibrées. Recalcul explicite après changement d'échelle ; aucune mesure périmée présentée comme valide. | AEC-001 |
| AEC-003 | P1 | Données de métrés | Relier géométrie, échelle, unités et identifiant d'annotation dans un format versionné. Réouverture/export fidèles ; signaler les métadonnées importées non prises en charge. | AEC-001, AEC-002 |
| AEC-004 | P1 | Tableau des annotations | Tri, filtres et recherche par page, auteur, date, statut, discipline et champs métier ; accès à l'annotation. Distinguer statuts métier et statuts PDF. Exports CSV/JSON avec inconnues explicites et protection contre les formules injectées. | Fils existants |
| AEC-005 | P1 | Comptage manuel | Repères de comptage, groupes par équipement, correction/suppression avec Annuler et export des totaux reliés aux repères. Distinguer candidats de recherche et occurrences validées. | AEC-003, AEC-004 |
| AEC-006 | P1 | Bibliothèques d'annotations | Réutiliser/partager styles, tampons image, symboles et groupes pris en charge dans un format ouvert documenté. Imports bornés, licences/assets validés, scripts et références dangereuses refusés. | Annotations et modèles existants |
| AEC-007 | P2 | Colonnes et formules | Champs texte/choix/date/nombre/formule, regroupements, totaux et calculs quantité/prix unitaire. Expressions bornées sans code exécutable, unités/devises/arrondis explicites. Prix fournis par l'utilisateur. | AEC-003, AEC-004 |
| AEC-008 | P2 | Échelles multiples et dessin coté | Zones nommées à échelle propre, règle de chevauchement et sélection explicite de l'échelle ; dessin aux dimensions réelles et accrochage aux vecteurs admissibles. | AEC-001, AEC-002 |
| AEC-009 | P2 | Volumes et déductions | Profondeur, trous/découpes et paramètres conservés ; unités cohérentes et refus des géométries dégénérées ou auto-intersectées. | AEC-002, AEC-003 |
| AEC-010 | P2 | Pièces et zones | Régions nommées/hiérarchiques, classement déterministe des annotations, filtres et rapports par pièce/étage ; chevauchements et éléments non classés visibles. | AEC-004, AEC-008 |
| AEC-011 | P2 | Recherche visuelle de symboles | Recherche locale à partir d'un symbole, sensibilité/rotations et prévisualisation des candidats. Seules les occurrences validées alimentent les comptages ; résultats partiels et faux positifs explicites. | AEC-005, AEC-026 |
| AEC-012 | P2 | Remplissage de régions | Prévisualiser une région délimitée par les traits, fermer manuellement les ouvertures et conserver les trous. Convertir le résultat validé en mesure ; documenter tolérance/résolution et refuser les fuites. | AEC-002, AEC-009, AEC-026 |
| AEC-013 | P2 | Superposition colorée recalée | Recaler plusieurs plans par points de référence, afficher/exporter les couches colorées. Conserver les originaux ; faire vérifier les alignements et correspondances ambigus. | Comparaison existante, AEC-026 |
| AEC-014 | P2 | Nuages de révision | Transformer les différences vérifiées en nuages et lignes du registre. Relier chaque résultat aux deux révisions ; distinguer changement réel et bruit de rendu. | Comparaison, AEC-004, AEC-013 |
| AEC-015 | P2 | Ensembles de plans | Parcourir des PDF séparés comme un dossier indexé par discipline/feuille/révision. Manifeste portable et signalement des fichiers déplacés, manquants ou dupliqués. | Labels/signets existants |
| AEC-016 | P2 | Remplacement de feuilles par lot | Vérifier les correspondances, transférer les annotations compatibles sous une transformation explicite, conserver les versions précédentes et produire un bilan par feuille. Aucun écrasement partiel. | AEC-013, AEC-015 |
| AEC-017 | P2 | Liens automatiques par lot | Détecter les références dans le texte/OCR, prévisualiser les destinations et créer les liens vérifiés dans de nouvelles copies. Références ambiguës, fichiers absents et chemins dangereux restent non résolus. | AEC-015, OCR/liens |
| AEC-018 | P2 | Légendes et rapports | Légendes, synthèses de réserves/métrés et rapports PDF/CSV/JSON liés aux filtres, annotations et révisions réelles. Aucun total inventé ou périmé. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Liaison tableur | Instantanés CSV/XLSX et rafraîchissement explicite des quantités avec unités/révision source. Formats ouverts sans Excel obligatoire ; aucune macro ni écrasement silencieux. | AEC-003, AEC-007 |
| AEC-020 | P3 | Revues partagées en temps réel | Plusieurs collègues annotent une base PDF commune, avec identités, historique, reconnexion, conflits et permissions testées entre clients. La réservation d'un fichier ne constitue pas une annotation simultanée. | AEC-003, AEC-004, AEC-021 |
| AEC-021 | P3 | Projets cloud d'entreprise | Entreprises isolées, lecteurs/éditeurs/administrateurs, PDF partagés, versions immuables, réservations et publication protégée. Téléchargement des anciennes versions et accès facultatif depuis l'app. Cloud géré payant ; tarifs ultérieurs. SSO, sauvegardes et exploitation à qualifier séparément. | [Plan cloud](CLOUD_STRATEGY.fr.md) |
| AEC-022 | P3 | CAO/BIM et import de pièces | Commencer par des échanges ouverts documentés, notamment IFC, et des échantillons licenciés. Import vérifié des zones/métadonnées. Adaptateurs Revit/AutoCAD facultatifs soumis à leurs propres SDK et licences. | AEC-010, AEC-015 |
| AEC-023 | P3 | PDF 3D | Étudier lecture/rendu U3D/PRC, annotations, isolation des ressources et licences sur des échantillons autorisés. Contenus non pris en charge explicites ; aucun script automatique. | AEC-026, étude distincte |
| AEC-024 | P3 | Appariement assisté des plans | Suggérer correspondances et recalages avec confiance, cas non résolus et validation humaine. Mesurer la qualité sur des exemples vérifiés avant d'automatiser les remplacements. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Automatisation/MCP facultative | Accès local/projet limité et volontaire, outils autorisés et typés, limites de ressources, écritures explicites et journal. Aucun fournisseur IA payant ou compte cloud obligatoire pour l'usage local. | CLI et permissions |
| AEC-026 | P1 | Corpus et qualification bâtiment | Plans synthétiques/licenciés avec dimensions, révisions, zones et comptages connus. Unités, quatre rotations/rognages, chiffrement, Annuler/échecs atomiques, lecteurs indépendants, accessibilité et mesures matérielles. | Toutes les cartes |

## Ordre proposé

1. Qualifier les fonctions locales de la 0.40.0. Le cloud et la collaboration sont reportés à la demande du propriétaire.
2. Calibration/mesures avec corpus, puis données de métrés, tableau et comptages.
3. Bibliothèques, rapports, superpositions, ensembles et gestion des révisions par lots vérifiables.
4. Reprendre le cloud et la collaboration après les priorités locales, dans un lot distinct avec isolation, identité, conflits, sauvegardes et exploitation qualifiés.
5. Évaluer reconnaissance avancée, BIM/3D et automatisation selon la demande et les licences compatibles.

Aucune date ni grille tarifaire n'est annoncée. Le CSV permet une future importation dans Projects ; les modifications du tableau GitHub restent reportées à la demande du propriétaire.

Les [références officielles](AEC_BACKLOG.md#reference-workflows) servent à décrire les usages demandés, sans annoncer de compatibilité SuniPDF avec les formats ou services Bluebeam.
