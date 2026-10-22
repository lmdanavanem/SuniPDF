# Problèmes connus

[English](KNOWN_ISSUES.md)

## 0.42.0 : mesures directement sur la page (Latest stable)

[0.42.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.42.0) sous `lmdanavanem`, source `96ad768b24813e19c01de2881d3cb53c5b2fdc6c`.

Mesures dispose d’un bouton libellé et d’un espace dans Tous les outils. Calibrez avec deux clics et une distance confirmée, puis tracez longueurs, polylignes, périmètres, surfaces et angles sur le PDF. Aperçu et Appliquer contrôlent chaque insertion ; retrait de points, annulation et alignement horizontal/vertical accompagnent le tracé. Chaque page conserve son échelle de session entre modifications d’annotations et changements habituels d’onglet. [Guide et limites](MEASUREMENTS.fr.md).

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696093211) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696104747) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696092839). 662 tests réussis, 10 ignorés par défaut. 1224 clés EN/FR. 34 scénarios portables. 28 références MSI / 140 journaux. Aucune alerte Code Scanning ouverte. Six téléchargements publics sans authentification et copies dist vérifiés par SHA256 ; quatre appels CLI du paquet vérifient version, aide, création d’une longueur de 10 m et réouverture sans modifier l’original.

Les annotations enregistrées gardent leur propre échelle. Échelles communes persistantes, édition des sommets enregistrés, accrochage aux vecteurs, vues multi-échelles et recalcul global restent ouverts. Paquets non signés, ARM64 compilé uniquement ; qualification native/matérielle et critères 1.0 restent ouverts. Cloud et mises à jour Projects restent différés. [Preuves](qa/RELEASE_0.42.0.json).

## 0.41.0 : comptage sur plans et tableau des annotations (publiée ; Latest à cette date)

[0.41.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) sous `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

Repères de comptage persistants placés par clic sur les plans PDF locaux, regroupement, déplacement/suppression avec Annuler, changement de groupe sans changer l’identité et export des totaux reliés aux repères. Le tableau de toutes les annotations ajoute recherche, filtres nommés, tri, navigation, statuts métier, disciplines et champs texte personnalisés, avec exports CSV/JSON protégés. Les changements refusent les cibles périmées et préservent originaux et permissions. Pillow 13.0.0 remplace la dépendance vulnérable réservée aux tests. Les paquets 0.40.0 rejoignent les migrations ; cloud et collaboration restent reportés. Les écritures communes vérifient le droit PDF d’annotation pour déplacements, suppressions, réponses et autres modifications ; l’export refuse aussi les inventaires de pages incomplets et les identités de comptage malformées. [Utilisation et limites](MARKUP_REGISTER.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681919685) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681938796) réussies : **643 tests, zéro échec**, 10 ignorés par défaut, **1198 clés EN/FR**, **33 scénarios portables**, **27 références MSI/135 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **100 scénarios, 339 appels CLI et 14 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. 6 alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **27,323,650 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](qa/RELEASE_0.41.0.json).

## 0.40.0 : mesures calibrées et tampons image (publiée ; Latest à cette date)

[0.40.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) sous `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

Annotations locales de longueur, polyligne, périmètre, surface polygonale simple et angle : calibration explicite sur deux points, unités métriques/impériales décimales, précision et données JSON. Chaque mesure conserve son échelle et sa géométrie ; aperçu, Annuler et copies CLI préservent les originaux. Tampons Stamp avec image intégrée, proportions et transparence. Filigranes texte/numérotation : vérification de toutes les pages et refus atomique des marges invalides ou débordements. Paquets 0.39.0 ajoutés aux migrations. Backlog AEC de 26 cartes donnant priorité au local ; cloud et collaboration reportés à la demande du propriétaire, tarifs ultérieurs. [Utilisation et limites](MEASUREMENTS.fr.md).

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

## Environnement de build (ce poste)

1. ~~Pas de Build Tools~~ **Résolu le 2026-10-13** : Build Tools installés, override GNU retiré, tests verts en MSVC.
   Le repli GNU + WinLibs reste documenté dans `scripts/dev-env.ps1` pour un poste sans droits admin.
2. **Mémoire limitée (14 Go, ~1,5 Go libres pendant la session).** Avec 16 cœurs, `cargo` lance 16 `rustc`
   et le système atteint sa limite de commit : abandons `0xC0000409` et erreurs trompeuses
   (« only metadata stub found », ICE). Contournement : `cargo build -j 2` (ou `CARGO_BUILD_JOBS=2`)
   et `debug = "line-tables-only"` en profil dev.
3. (Repli GNU seulement) **Les binutils livrés par rustup ne suffisent pas** pour les crates `raw-dylib`
   (`dlltool` sans `as`) et le rustc GNU abandonne sous clippy sur le crate `windows` : WinLibs complet requis,
   clippy limité aux crates sans dépendance `windows`.

## Dépendances

4. **Compatibilité GPU facultative** : la version 0.30.0 aligne les dépendances GPU Windows et ajoute wgpu/DX12
   facultatif avec contrôle logiciel. contrôles de la source publiée réussis. glow/OpenGL reste le défaut/repli ; comportement réel
   GPU/fenêtres et matériel ARM64 non vérifiés. Voir le [périmètre](RELEASE_0.30.0.fr.md).
5. **pdfium-render 0.9.4 cible le build PDFium 7881** ; les builds plus récents (8100 au 2026-10-12)
   fonctionnent probablement mais ne sont pas testés. Version épinglée dans `scripts/fetch-pdfium.ps1`.

11. ~~MSI non construit~~ **Résolu le 2026-10-14** : binaires WiX 3.14 autonomes (`third_party/wix314`, zip officiel, sans
    installation ni droits admin) ; `scripts/package-msi.ps1` les détecte via `WIX_BIN` ; MSI x64 produit et validé
    (ICE `smoke.exe`, extraction administrative).
