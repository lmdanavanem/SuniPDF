# Backlog des fonctions locales pour les plans PDF

[English](AEC_BACKLOG.md) · [CSV importable](AEC_BACKLOG.csv)

SuniPDF est une application locale gratuite et open source pour les plans, les métrés et les revues PDF, sous licence MIT OR Apache-2.0. Ce backlog décrit les fonctions à développer et leurs critères de réalisation ; les statuts distinguent les progrès partiels des fonctions qualifiées.

## Commandes de lecture et outils organisés — 0.50.0

Retrouvez 18 outils dans cinq familles filtrables depuis Outils, avec Mesures et la bibliothèque sous Réviser et mesurer. Basculez le volet gauche entre Outils, Pages et Signets ; utilisez quatre sections de mesure à droite. Les commandes de page et de zoom sont sous le PDF, les pourcentages comme 149% restent sur une ligne et les options de lecture restent accessibles dans les dispositions compactes. La rotation de lecture change uniquement l’affichage ; le rail droit ouvre les panneaux. Un seul panneau contextuel occupe le côté droit. Un brouillon de mesure actif bloque l’ouverture des autres panneaux ; quitter explicitement l’outil ou en changer annule ce brouillon local et conserve les échelles confirmées. Cette version réorganise les parcours existants de lecture et d’outils sans ajouter de types de mesure. AEC-002 et AEC-003 restent partiels ; accrochage vectoriel, recalcul par lots, vues multi-échelles et qualification sur plans réels restent ouverts. Les contrôles UI hors écran couvrent uniquement les dispositions et interactions consignées. Fenêtres natives, périphériques physiques, lecteurs d’écran et ARM64 natif restent non qualifiés. Les paquets restent non signés ; les critères complets 1.0 restent ouverts. [Preuves](qa/RELEASE_0.50.0.json).

## Ajout et suppression de sommets de mesures enregistrées — 0.49.0

Ajoutez ou supprimez des sommets de polylignes, périmètres et surfaces polygonales enregistrés, y compris sur les segments de fermeture, et sélectionnez un point par son numéro lorsque les poignées se confondent. Prévisualisez, ajustez puis appliquez avec une étape d’annulation. Calibration, unité, précision, identité, réponses et champs de quantité saisis sont conservés ; valeurs mesurées et totaux tarifaires compatibles sont recalculés. Les modifications périmées, invalides ou interdites restent contrôlées. AEC-002 et AEC-003 restent partiels. Les polylignes gardent 2 à 128 sommets ; les périmètres et surfaces polygonales, 3 à 128. Longueurs et angles gardent leur nombre fixe de points ; le type de mesure ne change pas. Accrochage vectoriel, recalcul par lots, vues multi-échelles et qualification sur plans réels restent ouverts. Les paquets restent non signés ; ARM64 natif, accessibilité, matériel et critères généraux 1.0 restent ouverts. [Preuves](qa/RELEASE_0.49.0.json).

## Modification des sommets des mesures enregistrées — 0.48.0

Modifiez les sommets existants d’une mesure enregistrée sur sa page d’origine, examinez l’aperçu, ajustez le brouillon puis appliquez avec une seule étape d’annulation. La mesure conserve ses propres calibration, unité, précision, identité, réponses et champs de quantité saisis. Valeurs mesurées et totaux tarifaires compatibles suivent la géométrie validée ; les modifications périmées, invalides ou interdites restent contrôlées. AEC-002 et AEC-003 restent partiels. Type de mesure et nombre de sommets restent fixes dans ce parcours. Ajout/suppression de sommets, accrochage vectoriel, recalcul par lots, vues multi-échelles et qualification sur plans réels restent ouverts. Les paquets restent non signés ; ARM64 natif, accessibilité, matériel et critères généraux 1.0 restent ouverts. [Preuves](qa/RELEASE_0.48.0.json).

## Superpositions de révisions alignées — 0.47.0

Superposez deux pages PDF enregistrées en magenta/cyan, alignez-les par translation, rotation horaire et échelle explicites ou par deux paires de points correspondants, puis exportez un nouveau PNG ou rapport JSON lié aux sources. Les deux fichiers doivent autoriser la copie ; empreintes exactes, géométrie recadrée/pivotée, rasters bornés et sorties nouvelles préservent les originaux. AEC-013 reste **En cours**. AEC-013 reste partiel. Recalage automatique, comparaison vectorielle, fusion PDF et contours automatiques des changements restent ouverts ; AEC-014 n’est pas livré. Cette superposition raster de deux pages sélectionnées est limitée au cadre de la base. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts. [Preuves](qa/RELEASE_0.47.0.json).

## Tables de quantités et champs typés — 0.46.0

