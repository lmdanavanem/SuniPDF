# SuniPDF 0.49.0 — ajouter et supprimer des sommets de mesures enregistrées

[English](RELEASE_0.49.0.md) · [Guide de modification géométrique](MEASUREMENT_GEOMETRY.fr.md)

Statut : publiée en Latest stable [0.49.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.49.0), source `a95774d56c3b9ce02a9743431a5d0c2b079b330e`, le 2026-10-22. [Version précédente](RELEASE_0.48.0.fr.md).

## Changements

- Insérez un sommet sur un segment d’une polyligne, d’un périmètre ou d’une surface polygonale enregistrée, y compris sur l’arête de fermeture d’un polygone. Retirez le sommet sélectionné dans les limites de la forme.
- Sélectionnez les sommets par numéro, même lorsque plusieurs points occupent la même position, et conservez leur déplacement sur la page.
- Examinez l’aperçu, ajustez le brouillon puis appliquez avec une seule étape d’annulation. Conservez type, calibration, unité, précision, identité, réponses et champs de quantité saisis ; valeurs mesurées et totaux tarifaires compatibles suivent la géométrie validée.
- Gardez le nombre de points fixe pour les longueurs et les angles, refusez les modifications invalides ou périmées, et préservez paramètres de page et autres mesures.
- Intégrez les paquets Windows 0.48.0 publiés aux tests de mise à niveau portable et MSI.

## Qualification

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831660301) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831682434) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39831660552). 761 tests réussis, 10 ignorés par défaut. 1397 clés EN/FR. 41 scénarios portables. 35 références MSI. 175 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 63 appels CLI vérifient insertion et suppression sur polylignes, périmètres et surfaces, nombres de sommets après réouverture, conservation des calibrations et quantités, recalcul des totaux, limites minimum/maximum et des types fixes, refus invalides/périmés et cycle de vie des mesures conservé. Ils ne constituent pas une qualification du bureau natif ni d’ARM64. [Preuves](qa/RELEASE_0.49.0.json).

## Périmètre

Les polylignes conservent au moins deux sommets ; les périmètres et surfaces polygonales, au moins trois. Ces formes acceptent au maximum 128 sommets. Les longueurs gardent deux points et les angles trois. La CLI existante accepte le remplacement explicite de la géométrie ; cette version n’ajoute aucune commande CLI.

Accrochage vectoriel, recalcul par lots, vues multi-échelles et qualification sur plans réels restent ouverts. AEC-002 et AEC-003 restent partiels. Les paquets restent non signés ; qualification ARM64 native, matérielle et d’accessibilité restent distinctes des tests automatisés.
