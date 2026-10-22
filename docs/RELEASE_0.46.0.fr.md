# SuniPDF 0.46.0 — tables de quantités et champs métier typés

[English](RELEASE_0.46.0.md) · [Guide des quantités](QUANTITY_TABLE.fr.md)

Statut : publiée en Latest stable [0.46.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.46.0), source `3e6763964d6b06f004a09fd7473ddf5bb4a6417f`, le 2026-10-22. [Version précédente](RELEASE_0.45.0.fr.md).

## Périmètre

**Table des quantités** est accessible depuis **Tous les outils**, les contrôles de mesure et le registre d’annotations. Elle lit marqueurs de comptage et mesures enregistrés avec leurs annotations sources. Filtres réels et pagination de 50 lignes maintiennent la correspondance entre lignes affichées et sous-totaux.

Chaque annotation compatible peut recevoir un prix unitaire explicite, une devise, une unité de référence du prix et une précision d’arrondi, ainsi que des colonnes typées texte, nombre, date et choix. **Appliquer les champs** modifie ses métadonnées en une étape Annuler ; **Annuler la modification** abandonne le brouillon. L’enregistrement du PDF conserve les champs. Géométrie des mesures, identités d’annotations et fils de revue sont préservés.

Quantités et prix absents restent absents plutôt que devenir zéro. Totaux de comptage, longueur et surface conservent leurs unités. Le calcul peut convertir une quantité vers une unité de prix compatible choisie explicitement ; les devises ne sont jamais converties. Les coûts sont arrondis par ligne et additionnés séparément par devise et précision. Prix manquants, lignes exclues et couverture incomplète des coûts restent visibles.

Les exports CSV et JSON comprennent la provenance de l’instantané source, les filtres et la couverture des sous-totaux. Exporter exige le droit de copie et un nouveau fichier. Modifier exige le droit d’annotation et une cible actuelle. La CLI propose `quantity-table`, `set-quantity-fields` et `clear-quantity-fields` ; une modification produit une nouvelle copie PDF, y compris pour les entrées protégées compatibles.

## Qualification

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738061382) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738073755) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39738061664). 732 tests réussis, 10 ignorés par défaut. 1348 clés EN/FR. 38 scénarios portables. 32 références MSI. 160 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 32 appels CLI vérifient quantités réelles, champs typés, coûts exacts, filtres/groupes, protection CSV, conversion d’unités, retrait, refus périmés/d’écrasement, permissions AES et conservation des sources. [Preuves](qa/RELEASE_0.46.0.json).

## Limites restantes

AEC-007 reste partiel. Ce lot ne fournit ni formules arbitraires, règles fiscales, conversion de devises, comptage automatique de symboles, agrégation entre documents ni système complet de devis. Les angles restent visibles mais sont exclus des totaux additifs et tarifés. Une annotation ordinaire sans quantité compatible ne reçoit aucune quantité inventée. Types de colonnes et options de choix doivent rester cohérents dans le document ; bornes, métadonnées périmées ou incompatibles peuvent provoquer un refus explicite. Qualification native/matérielle et critères 1.0 restent distincts.
