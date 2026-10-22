# Prochaines tâches (ordre conseillé)

[English](NEXT_TASKS.md)

Le prochain lot de qualité suit les [critères d’acceptation de la 1.0](RELEASE_1.0_CRITERIA.fr.md).

## 0.48.0 : modification des sommets des mesures enregistrées (Latest stable)

[0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0) — Modifiez les sommets existants d’une mesure enregistrée sur sa page d’origine, examinez l’aperçu, ajustez le brouillon puis appliquez avec une seule étape d’annulation. La mesure conserve ses propres calibration, unité, précision, identité, réponses et champs de quantité saisis. Valeurs mesurées et totaux tarifaires compatibles suivent la géométrie validée ; les modifications périmées, invalides ou interdites restent contrôlées.

AEC-002 et AEC-003 restent partiels. Type de mesure et nombre de sommets restent fixes dans ce parcours. Ajout/suppression de sommets, accrochage vectoriel, recalcul par lots, vues multi-échelles et qualification sur plans réels restent ouverts. Les paquets restent non signés ; ARM64 natif, accessibilité, matériel et critères généraux 1.0 restent ouverts.

[Guide et limites](MEASUREMENT_GEOMETRY.fr.md) · [Notes de version](RELEASE_0.48.0.fr.md) · [Preuves](qa/RELEASE_0.48.0.json).

## 0.47.0 : superpositions de révisions alignées (publiée ; Latest à cette date)

[0.47.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.47.0) — Superposez deux pages PDF enregistrées en magenta/cyan, alignez-les par translation, rotation horaire et échelle explicites ou par deux paires de points correspondants, puis exportez un nouveau PNG ou rapport JSON lié aux sources. Les deux fichiers doivent autoriser la copie ; empreintes exactes, géométrie recadrée/pivotée, rasters bornés et sorties nouvelles préservent les originaux.

AEC-013 reste partiel. Recalage automatique, comparaison vectorielle, fusion PDF et contours automatiques des changements restent ouverts ; AEC-014 n’est pas livré. Cette superposition raster de deux pages sélectionnées est limitée au cadre de la base. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts.

[Guide et limites](REVISION_OVERLAY.fr.md) · [Notes de version](RELEASE_0.47.0.fr.md) · [Preuves](qa/RELEASE_0.47.0.json).

## 0.46.0 : tables de quantités et champs typés (publiée ; Latest à cette date)

[0.46.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.46.0) — Consultez les comptages et mesures réels dans des tables filtrées, modifiez champs typés et prix unitaires explicites avec Annuler, puis exportez CSV/JSON liés à la source. Les totaux distinguent absence et zéro, convertissent les bases de prix compatibles et séparent les monnaies. La CLI exige un enregistrement attendu et une nouvelle copie PDF ; droits de copie et d’annotation restent contrôlés.

Formules libres, taxes, conversion monétaire, reconnaissance automatique de symboles, agrégation entre documents et autres critères complets d’estimation restent ouverts. Les angles sont visibles, mais exclus des quantités additives et des prix. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts.

[Guide et limites](QUANTITY_TABLE.fr.md) · [Notes de version](RELEASE_0.46.0.fr.md) · [Preuves](qa/RELEASE_0.46.0.json).

## 0.45.0 : bibliothèques locales d’annotations (publiée ; Latest à cette date)

[0.45.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.45.0) — Enregistrez des brouillons d’annotations pris en charge comme modèles nommés, regroupez jusqu’à 16 primitives puis placez un aperçu à une origine et une échelle explicites, avec une seule étape Annuler et de nouvelles identités. Le JSON autonome strict permet import, export et suppression ; la CLI écrit une nouvelle copie PDF en conservant chiffrement et permissions.

La capture arbitraire d’annotations PDF existantes, les groupes imbriqués, le transfert de mesures/calibration et les autres critères complets de bibliothèque restent ouverts. Les paquets restent non signés ; ARM64 natif, accessibilité et qualification matérielle restent ouverts.

