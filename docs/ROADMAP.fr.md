# Roadmap SuniPDF

[English](ROADMAP.md)

Cette roadmap distingue les fonctions livrées et le travail futur.
Les contrôles exécutés et preuves de publication sont dans [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md).

La préparation acceptée de la 1.0 dispose de [critères explicites](RELEASE_1.0_CRITERIA.fr.md).
Les contrôles partiels du corpus et des mutations ne clôturent pas les P1 natifs, d’interopérabilité,
d’authenticité ou de conformité.

## 0.44–0.48 : fonctions locales pour les plans, séquence prévue

| Version | Périmètre | État actuel |
|---|---|---|
| 0.44 | Modifier une mesure enregistrée ; corriger calibration implicite, retour au défaut, allocations de l'inspecteur, droits de copie, redimensionnement et cibles externes/périmées. | Publiée ; [Preuves](qa/RELEASE_0.44.0.json). |
| 0.45 | Bibliothèques locales d’annotations | Lot livré dans ses limites ; critères complets encore ouverts. [0.45.0](RELEASE_0.45.0.fr.md). |
| 0.46 | Tables de quantités et champs typés | Lot livré dans ses limites ; critères complets encore ouverts. [0.46.0](RELEASE_0.46.0.fr.md). |
| 0.47 | Superpositions de révisions alignées | Lot livré dans ses limites ; critères complets encore ouverts. [0.47.0](RELEASE_0.47.0.fr.md). |
| 0.48 | Modification des sommets des mesures enregistrées | Lot livré dans ses limites ; critères complets encore ouverts. [0.48.0](RELEASE_0.48.0.fr.md). |

L'éditeur 0.44 modifie distance de référence, unité et précision en conservant sommets et points de référence. La CLI peut remplacer explicitement la géométrie avec un enregistrement attendu ; les deux parcours conservent l'identité et refusent les cibles périmées ou incompatibles. Qualification de la source finale et limites : [Preuves](qa/RELEASE_0.44.0.json). [Périmètre et audit préalable 0.44](RELEASE_0.44.0.fr.md) · [Guide des mesures](MEASUREMENTS.fr.md).

Cette séquence apporte des incréments bornés. La modification des sommets existants est incluse en 0.48 ; ajout/suppression, accrochage vectoriel, recalcul par lots, vues multi-échelles, justesse sur plans réels et critères généraux 1.0 restent ouverts. AEC-001/002/003/004/026 restent partiels. Les preuves historiques ci-dessous restent attachées à leur version d’origine.

## 0.48.0 : modification des sommets des mesures enregistrées (Latest stable)

[0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0) — Modifiez les sommets existants d’une mesure enregistrée sur sa page d’origine, examinez l’aperçu, ajustez le brouillon puis appliquez avec une seule étape d’annulation. La mesure conserve ses propres calibration, unité, précision, identité, réponses et champs de quantité saisis. Valeurs mesurées et totaux tarifaires compatibles suivent la géométrie validée ; les modifications périmées, invalides ou interdites restent contrôlées.

[Guide et limites](MEASUREMENT_GEOMETRY.fr.md) · [Notes de version](RELEASE_0.48.0.fr.md) · [Preuves](qa/RELEASE_0.48.0.json).

## 0.47.0 : superpositions de révisions alignées (publiée ; Latest à cette date)

[0.47.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.47.0) — Superposez deux pages PDF enregistrées en magenta/cyan, alignez-les par translation, rotation horaire et échelle explicites ou par deux paires de points correspondants, puis exportez un nouveau PNG ou rapport JSON lié aux sources. Les deux fichiers doivent autoriser la copie ; empreintes exactes, géométrie recadrée/pivotée, rasters bornés et sorties nouvelles préservent les originaux.

[Guide et limites](REVISION_OVERLAY.fr.md) · [Notes de version](RELEASE_0.47.0.fr.md) · [Preuves](qa/RELEASE_0.47.0.json).

## 0.46.0 : tables de quantités et champs typés (publiée ; Latest à cette date)

[0.46.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.46.0) — Consultez les comptages et mesures réels dans des tables filtrées, modifiez champs typés et prix unitaires explicites avec Annuler, puis exportez CSV/JSON liés à la source. Les totaux distinguent absence et zéro, convertissent les bases de prix compatibles et séparent les monnaies. La CLI exige un enregistrement attendu et une nouvelle copie PDF ; droits de copie et d’annotation restent contrôlés.

[Guide et limites](QUANTITY_TABLE.fr.md) · [Notes de version](RELEASE_0.46.0.fr.md) · [Preuves](qa/RELEASE_0.46.0.json).

