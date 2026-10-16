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

## 0.11.0 : navigation, formulaires et copies protégées (contrôles locaux réussis)

Implémentés et validés localement : étiquettes en lecture seule, navigation par étiquette exacte et unique ou
indice physique `#N` ; destinations nommées et `/XYZ`, `/Fit`, `/FitH` non nul des liens/signets ; historique des sauts
dans la même page et du zoom ; erreurs de lecture de formulaire préservant les saisies avec relance explicite.
`/FitH` zéro/null et les autres modes gardent le saut de page. Aussi implémentés pour 0.11.0 : texte libre des listes
déroulantes déjà modifiables dans le panneau/sur la page, et copies d'extraction/division conservant chiffrement,
mots de passe et permissions, avec refus du chemin source. Politique des permissions inchangée ; création du
drapeau de liste modifiable à venir.

La source élargie réussit 390 tests du workspace (zéro échec, sept ignorés par défaut), formatage/Clippy avec
`-D warnings`, 858 clés EN/FR, 44 contrôles qpdf sur 42 PDF distincts et quatre documents NIST (744 pages ; 91,59 s,
pic mémoire enfant 329 347 072 octets). CI/paquets/publication restent en attente dans `PROJECT_STATUS.fr.md` ; source/paquets 0.10.0 publiés inchangés.

## Phases 0–1 : planification et fondations — livrées

Workspace Rust, PDFium, erreurs typées, ouverture/navigation/zoom/fermeture, mots de passe, glisser-déposer,
localisation FR/EN, paramètres persistés, thèmes clair/sombre, registre de commandes et ressources Windows sont en place.
Les jalons et décisions historiques restent dans `DECISIONS.fr.md` et `PROJECT_STATUS.fr.md`.

## Phase 2 : lecture et opérations PDF essentielles — largement livrée

Livrés : onglets, caches de rendu/miniatures bornés, vues continue/double page/Livre, sélection logique entre pages,
recherche, signets, fusion/division/extraction/rotation/réorganisation, enregistrement atomique, annulation/rétablissement et annotations.

Livrés en 0.10.0 : rotation de vue, plein écran et historique borné par onglet.
Implémentés en développement : lecture des étiquettes et navigation interne précise. Restent : création/recalage
sémantique des étiquettes et ordre voulu des colonnes complexes non balisées. qpdf ne prouve pas la préservation de
chaque lien, champ, calque ou pièce jointe.

## Phase 3 : édition professionnelle — partiellement livrée

Livrés : texte/images/formes, transformations d’objets, déplacement groupé et presse-papiers, polices système TrueType,
tampons/filigranes/numérotation, commentaires, métadonnées, signatures visuelles, remplissage/aplatissement AcroForm,
création de champs texte/case et sauvegarde/récupération.

Livrés en 0.10.0 : transformations groupées, recadrage souris, champs radio/listes, échanges FDF/XFDF de valeurs et clavier.
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

## Suite après le lot 0.11.0

1. Exécuter les dialogues natifs de mise à jour/enregistrement, Narrateur, imprimantes, ARM64 et lecteurs tiers.
2. Élargir corpus réel et mesures de performance ; les échantillons NIST ne prouvent pas une compatibilité universelle.
3. Définir séparément polices/paragraphes, conversion Office locale et services de confiance.

Aucune date/version suivante n’est engagée. L’IA facultative reste désactivée par défaut.
