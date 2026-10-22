# SuniPDF 0.45.0 — bibliothèques d’annotations réutilisables

[English](RELEASE_0.45.0.md) · [Guide de la bibliothèque](ANNOTATION_LIBRARY.fr.md)

Statut : publiée en Latest stable [0.45.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.45.0), source `51f07a6b27733e4bbfd46bcbc45bc2a82d4886c8`, le 2026-10-22. [Version précédente](RELEASE_0.44.0.fr.md).

## Périmètre

Une bibliothèque locale visible permet d’enregistrer les brouillons pris en charge, de nommer les modèles, de les réutiliser, d’importer/exporter un fichier JSON documenté et de supprimer explicitement les entrées. Les modèles comprennent formes, textes en fontes standard, polylignes/polygones, nuages de révision, tampons prédéfinis et images PNG incorporées. Un groupe plat contient au plus 16 primitives.

Le placement utilise une origine et une échelle explicites sur la page courante, puis Aperçu et Appliquer. L’application d’un groupe constitue une seule étape Annuler ; ses éléments restent des annotations PDF individuelles, avec de nouvelles identités. Le modèle reste disponible ; sa suppression ultérieure ne retire pas les annotations déjà placées.

Le fichier versionné contient les images et la provenance déclarée. L’import valide les types et les limites de ressources avant d’enregistrer une bibliothèque fusionnée ; conflits ou échecs conservent son état précédent. Scripts, chemins externes, identités PDF, fils de revue et calibrations de mesure sont exclus. Les textes de provenance et de licence sont des déclarations utilisateur, sans vérification indépendante des droits.

La gestion CLI utilise des fichiers de bibliothèque explicites. Le placement dans un PDF exige le droit d’annotation et une nouvelle copie de sortie ; originaux et destinations existantes restent protégés. Le chiffrement pris en charge est conservé.

## Qualification

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732620304) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732631044) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732620287). 724 tests réussis, 10 ignorés par défaut. 1302 clés EN/FR. 37 scénarios portables. 31 références MSI. 155 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 24 appels CLI vérifient imports stricts, modèles et groupes nommés, export sélectionné, suppression, placement, identités neuves, conservation des sources et refus explicites. [Preuves](qa/RELEASE_0.45.0.json).

## Limites restantes

Ce premier lot ne capture pas arbitrairement toutes les annotations d’un PDF, ne conserve pas les apparences importées quelconques et ne permet pas de groupes imbriqués. Il ne transfère ni mesures, ni échelles de page, ni pièces jointes, ni actions exécutables. AEC-006 reste partiel tant que ses critères complets ne sont pas démontrés. Qualification native/matérielle et critères 1.0 restent distincts.
