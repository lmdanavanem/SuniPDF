# Roadmap SuniPDF

[English](ROADMAP.md)

Cette roadmap distingue les fonctions livrées et le travail futur.
Les contrôles exécutés et preuves de publication sont dans [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md).

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

## 0.13.0 : améliorations de signature visuelle (publiée en Latest)

Nom/paraphe saisi avec aperçu natif explicite et parcours police/image/bibliothèque existants. Page courante,
toutes les pages et sélection partagent une géométrie affichée ajustée ; lot atomique, annulable entre cibles et
un undo. CLI `add-image --pages` additive ; comportement historique de `--page` compatible (D102–D103).
[Publiée en Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem`, release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation locale réussie : 410 tests workspace,
formatage/Clippy, 870 clés EN/FR, 53 contrôles qpdf et quatre documents licenciés (744 pages ; 76,40 s localement).
CI Windows, compilation ARM64 de même source, onze scénarios portables, cinq bases MSI et six fichiers préparés/publics
avec contrôles renforcés des binaires réussis. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le
ZIP Latest x64 de 19 730 578 octets sans installation. Preuves et limites matérielles/lecteurs : `PROJECT_STATUS.fr.md`.

## 0.14.0 : création d'étiquettes de page (en développement)

Appliquer six styles de navigation sur une plage physique inclusive via Pages/Organiser et deux commandes CLI ;
conserver les étiquettes hors plage, refuser un arbre existant malformé avant mutation, réinitialiser toutes les
règles sans annoncer d'effacement sécurisé. Numéros imprimés inchangés ; aucun recalage sémantique des plages
après opérations structurelles. Annuler/Rétablir contenu/étiquettes conserve la navigation si géométrie inchangée
(D104–D105). Validation et contrôles de release en cours ; la 0.13.0 reste Latest publiée. Voir `PROJECT_STATUS.fr.md`.

## Phases 0–1 : planification et fondations — livrées

Workspace Rust, PDFium, erreurs typées, ouverture/navigation/zoom/fermeture, mots de passe, glisser-déposer,
localisation FR/EN, paramètres persistés, thèmes clair/sombre, registre de commandes et ressources Windows sont en place.
Les jalons et décisions historiques restent dans `DECISIONS.fr.md` et `PROJECT_STATUS.fr.md`.

## Phase 2 : lecture et opérations PDF essentielles — largement livrée

Livrés : onglets, caches de rendu/miniatures bornés, vues continue/double page/Livre, sélection logique entre pages,
recherche, signets, fusion/division/extraction/rotation/réorganisation, enregistrement atomique, annulation/rétablissement et annotations.

Livrés en 0.10.0 : rotation de vue, plein écran et historique borné par onglet.
Livrés en 0.11.0 : lecture des étiquettes et navigation interne précise. Création/réinitialisation en développement
pour 0.14.0. Restent : recalage sémantique des étiquettes et ordre voulu des colonnes complexes non balisées. qpdf ne prouve pas la préservation de
chaque lien, champ, calque ou pièce jointe.

## Phase 3 : édition professionnelle — partiellement livrée

Livrés : texte/images/formes, transformations d’objets, déplacement groupé et presse-papiers, polices système TrueType,
tampons/filigranes/numérotation, commentaires, métadonnées, signatures visuelles, remplissage/aplatissement AcroForm,
création de champs texte/case et sauvegarde/récupération.

Livrés en 0.10.0 : transformations groupées, recadrage souris, champs radio/listes, échanges FDF/XFDF de valeurs et clavier.
Livrés en 0.12.0 : création de listes modifiables et exports radio `/Opt`, avec lecture seule conservatrice si nécessaire.
Livrés en 0.13.0 : signatures visuelles saisies et pose atomique sur page courante/toutes/sélection.
Restent : sous-ensembles de polices/CFF/OTF/TTC, paragraphes et collage éditable.
Critère : rendu, enregistrement et réouverture couverts pour chaque opération, avec contrôles d’accessibilité applicables.

## Phase 4 : conversion et OCR — partiellement livrée

Livrés : images ↔ PDF, PDF → texte/PNG, optimisation sans perte, lots CLI et OCR anglais/français local embarqué.

Restent : Office → PDF via convertisseur local détecté, PDF → Word/Excel/PowerPoint avec limites de fidélité mesurées
et explicites, profils réutilisables. Compression des images et lots compression/OCR livrés en 0.10.0.

## Phase 5 : sécurité et productivité — partiellement livrée

Livrés : AES-128/256 et permissions, retrait de protection autorisé par le propriétaire, nettoyage des métadonnées,
caviardage irréversible, vérification CMS et copies signées PFX/P12 incrémentales, CLI JSON et automatisation.

Restent : comparaison, recherche multi-documents, MCP opt-in confiné, intégration Explorateur et raccourcis configurables.
Confiance de chaîne, révocation, horodatage fiable, AATL/EUTL, PKCS#11 et LTV sont **non implémentés** ;
l’heure de signature PDF est déclarative. L’interopérabilité Acrobat/Foxit est une propriété distincte, non vérifiée.
La signature du code/MSI demande une identité appropriée fournie.

## Phase 6 : qualité et distribution — en cours

Base livrée : releases x64 portable/MSI et ARM64 portable, SHA256, notification au démarrage avec installation explicite,
guides FR/EN, notices tierces et workflows de compilation GitHub.

Livrés en 0.10.0 : quatre documents NIST publics et scénarios d’enregistrement interrompu/disque plein/échec.
Restent : corpus client/hostile/volumineux plus large, fuzzing,
benchmarks avec matériel, inventaire complet des licences, Narrateur et lecteurs tiers, parcours interactifs de mise à
jour MSI/portable, imprimantes physiques et exécution ARM64/OpenGL réelle.
La régression MSI ne couvre que son scénario scripté ; compiler un paquet ARM64 ne valide pas son exécution matérielle.

## Suite après le lot 0.13.0

1. Exécuter les dialogues natifs de mise à jour/enregistrement, Narrateur, imprimantes, ARM64 et lecteurs tiers.
2. Élargir corpus réel et mesures de performance ; les échantillons NIST ne prouvent pas une compatibilité universelle.
3. Définir séparément polices/paragraphes, conversion Office locale et services de confiance.

Aucune date/version suivante n’est engagée. L’IA facultative reste désactivée par défaut.
