# SuniPDF 0.48.0 — modifier les sommets des mesures enregistrées

[English](RELEASE_0.48.0.md) · [Guide de modification géométrique](MEASUREMENT_GEOMETRY.fr.md)

Statut : publiée en Latest stable [0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0), source `6f32c20259f6bae6e1b60090fe74acfb5dc69ba7`, le 2026-10-22. [Version précédente](RELEASE_0.47.0.fr.md).

## Changements

- Sélectionnez une mesure enregistrée et déplacez ses sommets existants sur la page. Aperçu, ajustement, Appliquer et Annuler en une étape maintiennent une validation explicite.
- Conservez calibration propre à la mesure, unités, précision, identité, réponses et champs de quantité. Quantités mesurées et totaux tarifaires compatibles suivent la géométrie acceptée.
- Empêchez la validation de modifications invalides ou périmées ; rotation, recadrage, annulation et documents protégés utilisent les contrôles existants des mesures.
- Validez les changements purement documentaires par un parcours CI borné, tout en conservant les contrôles complets pour sources, scripts, workflows et versions distribuées.
- Intégrez les paquets Windows 0.47.0 publiés aux tests de mise à niveau portable et MSI.

## Qualification

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760622965) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760636143) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39760622897). 752 tests réussis, 10 ignorés par défaut. 1388 clés EN/FR. 40 scénarios portables. 34 références MSI. 170 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 26 appels CLI vérifient échelles enregistrées/réouvertes, remplacement géométrique explicite, conversion d’unité, précision, recalibration, identité stable, indépendance des autres mesures, conservation des champs de quantité et bases tarifaires, recalcul des totaux, refus invalides/périmés et fichiers originaux inchangés. Ils ne constituent pas une qualification du bureau natif ni d’ARM64. [Preuves](qa/RELEASE_0.48.0.json).

## Périmètre

Le nombre de sommets et le type de mesure restent fixes. Accrochage vectoriel, ajout ou suppression de sommets, recalcul par lots et zones à échelles multiples restent ouverts. La commande CLI existante ne change pas. Les paquets restent non signés ; qualification ARM64 native, matérielle et d'accessibilité restent distinctes des tests automatisés.
