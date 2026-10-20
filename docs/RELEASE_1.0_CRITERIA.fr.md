# Critères d’acceptation de SuniPDF 1.0

Dernier lot vérifié : [0.39.0 — modèles de décoration réutilisables](RELEASE_0.39.0.fr.md). Les critères restants ci-dessous restent ouverts.

Lot précédemment vérifié : [0.38.0 — six zones d’en-tête et pied de page](RELEASE_0.38.0.fr.md). Les critères restants ci-dessous restent ouverts.

Lot précédemment vérifié : [0.37.0 — filigranes image](RELEASE_0.37.0.fr.md). Les critères restants ci-dessous restent ouverts.

[English](RELEASE_1.0_CRITERIA.md)

Ce document décrit la préparation acceptée après la 0.32.0 ; il ne déclare pas la 1.0 validée. La version
stable est maintenant la [0.39.0](RELEASE_0.39.0.fr.md). Les fonctions et refus prudents de la
[matrice](FEATURE_MATRIX.fr.md) restent applicables : le numéro 1.0 n’élargit pas la prise en charge des
polices, des exports Office, des signatures, de XFA ou de la conformité PDF.

| Critère | Preuve nécessaire | Écart actuel |
|---|---|---|
| Authenticité des mises à jour | Métadonnées signées et paquets vérifiés avec une clé publique intégrée ; contrôles de version/expiration, refus des modifications et gestion/rotation des clés documentées | HTTPS GitHub et SHA256 sur le même canal ; aucune clé indépendante configurée |
| Identité de publication Windows | Signatures Authenticode horodatées vérifiées sur les EXE/MSI distribués | Aucun certificat de production fourni ; un certificat de test ne qualifie pas la publication |
| Protection des documents | Corpus sous licence et empreintes fixes, entrées corrompues/volumineuses, échec atomique, sources inchangées et réouverture/rendu vérifiés | Le socle QA publié comprend quatre documents NIST/744 pages et sept exemples pédagogiques PDF Association/8 pages ; les cas clients, hostiles et volumineux restent à élargir |
| Interopérabilité indépendante | Documents chiffrés, formulaires, annotations et signatures sauvegardés puis vérifiés dans les lecteurs nommés avec versions et résultats précis | Les contrôles qpdf/MuPDF couvrent certains cas ; Acrobat/Foxit restent à qualifier |
| Utilisation Windows native | Installation et migration, notification/installation/annulation/retour arrière, dialogues de sauvegarde/récupération, clavier/Narrator, DPI/GPU et impression physique | Les tests scriptés, sans fenêtre ou hors écran ne qualifient pas les interactions natives ni une imprimante physique |
| Prise en charge ARM64 | Exécuter le paquet public sur une vraie machine Windows ARM64 et consigner appareil, système et résultats | Compilation croisée et inspection PE seulement ; exécution expérimentale jusqu’à qualification |
| Annonces PDF/A et PDF/UA | Nommer les profils pris en charge, conserver les rapports de validation indépendante et réaliser les vérifications humaines applicables | Préserver les métadonnées ne valide pas la conformité ; aucune annonce universelle PDF/A/PDF/UA |

Un résultat indisponible ou non pris en charge doit rester explicite. Un rapport partiel, une compilation
réussie, un test ignoré ou une signature de test ne permet pas de clôturer un critère non vérifié.

Le premier lot de préparation, publié en 0.33.0, ajoute sept exemples pédagogiques de la PDF Association, épinglés et sous licence
CC BY-SA 4.0, des mutations déterministes PDF/CMS, des mesures par commande et les caractéristiques de
l’environnement. Ces exemples couvrent les annotations UTF-8, les sauvegardes incrémentales, un en-tête
décalé et les structures colorimétriques/profils de sortie. Ils sont distincts des documents NIST et d’un
corpus client/hostile. Les téléchargements et copies PDF dérivées restent hors Git et des paquets.

La campagne de mutations est un contrôle borné, **pas du fuzzing guidé par la couverture**. Des contrôles
négatifs exécutables vérifient ses limites de durée et de sortie ; une modification réussie doit pouvoir
être rouverte et un échec ne doit publier aucun fichier. Les rapports identifient l’environnement et les
limites de mesure ; les résultats CI/machine virtuelle ne démontrent pas les performances chez un client.
Les deux cartes P1 concernées restent ouvertes jusqu’aux preuves complètes de leurs critères.

Préparer une version candidate après réussite de la CI, de la sécurité et des paquets sur la source figée,
et satisfaction des P1 du périmètre annoncé. Geler les fonctions pour une utilisation pilote, corriger les
régressions, puis qualifier cette source et les paquets réellement téléchargés. Conserver les preuves du
metteur à jour et des installations, les guides anglais/français, les inventaires de licences, les liens
vers les sources exactes et les instructions de récupération.

Les suggestions P3 restantes peuvent suivre la 1.0, sauf besoin utilisateur concret. Les lots signatures visuelles 0.35.0 et raccourcis 0.36.0 sont livrés dans leurs limites documentées ; qualification native et Explorer restent ouverts. Les compteurs Projects sont historiques tant que les modifications restent différées. Cette liste ne promet aucune date ni aucun résultat matériel non observé.