Consultez les comptages et mesures réels dans des tables filtrées, modifiez champs typés et prix unitaires explicites avec Annuler, puis exportez CSV/JSON liés à la source. Les totaux distinguent absence et zéro, convertissent les bases de prix compatibles et séparent les monnaies. La CLI exige un enregistrement attendu et une nouvelle copie PDF ; droits de copie et d’annotation restent contrôlés. AEC-007 reste **En cours**. Formules libres, taxes, conversion monétaire, reconnaissance automatique de symboles, agrégation entre documents et autres critères complets d’estimation restent ouverts. Les angles sont visibles, mais exclus des quantités additives et des prix. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts. [Preuves](qa/RELEASE_0.46.0.json).

## Bibliothèques locales d’annotations — 0.45.0

Enregistrez des brouillons d’annotations pris en charge comme modèles nommés, regroupez jusqu’à 16 primitives puis placez un aperçu à une origine et une échelle explicites, avec une seule étape Annuler et de nouvelles identités. Le JSON autonome strict permet import, export et suppression ; la CLI écrit une nouvelle copie PDF en conservant chiffrement et permissions. AEC-006 reste **En cours**. La capture arbitraire d’annotations PDF existantes, les groupes imbriqués, le transfert de mesures/calibration et les autres critères complets de bibliothèque restent ouverts. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts. [Preuves](qa/RELEASE_0.45.0.json).

## Base existante et statuts

Le lot 0.41.0 ajoute [comptages manuels et tableau des annotations](MARKUP_REGISTER.fr.md) : repères persistants par clic, groupes et totaux reliés aux identités, changement de groupe, déplacement/suppression/Annuler, recherche/filtres/navigation sur toutes les pages, champs métier distincts et exports CSV/JSON. AEC-005 est terminé dans les limites documentées et qualifié en 0.41.0. AEC-004 reste partiel : les champs typés et dates normalisées du lot 0.46 complètent ces bases ; le tri par clé personnalisée et les autres critères restent ouverts.

L'édition, les annotations, les fils de commentaires, les statuts PDF, l'OCR, les formulaires, la protection, la récupération et les opérations locales existent dans les limites de la [matrice](FEATURE_MATRIX.fr.md). La comparaison et la recherche multi-PDF existent ; le recalage de plans, les contours de révision automatiques et la reconnaissance de symboles restent distincts. Les modèles de décoration de la 0.39 et les tampons image du lot 0.40 ne constituaient pas, à eux seuls, une bibliothèque locale d'annotations.

Le lot local 0.40.0 a introduit calibration sur deux points, cinq types de mesures et données JSON par saisie de coordonnées. [Guide et limites](MEASUREMENTS.fr.md). Il constitue une base pour AEC-001/002/003/026 ; ces cartes restent ouvertes.

## Édition des mesures enregistrées livrée en 0.44.0

Le lot publié, dans ses limites documentées, ajoute : modification de la distance de référence, de l’unité et de la précision dans le parcours d’aperçu du bureau, ou de la géométrie par la CLI protégée. Le remplacement conserve identité, réponses et données du registre ; échelles de page et autres mesures restent indépendantes. Droits d’export, refus des cibles ambiguës et des redimensionnements incompatibles corrigent les constats de l’audit. AEC-001/002/003/026 restent **En cours** ; déplacement direct des sommets, recalcul par lot, échelles multiples et qualification native/sur plans réels restent ouverts. [Périmètre et audit](RELEASE_0.44.0.fr.md). [Preuves](qa/RELEASE_0.44.0.json).

## Échelles persistantes et inspection livrées en 0.43.0

Le lot 0.43.0 publié ajoute une échelle par défaut enregistrée explicitement sur chaque page, distincte de celle conservée dans chaque mesure. Son ajout ou retrait participe à Annuler et nécessite l'enregistrement du PDF ; seules les nouvelles mesures l'utilisent. Un inspecteur en lecture seule expose valeurs, unités, précision et identité des mesures, avec navigation et réutilisation explicite de l'échelle d'une mesure non angulaire pour la session. La CLI lit, enregistre et retire les échelles dans de nouvelles copies, puis peut les utiliser avec conversion physique des unités.

Les échelles persistantes exigent le droit de modification du document. Les métadonnées versionnées et bornées vérifient les limites de page et le contenu décodé avant réutilisation ; rognage, redimensionnement ou modification du contenu peuvent imposer une nouvelle calibration. Les annotations restent indépendantes. AEC-001/002/003/026 restent **En cours** : modification des sommets enregistrés, recalcul explicite, échelles multiples, accrochage aux vecteurs et qualification native/sur plans réels restent ouverts. Le lot publié est vérifié dans ses limites documentées ; les critères partiels restent ouverts. [Preuves](qa/RELEASE_0.43.0.json).

## Intégration des mesures livrée en 0.42.0