[Guide et limites](ANNOTATION_LIBRARY.fr.md) · [Notes de version](RELEASE_0.45.0.fr.md) · [Preuves](qa/RELEASE_0.45.0.json).

## 0.44.0 : modification des mesures enregistrées (publiée ; Latest à cette date)

Modifiez distance de référence, unité et précision d’une mesure enregistrée avec Aperçu puis Appliquer ; la CLI remplace aussi explicitement la géométrie avec un enregistrement attendu. Identité, réponses, échelles de page et autres mesures sont conservées. Les échelles temporaires peuvent être retirées, l’inspecteur partage ses données, les exports contrôlent le droit de copie et les redimensionnements incompatibles sont refusés.

Lot publié dans ses limites ; prochaine implémentation prévue : bibliothèque locale 0.45. [Preuves](qa/RELEASE_0.44.0.json).

### Séquence locale prévue

| Version | Résultat borné suivant | Preuves nécessaires |
|---|---|---|
| 0.44 | Modification individuelle et corrections livrées dans leurs limites. | Conserver originaux, géométrie fixe, identité, réponses et permissions ; vérifier annulation/historique et nouvelle copie. |
| 0.45 | Bibliothèques locales d’annotations | Lot livré dans ses limites ; critères complets encore ouverts. [0.45.0](RELEASE_0.45.0.fr.md). |
| 0.46 | Tables de quantités et champs typés | Lot livré dans ses limites ; critères complets encore ouverts. [0.46.0](RELEASE_0.46.0.fr.md). |
| 0.47 | Superpositions de révisions alignées | Lot livré dans ses limites ; critères complets encore ouverts. [0.47.0](RELEASE_0.47.0.fr.md). |
| 0.48 | Modification des sommets des mesures enregistrées | Lot livré dans ses limites ; critères complets encore ouverts. [0.48.0](RELEASE_0.48.0.fr.md). |

Les étapes futures exigent implémentation et qualification avant leur publication. AEC-001/002/003/004/026 restent partiels ; interaction native, vérification dimensionnelle sur plans réels et critères généraux 1.0 restent ouverts. [Roadmap](ROADMAP.fr.md).

## 0.43.0 : échelles de page enregistrées et inspection des mesures (publiée ; Latest à cette date)

[0.43.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.43.0) — Enregistrez ou retirez explicitement une échelle de page dans le PDF avec Annuler/Rétablir, consultez les mesures de la page et réutilisez l’échelle physique d’une mesure non angulaire. Calibration de session et valeurs des annotations existantes restent distinctes. La CLI lit, écrit et retire les échelles dans de nouvelles copies et convertit les unités lors de leur utilisation.

[Guide et limites](MEASUREMENTS.fr.md) · [Notes de version](RELEASE_0.43.0.fr.md) · [Preuves](qa/RELEASE_0.43.0.json).

### Prochain lot local

Le socle 0.43 alimente la séquence 0.44–0.48 ci-dessus. La modification des sommets existants est livrée dans les limites 0.48 ; ajout/suppression, accrochage, recalcul global, vues multi-échelles et qualification sur plans réels restent ouverts. AEC-001/002/003/004/026 restent partiels.

## 0.42.0 : mesures directement sur la page (publiée ; Latest à cette date)

[0.42.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.42.0) sous `lmdanavanem`, source `96ad768b24813e19c01de2881d3cb53c5b2fdc6c`.

Mesures dispose d’un bouton libellé et d’un espace dans Tous les outils. Calibrez avec deux clics et une distance confirmée, puis tracez longueurs, polylignes, périmètres, surfaces et angles sur le PDF. Aperçu et Appliquer contrôlent chaque insertion ; retrait de points, annulation et alignement horizontal/vertical accompagnent le tracé. Chaque page conserve son échelle de session entre modifications d’annotations et changements habituels d’onglet. [Guide et limites](MEASUREMENTS.fr.md).

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696093211) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696104747) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696092839). 662 tests réussis, 10 ignorés par défaut. 1224 clés EN/FR. 34 scénarios portables. 28 références MSI / 140 journaux. Aucune alerte Code Scanning ouverte. Six téléchargements publics sans authentification et copies dist vérifiés par SHA256 ; quatre appels CLI du paquet vérifient version, aide, création d’une longueur de 10 m et réouverture sans modifier l’original.

