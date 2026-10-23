# SuniPDF 0.60.0 — Espace de travail des plans et mesures calibrées

[English](RELEASE_0.60.0.md) · [Parcours et limites](PLAN_WORKSPACE.fr.md)

[0.60.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.60.0) le 2026-10-23, sous `lmdanavanem`.

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

Les nouveaux cas automatisés utilisent des PDF synthétiques à dimensions connues et le corpus de régression public/sous licence conservé. Ils couvrent calculs, rognage/rotation, transformations imbriquées, découpes incompatibles, enregistrements attendus, réouverture, refus atomiques, Annuler et chiffrement. Il s'agit du périmètre produit P1 implémenté en 0.60 ; les résultats propres à la source et les preuves des paquets sont enregistrés séparément.

Les plans privés de projets réels sont reportés à la demande du propriétaire. Justesse topographique, pointage/impression physiques, lecteurs d'écran, matériel ARM64 natif, interopérabilité complète entre lecteurs, authenticité des mises à jour et autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Le corpus synthétique ne clôture pas ces validations. Aucun objectif de temps d'image ou de filtrage n'est annoncé comme atteint sans mesure enregistrée.

Un lot est limité à 512 pages, 4096 mesures choisies et un document de 128 Mio ; une page à 64 zones. L'accrochage utilise les véritables extrémités des courbes et les intersections de segments droits ; intersections courbes et découpes non convexes/courbes incompatibles sont exclues. Les voisinages trop denses peuvent refuser l'accrochage. Une échelle de page/zone ne recalcule jamais silencieusement les annotations. Dessin dimensionné, volumes, déductions et reconnaissance automatique restent des lots distincts.

Les paquets Windows restent non signés ; ARM64 est compilé en croisé. Ubuntu 22.04/24.04 amd64 conserve ses [limites fonctionnelles documentées](UBUNTU.fr.md). La mise à jour de Projects reste reportée.

Les nouvelles échelles de page utilisent la garde version 2 incluant les ressources. Une ancienne échelle version 1 sur une page dépendant de ressources exige recalibration ou retrait explicite : son ancienne empreinte ne permettait pas d'établir si une ressource avait changé. Les zones périmées reconnues peuvent être retirées sous contrôle de l'état capturé, puis recréées ; les données inconnues ou malformées restent préservées. La CLI fournit `clear-measurement-viewports` vers une nouvelle copie.

## Qualification observée

Source `90691d0bca160a9bfc22752dc2cf4269c5e7b1e4`, PR [#33](https://github.com/lmdanavanem/SuniPDF/pull/33). Source exacte validée : [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004268835). Windows : 827 tests réussis, 0 échec, 14 ignorés. Linux main : 808 réussis, 0 échec, 9 ignorés ; paquet Linux : 808 réussis, 0 échec, 9 ignorés. 1469 clés EN/FR. 44 scénarios portables ; 38 références MSI / 190 journaux. Huit fichiers téléchargés publiquement sans authentification et vérifiés par SHA256. Aucune alerte Code Scanning ouverte.

| Exécution | Opération | Charge | Échantillons | p95 observé (µs) |
|---|---|---|---:|---:|
| [Windows main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) | Disposition en cache | 5000 pages | 1000 | 0 |
| [Windows main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269668) | Filtrage du registre | 10000 annotations | 100 | 12534 |
| [Linux main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) | Disposition en cache | 5000 pages | 1000 | 0 |
| [Linux main](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004269704) | Filtrage du registre | 10000 annotations | 100 | 10608 |
| [Linux release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) | Disposition en cache | 5000 pages | 1000 | 0 |
| [Linux release](https://github.com/lmdanavanem/SuniPDF/actions/runs/40004284990) | Filtrage du registre | 10000 annotations | 100 | 6399 |

Les contrôles portent sur des opérations bornées, une géométrie synthétique connue et le corpus public/sous licence conservé. Les plans privés réels restent reportés à la demande du propriétaire. Les p95 ci-dessous mesurent des opérations de modèle en cache dans des binaires de test debug sur les runners CI ; ni le temps complet d'une image, ni le matériel physique, ni un objectif universel de vitesse ne sont établis. Une valeur enregistrée à 0 µs signifie une durée mesurée inférieure à la précision de restitution de 1 µs, et non une durée nulle : la conversion en microsecondes entières tronque la fraction. Aucune médiane n'est enregistrée. Les preuves d'interface utilisent de vraies tuiles PDFium avec DX12 WARP hors écran sous Windows et les paquets Ubuntu installés avec Xvfb/Glow logiciel. Ubuntu 22.04/24.04 couvre la suppression/réinstallation du même candidat, pas une migration depuis une ancienne version. Les paquets Windows restent non signés ; ARM64 est compilé sans exécution native. Impression physique, accessibilité, interopérabilité complète, authenticité des mises à jour et autres validations 1.0 restent ouvertes. Projects reste reporté.

Les cinq scènes Windows rendent la vraie interface et les tuiles PDFium, dont le registre ancré et la table de quantités en 720 × 480 avec première ligne visible. [Preuves](qa/RELEASE_0.60.0.json).

## Fichiers publiés

| Fichier | Octets | SHA256 |
|---|---:|---|
| [SuniPDF-0.60.0-linux-amd64.deb](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-linux-amd64.deb) | 17505156 | `5a7b5c8df7564230144c0bb27e5d14377b1c42e5aed4d32fe687d5bcf07d9492` |
| [SuniPDF-0.60.0-linux-amd64.deb.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-linux-amd64.deb.sha256) | 97 | `faa8e0219b27f0e26a36b2d8f67b0868a541fc1050341efd1fe201976086266b` |
| [SuniPDF-0.60.0-win-arm64-portable.zip](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-arm64-portable.zip) | 27893500 | `005747ba3fef243e943694a991b2a13af08fb7346f087d8aee3b75e9ebbaa310` |
| [SuniPDF-0.60.0-win-arm64-portable.zip.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-arm64-portable.zip.sha256) | 105 | `02e5dcbd98a915448ae02a53007045e04215eeac48489d7b9ceefea74890e6ed` |
| [SuniPDF-0.60.0-win-x64-portable.zip](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64-portable.zip) | 29647182 | `713cb24f5c06320a068be1c4a15f64d7e94ca051355ce98fe57fbee4be369598` |
| [SuniPDF-0.60.0-win-x64-portable.zip.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64-portable.zip.sha256) | 103 | `951cdee87da2537057f703c7e4ff3ded9b7b6b7210781a0460efe22c60fe6ff4` |
| [SuniPDF-0.60.0-win-x64.msi](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64.msi) | 27525120 | `7bc83b5cae907b178ddf09422fd2a301bbbebdd2bd09898b47cf2f3248190650` |
| [SuniPDF-0.60.0-win-x64.msi.sha256](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.60.0/SuniPDF-0.60.0-win-x64.msi.sha256) | 94 | `52e7a4a1d6ecde021de7a35700a370b7930d21056e5b8eb75668248c5f71a5d3` |
