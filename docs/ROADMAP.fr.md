# Roadmap SuniPDF

[English](ROADMAP.md)

Cette roadmap distingue les fonctions livrées, les changements implémentés en attente de validation et le travail futur.
Les contrôles exécutés et preuves de publication sont dans [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md).

## Base livrée : 0.9.0

Les sept fonctions demandées sont livrées : sauvegarde/récupération, impression Windows, sélection enrichie,
vues double page/Livre, création de champs texte/case, OCR anglais/français hors ligne et signatures numériques CMS/PFX.
L’anglais est la langue par défaut ; le français reste disponible. Les 14 outils ont leurs commandes.
Voir [FEATURE_MATRIX.fr.md](FEATURE_MATRIX.fr.md) pour le périmètre exact et [KNOWN_ISSUES.fr.md](KNOWN_ISSUES.fr.md) pour les limites.

## 0.9.1 : fiabilité et distribution

Changements implémentés, en attente des validations et de la publication consignées dans `PROJECT_STATUS.fr.md` :

- Préserver l’identification PDF/A et les propriétés XMP indépendantes lors de l’édition des métadonnées ; refuser les XML malformés et DTD.
  La conservation de l’identification ne certifie pas la conformité PDF/A.
- Écrire explicitement la longueur de clé AES-256 attendue par les lecteurs stricts, y compris après rechiffrement.
- Ajouter en CI des contrôles structurels qpdf figés sur les fixtures synthétiques et les sorties produites.
  Il s’agit d’une QA structurelle indépendante, pas d’une couverture de corpus réel ou hostile.
- Étendre les contrôles de mise à jour/restauration portable et ajouter une régression réelle d’installation/mise à niveau MSI sur runners Windows jetables.
- Dériver l’icône Windows du visuel dédié fourni par l’auteur et retouché (`assets/icon/sunipdf-source.png`).

Critère de publication : contrôles du workspace, des PDF, du helper portable et régression MSI sur runner réussis ;
contenus distribués et empreintes conformes à la release publiée. L’UX interactive de mise à jour et le matériel réel
restent des vérifications distinctes, même après succès de l’automatisation.

## Phases 0–1 : planification et fondations — livrées

Workspace Rust, PDFium, erreurs typées, ouverture/navigation/zoom/fermeture, mots de passe, glisser-déposer,
localisation FR/EN, paramètres persistés, thèmes clair/sombre, registre de commandes et ressources Windows sont en place.
Les jalons et décisions historiques restent dans `DECISIONS.fr.md` et `PROJECT_STATUS.fr.md`.

## Phase 2 : lecture et opérations PDF essentielles — largement livrée

Livrés : onglets, caches de rendu/miniatures bornés, vues continue/double page/Livre, sélection logique entre pages,
recherche, signets, fusion/division/extraction/rotation/réorganisation, enregistrement atomique, annulation/rétablissement et annotations.

Restent : rotation de vue seule, historique de navigation, labels de pages, lecture plein écran et ordre voulu des
colonnes complexes non balisées. qpdf ne prouve pas la préservation de chaque lien, champ, calque ou pièce jointe.

## Phase 3 : édition professionnelle — partiellement livrée

Livrés : texte/images/formes, transformations d’objets, déplacement groupé et presse-papiers, polices système TrueType,
tampons/filigranes/numérotation, commentaires, métadonnées, signatures visuelles, remplissage/aplatissement AcroForm,
création de champs texte/case et sauvegarde/récupération.

Restent : sous-ensembles de polices et CFF/OTF/TTC ; paragraphes ; collage éditable ; rotation/redimensionnement groupés ;
recadrage à la souris ; autres types de champs et FDF/XFDF ; activation clavier des widgets sur page.
Critère : rendu, enregistrement et réouverture couverts pour chaque opération, avec contrôles d’accessibilité applicables.

## Phase 4 : conversion et OCR — partiellement livrée

Livrés : images ↔ PDF, PDF → texte/PNG, optimisation sans perte, lots CLI et OCR anglais/français local embarqué.

Restent : Office → PDF via convertisseur local détecté, PDF → Word/Excel/PowerPoint avec limites de fidélité mesurées
et explicites, rééchantillonnage/compression avec perte, interface de lots et profils réutilisables.

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

Restent : corpus PDF réel/hostile/volumineux avec fixtures licenciées, enregistrements interrompus/disque plein, fuzzing,
benchmarks avec matériel, inventaire complet des licences, Narrateur et lecteurs tiers, parcours interactifs de mise à
jour MSI/portable, imprimantes physiques et exécution ARM64/OpenGL réelle.
La régression MSI ne couvre que son scénario scripté ; compiler un paquet ARM64 ne valide pas son exécution matérielle.

## Ordre recommandé après 0.9.1

| Priorité | Travail | Preuve de sortie |
|---|---|---|
| P1 | Corpus réel et scénarios de sécurité des données | Fixtures reproductibles, contrôles des sorties et aucune perte de l’original |
| P1 | UX de mise à jour MSI/portable, Narrateur, imprimantes et lecteurs | Résultats consignés sur les vrais parcours Windows/lecteurs/matériels |
| P1 | Exécution ARM64 et mesures de performance | Machine/pilotes et mesures reproductibles d’ouverture/rendu/mémoire |
| P2 | Polices, paragraphes/groupes et recadrage souris | Enregistrer/réouvrir/rendre sur pages normales, pivotées et recadrées |
| P2 | Types de champs, XFDF/FDF et widgets clavier | Régressions d’échange, de clavier et d’accessibilité |
| P2 | Comparaison, lots et conversion Office locale | Travaux bornés, annulation, fidélité et source préservée |
| Plus tard | Services de confiance, horodatage/LTV, MCP et intégrations optionnelles | Modèle de confiance, périmètre et sécurité explicitement validés |

Aucune date ni version future n’est engagée par ce backlog. L’IA optionnelle reste désactivée par défaut.