Les annotations enregistrées gardent leur propre échelle. Échelles communes persistantes, édition des sommets enregistrés, accrochage aux vecteurs, vues multi-échelles et recalcul global restent ouverts. Paquets non signés, ARM64 compilé uniquement ; qualification native/matérielle et critères 1.0 restent ouverts. Les mises à jour Projects restent différées. [Preuves](qa/RELEASE_0.42.0.json).

## 0.41.0 : comptage sur plans et tableau des annotations (publiée ; Latest à cette date)

[0.41.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) sous `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

Repères de comptage persistants placés par clic sur les plans PDF locaux, regroupement, déplacement/suppression avec Annuler, changement de groupe sans changer l’identité et export des totaux reliés aux repères. Le tableau de toutes les annotations ajoute recherche, filtres nommés, tri, navigation, statuts métier, disciplines et champs texte personnalisés, avec exports CSV/JSON protégés. Les changements refusent les cibles périmées et préservent originaux et permissions. Pillow 13.0.0 remplace la dépendance vulnérable réservée aux tests. Les paquets 0.40.0 rejoignent les migrations. Les écritures communes vérifient le droit PDF d’annotation pour déplacements, suppressions, réponses et autres modifications ; l’export refuse aussi les inventaires de pages incomplets et les identités de comptage malformées. [Utilisation et limites](MARKUP_REGISTER.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681919685) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681938796) réussies : **643 tests, zéro échec**, 10 ignorés par défaut, **1198 clés EN/FR**, **33 scénarios portables**, **27 références MSI/135 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **100 scénarios, 339 appels CLI et 14 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. 6 alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **27,323,650 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.41.0.json).

## 0.40.0 : mesures calibrées et tampons image (publiée ; Latest à cette date)

[0.40.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) sous `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

Annotations locales de longueur, polyligne, périmètre, surface polygonale simple et angle : calibration explicite sur deux points, unités métriques/impériales décimales, précision et données JSON. Chaque mesure conserve son échelle et sa géométrie ; aperçu, Annuler et copies CLI préservent les originaux. Tampons Stamp avec image intégrée, proportions et transparence. Filigranes texte/numérotation : vérification de toutes les pages et refus atomique des marges invalides ou débordements. Paquets 0.39.0 ajoutés aux migrations. Backlog AEC de 26 cartes donnant priorité au local. [Utilisation et limites](MEASUREMENTS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661526853) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661547803) réussies : **637 tests, zéro échec**, 10 ignorés par défaut, **1163 clés EN/FR**, **32 scénarios portables**, **26 références MSI/130 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **80 scénarios, 256 appels CLI et 10 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **27,121,645 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.40.0.json).

## 0.39.0 : modèles de décoration réutilisables (publiée ; Latest à cette date)

[0.39.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) sous `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

Modèles locaux nommés pour filigranes texte/image, numéros et six zones d’en-tête/pied. Charger un brouillon, enregistrer sous un nom unique ou supprimer explicitement. Écriture atomique, préférences préservées en cas d’échec et modèles invalides isolés. Portée des pages explicite et images référencées par chemin. Paquets 0.38.0 ajoutés aux migrations. Vérification de version Office sans interface avec une limite annulable de 30 secondes au démarrage à froid ; limites de conversion et gardes de sécurité conservées. [Utilisation et limites](PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404372880) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404384844) réussies : **628 tests, zéro échec**, 10 ignorés par défaut, **1149 clés EN/FR**, **31 scénarios portables**, **25 références MSI/125 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,879,736 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.39.0.json).

## 0.38.0 : six zones d’en-tête et pied de page (publiée ; Latest à cette date)

[0.38.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) sous `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six zones d’en-tête/pied via Numéros de page et la commande headers-footers. Numéro absolu, total, marges, taille et pages choisies respectent rotation et recadrage. Dépassement refusé atomiquement ; chiffrement autorisé et nouvelle copie en CLI conservés. Paquets 0.37.0 ajoutés aux migrations. [Utilisation et limites](PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) réussies : **623 tests, zéro échec**, 10 ignorés par défaut, **1143 clés EN/FR**, **30 scénarios portables**, **24 références MSI/120 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,823,494 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.38.0.json).

