# SuniPDF

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/sunipdf-wordmark-dark.png">
    <img src="assets/logo/sunipdf-wordmark-light.png" width="240" alt="SuniPDF">
  </picture>
</p>

[English](README.md)

**Lecteur et éditeur PDF natif pour Windows 10/11, libre, traitement local, bilingue anglais / français.**

Créé par **Emanuel DE ALMEIDA** ([@lmdanavanem](https://github.com/lmdanavanem)) · [Navanem](https://navanem.com/projets/sunipdf/) ·
Licence MIT ou Apache-2.0, au choix.

[![Release](https://img.shields.io/github/v/release/lmdanavanem/SuniPDF)](https://github.com/lmdanavanem/SuniPDF/releases)
[![CI](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/lmdanavanem/SuniPDF/actions/workflows/ci.yml)
![Licence](https://img.shields.io/badge/licence-MIT%20ou%20Apache--2.0-blue)

## Pourquoi SuniPDF
- **Vos fichiers restent chez vous** : aucun PDF envoyé, aucun compte, aucune télémétrie, aucun traitement dans le cloud. La recherche de mises à jour contacte GitHub et peut être désactivée.
- **Natif** : Rust, interface egui, moteur PDFium ; ZIP portable ou MSI, avec OCR embarqué.
- **Complet** : lecture, annotation, édition du contenu, organisation des pages, formulaires, signature visuelle,
  tampons, conversions, protection, ligne de commande scriptable.
- **Honnête** : ce qui n'est pas fini reste dans la feuille de route (`docs/ROADMAP.fr.md`), jamais annoncé comme terminé.

## 0.39.0 : modèles de décoration réutilisables (Latest stable)

[0.39.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) sous `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

Modèles locaux nommés pour filigranes texte/image, numéros et six zones d’en-tête/pied. Charger un brouillon, enregistrer sous un nom unique ou supprimer explicitement. Écriture atomique, préférences préservées en cas d’échec et modèles invalides isolés. Portée des pages explicite et images référencées par chemin. Paquets 0.38.0 ajoutés aux migrations. Vérification de version Office sans interface avec une limite annulable de 30 secondes au démarrage à froid ; limites de conversion et gardes de sécurité conservées. [Utilisation et limites](docs/PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404372880) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404384844) réussies : **628 tests, zéro échec**, 10 ignorés par défaut, **1149 clés EN/FR**, **31 scénarios portables**, **25 références MSI/125 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,879,736 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.39.0.json).

## 0.38.0 : six zones d’en-tête et pied de page (publiée ; Latest à cette date)

[0.38.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) sous `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

Six zones d’en-tête/pied via Numéros de page et la commande headers-footers. Numéro absolu, total, marges, taille et pages choisies respectent rotation et recadrage. Dépassement refusé atomiquement ; chiffrement autorisé et nouvelle copie en CLI conservés. Paquets 0.37.0 ajoutés aux migrations. [Utilisation et limites](docs/PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) réussies : **623 tests, zéro échec**, 10 ignorés par défaut, **1143 clés EN/FR**, **30 scénarios portables**, **24 références MSI/120 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,823,494 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.38.0.json).

## 0.37.0 : filigranes image (publiée ; Latest à cette date)

[0.37.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) sous `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

Filigranes image dans Pages et la CLI : transparence, largeur proportionnelle, angle, sept positions et marges. Ajustement aux pages tournées/recadrées. Modification atomique, une annulation dans l’application et nouvelle copie en CLI ; chiffrement autorisé conservé. Paquets 0.36.0 ajoutés aux migrations. [Utilisation et limites](docs/PAGE_DECORATIONS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) réussies : **618 tests, zéro échec**, 10 ignorés par défaut, **1134 clés EN/FR**, **29 scénarios portables**, **23 références MSI/115 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **24 scénarios, 113 appels CLI et 1 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. Deux alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,806,746 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.37.0.json).

## 0.36.0 : raccourcis personnalisables (publiée ; Latest à cette date)

[0.36.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) sous `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

Raccourcis clavier personnalisables dans Aide et la palette : filtre, attribution, refus des conflits, désactivation et restauration. Brouillon annulable, préférences atomiques et libellés cohérents. Saisie et touches réservées préservées ; paquets 0.35.0 ajoutés aux références de migration. [Utilisation et bornes](docs/KEYBOARD_SHORTCUTS.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) réussies : **613 tests, zéro échec**, 10 ignorés par défaut, **1128 clés EN/FR**, **28 scénarios portables**, **22 références MSI/110 journaux**. Sept nouveaux tests egui sans fenêtre contrôlent raccourcis, conflits, focus, affichage, annulation et compatibilité des préférences. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, corpus, mutations et Office conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les **20 scénarios d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé vérifié, a téléchargé et contrôlé les **26,788,530 octets** du ZIP x64 sans installation sur le poste.

Claviers Windows natifs et Narrateur restent à qualifier. Explorer reste à faire ; seules les commandes de la table existante sont personnalisables. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects reportées à la demande du propriétaire, sans synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.36.0.json).

## 0.35.0 : préparation des signatures visuelles (publiée ; Latest à cette date)

[0.35.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) sous `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

Signatures visuelles : lissage facultatif du dessin, réduction des ombres du papier et commande PNG `prepare-signature-image`. Aperçu lié aux pixels préparés, conservation des points bruts et fichiers sources, limites explicites du dessin. Les paquets publiés de la 0.34.0 rejoignent les références de migration. [Utilisation et bornes](docs/VISUAL_SIGNATURE_PREPARATION.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) réussies : **606 tests, zéro échec**, 10 ignorés par défaut, **1114 clés EN/FR**, **20 scénarios d’image**, **14 scénarios d’inspection PDF**, **27 scénarios portables**, **21 références MSI/105 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, PDF, mutations et Office restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les suites image et inspection ont aussi été rejouées sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé et vérifié, a téléchargé et contrôlé les **26,752,605 octets** du ZIP x64, sans installation sur le poste.

Signatures visuelles sans preuve d’identité ; le nettoyage peut supprimer l’encre pâle et doit être vérifié dans l’aperçu. Interaction native/tablette/Narrateur et photos clients encore à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Projects reste reporté à la demande du propriétaire, sans synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.35.0.json).

## 0.34.0 : inspection du PDF enregistré (publiée ; Latest à cette date)

[0.34.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) sous `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

Inspection du PDF enregistré depuis Propriétés et la CLI JSON `inspect` : sept catégories de déclarations, SHA256, résultats partiels explicites, entrées chiffrées et refus des réponses périmées. La garde de version sauvegardée calcule les empreintes par blocs. [Utilisation et bornes](docs/PDF_INSPECTION.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) réussies : **600 tests, zéro échec**, 10 ignorés par défaut, **1108 clés EN/FR**, **14 scénarios d’inspection empaquetée**, **26 scénarios portables**, **20 références MSI/100 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, corpus PDF, mutations et Office existants restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié, a téléchargé et contrôlé les **26,741,242 octets** du ZIP x64, sans installation sur le poste.

Inventaire structurel sans garantie d’innocuité ni validation des signatures/PDF/A/PDF/UA. Paquets non signés ; validations natives/matérielles et critères 1.0 encore ouverts. Projects reste reporté à la demande du propriétaire ; aucune synchronisation nouvelle annoncée. [Preuves](docs/qa/RELEASE_0.34.0.json).

## 0.33.0 : préparation 1.0 et compatibilité PDF (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) sous `lmdanavanem`, release `417802490`, source immuable `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

Préservation des signets PDF 2.0 importés contenant des tabulations/sauts de ligne, garde stricte des nouveaux titres, migration groupée egui/eframe 0.37 et correction de la position Précédent/Suivant. Sept exemples PDF Association sous licence et empreintes fixes, 64 mutations PDF/CMS bornées, contrôles négatifs et mesures par commande. La 0.32.0 devient une référence de migration. [Critères 1.0](docs/RELEASE_1.0_CRITERIA.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) réussis : **592 tests**, zéro échec, 10 ignorés par défaut, **1092 clés EN/FR** ; 84 contrôles qpdf et 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, quatre documents NIST/744 pages ; **7 exemples PDF 2.0/8 pages/21 contrôles qpdf**, **64 mutations et 2 contrôles de garde**, **25 scénarios portables**, **19 références MSI/95 journaux**. La CLI empaquetée passe aussi les 7 exemples/21 contrôles qpdf, 11 scénarios d’ordre d’objets/3 contrôles qpdf, 11 d’analyse locale et 8 de sécurité. CodeQL Rust/Python/Actions réussit, sans alerte ouverte.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié par empreinte, a téléchargé et vérifié le ZIP x64 de **26,726,762 octets en 1.65 s**, sans installation.

La mise à jour Projects est reportée à la demande explicite du propriétaire. Le dernier contrôle public montre **43 cartes, 25 Done, 18 Todo** ; les deux P1 ont une livraison partielle documentée, sans clôture ni changement de statut annoncé.

Paquets Windows non signés ; aucune clé indépendante de publication. Les exemples pédagogiques et les mutations déterministes ne remplacent pas un corpus client/hostile, du fuzzing guidé ou des mesures matérielles. ARM64 expérimental ; validations natives, lecteurs tiers et conformité PDF/A/PDF/UA encore ouvertes. [Preuves](docs/qa/RELEASE_0.33.0.json).

## 0.32.0 : ordre des objets (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) sous `lmdanavanem`, release `417623635`, source immuable `8c492db35f1712648d5472593146077230befb7c`.

Réorganisation d’un objet ou de la sélection par glisser-déposer dans le panneau Objets, trait d’insertion, contrôle de la page du cache et du contexte, Échap et une seule étape Annuler. Commande JSON `reorder-objects` avec destination finale explicite, ordre relatif, permissions, sauvegarde et mots de passe existants. [Périmètre et utilisation](docs/OBJECT_ORDER.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) réussis : **588 tests**, zéro échec, 9 ignorés par défaut ; **1092 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **24 scénarios portables**, **18 références MSI/90 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte. La CLI empaquetée passe **11 scénarios d’ordre d’objets et 3 contrôles qpdf supplémentaires**, plus 11 scénarios d’analyse locale et 8 de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,119,935 octets en 1.75 s**, sans installation. Projects : **43 cartes, 25 Done, 18 Todo, zéro In Progress** ; Suggestion P3 livrée, critères conservés, description relue après rechargement et trois filtres enregistrés vérifiés.

Paquets Windows non signés ; aucune clé indépendante de publication. Les validations matérielles, natives/manuelles et lecteurs tiers restent séparées ; aucun certificat PDF/A/PDF/UA/LTV. [Preuves](docs/qa/RELEASE_0.32.0.json).

## 0.31.0 : comparaison locale et recherche multi-PDF (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) sous `lmdanavanem`, release `417463135`, source immuable `362c7f3979533000e454769e48a8cfbd4084dc0d`.

Comparaison locale de deux PDF enregistrés (texte, dimensions et aperçus), alignement physique explicite et refus des aperçus obsolètes. Recherche littérale dans plusieurs fichiers sélectionnés, extraits et navigation, progression, annulation et résultats partiels explicites. Commandes JSON `compare` et `search-files` ; interface EN/FR. [Périmètre et limites](docs/LOCAL_REVIEW.fr.md).

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) réussis : **582 tests**, zéro échec, 9 ignorés par défaut ; **1091 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **23 scénarios portables**, **17 références MSI/85 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte après revue individuelle de quatre secrets de tests synthétiques.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; onze scénarios d’analyse locale et huit de sécurité réussissent avec la CLI empaquetée. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,090,504 octets en 1.71 s**, sans installation. Projects : **43 cartes, 24 Done, 19 Todo, zéro In Progress** ; les deux Suggestions P2 sont livrées, les critères conservés et les trois filtres relus.

Paquets Windows non signés ; aucune clé de publication indépendante. ARM64, matériel et parcours manuels restent à qualifier. La comparaison bornée ne prouve ni identité des PDF ni conformité. [Preuves](docs/qa/RELEASE_0.31.0.json).

## 0.30.1 : correctif de sécurité (publiée ; Latest à cette date)

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) sous `lmdanavanem`, release `417149160`, source immuable `697a963c756b316e1eb1e678298886bc08521da1`.

Nettoyage XFA/Screen et des textes d'accessibilité, autorisation propriétaire avant reprotection, signatures renforcées, mots de passe CLI par environnement/entrée standard et gardes Office/runtime renforcées. Les P1 Roadmap et Suggestions non traités restent différés. [Détails](docs/RELEASE_0.30.1.fr.md).

Le [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) et la [CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) réussissent sur cette source : **575 tests**, zéro échec, 9 ignorés par défaut ; **1060 clés EN/FR**, **84 contrôles qpdf du corpus** et **9 sorties de fonctions indépendantes**, **4 cas de polices**, rendu logiciel DX12, **14 allers-retours Office**, 4 PDF sous licence/744 pages, **22 scénarios portables**, **16 références MSI/80 journaux de transactions**. CodeQL Rust, Python et Actions réussissent sans alerte ouverte.

Les six fichiers publics et les copies `dist/` correspondent aux paquets inspectés et aux SHA256. Huit régressions de sécurité passent aussi avec la CLI publiée. Le code du metteur à jour a vérifié le ZIP x64 de **27,029,229 octets** en **1.21 s**, sans installation sur ce poste, avec le harnais déjà compilé et le module inchangé.

Les mises à jour dépendent encore de GitHub HTTPS/SHA256 du même canal : aucune clé de publication indépendante n'est disponible. Les paquets Windows restent non signés ; le matériel ARM64 et les parcours manuels restent à qualifier. [Preuves et limites](docs/qa/RELEASE_0.30.1.json).

## 0.30.0 : édition, protection des documents et conversion locale (publiée ; Latest à cette date)

Publiée (Latest à cette date) en [0.30.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) sous `lmdanavanem`, release `415568680`, source immuable `4db31c0ee62cfae9d65239edf1eac93db147841e`. Les cinq Bugs et quatorze P2 Roadmap sont livrés dans leurs [limites documentées](docs/RELEASE_0.30.0.fr.md). P1 Roadmap et Suggestions restent différés.

[Workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) et [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) réussis sur cette source : **551 tests**, zéro échec, 9 ignorés par défaut ; **1047 clés EN/FR**, **84 contrôles qpdf** plus **9 sorties de fonctions indépendantes**, **4 cas de polices MuPDF**, DX12 logiciel WARP, **14 allers-retours Office**, 4 PDF licenciés/744 pages, **21 scénarios portables**, **15 bases MSI/75 journaux**. CodeQL Rust, Python et Actions réussis ; zéro alerte ouverte.

Six fichiers publics vérifiés octet par octet avec les paquets préparés et leurs SHA256. Le metteur à jour de production a vérifié le ZIP x64 de 26,978,019 octets en 2.55 s, sans installation sur cet hôte, au moyen du banc de test déjà compilé dont le module de mise à jour est inchangé. Projet relu : **42 éléments uniques, 20 Todo, 0 In Progress, 22 Done**, trois filtres enregistrés et aucune modification non enregistrée. [Preuves et limites](docs/PROJECT_STATUS.fr.md).

Les paquets restent non signés. ARM64 est compilé et empaqueté, sans exécution sur matériel ARM64. Imprimantes physiques, Narrateur, interactions manuelles de mise à jour et GPU/fenêtres physiques restent à qualifier ; WARP prouve uniquement le rendu logiciel hors écran. Les entrées non prises en charge restent refusées. La conservation PDF/A ne certifie pas PDF/A/PDF/UA.

## 0.21.1 : conservation des documents (publiée ; Latest à cette date)

Les deux correctifs P1 sont livrés dans la [0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) : refus du texte incompatible
sans perdre le bon brouillon, et conservation des annotations non-widget sans apparence utilisable à
l'aplatissement. 476 tests, 84 contrôles qpdf, 20 scénarios portables et 14 bases MSI réussis ; six fichiers
publics et mise à jour en ligne vérifiés. Voir l'[état du projet](docs/PROJECT_STATUS.fr.md).

## Fonctionnalités
| Domaine | Détail |
|---|---|
| Lire | Onglets, vues continue/page unique/double page/Livre, création/réinitialisation des étiquettes et page physique `#N`, destinations précises des liens/signets, rotation de vue, plein écran, historique de position/zoom, tuiles, recherche et sélection logique entre pages |
| Commenter | Surligner, souligner, barrer, notes, encre, texte libre, formes aux bordures continues/en tirets, liens aux destinations autorisées modifiables, fils de réponses et panneau des commentaires |
| Modifier | Texte (polices standard et polices système incorporées), images, formes, sélection / déplacement / redimensionnement / rotation / opacité des objets existants, rotation/redimensionnement groupés, copier-coller entre pages et onglets, panneau Objets, recadrage souris et redimensionnement des pages |
| Organiser | Pivoter, supprimer, insérer, réordonner, combiner, diviser, extraire, pages vierges ; extraction/division conservent chiffrement, mots de passe et permissions |
| Remplir et signer | AcroForm, texte libre des listes modifiables, exports radio `/Opt`, brouillons conservés/relance après erreur de lecture, Appliquer/Abandonner groupés, clavier, aplatissement, export JSON/XFDF et import JSON/XFDF/FDF de valeurs ; signatures visuelles/paraphes dessinés, saisis ou importés, pose atomique sur toutes les pages/la sélection, date |
| Créer des formulaires | Champs texte, cases, groupes radio, listes déroulantes ordinaires/modifiables et zones de liste simples/multiples nommés par rectangles à la souris ou numériques, documents pivotés/recadrés/protégés, annulation et remplissage/enregistrement |
| Récupérer | Instantanés automatiques toutes les 60 secondes et à la suspension ; source intacte, chiffrement PDF conservé, aucun mot de passe stocké, restauration Enregistrer sous |
| Imprimer | Sélection native Windows, plages/copies/orientation, modifications courantes, rendu tramé borné, progression et annulation |
| Tampons | Filigrane texte (angle, opacité, position), numérotation des pages |
| Convertir | Images → PDF, PDF → PNG, optimisation sans perte, compression des images avec perte vers une nouvelle copie ; OCR anglais/français embarqué hors ligne, texte invisible recherchable, pages textuelles ignorées, annulation et undo ; lots séquentiels compression/OCR |
| Protéger | Chiffrement AES-128 / AES-256 avec permissions, retrait de la protection, nettoyage des métadonnées, propriétés et caviardage irréversible |
| Certificats | CMS détaché sur les octets chargés/enregistrés (SHA-256/384/512), résultats Windows de confiance/révocation et RFC3161 ; copie PFX/P12 incrémentale ; signature des PDF chiffrés refusée |
| Automatiser | `sunipdf-cli` : 62 commandes JSON et codes stables, dont `set-link-target`, `set-page-labels`, `reset-page-labels`, `compress-images`, `add-form-field`, `ocr`, `signatures` et `sign` |

Les 14 outils possèdent de vraies commandes. La sélection suit l'ordre logique PDFium, sans reconstruire des colonnes
arbitraires non balisées. Impression réelle, matériel ARM64 et interopérabilité des signatures restent des contrôles QA distincts.

Détail fait / planifié : `docs/FEATURE_MATRIX.fr.md`, `docs/PROJECT_STATUS.fr.md`, `docs/ROADMAP.fr.md` ; nouveautés : `CHANGELOG.fr.md`.

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
des autres éléments est suivi dans PROJECT_STATUS.fr.md. Voir la [revue Code Scanning](docs/CODE_SCANNING_REVIEW.fr.md)
et l'[état du projet](docs/PROJECT_STATUS.fr.md). 53 commandes CLI conservées.

[Version stable précédente](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.0) sous `lmdanavanem`, release `415338013`,
source `f006b7dd027fd5a3084c2bfffc2fceaa1b66be46`. Validation locale : 471 tests réussis, zéro échec, sept ignorés par défaut ;
formatage/Clippy sans avertissement, 912 clés EN/FR, 75 contrôles qpdf, autotest du metteur à jour et corpus quatre PDF/744 pages.
[CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648173) et [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957647839/job/117164444654) réussis :
19 scénarios portables, 13 bases MSI et 65 journaux vérifiés. Six fichiers préparés/publics et copies `dist/` correspondent aux empreintes API/SHA256 ; contrôles complets des paquets réussis.
Le metteur à jour de production a vérifié le ZIP Latest x64 de 19,846,878 octets en 1.76 s, sans installation sur l'hôte.
[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) a corrigé #3/#119 sur cette source ; aucune alerte ouverte après triage individuel des 115 critiques.
Preuves détaillées et limites dans l'[état du projet](docs/PROJECT_STATUS.fr.md).

Depuis **0.15.0** : modifier la destination d'un lien existant via **Modifier la destination**,
double-clic ou `set-link-target`, en conservant zone, apparence et commentaires. Choisir une adresse web/mail
autorisée ou une page physique affichée en entier. Liens verrouillés/en lecture seule refusés. Création d'étiquettes
et navigation conservée par Annuler/Rétablir restent disponibles depuis 0.14.0 ; voir la [roadmap](docs/ROADMAP.fr.md).

Depuis **0.16.0** : bordures en tirets des annotations rectangle, ellipse, ligne et flèche via
**Bordure en tirets** et CLI `annotate --dashed`. Les motifs personnalisés restent conservés à la sélection,
recoloration et modification géométrique ; seul un changement explicite de case remplace le motif. Les styles
invalides sont refusés sans remplacement silencieux. Validation et limites dans l'[état du projet](docs/PROJECT_STATUS.fr.md).

Depuis **0.17.0** : incorporer les JPEG des huit orientations EXIF sans recompression, dont
les orientations 5 et 7 auparavant décodées. Images → PDF contrôle aussi un **budget raster cumulé estimé de
512 Mio** avant de décoder les entrées non-JPEG. Cette estimation porte sur les images décodées, pas la mémoire
totale du processus. Accès UI/CLI et 53 commandes inchangés. Contrôles de conversion debug et des paquets préparés/
publics réussis : flux JPEG exacts, rendus orientés, transparence PNG, refus de budget sans allocation raster et
entrées inchangées. Preuves exécutées et limites dans l'[état du projet](docs/PROJECT_STATUS.fr.md).

### Nouveautés 0.18.0

Regrouper les traits à main levée d'une page en une annotation et une étape Annuler via **Terminer le dessin** /
**Annuler le dessin**. Une erreur du worker conserve le brouillon pour réessayer ; les actions incompatibles
enregistrement/fermeture/document/outil attendent Terminer/Annuler. Réutiliser AddInk, simplification des points,
géométrie affichée et protection (D109). Au plus 64 traits et 4 000 points par trait. Brouillon en mémoire
jusqu'à Terminer, absent de la récupération après plantage. Aucune validation à l'inactivité ni gomme.
L'aide de `stamp` / `page-numbers` conserve `{n}` / `{total}` littéraux via le modèle clap existant (A16).
Aucune nouvelle dépendance ni commande CLI ; 53 commandes.

### Nouveautés 0.19.0

Le lot implémenté ajoute le placement à la souris dans **Préparer un formulaire** pour les types existants : texte,
case, radio, liste déroulante et zone de liste. **Dessiner sur la page** masque temporairement la boîte numérique ;
tracer un rectangle la rouvre en conservant nom, choix et options. Seul **Créer le champ** modifie le PDF par le parcours
existant avec annulation. La création numérique reste disponible ; aucun nouveau type, dépendance ou commande CLI.
Tests locaux, formatage et Clippy réussis ; contrôles CI/paquets/mise à jour/publication réussis. **0.19.0 était la Latest stable à sa publication**. Voir les
[prochaines tâches](docs/NEXT_TASKS.fr.md) et l'[état du projet](docs/PROJECT_STATUS.fr.md).

### Nouveau en 0.20.0

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
et metteur à jour de production réussis. **0.20.0 était publiée en Latest stable à cette date** ; l'[état du projet](docs/PROJECT_STATUS.fr.md) consigne les preuves exécutées.

## Captures d'écran
| Accueil (thème sombre) | Accueil (thème clair) |
|---|---|
| ![Accueil, thème sombre](docs/screenshots/home-dark.png) | ![Accueil, thème clair](docs/screenshots/home-light.png) |

| Modifier un PDF : sélection et style d'un objet | Organiser les pages : grille de miniatures |
|---|---|
| ![Modifier un PDF](docs/screenshots/edit-objects.png) | ![Organiser les pages](docs/screenshots/organize-pages.png) |

<p align="center"><img src="docs/screenshots/about.png" width="720" alt="Boîte À propos"></p>

## Installation
Page [Releases](https://github.com/lmdanavanem/SuniPDF/releases) : **MSI x64** (installation classique) ou
**ZIP portable x64** (décompresser, lancer `sunipdf.exe`), avec empreintes SHA256. Un ZIP ARM64 est compilé en
intégration continue mais n'a pas encore été exécuté sur du matériel ARM64. Aucun redistribuable Visual C++ n'est requis.
L'OCR inclut Tesseract 5.5.3, Leptonica 1.87.0 et les modèles rapides anglais/français figés ; gardez le dossier
`licenses/` avec l'application. Aucun modèle n'est téléchargé pendant la reconnaissance et aucun PDF n'est envoyé.

Depuis la 0.8.2, une notification au démarrage permet de télécharger la mise à jour, puis de fermer et installer
après confirmation. Le MSI met à jour une installation x64 ; les portables remplacent leur dossier avec une sauvegarde.
La recherche automatique se désactive dans À propos ; les versions antérieures demandent une première mise à niveau manuelle.

Guides : [`docs/USER_GUIDE.fr.md`](docs/USER_GUIDE.fr.md) · [`docs/USER_GUIDE.en.md`](docs/USER_GUIDE.en.md).

## Compiler
Prérequis : [Rust stable](https://rustup.rs), **Build Tools pour Visual Studio** (charge « Développement Desktop en C++ »)
et **CMake** dans PATH. La première préparation OCR compile les sources natives figées après contrôle SHA256.
```powershell
git clone https://github.com/lmdanavanem/SuniPDF.git
cd SuniPDF
.\scripts\fetch-pdfium.ps1              # binaires PDFium (bblanchon/pdfium-binaries), empreinte vérifiée
.\scripts\fetch-ocr.ps1 -Arch x64      # moteur OCR et modèles locaux, sources et empreintes figées
cargo test --workspace -j 2              # moteur, interface, CLI, i18n
cargo run -p sunipdf-desktop -j 2 -- fichier.pdf
cargo run -p sunipdf-cli -j 2 -- info fichier.pdf
.\scripts\package-portable.ps1 -Arch x64 # ZIP portable + SHA256
.\scripts\package-msi.ps1 -Arch x64      # MSI (WiX 3.14, téléchargé automatiquement)
```
Notes d'ingénierie (patrons de code, pièges, vérification visuelle) : `docs/ENGINEERING_NOTES.fr.md`.
Contribuer : `CONTRIBUTING.md` · Sécurité : `SECURITY.md`.

## Structure
```
apps/sunipdf-desktop   application (egui/eframe)      crates/sunipdf-core   moteur (PDFium, lopdf)
apps/sunipdf-cli       CLI JSON                       crates/sunipdf-i18n   traductions fr-FR / en-US
assets/i18n            catalogues                     docs/                 architecture, guides, décisions
packaging/wix          installateur MSI               scripts/              outils PowerShell / Python
```

## Licence et remerciements
Code SuniPDF : [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE), au choix. Composants tiers et leurs licences :
`docs/THIRD_PARTY_NOTICES.fr.md`. SuniPDF s'appuie sur [PDFium](https://pdfium.googlesource.com/pdfium/) via
[pdfium-render](https://github.com/ajrcarey/pdfium-render), [lopdf](https://github.com/J-F-Liu/lopdf),
[egui / eframe](https://github.com/emilk/egui) et les polices système Segoe UI. SuniPDF n'est affilié ni à Adobe,
ni à Google ou Foxit (PDFium).
L'OCR utilise [Tesseract](https://github.com/tesseract-ocr/tesseract) (Apache-2.0),
[Leptonica](https://github.com/DanBloomberg/leptonica) (BSD-2-Clause) et les modèles anglais/français
[tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) (Apache-2.0). Licences, auteurs, inventaire
des copyrights et notice de compilation sont livrés dans `licenses/`.