## 0.45.0 : bibliothèques locales d’annotations (publiée ; Latest à cette date)

[0.45.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.45.0) — Enregistrez des brouillons d’annotations pris en charge comme modèles nommés, regroupez jusqu’à 16 primitives puis placez un aperçu à une origine et une échelle explicites, avec une seule étape Annuler et de nouvelles identités. Le JSON autonome strict permet import, export et suppression ; la CLI écrit une nouvelle copie PDF en conservant chiffrement et permissions.

[Guide et limites](ANNOTATION_LIBRARY.fr.md) · [Notes de version](RELEASE_0.45.0.fr.md) · [Preuves](qa/RELEASE_0.45.0.json).

## 0.44.0 : modification des mesures enregistrées (publiée ; Latest à cette date)

[0.44.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.44.0) — Modifiez distance de référence, unité et précision d’une mesure enregistrée avec Aperçu puis Appliquer ; la CLI remplace aussi explicitement la géométrie avec un enregistrement attendu. Identité, réponses, échelles de page et autres mesures sont conservées. Les échelles temporaires peuvent être retirées, l’inspecteur partage ses données, les exports contrôlent le droit de copie et les redimensionnements incompatibles sont refusés.

[Guide et limites](MEASUREMENTS.fr.md) · [Notes de version](RELEASE_0.44.0.fr.md) · [Preuves](qa/RELEASE_0.44.0.json).

## 0.43.0 : échelles de page enregistrées et inspection des mesures (publiée ; Latest à cette date)

[0.43.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.43.0) — Enregistrez ou retirez explicitement une échelle de page dans le PDF avec Annuler/Rétablir, consultez les mesures de la page et réutilisez l’échelle physique d’une mesure non angulaire. Calibration de session et valeurs des annotations existantes restent distinctes. La CLI lit, écrit et retire les échelles dans de nouvelles copies et convertit les unités lors de leur utilisation.

[Guide et limites](MEASUREMENTS.fr.md) · [Notes de version](RELEASE_0.43.0.fr.md) · [Preuves](qa/RELEASE_0.43.0.json).

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

## 0.30.1 : correctif de sécurité (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) sous `lmdanavanem`, release `417149160`, source immuable `697a963c756b316e1eb1e678298886bc08521da1`.

Nettoyage XFA/Screen et des textes d'accessibilité, autorisation propriétaire avant reprotection, signatures renforcées, mots de passe CLI par environnement/entrée standard et gardes Office/runtime renforcées. Les P1 Roadmap et Suggestions non traités restent différés. [Détails](RELEASE_0.30.1.fr.md).

Le [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) et la [CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) réussissent sur cette source : **575 tests**, zéro échec, 9 ignorés par défaut ; **1060 clés EN/FR**, **84 contrôles qpdf du corpus** et **9 sorties de fonctions indépendantes**, **4 cas de polices**, rendu logiciel DX12, **14 allers-retours Office**, 4 PDF sous licence/744 pages, **22 scénarios portables**, **16 références MSI/80 journaux de transactions**. CodeQL Rust, Python et Actions réussissent sans alerte ouverte.

Les six fichiers publics et les copies `dist/` correspondent aux paquets inspectés et aux SHA256. Huit régressions de sécurité passent aussi avec la CLI publiée. Le code du metteur à jour a vérifié le ZIP x64 de **27,029,229 octets** en **1.21 s**, sans installation sur ce poste, avec le harnais déjà compilé et le module inchangé.

Les mises à jour dépendent encore de GitHub HTTPS/SHA256 du même canal : aucune clé de publication indépendante n'est disponible. Les paquets Windows restent non signés ; le matériel ARM64 et les parcours manuels restent à qualifier. [Preuves et limites](qa/RELEASE_0.30.1.json).

## 0.30.0 : édition, protection des documents et conversion locale (publiée ; Latest à cette date)

Publiée (Latest à cette date) en [0.30.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) sous `lmdanavanem`, release `415568680`, source immuable `4db31c0ee62cfae9d65239edf1eac93db147841e`. Les cinq Bugs et quatorze P2 Roadmap sont livrés dans leurs [limites documentées](RELEASE_0.30.0.fr.md). P1 Roadmap et Suggestions restent différés.

[Workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) et [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) réussis sur cette source : **551 tests**, zéro échec, 9 ignorés par défaut ; **1047 clés EN/FR**, **84 contrôles qpdf** plus **9 sorties de fonctions indépendantes**, **4 cas de polices MuPDF**, DX12 logiciel WARP, **14 allers-retours Office**, 4 PDF licenciés/744 pages, **21 scénarios portables**, **15 bases MSI/75 journaux**. CodeQL Rust, Python et Actions réussis ; zéro alerte ouverte.

Six fichiers publics vérifiés octet par octet avec les paquets préparés et leurs SHA256. Le metteur à jour de production a vérifié le ZIP x64 de 26,978,019 octets en 2.55 s, sans installation sur cet hôte, au moyen du banc de test déjà compilé dont le module de mise à jour est inchangé. Projet relu : **42 éléments uniques, 20 Todo, 0 In Progress, 22 Done**, trois filtres enregistrés et aucune modification non enregistrée. [Preuves et limites](PROJECT_STATUS.fr.md).

Les paquets restent non signés. ARM64 est compilé et empaqueté, sans exécution sur matériel ARM64. Imprimantes physiques, Narrateur, interactions manuelles de mise à jour et GPU/fenêtres physiques restent à qualifier ; WARP prouve uniquement le rendu logiciel hors écran. Les entrées non prises en charge restent refusées. La conservation PDF/A ne certifie pas PDF/A/PDF/UA.

## 0.21.1 : conservation des documents (publiée ; Latest à cette date)

Les deux correctifs P1 sont livrés dans la [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) : refus du texte incompatible
sans perdre le bon brouillon, et conservation des annotations non-widget sans apparence utilisable à
l'aplatissement. 476 tests, 84 contrôles qpdf, 20 scénarios portables et 14 bases MSI réussis ; six fichiers
publics et mise à jour en ligne vérifiés. Voir l'[état du projet](PROJECT_STATUS.fr.md).

Le lot 0.30.0 prévu après 0.21.1 est publié ; les comptes actuels et P1 différés figurent ci-dessus.

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

## Phases 0–1 : planification et fondations — livrées

Workspace Rust, PDFium, erreurs typées, ouverture/navigation/zoom/fermeture, mots de passe, glisser-déposer,
localisation FR/EN, paramètres persistés, thèmes clair/sombre, registre de commandes et ressources Windows sont en place.
Les jalons et décisions historiques restent dans `DECISIONS.fr.md` et `PROJECT_STATUS.fr.md`.

## Phase 2 : lecture et opérations PDF essentielles — largement livrée

Livrés : onglets, caches de rendu/miniatures bornés, vues continue/double page/Livre, sélection logique entre pages,
recherche, signets, fusion/division/extraction/rotation/réorganisation, enregistrement atomique, annulation/rétablissement et annotations.

Livrés en 0.10.0 : rotation de vue, plein écran et historique borné par onglet.
Livrés en 0.11.0 : lecture des étiquettes et navigation interne précise. Création/réinitialisation et navigation
conservée par Annuler/Rétablir livrées en 0.14.0. La version 0.30.0 ajoute le recalage sémantique des étiquettes,
l'édition des signets et la conservation bornée des destinations, champs, calques et pièces jointes pendant
les opérations de pages ; voir les [structures prises en charge](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md).
contrôles d’exécution et indépendants documentés réussis. L'ordre des colonnes complexes non balisées reste
distinct ; qpdf seul ne prouve pas l'apparence.

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
La version 0.30.0 ajoute les [sous-ensembles de polices et faces statiques TrueType/CFF1 de collections](FONT_EMBEDDING.fr.md),
l'aperçu/réagencement borné des paragraphes, le collage éditable explicite, les pivots stables, la lecture de
l'opacité des images, la mise à l'échelle des annotations, les [annotations enrichies](RICH_ANNOTATIONS.fr.md)
et la création de destinations précises. contrôles de la source publiée réussis. Polices variables/CFF2/couleur, composition
complexe/bidirectionnelle, XObjects arbitraires éditables et styles par portion de texte restent non pris en charge.
Critère : rendu, enregistrement et réouverture couverts pour chaque opération, avec contrôles d’accessibilité applicables.

## Phase 4 : conversion et OCR — partiellement livrée

Livrés : images ↔ PDF, PDF → texte/PNG, optimisation sans perte, lots CLI et OCR anglais/français local embarqué.

Livré en 0.17.0 : huit orientations JPEG EXIF incorporées directement et estimation raster cumulée ;
mesure des limites de mémoire native/processus distincte (D108).