## 0.37.0 : filigranes image (publiée ; Latest à cette date)

[0.37.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) sous `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Filigranes image dans Pages et la CLI : transparence, largeur proportionnelle, angle, sept positions et marges. Ajustement aux pages tournées/recadrées. Modification atomique, une annulation dans l’application et nouvelle copie en CLI ; chiffrement autorisé conservé. Paquets 0.36.0 ajoutés aux migrations. [Utilisation et limites](PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) réussies : **618 tests, zéro échec**, 10 ignorés par défaut, **1134 clés EN/FR**, **29 scénarios portables**, **23 références MSI/115 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **24 scénarios, 113 appels CLI et 1 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. Deux alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,806,746 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.37.0.json).

## 0.36.0 : raccourcis personnalisables (publiée ; Latest à cette date)

[0.36.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) sous `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Raccourcis clavier personnalisables dans Aide et la palette : filtre, attribution, refus des conflits, désactivation et restauration. Brouillon annulable, préférences atomiques et libellés cohérents. Saisie et touches réservées préservées ; paquets 0.35.0 ajoutés aux références de migration. [Utilisation et bornes](KEYBOARD_SHORTCUTS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) réussies : **613 tests, zéro échec**, 10 ignorés par défaut, **1128 clés EN/FR**, **28 scénarios portables**, **22 références MSI/110 journaux**. Sept nouveaux tests egui sans fenêtre contrôlent raccourcis, conflits, focus, affichage, annulation et compatibilité des préférences. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, corpus, mutations et Office conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les **20 scénarios d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé vérifié, a téléchargé et contrôlé les **26,788,530 octets** du ZIP x64 sans installation sur le poste.

Claviers Windows natifs et Narrateur restent à qualifier. Explorer reste à faire ; seules les commandes de la table existante sont personnalisables. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects reportées à la demande du propriétaire, sans synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.36.0.json).

## 0.35.0 : préparation des signatures visuelles (publiée ; Latest à cette date)

[0.35.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) sous `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Signatures visuelles : lissage facultatif du dessin, réduction des ombres du papier et commande PNG `prepare-signature-image`. Aperçu lié aux pixels préparés, conservation des points bruts et fichiers sources, limites explicites du dessin. Les paquets publiés de la 0.34.0 rejoignent les références de migration. [Utilisation et bornes](VISUAL_SIGNATURE_PREPARATION.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) réussies : **606 tests, zéro échec**, 10 ignorés par défaut, **1114 clés EN/FR**, **20 scénarios d’image**, **14 scénarios d’inspection PDF**, **27 scénarios portables**, **21 références MSI/105 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, PDF, mutations et Office restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les suites image et inspection ont aussi été rejouées sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé et vérifié, a téléchargé et contrôlé les **26,752,605 octets** du ZIP x64, sans installation sur le poste.

Signatures visuelles sans preuve d’identité ; le nettoyage peut supprimer l’encre pâle et doit être vérifié dans l’aperçu. Interaction native/tablette/Narrateur et photos clients encore à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Projects reste reporté à la demande du propriétaire, sans synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.35.0.json).

## 0.34.0 : inspection du PDF enregistré (publiée ; Latest à cette date)

[0.34.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) sous `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspection du PDF enregistré depuis Propriétés et la CLI JSON `inspect` : sept catégories de déclarations, SHA256, résultats partiels explicites, entrées chiffrées et refus des réponses périmées. La garde de version sauvegardée calcule les empreintes par blocs. [Utilisation et bornes](PDF_INSPECTION.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) réussies : **600 tests, zéro échec**, 10 ignorés par défaut, **1108 clés EN/FR**, **14 scénarios d’inspection empaquetée**, **26 scénarios portables**, **20 références MSI/100 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, corpus PDF, mutations et Office existants restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié, a téléchargé et contrôlé les **26,741,242 octets** du ZIP x64, sans installation sur le poste.

