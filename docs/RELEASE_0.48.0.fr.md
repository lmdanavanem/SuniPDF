# SuniPDF 0.48.0 — modifier les sommets des mesures enregistrées

[English](RELEASE_0.48.0.md) · [Guide de modification géométrique](MEASUREMENT_GEOMETRY.fr.md)

Cette version comprend le périmètre ci-dessous. Les fichiers publiés et la qualification sont suivis sur la [page de version 0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0).

## Changements

- Sélectionnez une mesure enregistrée et déplacez ses sommets existants sur la page. Aperçu, ajustement, Appliquer et Annuler en une étape maintiennent une validation explicite.
- Conservez calibration propre à la mesure, unités, précision, identité, réponses et champs de quantité. Quantités mesurées et totaux tarifaires compatibles suivent la géométrie acceptée.
- Empêchez la validation de modifications invalides ou périmées ; rotation, recadrage, annulation et documents protégés utilisent les contrôles existants des mesures.
- Validez les changements purement documentaires par un parcours CI borné, tout en conservant les contrôles complets pour sources, scripts, workflows et versions distribuées.
- Intégrez les paquets Windows 0.47.0 publiés aux tests de mise à niveau portable et MSI.

## Qualification

Les preuves et fichiers de distribution sont suivis sur la [page de version 0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0). Les résultats concernent la source exacte qui y figure.

## Périmètre

Le nombre de sommets et le type de mesure restent fixes. Accrochage vectoriel, ajout ou suppression de sommets, recalcul par lots et zones à échelles multiples restent ouverts. La commande CLI existante ne change pas. Les paquets restent non signés ; qualification ARM64 native, matérielle et d'accessibilité restent distinctes des tests automatisés.
