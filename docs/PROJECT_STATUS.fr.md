# État du projet — mis à jour le 2026-10-16 (0.19.0 Latest stable)

[English](PROJECT_STATUS.md)

Règle : une case cochée = code présent **et** commande exécutée avec résultat observé sur ce poste
(Windows 11 Pro 26300, Rust 1.99.0, cible `x86_64-pc-windows-msvc`, PDFium 7881).

## 0.19.0 : placement à la souris des champs existants (publiée en Latest stable)

Le lot retenu ajoute **Dessiner sur la page** à **Préparer un formulaire** pour texte, case, radio, liste déroulante
et zone de liste. La boîte numérique se masque pendant le tracé, puis se rouvre avec nom, choix et options conservés.
Le tracé seul ne modifie pas le PDF ; **Créer le champ** explicite réutilise validation/protection et une étape Annuler.
Rectangles finis bornés à la page initiale, minimum quatre points PDF par dimension. Glissement inverse accepté ;
gestes minuscules : mode dessin conservé pour réessayer. Échap annule sans écriture. Révision du document,
changement d'onglet/outil/espace de travail : placement périmé annulé ; zoom/navigation possibles, avec repli au
relâchement si la page initiale est masquée. Réticule/aperçu et exclusion des widgets accompagnent le tracé (D110/A38).

Implémentation et revue indépendante réussies. Workspace local : 460 réussis, zéro échec, sept ignorés par défaut ;
909 clés EN/FR correspondantes. Huit nouveaux tests UI/sauvegarde automatique et régression worker native des cinq types réussis.
Le contrôle qpdf local a réussi 75 vérifications ; autotest du metteur à jour réussi pour chemins/empreintes/rollback.
Quatre documents NIST/NBS licenciés (744 pages) ont réussi le contrôle local du corpus réel en 76,762 s, avec
un pic mémoire enfant échantillonné de 331 276 288 octets ; tous les fichiers source sont restés inchangés.
Formatage (`cargo fmt --all -- --check`) et Clippy toutes cibles avec `-D warnings` réussis.
[Publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.19.0) sous `lmdanavanem`, release
`415214602`, le 2026-10-16 à 17:35:42 UTC. Tag/source immuable : `903181fee3bba5794b39714106e5814bca753fb3`.
La [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38938525308) a réussi ses 32 étapes en 844 s,
dont les mêmes contrôles tests/traductions/qpdf, 17 cas portables (13 mises à niveau et quatre gardes), et onze
bases MSI avec 55 journaux d'installation vérifiés. Son corpus de quatre documents termine en 47,721 s, avec
un pic mémoire enfant échantillonné de 321 085 440 octets. Le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38938524836/job/117100003633)
a réussi en 680 s, upload et post-cache inclus ; le workflow en double a ensuite été annulé. Artefact ARM64
`12062064636` ; paquets x64 de la CI principale réussie et cet artefact ARM64 vérifié publiés.

Les six fichiers préparés/publics et les copies `dist/` concordent avec les empreintes API et sidecars SHA256.
Contrôles renforcés des paquets publics réussis : architectures, versions anglaises, images exactes d'icône,
OCR/modèles/licences, contenu MSI/ZIP, formulaires, chiffrement, signatures, compression, étiquettes, liens,
formes en tirets, encre groupée et OCR français hors ligne. Les contrôles indépendants de conversion passent
35 sous-processus avec les treize sources inchangées. Le metteur à jour de production a téléchargé le ZIP x64
Latest public (19 837 875 octets) et vérifié son SHA256 via le binaire de test du workspace déjà validé, sans
installation sur l'hôte. Les scénarios réels de mise à jour/installation portable et MSI ont réussi séparément
sur le runner Windows jetable de la CI.

| Fichier | Octets | SHA256 |
|---|---:|---|
| `SuniPDF-0.19.0-win-x64-portable.zip` | 19837875 | `C713DC4495D024E0BCBC715A29832FD5C49B164EFE165907496E8A1F9690E3BB` |
| `SuniPDF-0.19.0-win-arm64-portable.zip` | 18428126 | `D40E8EA991674FF790F356BA054D8FADC01CB5C596305FD99E5C33B13C712313` |
| `SuniPDF-0.19.0-win-x64.msi` | 19378176 | `E1CD1E6894B09A107238F96DFEFB5F7075E94D31DCC4FDFE3CCDF01100EE0EEE` |

Trois sidecars SHA256 complètent les six fichiers publiés. ProductCode MSI
`{AA82CB16-BEC5-40FD-83D3-C86852E44676}`, langue 1033. API moteur/CLI et 53 commandes inchangés ; aucune
dépendance ni type ajouté. Paquets toujours non signés. Souris/stylet/tactile natifs, Narrateur, imprimantes
physiques, matériel ARM64 et lecteurs tiers restent sans vérification manuelle. Le brouillon de champ reste
uniquement en mémoire jusqu'à **Créer le champ**. Si la vue simple masque sa page d'origine avant le relâchement,
vérifier le dernier rectangle connu dans la boîte rouverte avant de créer le champ.

## 0.18.0 : dessins à main levée groupés et modèles CLI lisibles (publiée ; Latest à cette date)

L'application regroupe plusieurs traits à main levée sur une page dans une seule annotation Ink, via
l'édition moteur/worker existante. Le brouillon dispose de Terminer/Annuler, d'un seul Annuler, de bornes
sur les traits/points et de gardes empêchant de perdre les traits lors d'une édition, sauvegarde, ouverture
d'un autre document ou fermeture. Un échec moteur conserve le brouillon pour réessayer ou annuler.
L'anglais reste la langue par défaut, avec contrôles français équivalents. Implémentation et validation bureau réussies. Jusqu'à 64 traits et 4 000 points par trait sont conservés,
avec messages de limite visibles. Couleur, épaisseur et opacité s'appliquent au brouillon entier. Navigation,
zoom et modes de vue conservent la page d'origine ; la finalisation globale au relâchement évite un trait
bloqué après un saut en vue simple. Les changements de formulaire et marqueurs de récupération différés
doivent finir avant de dessiner. Le brouillon reste uniquement en mémoire jusqu'à Terminer, sans récupération
après plantage.

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

Les seize cas portables couvrent douze mises à niveau, dont 0.17.0, et quatre gardes. Dix bases MSI :
0.8.0, 0.9.1, 0.10.0 à 0.17.0 ; 50 journaux install/rollback/upgrade/downgrade/uninstall vérifiés.
Le job ARM64 a réussi en 612 s, upload et post-cache inclus ; le workflow en double a ensuite été annulé.
Artefact ARM64 `12056658063`. Publication le 2026-10-16 à 16:03:33 UTC.

Les cinq relectures qpdf indépendantes confirment une annotation Ink et trois traits, dont un point, avec
longueurs InkList `[6,4,2]`, pour trois sorties clair/AES-128/AES-256 relues avec les deux mots de passe.
Les contrôles de conversion publics incluent 35 sous-processus EXIF/transparence/budget/DPI et 13 sources intactes.

| Fichier | Octets | SHA256 |
|---|---:|---|
| `SuniPDF-0.18.0-win-x64-portable.zip` | 19834488 | `8D1C76930C7D929D289745DC20897C91418DF63873B6E66B2566CA6AF289A08C` |
| `SuniPDF-0.18.0-win-arm64-portable.zip` | 18422022 | `89A814C2395CAD59D761B02CC8ACC7C9E2629B23FA8D3DB22D5B7C1AB90815FB` |
| `SuniPDF-0.18.0-win-x64.msi` | 19378176 | `D3DABCB44438E4E005D33A5D4544ACD6C3514DE10BF7BB07B745123E4CD7DDC4` |

Trois sidecars SHA256 complètent les six fichiers ; copies `dist/` vérifiées indépendamment contre API,
public et préparation. ProductCode MSI `{2C188A13-8DBB-475A-B48D-78BCC8D058EA}`, langue 1033.

Stylet/tactile natifs, boîtes interactives, imprimantes, Narrator, matériel ARM64 et lecteurs tiers restent non vérifiés. Paquets non signés ; brouillons d'encre non validés sans récupération après plantage.

## 0.17.0 : conversion JPEG orientée sans recompression et contrôle des lots raster (publiée ; Latest à cette date)