Inventaire structurel sans garantie d’innocuité ni validation des signatures/PDF/A/PDF/UA. Paquets non signés ; validations natives/matérielles et critères 1.0 encore ouverts. Projects reste reporté à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.34.0.json).

## 0.33.0 : préparation 1.0 et compatibilité PDF (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) sous `lmdanavanem`, release `417802490`, source immuable `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Préservation des signets PDF 2.0 importés contenant des tabulations/sauts de ligne, garde stricte des nouveaux titres, migration groupée egui/eframe 0.37 et correction de la position Précédent/Suivant. Sept exemples PDF Association sous licence et empreintes fixes, 64 mutations PDF/CMS bornées, contrôles négatifs et mesures par commande. La 0.32.0 devient une référence de migration. [Critères 1.0](RELEASE_1.0_CRITERIA.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) réussis : **592 tests**, zéro échec, 10 ignorés par défaut, **1092 clés EN/FR** ; 84 contrôles qpdf et 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, quatre documents NIST/744 pages ; **7 exemples PDF 2.0/8 pages/21 contrôles qpdf**, **64 mutations et 2 contrôles de garde**, **25 scénarios portables**, **19 références MSI/95 journaux**. La CLI empaquetée passe aussi les 7 exemples/21 contrôles qpdf, 11 scénarios d’ordre d’objets/3 contrôles qpdf, 11 d’analyse locale et 8 de sécurité. CodeQL Rust/Python/Actions réussit, sans alerte ouverte.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié par empreinte, a téléchargé et vérifié le ZIP x64 de **26,726,762 octets en 1.65 s**, sans installation.

La mise à jour Projects est reportée à la demande explicite du propriétaire. Le dernier contrôle public montre **43 cartes, 25 Done, 18 Todo** ; les deux P1 ont une livraison partielle documentée, sans clôture ni changement de statut annoncé.

Paquets Windows non signés ; aucune clé indépendante de publication. Les exemples pédagogiques et les mutations déterministes ne remplacent pas un corpus client/hostile, du fuzzing guidé ou des mesures matérielles. ARM64 expérimental ; validations natives, lecteurs tiers et conformité PDF/A/PDF/UA encore ouvertes. [Preuves](qa/RELEASE_0.33.0.json).

## 0.32.0 : ordre des objets (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) sous `lmdanavanem`, release `417623635`, source immuable `8c492db35f1712648d5472593146077230befb7c`.

Réorganisation d’un objet ou de la sélection par glisser-déposer dans le panneau Objets, trait d’insertion, contrôle de la page du cache et du contexte, Échap et une seule étape Annuler. Commande JSON `reorder-objects` avec destination finale explicite, ordre relatif, permissions, sauvegarde et mots de passe existants. [Périmètre et utilisation](OBJECT_ORDER.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) réussis : **588 tests**, zéro échec, 9 ignorés par défaut ; **1092 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **24 scénarios portables**, **18 références MSI/90 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte. La CLI empaquetée passe **11 scénarios d’ordre d’objets et 3 contrôles qpdf supplémentaires**, plus 11 scénarios d’analyse locale et 8 de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,119,935 octets en 1.75 s**, sans installation. Projects : **43 cartes, 25 Done, 18 Todo, zéro In Progress** ; Suggestion P3 livrée, critères conservés, description relue après rechargement et trois filtres enregistrés vérifiés.

Paquets Windows non signés ; aucune clé indépendante de publication. Les validations matérielles, natives/manuelles et lecteurs tiers restent séparées ; aucun certificat PDF/A/PDF/UA/LTV. [Preuves](qa/RELEASE_0.32.0.json).