12. **Build ARM64 local bloqué par un composant Visual Studio manquant** (tenté le 2026-10-14) : la cible Rust
    `aarch64-pc-windows-msvc` est installée et le PDFium ARM64 se télécharge, mais l'édition de liens échoue car
    les Build Tools n'ont que les outils x64 et x86 (`VC\Tools\MSVC\<version>\bin\Hostx64\arm64` absent, CRT ARM64
    absente). Installation (droits administrateur requis, refusée pendant la session autonome) :
    `& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify --installPath
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools" --add Microsoft.VisualStudio.Component.VC.Tools.ARM64
    --passive --norestart`, puis `scripts\package-portable.ps1 -Arch arm64` (depuis Git Bash).
    Les runners GitHub `windows-latest` ont ce composant : `release.yml` construit l'ARM64 en CI.

## Application

**Inspection du PDF enregistré (0.34.0) :** inventaire borné incluant les objets inutilisés, sans examen
des contenus, vérification d’intégrité des signatures ni garantie d’innocuité/conformité. Les compteurs
partiels restent explicites. Enregistrer/recharger après changement de source ; interface native/Narrator
restent à qualifier séparément. [Utilisation et bornes](PDF_INSPECTION.fr.md).

A1. **Icônes Fluent** : lues dans `C:\Windows\Fonts\SegoeIcons.ttf` (ou `segmdl2.ttf`). Sur un Windows sans ces
    polices (cas rare : LTSC allégé), les icônes sont remplacées par des symboles texte, moins élégants mais lisibles.
A2. ~~Affichage page par page~~ **Résolu (SUNI-012b)** : défilement continu par défaut, page unique en option (F5).
A3. ~~Rendu à fort zoom limité à 16 384 px~~ **Résolu pour l'écran (SUNI-012b)** : rendu en tuiles jusqu'à
    3200 % (côté de page rendue <= 131 072 px). Corrige aussi un plantage latent : une page rendue d'un seul
    tenant pouvait dépasser la taille de texture maximale du GPU (panique d'`egui_glow` si le pilote limite à
    8192 px) ; aucune texture ne dépasse plus `max_texture_side`. La limite de 16 384 px reste vraie pour
    `Document::render_page`, l'export d'images et `sunipdf-cli render` sans `--region`.
A4. **Chiffrement et QA indépendante** : AES-128/256 est écrit par lopdf et relu par PDFium ; RC4 40 bits est couvert
    par fixture. En 0.9.1, le helper partagé écrit explicitement `/Encrypt /Length 256` pour AES-256, valeur omise
    auparavant et signalée par qpdf, y compris après rechiffrement. Des contrôles qpdf synthétiques et fixtures AES
    générées par qpdf sont ajoutés en CI. Résultats actuels dans `PROJECT_STATUS.fr.md` ; ce n'est pas un corpus réel/hostile.
A5. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
A6. **Édition du contenu : la modification d'un texte existant réutilise sa police d'origine.** Si cette police
    (souvent un sous-ensemble incorporé) ne contient pas un caractère saisi, PDFium le remplace ou l'omet.
    Contournement en 0.21.0 publiée : supprimer l'objet et en ajouter un nouveau (Helvetica, accents latins pris en charge).
    **Livré en 0.21.1 :** garder cette police, refuser atomiquement
    le texte incompatible et rouvrir le bon brouillon. Correctif absent des paquets 0.21.0 ; voir les [contrôles exécutés](PROJECT_STATUS.fr.md).
A7. **Rotation et pivot** : rotation libre/groupée antérieure conservée. La version 0.30.0 mémorise le pivot
    choisi pour éviter de recalculer le centre de la boîte englobante d'un tracé asymétrique à chaque rotation ;
    contrôles de la source publiée réussis. Les poignées de redimensionnement restent absentes pour les objets non orthogonaux afin
    d'éviter leur déformation. Voir le [périmètre](RELEASE_0.30.0.fr.md).
A8. **Ordre et presse-papiers** : l'ordre Z fusionne les flux de page avant réordonnancement. Duplication/collage
    groupés existants utilisent des XObjects formulaires, sans rendre leur contenu texte éditable ; couleur/opacité
    des groupes restent limitées. La version 0.30.0 ajoute **Coller éditable** explicite pour les objets natifs
    pris en charge, un Annuler et refus atomique des graphes dangereux, sans repli groupé silencieux. contrôles de la source publiée réussis ; [périmètre](RELEASE_0.30.0.fr.md). Presse-papiers de session et limites du collage groupé conservés.
A18. **Formats, droits et réutilisation des polices** : la version 0.30.0 remplace les noms OEM de `reg.exe`
     par la découverte Windows Unicode, faces statiques TrueType/CFF1 TTF/OTF/TTC/OTC choisies et sous-ensembles admissibles.
     Droits contrôlés avant mutation ; interdiction de sous-ensemble : face entière conservée. Les programmes
     identiques peuvent être réutilisés après Annuler/Rétablir/réouverture. Polices variables/CFF2/couleur, faces
     endommagées, composition non prise en charge et glyphes absents refusés. Limites taille/faces/glyphes applicables ;
     rendu interne distinct de validation d'autres lecteurs. contrôles d’exécution et du lecteur documenté réussis ; [politique](FONT_EMBEDDING.fr.md).
A19. **Recadrer masque, n'efface pas** : le contenu hors cadre reste dans le fichier (ce n'est pas un
     caviardage ; dit dans le dialogue). Recadrage souris dans l’aperçu implémenté en 0.10.0 ; marges numériques conservées.
A20. **Édition du texte et aperçu de paragraphe** : la saisie ordinaire utilise encore la police d'interface ;
     son brouillon n'est pas une preuve de mise en page PDF. La version 0.30.0 ajoute le réagencement multiligne
     borné avec métriques natives, Aperçu puis Appliquer explicite, brouillon conservé et un Annuler. Polices/texte/
     états graphiques non pris en charge refusés. Réagencement général et composition bidirectionnelle/complexe
     restent exclus ; contrôles de la source publiée réussis. [Périmètre](RELEASE_0.30.0.fr.md) et [limites des polices](FONT_EMBEDDING.fr.md).