Le lot 0.42.0 publié rend **Mesures** visible dans la barre principale, **Tous les outils** et **Commenter**. Il ajoute calibration de page par deux clics et confirmation explicite de la distance, tracé direct des cinq types de mesures, contours temporaires, aperçu rendu puis application, retrait de points, annulation et alignement horizontal/vertical. Une échelle confirmée peut être réutilisée sur sa page pendant la session du document et résiste aux modifications d'annotations ainsi qu'aux changements habituels d'onglet ; les changements de géométrie/structure des pages l'invalident, et fermer puis rouvrir le document impose une nouvelle calibration. Chaque annotation appliquée conserve sa propre calibration enregistrée. Ce progrès est livré en 0.42.0, mais ne clôture pas les cartes AEC-001/002/003/026. Modification des sommets enregistrés, échelles communes persistantes et recalcul, accrochage aux vecteurs PDF, vues multi-échelles et qualification restante sont toujours ouverts. Les comptages et le tableau conservent leur statut 0.41.0 documenté plus haut.

## Priorités et critères

P1 : fondations prioritaires. P2 : extensions métier. P3 : intégrations ou infrastructures distinctes. Les cartes restent ouvertes tant que leurs critères complets ne sont pas démontrés ; le CSV distingue implémentation et qualification. Les progrès partiels ne ferment aucune carte ; les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent indépendants.

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
| AEC-014 | P2 | Contours de révision | Transformer les différences vérifiées en contours modifiables et lignes du registre. Relier chaque résultat aux deux révisions ; distinguer changement réel et bruit de rendu. | Comparaison, AEC-004, AEC-013 |
| AEC-015 | P2 | Ensembles de plans | Parcourir des PDF séparés comme un dossier indexé par discipline/feuille/révision. Manifeste portable et signalement des fichiers déplacés, manquants ou dupliqués. | Labels/signets existants |
| AEC-016 | P2 | Remplacement de feuilles par lot | Vérifier les correspondances, transférer les annotations compatibles sous une transformation explicite, conserver les versions précédentes et produire un bilan par feuille. Aucun écrasement partiel. | AEC-013, AEC-015 |
| AEC-017 | P2 | Liens automatiques par lot | Détecter les références dans le texte/OCR, prévisualiser les destinations et créer les liens vérifiés dans de nouvelles copies. Références ambiguës, fichiers absents et chemins dangereux restent non résolus. | AEC-015, OCR/liens |
| AEC-018 | P2 | Légendes et rapports | Légendes, synthèses de réserves/métrés et rapports PDF/CSV/JSON liés aux filtres, annotations et révisions réelles. Aucun total inventé ou périmé. | AEC-004, AEC-007, AEC-010 |
| AEC-019 | P2 | Liaison tableur | Instantanés CSV/XLSX et rafraîchissement explicite des quantités avec unités/révision source. Formats de tableur ouverts documentés ; aucune macro ni écrasement silencieux. | AEC-003, AEC-007 |
| AEC-022 | P3 | CAO/BIM et import de pièces | Commencer par des échanges ouverts documentés, notamment IFC, et des échantillons licenciés. Import vérifié des zones/métadonnées. Adaptateurs facultatifs pour formats propriétaires, soumis à une vérification distincte des SDK et licences. | AEC-010, AEC-015 |
| AEC-023 | P3 | PDF 3D | Étudier lecture/rendu U3D/PRC, annotations, isolation des ressources et licences sur des échantillons autorisés. Contenus non pris en charge explicites ; aucun script automatique. | AEC-026, étude distincte |
| AEC-024 | P3 | Appariement assisté des plans | Suggérer correspondances et recalages avec confiance, cas non résolus et validation humaine. Mesurer la qualité sur des exemples vérifiés avant d'automatiser les remplacements. | AEC-013, AEC-015, AEC-016 |
| AEC-025 | P3 | Automatisation locale | Accès volontaire et limité aux fichiers locaux, commandes autorisées et typées, limites de ressources, écritures explicites et journal. | CLI et permissions |
| AEC-026 | P1 | Corpus et qualification bâtiment | Plans synthétiques/licenciés avec dimensions, révisions, zones et comptages connus. Unités, quatre rotations/rognages, chiffrement, Annuler/échecs atomiques, lecteurs indépendants, accessibilité et mesures matérielles. | Toutes les cartes |

## Ordre proposé

1. Étendre la qualification du parcours de mesure directe sur plans réels et interfaces natives.
2. Étendre la qualification des échelles de page et de l'inspecteur, puis compléter AEC-001/002 avec AEC-026 : édition des géométries enregistrées, recalcul explicite, vérifications restantes sur plans réels et accessibilité. S'appuyer sur les données de métrés, le tableau et les comptages existants.
3. Développer bibliothèques, rapports, superpositions, ensembles de plans et gestion des révisions par lots vérifiables.
4. Évaluer reconnaissance avancée, BIM/3D et automatisation selon les besoins et les licences compatibles.

Le CSV conserve les identifiants des fonctions et permet une future importation dans Projects. Les modifications du tableau GitHub restent reportées à la demande du propriétaire.

[Preuves de release](qa/RELEASE_0.42.0.json).