## 0.31.0 : comparaison locale et recherche multi-PDF (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) sous `lmdanavanem`, release `417463135`, source immuable `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Comparaison locale de deux PDF enregistrés (texte, dimensions et aperçus), alignement physique explicite et refus des aperçus obsolètes. Recherche littérale dans plusieurs fichiers sélectionnés, extraits et navigation, progression, annulation et résultats partiels explicites. Commandes JSON `compare` et `search-files` ; interface EN/FR. [Périmètre et limites](LOCAL_REVIEW.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) réussis : **582 tests**, zéro échec, 9 ignorés par défaut ; **1091 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **23 scénarios portables**, **17 références MSI/85 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte après revue individuelle de quatre secrets de tests synthétiques.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; onze scénarios d’analyse locale et huit de sécurité réussissent avec la CLI empaquetée. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,090,504 octets en 1.71 s**, sans installation. Projects : **43 cartes, 24 Done, 19 Todo, zéro In Progress** ; les deux Suggestions P2 sont livrées, les critères conservés et les trois filtres relus.

Paquets Windows non signés ; aucune clé de publication indépendante. ARM64, matériel et parcours manuels restent à qualifier. La comparaison bornée ne prouve ni identité des PDF ni conformité. [Preuves](qa/RELEASE_0.31.0.json).

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

Le lot 0.30.0 prévu après 0.21.1 est publié ; les comptes actuels et P1 différés figurent ci-dessus.

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
et l'[état du projet](PROJECT_STATUS.fr.md). Le compte de 53 commandes décrit la base 0.21.x ;
la version 0.30.0 ajoute des commandes validées dans les preuves de publication.

## Travaux différés après 0.30.0

1. Qualifier le matériel ARM64, les imprimantes, Narrateur, les GPU/fenêtres et les interactions natives de mise à jour.
2. Étendre les corpus licenciés, les formes complexes, le fuzzing et les mesures de performances avec matériel documenté.
3. Étendre la fidélité Office, la composition complexe et le texte bidirectionnel au-delà des entrées actuellement prises en charge.
4. Qualifier PDF/A/PDF/UA, la confiance d’archivage, les lecteurs tiers et les intégrations selon un périmètre accepté.

Les cinq Bugs et quatorze P2 Roadmap sont livrés dans la version publiée 0.30.0. P1 Roadmap et Suggestions restent différés ; aucune date ni version future n’est engagée.

## Actions nécessitant l'utilisateur
1. Valider l'interface sur de vrais PDF (accueil, outils, rail droit, palette Ctrl+K, thème sombre, FR/EN) et signaler les écarts.
2. Exécuter le ZIP ARM64 sur une machine ARM64 réelle (jamais fait), vérifier OpenGL et qualifier wgpu séparément si choisi.
3. Vérifier l'interopérabilité des annotations et du caviardage dans lecteur PDF tiers, lecteur PDF tiers et Chrome.
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

## Après redaction
La version 0.30.0 implémente l'aperçu natif avant/après, remplissage et texte de remplacement borné, marques
`/Redact` non appliquées persistantes, découpe conservatrice des glyphes Base14 latins et assainissement partagé.
Voir les [garanties et limites](REDACTION.fr.md) et le [modèle de sécurité](SECURITY_MODEL.fr.md) ; contrôles de la source publiée réussis.
Enregistrer des marques ne supprime pas le contenu. Polices complexes, OCR, images et graphiques imbriqués gardent
le repli par image ; Annuler, originaux et sauvegardes demandent des précautions distinctes.
Restent : corpus hostile/volumineux/balisé/OCR/Type3 licencié élargi, mémoire/performance natives mesurées et
contrôles manuels lecteurs/UI. Ces P1 sont différés ; qpdf/lecteur synthétique ne remplace pas ce corpus.

## Après forms (formulaires AcroForm, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A38, décisions D061–D066) :
- échanges JSON/XFDF/FDF et panneau Appliquer groupé implémentés en 0.10.0 ; validation dans `PROJECT_STATUS.fr.md` ;
- Texte libre livré en 0.11.0 ; création de listes modifiables et correspondances radio `/Opt` livrées en 0.12.0.
  Activation clavier livrée en 0.10.0 ; Narrateur Windows non vérifié
  (noms `labelled_by` posés, non écoutés).