A21. **Redimensionnement des pages** : la version 0.30.0 met facultativement les annotations/liens pris en
     charge à l'échelle avec le contenu. Géométrie malformée et apparences non prises en charge : refus atomique,
     sans décalage silencieux. Prise en charge bornée, sans promesse pour toute extension PDF ; contrôles de la source publiée réussis.
     [Périmètre](RELEASE_0.30.0.fr.md).
A22. **Opacité d'image** : la version 0.30.0 lit l'opacité existante prise en charge et affiche l'état inconnu
     au lieu de conserver la valeur de l'objet précédent. Remplacement explicite. Masques/états graphiques complexes
     ne sont pas une valeur alpha universelle éditable ; contrôles de la source publiée réussis. [Périmètre](RELEASE_0.30.0.fr.md).
A23. **Angle affiché des objets d'une page pivotée** : l'angle d'un objet est celui de son contenu à l'écran ;
     un rectangle dessiné dans le PDF sur une page `/Rotate 90` est donc annoncé à 90° (CLI `objects`, champ
     Angle). Les poignées d'angle restent disponibles (multiple de 90°).
A24. **Presse-papiers d'objets et presse-papiers système** : Ctrl+V ne colle les objets que si le repère écrit
     par la copie (texte des objets, ou « [SuniPDF : n objet(s) copié(s)] ») est encore dans le presse-papiers
     Windows ; le menu Édition, la palette et la barre d'objet collent sans cette condition. Le presse-papiers
     d'objets vit en mémoire le temps de la session (perdu à la fermeture de l'application).
A9. **Tests moteur sérialisés** : PDFium ne peut pas être réinitialisé dans un même processus après
    `FPDF_DestroyLibrary` ; les tests partagent un moteur unique jamais détruit (`tests/engine.rs`).
A10. **Validation de distribution** : l'état courant des tests, CI, paquets et releases est consigné dans
     `PROJECT_STATUS.fr.md`. La 0.10.0 publiée réussit huit scénarios portables réels de mise à jour/échec/restauration et
     deux bases MSI sur runners jetables : installation/rollback transactionnel/mise à niveau/refus de downgrade/désinstallation,
     avec conservation des données. Parcours interactifs, matériel ARM64 et imprimantes physiques restent distincts.
A11. **Hub « Tous les outils » : 14 outils implémentés en 0.9.0.** OCR, création de champs et certificat
     ont désormais de vraies commandes. Couverture automatisée et matérielle dans `PROJECT_STATUS.fr.md`.
     La 0.42.0 ajoute Mesures comme quinzième espace, avec un bouton libellé dans la barre principale.
A12. **Interface du hub vérifiée sans rendu au pixel automatisé** : pas de rendu logiciel ni d'`egui_kittest`
     dans les dépendances. Les tests couvrent les données (table, registre, contraste des glyphes), la bascule
     d'outil, la géométrie des lignes, l'arbre AccessKit et l'anneau de focus (passe egui sans fenêtre) ; le
     rendu réel est contrôlé par captures (`scripts/dev-screenshot.ps1`). Les glyphes de repli (sans Segoe
     Fluent Icons) des nouvelles icônes ne sont pas vérifiés visuellement (voir A1).
