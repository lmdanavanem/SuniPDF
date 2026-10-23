# SuniPDF 0.60.0 — Espace de travail des plans et mesures calibrées

[English](RELEASE_0.60.0.md) · [Parcours et limites](PLAN_WORKSPACE.fr.md)

Statut : version candidate. La qualification native est requise avant publication.

- Réutilisation de la disposition des pages pendant le défilement ; mise en cache des filtres, tris et totaux du registre lorsque les données ne changent pas.
- Recherche et inventaire des annotations traités page par page, avec progression et annulation entre pages pour laisser passer les demandes de rendu visibles.
- Tables d'annotations et de quantités dans un panneau inférieur redimensionnable ou une fenêtre flottante ; position et hauteur mémorisées, navigation d'une ligne vers l'annotation et ses propriétés.
- Accrochage aux extrémités vectorielles et intersections de segments droits du PDF, avec repère visible. Rognage, rotation, transformations et découpes convexes compatibles sont pris en compte ; aucun point vectoriel n'est inventé sur un scan.
- Saisie des distances en décimal ou en pieds/pouces, fractions comprises (`5' 6 1/2"`), avec conversion explicite de l'unité.
- Zones de mesure nommées et enregistrées avec des échelles indépendantes. Les chevauchements exigent un choix explicite ; une mesure qui sort de la zone sélectionnée est refusée. Les annotations existantes conservent leur propre calibration.
- Aperçu du recalcul sur des pages et mesures explicitement sélectionnées, puis application du lot avec un seul Annuler. Document modifié, page ultérieure invalide, données incompatibles et PDF signés sont refusés ; droits AES et identités des annotations sont conservés.
- Commandes CLI pour lire/remplacer les zones avec contrôle de l'état attendu, prévisualiser/recalculer dans une nouvelle copie et inspecter les points d'accrochage.
- Ajout des paquets Windows portables et MSI 0.51.0 aux régressions de mise à jour ; conservation des formats Ubuntu amd64 et Windows.

## Limite de qualification

Les nouveaux cas automatisés utilisent des PDF synthétiques à dimensions connues et le corpus de régression public/sous licence conservé. Ils couvrent calculs, rognage/rotation, transformations imbriquées, découpes incompatibles, enregistrements attendus, réouverture, refus atomiques, Annuler et chiffrement. Il s'agit du périmètre produit P1 implémenté en 0.60 ; les résultats de la source finale et preuves des paquets doivent être enregistrés séparément avant publication.

Les plans privés de projets réels sont reportés à la demande du propriétaire. Justesse topographique, pointage/impression physiques, lecteurs d'écran, matériel ARM64 natif, interopérabilité complète entre lecteurs, authenticité des mises à jour et autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Le corpus synthétique ne clôture pas ces validations. Aucun objectif de temps d'image ou de filtrage n'est annoncé comme atteint sans mesure enregistrée.

Un lot est limité à 512 pages, 4096 mesures choisies et un document de 128 Mio ; une page à 64 zones. L'accrochage utilise les véritables extrémités des courbes et les intersections de segments droits ; intersections courbes et découpes non convexes/courbes incompatibles sont exclues. Les voisinages trop denses peuvent refuser l'accrochage. Une échelle de page/zone ne recalcule jamais silencieusement les annotations. Dessin dimensionné, volumes, déductions et reconnaissance automatique restent des lots distincts.

Les paquets Windows restent non signés ; ARM64 est compilé en croisé. Ubuntu 22.04/24.04 amd64 conserve ses [limites fonctionnelles documentées](UBUNTU.fr.md). La mise à jour de Projects reste reportée.