La version 0.30.0 ajoute la [conversion Office locale et les profils Standard/Impression mémorisés](OFFICE_CONVERSION.fr.md) :
LibreOffice installé et pris en charge, texte simple éditable DOCX, grilles XLSX de chaînes littérales et pages PPTX
tramées. L'absence du convertisseur est explicite ; l'application ne l'installe pas. Les exports éditables complexes
sont refusés. contrôles d’exécution et indépendants documentés réussis ; reconstruction arbitraire de la mise en page
Office et concepteur général de workflows restent futurs. Lots compression/OCR livrés en 0.10.0.

## Phase 5 : sécurité et productivité — partiellement livrée

Livrés : AES-128/256 et permissions, retrait de protection autorisé par le propriétaire, nettoyage des métadonnées,
caviardage irréversible, vérification CMS et copies signées PFX/P12 incrémentales, CLI JSON et automatisation.

La 0.31.0 implémente la comparaison locale bornée et la recherche dans des fichiers enregistrés sélectionnés ; [périmètre](LOCAL_REVIEW.fr.md).
La 0.36.0 ajoute les [raccourcis personnalisables](KEYBOARD_SHORTCUTS.fr.md). Restent : intégration à l’Explorateur et qualification native clavier/Narrateur.
La version 0.30.0 ajoute la [vérification Windows des chaînes/révocations et jetons RFC3161, ainsi que le stockage
DPAPI par utilisateur](SIGNATURE_SECURITY.fr.md), et l'[aperçu de caviardage, les marques persistantes, la découpe
bornée du texte et l'assainissement](REDACTION.fr.md). contrôles de la source publiée réussis. Chaque vérification réseau demande un
consentement explicite ; un résultat hors ligne/inconnu ne devient jamais fiable. Le DSS incorporé reste non vérifié,
sans validation LTV d'archivage. AATL/EUTL, PKCS#11, qualification des signatures et création d'horodatages restent
non pris en charge. L'heure PDF reste déclarative ; interopérabilité lecteurs PDF tiers distincte, non vérifiée.
La signature du code/MSI demande une identité appropriée fournie.

## Phase 6 : qualité et distribution — en cours

Base livrée : releases x64 portable/MSI et ARM64 portable, SHA256, notification au démarrage avec installation explicite,
guides FR/EN, notices tierces et workflows de compilation GitHub.

Livrés en 0.10.0 : quatre documents NIST publics et scénarios d’enregistrement interrompu/disque plein/échec.
La version 0.30.0 ajoute un inventaire hors ligne des licences et la compatibilité du moteur GPU facultatif ;
contrôles de release réussis. OpenGL reste le défaut. Restent : corpus client/hostile/volumineux plus large, fuzzing,
benchmarks avec matériel, maintenance de l'inventaire des licences, Narrateur et lecteurs tiers, parcours interactifs de mise à
jour MSI/portable, imprimantes physiques et exécution ARM64/OpenGL réelle.
La régression MSI ne couvre que son scénario scripté ; compiler un paquet ARM64 ne valide pas son exécution matérielle.

La [revue Code Scanning historique](CODE_SCANNING_REVIEW.fr.md) a examiné 117 alertes, dont 115 critiques,
sans confirmer de secret de production. Corrections/triage et permissions explicites des workflows font partie
du lot 0.21.x publié ; preuves de clôture dans l'[état du projet](PROJECT_STATUS.fr.md). La source exacte 0.30.0 réussit CodeQL Rust, Python et Actions, sans alerte ouverte ; preuves exécutées ci-dessus.

## Suite au-delà de la version 0.43.0

Priorités AEC locales : insertion/suppression de sommets, accrochage vectoriel, recalcul explicite par lots, vues multi-échelles, périmètre restant des bibliothèques et plans réels. Les 24 cartes conservent leurs critères ; les cartes partielles restent ouvertes.

1. Exécuter les dialogues natifs de mise à jour/enregistrement, Narrateur, imprimantes, ARM64 et lecteurs tiers.
2. Élargir corpus réel et mesures de performance ; les échantillons NIST ne prouvent pas une compatibilité universelle.
3. Maintenir les P1 Roadmap et Suggestions restants hors lot : validation PDF/A/PDF/UA dédiée, corpus/fuzzing/mesures
   matérielles élargis et intégrations demandent leur périmètre accepté. Étendre édition, fidélité Office et
   confiance d'archivage seulement au-delà des limites explicites de cette version.

Aucune date/version suivante n’est engagée.

## Fonctions locales pour les plans

Suivre le [backlog des fonctions locales](AEC_BACKLOG.fr.md) : mesures calibrées, quantités, tableaux, bibliothèques d’annotations et révisions des plans.