[SuniPDF 0.17.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) était stable **Latest** à cette date sous
`lmdanavanem`, release ID `415074108`, avec six fichiers. Tag/source immuable :
`5785de282e126fb4666b28fb83340d0e9c6e9acf`. La [CI Windows x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
ont réussi. Le workflow de release en double a été annulé après le succès complet ARM64, upload et post-cache inclus ;
les paquets x64 de la CI principale et l’artefact ARM64 vérifié ont été utilisés. Les anciennes releases restent disponibles.

Images vers PDF incorpore désormais les JPEG sans décodage des pixels ni recompression pour les huit orientations
EXIF, y compris les rotations avec miroir 5 et 7. Les dimensions de page suivent l’image orientée. L’API moteur
partagée et les appels bureau/CLI existants sont conservés ; les formats décodés gardent leur transparence.

Avant de créer un bitmap, la conversion contrôle tous les en-têtes et refuse un lot dont les images non-JPEG ont
un coût raster cumulé estimé dépassant 512 Mio. Chacune compte le maximum entre ses octets décodés annoncés et largeur ×
hauteur × 4, avec arithmétique vérifiée. Le décodeur réellement chargé est contrôlé de nouveau avant décodage.
Ce n’est pas un plafond mémoire du processus : tampons natifs PDFium, métadonnées JPEG encodées, copies temporaires
et PDF sérialisé restent hors estimation. Les dimensions physiques des pages doivent être finies et positives.

Dépassement ou overflow du budget renvoie `image_batch_too_large` ; le bureau affiche une erreur EN/FR exploitable
et conserve le document courant et ses modifications non enregistrées. Gardes de sortie et écritures atomiques
existantes conservées. Aucune dépendance, option, commande ou outil ajouté ; toujours 53 commandes CLI et
896 clés de traduction EN/FR concordantes.

Validation locale réussie : 445 tests workspace, aucun échec, sept ignorés par défaut ; formatage, Clippy sans
avertissement, 896 clés EN/FR concordantes, 74 contrôles qpdf indépendants et quatre documents NIST/NBS sous
licence (744 pages ; 48,187 secondes, pic mémoire échantillonné du processus enfant 330 264 576 octets).
Self-test de l’assistant de mise à jour réussi. Revue indépendante sans blocage. Tests natifs/CLI : huit orientations,
flux JPEG exacts, coins asymétriques, repli EXIF malformé, transparence, bornes cumulées/overflow/en-têtes,
DPI extrêmes, entrées inchangées et gardes de sortie. Le test de réponse bureau conserve le document modifié au refus.

Un contrôle indépendant Python/Pillow/qpdf passe sur la CLI debug : neuf comparaisons de flux JPEG exacts,
32 coins JPEG orientés, quatre coins PNG avec masque alpha, trois contrôles qpdf, deux refus de budget sur
en-têtes seuls avant décodage, dix refus de DPI invalides et géométrie positive/finie après sauvegarde au DPI f32
maximum. Les 13 empreintes d’entrée restent inchangées. Ce résultat debug ne prouve ni paquet publié ni GUI manuelle.

La CI Windows passe les mêmes contrôles de tests/traductions/qpdf. Son corpus de quatre documents termine en
45,791 secondes, avec un pic mémoire échantillonné du processus enfant de 321 413 120 octets. Les 15 scénarios
portables réels réussissent : mises à jour depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0,
0.14.0, 0.15.0 et 0.16.0, plus empreinte incorrecte, mauvaise architecture, restauration après verrouillage et
attente réelle de 120 secondes avec garde contre une mise à jour concurrente. Les neuf baselines MSI (0.8.0,
0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0, 0.14.0, 0.15.0, 0.16.0) passent installation, restauration transactionnelle,
mise à jour, refus du downgrade, fichiers/données utilisateur et désinstallation sur runners Windows jetables.
Les 45 journaux MSI terminent avec le code attendu 0 ou 1603. Preuves dans le journal principal et les artefacts
portable/MSI/corpus téléchargés.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 825 743 | `55D8CBC224D1D6D9582AD9402F51B7F76CEC84C93A0FC90AFE91C43471ACF1D4` |
| ZIP portable ARM64 | 18 413 678 | `CBFA86B5D1EEE8B83A82EE97D4105BCE7539E19A04CB6916938D5678FBB373E4` |
| MSI x64 | 19 369 984 | `C59DE003132EFC077B6D5F0C70B7FC8DAE06616D6F4E4B12DDD45BCB479A603E` |

Les six fichiers préparés passent empreintes, architectures, versions/icônes anglaises, OCR/licences, contenu
MSI/ZIP et contrôles des fonctions existantes. Les contrôles indépendants de conversion passent sur la CLI x64
du paquet : flux JPEG exacts, coins orientés, masques PNG, refus budget/DPI et entrées/sorties préservées.
Les six téléchargements publics sans authentification concordent avec les empreintes API, fichiers SHA256 et
paquets préparés ; les contrôles renforcés passent de nouveau sur ces fichiers. Copies vérifiées dans `dist/`.
L’updater de production a téléchargé et vérifié le ZIP x64 Latest de 19 825 743 octets sans installation sur ce poste.

Dialogues natifs interactifs, matériel ARM64, imprimantes physiques, Narrator et lecteurs tiers restent non vérifiés ;
les paquets ne sont pas signés. Le budget raster de 512 Mio ne plafonne pas la mémoire native/totale du processus.
Les réécritures PDF complètes peuvent invalider les signatures numériques ; l’avertissement existant reste présent.

## 0.16.0 : annotations de forme en pointillés (publiée en Latest à cette date)

[SuniPDF 0.16.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) était stable **Latest** à cette date sous
`lmdanavanem`, release ID `414934190`, avec six fichiers. Tag/source immuable :
`6cd7843734f3924e87033d8b05539d493ccbcb95`. La [CI Windows x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589)
ont réussi. Le workflow de release en double a été annulé après le succès complet du job ARM64 ;
les paquets x64 de la CI principale et l’artefact ARM64 vérifié ont été utilisés. Les anciennes releases restent disponibles.

Cette version ajoute un contrôle Bordure en tirets pour les rectangles, ellipses, lignes et flèches, et `annotate --dashed`
avec le même motif de 6 points de trait / 3 points d'espace. Le plein reste la valeur par défaut. Le moteur
conserve le motif réel pour que sélection, recoloration, déplacement, redimensionnement et Annuler/Rétablir
le préservent ; seul un changement explicite de la case remplace un motif personnalisé. Dictionnaires de contour
et flux d'apparence utilisent le même motif et une phase nulle ; les pointes de flèche restent pleines et les
fonds continus. Les types d'annotation non pris en charge refusent un motif non vide.

Lecture des motifs et génération des apparences sont bornées ; les styles de contour malformés ou non pris en
charge des annotations créées par l'app refusent les modifications régénératrices au lieu de devenir pleins.
Aucune dépendance, commande ou outil ajouté. L'anglais reste la langue par défaut, le français reste disponible.
La CLI compte toujours 53 commandes.

Validation locale réussie : 439 tests du workspace, aucun échec, sept ignorés par défaut ; formatage, Clippy sans
avertissement, 895 clés EN/FR concordantes, 73 contrôles qpdf indépendants et quatre documents NIST/NBS sous
licence (744 pages ; 53,542 secondes, pic mémoire échantillonné du processus enfant 331 517 952 octets).
La CI Windows passe les mêmes contrôles de tests/traductions/qpdf ; son corpus réel termine en 47,628 secondes,
avec un pic mémoire échantillonné du processus enfant de 321 306 624 octets. Les tests natifs couvrent les motifs
personnalisés/historiques/par défaut/vides, les bornes numériques et de densité, les pages tournées/recadrées,
les refus atomiques, les formulaires/liens/contenus/sources inchangés et les deux rôles AES-128/256.
Les tests du worker réel et les contrôles accessibles EN/FR sans fenêtre couvrent création, sélection,
conservation du motif personnalisé, style, déplacement, redimensionnement et Annuler/Rétablir. Ces tests
ne prouvent pas une interaction manuelle avec les dialogues natifs.

Quatorze scénarios portables réels ont réussi : mises à jour depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0,
0.12.0, 0.13.0, 0.14.0 et 0.15.0, plus empreinte incorrecte, mauvaise architecture, restauration après
verrouillage et attente réelle de 120 secondes avec garde contre une mise à jour concurrente. Les huit baselines
MSI (0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0, 0.14.0, 0.15.0) passent installation, restauration
transactionnelle, mise à jour, refus du downgrade, contrôles des fichiers/données utilisateur et désinstallation
sur runners Windows jetables. Les 40 journaux MSI terminent avec le code attendu 0 ou 1603 pour les échecs
injectés. Les preuves figurent dans le journal principal et les artefacts portable/MSI/corpus téléchargés.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 821 719 | `F03B7BB283A58414F1F26FD707FE120231B1B9502762B5588F75C97DC4526DD0` |
| ZIP portable ARM64 | 18 408 941 | `ECD3BE771391105DDC119E8F5A59415349C1B1EE9C1A393773DCDE9EBC67A75E` |
| MSI x64 | 19 365 888 | `5B4043B271C36C2CBBFBFD3FAA419ED96CB2D151CD32250C43C8DC121FB1328D` |

Les six fichiers préparés passent empreintes, architectures, versions/icônes anglaises, OCR/licences,
contenu MSI/ZIP et contrôles des fonctions existantes. Les bordures en tirets des paquets couvrent quatre
formes sur PDF simples et AES-128/256 ouverts en propriétaire, les deux mots de passe, 20 contrôles qpdf
indépendants, 20 paires de rendus plein/tirets distincts et les empreintes des entrées inchangées.
Les six téléchargements publics sans authentification concordent avec les empreintes API, les fichiers SHA256
et les paquets préparés ; les contrôles renforcés passent de nouveau sur ces fichiers. Les copies vérifiées sont
dans `dist/`. L’updater a téléchargé et vérifié le ZIP x64 Latest de 19 821 719 octets sans installation.

Dialogues natifs interactifs, matériel ARM64, imprimantes physiques, Narrator et lecteurs tiers restent non vérifiés ;
les paquets ne sont pas signés. Création de polygones/polylignes et bordures avancées de texte libre restent à faire.
Les réécritures PDF complètes peuvent invalider les signatures numériques ; l’avertissement existant reste présent.
Les étiquettes de page restent liées aux positions après modification structurelle.

## 0.15.0 : modification des destinations de liens existants (publiée en Latest à cette date)

[SuniPDF 0.15.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) était stable **Latest** à cette date sous
`lmdanavanem`, release ID `414866395`, avec six fichiers. Tag/source immuable :
`952f8699cac451e7c3004b6d816105d52e41f249`. La [CI Windows x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541)
ont réussi. Le workflow de release en double a été annulé après le succès complet du job ARM64 ;
les paquets x64 de la CI principale et l'artefact ARM64 vérifié ont été utilisés. Les anciennes releases sont conservées.

Cette version ajoute `set-link-target` et une action Modifier la destination pour les liens existants. Les adresses
HTTP/HTTPS/mailto sûres et les destinations de page entière remplacent `/A`, `/Dest` et `/AA`, en conservant
l'indice, le rectangle, l'apparence, les commentaires/réponses et les propriétés sans rapport. Les destinations
nommées/détaillées sont résolues pour le préremplissage si disponibles ; le dialogue explique leur remplacement
par une destination de page entière. Les flags en lecture seule, verrouillés, au contenu verrouillé ou malformés
sont refusés, y compris les flags indirects.

Le bureau réutilise son dialogue de lien, propose un seul Annuler/Rétablir préservant la géométrie et ferme les
dialogues périmés après modification ou changement d'onglet. L'anglais reste la langue par défaut, le français
reste disponible ; aucune dépendance ajoutée. La CLI publiée compte 53 commandes. Sélectionner un lien avec
l'outil de sélection d'annotation, puis Modifier la destination ou double-cliquer. La création de destinations
avec coordonnées/vue précises reste à faire ; une page cible utilise l'affichage de la page entière.

Validation locale et CI Windows réussie : **429 tests, aucun échec, sept ignorés par défaut**, formatage,
Clippy sans avertissement, **894 clés EN/FR concordantes**, **68 contrôles qpdf indépendants** et quatre documents
NIST/NBS sous licence (**744 pages**). Corpus local : 48,891 secondes, pic mémoire échantillonné du processus
enfant 328 736 768 octets. Corpus CI : 32,955 secondes, 321 470 464 octets. Les tests natifs couvrent les
annotations directes/indirectes, les flags malformés/verrouillés, les cibles refusées, les formulaires/contenus/rendus/
sources inchangés et les deux rôles AES. Le chargeur de test fournit le mot de passe pendant le parsing des flux
d'objets chiffrés ; les apparences sont générées avant l'instantané de référence strict. Les tests du worker réel
et sans fenêtre EN/FR couvrent Annuler/Rétablir, les contrôles accessibles réellement cliqués, la validation,
le préremplissage résolu, l'annulation et les dialogues périmés.

Treize scénarios portables réels ont réussi : mises à jour depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0,
0.12.0, 0.13.0 et 0.14.0, plus empreinte incorrecte, mauvaise architecture, restauration après verrouillage et
attente réelle de 120 secondes avec garde contre une mise à jour concurrente. Les sept baselines MSI (0.8.0,
0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0, 0.14.0) passent installation, restauration transactionnelle, mise à jour,
refus du downgrade, contrôles des fichiers/données utilisateur et désinstallation sur runners Windows jetables.
Les 35 journaux MSI terminent avec le code attendu 0 ou 1603 pour les échecs injectés. Les preuves figurent dans
le journal du job principal et les artefacts portable/MSI/corpus téléchargés.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 803 226 | `CFC093316717596EE4BD3944DB6108A5512AC191F2D37782F55B4A389CE7E733` |
| ZIP portable ARM64 | 18 392 243 | `BA09DD4C57F0449AAFF767E7B2E77A894FA298576C3843A33F5334017C2A493D` |
| MSI x64 | 19 353 600 | `119C4725324AF01FC1245D685604C9A0927773BB2D08E48C4FEA70AFF230CCE7` |

Les six fichiers préparés passent empreintes, architectures, versions/icônes anglaises, OCR/licences,
contenu MSI/ZIP et contrôles des fonctions existantes. Le remplacement de liens couvre les PDF simples et
AES-128/256 ouverts en propriétaire, les deux mots de passe, l'identité/le rectangle stables, dix contrôles
qpdf, dix comparaisons PNG identiques et les empreintes inchangées de toutes les entrées des commandes.
Les six téléchargements publics sans authentification concordent avec les empreintes API, les fichiers SHA256
et les paquets préparés ; les contrôles renforcés passent de nouveau sur ces fichiers. Les copies vérifiées sont
dans `dist/`. Le mécanisme de mise à jour a téléchargé et vérifié le ZIP x64 Latest de 19 803 226 octets sans installation.

Dialogues natifs interactifs, matériel ARM64, imprimantes physiques, Narrator et lecteurs tiers restent non vérifiés ;
les paquets ne sont pas signés. Remplacer les actions n'efface pas sûrement les anciens objets PDF orphelins.
Les réécritures complètes peuvent invalider les signatures numériques ; l'avertissement existant reste présent.
Les étiquettes de page restent liées aux positions après modification structurelle.

## 0.14.0 : création d'étiquettes de page et historique (publiée en Latest à cette date)

[SuniPDF 0.14.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) était stable **Latest** à cette date sous
`lmdanavanem`, release ID `414812776`, avec six fichiers vérifiés. Tag/source immuable :
`af532888804c196aabe216b8f6a2980c3d1e9e2d`. La [CI Windows x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
ont réussi. Le build x64 en double et la publication automatique en brouillon ont été annulés après le succès
ARM64 ; les paquets x64 de la CI principale et l'artefact ARM64 ont été utilisés.

Pages / Organiser ouvre Étiquettes de page : plage physique inclusive, numérotation décimale/romaine/par lettres,
préfixe facultatif et numéro initial positif, ou préfixe seul. Les étiquettes servent à la navigation sans modifier
le contenu imprimé ; `#N` conserve l'accès physique si elles sont ambiguës. La CLI ajoute `set-page-labels` et
`reset-page-labels`, pour 52 commandes publiées. Les protections existantes contre l'écrasement sont conservées.

Les règles hors plage sont conservées, avec reprise de leur numéro après la plage. Préfixes limités à 256 octets
UTF-8 et étiquettes produites à 4096 octets avant chargement du candidat. Parcours des objets décodés borné en
profondeur/nœuds/références/longueurs ; refus des règles malformées ou non prises en charge. Réinitialiser tout
retire l'entrée du catalogue, même malformée ; ce n'est pas un effacement sécurisé. Le candidat et ses métadonnées
sont chargés avant modification de l'état actif. Mots de passe, chiffrement, permissions, contenu, formulaires et
liens utilisent l'écriture commune. Une réécriture peut invalider les signatures numériques ; l'avertissement de
sauvegarde existant reste en place. Étiquettes liées aux positions après déplacement/insertion/suppression :
le recalage sémantique n'est pas implémenté (D104).

Undo mémorise si l'opération change la géométrie/l'ordre des pages. Undo/Redo des étiquettes et éditions de contenu
conserve l'historique de lecture ; les opérations structurelles l'invalident. Le dialogue se ferme après changement
d'onglet/document/structure (D105). Anglais par défaut, français disponible ; aucune dépendance ajoutée.

Validation locale et CI Windows réussie : **420 tests, aucun échec, sept ignorés par défaut**, formatage,
Clippy sans avertissement, **890 clés EN/FR identiques**, **63 contrôles qpdf indépendants**, **quatre PDF NIST/NBS
sous licence (744 pages)**. Corpus local : 53,91 secondes, pic enfant échantillonné 332 460 032 octets ;
corpus CI : 51,89 secondes, 321 134 592 octets. Tests natifs : six styles, arbres malformés/imbriqués, reprise
hors plage, étiquettes par défaut/vides, protection, rendu/texte inchangés et sauvegarde/reset/réouverture.
Worker réel/tests UI sans fenêtre : Apply/Reset, entrées invalides, dialogues périmés et Undo/Redo selon géométrie.

Douze scénarios portables réels réussis : mises à niveau depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0,
0.12.0 et 0.13.0, plus mauvaise empreinte, mauvaise architecture, restauration après fichier verrouillé et attente
réelle de fermeture 120 secondes/garde contre les mises à jour concurrentes. Installation MSI, rollback
transactionnel, mise à niveau, refus de downgrade, empreintes des fichiers, désinstallation et conservation des
données réussis depuis 0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0 et 0.13.0 sur runners Windows jetables. Preuves :
journal CI, 12 résultats portables, 30 journaux MSI et rapport du corpus réel.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 785 200 | `9A2A085FDCC0E30852A2F72755705CB42EE2F5C866EF9A4E00ADCEEA9C6D6960` |
| ZIP portable ARM64 | 18 382 324 | `20CE7CA3C04495548FA45CADDB568266B52C574D6292A2A35A7DDAE0CAD476D0` |
| MSI x64 | 19 345 408 | `838DA744E862C840B4000678FC6E4757B3EFD85DBDB5910764178334EFFB5FBA` |

Les six téléchargements publics non authentifiés correspondent aux empreintes API, fichiers SHA256 et paquets
CI ; copies vérifiées dans `dist/`. Les binaires préparés/publics ont passé architecture, version/métadonnées
anglaises, icône exacte, licences/OCR, contenus MSI/ZIP et fonctions existantes. Nouveaux contrôles : plages,
réinitialisation, Unicode et règles hors plage sur PDF simples et AES-128/256 ouverts en propriétaire, les deux
mots de passe, qpdf, texte lisible, PNG identiques pour les deux pages de test et empreintes source inchangées.
Le mécanisme de mise à jour a téléchargé et vérifié le ZIP x64 Latest de 19 785 200 octets, sans installation.

Dialogues interactifs natifs, matériel ARM64, impression physique, Narrator et lecteurs tiers restent non vérifiés ;
paquets non signés. Les installations automatisées CI ne prouvent pas l'usage interactif sur matériel réel.
Les réécritures de métadonnées et instantanés Undo augmentent la mémoire pour les gros documents.

## 0.13.0 : signatures visuelles saisies et multipages (Latest à cette date)

[SuniPDF 0.13.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) a été publiée en stable **Latest** sous
`lmdanavanem`, release ID `414763678`, avec six fichiers vérifiés. Tag/source immuable :
`b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. La [CI Windows x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
ont réussi. Le build x64 en double et la publication automatique en brouillon ont été annulés après le succès
ARM64 ; les paquets x64 de la CI principale et l'artefact ARM64 ont été utilisés. Aucun ancien tag ou fichier publié
n'a été remplacé.

Création d'une signature visuelle ou d'un paraphe en saisissant une ligne, avec une police standard ou TrueType
installée et une encre noire/bleue, puis aperçu explicite en PNG transparent natif. Times italique est le défaut.
Toute modification du texte, de la police, de l'encre ou du mode invalide l'ancien aperçu ; changer de document
écarte les réponses en attente. La bibliothèque locale facultative conserve son format PNG non chiffré, sans
preuve d'identité. La signature CMS/PFX par certificat reste une fonction distincte déjà existante. Limites :
256 caractères, 1600 pixels par côté ; contrôle des glyphes et restrictions d'incorporation existantes.
Aucune nouvelle dépendance.

Choix entre page courante, toutes les pages ou sélection capturée dans Organiser. Le lot valide toutes les cibles,
travaille sur un document temporaire et valide une seule fois, avec annulation coopérative et un seul Undo/Redo.
L'aperçu et la pose partagent une géométrie ajustée : proportions et taille physique conservées, réduction seulement
pour une page plus petite. Même centre en points PDF affichés sur chaque cible ; les modifications structurelles
écartent les cibles capturées. `add-image --pages` est un ajout compatible avec la commande et le JSON monopages.
Annulation/erreur sans lot partiel ni ancien rappel de placement ; la sauvegarde reste explicite.

Contrôles locaux finaux réussis : **410 tests, aucun échec, sept ignorés par défaut**, formatage, Clippy sans
avertissement, **870 clés EN/FR identiques**, **53 contrôles qpdf indépendants**, **quatre PDF NIST/NBS sous licence
(744 pages)** en 76,40 secondes. Mémoire de pointe échantillonnée des processus enfants : 331 522 048 octets.
Les tests natifs couvrent aperçu/polices/transparence, recadrage/rotation/protection, annulation, sauvegarde/réouverture
et qpdf. Le worker réel vérifie aperçu sans mutation, progression et Undo/Redo unique ; les tests applicatifs sans
fenêtre vérifient sélection, réponses périmées, invalidation structurelle, fermeture et messages de réussite/échec.

Onze scénarios portables réels réussis : mises à niveau depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0 et 0.12.0,
plus mauvaise empreinte, mauvaise architecture, restauration après fichier verrouillé et attente réelle de fermeture
120 secondes/garde contre les mises à jour concurrentes. Installation MSI, rollback transactionnel, mise à niveau,
refus de downgrade, empreintes des fichiers, désinstallation et conservation des données réussis depuis 0.8.0,
0.9.1, 0.10.0, 0.11.0 et 0.12.0 sur runners Windows jetables. Corpus réel CI : 32,09 secondes, pic enfant
320 385 024 octets.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 730 578 | `B5CC0CBAB1A1ABE4CFF346093A984760ECE458C2986CBF650C6EC3769C2A3FDD` |
| ZIP portable ARM64 | 18 320 921 | `C14B8F70E21ED3A811556868A267B1A862B31E55EBF71C6044EFF7C3B06F069D` |
| MSI x64 | 19 292 160 | `0727828749D235CCCD52BE6440C34F980F59FCABC7AB2B1027D2A77228D65C63` |

Les six téléchargements publics non authentifiés correspondent aux empreintes API, aux fichiers SHA256 et aux
paquets CI ; copies vérifiées dans `dist/`. Les binaires préparés et publics ont passé les contrôles architecture,
métadonnées/version anglaises, sept images d'icône exactes, licences/OCR, contenus MSI/ZIP et fonctions existantes
(formulaires/compression/transformation groupée/copies protégées). Nouveaux contrôles : pose ajustée de signatures
sur PDF simples et AES-128/256 ouverts avec mot de passe propriétaire, réouverture avec les deux mots de passe,
qpdf, texte lisible et originaux inchangés. Le mécanisme de mise à jour a téléchargé et vérifié le ZIP x64 Latest
de 19 730 578 octets, sans installation. Le contrôle de pose utilise les dimensions de chaque page d'origine,
y compris la page paysage du PDF de test.

Dialogues interactifs natifs, matériel ARM64, impression physique, Narrator et lecteurs tiers restent non vérifiés ;
les paquets restent non signés. Instantanés et document temporaire augmentent la mémoire de pointe ; un appel natif
de page ne peut pas être interrompu en cours d'exécution. Les installations automatisées CI ne prouvent pas l'usage
interactif sur matériel réel.

## 0.12.0 : création de formulaires et exports radio (Latest à cette date)

[SuniPDF 0.12.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) a été publiée en stable **Latest** sous
`lmdanavanem`, release ID `414674886`, avec six fichiers vérifiés. Tag/source immuable :
`3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. La [CI Windows x64 finale](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688)
et le [job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878)
ont réussi. Le build x64 en double et la publication automatique en brouillon ont été annulés après le succès
ARM64 ; les paquets x64 de la CI principale et l'artefact ARM64 ont été utilisés.

Création de listes déroulantes à saisie libre dans Préparer un formulaire ou avec
`add-form-field --kind combo --editable`, en conservant des choix prédéfinis. Les API existantes gardent leur
comportement par défaut. Les valeurs radio `/Opt` restent distinctes des noms d'apparence et servent dans
l'application, JSON/XFDF et l'import FDF. Les widgets invisibles conservent leurs indices d'origine.
Les informations CLI ajoutent `widgets[].export_value`.

Un mapping malformé ou ambigu, des références de widgets dupliquées ou des noms d'apparence non UTF-8 rendent
uniquement le champ radio concerné non modifiable. Un export inchangé, même une chaîne vide ou réservée, peut
être réimporté. Les tests contrôlent `/V`, `/AS` et les clés d'apparence après une autre modification. PDFium peut
régénérer une apparence manquante. Les radios sans `/Opt` gardent leur comportement ; la modification du flag
éditable d'un champ existant reste à réaliser.

Contrôles locaux finaux réussis : **398 tests workspace, aucun échec, sept ignorés par défaut**, formatage,
Clippy sans avertissement, **859 clés EN/FR correspondantes**, **48 contrôles qpdf indépendants** et les
**quatre documents NIST/NBS sous licence (744 pages)** en 59,68 secondes. Couverture : création headless EN/FR,
création/remplissage sur le vrai worker avec Annuler/Rétablir, CLI avec sauvegarde/relecture, pages cadrées/pivotées
et protégées, ouverture propriétaire AES-128/256 et refus des mappings malformés.
Les contrôles de mise à jour portable/MSI incluent maintenant les paquets publics 0.11.0 vérifiés indépendamment.

Dix scénarios portables réels ont réussi : mises à jour depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0 et 0.11.0,
empreinte incorrecte, mauvaise architecture, retour arrière sur fichier verrouillé et vrai délai de fermeture de
120 secondes avec verrou de mise à jour concurrente. Installation MSI, retour arrière transactionnel, mise à jour,
refus de retour de version, empreintes, désinstallation et conservation des données ont réussi depuis 0.8.0,
0.9.1, 0.10.0 et 0.11.0 sur des runners Windows jetables.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 699 323 | `AD1ECB72A999F1A43602E58AD2634F9E80FD7925C90A90B9AD25FB9B6BDC1B83` |
| ZIP portable ARM64 | 18 290 932 | `567362FB988C9C2C7FF60B67221BAC8D6901E05EF6F6FA25CB8CB1ADE47CD358` |
| MSI x64 | 19 267 584 | `390A20B7B5999635FDC6F2FE99ED4BD25DE53EAB4623B820FEC2F9AF04AA0C89` |

Les six téléchargements publics sans authentification correspondent aux empreintes API, aux fichiers SHA256 et
aux paquets CI ; les copies vérifiées sont dans `dist/`. Les binaires préparés puis publics ont passé les contrôles
d'architecture, version/métadonnées anglaises, icône exacte, licences/OCR, contenu MSI/ZIP, compression,
formulaires/XFDF, transformations groupées et copies protégées. Les nouveaux contrôles couvrent les combos éditables
créés et une fixture radio `/Opt` produite par QPDF, en clair et en AES-128/256 ouvert par le propriétaire, avec
validation qpdf indépendante. Les sources sont restées inchangées. Le vrai module de mise à jour a téléchargé
et vérifié le ZIP x64 Latest de 19 699 323 octets sans installation.
Dialogues natifs interactifs, matériel ARM64, impression physique, Narrator et lecteurs tiers restent non vérifiés ;
les paquets restent non signés. Une installation CI scriptée ne constitue pas une validation interactive matérielle.

## 0.11.0 : navigation, formulaires et copies protégées (publiée en Latest à cette date)

[SuniPDF 0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) a été publiée stable **Latest** sous `lmdanavanem`,
release ID `414626617`, avec six fichiers. Tag/source immuable : `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`.
[CI Windows x64 finale](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) et
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) réussis.
Le build x64/publication automatique en doublon a été annulé après réussite ARM64 ; les artefacts x64 de la CI main
réussie et l’artefact ARM64 ont servi à publier. Auteur, tag, empreintes des fichiers et état API Latest stable vérifiés.

Neuf scénarios portables réels réussis : mises à jour depuis 0.8.2, 0.8.3, 0.9.0, 0.9.1 et 0.10.0, checksum invalide,
mauvaise architecture, rollback sur fichier verrouillé et délai réel de fermeture/mutex. Installation MSI réelle,
rollback transactionnel, mise à jour, refus du downgrade, empreintes des fichiers, désinstallation et conservation
des données réussis depuis 0.8.0, 0.9.1 et 0.10.0 sur runners Windows jetables. Tests automatisés isolés, pas preuve
interactive matérielle. Les six fichiers préparés passent architecture/version/icône/licences/OCR et égalité MSI/ZIP ;
binaires x64 vérifiés sur libellés, copies/métadonnées AES-128/256 ouvertes propriétaire, deux mots de passe, qpdf/texte
lisible, compression, formulaires/XFDF, transformations groupées et OCR français. Les originaux sont restés inchangés.

| Paquet | Octets | SHA256 |
|---|---:|---|
| ZIP portable x64 | 19 687 375 | `6A08A518619D89D450332F8AE1BA31FB336C3DF88910D5471183B25C173B2421` |
| ZIP portable ARM64 | 18 279 249 | `8A6B958F2211BC95666DE10342E3338BBE80170667BBBB1D5DBD012584D04C33` |
| MSI x64 | 19 255 296 | `E99FE4416DA5E55086059884CCBE770EA1D9AB437D8E3BE26D49D696B5C9C4BD` |

Les six téléchargements publics sans authentification correspondent aux empreintes API, fichiers SHA256 et paquets CI de même source ; copies vérifiées dans `dist/`. Contrôles renforcés réussis sur les binaires publics, dont contenu/métadonnées AES-128/256 ouverts propriétaire. L'updater de production télécharge et vérifie le ZIP x64 Latest de 19 687 375 octets sans installation. Paquets non signés.
ARM64 réel, imprimantes physiques, Narrateur, dialogues Windows interactifs et lecteurs tiers non vérifiés.

- Libellés PDF en cache, navigation par libellé exact unique et positions physiques explicites `#N` ; miniatures,
  statut, accessibilité et ajout CLI `info.pages[].label`. Un libellé illisible, contenant des contrôles ou dépassant
  4096 octets utilise la position physique. Création de libellés et recalage sémantique après réorganisation non implémentés.
- Destinations locales XYZ/Fit/FitH non nul pour liens et signets, avec CropBox/rotation PDF de la page cible puis
  rotation de lecture. Retour/avance conserve positions sur une même page et zoom, même lors de sauts rapides.
  La sélection de texte laisse désormais passer les clics sur les liens. Confirmation des URI conservée ; actions
  distantes, embarquées, Launch et JavaScript non suivies.
- Échecs de lecture typés des formulaires : textes, choix, saisies en place et actions en attente conservés, validation
  sur modèle périmé bloquée, réessai explicite et aucune boucle de relecture. Un formulaire vide confirmé reste valide.

Contrôles locaux finaux du lot élargi 0.11.0 réussis : **390 tests workspace, zéro échec, sept ignorés par défaut**,
formatage, Clippy sans avertissement et **858 clés EN/FR identiques**. qpdf valide **42 PDF distincts (44 contrôles selon
le mot de passe)**, avec refus des entrées malformées. Les **quatre documents NIST/NBS sous licence, 744 pages**, passent
en 91,59 secondes ; pic mémoire enfant échantillonné 329 347 072 octets. Les tests couvrent panneau/saisie en place des
combos éditables, valeurs d’export ambiguës, première saisie clavier, échecs/réessais, huit cas cadres/rotations/chiffrement,
JSON/XFDF, extraction/découpage protégés, deux mots de passe, RC4/AES-128/AES-256, préservation source/collisions,
sorties du vrai worker et rotations non enregistrées. CI Windows, ARM64 de même source, paquets préparés et publication réussis ; contrôles publics et updater de production réussis.

Avant publication, qpdf sur le candidat `1714846` a détecté des flux AES-128 illisibles après ouverture avec un
mot de passe propriétaire distinct. Ces paquets n’ont jamais été publiés. lopdf est fixé à la révision officielle
`6ce451328c5caa65d50462daaf4ee8db9c85bb8e`, avec récupération de clé/encodage corrigés et licence MIT incluse.
Les résultats locaux finaux ci-dessus couvrent cette dépendance corrigée et les deux rôles à l’ouverture, y compris
texte/flux déchiffrés, métadonnées et déprotection. Builds Windows et contrôles des paquets doivent utiliser cette
source corrigée `0b7a22b2cf293c2908e8e6c6107c3f89bde01862` ; les anciens artefacts du candidat ont été exclus.
Tranche navigation/lecture des formulaires antérieure au lot élargi 0.11.0 : **383 tests workspace réussis, zéro échec, sept ignorés par défaut** ; formatage, Clippy sans
avertissement et **857 clés EN/FR** validés. Le contrôle qpdf explicite valide **34 PDF** et refuse ses entrées malformées.
Les **quatre documents réels sous licence (744 pages)** passent en 63,01 secondes ; pic mémoire enfant échantillonné
328 953 856 octets. Tests moteur/CLI : destinations nommées/directes/actions, rotations/cadres, AES-256 et relecture.
Tests UI sans fenêtre native : libellés, clics sur liens, historique avec zoom sur une même page, enchaînements rapides
Retour/lien et lectures de formulaire échouées. Les vrais binaires GUI/CLI de développement locaux sont construits.
La CI GitHub reste distincte. Source corrigée 0.11.0 publiée ; fichiers historiques 0.10.0 inchangés. Usage natif interactif, Narrateur
et lecteurs tiers non vérifiés.

Le lot élargi ajoute la saisie libre dans les listes déjà éditables, par panneau ou sur la page, avec conservation
des valeurs d’export lors de libellés ambigus, lectures échouées et réessais. Extraits et parties découpées conservent
mots de passe, clé de chiffrement et permissions. La source ne peut être remplacée ; sans écrasement, les parties
utilisent une publication atomique sans remplacement. Une source non protégée reste non protégée ; la politique de
permissions ne change pas. Un échec peut laisser des parties déjà terminées, complètes et protégées. La CLI conserve
`encryption_removed: false` et ajoute `encrypted`. Les tests de mise à jour incluent les bases portable/MSI 0.10.0.
Libellés et rotations partagent une seule lecture mise en cache. Une rotation de page met à jour les seules entrées
concernées ; une suppression groupée valide les indices avant mutation et relit les métadonnées une fois.
Sur AMD Ryzen 7 6800HS, `info` CLI d’un PDF de 492 pages prend 2,332/2,848/2,869 secondes avec le build dev optimisé,
contre 2,387/2,455/2,536 secondes avec la release publiée. Profils différents : observations de la commande entière,
pas mesure isolée de l’ouverture ni garantie de performance. La lecture initiale des libellés ajoute toujours du
travail à l’ouverture ou à la reconstruction de l’état du document.

## Version 0.10.0 : lot P1/P2 publié et vérifié

Les six domaines demandés sont implémentés : sauvegardes/corpus, compression réelle, lecture, formulaires,
recadrage/transformations groupées et lots compression/OCR. La CLI déclare 50 sous-commandes dans son code source.
L’anglais reste la langue par défaut et le français reste disponible.

Le workspace local complet consigne **362 tests, zéro échec, sept ignorés par défaut**. Formatage, Clippy sans
avertissement et 853 clés EN/FR identiques réussissent. Les deux corpus indépendants ont ensuite été exécutés
explicitement : **31 PDF synthétiques/générés** validés par qpdf et **quatre publications authentiques, 744 pages**,
validées par les régressions CLI bornées. CI GitHub de la source finale, compilations, paquets préparés et mises à jour
portables/MSI réelles réussissent comme détaillé ci-dessous. [SuniPDF 0.10.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0)
est publiée en **Latest** sous `lmdanavanem`, avec six fichiers (release `414522111`), sans brouillon ni préversion.

[CI finale main x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848137022) réussie pour le tag/source
immuable `f7510afd314846079bbdd838525cd28372fd3100`, avec workspace, Clippy, corpus indépendants et régressions
réelles de mise à jour. [Compilation ARM64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848138423/job/116803186215)
réussie depuis la même source ; exécution matérielle ARM64 non vérifiée. Le workflow x64 de release en double a été
annulé après ARM64 ; les fichiers x64 testés de la CI main et le job ARM64 réussi fournissent les paquets de publication.

Les **huit scénarios portables réels** réussissent : 0.8.2/0.8.3/0.9.0/0.9.1 vers 0.10.0, avec relance réelle,
empreintes des fichiers/sauvegardes et conservation documents/paramètres ; SHA erroné, mauvaise architecture,
rollback après fichier verrouillé et garde réelle 120 secondes/verrou concurrent réussissent aussi. La régression
MSI réelle par machine réussit depuis **0.8.0 (français) et 0.9.1 (anglais)** : installation, échec transactionnel
injecté/rollback, mise à niveau, refus de downgrade, empreintes, désinstallation et données conservées. Runners
Windows jetables avec Mesa 26.2.4 llvmpipe pour le démarrage GUI ; ses DLL de test ne sont jamais distribuées.

Les six fichiers préparés réussissent SHA256/fichiers d'empreintes ; architectures GUI/CLI/PDFium/Tesseract x64/ARM64,
métadonnées anglaises, sept images ICO incorporées, modèles FR/EN figés et notices complètes vérifiés. Les binaires
x64 empaquetés réussissent lecture/rendu, OCR français, gain de compression/source intacte, création radio/listes,
échanges XFDF et rendu des transformations groupées. Inspection MSI en lecture seule : langue 1033, transaction de
mise à niveau compatible et empreintes identiques au ZIP x64. L'installation réelle est prouvée par la CI ci-dessus ;
l'inspection des paquets n'installe rien.

Les six fichiers publics ont été téléchargés sans authentification et correspondent aux empreintes API GitHub,
fichiers SHA256 et fichiers préparés exacts, puis copiés dans `dist/`. Les contrôles renforcés sur ces octets publics
confirment architectures, métadonnées anglaises, sept images ICO, modèles/notices, gain de compression réel,
champs de choix/XFDF, groupes, OCR français et contenu/transaction MSI en lecture seule. Le test de l'updater de
production réussit sur le ZIP x64 public Latest : **19 651 167 octets**, SHA256
`add8ef3f1f385b47aa4e3fb9061f8735c7e31004d408abd853924a8bd9a35742`. HTTPS/redirections, taille et empreinte
réussissent sans installation locale ; les installations réelles ont réussi séparément en CI. Le corpus réel réussit
aussi en CI en 33,747 secondes, avec un pic enfant observé de 333 922 304 octets. Les mesures locales ci-dessous
décrivent leur propre exécution et ne garantissent pas des performances indépendantes du matériel.

| Domaine | Périmètre implémenté et contrôles attendus |
|---|---|
| Sauvegardes | NamedTempFile voisin, synchronisation, fermeture du handle puis renommage atomique ; erreurs injectées interruption/disque plein/écriture/permission et arrêt forcé d’une écriture partielle ; original lisible et intact |
| Corpus indépendant | Quatre publications NIST/NBS figées (66/25/161/492 pages), dont un vrai scan monochrome et des photographies en gris ; SHA256/taille source, texte/rendu, extraction, optimisation, compression, refus d'entrée corrompue et source intacte vérifiés |
| Compression | DeviceRGB/DeviceGray 8 bits, JPEG/brut/Flate, 32 millions de pixels/image maximum ; côté 128..8192 px, qualité JPEG 10..100 ; graphes des masques/profils/décodages particuliers conservés ; copie exacte sans gain ; PDF signés/restreints refusés |
| Lots | 1..128 PDF séquentiels, 256 Mio/entrée, compression ou OCR FR/EN local, progression, annulation et erreurs individuelles ; nouvelles copies atomiques sans écrasement |
| Lecture | Rotation de vue sans mutation PDF, plein écran et historique par onglet de 256 positions ; retour à la vue droite pour éditer |
| Formulaires | Création radio/liste déroulante/zone de liste, export XFDF, import JSON/XFDF/FDF borné et strict, Appliquer/Abandonner groupés, une annulation, Espace/Entrée et focus des listes |
| Édition | Recadrage souris dans l’aperçu et rotation/redimensionnement groupés ; une annulation, enregistrement/réouverture/rendu et géométrie normale/pivotée/recadrée/AES-256 vérifiés |

Le corpus réel s'exécute en 44,3 secondes ; le plus haut jeu de travail enfant observé atteint 343 621 632 octets,
avec une garde mémoire échantillonnée de 768 Mio. Chaque processus est limité à 180 secondes et 2 Mio de sortie
capturée. Quatre photographies du rapport de 161 pages sont recompressées : 9 791 921 vers 9 592 202 octets ;
les trois autres compressions conservent exactement la source. Trois originaux contiennent déjà des avertissements
Adobe de linéarisation/indices : leurs diagnostics qpdf exacts sont figés. Les sorties réécrites passent sans
avertissement ; les copies exactes sans gain conservent uniquement le diagnostic source figé. Ces avertissements
d'origine ne sont pas comptés comme des réécritures sans avertissement.

Les documents réels ont révélé des objets de stockage, bornes `/Size` et offsets de révision périmés dans les
réécritures lopdf. Le writer partagé retire désormais stockage/linéarisation obsolètes et `/Prev`/`XRefStm`, recalcule
l'identifiant d'objet maximal et conserve clé/permissions de chiffrement. qpdf et les régressions chiffrées réussissent.
Les injections d'erreur de sauvegarde, processus d'écriture partielle arrêté, destination verrouillée et contrôles
groupés édition/formulaires/lecture sans interface native réussissent aussi. Interaction GUI et résistance matérielle
à une coupure d'alimentation restent distinctes.

Restent indépendants : matériel ARM64, imprimantes physiques, Narrateur, dialogues natifs de mise à jour/enregistrement,
Acrobat/Foxit et corpus client/hostile/très volumineux. Identification PDF/A conservée ne signifie pas conformité
certifiée. Confiance de chaîne, révocation et horodatage fiable/LTV restent non implémentés.

## Version 0.9.1 : preuves de la publication précédente

[SuniPDF 0.9.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) a été publiée en Latest sous
`lmdanavanem`, avec six fichiers et le tag/source `c8473ecafe5f88b9dad846677fa610226955b061`.

Implémentés : icône agrandie (environ 91 % de largeur visible contre 50 %), conservation du XMP PDF/A/personnalisé,
refus du XMP malformé, longueur AES-256 explicite, résultat/erreurs de relance de l'updater et calcul SHA256 .NET,
corpus QPDF figé et régressions réelles de mise à jour portable/MSI dans la CI Windows.

Workspace local : 322 tests réussis, zéro échec, cinq ignorés par défaut ; Clippy, formatage et 804 clés bilingues réussis.
Le contrôle QPDF explicite valide 22 PDF synthétiques/générés et rejette trois entrées malformées. Conservation du XMP,
flux compressés/chiffrés et tours AES couverts. Windows PowerShell 5 réussit avec Get-FileHash indisponible.

[CI finale main x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38839336101) réussie pour la source
`c8473ecafe5f88b9dad846677fa610226955b061`, avec workspace, Clippy, qpdf et régressions réelles de mise à jour 0.9.1.
Sept scénarios portables réussis : 0.8.2/0.8.3/0.9.0 vers 0.9.1 avec relance réelle et contrôle des fichiers, SHA256 erroné,
mauvaise architecture, restauration après fichier verrouillé et garde de fermeture réelle 120 secondes/verrou de mise à jour concurrente.
La régression MSI réelle par machine réussit 0.8.0 vers 0.9.1 : installation, rollback transactionnel, mise à niveau,
refus de downgrade et désinstallation, avec conservation des données sur runner jetable.

Les tests GUI emploient Mesa 26.2.4 llvmpipe figé sur ces runners. Ses deux DLL restent dans les copies temporaires de test,
jamais dans les distributions ; le pilote graphique natif reste à valider. La
[régression Windows rapide](https://github.com/lmdanavanem/SuniPDF/actions/runs/38838856238) précédente a aussi réussi sur
la cible publique 0.9.0 figée sous Windows 2022 et récent, avec un scénario MSI réel.

[Compilation ARM64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38839338858/job/116775562315) réussie depuis la même source ;
exécution matérielle non vérifiée. Les six fichiers préparés réussissent les contrôles SHA256/empreintes, architectures GUI/CLI/PDFium/Tesseract,
version GUI anglaise, modèles eng/fra figés et notices. Les sept images ICO attendues sont incorporées dans les deux exécutables GUI ;
lecture/rendu, OCR français et création/remplissage/réouverture de champs du paquet x64 réussis. Contenu MSI identique au ZIP x64 ;
langue 1033 et séquence de rollback vérifiées.

Les six fichiers publics ont été téléchargés sans authentification et correspondent aux empreintes API GitHub,
aux fichiers SHA256 et aux fichiers préparés exacts, puis copiés dans `dist/`. Les binaires x64 publics réussissent
lecture/rendu PDF, OCR français et création/remplissage/réouverture de champs ; contenu MSI public, langue et séquence
de rollback vérifiés en lecture seule. Le test explicite de l'updater de production réussit sur le ZIP public x64 0.9.1
(19 270 287 octets, SHA256 `601306861be0c80bda62e568473172504b530d4344635d3bfa28cbf6adbfc082`) : HTTPS/redirections,
taille et SHA256 contrôlés sans installation. Les scénarios réels de mise à niveau/installation ci-dessus ont réussi séparément en CI.
Le second job x64 de release s'est arrêté avant ses tests de mise à jour : GitHub a limité ses requêtes de métadonnées
non authentifiées. Les paquets x64 vérifiés viennent de la CI main réussie ci-dessus ; le job ARM64 a réussi indépendamment.
Après publication, le script CI authentifie uniquement les requêtes de métadonnées vers l'API GitHub fixe, sans redirection,
et retire le jeton avant les processus enfants. Contrôles hors ligne anonyme/authentifié réussis ; le workflow complet
n'a pas été relancé pour ce changement de test seul. Les octets des paquets publiés restent identiques.

Compilation locale limitée à une commande Cargo avec `-j 2`. La reconstruction workspace a atteint l'erreur1455
du fichier d'échange Windows ; les compilations séquentielles par crate puis la suite workspace complète ont réussi.
Aucune installation ni profil utilisateur modifié localement.

Contrôles indépendants restants : corpus réel client/hostile, matériel ARM64, imprimantes physiques, Narrateur,
boîtes interactives de mise à jour/enregistrement et interopérabilité Acrobat/Foxit. Préserver l'identification PDF/A ne certifie pas la conformité.
Confiance du certificat/révocation/horodatage fiable/LTV restent non implémentés.

## Version 0.9.0 publiée et validée (version précédente)

Implémentés : récupération 60 s/suspension ; impression Windows ; sélection logique entre pages ;
double page/Livre avec couverture seule ; création texte/case ; OCR FR/EN local embarqué ; signature/vérification CMS/PFX.
Les 14 outils possèdent des commandes. La CLI compte 46 sous-commandes. Anglais par défaut ; français disponible.

| Contrôle | Résultat actuel |
|---|---|
| Nouveaux tests d'intégration moteur | 10 réussis : plages/rotations, création de champs, OCR et signatures par certificat |
| Nouveaux tests CLI de bout en bout | 3 réussis : champs, OCR réel, signatures des octets enregistrés et préservation source |
| `cargo test --workspace --locked -j 2` | 318 réussis, 0 échec, 4 ignorés (deux mesures, updater réseau et impression native), dernière réexécution réussie |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | Réussi sans avertissement |
| `cargo fmt --all -- --check` | Réussi |
| Catalogues bilingues | Réussi : 804 clés EN/FR identiques |
| Autotest du helper de mise à jour | Réussi, y compris remplacement et retour arrière du paquet OCR à plat |
| Microsoft Print to PDF natif | Test ignoré lancé explicitement réussi via le pilote Windows : sortie d'une page rouverte avec modifications non enregistrées ; objets/trailer source (hors ID généré) et octets d'origine inchangés |
| Publication 0.9.0 | [Publiée en Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.0) sous `lmdanavanem` ; tag/source `dbeafdf6b484c784a0dab26fe6698049ec2e49ca` |
| Provenance des compilations | [CI main x64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38830535213) réussie ; [build ARM64](https://github.com/lmdanavanem/SuniPDF/actions/runs/38830543193/job/116748068977) réussi. Seconde compilation x64 de release annulée après disponibilité des deux artefacts ; publisher automatique ignoré et six fichiers vérifiés publiés par le propriétaire |
| Fichiers publics | Six fichiers téléchargés et vérifiés par empreintes API et SHA256, puis copiés dans `dist/` : ZIP x64/ARM64, MSI x64 et trois empreintes. Architectures GUI/CLI/PDFium/Tesseract, version GUI anglaise, modèles eng/fra figés et notices OCR complètes contrôlés |
| Application/CLI x64 distribuées | Binaires publics vérifiés : lecture/rendu PDF, OCR français (`Bonjour`, `français`, `bientôt`) et création/remplissage/réouverture de champs |
| MSI x64 | Produit localement depuis le ZIP exact de la CI main ; empreintes de tous les fichiers MSI identiques au ZIP. Langue 1033, mise à niveau sans filtre de langue et séquence de rollback vérifiées. WiX smoke réussi avec l'avertissement ICE61 existant sur les mises à niveau de version identique |
| Updater réel de production | Test réseau ignoré lancé explicitement réussi sur le ZIP public x64 0.9.0 : HTTPS/redirections, taille et SHA256 de production ; aucune installation exécutée |

Non établis : impression physique, installation/mise à jour portable complète, matériel ARM64, nouvelles captures
de l'UI native ou boîtes interactives d'imprimante, Narrateur, compatibilité de signature Acrobat/Foxit et confiance de l'identité. Horodatage fiable,
révocation et LTV ne font pas partie de cette version.

## Historique d'implémentation jusqu'à la 0.8.3

Les phases et le tableau ci-dessous conservent leurs preuves propres aux anciennes versions. Anciens nombres
de commandes/clés, observations CI et captures sont historiques, pas des résultats courants de la release.

## Phase 0 — Audit et planification : terminée (historique)
- [x] `FEATURE_MATRIX.fr.md`, `ARCHITECTURE.fr.md`,
      `TECH_STACK.fr.md`, `ROADMAP.fr.md`, `I18N.fr.md`, `DECISIONS.fr.md`, `KNOWN_ISSUES.fr.md`, `THIRD_PARTY_NOTICES.fr.md`.

## Phase 1 — Fondations : terminée (historique ; validation visuelle finale par l'utilisateur recommandée)
- [x] Workspace Rust (`sunipdf-core`, `sunipdf-i18n`, `sunipdf-desktop`, `sunipdf-cli`), `cargo build` OK.
- [x] Moteur PDFium intégré (`pdfium-render` 0.9.4, DLL chargée dynamiquement, 4 emplacements de recherche).
- [x] Ouvrir / afficher / naviguer (clavier, barre d'outils, champ page) / zoom (ajuster largeur, page, paliers) / fermer.
- [x] Documents protégés : boîte de mot de passe, message « mot de passe incorrect ».
- [x] Glisser-déposer d'un `.pdf`, ouverture par argument de ligne de commande, titre de fenêtre.
- [x] États UI : vide, moteur en démarrage, ouverture, rendu en cours (spinner), erreur (bandeau), moteur absent.
- [x] i18n fr-FR / en-US : 76 clés, changement à chaud, préférence persistée (écriture atomique), thème clair/sombre.
- [x] Rendu hors thread UI (thread moteur unique, générations, un rendu en vol).
- [x] CLI : `info` (JSON), `render` (PNG), `commands` (registre), codes d'erreur stables, codes de sortie 3/4/5.
- [x] Tranche **annotations-advanced** (annotations avancées, SUNI-021b, D067–D076) : moteur `sunipdf-core::annot_write` (encre, zone de texte libre, rectangle, ellipse, ligne, flèche, lien, réponse, statut de relecture écrits par lopdf avec un flux `/AP` explicite ; `annotation_details` aligné sur `/Annots` avec les droits de chaque annotation ; `move` / `resize` / `set_annotation_style` / `update_free_text` / `set_annotation_contents` / `delete_annotation_thread` ; régénération d'apparence réservée aux annotations `/NM SuniPDF-…`), `appearance` (fonctions pures : largeurs Helvetica, renvoi à la ligne, WinAnsi, simplification et lissage des traits, flux de chaque forme), `links` (`page_links` par `annotations()`, `is_safe_external_uri`), `annot::annotations` aligné par indice ; coordonnées en espace d'affichage, apparences à `/Matrix` de rotation (texte droit sur page pivotée) ; documents chiffrés conservés chiffrés ; worker (12 `EditOp`, `Request::Annotations { detailed }` et `Links`) ; interface `annot_adv.rs` : 7 outils dans **Commenter** (Sélectionner une annotation A, Dessin D, Zone de texte F, Rectangle, Ellipse, Ligne ou flèche, Lien), barre flottante, boîtes zone de texte / lien / réponse / **confirmation d'adresse externe**, clic sur les liens (saut de page ; adresse après confirmation), sélection / déplacement / redimensionnement / recoloration / suppression, panneau Commentaires en fils (statut, réponses, auteur mémorisé dans `settings.json`) ; CLI `annotations`, `annotate`, `reply`, `review-state`, `annotation-delete` (41 commandes). 7 commandes (97 au registre), 64 clés i18n (676). Fixture `annotated.pdf` (12 annotations de plusieurs producteurs). Tests : 16 unitaires moteur nouveaux (`appearance` : largeurs, renvoi, WinAnsi, nombres, simplification, lissage, flux ; `annot_write` : dates, noms, droits, mise à l'échelle, translation, matrice, statut de fil ; `links` : 14 adresses hostiles refusées) + 12 d'intégration moteur (encre aller-retour au pixel, formes rendues, texte libre qui grandit et garde son texte, **pages pivotées et rognées au pixel** sur `boxes.pdf` avec flèche orientée et texte droit comparé à une page droite, liens sans doublon, lecture fidèle d'`annotated.pdf`, déplacement / style / contenu / suppression de fil sans toucher aux autres annotations, annotations de SuniPDF redimensionnées, réponses invisibles au pixel, **chiffrement AES-256 et RC4 conservé**, instantané / annulation, entrée illisible dans `/Annots`) + 7 unitaires application (prise, redimensionnement, boîte par défaut, dates, fils, barre, décision de lien) + 2 CLI unitaires + 3 CLI de bout en bout (`annotate` ×6 genres relus par `annotations`, réponse / statut / suppression de fil, erreurs typées dont adresses dangereuses, `already_exists`, document chiffré). Captures inspectées : barre Commenter à 14 boutons, encre tracée à la souris, boîte Zone de texte libre, boîte Lien, **confirmation d'adresse externe** (clic sur le lien de `annotated.pdf`), panneau Commentaires en fils (note + réponse + statut Accepté, thèmes clair et sombre), sélection d'une annotation d'Acrobat (cadre sans poignées, réglages masqués) et d'une annotation de SuniPDF (cadre, quatre poignées, barre), **déplacement et redimensionnement à la souris** avec resélection. Non livré : voir `NEXT_TASKS.fr.md` (SUNI-021b) et `KNOWN_ISSUES.fr.md` A39 (interopérabilité Acrobat / Foxit non vérifiée).
- [x] Tranche **forms** (formulaires AcroForm, moitié « remplir » de Remplir et signer, D061–D066) : moteur `sunipdf-core::forms` (modèle lu par lopdf : hiérarchie `/Kids` avec héritage, champs à plusieurs widgets, états `/AP /N`, pages par `/Annots`, listes avec export et libellé, rectangles en espace d'affichage par `PageGeometry` ; `form_info`, `validate`, `fill_form` strict/tolérant avec validation complète avant toute écriture, `clear_form`, `flatten` avec indicateur Print posé et `/AcroForm` retiré, `ensure_appearances` ; `DocumentInfo.form`) et `form_data` (JSON export/import/structure, coercition CLI) ; apparences régénérées par PDFium et **présentes dans les octets enregistrés** (`(Jean Dupont) Tj` relu par lopdf) ; document chiffré rempli et conservé chiffré ; XFA lecture seule ; sauts de ligne conservés malgré la table PDFDocEncoding de lopdf ; worker `Request::FormInfo`, `EditOp::{FillForm, ClearForm, FlattenForm}`, `BatchOp::ExportFormData` ; interface `forms.rs` : outil **Remplir le formulaire** (rectangles par page en vue continue, bleu / rouge obligatoire / gris lecture seule, clic sur case et radio, éditeur posé sur le champ texte avec Entrée / Échap / Tab / Maj+Tab et sélection du contenu, liste de choix flottante, popup à cases pour le choix multiple), panneau **Champs du formulaire** (Ctrl+Maj+F, un contrôle egui natif par champ, page courante ou document, bandeaux XFA et scripts, aller au champ), bandeau à l'ouverture, boîte **Aplatir** (toutes les pages / page courante), **Effacer les champs**, **Exporter / Importer (JSON)**, menu **Formulaires**, groupe « remplir » de l'outil Remplir et signer, état par onglet, widgets exclus du panneau Commentaires ; CLI `form-fields` (`--values-only`), `fill-form` (`--values`, `--set`, `--ignore-unknown`, `--flatten`), `flatten` (36 commandes). 6 commandes (90 au registre), 59 clés i18n (611). Tests : 3 unitaires moteur (oracle PDFium `lopdf_model_matches_pdfium_enumeration` sur les 16 widgets, règles de validation, JSON et coercition) + 10 d'intégration moteur (champs/types/valeurs/widgets, remplissage relu par instantané et par fichier avec pixels, erreurs typées et atomicité, apparences dans les octets, aplatissement avec surlignage `/F 0` et document de trois pages, XFA, chiffrement conservé, rotation 90/180/270, JSON aller-retour, effacement), 3 unitaires application (ordre de lecture, clés `forms.kind.*`, remise à zéro ; clés de `forms.rs` balayées par `ui_keys_used_by_app_exist`), 1 CLI unitaire (`--set`) + 1 CLI de bout en bout (`form-fields`, `fill-form --set` relu, `--values --flatten` puis rendu au pixel, refus strict sans fichier écrit, `--ignore-unknown`, `flatten`, `already_exists`, `form_xfa`). Captures inspectées : outil + panneau (16 rectangles, `ref` gris), case cochée + radio déplacé + liste ouverte, « Canada » choisi (flèche de liste dessinée par PDFium) puis Tab de `nom` à `ville` à `commentaire` (contenu remplacé, multiligne), boîte Aplatir, document aplati (valeurs dans la page, panneau vide, commandes grisées), Ctrl+Z (champs revenus), bandeau à l'ouverture sans outil, XFA (bandeau, champs gris), thème sombre + anglais avec liste ouverte. Non livré (D066) : création de champs, XFDF/FDF.
- [x] Tranche **sign-image** (signature visuelle, moitié « signer » de Remplir et signer, D054–D060) : moteur `Document::add_image_data` (image décodée, alpha conservé par le `/SMask` que PDFium 7881 écrit, vérifié au pixel : mélange à 50 %, bleu intact sous la zone transparente, persistance après rechargement et réouverture, masque qui suit déplacement et redimensionnement) et module `signature_image` (traits tramés anti-crénelés, fond clair retiré par seuil à rampe, rognage, lecture bornée à 16 384 px et 512 Mio décodés — refus avant décodage — avec orientation EXIF, PNG atomique) ; worker `EditOp::AddImageData`, `Request::LoadImage` / `Response::ImageLoaded` ; interface `sign.rs` : boîte « Signature manuscrite / Paraphe » (encadré permanent « sans valeur cryptographique », bibliothèque locale `%APPDATA%\SuniPDF\signatures` avec vignettes sur papier blanc et suppression en deux clics, pavé de dessin encre noire/bleue + épaisseur + annuler/effacer, import d'image décodé par le thread moteur avec aperçu et seuil, enregistrement facultatif), placement par fantôme (clic, Maj+clic, Échap, curseur en croix), objet image resélectionné (poignées, rotation, Ctrl+Z), date du jour par catalogue (`{d}/{m}/{y}`), outil **Remplir et signer** activé dans le hub (barre Signer / Parapher / Date), commandes au menu Édition, raccourci S ; CLI `add-image` (`--remove-background`, `"digital_signature": false`, 33 commandes). 4 commandes (84 au registre), 45 clés i18n (552). Tests : 6 unitaires moteur (`signature_image`, dont le refus d'un PNG annonçant 2 Gio sans rien allouer), 2 d'intégration moteur (`image_alpha_is_preserved_as_soft_mask`, `signature_strokes_to_page_pipeline` avec garde-fou de taille < 300 Ko), 5 unitaires application (placement borné, date maintenue dans la page, listage/tri de la bibliothèque, date par langue, état par défaut ; clés de `sign.rs` balayées par `ui_keys_used_by_app_exist`), 1 CLI unitaire + 1 CLI de bout en bout (`add-image` puis `render` relu au pixel, `already_exists`, image vide). Captures inspectées (`SUNIPDF_SIGNATURES_DIR`, `SUNIPDF_PICK_FILE`) : boîte vide (A), trois traits posés sur le rectangle bleu et sélectionnés (B), Ctrl+Z (C), bibliothèque avec vignettes depuis l'outil du hub (D1) et réutilisation d'une vignette posée sous le rectangle (D2), fantôme de la date avec message d'état (E1), date posée et sélectionnée (E2), Échap (F), import d'une photo avec fond retiré et seuil (G), Paraphe en sombre + anglais avec trois traits (H).
- [x] Tranche **review-fixes** (correctifs des 39 constats des revues adverses, D046–D053) : moteur (AES-128 refuse les mots de passe hors Latin-1 imprimable, aller-retour PDFium testé, bit d'accessibilité toujours accordé, XMP régénéré par `set_metadata`, `strip_metadata` sur pages/images/`PieceInfo`, `write_atomic` avec `sync_all`, orientation EXIF et JPEG incorporés tels quels, premier plan fiable après rechargement, tampons pivotés dans la page, redimensionnement uniforme des objets inclinés, `add_text` avec tabulations et liste des caractères absents, texte complet des pages pivotées, sous-types d'annotation stables, `Document::is_encrypted`), interface (tables de raccourcis `FIELD_SHORTCUTS` / `SHORTCUTS` avec Maj lu d'abord et invariant testé, dialogues et éditeurs dans la garde des raccourcis, Échap d'une liste ouverte consommé par `dialog::show`, corps des boîtes défilant à hauteur fixe et pied à hauteur fixe, zoom clavier d'egui désactivé, point d'enregistrement dans le thread moteur, couper atomique (copie puis suppression dans le thread moteur), intention du marquage mémorisée, sélection de la grille vidée après suppression, dépôt sans déplacement ignoré, Déplacer à gauche/droite grisés hors cas valide, Appliquer les métadonnées grisé pendant le chargement, cache des propriétés invalidé, Apposer grisé sans texte, unités et suffixes traduits, couleur d'erreur `Palette::error`, annonce du retrait de chiffrement à l'extraction/division), CLI (`extract` / `split` : `encryption_removed`). 23 clés i18n nouvelles, 1 retirée (507). Tests : 6 d'intégration moteur nouveaux (AES-128/accessibilité, XMP/PieceInfo, extraction d'un document protégé, EXIF/JPEG au pixel, premier plan après rechargement + tampons en coin, objet incliné + texte des pages pivotées) + 3 unitaires moteur (PDFDocEncoding/XMP, matrices d'orientation, sous-types), 8 unitaires application (ordre des raccourcis avec scénario egui, Échap et liste ouverte, contraste ≥ 4,5:1, point d'enregistrement ×2, tailles et séparateur décimal, messages de protection, libellés d'annotations). Captures inspectées : Propriétés en 720 × 480 (en-tête et boutons visibles, corps défilant), Propriétés anglais sombre (« 1 KB »), Tampon sans texte (Apposer grisé), Commentaires « Barré », Protéger sombre avec erreur lisible, Pivoter puis Ctrl+Z (titre sans « modifié », Enregistrer grisé).
- [x] SUNI-012b (tranche continuous-view) Vue continue, tuiles et zoom ancré : **défilement continu par défaut** (pages empilées, marge 16 px, intervalle 12 px, ombre douce), **page unique** en option (F5, bouton du rail, menu Affichage avec mode actif surligné, palette ; préférence `continuous_view`), **virtualisation** (seules les pages visibles créent widgets, demandes et textures ; `Layout` + recherche dichotomique), **page courante suivie** au défilement avec verrou après « aller à » (champ, miniature ramenée dans le panneau, signets, Origine/Fin, recherche amenée au tiers supérieur, y compris horizontalement à fort zoom), **rendu en tuiles** (`sunipdf-core::tiles`, `Document::render_region` : pages entières <= 4096 px / 8 Mpx, sinon tuiles de 1024 px, bornées par `max_texture_side`) avec **annulation par remplacement** de la liste voulue (`SetWanted` / `Tile` / `TileFailed`, époques), **pool de textures** borné (192 Mio, LRU) et textures d'une autre échelle ou périmées affichées pendant un zoom ou une retouche (`pages_unchanged`), **zoom ancré** (Ctrl + molette sous le pointeur, boutons au centre, jusqu'à 3200 %, plafonds `MAX_TILED_SIDE_PX` / hauteur totale), clavier (Espace, Page suiv./préc. d'un écran, flèches d'une ligne), miniatures bornées (256) et synchronisées, onglets qui gardent leur défilement ; Panneau Objets F5 → F3 ; rail droit : bascules actives surlignées, groupe du haut défilant en fenêtre basse ; CLI `render --region` ; fixture `many-pages.pdf` ; 2 commandes (80 au registre), 4 clés i18n nouvelles + 4 modifiées (485) ; D038–D045. Tests : 7 d'intégration moteur (régions = découpe exacte du rendu complet, page pivotée, zoom ×40 au-delà de 16 384 px, page image à ×50, erreurs, fixture 120 pages, recollage des tuiles à 99,9 %) + 1 mesure ignorée, 7 unitaires moteur (tuiles, cache), 13 application (mise en page, 600 pages virtualisées, ancre de zoom, plafonds, pool borné, tuiles voulues, envoi, file du moteur, `keeps_pages`, préférences) + 1 mesure ignorée, 1 CLI. Captures inspectées : ouverture continue (S1), dernière page (S2), page unique identique à l'ancienne version (S3), zoom 1600 % sans panique (S4) et texte net à 800 % sur un résultat de recherche, recherche « Page 7 » (S5), sélection + Ctrl+C sur la page 2 (S6), surlignage page 2 + panneau commentaires (S7), rectangle et sélection d'objet avec poignées page 2 (S8), Page suiv. après un clic (S9), ajuster à la page (S10), Origine (S11), texte sur la page 2, rotation d'une page en continu, onglets (défilement restauré), Ctrl + molette (`-Wheel`), menu Affichage, thème sombre, fenêtre basse.
- [x] SUNI-031c (tranche edit-advanced) Édition avancée du contenu et pages pivotées : moteur en **espace d'affichage** (`geometry.rs` : objets, texte, recherche, surlignage, notes justes sur les pages `/Rotate` 90/180/270 et rognées, hypothèses H1–H7 du plan vérifiées au pixel), **opacité** (textes, formes, images ; alpha conservé au changement de couleur), **rotation libre** (poignée, Maj 15°, champ Angle, cadre orienté), **sélection multiple** (rectangle, Alt, Maj/Ctrl+clic, Ctrl+A, prise directe au glisser) et déplacement/suppression/duplication/ordre/style groupés, **presse-papiers d'objets** (Ctrl+C/X/V réels via les événements egui Copy/Cut/Paste, collage entre pages et onglets en groupe XObject, duplication sans remonter l'original), **panneau Objets** (F5, F3 depuis SUNI-012b), **texte sur la page** (mini-barre, plusieurs lignes, texte existant droit éditable sur place), **polices TrueType du système** incorporées (liste par registre, licence `fsType`, refus typé des caractères absents, y compris en Helvetica), **remplacer l'image**, **page vierge**, **recadrer** (aperçu), **redimensionner** (conserver / adapter) ; barre d'objet contextuelle sous la sélection ; corrigés au passage : Ctrl+C mort, duplication qui remontait l'original, alpha perdu au changement de couleur, taille des textes pivotés (0 à 90°, multipliée à la modification), tampons sur pages pivotées, « ÿ » des textes non latins en police standard ; 9 commandes (78 au registre), 83 clés i18n nouvelles (481) ; CLI `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts` exercées sur les fixtures ; fixture `boxes.pdf` ; 11 tests d'intégration + 8 unitaires moteur, 5 unitaires application, 1 CLI. Captures inspectées (FR clair, EN sombre) : panneau Objets, sélection et poignées, rotation libre, rectangle élastique et déplacement groupé, Ctrl+C/Ctrl+V, collage d'un onglet à l'autre, texte sur page pivotée, texte sur la page + mini-barre, sélecteur de polices filtré, édition sur place d'un texte existant (F2), dialogues Recadrer (aperçu) / Redimensionner / Page vierge, recadrage appliqué.
- [x] SUNI-040 (tranche tool-hub) Hub « Tous les outils » : `workspace.rs` (table des 14 outils par tâche, 9 visibles car réellement implémentés : Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger, Compresser, Ajouter un tampon), panneau gauche à pastilles colorées (F9), barre contextuelle par outil (boutons groupés, libellés selon la largeur puis mode compact et défilement, actions grisées sans sélection, Quitter l'outil / Échap / retour), menu Outils, 6 commandes `hub.*` (69 au registre, `sunipdf-cli commands` vérifié), raccourcis à une lettre qui ouvrent l'outil, outil actif par onglet, accueil à cartes entièrement cliquables + carte « Ouvrir un fichier », accès rapide Ouvrir/Enregistrer/Annuler/Rétablir ; palette flottante supprimée, barre d'objet et barre de grille réduites ; finition du chrome (bouton plein lisible en sombre, glyphes contrastés, fondus, anneau de focus, ombres, onglet actif souligné, icônes) ; 12 tests (table, couverture du registre, contraste ≥ 3,5:1, bascule et suivi d'outil, passe egui sans fenêtre : géométrie, arbre AccessKit, focus clavier). Captures inspectées : accueil clair/sombre/anglais, Commenter, Modifier (+ rectangle), Organiser, Protéger, Échap, recherche + Échap, Surligner par raccourci, Tampon + dialogue sombre, onglets, palette, fenêtre étroite (740 px).
- [x] SUNI-033 Tampons, filigranes et numéros de page : dialogue « Tampon ou filigrane » (texte, taille, couleur, gras, angle, opacité, 7 positions, toutes les pages ou page courante) et « Numéroter les pages » (modèles `{n}`, `{n} / {total}`, `Page {n}`), objets texte ajoutés au contenu donc modifiables et annulables ; moteur `sunipdf-core::stamp` (`stamp_pages`, opacité du texte dans `TextStyle`) ; CLI `stamp` et `page-numbers` exercées sur la fixture (texte relu : « BROUILLON 1/2 », « Page 1 ») ; 1 test moteur (centre de page, rotation 45°, pixels atténués, numéros en bas à droite, persistance).
- [x] SUNI-032 Finition visuelle et édition : cadre commun des dialogues (`dialog.rs`, en-tête icône/titre/sous-titre, pied à boutons principal/secondaire/danger), À propos avec logo et crédit Navanem, panneau d'outils et accueil à icônes colorées par catégorie, outils utilisables sans document (ouverture puis exécution différée), édition : rotation ±90°, duplication, premier plan / arrière-plan (fusion des flux de contenu lopdf + réinsertion), flèches pour décaler, Échap, proportions (Maj), boutons d'action dans la barre d'objet ; 5 commandes ; 1 test moteur (rotation, duplication, ordre Z vérifié au rendu) ; tests moteur sérialisés sur un moteur unique.
- [x] SUNI-031b Édition avancée : redimensionnement par poignées d'angle (aperçu en direct, curseurs dédiés), barre des propriétés d'objet (couleur/épaisseur du trait, remplissage ; « Appliquer à l'objet »), polices Helvetica / Times / Courier + gras / italique, taille et couleur du texte existant (F2), aperçu des formes pendant le tracé, image insérée avec ses proportions (coin haut-gauche sous le clic) ; correction de l'origine des glissements (position de pression) ; moteur `resize_object`, `set_object_style`, `set_text_size`, `FontFamily`, infos couleur/police dans `ObjectInfo` ; 1 test moteur supplémentaire (redimensionnement, recoloration vérifiée au rendu, Times italique, taille ×2). Captures vérifiées : déplacement, redimensionnement, ellipse avec barre, dialogue F2.
- [x] SUNI-031 Édition du contenu des pages : catégorie « Éditer » et outils dans la palette (sélectionner un objet E, ajouter du texte T, insérer une image I, rectangle R, ellipse O, ligne L), sélection d'un objet existant au clic (cadre + poignées), déplacement par glissement, suppression (Suppr), modification du texte d'un objet existant (F2 ou double-clic, police d'origine conservée), ajout de texte Helvetica / Helvetica-Bold avec taille, insertion d'image (PNG, JPEG, BMP, GIF, TIFF, WebP ; largeur 200 pt), formes vectorielles ; tout est annulable et enregistré dans le flux de contenu (pas des annotations) ; moteur `sunipdf-core::edit` (PDFium `FPDFPageObj_*`, régénération du contenu) ; 1 test moteur (ajout, hit-test, modification, déplacement, suppression, image et formes vérifiées au rendu, persistance après enregistrement). Limites restantes après SUNI-031b : pas de rotation d'objet, polices limitées aux 14 standard, texte existant limité aux caractères de sa police.
- [x] SUNI-012a Onglets multi-documents : plusieurs PDF ouverts (barre d'onglets, Ctrl+Tab / Ctrl+Maj+Tab, fermeture par onglet, accueil sans fermer), documents inactifs suspendus dans le moteur (octets + historique d'annulation conservés) et état d'interface conservé par onglet ; confirmation d'enregistrement onglet par onglet à la fermeture ; plusieurs fichiers acceptés en arguments de ligne de commande.
- [x] SUNI-030 Livraison Windows : icône originale générée par `scripts/make_icon.py` (Pillow, aucune ressource tierce), icône + informations de version + manifeste DPI PerMonitorV2 intégrés à `sunipdf.exe` (`build.rs`, `winresource`), icône de fenêtre, `scripts/package-portable.ps1` (ZIP portable + SHA256, x64/arm64), `packaging/wix/SuniPDF.wxs` + `scripts/package-msi.ps1` (WiX v3), `.github/workflows/release.yml` (portable x64 + ARM64, MSI x64, signature si certificat), guides utilisateur FR/EN.
- [x] SUNI-024 / 014d Outils de fichiers dans l'interface (catégorie « Convertir », menus, palette) : combiner des PDF, diviser (dialogue « pages par fichier »), extraire la sélection, créer un PDF à partir d'images (PNG, JPEG, BMP, GIF, TIFF, WebP), exporter les pages en PNG 150 ppp, optimiser sans perte (recompression lopdf, objets inutilisés) ; tâches sur le thread moteur avec état « en cours » ; CLI `images2pdf`, `export-images`, `optimize` ; 1 test moteur (images → PDF rendu vérifié, export PNG, optimisation avec signets conservés).
- [x] SUNI-022/023 Signets (panneau F6, navigation), propriétés du document (Ctrl+D : métadonnées éditables, version, taille, chiffrement, permissions), protection par mot de passe AES-128/AES-256 avec permissions (dialogue + CLI `protect`), retrait de la protection (`unprotect`), suppression des métadonnées (`strip-metadata`), `set-metadata`, `properties`, `bookmarks` en CLI ; moteur `sunipdf-core::security` (lopdf pour l'écriture, PDFium pour la relecture) ; 2 tests moteur dont un aller-retour chiffrement/déchiffrement vérifié par PDFium.
- [x] SUNI-021 Annotations : sélection de texte par glissement et copie (Ctrl+C), outils surligner / souligner / barrer / note (palette flottante, raccourcis V H U X N), panneau Commentaires (F7, liste par page, suppression), catégorie « Annoter », toutes annulables ; moteur `sunipdf-core::annot` (PDFium génère les apparences) ; 1 test moteur avec vérification du rendu.
- [x] SUNI-014c Annuler/Rétablir par instantanés (Ctrl+Z / Ctrl+Y, 50 niveaux, 512 Mo max), insertion de pages depuis un autre PDF, déplacement de pages (glisser-déposer dans les miniatures et dans la vue Organiser, boutons avant/arrière), vue **Organiser les pages** (F8 : grille, sélection multiple clic/Ctrl/Maj, actions groupées), variable `SUNIPDF_STARTUP_COMMANDS` pour l'automatisation des captures ; CLI `insert`, `move` ; 1 test moteur (insertion, déplacement, instantané, chiffré).
- [x] SUNI-020 (UI) Coquille : barre d'onglets (accueil, document, + Ouvrir, langue, aide, thème), barre d'outils (Menu, catégories Tous les outils / Lire / Pages, recherche de commandes), panneau « Tous les outils », page d'accueil (cartes, fichiers récents persistés, confidentialité), rail droit (miniatures, recherche, page, zoom, ajustements, rotation), palette de commandes Ctrl+K, polices système Segoe UI + Segoe Fluent Icons chargées à l'exécution, thèmes clair/sombre retravaillés. Captures vérifiées par `scripts/dev-screenshot.ps1`.
- [x] SUNI-014b (UI) Menu Pages (pivoter à droite/gauche, supprimer la page), Enregistrer / Enregistrer sous (Ctrl+S, Ctrl+Maj+S), état « modifié » dans le titre, invite Enregistrer / Ne pas enregistrer / Annuler avant fermeture, ouverture ou quitter (y compris la croix de la fenêtre). 20 commandes au registre.
- [x] SUNI-014 (moteur + CLI) Fusion, extraction, division, rotation, suppression de page, enregistrement atomique (`sunipdf-core::pages`, `write_atomic` partagé avec les préférences) ; CLI `merge`, `extract`, `split`, `rotate` ; 4 tests.
- [x] SUNI-013 Recherche plein texte (Ctrl+F, barre de recherche, casse, occurrence suivante/précédente, surbrillance sur la page) et extraction de texte ; CLI `text` et `search` ; 1 test moteur.
- [x] SUNI-011 Cache LRU de rendu plafonné (256 Mo, `sunipdf-core::cache`), pré-rendu des pages voisines, panneau de miniatures virtualisé (F4), 2 tests cache.
- [x] SUNI-010 Registre de commandes (`sunipdf-core::command`) : 57 commandes, menus et raccourcis routés par `SuniApp::execute`, 3 tests.
- [x] Tests exécutés : `cargo test --workspace` → voir « Dernière exécution ».
- [x] Test de fumée : `sunipdf.exe two-pages.pdf` démarre et reste actif (fenêtre non inspectée visuellement par l'agent).
- [x] Captures d'écran réelles (accueil, document, palette) vérifiées par l'agent via `scripts/dev-screenshot.ps1` ; validation finale par l'utilisateur à faire.
- [x] Compilation MSVC (cible de référence) : Build Tools installés le 13/10 au soir, `cargo test --workspace` et `cargo clippy --workspace --all-targets -D warnings` verts.
- [x] Icône / ressources de version Windows (vérifié : `(Get-Item sunipdf.exe).VersionInfo` → SuniPDF, SuniWorks, 0.1.0).
- [ ] Observation CI historique (2026-10-14, v0.1.0/v0.2.0) : workflows déclenchés, résultat non consigné dans cette première liste. Les preuves de publication 0.8.x figurent dans le tableau historique ci-dessous.
- [x] MSI x64 construit localement avec les binaires WiX 3.14 autonomes : `dist\SuniPDF-0.2.0-win-x64.msi` (11,0 Mo) + SHA256 (0.1.0 validé par `smoke.exe` et une extraction administrative ; 0.2.0 reconstruit le 14/10, même procédé).
- [ ] Tentative ARM64 locale historique (2026-10-14) : outils Visual Studio ARM64 manquants, liaison impossible. Les paquets CI ultérieurs ne prouvent pas l'exécution sur matériel ARM64 (voir `KNOWN_ISSUES.fr.md` n° 12).

## Validation historique 0.8.3 (2026-10-16, cible **MSVC**, `-j 2`)
| Commande | Résultat |
|---|---|
| `cargo test --workspace --locked -j 2` | 276/276 OK, 3 ignorés (2 mesures et 1 contrôle réseau réel) : core 57 unitaires + 84 intégration + 7 protection, application 102, i18n 6, CLI 10 unitaires + 1 `add_image` + 6 `annotations_cli` + 2 `forms_cli` + 1 `redact_cli` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 avertissement |
| `cargo fmt --all -- --check` | OK |
| `scripts\check-i18n.ps1` | OK : 726 clés bilingues (fr-FR / en-US identiques) |
| Téléchargement updater réel | Contrôle réseau ignoré lancé explicitement après publication : ZIP public x64 de la 0.8.3 téléchargé avec les contrôles HTTPS/redirections de production, taille et SHA256 vérifiés ; aucune installation. |
| Helper `-SelfTest` | Réussi : chemins, SHA incorrect, ZIP dangereux/limites, architecture, copie, retour arrière et préservation des fichiers tiers. Lancement complet de l'installateur et installation réelle non exécutés. |
| Distribution 0.8.1 | CI et release réussies ; publiée sous lmdanavanem, six fichiers publics téléchargés et SHA256 vérifiés. Architectures x64/ARM64 et version bureau contrôlées ; version, lecture PDF et rendu CLI x64 réussis. |
| Distribution 0.8.2 | Publiée en Latest sous lmdanavanem. CI x64 et build ARM64 réussis. MSI produit depuis les binaires x64 vérifiés et validé statiquement ; seconde compilation x64 de release annulée. Six fichiers publics téléchargés et SHA256 vérifiés ; ressources anglaises, architectures, licences, lecture/rendu CLI et séquence de retour arrière MSI contrôlés. |
| Distribution 0.8.3 | [Publiée en Latest sous lmdanavanem](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.8.3), depuis `cf79e74`. CI x64 et build ARM64 réussis ; seconde compilation x64 de release annulée. Six fichiers publics téléchargés, SHA256 vérifiés, puis copiés dans `dist/`. Versions bureau/CLI, architectures x64/ARM64, contenu et licences contrôlés. Lecture/rendu et refus d'opacité CLI x64 réussis. Le MSI contient les binaires du ZIP CI et retire l'ancienne version dans la transaction de retour arrière. Validation statique WiX réussie avec l'avertissement ICE61 existant sur les mises à niveau de version identique. |
| Mesures vue continue (AMD Ryzen 7 6800HS, 13,7 Go, NVIDIA RTX 3050 Laptop, Windows à 150 %) | `SUNIPDF_PDFIUM_DIR=third_party\pdfium\win-x64\bin cargo test -p sunipdf-core --release -j 2 --test engine -- --ignored --nocapture render_scroll_100_pages_timing` : 100 pages de `many-pages.pdf` à 1,5 px/pt en pages entières, 150 à 167 ms au total (1,5 à 1,7 ms par page, première 4 à 9 ms) ; 10 pages à 6 px/pt en 200 tuiles, 230 à 239 ms (1,2 ms par tuile 1024², première page 24 ms) — deux exécutions. `cargo test -p sunipdf-desktop -j 2 -- --ignored --nocapture layout_cost` (débogage) : `Layout::new` + `visible` 14 µs pour 500 pages, 144 µs pour 5000 pages par image. Mémoire (`target\debug\sunipdf.exe`, `many-500.pdf`, 300 Page suiv. / Fin / 300 Page préc. au clavier) : privé 176 Mo à l'ouverture, plateau 810–827 Mo, WorkingSet 438 Mo (A28) |
| Distribution 0.8.0 | CI et release GitHub réussies ; six fichiers publics téléchargés et SHA256 vérifiés (ZIP x64/ARM64, MSI x64 et empreintes). Versions et architectures des binaires vérifiées ; contenu du MSI identique au ZIP x64. Validation WiX réussie avec avertissement ICE61 lié à `AllowSameVersionUpgrades`. CLI x64 : lecture et rendu vérifiés ; ARM64 non exécuté. |
| Captures `scripts\dev-screenshot.ps1` | annotations-advanced (`two-pages.pdf`, `annotated.pdf`, `-Commands hub.comment,tool.ink` etc.) : encre tracée, boîtes Zone de texte libre / Lien / confirmation d'adresse, panneau Commentaires en fils clair et sombre, sélection, déplacement et redimensionnement d'une annotation de SuniPDF (`ann2.pdf`) ; forms (`acroform.pdf`, `-Commands forms.fill`) : 16 rectangles surlignés et panneau à 13 champs, clic sur la case « Accepte » (coche dessinée par PDFium) puis sur le radio « carte » (déplacé) puis sur « Pays » (liste flottante France / Belgique / Canada avec « (aucun) »), « Canada » choisi et relu avec la flèche de liste régénérée, clic sur « Nom » puis `Jean{TAB}Paris{TAB}Deux lignes{ENTER}` (Tab sélectionne le contenu du champ suivant, Entrée fait un saut de ligne dans le champ multiligne), boîte Aplatir, aplati par Entrée (valeurs dans la page, panneau « aucun champ », outil grisé, message d'état), Ctrl+Z après aplatissement (champs et panneau revenus, titre intact), bandeau « Ce document contient un formulaire à remplir » sans outil, `acroform-xfa.pdf` (bandeau XFA, champs gris, commandes grisées), thème sombre + anglais avec la liste ouverte (préférences restaurées) ; le script vise `GetWindowRect` : un clic à (x, y) arrive en (x − 8, y − 31) côté egui (tracé `RUST_LOG=trace`) ; sign-image : boîte Signer vide (encadré d'avertissement, pavé blanc, bouton Placer grisé), trois traits → signature posée sur le rectangle bleu et sélectionnée (poignées, barre d'objet, onglet modifié, PNG écrit dans `SUNIPDF_SIGNATURES_DIR`), Ctrl+Z (signature disparue, titre intact), outil Remplir et signer du hub (barre Signer / Parapher / Date, ligne active) + bibliothèque à deux vignettes, vignette cliquée puis posée sous le rectangle, fantôme de la date sous le pointeur avec message d'état, date `15/10/2026` posée et sélectionnée, Échap (rien d'ajouté), import d'une photo (`SUNIPDF_PICK_FILE`) avec fond et ombre retirés au seuil 200, Paraphe en thème sombre + anglais ; review-fixes (2e passe) : Protéger avec « abc » tapé à l'ouverture (arrive dans le champ d'ouverture), liste Position du tampon ouverte (sept entrées visibles) ; review-fixes : Propriétés en fenêtre 720 × 480 (en-tête, corps défilant et boutons visibles), Propriétés anglais + sombre (« File size 1 KB »), Tampon sans texte (Apposer grisé), outil Barrer + panneau Commentaires (« Barré »), Protéger sombre + anglais avec erreur lisible (« Enter at least one password. »), Pivoter puis Ctrl+Z (titre sans « modifié », Enregistrer grisé, Rétablir actif) ; tranches précédentes (accueil, document, palette, Organiser, annotations, signets, propriétés, protection, onglets, hub d'outils clair/sombre/anglais…) ; edit-advanced : panneau Objets, sélection + poignée de rotation, rotation libre à 77°, rectangle élastique + déplacement groupé, Ctrl+C/Ctrl+V au clavier (deux collages décalés), Ctrl+A/Ctrl+C/Ctrl+Tab/Ctrl+V d'un onglet à l'autre, texte ajouté droit sur la page A (`/Rotate 90`) de `boxes.pdf`, éditeur sur la page avec Maj+Entrée, sélecteur de polices filtré (« ari »), F2 sur un texte existant, dialogues Recadrer (aperçu) / Redimensionner / Page vierge, recadrage appliqué, thème sombre + anglais : inspectées par l'agent |
| `sunipdf-cli` | annotations (test `annotations_cli`) : `annotate` ×6 genres sur `two-pages.pdf` relus par `annotations` (indices, sous-types, rectangles, auteur, couleurs), réponse + statut + `annotation-delete --thread` (3 retirés), 10 erreurs typées (adresses `file:` et `javascript:` refusées), `already_exists` (code 6), document RC4 annoté puis `password_required` (code 3) ; forms (test `forms_cli` sur les fixtures) : `form-fields acroform.pdf` → 13 champs, `kind acroform`, `ville` « Élodie Müller », `pays` combo avec options et widget page 1 rect [150, 436, 350, 456] ; `--values-only` → `{ nom: valeur }` sans `signature` ; `fill-form --set nom=Jean --set accepte=true --set "langues=Français;Allemand"` → `applied 3` relus par `form-fields` ; `fill-form --values values.json --flatten` → `kind none`, `count 0`, rendu au pixel (> 300 pixels sombres dans « nom ») ; `--set inconnu=1` → `form_field_not_found` (code 1) sans fichier, `--ignore-unknown` → `skipped.unknown ["inconnu"]`, `skipped.invalid[0].field "ref"` ; `--set ref=x` → `form_value_invalid` ; `flatten` → `annotations ≥ 16`, puis sortie existante → `already_exists` (6) ; `acroform-xfa.pdf` → `form_xfa` ; `two-pages.pdf` → `kind none`. sign-image : `add-image two-pages.pdf --image signature-transparent.png --x 100 --y 520 --width 150` → `height_pt 50`, `digital_signature false`, rendu relu au pixel (621 pixels d'encre, 9 879 pixels bleus visibles entre les traits dans la zone, aucun autre) ; `--image signature-photo.jpg --remove-background 200` → `background_removed true`, rendu propre (fond et ombre retirés, 613 pixels d'encre) ; `--remove-background 1` → `other` « image is empty after background removal » (code 1) ; image absente → `io` (5) ; sortie existante sans `--overwrite` → `already_exists` (6) ; `--page 7` → `page_out_of_range` (1). review-fixes : `protect --aes128` avec « пароль » ou « œuvre » → `password_not_latin1` (code 1), avec « Été » → fichier rouvert par `properties --password` ; `extract` et `split` d'un fichier AES-256 → `"encryption_removed": true` (false sur un fichier clair) ; `rotate 90` puis `text --page 1` → « SuniPDF page 1 » (texte en y = 700 > 612, perdu avant) ; `add-text` « Привет » en Helvetica → `missing_glyphs` avec la liste des caractères. continuous-view : `render --page 1 --scale 40 --region 4000,7000,256,256` → PNG 256 x 256 uni bleu, `page_pixels` [24480, 31680] ; région hors page → `other` (code 1) ; page entière à ×40 → `render_too_large`. edit-advanced : `objects`, `add-text --font-name Arial` (cyrillique relu par `text`), `set-opacity`, `rotate-object`, `copy-objects --into`, `crop --unit mm`, `resize-pages --mode scale`, `insert-blank --like`, `info` (rotation), `fonts` (323 polices), refus `missing_glyphs` (code 1) et `already_exists` (code 6) exercés sur les fixtures ; autres commandes exercées par les tranches précédentes |

## Non réalisé (ne pas confondre avec « fait »)
Rotation de vue, ordre complexe de colonnes non balisées, sous-ensembles de polices/CFF/TTC (TrueType entières),
collage d'objets éditable, transformation de groupes, autres types de champs et FDF/XFDF, XFA modifiable,
conversion Office, confiance/révocation/horodatage fiable/LTV, matériel ARM64 et QA corpus/performances élargie.

## Journal
- 2026-10-16 : la 0.9.0 implémente les sept fonctions demandées. Workspace : 318 réussis/4 ignorés, Clippy/formatage
  réussis, 804 clés bilingues, autotest updater et sortie réelle Microsoft Print to PDF réussis. Publiée en Latest
  sous `lmdanavanem` ; six fichiers publics/SHA256, contenu des paquets, OCR français, champs et téléchargement updater vérifiés.
- 2026-10-16 : version 0.8.3, quatre corrections P1/P2 terminées : coordonnées de texte finies, textures et
  demandes de miniatures bornées dans Organiser/le panneau avec gestion des résultats périmés et des échecs,
  tampons et modèles de numérotation sur une ligne, refus explicite de l'opacité non prise en charge du texte libre CLI.
- 2026-10-16 : version 0.8.2, notifications au démarrage, téléchargement vérifié et installation explicite
  après confirmation de fermeture. Anglais par défaut et documentation principale anglaise ; français et
  préférences existantes conservés. Retour arrière des mises à niveau MSI.
- 2026-10-15 : version 0.8.1, croix des onglets corrigée ; quatre tests de clic couvrent fermeture active,
  fermeture en arrière-plan avec/sans modifications, navigation par le titre et refus pendant un enregistrement.
- 2026-10-15 : version 0.8.0, six corrections prioritaires : mots de passe effectifs, brouillons du panneau
  Formulaire, sélections pendant les changements d'onglet, fermeture avec modification en attente, déplacement
  préservant champs/signets/liens et masquage des champs mot de passe dans le listing JSON complet.
- 2026-10-15 : annotations-advanced (annotations avancées, D067–D076) : voir la case cochée en tête. Écarts au plan : documents
  chiffrés conservés chiffrés au lieu d'un refus (D067) ; apparences en coordonnées d'affichage avec matrice de rotation (D068) ;
  les sept outils rejoignent l'outil Commenter du hub au lieu d'une palette flottante (D074). Captures : le glisser à la souris du
  harnais est fiable après calibrage (coordonnées de fenêtre = pixels de capture / 1,5 à 150 %, premier appui à la position
  périmée du centre de la fenêtre : précéder d'un glissement d'échauffement ; un clic = glissement de longueur nulle `x,y,x,y`) ;
  un panneau qui s'élargit de lui-même à chaque image venait d'un cadre imbriqué dans une rangée `horizontal` (corrigé par
  `outer_margin`). 207 tests, clippy, fmt et i18n (676 clés) verts.
- 2026-10-15 : sign-image revérifiée sur HEAD (tranche déjà livrée par e4d3016 → 786e83f) : captures A (boîte
  vide), B (trois traits posés sur le rectangle bleu, sélectionnés, bleu visible entre les traits, PNG écrit dans
  `SUNIPDF_SIGNATURES_DIR`), D (bibliothèque à une vignette) rejouées ; gouttière de 10 px pour la barre de
  défilement des boîtes (A37 réduite). Au passage, reprise du correctif de revue forms trouvé non commité
  (liens conservés à l'aplatissement : copie profonde, D062 corrigé ; `eprintln!` de débogage retirés) ;
  167 tests, clippy, fmt et i18n (612 clés) verts.
- 2026-10-15 : forms (formulaires AcroForm, D061–D066) : fixtures `acroform*.pdf`, moteur `forms` / `form_data`
  (lecture et écriture lopdf, apparences PDFium écrites dans le fichier, aplatissement, JSON), erreurs `form_*`,
  CLI `form-fields` / `fill-form` / `flatten`, outil Remplir le formulaire (surcouche par page, éditeur posé,
  listes), panneau Champs du formulaire, bandeau, boîte Aplatir, export/import JSON ; 165 tests, 611 clés ;
  création de champs et XFDF/FDF en feuille de route.
- 2026-10-15 : review-fixes, 2e passe (revue indépendante de la tranche) : AES-128 limité au Latin-1 imprimable
  que PDFium rouvre (€, œ, tirets et guillemets typographiques refusés ; test d'aller-retour moteur, A30 et D047
  corrigés) ; couper atomique dans le thread moteur ; liste des caractères absents dédoublonnée ; focus dans
  Protéger ; sept positions de tampon visibles ; indicateur « modifié » des onglets aligné sur le moteur ;
  A34 (XMP régénéré sans `pdfaid`). 130 tests, clippy, fmt et i18n verts, deux captures inspectées.
- 2026-10-14 : tranche review-fixes : 39 constats des revues adverses rejoués sur HEAD puis corrigés à la racine (moteur, thread moteur, interface, CLI), décisions D046–D053 ; 129 tests, clippy et fmt verts, 507 clés i18n, captures inspectées (Propriétés 720 × 480 et anglais sombre, Tampon, Commentaires, Protéger sombre, annulation jusqu'à l'état enregistré).
- 2026-10-13 : étude des lecteurs existants, choix PDFium (D005), squelette → tranche verticale 1, environnement GNU faute de MSVC (D008),
  wgpu désactivé (D010), fixtures générées (`scripts/make_fixtures.py`), dépôt git initialisé.
- 2026-10-14 : SUNI-031 édition du contenu (moteur `edit.rs` + catégorie « Éditer », D019) ; 36 tests, clippy vert, captures vérifiées.
- 2026-10-14 : SUNI-031b édition avancée (poignées, couleurs, polices), version 0.2.0, `CHANGELOG.fr.md`, tag `v0.2.0` poussé (Release GitHub produite par `release.yml`, à vérifier dans l'onglet Actions/Releases).
- 2026-10-14 : tranche tool-hub (SUNI-040) : hub « Tous les outils », barre contextuelle, finition du chrome (D021–D026) ; 55 tests, clippy et fmt verts, captures inspectées.
- 2026-10-14 : correctifs de la revue du hub : Échap avec un menu ouvert ferme le menu (point unique `workspace::take_escape`, ordre testé), icônes de rotation et de déplacement distinctes, barre d'objet bornée à la zone document, palette, focus de l'accès rapide, chemin absolu des fichiers récents ; 58 tests, clippy et fmt verts, captures inspectées.
- 2026-10-14 : SUNI-032 finition visuelle (dialogues, accueil, outils colorés, À propos) + édition (rotation, duplication, ordre Z, clavier), version 0.3.0, tag `v0.3.0`.
- 2026-10-14 : correctifs de la revue de la vue continue : Maj+Espace remonte d'un écran au lieu de descendre (lu avant Espace, test sans fenêtre), textures périmées dessinées sous les rendus à jour d'une autre échelle, commandes scriptées (`SUNIPDF_STARTUP_COMMANDS`) exécutées après la première image du document (3 × zoom arrière : 33 % depuis l'ajustement à 71 %), onglet repris sans état ramené en haut, rail droit en fenêtre basse limité à des icônes entières avec barre de défilement visible ; 112 tests, clippy et fmt verts, captures inspectées (Maj+Espace, fenêtres 1000 x 560 / 680 et pleine hauteur).
- 2026-10-14 : SUNI-012b vue continue (tranche continuous-view) : défilement continu par défaut, tuiles, zoom ancré, virtualisation, mesures ; décisions D038–D045 ; 111 tests, clippy et fmt verts, captures inspectées.
- 2026-10-14 : SUNI-031c édition avancée (tranche edit-advanced) : espace d'affichage, opacité, rotation libre, sélection multiple, presse-papiers d'objets, panneau Objets, texte sur la page, polices système, cadres de page, CLI ; décisions D027–D037 ; 83 tests, clippy et fmt verts, captures inspectées.