A13. **Barre d'outil en fenêtre étroite** : la largeur des libellés est estimée avant le dessin ; sous la
     largeur nécessaire, icônes seules, puis mode compact (nom et « Quitter l'outil » sans libellé) et
     défilement horizontal des boutons (barre de défilement flottante, au survol ; la molette verticale ne fait
     pas défiler, Maj+molette oui). **Défilement non démontré** : à 100 % d'échelle et à la largeur minimale
     de la fenêtre (720 px), la barre la plus longue (Modifier) tient en mode compact ; le défilement n'est
     atteint qu'avec une échelle d'affichage Windows plus grande et n'a été vérifié ni par un test ni par une
     capture.
     Le libellé Mesures reste visible ; les tests egui couvrent les fenêtres étroites, sans qualifier toutes les interactions Windows/DPI natives.
A14. **Clavier** : après une activation au clavier (Entrée) d'une ligne ou d'un bouton du hub ou de l'accès
     rapide (Ouvrir, Enregistrer, Annuler, Rétablir), le focus est relâché pour que les raccourcis à une
     lettre restent actifs ; Tab repart donc du début. Les boutons egui standard (barre d'onglets, menu)
     n'affichent qu'un indice de focus discret ; les lignes, cartes et boutons du hub ont un anneau de 2 px.
     Échap agit dans l'ordre (un Échap, un effet ; `workspace::EscState::action`, testé) : menu ou liste
     déroulante ouverts, champ ou recherche, objet sélectionné, outil actif. Deux Échap reçus dans la même
     image (impossible au clavier, possible par injection) peuvent encore produire deux effets.
A15. ~~Rail droit en fenêtre basse : chevauchement~~ **Corrigé (SUNI-012b)** : sous ~700 px de haut, le groupe du
     haut (panneaux, bascule de vue, recherche) défile (molette au survol) au lieu de chevaucher la navigation ;
     ses dernières icônes sont alors masquées tant qu'on ne fait pas défiler. Seules des icônes entières sont
     montrées et une fine barre de défilement, visible sans survol, signale la suite (capture 1000 x 560).
A16. ~~Aide CLI des variables de `stamp` et `page-numbers`~~ **Corrigé en 0.18.0** : le modèle d'aide
     clap existant conserve `{n}` / `{total}` littéraux, sans changer les commandes. Régression `-h` / `--help`
     des deux commandes et quatre contrôles d'aide préparés/publics réussis.

A17. ~~Indicateur « modifié » après annulation~~ **Corrigé (review-fixes, D052)** : point d'enregistrement dans
     l'historique ; perdu seulement si l'on modifie après une annulation ou si l'instantané enregistré sort des 50
     niveaux (le document reste alors « modifié » jusqu'au prochain enregistrement).
A29. ~~Extraction/division retirant la protection PDF~~ **Résolu en 0.11.0** :
     les copies conservent chiffrement, mots de passe et permissions d'origine via le writer protégé (D097 remplace
     D046). Sortie au chemin source résolu refusée. CLI `encryption_removed: false`, ajout `encrypted`.
     Politique des permissions inchangée ; validations courantes dans `PROJECT_STATUS.fr.md`.
A30b. **AES-256 et mots de passe** : le mot de passe est normalisé SASLprep avant chiffrement (espaces spéciaux
    ramenés à l'espace, ligatures décomposées), puis limité à un préfixe UTF-8 valide de 127 octets. SuniPDF
    applique la même transformation à l'ouverture ; un autre lecteur peut exiger cette forme effective
    (par exemple « pass word » pour une saisie avec espace insécable). Depuis 0.8.0, un mot de passe d'ouverture
    devenu vide est refusé ; les restrictions exigent des mots de passe effectifs distincts.
A30. **AES-128 et mots de passe** : seul le Latin-1 imprimable est accepté (0x20–0x7E, 0xA1–0xFF sauf 0xAD ;
     lettres accentuées comprises). Les symboles propres à PDFDocEncoding hors Latin-1 (€, œ, Œ, ligatures,
     tirets et guillemets typographiques…) sont refusés (`password_not_latin1`) bien que lopdf les code : PDFium
     reçoit le mot de passe en UTF-8 et ne réessaie qu'en Latin-1, SuniPDF ne rouvrirait donc pas le fichier
     (vérifié par `review_aes128_latin1_passwords_round_trip_through_pdfium`). Choisir AES-256 pour ces
     mots de passe. AES-128 utilise les 32 premiers octets encodés, complétés selon le format PDF ; depuis 0.8.0,
     la différence propriétaire/utilisateur est vérifiée sur ces octets effectifs.
A31. **Images → PDF** : la 0.17.0 publiée incorpore les JPEG des huit orientations EXIF directement sans recompression
     et contrôle avant allocation une estimation raster non-JPEG cumulée de 512 Mio :
     `somme(max(decoder.total_bytes(), largeur × hauteur × 4))`. Arithmétique vérifiée, dimensions physiques finies/
     positives, contrôles répétés au chargement pour les changements simples de fichiers (D108). Workspace local,
     qpdf/corpus, conversion indépendante debug/paquets préparés/publics, CI Windows, compilation ARM64 de même
     source, mises à niveau et metteur à jour de production réussis ; preuves et limites dans `PROJECT_STATUS.fr.md`.
     PNG, TIFF, BMP, GIF et WebP restent décodés avec qualité/transparence conservées, puis stockés en pixels
     comprimés (Flate) ; le PDF peut être plus lourd que l'original. Estimation distincte d'un plafond RSS natif/
     processus ou d'une limite JPEG comprimé ; coût du décodeur/de PDFium/de sortie et mémoire du poste restent distincts.
     Un JPEG CMJN incorporé tel quel est converti en RVB par PDFium au rendu (rouge pur → 238,29,35) :
     conversion normale d'un flux DCT, pas d'inversion.
A32. **Texte ajouté** : une tabulation vaut quatre espaces (aucune police PDF n'a de glyphe de tabulation).
     Le panneau Commentaires nomme 17 sous-types d'annotation ; les autres (son, vidéo, 3D, filigrane,
     caviardage…) s'affichent « Autre annotation ».
A33. **Point d'enregistrement** : « modifié » s'efface en annulant jusqu'à l'état enregistré, sauf si une
     modification a suivi une annulation (l'état enregistré est alors inatteignable) ou si l'instantané
     enregistré est sorti des 50 niveaux d'historique (D052).
A34. **Propriétés et PDF/A** : la 0.9.1 conserve l'identification PDF/A (`pdfaid:part`, `pdfaid:conformance`) et les
     propriétés/espaces de noms XMP indépendants lors de la modification des quatre propriétés standard. Les XML
     malformés, DTD et flux illisibles sont refusés clairement ; aucune sortie ne remplace silencieusement le paquet.
     Conservation de l'identification ne signifie pas certification de conformité PDF/A. Validateur dédié et corpus
     réels restent à ajouter ; preuves d'exécution actuelles dans `PROJECT_STATUS.fr.md`.
A25. **Vue continue : limites (SUNI-012b)**.
     - La sélection suit l’ordre logique PDFium, y compris entre pages et au bord de la vue ; l’ordre voulu
       de colonnes arbitraires non balisées n’est pas déduit. Le marquage reste rectangulaire par page.
     - Défilement **sans animation ni inertie** pour les « aller à » et le clavier (saut direct).
     - **Pincement** (pavé tactile ou écran tactile) : même chemin que Ctrl + molette (`zoom_delta`), non testé
       (aucun test sur ce poste).
       Ctrl + molette vérifié par capture (`dev-screenshot.ps1 -Wheel`).
     - Un **rendu de tuile déjà commencé ne s'interrompt pas** : en défilant vite, la latence d'annulation est
       la durée d'une tuile (mesurée : 1,2 ms par tuile 1024² sur `many-pages.pdf`, plus pour une page lourde).
     - Recollage des tuiles : les glyphes qui traversent un bord de tuile peuvent différer d'un pixel
       d'anticrénelage du rendu d'un seul tenant (121 pixels sur 1,9 million mesurés, D044).
     - Double page et Livre gardent les mêmes plafonds ; rotation de vue/plein écran/historique implémentés en 0.10.0. L’édition revient à une vue droite ; contrôles interactifs distincts.
A26. **« Ajuster » en défilement continu sur un document de tailles mixtes** : le zoom est commun à toutes les
     pages et ajuste la **plus grande** page (D041) ; une page portrait parmi des pages paysage est donc plus
     étroite que l'écran. En page unique, l'ajustement suit la page courante. En double page/Livre, la plus large
     paire et la page la plus haute déterminent le zoom partagé.
A27. **Très longs documents** : positions en `f32` ; le zoom est plafonné pour que la hauteur totale reste
     <= 8 000 000 px logiques (message « Zoom maximal atteint pour ce document »). Un document de 5000 pages
     Lettre est ainsi limité à environ 150 %.
A28. **Mémoire de la vue** : bornée mais notable. Textures des pages 192 Mio (hors textures de l'écran), cache du
     moteur 256 Mio (tuiles et miniatures), 256 textures de miniatures au plus dans le panneau et la grille
     Organiser. Les demandes de miniatures en attente sont aussi plafonnées à 256 ; les résultats périmés
     sont écartés. Tests de régression sur une grille de 5 000 pages. La mesure suivante est antérieure
     à la 0.8.3 et concerne la lecture : build de débogage, 500 pages,
     300 « Page suiv. » puis Fin puis 300 « Page préc. » : mémoire privée stable vers 810-830 Mo (WorkingSet
     438 Mo) après 175 Mo à l'ouverture ; pas de croissance au-delà (pilote OpenGL NVIDIA, copies système des
     textures comprises). Plafonds en constantes (`view.rs`, `worker.rs`), pas de réglage utilisateur.
A35. **Signature visuelle = image sans preuve cryptographique** : dessinée, saisie ou importée, copiable ou supprimable par un éditeur.
     La CLI `add-image` indique `digital_signature: false`. La signature CMS/PFX est un autre parcours dans
     Utiliser un certificat (A44) ; l’application avertit avant la réécriture d’un PDF signé.
A36. **Signatures visuelles enregistrées** : la version 0.30.0 utilise des fichiers `.dpapi` chiffrés pour
     l'utilisateur courant, ACL privée de son SID. Les PNG bornés anciens migrent atomiquement après déchiffrement
     et comparaison des octets chiffrés persistés ; un échec conserve les originaux et signale le risque du clair.
     Enregistrement facultatif ; suppression en deux clics sans effacement sécurisé des secteurs/sauvegardes.
     Fichiers illisibles conservés ; autre utilisateur ou profil perdu/réinitialisé : le fichier chiffré seul ne
     permet pas de récupérer l'image. Une session ouverte permet encore la réutilisation. contrôles de la source publiée réussis ;
     [récupération et bornes](SIGNATURE_SECURITY.fr.md).
A37. **Pavé et import** : la 0.35.0 ajoute le lissage facultatif par courbes quadratiques, le nettoyage adaptatif du papier clair et un aperçu lié aux pixels préparés.
     Les points bruts et fichiers sources sont conservés ; au plus 20 000 points et 256 traits, avec refus de placement à la limite.
     La pression du stylet reste absente ; l’encre pâle, les traits larges et les fonds texturés exigent un contrôle de l’aperçu. Interaction native et photos clients restent à qualifier ;
     images importées limitées à 16 384 px de côté et 512 Mio décodés (refus à la lecture de
     l'en-tête, même borne de 512 Mio par image pour Insérer une image, Remplacer l'image et Images → PDF), puis réduites
     à 1600 px de côté ; la date posée est maintenue dans la page sur une largeur **estimée** (0,556 em par
     caractère, chiffres Helvetica), non mesurée. **0.13.0** : nom/paraphe saisi et aperçu natif
     explicite (une ligne, 256 caractères, 1600 px/côté ; Times italique par défaut, contrôles existants des glyphes
     et droits d'incorporation TrueType standard/système), pose courante/toutes/sélection. Saisie, changement de mode
     ou annulation de boîte/document invalident l'aperçu. Même centre en points PDF affichés, ajusté aux bords visibles,
     réduction seulement si trop grand ; aucun positionnement relatif proportionnel. L'annulation est vérifiée entre
     les pages, pas pendant un appel natif PDFium ; un lot volumineux demande aussi un instantané et une copie temporaire
     en mémoire. Un undo rétablit le lot entier ; la source change seulement à Enregistrer. CLI : ajustement opt-in
     par `add-image --pages`, sémantique historique de `--page` conservée. Publié en 0.13.0 ; contrôles exécutés et
     limites restantes dans `PROJECT_STATUS.fr.md`. Bibliothèque pleine : le corps de la boîte défile et sa barre flottante reçoit une gouttière de 10 px
     (le contenu se resserre d'autant à l'image qui suit l'apparition de la barre).
A38. **Formulaires AcroForm** : XFA lu, jamais rempli/aplati ; JavaScript, calculs et actions jamais exécutés.
     La 0.10.0 implémente création texte/case/radio/listes, export XFDF et import strict borné JSON/XFDF/FDF de valeurs.
     Chaque champ/valeur est validé avant mutation ; une entrée inconnue/invalide refuse tout l’import de l’interface.
     Les modifications du panneau restent en attente jusqu’à Appliquer ou Enregistrer, une seule annulation/relecture ;
     Abandonner revient aux valeurs courantes. La confirmation sur la page reste immédiate. Espace/Entrée activent
     les widgets focalisés ; les listes reçoivent le focus clavier. Noms uniques sans point/contrôle et rectangles
     numériques dans la page, dimensions minimales 4 points. Radios verticaux ; libellés dans le panneau, pas en texte PDF.
     FDF limité aux tables xref ordinaires et valeurs texte/nom/tableau ; flux, actions, pièces jointes, texte enrichi
     et export FDF non pris en charge. XML DTD/instructions de traitement/éléments inconnus/doublons/taille/profondeur
     excessives refusés ; chemins de document référencés jamais ouverts. Import limité à 16 Mio.
     La 0.11.0 livre le texte libre des combos déjà modifiables dans le panneau/sur la page ; contrôles locaux, CI et paquets réussis.
     La 0.12.0 livre la création de listes modifiables (choix prédéfinis requis) et les exports radio `/Opt`
     distincts des états d'apparence. Correspondances malformées/dupliquées : champ concerné en lecture seule ;
     même règle si `/Kids` brut malformé, références de widget dupliquées ou noms d'apparence non UTF-8. Les valeurs
     en lecture seule se réimportent inchangées, y compris `Off` littéral ou chaîne vide. Remplir un autre champ
     préserve `/V` et `/AS` bruts ; PDFium peut régénérer une apparence manquante et retirer la lecture seule déduite
     une fois le champ valide. Sans `/Opt`, noms d'état conservés. Contrôles locaux, CI, paquets/publics et mises à niveau réussis. Modification du drapeau d'un champ existant
     indisponible. Création souris publiée en 0.19.0 (D110) : cinq types et boîte numérique
     existants, paramètres conservés, rectangle fini borné à une page (minimum 4 pt par dimension), puis
     Créer le champ explicite/un Annuler. Changement de révision/onglet/outil : placement périmé annulé ; aucune
     édition de champ existant par le tracé. Si la navigation en page unique masque la page initiale pendant le geste,
     le relâchement conserve le dernier rectangle connu et rouvre la boîte numérique ; aucun champ créé sans
     Créer le champ explicite. Tests locaux, formatage et Clippy réussis ; contrôles CI/paquets/mise à jour/publication réussis.
     0.19.0 était la Latest stable à sa publication.
     Texte enrichi remplacé par texte simple ; mots de passe jamais stockés/exportés.
     En 0.21.0 publiée, l'aplatissement partiel peut laisser des champs orphelins ; les annotations autres que liens sans apparence peuvent disparaître.
     **Livré en 0.21.1 :** conserver graphes bruts non-widget, pièces jointes et références
     partagées sans apparence utilisable ; widgets/apparences dessinables restent aplatis, annotations
     conservées exclues du compte et document temporaire atomique. Entrées malformées ou graphes au-delà de
     la limite de copie de 32 niveaux : opération refusée sans modifier le PDF actif. Champs orphelins distincts. Apparences Unicode susceptibles d’utiliser une police système non
     embarquée, donc rendu variable ailleurs. Narrateur et lecteurs tiers non vérifiés ; preuves dans `PROJECT_STATUS.fr.md`.


A39. **Annotations avancées** : la version 0.30.0 ajoute FreeText à police incorporée avec aperçu réel, polygones,
    polylignes, nuages, tampons standard, pièces jointes et gomme des traits d'encre entiers. Style uniforme seulement ;
    composition non prise en charge refusée. Restylage/redimensionnement riches et données enrichies étrangères
    arbitraires restent limités. contrôles de la source publiée réussis ; [entrées et bornes](RICH_ANNOTATIONS.fr.md).
    **Base historique 0.9.0–0.21.1 ci-dessous ; ses limites décrivent ces releases :**
    **Annotations avancées (tranche annotations-advanced, D067–D076)** : **texte libre en Helvetica WinAnsi**
    seulement (accents latins, € et ponctuation courante ; tout autre caractère s'affiche « ? » dans l'apparence,
    `/Contents` garde le texte exact ; pas de gras, d'italique, de texte riche `/RC`, de CJK ni de droite à gauche) et
    largeur de renvoi à la ligne calculée avec la table Helvetica (marge de 2 %) ; jusqu'en 0.17.0, **un trait d'encre = une annotation**
    et chaque opération (trait, déplacement, style) coûte un aller-retour complet instantané + lopdf + rechargement :
    imperceptible sur un PDF courant, de l'ordre de la seconde sur un fichier de dizaines de Mo ; **fils de réponses et
    statuts** écrits selon ISO 32000-1 mais **jamais testés dans Acrobat, Foxit ni Chrome** (réponses sans icône :
    apparence vide + `F 4`), pas de modèle d'état « Marked » ; **lecture détaillée** des annotations (panneau,
    sélection) demandée à la demande et par page, qui relit tout le document. Navigation nommée/XYZ livrée en
    0.11.0 (A49). **0.15.0** ajoute le remplacement de cible existante UI/CLI (D106) : URI autorisée
    ou page physique Fit, vue détaillée explicitement remplacée. Zone/style/noms/commentaires, réponses et données
    indépendantes conservés ; `A`/`Dest`/`AA` remplacés ensemble. ReadOnly (64), Locked (128), LockedContents (512),
    drapeaux négatifs/malformés ou indirects illisibles : édition refusée. Cible inconnue : adresse vide, jamais page 1.
    Un Annuler conserve géométrie/navigation ; éditions et changements d'onglet ferment les boîtes périmées.
    Publié en 0.15.0 ; contrôles locaux, CI, mises à niveau, paquets préparés/publics et metteur à jour réussis ;
    preuves et limites restantes dans `PROJECT_STATUS.fr.md`.
    Launch / JavaScript / GoToR jamais
    exécutées ; **redimensionnement et style réservés aux annotations de SuniPDF**
    (`/NM` préfixé) : celles d'un autre producteur se déplacent, se commentent, se suppriment (surlignages et notes se
    recolorent aussi). **0.16.0 publiée** ajoute les tirets uniquement aux annotations rectangle, ellipse,
    ligne et flèche. UI/CLI choisissent `[6,3]` points ; vecteur réel conservé à la sélection, recoloration,
    déplacement/redimensionnement et annulation sauf changement explicite de case. Phase zéro, remplissage et
    pointes de flèche pleins. Motifs non vides refusés sur encre/texte libre/lien/marquages. `BS` malformé/non pris
    en charge sur une annotation SuniPDF : édition régénérant l'apparence refusée, sans remplacer le style. Au plus
    16 valeurs finies non négatives, chacune ≤ 1000 pt, valeur positive ≥ 0,1 pt et somme positive ; borne conservatrice
    de 100 000 segments. `D []` explicite : continu valide ; `D` absent : `[3]`. Bordure SuniPDF malformée :
    Déplacer/Redimensionner/Style désactivés. Commentaire de forme autorisé selon les permissions ordinaires sans
    reconstruire l'apparence ; commentaire de texte libre malformé refusé. Aperçu : motifs sous-pixel valides
    conservés, contour non vide invalide/excessif omis sans repli continu, sous la même borne de travail.
    Aucun éditeur de motif/phase. Contrôles locaux/CI/mises à niveau, paquets préparés/publics et metteur à jour de
    production réussis ; preuves et limites dans `PROJECT_STATUS.fr.md` (D107). Pas de gomme, polygone, nuage, tampon ou pièce jointe ;
    **Livré en 0.18.0 (D109).**
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
    Stylet/tactile natifs, boîtes interactives, imprimantes, Narrator, matériel ARM64 et lecteurs tiers restent non vérifiés. Paquets non signés ; brouillons d'encre non validés sans récupération après plantage.

    **Publié en 0.20.0 (D111) :** création/édition FreeText dans un éditeur de page multiligne ancré avec
    Ajouter/Enregistrer/Annuler explicites, réglages existants, validation/protection moteur et une étape Annuler.
    Page d'origine fixée ; zoom disponible ; navigation incompatible bloquée jusqu'à validation/annulation.
    Opérations en attente, brouillons concurrents et document/onglet/outil/espace de travail périmés protégés ;
    erreur de validation/worker : texte conservé. Brouillon en mémoire, exclu de la récupération après plantage.
    Helvetica WinAnsi, texte simple et agrandissement vers le bas inchangés ; aucune extension police incorporée/
    texte enrichi. Relecture, workspace, formatage, traductions et Clippy réussis en local ; CI/mises à niveau,
    paquets préparés/publics et metteur à jour de production vérifiés. 0.20.0 était Latest stable à sa publication. QA native/lecteurs distincte.

    Plafonds : 64 traits de 4 000 points, épaisseur ≤ 50 pt,
    zone de lien ≥ 6 pt ; **vérifié en capture** (`dev-screenshot.ps1`) : encre,
    sélection d'une annotation de SuniPDF (cadre, quatre poignées, barre flottante) puis déplacement et
    redimensionnement à la souris, déplacement d'une annotation d'un autre producteur (cadre sans poignées), boîtes de
    dialogue (zone de texte, lien, confirmation d'adresse), panneau Commentaires avec fil, thèmes clair et sombre ;
    **non rejoués en capture** : la zone de texte posée après sa boîte, la modification par double-clic, les flèches du
    clavier, le redimensionnement d'une encre, d'une ligne ou d'une zone de texte (tous couverts par les tests moteur et
    les tests purs de `annot_adv.rs`) ; contrôle structurel qpdf synthétique ajouté pour 0.9.1 (A4) ;
    interopérabilité des lecteurs toujours non établie.
A40. **Caviardage** : la version 0.30.0 ajoute aperçu natif avant/après, remplissage/texte, marques persistantes
    et assainissement partagé. contrôles de la source publiée réussis ; [garanties et bornes](REDACTION.fr.md). La sortie appliquée
    retire le contenu couvert par réécriture complète et vérification résiduelle, avec ces limites :
    1. **Granularité** : du texte sûr, opaque, Base14 latin peut conserver les glyphes non caviardés sélectionnables
       à leur position native. Polices complexes, composition, OCR, images, découpe et objets imbriqués gardent le
       repli par image ; leur reste visible peut perdre sélection/recherche. Graphes dangereux refusés atomiquement.
    2. **Texte d'une image numérisée sans OCR** : invisible pour « Caviarder un texte… » (il faut tracer la zone). Une
       couche OCR décalée par rapport à l'image peut échapper à une zone tracée : l'option « Convertir entièrement
       les pages caviardées en image » la supprime dans tous les cas.
    3. **Polices incorporées** : si la police du texte caviardé sert encore ailleurs, son sous-ensemble de glyphes
       reste dans le fichier (il révèle des caractères utilisés, ni leur ordre ni leur position).
    4. **Données cachées** : nettoyage par défaut des métadonnées prises en charge, texte de structure, pièces jointes,
       scripts et métadonnées de signets/destinations via l'assainisseur vérifié. Le désactiver conserve des données
       cachées susceptibles de répéter le contenu caviardé. Les pages non marquées ne sont pas effacées globalement.
    5. **Champs de formulaire** : un champ dont un widget est sous une zone disparaît partout (tous ses widgets,
       toutes les pages) ; `/XFA` est alors retiré.
    6. **Balisage d'accessibilité** : supprimé pour tout le document quand « informations cachées » est coché (défaut)
       ; conservé sinon, avec le risque qu'un texte alternatif décrive le contenu caviardé.
    7. **Signatures numériques** : toute signature existante devient invalide (réécriture complète).
    8. **Mémoire et disque** : annulation et onglets suspendus gardent les états antérieurs en mémoire. Le caviardage
       réussi actualise immédiatement l'instantané courant (ou le retire si l'actualisation échoue), mais les copies
       d'anciennes sessions interrompues peuvent contenir du texte non caviardé. L'original, les sauvegardes et les
       copies supprimées ne sont pas effacés du support de façon sécurisée.
    9. **Hypothèse restante** : les limites d'objet de PDFium englobent l'encre de l'objet. Pour le texte, elles sont
       agrandies de 30 % et contrôlées à l'exécution (texte résiduel sous une zone = erreur, document inchangé) ; pour
       images et tracés elles sont géométriques.
    10. **Documents refusés** : un flux de contenu qui ne se décode pas strictement (lopdf) donne
        `redaction_incomplete` sans rien modifier ; réessayer avec « page en image ».
    11. **Supprimer un objet avec l'outil Modifier n'est pas un caviardage** : PDFium peut laisser l'objet orphelin ou
        la ressource dans le fichier ; seul l'outil Caviarder assainit et réécrit le document.
    12. Brouillons/candidates en mémoire invalidés par les changements du document. **Conserver les marques PDF
        non appliquées** persiste des annotations `/Redact` récupérées à l'ouverture ; contenu encore présent avant
        Appliquer. Police/texte de remplacement non persistables dans cette tranche. Retirer un rectangle brouillon
        ne supprime pas une marque déjà persistée. Pas de nœud accessible par zone ; nombre annoncé dans la barre d'état.
    13. Non vérifiés : numérisations/PDF balisés lourds réels et pages de plusieurs dizaines de milliers d'objets
        (`prune_objects` de lopdf est quadratique). Le contrôle qpdf synthétique ne remplace pas ce corpus (A4).


## Limites des fonctions 0.9.0

Les résultats de validation exécutés sont suivis dans `PROJECT_STATUS.fr.md`.

A41. **Récupération** : instantanés toutes les 60 secondes et à la suspension d’un onglet, donc une modification
     plus récente peut manquer après un arrêt forcé. Une saisie de dialogue non validée n’est pas un changement PDF.
     Les instantanés protégés restent chiffrés, mais noms de fichiers/chemins et nombre de pages sont dans les métadonnées JSON.
     Aucun mot de passe n’est conservé ; il faut le ressaisir pour restaurer. L’original n’est jamais remplacé par la récupération.
A42. **Impression Windows** : rendu tramé jusqu’à 300 dpi par bandes bornées, ajusté à la zone imprimable ; pas de
     sortie vectorielle ni de garantie de couleur professionnelle. Le pilote gère copies/orientation. Une opération native
     déjà lancée peut retarder l’annulation. Sortie réelle Microsoft Print to PDF rouverte avec modifications non
     enregistrées, source/octet/objets inchangés. Imprimantes physiques et boîte interactive non vérifiées.
A43. **OCR local** : anglais/français uniquement, modèles rapides Tesseract ; qualité variable selon scan et langue.
     Les pages ayant déjà du texte sont ignorées, même si cette couche est mauvaise. Images limitées à 16 Mpx et 8192 px/côté,
     sortie TSV 16 Mio, 50 000 mots/page, délai 120 s/page ; très grandes pages refusées. Pas de reconstruction sémantique
     des tableaux/colonnes. Annulation/échec ne valide aucune page ; succès en un undo. Autres lecteurs à vérifier.
A44. **Signatures numériques et confiance** : intégrité CMS détachée et copie PFX/P12 incrémentale restent
     distinctes de la confiance d'identité ; signature chiffrée refusée. La version 0.30.0 ajoute vérification Windows
     des chaînes/révocations et horodatages de signature RFC3161. Récupération en cache par défaut, consentement
     explicite par vérification réseau ; révocation indisponible/inconnue jamais assimilée à une confiance établie.
     Empreintes fortes, EKU/temps/liaison du jeton et confiance TSA contrôlés. Heure PDF toujours déclarative ; aucune
     création d'horodatage ajoutée. DSS incorporé annoncé non vérifié, sans LTV d'archivage authentifiée. AATL/EUTL,
     PKCS#11, qualification et certification par lecteur tiers restent exclus. contrôles de la source publiée réussis ;
     [garanties et limites](SIGNATURE_SECURITY.fr.md).
A45. **Compression des images 0.10.0** : avec perte, images RGB/gris 8 bits prises en charge et limitées à 32 millions
     de pixels. Graphes des masques, profils, Decode/DecodeParms particuliers et autres images conservés. Si le fichier
     grossit, restitution exacte des octets source. Aucun poids cible garanti. PDF signés ou restreints refusés ; nouvelle copie.
A46. **Lots 0.10.0** : compression ou OCR anglais/français séquentiels, 128 PDF et 256 Mio/entrée maximum. Aucun fichier
     existant remplacé ; copies terminées conservées après erreur/annulation. Pas de demande de mot de passe par fichier ;
     document qui en nécessite un refusé. Annulation aux points de contrôle ; codec/appel natif actif peut retarder l’arrêt.
A47. **Sauvegardes/corpus 0.10.0** : erreurs de stockage injectées et arrêt forcé d’une écriture partielle contrôlent
     la préservation de la source. Ce ne sont pas toutes les pannes physiques/coupures possibles. Quatre PDF NIST publics
     complètent les fixtures synthétiques : 31 contrôles qpdf synthétiques et quatre documents réels (744 pages) réussis en 0.10.0.
     Cela ne prouve pas une couverture client/hostile/volumineuse universelle ; preuves exécutées dans `PROJECT_STATUS.fr.md`.

## Changements publiés et limites 0.11.0

Ces ajouts sont publiés en [0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0). Contrôles locaux, CI, paquets et mises
à niveau réussis ; contrôles publics/mise à jour aussi réussis ; preuves et limites dans `PROJECT_STATUS.fr.md`.

A48. **Étiquettes de page** : métadonnées facultatives distinctes des numéros imprimés. Lecture livrée en 0.11.0 ;
     création/réinitialisation à six styles en 0.14.0. Libellé vide/illisible/caractères de contrôle ou au-delà de
     4 096 octets UTF-8 : indice physique ; libellé exact unique prioritaire, `#N` physique, doublons exigeant un indice
     physique. Préfixe nouveau sur une ligne/256 octets, départ positif. La version 0.30.0 recale les étiquettes
     sémantiques par provenance lors des fusions/extractions/réorganisations/suppressions/insertions, sans modifier
     le texte imprimé. Arbres malformés/dangereux : refus structurel atomique ; doublons explicites. Réinitialiser retire
     la métadonnée, sans effacer les anciennes copies disque. contrôles de la source publiée réussis ; [périmètre](RELEASE_0.30.0.fr.md),
     [limites structurelles](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md).
A49. **Destinations** : la version 0.30.0 ajoute création nommée/XYZ/Fit/FitH et résolution brute bornée,
     distinguant axes nuls et zéro explicite, y compris FitH. Axes/zoom conservés selon la vue courante ; coordonnées
     CropBox/rotation affichées, zoom explicite borné. Cycles, ambiguïtés et destinations locales malformées refusés ;
     autres ajustements : navigation à la page seule. Actions non prises en charge conservées comme données inertes,
     jamais exécutées. contrôles de la source publiée réussis ; lecteurs indépendants et accessibilité restent distincts.
     [Périmètre](RELEASE_0.30.0.fr.md), [limites de conservation](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md).
A50. **Échec de lecture du formulaire** : distinct d'un formulaire vide, avec une nouvelle tentative explicite depuis
     le panneau. Brouillons et valeurs en attente sont conservés pendant l'échec. Un import/effacement en attente
     reprend seulement après une lecture réussie ; les nouvelles tentatives automatiques répétées sont bloquées.