- aplatissement : conservation des annotations non-Link sans apparence utilisable, pièces jointes et
  références partagées vérifiée localement et par GitHub CI/CodeQL sur sa source d'implémentation.
  Livré en 0.21.1 ; contrôles de release et paquets publics réussis ;
  [contrôles exécutés et limites](PROJECT_STATUS.fr.md).

## Après sign-image (signature visuelle, tranche livrée)

Signatures saisies et pose sur toutes les pages/la sélection livrées en 0.13.0 (D102–D103).

Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A35–A37, décisions D054–D060) :
- La version 0.30.0 implémente le stockage DPAPI facultatif par utilisateur, la migration vérifiée/atomique
  des PNG en clair, refus d'enregistrement/suppression et gardes profil/ACL ; contrôles de la source publiée réussis.
  Limites de récupération et d'accès à la session dans la [sécurité des signatures](SIGNATURE_SECURITY.fr.md).
- **Livré en 0.35.0** : lissage facultatif par courbes quadratiques, nettoyage adaptatif des ombres du papier, aperçu lié aux pixels préparés et commande PNG. [Utilisation et bornes](VISUAL_SIGNATURE_PREPARATION.fr.md).
- Restent : pression du stylet, qualification native souris/tablette/Narrateur et photos réelles de clients ; le nettoyage peut supprimer l’encre pâle.

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
La version 0.30.0 ajoute les [sous-ensembles/faces statiques de collections](FONT_EMBEDDING.fr.md), découverte
Unicode des polices Windows, aperçu/réagencement borné des paragraphes, collage natif éditable explicite, pivots
mémorisés stables et lecture de l'opacité. Le collage groupé existant reste disponible. Contrôles de la source publiée réussis ; A7, A8, A18–A22.
Couleur/opacité des groupes livrée en 0.30.0. Glisser-déposer d’objets/sélections livré en 0.32.0 dans le [périmètre documenté](OBJECT_ORDER.fr.md).
Restent : polices variables/CFF2/couleur, composition complexe/bidirectionnelle et XObjects arbitraires éditables. Une opacité inconnue est explicite ; un cas non
pris en charge doit préserver le document, sans aplatissement ni substitution de police silencieux.
Acceptation : chaque opération couverte par un test moteur avec rendu vérifié.

## SUNI-021b — Annotations avancées : livré, reste (owner : PDF-EDITOR)
Livré (D067–D076) : encre, zone de texte libre, rectangle, ellipse, ligne, flèche, liens cliquables, sélection /
déplacement / redimensionnement / recoloration / suppression, fils de réponses et statuts, auteur mémorisé, CLI
`annotations` / `annotate` / `reply` / `review-state` / `annotation-delete`. Reste :
- **vérifier l'interopérabilité** des réponses et statuts, des apparences d'encre, de texte libre et de formes dans
  lecteur PDF tiers, lecteur PDF tiers et Chrome (non disponibles sur le poste de développement) ; un contrôle structurel ne prouve pas l’interopérabilité visuelle ;
- encre groupée livrée en 0.18.0 (D109). La version 0.30.0 ajoute gomme de traits entiers, polygones, polylignes,
  nuages, tampons standard, pièces jointes et FreeText à style uniforme/police incorporée avec aperçu réel ;
  [bornes et refus](RICH_ANNOTATIONS.fr.md), contrôles de la source publiée réussis. Styles par portion, `/RC` arbitraire et composition complexe restent exclus ;
- création de destinations nommées/XYZ/Fit/FitH présente dans la version 0.30.0, axes/zoom conservés et résolution
  brute bornée ; contrôles de la source publiée réussis. Les actions actives non prises en charge ne sont jamais exécutées ;
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
- La version 0.30.0 rétablit la compatibilité wgpu facultative et ajoute un contrôle DX12 logiciel ;
  contrôles de la source publiée réussis. OpenGL reste le défaut ; GPU/fenêtres réels et matériel ARM64 non vérifiés.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.

## Fonctions locales pour les plans

Suivre le [backlog des fonctions locales](AEC_BACKLOG.fr.md) : mesures calibrées, quantités, tableaux, bibliothèques d’annotations et révisions des plans.
